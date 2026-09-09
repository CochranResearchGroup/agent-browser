//! Request-bound operator focus proof. It grants no agent child ownership.
use super::*;

const PURPOSE: &str = "agent-browser.operator-focus.v1";
const TTL_SECONDS: u64 = 30;

fn current_binding(
    state: &crate::native::service_model::ServiceState,
    command: &serde_json::Value,
) -> Result<OperatorFocusBinding, String> {
    let field = |name: &str| {
        command
            .get(name)
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("operator_focus_binding_required: missing {name}"))
    };
    if command.get("action").and_then(serde_json::Value::as_str) != Some("view_focus") {
        return Err("operator_focus_action_mismatch".into());
    }
    let browser_id = field("browserId")?;
    let browser = state
        .browsers
        .get(&browser_id)
        .ok_or("operator_focus_browser_missing")?;
    let profile_id = browser
        .profile_id
        .clone()
        .ok_or("operator_focus_profile_missing")?;
    let profile = state
        .profiles
        .get(&profile_id)
        .ok_or("operator_focus_profile_missing")?;
    let route_id = field("routeId")?;
    let route = state
        .remote_view_routes
        .get(&route_id)
        .ok_or("operator_focus_route_missing")?;
    Ok(OperatorFocusBinding {
        request_id: field("id")?,
        browser_id,
        profile_id,
        session_name: field("sessionName")?,
        target_id: field("targetId")?,
        browser_pid: browser.pid.ok_or("operator_focus_process_missing")?,
        route_id,
        controller_lease_id: field("controllerLeaseId")?,
        controller_epoch: route.controller_epoch,
        policy_revision: profile
            .access_policy
            .as_ref()
            .map_or(1, |policy| policy.revision),
    })
}

/// Called only after HTTP has authenticated the dashboard cookie. The public
/// normalizer removes caller-supplied proof tokens before this issuance step.
pub(crate) fn issue_operator_focus(
    state: &crate::native::service_model::ServiceState,
    command: &serde_json::Value,
    username: &str,
) -> Result<String, String> {
    let binding = current_binding(state, command)?;
    let now = now_epoch_seconds();
    authorize_current(state, username, &binding, now)?;
    issue(&load_auth_store()?, username, binding, now)
}

/// Verify again at child admission without transferring the child's owner.
pub(crate) fn verify_operator_focus(
    state: &crate::native::service_model::ServiceState,
    command: &serde_json::Value,
) -> Result<(), String> {
    let binding = current_binding(state, command)?;
    let token = command
        .get("operatorFocusProofToken")
        .and_then(serde_json::Value::as_str)
        .ok_or("operator_focus_authority_required: authenticated operator proof missing")?;
    let now = now_epoch_seconds();
    let username = verify(&load_auth_store()?, token, &binding, now)?;
    authorize_current(state, &username, &binding, now)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OperatorFocusBinding {
    pub request_id: String,
    pub browser_id: String,
    pub session_name: String,
    pub profile_id: String,
    pub target_id: String,
    pub browser_pid: u32,
    pub route_id: String,
    pub controller_lease_id: String,
    pub controller_epoch: u64,
    pub policy_revision: u64,
}

impl OperatorFocusBinding {
    fn valid(&self) -> bool {
        self.browser_pid != 0
            && self.controller_epoch != 0
            && self.policy_revision != 0
            && [
                &self.request_id,
                &self.browser_id,
                &self.session_name,
                &self.profile_id,
                &self.target_id,
                &self.route_id,
                &self.controller_lease_id,
            ]
            .iter()
            .all(|value| !value.trim().is_empty())
    }
}

/// Recompute current policy and controller authority without changing child custody.
fn authorize_current(
    state: &crate::native::service_model::ServiceState,
    username: &str,
    binding: &OperatorFocusBinding,
    now: u64,
) -> Result<(), String> {
    use crate::native::service_model::{BrowserHealth, TabLifecycle};
    use crate::native::service_profile_access_policy::{
        evaluate_profile_access, ProfileAccessEvaluation, ProfileIdentityAssurance,
        ProfilePermission,
    };
    let denied = || {
        "operator_focus_binding_changed: inspect current profile and controller authority"
            .to_string()
    };
    let browser = state.browsers.get(&binding.browser_id).ok_or_else(denied)?;
    let tab = state
        .tabs
        .get(&format!("target:{}", binding.target_id))
        .ok_or_else(denied)?;
    if !binding.valid()
        || browser.health != BrowserHealth::Ready
        || browser.pid != Some(binding.browser_pid)
        || browser.profile_id.as_deref() != Some(binding.profile_id.as_str())
        || !browser.active_session_ids.contains(&binding.session_name)
        || tab.browser_id != binding.browser_id
        || tab.target_id.as_deref() != Some(binding.target_id.as_str())
        || tab.lifecycle != TabLifecycle::Ready
        || tab.owner_session_id.as_ref().or(tab.session_id.as_ref()) != Some(&binding.session_name)
    {
        return Err(denied());
    }
    let profile = state.profiles.get(&binding.profile_id).ok_or_else(denied)?;
    let subject = format!("dashboard:{username}");
    let (policy, decision) = evaluate_profile_access(ProfileAccessEvaluation {
        profile_id: &binding.profile_id,
        explicit_policy: profile.access_policy.as_ref(),
        subject_id: Some(subject.clone()),
        assurance: ProfileIdentityAssurance::AuthenticatedIngress,
        connection_instance_id: None,
        permission: ProfilePermission::ViewOpen,
        operation: "view_focus",
        incompatible_occupancy: Vec::new(),
    });
    if !decision.allowed || policy.revision != binding.policy_revision {
        return Err(
            "operator_focus_profile_policy_denied: inspect current profile view permission".into(),
        );
    }
    let route = state
        .remote_view_routes
        .get(&binding.route_id)
        .ok_or_else(denied)?;
    let lease = state
        .viewer_leases
        .get(&binding.controller_lease_id)
        .ok_or_else(denied)?;
    // Viewer leases without an expiry remain active until release or controller
    // replacement. The signed focus proof itself is always short-lived.
    let unexpired = lease.expires_at.as_deref().is_none_or(|value| {
        chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|value| value.timestamp())
            .is_some_and(|expires| expires > 0 && expires as u64 > now)
    });
    if route.browser_id.as_ref() != Some(&binding.browser_id)
        || route.session_id.as_ref() != Some(&binding.session_name)
        || route.display_allocation_id != browser.display_allocation_id
        || route.display_allocation_id.is_none()
        || route.read_only
        || route.state != "ready"
        || route.controller_epoch != binding.controller_epoch
        || route.controller_lease_id.as_ref() != Some(&binding.controller_lease_id)
        || !route
            .viewer_lease_ids
            .contains(&binding.controller_lease_id)
        || lease.id != binding.controller_lease_id
        || lease.route_id.as_ref() != Some(&binding.route_id)
        || lease.browser_id.as_ref() != Some(&binding.browser_id)
        || lease.viewer_id.as_ref() != Some(&subject)
        || lease.viewer_role != "controller"
        || lease.state != "controlling"
        || !unexpired
    {
        return Err(
            "operator_focus_controller_required: acquire current controller authority".into(),
        );
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FocusProof {
    purpose: String,
    username: String,
    issued_at: u64,
    expires_at: u64,
    binding: OperatorFocusBinding,
}

fn issue(
    store: &DashboardAuthStore,
    username: &str,
    binding: OperatorFocusBinding,
    now: u64,
) -> Result<String, String> {
    if !binding.valid()
        || !store
            .users
            .iter()
            .any(|user| user.username == username && user.role == DASHBOARD_ROLE_SUPERUSER)
    {
        return Err(
            "operator_focus_authority_required: current superuser and exact target required".into(),
        );
    }
    let payload = serde_json::to_vec(&FocusProof {
        purpose: PURPOSE.into(),
        username: username.into(),
        issued_at: now,
        expires_at: now.saturating_add(TTL_SECONDS),
        binding,
    })
    .map_err(|_| "operator_focus_proof_encoding_failed".to_string())?;
    let signature = sign_payload(store, &payload)?;
    Ok(format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(payload),
        URL_SAFE_NO_PAD.encode(signature)
    ))
}

fn verify(
    store: &DashboardAuthStore,
    token: &str,
    binding: &OperatorFocusBinding,
    now: u64,
) -> Result<String, String> {
    let denied =
        || "operator_focus_authority_unproven: refresh authenticated operator focus".to_string();
    if token.len() > 8192 || !binding.valid() {
        return Err(denied());
    }
    let (payload, signature) = token.split_once('.').ok_or_else(denied)?;
    let payload = URL_SAFE_NO_PAD.decode(payload).map_err(|_| denied())?;
    let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| denied())?;
    if !constant_time_eq(&sign_payload(store, &payload)?, &signature) {
        return Err(denied());
    }
    let proof: FocusProof = serde_json::from_slice(&payload).map_err(|_| denied())?;
    if proof.purpose != PURPOSE
        || proof.binding != *binding
        || proof.issued_at > now
        || proof.expires_at <= now
        || proof.expires_at.saturating_sub(proof.issued_at) != TTL_SECONDS
        || !store
            .users
            .iter()
            .any(|user| user.username == proof.username && user.role == DASHBOARD_ROLE_SUPERUSER)
    {
        return Err(denied());
    }
    Ok(proof.username)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_focus_requires_current_policy_controller_and_target_without_rebinding_child() {
        use crate::native::service_model::ServiceState;
        let binding = OperatorFocusBinding {
            request_id: "request-a".into(),
            browser_id: "browser-a".into(),
            session_name: "session-a".into(),
            profile_id: "profile-a".into(),
            target_id: "target-a".into(),
            browser_pid: 100,
            route_id: "route-a".into(),
            controller_lease_id: "controller-a".into(),
            controller_epoch: 1,
            policy_revision: 1,
        };
        let mut state = ServiceState::default();
        state.profiles.insert(
            "profile-a".into(),
            serde_json::from_value(json!({"id":"profile-a"})).unwrap(),
        );
        state.browsers.insert(
            "browser-a".into(),
            serde_json::from_value(json!({
                "id":"browser-a", "profileId":"profile-a", "health":"ready", "pid":100,
                "activeSessionIds":["session-a"], "displayAllocationId":"display-a"
            }))
            .unwrap(),
        );
        state.tabs.insert(
            "target:target-a".into(),
            serde_json::from_value(json!({
                "id":"target:target-a", "browserId":"browser-a", "targetId":"target-a",
                "sessionId":"session-a", "ownerSessionId":"session-a", "lifecycle":"ready",
                "principalId":"original-agent"
            }))
            .unwrap(),
        );
        state.remote_view_routes.insert(
            "route-a".into(),
            serde_json::from_value(json!({
                "id":"route-a", "browserId":"browser-a", "sessionId":"session-a",
                "displayAllocationId":"display-a", "state":"ready", "readOnly":false,
                "controllerLeaseId":"controller-a", "controllerEpoch":1,
                "viewerLeaseIds":["controller-a"]
            }))
            .unwrap(),
        );
        state.viewer_leases.insert(
            "controller-a".into(),
            serde_json::from_value(json!({
                "id":"controller-a", "routeId":"route-a", "browserId":"browser-a",
                "viewerId":"dashboard:operator", "viewerRole":"controller", "state":"controlling",
                "expiresAt":"1970-01-01T01:00:00Z"
            }))
            .unwrap(),
        );
        let original = serde_json::to_value(&state).unwrap();
        authorize_current(&state, "operator", &binding, 1000).unwrap();
        assert_eq!(serde_json::to_value(&state).unwrap(), original);
        assert!(authorize_current(&state, "other", &binding, 1000).is_err());
        assert!(authorize_current(&state, "operator", &binding, 3600).is_err());
        let mut unbounded = state.clone();
        unbounded
            .viewer_leases
            .get_mut("controller-a")
            .unwrap()
            .expires_at = None;
        authorize_current(&unbounded, "operator", &binding, 3600).unwrap();
        for (pointer, value) in [
            ("/browsers/browser-a/pid", json!(101)),
            ("/browsers/browser-a/profileId", json!("other")),
            ("/tabs/target:target-a/targetId", json!("other")),
            ("/tabs/target:target-a/lifecycle", json!("closed")),
            ("/remoteViewRoutes/route-a/controllerEpoch", json!(2)),
            ("/remoteViewRoutes/route-a/readOnly", json!(true)),
            ("/remoteViewRoutes/route-a/viewerLeaseIds", json!([])),
            ("/viewerLeases/controller-a/viewerRole", json!("observer")),
            ("/viewerLeases/controller-a/state", json!("released")),
            ("/viewerLeases/controller-a/expiresAt", json!("invalid")),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = value;
            let changed = serde_json::from_value(changed).unwrap();
            assert!(
                authorize_current(&changed, "operator", &binding, 1000).is_err(),
                "{pointer}"
            );
        }
        let profile = state.profiles.get_mut("profile-a").unwrap();
        let mut policy = crate::native::service_profile_access_policy::ServiceProfileAccessPolicy::shared_local_default("profile-a");
        policy.default_permissions.clear();
        profile.access_policy = Some(policy);
        assert!(authorize_current(&state, "operator", &binding, 1000).is_err());
    }

    #[test]
    fn operator_focus_proof_binds_request_target_and_current_role() {
        let mut store = DashboardAuthStore {
            version: 1,
            created_at: String::new(),
            session_secret: URL_SAFE_NO_PAD.encode([17u8; 32]),
            users: vec![DashboardAuthUser {
                username: "operator".into(),
                display_name: "Operator".into(),
                role: DASHBOARD_ROLE_SUPERUSER.into(),
                password_hash: String::new(),
                created_at: String::new(),
                bootstrap: false,
            }],
        };
        let binding = OperatorFocusBinding {
            request_id: "request-a".into(),
            browser_id: "browser-a".into(),
            session_name: "session-a".into(),
            profile_id: "profile-a".into(),
            target_id: "target-a".into(),
            browser_pid: 100,
            route_id: "route-a".into(),
            controller_lease_id: "controller-a".into(),
            controller_epoch: 1,
            policy_revision: 1,
        };
        let token = issue(&store, "operator", binding.clone(), 1000).unwrap();
        assert_eq!(verify(&store, &token, &binding, 1001).unwrap(), "operator");
        for field in [
            "requestId",
            "browserId",
            "sessionName",
            "profileId",
            "targetId",
            "browserPid",
            "routeId",
            "controllerLeaseId",
            "controllerEpoch",
            "policyRevision",
        ] {
            let mut changed = serde_json::to_value(&binding).unwrap();
            changed[field] = if matches!(field, "browserPid" | "controllerEpoch" | "policyRevision")
            {
                json!(101)
            } else {
                json!("other")
            };
            let changed = serde_json::from_value(changed).unwrap();
            assert!(verify(&store, &token, &changed, 1001).is_err(), "{field}");
        }
        assert!(verify(&store, &format!("{token}x"), &binding, 1001).is_err());
        assert!(verify(&store, &token, &binding, 999).is_err());
        assert!(verify(&store, &token, &binding, 1030).is_err());
        assert!(issue(&store, "unknown", binding.clone(), 1000).is_err());
        store.users[0].role = DASHBOARD_ROLE_OBSERVER.into();
        assert!(issue(&store, "operator", binding.clone(), 1000).is_err());
        assert!(verify(&store, &token, &binding, 1001).is_err());
    }
}
