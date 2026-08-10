#[allow(dead_code, unused_imports)]
pub(crate) mod service_commands {
    use crate::native::action_runtime::common::*;
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::service_diagnostics::truncate_utf8;
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct ServiceRoutePoolRepairOptions {
        pub(crate) apply: bool,
        pub(crate) stale_checkouts: bool,
        pub(crate) stale_pending_acquisitions: bool,
    }
    impl ServiceRoutePoolRepairOptions {
        pub(crate) fn from_command(cmd: &Value) -> Self {
            Self {
                apply: cmd.get("apply").and_then(Value::as_bool).unwrap_or(false),
                stale_checkouts: cmd
                    .get("staleCheckouts")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                stale_pending_acquisitions: cmd
                    .get("stalePendingAcquisitions")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            }
        }
    }
}
pub(crate) use service_commands::*;
