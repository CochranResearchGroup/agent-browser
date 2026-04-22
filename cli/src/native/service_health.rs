//! Health probes for persisted service-mode browser records.

use std::time::Duration;

use super::service_model::{BrowserHealth, BrowserProcess, ServiceState};
use super::service_store::{JsonServiceStateStore, ServiceStateStore};

const CDP_PROBE_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceReconcileSummary {
    pub browser_count: usize,
    pub changed_browsers: usize,
}

pub async fn reconcile_service_state(state: &mut ServiceState) -> ServiceReconcileSummary {
    let before = state.clone();
    refresh_persisted_browser_health(state).await;

    let changed_browsers = state
        .browsers
        .iter()
        .filter(|(id, browser)| {
            before
                .browsers
                .get(*id)
                .map(|previous| {
                    previous.health != browser.health || previous.last_error != browser.last_error
                })
                .unwrap_or(true)
        })
        .count();

    ServiceReconcileSummary {
        browser_count: state.browsers.len(),
        changed_browsers,
    }
}

pub async fn reconcile_persisted_service_state() -> Result<ServiceReconcileSummary, String> {
    let store = JsonServiceStateStore::new(JsonServiceStateStore::default_path()?);
    let mut state = store.load()?;
    let summary = reconcile_service_state(&mut state).await;
    store.save(&state)?;
    Ok(summary)
}

pub async fn refresh_persisted_browser_health(state: &mut ServiceState) {
    for browser in state.browsers.values_mut() {
        refresh_browser_record_health(browser).await;
    }
}

async fn refresh_browser_record_health(browser: &mut BrowserProcess) {
    if matches!(
        browser.health,
        BrowserHealth::NotStarted | BrowserHealth::Launching | BrowserHealth::Closing
    ) {
        return;
    }

    if let Some(pid) = browser.pid {
        if !pid_is_running(pid) {
            browser.health = BrowserHealth::ProcessExited;
            browser.last_error = Some(format!("Recorded browser PID {} is no longer running", pid));
            return;
        }
    }

    if let Some(endpoint) = browser.cdp_endpoint.as_deref() {
        if cdp_endpoint_reachable(endpoint).await {
            browser.health = BrowserHealth::Ready;
            browser.last_error = None;
        } else if browser.pid.is_some() {
            browser.health = BrowserHealth::CdpDisconnected;
            browser.last_error = Some(format!("CDP endpoint is unreachable: {}", endpoint));
        } else {
            browser.health = BrowserHealth::Unreachable;
            browser.last_error = Some(format!("CDP endpoint is unreachable: {}", endpoint));
        }
    }
}

async fn cdp_endpoint_reachable(endpoint: &str) -> bool {
    let Some(url) = cdp_version_url(endpoint) else {
        return false;
    };
    let Ok(client) = reqwest::Client::builder()
        .timeout(CDP_PROBE_TIMEOUT)
        .build()
    else {
        return false;
    };
    let Ok(response) = client.get(url).send().await else {
        return false;
    };
    response.status().is_success()
}

fn cdp_version_url(endpoint: &str) -> Option<String> {
    let mut url = url::Url::parse(endpoint).ok()?;
    match url.scheme() {
        "ws" => url.set_scheme("http").ok()?,
        "wss" => url.set_scheme("https").ok()?,
        "http" | "https" => {}
        _ => return None,
    }
    url.set_path("/json/version");
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

#[cfg(unix)]
fn pid_is_running(pid: u32) -> bool {
    let rc = unsafe { libc::kill(pid as i32, 0) };
    rc == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn pid_is_running(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, STILL_ACTIVE,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return false;
        }
        let mut exit_code = 0;
        let ok = GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);
        ok != 0 && exit_code == STILL_ACTIVE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn service_state_with_browser(browser: BrowserProcess) -> ServiceState {
        ServiceState {
            browsers: BTreeMap::from([(browser.id.clone(), browser)]),
            ..ServiceState::default()
        }
    }

    async fn serve_json_version(listener: TcpListener) {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf).await;
        let body = r#"{"Browser":"Chrome/123"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    }

    #[test]
    fn cdp_version_url_normalizes_websocket_endpoint() {
        assert_eq!(
            cdp_version_url("ws://127.0.0.1:9222/devtools/browser/abc?token=1").as_deref(),
            Some("http://127.0.0.1:9222/json/version")
        );
        assert_eq!(
            cdp_version_url("https://example.com/devtools/browser/abc").as_deref(),
            Some("https://example.com/json/version")
        );
        assert!(cdp_version_url("file:///tmp/not-cdp").is_none());
    }

    #[tokio::test]
    async fn refresh_marks_reachable_cdp_ready() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_json_version(listener));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Unreachable,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            last_error: Some("previous failure".to_string()),
            ..BrowserProcess::default()
        });

        refresh_persisted_browser_health(&mut state).await;
        server.await.unwrap();

        let browser = &state.browsers["browser-1"];
        assert_eq!(browser.health, BrowserHealth::Ready);
        assert_eq!(browser.last_error, None);
    }

    #[tokio::test]
    async fn refresh_marks_unreachable_cdp_without_pid_unreachable() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some("ws://127.0.0.1:9/devtools/browser/abc".to_string()),
            ..BrowserProcess::default()
        });

        refresh_persisted_browser_health(&mut state).await;

        let browser = &state.browsers["browser-1"];
        assert_eq!(browser.health, BrowserHealth::Unreachable);
        assert!(browser
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("CDP endpoint is unreachable"));
    }

    #[tokio::test]
    async fn refresh_marks_dead_pid_process_exited() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            pid: Some(i32::MAX as u32),
            cdp_endpoint: Some("ws://127.0.0.1:9/devtools/browser/abc".to_string()),
            ..BrowserProcess::default()
        });

        refresh_persisted_browser_health(&mut state).await;

        let browser = &state.browsers["browser-1"];
        assert_eq!(browser.health, BrowserHealth::ProcessExited);
        assert!(browser
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("no longer running"));
    }
}
