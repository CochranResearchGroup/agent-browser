use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const REMOTE_VIEW_FOUNDATION_SCHEMA_VERSION: u32 = 1;
pub const REMOTE_VIEW_FOUNDATION_CONTRACT_VERSION: u32 = 1;

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
}
