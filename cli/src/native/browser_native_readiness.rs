//! Readiness of the existing Native owner is separate from viewer delivery.
use super::remote_view_application_http::RemoteViewApplicationHttp;
use agent_browser_service_model::{
    RemoteViewApplicationTarget, RemoteViewApplicationTransportError,
};
use serde_json::{json, Value};

/// Inspect configured guarded-input policy for the exact retained desktop.
/// This read never commissions a consumer or grants control authority.
pub(crate) fn qualify_readiness(
    http: &mut RemoteViewApplicationHttp,
    target: &RemoteViewApplicationTarget,
) -> Result<Value, String> {
    if !http.has_control_authorization() {
        return Err(
            "browser_native_owner_authentication_missing:configure_private_owner_control_token"
                .into(),
        );
    }
    let response = http.desktop_input_capability(target).map_err(|error| {
        match error {
            RemoteViewApplicationTransportError::OwnerAuthenticationRequired => {
                "browser_native_owner_authentication_rejected:verify_private_owner_control_token".into()
            }
            RemoteViewApplicationTransportError::OwnerAccessDenied => {
                "browser_native_owner_access_denied:verify_owner_control_access".into()
            }
            RemoteViewApplicationTransportError::OwnerInputDisabled => {
                "browser_native_input_disabled:configure_consumer_selective_input".into()
            }
            _ => format!(
                "browser_native_input_inspection_unavailable:{error:?}:inspect_consumer_input_policy"
            ),
        }
    })?;
    // Lifecycle and viewing generations are independent. The caller retains its
    // joined assignment; final Native input still fences the viewing generation.
    if response.pointer("/desktop/desktopId") != Some(&json!(target.desktop_id))
        || response.pointer("/desktop/generation") != Some(&json!(target.lifecycle_generation))
    {
        return Ok(projection(
            false,
            "unverified",
            "native_input_target_changed",
            "refresh_owned_task",
        ));
    }
    if response["schemaVersion"] != 1
        || response.pointer("/desktop/state").and_then(Value::as_str) != Some("local_ready")
    {
        return Ok(projection(
            false,
            "unverified",
            "native_input_policy_unverified",
            "inspect_consumer_input_policy",
        ));
    }
    // Provider descriptions can contain arbitrary text. Project only the known
    // policy enum into local reason/action literals; never relay private prose.
    Ok(
        match response
            .pointer("/inputCapability/policyState")
            .and_then(Value::as_str)
        {
            Some("enabled") => projection(
                true,
                "enabled",
                "guarded_native_input_enabled",
                "read_desktop_control_status",
            ),
            Some("disabled") => projection(
                false,
                "disabled",
                "selective_control_unavailable",
                "configure_consumer_selective_input",
            ),
            _ => projection(
                false,
                "unverified",
                "native_input_policy_unverified",
                "inspect_consumer_input_policy",
            ),
        },
    )
}

fn projection(input_ready: bool, policy: &str, reason: &str, next_action: &str) -> Value {
    json!({
        "ownerAuthentication":"authenticated",
        "inputReady":input_ready,
        "policyState":policy,
        "reasonCode":reason,
        "nextAction":next_action,
        "viewerReadiness":"not_checked",
        "controlAuthority":"not_granted"
    })
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
        Err(match readiness["reasonCode"].as_str() {
            Some("selective_control_unavailable") => {
                "browser_native_input_disabled:configure_consumer_selective_input"
            }
            Some("native_input_target_changed") => {
                "browser_native_input_target_changed:refresh_owned_task"
            }
            _ => "browser_native_input_unverified:inspect_consumer_input_policy",
        }
        .into())
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
            desktop_id: "11111111-1111-4111-8111-111111111519".into(),
            lifecycle_generation: 3,
            viewing_desktop_id: "22222222-2222-4222-8222-222222222519".into(),
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
