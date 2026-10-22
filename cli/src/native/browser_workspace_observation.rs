//! Bounded read-only presence and display metadata for the managed inventory.
//! Remote View owns desktop application observation; the browser owner joins
//! exact process and CDP targets. No launch, recovery, activation or state writes.

use agent_browser_service_model::{
    BrowserSessionState, ManagedBrowserInstance, RemoteViewApplicationWindows,
};
use futures_util::{stream, StreamExt};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

#[derive(Default)]
enum Observation {
    Absent,
    #[default]
    Unknown,
    Live {
        targets: Value,
        processes: BTreeSet<u32>,
    },
}

async fn targets(browser: &ManagedBrowserInstance) -> Observation {
    use crate::process_identity::{observe_process, ProcessObservation};
    let before = observe_process(browser.pid);
    match before {
        ProcessObservation::Missing => return Observation::Absent,
        ProcessObservation::Failed { .. } => return Observation::Unknown,
        ProcessObservation::Observed(_) => {}
    }
    if browser.cdp_endpoint.is_empty() {
        return match crate::native::browser_session_runtime::manual_browser_is_live(browser) {
            Ok(true) => Observation::Live {
                targets: json!({"targetInfos":[{"targetId":format!("manual-window:{}",browser.id),
                    "type":"manual_window","title":"Manual login"}]}),
                processes: BTreeSet::from([browser.pid]),
            },
            Ok(false) => Observation::Absent,
            Err(_) => Observation::Unknown,
        };
    }
    let read = async {
        let endpoint = reqwest::Url::parse(&browser.cdp_endpoint).ok()?;
        if !matches!(endpoint.scheme(), "ws" | "wss")
            || !matches!(
                endpoint.host_str(),
                Some("localhost" | "127.0.0.1" | "[::1]")
            )
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
        {
            return None;
        }
        let client = agent_browser_cdp::client::CdpClient::connect(&browser.cdp_endpoint)
            .await
            .ok()?;
        let processes = client
            .send_command_no_params("SystemInfo.getProcessInfo", None)
            .await
            .ok()?;
        let processes = processes["processInfo"].as_array()?;
        if !processes.iter().any(|process| {
            process["type"] == "browser" && process["id"].as_u64() == Some(u64::from(browser.pid))
        }) {
            return None;
        }
        let process_ids = processes
            .iter()
            .filter_map(|process| process["id"].as_u64().and_then(|id| u32::try_from(id).ok()))
            .collect();
        let targets = client
            .send_command_no_params("Target.getTargets", None)
            .await
            .ok()?;
        targets["targetInfos"].as_array()?;
        (observe_process(browser.pid) == before).then_some(Observation::Live {
            targets,
            processes: process_ids,
        })
    };
    read.await.unwrap_or_default()
}

/// Publish fresh descriptive presence without changing TTL, handoffs or custody.
/// Failure and unfinished reads stay unknown. Only a successful exact join makes
/// a tab live; a missing process, target or native window proves display absence.
pub(super) async fn enrich(state: BrowserSessionState, status: &mut Value) {
    let occupied = state
        .tabs
        .values()
        .map(|tab| tab.browser_id.as_str())
        .collect::<BTreeSet<_>>();
    let browsers = state
        .browsers
        .values()
        .filter(|browser| {
            !state.idle_closed_browsers.contains_key(&browser.id)
                && occupied.contains(browser.id.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let desktops = browsers
        .iter()
        .filter_map(|browser| browser.desktop.clone())
        .map(|desktop| ((desktop.desktop_id.clone(), desktop.generation), desktop))
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect::<Vec<_>>();
    let native_started = std::time::Instant::now();
    let mut native =
        super::super::browser_session_remote_view::observe_workspace_desktops(&desktops);
    static CURSOR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let offset = CURSOR.fetch_add(4, std::sync::atomic::Ordering::Relaxed);
    let observed = collect_browsers(browsers, offset).await;
    // Move past the completed batch, rather than advancing only four slots
    // when a healthy large inventory fits many more probes into the budget.
    CURSOR.fetch_add(
        observed.len().saturating_sub(4),
        std::sync::atomic::Ordering::Relaxed,
    );
    native.age_ms = native
        .age_ms
        .saturating_add(native_started.elapsed().as_millis() as u64);
    if native.age_ms >= 30_000 {
        native.windows.clear();
    }
    if let Some(workspace) = status.get_mut("workspace") {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|time| time.as_millis() as u64)
            .unwrap_or_default();
        workspace["observedAtMs"] =
            json!(native.observed_at_ms.map_or(now_ms, |at| at.min(now_ms)));
        workspace["observationAgeMs"] = json!(native.age_ms);
    }
    for browser in state.browsers.values() {
        let sleeping = state.idle_closed_browsers.contains_key(&browser.id);
        let observation = observed.get(&browser.id).unwrap_or(&Observation::Unknown);
        let presence = if sleeping {
            "absent"
        } else {
            browser_presence(browser, observation, &native.windows)
        };
        publish(&state, browser, observation, presence, status);
    }
}

async fn collect_browsers(
    mut browsers: Vec<ManagedBrowserInstance>,
    offset: usize,
) -> BTreeMap<String, Observation> {
    if !browsers.is_empty() {
        let count = browsers.len();
        browsers.rotate_left(offset % count);
    }
    let observations = stream::iter(browsers)
        .map(|browser| async move {
            let result = tokio::time::timeout(Duration::from_millis(600), targets(&browser))
                .await
                .unwrap_or_default();
            (browser.id, result)
        })
        .buffer_unordered(4);
    tokio::pin!(observations);
    let mut result = BTreeMap::new();
    let _ = tokio::time::timeout(Duration::from_millis(800), async {
        while let Some((id, observed)) = observations.next().await {
            result.insert(id, observed);
        }
    })
    .await;
    result
}

fn browser_presence(
    browser: &ManagedBrowserInstance,
    observation: &Observation,
    native: &BTreeMap<(String, u64), Option<RemoteViewApplicationWindows>>,
) -> &'static str {
    let processes = match observation {
        Observation::Absent => return "absent",
        Observation::Unknown => return "unknown",
        Observation::Live { processes, .. } => processes,
    };
    let Some(desktop) = &browser.desktop else {
        return "live";
    };
    match native.get(&(desktop.desktop_id.clone(), desktop.generation)) {
        Some(Some(windows))
            if windows.desktop_id != desktop.desktop_id
                || windows.generation != desktop.generation =>
        {
            "unknown"
        }
        Some(Some(windows))
            if windows
                .windows
                .iter()
                .any(|window| window.pid.is_some_and(|pid| processes.contains(&pid))) =>
        {
            "live"
        }
        Some(Some(windows)) if windows.windows.iter().any(|window| window.pid.is_none()) => {
            "unknown"
        }
        Some(_) => "absent",
        None => "unknown",
    }
}

fn publish(
    state: &BrowserSessionState,
    browser: &ManagedBrowserInstance,
    observation: &Observation,
    presence: &str,
    status: &mut Value,
) {
    let Some(published) = status
        .pointer_mut("/workspace/browsers")
        .and_then(|browsers| browsers.get_mut(&browser.id))
    else {
        return;
    };
    if published["id"] != browser.id || published["pid"].as_u64() != Some(u64::from(browser.pid)) {
        return;
    }
    published["presence"] = json!({"state": presence, "source":
        if browser.cdp_endpoint.is_empty() { "remote-view-and-process" }
        else if browser.desktop.is_some() { "remote-view-and-cdp" } else { "cdp" }});
    if browser.cdp_endpoint.is_empty() {
        published["mode"] = json!("manual_login");
    }
    let targets = match observation {
        Observation::Live { targets, .. } => targets["targetInfos"].as_array(),
        _ => None,
    };
    for tab in state
        .tabs
        .values()
        .filter(|tab| tab.browser_id == browser.id)
    {
        if state
            .sessions
            .get(&tab.session_id)
            .is_none_or(|session| session.browser_id != browser.id)
        {
            continue;
        }
        let Some(published) = status
            .pointer_mut("/workspace/tabs")
            .and_then(|tabs| tabs.get_mut(&tab.id))
        else {
            continue;
        };
        if published["browserId"] != browser.id
            || published["sessionId"] != tab.session_id
            || published["targetId"] != tab.target_id
        {
            continue;
        }
        let target = targets.and_then(|targets| {
            targets.iter().find(|target| {
                target["targetId"] == tab.target_id
                    && (target["type"] == "page"
                        || (browser.cdp_endpoint.is_empty() && target["type"] == "manual_window"))
            })
        });
        let tab_presence = if presence == "live" && target.is_none() {
            "absent"
        } else {
            presence
        };
        published["presence"] = json!({"state": tab_presence});
        if let Some(target) = target {
            if let Some(title) = target["title"].as_str() {
                published["title"] = Value::String(title.to_owned());
            }
            if let Some(url) = target["url"].as_str() {
                published["url"] = Value::String(url.to_owned());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::{ManagedBrowserSession, ManagedBrowserTab};
    use serde_json::json;

    #[tokio::test]
    async fn live_workspace_title_observation_reads_only_exact_targets() {
        use futures_util::{SinkExt, StreamExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "ws://{}/devtools/browser/synthetic",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            let request: Value =
                serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(request["method"], "SystemInfo.getProcessInfo");
            ws.send(tokio_tungstenite::tungstenite::Message::Text(json!({"id":request["id"],"result":{"processInfo":[{"type":"browser","id":std::process::id()}]}}).to_string())).await.unwrap();
            let request: Value =
                serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(
                request["method"], "Target.getTargets",
                "inventory must never activate a tab or evaluate page scripts"
            );
            ws.send(tokio_tungstenite::tungstenite::Message::Text(json!({"id":request["id"],"result":{"targetInfos":[
                {"targetId":"target", "type":"page", "title":"P237 installed dev inspection", "url":"data:text/html,<title>P237 installed dev inspection</title>"},
                {"targetId":"other", "type":"page", "title":"Other browser page", "url":"https://other.test"}
            ]}}).to_string())).await.unwrap();
        });
        let mut state = BrowserSessionState::default();
        state.browsers.insert(
            "browser".into(),
            ManagedBrowserInstance {
                id: "browser".into(),
                profile_id: "profile".into(),
                pid: std::process::id(),
                cdp_endpoint: endpoint,
                desktop: None,
                active_session_ids: vec!["session".into()],
            },
        );
        // Unavailable peers must not terminate collection of healthy browsers.
        state.browsers.insert(
            "a-unavailable".into(),
            ManagedBrowserInstance {
                id: "a-unavailable".into(),
                profile_id: "other".into(),
                pid: 0,
                cdp_endpoint: "ws://127.0.0.1:1".into(),
                desktop: None,
                active_session_ids: vec![],
            },
        );
        state.tabs.insert(
            "missing".into(),
            ManagedBrowserTab {
                id: "missing".into(),
                target_id: "missing".into(),
                browser_id: "a-unavailable".into(),
                session_id: "missing".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
            },
        );
        state.sessions.insert(
            "session".into(),
            ManagedBrowserSession {
                id: "session".into(),
                name: "task".into(),
                profile_id: "profile".into(),
                browser_id: "browser".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
                expires_at_ms: 0,
                current_tab_id: Some("tab".into()),
            },
        );
        state.tabs.insert(
            "tab".into(),
            ManagedBrowserTab {
                id: "tab".into(),
                target_id: "target".into(),
                browser_id: "browser".into(),
                session_id: "session".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
            },
        );
        let mut status = json!({"workspace":{"browsers":{"browser":{"id":"browser","pid":std::process::id()}},"tabs":{
            "tab":{"browserId":"browser","sessionId":"session","targetId":"target","url":"old"},
            "foreign":{"browserId":"foreign","sessionId":"session","targetId":"target","title":"Preserved"}
        }}});
        enrich(state.clone(), &mut status).await;
        server.await.unwrap();
        assert_eq!(
            status["workspace"]["browsers"]["browser"]["presence"]["state"],
            "live"
        );
        assert_eq!(
            status["workspace"]["tabs"]["tab"]["presence"]["state"],
            "live"
        );
        assert_eq!(
            status["workspace"]["tabs"]["tab"]["title"],
            "P237 installed dev inspection"
        );
        assert!(status["workspace"]["tabs"]["tab"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:"));
        assert_eq!(status["workspace"]["tabs"]["foreign"]["title"], "Preserved");
        assert!(
            status["workspace"]["tabs"].get("missing").is_none(),
            "observation cannot invent published tabs"
        );
        let before = status.clone();
        status["workspace"]["tabs"]["tab"]["targetId"] = json!("replacement");
        publish(
            &state,
            &state.browsers["browser"],
            &Observation::Live {
                targets: json!({"targetInfos":[{"targetId":"target","type":"page","title":"Wrong"}]}),
                processes: BTreeSet::new(),
            },
            "live",
            &mut status,
        );
        assert_eq!(
            status["workspace"]["tabs"]["tab"]["title"],
            before["workspace"]["tabs"]["tab"]["title"]
        );
    }
    #[test]
    fn workspace_presence_requires_native_window_and_exact_browser_family() {
        let mut browser = ManagedBrowserInstance {
            id: "browser".into(),
            profile_id: "profile".into(),
            pid: 42,
            cdp_endpoint: "ws://127.0.0.1:1".into(),
            desktop: Some(agent_browser_service_model::RemoteViewFixedDesktop {
                desktop_id: "owned".into(),
                generation: 7,
                friendly_route_label: "Synthetic".into(),
            }),
            active_session_ids: vec![],
        };
        let observed = Observation::Live {
            targets: json!({"targetInfos":[]}),
            processes: BTreeSet::from([42, 43]),
        };
        let key = ("owned".into(), 7);
        let mut native = BTreeMap::new();
        assert_eq!(browser_presence(&browser, &observed, &native), "unknown");
        native.insert(key.clone(), None);
        assert_eq!(browser_presence(&browser, &observed, &native), "absent");
        let mut windows: RemoteViewApplicationWindows = serde_json::from_value(json!({"schemaVersion":1,"operation":"desktop.windows","desktopId":"owned","generation":7,"viewingGeneration":1,"windows":[]})).unwrap();
        native.insert(key.clone(), Some(windows.clone()));
        assert_eq!(
            browser_presence(&browser, &observed, &native),
            "absent",
            "empty desktop is not a browser"
        );
        windows.windows.push(serde_json::from_value(json!({"id":1,"pid":99,"title":"Chrome","class":"Chromium","x":0,"y":0,"width":100,"height":100,"active":true,"maximized":true})).unwrap());
        native.insert(key.clone(), Some(windows.clone()));
        assert_eq!(
            browser_presence(&browser, &observed, &native),
            "absent",
            "a peer browser window cannot satisfy this browser's identity"
        );
        windows.windows[0].pid = None;
        native.insert(key.clone(), Some(windows.clone()));
        assert_eq!(
            browser_presence(&browser, &observed, &native),
            "unknown",
            "a window without process attribution cannot prove browser absence"
        );
        windows.windows[0].pid = Some(43);
        native.insert(key.clone(), Some(windows.clone()));
        assert_eq!(browser_presence(&browser, &observed, &native), "live");
        windows.generation = 8;
        native.insert(key, Some(windows));
        assert_eq!(
            browser_presence(&browser, &observed, &native),
            "unknown",
            "stale provider generation is not absence proof"
        );
        assert_eq!(
            browser_presence(&browser, &Observation::Absent, &native),
            "absent"
        );
        browser.desktop = None;
        assert_eq!(
            browser_presence(&browser, &observed, &BTreeMap::new()),
            "live",
            "an observed headless browser needs no invented native desktop"
        );
    }

    #[test]
    fn workspace_presence_retains_tasks_but_marks_missing_targets_absent() {
        let mut state = BrowserSessionState::default();
        let browser = ManagedBrowserInstance {
            id: "browser".into(),
            profile_id: "profile".into(),
            pid: 42,
            cdp_endpoint: "ws://127.0.0.1:1".into(),
            desktop: None,
            active_session_ids: vec!["session".into()],
        };
        state.browsers.insert(browser.id.clone(), browser.clone());
        state.sessions.insert(
            "session".into(),
            ManagedBrowserSession {
                id: "session".into(),
                name: "task".into(),
                profile_id: "profile".into(),
                browser_id: "browser".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
                expires_at_ms: 2,
                current_tab_id: Some("tab".into()),
            },
        );
        state.tabs.insert(
            "tab".into(),
            ManagedBrowserTab {
                id: "tab".into(),
                target_id: "exact".into(),
                browser_id: "browser".into(),
                session_id: "session".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
            },
        );
        let mut status = json!({"workspace":{"browsers":{"browser":{"id":"browser","pid":42}},"tabs":{"tab":{"browserId":"browser","sessionId":"session","targetId":"exact","url":"retained"}},"handoffs":{"link":{"handoffId":"link"}}}});
        let original = state.clone();
        let missing = Observation::Live {
            targets: json!({"targetInfos":[{"targetId":"peer","type":"page"}]}),
            processes: BTreeSet::from([42]),
        };
        publish(&state, &browser, &missing, "live", &mut status);
        assert_eq!(
            status["workspace"]["browsers"]["browser"]["presence"]["state"],
            "live"
        );
        assert_eq!(
            status["workspace"]["tabs"]["tab"]["presence"]["state"],
            "absent"
        );
        assert_eq!(status["workspace"]["tabs"]["tab"]["url"], "retained");
        publish(
            &state,
            &browser,
            &Observation::Unknown,
            "unknown",
            &mut status,
        );
        assert_eq!(
            status["workspace"]["tabs"]["tab"]["presence"]["state"],
            "unknown"
        );
        let restored = Observation::Live {
            targets: json!({"targetInfos":[{"targetId":"exact","type":"page","title":"Restored"}]}),
            processes: BTreeSet::from([42]),
        };
        publish(&state, &browser, &restored, "live", &mut status);
        assert_eq!(
            status["workspace"]["tabs"]["tab"]["presence"]["state"],
            "live"
        );
        assert_eq!(status["workspace"]["handoffs"]["link"]["handoffId"], "link");
        assert_eq!(
            state, original,
            "observation must not rewrite TTL, tasks or custody"
        );
    }
    #[tokio::test]
    async fn workspace_collection_rotates_past_slow_browsers() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "ws://{}/devtools/browser/slow",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move {
            let mut clients = tokio::task::JoinSet::new();
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                clients.spawn(async move {
                    let _websocket = tokio_tungstenite::accept_async(socket).await.unwrap();
                    std::future::pending::<()>().await;
                });
            }
        });
        let mut browsers = (0..8)
            .map(|index| ManagedBrowserInstance {
                id: format!("slow-{index}"),
                profile_id: "p".into(),
                pid: std::process::id(),
                cdp_endpoint: endpoint.clone(),
                desktop: None,
                active_session_ids: vec![],
            })
            .collect::<Vec<_>>();
        browsers.push(ManagedBrowserInstance {
            id: "last-missing".into(),
            profile_id: "p".into(),
            pid: 0,
            cdp_endpoint: endpoint,
            desktop: None,
            active_session_ids: vec![],
        });
        let first = collect_browsers(browsers.clone(), 0).await;
        assert!(!first.contains_key("last-missing"));
        let next = collect_browsers(browsers, 4).await;
        assert!(
            matches!(next.get("last-missing"), Some(Observation::Absent)),
            "later groups must receive a turn even when earlier CDP clients stall"
        );
        server.abort();
        let _ = server.await;
    }
}
