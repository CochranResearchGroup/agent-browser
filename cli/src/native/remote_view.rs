use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::env;
#[cfg(all(unix, not(test)))]
use std::fs;
#[cfg(all(unix, not(test)))]
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::runtime_profile::read_runtime_state;

use super::service_model::{
    BrowserHost, DisplayAllocation, RoutePoolEntry, ServiceState, ViewStreamProvider,
};

const ROUTE_DISPLAY_CONTENT_TTL: Duration = Duration::from_secs(5);
const ROUTE_DISPLAY_NAME_TTL: Duration = Duration::from_secs(10);

struct RouteDisplayContentCacheEntry {
    observed_at: Instant,
    content: Value,
}

struct RouteDisplayNameCacheEntry {
    observed_at: Instant,
    names: HashSet<String>,
}

static ROUTE_DISPLAY_CONTENT_CACHE: OnceLock<
    Mutex<HashMap<String, RouteDisplayContentCacheEntry>>,
> = OnceLock::new();
static ROUTE_DISPLAY_NAME_CACHE: OnceLock<Mutex<RouteDisplayNameCacheEntry>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewRouteBinding {
    pub route_id: String,
    pub route_pool_entry_id: Option<String>,
    pub display_allocation_id: String,
    pub route_pool_entry_state: Option<String>,
    pub current_route_allocation_id: Option<String>,
    pub display_name: Option<String>,
    pub launch_display_name: Option<String>,
    pub display_isolation: String,
    pub route_user: Option<String>,
    pub display_access: Option<Value>,
    pub provider: ViewStreamProvider,
    pub provider_mode: String,
    pub connection_id: Option<String>,
    pub connection_name: Option<String>,
    pub frame_url: Option<String>,
    pub external_url: Option<String>,
    pub route_descriptor: Option<Value>,
    pub readiness: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewOpenIntent {
    pub url: Option<String>,
    pub runtime_profile: Option<String>,
    pub profile: Option<String>,
    pub browser_id: Option<String>,
    pub session_name: Option<String>,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub task_name: Option<String>,
    pub browser_build: Option<String>,
    pub browser_host: String,
    pub view_stream_provider: ViewStreamProvider,
    pub control_input: String,
    pub route_pool_entry_id: Option<String>,
    pub route_id: Option<String>,
    pub display_allocation_id: Option<String>,
    pub remote_headed_display: Option<String>,
    pub display_isolation: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewAcquisitionDecision {
    pub step: String,
    pub outcome: String,
    pub reason: String,
    pub detail: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewAcquisitionBlocker {
    pub code: String,
    pub message: String,
    pub detail: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteViewAcquisitionPlan {
    pub mode: String,
    pub reuse_policy: String,
    pub tab_policy: String,
    pub requested_profile: Option<String>,
    pub requested_browser_build: Option<String>,
    pub requested_browser_host: String,
    pub requested_view_stream_provider: ViewStreamProvider,
    pub requested_control_input: String,
    pub requested_display_isolation: Option<String>,
    pub requested_route_pool_entry_id: Option<String>,
    pub requested_route_id: Option<String>,
    pub selected_route_pool_entry_id: Option<String>,
    pub selected_route_id: String,
    pub display_allocation_id: String,
    pub display_name: Option<String>,
    pub route_binding: RemoteViewRouteBinding,
    pub decisions: Vec<RemoteViewAcquisitionDecision>,
    pub blockers: Vec<RemoteViewAcquisitionBlocker>,
    pub proof_required: Vec<String>,
    pub cleanup_on_failure: Vec<String>,
    pub suggested_commands: Vec<String>,
}

fn acquisition_decision(
    step: impl Into<String>,
    outcome: impl Into<String>,
    reason: impl Into<String>,
    detail: Option<Value>,
) -> RemoteViewAcquisitionDecision {
    RemoteViewAcquisitionDecision {
        step: step.into(),
        outcome: outcome.into(),
        reason: reason.into(),
        detail,
    }
}

fn acquisition_blocker(
    code: impl Into<String>,
    message: impl Into<String>,
    detail: Option<Value>,
) -> RemoteViewAcquisitionBlocker {
    RemoteViewAcquisitionBlocker {
        code: code.into(),
        message: message.into(),
        detail,
    }
}

fn command_or_params_string(command: &Value, key: &str) -> Option<String> {
    command
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            command
                .get("params")
                .and_then(|params| command_or_params_string(params, key))
        })
}

fn command_or_params_bool(command: &Value, key: &str) -> Option<bool> {
    command.get(key).and_then(Value::as_bool).or_else(|| {
        command
            .get("params")
            .and_then(|params| command_or_params_bool(params, key))
    })
}

fn parse_remote_view_provider(value: &str) -> Option<ViewStreamProvider> {
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

fn remote_view_provider_label(provider: ViewStreamProvider) -> &'static str {
    match provider {
        ViewStreamProvider::CdpScreencast => "cdp_screencast",
        ViewStreamProvider::ChromeTabWebrtc => "chrome_tab_webrtc",
        ViewStreamProvider::VirtualDisplayWebrtc => "virtual_display_webrtc",
        ViewStreamProvider::Novnc => "novnc",
        ViewStreamProvider::RdpGateway => "rdp_gateway",
        ViewStreamProvider::ExternalUrl => "external_url",
    }
}

pub fn normalize_remote_view_open_intent(command: &Value) -> Result<RemoteViewOpenIntent, String> {
    let view_stream_provider = command_or_params_string(command, "viewStreamProvider")
        .or_else(|| command_or_params_string(command, "viewStream"))
        .map(|value| {
            parse_remote_view_provider(&value).ok_or_else(|| {
                format!(
                    "invalid_view_stream_provider: remote_view_open does not recognize viewStreamProvider '{}'",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(ViewStreamProvider::RdpGateway);

    if let Some(provider) = command_or_params_string(command, "provider") {
        let alias = parse_remote_view_provider(&provider).ok_or_else(|| {
            format!(
                "provider_ambiguity: remote_view_open provider '{}' is not a cloud browser provider; use viewStreamProvider for remote-view streams",
                provider
            )
        })?;
        if alias != ViewStreamProvider::RdpGateway {
            return Err(format!(
                "provider_ambiguity: remote_view_open provider '{}' is ambiguous; use viewStreamProvider='{}' for remote-view streams",
                provider,
                remote_view_provider_label(alias)
            ));
        }
        if view_stream_provider != ViewStreamProvider::RdpGateway {
            return Err(format!(
                "provider_ambiguity: remote_view_open provider 'rdp_gateway' conflicts with viewStreamProvider '{}'",
                remote_view_provider_label(view_stream_provider)
            ));
        }
    }

    Ok(RemoteViewOpenIntent {
        url: command_or_params_string(command, "url"),
        runtime_profile: command_or_params_string(command, "runtimeProfile"),
        profile: command_or_params_string(command, "profile"),
        browser_id: command_or_params_string(command, "browserId"),
        session_name: command_or_params_string(command, "sessionName"),
        service_name: command_or_params_string(command, "serviceName"),
        agent_name: command_or_params_string(command, "agentName"),
        task_name: command_or_params_string(command, "taskName"),
        browser_build: command_or_params_string(command, "browserBuild"),
        browser_host: command_or_params_string(command, "browserHost")
            .unwrap_or_else(|| "remote_headed".to_string()),
        view_stream_provider,
        control_input: command_or_params_string(command, "controlInput")
            .or_else(|| command_or_params_string(command, "controlInputProvider"))
            .unwrap_or_else(|| "manual_attached_desktop".to_string()),
        route_pool_entry_id: command_or_params_string(command, "routePoolEntryId"),
        route_id: command_or_params_string(command, "routeId")
            .or_else(|| command_or_params_string(command, "remoteViewRouteId")),
        display_allocation_id: command_or_params_string(command, "displayAllocationId"),
        remote_headed_display: command_or_params_string(command, "remoteHeadedDisplay")
            .or_else(|| command_or_params_string(command, "display"))
            .or_else(|| command_or_params_string(command, "displayName")),
        display_isolation: command_or_params_string(command, "displayIsolation"),
        dry_run: command_or_params_bool(command, "dryRun").unwrap_or(false),
    })
}

pub fn route_pool_target_string(entry: &RoutePoolEntry, key: &str) -> Option<String> {
    entry
        .target
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

#[cfg(all(unix, not(test)))]
/// Returns whether the named X11 display currently has a filesystem or abstract socket.
pub(crate) fn route_display_socket_available(display_name: &str) -> bool {
    let Some(display_number) = display_name.strip_prefix(':') else {
        return false;
    };
    if display_number.is_empty() || !display_number.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    let socket_path = format!("/tmp/.X11-unix/X{display_number}");
    if Path::new(&socket_path).exists() {
        return true;
    }

    let abstract_socket = format!("@{socket_path}");
    fs::read_to_string("/proc/net/unix")
        .map(|unix_sockets| {
            unix_sockets
                .lines()
                .any(|line| line.contains(&abstract_socket))
        })
        .unwrap_or(false)
}

#[cfg(any(not(unix), test))]
/// Treats display sockets as available where Linux socket inspection is unsupported or injected by tests.
pub(crate) fn route_display_socket_available(_display_name: &str) -> bool {
    true
}

pub fn route_pool_entry_request_matches(
    entry: &RoutePoolEntry,
    requested_pool_entry_id: Option<&str>,
    requested_route_id: Option<&str>,
    provider: ViewStreamProvider,
) -> bool {
    entry.provider == provider
        && requested_pool_entry_id
            .map(|id| entry.id == id)
            .unwrap_or(true)
        && requested_route_id
            .map(|route_id| entry.route_id == route_id)
            .unwrap_or(true)
}

fn select_route_pool_entry_for_unbound_display<'a>(
    state: &'a ServiceState,
    requested_pool_entry_id: Option<&str>,
    requested_route_id: Option<&str>,
    provider: ViewStreamProvider,
) -> Option<&'a RoutePoolEntry> {
    if let Some(id) = requested_pool_entry_id {
        return state.route_pool.get(id);
    }
    state
        .route_pool
        .values()
        .filter(|entry| entry.provider == provider)
        .filter(|entry| {
            requested_route_id
                .map(|route_id| entry.route_id == route_id)
                .unwrap_or(true)
        })
        .find(|entry| entry.state == "available")
}

pub fn checked_out_route_matches_owner(
    state: &ServiceState,
    route_id: &str,
    browser_id: &str,
    session_id: &str,
    display_allocation_id: &str,
) -> bool {
    state.remote_view_routes.get(route_id).is_some_and(|route| {
        matches!(
            route.state.as_str(),
            "ready" | "pending" | "checked_out" | "orphaned"
        ) && route.display_allocation_id.as_deref() == Some(display_allocation_id)
            && route.browser_id.as_deref() == Some(browser_id)
            && route.session_id.as_deref() == Some(session_id)
    })
}

#[allow(clippy::too_many_arguments)]
fn checked_out_route_pool_entry_id_for_owner(
    state: &ServiceState,
    requested_pool_entry_id: Option<&str>,
    requested_route_id: Option<&str>,
    display_allocation_id: &str,
    allocation: Option<&DisplayAllocation>,
    browser_id: &str,
    session_id: &str,
    provider: ViewStreamProvider,
) -> Option<String> {
    state
        .route_pool
        .values()
        .filter(|entry| entry.provider == provider)
        .filter(|entry| {
            requested_pool_entry_id
                .map(|id| entry.id == id)
                .unwrap_or(true)
        })
        .filter(|entry| {
            requested_route_id
                .map(|route_id| entry.route_id == route_id)
                .unwrap_or(true)
        })
        .filter(|entry| {
            matches!(
                entry.state.as_str(),
                "available" | "checked_out" | "pending"
            )
        })
        .filter(|entry| route_pool_entry_matches_display(entry, display_allocation_id, allocation))
        .find(|entry| {
            let route_id = entry
                .current_route_allocation_id
                .as_deref()
                .or_else(|| (!entry.route_id.trim().is_empty()).then_some(entry.route_id.as_str()));
            route_id.is_some_and(|route_id| {
                checked_out_route_matches_owner(
                    state,
                    route_id,
                    browser_id,
                    session_id,
                    display_allocation_id,
                )
            })
        })
        .map(|entry| entry.id.clone())
}

fn checked_out_route_display_allocation_id(
    state: &ServiceState,
    requested_pool_entry_id: Option<&str>,
    requested_route_id: Option<&str>,
) -> Option<String> {
    let entry = requested_pool_entry_id
        .and_then(|id| state.route_pool.get(id))
        .or_else(|| {
            requested_route_id.and_then(|route_id| {
                state
                    .route_pool
                    .values()
                    .find(|entry| entry.route_id == route_id)
            })
        })?;
    if entry.state != "checked_out" {
        return None;
    }
    let route_id = entry
        .current_route_allocation_id
        .as_deref()
        .or_else(|| (!entry.route_id.trim().is_empty()).then_some(entry.route_id.as_str()))?;
    state
        .remote_view_routes
        .get(route_id)
        .and_then(|route| route.display_allocation_id.clone())
}

fn requested_profile_hint(intent: &RemoteViewOpenIntent) -> Option<&str> {
    intent
        .runtime_profile
        .as_deref()
        .or(intent.profile.as_deref())
}

fn runtime_profile_pid_matches(runtime_profile: &str, browser_pid: Option<u32>) -> bool {
    let Some(browser_pid) = browser_pid else {
        return false;
    };
    read_runtime_state(runtime_profile)
        .ok()
        .flatten()
        .is_some_and(|state| state.browser_pid == browser_pid)
}

fn profile_id_matches_request(
    actual_profile_id: Option<&str>,
    requested_profile: &str,
    requested_runtime_profile: Option<&str>,
    browser_pid: Option<u32>,
) -> bool {
    if actual_profile_id == Some(requested_profile) {
        return true;
    }
    requested_runtime_profile.is_some_and(|runtime_profile| {
        actual_profile_id.is_some_and(|profile_id| profile_id.starts_with("custom:"))
            && runtime_profile_pid_matches(runtime_profile, browser_pid)
    })
}

fn allocation_reuse_mismatches(
    state: &ServiceState,
    allocation: Option<&DisplayAllocation>,
    browser_id: &str,
    session_id: &str,
    intent: &RemoteViewOpenIntent,
) -> Vec<String> {
    let mut mismatches = Vec::new();
    let Some(allocation) = allocation else {
        return mismatches;
    };
    if allocation.state != "ready" {
        mismatches.push(format!("display_allocation_state:{}", allocation.state));
    }
    if allocation
        .owner_browser_id
        .as_deref()
        .is_some_and(|owner| owner != browser_id)
    {
        mismatches.push("owner_browser_mismatch".to_string());
    }
    if allocation
        .owner_session_id
        .as_deref()
        .is_some_and(|owner| owner != session_id)
    {
        mismatches.push("owner_session_mismatch".to_string());
    }
    if let Some(requested_profile) = requested_profile_hint(intent) {
        let allocation_owner_pid = allocation
            .owner_browser_id
            .as_deref()
            .and_then(|browser_id| state.browsers.get(browser_id))
            .and_then(|browser| browser.pid);
        if allocation.profile_id.as_deref().is_some_and(|profile| {
            !profile_id_matches_request(
                Some(profile),
                requested_profile,
                intent.runtime_profile.as_deref(),
                allocation_owner_pid,
            )
        }) {
            mismatches.push("profile_mismatch".to_string());
        }
    }
    if let Some(requested_build) = intent.browser_build.as_deref() {
        if allocation
            .browser_build
            .as_deref()
            .is_some_and(|build| build != requested_build)
        {
            mismatches.push("browser_build_mismatch".to_string());
        }
    }
    if allocation
        .host
        .is_some_and(|host| host != BrowserHost::RemoteHeaded)
    {
        mismatches.push("host_mismatch".to_string());
    }
    if let Some(browser) = state.browsers.get(browser_id) {
        if browser.host != BrowserHost::RemoteHeaded {
            mismatches.push("browser_host_mismatch".to_string());
        }
        if let Some(requested_profile) = requested_profile_hint(intent) {
            if browser.profile_id.as_deref().is_some_and(|profile| {
                !profile_id_matches_request(
                    Some(profile),
                    requested_profile,
                    intent.runtime_profile.as_deref(),
                    browser.pid,
                )
            }) {
                mismatches.push("browser_profile_mismatch".to_string());
            }
        }
    }
    mismatches
}

fn command_display_allocation_from_intent(
    intent: &RemoteViewOpenIntent,
    display_allocation_id: &str,
    existing: Option<&DisplayAllocation>,
) -> Option<DisplayAllocation> {
    if existing.is_some() {
        return None;
    }
    let display_name = intent.remote_headed_display.clone()?;
    Some(DisplayAllocation {
        id: display_allocation_id.to_string(),
        display_name: Some(display_name),
        display_isolation: intent
            .display_isolation
            .clone()
            .unwrap_or_else(|| "shared_display".to_string()),
        state: "ready".to_string(),
        ..DisplayAllocation::default()
    })
}

pub fn plan_remote_view_acquisition(
    state: &ServiceState,
    intent: &RemoteViewOpenIntent,
    inline_route_pool_entry: Option<&RoutePoolEntry>,
    browser_id: &str,
    session_id: &str,
) -> Result<RemoteViewAcquisitionPlan, String> {
    let provider = intent.view_stream_provider;
    let route_pool_entry_id = intent.route_pool_entry_id.clone();
    let requested_route_id = inline_route_pool_entry
        .filter(|entry| route_pool_entry_id.as_deref() == Some(entry.id.as_str()))
        .map(|entry| entry.route_id.clone())
        .or_else(|| intent.route_id.clone());
    let mut decisions = Vec::new();
    let mut blockers = Vec::new();

    let inline_display_allocation_id = inline_route_pool_entry
        .filter(|entry| {
            route_pool_entry_request_matches(
                entry,
                route_pool_entry_id.as_deref(),
                requested_route_id.as_deref(),
                provider,
            )
        })
        .map(display_allocation_id_for_route_pool_entry);
    let explicit_display_allocation_id = intent.display_allocation_id.clone();
    let browser_display_allocation_id = state
        .browsers
        .get(browser_id)
        .and_then(|browser| browser.display_allocation_id.clone());
    let same_owner_browser_route_pool_entry_id =
        browser_display_allocation_id.as_deref().and_then(|display_id| {
            let allocation = state.display_allocations.get(display_id);
            let mismatches =
                allocation_reuse_mismatches(state, allocation, browser_id, session_id, intent);
            if mismatches.is_empty() {
                checked_out_route_pool_entry_id_for_owner(
                    state,
                    route_pool_entry_id.as_deref(),
                    requested_route_id.as_deref(),
                    display_id,
                    allocation,
                    browser_id,
                    session_id,
                    provider,
                )
            } else {
                decisions.push(acquisition_decision(
                    "browser_reuse",
                    "skipped",
                    "existing browser display allocation is not eligible for strict operator open reuse",
                    Some(json!({
                        "displayAllocationId": display_id,
                        "mismatches": mismatches,
                    })),
                ));
                None
            }
        });
    let same_owner_display_allocation_id = same_owner_browser_route_pool_entry_id
        .as_deref()
        .and(browser_display_allocation_id.clone());
    let checked_out_display_allocation_id = checked_out_route_display_allocation_id(
        state,
        route_pool_entry_id.as_deref(),
        requested_route_id.as_deref(),
    );
    let explicit_route_pool_display_allocation_id = route_pool_entry_id
        .as_deref()
        .and_then(|id| state.route_pool.get(id))
        .filter(|entry| {
            route_pool_entry_request_matches(entry, route_pool_entry_id.as_deref(), None, provider)
        })
        .map(display_allocation_id_for_route_pool_entry);
    let available_route_pool_entry = select_route_pool_entry_for_unbound_display(
        state,
        route_pool_entry_id.as_deref(),
        requested_route_id.as_deref(),
        provider,
    );
    let available_route_pool_display_allocation_id =
        available_route_pool_entry.map(display_allocation_id_for_route_pool_entry);
    let route_pool_capacity_available = available_route_pool_display_allocation_id.is_some();
    let same_owner_display_available = same_owner_display_allocation_id.is_some();
    let exhausted_display_allocation_id = inline_route_pool_entry
        .filter(|entry| entry.state == "checked_out")
        .map(display_allocation_id_for_route_pool_entry)
        .or_else(|| browser_display_allocation_id.clone());
    if route_pool_entry_id.is_none()
        && requested_route_id.is_none()
        && !route_pool_capacity_available
        && !same_owner_display_available
        && explicit_display_allocation_id.is_none()
        && exhausted_display_allocation_id.is_some()
    {
        let display_allocation_id =
            exhausted_display_allocation_id.unwrap_or_else(|| "unknown".to_string());
        let allocation = state.display_allocations.get(&display_allocation_id);
        let detail = route_pool_request_diagnostic(
            state,
            route_pool_entry_id.as_deref(),
            requested_route_id.as_deref(),
            &display_allocation_id,
            allocation,
            provider,
        );
        blockers.push(acquisition_blocker(
            "route_pool_exhausted",
            "no available route-pool entries remain for unpinned route-bound demand",
            Some(detail.clone()),
        ));
        return Err(format!(
            "route_pool_exhausted: no available route-pool entries remain; diagnostic={}",
            compact_json(&detail)
        ));
    }

    let (display_allocation_id, display_reason) = if let Some(id) = inline_display_allocation_id {
        (id, "inline_route_entry")
    } else if let Some(id) = explicit_display_allocation_id {
        (id, "explicit_display_allocation")
    } else if let Some(id) = same_owner_display_allocation_id {
        (id, "same_owner_checked_out_browser_route")
    } else if let Some(id) = checked_out_display_allocation_id {
        (id, "checked_out_requested_route")
    } else if let Some(id) = explicit_route_pool_display_allocation_id {
        (id, "explicit_route_pool_entry")
    } else if let Some(id) = available_route_pool_display_allocation_id {
        (id, "available_route_pool_entry")
    } else if let Some(id) = browser_display_allocation_id {
        decisions.push(acquisition_decision(
            "browser_reuse",
            "fallback",
            "using existing browser display allocation only because no route-pool display was available",
            Some(json!({ "displayAllocationId": id })),
        ));
        (id, "existing_browser_display_fallback")
    } else {
        return Err("service_remote_view_route_preflight requires displayAllocationId, a browser with displayAllocationId, or an available route pool entry".to_string());
    };

    decisions.push(acquisition_decision(
        "display_allocation",
        "selected",
        display_reason,
        Some(json!({ "displayAllocationId": display_allocation_id })),
    ));

    let existing_display_allocation = state.display_allocations.get(&display_allocation_id);
    let selected_checked_out_route_pool_entry = state
        .route_pool
        .values()
        .filter(|entry| entry.provider == provider)
        .filter(|entry| entry.state == "checked_out")
        .find(|entry| {
            route_pool_entry_matches_display(
                entry,
                &display_allocation_id,
                existing_display_allocation,
            )
        });
    if route_pool_entry_id.is_none()
        && requested_route_id.is_none()
        && !route_pool_capacity_available
        && !same_owner_display_available
        && selected_checked_out_route_pool_entry.is_some()
        && checked_out_route_pool_entry_id_for_owner(
            state,
            route_pool_entry_id.as_deref(),
            requested_route_id.as_deref(),
            &display_allocation_id,
            existing_display_allocation,
            browser_id,
            session_id,
            provider,
        )
        .is_none()
    {
        let detail = route_pool_request_diagnostic(
            state,
            route_pool_entry_id.as_deref(),
            requested_route_id.as_deref(),
            &display_allocation_id,
            existing_display_allocation,
            provider,
        );
        blockers.push(acquisition_blocker(
            "route_pool_exhausted",
            "no available route-pool entries remain for unpinned route-bound demand",
            Some(detail.clone()),
        ));
        return Err(format!(
            "route_pool_exhausted: no available route-pool entries remain; diagnostic={}",
            compact_json(&detail)
        ));
    }
    if let Some(allocation) = existing_display_allocation {
        let allocation_is_inactive = matches!(
            allocation.state.as_str(),
            "released" | "orphaned" | "failed" | "unavailable"
        ) || allocation
            .readiness
            .as_ref()
            .and_then(readiness_state)
            .is_some_and(|state| {
                matches!(
                    state.as_str(),
                    "released" | "orphaned" | "failed" | "unavailable"
                )
            });
        if allocation
            .owner_session_id
            .as_deref()
            .is_some_and(|owner| owner != session_id)
            && !allocation_is_inactive
        {
            let detail = route_pool_request_diagnostic(
                state,
                route_pool_entry_id.as_deref(),
                requested_route_id.as_deref(),
                &display_allocation_id,
                existing_display_allocation,
                provider,
            );
            blockers.push(acquisition_blocker(
                "display_allocation_owner_mismatch",
                format!(
                    "display allocation '{}' belongs to session '{}', not '{}'",
                    display_allocation_id,
                    allocation.owner_session_id.as_deref().unwrap_or("unknown"),
                    session_id
                ),
                Some(detail.clone()),
            ));
            return Err(format!(
                "display_allocation_owner_mismatch: display allocation '{}' belongs to another session; diagnostic={}",
                display_allocation_id,
                compact_json(&detail)
            ));
        }
    }

    let selected_route_pool_entry_id = if let Some(id) = same_owner_browser_route_pool_entry_id {
        decisions.push(acquisition_decision(
            "route_pool_entry",
            "selected",
            "same_owner_checked_out_route",
            Some(json!({ "routePoolEntryId": id })),
        ));
        Some(id)
    } else if let Some(id) = checked_out_route_pool_entry_id_for_owner(
        state,
        route_pool_entry_id.as_deref(),
        requested_route_id.as_deref(),
        &display_allocation_id,
        existing_display_allocation,
        browser_id,
        session_id,
        provider,
    ) {
        decisions.push(acquisition_decision(
            "route_pool_entry",
            "selected",
            "same_owner_checked_out_route",
            Some(json!({ "routePoolEntryId": id })),
        ));
        Some(id)
    } else {
        let selected = resolve_route_pool_entry_id(
            state,
            route_pool_entry_id.as_deref(),
            requested_route_id.as_deref(),
            &display_allocation_id,
            existing_display_allocation,
            provider,
        )?;
        if let Some(id) = selected.as_ref() {
            decisions.push(acquisition_decision(
                "route_pool_entry",
                "selected",
                "available_or_explicit_route_pool_entry",
                Some(json!({ "routePoolEntryId": id })),
            ));
        } else {
            decisions.push(acquisition_decision(
                "route_pool_entry",
                "skipped",
                "no route pool entry required for unbound route",
                None,
            ));
        }
        selected
    };

    let pool_entry = if let Some(id) = selected_route_pool_entry_id.as_ref() {
        inline_route_pool_entry
            .filter(|entry| entry.id == *id)
            .or_else(|| state.route_pool.get(id))
    } else {
        inline_route_pool_entry.filter(|entry| {
            route_pool_entry_request_matches(
                entry,
                route_pool_entry_id.as_deref(),
                requested_route_id.as_deref(),
                provider,
            )
        })
    };
    let command_display_allocation = command_display_allocation_from_intent(
        intent,
        &display_allocation_id,
        existing_display_allocation,
    );
    let display_allocation = existing_display_allocation.or(command_display_allocation.as_ref());
    if let Some(id) = selected_route_pool_entry_id.as_ref() {
        if pool_entry.is_none() {
            let detail = route_pool_request_diagnostic(
                state,
                route_pool_entry_id.as_deref(),
                requested_route_id.as_deref(),
                &display_allocation_id,
                existing_display_allocation,
                provider,
            );
            blockers.push(acquisition_blocker(
                "route_pool_entry_missing",
                format!("route pool entry '{}' not found", id),
                Some(detail.clone()),
            ));
            return Err(format!(
                "route_pool_entry_missing: route pool entry '{}' not found; diagnostic={}",
                id,
                compact_json(&detail)
            ));
        }
    }
    if let Some(entry) = pool_entry {
        let reusable_route_id = requested_route_id
            .as_deref()
            .or_else(|| (!entry.route_id.trim().is_empty()).then_some(entry.route_id.as_str()));
        let same_owner_checked_out = matches!(entry.state.as_str(), "checked_out" | "pending")
            && entry.current_route_allocation_id.as_deref() == reusable_route_id
            && reusable_route_id.is_some_and(|route_id| {
                checked_out_route_matches_owner(
                    state,
                    route_id,
                    browser_id,
                    session_id,
                    &display_allocation_id,
                )
            });
        let entry_available = entry.state == "available"
            || same_owner_checked_out
            || (entry.state != "checked_out"
                && entry.readiness.as_ref().is_some_and(|readiness| {
                    readiness
                        .get("state")
                        .and_then(Value::as_str)
                        .is_some_and(|state| state.trim() == "ready")
                        || readiness_state(readiness).as_deref() == Some("ready")
                }));
        if !entry_available {
            let detail = route_pool_request_diagnostic(
                state,
                route_pool_entry_id.as_deref(),
                requested_route_id.as_deref(),
                &display_allocation_id,
                display_allocation,
                provider,
            );
            blockers.push(acquisition_blocker(
                "route_pool_entry_unavailable",
                format!(
                    "route pool entry '{}' is not available for checkout",
                    entry.id
                ),
                Some(detail.clone()),
            ));
            return Err(format!(
                "route_pool_entry_unavailable: route pool entry '{}' is not available for checkout; diagnostic={}",
                entry.id,
                compact_json(&detail)
            ));
        }
    }

    let route_binding = build_route_binding(
        pool_entry,
        &display_allocation_id,
        display_allocation,
        requested_route_id.as_deref(),
        provider,
    )?;
    let selected_route_id = route_binding.route_id.clone();
    let suggested_commands = route_pool_recommended_commands(state, provider);
    let proof_required = vec![
        "cdp_target_url".to_string(),
        "x11_browser_window_visible".to_string(),
        "route_pool_display_browser_session_agreement".to_string(),
        "guacamole_route_routable".to_string(),
    ];
    let cleanup_on_failure = vec![
        "close_opened_tab_for_reused_browser".to_string(),
        "close_new_browser_for_new_launch".to_string(),
        "preserve_compact_diagnostic_event".to_string(),
    ];

    Ok(RemoteViewAcquisitionPlan {
        mode: "strict_operator_open".to_string(),
        reuse_policy: "reuse_only_when_route_display_browser_session_profile_provider_match"
            .to_string(),
        tab_policy: "open_new".to_string(),
        requested_profile: requested_profile_hint(intent).map(str::to_string),
        requested_browser_build: intent.browser_build.clone(),
        requested_browser_host: intent.browser_host.clone(),
        requested_view_stream_provider: provider,
        requested_control_input: intent.control_input.clone(),
        requested_display_isolation: intent.display_isolation.clone(),
        requested_route_pool_entry_id: route_pool_entry_id,
        requested_route_id,
        selected_route_pool_entry_id,
        selected_route_id,
        display_allocation_id,
        display_name: route_binding.launch_display_name.clone(),
        route_binding,
        decisions,
        blockers,
        proof_required,
        cleanup_on_failure,
        suggested_commands,
    })
}

pub fn display_allocation_id_for_route_pool_entry(entry: &RoutePoolEntry) -> String {
    route_pool_target_string(entry, "displayAllocationId").unwrap_or_else(|| {
        let seed = route_pool_target_string(entry, "displayName")
            .or_else(|| {
                if entry.route_id.trim().is_empty() {
                    None
                } else {
                    Some(entry.route_id.clone())
                }
            })
            .unwrap_or_else(|| entry.id.clone());
        format!("remote-view-display:{}", route_binding_id_component(&seed))
    })
}

fn route_binding_id_component(value: &str) -> String {
    let mut output = String::new();
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
        } else if !output.ends_with('-') {
            output.push('-');
        }
    }
    let output = output.trim_matches('-').to_string();
    if output.is_empty() {
        "route".to_string()
    } else {
        output
    }
}

pub fn route_pool_entry_matches_display(
    entry: &RoutePoolEntry,
    display_allocation_id: &str,
    allocation: Option<&DisplayAllocation>,
) -> bool {
    if let Some(target_allocation_id) = route_pool_target_string(entry, "displayAllocationId") {
        return target_allocation_id == display_allocation_id;
    }
    if let Some(target_browser_id) = route_pool_target_string(entry, "browserId") {
        return allocation.and_then(|allocation| allocation.owner_browser_id.as_deref())
            == Some(target_browser_id.as_str());
    }
    if let Some(target_session_id) = route_pool_target_string(entry, "sessionId") {
        return allocation.and_then(|allocation| allocation.owner_session_id.as_deref())
            == Some(target_session_id.as_str());
    }
    if let Some(target_display_name) = route_pool_target_string(entry, "displayName") {
        let Some(allocation) = allocation else {
            return true;
        };
        return allocation.display_name.as_deref() == Some(target_display_name.as_str());
    }
    true
}

pub fn ensure_route_pool_entry_matches_display(
    entry: &RoutePoolEntry,
    display_allocation_id: &str,
    allocation: Option<&DisplayAllocation>,
) -> Result<(), String> {
    if route_pool_entry_matches_display(entry, display_allocation_id, allocation) {
        return Ok(());
    }
    Err(format!(
        "route_pool_target_mismatch: route pool entry '{}' does not target display allocation '{}'",
        entry.id, display_allocation_id
    ))
}

pub fn readiness_state(readiness: &Value) -> Option<String> {
    match readiness {
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Array(items) => items
            .iter()
            .filter_map(readiness_state)
            .find(|state| state != "ready"),
        Value::Object(record) => {
            let mut scalar_state = None;
            for key in ["state", "status", "readiness", "lastProviderEvent"] {
                if let Some(value) = record.get(key).and_then(Value::as_str) {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        scalar_state = Some(trimmed.to_string());
                        if trimmed != "ready" {
                            return scalar_state;
                        }
                        break;
                    }
                }
            }
            for key in ["components", "checks", "results"] {
                if let Some(Value::Array(items)) = record.get(key) {
                    if let Some(state) = items
                        .iter()
                        .filter_map(readiness_state)
                        .find(|state| state != "ready")
                    {
                        return Some(state);
                    }
                }
            }
            scalar_state
        }
        _ => None,
    }
}

pub fn ensure_route_pool_entry_ready_for_checkout(entry: &RoutePoolEntry) -> Result<(), String> {
    let Some(readiness) = entry.readiness.as_ref() else {
        return Ok(());
    };
    if readiness
        .get("state")
        .and_then(Value::as_str)
        .is_some_and(|state| state.trim() == "ready")
    {
        return Ok(());
    }
    let Some(state) = readiness_state(readiness) else {
        return Ok(());
    };
    if state == "ready" {
        return Ok(());
    }
    if entry.state == "pending" && state == "pending" {
        return Ok(());
    }
    if entry.state == "available"
        && entry.current_route_allocation_id.is_none()
        && state == "pending"
        && readiness
            .get("component")
            .and_then(Value::as_str)
            .is_some_and(|component| component == "remote_view_open_acquisition")
    {
        return Ok(());
    }
    Err(format!(
        "route_pool_not_ready: route pool entry '{}' readiness is '{}'",
        entry.id, state
    ))
}

pub fn build_route_binding(
    entry: Option<&RoutePoolEntry>,
    display_allocation_id: &str,
    allocation: Option<&DisplayAllocation>,
    requested_route_id: Option<&str>,
    provider: ViewStreamProvider,
) -> Result<RemoteViewRouteBinding, String> {
    if let Some(entry) = entry {
        ensure_route_pool_entry_matches_display(entry, display_allocation_id, allocation)?;
        ensure_route_pool_entry_ready_for_checkout(entry)?;
    }

    let provider = entry.map(|entry| entry.provider).unwrap_or(provider);
    let route_id = requested_route_id
        .map(str::to_string)
        .or_else(|| entry.map(|entry| entry.route_id.clone()))
        .unwrap_or_else(|| format!("remote-view-route:{display_allocation_id}"));
    let display_name = entry
        .and_then(|entry| route_pool_target_string(entry, "displayName"))
        .or_else(|| allocation.and_then(|allocation| allocation.display_name.clone()));
    let route_entry_display_isolation =
        entry.and_then(|entry| route_pool_target_string(entry, "displayIsolation"));
    let display_isolation = route_entry_display_isolation
        .or_else(|| {
            if provider == ViewStreamProvider::RdpGateway
                && entry.is_some()
                && display_name.is_some()
            {
                Some("shared_display".to_string())
            } else {
                allocation.map(|allocation| allocation.display_isolation.clone())
            }
        })
        .unwrap_or_else(|| {
            if provider == ViewStreamProvider::RdpGateway && entry.is_some() {
                "shared_display".to_string()
            } else {
                "private_virtual_display".to_string()
            }
        });
    let route_descriptor = entry.and_then(|entry| entry.route_descriptor.clone());
    let frame_url = entry.and_then(|entry| entry.frame_url.clone());
    let external_url = entry.and_then(|entry| entry.external_url.clone());

    if let Some(entry) = entry.filter(|_| provider == ViewStreamProvider::RdpGateway) {
        if display_name.is_none() {
            return Err(format!(
                "route_display_missing: route pool entry '{}' has no target.displayName for display allocation '{}'",
                entry.id,
                display_allocation_id
            ));
        }
        if let Some(display_name) = display_name.as_deref() {
            if !route_display_socket_available(display_name) {
                return Err(format!(
                    "route_display_unavailable: route pool entry '{}' target display '{}' has no local filesystem or abstract X11 socket",
                    entry.id, display_name
                ));
            }
        }
        if !has_concrete_guacamole_route(frame_url.as_deref())
            && !has_concrete_guacamole_route(external_url.as_deref())
            && !route_descriptor_has_concrete_route(route_descriptor.as_ref())
        {
            return Err(format!(
                "dashboard_embed_not_routable: route pool entry '{}' has no concrete Guacamole #/client route URL",
                entry.id
            ));
        }
    }

    Ok(RemoteViewRouteBinding {
        route_id,
        route_pool_entry_id: entry.map(|entry| entry.id.clone()),
        display_allocation_id: display_allocation_id.to_string(),
        route_pool_entry_state: entry.map(|entry| entry.state.clone()),
        current_route_allocation_id: entry
            .and_then(|entry| entry.current_route_allocation_id.clone()),
        display_name: display_name.clone(),
        launch_display_name: display_name,
        display_isolation,
        route_user: entry.and_then(|entry| {
            route_pool_target_string(entry, "routeUser")
                .or_else(|| route_pool_target_string(entry, "username"))
                .or_else(|| route_pool_target_string(entry, "user"))
        }),
        display_access: entry.and_then(|entry| {
            entry
                .target
                .get("displayAccess")
                .cloned()
                .or_else(|| entry.target.get("x11Access").cloned())
        }),
        provider,
        provider_mode: entry
            .map(|entry| entry.provider_mode.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        connection_id: entry.and_then(|entry| entry.connection_id.clone()),
        connection_name: entry.and_then(|entry| entry.connection_name.clone()),
        frame_url,
        external_url,
        route_descriptor,
        readiness: entry.and_then(|entry| entry.readiness.clone()),
    })
}

pub fn route_binding_readiness(binding: &RemoteViewRouteBinding) -> Value {
    json!({
        "state": "ready",
        "component": "remote_view_route_binding",
        "routeId": binding.route_id,
        "routePoolEntryId": binding.route_pool_entry_id,
        "displayAllocationId": binding.display_allocation_id,
        "routePoolEntryState": binding.route_pool_entry_state,
        "currentRouteAllocationId": binding.current_route_allocation_id,
        "displayName": binding.display_name,
        "launchDisplayName": binding.launch_display_name,
        "displayIsolation": binding.display_isolation,
        "provider": binding.provider,
        "providerMode": binding.provider_mode,
        "connectionId": binding.connection_id,
        "connectionName": binding.connection_name,
        "routeUser": binding.route_user,
        "displayAccess": binding.display_access,
        "routeDescriptor": binding.route_descriptor,
    })
}

pub fn route_display_content(display_name: &str) -> Option<Value> {
    let display_name = display_name.trim();
    if display_name.is_empty() {
        return None;
    }
    if !should_probe_route_display(display_name) {
        return Some(json!({
            "state": "display_probe_unavailable",
            "displayName": display_name,
            "windows": [],
            "error": "display is not a configured RDP route display",
        }));
    }

    let cache = ROUTE_DISPLAY_CONTENT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut cache) = cache.lock() {
        if let Some(entry) = cache.get(display_name) {
            if entry.observed_at.elapsed() < ROUTE_DISPLAY_CONTENT_TTL {
                return Some(entry.content.clone());
            }
        }

        let content = inspect_route_display_content(display_name);
        cache.insert(
            display_name.to_string(),
            RouteDisplayContentCacheEntry {
                observed_at: Instant::now(),
                content: content.clone(),
            },
        );
        return Some(content);
    }
    Some(inspect_route_display_content(display_name))
}

pub fn should_probe_route_display(display_name: &str) -> bool {
    route_display_names().contains(display_name)
}

fn route_display_names() -> HashSet<String> {
    let cache = ROUTE_DISPLAY_NAME_CACHE.get_or_init(|| {
        Mutex::new(RouteDisplayNameCacheEntry {
            observed_at: Instant::now() - ROUTE_DISPLAY_NAME_TTL,
            names: HashSet::new(),
        })
    });
    if let Ok(mut cache) = cache.lock() {
        if cache.observed_at.elapsed() >= ROUTE_DISPLAY_NAME_TTL {
            cache.names = inspect_route_display_names();
            cache.observed_at = Instant::now();
        }
        let mut names = cache.names.clone();
        names.extend(env_route_display_names());
        return names;
    }
    let mut names = inspect_route_display_names();
    names.extend(env_route_display_names());
    names
}

fn env_route_display_names() -> HashSet<String> {
    let mut names = HashSet::new();
    for key in [
        "AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME",
        "AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME",
        "AGENT_BROWSER_REMOTE_HEADED_DISPLAY",
    ] {
        if let Ok(value) = env::var(key) {
            let value = value.trim();
            if is_x11_display_name(value) {
                names.insert(value.to_string());
            }
        }
    }
    names
}

fn inspect_route_display_names() -> HashSet<String> {
    let mut names = HashSet::new();
    let route_users = [
        env::var("AGENT_BROWSER_RDP_ROUTE_A_USERNAME")
            .unwrap_or_else(|_| "agent-browser-rdp-a".to_string()),
        env::var("AGENT_BROWSER_RDP_ROUTE_B_USERNAME")
            .unwrap_or_else(|_| "agent-browser-rdp-b".to_string()),
        env::var("AGENT_BROWSER_RDP_EXISTING_USERNAME")
            .or_else(|_| env::var("XRDP_AGENT_BROWSER_USERNAME"))
            .unwrap_or_else(|_| "agent-browser-rdp".to_string()),
    ];

    if let Ok(output) = Command::new("ps")
        .args(["-eo", "user:64=,comm=,args="])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                let mut parts = trimmed.split_whitespace();
                let Some(user) = parts.next() else {
                    continue;
                };
                if !route_users.iter().any(|route_user| route_user == user) {
                    continue;
                }
                let Some(command) = parts.next() else {
                    continue;
                };
                let args = parts.collect::<Vec<_>>().join(" ");
                if !matches!(command, "Xorg" | "Xvnc" | "Xvfb")
                    && !args.contains("Xorg")
                    && !args.contains("Xvnc")
                    && !args.contains("Xvfb")
                {
                    continue;
                }
                if let Some(display_name) = x11_display_name_from_args(&args) {
                    names.insert(display_name);
                }
            }
        }
    }

    names
}

fn is_x11_display_name(value: &str) -> bool {
    if !value.starts_with(':') {
        return false;
    }
    value[1..]
        .split('.')
        .next()
        .is_some_and(|number| number.parse::<u16>().is_ok())
}

fn x11_display_name_from_args(args: &str) -> Option<String> {
    args.split_whitespace()
        .find(|part| is_x11_display_name(part))
        .map(str::to_string)
}

fn inspect_route_display_content(display_name: &str) -> Value {
    let output = Command::new("timeout")
        .args([
            "--kill-after=1",
            "2",
            "xwininfo",
            "-display",
            display_name,
            "-root",
            "-tree",
        ])
        .output();
    let Ok(output) = output else {
        return json!({
            "state": "display_probe_unavailable",
            "displayName": display_name,
            "windows": [],
            "error": "xwininfo probe could not be started",
        });
    };
    if !output.status.success() {
        let error = String::from_utf8_lossy(if output.stderr.is_empty() {
            &output.stdout
        } else {
            &output.stderr
        })
        .trim()
        .chars()
        .take(240)
        .collect::<String>();
        return json!({
            "state": "display_probe_unavailable",
            "displayName": display_name,
            "windows": [],
            "error": if error.is_empty() { "xwininfo probe failed" } else { error.as_str() },
        });
    }
    display_content_from_xwininfo(display_name, &String::from_utf8_lossy(&output.stdout))
}

pub fn display_content_from_xwininfo(display_name: &str, text: &str) -> Value {
    let windows = xwininfo_windows(text);
    let top_window = windows
        .iter()
        .find(|window| !route_display_window_is_window_manager(window))
        .cloned();
    let joined = windows
        .iter()
        .filter_map(|window| window.as_object())
        .flat_map(|window| {
            [
                window.get("title").and_then(Value::as_str).unwrap_or(""),
                window
                    .get("className")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            ]
        })
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    let browser_visible = ["chromium", "google chrome", "chrome browser", "firefox"]
        .iter()
        .any(|needle| joined.contains(needle));
    let terminal_visible = ["xterm", "terminal", "shell"]
        .iter()
        .any(|needle| joined.contains(needle));
    let top_window_text = top_window
        .as_ref()
        .and_then(Value::as_object)
        .map(|window| {
            [
                window.get("title").and_then(Value::as_str).unwrap_or(""),
                window
                    .get("className")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            ]
            .join("\n")
            .to_lowercase()
        })
        .unwrap_or_default();
    let terminal_topmost = browser_visible
        && ["xterm", "terminal", "shell"]
            .iter()
            .any(|needle| top_window_text.contains(needle));
    let state = if terminal_topmost {
        "terminal_topmost"
    } else if browser_visible {
        "browser_window_visible"
    } else if terminal_visible {
        "terminal_only"
    } else if windows.is_empty() {
        "empty_display"
    } else {
        "non_browser_windows"
    };
    json!({
        "state": state,
        "displayName": display_name,
        "windowCount": windows.len(),
        "topWindow": top_window,
        "windows": windows,
    })
}

fn route_display_window_is_window_manager(window: &Value) -> bool {
    let Some(window) = window.as_object() else {
        return false;
    };
    let text = [
        window.get("title").and_then(Value::as_str).unwrap_or(""),
        window
            .get("className")
            .and_then(Value::as_str)
            .unwrap_or(""),
    ]
    .join("\n")
    .to_lowercase();
    text.contains("openbox")
}

pub fn visible_browser_window_proof(
    route_id: &str,
    display_name: &str,
    display_content: Value,
) -> Result<Value, String> {
    let state = display_content
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if state == "browser_window_visible" {
        return Ok(json!({
            "state": "ready",
            "displayName": display_name,
            "displayContent": display_content,
        }));
    }
    let code = if state == "terminal_only" {
        "terminal_only_route"
    } else if state == "terminal_topmost" {
        "terminal_topmost_route"
    } else {
        "browser_window_not_visible"
    };
    Err(format!(
        "{code}: route '{}' display '{}' state is '{}'",
        route_id, display_name, state
    ))
}

fn xwininfo_windows(text: &str) -> Vec<Value> {
    text.lines()
        .filter_map(|line| xwininfo_window(line.trim()))
        .collect()
}

fn xwininfo_window(line: &str) -> Option<Value> {
    if !line.starts_with("0x") {
        return None;
    }
    let title_start = line.find('"')?;
    let rest = &line[title_start + 1..];
    let title_end = rest.find('"')?;
    let title = rest[..title_end].chars().take(120).collect::<String>();
    let id = line[..title_start].split_whitespace().next()?.to_string();
    let class_name = line[title_start + title_end + 2..]
        .split_once('(')
        .and_then(|(_, after)| after.split_once(')'))
        .map(|(inside, _)| inside)
        .and_then(|inside| inside.rsplit('"').nth(1))
        .map(|value| value.chars().take(80).collect::<String>());
    Some(json!({
        "id": id,
        "title": title,
        "className": class_name,
    }))
}

fn has_concrete_guacamole_route(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|value| value.contains("#/client/"))
}

fn route_descriptor_has_concrete_route(route_descriptor: Option<&Value>) -> bool {
    let Some(Value::Object(record)) = route_descriptor else {
        return false;
    };
    [
        "localEmbedUrl",
        "dashboardEmbedUrl",
        "publicOperatorUrl",
        "externalUrl",
        "healthUrl",
    ]
    .iter()
    .any(|key| has_concrete_guacamole_route(record.get(*key).and_then(Value::as_str)))
}

pub fn resolve_route_pool_entry_id(
    state: &ServiceState,
    requested_pool_entry_id: Option<&str>,
    requested_route_id: Option<&str>,
    display_allocation_id: &str,
    allocation: Option<&DisplayAllocation>,
    provider: ViewStreamProvider,
) -> Result<Option<String>, String> {
    if let Some(id) = requested_pool_entry_id {
        return Ok(Some(id.to_string()));
    }
    if state.route_pool.is_empty() {
        return Ok(None);
    }

    let matching_entries = state
        .route_pool
        .values()
        .filter(|entry| entry.provider == provider)
        .filter(|entry| {
            requested_route_id
                .map(|route_id| entry.route_id == route_id)
                .unwrap_or(true)
        })
        .filter(|entry| route_pool_entry_matches_display(entry, display_allocation_id, allocation))
        .collect::<Vec<_>>();

    if let Some(entry) = matching_entries
        .iter()
        .find(|entry| entry.state == "available")
    {
        return Ok(Some(entry.id.clone()));
    }

    if requested_route_id.is_some()
        || allocation
            .is_some_and(|allocation| allocation.display_isolation == "private_virtual_display")
    {
        let diagnostic = route_pool_request_diagnostic(
            state,
            requested_pool_entry_id,
            requested_route_id,
            display_allocation_id,
            allocation,
            provider,
        );
        return Err(format!(
            "route_pool_unavailable: no available route pool entry for display allocation '{}'; diagnostic={}",
            display_allocation_id,
            compact_json(&diagnostic)
        ));
    }
    Ok(None)
}

pub fn route_pool_request_diagnostic(
    state: &ServiceState,
    requested_pool_entry_id: Option<&str>,
    requested_route_id: Option<&str>,
    display_allocation_id: &str,
    allocation: Option<&DisplayAllocation>,
    provider: ViewStreamProvider,
) -> Value {
    let available_route_pool_entries = state
        .route_pool
        .values()
        .filter(|entry| entry.provider == provider)
        .filter(|entry| entry.state == "available")
        .map(route_pool_entry_diagnostic)
        .collect::<Vec<_>>();
    let matching_route_pool_entries = state
        .route_pool
        .values()
        .filter(|entry| entry.provider == provider)
        .filter(|entry| {
            requested_pool_entry_id
                .map(|id| entry.id == id)
                .unwrap_or(true)
        })
        .filter(|entry| {
            requested_route_id
                .map(|route_id| entry.route_id == route_id)
                .unwrap_or(true)
        })
        .filter(|entry| route_pool_entry_matches_display(entry, display_allocation_id, allocation))
        .map(route_pool_entry_diagnostic)
        .collect::<Vec<_>>();
    let recommended_commands = route_pool_recommended_commands(state, provider);

    json!({
        "requested": {
            "routePoolEntryId": requested_pool_entry_id,
            "routeId": requested_route_id,
            "displayAllocationId": display_allocation_id,
            "displayName": allocation.and_then(|allocation| allocation.display_name.as_deref()),
            "displayIsolation": allocation.map(|allocation| allocation.display_isolation.as_str()),
            "ownerBrowserId": allocation.and_then(|allocation| allocation.owner_browser_id.as_deref()),
            "ownerSessionId": allocation.and_then(|allocation| allocation.owner_session_id.as_deref()),
            "profileId": allocation.and_then(|allocation| allocation.profile_id.as_deref()),
            "provider": provider,
        },
        "matchingRoutePoolEntries": matching_route_pool_entries,
        "availableRoutePoolEntries": available_route_pool_entries,
        "availableDisplayAllocationIds": state.display_allocations.values()
            .filter(|allocation| allocation.state == "ready")
            .map(|allocation| allocation.id.clone())
            .collect::<Vec<_>>(),
        "recommendedCommands": recommended_commands,
        "routePoolEntries": state.route_pool.values().take(16).map(route_pool_entry_diagnostic).collect::<Vec<_>>(),
        "displayAllocations": state.display_allocations.values().take(16).map(display_allocation_diagnostic).collect::<Vec<_>>(),
        "remoteViewRoutes": state.remote_view_routes.values().take(16).map(|route| {
            json!({
                "id": route.id,
                "provider": route.provider,
                "displayAllocationId": route.display_allocation_id,
                "browserId": route.browser_id,
                "sessionId": route.session_id,
                "providerMode": route.provider_mode,
                "state": route.state,
                "lastProviderEvent": route.last_provider_event,
                "readinessState": route.readiness.as_ref().and_then(readiness_state),
            })
        }).collect::<Vec<_>>(),
    })
}

fn route_pool_recommended_commands(
    state: &ServiceState,
    provider: ViewStreamProvider,
) -> Vec<String> {
    let mut commands = state
        .route_pool
        .values()
        .filter(|entry| entry.provider == provider)
        .filter(|entry| entry.state == "available")
        .take(3)
        .map(|entry| {
            format!(
                "agent-browser --json remote-view open <url> --provider {} --route-pool-entry-id {} --dry-run",
                provider_name(provider),
                entry.id
            )
        })
        .collect::<Vec<_>>();
    if commands.is_empty() {
        commands.push("agent-browser service route-pool repair --dry-run".to_string());
        commands.push("agent-browser doctor remote-view --json".to_string());
    }
    commands
}

fn provider_name(provider: ViewStreamProvider) -> &'static str {
    match provider {
        ViewStreamProvider::CdpScreencast => "cdp_screencast",
        ViewStreamProvider::ChromeTabWebrtc => "chrome_tab_webrtc",
        ViewStreamProvider::VirtualDisplayWebrtc => "virtual_display_webrtc",
        ViewStreamProvider::Novnc => "novnc",
        ViewStreamProvider::RdpGateway => "rdp_gateway",
        ViewStreamProvider::ExternalUrl => "external_url",
    }
}

pub fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

fn route_pool_entry_diagnostic(entry: &RoutePoolEntry) -> Value {
    json!({
        "id": entry.id,
        "provider": entry.provider,
        "routeId": entry.route_id,
        "connectionId": entry.connection_id,
        "connectionName": entry.connection_name,
        "targetDisplayAllocationId": route_pool_target_string(entry, "displayAllocationId"),
        "targetDisplayName": route_pool_target_string(entry, "displayName"),
        "targetBrowserId": route_pool_target_string(entry, "browserId"),
        "targetSessionId": route_pool_target_string(entry, "sessionId"),
        "providerMode": entry.provider_mode,
        "state": entry.state,
        "currentRouteAllocationId": entry.current_route_allocation_id,
        "readinessState": entry.readiness.as_ref().and_then(readiness_state),
    })
}

fn display_allocation_diagnostic(allocation: &DisplayAllocation) -> Value {
    json!({
        "id": allocation.id,
        "displayName": allocation.display_name,
        "displayIsolation": allocation.display_isolation,
        "ownerBrowserId": allocation.owner_browser_id,
        "ownerSessionId": allocation.owner_session_id,
        "profileId": allocation.profile_id,
        "browserBuild": allocation.browser_build,
        "host": allocation.host,
        "state": allocation.state,
        "routeIds": allocation.route_ids,
        "readinessState": allocation.readiness.as_ref().and_then(readiness_state),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::collections::BTreeMap;

    use super::super::service_model::{BrowserProcess, RemoteViewRoute};
    use super::*;

    #[test]
    fn readiness_state_fails_closed_on_nested_component() {
        let readiness = json!({
            "state": "ready",
            "components": [
                { "component": "guacamole_web_route", "status": "ready" },
                { "component": "route_display", "status": "terminal_only_route" }
            ]
        });

        assert_eq!(
            readiness_state(&readiness),
            Some("terminal_only_route".to_string())
        );
    }

    #[test]
    fn route_pool_entry_matches_display_name_binding() {
        let entry = RoutePoolEntry {
            target: json!({ "displayName": ":10" }),
            ..RoutePoolEntry::default()
        };
        let allocation = DisplayAllocation {
            id: "display-a".to_string(),
            display_name: Some(":10".to_string()),
            ..DisplayAllocation::default()
        };

        assert!(route_pool_entry_matches_display(
            &entry,
            "display-a",
            Some(&allocation)
        ));
    }

    #[test]
    fn profile_id_matches_request_accepts_exact_profile_id() {
        assert!(profile_id_matches_request(
            Some("last30days-facebook"),
            "last30days-facebook",
            Some("last30days-facebook"),
            None,
        ));
    }

    #[test]
    fn profile_id_matches_request_rejects_custom_profile_without_runtime_pid_match() {
        assert!(!profile_id_matches_request(
            Some("custom:stale"),
            "last30days-facebook",
            Some("last30days-facebook"),
            Some(12345),
        ));
    }

    #[test]
    fn display_allocation_id_for_route_pool_entry_prefers_configured_target() {
        let configured = RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            target: json!({
                "displayAllocationId": "display-configured",
                "displayName": ":21"
            }),
            ..RoutePoolEntry::default()
        };
        let derived = RoutePoolEntry {
            id: "pool-b".to_string(),
            route_id: "route-b".to_string(),
            target: json!({ "displayName": ":21" }),
            ..RoutePoolEntry::default()
        };

        assert_eq!(
            display_allocation_id_for_route_pool_entry(&configured),
            "display-configured"
        );
        assert_eq!(
            display_allocation_id_for_route_pool_entry(&derived),
            "remote-view-display:21"
        );
    }

    #[test]
    fn normalize_remote_view_open_intent_accepts_params_view_stream_provider() {
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "runtimeProfile": "last30days-facebook",
            "browserBuild": "stealthcdp_chromium",
            "params": {
                "url": "https://www.facebook.com/",
                "viewStreamProvider": "rdp_gateway",
                "routePoolEntryId": "pool-a",
                "dryRun": true
            }
        }))
        .unwrap();

        assert_eq!(intent.url.as_deref(), Some("https://www.facebook.com/"));
        assert_eq!(
            intent.runtime_profile.as_deref(),
            Some("last30days-facebook")
        );
        assert_eq!(intent.browser_build.as_deref(), Some("stealthcdp_chromium"));
        assert_eq!(intent.view_stream_provider, ViewStreamProvider::RdpGateway);
        assert_eq!(intent.route_pool_entry_id.as_deref(), Some("pool-a"));
        assert!(intent.dry_run);
    }

    #[test]
    fn normalize_remote_view_open_intent_rejects_provider_conflict() {
        let err = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "provider": "rdp_gateway",
            "viewStreamProvider": "external_url"
        }))
        .unwrap_err();

        assert!(err.contains("provider_ambiguity"));
        assert!(err.contains("conflicts with viewStreamProvider 'external_url'"));
    }

    #[test]
    fn normalize_remote_view_open_intent_rejects_cloud_provider_field() {
        let err = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "provider": "browserbase"
        }))
        .unwrap_err();

        assert!(err.contains("provider_ambiguity"));
        assert!(err.contains("browserbase"));
    }

    #[test]
    fn acquisition_plan_prefers_available_route_over_stale_browser_display() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-clean".to_string(),
                RoutePoolEntry {
                    id: "pool-clean".to_string(),
                    route_id: "route-clean".to_string(),
                    frame_url: Some("https://guac.example/#/client/clean".to_string()),
                    target: json!({
                        "displayName": ":31",
                        "displayIsolation": "shared_display"
                    }),
                    provider_mode: "simultaneous_view".to_string(),
                    state: "available".to_string(),
                    readiness: Some(json!({ "state": "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "display-stale".to_string(),
                DisplayAllocation {
                    id: "display-stale".to_string(),
                    display_name: Some(":10".to_string()),
                    display_isolation: "shared_display".to_string(),
                    state: "ready".to_string(),
                    readiness: Some(json!({ "state": "released" })),
                    ..DisplayAllocation::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:default".to_string(),
                BrowserProcess {
                    id: "session:default".to_string(),
                    host: BrowserHost::RemoteHeaded,
                    display_allocation_id: Some("display-stale".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "runtimeProfile": "stealthcdp-default",
            "url": "https://www.linkedin.com/",
            "dryRun": true
        }))
        .unwrap();

        let plan =
            plan_remote_view_acquisition(&state, &intent, None, "session:default", "default")
                .unwrap();

        assert_eq!(
            plan.selected_route_pool_entry_id.as_deref(),
            Some("pool-clean")
        );
        assert_eq!(plan.selected_route_id, "route-clean");
        assert_eq!(plan.display_allocation_id, "remote-view-display:31");
        assert_eq!(plan.display_name.as_deref(), Some(":31"));
        assert!(plan.decisions.iter().any(|decision| {
            decision.step == "display_allocation" && decision.reason == "available_route_pool_entry"
        }));
    }

    #[test]
    fn acquisition_plan_reuses_checked_out_same_owner_route() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!({
                        "displayName": ":10",
                        "displayIsolation": "shared_display"
                    }),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    readiness: Some(json!({ "state": "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    display_allocation_id: Some("remote-view-display:10".to_string()),
                    browser_id: Some("session:default".to_string()),
                    session_id: Some("default".to_string()),
                    state: "ready".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:10".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:10".to_string(),
                    display_name: Some(":10".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:default".to_string()),
                    owner_session_id: Some("default".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:default".to_string(),
                BrowserProcess {
                    id: "session:default".to_string(),
                    host: BrowserHost::RemoteHeaded,
                    display_allocation_id: Some("remote-view-display:10".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "routePoolEntryId": "pool-a",
            "dryRun": true
        }))
        .unwrap();

        let plan =
            plan_remote_view_acquisition(&state, &intent, None, "session:default", "default")
                .unwrap();

        assert_eq!(plan.selected_route_pool_entry_id.as_deref(), Some("pool-a"));
        assert_eq!(plan.display_allocation_id, "remote-view-display:10");
        assert!(plan.decisions.iter().any(|decision| {
            decision.step == "route_pool_entry" && decision.reason == "same_owner_checked_out_route"
        }));
    }

    #[test]
    fn acquisition_plan_reuses_same_owner_route_when_pool_row_was_rolled_back_available() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!({
                        "displayName": ":10",
                        "displayIsolation": "shared_display"
                    }),
                    state: "available".to_string(),
                    current_route_allocation_id: None,
                    readiness: Some(json!({ "state": "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    display_allocation_id: Some("remote-view-display:10".to_string()),
                    browser_id: Some("session:default".to_string()),
                    session_id: Some("default".to_string()),
                    state: "ready".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:10".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:10".to_string(),
                    display_name: Some(":10".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:default".to_string()),
                    owner_session_id: Some("default".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:default".to_string(),
                BrowserProcess {
                    id: "session:default".to_string(),
                    host: BrowserHost::RemoteHeaded,
                    display_allocation_id: Some("remote-view-display:10".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "routePoolEntryId": "pool-a",
            "dryRun": true
        }))
        .unwrap();

        let plan =
            plan_remote_view_acquisition(&state, &intent, None, "session:default", "default")
                .unwrap();

        assert_eq!(plan.selected_route_pool_entry_id.as_deref(), Some("pool-a"));
        assert_eq!(plan.display_allocation_id, "remote-view-display:10");
        assert!(plan.decisions.iter().any(|decision| {
            decision.step == "route_pool_entry" && decision.reason == "same_owner_checked_out_route"
        }));
    }

    #[test]
    fn acquisition_plan_rejects_checked_out_route_for_other_owner() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!({
                        "displayName": ":10",
                        "displayIsolation": "shared_display"
                    }),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    readiness: Some(json!({ "state": "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    display_allocation_id: Some("remote-view-display:10".to_string()),
                    browser_id: Some("session:other".to_string()),
                    session_id: Some("other".to_string()),
                    state: "ready".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:10".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:10".to_string(),
                    display_name: Some(":10".to_string()),
                    display_isolation: "shared_display".to_string(),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "routePoolEntryId": "pool-a",
            "dryRun": true
        }))
        .unwrap();

        let err = plan_remote_view_acquisition(&state, &intent, None, "session:default", "default")
            .unwrap_err();

        assert!(err.contains("route_pool_entry_unavailable"));
        assert!(err.contains("routePoolEntries"));
    }

    #[test]
    fn acquisition_plan_reports_route_pool_exhausted_for_unpinned_checked_out_inline_entry() {
        let inline_entry = RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            frame_url: Some("https://guac.example/#/client/route-a".to_string()),
            target: json!({
                "displayName": ":10",
                "displayIsolation": "shared_display"
            }),
            state: "checked_out".to_string(),
            current_route_allocation_id: Some("route-a".to_string()),
            readiness: Some(json!({ "state": "ready" })),
            ..RoutePoolEntry::default()
        };
        let state = ServiceState {
            route_pool: BTreeMap::from([("pool-a".to_string(), inline_entry.clone())]),
            remote_view_routes: BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    display_allocation_id: Some("remote-view-display:10".to_string()),
                    browser_id: Some("session:other".to_string()),
                    session_id: Some("other".to_string()),
                    state: "ready".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:10".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:10".to_string(),
                    display_name: Some(":10".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:other".to_string()),
                    owner_session_id: Some("other".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "dryRun": true
        }))
        .unwrap();

        let err = plan_remote_view_acquisition(
            &state,
            &intent,
            Some(&inline_entry),
            "session:default",
            "default",
        )
        .unwrap_err();

        assert!(err.contains("route_pool_exhausted"));
        assert!(err.contains("availableRoutePoolEntries"));
    }

    #[test]
    fn acquisition_plan_reports_route_pool_exhausted_before_foreign_browser_display_fallback() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!({
                        "displayName": ":10",
                        "displayIsolation": "shared_display"
                    }),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    readiness: Some(json!({ "state": "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:10".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:10".to_string(),
                    display_name: Some(":10".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:other".to_string()),
                    owner_session_id: Some("other".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "session:default".to_string(),
                BrowserProcess {
                    id: "session:default".to_string(),
                    host: BrowserHost::RemoteHeaded,
                    display_allocation_id: Some("remote-view-display:10".to_string()),
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "dryRun": true
        }))
        .unwrap();

        let err = plan_remote_view_acquisition(&state, &intent, None, "session:default", "default")
            .unwrap_err();

        assert!(err.contains("route_pool_exhausted"));
        assert!(err.contains("availableRoutePoolEntries"));
    }

    #[test]
    fn acquisition_plan_reports_named_session_display_mismatch_with_routes() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-clean".to_string(),
                RoutePoolEntry {
                    id: "pool-clean".to_string(),
                    route_id: "route-clean".to_string(),
                    frame_url: Some("https://guac.example/#/client/clean".to_string()),
                    target: json!({
                        "displayName": ":31",
                        "displayIsolation": "shared_display"
                    }),
                    state: "available".to_string(),
                    readiness: Some(json!({ "state": "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "display-other".to_string(),
                DisplayAllocation {
                    id: "display-other".to_string(),
                    display_name: Some(":10".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:other".to_string()),
                    owner_session_id: Some("other".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "displayAllocationId": "display-other",
            "dryRun": true
        }))
        .unwrap();

        let err = plan_remote_view_acquisition(&state, &intent, None, "session:default", "default")
            .unwrap_err();

        assert!(err.contains("display_allocation_owner_mismatch"));
        assert!(err.contains("availableRoutePoolEntries"));
        assert!(err.contains("pool-clean"));
    }

    #[test]
    fn acquisition_plan_reclaims_released_display_allocation_from_previous_session() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-clean".to_string(),
                RoutePoolEntry {
                    id: "pool-clean".to_string(),
                    route_id: "route-clean".to_string(),
                    frame_url: Some("https://guac.example/#/client/clean".to_string()),
                    target: json!({
                        "displayName": ":31",
                        "displayIsolation": "shared_display"
                    }),
                    state: "available".to_string(),
                    readiness: Some(json!({ "state": "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:31".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:31".to_string(),
                    display_name: Some(":31".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:old".to_string()),
                    owner_session_id: Some("old".to_string()),
                    state: "released".to_string(),
                    readiness: Some(json!({ "state": "released" })),
                    ..DisplayAllocation::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "routePoolEntryId": "pool-clean",
            "dryRun": true
        }))
        .unwrap();

        let plan =
            plan_remote_view_acquisition(&state, &intent, None, "session:new", "new").unwrap();

        assert_eq!(
            plan.selected_route_pool_entry_id.as_deref(),
            Some("pool-clean")
        );
        assert_eq!(plan.selected_route_id, "route-clean");
        assert_eq!(plan.display_allocation_id, "remote-view-display:31");
        assert_eq!(plan.display_name.as_deref(), Some(":31"));
    }

    #[test]
    fn route_pool_target_display_overrides_stale_private_allocation_isolation() {
        let entry = RoutePoolEntry {
            id: "guacamole-rdp-a".to_string(),
            route_id: "guacamole:3".to_string(),
            frame_url: Some("https://guac.example/#/client/route-a".to_string()),
            target: json!({
                "displayName": ":13"
            }),
            provider: ViewStreamProvider::RdpGateway,
            provider_mode: "simultaneous_view".to_string(),
            state: "available".to_string(),
            readiness: Some(json!({ "state": "ready" })),
            ..RoutePoolEntry::default()
        };
        let stale_allocation = DisplayAllocation {
            id: "remote-view-display:13".to_string(),
            display_name: Some(":13".to_string()),
            display_isolation: "private_virtual_display".to_string(),
            state: "released".to_string(),
            readiness: Some(json!({ "state": "released" })),
            ..DisplayAllocation::default()
        };

        let binding = build_route_binding(
            Some(&entry),
            "remote-view-display:13",
            Some(&stale_allocation),
            None,
            ViewStreamProvider::RdpGateway,
        )
        .unwrap();

        assert_eq!(binding.launch_display_name.as_deref(), Some(":13"));
        assert_eq!(binding.display_isolation, "shared_display");
    }

    #[test]
    fn acquisition_plan_reuses_same_owner_pending_route_reservation() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!({
                        "displayName": ":41",
                        "displayIsolation": "shared_display"
                    }),
                    state: "pending".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    readiness: Some(json!({ "state": "pending" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:41".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:41".to_string(),
                    display_name: Some(":41".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:current".to_string()),
                    owner_session_id: Some("current".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    display_allocation_id: Some("remote-view-display:41".to_string()),
                    browser_id: Some("session:current".to_string()),
                    session_id: Some("current".to_string()),
                    state: "pending".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "routeId": "route-a",
            "displayAllocationId": "remote-view-display:41",
            "dryRun": true
        }))
        .unwrap();

        let plan =
            plan_remote_view_acquisition(&state, &intent, None, "session:current", "current")
                .unwrap();

        assert_eq!(plan.selected_route_pool_entry_id.as_deref(), Some("pool-a"));
        assert_eq!(plan.selected_route_id, "route-a");
        assert_eq!(plan.display_allocation_id, "remote-view-display:41");
    }

    #[test]
    fn acquisition_plan_reuses_same_owner_checked_out_route() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!({
                        "displayName": ":41",
                        "displayIsolation": "shared_display"
                    }),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    readiness: Some(json!({ "state": "ready" })),
                    ..RoutePoolEntry::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:41".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:41".to_string(),
                    display_name: Some(":41".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:current".to_string()),
                    owner_session_id: Some("current".to_string()),
                    state: "ready".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    display_allocation_id: Some("remote-view-display:41".to_string()),
                    browser_id: Some("session:current".to_string()),
                    session_id: Some("current".to_string()),
                    state: "checked_out".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "routePoolEntryId": "pool-a",
            "routeId": "route-a",
            "displayAllocationId": "remote-view-display:41",
            "dryRun": true
        }))
        .unwrap();

        let plan =
            plan_remote_view_acquisition(&state, &intent, None, "session:current", "current")
                .unwrap();

        assert_eq!(plan.selected_route_pool_entry_id.as_deref(), Some("pool-a"));
        assert_eq!(plan.selected_route_id, "route-a");
        assert_eq!(plan.display_allocation_id, "remote-view-display:41");
        assert!(plan.decisions.iter().any(|decision| {
            decision.step == "route_pool_entry" && decision.reason == "same_owner_checked_out_route"
        }));
    }

    #[test]
    fn acquisition_plan_reuses_same_owner_orphaned_route_for_repeat_open() {
        let state = ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!({
                        "displayName": ":41",
                        "displayIsolation": "shared_display"
                    }),
                    state: "checked_out".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    readiness: Some(json!({
                        "component": "remote_view_open_visible_window",
                        "state": "ready"
                    })),
                    ..RoutePoolEntry::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "remote-view-display:41".to_string(),
                DisplayAllocation {
                    id: "remote-view-display:41".to_string(),
                    display_name: Some(":41".to_string()),
                    display_isolation: "shared_display".to_string(),
                    owner_browser_id: Some("session:current".to_string()),
                    owner_session_id: Some("current".to_string()),
                    state: "pending".to_string(),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                "route-a".to_string(),
                RemoteViewRoute {
                    id: "route-a".to_string(),
                    display_allocation_id: Some("remote-view-display:41".to_string()),
                    browser_id: Some("session:current".to_string()),
                    session_id: Some("current".to_string()),
                    state: "orphaned".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        };
        let intent = normalize_remote_view_open_intent(&json!({
            "action": "remote_view_open",
            "routePoolEntryId": "pool-a",
            "routeId": "route-a",
            "displayAllocationId": "remote-view-display:41",
            "dryRun": true
        }))
        .unwrap();

        let plan =
            plan_remote_view_acquisition(&state, &intent, None, "session:current", "current")
                .unwrap();

        assert_eq!(plan.selected_route_pool_entry_id.as_deref(), Some("pool-a"));
        assert_eq!(plan.selected_route_id, "route-a");
        assert_eq!(plan.display_allocation_id, "remote-view-display:41");
        assert!(plan.decisions.iter().any(|decision| {
            decision.step == "route_pool_entry" && decision.reason == "same_owner_checked_out_route"
        }));
    }

    #[test]
    fn build_route_binding_requires_rdp_route_display() {
        let entry = RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            frame_url: Some("https://guac.example/#/client/route-a".to_string()),
            state: "available".to_string(),
            ..RoutePoolEntry::default()
        };

        let err = build_route_binding(
            Some(&entry),
            "display-a",
            None,
            None,
            ViewStreamProvider::RdpGateway,
        )
        .unwrap_err();

        assert!(err.contains("route_display_missing"));
    }

    #[test]
    fn build_route_binding_requires_concrete_guacamole_client_url() {
        let entry = RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            frame_url: Some("https://guac.example/guacamole/".to_string()),
            target: json!({ "displayName": ":10" }),
            state: "available".to_string(),
            ..RoutePoolEntry::default()
        };

        let err = build_route_binding(
            Some(&entry),
            "display-a",
            None,
            None,
            ViewStreamProvider::RdpGateway,
        )
        .unwrap_err();

        assert!(err.contains("dashboard_embed_not_routable"));
    }

    #[test]
    fn route_pool_entry_ready_allows_stale_available_acquisition_pending() {
        let entry = RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            state: "available".to_string(),
            current_route_allocation_id: None,
            readiness: Some(json!({
                "state": "pending",
                "component": "remote_view_open_acquisition",
                "leaseId": "remote-view-open:default:route-a:stale"
            })),
            ..RoutePoolEntry::default()
        };

        ensure_route_pool_entry_ready_for_checkout(&entry).unwrap();
    }

    #[test]
    fn route_pool_entry_ready_rejects_available_provider_pending() {
        let entry = RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            state: "available".to_string(),
            current_route_allocation_id: None,
            readiness: Some(json!({
                "state": "pending",
                "component": "rdp_backend"
            })),
            ..RoutePoolEntry::default()
        };

        let err = ensure_route_pool_entry_ready_for_checkout(&entry).unwrap_err();
        assert!(err.contains("route_pool_not_ready"));
    }

    #[test]
    fn build_route_binding_returns_launch_display_and_route_roles() {
        let entry = RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            frame_url: Some("http://127.0.0.1:8092/guacamole/#/client/route-a".to_string()),
            external_url: Some("https://guac.example/#/client/route-a".to_string()),
            route_descriptor: Some(json!({
                "localEmbedUrl": "http://127.0.0.1:8092/guacamole/#/client/route-a",
                "publicOperatorUrl": "https://guac.example/#/client/route-a",
                "dashboardEmbedUrl": "https://guac.example/#/client/route-a",
                "healthUrl": "http://127.0.0.1:8092/guacamole/#/client/route-a",
                "externalUrl": "https://guac.example/#/client/route-a"
            })),
            target: json!({
                "displayName": ":10",
                "displayIsolation": "shared_display",
                "routeUser": "agent-browser-rdp-a",
                "displayAccess": { "state": "ready" }
            }),
            provider_mode: "simultaneous_view".to_string(),
            state: "available".to_string(),
            ..RoutePoolEntry::default()
        };

        let binding = build_route_binding(
            Some(&entry),
            "display-a",
            None,
            None,
            ViewStreamProvider::RdpGateway,
        )
        .unwrap();

        assert_eq!(binding.route_id, "route-a");
        assert_eq!(binding.display_name.as_deref(), Some(":10"));
        assert_eq!(binding.launch_display_name.as_deref(), Some(":10"));
        assert_eq!(binding.display_isolation, "shared_display");
        assert_eq!(binding.route_user.as_deref(), Some("agent-browser-rdp-a"));
        assert_eq!(
            binding
                .route_descriptor
                .as_ref()
                .unwrap()
                .get("publicOperatorUrl")
                .and_then(Value::as_str),
            Some("https://guac.example/#/client/route-a")
        );
    }

    #[test]
    fn display_content_detects_terminal_only_route() {
        let content = display_content_from_xwininfo(
            ":12",
            r#"
        0x60011f "Openbox": ("" (none))  1x1+-100+-100  +-100+-100
        0x40000e "agent-browser-rdp-a@cooper: ~": ("xterm" "XTerm")  604x368+1+22  +41+62
"#,
        );

        assert_eq!(content["state"], "terminal_only");
        assert_eq!(content["displayName"], ":12");
        assert_eq!(content["windowCount"], 2);
        assert_eq!(content["windows"][1]["className"], "XTerm");
        assert!(visible_browser_window_proof("route-a", ":12", content)
            .unwrap_err()
            .contains("terminal_only_route"));
    }

    #[test]
    fn display_content_detects_browser_window() {
        let content = display_content_from_xwininfo(
            ":12",
            r#"
        0x60011f "Openbox": ("" (none))  1x1+-100+-100  +-100+-100
        0x800003 "Example Domain - Chromium": ("chromium-browser (/tmp/profile)" "Chromium-browser")  504x320+0+0  +0+0
        0x40000e "agent-browser-rdp-a@cooper: ~": ("xterm" "XTerm")  604x368+1+22  +41+62
"#,
        );

        assert_eq!(content["state"], "browser_window_visible");
        assert_eq!(content["windowCount"], 3);
        assert_eq!(content["windows"][1]["title"], "Example Domain - Chromium");
        let proof = visible_browser_window_proof("route-a", ":12", content).unwrap();
        assert_eq!(proof["state"], "ready");
        assert_eq!(proof["displayContent"]["state"], "browser_window_visible");
    }

    #[test]
    fn display_content_rejects_terminal_topmost_over_browser() {
        let content = display_content_from_xwininfo(
            ":12",
            r#"
        0x60011f "Openbox": ("" (none))  1x1+-100+-100  +-100+-100
        0x40000e "agent-browser-rdp-a@cooper: ~": ("xterm" "XTerm")  604x368+1+22  +41+62
        0x800003 "Example Domain - Chromium": ("chromium-browser (/tmp/profile)" "Chromium-browser")  504x320+0+0  +0+0
"#,
        );

        assert_eq!(content["state"], "terminal_topmost");
        assert_eq!(content["topWindow"]["className"], "XTerm");
        assert!(visible_browser_window_proof("route-a", ":12", content)
            .unwrap_err()
            .contains("terminal_topmost_route"));
    }
}
