//! First-class profile lease projection and guarded lifecycle operations.
//!
//! Profile leases are derived from authenticated principal capability, the
//! existing runtime owner registry, and subordinate session and tab work
//! leases. Labels never grant authority. Every mutation uses an exact lease
//! revision and the caller's authenticated profile capability.
//! Rejoin may repair one unproven live session only when it is the unique exact
//! session of the capability-bound runtime owner. It may compare-and-swap the
//! same capability binding to a newer current owner generation, then binds only
//! that session and its same-browser active tabs, with an explicit expiry.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use super::service_model::{
    LeaseState, ServiceEvent, ServiceEventKind, ServiceState, TabLifecycle,
};
use super::service_principal::{
    authenticate_profile_capability, authenticated_authority_is_current, bind_session_work_lease,
    bind_tab_work_lease, generate_profile_capability_token, register_profile_capability,
    AuthenticatedServicePrincipal, PrincipalContinuityRecourse, ServicePrincipalProvenance,
    ServicePrincipalRegistrationRequest, ServicePrincipalState, ServiceProfileCapabilityState,
};
use super::service_resources::load_service_state_for_maintenance;
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use super::service_trace::service_commands::service_now_timestamp;

pub(crate) const PROFILE_LEASE_SCHEMA_VERSION: &str = "agent-browser.profile-lease.v1";
pub(crate) const PROFILE_LEASE_RECONCILE_PLAN_SCHEMA_VERSION: &str =
    "agent-browser.profile-lease-reconcile-plan.v1";
pub(crate) const PROFILE_LEASE_RECONCILE_RECEIPT_SCHEMA_VERSION: &str =
    "agent-browser.profile-lease-reconcile-receipt.v1";

const READ_ACTIONS: [&str; 5] = ["list", "inspect", "explain", "doctor", "watch"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseRecord {
    pub(crate) schema_version: String,
    pub(crate) id: String,
    pub(crate) lease_revision: String,
    pub(crate) principal_id: Option<String>,
    pub(crate) principal_provenance: Option<ServicePrincipalProvenance>,
    pub(crate) profile_id: String,
    pub(crate) profile_identity_digest: Option<String>,
    pub(crate) browser_id: Option<String>,
    pub(crate) session_ids: Vec<String>,
    pub(crate) tab_ids: Vec<String>,
    pub(crate) mode: String,
    pub(crate) state: String,
    pub(crate) owner_generation: Option<u64>,
    pub(crate) process_instance_digest: Option<String>,
    pub(crate) route_ids: Vec<String>,
    pub(crate) last_heartbeat_at: Option<String>,
    pub(crate) expires_at: Option<String>,
    pub(crate) cleanup_obligation: Option<String>,
    pub(crate) blocking_identity_axes: Vec<String>,
    pub(crate) authorized_actions: Vec<String>,
    pub(crate) recourse: PrincipalContinuityRecourse,
    pub(crate) observation_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseFinding {
    pub(crate) code: String,
    pub(crate) severity: String,
    pub(crate) lease_id: String,
    pub(crate) profile_id: String,
    pub(crate) message: String,
    pub(crate) safe_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseDoctorReport {
    pub(crate) schema_version: String,
    pub(crate) observed_at: String,
    pub(crate) healthy: bool,
    pub(crate) lease_count: usize,
    pub(crate) findings: Vec<ProfileLeaseFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseTransition {
    pub(crate) action: String,
    pub(crate) session_id: String,
    pub(crate) from_state: String,
    pub(crate) to_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseReconcilePlan {
    pub(crate) schema_version: String,
    pub(crate) plan_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_revision: String,
    pub(crate) owner_generation: Option<u64>,
    pub(crate) principal_id: String,
    pub(crate) profile_id: String,
    pub(crate) browser_id: Option<String>,
    pub(crate) process_instance_digest: Option<String>,
    pub(crate) route_ids: Vec<String>,
    pub(crate) boot_epoch: Option<String>,
    pub(crate) proposed_transitions: Vec<ProfileLeaseTransition>,
    pub(crate) idempotency_key: String,
    pub(crate) issued_at: String,
    pub(crate) expires_at: String,
    pub(crate) effect_capable: bool,
    pub(crate) blocked_reasons: Vec<String>,
    pub(crate) seal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileLeaseReconcileReceipt {
    pub(crate) schema_version: String,
    pub(crate) idempotency_key: String,
    pub(crate) plan_id: String,
    pub(crate) lease_id: String,
    pub(crate) principal_id: String,
    pub(crate) applied_at: String,
    pub(crate) replayed: bool,
    pub(crate) transition_count: usize,
    pub(crate) resulting_lease_revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileLeaseFailureCode {
    LeaseNotFound,
    AuthorityMismatch,
    RevisionMismatch,
    ActionNotAuthorized,
    ActiveSubordinateWork,
    PlanInvalid,
    PlanExpired,
    BootEpochMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileLeaseError {
    pub(crate) code: ProfileLeaseFailureCode,
    pub(crate) message: String,
}

impl ProfileLeaseFailureCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LeaseNotFound => "lease_not_found",
            Self::AuthorityMismatch => "authority_mismatch",
            Self::RevisionMismatch => "revision_mismatch",
            Self::ActionNotAuthorized => "action_not_authorized",
            Self::ActiveSubordinateWork => "active_subordinate_work",
            Self::PlanInvalid => "plan_invalid",
            Self::PlanExpired => "plan_expired",
            Self::BootEpochMismatch => "boot_epoch_mismatch",
        }
    }
}

/// Projects the canonical first-class profile lease collection from retained authority and work.
/// Released and expired legacy sessions remain visible as nonblocking history;
/// only current legacy session or owner evidence requires identity reconciliation.
pub(crate) fn profile_leases_for_state(state: &ServiceState, now: &str) -> Vec<ProfileLeaseRecord> {
    let mut records = Vec::new();
    let mut bound_profiles = BTreeSet::new();
    let mut bound_capability_ids = BTreeSet::new();
    for binding in state.runtime_owner_registry.principal_bindings.values() {
        bound_profiles.insert(binding.profile_id.clone());
        bound_capability_ids.insert(binding.capability_id.clone());
        records.push(bound_profile_lease(state, binding, now));
    }

    for capability in state
        .service_principals
        .profile_capabilities
        .values()
        .filter(|capability| {
            capability.state == ServiceProfileCapabilityState::Active
                && !bound_capability_ids.contains(&capability.capability_id)
        })
    {
        bound_profiles.insert(capability.profile_id.clone());
        records.push(unbound_capability_profile_lease(state, capability, now));
    }

    let mut legacy_profiles = state
        .sessions
        .values()
        .filter_map(|session| session.profile_id.clone())
        .filter(|profile_id| !bound_profiles.contains(profile_id))
        .collect::<BTreeSet<_>>();
    for owner in state.runtime_owner_registry.owners.values() {
        if state
            .runtime_owner_registry
            .principal_bindings
            .contains_key(&owner.profile_identity_digest)
        {
            continue;
        }
        if let Some(profile_id) = state
            .browsers
            .get(&owner.browser_id)
            .and_then(|browser| browser.profile_id.clone())
        {
            legacy_profiles.insert(profile_id);
        }
    }
    for profile_id in legacy_profiles {
        records.push(legacy_profile_lease(state, &profile_id, now));
    }
    records.sort_by(|left, right| left.id.cmp(&right.id));
    records
}

/// Handles the no-launch CLI collection read and includes the matching doctor result.
pub(crate) async fn handle_service_profile_leases(
    command: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = load_service_state_for_maintenance(command)?;
    let now = service_now_timestamp();
    let profile_leases = profile_leases_for_state(&state, &now);
    let doctor = doctor_profile_leases(&state, &now);
    Ok(json!({
        "profileLeases": profile_leases,
        "count": profile_leases.len(),
        "observedAt": now,
        "doctor": doctor,
    }))
}

/// Handles first-class profile lease detail, diagnosis, registration, and owner actions.
///
/// Raw profile capabilities are accepted only as ephemeral command input or read
/// from a private capability file. They are authenticated against the retained
/// digest and never persisted in Service State or lifecycle events.
pub(crate) async fn handle_service_profile_lease_command(
    command: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let action = required_command_string(command, "action")?;
    match action {
        "service_profile_lease_inspect" => {
            let state = load_service_state_for_maintenance(command)?;
            let now = service_now_timestamp();
            let lease_id = required_command_string(command, "leaseId")?;
            let lease =
                inspect_profile_lease(&state, lease_id, &now).map_err(lease_error_string)?;
            Ok(json!({ "lease": lease, "observedAt": now }))
        }
        "service_profile_lease_explain" => {
            let state = load_service_state_for_maintenance(command)?;
            let now = service_now_timestamp();
            let lease_id = required_command_string(command, "leaseId")?;
            let lease =
                inspect_profile_lease(&state, lease_id, &now).map_err(lease_error_string)?;
            let doctor = doctor_profile_leases(&state, &now);
            let findings = doctor
                .findings
                .iter()
                .filter(|finding| finding.lease_id == lease.id)
                .cloned()
                .collect::<Vec<_>>();
            Ok(json!({
                "lease": lease,
                "explanation": {
                    "recourse": lease.recourse,
                    "blockingIdentityAxes": lease.blocking_identity_axes,
                    "authorizedActions": lease.authorized_actions,
                    "observationOnly": lease.observation_only,
                    "findings": findings,
                },
                "observedAt": now,
            }))
        }
        "service_profile_lease_doctor" => {
            let state = load_service_state_for_maintenance(command)?;
            let now = service_now_timestamp();
            Ok(json!({ "doctor": doctor_profile_leases(&state, &now) }))
        }
        "service_profile_lease_register" => register_profile_lease_principal(command),
        "service_profile_lease_rejoin"
        | "service_profile_lease_renew"
        | "service_profile_lease_release" => mutate_profile_lease(command),
        "service_profile_lease_reconcile_plan" => plan_profile_lease_command(command),
        "service_profile_lease_reconcile_apply" => apply_profile_lease_command(command),
        _ => Err(format!("Unsupported profile lease command: {action}")),
    }
}

fn plan_profile_lease_command(command: &serde_json::Value) -> Result<serde_json::Value, String> {
    let state = load_service_state_for_maintenance(command)?;
    let lease_id = required_command_string(command, "leaseId")?;
    let expected_revision = required_command_string(command, "leaseRevision")?;
    let expires_at = required_command_string(command, "expiresAt")?;
    let raw_capability = profile_capability_from_command(command)?;
    let authority =
        authenticate_profile_capability(&state.service_principals, raw_capability.as_str(), None)
            .map_err(principal_error_string)?;
    let idempotency_key = optional_command_string(command, "idempotencyKey")
        .unwrap_or_else(|| format!("profile-lease-reconcile-{}", uuid::Uuid::new_v4()));
    let now = service_now_timestamp();
    let boot_epoch = crate::process_identity::current_boot_epoch();
    let plan = plan_profile_lease_reconciliation(
        &state,
        lease_id,
        expected_revision,
        &authority,
        &now,
        expires_at,
        boot_epoch,
        idempotency_key,
        raw_capability.as_bytes(),
    )
    .map_err(lease_error_string)?;
    Ok(json!({ "plan": plan }))
}

fn apply_profile_lease_command(command: &serde_json::Value) -> Result<serde_json::Value, String> {
    let raw_capability = profile_capability_from_command(command)?;
    let plan = if let Some(plan) = command.get("plan") {
        serde_json::from_value::<ProfileLeaseReconcilePlan>(plan.clone())
            .map_err(|error| format!("profile_lease_plan_invalid:{error}"))?
    } else {
        let path = absolute_command_path(command, "planFile")?;
        let encoded = fs::read(&path).map_err(|error| {
            format!("Failed to read reconcile plan {}: {error}", path.display())
        })?;
        serde_json::from_slice::<ProfileLeaseReconcilePlan>(&encoded)
            .map_err(|error| format!("profile_lease_plan_invalid:{error}"))?
    };
    let now = service_now_timestamp();
    let boot_epoch = crate::process_identity::current_boot_epoch();
    let repository = LockedServiceStateRepository::default_json()?;
    repository.mutate(|state| {
        let authority = authenticate_profile_capability(
            &state.service_principals,
            raw_capability.as_str(),
            None,
        )
        .map_err(principal_error_string)?;
        let receipt = apply_profile_lease_reconciliation(
            state,
            &plan,
            &authority,
            &now,
            boot_epoch.as_deref(),
            raw_capability.as_bytes(),
        )
        .map_err(lease_error_string)?;
        append_profile_lease_event(
            state,
            "reconcile_apply",
            None,
            None,
            &authority.principal_id,
            &authority.profile_id,
            plan.browser_id.as_deref(),
            &now,
            command,
        );
        Ok(json!({ "receipt": receipt }))
    })
}

fn register_profile_lease_principal(
    command: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let principal_id = required_command_string(command, "principalId")?.to_string();
    let profile_id = required_command_string(command, "profileId")?.to_string();
    let capability_path = absolute_command_path(command, "capabilityOut")?;
    let display_name = optional_command_string(command, "displayName");
    let registered_by = optional_command_string(command, "registeredBy");
    let now = service_now_timestamp();
    let raw_capability = generate_profile_capability_token();

    write_private_capability_file(&capability_path, &raw_capability)?;
    let repository = LockedServiceStateRepository::default_json()?;
    let result = repository.mutate(|state| {
        let registered = register_profile_capability(
            &mut state.service_principals,
            ServicePrincipalRegistrationRequest {
                principal_id: principal_id.clone(),
                display_name: display_name.clone(),
                profile_id: profile_id.clone(),
                registered_at: Some(now.clone()),
                registered_by: registered_by.clone(),
            },
            &raw_capability,
        )
        .map_err(principal_error_string)?;

        let owner_binding = bind_registered_principal_to_current_owner(state, &registered)?;
        append_profile_lease_event(
            state,
            "register",
            None,
            None,
            &registered.principal.principal_id,
            &registered.capability.profile_id,
            None,
            &now,
            command,
        );
        Ok(json!({
            "principal": registered.principal,
            "capability": {
                "capabilityId": registered.capability.capability_id,
                "profileId": registered.capability.profile_id,
                "revision": registered.capability.revision,
                "state": registered.capability.state,
            },
            "boundToCurrentOwner": owner_binding,
            "capabilityFile": capability_path,
            "capabilityWritten": true,
        }))
    });
    if let Err(error) = result {
        let cleanup = fs::remove_file(&capability_path);
        return match cleanup {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}; failed to remove unregistered capability file {}: {cleanup_error}",
                capability_path.display()
            )),
        };
    }
    result
}

fn bind_registered_principal_to_current_owner(
    state: &mut ServiceState,
    registered: &super::service_principal::RegisteredProfileCapability,
) -> Result<bool, String> {
    let Some(profile_path) = state
        .profiles
        .get(&registered.capability.profile_id)
        .and_then(|profile| profile.user_data_dir.as_deref())
    else {
        return Ok(false);
    };
    let profile_identity_digest =
        crate::runtime_profile::canonical_profile_identity_digest(Path::new(profile_path))?;
    let Some(owner) = state
        .runtime_owner_registry
        .owners
        .get(&profile_identity_digest)
        .cloned()
    else {
        return Ok(false);
    };
    state
        .runtime_owner_registry
        .bind_principal_authority(
            crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                principal_id: registered.principal.principal_id.clone(),
                profile_id: registered.capability.profile_id.clone(),
                profile_identity_digest,
                capability_id: registered.capability.capability_id.clone(),
                provenance: ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: owner.owner_generation,
            },
        )
        .map_err(|error| format!("profile_owner_binding_failed:{error:?}"))?;
    Ok(true)
}

fn mutate_profile_lease(command: &serde_json::Value) -> Result<serde_json::Value, String> {
    let action = required_command_string(command, "action")?;
    let operation = action
        .strip_prefix("service_profile_lease_")
        .ok_or_else(|| format!("Invalid profile lease action: {action}"))?;
    let lease_id = required_command_string(command, "leaseId")?.to_string();
    let expected_revision = required_command_string(command, "leaseRevision")?.to_string();
    let raw_capability = profile_capability_from_command(command)?;
    let expires_at = optional_command_string(command, "expiresAt");
    let now = service_now_timestamp();
    let repository = LockedServiceStateRepository::default_json()?;

    repository.mutate(|state| {
        let authority = authenticate_profile_capability(
            &state.service_principals,
            raw_capability.as_str(),
            None,
        )
        .map_err(principal_error_string)?;
        let before = inspect_profile_lease(state, &lease_id, &now).map_err(lease_error_string)?;
        let lease = match operation {
            "rejoin" => rejoin_profile_lease(
                state,
                &lease_id,
                &expected_revision,
                &authority,
                &now,
                expires_at.as_deref(),
            ),
            "renew" => renew_profile_lease(
                state,
                &lease_id,
                &expected_revision,
                &authority,
                &now,
                expires_at
                    .as_deref()
                    .ok_or_else(|| "profile_lease_expires_at_required".to_string())?,
            ),
            "release" => {
                release_profile_lease(state, &lease_id, &expected_revision, &authority, &now)
            }
            _ => return Err(format!("Unsupported profile lease operation: {operation}")),
        }
        .map_err(lease_error_string)?;
        append_profile_lease_event(
            state,
            operation,
            Some(&before),
            Some(&lease),
            &authority.principal_id,
            &authority.profile_id,
            lease.browser_id.as_deref(),
            &now,
            command,
        );
        Ok(json!({
            "operation": operation,
            "lease": lease,
            "previousLeaseRevision": before.lease_revision,
            "leaseRevision": lease.lease_revision,
            "principalId": authority.principal_id,
            "profileId": authority.profile_id,
            "appliedAt": now,
        }))
    })
}

#[allow(clippy::too_many_arguments)]
fn append_profile_lease_event(
    state: &mut ServiceState,
    operation: &str,
    before: Option<&ProfileLeaseRecord>,
    after: Option<&ProfileLeaseRecord>,
    principal_id: &str,
    profile_id: &str,
    browser_id: Option<&str>,
    now: &str,
    command: &serde_json::Value,
) {
    state.events.push(ServiceEvent {
        id: format!("event-{}", uuid::Uuid::new_v4()),
        timestamp: now.to_string(),
        kind: ServiceEventKind::ProfileLeaseLifecycleChanged,
        message: format!("Profile lease {operation}: {profile_id}"),
        browser_id: browser_id.map(ToString::to_string),
        profile_id: Some(profile_id.to_string()),
        session_id: after
            .or(before)
            .and_then(|lease| lease.session_ids.first().cloned()),
        service_name: optional_command_string(command, "serviceName"),
        agent_name: optional_command_string(command, "agentName"),
        task_name: optional_command_string(command, "taskName"),
        details: Some(json!({
            "operation": operation,
            "principalId": principal_id,
            "previousLeaseRevision": before.map(|lease| lease.lease_revision.as_str()),
            "leaseRevision": after.map(|lease| lease.lease_revision.as_str()),
            "authorizedActions": after.map(|lease| &lease.authorized_actions),
            "recourse": after.map(|lease| lease.recourse),
        })),
        ..ServiceEvent::default()
    });
    if state.events.len() > 100 {
        let excess = state.events.len() - 100;
        state.events.drain(0..excess);
    }
}

fn profile_capability_from_command(command: &serde_json::Value) -> Result<String, String> {
    if let Some(capability) = optional_command_string(command, "profileCapability") {
        return Ok(capability);
    }
    let path = absolute_command_path(command, "profileCapabilityFile")?;
    read_private_capability_file(&path)
}

fn read_private_capability_file(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|error| {
        format!(
            "profile_capability_file_unreadable:{}:{error}",
            path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "profile_capability_file_not_regular:{}",
            path.display()
        ));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(format!(
            "profile_capability_file_permissions_too_open:{}",
            path.display()
        ));
    }
    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "profile_capability_file_unreadable:{}:{error}",
            path.display()
        )
    })?;
    if raw.len() > 4096 {
        return Err("profile_capability_file_too_large".to_string());
    }
    let capability = raw.trim().to_string();
    if capability.is_empty() {
        return Err("profile_capability_file_empty".to_string());
    }
    Ok(capability)
}

fn write_private_capability_file(path: &Path, capability: &str) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("profile_capability_output_must_be_absolute".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "profile_capability_output_has_no_parent".to_string())?;
    if !parent.is_dir() {
        return Err(format!(
            "profile_capability_output_parent_missing:{}",
            parent.display()
        ));
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).map_err(|error| {
        format!(
            "profile_capability_output_create_failed:{}:{error}",
            path.display()
        )
    })?;
    if let Err(error) = file
        .write_all(format!("{capability}\n").as_bytes())
        .and_then(|_| file.sync_all())
    {
        let _ = fs::remove_file(path);
        return Err(format!(
            "profile_capability_output_write_failed:{}:{error}",
            path.display()
        ));
    }
    Ok(())
}

fn required_command_string<'a>(
    command: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, String> {
    command
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Missing required profile lease field: {field}"))
}

fn optional_command_string(command: &serde_json::Value, field: &str) -> Option<String> {
    command
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn absolute_command_path(command: &serde_json::Value, field: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(required_command_string(command, field)?);
    if !path.is_absolute() {
        return Err(format!("{field} must be an absolute path"));
    }
    Ok(path)
}

fn lease_error_string(error: ProfileLeaseError) -> String {
    format!("profile_lease_{}:{}", error.code.as_str(), error.message)
}

fn principal_error_string(error: super::service_principal::ServicePrincipalError) -> String {
    format!(
        "service_principal_{}:{}",
        error.code.as_str(),
        error.message
    )
}

pub(crate) fn inspect_profile_lease(
    state: &ServiceState,
    lease_id: &str,
    now: &str,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    profile_leases_for_state(state, now)
        .into_iter()
        .find(|lease| lease.id == lease_id)
        .ok_or_else(|| lease_error(ProfileLeaseFailureCode::LeaseNotFound, lease_id))
}

pub(crate) fn doctor_profile_leases(state: &ServiceState, now: &str) -> ProfileLeaseDoctorReport {
    let leases = profile_leases_for_state(state, now);
    let mut findings = Vec::new();
    for lease in &leases {
        for axis in &lease.blocking_identity_axes {
            findings.push(ProfileLeaseFinding {
                code: axis.clone(),
                severity: if lease.observation_only {
                    "warning"
                } else {
                    "error"
                }
                .to_string(),
                lease_id: lease.id.clone(),
                profile_id: lease.profile_id.clone(),
                message: format!("Profile lease {} is blocked by {}", lease.id, axis),
                safe_actions: lease
                    .authorized_actions
                    .iter()
                    .filter(|action| {
                        matches!(
                            action.as_str(),
                            "inspect" | "explain" | "doctor" | "reconcile_plan"
                        )
                    })
                    .cloned()
                    .collect(),
            });
        }
    }
    ProfileLeaseDoctorReport {
        schema_version: PROFILE_LEASE_SCHEMA_VERSION.to_string(),
        observed_at: now.to_string(),
        healthy: findings.is_empty(),
        lease_count: leases.len(),
        findings,
    }
}

/// Re-establishes exact session work under an authenticated profile capability.
/// A capability registered before first launch may bind the later unique,
/// uncontested ready owner as part of the same revision-guarded mutation.
pub(crate) fn rejoin_profile_lease(
    state: &mut ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
    expires_at: Option<&str>,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    let lease = authorized_lease(state, lease_id, expected_revision, authority, now)?;
    require_action(&lease, "rejoin")?;
    if !lease.observation_only {
        return Ok(lease);
    }
    let expires_at = expires_at
        .filter(|expires_at| *expires_at > now)
        .ok_or_else(|| lease_error(ProfileLeaseFailureCode::ActionNotAuthorized, lease_id))?;
    let has_principal_binding = state
        .runtime_owner_registry
        .principal_bindings
        .values()
        .any(|binding| {
            binding.principal_id == authority.principal_id
                && binding.profile_id == authority.profile_id
                && binding.capability_id == authority.capability_id
                && binding.provenance == authority.provenance
        });
    let (session_id, tab_ids) = if has_principal_binding {
        exact_rejoin_target(state, authority, now)
    } else {
        lease.profile_identity_digest.as_deref().and_then(|digest| {
            (!state
                .runtime_owner_registry
                .principal_bindings
                .contains_key(digest))
            .then(|| exact_rejoin_target_for_owner(state, authority, digest, now))
            .flatten()
        })
    }
    .ok_or_else(|| lease_error(ProfileLeaseFailureCode::ActionNotAuthorized, lease_id))?;
    if has_principal_binding {
        refresh_principal_binding_to_current_owner(state, authority)
    } else {
        bind_principal_to_current_owner(state, authority, &lease)
    }
    .map_err(|_| lease_error(ProfileLeaseFailureCode::ActionNotAuthorized, lease_id))?;
    bind_session_work_lease(state, &session_id, authority, expires_at.to_string())
        .map_err(|_| lease_error(ProfileLeaseFailureCode::ActionNotAuthorized, lease_id))?;
    for tab_id in tab_ids {
        bind_tab_work_lease(state, &tab_id, authority, expires_at.to_string())
            .map_err(|_| lease_error(ProfileLeaseFailureCode::ActionNotAuthorized, lease_id))?;
    }
    inspect_profile_lease(state, lease_id, now)
}

pub(crate) fn renew_profile_lease(
    state: &mut ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
    expires_at: &str,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    let lease = authorized_lease(state, lease_id, expected_revision, authority, now)?;
    require_action(&lease, "renew")?;
    if expires_at <= now {
        return Err(lease_error(
            ProfileLeaseFailureCode::ActionNotAuthorized,
            lease_id,
        ));
    }
    for session in state.sessions.values_mut().filter(|session| {
        session.profile_id.as_deref() == Some(authority.profile_id.as_str())
            && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
            && !inactive_session(session.lease)
    }) {
        session.expires_at = Some(expires_at.to_string());
        session.last_lease_observed_at = Some(now.to_string());
        session.boot_epoch = crate::process_identity::current_boot_epoch();
        session.work_lease_revision = session.work_lease_revision.saturating_add(1).max(1);
    }
    inspect_profile_lease(state, lease_id, now)
}

pub(crate) fn release_profile_lease(
    state: &mut ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    let lease = authorized_lease(state, lease_id, expected_revision, authority, now)?;
    require_action(&lease, "release")?;
    if active_subordinate_tabs(state, authority, now)
        .next()
        .is_some()
    {
        return Err(lease_error(
            ProfileLeaseFailureCode::ActiveSubordinateWork,
            lease_id,
        ));
    }
    for session in state.sessions.values_mut().filter(|session| {
        session.profile_id.as_deref() == Some(authority.profile_id.as_str())
            && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
            && !inactive_session(session.lease)
    }) {
        session.lease = LeaseState::Released;
        session.expires_at = Some(now.to_string());
        session.last_lease_observed_at = Some(now.to_string());
        session.boot_epoch = crate::process_identity::current_boot_epoch();
        session.work_lease_revision = session.work_lease_revision.saturating_add(1).max(1);
    }
    inspect_profile_lease(state, lease_id, now)
}

#[allow(clippy::too_many_arguments)]
/// Builds a sealed, no-effect descriptor for exact stale-session release or
/// same-capability owner-generation refresh. A binding refresh is proposed
/// only when the ready owner and every retained lease identity already agree.
pub(crate) fn plan_profile_lease_reconciliation(
    state: &ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
    expires_at: &str,
    boot_epoch: Option<String>,
    idempotency_key: String,
    seal_key: &[u8],
) -> Result<ProfileLeaseReconcilePlan, ProfileLeaseError> {
    let lease = authorized_lease(state, lease_id, expected_revision, authority, now)?;
    require_action(&lease, "reconcile_plan")?;
    if seal_key.len() < 32 || idempotency_key.trim().is_empty() {
        return Err(lease_error(ProfileLeaseFailureCode::PlanInvalid, lease_id));
    }
    match rfc3339_at_or_after(now, expires_at) {
        Ok(true) => {
            return Err(lease_error(ProfileLeaseFailureCode::PlanExpired, lease_id));
        }
        Ok(false) => {}
        Err(()) => {
            return Err(lease_error(ProfileLeaseFailureCode::PlanInvalid, lease_id));
        }
    }
    let boot_epoch = boot_epoch.filter(|epoch| !epoch.trim().is_empty());
    let mut proposed_transitions = state
        .sessions
        .values()
        .filter(|session| {
            session.profile_id.as_deref() == Some(authority.profile_id.as_str())
                && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
                && inactive_session(session.lease)
        })
        .map(|session| ProfileLeaseTransition {
            action: "confirm_stale_session_released".to_string(),
            session_id: session.id.clone(),
            from_state: lease_state_name(session.lease).to_string(),
            to_state: "released".to_string(),
        })
        .collect::<Vec<_>>();
    if lease.blocking_identity_axes == ["owner_generation_or_binding_mismatch"] {
        let binding = state
            .runtime_owner_registry
            .principal_bindings
            .values()
            .find(|binding| {
                binding.principal_id == authority.principal_id
                    && binding.profile_id == authority.profile_id
                    && binding.capability_id == authority.capability_id
                    && binding.provenance == authority.provenance
                    && lease.profile_identity_digest.as_deref()
                        == Some(binding.profile_identity_digest.as_str())
            });
        let owner = binding.and_then(|binding| {
            state
                .runtime_owner_registry
                .owners
                .get(&binding.profile_identity_digest)
                .map(|owner| (binding, owner))
        });
        if let Some((binding, owner)) = owner.filter(|(binding, owner)| {
            owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready
                && owner.owner_generation > binding.owner_generation
                && lease.owner_generation == Some(owner.owner_generation)
                && lease.browser_id.as_deref() == Some(owner.browser_id.as_str())
                && lease.process_instance_digest.as_deref()
                    == Some(owner.process_instance_digest.as_str())
        }) {
            proposed_transitions.push(ProfileLeaseTransition {
                action: "refresh_principal_owner_binding".to_string(),
                session_id: owner.daemon_session_route.clone(),
                from_state: binding.owner_generation.to_string(),
                to_state: owner.owner_generation.to_string(),
            });
        }
    }
    let mut blocked_reasons = Vec::new();
    if boot_epoch.is_none() {
        blocked_reasons.push("boot_epoch_unavailable".to_string());
    }
    if proposed_transitions.is_empty() {
        blocked_reasons.push("no_safe_reconciliation_transition".to_string());
    }
    let effect_capable = blocked_reasons.is_empty();
    let mut plan = ProfileLeaseReconcilePlan {
        schema_version: PROFILE_LEASE_RECONCILE_PLAN_SCHEMA_VERSION.to_string(),
        plan_id: format!(
            "profile-lease-reconcile-plan-v1:{}",
            digest_prefix(format!("{lease_id}\0{expected_revision}\0{idempotency_key}").as_bytes())
        ),
        lease_id: lease.id,
        lease_revision: lease.lease_revision,
        owner_generation: lease.owner_generation,
        principal_id: authority.principal_id.clone(),
        profile_id: authority.profile_id.clone(),
        browser_id: lease.browser_id,
        process_instance_digest: lease.process_instance_digest,
        route_ids: lease.route_ids,
        boot_epoch,
        proposed_transitions,
        idempotency_key,
        issued_at: now.to_string(),
        expires_at: expires_at.to_string(),
        effect_capable,
        blocked_reasons,
        seal: String::new(),
    };
    plan.seal = seal_reconcile_plan(&plan, seal_key);
    Ok(plan)
}

/// Applies only the exact sealed transitions whose lease, owner, capability,
/// boot epoch, and compare-and-swap evidence remain current.
pub(crate) fn apply_profile_lease_reconciliation(
    state: &mut ServiceState,
    plan: &ProfileLeaseReconcilePlan,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
    current_boot_epoch: Option<&str>,
    seal_key: &[u8],
) -> Result<ProfileLeaseReconcileReceipt, ProfileLeaseError> {
    if !authenticated_authority_is_current(&state.service_principals, authority) {
        return Err(lease_error(
            ProfileLeaseFailureCode::AuthorityMismatch,
            &plan.lease_id,
        ));
    }
    if let Some(receipt) = state
        .profile_lease_reconcile_receipts
        .get(&plan.idempotency_key)
    {
        if receipt.principal_id != authority.principal_id {
            return Err(lease_error(
                ProfileLeaseFailureCode::AuthorityMismatch,
                &plan.lease_id,
            ));
        }
        let mut replay = receipt.clone();
        replay.replayed = true;
        return Ok(replay);
    }
    if seal_key.len() < 32
        || plan.seal != seal_reconcile_plan(plan, seal_key)
        || !plan.effect_capable
    {
        return Err(lease_error(
            ProfileLeaseFailureCode::PlanInvalid,
            &plan.lease_id,
        ));
    }
    match rfc3339_at_or_after(now, &plan.expires_at) {
        Ok(true) => {
            return Err(lease_error(
                ProfileLeaseFailureCode::PlanExpired,
                &plan.lease_id,
            ));
        }
        Ok(false) => {}
        Err(()) => {
            return Err(lease_error(
                ProfileLeaseFailureCode::PlanInvalid,
                &plan.lease_id,
            ));
        }
    }
    if plan.boot_epoch.as_deref() != current_boot_epoch {
        return Err(lease_error(
            ProfileLeaseFailureCode::BootEpochMismatch,
            &plan.lease_id,
        ));
    }
    let lease = authorized_lease(state, &plan.lease_id, &plan.lease_revision, authority, now)?;
    if plan.owner_generation != lease.owner_generation
        || plan.principal_id != authority.principal_id
        || plan.profile_id != authority.profile_id
        || plan.browser_id != lease.browser_id
        || plan.process_instance_digest != lease.process_instance_digest
        || plan.route_ids != lease.route_ids
    {
        return Err(lease_error(
            ProfileLeaseFailureCode::PlanInvalid,
            &plan.lease_id,
        ));
    }
    for transition in &plan.proposed_transitions {
        if transition.action == "refresh_principal_owner_binding" {
            let binding = state
                .runtime_owner_registry
                .principal_bindings
                .values()
                .find(|binding| {
                    binding.principal_id == authority.principal_id
                        && binding.profile_id == authority.profile_id
                        && binding.capability_id == authority.capability_id
                        && binding.provenance == authority.provenance
                })
                .ok_or_else(|| lease_error(ProfileLeaseFailureCode::PlanInvalid, &plan.lease_id))?;
            let owner = state
                .runtime_owner_registry
                .owners
                .get(&binding.profile_identity_digest)
                .ok_or_else(|| lease_error(ProfileLeaseFailureCode::PlanInvalid, &plan.lease_id))?;
            if binding.owner_generation.to_string() != transition.from_state
                || owner.owner_generation.to_string() != transition.to_state
                || owner.daemon_session_route != transition.session_id
                || owner.state != crate::runtime_owner_transfer::ProfileOwnerState::Ready
            {
                return Err(lease_error(
                    ProfileLeaseFailureCode::PlanInvalid,
                    &plan.lease_id,
                ));
            }
            refresh_principal_binding_to_current_owner(state, authority)
                .map_err(|_| lease_error(ProfileLeaseFailureCode::PlanInvalid, &plan.lease_id))?;
            continue;
        }
        let session = state
            .sessions
            .get_mut(&transition.session_id)
            .ok_or_else(|| lease_error(ProfileLeaseFailureCode::PlanInvalid, &plan.lease_id))?;
        if session.profile_id.as_deref() != Some(authority.profile_id.as_str())
            || session.principal_id.as_deref() != Some(authority.principal_id.as_str())
            || !inactive_session(session.lease)
            || transition.action != "confirm_stale_session_released"
        {
            return Err(lease_error(
                ProfileLeaseFailureCode::PlanInvalid,
                &plan.lease_id,
            ));
        }
        session.lease = LeaseState::Released;
        session.expires_at = Some(now.to_string());
        session.last_lease_observed_at = Some(now.to_string());
        session.boot_epoch = crate::process_identity::current_boot_epoch();
        session.work_lease_revision = session.work_lease_revision.saturating_add(1).max(1);
    }
    let resulting = inspect_profile_lease(state, &plan.lease_id, now)?;
    let receipt = ProfileLeaseReconcileReceipt {
        schema_version: PROFILE_LEASE_RECONCILE_RECEIPT_SCHEMA_VERSION.to_string(),
        idempotency_key: plan.idempotency_key.clone(),
        plan_id: plan.plan_id.clone(),
        lease_id: plan.lease_id.clone(),
        principal_id: plan.principal_id.clone(),
        applied_at: now.to_string(),
        replayed: false,
        transition_count: plan.proposed_transitions.len(),
        resulting_lease_revision: resulting.lease_revision,
    };
    state
        .profile_lease_reconcile_receipts
        .insert(receipt.idempotency_key.clone(), receipt.clone());
    Ok(receipt)
}

fn bound_profile_lease(
    state: &ServiceState,
    binding: &crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding,
    now: &str,
) -> ProfileLeaseRecord {
    let owner = state
        .runtime_owner_registry
        .owners
        .get(&binding.profile_identity_digest);
    let owner_current = state
        .runtime_owner_registry
        .principal_binding_is_current(Some(binding));
    let capability_current = state
        .service_principals
        .profile_capabilities
        .get(&binding.capability_id)
        .is_some_and(|capability| {
            capability.state == ServiceProfileCapabilityState::Active
                && capability.principal_id == binding.principal_id
                && capability.profile_id == binding.profile_id
                && state
                    .service_principals
                    .principals
                    .get(&binding.principal_id)
                    .is_some_and(|principal| principal.state == ServicePrincipalState::Active)
        });
    let sessions = sessions_for_profile(state, &binding.profile_id);
    let active = sessions
        .iter()
        .copied()
        .filter(|session| !inactive_or_expired(session.lease, session.expires_at.as_deref(), now))
        .collect::<Vec<_>>();
    let same = active
        .iter()
        .copied()
        .filter(|session| session.principal_id.as_deref() == Some(binding.principal_id.as_str()))
        .collect::<Vec<_>>();
    let foreign = active
        .iter()
        .copied()
        .filter(|session| {
            session
                .principal_id
                .as_deref()
                .is_some_and(|principal_id| principal_id != binding.principal_id)
        })
        .collect::<Vec<_>>();
    let unproven = active
        .iter()
        .copied()
        .filter(|session| {
            session.principal_id.is_none()
                || session.principal_provenance
                    != Some(ServicePrincipalProvenance::RegisteredCapability)
        })
        .collect::<Vec<_>>();
    let stale_same = sessions.iter().copied().any(|session| {
        session.principal_id.as_deref() == Some(binding.principal_id.as_str())
            && inactive_or_expired(session.lease, session.expires_at.as_deref(), now)
    });
    let mut blocking = Vec::new();
    if !owner_current {
        blocking.push("owner_generation_or_binding_mismatch".to_string());
    }
    if !capability_current {
        blocking.push("principal_capability_not_current".to_string());
    }
    if !foreign.is_empty() {
        blocking.push("foreign_principal_active_work".to_string());
    }
    if !unproven.is_empty() {
        blocking.push("unproven_session_authority".to_string());
    }
    let coherent = blocking.is_empty();
    let authority = authority_for_binding(state, binding);
    let rejoin_repairable = !blocking.is_empty()
        && blocking.iter().all(|axis| {
            matches!(
                axis.as_str(),
                "owner_generation_or_binding_mismatch" | "unproven_session_authority"
            )
        })
        && authority
            .as_ref()
            .is_some_and(|authority| exact_rejoin_target(state, authority, now).is_some());
    let (lease_state, recourse) = if !foreign.is_empty() {
        (
            "foreign_held",
            PrincipalContinuityRecourse::WaitForForeignPrincipal,
        )
    } else if rejoin_repairable {
        (
            "identity_reconciliation_required",
            PrincipalContinuityRecourse::RejoinOwnedBrowser,
        )
    } else if !coherent {
        (
            "identity_reconciliation_required",
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
        )
    } else if !same.is_empty() {
        ("active", PrincipalContinuityRecourse::RejoinOwnedBrowser)
    } else if stale_same {
        (
            "stale",
            PrincipalContinuityRecourse::ReplaceStaleSamePrincipalSession,
        )
    } else {
        (
            "owned_idle",
            PrincipalContinuityRecourse::RejoinOwnedBrowser,
        )
    };
    let mut actions = READ_ACTIONS
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if coherent {
        actions.push("rejoin".to_string());
        if !same.is_empty() {
            actions.push("renew".to_string());
        }
        let has_active_tabs = authority.as_ref().is_some_and(|authority| {
            active_subordinate_tabs(state, authority, now)
                .next()
                .is_some()
        });
        if !same.is_empty() && !has_active_tabs {
            actions.push("release".to_string());
        }
    } else if rejoin_repairable {
        actions.push("rejoin".to_string());
    }
    if stale_same || !blocking.is_empty() {
        actions.push("reconcile_plan".to_string());
    }
    actions.sort();
    actions.dedup();
    let session_ids = sessions
        .iter()
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    let tab_ids = tabs_for_sessions(state, &session_ids);
    let route_ids = owner
        .map(|owner| routes_for_browser(state, &owner.browser_id))
        .unwrap_or_default();
    let mut record = ProfileLeaseRecord {
        schema_version: PROFILE_LEASE_SCHEMA_VERSION.to_string(),
        id: profile_lease_id(&binding.principal_id, &binding.profile_identity_digest),
        lease_revision: String::new(),
        principal_id: Some(binding.principal_id.clone()),
        principal_provenance: Some(binding.provenance),
        profile_id: binding.profile_id.clone(),
        profile_identity_digest: Some(binding.profile_identity_digest.clone()),
        browser_id: owner.map(|owner| owner.browser_id.clone()),
        session_ids,
        tab_ids,
        mode: lease_mode(&active),
        state: lease_state.to_string(),
        owner_generation: owner.map(|owner| owner.owner_generation),
        process_instance_digest: owner.map(|owner| owner.process_instance_digest.clone()),
        route_ids,
        last_heartbeat_at: latest_value(
            sessions
                .iter()
                .filter_map(|session| session.last_lease_observed_at.as_deref()),
        ),
        expires_at: earliest_value(
            same.iter()
                .filter_map(|session| session.expires_at.as_deref()),
        ),
        cleanup_obligation: owner
            .and_then(|owner| cleanup_obligation_for_browser(state, &owner.browser_id)),
        blocking_identity_axes: blocking,
        authorized_actions: actions,
        recourse,
        observation_only: !coherent,
    };
    record.lease_revision = lease_revision(&record);
    record
}

fn unbound_capability_profile_lease(
    state: &ServiceState,
    capability: &super::service_principal::ServiceProfileCapability,
    now: &str,
) -> ProfileLeaseRecord {
    let profile_identity_digest = state
        .profiles
        .get(&capability.profile_id)
        .and_then(|profile| profile.user_data_dir.as_deref())
        .and_then(|path| {
            crate::runtime_profile::canonical_profile_identity_digest(Path::new(path)).ok()
        });
    let owner = profile_identity_digest
        .as_deref()
        .and_then(|digest| state.runtime_owner_registry.owners.get(digest));
    let sessions = sessions_for_profile(state, &capability.profile_id);
    let session_ids = sessions
        .iter()
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    let active = sessions
        .iter()
        .copied()
        .filter(|session| !inactive_or_expired(session.lease, session.expires_at.as_deref(), now))
        .collect::<Vec<_>>();
    let authority = AuthenticatedServicePrincipal {
        principal_id: capability.principal_id.clone(),
        profile_id: capability.profile_id.clone(),
        capability_id: capability.capability_id.clone(),
        capability_revision: capability.revision,
        provenance: ServicePrincipalProvenance::RegisteredCapability,
    };
    let rejoin_repairable =
        authenticated_authority_is_current(&state.service_principals, &authority)
            && profile_identity_digest.as_deref().is_some_and(|digest| {
                !state
                    .runtime_owner_registry
                    .principal_bindings
                    .contains_key(digest)
                    && exact_rejoin_target_for_owner(state, &authority, digest, now).is_some()
            });
    let mut authorized_actions = READ_ACTIONS
        .iter()
        .chain([&"profile_acquire", &"reconcile_plan"])
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if rejoin_repairable {
        authorized_actions.push("rejoin".to_string());
    }
    authorized_actions.sort();
    authorized_actions.dedup();
    let mut record = ProfileLeaseRecord {
        schema_version: PROFILE_LEASE_SCHEMA_VERSION.to_string(),
        id: profile_lease_id(
            &capability.principal_id,
            profile_identity_digest
                .as_deref()
                .unwrap_or(capability.profile_id.as_str()),
        ),
        lease_revision: String::new(),
        principal_id: Some(capability.principal_id.clone()),
        principal_provenance: Some(ServicePrincipalProvenance::RegisteredCapability),
        profile_id: capability.profile_id.clone(),
        profile_identity_digest,
        browser_id: owner.map(|owner| owner.browser_id.clone()),
        session_ids: session_ids.clone(),
        tab_ids: tabs_for_sessions(state, &session_ids),
        mode: lease_mode(&active),
        state: "identity_reconciliation_required".to_string(),
        owner_generation: owner.map(|owner| owner.owner_generation),
        process_instance_digest: owner.map(|owner| owner.process_instance_digest.clone()),
        route_ids: owner
            .map(|owner| routes_for_browser(state, &owner.browser_id))
            .unwrap_or_default(),
        last_heartbeat_at: latest_value(
            sessions
                .iter()
                .filter_map(|session| session.last_lease_observed_at.as_deref()),
        ),
        expires_at: earliest_value(
            active
                .iter()
                .filter_map(|session| session.expires_at.as_deref()),
        ),
        cleanup_obligation: owner
            .and_then(|owner| cleanup_obligation_for_browser(state, &owner.browser_id)),
        blocking_identity_axes: vec!["runtime_owner_principal_binding_missing".to_string()],
        authorized_actions,
        recourse: if rejoin_repairable {
            PrincipalContinuityRecourse::RejoinOwnedBrowser
        } else {
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity
        },
        observation_only: true,
    };
    record.lease_revision = lease_revision(&record);
    record
}

fn legacy_profile_lease(state: &ServiceState, profile_id: &str, now: &str) -> ProfileLeaseRecord {
    let sessions = sessions_for_profile(state, profile_id);
    let session_ids = sessions
        .iter()
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    let active = sessions
        .iter()
        .copied()
        .filter(|session| !inactive_or_expired(session.lease, session.expires_at.as_deref(), now))
        .collect::<Vec<_>>();
    let has_nonterminal_owner = state.runtime_owner_registry.owners.values().any(|owner| {
        owner.state != crate::runtime_owner_transfer::ProfileOwnerState::Failed
            && state
                .browsers
                .get(&owner.browser_id)
                .and_then(|browser| browser.profile_id.as_deref())
                == Some(profile_id)
    });
    let historical = active.is_empty() && !has_nonterminal_owner;
    let mut authorized_actions = READ_ACTIONS.map(ToString::to_string).to_vec();
    if !historical {
        authorized_actions.push("profile_acquire".to_string());
    }
    let mut record = ProfileLeaseRecord {
        schema_version: PROFILE_LEASE_SCHEMA_VERSION.to_string(),
        id: profile_lease_id("unproven-legacy", profile_id),
        lease_revision: String::new(),
        principal_id: None,
        principal_provenance: Some(ServicePrincipalProvenance::UnprovenLegacy),
        profile_id: profile_id.to_string(),
        profile_identity_digest: None,
        browser_id: None,
        session_ids: session_ids.clone(),
        tab_ids: tabs_for_sessions(state, &session_ids),
        mode: lease_mode(&active),
        state: if historical {
            "historical"
        } else {
            "identity_reconciliation_required"
        }
        .to_string(),
        owner_generation: None,
        process_instance_digest: None,
        route_ids: Vec::new(),
        last_heartbeat_at: latest_value(
            sessions
                .iter()
                .filter_map(|session| session.last_lease_observed_at.as_deref()),
        ),
        expires_at: earliest_value(
            active
                .iter()
                .filter_map(|session| session.expires_at.as_deref()),
        ),
        cleanup_obligation: None,
        blocking_identity_axes: if historical {
            Vec::new()
        } else {
            vec!["legacy_principal_unproven".to_string()]
        },
        authorized_actions,
        recourse: PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
        observation_only: true,
    };
    record.lease_revision = lease_revision(&record);
    record
}

fn authorized_lease(
    state: &ServiceState,
    lease_id: &str,
    expected_revision: &str,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
) -> Result<ProfileLeaseRecord, ProfileLeaseError> {
    if !authenticated_authority_is_current(&state.service_principals, authority) {
        return Err(lease_error(
            ProfileLeaseFailureCode::AuthorityMismatch,
            lease_id,
        ));
    }
    let lease = inspect_profile_lease(state, lease_id, now)?;
    if lease.lease_revision != expected_revision {
        return Err(lease_error(
            ProfileLeaseFailureCode::RevisionMismatch,
            lease_id,
        ));
    }
    if lease.principal_id.as_deref() != Some(authority.principal_id.as_str())
        || lease.profile_id != authority.profile_id
        || lease.principal_provenance != Some(authority.provenance)
    {
        return Err(lease_error(
            ProfileLeaseFailureCode::AuthorityMismatch,
            lease_id,
        ));
    }
    Ok(lease)
}

fn require_action(lease: &ProfileLeaseRecord, action: &str) -> Result<(), ProfileLeaseError> {
    if lease
        .authorized_actions
        .iter()
        .any(|candidate| candidate == action)
    {
        Ok(())
    } else {
        Err(lease_error(
            ProfileLeaseFailureCode::ActionNotAuthorized,
            &lease.id,
        ))
    }
}

fn authority_for_binding(
    state: &ServiceState,
    binding: &crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding,
) -> Option<AuthenticatedServicePrincipal> {
    let capability = state
        .service_principals
        .profile_capabilities
        .get(&binding.capability_id)?;
    Some(AuthenticatedServicePrincipal {
        principal_id: binding.principal_id.clone(),
        profile_id: binding.profile_id.clone(),
        capability_id: binding.capability_id.clone(),
        capability_revision: capability.revision,
        provenance: binding.provenance,
    })
    .filter(|authority| authenticated_authority_is_current(&state.service_principals, authority))
}

fn active_subordinate_tabs<'a>(
    state: &'a ServiceState,
    authority: &'a AuthenticatedServicePrincipal,
    now: &'a str,
) -> impl Iterator<Item = &'a super::service_model::BrowserTab> + 'a {
    state.tabs.values().filter(move |tab| {
        let session_matches_profile = tab
            .owner_session_id
            .as_deref()
            .and_then(|session_id| state.sessions.get(session_id))
            .is_some_and(|session| {
                session.profile_id.as_deref() == Some(authority.profile_id.as_str())
                    && session.principal_id.as_deref() == Some(authority.principal_id.as_str())
            });
        tab.principal_id.as_deref() == Some(authority.principal_id.as_str())
            && tab.principal_provenance == Some(authority.provenance)
            && session_matches_profile
            && !matches!(tab.lifecycle, TabLifecycle::Closed | TabLifecycle::Crashed)
            && tab
                .work_lease_expires_at
                .as_deref()
                .is_none_or(|expires_at| expires_at > now)
    })
}

fn exact_rejoin_target(
    state: &ServiceState,
    authority: &AuthenticatedServicePrincipal,
    now: &str,
) -> Option<(String, Vec<String>)> {
    if !authenticated_authority_is_current(&state.service_principals, authority) {
        return None;
    }
    let binding = state
        .runtime_owner_registry
        .principal_bindings
        .values()
        .find(|binding| {
            binding.principal_id == authority.principal_id
                && binding.profile_id == authority.profile_id
                && binding.capability_id == authority.capability_id
                && binding.provenance == authority.provenance
        })?;
    let owner = state
        .runtime_owner_registry
        .owners
        .get(&binding.profile_identity_digest)?;
    if owner.state != crate::runtime_owner_transfer::ProfileOwnerState::Ready
        || owner.owner_generation < binding.owner_generation
    {
        return None;
    }
    exact_rejoin_target_for_owner(state, authority, &binding.profile_identity_digest, now)
}

fn exact_rejoin_target_for_owner(
    state: &ServiceState,
    authority: &AuthenticatedServicePrincipal,
    profile_identity_digest: &str,
    now: &str,
) -> Option<(String, Vec<String>)> {
    if !authenticated_authority_is_current(&state.service_principals, authority) {
        return None;
    }
    let owner = state
        .runtime_owner_registry
        .owners
        .get(profile_identity_digest)?;
    if owner.state != crate::runtime_owner_transfer::ProfileOwnerState::Ready {
        return None;
    }
    let active_sessions = state
        .sessions
        .values()
        .filter(|session| {
            session.profile_id.as_deref() == Some(authority.profile_id.as_str())
                && !inactive_or_expired(session.lease, session.expires_at.as_deref(), now)
        })
        .collect::<Vec<_>>();
    let [session] = active_sessions.as_slice() else {
        return None;
    };
    let exact_owner_session = session.id == owner.daemon_session_route
        || session
            .browser_ids
            .iter()
            .any(|browser_id| browser_id == &owner.browser_id);
    let authority_is_uncontested = session
        .principal_id
        .as_deref()
        .is_none_or(|principal_id| principal_id == authority.principal_id);
    if !exact_owner_session || !authority_is_uncontested {
        return None;
    }
    let active_tabs = state
        .tabs
        .values()
        .filter(|tab| {
            tab.owner_session_id.as_deref() == Some(session.id.as_str())
                && !matches!(tab.lifecycle, TabLifecycle::Closed | TabLifecycle::Crashed)
        })
        .collect::<Vec<_>>();
    if active_tabs.iter().any(|tab| {
        tab.browser_id != owner.browser_id
            || tab
                .principal_id
                .as_deref()
                .is_some_and(|principal_id| principal_id != authority.principal_id)
    }) {
        return None;
    }
    let mut tab_ids = active_tabs
        .into_iter()
        .map(|tab| tab.id.clone())
        .collect::<Vec<_>>();
    tab_ids.sort();
    Some((session.id.clone(), tab_ids))
}

fn bind_principal_to_current_owner(
    state: &mut ServiceState,
    authority: &AuthenticatedServicePrincipal,
    lease: &ProfileLeaseRecord,
) -> Result<(), String> {
    let profile_identity_digest = lease
        .profile_identity_digest
        .as_deref()
        .ok_or_else(|| "profile_identity_digest_missing".to_string())?;
    let owner_generation = state
        .runtime_owner_registry
        .owners
        .get(profile_identity_digest)
        .filter(|owner| owner.state == crate::runtime_owner_transfer::ProfileOwnerState::Ready)
        .map(|owner| owner.owner_generation)
        .ok_or_else(|| "profile_owner_missing".to_string())?;
    state
        .runtime_owner_registry
        .bind_principal_authority(
            crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                principal_id: authority.principal_id.clone(),
                profile_id: authority.profile_id.clone(),
                profile_identity_digest: profile_identity_digest.to_string(),
                capability_id: authority.capability_id.clone(),
                provenance: authority.provenance,
                owner_generation,
            },
        )
        .map_err(|error| format!("profile_owner_binding_failed:{error:?}"))?;
    Ok(())
}

fn refresh_principal_binding_to_current_owner(
    state: &mut ServiceState,
    authority: &AuthenticatedServicePrincipal,
) -> Result<(), String> {
    let existing = state
        .runtime_owner_registry
        .principal_bindings
        .values()
        .find(|binding| {
            binding.principal_id == authority.principal_id
                && binding.profile_id == authority.profile_id
                && binding.capability_id == authority.capability_id
                && binding.provenance == authority.provenance
        })
        .cloned()
        .ok_or_else(|| "profile_owner_binding_missing".to_string())?;
    if state
        .runtime_owner_registry
        .principal_binding_is_current(Some(&existing))
    {
        return Ok(());
    }
    let owner_generation = state
        .runtime_owner_registry
        .owners
        .get(&existing.profile_identity_digest)
        .map(|owner| owner.owner_generation)
        .ok_or_else(|| "profile_owner_missing".to_string())?;
    state
        .runtime_owner_registry
        .refresh_principal_authority(
            crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                owner_generation,
                ..existing
            },
        )
        .map_err(|error| format!("profile_owner_binding_refresh_failed:{error:?}"))?;
    Ok(())
}

fn sessions_for_profile<'a>(
    state: &'a ServiceState,
    profile_id: &str,
) -> Vec<&'a super::service_model::BrowserSession> {
    let mut sessions = state
        .sessions
        .values()
        .filter(|session| session.profile_id.as_deref() == Some(profile_id))
        .collect::<Vec<_>>();
    sessions.sort_by(|left, right| left.id.cmp(&right.id));
    sessions
}

fn tabs_for_sessions(state: &ServiceState, session_ids: &[String]) -> Vec<String> {
    let session_ids = session_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut tab_ids = state
        .tabs
        .values()
        .filter(|tab| {
            tab.owner_session_id
                .as_deref()
                .is_some_and(|id| session_ids.contains(id))
        })
        .map(|tab| tab.id.clone())
        .collect::<Vec<_>>();
    tab_ids.sort();
    tab_ids
}

fn routes_for_browser(state: &ServiceState, browser_id: &str) -> Vec<String> {
    let mut route_ids = state
        .remote_view_routes
        .values()
        .filter(|route| route.browser_id.as_deref() == Some(browser_id))
        .map(|route| route.id.clone())
        .collect::<Vec<_>>();
    route_ids.sort();
    route_ids
}

fn cleanup_obligation_for_browser(state: &ServiceState, browser_id: &str) -> Option<String> {
    state
        .runtime_owner_registry
        .lifecycle_records
        .get(browser_id)
        .and_then(|record| {
            serde_json::to_value(record.cleanup_obligation_state)
                .ok()
                .and_then(|value| value.as_str().map(ToString::to_string))
        })
}

fn lease_mode(sessions: &[&super::service_model::BrowserSession]) -> String {
    if sessions
        .iter()
        .any(|session| session.lease == LeaseState::HumanTakeover)
    {
        "human_takeover".to_string()
    } else if sessions
        .iter()
        .any(|session| session.lease == LeaseState::Exclusive)
    {
        "exclusive".to_string()
    } else if sessions.is_empty() {
        "idle".to_string()
    } else {
        "shared".to_string()
    }
}

fn inactive_session(lease: LeaseState) -> bool {
    matches!(lease, LeaseState::Released | LeaseState::Expired)
}

fn inactive_or_expired(lease: LeaseState, expires_at: Option<&str>, now: &str) -> bool {
    inactive_session(lease)
        || expires_at
            .is_some_and(|expires_at| rfc3339_at_or_after(now, expires_at).unwrap_or(false))
}

fn rfc3339_at_or_after(now: &str, expires_at: &str) -> Result<bool, ()> {
    let now = chrono::DateTime::parse_from_rfc3339(now).map_err(|_| ())?;
    let expires_at = chrono::DateTime::parse_from_rfc3339(expires_at).map_err(|_| ())?;
    Ok(now >= expires_at)
}

fn lease_state_name(lease: LeaseState) -> &'static str {
    match lease {
        LeaseState::Shared => "shared",
        LeaseState::Exclusive => "exclusive",
        LeaseState::HumanTakeover => "human_takeover",
        LeaseState::Released => "released",
        LeaseState::Expired => "expired",
    }
}

fn latest_value<'a>(values: impl Iterator<Item = &'a str>) -> Option<String> {
    values.max().map(ToString::to_string)
}

fn earliest_value<'a>(values: impl Iterator<Item = &'a str>) -> Option<String> {
    values.min().map(ToString::to_string)
}

fn profile_lease_id(principal_id: &str, profile_identity: &str) -> String {
    format!(
        "profile-lease-v1:{}",
        digest_prefix(format!("{principal_id}\0{profile_identity}").as_bytes())
    )
}

fn lease_revision(record: &ProfileLeaseRecord) -> String {
    let mut projection = serde_json::to_value(record).expect("profile lease record must serialize");
    projection["leaseRevision"] = json!("");
    format!(
        "profile-lease-revision-v1:{}",
        digest_prefix(
            serde_json::to_vec(&projection)
                .expect("profile lease projection must encode")
                .as_slice()
        )
    )
}

fn seal_reconcile_plan(plan: &ProfileLeaseReconcilePlan, seal_key: &[u8]) -> String {
    let mut projection = serde_json::to_value(plan).expect("reconcile plan must serialize");
    projection["seal"] = json!("");
    let encoded = serde_json::to_vec(&projection).expect("reconcile plan must encode");
    let mut hasher = Sha256::new();
    hasher.update(b"agent-browser.profile-lease-reconcile-seal.v1\0");
    hasher.update(seal_key);
    hasher.update(b"\0");
    hasher.update(encoded);
    hasher.update(b"\0");
    hasher.update(seal_key);
    format!("sha256:{:x}", hasher.finalize())
}

fn digest_prefix(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))[..24].to_string()
}

fn lease_error(code: ProfileLeaseFailureCode, lease_id: &str) -> ProfileLeaseError {
    let label = match code {
        ProfileLeaseFailureCode::LeaseNotFound => "profile lease was not found",
        ProfileLeaseFailureCode::AuthorityMismatch => "profile lease authority does not match",
        ProfileLeaseFailureCode::RevisionMismatch => "profile lease revision does not match",
        ProfileLeaseFailureCode::ActionNotAuthorized => "profile lease action is not authorized",
        ProfileLeaseFailureCode::ActiveSubordinateWork => {
            "profile lease has active subordinate work"
        }
        ProfileLeaseFailureCode::PlanInvalid => "profile lease reconcile plan is invalid",
        ProfileLeaseFailureCode::PlanExpired => "profile lease reconcile plan is expired",
        ProfileLeaseFailureCode::BootEpochMismatch => {
            "profile lease reconcile boot epoch does not match"
        }
    };
    ProfileLeaseError {
        code,
        message: format!("{label}: {lease_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{BrowserProfile, BrowserSession, BrowserTab};
    use crate::native::service_principal::{
        authenticate_profile_capability, bind_session_work_lease, bind_tab_work_lease,
        register_profile_capability, ServicePrincipalRegistrationRequest,
    };
    use crate::runtime_owner_transfer::{
        ProfileOwner, ProfileOwnerState, RuntimeOwnerPrincipalBinding, RuntimeOwnerRegistry,
    };
    use crate::test_utils::EnvGuard;
    use std::collections::BTreeMap;

    const CAPABILITY: &str = "profile-lease-test-capability-with-more-than-thirty-two-characters";
    const SEAL_KEY: &[u8] = b"profile-lease-test-seal-key-more-than-thirty-two-bytes";
    const NOW: &str = "2026-08-27T12:00:00Z";

    fn state_with_lease() -> (ServiceState, AuthenticatedServicePrincipal, String) {
        let profile_id = "odollo-fulfillment";
        let principal_id = "principal:odollo-fulfillment";
        let profile_path = "/tmp/agent-browser-p134/odollo-fulfillment";
        let digest = crate::runtime_profile::canonical_profile_identity_digest(
            std::path::Path::new(profile_path),
        )
        .unwrap();
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                profile_id.to_string(),
                BrowserProfile {
                    id: profile_id.to_string(),
                    user_data_dir: Some(profile_path.to_string()),
                    ..BrowserProfile::default()
                },
            )]),
            runtime_owner_registry: RuntimeOwnerRegistry::from_owner(ProfileOwner {
                owner_id: "owner-odollo".to_string(),
                profile_identity_digest: digest.clone(),
                state: ProfileOwnerState::Ready,
                owner_generation: 9,
                browser_id: "browser-odollo".to_string(),
                daemon_session_route: "session-odollo".to_string(),
                process_instance_digest: "process-odollo".to_string(),
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "cdp-odollo".to_string(),
                target_set_digest: "targets-odollo".to_string(),
                pending_transfer: None,
                last_transition: None,
            }),
            ..ServiceState::default()
        };
        let registered = register_profile_capability(
            &mut state.service_principals,
            ServicePrincipalRegistrationRequest {
                principal_id: principal_id.to_string(),
                display_name: Some("Odollo fulfillment".to_string()),
                profile_id: profile_id.to_string(),
                registered_at: Some(NOW.to_string()),
                registered_by: Some("test".to_string()),
            },
            CAPABILITY,
        )
        .unwrap();
        state
            .runtime_owner_registry
            .bind_principal_authority(RuntimeOwnerPrincipalBinding {
                principal_id: principal_id.to_string(),
                profile_id: profile_id.to_string(),
                profile_identity_digest: digest,
                capability_id: registered.capability.capability_id,
                provenance: ServicePrincipalProvenance::RegisteredCapability,
                owner_generation: 9,
            })
            .unwrap();
        let authority = authenticate_profile_capability(
            &state.service_principals,
            CAPABILITY,
            Some(profile_id),
        )
        .unwrap();
        state.sessions.insert(
            "session-odollo".to_string(),
            BrowserSession {
                id: "session-odollo".to_string(),
                profile_id: Some(profile_id.to_string()),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );
        bind_session_work_lease(
            &mut state,
            "session-odollo",
            &authority,
            "2099-08-27T13:00:00Z".to_string(),
        )
        .unwrap();
        let lease_id = profile_leases_for_state(&state, NOW)[0].id.clone();
        (state, authority, lease_id)
    }

    fn temp_service_home(label: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "agent-browser-profile-lease-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&home).unwrap();
        home
    }

    fn save_default_state(state: ServiceState) {
        let repository = LockedServiceStateRepository::default_json().unwrap();
        repository
            .mutate(|current| {
                *current = state;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn projection_exposes_exact_owner_and_authorized_recourse() {
        let (state, _, lease_id) = state_with_lease();
        let lease = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        assert_eq!(lease.state, "active");
        assert_eq!(lease.owner_generation, Some(9));
        assert_eq!(
            lease.recourse,
            PrincipalContinuityRecourse::RejoinOwnedBrowser
        );
        assert!(lease.authorized_actions.contains(&"rejoin".to_string()));
        assert!(lease.authorized_actions.contains(&"renew".to_string()));
        assert!(lease.authorized_actions.contains(&"release".to_string()));
        assert!(!lease.observation_only);

        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-profile-lease-record.v1.schema.json"
        ))
        .unwrap();
        let value = serde_json::to_value(&lease).unwrap();
        for field in schema["required"].as_array().unwrap() {
            assert!(
                value.get(field.as_str().unwrap()).is_some(),
                "profile lease record omitted required field {field}"
            );
        }
        assert_eq!(
            value.as_object().unwrap().len(),
            schema["required"].as_array().unwrap().len()
        );
    }

    #[tokio::test]
    async fn registration_writes_private_capability_and_persists_only_its_digest() {
        let home = temp_service_home("register");
        let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
        let (mut state, _, _) = state_with_lease();
        state.service_principals = Default::default();
        state.runtime_owner_registry.principal_bindings.clear();
        let session = state.sessions.get_mut("session-odollo").unwrap();
        session.principal_id = None;
        session.principal_provenance = None;
        session.work_lease_id = None;
        session.expires_at = None;
        save_default_state(state);

        let capability_path = home.join("odollo.cap");
        let response = handle_service_profile_lease_command(&json!({
            "action": "service_profile_lease_register",
            "principalId": "principal:odollo-fulfillment",
            "profileId": "odollo-fulfillment",
            "displayName": "Odollo fulfillment",
            "registeredBy": "test-operator",
            "capabilityOut": capability_path,
        }))
        .await
        .unwrap();

        assert_eq!(response["boundToCurrentOwner"], true);
        assert_eq!(response["capabilityWritten"], true);
        let raw_capability = fs::read_to_string(&capability_path).unwrap();
        assert!(raw_capability.starts_with("abpc_v1_"));
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&capability_path).unwrap().permissions().mode() & 0o077,
            0
        );
        let state_path = super::super::service_store::default_service_state_path().unwrap();
        let persisted_raw = fs::read_to_string(&state_path).unwrap();
        assert!(!persisted_raw.contains(raw_capability.trim()));
        let persisted = LockedServiceStateRepository::default_json()
            .unwrap()
            .load_snapshot()
            .unwrap();
        assert_eq!(persisted.service_principals.profile_capabilities.len(), 1);
        assert_eq!(persisted.runtime_owner_registry.principal_bindings.len(), 1);
        let blocked = profile_leases_for_state(&persisted, NOW)
            .into_iter()
            .find(|lease| lease.profile_id == "odollo-fulfillment")
            .unwrap();
        assert!(blocked.observation_only);
        assert!(blocked.authorized_actions.contains(&"rejoin".to_string()));
        assert!(persisted.events.iter().any(|event| {
            event.kind == ServiceEventKind::ProfileLeaseLifecycleChanged
                && event
                    .details
                    .as_ref()
                    .and_then(|details| details["operation"].as_str())
                    == Some("register")
        }));

        let rejoined = handle_service_profile_lease_command(&json!({
            "action": "service_profile_lease_rejoin",
            "leaseId": blocked.id,
            "leaseRevision": blocked.lease_revision,
            "profileCapabilityFile": capability_path,
            "expiresAt": "2100-08-27T13:00:00Z",
        }))
        .await
        .unwrap();
        assert_eq!(rejoined["operation"], "rejoin");
        assert_eq!(rejoined["lease"]["observationOnly"], false);
        let persisted = LockedServiceStateRepository::default_json()
            .unwrap()
            .load_snapshot()
            .unwrap();
        assert_eq!(
            persisted.sessions["session-odollo"].principal_id.as_deref(),
            Some("principal:odollo-fulfillment")
        );

        drop(guard);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn pre_registered_capability_can_rejoin_unique_owner_after_launch() {
        let (mut state, authority, _) = state_with_lease();
        state.runtime_owner_registry.principal_bindings.clear();
        let session = state.sessions.get_mut("session-odollo").unwrap();
        session.principal_id = None;
        session.principal_provenance = None;
        session.work_lease_id = None;
        session.work_lease_revision = 0;
        session.expires_at = None;

        let blocked = profile_leases_for_state(&state, NOW)
            .into_iter()
            .find(|lease| lease.principal_id.as_deref() == Some(authority.principal_id.as_str()))
            .unwrap();
        assert!(blocked.observation_only);
        assert_eq!(
            blocked.blocking_identity_axes,
            vec!["runtime_owner_principal_binding_missing"]
        );
        assert!(blocked.authorized_actions.contains(&"rejoin".to_string()));
        assert_eq!(
            blocked.recourse,
            PrincipalContinuityRecourse::RejoinOwnedBrowser
        );

        let rejoined = rejoin_profile_lease(
            &mut state,
            &blocked.id,
            &blocked.lease_revision,
            &authority,
            NOW,
            Some("2026-08-27T13:00:00Z"),
        )
        .unwrap();

        assert!(!rejoined.observation_only);
        assert_eq!(rejoined.state, "active");
        assert_eq!(
            state.sessions["session-odollo"].principal_id.as_deref(),
            Some("principal:odollo-fulfillment")
        );
        assert_eq!(state.runtime_owner_registry.principal_bindings.len(), 1);
        let binding = state
            .runtime_owner_registry
            .principal_bindings
            .values()
            .next()
            .unwrap();
        assert_eq!(binding.principal_id, authority.principal_id);
        assert_eq!(binding.capability_id, authority.capability_id);
        assert_eq!(binding.owner_generation, 9);
    }

    #[test]
    fn pre_registered_capability_cannot_rejoin_ambiguous_profile_work() {
        let (mut state, authority, _) = state_with_lease();
        state.runtime_owner_registry.principal_bindings.clear();
        let session = state.sessions.get_mut("session-odollo").unwrap();
        session.principal_id = None;
        session.principal_provenance = None;
        session.work_lease_id = None;
        session.work_lease_revision = 0;
        session.expires_at = None;
        state.sessions.insert(
            "session-odollo-foreign".to_string(),
            BrowserSession {
                id: "session-odollo-foreign".to_string(),
                profile_id: Some("odollo-fulfillment".to_string()),
                lease: LeaseState::Exclusive,
                browser_ids: vec!["browser-odollo-foreign".to_string()],
                ..BrowserSession::default()
            },
        );

        let blocked = profile_leases_for_state(&state, NOW)
            .into_iter()
            .find(|lease| lease.principal_id.as_deref() == Some(authority.principal_id.as_str()))
            .unwrap();

        assert!(blocked.observation_only);
        assert!(!blocked.authorized_actions.contains(&"rejoin".to_string()));
        assert_eq!(
            blocked.recourse,
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity
        );
        let error = rejoin_profile_lease(
            &mut state,
            &blocked.id,
            &blocked.lease_revision,
            &authority,
            NOW,
            Some("2026-08-27T13:00:00Z"),
        )
        .unwrap_err();
        assert_eq!(error.code, ProfileLeaseFailureCode::ActionNotAuthorized);
        assert!(state.runtime_owner_registry.principal_bindings.is_empty());
        assert!(state.sessions["session-odollo"].principal_id.is_none());
        assert!(state.sessions["session-odollo-foreign"]
            .principal_id
            .is_none());
    }

    #[tokio::test]
    async fn capability_file_mutation_is_revision_guarded_and_never_persisted() {
        let home = temp_service_home("mutate");
        let guard = EnvGuard::new(&["HOME", "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME"]);
        guard.set("HOME", home.to_str().unwrap());
        guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
        let (state, _, lease_id) = state_with_lease();
        let lease_revision = inspect_profile_lease(&state, &lease_id, NOW)
            .unwrap()
            .lease_revision;
        save_default_state(state);
        let capability_path = home.join("odollo.cap");
        write_private_capability_file(&capability_path, CAPABILITY).unwrap();

        let response = handle_service_profile_lease_command(&json!({
            "action": "service_profile_lease_renew",
            "leaseId": lease_id,
            "leaseRevision": lease_revision,
            "profileCapabilityFile": capability_path,
            "expiresAt": "2100-08-27T14:00:00Z",
            "serviceName": "OdolloFulfillment",
        }))
        .await
        .unwrap();
        assert_eq!(response["operation"], "renew");
        assert_ne!(response["leaseRevision"], response["previousLeaseRevision"]);

        let state_path = super::super::service_store::default_service_state_path().unwrap();
        let persisted_raw = fs::read_to_string(&state_path).unwrap();
        assert!(!persisted_raw.contains(CAPABILITY));
        let persisted = LockedServiceStateRepository::default_json()
            .unwrap()
            .load_snapshot()
            .unwrap();
        let event = persisted
            .events
            .iter()
            .find(|event| event.kind == ServiceEventKind::ProfileLeaseLifecycleChanged)
            .unwrap();
        assert_eq!(event.details.as_ref().unwrap()["operation"], "renew");
        assert!(!serde_json::to_string(event).unwrap().contains(CAPABILITY));

        drop(guard);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn legacy_profile_is_observation_only_and_doctor_reports_it() {
        let state = ServiceState {
            sessions: BTreeMap::from([(
                "legacy".to_string(),
                BrowserSession {
                    id: "legacy".to_string(),
                    profile_id: Some("legacy-profile".to_string()),
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };
        let leases = profile_leases_for_state(&state, NOW);
        assert_eq!(leases.len(), 1);
        assert!(leases[0].observation_only);
        assert!(!leases[0]
            .authorized_actions
            .contains(&"release".to_string()));
        assert!(leases[0]
            .authorized_actions
            .contains(&"profile_acquire".to_string()));
        assert!(!leases[0]
            .authorized_actions
            .contains(&"reconcile_plan".to_string()));
        let doctor = doctor_profile_leases(&state, NOW);
        assert!(!doctor.healthy);
        assert_eq!(doctor.findings[0].code, "legacy_principal_unproven");
    }

    #[test]
    fn historical_legacy_profile_remains_visible_without_operational_blockers() {
        let state = ServiceState {
            sessions: BTreeMap::from([
                (
                    "released-legacy".to_string(),
                    BrowserSession {
                        id: "released-legacy".to_string(),
                        profile_id: Some("historical-profile".to_string()),
                        lease: LeaseState::Released,
                        ..BrowserSession::default()
                    },
                ),
                (
                    "expired-legacy".to_string(),
                    BrowserSession {
                        id: "expired-legacy".to_string(),
                        profile_id: Some("historical-profile".to_string()),
                        lease: LeaseState::Expired,
                        ..BrowserSession::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        };

        let leases = profile_leases_for_state(&state, NOW);
        assert_eq!(leases.len(), 1);
        assert_eq!(leases[0].state, "historical");
        assert_eq!(leases[0].mode, "idle");
        assert_eq!(leases[0].session_ids, ["expired-legacy", "released-legacy"]);
        assert!(leases[0].observation_only);
        assert!(leases[0].blocking_identity_axes.is_empty());
        assert_eq!(
            leases[0].authorized_actions,
            READ_ACTIONS.map(ToString::to_string)
        );

        let doctor = doctor_profile_leases(&state, NOW);
        assert!(doctor.healthy);
        assert!(doctor.findings.is_empty());
    }

    #[test]
    fn renew_uses_revision_cas_and_release_refuses_active_tab() {
        let (mut state, authority, lease_id) = state_with_lease();
        let initial = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        let renewed = renew_profile_lease(
            &mut state,
            &lease_id,
            &initial.lease_revision,
            &authority,
            NOW,
            "2026-08-27T14:00:00Z",
        )
        .unwrap();
        assert_ne!(renewed.lease_revision, initial.lease_revision);
        let stale = renew_profile_lease(
            &mut state,
            &lease_id,
            &initial.lease_revision,
            &authority,
            NOW,
            "2026-08-27T15:00:00Z",
        )
        .unwrap_err();
        assert_eq!(stale.code, ProfileLeaseFailureCode::RevisionMismatch);

        state.tabs.insert(
            "tab-odollo".to_string(),
            BrowserTab {
                id: "tab-odollo".to_string(),
                browser_id: "browser-odollo".to_string(),
                owner_session_id: Some("session-odollo".to_string()),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );
        bind_tab_work_lease(
            &mut state,
            "tab-odollo",
            &authority,
            "2026-08-27T13:30:00Z".to_string(),
        )
        .unwrap();
        let with_tab = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        let error = release_profile_lease(
            &mut state,
            &lease_id,
            &with_tab.lease_revision,
            &authority,
            NOW,
        )
        .unwrap_err();
        assert_eq!(error.code, ProfileLeaseFailureCode::ActionNotAuthorized);
    }

    #[test]
    fn rejoin_claims_exact_unproven_owner_session_and_tabs() {
        let (mut state, authority, lease_id) = state_with_lease();
        let session = state.sessions.get_mut("session-odollo").unwrap();
        session.principal_id = None;
        session.principal_provenance = None;
        session.work_lease_id = None;
        session.expires_at = None;
        session.browser_ids = vec!["browser-odollo".to_string()];
        state.tabs.insert(
            "tab-odollo".to_string(),
            BrowserTab {
                id: "tab-odollo".to_string(),
                browser_id: "browser-odollo".to_string(),
                owner_session_id: Some("session-odollo".to_string()),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );

        let blocked = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        assert!(blocked.observation_only);
        assert_eq!(
            blocked.blocking_identity_axes,
            vec!["unproven_session_authority"]
        );
        assert!(blocked.authorized_actions.contains(&"rejoin".to_string()));

        let rejoined = rejoin_profile_lease(
            &mut state,
            &lease_id,
            &blocked.lease_revision,
            &authority,
            NOW,
            Some("2026-08-27T13:00:00Z"),
        )
        .unwrap();
        assert!(!rejoined.observation_only);
        assert_eq!(rejoined.state, "active");
        assert_eq!(
            state.sessions["session-odollo"].principal_id.as_deref(),
            Some("principal:odollo-fulfillment")
        );
        assert_eq!(
            state.tabs["tab-odollo"].principal_id.as_deref(),
            Some("principal:odollo-fulfillment")
        );
        assert_eq!(
            state.tabs["tab-odollo"].work_lease_expires_at.as_deref(),
            Some("2026-08-27T13:00:00Z")
        );
    }

    #[test]
    fn rejoin_refuses_ambiguous_unproven_profile_work() {
        let (mut state, authority, lease_id) = state_with_lease();
        let session = state.sessions.get_mut("session-odollo").unwrap();
        session.principal_id = None;
        session.principal_provenance = None;
        session.work_lease_id = None;
        session.expires_at = None;
        session.browser_ids = vec!["browser-odollo".to_string()];
        state.sessions.insert(
            "unrelated-same-profile".to_string(),
            BrowserSession {
                id: "unrelated-same-profile".to_string(),
                profile_id: Some("odollo-fulfillment".to_string()),
                lease: LeaseState::Exclusive,
                browser_ids: vec!["browser-unrelated".to_string()],
                ..BrowserSession::default()
            },
        );

        let blocked = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        assert!(!blocked.authorized_actions.contains(&"rejoin".to_string()));
        let error = rejoin_profile_lease(
            &mut state,
            &lease_id,
            &blocked.lease_revision,
            &authority,
            NOW,
            Some("2026-08-27T13:00:00Z"),
        )
        .unwrap_err();
        assert_eq!(error.code, ProfileLeaseFailureCode::ActionNotAuthorized);
        assert!(state.sessions["session-odollo"].principal_id.is_none());
        assert!(state.sessions["unrelated-same-profile"]
            .principal_id
            .is_none());
    }

    #[test]
    fn rejoin_refuses_foreign_tab_on_exact_owner_session() {
        let (mut state, authority, lease_id) = state_with_lease();
        let session = state.sessions.get_mut("session-odollo").unwrap();
        session.principal_id = None;
        session.principal_provenance = None;
        session.work_lease_id = None;
        session.expires_at = None;
        session.browser_ids = vec!["browser-odollo".to_string()];
        state.tabs.insert(
            "tab-foreign".to_string(),
            BrowserTab {
                id: "tab-foreign".to_string(),
                browser_id: "browser-odollo".to_string(),
                owner_session_id: Some("session-odollo".to_string()),
                principal_id: Some("principal:foreign".to_string()),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );

        let blocked = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        assert!(!blocked.authorized_actions.contains(&"rejoin".to_string()));
        let error = rejoin_profile_lease(
            &mut state,
            &lease_id,
            &blocked.lease_revision,
            &authority,
            NOW,
            Some("2026-08-27T13:00:00Z"),
        )
        .unwrap_err();
        assert_eq!(error.code, ProfileLeaseFailureCode::ActionNotAuthorized);
        assert!(state.sessions["session-odollo"].principal_id.is_none());
        assert_eq!(
            state.tabs["tab-foreign"].principal_id.as_deref(),
            Some("principal:foreign")
        );
    }

    #[test]
    fn rejoin_refreshes_same_capability_after_owner_generation_changes() {
        let (mut state, authority, lease_id) = state_with_lease();
        let profile_digest = state
            .runtime_owner_registry
            .principal_bindings
            .values()
            .next()
            .unwrap()
            .profile_identity_digest
            .clone();
        state.sessions.get_mut("session-odollo").unwrap().lease = LeaseState::Released;
        let owner = state
            .runtime_owner_registry
            .owners
            .get_mut(&profile_digest)
            .unwrap();
        owner.owner_generation = 10;
        owner.browser_id = "browser-odollo-replayed".to_string();
        owner.daemon_session_route = "session-odollo-replayed".to_string();
        owner.process_instance_digest = "process-odollo-replayed".to_string();
        state.sessions.insert(
            "session-odollo-replayed".to_string(),
            BrowserSession {
                id: "session-odollo-replayed".to_string(),
                profile_id: Some("odollo-fulfillment".to_string()),
                lease: LeaseState::Exclusive,
                browser_ids: vec!["browser-odollo-replayed".to_string()],
                ..BrowserSession::default()
            },
        );
        state.tabs.insert(
            "tab-odollo-replayed".to_string(),
            BrowserTab {
                id: "tab-odollo-replayed".to_string(),
                browser_id: "browser-odollo-replayed".to_string(),
                owner_session_id: Some("session-odollo-replayed".to_string()),
                lifecycle: TabLifecycle::Ready,
                ..BrowserTab::default()
            },
        );

        let blocked = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        assert_eq!(
            blocked.blocking_identity_axes,
            vec![
                "owner_generation_or_binding_mismatch",
                "unproven_session_authority"
            ]
        );
        assert!(blocked.authorized_actions.contains(&"rejoin".to_string()));
        assert_eq!(
            blocked.recourse,
            PrincipalContinuityRecourse::RejoinOwnedBrowser
        );

        let rejoined = rejoin_profile_lease(
            &mut state,
            &lease_id,
            &blocked.lease_revision,
            &authority,
            NOW,
            Some("2026-08-27T13:00:00Z"),
        )
        .unwrap();
        assert!(!rejoined.observation_only);
        assert_eq!(rejoined.owner_generation, Some(10));
        assert_eq!(
            state.runtime_owner_registry.principal_bindings[&profile_digest].owner_generation,
            10
        );
        assert_eq!(
            state.sessions["session-odollo-replayed"]
                .principal_id
                .as_deref(),
            Some("principal:odollo-fulfillment")
        );
        assert_eq!(
            state.tabs["tab-odollo-replayed"].principal_id.as_deref(),
            Some("principal:odollo-fulfillment")
        );
    }

    #[test]
    fn reconcile_refreshes_exact_stale_owner_generation_without_session_work() {
        let (mut state, authority, lease_id) = state_with_lease();
        let profile_digest = state
            .runtime_owner_registry
            .principal_bindings
            .values()
            .find(|binding| binding.principal_id == authority.principal_id)
            .unwrap()
            .profile_identity_digest
            .clone();
        let owner = state
            .runtime_owner_registry
            .owners
            .get_mut(&profile_digest)
            .unwrap();
        owner.owner_generation += 1;
        state.sessions.clear();
        state.tabs.clear();

        let blocked = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        assert_eq!(
            blocked.blocking_identity_axes,
            vec!["owner_generation_or_binding_mismatch"]
        );
        assert_eq!(
            blocked.recourse,
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity
        );

        let plan = plan_profile_lease_reconciliation(
            &state,
            &lease_id,
            &blocked.lease_revision,
            &authority,
            NOW,
            "2026-08-27T12:05:00Z",
            Some("boot-epoch-1".to_string()),
            "idempotency-owner-generation-refresh".to_string(),
            SEAL_KEY,
        )
        .unwrap();
        assert!(plan.effect_capable);
        assert_eq!(plan.proposed_transitions.len(), 1);
        assert_eq!(
            plan.proposed_transitions[0].action,
            "refresh_principal_owner_binding"
        );

        let receipt = apply_profile_lease_reconciliation(
            &mut state,
            &plan,
            &authority,
            "2026-08-27T12:01:00Z",
            Some("boot-epoch-1"),
            SEAL_KEY,
        )
        .unwrap();
        assert_eq!(receipt.transition_count, 1);
        let resolved = inspect_profile_lease(&state, &lease_id, "2026-08-27T12:01:00Z").unwrap();
        assert!(!resolved.observation_only);
        assert!(resolved.blocking_identity_axes.is_empty());
        assert_eq!(resolved.state, "owned_idle");
        assert_eq!(
            state.runtime_owner_registry.principal_bindings[&profile_digest].owner_generation,
            state.runtime_owner_registry.owners[&profile_digest].owner_generation
        );
    }

    #[test]
    fn reconcile_rejects_owner_binding_refresh_after_binding_changes() {
        let (mut state, authority, lease_id) = state_with_lease();
        let profile_digest = state
            .runtime_owner_registry
            .principal_bindings
            .values()
            .find(|binding| binding.principal_id == authority.principal_id)
            .unwrap()
            .profile_identity_digest
            .clone();
        state
            .runtime_owner_registry
            .owners
            .get_mut(&profile_digest)
            .unwrap()
            .owner_generation += 1;
        state.sessions.clear();
        state.tabs.clear();
        let blocked = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        let plan = plan_profile_lease_reconciliation(
            &state,
            &lease_id,
            &blocked.lease_revision,
            &authority,
            NOW,
            "2026-08-27T12:05:00Z",
            Some("boot-epoch-1".to_string()),
            "idempotency-stale-owner-generation-refresh".to_string(),
            SEAL_KEY,
        )
        .unwrap();

        refresh_principal_binding_to_current_owner(&mut state, &authority).unwrap();
        let error = apply_profile_lease_reconciliation(
            &mut state,
            &plan,
            &authority,
            "2026-08-27T12:01:00Z",
            Some("boot-epoch-1"),
            SEAL_KEY,
        )
        .unwrap_err();
        assert_eq!(error.code, ProfileLeaseFailureCode::RevisionMismatch);
        assert!(!state
            .profile_lease_reconcile_receipts
            .contains_key(&plan.idempotency_key));
    }

    #[test]
    fn reconcile_plan_is_boot_bound_sealed_and_idempotent() {
        let (mut state, authority, lease_id) = state_with_lease();
        let session = state.sessions.get_mut("session-odollo").unwrap();
        session.lease = LeaseState::Expired;
        session.expires_at = Some("2026-08-27T11:00:00Z".to_string());
        let stale = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        let blocked = plan_profile_lease_reconciliation(
            &state,
            &lease_id,
            &stale.lease_revision,
            &authority,
            NOW,
            "2026-08-27T12:05:00Z",
            None,
            "idempotency-blocked".to_string(),
            SEAL_KEY,
        )
        .unwrap();
        assert!(!blocked.effect_capable);
        assert_eq!(blocked.blocked_reasons, vec!["boot_epoch_unavailable"]);

        let plan = plan_profile_lease_reconciliation(
            &state,
            &lease_id,
            &stale.lease_revision,
            &authority,
            NOW,
            "2026-08-27T12:05:00Z",
            Some("boot-epoch-1".to_string()),
            "idempotency-1".to_string(),
            SEAL_KEY,
        )
        .unwrap();
        assert!(plan.effect_capable);
        let receipt = apply_profile_lease_reconciliation(
            &mut state,
            &plan,
            &authority,
            "2026-08-27T12:01:00Z",
            Some("boot-epoch-1"),
            SEAL_KEY,
        )
        .unwrap();
        assert!(!receipt.replayed);
        let replay = apply_profile_lease_reconciliation(
            &mut state,
            &plan,
            &authority,
            "2026-08-27T12:02:00Z",
            Some("boot-epoch-1"),
            SEAL_KEY,
        )
        .unwrap();
        assert!(replay.replayed);
    }

    #[test]
    fn reconcile_plan_compares_rfc3339_instants_across_offsets() {
        let (mut state, authority, lease_id) = state_with_lease();
        let session = state.sessions.get_mut("session-odollo").unwrap();
        session.lease = LeaseState::Expired;
        session.expires_at = Some("2026-08-27T11:00:00Z".to_string());
        let stale = inspect_profile_lease(&state, &lease_id, NOW).unwrap();
        let plan = plan_profile_lease_reconciliation(
            &state,
            &lease_id,
            &stale.lease_revision,
            &authority,
            "2026-08-27T07:00:00-05:00",
            "2026-08-27T12:05:00Z",
            Some("boot-epoch-1".to_string()),
            "idempotency-offset-expiry".to_string(),
            SEAL_KEY,
        )
        .unwrap();
        assert!(plan.effect_capable);

        let receipt = apply_profile_lease_reconciliation(
            &mut state,
            &plan,
            &authority,
            "2026-08-27T07:01:00-05:00",
            Some("boot-epoch-1"),
            SEAL_KEY,
        )
        .unwrap();
        assert_eq!(receipt.transition_count, 1);
    }
}
