use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::RemoteViewDesktopCandidate;

pub const REMOTE_VIEW_FOUNDATION_SCHEMA_VERSION: u32 = 1;
pub const REMOTE_VIEW_FOUNDATION_CONTRACT_VERSION: u32 = 1;
pub const REMOTE_VIEW_J1_SOURCE_CHECKPOINT: &str = "f674518e34fea346002c72c4adc3966b628d0b78";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewResourceKind {
    Installation,
    Desktop,
    Generation,
    Pool,
    ApplicationPlacement,
    Operation,
    ViewerSession,
    EventCursor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RemoteViewFoundationCommand {
    #[serde(rename = "foundation.inspect")]
    Inspect,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewResourceState {
    Requested,
    Provisioning,
    Ready,
    Unhealthy,
    Draining,
    Removed,
    Failed,
    AwaitingIntervention,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewEffectBoundary {
    CommitDesiredStateBeforeEffects,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewEffectEvidence {
    EffectJournal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteViewFoundationObservation {
    pub schema_version: u32,
    pub command: RemoteViewFoundationCommand,
    pub contract_version: u32,
    pub read_only: bool,
    pub resource_kinds: Vec<RemoteViewResourceKind>,
    pub resource_states: Vec<RemoteViewResourceState>,
    pub effect_boundary: RemoteViewEffectBoundary,
    pub effect_evidence: RemoteViewEffectEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RemoteViewFixedDesktop {
    pub desktop_id: String,
    pub friendly_route_label: String,
    pub generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewDesktopResources {
    pub display: u32,
    pub vnc_port: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewDesktopRecord {
    pub desktop_id: String,
    pub friendly_route: String,
    pub generation: u64,
    pub state: RemoteViewResourceState,
    pub resources: RemoteViewDesktopResources,
    pub allocated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewLifecycleObservation {
    pub schema_version: u32,
    pub operation: String,
    pub desktop: RemoteViewDesktopRecord,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewJ1ConsumerFixture {
    pub schema_version: u32,
    pub source_checkpoint: String,
    pub lifecycle_observations: Vec<RemoteViewLifecycleObservation>,
    pub operation_statuses: Vec<RemoteViewOperationRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewOperationState {
    Requested,
    Running,
    Succeeded,
    Cancelled,
    TimedOut,
    Failed,
    AwaitingIntervention,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteViewOperationRecord {
    pub schema_version: u32,
    pub id: String,
    pub idempotency_key: String,
    pub kind: String,
    pub payload: Value,
    pub payload_hash: String,
    pub state: RemoteViewOperationState,
    pub attempt: u32,
    pub created_unix_ms: u64,
    pub updated_unix_ms: u64,
    pub timeout_unix_ms: u64,
    pub journal_operation_ids: Vec<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteViewDesktopReference {
    pub desktop_id: String,
    pub expected_generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentBrowserDesktopAssociation {
    pub browser_id: String,
    pub profile_id: String,
    pub desktop: RemoteViewFixedDesktop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RemoteViewConsumerError {
    IncompatibleFoundation,
    InvalidIdentity,
    DuplicateBrowser,
    DuplicateProfile,
    DuplicateDesktop,
    DuplicateRoute,
    InvalidLifecycleObservation,
    DesktopNotAllocated,
    InvalidOperationRecord,
}

pub fn validate_remote_view_foundation(
    observation: &RemoteViewFoundationObservation,
) -> Result<(), RemoteViewConsumerError> {
    let required = BTreeSet::from([
        RemoteViewResourceKind::Installation,
        RemoteViewResourceKind::Desktop,
        RemoteViewResourceKind::Generation,
        RemoteViewResourceKind::Pool,
        RemoteViewResourceKind::ApplicationPlacement,
        RemoteViewResourceKind::Operation,
        RemoteViewResourceKind::ViewerSession,
        RemoteViewResourceKind::EventCursor,
    ]);
    let required_states = BTreeSet::from([
        RemoteViewResourceState::Requested,
        RemoteViewResourceState::Provisioning,
        RemoteViewResourceState::Ready,
        RemoteViewResourceState::Unhealthy,
        RemoteViewResourceState::Draining,
        RemoteViewResourceState::Removed,
        RemoteViewResourceState::Failed,
        RemoteViewResourceState::AwaitingIntervention,
    ]);
    let observed: BTreeSet<_> = observation.resource_kinds.iter().copied().collect();
    let observed_states: BTreeSet<_> = observation.resource_states.iter().copied().collect();
    if observation.schema_version != REMOTE_VIEW_FOUNDATION_SCHEMA_VERSION
        || observation.command != RemoteViewFoundationCommand::Inspect
        || observation.contract_version != REMOTE_VIEW_FOUNDATION_CONTRACT_VERSION
        || !observation.read_only
        || observation.resource_kinds.len() != required.len()
        || observed != required
        || observation.resource_states.len() != required_states.len()
        || observed_states != required_states
        || observation.effect_boundary != RemoteViewEffectBoundary::CommitDesiredStateBeforeEffects
        || observation.effect_evidence != RemoteViewEffectEvidence::EffectJournal
    {
        return Err(RemoteViewConsumerError::IncompatibleFoundation);
    }
    Ok(())
}

pub fn validate_fixed_desktop_associations(
    associations: &[AgentBrowserDesktopAssociation],
) -> Result<(), RemoteViewConsumerError> {
    let mut browsers = BTreeSet::new();
    let mut profiles = BTreeSet::new();
    let mut desktops = BTreeSet::new();
    let mut routes = BTreeSet::new();
    for association in associations {
        let desktop = &association.desktop;
        if !valid_token(&association.browser_id)
            || !valid_token(&association.profile_id)
            || !valid_uuid(&desktop.desktop_id)
            || !valid_token(&desktop.friendly_route_label)
            || desktop.generation == 0
        {
            return Err(RemoteViewConsumerError::InvalidIdentity);
        }
        if !browsers.insert(&association.browser_id) {
            return Err(RemoteViewConsumerError::DuplicateBrowser);
        }
        if !profiles.insert(&association.profile_id) {
            return Err(RemoteViewConsumerError::DuplicateProfile);
        }
        if !desktops.insert(&desktop.desktop_id) {
            return Err(RemoteViewConsumerError::DuplicateDesktop);
        }
        if !routes.insert(&desktop.friendly_route_label) {
            return Err(RemoteViewConsumerError::DuplicateRoute);
        }
    }
    Ok(())
}

pub fn allocated_desktop_candidate(
    observation: &RemoteViewLifecycleObservation,
) -> Result<RemoteViewDesktopCandidate, RemoteViewConsumerError> {
    let desktop = &observation.desktop;
    if observation.schema_version != REMOTE_VIEW_FOUNDATION_SCHEMA_VERSION
        || !matches!(
            observation.operation.as_str(),
            "desktop.allocate" | "desktop.inspect" | "desktop.reconcile"
        )
        || !valid_uuid(&desktop.desktop_id)
        || !valid_token(&desktop.friendly_route)
        || desktop.generation == 0
        || desktop.resources.display == 0
        || desktop.resources.vnc_port == 0
    {
        return Err(RemoteViewConsumerError::InvalidLifecycleObservation);
    }
    if desktop.state != RemoteViewResourceState::Ready || !desktop.allocated {
        return Err(RemoteViewConsumerError::DesktopNotAllocated);
    }
    Ok(RemoteViewDesktopCandidate {
        desktop: RemoteViewFixedDesktop {
            desktop_id: desktop.desktop_id.clone(),
            friendly_route_label: desktop.friendly_route.clone(),
            generation: desktop.generation,
        },
        ready: true,
    })
}

pub fn exact_release_reference(
    association: &AgentBrowserDesktopAssociation,
) -> RemoteViewDesktopReference {
    RemoteViewDesktopReference {
        desktop_id: association.desktop.desktop_id.clone(),
        expected_generation: association.desktop.generation,
    }
}

pub fn validate_remote_view_operation(
    operation: &RemoteViewOperationRecord,
) -> Result<(), RemoteViewConsumerError> {
    let expected_hash = serde_json::to_vec(&(&operation.kind, &operation.payload))
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|_| RemoteViewConsumerError::InvalidOperationRecord)?;
    let journal_ids_valid = operation
        .journal_operation_ids
        .iter()
        .all(|id| valid_token(id));
    let state_attempt_valid = match operation.state {
        RemoteViewOperationState::Requested => operation.attempt == 0,
        RemoteViewOperationState::Running
        | RemoteViewOperationState::Succeeded
        | RemoteViewOperationState::AwaitingIntervention => operation.attempt > 0,
        RemoteViewOperationState::Cancelled
        | RemoteViewOperationState::TimedOut
        | RemoteViewOperationState::Failed => true,
    };
    if operation.schema_version != REMOTE_VIEW_FOUNDATION_SCHEMA_VERSION
        || !valid_token(&operation.id)
        || !valid_token(&operation.idempotency_key)
        || !matches!(operation.kind.as_str(), "desktop.create" | "desktop.delete")
        || operation.payload_hash != expected_hash
        || operation.updated_unix_ms < operation.created_unix_ms
        || operation.timeout_unix_ms <= operation.created_unix_ms
        || !journal_ids_valid
        || !state_attempt_valid
    {
        return Err(RemoteViewConsumerError::InvalidOperationRecord);
    }
    Ok(())
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn foundation() -> RemoteViewFoundationObservation {
        serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-foundation.v1.fixture.json"
        ))
        .unwrap()
    }

    fn j1_fixture() -> RemoteViewJ1ConsumerFixture {
        serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-j1-consumer.v1.fixture.json"
        ))
        .unwrap()
    }

    fn association(
        browser: &str,
        profile: &str,
        desktop: &str,
        route: &str,
    ) -> AgentBrowserDesktopAssociation {
        AgentBrowserDesktopAssociation {
            browser_id: browser.into(),
            profile_id: profile.into(),
            desktop: RemoteViewFixedDesktop {
                desktop_id: desktop.into(),
                friendly_route_label: route.into(),
                generation: 1,
            },
        }
    }

    #[test]
    fn accepts_current_foundation_and_two_distinct_browser_associations() {
        validate_remote_view_foundation(&foundation()).unwrap();
        let associations = [
            association(
                "browser-alice",
                "profile-alice",
                "11111111-1111-1111-1111-111111111111",
                "desktop-alice",
            ),
            association(
                "browser-bob",
                "profile-bob",
                "22222222-2222-2222-2222-222222222222",
                "desktop-bob",
            ),
        ];
        validate_fixed_desktop_associations(&associations).unwrap();
    }

    #[test]
    fn rejects_contract_drift_and_duplicate_desktop_ownership() {
        let mut incompatible = foundation();
        incompatible.contract_version = 2;
        assert_eq!(
            validate_remote_view_foundation(&incompatible),
            Err(RemoteViewConsumerError::IncompatibleFoundation)
        );

        let associations = [
            association(
                "browser-alice",
                "profile-alice",
                "11111111-1111-1111-1111-111111111111",
                "desktop-alice",
            ),
            association(
                "browser-bob",
                "profile-bob",
                "11111111-1111-1111-1111-111111111111",
                "desktop-bob",
            ),
        ];
        assert_eq!(
            validate_fixed_desktop_associations(&associations),
            Err(RemoteViewConsumerError::DuplicateDesktop)
        );
    }

    #[test]
    fn rejects_unversioned_foundation_shape_drift() {
        let changed =
            include_str!("../../../docs/dev/contracts/remote-view-foundation.v1.fixture.json")
                .replace("\n}", ",\n  \"unpublished_field\": true\n}");
        assert!(serde_json::from_str::<RemoteViewFoundationObservation>(&changed).is_err());
    }

    #[test]
    fn consumes_source_bound_j1_lifecycle_and_operation_contracts() {
        let fixture = j1_fixture();
        assert_eq!(
            fixture.schema_version,
            REMOTE_VIEW_FOUNDATION_SCHEMA_VERSION
        );
        assert_eq!(fixture.source_checkpoint, REMOTE_VIEW_J1_SOURCE_CHECKPOINT);

        let candidates = fixture
            .lifecycle_observations
            .iter()
            .map(allocated_desktop_candidate)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let associations = [
            AgentBrowserDesktopAssociation {
                browser_id: "browser-alice".into(),
                profile_id: "profile-alice".into(),
                desktop: candidates[0].desktop.clone(),
            },
            AgentBrowserDesktopAssociation {
                browser_id: "browser-bob".into(),
                profile_id: "profile-bob".into(),
                desktop: candidates[1].desktop.clone(),
            },
        ];
        validate_fixed_desktop_associations(&associations).unwrap();
        assert_eq!(
            exact_release_reference(&associations[1]),
            RemoteViewDesktopReference {
                desktop_id: "22222222-2222-2222-2222-222222222222".into(),
                expected_generation: 3,
            }
        );
        for operation in &fixture.operation_statuses {
            validate_remote_view_operation(operation).unwrap();
        }
    }

    #[test]
    fn rejects_unallocated_desktops_and_invalid_operation_evidence() {
        let fixture = j1_fixture();
        let mut unavailable = fixture.lifecycle_observations[0].clone();
        unavailable.desktop.allocated = false;
        assert_eq!(
            allocated_desktop_candidate(&unavailable),
            Err(RemoteViewConsumerError::DesktopNotAllocated)
        );

        let mut invalid_operation = fixture.operation_statuses[0].clone();
        invalid_operation.payload["friendlyRoute"] = Value::String("different-route".into());
        assert_eq!(
            validate_remote_view_operation(&invalid_operation),
            Err(RemoteViewConsumerError::InvalidOperationRecord)
        );
    }

    #[test]
    fn rejects_unversioned_j1_shape_drift() {
        let changed =
            include_str!("../../../docs/dev/contracts/remote-view-j1-consumer.v1.fixture.json")
                .replace(
                    "\n  \"operationStatuses\"",
                    "\n  \"unpublishedField\": true,\n  \"operationStatuses\"",
                );
        assert!(serde_json::from_str::<RemoteViewJ1ConsumerFixture>(&changed).is_err());
    }
}
