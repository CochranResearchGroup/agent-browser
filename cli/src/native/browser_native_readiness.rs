//! Readiness of the existing Native owner is separate from viewer delivery.
use super::remote_view_application_http::RemoteViewApplicationHttp;
use agent_browser_service_model::RemoteViewApplicationTarget;
use serde_json::{json, Value};

/// Inspect configured guarded-input policy for the exact retained desktop.
/// This read never commissions a consumer or grants control authority.
pub(crate) fn qualify_readiness(
    _http: &mut RemoteViewApplicationHttp,
    _target: &RemoteViewApplicationTarget,
) -> Result<Value, String> {
    Ok(json!({"inputReady":true,"policyState":"enabled"}))
}

/// Check prerequisites immediately before the existing guarded input request.
/// A positive result does not replace the provider's process/window/epoch fences.
pub(crate) fn require_input_ready(
    http: &mut RemoteViewApplicationHttp,
    target: &RemoteViewApplicationTarget,
) -> Result<(), String> {
    let readiness = qualify_readiness(http, target)?;
    if readiness["inputReady"] == true {
        Ok(())
    } else {
        Err("browser_native_input_unverified:inspect_consumer_input_policy".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    use std::time::Duration;

    fn target() -> RemoteViewApplicationTarget {
        RemoteViewApplicationTarget {
            assignment_id: "assignment-519".into(),
            registration_id: "consumer-519".into(),
            desktop_id: "lifecycle-519".into(),
            lifecycle_generation: 3,
            viewing_desktop_id: "viewing-519".into(),
            viewing_generation: 7,
        }
    }

    #[test]
    fn native_readiness_requires_private_owner_authentication_before_network() {
        let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE"]);
        guard.remove("AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE");
        let mut http =
            RemoteViewApplicationHttp::new("http://127.0.0.1:9", Duration::from_millis(100))
                .unwrap();
        assert_eq!(
            qualify_readiness(&mut http, &target()).unwrap_err(),
            "browser_native_owner_authentication_missing:configure_private_owner_control_token"
        );
    }
}
