//! First-tab admission for native commands already routed to an explicit lane.
//! This grants profile permissions only; normal launch still proves physical
//! ownership. Retained browsers must use child custody instead of cold admission.

use super::{apply_auto_launch_command_hints, launch_options_from_env, DaemonState};
use crate::native::service_lifecycle::ServiceLaunchMetadata;
use crate::native::service_model::ServiceState;
use crate::native::service_profile_access_policy::{
    evaluate_profile_access, ProfileAccessEvaluation, ProfileChildAccess, ProfileIdentityAssurance,
    ProfilePermission,
};
use serde_json::{json, Value};

pub(super) fn admit_cold_navigation(
    command: &Value,
    daemon: &DaemonState,
    snapshot: &ServiceState,
) -> Result<Option<Value>, String> {
    if command["action"] != "navigate"
        || daemon.browser.is_some()
        || command.get("targetId").is_some()
        || command.get("tabId").is_some()
        || snapshot
            .browsers
            .contains_key(&format!("session:{}", daemon.session_id))
        || snapshot
            .runtime_owner_registry
            .binding_for_session(&daemon.session_id)?
            .is_some()
    {
        return Ok(None);
    }
    let Some(subject) = command["clientSubjectId"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
    else {
        return Ok(None);
    };
    if command
        .get("sessionName")
        .is_some_and(|value| value.as_str() != Some(&daemon.session_id))
        || command.get("browserId").is_some_and(|value| {
            value.as_str() != Some(format!("session:{}", daemon.session_id).as_str())
        })
    {
        return Err(
            "service_tab_target_selector_conflict: cold navigation route differs from native lane"
                .into(),
        );
    }
    let mut options = launch_options_from_env();
    let (_, selection, _, mut admitted) =
        apply_auto_launch_command_hints(&mut options, command, None, &daemon.session_id)?;
    let metadata = ServiceLaunchMetadata::from_launch_options(&options, Some(&admitted), selection);
    let Some(profile_id) = metadata.profile_id.as_deref() else {
        return Ok(None);
    };
    // A missing daemon-local browser is not evidence that a retained owner or
    // another lane's browser can be replaced. Those routes need exact handles.
    if snapshot
        .browsers
        .values()
        .any(|browser| browser.profile_id.as_deref() == Some(profile_id))
        || snapshot
            .tabs
            .values()
            .any(|tab| tab.owner_session_id.as_deref() == Some(&daemon.session_id))
    {
        return Ok(None);
    }
    let connection = command["connectionInstanceId"].as_str().map(str::to_string);
    let profile = snapshot.profiles.get(profile_id);
    let mut child = None;
    for permission in [ProfilePermission::ProfileUse, ProfilePermission::TabCreate] {
        let (policy, decision) = evaluate_profile_access(ProfileAccessEvaluation {
            profile_id,
            explicit_policy: profile.and_then(|profile| profile.access_policy.as_ref()),
            subject_id: Some(subject.to_string()),
            // Native labels never establish registered-capability authority.
            assurance: ProfileIdentityAssurance::SelfDeclared,
            connection_instance_id: connection.clone(),
            permission,
            operation: "native_cold_navigation",
            incompatible_occupancy: Vec::new(),
        });
        if !decision.allowed {
            return Err(format!(
                "profile_access_denied: native cold navigation; decision={}",
                serde_json::to_string(&decision).map_err(|error| error.to_string())?
            ));
        }
        child = Some(ProfileChildAccess::from_admission(
            &policy,
            &decision,
            connection.clone(),
        ));
    }
    // Pin the identity evaluated above so launch cannot select another profile
    // from ambient defaults. Keep the original explicit lane unchanged.
    admitted["runtimeProfile"] = json!(profile_id);
    if let Some(path) = metadata.user_data_dir {
        admitted["profile"] = json!(path);
    }
    admitted["identityAssurance"] = json!("self-declared");
    admitted["profileChildAccess"] =
        serde_json::to_value(child).map_err(|error| error.to_string())?;
    Ok(Some(admitted))
}
