//! Provider-neutral current-boot presentation inventory requalification.
//!
//! This module deliberately models only the stable configured route/display
//! relationship and an observer's current-boot evidence. Retained claims from
//! earlier boots are historical context, never admission authority.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfiguredPresentationRoute {
    pub(crate) route_id: String,
    pub(crate) display_id: String,
    pub(crate) ownership_identity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetainedPresentationClaimState {
    Active,
    Quarantined,
    Orphaned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetainedPresentationClaim {
    pub(crate) boot_epoch: String,
    pub(crate) route_id: String,
    pub(crate) display_id: String,
    pub(crate) state: RetainedPresentationClaimState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObservedPresentationRoute {
    pub(crate) route_id: String,
    pub(crate) display_id: String,
    pub(crate) ownership_identity: Option<String>,
    pub(crate) healthy: bool,
}

pub(crate) trait PresentationRouteObserver {
    fn observe(
        &mut self,
        configured: &ConfiguredPresentationRoute,
    ) -> Result<ObservedPresentationRoute, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PresentationRequalificationInput {
    pub(crate) current_boot_epoch: String,
    pub(crate) configured_routes: Vec<ConfiguredPresentationRoute>,
    pub(crate) retained_claims: Vec<RetainedPresentationClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdmittedPresentationBinding {
    pub(crate) boot_epoch: String,
    pub(crate) route_id: String,
    pub(crate) display_id: String,
    pub(crate) ownership_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PresentationRequalificationExclusionReason {
    InvalidConfiguration,
    DuplicateConfiguration,
    Quarantined,
    Orphaned,
    ObservationFailed { message: String },
    RouteMismatch { observed_route_id: String },
    DisplayMismatch { observed_display_id: String },
    Unhealthy,
    OwnershipUnproven,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PresentationRequalificationExclusion {
    pub(crate) route_id: String,
    pub(crate) display_id: String,
    pub(crate) reason: PresentationRequalificationExclusionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PresentationRequalificationReceipt {
    pub(crate) admitted: Vec<AdmittedPresentationBinding>,
    pub(crate) excluded: Vec<PresentationRequalificationExclusion>,
}

/// Re-observes stable configured presentation routes for the current boot.
///
/// A matching retained claim can only exclude a route when it belongs to this
/// boot and is currently quarantined or orphaned. Claims from any other boot
/// are deliberately ignored so they cannot veto fresh observation.
pub(crate) fn requalify_presentation_inventory(
    input: &PresentationRequalificationInput,
    observer: &mut dyn PresentationRouteObserver,
) -> PresentationRequalificationReceipt {
    let mut configured_routes = input.configured_routes.clone();
    configured_routes.sort();
    let duplicate_configurations = duplicate_configurations(&configured_routes);

    let mut admitted = Vec::new();
    let mut excluded = Vec::new();

    for configured in configured_routes {
        let exclusion = |reason| PresentationRequalificationExclusion {
            route_id: configured.route_id.clone(),
            display_id: configured.display_id.clone(),
            reason,
        };

        if configured.route_id.is_empty()
            || configured.display_id.is_empty()
            || configured.ownership_identity.is_empty()
        {
            excluded.push(exclusion(
                PresentationRequalificationExclusionReason::InvalidConfiguration,
            ));
            continue;
        }

        if duplicate_configurations
            .contains(&(configured.route_id.clone(), configured.display_id.clone()))
        {
            excluded.push(exclusion(
                PresentationRequalificationExclusionReason::DuplicateConfiguration,
            ));
            continue;
        }

        if current_claim_state(input, &configured)
            == Some(RetainedPresentationClaimState::Quarantined)
        {
            excluded.push(exclusion(
                PresentationRequalificationExclusionReason::Quarantined,
            ));
            continue;
        }
        if current_claim_state(input, &configured) == Some(RetainedPresentationClaimState::Orphaned)
        {
            excluded.push(exclusion(
                PresentationRequalificationExclusionReason::Orphaned,
            ));
            continue;
        }

        match observer.observe(&configured) {
            Err(message) => excluded.push(exclusion(
                PresentationRequalificationExclusionReason::ObservationFailed { message },
            )),
            Ok(observed) if observed.route_id != configured.route_id => excluded.push(exclusion(
                PresentationRequalificationExclusionReason::RouteMismatch {
                    observed_route_id: observed.route_id,
                },
            )),
            Ok(observed) if observed.display_id != configured.display_id => {
                excluded.push(exclusion(
                    PresentationRequalificationExclusionReason::DisplayMismatch {
                        observed_display_id: observed.display_id,
                    },
                ))
            }
            Ok(observed) if !observed.healthy => {
                excluded.push(exclusion(
                    PresentationRequalificationExclusionReason::Unhealthy,
                ));
            }
            Ok(observed)
                if observed.ownership_identity.as_deref()
                    != Some(configured.ownership_identity.as_str()) =>
            {
                excluded.push(exclusion(
                    PresentationRequalificationExclusionReason::OwnershipUnproven,
                ));
            }
            Ok(_) => admitted.push(AdmittedPresentationBinding {
                boot_epoch: input.current_boot_epoch.clone(),
                route_id: configured.route_id,
                display_id: configured.display_id,
                ownership_identity: configured.ownership_identity,
            }),
        }
    }

    PresentationRequalificationReceipt { admitted, excluded }
}

fn duplicate_configurations(
    configured_routes: &[ConfiguredPresentationRoute],
) -> BTreeSet<(String, String)> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for configured in configured_routes {
        let identity = (configured.route_id.clone(), configured.display_id.clone());
        if !seen.insert(identity.clone()) {
            duplicates.insert(identity);
        }
    }
    duplicates
}

fn current_claim_state(
    input: &PresentationRequalificationInput,
    configured: &ConfiguredPresentationRoute,
) -> Option<RetainedPresentationClaimState> {
    input
        .retained_claims
        .iter()
        .filter(|claim| {
            claim.boot_epoch == input.current_boot_epoch
                && claim.route_id == configured.route_id
                && claim.display_id == configured.display_id
        })
        .map(|claim| claim.state)
        .max_by_key(|state| match state {
            RetainedPresentationClaimState::Active => 0,
            RetainedPresentationClaimState::Orphaned => 1,
            RetainedPresentationClaimState::Quarantined => 2,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct FixtureObserver {
        observations: BTreeMap<(String, String), Result<ObservedPresentationRoute, String>>,
        observed: Vec<(String, String)>,
    }

    impl FixtureObserver {
        fn observed_route(mut self, route: &str, display: &str, healthy: bool) -> Self {
            self.observations.insert(
                (route.to_string(), display.to_string()),
                Ok(ObservedPresentationRoute {
                    route_id: route.to_string(),
                    display_id: display.to_string(),
                    ownership_identity: Some(format!("owner-{route}")),
                    healthy,
                }),
            );
            self
        }
    }

    impl PresentationRouteObserver for FixtureObserver {
        fn observe(
            &mut self,
            configured: &ConfiguredPresentationRoute,
        ) -> Result<ObservedPresentationRoute, String> {
            self.observed
                .push((configured.route_id.clone(), configured.display_id.clone()));
            self.observations
                .remove(&(configured.route_id.clone(), configured.display_id.clone()))
                .unwrap_or_else(|| Err("observation_missing".to_string()))
        }
    }

    fn configured(route_id: &str, display_id: &str) -> ConfiguredPresentationRoute {
        ConfiguredPresentationRoute {
            route_id: route_id.to_string(),
            display_id: display_id.to_string(),
            ownership_identity: format!("owner-{route_id}"),
        }
    }

    fn input(routes: Vec<ConfiguredPresentationRoute>) -> PresentationRequalificationInput {
        PresentationRequalificationInput {
            current_boot_epoch: "boot-22".to_string(),
            configured_routes: routes,
            retained_claims: Vec::new(),
        }
    }

    #[test]
    fn changed_boot_reobserves_prior_quarantine_and_admits_the_healthy_route() {
        let mut input = input(vec![configured("route-a", "display-a")]);
        input.retained_claims.push(RetainedPresentationClaim {
            boot_epoch: "boot-21".to_string(),
            route_id: "route-a".to_string(),
            display_id: "display-a".to_string(),
            state: RetainedPresentationClaimState::Quarantined,
        });
        let mut observer = FixtureObserver::default().observed_route("route-a", "display-a", true);

        let receipt = requalify_presentation_inventory(&input, &mut observer);

        assert_eq!(receipt.admitted.len(), 1);
        assert!(receipt.excluded.is_empty());
        assert_eq!(
            observer.observed,
            vec![("route-a".to_string(), "display-a".to_string())]
        );
    }

    #[test]
    fn partial_recovery_admits_only_the_independently_healthy_route_in_sorted_order() {
        let input = input(vec![
            configured("route-b", "display-b"),
            configured("route-a", "display-a"),
        ]);
        let mut observer = FixtureObserver::default()
            .observed_route("route-a", "display-a", true)
            .observed_route("route-b", "display-b", false);

        let receipt = requalify_presentation_inventory(&input, &mut observer);

        assert_eq!(
            receipt
                .admitted
                .iter()
                .map(|binding| binding.route_id.as_str())
                .collect::<Vec<_>>(),
            vec!["route-a"]
        );
        assert_eq!(receipt.excluded[0].route_id, "route-b");
        assert_eq!(
            receipt.excluded[0].reason,
            PresentationRequalificationExclusionReason::Unhealthy
        );
        assert_eq!(
            observer.observed,
            vec![
                ("route-a".to_string(), "display-a".to_string()),
                ("route-b".to_string(), "display-b".to_string()),
            ]
        );
    }

    #[test]
    fn current_boot_quarantine_excludes_without_observation() {
        let mut input = input(vec![configured("route-a", "display-a")]);
        input.retained_claims.push(RetainedPresentationClaim {
            boot_epoch: "boot-22".to_string(),
            route_id: "route-a".to_string(),
            display_id: "display-a".to_string(),
            state: RetainedPresentationClaimState::Quarantined,
        });
        let mut observer = FixtureObserver::default();

        let receipt = requalify_presentation_inventory(&input, &mut observer);

        assert_eq!(
            receipt.excluded[0].reason,
            PresentationRequalificationExclusionReason::Quarantined
        );
        assert!(observer.observed.is_empty());
    }

    #[test]
    fn current_boot_orphan_excludes_without_observation() {
        let mut input = input(vec![configured("route-a", "display-a")]);
        input.retained_claims.push(RetainedPresentationClaim {
            boot_epoch: "boot-22".to_string(),
            route_id: "route-a".to_string(),
            display_id: "display-a".to_string(),
            state: RetainedPresentationClaimState::Orphaned,
        });
        let mut observer = FixtureObserver::default();

        let receipt = requalify_presentation_inventory(&input, &mut observer);

        assert_eq!(
            receipt.excluded[0].reason,
            PresentationRequalificationExclusionReason::Orphaned
        );
        assert!(observer.observed.is_empty());
    }

    #[test]
    fn active_claim_is_not_ownership_proof_without_a_current_observation() {
        let mut input = input(vec![configured("route-a", "display-a")]);
        input.retained_claims.push(RetainedPresentationClaim {
            boot_epoch: "boot-22".to_string(),
            route_id: "route-a".to_string(),
            display_id: "display-a".to_string(),
            state: RetainedPresentationClaimState::Active,
        });
        let mut observer = FixtureObserver::default().observed_route("route-a", "display-a", true);

        let receipt = requalify_presentation_inventory(&input, &mut observer);

        assert_eq!(receipt.admitted.len(), 1);
        assert_eq!(observer.observed.len(), 1);
    }

    #[test]
    fn healthy_observation_without_exact_ownership_is_excluded() {
        let input = input(vec![configured("route-a", "display-a")]);
        let mut observer = FixtureObserver::default();
        observer.observations.insert(
            ("route-a".to_string(), "display-a".to_string()),
            Ok(ObservedPresentationRoute {
                route_id: "route-a".to_string(),
                display_id: "display-a".to_string(),
                ownership_identity: Some("foreign-owner".to_string()),
                healthy: true,
            }),
        );

        let receipt = requalify_presentation_inventory(&input, &mut observer);

        assert_eq!(
            receipt.excluded[0].reason,
            PresentationRequalificationExclusionReason::OwnershipUnproven
        );
    }

    #[test]
    fn route_and_display_mismatches_are_typed_exclusions() {
        let input = input(vec![
            configured("route-a", "display-a"),
            configured("route-b", "display-b"),
        ]);
        let mut observer = FixtureObserver::default();
        observer.observations.insert(
            ("route-a".to_string(), "display-a".to_string()),
            Ok(ObservedPresentationRoute {
                route_id: "different-route".to_string(),
                display_id: "display-a".to_string(),
                ownership_identity: Some("owner-route-a".to_string()),
                healthy: true,
            }),
        );
        observer.observations.insert(
            ("route-b".to_string(), "display-b".to_string()),
            Ok(ObservedPresentationRoute {
                route_id: "route-b".to_string(),
                display_id: "different-display".to_string(),
                ownership_identity: Some("owner-route-b".to_string()),
                healthy: true,
            }),
        );

        let receipt = requalify_presentation_inventory(&input, &mut observer);

        assert_eq!(
            receipt.excluded[0].reason,
            PresentationRequalificationExclusionReason::RouteMismatch {
                observed_route_id: "different-route".to_string()
            }
        );
        assert_eq!(
            receipt.excluded[1].reason,
            PresentationRequalificationExclusionReason::DisplayMismatch {
                observed_display_id: "different-display".to_string()
            }
        );
    }

    #[test]
    fn observation_failure_is_a_typed_exclusion() {
        let input = input(vec![configured("route-a", "display-a")]);
        let mut observer = FixtureObserver::default();
        observer.observations.insert(
            ("route-a".to_string(), "display-a".to_string()),
            Err("provider_unreachable".to_string()),
        );

        let receipt = requalify_presentation_inventory(&input, &mut observer);

        assert_eq!(
            receipt.excluded[0].reason,
            PresentationRequalificationExclusionReason::ObservationFailed {
                message: "provider_unreachable".to_string()
            }
        );
    }
}
