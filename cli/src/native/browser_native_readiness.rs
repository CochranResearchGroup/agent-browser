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

    struct OwnerFixture {
        http: RemoteViewApplicationHttp,
        server: std::thread::JoinHandle<()>,
        token_path: std::path::PathBuf,
        _guard: EnvGuard<'static>,
    }

    impl OwnerFixture {
        fn new(response: Value, status: &'static str, requests: usize) -> Self {
            use std::io::{Read, Write};
            let guard = EnvGuard::new(&["AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE"]);
            let token_path = std::env::temp_dir()
                .join(format!("native-readiness-token-{}", uuid::Uuid::new_v4()));
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options
                .open(&token_path)
                .unwrap()
                .write_all(b"fixture-native-readiness-token-519\n")
                .unwrap();
            guard.set(
                "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
                token_path.to_str().unwrap(),
            );
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let origin = format!("http://{}", listener.local_addr().unwrap());
            let expected_path = format!("GET /v1/desktops/{} HTTP/1.1", target().desktop_id);
            let server = std::thread::spawn(move || {
                for _ in 0..requests {
                    let deadline = std::time::Instant::now() + Duration::from_secs(3);
                    let mut stream = loop {
                        match listener.accept() {
                            Ok((stream, _)) => break stream,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                assert!(
                                    std::time::Instant::now() < deadline,
                                    "owner inspect was not requested"
                                );
                                std::thread::sleep(Duration::from_millis(5));
                            }
                            Err(error) => panic!("synthetic owner accept failed: {error}"),
                        }
                    };
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut request = Vec::new();
                    loop {
                        let mut chunk = [0; 1024];
                        let count = stream.read(&mut chunk).unwrap();
                        assert!(count > 0, "incomplete owner HTTP request");
                        request.extend_from_slice(&chunk[..count]);
                        if request.windows(4).any(|part| part == b"\r\n\r\n") {
                            break;
                        }
                        assert!(request.len() <= 8192, "unexpected synthetic request size");
                    }
                    let request = String::from_utf8(request).unwrap();
                    assert!(
                        request.starts_with(&expected_path),
                        "readiness must inspect the lifecycle desktop without dispatching input"
                    );
                    assert!(
                        request.lines().any(|line| line
                            .to_ascii_lowercase()
                            .starts_with("authorization: bearer ")),
                        "private owner authentication missing"
                    );
                    let body = serde_json::to_string(&response).unwrap();
                    write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                }
            });
            let http = RemoteViewApplicationHttp::new(&origin, Duration::from_secs(2)).unwrap();
            Self {
                http,
                server,
                token_path,
                _guard: guard,
            }
        }

        fn finish(self) {
            self.server.join().unwrap();
            std::fs::remove_file(self.token_path).unwrap();
        }
    }

    fn inspection(policy: &str) -> Value {
        json!({
            "schemaVersion":1,
            "operation":"desktop.inspect",
            "desktop":{
                "desktopId":target().desktop_id,
                "generation":3,
                "state":"local_ready",
                "readinessScope":"live_resource"
            },
            "inputCapability":{
                "policyState":policy,
                "reasonCode":"provider-private-sentinel",
                "slot":6,
                "nextAction":"provider-private-sentinel"
            }
        })
    }

    #[test]
    fn native_readiness_disabled_consumer_policy_is_not_viewer_readiness() {
        let mut fixture = OwnerFixture::new(inspection("disabled"), "200 OK", 2);
        let readiness = qualify_readiness(&mut fixture.http, &target()).unwrap();
        assert_eq!(readiness["inputReady"], false);
        assert_eq!(readiness["policyState"], "disabled");
        assert_eq!(readiness["ownerAuthentication"], "authenticated");
        assert_eq!(
            readiness["nextAction"],
            "configure_consumer_selective_input"
        );
        assert_eq!(readiness["viewerReadiness"], "not_checked");
        assert_eq!(
            require_input_ready(&mut fixture.http, &target()).unwrap_err(),
            "browser_native_input_disabled:configure_consumer_selective_input"
        );
        fixture.finish();
    }

    #[test]
    fn native_readiness_enabled_policy_preserves_independent_authority_and_secrecy() {
        let mut fixture = OwnerFixture::new(inspection("enabled"), "200 OK", 2);
        let readiness = qualify_readiness(&mut fixture.http, &target()).unwrap();
        assert_eq!(readiness["inputReady"], true);
        assert_eq!(readiness["policyState"], "enabled");
        assert_eq!(readiness["nextAction"], "read_desktop_control_status");
        assert_eq!(readiness["controlAuthority"], "not_granted");
        assert_eq!(readiness["viewerReadiness"], "not_checked");
        assert!(!readiness.to_string().contains("provider-private-sentinel"));
        assert!(!readiness
            .to_string()
            .contains("fixture-native-readiness-token"));
        assert!(!readiness.to_string().contains("resources"));
        assert!(!readiness.to_string().contains("slot"));
        require_input_ready(&mut fixture.http, &target()).unwrap();
        fixture.finish();
    }

    #[test]
    fn native_readiness_changed_or_unverified_lifecycle_never_permits_input() {
        for condition in [
            "other-desktop",
            "other-generation",
            "not-live",
            "unknown-schema",
            "unknown-policy",
            "missing-policy",
        ] {
            let mut response = inspection("enabled");
            match condition {
                "other-desktop" => {
                    response["desktop"]["desktopId"] = json!(target().viewing_desktop_id)
                }
                "other-generation" => {
                    response["desktop"]["generation"] = json!(target().viewing_generation)
                }
                "not-live" => response["desktop"]["state"] = json!("draining"),
                "unknown-schema" => response["schemaVersion"] = json!(2),
                "unknown-policy" => {
                    response["inputCapability"]["policyState"] = json!("provider-private-sentinel")
                }
                "missing-policy" => {
                    response.as_object_mut().unwrap().remove("inputCapability");
                }
                _ => unreachable!(),
            }
            let mut fixture = OwnerFixture::new(response, "200 OK", 1);
            let error = require_input_ready(&mut fixture.http, &target()).unwrap_err();
            assert!(
                error.starts_with("browser_native_input_target_changed:")
                    || error.starts_with("browser_native_input_unverified:"),
                "unexpected readiness outcome for {condition}"
            );
            assert!(!error.contains("provider-private-sentinel"));
            fixture.finish();
        }
    }

    #[test]
    fn native_readiness_rejected_owner_access_is_not_disabled_input() {
        for (status, expected) in [
            (
                "401 Unauthorized",
                "browser_native_owner_authentication_rejected:verify_private_owner_control_token",
            ),
            (
                "403 Forbidden",
                "browser_native_owner_access_denied:verify_owner_control_access",
            ),
        ] {
            let mut fixture =
                OwnerFixture::new(json!({"code":"provider-private-sentinel"}), status, 1);
            let error = qualify_readiness(&mut fixture.http, &target()).unwrap_err();
            assert_eq!(error, expected);
            fixture.finish();
        }
    }
}
