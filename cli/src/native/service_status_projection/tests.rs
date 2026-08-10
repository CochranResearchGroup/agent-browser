use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use serde_json::{json, Value};

use super::observation::{StatusRoutePresentation, StatusViewStreamObservation};
use super::*;
use crate::native::browser_session_authority::browser_session_authority_snapshot;
use crate::native::service_model::{
    BrowserHost, BrowserProcess, BrowserSession, BrowserTab, ServiceTabHandle, TabLifecycle,
    ViewStream, ViewStreamProvider,
};

#[derive(Debug)]
struct FixedClock;

impl ProjectionClock for FixedClock {
    fn now(&self) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 9, 21, 0, 5).single().unwrap()
    }
}

#[derive(Debug)]
struct CancellationObservationAdapter {
    entered: Arc<tokio::sync::Notify>,
    dropped: Arc<AtomicBool>,
}

#[derive(Debug)]
struct InvocationObservationAdapter {
    invoked: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl StatusObservationSource for InvocationObservationAdapter {
    async fn snapshot(&self, _request: StatusObservationRequest) -> StatusObservationSnapshot {
        self.invoked.store(true, Ordering::SeqCst);
        unavailable_observation()
    }
}

fn valid_launch_config() -> Value {
    json!({
        "defaultBrowserBuild": null,
        "stealthCdpChromiumRequired": false,
        "stealthCdpChromiumReady": true,
        "executablePath": null,
        "executablePathSource": null,
        "executablePathExists": null,
        "browserBuildManifests": {},
        "profileSmoke": {
            "available": false,
            "command": "pnpm test:wsl-windows-chromium-profile-live",
            "reason": "stealthcdp_chromium_not_selected",
            "isWsl": false,
            "executableOnWindowsMount": false,
            "description": "fixed no-launch profile smoke"
        },
        "warnings": []
    })
}

#[async_trait::async_trait]
impl StatusObservationSource for CancellationObservationAdapter {
    async fn snapshot(&self, _request: StatusObservationRequest) -> StatusObservationSnapshot {
        struct DropMarker(Arc<AtomicBool>);

        impl Drop for DropMarker {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let _marker = DropMarker(self.dropped.clone());
        self.entered.notify_one();
        std::future::pending().await
    }
}

fn unavailable_observation() -> StatusObservationSnapshot {
    StatusObservationSnapshot {
        state: StatusObservationState::Unavailable,
        source: StatusObservationSourceKind::InMemory,
        source_host_id: None,
        observed_at: None,
        valid_until: None,
        max_age_ms: 5000,
        manual_browsers_state: StatusObservationComponentState::Unavailable,
        browser_process_state: StatusObservationComponentState::Unavailable,
        errors: vec![StatusObservationError {
            code: StatusObservationErrorCode::ProcessInventoryUnavailable,
            subject: "host".to_string(),
            message: "process inventory was unavailable".to_string(),
        }],
        view_streams: Vec::new(),
        manual_browsers: Vec::new(),
        browser_process_stats: BTreeMap::new(),
    }
}

fn input(state: ServiceState, full_tab_history: bool) -> StatusAuthorityInput {
    StatusAuthorityInput {
        browser_session_authority: browser_session_authority_snapshot(&state),
        service_state: state,
        control_plane: StatusControlPlaneAuthority::try_from(json!({
            "worker_state": "Ready",
            "browser_health": "NotStarted",
            "queue_depth": 0,
            "queue_capacity": 256,
            "waiting_profile_lease_job_count": 0,
            "service_job_timeout_ms": null,
            "service_monitor_interval_ms": null
        }))
        .unwrap(),
        launch_config: StatusLaunchConfiguration::try_from(valid_launch_config()).unwrap(),
        full_tab_history,
    }
}

async fn project(state: ServiceState, full_tab_history: bool) -> ServiceStatusResponse {
    ServiceStatusProjector::new(
        Arc::new(InMemoryStatusObservationAdapter::new(
            unavailable_observation(),
        )),
        Arc::new(FixedClock),
    )
    .project(input(state, full_tab_history))
    .await
    .unwrap()
}

fn closed_tab(id: &str) -> BrowserTab {
    BrowserTab {
        id: id.to_string(),
        lifecycle: TabLifecycle::Closed,
        ..BrowserTab::default()
    }
}

#[tokio::test]
async fn public_interface_caps_only_unreferenced_closed_history_without_mutating_input() {
    let mut state = ServiceState::default();
    for index in 0..55 {
        let id = format!("closed-{index:03}");
        state.tabs.insert(id.clone(), closed_tab(&id));
    }
    state.tabs.insert(
        "ready".to_string(),
        BrowserTab {
            id: "ready".to_string(),
            lifecycle: TabLifecycle::Ready,
            ..BrowserTab::default()
        },
    );
    let mut referenced = closed_tab("closed-referenced");
    referenced.service_tab_handle = Some(ServiceTabHandle {
        valid: false,
        stale_reason: Some("target_closed".to_string()),
        ..ServiceTabHandle::default()
    });
    state.tabs.insert(referenced.id.clone(), referenced);
    let before = state.clone();

    let response = project(state.clone(), false).await;

    assert_eq!(state, before);
    assert_eq!(
        response.service_state["tabs"].as_object().unwrap().len(),
        52
    );
    assert!(response.service_state["tabs"].get("ready").is_some());
    assert!(response.service_state["tabs"]
        .get("closed-referenced")
        .is_some());
    assert_eq!(response.closed_tab_projection.omitted_closed_count, 5);
}

#[tokio::test]
async fn unavailable_observations_are_typed_unknown_and_keep_legacy_manual_array() {
    let response = project(ServiceState::default(), false).await;
    let value = serde_json::to_value(response).unwrap();

    assert_eq!(value["manualBrowsers"], json!([]));
    assert_eq!(
        value["statusProjection"]["observations"]["state"],
        "unavailable"
    );
    assert_eq!(
        value["statusProjection"]["observations"]["observedAt"],
        Value::Null
    );
    assert_eq!(
        value["statusProjection"]["authority"]["projectedAt"],
        "2026-08-09T21:00:05.000Z"
    );
}

#[tokio::test]
async fn invalid_authority_fails_before_observation_projection() {
    let mut state = ServiceState::default();
    state.browsers.insert(
        "browser-key".to_string(),
        BrowserProcess {
            id: "different-id".to_string(),
            ..BrowserProcess::default()
        },
    );
    let error = ServiceStatusProjector::new(
        Arc::new(InMemoryStatusObservationAdapter::new(
            unavailable_observation(),
        )),
        Arc::new(FixedClock),
    )
    .project(input(state, false))
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        ServiceStatusProjectionError::InvalidAuthority(_)
    ));
}

#[tokio::test]
async fn cancelling_projection_drops_the_in_flight_observation_future() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let dropped = Arc::new(AtomicBool::new(false));
    let projector = ServiceStatusProjector::new(
        Arc::new(CancellationObservationAdapter {
            entered: entered.clone(),
            dropped: dropped.clone(),
        }),
        Arc::new(FixedClock),
    );
    let task = tokio::spawn(async move {
        projector
            .project(input(ServiceState::default(), false))
            .await
    });

    entered.notified().await;
    task.abort();
    let _ = task.await;

    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn full_history_keeps_all_closed_tabs_and_session_references() {
    let mut state = ServiceState::default();
    for index in 0..55 {
        let id = format!("closed-{index:03}");
        state.tabs.insert(id.clone(), closed_tab(&id));
    }
    state.sessions.insert(
        "session".to_string(),
        BrowserSession {
            id: "session".to_string(),
            tab_ids: vec!["closed-000".to_string()],
            ..BrowserSession::default()
        },
    );

    let response = project(state, true).await;
    assert_eq!(
        response.service_state["tabs"].as_object().unwrap().len(),
        55
    );
    assert_eq!(response.closed_tab_projection.mode, "full");
}

#[tokio::test]
async fn compatibility_mirror_fills_only_missing_eligible_urls_and_preserves_guacamole_root() {
    let mut state = ServiceState::default();
    state.browsers.insert(
        "browser".to_string(),
        BrowserProcess {
            id: "browser".to_string(),
            host: BrowserHost::RemoteHeaded,
            view_streams: vec![ViewStream {
                id: "rdp".to_string(),
                provider: ViewStreamProvider::RdpGateway,
                url: Some("https://agent-browser.example/guacamole/".to_string()),
                frame_url: Some("/explicit-frame".to_string()),
                ..ViewStream::default()
            }],
            ..BrowserProcess::default()
        },
    );
    let mut observation = unavailable_observation();
    observation.state = StatusObservationState::Complete;
    observation.source_host_id = Some(format!("sha256:{}", "a".repeat(64)));
    observation.observed_at = Some("2026-08-09T21:00:04.000Z".to_string());
    observation.valid_until = Some("2026-08-09T21:00:09.000Z".to_string());
    observation.manual_browsers_state = StatusObservationComponentState::Observed;
    observation.browser_process_state = StatusObservationComponentState::Observed;
    observation.errors.clear();
    observation.view_streams = vec![StatusViewStreamObservation {
        browser_id: "browser".to_string(),
        stream_id: "rdp".to_string(),
        state: StatusViewStreamObservationState::Observed,
        observed_at: observation.observed_at.clone(),
        valid_until: observation.valid_until.clone(),
        max_age_ms: 5000,
        route_presentation: Some(StatusRoutePresentation {
            frame_url: "/configured-client".to_string(),
            external_url: "/configured-client".to_string(),
            source: StatusRoutePresentationSource::ConfiguredClientUrl,
        }),
        display_content: None,
    }];
    let response = ServiceStatusProjector::new(
        Arc::new(InMemoryStatusObservationAdapter::new(observation)),
        Arc::new(FixedClock),
    )
    .project(input(state, false))
    .await
    .unwrap();
    let stream = &response.service_state["browsers"]["browser"]["viewStreams"][0];

    assert_eq!(stream["url"], "https://agent-browser.example/guacamole/");
    assert_eq!(stream["frameUrl"], "/explicit-frame");
    assert_eq!(stream["externalUrl"], "/configured-client");
}

#[test]
fn observation_validation_rejects_invalid_vocabulary_invariants() {
    let mut invalid_host = unavailable_observation();
    invalid_host.source_host_id = Some("local-hostname".to_string());
    assert!(invalid_host.validate().unwrap_err().contains("sha256"));

    let mut invalid_unavailable = unavailable_observation();
    invalid_unavailable.observed_at = Some("2026-08-09T21:00:04.000Z".to_string());
    invalid_unavailable.valid_until = Some("2026-08-09T21:00:09.000Z".to_string());
    assert!(invalid_unavailable
        .validate()
        .unwrap_err()
        .contains("unavailable observations require null timestamps"));

    let mut invalid_stream = unavailable_observation();
    invalid_stream
        .view_streams
        .push(StatusViewStreamObservation {
            browser_id: "browser".to_string(),
            stream_id: "rdp".to_string(),
            state: StatusViewStreamObservationState::TimedOut,
            observed_at: Some("2026-08-09T21:00:04.000Z".to_string()),
            valid_until: Some("2026-08-09T21:00:09.000Z".to_string()),
            max_age_ms: 5_000,
            route_presentation: None,
            display_content: Some(json!({"state": "stale"})),
        });
    assert!(invalid_stream
        .validate()
        .unwrap_err()
        .contains("requires null observation values"));
}

#[test]
fn control_plane_authority_accepts_legacy_lowercase_and_rejects_unknown_vocabulary() {
    let lowercase = StatusControlPlaneAuthority::try_from(json!({
        "worker_state": "ready",
        "browser_health": "ready",
        "queue_depth": 0,
        "queue_capacity": 1,
        "waiting_profile_lease_job_count": 0,
        "service_job_timeout_ms": null,
        "service_monitor_interval_ms": null
    }))
    .unwrap();
    assert_eq!(lowercase.worker_state, StatusWorkerState::Ready);
    assert_eq!(lowercase.browser_health, StatusBrowserHealth::Ready);

    for (field, value) in [
        ("worker_state", "unknown_worker"),
        ("browser_health", "unknown_browser"),
    ] {
        let mut input = json!({
            "worker_state": "Ready",
            "browser_health": "NotStarted",
            "queue_depth": 0,
            "queue_capacity": 1,
            "waiting_profile_lease_job_count": 0,
            "service_job_timeout_ms": null,
            "service_monitor_interval_ms": null
        });
        input[field] = json!(value);
        let invalid_vocabulary = StatusControlPlaneAuthority::try_from(input).unwrap_err();
        assert!(invalid_vocabulary
            .to_string()
            .contains("invalid control-plane snapshot"));
    }
}

#[test]
fn control_plane_authority_rejects_queue_invariants() {
    let invalid_queue = StatusControlPlaneAuthority::try_from(json!({
        "worker_state": "Ready",
        "browser_health": "NotStarted",
        "queue_depth": 2,
        "queue_capacity": 1,
        "waiting_profile_lease_job_count": 0,
        "service_job_timeout_ms": null,
        "service_monitor_interval_ms": null
    }))
    .unwrap_err();
    assert!(invalid_queue
        .to_string()
        .contains("queueDepth exceeds queueCapacity"));
}

#[test]
fn launch_configuration_requires_all_nine_typed_fields() {
    let required = [
        "defaultBrowserBuild",
        "stealthCdpChromiumRequired",
        "stealthCdpChromiumReady",
        "executablePath",
        "executablePathSource",
        "executablePathExists",
        "browserBuildManifests",
        "profileSmoke",
        "warnings",
    ];
    for field in required {
        let mut value = valid_launch_config();
        value.as_object_mut().unwrap().remove(field);
        let error = StatusLaunchConfiguration::try_from(value).unwrap_err();
        assert!(error.to_string().contains(field), "{field}: {error}");
    }

    for (field, wrong_value) in [
        ("defaultBrowserBuild", json!(7)),
        ("stealthCdpChromiumRequired", json!("false")),
        ("executablePathExists", json!("unknown")),
        ("browserBuildManifests", json!([])),
        ("profileSmoke", json!([])),
        ("warnings", json!({})),
    ] {
        let mut value = valid_launch_config();
        value[field] = wrong_value;
        let error = StatusLaunchConfiguration::try_from(value).unwrap_err();
        assert!(
            error.to_string().contains("invalid launchConfig"),
            "{field}: {error}"
        );
    }
}

#[test]
fn launch_configuration_preserves_schema_allowed_additional_properties() {
    let mut value = valid_launch_config();
    value["futureDiagnostic"] = json!({"enabled": true});
    let typed = StatusLaunchConfiguration::try_from(value).unwrap();
    let serialized = serde_json::to_value(typed).unwrap();

    assert_eq!(serialized["futureDiagnostic"]["enabled"], true);
}

#[test]
fn legacy_ingress_defaults_only_an_absent_launch_configuration() {
    let absent = launch_configuration_from_status_command(&json!({
        "action": "service_status"
    }));
    let typed = StatusLaunchConfiguration::try_from(absent.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), absent);
    assert_eq!(absent["defaultBrowserBuild"], Value::Null);
    assert_eq!(absent["stealthCdpChromiumRequired"], false);
    assert_eq!(absent["stealthCdpChromiumReady"], true);
    assert_eq!(
        absent["profileSmoke"]["reason"],
        "stealthcdp_chromium_not_selected"
    );

    let malformed = launch_configuration_from_status_command(&json!({
        "action": "service_status",
        "launchConfig": {}
    }));
    assert_eq!(malformed, json!({}));
    assert!(StatusLaunchConfiguration::try_from(malformed).is_err());
}

#[tokio::test]
async fn invalid_launch_configuration_fails_before_observation() {
    let invoked = Arc::new(AtomicBool::new(false));
    let projector = ServiceStatusProjector::new(
        Arc::new(InvocationObservationAdapter {
            invoked: invoked.clone(),
        }),
        Arc::new(FixedClock),
    );
    let state = ServiceState::default();
    let mut invalid_launch = valid_launch_config();
    invalid_launch.as_object_mut().unwrap().remove("warnings");

    let error = project_status_with_launch_configuration(
        &projector,
        state.clone(),
        input(state, false).control_plane,
        browser_session_authority_snapshot(&ServiceState::default()),
        invalid_launch,
        false,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("warnings"));
    assert!(!invoked.load(Ordering::SeqCst));
}

#[tokio::test]
async fn fixed_input_real_status_producer_and_fallback_data_seams_match() {
    let response = project(ServiceState::default(), false).await;
    let canonical = serde_json::to_value(response.clone()).unwrap();
    let control_envelope =
        crate::native::control_plane::service_status_result_envelope("fixed-status", Ok(response));
    let direct_http =
        crate::native::stream::service_status_http_fixture(control_envelope.to_string());
    let dashboard_backend = crate::native::stream::service_status_handler_fixture(direct_http);
    let direct_body: Value = serde_json::from_slice(
        crate::native::stream::service_status_http_body_fixture(&dashboard_backend).unwrap(),
    )
    .unwrap();
    let fallback =
        crate::native::stream::service_status_dashboard_cli_fallback_fixture(canonical.to_string());
    let fallback_body: Value = serde_json::from_slice(
        crate::native::stream::service_status_http_body_fixture(&fallback).unwrap(),
    )
    .unwrap();

    assert_eq!(direct_body["data"], canonical);
    assert_eq!(fallback_body, canonical);
    assert_eq!(
        dashboard_backend,
        crate::native::stream::service_status_http_fixture(control_envelope.to_string())
    );
}
