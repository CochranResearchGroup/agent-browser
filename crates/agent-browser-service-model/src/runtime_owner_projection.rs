//! Purpose-specific retained runtime-owner evidence. Adapters retain observation,
//! diagnosis formatting, resource policy, and runtime effects.

use agent_browser_lease_authority::{
    ProfileOwner, RuntimeLifecycleRecord, RuntimeOwnerAttestation, RuntimeOwnerPrincipalBinding,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLifecycleAuthoritySummary {
    pub registry_revision: u64,
    pub owner_count: usize,
    pub record_count: usize,
    pub lifecycle_state_counts: BTreeMap<String, usize>,
    pub cleanup_obligation_state_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLifecycleBootEpochObservation {
    pub logical_browser_id: String,
    pub boot_epoch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRuntimeAuthority<'a> {
    pub registry_revision: u64,
    pub owner: Option<&'a ProfileOwner>,
    pub principal_binding: Option<&'a RuntimeOwnerPrincipalBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeControlPlaneAuthority<'a> {
    pub attestation: Option<RuntimeOwnerAttestation>,
    pub current_owner: Option<&'a ProfileOwner>,
    pub lifecycle: Option<&'a RuntimeLifecycleRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLaneAuthority<'a> {
    pub owner: Option<&'a ProfileOwner>,
    pub lifecycle: Option<&'a RuntimeLifecycleRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeResourceLane<'a> {
    pub browser_id: &'a str,
    pub lifecycle: &'a RuntimeLifecycleRecord,
    pub browser_pid: Option<u32>,
    pub tab_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BrowserProcess, BrowserTab, ServiceState};
    use serde_json::{json, Value};

    fn owner(digest: &str, browser: &str, route: &str) -> Value {
        json!({
            "ownerId": "owner", "profileIdentityDigest": digest,
            "state": "ready", "ownerGeneration": 7, "browserId": browser,
            "daemonSessionRoute": route, "processInstanceDigest": "process",
            "browserFamily": "chrome", "cdpEndpointIdentityDigest": "cdp",
            "targetSetDigest": "targets"
        })
    }

    fn fixture(owners: Value, bindings: Value, lifecycle: Value) -> ServiceState {
        // Decode retained evidence directly: projections must not revalidate or
        // conceal historical mismatches accepted by the durable wire contract.
        serde_json::from_value(json!({
            "stateRevision": 31,
            "futureProjectionEvidence": {"preserve": true},
            "runtimeOwnerRegistry": {
                "revision": u64::MAX, "owners": owners,
                "principalBindings": bindings, "lifecycleRecords": lifecycle
            }
        }))
        .unwrap()
    }

    fn assert_session_binding_parity(
        state: &ServiceState,
        session: &str,
    ) -> Result<Option<agent_browser_lease_authority::RuntimeOwnerBinding>, String> {
        let before = state.clone();
        let expected = state.runtime_owner_registry.binding_for_session(session);
        let actual = state.runtime_owner_binding_for_session(session);
        assert_eq!(actual, expected);
        assert_eq!(*state, before);
        actual
    }

    #[test]
    fn session_binding_preserves_absence_exact_route_and_browser_alias() {
        let empty = fixture(json!({}), json!({}), json!({}));
        assert_eq!(assert_session_binding_parity(&empty, "route"), Ok(None));
        let state = fixture(
            json!({"profile": owner("profile", "session:alias", "route")}),
            json!({}),
            json!({}),
        );
        assert!(state.sessions.is_empty());
        assert_eq!(state.service_principal_registry_revision(), 0);
        for (session, effect_capable) in [("route", true), ("alias", false)] {
            let binding = assert_session_binding_parity(&state, session)
                .unwrap()
                .unwrap();
            assert_eq!(binding.effect_capable, effect_capable);
            assert_eq!(
                binding.claim,
                agent_browser_lease_authority::OwnerAuthorityClaim::from_owner(
                    state.profile_runtime_authority("profile").owner.unwrap()
                )
            );
        }
        assert_eq!(assert_session_binding_parity(&state, " route"), Ok(None));
        let mut not_ready = owner("profile", "session:alias", "route");
        not_ready["state"] = json!("orphaned");
        let state = fixture(json!({"profile": not_ready}), json!({}), json!({}));
        assert_eq!(assert_session_binding_parity(&state, "route"), Ok(None));
    }

    #[test]
    fn session_binding_preserves_pending_transfer_and_previous_route_alias_without_authentication()
    {
        use agent_browser_lease_authority::{
            BrowserAdoptionMode, CandidateOwnerAttachment, OwnerTransferRequest,
        };

        let profile_digest = "a".repeat(64);
        let mut current = owner(&profile_digest, "session:alias", "original-route");
        current["processInstanceDigest"] = json!("b".repeat(64));
        current["cdpEndpointIdentityDigest"] = json!("c".repeat(64));
        current["targetSetDigest"] = json!("d".repeat(64));
        let mut state = fixture(
            serde_json::to_value(BTreeMap::from([(profile_digest.clone(), current)])).unwrap(),
            json!({}),
            json!({}),
        );
        let request = OwnerTransferRequest {
            mode: BrowserAdoptionMode::CooperativeTransfer,
            logical_browser_id: "session:alias".into(),
            profile_identity_digest: profile_digest,
            expected_owner_id: Some("owner".into()),
            expected_owner_generation: 7,
            candidate_owner_id: "candidate".into(),
            candidate_daemon_session_route: "candidate-route".into(),
            process_instance_digest: "b".repeat(64),
            browser_family: "chrome".into(),
            cdp_endpoint_identity_digest: "c".repeat(64),
            target_set_digest: "d".repeat(64),
            selected_target_identity_digest: "e".repeat(64),
            transfer_nonce_digest: "f".repeat(64),
        };
        state
            .runtime_owner_registry
            .begin_transfer(request.clone())
            .unwrap();
        assert!(
            assert_session_binding_parity(&state, "original-route")
                .unwrap()
                .unwrap()
                .effect_capable
        );
        state
            .runtime_owner_registry
            .commit_candidate(CandidateOwnerAttachment::from_request(&request, 8))
            .unwrap();
        for (session, effect_capable) in [
            ("original-route", false),
            ("alias", false),
            ("candidate-route", true),
        ] {
            let binding = assert_session_binding_parity(&state, session)
                .unwrap()
                .unwrap();
            assert_eq!(binding.effect_capable, effect_capable);
            assert_eq!(binding.claim.owner_id, "candidate");
            assert_eq!(binding.claim.owner_generation, 8);
            assert_eq!(binding.claim.daemon_session_route, "candidate-route");
        }
        assert!(state.sessions.is_empty());
        assert_eq!(state.service_principal_registry_revision(), 0);
        assert_eq!(state.state_revision(), 31);
        assert_eq!(state.runtime_owner_registry.revision(), u64::MAX);
    }

    #[test]
    fn session_binding_retains_sole_terminal_history_and_exact_history_filtering() {
        let history = json!({
            "logicalBrowserId": "old-browser", "profileIdentityDigest": "old",
            "ownerGeneration": 7, "lifecycleState": "terminal", "cleanupObligationState": "satisfied"
        });
        let sole = fixture(
            json!({"old": owner("old", "old-browser", "route")}),
            json!({}),
            json!({"old-browser": history.clone()}),
        );
        let binding = assert_session_binding_parity(&sole, "route")
            .unwrap()
            .unwrap();
        assert!(binding.effect_capable);
        assert_eq!(binding.claim.profile_identity_digest, "old");
        assert!(agent_browser_lease_authority::owner_binding_in_registry(
            &sole.runtime_owner_registry,
            "route"
        )
        .unwrap()
        .is_none());

        for case in 0..7 {
            let mut row = history.clone();
            let mut key = "old-browser";
            match case {
                0 => {}
                // Embedded logical browser ID is intentionally not a filtering predicate.
                1 => row["logicalBrowserId"] = json!("different-embedded-browser"),
                2 => key = "different-map-key",
                3 => row["profileIdentityDigest"] = json!("different-profile"),
                4 => row["ownerGeneration"] = json!(8),
                5 => row["lifecycleState"] = json!("ready"),
                6 => row["cleanupObligationState"] = json!("owned"),
                _ => unreachable!(),
            }
            let state = fixture(
                json!({"old": owner("old", "old-browser", "route"), "new": owner("new", "new-browser", "route")}),
                json!({}),
                serde_json::to_value(BTreeMap::from([(key, row)])).unwrap(),
            );
            let result = assert_session_binding_parity(&state, "route");
            if case <= 1 {
                assert_eq!(
                    result.unwrap().unwrap().claim.profile_identity_digest,
                    "new"
                );
            } else {
                assert_eq!(result.unwrap_err(), "runtime_owner_session_ambiguous: session 'route' matches multiple profile owners");
            }
        }
    }

    #[test]
    fn session_binding_preserves_current_and_terminal_only_ambiguity() {
        for terminal in [false, true] {
            let records = if terminal {
                json!({
                    "a-browser": {"profileIdentityDigest": "a", "ownerGeneration": 7, "lifecycleState": "terminal", "cleanupObligationState": "satisfied"},
                    "b-browser": {"profileIdentityDigest": "b", "ownerGeneration": 7, "lifecycleState": "terminal", "cleanupObligationState": "satisfied"}
                })
            } else {
                json!({})
            };
            let state = fixture(
                json!({"a": owner("a", "a-browser", "route"), "b": owner("b", "b-browser", "route")}),
                json!({}),
                records,
            );
            assert_eq!(
                assert_session_binding_parity(&state, "route").unwrap_err(),
                "runtime_owner_session_ambiguous: session 'route' matches multiple profile owners"
            );
        }
    }

    #[test]
    fn summary_preserves_all_labels_counts_and_revision() {
        let empty = ServiceState::default().runtime_lifecycle_authority_summary();
        assert_eq!(empty.registry_revision, 0);
        assert_eq!((empty.owner_count, empty.record_count), (0, 0));
        assert!(empty.lifecycle_state_counts.is_empty());
        assert!(empty.cleanup_obligation_state_counts.is_empty());

        let lifecycle_labels = [
            "unknown",
            "planned",
            "launching",
            "ready",
            "retained",
            "transferring",
            "closing",
            "terminal",
            "quarantined",
        ];
        let cleanup_labels = [
            "unknown",
            "owned",
            "transferring",
            "reclaimable",
            "reclaiming",
            "satisfied",
            "quarantined",
        ];
        let mut records = serde_json::Map::new();
        for (index, lifecycle) in lifecycle_labels.iter().enumerate() {
            records.insert(
                format!("key-{index}"),
                json!({
                    "lifecycleState": lifecycle,
                    "cleanupObligationState": cleanup_labels[index % cleanup_labels.len()]
                }),
            );
        }
        let state = fixture(
            json!({"a": owner("a", "b", "r"), "z": owner("z", "c", "s")}),
            json!({}),
            Value::Object(records),
        );
        let before = state.clone();
        let summary = state.runtime_lifecycle_authority_summary();
        assert_eq!(summary.registry_revision, u64::MAX);
        assert_eq!((summary.owner_count, summary.record_count), (2, 9));
        assert_eq!(
            summary.lifecycle_state_counts,
            lifecycle_labels
                .into_iter()
                .map(|label| (label.to_string(), 1))
                .collect::<BTreeMap<_, _>>()
        );
        assert_eq!(
            summary.cleanup_obligation_state_counts,
            cleanup_labels
                .into_iter()
                .enumerate()
                .map(|(index, label)| (label.to_string(), if index < 2 { 2 } else { 1 }))
                .collect::<BTreeMap<_, _>>()
        );
        assert_eq!(state, before);
    }

    #[test]
    fn boot_observations_preserve_inclusion_order_embedded_identity_and_missing_epoch() {
        let state = fixture(
            json!({}),
            json!({}),
            json!({
                "z": {"logicalBrowserId": "first-embedded", "processGroupId": 9, "packageLaunchIdentityDigest": "launch", "bootEpoch": "boot-z"},
                "a": {"logicalBrowserId": "last-embedded", "processGroupId": 0},
                "b": {"logicalBrowserId": "digest-only", "packageLaunchIdentityDigest": "", "bootEpoch": ""},
                "c": {"logicalBrowserId": "excluded", "bootEpoch": "boot-c"}
            }),
        );
        let before = state.clone();
        let observations = state.runtime_lifecycle_boot_epoch_observations();
        assert_eq!(
            observations,
            vec![
                RuntimeLifecycleBootEpochObservation {
                    logical_browser_id: "last-embedded".into(),
                    boot_epoch: None
                },
                RuntimeLifecycleBootEpochObservation {
                    logical_browser_id: "digest-only".into(),
                    boot_epoch: Some("".into())
                },
                RuntimeLifecycleBootEpochObservation {
                    logical_browser_id: "first-embedded".into(),
                    boot_epoch: Some("boot-z".into())
                },
            ]
        );
        assert_eq!(state, before);
    }

    #[test]
    fn profile_and_lane_options_remain_independent_and_unvalidated() {
        let state = fixture(
            json!({"owner": owner("different-embedded-digest", "browser", "route")}),
            json!({
                "binding": {"principalId": "principal", "profileId": "profile", "profileIdentityDigest": "different-binding-digest", "capabilityId": "capability", "provenance": "unproven_legacy", "ownerGeneration": 99},
                "owner": {"principalId": "other-principal", "profileId": "other-profile", "profileIdentityDigest": "mismatch", "capabilityId": "other-capability", "provenance": "registered_capability", "ownerGeneration": 88}
            }),
            json!({"lane": {"logicalBrowserId": "different-browser", "ownerGeneration": 100}}),
        );
        let before = state.clone();
        for (digest, has_owner, has_binding) in [
            ("missing", false, false),
            ("binding", false, true),
            ("owner", true, true),
            (" owner", false, false),
        ] {
            let projection = state.profile_runtime_authority(digest);
            assert_eq!(projection.registry_revision, u64::MAX);
            assert_eq!(projection.owner.is_some(), has_owner);
            assert_eq!(projection.principal_binding.is_some(), has_binding);
        }
        let both = state.profile_runtime_authority("owner");
        assert_eq!(both.owner.unwrap().owner_generation, 7);
        assert_eq!(both.principal_binding.unwrap().owner_generation, 88);
        for (digest, browser, owner_present, lifecycle_present) in [
            ("missing", "missing", false, false),
            ("owner", "missing", true, false),
            ("missing", "lane", false, true),
            ("owner", "lane", true, true),
        ] {
            let lane = state.runtime_lane_authority(digest, browser);
            assert_eq!(lane.owner.is_some(), owner_present);
            assert_eq!(lane.lifecycle.is_some(), lifecycle_present);
        }
        let owner_only = fixture(
            json!({"owner": owner("owner", "b", "r")}),
            json!({}),
            json!({}),
        );
        let projection = owner_only.profile_runtime_authority("owner");
        assert!(projection.owner.is_some());
        assert!(projection.principal_binding.is_none());
        assert_eq!(state, before);
    }

    #[test]
    fn control_plane_preserves_absence_independent_lifecycle_and_ambiguity_errors() {
        let absent = fixture(json!({}), json!({}), json!({"lane": {}}));
        let before = absent.clone();
        let projection = absent
            .runtime_control_plane_authority("missing", "lane")
            .unwrap();
        assert!(projection.attestation.is_none());
        assert!(projection.current_owner.is_none());
        assert!(projection.lifecycle.is_some());
        assert_eq!(absent, before);

        let ambiguous = fixture(
            json!({"a": owner("a", "a", "route"), "b": owner("b", "b", "route")}),
            json!({}),
            json!({}),
        );
        let before = ambiguous.clone();
        let expected = ambiguous
            .runtime_owner_registry
            .attestation_for_session("route")
            .unwrap_err();
        assert_eq!(
            expected,
            "runtime_owner_session_ambiguous: session 'route' matches multiple profile owners"
        );
        assert_eq!(
            ambiguous
                .runtime_control_plane_authority("route", "a")
                .unwrap_err(),
            expected
        );
        assert_eq!(ambiguous, before);
    }

    #[test]
    fn control_plane_uses_first_exact_id_and_generation_not_browser_or_profile_match() {
        let mut wrong_generation = owner("a", "a", "unrelated-a");
        wrong_generation["ownerGeneration"] = json!(8);
        let mut wrong_id = owner("b", "b", "unrelated-b");
        wrong_id["ownerId"] = json!("other-owner");
        let state = fixture(
            json!({
                "a": wrong_generation, "b": wrong_id,
                "c": owner("c", "unrelated-browser", "unrelated-c"),
                "d": owner("d", "also-unrelated", "unrelated-d"),
                "z": owner("z", "session:alias", "route")
            }),
            json!({}),
            json!({"supplied-browser": {"logicalBrowserId": "mismatched"}}),
        );
        let before = state.clone();
        for (session, effect_capable) in [("route", true), ("alias", false)] {
            let projection = state
                .runtime_control_plane_authority(session, "supplied-browser")
                .unwrap();
            assert_eq!(
                projection.attestation,
                state
                    .runtime_owner_registry
                    .attestation_for_session(session)
                    .unwrap()
            );
            assert_eq!(
                projection.attestation.unwrap().effect_capable,
                effect_capable
            );
            assert_eq!(
                projection.current_owner.unwrap().profile_identity_digest,
                "c"
            );
            assert_eq!(
                projection.lifecycle.unwrap().logical_browser_id,
                "mismatched"
            );
        }
        assert_eq!(state, before);
    }

    #[test]
    fn control_plane_delegates_terminal_history_selection() {
        let state = fixture(
            json!({"old": owner("old", "old-browser", "route"), "new": owner("new", "new-browser", "route")}),
            json!({}),
            json!({
                "old-browser": {"logicalBrowserId": "old-browser", "profileIdentityDigest": "old", "ownerGeneration": 7, "lifecycleState": "terminal", "cleanupObligationState": "satisfied"}
            }),
        );
        let before = state.clone();
        let projection = state
            .runtime_control_plane_authority("route", "absent")
            .unwrap();
        assert_eq!(
            projection.attestation,
            state
                .runtime_owner_registry
                .attestation_for_session("route")
                .unwrap()
        );
        assert_eq!(
            projection.attestation.unwrap().logical_browser_id,
            "new-browser"
        );
        assert!(projection.lifecycle.is_none());
        assert_eq!(state, before);
    }

    #[test]
    fn resource_rows_preserve_map_keys_order_and_exact_browser_tab_joins() {
        let mut state = fixture(
            json!({"owner-only": owner("owner-only", "owner-only", "route")}),
            json!({}),
            json!({
                "z": {"logicalBrowserId": "a", "lifecycleState": "terminal"},
                "a": {"logicalBrowserId": "z"}, "m": {"logicalBrowserId": "m"}
            }),
        );
        state.browsers.insert(
            "a".into(),
            BrowserProcess {
                pid: Some(42),
                ..BrowserProcess::default()
            },
        );
        state.browsers.insert("m".into(), BrowserProcess::default());
        for (tab, browser) in [("1", "a"), ("2", "a"), ("3", "z"), ("4", "unrelated")] {
            state.tabs.insert(
                tab.into(),
                BrowserTab {
                    browser_id: browser.into(),
                    ..BrowserTab::default()
                },
            );
        }
        let before = state.clone();
        let rows = state.runtime_resource_lanes();
        assert_eq!(
            rows.iter()
                .map(|row| (row.browser_id, row.browser_pid, row.tab_count))
                .collect::<Vec<_>>(),
            vec![("a", Some(42), 2), ("m", None, 0), ("z", None, 1)]
        );
        assert_eq!(rows[0].lifecycle.logical_browser_id, "z");
        assert_eq!(rows[2].lifecycle.logical_browser_id, "a");
        assert_eq!(
            serde_json::to_value(rows.iter().map(|row| row.lifecycle).collect::<Vec<_>>()).unwrap(),
            serde_json::to_value(
                state
                    .runtime_owner_registry
                    .lifecycle_records()
                    .values()
                    .collect::<Vec<_>>()
            )
            .unwrap()
        );
        assert_eq!(state, before);
    }
}
