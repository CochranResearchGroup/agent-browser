use super::desktop_evidence::{DesktopEpisodeAdmissionFailure, PresentationSlotAdapter};
use super::presentation_capacity::{CapacityDecision, PresentationRequest, PressureAdmission};
use super::service_store::ServiceStateRepository;

/// Durable presentation-capacity adapter for one configured desktop evidence
/// episode. The adapter never invents a slot when Service State cannot commit
/// the admission or release mutation.
pub(crate) struct ConfiguredPresentationSlotAdapter<R> {
    repository: R,
    request_id: String,
    pressure: PressureAdmission,
    reservation: Option<(String, String)>,
}

impl<R> ConfiguredPresentationSlotAdapter<R>
where
    R: ServiceStateRepository,
{
    pub(crate) fn new(
        repository: R,
        request_id: impl Into<String>,
        admitted_maximum: usize,
    ) -> Self {
        Self {
            repository,
            request_id: request_id.into(),
            pressure: PressureAdmission::admit(admitted_maximum),
            reservation: None,
        }
    }

    fn admission_failure(decision: CapacityDecision) -> DesktopEpisodeAdmissionFailure {
        match decision {
            CapacityDecision::Queued {
                queue_position,
                limiting_resource,
                next_safe_action,
                ..
            } => DesktopEpisodeAdmissionFailure::new(
                "presentation_capacity_queued",
                format!(
                    "queuePosition={queue_position}; limitingResource={limiting_resource:?}; nextSafeAction={next_safe_action:?}"
                ),
            ),
            CapacityDecision::Rejected {
                limiting_resource,
                next_safe_action,
                ..
            } => DesktopEpisodeAdmissionFailure::new(
                "presentation_capacity_rejected",
                format!(
                    "limitingResource={limiting_resource:?}; nextSafeAction={next_safe_action:?}"
                ),
            ),
            CapacityDecision::Granted { .. } => DesktopEpisodeAdmissionFailure::new(
                "presentation_capacity_invalid_decision",
                "granted admission was handled as unavailable",
            ),
        }
    }
}

impl<R> PresentationSlotAdapter for ConfiguredPresentationSlotAdapter<R>
where
    R: ServiceStateRepository,
{
    fn reserve(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdmissionFailure> {
        if self.reservation.is_some() {
            return Err(DesktopEpisodeAdmissionFailure::new(
                "presentation_capacity_duplicate_reservation",
                "the configured episode already owns a presentation slot",
            ));
        }
        let request_id = self.request_id.clone();
        let browser_id = browser_id.to_string();
        let pressure = self.pressure;
        let mut unavailable = None;
        let mutation = self.repository.mutate(|state| {
            let Some(mut capacity) = state.presentation_capacity.take() else {
                return Err("presentation_capacity_unavailable".to_string());
            };
            let decision = capacity.request_with_service_state(
                PresentationRequest::observation(request_id.clone()).for_browser(&browser_id),
                pressure,
                Some(state),
            );
            state.presentation_capacity = Some(capacity);
            match decision {
                CapacityDecision::Granted { slot_id, .. } => Ok(slot_id),
                decision => {
                    unavailable = Some(decision);
                    Err("presentation_capacity_not_admitted".to_string())
                }
            }
        });

        let slot_id = match mutation {
            Ok(slot_id) => slot_id,
            Err(_) if unavailable.is_some() => {
                return Err(Self::admission_failure(unavailable.expect("checked above")));
            }
            Err(error) => {
                return Err(DesktopEpisodeAdmissionFailure::new(
                    "presentation_capacity_persistence_failed",
                    error,
                ));
            }
        };
        self.reservation = Some((browser_id, slot_id.clone()));
        Ok(format!("presentation-admission:{request_id}:{slot_id}"))
    }

    fn release(&mut self, browser_id: &str) -> Result<String, DesktopEpisodeAdmissionFailure> {
        let Some((reserved_browser_id, slot_id)) = self.reservation.clone() else {
            return Err(DesktopEpisodeAdmissionFailure::new(
                "presentation_release_without_reservation",
                "the configured episode does not own a presentation slot",
            ));
        };
        if reserved_browser_id != browser_id {
            return Err(DesktopEpisodeAdmissionFailure::new(
                "presentation_release_browser_mismatch",
                "the release browser does not match the admitted browser",
            ));
        }
        let pressure = self.pressure;
        let mutation = self.repository.mutate(|state| {
            let Some(mut capacity) = state.presentation_capacity.take() else {
                return Err("presentation_capacity_unavailable".to_string());
            };
            if !capacity.slots.iter().any(|slot| slot.id == slot_id) {
                state.presentation_capacity = Some(capacity);
                return Err("presentation_reserved_slot_missing".to_string());
            }
            capacity.release_and_dispatch_with_service_state(&slot_id, pressure, Some(state));
            state.presentation_capacity = Some(capacity);
            Ok(())
        });
        if let Err(error) = mutation {
            return Err(DesktopEpisodeAdmissionFailure::new(
                "presentation_release_persistence_failed",
                error,
            ));
        }
        self.reservation = None;
        Ok(format!(
            "presentation-release:{}:{slot_id}",
            self.request_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::presentation_capacity::{
        PresentationCapacityAuthority, PresentationCapacityConfig, PresentationSlot,
        PresentationSlotState,
    };
    use crate::native::service_model::ServiceState;
    use std::sync::Mutex;

    struct MemoryRepository(Mutex<ServiceState>);

    impl MemoryRepository {
        fn new(state: ServiceState) -> Self {
            Self(Mutex::new(state))
        }
    }

    impl ServiceStateRepository for MemoryRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn mutate<T>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<T, String>,
        ) -> Result<T, String> {
            let mut state = self.0.lock().unwrap();
            let mut candidate = state.clone();
            let result = mutator(&mut candidate)?;
            *state = candidate;
            Ok(result)
        }
    }

    fn state(slot_count: usize) -> ServiceState {
        ServiceState {
            presentation_capacity: Some(
                PresentationCapacityAuthority::new(
                    PresentationCapacityConfig {
                        warm_minimum: slot_count,
                        hard_maximum: slot_count,
                        human_priority_reserve: usize::from(slot_count > 0),
                        recovery_reserve: usize::from(slot_count > 1),
                        max_queue_depth: 8,
                    },
                    (0..slot_count)
                        .map(|index| PresentationSlot::warm_idle(format!("slot-{index}")))
                        .collect(),
                )
                .unwrap(),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn configured_adapter_commits_exact_reservation_and_release() {
        let repository = MemoryRepository::new(state(4));
        let mut adapter = ConfiguredPresentationSlotAdapter::new(repository, "episode-1", 4);

        let admission = adapter.reserve("browser-1").unwrap();
        assert_eq!(admission, "presentation-admission:episode-1:slot-0");
        let snapshot = adapter.repository.load_snapshot().unwrap();
        let slot = &snapshot.presentation_capacity.unwrap().slots[0];
        assert_eq!(slot.state, PresentationSlotState::Reserved);
        assert_eq!(slot.browser_id.as_deref(), Some("browser-1"));

        let release = adapter.release("browser-1").unwrap();
        assert_eq!(release, "presentation-release:episode-1:slot-0");
        let snapshot = adapter.repository.load_snapshot().unwrap();
        let slot = &snapshot.presentation_capacity.unwrap().slots[0];
        assert_eq!(slot.state, PresentationSlotState::WarmIdle);
        assert_eq!(slot.browser_id, None);
    }

    #[test]
    fn configured_adapter_does_not_persist_unresumable_queue_entries() {
        let repository = MemoryRepository::new(state(2));
        let mut adapter = ConfiguredPresentationSlotAdapter::new(repository, "episode-2", 2);

        let failure = adapter.reserve("browser-2").unwrap_err();
        assert_eq!(failure.code, "presentation_capacity_queued");
        let snapshot = adapter.repository.load_snapshot().unwrap();
        let capacity = snapshot.presentation_capacity.unwrap();
        assert!(capacity.queued_requests.is_empty());
        assert!(capacity
            .slots
            .iter()
            .all(|slot| slot.state == PresentationSlotState::WarmIdle));
    }
}
