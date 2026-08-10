use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::cdp::client::CdpClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    #[serde(default)]
    pub expires: f64,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub session: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
}

pub async fn get_all_cookies(client: &CdpClient, session_id: &str) -> Result<Vec<Cookie>, String> {
    let result = client
        .send_command_no_params("Network.getAllCookies", Some(session_id))
        .await?;

    let cookies: Vec<Cookie> = result
        .get("cookies")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    Ok(cookies)
}

pub async fn get_cookies(
    client: &CdpClient,
    session_id: &str,
    urls: Option<Vec<String>>,
) -> Result<Vec<Cookie>, String> {
    let params = match urls {
        Some(ref u) if !u.is_empty() => json!({ "urls": u }),
        _ => json!({}),
    };

    let result = client
        .send_command("Network.getCookies", Some(params), Some(session_id))
        .await?;

    let cookies: Vec<Cookie> = result
        .get("cookies")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    Ok(cookies)
}

pub async fn set_cookies(
    client: &CdpClient,
    session_id: &str,
    cookies: Vec<Value>,
    current_url: Option<&str>,
) -> Result<(), String> {
    let cookies: Vec<Value> = cookies
        .into_iter()
        .map(|mut c| {
            // Auto-fill url if no domain/path/url provided
            if c.get("url").is_none() && c.get("domain").is_none() && current_url.is_some() {
                c.as_object_mut().map(|m| {
                    m.insert(
                        "url".to_string(),
                        Value::String(current_url.unwrap().to_string()),
                    )
                });
            }
            c
        })
        .collect();

    client
        .send_command(
            "Network.setCookies",
            Some(json!({ "cookies": cookies })),
            Some(session_id),
        )
        .await?;

    Ok(())
}

pub async fn clear_cookies(client: &CdpClient, session_id: &str) -> Result<(), String> {
    client
        .send_command_no_params("Network.clearBrowserCookies", Some(session_id))
        .await?;
    Ok(())
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
    use crate::native::cdp::client::CdpClient;
    use crate::native::cookies;
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use crate::native::webdriver::backend::BrowserBackend;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Map, Value};
    pub(crate) async fn handle_cookies_get(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        if let Some(ref wb) = state.webdriver_backend {
            if state.browser.is_none() {
                let cookies_list = wb.get_cookies().await?;
                return Ok(json!({ "cookies" : cookies_list }));
            }
        }
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let urls = cmd.get("urls").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
        let cookies_list = cookies::get_cookies(&mgr.client, &session_id, urls).await?;
        Ok(json!({ "cookies" : cookies_list }))
    }
    pub(crate) async fn handle_cookies_set(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let url = mgr.get_url().await.ok();
        let cookie_values = if let Some(arr) = cmd.get("cookies").and_then(|v| v.as_array()) {
            arr.clone()
        } else {
            let mut cookie = serde_json::Map::new();
            for key in &[
                "name", "value", "domain", "path", "expires", "httpOnly", "secure", "sameSite",
                "url",
            ] {
                if let Some(v) = cmd.get(*key) {
                    if !v.is_null() {
                        cookie.insert(key.to_string(), v.clone());
                    }
                }
            }
            vec![Value::Object(cookie)]
        };
        cookies::set_cookies(&mgr.client, &session_id, cookie_values, url.as_deref()).await?;
        Ok(json!({ "set" : true }))
    }
    pub(crate) async fn handle_cookies_clear(state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        cookies::clear_cookies(&mgr.client, &session_id).await?;
        Ok(json!({ "cleared" : true }))
    }
}
pub(crate) use action_commands::*;
