use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};

/// Process-local serialization for one route's controller mutations and input.
#[derive(Default)]
pub struct DesktopControlCoordinator {
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
pub struct DesktopInteractionClaim {
    route_id: String,
    claim_id: String,
    route: Arc<RouteControl>,
}

#[derive(Debug)]
pub struct DesktopControlEventGuard {
    route: Arc<RouteControl>,
}

pub struct DesktopControllerMutationGuard {
    route: Arc<RouteControl>,
}

impl DesktopControlCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn claim(&self, route_id: &str, claim_id: &str) -> Result<DesktopInteractionClaim, String> {
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

    pub fn begin_controller_mutation(
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
    pub fn route_id(&self) -> &str {
        &self.route_id
    }

    pub fn begin_event(&self) -> Result<DesktopControlEventGuard, String> {
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

    pub fn begin_cleanup_event(&self) -> Result<DesktopControlEventGuard, String> {
        let mut state = self
            .route
            .state
            .lock()
            .map_err(|_| "desktop_control_coordinator_poisoned".to_string())?;
        while state.controller_mutation_in_flight || state.event_in_flight {
            state = self
                .route
                .changed
                .wait(state)
                .map_err(|_| "desktop_control_coordinator_poisoned".to_string())?;
        }
        if state.interaction_claim_id.as_deref() != Some(self.claim_id.as_str()) {
            return Err("desktop_interaction_authority_changed".to_string());
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
    fn controller_mutation_cancels_and_drains_current_event() {
        let coordinator = Arc::new(DesktopControlCoordinator::new());
        let claim = coordinator.claim("route-a", "interaction-a").unwrap();
        let event = claim.begin_event().unwrap();
        let (finished_tx, finished_rx) = mpsc::channel();
        let mutation_coordinator = coordinator.clone();
        let mutation = thread::spawn(move || {
            let guard = mutation_coordinator
                .begin_controller_mutation("route-a")
                .unwrap();
            finished_tx.send(()).unwrap();
            drop(guard);
        });

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
    fn unrelated_routes_do_not_share_a_fence() {
        let coordinator = DesktopControlCoordinator::new();
        let claim_a = coordinator.claim("route-a", "interaction-a").unwrap();
        let event_a = claim_a.begin_event().unwrap();
        let claim_b = coordinator.claim("route-b", "interaction-b").unwrap();
        let event_b = claim_b.begin_event().unwrap();
        drop(event_b);
        drop(claim_b);
        drop(event_a);
        drop(claim_a);
    }

    #[test]
    fn duplicate_route_claim_is_rejected_until_release() {
        let coordinator = DesktopControlCoordinator::new();
        let first = coordinator.claim("route-a", "interaction-a").unwrap();
        assert_eq!(
            coordinator.claim("route-a", "interaction-b").unwrap_err(),
            "desktop_interaction_conflict"
        );
        drop(first);
        let second = coordinator.claim("route-a", "interaction-b").unwrap();
        drop(second);
    }
}
