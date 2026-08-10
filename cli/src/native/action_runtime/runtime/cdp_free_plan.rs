#![allow(unused_imports)]
use super::super::browser_operations::{
    add_manual_login_hint_warning, har_cdp_protocol_to_http_version, har_extract_headers,
    persist_service_owned_navigate_tab, resolve_fetch_paused, stream_file_path, write_engine_file,
    write_extensions_file, write_provider_file,
};
use super::super::common::*;
use super::super::service_workflows::{runtime_handoff_path, write_runtime_handoff};
use super::remote_headed::remote_headed_view_streams_from_command;
pub(crate) struct CdpFreeLaunchPlan {
    pub(crate) launch_options: LaunchOptions,
    pub(crate) metadata: ServiceLaunchMetadata,
    pub(crate) url: Option<String>,
}
impl ServiceLaunchMetadata {
    /// Captures the service-model metadata that can be inferred from a launch
    /// command before Chrome starts, so every launch path writes the same
    /// browser/profile/session relationships into persisted service state.
    pub(crate) fn from_launch_options(
        options: &LaunchOptions,
        command: Option<&Value>,
        selection_reason: Option<ProfileSelectionReason>,
    ) -> Self {
        let profile_id = launch_options_service_profile_id(options);
        let user_data_dir = options.profile.clone().or_else(|| {
            options.runtime_profile.as_ref().and_then(|name| {
                runtime_profile_user_data_dir(name)
                    .ok()
                    .map(|path| path.to_string_lossy().to_string())
            })
        });
        Self {
            profile_name: options.runtime_profile.clone().or(options.profile.clone()),
            user_data_dir,
            persistent_profile: profile_id.is_some(),
            keyring: if options.use_real_keychain {
                ProfileKeyringPolicy::RealOsKeychain
            } else {
                ProfileKeyringPolicy::BasicPasswordStore
            },
            profile_id: profile_id.clone(),
            service_name: command.and_then(|cmd| optional_command_string(cmd, "serviceName")),
            agent_name: command.and_then(|cmd| optional_command_string(cmd, "agentName")),
            task_name: command.and_then(|cmd| optional_command_string(cmd, "taskName")),
            cleanup: if command
                .and_then(|cmd| cmd.get("leaveOpen"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                SessionCleanupPolicy::Detach
            } else {
                SessionCleanupPolicy::CloseBrowser
            },
            profile_selection_reason: selection_reason.or_else(|| {
                profile_id
                    .is_some()
                    .then_some(ProfileSelectionReason::ExplicitProfile)
            }),
            browser_stderr_log_path: None,
            browser_capability_launch: None,
            view_streams: command
                .map(remote_headed_view_streams_from_command)
                .unwrap_or_default(),
            display_isolation: remote_headed_display_isolation(options),
            display_name: match remote_headed_display_isolation(options).as_deref() {
                Some("private_virtual_display") => None,
                _ => options.display.clone().or_else(|| {
                    (options.remote_headed && !options.headless)
                        .then(|| env::var("DISPLAY").ok())
                        .flatten()
                }),
            },
        }
    }
}
pub(crate) fn launch_options_service_profile_id(options: &LaunchOptions) -> Option<String> {
    if options
        .runtime_profile
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return options.runtime_profile.clone();
    }
    let profile = options.profile.as_deref()?.trim();
    if profile.is_empty() {
        return None;
    }
    if looks_like_path(profile) {
        return service_profile_id(Some(profile), None);
    }
    Some(profile.to_string())
}
pub(crate) fn remote_headed_display_isolation(options: &LaunchOptions) -> Option<String> {
    if !options.remote_headed || options.headless {
        return None;
    }
    if let Some(value) = options.remote_headed_display_isolation.as_ref() {
        return Some(value.clone());
    }
    if options.display.is_some() {
        return Some("shared_display".to_string());
    }
    Some("private_virtual_display".to_string())
}
pub(crate) fn optional_command_string(command: &Value, name: &str) -> Option<String> {
    command
        .get(name)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
pub(crate) fn optional_command_or_params_string(command: &Value, name: &str) -> Option<String> {
    optional_command_string(command, name).or_else(|| {
        command
            .get("params")
            .and_then(|params| optional_command_string(params, name))
    })
}
pub(crate) fn optional_command_or_params_bool(command: &Value, name: &str) -> Option<bool> {
    command_or_params_value(command, name).and_then(Value::as_bool)
}
pub(crate) fn manual_login_launch_from_command(
    command: &Value,
    headless: bool,
) -> Result<bool, String> {
    let enabled = optional_command_or_params_bool(command, "manualLoginLaunch").unwrap_or(false);
    if enabled && headless {
        return Err(
            "manual_login_launch_requires_headed: set headless=false for the minimal manual-login-safe Chrome launch posture"
                .to_string(),
        );
    }
    Ok(enabled)
}
pub(crate) fn command_or_params_value<'a>(command: &'a Value, name: &str) -> Option<&'a Value> {
    command
        .get(name)
        .or_else(|| command.get("params").and_then(|params| params.get(name)))
}
pub(crate) fn browser_host_from_command(command: &Value) -> Option<ServiceBrowserHost> {
    for name in ["browserHost", "browser_host", "browser-host"] {
        if let Some(host) = command.get(name).and_then(Value::as_str) {
            return parse_service_browser_host(host);
        }
    }
    command.get("params").and_then(browser_host_from_command)
}
pub(crate) fn parse_service_browser_host(value: &str) -> Option<ServiceBrowserHost> {
    match value.trim() {
        "local_headless" | "local-headless" => Some(ServiceBrowserHost::LocalHeadless),
        "local_headed" | "local-headed" => Some(ServiceBrowserHost::LocalHeaded),
        "docker_headed" | "docker-headed" => Some(ServiceBrowserHost::DockerHeaded),
        "remote_headed" | "remote-headed" => Some(ServiceBrowserHost::RemoteHeaded),
        "cloud_provider" | "cloud-provider" => Some(ServiceBrowserHost::CloudProvider),
        "attached_existing" | "attached-existing" => Some(ServiceBrowserHost::AttachedExisting),
        _ => None,
    }
}
#[derive(Debug, Clone)]
pub(crate) struct RetainedRemoteHeadedLaunchHint {
    pub(crate) view_streams: Vec<ViewStream>,
    pub(crate) display_isolation: Option<String>,
    pub(crate) display_name: Option<String>,
}
pub(crate) fn command_has_explicit_launch_surface(command: &Value) -> bool {
    browser_host_from_command(command).is_some()
        || command.get("provider").is_some()
        || command.get("cdpUrl").is_some()
        || command.get("cdpPort").is_some()
        || command
            .get("autoConnect")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        || command
            .get("headlessExplicit")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        || command
            .get("params")
            .and_then(|params| params.get("headlessExplicit"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}
pub(crate) fn retained_remote_headed_launch_hint(
    session_id: &str,
    command: &Value,
) -> Option<RetainedRemoteHeadedLaunchHint> {
    if command_has_explicit_launch_surface(command) {
        return None;
    }
    let repository = LockedServiceStateRepository::default_json().ok()?;
    let service_state = repository.load_snapshot().ok()?;
    let browser = service_state
        .browsers
        .get(&format!("session:{}", session_id))?;
    if browser.host != ServiceBrowserHost::RemoteHeaded || browser.view_streams.is_empty() {
        return None;
    }
    Some(RetainedRemoteHeadedLaunchHint {
        view_streams: browser.view_streams.clone(),
        display_isolation: browser.display_isolation.clone(),
        display_name: browser.display_name.clone(),
    })
}
pub(crate) fn apply_retained_remote_headed_launch_hints(
    options: &mut LaunchOptions,
    retained: Option<&RetainedRemoteHeadedLaunchHint>,
) {
    let Some(retained) = retained else {
        return;
    };
    options.headless = false;
    options.remote_headed = true;
    if options.remote_headed_display_isolation.is_none() {
        options.remote_headed_display_isolation = retained
            .display_isolation
            .as_deref()
            .and_then(normalize_remote_headed_display_isolation)
            .or_else(|| {
                retained
                    .display_name
                    .as_ref()
                    .map(|_| "shared_display".to_string())
            });
    }
    if options.display.is_none()
        && !matches!(
            options.remote_headed_display_isolation.as_deref(),
            Some("private_virtual_display" | "ambient_display")
        )
    {
        options.display = retained.display_name.clone();
    }
}
pub(crate) fn apply_retained_remote_headed_metadata(
    metadata: &mut ServiceLaunchMetadata,
    retained: Option<&RetainedRemoteHeadedLaunchHint>,
) {
    let Some(retained) = retained else {
        return;
    };
    if metadata.view_streams.is_empty() {
        metadata.view_streams = retained.view_streams.clone();
    }
    if metadata.display_isolation.is_none() {
        metadata.display_isolation = retained.display_isolation.clone();
    }
    if metadata.display_name.is_none() {
        metadata.display_name = retained.display_name.clone();
    }
}
pub(crate) fn apply_launch_host_hints(
    options: &mut LaunchOptions,
    command: &Value,
) -> ServiceBrowserHost {
    let host = browser_host_from_command(command).unwrap_or_else(|| {
        if command.get("provider").is_some() {
            ServiceBrowserHost::CloudProvider
        } else if command.get("cdpUrl").is_some()
            || command.get("cdpPort").is_some()
            || command
                .get("autoConnect")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        {
            ServiceBrowserHost::AttachedExisting
        } else if options.remote_headed {
            ServiceBrowserHost::RemoteHeaded
        } else if options.headless {
            ServiceBrowserHost::LocalHeadless
        } else {
            ServiceBrowserHost::LocalHeaded
        }
    });
    if matches!(
        host,
        ServiceBrowserHost::LocalHeaded
            | ServiceBrowserHost::RemoteHeaded
            | ServiceBrowserHost::DockerHeaded
    ) {
        options.headless = false;
    }
    if host == ServiceBrowserHost::LocalHeadless {
        options.headless = true;
        options.remote_headed = false;
    }
    if host == ServiceBrowserHost::RemoteHeaded {
        options.remote_headed = true;
        let explicit_remote_display = remote_headed_display_from_command_only(command);
        if let Some(display_isolation) = remote_headed_display_isolation_from_command(command) {
            options.remote_headed_display_isolation = Some(display_isolation);
        } else if options.remote_headed_display_isolation.is_none()
            && explicit_remote_display.is_some()
        {
            options.remote_headed_display_isolation = Some("shared_display".to_string());
        } else if options.remote_headed_display_isolation.is_none() {
            options.remote_headed_display_isolation = Some("private_virtual_display".to_string());
        }
        options.display = match options.remote_headed_display_isolation.as_deref() {
            Some("private_virtual_display") | Some("ambient_display") => None,
            _ => remote_headed_display_from_command(command).or_else(|| options.display.clone()),
        };
    } else {
        options.remote_headed = false;
        options.remote_headed_display_isolation = None;
    }
    host
}
pub(crate) fn remote_headed_display_isolation_from_command(command: &Value) -> Option<String> {
    optional_command_string(command, "displayIsolation")
        .or_else(|| optional_command_string(command, "displayAllocation"))
        .or_else(|| optional_command_string(command, "displayAllocationPolicy"))
        .or_else(|| {
            command
                .get("params")
                .and_then(remote_headed_display_isolation_from_command)
        })
        .and_then(|value| normalize_remote_headed_display_isolation(&value))
}
pub(crate) fn normalize_remote_headed_display_isolation(value: &str) -> Option<String> {
    match value.trim() {
        "private_virtual_display" | "private-virtual-display" | "private" => {
            Some("private_virtual_display".to_string())
        }
        "shared_display" | "shared-display" | "shared" => Some("shared_display".to_string()),
        "ambient_display" | "ambient-display" | "ambient" => Some("ambient_display".to_string()),
        _ => None,
    }
}
pub(crate) fn remote_headed_display_from_command(command: &Value) -> Option<String> {
    remote_headed_display_from_command_only(command)
        .or_else(|| env::var("AGENT_BROWSER_REMOTE_HEADED_DISPLAY").ok())
}
pub(crate) fn remote_headed_display_from_command_only(command: &Value) -> Option<String> {
    optional_command_string(command, "remoteHeadedDisplay")
        .or_else(|| optional_command_string(command, "display"))
        .or_else(|| {
            command
                .get("params")
                .and_then(remote_headed_display_from_command_only)
        })
}
