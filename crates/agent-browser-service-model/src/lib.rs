//! Provider-free durable record models for Agent Browser.
//!
//! This module owns record compatibility and deterministic lifecycle decisions.
//! Adapters own persistence, browser observation, process control, and transport.

mod abandoned_browser_retirement;
mod browser_capability_registry;
mod browser_desktop_selector;
mod browser_process;
mod browser_profile;
mod browser_profile_catalog;
mod browser_retirement;
mod browser_session_manager;
mod crash_regeneration;
mod entity_source;
mod failure_recourse;
mod incident;
mod job_event;
mod monitor;
mod operational_snapshot;
mod presentation;
mod presentation_capacity;
mod presentation_request_queue;
mod presentation_scale_in;
mod principal_provenance;
mod profile_access;
mod profile_lifecycle;
mod profile_policy_migration;
mod profile_readiness;
mod profile_recovery_receipt;
mod profile_reset_receipt;
mod profile_seeding;
mod request_provenance;
mod route_keeper;
mod service_authentication_run;
mod service_challenge_task;
mod service_state;
mod session_tab;
mod site_policy;
mod terminal_outcome;

pub use abandoned_browser_retirement::{
    AbandonedBrowserRetirementPlan, AbandonedBrowserRetirementReceipt,
    AbandonedBrowserRetirementTransaction, ResourceRetirementPolicy, RetirementExitEvidence,
    RetirementExitFailure, RetirementRecourse, RetirementTerminalProjection,
    ABANDONED_BROWSER_RETIREMENT_PLAN_SCHEMA_V1,
};
pub use browser_capability_registry::{
    browser_profile_compatibility_matches, BrowserCapabilityRegistry,
};
pub use browser_desktop_selector::{
    select_browser_desktop_with_capacity, select_least_crowded_browser_desktop,
    BrowserDesktopAssignment, BrowserDesktopRoute,
};
pub use browser_process::{
    BrowserHealth, BrowserHealthObservation, BrowserProcess, BrowserRecordAuthoritySource,
    BrowserRecordLifecycleClassification, BrowserRecordProvenance, BrowserRecordSource,
    ProtectedBrowserOwnerObservation, RecordedProcessIdentity, ServiceBrowserProcessIdentity,
    SERVICE_BROWSER_HEALTH_VALUES,
};
pub use browser_profile::{
    BrowserHost, BrowserProfile, BrowserProfileRegistration, ProfileClass, ProfileOrigin,
    ProfileSourceRecord, SitePolicySourceRecord, SERVICE_BROWSER_HOST_VALUES,
    SERVICE_PROFILE_CLASS_VALUES,
};
pub use browser_profile_catalog::{
    BrowserDisposableProfilePolicy, BrowserProfileCatalog, BrowserProfileCatalogDiagnostic,
    BrowserProfileCatalogEntry, BrowserProfileCatalogImport, BrowserProfileKind,
    BROWSER_PROFILE_CATALOG_SCHEMA_V1,
};
pub use browser_retirement::{
    BrowserContaminationReport, BrowserRetirementPlan, BrowserRetirementReceipt,
    BROWSER_RETIREMENT_PLAN_SCHEMA_V1, BROWSER_RETIREMENT_RECEIPT_SCHEMA_V1,
};
pub use browser_session_manager::{
    BrowserLaunch, BrowserNavigationRecord, BrowserOpenReservation, BrowserProfileIntent,
    BrowserSessionEffects, BrowserSessionManager, BrowserSessionManagerConfig, BrowserSessionState,
    BrowserTabAcquisition, BrowserTabEndReason, BrowserTabSource, CloseBrowserSessionResult,
    CloseBrowserTabResult, FocusBrowserResult, ManagedBrowserInstance, ManagedBrowserSession,
    ManagedBrowserTab, ManagedDisposableProfile, OpenBrowserSession, OpenBrowserSessionResult,
    ReapBrowserSessionsResult, SessionBrowserDisposition, SessionCloseDisposition,
    SessionEndReason, SessionRecordDisposition, TerminalBrowserSession, TerminalBrowserTab,
    BROWSER_SESSION_STATE_SCHEMA_V1,
};
pub use crash_regeneration::{
    apply_phase_receipt, begin_or_resume, crash_regeneration_statuses, finish_ready, interrupt,
    next_phase, validate_receipt, CrashRegenerationEvidence, CrashRegenerationOperation,
    CrashRegenerationPhase, CrashRegenerationPhaseReceipt, CrashRegenerationRequest,
    CrashRegenerationStableIdentities, CrashRegenerationState, CrashRegenerationStatus,
    CrashRegenerationTransaction, CRASH_REGENERATION_STATUS_SCHEMA_VERSION,
};
pub use entity_source::{ServiceEntitySource, ServiceEntitySources};
pub use failure_recourse::{
    child_access_failure_evidence, classify_service_failure, operator_focus_failure_code,
    profile_child_denial_error, ServiceEffectState, ServiceFailureAxis, ServiceFailurePhase,
    ServiceFailureRecourse, ServiceRetryDisposition, SERVICE_FAILURE_RECOURSE_SCHEMA_VERSION,
};
pub use incident::{
    ServiceIncident, ServiceIncidentEscalation, ServiceIncidentSeverity, ServiceIncidentState,
    SERVICE_INCIDENT_ESCALATION_VALUES, SERVICE_INCIDENT_SEVERITY_VALUES,
    SERVICE_INCIDENT_STATE_VALUES,
};
pub use job_event::{
    JobControlPlaneMode, JobPriority, JobState, JobTarget, ServiceEvent, ServiceEventKind,
    ServiceJob, SERVICE_EVENT_KIND_VALUES, SERVICE_JOB_CONTROL_PLANE_MODE_VALUES,
    SERVICE_JOB_NAMING_WARNING_MISSING_AGENT_NAME, SERVICE_JOB_NAMING_WARNING_MISSING_SERVICE_NAME,
    SERVICE_JOB_NAMING_WARNING_MISSING_TASK_NAME, SERVICE_JOB_NAMING_WARNING_VALUES,
    SERVICE_JOB_PRIORITY_VALUES, SERVICE_JOB_STATE_VALUES,
};
pub use monitor::{MonitorState, MonitorTarget, SiteMonitor, SERVICE_MONITOR_STATE_VALUES};
pub use operational_snapshot::{ControlPlaneSnapshot, ServiceReconciliationSnapshot};
pub use presentation::{
    route_pool_entry_matches_display, route_pool_target_string, ControlInputProvider,
    DisplayAllocation, DurableHandoffPresentationReceipt, RemoteViewAcquisitionLease,
    RemoteViewHandoff, RemoteViewRoute, RetainedDisplayAllocationCandidate, RoutePoolEntry,
    ViewStream, ViewStreamProvider, ViewerLease, SERVICE_CONTROL_INPUT_PROVIDER_VALUES,
    SERVICE_VIEW_STREAM_PROVIDER_VALUES,
};
pub use presentation_capacity::{
    CapacityDecision, CapacityLimitingResource, CapacityNextSafeAction,
    PresentationAcquisitionRetention, PresentationCapacityAuthority, PresentationCapacityConfig,
    PresentationCapacityObservations, PresentationCapacityProjection,
    PresentationInventoryCustodyObservation, PresentationPriority, PresentationRequest,
    PresentationRetirementConflict, PresentationSlot, PresentationSlotObservation,
    PresentationSlotState, PressureAdmission, SlotTransitionReceipt,
};
pub use presentation_scale_in::{PresentationScaleInIdleEvidence, PresentationScaleInState};
pub use profile_access::{
    effective_profile_permissions, evaluate_profile_access, evaluate_profile_child_access,
    mutate_profile_policy, profile_policy_target_for_preset, record_profile_eviction_receipt,
    ProfileAccessDrain, ProfileAccessEvaluation, ProfileAccessGrant, ProfileAccessMode,
    ProfileAccessNextAction, ProfileAccessPolicyState, ProfileAccessPreset, ProfileAccessResource,
    ProfileAccessSubject, ProfileChildAccess, ProfileChildAccessEvidence,
    ProfileChildAccessRequest, ProfileChildAccessResult, ProfileConnectionState,
    ProfileEvictionMode, ProfileEvictionPlan, ProfileEvictionReceipt, ProfileIdentityAssurance,
    ProfilePermission, ProfilePolicyAuditReceipt, ProfilePolicyMutationFailure,
    ProfilePolicyMutationOutcome, ProfilePolicyMutationRequest, ProfilePolicyMutationResult,
    ProfilePolicyRevisionDiff, ProfilePolicyTarget, ServiceProfileAccessDecision,
    ServiceProfileAccessPolicy, PROFILE_ACCESS_DECISION_SCHEMA_V1, PROFILE_ACCESS_POLICY_SCHEMA_V1,
    PROFILE_CHILD_ACCESS_SCHEMA_V1,
};
pub use profile_lifecycle::{
    register_profile_eviction_authorization, ProfileLifecycleAuthorization,
    ProfileLifecycleAuthorizationState, ProfileLifecycleEffectReceipt, ProfileLifecycleProof,
    PROFILE_LIFECYCLE_AUTHORIZATION_SCHEMA_V1, PROFILE_LIFECYCLE_PROOF_SCHEMA_V1,
    PROFILE_LIFECYCLE_RECEIPT_SCHEMA_V1,
};
pub use profile_policy_migration::{
    ProfilePolicyMigrationEntry, ProfilePolicyMigrationReport,
    PROFILE_POLICY_MIGRATION_SCHEMA_VERSION,
};
pub use profile_readiness::{
    BrowserBuild, BrowserProfileCompatibilityEvidence, ProfileAllocationPolicy,
    ProfileKeyringPolicy, ProfileReadinessState, ProfileTargetReadiness,
    SERVICE_BROWSER_BUILD_VALUES, SERVICE_PROFILE_ALLOCATION_VALUES,
    SERVICE_PROFILE_KEYRING_VALUES, SERVICE_PROFILE_READINESS_VALUES,
};
pub use profile_recovery_receipt::{
    ProfileAcquisitionState, RecoveryReceipt, PROFILE_RECOVERY_RECEIPT_SCHEMA_V1,
};
pub use profile_reset_receipt::{
    ProfileResetReceipt, ProfileResetScope, PROFILE_RESET_RECEIPT_SCHEMA_V1,
};
pub use profile_seeding::{
    profile_seeding_handoff_id, ProfileSeedingHandoffRecord, ProfileSeedingHandoffState,
    ProfileSeedingMode, SERVICE_PROFILE_SEEDING_HANDOFF_STATE_VALUES,
    SERVICE_PROFILE_SEEDING_MODE_VALUES,
};
pub use request_provenance::{
    normalize_identity_assurance, stable_self_declared_subject, ServiceRequestProvenance,
    SERVICE_REQUEST_PROVENANCE_SCHEMA_VERSION,
};
pub use route_keeper::{
    RouteKeeperAdoptionReceipt, RouteKeeperAuthority, RouteKeeperCleanupObligation,
    RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog, RouteKeeperFence,
    RouteKeeperHandoffBinding, RouteKeeperHostProcessClaim, RouteKeeperPhase, RouteKeeperPolicy,
    RouteKeeperProjection, RouteKeeperProtocolReadyReceipt, RouteKeeperProviderState,
    RouteKeeperReconcileAction, RouteKeeperRecord, RouteKeeperStartPriority,
    RouteKeeperStopDisposition, RouteKeeperStopReceipt, RouteKeeperXrdpOwnershipWitness,
    ROUTE_KEEPER_AUTHORITY_SCHEMA_V1, ROUTE_KEEPER_AUTHORITY_SCHEMA_V2,
    ROUTE_KEEPER_AUTHORITY_SCHEMA_V3, ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
};
pub use service_authentication_run::{
    authentication_run_map_is_empty, cancel_authentication_run,
    complete_challenge_authentication_action, complete_credential_delivery_action,
    complete_service_authentication_run_start, complete_site_authentication_action,
    prepare_service_authentication_run_start, project_service_authentication_run,
    require_live_authentication_run, reserve_authentication_effect, PendingAuthenticationEffect,
    PreparedServiceAuthenticationRunStart, ServiceAuthenticationRunCompletion,
    ServiceAuthenticationRunError, ServiceAuthenticationRunProjection,
    ServiceAuthenticationRunRecord, ServiceAuthenticationRunStartDecision,
    ServiceAuthenticationRunStartInput, ServiceAuthenticationRunStateError,
    SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION,
};
pub use service_challenge_task::{
    admit_challenge_consumer_from_receipt, cancel_service_challenge_task,
    challenge_task_map_is_empty, challenge_task_summary, complete_service_challenge_task_resume,
    complete_service_challenge_task_start, prepare_service_challenge_task_resume,
    prepare_service_challenge_task_start, project_service_challenge_task,
    service_challenge_task_status, PreparedServiceChallengeTaskResume,
    PreparedServiceChallengeTaskStart, ServiceChallengeTaskCancelDecision,
    ServiceChallengeTaskCancelInput, ServiceChallengeTaskError, ServiceChallengeTaskProjection,
    ServiceChallengeTaskRecord, ServiceChallengeTaskResumeDecision,
    ServiceChallengeTaskResumeInput, ServiceChallengeTaskStartDecision,
    ServiceChallengeTaskStartInput, ServiceChallengeTaskState, ServiceChallengeTaskSummary,
    AUTHENTICATION_CHALLENGE_INTENT_ID, NAVIGATION_CHALLENGE_INTENT_ID,
    SERVICE_CHALLENGE_TASK_SCHEMA_VERSION,
};
pub use service_state::{
    builtin_site_policies, builtin_site_policy, decode_persisted_service_state_json,
    default_profile_seeding_url, encode_prepared_service_state_pretty,
    prepare_service_state_for_persistence, service_profile_sources, service_site_policy_sources,
    validate_service_state_invariants, ColdShutdownStateReceipt, ConfiguredServiceStateInput,
    ProfileReceiptReplayError, ProfileRecoveryReceiptIdentity, ProfileResetReceiptIdentity,
    RuntimeOwnerPersistenceParts, RuntimeOwnerPersistenceRestore, RuntimeOwnerPersistenceSnapshot,
    ServiceState, ServiceStateCodecError, LEGACY_SERVICE_STATE_SCHEMA_VERSION,
    SERVICE_STATE_SCHEMA_VERSION,
};
pub use session_tab::{
    BrowserSession, BrowserTab, LeaseState, ProfileLeaseDisposition, ProfileSelectionReason,
    ServiceActor, ServiceTabHandle, ServiceTabHandleTraceFilter, SessionCleanupPolicy,
    TabLifecycle, SERVICE_LEASE_STATE_VALUES, SERVICE_PROFILE_LEASE_DISPOSITION_VALUES,
    SERVICE_PROFILE_SELECTION_REASON_VALUES, SERVICE_SESSION_CLEANUP_VALUES,
    SERVICE_TAB_LIFECYCLE_VALUES,
};
pub use site_policy::{
    challenge_required_capabilities, interaction_decision, provider_allowed_for_challenge,
    provider_capability_wire_name, provider_decision, Challenge, ChallengeKind, ChallengePolicy,
    ChallengeState, InteractionDecision, InteractionMode, ProviderCapability, ProviderDecision,
    ProviderKind, RateLimitPolicy, ServiceProvider, SitePolicy, SERVICE_CHALLENGE_KIND_VALUES,
    SERVICE_CHALLENGE_POLICY_VALUES, SERVICE_CHALLENGE_STATE_VALUES,
    SERVICE_INTERACTION_MODE_VALUES, SERVICE_PROVIDER_CAPABILITY_VALUES,
    SERVICE_PROVIDER_KIND_VALUES,
};
pub use terminal_outcome::{
    ServiceTerminalOutcome, ServiceTerminalPhase, ServiceTerminalState,
    SERVICE_TERMINAL_OUTCOME_SCHEMA_VERSION,
};

pub use presentation_request_queue::{
    PresentationRequestEntry, PresentationRequestPriority, PresentationRequestQueue,
    PresentationRequestState,
};
pub use principal_provenance::ServicePrincipalProvenance;
