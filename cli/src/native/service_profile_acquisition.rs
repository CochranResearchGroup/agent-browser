//! Typed profile-acquisition decision shared by planning and execution.
//!
//! Public access-plan JSON is a projection of this decision, not an interface
//! that execution callers must parse. This module starts the behavior-preserving
//! extraction by giving callers one typed representation of the selected
//! browser and daemon route. Policy semantics remain unchanged in this slice.

use serde_json::{json, Value};

use super::service_model::{BrowserProfile, LeaseState, ServiceState};
use super::service_principal::AuthenticatedServicePrincipal;
use super::{
    action_runtime, service_access, service_lease_authority, service_model, service_principal,
    service_resources, service_store, service_trace,
};

#[path = "service_profile_recovery.rs"]
mod recovery;

pub(crate) use recovery::*;

/// Project replacement authority and the exact collision-free daemon route
/// that can supersede one cleanup-satisfied terminal owner.
pub(crate) fn lifecycle_replacement_decision(
    selected_profile: Option<&BrowserProfile>,
    service_state: &ServiceState,
) -> Value {
    let Some(profile) = selected_profile else {
        return json!({
            "available": false,
            "replacementEligible": false,
            "reason": "no_selected_profile",
        });
    };
    let profile_path = profile
        .user_data_dir
        .as_deref()
        .map(std::path::PathBuf::from)
        .or_else(|| crate::runtime_profile::runtime_profile_user_data_dir(&profile.id).ok());
    let Some(profile_identity_digest) = profile_path
        .as_deref()
        .and_then(|path| crate::runtime_profile::canonical_profile_identity_digest(path).ok())
    else {
        return json!({
            "available": false,
            "profileId": profile.id,
            "replacementEligible": false,
            "reason": "profile_identity_unavailable",
        });
    };
    let owner = service_state
        .runtime_owner_registry
        .owners
        .get(&profile_identity_digest);
    let mut records = service_state
        .runtime_owner_registry
        .lifecycle_records
        .values()
        .filter(|record| record.profile_identity_digest == profile_identity_digest)
        .collect::<Vec<_>>();
    records.sort_by_key(|record| record.owner_generation);
    let owner_lifecycle = owner.and_then(|owner| {
        records.iter().copied().find(|record| {
            record.logical_browser_id == owner.browser_id
                && record.owner_generation == owner.owner_generation
        })
    });
    // Lifecycle history is observational. Only the record joined to the exact
    // current owner generation may participate in an operational decision.
    let lifecycle = owner_lifecycle;
    let terminal_cleanup_satisfied = lifecycle.is_some_and(|record| {
        record.lifecycle_state == crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Terminal
            && record.cleanup_obligation_state
                == crate::runtime_owner_transfer::CleanupObligationState::Satisfied
    });
    let terminal_process_exit_recorded = lifecycle.is_some_and(|record| {
        record.terminal_evidence.iter().any(|evidence| {
            evidence == "exact_process_exited"
                || evidence.starts_with("service_reconcile_process_group_absent:")
        })
    });
    let terminal_profile_lock_release_recorded = lifecycle.is_some_and(|record| {
        record.terminal_evidence.iter().any(|evidence| {
            evidence == "profile_lock_released"
                || evidence == "service_reconcile_profile_lock_absent"
                || evidence.starts_with("service_reconcile_profile_lock_stale_pid_absent:")
        })
    });
    let current_process_proven = owner.is_some_and(|owner| {
        service_state
            .browsers
            .get(&owner.browser_id)
            .is_some_and(|browser| browser.pid.is_some())
    });
    let terminal_process_absence_proven = terminal_process_exit_recorded
        && terminal_profile_lock_release_recorded
        && !current_process_proven;
    let active_profile_lease_session_ids = service_state
        .sessions
        .values()
        .filter(|session| {
            session.profile_id.as_deref() == Some(profile.id.as_str())
                && matches!(
                    session.lease,
                    LeaseState::Shared | LeaseState::Exclusive | LeaseState::HumanTakeover
                )
        })
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    let replacement_route = owner.zip(owner_lifecycle).and_then(|(owner, record)| {
        (terminal_cleanup_satisfied
            && terminal_process_absence_proven
            && active_profile_lease_session_ids.is_empty()
            && record.logical_browser_id == owner.browser_id
            && record.owner_generation == owner.owner_generation)
            .then(|| (owner.browser_id.clone(), owner.daemon_session_route.clone()))
    });
    let replacement_eligible = match (owner, lifecycle) {
        (None, None) => true,
        (Some(_), Some(_)) => replacement_route.is_some(),
        _ => false,
    };
    let reason = match lifecycle {
        None if owner.is_none() => "no_lifecycle_owner",
        None => "lifecycle_owner_record_missing",
        Some(_) if replacement_route.is_some() => "terminal_cleanup_satisfied",
        Some(_) if current_process_proven => "terminal_process_still_live",
        Some(_) if !active_profile_lease_session_ids.is_empty() => "terminal_profile_lease_active",
        Some(_) if terminal_cleanup_satisfied => "terminal_replacement_route_inconsistent",
        Some(record)
            if record.lifecycle_state
                == crate::runtime_owner_transfer::RuntimeLaneLifecycleState::Closing =>
        {
            "closing_lifecycle_requires_reconciliation"
        }
        Some(_) => "lifecycle_observation_not_replacement_eligible",
    };
    let required_action = match reason {
        "no_lifecycle_owner" => "launch_new_browser",
        "terminal_cleanup_satisfied" => "supersede_terminal_owner",
        "closing_lifecycle_requires_reconciliation" => "reconcile_lifecycle_owner",
        _ => "inspect_lifecycle_owner",
    };

    json!({
        "available": true,
        "profileId": profile.id,
        "registryRevision": service_state.runtime_owner_registry.revision,
        "ownerId": owner.map(|owner| owner.owner_id.clone()),
        "ownerState": owner.map(|owner| owner.state),
        "replacementBrowserId": replacement_route.as_ref().map(|(browser_id, _)| browser_id.clone()),
        "replacementSessionName": replacement_route.as_ref().map(|(_, session_name)| session_name.clone()),
        "logicalBrowserId": lifecycle.map(|record| record.logical_browser_id.clone()),
        "ownerGeneration": lifecycle.map(|record| record.owner_generation),
        "lifecycleState": lifecycle.map(|record| record.lifecycle_state),
        "cleanupObligationState": lifecycle.map(|record| record.cleanup_obligation_state),
        "processAbsenceProven": terminal_process_absence_proven,
        "activeProfileLeaseSessionIds": active_profile_lease_session_ids,
        "terminalEvidence": lifecycle.map(|record| record.terminal_evidence.clone()).unwrap_or_default(),
        "replacementEligible": replacement_eligible,
        "reason": reason,
        "requiredAction": required_action,
    })
}

/// Executable posture selected for one profile-acquisition request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileAcquisitionDisposition {
    ReuseExistingBrowser,
    LaunchNewBrowser,
    Blocked,
}

/// Joined acquisition result consumed by access-plan and action-runtime code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileAcquisitionDecision {
    disposition: ProfileAcquisitionDisposition,
    selected_profile_id: Option<String>,
    browser_id: Option<String>,
    session_name: Option<String>,
    acquisition_blocker: Option<String>,
    service_request_available: bool,
    profile_reuse_action: String,
    profile_reuse_reasons: Vec<String>,
    replacement_eligible: bool,
    replacement_session_name: Option<String>,
    runtime_owner_registry_revision: Option<u64>,
    owner_id: Option<String>,
    owner_generation: Option<u64>,
}

impl ProfileAcquisitionDecision {
    /// Build the typed decision from the canonical access-plan projection.
    ///
    /// This constructor is temporary extraction scaffolding. It validates the
    /// existing projection at the owner seam so downstream callers stop
    /// reconstructing acquisition truth independently. Later P157 slices move
    /// the underlying policy computation behind this same interface.
    pub(crate) fn from_access_plan(plan: &Value) -> Result<Self, String> {
        let decision = plan
            .get("decision")
            .and_then(Value::as_object)
            .ok_or_else(|| "profile_acquisition_decision_missing".to_string())?;
        let profile_reuse = decision
            .get("profileReuse")
            .and_then(Value::as_object)
            .ok_or_else(|| "profile_acquisition_profile_reuse_missing".to_string())?;
        let service_request = decision
            .get("serviceRequest")
            .and_then(Value::as_object)
            .ok_or_else(|| "profile_acquisition_service_request_missing".to_string())?;
        let recommended_action = profile_reuse
            .get("recommendedAction")
            .and_then(Value::as_str)
            .ok_or_else(|| "profile_acquisition_recommended_action_missing".to_string())?;
        let available = service_request
            .get("available")
            .and_then(Value::as_bool)
            .ok_or_else(|| "profile_acquisition_availability_missing".to_string())?;
        let acquisition_blocker = service_request
            .get("acquisitionBlocker")
            .and_then(Value::as_str)
            .map(str::to_string);
        let disposition = if recommended_action == "reuse_existing_browser" && available {
            ProfileAcquisitionDisposition::ReuseExistingBrowser
        } else if recommended_action == "launch_new_browser" && available {
            ProfileAcquisitionDisposition::LaunchNewBrowser
        } else {
            ProfileAcquisitionDisposition::Blocked
        };
        let browser_id = profile_reuse
            .get("reusableBrowserId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);
        let session_name = service_request
            .get("request")
            .and_then(Value::as_object)
            .and_then(|request| request.get("sessionName"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                profile_reuse
                    .get("reusableSessionName")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
            })
            .map(str::to_string);

        if disposition == ProfileAcquisitionDisposition::ReuseExistingBrowser
            && (browser_id.is_none() || session_name.is_none())
        {
            return Err("profile_acquisition_reuse_route_incomplete".to_string());
        }

        Ok(Self {
            disposition,
            selected_profile_id: decision
                .get("profileId")
                .and_then(Value::as_str)
                .map(str::to_string),
            browser_id,
            session_name,
            acquisition_blocker,
            service_request_available: available,
            profile_reuse_action: recommended_action.to_string(),
            profile_reuse_reasons: profile_reuse
                .get("reasons")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            replacement_eligible: decision
                .get("lifecycleReplacement")
                .and_then(|value| value.get("replacementEligible"))
                .and_then(Value::as_bool)
                == Some(true),
            replacement_session_name: decision
                .get("lifecycleReplacement")
                .and_then(|value| value.get("replacementSessionName"))
                .and_then(Value::as_str)
                .map(str::to_string),
            runtime_owner_registry_revision: decision
                .get("lifecycleReplacement")
                .and_then(|value| value.get("registryRevision"))
                .and_then(Value::as_u64),
            owner_id: decision
                .get("lifecycleReplacement")
                .and_then(|value| value.get("ownerId"))
                .and_then(Value::as_str)
                .map(str::to_string),
            owner_generation: decision
                .get("lifecycleReplacement")
                .and_then(|value| value.get("ownerGeneration"))
                .and_then(Value::as_u64),
        })
    }

    pub(crate) fn disposition(&self) -> ProfileAcquisitionDisposition {
        self.disposition
    }

    pub(crate) fn selected_profile_id(&self) -> Option<&str> {
        self.selected_profile_id.as_deref()
    }

    pub(crate) fn browser_id(&self) -> Option<&str> {
        self.browser_id.as_deref()
    }

    pub(crate) fn session_name(&self) -> Option<&str> {
        self.session_name.as_deref()
    }

    pub(crate) fn acquisition_blocker(&self) -> Option<&str> {
        self.acquisition_blocker.as_deref()
    }

    /// Apply the already-joined route decision without reparsing public plan
    /// JSON in the action-runtime caller.
    pub(crate) fn apply_to_service_command(
        &self,
        command: &mut Value,
        authenticated_principal: Option<&AuthenticatedServicePrincipal>,
    ) -> Result<(), String> {
        if !self.service_request_available {
            return Err(format!(
                "service_access_plan_request_unavailable:{}",
                self.acquisition_blocker()
                    .unwrap_or("service_request_unavailable")
            ));
        }

        if self.disposition != ProfileAcquisitionDisposition::ReuseExistingBrowser {
            if service_request_has_partial_route_hints(command) {
                let requested_session = command.get("sessionName").and_then(Value::as_str);
                let exact_terminal_replacement = !service_request_has_browser_hint(command)
                    && self.replacement_eligible
                    && requested_session == self.replacement_session_name.as_deref();
                let exact_authenticated_cold_route = !service_request_has_browser_hint(command)
                    && self.profile_reuse_action == "launch_new_browser"
                    && self
                        .profile_reuse_reasons
                        .iter()
                        .any(|reason| reason == "explicit_authenticated_cold_route_selected")
                    && requested_session == self.session_name();
                let exact_terminal_launch_route = !service_request_has_browser_hint(command)
                    && self.replacement_eligible
                    && self
                        .profile_reuse_reasons
                        .iter()
                        .any(|reason| reason == "explicit_session_terminal_launch_selected")
                    && requested_session == self.session_name()
                    && requested_session != self.replacement_session_name.as_deref();
                if !exact_terminal_replacement
                    && !exact_authenticated_cold_route
                    && !exact_terminal_launch_route
                {
                    return Err("service_access_plan_incomplete_route_hints".to_string());
                }
                if let Some(authority) = authenticated_principal {
                    self.attach_launch_route_authorization(
                        command,
                        authority,
                        if exact_terminal_replacement {
                            "terminal_replacement"
                        } else {
                            "authenticated_cold"
                        },
                    );
                }
                return Ok(());
            }
            if !service_request_has_session_hint(command) {
                if let Some(session_name) = self.session_name() {
                    command["sessionName"] = json!(session_name);
                }
            }
            if let Some(authority) = authenticated_principal {
                let planned_session = command.get("sessionName").and_then(Value::as_str);
                self.attach_launch_route_authorization(
                    command,
                    authority,
                    if self.replacement_eligible
                        && planned_session == self.replacement_session_name.as_deref()
                        && self.replacement_session_name.is_some()
                    {
                        "terminal_replacement"
                    } else {
                        "authenticated_cold"
                    },
                );
            }
            return Ok(());
        }

        let browser_id = self
            .browser_id()
            .ok_or_else(|| "service_access_plan_reuse_missing_browser_id".to_string())?;
        let session_name = self
            .session_name()
            .ok_or_else(|| "service_access_plan_reuse_missing_session_name".to_string())?;
        command["browserId"] = json!(browser_id);
        command["sessionName"] = json!(session_name);
        Ok(())
    }

    fn attach_launch_route_authorization(
        &self,
        command: &mut Value,
        authority: &AuthenticatedServicePrincipal,
        route_kind: &str,
    ) {
        let session_name = command.get("sessionName").and_then(Value::as_str);
        if session_name.is_none()
            || self.selected_profile_id() != Some(authority.profile_id.as_str())
        {
            return;
        }
        command["serviceProfileRouteAuthorization"] = json!({
            "schemaVersion": "agent-browser.profile-launch-route-authorization.v1",
            "kind": route_kind,
            "sessionName": session_name,
            "profileId": self.selected_profile_id,
            "principalId": authority.principal_id,
            "capabilityId": authority.capability_id,
            "capabilityRevision": authority.capability_revision,
            "runtimeOwnerRegistryRevision": self.runtime_owner_registry_revision,
            "ownerId": self.owner_id,
            "ownerGeneration": self.owner_generation,
        });
    }
}

fn service_request_route_hint_count(command: &Value) -> usize {
    ["browserId", "sessionName"]
        .iter()
        .filter(|key| {
            command
                .get(**key)
                .and_then(Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
        })
        .count()
}

fn service_request_has_browser_hint(command: &Value) -> bool {
    service_request_has_route_hint_field(command, "browserId")
}

fn service_request_has_session_hint(command: &Value) -> bool {
    service_request_has_route_hint_field(command, "sessionName")
}

fn service_request_has_route_hint_field(command: &Value, field: &str) -> bool {
    command
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn service_request_has_partial_route_hints(command: &Value) -> bool {
    service_request_route_hint_count(command) == 1
}
