//! Bounded live display metadata for the managed inventory. This does not launch,
//! recover, activate, attach to a page, or change durable browser/session state.

use agent_browser_service_model::{BrowserSessionState, ManagedBrowserInstance};
use futures_util::{stream, StreamExt};
use serde_json::Value;
use std::time::Duration;

async fn targets(browser: &ManagedBrowserInstance) -> Option<Value> {
    use crate::process_identity::{observe_process, ProcessObservation};
    let before = observe_process(browser.pid);
    if !matches!(before, ProcessObservation::Observed(_)) {
        return None;
    }
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
    let result = client
        .send_command_no_params("Target.getTargets", None)
        .await
        .ok()?;
    (observe_process(browser.pid) == before).then_some(result)
}

/// Join observed metadata only to exact owner browser/session/target identities.
/// Missing/slow targets retain their stored URL and an unknown title. Inventory
/// evidence is descriptive and never grants lifecycle or viewer authority.
pub(super) async fn enrich(state: BrowserSessionState, status: &mut Value) {
    let browsers = state
        .browsers
        .values()
        .filter(|browser| {
            !state.idle_closed_browsers.contains_key(&browser.id)
                && state.tabs.values().any(|tab| tab.browser_id == browser.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let observations = stream::iter(browsers)
        .map(|browser| async move {
            let result = tokio::time::timeout(Duration::from_millis(600), targets(&browser))
                .await
                .ok()
                .flatten();
            (browser, result)
        })
        .buffer_unordered(4);
    tokio::pin!(observations);
    // One slow browser must not hold the status request indefinitely. Drop all
    // unfinished clients at the deadline; CdpClient releases its background tasks.
    let _ = tokio::time::timeout(Duration::from_millis(800), async {
        while let Some((browser, observed)) = observations.next().await {
            if let Some(observed) = observed {
                join(&state, &browser, &observed, status);
            }
        }
    })
    .await;
}

fn join(
    state: &BrowserSessionState,
    browser: &ManagedBrowserInstance,
    observed: &Value,
    status: &mut Value,
) {
    let Some(targets) = observed["targetInfos"].as_array() else {
        return;
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
            .get_mut("workspace")
            .and_then(|workspace| workspace.get_mut("tabs"))
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
        let Some(target) = targets
            .iter()
            .find(|target| target["targetId"] == tab.target_id && target["type"] == "page")
        else {
            continue;
        };
        if let Some(title) = target["title"].as_str() {
            published["title"] = Value::String(title.to_owned());
        }
        if let Some(url) = target["url"].as_str() {
            published["url"] = Value::String(url.to_owned());
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
        let mut status = json!({"workspace":{"tabs":{
            "tab":{"browserId":"browser","sessionId":"session","targetId":"target","url":"old"},
            "foreign":{"browserId":"foreign","sessionId":"session","targetId":"target","title":"Preserved"}
        }}});
        enrich(state.clone(), &mut status).await;
        server.await.unwrap();
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
        join(
            &state,
            &state.browsers["browser"],
            &json!({"targetInfos":[{"targetId":"target","type":"page","title":"Wrong"}]}),
            &mut status,
        );
        assert_eq!(
            status["workspace"]["tabs"]["tab"]["title"],
            before["workspace"]["tabs"]["tab"]["title"]
        );
    }
}
