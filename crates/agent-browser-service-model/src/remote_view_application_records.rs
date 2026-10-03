//! Revision 3 public inventory, window, event and viewing response records.
//! These models contain no transport credentials or provider-private state.
use std::collections::{BTreeMap, BTreeSet};

use crate::remote_view_application_response::{uuid, viewing_route_id};
use serde::{Deserialize, Serialize};

use crate::{
    RemoteViewApplicationReadinessScope, RemoteViewApplicationRegistration,
    RemoteViewApplicationResponseError, RemoteViewApplicationTarget,
    RemoteViewApplicationViewCapability, RemoteViewAssignmentRecord, RemoteViewAssignmentState,
    RemoteViewOperationState, RemoteViewResourceState,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteViewApplicationPoolPolicy {
    pub minimum_reserved: u32,
    pub maximum_reserved: u32,
    pub idle_retention_seconds: u32,
    pub allow_scale_in: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteViewApplicationPolicy {
    pub display_name: String,
    pub enabled: bool,
    pub pools: BTreeMap<String, RemoteViewApplicationPoolPolicy>,
    pub root_visibility: String,
    pub capabilities: Vec<String>,
    pub application_mode: String,
    #[serde(default)]
    pub allowed_application_profiles: Vec<String>,
    #[serde(default)]
    pub embed_origins: Vec<String>,
    pub max_grant_seconds: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationPool {
    pub pool_id: String,
    pub registration_id: String,
    pub name: String,
    pub capacity: u32,
    pub desktop_members: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationDesktopLifecycle {
    pub generation: u64,
    pub state: RemoteViewResourceState,
    pub allocated: bool,
    pub readiness_scope: RemoteViewApplicationReadinessScope,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationDesktopViewing {
    pub lifecycle_generation: u64,
    pub desktop_id: String,
    pub generation: u64,
    pub public_route: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationDesktop {
    pub desktop_id: String,
    pub lifecycle: Option<RemoteViewApplicationDesktopLifecycle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewing: Option<RemoteViewApplicationDesktopViewing>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationInventory {
    pub schema_version: u32,
    pub registration: RemoteViewApplicationRegistration,
    pub policy: RemoteViewApplicationPolicy,
    pub pools: Vec<RemoteViewApplicationPool>,
    pub assignments: Vec<RemoteViewAssignmentRecord>,
    pub desktops: Vec<RemoteViewApplicationDesktop>,
}

impl RemoteViewApplicationInventory {
    /// This validates resource membership and external lifecycle mode. Inventory
    /// availability alone does not establish live launch or viewing readiness.
    pub fn validate_external(
        &self,
        application: &str,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        let registration = &self.registration.registration_id;
        if self.schema_version != 1
            || registration.is_empty()
            || self.registration.consumer_key != application
            || !self.policy.enabled
            || self.policy.application_mode != "external"
            || !self.policy.allowed_application_profiles.is_empty()
            || self.pools.iter().any(|pool| {
                pool.registration_id != *registration
                    || pool.capacity == 0
                    || pool.pool_id.is_empty()
                    || !self.policy.pools.contains_key(&pool.name)
            })
            || self.assignments.iter().any(|assignment| {
                assignment.registration_id != *registration
                    || assignment.generation == 0
                    || assignment.assignment_id.is_empty()
                    || assignment.pool_id.is_empty()
                    || !uuid(&assignment.desktop_id)
                    || (assignment.state == RemoteViewAssignmentState::Active
                        && !self.pools.iter().any(|pool| {
                            pool.pool_id == assignment.pool_id
                                && pool.desktop_members.contains(&assignment.desktop_id)
                        }))
            })
            || !unique(self.pools.iter().map(|pool| pool.pool_id.as_str()))
            || !unique(self.pools.iter().map(|pool| pool.name.as_str()))
            || !unique(
                self.assignments
                    .iter()
                    .map(|assignment| assignment.assignment_id.as_str()),
            )
            || !unique(
                self.desktops
                    .iter()
                    .map(|desktop| desktop.desktop_id.as_str()),
            )
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        Ok(())
    }

    pub fn validate_acquisition(
        &self,
        pool_name: &str,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        let pool = self
            .pools
            .iter()
            .find(|pool| pool.name == pool_name)
            .ok_or(RemoteViewApplicationResponseError::InvalidTarget)?;
        if assignment.registration_id != self.registration.registration_id
            || assignment.pool_id != pool.pool_id
            || assignment.state != RemoteViewAssignmentState::Active
            || assignment.generation == 0
            || assignment.assignment_id.is_empty()
            || !uuid(&assignment.desktop_id)
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        // Acquisition may grow the configured pool, so the prior membership
        // snapshot need not contain the newly returned desktop.
        Ok(())
    }
}

pub(crate) fn unique<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .all(|value| !value.is_empty() && seen.insert(value))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationWindow {
    pub id: u32,
    pub title: String,
    pub class: String,
    pub pid: Option<u32>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub active: bool,
    pub maximized: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationWindows {
    pub schema_version: u32,
    pub operation: String,
    pub desktop_id: String,
    pub generation: u64,
    pub viewing_generation: u64,
    pub windows: Vec<RemoteViewApplicationWindow>,
}

impl RemoteViewApplicationWindows {
    pub fn validate_target(
        &self,
        target: &RemoteViewApplicationTarget,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        if self.schema_version != 1 || self.operation != "desktop.windows" {
            return Err(RemoteViewApplicationResponseError::InvalidShape);
        }
        validate_window_target(
            &self.desktop_id,
            self.generation,
            self.viewing_generation,
            target,
        )?;
        let mut ids = BTreeSet::new();
        if self
            .windows
            .iter()
            .any(|window| window.id == 0 || !ids.insert(window.id))
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        Ok(())
    }
}

fn validate_window_target(
    desktop: &str,
    lifecycle: u64,
    viewing: u64,
    target: &RemoteViewApplicationTarget,
) -> Result<(), RemoteViewApplicationResponseError> {
    if desktop != target.desktop_id
        || lifecycle != target.lifecycle_generation
        || viewing != target.viewing_generation
    {
        return Err(RemoteViewApplicationResponseError::StaleTarget);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationActivation {
    pub schema_version: u32,
    pub operation: String,
    pub desktop_id: String,
    pub generation: u64,
    pub viewing_generation: u64,
    pub window_id: u32,
}

impl RemoteViewApplicationActivation {
    pub fn validate_target(
        &self,
        target: &RemoteViewApplicationTarget,
        window_id: u32,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        if self.schema_version != 1
            || self.operation != "desktop.window.activate"
            || window_id == 0
            || self.window_id != window_id
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        validate_window_target(
            &self.desktop_id,
            self.generation,
            self.viewing_generation,
            target,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationEventBinding {
    pub registration_id: String,
    pub assignment_id: String,
    pub desktop_id: String,
    pub lifecycle_generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "observation", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteViewApplicationEventObservation {
    Assignment {
        target: RemoteViewApplicationTarget,
    },
    Windows {
        target: RemoteViewApplicationTarget,
        windows: RemoteViewApplicationWindows,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteViewApplicationEvent {
    pub schema_version: u32,
    pub cursor: u64,
    pub operation_id: String,
    pub state: RemoteViewOperationState,
    pub occurred_unix_ms: u64,
    #[serde(rename = "operationKind")]
    pub operation_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<RemoteViewApplicationEventObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<RemoteViewApplicationActivation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationEvents {
    pub schema_version: u32,
    pub binding: RemoteViewApplicationEventBinding,
    pub events: Vec<RemoteViewApplicationEvent>,
    pub observation_state: String,
}

impl RemoteViewApplicationEvents {
    pub fn validate_assignment(
        &self,
        assignment: &RemoteViewAssignmentRecord,
        after: Option<u64>,
        limit: usize,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        if self.schema_version != 1
            || self.binding.registration_id != assignment.registration_id
            || self.binding.assignment_id != assignment.assignment_id
            || self.binding.desktop_id != assignment.desktop_id
            || self.binding.lifecycle_generation != assignment.generation
            || self.events.len() > limit
            || !matches!(
                self.observation_state.as_str(),
                "refreshed" | "not_authorized" | "not_configured"
            )
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        let mut cursor = after.unwrap_or(0);
        for event in &self.events {
            if event.schema_version != 1
                || event.cursor <= cursor
                || event.operation_id.is_empty()
                || (event.observation.is_some() && event.operation_kind != "consumer.observation")
                || (event.receipt.is_some()
                    && (event.operation_kind != "consumer.window.activate"
                        || event.state != RemoteViewOperationState::Succeeded))
            {
                return Err(RemoteViewApplicationResponseError::InvalidShape);
            }
            if let Some(observation) = &event.observation {
                let target = match observation {
                    RemoteViewApplicationEventObservation::Assignment { target } => target,
                    RemoteViewApplicationEventObservation::Windows { target, windows } => {
                        windows.validate_target(target)?;
                        target
                    }
                };
                target.validate_assignment(assignment)?;
            }
            if let Some(receipt) = &event.receipt {
                if receipt.schema_version != 1
                    || receipt.operation != "desktop.window.activate"
                    || receipt.desktop_id != assignment.desktop_id
                    || receipt.generation != assignment.generation
                    || receipt.viewing_generation == 0
                    || receipt.window_id == 0
                {
                    return Err(RemoteViewApplicationResponseError::InvalidTarget);
                }
            }
            cursor = event.cursor;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationGrantRequest {
    pub idempotency_key: String,
    pub target: RemoteViewApplicationTarget,
    pub application: String,
    pub audience: String,
    pub capability: RemoteViewApplicationViewCapability,
    pub policy_revision: String,
    pub lifetime_seconds: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationGrant {
    pub schema_version: u32,
    pub route_id: String,
    pub request: RemoteViewApplicationGrantRequest,
    pub issued_at: u64,
    pub expires_at: u64,
    pub revoked: bool,
}

impl RemoteViewApplicationGrant {
    /// Match the provider's opaque route identity syntax independently of lifetime.
    /// This does not establish current grant or viewer readiness.
    pub fn has_valid_route_id(&self) -> bool {
        viewing_route_id(&self.route_id)
    }

    /// Issuance is route admission, not evidence that pixels or input are ready.
    pub fn validate_target(
        &self,
        target: &RemoteViewApplicationTarget,
        application: &str,
        audience: &str,
        now_ms: u64,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        if self.schema_version != 1
            || !self.has_valid_route_id()
            || self.request.target != *target
            || self.request.application != application
            || self.request.audience != audience
            || self.request.idempotency_key.is_empty()
            || self.request.policy_revision.is_empty()
            || self.request.lifetime_seconds == 0
            || self.expires_at.checked_sub(self.issued_at)
                != Some(u64::from(self.request.lifetime_seconds) * 1000)
            || self.issued_at > now_ms
            || self.expires_at <= now_ms
            || self.revoked
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationViewIssuance {
    pub schema_version: u32,
    pub path: String,
    pub presentation_state: String,
    pub grant: RemoteViewApplicationGrant,
    pub readiness_scope: RemoteViewApplicationReadinessScope,
}

impl RemoteViewApplicationViewIssuance {
    pub fn validate_target(
        &self,
        target: &RemoteViewApplicationTarget,
        application: &str,
        audience: &str,
        capability: RemoteViewApplicationViewCapability,
        lifetime_seconds: u32,
        now_ms: u64,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        self.grant
            .validate_target(target, application, audience, now_ms)?;
        let prefix = if audience == "remote_view" {
            "view"
        } else {
            "embed"
        };
        if self.schema_version != 1
            || self.presentation_state != "grant_issued"
            || self.path != format!("/{prefix}/{}", self.grant.route_id)
            || self.grant.request.capability != capability
            || self.grant.request.lifetime_seconds != lifetime_seconds
            || self.readiness_scope != RemoteViewApplicationReadinessScope::LiveResource
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationViewRevocation {
    pub schema_version: u32,
    pub route_id: String,
    pub state: String,
}
