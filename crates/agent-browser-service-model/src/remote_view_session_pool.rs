//! Demand-driven public pool preparation, with durable request identity.
use crate::*;

/// Agent Browser's desired service capacity, bounded by Remote View policy.
/// This is demand, not a copy of Remote View allocation state.
pub struct RemoteViewSessionPool {
    pub name: String,
    pub desired_desktops: u32,
}

pub struct RemoteViewPreparedSessionPool {
    pub assignments: Vec<RemoteViewAssignmentRecord>,
    pub desktops: Vec<RemoteViewDesktopCandidate>,
}

/// Owns a durable acquisition request head and its unresolved public requests.
/// Concurrent callers must receive the same unsubmitted/pending key. A new key
/// after completion requires an exact active acquisition or confirmed release;
/// absence from provider inventory alone cannot retire a previous request.
pub trait RemoteViewPoolRequestStore: RemoteViewApplicationMutationStore {
    fn pending_pool_acquisitions(
        &mut self,
        application: &str,
        pool: &str,
    ) -> Result<Vec<RemoteViewApplicationMutationRecord>, RemoteViewApplicationMutationStoreError>;
    fn pool_acquisition_request_key(
        &mut self,
        application: &str,
        pool: &str,
        active_assignments: &[RemoteViewAssignmentRecord],
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
    for record in pending {
        if !matches!(&record.envelope.request,
            RemoteViewApplicationRequest::Acquire { pool_name, .. } if pool_name == &pool.name)
        {
            return Err("remote_view_pool_acquisition_readback_required".into());
        }
        adapter
            .reconcile_acquisition(&record, store)
            .map_err(|_| "remote_view_pool_acquisition_readback_required".to_string())?;
    }
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
    // The desired capacity bounds this operation's number of acquisitions.
    // A cached result that makes no progress stops, rather than spinning.
    while assignments.len() < pool.desired_desktops as usize {
        let key = store
            .pool_acquisition_request_key(&adapter.application, &pool.name, &assignments)
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
    let mut desktop_ids = std::collections::BTreeSet::new();
    let mut candidates = Vec::new();
    for assignment in &assignments {
        if !desktop_ids.insert(assignment.desktop_id.clone()) {
            return Err("remote_view_session_assignment_invalid".into());
        }
        // Windows observation refreshes both independent generations before
        // and after the read. A failed peer cannot erase healthy candidates.
        let Ok(windows) = adapter.windows(assignment) else {
            continue;
        };
        candidates.push((
            !windows.windows.is_empty(),
            RemoteViewDesktopCandidate {
                ready: true,
                desktop: RemoteViewFixedDesktop {
                    desktop_id: assignment.desktop_id.clone(),
                    generation: assignment.generation,
                    friendly_route_label: assignment.desktop_id.clone(),
                },
            },
        ));
    }
    if candidates.is_empty() {
        return Err("remote_view_runtime_assignment_unavailable".into());
    }
    // Window-free desktops win ties in the ordinary manager's browser count.
    // Occupied desktops remain eligible and no window titles are retained.
    candidates.sort_by_key(|(occupied, _)| *occupied);
    Ok(RemoteViewPreparedSessionPool {
        assignments,
        desktops: candidates.into_iter().map(|(_, desktop)| desktop).collect(),
    })
}
