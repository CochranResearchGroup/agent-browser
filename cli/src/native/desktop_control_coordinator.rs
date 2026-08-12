//! Process-owned, route-scoped serialization for desktop controller mutation.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

#[derive(Default)]
pub(crate) struct DesktopControlCoordinator {
    routes: Mutex<HashMap<String, Arc<RouteControl>>>,
}

#[derive(Debug, Default)]
struct RouteControl {
    state: Mutex<RouteControlState>,
    changed: Condvar,
}

#[derive(Debug, Default)]
struct RouteControlState {
    interaction_claim_id: Option<String>,
    interaction_cancelled: bool,
    event_in_flight: bool,
    controller_mutation_in_flight: bool,
}

#[derive(Debug)]
pub(crate) struct DesktopInteractionClaim {
    route_id: String,
    claim_id: String,
    route: Arc<RouteControl>,
}

#[derive(Debug)]
pub(crate) struct DesktopControlEventGuard {
    route: Arc<RouteControl>,
}

pub(crate) struct DesktopControllerMutationGuard {
    route: Arc<RouteControl>,
}

static DESKTOP_CONTROL_COORDINATOR: OnceLock<DesktopControlCoordinator> = OnceLock::new();

pub(crate) fn global_desktop_control_coordinator() -> &'static DesktopControlCoordinator {
    DESKTOP_CONTROL_COORDINATOR.get_or_init(DesktopControlCoordinator::new)
}

impl DesktopControlCoordinator {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn claim(
        &self,
        route_id: &str,
        claim_id: &str,
    ) -> Result<DesktopInteractionClaim, String> {
        let route = self.route(route_id);
        let mut state = route
            .state
            .lock()
            .map_err(|_| "desktop_control_coordinator_poisoned".to_string())?;
        if state.controller_mutation_in_flight || state.interaction_claim_id.is_some() {
            return Err("desktop_interaction_conflict".to_string());
        }
        state.interaction_claim_id = Some(claim_id.to_string());
        state.interaction_cancelled = false;
        drop(state);
        Ok(DesktopInteractionClaim {
            route_id: route_id.to_string(),
            claim_id: claim_id.to_string(),
            route,
        })
    }

    pub(crate) fn begin_controller_mutation(
        &self,
        route_id: &str,
    ) -> Result<DesktopControllerMutationGuard, String> {
        let route = self.route(route_id);
        let mut state = route
            .state
            .lock()
            .map_err(|_| "desktop_control_coordinator_poisoned".to_string())?;
        while state.controller_mutation_in_flight {
            state = route
                .changed
                .wait(state)
                .map_err(|_| "desktop_control_coordinator_poisoned".to_string())?;
        }
        state.controller_mutation_in_flight = true;
        if state.interaction_claim_id.is_some() {
            state.interaction_cancelled = true;
        }
        while state.event_in_flight {
            state = route
                .changed
                .wait(state)
                .map_err(|_| "desktop_control_coordinator_poisoned".to_string())?;
        }
        drop(state);
        Ok(DesktopControllerMutationGuard { route })
    }

    fn route(&self, route_id: &str) -> Arc<RouteControl> {
        let mut routes = self
            .routes
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        routes
            .entry(route_id.to_string())
            .or_insert_with(|| Arc::new(RouteControl::default()))
            .clone()
    }
}

impl DesktopInteractionClaim {
    pub(crate) fn route_id(&self) -> &str {
        &self.route_id
    }

    pub(crate) fn begin_event(&self) -> Result<DesktopControlEventGuard, String> {
        let mut state = self
            .route
            .state
            .lock()
            .map_err(|_| "desktop_control_coordinator_poisoned".to_string())?;
        if state.interaction_claim_id.as_deref() != Some(self.claim_id.as_str())
            || state.interaction_cancelled
            || state.controller_mutation_in_flight
        {
            return Err("desktop_interaction_authority_changed".to_string());
        }
        if state.event_in_flight {
            return Err("desktop_interaction_conflict".to_string());
        }
        state.event_in_flight = true;
        drop(state);
        Ok(DesktopControlEventGuard {
            route: self.route.clone(),
        })
    }
}

impl Drop for DesktopInteractionClaim {
    fn drop(&mut self) {
        let mut state = self
            .route
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.interaction_claim_id.as_deref() == Some(self.claim_id.as_str()) {
            state.interaction_claim_id = None;
            state.interaction_cancelled = false;
            self.route.changed.notify_all();
        }
    }
}

impl Drop for DesktopControlEventGuard {
    fn drop(&mut self) {
        let mut state = self
            .route
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.event_in_flight = false;
        self.route.changed.notify_all();
    }
}

impl Drop for DesktopControllerMutationGuard {
    fn drop(&mut self) {
        let mut state = self
            .route
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.controller_mutation_in_flight = false;
        self.route.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::DesktopControlCoordinator;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn controller_mutation_cancels_and_drains_the_current_event() {
        let coordinator = Arc::new(DesktopControlCoordinator::new());
        let claim = coordinator.claim("route-a", "interaction-a").unwrap();
        let event = claim.begin_event().unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        let mutation_coordinator = coordinator.clone();
        let mutation = thread::spawn(move || {
            started_tx.send(()).unwrap();
            let guard = mutation_coordinator
                .begin_controller_mutation("route-a")
                .unwrap();
            finished_tx.send(()).unwrap();
            drop(guard);
        });

        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(finished_rx.recv_timeout(Duration::from_millis(50)).is_err());
        drop(event);
        finished_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(
            claim.begin_event().unwrap_err(),
            "desktop_interaction_authority_changed"
        );
        mutation.join().unwrap();
    }

    #[test]
    fn unrelated_routes_do_not_share_an_event_or_mutation_fence() {
        let coordinator = DesktopControlCoordinator::new();
        let claim = coordinator.claim("route-a", "interaction-a").unwrap();
        let _event = claim.begin_event().unwrap();

        let mutation = coordinator.begin_controller_mutation("route-b").unwrap();
        let route_a_conflict = coordinator.claim("route-a", "interaction-b").unwrap_err();
        assert_eq!(route_a_conflict, "desktop_interaction_conflict");
        drop(mutation);
        let route_b_claim = coordinator.claim("route-b", "interaction-b").unwrap();
        assert_eq!(route_b_claim.route_id(), "route-b");
    }

    #[test]
    fn mutation_fence_rejects_a_new_claim_until_persisted_change_finishes() {
        let coordinator = DesktopControlCoordinator::new();
        let mutation = coordinator.begin_controller_mutation("route-a").unwrap();

        assert_eq!(
            coordinator.claim("route-a", "interaction-a").unwrap_err(),
            "desktop_interaction_conflict"
        );
        drop(mutation);
        assert!(coordinator.claim("route-a", "interaction-a").is_ok());
    }
}
