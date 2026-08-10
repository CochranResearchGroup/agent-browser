#![allow(unused_imports)]
use super::common::*;
use super::runtime::{
    account_ids_from_command, apply_service_browser_capability_selection,
    browser_build_from_command, browser_build_label, browser_host_from_command, handle_close,
    launch_profile_from_sources, optional_command_string, parse_control_input_provider,
    parse_view_stream_provider, registry_string_field,
    remote_headed_display_isolation_from_command, runtime_profile_from_sources, service_browser_id,
    target_service_ids_from_command, target_url_from_command, DaemonState,
};
pub(crate) async fn handle_service_resources(cmd: &Value) -> Result<Value, String> {
    let service_state = load_service_state_for_maintenance(cmd)?;
    Ok(service_resources_response(&service_state))
}
pub(crate) async fn handle_service_resources_monitor_summary() -> Result<Value, String> {
    service_resources_monitor_summary_response()
}
pub(crate) async fn handle_service_resources_write_monitor_summary(
    cmd: &Value,
) -> Result<Value, String> {
    let service_state = load_service_state_for_maintenance(cmd)?;
    service_resources_write_monitor_summary_response(&service_state)
}
pub(crate) async fn handle_service_gc(cmd: &Value) -> Result<Value, String> {
    let apply = cmd.get("apply").and_then(Value::as_bool).unwrap_or(false);
    if apply {
        let review_token = cmd.get("reviewToken").and_then(Value::as_str);
        let force_without_review = cmd
            .get("forceWithoutReview")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let repository = LockedServiceStateRepository::default_json()?;
        repository.mutate(|state| {
            let response = service_gc_apply_response(state, review_token, force_without_review);
            if let Some(error) = response.get("error").and_then(Value::as_str) {
                Err(error.to_string())
            } else {
                Ok(response)
            }
        })
    } else {
        let service_state = load_service_state_for_maintenance(cmd)?;
        Ok(service_gc_dry_run_response(&service_state))
    }
}
pub(crate) fn load_service_state_for_maintenance(cmd: &Value) -> Result<ServiceState, String> {
    if let Some(service_state) = cmd.get("serviceState") {
        serde_json::from_value::<ServiceState>(service_state.clone())
            .map_err(|err| format!("Invalid serviceState: {}", err))
    } else {
        LockedServiceStateRepository::default_json()?.load_snapshot()
    }
}
/// Return the no-launch service access plan from the current service state.
pub(crate) async fn handle_service_access_plan(cmd: &Value) -> Result<Value, String> {
    let mut service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    service_state.refresh_profile_readiness();
    let request = ServiceAccessPlanRequest {
        service_name: optional_command_string(cmd, "serviceName"),
        agent_name: optional_command_string(cmd, "agentName"),
        task_name: optional_command_string(cmd, "taskName"),
        target_service_ids: target_service_ids_from_command(cmd),
        account_ids: account_ids_from_command(cmd),
        target_url: target_url_from_command(cmd),
        site_policy_id: optional_command_string(cmd, "sitePolicyId"),
        challenge_id: optional_command_string(cmd, "challengeId"),
        readiness_profile_id: optional_command_string(cmd, "readinessProfileId"),
        runtime_profile: runtime_profile_from_sources(cmd, false),
        browser_build: browser_build_from_command(cmd),
        browser_build_explicit: cmd.get("browserBuild").and_then(Value::as_str).is_some(),
        browser_host: browser_host_from_command(cmd),
        view_stream_provider: optional_command_string(cmd, "viewStreamProvider")
            .or_else(|| optional_command_string(cmd, "viewStream"))
            .or_else(|| {
                cmd.get("params").and_then(|params| {
                    optional_command_string(params, "viewStreamProvider")
                        .or_else(|| optional_command_string(params, "viewStream"))
                })
            })
            .and_then(|value| parse_view_stream_provider(&value)),
        control_input_provider: optional_command_string(cmd, "controlInputProvider")
            .or_else(|| optional_command_string(cmd, "controlInput"))
            .or_else(|| {
                cmd.get("params").and_then(|params| {
                    optional_command_string(params, "controlInputProvider")
                        .or_else(|| optional_command_string(params, "controlInput"))
                })
            })
            .and_then(|value| parse_control_input_provider(&value)),
        display_isolation: remote_headed_display_isolation_from_command(cmd),
    };
    let mut plan = service_access_plan_for_state(&service_state, request);
    let summary = access_plan_browser_build_selection_summary(&plan);
    if let Some(object) = plan.as_object_mut() {
        object.insert("browserBuildSelectionSummary".to_string(), summary);
    }
    Ok(plan)
}
pub(crate) fn access_plan_browser_build_selection_summary(plan: &Value) -> Value {
    let selection = plan
        .pointer("/decision/launchPosture/browserBuildSelection")
        .unwrap_or(&Value::Null);
    let profile_compatibility = selection
        .get("profileCompatibility")
        .unwrap_or(&Value::Null);
    let validation_evidence = selection.get("validationEvidence").unwrap_or(&Value::Null);
    let browser_build = optional_command_string(selection, "browserBuild");
    let source = optional_command_string(selection, "source");
    let evidence_source = optional_command_string(selection, "evidenceSource");
    let profile_compatibility_status = optional_command_string(profile_compatibility, "status");
    let validation_evidence_status = optional_command_string(validation_evidence, "status");
    let selected_preference_binding_id =
        optional_command_string(selection, "selectedPreferenceBindingId");
    let mut compact_parts = vec![
        format!("build={}", browser_build.as_deref().unwrap_or("unknown")),
        format!("source={}", source.as_deref().unwrap_or("unknown")),
        format!(
            "evidence={}",
            evidence_source.as_deref().unwrap_or("unknown")
        ),
        format!(
            "override={}",
            if selection
                .get("operatorOverride")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "yes"
            } else {
                "no"
            }
        ),
        format!(
            "profileCompatibility={}",
            profile_compatibility_status.as_deref().unwrap_or("unknown")
        ),
        format!(
            "validation={}",
            validation_evidence_status.as_deref().unwrap_or("unknown")
        ),
    ];
    if let Some(binding_id) = selected_preference_binding_id.as_deref() {
        compact_parts.push(format!("preferenceBinding={binding_id}"));
    }
    let mut audit_flags = Vec::new();
    if source.as_deref() == Some("browser_preference_binding") {
        audit_flags.push("preference_binding_selected");
    }
    if selection
        .get("operatorOverride")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        audit_flags.push("operator_override");
    }
    if selection
        .get("requiresCdpFree")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        audit_flags.push("requires_cdp_free");
    }
    if profile_compatibility_status.as_deref() == Some("incompatible_or_mixed") {
        audit_flags.push("profile_compatibility_attention");
    }
    if matches!(
        validation_evidence_status.as_deref(),
        Some("failed_or_mixed" | "missing")
    ) {
        audit_flags.push("validation_evidence_attention");
    }
    let attention_required = audit_flags.iter().any(|flag| flag.ends_with("_attention"));
    json!(
        { "browserBuild" : browser_build, "source" : source, "evidenceSource" :
        evidence_source, "summary" : optional_command_string(selection, "summary"),
        "operatorOverride" : selection.get("operatorOverride").and_then(Value::as_bool)
        .unwrap_or(false), "requiresCdpFree" : selection.get("requiresCdpFree")
        .and_then(Value::as_bool).unwrap_or(false), "selectedProfileId" :
        optional_command_string(selection, "selectedProfileId"),
        "selectedProfileBrowserBuild" : optional_command_string(selection,
        "selectedProfileBrowserBuild"), "selectedPreferenceBindingId" :
        selected_preference_binding_id, "selectedPreferenceBindingReason" :
        optional_command_string(selection, "selectedPreferenceBindingReason"),
        "profileCompatibilityStatus" : profile_compatibility_status,
        "profileCompatibilityReason" : optional_command_string(profile_compatibility,
        "reason"), "profileCompatibilityIds" : string_array_field(profile_compatibility,
        "matchingIds"), "validationEvidenceStatus" : validation_evidence_status,
        "validationEvidenceReason" : optional_command_string(validation_evidence,
        "reason"), "validationEvidenceIds" : string_array_field(validation_evidence,
        "matchingIds"), "auditFlags" : audit_flags, "attentionRequired" :
        attention_required, "compact" : compact_parts.join(" "), }
    )
}
pub(crate) fn string_array_field(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
/// Evaluate browser capability launch gates without starting Chrome.
pub(crate) async fn handle_service_browser_capability_preflight(
    cmd: &Value,
) -> Result<Value, String> {
    let requested_build = browser_build_from_command(cmd);
    let cdp_free = cmd
        .get("requiresCdpFree")
        .or_else(|| cmd.get("cdpFree"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || requested_build == Some(BrowserBuild::CdpFreeHeaded)
        || cmd
            .get("cdpAttachmentAllowed")
            .and_then(Value::as_bool)
            .is_some_and(|allowed| !allowed);
    let headless = if cdp_free {
        false
    } else {
        cmd.get("headless").and_then(Value::as_bool).unwrap_or(true)
    };
    let mut launch_options = LaunchOptions {
        headless,
        executable_path: cmd
            .get("executablePath")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .or_else(|| env::var("AGENT_BROWSER_EXECUTABLE_PATH").ok()),
        profile: launch_profile_from_sources(cmd, true),
        runtime_profile: runtime_profile_from_sources(cmd, true),
        manual_login: cdp_free,
        attachable: !cdp_free,
        ..LaunchOptions::default()
    };
    let browser_capability_launch =
        apply_service_browser_capability_selection(&mut launch_options, cmd);
    Ok(json!(
        { "preflight" : true, "wouldLaunch" : false, "wouldApplyExecutable" :
        browser_capability_launch.applied, "browserCapabilityLaunch" :
        browser_capability_launch.to_value(), "request" : { "browserBuild" :
        requested_build.map(browser_build_label), "profileId" :
        service_profile_id(launch_options.profile.as_deref(), launch_options
        .runtime_profile.as_deref()), "headless" : launch_options.headless, "cdpFree"
        : cdp_free, "serviceName" : optional_command_string(cmd, "serviceName"),
        "agentName" : optional_command_string(cmd, "agentName"), "taskName" :
        optional_command_string(cmd, "taskName"), "targetServiceIds" :
        target_service_ids_from_command(cmd), "accountIds" :
        account_ids_from_command(cmd), "url" : target_url_from_command(cmd), },
        "selectedExecutablePath" : launch_options.executable_path, }
    ))
}
/// Generate operator-facing commands for binding a site/account to a known browser executable.
pub(crate) async fn handle_service_browser_capability_preference_guide(
    cmd: &Value,
) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    Ok(browser_capability_preference_guide(&service_state, cmd))
}
pub(crate) fn browser_capability_preference_guide(
    service_state: &ServiceState,
    cmd: &Value,
) -> Value {
    let registry = &service_state.browser_capability_registry;
    let requested_build = optional_command_string(cmd, "browserBuild");
    let target_service_ids = target_service_ids_from_command(cmd);
    let account_ids = account_ids_from_command(cmd);
    let service_name = optional_command_string(cmd, "serviceName");
    let task_name = optional_command_string(cmd, "taskName");
    let reason = optional_command_string(cmd, "reason")
        .unwrap_or_else(|| "operator_primary_browser_preference".to_string());
    let has_filter = !target_service_ids.is_empty()
        || !account_ids.is_empty()
        || service_name.is_some()
        || task_name.is_some();
    let mut suggestions = registry
        .browser_executables
        .iter()
        .filter(|executable| {
            requested_build.as_deref().is_none_or(|build| {
                registry_string_field(executable, "buildLabel").as_deref() == Some(build)
            })
        })
        .filter_map(|executable| {
            let executable_id = registry_string_field(executable, "id")?;
            let browser_build = registry_string_field(executable, "buildLabel")
                .or_else(|| requested_build.clone())
                .unwrap_or_else(|| "stock_chrome".to_string());
            let host_id = registry_string_field(executable, "hostId");
            let capability_id =
                matching_capability_id(registry, host_id.as_deref(), &executable_id);
            let command = browser_preference_command(BrowserPreferenceCommandInput {
                browser_build: &browser_build,
                executable_id: &executable_id,
                host_id: host_id.as_deref(),
                capability_id: capability_id.as_deref(),
                target_service_ids: &target_service_ids,
                account_ids: &account_ids,
                service_name: service_name.as_deref(),
                task_name: task_name.as_deref(),
                reason: &reason,
            });
            let existing_binding_ids = registry
                .browser_preference_bindings
                .iter()
                .filter(|binding| {
                    registry_string_field(binding, "preferredExecutableId").as_deref()
                        == Some(executable_id.as_str())
                })
                .filter_map(|binding| registry_string_field(binding, "id"))
                .collect::<Vec<_>>();
            Some(json!(
                { "executableId" : executable_id, "browserBuild" : browser_build,
                "hostId" : host_id, "capabilityId" : capability_id, "source" :
                registry_string_field(executable, "source"), "executablePath" :
                registry_string_field(executable, "executablePath"), "fresh" :
                executable.get("fresh").cloned().unwrap_or(Value::Null), "tags" :
                executable.get("tags").cloned().unwrap_or_else(|| json!([])),
                "existingBindingIds" : existing_binding_ids, "copyable" : has_filter,
                "command" : command, }
            ))
        })
        .collect::<Vec<_>>();
    suggestions.sort_by(|left, right| {
        json_string_field(left, "browserBuild")
            .cmp(&json_string_field(right, "browserBuild"))
            .then_with(|| {
                json_string_field(left, "executableId")
                    .cmp(&json_string_field(right, "executableId"))
            })
    });
    json!(
        { "guide" : true, "advisory" : true, "copyable" : has_filter, "requested" : {
        "browserBuild" : requested_build, "targetServiceIds" : target_service_ids,
        "accountIds" : account_ids, "serviceName" : service_name, "taskName" : task_name,
        "reason" : reason, }, "counts" : { "browserExecutables" : registry
        .browser_executables.len(), "matchingExecutables" : suggestions.len(),
        "browserPreferenceBindings" : registry.browser_preference_bindings.len(), },
        "suggestions" : suggestions, "recommendedNextStep" : if has_filter {
        "Copy the preferred command, run it, then run service browser-capability preflight for the same site/account before requesting browser work."
        } else {
        "Rerun with --target-service-id and --account-id to produce exact copyable prefer commands."
        }, }
    )
}
pub(crate) fn json_string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
pub(crate) fn matching_capability_id(
    registry: &BrowserCapabilityRegistry,
    host_id: Option<&str>,
    executable_id: &str,
) -> Option<String> {
    registry.browser_capabilities.iter().find_map(|capability| {
        let executable_matches =
            registry_string_field(capability, "executableId").as_deref() == Some(executable_id);
        let host_matches = host_id.is_none_or(|host_id| {
            registry_string_field(capability, "hostId").as_deref() == Some(host_id)
        });
        (executable_matches && host_matches).then(|| registry_string_field(capability, "id"))?
    })
}
pub(crate) struct BrowserPreferenceCommandInput<'a> {
    pub(crate) browser_build: &'a str,
    pub(crate) executable_id: &'a str,
    pub(crate) host_id: Option<&'a str>,
    pub(crate) capability_id: Option<&'a str>,
    pub(crate) target_service_ids: &'a [String],
    pub(crate) account_ids: &'a [String],
    pub(crate) service_name: Option<&'a str>,
    pub(crate) task_name: Option<&'a str>,
    pub(crate) reason: &'a str,
}
pub(crate) fn browser_preference_command(input: BrowserPreferenceCommandInput<'_>) -> String {
    let mut args = vec![
        "agent-browser".to_string(),
        "service".to_string(),
        "browser-capability".to_string(),
        "prefer".to_string(),
        "--browser-build".to_string(),
        input.browser_build.to_string(),
        "--preferred-executable-id".to_string(),
        input.executable_id.to_string(),
    ];
    if let Some(host_id) = input.host_id {
        args.push("--preferred-host-id".to_string());
        args.push(host_id.to_string());
    }
    if let Some(capability_id) = input.capability_id {
        args.push("--preferred-capability-id".to_string());
        args.push(capability_id.to_string());
    }
    if input.target_service_ids.is_empty()
        && input.account_ids.is_empty()
        && input.service_name.is_none()
        && input.task_name.is_none()
    {
        args.push("--target-service-id".to_string());
        args.push("<site>".to_string());
        args.push("--account-id".to_string());
        args.push("<account>".to_string());
    } else {
        for target in input.target_service_ids {
            args.push("--target-service-id".to_string());
            args.push(target.clone());
        }
        for account in input.account_ids {
            args.push("--account-id".to_string());
            args.push(account.clone());
        }
        if let Some(service_name) = input.service_name {
            args.push("--service-name".to_string());
            args.push(service_name.to_string());
        }
        if let Some(task_name) = input.task_name {
            args.push("--task-name".to_string());
            args.push(task_name.to_string());
        }
    }
    args.push("--reason".to_string());
    args.push(input.reason.to_string());
    args.into_iter()
        .map(|arg| shell_quote_command_arg(&arg))
        .collect::<Vec<_>>()
        .join(" ")
}
pub(crate) fn shell_quote_command_arg(arg: &str) -> String {
    if arg.starts_with('<') && arg.ends_with('>') {
        return arg.to_string();
    }
    if arg
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '='))
    {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}
/// Return the service-owned profile collection without the full status payload.
pub(crate) async fn handle_service_profiles(cmd: &Value) -> Result<Value, String> {
    let mut service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    service_state.refresh_profile_readiness();
    let profile_allocations = service_profile_allocations(&service_state);
    let profile_sources = service_profile_sources(&service_state);
    let mut profiles = service_state.profiles.into_values().collect::<Vec<_>>();
    profiles.sort_by(|left, right| left.id.cmp(&right.id));
    let count = profiles.len();
    Ok(json!(
        { "profiles" : profiles, "profileSources" : profile_sources,
        "profileAllocations" : profile_allocations, "count" : count, }
    ))
}
/// Resolve a profile identity query without launching or mutating a browser.
pub(crate) async fn handle_service_profile_lookup(cmd: &Value) -> Result<Value, String> {
    let mut service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    service_state.refresh_profile_readiness();
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    for (field, parameter) in [
        ("serviceName", "serviceName"),
        ("targetServiceId", "targetServiceId"),
        ("siteId", "siteId"),
        ("loginId", "loginId"),
        ("accountId", "accountId"),
        ("profileId", "profileId"),
        ("profileName", "profileName"),
        ("hostname", "hostname"),
        ("authenticationState", "authenticationState"),
        ("freshnessState", "freshnessState"),
        ("tag", "tag"),
        ("query", "query"),
        ("url", "url"),
        ("readinessProfileId", "readinessProfileId"),
        ("browserBuild", "browserBuild"),
    ] {
        if let Some(value) = cmd.get(field).and_then(Value::as_str) {
            query.append_pair(parameter, value);
        }
    }
    let query = query.finish();
    stream::service_profile_lookup_response_for_state(
        (!query.is_empty()).then_some(query.as_str()),
        &service_state,
    )
}
pub(crate) async fn handle_service_profile_seeding_handoff(cmd: &Value) -> Result<Value, String> {
    let mut service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    service_state.refresh_profile_readiness();
    let profile_id = cmd
        .get("profileId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "service_profile_seeding_handoff requires profileId".to_string())?;
    let target_service_id = cmd
        .get("targetServiceId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty());
    service_profile_seeding_handoff(&service_state, profile_id, target_service_id)
}
/// Return the service-owned session collection without the full status payload.
pub(crate) async fn handle_service_sessions(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let mut sessions = service_state.sessions.into_values().collect::<Vec<_>>();
    sessions.sort_by(|left, right| left.id.cmp(&right.id));
    let count = sessions.len();
    Ok(json!({ "sessions" : sessions, "count" : count, }))
}
/// Return the service-owned browser collection without the full status payload.
pub(crate) async fn handle_service_browsers(cmd: &Value) -> Result<Value, String> {
    let mut service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    service_state.refresh_service_tab_handles();
    let mut browsers = service_state.browsers.into_values().collect::<Vec<_>>();
    browsers.sort_by(|left, right| left.id.cmp(&right.id));
    let count = browsers.len();
    Ok(json!({ "browsers" : browsers, "count" : count, }))
}
/// Return the service-owned tab collection without the full status payload.
pub(crate) async fn handle_service_tabs(cmd: &Value) -> Result<Value, String> {
    let mut service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    service_state.refresh_service_tab_handles();
    let mut tabs = service_state.tabs.into_values().collect::<Vec<_>>();
    tabs.sort_by(|left, right| left.id.cmp(&right.id));
    let count = tabs.len();
    Ok(json!({ "tabs" : tabs, "count" : count, }))
}
/// Return the service-owned monitor collection without the full status payload.
pub(crate) async fn handle_service_monitors(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let state = optional_command_string(cmd, "monitorState")
        .map(|state| {
            parse_monitor_state(&state).ok_or_else(|| format!("Invalid monitor state: {state}"))
        })
        .transpose()?;
    let filters = MonitorCollectionFilters {
        state,
        failed_only: cmd
            .get("failedOnly")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        summary: cmd
            .get("summary")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
    };
    Ok(service_monitors_response(&service_state, filters))
}
/// Return the service-owned site-policy collection without the full status payload.
pub(crate) async fn handle_service_site_policies(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let site_policy_sources = cmd.get("sitePolicySources").cloned().unwrap_or_else(|| {
        json!(crate::native::service_model::service_site_policy_sources(
            &service_state
        ))
    });
    let mut site_policies = service_state
        .site_policies
        .into_values()
        .collect::<Vec<_>>();
    site_policies.sort_by(|left, right| left.id.cmp(&right.id));
    let count = site_policies.len();
    Ok(json!(
        { "sitePolicies" : site_policies, "sitePolicySources" : site_policy_sources,
        "count" : count, }
    ))
}
/// Return the service-owned provider collection without the full status payload.
pub(crate) async fn handle_service_providers(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let mut providers = service_state.providers.into_values().collect::<Vec<_>>();
    providers.sort_by(|left, right| left.id.cmp(&right.id));
    let count = providers.len();
    Ok(json!({ "providers" : providers, "count" : count, }))
}
/// Return the service-owned challenge collection without the full status payload.
pub(crate) async fn handle_service_challenges(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let mut challenges = service_state.challenges.into_values().collect::<Vec<_>>();
    challenges.sort_by(|left, right| left.id.cmp(&right.id));
    let count = challenges.len();
    Ok(json!({ "challenges" : challenges, "count" : count, }))
}
pub(crate) async fn handle_service_reconcile(cmd: &Value) -> Result<Value, String> {
    let mut service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let before = service_state.clone();
    let summary = reconcile_service_state(&mut service_state).await;
    let route_pool_refresh =
        refresh_authoritative_route_pool(&mut service_state, cmd.get("authoritativeRoutePool"))?;
    let reconciled_state = service_state.clone();
    let repository = LockedServiceStateRepository::default_json()?;
    persist_reconciled_service_state_in_repository(&repository, &before, &reconciled_state)?;
    Ok(json!(
        { "reconciled" : true, "browserCount" : summary.browser_count,
        "changedBrowsers" : summary.changed_browsers, "expiredSessionLeases" :
        summary.expired_session_leases.clone(), "expiredSessionLeaseCount" : summary
        .expired_session_leases.len(), "remoteViewRepair" : summary
        .remote_view_repair.to_json(), "routePoolRefresh" : route_pool_refresh,
        "service_state" : service_state, }
    ))
}
/// Refresh retained route definitions from a readiness-verified authoritative pool.
///
/// Active allocations keep their lease state. A conflicting active route is left
/// unchanged so reconciliation cannot redirect an in-use desktop.
pub(crate) fn refresh_authoritative_route_pool(
    state: &mut ServiceState,
    authoritative_route_pool: Option<&Value>,
) -> Result<Value, String> {
    let Some(authoritative_route_pool) = authoritative_route_pool else {
        return Ok(json!(
            { "requested" : false, "authoritativeEntryCount" : 0, "insertedEntryIds"
            : [], "updatedEntryIds" : [], "unchangedEntryIds" : [],
            "skippedActiveConflictEntryIds" : [], }
        ));
    };
    let values = authoritative_route_pool
        .as_array()
        .ok_or("Invalid authoritativeRoutePool: expected a JSON array of route-pool entries")?;
    let mut seen_ids = BTreeSet::new();
    let mut entries = Vec::with_capacity(values.len());
    for value in values {
        let mut entry = serde_json::from_value::<RoutePoolEntry>(value.clone())
            .map_err(|err| format!("Invalid authoritativeRoutePool entry: {}", err))?;
        if entry.id.trim().is_empty() {
            return Err("Invalid authoritativeRoutePool entry: id is required".to_string());
        }
        if entry.route_id.trim().is_empty() {
            return Err(format!(
                "Invalid authoritativeRoutePool entry '{}': routeId is required",
                entry.id
            ));
        }
        if !seen_ids.insert(entry.id.clone()) {
            return Err(format!(
                "Invalid authoritativeRoutePool: duplicate entry id '{}'",
                entry.id
            ));
        }
        entry.current_route_allocation_id = None;
        entry.state = "available".to_string();
        entries.push(entry);
    }
    let mut inserted_entry_ids = Vec::new();
    let mut updated_entry_ids = Vec::new();
    let mut unchanged_entry_ids = Vec::new();
    let mut skipped_active_conflict_entry_ids = Vec::new();
    for authoritative in entries {
        let existing = state.route_pool.get(&authoritative.id).cloned();
        let active_conflict = existing.as_ref().is_some_and(|entry| {
            matches!(entry.state.as_str(), "checked_out" | "pending")
                && entry.route_id != authoritative.route_id
        });
        if active_conflict {
            skipped_active_conflict_entry_ids.push(authoritative.id);
            continue;
        }
        let replacement = if let Some(existing) = existing.as_ref().filter(|entry| {
            matches!(entry.state.as_str(), "checked_out" | "pending")
                && entry.route_id == authoritative.route_id
        }) {
            RoutePoolEntry {
                state: existing.state.clone(),
                current_route_allocation_id: existing.current_route_allocation_id.clone(),
                ..authoritative
            }
        } else {
            authoritative
        };
        match existing {
            None => {
                inserted_entry_ids.push(replacement.id.clone());
                state.route_pool.insert(replacement.id.clone(), replacement);
            }
            Some(existing) if existing == replacement => {
                unchanged_entry_ids.push(replacement.id);
            }
            Some(_) => {
                updated_entry_ids.push(replacement.id.clone());
                state.route_pool.insert(replacement.id.clone(), replacement);
            }
        }
    }
    state.refresh_derived_views();
    Ok(json!(
        { "requested" : true, "authoritativeEntryCount" : values.len(),
        "insertedEntryIds" : inserted_entry_ids, "updatedEntryIds" :
        updated_entry_ids, "unchangedEntryIds" : unchanged_entry_ids,
        "skippedActiveConflictEntryIds" : skipped_active_conflict_entry_ids, }
    ))
}
pub(crate) async fn handle_service_browser_close(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let browser_id = cmd
        .get("browserId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or("Missing browserId")?;
    let active_browser_id = service_browser_id(&state.session_id);
    if browser_id != active_browser_id {
        return Err(format!(
            "service_browser_close can only close the active service browser {}; requested {}",
            active_browser_id, browser_id
        ));
    }
    if state.browser.is_none() {
        return Err(format!(
            "Service browser {} is not attached to this control plane",
            browser_id
        ));
    }
    let mut result = handle_close(state).await?;
    result["browserId"] = json!(browser_id);
    result["requestedBrowserId"] = json!(browser_id);
    result["serviceOwned"] = json!(true);
    Ok(result)
}
pub(crate) async fn handle_service_browser_repair(cmd: &Value) -> Result<Value, String> {
    let browser_id = cmd
        .get("browserId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or("Missing browserId")?;
    let by = cmd.get("by").and_then(Value::as_str);
    let note = cmd.get("note").and_then(Value::as_str);
    let service_name = optional_command_string(cmd, "serviceName");
    let agent_name = optional_command_string(cmd, "agentName");
    let task_name = optional_command_string(cmd, "taskName");
    let actor = normalized_operator(by);
    let note = normalized_note(note);
    let timestamp = service_now_timestamp();
    let repository = LockedServiceStateRepository::default_json().map_err(|err| {
        if err.starts_with("Failed to") || err.starts_with("Invalid service state") {
            format!("Unable to load service state: {}", err)
        } else {
            err
        }
    })?;
    let (browser, incident) = repository.mutate(|state| {
        let health = state
            .browsers
            .get(browser_id)
            .map(|browser| browser.health)
            .ok_or_else(|| format!("Service browser not found: {}", browser_id))?;
        match health {
            ServiceBrowserHealth::Faulted => retry_service_browser_in_state(
                state,
                browser_id,
                timestamp.as_str(),
                actor.as_str(),
                note.as_deref(),
                service_name.as_deref(),
                agent_name.as_deref(),
                task_name.as_deref(),
            ),
            ServiceBrowserHealth::Degraded => retry_degraded_service_browser_in_state(
                state,
                browser_id,
                timestamp.as_str(),
                actor.as_str(),
                note.as_deref(),
                service_name.as_deref(),
                agent_name.as_deref(),
                task_name.as_deref(),
            ),
            _ => Err(format!(
                "Service browser {} is not degraded or faulted; current health is {}",
                browser_id,
                service_browser_health_label(health)
            )),
        }
    })?;
    Ok(json!({ "repaired" : true, "browser" : browser, "incident" : incident, }))
}
pub(crate) fn service_browser_health_label(health: ServiceBrowserHealth) -> String {
    serde_json::to_value(health)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct ServiceRetentionPruneOptions {
    pub(crate) apply: bool,
    pub(crate) closed_tabs: bool,
    pub(crate) not_started_browsers: bool,
    pub(crate) process_exited_browsers: bool,
    pub(crate) released_sessions: bool,
    pub(crate) abandoned_sessions: bool,
    pub(crate) orphaned_profiles: bool,
    pub(crate) display_allocations: bool,
    pub(crate) abandoned_session_min_age_minutes: u64,
}
impl ServiceRetentionPruneOptions {
    pub(crate) fn from_command(cmd: &Value) -> Self {
        Self {
            apply: cmd.get("apply").and_then(Value::as_bool).unwrap_or(false),
            closed_tabs: cmd
                .get("closedTabs")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            not_started_browsers: cmd
                .get("notStartedBrowsers")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            process_exited_browsers: cmd
                .get("processExitedBrowsers")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            released_sessions: cmd
                .get("releasedSessions")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            abandoned_sessions: cmd
                .get("abandonedSessions")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            orphaned_profiles: cmd
                .get("orphanedProfiles")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            display_allocations: cmd
                .get("displayAllocations")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            abandoned_session_min_age_minutes: cmd
                .get("abandonedSessionMinAgeMinutes")
                .and_then(Value::as_u64)
                .unwrap_or(1440),
        }
    }
}
pub(crate) async fn handle_service_prune_retained(cmd: &Value) -> Result<Value, String> {
    let options = ServiceRetentionPruneOptions::from_command(cmd);
    if options.apply {
        let repository = LockedServiceStateRepository::default_json()?;
        repository.mutate(|state| Ok(prune_retained_service_state(state, options)))
    } else {
        let mut service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        Ok(prune_retained_service_state(&mut service_state, options))
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct ServiceRetentionRepairOptions {
    pub(crate) apply: bool,
    pub(crate) missing_lease_observed_at: bool,
}
impl ServiceRetentionRepairOptions {
    pub(crate) fn from_command(cmd: &Value) -> Self {
        Self {
            apply: cmd.get("apply").and_then(Value::as_bool).unwrap_or(false),
            missing_lease_observed_at: cmd
                .get("missingLeaseObservedAt")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        }
    }
}
pub(crate) async fn handle_service_repair_retained(cmd: &Value) -> Result<Value, String> {
    let options = ServiceRetentionRepairOptions::from_command(cmd);
    let observed_at = chrono::Utc::now().to_rfc3339();
    if options.apply {
        let repository = LockedServiceStateRepository::default_json()?;
        repository.mutate(|state| {
            Ok(repair_retained_service_state(
                state,
                options,
                observed_at.as_str(),
            ))
        })
    } else {
        let mut service_state = cmd
            .get("serviceState")
            .cloned()
            .map(serde_json::from_value::<ServiceState>)
            .transpose()
            .map_err(|err| format!("Invalid serviceState: {}", err))?
            .unwrap_or_default();
        Ok(repair_retained_service_state(
            &mut service_state,
            options,
            observed_at.as_str(),
        ))
    }
}
pub(crate) fn repair_retained_service_state(
    state: &mut ServiceState,
    options: ServiceRetentionRepairOptions,
    observed_at: &str,
) -> Value {
    let before_session_count = state.sessions.len();
    let mut missing_lease_observed_at = Vec::new();
    let mut skipped = Vec::new();
    if options.missing_lease_observed_at {
        for session in state.sessions.values() {
            if session_has_parseable_age_evidence(session) {
                continue;
            }
            if matches!(session.lease, LeaseState::Released | LeaseState::Expired)
                || !legacy_inert_session_placeholder(state, session)
            {
                skipped.push(session.id.clone());
                continue;
            }
            missing_lease_observed_at.push(session.id.clone());
        }
    }
    missing_lease_observed_at.sort();
    skipped.sort();
    let repaired_count = missing_lease_observed_at.len();
    if options.apply {
        for session_id in &missing_lease_observed_at {
            if let Some(session) = state.sessions.get_mut(session_id) {
                session.last_lease_observed_at = Some(observed_at.to_string());
            }
        }
    }
    json!(
        { "repaired" : options.apply, "dryRun" : ! options.apply, "observedAt" :
        observed_at, "policy" : { "missingLeaseObservedAt" : options
        .missing_lease_observed_at, "requiresInertSessionPlaceholder" : true,
        "excludesReleasedOrExpiredSessions" : true, "stampSource" :
        "currentObservationTime", }, "before" : { "sessionCount" : before_session_count,
        }, "candidates" : { "missingLeaseObservedAt" : missing_lease_observed_at, },
        "candidateCounts" : { "missingLeaseObservedAt" : repaired_count, "total" :
        repaired_count, }, "skipped" : { "missingLeaseObservedAt" : skipped, },
        "skippedCounts" : { "missingLeaseObservedAt" : skipped.len(), }, "repairedCounts"
        : { "missingLeaseObservedAt" : if options.apply { repaired_count } else { 0 },
        "total" : if options.apply { repaired_count } else { 0 }, }, "after" : {
        "sessionCount" : state.sessions.len(), }, "recommendedNextStep" : if options
        .apply {
        "Run agent-browser service prune-retained --abandoned-sessions as a dry-run; repaired sessions should now be too fresh until the minimum age guard elapses."
        } else {
        "Review candidates, then rerun with --apply to stamp current observation time onto safe legacy placeholders."
        }, }
    )
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct ServiceRoutePoolRepairOptions {
    pub(crate) apply: bool,
    pub(crate) stale_checkouts: bool,
    pub(crate) stale_pending_acquisitions: bool,
}
impl ServiceRoutePoolRepairOptions {
    pub(crate) fn from_command(cmd: &Value) -> Self {
        Self {
            apply: cmd.get("apply").and_then(Value::as_bool).unwrap_or(false),
            stale_checkouts: cmd
                .get("staleCheckouts")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            stale_pending_acquisitions: cmd
                .get("stalePendingAcquisitions")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        }
    }
}
pub(crate) async fn handle_service_route_pool_repair(cmd: &Value) -> Result<Value, String> {
    let options = ServiceRoutePoolRepairOptions::from_command(cmd);
    let observed_at = chrono::Utc::now().to_rfc3339();
    if options.apply {
        let repository = LockedServiceStateRepository::default_json()?;
        repository.mutate(|state| {
            Ok(repair_route_pool_service_state(
                state,
                options,
                observed_at.as_str(),
            ))
        })
    } else {
        let mut service_state = if let Some(service_state) = cmd.get("serviceState") {
            serde_json::from_value::<ServiceState>(service_state.clone())
                .map_err(|err| format!("Invalid serviceState: {}", err))?
        } else {
            LockedServiceStateRepository::default_json()?.load_snapshot()?
        };
        Ok(repair_route_pool_service_state(
            &mut service_state,
            options,
            observed_at.as_str(),
        ))
    }
}
pub(crate) fn repair_route_pool_service_state(
    state: &mut ServiceState,
    options: ServiceRoutePoolRepairOptions,
    observed_at: &str,
) -> Value {
    let before_route_pool_count = state.route_pool.len();
    let mut stale_checkouts = Vec::new();
    let mut stale_checkout_reasons = serde_json::Map::new();
    let mut skipped_active_checkouts = Vec::new();
    let mut stale_pending_acquisitions = Vec::new();
    let mut stale_pending_acquisition_reasons = serde_json::Map::new();
    let mut stale_route_ids = BTreeSet::new();
    let mut stale_display_allocation_ids = BTreeSet::new();
    if options.stale_pending_acquisitions {
        for lease in state.remote_view_acquisition_leases.values() {
            if lease.state != "pending" {
                continue;
            }
            match pending_acquisition_stale_reason(state, lease) {
                Some(reason) => {
                    stale_pending_acquisitions.push(lease.id.clone());
                    stale_pending_acquisition_reasons.insert(
                        lease.id.clone(),
                        json!(
                            { "reason" : reason, "routeId" : lease.route_id,
                            "displayAllocationId" : lease.display_allocation_id,
                            "routePoolEntryId" : lease.route_pool_entry_id, }
                        ),
                    );
                    stale_route_ids.insert(lease.route_id.clone());
                    stale_display_allocation_ids.insert(lease.display_allocation_id.clone());
                }
                None => skipped_active_checkouts.push(lease.id.clone()),
            }
        }
    }
    if options.stale_checkouts {
        for entry in state.route_pool.values() {
            if entry.state != "checked_out" {
                continue;
            }
            let Some(route_id) = entry.current_route_allocation_id.as_deref() else {
                stale_checkouts.push(entry.id.clone());
                stale_checkout_reasons.insert(
                    entry.id.clone(),
                    json!({ "reason" : "missing_current_route_allocation_id", }),
                );
                continue;
            };
            match route_pool_checkout_stale_reason(state, route_id) {
                Some(reason) => {
                    stale_checkouts.push(entry.id.clone());
                    if state.remote_view_routes.contains_key(route_id) {
                        stale_route_ids.insert(route_id.to_string());
                    }
                    stale_checkout_reasons.insert(
                        entry.id.clone(),
                        json!({ "reason" : reason, "routeId" : route_id, }),
                    );
                }
                None => skipped_active_checkouts.push(entry.id.clone()),
            }
        }
    }
    let stale_route_id_set = stale_route_ids.clone();
    for route_id in &stale_route_id_set {
        if let Some(display_allocation_id) = state
            .remote_view_routes
            .get(route_id)
            .and_then(|route| route.display_allocation_id.clone())
        {
            let referenced_by_active_route = state.remote_view_routes.iter().any(|(id, route)| {
                !stale_route_id_set.contains(id)
                    && route.display_allocation_id.as_deref()
                        == Some(display_allocation_id.as_str())
                    && matches!(
                        route.state.as_str(),
                        "ready" | "allocating" | "reconnecting"
                    )
            });
            if !referenced_by_active_route {
                stale_display_allocation_ids.insert(display_allocation_id);
            }
        }
    }
    stale_checkouts.sort();
    stale_pending_acquisitions.sort();
    skipped_active_checkouts.sort();
    let stale_routes = stale_route_ids.into_iter().collect::<Vec<_>>();
    let stale_display_allocations = stale_display_allocation_ids.into_iter().collect::<Vec<_>>();
    let repaired_count = stale_checkouts.len();
    let repaired_pending_count = stale_pending_acquisitions.len();
    let released_route_count = stale_routes.len();
    let released_display_allocation_count = stale_display_allocations.len();
    if options.apply {
        for lease_id in &stale_pending_acquisitions {
            if let Some(lease_snapshot) =
                state.remote_view_acquisition_leases.get(lease_id).cloned()
            {
                match lease_snapshot.previous_route_pool_entry.clone() {
                    Some(entry) => {
                        state.route_pool.insert(entry.id.clone(), entry);
                    }
                    None => {
                        if let Some(id) = lease_snapshot.route_pool_entry_id.as_ref() {
                            state.route_pool.remove(id);
                        }
                    }
                }
                match lease_snapshot.previous_display_allocation.clone() {
                    Some(allocation) => {
                        state
                            .display_allocations
                            .insert(allocation.id.clone(), allocation);
                    }
                    None => {
                        state
                            .display_allocations
                            .remove(&lease_snapshot.display_allocation_id);
                    }
                }
                match lease_snapshot.previous_remote_view_route.clone() {
                    Some(route) => {
                        state.remote_view_routes.insert(route.id.clone(), route);
                    }
                    None => {
                        state.remote_view_routes.remove(&lease_snapshot.route_id);
                    }
                }
                if let Some(browser) = state.browsers.get_mut(&lease_snapshot.browser_id) {
                    browser.display_allocation_id = lease_snapshot
                        .previous_browser_display_allocation_id
                        .clone();
                }
                let reason = stale_pending_acquisition_reasons
                    .get(lease_id)
                    .and_then(|value| value.get("reason"))
                    .and_then(Value::as_str)
                    .unwrap_or("stale_pending_acquisition");
                let rollback = json!(
                    { "state" : "rolled_back", "leaseId" : lease_id, "phase" :
                    "stale_pending_acquisition_repair", "routeId" : lease_snapshot
                    .route_id, "displayAllocationId" : lease_snapshot
                    .display_allocation_id, "routePoolEntryId" : lease_snapshot
                    .route_pool_entry_id, "restoredRoutePoolEntry" : lease_snapshot
                    .previous_route_pool_entry.is_some(), "restoredDisplayAllocation" :
                    lease_snapshot.previous_display_allocation.is_some(),
                    "restoredRemoteViewRoute" : lease_snapshot.previous_remote_view_route
                    .is_some(), "restoredBrowserDisplayAllocation" : lease_snapshot
                    .previous_browser_display_allocation_id, "cleanup" : { "state" :
                    "stale_pending_acquisition_repaired", "reason" : reason, },
                    "updatedAt" : observed_at, }
                );
                if let Some(lease) = state.remote_view_acquisition_leases.get_mut(lease_id) {
                    lease.state = "failed".to_string();
                    lease.phase = "rollback_complete".to_string();
                    lease.updated_at = Some(observed_at.to_string());
                    lease.failed_at = Some(observed_at.to_string());
                    lease.failure_reason =
                        Some(format!("stale_pending_acquisition_repair: {reason}"));
                    lease.cleanup = Some(rollback);
                }
            }
        }
        for entry_id in &stale_checkouts {
            if let Some(entry) = state.route_pool.get_mut(entry_id) {
                let previous_route_allocation_id = entry.current_route_allocation_id.clone();
                let reason = stale_checkout_reasons
                    .get(entry_id)
                    .and_then(|value| value.get("reason"))
                    .and_then(Value::as_str)
                    .unwrap_or("stale_route_pool_checkout");
                entry.state = "available".to_string();
                entry.current_route_allocation_id = None;
                entry.readiness = Some(json!(
                    { "state" : "ready", "reason" :
                    "stale_route_pool_checkout_repaired", "staleReason" : reason,
                    "previousRouteAllocationId" : previous_route_allocation_id,
                    "updatedAt" : observed_at, }
                ));
            }
        }
        for route_id in &stale_routes {
            if let Some(route) = state.remote_view_routes.get_mut(route_id) {
                let previous_state = route.state.clone();
                route.state = "released".to_string();
                route.readiness = Some(json!(
                    { "state" : "released", "reason" :
                    "stale_route_pool_checkout_repaired", "previousState" :
                    previous_state, "updatedAt" : observed_at, }
                ));
            }
        }
        for display_allocation_id in &stale_display_allocations {
            if let Some(allocation) = state.display_allocations.get_mut(display_allocation_id) {
                let previous_state = allocation.state.clone();
                allocation.state = "released".to_string();
                allocation.updated_at = Some(observed_at.to_string());
                allocation.readiness = Some(json!(
                    { "state" : "released", "reason" :
                    "stale_route_pool_checkout_repaired", "previousState" :
                    previous_state, "updatedAt" : observed_at, }
                ));
            }
        }
    }
    let repaired_total = if options.apply {
        repaired_pending_count
            + repaired_count
            + released_route_count
            + released_display_allocation_count
    } else {
        0
    };
    json!(
        { "repaired" : options.apply, "dryRun" : ! options.apply, "observedAt" :
        observed_at, "policy" : { "staleCheckouts" : options.stale_checkouts,
        "stalePendingAcquisitions" : options.stale_pending_acquisitions,
        "repairsCheckedOutEntriesOnly" : false, "preservesActiveReadyRoutes" : true, },
        "before" : { "routePoolEntryCount" : before_route_pool_count, }, "candidates" : {
        "stalePendingAcquisitions" : stale_pending_acquisitions, "staleCheckouts" :
        stale_checkouts, "staleRoutes" : stale_routes, "staleDisplayAllocations" :
        stale_display_allocations, }, "candidateReasons" : { "staleCheckouts" :
        stale_checkout_reasons, "stalePendingAcquisitions" :
        stale_pending_acquisition_reasons, }, "candidateCounts" : {
        "stalePendingAcquisitions" : repaired_pending_count, "staleCheckouts" :
        repaired_count, "staleRoutes" : released_route_count, "staleDisplayAllocations" :
        released_display_allocation_count, "total" : repaired_pending_count +
        repaired_count + released_route_count + released_display_allocation_count, },
        "skipped" : { "activeCheckouts" : skipped_active_checkouts, }, "skippedCounts" :
        { "activeCheckouts" : skipped_active_checkouts.len(), }, "repairedCounts" : {
        "stalePendingAcquisitions" : if options.apply { repaired_pending_count } else { 0
        }, "staleCheckouts" : if options.apply { repaired_count } else { 0 },
        "staleRoutes" : if options.apply { released_route_count } else { 0 },
        "staleDisplayAllocations" : if options.apply { released_display_allocation_count
        } else { 0 }, "total" : repaired_total, }, "after" : { "routePoolEntryCount" :
        state.route_pool.len(), },
        "recommendedNextStep" : if options.apply {
        "Run service_remote_view_route_checkout for the intended display allocations, then run service_reconcile to refresh derived remote-view incidents."
        } else {
        "Review stale checkout candidates, then rerun with apply=true to return those route-pool entries to available state."
        }, }
    )
}
pub(crate) fn pending_acquisition_stale_reason(
    state: &ServiceState,
    lease: &RemoteViewAcquisitionLease,
) -> Option<&'static str> {
    let route_pending = state
        .remote_view_routes
        .get(&lease.route_id)
        .map(|route| route.state == "pending")
        .unwrap_or(false);
    let display_pending = state
        .display_allocations
        .get(&lease.display_allocation_id)
        .map(|allocation| allocation.state == "pending")
        .unwrap_or(false);
    let pool_pending = lease.route_pool_entry_id.as_ref().is_some_and(|entry_id| {
        state
            .route_pool
            .get(entry_id)
            .map(|entry| {
                entry.state == "pending"
                    && entry.current_route_allocation_id.as_deref() == Some(lease.route_id.as_str())
            })
            .unwrap_or(false)
    });
    let browser_ready = state
        .browsers
        .get(&lease.browser_id)
        .map(|browser| browser.health == ServiceBrowserHealth::Ready)
        .unwrap_or(false);
    if !browser_ready && (route_pending || display_pending || pool_pending) {
        return Some("pending_acquisition_without_ready_browser");
    }
    None
}
pub(crate) fn route_pool_checkout_stale_reason(
    state: &ServiceState,
    route_id: &str,
) -> Option<&'static str> {
    let Some(route) = state.remote_view_routes.get(route_id) else {
        return Some("route_missing");
    };
    if matches!(
        route.state.as_str(),
        "released" | "orphaned" | "failed" | "unavailable"
    ) {
        return Some("route_not_active");
    }
    if let Some(display_allocation_id) = route.display_allocation_id.as_deref() {
        match state.display_allocations.get(display_allocation_id) {
            Some(allocation) if matches!(allocation.state.as_str(), "ready" | "allocating") => {}
            Some(_) => return Some("display_allocation_not_active"),
            None => return Some("display_allocation_missing"),
        }
    }
    if let Some(browser_id) = route.browser_id.as_deref() {
        match state.browsers.get(browser_id) {
            Some(browser) if browser.health == ServiceBrowserHealth::Ready => {}
            Some(_) => return Some("browser_not_ready"),
            None => return Some("browser_missing"),
        }
    }
    None
}
pub(crate) fn prune_retained_service_state(
    state: &mut ServiceState,
    options: ServiceRetentionPruneOptions,
) -> Value {
    let before_profile_count = state.profiles.len();
    let before_browser_count = state.browsers.len();
    let before_tab_count = state.tabs.len();
    let before_session_count = state.sessions.len();
    let before_display_allocation_count = state.display_allocations.len();
    let closed_tab_ids = if options.closed_tabs {
        state
            .tabs
            .iter()
            .filter(|(_, tab)| tab.lifecycle == TabLifecycle::Closed)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut skipped_abandoned_sessions_missing_age_timestamp = Vec::new();
    let mut skipped_abandoned_sessions_too_fresh = Vec::new();
    let session_ids = state
        .sessions
        .iter()
        .filter(|(_, session)| {
            let released_lease_matches = options.released_sessions
                && matches!(session.lease, LeaseState::Released | LeaseState::Expired);
            let abandoned_age_status = abandoned_session_age_status(
                session
                    .last_lease_observed_at
                    .as_deref()
                    .or(session.created_at.as_deref()),
                options.abandoned_session_min_age_minutes,
            );
            let abandoned_lease_matches = options.abandoned_sessions
                && !matches!(session.lease, LeaseState::Released | LeaseState::Expired)
                && matches!(abandoned_age_status, SessionAgeStatus::OldEnough);
            let lease_matches = released_lease_matches || abandoned_lease_matches;
            let session_shape_matches = session.tab_ids.is_empty()
                && !session.browser_ids.is_empty()
                && session.browser_ids.iter().all(|browser_id| {
                    prunable_session_browser_placeholder(
                        state,
                        browser_id,
                        session.id.as_str(),
                        options.process_exited_browsers,
                    )
                });
            if options.abandoned_sessions
                && !matches!(session.lease, LeaseState::Released | LeaseState::Expired)
                && session_shape_matches
                && !abandoned_lease_matches
            {
                match abandoned_age_status {
                    SessionAgeStatus::MissingOrInvalid => {
                        skipped_abandoned_sessions_missing_age_timestamp.push(session.id.clone())
                    }
                    SessionAgeStatus::TooFresh => {
                        skipped_abandoned_sessions_too_fresh.push(session.id.clone())
                    }
                    SessionAgeStatus::OldEnough => {}
                }
            }
            lease_matches && session_shape_matches
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let pruned_session_ids = session_ids.iter().cloned().collect::<HashSet<_>>();
    let browser_ids = state
        .browsers
        .iter()
        .filter(|(id, browser)| {
            let not_started_matches = options.not_started_browsers
                && retained_not_started_browser_placeholder(state, id, browser);
            let process_exited_matches = options.process_exited_browsers
                && retained_failed_browser_placeholder(state, id, browser);
            (not_started_matches || process_exited_matches)
                && browser
                    .active_session_ids
                    .iter()
                    .all(|session_id| !state.sessions.contains_key(session_id))
        })
        .map(|(id, _)| id.clone())
        .chain(session_ids.iter().flat_map(|session_id| {
            state
                .sessions
                .get(session_id)
                .map(|session| session.browser_ids.clone())
                .unwrap_or_default()
        }))
        .collect::<Vec<_>>();
    let browser_ids = browser_ids.into_iter().collect::<HashSet<_>>();
    let mut browser_ids = browser_ids.into_iter().collect::<Vec<_>>();
    browser_ids.sort();
    let referenced_profile_ids = referenced_service_profile_ids(state);
    let mut orphaned_profile_reasons = serde_json::Map::new();
    let mut profile_ids = if options.orphaned_profiles {
        state
            .profiles
            .iter()
            .filter_map(|(profile_id, profile)| {
                orphaned_profile_prune_reason(profile_id, profile, &referenced_profile_ids).map(
                    |reason| {
                        orphaned_profile_reasons.insert(
                            profile_id.clone(),
                            json!(
                                { "reason" : reason, "userDataDir" : profile.user_data_dir,
                                }
                            ),
                        );
                        profile_id.clone()
                    },
                )
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    profile_ids.sort();
    let display_allocation_candidates = if options.display_allocations {
        retained_display_allocation_candidates(state)
    } else {
        Vec::new()
    };
    let display_allocation_ids = display_allocation_candidates
        .iter()
        .filter(|candidate| candidate.apply_safe)
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    let mut display_allocation_reasons = serde_json::Map::new();
    let mut display_allocation_class_counts = BTreeMap::new();
    for candidate in &display_allocation_candidates {
        *display_allocation_class_counts
            .entry(candidate.class_name)
            .or_insert(0usize) += 1;
        display_allocation_reasons.insert(candidate.id.clone(), candidate.to_json());
    }
    let skipped_abandoned_sessions_missing_age_timestamp_count =
        skipped_abandoned_sessions_missing_age_timestamp.len();
    let skipped_abandoned_sessions_too_fresh_count = skipped_abandoned_sessions_too_fresh.len();
    let skipped_abandoned_sessions_missing_age_timestamp_summary =
        summarize_skipped_session_groups(&skipped_abandoned_sessions_missing_age_timestamp);
    let skipped_abandoned_sessions_too_fresh_summary =
        summarize_skipped_session_groups(&skipped_abandoned_sessions_too_fresh);
    let mut session_tab_refs_removed = 0usize;
    let mut session_browser_refs_removed = 0usize;
    if options.apply {
        for session_id in &session_ids {
            state.sessions.remove(session_id);
        }
        for tab_id in &closed_tab_ids {
            state.tabs.remove(tab_id);
        }
        for browser_id in &browser_ids {
            state.browsers.remove(browser_id);
        }
        for profile_id in &profile_ids {
            state.profiles.remove(profile_id);
            state.entity_sources.profiles.remove(profile_id);
        }
        for display_allocation_id in &display_allocation_ids {
            state.display_allocations.remove(display_allocation_id);
        }
        for session in state.sessions.values_mut() {
            let before = session.tab_ids.len();
            session
                .tab_ids
                .retain(|tab_id| !closed_tab_ids.contains(tab_id));
            session_tab_refs_removed += before.saturating_sub(session.tab_ids.len());
            let before = session.browser_ids.len();
            session
                .browser_ids
                .retain(|browser_id| !browser_ids.contains(browser_id));
            session_browser_refs_removed += before.saturating_sub(session.browser_ids.len());
        }
        for browser in state.browsers.values_mut() {
            let before = browser.active_session_ids.len();
            browser
                .active_session_ids
                .retain(|session_id| !pruned_session_ids.contains(session_id));
            session_browser_refs_removed += before.saturating_sub(browser.active_session_ids.len());
        }
        state.refresh_derived_views();
    }
    json!(
        { "pruned" : options.apply, "dryRun" : ! options.apply, "policy" : { "closedTabs"
        : options.closed_tabs, "notStartedBrowsers" : options.not_started_browsers,
        "processExitedBrowsers" : options.process_exited_browsers,
        "processExitedBrowsersIncludesUnreachable" : true,
        "processExitedBrowsersIncludesFaultedPlaceholders" : true,
        "releasedSessionPruneRemovesRetainedViewStreams" : true, "releasedSessions" :
        options.released_sessions, "abandonedSessions" : options.abandoned_sessions,
        "orphanedProfiles" : options.orphaned_profiles, "displayAllocations" : options
        .display_allocations, "abandonedSessionMinAgeMinutes" : options
        .abandoned_session_min_age_minutes, "processExitedRequiresExplicitFlag" : true,
        "abandonedSessionsRequiresExplicitFlag" : true,
        "abandonedSessionsRequireAgeTimestamp" : true, "abandonedSessionAgeSource" :
        "lastLeaseObservedAtOrCreatedAt", "orphanedProfilesRequiresExplicitFlag" : true,
        "orphanedProfilesScope" :
        "customProfilesWithMissingEphemeralUserDataDirOrManagedOneTimeWithoutRetainedReferences",
        "displayAllocationsRequiresExplicitFlag" : true,
        "displayAllocationsApplyRequiresApplySafeClassification" : true, }, "before" : {
        "profileCount" : before_profile_count, "browserCount" : before_browser_count,
        "tabCount" : before_tab_count, "sessionCount" : before_session_count,
        "displayAllocationCount" : before_display_allocation_count, }, "candidates" : {
        "closedTabs" : closed_tab_ids, "browsers" : browser_ids, "sessions" :
        session_ids, "orphanedProfiles" : profile_ids, "displayAllocations" :
        display_allocation_ids, }, "candidateReasons" : { "orphanedProfiles" :
        orphaned_profile_reasons, "displayAllocations" : display_allocation_reasons, },
        "candidateClassCounts" : { "displayAllocations" :
        display_allocation_class_counts, }, "candidateCounts" : { "closedTabs" :
        closed_tab_ids.len(), "browsers" : browser_ids.len(), "sessions" : session_ids
        .len(), "orphanedProfiles" : profile_ids.len(), "displayAllocations" :
        display_allocation_ids.len(), "total" : closed_tab_ids.len() + browser_ids.len()
        + session_ids.len() + profile_ids.len() + display_allocation_ids.len(), },
        "skipped" : { "abandonedSessionsMissingAgeTimestamp" :
        skipped_abandoned_sessions_missing_age_timestamp, "abandonedSessionsTooFresh" :
        skipped_abandoned_sessions_too_fresh, }, "skippedCounts" : {
        "abandonedSessionsMissingAgeTimestamp" :
        skipped_abandoned_sessions_missing_age_timestamp_count,
        "abandonedSessionsTooFresh" : skipped_abandoned_sessions_too_fresh_count, },
        "skippedSummary" : { "abandonedSessionsMissingAgeTimestamp" :
        skipped_abandoned_sessions_missing_age_timestamp_summary,
        "abandonedSessionsTooFresh" : skipped_abandoned_sessions_too_fresh_summary, },
        "removed" : { "closedTabs" : if options.apply { closed_tab_ids.len() } else { 0
        }, "browsers" : if options.apply { browser_ids.len() } else { 0 }, "sessions" :
        if options.apply { session_ids.len() } else { 0 }, "orphanedProfiles" : if
        options.apply { profile_ids.len() } else { 0 }, "displayAllocations" : if options
        .apply { display_allocation_ids.len() } else { 0 }, "sessionTabRefs" :
        session_tab_refs_removed, "sessionBrowserRefs" : session_browser_refs_removed, },
        "after" : { "profileCount" : state.profiles.len(), "browserCount" : state
        .browsers.len(), "tabCount" : state.tabs.len(), "sessionCount" : state.sessions
        .len(), "displayAllocationCount" : state.display_allocations.len(), },
        "recommendedNextStep" : if options.apply {
        "Run agent-browser service reconcile and inspect agent-browser service status." }
        else {
        "Review the candidates, then rerun with --apply when the retained records are safe to remove."
        }, }
    )
}
pub(crate) fn referenced_service_profile_ids(state: &ServiceState) -> HashSet<String> {
    let mut profile_ids = HashSet::new();
    for browser in state.browsers.values() {
        if let Some(profile_id) = browser.profile_id.as_deref().filter(|id| !id.is_empty()) {
            profile_ids.insert(profile_id.to_string());
        }
    }
    for session in state.sessions.values() {
        if let Some(profile_id) = session.profile_id.as_deref().filter(|id| !id.is_empty()) {
            profile_ids.insert(profile_id.to_string());
        }
        if let Some(value) = session.browser_capability_launch.as_ref() {
            collect_profile_ids_from_json(value, &mut profile_ids);
        }
    }
    for allocation in state.display_allocations.values() {
        if let Some(profile_id) = allocation.profile_id.as_deref().filter(|id| !id.is_empty()) {
            profile_ids.insert(profile_id.to_string());
        }
        if let Some(value) = allocation.readiness.as_ref() {
            collect_profile_ids_from_json(value, &mut profile_ids);
        }
    }
    for event in &state.events {
        if let Some(profile_id) = event.profile_id.as_deref().filter(|id| !id.is_empty()) {
            profile_ids.insert(profile_id.to_string());
        }
        if let Some(value) = event.details.as_ref() {
            collect_profile_ids_from_json(value, &mut profile_ids);
        }
    }
    for job in state.jobs.values() {
        if let Some(value) = job.result.as_ref() {
            collect_profile_ids_from_json(value, &mut profile_ids);
        }
    }
    for handoff in state.profile_seeding_handoffs.values() {
        if !handoff.profile_id.is_empty() {
            profile_ids.insert(handoff.profile_id.clone());
        }
    }
    profile_ids
}
pub(crate) fn collect_profile_ids_from_json(value: &Value, profile_ids: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            for key in ["profileId", "profile_id"] {
                if let Some(profile_id) = map.get(key).and_then(Value::as_str) {
                    if !profile_id.is_empty() {
                        profile_ids.insert(profile_id.to_string());
                    }
                }
            }
            for value in map.values() {
                collect_profile_ids_from_json(value, profile_ids);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_profile_ids_from_json(value, profile_ids);
            }
        }
        _ => {}
    }
}
pub(crate) fn orphaned_profile_prune_reason(
    profile_id: &str,
    profile: &BrowserProfile,
    referenced_profile_ids: &HashSet<String>,
) -> Option<&'static str> {
    if matches!(
        profile.profile_origin,
        ProfileOrigin::ExternalByop | ProfileOrigin::ExternalObserved
    ) {
        return None;
    }
    if referenced_profile_ids.contains(profile_id) {
        return None;
    }
    if profile.profile_class == ProfileClass::ManagedOneTime && !profile.persistent {
        return Some("managed_one_time_unreferenced");
    }
    if !profile_id.starts_with("custom:") {
        return None;
    }
    if !profile.site_policy_ids.is_empty()
        || !profile.target_service_ids.is_empty()
        || !profile.authenticated_service_ids.is_empty()
        || !profile.account_ids.is_empty()
        || !profile.shared_service_ids.is_empty()
        || !profile.credential_provider_ids.is_empty()
        || !profile.target_readiness.is_empty()
    {
        return None;
    }
    let user_data_dir = profile.user_data_dir.as_deref()?;
    let path = Path::new(user_data_dir);
    if !is_ephemeral_profile_path(path) || path.exists() {
        return None;
    }
    Some("missing_ephemeral_user_data_dir")
}
pub(crate) fn is_ephemeral_profile_path(path: &Path) -> bool {
    if path.starts_with("/tmp") || path.starts_with("/var/tmp") {
        return true;
    }
    let path_text = path.to_string_lossy();
    path_text.contains("/AppData/Local/Temp/")
        || path_text.contains("\\AppData\\Local\\Temp\\")
        || (path_text.contains("/workspace.local/") && path_text.contains("/tmp/"))
        || (path_text.contains("/.local/state/")
            && (path_text.contains("/browser-smokes/") || path_text.contains("/ui-audits/")))
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionAgeStatus {
    OldEnough,
    TooFresh,
    MissingOrInvalid,
}
pub(crate) fn abandoned_session_age_status(
    created_at: Option<&str>,
    min_age_minutes: u64,
) -> SessionAgeStatus {
    let Some(created_at) = created_at else {
        return SessionAgeStatus::MissingOrInvalid;
    };
    let Ok(created_at) = DateTime::parse_from_rfc3339(created_at) else {
        return SessionAgeStatus::MissingOrInvalid;
    };
    let Ok(min_age_minutes) = i64::try_from(min_age_minutes) else {
        return SessionAgeStatus::MissingOrInvalid;
    };
    let threshold = chrono::Utc::now() - chrono::Duration::minutes(min_age_minutes);
    if created_at.with_timezone(&chrono::Utc) <= threshold {
        SessionAgeStatus::OldEnough
    } else {
        SessionAgeStatus::TooFresh
    }
}
pub(crate) fn summarize_skipped_session_groups(session_ids: &[String]) -> Value {
    let mut groups = BTreeMap::<String, Vec<String>>::new();
    for session_id in session_ids {
        groups
            .entry(skipped_session_group(session_id))
            .or_default()
            .push(session_id.clone());
    }
    let group_count = groups.len();
    let mut groups = groups
        .into_iter()
        .map(|(group, mut ids)| {
            ids.sort();
            json!(
                { "group" : group, "count" : ids.len(), "examples" : ids.into_iter()
                .take(3).collect::< Vec < _ >> (), }
            )
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        let left_count = left
            .get("count")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let right_count = right
            .get("count")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        right_count.cmp(&left_count).then_with(|| {
            left.get("group")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .cmp(
                    right
                        .get("group")
                        .and_then(|value| value.as_str())
                        .unwrap_or(""),
                )
        })
    });
    let omitted_group_count = group_count.saturating_sub(10);
    groups.truncate(10);
    json!(
        { "groupCount" : group_count, "omittedGroupCount" : omitted_group_count, "groups"
        : groups, }
    )
}
pub(crate) fn skipped_session_group(session_id: &str) -> String {
    let Some((prefix, suffix)) = session_id.rsplit_once('-') else {
        return session_id.to_string();
    };
    if !prefix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) {
        prefix.to_string()
    } else {
        session_id.to_string()
    }
}
pub(crate) fn session_has_parseable_age_evidence(session: &BrowserSession) -> bool {
    session
        .last_lease_observed_at
        .as_deref()
        .or(session.created_at.as_deref())
        .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
        .is_some()
}
pub(crate) fn legacy_inert_session_placeholder(
    state: &ServiceState,
    session: &BrowserSession,
) -> bool {
    session.tab_ids.is_empty()
        && !session.browser_ids.is_empty()
        && session.browser_ids.iter().all(|browser_id| {
            inert_session_browser_placeholder(state, browser_id, session.id.as_str())
        })
}
pub(crate) fn prunable_session_browser_placeholder(
    state: &ServiceState,
    browser_id: &str,
    session_id: &str,
    allow_failed_retained: bool,
) -> bool {
    inert_session_browser_placeholder(state, browser_id, session_id)
        || (allow_failed_retained
            && failed_retained_session_browser_placeholder(state, browser_id, session_id))
}
pub(crate) fn inert_session_browser_placeholder(
    state: &ServiceState,
    browser_id: &str,
    session_id: &str,
) -> bool {
    let Some(browser) = state.browsers.get(browser_id) else {
        return false;
    };
    retained_not_started_browser_placeholder(state, browser_id, browser)
        && (browser.active_session_ids.is_empty()
            || browser.active_session_ids == vec![session_id.to_string()])
}
pub(crate) fn failed_retained_session_browser_placeholder(
    state: &ServiceState,
    browser_id: &str,
    session_id: &str,
) -> bool {
    let Some(browser) = state.browsers.get(browser_id) else {
        return false;
    };
    retained_failed_browser_placeholder(state, browser_id, browser)
        && (browser.active_session_ids.is_empty()
            || browser.active_session_ids == vec![session_id.to_string()])
}
pub(crate) fn failed_retained_browser_health(health: ServiceBrowserHealth) -> bool {
    matches!(
        health,
        ServiceBrowserHealth::ProcessExited
            | ServiceBrowserHealth::Unreachable
            | ServiceBrowserHealth::Faulted
    )
}
pub(crate) fn retained_not_started_browser_placeholder(
    state: &ServiceState,
    browser_id: &str,
    browser: &BrowserProcess,
) -> bool {
    browser.health == ServiceBrowserHealth::NotStarted
        && browser.pid.is_none()
        && browser.cdp_endpoint.is_none()
        && !browser_has_live_tabs(state, browser_id)
}
pub(crate) fn retained_failed_browser_placeholder(
    state: &ServiceState,
    browser_id: &str,
    browser: &BrowserProcess,
) -> bool {
    failed_retained_browser_health(browser.health)
        && (matches!(
            browser.health,
            ServiceBrowserHealth::ProcessExited | ServiceBrowserHealth::Unreachable
        ) || (browser.pid.is_none() && browser.cdp_endpoint.is_none()))
        && !browser_has_live_tabs(state, browser_id)
}
pub(crate) fn browser_has_live_tabs(state: &ServiceState, browser_id: &str) -> bool {
    state
        .tabs
        .values()
        .any(|tab| tab.browser_id == browser_id && tab.lifecycle != TabLifecycle::Closed)
}
pub(crate) async fn handle_service_job_cancel(cmd: &Value) -> Result<Value, String> {
    let job_id = cmd
        .get("jobId")
        .and_then(|value| value.as_str())
        .ok_or("Missing jobId")?;
    let reason = cmd.get("reason").and_then(|value| value.as_str());
    let job = cancel_persisted_service_job(job_id, reason)?;
    Ok(json!({ "cancelled" : true, "job" : job, }))
}
pub(crate) async fn handle_service_profile_upsert(cmd: &Value) -> Result<Value, String> {
    let profile_id = required_service_config_id(cmd, "profileId")?;
    let body = cmd.get("profile").cloned().ok_or("Missing profile")?;
    let profile = upsert_persisted_profile(profile_id, body)?;
    Ok(json!({ "id" : profile_id, "profile" : profile, "upserted" : true, }))
}
pub(crate) async fn handle_service_profile_freshness_update(cmd: &Value) -> Result<Value, String> {
    let profile_id = required_service_config_id(cmd, "profileId")?;
    let body = cmd.get("freshness").cloned().ok_or("Missing freshness")?;
    let profile = update_persisted_profile_freshness(profile_id, body)?;
    Ok(json!({ "id" : profile_id, "profile" : profile, "upserted" : true, }))
}
pub(crate) async fn handle_service_profile_seeding_handoff_update(
    cmd: &Value,
) -> Result<Value, String> {
    let profile_id = required_service_config_id(cmd, "profileId")?;
    let body = cmd
        .get("handoff")
        .cloned()
        .ok_or("Missing profile seeding handoff")?;
    let handoff = update_persisted_profile_seeding_handoff(profile_id, body)?;
    let repository = LockedServiceStateRepository::default_json()?;
    let mut service_state = repository.load_snapshot()?;
    service_state.refresh_profile_readiness();
    let response = service_profile_seeding_handoff(
        &service_state,
        profile_id,
        Some(handoff.target_service_id.as_str()),
    )?;
    Ok(json!(
        { "id" : handoff.id, "profileId" : profile_id, "targetServiceId" : handoff
        .target_service_id, "handoff" : handoff, "seedingHandoff" : response,
        "updated" : true, }
    ))
}
pub(crate) async fn handle_service_profile_delete(cmd: &Value) -> Result<Value, String> {
    let profile_id = required_service_config_id(cmd, "profileId")?;
    let removed = delete_persisted_profile(profile_id)?;
    Ok(json!({ "id" : profile_id, "deleted" : removed.is_some(), "profile" : removed, }))
}
pub(crate) async fn handle_service_session_upsert(cmd: &Value) -> Result<Value, String> {
    let session_id = required_service_config_id(cmd, "sessionId")?;
    let body = cmd.get("session").cloned().ok_or("Missing session")?;
    let session = upsert_persisted_session(session_id, body)?;
    Ok(json!({ "id" : session_id, "session" : session, "upserted" : true, }))
}
pub(crate) async fn handle_service_session_delete(cmd: &Value) -> Result<Value, String> {
    let session_id = required_service_config_id(cmd, "sessionId")?;
    let removed = delete_persisted_session(session_id)?;
    Ok(json!({ "id" : session_id, "deleted" : removed.is_some(), "session" : removed, }))
}
pub(crate) async fn handle_service_site_policy_upsert(cmd: &Value) -> Result<Value, String> {
    let site_policy_id = required_service_config_id(cmd, "sitePolicyId")?;
    let body = cmd.get("sitePolicy").cloned().ok_or("Missing sitePolicy")?;
    let site_policy = upsert_persisted_site_policy(site_policy_id, body)?;
    Ok(json!({ "id" : site_policy_id, "sitePolicy" : site_policy, "upserted" : true, }))
}
pub(crate) async fn handle_service_site_policy_delete(cmd: &Value) -> Result<Value, String> {
    let site_policy_id = required_service_config_id(cmd, "sitePolicyId")?;
    let removed = delete_persisted_site_policy(site_policy_id)?;
    Ok(json!(
        { "id" : site_policy_id, "deleted" : removed.is_some(), "sitePolicy" :
        removed, }
    ))
}
pub(crate) async fn handle_service_monitor_upsert(cmd: &Value) -> Result<Value, String> {
    let monitor_id = required_service_config_id(cmd, "monitorId")?;
    let body = cmd.get("monitor").cloned().ok_or("Missing monitor")?;
    let monitor = upsert_persisted_monitor(monitor_id, body)?;
    Ok(json!({ "id" : monitor_id, "monitor" : monitor, "upserted" : true, }))
}
pub(crate) async fn handle_service_monitor_delete(cmd: &Value) -> Result<Value, String> {
    let monitor_id = required_service_config_id(cmd, "monitorId")?;
    let removed = delete_persisted_monitor(monitor_id)?;
    Ok(json!({ "id" : monitor_id, "deleted" : removed.is_some(), "monitor" : removed, }))
}
pub(crate) async fn handle_service_monitor_state_update(
    cmd: &Value,
    monitor_state: MonitorState,
) -> Result<Value, String> {
    let monitor_id = required_service_config_id(cmd, "monitorId")?;
    let monitor = update_persisted_monitor_state(monitor_id, monitor_state)?;
    Ok(json!(
        { "id" : monitor_id, "monitor" : monitor, "state" : monitor_state, "updated"
        : true, }
    ))
}
pub(crate) async fn handle_service_monitor_reset_failures(cmd: &Value) -> Result<Value, String> {
    let monitor_id = required_service_config_id(cmd, "monitorId")?;
    let monitor = reset_persisted_monitor_failures(monitor_id)?;
    let state = monitor.state;
    Ok(json!(
        { "id" : monitor_id, "monitor" : monitor, "state" : state, "updated" : true,
        "resetFailures" : true, }
    ))
}
pub(crate) async fn handle_service_monitor_triage(cmd: &Value) -> Result<Value, String> {
    let monitor_id = required_service_config_id(cmd, "monitorId")?;
    let by = cmd.get("by").and_then(|value| value.as_str());
    let note = cmd.get("note").and_then(|value| value.as_str());
    let actor = normalized_operator(by);
    let note = normalized_note(note);
    let timestamp = service_now_timestamp();
    let (monitor, incident) =
        triage_persisted_service_monitor(monitor_id, &timestamp, &actor, note.as_deref())?;
    let state = monitor.state;
    Ok(json!(
        { "id" : monitor_id, "monitor" : monitor, "state" : state, "updated" : true,
        "resetFailures" : true, "acknowledged" : incident.is_some(), "incident" :
        incident, }
    ))
}
pub(crate) async fn handle_service_monitors_run_due(_cmd: &Value) -> Result<Value, String> {
    let summary = run_due_persisted_monitors().await?;
    Ok(json!(
        { "checked" : summary.checked, "succeeded" : summary.succeeded, "failed" :
        summary.failed, "monitorIds" : summary.monitor_ids, "results" : summary
        .results, }
    ))
}
pub(crate) async fn handle_service_provider_upsert(cmd: &Value) -> Result<Value, String> {
    let provider_id = required_service_config_id(cmd, "providerId")?;
    let body = cmd.get("provider").cloned().ok_or("Missing provider")?;
    let provider = upsert_persisted_provider(provider_id, body)?;
    Ok(json!({ "id" : provider_id, "provider" : provider, "upserted" : true, }))
}
pub(crate) async fn handle_service_provider_delete(cmd: &Value) -> Result<Value, String> {
    let provider_id = required_service_config_id(cmd, "providerId")?;
    let removed = delete_persisted_provider(provider_id)?;
    Ok(json!(
        { "id" : provider_id, "deleted" : removed.is_some(), "provider" : removed, }
    ))
}
pub(crate) async fn handle_service_browser_capability_registry_upsert(
    cmd: &Value,
) -> Result<Value, String> {
    let collection = required_service_config_id(cmd, "collection")?;
    let record_id = required_service_config_id(cmd, "recordId")?;
    let body = cmd.get("record").cloned().ok_or("Missing record")?;
    let (record, registry, counts) =
        upsert_persisted_browser_capability_registry_record(collection, record_id, body)?;
    Ok(json!(
        { "id" : record_id, "collection" : collection, "record" : record,
        "browserCapabilityRegistry" : registry, "counts" : counts, "upserted" : true,
        "advisory" : true, "routingApplied" : false, }
    ))
}
pub(crate) fn required_service_config_id<'a>(
    cmd: &'a Value,
    field: &str,
) -> Result<&'a str, String> {
    cmd.get(field)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Missing {field}"))
}
pub(crate) async fn handle_service_browser_retry(cmd: &Value) -> Result<Value, String> {
    let browser_id = cmd
        .get("browserId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or("Missing browserId")?;
    let by = cmd.get("by").and_then(|value| value.as_str());
    let note = cmd.get("note").and_then(|value| value.as_str());
    let service_name = optional_command_string(cmd, "serviceName");
    let agent_name = optional_command_string(cmd, "agentName");
    let task_name = optional_command_string(cmd, "taskName");
    let actor = normalized_operator(by);
    let note = normalized_note(note);
    let timestamp = service_now_timestamp();
    let repository = LockedServiceStateRepository::default_json().map_err(|err| {
        if err.starts_with("Failed to") || err.starts_with("Invalid service state") {
            format!("Unable to load service state: {}", err)
        } else {
            err
        }
    })?;
    let (retryable, incident) = retry_persisted_service_browser_in_repository(
        &repository,
        browser_id,
        &timestamp,
        &actor,
        note.as_deref(),
        service_name.as_deref(),
        agent_name.as_deref(),
        task_name.as_deref(),
    )?;
    Ok(json!({ "retryEnabled" : true, "browser" : retryable, "incident" : incident, }))
}
pub(crate) async fn handle_service_remedies_apply(cmd: &Value) -> Result<Value, String> {
    let escalation = cmd
        .get("escalation")
        .and_then(|value| value.as_str())
        .unwrap_or("monitor_attention");
    let by = cmd.get("by").and_then(|value| value.as_str());
    let note = cmd.get("note").and_then(|value| value.as_str());
    let service_name = optional_command_string(cmd, "serviceName");
    let agent_name = optional_command_string(cmd, "agentName");
    let task_name = optional_command_string(cmd, "taskName");
    let actor = normalized_operator(by);
    let note = normalized_note(note);
    let timestamp = service_now_timestamp();
    apply_persisted_service_remedies(
        escalation,
        &timestamp,
        &actor,
        note.as_deref(),
        service_name.as_deref(),
        agent_name.as_deref(),
        task_name.as_deref(),
    )
}
pub(crate) async fn handle_service_incident_acknowledge(cmd: &Value) -> Result<Value, String> {
    let incident_id = cmd
        .get("incidentId")
        .and_then(|value| value.as_str())
        .ok_or("Missing incidentId")?;
    let by = cmd.get("by").and_then(|value| value.as_str());
    let note = cmd.get("note").and_then(|value| value.as_str());
    let actor = normalized_operator(by);
    let note = normalized_note(note);
    let timestamp = service_now_timestamp();
    let incident =
        acknowledge_persisted_service_incident(incident_id, &timestamp, &actor, note.as_deref())?;
    Ok(json!({ "acknowledged" : true, "incident" : incident, }))
}
pub(crate) async fn handle_service_incident_resolve(cmd: &Value) -> Result<Value, String> {
    let incident_id = cmd
        .get("incidentId")
        .and_then(|value| value.as_str())
        .ok_or("Missing incidentId")?;
    let by = cmd.get("by").and_then(|value| value.as_str());
    let note = cmd.get("note").and_then(|value| value.as_str());
    let actor = normalized_operator(by);
    let note = normalized_note(note);
    let timestamp = service_now_timestamp();
    let incident =
        resolve_persisted_service_incident(incident_id, &timestamp, &actor, note.as_deref())?;
    Ok(json!({ "resolved" : true, "incident" : incident, }))
}
pub(crate) fn normalized_operator(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("operator")
        .to_string()
}
pub(crate) fn normalized_note(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
pub(crate) async fn handle_service_events(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let limit = cmd
        .get("limit")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or(20);
    let kind = cmd.get("kind").and_then(|value| value.as_str());
    let browser_id = cmd.get("browserId").and_then(|value| value.as_str());
    let profile_id = cmd.get("profileId").and_then(|value| value.as_str());
    let session_id = cmd.get("sessionId").and_then(|value| value.as_str());
    let service_name = cmd.get("serviceName").and_then(|value| value.as_str());
    let agent_name = cmd.get("agentName").and_then(|value| value.as_str());
    let task_name = cmd.get("taskName").and_then(|value| value.as_str());
    let since = cmd
        .get("since")
        .and_then(|value| value.as_str())
        .map(parse_service_event_timestamp)
        .transpose()?;
    let total = service_state.events.len();
    let mut events = service_state
        .events
        .into_iter()
        .filter(|event| {
            kind.is_none_or(|expected| service_event_kind_name(event.kind) == expected)
                && browser_id.is_none_or(|expected| event.browser_id.as_deref() == Some(expected))
                && profile_id.is_none_or(|expected| event.profile_id.as_deref() == Some(expected))
                && session_id.is_none_or(|expected| event.session_id.as_deref() == Some(expected))
                && service_name
                    .is_none_or(|expected| event.service_name.as_deref() == Some(expected))
                && agent_name.is_none_or(|expected| event.agent_name.as_deref() == Some(expected))
                && task_name.is_none_or(|expected| event.task_name.as_deref() == Some(expected))
                && since.is_none_or(|minimum| service_event_at_or_after(event, minimum))
        })
        .collect::<Vec<_>>();
    let matched = events.len();
    let start = matched.saturating_sub(limit);
    events = events[start..].to_vec();
    Ok(json!(
        { "events" : events, "count" : events.len(), "matched" : matched, "total" :
        total, }
    ))
}
pub(crate) async fn handle_service_incidents(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let limit = cmd
        .get("limit")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or(20);
    let remedies_only = cmd
        .get("remediesOnly")
        .or_else(|| cmd.get("remedies"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let summarize = cmd
        .get("summary")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || remedies_only;
    let incident_state = cmd
        .get("state")
        .and_then(|value| value.as_str())
        .or(if remedies_only { Some("active") } else { None });
    let mut response = service_incidents_response(
        &service_state,
        ServiceIncidentFilters {
            limit,
            incident_id: cmd.get("incidentId").and_then(|value| value.as_str()),
            state: incident_state,
            severity: cmd.get("severity").and_then(|value| value.as_str()),
            escalation: cmd.get("escalation").and_then(|value| value.as_str()),
            handling_state: cmd.get("handlingState").and_then(|value| value.as_str()),
            kind: cmd.get("kind").and_then(|value| value.as_str()),
            browser_id: cmd.get("browserId").and_then(|value| value.as_str()),
            profile_id: cmd.get("profileId").and_then(|value| value.as_str()),
            session_id: cmd.get("sessionId").and_then(|value| value.as_str()),
            service_name: cmd.get("serviceName").and_then(|value| value.as_str()),
            agent_name: cmd.get("agentName").and_then(|value| value.as_str()),
            task_name: cmd.get("taskName").and_then(|value| value.as_str()),
            since: cmd.get("since").and_then(|value| value.as_str()),
            remedies_only,
        },
    )?;
    if summarize {
        let incidents = response
            .get("incidents")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        response["summary"] = service_incident_summary(&incidents);
    }
    Ok(response)
}
pub(crate) async fn handle_service_jobs(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let limit = cmd
        .get("limit")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or(20);
    let state = cmd.get("state").and_then(|value| value.as_str());
    let action = cmd.get("jobAction").and_then(|value| value.as_str());
    let profile_id = cmd.get("profileId").and_then(|value| value.as_str());
    let session_id = cmd.get("sessionId").and_then(|value| value.as_str());
    let service_name = cmd.get("serviceName").and_then(|value| value.as_str());
    let agent_name = cmd.get("agentName").and_then(|value| value.as_str());
    let task_name = cmd.get("taskName").and_then(|value| value.as_str());
    let since = cmd
        .get("since")
        .and_then(|value| value.as_str())
        .map(parse_service_event_timestamp)
        .transpose()?;
    let total = service_state.jobs.len();
    if let Some(job_id) = cmd.get("jobId").and_then(|value| value.as_str()) {
        let job = service_state
            .jobs
            .get(job_id)
            .cloned()
            .ok_or_else(|| format!("Service job not found: {}", job_id))?;
        return Ok(json!(
            { "job" : job, "jobs" : [job], "count" : 1, "matched" : 1, "total" :
            total, }
        ));
    }
    let mut jobs = service_state.jobs.values().cloned().collect::<Vec<_>>();
    jobs.sort_by(|left, right| {
        let left_time = left.submitted_at.as_deref().unwrap_or_default();
        let right_time = right.submitted_at.as_deref().unwrap_or_default();
        left_time
            .cmp(right_time)
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut jobs = jobs
        .into_iter()
        .filter(|job| {
            state.is_none_or(|expected| service_job_state_name(job.state) == expected)
                && action.is_none_or(|expected| job.action == expected)
                && service_job_matches_trace_filters(
                    job,
                    &service_state,
                    profile_id,
                    session_id,
                    service_name,
                    agent_name,
                    task_name,
                )
                && since.is_none_or(|minimum| service_job_at_or_after(job, minimum))
        })
        .collect::<Vec<_>>();
    let matched = jobs.len();
    let start = matched.saturating_sub(limit);
    jobs = jobs[start..].to_vec();
    Ok(json!(
        { "jobs" : jobs, "count" : jobs.len(), "matched" : matched, "total" : total,
        }
    ))
}
pub(crate) async fn handle_service_incident_activity(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let incident_id = cmd
        .get("incidentId")
        .and_then(|value| value.as_str())
        .ok_or("Missing incidentId")?;
    service_incident_activity_response(&service_state, incident_id)
}
pub(crate) async fn handle_service_trace(cmd: &Value) -> Result<Value, String> {
    let service_state = cmd
        .get("serviceState")
        .cloned()
        .map(serde_json::from_value::<ServiceState>)
        .transpose()
        .map_err(|err| format!("Invalid serviceState: {}", err))?
        .unwrap_or_default();
    let limit = cmd
        .get("limit")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or(20);
    let browser_id = cmd.get("browserId").and_then(|value| value.as_str());
    let profile_id = cmd.get("profileId").and_then(|value| value.as_str());
    let session_id = cmd.get("sessionId").and_then(|value| value.as_str());
    let service_name = cmd.get("serviceName").and_then(|value| value.as_str());
    let agent_name = cmd.get("agentName").and_then(|value| value.as_str());
    let task_name = cmd.get("taskName").and_then(|value| value.as_str());
    let since = cmd.get("since").and_then(|value| value.as_str());
    service_trace_response(
        &service_state,
        ServiceTraceFilters {
            limit,
            browser_id,
            profile_id,
            session_id,
            service_name,
            agent_name,
            task_name,
            since,
        },
    )
}
pub(crate) fn service_event_kind_name(kind: ServiceEventKind) -> &'static str {
    match kind {
        ServiceEventKind::Reconciliation => "reconciliation",
        ServiceEventKind::BrowserLaunchRecorded => "browser_launch_recorded",
        ServiceEventKind::BrowserHealthChanged => "browser_health_changed",
        ServiceEventKind::BrowserRecoveryStarted => "browser_recovery_started",
        ServiceEventKind::BrowserRecoveryOverride => "browser_recovery_override",
        ServiceEventKind::TabLifecycleChanged => "tab_lifecycle_changed",
        ServiceEventKind::ProfileLeaseWaitStarted => "profile_lease_wait_started",
        ServiceEventKind::ProfileLeaseWaitEnded => "profile_lease_wait_ended",
        ServiceEventKind::ViewerTakeoverRequested => "viewer_takeover_requested",
        ServiceEventKind::ViewerConnected => "viewer_connected",
        ServiceEventKind::ViewerDisconnected => "viewer_disconnected",
        ServiceEventKind::ControllerRequested => "controller_requested",
        ServiceEventKind::ControllerGranted => "controller_granted",
        ServiceEventKind::ControllerDenied => "controller_denied",
        ServiceEventKind::RouteReleased => "route_released",
        ServiceEventKind::ReconciliationError => "reconciliation_error",
        ServiceEventKind::IncidentAcknowledged => "incident_acknowledged",
        ServiceEventKind::IncidentResolved => "incident_resolved",
    }
}
pub(crate) fn service_job_state_name(state: ServiceJobState) -> &'static str {
    match state {
        ServiceJobState::Queued => "queued",
        ServiceJobState::WaitingProfileLease => "waiting_profile_lease",
        ServiceJobState::Running => "running",
        ServiceJobState::Succeeded => "succeeded",
        ServiceJobState::Failed => "failed",
        ServiceJobState::Cancelled => "cancelled",
        ServiceJobState::TimedOut => "timed_out",
    }
}
pub(crate) fn service_incident_state_name(
    state: super::super::service_model::ServiceIncidentState,
) -> &'static str {
    match state {
        super::super::service_model::ServiceIncidentState::Active => "active",
        super::super::service_model::ServiceIncidentState::Recovered => "recovered",
        super::super::service_model::ServiceIncidentState::Service => "service",
    }
}
pub(crate) fn service_incident_severity_name(
    severity: super::super::service_model::ServiceIncidentSeverity,
) -> &'static str {
    match severity {
        super::super::service_model::ServiceIncidentSeverity::Info => "info",
        super::super::service_model::ServiceIncidentSeverity::Warning => "warning",
        super::super::service_model::ServiceIncidentSeverity::Error => "error",
        super::super::service_model::ServiceIncidentSeverity::Critical => "critical",
    }
}
pub(crate) fn service_incident_escalation_name(
    escalation: super::super::service_model::ServiceIncidentEscalation,
) -> &'static str {
    match escalation {
        super::super::service_model::ServiceIncidentEscalation::None => "none",
        super::super::service_model::ServiceIncidentEscalation::BrowserDegraded => {
            "browser_degraded"
        }
        super::super::service_model::ServiceIncidentEscalation::BrowserRecovery => {
            "browser_recovery"
        }
        super::super::service_model::ServiceIncidentEscalation::JobAttention => "job_attention",
        super::super::service_model::ServiceIncidentEscalation::MonitorAttention => {
            "monitor_attention"
        }
        super::super::service_model::ServiceIncidentEscalation::ServiceTriage => "service_triage",
        super::super::service_model::ServiceIncidentEscalation::OsDegradedPossible => {
            "os_degraded_possible"
        }
    }
}
pub(crate) fn service_incident_handling_state_name(
    incident: &super::super::service_model::ServiceIncident,
) -> &'static str {
    if incident.resolved_at.is_some() {
        "resolved"
    } else if incident.acknowledged_at.is_some() {
        "acknowledged"
    } else {
        "unacknowledged"
    }
}
pub(crate) fn parse_service_event_timestamp(raw: &str) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_rfc3339(raw)
        .map_err(|err| format!("Invalid --since timestamp '{}': {}", raw, err))
}
pub(crate) fn service_job_at_or_after(
    job: &super::super::service_model::ServiceJob,
    minimum: DateTime<FixedOffset>,
) -> bool {
    job.submitted_at
        .as_deref()
        .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
        .is_some_and(|timestamp| timestamp >= minimum)
}
pub(crate) fn service_event_at_or_after(
    event: &ServiceEvent,
    minimum: DateTime<FixedOffset>,
) -> bool {
    DateTime::parse_from_rfc3339(&event.timestamp)
        .map(|timestamp| timestamp >= minimum)
        .unwrap_or(false)
}
pub(crate) fn service_incident_matches_trace_filters(
    incident: &super::super::service_model::ServiceIncident,
    service_state: &ServiceState,
    profile_id: Option<&str>,
    session_id: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> bool {
    if profile_id.is_none()
        && session_id.is_none()
        && service_name.is_none()
        && agent_name.is_none()
        && task_name.is_none()
    {
        return true;
    }
    incident.event_ids.iter().any(|event_id| {
        service_state
            .events
            .iter()
            .find(|event| &event.id == event_id)
            .is_some_and(|event| {
                service_event_matches_trace_filters(
                    event,
                    profile_id,
                    session_id,
                    service_name,
                    agent_name,
                    task_name,
                )
            })
    }) || incident.job_ids.iter().any(|job_id| {
        service_state.jobs.get(job_id).is_some_and(|job| {
            service_job_matches_trace_filters(
                job,
                service_state,
                profile_id,
                session_id,
                service_name,
                agent_name,
                task_name,
            )
        })
    })
}
pub(crate) fn service_event_matches_trace_filters(
    event: &ServiceEvent,
    profile_id: Option<&str>,
    session_id: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> bool {
    profile_id.is_none_or(|expected| event.profile_id.as_deref() == Some(expected))
        && session_id.is_none_or(|expected| event.session_id.as_deref() == Some(expected))
        && service_name.is_none_or(|expected| event.service_name.as_deref() == Some(expected))
        && agent_name.is_none_or(|expected| event.agent_name.as_deref() == Some(expected))
        && task_name.is_none_or(|expected| event.task_name.as_deref() == Some(expected))
}
pub(crate) fn service_job_matches_trace_filters(
    job: &super::super::service_model::ServiceJob,
    service_state: &ServiceState,
    profile_id: Option<&str>,
    session_id: Option<&str>,
    service_name: Option<&str>,
    agent_name: Option<&str>,
    task_name: Option<&str>,
) -> bool {
    profile_id.is_none_or(|expected| service_job_profile_id(job, service_state) == Some(expected))
        && session_id
            .is_none_or(|expected| service_job_session_id(job, service_state) == Some(expected))
        && service_name.is_none_or(|expected| job.service_name.as_deref() == Some(expected))
        && agent_name.is_none_or(|expected| job.agent_name.as_deref() == Some(expected))
        && task_name.is_none_or(|expected| job.task_name.as_deref() == Some(expected))
}
pub(crate) fn service_job_profile_id<'a>(
    job: &'a super::super::service_model::ServiceJob,
    service_state: &'a ServiceState,
) -> Option<&'a str> {
    match &job.target {
        super::super::service_model::JobTarget::Profile(profile_id) => Some(profile_id.as_str()),
        super::super::service_model::JobTarget::Browser(browser_id) => service_state
            .browsers
            .get(browser_id)
            .and_then(|browser| browser.profile_id.as_deref()),
        super::super::service_model::JobTarget::Tab(tab_id) => {
            service_state.tabs.get(tab_id).and_then(|tab| {
                tab.owner_session_id
                    .as_deref()
                    .and_then(|session_id| service_state.sessions.get(session_id))
                    .and_then(|session| session.profile_id.as_deref())
                    .or_else(|| {
                        service_state
                            .browsers
                            .get(&tab.browser_id)
                            .and_then(|browser| browser.profile_id.as_deref())
                    })
            })
        }
        super::super::service_model::JobTarget::Service
        | super::super::service_model::JobTarget::Monitor(_)
        | super::super::service_model::JobTarget::Challenge(_) => None,
    }
}
pub(crate) fn service_job_session_id<'a>(
    job: &'a super::super::service_model::ServiceJob,
    service_state: &'a ServiceState,
) -> Option<&'a str> {
    match &job.target {
        super::super::service_model::JobTarget::Browser(browser_id) => service_state
            .browsers
            .get(browser_id)
            .and_then(|browser| browser.active_session_ids.first().map(String::as_str))
            .or_else(|| session_id_for_browser(service_state, browser_id)),
        super::super::service_model::JobTarget::Tab(tab_id) => service_state
            .tabs
            .get(tab_id)
            .and_then(|tab| tab.owner_session_id.as_deref()),
        super::super::service_model::JobTarget::Service
        | super::super::service_model::JobTarget::Profile(_)
        | super::super::service_model::JobTarget::Monitor(_)
        | super::super::service_model::JobTarget::Challenge(_) => None,
    }
}
pub(crate) fn session_id_for_browser<'a>(
    service_state: &'a ServiceState,
    browser_id: &str,
) -> Option<&'a str> {
    service_state
        .sessions
        .iter()
        .find_map(|(session_id, session)| {
            session
                .browser_ids
                .iter()
                .any(|id| id == browser_id)
                .then_some(session_id.as_str())
        })
}
pub(crate) fn service_incident_at_or_after(
    incident: &super::super::service_model::ServiceIncident,
    minimum: DateTime<FixedOffset>,
) -> bool {
    DateTime::parse_from_rfc3339(&incident.latest_timestamp)
        .map(|timestamp| timestamp >= minimum)
        .unwrap_or(false)
}
pub(crate) fn service_now_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}
pub(crate) async fn handle_screencast_start(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    if state.screencasting {
        return Err("Screencast already active".to_string());
    }
    let (default_w, default_h) = if let Some(ref server) = state.stream_server {
        server.viewport().await
    } else {
        (1280, 720)
    };
    let format = cmd.get("format").and_then(|v| v.as_str()).unwrap_or("jpeg");
    let quality = cmd.get("quality").and_then(|v| v.as_i64()).unwrap_or(80) as i32;
    let max_width = cmd
        .get("maxWidth")
        .and_then(|v| v.as_i64())
        .unwrap_or(default_w as i64) as i32;
    let max_height = cmd
        .get("maxHeight")
        .and_then(|v| v.as_i64())
        .unwrap_or(default_h as i64) as i32;
    stream::start_screencast(
        &mgr.client,
        &session_id,
        format,
        quality,
        max_width,
        max_height,
    )
    .await?;
    state.screencasting = true;
    if let Some(ref server) = state.stream_server {
        server.set_screencasting(true).await;
        server
            .broadcast_status(
                true,
                true,
                max_width as u32,
                max_height as u32,
                &state.engine,
            )
            .await;
    }
    Ok(json!({ "started" : true }))
}
pub(crate) async fn handle_screencast_stop(state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?;
    if !state.screencasting {
        return Err("No screencast active".to_string());
    }
    stream::stop_screencast(&mgr.client, session_id).await?;
    state.screencasting = false;
    if let Some(ref server) = state.stream_server {
        server.set_screencasting(false).await;
        let (vw, vh) = server.viewport().await;
        server
            .broadcast_status(true, false, vw, vh, &state.engine)
            .await;
    }
    Ok(json!({ "stopped" : true }))
}
