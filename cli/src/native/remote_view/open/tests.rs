use super::shared::*;
use super::*;
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
    ) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("launch_browser", json!({ "launched": true }))
    }

    fn refresh_targets(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation> {
        Box::pin(async move {
            self.events.lock().unwrap().push("refresh_targets");
            Ok(self.observation.clone())
        })
    }

    fn switch_target(&mut self, _request: SwitchTargetRequest) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("switch_target", json!({ "targetId": "target-1" }))
    }

    fn navigate_target(
        &mut self,
        _request: NavigateTargetRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("navigate_target", json!({ "url": "https://example.test" }))
    }

    fn open_target(&mut self, _request: OpenTargetRequest) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("open_target", json!({ "targetId": "target-1" }))
    }

    fn focus_target(&mut self, _request: FocusTargetRequest) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("focus_target", json!({ "focused": true }))
    }

    fn close_created_target(
        &mut self,
        _request: CloseCreatedTargetRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("close_created_target", json!({ "closed": true }))
    }

    fn close_created_browser(
        &mut self,
        _request: CloseCreatedBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("close_created_browser", json!({ "closed": true }))
    }

    fn checkout_route(
        &mut self,
        _request: CheckoutRouteRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("checkout_route", json!({ "status": "ready" }))
    }

    fn ensure_display_access(
        &mut self,
        _request: DisplayAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("ensure_display_access", json!({ "state": "ready" }))
    }

    fn observe_visible_window(
        &mut self,
        _request: VisibleWindowRequest,
    ) -> RouteBoundOpenFuture<'_, Value> {
        self.effect("observe_visible_window", json!({ "state": "ready" }))
    }

    fn observe_operator_access(
        &mut self,
        _request: OperatorAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Option<Value>> {
        Box::pin(async move {
            self.events.lock().unwrap().push("observe_operator_access");
            Ok(Some(json!({ "state": "ready" })))
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
            0 => {
                supervisor
                    .forward(
                        "launch_browser",
                        runtime.launch_browser(LaunchBrowserRequest { command: json!({}) }),
                    )
                    .await
            }
            1 => {
                supervisor
                    .forward(
                        "switch_target",
                        runtime.switch_target(SwitchTargetRequest {
                            target_id: "target-1".to_string(),
                        }),
                    )
                    .await
            }
            2 => {
                supervisor
                    .forward(
                        "navigate_target",
                        runtime.navigate_target(NavigateTargetRequest {
                            url: "https://example.test".to_string(),
                        }),
                    )
                    .await
            }
            3 => {
                supervisor
                    .forward(
                        "open_target",
                        runtime.open_target(OpenTargetRequest { command: json!({}) }),
                    )
                    .await
            }
            4 => {
                supervisor
                    .forward(
                        "focus_target",
                        runtime.focus_target(FocusTargetRequest { command: json!({}) }),
                    )
                    .await
            }
            5 => {
                supervisor
                    .forward(
                        "close_created_target",
                        runtime.close_created_target(CloseCreatedTargetRequest {
                            target_id: "target-1".to_string(),
                        }),
                    )
                    .await
            }
            6 => {
                supervisor
                    .forward(
                        "close_created_browser",
                        runtime.close_created_browser(CloseCreatedBrowserRequest {
                            browser_identity: json!({}),
                        }),
                    )
                    .await
            }
            7 => {
                supervisor
                    .forward(
                        "checkout_route",
                        runtime.checkout_route(CheckoutRouteRequest { command: json!({}) }),
                    )
                    .await
            }
            _ => {
                supervisor
                    .forward(
                        "ensure_display_access",
                        runtime.ensure_display_access(DisplayAccessRequest {
                            binding: test_binding(),
                        }),
                    )
                    .await
            }
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
            runtime.open_target(OpenTargetRequest { command: json!({}) }),
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
            runtime.launch_browser(LaunchBrowserRequest { command: json!({}) }),
        )
        .await
        .unwrap();
    supervisor
        .forward(
            "open_target",
            runtime.open_target(OpenTargetRequest { command: json!({}) }),
        )
        .await
        .unwrap();
    assert_eq!(
        *runtime.events.lock().unwrap(),
        vec!["observe_browser", "launch_browser", "open_target"]
    );
}
