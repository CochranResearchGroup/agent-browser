//! Provider-free durable record models for Agent Browser.
//!
//! This module owns record compatibility and deterministic lifecycle decisions.
//! Adapters own persistence, browser observation, process control, and transport.

mod browser_process;
mod browser_profile;
mod entity_source;
mod presentation;
mod profile_access;
mod profile_readiness;
mod profile_seeding;
mod session_tab;
mod site_policy;

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
pub use entity_source::{ServiceEntitySource, ServiceEntitySources};
pub use presentation::{
    ControlInputProvider, DisplayAllocation, DurableHandoffPresentationReceipt,
    RemoteViewAcquisitionLease, RemoteViewHandoff, RemoteViewRoute,
    RetainedDisplayAllocationCandidate, RoutePoolEntry, ViewStream, ViewStreamProvider,
    ViewerLease, SERVICE_CONTROL_INPUT_PROVIDER_VALUES, SERVICE_VIEW_STREAM_PROVIDER_VALUES,
};
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
pub use profile_readiness::{
    BrowserBuild, BrowserProfileCompatibilityEvidence, ProfileAllocationPolicy,
    ProfileKeyringPolicy, ProfileReadinessState, ProfileTargetReadiness,
    SERVICE_BROWSER_BUILD_VALUES, SERVICE_PROFILE_ALLOCATION_VALUES,
    SERVICE_PROFILE_KEYRING_VALUES, SERVICE_PROFILE_READINESS_VALUES,
};
pub use profile_seeding::{
    profile_seeding_handoff_id, ProfileSeedingHandoffRecord, ProfileSeedingHandoffState,
    ProfileSeedingMode, SERVICE_PROFILE_SEEDING_HANDOFF_STATE_VALUES,
    SERVICE_PROFILE_SEEDING_MODE_VALUES,
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
