#![allow(unused_imports)]
use super::capability::service_browser_id;
use super::cdp_free_plan::{browser_host_from_command, optional_command_string};
use super::daemon::ServiceProfileLeaseGate;
use super::profile_lease::{
    profile_lease_wait_timeout_ms_from_command, service_profile_lease_gate,
};
use super::recovery::DaemonState;
use crate::native::browser_navigation::{
    add_manual_login_hint_warning, persist_service_owned_navigate_tab,
};
use crate::native::network::resolve_fetch_paused;
use crate::native::network_archive::{har_cdp_protocol_to_http_version, har_extract_headers};
use crate::native::service_health::{
    persist_browser_recovery_started_in_repository, persist_closed_browser_health_in_repository,
    persist_current_browser_stale_health_in_repository,
    persist_reconciled_service_state_in_repository, persist_service_browser_record_in_repository,
    reconcile_service_state, retry_degraded_service_browser_in_state,
    retry_persisted_service_browser_in_repository, retry_service_browser_in_state,
    BrowserRecoveryPersistence, BrowserRecoveryPolicyConfig, BrowserRecoveryPolicySource,
    BrowserRecoveryPolicyValueSource, BrowserRecoveryReasonKind,
};
use crate::native::service_lifecycle::{
    profile_lease_telemetry, select_service_profile_for_request, service_profile_id,
    ProfileSelectionRequest, ServiceLaunchMetadata,
};
use crate::native::service_model::{
    retained_display_allocation_candidates, service_profile_allocations,
    service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
    BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
    BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
    ControlInputProvider, DisplayAllocation, JobState as ServiceJobState, LeaseState, MonitorState,
    ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy, ProfileLeaseDisposition,
    ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease, RemoteViewHandoff,
    RemoteViewRoute, RoutePoolEntry, ServiceBrowserProcessIdentity, ServiceEntitySource,
    ServiceEvent, ServiceEventKind, ServiceState, ServiceTabHandle, SessionCleanupPolicy,
    TabLifecycle, ViewStream, ViewStreamProvider, ViewerLease,
};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use crate::native::state;
use crate::native::stream_runtime::{
    stream_file_path, write_engine_file, write_extensions_file, write_provider_file,
};
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::path::Path;
pub(crate) fn remote_headed_view_streams_from_command(command: &Value) -> Vec<ViewStream> {
    if browser_host_from_command(command) != Some(ServiceBrowserHost::RemoteHeaded) {
        return Vec::new();
    }
    let provider = optional_command_string(command, "viewStream")
        .or_else(|| optional_command_string(command, "viewStreamProvider"))
        .or_else(|| {
            command.get("params").and_then(|params| {
                optional_command_string(params, "viewStream")
                    .or_else(|| optional_command_string(params, "viewStreamProvider"))
            })
        })
        .or_else(|| env::var("AGENT_BROWSER_REMOTE_VIEW_PROVIDER").ok())
        .and_then(|provider| parse_view_stream_provider(&provider))
        .unwrap_or(ViewStreamProvider::CdpScreencast);
    let url = view_stream_command_string(
        command,
        &["remoteViewUrl", "viewStreamUrl"],
        "AGENT_BROWSER_REMOTE_VIEW_URL",
    );
    let frame_url = view_stream_command_string(
        command,
        &["frameUrl", "viewStreamFrameUrl", "remoteViewFrameUrl"],
        "AGENT_BROWSER_REMOTE_VIEW_FRAME_URL",
    )
    .or_else(|| guacamole_client_url(url.as_deref()));
    let external_url = view_stream_command_string(
        command,
        &[
            "externalUrl",
            "viewStreamExternalUrl",
            "remoteViewExternalUrl",
        ],
        "AGENT_BROWSER_REMOTE_VIEW_EXTERNAL_URL",
    )
    .or_else(|| guacamole_client_url(url.as_deref()))
    .or_else(|| frame_url.clone());
    let explicit_route_id = view_stream_command_string(
        command,
        &["routeId", "viewStreamRouteId", "guacamoleRouteId"],
        "AGENT_BROWSER_REMOTE_VIEW_ROUTE_ID",
    );
    let connection_id = view_stream_command_string(
        command,
        &["connectionId", "guacamoleConnectionId"],
        "AGENT_BROWSER_GUACAMOLE_CONNECTION_ID",
    )
    .or_else(|| {
        frame_url
            .as_deref()
            .or(external_url.as_deref())
            .or(url.as_deref())
            .and_then(guacamole_connection_id_from_url)
    });
    let route_id = explicit_route_id.or_else(|| {
        connection_id
            .as_ref()
            .map(|value| format!("guacamole:{}", value))
    });
    let connection_name = view_stream_command_string(
        command,
        &["connectionName", "guacamoleConnectionName"],
        "AGENT_BROWSER_GUACAMOLE_CONNECTION_NAME",
    );
    let route_descriptor = command
        .get("routeDescriptor")
        .cloned()
        .or_else(|| command.get("route_descriptor").cloned())
        .or_else(|| {
            command
                .get("params")
                .and_then(|params| params.get("routeDescriptor").cloned())
        });
    let display_allocation_id = optional_command_string(command, "displayAllocationId")
        .or_else(|| optional_command_string(command, "requestedDisplayAllocationId"))
        .or_else(|| {
            command.get("params").and_then(|params| {
                optional_command_string(params, "displayAllocationId")
                    .or_else(|| optional_command_string(params, "requestedDisplayAllocationId"))
            })
        });
    let provider_mode = optional_command_string(command, "providerMode").or_else(|| {
        command
            .get("params")
            .and_then(|params| optional_command_string(params, "providerMode"))
    });
    let route_source = if route_id.is_some()
        || connection_id.is_some()
        || frame_url.is_some()
        || external_url.is_some()
    {
        Some("service_request".to_string())
    } else {
        None
    };
    let url = url
        .or_else(|| frame_url.clone())
        .or_else(|| external_url.clone());
    let control_input = optional_command_string(command, "controlInput")
        .or_else(|| optional_command_string(command, "controlInputProvider"))
        .or_else(|| {
            command.get("params").and_then(|params| {
                optional_command_string(params, "controlInput")
                    .or_else(|| optional_command_string(params, "controlInputProvider"))
            })
        })
        .or_else(|| env::var("AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER").ok())
        .and_then(|provider| parse_control_input_provider(&provider))
        .or_else(|| default_control_input_provider(provider));
    vec![ViewStream {
        id: "remote-headed-view".to_string(),
        provider,
        control_input,
        url,
        frame_url,
        external_url,
        route_descriptor,
        route_id,
        display_allocation_id,
        connection_id,
        connection_name,
        route_source,
        provider_mode,
        viewer_lease_ids: Vec::new(),
        controller_lease_id: None,
        controller_epoch: 0,
        read_only: false,
        readiness: None,
        remote_readiness: None,
        attachability: None,
    }]
}
pub(crate) fn view_stream_command_string(
    command: &Value,
    keys: &[&str],
    env_key: &str,
) -> Option<String> {
    for key in keys {
        if let Some(value) = optional_command_string(command, key) {
            return Some(value);
        }
    }
    if let Some(params) = command.get("params") {
        for key in keys {
            if let Some(value) = optional_command_string(params, key) {
                return Some(value);
            }
        }
    }
    env::var(env_key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
pub(crate) fn guacamole_client_url(root_url: Option<&str>) -> Option<String> {
    if let Ok(configured_url) = env::var("AGENT_BROWSER_REMOTE_VIEW_URL") {
        let configured_url = configured_url.trim();
        if !configured_url.is_empty() && configured_url.contains("#/client/") {
            return Some(configured_url.to_string());
        }
    }
    let root_url = root_url.map(str::trim).filter(|url| !url.is_empty())?;
    if root_url.contains("#/client/") {
        return Some(root_url.to_string());
    }
    None
}
pub(crate) fn guacamole_connection_id_from_url(url: &str) -> Option<String> {
    let (_, route) = url.split_once("#/client/")?;
    let connection_id = route
        .split(['?', '&', '#', '/'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(connection_id.to_string())
}
pub(crate) fn parse_view_stream_provider(value: &str) -> Option<ViewStreamProvider> {
    match value.trim() {
        "cdp_screencast" | "cdp-screencast" => Some(ViewStreamProvider::CdpScreencast),
        "chrome_tab_webrtc" | "chrome-tab-webrtc" => Some(ViewStreamProvider::ChromeTabWebrtc),
        "virtual_display_webrtc" | "virtual-display-webrtc" => {
            Some(ViewStreamProvider::VirtualDisplayWebrtc)
        }
        "novnc" => Some(ViewStreamProvider::Novnc),
        "rdp_gateway" | "rdp-gateway" | "rdp" => Some(ViewStreamProvider::RdpGateway),
        "external_url" | "external-url" => Some(ViewStreamProvider::ExternalUrl),
        _ => None,
    }
}
pub(crate) fn parse_control_input_provider(value: &str) -> Option<ControlInputProvider> {
    match value.trim() {
        "cdp_input" | "cdp-input" | "cdp" => Some(ControlInputProvider::CdpInput),
        "webrtc_input" | "webrtc-input" | "webrtc" => Some(ControlInputProvider::WebrtcInput),
        "vnc_input" | "vnc-input" | "vnc" => Some(ControlInputProvider::VncInput),
        "manual_attached_desktop"
        | "manual-attached-desktop"
        | "manual_desktop"
        | "manual-desktop"
        | "manual" => Some(ControlInputProvider::ManualAttachedDesktop),
        _ => None,
    }
}
pub(crate) fn default_control_input_provider(
    provider: ViewStreamProvider,
) -> Option<ControlInputProvider> {
    let input = match provider {
        ViewStreamProvider::CdpScreencast => ControlInputProvider::CdpInput,
        ViewStreamProvider::ChromeTabWebrtc | ViewStreamProvider::VirtualDisplayWebrtc => {
            ControlInputProvider::WebrtcInput
        }
        ViewStreamProvider::Novnc => ControlInputProvider::VncInput,
        ViewStreamProvider::RdpGateway | ViewStreamProvider::ExternalUrl => {
            ControlInputProvider::ManualAttachedDesktop
        }
    };
    Some(input)
}
pub(crate) fn service_browser_host_for_launch(cmd: &Value, headless: bool) -> ServiceBrowserHost {
    if let Some(host) = browser_host_from_command(cmd) {
        return host;
    }
    if cmd.get("provider").is_some() {
        ServiceBrowserHost::CloudProvider
    } else if cmd.get("cdpUrl").is_some()
        || cmd.get("cdpPort").is_some()
        || cmd
            .get("autoConnect")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    {
        ServiceBrowserHost::AttachedExisting
    } else if headless {
        ServiceBrowserHost::LocalHeadless
    } else {
        ServiceBrowserHost::LocalHeaded
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn persist_service_browser_record(
    session_id: &str,
    host: ServiceBrowserHost,
    health: ServiceBrowserHealth,
    pid: Option<u32>,
    cdp_endpoint: Option<String>,
    last_error: Option<String>,
    metadata: Option<ServiceLaunchMetadata>,
    process_identity: Option<ServiceBrowserProcessIdentity>,
) {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        let _ = persist_service_browser_record_in_repository(
            &repository,
            session_id,
            host,
            health,
            pid,
            cdp_endpoint,
            last_error,
            metadata,
            process_identity,
        );
    }
}
pub(crate) fn cdp_stream_supported_host(host: ServiceBrowserHost) -> bool {
    matches!(
        host,
        ServiceBrowserHost::LocalHeadless
            | ServiceBrowserHost::LocalHeaded
            | ServiceBrowserHost::AttachedExisting
    )
}
pub(crate) fn cdp_screencast_url(port: u16) -> String {
    format!("http://127.0.0.1:{}/", port)
}
pub(crate) fn cdp_screencast_readiness(
    state: &str,
    reason: &str,
    session_id: &str,
    port: Option<u16>,
    cdp_endpoint: Option<&str>,
) -> Value {
    let mut readiness = json!(
        { "state" : state, "reason" : reason, "sessionName" : session_id, "browserId" :
        service_browser_id(session_id), }
    );
    if let Some(port) = port {
        readiness["streamPort"] = json!(port);
    }
    if let Some(endpoint) = cdp_endpoint {
        readiness["cdpEndpoint"] = json!(endpoint);
    }
    readiness
}
pub(crate) fn cdp_screencast_view_stream(
    session_id: &str,
    host: ServiceBrowserHost,
    health: ServiceBrowserHealth,
    cdp_endpoint: Option<&str>,
    stream_port: Option<u16>,
) -> Option<ViewStream> {
    if !cdp_stream_supported_host(host) {
        return None;
    }
    let (ready, reason) = if health != ServiceBrowserHealth::Ready {
        (false, "browser_not_ready")
    } else if cdp_endpoint
        .map(str::trim)
        .is_none_or(|endpoint| endpoint.is_empty())
    {
        (false, "missing_cdp_endpoint")
    } else if stream_port.is_none() {
        (false, "missing_stream_server")
    } else {
        (true, "stream_server_ready")
    };
    let url = if ready {
        stream_port.map(cdp_screencast_url)
    } else {
        None
    };
    Some(ViewStream {
        id: "cdp-screencast".to_string(),
        provider: ViewStreamProvider::CdpScreencast,
        control_input: ready.then_some(ControlInputProvider::CdpInput),
        url: url.clone(),
        frame_url: url.clone(),
        external_url: url,
        route_descriptor: None,
        route_id: None,
        display_allocation_id: None,
        connection_id: None,
        connection_name: None,
        route_source: Some("daemon_stream_server".to_string()),
        provider_mode: Some("simultaneous_view".to_string()),
        viewer_lease_ids: Vec::new(),
        controller_lease_id: None,
        controller_epoch: 0,
        read_only: !ready,
        readiness: Some(cdp_screencast_readiness(
            if ready { "ready" } else { "unavailable" },
            reason,
            session_id,
            stream_port,
            cdp_endpoint,
        )),
        remote_readiness: None,
        attachability: None,
    })
}
pub(crate) fn upsert_cdp_screencast_view_stream(
    metadata: &mut ServiceLaunchMetadata,
    session_id: &str,
    host: ServiceBrowserHost,
    health: ServiceBrowserHealth,
    cdp_endpoint: Option<&str>,
    stream_port: Option<u16>,
) {
    let Some(cdp_stream) =
        cdp_screencast_view_stream(session_id, host, health, cdp_endpoint, stream_port)
    else {
        return;
    };
    if let Some(existing) = metadata
        .view_streams
        .iter_mut()
        .find(|stream| stream.provider == ViewStreamProvider::CdpScreencast)
    {
        *existing = cdp_stream;
    } else {
        metadata.view_streams.push(cdp_stream);
    }
}
pub(crate) fn service_browser_session_id(browser: &BrowserProcess) -> Option<String> {
    browser
        .active_session_ids
        .iter()
        .find(|session_id| !session_id.trim().is_empty())
        .cloned()
        .or_else(|| {
            browser
                .id
                .strip_prefix("session:")
                .filter(|session_id| !session_id.trim().is_empty())
                .map(ToOwned::to_owned)
        })
}
pub(crate) fn read_stream_port_for_session(session_id: &str) -> Option<u16> {
    fs::read_to_string(stream_file_path(session_id))
        .ok()
        .and_then(|contents| contents.trim().parse::<u16>().ok())
}
pub(crate) fn upsert_browser_cdp_screencast_view_stream(browser: &mut BrowserProcess) {
    let Some(session_id) = service_browser_session_id(browser) else {
        return;
    };
    let stream_port = read_stream_port_for_session(&session_id);
    let Some(cdp_stream) = cdp_screencast_view_stream(
        &session_id,
        browser.host,
        browser.health,
        browser.cdp_endpoint.as_deref(),
        stream_port,
    ) else {
        return;
    };
    if let Some(existing) = browser
        .view_streams
        .iter_mut()
        .find(|stream| stream.provider == ViewStreamProvider::CdpScreencast)
    {
        *existing = cdp_stream;
    } else {
        browser.view_streams.push(cdp_stream);
    }
}
pub(crate) fn refresh_cdp_screencast_view_streams(service_state: &mut ServiceState) {
    for browser in service_state.browsers.values_mut() {
        upsert_browser_cdp_screencast_view_stream(browser);
    }
}
pub(crate) fn persist_current_browser_health(
    state: &mut DaemonState,
    host: ServiceBrowserHost,
    health: ServiceBrowserHealth,
    metadata: Option<ServiceLaunchMetadata>,
) -> Result<(), String> {
    register_current_browser_lifecycle(state)?;
    let preserves_existing_metadata = metadata.is_none();
    let (pid, cdp_endpoint, browser_stderr_log_path) = state
        .browser
        .as_ref()
        .map(|mgr| {
            (
                mgr.browser_pid().or(state.attached_browser_pid),
                Some(mgr.get_cdp_url().to_string()),
                mgr.browser_stderr_log_path()
                    .map(|path| path.to_string_lossy().to_string()),
            )
        })
        .unwrap_or((None, None, None));
    let process_identity = state.browser.as_ref().and_then(|manager| {
        let pid = manager.browser_pid().or(state.attached_browser_pid)?;
        crate::process_identity::capture_process_identity(pid, None, None).map(|process_identity| {
            ServiceBrowserProcessIdentity {
                process_identity,
                user_data_dir: manager
                    .browser_user_data_dir()
                    .map(|path| path.to_string_lossy().into_owned()),
                runtime_profile: manager.runtime_profile_name().map(str::to_string),
            }
        })
    });
    let metadata = metadata.map(|mut metadata| {
        metadata.browser_stderr_log_path = browser_stderr_log_path;
        if metadata.display_name.is_none() {
            metadata.display_name = state
                .browser
                .as_ref()
                .and_then(|mgr| mgr.browser_display_name().map(str::to_string));
        }
        upsert_cdp_screencast_view_stream(
            &mut metadata,
            &state.session_id,
            host,
            health,
            cdp_endpoint.as_deref(),
            state.stream_server.as_ref().map(|server| server.port()),
        );
        metadata
    });
    persist_service_browser_record(
        &state.session_id,
        host,
        health,
        pid,
        cdp_endpoint,
        None,
        metadata,
        process_identity,
    );
    if preserves_existing_metadata {
        if let Ok(repository) = LockedServiceStateRepository::default_json() {
            let _ = repository.mutate(|service_state| {
                refresh_cdp_screencast_view_streams(service_state);
                Ok(())
            });
        }
    }
    Ok(())
}

fn register_current_browser_lifecycle(state: &mut DaemonState) -> Result<(), String> {
    let Some(manager) = state.browser.as_ref() else {
        return Ok(());
    };
    let Some(pid) = manager.browser_pid().or(state.attached_browser_pid) else {
        return Ok(());
    };
    let profile_root = manager
        .browser_user_data_dir()
        .map(Path::to_path_buf)
        .or_else(|| {
            state
                .attached_runtime_profile
                .as_deref()
                .and_then(|profile| {
                    crate::runtime_profile::runtime_profile_user_data_dir(profile).ok()
                })
        });
    let Some(profile_root) = profile_root else {
        if manager.browser_pid().is_some() {
            return Err("runtime_lifecycle_owned_browser_profile_unavailable".to_string());
        }
        return Ok(());
    };
    let process_identity = crate::process_identity::capture_process_identity(pid, None, None)
        .ok_or_else(|| "runtime_lifecycle_process_identity_unavailable".to_string())?;
    let cdp_endpoint = manager.get_cdp_url().to_string();
    let target_ids = manager
        .pages_list()
        .into_iter()
        .map(|page| page.target_id)
        .collect::<Vec<_>>();
    let logical_browser_id = super::capability::service_browser_id(&state.session_id);
    let repository = LockedServiceStateRepository::default_json()?;
    let authority = crate::native::runtime_lifecycle::RuntimeLifecycleAuthority::new(&repository);
    let registration = crate::native::runtime_lifecycle::ManagedLaneRegistration {
        logical_browser_id,
        profile_root,
        daemon_session_route: state.session_id.clone(),
        process_group_id: crate::process_identity::observe_process_group_id(pid),
        process_identity: process_identity.clone(),
        browser_family: state.engine.clone(),
        cdp_endpoint,
        target_ids,
    };
    let binding = authority.register_managed_lane(registration)?;
    state.runtime_owner_binding = Some(binding);
    let binding = state
        .runtime_owner_binding
        .as_ref()
        .expect("newly registered runtime owner binding remains present");
    let reviewed_process_tree = authority.reviewed_process_tree(binding, &process_identity)?;
    if let Some(manager) = state.browser.as_mut() {
        manager.mark_lifecycle_managed(reviewed_process_tree);
    }
    Ok(())
}
/// Enforces service-owned profile leases before Chrome starts.
///
/// The control-plane scheduler handles bounded `wait` policy by requeueing the
/// request so the worker can run other jobs. This launch-path guard remains as
/// a deterministic fallback for direct execution and rejects unresolved waits.
/// The same retained session may still reuse its browser, and non-service
/// launches keep the existing direct-control behavior.
pub(crate) async fn ensure_service_profile_lease_available(
    _metadata: &ServiceLaunchMetadata,
    session_id: &str,
    command: &Value,
) -> Result<(), String> {
    let wait_timeout_ms = profile_lease_wait_timeout_ms_from_command(command)?;
    match service_profile_lease_gate(command, session_id, Some(wait_timeout_ms))? {
        ServiceProfileLeaseGate::Ready => Ok(()),
        ServiceProfileLeaseGate::Reject { error } => Err(error),
        ServiceProfileLeaseGate::Wait { .. } => Err(
            "Service profile lease wait must be handled by the control-plane scheduler".to_string(),
        ),
    }
}
