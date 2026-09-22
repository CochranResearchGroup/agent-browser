//! Provider-neutral adapter between durable route-keeper authority and protocol effects.
//!
//! This module contains no browser, profile, tab, manager-session, handoff,
//! provider-inventory, or environment authority. A later runtime connector may
//! reuse the Guacamole protocol transport behind this interface.

use agent_browser_service_model::{
    RecordedProcessIdentity, RouteKeeperAdoptionReceipt, RouteKeeperAuthority, RouteKeeperFence,
    RouteKeeperHostProcessClaim, RouteKeeperPhase, RouteKeeperProjection,
    RouteKeeperProtocolReadyReceipt, RouteKeeperReconcileAction, RouteKeeperStopDisposition,
    RouteKeeperStopReceipt,
};
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, watch};

use super::browser_session_store::BrowserRuntimeSqliteStore;
use crate::process_identity::{
    assess_process_ownership, LegacyProfileProof, ProcessObservation, RuntimeProcessOwnership,
};

pub(crate) trait RouteKeeperRepository {
    fn load_route_keeper_authority(&self) -> Result<RouteKeeperAuthority, String>;

    fn compare_and_swap_route_keeper_authority(
        &self,
        expected: &RouteKeeperAuthority,
        next: &RouteKeeperAuthority,
    ) -> Result<(), String>;
}

pub(crate) struct SqliteRouteKeeperRepository {
    path: PathBuf,
}

impl SqliteRouteKeeperRepository {
    pub(crate) fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }
}

impl RouteKeeperRepository for SqliteRouteKeeperRepository {
    fn load_route_keeper_authority(&self) -> Result<RouteKeeperAuthority, String> {
        BrowserRuntimeSqliteStore::open(&self.path)?.load_route_keeper_authority()
    }

    fn compare_and_swap_route_keeper_authority(
        &self,
        expected: &RouteKeeperAuthority,
        next: &RouteKeeperAuthority,
    ) -> Result<(), String> {
        BrowserRuntimeSqliteStore::open(&self.path)?
            .compare_and_swap_route_keeper_authority(expected, next)
    }
}

/// Append or idempotently reuse the exact process claim for the running host.
///
/// Allocation and publication happen against one SQLite authority snapshot.
/// A conflict reloads rather than guessing whether another writer committed.
pub(crate) fn register_route_keeper_host_process(
    repository: &impl RouteKeeperRepository,
    boot_epoch: &str,
    process_identity: RecordedProcessIdentity,
) -> Result<(RouteKeeperAuthority, u64), String> {
    if boot_epoch.trim().is_empty() {
        return Err("route_keeper_host_process_boot_epoch_invalid".to_string());
    }
    for _ in 0..3 {
        let expected = repository.load_route_keeper_authority()?;
        let mut matches = expected
            .host_process_claims
            .iter()
            .filter_map(|(generation, claim)| {
                (claim.boot_epoch == boot_epoch && claim.process_identity == process_identity)
                    .then_some(*generation)
            });
        let existing_generation = matches.next();
        if matches.next().is_some() {
            return Err("route_keeper_host_process_claim_duplicate".to_string());
        }
        let generation = if let Some(generation) = existing_generation {
            generation
        } else {
            expected
                .host_process_claims
                .keys()
                .copied()
                .chain(
                    expected
                        .records
                        .values()
                        .map(|record| record.fence.host_generation),
                )
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| "route_keeper_host_generation_exhausted".to_string())?
        };
        let mut next = expected.clone();
        next.register_host_process_claim(RouteKeeperHostProcessClaim {
            host_generation: generation,
            boot_epoch: boot_epoch.to_string(),
            process_identity: process_identity.clone(),
        })?;
        if next == expected {
            return Ok((expected, generation));
        }
        match repository.compare_and_swap_route_keeper_authority(&expected, &next) {
            Ok(()) => return Ok((next, generation)),
            Err(error) if error == "route_keeper_authority_compare_and_swap_conflict" => {}
            Err(error) => return Err(error),
        }
    }
    Err("route_keeper_host_process_claim_compare_and_swap_exhausted".to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RouteKeeperConnectorObservation {
    Pending,
    Ready(Box<RouteKeeperProtocolReadyReceipt>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RouteKeeperAdoptionObservation {
    Pending,
    Adopted(Box<RouteKeeperAdoptionReceipt>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RouteKeeperStopObservation {
    Stopped(RouteKeeperStopReceipt),
    OwnershipUnproven {
        preserved_observed_keeper_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteKeeperDisconnectEvent {
    pub(crate) slot_id: String,
    pub(crate) fence: RouteKeeperFence,
    pub(crate) guacamole_connection_uuid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteKeeperTerminalEvent {
    pub(crate) slot_id: String,
    pub(crate) keeper_id: String,
    pub(crate) fence: RouteKeeperFence,
    pub(crate) occurrence_id: String,
    pub(crate) guacamole_connection_uuid: Option<String>,
    pub(crate) code: &'static str,
    pub(crate) elapsed_ms: u64,
}

#[async_trait::async_trait]
pub(crate) trait PresentationRouteConnector: Send {
    async fn start(&mut self, action: &RouteKeeperReconcileAction) -> Result<(), String>;

    async fn observe(
        &mut self,
        action: &RouteKeeperReconcileAction,
    ) -> Result<RouteKeeperConnectorObservation, String>;

    async fn adopt(
        &mut self,
        action: &RouteKeeperReconcileAction,
    ) -> Result<RouteKeeperAdoptionObservation, String>;

    async fn stop(
        &mut self,
        action: &RouteKeeperReconcileAction,
    ) -> Result<RouteKeeperStopObservation, String>;
}

#[async_trait::async_trait]
pub(crate) trait SupervisedPresentationRouteConnector: PresentationRouteConnector {
    fn take_terminal_events(&mut self) -> Vec<RouteKeeperTerminalEvent>;

    fn terminal_event_is_current(&self, event: &RouteKeeperTerminalEvent) -> bool;

    fn restore_terminal_event(&mut self, event: RouteKeeperTerminalEvent);

    fn acknowledge_terminal_event(&mut self, event: &RouteKeeperTerminalEvent);

    async fn shutdown_primaries(&mut self);
}

pub(crate) async fn run_route_keeper_supervisor(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl SupervisedPresentationRouteConnector,
    ticks: &mut mpsc::Receiver<()>,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    run_route_keeper_supervisor_with_shutdown_policy(repository, connector, ticks, shutdown, false)
        .await
}

pub(crate) async fn run_configured_route_keeper_supervisor(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl SupervisedPresentationRouteConnector,
    ticks: &mut mpsc::Receiver<()>,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    run_route_keeper_supervisor_with_shutdown_policy(repository, connector, ticks, shutdown, true)
        .await
}

async fn run_route_keeper_supervisor_with_shutdown_policy(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl SupervisedPresentationRouteConnector,
    ticks: &mut mpsc::Receiver<()>,
    shutdown: &mut watch::Receiver<bool>,
    stop_ready_on_shutdown: bool,
) -> Result<(), String> {
    if *shutdown.borrow() {
        return shutdown_and_persist(repository, connector, stop_ready_on_shutdown).await;
    }
    loop {
        let startup = tokio::select! {
            result = reconcile_until_minimum(repository, connector) => Some(result),
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() { None } else { continue }
            }
        };
        match startup {
            Some(Ok(_)) => break,
            Some(Err(error)) => {
                return finish_supervisor_error(
                    repository,
                    connector,
                    error,
                    stop_ready_on_shutdown,
                )
                .await;
            }
            None => {
                return shutdown_and_persist(repository, connector, stop_ready_on_shutdown).await
            }
        }
    }
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return shutdown_and_persist(repository, connector, stop_ready_on_shutdown).await;
                }
            }
            tick = ticks.recv() => {
                if tick.is_none() {
                    return finish_supervisor_error(
                        repository,
                        connector,
                        "route_keeper_supervisor_tick_source_closed".to_string(),
                        stop_ready_on_shutdown,
                    ).await;
                }
                loop {
                    let reconciliation = tokio::select! {
                        result = supervise_once(repository, connector) => Some(result),
                        changed = shutdown.changed() => {
                            if changed.is_err() || *shutdown.borrow() { None } else { continue }
                        }
                    };
                    match reconciliation {
                        Some(Ok(_)) => break,
                        Some(Err(error)) => {
                            return finish_supervisor_error(
                                repository,
                                connector,
                                error,
                                stop_ready_on_shutdown,
                            )
                            .await;
                        }
                        None => {
                            return shutdown_and_persist(repository, connector, stop_ready_on_shutdown).await
                        }
                    }
                }
            }
        }
    }
}

async fn shutdown_and_persist(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl SupervisedPresentationRouteConnector,
    stop_ready: bool,
) -> Result<(), String> {
    let stop_result = if stop_ready {
        stop_ready_routes(repository, connector).await
    } else {
        Ok(())
    };
    connector.shutdown_primaries().await;
    let terminal_result = process_terminal_events(repository, connector);
    match (stop_result, terminal_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(stop), Ok(())) => Err(stop),
        (Ok(()), Err(terminal)) => Err(terminal),
        (Err(stop), Err(terminal)) => Err(format!(
            "route_keeper_supervisor_cleanup_failed:{stop}:{terminal}"
        )),
    }
}

async fn stop_ready_routes(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl SupervisedPresentationRouteConnector,
) -> Result<(), String> {
    loop {
        let authority = repository.load_route_keeper_authority()?;
        let Some(slot_id) = authority
            .records
            .values()
            .find(|record| record.phase == RouteKeeperPhase::Ready)
            .map(|record| record.slot_id.clone())
        else {
            return Ok(());
        };
        stop_once(repository, connector, &slot_id).await?;
        if repository.load_route_keeper_authority()?.records[&slot_id].phase
            == RouteKeeperPhase::Quarantined
        {
            return Err(format!(
                "route_keeper_supervisor_shutdown_stop_unproven:{slot_id}"
            ));
        }
    }
}

async fn finish_supervisor_error(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl SupervisedPresentationRouteConnector,
    error: String,
    stop_ready: bool,
) -> Result<(), String> {
    match shutdown_and_persist(repository, connector, stop_ready).await {
        Ok(()) => Err(error),
        Err(cleanup) => Err(format!(
            "route_keeper_supervisor_cleanup_failed:{error}:{cleanup}"
        )),
    }
}

pub(crate) async fn reconcile_until_minimum(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl SupervisedPresentationRouteConnector,
) -> Result<RouteKeeperProjection, String> {
    process_terminal_events(repository, connector)?;
    let maximum_steps = repository
        .load_route_keeper_authority()?
        .policy
        .maximum_slots
        .saturating_mul(2)
        .saturating_add(1);
    for _ in 0..maximum_steps {
        process_terminal_events(repository, connector)?;
        let before = repository.load_route_keeper_authority()?;
        let before_projection = before.projection()?;
        if before_projection.minimum_satisfied {
            return Ok(before_projection);
        }
        let projection = reconcile_once(repository, connector).await?;
        process_terminal_events(repository, connector)?;
        let after = repository.load_route_keeper_authority()?;
        if after == before {
            return Ok(projection);
        }
    }
    Err("route_keeper_supervisor_minimum_reconcile_exhausted".to_string())
}

pub(crate) async fn supervise_once(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl SupervisedPresentationRouteConnector,
) -> Result<RouteKeeperProjection, String> {
    process_terminal_events(repository, connector)?;
    reconcile_once(repository, connector).await
}

fn process_terminal_events(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl SupervisedPresentationRouteConnector,
) -> Result<(), String> {
    let events = connector.take_terminal_events();
    let mut pending = events.into_iter();
    while let Some(event) = pending.next() {
        if !connector.terminal_event_is_current(&event) {
            connector.acknowledge_terminal_event(&event);
            continue;
        }
        if let Err(error) = record_terminal_event(repository, &event) {
            connector.restore_terminal_event(event);
            for remaining in pending {
                connector.restore_terminal_event(remaining);
            }
            return Err(error);
        }
        connector.acknowledge_terminal_event(&event);
    }
    Ok(())
}

fn record_terminal_event(
    repository: &impl RouteKeeperRepository,
    event: &RouteKeeperTerminalEvent,
) -> Result<RouteKeeperProjection, String> {
    let expected = repository.load_route_keeper_authority()?;
    let Some(record) = expected.records.get(&event.slot_id) else {
        return expected.projection();
    };
    if record.keeper_id != event.keeper_id || record.fence != event.fence {
        return expected.projection();
    }
    let mut next = expected.clone();
    match record.phase {
        agent_browser_service_model::RouteKeeperPhase::Starting
        | agent_browser_service_model::RouteKeeperPhase::Observing => {
            next.record_start_terminated(&event.slot_id, &event.fence)?;
        }
        agent_browser_service_model::RouteKeeperPhase::Ready => {
            let Some(connection_uuid) = event.guacamole_connection_uuid.as_deref() else {
                return expected.projection();
            };
            next.record_disconnect(&event.slot_id, &event.fence, connection_uuid)?;
        }
        agent_browser_service_model::RouteKeeperPhase::Adopting => {
            next.record_adoption_terminated(
                &event.slot_id,
                &event.fence,
                event.guacamole_connection_uuid.as_deref(),
            )?;
        }
        agent_browser_service_model::RouteKeeperPhase::Stopping => {
            let ready = record
                .protocol_ready
                .as_ref()
                .ok_or_else(|| "route_keeper_ready_receipt_missing".to_string())?;
            let preserved_observed_keeper_id = format!(
                "keeper={};guacamole={};xrdp={}",
                record.keeper_id, ready.guacamole_connection_uuid, ready.xrdp_session_id
            );
            next.quarantine_unproven_stop(
                &event.slot_id,
                &event.fence,
                preserved_observed_keeper_id,
            )?;
        }
        _ => return expected.projection(),
    }
    repository.compare_and_swap_route_keeper_authority(&expected, &next)?;
    next.projection()
}

pub(crate) async fn reconcile_once(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl PresentationRouteConnector,
) -> Result<RouteKeeperProjection, String> {
    let expected = repository.load_route_keeper_authority()?;
    let mut reserved = expected.clone();
    let action = reserved.next_reconcile_action()?;
    match &action {
        RouteKeeperReconcileAction::Start { slot_id, fence, .. } => {
            repository.compare_and_swap_route_keeper_authority(&expected, &reserved)?;
            connector.start(&action).await?;
            let mut observing = reserved.clone();
            observing.record_observing(slot_id, fence)?;
            repository.compare_and_swap_route_keeper_authority(&reserved, &observing)?;
            observing.projection()
        }
        RouteKeeperReconcileAction::Observe { .. } => {
            let observation = connector.observe(&action).await?;
            if let RouteKeeperConnectorObservation::Ready(receipt) = &observation {
                require_ready_matches_action(&action, receipt)?;
            }
            apply_ready_observation(repository, expected, observation)
        }
        RouteKeeperReconcileAction::Adopt { .. } => {
            let observation = connector.adopt(&action).await?;
            if let RouteKeeperAdoptionObservation::Adopted(receipt) = &observation {
                require_adoption_matches_action(&action, receipt)?;
            }
            apply_adoption_observation(repository, expected, observation)
        }
        RouteKeeperReconcileAction::Stop { .. } => {
            let observation = connector.stop(&action).await?;
            apply_stop_observation(repository, expected, &action, observation)
        }
        RouteKeeperReconcileAction::Noop => expected.projection(),
    }
}

pub(crate) fn record_transport_disconnect(
    repository: &impl RouteKeeperRepository,
    event: RouteKeeperDisconnectEvent,
) -> Result<RouteKeeperProjection, String> {
    let expected = repository.load_route_keeper_authority()?;
    let mut next = expected.clone();
    next.record_disconnect(
        &event.slot_id,
        &event.fence,
        &event.guacamole_connection_uuid,
    )?;
    repository.compare_and_swap_route_keeper_authority(&expected, &next)?;
    next.projection()
}

pub(crate) async fn adopt_once(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl PresentationRouteConnector,
    slot_id: &str,
    new_host_generation: u64,
) -> Result<RouteKeeperProjection, String> {
    let expected = repository.load_route_keeper_authority()?;
    let mut adopting = expected.clone();
    let action = adopting.begin_adoption(slot_id, new_host_generation)?;
    repository.compare_and_swap_route_keeper_authority(&expected, &adopting)?;
    let observation = connector.adopt(&action).await?;
    if let RouteKeeperAdoptionObservation::Adopted(receipt) = &observation {
        require_adoption_matches_action(&action, receipt)?;
    }
    apply_adoption_observation(repository, adopting, observation)
}

/// Exact predecessor-exit evidence for one retained route.
///
/// Fields are private so persistence alone cannot manufacture this authority.
/// A later runtime-ownership verifier may construct the proof only after it
/// establishes that the predecessor process instance is absent.
#[derive(Clone)]
pub(crate) struct VerifiedRouteKeeperPredecessorExit {
    expected_ready: RouteKeeperProtocolReadyReceipt,
    successor_host_generation: u64,
}

/// Build the opaque predecessor-exit capability from one validated authority
/// snapshot and one caller-supplied process observation.
///
/// This seam is pure and provider-free. The caller owns the OS observation;
/// this function only classifies it against the exact durable process claim.
pub(crate) fn try_prove_route_keeper_predecessor_exit(
    authority: &RouteKeeperAuthority,
    expected_ready: RouteKeeperProtocolReadyReceipt,
    successor_host_generation: u64,
    current_boot_epoch: &str,
    observation: ProcessObservation,
) -> Result<Option<VerifiedRouteKeeperPredecessorExit>, String> {
    authority.projection()?;
    if current_boot_epoch.trim().is_empty() {
        return Err("route_keeper_predecessor_proof_boot_epoch_invalid".to_string());
    }
    let predecessor_generation = expected_ready.fence.host_generation;
    if successor_host_generation <= predecessor_generation {
        return Err("route_keeper_predecessor_proof_successor_generation_invalid".to_string());
    }
    let record = authority
        .records
        .get(&expected_ready.slot_id)
        .ok_or_else(|| "route_keeper_predecessor_proof_candidate_changed".to_string())?;
    if !retained_predecessor_matches_candidate(record, &expected_ready, successor_host_generation)?
    {
        return Err("route_keeper_predecessor_proof_candidate_changed".to_string());
    }
    let predecessor = authority
        .host_process_claims
        .get(&predecessor_generation)
        .ok_or_else(|| {
            format!("route_keeper_host_process_claim_missing:{predecessor_generation}")
        })?;
    let successor = authority
        .host_process_claims
        .get(&successor_host_generation)
        .ok_or_else(|| {
            format!("route_keeper_host_process_claim_missing:{successor_host_generation}")
        })?;
    if successor.boot_epoch != current_boot_epoch {
        return Err("route_keeper_predecessor_proof_successor_boot_epoch_mismatch".to_string());
    }

    let predecessor_exited = if predecessor.boot_epoch != current_boot_epoch {
        true
    } else {
        matches!(
            assess_process_ownership(
                Some(&predecessor.process_identity),
                observation,
                LegacyProfileProof::Unproven,
            )
            .ownership,
            RuntimeProcessOwnership::Missing | RuntimeProcessOwnership::ReusedUnrelated
        )
    };
    Ok(
        predecessor_exited.then_some(VerifiedRouteKeeperPredecessorExit {
            expected_ready,
            successor_host_generation,
        }),
    )
}

/// A cold successor may reconstruct one proof only from the exact retained
/// predecessor receipt. `Ready` and `Degraded` retain that receipt under its
/// original fence. An interrupted adoption retains it under the one
/// deterministic successor fence that `begin_adoption` produces.
fn retained_predecessor_matches_candidate(
    record: &agent_browser_service_model::RouteKeeperRecord,
    expected_ready: &RouteKeeperProtocolReadyReceipt,
    successor_host_generation: u64,
) -> Result<bool, String> {
    if record.keeper_id != expected_ready.keeper_id
        || record.protocol_ready.as_ref() != Some(expected_ready)
    {
        return Ok(false);
    }
    match record.phase {
        RouteKeeperPhase::Ready | RouteKeeperPhase::Degraded => {
            Ok(record.fence == expected_ready.fence)
        }
        RouteKeeperPhase::Adopting => {
            let operation_generation = expected_ready
                .fence
                .operation_generation
                .checked_add(1)
                .ok_or_else(|| "route_keeper_operation_generation_exhausted".to_string())?;
            Ok(record.adoption.is_none()
                && record.fence.host_generation == successor_host_generation
                && record.fence.operation_generation == operation_generation
                && record.fence.operation_id
                    == format!(
                        "route-keeper:{successor_host_generation}:{}:{operation_generation}",
                        expected_ready.slot_id
                    )
                && record.fence.connection_catalog_digest
                    == expected_ready.fence.connection_catalog_digest)
        }
        _ => Ok(false),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RouteKeeperColdRecoveryProgress {
    Pending(RouteKeeperProjection),
    Ready(RouteKeeperProjection),
}

/// Rebinds one proof-bound retained route to its verified successor host.
///
/// The exact predecessor receipt is durably degraded before adoption. An
/// interrupted `Adopting` record replays the same action, while pending
/// observation remains explicitly pending. This function does not claim whole
/// authority recovery, rebase absent slots, or clear quarantine.
pub(crate) async fn recover_proven_cold_process_route(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl PresentationRouteConnector,
    proof: &VerifiedRouteKeeperPredecessorExit,
) -> Result<RouteKeeperColdRecoveryProgress, String> {
    let slot_id = proof.expected_ready.slot_id.as_str();
    let mut expected = repository.load_route_keeper_authority()?;
    require_cold_process_candidate(&expected, proof)?;

    if expected.records[slot_id].phase == RouteKeeperPhase::Ready {
        let mut degraded = expected.clone();
        degraded.record_disconnect(
            slot_id,
            &proof.expected_ready.fence,
            &proof.expected_ready.guacamole_connection_uuid,
        )?;
        compare_and_swap_cold_process_candidate(repository, &expected, &degraded, slot_id)?;
        expected = degraded;
    }

    let action = if expected.records[slot_id].phase == RouteKeeperPhase::Degraded {
        let mut adopting = expected.clone();
        let action = adopting.begin_adoption(slot_id, proof.successor_host_generation)?;
        compare_and_swap_cold_process_candidate(repository, &expected, &adopting, slot_id)?;
        expected = adopting;
        action
    } else {
        let record = &expected.records[slot_id];
        RouteKeeperReconcileAction::Adopt {
            slot_id: record.slot_id.clone(),
            keeper_id: record.keeper_id.clone(),
            fence: record.fence.clone(),
            previous_host_generation: proof.expected_ready.fence.host_generation,
        }
    };

    let observation = connector.adopt(&action).await?;
    if let RouteKeeperAdoptionObservation::Adopted(receipt) = &observation {
        require_adoption_matches_action(&action, receipt)?;
    }
    let pending = matches!(observation, RouteKeeperAdoptionObservation::Pending);
    let projection = apply_adoption_observation(repository, expected, observation)?;
    Ok(if pending {
        RouteKeeperColdRecoveryProgress::Pending(projection)
    } else {
        RouteKeeperColdRecoveryProgress::Ready(projection)
    })
}

fn require_cold_process_candidate(
    authority: &RouteKeeperAuthority,
    proof: &VerifiedRouteKeeperPredecessorExit,
) -> Result<(), String> {
    let slot_id = proof.expected_ready.slot_id.as_str();
    let record = authority
        .records
        .get(slot_id)
        .ok_or_else(|| "route_keeper_slot_missing".to_string())?;
    if proof.successor_host_generation <= proof.expected_ready.fence.host_generation
        || !retained_predecessor_matches_candidate(
            record,
            &proof.expected_ready,
            proof.successor_host_generation,
        )?
    {
        return Err(format!(
            "route_keeper_cold_process_candidate_changed:{slot_id}"
        ));
    }
    Ok(())
}

fn compare_and_swap_cold_process_candidate(
    repository: &impl RouteKeeperRepository,
    expected: &RouteKeeperAuthority,
    next: &RouteKeeperAuthority,
    slot_id: &str,
) -> Result<(), String> {
    match repository.compare_and_swap_route_keeper_authority(expected, next) {
        Err(error) if error == "route_keeper_authority_compare_and_swap_conflict" => Err(format!(
            "route_keeper_cold_process_candidate_changed:{slot_id}"
        )),
        result => result,
    }
}

pub(crate) async fn stop_once(
    repository: &impl RouteKeeperRepository,
    connector: &mut impl PresentationRouteConnector,
    slot_id: &str,
) -> Result<RouteKeeperProjection, String> {
    let expected = repository.load_route_keeper_authority()?;
    let mut stopping = expected.clone();
    let fence = stopping
        .records
        .get(slot_id)
        .ok_or_else(|| "route_keeper_slot_missing".to_string())?
        .fence
        .clone();
    let action = stopping.begin_stop(slot_id, &fence)?;
    repository.compare_and_swap_route_keeper_authority(&expected, &stopping)?;
    let observation = connector.stop(&action).await?;
    apply_stop_observation(repository, stopping, &action, observation)
}

fn apply_ready_observation(
    repository: &impl RouteKeeperRepository,
    expected: RouteKeeperAuthority,
    observation: RouteKeeperConnectorObservation,
) -> Result<RouteKeeperProjection, String> {
    let RouteKeeperConnectorObservation::Ready(receipt) = observation else {
        return expected.projection();
    };
    let mut ready = expected.clone();
    ready.record_protocol_ready(*receipt)?;
    repository.compare_and_swap_route_keeper_authority(&expected, &ready)?;
    ready.projection()
}

fn apply_adoption_observation(
    repository: &impl RouteKeeperRepository,
    expected: RouteKeeperAuthority,
    observation: RouteKeeperAdoptionObservation,
) -> Result<RouteKeeperProjection, String> {
    let RouteKeeperAdoptionObservation::Adopted(receipt) = observation else {
        return expected.projection();
    };
    let mut adopted = expected.clone();
    adopted.adopt(*receipt)?;
    repository.compare_and_swap_route_keeper_authority(&expected, &adopted)?;
    adopted.projection()
}

fn apply_stop_observation(
    repository: &impl RouteKeeperRepository,
    expected: RouteKeeperAuthority,
    action: &RouteKeeperReconcileAction,
    observation: RouteKeeperStopObservation,
) -> Result<RouteKeeperProjection, String> {
    let (slot_id, _, fence) = reconcile_action_identity(action)?;
    let mut next = expected.clone();
    match observation {
        RouteKeeperStopObservation::Stopped(receipt) => {
            require_stop_matches_action(action, &receipt)?;
            let _disposition: RouteKeeperStopDisposition = next.record_stopped(receipt)?;
        }
        RouteKeeperStopObservation::OwnershipUnproven {
            preserved_observed_keeper_id,
        } => {
            next.quarantine_unproven_stop(slot_id, fence, preserved_observed_keeper_id)?;
        }
    }
    repository.compare_and_swap_route_keeper_authority(&expected, &next)?;
    next.projection()
}

fn reconcile_action_identity(
    action: &RouteKeeperReconcileAction,
) -> Result<(&str, &str, &RouteKeeperFence), String> {
    match action {
        RouteKeeperReconcileAction::Start {
            slot_id,
            keeper_id,
            fence,
            ..
        }
        | RouteKeeperReconcileAction::Observe {
            slot_id,
            keeper_id,
            fence,
        }
        | RouteKeeperReconcileAction::Adopt {
            slot_id,
            keeper_id,
            fence,
            ..
        }
        | RouteKeeperReconcileAction::Stop {
            slot_id,
            keeper_id,
            fence,
        } => Ok((slot_id, keeper_id, fence)),
        RouteKeeperReconcileAction::Noop => {
            Err("route_keeper_connector_action_identity_missing".to_string())
        }
    }
}

fn require_ready_matches_action(
    action: &RouteKeeperReconcileAction,
    receipt: &RouteKeeperProtocolReadyReceipt,
) -> Result<(), String> {
    let (slot_id, keeper_id, fence) = reconcile_action_identity(action)?;
    if receipt.slot_id != slot_id || receipt.keeper_id != keeper_id || receipt.fence != *fence {
        return Err("route_keeper_connector_observation_mismatch".to_string());
    }
    Ok(())
}

fn require_adoption_matches_action(
    action: &RouteKeeperReconcileAction,
    receipt: &RouteKeeperAdoptionReceipt,
) -> Result<(), String> {
    require_ready_matches_action(action, &receipt.ready)?;
    let RouteKeeperReconcileAction::Adopt {
        previous_host_generation,
        ..
    } = action
    else {
        return Err("route_keeper_connector_action_identity_missing".to_string());
    };
    if receipt.previous_host_generation != *previous_host_generation {
        return Err("route_keeper_connector_observation_mismatch".to_string());
    }
    Ok(())
}

fn require_stop_matches_action(
    action: &RouteKeeperReconcileAction,
    receipt: &RouteKeeperStopReceipt,
) -> Result<(), String> {
    let (slot_id, _, fence) = reconcile_action_identity(action)?;
    if receipt.slot_id != slot_id || receipt.fence != *fence {
        return Err("route_keeper_connector_observation_mismatch".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::{
        RecordedProcessIdentity, RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog,
        RouteKeeperHostProcessClaim, RouteKeeperPhase, RouteKeeperStartPriority,
        RouteKeeperStopReceipt, RouteKeeperXrdpOwnershipWitness,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use crate::native::browser_session_store::LegacyBrowserRuntimeSources;
    use crate::process_identity::{ObservedProcessIdentity, ProcessObservation};

    fn host_process_claim(host_generation: u64) -> RouteKeeperHostProcessClaim {
        RouteKeeperHostProcessClaim {
            host_generation,
            boot_epoch: "linux:boot:fixture".to_string(),
            process_identity: RecordedProcessIdentity {
                pid: u32::try_from(4_000 + host_generation).unwrap(),
                start_token: format!("linux:start:{host_generation}"),
                executable_path: Some("/opt/agent-browser".to_string()),
                browser_family: None,
            },
        }
    }

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "agent-browser-{label}-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn repository(directory: &TempDirectory) -> SqliteRouteKeeperRepository {
        let database_path = directory.0.join("runtime.sqlite3");
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join("browser-session-state.json"),
                profile_catalog_path: &directory.0.join("browser-profile-catalog.json"),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let original = store.load_route_keeper_authority().unwrap();
        let mut configured = original.clone();
        configured
            .register_host_process_claim(host_process_claim(1))
            .unwrap();
        configured
            .replace_connection_catalog(
                RouteKeeperConnectionCatalog::new((1_u32..=6).map(|sequence| {
                    RouteKeeperConnectionBinding {
                        slot_id: format!("route-slot-{sequence:02}"),
                        connection_key: format!("route-{sequence:02}"),
                        connection_name: format!("Agent Browser Route {sequence:02}"),
                        route_user: format!("agent-browser-rdp-{sequence}"),
                        guacamole_connection_id: u64::from(sequence),
                    }
                }))
                .unwrap(),
            )
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&original, &configured)
            .unwrap();
        SqliteRouteKeeperRepository::new(&database_path)
    }

    fn register_host_claim(
        repository: &SqliteRouteKeeperRepository,
        host_generation: u64,
    ) -> RouteKeeperAuthority {
        let expected = repository.load_route_keeper_authority().unwrap();
        let mut next = expected.clone();
        next.register_host_process_claim(host_process_claim(host_generation))
            .unwrap();
        repository
            .compare_and_swap_route_keeper_authority(&expected, &next)
            .unwrap();
        next
    }

    fn prove_missing_predecessor(
        authority: &RouteKeeperAuthority,
        expected_ready: RouteKeeperProtocolReadyReceipt,
        successor_host_generation: u64,
    ) -> VerifiedRouteKeeperPredecessorExit {
        try_prove_route_keeper_predecessor_exit(
            authority,
            expected_ready,
            successor_host_generation,
            "linux:boot:fixture",
            ProcessObservation::Missing,
        )
        .unwrap()
        .expect("the exact fixture predecessor must be absent")
    }

    #[derive(Default)]
    struct FakeConnector {
        starts: Vec<RouteKeeperReconcileAction>,
        observes: Vec<RouteKeeperReconcileAction>,
        adoptions: Vec<RouteKeeperReconcileAction>,
        adoption_error: Option<String>,
        adoption_pending_once: bool,
        stops: Vec<RouteKeeperReconcileAction>,
        routes: BTreeMap<String, RouteKeeperProtocolReadyReceipt>,
        unproven_stop: Option<String>,
        terminal_events: Vec<RouteKeeperTerminalEvent>,
        current_terminal_occurrences: BTreeMap<String, String>,
        acknowledged_terminal_occurrences: Vec<String>,
        restored_terminal_occurrences: Vec<String>,
        shutdowns: usize,
    }

    impl FakeConnector {
        fn ready_for_action(
            action: &RouteKeeperReconcileAction,
        ) -> RouteKeeperProtocolReadyReceipt {
            let (slot_id, keeper_id, fence) = action_identity(action);
            let xrdp_session_id = format!("xrdp-{slot_id}");
            let display_name = format!(":{}", slot_id.trim_start_matches("route-slot-"));
            RouteKeeperProtocolReadyReceipt {
                slot_id: slot_id.to_string(),
                keeper_id: keeper_id.to_string(),
                fence: fence.clone(),
                guacamole_connection_uuid: format!("connection-{slot_id}"),
                xrdp_session_id: xrdp_session_id.clone(),
                display_name: display_name.clone(),
                xrdp_ownership: Some(RouteKeeperXrdpOwnershipWitness {
                    schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
                    boot_id: "boot-fixture".to_string(),
                    route_user: "agent-browser-rdp-1".to_string(),
                    route_uid: 2001,
                    session_id: xrdp_session_id.clone(),
                    session_service: "xrdp-sesman".to_string(),
                    session_scope: format!("session-{xrdp_session_id}.scope"),
                    scope_invocation_id: "invocation-fixture".to_string(),
                    cgroup_path: format!(
                        "/user.slice/user-2001.slice/session-{xrdp_session_id}.scope"
                    ),
                    cgroup_device: 28,
                    cgroup_inode: 1001,
                    leader_pid: 4101,
                    leader_start_ticks: 5101,
                    x_server_pid: 4102,
                    x_server_start_ticks: 5102,
                    display_name,
                    x11_socket_inode: 6101,
                }),
                observed_at: "2026-09-19T18:00:00Z".to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl PresentationRouteConnector for FakeConnector {
        async fn start(&mut self, action: &RouteKeeperReconcileAction) -> Result<(), String> {
            let receipt = Self::ready_for_action(action);
            self.routes.insert(receipt.slot_id.clone(), receipt);
            self.starts.push(action.clone());
            Ok(())
        }

        async fn observe(
            &mut self,
            action: &RouteKeeperReconcileAction,
        ) -> Result<RouteKeeperConnectorObservation, String> {
            self.observes.push(action.clone());
            let (slot_id, _, fence) = action_identity(action);
            let mut receipt = self
                .routes
                .get(slot_id)
                .cloned()
                .ok_or_else(|| "fixture_route_missing".to_string())?;
            receipt.fence = fence.clone();
            Ok(RouteKeeperConnectorObservation::Ready(Box::new(receipt)))
        }

        async fn adopt(
            &mut self,
            action: &RouteKeeperReconcileAction,
        ) -> Result<RouteKeeperAdoptionObservation, String> {
            self.adoptions.push(action.clone());
            if let Some(error) = self.adoption_error.take() {
                return Err(error);
            }
            if self.adoption_pending_once {
                self.adoption_pending_once = false;
                return Ok(RouteKeeperAdoptionObservation::Pending);
            }
            let (slot_id, _, fence) = action_identity(action);
            let mut ready = self
                .routes
                .get(slot_id)
                .cloned()
                .ok_or_else(|| "fixture_route_missing".to_string())?;
            let previous_host_generation = ready.fence.host_generation;
            let previous_guacamole_connection_uuid = ready.guacamole_connection_uuid.clone();
            ready.fence = fence.clone();
            ready.observed_at = "2026-09-19T18:01:00Z".to_string();
            Ok(RouteKeeperAdoptionObservation::Adopted(Box::new(
                RouteKeeperAdoptionReceipt {
                    previous_host_generation,
                    previous_guacamole_connection_uuid,
                    ready,
                    adopted_at: "2026-09-19T18:01:01Z".to_string(),
                },
            )))
        }

        async fn stop(
            &mut self,
            action: &RouteKeeperReconcileAction,
        ) -> Result<RouteKeeperStopObservation, String> {
            self.stops.push(action.clone());
            if let Some(identity) = &self.unproven_stop {
                return Ok(RouteKeeperStopObservation::OwnershipUnproven {
                    preserved_observed_keeper_id: identity.clone(),
                });
            }
            let (slot_id, keeper_id, fence) = action_identity(action);
            let ready = self
                .routes
                .get(slot_id)
                .ok_or_else(|| "fixture_route_missing".to_string())?;
            Ok(RouteKeeperStopObservation::Stopped(
                RouteKeeperStopReceipt {
                    slot_id: slot_id.to_string(),
                    keeper_id: keeper_id.to_string(),
                    fence: fence.clone(),
                    guacamole_connection_uuid: Some(ready.guacamole_connection_uuid.clone()),
                    xrdp_session_id: Some(ready.xrdp_session_id.clone()),
                    stopped_at: "2026-09-19T18:02:00Z".to_string(),
                },
            ))
        }
    }

    #[async_trait::async_trait]
    impl SupervisedPresentationRouteConnector for FakeConnector {
        fn take_terminal_events(&mut self) -> Vec<RouteKeeperTerminalEvent> {
            std::mem::take(&mut self.terminal_events)
        }

        fn terminal_event_is_current(&self, event: &RouteKeeperTerminalEvent) -> bool {
            self.current_terminal_occurrences
                .get(&event.slot_id)
                .is_some_and(|occurrence| occurrence == &event.occurrence_id)
        }

        fn restore_terminal_event(&mut self, event: RouteKeeperTerminalEvent) {
            self.restored_terminal_occurrences
                .push(event.occurrence_id.clone());
            self.terminal_events.push(event);
        }

        fn acknowledge_terminal_event(&mut self, event: &RouteKeeperTerminalEvent) {
            self.acknowledged_terminal_occurrences
                .push(event.occurrence_id.clone());
        }

        async fn shutdown_primaries(&mut self) {
            self.shutdowns += 1;
        }
    }

    fn action_identity(action: &RouteKeeperReconcileAction) -> (&str, &str, &RouteKeeperFence) {
        match action {
            RouteKeeperReconcileAction::Start {
                slot_id,
                keeper_id,
                fence,
                ..
            }
            | RouteKeeperReconcileAction::Observe {
                slot_id,
                keeper_id,
                fence,
            }
            | RouteKeeperReconcileAction::Adopt {
                slot_id,
                keeper_id,
                fence,
                ..
            }
            | RouteKeeperReconcileAction::Stop {
                slot_id,
                keeper_id,
                fence,
            } => (slot_id, keeper_id, fence),
            RouteKeeperReconcileAction::Noop => panic!("noop has no identity"),
        }
    }

    async fn make_minimum_ready(
        repository: &impl RouteKeeperRepository,
        connector: &mut FakeConnector,
    ) -> RouteKeeperAuthority {
        reconcile_once(repository, connector).await.unwrap();
        reconcile_once(repository, connector).await.unwrap();
        repository.load_route_keeper_authority().unwrap()
    }

    #[tokio::test]
    async fn reconcile_establishes_minimum_before_starting_warm_routes() {
        let directory = TempDirectory::new("route-keeper-reconcile");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();

        for index in 0..4 {
            let starting = reconcile_once(&repository, &mut connector).await.unwrap();
            assert_eq!(starting.ready_count, index);
            let authority = repository.load_route_keeper_authority().unwrap();
            assert_eq!(
                authority.records[&format!("route-slot-{:02}", index + 1)].phase,
                RouteKeeperPhase::Observing
            );
            let ready = reconcile_once(&repository, &mut connector).await.unwrap();
            assert_eq!(ready.ready_count, index + 1);
            assert!(ready.minimum_satisfied);
        }

        assert_eq!(connector.starts.len(), 4);
        assert_eq!(connector.observes.len(), 4);
        assert!(connector.adoptions.is_empty());
        assert!(connector.stops.is_empty());
        assert_eq!(
            connector
                .starts
                .iter()
                .map(|action| match action {
                    RouteKeeperReconcileAction::Start { priority, .. } => *priority,
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>(),
            [
                RouteKeeperStartPriority::Minimum,
                RouteKeeperStartPriority::Warm,
                RouteKeeperStartPriority::Warm,
                RouteKeeperStartPriority::Warm,
            ]
        );
        let final_projection = reconcile_once(&repository, &mut connector).await.unwrap();
        assert_eq!(final_projection.ready_count, 4);
        assert!(final_projection.warm_target_satisfied);
        assert_eq!(connector.starts.len(), 4);
    }

    struct ConflictRepository {
        inner: SqliteRouteKeeperRepository,
        cas_calls: AtomicUsize,
        reject_call: usize,
    }

    impl RouteKeeperRepository for ConflictRepository {
        fn load_route_keeper_authority(&self) -> Result<RouteKeeperAuthority, String> {
            self.inner.load_route_keeper_authority()
        }

        fn compare_and_swap_route_keeper_authority(
            &self,
            expected: &RouteKeeperAuthority,
            next: &RouteKeeperAuthority,
        ) -> Result<(), String> {
            let call = self.cas_calls.fetch_add(1, Ordering::SeqCst) + 1;
            if call == self.reject_call {
                return Err("route_keeper_authority_compare_and_swap_conflict".to_string());
            }
            self.inner
                .compare_and_swap_route_keeper_authority(expected, next)
        }
    }

    struct RebindingRepository {
        inner: SqliteRouteKeeperRepository,
        cas_calls: AtomicUsize,
        replacement: Mutex<Option<RouteKeeperAuthority>>,
    }

    impl RouteKeeperRepository for RebindingRepository {
        fn load_route_keeper_authority(&self) -> Result<RouteKeeperAuthority, String> {
            self.inner.load_route_keeper_authority()
        }

        fn compare_and_swap_route_keeper_authority(
            &self,
            expected: &RouteKeeperAuthority,
            next: &RouteKeeperAuthority,
        ) -> Result<(), String> {
            let call = self.cas_calls.fetch_add(1, Ordering::SeqCst) + 1;
            if call == 1 {
                let current = self.inner.load_route_keeper_authority()?;
                let replacement = self
                    .replacement
                    .lock()
                    .unwrap()
                    .take()
                    .ok_or_else(|| "fixture_replacement_missing".to_string())?;
                self.inner
                    .compare_and_swap_route_keeper_authority(&current, &replacement)?;
                return Err("route_keeper_authority_compare_and_swap_conflict".to_string());
            }
            self.inner
                .compare_and_swap_route_keeper_authority(expected, next)
        }
    }

    #[tokio::test]
    async fn post_start_cas_conflict_recovers_by_observation_without_restarting() {
        let directory = TempDirectory::new("route-keeper-conflict");
        let repository = ConflictRepository {
            inner: repository(&directory),
            cas_calls: AtomicUsize::new(0),
            reject_call: 2,
        };
        let mut connector = FakeConnector::default();

        assert_eq!(
            reconcile_once(&repository, &mut connector).await,
            Err("route_keeper_authority_compare_and_swap_conflict".to_string())
        );
        assert_eq!(connector.starts.len(), 1);
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Starting
        );

        let projection = reconcile_once(&repository, &mut connector).await.unwrap();
        assert_eq!(projection.ready_count, 1);
        assert_eq!(connector.starts.len(), 1);
        assert_eq!(connector.observes.len(), 1);
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"]
                .fence
                .operation_generation,
            1
        );
    }

    #[tokio::test]
    async fn disconnect_is_durable_before_recovery_and_adoption_preserves_exact_resources() {
        let directory = TempDirectory::new("route-keeper-disconnect-adopt");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut connector).await;
        let original = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();

        let degraded = record_transport_disconnect(
            &repository,
            RouteKeeperDisconnectEvent {
                slot_id: original.slot_id.clone(),
                fence: original.fence.clone(),
                guacamole_connection_uuid: original.guacamole_connection_uuid.clone(),
            },
        )
        .unwrap();
        assert_eq!(degraded.ready_count, 0);
        assert!(!degraded.minimum_satisfied);
        assert_eq!(
            degraded.state,
            agent_browser_service_model::RouteKeeperProviderState::Degraded
        );
        assert_eq!(connector.starts.len(), 1);

        register_host_claim(&repository, 2);
        let adopted = adopt_once(&repository, &mut connector, "route-slot-01", 2)
            .await
            .unwrap();
        assert_eq!(adopted.ready_count, 1);
        assert_eq!(connector.starts.len(), 1);
        assert_eq!(connector.adoptions.len(), 1);
        let authority = repository.load_route_keeper_authority().unwrap();
        let current = authority.records["route-slot-01"]
            .protocol_ready
            .as_ref()
            .unwrap();
        assert_eq!(current.fence.host_generation, 2);
        assert_eq!(
            (
                &current.guacamole_connection_uuid,
                &current.xrdp_session_id,
                &current.display_name,
            ),
            (
                &original.guacamole_connection_uuid,
                &original.xrdp_session_id,
                &original.display_name,
            )
        );
    }

    #[tokio::test]
    async fn cold_process_recovery_adopts_retained_ready_route_before_starting_new_work() {
        let directory = TempDirectory::new("route-keeper-cold-process-ready");
        let repository = repository(&directory);
        let mut predecessor = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut predecessor).await;
        let original = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();

        let mut successor = FakeConnector::default();
        successor
            .routes
            .insert(original.slot_id.clone(), original.clone());

        let claimed = register_host_claim(&repository, 2);
        let proof = prove_missing_predecessor(&claimed, original.clone(), 2);
        let RouteKeeperColdRecoveryProgress::Ready(projection) =
            recover_proven_cold_process_route(&repository, &mut successor, &proof)
                .await
                .unwrap()
        else {
            panic!("exact fixture adoption must become ready");
        };

        assert_eq!(projection.ready_count, 1);
        assert_eq!(successor.starts.len(), 0);
        assert_eq!(successor.adoptions.len(), 1);
        let current = repository.load_route_keeper_authority().unwrap();
        let recovered = current.records["route-slot-01"]
            .protocol_ready
            .as_ref()
            .unwrap();
        assert_eq!(recovered.fence.host_generation, 2);
        assert_eq!(
            (
                &recovered.guacamole_connection_uuid,
                &recovered.xrdp_session_id,
                &recovered.display_name,
            ),
            (
                &original.guacamole_connection_uuid,
                &original.xrdp_session_id,
                &original.display_name,
            )
        );
    }

    #[tokio::test]
    async fn predecessor_exit_proof_requires_exact_persisted_host_claim_and_absence() {
        let directory = TempDirectory::new("route-keeper-predecessor-exit-proof");
        let repository = repository(&directory);
        let mut predecessor = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut predecessor).await;
        let original = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let authority = register_host_claim(&repository, 2);
        let recorded = authority.host_process_claims[&1].process_identity.clone();

        assert!(try_prove_route_keeper_predecessor_exit(
            &authority,
            original.clone(),
            2,
            "linux:boot:fixture",
            ProcessObservation::Missing,
        )
        .unwrap()
        .is_some());

        let observed = |start_token: Option<&str>, executable_path: Option<&str>| {
            ProcessObservation::Observed(ObservedProcessIdentity {
                pid: recorded.pid,
                start_token: start_token.map(str::to_string),
                executable_path: executable_path.map(str::to_string),
                browser_family: None,
                command_line: None,
            })
        };
        assert!(try_prove_route_keeper_predecessor_exit(
            &authority,
            original.clone(),
            2,
            "linux:boot:fixture",
            observed(
                Some(&recorded.start_token),
                recorded.executable_path.as_deref()
            ),
        )
        .unwrap()
        .is_none());
        assert!(try_prove_route_keeper_predecessor_exit(
            &authority,
            original.clone(),
            2,
            "linux:boot:fixture",
            observed(
                Some("linux:reused-process"),
                recorded.executable_path.as_deref()
            ),
        )
        .unwrap()
        .is_some());
        assert!(try_prove_route_keeper_predecessor_exit(
            &authority,
            original.clone(),
            2,
            "linux:boot:fixture",
            observed(Some(&recorded.start_token), None),
        )
        .unwrap()
        .is_none());
        assert!(try_prove_route_keeper_predecessor_exit(
            &authority,
            original,
            2,
            "linux:boot:fixture",
            ProcessObservation::Failed {
                reason: "fixture observation failed".to_string(),
            },
        )
        .unwrap()
        .is_none());
    }

    #[tokio::test]
    async fn predecessor_exit_proof_reconstructs_only_exact_retained_candidates() {
        let directory = TempDirectory::new("route-keeper-reconstruct-predecessor-proof");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut connector).await;
        let original = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let authority = register_host_claim(&repository, 2);

        assert!(try_prove_route_keeper_predecessor_exit(
            &authority,
            original.clone(),
            2,
            "linux:boot:fixture",
            ProcessObservation::Missing,
        )
        .unwrap()
        .is_some());

        let mut degraded = authority.clone();
        degraded
            .record_disconnect(
                &original.slot_id,
                &original.fence,
                &original.guacamole_connection_uuid,
            )
            .unwrap();
        assert!(try_prove_route_keeper_predecessor_exit(
            &degraded,
            original.clone(),
            2,
            "linux:boot:fixture",
            ProcessObservation::Missing,
        )
        .unwrap()
        .is_some());

        let action = degraded.begin_adoption("route-slot-01", 2).unwrap();
        let RouteKeeperReconcileAction::Adopt {
            fence,
            previous_host_generation,
            ..
        } = action
        else {
            panic!("fixture must prepare an adoption action");
        };
        assert_eq!(previous_host_generation, original.fence.host_generation);
        assert_eq!(fence.host_generation, 2);
        assert!(try_prove_route_keeper_predecessor_exit(
            &degraded,
            original.clone(),
            2,
            "linux:boot:fixture",
            ProcessObservation::Missing,
        )
        .unwrap()
        .is_some());

        let mut foreign = degraded;
        foreign
            .records
            .get_mut("route-slot-01")
            .unwrap()
            .fence
            .operation_id = "route-keeper:foreign-operation".to_string();
        assert!(matches!(
            try_prove_route_keeper_predecessor_exit(
                &foreign,
                original,
                2,
                "linux:boot:fixture",
                ProcessObservation::Missing,
            ),
            Err(error) if error == "route_keeper_predecessor_proof_candidate_changed"
        ));
    }

    #[tokio::test]
    async fn host_process_registration_preserves_active_predecessor_and_reuses_successor() {
        let directory = TempDirectory::new("route-keeper-host-process-registration");
        let repository = repository(&directory);
        let mut predecessor = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut predecessor).await;
        let original = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let successor_identity = RecordedProcessIdentity {
            pid: 4_002,
            start_token: "linux:start:successor".to_string(),
            executable_path: Some("/opt/agent-browser".to_string()),
            browser_family: None,
        };

        let (registered, successor_generation) = register_route_keeper_host_process(
            &repository,
            "linux:boot:fixture",
            successor_identity.clone(),
        )
        .unwrap();
        assert_eq!(successor_generation, 2);
        assert_eq!(
            registered.records["route-slot-01"].protocol_ready,
            Some(original)
        );
        assert_eq!(registered.records["route-slot-01"].fence.host_generation, 1);
        assert!(registered
            .records
            .values()
            .filter(|record| record.phase == RouteKeeperPhase::Absent)
            .all(|record| record.fence.host_generation == successor_generation));

        let (replayed, replayed_generation) = register_route_keeper_host_process(
            &repository,
            "linux:boot:fixture",
            successor_identity,
        )
        .unwrap();
        assert_eq!(replayed_generation, successor_generation);
        assert_eq!(replayed, registered);
    }

    #[tokio::test]
    async fn cold_process_recovery_rejects_route_rebound_after_initial_candidate_scan() {
        let directory = TempDirectory::new("route-keeper-cold-process-rebound");
        let inner = repository(&directory);
        let mut predecessor = FakeConnector::default();
        let original_authority = make_minimum_ready(&inner, &mut predecessor).await;
        let original = original_authority.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let _ = register_host_claim(&inner, 2);
        let original_authority = register_host_claim(&inner, 3);

        let mut degraded = original_authority.clone();
        degraded
            .record_disconnect(
                &original.slot_id,
                &original.fence,
                &original.guacamole_connection_uuid,
            )
            .unwrap();
        let mut replacement = degraded;
        let action = replacement.begin_adoption("route-slot-01", 2).unwrap();
        let RouteKeeperReconcileAction::Adopt { fence, .. } = action else {
            unreachable!();
        };
        let mut rebound_ready = original.clone();
        rebound_ready.fence = fence;
        rebound_ready.observed_at = "2026-09-19T18:01:30Z".to_string();
        replacement
            .adopt(RouteKeeperAdoptionReceipt {
                previous_host_generation: 1,
                previous_guacamole_connection_uuid: original.guacamole_connection_uuid.clone(),
                ready: rebound_ready,
                adopted_at: "2026-09-19T18:01:31Z".to_string(),
            })
            .unwrap();

        let repository = RebindingRepository {
            inner,
            cas_calls: AtomicUsize::new(0),
            replacement: Mutex::new(Some(replacement.clone())),
        };
        let mut successor = FakeConnector::default();
        successor
            .routes
            .insert(original.slot_id.clone(), original.clone());

        let proof = prove_missing_predecessor(&original_authority, original.clone(), 3);
        assert_eq!(
            recover_proven_cold_process_route(&repository, &mut successor, &proof).await,
            Err("route_keeper_cold_process_candidate_changed:route-slot-01".to_string())
        );
        assert!(successor.adoptions.is_empty());
        assert_eq!(
            repository.inner.load_route_keeper_authority().unwrap(),
            replacement
        );
    }

    #[tokio::test]
    async fn cold_process_recovery_rejects_adopting_route_with_foreign_operation_id() {
        let directory = TempDirectory::new("route-keeper-cold-process-foreign-operation");
        let repository = repository(&directory);
        let mut predecessor = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut predecessor).await;
        let original = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let ready = register_host_claim(&repository, 2);
        let proof = prove_missing_predecessor(&ready, original.clone(), 2);

        let mut foreign = ready.clone();
        foreign
            .record_disconnect(
                &original.slot_id,
                &original.fence,
                &original.guacamole_connection_uuid,
            )
            .unwrap();
        foreign.begin_adoption("route-slot-01", 2).unwrap();
        foreign
            .records
            .get_mut("route-slot-01")
            .unwrap()
            .fence
            .operation_id = "route-keeper:foreign-operation".to_string();
        repository
            .compare_and_swap_route_keeper_authority(&ready, &foreign)
            .unwrap();
        let before = repository.load_route_keeper_authority().unwrap();
        let mut successor = FakeConnector::default();
        successor.routes.insert(original.slot_id.clone(), original);

        assert_eq!(
            recover_proven_cold_process_route(&repository, &mut successor, &proof).await,
            Err("route_keeper_cold_process_candidate_changed:route-slot-01".to_string())
        );
        assert!(successor.adoptions.is_empty());
        assert_eq!(repository.load_route_keeper_authority().unwrap(), before);
    }

    #[tokio::test]
    async fn cold_process_recovery_replays_prepared_adoption_after_connector_error() {
        let directory = TempDirectory::new("route-keeper-cold-process-replay");
        let repository = repository(&directory);
        let mut predecessor = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut predecessor).await;
        let original = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let claimed = register_host_claim(&repository, 2);
        let proof = prove_missing_predecessor(&claimed, original.clone(), 2);
        let mut successor = FakeConnector {
            adoption_error: Some("fixture_adoption_interrupted".to_string()),
            ..FakeConnector::default()
        };
        successor
            .routes
            .insert(original.slot_id.clone(), original.clone());

        assert_eq!(
            recover_proven_cold_process_route(&repository, &mut successor, &proof).await,
            Err("fixture_adoption_interrupted".to_string())
        );
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Adopting
        );

        let RouteKeeperColdRecoveryProgress::Ready(projection) =
            recover_proven_cold_process_route(&repository, &mut successor, &proof)
                .await
                .unwrap()
        else {
            panic!("replayed exact adoption must become ready");
        };
        assert_eq!(projection.ready_count, 1);
        assert!(successor.starts.is_empty());
        assert_eq!(successor.adoptions.len(), 2);
    }

    #[tokio::test]
    async fn cold_process_recovery_reconstructs_interrupted_exact_adoption() {
        let directory = TempDirectory::new("route-keeper-cold-process-reconstruct-adopting");
        let repository = repository(&directory);
        let mut predecessor = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut predecessor).await;
        let original = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let claimed = register_host_claim(&repository, 2);

        let mut interrupted = claimed.clone();
        interrupted
            .record_disconnect(
                &original.slot_id,
                &original.fence,
                &original.guacamole_connection_uuid,
            )
            .unwrap();
        interrupted.begin_adoption("route-slot-01", 2).unwrap();
        repository
            .compare_and_swap_route_keeper_authority(&claimed, &interrupted)
            .unwrap();

        let proof = prove_missing_predecessor(&interrupted, original.clone(), 2);
        let mut successor = FakeConnector::default();
        successor.routes.insert(original.slot_id.clone(), original);
        assert!(matches!(
            recover_proven_cold_process_route(&repository, &mut successor, &proof)
                .await
                .unwrap(),
            RouteKeeperColdRecoveryProgress::Ready(_)
        ));
        assert_eq!(successor.starts.len(), 0);
        assert_eq!(successor.adoptions.len(), 1);
    }

    #[tokio::test]
    async fn cold_process_recovery_reports_pending_until_exact_adoption_receipt() {
        let directory = TempDirectory::new("route-keeper-cold-process-pending");
        let repository = repository(&directory);
        let mut predecessor = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut predecessor).await;
        let original = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let claimed = register_host_claim(&repository, 2);
        let proof = prove_missing_predecessor(&claimed, original.clone(), 2);
        let mut successor = FakeConnector {
            adoption_pending_once: true,
            ..FakeConnector::default()
        };
        successor.routes.insert(original.slot_id.clone(), original);

        let RouteKeeperColdRecoveryProgress::Pending(projection) =
            recover_proven_cold_process_route(&repository, &mut successor, &proof)
                .await
                .unwrap()
        else {
            panic!("missing adoption receipt must remain pending");
        };
        assert_eq!(projection.ready_count, 0);
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Adopting
        );

        assert!(matches!(
            recover_proven_cold_process_route(&repository, &mut successor, &proof)
                .await
                .unwrap(),
            RouteKeeperColdRecoveryProgress::Ready(_)
        ));
    }

    #[tokio::test]
    async fn unproven_stop_quarantines_exact_slot_and_preserves_observed_identity() {
        let directory = TempDirectory::new("route-keeper-stop-quarantine");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();
        make_minimum_ready(&repository, &mut connector).await;
        connector.unproven_stop = Some("foreign-keeper-09".to_string());

        let projection = stop_once(&repository, &mut connector, "route-slot-01")
            .await
            .unwrap();
        assert_eq!(projection.ready_count, 0);
        assert_eq!(connector.stops.len(), 1);
        let authority = repository.load_route_keeper_authority().unwrap();
        let record = &authority.records["route-slot-01"];
        assert_eq!(record.phase, RouteKeeperPhase::Quarantined);
        let obligation = record.cleanup_obligation.as_ref().unwrap();
        assert_eq!(obligation.preserved_observed_keeper_id, "foreign-keeper-09");
        assert_eq!(obligation.reason, "route_keeper_stop_ownership_unproven");
    }

    #[tokio::test]
    async fn foreign_keeper_stop_receipt_quarantines_instead_of_retrying_stop() {
        let directory = TempDirectory::new("route-keeper-foreign-stop");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();
        let expected = make_minimum_ready(&repository, &mut connector).await;
        let ready = expected.records["route-slot-01"]
            .protocol_ready
            .as_ref()
            .unwrap()
            .clone();
        let mut stopping = expected.clone();
        let action = stopping.begin_stop("route-slot-01", &ready.fence).unwrap();
        repository
            .compare_and_swap_route_keeper_authority(&expected, &stopping)
            .unwrap();

        let projection = apply_stop_observation(
            &repository,
            stopping,
            &action,
            RouteKeeperStopObservation::Stopped(RouteKeeperStopReceipt {
                slot_id: ready.slot_id,
                keeper_id: "foreign-keeper-09".to_string(),
                fence: ready.fence,
                guacamole_connection_uuid: Some(ready.guacamole_connection_uuid),
                xrdp_session_id: Some(ready.xrdp_session_id),
                stopped_at: "2026-09-19T18:02:30Z".to_string(),
            }),
        )
        .unwrap();

        assert_eq!(projection.ready_count, 0);
        let authority = repository.load_route_keeper_authority().unwrap();
        let record = &authority.records["route-slot-01"];
        assert_eq!(record.phase, RouteKeeperPhase::Quarantined);
        let obligation = record.cleanup_obligation.as_ref().unwrap();
        assert!(obligation
            .preserved_observed_keeper_id
            .starts_with("keeper=foreign-keeper-09;"));
        assert_eq!(obligation.reason, "route_keeper_stop_observation_unproven");
    }

    #[test]
    fn connector_receipts_must_match_the_dispatched_action() {
        let action = RouteKeeperReconcileAction::Adopt {
            slot_id: "route-slot-01".to_string(),
            keeper_id: "route-keeper-01".to_string(),
            fence: RouteKeeperFence {
                host_generation: 2,
                operation_id: "route-keeper:2:route-slot-01:2".to_string(),
                operation_generation: 2,
                connection_catalog_digest: "0".repeat(64),
            },
            previous_host_generation: 1,
        };
        let foreign_ready = RouteKeeperProtocolReadyReceipt {
            slot_id: "route-slot-02".to_string(),
            keeper_id: "route-keeper-02".to_string(),
            fence: RouteKeeperFence {
                host_generation: 2,
                operation_id: "route-keeper:2:route-slot-02:2".to_string(),
                operation_generation: 2,
                connection_catalog_digest: "0".repeat(64),
            },
            guacamole_connection_uuid: "connection-route-slot-02".to_string(),
            xrdp_session_id: "xrdp-route-slot-02".to_string(),
            display_name: ":02".to_string(),
            xrdp_ownership: Some(RouteKeeperXrdpOwnershipWitness {
                schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
                boot_id: "boot-fixture".to_string(),
                route_user: "agent-browser-rdp-2".to_string(),
                route_uid: 2002,
                session_id: "xrdp-route-slot-02".to_string(),
                session_service: "xrdp-sesman".to_string(),
                session_scope: "session-xrdp-route-slot-02.scope".to_string(),
                scope_invocation_id: "invocation-fixture".to_string(),
                cgroup_path: "/user.slice/user-2002.slice/session-xrdp-route-slot-02.scope"
                    .to_string(),
                cgroup_device: 28,
                cgroup_inode: 1002,
                leader_pid: 4201,
                leader_start_ticks: 5201,
                x_server_pid: 4202,
                x_server_start_ticks: 5202,
                display_name: ":02".to_string(),
                x11_socket_inode: 6201,
            }),
            observed_at: "2026-09-19T18:03:00Z".to_string(),
        };
        assert_eq!(
            require_ready_matches_action(&action, &foreign_ready),
            Err("route_keeper_connector_observation_mismatch".to_string())
        );
        assert_eq!(
            require_adoption_matches_action(
                &action,
                &RouteKeeperAdoptionReceipt {
                    previous_host_generation: 1,
                    previous_guacamole_connection_uuid: foreign_ready
                        .guacamole_connection_uuid
                        .clone(),
                    ready: foreign_ready.clone(),
                    adopted_at: "2026-09-19T18:03:01Z".to_string(),
                },
            ),
            Err("route_keeper_connector_observation_mismatch".to_string())
        );
        assert_eq!(
            require_stop_matches_action(
                &action,
                &RouteKeeperStopReceipt {
                    slot_id: foreign_ready.slot_id,
                    keeper_id: foreign_ready.keeper_id,
                    fence: foreign_ready.fence,
                    guacamole_connection_uuid: Some(foreign_ready.guacamole_connection_uuid,),
                    xrdp_session_id: Some(foreign_ready.xrdp_session_id),
                    stopped_at: "2026-09-19T18:03:02Z".to_string(),
                },
            ),
            Err("route_keeper_connector_observation_mismatch".to_string())
        );
    }

    fn terminal_event(
        authority: &RouteKeeperAuthority,
        slot_id: &str,
        occurrence_id: &str,
        guacamole_connection_uuid: Option<String>,
    ) -> RouteKeeperTerminalEvent {
        let record = &authority.records[slot_id];
        RouteKeeperTerminalEvent {
            slot_id: record.slot_id.clone(),
            keeper_id: record.keeper_id.clone(),
            fence: record.fence.clone(),
            occurrence_id: occurrence_id.to_string(),
            guacamole_connection_uuid,
            code: "fixture_primary_closed",
            elapsed_ms: 7,
        }
    }

    #[tokio::test]
    async fn supervisor_establishes_minimum_before_warming_on_later_ticks() {
        let directory = TempDirectory::new("route-keeper-supervisor-minimum");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();

        let minimum = reconcile_until_minimum(&repository, &mut connector)
            .await
            .unwrap();
        assert_eq!(minimum.ready_count, 1);
        assert!(minimum.minimum_satisfied);
        assert_eq!(connector.starts.len(), 1);
        assert_eq!(connector.observes.len(), 1);

        let warming = supervise_once(&repository, &mut connector).await.unwrap();
        assert_eq!(warming.ready_count, 1);
        assert_eq!(connector.starts.len(), 2);
        assert_eq!(
            connector.starts.last(),
            Some(&RouteKeeperReconcileAction::Start {
                slot_id: "route-slot-02".to_string(),
                keeper_id: "route-keeper-02".to_string(),
                fence: repository.load_route_keeper_authority().unwrap().records["route-slot-02"]
                    .fence
                    .clone(),
                priority: RouteKeeperStartPriority::Warm,
            })
        );
    }

    #[tokio::test]
    async fn supervisor_persists_terminal_before_recovery_and_discards_stale_fence() {
        let directory = TempDirectory::new("route-keeper-supervisor-terminal");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut connector).await;
        let receipt = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        connector
            .current_terminal_occurrences
            .insert("route-slot-01".to_string(), "occurrence-ready".to_string());
        connector.terminal_events.push(terminal_event(
            &ready,
            "route-slot-01",
            "occurrence-ready",
            Some(receipt.guacamole_connection_uuid),
        ));

        let recovery = supervise_once(&repository, &mut connector).await.unwrap();
        assert_eq!(recovery.ready_count, 0);
        assert_eq!(
            connector.acknowledged_terminal_occurrences,
            ["occurrence-ready"]
        );
        assert!(matches!(
            connector.starts.last(),
            Some(RouteKeeperReconcileAction::Start {
                slot_id,
                priority: RouteKeeperStartPriority::Recovery,
                ..
            }) if slot_id == "route-slot-01"
        ));
        let recovered = repository.load_route_keeper_authority().unwrap();
        assert_eq!(
            recovered.records["route-slot-01"].phase,
            RouteKeeperPhase::Observing
        );
        assert!(
            recovered.records["route-slot-01"]
                .fence
                .operation_generation
                > ready.records["route-slot-01"].fence.operation_generation
        );

        let before_stale = recovered.clone();
        connector.terminal_events.push(terminal_event(
            &ready,
            "route-slot-01",
            "occurrence-stale",
            None,
        ));
        process_terminal_events(&repository, &mut connector).unwrap();
        assert_eq!(
            repository.load_route_keeper_authority().unwrap(),
            before_stale
        );
        assert_eq!(
            connector.acknowledged_terminal_occurrences,
            ["occurrence-ready", "occurrence-stale"]
        );
    }

    #[tokio::test]
    async fn supervisor_persists_current_adoption_terminal_without_touching_predecessor() {
        let directory = TempDirectory::new("route-keeper-supervisor-adoption-terminal");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();
        let ready = make_minimum_ready(&repository, &mut connector).await;
        let predecessor = ready.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let claimed = register_host_claim(&repository, 2);
        let mut adopting = claimed.clone();
        adopting
            .record_disconnect(
                &predecessor.slot_id,
                &predecessor.fence,
                &predecessor.guacamole_connection_uuid,
            )
            .unwrap();
        adopting.begin_adoption("route-slot-01", 2).unwrap();
        repository
            .compare_and_swap_route_keeper_authority(&claimed, &adopting)
            .unwrap();

        connector.current_terminal_occurrences.insert(
            "route-slot-01".to_string(),
            "occurrence-adopting".to_string(),
        );
        connector.terminal_events.push(terminal_event(
            &adopting,
            "route-slot-01",
            "occurrence-adopting",
            Some("successor-connection".to_string()),
        ));
        process_terminal_events(&repository, &mut connector).unwrap();

        let retained = repository.load_route_keeper_authority().unwrap();
        let record = &retained.records["route-slot-01"];
        assert_eq!(record.phase, RouteKeeperPhase::RecoveryFailed);
        assert_eq!(record.protocol_ready.as_ref(), Some(&predecessor));
        assert_eq!(
            connector.acknowledged_terminal_occurrences,
            ["occurrence-adopting"]
        );
    }

    #[tokio::test]
    async fn supervisor_retries_unpublished_terminal_and_shuts_down_once() {
        let directory = TempDirectory::new("route-keeper-supervisor-retry");
        let database_path = directory.0.join("runtime.sqlite3");
        let base = repository(&directory);
        let mut connector = FakeConnector::default();
        reconcile_once(&base, &mut connector).await.unwrap();
        let observing = base.load_route_keeper_authority().unwrap();
        connector.current_terminal_occurrences.insert(
            "route-slot-01".to_string(),
            "occurrence-before-ready".to_string(),
        );
        connector.terminal_events.push(terminal_event(
            &observing,
            "route-slot-01",
            "occurrence-before-ready",
            None,
        ));
        let conflicting = ConflictRepository {
            inner: SqliteRouteKeeperRepository::new(&database_path),
            cas_calls: AtomicUsize::new(0),
            reject_call: 1,
        };
        assert_eq!(
            supervise_once(&conflicting, &mut connector).await,
            Err("route_keeper_authority_compare_and_swap_conflict".to_string())
        );
        assert_eq!(
            connector.restored_terminal_occurrences,
            ["occurrence-before-ready"]
        );
        assert!(connector.acknowledged_terminal_occurrences.is_empty());

        let projection = supervise_once(&conflicting, &mut connector).await.unwrap();
        assert_eq!(projection.ready_count, 0);
        assert_eq!(connector.starts.len(), 2);
        assert_eq!(
            connector.acknowledged_terminal_occurrences,
            ["occurrence-before-ready"]
        );

        let (_tick_tx, mut ticks) = mpsc::channel(1);
        let (shutdown_tx, mut shutdown) = watch::channel(false);
        let signal = tokio::spawn(async move {
            tokio::task::yield_now().await;
            shutdown_tx.send(true).unwrap();
        });
        run_route_keeper_supervisor(&base, &mut connector, &mut ticks, &mut shutdown)
            .await
            .unwrap();
        signal.await.unwrap();
        assert_eq!(connector.shutdowns, 1);
    }

    struct BlockingStartConnector {
        action: Option<RouteKeeperReconcileAction>,
        entered: Arc<tokio::sync::Notify>,
        terminal_events: Vec<RouteKeeperTerminalEvent>,
        shutdowns: usize,
    }

    #[async_trait::async_trait]
    impl PresentationRouteConnector for BlockingStartConnector {
        async fn start(&mut self, action: &RouteKeeperReconcileAction) -> Result<(), String> {
            self.action = Some(action.clone());
            self.entered.notify_one();
            std::future::pending().await
        }

        async fn observe(
            &mut self,
            _action: &RouteKeeperReconcileAction,
        ) -> Result<RouteKeeperConnectorObservation, String> {
            unreachable!()
        }

        async fn adopt(
            &mut self,
            _action: &RouteKeeperReconcileAction,
        ) -> Result<RouteKeeperAdoptionObservation, String> {
            unreachable!()
        }

        async fn stop(
            &mut self,
            _action: &RouteKeeperReconcileAction,
        ) -> Result<RouteKeeperStopObservation, String> {
            unreachable!()
        }
    }

    #[async_trait::async_trait]
    impl SupervisedPresentationRouteConnector for BlockingStartConnector {
        fn take_terminal_events(&mut self) -> Vec<RouteKeeperTerminalEvent> {
            std::mem::take(&mut self.terminal_events)
        }

        fn terminal_event_is_current(&self, event: &RouteKeeperTerminalEvent) -> bool {
            self.action.as_ref().is_some_and(|action| {
                let (slot_id, keeper_id, fence) = action_identity(action);
                event.slot_id == slot_id
                    && event.keeper_id == keeper_id
                    && event.fence == *fence
                    && event.occurrence_id == "blocking-start"
            })
        }

        fn restore_terminal_event(&mut self, event: RouteKeeperTerminalEvent) {
            self.terminal_events.push(event);
        }

        fn acknowledge_terminal_event(&mut self, _event: &RouteKeeperTerminalEvent) {
            self.action = None;
        }

        async fn shutdown_primaries(&mut self) {
            self.shutdowns += 1;
            let Some(action) = self.action.as_ref() else {
                return;
            };
            let (slot_id, keeper_id, fence) = action_identity(action);
            self.terminal_events.push(RouteKeeperTerminalEvent {
                slot_id: slot_id.to_string(),
                keeper_id: keeper_id.to_string(),
                fence: fence.clone(),
                occurrence_id: "blocking-start".to_string(),
                guacamole_connection_uuid: None,
                code: "fixture_shutdown",
                elapsed_ms: 1,
            });
        }
    }

    #[tokio::test]
    async fn supervisor_shutdown_interrupts_pending_start_and_persists_terminal_state() {
        let directory = TempDirectory::new("route-keeper-supervisor-interrupt");
        let repository = repository(&directory);
        let entered = Arc::new(tokio::sync::Notify::new());
        let mut connector = BlockingStartConnector {
            action: None,
            entered: entered.clone(),
            terminal_events: Vec::new(),
            shutdowns: 0,
        };
        let (_tick_tx, mut ticks) = mpsc::channel(1);
        let (shutdown_tx, mut shutdown) = watch::channel(false);
        let signal = tokio::spawn(async move {
            entered.notified().await;
            shutdown_tx.send(true).unwrap();
        });

        run_route_keeper_supervisor(&repository, &mut connector, &mut ticks, &mut shutdown)
            .await
            .unwrap();
        signal.await.unwrap();
        assert_eq!(connector.shutdowns, 1);
        assert!(connector.action.is_none());
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Absent
        );
    }

    #[tokio::test]
    async fn configured_supervisor_exactly_stops_ready_routes_before_shutdown() {
        let directory = TempDirectory::new("route-keeper-configured-shutdown");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();
        reconcile_once(&repository, &mut connector).await.unwrap();
        reconcile_once(&repository, &mut connector).await.unwrap();
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Ready
        );
        let (_tick_tx, mut ticks) = mpsc::channel(1);
        let (shutdown_tx, mut shutdown) = watch::channel(false);
        shutdown_tx.send(true).unwrap();

        run_configured_route_keeper_supervisor(
            &repository,
            &mut connector,
            &mut ticks,
            &mut shutdown,
        )
        .await
        .unwrap();

        assert_eq!(connector.stops.len(), 1);
        assert_eq!(connector.shutdowns, 1);
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Absent
        );
    }

    #[tokio::test]
    async fn configured_supervisor_error_keeps_exact_stop_policy() {
        let directory = TempDirectory::new("route-keeper-configured-error-shutdown");
        let repository = repository(&directory);
        let mut connector = FakeConnector::default();
        reconcile_once(&repository, &mut connector).await.unwrap();
        reconcile_once(&repository, &mut connector).await.unwrap();
        let (tick_tx, mut ticks) = mpsc::channel(1);
        drop(tick_tx);
        let (_shutdown_tx, mut shutdown) = watch::channel(false);

        assert_eq!(
            run_configured_route_keeper_supervisor(
                &repository,
                &mut connector,
                &mut ticks,
                &mut shutdown,
            )
            .await,
            Err("route_keeper_supervisor_tick_source_closed".to_string())
        );
        assert_eq!(connector.stops.len(), 1);
        assert_eq!(connector.shutdowns, 1);
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Absent
        );
    }

    struct CancelledStartConnector {
        starts: Arc<AtomicUsize>,
        routes: Arc<Mutex<BTreeMap<String, RouteKeeperProtocolReadyReceipt>>>,
        effect_created: Arc<tokio::sync::Notify>,
    }

    #[async_trait::async_trait]
    impl PresentationRouteConnector for CancelledStartConnector {
        async fn start(&mut self, action: &RouteKeeperReconcileAction) -> Result<(), String> {
            self.starts.fetch_add(1, Ordering::SeqCst);
            let receipt = FakeConnector::ready_for_action(action);
            self.routes
                .lock()
                .unwrap()
                .insert(receipt.slot_id.clone(), receipt);
            self.effect_created.notify_one();
            std::future::pending().await
        }

        async fn observe(
            &mut self,
            _action: &RouteKeeperReconcileAction,
        ) -> Result<RouteKeeperConnectorObservation, String> {
            unreachable!()
        }

        async fn adopt(
            &mut self,
            _action: &RouteKeeperReconcileAction,
        ) -> Result<RouteKeeperAdoptionObservation, String> {
            unreachable!()
        }

        async fn stop(
            &mut self,
            _action: &RouteKeeperReconcileAction,
        ) -> Result<RouteKeeperStopObservation, String> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn cancelled_start_recovers_by_observation_without_duplicate_start() {
        let directory = TempDirectory::new("route-keeper-cancelled-start");
        let database_path = directory.0.join("runtime.sqlite3");
        let repository = repository(&directory);
        let starts = Arc::new(AtomicUsize::new(0));
        let routes = Arc::new(Mutex::new(BTreeMap::new()));
        let effect_created = Arc::new(tokio::sync::Notify::new());
        let task_starts = starts.clone();
        let task_routes = routes.clone();
        let task_created = effect_created.clone();
        let task = tokio::spawn(async move {
            let repository = SqliteRouteKeeperRepository::new(&database_path);
            let mut connector = CancelledStartConnector {
                starts: task_starts,
                routes: task_routes,
                effect_created: task_created,
            };
            reconcile_once(&repository, &mut connector).await
        });

        effect_created.notified().await;
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Starting
        );

        let mut recovery = FakeConnector {
            routes: routes.lock().unwrap().clone(),
            ..FakeConnector::default()
        };
        let projection = reconcile_once(&repository, &mut recovery).await.unwrap();
        assert_eq!(projection.ready_count, 1);
        assert!(recovery.starts.is_empty());
        assert_eq!(recovery.observes.len(), 1);
        assert_eq!(starts.load(Ordering::SeqCst), 1);
    }
}
