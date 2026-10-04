//! Authenticated, click-driven top-level presentation for logical tab handoffs.

use agent_browser_service_model::RemoteViewApplicationViewIssuance;
use std::future::Future;

async fn prepare_then_read<P, R, F>(
    prepare: P,
    read: R,
) -> Result<RemoteViewApplicationViewIssuance, String>
where
    P: Future<Output = Result<(), String>>,
    R: FnOnce() -> F,
    F: Future<Output = Result<RemoteViewApplicationViewIssuance, String>>,
{
    prepare.await?;
    read().await
}

/// Refresh through the existing authenticated owner command path. The dashboard
/// must not mutate the browser-session aggregate or launch a replacement tab.
async fn prepare_owner_view(id: &str, operator: &str) -> Result<(), String> {
    use super::http::{
        ensure_service_daemon_session, load_service_state, relay_command_to_daemon,
        service_request_command_with_dashboard_generation, service_request_relay_session,
    };
    let default_session = "dashboard-service-backend";
    let body = serde_json::json!({
        "action": "service_remote_view_handoff_resolve",
        "serviceName": "agent-browser-dashboard",
        "agentName": operator,
        "taskName": "durable-remote-view-presentation",
        "params": { "handoffId": id, "allowReopenClosed": false },
        "serviceStateLockTimeoutMs": 30000,
        "jobTimeoutMs": 90000,
    })
    .to_string();
    let state = load_service_state();
    let command = service_request_command_with_dashboard_generation(
        &body,
        Some(&state),
        operator,
        default_session,
        std::env::var("AGENT_BROWSER_DASHBOARD_GENERATION")
            .ok()
            .as_deref(),
    )
    .map_err(|_| "remote_view_presentation_owner_request_invalid")?;
    let session = service_request_relay_session(default_session, &body, &command);
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        ensure_service_daemon_session(&session, Some(&command)).await?;
        let response = relay_command_to_daemon(&session, &command.to_string()).await?;
        let response: serde_json::Value = serde_json::from_str(&response)
            .map_err(|_| "remote_view_presentation_owner_response_invalid")?;
        if response["success"].as_bool() != Some(true) {
            return Err("remote_view_presentation_owner_resolve_failed".into());
        }
        Ok(())
    })
    .await
    .map_err(|_| "remote_view_presentation_owner_timeout")?
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

use crate::native::browser_session_remote_view::public_origin;

fn presentation_url(
    origin: &url::Url,
    view: &RemoteViewApplicationViewIssuance,
) -> Result<String, String> {
    if view.grant.request.audience != "remote_view"
        || view.path != format!("/view/{}", view.grant.route_id)
        || !view.grant.has_valid_route_id()
    {
        return Err("remote_view_presentation_path_invalid".into());
    }
    origin
        .join(&view.path)
        .map(|url| url.to_string())
        .map_err(|_| "remote_view_presentation_path_invalid".into())
}

/// The caller must authenticate the dashboard request before invoking this.
/// Configuration is checked before any provider request. No raw request URL
/// selects the external origin, assignment, tab, or provider route.
pub(super) async fn resolve_location(id: &str, operator: &str) -> Result<String, String> {
    let configured = std::env::var("AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN")
        .map_err(|_| "remote_view_public_origin_missing")?;
    let origin = public_origin(&configured)?;
    let published_id = id.to_string();
    let view = prepare_then_read(prepare_owner_view(id, operator), || async move {
        tokio::task::spawn_blocking(move || {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "remote_view_clock_unavailable")?
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            let view = super::super::browser_session_remote_view::resolve_published_handoff_view(
                &published_id,
                now_ms,
            )?;
            Ok(view)
        })
        .await
        .map_err(|_| "remote_view_presentation_join_failed")?
    })
    .await?;
    presentation_url(&origin, &view)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn presentation_prepares_owner_before_reading_expired_publication() {
        let refreshed = std::cell::Cell::new(false);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let issuance: RemoteViewApplicationViewIssuance =
            serde_json::from_value(fixture["viewIssuance"].clone()).unwrap();
        let result = prepare_then_read(
            async {
                refreshed.set(true);
                Ok(())
            },
            || async {
                if !refreshed.get() {
                    return Err("remote_view_tab_view_expired".into());
                }
                Ok(issuance.clone())
            },
        )
        .await;
        assert_eq!(result.unwrap(), issuance);
    }

    #[test]
    fn presentation_origin_and_path_cannot_be_selected_by_handoff_input() {
        assert_eq!(
            handoff_id("/api/remote-view/opaque-a/presentation"),
            Some("opaque-a")
        );
        for path in [
            "/api/remote-view/../presentation",
            "/api/remote-view/a/b/presentation",
            "/api/remote-view/a%2fb/presentation",
            "/api/remote-view//presentation",
        ] {
            assert!(handoff_id(path).is_none());
        }
        for origin in [
            "http://view.example",
            "https://user:secret@view.example",
            "https://view.example/path",
            "https://view.example/?token=x",
            "https://view.example/#x",
            "https://127.0.0.1",
            "https://[::1]",
            "https://localhost",
        ] {
            assert!(public_origin(origin).is_err());
        }
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let mut view: RemoteViewApplicationViewIssuance =
            serde_json::from_value(fixture["viewIssuance"].clone()).unwrap();
        let origin = public_origin("https://view.example").unwrap();
        assert_eq!(
            presentation_url(&origin, &view).unwrap(),
            "https://view.example/view/11111111-1111-4111-8111-111111111111"
        );
        view.grant.route_id = "65ce69abea0c3-1bf6ba08f07234c67f9f17826feb9594".into();
        view.path = format!("/view/{}", view.grant.route_id);
        assert_eq!(
            presentation_url(&origin, &view).unwrap(),
            "https://view.example/view/65ce69abea0c3-1bf6ba08f07234c67f9f17826feb9594"
        );
        view.path = "//foreign.example/".into();
        assert!(presentation_url(&origin, &view).is_err());
    }
}
