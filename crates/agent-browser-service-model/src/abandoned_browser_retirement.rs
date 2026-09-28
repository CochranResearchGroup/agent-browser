//! Frozen durable contracts for abandoned-browser retirement.
//! Aggregate transitions, observation, and process effects belong to CLI adapters.

use crate::{BrowserHealth, RecordedProcessIdentity};
use serde::{Deserialize, Serialize};

pub const ABANDONED_BROWSER_RETIREMENT_PLAN_SCHEMA_V1: &str =
    "agent-browser.abandoned-browser-retirement-plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbandonedBrowserRetirementPlan {
    pub schema_version: String,
    pub plan_id: String,
    pub state_revision: u64,
    pub browser_id: String,
    pub browser_record_digest: String,
    pub root: RecordedProcessIdentity,
    pub descendants: Vec<RecordedProcessIdentity>,
    pub process_group_id: u32,
    pub profile_path: String,
    pub profile_id: Option<String>,
    pub profile_identity_digest: String,
    pub owner_generation: u64,
    pub owner_digest: String,
    pub package_launch_identity_digest: String,
    pub activity_digest: String,
    pub policy: ResourceRetirementPolicy,
    pub created_at: String,
    pub expires_at: String,
    pub expected_terminal: RetirementTerminalProjection,
}

/// Exact public-state postcondition. Profile records and profile files survive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetirementTerminalProjection {
    pub browser_health: BrowserHealth,
    pub detached_session_ids: Vec<String>,
    pub closed_tab_ids: Vec<String>,
    pub released_display_allocation_ids: Vec<String>,
    pub released_route_ids: Vec<String>,
    pub released_viewer_lease_ids: Vec<String>,
    pub released_acquisition_lease_ids: Vec<String>,
    pub released_route_pool_entry_ids: Vec<String>,
    pub preserved_profile_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbandonedBrowserRetirementTransaction {
    pub plan: AbandonedBrowserRetirementPlan,
    pub reserved_revision: u64,
    pub reserved_browser_digest: String,
    pub receipt: Option<AbandonedBrowserRetirementReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbandonedBrowserRetirementReceipt {
    pub plan_id: String,
    pub browser_id: String,
    pub completed_at: String,
    pub terminal_revision: u64,
    pub effect_evidence: RetirementExitEvidence,
    pub terminal_projection: RetirementTerminalProjection,
}

/// Adapter-observed proof, tied to the exact reservation and process group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetirementExitEvidence {
    pub plan_id: String,
    pub reserved_revision: u64,
    pub process_group_id: u32,
    pub observed_at: String,
    pub root_exited: bool,
    pub descendants_exited: bool,
    pub process_group_empty: bool,
    pub profile_lock_released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetirementExitFailure {
    pub failed_conditions: Vec<String>,
    pub evidence: RetirementExitEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum RetirementRecourse {
    InvalidPlan,
    Expired,
    StateRevisionChanged,
    BrowserRecordChanged,
    RootChanged,
    DescendantsChanged,
    ProcessGroupChanged,
    ProfileChanged,
    OwnerChanged,
    ActivityChanged,
    Ineligible(String),
    ReservationMissing,
    RecoveryRequired,
    ExitUnproven(Box<RetirementExitFailure>),
    TerminalCompareAndSwapFailed,
    ObservationFailed(String),
}

impl std::fmt::Display for RetirementRecourse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "abandoned_browser_retirement:{self:?}")
    }
}

const DEFAULT_ABANDONED_LANE_INACTIVITY_SECONDS: u64 = 5 * 60;
const DEFAULT_PER_BROWSER_RSS_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const DEFAULT_PER_BROWSER_DESCENDANTS: usize = 64;
const DEFAULT_PER_BROWSER_TABS: usize = 128;
const DEFAULT_WORKSTATION_LANES: usize = 16;
const DEFAULT_WORKSTATION_PROCESSES: usize = 256;
const DEFAULT_WORKSTATION_RSS_BYTES: u64 = 64 * 1024 * 1024 * 1024;

/// Bounded, read-only thresholds used by the resource projection. A policy is
/// deliberately not effect authority: retirement still revalidates the exact
/// owner, process, and activity observation immediately before any shutdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ResourceRetirementPolicy {
    pub inactivity_minimum_seconds: u64,
    pub per_browser_max_rss_bytes: u64,
    pub per_browser_max_descendants: usize,
    pub per_browser_max_tabs: usize,
    pub workstation_max_lanes: usize,
    pub workstation_max_processes: usize,
    pub workstation_max_rss_bytes: u64,
}

impl Default for ResourceRetirementPolicy {
    fn default() -> Self {
        Self {
            inactivity_minimum_seconds: DEFAULT_ABANDONED_LANE_INACTIVITY_SECONDS,
            per_browser_max_rss_bytes: DEFAULT_PER_BROWSER_RSS_BYTES,
            per_browser_max_descendants: DEFAULT_PER_BROWSER_DESCENDANTS,
            per_browser_max_tabs: DEFAULT_PER_BROWSER_TABS,
            workstation_max_lanes: DEFAULT_WORKSTATION_LANES,
            workstation_max_processes: DEFAULT_WORKSTATION_PROCESSES,
            workstation_max_rss_bytes: DEFAULT_WORKSTATION_RSS_BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};

    // Frozen in the original CLI declaration order, including explicit nulls.
    const UNSIGNED_PLAN: &str = r#"{"schemaVersion":"agent-browser.abandoned-browser-retirement-plan.v1","planId":"","stateRevision":7,"browserId":"browser","browserRecordDigest":"browser-digest","root":{"pid":42,"startToken":"start"},"descendants":[{"pid":43,"startToken":"child","executablePath":"/browser","browserFamily":"chrome"}],"processGroupId":42,"profilePath":"/profile","profileId":null,"profileIdentityDigest":"identity","ownerGeneration":2,"ownerDigest":"owner","packageLaunchIdentityDigest":"launch","activityDigest":"activity","policy":{"inactivityMinimumSeconds":300,"perBrowserMaxRssBytes":4294967296,"perBrowserMaxDescendants":64,"perBrowserMaxTabs":128,"workstationMaxLanes":16,"workstationMaxProcesses":256,"workstationMaxRssBytes":68719476736},"createdAt":"2026-09-17T00:00:00Z","expiresAt":"2026-09-17T00:10:00Z","expectedTerminal":{"browserHealth":"process_exited","detachedSessionIds":["session"],"closedTabIds":["tab"],"releasedDisplayAllocationIds":[],"releasedRouteIds":[],"releasedViewerLeaseIds":[],"releasedAcquisitionLeaseIds":[],"releasedRoutePoolEntryIds":[],"preservedProfileDigest":"profile"}}"#;

    fn plan() -> AbandonedBrowserRetirementPlan {
        serde_json::from_str(UNSIGNED_PLAN).unwrap()
    }

    fn evidence() -> RetirementExitEvidence {
        RetirementExitEvidence {
            plan_id: "plan".into(),
            reserved_revision: 8,
            process_group_id: 42,
            observed_at: "2026-09-17T00:01:00Z".into(),
            root_exited: true,
            descendants_exited: true,
            process_group_empty: false,
            profile_lock_released: false,
        }
    }

    fn assert_strict_record<T>(record: &T)
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let mut value = serde_json::to_value(record).unwrap();
        assert_eq!(&serde_json::from_value::<T>(value.clone()).unwrap(), record);
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        assert!(serde_json::from_value::<T>(value).is_err());
    }

    #[test]
    fn abandoned_retirement_plan_preserves_wire_order_and_unsigned_seal() {
        let plan = plan();
        let wire = serde_json::to_string(&plan).unwrap();
        assert_eq!(wire, UNSIGNED_PLAN);
        assert_eq!(
            format!("{:x}", Sha256::digest(wire.as_bytes())),
            "e0098cc404c819c379a00bca438da6e3195d4a93a8cdc90811b37ad7166a9292"
        );
        assert_eq!(
            plan.schema_version,
            ABANDONED_BROWSER_RETIREMENT_PLAN_SCHEMA_V1
        );
        // Decoding remains inert: validation of seals and timestamps stays in CLI.
        assert!(plan.plan_id.is_empty());
        let mut invalid = serde_json::to_value(&plan).unwrap();
        invalid["schemaVersion"] = json!("unknown");
        invalid["createdAt"] = json!("not-a-time");
        assert!(serde_json::from_value::<AbandonedBrowserRetirementPlan>(invalid).is_ok());
    }

    #[test]
    fn abandoned_retirement_records_roundtrip_and_reject_unknown_fields() {
        let plan = plan();
        let receipt = AbandonedBrowserRetirementReceipt {
            plan_id: "plan".into(),
            browser_id: "browser".into(),
            completed_at: "2026-09-17T00:01:00Z".into(),
            terminal_revision: 9,
            effect_evidence: evidence(),
            terminal_projection: plan.expected_terminal.clone(),
        };
        let mut transaction = AbandonedBrowserRetirementTransaction {
            plan: plan.clone(),
            reserved_revision: 8,
            reserved_browser_digest: "reserved".into(),
            receipt: None,
        };
        assert_eq!(
            serde_json::to_value(&transaction).unwrap()["receipt"],
            Value::Null
        );
        assert_strict_record(&transaction);
        transaction.receipt = Some(receipt.clone());
        assert_strict_record(&transaction);
        assert_strict_record(&plan);
        assert_strict_record(&plan.expected_terminal);
        assert_strict_record(&receipt);
        assert_strict_record(&evidence());
        assert_strict_record(&RetirementExitFailure {
            failed_conditions: vec!["process_group_empty".into(), "profile_lock_released".into()],
            evidence: evidence(),
        });
    }

    #[test]
    fn abandoned_retirement_policy_defaults_and_permissive_decode_are_frozen() {
        let defaults = ResourceRetirementPolicy::default();
        assert_eq!(
            serde_json::to_string(&defaults).unwrap(),
            r#"{"inactivityMinimumSeconds":300,"perBrowserMaxRssBytes":4294967296,"perBrowserMaxDescendants":64,"perBrowserMaxTabs":128,"workstationMaxLanes":16,"workstationMaxProcesses":256,"workstationMaxRssBytes":68719476736}"#
        );
        assert_eq!(
            serde_json::from_str::<ResourceRetirementPolicy>("{}").unwrap(),
            defaults
        );
        assert_eq!(
            serde_json::from_value::<ResourceRetirementPolicy>(json!({"unknown": 7})).unwrap(),
            defaults
        );
        let partial: ResourceRetirementPolicy =
            serde_json::from_value(json!({"perBrowserMaxTabs": 3})).unwrap();
        assert_eq!(
            partial,
            ResourceRetirementPolicy {
                per_browser_max_tabs: 3,
                ..defaults
            }
        );
    }

    #[test]
    fn abandoned_retirement_recourse_wire_and_display_are_frozen() {
        let cases = [
            (RetirementRecourse::InvalidPlan, "invalid_plan"),
            (RetirementRecourse::Expired, "expired"),
            (
                RetirementRecourse::StateRevisionChanged,
                "state_revision_changed",
            ),
            (
                RetirementRecourse::BrowserRecordChanged,
                "browser_record_changed",
            ),
            (RetirementRecourse::RootChanged, "root_changed"),
            (
                RetirementRecourse::DescendantsChanged,
                "descendants_changed",
            ),
            (
                RetirementRecourse::ProcessGroupChanged,
                "process_group_changed",
            ),
            (RetirementRecourse::ProfileChanged, "profile_changed"),
            (RetirementRecourse::OwnerChanged, "owner_changed"),
            (RetirementRecourse::ActivityChanged, "activity_changed"),
            (
                RetirementRecourse::ReservationMissing,
                "reservation_missing",
            ),
            (RetirementRecourse::RecoveryRequired, "recovery_required"),
            (
                RetirementRecourse::TerminalCompareAndSwapFailed,
                "terminal_compare_and_swap_failed",
            ),
        ];
        for (recourse, code) in cases {
            let wire = json!({"code": code});
            assert_eq!(serde_json::to_value(&recourse).unwrap(), wire);
            assert_eq!(
                serde_json::from_value::<RetirementRecourse>(wire).unwrap(),
                recourse
            );
            assert_eq!(
                recourse.to_string(),
                format!("abandoned_browser_retirement:{recourse:?}")
            );
        }
        for (recourse, code) in [
            (
                RetirementRecourse::Ineligible("reason".into()),
                "ineligible",
            ),
            (
                RetirementRecourse::ObservationFailed("reason".into()),
                "observation_failed",
            ),
        ] {
            let wire = json!({"code": code, "detail": "reason"});
            assert_eq!(serde_json::to_value(&recourse).unwrap(), wire);
            assert_eq!(
                serde_json::from_value::<RetirementRecourse>(wire).unwrap(),
                recourse
            );
        }
        assert_eq!(
            RetirementRecourse::Ineligible("reason".into()).to_string(),
            "abandoned_browser_retirement:Ineligible(\"reason\")"
        );
        let failure = RetirementExitFailure {
            failed_conditions: vec!["process_group_empty".into(), "profile_lock_released".into()],
            evidence: evidence(),
        };
        let recourse = RetirementRecourse::ExitUnproven(Box::new(failure.clone()));
        let wire = json!({"code": "exit_unproven", "detail": failure});
        assert_eq!(serde_json::to_value(&recourse).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<RetirementRecourse>(wire).unwrap(),
            recourse
        );
    }
}
