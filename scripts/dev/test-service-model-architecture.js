#!/usr/bin/env node

import { deepEqual, doesNotThrow, ok, throws } from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { check } from './check-service-model-architecture.js';

const retirementRecords = [
  'AbandonedBrowserRetirementPlan', 'AbandonedBrowserRetirementTransaction',
  'AbandonedBrowserRetirementReceipt', 'RetirementTerminalProjection',
  'RetirementExitEvidence', 'RetirementExitFailure', 'RetirementRecourse',
  'ResourceRetirementPolicy',
];
const validRetirement = retirementRecords.map((name) => `pub struct ${name};`).join('\n')
  + '\npub const ABANDONED_BROWSER_RETIREMENT_PLAN_SCHEMA_V1: &str = "schema";\n';
const crashRegenerationRecords = [
  'CrashRegenerationPhase', 'CrashRegenerationState',
  'CrashRegenerationStableIdentities', 'CrashRegenerationEvidence',
  'CrashRegenerationTransaction', 'CrashRegenerationStatus',
  'CrashRegenerationRequest', 'CrashRegenerationOperation',
  'CrashRegenerationPhaseReceipt',
];
const validCrashRegeneration = crashRegenerationRecords
  .map((name) => `pub struct ${name};`).join('\n')
  + '\npub const CRASH_REGENERATION_STATUS_SCHEMA_VERSION: &str = "schema";\n';
const validCrashExport = `mod crash_regeneration;
pub use crash_regeneration::{
  ${crashRegenerationRecords.join(',\n  ')},
  CRASH_REGENERATION_STATUS_SCHEMA_VERSION,
};
`;
const serviceStateMigrationFields = [
  'schema_version', 'state_revision', 'profile_lease_schema_version',
  'presentation_capacity', 'profile_policy_migration', 'service_principals',
  'lease_authority', 'profile_lease_reconcile_receipts', 'profile_recovery_receipts',
  'profile_reset_receipts', 'profile_lifecycle_authorizations',
  'profile_lifecycle_effect_receipts', 'browser_retirement_receipts',
  'abandoned_browser_retirements', 'crash_regeneration_transactions',
  'protected_browser_owner_observations', 'runtime_owner_registry',
  'authentication_runs', 'challenge_tasks', 'unknown_fields',
];
const serviceStateFieldTypes = {
  presentation_capacity: 'Option<agent_browser_service_model::PresentationCapacityAuthority>',
  profile_policy_migration: 'Option<agent_browser_service_model::ProfilePolicyMigrationReport>',
  service_principals: 'agent_browser_lease_authority::ServicePrincipalRegistry',
  profile_lease_reconcile_receipts: 'BTreeMap<String, agent_browser_service_model::ProfileLeaseReconcileReceipt>',
  profile_recovery_receipts: 'BTreeMap<String, agent_browser_service_model::RecoveryReceipt>',
  profile_reset_receipts: 'BTreeMap<String, agent_browser_service_model::ProfileResetReceipt>',
  profile_lifecycle_authorizations: 'BTreeMap<String, agent_browser_service_model::ProfileLifecycleAuthorization>',
  profile_lifecycle_effect_receipts: 'BTreeMap<String, agent_browser_service_model::ProfileLifecycleEffectReceipt>',
  browser_retirement_receipts: 'BTreeMap<String, agent_browser_service_model::BrowserRetirementReceipt>',
  crash_regeneration_transactions: 'BTreeMap<String, agent_browser_service_model::CrashRegenerationTransaction>',
  browser_capability_registry: 'agent_browser_service_model::BrowserCapabilityRegistry',
  authentication_runs: 'BTreeMap<String, agent_browser_service_model::ServiceAuthenticationRunRecord>',
  challenge_tasks: 'BTreeMap<String, agent_browser_service_model::ServiceChallengeTaskRecord>',
};
const serviceStateFieldAttributes = {
  service_principals: '#[serde(skip_serializing_if = "agent_browser_lease_authority::ServicePrincipalRegistry::is_empty")]\n',
  authentication_runs: '#[serde(skip_serializing_if = "agent_browser_service_model::authentication_run_map_is_empty")]\n',
  challenge_tasks: '#[serde(skip_serializing_if = "agent_browser_service_model::challenge_task_map_is_empty")]\n',
};
const validServiceState = `
pub struct ConfiguredServiceStateInput;
pub struct ProfileRecoveryReceiptIdentity;
pub struct ProfileResetReceiptIdentity;
pub enum ProfileReceiptReplayError {}
pub struct RuntimeOwnerPersistenceSnapshot;
pub struct RuntimeOwnerPersistenceRestore;
pub struct RuntimeOwnerPersistenceParts;
pub struct ServiceState {
${serviceStateMigrationFields.map((field) => `  ${serviceStateFieldAttributes[field] || ''}#[doc(hidden)]\n  pub ${field}: ${serviceStateFieldTypes[field] || 'String'},`).join('\n')}
  #[serde(skip_serializing_if = "agent_browser_lease_authority::ServicePrincipalRegistry::is_empty")]
  pub browser_capability_registry: agent_browser_service_model::BrowserCapabilityRegistry,
}
impl ServiceState {
  pub fn from_configured_entities() -> Self { Self {} }
  pub fn state_revision(&self) -> u64 { 0 }
  pub fn crash_regeneration_transaction(&self) {}
  pub fn crash_regeneration_statuses(&self) {}
  pub fn begin_or_resume_crash_regeneration(&mut self) {}
  pub fn apply_crash_regeneration_phase(&mut self) {}
  pub fn interrupt_crash_regeneration(&mut self) {}
  pub fn finish_crash_regeneration(&mut self) {}
  pub fn without_crash_regeneration_transactions(&self) {}
  pub fn current_lease_claim(&self) {}
  pub fn replay_lease_claim_release(&self) {}
  pub fn replay_lease_claim_recovery(&self) {}
  pub fn replay_lease_claim_revocation(&self) {}
  pub fn release_lease_claim(&mut self) {}
  pub fn recover_lease_claim(&mut self) {}
  pub fn revoke_lease_claim(&mut self) {}
  pub fn replay_profile_lease_reconciliation(&self) {}
  pub fn record_profile_lease_reconciliation(&mut self) {}
  pub fn profile_recovery_receipt(&self) {}
  pub fn replay_profile_recovery(&self) {}
  pub fn record_profile_recovery(&mut self) {}
  pub fn replay_profile_reset(&self) {}
  pub fn record_profile_reset(&mut self) {}
  pub fn service_principal_registry_revision(&self) {}
  pub fn service_principal(&self) {}
  pub fn profile_capability(&self) {}
  pub fn profile_capabilities(&self) {}
  pub fn authenticate_profile_capability(&self) {}
  pub fn authenticated_authority_is_current(&self) {}
  pub fn register_profile_capability(&mut self) {}
  pub fn rotate_profile_capability(&mut self) {}
  pub fn lease_authority_view(&self) {}
  pub fn authenticated_session_work_authority(&self) {}
  pub fn principal_continuity_decision(&self) {}
  pub fn plan_legacy_session_principal_migration(&self) {}
  pub fn bind_session_work_lease(&mut self) {}
  pub fn bind_tab_work_lease(&mut self) {}
  pub fn restore_runtime_owner_persistence(&mut self) {}
  pub fn runtime_owner_persistence_parts(&self) {}
  pub fn strip_runtime_lifecycle_for_persistence(&mut self) {}
  pub fn runtime_lifecycle_authority_summary(&self) {}
  pub fn runtime_lifecycle_boot_epoch_observations(&self) {}
  pub fn profile_runtime_authority(&self) {}
  pub fn runtime_control_plane_authority(&self) {}
  pub fn runtime_lane_authority(&self) {}
  pub fn runtime_resource_lanes(&self) {}
  pub fn apply_runtime_lifecycle_transition_atomically(
    &mut self,
    intent: agent_browser_lease_authority::RuntimeLifecycleIntent,
  ) -> Result<agent_browser_lease_authority::RuntimeLifecycleTransition, String> {
    let mut staged = self.runtime_owner_registry.clone();
    let transition = staged.apply_lifecycle_transition(intent)?;
    self.runtime_owner_registry = staged;
    Ok(transition)
  }
  pub fn apply_runtime_lifecycle_transition_with_profile_sync_atomically(
    &mut self,
    profile_id: &str,
    user_data_dir: String,
    intent: agent_browser_lease_authority::RuntimeLifecycleIntent,
  ) -> Result<agent_browser_lease_authority::RuntimeLifecycleTransition, String> {
    let profile = self
      .profiles
      .get(profile_id)
      .ok_or_else(|| "runtime_lifecycle_profile_record_missing".to_string())?;
    if profile.id != profile_id || profile_id.trim().is_empty() {
      return Err("runtime_lifecycle_profile_record_sync_rejected".to_string());
    }
    let mut staged = self.runtime_owner_registry.clone();
    let transition = staged.apply_lifecycle_transition(intent)?;
    self.profiles
      .get_mut(profile_id)
      .expect("validated profile remains present")
      .user_data_dir = Some(user_data_dir);
    self.runtime_owner_registry = staged;
    Ok(transition)
  }
  pub fn revoke_process_exited_session_owner(
    &mut self,
    session_id: &str,
  ) -> Option<agent_browser_lease_authority::ProfileOwner> {
    let binding = self
      .runtime_owner_registry
      .binding_for_session(session_id)
      .ok()
      .flatten()?;
    if !binding.effect_capable {
      return None;
    }
    let transition = self
      .runtime_owner_registry
      .apply_lifecycle_transition(
        agent_browser_lease_authority::RuntimeLifecycleIntent::RevokeLegacyOwner {
          profile_identity_digest: binding.claim.profile_identity_digest,
          logical_browser_id: binding.claim.logical_browser_id,
          expected_daemon_session_route: binding.claim.daemon_session_route,
          expected_owner_id: binding.claim.owner_id,
          expected_owner_generation: binding.claim.owner_generation,
        },
      )
      .ok()?;
    let agent_browser_lease_authority::RuntimeLifecycleTransition::LegacyOwnerRevoked(owner) =
      transition
    else {
      return None;
    };
    Some(owner)
  }
  pub fn service_authentication_run(&self) {}
  pub fn prepare_service_authentication_run_start(&self) {}
  pub fn complete_service_authentication_run_start(&mut self) {}
  pub fn reserve_service_authentication_effect(&mut self) {}
  pub fn observe_service_authentication_run(&mut self) {}
  pub fn prepare_service_authentication_watch(&mut self) {}
  pub fn verify_service_authentication_run(&mut self) {}
  pub fn complete_service_authentication_site_action(&mut self) {}
  pub fn complete_service_authentication_delivery_action(&mut self) {}
  pub fn complete_service_authentication_challenge_action(&mut self) {}
  pub fn cancel_service_authentication_run(&mut self) {}
  pub fn service_challenge_task(&self) {}
  pub fn service_challenge_task_summary(&self) {}
  pub fn start_service_challenge_task(&mut self) {}
  pub fn status_service_challenge_task(&self) {}
  pub fn resume_service_challenge_task(&mut self) {}
  pub fn cancel_service_challenge_task(&mut self) {}
}
pub const SERVICE_STATE_SCHEMA_VERSION: &str = "v2";
pub const LEGACY_SERVICE_STATE_SCHEMA_VERSION: &str = "legacy";
pub enum ServiceStateCodecError {}
pub fn builtin_site_policies() {}
pub fn builtin_site_policy() {}
pub fn default_profile_seeding_url() {}
pub fn decode_persisted_service_state_json() {}
pub fn encode_prepared_service_state_pretty() {}
pub fn prepare_service_state_for_persistence() {}
pub fn service_profile_sources() {}
pub fn service_site_policy_sources() {}
pub fn validate_service_state_invariants() {}
`;
const validServiceStateExport = `mod service_state;
pub use service_state::{
  builtin_site_policies, builtin_site_policy, decode_persisted_service_state_json,
  default_profile_seeding_url,
  encode_prepared_service_state_pretty, prepare_service_state_for_persistence,
  service_profile_sources, service_site_policy_sources,
  validate_service_state_invariants, ConfiguredServiceStateInput, ServiceState,
  ProfileReceiptReplayError, ProfileRecoveryReceiptIdentity,
  ProfileResetReceiptIdentity, RuntimeOwnerPersistenceParts,
  RuntimeOwnerPersistenceRestore, RuntimeOwnerPersistenceSnapshot,
  ServiceStateCodecError,
  LEGACY_SERVICE_STATE_SCHEMA_VERSION, SERVICE_STATE_SCHEMA_VERSION,
};
`;
const validCrashStateUse = `
pub struct ServiceState {
  presentation_capacity:
    Option<agent_browser_service_model::PresentationCapacityAuthority>,
  profile_policy_migration:
    Option<agent_browser_service_model::ProfilePolicyMigrationReport>,
  #[serde(skip_serializing_if = "agent_browser_lease_authority::ServicePrincipalRegistry::is_empty")]
  service_principals:
    agent_browser_lease_authority::ServicePrincipalRegistry,
  profile_lease_reconcile_receipts:
    BTreeMap<String, agent_browser_service_model::ProfileLeaseReconcileReceipt>,
  profile_recovery_receipts:
    BTreeMap<String, agent_browser_service_model::RecoveryReceipt>,
  profile_reset_receipts:
    BTreeMap<String, agent_browser_service_model::ProfileResetReceipt>,
  profile_lifecycle_authorizations:
    BTreeMap<String, agent_browser_service_model::ProfileLifecycleAuthorization>,
  profile_lifecycle_effect_receipts:
    BTreeMap<String, agent_browser_service_model::ProfileLifecycleEffectReceipt>,
  browser_retirement_receipts:
    BTreeMap<String, agent_browser_service_model::BrowserRetirementReceipt>,
  crash_regeneration_transactions:
    BTreeMap<String, agent_browser_service_model::CrashRegenerationTransaction>,
  browser_capability_registry:
    agent_browser_service_model::BrowserCapabilityRegistry,
  #[serde(skip_serializing_if = "agent_browser_service_model::authentication_run_map_is_empty")]
  authentication_runs:
    BTreeMap<String, agent_browser_service_model::ServiceAuthenticationRunRecord>,
  #[serde(skip_serializing_if = "agent_browser_service_model::challenge_task_map_is_empty")]
  challenge_tasks:
    BTreeMap<String, agent_browser_service_model::ServiceChallengeTaskRecord>,
}
`;
const validCapabilityRegistry = `
pub struct BrowserCapabilityRegistry;
pub fn browser_profile_compatibility_matches() {}
`;
const validCapabilityExport = `mod browser_capability_registry;
pub use browser_capability_registry::{
  browser_profile_compatibility_matches,
  BrowserCapabilityRegistry,
};
`;
const validPrincipalContinuity = `
pub enum PrincipalContinuityRecourse {}
pub struct PrincipalContinuityDecision;
pub enum LegacyPrincipalMigrationDisposition {}
pub struct LegacySessionPrincipalMigrationPlan;
`;
const validPrincipalContinuityExport = `mod principal_continuity;
pub use principal_continuity::{
  PrincipalContinuityRecourse,
  PrincipalContinuityDecision,
  LegacyPrincipalMigrationDisposition,
  LegacySessionPrincipalMigrationPlan,
};
`;
const runtimeOwnerProjectionDefinitions = [
  'RuntimeLifecycleAuthoritySummary', 'RuntimeLifecycleBootEpochObservation',
  'ProfileRuntimeAuthority', 'RuntimeControlPlaneAuthority',
  'RuntimeLaneAuthority', 'RuntimeResourceLane',
];
const validRuntimeOwnerProjection = runtimeOwnerProjectionDefinitions
  .map((name) => `pub struct ${name};`).join('\n');
const validRuntimeOwnerProjectionExport = `mod runtime_owner_projection;
pub use runtime_owner_projection::{
  ${runtimeOwnerProjectionDefinitions.join(',\n  ')},
};
`;
const serviceAuthenticationDefinitions = [
  'ServiceAuthenticationRunRecord', 'PendingAuthenticationEffect',
  'ServiceAuthenticationRunStartInput', 'PreparedServiceAuthenticationRunStart',
  'ServiceAuthenticationRunStartDecision', 'ServiceAuthenticationRunCompletion',
  'ServiceAuthenticationRunError', 'ServiceAuthenticationRunProjection',
];
const serviceAuthenticationDecisions = [
  'prepare_service_authentication_run_start',
  'complete_service_authentication_run_start', 'project_service_authentication_run',
  'require_live_authentication_run', 'reserve_authentication_effect',
  'complete_site_authentication_action', 'complete_credential_delivery_action',
  'complete_challenge_authentication_action', 'cancel_authentication_run',
  'authentication_run_map_is_empty',
];
const validServiceAuthentication = `${serviceAuthenticationDefinitions
  .map((name) => `pub struct ${name};`).join('\n')}
${serviceAuthenticationDecisions.map((name) => `pub fn ${name}() {}`).join('\n')}
pub const SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION: &str = "schema";
`;
const validServiceAuthenticationExport = `mod service_authentication_run;
pub use service_authentication_run::{
  ${serviceAuthenticationDefinitions.join(',\n  ')},
  ${serviceAuthenticationDecisions.join(',\n  ')},
  SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION,
};
`;
const serviceChallengeDefinitions = [
  'ServiceChallengeTaskState', 'ServiceChallengeTaskSummary',
  'PendingChallengeTaskEffect', 'ChallengeTaskEffectKind',
  'ServiceChallengeTaskRecord', 'ServiceChallengeTaskStartInput',
  'PreparedServiceChallengeTaskStart', 'ServiceChallengeTaskStartDecision',
  'ServiceChallengeTaskResumeInput', 'PreparedServiceChallengeTaskResume',
  'ServiceChallengeTaskResumeDecision', 'ServiceChallengeTaskCancelInput',
  'ServiceChallengeTaskCancelDecision', 'ServiceChallengeTaskError',
  'ServiceChallengeTaskProjection',
];
const serviceChallengeExportedDefinitions = serviceChallengeDefinitions.filter(
  (name) => !['PendingChallengeTaskEffect', 'ChallengeTaskEffectKind'].includes(name),
);
const serviceChallengeDecisions = [
  'prepare_service_challenge_task_start', 'complete_service_challenge_task_start',
  'service_challenge_task_status', 'prepare_service_challenge_task_resume',
  'complete_service_challenge_task_resume', 'cancel_service_challenge_task',
  'project_service_challenge_task', 'challenge_task_map_is_empty',
  'challenge_task_summary', 'admit_challenge_consumer_from_receipt',
];
const serviceChallengeConstants = [
  'SERVICE_CHALLENGE_TASK_SCHEMA_VERSION', 'AUTHENTICATION_CHALLENGE_INTENT_ID',
  'NAVIGATION_CHALLENGE_INTENT_ID',
];
const validServiceChallenge = `${serviceChallengeDefinitions
  .map((name) => `${['PendingChallengeTaskEffect', 'ChallengeTaskEffectKind'].includes(name)
    ? '' : 'pub '}struct ${name};`).join('\n')}
${serviceChallengeDecisions.map((name) => `pub fn ${name}() {}`).join('\n')}
${serviceChallengeConstants.map((name) => `pub const ${name}: &str = "value";`).join('\n')}
`;
const validServiceChallengeExport = `mod service_challenge_task;
pub use service_challenge_task::{
  ${serviceChallengeExportedDefinitions.join(',\n  ')},
  ${serviceChallengeDecisions.join(',\n  ')},
  ${serviceChallengeConstants.join(',\n  ')},
};
`;

const authenticationControlRecords = [
  'AuthenticationRunBinding', 'AuthenticationRunState',
  'AuthenticationChallengeChannel', 'AuthenticationActionKind', 'SiteLoginState',
  'PasswordPersistencePolicy', 'SiteLoginObservationReceipt',
  'SiteLoginActionReceipt', 'SiteLoginActionContext', 'ProviderWatchReceipt',
  'SameProfileNewTabProof', 'AuthenticationActionReceipt',
  'AuthenticationActionContext', 'AuthenticationActionFailure',
  'AuthenticationVerifierReceipt', 'AuthenticationVerificationContext',
  'AuthenticationVerifierFailure', 'AuthenticationTransitionReceipt',
  'ActiveAuthenticationChallenge', 'AuthenticationRun', 'AuthenticationRunError',
];
const authenticationControlTraits = [
  'ResponseOnlySiteLoginAction', 'ResponseOnlyAuthenticationAction',
  'AuthenticationVerifier',
];
const validAuthenticationControl = `${authenticationControlRecords
  .map((name) => `pub struct ${name};`).join('\n')}
${authenticationControlTraits.map((name) => `pub trait ${name} {}`).join('\n')}
pub const AUTHENTICATION_RUN_SCHEMA_VERSION: &str = "schema";
`;
const validAuthenticationFacade = 'pub(crate) use agent_browser_authentication_control::*;\n';
const validRuntimeLifecycle = `
impl Authority {
  pub(crate) fn transition(&self) {
    state.apply_runtime_lifecycle_transition_atomically(prepare_lifecycle_intent(intent));
  }
  fn transition_terminal_replacement_with_profile_sync(
    &self,
    intent: RuntimeLifecycleIntent,
    profile_id: &str,
    profile_root: &std::path::Path,
  ) -> Result<RuntimeLifecycleTransition, String> {
    let user_data_dir = profile_root.to_str().ok_or_else(error)?.to_string();
    self.repository.mutate(|state| {
      let profile = state.profiles.get(profile_id).ok_or_else(error)?;
      if profile.id != profile_id
        || !canonical_route_viewer_runtime_profile(profile_id)
        || profile_id.trim().is_empty()
      {
        return Err(error);
      }
      let prepared_intent = prepare_lifecycle_intent(intent.clone());
      state.apply_runtime_lifecycle_transition_with_profile_sync_atomically(
        profile_id,
        user_data_dir.clone(),
        prepared_intent,
      )
    })
  }
}
`;
const validControlPlane = `
fn persist_process_exited_browser_health_in_repository() {
  record_browser_health_changed_event(service_state, id, previous, browser);
  let revoked_owner_and_session = authenticated_session_work_authority()
    .and_then(|_| {
      let revoked_owner = service_state.revoke_process_exited_session_owner(&state.session_id)?;
      let session = service_state.sessions.get(&state.session_id).cloned()?;
      Some((revoked_owner, session))
    });
  remove_browser_operational_record(service_state, id, session_id);
}
`;
const validRuntimeReconciliation = `
fn classify(state: &ServiceState) {
  let authority = state.runtime_lane_authority(profile_identity_digest, logical_browser_id);
  let owner = authority.owner.ok_or(owner_error)?;
  let lifecycle = authority.lifecycle.ok_or(lifecycle_error)?;
}
`;
const validLeaseAuthorityAdapter = `
fn authorize(state: &ServiceState) {
  let authority = state.profile_runtime_authority(profile_identity_digest);
  match (claim.owner_generation(), authority.owner) {
    (None, None) => true,
    (Some(expected), Some(owner)) => authority.principal_binding.is_some(),
    _ => false,
  };
}
`;

function fixture({ manifest = '', source = '', workspace = true, retirement = validRetirement,
  crash = validCrashRegeneration, capability = validCapabilityRegistry, cli = '',
  cliTest = '', principalCli = '', serviceStore = '',
  projectionCli = '', runtimeOwnerProjection = validRuntimeOwnerProjection,
  runtimeLifecycle = validRuntimeLifecycle,
  controlPlane = validControlPlane,
  runtimeReconciliation = validRuntimeReconciliation,
  leaseAuthorityAdapter = validLeaseAuthorityAdapter,
  principalContinuity = validPrincipalContinuity,
  serviceModel = 'pub use agent_browser_service_model::{ServiceState};\n',
  serviceState = validServiceState, authenticationManifest = validAuthenticationManifest,
  authentication = validAuthenticationControl,
  authenticationFacade = validAuthenticationFacade,
  serviceAuthentication = validServiceAuthentication,
  serviceChallenge = validServiceChallenge } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'agent-browser-service-model-architecture-'));
  mkdirSync(join(root, 'crates/agent-browser-service-model/src'), { recursive: true });
  mkdirSync(join(root, 'crates/agent-browser-authentication-control/src'), { recursive: true });
  writeFileSync(join(root, 'Cargo.toml'), workspace
    ? '[workspace]\nmembers = ["crates/agent-browser-service-model", "crates/agent-browser-authentication-control"]\n'
    : 'workspace = false\n');
  writeFileSync(join(root, 'crates/agent-browser-service-model/Cargo.toml'), manifest);
  writeFileSync(join(root, 'crates/agent-browser-authentication-control/Cargo.toml'),
    authenticationManifest);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/lib.rs'), source);
  writeFileSync(join(root, 'crates/agent-browser-authentication-control/src/lib.rs'),
    authentication);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/abandoned_browser_retirement.rs'), retirement);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/crash_regeneration.rs'), crash);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/browser_capability_registry.rs'), capability);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/principal_continuity.rs'),
    principalContinuity);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/runtime_owner_projection.rs'),
    runtimeOwnerProjection);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/service_authentication_run.rs'),
    serviceAuthentication);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/service_challenge_task.rs'),
    serviceChallenge);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/service_state.rs'), serviceState);
  mkdirSync(join(root, 'cli/src/native'), { recursive: true });
  writeFileSync(join(root, 'cli/src/native/retirement.rs'), cli);
  writeFileSync(join(root, 'cli/src/native/service_principal.rs'), principalCli);
  writeFileSync(join(root, 'cli/src/native/service_store.rs'), serviceStore);
  writeFileSync(join(root, 'cli/src/install.rs'), projectionCli);
  writeFileSync(join(root, 'cli/src/native/runtime_lifecycle.rs'), runtimeLifecycle);
  writeFileSync(join(root, 'cli/src/native/control_plane.rs'), controlPlane);
  writeFileSync(join(root, 'cli/src/native/runtime_reconciliation.rs'), runtimeReconciliation);
  writeFileSync(join(root, 'cli/src/native/service_lease_authority_adapter.rs'),
    leaseAuthorityAdapter);
  writeFileSync(join(root, 'cli/src/native/service_model_tests.rs'), cliTest);
  writeFileSync(join(root, 'cli/src/native/authentication_run.rs'), authenticationFacade);
  writeFileSync(join(root, 'cli/src/native/service_model.rs'), serviceModel);
  return root;
}

const validManifest = '[package]\nname = "agent-browser-service-model"\nversion = "0.1.0"\n[dependencies]\nserde = "1"\n';
const validAuthenticationManifest = '[package]\nname = "agent-browser-authentication-control"\nversion = "0.1.0"\n[dependencies]\nserde = "1"\n';
const validSource = `${validCrashExport}
${validCapabilityExport}
${validPrincipalContinuityExport}
${validRuntimeOwnerProjectionExport}
${validServiceAuthenticationExport}
${validServiceChallengeExport}
${validServiceStateExport}
// std::fs and provider are allowed in prose.
pub struct BrowserProfile { pub id: String }
pub fn profile(id: String) -> BrowserProfile { BrowserProfile { id } }
`;

for (const [name, mutation] of [
  ['missing aggregate module', { serviceState: '' }],
  ['duplicate CLI aggregate', { serviceModel: 'pub use agent_browser_service_model::ServiceState;\npub struct ServiceState {}\n' }],
  ['foreign CLI aggregate impl', { serviceModel: 'pub use agent_browser_service_model::ServiceState;\nimpl ServiceState {}\n' }],
  ['second model aggregate impl', { serviceAuthentication: `${validServiceAuthentication}\nimpl ServiceState {}\n` }],
  ['missing CLI aggregate reexport', { serviceModel: '' }],
  ['extra hidden migration field', { serviceState: validServiceState.replace(
    'pub browser_capability_registry:',
    '#[doc(hidden)]\n  pub extra_migration_field: String,\n  pub browser_capability_registry:',
  ) }],
  ['missing persisted codec', { serviceState: validServiceState.replace(
    'pub fn decode_persisted_service_state_json() {}',
    '',
  ) }],
  ['missing revision accessor', { serviceState: validServiceState.replace(
    'pub fn state_revision(&self) -> u64 { 0 }',
    '',
  ) }],
  ['missing configured constructor', { serviceState: validServiceState.replace(
    'pub fn from_configured_entities() -> Self { Self {} }',
    '',
  ) }],
  ['missing crash aggregate method', { serviceState: validServiceState.replace(
    'pub fn interrupt_crash_regeneration(&mut self) {}',
    '',
  ) }],
  ['direct CLI crash map access', { cli: 'fn leak(state: &ServiceState) { let _ = &state.crash_regeneration_transactions; }\n' }],
  ['missing lease authority method', { serviceState: validServiceState.replace(
    'pub fn recover_lease_claim(&mut self) {}',
    '',
  ) }],
  ['direct CLI lease authority access', { cli: 'fn leak(state: &ServiceState) { let _ = &state.lease_authority; }\n' }],
  ['missing receipt interface type', { serviceState: validServiceState.replace(
    'pub struct ProfileResetReceiptIdentity;',
    '',
  ) }],
  ['missing receipt aggregate method', { serviceState: validServiceState.replace(
    'pub fn replay_profile_recovery(&self) {}',
    '',
  ) }],
  ['direct CLI lease receipt access', { cli: 'fn leak(state: &ServiceState) { let _ = &state.profile_lease_reconcile_receipts; }\n' }],
  ['direct CLI recovery receipt access', { cli: 'fn leak(state: &ServiceState) { let _ = &state.profile_recovery_receipts; }\n' }],
  ['direct CLI reset receipt access', { cli: 'fn leak(state: &ServiceState) { let _ = &state.profile_reset_receipts; }\n' }],
  ['missing principal authority method', { serviceState: validServiceState.replace(
    'pub fn authenticated_authority_is_current(&self) {}',
    '',
  ) }],
  ['direct CLI principal registry access', { cli: 'fn leak(state: &ServiceState) { let _ = &state.service_principals; }\n' }],
  ['direct CLI principal registry access after test-only item', {
    cli: '#[cfg(test)]\nuse crate::fixture;\nfn leak(state: &ServiceState) { let _ = &state.service_principals; }\n',
  }],
  ['missing principal continuity definition', {
    principalContinuity: validPrincipalContinuity.replace(
      'pub struct PrincipalContinuityDecision;',
      '',
    ),
  }],
  ['duplicate CLI principal continuity definition', {
    principalCli: 'pub struct PrincipalContinuityDecision;\n',
  }],
  ['missing principal continuity aggregate method', { serviceState: validServiceState.replace(
    'pub fn principal_continuity_decision(&self) {}',
    '',
  ) }],
  ['copied CLI principal continuity state decision', {
    principalCli: 'fn copied(state: &ServiceState) { let _ = &state.runtime_owner_registry; }\n',
  }],
  ['missing runtime-owner persistence type', { serviceState: validServiceState.replace(
    'pub struct RuntimeOwnerPersistenceSnapshot;',
    '',
  ) }],
  ['missing runtime-owner persistence method', { serviceState: validServiceState.replace(
    'pub fn restore_runtime_owner_persistence(&mut self) {}',
    '',
  ) }],
  ['direct repository runtime-owner access', {
    serviceStore: 'fn leak(state: &ServiceState) { let _ = &state.runtime_owner_registry; }\n',
  }],
  ['runtime-owner persistence call outside repository adapter', {
    cli: 'fn leak(state: &mut ServiceState) { state.strip_runtime_lifecycle_for_persistence(); }\n',
  }],
  ['copied repository lifecycle strip', {
    serviceStore: 'fn leak(registry: &RuntimeOwnerRegistry) { let _ = registry.persistence_projection_without_lifecycle_records(); }\n',
  }],
  ['runtime-owner persistence getter escape hatch', {
    serviceState: `${validServiceState}\nimpl RuntimeOwnerPersistenceSnapshot { pub fn registry(&self) {} }\n`,
  }],
  ['runtime-owner persistence public field escape hatch', {
    serviceState: validServiceState.replace(
      'pub struct RuntimeOwnerPersistenceSnapshot;',
      'pub struct RuntimeOwnerPersistenceSnapshot { pub registry: RuntimeOwnerRegistry }',
    ),
  }],
  ['runtime-owner persistence getter after private helper', {
    serviceState: `${validServiceState}\nimpl RuntimeOwnerPersistenceSnapshot { fn helper(&self) {} pub fn registry(&self) {} }\n`,
  }],
  ['runtime-owner persistence deref escape hatch', {
    serviceState: `${validServiceState}\nimpl Deref for RuntimeOwnerPersistenceSnapshot { type Target = (); fn deref(&self) -> &() { &() } }\n`,
  }],
  ['missing runtime-owner projection definition', {
    runtimeOwnerProjection: validRuntimeOwnerProjection.replace(
      'pub struct RuntimeLaneAuthority;',
      '',
    ),
  }],
  ['missing runtime-owner projection method', { serviceState: validServiceState.replace(
    'pub fn runtime_lane_authority(&self) {}',
    '',
  ) }],
  ['direct runtime-owner projection caller access', {
    projectionCli: 'fn leak(state: &ServiceState) { let _ = &state.runtime_owner_registry; }\n',
  }],
  ['direct runtime-owner projection caller access after test-only item', {
    projectionCli: '#[cfg(test)]\nfn fixture() {}\nfn leak(state: &ServiceState) { let _ = &state.runtime_owner_registry; }\n',
  }],
  ['raw runtime-owner registry projection parameter', {
    projectionCli: 'fn leak(registry: &RuntimeOwnerRegistry) {}\n',
  }],
  ['runtime-owner projection registry return', { serviceState: validServiceState.replace(
    'pub fn runtime_resource_lanes(&self) {}',
    'pub fn runtime_resource_lanes(&self) -> &RuntimeOwnerRegistry {}',
  ) }],
  ['runtime-owner projection iterator return', { serviceState: validServiceState.replace(
    'pub fn runtime_resource_lanes(&self) {}',
    'pub fn runtime_resource_lanes(&self) -> impl Iterator<Item = ()> {}',
  ) }],
  ['runtime-owner projection mutable receiver', { serviceState: validServiceState.replace(
    'pub fn profile_runtime_authority(&self) {}',
    'pub fn profile_runtime_authority(&mut self) {}',
  ) }],
  ['runtime-owner projection persistence bypass', {
    runtimeOwnerProjection: `${validRuntimeOwnerProjection}\npub fn bypass(_: RuntimeOwnerPersistenceSnapshot) {}\n`,
  }],
  ['runtime-owner projection free registry helper', {
    runtimeOwnerProjection: `${validRuntimeOwnerProjection}\npub fn registry(_: &ServiceState) -> &RuntimeOwnerRegistry { todo!() }\n`,
  }],
  ['runtime-owner projection impl escape hatch', {
    runtimeOwnerProjection: `${validRuntimeOwnerProjection}\nimpl RuntimeResourceLane { pub fn registry(&self) -> &RuntimeOwnerRegistry { todo!() } }\n`,
  }],
  ['missing atomic runtime lifecycle method', { serviceState: validServiceState.replace(
    /  pub fn apply_runtime_lifecycle_transition_atomically\([\s\S]*?\n  \}\n/,
    '',
  ) }],
  ['atomic runtime lifecycle registry argument', { serviceState: validServiceState.replace(
    'intent: agent_browser_lease_authority::RuntimeLifecycleIntent,',
    'registry: &mut RuntimeOwnerRegistry,',
  ) }],
  ['atomic runtime lifecycle callback argument', { serviceState: validServiceState.replace(
    'intent: agent_browser_lease_authority::RuntimeLifecycleIntent,',
    'intent: agent_browser_lease_authority::RuntimeLifecycleIntent, callback: impl FnOnce(),',
  ) }],
  ['atomic runtime lifecycle persistence argument', { serviceState: validServiceState.replace(
    'intent: agent_browser_lease_authority::RuntimeLifecycleIntent,',
    'intent: agent_browser_lease_authority::RuntimeLifecycleIntent, snapshot: RuntimeOwnerPersistenceSnapshot,',
  ) }],
  ['atomic runtime lifecycle wrong receiver', { serviceState: validServiceState.replace(
    '    &mut self,\n    intent: agent_browser_lease_authority::RuntimeLifecycleIntent,',
    '    &self,\n    intent: agent_browser_lease_authority::RuntimeLifecycleIntent,',
  ) }],
  ['atomic runtime lifecycle registry result', { serviceState: validServiceState.replace(
    ') -> Result<agent_browser_lease_authority::RuntimeLifecycleTransition, String> {',
    ') -> Result<&mut RuntimeOwnerRegistry, String> {',
  ) }],
  ['atomic runtime lifecycle iterator result', { serviceState: validServiceState.replace(
    ') -> Result<agent_browser_lease_authority::RuntimeLifecycleTransition, String> {',
    ') -> impl Iterator<Item = RuntimeOwnerRegistry> {',
  ) }],
  ['atomic runtime lifecycle error mapping', { serviceState: validServiceState.replace(
    '    let mut staged = self.runtime_owner_registry.clone();\n',
    '    if false { return Err("changed_error".into()); }\n    let mut staged = self.runtime_owner_registry.clone();\n',
  ) }],
  ['atomic runtime lifecycle missing commit', { serviceState: validServiceState.replace(
    '    self.runtime_owner_registry = staged;\n',
    '',
  ) }],
  ['atomic runtime lifecycle misplaced structure', { serviceState: validServiceState.replace(
    '    let mut staged = self.runtime_owner_registry.clone();\n    let transition = staged.apply_lifecycle_transition(intent)?;\n    self.runtime_owner_registry = staged;\n    Ok(transition)\n',
    '    helper(self, intent)\n',
  ) }],
  ['missing atomic profile-sync lifecycle method', { serviceState: validServiceState.replace(
    /  pub fn apply_runtime_lifecycle_transition_with_profile_sync_atomically\([\s\S]*?\n  \}\n/,
    '',
  ) }],
  ['atomic profile-sync lifecycle callback argument', { serviceState: validServiceState.replace(
    '    user_data_dir: String,\n    intent: agent_browser_lease_authority::RuntimeLifecycleIntent,',
    '    user_data_dir: String,\n    intent: agent_browser_lease_authority::RuntimeLifecycleIntent,\n    callback: impl FnOnce(),',
  ) }],
  ['atomic profile-sync lifecycle error mapping', { serviceState: validServiceState.replace(
    '    let mut staged = self.runtime_owner_registry.clone();\n    let transition = staged.apply_lifecycle_transition(intent)?;\n    self.profiles\n',
    '    let mut staged = self.runtime_owner_registry.clone();\n    let transition = staged.apply_lifecycle_transition(intent).map_err(|error| error)?;\n    self.profiles\n',
  ) }],
  ['atomic profile-sync lifecycle result filtering', { serviceState: validServiceState.replace(
    '    self.runtime_owner_registry = staged;\n    Ok(transition)\n  }\n  pub fn revoke_process_exited_session_owner',
    '    self.runtime_owner_registry = staged;\n    match transition { _ => Err("filtered".into()) }\n  }\n  pub fn revoke_process_exited_session_owner',
  ) }],
  ['terminal profile-sync CLI direct registry access', { runtimeLifecycle: validRuntimeLifecycle.replace(
    '      let profile = state.profiles.get(profile_id).ok_or_else(error)?;',
    '      let registry = state.runtime_owner_registry.clone();\n      let profile = state.profiles.get(profile_id).ok_or_else(error)?;',
  ) }],
  ['terminal profile-sync CLI direct path mutation', { runtimeLifecycle: validRuntimeLifecycle.replace(
    '      state.apply_runtime_lifecycle_transition_with_profile_sync_atomically(',
    '      state.profiles.get_mut(profile_id).unwrap().user_data_dir = Some(user_data_dir.clone());\n      state.apply_runtime_lifecycle_transition_with_profile_sync_atomically(',
  ) }],
  ['terminal profile-sync CLI prepares intent before preflight', { runtimeLifecycle: validRuntimeLifecycle.replace(
    '    self.repository.mutate(|state| {\n      let profile = state.profiles.get(profile_id).ok_or_else(error)?;',
    '    self.repository.mutate(|state| {\n      let prepared_intent = prepare_lifecycle_intent(intent.clone());\n      let profile = state.profiles.get(profile_id).ok_or_else(error)?;',
  ).replace(
    '      let prepared_intent = prepare_lifecycle_intent(intent.clone());\n      state.apply_runtime_lifecycle_transition_with_profile_sync_atomically(',
    '      state.apply_runtime_lifecycle_transition_with_profile_sync_atomically(',
  ) }],
  ['terminal profile-sync CLI omits nonblank preflight', { runtimeLifecycle: validRuntimeLifecycle.replace(
    '        || profile_id.trim().is_empty()\n',
    '',
  ) }],
  ['terminal profile-sync CLI moves path conversion into mutation', { runtimeLifecycle: validRuntimeLifecycle.replace(
    '    let user_data_dir = profile_root.to_str().ok_or_else(error)?.to_string();\n    self.repository.mutate(|state| {',
    '    self.repository.mutate(|state| {\n      let user_data_dir = profile_root.to_str().ok_or_else(error)?.to_string();',
  ) }],
  ['missing process-exit owner revocation method', { serviceState: validServiceState.replace(
    /  pub fn revoke_process_exited_session_owner\([\s\S]*?\n  \}\n/,
    '',
  ) }],
  ['process-exit owner revocation stages registry', { serviceState: validServiceState.replace(
    '    let binding = self\n      .runtime_owner_registry\n      .binding_for_session(session_id)',
    '    let mut staged = self.runtime_owner_registry.clone();\n    let binding = staged\n      .binding_for_session(session_id)',
  ) }],
  ['process-exit owner revocation accepts generic intent', { serviceState: validServiceState.replace(
    '    session_id: &str,\n  ) -> Option<agent_browser_lease_authority::ProfileOwner> {',
    '    session_id: &str,\n    intent: agent_browser_lease_authority::RuntimeLifecycleIntent,\n  ) -> Option<agent_browser_lease_authority::ProfileOwner> {',
  ) }],
  ['process-exit CLI direct registry access', { controlPlane: validControlPlane.replace(
    '      let revoked_owner = service_state.revoke_process_exited_session_owner(&state.session_id)?;',
    '      let _ = service_state.runtime_owner_registry.binding_for_session(&state.session_id);\n      let revoked_owner = service_state.revoke_process_exited_session_owner(&state.session_id)?;',
  ) }],
  ['process-exit CLI session lookup before revocation', { controlPlane: validControlPlane.replace(
    '      let revoked_owner = service_state.revoke_process_exited_session_owner(&state.session_id)?;\n      let session = service_state.sessions.get(&state.session_id).cloned()?;',
    '      let session = service_state.sessions.get(&state.session_id).cloned()?;\n      let revoked_owner = service_state.revoke_process_exited_session_owner(&state.session_id)?;',
  ) }],
  ['process-exit CLI health event after revocation', { controlPlane: validControlPlane.replace(
    '  record_browser_health_changed_event(service_state, id, previous, browser);\n',
    '',
  ).replace(
    '  remove_browser_operational_record(service_state, id, session_id);',
    '  record_browser_health_changed_event(service_state, id, previous, browser);\n  remove_browser_operational_record(service_state, id, session_id);',
  ) }],
  ['superseded CLI legacy owner revocation wrapper', { runtimeLifecycle: `${validRuntimeLifecycle}
pub(crate) fn revoke_legacy_owner_in_registry() {}
` }],
  ['runtime reconciliation direct registry access after test item', {
    runtimeReconciliation: `${validRuntimeReconciliation}
#[cfg(test)]
mod tests { fn fixture(state: &ServiceState) { let _ = &state.runtime_owner_registry; } }
fn production(state: &ServiceState) { let _ = &state.runtime_owner_registry; }
`,
  }],
  ['lease effect direct registry access after test item', {
    leaseAuthorityAdapter: `${validLeaseAuthorityAdapter}
#[cfg(test)]
mod tests { fn fixture(state: &ServiceState) { let _ = &state.runtime_owner_registry; } }
fn production(state: &ServiceState) { let _ = &state.runtime_owner_registry; }
`,
  }],
  ['runtime reconciliation omits lane projection', {
    runtimeReconciliation: validRuntimeReconciliation.replace(
      'state.runtime_lane_authority(profile_identity_digest, logical_browser_id)',
      'authority_fixture()',
    ),
  }],
  ['lease effect omits profile projection', {
    leaseAuthorityAdapter: validLeaseAuthorityAdapter.replace(
      'state.profile_runtime_authority(profile_identity_digest)',
      'authority_fixture()',
    ),
  }],
  ['ordinary lifecycle transition direct field access', {
    runtimeLifecycle: `
impl Authority {
  pub(crate) fn transition(&self) { let _ = state.runtime_owner_registry.clone(); }
  fn transition_terminal_replacement_with_profile_sync(&self) {}
}
`,
  }],
  ['missing authentication aggregate method', { serviceState: validServiceState.replace(
    'pub fn observe_service_authentication_run(&mut self) {}',
    '',
  ) }],
  ['direct CLI authentication map access', { cli: 'fn leak(state: &ServiceState) { let _ = &state.authentication_runs; }\n' }],
  ['missing challenge aggregate method', { serviceState: validServiceState.replace(
    'pub fn resume_service_challenge_task(&mut self) {}',
    '',
  ) }],
  ['direct CLI challenge map access', { cli: 'fn leak(state: &ServiceState) { let _ = &state.challenge_tasks; }\n' }],
  ['upward aggregate import', { serviceState: `${validServiceState}\nuse crate::native::service_store::ServiceStateRepository;\n` }],
]) {
  const root = fixture({ manifest: validManifest, source: validSource, ...mutation });
  try {
    ok(check(root).length > 0, `${name} was accepted`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const missing = mkdtempSync(join(tmpdir(), 'agent-browser-service-model-missing-'));
try {
  deepEqual(check(missing).some((failure) => failure.includes('Cargo manifest must exist')), true);
} finally {
  rmSync(missing, { recursive: true, force: true });
}

const missingCrashModule = fixture({ manifest: validManifest, source: validSource,
  authenticationManifest: validAuthenticationManifest });
try {
  rmSync(join(missingCrashModule, 'crates/agent-browser-service-model/src/crash_regeneration.rs'));
  ok(check(missingCrashModule).some((failure) => failure.includes('crash_regeneration.rs')),
    'missing crash-regeneration module was accepted');
} finally {
  rmSync(missingCrashModule, { recursive: true, force: true });
}

const missingCapabilityModule = fixture({ manifest: validManifest, source: validSource,
  authenticationManifest: validAuthenticationManifest });
try {
  rmSync(join(missingCapabilityModule,
    'crates/agent-browser-service-model/src/browser_capability_registry.rs'));
  ok(check(missingCapabilityModule).some((failure) =>
    failure.includes('browser_capability_registry.rs')),
  'missing capability-registry module was accepted');
} finally {
  rmSync(missingCapabilityModule, { recursive: true, force: true });
}

const clean = fixture({ manifest: validManifest, source: validSource,
  authenticationManifest: validAuthenticationManifest });
try {
  doesNotThrow(() => ok(check(clean).length === 0, check(clean).join('\n')));
} finally {
  rmSync(clean, { recursive: true, force: true });
}

const testOnlyPrincipalAccess = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationManifest: validAuthenticationManifest,
  cliTest: 'fn inspect(state: &ServiceState) { let _ = &state.service_principals; }\n',
});
try {
  doesNotThrow(() => ok(check(testOnlyPrincipalAccess).length === 0,
    check(testOnlyPrincipalAccess).join('\n')));
} finally {
  rmSync(testOnlyPrincipalAccess, { recursive: true, force: true });
}

const indirectAggregateOwner = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    'agent_browser_service_model::PresentationCapacityAuthority',
    'super::presentation_capacity::PresentationCapacityAuthority',
  ),
});
try {
  ok(check(indirectAggregateOwner).length > 0,
  'indirect aggregate owner path was accepted');
} finally {
  rmSync(indirectAggregateOwner, { recursive: true, force: true });
}

const indirectPrincipalPredicate = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    'agent_browser_lease_authority::ServicePrincipalRegistry::is_empty',
    'super::service_principal::ServicePrincipalRegistry::is_empty',
  ),
});
try {
  ok(check(indirectPrincipalPredicate).length > 0,
  'indirect aggregate serde predicate was accepted');
} finally {
  rmSync(indirectPrincipalPredicate, { recursive: true, force: true });
}

const missingAuthenticationModule = fixture({ manifest: validManifest, source: validSource });
try {
  rmSync(join(missingAuthenticationModule,
    'crates/agent-browser-authentication-control/src/lib.rs'));
  ok(check(missingAuthenticationModule).some((failure) =>
    failure.includes('authentication-control must own src/lib.rs')),
  'missing authentication-control module was accepted');
} finally {
  rmSync(missingAuthenticationModule, { recursive: true, force: true });
}

const missingServiceAuthenticationModule = fixture({ manifest: validManifest, source: validSource });
try {
  rmSync(join(missingServiceAuthenticationModule,
    'crates/agent-browser-service-model/src/service_authentication_run.rs'));
  ok(check(missingServiceAuthenticationModule).some((failure) =>
    failure.includes('service_authentication_run.rs')),
  'missing Service authentication module was accepted');
} finally {
  rmSync(missingServiceAuthenticationModule, { recursive: true, force: true });
}

const duplicateServiceAuthenticationDefinition = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) struct ServiceAuthenticationRunRecord;\n',
});
try {
  ok(check(duplicateServiceAuthenticationDefinition).some((failure) =>
    failure.includes('ServiceAuthenticationRunRecord')),
  'duplicate CLI Service authentication definition was accepted');
} finally {
  rmSync(duplicateServiceAuthenticationDefinition, { recursive: true, force: true });
}

const missingServiceAuthenticationExport = fixture({
  manifest: validManifest,
  source: validSource.replace(validServiceAuthenticationExport, ''),
});
try {
  ok(check(missingServiceAuthenticationExport).some((failure) =>
    failure.includes('export the Service authentication')),
  'missing Service authentication export was accepted');
} finally {
  rmSync(missingServiceAuthenticationExport, { recursive: true, force: true });
}

const indirectServiceAuthenticationStateType = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    'agent_browser_service_model::ServiceAuthenticationRunRecord',
    'super::service_authentication_run::ServiceAuthenticationRunRecord',
  ),
});
try {
  ok(check(indirectServiceAuthenticationStateType).length > 0,
  'indirect Service authentication aggregate type was accepted');
} finally {
  rmSync(indirectServiceAuthenticationStateType, { recursive: true, force: true });
}

const duplicateServiceAuthenticationDecision = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) fn reserve_authentication_effect() {}\n',
});
try {
  ok(check(duplicateServiceAuthenticationDecision).some((failure) =>
    failure.includes('reserve_authentication_effect')),
  'duplicate CLI Service authentication decision was accepted');
} finally {
  rmSync(duplicateServiceAuthenticationDecision, { recursive: true, force: true });
}

const missingServiceChallengeModule = fixture({ manifest: validManifest, source: validSource });
try {
  rmSync(join(missingServiceChallengeModule,
    'crates/agent-browser-service-model/src/service_challenge_task.rs'));
  ok(check(missingServiceChallengeModule).some((failure) =>
    failure.includes('service_challenge_task.rs')),
  'missing Service challenge module was accepted');
} finally {
  rmSync(missingServiceChallengeModule, { recursive: true, force: true });
}

const duplicateServiceChallengeDefinition = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) struct ServiceChallengeTaskRecord;\n',
});
try {
  ok(check(duplicateServiceChallengeDefinition).some((failure) =>
    failure.includes('ServiceChallengeTaskRecord')),
  'duplicate CLI Service challenge definition was accepted');
} finally {
  rmSync(duplicateServiceChallengeDefinition, { recursive: true, force: true });
}

const missingServiceChallengeExport = fixture({
  manifest: validManifest,
  source: validSource.replace(validServiceChallengeExport, ''),
});
try {
  ok(check(missingServiceChallengeExport).some((failure) =>
    failure.includes('export the Service challenge')),
  'missing Service challenge export was accepted');
} finally {
  rmSync(missingServiceChallengeExport, { recursive: true, force: true });
}

const indirectServiceChallengeStateType = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    'agent_browser_service_model::ServiceChallengeTaskRecord',
    'super::service_challenge_task::ServiceChallengeTaskRecord',
  ),
});
try {
  ok(check(indirectServiceChallengeStateType).length > 0,
  'indirect Service challenge aggregate type was accepted');
} finally {
  rmSync(indirectServiceChallengeStateType, { recursive: true, force: true });
}

const duplicateServiceChallengeDecision = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) fn cancel_service_challenge_task() {}\n',
});
try {
  ok(check(duplicateServiceChallengeDecision).some((failure) =>
    failure.includes('cancel_service_challenge_task')),
  'duplicate CLI Service challenge decision was accepted');
} finally {
  rmSync(duplicateServiceChallengeDecision, { recursive: true, force: true });
}

const commentedServiceChallengeSkipPredicate = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    '#[serde(skip_serializing_if = "agent_browser_service_model::challenge_task_map_is_empty")]',
    '// #[serde(skip_serializing_if = "agent_browser_service_model::challenge_task_map_is_empty")]',
  ),
});
try {
  ok(check(commentedServiceChallengeSkipPredicate).some((failure) =>
    failure.includes('Service challenge empty-map decision')),
  'commented Service challenge empty-map predicate was accepted');
} finally {
  rmSync(commentedServiceChallengeSkipPredicate, { recursive: true, force: true });
}

const challengePredicateOnEarlierFieldOnly = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    'pub challenge_tasks:',
    'pub unrelated_challenges: BTreeMap<String, String>,\n  #[serde(default)]\n  pub challenge_tasks:',
  ),
});
try {
  ok(check(challengePredicateOnEarlierFieldOnly).some((failure) =>
    failure.includes('Service challenge empty-map decision')),
  'Service challenge guard borrowed a skip predicate from an earlier field');
} finally {
  rmSync(challengePredicateOnEarlierFieldOnly, { recursive: true, force: true });
}

const commentedServiceAuthenticationSkipPredicate = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    '#[serde(skip_serializing_if = "agent_browser_service_model::authentication_run_map_is_empty")]',
    '// #[serde(skip_serializing_if = "agent_browser_service_model::authentication_run_map_is_empty")]',
  ),
});
try {
  ok(check(commentedServiceAuthenticationSkipPredicate).some((failure) =>
    failure.includes('empty-map decision')),
  'commented Service authentication empty-map predicate was accepted');
} finally {
  rmSync(commentedServiceAuthenticationSkipPredicate, { recursive: true, force: true });
}

const predicateOnEarlierFieldOnly = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    'pub authentication_runs:',
    'pub unrelated_runs: BTreeMap<String, String>,\n  #[serde(default)]\n  pub authentication_runs:',
  ),
});
try {
  ok(check(predicateOnEarlierFieldOnly).some((failure) =>
    failure.includes('empty-map decision')),
  'Service authentication guard borrowed a skip predicate from an earlier field');
} finally {
  rmSync(predicateOnEarlierFieldOnly, { recursive: true, force: true });
}

const duplicateAuthenticationDefinition = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationFacade: `${validAuthenticationFacade}\npub(crate) struct AuthenticationRun;\n`,
});
try {
  ok(check(duplicateAuthenticationDefinition).some((failure) =>
    failure.includes('AuthenticationRun')),
  'duplicate CLI authentication-control definition was accepted');
} finally {
  rmSync(duplicateAuthenticationDefinition, { recursive: true, force: true });
}

const missingAuthenticationFacade = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationFacade: '',
});
try {
  ok(check(missingAuthenticationFacade).some((failure) =>
    failure.includes('only the authentication-control compatibility re-export')),
  'missing CLI authentication-control compatibility facade was accepted');
} finally {
  rmSync(missingAuthenticationFacade, { recursive: true, force: true });
}

const forbiddenAuthenticationDependency = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationManifest: `${validAuthenticationManifest}agent-browser-service-model = { path = "../agent-browser-service-model" }\n`,
});
try {
  ok(check(forbiddenAuthenticationDependency).some((failure) =>
    failure.includes('agent-browser-service-model')),
  'authentication-control upward Cargo dependency was accepted');
} finally {
  rmSync(forbiddenAuthenticationDependency, { recursive: true, force: true });
}

const unknownAuthenticationDependency = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationManifest: `${validAuthenticationManifest}keyring = "3"\n`,
});
try {
  ok(check(unknownAuthenticationDependency).some((failure) =>
    failure.includes('provider-free allowlist')),
  'unknown authentication-control dependency was accepted');
} finally {
  rmSync(unknownAuthenticationDependency, { recursive: true, force: true });
}

const aliasedAuthenticationDependency = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationManifest: validAuthenticationManifest.replace(
    'serde = "1"',
    'serde = { package = "keyring", version = "3" }',
  ),
});
try {
  ok(check(aliasedAuthenticationDependency).some((failure) =>
    failure.includes('must not alias')),
  'aliased authentication-control dependency was accepted');
} finally {
  rmSync(aliasedAuthenticationDependency, { recursive: true, force: true });
}

for (const authenticationManifest of [
  `${validAuthenticationManifest}\n[dependencies.keyring]\nversion = "3"\n`,
  `${validAuthenticationManifest}\n[build-dependencies.codegen]\nversion = "1"\n`,
  `${validAuthenticationManifest}\n[target.'cfg(unix)'.dependencies.keyring]\nversion = "3"\n`,
  `${validAuthenticationManifest}\n[dependencies.serde]\npackage = "keyring"\nversion = "3"\n`,
]) {
  const root = fixture({
    manifest: validManifest,
    source: validSource,
    authenticationManifest,
  });
  try {
    ok(check(root).some((failure) =>
      failure.includes('provider-free allowlist') || failure.includes('must not alias')),
    `authentication-control dependency table bypass was accepted: ${authenticationManifest}`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const authenticationFacadeWithLogic = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationFacade: `${validAuthenticationFacade}\npub(crate) fn adapter_logic() {}\n`,
});
try {
  ok(check(authenticationFacadeWithLogic).some((failure) =>
    failure.includes('only the authentication-control compatibility re-export')),
  'CLI authentication-control facade accepted extra logic');
} finally {
  rmSync(authenticationFacadeWithLogic, { recursive: true, force: true });
}

const forbiddenAuthenticationImport = fixture({
  manifest: validManifest,
  source: validSource,
  authentication: `${validAuthenticationControl}\nuse std::process::Command;\n`,
});
try {
  ok(check(forbiddenAuthenticationImport).some((failure) =>
    failure.includes('std::process')),
  'authentication-control process import was accepted');
} finally {
  rmSync(forbiddenAuthenticationImport, { recursive: true, force: true });
}

for (const name of authenticationControlRecords) {
  const root = fixture({
    manifest: validManifest,
    source: validSource,
    authentication: validAuthenticationControl.replace(`pub struct ${name};`, ''),
  });
  try {
    ok(check(root).some((failure) => failure.includes(name)),
      `missing authentication-control definition was accepted: ${name}`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

for (const name of authenticationControlTraits) {
  const root = fixture({
    manifest: validManifest,
    source: validSource,
    authentication: validAuthenticationControl.replace(`pub trait ${name} {}`, ''),
  });
  try {
    ok(check(root).some((failure) => failure.includes(name)),
      `missing authentication-control trait was accepted: ${name}`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const missingAuthenticationSchema = fixture({
  manifest: validManifest,
  source: validSource,
  authentication: validAuthenticationControl.replace(
    'pub const AUTHENTICATION_RUN_SCHEMA_VERSION',
    'const OTHER_SCHEMA_VERSION',
  ),
});
try {
  ok(check(missingAuthenticationSchema).some((failure) =>
    failure.includes('authentication run schema constant')),
  'missing authentication-control schema constant was accepted');
} finally {
  rmSync(missingAuthenticationSchema, { recursive: true, force: true });
}

for (const manifest of [
  `${validManifest}agent-browser = { path = "../../cli" }\n`,
  `${validManifest}tokio = "1"\n`,
  `${validManifest}"tokio" = "1"\n`,
  `${validManifest}\n[dependencies."tokio"]\nversion = "1"\n`,
  validManifest.replace(
    'serde = "1"',
    '"serde" = { "package" = "tokio", version = "1" }',
  ),
  validManifest.replace('serde = "1"', 'serde = { package = "tokio", version = "1" }'),
]) {
  const root = fixture({ manifest, source: validSource });
  try {
    ok(check(root).length > 0, 'forbidden Cargo dependency was accepted');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const forbiddenImport = fixture({
  manifest: validManifest,
  source: 'use std::fs;\npub struct Model;\n',
});
try {
  throws(() => ok(check(forbiddenImport).length === 0, 'forbidden Rust import was accepted'));
} finally {
  rmSync(forbiddenImport, { recursive: true, force: true });
}

const forbiddenCrashImport = fixture({
  manifest: validManifest,
  source: validSource,
  crash: `${validCrashRegeneration}\nuse crate::native::service_store::ServiceStateRepository;\n`,
});
try {
  ok(check(forbiddenCrashImport).some((failure) => failure.includes('crate::native')),
    'forbidden crash-regeneration upward import was accepted');
} finally {
  rmSync(forbiddenCrashImport, { recursive: true, force: true });
}

const duplicateCrashDefinition = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) struct CrashRegenerationTransaction;\n',
});
try {
  ok(check(duplicateCrashDefinition).some((failure) => failure.includes('CrashRegenerationTransaction')),
    'duplicate CLI crash-regeneration definition was accepted');
} finally {
  rmSync(duplicateCrashDefinition, { recursive: true, force: true });
}

const missingCrashExport = fixture({
  manifest: validManifest,
  source: validSource.replace(validCrashExport, ''),
});
try {
  ok(check(missingCrashExport).some((failure) => failure.includes('export the crash regeneration')),
    'missing crash-regeneration public export was accepted');
} finally {
  rmSync(missingCrashExport, { recursive: true, force: true });
}

const indirectCrashStateType = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    'agent_browser_service_model::CrashRegenerationTransaction',
    'super::service_crash_regeneration::CrashRegenerationTransaction',
  ),
});
try {
  ok(check(indirectCrashStateType).length > 0,
    'indirect CLI crash-regeneration transaction type was accepted');
} finally {
  rmSync(indirectCrashStateType, { recursive: true, force: true });
}

const duplicateCapabilityDefinition = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) struct BrowserCapabilityRegistry;\n',
});
try {
  ok(check(duplicateCapabilityDefinition).some((failure) =>
    failure.includes('BrowserCapabilityRegistry')),
  'duplicate CLI capability-registry definition was accepted');
} finally {
  rmSync(duplicateCapabilityDefinition, { recursive: true, force: true });
}

const missingCapabilityExport = fixture({
  manifest: validManifest,
  source: validSource.replace(validCapabilityExport, ''),
});
try {
  ok(check(missingCapabilityExport).some((failure) =>
    failure.includes('export the browser capability registry')),
  'missing capability-registry public export was accepted');
} finally {
  rmSync(missingCapabilityExport, { recursive: true, force: true });
}

const indirectCapabilityStateType = fixture({
  manifest: validManifest,
  source: validSource,
  serviceState: validServiceState.replace(
    'agent_browser_service_model::BrowserCapabilityRegistry',
    'BrowserCapabilityRegistry',
  ),
});
try {
  ok(check(indirectCapabilityStateType).length > 0,
  'indirect CLI capability-registry type was accepted');
} finally {
  rmSync(indirectCapabilityStateType, { recursive: true, force: true });
}

const duplicateCapabilityMatcher = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) fn browser_profile_compatibility_matches() {}\n',
});
try {
  ok(check(duplicateCapabilityMatcher).some((failure) =>
    failure.includes('browser_profile_compatibility_matches')),
  'duplicate CLI capability-registry matcher was accepted');
} finally {
  rmSync(duplicateCapabilityMatcher, { recursive: true, force: true });
}

for (const name of retirementRecords) {
  for (const mutation of [
    { retirement: validRetirement.replace(`pub struct ${name};`, '') },
    { cli: `pub(crate) struct ${name};` },
    { cli: `type ${name} = Other;` },
    { source: validSource + `\npub struct ${name};` },
  ]) {
    const root = fixture({ manifest: validManifest, source: validSource, ...mutation });
    try {
      ok(check(root).some((failure) => failure.includes(name)), `record ownership drift accepted: ${name}`);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  }
}

const missingSchema = fixture({
  manifest: validManifest,
  source: validSource,
  retirement: validRetirement.replace('pub const ABANDONED_BROWSER_RETIREMENT_PLAN_SCHEMA_V1', 'const OTHER_SCHEMA'),
});
try {
  ok(check(missingSchema).some((failure) => failure.includes('schema constant')));
} finally {
  rmSync(missingSchema, { recursive: true, force: true });
}

for (const addition of [
  'use crate::native::service_model::ServiceState;',
  'pub fn from_environment() {}',
  'pub fn read_env() { std::env::var("CONFIG"); }',
  'pub struct RetirementObservation;',
  'pub enum RetirementReservation {}',
  'pub trait AbandonedBrowserRetirementRuntime {}',
]) {
  const root = fixture({ manifest: validManifest, source: validSource, retirement: validRetirement + addition });
  try {
    ok(check(root).length > 0, `adapter import accepted: ${addition}`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

console.log('Service-model architecture guard fixture tests passed');
