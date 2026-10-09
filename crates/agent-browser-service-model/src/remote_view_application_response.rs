//! Exact revision 3 assignment joins and private launch inputs.
use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{RemoteViewAssignmentRecord, RemoteViewAssignmentState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteViewApplicationResponseError {
    InvalidShape,
    InvalidTarget,
    StaleTarget,
    LiveResourceRequired,
    InvalidEnvironment,
}

/// Lifecycle and viewing identities are independent, authoritative joins.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationTarget {
    pub assignment_id: String,
    pub registration_id: String,
    pub desktop_id: String,
    pub lifecycle_generation: u64,
    pub viewing_desktop_id: String,
    pub viewing_generation: u64,
}

impl RemoteViewApplicationTarget {
    pub fn validate_assignment(
        &self,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        if assignment.state != RemoteViewAssignmentState::Active
            || self.lifecycle_generation == 0
            || self.viewing_generation == 0
            || !uuid(&self.desktop_id)
            || !uuid(&self.viewing_desktop_id)
            || self.assignment_id.is_empty()
            || self.registration_id.is_empty()
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        if self.assignment_id != assignment.assignment_id
            || self.registration_id != assignment.registration_id
            || self.desktop_id != assignment.desktop_id
            || self.lifecycle_generation != assignment.generation
        {
            return Err(RemoteViewApplicationResponseError::StaleTarget);
        }
        Ok(())
    }
}

/// Provider viewing route identities are opaque hexadecimal tokens, not UUIDs.
pub(crate) fn viewing_route_id(value: &str) -> bool {
    (32..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
}

pub(crate) fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewApplicationReadinessScope {
    ProviderRecord,
    LiveResource,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewAssignmentObservation {
    pub schema_version: u32,
    pub target: RemoteViewApplicationTarget,
    pub readiness_scope: RemoteViewApplicationReadinessScope,
}

/// Native owner evidence for returning unused capacity. Old providers that do
/// not support this observation cannot qualify an automatic release.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewIdleAssignmentObservation {
    pub schema_version: u32,
    pub target: RemoteViewApplicationTarget,
    pub readiness_scope: RemoteViewApplicationReadinessScope,
    pub retirement: crate::RemoteViewDesktopViewingRetirement,
    pub idle: bool,
    pub viewer_active: bool,
}

impl RemoteViewIdleAssignmentObservation {
    pub fn release_target(
        &self,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<crate::RemoteViewApplicationReleaseTarget, RemoteViewApplicationResponseError> {
        RemoteViewAssignmentObservation {
            schema_version: self.schema_version,
            target: self.target.clone(),
            readiness_scope: self.readiness_scope,
        }
        .validate_live_assignment(assignment)?;
        if self.retirement.schema_version != 1
            || self.retirement.desktop_id != assignment.desktop_id
            || self.retirement.generation != assignment.generation
            || (self.idle && (!self.retirement.sessions.is_empty() || self.viewer_active))
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        let target = crate::RemoteViewApplicationReleaseTarget {
            assignment: assignment.clone(),
            route_ids: self.retirement.routes.clone(),
            viewer_session_ids: self.retirement.sessions.clone(),
        };
        target.validate()?;
        Ok(target)
    }
}

impl RemoteViewAssignmentObservation {
    /// Validate an observation against the retained acquisition, without
    /// equating viewing generation to lifecycle generation.
    pub fn validate_live_assignment(
        &self,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        if self.schema_version != 1 {
            return Err(RemoteViewApplicationResponseError::InvalidShape);
        }
        self.target.validate_assignment(assignment)?;
        if self.readiness_scope != RemoteViewApplicationReadinessScope::LiveResource {
            return Err(RemoteViewApplicationResponseError::LiveResourceRequired);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LaunchEnvironmentResponse {
    schema_version: u32,
    target: RemoteViewApplicationTarget,
    environment: BTreeMap<String, String>,
    readiness_scope: RemoteViewApplicationReadinessScope,
}

/// Private process-launch inputs. Deliberately implements neither Serialize
/// nor Clone. Consume once at a launch boundary after a fresh observation;
/// storing this value cannot establish continuing resource freshness.
///
/// ```compile_fail
/// use agent_browser_service_model::RemoteViewPrivateLaunchEnvironment;
/// fn publish(environment: RemoteViewPrivateLaunchEnvironment) {
///     serde_json::to_value(environment).unwrap();
/// }
/// ```
pub struct RemoteViewPrivateLaunchEnvironment {
    target: RemoteViewApplicationTarget,
    environment: BTreeMap<String, String>,
}

impl fmt::Debug for RemoteViewPrivateLaunchEnvironment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemoteViewPrivateLaunchEnvironment")
            .field("target", &self.target)
            .field("environment", &"redacted")
            .finish()
    }
}

impl RemoteViewPrivateLaunchEnvironment {
    /// Parse without returning provider payloads or parser messages in errors.
    pub fn from_response(
        response: Value,
        expected: &RemoteViewAssignmentObservation,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<Self, RemoteViewApplicationResponseError> {
        expected.validate_live_assignment(assignment)?;
        let response: LaunchEnvironmentResponse = serde_json::from_value(response)
            .map_err(|_| RemoteViewApplicationResponseError::InvalidShape)?;
        if response.schema_version != 1 {
            return Err(RemoteViewApplicationResponseError::InvalidShape);
        }
        if response.readiness_scope != RemoteViewApplicationReadinessScope::LiveResource {
            return Err(RemoteViewApplicationResponseError::LiveResourceRequired);
        }
        if response.target != expected.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget);
        }
        let environment = response.environment;
        if environment
            .get("DISPLAY")
            .is_none_or(|value| value.is_empty())
            || environment
                .get("XAUTHORITY")
                .is_none_or(|value| !value.starts_with('/'))
            || environment.get("REMOTE_VIEW_SLOT_GENERATION")
                != Some(&expected.target.viewing_generation.to_string())
            || environment.iter().any(|(key, value)| {
                key.is_empty() || key.contains(['=', '\0']) || value.contains('\0')
            })
        {
            return Err(RemoteViewApplicationResponseError::InvalidEnvironment);
        }
        Ok(Self {
            target: response.target,
            environment,
        })
    }

    /// Recheck the joined observation at consumption. The transport adapter
    /// supplies a newly read observation immediately before launch.
    pub fn into_environment(
        self,
        current: &RemoteViewAssignmentObservation,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<BTreeMap<String, String>, RemoteViewApplicationResponseError> {
        self.validate_observation(current, assignment)?;
        Ok(self.environment)
    }

    pub(crate) fn validate_observation(
        &self,
        current: &RemoteViewAssignmentObservation,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        current.validate_live_assignment(assignment)?;
        if self.target != current.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget);
        }
        Ok(())
    }
}
