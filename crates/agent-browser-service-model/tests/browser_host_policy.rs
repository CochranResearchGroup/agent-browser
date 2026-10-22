use agent_browser_service_model::*;
use std::collections::{BTreeMap, BTreeSet};
fn denied(action: &str) -> BrowserTaskPolicy {
    BrowserTaskPolicy {
        denied_actions: BTreeSet::from([action.into()]),
        ..Default::default()
    }
}
#[test]
fn unrestricted_tasks_need_no_grant_and_app_defaults_can_be_replaced_then_restored_per_task() {
    let mut policies = BrowserHostPolicies::default();
    policies
        .check("app", "alpha", "navigate", "ordinary", None)
        .unwrap();
    policies
        .update(
            0,
            "app",
            "alpha",
            "set_app",
            Some(denied("navigate")),
            "app",
            "Restrict navigation",
        )
        .unwrap();
    assert!(policies
        .check("app", "alpha", "browser_session_navigate", "a", None)
        .is_err());
    assert!(policies.check("app", "beta", "goto", "b", None).is_err());
    policies
        .update(
            1,
            "app",
            "alpha",
            "set_task",
            Some(BrowserTaskPolicy::default()),
            "app",
            "Allow this task",
        )
        .unwrap();
    policies
        .check("app", "alpha", "navigate", "a", None)
        .unwrap();
    assert!(policies
        .check("app", "beta", "navigate", "b", None)
        .is_err());
    policies
        .update(
            2,
            "app",
            "alpha",
            "clear_task",
            None,
            "app",
            "Restore defaults",
        )
        .unwrap();
    assert!(policies
        .check("app", "alpha", "navigate", "a", None)
        .is_err());
    policies
        .update(
            3,
            "app",
            "alpha",
            "clear_app",
            None,
            "app",
            "Remove app restrictions",
        )
        .unwrap();
    assert!(!policies.has_rules());
    assert!(policies.initialized_hosts.contains("app"));
    let restored: BrowserHostPolicies =
        serde_json::from_slice(&serde_json::to_vec(&policies).unwrap()).unwrap();
    assert!(restored.initialized_hosts.contains("app"));
    restored
        .check("app", "alpha", "navigate", "a", None)
        .unwrap();
}
#[test]
fn explicit_override_is_exactly_scoped_and_unrelated_task_changes_do_not_revoke_it() {
    let mut policies = BrowserHostPolicies::default();
    policies
        .update(
            0,
            "app",
            "alpha",
            "set_app",
            Some(denied("click")),
            "app",
            "Restrict clicks",
        )
        .unwrap();
    policies
        .issue_override(
            1,
            "override".into(),
            "app",
            "alpha",
            "browser_click",
            "request-a",
            "operator",
            "One scoped click",
        )
        .unwrap();
    policies
        .check("app", "alpha", "click", "request-a", Some("override"))
        .unwrap();
    assert!(policies
        .check("app", "beta", "click", "request-a", Some("override"))
        .is_err());
    assert!(policies
        .check("app", "alpha", "click", "request-b", Some("override"))
        .is_err());
    policies
        .update(
            1,
            "app",
            "beta",
            "set_task",
            Some(denied("evaluate")),
            "app",
            "Peer rule",
        )
        .unwrap();
    policies
        .check("app", "alpha", "click", "request-a", Some("override"))
        .unwrap();
    let mut closed = policies.clone();
    let prior_audit = closed.audit.clone();
    closed.retain_active_tasks(&["beta".to_string()].into_iter().collect());
    assert!(!closed.operator_overrides.contains_key("override"));
    assert!(closed.task_overrides.contains_key("beta"));
    assert_eq!(closed.app_defaults, policies.app_defaults);
    assert_eq!(closed.audit, prior_audit);

    policies
        .update(
            2,
            "app",
            "alpha",
            "set_task",
            Some(denied("click")),
            "app",
            "Replace task policy",
        )
        .unwrap();
    assert!(policies
        .check("app", "alpha", "click", "request-a", Some("override"))
        .is_err());
}
#[test]
fn failed_updates_preserve_state_and_status_control_remain_available() {
    let mut policies = BrowserHostPolicies::default();
    let before = policies.clone();
    assert!(policies
        .update(0, "app", "task", "set_app", None, "app", "Missing policy")
        .is_err());
    assert_eq!(policies, before);
    assert!(policies
        .update(
            1,
            "app",
            "task",
            "set_app",
            Some(denied("click")),
            "app",
            "Stale revision"
        )
        .is_err());
    assert_eq!(policies, before);
    assert!(denied("browser_control").validate().is_err());
    let mut policy = denied("navigate");
    policy.artifact_destinations = BTreeMap::from([("exports".into(), "relative".into())]);
    assert!(policy.validate().is_err());
    assert!(serde_json::from_value::<BrowserTaskPolicy>(
        serde_json::json!({"version":1,"grantEverything":true})
    )
    .is_err());
    policies
        .check("app", "task", "browser_control", "status", None)
        .unwrap();
    policies
        .check("app", "task", "browser_session_policy", "status", None)
        .unwrap();
}
