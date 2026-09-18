//! Boot-scoped provenance for package-owned ephemeral Service State evidence.
//!
//! Stable profile, logical-browser, route, connection, route-user, and durable
//! handoff identities intentionally remain outside this model. A non-current
//! epoch requires rediscovery and never authorizes deletion by itself.

use super::service_model::{LeaseState, ServiceState};
use crate::process_identity::{boot_epoch_status, BootEpochStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BootEpochFinding {
    pub(crate) resource_type: String,
    pub(crate) resource_id: String,
    pub(crate) status: BootEpochStatus,
    pub(crate) recorded_boot_epoch: Option<String>,
    pub(crate) current_boot_epoch: Option<String>,
    pub(crate) recourse: String,
    pub(crate) authorizes_effects: bool,
    pub(crate) authorizes_cleanup: bool,
}

pub(crate) fn assess_boot_scoped_observation(
    resource_type: &str,
    resource_id: &str,
    recorded_boot_epoch: Option<&str>,
    current_boot_epoch: Option<&str>,
) -> Option<BootEpochFinding> {
    let status = boot_epoch_status(recorded_boot_epoch, current_boot_epoch);
    if status == BootEpochStatus::Current {
        return None;
    }
    Some(BootEpochFinding {
        resource_type: resource_type.to_string(),
        resource_id: resource_id.to_string(),
        status,
        recorded_boot_epoch: recorded_boot_epoch.map(ToOwned::to_owned),
        current_boot_epoch: current_boot_epoch.map(ToOwned::to_owned),
        recourse: "rediscover_current_evidence".to_string(),
        authorizes_effects: false,
        authorizes_cleanup: false,
    })
}

pub(crate) fn service_boot_epoch_findings(
    state: &ServiceState,
    current_boot_epoch: Option<&str>,
) -> Vec<BootEpochFinding> {
    let mut findings = Vec::new();
    for browser in state
        .browsers
        .values()
        .filter(|browser| browser.pid.is_some())
    {
        push_finding(
            &mut findings,
            "browser_process",
            &browser.id,
            browser.boot_epoch.as_deref(),
            current_boot_epoch,
        );
    }
    for allocation in state.display_allocations.values().filter(|allocation| {
        allocation.pid_hints.is_some()
            || matches!(
                allocation.state.as_str(),
                "allocating" | "ready" | "reconnecting"
            )
    }) {
        push_finding(
            &mut findings,
            "display_allocation",
            &allocation.id,
            allocation.boot_epoch.as_deref(),
            current_boot_epoch,
        );
    }
    for lease in state
        .remote_view_acquisition_leases
        .values()
        .filter(|lease| !matches!(lease.state.as_str(), "completed" | "failed" | "released"))
    {
        push_finding(
            &mut findings,
            "remote_view_acquisition_lease",
            &lease.id,
            lease.boot_epoch.as_deref(),
            current_boot_epoch,
        );
    }
    for lease in state
        .viewer_leases
        .values()
        .filter(|lease| !matches!(lease.state.as_str(), "released" | "expired" | "failed"))
    {
        push_finding(
            &mut findings,
            "viewer_lease",
            &lease.id,
            lease.boot_epoch.as_deref(),
            current_boot_epoch,
        );
    }
    for session in state.sessions.values().filter(|session| {
        session.work_lease_id.is_some()
            || session.last_lease_observed_at.is_some()
            || !matches!(session.lease, LeaseState::Released | LeaseState::Expired)
    }) {
        push_finding(
            &mut findings,
            "session_lease_observation",
            &session.id,
            session.boot_epoch.as_deref(),
            current_boot_epoch,
        );
    }
    for lifecycle in state.runtime_lifecycle_boot_epoch_observations() {
        push_finding(
            &mut findings,
            "runtime_lifecycle",
            &lifecycle.logical_browser_id,
            lifecycle.boot_epoch.as_deref(),
            current_boot_epoch,
        );
    }
    findings.sort_by(|left, right| {
        left.resource_type
            .cmp(&right.resource_type)
            .then_with(|| left.resource_id.cmp(&right.resource_id))
    });
    findings
}

fn push_finding(
    findings: &mut Vec<BootEpochFinding>,
    resource_type: &str,
    resource_id: &str,
    recorded_boot_epoch: Option<&str>,
    current_boot_epoch: Option<&str>,
) {
    if let Some(finding) = assess_boot_scoped_observation(
        resource_type,
        resource_id,
        recorded_boot_epoch,
        current_boot_epoch,
    ) {
        findings.push(finding);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserProcess, DisplayAllocation, RemoteViewAcquisitionLease, ViewerLease,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn prior_boot_evidence_requires_rediscovery_and_never_authorizes_cleanup() {
        let prior = "boot:synthetic-previous".to_string();
        let mut state = ServiceState {
            browsers: BTreeMap::from([(
                "stable-browser".to_string(),
                BrowserProcess {
                    id: "stable-browser".to_string(),
                    boot_epoch: Some(prior.clone()),
                    pid: Some(41001),
                    ..BrowserProcess::default()
                },
            )]),
            display_allocations: BTreeMap::from([(
                "stable-display".to_string(),
                DisplayAllocation {
                    id: "stable-display".to_string(),
                    boot_epoch: Some(prior.clone()),
                    state: "ready".to_string(),
                    pid_hints: Some(json!({"xrdp": 41002})),
                    ..DisplayAllocation::default()
                },
            )]),
            remote_view_acquisition_leases: BTreeMap::from([(
                "stable-acquisition".to_string(),
                RemoteViewAcquisitionLease {
                    id: "stable-acquisition".to_string(),
                    boot_epoch: Some(prior.clone()),
                    ..RemoteViewAcquisitionLease::default()
                },
            )]),
            viewer_leases: BTreeMap::from([(
                "stable-viewer".to_string(),
                ViewerLease {
                    id: "stable-viewer".to_string(),
                    boot_epoch: Some(prior),
                    state: "active".to_string(),
                    ..ViewerLease::default()
                },
            )]),
            ..ServiceState::default()
        };

        let findings = service_boot_epoch_findings(&state, Some("boot:synthetic-current"));
        assert_eq!(findings.len(), 4);
        assert!(findings
            .iter()
            .all(|finding| finding.status == BootEpochStatus::Prior
                && finding.recourse == "rediscover_current_evidence"
                && !finding.authorizes_effects
                && !finding.authorizes_cleanup));

        state.browsers.get_mut("stable-browser").unwrap().boot_epoch =
            Some("boot:synthetic-current".to_string());
        let findings = service_boot_epoch_findings(&state, Some("boot:synthetic-current"));
        assert!(!findings
            .iter()
            .any(|finding| finding.resource_type == "browser_process"
                && finding.resource_id == "stable-browser"));
        assert!(state.browsers.contains_key("stable-browser"));
    }

    #[test]
    fn missing_legacy_epoch_is_typed_and_preserved() {
        let finding = assess_boot_scoped_observation(
            "runtime_host",
            "stable-host",
            None,
            Some("boot:synthetic-current"),
        )
        .expect("legacy evidence must require rediscovery");
        assert_eq!(finding.status, BootEpochStatus::Missing);
        assert!(!finding.authorizes_effects);
        assert!(!finding.authorizes_cleanup);
    }

    #[test]
    fn lifecycle_boot_evidence_is_included_and_sorted_by_embedded_browser_id() {
        use crate::runtime_owner_transfer::RuntimeLifecycleRecord;

        let mut state = ServiceState::default();
        let mut registry =
            crate::runtime_owner_transfer::edit_registry_fixture(&mut state.runtime_owner_registry);
        registry.lifecycle_rows.insert(
            "z-map-key".to_string(),
            RuntimeLifecycleRecord {
                logical_browser_id: "browser-b".to_string(),
                boot_epoch: Some("boot:synthetic-previous".to_string()),
                process_group_id: Some(4100),
                ..RuntimeLifecycleRecord::default()
            },
        );
        registry.lifecycle_rows.insert(
            "a-map-key".to_string(),
            RuntimeLifecycleRecord {
                logical_browser_id: "browser-a".to_string(),
                boot_epoch: Some("boot:synthetic-previous".to_string()),
                package_launch_identity_digest: Some("a".repeat(64)),
                ..RuntimeLifecycleRecord::default()
            },
        );
        registry.lifecycle_rows.insert(
            "ignored".to_string(),
            RuntimeLifecycleRecord {
                logical_browser_id: "browser-ignored".to_string(),
                boot_epoch: Some("boot:synthetic-previous".to_string()),
                ..RuntimeLifecycleRecord::default()
            },
        );
        drop(registry);

        let lifecycle_findings =
            service_boot_epoch_findings(&state, Some("boot:synthetic-current"))
                .into_iter()
                .filter(|finding| finding.resource_type == "runtime_lifecycle")
                .collect::<Vec<_>>();

        assert_eq!(lifecycle_findings.len(), 2);
        assert_eq!(lifecycle_findings[0].resource_id, "browser-a");
        assert_eq!(lifecycle_findings[1].resource_id, "browser-b");
        assert!(lifecycle_findings
            .iter()
            .all(|finding| finding.status == BootEpochStatus::Prior));
    }
}
