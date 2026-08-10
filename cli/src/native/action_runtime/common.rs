#![allow(unused_imports)]
pub(crate) use super::super::auth;
pub(crate) use super::super::browser::{
    should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo, ProcessExitObservation,
    WaitUntil,
};
pub(crate) use super::super::cancellation::CancellationToken;
pub(crate) use super::super::cdp::chrome::{
    launch_chrome_detached, LaunchOptions, ManualChromeLaunch,
};
pub(crate) use super::super::cdp::client::CdpClient;
pub(crate) use super::super::cdp::types::{
    AttachToTargetParams, AttachToTargetResult, CdpEvent, CreateTargetResult,
    DispatchMouseEventParams, ExceptionThrownEvent, JavascriptDialogOpeningEvent,
    TargetCreatedEvent, TargetDestroyedEvent, TargetInfoChangedEvent,
};
pub(crate) use super::super::cookies;
pub(crate) use super::super::diff;
pub(crate) use super::super::element::RefMap;
pub(crate) use super::super::inspect_server::InspectServer;
pub(crate) use super::super::interaction;
pub(crate) use super::super::network::{self, DomainFilter, EventTracker};
pub(crate) use super::super::policy::{ActionPolicy, ConfirmActions, PolicyResult};
pub(crate) use super::super::providers;
pub(crate) use super::super::recording::{self, RecordingState};
pub(crate) use super::super::remote_view::{
    display_allocation_id_for_route_pool_entry, normalize_remote_view_open_intent,
    plan_remote_view_acquisition, readiness_state, route_binding_readiness,
    route_bound_display_content, route_display_content, visible_browser_window_proof,
    RemoteViewAcquisitionPlan, RemoteViewRouteBinding,
};
pub(crate) use super::super::remote_view_attachability::refresh_remote_view_attachability;
pub(crate) use super::super::remote_view_handoff::{
    apply_retained_remote_view_route, begin_route_bound_handoff_failure_recovery,
    begin_route_bound_handoff_plan_acquisition, complete_route_bound_handoff_failure_cleanup,
    complete_route_bound_handoff_open, planned_route_bound_handoff_response,
    remote_view_handoff_resolution_command, remote_view_handoff_was_explicitly_closed,
    route_bound_handoff_checkout_command_with_visible_window_proof,
    route_bound_handoff_checkout_failure, route_bound_handoff_failure_cleanup_task_result,
    route_bound_handoff_focus_command, route_bound_handoff_focus_failure,
    route_bound_handoff_immediate_failure, route_bound_handoff_launch_failure_cleanup,
    route_bound_handoff_operator_visible,
    route_bound_handoff_operator_visible_failure_if_not_ready, route_bound_handoff_plan,
    route_bound_handoff_post_checkout_proof, route_bound_handoff_pre_launch_failure_cleanup,
    route_bound_handoff_reused_browser_launch_result, route_bound_handoff_tab_open_failure,
    route_bound_handoff_target_url_readiness, route_bound_handoff_visible_window_proof_failure,
    shared_profile_acquisition_result, CompleteRouteBoundHandoffOpenInput,
    RouteBoundHandoffFailureCleanupInput, RouteBoundHandoffFailureCleanupSummary,
    RouteBoundHandoffFailureCleanupTask, RouteBoundHandoffFailureRecoveryInput,
    RouteBoundHandoffImmediateFailureInput, RouteBoundHandoffPlan,
    RouteBoundHandoffPlannedResponseInput, RouteBoundHandoffPostCheckoutProofInput,
    SharedProfileAcquisitionResultInput,
};
pub(crate) use super::super::screenshot::{self, ScreenshotOptions};
pub(crate) use super::super::service_access::{
    service_access_plan_for_state, ServiceAccessPlanRequest,
};
pub(crate) use super::super::service_activity::service_incident_activity_response;
pub(crate) use super::super::service_config::{
    delete_persisted_monitor, delete_persisted_profile, delete_persisted_provider,
    delete_persisted_session, delete_persisted_site_policy, reset_persisted_monitor_failures,
    update_persisted_monitor_state, update_persisted_profile_freshness,
    update_persisted_profile_seeding_handoff, upsert_persisted_browser_capability_registry_record,
    upsert_persisted_monitor, upsert_persisted_profile, upsert_persisted_provider,
    upsert_persisted_session, upsert_persisted_site_policy,
};
pub(crate) use super::super::service_health::{
    persist_browser_recovery_started_in_repository, persist_closed_browser_health_in_repository,
    persist_current_browser_stale_health_in_repository,
    persist_reconciled_service_state_in_repository, persist_service_browser_record_in_repository,
    reconcile_service_state, retry_degraded_service_browser_in_state,
    retry_persisted_service_browser_in_repository, retry_service_browser_in_state,
    BrowserRecoveryPersistence, BrowserRecoveryPolicyConfig, BrowserRecoveryPolicySource,
    BrowserRecoveryPolicyValueSource, BrowserRecoveryReasonKind,
};
pub(crate) use super::super::service_incidents::{
    acknowledge_persisted_service_incident, apply_persisted_service_remedies,
    resolve_persisted_service_incident, service_incident_summary, service_incidents_response,
    triage_persisted_service_monitor, ServiceIncidentFilters,
};
pub(crate) use super::super::service_jobs::cancel_persisted_service_job;
pub(crate) use super::super::service_lifecycle::{
    profile_lease_telemetry, select_service_profile_for_request, service_profile_id,
    ProfileSelectionRequest, ServiceLaunchMetadata,
};
pub(crate) use super::super::service_model::{
    retained_display_allocation_candidates, service_profile_allocations,
    service_profile_seeding_handoff, service_profile_sources, BrowserBuild,
    BrowserCapabilityRegistry, BrowserHealth as ServiceBrowserHealth,
    BrowserHost as ServiceBrowserHost, BrowserProcess, BrowserProfile, BrowserSession, BrowserTab,
    ControlInputProvider, DisplayAllocation, JobState as ServiceJobState, LeaseState, MonitorState,
    ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy, ProfileLeaseDisposition,
    ProfileOrigin, ProfileSelectionReason, RemoteViewAcquisitionLease, RemoteViewHandoff,
    RemoteViewRoute, RoutePoolEntry, ServiceEntitySource, ServiceEvent, ServiceEventKind,
    ServiceState, ServiceTabHandle, SessionCleanupPolicy, TabLifecycle, ViewStream,
    ViewStreamProvider, ViewerLease,
};
pub(crate) use super::super::service_monitors::{
    parse_monitor_state, run_due_persisted_monitors, service_monitors_response,
    MonitorCollectionFilters,
};
pub(crate) use super::super::service_resources::{
    service_gc_apply_response, service_gc_dry_run_response,
    service_resources_monitor_summary_response, service_resources_response,
    service_resources_write_monitor_summary_response,
};
pub(crate) use super::super::service_store::{
    LockedServiceStateRepository, ServiceStateRepository,
};
pub(crate) use super::super::service_trace::{service_trace_response, ServiceTraceFilters};
pub(crate) use super::super::snapshot::{self, SnapshotOptions};
pub(crate) use super::super::state;
pub(crate) use super::super::storage;
pub(crate) use super::super::stream::{self, StreamServer};
pub(crate) use super::super::tracing::{self as native_tracing, TracingState};
pub(crate) use super::super::webdriver::appium::AppiumManager;
pub(crate) use super::super::webdriver::backend::{
    BrowserBackend, WebDriverBackend, WEBDRIVER_UNSUPPORTED_ACTIONS,
};
pub(crate) use super::super::webdriver::ios;
pub(crate) use super::super::webdriver::safari;
pub(crate) use crate::connection::get_socket_dir;
pub(crate) use crate::runtime_profile::{
    clear_runtime_state, looks_like_path, pid_is_running, read_devtools_port, read_runtime_state,
    runtime_profile_user_data_dir,
};
pub(crate) use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
pub(crate) use chrono::{DateTime, FixedOffset};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use serde_json::{json, Map, Value};
pub(crate) use sha2::{Digest, Sha256};
pub(crate) use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
pub(crate) use std::env;
pub(crate) use std::fs;
pub(crate) use std::future::Future;
pub(crate) use std::io::Write;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process::{Command, Stdio};
pub(crate) use std::sync::atomic::AtomicU64;
pub(crate) use std::sync::Arc;
pub(crate) use std::time::{Duration, Instant};
pub(crate) use time::{format_description::well_known::Rfc3339, OffsetDateTime};
pub(crate) use tokio::sync::{broadcast, oneshot, RwLock};

/// Stable cancellation error shared by dispatcher and domain action owners.
pub(crate) fn cancellation_error() -> String {
    "Service job was cancelled while running".to_string()
}

/// Await one action effect or the daemon's cooperative cancellation signal.
#[rustfmt::skip]
pub(crate) async fn cancellable<F, T>(future: F, cancellation: Option<CancellationToken>) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    let Some(cancellation) = cancellation else {
        return future.await;
    };
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => Err(cancellation_error()),
        result = future => result,
    }
}
