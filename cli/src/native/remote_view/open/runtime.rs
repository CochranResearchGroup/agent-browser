#![allow(unused_imports)]
use super::deadline::{RouteBoundOpenFuture, RouteBoundRuntimeIssue};
use super::operator_route::{
    remote_view_open_ensure_display_access, remote_view_open_operator_access_readiness,
};
use super::proof::remote_view_open_visible_window_proof;
use super::route_lifecycle::handle_service_remote_view_route_checkout;
use super::shared::*;
use crate::native::service_store::JsonServiceStateStore;
/// Raw browser facts observed by the coordinator. The runtime adapter does
/// not decide whether a browser or target is reusable.
#[derive(Debug, Clone)]
pub(crate) struct RouteBoundBrowserObservation {
    pub(crate) browser_present: bool,
    pub(crate) browser_pid: Option<u32>,
    pub(crate) browser_id: String,
    pub(crate) session_id: String,
    pub(crate) runtime_profile: Option<String>,
    pub(crate) active_target_id: Option<String>,
    pub(crate) active_url: Option<String>,
    pub(crate) active_title: Option<String>,
    pub(crate) pages: Vec<PageInfo>,
}
#[derive(Debug, Clone)]
pub(crate) struct LaunchBrowserRequest {
    pub(crate) command: LaunchBrowserCommand,
}
#[derive(Debug, Clone)]
pub(crate) struct SwitchTargetRequest {
    pub(crate) target_id: String,
}
#[derive(Debug, Clone)]
pub(crate) struct NavigateTargetRequest {
    pub(crate) url: String,
}
#[derive(Debug, Clone)]
pub(crate) struct OpenTargetRequest {
    pub(crate) command: OpenTargetCommand,
}
#[derive(Debug, Clone)]
pub(crate) struct FocusTargetRequest {
    pub(crate) command: FocusTargetCommand,
}
#[derive(Debug, Clone)]
pub(crate) struct CloseCreatedTargetRequest {
    pub(crate) target_id: String,
}
#[derive(Debug, Clone)]
pub(crate) struct CloseCreatedBrowserRequest {
    pub(crate) browser_identity: Value,
}
#[derive(Debug, Clone)]
pub(crate) struct CheckoutRouteRequest {
    pub(crate) command: CheckoutRouteCommand,
}
#[derive(Debug, Clone)]
pub(crate) struct DisplayAccessRequest {
    pub(crate) binding: RemoteViewRouteBinding,
}
#[derive(Debug, Clone)]
pub(crate) struct VisibleWindowRequest {
    pub(crate) binding: RemoteViewRouteBinding,
}
#[derive(Debug, Clone)]
pub(crate) struct OperatorAccessRequest {
    pub(crate) binding: RemoteViewRouteBinding,
}

macro_rules! route_bound_document_type {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, PartialEq)]
            pub(crate) struct $name(Value);

            impl $name {
                pub(crate) fn as_value(&self) -> &Value {
                    &self.0
                }

                pub(crate) fn into_value(self) -> Value {
                    self.0
                }
            }

            impl From<Value> for $name {
                fn from(value: Value) -> Self {
                    Self(value)
                }
            }
        )+
    };
}

route_bound_document_type!(
    LaunchBrowserCommand,
    OpenTargetCommand,
    FocusTargetCommand,
    CheckoutRouteCommand,
    LaunchBrowserResult,
    SwitchTargetResult,
    NavigateTargetResult,
    OpenTargetResult,
    FocusTargetResult,
    CloseCreatedTargetResult,
    CloseCreatedBrowserResult,
    CheckoutRouteResult,
    DisplayAccessResult,
    VisibleWindowResult,
    OperatorAccessResult,
);
/// The frozen route-bound effect ledger. It deliberately has no generic
/// execute command or daemon-state escape hatch.
pub(crate) trait RouteBoundOpenRuntime {
    fn observe_browser(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation>;
    fn launch_browser(
        &mut self,
        request: LaunchBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, LaunchBrowserResult>;
    fn refresh_targets(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation>;
    fn switch_target(
        &mut self,
        request: SwitchTargetRequest,
    ) -> RouteBoundOpenFuture<'_, SwitchTargetResult>;
    fn navigate_target(
        &mut self,
        request: NavigateTargetRequest,
    ) -> RouteBoundOpenFuture<'_, NavigateTargetResult>;
    fn open_target(
        &mut self,
        request: OpenTargetRequest,
    ) -> RouteBoundOpenFuture<'_, OpenTargetResult>;
    fn focus_target(
        &mut self,
        request: FocusTargetRequest,
    ) -> RouteBoundOpenFuture<'_, FocusTargetResult>;
    fn close_created_target(
        &mut self,
        request: CloseCreatedTargetRequest,
    ) -> RouteBoundOpenFuture<'_, CloseCreatedTargetResult>;
    fn close_created_browser(
        &mut self,
        request: CloseCreatedBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, CloseCreatedBrowserResult>;
    fn checkout_route(
        &mut self,
        request: CheckoutRouteRequest,
    ) -> RouteBoundOpenFuture<'_, CheckoutRouteResult>;
    fn ensure_display_access(
        &mut self,
        request: DisplayAccessRequest,
    ) -> RouteBoundOpenFuture<'_, DisplayAccessResult>;
    fn observe_visible_window(
        &mut self,
        request: VisibleWindowRequest,
    ) -> RouteBoundOpenFuture<'_, VisibleWindowResult>;
    fn observe_operator_access(
        &mut self,
        request: OperatorAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Option<OperatorAccessResult>>;
}

/// Deadline-supervised repository work used by route-bound coordination.
/// The future is owned by the coordinator and is never detached, so dropping
/// the coordinator at the total deadline leaves no repository task behind.
pub(crate) trait RouteBoundOpenRepository {
    fn snapshot(&self) -> RouteBoundOpenFuture<'_, ServiceState>;

    fn execute<'a, T, F>(&'a self, operation: &'static str, work: F) -> RouteBoundOpenFuture<'a, T>
    where
        T: Send + 'a,
        F: FnOnce(&LockedServiceStateRepository<JsonServiceStateStore>) -> Result<T, String>
            + Send
            + 'a;
}

pub(crate) struct DaemonRouteBoundOpenRepository {
    repository: LockedServiceStateRepository<JsonServiceStateStore>,
}

impl DaemonRouteBoundOpenRepository {
    pub(crate) fn new() -> Result<Self, String> {
        Ok(Self {
            repository: LockedServiceStateRepository::default_json()?,
        })
    }
}

impl RouteBoundOpenRepository for DaemonRouteBoundOpenRepository {
    fn snapshot(&self) -> RouteBoundOpenFuture<'_, ServiceState> {
        Box::pin(async move {
            self.repository.load_snapshot().map_err(|message| {
                RouteBoundRuntimeIssue::EffectFailed {
                    operation: "repository_load_snapshot",
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

/// Permanent daemon and browser effect adapter. Route selection, reuse,
/// proof classification, and compensation ownership stay in the coordinator.
pub(crate) struct DaemonRouteBoundOpenRuntime<'a> {
    pub(crate) state: &'a mut DaemonState,
}
impl<'a> DaemonRouteBoundOpenRuntime<'a> {
    pub(crate) fn new(state: &'a mut DaemonState) -> Self {
        Self { state }
    }
}
pub(crate) async fn observe_daemon_browser(
    state: &mut DaemonState,
) -> Result<RouteBoundBrowserObservation, RouteBoundRuntimeIssue> {
    let session_id = state.session_id.clone();
    let browser_id = service_browser_id(&session_id);
    let Some(manager) = state.browser.as_mut() else {
        return Ok(RouteBoundBrowserObservation {
            browser_present: false,
            browser_pid: None,
            browser_id,
            session_id,
            runtime_profile: None,
            active_target_id: None,
            active_url: None,
            active_title: None,
            pages: Vec::new(),
        });
    };
    Ok(RouteBoundBrowserObservation {
        browser_present: true,
        browser_pid: manager.browser_pid(),
        browser_id,
        session_id,
        runtime_profile: manager.runtime_profile_name().map(str::to_string),
        active_target_id: manager.active_target_id().ok().map(str::to_string),
        active_url: manager.get_url().await.ok(),
        active_title: manager.get_title().await.ok(),
        pages: manager.pages_list(),
    })
}
pub(crate) fn route_bound_runtime_issue(
    operation: &'static str,
    message: String,
    command: Option<&Value>,
) -> RouteBoundRuntimeIssue {
    let marker = "already in use by PID";
    if let Some(marker_index) = message.find(marker) {
        let pid = message[marker_index + marker.len()..]
            .trim_start_matches(|character: char| {
                character.is_ascii_whitespace() || matches!(character, ':' | '=')
            })
            .split(|character: char| !character.is_ascii_digit())
            .next()
            .and_then(|value| value.parse::<u32>().ok());
        if let Some(pid) = pid {
            let profile_id = command
                .and_then(|command| {
                    optional_command_string(command, "runtimeProfile")
                        .or_else(|| optional_command_string(command, "profileId"))
                        .or_else(|| optional_command_string(command, "profile"))
                })
                .unwrap_or_default();
            return RouteBoundRuntimeIssue::RequestedProfileInUseByPid {
                profile_id,
                pid,
                owner_browser_id: command
                    .and_then(|command| optional_command_string(command, "browserId")),
                owner_session_id: command
                    .and_then(|command| optional_command_string(command, "sessionName")),
                compatibility_message: message,
            };
        }
    }
    RouteBoundRuntimeIssue::EffectFailed { operation, message }
}
impl RouteBoundOpenRuntime for DaemonRouteBoundOpenRuntime<'_> {
    fn observe_browser(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation> {
        Box::pin(observe_daemon_browser(self.state))
    }
    fn launch_browser(
        &mut self,
        request: LaunchBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, LaunchBrowserResult> {
        Box::pin(async move {
            handle_launch(request.command.as_value(), self.state)
                .await
                .map(LaunchBrowserResult::from)
                .map_err(|message| {
                    route_bound_runtime_issue(
                        "launch_browser",
                        message,
                        Some(request.command.as_value()),
                    )
                })
        })
    }
    fn refresh_targets(&mut self) -> RouteBoundOpenFuture<'_, RouteBoundBrowserObservation> {
        Box::pin(async move {
            self.state.drain_cdp_events_background().await;
            observe_daemon_browser(self.state).await
        })
    }
    fn switch_target(
        &mut self,
        request: SwitchTargetRequest,
    ) -> RouteBoundOpenFuture<'_, SwitchTargetResult> {
        Box::pin(async move {
            let manager = self.state.browser.as_mut().ok_or_else(|| {
                route_bound_runtime_issue("switch_target", "Browser not launched".to_string(), None)
            })?;
            manager
                .tab_switch_target_id(&request.target_id)
                .await
                .map(SwitchTargetResult::from)
                .map_err(|message| route_bound_runtime_issue("switch_target", message, None))
        })
    }
    fn navigate_target(
        &mut self,
        request: NavigateTargetRequest,
    ) -> RouteBoundOpenFuture<'_, NavigateTargetResult> {
        Box::pin(async move {
            let manager = self.state.browser.as_mut().ok_or_else(|| {
                route_bound_runtime_issue(
                    "navigate_target",
                    "Browser not launched".to_string(),
                    None,
                )
            })?;
            manager
                .navigate(&request.url, WaitUntil::None)
                .await
                .map(NavigateTargetResult::from)
                .map_err(|message| route_bound_runtime_issue("navigate_target", message, None))
        })
    }
    fn open_target(
        &mut self,
        request: OpenTargetRequest,
    ) -> RouteBoundOpenFuture<'_, OpenTargetResult> {
        Box::pin(async move {
            handle_tab_new(request.command.as_value(), self.state)
                .await
                .map(OpenTargetResult::from)
                .map_err(|message| {
                    route_bound_runtime_issue(
                        "open_target",
                        message,
                        Some(request.command.as_value()),
                    )
                })
        })
    }
    fn focus_target(
        &mut self,
        request: FocusTargetRequest,
    ) -> RouteBoundOpenFuture<'_, FocusTargetResult> {
        Box::pin(async move {
            handle_view_focus(request.command.as_value(), self.state)
                .await
                .map(FocusTargetResult::from)
                .map_err(|message| {
                    route_bound_runtime_issue(
                        "focus_target",
                        message,
                        Some(request.command.as_value()),
                    )
                })
        })
    }
    fn close_created_target(
        &mut self,
        request: CloseCreatedTargetRequest,
    ) -> RouteBoundOpenFuture<'_, CloseCreatedTargetResult> {
        Box::pin(async move {
            let manager = self.state.browser.as_mut().ok_or_else(|| {
                route_bound_runtime_issue(
                    "close_created_target",
                    "Browser not launched".to_string(),
                    None,
                )
            })?;
            manager
                .tab_close_target_id(&request.target_id)
                .await
                .map(CloseCreatedTargetResult::from)
                .map_err(|message| route_bound_runtime_issue("close_created_target", message, None))
        })
    }
    fn close_created_browser(
        &mut self,
        request: CloseCreatedBrowserRequest,
    ) -> RouteBoundOpenFuture<'_, CloseCreatedBrowserResult> {
        Box::pin(async move {
            let _ = request.browser_identity;
            handle_close(self.state)
                .await
                .map(CloseCreatedBrowserResult::from)
                .map_err(|message| {
                    route_bound_runtime_issue("close_created_browser", message, None)
                })
        })
    }
    fn checkout_route(
        &mut self,
        request: CheckoutRouteRequest,
    ) -> RouteBoundOpenFuture<'_, CheckoutRouteResult> {
        Box::pin(async move {
            handle_service_remote_view_route_checkout(request.command.as_value(), self.state)
                .await
                .map(CheckoutRouteResult::from)
                .map_err(|message| {
                    route_bound_runtime_issue(
                        "checkout_route",
                        message,
                        Some(request.command.as_value()),
                    )
                })
        })
    }
    fn ensure_display_access(
        &mut self,
        request: DisplayAccessRequest,
    ) -> RouteBoundOpenFuture<'_, DisplayAccessResult> {
        Box::pin(async move {
            remote_view_open_ensure_display_access(&request.binding)
                .map(DisplayAccessResult::from)
                .map_err(|message| {
                    route_bound_runtime_issue("ensure_display_access", message, None)
                })
        })
    }
    fn observe_visible_window(
        &mut self,
        request: VisibleWindowRequest,
    ) -> RouteBoundOpenFuture<'_, VisibleWindowResult> {
        Box::pin(async move {
            remote_view_open_visible_window_proof(&request.binding)
                .map(VisibleWindowResult::from)
                .map_err(|message| {
                    route_bound_runtime_issue("observe_visible_window", message, None)
                })
        })
    }
    fn observe_operator_access(
        &mut self,
        request: OperatorAccessRequest,
    ) -> RouteBoundOpenFuture<'_, Option<OperatorAccessResult>> {
        Box::pin(async move {
            Ok(remote_view_open_operator_access_readiness(&request.binding)
                .await
                .map(OperatorAccessResult::from))
        })
    }
}
