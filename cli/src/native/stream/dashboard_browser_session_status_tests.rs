use super::*;
use serde_json::json;

#[test]
fn dashboard_cli_fallback_service_status_attaches_browser_session_state() {
    let body = json!({
        "success": true,
        "data": {
            "service_state": {
                "browsers": {
                    "legacy": { "id": "legacy" }
                }
            }
        }
    })
    .to_string();
    let snapshot = json!({
        "schemaVersion": "agent-browser.browser-session-state.v1",
        "browsers": {
            "browser:profile:operation:1": {
                "id": "browser:profile:operation:1"
            }
        }
    });

    let joined = attach_browser_session_state_to_dashboard_cli_fallback(body, Ok(snapshot));
    let value: Value = serde_json::from_str(&joined).unwrap();

    assert_eq!(
        value["data"]["service_state"]["browsers"]["legacy"]["id"],
        "legacy"
    );
    assert_eq!(
        value["data"]["browserSessionState"]["browsers"]["browser:profile:operation:1"]["id"],
        "browser:profile:operation:1"
    );
}

#[test]
fn dashboard_cli_fallback_service_status_reports_nonblocking_snapshot_failure() {
    let body = json!({ "success": true, "data": {} }).to_string();

    let joined = attach_browser_session_state_to_dashboard_cli_fallback(
        body,
        Err("browser-session-state unreadable".to_string()),
    );
    let value: Value = serde_json::from_str(&joined).unwrap();

    assert_eq!(value["success"], true);
    assert!(value["data"]["browserSessionState"].is_null());
    assert_eq!(
        value["data"]["browserSessionStateError"],
        "browser-session-state unreadable"
    );
}

#[test]
fn dashboard_cli_fallback_service_status_preserves_unparseable_output() {
    let body = "not-json".to_string();

    assert_eq!(
        attach_browser_session_state_to_dashboard_cli_fallback(
            body.clone(),
            Ok(json!({ "schemaVersion": "ignored" })),
        ),
        body
    );
}
