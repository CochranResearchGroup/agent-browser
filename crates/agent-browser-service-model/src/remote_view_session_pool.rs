//! Demand-driven public pool preparation, with durable request identity.
use crate::*;

/// Agent Browser's warm capacity minimum, bounded by Remote View policy.
/// This is demand, not a copy of Remote View allocation state.
pub struct RemoteViewSessionPool {
    pub name: String,
    pub desired_desktops: u32,
}

pub struct RemoteViewPreparedSessionPool {
    pub assignments: Vec<RemoteViewAssignmentRecord>,
    pub desktops: Vec<RemoteViewDesktopCandidate>,
}

/// Background maintenance for an already-used pool. An unused environment
/// does not acquire desktops merely because its daemon is running.
pub fn replenish_remote_view_session_pool<
    T: RemoteViewApplicationTransport,
    S: RemoteViewPoolRequestStore,
>(
    adapter: &mut RemoteViewApplicationAdapter<T>,
    store: &mut S,
    pool: &RemoteViewSessionPool,
) -> Result<Option<RemoteViewPreparedSessionPool>, String> {
    let inventory = adapter
        .inventory()
        .map_err(|_| "remote_view_runtime_inventory_unavailable".to_string())?;
    let used = inventory
        .pools
        .iter()
        .find(|entry| entry.name == pool.name)
        .is_some_and(|entry| {
            inventory.assignments.iter().any(|assignment| {
                assignment.pool_id == entry.pool_id
                    && assignment.state == RemoteViewAssignmentState::Active
            })
        });
    if !used {
        return Ok(None);
    }
    prepare_remote_view_session_pool(adapter, store, pool).map(Some)
}

/// Owns a durable acquisition request head and its unresolved public requests.
/// Concurrent callers must receive the same unsubmitted/pending key. A new key
/// after completion requires an exact active acquisition or confirmed release,
/// including a matching native Released record. Observed assignments include
/// terminal records; inventory absence alone cannot retire a previous request.
pub trait RemoteViewPoolRequestStore: RemoteViewApplicationMutationStore {
    fn pool_assignment_available(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<bool, RemoteViewApplicationMutationStoreError>;
    fn confirmed_pool_acquisitions(
        &mut self,
        application: &str,
        pool: &str,
    ) -> Result<Vec<RemoteViewAssignmentRecord>, RemoteViewApplicationMutationStoreError>;
    fn pending_pool_acquisitions(
        &mut self,
        application: &str,
        pool: &str,
    ) -> Result<Vec<RemoteViewApplicationMutationRecord>, RemoteViewApplicationMutationStoreError>;
    fn pool_acquisition_request_key(
        &mut self,
        application: &str,
        pool: &str,
        observed_assignments: &[RemoteViewAssignmentRecord],
    ) -> Result<String, RemoteViewApplicationMutationStoreError>;
}

/// Called only when a new browser needs placement. Healthy existing browsers
/// and read-only status do not invoke this path. Every acquisition is retained
/// before sending and must be visible in current inventory before launch.
pub fn prepare_remote_view_session_pool<
    T: RemoteViewApplicationTransport,
    S: RemoteViewPoolRequestStore,
>(
    adapter: &mut RemoteViewApplicationAdapter<T>,
    store: &mut S,
    pool: &RemoteViewSessionPool,
) -> Result<RemoteViewPreparedSessionPool, String> {
    if pool.name.is_empty() || pool.desired_desktops == 0 {
        return Err("remote_view_session_pool_invalid".into());
    }
    let pending = store
        .pending_pool_acquisitions(&adapter.application, &pool.name)
        .map_err(|_| "remote_view_pool_custody_unavailable".to_string())?;
    let mut unresolved = false;
    for record in pending {
        if !matches!(&record.envelope.request,
            RemoteViewApplicationRequest::Acquire { pool_name, .. } if pool_name == &pool.name)
        {
            return Err("remote_view_pool_acquisition_readback_required".into());
        }
        unresolved |= adapter.reconcile_acquisition(&record, store).is_err();
    }
    let confirmed = if unresolved {
        store
            .confirmed_pool_acquisitions(&adapter.application, &pool.name)
            .map_err(|_| "remote_view_pool_custody_unavailable".to_string())?
    } else {
        Vec::new()
    };
    let mut inventory = adapter
        .inventory()
        .map_err(|_| "remote_view_runtime_inventory_unavailable".to_string())?;
    let pool_id = inventory
        .pools
        .iter()
        .find(|entry| entry.name == pool.name)
        .ok_or_else(|| "remote_view_runtime_pool_missing".to_string())?
        .pool_id
        .clone();
    let maximum = inventory
        .policy
        .pools
        .get(&pool.name)
        .ok_or_else(|| "remote_view_runtime_pool_missing".to_string())?
        .maximum_reserved;
    if pool.desired_desktops > maximum {
        return Err("remote_view_pool_demand_exceeds_policy".into());
    }
    let active = |inventory: &RemoteViewApplicationInventory| {
        inventory
            .assignments
            .iter()
            .filter(|assignment| {
                assignment.pool_id == pool_id
                    && assignment.state == RemoteViewAssignmentState::Active
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    let mut assignments = active(&inventory);
    loop {
        let mut desktop_ids = std::collections::BTreeSet::new();
        let mut candidates = Vec::new();
        let mut observed = Vec::new();
        let mut occupied = false;
        let mut unavailable = false;
        for assignment in &assignments {
            if !desktop_ids.insert(assignment.desktop_id.clone()) {
                return Err("remote_view_session_assignment_invalid".into());
            }
            // A separate uncertain acquisition cannot invalidate an already
            // confirmed free destination, nor qualify its own unknown result.
            if unresolved && !confirmed.contains(assignment) {
                continue;
            }
            if !store
                .pool_assignment_available(assignment)
                .map_err(|_| "remote_view_pool_custody_unavailable".to_string())?
            {
                unavailable = true;
                continue;
            }
            // A failed observation never qualifies a desktop as free.
            let Ok(windows) = adapter.windows(assignment) else {
                unavailable = true;
                continue;
            };
            let candidate = RemoteViewDesktopCandidate {
                ready: true,
                desktop: RemoteViewFixedDesktop {
                    desktop_id: assignment.desktop_id.clone(),
                    generation: assignment.generation,
                    friendly_route_label: assignment.desktop_id.clone(),
                },
            };
            observed.push(candidate.clone());
            if !windows.windows.is_empty() {
                occupied = true;
                continue;
            }
            candidates.push(candidate);
        }
        let warm_minimum_met = assignments.len() >= pool.desired_desktops as usize;
        if warm_minimum_met && !candidates.is_empty() {
            return Ok(RemoteViewPreparedSessionPool {
                assignments,
                desktops: candidates,
            });
        }
        if unresolved {
            return Err("remote_view_pool_acquisition_readback_required".into());
        }
        // At the growth limit known healthy occupied desktops are eligible
        // for wrap-around. Unknown peers never become placement candidates.
        if assignments.len() >= maximum as usize && !observed.is_empty() {
            return Ok(RemoteViewPreparedSessionPool {
                assignments,
                desktops: observed,
            });
        }
        if warm_minimum_met && unavailable {
            return Err("remote_view_runtime_assignment_unavailable".into());
        }
        if assignments.len() >= maximum as usize {
            return Err(if occupied {
                "remote_view_pool_capacity_exhausted"
            } else {
                "remote_view_runtime_assignment_unavailable"
            }
            .into());
        }
        // Once the minimum is met, unknown observations cannot justify growth.
        if warm_minimum_met && !occupied {
            return Err("remote_view_runtime_assignment_unavailable".into());
        }
        let key = store
            .pool_acquisition_request_key(&adapter.application, &pool.name, &inventory.assignments)
            .map_err(|_| "remote_view_pool_acquisition_readback_required".to_string())?;
        let acquired = adapter
            .acquire(pool.name.clone(), key, store)
            .map_err(|_| "remote_view_pool_acquisition_readback_required".to_string())?;
        inventory = adapter
            .inventory()
            .map_err(|_| "remote_view_runtime_inventory_unavailable".to_string())?;
        let refreshed = active(&inventory);
        if !refreshed.contains(&acquired) || refreshed.len() <= assignments.len() {
            return Err("remote_view_pool_acquisition_readback_required".into());
        }
        assignments = refreshed;
    }
}
