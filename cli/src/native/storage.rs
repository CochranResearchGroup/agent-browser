use serde_json::{json, Value};

use super::cdp::client::CdpClient;
use super::cdp::types::EvaluateParams;

pub async fn storage_get(
    client: &CdpClient,
    session_id: &str,
    storage_type: &str,
    key: Option<&str>,
) -> Result<Value, String> {
    let st = storage_js_name(storage_type);

    if let Some(k) = key {
        let js = format!(
            "{}.getItem({})",
            st,
            serde_json::to_string(k).unwrap_or_default()
        );
        let result = eval_simple(client, session_id, &js).await?;
        Ok(json!({ "key": k, "value": result }))
    } else {
        let js = format!(
            r#"(() => {{
                const s = {};
                const data = {{}};
                for (let i = 0; i < s.length; i++) {{
                    const key = s.key(i);
                    data[key] = s.getItem(key);
                }}
                return data;
            }})()"#,
            st
        );
        let result = eval_simple(client, session_id, &js).await?;
        Ok(json!({ "data": result }))
    }
}

pub async fn storage_set(
    client: &CdpClient,
    session_id: &str,
    storage_type: &str,
    key: &str,
    value: &str,
) -> Result<(), String> {
    let st = storage_js_name(storage_type);
    let js = format!(
        "{}.setItem({}, {})",
        st,
        serde_json::to_string(key).unwrap_or_default(),
        serde_json::to_string(value).unwrap_or_default(),
    );
    eval_simple(client, session_id, &js).await?;
    Ok(())
}

pub async fn storage_clear(
    client: &CdpClient,
    session_id: &str,
    storage_type: &str,
) -> Result<(), String> {
    let st = storage_js_name(storage_type);
    let js = format!("{}.clear()", st);
    eval_simple(client, session_id, &js).await?;
    Ok(())
}

fn storage_js_name(storage_type: &str) -> &str {
    match storage_type {
        "session" => "sessionStorage",
        _ => "localStorage",
    }
}

async fn eval_simple(client: &CdpClient, session_id: &str, js: &str) -> Result<Value, String> {
    let result: super::cdp::types::EvaluateResult = client
        .send_command_typed(
            "Runtime.evaluate",
            &EvaluateParams {
                expression: js.to_string(),
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(session_id),
        )
        .await?;

    if let Some(ref details) = result.exception_details {
        return Err(format!("Storage error: {}", details.text));
    }

    Ok(result.result.value.unwrap_or(Value::Null))
}
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
    pub(crate) async fn handle_storage_get(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let storage_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("local");
        let key = cmd.get("key").and_then(|v| v.as_str());
        storage::storage_get(&mgr.client, &session_id, storage_type, key).await
    }
    pub(crate) async fn handle_storage_set(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let storage_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("local");
        let key = cmd
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'key' parameter")?;
        let value = cmd
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'value' parameter")?;
        storage::storage_set(&mgr.client, &session_id, storage_type, key, value).await?;
        let current_url = mgr.get_url().await.unwrap_or_default();
        if let Ok(parsed) = url::Url::parse(&current_url) {
            let origin = parsed.origin().ascii_serialization();
            if origin != "null" {
                let tracked = state
                    .tracked_origin_storage
                    .entry(origin.clone())
                    .or_insert_with(|| state::OriginStorage {
                        origin,
                        local_storage: Vec::new(),
                        session_storage: Vec::new(),
                    });
                let entries = if storage_type == "session" {
                    &mut tracked.session_storage
                } else {
                    &mut tracked.local_storage
                };
                if let Some(existing) = entries.iter_mut().find(|entry| entry.name == key) {
                    existing.value = value.to_string();
                } else {
                    entries.push(state::StorageEntry {
                        name: key.to_string(),
                        value: value.to_string(),
                    });
                }
            }
        }
        Ok(json!({ "set" : true }))
    }
    pub(crate) async fn handle_storage_clear(
        cmd: &Value,
        state: &DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let storage_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("local");
        storage::storage_clear(&mgr.client, &session_id, storage_type).await?;
        Ok(json!({ "cleared" : true }))
    }
}
pub(crate) use action_commands::*;
