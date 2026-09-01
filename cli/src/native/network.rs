use serde_json::{json, Value};
use std::collections::HashMap;

use agent_browser_cdp::client::CdpClient;

pub async fn set_extra_headers(
    client: &CdpClient,
    session_id: &str,
    headers: &HashMap<String, String>,
) -> Result<(), String> {
    let headers_value: Value = headers
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect::<serde_json::Map<String, Value>>()
        .into();

    client
        .send_command(
            "Network.setExtraHTTPHeaders",
            Some(json!({ "headers": headers_value })),
            Some(session_id),
        )
        .await?;

    Ok(())
}

pub async fn set_offline(
    client: &CdpClient,
    session_id: &str,
    offline: bool,
) -> Result<(), String> {
    client
        .send_command(
            "Network.emulateNetworkConditions",
            Some(json!({
                "offline": offline,
                "latency": 0,
                "downloadThroughput": -1,
                "uploadThroughput": -1,
            })),
            Some(session_id),
        )
        .await?;
    Ok(())
}

pub async fn set_content(client: &CdpClient, session_id: &str, html: &str) -> Result<(), String> {
    // Get current frame ID
    let tree_result = client
        .send_command_no_params("Page.getFrameTree", Some(session_id))
        .await?;

    let frame_id = tree_result
        .get("frameTree")
        .and_then(|t| t.get("frame"))
        .and_then(|f| f.get("id"))
        .and_then(|id| id.as_str())
        .ok_or("Could not determine frame ID")?;

    client
        .send_command(
            "Page.setDocumentContent",
            Some(json!({
                "frameId": frame_id,
                "html": html,
            })),
            Some(session_id),
        )
        .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Domain filter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DomainFilter {
    pub allowed_domains: Vec<String>,
}

impl DomainFilter {
    pub fn new(domains: &str) -> Self {
        let allowed = parse_domain_list(domains);
        Self {
            allowed_domains: allowed,
        }
    }

    pub fn is_allowed(&self, hostname: &str) -> bool {
        if self.allowed_domains.is_empty() {
            return true;
        }
        let hostname = hostname.to_lowercase();
        for pattern in &self.allowed_domains {
            if let Some(suffix) = pattern.strip_prefix("*.") {
                if hostname == suffix || hostname.ends_with(&format!(".{}", suffix)) {
                    return true;
                }
            } else if hostname == *pattern {
                return true;
            }
        }
        false
    }

    pub fn check_url(&self, url: &str) -> Result<(), String> {
        if self.allowed_domains.is_empty() {
            return Ok(());
        }
        let parsed = url::Url::parse(url).map_err(|_| format!("Invalid URL: {}", url))?;
        let hostname = parsed
            .host_str()
            .ok_or_else(|| format!("No hostname in URL: {}", url))?;
        if self.is_allowed(hostname) {
            Ok(())
        } else {
            Err(format!(
                "Domain '{}' is not in the allowed domains list",
                hostname
            ))
        }
    }
}

fn parse_domain_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

pub async fn sanitize_existing_pages(
    client: &CdpClient,
    pages: &[super::browser::PageInfo],
    filter: &DomainFilter,
) {
    for page in pages {
        if page.url.is_empty() || page.url == "about:blank" {
            continue;
        }
        if let Ok(parsed) = url::Url::parse(&page.url) {
            if let Some(hostname) = parsed.host_str() {
                if !filter.is_allowed(hostname) {
                    let _ = client
                        .send_command(
                            "Page.navigate",
                            Some(json!({ "url": "about:blank" })),
                            Some(&page.session_id),
                        )
                        .await;
                }
            }
        }
    }
}

pub async fn install_domain_filter_script(
    client: &CdpClient,
    session_id: &str,
    allowed_domains: &[String],
) -> Result<(), String> {
    if allowed_domains.is_empty() {
        return Ok(());
    }

    let domains_json = serde_json::to_string(allowed_domains).unwrap_or("[]".to_string());
    let script = format!(
        r#"(() => {{
            const _allowed = {};
            function _isDomainAllowed(hostname) {{
                hostname = hostname.toLowerCase();
                for (const p of _allowed) {{
                    if (p.startsWith('*.')) {{
                        const suffix = p.slice(2);
                        if (hostname === suffix || hostname.endsWith('.' + suffix)) return true;
                    }} else if (hostname === p) return true;
                }}
                return false;
            }}
            const OrigWS = window.WebSocket;
            window.WebSocket = function(url, protocols) {{
                try {{
                    const u = new URL(url, location.href);
                    if (!_isDomainAllowed(u.hostname)) throw new DOMException('WebSocket blocked: ' + u.hostname, 'SecurityError');
                }} catch(e) {{ if (e instanceof DOMException) throw e; }}
                return new OrigWS(url, protocols);
            }};
            window.WebSocket.prototype = OrigWS.prototype;
            const OrigES = window.EventSource;
            if (OrigES) {{
                window.EventSource = function(url, opts) {{
                    try {{
                        const u = new URL(url, location.href);
                        if (!_isDomainAllowed(u.hostname)) throw new DOMException('EventSource blocked: ' + u.hostname, 'SecurityError');
                    }} catch(e) {{ if (e instanceof DOMException) throw e; }}
                    return new OrigES(url, opts);
                }};
                window.EventSource.prototype = OrigES.prototype;
            }}
            const origBeacon = navigator.sendBeacon;
            if (origBeacon) {{
                navigator.sendBeacon = function(url, data) {{
                    try {{
                        const u = new URL(url, location.href);
                        if (!_isDomainAllowed(u.hostname)) return false;
                    }} catch(e) {{ return false; }}
                    return origBeacon.call(navigator, url, data);
                }};
            }}
        }})()"#,
        domains_json,
    );

    client
        .send_command(
            "Page.addScriptToEvaluateOnNewDocument",
            Some(json!({ "source": script })),
            Some(session_id),
        )
        .await?;

    Ok(())
}

/// Enable Fetch-based network interception for domain filtering.
/// This intercepts all requests and checks them against the allowed domains list.
/// The actual handling of `Fetch.requestPaused` events happens in
/// `resolve_fetch_paused` in the actions module.
pub async fn install_domain_filter_fetch(
    client: &CdpClient,
    session_id: &str,
    handle_auth_requests: bool,
) -> Result<(), String> {
    let mut params = json!({
        "patterns": [{ "urlPattern": "*" }]
    });
    if handle_auth_requests {
        params["handleAuthRequests"] = json!(true);
    }
    client
        .send_command("Fetch.enable", Some(params), Some(session_id))
        .await?;
    Ok(())
}

/// Install both layers of domain filtering on a session:
/// 1. JS patching (WebSocket, EventSource, sendBeacon)
/// 2. Fetch-based network interception
pub async fn install_domain_filter(
    client: &CdpClient,
    session_id: &str,
    allowed_domains: &[String],
    handle_auth_requests: bool,
) -> Result<(), String> {
    install_domain_filter_script(client, session_id, allowed_domains).await?;
    install_domain_filter_fetch(client, session_id, handle_auth_requests).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Console arg formatting (CDP RemoteObject → human-readable string)
// ---------------------------------------------------------------------------

/// Format a single CDP RemoteObject arg into a human-readable string.
/// Priority: value → preview → description.
pub fn format_console_arg(arg: &Value) -> Option<String> {
    let obj_type = arg.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let subtype = arg.get("subtype").and_then(|v| v.as_str());

    if obj_type == "undefined" {
        return Some("undefined".to_string());
    }

    if subtype == Some("null") {
        return Some("null".to_string());
    }

    // Primitive value
    if let Some(v) = arg.get("value") {
        return Some(match v {
            Value::String(s) => s.clone(),
            Value::Null => "null".to_string(),
            other => other.to_string(),
        });
    }

    // Skip preview for Map/Set — their description ("Map(1)", "Set(3)") is more useful
    // than their preview properties (which only show "size")
    if let Some(preview) = arg.get("preview") {
        let preview_subtype = preview.get("subtype").and_then(|v| v.as_str());
        if matches!(preview_subtype, Some("map" | "set" | "weakmap" | "weakset")) {
            return arg
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        let is_array = subtype == Some("array") || preview_subtype == Some("array");
        if let Some(props) = preview.get("properties").and_then(|v| v.as_array()) {
            let overflow = preview
                .get("overflow")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let formatted_props: Vec<String> = props
                .iter()
                .filter_map(|p| {
                    let value_str = p.get("value").and_then(|v| v.as_str())?;
                    let prop_type = p.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let formatted_value = if prop_type == "string" {
                        format!("\"{}\"", value_str)
                    } else {
                        value_str.to_string()
                    };
                    if is_array {
                        Some(formatted_value)
                    } else {
                        let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                        Some(format!("{}: {}", name, formatted_value))
                    }
                })
                .collect();

            let inner = if overflow {
                format!("{}, ...", formatted_props.join(", "))
            } else {
                formatted_props.join(", ")
            };

            return if is_array {
                Some(format!("[{}]", inner))
            } else {
                Some(format!("{{{}}}", inner))
            };
        }
    }

    // Fallback to description
    arg.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Format an array of CDP RemoteObject args into a single space-separated string.
pub fn format_console_args(args: &[Value]) -> String {
    args.iter()
        .filter_map(format_console_arg)
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Console and error tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ConsoleEntry {
    pub level: String,
    pub text: String,
    pub args: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct ErrorEntry {
    pub text: String,
    pub url: Option<String>,
    pub line: Option<i64>,
    pub column: Option<i64>,
}

pub struct EventTracker {
    pub console_entries: Vec<ConsoleEntry>,
    pub error_entries: Vec<ErrorEntry>,
    pub max_entries: usize,
}

impl EventTracker {
    pub fn new() -> Self {
        Self {
            console_entries: Vec::new(),
            error_entries: Vec::new(),
            max_entries: 1000,
        }
    }

    pub fn add_console(&mut self, level: &str, text: &str, args: Vec<Value>) {
        if self.console_entries.len() >= self.max_entries {
            self.console_entries.remove(0);
        }
        self.console_entries.push(ConsoleEntry {
            level: level.to_string(),
            text: text.to_string(),
            args,
        });
    }

    pub fn add_error(
        &mut self,
        text: &str,
        url: Option<&str>,
        line: Option<i64>,
        col: Option<i64>,
    ) {
        if self.error_entries.len() >= self.max_entries {
            self.error_entries.remove(0);
        }
        self.error_entries.push(ErrorEntry {
            text: text.to_string(),
            url: url.map(String::from),
            line,
            column: col,
        });
    }

    pub fn clear_console(&mut self) {
        self.console_entries.clear();
    }

    pub fn get_console_json(&self) -> Value {
        let messages: Vec<Value> = self
            .console_entries
            .iter()
            .map(|e| {
                let mut msg = json!({ "type": e.level, "text": e.text });
                if !e.args.is_empty() {
                    msg.as_object_mut()
                        .unwrap()
                        .insert("args".to_string(), Value::Array(e.args.clone()));
                }
                msg
            })
            .collect();
        json!({ "messages": messages })
    }

    pub fn get_errors_json(&self) -> Value {
        let entries: Vec<Value> = self
            .error_entries
            .iter()
            .map(|e| {
                json!({
                    "text": e.text,
                    "url": e.url,
                    "line": e.line,
                    "column": e.column,
                })
            })
            .collect();
        json!({ "errors": entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_filter_exact() {
        let filter = DomainFilter::new("example.com");
        assert!(filter.is_allowed("example.com"));
        assert!(!filter.is_allowed("other.com"));
    }

    #[test]
    fn test_domain_filter_wildcard() {
        let filter = DomainFilter::new("*.example.com");
        assert!(filter.is_allowed("example.com"));
        assert!(filter.is_allowed("api.example.com"));
        assert!(filter.is_allowed("sub.api.example.com"));
        assert!(!filter.is_allowed("other.com"));
    }

    #[test]
    fn test_domain_filter_empty() {
        let filter = DomainFilter::new("");
        assert!(filter.is_allowed("anything.com"));
    }

    #[test]
    fn test_domain_filter_multiple() {
        let filter = DomainFilter::new("example.com, *.api.io");
        assert!(filter.is_allowed("example.com"));
        assert!(filter.is_allowed("api.io"));
        assert!(filter.is_allowed("v1.api.io"));
        assert!(!filter.is_allowed("other.com"));
    }

    #[test]
    fn test_parse_domain_list() {
        let domains = parse_domain_list("A.com, B.com , *.C.com");
        assert_eq!(domains, vec!["a.com", "b.com", "*.c.com"]);
    }

    #[test]
    fn test_event_tracker() {
        let mut tracker = EventTracker::new();
        tracker.add_console("log", "hello", vec![]);
        tracker.add_error("oops", Some("test.js"), Some(1), Some(5));

        assert_eq!(tracker.console_entries.len(), 1);
        assert_eq!(tracker.error_entries.len(), 1);
    }

    #[test]
    fn test_console_json_includes_args() {
        let mut tracker = EventTracker::new();
        let raw_args = vec![
            json!({"type": "string", "value": "hello"}),
            json!({"type": "number", "value": 42}),
        ];
        tracker.add_console("log", "hello 42", raw_args);

        let result = tracker.get_console_json();
        let messages = result.get("messages").unwrap().as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].get("text").unwrap(), "hello 42");
        let args = messages[0].get("args").unwrap().as_array().unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], json!({"type": "string", "value": "hello"}));
        assert_eq!(args[1], json!({"type": "number", "value": 42}));
    }

    #[test]
    fn test_console_json_empty_args_omits_field() {
        let mut tracker = EventTracker::new();
        tracker.add_console("log", "text only", vec![]);

        let result = tracker.get_console_json();
        let messages = result.get("messages").unwrap().as_array().unwrap();
        assert!(messages[0].get("args").is_none());
    }

    // -- format_console_arg: primitives --

    #[test]
    fn test_format_arg_string() {
        let arg = json!({"type": "string", "value": "hello"});
        assert_eq!(format_console_arg(&arg), Some("hello".to_string()));
    }

    #[test]
    fn test_format_arg_number() {
        let arg = json!({"type": "number", "value": 42});
        assert_eq!(format_console_arg(&arg), Some("42".to_string()));
    }

    #[test]
    fn test_format_arg_null() {
        let arg = json!({"type": "object", "subtype": "null", "value": null});
        assert_eq!(format_console_arg(&arg), Some("null".to_string()));
    }

    #[test]
    fn test_format_arg_undefined() {
        let arg = json!({"type": "undefined"});
        assert_eq!(format_console_arg(&arg), Some("undefined".to_string()));
    }

    // -- format_console_arg: objects with preview --

    #[test]
    fn test_format_arg_object_preview() {
        let arg = json!({
            "type": "object",
            "preview": {
                "properties": [
                    {"name": "userId", "type": "string", "value": "abc123"},
                    {"name": "count", "type": "number", "value": "42"}
                ],
                "overflow": false
            }
        });
        assert_eq!(
            format_console_arg(&arg),
            Some("{userId: \"abc123\", count: 42}".to_string())
        );
    }

    #[test]
    fn test_format_arg_object_preview_overflow() {
        let arg = json!({
            "type": "object",
            "preview": {
                "properties": [
                    {"name": "a", "type": "number", "value": "1"}
                ],
                "overflow": true
            }
        });
        assert_eq!(format_console_arg(&arg), Some("{a: 1, ...}".to_string()));
    }

    // -- format_console_arg: arrays with preview --

    #[test]
    fn test_format_arg_array_preview() {
        let arg = json!({
            "type": "object",
            "subtype": "array",
            "preview": {
                "subtype": "array",
                "properties": [
                    {"name": "0", "type": "number", "value": "1"},
                    {"name": "1", "type": "number", "value": "2"},
                    {"name": "2", "type": "number", "value": "3"}
                ],
                "overflow": false
            }
        });
        assert_eq!(format_console_arg(&arg), Some("[1, 2, 3]".to_string()));
    }

    // -- format_console_arg: map/set use description --

    #[test]
    fn test_format_arg_map_uses_description() {
        let arg = json!({
            "type": "object",
            "subtype": "map",
            "description": "Map(1)",
            "preview": {
                "subtype": "map",
                "properties": [{"name": "size", "type": "number", "value": "1"}]
            }
        });
        assert_eq!(format_console_arg(&arg), Some("Map(1)".to_string()));
    }

    // -- format_console_arg: fallback --

    #[test]
    fn test_format_arg_description_fallback() {
        let arg = json!({"type": "object", "description": "RegExp"});
        assert_eq!(format_console_arg(&arg), Some("RegExp".to_string()));
    }

    #[test]
    fn test_format_arg_no_value_no_preview_no_description() {
        let arg = json!({"type": "object"});
        assert_eq!(format_console_arg(&arg), None);
    }

    // -- format_console_args --

    #[test]
    fn test_format_console_args_join() {
        let args = vec![
            json!({"type": "string", "value": "user"}),
            json!({
                "type": "object",
                "preview": {
                    "properties": [{"name": "id", "type": "number", "value": "1"}],
                    "overflow": false
                }
            }),
        ];
        assert_eq!(format_console_args(&args), "user {id: 1}");
    }

    #[test]
    fn test_format_console_args_filters_none() {
        // An arg that returns None should be skipped, not produce empty string
        let args = vec![
            json!({"type": "string", "value": "before"}),
            json!({"type": "object"}), // no value, preview, or description → None
            json!({"type": "string", "value": "after"}),
        ];
        assert_eq!(format_console_args(&args), "before after");
    }
}
#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::browser::{
        should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo,
        ProcessExitObservation, WaitUntil,
    };
    use crate::native::network::{self, DomainFilter, EventTracker};
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use agent_browser_cdp::client::CdpClient;
    use serde_json::{json, Map, Value};
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
    use std::time::{Duration, Instant};
    use tokio::sync::{broadcast, oneshot, RwLock};
    pub(crate) async fn handle_headers(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let headers_value = cmd.get("headers").ok_or("Missing 'headers' parameter")?;
        let headers: HashMap<String, String> = headers_value
            .as_object()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            })
            .unwrap_or_default();
        network::set_extra_headers(&mgr.client, &session_id, &headers).await?;
        Ok(json!({ "set" : true }))
    }
    pub(crate) async fn handle_offline(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let offline = cmd.get("offline").and_then(|v| v.as_bool()).unwrap_or(true);
        network::set_offline(&mgr.client, &session_id, offline).await?;
        Ok(json!({ "offline" : offline }))
    }
    pub(crate) async fn handle_responsebody(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let url_pattern = cmd
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'url' parameter")?;
        let timeout_ms = state.timeout_ms(cmd);
        let mut rx = mgr.client.subscribe();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(format!(
                    "Timeout waiting for response matching '{}'",
                    url_pattern
                ));
            }
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(event)) => {
                    if event.method == "Network.responseReceived"
                        && event.session_id.as_deref() == Some(&session_id)
                    {
                        if let Some(resp_url) = event
                            .params
                            .get("response")
                            .and_then(|r| r.get("url"))
                            .and_then(|u| u.as_str())
                        {
                            if resp_url.contains(url_pattern) {
                                let request_id = event
                                    .params
                                    .get("requestId")
                                    .and_then(|v| v.as_str())
                                    .ok_or("No requestId in response event")?;
                                let status = event
                                    .params
                                    .get("response")
                                    .and_then(|r| r.get("status"))
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                let headers = event
                                    .params
                                    .get("response")
                                    .and_then(|r| r.get("headers"))
                                    .cloned()
                                    .unwrap_or(json!({}));
                                let body_result = mgr
                                    .client
                                    .send_command(
                                        "Network.getResponseBody",
                                        Some(json!({ "requestId" : request_id })),
                                        Some(&session_id),
                                    )
                                    .await?;
                                let body = body_result
                                    .get("body")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                return Ok(json!(
                                    { "body" : body, "status" : status, "headers" : headers }
                                ));
                            }
                        }
                    }
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(_)) => return Err("Event stream closed".to_string()),
                Err(_) => {
                    return Err(format!(
                        "Timeout waiting for response matching '{}'",
                        url_pattern
                    ));
                }
            }
        }
    }
    pub(crate) async fn resolve_fetch_paused(
        client: &CdpClient,
        domain_filter: Option<&DomainFilter>,
        routes: &[RouteEntry],
        origin_headers: &HashMap<String, HashMap<String, String>>,
        paused: &FetchPausedRequest,
    ) {
        let session_id = &paused.session_id;
        if let Some(filter) = domain_filter {
            if let Ok(parsed) = url::Url::parse(&paused.url) {
                let scheme = parsed.scheme();
                if scheme != "http" && scheme != "https" {
                    if paused.resource_type.eq_ignore_ascii_case("document") {
                        let _ = client
                            .send_command(
                                "Fetch.failRequest",
                                Some(json!(
                                    { "requestId" : paused.request_id, "errorReason" :
                                    "BlockedByClient" }
                                )),
                                Some(session_id),
                            )
                            .await;
                    } else {
                        let _ = client
                            .send_command(
                                "Fetch.continueRequest",
                                Some(json!({ "requestId" : paused.request_id })),
                                Some(session_id),
                            )
                            .await;
                    }
                    return;
                }
                if let Some(hostname) = parsed.host_str() {
                    if !filter.is_allowed(hostname) {
                        if paused.resource_type.eq_ignore_ascii_case("document") {
                            let error_body = format!(
                                "<html><body><h1>Blocked</h1><p>Navigation to {} is not allowed by domain filter.</p></body></html>",
                                hostname
                            );
                            let encoded = base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                error_body.as_bytes(),
                            );
                            let _ = client
                                .send_command(
                                    "Fetch.fulfillRequest",
                                    Some(json!(
                                        { "requestId" : paused.request_id, "responseCode" : 403,
                                        "responseHeaders" : [{ "name" : "Content-Type", "value" :
                                        "text/html" },], "body" : encoded, }
                                    )),
                                    Some(session_id),
                                )
                                .await;
                        } else {
                            let _ = client
                                .send_command(
                                    "Fetch.failRequest",
                                    Some(json!(
                                        { "requestId" : paused.request_id, "errorReason" :
                                        "BlockedByClient" }
                                    )),
                                    Some(session_id),
                                )
                                .await;
                        }
                        return;
                    }
                }
            }
        }
        for route in routes {
            let matches = if route.url_pattern == "*" {
                true
            } else if route.url_pattern.contains('*') {
                let parts: Vec<&str> = route.url_pattern.split('*').collect();
                if parts.len() == 2 {
                    paused.url.starts_with(parts[0]) && paused.url.ends_with(parts[1])
                } else {
                    paused.url.contains(&route.url_pattern)
                }
            } else {
                paused.url.contains(&route.url_pattern)
            };
            if matches {
                if route.abort {
                    let _ = client
                        .send_command(
                            "Fetch.failRequest",
                            Some(json!(
                                { "requestId" : paused.request_id, "errorReason" : "Failed"
                                }
                            )),
                            Some(session_id),
                        )
                        .await;
                    return;
                }
                if let Some(ref resp) = route.response {
                    let status = resp.status.unwrap_or(200);
                    let body_str = resp.body.as_deref().unwrap_or("");
                    let encoded = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        body_str.as_bytes(),
                    );
                    let mut headers = vec![];
                    if let Some(ct) = &resp.content_type {
                        headers.push(json!({ "name" : "Content-Type", "value" : ct }));
                    }
                    if let Some(h) = &resp.headers {
                        for (k, v) in h {
                            headers.push(json!({ "name" : k, "value" : v }));
                        }
                    }
                    let _ = client
                        .send_command(
                            "Fetch.fulfillRequest",
                            Some(json!(
                                { "requestId" : paused.request_id, "responseCode" : status,
                                "responseHeaders" : headers, "body" : encoded, }
                            )),
                            Some(session_id),
                        )
                        .await;
                    return;
                }
            }
        }
        let extra = url::Url::parse(&paused.url)
            .ok()
            .map(|u| u.origin().ascii_serialization())
            .and_then(|o| origin_headers.get(&o));
        if let Some(extra_headers) = extra {
            let mut combined: Vec<Value> = Vec::new();
            if let Some(ref orig) = paused.request_headers {
                for (k, v) in orig {
                    if !extra_headers.keys().any(|ek| ek.eq_ignore_ascii_case(k)) {
                        if let Some(s) = v.as_str() {
                            combined.push(json!({ "name" : k, "value" : s }));
                        }
                    }
                }
            }
            for (k, v) in extra_headers {
                combined.push(json!({ "name" : k, "value" : v }));
            }
            let _ = client
                .send_command(
                    "Fetch.continueRequest",
                    Some(json!({ "requestId" : paused.request_id, "headers" : combined })),
                    Some(session_id),
                )
                .await;
        } else {
            let _ = client
                .send_command(
                    "Fetch.continueRequest",
                    Some(json!({ "requestId" : paused.request_id })),
                    Some(session_id),
                )
                .await;
        }
    }
    /// Build the Fetch.enable patterns list from current routes, domain filter,
    /// and origin headers state.  When domain filtering or origin-scoped headers
    /// are active a wildcard pattern is included so all requests are intercepted.
    pub(crate) async fn build_fetch_patterns(state: &DaemonState) -> Vec<Value> {
        let routes = state.routes.read().await;
        let mut patterns: Vec<Value> = routes
            .iter()
            .map(|r| json!({ "urlPattern" : r.url_pattern }))
            .collect();
        let has_domain_filter = state.domain_filter.read().await.is_some();
        let has_origin_headers = !state.origin_headers.read().await.is_empty();
        let has_proxy_creds = state.proxy_credentials.read().await.is_some();
        if (has_domain_filter || has_origin_headers || has_proxy_creds)
            && !patterns.iter().any(|p| p["urlPattern"] == "*")
        {
            patterns.push(json!({ "urlPattern" : "*" }));
        }
        patterns
    }
    /// Build the full Fetch.enable params object, including `handleAuthRequests`
    /// when proxy credentials are configured.
    pub(crate) async fn build_fetch_enable_params(
        state: &DaemonState,
        patterns: Vec<Value>,
    ) -> Value {
        let has_proxy_creds = state.proxy_credentials.read().await.is_some();
        if has_proxy_creds {
            json!({ "patterns" : patterns, "handleAuthRequests" : true })
        } else {
            json!({ "patterns" : patterns })
        }
    }
    pub(crate) async fn handle_route(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let url_pattern = cmd
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'url' parameter")?
            .to_string();
        let abort = cmd.get("abort").and_then(|v| v.as_bool()).unwrap_or(false);
        let response = cmd.get("response").and_then(|v| {
            if v.is_null() {
                return None;
            }
            Some(RouteResponse {
                status: v.get("status").and_then(|s| s.as_u64()).map(|s| s as u16),
                body: v.get("body").and_then(|s| s.as_str()).map(String::from),
                content_type: v
                    .get("contentType")
                    .and_then(|s| s.as_str())
                    .map(String::from),
                headers: v.get("headers").and_then(|h| {
                    h.as_object().map(|m| {
                        m.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                }),
            })
        });
        {
            let mut routes = state.routes.write().await;
            routes.push(RouteEntry {
                url_pattern: url_pattern.clone(),
                response,
                abort,
            });
        }
        let patterns = build_fetch_patterns(state).await;
        let params = build_fetch_enable_params(state, patterns).await;
        mgr.client
            .send_command("Fetch.enable", Some(params), Some(&session_id))
            .await?;
        Ok(json!({ "routed" : url_pattern }))
    }
    pub(crate) async fn handle_unroute(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let url = cmd.get("url").and_then(|v| v.as_str());
        {
            let mut routes = state.routes.write().await;
            match url {
                Some(pattern) => {
                    routes.retain(|r| r.url_pattern != pattern);
                }
                None => {
                    routes.clear();
                }
            }
        }
        let patterns = build_fetch_patterns(state).await;
        if patterns.is_empty() {
            mgr.client
                .send_command("Fetch.disable", None, Some(&session_id))
                .await?;
        } else {
            let params = build_fetch_enable_params(state, patterns).await;
            mgr.client
                .send_command("Fetch.enable", Some(params), Some(&session_id))
                .await?;
        }
        let label = url.unwrap_or("all");
        Ok(json!({ "unrouted" : label }))
    }
    pub(crate) fn matches_status_filter(status: Option<i64>, filter: &str) -> bool {
        let Some(code) = status else { return false };
        let f = filter.to_lowercase();
        if let Ok(exact) = f.parse::<i64>() {
            return code == exact;
        }
        if f.len() == 3 && f.ends_with("xx") {
            if let Ok(prefix) = f[..1].parse::<i64>() {
                return code / 100 == prefix;
            }
        }
        if let Some((lo, hi)) = f.split_once('-') {
            if let (Ok(lo), Ok(hi)) = (lo.parse::<i64>(), hi.parse::<i64>()) {
                return code >= lo && code <= hi;
            }
        }
        false
    }
}
pub(crate) use action_commands::*;
#[cfg(test)]
mod action_tests;
