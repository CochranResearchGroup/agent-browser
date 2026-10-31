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

pub(super) fn public_handoff_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/remote-view/")?;
    (!id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
    .then_some(id)
}

/// Private gateway callback, never a browser authentication alternative.
pub(super) fn native_resume_id(path: &str) -> Option<&str> {
    let id = path
        .strip_prefix("/api/native-remote-view/")?
        .strip_suffix("/resume")?;
    (!id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')))
    .then_some(id)
}

/// Authenticate only the narrowly configured native gateway capability. Native
/// authentication happens at Remote View; no cookie or caller-selected target
/// reaches browser recovery through this callback.
pub(super) fn native_resume_operator(
    peer: std::net::IpAddr,
    headers: &[(String, String)],
) -> Result<String, String> {
    if !peer.is_loopback()
        || headers.iter().any(|(name, _)| {
            name.eq_ignore_ascii_case("cookie") || name.eq_ignore_ascii_case("transfer-encoding")
        })
    {
        return Err("remote_view_native_callback_denied".into());
    }
    let one = |name: &str| -> Result<&str, String> {
        let mut values = headers
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name));
        let value = values
            .next()
            .map(|(_, value)| value.trim())
            .ok_or("remote_view_native_callback_denied")?;
        if values.next().is_some() {
            return Err("remote_view_native_callback_denied".into());
        }
        Ok(value)
    };
    if one("content-length")? != "0" {
        return Err("remote_view_native_callback_denied".into());
    }
    let supplied = one("authorization")?
        .strip_prefix("Bearer ")
        .ok_or("remote_view_native_callback_denied")?;
    let path = std::env::var_os("AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("remote_view_native_callback_unconfigured")?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(&path)
        .map_err(|_| "remote_view_native_callback_unconfigured")?;
    let metadata = file
        .metadata()
        .map_err(|_| "remote_view_native_callback_unconfigured")?;
    if !metadata.is_file() || metadata.len() > 257 {
        return Err("remote_view_native_callback_unconfigured".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
            return Err("remote_view_native_callback_unconfigured".into());
        }
    }
    use std::io::Read;
    let mut token = String::new();
    file.take(258)
        .read_to_string(&mut token)
        .map_err(|_| "remote_view_native_callback_unconfigured")?;
    let token = token.strip_suffix('\n').unwrap_or(&token);
    if !(32..=256).contains(&token.len()) || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("remote_view_native_callback_unconfigured".into());
    }
    let different = token
        .as_bytes()
        .iter()
        .zip(supplied.as_bytes())
        .fold(0u8, |different, (a, b)| different | (a ^ b));
    if token.len() != supplied.len() || different != 0 {
        return Err("remote_view_native_callback_denied".into());
    }
    let operator = one("x-remote-view-operator")?;
    if operator.is_empty() || operator.len() > 128 || operator.chars().any(char::is_control) {
        return Err("remote_view_native_callback_denied".into());
    }
    Ok(operator.to_owned())
}

fn native_continuation_location(origin: &url::Url, id: &str) -> Result<String, String> {
    let state = crate::native::browser_session_store::BrowserSessionSqliteStore::default_sqlite()?
        .load_session_state()?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "remote_view_clock_invalid")?
        .as_millis() as u64;
    agent_browser_service_model::remote_view_handoff_resume_intent(&state, id, now_ms)?;
    origin
        .join(&format!("/handoff/agent-browser/{id}"))
        .map(|url| url.to_string())
        .map_err(|_| "remote_view_presentation_path_invalid".into())
}

/// Validate the public link without launching, recovering or focusing a browser.
/// The destination's native Remote View authentication protects desktop access.
/// Resolve without effects. Embedded callers use Remote View's existing
/// same-origin workspace mode; this never grants access or weakens its CSP.
pub(super) async fn resolve_public_location(id: &str, workspace: bool) -> Result<String, String> {
    let configured = std::env::var("AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN")
        .map_err(|_| "remote_view_public_origin_missing")?;
    let origin = public_origin(&configured)?;
    let id = id.to_string();
    if !workspace {
        // A live browser may show a different tab, or exit while native login
        // is pending. Select the addressed target only after native auth.
        return tokio::task::spawn_blocking(move || native_continuation_location(&origin, &id))
            .await
            .map_err(|_| "remote_view_presentation_join_failed")?;
    }
    let handoff = id.clone();
    let presentation = tokio::task::spawn_blocking(move || {
        crate::native::browser_session_remote_view::resolve_handoff_desktop_presentation(&handoff)
    })
    .await
    .map_err(|_| "remote_view_presentation_join_failed")?;
    let (route, correlation) = match presentation {
        Ok(presentation) => presentation,
        Err(reason)
            if matches!(
                reason.as_str(),
                "remote_view_tab_handoff_session_unavailable"
                    | "remote_view_tab_handoff_tab_unavailable"
                    | "remote_view_tab_handoff_browser_unavailable"
                    | "remote_view_human_use_target_changed"
            ) =>
        {
            // Lost processes and changed physical observations require the
            // authenticated owner continuation, not a stale desktop or a stuck
            // link. Validate retained intent without starting its owner here.
            return native_continuation_location(&origin, &id);
        }
        Err(reason) => return Err(reason),
    };
    let mut location = url::Url::parse(&presentation_url(&origin, &route)?)
        .map_err(|_| "remote_view_presentation_path_invalid")?;
    if workspace {
        location.query_pairs_mut().append_pair("workspace", "true");
    }
    if let Some(correlation) = correlation {
        location
            .query_pairs_mut()
            .append_pair("handoff-observation", &correlation);
    }
    // Workspace CSP allows only its own origin. The dashboard origin routes
    // these native paths through the existing Remote View gateway; standalone
    // handoffs above continue to the configured native public origin.
    Ok(location[url::Position::BeforePath..].to_owned())
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

/// Prepare a correlation for authenticated document navigation. A dashboard
/// redirect loses transient Fetch-User activation; the native viewer's private
/// connection and frame evidence remains the authority for successful use.
/// Navigation cannot acknowledge use; the private provider read supplies
/// successful-use facts.
pub(super) fn human_use_registration(
    id: &str,
    operator: Option<&str>,
    headers: &[(String, String)],
    ready: &serde_json::Value,
) -> Option<serde_json::Value> {
    operator.filter(|name| !name.trim().is_empty())?;
    let has = |name: &str, value: &str| {
        headers
            .iter()
            .any(|(key, observed)| key.eq_ignore_ascii_case(name) && observed.trim() == value)
    };
    let prefetch = headers.iter().any(|(key, value)| {
        (key.eq_ignore_ascii_case("purpose") || key.eq_ignore_ascii_case("sec-purpose"))
            && value.to_ascii_lowercase().contains("prefetch")
    });
    if prefetch
        || !has("sec-fetch-mode", "navigate")
        || !has("sec-fetch-dest", "document")
        || ready["success"] != true
        || ready["data"]["operatorVisible"]["state"] != "ready"
        || ready["data"]["handoffId"] != id
    {
        return None;
    }
    let tab_id = ready["data"]["tabId"]
        .as_str()
        .filter(|value| !value.is_empty())?;
    let target_id = ready["data"]["targetId"]
        .as_str()
        .filter(|value| !value.is_empty())?;
    Some(
        serde_json::json!({"action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"prepare_native_use",
        "expectedTabId":tab_id, "expectedTargetId":target_id}),
    )
}

/// Restore the retained browser through its owner before redirecting to the native desktop.
/// Native authenticated callbacks and the existing dashboard compatibility API
/// share this owner path; no temporary viewer grant is issued.
pub(super) async fn resolve_location(
    id: &str,
    operator: Option<&str>,
    headers: &[(String, String)],
) -> Result<String, String> {
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
        let intent = agent_browser_service_model::remote_view_handoff_resume_intent(
            &state,
            &handoff,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "remote_view_clock_invalid")?
                .as_millis() as u64,
        )?;
        Ok::<_, String>(intent.session_name)
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
    let retained_id = id.to_string();
    let route = tokio::task::spawn_blocking(move || resolve_handoff_desktop_route(&retained_id))
        .await
        .map_err(|_| "remote_view_presentation_join_failed")??;
    let mut location = presentation_url(&origin, &route)?;
    if let Some(registration) = human_use_registration(id, operator, headers, &response) {
        let prepared = tokio::time::timeout(
            std::time::Duration::from_secs(90),
            super::http::relay_command_to_daemon(&session, &registration.to_string()),
        )
        .await
        .map_err(|_| "remote_view_native_registration_timeout")??;
        let prepared: serde_json::Value = serde_json::from_str(&prepared)
            .map_err(|_| "remote_view_native_registration_response_invalid")?;
        if prepared["success"] != true {
            return Err(prepared["error"]
                .as_str()
                .unwrap_or("remote_view_native_registration_failed")
                .to_string());
        }
        let correlation = prepared["data"]["correlationId"]
            .as_str()
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or("remote_view_native_registration_response_invalid")?;
        if prepared["data"]["tabId"] != response["data"]["tabId"]
            || prepared["data"]["targetId"] != response["data"]["targetId"]
        {
            return Err("remote_view_human_use_target_changed".into());
        }
        let mut url =
            url::Url::parse(&location).map_err(|_| "remote_view_presentation_path_invalid")?;
        url.query_pairs_mut()
            .append_pair("handoff-observation", correlation);
        location = url.to_string();
        let generation = crate::dashboard_ingress::native_journey::selected_generation();
        poll_native_handoff_use(session, id.to_string(), correlation.to_string(), generation);
    }
    Ok(location)
}

/// Foreground continuation reads the private owner until use or expiry, with one
/// task per exact correlation. Restart recovery also reads durable observations
/// from the existing session reaper; no browser-supplied success is accepted.
fn poll_native_handoff_use(
    session: String,
    id: String,
    correlation: String,
    generation: Option<String>,
) {
    static ACTIVE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    let Ok(mut active) = ACTIVE.lock() else {
        return;
    };
    if !active.insert(correlation.clone()) {
        return;
    }
    drop(active);
    tokio::spawn(async move {
        struct PollGuard(String);
        impl Drop for PollGuard {
            fn drop(&mut self) {
                if let Ok(mut active) = ACTIVE.lock() {
                    active.remove(&self.0);
                }
            }
        }
        let _guard = PollGuard(correlation.clone());
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(300);
        let command = serde_json::json!({"action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"observe_native_use"}).to_string();
        while tokio::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let Ok(Ok(reply)) = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                super::http::relay_command_to_daemon(&session, &command),
            )
            .await
            else {
                break;
            };
            let Ok(reply) = serde_json::from_str::<serde_json::Value>(&reply) else {
                break;
            };
            if reply["success"] == true && reply["data"]["status"] == "native_use_pending" {
                continue;
            }
            if native_journey_confirmed(&reply) {
                if let Some(generation) = generation {
                    let result = tokio::task::spawn_blocking(move || {
                        crate::dashboard_ingress::native_journey::record(
                            &generation,
                            &id,
                            &correlation,
                        )
                    })
                    .await;
                    if !matches!(result, Ok(Ok(()))) {
                        eprintln!("Native operator-journey receipt recording failed; inspect workstation status");
                    }
                }
            }
            break;
        }
    });
}

/// Owner confirmation triggers a private evidence read, never caller-supplied success.
fn native_journey_confirmed(reply: &serde_json::Value) -> bool {
    reply["success"] == true
        && matches!(
            reply["data"]["status"].as_str(),
            Some("human_use_confirmed" | "human_use_already_confirmed")
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_journey_requires_successful_owner_use_before_private_recording() {
        for status in [
            "native_use_pending",
            "native_use_expired",
            "ready",
            "human_use_confirmed",
            "human_use_already_confirmed",
        ] {
            for success in [true, false] {
                let reply = serde_json::json!({"success":success,"data":{"status":status}});
                assert_eq!(
                    native_journey_confirmed(&reply),
                    success && status.starts_with("human_use_")
                );
            }
        }
        assert!(!native_journey_confirmed(
            &serde_json::json!({"success":true,"data":{"operatorVisible":{"state":"ready"}}})
        ));
    }

    #[test]
    fn authenticated_user_navigation_registers_exact_target_without_acknowledging_use() {
        let headers = vec![
            ("Sec-Fetch-User".into(), "?1".into()),
            ("Sec-Fetch-Mode".into(), "navigate".into()),
            ("Sec-Fetch-Dest".into(), "document".into()),
        ];
        let ready = serde_json::json!({"success":true,"data":{"handoffId":"handoff-a",
            "tabId":"tab-a","targetId":"target-a","operatorVisible":{"state":"ready"}}});
        let acknowledgement =
            human_use_registration("handoff-a", Some("synthetic-operator"), &headers, &ready)
                .expect("authenticated user navigation after readiness must register correlation");
        assert_eq!(acknowledgement["handoffOperation"], "prepare_native_use");
        assert_eq!(acknowledgement["handoffId"], "handoff-a");
        assert_eq!(acknowledgement["expectedTabId"], "tab-a");
        assert_eq!(acknowledgement["expectedTargetId"], "target-a");
        assert!(human_use_registration("handoff-a", None, &headers, &ready).is_none());
        assert!(
            human_use_registration("handoff-a", Some("synthetic-operator"), &[], &ready).is_none()
        );
        let mut prefetch = headers.clone();
        prefetch.push(("Sec-Purpose".into(), "prefetch".into()));
        assert!(
            human_use_registration("handoff-a", Some("synthetic-operator"), &prefetch, &ready)
                .is_none()
        );
        let mut pending = ready.clone();
        pending["data"]["operatorVisible"]["state"] = "pending".into();
        assert!(human_use_registration(
            "handoff-a",
            Some("synthetic-operator"),
            &headers,
            &pending
        )
        .is_none());
    }

    #[test]
    fn authenticated_dashboard_redirect_registers_without_transient_user_activation() {
        // Captured from the real dashboard after a trusted opaque-link click:
        // its React redirect remains a document navigation but loses Fetch-User.
        let headers = vec![
            ("Sec-Fetch-Site".into(), "same-origin".into()),
            ("Sec-Fetch-Mode".into(), "navigate".into()),
            ("Sec-Fetch-Dest".into(), "document".into()),
        ];
        let ready = serde_json::json!({"success":true,"data":{
            "handoffId":"handoff-a","tabId":"tab-a","targetId":"target-a",
            "operatorVisible":{"state":"ready"}
        }});
        let registration =
            human_use_registration("handoff-a", Some("synthetic-operator"), &headers, &ready)
                .expect("actual opaque-link navigation must register native viewer observation");
        assert_eq!(registration["handoffOperation"], "prepare_native_use");
        assert!(registration.get("humanUseEvidence").is_none());
        assert!(human_use_registration("handoff-a", None, &headers, &ready).is_none());
        let mut prefetch = headers;
        prefetch.push(("Sec-Purpose".into(), "prefetch".into()));
        assert!(
            human_use_registration("handoff-a", Some("synthetic-operator"), &prefetch, &ready)
                .is_none()
        );
    }

    #[test]
    fn authenticated_navigation_without_native_viewer_use_cannot_acknowledge_first_use() {
        let headers = vec![
            ("Sec-Fetch-User".into(), "?1".into()),
            ("Sec-Fetch-Mode".into(), "navigate".into()),
            ("Sec-Fetch-Dest".into(), "document".into()),
        ];
        let ready = serde_json::json!({"success":true,"data":{
            "handoffId":"handoff-a","tabId":"tab-a","targetId":"target-a",
            "operatorVisible":{"state":"ready"}
        }});
        let registration =
            human_use_registration("handoff-a", Some("synthetic-operator"), &headers, &ready)
                .unwrap();
        assert_ne!(
            registration["handoffOperation"], "confirm_human_use",
            "authenticated navigation and target readiness alone acknowledged native viewer use"
        );
        assert!(registration.get("humanUseEvidence").is_none());
    }

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
