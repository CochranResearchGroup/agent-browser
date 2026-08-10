#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::common::*;
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::service_diagnostics::truncate_utf8;
    /// Stop HAR recording and write the captured requests to disk.
    pub(crate) async fn handle_har_stop(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let path = har_output_path(cmd.get("path").and_then(|v| v.as_str()));
        state.har_recording = false;
        let entries: Vec<Value> = state.har_entries.drain(..).map(har_entry_to_json).collect();
        let request_count = entries.len();
        let browser = har_browser_metadata(state).await;
        let mut log = json!(
            { "version" : "1.2", "creator" : { "name" : "agent-browser", "version" :
            env!("CARGO_PKG_VERSION") }, "entries" : entries }
        );
        if let Some(browser) = browser {
            log["browser"] = browser;
        }
        let har = json!({ "log" : log });
        let har_str = serde_json::to_string_pretty(&har)
            .map_err(|e| format!("Failed to serialize HAR: {}", e))?;
        std::fs::write(&path, har_str).map_err(|e| format!("Failed to write HAR: {}", e))?;
        Ok(json!({ "path" : path, "requestCount" : request_count }))
    }
    /// Convert a `HarEntry` (collected from CDP events) into a HAR 1.2 entry object.
    pub(crate) fn har_entry_to_json(e: HarEntry) -> Value {
        let started_date_time = har_wall_time_to_rfc3339(e.wall_time);
        let request_cookies = e
            .request_headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("cookie"))
            .map(|(_, v)| har_parse_request_cookies(v))
            .unwrap_or_default();
        let query_string = har_parse_query_string(&e.url);
        let req_headers: Vec<Value> = e
            .request_headers
            .iter()
            .map(|(k, v)| json!({ "name" : k, "value" : v }))
            .collect();
        let resp_cookies: Vec<Value> = e
            .response_headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
            .map(|(_, v)| {
                let name_value = v.split(';').next().unwrap_or("");
                let (name, value) = name_value.split_once('=').unwrap_or((name_value, ""));
                json!({ "name" : name.trim(), "value" : value.trim() })
            })
            .collect();
        let resp_headers: Vec<Value> = e
            .response_headers
            .iter()
            .map(|(k, v)| json!({ "name" : k, "value" : v }))
            .collect();
        let (timings, total_time) =
            har_compute_timings(e.cdp_timing.as_ref(), e.loading_finished_timestamp);
        let mime_type = if e.mime_type.is_empty() {
            "application/octet-stream".to_string()
        } else {
            e.mime_type
        };
        let post_content_type = e
            .request_headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("text/plain")
            .to_string();
        let mut request = json!(
            { "method" : e.method, "url" : e.url, "httpVersion" : e.http_version,
            "cookies" : request_cookies, "headers" : req_headers, "queryString" :
            query_string, "headersSize" : - 1, "bodySize" : e.request_body_size, }
        );
        if let Some(body) = e.post_data {
            request["postData"] = json!(
                { "mimeType" : post_content_type, "text" : body }
            );
        }
        json!(
            { "startedDateTime" : started_date_time, "time" : total_time, "request" :
            request, "response" : { "status" : e.status.unwrap_or(0), "statusText" : e
            .status_text, "httpVersion" : e.http_version, "cookies" : resp_cookies,
            "headers" : resp_headers, "content" : { "size" : e.response_body_size,
            "mimeType" : mime_type, }, "redirectURL" : e.redirect_url, "headersSize" : -
            1, "bodySize" : e.response_body_size, }, "cache" : {}, "timings" : timings,
            "_resourceType" : e.resource_type, }
        )
    }
    /// Convert a CDP headers object (`{ "Name": "value", ... }`) into a flat
    /// `Vec<(name, value)>` preserving insertion order.
    pub(crate) fn har_extract_headers(headers_val: Option<&Value>) -> Vec<(String, String)> {
        headers_val
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Map a CDP `response.protocol` value to an HTTP-version string as required
    /// by the HAR spec (e.g. `"h2"` → `"HTTP/2.0"`).
    pub(crate) fn har_cdp_protocol_to_http_version(protocol: &str) -> String {
        match protocol.to_ascii_lowercase().as_str() {
            "h2" => "HTTP/2.0".to_string(),
            "h3" => "HTTP/3.0".to_string(),
            "http/1.0" => "HTTP/1.0".to_string(),
            _ => "HTTP/1.1".to_string(),
        }
    }
    /// Parse query-string parameters from a URL into a HAR `queryString` array.
    pub(crate) fn har_parse_query_string(url_str: &str) -> Vec<Value> {
        url::Url::parse(url_str)
            .map(|u| {
                u.query_pairs()
                    .map(|(k, v)| json!({ "name" : k.as_ref(), "value" : v.as_ref() }))
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Parse a `Cookie: name1=val1; name2=val2` header value into HAR cookie objects.
    pub(crate) fn har_parse_request_cookies(cookie_header: &str) -> Vec<Value> {
        cookie_header
            .split(';')
            .filter_map(|pair| {
                let pair = pair.trim();
                if pair.is_empty() {
                    return None;
                }
                let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
                Some(json!({ "name" : name.trim(), "value" : value.trim() }))
            })
            .collect()
    }
    /// Compute HAR `timings` and total `time` (ms) from a CDP `ResourceTiming`
    /// object and the optional `Network.loadingFinished` monotonic timestamp.
    ///
    /// CDP timing values are milliseconds relative to `requestTime` (seconds since
    /// browser start). A value of `-1` means the phase did not occur.
    pub(crate) fn har_compute_timings(
        cdp_timing: Option<&Value>,
        loading_finished_ts: Option<f64>,
    ) -> (Value, f64) {
        let Some(t) = cdp_timing else {
            return (json!({ "send" : 0, "wait" : 0, "receive" : 0 }), 0.0);
        };
        let get = |key: &str| t.get(key).and_then(|v| v.as_f64()).unwrap_or(-1.0);
        let request_time = get("requestTime");
        let dns_start = get("dnsStart");
        let dns_end = get("dnsEnd");
        let connect_start = get("connectStart");
        let connect_end = get("connectEnd");
        let ssl_start = get("sslStart");
        let ssl_end = get("sslEnd");
        let send_start = get("sendStart");
        let send_end = get("sendEnd");
        let recv_headers_start = get("receiveHeadersStart");
        let recv_headers_end = get("receiveHeadersEnd");
        let dns = if dns_start >= 0.0 && dns_end >= 0.0 {
            dns_end - dns_start
        } else {
            -1.0
        };
        let connect = if connect_start >= 0.0 && connect_end >= 0.0 {
            connect_end - connect_start
        } else {
            -1.0
        };
        let ssl = if ssl_start >= 0.0 && ssl_end >= 0.0 {
            ssl_end - ssl_start
        } else {
            -1.0
        };
        let send = (send_end - send_start).max(0.0);
        let wait_end = if recv_headers_start >= 0.0 {
            recv_headers_start
        } else {
            recv_headers_end
        };
        let wait = if send_end >= 0.0 && wait_end >= send_end {
            wait_end - send_end
        } else {
            0.0
        };
        let receive = loading_finished_ts
            .filter(|_| request_time >= 0.0 && recv_headers_end >= 0.0)
            .map(|lf_ts| {
                let recv_start_abs = request_time + recv_headers_end / 1000.0;
                ((lf_ts - recv_start_abs) * 1000.0).max(0.0)
            })
            .unwrap_or(0.0);
        let blocked = if dns_start > 0.0 {
            dns_start
        } else if connect_start > 0.0 {
            connect_start
        } else if send_start > 0.0 {
            send_start
        } else {
            -1.0
        };
        let total: f64 = [
            if blocked > 0.0 { blocked } else { 0.0 },
            if dns >= 0.0 { dns } else { 0.0 },
            if connect >= 0.0 { connect } else { 0.0 },
            send,
            wait,
            receive,
        ]
        .iter()
        .sum();
        let mut timings = json!({ "send" : send, "wait" : wait, "receive" : receive });
        if blocked > 0.0 {
            timings["blocked"] = json!(blocked);
        }
        if dns >= 0.0 {
            timings["dns"] = json!(dns);
        }
        if connect >= 0.0 {
            timings["connect"] = json!(connect);
        }
        if ssl >= 0.0 {
            timings["ssl"] = json!(ssl);
        }
        (timings, total)
    }
    /// Format a Unix epoch timestamp (seconds, fractional) as RFC 3339 using the
    /// `time` crate, e.g. `"2024-03-17T10:30:00.456Z"`.
    pub(crate) fn har_wall_time_to_rfc3339(wall_time: f64) -> String {
        if wall_time > 0.0 {
            let nanos = (wall_time * 1_000_000_000.0).round() as i128;
            if let Ok(dt) = OffsetDateTime::from_unix_timestamp_nanos(nanos) {
                if let Ok(s) = dt.format(&Rfc3339) {
                    return s;
                }
            }
        }
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    }
    pub(crate) fn har_output_path(explicit_path: Option<&str>) -> String {
        match explicit_path {
            Some(path) => path.to_string(),
            None => {
                let dir = get_har_dir();
                let _ = std::fs::create_dir_all(&dir);
                dir.join(format!("har-{}.har", unix_timestamp_millis()))
                    .to_string_lossy()
                    .to_string()
            }
        }
    }
    pub(crate) fn get_har_dir() -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            home.join(".agent-browser").join("tmp").join("har")
        } else {
            std::env::temp_dir().join("agent-browser").join("har")
        }
    }
    pub(crate) fn unix_timestamp_millis() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    }
    pub(crate) async fn har_browser_metadata(state: &DaemonState) -> Option<Value> {
        let mgr = state.browser.as_ref()?;
        if !mgr.is_connection_alive().await {
            return None;
        }
        let version = mgr
            .client
            .send_command_no_params("Browser.getVersion", None)
            .await
            .ok()?;
        browser_metadata_from_version(&version)
    }
    pub(crate) fn browser_metadata_from_version(version: &Value) -> Option<Value> {
        let product = version.get("product").and_then(|v| v.as_str())?;
        let (name, browser_version) = product.split_once('/').unwrap_or((product, ""));
        Some(json!({ "name" : name, "version" : browser_version, }))
    }
}
pub(crate) use action_commands::*;
