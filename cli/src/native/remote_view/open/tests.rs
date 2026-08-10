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
    observation: RouteBoundBrowserObservation,
    launch_issue: Option<RouteBoundRuntimeIssue>,
}

impl ScriptedRuntime {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
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

    fn launch_browser(
        &mut self,
        _request: LaunchBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, LaunchBrowserResult> {
        Box::pin(async move {
            self.events.lock().unwrap().push("launch_browser");
            match self.launch_issue.clone() {
                Some(issue) => Err(issue),
                None => Ok(json!({ "launched": true }).into()),
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
                .map(SwitchTargetResult::from)
        })
    }

    fn navigate_target(
        &mut self,
        _request: NavigateTargetRequest,
    ) -> RouteBoundOpenFuture<'_, NavigateTargetResult> {
        Box::pin(async move {
            self.effect("navigate_target", json!({ "url": "https://example.test" }))
                .await
                .map(NavigateTargetResult::from)
        })
    }

    fn open_target(
        &mut self,
        _request: OpenTargetRequest,
    ) -> RouteBoundOpenFuture<'_, OpenTargetResult> {
        Box::pin(async move {
            self.effect("open_target", json!({ "targetId": "target-1" }))
                .await
                .map(OpenTargetResult::from)
        })
    }

    fn focus_target(
        &mut self,
        _request: FocusTargetRequest,
    ) -> RouteBoundOpenFuture<'_, FocusTargetResult> {
        Box::pin(async move {
            self.effect("focus_target", json!({ "focused": true }))
                .await
                .map(FocusTargetResult::from)
        })
    }

    fn close_created_target(
        &mut self,
        _request: CloseCreatedTargetRequest,
    ) -> RouteBoundOpenFuture<'_, CloseCreatedTargetResult> {
        Box::pin(async move {
            self.effect("close_created_target", json!({ "closed": true }))
                .await
                .map(CloseCreatedTargetResult::from)
        })
    }

    fn close_created_browser(
        &mut self,
        _request: CloseCreatedBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, CloseCreatedBrowserResult> {
        Box::pin(async move {
            self.effect("close_created_browser", json!({ "closed": true }))
                .await
                .map(CloseCreatedBrowserResult::from)
        })
    }

    fn checkout_route(
        &mut self,
        _request: CheckoutRouteRequest,
    ) -> RouteBoundOpenFuture<'_, CheckoutRouteResult> {
        Box::pin(async move {
            self.effect("checkout_route", json!({ "status": "ready" }))
                .await
                .map(CheckoutRouteResult::from)
        })
    }

    fn ensure_display_access(
        &mut self,
        _request: DisplayAccessRequest,
    ) -> RouteBoundOpenFuture<'_, DisplayAccessResult> {
        Box::pin(async move {
            self.effect("ensure_display_access", json!({ "state": "ready" }))
                .await
                .map(DisplayAccessResult::from)
        })
    }

    fn observe_visible_window(
        &mut self,
        _request: VisibleWindowRequest,
    ) -> RouteBoundOpenFuture<'_, VisibleWindowResult> {
        Box::pin(async move {
            self.effect("observe_visible_window", json!({ "state": "ready" }))
                .await
                .map(VisibleWindowResult::from)
        })
    }

    fn observe_operator_access(
        &mut self,
        _request: OperatorAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Option<OperatorAccessResult>> {
        Box::pin(async move {
            self.events.lock().unwrap().push("observe_operator_access");
            Ok(Some(json!({ "state": "ready" }).into()))
        })
    }
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
                        command: json!({}).into(),
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
                        command: json!({}).into(),
                    }),
                )
                .await
                .map(|_| ()),
            4 => supervisor
                .forward(
                    "focus_target",
                    runtime.focus_target(FocusTargetRequest {
                        command: json!({}).into(),
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
                        browser_identity: json!({}),
                    }),
                )
                .await
                .map(|_| ()),
            7 => supervisor
                .forward(
                    "checkout_route",
                    runtime.checkout_route(CheckoutRouteRequest {
                        command: json!({}).into(),
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
                command: json!({}).into(),
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
                command: json!({}).into(),
            }),
        )
        .await
        .unwrap();
    supervisor
        .forward(
            "open_target",
            runtime.open_target(OpenTargetRequest {
                command: json!({}).into(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(
        *runtime.events.lock().unwrap(),
        vec!["observe_browser", "launch_browser", "open_target"]
    );
}

fn fallback_snapshot() -> RouteBoundResolutionSnapshot {
    let handoff = RemoteViewHandoff {
        id: "handoff-a".to_string(),
        browser_id: Some("session:im-receipts".to_string()),
        session_name: Some("im-receipts".to_string()),
        tab_id: Some("target:tab-a".to_string()),
        target_id: Some("tab-a".to_string()),
        profile_id: Some("im-receipts-main".to_string()),
        view_stream_provider: Some(ViewStreamProvider::RdpGateway),
        last_route_id: Some("guacamole:2".to_string()),
        ..RemoteViewHandoff::default()
    };
    RouteBoundResolutionSnapshot {
        state: ServiceState {
            remote_view_routes: BTreeMap::from([(
                "guacamole:2".to_string(),
                RemoteViewRoute {
                    id: "guacamole:2".to_string(),
                    provider: ViewStreamProvider::RdpGateway,
                    browser_id: handoff.browser_id.clone(),
                    session_id: handoff.session_name.clone(),
                    external_url: Some(
                        "https://dashboard.example/guacamole/#/client/route-b".to_string(),
                    ),
                    ..RemoteViewRoute::default()
                },
            )]),
            ..ServiceState::default()
        },
        handoff,
        loaded_at: "2026-08-10T09:00:00Z".to_string(),
    }
}

fn profile_conflict_issue() -> RouteBoundRuntimeIssue {
    RouteBoundRuntimeIssue::RequestedProfileInUseByPid {
        profile_id: "im-receipts-main".to_string(),
        pid: 42,
        owner_browser_id: Some("session:im-receipts".to_string()),
        owner_session_id: Some("im-receipts".to_string()),
        compatibility_message: "profile is already in use by PID 42".to_string(),
    }
}

#[test]
fn fallback_eligibility_requires_each_of_the_nine_closed_predicates() {
    for predicate in 0..9 {
        let mut eligibility = RouteBoundFallbackEligibility {
            prior_provider: true,
            snapshot_identity: true,
            snapshot_timing: true,
            exact_ownership_cause: true,
            retained_route: true,
            authorized_ingress: true,
            operator_evidence: true,
            browser_preserved: true,
            duplicate_lane_prohibited: true,
        };
        match predicate {
            0 => eligibility.prior_provider = false,
            1 => eligibility.snapshot_identity = false,
            2 => eligibility.snapshot_timing = false,
            3 => eligibility.exact_ownership_cause = false,
            4 => eligibility.retained_route = false,
            5 => eligibility.authorized_ingress = false,
            6 => eligibility.operator_evidence = false,
            7 => eligibility.browser_preserved = false,
            _ => eligibility.duplicate_lane_prohibited = false,
        }
        assert!(
            !eligibility.is_eligible(),
            "predicate {predicate} must fail closed"
        );
    }
}

#[tokio::test]
async fn provider_fallback_uses_one_immutable_snapshot_and_preserves_the_browser_lane() {
    let snapshot = fallback_snapshot();
    let before = serde_json::to_value((&snapshot.state, &snapshot.handoff)).unwrap();
    let supervisor = RouteBoundOpenSupervisor::system(Some(1_000), None);
    let mut runtime = ScriptedRuntime::new();
    let issue = profile_conflict_issue();

    let fallback = remote_view_handoff_provider_fallback_if_eligible(
        &snapshot,
        Some(&issue),
        RouteBoundOpenAuthorization::AuthenticatedDaemonCommand,
        &mut runtime,
        &supervisor,
    )
    .await
    .unwrap()
    .into_value();

    assert_eq!(fallback["providerFallback"], true);
    assert_eq!(fallback["resolutionSnapshotLoadedAt"], snapshot.loaded_at);
    assert!(fallback["fallbackEligibility"]
        .as_object()
        .unwrap()
        .values()
        .all(|value| value.as_bool() == Some(true)));
    assert_eq!(
        *runtime.events.lock().unwrap(),
        vec!["observe_operator_access"]
    );
    assert_eq!(
        before,
        serde_json::to_value((&snapshot.state, &snapshot.handoff)).unwrap()
    );
}

#[tokio::test]
async fn provider_fallback_rejects_unauthorized_ingress_before_any_runtime_effect() {
    let snapshot = fallback_snapshot();
    let supervisor = RouteBoundOpenSupervisor::system(Some(1_000), None);
    let mut runtime = ScriptedRuntime::new();
    let issue = profile_conflict_issue();

    let fallback = remote_view_handoff_provider_fallback_if_eligible(
        &snapshot,
        Some(&issue),
        RouteBoundOpenAuthorization::Rejected,
        &mut runtime,
        &supervisor,
    )
    .await;

    assert!(fallback.is_none());
    assert!(runtime.events.lock().unwrap().is_empty());
}

struct StaticRepository {
    state: ServiceState,
}

struct FixtureRepository {
    repository: LockedServiceStateRepository<JsonServiceStateStore>,
}

impl RouteBoundOpenRepository for FixtureRepository {
    fn snapshot(&self) -> RouteBoundOpenFuture<'_, ServiceState> {
        Box::pin(async move {
            self.repository.load_snapshot().map_err(|message| {
                RouteBoundRuntimeIssue::EffectFailed {
                    operation: "fixture_snapshot",
                    message,
                }
            })
        })
    }

    fn execute<'a, T, F>(&'a self, operation: &'static str, work: F) -> RouteBoundOpenFuture<'a, T>
    where
        T: Send + 'a,
        F: FnOnce(&LockedServiceStateRepository<JsonServiceStateStore>) -> Result<T, String>
            + Send
            + 'a,
    {
        Box::pin(async move {
            work(&self.repository)
                .map_err(|message| RouteBoundRuntimeIssue::EffectFailed { operation, message })
        })
    }
}

impl RouteBoundOpenRepository for StaticRepository {
    fn snapshot(&self) -> RouteBoundOpenFuture<'_, ServiceState> {
        Box::pin(async { Ok(self.state.clone()) })
    }

    fn execute<'a, T, F>(&'a self, operation: &'static str, _work: F) -> RouteBoundOpenFuture<'a, T>
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
        authorization: RouteBoundOpenAuthorization::AuthenticatedDaemonCommand,
    }
}

#[tokio::test]
async fn coordinator_returns_typed_not_found_without_starting_a_runtime_effect() {
    let repository = StaticRepository {
        state: ServiceState::default(),
    };
    let supervisor = RouteBoundOpenSupervisor::system(Some(1_000), None);
    let mut runtime = ScriptedRuntime::new();
    let outcome = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::DurableResolution {
            handoff_id: "missing".to_string(),
            allow_reopen_closed: false,
            attribution: authorized_attribution(),
        },
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
        RouteBoundOpenInvocation::DurableResolution {
            handoff_id: "handoff-closed".to_string(),
            allow_reopen_closed: false,
            attribution: authorized_attribution(),
        },
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
        RouteBoundOpenInvocation::DirectOpen(Box::new(request)),
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
async fn coordinator_returns_provider_fallback_without_creating_a_duplicate_lane() {
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
                    "displayAllocationId": "display-a",
                    "displayName": ":31",
                    "displayIsolation": "shared_display",
                    "displayAccess": {"state": "ready"}
                }),
                state: "checked_out".to_string(),
                current_route_allocation_id: Some("route-a".to_string()),
                ..RoutePoolEntry::default()
            },
        )]),
        ..ServiceState::default()
    };
    let store = JsonServiceStateStore::new(&state_path);
    store.save(&initial).unwrap();
    let repository = FixtureRepository {
        repository: LockedServiceStateRepository::new(store.clone()),
    };
    let supervisor = RouteBoundOpenSupervisor::system(Some(10_000), None);
    let mut runtime = ScriptedRuntime::new();
    runtime.launch_issue = Some(profile_conflict_issue());

    let outcome = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::DurableResolution {
            handoff_id: "handoff-a".to_string(),
            allow_reopen_closed: false,
            attribution: authorized_attribution(),
        },
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();

    assert!(matches!(
        outcome,
        RouteBoundOpenOutcome::ProviderFallback { .. }
    ));
    assert_eq!(
        *runtime.events.lock().unwrap(),
        vec![
            "observe_browser",
            "ensure_display_access",
            "launch_browser",
            "observe_operator_access"
        ]
    );
    let retained = store.load().unwrap();
    assert_eq!(
        retained.remote_view_routes["route-a"].browser_id.as_deref(),
        Some("session:im-receipts")
    );
    assert!(retained.browsers.is_empty());

    runtime.launch_issue = None;
    runtime.events.lock().unwrap().clear();
    runtime.observation.browser_present = true;
    runtime.observation.browser_id = "session:im-receipts".to_string();
    runtime.observation.session_id = "im-receipts".to_string();
    runtime.observation.runtime_profile = Some("im-receipts-main".to_string());
    runtime.observation.active_target_id = Some("tab-a".to_string());
    runtime.observation.active_url = Some("https://example.test/".to_string());
    runtime.observation.pages = vec![PageInfo {
        target_id: "tab-a".to_string(),
        session_id: "page-session".to_string(),
        url: "https://example.test/".to_string(),
        title: "Example".to_string(),
        target_type: "page".to_string(),
    }];
    let reopened = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::DurableResolution {
            handoff_id: "handoff-a".to_string(),
            allow_reopen_closed: true,
            attribution: authorized_attribution(),
        },
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();
    assert!(matches!(reopened, RouteBoundOpenOutcome::Reopened { .. }));
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
        RouteBoundOpenInvocation::DirectOpen(Box::new(request)),
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

    let rollback_request = RouteBoundDirectOpenRequest::from_compatibility_command(
        json!({
            "action": "remote_view_open",
            "routePoolEntryId": "pool-a",
            "provider": "rdp_gateway",
            "runtimeProfile": "stealthcdp-default",
            "url": "https://example.test/",
            "serviceJobId": "job-rollback",
            "displayAllocationId": "remote-view-display:31",
            "routePoolEntry": {
                "id": "pool-a",
                "routeId": "route-a",
                "provider": "rdp_gateway",
                "frameUrl": "https://dashboard.example/guacamole/#/client/route-a",
                "externalUrl": "https://guac.example/#/client/route-a",
                "providerMode": "single_controller",
                "state": "available",
                "target": {
                    "displayAllocationId": "remote-view-display:31",
                    "displayName": ":31",
                    "displayIsolation": "shared_display",
                    "displayAccess": {"state": "ready"}
                }
            }
        }),
        None,
        authorized_attribution(),
    )
    .unwrap();
    runtime.events.lock().unwrap().clear();
    runtime.observation = ScriptedRuntime::new().observation;
    runtime.launch_issue = Some(RouteBoundRuntimeIssue::EffectFailed {
        operation: "launch_browser",
        message: "scripted launch failure".to_string(),
    });
    let rolled_back = RouteBoundOpenCoordinator::open(
        RouteBoundOpenInvocation::DirectOpen(Box::new(rollback_request)),
        &mut runtime,
        &repository,
        &supervisor,
    )
    .await
    .unwrap();
    assert!(matches!(
        rolled_back,
        RouteBoundOpenOutcome::RolledBack { .. }
    ));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn every_ingress_uses_the_same_transport_neutral_authorization_fact() {
    for ingress in ["daemon", "cli", "http", "mcp", "dashboard"] {
        let attribution = authorized_attribution();
        assert!(
            attribution.authorization.is_authorized(),
            "{ingress} must reach invocation construction only after daemon authentication"
        );
    }
    let rejected = RouteBoundDirectOpenRequest::from_compatibility_command(
        json!({"action": "remote_view_open"}),
        None,
        RouteBoundOpenAttribution {
            caller_id: None,
            service_job_id: None,
            authorization: RouteBoundOpenAuthorization::Rejected,
        },
    );
    assert_eq!(
        rejected.unwrap_err(),
        "Unauthorized route-bound open invocation"
    );
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

struct PendingRepository;

impl RouteBoundOpenRepository for PendingRepository {
    fn snapshot(&self) -> RouteBoundOpenFuture<'_, ServiceState> {
        Box::pin(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(ServiceState::default())
        })
    }

    fn execute<'a, T, F>(&'a self, operation: &'static str, _work: F) -> RouteBoundOpenFuture<'a, T>
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
        .forward("repository_load_snapshot", repository.snapshot())
        .await;

    assert!(matches!(
        result,
        Err(RouteBoundRuntimeIssue::ForwardDeadlineElapsed {
            operation: "repository_load_snapshot",
            total_ms: 2,
        })
    ));
}
