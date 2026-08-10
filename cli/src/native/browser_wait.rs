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
    pub(crate) async fn wait_for_selector(
        client: &super::super::cdp::client::CdpClient,
        session_id: &str,
        selector: &str,
        state: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let check_fn = match state {
            "attached" => {
                format!(
                    "!!document.querySelector({})",
                    serde_json::to_string(selector).unwrap_or_default()
                )
            }
            "detached" => {
                format!(
                    "!document.querySelector({})",
                    serde_json::to_string(selector).unwrap_or_default()
                )
            }
            "hidden" => {
                format!(
                    r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) return true;
                const s = window.getComputedStyle(el);
                return s.display === 'none' || s.visibility === 'hidden' || parseFloat(s.opacity) === 0;
            }})()"#,
                    sel = serde_json::to_string(selector).unwrap_or_default()
                )
            }
            _ => {
                format!(
                    r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) return false;
                const r = el.getBoundingClientRect();
                const s = window.getComputedStyle(el);
                return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none';
            }})()"#,
                    sel = serde_json::to_string(selector).unwrap_or_default()
                )
            }
        };
        poll_until_true(client, session_id, &check_fn, timeout_ms).await
    }
    pub(crate) async fn wait_for_url(
        client: &super::super::cdp::client::CdpClient,
        session_id: &str,
        pattern: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let check_fn = format!(
            "location.href.includes({})",
            serde_json::to_string(pattern).unwrap_or_default()
        );
        poll_until_true(client, session_id, &check_fn, timeout_ms).await
    }
    pub(crate) async fn wait_for_text(
        client: &super::super::cdp::client::CdpClient,
        session_id: &str,
        text: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let check_fn = format!(
            "(document.body.innerText || '').includes({})",
            serde_json::to_string(text).unwrap_or_default()
        );
        poll_until_true(client, session_id, &check_fn, timeout_ms).await
    }
    pub(crate) async fn wait_for_function(
        client: &super::super::cdp::client::CdpClient,
        session_id: &str,
        fn_str: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let check_fn = format!("!!({})", fn_str);
        poll_until_true(client, session_id, &check_fn, timeout_ms).await
    }
    pub(crate) async fn poll_until_true(
        client: &super::super::cdp::client::CdpClient,
        session_id: &str,
        expression: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
        loop {
            let result: super::super::cdp::types::EvaluateResult = client
                .send_command_typed(
                    "Runtime.evaluate",
                    &super::super::cdp::types::EvaluateParams {
                        expression: expression.to_string(),
                        return_by_value: Some(true),
                        await_promise: Some(true),
                    },
                    Some(session_id),
                )
                .await?;
            if result
                .result
                .value
                .as_ref()
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("Wait timed out after {}ms", timeout_ms));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}
pub(crate) use action_commands::*;
