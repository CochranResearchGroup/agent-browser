//! Authenticated deep links to Remote View's current owned desktop route.

use crate::native::browser_session_remote_view::{public_origin, resolve_handoff_desktop_route};

/// Return actionable terminal link failures without exposing internal identities
/// or presenting a closed task as a transient provider outage.
pub(super) fn failure_response(reason: &str, html: bool) -> String {
    let (status, error, message) = match reason {
        "remote_view_handoff_expired" => (
            "410 Gone",
            "remote_view_handoff_expired",
            "This handoff link expired. Ask the task owner for a current link.",
        ),
        "remote_view_tab_handoff_session_unavailable"
        | "remote_view_tab_handoff_tab_unavailable"
        | "remote_view_tab_handoff_browser_unavailable" => (
            "410 Gone",
            "remote_view_handoff_target_closed",
            "The browser task behind this link is no longer available. Start a new task and use its new handoff link.",
        ),
        "remote_view_tab_handoff_missing" => (
            "404 Not Found",
            "remote_view_handoff_not_found",
            "This handoff link was not found. Check the link with the task owner.",
        ),
        _ => (
            "503 Service Unavailable",
            "remote_view_presentation_unavailable",
            "The desktop is unavailable right now. Keep this link and ask the task owner to check browser readiness before reconnecting.",
        ),
    };
    let (content_type, body) = if html {
        // Only the fixed messages above enter this page, never the raw reason.
        ("text/html; charset=utf-8", format!("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Browser handoff</title><body><main><h1>Browser handoff</h1><p>{message}</p></main></body></html>"))
    } else {
        (
            "application/json",
            serde_json::json!({"success":false,"error":error,"message":message}).to_string(),
        )
    };
    format!("HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

pub(super) fn handoff_id(path: &str) -> Option<&str> {
    let id = path
        .strip_prefix("/api/remote-view/")?
        .strip_suffix("/presentation")?;
    (!id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
    .then_some(id)
}

fn presentation_url(origin: &url::Url, route: &str) -> Result<String, String> {
    let slot = route
        .strip_prefix('/')
        .and_then(|slot| slot.parse::<u32>().ok())
        .filter(|slot| *slot > 0);
    if slot.is_none() || route != format!("/{}", slot.unwrap()) {
        return Err("remote_view_presentation_path_invalid".into());
    }
    origin
        .join(route)
        .map(|url| url.to_string())
        .map_err(|_| "remote_view_presentation_path_invalid".into())
}

/// Restore the retained browser through its owner before redirecting to the native desktop.
/// Existing dashboard authentication applies; no temporary viewer grant is issued.
pub(super) async fn resolve_location(id: &str, _operator: &str) -> Result<String, String> {
    let configured = std::env::var("AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN")
        .map_err(|_| "remote_view_public_origin_missing")?;
    let origin = public_origin(&configured)?;
    let handoff = id.to_string();
    let session = tokio::task::spawn_blocking(move || {
        let state =
            crate::native::browser_session_store::BrowserSessionSqliteStore::default_sqlite()?
                .load_session_state()?;
        state
            .remote_view_tab_handoffs
            .get(&handoff)
            .ok_or("remote_view_tab_handoff_missing")?
            .check_link_at(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| "remote_view_clock_invalid")?
                    .as_millis() as u64,
            )?;
        let target =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&state, &handoff)?;
        Ok::<_, String>(target.session.name)
    })
    .await
    .map_err(|_| "remote_view_presentation_join_failed")??;
    let command = serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id, "nativeDesktop":true,
    });
    super::http::ensure_service_daemon_session(&session, Some(&command)).await?;
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        super::http::relay_command_to_daemon(&session, &command.to_string()),
    )
    .await
    .map_err(|_| "remote_view_browser_recovery_timeout")??;
    let response: serde_json::Value = serde_json::from_str(&response)
        .map_err(|_| "remote_view_browser_recovery_response_invalid")?;
    if response["success"] != true || response["data"]["operatorVisible"]["state"] != "ready" {
        return Err(response["error"]
            .as_str()
            .unwrap_or("remote_view_browser_recovery_pending")
            .to_string());
    }
    let id = id.to_string();
    let route = tokio::task::spawn_blocking(move || resolve_handoff_desktop_route(&id))
        .await
        .map_err(|_| "remote_view_presentation_join_failed")??;
    presentation_url(&origin, &route)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_handoff_failures_are_actionable_and_never_echo_private_reasons() {
        for reason in [
            "remote_view_tab_handoff_session_unavailable",
            "remote_view_tab_handoff_tab_unavailable",
            "remote_view_tab_handoff_browser_unavailable",
        ] {
            let json = failure_response(reason, false);
            assert!(json.starts_with("HTTP/1.1 410 Gone\r\n"));
            assert!(json.contains("remote_view_handoff_target_closed"));
            assert!(json.contains("Start a new task"));
            assert!(!json.contains(reason));
            let html = failure_response(reason, true);
            assert!(html.contains("Content-Type: text/html; charset=utf-8"));
            assert!(html.contains("Start a new task"));
            assert!(!html.contains("remote_view_"));
        }
        assert!(
            failure_response("remote_view_handoff_expired", false).starts_with("HTTP/1.1 410 Gone")
        );
        assert!(failure_response("remote_view_tab_handoff_missing", false)
            .starts_with("HTTP/1.1 404 Not Found"));
        let private_reason = "transport_failed:/private/profile?token=secret";
        for html in [false, true] {
            let response = failure_response(private_reason, html);
            assert!(response.starts_with("HTTP/1.1 503 Service Unavailable"));
            assert!(!response.contains(private_reason));
            assert!(response.contains("Keep this link"));
        }
    }

    #[test]
    fn presentation_rejects_foreign_routes_and_uses_native_desktop_path() {
        let origin = public_origin("https://view.example").unwrap();
        assert_eq!(
            presentation_url(&origin, "/1").unwrap(),
            "https://view.example/1"
        );
        for route in [
            "//foreign.example/",
            "/view/grant",
            "/0",
            "/01",
            "/1?token=x",
            "/1#x",
            "/../1",
        ] {
            assert!(presentation_url(&origin, route).is_err());
        }
        assert_eq!(
            handoff_id("/api/remote-view/opaque-a/presentation"),
            Some("opaque-a")
        );
        for path in [
            "/api/remote-view/../presentation",
            "/api/remote-view/a/b/presentation",
            "/api/remote-view/a%2fb/presentation",
        ] {
            assert!(handoff_id(path).is_none());
        }
    }
}
