//! Typed renderer-crash correlation and durable service lifecycle projection.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::cdp::types::CdpEvent;
use super::service_model::{ServiceEvent, ServiceEventKind, TabLifecycle};
use super::service_store::ServiceStateRepository;

/// A renderer-crash event observed on the daemon CDP stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RendererCrashSignal {
    pub method: String,
    pub reason: Option<String>,
    pub status: Option<String>,
    pub error_code: Option<i64>,
    pub target_id: Option<String>,
    pub page_session_id: Option<String>,
    pub observed_at: String,
    pub source: String,
}

impl RendererCrashSignal {
    pub(crate) fn from_cdp_event(event: &CdpEvent) -> Option<Self> {
        if event.method != "Inspector.targetCrashed" {
            return None;
        }
        Some(Self {
            method: event.method.clone(),
            reason: optional_param_string(&event.params, "reason"),
            status: optional_param_string(&event.params, "status"),
            error_code: event.params.get("errorCode").and_then(Value::as_i64),
            target_id: optional_param_string(&event.params, "targetId"),
            page_session_id: event.session_id.clone(),
            observed_at: current_timestamp(),
            source: "cdp_event_stream".to_string(),
        })
    }
}

/// Command and retained browser subject that can own a crash observation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RendererCrashCommandContext {
    pub action: String,
    pub request_id: String,
    pub local_principal: Option<String>,
    pub service_name: Option<String>,
    pub agent_name: Option<String>,
    pub task_name: Option<String>,
    pub daemon_session: String,
    pub requested_profile: Option<String>,
    pub detected_profile: Option<String>,
    pub browser_id: Option<String>,
    pub pid: Option<u32>,
    pub endpoint: Option<String>,
    pub browser_build: Option<String>,
    pub stderr_path: Option<String>,
    pub target_id: Option<String>,
    pub page_session_id: Option<String>,
}

/// One crash correlated to the command and service subject that owned it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RendererCrashObservation {
    pub correlation_id: String,
    pub signal: RendererCrashSignal,
    pub command: RendererCrashCommandContext,
}

/// Durable identifiers produced by one atomic crash projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RendererCrashPersistence {
    pub tab_id: Option<String>,
    pub event_id: String,
    pub incident_id: String,
    pub duplicate: bool,
}

/// Build the stable command envelope for a correlated renderer crash.
pub(crate) fn renderer_crash_error_response(
    id: &str,
    observation: RendererCrashObservation,
    persistence: Result<RendererCrashPersistence, String>,
) -> Value {
    let (persistence, persistence_error) = match persistence {
        Ok(persistence) => (json!(persistence), Value::Null),
        Err(error) => (Value::Null, json!(error)),
    };
    json!({
        "id": id,
        "success": false,
        "code": "target_crashed",
        "error": "The active renderer target crashed while the command was running",
        "data": {
            "crash": observation,
            "persistence": persistence,
            "persistenceError": persistence_error,
        },
    })
}

pub(crate) fn correlate_renderer_crash(
    signal: RendererCrashSignal,
    context: &RendererCrashCommandContext,
) -> Option<RendererCrashObservation> {
    if matches!(
        context.action.as_str(),
        "close" | "tab_close" | "service_browser_close"
    ) {
        return None;
    }
    if !renderer_crash_targets_context(&signal, context) {
        return None;
    }
    let subject = signal
        .target_id
        .as_deref()
        .or(signal.page_session_id.as_deref())
        .unwrap_or("unknown");
    let request_subject = if context.request_id.is_empty() {
        signal.observed_at.as_str()
    } else {
        context.request_id.as_str()
    };
    Some(RendererCrashObservation {
        correlation_id: format!("{}:{}:{}", context.daemon_session, request_subject, subject),
        signal,
        command: context.clone(),
    })
}

pub(crate) fn renderer_crash_targets_context(
    signal: &RendererCrashSignal,
    context: &RendererCrashCommandContext,
) -> bool {
    match (&signal.target_id, &context.target_id) {
        (Some(observed), Some(active)) => observed == active,
        (Some(_), None) | (None, Some(_)) => false,
        (None, None) => {
            signal.page_session_id.is_some() && signal.page_session_id == context.page_session_id
        }
    }
}

/// Wait for the exact renderer crash that owns `context` without consuming the
/// daemon's primary event receiver. Broadcast subscription keeps ordinary CDP
/// event processing intact while allowing an in-flight command to fail fast.
pub(crate) async fn wait_for_renderer_crash(
    receiver: &mut tokio::sync::broadcast::Receiver<CdpEvent>,
    context: &RendererCrashCommandContext,
) -> RendererCrashObservation {
    loop {
        match receiver.recv().await {
            Ok(event) => {
                if let Some(observation) = RendererCrashSignal::from_cdp_event(&event)
                    .and_then(|signal| correlate_renderer_crash(signal, context))
                {
                    return observation;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                std::future::pending::<()>().await;
            }
        }
    }
}

pub(crate) fn persist_renderer_crash_in_repository<R: ServiceStateRepository>(
    repository: &R,
    observation: &RendererCrashObservation,
) -> Result<RendererCrashPersistence, String> {
    repository.mutate(|state| {
        let tab_id = state
            .tabs
            .iter()
            .find(|(_, tab)| {
                observation.signal.target_id.is_some()
                    && tab.target_id == observation.signal.target_id
            })
            .or_else(|| {
                state.tabs.iter().find(|(_, tab)| {
                    observation.signal.target_id.is_none()
                        && tab.session_id.as_deref()
                            == Some(observation.command.daemon_session.as_str())
                        && tab.service_tab_handle.as_ref().is_some_and(|handle| {
                            handle.session_name.as_deref()
                                == Some(observation.command.daemon_session.as_str())
                        })
                })
            })
            .map(|(id, _)| id.clone());
        let browser_id = tab_id
            .as_ref()
            .and_then(|id| state.tabs.get(id))
            .map(|tab| tab.browser_id.clone())
            .or_else(|| observation.command.browser_id.clone())
            .unwrap_or_else(|| format!("session:{}", observation.command.daemon_session));
        let incident_subject = tab_id
            .as_deref()
            .or(observation.signal.target_id.as_deref())
            .or(observation.signal.page_session_id.as_deref())
            .unwrap_or("unknown");
        let incident_id = format!("renderer-crash:{browser_id}:{incident_subject}");
        let event_id = format!("renderer-crash:{}", observation.correlation_id);

        if let Some(tab_id) = tab_id.as_ref() {
            if let Some(tab) = state.tabs.get_mut(tab_id) {
                tab.lifecycle = TabLifecycle::Crashed;
                if let Some(handle) = tab.service_tab_handle.as_mut() {
                    handle.valid = false;
                    handle.stale_reason = Some("target_crashed".to_string());
                }
            }
        }

        let duplicate = state.events.iter().any(|event| event.id == event_id);
        if !duplicate {
            let mut details = serde_json::to_value(observation)
                .map_err(|err| format!("Failed to serialize renderer crash evidence: {err}"))?;
            let object = details.as_object_mut().ok_or_else(|| {
                "Renderer crash evidence did not serialize as an object".to_string()
            })?;
            object.insert("incidentId".to_string(), json!(incident_id));
            object.insert("tabId".to_string(), json!(tab_id));
            object.insert("result".to_string(), json!("target_crashed"));
            state.events.push(ServiceEvent {
                id: event_id.clone(),
                timestamp: observation.signal.observed_at.clone(),
                kind: ServiceEventKind::TabLifecycleChanged,
                message: format!(
                    "Renderer crashed for {} while running '{}'.",
                    incident_subject, observation.command.action
                ),
                browser_id: Some(browser_id),
                profile_id: observation
                    .command
                    .detected_profile
                    .clone()
                    .or_else(|| observation.command.requested_profile.clone()),
                session_id: Some(observation.command.daemon_session.clone()),
                service_name: observation.command.service_name.clone(),
                agent_name: observation.command.agent_name.clone(),
                task_name: observation.command.task_name.clone(),
                details: Some(details),
                ..ServiceEvent::default()
            });
            if state.events.len() > 100 {
                let excess = state.events.len() - 100;
                state.events.drain(0..excess);
            }
        }

        Ok(RendererCrashPersistence {
            tab_id,
            event_id,
            incident_id,
            duplicate,
        })
    })
}

fn optional_param_string(params: &Value, name: &str) -> Option<String> {
    params
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::native::action_runtime::runtime::DaemonState;
    use crate::native::service_model::{
        BrowserProcess, BrowserTab, ServiceState, ServiceTabHandle, TabLifecycle,
    };
    use crate::native::service_store::{
        JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
    };
    use tokio::sync::broadcast;

    fn crash_event(method: &str) -> CdpEvent {
        CdpEvent {
            method: method.to_string(),
            params: json!({
                "targetId": "target-a",
                "status": "crashed",
                "errorCode": 139,
                "reason": "renderer process exited"
            }),
            session_id: Some("page-session-a".to_string()),
        }
    }

    fn context() -> RendererCrashCommandContext {
        RendererCrashCommandContext {
            action: "snapshot".to_string(),
            request_id: "request-1".to_string(),
            local_principal: Some("local:test".to_string()),
            service_name: Some("messages".to_string()),
            agent_name: Some("tester".to_string()),
            task_name: Some("capture".to_string()),
            daemon_session: "session-a".to_string(),
            requested_profile: Some("profile-a".to_string()),
            detected_profile: Some("profile-a".to_string()),
            browser_id: Some("session:session-a".to_string()),
            pid: Some(4242),
            endpoint: Some("http://127.0.0.1:9222".to_string()),
            browser_build: Some("chrome".to_string()),
            stderr_path: Some("/tmp/chrome.stderr.log".to_string()),
            target_id: Some("target-a".to_string()),
            page_session_id: Some("page-session-a".to_string()),
        }
    }

    #[test]
    fn inspector_target_crashed_is_the_only_crash_signal() {
        let signal = RendererCrashSignal::from_cdp_event(&crash_event("Inspector.targetCrashed"))
            .expect("Inspector.targetCrashed should parse");
        assert_eq!(signal.target_id.as_deref(), Some("target-a"));
        assert_eq!(signal.page_session_id.as_deref(), Some("page-session-a"));
        assert_eq!(signal.error_code, Some(139));
        assert!(
            RendererCrashSignal::from_cdp_event(&crash_event("Target.targetDestroyed")).is_none()
        );
        assert!(
            RendererCrashSignal::from_cdp_event(&crash_event("Target.detachedFromTarget"))
                .is_none()
        );
    }

    #[test]
    fn correlation_is_target_stable_and_excludes_explicit_close() {
        let signal = RendererCrashSignal::from_cdp_event(&crash_event("Inspector.targetCrashed"))
            .expect("crash signal");
        let observation = correlate_renderer_crash(signal.clone(), &context())
            .expect("matching active target should correlate");
        assert_eq!(observation.correlation_id, "session-a:request-1:target-a");

        let mut unrelated = context();
        unrelated.target_id = Some("target-b".to_string());
        unrelated.page_session_id = Some("page-session-b".to_string());
        assert!(correlate_renderer_crash(signal.clone(), &unrelated).is_none());

        let mut explicit_close = context();
        explicit_close.action = "tab_close".to_string();
        assert!(correlate_renderer_crash(signal, &explicit_close).is_none());
    }

    #[test]
    fn crash_projection_is_atomic_deduplicated_and_tab_scoped() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-renderer-crash-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temp dir");
        let path = root.join("state.json");
        let repository = LockedServiceStateRepository::new(JsonServiceStateStore::new(&path));
        repository
            .mutate(|state| {
                *state = ServiceState {
                    browsers: BTreeMap::from([(
                        "session:session-a".to_string(),
                        BrowserProcess {
                            id: "session:session-a".to_string(),
                            active_session_ids: vec!["session-a".to_string()],
                            ..BrowserProcess::default()
                        },
                    )]),
                    tabs: BTreeMap::from([
                        (
                            "target:target-a".to_string(),
                            BrowserTab {
                                id: "target:target-a".to_string(),
                                browser_id: "session:session-a".to_string(),
                                target_id: Some("target-a".to_string()),
                                session_id: Some("session-a".to_string()),
                                lifecycle: TabLifecycle::Ready,
                                service_tab_handle: Some(ServiceTabHandle {
                                    valid: true,
                                    ..ServiceTabHandle::default()
                                }),
                                ..BrowserTab::default()
                            },
                        ),
                        (
                            "target:target-b".to_string(),
                            BrowserTab {
                                id: "target:target-b".to_string(),
                                browser_id: "session:session-a".to_string(),
                                target_id: Some("target-b".to_string()),
                                session_id: Some("session-a".to_string()),
                                lifecycle: TabLifecycle::Ready,
                                ..BrowserTab::default()
                            },
                        ),
                    ]),
                    ..ServiceState::default()
                };
                Ok(())
            })
            .expect("seed state");

        let signal = RendererCrashSignal::from_cdp_event(&crash_event("Inspector.targetCrashed"))
            .expect("crash signal");
        let observation = correlate_renderer_crash(signal, &context()).expect("observation");
        let first = persist_renderer_crash_in_repository(&repository, &observation)
            .expect("first projection");
        let second = persist_renderer_crash_in_repository(&repository, &observation)
            .expect("deduplicated projection");
        assert!(!first.duplicate);
        assert!(second.duplicate);

        let state = repository.load_snapshot().expect("persisted state");
        let crashed = &state.tabs["target:target-a"];
        assert_eq!(crashed.lifecycle, TabLifecycle::Crashed);
        assert_eq!(
            crashed
                .service_tab_handle
                .as_ref()
                .map(|handle| handle.valid),
            Some(false)
        );
        assert_eq!(
            crashed
                .service_tab_handle
                .as_ref()
                .and_then(|handle| handle.stale_reason.as_deref()),
            Some("tab_crashed")
        );
        assert_eq!(state.tabs["target:target-b"].lifecycle, TabLifecycle::Ready);
        assert!(state.browsers.contains_key("session:session-a"));
        assert_eq!(state.events.len(), 1);
        assert_eq!(state.incidents.len(), 1);
        assert_eq!(state.incidents[0].event_ids, vec![first.event_id]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn drained_events_carry_typed_crashes_without_reclassifying_target_events() {
        let (tx, rx) = broadcast::channel(8);
        let mut state = DaemonState::new();
        state.event_rx = Some(rx);
        tx.send(crash_event("Target.targetDestroyed"))
            .expect("target event");
        tx.send(crash_event("Inspector.targetCrashed"))
            .expect("crash event");

        let drained = state.drain_cdp_events();
        assert_eq!(drained.renderer_crashes.len(), 1);
        assert_eq!(
            drained.renderer_crashes[0].method,
            "Inspector.targetCrashed"
        );
        assert_eq!(drained.destroyed_targets, vec!["target-a"]);
    }

    #[test]
    fn command_failure_uses_stable_target_crashed_code_and_retains_evidence() {
        let signal = RendererCrashSignal::from_cdp_event(&crash_event("Inspector.targetCrashed"))
            .expect("crash signal");
        let observation = correlate_renderer_crash(signal, &context()).expect("observation");
        let response = renderer_crash_error_response(
            "request-1",
            observation,
            Ok(RendererCrashPersistence {
                tab_id: Some("target:target-a".to_string()),
                event_id: "event-1".to_string(),
                incident_id: "incident-1".to_string(),
                duplicate: false,
            }),
        );
        assert_eq!(response["success"], false);
        assert_eq!(response["code"], "target_crashed");
        assert_eq!(response["data"]["crash"]["command"]["action"], "snapshot");
        assert_eq!(response["data"]["persistence"]["incidentId"], "incident-1");
    }

    #[tokio::test]
    async fn in_flight_observer_ignores_unrelated_events_and_returns_matching_crash() {
        let (tx, mut rx) = broadcast::channel(8);
        let mut unrelated = crash_event("Inspector.targetCrashed");
        unrelated.params["targetId"] = json!("target-b");
        tx.send(crash_event("Target.detachedFromTarget"))
            .expect("detach event");
        tx.send(unrelated).expect("unrelated crash");
        tx.send(crash_event("Inspector.targetCrashed"))
            .expect("matching crash");

        let observation = wait_for_renderer_crash(&mut rx, &context()).await;
        assert_eq!(observation.signal.target_id.as_deref(), Some("target-a"));
        assert_eq!(observation.command.request_id, "request-1");
    }
}
