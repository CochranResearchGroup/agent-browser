#![allow(unused_imports)]
use super::cdp_free_plan::optional_command_string;
use super::daemon::{
    account_ids_from_command, target_service_ids_from_command, BrowserCapabilityLaunchSelection,
    CloseBehavior,
};
use crate::native::browser_navigation::{
    add_manual_login_hint_warning, persist_service_owned_navigate_tab,
};
use crate::native::network::resolve_fetch_paused;
use crate::native::network_archive::{har_cdp_protocol_to_http_version, har_extract_headers};
use crate::native::service_model::{
    retained_display_allocation_candidates, service_profile_allocations,
    service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
    BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
    BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
    ControlInputProvider, DisplayAllocation, JobState as ServiceJobState, LeaseState, MonitorState,
    ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy, ProfileLeaseDisposition,
    ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease, RemoteViewHandoff,
    RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent, ServiceEventKind,
    ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle, ViewStream,
    ViewStreamProvider, ViewerLease,
};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::state;
use crate::native::stream_runtime::{
    stream_file_path, write_engine_file, write_extensions_file, write_provider_file,
};
use serde_json::{json, Map, Value};
use std::env;
use std::path::{Path, PathBuf};
pub(crate) fn browser_capability_service_state(cmd: &Value) -> Result<ServiceState, String> {
    if let Some(service_state) = cmd.get("serviceState") {
        return serde_json::from_value::<ServiceState>(service_state.clone())
            .map_err(|err| format!("Invalid serviceState: {}", err));
    }
    LockedServiceStateRepository::default_json()
        .and_then(|repository| repository.load_snapshot())
        .map_err(|err| err.to_string())
}
pub(crate) fn executable_path_is_operator_supplied(
    executable_path: Option<&str>,
    cmd: &Value,
) -> bool {
    if cmd.get("executablePath").is_some() {
        return !matches!(
            optional_command_string(cmd, "executablePathSource").as_deref(),
            Some("manifest")
        );
    }
    let Some(executable_path) = executable_path else {
        return false;
    };
    let Ok(env_executable_path) = env::var("AGENT_BROWSER_EXECUTABLE_PATH") else {
        return true;
    };
    if env_executable_path != executable_path {
        return true;
    }
    !matches!(
        env::var("AGENT_BROWSER_EXECUTABLE_PATH_SOURCE").as_deref(),
        Ok("manifest")
    )
}
pub(crate) fn select_browser_capability_launch_binding(
    service_state: &ServiceState,
    cmd: &Value,
    browser_build: BrowserBuild,
    profile_id: Option<&str>,
    headless: bool,
    cdp_free: bool,
) -> Result<BrowserCapabilityLaunchSelection, &'static str> {
    let registry = &service_state.browser_capability_registry;
    let browser_build_label = browser_build_label(browser_build);
    let binding = registry
        .browser_preference_bindings
        .iter()
        .filter(|binding| {
            preference_binding_matches_launch_command(binding, cmd, Some(browser_build_label))
        })
        .max_by(|left, right| {
            preference_binding_rank(left, cmd).cmp(&preference_binding_rank(right, cmd))
        })
        .ok_or("no_matching_preference_binding")?;
    let binding_id = registry_string_field(binding, "id").ok_or("binding_missing_id")?;
    let executable_id = registry_string_field(binding, "preferredExecutableId")
        .ok_or("binding_missing_executable_id")?;
    let executable = registry
        .browser_executables
        .iter()
        .find(|candidate| {
            registry_string_field(candidate, "id").as_deref() == Some(executable_id.as_str())
                && registry_string_field(candidate, "buildLabel").as_deref()
                    == Some(browser_build_label)
        })
        .ok_or("executable_not_found")?;
    let executable_path =
        registry_string_field(executable, "executablePath").ok_or("executable_path_missing")?;
    if !PathBuf::from(&executable_path).is_file() {
        return Err("executable_path_not_found");
    }
    let host_id = registry_string_field(binding, "preferredHostId")
        .or_else(|| registry_string_field(executable, "hostId"))
        .ok_or("host_id_missing")?;
    let host = registry
        .browser_hosts
        .iter()
        .find(|candidate| {
            registry_string_field(candidate, "id").as_deref() == Some(host_id.as_str())
        })
        .ok_or("host_not_found")?;
    if registry_string_field(host, "hostKind").as_deref() != Some("local")
        || host.get("reachable").and_then(Value::as_bool) != Some(true)
        || registry_string_field(host, "lifecycleOwner").as_deref() != Some("agent_browser")
    {
        return Err("host_not_local_reachable_agent_browser_owned");
    }
    let capability_id = registry_string_field(binding, "preferredCapabilityId");
    let capability = capability_id
        .as_ref()
        .and_then(|id| {
            registry.browser_capabilities.iter().find(|candidate| {
                registry_string_field(candidate, "id").as_deref() == Some(id.as_str())
            })
        })
        .or_else(|| {
            registry.browser_capabilities.iter().find(|candidate| {
                registry_string_field(candidate, "executableId").as_deref()
                    == Some(executable_id.as_str())
                    && registry_string_field(candidate, "hostId").as_deref()
                        == Some(host_id.as_str())
            })
        })
        .ok_or("capability_not_found")?;
    let capability_id = capability_id.or_else(|| registry_string_field(capability, "id"));
    if registry_string_field(capability, "executableId").as_deref() != Some(executable_id.as_str())
    {
        return Err("capability_executable_mismatch");
    }
    if headless {
        if capability.get("headlessSupported").and_then(Value::as_bool) != Some(true) {
            return Err("headless_not_supported");
        }
    } else if capability.get("headedSupported").and_then(Value::as_bool) != Some(true) {
        return Err("headed_not_supported");
    }
    if cdp_free {
        if capability
            .get("cdpFreeLaunchSupported")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err("cdp_free_launch_not_supported");
        }
    } else if capability.get("cdpSupported").and_then(Value::as_bool) != Some(true) {
        return Err("cdp_not_supported");
    }
    let profile_compatibility =
        profile_compatibility_gate(service_state, profile_id, &host_id, &executable_id);
    if !profile_compatibility.allowed {
        return Err("profile_compatibility_missing_or_blocked");
    }
    let validation = validation_gate(
        service_state,
        &host_id,
        &executable_id,
        capability_id.as_deref(),
        cdp_free,
    );
    if !validation.allowed {
        return Err("validation_evidence_missing_or_not_passed");
    }
    Ok(BrowserCapabilityLaunchSelection {
        binding_id,
        host_id,
        executable_id,
        capability_id,
        executable_path,
        profile_compatibility_ids: profile_compatibility.allowed_ids,
        validation_evidence_ids: validation.passed_ids,
    })
}
pub(crate) fn browser_build_label(browser_build: BrowserBuild) -> &'static str {
    match browser_build {
        BrowserBuild::StockChrome => "stock_chrome",
        BrowserBuild::StealthcdpChromium => "stealthcdp_chromium",
        BrowserBuild::CdpFreeHeaded => "cdp_free_headed",
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProfileCompatibilityGate {
    pub(crate) allowed: bool,
    pub(crate) allowed_ids: Vec<String>,
}
pub(crate) fn profile_compatibility_gate(
    service_state: &ServiceState,
    profile_id: Option<&str>,
    host_id: &str,
    executable_id: &str,
) -> ProfileCompatibilityGate {
    let Some(profile_id) = profile_id else {
        return ProfileCompatibilityGate {
            allowed: true,
            allowed_ids: Vec::new(),
        };
    };
    let matching_rows = service_state
        .browser_capability_registry
        .profile_compatibility
        .iter()
        .filter(|compatibility| {
            registry_string_field(compatibility, "profileId").as_deref() == Some(profile_id)
                && registry_string_field(compatibility, "hostId").as_deref() == Some(host_id)
                && registry_string_field(compatibility, "executableId").as_deref()
                    == Some(executable_id)
        })
        .collect::<Vec<_>>();
    let blocked = matching_rows.iter().any(|compatibility| {
        compatibility.get("compatible").and_then(Value::as_bool) != Some(true)
            || compatibility
                .get("requiresOperatorOverride")
                .and_then(Value::as_bool)
                == Some(true)
    });
    let allowed_ids = matching_rows
        .iter()
        .filter(|compatibility| {
            compatibility.get("compatible").and_then(Value::as_bool) == Some(true)
                && compatibility
                    .get("requiresOperatorOverride")
                    .and_then(Value::as_bool)
                    != Some(true)
        })
        .filter_map(|compatibility| registry_string_field(compatibility, "id"))
        .collect::<Vec<_>>();
    ProfileCompatibilityGate {
        allowed: !allowed_ids.is_empty() && !blocked,
        allowed_ids,
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ValidationGate {
    pub(crate) allowed: bool,
    pub(crate) passed_ids: Vec<String>,
}
pub(crate) fn validation_gate(
    service_state: &ServiceState,
    host_id: &str,
    executable_id: &str,
    capability_id: Option<&str>,
    cdp_free: bool,
) -> ValidationGate {
    let matching_rows = service_state
        .browser_capability_registry
        .validation_evidence
        .iter()
        .filter(|evidence| {
            registry_string_field(evidence, "hostId").as_deref() == Some(host_id)
                && registry_string_field(evidence, "executableId").as_deref() == Some(executable_id)
                && capability_id.is_none_or(|capability_id| {
                    registry_string_field(evidence, "capabilityId").as_deref()
                        == Some(capability_id)
                })
                && validation_kind_matches_launch(evidence, cdp_free)
        })
        .collect::<Vec<_>>();
    let blocked = matching_rows.iter().any(|evidence| {
        matches!(
            evidence.get("state").and_then(Value::as_str),
            Some("failed") | Some("stale")
        )
    });
    let passed_ids = matching_rows
        .iter()
        .filter(|evidence| evidence.get("state").and_then(Value::as_str) == Some("passed"))
        .filter_map(|evidence| registry_string_field(evidence, "id"))
        .collect::<Vec<_>>();
    ValidationGate {
        allowed: !passed_ids.is_empty() && !blocked,
        passed_ids,
    }
}
pub(crate) fn validation_kind_matches_launch(evidence: &Value, cdp_free: bool) -> bool {
    matches!(
        evidence.get("kind").and_then(Value::as_str), Some("launch") |
        Some("site_reliability") | Some("cdp_attach") if ! cdp_free
    ) || cdp_free
        && matches!(
            evidence.get("kind").and_then(Value::as_str),
            Some("launch") | Some("cdp_free_launch") | Some("site_reliability")
        )
}
pub(crate) fn preference_binding_matches_launch_command(
    binding: &Value,
    cmd: &Value,
    browser_build_label: Option<&str>,
) -> bool {
    let browser_build_matches = browser_build_label.is_none_or(|label| {
        registry_string_field(binding, "browserBuild")
            .as_deref()
            .is_none_or(|build| build == label)
    });
    let target_service_ids = target_service_ids_from_command(cmd);
    let account_ids = account_ids_from_command(cmd);
    let service_name = optional_command_string(cmd, "serviceName");
    let task_name = optional_command_string(cmd, "taskName");
    let has_filters = registry_array_field_has_items(binding, "targetServiceIds")
        || registry_array_field_has_items(binding, "accountIds")
        || registry_array_field_has_items(binding, "serviceNames")
        || registry_array_field_has_items(binding, "taskNames");
    let identity_matches = registry_string_field(binding, "scope").as_deref() == Some("global")
        && !has_filters
        || has_filters
            && registry_binding_filter_matches(binding, "targetServiceIds", &target_service_ids)
            && registry_binding_filter_matches(binding, "accountIds", &account_ids)
            && registry_binding_optional_filter_matches(
                binding,
                "serviceNames",
                service_name.as_deref(),
            )
            && registry_binding_optional_filter_matches(binding, "taskNames", task_name.as_deref());
    browser_build_matches && identity_matches
}
pub(crate) fn preference_binding_rank(binding: &Value, cmd: &Value) -> (i64, i64, String) {
    let priority = binding
        .get("priority")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let specificity = i64::from(registry_array_field_intersects(
        binding,
        "accountIds",
        &account_ids_from_command(cmd),
    )) * 16
        + i64::from(registry_array_field_intersects(
            binding,
            "targetServiceIds",
            &target_service_ids_from_command(cmd),
        )) * 8
        + i64::from(
            optional_command_string(cmd, "serviceName").is_some_and(|service_name| {
                registry_array_field_contains(binding, "serviceNames", &service_name)
            }),
        ) * 4
        + i64::from(
            optional_command_string(cmd, "taskName").is_some_and(|task_name| {
                registry_array_field_contains(binding, "taskNames", &task_name)
            }),
        ) * 2
        + i64::from(registry_string_field(binding, "scope").as_deref() != Some("global"));
    let id = registry_string_field(binding, "id").unwrap_or_default();
    (priority, specificity, id)
}
pub(crate) fn registry_string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
pub(crate) fn registry_array_field_contains(value: &Value, field: &str, expected: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .any(|item| item == expected)
        })
}
pub(crate) fn registry_array_field_has_items(value: &Value, field: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.as_str().is_some_and(|item| !item.is_empty()))
        })
}
pub(crate) fn registry_array_field_intersects(
    value: &Value,
    field: &str,
    expected: &[String],
) -> bool {
    expected
        .iter()
        .any(|item| registry_array_field_contains(value, field, item))
}
pub(crate) fn registry_binding_filter_matches(
    value: &Value,
    field: &str,
    expected: &[String],
) -> bool {
    !registry_array_field_has_items(value, field)
        || registry_array_field_intersects(value, field, expected)
}
pub(crate) fn registry_binding_optional_filter_matches(
    value: &Value,
    field: &str,
    expected: Option<&str>,
) -> bool {
    !registry_array_field_has_items(value, field)
        || expected.is_some_and(|expected| registry_array_field_contains(value, field, expected))
}
pub(crate) fn close_behavior_for_attached_browser(
    runtime_attach_managed: bool,
    leave_open: bool,
) -> CloseBehavior {
    if runtime_attach_managed && !leave_open {
        CloseBehavior::CloseBrowser
    } else {
        CloseBehavior::Detach
    }
}
pub(crate) fn close_behavior_for_launched_browser(
    runtime_profile_name: Option<&str>,
    leave_open: bool,
) -> CloseBehavior {
    if leave_open && runtime_profile_name.is_some() {
        CloseBehavior::Detach
    } else {
        CloseBehavior::CloseBrowser
    }
}
pub(crate) fn service_browser_id(session_id: &str) -> String {
    format!("session:{}", session_id)
}
