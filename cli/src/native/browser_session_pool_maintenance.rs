//! Background native pool maintenance. Browser work never waits for spare growth.
use super::browser_session_store::BrowserSessionSqliteStore;
use agent_browser_service_model::*;

/// Return excess unused assignments; Remote View owns their physical cooldown
/// and guarded retirement. Preserve one owner-observed ready spare. Durable
/// fences and mutation records make interruption and concurrent maintenance safe.
pub(crate) fn maintain_pool<T: RemoteViewApplicationTransport>(
    adapter: &mut RemoteViewApplicationAdapter<T>,
    store: &mut BrowserSessionSqliteStore,
    pool: &RemoteViewSessionPool,
) -> Result<(), String> {
    let mut failure = None;
    for obligation in store.idle_release_obligations()? {
        let outcome = match obligation.record {
            None => {
                store.cancel_unsubmitted_or_deferred_idle_release(&obligation.release_id)?;
                continue;
            }
            Some(record) => match record.outcome.clone() {
                Some(outcome) => Ok(outcome),
                None => adapter.reconcile_idle_release(&record, &obligation.target, store),
            },
        };
        match outcome {
            Ok(RemoteViewApplicationMutationOutcome::Release(outcome)) => {
                store
                    .complete_assignment_release(
                        &obligation.target,
                        &obligation.release_id,
                        &outcome,
                    )
                    .map_err(|_| "remote_view_release_readback_required")?;
            }
            Ok(RemoteViewApplicationMutationOutcome::ReleaseIdleDeferred) => {
                store.cancel_unsubmitted_or_deferred_idle_release(&obligation.release_id)?;
            }
            _ => failure = Some("remote_view_release_readback_required".to_string()),
        }
    }
    let inventory = adapter
        .inventory()
        .map_err(|_| "remote_view_runtime_inventory_unavailable")?;
    let configured = inventory
        .pools
        .iter()
        .find(|entry| entry.name == pool.name)
        .ok_or("remote_view_runtime_pool_missing")?;
    let mut unused = Vec::new();
    for assignment in inventory.assignments.iter().filter(|assignment| {
        assignment.pool_id == configured.pool_id
            && assignment.state == RemoteViewAssignmentState::Active
    }) {
        if !store
            .pool_assignment_available(assignment)
            .map_err(|_| "remote_view_pool_custody_unavailable")?
        {
            continue;
        }
        // An unavailable owner provides no idle evidence for automatic cleanup.
        let Ok(observation) = adapter.observe_idle(assignment) else {
            continue;
        };
        if observation.idle {
            unused.push((assignment.clone(), observation));
        }
    }
    for (assignment, observation) in unused.into_iter().skip(1) {
        let target = observation
            .release_target(&assignment)
            .map_err(|_| "remote_view_idle_evidence_invalid")?;
        let pending = store
            .unpublished_launch_records()
            .map_err(|_| "remote_view_session_custody_unavailable")?;
        let mutations =
            super::remote_view_pool_request_store::records(store.remote_view_mutation_connection())
                .map_err(|_| "remote_view_pool_custody_unavailable")?;
        let pending_ids = pending
            .iter()
            .filter(|record| record.intent.assignment.desktop_id == assignment.desktop_id)
            .map(|record| record.intent.intent_id.clone())
            .collect();
        let cleanup_ids = mutations
            .iter()
            .filter(|record| {
                record.outcome.is_none()
                    && match &record.envelope.request {
                        RemoteViewApplicationRequest::Activate { assignment_id, .. }
                        | RemoteViewApplicationRequest::IssueView { assignment_id, .. }
                        | RemoteViewApplicationRequest::Release { assignment_id, .. }
                        | RemoteViewApplicationRequest::ReleaseIdle { assignment_id, .. } => {
                            assignment_id == &assignment.assignment_id
                        }
                        _ => false,
                    }
            })
            .map(|record| record.envelope.mutation_key())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "remote_view_pool_custody_unavailable")?;
        let snapshot = RemoteViewApplicationCleanupSnapshot {
            schema_version: 1,
            assignment_id: assignment.assignment_id.clone(),
            desktop_id: assignment.desktop_id.clone(),
            lifecycle_generation: assignment.generation,
            pending_recovery: RemoteViewApplicationObligationInventory::Complete(pending_ids),
            // Current native viewer/control evidence comes from the owner; the
            // guarded release rechecks it under the native admission lock.
            foreground_leases: RemoteViewApplicationObligationInventory::Complete(
                observation.retirement.sessions.clone(),
            ),
            cleanup_tasks: RemoteViewApplicationObligationInventory::Complete(cleanup_ids),
        };
        let release_id = uuid::Uuid::new_v4().to_string();
        let permit = match store.admit_idle_assignment_release(&target, &snapshot, &release_id) {
            Ok(permit) => permit,
            Err(
                LaunchCustodyStoreError::CleanupBlocked
                | LaunchCustodyStoreError::LaunchPending
                | LaunchCustodyStoreError::Conflict,
            ) => continue,
            Err(_) => return Err("remote_view_release_custody_unavailable".into()),
        };
        match adapter.release_idle(&assignment, &observation, permit, release_id.clone(), store) {
            Ok(outcome) => store
                .complete_assignment_release(&target, &release_id, &outcome)
                .map_err(|_| "remote_view_release_readback_required")?,
            Err(RemoteViewApplicationAdapterError::IdleReleaseDeferred) => {
                store.cancel_unsubmitted_or_deferred_idle_release(&release_id)?;
            }
            Err(_) => failure = Some("remote_view_release_readback_required".into()),
        }
    }
    // Growth is independent of an unrelated pending release. The pool prepares
    // only destinations whose current launch/release custody is admissible.
    let growth = replenish_remote_view_session_pool(adapter, store, pool).map(|_| ());
    if let Some(error) = failure {
        Err(error)
    } else {
        growth
    }
}
