//! Read-only causal recovery of an interrupted reserved Chrome launch.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use agent_browser_cdp::client::CdpClient;
use agent_browser_service_model::{BrowserDesktopAssignment, BrowserProfileCatalogEntry};
use serde_json::Value;

use super::reserved_browser_proof::{prove_reserved_browser, ReservedBrowserObservations};
use super::ReservedBrowserRecovery;
use crate::process_identity::{observe_process, ProcessObservation};

const MAX_ENDPOINT_BYTES: u64 = 4096;

/// Read only the selected profile's browser endpoint; never discover another
/// profile or fall back to a conventional debugging port.
fn profile_endpoint(profile_path: &Path) -> Result<String, String> {
    let path = profile_path.join("DevToolsActivePort");
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let file = options
        .open(path)
        .map_err(|_| "reserved_browser_endpoint_unavailable")?;
    if !file
        .metadata()
        .map_err(|_| "reserved_browser_endpoint_unavailable")?
        .is_file()
    {
        return Err("reserved_browser_endpoint_invalid".to_string());
    }
    let mut text = String::new();
    file.take(MAX_ENDPOINT_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|_| "reserved_browser_endpoint_invalid")?;
    if text.len() as u64 > MAX_ENDPOINT_BYTES {
        return Err("reserved_browser_endpoint_invalid".to_string());
    }
    parse_profile_endpoint(&text)
}

fn parse_profile_endpoint(text: &str) -> Result<String, String> {
    let mut lines = text.lines();
    let port = lines.next().unwrap_or_default();
    let path = lines.next().unwrap_or_default();
    let port = port
        .parse::<u16>()
        .ok()
        .filter(|value| *value != 0)
        .ok_or("reserved_browser_endpoint_invalid")?;
    let token = path
        .strip_prefix("/devtools/browser/")
        .filter(|token| !token.is_empty() && token.len() <= 128)
        .filter(|token| {
            token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
        .ok_or("reserved_browser_endpoint_invalid")?;
    if lines.any(|line| !line.is_empty()) {
        return Err("reserved_browser_endpoint_invalid".to_string());
    }
    Ok(format!("ws://127.0.0.1:{port}/devtools/browser/{token}"))
}

fn browser_pid(response: &Value) -> Result<u32, String> {
    let processes = response
        .get("processInfo")
        .and_then(Value::as_array)
        .ok_or("reserved_browser_process_info_invalid")?;
    let mut browsers = processes
        .iter()
        .filter(|process| process["type"] == "browser");
    let browser = browsers
        .next()
        .ok_or("reserved_browser_process_info_invalid")?;
    if browsers.next().is_some() {
        return Err("reserved_browser_process_info_ambiguous".to_string());
    }
    let pid = browser["id"]
        .as_f64()
        .filter(|pid| *pid >= 1.0 && *pid <= f64::from(u32::MAX) && pid.fract() == 0.0)
        .ok_or("reserved_browser_process_info_invalid")?;
    Ok(pid as u32)
}

pub(super) async fn recover_reserved_browser(
    profile: &BrowserProfileCatalogEntry,
    desktop: &BrowserDesktopAssignment,
    browser_id: &str,
    timeout: Duration,
) -> ReservedBrowserRecovery {
    recover_with_observer(profile, desktop, browser_id, timeout, observe_process).await
}

async fn recover_with_observer(
    profile: &BrowserProfileCatalogEntry,
    desktop: &BrowserDesktopAssignment,
    browser_id: &str,
    timeout: Duration,
    mut observe: impl FnMut(u32) -> ProcessObservation,
) -> ReservedBrowserRecovery {
    let result = tokio::time::timeout(timeout, async {
        let endpoint = profile_endpoint(Path::new(&profile.user_data_dir))?;
        let client = CdpClient::connect(&endpoint)
            .await
            .map_err(|_| "reserved_browser_endpoint_unresponsive")?;
        let first = client
            .send_command_no_params("SystemInfo.getProcessInfo", None)
            .await
            .map_err(|_| "reserved_browser_process_info_unavailable")?;
        let pid = browser_pid(&first)?;
        let before = observe(pid);
        let second = client
            .send_command_no_params("SystemInfo.getProcessInfo", None)
            .await
            .map_err(|_| "reserved_browser_process_info_unavailable")?;
        if browser_pid(&second)? != pid {
            return Err("reserved_browser_endpoint_process_changed".to_string());
        }
        let after = observe(pid);
        if profile_endpoint(Path::new(&profile.user_data_dir))? != endpoint {
            return Err("reserved_browser_endpoint_changed".to_string());
        }
        prove_reserved_browser(
            profile,
            desktop,
            browser_id,
            &endpoint,
            pid,
            ReservedBrowserObservations {
                before: &before,
                after: &after,
            },
        )
    })
    .await;
    match result {
        Ok(Ok(launch)) => ReservedBrowserRecovery::Recovered(launch),
        Ok(Err(reason)) => ReservedBrowserRecovery::Unproven { reason },
        Err(_) => ReservedBrowserRecovery::Unproven {
            reason: "reserved_browser_probe_timeout".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_identity::ObservedProcessIdentity;
    use agent_browser_service_model::BrowserProfileKind;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    struct ProfileFixture(PathBuf);
    impl ProfileFixture {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("reserved-browser-proof-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn profile(&self) -> BrowserProfileCatalogEntry {
            BrowserProfileCatalogEntry {
                id: "work".to_string(),
                name: "Work".to_string(),
                user_data_dir: self.0.to_string_lossy().into_owned(),
                kind: BrowserProfileKind::Named,
            }
        }
    }
    impl Drop for ProfileFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn desktop() -> BrowserDesktopAssignment {
        BrowserDesktopAssignment {
            route_id: "route-1".to_string(),
            display_name: ":10".to_string(),
            live_browser_count: 0,
        }
    }
    fn observation(pid: u32, profile: &BrowserProfileCatalogEntry) -> ProcessObservation {
        ProcessObservation::Observed(ObservedProcessIdentity {
            pid,
            start_token: Some("boot:start:42".to_string()),
            executable_path: Some("/opt/chrome/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
            command_line: Some(vec![
                "/opt/chrome/chrome".to_string(),
                "--agent-browser-reservation-id=reserved-1".to_string(),
                format!("--user-data-dir={}", profile.user_data_dir),
            ]),
        })
    }
    async fn server(
        fixture: &ProfileFixture,
        pids: [u32; 2],
    ) -> (Arc<Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        std::fs::write(
            fixture.0.join("DevToolsActivePort"),
            format!("{port}\n/devtools/browser/fixture-1\n"),
        )
        .unwrap();
        let methods = Arc::new(Mutex::new(Vec::new()));
        let received = methods.clone();
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            for pid in pids {
                let message = socket.next().await.unwrap().unwrap();
                let request: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
                received
                    .lock()
                    .unwrap()
                    .push(request["method"].as_str().unwrap().to_string());
                socket.send(Message::Text(json!({"id":request["id"],"result":{
                    "processInfo":[{"type":"browser","id":pid,"cpuTime":0}, {"type":"renderer","id":999}]
                }}).to_string())).await.unwrap();
            }
            // Keep the transport alive until the probe drops its read-only client.
            while socket.next().await.is_some() {}
        });
        (methods, handle)
    }

    #[test]
    fn endpoint_and_process_info_reject_ambiguous_or_external_inputs() {
        assert_eq!(
            parse_profile_endpoint("9222\n/devtools/browser/abc-123\n").unwrap(),
            "ws://127.0.0.1:9222/devtools/browser/abc-123"
        );
        for invalid in [
            "0\n/devtools/browser/id",
            "9222\nws://example.test",
            "9222\n/devtools/browser/id?redirect=foreign",
            "9222\n/devtools/page/id",
            "9222\n/devtools/browser/",
            "9222\n/devtools/browser/id\nextra",
        ] {
            assert!(parse_profile_endpoint(invalid).is_err(), "{invalid}");
        }
        assert_eq!(
            browser_pid(&json!({"processInfo":[{"type":"browser","id":42.0}]})).unwrap(),
            42
        );
        for invalid in [
            json!({}),
            json!({"processInfo":[]}),
            json!({"processInfo":[{"type":"browser","id":0}]}),
            json!({"processInfo":[{"type":"browser","id":1.5}]}),
            json!({"processInfo":[{"type":"browser","id":1},{"type":"browser","id":2}]}),
        ] {
            assert!(browser_pid(&invalid).is_err());
        }
        let fixture = ProfileFixture::new();
        std::fs::write(fixture.0.join("DevToolsActivePort"), "x".repeat(4097)).unwrap();
        assert!(profile_endpoint(&fixture.0).is_err());
    }

    #[tokio::test]
    async fn local_cdp_recovery_binds_reserved_identity_without_browser_mutation() {
        let fixture = ProfileFixture::new();
        let profile = fixture.profile();
        let (methods, handle) = server(&fixture, [4250, 4250]).await;
        let recovered = recover_with_observer(
            &profile,
            &desktop(),
            "reserved-1",
            Duration::from_secs(2),
            |pid| observation(pid, &profile),
        )
        .await;
        let ReservedBrowserRecovery::Recovered(launch) = recovered else {
            panic!("{recovered:?}")
        };
        assert_eq!(launch.browser_id, "reserved-1");
        assert_eq!(launch.pid, 4250);
        assert_eq!(launch.desktop, Some(desktop()));
        assert_eq!(launch.cdp_endpoint, profile_endpoint(&fixture.0).unwrap());
        assert_eq!(
            methods.lock().unwrap().as_slice(),
            ["SystemInfo.getProcessInfo", "SystemInfo.getProcessInfo"]
        );
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn local_cdp_recovery_retains_obligation_on_changed_pid_or_endpoint() {
        for change_pid in [true, false] {
            let fixture = ProfileFixture::new();
            let profile = fixture.profile();
            let (_, handle) = server(&fixture, [4250, if change_pid { 4251 } else { 4250 }]).await;
            let mut observations = 0;
            let recovered = recover_with_observer(
                &profile,
                &desktop(),
                "reserved-1",
                Duration::from_secs(2),
                |pid| {
                    observations += 1;
                    if !change_pid && observations == 2 {
                        std::fs::write(
                            fixture.0.join("DevToolsActivePort"),
                            "9222\n/devtools/browser/replaced\n",
                        )
                        .unwrap();
                    }
                    observation(pid, &profile)
                },
            )
            .await;
            let expected = if change_pid {
                "reserved_browser_endpoint_process_changed"
            } else {
                "reserved_browser_endpoint_changed"
            };
            assert_eq!(
                recovered,
                ReservedBrowserRecovery::Unproven {
                    reason: expected.to_string()
                }
            );
            tokio::time::timeout(Duration::from_secs(2), handle)
                .await
                .unwrap()
                .unwrap();
        }
    }

    #[test]
    fn runtime_actor_uses_selected_profile_probe_instead_of_default_unproven_stub() {
        use super::super::{
            BrowserManagerRuntime, BrowserManagerRuntimeConfig, BrowserRuntimeDriver,
        };
        let fixture = ProfileFixture::new();
        let mut runtime =
            BrowserManagerRuntime::start(BrowserManagerRuntimeConfig::default()).unwrap();
        let result = runtime
            .recover_browser_reserved(&fixture.profile(), &desktop(), "reserved-1")
            .unwrap();
        assert_eq!(
            result,
            ReservedBrowserRecovery::Unproven {
                reason: "reserved_browser_endpoint_unavailable".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn stalled_endpoint_stops_at_injected_deadline() {
        let fixture = ProfileFixture::new();
        let profile = fixture.profile();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        std::fs::write(
            fixture.0.join("DevToolsActivePort"),
            format!(
                "{}\n/devtools/browser/stalled\n",
                listener.local_addr().unwrap().port()
            ),
        )
        .unwrap();
        let handle = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            std::future::pending::<()>().await;
        });
        let recovered = recover_with_observer(
            &profile,
            &desktop(),
            "reserved-1",
            Duration::from_millis(20),
            |_| panic!("no process observation before CDP proof"),
        )
        .await;
        assert_eq!(
            recovered,
            ReservedBrowserRecovery::Unproven {
                reason: "reserved_browser_probe_timeout".to_string()
            }
        );
        handle.abort();
        assert!(handle.await.unwrap_err().is_cancelled());
    }
}
