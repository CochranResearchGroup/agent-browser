use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::env;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::service_model::{DisplayAllocation, RoutePoolEntry, ServiceState, ViewStreamProvider};

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

pub fn route_pool_target_string(entry: &RoutePoolEntry, key: &str) -> Option<String> {
    entry
        .target
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
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
    let display_isolation = entry
        .and_then(|entry| route_pool_target_string(entry, "displayIsolation"))
        .or_else(|| allocation.map(|allocation| allocation.display_isolation.clone()))
        .unwrap_or_else(|| {
            if provider == ViewStreamProvider::RdpGateway && entry.is_some() {
                "shared_display".to_string()
            } else {
                "private_virtual_display".to_string()
            }
        });
    let display_name = entry
        .and_then(|entry| route_pool_target_string(entry, "displayName"))
        .or_else(|| allocation.and_then(|allocation| allocation.display_name.clone()));
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
    let state = if browser_visible {
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
        "windows": windows,
    })
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
}
