//! Process-owned, route-scoped serialization for desktop controller mutation.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use agent_browser_desktop_services::DesktopControllerMutationGuard as ProcessMutationGuard;
pub(crate) use agent_browser_desktop_services::{
    DesktopControlCoordinator, DesktopControlEventGuard, DesktopInteractionClaim,
};

use super::desktop_input_provider::{RouteEffectFence, RouteEffectFenceIdentity};
use super::service_model::ServiceState;
use super::service_store::default_service_state_path;

const CONTROLLER_MUTATION_FENCE_DEADLINE: Duration = Duration::from_secs(5);

pub(crate) struct DesktopControllerMutationGuard {
    _process_guard: ProcessMutationGuard,
    _external_fence: Option<RouteEffectFence>,
}

static DESKTOP_CONTROL_COORDINATOR: OnceLock<DesktopControlCoordinator> = OnceLock::new();

pub(crate) fn global_desktop_control_coordinator() -> &'static DesktopControlCoordinator {
    DESKTOP_CONTROL_COORDINATOR.get_or_init(DesktopControlCoordinator::new)
}

/// Serialize a route controller-authority mutation with desktop input effects.
///
/// The route and display identities come only from the current service state.
/// Callers cannot supply a display, lock path, or provider-owned coordinate.
pub(crate) fn begin_service_controller_mutation(
    state: &ServiceState,
    route_id: &str,
) -> Result<DesktopControllerMutationGuard, String> {
    let (runtime_state_root, identity) = service_route_fence_scope(state, route_id)?;
    global_desktop_control_coordinator().begin_fenced_controller_mutation(
        route_id,
        &runtime_state_root,
        &identity,
        CONTROLLER_MUTATION_FENCE_DEADLINE,
    )
}

/// Focus does not transfer controller authority or cancel an existing claim.
/// Holding the event and external fence serializes focus with desktop effects;
/// the caller must revalidate its operator proof before performing any effect.
#[derive(Debug)]
pub(crate) struct OperatorFocusGuard {
    _external_fence: RouteEffectFence,
    _event: DesktopControlEventGuard,
    _claim: DesktopInteractionClaim,
}

pub(crate) fn begin_service_operator_focus(
    state: &ServiceState,
    route_id: &str,
    request_id: &str,
) -> Result<OperatorFocusGuard, String> {
    let (runtime_state_root, identity) = service_route_fence_scope(state, route_id)?;
    global_desktop_control_coordinator().begin_fenced_focus(
        route_id,
        request_id,
        &runtime_state_root,
        &identity,
    )
}

fn service_route_fence_scope(
    state: &ServiceState,
    route_id: &str,
) -> Result<(PathBuf, RouteEffectFenceIdentity), String> {
    let route = state
        .remote_view_routes
        .get(route_id)
        .ok_or_else(|| "desktop_control_route_not_found".to_string())?;
    let display_allocation_id = route
        .display_allocation_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "desktop_control_display_binding_missing".to_string())?;
    let state_path = default_service_state_path()?;
    let runtime_state_root = state_path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "desktop_control_runtime_state_root_unavailable".to_string())?;
    let environment_id = std::env::var("AGENT_BROWSER_RUNTIME_ENVIRONMENT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "production".to_string());
    let identity = RouteEffectFenceIdentity::new(environment_id, route_id, display_allocation_id);
    Ok((runtime_state_root.to_path_buf(), identity))
}

trait FencedDesktopControlCoordinator {
    fn begin_fenced_focus(
        &self,
        route_id: &str,
        request_id: &str,
        runtime_state_root: &Path,
        identity: &RouteEffectFenceIdentity,
    ) -> Result<OperatorFocusGuard, String>;

    fn begin_fenced_controller_mutation(
        &self,
        route_id: &str,
        runtime_state_root: &Path,
        identity: &RouteEffectFenceIdentity,
        deadline: Duration,
    ) -> Result<DesktopControllerMutationGuard, String>;
}

impl FencedDesktopControlCoordinator for DesktopControlCoordinator {
    fn begin_fenced_focus(
        &self,
        route_id: &str,
        request_id: &str,
        runtime_state_root: &Path,
        identity: &RouteEffectFenceIdentity,
    ) -> Result<OperatorFocusGuard, String> {
        let claim = self.claim(route_id, request_id)?;
        let event = claim.begin_event()?;
        let external_fence = RouteEffectFence::acquire(
            runtime_state_root,
            identity,
            CONTROLLER_MUTATION_FENCE_DEADLINE,
        )
        .map_err(|error| error.code().to_string())?;
        Ok(OperatorFocusGuard {
            _external_fence: external_fence,
            _event: event,
            _claim: claim,
        })
    }

    fn begin_fenced_controller_mutation(
        &self,
        route_id: &str,
        runtime_state_root: &Path,
        identity: &RouteEffectFenceIdentity,
        deadline: Duration,
    ) -> Result<DesktopControllerMutationGuard, String> {
        let process_guard = self.begin_controller_mutation(route_id)?;
        let external_fence = RouteEffectFence::acquire(runtime_state_root, identity, deadline)
            .map_err(|error| error.code().to_string())?;
        Ok(DesktopControllerMutationGuard {
            _process_guard: process_guard,
            _external_fence: Some(external_fence),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DesktopControlCoordinator, FencedDesktopControlCoordinator};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn rejected_focus_preserves_existing_claim_and_releases_its_own_fences() {
        use crate::native::desktop_input_provider::{RouteEffectFence, RouteEffectFenceIdentity};
        let root = std::env::temp_dir().join(format!(
            "agent-browser-operator-focus-fence-{}",
            uuid::Uuid::new_v4()
        ));
        let identity = RouteEffectFenceIdentity::new("development", "route-a", "display-a");
        let coordinator = DesktopControlCoordinator::new();
        let original = coordinator.claim("route-a", "original-agent").unwrap();
        assert_eq!(
            coordinator
                .begin_fenced_focus("route-a", "focus", &root, &identity)
                .unwrap_err(),
            "desktop_interaction_conflict"
        );
        // Unlike controller mutation, refused focus does not cancel the agent.
        drop(original.begin_event().unwrap());
        assert!(!root.exists());
        drop(original);

        let focus = coordinator
            .begin_fenced_focus("route-a", "focus", &root, &identity)
            .unwrap();
        assert!(coordinator.claim("route-a", "other").is_err());
        assert!(RouteEffectFence::acquire(&root, &identity, Duration::ZERO).is_err());
        // A failed post-lock proof check drops this guard without any effect.
        drop(focus);
        let resumed = coordinator.claim("route-a", "resumed").unwrap();
        drop(resumed.begin_event().unwrap());
        drop(RouteEffectFence::acquire(&root, &identity, Duration::ZERO).unwrap());
        drop(resumed);
        std::fs::remove_dir_all(root).unwrap();
    }

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

    #[test]
    fn controller_mutation_waits_for_the_external_route_effect_fence() {
        use crate::native::desktop_input_provider::{RouteEffectFence, RouteEffectFenceIdentity};
        use std::fs;

        let root = std::env::temp_dir().join(format!(
            "agent-browser-controller-route-fence-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let identity = RouteEffectFenceIdentity::new("development", "route-a", "display-a");
        let held = RouteEffectFence::acquire(&root, &identity, Duration::ZERO).unwrap();
        let coordinator = Arc::new(DesktopControlCoordinator::new());
        let mutation_coordinator = coordinator.clone();
        let mutation_root = root.clone();
        let mutation_identity = identity.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        let mutation = thread::spawn(move || {
            started_tx.send(()).unwrap();
            let guard = mutation_coordinator
                .begin_fenced_controller_mutation(
                    "route-a",
                    &mutation_root,
                    &mutation_identity,
                    Duration::from_secs(1),
                )
                .unwrap();
            finished_tx.send(()).unwrap();
            drop(guard);
        });

        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(finished_rx.recv_timeout(Duration::from_millis(50)).is_err());
        drop(held);
        finished_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        mutation.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
