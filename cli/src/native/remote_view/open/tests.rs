use super::shared::*;
use super::*;
use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Default)]
struct FakeClock {
    now_ms: AtomicU64,
}

impl FakeClock {
    fn set(&self, now_ms: u64) {
        self.now_ms.store(now_ms, Ordering::Relaxed);
    }

    fn advance(&self, elapsed_ms: u64) {
        self.now_ms.fetch_add(elapsed_ms, Ordering::Relaxed);
    }
}

impl RouteBoundOpenClock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.load(Ordering::Relaxed)
    }
}

struct ScriptedRuntime {
    events: Arc<Mutex<Vec<&'static str>>>,
    adoption_requests: Arc<Mutex<Vec<AdoptRetainedBrowserRequest>>>,
    observation: RouteBoundBrowserObservation,
    launch_issue: Option<RouteBoundRuntimeIssue>,
    operator_access: Option<Value>,
    visible_window_issue: Option<RouteBoundRuntimeIssue>,
    adoption_observation: Option<RouteBoundBrowserObservation>,
    adoption_issue: Option<RouteBoundRuntimeIssue>,
}

impl ScriptedRuntime {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            adoption_requests: Arc::new(Mutex::new(Vec::new())),
            observation: RouteBoundBrowserObservation {
                browser_present: false,
                browser_pid: None,
                browser_id: "session:test".to_string(),
                session_id: "test".to_string(),
                runtime_profile: None,
                active_target_id: None,
                active_url: None,
                active_title: None,
                pages: Vec::new(),
            },
            launch_issue: None,
            operator_access: Some(json!({ "state": "ready" })),
            visible_window_issue: None,
            adoption_observation: None,
            adoption_issue: Some(RouteBoundRuntimeIssue::EffectFailed {
                operation: "adopt_retained_browser",
                message: "scripted retained browser is unavailable".to_string(),
            }),
        }
    }

    fn effect(&self, name: &'static str, value: Value) -> RouteBoundOpenFuture<'_, Value> {
        Box::pin(async move {
            self.events.lock().unwrap().push(name);
            Ok(value)
        })
    }
}

impl RouteBoundOpenRuntime for ScriptedRuntime {
    fn observe_browser(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation> {
        Box::pin(async move {
            self.events.lock().unwrap().push("observe_browser");
            Ok(self.observation.clone())
        })
    }

    fn adopt_retained_browser(
        &mut self,
        request: AdoptRetainedBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation> {
        Box::pin(async move {
            self.events.lock().unwrap().push("adopt_retained_browser");
            self.adoption_requests.lock().unwrap().push(request);
            if let Some(issue) = self.adoption_issue.clone() {
                return Err(issue);
            }
            let observation = self.adoption_observation.clone().ok_or_else(|| {
                RouteBoundRuntimeIssue::EffectFailed {
                    operation: "adopt_retained_browser",
                    message: "scripted retained browser observation is missing".to_string(),
                }
            })?;
            self.observation = observation.clone();
            Ok(observation)
        })
    }

    fn launch_browser(
        &mut self,
        _request: LaunchBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, LaunchBrowserResult> {
        Box::pin(async move {
            self.events.lock().unwrap().push("launch_browser");
            match self.launch_issue.clone() {
                Some(issue) => Err(issue),
                None => Ok(
                    LaunchBrowserResult::from_compatibility(json!({ "launched": true })).unwrap(),
                ),
            }
        })
    }

    fn launch_cdp_free_browser(
        &mut self,
        _request: LaunchBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, LaunchBrowserResult> {
        Box::pin(async move {
            self.events.lock().unwrap().push("launch_cdp_free_browser");
            match self.launch_issue.clone() {
                Some(issue) => Err(issue),
                None => Ok(LaunchBrowserResult::from_compatibility(json!({
                    "launched": true,
                    "mode": "cdp_free_launch",
                    "browserPid": 4242,
                }))
                .unwrap()),
            }
        })
    }

    fn refresh_targets(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation> {
        Box::pin(async move {
            self.events.lock().unwrap().push("refresh_targets");
            Ok(self.observation.clone())
        })
    }

    fn switch_target(
        &mut self,
        _request: SwitchTargetRequest,
    ) -> RouteBoundOpenFuture<'_, SwitchTargetResult> {
        Box::pin(async move {
            self.effect("switch_target", json!({ "targetId": "target-1" }))
                .await
                .map(|value| SwitchTargetResult::from_compatibility(value).unwrap())
        })
    }

    fn navigate_target(
        &mut self,
        request: NavigateTargetRequest,
    ) -> RouteBoundOpenFuture<'_, NavigateTargetResult> {
        Box::pin(async move {
            self.observation.active_target_id = Some("target-1".to_string());
            self.observation.active_url = Some(request.url.clone());
            self.effect("navigate_target", json!({ "url": request.url }))
                .await
                .map(|value| NavigateTargetResult::from_compatibility(value).unwrap())
        })
    }

    fn open_target(
        &mut self,
        _request: OpenTargetRequest,
    ) -> RouteBoundOpenFuture<'_, OpenTargetResult> {
        Box::pin(async move {
            self.effect("open_target", json!({ "targetId": "target-1" }))
                .await
                .map(|value| OpenTargetResult::from_compatibility(value).unwrap())
        })
    }

    fn focus_target(
        &mut self,
        _request: FocusTargetRequest,
    ) -> RouteBoundOpenFuture<'_, FocusTargetResult> {
        Box::pin(async move {
            self.effect("focus_target", json!({ "focused": true }))
                .await
                .map(|value| FocusTargetResult::from_compatibility(value).unwrap())
        })
    }

    fn close_created_target(
        &mut self,
        _request: CloseCreatedTargetRequest,
    ) -> RouteBoundOpenFuture<'_, CloseCreatedTargetResult> {
        Box::pin(async move {
            self.effect("close_created_target", json!({ "closed": true }))
                .await
                .map(|value| CloseCreatedTargetResult::from_compatibility(value).unwrap())
        })
    }

    fn close_created_browser(
        &mut self,
        _request: CloseCreatedBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, CloseCreatedBrowserResult> {
        Box::pin(async move {
            self.effect("close_created_browser", json!({ "closed": true }))
                .await
                .map(|value| CloseCreatedBrowserResult::from_compatibility(value).unwrap())
        })
    }

    fn checkout_route(
        &mut self,
        _request: CheckoutRouteRequest,
    ) -> RouteBoundOpenFuture<'_, CheckoutRouteResult> {
        Box::pin(async move {
            self.effect("checkout_route", json!({ "status": "ready" }))
                .await
                .map(|value| CheckoutRouteResult::from_compatibility(value).unwrap())
        })
    }

    fn ensure_display_access(
        &mut self,
        _request: DisplayAccessRequest,
    ) -> RouteBoundOpenFuture<'_, DisplayAccessResult> {
        Box::pin(async move {
            self.effect("ensure_display_access", json!({ "state": "ready" }))
                .await
                .map(|value| DisplayAccessResult::from_compatibility(value).unwrap())
        })
    }

    fn stage_visible_window(
        &mut self,
        _request: StageVisibleWindowRequest,
    ) -> RouteBoundOpenFuture<'_, StageVisibleWindowResult> {
        Box::pin(async move {
            self.effect("stage_visible_window", json!({ "state": "staged" }))
                .await
                .map(|value| StageVisibleWindowResult::from_compatibility(value).unwrap())
        })
    }

    fn observe_visible_window(
        &mut self,
        _request: VisibleWindowRequest,
    ) -> RouteBoundOpenFuture<'_, VisibleWindowResult> {
        Box::pin(async move {
            self.events.lock().unwrap().push("observe_visible_window");
            if let Some(issue) = self.visible_window_issue.clone() {
                return Err(issue);
            }
            Ok(VisibleWindowResult::from_compatibility(json!({ "state": "ready" })).unwrap())
        })
    }

    fn observe_operator_access(
        &mut self,
        _request: OperatorAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Option<OperatorAccessResult>> {
        Box::pin(async move {
            self.events.lock().unwrap().push("observe_operator_access");
            Ok(self
                .operator_access
                .clone()
                .map(|value| OperatorAccessResult::from_compatibility(value).unwrap()))
        })
    }
}

#[tokio::test]
async fn focus_success_followed_by_failed_reobservation_never_returns_ready() {
    let mut runtime = ScriptedRuntime::new();
    runtime.visible_window_issue = Some(RouteBoundRuntimeIssue::EffectFailed {
        operation: "observe_visible_window",
        message: "operator_presentation_not_ready: blockers=occluded".to_string(),
    });

    let focus = runtime
        .focus_target(FocusTargetRequest {
            command: FocusTargetCommand::from_compatibility(json!({})).unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(focus.into_value()["focused"], true);
    let error = runtime
        .observe_visible_window(VisibleWindowRequest {
            binding: test_binding(),
            browser_pid: Some(4242),
        })
        .await
        .unwrap_err();

    assert!(error
        .compatibility_message()
        .contains("operator_presentation_not_ready"));
    assert_eq!(
        *runtime.events.lock().unwrap(),
        vec!["focus_target", "observe_visible_window"]
    );
}

fn test_binding() -> RemoteViewRouteBinding {
    RemoteViewRouteBinding {
        route_id: "route-1".to_string(),
        route_pool_entry_id: None,
        display_allocation_id: "display-1".to_string(),
        route_pool_entry_state: None,
        current_route_allocation_id: None,
        display_name: Some(":10".to_string()),
        launch_display_name: Some(":10".to_string()),
        display_isolation: "private_virtual_display".to_string(),
        route_user: None,
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "managed".to_string(),
        connection_id: None,
        connection_name: None,
        frame_url: Some("https://example.test/frame".to_string()),
        external_url: Some("https://example.test/view".to_string()),
        route_descriptor: None,
        readiness: None,
    }
}

#[test]
fn deadline_reserves_one_fifth_with_floor_and_cap() {
    assert_eq!(
        RouteBoundOpenDeadline::from_total_ms(1_000),
        RouteBoundOpenDeadline {
            total_ms: 1_000,
            compensation_reserve_ms: 250,
            forward_deadline_ms: 750,
        }
    );
    assert_eq!(
        RouteBoundOpenDeadline::from_total_ms(100_000),
        RouteBoundOpenDeadline {
            total_ms: 100_000,
            compensation_reserve_ms: 15_000,
            forward_deadline_ms: 85_000,
        }
    );
    assert_eq!(
        RouteBoundOpenDeadline::from_total_ms(200).forward_deadline_ms,
        0
    );
}

#[tokio::test]
async fn scripted_runtime_stops_before_every_mutating_effect_after_cancellation() {
    for phase in 0..9 {
        let clock = Arc::new(FakeClock::default());
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let supervisor =
            RouteBoundOpenSupervisor::with_clock(Some(1_000), Some(cancellation), clock);
        let mut runtime = ScriptedRuntime::new();
        let result = match phase {
            0 => supervisor
                .forward(
                    "launch_browser",
                    runtime.launch_browser(LaunchBrowserRequest {
                        command: LaunchBrowserCommand::from_compatibility(json!({})).unwrap(),
                    }),
                )
                .await
                .map(|_| ()),
            1 => supervisor
                .forward(
                    "switch_target",
                    runtime.switch_target(SwitchTargetRequest {
                        target_id: "target-1".to_string(),
                    }),
                )
                .await
                .map(|_| ()),
            2 => supervisor
                .forward(
                    "navigate_target",
                    runtime.navigate_target(NavigateTargetRequest {
                        url: "https://example.test".to_string(),
                    }),
                )
                .await
                .map(|_| ()),
            3 => supervisor
                .forward(
                    "open_target",
                    runtime.open_target(OpenTargetRequest {
                        command: OpenTargetCommand::from_compatibility(json!({})).unwrap(),
                    }),
                )
                .await
                .map(|_| ()),
            4 => supervisor
                .forward(
                    "focus_target",
                    runtime.focus_target(FocusTargetRequest {
                        command: FocusTargetCommand::from_compatibility(json!({})).unwrap(),
                    }),
                )
                .await
                .map(|_| ()),
            5 => supervisor
                .forward(
                    "close_created_target",
                    runtime.close_created_target(CloseCreatedTargetRequest {
                        target_id: "target-1".to_string(),
                    }),
                )
                .await
                .map(|_| ()),
            6 => supervisor
                .forward(
                    "close_created_browser",
                    runtime.close_created_browser(CloseCreatedBrowserRequest {
                        browser_identity: RouteBoundBrowserIdentity::from_compatibility(json!({}))
                            .unwrap(),
                    }),
                )
                .await
                .map(|_| ()),
            7 => supervisor
                .forward(
                    "checkout_route",
                    runtime.checkout_route(CheckoutRouteRequest {
                        command: CheckoutRouteCommand::from_compatibility(json!({})).unwrap(),
                    }),
                )
                .await
                .map(|_| ()),
            _ => supervisor
                .forward(
                    "ensure_display_access",
                    runtime.ensure_display_access(DisplayAccessRequest {
                        binding: test_binding(),
                    }),
                )
                .await
                .map(|_| ()),
        };
        assert!(matches!(
            result,
            Err(RouteBoundRuntimeIssue::Cancelled { .. })
        ));
        assert!(runtime.events.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn forward_timeout_preserves_reserve_and_compensation_stops_at_total_deadline() {
    let clock = Arc::new(FakeClock::default());
    let supervisor = RouteBoundOpenSupervisor::with_clock(Some(1_000), None, clock.clone());
    clock.set(750);
    let mut runtime = ScriptedRuntime::new();
    let forward = supervisor
        .forward(
            "open_target",
            runtime.open_target(OpenTargetRequest {
                command: OpenTargetCommand::from_compatibility(json!({})).unwrap(),
            }),
        )
        .await;
    assert!(matches!(
        forward,
        Err(RouteBoundRuntimeIssue::ForwardDeadlineElapsed { .. })
    ));
    assert!(runtime.events.lock().unwrap().is_empty());

    let cleanup_clock = clock.clone();
    let cleanup: RouteBoundOpenFuture<'_, Value> = Box::pin(async move {
        cleanup_clock.advance(249);
        Ok(json!({ "closed": true }))
    });
    assert!(supervisor
        .compensate("close_created_target", cleanup)
        .await
        .is_ok());
    assert_eq!(clock.now_ms(), 999);

    clock.set(1_000);
    let invoked = Arc::new(AtomicU64::new(0));
    let invoked_by_cleanup = invoked.clone();
    let late_cleanup: RouteBoundOpenFuture<'_, Value> = Box::pin(async move {
        invoked_by_cleanup.fetch_add(1, Ordering::Relaxed);
        Ok(json!({ "closed": true }))
    });
    let incomplete = supervisor
        .compensate("close_created_target", late_cleanup)
        .await;
    assert!(matches!(
        incomplete,
        Err(RouteBoundRuntimeIssue::EffectFailed { .. })
    ));
    assert_eq!(invoked.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn scripted_runtime_success_records_only_completed_effects() {
    let clock = Arc::new(FakeClock::default());
    let supervisor = RouteBoundOpenSupervisor::with_clock(Some(2_000), None, clock);
    let mut runtime = ScriptedRuntime::new();
    supervisor
        .forward("observe_browser", runtime.observe_browser())
        .await
        .unwrap();
    supervisor
        .forward(
            "launch_browser",
            runtime.launch_browser(LaunchBrowserRequest {
                command: LaunchBrowserCommand::from_compatibility(json!({})).unwrap(),
            }),
        )
        .await
        .unwrap();
    supervisor
        .forward(
            "open_target",
            runtime.open_target(OpenTargetRequest {
                command: OpenTargetCommand::from_compatibility(json!({})).unwrap(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(
        *runtime.events.lock().unwrap(),
        vec!["observe_browser", "launch_browser", "open_target"]
    );
}

#[tokio::test]
async fn newly_opened_target_navigates_without_redundant_target_switch() {
    let supervisor = RouteBoundOpenSupervisor::system(None, None);
    let mut runtime = ScriptedRuntime::new();
    let mut tab = json!({
        "targetId": "target-1",
        "url": "about:blank",
        "tabAcquisitionDecision": "opened_new_target",
    });

    route_bound_open_wait_for_target(
        &json!({ "url": "https://example.test" }),
        &mut runtime,
        &supervisor,
        &mut tab,
    )
    .await;

    assert_eq!(tab["targetReadiness"], "ready");
    assert_eq!(
        *runtime.events.lock().unwrap(),
        vec!["navigate_target", "refresh_targets"]
    );
}

struct StaticRepository {
    state: ServiceState,
}

struct FixtureRepository {
    repository: LockedServiceStateRepository<JsonServiceStateStore>,
}

impl RouteBoundOpenRepository for FixtureRepository {
    fn snapshot(&self, lock_timeout: Duration) -> RouteBoundOpenFuture<'_, ServiceState> {
        Box::pin(async move {
            self.repository
                .load_snapshot_with_lock_timeout(lock_timeout)
                .map_err(|message| RouteBoundRuntimeIssue::EffectFailed {
                    operation: "fixture_snapshot",
                    message,
                })
        })
    }

    fn execute<'a, T, F>(
        &'a self,
        operation: &'static str,
        lock_timeout: Duration,
        work: F,
    ) -> RouteBoundOpenFuture<'a, T>
    where
        T: Send + 'a,
        F: FnOnce(&LockedServiceStateRepository<JsonServiceStateStore>) -> Result<T, String>
            + Send
            + 'a,
    {
        Box::pin(async move {
            let repository = self.repository.with_lock_timeout(lock_timeout);
            work(&repository)
                .map_err(|message| RouteBoundRuntimeIssue::EffectFailed { operation, message })
        })
    }
}

impl RouteBoundOpenRepository for StaticRepository {
    fn snapshot(&self, _lock_timeout: Duration) -> RouteBoundOpenFuture<'_, ServiceState> {
        Box::pin(async { Ok(self.state.clone()) })
    }

    fn execute<'a, T, F>(
        &'a self,
        operation: &'static str,
        _lock_timeout: Duration,
        _work: F,
    ) -> RouteBoundOpenFuture<'a, T>
    where
        T: Send + 'a,
        F: FnOnce(&LockedServiceStateRepository<JsonServiceStateStore>) -> Result<T, String>
            + Send
            + 'a,
    {
        Box::pin(async move {
            Err(RouteBoundRuntimeIssue::EffectFailed {
                operation,
                message: "unexpected repository mutation".to_string(),
            })
        })
    }
}

fn authorized_attribution() -> RouteBoundOpenAttribution {
    RouteBoundOpenAttribution {
        caller_id: Some("operator-a".to_string()),
        service_job_id: Some("job-a".to_string()),
        dashboard_deployment_generation: Some("dashboard-test".to_string()),
        service_principal_id: None,
        service_principal_provenance: None,
        authorization: RouteBoundOpenAuthorization::AuthenticatedDaemonCommand,
    }
}

#[test]
fn durable_resolution_reuses_only_the_exact_live_profile_owner() {
    let intent = normalize_remote_view_open_intent(&json!({
        "browserId": "session:qbo-soylei",
        "sessionName": "qbo-soylei",
        "runtimeProfile": "qbo-soylei",
        "viewStreamProvider": "rdp_gateway"
    }))
    .unwrap();
    let mut observation = RouteBoundBrowserObservation {
        browser_present: true,
        browser_pid: Some(51579),
        browser_id: "session:qbo-soylei".to_string(),
        session_id: "qbo-soylei".to_string(),
        runtime_profile: None,
        active_target_id: Some("target-qbo".to_string()),
        active_url: Some("https://accounts.intuit.com/".to_string()),
        active_title: Some("QuickBooks".to_string()),
        pages: Vec::new(),
    };
    let mut service_state = ServiceState {
        browsers: BTreeMap::from([(
            "session:qbo-soylei".to_string(),
            BrowserProcess {
                id: "session:qbo-soylei".to_string(),
                pid: Some(51579),
                profile_id: Some("qbo-soylei".to_string()),
                active_session_ids: vec!["qbo-soylei".to_string()],
                health: ServiceBrowserHealth::Ready,
                ..BrowserProcess::default()
            },
        )]),
        ..ServiceState::default()
    };

    assert!(
        remote_view_open_should_reuse_current_browser_for_durable_resolution(
            &observation,
            &intent,
            "session:qbo-soylei",
            "qbo-soylei",
            &service_state,
        )
    );

    service_state
        .browsers
        .get_mut("session:qbo-soylei")
        .unwrap()
        .profile_id = Some("other-profile".to_string());
    assert!(
        !remote_view_open_should_reuse_current_browser_for_durable_resolution(
            &observation,
            &intent,
            "session:qbo-soylei",
            "qbo-soylei",
            &service_state,
        )
    );

    service_state
        .browsers
        .get_mut("session:qbo-soylei")
        .unwrap()
        .profile_id = Some("qbo-soylei".to_string());
    observation.session_id = "other-session".to_string();
    assert!(
        !remote_view_open_should_reuse_current_browser_for_durable_resolution(
            &observation,
            &intent,
            "session:qbo-soylei",
            "qbo-soylei",
            &service_state,
        )
    );
}

#[tokio::test]
async fn coordinator_returns_typed_not_found_without_starting_a_runtime_effect() {
    let repository = StaticRepository {
        state: ServiceState::default(),
    };
    let supervisor = RouteBoundOpenSupervisor::system(Some(1_000), None);
    let mut runtime = ScriptedRuntime::new();
    let outcome = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::durable_resolution(
            "missing".to_string(),
            false,
            authorized_attribution(),
        )
        .unwrap(),
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();

    assert!(matches!(outcome, RouteBoundOpenOutcome::NotFound { .. }));
    assert!(runtime.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn coordinator_returns_typed_explicit_close_without_starting_a_runtime_effect() {
    let handoff = RemoteViewHandoff {
        id: "handoff-closed".to_string(),
        tab_id: Some("target:closed".to_string()),
        target_id: Some("closed".to_string()),
        ..RemoteViewHandoff::default()
    };
    let repository = StaticRepository {
        state: ServiceState {
            remote_view_handoffs: BTreeMap::from([(handoff.id.clone(), handoff)]),
            tabs: BTreeMap::from([(
                "target:closed".to_string(),
                BrowserTab {
                    id: "target:closed".to_string(),
                    target_id: Some("closed".to_string()),
                    lifecycle: TabLifecycle::Closed,
                    ..BrowserTab::default()
                },
            )]),
            ..ServiceState::default()
        },
    };
    let supervisor = RouteBoundOpenSupervisor::system(Some(1_000), None);
    let mut runtime = ScriptedRuntime::new();
    let outcome = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::durable_resolution(
            "handoff-closed".to_string(),
            false,
            authorized_attribution(),
        )
        .unwrap(),
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();

    assert!(matches!(
        outcome,
        RouteBoundOpenOutcome::ExplicitlyClosed { .. }
    ));
    assert!(runtime.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn coordinator_returns_typed_planned_outcome_without_launching_a_browser() {
    let root = std::env::temp_dir().join(format!(
        "agent-browser-route-open-planned-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_path = root.join("state.json");
    let store = JsonServiceStateStore::new(&state_path);
    store
        .save(&ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    frame_url: Some(
                        "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                    ),
                    external_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!({
                        "displayName": ":31",
                        "displayIsolation": "shared_display",
                        "routeUser": "agent-browser-rdp-a",
                        "displayAccess": {"state": "ready"}
                    }),
                    provider_mode: "single_controller".to_string(),
                    state: "available".to_string(),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let repository = FixtureRepository {
        repository: LockedServiceStateRepository::new(store),
    };
    let request = RouteBoundDirectOpenRequest::from_compatibility_command(
        json!({
            "action": "remote_view_open",
            "routePoolEntryId": "pool-a",
            "provider": "rdp_gateway",
            "runtimeProfile": "stealthcdp-default",
            "url": "https://example.test/",
            "dryRun": true
        }),
        None,
        authorized_attribution(),
    )
    .unwrap();
    let supervisor = RouteBoundOpenSupervisor::system(Some(1_000), None);
    let mut runtime = ScriptedRuntime::new();

    let outcome = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::direct(request),
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();

    assert!(matches!(outcome, RouteBoundOpenOutcome::Planned { .. }));
    assert_eq!(*runtime.events.lock().unwrap(), vec!["observe_browser"]);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn manual_seeding_route_exhaustion_stops_before_cdp_free_launch() {
    let repository = StaticRepository {
        state: ServiceState::default(),
    };
    let request = RouteBoundDirectOpenRequest::from_compatibility_command(
        json!({
            "action": "service_profile_manual_seeding_acquire",
            "manualSeeding": true,
            "runtimeProfile": "google-seeding",
            "targetServiceId": "google",
            "provider": "rdp_gateway",
            "url": "https://accounts.google.com/"
        }),
        Some("manual-seeding-test".to_string()),
        authorized_attribution(),
    )
    .unwrap();
    let supervisor = RouteBoundOpenSupervisor::system(Some(1_000), None);
    let mut runtime = ScriptedRuntime::new();

    let result = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::direct(request),
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await;

    assert!(result.is_err());
    assert_eq!(*runtime.events.lock().unwrap(), vec!["observe_browser"]);
}

#[test]
fn manual_seeding_visibility_requires_process_bound_ready_proof_without_auth_claim() {
    let mut binding = test_binding();
    binding.route_pool_entry_id = Some("pool-manual".to_string());
    binding.route_pool_entry_state = Some("available".to_string());
    let visible = route_bound_manual_seeding_operator_visible(
        &binding,
        "browser-manual",
        "session-manual",
        Some(4242),
        Some(&json!({"state": "ready"})),
    );
    assert_eq!(visible["state"], "ready");
    assert_eq!(visible["manualSeedingProcess"]["state"], "ready");
    assert_eq!(visible["target"]["state"], "not_applicable");
    assert_eq!(visible["authentication"]["state"], "not_probed");

    let absent_process = route_bound_manual_seeding_operator_visible(
        &binding,
        "browser-manual",
        "session-manual",
        None,
        Some(&json!({"state": "ready"})),
    );
    assert_ne!(absent_process["state"], "ready");
    assert_eq!(
        absent_process["notVisible"]["code"],
        "process_identity_unproven"
    );

    for proof_state in [
        "wrong_process",
        "browser_window_absent",
        "display_socket_unavailable",
    ] {
        let not_visible = route_bound_manual_seeding_operator_visible(
            &binding,
            "browser-manual",
            "session-manual",
            Some(4242),
            Some(&json!({"state": proof_state})),
        );
        assert_ne!(not_visible["state"], "ready");
        assert_eq!(not_visible["notVisible"]["code"], proof_state);
    }

    let mut stale_route = binding.clone();
    stale_route.route_id.clear();
    stale_route.route_pool_entry_state = Some("quarantined".to_string());
    stale_route.frame_url = None;
    stale_route.external_url = None;
    let not_visible = route_bound_manual_seeding_operator_visible(
        &stale_route,
        "browser-manual",
        "session-manual",
        Some(4242),
        Some(&json!({"state": "ready"})),
    );
    assert_eq!(not_visible["notVisible"]["code"], "stale_or_unready_route");

    let mut unavailable_guacamole = binding;
    unavailable_guacamole.frame_url = None;
    unavailable_guacamole.external_url = None;
    let not_visible = route_bound_manual_seeding_operator_visible(
        &unavailable_guacamole,
        "browser-manual",
        "session-manual",
        Some(4242),
        Some(&json!({"state": "ready"})),
    );
    assert_eq!(not_visible["notVisible"]["code"], "guacamole_unavailable");
}

#[test]
fn durable_handoff_observation_accepts_reacquired_intent_target() {
    let handoff = RemoteViewHandoff {
        browser_id: Some("session:qbo".to_string()),
        session_name: Some("qbo-owner".to_string()),
        target_id: Some("expired-target".to_string()),
        desired_url: Some("https://accounts.example.com/".to_string()),
        profile_id: Some("qbo-profile".to_string()),
        ..RemoteViewHandoff::default()
    };
    let observation = RouteBoundBrowserObservation {
        browser_present: true,
        browser_pid: Some(4242),
        browser_id: "session:qbo".to_string(),
        session_id: "qbo-owner".to_string(),
        runtime_profile: Some("qbo-profile".to_string()),
        active_target_id: Some("replacement-target".to_string()),
        active_url: Some("https://accounts.example.com/sign-in".to_string()),
        active_title: Some("Sign in".to_string()),
        pages: vec![PageInfo {
            target_id: "replacement-target".to_string(),
            session_id: "page-session".to_string(),
            url: "https://accounts.example.com/sign-in".to_string(),
            title: "Sign in".to_string(),
            target_type: "page".to_string(),
        }],
    };

    assert!(durable_handoff_observation_matches(&observation, &handoff));
}

#[tokio::test]
async fn durable_resolution_adopts_the_exact_browser_without_provider_redirect() {
    let root = std::env::temp_dir().join(format!(
        "agent-browser-route-open-fallback-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_path = root.join("state.json");
    let handoff = RemoteViewHandoff {
        id: "handoff-a".to_string(),
        handoff_url: Some("/remote-view/handoff-a".to_string()),
        intent: json!({
            "url": "https://example.test/",
            "viewStreamProvider": "rdp_gateway",
            "runtimeProfile": "im-receipts-main",
            "controlInput": "manual_attached_desktop"
        }),
        browser_id: Some("session:im-receipts".to_string()),
        session_name: Some("im-receipts".to_string()),
        tab_id: Some("target:tab-a".to_string()),
        target_id: Some("tab-a".to_string()),
        profile_id: Some("im-receipts-main".to_string()),
        view_stream_provider: Some(ViewStreamProvider::RdpGateway),
        last_route_id: Some("route-a".to_string()),
        last_route_pool_entry_id: Some("pool-a".to_string()),
        last_display_allocation_id: Some("display-a".to_string()),
        presentation_receipt: Some(
            crate::native::service_model::DurableHandoffPresentationReceipt {
                schema_version: "agent-browser.durable-handoff-presentation.v1".to_string(),
                generation: 4,
                dashboard_deployment_generation: "dashboard-old".to_string(),
                logical_browser_id: "session:im-receipts".to_string(),
                daemon_owner_generation: Some(7),
                process_instance_digest: Some("process-old".to_string()),
                target_id: "tab-a".to_string(),
                required_stream_provider: ViewStreamProvider::RdpGateway,
                observed_stream_provider: ViewStreamProvider::RdpGateway,
                route_id: "route-old".to_string(),
                display_allocation_id: "display-old".to_string(),
                observed_at: "2026-08-15T12:00:00Z".to_string(),
                state: "ready".to_string(),
            },
        ),
        ..RemoteViewHandoff::default()
    };
    let initial = ServiceState {
        remote_view_handoffs: BTreeMap::from([(handoff.id.clone(), handoff)]),
        display_allocations: BTreeMap::from([(
            "display-a".to_string(),
            DisplayAllocation {
                id: "display-a".to_string(),
                display_name: Some(":31".to_string()),
                display_isolation: "shared_display".to_string(),
                owner_browser_id: Some("session:im-receipts".to_string()),
                owner_session_id: Some("im-receipts".to_string()),
                state: "ready".to_string(),
                route_ids: vec!["route-a".to_string()],
                ..DisplayAllocation::default()
            },
        )]),
        remote_view_routes: BTreeMap::from([(
            "route-a".to_string(),
            RemoteViewRoute {
                id: "route-a".to_string(),
                provider: ViewStreamProvider::RdpGateway,
                display_allocation_id: Some("display-a".to_string()),
                browser_id: Some("session:im-receipts".to_string()),
                session_id: Some("im-receipts".to_string()),
                state: "ready".to_string(),
                external_url: Some("https://guac.example/#/client/route-a".to_string()),
                ..RemoteViewRoute::default()
            },
        )]),
        route_pool: BTreeMap::from([(
            "pool-a".to_string(),
            RoutePoolEntry {
                id: "pool-a".to_string(),
                provider: ViewStreamProvider::RdpGateway,
                route_id: "route-a".to_string(),
                frame_url: Some("https://dashboard.example/guacamole/#/client/route-a".to_string()),
                external_url: Some("https://guac.example/#/client/route-a".to_string()),
                target: json!({
                    "displayName": ":31",
                    "displayIsolation": "shared_display",
                    "displayAccess": {"state": "ready"}
                }),
                state: "checked_out".to_string(),
                current_route_allocation_id: Some("route-a".to_string()),
                ..RoutePoolEntry::default()
            },
        )]),
        runtime_owner_registry: crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
            crate::runtime_owner_transfer::ProfileOwner {
                owner_id: "owner-current".to_string(),
                profile_identity_digest: "profile-digest".to_string(),
                state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                owner_generation: 8,
                browser_id: "session:im-receipts".to_string(),
                daemon_session_route: "im-receipts".to_string(),
                process_instance_digest: "process-digest".to_string(),
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "cdp-digest".to_string(),
                target_set_digest: "target-digest".to_string(),
                pending_transfer: None,
                last_transition: None,
            },
        ),
        ..ServiceState::default()
    };
    let store = JsonServiceStateStore::new(&state_path);
    store.save(&initial).unwrap();
    let repository = FixtureRepository {
        repository: LockedServiceStateRepository::new(store.clone()),
    };
    let supervisor = RouteBoundOpenSupervisor::system(Some(10_000), None);
    let mut runtime = ScriptedRuntime::new();
    let outcome = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::durable_resolution(
            "handoff-a".to_string(),
            false,
            authorized_attribution(),
        )
        .unwrap(),
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();

    assert!(matches!(outcome, RouteBoundOpenOutcome::Converging { .. }));
    assert_eq!(
        *runtime.events.lock().unwrap(),
        vec!["observe_browser", "adopt_retained_browser"]
    );
    let adoption_requests = runtime.adoption_requests.lock().unwrap();
    assert_eq!(adoption_requests.len(), 1);
    assert_eq!(adoption_requests[0].source_session, "im-receipts");
    assert_eq!(
        adoption_requests[0].logical_browser_id,
        "session:im-receipts"
    );
    drop(adoption_requests);
    let retained = store.load().unwrap();
    assert_eq!(
        retained.remote_view_routes["route-a"].browser_id.as_deref(),
        Some("session:im-receipts")
    );
    assert!(retained.browsers.is_empty());

    runtime.adoption_issue = None;
    runtime.adoption_observation = Some(RouteBoundBrowserObservation {
        browser_present: true,
        browser_pid: Some(4242),
        browser_id: "session:im-receipts".to_string(),
        session_id: "im-receipts".to_string(),
        runtime_profile: Some("im-receipts-main".to_string()),
        active_target_id: Some("tab-a".to_string()),
        active_url: Some("https://example.test/".to_string()),
        active_title: Some("Retained".to_string()),
        pages: vec![PageInfo {
            target_id: "tab-a".to_string(),
            session_id: "page-session".to_string(),
            url: "https://example.test/".to_string(),
            title: "Retained".to_string(),
            target_type: "page".to_string(),
        }],
    });
    runtime.events.lock().unwrap().clear();
    let adopted = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::durable_resolution(
            "handoff-a".to_string(),
            false,
            authorized_attribution(),
        )
        .unwrap(),
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();
    assert!(
        matches!(adopted, RouteBoundOpenOutcome::Opened { .. }),
        "unexpected adopted resolution: {adopted:?}"
    );
    let adoption_events = runtime.events.lock().unwrap().clone();
    assert!(adoption_events.contains(&"adopt_retained_browser"));
    assert!(!adoption_events.contains(&"launch_browser"));
    assert!(!adoption_events.contains(&"open_target"));
    assert!(!adoption_events.contains(&"navigate_target"));
    let adopted_value = adopted.clone().into_compatibility_result().unwrap();
    assert_eq!(adopted_value["presentationGeneration"], 5);
    assert_eq!(adopted_value["targetId"], "tab-a");
    assert_eq!(
        adopted_value["presentationReceipt"]["dashboardDeploymentGeneration"],
        "dashboard-test"
    );
    assert_eq!(
        adopted_value["presentationReceipt"]["requiredStreamProvider"],
        "rdp_gateway"
    );
    assert_eq!(
        adopted_value["presentationReceipt"]["observedStreamProvider"],
        "rdp_gateway"
    );
    let adopted_state = store.load().unwrap();
    let adopted_receipt = adopted_state.remote_view_handoffs["handoff-a"]
        .presentation_receipt
        .as_ref()
        .unwrap();
    assert_eq!(adopted_receipt.generation, 5);
    assert_eq!(adopted_receipt.target_id, "tab-a");
    assert_eq!(
        adopted_receipt.dashboard_deployment_generation,
        "dashboard-test"
    );
    assert_eq!(adopted_receipt.route_id, "route-a");
    assert_eq!(adopted_receipt.display_allocation_id, "display-a");
    assert_eq!(adopted_receipt.daemon_owner_generation, Some(8));
    assert_eq!(
        adopted_receipt.process_instance_digest.as_deref(),
        Some("process-digest")
    );

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn coordinator_returns_typed_opened_outcome_from_scripted_effects() {
    let root = std::env::temp_dir().join(format!(
        "agent-browser-route-open-opened-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_path = root.join("state.json");
    let store = JsonServiceStateStore::new(&state_path);
    store
        .save(&ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    provider: ViewStreamProvider::RdpGateway,
                    frame_url: Some(
                        "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                    ),
                    external_url: Some("https://guac.example/#/client/route-a".to_string()),
                    target: json!({
                        "displayName": ":31",
                        "displayIsolation": "shared_display",
                        "routeUser": "agent-browser-rdp-a",
                        "displayAccess": {"state": "ready"}
                    }),
                    provider_mode: "single_controller".to_string(),
                    state: "available".to_string(),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let repository = FixtureRepository {
        repository: LockedServiceStateRepository::new(store),
    };
    let request = RouteBoundDirectOpenRequest::from_compatibility_command(
        json!({
            "action": "remote_view_open",
            "routePoolEntryId": "pool-a",
            "provider": "rdp_gateway",
            "runtimeProfile": "stealthcdp-default",
            "url": "https://example.test/"
        }),
        None,
        authorized_attribution(),
    )
    .unwrap();
    let supervisor = RouteBoundOpenSupervisor::system(Some(10_000), None);
    let mut runtime = ScriptedRuntime::new();
    runtime.observation.browser_present = true;
    runtime.observation.runtime_profile = Some("stealthcdp-default".to_string());
    runtime.observation.active_target_id = Some("target-1".to_string());
    runtime.observation.active_url = Some("https://example.test/".to_string());
    runtime.observation.pages = vec![PageInfo {
        target_id: "target-1".to_string(),
        session_id: "page-session".to_string(),
        url: "https://example.test/".to_string(),
        title: "Example".to_string(),
        target_type: "page".to_string(),
    }];

    let outcome = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::direct(request),
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();

    assert!(
        matches!(outcome, RouteBoundOpenOutcome::Opened { .. }),
        "unexpected outcome: {outcome:?}"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn manual_seeding_uses_reserved_route_before_cdp_free_launch_and_persists_handoff() {
    let root = std::env::temp_dir().join(format!(
        "agent-browser-manual-seeding-opened-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_path = root.join("state.json");
    let store = JsonServiceStateStore::new(&state_path);
    store
        .save(&ServiceState {
            profiles: BTreeMap::from([(
                "google-seeding".to_string(),
                BrowserProfile {
                    id: "google-seeding".to_string(),
                    name: "Google manual seeding".to_string(),
                    user_data_dir: Some(root.join("profile").to_string_lossy().into_owned()),
                    target_service_ids: vec!["google".to_string()],
                    persistent: true,
                    ..BrowserProfile::default()
                },
            )]),
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    provider: ViewStreamProvider::RdpGateway,
                    frame_url: Some(
                        "https://dashboard.example/guacamole/#/client/route-a".to_string(),
                    ),
                    external_url: Some("https://guac.example/#/client/route-a".to_string()),
                    route_descriptor: Some(json!({
                        "publicOperatorUrl": "https://dashboard.example/remote-view",
                    })),
                    target: json!({
                        "displayName": ":31",
                        "displayIsolation": "shared_display",
                        "routeUser": "agent-browser-rdp-a",
                        "displayAccess": {"state": "ready"}
                    }),
                    provider_mode: "single_controller".to_string(),
                    state: "available".to_string(),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let repository = FixtureRepository {
        repository: LockedServiceStateRepository::new(store.clone()),
    };
    let request = RouteBoundDirectOpenRequest::from_compatibility_command(
        json!({
            "action": "service_profile_manual_seeding_acquire",
            "manualSeeding": true,
            "remoteViewHandoffId": "manual-seeding-a",
            "routePoolEntryId": "pool-a",
            "provider": "rdp_gateway",
            "runtimeProfile": "google-seeding",
            "targetServiceId": "google",
            "url": "https://accounts.google.com/"
        }),
        Some("manual-seeding-a".to_string()),
        authorized_attribution(),
    )
    .unwrap();
    let supervisor = RouteBoundOpenSupervisor::system(Some(10_000), None);
    let mut runtime = ScriptedRuntime::new();

    let outcome = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::direct(request),
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();
    let opened = match outcome {
        RouteBoundOpenOutcome::Opened { opened } => opened.into_value(),
        other => panic!("unexpected manual-seeding outcome: {other:?}"),
    };

    let events = runtime.events.lock().unwrap().clone();
    assert_eq!(
        events,
        vec![
            "observe_browser",
            "ensure_display_access",
            "launch_cdp_free_browser",
            "stage_visible_window",
            "observe_visible_window",
            "observe_operator_access",
            "checkout_route",
        ]
    );
    assert_eq!(opened["operatorVisible"]["state"], "ready");
    assert_eq!(opened["focus"]["state"], "staged");
    assert_eq!(opened["lifecycleState"], "manual_seeding");
    assert_eq!(opened["cdpAttachmentAllowed"], false);
    assert_eq!(opened["authentication"]["state"], "not_probed");
    assert_eq!(
        opened["handoffUrl"],
        "https://dashboard.example/remote-view/manual-seeding-a"
    );

    let persisted = store.load().unwrap();
    let lifecycle = &persisted.profile_seeding_handoffs["google-seeding:google"];
    assert_eq!(
        lifecycle.state,
        crate::native::service_model::ProfileSeedingHandoffState::SeedingWaitingForClose
    );
    assert_eq!(lifecycle.pid, Some(4242));
    assert!(persisted
        .remote_view_handoffs
        .contains_key("manual-seeding-a"));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn manual_seeding_close_replay_advances_to_separate_auth_probe_without_broad_cleanup() {
    let state_path = JsonServiceStateStore::default_path().unwrap();
    let store = JsonServiceStateStore::new(&state_path);
    let profile_id = "manual-close-profile";
    let target_service_id = "google";
    let handoff_id = "manual-close-handoff";
    let route_id = "manual-close-route";
    let pid = u32::MAX - 7;
    store
        .save(&ServiceState {
            profiles: BTreeMap::from([(
                profile_id.to_string(),
                BrowserProfile {
                    id: profile_id.to_string(),
                    name: "Manual close profile".to_string(),
                    persistent: true,
                    ..BrowserProfile::default()
                },
            )]),
            profile_seeding_handoffs: BTreeMap::from([(
                format!("{profile_id}:{target_service_id}"),
                crate::native::service_model::ProfileSeedingHandoffRecord {
                    id: format!("{profile_id}:{target_service_id}"),
                    profile_id: profile_id.to_string(),
                    target_service_id: target_service_id.to_string(),
                    state: crate::native::service_model::ProfileSeedingHandoffState::SeedingWaitingForClose,
                    pid: Some(pid),
                    ..crate::native::service_model::ProfileSeedingHandoffRecord::default()
                },
            )]),
            remote_view_handoffs: BTreeMap::from([(
                handoff_id.to_string(),
                RemoteViewHandoff {
                    id: handoff_id.to_string(),
                    profile_id: Some(profile_id.to_string()),
                    last_route_id: Some(route_id.to_string()),
                    ..RemoteViewHandoff::default()
                },
            )]),
            remote_view_routes: BTreeMap::from([(
                route_id.to_string(),
                RemoteViewRoute {
                    id: route_id.to_string(),
                    state: "released".to_string(),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();

    let mismatch = handle_service_profile_manual_seeding_close(
        &json!({
            "profileId": profile_id,
            "targetServiceId": target_service_id,
            "handoffId": handoff_id,
            "pid": pid - 1,
        }),
        &DaemonState::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(mismatch, "manual_seeding_close_pid_mismatch");

    let result = handle_service_profile_manual_seeding_close(
        &json!({
            "profileId": profile_id,
            "targetServiceId": target_service_id,
            "handoffId": handoff_id,
            "pid": pid,
        }),
        &DaemonState::new(),
    )
    .await
    .unwrap();

    assert_eq!(result["closed"], true);
    assert_eq!(result["routeRelease"]["replayed"], true);
    assert_eq!(
        result["attachableRelaunch"]["action"],
        "service_profile_acquire"
    );
    assert_eq!(
        result["authenticationProbe"]["visibilityAcceptedAsAuthentication"],
        false
    );
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.profile_seeding_handoffs[&format!("{profile_id}:{target_service_id}")].state,
        crate::native::service_model::ProfileSeedingHandoffState::SeedingClosedUnverified
    );
}

#[test]
fn every_ingress_uses_the_same_transport_neutral_authorization_fact() {
    let ingress_commands = [
        json!({"action": "remote_view_open", "callerId": "daemon"}),
        json!({"action": "remote_view_open", "callerId": "cli"}),
        json!({"action": "remote_view_open", "callerId": "http"}),
        json!({"action": "remote_view_open", "callerId": "mcp"}),
        json!({"action": "remote_view_open", "callerId": "dashboard"}),
    ];
    for command in ingress_commands {
        let attribution = route_bound_open_attribution_from_authenticated_dispatch(&command);
        assert!(
            attribution.authorization.is_authorized(),
            "authenticated dispatch must produce the sealed authorization fact"
        );
        assert_eq!(
            attribution.caller_id,
            command
                .get("callerId")
                .and_then(Value::as_str)
                .map(str::to_string)
        );
        assert!(RouteBoundDirectOpenRequest::from_compatibility_command(
            command,
            None,
            attribution,
        )
        .is_ok());
    }
    let authenticated_resolver = json!({
        "action": "service_remote_view_handoff_resolve",
        "servicePrincipalId": "odollo-fulfillment",
        "servicePrincipalProvenance": "registered_capability",
    });
    let resolver_attribution =
        route_bound_open_attribution_from_authenticated_dispatch(&authenticated_resolver);
    let rebuilt = RouteBoundDirectOpenRequest::from_compatibility_command(
        json!({"action": "remote_view_open"}),
        Some("handoff-a".to_string()),
        resolver_attribution,
    )
    .expect("authenticated resolver attribution should cross the durable boundary")
    .command();
    assert_eq!(rebuilt["servicePrincipalId"], "odollo-fulfillment");
    assert_eq!(
        rebuilt["servicePrincipalProvenance"],
        "registered_capability"
    );
    let rejected_attribution = RouteBoundOpenAttribution {
        caller_id: None,
        service_job_id: None,
        dashboard_deployment_generation: None,
        service_principal_id: None,
        service_principal_provenance: None,
        authorization: RouteBoundOpenAuthorization::Rejected,
    };
    let rejected_direct = RouteBoundDirectOpenRequest::from_compatibility_command(
        json!({"action": "remote_view_open"}),
        None,
        rejected_attribution.clone(),
    );
    assert_eq!(
        rejected_direct.unwrap_err(),
        "Unauthorized route-bound open invocation"
    );
    let rejected_durable = RouteBoundOpenInvocation::durable_resolution(
        "handoff-a".to_string(),
        false,
        rejected_attribution,
    );
    assert_eq!(
        rejected_durable.unwrap_err(),
        "Unauthorized route-bound open invocation"
    );
}

#[test]
fn concrete_route_records_reject_untyped_payloads() {
    assert!(LaunchBrowserCommand::from_compatibility(json!("launch")).is_err());
    assert!(LaunchBrowserCommand::from_compatibility(json!({"dryRun": "yes"})).is_err());
    assert!(LaunchBrowserResult::from_compatibility(json!({"pid": "forty-two"})).is_err());
    assert!(RouteBoundOpenDocument::from_compatibility(json!({"resolved": "yes"})).is_err());
}

#[test]
fn typed_terminal_failure_selects_rollback_state_without_parsing_compatibility_text() {
    let outcome = rolled_back_outcome(route_bound_message_error_with_cleanup(
        "final_proof_failed",
        "operator proof failed".to_string(),
        json!({"state": "rollback_incomplete", "leaseId": "lease-a"}),
        "opaque compatibility summary without state words",
    ))
    .unwrap();

    match outcome {
        RouteBoundOpenOutcome::RolledBack {
            blocker,
            compensation,
            compatibility_error,
        } => {
            assert_eq!(blocker.code, "final_proof_failed");
            assert_eq!(blocker.message, "operator proof failed");
            assert_eq!(compensation.state, "rollback_incomplete");
            assert_eq!(compensation.evidence["leaseId"], "lease-a");
            assert!(compatibility_error.starts_with("operator proof failed; cleanup="));
        }
        _ => panic!("structured terminal failure must produce RolledBack"),
    }
}

#[test]
fn completed_route_bound_rollback_survives_compatibility_failure_normalization() {
    let outcome = rolled_back_outcome(route_bound_message_error_with_cleanup(
        "checkout_failed",
        "route pool checkout failed".to_string(),
        json!({"state": "rolled_back", "leaseId": "lease-a"}),
        "opaque cleanup summary",
    ))
    .unwrap();
    let compatibility_error = outcome.into_compatibility_result().unwrap_err();

    let recourse = crate::native::service_failure::classify_service_failure(&compatibility_error);

    assert_eq!(recourse.code, "checkout_failed");
    assert_eq!(
        recourse.axis,
        crate::native::service_failure::ServiceFailureAxis::Presentation
    );
    assert_eq!(
        recourse.effect_state,
        crate::native::service_failure::ServiceEffectState::NoEffect
    );
    assert_eq!(
        recourse.retry_disposition,
        crate::native::service_failure::ServiceRetryDisposition::InspectBeforeRetry
    );
}

#[test]
fn exact_pending_acquisition_can_claim_ownerless_warm_provider_route() {
    let state = ServiceState {
        remote_view_routes: BTreeMap::from([(
            "route-a".to_string(),
            RemoteViewRoute {
                id: "route-a".to_string(),
                display_allocation_id: Some("provider-display-a".to_string()),
                browser_id: None,
                session_id: None,
                state: "ready".to_string(),
                ..RemoteViewRoute::default()
            },
        )]),
        remote_view_acquisition_leases: BTreeMap::from([(
            "lease-a".to_string(),
            RemoteViewAcquisitionLease {
                id: "lease-a".to_string(),
                boot_epoch: crate::process_identity::current_boot_epoch(),
                browser_id: "session:current".to_string(),
                session_id: "current".to_string(),
                route_id: "route-a".to_string(),
                display_allocation_id: "remote-view-display:route-a".to_string(),
                route_pool_entry_id: Some("pool-a".to_string()),
                state: "pending".to_string(),
                phase: "reserved".to_string(),
                ..RemoteViewAcquisitionLease::default()
            },
        )]),
        ..ServiceState::default()
    };
    let allocation = DisplayAllocation {
        id: "remote-view-display:route-a".to_string(),
        owner_browser_id: Some("session:current".to_string()),
        owner_session_id: Some("current".to_string()),
        display_isolation: "shared_display".to_string(),
        state: "pending".to_string(),
        ..DisplayAllocation::default()
    };

    ensure_remote_view_route_available_for_display(
        &state,
        "route-a",
        "remote-view-display:route-a",
        "session:current",
        "current",
        Some(&allocation),
    )
    .unwrap();

    let mut foreign_state = state;
    foreign_state
        .remote_view_acquisition_leases
        .get_mut("lease-a")
        .unwrap()
        .session_id = "other".to_string();
    let error = ensure_remote_view_route_available_for_display(
        &foreign_state,
        "route-a",
        "remote-view-display:route-a",
        "session:current",
        "current",
        Some(&allocation),
    )
    .unwrap_err();
    assert!(error.starts_with("route_pool_contention:"));
}

struct PendingRepository;

impl RouteBoundOpenRepository for PendingRepository {
    fn snapshot(&self, _lock_timeout: Duration) -> RouteBoundOpenFuture<'_, ServiceState> {
        Box::pin(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(ServiceState::default())
        })
    }

    fn execute<'a, T, F>(
        &'a self,
        operation: &'static str,
        _lock_timeout: Duration,
        _work: F,
    ) -> RouteBoundOpenFuture<'a, T>
    where
        T: Send + 'a,
        F: FnOnce(&LockedServiceStateRepository<JsonServiceStateStore>) -> Result<T, String>
            + Send
            + 'a,
    {
        Box::pin(async move {
            Err(RouteBoundRuntimeIssue::EffectFailed {
                operation,
                message: "unexpected repository mutation".to_string(),
            })
        })
    }
}

#[tokio::test]
async fn repository_snapshot_is_dropped_at_the_forward_deadline() {
    let supervisor = RouteBoundOpenSupervisor::system(Some(2), None);
    let repository = PendingRepository;

    let result = supervisor
        .forward(
            "repository_load_snapshot",
            repository.snapshot(supervisor.forward_repository_lock_timeout()),
        )
        .await;

    assert!(matches!(
        result,
        Err(RouteBoundRuntimeIssue::ForwardDeadlineElapsed {
            operation: "repository_load_snapshot",
            total_ms: 2,
        })
    ));
}

#[tokio::test]
async fn cleanup_timeout_preserves_precleanup_quarantine() {
    let root = std::env::temp_dir().join(format!(
        "agent-browser-route-cleanup-timeout-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_path = root.join("state.json");
    let store = JsonServiceStateStore::new(&state_path);
    store
        .save(&ServiceState {
            route_pool: BTreeMap::from([(
                "pool-a".to_string(),
                RoutePoolEntry {
                    id: "pool-a".to_string(),
                    route_id: "route-a".to_string(),
                    state: "pending".to_string(),
                    current_route_allocation_id: Some("route-a".to_string()),
                    ..RoutePoolEntry::default()
                },
            )]),
            ..ServiceState::default()
        })
        .unwrap();
    let repository = LockedServiceStateRepository::new(store.clone());
    let lease = RemoteViewAcquisitionLease {
        id: "lease-timeout".to_string(),
        browser_id: "browser-a".to_string(),
        session_id: "session-a".to_string(),
        route_id: "route-a".to_string(),
        display_allocation_id: "display-a".to_string(),
        route_pool_entry_id: Some("pool-a".to_string()),
        state: "pending".to_string(),
        phase: "reserved".to_string(),
        previous_route_pool_entry: Some(RoutePoolEntry {
            id: "pool-a".to_string(),
            route_id: "route-a".to_string(),
            state: "available".to_string(),
            ..RoutePoolEntry::default()
        }),
        ..RemoteViewAcquisitionLease::default()
    };
    let recovery = begin_route_bound_handoff_failure_recovery(
        &repository,
        RouteBoundHandoffFailureRecoveryInput {
            lease: &lease,
            phase: "proof_failed",
            error: "operator proof failed",
            rollback_cleanup: &json!({"state": "pending_after_rollback"}),
            launch: &json!({"launched": true}),
            tab: Some(&json!({"targetId": "target-a"})),
            observed_at: "2026-08-10T12:00:00Z",
        },
    )
    .unwrap();
    assert_eq!(recovery.rollback["state"], "rollback_incomplete");

    let supervisor = RouteBoundOpenSupervisor::system(Some(10), None);
    let pending_cleanup: RouteBoundOpenFuture<'_, Value> = Box::pin(std::future::pending());
    let result = supervisor
        .compensate("close_created_target", pending_cleanup)
        .await;

    assert!(matches!(
        result,
        Err(RouteBoundRuntimeIssue::EffectFailed { .. })
    ));
    let persisted = store.load().unwrap();
    assert_eq!(
        persisted.remote_view_acquisition_leases["lease-timeout"].phase,
        "rollback_incomplete"
    );
    assert_eq!(persisted.route_pool["pool-a"].state, "quarantined");
    let _ = std::fs::remove_dir_all(root);
}
