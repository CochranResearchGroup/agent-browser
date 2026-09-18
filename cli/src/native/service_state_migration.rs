//! Versioned, staged migration for the primary Service State document.
//!
//! Legacy unversioned JSON is an input format only. It never grants principal
//! authority from service, agent, task, session, or profile labels. The
//! existing principal registry and exact runtime-owner bindings remain the
//! only migration inputs that can produce effect-capable lease authority.

use super::service_model::{
    BrowserProcess, BrowserProfile, BrowserSession, LeaseState, ServiceState,
};
#[cfg(test)]
use super::service_profile_access_policy::{ProfileAccessMode, ProfileIdentityAssurance};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) use agent_browser_service_model::{
    LEGACY_SERVICE_STATE_SCHEMA_VERSION, SERVICE_STATE_SCHEMA_VERSION,
};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ServiceStateMigrationStatus {
    NotRequired,
    Required,
    BlockedNewerSchema,
    BlockedInvariant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateMigrationPlan {
    pub(crate) schema_version: &'static str,
    pub(crate) source_state_schema: String,
    pub(crate) target_state_schema: &'static str,
    pub(crate) source_profile_lease_schema: Option<String>,
    pub(crate) target_profile_lease_schema: &'static str,
    pub(crate) status: ServiceStateMigrationStatus,
    pub(crate) forward_reader_available: bool,
    pub(crate) old_reader_compatible: bool,
    pub(crate) principal_authority_policy: &'static str,
    pub(crate) stable_identity_policy: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StagedServiceStateMigration {
    pub(crate) plan: ServiceStateMigrationPlan,
    pub(crate) bytes: Vec<u8>,
    pub(crate) summary: ServiceStateMigrationSummary,
    pub(crate) contamination_report: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateMigrationRecordClassSummary {
    pub(crate) before_count: usize,
    pub(crate) after_count: usize,
    pub(crate) added_ids: Vec<String>,
    pub(crate) removed_ids: Vec<String>,
    pub(crate) changed_ids: Vec<String>,
    pub(crate) preserved_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateMigrationSummary {
    pub(crate) record_classes: BTreeMap<String, ServiceStateMigrationRecordClassSummary>,
    pub(crate) affected_ids: Vec<String>,
    pub(crate) protected_record_removals: Vec<String>,
    pub(crate) unknown_top_level_fields_preserved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceStateMigrationPreview {
    pub(crate) schema_version: &'static str,
    pub(crate) mutation: bool,
    pub(crate) plan: ServiceStateMigrationPlan,
    pub(crate) summary: ServiceStateMigrationSummary,
    pub(crate) contamination_report: Value,
    pub(crate) recovery_artifact_compatibility: Vec<RecoveryArtifactCompatibility>,
    pub(crate) authoritative_state_path: String,
    pub(crate) backup_directory: String,
    pub(crate) backup_created: bool,
    pub(crate) restore_procedure: &'static str,
    pub(crate) receipt_directory: String,
    pub(crate) default_deletion_policy: &'static str,
    pub(crate) next_action: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryArtifactCompatibility {
    pub(crate) schema_version: &'static str,
    pub(crate) artifact_schema_version: String,
    pub(crate) artifact_kind: &'static str,
    pub(crate) reader_mode: &'static str,
    pub(crate) effect_capable: bool,
    pub(crate) preserved: bool,
    pub(crate) unknown_action_types: Vec<String>,
    pub(crate) next_action: &'static str,
}

pub(crate) fn read_service_state(raw: &str) -> Result<ServiceState, serde_json::Error> {
    agent_browser_service_model::decode_persisted_service_state_json(raw)
        .map_err(|error| custom_json_error(error.to_string()))
}

pub(crate) fn prepare_service_state_for_persistence(
    state: &mut ServiceState,
) -> Result<(), String> {
    agent_browser_service_model::prepare_service_state_for_persistence(state)
        .map_err(|error| error.to_string())
}

pub(crate) fn plan_service_state_migration(raw: &str) -> Result<ServiceStateMigrationPlan, String> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| format!("service_state_migration_json_invalid:{error}"))?;
    let source_state_schema = value
        .get("schemaVersion")
        .and_then(Value::as_str)
        .unwrap_or(LEGACY_SERVICE_STATE_SCHEMA_VERSION)
        .to_string();
    let source_profile_lease_schema = value
        .get("profileLeaseSchemaVersion")
        .and_then(Value::as_str)
        .map(str::to_string);
    let status = if source_state_schema == SERVICE_STATE_SCHEMA_VERSION {
        match source_profile_lease_schema.as_deref() {
            Some(schema)
                if schema == super::service_profile_lease::PROFILE_LEASE_SCHEMA_VERSION =>
            {
                ServiceStateMigrationStatus::NotRequired
            }
            None => ServiceStateMigrationStatus::Required,
            Some(_) => ServiceStateMigrationStatus::BlockedNewerSchema,
        }
    } else if source_state_schema == LEGACY_SERVICE_STATE_SCHEMA_VERSION {
        ServiceStateMigrationStatus::Required
    } else {
        ServiceStateMigrationStatus::BlockedNewerSchema
    };
    Ok(ServiceStateMigrationPlan {
        schema_version: "agent-browser.service-state-migration-plan.v1",
        source_state_schema,
        target_state_schema: SERVICE_STATE_SCHEMA_VERSION,
        source_profile_lease_schema,
        target_profile_lease_schema: super::service_profile_lease::PROFILE_LEASE_SCHEMA_VERSION,
        forward_reader_available: status != ServiceStateMigrationStatus::BlockedNewerSchema,
        old_reader_compatible: true,
        status,
        principal_authority_policy: "authenticated_capability_or_exact_current_owner_only",
        stable_identity_policy: "preserve_stable_ids_refresh_ephemeral_evidence",
    })
}

pub(crate) fn stage_service_state_migration(
    raw: &str,
) -> Result<StagedServiceStateMigration, String> {
    let source: Value = serde_json::from_str(raw)
        .map_err(|error| format!("service_state_migration_json_invalid:{error}"))?;
    let plan = plan_service_state_migration(raw)?;
    if !plan.forward_reader_available {
        return Err(format!(
            "service_state_migration_blocked_newer_schema:{}",
            plan.source_state_schema
        ));
    }
    let mut state = read_service_state(raw).map_err(|error| error.to_string())?;
    prepare_service_state_for_persistence(&mut state)?;
    discard_confirmed_dead_unreferenced_process_identities(&mut state);
    materialize_inert_owner_only_process_placeholders(&mut state);
    materialize_inert_legacy_profile_placeholders(&mut state);
    materialize_inert_legacy_remote_view_placeholders(&mut state);
    materialize_inert_legacy_browser_placeholders(&mut state);
    materialize_inert_legacy_session_placeholders(&mut state);
    // Full cross-projection integrity is an installation migration commit
    // gate. Ordinary runtime writes can update related projections in
    // multiple repository mutations and must remain able to converge them.
    validate_service_state_invariants(&state)?;
    let contamination_report = serde_json::to_value(
        super::service_browser_retirement::detect_browser_contamination(&state),
    )
    .map_err(|error| format!("service_state_migration_contamination_serialize_failed:{error}"))?;
    let mut migrated = serde_json::to_value(&state)
        .map_err(|error| format!("service_state_migration_serialize_failed:{error}"))?;
    let unknown_top_level_fields_preserved =
        preserve_unknown_successor_fields(&source, &mut migrated);
    let mut summary = summarize_service_state_migration(&source, &migrated);
    summary.unknown_top_level_fields_preserved = unknown_top_level_fields_preserved;
    let mut bytes = serde_json::to_vec_pretty(&migrated)
        .map_err(|error| format!("service_state_migration_serialize_failed:{error}"))?;
    bytes.push(b'\n');
    Ok(StagedServiceStateMigration {
        plan,
        bytes,
        summary,
        contamination_report,
    })
}

pub(crate) fn preview_service_state_migration(
    raw: &str,
    authoritative_state_path: &Path,
    artifact_directory: &Path,
) -> Result<ServiceStateMigrationPreview, String> {
    let source: Value = serde_json::from_str(raw)
        .map_err(|error| format!("service_state_migration_json_invalid:{error}"))?;
    let staged = stage_service_state_migration(raw)?;
    Ok(ServiceStateMigrationPreview {
        schema_version: "agent-browser.service-state-migration-preview.v1",
        mutation: false,
        plan: staged.plan,
        summary: staged.summary,
        contamination_report: staged.contamination_report,
        recovery_artifact_compatibility: recovery_artifact_compatibility_from_state(&source)?,
        authoritative_state_path: authoritative_state_path.display().to_string(),
        backup_directory: artifact_directory.display().to_string(),
        backup_created: false,
        restore_procedure: "Inspect the exact install transaction, then use its current revision, candidate generation, and census digest with install transactions rollback.",
        receipt_directory: artifact_directory.display().to_string(),
        default_deletion_policy: "preserve_browsers_profiles_leases_displays_routes_and_handoffs",
        next_action: "Review the contamination report and exact class diff before any apply.",
    })
}

fn recovery_artifact_compatibility_from_state(
    state: &Value,
) -> Result<Vec<RecoveryArtifactCompatibility>, String> {
    let mut results = Vec::new();
    for pointer in ["/profileRecoveryReceipts", "/browserRetirementReceipts"] {
        for artifact in state
            .pointer(pointer)
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(|records| records.values())
        {
            let raw = serde_json::to_string(artifact)
                .map_err(|error| format!("recovery_artifact_serialize_failed:{error}"))?;
            results.push(read_recovery_artifact_compatibility(&raw)?);
        }
    }
    Ok(results)
}

/// Read plan and receipt compatibility without granting effect authority.
/// Unknown successor fields and action types remain in caller-owned bytes;
/// only exact current schemas with known actions are eligible for typed apply.
pub(crate) fn read_recovery_artifact_compatibility(
    raw: &str,
) -> Result<RecoveryArtifactCompatibility, String> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| format!("recovery_artifact_json_invalid:{error}"))?;
    let schema = value
        .get("schemaVersion")
        .and_then(Value::as_str)
        .ok_or_else(|| "recovery_artifact_schema_required".to_string())?;
    let artifact_kind = if schema.contains("profile-recovery-plan") {
        "profile_recovery_plan"
    } else if schema.contains("profile-recovery-receipt") {
        "profile_recovery_receipt"
    } else if schema.contains("browser-retirement-plan") {
        "browser_retirement_plan"
    } else if schema.contains("browser-retirement-receipt") {
        "browser_retirement_receipt"
    } else {
        "unknown_recovery_artifact"
    };
    let known_current_schemas = [
        super::service_profile_acquisition::PROFILE_RECOVERY_PLAN_SCHEMA_V1,
        super::service_profile_acquisition::PROFILE_RECOVERY_RECEIPT_SCHEMA_V1,
        super::service_browser_retirement::BROWSER_RETIREMENT_PLAN_SCHEMA_V1,
        super::service_browser_retirement::BROWSER_RETIREMENT_RECEIPT_SCHEMA_V1,
    ];
    let known_actions = [
        "supersede_terminal_owner",
        "reconcile_exact_principal_profile_identity",
        "reconcile_legacy_principal",
        "bind_owner_principal_authority",
        "repair_owner_generation_binding",
        "release_expired_ownerless_lease",
        "adopt_exact_live_browser",
        "repair_subordinate_profile_binding",
        "repair_independent_route_identity",
        "finalize_terminal_installation_bookkeeping",
        "retire_inert_browser_record",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let unknown_action_types = value
        .get("actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|action| action.get("actionType").and_then(Value::as_str))
        .filter(|action| !known_actions.contains(*action))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let current = known_current_schemas.contains(&schema);
    let reader_mode = if current {
        "current"
    } else if schema.ends_with(".v0") || !schema.starts_with("agent-browser.") {
        "legacy_preserve_only"
    } else {
        "future_preserve_only"
    };
    let effect_capable =
        current && unknown_action_types.is_empty() && artifact_kind != "unknown_recovery_artifact";
    Ok(RecoveryArtifactCompatibility {
        schema_version: "agent-browser.recovery-artifact-compatibility.v1",
        artifact_schema_version: schema.to_string(),
        artifact_kind,
        reader_mode,
        effect_capable,
        preserved: true,
        unknown_action_types,
        next_action: if effect_capable {
            "Use the exact typed apply path with its normal compare-and-swap checks."
        } else {
            "Preserve the artifact unchanged and use a compatible reader before apply."
        },
    })
}

const SERVICE_STATE_COLLECTIONS: [(&str, &str, bool); 17] = [
    ("profiles", "/profiles", true),
    ("browsers", "/browsers", true),
    ("sessions", "/sessions", true),
    ("tabs", "/tabs", false),
    ("displayAllocations", "/displayAllocations", true),
    ("remoteViewRoutes", "/remoteViewRoutes", true),
    ("routePool", "/routePool", false),
    (
        "remoteViewAcquisitionLeases",
        "/remoteViewAcquisitionLeases",
        true,
    ),
    ("remoteViewHandoffs", "/remoteViewHandoffs", true),
    ("viewerLeases", "/viewerLeases", true),
    ("profileSeedingHandoffs", "/profileSeedingHandoffs", true),
    ("profileRecoveryReceipts", "/profileRecoveryReceipts", false),
    (
        "browserRetirementReceipts",
        "/browserRetirementReceipts",
        false,
    ),
    ("principalRecords", "/servicePrincipals/principals", true),
    (
        "profileCapabilities",
        "/servicePrincipals/profileCapabilities",
        true,
    ),
    ("runtimeOwners", "/runtimeOwnerRegistry/owners", true),
    (
        "runtimeLifecycleIdentities",
        "/runtimeOwnerRegistry/lifecycleRecords",
        true,
    ),
];

fn summarize_service_state_migration(
    before: &Value,
    after: &Value,
) -> ServiceStateMigrationSummary {
    let mut summary = ServiceStateMigrationSummary::default();
    let mut affected = BTreeSet::new();
    for (class, pointer, protected) in SERVICE_STATE_COLLECTIONS {
        let before_records = before.pointer(pointer).and_then(Value::as_object);
        let after_records = after.pointer(pointer).and_then(Value::as_object);
        let before_ids = before_records
            .into_iter()
            .flat_map(|records| records.keys().cloned())
            .collect::<BTreeSet<_>>();
        let after_ids = after_records
            .into_iter()
            .flat_map(|records| records.keys().cloned())
            .collect::<BTreeSet<_>>();
        let added_ids = after_ids
            .difference(&before_ids)
            .cloned()
            .collect::<Vec<_>>();
        let removed_ids = before_ids
            .difference(&after_ids)
            .cloned()
            .collect::<Vec<_>>();
        let mut changed_ids = Vec::new();
        let mut preserved_ids = Vec::new();
        for id in before_ids.intersection(&after_ids) {
            let before_value = before_records.and_then(|records| records.get(id));
            let after_value = after_records.and_then(|records| records.get(id));
            if before_value == after_value {
                preserved_ids.push(id.clone());
            } else {
                changed_ids.push(id.clone());
            }
        }
        for id in added_ids
            .iter()
            .chain(removed_ids.iter())
            .chain(changed_ids.iter())
        {
            affected.insert(format!("{class}:{id}"));
        }
        if protected {
            summary
                .protected_record_removals
                .extend(removed_ids.iter().map(|id| format!("{class}:{id}")));
        }
        summary.record_classes.insert(
            class.to_string(),
            ServiceStateMigrationRecordClassSummary {
                before_count: before_ids.len(),
                after_count: after_ids.len(),
                added_ids,
                removed_ids,
                changed_ids,
                preserved_ids,
            },
        );
    }
    summary.affected_ids = affected.into_iter().collect();
    summary
}

fn preserve_unknown_successor_fields(source: &Value, migrated: &mut Value) -> Vec<String> {
    let Some(source_object) = source.as_object() else {
        return Vec::new();
    };
    let Some(migrated_object) = migrated.as_object_mut() else {
        return Vec::new();
    };
    let known_top_level = [
        "schemaVersion",
        "profileLeaseSchemaVersion",
        "controlPlane",
        "reconciliation",
        "events",
        "incidents",
        "displayAllocations",
        "remoteViewRoutes",
        "routePool",
        "remoteViewAcquisitionLeases",
        "remoteViewHandoffs",
        "viewerLeases",
        "presentationCapacity",
        "profiles",
        "servicePrincipals",
        "profileLeaseReconcileReceipts",
        "profileRecoveryReceipts",
        "browserRetirementReceipts",
        "crashRegenerationTransactions",
        "browsers",
        "browserProcessIdentities",
        "runtimeOwnerRegistry",
        "sessions",
        "tabs",
        "jobs",
        "monitors",
        "sitePolicies",
        "providers",
        "challenges",
        "profileSeedingHandoffs",
        "browserCapabilityRegistry",
        "defaultBrowserBuild",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let collection_names = SERVICE_STATE_COLLECTIONS
        .into_iter()
        .filter_map(|(_, pointer, _)| pointer.strip_prefix('/').filter(|tail| !tail.contains('/')))
        .collect::<BTreeSet<_>>();
    let mut preserved = Vec::new();
    for (key, source_value) in source_object {
        if !known_top_level.contains(key.as_str()) {
            migrated_object.insert(key.clone(), source_value.clone());
            preserved.push(key.clone());
            continue;
        }
        let Some(migrated_value) = migrated_object.get_mut(key) else {
            continue;
        };
        if collection_names.contains(key.as_str()) {
            merge_existing_record_fields(source_value, migrated_value);
        } else {
            merge_unknown_object_fields(source_value, migrated_value);
        }
    }
    preserved.sort();
    preserved
}

fn merge_existing_record_fields(source: &Value, migrated: &mut Value) {
    let (Some(source_records), Some(migrated_records)) =
        (source.as_object(), migrated.as_object_mut())
    else {
        return;
    };
    for (id, source_record) in source_records {
        if let Some(migrated_record) = migrated_records.get_mut(id) {
            merge_unknown_object_fields(source_record, migrated_record);
        }
    }
}

fn merge_unknown_object_fields(source: &Value, migrated: &mut Value) {
    match (source, migrated) {
        (Value::Object(source_object), Value::Object(migrated_object)) => {
            for (key, source_value) in source_object {
                if let Some(migrated_value) = migrated_object.get_mut(key) {
                    merge_unknown_object_fields(source_value, migrated_value);
                } else {
                    migrated_object.insert(key.clone(), source_value.clone());
                }
            }
        }
        (Value::Array(source_items), Value::Array(migrated_items)) => {
            for (source_item, migrated_item) in source_items.iter().zip(migrated_items.iter_mut()) {
                merge_unknown_object_fields(source_item, migrated_item);
            }
        }
        _ => {}
    }
}

/// Remove ephemeral process evidence only when it has no retained browser
/// projection or authority edge and the current host positively reports the
/// exact recorded process as absent, including a validated prior Linux boot.
/// A current-boot live or indeterminate observation stays
/// in place and fails invariant validation.
fn discard_confirmed_dead_unreferenced_process_identities(state: &mut ServiceState) {
    let discard = state
        .browser_process_identities
        .iter()
        .filter(|(browser_id, identity)| {
            !state.browsers.contains_key(*browser_id)
                && !retained_browser_reference_exists(state, browser_id)
                && orphan_process_is_confirmed_dead(&identity.process_identity)
        })
        .map(|(browser_id, _)| browser_id.clone())
        .collect::<Vec<_>>();
    for browser_id in discard {
        state.browser_process_identities.remove(&browser_id);
    }
}

/// Migration-only absence proof; this never authorizes signalling a reused PID.
/// A well-formed Linux identity from another boot cannot still be running.
/// Other platforms and uncertain tokens retain the ordinary observation gate.
fn orphan_process_is_confirmed_dead(
    identity: &crate::process_identity::RecordedProcessIdentity,
) -> bool {
    #[cfg(target_os = "linux")]
    if let Some(current_epoch) = crate::process_identity::current_boot_epoch() {
        let prior_boot = identity
            .start_token
            .strip_prefix("linux:")
            .and_then(|token| {
                let (boot, ticks) = token.split_once(':')?;
                let boot = uuid::Uuid::parse_str(boot).ok()?;
                let ticks = ticks.parse::<u64>().ok()?;
                (!boot.is_nil() && ticks > 0).then_some(boot)
            });
        let current_boot = current_epoch
            .strip_prefix("linux:")
            .and_then(|boot| uuid::Uuid::parse_str(boot).ok())
            .filter(|boot| !boot.is_nil());
        if matches!((prior_boot, current_boot), (Some(prior), Some(current)) if prior != current) {
            return true;
        }
    }
    matches!(
        crate::process_identity::recorded_process_is_running(identity),
        Ok(false)
    )
}

fn retained_browser_reference_exists(state: &ServiceState, browser_id: &str) -> bool {
    retained_browser_projection_reference_exists(state, browser_id)
        || state
            .runtime_owner_registry
            .owners()
            .values()
            .any(|owner| owner.browser_id == browser_id)
}

fn retained_browser_projection_reference_exists(state: &ServiceState, browser_id: &str) -> bool {
    state
        .sessions
        .values()
        .any(|session| session.browser_ids.iter().any(|id| id == browser_id))
        || state.tabs.values().any(|tab| tab.browser_id == browser_id)
        || state
            .display_allocations
            .values()
            .any(|allocation| allocation.owner_browser_id.as_deref() == Some(browser_id))
        || state
            .remote_view_routes
            .values()
            .any(|route| route.browser_id.as_deref() == Some(browser_id))
        || state
            .remote_view_acquisition_leases
            .values()
            .any(|lease| lease.browser_id == browser_id)
        || state
            .remote_view_handoffs
            .values()
            .any(|handoff| handoff.browser_id.as_deref() == Some(browser_id))
        || state
            .viewer_leases
            .values()
            .any(|lease| lease.browser_id.as_deref() == Some(browser_id))
        || state
            .route_pool
            .values()
            .any(|entry| entry.target.get("browserId").and_then(Value::as_str) == Some(browser_id))
}

/// Preserve the identity of a browser whose only remaining authority edge is
/// a single runtime-owner record after its exact process has exited.
///
/// This is deliberately narrower than general missing-browser repair. A live
/// or indeterminate process, a pending transfer, a principal binding, or any
/// retained browser projection remains a migration blocker.
fn materialize_inert_owner_only_process_placeholders(state: &mut ServiceState) {
    let missing_browsers = state
        .runtime_owner_registry
        .owners()
        .values()
        .filter(|owner| !owner.browser_id.trim().is_empty())
        .filter(|owner| !state.browsers.contains_key(&owner.browser_id))
        .filter(|owner| owner.pending_transfer.is_none())
        .filter(|owner| {
            !state
                .runtime_owner_registry
                .principal_bindings()
                .contains_key(&owner.profile_identity_digest)
        })
        .filter(|owner| {
            state
                .runtime_owner_registry
                .owners()
                .values()
                .filter(|candidate| candidate.browser_id == owner.browser_id)
                .count()
                == 1
        })
        .filter(|owner| !retained_browser_projection_reference_exists(state, &owner.browser_id))
        .filter(|owner| {
            state
                .browser_process_identities
                .get(&owner.browser_id)
                .is_some_and(|identity| {
                    matches!(
                        crate::process_identity::recorded_process_is_running(
                            &identity.process_identity
                        ),
                        Ok(false)
                    )
                })
        })
        .map(|owner| owner.browser_id.clone())
        .collect::<std::collections::BTreeSet<_>>();

    for browser_id in missing_browsers {
        state.browser_process_identities.remove(&browser_id);
        state.browsers.insert(
            browser_id.clone(),
            BrowserProcess {
                id: browser_id,
                last_error: Some(
                    "Migrated inert placeholder for an owner-only confirmed-dead process."
                        .to_string(),
                ),
                ..BrowserProcess::default()
            },
        );
    }
}

/// Preserve the identity of an already-terminal remote-view projection.
///
/// Orphaned and released display or route rows are historical evidence, not
/// live browser authority. A placeholder is allowed only when every reference
/// to the missing browser is terminal or already-invalid, no retained process
/// identity still resolves to a live process, any retained owner is unique with
/// no pending transfer, and no viewer or controller lease can still authorize
/// interaction. A principal binding is preserved only when the registered
/// principal and capability are both active. A binding behind the retained
/// owner generation remains visible for first-class lease reconciliation, but
/// cannot become effect-capable through the inert placeholder.
fn materialize_inert_legacy_remote_view_placeholders(state: &mut ServiceState) {
    let missing_browsers = state
        .display_allocations
        .values()
        .filter_map(|allocation| allocation.owner_browser_id.as_ref())
        .chain(
            state
                .remote_view_routes
                .values()
                .filter_map(|route| route.browser_id.as_ref()),
        )
        .filter(|browser_id| !browser_id.trim().is_empty())
        .filter(|browser_id| !state.browsers.contains_key(*browser_id))
        .filter(|browser_id| retained_process_identity_is_inert(state, browser_id))
        .filter(|browser_id| {
            let owners = state
                .runtime_owner_registry
                .owners()
                .values()
                .filter(|owner| owner.browser_id == ***browser_id)
                .collect::<Vec<_>>();
            owners.is_empty()
                || (owners.len() == 1
                    && owners[0].pending_transfer.is_none()
                    && owner_principal_binding_is_migration_safe(state, owners[0]))
        })
        .filter(|browser_id| {
            !state
                .sessions
                .values()
                .any(|session| session.browser_ids.iter().any(|id| id == *browser_id))
        })
        .filter(|browser_id| {
            let allocations = state
                .display_allocations
                .values()
                .filter(|allocation| allocation.owner_browser_id.as_ref() == Some(*browser_id))
                .collect::<Vec<_>>();
            allocations.iter().all(|allocation| {
                allocation.readiness.as_ref().is_some_and(|readiness| {
                    (allocation.state == "orphaned"
                        && readiness.get("state").and_then(Value::as_str) == Some("orphaned")
                        && readiness.get("reason").and_then(Value::as_str)
                            == Some("owner_browser_not_ready"))
                        || (allocation.state == "released"
                            && readiness.get("state").and_then(Value::as_str) == Some("released")
                            && matches!(
                                readiness.get("reason").and_then(Value::as_str),
                                Some(
                                    "operator_requested_close"
                                        | "duplicate_browser_record_merged"
                                        | "route_switch_parking"
                                )
                            ))
                })
            })
        })
        .filter(|browser_id| {
            state
                .tabs
                .values()
                .filter(|tab| tab.browser_id == ***browser_id)
                .all(|tab| {
                    tab.principal_id.is_none()
                        && tab.work_lease_id.is_none()
                        && tab.service_tab_handle.as_ref().is_some_and(|handle| {
                            !handle.valid
                                && handle.stale_reason.as_deref() == Some("browser_missing")
                        })
                })
        })
        .filter(|browser_id| {
            state
                .remote_view_routes
                .values()
                .filter(|route| route.browser_id.as_ref() == Some(*browser_id))
                .all(|route| {
                    route.viewer_lease_ids.is_empty()
                        && route.controller_lease_id.is_none()
                        // A controller epoch is historical fencing evidence, not
                        // current control authority. An orphaned route with no
                        // retained viewer or controller lease remains inert even
                        // when an earlier controller advanced the epoch.
                        && (route.state == "orphaned"
                            || (route.state == "released"
                                && matches!(
                                    route.last_provider_event.as_deref(),
                                    Some("route_released" | "route_released_after_browser_close")
                                )))
                })
        })
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();

    for browser_id in missing_browsers {
        // The terminal projection checks above prove that this identity cannot
        // authorize more work. Drop only a process identity that the current
        // host positively reports as absent. Live and indeterminate identities
        // remain migration blockers.
        state.browser_process_identities.remove(&browser_id);
        let allocations = state
            .display_allocations
            .values()
            .filter(|allocation| allocation.owner_browser_id.as_deref() == Some(&browser_id))
            .collect::<Vec<_>>();
        let profile_ids = allocations
            .iter()
            .filter_map(|allocation| allocation.profile_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let allocation_ids = allocations
            .iter()
            .map(|allocation| allocation.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let session_ids = allocations
            .iter()
            .filter_map(|allocation| allocation.owner_session_id.clone())
            .chain(
                state
                    .remote_view_routes
                    .values()
                    .filter(|route| route.browser_id.as_deref() == Some(&browser_id))
                    .filter_map(|route| route.session_id.clone()),
            )
            .filter(|session_id| !session_id.trim().is_empty())
            .collect::<std::collections::BTreeSet<_>>();

        let profile_id = (profile_ids.len() == 1)
            .then(|| profile_ids.iter().next().cloned())
            .flatten();
        let display_allocation_id = (allocation_ids.len() == 1)
            .then(|| allocation_ids.iter().next().cloned())
            .flatten();
        state.browsers.insert(
            browser_id.clone(),
            BrowserProcess {
                id: browser_id.clone(),
                profile_id: profile_id.clone(),
                host: super::service_model::BrowserHost::RemoteHeaded,
                display_allocation_id,
                active_session_ids: session_ids.iter().cloned().collect(),
                last_error: Some(
                    "Migrated inert placeholder for orphaned remote-view evidence.".to_string(),
                ),
                ..BrowserProcess::default()
            },
        );
        for session_id in session_ids {
            state
                .sessions
                .entry(session_id.clone())
                .or_insert_with(|| BrowserSession {
                    id: session_id,
                    profile_id: profile_id.clone(),
                    lease: LeaseState::Released,
                    browser_ids: vec![browser_id.clone()],
                    ..BrowserSession::default()
                });
        }
    }
}

fn retained_process_identity_is_inert(state: &ServiceState, browser_id: &str) -> bool {
    state
        .browser_process_identities
        .get(browser_id)
        .is_none_or(|identity| {
            matches!(
                crate::process_identity::recorded_process_is_running(&identity.process_identity),
                Ok(false)
            )
        })
}

fn owner_principal_binding_is_migration_safe(
    state: &ServiceState,
    owner: &crate::runtime_owner_transfer::ProfileOwner,
) -> bool {
    let Some(binding) = state
        .runtime_owner_registry
        .principal_bindings()
        .get(&owner.profile_identity_digest)
    else {
        return true;
    };
    binding.profile_identity_digest == owner.profile_identity_digest
        && binding.owner_generation <= owner.owner_generation
        && binding.provenance
            == crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability
        && state
            .service_principal(&binding.principal_id)
            .is_some_and(|principal| {
                principal.state
                    == crate::native::service_principal::ServicePrincipalState::Active
                    && principal.provenance
                        == crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability
            })
        && state
            .profile_capability(&binding.capability_id)
            .is_some_and(|capability| {
                capability.principal_id == binding.principal_id
                    && capability.profile_id == binding.profile_id
                    && capability.state
                        == crate::native::service_principal::ServiceProfileCapabilityState::Active
            })
}

/// Preserve an inert legacy lease whose profile row was never persisted.
///
/// A placeholder restores only referential identity. It cannot manufacture
/// principal authority because the source session has no principal, work
/// capability, browser, or tab binding. Any dangling profile reference with
/// effect-bearing evidence remains untouched and fails invariant validation.
fn materialize_inert_legacy_profile_placeholders(state: &mut ServiceState) {
    let missing_profiles = state
        .sessions
        .values()
        .filter_map(|session| {
            let profile_id = session.profile_id.as_ref()?;
            (!profile_id.trim().is_empty()
                && !state.profiles.contains_key(profile_id)
                && session.principal_id.is_none()
                && session.work_lease_id.is_none()
                && session.browser_ids.is_empty()
                && session.tab_ids.is_empty())
            .then(|| profile_id.clone())
        })
        .collect::<std::collections::BTreeSet<_>>();

    for profile_id in missing_profiles {
        state.profiles.insert(
            profile_id.clone(),
            BrowserProfile {
                id: profile_id.clone(),
                name: profile_id,
                description: Some(
                    "Migrated placeholder for an inert legacy session; principal authority remains unproven."
                        .to_string(),
                ),
                persistent: true,
                ..BrowserProfile::default()
            },
        );
    }
}

/// Preserve a stale tab record whose historical browser row was never kept.
///
/// The invalid `browser_missing` handle is the migration proof that the tab
/// cannot authorize work. Any principal, work lease, live or indeterminate
/// process identity or retained session reference keeps the missing browser as
/// a hard blocker. A single owner may be preserved only when its exact profile
/// digest is still backed by an active registered principal capability and no
/// transfer is pending.
fn materialize_inert_legacy_browser_placeholders(state: &mut ServiceState) {
    let missing_browsers = state
        .tabs
        .values()
        .map(|tab| tab.browser_id.as_str())
        .filter(|browser_id| !browser_id.trim().is_empty())
        .filter(|browser_id| !state.browsers.contains_key(*browser_id))
        .filter(|browser_id| retained_process_identity_is_inert(state, browser_id))
        .filter(|browser_id| {
            !state
                .sessions
                .values()
                .any(|session| session.browser_ids.iter().any(|id| id == *browser_id))
        })
        .filter(|browser_id| {
            let owners = state
                .runtime_owner_registry
                .owners()
                .values()
                .filter(|owner| owner.browser_id == **browser_id)
                .collect::<Vec<_>>();
            owners.is_empty()
                || (owners.len() == 1
                    && owners[0].pending_transfer.is_none()
                    && state
                        .runtime_owner_registry
                        .principal_bindings()
                        .contains_key(&owners[0].profile_identity_digest)
                    && owner_principal_binding_is_migration_safe(state, owners[0]))
        })
        .filter(|browser_id| {
            let tabs = state
                .tabs
                .values()
                .filter(|tab| tab.browser_id == **browser_id)
                .collect::<Vec<_>>();
            !tabs.is_empty()
                && tabs.iter().all(|tab| {
                    tab.principal_id.is_none()
                        && tab.work_lease_id.is_none()
                        && tab.service_tab_handle.as_ref().is_some_and(|handle| {
                            !handle.valid
                                && handle.stale_reason.as_deref() == Some("browser_missing")
                        })
                })
        })
        .map(str::to_string)
        .collect::<std::collections::BTreeSet<_>>();

    for browser_id in missing_browsers {
        state.browser_process_identities.remove(&browser_id);
        state.browsers.insert(
            browser_id.clone(),
            BrowserProcess {
                id: browser_id,
                last_error: Some(
                    "Migrated inert placeholder for stale browser_missing tab evidence."
                        .to_string(),
                ),
                ..BrowserProcess::default()
            },
        );
    }
}

/// Restore the routing identity around an already-invalid stale tab.
///
/// The synthesized session is released and system-owned. It retains the
/// historical browser and tab links without creating a principal or work
/// capability, so the stale tab stays non-effect-capable.
fn materialize_inert_legacy_session_placeholders(state: &mut ServiceState) {
    let mut missing_sessions = std::collections::BTreeMap::<
        String,
        (
            std::collections::BTreeSet<String>,
            std::collections::BTreeSet<String>,
        ),
    >::new();
    for tab in state.tabs.values() {
        let inert_missing_browser = tab.principal_id.is_none()
            && tab.work_lease_id.is_none()
            && state.browsers.get(&tab.browser_id).is_some_and(|browser| {
                browser.health == super::service_model::BrowserHealth::NotStarted
                    && browser.pid.is_none()
                    && browser.last_error.as_deref()
                        == Some(
                            "Migrated inert placeholder for stale browser_missing tab evidence.",
                        )
            })
            && tab.service_tab_handle.as_ref().is_some_and(|handle| {
                !handle.valid && handle.stale_reason.as_deref() == Some("browser_missing")
            });
        if !inert_missing_browser {
            continue;
        }
        let session_ids = [tab.session_id.as_ref(), tab.owner_session_id.as_ref()]
            .into_iter()
            .flatten()
            .filter(|session_id| !session_id.trim().is_empty())
            .filter(|session_id| !state.sessions.contains_key(*session_id))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        for session_id in session_ids {
            let (browser_ids, tab_ids) = missing_sessions.entry(session_id).or_default();
            browser_ids.insert(tab.browser_id.clone());
            tab_ids.insert(tab.id.clone());
        }
    }

    for (session_id, (browser_ids, tab_ids)) in missing_sessions {
        state.sessions.insert(
            session_id.clone(),
            BrowserSession {
                id: session_id,
                lease: LeaseState::Released,
                browser_ids: browser_ids.into_iter().collect(),
                tab_ids: tab_ids.into_iter().collect(),
                ..BrowserSession::default()
            },
        );
    }
}

/// Validate the cross-record relationships required by a persisted Service
/// State document without modifying the decoded snapshot or durable storage.
pub(crate) fn validate_service_state_invariants(state: &ServiceState) -> Result<(), String> {
    agent_browser_service_model::validate_service_state_invariants(state)
        .map_err(|error| error.to_string())
}

fn custom_json_error(message: String) -> serde_json::Error {
    <serde_json::Error as serde::de::Error>::custom(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_shared_profile_policy_materializes_as_nonblocking_shared_local() {
        let raw = json!({
            "stateRevision": 9,
            "profiles": {
                "research-gov": {
                    "id": "research-gov",
                    "name": "Research.gov"
                }
            }
        })
        .to_string();

        let first = read_service_state(&raw).unwrap();
        let second = read_service_state(&raw).unwrap();
        let policy = first.profiles["research-gov"]
            .access_policy
            .as_ref()
            .unwrap();
        assert_eq!(policy.mode, ProfileAccessMode::SharedLocal);
        assert_eq!(policy.revision, 1);
        let report = first.profile_policy_migration.as_ref().unwrap();
        assert_eq!(report.source_revision, 9);
        assert_eq!(report.target_revision, 10);
        assert_eq!(report.blocking_issue_count, 0);
        assert_eq!(report.entries[0].classification, "shared-local-default");
        assert!(!report.entries[0].ambiguity);
        assert!(!report.entries[0].blocking);
        assert_eq!(
            report.migration_id,
            second.profile_policy_migration.unwrap().migration_id
        );
    }

    #[test]
    fn ambiguous_legacy_occupancy_is_observable_but_does_not_block() {
        let raw = json!({
            "profiles": {
                "research-gov": {
                    "id": "research-gov",
                    "name": "Research.gov"
                }
            },
            "sessions": {
                "fieldwork": {
                    "id": "fieldwork",
                    "profileId": "research-gov",
                    "lease": "shared"
                }
            }
        })
        .to_string();

        let state = read_service_state(&raw).unwrap();
        let report = state.profile_policy_migration.unwrap();
        assert_eq!(report.blocking_issue_count, 0);
        assert_eq!(report.entries[0].classification, "ambiguous-legacy");
        assert!(report.entries[0].ambiguity);
        assert!(!report.entries[0].blocking);
        assert_eq!(
            state.profiles["research-gov"]
                .access_policy
                .as_ref()
                .unwrap()
                .mode,
            ProfileAccessMode::SharedLocal
        );
    }

    #[test]
    fn proven_exclusive_legacy_occupancy_migrates_to_subject_bound_administration() {
        let raw = json!({
            "stateRevision": 4,
            "profiles": {
                "research-gov": {
                    "id": "research-gov",
                    "name": "Research.gov"
                }
            },
            "sessions": {
                "fieldwork": {
                    "id": "fieldwork",
                    "profileId": "research-gov",
                    "lease": "exclusive",
                    "principalId": "principal:fieldwork",
                    "principalProvenance": "registered_capability"
                }
            }
        })
        .to_string();

        let state = read_service_state(&raw).unwrap();
        let policy = state.profiles["research-gov"]
            .access_policy
            .as_ref()
            .unwrap();
        assert_eq!(policy.mode, ProfileAccessMode::Exclusive);
        assert!(policy.default_permissions.is_empty());
        assert_eq!(policy.grants.len(), 1);
        assert_eq!(policy.grants[0].subject_id, "principal:fieldwork");
        assert_eq!(
            policy.grants[0].minimum_assurance,
            ProfileIdentityAssurance::RegisteredCapability
        );
        assert!(policy.grants[0].permissions.contains(
            &super::super::service_profile_access_policy::ProfilePermission::FullShutdown
        ));
        let report = state.profile_policy_migration.as_ref().unwrap();
        assert_eq!(
            report.entries[0].classification,
            "proven-strict-compatibility"
        );
        assert!(!report.entries[0].ambiguity);
        assert_eq!(report.target_revision, 5);
    }

    #[test]
    fn legacy_labels_remain_unproven_and_stage_without_effects() {
        let raw = json!({
            "profiles": {
                "fedex": { "id": "fedex", "name": "FedEx" }
            },
            "sessions": {
                "odollo": {
                    "id": "odollo",
                    "serviceName": "Odollo fulfillment",
                    "profileId": "fedex",
                    "lease": "exclusive"
                }
            }
        })
        .to_string();
        let staged = stage_service_state_migration(&raw).unwrap();
        assert_eq!(staged.plan.status, ServiceStateMigrationStatus::Required);
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(migrated["schemaVersion"], SERVICE_STATE_SCHEMA_VERSION);
        assert_eq!(
            migrated["profileLeaseSchemaVersion"],
            super::super::service_profile_lease::PROFILE_LEASE_SCHEMA_VERSION
        );
        assert!(migrated
            .get("servicePrincipals")
            .and_then(Value::as_object)
            .is_none_or(|registry| registry.is_empty()));
    }

    #[test]
    fn unknown_newer_schema_is_read_only_and_bytes_are_unchanged() {
        let raw = r#"{"schemaVersion":"agent-browser.service-state.v99","profiles":{}}"#;
        let before = raw.as_bytes().to_vec();
        let plan = plan_service_state_migration(raw).unwrap();
        assert_eq!(plan.status, ServiceStateMigrationStatus::BlockedNewerSchema);
        assert!(!plan.forward_reader_available);
        assert!(stage_service_state_migration(raw)
            .unwrap_err()
            .contains("blocked_newer_schema"));
        assert_eq!(raw.as_bytes(), before);
    }

    #[test]
    fn unknown_lease_schema_blocks_without_rewriting_current_state() {
        let raw = format!(
            r#"{{"schemaVersion":"{SERVICE_STATE_SCHEMA_VERSION}","profileLeaseSchemaVersion":"agent-browser.profile-leases.v99"}}"#
        );
        let before = raw.as_bytes().to_vec();
        let plan = plan_service_state_migration(&raw).unwrap();
        assert_eq!(plan.status, ServiceStateMigrationStatus::BlockedNewerSchema);
        assert!(!plan.forward_reader_available);
        assert!(read_service_state(&raw)
            .unwrap_err()
            .to_string()
            .contains("profile_lease_schema_unsupported"));
        assert!(stage_service_state_migration(&raw)
            .unwrap_err()
            .contains("blocked_newer_schema"));
        assert_eq!(raw.as_bytes(), before);
    }

    #[test]
    fn staged_additive_versions_remain_readable_by_a_legacy_projection() {
        #[derive(Debug, Default, Deserialize)]
        #[serde(default, rename_all = "camelCase")]
        struct LegacyProjection {
            profiles:
                std::collections::BTreeMap<String, super::super::service_model::BrowserProfile>,
            sessions:
                std::collections::BTreeMap<String, super::super::service_model::BrowserSession>,
        }

        let raw = json!({
            "profiles": {"fedex": {"id": "fedex", "name": "FedEx"}},
            "sessions": {}
        })
        .to_string();
        let staged = stage_service_state_migration(&raw).unwrap();
        assert!(staged.plan.old_reader_compatible);
        let legacy: LegacyProjection = serde_json::from_slice(&staged.bytes).unwrap();
        assert!(legacy.profiles.contains_key("fedex"));
        assert!(legacy.sessions.is_empty());
    }

    #[test]
    fn inert_legacy_session_materializes_missing_profile_without_authority() {
        let raw = json!({
            "sessions": {
                "holder": {
                    "id": "holder",
                    "owner": "system",
                    "lease": "exclusive",
                    "profileId": "work",
                    "browserIds": [],
                    "tabIds": []
                }
            }
        })
        .to_string();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(migrated["profiles"]["work"]["id"], "work");
        assert_eq!(migrated["profiles"]["work"]["name"], "work");
        assert_eq!(migrated["profiles"]["work"]["persistent"], true);
        assert_eq!(
            migrated["profiles"]["work"]["description"],
            "Migrated placeholder for an inert legacy session; principal authority remains unproven."
        );
        assert!(migrated
            .get("servicePrincipals")
            .and_then(Value::as_object)
            .is_none_or(|registry| registry.is_empty()));
    }

    #[test]
    fn principal_bound_session_with_missing_profile_stays_blocked() {
        let raw = json!({
            "sessions": {
                "holder": {
                    "id": "holder",
                    "principalId": "service-a",
                    "lease": "exclusive",
                    "profileId": "work",
                    "browserIds": [],
                    "tabIds": []
                }
            }
        })
        .to_string();

        assert_eq!(
            stage_service_state_migration(&raw).unwrap_err(),
            "service_state_session_profile_missing:holder:work"
        );
    }

    #[test]
    fn stale_invalid_tab_materializes_inert_missing_browser() {
        let raw = json!({
            "tabs": {
                "target:tab-a": {
                    "id": "target:tab-a",
                    "browserId": "session:im-receipts",
                    "targetId": "tab-a",
                    "sessionId": "im-receipts",
                    "lifecycle": "ready",
                    "serviceTabHandle": {
                        "browserId": "session:im-receipts",
                        "sessionName": "im-receipts",
                        "tabId": "target:tab-a",
                        "targetId": "tab-a",
                        "valid": false,
                        "staleReason": "browser_missing"
                    }
                }
            }
        })
        .to_string();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["browsers"]["session:im-receipts"]["id"],
            "session:im-receipts"
        );
        assert_eq!(
            migrated["browsers"]["session:im-receipts"]["health"],
            "not_started"
        );
        assert_eq!(
            migrated["browsers"]["session:im-receipts"]["lastError"],
            "Migrated inert placeholder for stale browser_missing tab evidence."
        );
        assert_eq!(migrated["sessions"]["im-receipts"]["lease"], "released");
        assert_eq!(
            migrated["sessions"]["im-receipts"]["browserIds"],
            json!(["session:im-receipts"])
        );
        assert_eq!(
            migrated["sessions"]["im-receipts"]["tabIds"],
            json!(["target:tab-a"])
        );
        assert_eq!(migrated["tabs"]["target:tab-a"]["lifecycle"], "ready");
    }

    #[test]
    fn registered_owner_bound_stale_invalid_tab_preserves_authority_through_placeholder() {
        let raw = json!({
            "profiles": {
                "last30days-facebook": {
                    "id": "last30days-facebook",
                    "name": "Last30Days Facebook",
                    "persistent": true
                }
            },
            "tabs": {
                "target:tab-a": {
                    "id": "target:tab-a",
                    "browserId": "session:last30days",
                    "targetId": "tab-a",
                    "sessionId": "last30days",
                    "ownerSessionId": "last30days",
                    "lifecycle": "ready",
                    "serviceTabHandle": {
                        "browserId": "session:last30days",
                        "sessionName": "last30days",
                        "tabId": "target:tab-a",
                        "targetId": "tab-a",
                        "valid": false,
                        "staleReason": "browser_missing"
                    }
                }
            }
        })
        .to_string();
        let mut state = read_service_state(&raw).unwrap();
        let profile_identity_digest = "1".repeat(64);
        let owner_generation = 90;
        state.runtime_owner_registry =
            crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                crate::runtime_owner_transfer::ProfileOwner {
                    owner_id: "owner-last30days".to_string(),
                    profile_identity_digest: profile_identity_digest.clone(),
                    state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                    owner_generation,
                    browser_id: "session:last30days".to_string(),
                    daemon_session_route: "last30days".to_string(),
                    process_instance_digest: "2".repeat(64),
                    browser_family: "chrome".to_string(),
                    cdp_endpoint_identity_digest: "3".repeat(64),
                    target_set_digest: "4".repeat(64),
                    pending_transfer: None,
                    last_transition: None,
                },
            );
        let registered = crate::native::service_principal::register_profile_capability(
            &mut state.service_principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "last30days".to_string(),
                display_name: Some("Last30Days".to_string()),
                profile_id: "last30days-facebook".to_string(),
                registered_at: Some("2026-09-02T12:54:45Z".to_string()),
                registered_by: Some("operator".to_string()),
            },
            "synthetic-last30days-capability-more-than-thirty-two-characters",
        )
        .unwrap();
        state
            .runtime_owner_registry
            .bind_principal_authority(
                crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                    principal_id: registered.principal.principal_id,
                    profile_id: registered.capability.profile_id,
                    profile_identity_digest: profile_identity_digest.clone(),
                    capability_id: registered.capability.capability_id,
                    provenance: crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
                    owner_generation,
                },
            )
            .unwrap();

        let expected_error = "service_state_tab_browser_missing:target:tab-a:session:last30days";

        let mut unbound = state.clone();
        crate::runtime_owner_transfer::edit_registry_fixture(&mut unbound.runtime_owner_registry)
            .principal_records
            .remove(&profile_identity_digest);
        assert_eq!(
            stage_service_state_migration(&serde_json::to_string(&unbound).unwrap()).unwrap_err(),
            expected_error
        );

        let capability_id = state.runtime_owner_registry.principal_bindings()
            [&profile_identity_digest]
            .capability_id
            .clone();
        let mut revoked = state.clone();
        revoked
            .service_principals
            .profile_capabilities
            .get_mut(&capability_id)
            .unwrap()
            .state = crate::native::service_principal::ServiceProfileCapabilityState::Revoked;
        assert_eq!(
            stage_service_state_migration(&serde_json::to_string(&revoked).unwrap()).unwrap_err(),
            expected_error
        );

        let mut generation_ahead = state.clone();
        crate::runtime_owner_transfer::edit_registry_fixture(
            &mut generation_ahead.runtime_owner_registry,
        )
        .principal_records
        .get_mut(&profile_identity_digest)
        .unwrap()
        .owner_generation += 1;
        assert_eq!(
            stage_service_state_migration(&serde_json::to_string(&generation_ahead).unwrap())
                .unwrap_err(),
            expected_error
        );

        let mut valid_tab = state.clone();
        valid_tab
            .tabs
            .get_mut("target:tab-a")
            .unwrap()
            .service_tab_handle
            .as_mut()
            .unwrap()
            .valid = true;
        assert_eq!(
            stage_service_state_migration(&serde_json::to_string(&valid_tab).unwrap()).unwrap_err(),
            expected_error
        );

        let staged =
            stage_service_state_migration(&serde_json::to_string(&state).unwrap()).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["browsers"]["session:last30days"]["health"],
            "not_started"
        );
        assert_eq!(migrated["sessions"]["last30days"]["lease"], "released");
        assert_eq!(
            migrated["runtimeOwnerRegistry"]["owners"][&profile_identity_digest]["ownerGeneration"],
            owner_generation
        );
        assert_eq!(
            migrated["runtimeOwnerRegistry"]["principalBindings"][&profile_identity_digest]
                ["ownerGeneration"],
            owner_generation
        );
        assert_eq!(migrated["tabs"]["target:tab-a"]["lifecycle"], "ready");
    }

    #[test]
    fn valid_tab_with_missing_browser_stays_blocked() {
        let raw = json!({
            "tabs": {
                "target:tab-a": {
                    "id": "target:tab-a",
                    "browserId": "session:im-receipts",
                    "lifecycle": "ready",
                    "serviceTabHandle": {
                        "browserId": "session:im-receipts",
                        "tabId": "target:tab-a",
                        "valid": true
                    }
                }
            }
        })
        .to_string();

        assert_eq!(
            stage_service_state_migration(&raw).unwrap_err(),
            "service_state_tab_browser_missing:target:tab-a:session:im-receipts"
        );
    }

    #[test]
    fn stale_missing_browser_tab_discards_only_a_confirmed_dead_process_identity() {
        let raw = json!({
            "browserProcessIdentities": {
                "session:stale": {
                    "processIdentity": {
                        "pid": 2_000_000_000_u32,
                        "startToken": "definitely-absent-process"
                    },
                    "runtimeProfile": "stale"
                }
            },
            "tabs": {
                "target:stale": {
                    "id": "target:stale",
                    "browserId": "session:stale",
                    "lifecycle": "closed",
                    "serviceTabHandle": {
                        "browserId": "session:stale",
                        "tabId": "target:stale",
                        "valid": false,
                        "staleReason": "browser_missing"
                    }
                }
            }
        })
        .to_string();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["browsers"]["session:stale"]["health"],
            "not_started"
        );
        assert!(migrated["browserProcessIdentities"]
            .get("session:stale")
            .is_none());
    }

    #[test]
    fn unreferenced_confirmed_dead_process_identity_is_discarded() {
        let raw = json!({
            "browserProcessIdentities": {
                "session:stale": {
                    "processIdentity": {
                        "pid": 2_000_000_000_u32,
                        "startToken": "definitely-absent-process"
                    },
                    "runtimeProfile": "stale"
                }
            }
        })
        .to_string();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert!(migrated["browserProcessIdentities"]
            .get("session:stale")
            .is_none());
        assert!(migrated["browsers"].as_object().unwrap().is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unreferenced_prior_boot_identity_with_reused_pid_is_discarded() {
        let current = crate::process_identity::current_boot_epoch().unwrap();
        let prior = if current == "linux:11111111-1111-4111-8111-111111111111" {
            "linux:22222222-2222-4222-8222-222222222222"
        } else {
            "linux:11111111-1111-4111-8111-111111111111"
        };
        let raw = json!({
            "browserProcessIdentities": {
                "session:stale": {
                    "processIdentity": {
                        "pid": std::process::id(),
                        "startToken": format!("{prior}:1")
                    },
                    "runtimeProfile": "stale"
                }
            }
        });
        let staged = stage_service_state_migration(&raw.to_string()).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert!(migrated["browserProcessIdentities"]
            .get("session:stale")
            .is_none());
        assert!(migrated["browsers"].as_object().unwrap().is_empty());

        for token in [
            format!("{current}:1"),
            "linux:unknown:1".into(),
            format!("{prior}:invalid"),
            format!("{prior}:1:extra"),
        ] {
            let mut uncertain = raw.clone();
            uncertain["browserProcessIdentities"]["session:stale"]["processIdentity"]
                ["startToken"] = json!(token);
            assert!(stage_service_state_migration(&uncertain.to_string()).is_err());
        }
        let mut referenced = raw;
        referenced["sessions"] =
            json!({"retained": {"id": "retained", "browserIds": ["session:stale"]}});
        assert!(stage_service_state_migration(&referenced.to_string()).is_err());
        assert!(
            crate::process_identity::capture_process_identity(std::process::id(), None, None)
                .is_some()
        );
    }

    #[test]
    fn owner_only_confirmed_dead_process_materializes_inert_browser() {
        let raw = json!({
            "browserProcessIdentities": {
                "session:stale-owner": {
                    "processIdentity": {
                        "pid": 2_000_000_000_u32,
                        "startToken": "definitely-absent-process"
                    },
                    "runtimeProfile": "stale-owner"
                }
            }
        })
        .to_string();
        let mut state = read_service_state(&raw).unwrap();
        let profile_identity_digest = "1".repeat(64);
        state.runtime_owner_registry =
            crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(
                crate::runtime_owner_transfer::ProfileOwner {
                    owner_id: "owner-stale".to_string(),
                    profile_identity_digest: profile_identity_digest.clone(),
                    state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
                    owner_generation: 1,
                    browser_id: "session:stale-owner".to_string(),
                    daemon_session_route: "stale-owner".to_string(),
                    process_instance_digest: "2".repeat(64),
                    browser_family: "chrome".to_string(),
                    cdp_endpoint_identity_digest: "3".repeat(64),
                    target_set_digest: "4".repeat(64),
                    pending_transfer: None,
                    last_transition: None,
                },
            );

        let mut with_live_process_evidence = state.clone();
        with_live_process_evidence
            .browser_process_identities
            .get_mut("session:stale-owner")
            .unwrap()
            .process_identity =
            crate::process_identity::capture_process_identity(std::process::id(), None, None)
                .unwrap();
        assert_eq!(
            stage_service_state_migration(
                &serde_json::to_string(&with_live_process_evidence).unwrap()
            )
            .unwrap_err(),
            "service_state_process_browser_missing:session:stale-owner"
        );

        let staged =
            stage_service_state_migration(&serde_json::to_string(&state).unwrap()).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["browsers"]["session:stale-owner"]["health"],
            "not_started"
        );
        assert_eq!(
            migrated["browsers"]["session:stale-owner"]["lastError"],
            "Migrated inert placeholder for an owner-only confirmed-dead process."
        );
        assert!(migrated["browserProcessIdentities"]
            .get("session:stale-owner")
            .is_none());
        assert_eq!(
            migrated["runtimeOwnerRegistry"]["owners"][&profile_identity_digest]["state"],
            "ready"
        );
    }

    #[test]
    fn orphaned_remote_view_materializes_inert_browser_and_session() {
        let raw = json!({
            "profiles": {
                "bill-soylei": { "id": "bill-soylei", "name": "bill-soylei" }
            },
            "displayAllocations": {
                "display:bill": {
                    "id": "display:bill",
                    "ownerBrowserId": "session:bill-soylei",
                    "ownerSessionId": "bill-soylei",
                    "profileId": "bill-soylei",
                    "state": "orphaned",
                    "readiness": {
                        "state": "orphaned",
                        "reason": "owner_browser_not_ready"
                    }
                }
            },
            "remoteViewRoutes": {
                "guacamole:2": {
                    "id": "guacamole:2",
                    "browserId": "session:bill-soylei",
                    "sessionId": "bill-soylei",
                    "state": "orphaned",
                    "viewerLeaseIds": [],
                    "controllerLeaseId": null,
                    "controllerEpoch": 0
                }
            }
        })
        .to_string();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["browsers"]["session:bill-soylei"]["health"],
            "not_started"
        );
        assert_eq!(
            migrated["browsers"]["session:bill-soylei"]["profileId"],
            "bill-soylei"
        );
        assert_eq!(
            migrated["browsers"]["session:bill-soylei"]["lastError"],
            "Migrated inert placeholder for orphaned remote-view evidence."
        );
        assert_eq!(migrated["sessions"]["bill-soylei"]["lease"], "released");
        assert_eq!(
            migrated["sessions"]["bill-soylei"]["browserIds"],
            json!(["session:bill-soylei"])
        );
    }

    #[test]
    fn orphaned_remote_view_discards_only_a_confirmed_dead_process_identity() {
        let raw = json!({
            "profiles": {
                "candidate": { "id": "candidate", "name": "candidate" }
            },
            "browserProcessIdentities": {
                "session:candidate": {
                    "processIdentity": {
                        "pid": 2_000_000_000_u32,
                        "startToken": "definitely-absent-process"
                    },
                    "runtimeProfile": "candidate"
                }
            },
            "displayAllocations": {
                "display:candidate": {
                    "id": "display:candidate",
                    "ownerBrowserId": "session:candidate",
                    "ownerSessionId": "candidate",
                    "profileId": "candidate",
                    "state": "orphaned",
                    "readiness": {
                        "state": "orphaned",
                        "reason": "owner_browser_not_ready"
                    }
                }
            },
            "remoteViewRoutes": {
                "guacamole:candidate": {
                    "id": "guacamole:candidate",
                    "browserId": "session:candidate",
                    "sessionId": "candidate",
                    "state": "orphaned",
                    "viewerLeaseIds": [],
                    "controllerLeaseId": null,
                    "controllerEpoch": 0
                }
            }
        })
        .to_string();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["browsers"]["session:candidate"]["health"],
            "not_started"
        );
        assert!(migrated["browserProcessIdentities"]
            .get("session:candidate")
            .is_none());
    }

    #[test]
    fn controlled_remote_view_with_missing_browser_stays_blocked() {
        let raw = json!({
            "displayAllocations": {
                "display:bill": {
                    "id": "display:bill",
                    "ownerBrowserId": "session:bill-soylei",
                    "state": "orphaned",
                    "readiness": {
                        "state": "orphaned",
                        "reason": "owner_browser_not_ready"
                    }
                }
            },
            "remoteViewRoutes": {
                "guacamole:2": {
                    "id": "guacamole:2",
                    "browserId": "session:bill-soylei",
                    "state": "orphaned",
                    "controllerLeaseId": "controller:active",
                    "controllerEpoch": 1
                }
            }
        })
        .to_string();

        assert_eq!(
            stage_service_state_migration(&raw).unwrap_err(),
            "service_state_display_browser_missing:display:bill:session:bill-soylei"
        );
    }

    #[test]
    fn released_remote_view_materializes_inert_browser_and_session() {
        let raw = json!({
            "profiles": {
                "bill-soylei": { "id": "bill-soylei", "name": "bill-soylei" }
            },
            "displayAllocations": {
                "display:dashboard": {
                    "id": "display:dashboard",
                    "bootEpoch": "legacy-boot-epoch",
                    "ownerBrowserId": "session:dashboard-service-backend",
                    "ownerSessionId": "dashboard-service-backend",
                    "profileId": "bill-soylei",
                    "state": "released",
                    "pidHints": { "browserPid": 34544 },
                    "readiness": {
                        "state": "released",
                        "reason": "operator_requested_close"
                    }
                },
                "display:dashboard-duplicate": {
                    "id": "display:dashboard-duplicate",
                    "ownerBrowserId": "session:dashboard-service-backend",
                    "ownerSessionId": "dashboard-service-backend",
                    "profileId": "bill-soylei",
                    "state": "released",
                    "readiness": {
                        "state": "released",
                        "reason": "duplicate_browser_record_merged"
                    }
                },
                "display:dashboard-parked": {
                    "id": "display:dashboard-parked",
                    "ownerBrowserId": "session:dashboard-service-backend",
                    "ownerSessionId": "dashboard-service-backend",
                    "profileId": "bill-soylei",
                    "state": "released",
                    "readiness": {
                        "state": "released",
                        "reason": "route_switch_parking"
                    }
                }
            },
            "remoteViewRoutes": {
                "guacamole:parked": {
                    "id": "guacamole:parked",
                    "browserId": "session:dashboard-service-backend",
                    "sessionId": "dashboard-service-backend",
                    "state": "released",
                    "viewerLeaseIds": [],
                    "controllerLeaseId": null,
                    "controllerEpoch": 4,
                    "lastProviderEvent": "route_released_after_browser_close"
                }
            }
        })
        .to_string();

        let mut state = read_service_state(&raw).unwrap();
        let profile_identity_digest = "1".repeat(64);
        let owner = crate::runtime_owner_transfer::ProfileOwner {
            owner_id: "owner-dashboard-service-backend".to_string(),
            profile_identity_digest: profile_identity_digest.clone(),
            state: crate::runtime_owner_transfer::ProfileOwnerState::Ready,
            owner_generation: 2,
            browser_id: "session:dashboard-service-backend".to_string(),
            daemon_session_route: "dashboard-service-backend".to_string(),
            process_instance_digest: "2".repeat(64),
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "3".repeat(64),
            target_set_digest: "4".repeat(64),
            pending_transfer: None,
            last_transition: None,
        };
        state.runtime_owner_registry =
            crate::runtime_owner_transfer::RuntimeOwnerRegistry::from_owner(owner);
        let registered = crate::native::service_principal::register_profile_capability(
            &mut state.service_principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "agent-browser-install".to_string(),
                display_name: Some("Agent Browser installer".to_string()),
                profile_id: "bill-soylei".to_string(),
                registered_at: Some("2026-08-28T00:00:00Z".to_string()),
                registered_by: Some("operator".to_string()),
            },
            "synthetic-installer-capability-more-than-thirty-two-characters",
        )
        .unwrap();
        state
            .runtime_owner_registry
            .bind_principal_authority(
                crate::runtime_owner_transfer::RuntimeOwnerPrincipalBinding {
                    principal_id: registered.principal.principal_id,
                    profile_id: registered.capability.profile_id,
                    profile_identity_digest: profile_identity_digest.clone(),
                    capability_id: registered.capability.capability_id,
                    provenance: crate::native::service_principal::ServicePrincipalProvenance::RegisteredCapability,
                    owner_generation: 2,
                },
            )
            .unwrap();
        crate::runtime_owner_transfer::edit_registry_fixture(&mut state.runtime_owner_registry)
            .principal_records
            .get_mut(&profile_identity_digest)
            .unwrap()
            .owner_generation = 1;
        let raw = serde_json::to_string(&state).unwrap();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["browsers"]["session:dashboard-service-backend"]["health"],
            "not_started"
        );
        assert_eq!(
            migrated["sessions"]["dashboard-service-backend"]["lease"],
            "released"
        );
        assert_eq!(
            migrated["runtimeOwnerRegistry"]["principalBindings"][&profile_identity_digest]
                ["ownerGeneration"],
            1
        );
    }

    #[test]
    fn orphaned_route_only_materializes_inert_browser_and_session() {
        let raw = json!({
            "remoteViewRoutes": {
                "guacamole:1": {
                    "id": "guacamole:1",
                    "browserId": "session:retired-browser",
                    "sessionId": "retired-session",
                    "state": "orphaned",
                    "viewerLeaseIds": [],
                    "controllerLeaseId": null,
                    "controllerEpoch": 0
                }
            }
        })
        .to_string();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["browsers"]["session:retired-browser"]["health"],
            "not_started"
        );
        assert_eq!(
            migrated["sessions"]["retired-session"]["browserIds"],
            json!(["session:retired-browser"])
        );
    }

    #[test]
    fn orphaned_route_with_historical_controller_epoch_materializes_inert_browser() {
        let raw = json!({
            "profiles": {
                "managed-one-time": { "id": "managed-one-time", "name": "managed-one-time" }
            },
            "displayAllocations": {
                "display:shared_display:12": {
                    "id": "display:shared_display:12",
                    "ownerBrowserId": "session:retired-browser",
                    "ownerSessionId": "retired-session",
                    "profileId": "managed-one-time",
                    "state": "orphaned",
                    "readiness": {
                        "state": "orphaned",
                        "reason": "owner_browser_not_ready"
                    }
                }
            },
            "remoteViewRoutes": {
                "guacamole:3": {
                    "id": "guacamole:3",
                    "browserId": "session:retired-browser",
                    "sessionId": "retired-session",
                    "displayAllocationId": "display:shared_display:12",
                    "state": "orphaned",
                    "viewerLeaseIds": [],
                    "controllerLeaseId": null,
                    "controllerEpoch": 4,
                    "lastProviderEvent": "display_allocation_unavailable"
                }
            }
        })
        .to_string();

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["browsers"]["session:retired-browser"]["health"],
            "not_started"
        );
        assert_eq!(
            migrated["sessions"]["retired-session"]["browserIds"],
            json!(["session:retired-browser"])
        );
    }

    #[test]
    fn orphaned_route_with_current_controller_lease_remains_a_migration_blocker() {
        let raw = json!({
            "profiles": {
                "managed-one-time": { "id": "managed-one-time", "name": "managed-one-time" }
            },
            "displayAllocations": {
                "display:shared_display:12": {
                    "id": "display:shared_display:12",
                    "ownerBrowserId": "session:retired-browser",
                    "ownerSessionId": "retired-session",
                    "profileId": "managed-one-time",
                    "state": "orphaned",
                    "readiness": {
                        "state": "orphaned",
                        "reason": "owner_browser_not_ready"
                    }
                }
            },
            "remoteViewRoutes": {
                "guacamole:3": {
                    "id": "guacamole:3",
                    "browserId": "session:retired-browser",
                    "sessionId": "retired-session",
                    "displayAllocationId": "display:shared_display:12",
                    "state": "orphaned",
                    "viewerLeaseIds": [],
                    "controllerLeaseId": "controller-current",
                    "controllerEpoch": 4,
                    "lastProviderEvent": "display_allocation_unavailable"
                }
            }
        })
        .to_string();

        let error = stage_service_state_migration(&raw).unwrap_err();
        assert_eq!(
            error,
            "service_state_display_browser_missing:display:shared_display:12:session:retired-browser"
        );
    }

    #[test]
    fn broken_references_fail_before_staged_bytes_exist() {
        let raw = json!({
            "sessions": {
                "odollo": {
                    "id": "odollo",
                    "browserIds": ["missing-browser"],
                    "lease": "exclusive"
                }
            }
        })
        .to_string();
        assert!(stage_service_state_migration(&raw)
            .unwrap_err()
            .contains("service_state_session_browser_missing"));
    }

    #[test]
    fn preview_reports_exact_ids_contamination_and_preserves_successor_fields() {
        let raw = json!({
            "profiles": {
                "contractor": {"id": "contractor", "name": "Contractor"}
            },
            "browsers": {
                "browser-cdp": {
                    "id": "browser-cdp",
                    "futureBrowserEvidence": {"source": "successor"},
                    "recordProvenance": {
                        "source": "persisted_state",
                        "authoritySource": "legacy_unproven",
                        "lifecycleClassification": "inert_legacy",
                        "recommendedAction": "retire",
                        "recordRevision": 2,
                        "evidenceDigest": "evidence",
                        "futureProvenanceField": "preserve"
                    }
                }
            },
            "successorActionRecords": {
                "future-action": {"actionType": "future_exact_repair", "effectCapable": false}
            }
        })
        .to_string();
        let preview = preview_service_state_migration(
            &raw,
            Path::new("/runtime/service/state.json"),
            Path::new("/runtime/transaction-artifacts"),
        )
        .unwrap();
        assert!(!preview.mutation);
        assert!(!preview.backup_created);
        assert_eq!(preview.contamination_report["defaultEffect"], "none");
        assert_eq!(
            preview.contamination_report["inertBrowserIds"],
            json!(["browser-cdp"])
        );
        assert!(preview.summary.protected_record_removals.is_empty());
        assert_eq!(
            preview.summary.unknown_top_level_fields_preserved,
            vec!["successorActionRecords".to_string()]
        );

        let staged = stage_service_state_migration(&raw).unwrap();
        let migrated: Value = serde_json::from_slice(&staged.bytes).unwrap();
        assert_eq!(
            migrated["successorActionRecords"]["future-action"]["actionType"],
            "future_exact_repair"
        );
        assert_eq!(
            migrated["browsers"]["browser-cdp"]["futureBrowserEvidence"]["source"],
            "successor"
        );
        assert_eq!(
            migrated["browsers"]["browser-cdp"]["recordProvenance"]["futureProvenanceField"],
            "preserve"
        );
    }

    #[test]
    fn recovery_artifact_reader_is_forward_backward_and_mixed_version_safe() {
        let current = json!({
            "schemaVersion": super::super::service_profile_acquisition::PROFILE_RECOVERY_PLAN_SCHEMA_V1,
            "actions": [{"actionType": "retire_inert_browser_record"}],
        })
        .to_string();
        let current_result = read_recovery_artifact_compatibility(&current).unwrap();
        assert_eq!(current_result.reader_mode, "current");
        assert!(current_result.effect_capable);

        let legacy = r#"{"schemaVersion":"legacy.profile-recovery-plan.v0","legacyField":true}"#;
        let legacy_before = legacy.as_bytes().to_vec();
        let legacy_result = read_recovery_artifact_compatibility(legacy).unwrap();
        assert_eq!(legacy_result.reader_mode, "legacy_preserve_only");
        assert!(!legacy_result.effect_capable);
        assert_eq!(legacy.as_bytes(), legacy_before);

        let future = json!({
            "schemaVersion": "agent-browser.profile-recovery-plan.v2",
            "futureSeal": {"algorithm": "successor"},
            "actions": [
                {"actionType": "retire_inert_browser_record"},
                {"actionType": "future_exact_repair", "futureGuard": true}
            ]
        })
        .to_string();
        let future_before = future.as_bytes().to_vec();
        let future_result = read_recovery_artifact_compatibility(&future).unwrap();
        assert_eq!(future_result.reader_mode, "future_preserve_only");
        assert_eq!(
            future_result.unknown_action_types,
            vec!["future_exact_repair".to_string()]
        );
        assert!(!future_result.effect_capable);
        assert!(future_result.preserved);
        assert_eq!(future.as_bytes(), future_before);
    }
}
