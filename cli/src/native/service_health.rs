//! Health and target probes for persisted service-mode browser records.

use std::collections::BTreeSet;
use std::time::Duration;

use serde::Deserialize;

use super::service_model::{
    BrowserHealth, BrowserProcess, BrowserSession, BrowserTab, ServiceEvent, ServiceEventKind,
    ServiceReconciliationSnapshot, ServiceState, TabLifecycle,
};
use super::service_store::{JsonServiceStateStore, ServiceStateStore};

const CDP_PROBE_TIMEOUT: Duration = Duration::from_millis(750);
const MAX_SERVICE_EVENTS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceReconcileSummary {
    pub browser_count: usize,
    pub changed_browsers: usize,
}

pub async fn reconcile_service_state(state: &mut ServiceState) -> ServiceReconcileSummary {
    let before = state.clone();
    refresh_persisted_browser_health(state).await;
    reconcile_live_browser_targets(state).await;

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

    let summary = ServiceReconcileSummary {
        browser_count: state.browsers.len(),
        changed_browsers,
    };
    state.reconciliation = Some(ServiceReconciliationSnapshot {
        last_reconciled_at: Some(current_timestamp()),
        last_error: None,
        browser_count: summary.browser_count,
        changed_browsers: summary.changed_browsers,
    });
    record_health_transition_events(state, &before);
    record_tab_lifecycle_events(state, &before);
    push_service_event(
        state,
        ServiceEvent {
            kind: ServiceEventKind::Reconciliation,
            message: format!(
                "Reconciled {} browser records, {} changed",
                summary.browser_count, summary.changed_browsers
            ),
            details: Some(serde_json::json!({
                "browserCount": summary.browser_count,
                "changedBrowsers": summary.changed_browsers,
                "tabCount": state.tabs.len(),
                "changedTabs": changed_tab_count(state, &before),
            })),
            ..new_service_event()
        },
    );
    summary
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

async fn reconcile_live_browser_targets(state: &mut ServiceState) {
    let browser_records = state
        .browsers
        .iter()
        .map(|(id, browser)| (id.clone(), browser.clone()))
        .collect::<Vec<_>>();

    for (browser_id, browser) in browser_records {
        if browser.health != BrowserHealth::Ready {
            close_browser_tabs(state, &browser_id);
            continue;
        }
        let Some(endpoint) = browser.cdp_endpoint.as_deref() else {
            close_browser_tabs(state, &browser_id);
            continue;
        };
        let targets = match fetch_cdp_targets(endpoint).await {
            Ok(targets) => targets,
            Err(err) => {
                if let Some(browser) = state.browsers.get_mut(&browser_id) {
                    browser.health = BrowserHealth::Degraded;
                    browser.last_error = Some(err);
                }
                close_browser_tabs(state, &browser_id);
                continue;
            }
        };
        reconcile_browser_targets(state, &browser_id, &browser, targets);
    }
}

fn reconcile_browser_targets(
    state: &mut ServiceState,
    browser_id: &str,
    browser: &BrowserProcess,
    targets: Vec<CdpHttpTargetInfo>,
) {
    let mut live_tab_ids = BTreeSet::new();
    let owner_session_id = browser.active_session_ids.first().cloned();

    for target in targets.into_iter().filter(should_track_service_target) {
        let tab_id = format!("target:{}", target.id);
        live_tab_ids.insert(tab_id.clone());
        state.tabs.insert(
            tab_id.clone(),
            BrowserTab {
                id: tab_id.clone(),
                browser_id: browser_id.to_string(),
                target_id: Some(target.id),
                lifecycle: TabLifecycle::Ready,
                url: empty_to_none(target.url),
                title: empty_to_none(target.title),
                owner_session_id: owner_session_id.clone(),
                ..state.tabs.get(&tab_id).cloned().unwrap_or_default()
            },
        );
    }

    for tab in state.tabs.values_mut() {
        if tab.browser_id == browser_id && !live_tab_ids.contains(&tab.id) {
            tab.lifecycle = TabLifecycle::Closed;
        }
    }

    if let Some(owner_session_id) = owner_session_id {
        let session = state
            .sessions
            .entry(owner_session_id.clone())
            .or_insert_with(|| BrowserSession {
                id: owner_session_id.clone(),
                ..BrowserSession::default()
            });
        merge_unique(&mut session.browser_ids, browser_id.to_string());
        session.tab_ids.retain(|tab_id| {
            state
                .tabs
                .get(tab_id)
                .is_none_or(|tab| tab.browser_id != browser_id)
        });
        for tab_id in live_tab_ids {
            merge_unique(&mut session.tab_ids, tab_id);
        }
    }
}

fn close_browser_tabs(state: &mut ServiceState, browser_id: &str) {
    let closed_tab_ids = state
        .tabs
        .values_mut()
        .filter_map(|tab| {
            if tab.browser_id == browser_id && tab.lifecycle != TabLifecycle::Closed {
                tab.lifecycle = TabLifecycle::Closed;
                Some(tab.id.clone())
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();

    if closed_tab_ids.is_empty() {
        return;
    }

    for session in state.sessions.values_mut() {
        session
            .tab_ids
            .retain(|tab_id| !closed_tab_ids.contains(tab_id));
    }
}

async fn fetch_cdp_targets(endpoint: &str) -> Result<Vec<CdpHttpTargetInfo>, String> {
    let Some(url) = cdp_list_url(endpoint) else {
        return Err("Invalid CDP endpoint".to_string());
    };
    let client = reqwest::Client::builder()
        .timeout(CDP_PROBE_TIMEOUT)
        .build()
        .map_err(|err| format!("Failed to build CDP target client: {}", err))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("Failed to fetch CDP targets: {}", err))?;
    if !response.status().is_success() {
        return Err(format!("CDP target list returned {}", response.status()));
    }
    response
        .json::<Vec<CdpHttpTargetInfo>>()
        .await
        .map_err(|err| format!("Failed to parse CDP targets: {}", err))
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

fn cdp_list_url(endpoint: &str) -> Option<String> {
    let mut url = cdp_root_url(endpoint)?;
    url.set_path("/json/list");
    Some(url.to_string())
}

fn cdp_version_url(endpoint: &str) -> Option<String> {
    let mut url = cdp_root_url(endpoint)?;
    url.set_path("/json/version");
    Some(url.to_string())
}

fn cdp_root_url(endpoint: &str) -> Option<url::Url> {
    let mut url = url::Url::parse(endpoint).ok()?;
    match url.scheme() {
        "ws" => url.set_scheme("http").ok()?,
        "wss" => url.set_scheme("https").ok()?,
        "http" | "https" => {}
        _ => return None,
    }
    url.set_query(None);
    url.set_fragment(None);
    Some(url)
}

fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn new_service_event() -> ServiceEvent {
    let timestamp = current_timestamp();
    ServiceEvent {
        id: format!("event-{}", uuid::Uuid::new_v4()),
        timestamp,
        ..ServiceEvent::default()
    }
}

fn push_service_event(state: &mut ServiceState, event: ServiceEvent) {
    state.events.push(event);
    if state.events.len() > MAX_SERVICE_EVENTS {
        let excess = state.events.len() - MAX_SERVICE_EVENTS;
        state.events.drain(0..excess);
    }
}

pub fn record_browser_health_changed_event(
    state: &mut ServiceState,
    browser_id: &str,
    previous: Option<&BrowserProcess>,
    current: &BrowserProcess,
) {
    let Some(previous) = previous else {
        return;
    };
    if previous.health == current.health && previous.last_error == current.last_error {
        return;
    }
    let mut event = ServiceEvent {
        kind: ServiceEventKind::BrowserHealthChanged,
        message: format!(
            "Browser {} health changed from {:?} to {:?}",
            browser_id, previous.health, current.health
        ),
        browser_id: Some(browser_id.to_string()),
        previous_health: Some(previous.health),
        current_health: Some(current.health),
        details: Some(serde_json::json!({
            "previousError": previous.last_error,
            "currentError": current.last_error,
        })),
        ..new_service_event()
    };
    enrich_service_event_with_browser_context(&mut event, state, browser_id, current);
    push_service_event(state, event);
}

pub fn record_browser_launch_recorded_event(
    state: &mut ServiceState,
    browser_id: &str,
    previous: Option<&BrowserProcess>,
    current: &BrowserProcess,
) {
    let mut event = ServiceEvent {
        kind: ServiceEventKind::BrowserLaunchRecorded,
        message: format!("Browser {} launch metadata recorded", browser_id),
        browser_id: Some(browser_id.to_string()),
        current_health: Some(current.health),
        details: Some(serde_json::json!({
            "previousProfileId": previous.and_then(|browser| browser.profile_id.clone()),
            "currentProfileId": current.profile_id,
            "previousSessionIds": previous
                .map(|browser| browser.active_session_ids.clone())
                .unwrap_or_default(),
            "currentSessionIds": current.active_session_ids,
            "host": current.host,
            "pid": current.pid,
            "cdpEndpoint": current.cdp_endpoint,
        })),
        ..new_service_event()
    };
    enrich_service_event_with_browser_context(&mut event, state, browser_id, current);
    push_service_event(state, event);
}

fn enrich_service_event_with_browser_context(
    event: &mut ServiceEvent,
    state: &ServiceState,
    browser_id: &str,
    browser: &BrowserProcess,
) {
    let session_id = browser
        .active_session_ids
        .first()
        .cloned()
        .or_else(|| session_id_for_browser(state, browser_id));
    let session = session_id
        .as_ref()
        .and_then(|session_id| state.sessions.get(session_id));

    event.profile_id = browser
        .profile_id
        .clone()
        .or_else(|| session.and_then(|session| session.profile_id.clone()));
    event.session_id = session_id;
    event.service_name = session.and_then(|session| session.service_name.clone());
    event.agent_name = session.and_then(|session| session.agent_name.clone());
    event.task_name = session.and_then(|session| session.task_name.clone());
}

fn session_id_for_browser(state: &ServiceState, browser_id: &str) -> Option<String> {
    state.sessions.iter().find_map(|(session_id, session)| {
        session
            .browser_ids
            .iter()
            .any(|id| id == browser_id)
            .then(|| session_id.clone())
    })
}

fn record_health_transition_events(state: &mut ServiceState, before: &ServiceState) {
    let transitions: Vec<(String, BrowserProcess, BrowserProcess)> = state
        .browsers
        .iter()
        .filter_map(|(id, browser)| {
            before
                .browsers
                .get(id)
                .map(|previous| (id.clone(), previous.clone(), browser.clone()))
        })
        .collect();

    for (id, previous, current) in transitions {
        record_browser_health_changed_event(state, &id, Some(&previous), &current);
    }
}

fn record_tab_lifecycle_events(state: &mut ServiceState, before: &ServiceState) {
    let mut events = Vec::new();
    for (id, tab) in &state.tabs {
        let previous = before.tabs.get(id);
        if previous.is_some_and(|previous| previous == tab) {
            continue;
        }

        let message = match previous {
            None => format!("Tab {} opened", id),
            Some(previous) if previous.lifecycle != tab.lifecycle => format!(
                "Tab {} lifecycle changed from {:?} to {:?}",
                id, previous.lifecycle, tab.lifecycle
            ),
            Some(_) => format!("Tab {} metadata changed", id),
        };

        events.push(ServiceEvent {
            kind: ServiceEventKind::TabLifecycleChanged,
            message,
            browser_id: Some(tab.browser_id.clone()),
            details: Some(serde_json::json!({
                "tabId": id,
                "targetId": tab.target_id.clone(),
                "previousLifecycle": previous.map(|tab| tab.lifecycle),
                "currentLifecycle": tab.lifecycle,
                "previousUrl": previous.and_then(|tab| tab.url.clone()),
                "currentUrl": tab.url.clone(),
                "previousTitle": previous.and_then(|tab| tab.title.clone()),
                "currentTitle": tab.title.clone(),
                "ownerSessionId": tab.owner_session_id.clone(),
            })),
            ..new_service_event()
        });
    }
    for event in events {
        push_service_event(state, event);
    }
}

fn changed_tab_count(state: &ServiceState, before: &ServiceState) -> usize {
    state
        .tabs
        .iter()
        .filter(|(id, tab)| before.tabs.get(*id) != Some(*tab))
        .count()
}

fn empty_to_none(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn merge_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn should_track_service_target(target: &CdpHttpTargetInfo) -> bool {
    (target.target_type == "page" || target.target_type == "webview")
        && !target.url.starts_with("chrome://")
        && !target.url.starts_with("chrome-extension://")
        && !target.url.starts_with("devtools://")
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct CdpHttpTargetInfo {
    id: String,
    #[serde(rename = "type")]
    target_type: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
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

    #[test]
    fn launch_recorded_event_copies_browser_session_context() {
        let mut state = ServiceState {
            sessions: BTreeMap::from([(
                "session-1".to_string(),
                BrowserSession {
                    id: "session-1".to_string(),
                    profile_id: Some("work".to_string()),
                    service_name: Some("JournalDownloader".to_string()),
                    agent_name: Some("codex".to_string()),
                    task_name: Some("probeACSwebsite".to_string()),
                    browser_ids: vec!["browser-1".to_string()],
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            profile_id: Some("work".to_string()),
            active_session_ids: vec!["session-1".to_string()],
            health: BrowserHealth::Ready,
            ..BrowserProcess::default()
        };

        record_browser_launch_recorded_event(&mut state, "browser-1", None, &browser);

        let event = state.events.first().unwrap();
        assert_eq!(event.kind, ServiceEventKind::BrowserLaunchRecorded);
        assert_eq!(event.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(event.profile_id.as_deref(), Some("work"));
        assert_eq!(event.session_id.as_deref(), Some("session-1"));
        assert_eq!(event.service_name.as_deref(), Some("JournalDownloader"));
        assert_eq!(event.agent_name.as_deref(), Some("codex"));
        assert_eq!(event.task_name.as_deref(), Some("probeACSwebsite"));
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

    async fn serve_cdp_version_and_list(listener: TcpListener, list_body: &'static str) {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let read = stream.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..read]);
            let body = if request.contains("/json/list") {
                list_body
            } else {
                r#"{"Browser":"Chrome/123"}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        }
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

    #[test]
    fn cdp_list_url_normalizes_websocket_endpoint() {
        assert_eq!(
            cdp_list_url("ws://127.0.0.1:9222/devtools/browser/abc?token=1").as_deref(),
            Some("http://127.0.0.1:9222/json/list")
        );
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
    async fn reconcile_discovers_live_cdp_targets() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_cdp_version_and_list(
            listener,
            r#"[
                {"id":"page-1","type":"page","title":"Example","url":"https://example.com"},
                {"id":"devtools-1","type":"page","title":"DevTools","url":"devtools://devtools/bundled/inspector.html"}
            ]"#,
        ));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            active_session_ids: vec!["session-1".to_string()],
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:stale".to_string(),
            BrowserTab {
                id: "target:stale".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("stale".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-1".to_string()),
                ..BrowserTab::default()
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["target:stale".to_string()],
                ..BrowserSession::default()
            },
        );

        reconcile_service_state(&mut state).await;
        server.await.unwrap();

        let tab = &state.tabs["target:page-1"];
        assert_eq!(tab.browser_id, "browser-1");
        assert_eq!(tab.target_id.as_deref(), Some("page-1"));
        assert_eq!(tab.lifecycle, TabLifecycle::Ready);
        assert_eq!(tab.url.as_deref(), Some("https://example.com"));
        assert_eq!(tab.title.as_deref(), Some("Example"));
        assert_eq!(tab.owner_session_id.as_deref(), Some("session-1"));
        assert!(!state.tabs.contains_key("target:devtools-1"));

        let session = &state.sessions["session-1"];
        assert_eq!(session.browser_ids, vec!["browser-1"]);
        assert_eq!(session.tab_ids, vec!["target:page-1"]);
        assert_eq!(state.tabs["target:stale"].lifecycle, TabLifecycle::Closed);
        assert_eq!(
            state.events.last().unwrap().details.as_ref().unwrap()["changedTabs"],
            2
        );
        let tab_event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::TabLifecycleChanged)
            .unwrap();
        assert_eq!(tab_event.browser_id.as_deref(), Some("browser-1"));
        assert_eq!(
            tab_event.details.as_ref().unwrap()["tabId"],
            "target:page-1"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentLifecycle"],
            "ready"
        );
    }

    #[tokio::test]
    async fn reconcile_marks_target_list_failure_degraded() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_cdp_version_and_list(listener, r#"not-json"#));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:page-1".to_string(),
            BrowserTab {
                id: "target:page-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("page-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-1".to_string()),
                ..BrowserTab::default()
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["target:page-1".to_string()],
                ..BrowserSession::default()
            },
        );

        reconcile_service_state(&mut state).await;
        server.await.unwrap();

        let browser = &state.browsers["browser-1"];
        assert_eq!(browser.health, BrowserHealth::Degraded);
        assert!(browser
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("Failed to parse CDP targets"));
        assert_eq!(
            state
                .events
                .iter()
                .find(|event| event.kind == ServiceEventKind::BrowserHealthChanged)
                .unwrap()
                .current_health,
            Some(BrowserHealth::Degraded)
        );
        assert_eq!(
            state
                .reconciliation
                .as_ref()
                .map(|snapshot| snapshot.changed_browsers),
            Some(1)
        );
        assert_eq!(state.tabs["target:page-1"].lifecycle, TabLifecycle::Closed);
        assert!(state.sessions["session-1"].tab_ids.is_empty());
    }

    #[tokio::test]
    async fn reconcile_closes_tabs_for_non_ready_browser() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::ProcessExited,
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:page-1".to_string(),
            BrowserTab {
                id: "target:page-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("page-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-1".to_string()),
                ..BrowserTab::default()
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                browser_ids: vec!["browser-1".to_string()],
                tab_ids: vec!["target:page-1".to_string()],
                ..BrowserSession::default()
            },
        );

        reconcile_service_state(&mut state).await;

        assert_eq!(state.tabs["target:page-1"].lifecycle, TabLifecycle::Closed);
        assert!(state.sessions["session-1"].tab_ids.is_empty());
        let tab_event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::TabLifecycleChanged)
            .unwrap();
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentLifecycle"],
            "closed"
        );
    }

    #[tokio::test]
    async fn reconcile_marks_missing_live_targets_closed() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_cdp_version_and_list(listener, r#"[]"#));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:stale".to_string(),
            BrowserTab {
                id: "target:stale".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("stale".to_string()),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );

        reconcile_service_state(&mut state).await;
        server.await.unwrap();

        assert_eq!(state.tabs["target:stale"].lifecycle, TabLifecycle::Closed);
        assert_eq!(
            state.events.last().unwrap().details.as_ref().unwrap()["changedTabs"],
            1
        );
        let tab_event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::TabLifecycleChanged)
            .unwrap();
        assert_eq!(tab_event.details.as_ref().unwrap()["tabId"], "target:stale");
        assert_eq!(
            tab_event.details.as_ref().unwrap()["previousLifecycle"],
            "ready"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentLifecycle"],
            "closed"
        );
    }

    #[tokio::test]
    async fn reconcile_records_tab_metadata_change_events() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(serve_cdp_version_and_list(
            listener,
            r#"[
                {"id":"page-1","type":"page","title":"New Title","url":"https://example.com/new"}
            ]"#,
        ));
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some(format!("ws://127.0.0.1:{}/devtools/browser/abc", port)),
            ..BrowserProcess::default()
        });
        state.tabs.insert(
            "target:page-1".to_string(),
            BrowserTab {
                id: "target:page-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("page-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                url: Some("https://example.com/old".to_string()),
                title: Some("Old Title".to_string()),
                ..BrowserTab::default()
            },
        );

        reconcile_service_state(&mut state).await;
        server.await.unwrap();

        let tab_event = state
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::TabLifecycleChanged)
            .unwrap();
        assert_eq!(tab_event.message, "Tab target:page-1 metadata changed");
        assert_eq!(
            tab_event.details.as_ref().unwrap()["previousUrl"],
            "https://example.com/old"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentUrl"],
            "https://example.com/new"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["previousTitle"],
            "Old Title"
        );
        assert_eq!(
            tab_event.details.as_ref().unwrap()["currentTitle"],
            "New Title"
        );
    }

    #[tokio::test]
    async fn reconcile_records_summary_snapshot() {
        let mut state = service_state_with_browser(BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            cdp_endpoint: Some("ws://127.0.0.1:9/devtools/browser/abc".to_string()),
            ..BrowserProcess::default()
        });

        let summary = reconcile_service_state(&mut state).await;

        assert_eq!(summary.browser_count, 1);
        assert_eq!(summary.changed_browsers, 1);
        let reconciliation = state.reconciliation.as_ref().unwrap();
        assert_eq!(reconciliation.browser_count, 1);
        assert_eq!(reconciliation.changed_browsers, 1);
        assert!(reconciliation.last_reconciled_at.is_some());
        assert_eq!(reconciliation.last_error, None);
        assert_eq!(state.events.len(), 2);
        assert_eq!(state.events[0].kind, ServiceEventKind::BrowserHealthChanged);
        assert_eq!(state.events[0].browser_id.as_deref(), Some("browser-1"));
        assert_eq!(state.events[0].previous_health, Some(BrowserHealth::Ready));
        assert_eq!(
            state.events[0].current_health,
            Some(BrowserHealth::Unreachable)
        );
        assert_eq!(state.events[1].kind, ServiceEventKind::Reconciliation);
    }

    #[tokio::test]
    async fn reconcile_event_log_is_bounded() {
        let mut state = ServiceState {
            events: (0..MAX_SERVICE_EVENTS)
                .map(|i| ServiceEvent {
                    id: format!("old-{i}"),
                    timestamp: "2026-04-22T00:00:00Z".to_string(),
                    kind: ServiceEventKind::Reconciliation,
                    message: "old".to_string(),
                    ..ServiceEvent::default()
                })
                .collect(),
            browsers: BTreeMap::from([(
                "browser-1".to_string(),
                BrowserProcess {
                    id: "browser-1".to_string(),
                    health: BrowserHealth::NotStarted,
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        };

        reconcile_service_state(&mut state).await;

        assert_eq!(state.events.len(), MAX_SERVICE_EVENTS);
        assert_ne!(state.events[0].id, "old-0");
        assert_eq!(
            state.events.last().map(|event| event.kind),
            Some(ServiceEventKind::Reconciliation)
        );
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
