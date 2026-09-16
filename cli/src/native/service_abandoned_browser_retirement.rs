//! Sealed, provider-free retirement of one abandoned service-owned browser.
//!
//! Planning, reservation and finalization are pure transformations. Callers
//! persist reservation before invoking the effect adapter, never from a
//! replayable repository mutation. A repeated reservation requires recovery;
//! it does not authorize repeating an uncertain process effect.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use super::service_model::{BrowserHealth, ServiceState, TabLifecycle};
use super::service_resources::{
    classify_abandoned_browser_lane_with_profile_identity_at, AbandonedBrowserLaneDecision,
    ProcessSample, ResourceRetirementPolicy,
};
use crate::process_identity::RecordedProcessIdentity;
use crate::runtime_owner_transfer::{
    CleanupObligationState, OwnerAuthorityClaim, RuntimeLaneLifecycleState,
};

const PLAN_SCHEMA: &str = "agent-browser.abandoned-browser-retirement-plan.v1";

/// External observation prepared before entering a replayable mutation.
#[derive(Debug, Clone)]
pub(crate) struct RetirementObservation {
    pub(crate) processes: Vec<ProcessSample>,
    pub(crate) profile_identity_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AbandonedBrowserRetirementPlan {
    pub(crate) schema_version: String,
    pub(crate) plan_id: String,
    pub(crate) state_revision: u64,
    pub(crate) browser_id: String,
    pub(crate) browser_record_digest: String,
    pub(crate) root: RecordedProcessIdentity,
    pub(crate) descendants: Vec<RecordedProcessIdentity>,
    pub(crate) process_group_id: u32,
    pub(crate) profile_path: String,
    pub(crate) profile_id: Option<String>,
    pub(crate) profile_identity_digest: String,
    pub(crate) owner_generation: u64,
    pub(crate) owner_digest: String,
    pub(crate) package_launch_identity_digest: String,
    pub(crate) activity_digest: String,
    pub(crate) policy: ResourceRetirementPolicy,
    pub(crate) created_at: String,
    pub(crate) expires_at: String,
    pub(crate) expected_terminal: RetirementTerminalProjection,
}

/// Exact public-state postcondition. Profile records and profile files survive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RetirementTerminalProjection {
    pub(crate) browser_health: BrowserHealth,
    pub(crate) lifecycle_state: RuntimeLaneLifecycleState,
    pub(crate) cleanup_obligation_state: CleanupObligationState,
    pub(crate) detached_session_ids: Vec<String>,
    pub(crate) closed_tab_ids: Vec<String>,
    pub(crate) released_display_allocation_ids: Vec<String>,
    pub(crate) released_route_ids: Vec<String>,
    pub(crate) released_viewer_lease_ids: Vec<String>,
    pub(crate) released_acquisition_lease_ids: Vec<String>,
    pub(crate) released_route_pool_entry_ids: Vec<String>,
    pub(crate) preserved_profile_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AbandonedBrowserRetirementTransaction {
    pub(crate) plan: AbandonedBrowserRetirementPlan,
    pub(crate) reserved_revision: u64,
    pub(crate) reserved_browser_digest: String,
    pub(crate) receipt: Option<AbandonedBrowserRetirementReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AbandonedBrowserRetirementReceipt {
    pub(crate) plan_id: String,
    pub(crate) browser_id: String,
    pub(crate) completed_at: String,
    pub(crate) terminal_revision: u64,
    pub(crate) effect_evidence: RetirementExitEvidence,
    pub(crate) terminal_projection: RetirementTerminalProjection,
}

/// Adapter-observed proof, tied to the exact reservation and process group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RetirementExitEvidence {
    pub(crate) plan_id: String,
    pub(crate) reserved_revision: u64,
    pub(crate) process_group_id: u32,
    pub(crate) observed_at: String,
    pub(crate) root_exited: bool,
    pub(crate) descendants_exited: bool,
    pub(crate) process_group_empty: bool,
    pub(crate) profile_lock_released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub(crate) enum RetirementRecourse {
    InvalidPlan,
    Expired,
    StateRevisionChanged,
    BrowserRecordChanged,
    RootChanged,
    DescendantsChanged,
    ProcessGroupChanged,
    ProfileChanged,
    OwnerChanged,
    ActivityChanged,
    Ineligible(String),
    ReservationMissing,
    RecoveryRequired,
    ExitUnproven,
    TerminalCompareAndSwapFailed,
    ObservationFailed(String),
}

impl std::fmt::Display for RetirementRecourse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "abandoned_browser_retirement:{self:?}")
    }
}

type RetirementResult<T> = Result<T, RetirementRecourse>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RetirementReservation {
    Reserved { revision: u64 },
    AlreadyReserved,
    Completed(AbandonedBrowserRetirementReceipt),
}

fn digest(value: &impl Serialize) -> RetirementResult<String> {
    let bytes = serde_json::to_vec(value).map_err(|_| RetirementRecourse::InvalidPlan)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn timestamp(value: &str) -> RetirementResult<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|time| time.timestamp())
        .map_err(|_| RetirementRecourse::InvalidPlan)
}

fn seal(plan: &AbandonedBrowserRetirementPlan) -> RetirementResult<String> {
    let mut unsigned = plan.clone();
    unsigned.plan_id.clear();
    digest(&unsigned)
}

fn validate_plan(plan: &AbandonedBrowserRetirementPlan, now: &str) -> RetirementResult<()> {
    if plan.schema_version != PLAN_SCHEMA || seal(plan)? != plan.plan_id {
        return Err(RetirementRecourse::InvalidPlan);
    }
    let now = timestamp(now)?;
    if timestamp(&plan.created_at)? > now || now >= timestamp(&plan.expires_at)? {
        return Err(RetirementRecourse::Expired);
    }
    Ok(())
}

fn process_identity(sample: &ProcessSample) -> RetirementResult<RecordedProcessIdentity> {
    let start_token = sample
        .start_token
        .clone()
        .filter(|v| !v.is_empty())
        .ok_or(RetirementRecourse::RootChanged)?;
    let executable_path = sample
        .executable
        .clone()
        .filter(|v| !v.is_empty())
        .ok_or(RetirementRecourse::RootChanged)?;
    Ok(RecordedProcessIdentity {
        pid: sample.pid,
        start_token,
        executable_path: Some(executable_path),
        browser_family: None,
    })
}

fn physical_identity(recorded: &RecordedProcessIdentity) -> RecordedProcessIdentity {
    let mut identity = recorded.clone();
    identity.browser_family = None;
    identity
}

/// Collect a closed descendant set and reject unrelated members of the group.
fn descendants(
    root: &ProcessSample,
    samples: &[ProcessSample],
) -> RetirementResult<Vec<RecordedProcessIdentity>> {
    if samples
        .iter()
        .map(|sample| sample.pid)
        .collect::<BTreeSet<_>>()
        .len()
        != samples.len()
    {
        return Err(RetirementRecourse::DescendantsChanged);
    }
    let mut pids = BTreeSet::from([root.pid]);
    loop {
        let before = pids.len();
        for sample in samples {
            if sample.ppid.is_some_and(|ppid| pids.contains(&ppid)) {
                pids.insert(sample.pid);
            }
        }
        if pids.len() == before {
            break;
        }
    }
    if samples.iter().any(|sample| {
        (pids.contains(&sample.pid) && sample.process_group_id != root.process_group_id)
            || (sample.process_group_id == root.process_group_id && !pids.contains(&sample.pid))
    }) {
        return Err(RetirementRecourse::ProcessGroupChanged);
    }
    let mut result = samples
        .iter()
        .filter(|sample| sample.pid != root.pid && pids.contains(&sample.pid))
        .map(process_identity)
        .collect::<RetirementResult<Vec<_>>>()?;
    result.sort_by_key(|identity| identity.pid);
    Ok(result)
}

fn terminal_projection(
    state: &ServiceState,
    browser_id: &str,
) -> RetirementResult<RetirementTerminalProjection> {
    let browser = state
        .browsers
        .get(browser_id)
        .ok_or(RetirementRecourse::BrowserRecordChanged)?;
    let released_display_allocation_ids = state
        .display_allocations
        .iter()
        .filter(|(id, allocation)| {
            browser.display_allocation_id.as_deref() == Some(id.as_str())
                || allocation.owner_browser_id.as_deref() == Some(browser_id)
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let released_route_ids = state
        .remote_view_routes
        .iter()
        .filter(|(_, route)| {
            route.browser_id.as_deref() == Some(browser_id)
                || route
                    .display_allocation_id
                    .as_ref()
                    .is_some_and(|id| released_display_allocation_ids.contains(id))
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    Ok(RetirementTerminalProjection {
        browser_health: BrowserHealth::ProcessExited,
        lifecycle_state: RuntimeLaneLifecycleState::Terminal,
        cleanup_obligation_state: CleanupObligationState::Satisfied,
        detached_session_ids: state
            .sessions
            .iter()
            .filter(|(_, session)| session.browser_ids.iter().any(|id| id == browser_id))
            .map(|(id, _)| id.clone())
            .collect(),
        closed_tab_ids: state
            .tabs
            .iter()
            .filter(|(_, tab)| tab.browser_id == browser_id)
            .map(|(id, _)| id.clone())
            .collect(),
        released_display_allocation_ids,
        released_route_ids: released_route_ids.clone(),
        released_viewer_lease_ids: state
            .viewer_leases
            .iter()
            .filter(|(_, lease)| {
                lease.browser_id.as_deref() == Some(browser_id)
                    || lease
                        .route_id
                        .as_ref()
                        .is_some_and(|id| released_route_ids.contains(id))
            })
            .map(|(id, _)| id.clone())
            .collect(),
        released_acquisition_lease_ids: state
            .remote_view_acquisition_leases
            .iter()
            .filter(|(_, lease)| {
                lease.browser_id == browser_id || released_route_ids.contains(&lease.route_id)
            })
            .map(|(id, _)| id.clone())
            .collect(),
        released_route_pool_entry_ids: state
            .route_pool
            .iter()
            .filter(|(_, entry)| released_route_ids.contains(&entry.route_id))
            .map(|(id, _)| id.clone())
            .collect(),
        preserved_profile_digest: digest(
            &browser
                .profile_id
                .as_ref()
                .and_then(|id| state.profiles.get(id)),
        )?,
    })
}

/// Build a deterministic sealed plan from one complete external observation.
pub(crate) fn plan_abandoned_browser_retirement(
    state: &ServiceState,
    browser_id: &str,
    observed: &RetirementObservation,
    policy: &ResourceRetirementPolicy,
    now: &str,
    expires_at: &str,
) -> RetirementResult<AbandonedBrowserRetirementPlan> {
    if timestamp(expires_at)? <= timestamp(now)? {
        return Err(RetirementRecourse::InvalidPlan);
    }
    let browser = state
        .browsers
        .get(browser_id)
        .ok_or(RetirementRecourse::BrowserRecordChanged)?;
    let samples = observed.processes.as_slice();
    let root = samples
        .iter()
        .find(|sample| Some(sample.pid) == browser.pid)
        .ok_or(RetirementRecourse::RootChanged)?;
    let observation = match classify_abandoned_browser_lane_with_profile_identity_at(
        state,
        root,
        samples,
        policy,
        now,
        &observed.profile_identity_digest,
    ) {
        AbandonedBrowserLaneDecision::Candidate(observation) => observation,
        AbandonedBrowserLaneDecision::Protected { reason } => {
            return Err(RetirementRecourse::Ineligible(reason))
        }
    };
    if observation.browser_id != browser_id {
        return Err(RetirementRecourse::BrowserRecordChanged);
    }
    let identity = state
        .browser_process_identities
        .get(browser_id)
        .ok_or(RetirementRecourse::RootChanged)?;
    if process_identity(root)? != physical_identity(&identity.process_identity) {
        return Err(RetirementRecourse::RootChanged);
    }
    let owner = state
        .runtime_owner_registry
        .owner(&observation.profile_identity_digest)
        .ok_or(RetirementRecourse::OwnerChanged)?;
    let mut plan = AbandonedBrowserRetirementPlan {
        schema_version: PLAN_SCHEMA.to_string(),
        plan_id: String::new(),
        state_revision: state.state_revision,
        browser_id: browser_id.to_string(),
        browser_record_digest: digest(browser)?,
        root: identity.process_identity.clone(),
        descendants: descendants(root, samples)?,
        process_group_id: observation.process_group_id,
        profile_path: identity
            .user_data_dir
            .clone()
            .ok_or(RetirementRecourse::ProfileChanged)?,
        profile_id: browser.profile_id.clone(),
        profile_identity_digest: observation.profile_identity_digest,
        owner_generation: observation.owner_generation,
        owner_digest: digest(owner)?,
        package_launch_identity_digest: observation.package_launch_identity_digest,
        activity_digest: observation.activity_digest,
        policy: policy.clone(),
        created_at: now.to_string(),
        expires_at: expires_at.to_string(),
        expected_terminal: terminal_projection(state, browser_id)?,
    };
    plan.plan_id = seal(&plan)?;
    Ok(plan)
}

fn recheck_identity(
    state: &ServiceState,
    plan: &AbandonedBrowserRetirementPlan,
) -> RetirementResult<()> {
    let identity = state
        .browser_process_identities
        .get(&plan.browser_id)
        .ok_or(RetirementRecourse::RootChanged)?;
    if identity.process_identity != plan.root {
        return Err(RetirementRecourse::RootChanged);
    }
    if identity.user_data_dir.as_deref() != Some(&plan.profile_path)
        || terminal_projection(state, &plan.browser_id)? != plan.expected_terminal
    {
        return Err(RetirementRecourse::ProfileChanged);
    }
    let owner = state
        .runtime_owner_registry
        .owner(&plan.profile_identity_digest)
        .ok_or(RetirementRecourse::OwnerChanged)?;
    if digest(owner)? != plan.owner_digest || owner.browser_id != plan.browser_id {
        return Err(RetirementRecourse::OwnerChanged);
    }
    let lifecycle = state
        .runtime_owner_registry
        .lifecycle_records
        .get(&plan.browser_id)
        .ok_or(RetirementRecourse::OwnerChanged)?;
    if lifecycle.logical_browser_id != plan.browser_id
        || lifecycle.owner_generation != plan.owner_generation
        || lifecycle.package_launch_identity_digest.as_ref()
            != Some(&plan.package_launch_identity_digest)
        || lifecycle.profile_identity_digest != plan.profile_identity_digest
    {
        return Err(RetirementRecourse::OwnerChanged);
    }
    if lifecycle.process_group_id != Some(plan.process_group_id) {
        return Err(RetirementRecourse::ProcessGroupChanged);
    }
    Ok(())
}

/// Pure reservation for use inside the existing revision-aware repository CAS.
/// The repository presents the mutator with the candidate revision already
/// advanced by one, and commits that same revision after this function returns.
pub(crate) fn reserve_abandoned_browser_retirement(
    state: &mut ServiceState,
    plan: &AbandonedBrowserRetirementPlan,
    observed: &RetirementObservation,
    now: &str,
) -> RetirementResult<RetirementReservation> {
    validate_plan(plan, now)?;
    if let Some(transaction) = state.abandoned_browser_retirements.get(&plan.plan_id) {
        if transaction.plan != *plan {
            return Err(RetirementRecourse::InvalidPlan);
        }
        return Ok(match &transaction.receipt {
            Some(receipt) => RetirementReservation::Completed(receipt.clone()),
            None => RetirementReservation::AlreadyReserved,
        });
    }
    let revision = plan
        .state_revision
        .checked_add(1)
        .ok_or(RetirementRecourse::StateRevisionChanged)?;
    if state.state_revision != revision {
        return Err(RetirementRecourse::StateRevisionChanged);
    }
    recheck_identity(state, plan)?;
    let browser = state
        .browsers
        .get(&plan.browser_id)
        .ok_or(RetirementRecourse::BrowserRecordChanged)?;
    if digest(browser)? != plan.browser_record_digest {
        return Err(RetirementRecourse::BrowserRecordChanged);
    }
    recheck_processes(plan, &observed.processes)?;
    if observed.profile_identity_digest != plan.profile_identity_digest {
        return Err(RetirementRecourse::ProfileChanged);
    }
    let fresh = plan_abandoned_browser_retirement(
        state,
        &plan.browser_id,
        observed,
        &plan.policy,
        now,
        &plan.expires_at,
    )?;
    compare_observation(plan, &fresh)?;
    let lifecycle = &state.runtime_owner_registry.lifecycle_records[&plan.browser_id];
    if !matches!(
        lifecycle.lifecycle_state,
        RuntimeLaneLifecycleState::Ready | RuntimeLaneLifecycleState::Retained
    ) || lifecycle.cleanup_obligation_state != CleanupObligationState::Owned
    {
        return Err(RetirementRecourse::Ineligible(
            "lane_not_ready_or_retained_owned".to_string(),
        ));
    }
    let claim = OwnerAuthorityClaim::from_owner(
        state
            .runtime_owner_registry
            .owner(&plan.profile_identity_digest)
            .unwrap(),
    );
    let mut registry = state.runtime_owner_registry.clone();
    super::runtime_lifecycle::begin_abandoned_browser_close(&mut registry, claim)
        .map_err(|_| RetirementRecourse::OwnerChanged)?;
    let mut browser = browser.clone();
    browser.health = BrowserHealth::Closing;
    state.runtime_owner_registry = registry;
    state.browsers.insert(plan.browser_id.clone(), browser);
    // Service persistence refreshes browser tab-handle projections before it
    // writes. Seal the same normalized candidate now so the post-commit effect
    // read cannot mistake deterministic derived-view refresh for concurrent
    // browser mutation.
    state.refresh_derived_views();
    let reserved_browser_digest = digest(
        state
            .browsers
            .get(&plan.browser_id)
            .ok_or(RetirementRecourse::BrowserRecordChanged)?,
    )?;
    state.abandoned_browser_retirements.insert(
        plan.plan_id.clone(),
        AbandonedBrowserRetirementTransaction {
            plan: plan.clone(),
            reserved_revision: revision,
            reserved_browser_digest,
            receipt: None,
        },
    );
    Ok(RetirementReservation::Reserved { revision })
}

fn compare_observation(
    plan: &AbandonedBrowserRetirementPlan,
    fresh: &AbandonedBrowserRetirementPlan,
) -> RetirementResult<()> {
    if plan.root != fresh.root {
        return Err(RetirementRecourse::RootChanged);
    }
    if plan.descendants != fresh.descendants {
        return Err(RetirementRecourse::DescendantsChanged);
    }
    if plan.process_group_id != fresh.process_group_id {
        return Err(RetirementRecourse::ProcessGroupChanged);
    }
    if plan.profile_identity_digest != fresh.profile_identity_digest
        || plan.profile_path != fresh.profile_path
    {
        return Err(RetirementRecourse::ProfileChanged);
    }
    if plan.owner_digest != fresh.owner_digest
        || plan.package_launch_identity_digest != fresh.package_launch_identity_digest
    {
        return Err(RetirementRecourse::OwnerChanged);
    }
    if plan.activity_digest != fresh.activity_digest {
        return Err(RetirementRecourse::ActivityChanged);
    }
    Ok(())
}

fn recheck_processes(
    plan: &AbandonedBrowserRetirementPlan,
    samples: &[ProcessSample],
) -> RetirementResult<()> {
    let root = samples
        .iter()
        .find(|sample| sample.pid == plan.root.pid)
        .ok_or(RetirementRecourse::RootChanged)?;
    if process_identity(root)? != physical_identity(&plan.root) {
        return Err(RetirementRecourse::RootChanged);
    }
    if root.process_group_id != Some(plan.process_group_id) {
        return Err(RetirementRecourse::ProcessGroupChanged);
    }
    if descendants(root, samples)? != plan.descendants {
        return Err(RetirementRecourse::DescendantsChanged);
    }
    Ok(())
}

/// New claims must not race a pending exact-profile retirement reservation.
/// Failed effects retain this fence until explicit recovery or finalization.
pub(crate) fn blocks_profile_claim(
    state: &ServiceState,
    resource: &agent_browser_lease_authority::LeaseResourceKey,
) -> bool {
    resource.kind == agent_browser_lease_authority::LeaseResourceKind::Profile
        && state
            .abandoned_browser_retirements
            .values()
            .any(|transaction| {
                transaction.receipt.is_none()
                    && transaction.plan.profile_id.as_deref() == Some(resource.id.as_str())
            })
}

/// Validate a fresh observation immediately before each external signal.
pub(crate) fn revalidate_abandoned_browser_retirement(
    state: &ServiceState,
    plan: &AbandonedBrowserRetirementPlan,
    observed: &RetirementObservation,
    now: &str,
) -> RetirementResult<()> {
    validate_plan(plan, now)?;
    let transaction = state
        .abandoned_browser_retirements
        .get(&plan.plan_id)
        .ok_or(RetirementRecourse::ReservationMissing)?;
    if transaction.receipt.is_some() {
        return Err(RetirementRecourse::RecoveryRequired);
    }
    if transaction.plan != *plan {
        return Err(RetirementRecourse::InvalidPlan);
    }
    if state.state_revision != transaction.reserved_revision {
        return Err(RetirementRecourse::StateRevisionChanged);
    }
    recheck_identity(state, plan)?;
    if digest(&state.browsers[&plan.browser_id])? != transaction.reserved_browser_digest {
        return Err(RetirementRecourse::BrowserRecordChanged);
    }
    let lifecycle = &state.runtime_owner_registry.lifecycle_records[&plan.browser_id];
    if lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Closing
        || lifecycle.cleanup_obligation_state != CleanupObligationState::Owned
    {
        return Err(RetirementRecourse::OwnerChanged);
    }
    recheck_processes(plan, &observed.processes)?;
    if observed.profile_identity_digest != plan.profile_identity_digest {
        return Err(RetirementRecourse::ProfileChanged);
    }
    let fresh = plan_abandoned_browser_retirement(
        state,
        &plan.browser_id,
        observed,
        &plan.policy,
        now,
        &plan.expires_at,
    )
    .map_err(|error| match error {
        RetirementRecourse::Ineligible(reason) if reason.contains("profile") => {
            RetirementRecourse::ProfileChanged
        }
        RetirementRecourse::Ineligible(reason) if reason.contains("owner") => {
            RetirementRecourse::OwnerChanged
        }
        RetirementRecourse::Ineligible(_) => RetirementRecourse::ActivityChanged,
        other => other,
    })?;
    compare_observation(plan, &fresh)
}

/// Pure terminal mutation; an external proof never overrides a stale CAS.
pub(crate) fn finalize_abandoned_browser_retirement(
    state: &mut ServiceState,
    plan: &AbandonedBrowserRetirementPlan,
    evidence: &RetirementExitEvidence,
    now: &str,
) -> RetirementResult<AbandonedBrowserRetirementReceipt> {
    if seal(plan)? != plan.plan_id {
        return Err(RetirementRecourse::InvalidPlan);
    }
    let transaction = state
        .abandoned_browser_retirements
        .get(&plan.plan_id)
        .ok_or(RetirementRecourse::ReservationMissing)?;
    if transaction.plan != *plan {
        return Err(RetirementRecourse::InvalidPlan);
    }
    if let Some(receipt) = &transaction.receipt {
        return Ok(receipt.clone());
    }
    let terminal_revision = transaction
        .reserved_revision
        .checked_add(1)
        .ok_or(RetirementRecourse::TerminalCompareAndSwapFailed)?;
    if state.state_revision != terminal_revision {
        return Err(RetirementRecourse::TerminalCompareAndSwapFailed);
    }
    recheck_identity(state, plan)?;
    if digest(&state.browsers[&plan.browser_id])? != transaction.reserved_browser_digest {
        return Err(RetirementRecourse::TerminalCompareAndSwapFailed);
    }
    if evidence.plan_id != plan.plan_id
        || evidence.reserved_revision != transaction.reserved_revision
        || evidence.process_group_id != plan.process_group_id
        || !evidence.root_exited
        || !evidence.descendants_exited
        || !evidence.process_group_empty
        || !evidence.profile_lock_released
        || timestamp(&evidence.observed_at)? < timestamp(&plan.created_at)?
        || timestamp(&evidence.observed_at)? > timestamp(now)?
    {
        return Err(RetirementRecourse::ExitUnproven);
    }
    if state
        .presentation_capacity
        .as_ref()
        .is_some_and(|capacity| {
            capacity.slots.iter().any(|slot| {
                slot.browser_id.as_deref() == Some(plan.browser_id.as_str())
                    && slot.lease_request_id.is_some()
            })
        })
    {
        return Err(RetirementRecourse::TerminalCompareAndSwapFailed);
    }
    let mut registry = state.runtime_owner_registry.clone();
    super::runtime_lifecycle::complete_reconciled_close(
        &mut registry,
        plan.browser_id.clone(),
        plan.profile_identity_digest.clone(),
        plan.owner_generation,
        vec![
            "exact_process_exited".to_string(),
            "profile_lock_released".to_string(),
            format!("abandoned_retirement:{}", plan.plan_id),
            format!("exit_evidence:{}", digest(evidence)?),
        ],
    )
    .map_err(|_| RetirementRecourse::TerminalCompareAndSwapFailed)?;
    let receipt = AbandonedBrowserRetirementReceipt {
        plan_id: plan.plan_id.clone(),
        browser_id: plan.browser_id.clone(),
        completed_at: now.to_string(),
        terminal_revision,
        effect_evidence: evidence.clone(),
        terminal_projection: plan.expected_terminal.clone(),
    };
    state.runtime_owner_registry = registry;
    let browser = state.browsers.get_mut(&plan.browser_id).unwrap();
    browser.health = BrowserHealth::ProcessExited;
    browser.pid = None;
    browser.cdp_endpoint = None;
    browser.active_session_ids.clear();
    browser.tab_handles.clear();
    browser.view_streams.clear();
    browser.attachability = None;
    browser.display_allocation_id = None;
    browser.display_name = None;
    state.browser_process_identities.remove(&plan.browser_id);
    state
        .protected_browser_owner_observations
        .remove(&plan.browser_id);
    for id in &plan.expected_terminal.detached_session_ids {
        if let Some(session) = state.sessions.get_mut(id) {
            session.browser_ids.retain(|id| id != &plan.browser_id);
            session
                .tab_ids
                .retain(|id| !plan.expected_terminal.closed_tab_ids.contains(id));
        }
    }
    for id in &plan.expected_terminal.closed_tab_ids {
        if let Some(tab) = state.tabs.get_mut(id) {
            tab.lifecycle = TabLifecycle::Closed;
            tab.target_id = None;
            tab.service_tab_handle = None;
        }
    }
    for id in &plan.expected_terminal.released_viewer_lease_ids {
        if let Some(lease) = state.viewer_leases.get_mut(id) {
            lease.state = "released".to_string();
            lease.browser_id = None;
            lease.route_id = None;
            lease.updated_at = Some(now.to_string());
        }
    }
    for id in &plan.expected_terminal.released_route_ids {
        if let Some(route) = state.remote_view_routes.get_mut(id) {
            route.state = "released".to_string();
            route.browser_id = None;
            route.session_id = None;
            route.viewer_lease_ids.clear();
            route.advance_controller(None);
            route.last_provider_event = Some(now.to_string());
        }
    }
    for id in &plan.expected_terminal.released_display_allocation_ids {
        if let Some(allocation) = state.display_allocations.get_mut(id) {
            allocation.state = "released".to_string();
            allocation.owner_browser_id = None;
            allocation.owner_session_id = None;
            allocation.route_ids.clear();
            allocation.updated_at = Some(now.to_string());
        }
    }
    for id in &plan.expected_terminal.released_acquisition_lease_ids {
        if let Some(lease) = state.remote_view_acquisition_leases.get_mut(id) {
            lease.state = "released".to_string();
            lease.phase = "completed".to_string();
            lease.updated_at = Some(now.to_string());
            lease.completed_at = Some(now.to_string());
            lease.cleanup = None;
        }
    }
    for id in &plan.expected_terminal.released_route_pool_entry_ids {
        if let Some(entry) = state.route_pool.get_mut(id) {
            entry.current_route_allocation_id = None;
            entry.state = "ready".to_string();
        }
    }
    if let Some(capacity) = state.presentation_capacity.as_mut() {
        for slot in &mut capacity.slots {
            if slot.browser_id.as_deref() == Some(plan.browser_id.as_str()) {
                slot.browser_id = None;
                slot.state = super::presentation_capacity::PresentationSlotState::WarmIdle;
                slot.lease_priority = None;
                slot.restoration_pending = false;
            }
        }
    }
    state
        .abandoned_browser_retirements
        .get_mut(&plan.plan_id)
        .unwrap()
        .receipt = Some(receipt.clone());
    Ok(receipt)
}

/// External-only adapter. Observation and signaling never occur in CAS closures.
pub(crate) trait AbandonedBrowserRetirementRuntime {
    fn observe(&mut self) -> RetirementResult<(ServiceState, RetirementObservation, String)>;
    fn terminate_exact_tree(
        &mut self,
        plan: &AbandonedBrowserRetirementPlan,
    ) -> RetirementResult<RetirementExitEvidence>;
}

/// Execute once after a newly committed reservation. The adapter must recheck
/// identity and activity before any escalation beyond its first signal.
pub(crate) fn effect_abandoned_browser_retirement(
    plan: &AbandonedBrowserRetirementPlan,
    runtime: &mut impl AbandonedBrowserRetirementRuntime,
) -> RetirementResult<RetirementExitEvidence> {
    let (state, samples, now) = runtime.observe()?;
    revalidate_abandoned_browser_retirement(&state, plan, &samples, &now)?;
    runtime.terminate_exact_tree(plan)
}

#[cfg(test)]
mod tests {
    use super::super::service_model::{
        BrowserProcess, BrowserProfile, BrowserSession, BrowserTab, DisplayAllocation, LeaseState,
        RemoteViewAcquisitionLease, RemoteViewRoute, RoutePoolEntry, ServiceBrowserProcessIdentity,
        SessionCleanupPolicy, ViewerLease,
    };
    use super::*;
    use crate::native::presentation_capacity::{
        PresentationCapacityAuthority, PresentationSlot, PresentationSlotState,
    };
    use crate::runtime_owner_transfer::{
        ProfileOwner, ProfileOwnerState, RuntimeLifecycleRecord, RuntimeOwnerRegistry,
    };

    const NOW: &str = "2026-09-16T12:00:00Z";
    const EXPIRES: &str = "2026-09-16T12:05:00Z";
    const BROWSER: &str = "retirement-fixture";

    fn fixture() -> (ServiceState, RetirementObservation) {
        let profile_digest = agent_browser_lease_authority::canonical_profile_identity_digest(
            std::path::Path::new("/tmp/retirement-fixture"),
        )
        .unwrap();
        let root = RecordedProcessIdentity {
            pid: 4100,
            start_token: "linux:fixture:4100".into(),
            executable_path: Some("/opt/agent-browser/chromium".into()),
            browser_family: Some("chromium".into()),
        };
        let owner = ProfileOwner {
            owner_id: "fixture-owner".into(),
            profile_identity_digest: profile_digest.clone(),
            state: ProfileOwnerState::Ready,
            owner_generation: 4,
            browser_id: BROWSER.into(),
            daemon_session_route: "fixture-session".into(),
            process_instance_digest: super::super::runtime_lifecycle::digest_json(&root).unwrap(),
            browser_family: "chromium".into(),
            cdp_endpoint_identity_digest: "c".repeat(64),
            target_set_digest: "d".repeat(64),
            pending_transfer: None,
            last_transition: None,
        };
        let launch =
            super::super::runtime_lifecycle::package_launch_identity_digest(&owner, Some(4100))
                .unwrap();
        let mut state = ServiceState {
            state_revision: 41,
            runtime_owner_registry: RuntimeOwnerRegistry::from_owner(owner),
            ..ServiceState::default()
        };
        state.runtime_owner_registry.lifecycle_records.insert(
            BROWSER.into(),
            RuntimeLifecycleRecord {
                logical_browser_id: BROWSER.into(),
                profile_identity_digest: profile_digest.clone(),
                owner_generation: 4,
                lifecycle_state: RuntimeLaneLifecycleState::Retained,
                cleanup_obligation_state: CleanupObligationState::Owned,
                process_group_id: Some(4100),
                package_launch_identity_digest: Some(launch),
                ..RuntimeLifecycleRecord::default()
            },
        );
        state.profiles.insert(
            "fixture-profile".into(),
            BrowserProfile {
                id: "fixture-profile".into(),
                profile_class: super::super::service_model::ProfileClass::ManagedOneTime,
                user_data_dir: Some("/tmp/retirement-fixture".into()),
                ..BrowserProfile::default()
            },
        );
        state.browsers.insert(
            BROWSER.into(),
            BrowserProcess {
                id: BROWSER.into(),
                pid: Some(4100),
                profile_id: Some("fixture-profile".into()),
                health: BrowserHealth::Ready,
                active_session_ids: vec!["fixture-session".into()],
                display_allocation_id: Some("fixture-display".into()),
                ..BrowserProcess::default()
            },
        );
        state.display_allocations.insert(
            "fixture-display".into(),
            DisplayAllocation {
                id: "fixture-display".into(),
                owner_browser_id: Some(BROWSER.into()),
                state: "ready".into(),
                route_ids: vec!["fixture-route".into()],
                ..DisplayAllocation::default()
            },
        );
        state.remote_view_routes.insert(
            "fixture-route".into(),
            RemoteViewRoute {
                id: "fixture-route".into(),
                display_allocation_id: Some("fixture-display".into()),
                browser_id: Some(BROWSER.into()),
                state: "ready".into(),
                viewer_lease_ids: vec!["fixture-viewer".into()],
                ..RemoteViewRoute::default()
            },
        );
        state.viewer_leases.insert(
            "fixture-viewer".into(),
            ViewerLease {
                id: "fixture-viewer".into(),
                route_id: Some("fixture-route".into()),
                browser_id: Some(BROWSER.into()),
                state: "released".into(),
                ..ViewerLease::default()
            },
        );
        state.remote_view_acquisition_leases.insert(
            "fixture-acquisition".into(),
            RemoteViewAcquisitionLease {
                id: "fixture-acquisition".into(),
                browser_id: BROWSER.into(),
                route_id: "fixture-route".into(),
                display_allocation_id: "fixture-display".into(),
                route_pool_entry_id: Some("fixture-pool".into()),
                state: "completed".into(),
                phase: "completed".into(),
                ..RemoteViewAcquisitionLease::default()
            },
        );
        state.route_pool.insert(
            "fixture-pool".into(),
            RoutePoolEntry {
                id: "fixture-pool".into(),
                route_id: "fixture-route".into(),
                state: "active".into(),
                current_route_allocation_id: Some("fixture-acquisition".into()),
                ..RoutePoolEntry::default()
            },
        );
        let mut slot = PresentationSlot::warm_idle("fixture-slot");
        slot.state = PresentationSlotState::Active;
        slot.route_id = Some("fixture-route".into());
        slot.display_allocation_id = Some("fixture-display".into());
        slot.browser_id = Some(BROWSER.into());
        state.presentation_capacity = Some(PresentationCapacityAuthority {
            slots: vec![slot],
            ..PresentationCapacityAuthority::default()
        });
        state.browser_process_identities.insert(
            BROWSER.into(),
            ServiceBrowserProcessIdentity {
                process_identity: root,
                user_data_dir: Some("/tmp/retirement-fixture".into()),
                runtime_profile: None,
            },
        );
        state.sessions.insert(
            "fixture-session".into(),
            BrowserSession {
                id: "fixture-session".into(),
                browser_ids: vec![BROWSER.into()],
                tab_ids: vec!["fixture-tab".into()],
                lease: LeaseState::Expired,
                cleanup: SessionCleanupPolicy::CloseBrowser,
                last_lease_observed_at: Some("2026-09-16T10:00:00Z".into()),
                expires_at: Some("2026-09-16T10:05:00Z".into()),
                ..BrowserSession::default()
            },
        );
        state.tabs.insert(
            "fixture-tab".into(),
            BrowserTab {
                id: "fixture-tab".into(),
                browser_id: BROWSER.into(),
                lifecycle: TabLifecycle::Closed,
                ..BrowserTab::default()
            },
        );
        let sample = |pid, ppid| ProcessSample {
            pid,
            ppid: Some(ppid),
            process_group_id: Some(4100),
            start_token: Some(format!("linux:fixture:{pid}")),
            executable: Some("/opt/agent-browser/chromium".into()),
            command: vec![
                "/opt/agent-browser/chromium".into(),
                "--user-data-dir=/tmp/retirement-fixture".into(),
            ],
            ..ProcessSample::default()
        };
        (
            state,
            RetirementObservation {
                processes: vec![sample(4100, 1), sample(4102, 4101), sample(4101, 4100)],
                profile_identity_digest: profile_digest,
            },
        )
    }

    fn plan(
        state: &ServiceState,
        observed: &RetirementObservation,
    ) -> AbandonedBrowserRetirementPlan {
        plan_abandoned_browser_retirement(
            state,
            BROWSER,
            observed,
            &ResourceRetirementPolicy::default(),
            NOW,
            EXPIRES,
        )
        .unwrap()
    }

    fn reserve(
        state: &mut ServiceState,
        plan: &AbandonedBrowserRetirementPlan,
        observed: &RetirementObservation,
    ) {
        state.state_revision += 1; // The real repository pre-advances the candidate revision.
        assert_eq!(
            reserve_abandoned_browser_retirement(state, plan, observed, NOW).unwrap(),
            RetirementReservation::Reserved { revision: 42 }
        );
    }

    fn exit(plan: &AbandonedBrowserRetirementPlan) -> RetirementExitEvidence {
        RetirementExitEvidence {
            plan_id: plan.plan_id.clone(),
            reserved_revision: 42,
            process_group_id: 4100,
            observed_at: NOW.into(),
            root_exited: true,
            descendants_exited: true,
            process_group_empty: true,
            profile_lock_released: true,
        }
    }

    #[test]
    fn retirement_happy_path_preserves_profile_and_terminal_replay() {
        let (mut state, observed) = fixture();
        let profiles = state.profiles.clone();
        let plan = plan(&state, &observed);
        assert_eq!(
            plan.descendants.iter().map(|p| p.pid).collect::<Vec<_>>(),
            vec![4101, 4102]
        );
        reserve(&mut state, &plan, &observed);
        assert_eq!(
            state.runtime_owner_registry.lifecycle_records[BROWSER].lifecycle_state,
            RuntimeLaneLifecycleState::Closing
        );
        revalidate_abandoned_browser_retirement(&state, &plan, &observed, NOW).unwrap();
        assert_eq!(
            reserve_abandoned_browser_retirement(&mut state, &plan, &observed, NOW).unwrap(),
            RetirementReservation::AlreadyReserved
        );
        state.state_revision += 1; // The terminal repository candidate revision.
        let receipt =
            finalize_abandoned_browser_retirement(&mut state, &plan, &exit(&plan), NOW).unwrap();
        assert_eq!(state.profiles, profiles);
        assert_eq!(state.browsers[BROWSER].pid, None);
        assert_eq!(state.browsers[BROWSER].health, BrowserHealth::ProcessExited);
        assert!(state.browsers[BROWSER].active_session_ids.is_empty());
        assert!(state.sessions["fixture-session"].browser_ids.is_empty());
        assert!(!state.browser_process_identities.contains_key(BROWSER));
        assert_eq!(
            state.display_allocations["fixture-display"].state,
            "released"
        );
        assert_eq!(state.remote_view_routes["fixture-route"].state, "released");
        assert_eq!(state.viewer_leases["fixture-viewer"].state, "released");
        assert_eq!(
            state.remote_view_acquisition_leases["fixture-acquisition"].state,
            "released"
        );
        assert_eq!(state.route_pool["fixture-pool"].state, "ready");
        assert_eq!(
            state.presentation_capacity.as_ref().unwrap().slots[0].state,
            PresentationSlotState::WarmIdle
        );
        assert_eq!(
            state.presentation_capacity.as_ref().unwrap().slots[0].browser_id,
            None
        );
        assert_eq!(
            state.runtime_owner_registry.lifecycle_records[BROWSER].cleanup_obligation_state,
            CleanupObligationState::Satisfied
        );
        assert_eq!(
            finalize_abandoned_browser_retirement(&mut state, &plan, &exit(&plan), EXPIRES)
                .unwrap(),
            receipt
        );
        assert_eq!(
            reserve_abandoned_browser_retirement(&mut state, &plan, &observed, NOW).unwrap(),
            RetirementReservation::Completed(receipt)
        );
    }

    #[test]
    fn retirement_rejects_active_explicit_retention_and_incomplete_identity() {
        for case in 0..5 {
            let (mut state, mut observed) = fixture();
            match case {
                0 => {
                    state.sessions.get_mut("fixture-session").unwrap().lease = LeaseState::Exclusive
                }
                1 => {
                    state.sessions.get_mut("fixture-session").unwrap().cleanup =
                        SessionCleanupPolicy::Detach
                }
                2 => {
                    state
                        .profiles
                        .get_mut("fixture-profile")
                        .unwrap()
                        .persistent = true
                }
                3 => observed.processes[1].start_token = None,
                _ => {
                    state
                        .sessions
                        .get_mut("fixture-session")
                        .unwrap()
                        .last_lease_observed_at = None
                }
            }
            assert!(
                plan_abandoned_browser_retirement(
                    &state,
                    BROWSER,
                    &observed,
                    &ResourceRetirementPolicy::default(),
                    NOW,
                    EXPIRES
                )
                .is_err(),
                "case {case}"
            );
        }
    }

    #[test]
    fn retirement_ready_lane_and_census_order_share_the_same_sealed_contract() {
        let (mut state, mut observed) = fixture();
        state
            .runtime_owner_registry
            .lifecycle_records
            .get_mut(BROWSER)
            .unwrap()
            .lifecycle_state = RuntimeLaneLifecycleState::Ready;
        let plan = plan(&state, &observed);
        observed.processes.reverse();
        assert_eq!(
            plan_abandoned_browser_retirement(
                &state,
                BROWSER,
                &observed,
                &plan.policy,
                NOW,
                EXPIRES
            )
            .unwrap(),
            plan
        );
        assert_eq!(
            reserve_abandoned_browser_retirement(&mut state.clone(), &plan, &observed, EXPIRES),
            Err(RetirementRecourse::Expired)
        );
        reserve(&mut state, &plan, &observed);
        assert_eq!(
            revalidate_abandoned_browser_retirement(&state, &plan, &observed, EXPIRES),
            Err(RetirementRecourse::Expired)
        );
    }

    #[test]
    fn retirement_seal_and_reservation_are_exact_and_atomic() {
        let (state, observed) = fixture();
        let original = plan(&state, &observed);
        for case in 0..4 {
            let mut state = state.clone();
            let mut plan = original.clone();
            state.state_revision += 1;
            match case {
                0 => plan.policy.inactivity_minimum_seconds = 0,
                1 => state.state_revision += 1,
                2 => state.browsers.get_mut(BROWSER).unwrap().last_error = Some("changed".into()),
                _ => {
                    state
                        .runtime_owner_registry
                        .lifecycle_records
                        .get_mut(BROWSER)
                        .unwrap()
                        .lifecycle_state = RuntimeLaneLifecycleState::Closing
                }
            }
            let before = state.clone();
            assert!(
                reserve_abandoned_browser_retirement(&mut state, &plan, &observed, NOW).is_err()
            );
            assert_eq!(state, before);
        }
    }

    #[test]
    fn retirement_returns_typed_recourse_for_each_pre_effect_drift() {
        let (mut baseline, observed) = fixture();
        let plan = plan(&baseline, &observed);
        reserve(&mut baseline, &plan, &observed);
        for case in 0..8 {
            let mut state = baseline.clone();
            let mut observation = observed.clone();
            let expected = match case {
                0 => {
                    state.state_revision += 1;
                    RetirementRecourse::StateRevisionChanged
                }
                1 => {
                    observation.processes[0].start_token = Some("reused".into());
                    RetirementRecourse::RootChanged
                }
                2 => {
                    observation.processes.pop();
                    RetirementRecourse::ProcessGroupChanged
                }
                3 => {
                    observation.processes[1].start_token = Some("reused".into());
                    RetirementRecourse::DescendantsChanged
                }
                4 => {
                    observation.processes[0].process_group_id = Some(1);
                    RetirementRecourse::ProcessGroupChanged
                }
                5 => {
                    observation.profile_identity_digest = "different".into();
                    RetirementRecourse::ProfileChanged
                }
                6 => {
                    state
                        .runtime_owner_registry
                        .owners
                        .values_mut()
                        .next()
                        .unwrap()
                        .owner_generation += 1;
                    RetirementRecourse::OwnerChanged
                }
                _ => {
                    state.sessions.get_mut("fixture-session").unwrap().lease =
                        LeaseState::Exclusive;
                    RetirementRecourse::ActivityChanged
                }
            };
            assert_eq!(
                revalidate_abandoned_browser_retirement(&state, &plan, &observation, NOW),
                Err(expected),
                "case {case}"
            );
        }
    }

    #[test]
    fn retirement_terminal_cas_failure_and_residue_keep_cleanup_owned() {
        let (mut state, observed) = fixture();
        let plan = plan(&state, &observed);
        reserve(&mut state, &plan, &observed);
        for case in 0..6 {
            let mut state = state.clone();
            let mut evidence = exit(&plan);
            state.state_revision += 1;
            match case {
                0 => state.state_revision += 1,
                1 => evidence.root_exited = false,
                2 => evidence.descendants_exited = false,
                3 => evidence.process_group_empty = false,
                4 => evidence.profile_lock_released = false,
                _ => {
                    state
                        .runtime_owner_registry
                        .lifecycle_records
                        .get_mut(BROWSER)
                        .unwrap()
                        .owner_generation += 1
                }
            }
            let before = state.clone();
            assert!(
                finalize_abandoned_browser_retirement(&mut state, &plan, &evidence, NOW).is_err()
            );
            assert_eq!(state, before);
        }
    }

    #[test]
    fn retirement_reservation_fences_both_profile_claim_acquisition_paths() {
        use agent_browser_lease_authority::{
            AcquireLeaseClaimRequest, LeaseAuthorityError, LeaseClaimMode, LeaseResourceKey,
        };
        let (mut state, observed) = fixture();
        let plan = plan(&state, &observed);
        let key = LeaseResourceKey::profile("fixture-profile");
        assert!(!blocks_profile_claim(&state, &key));
        reserve(&mut state, &plan, &observed);
        assert!(blocks_profile_claim(&state, &key));
        assert!(!blocks_profile_claim(
            &state,
            &LeaseResourceKey::profile("another-profile")
        ));
        let request = AcquireLeaseClaimRequest {
            resource: key,
            parent_claim_id: None,
            principal_id: "principal".into(),
            capability_id: "capability".into(),
            capability_revision: 1,
            mode: LeaseClaimMode::Ephemeral,
            expected_claim_revision: 0,
            idempotency_key: "test-acquire".into(),
            now: NOW.into(),
            expires_at: EXPIRES.into(),
            transition_deadline: None,
            recovery_controller_id: None,
            boot_epoch: None,
            owner_generation: None,
        };
        assert_eq!(
            state.acquire_lease_claim(request.clone()),
            Err(LeaseAuthorityError::ClaimUnavailable)
        );
        assert_eq!(
            state.acquire_lease_claim_with_receipt(request),
            Err(LeaseAuthorityError::ClaimUnavailable)
        );
        state.state_revision += 1;
        finalize_abandoned_browser_retirement(&mut state, &plan, &exit(&plan), NOW).unwrap();
        assert!(!blocks_profile_claim(
            &state,
            &LeaseResourceKey::profile("fixture-profile")
        ));
    }

    struct FakeRuntime {
        state: ServiceState,
        observed: RetirementObservation,
        signals: usize,
    }

    impl AbandonedBrowserRetirementRuntime for FakeRuntime {
        fn observe(&mut self) -> RetirementResult<(ServiceState, RetirementObservation, String)> {
            Ok((self.state.clone(), self.observed.clone(), NOW.into()))
        }
        fn terminate_exact_tree(
            &mut self,
            plan: &AbandonedBrowserRetirementPlan,
        ) -> RetirementResult<RetirementExitEvidence> {
            self.signals += 1;
            Ok(exit(plan))
        }
    }

    #[test]
    fn retirement_effect_adapter_never_signals_on_revalidation_failure() {
        let (mut state, observed) = fixture();
        let plan = plan(&state, &observed);
        reserve(&mut state, &plan, &observed);
        let mut runtime = FakeRuntime {
            state,
            observed,
            signals: 0,
        };
        runtime.state.state_revision += 1;
        assert_eq!(
            effect_abandoned_browser_retirement(&plan, &mut runtime),
            Err(RetirementRecourse::StateRevisionChanged)
        );
        assert_eq!(runtime.signals, 0);
        runtime.state.state_revision -= 1;
        effect_abandoned_browser_retirement(&plan, &mut runtime).unwrap();
        assert_eq!(runtime.signals, 1);
    }

    #[test]
    fn retirement_repository_cas_uses_pre_advanced_candidate_revisions() {
        use super::super::service_store::{
            JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
            ServiceStateStore,
        };
        use std::time::{SystemTime, UNIX_EPOCH};

        let (state, observed) = fixture();
        let root = std::env::temp_dir().join(format!(
            "agent-browser-abandoned-retirement-cas-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let store = JsonServiceStateStore::new(root.join("state.json"));
        store.save(&state).unwrap();
        let repository = LockedServiceStateRepository::new(store);
        let persisted = repository.load_snapshot().unwrap();
        let plan = plan(&persisted, &observed);

        let reservation = repository
            .mutate(|current| {
                reserve_abandoned_browser_retirement(current, &plan, &observed, NOW)
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        assert_eq!(
            reservation,
            RetirementReservation::Reserved { revision: 42 }
        );
        let reserved = repository.load_snapshot().unwrap();
        assert_eq!(reserved.state_revision, 42);
        revalidate_abandoned_browser_retirement(&reserved, &plan, &observed, NOW).unwrap();

        let receipt = repository
            .mutate(|current| {
                finalize_abandoned_browser_retirement(current, &plan, &exit(&plan), NOW)
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        assert_eq!(receipt.effect_evidence.reserved_revision, 42);
        assert_eq!(receipt.terminal_revision, 43);
        assert_eq!(repository.load_snapshot().unwrap().state_revision, 43);

        std::fs::remove_dir_all(root).unwrap();
    }
}
