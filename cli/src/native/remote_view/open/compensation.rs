#![allow(unused_imports)]
use super::deadline::RouteBoundOpenSupervisor;
use super::route_lifecycle::service_remote_view_timestamp;
use super::runtime::{
    route_bound_runtime_issue, CloseCreatedBrowserRequest, CloseCreatedTargetRequest,
    RouteBoundOpenRuntime,
};
use super::shared::*;
pub(crate) async fn remote_view_open_cleanup_after_failure<R: RouteBoundOpenRuntime>(
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
    task: &RouteBoundHandoffFailureCleanupTask,
    created_target_id: Option<&str>,
) -> Value {
    let result = match task {
        RouteBoundHandoffFailureCleanupTask::CloseOpenedTab { .. } => match created_target_id {
            Some(target_id) => {
                supervisor
                    .compensate(
                        "close_created_target",
                        runtime.close_created_target(CloseCreatedTargetRequest {
                            target_id: target_id.to_string(),
                        }),
                    )
                    .await
            }
            None => Err(route_bound_runtime_issue(
                "close_created_target",
                "rollback_incomplete: created target identity was unavailable".to_string(),
                None,
            )),
        },
        RouteBoundHandoffFailureCleanupTask::CloseNewBrowser { command } => {
            supervisor
                .compensate(
                    "close_created_browser",
                    runtime.close_created_browser(CloseCreatedBrowserRequest {
                        browser_identity: command.clone(),
                    }),
                )
                .await
        }
        RouteBoundHandoffFailureCleanupTask::Skipped { cleanup } => {
            return cleanup.clone();
        }
    };
    route_bound_handoff_failure_cleanup_task_result(
        task,
        result.map_err(|issue| issue.compatibility_message().to_string()),
    )
}
pub(crate) struct RemoteViewOpenFailureCleanupInput<'a> {
    pub(crate) repository:
        &'a LockedServiceStateRepository<super::super::super::service_store::JsonServiceStateStore>,
    pub(crate) lease: &'a RemoteViewAcquisitionLease,
    pub(crate) phase: &'a str,
    pub(crate) error: &'a str,
    pub(crate) rollback_cleanup: &'a Value,
    pub(crate) launch: &'a Value,
    pub(crate) tab: Option<&'a Value>,
}
pub(crate) async fn remote_view_open_rollback_failure_after_cleanup<R: RouteBoundOpenRuntime>(
    runtime: &mut R,
    supervisor: &RouteBoundOpenSupervisor,
    input: RemoteViewOpenFailureCleanupInput<'_>,
) -> Result<RouteBoundHandoffFailureCleanupSummary, String> {
    let now = service_remote_view_timestamp();
    let recovery = begin_route_bound_handoff_failure_recovery(
        input.repository,
        RouteBoundHandoffFailureRecoveryInput {
            lease: input.lease,
            phase: input.phase,
            error: input.error,
            rollback_cleanup: input.rollback_cleanup,
            launch: input.launch,
            tab: input.tab,
            observed_at: &now,
        },
    )?;
    let created_target_id = input
        .tab
        .and_then(|tab| tab.get("targetId"))
        .and_then(Value::as_str);
    let cleanup = remote_view_open_cleanup_after_failure(
        runtime,
        supervisor,
        &recovery.cleanup_task,
        created_target_id,
    )
    .await;
    remote_view_open_complete_handoff_failure_cleanup(
        input.repository,
        &input.lease.id,
        &recovery.rollback,
        &cleanup,
    )
}
pub(crate) fn remote_view_open_complete_handoff_failure_cleanup(
    repository: &LockedServiceStateRepository<
        super::super::super::service_store::JsonServiceStateStore,
    >,
    lease_id: &str,
    rollback: &Value,
    cleanup: &Value,
) -> Result<RouteBoundHandoffFailureCleanupSummary, String> {
    let now = service_remote_view_timestamp();
    complete_route_bound_handoff_failure_cleanup(
        repository,
        RouteBoundHandoffFailureCleanupInput {
            lease_id,
            rollback,
            cleanup,
            observed_at: &now,
        },
    )
}
