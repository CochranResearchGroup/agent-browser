//! Optional app/task rules narrow cooperating dispatch without adding authority.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserTaskPolicy {
    pub version: u32,
    #[serde(default)]
    pub denied_actions: BTreeSet<String>,
    #[serde(default)]
    pub artifact_destinations: BTreeMap<String, String>,
}
impl Default for BrowserTaskPolicy {
    fn default() -> Self {
        Self {
            version: 1,
            denied_actions: BTreeSet::new(),
            artifact_destinations: BTreeMap::new(),
        }
    }
}
impl BrowserTaskPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.denied_actions.len() > 128
            || self.artifact_destinations.len() > 32
        {
            return Err("browser_policy_invalid".into());
        }
        for action in &self.denied_actions {
            if !policy_token(action) || policy_status_action(action) {
                return Err("browser_policy_action_invalid: status, control and policy management remain available".into());
            }
        }
        for (handle, root) in &self.artifact_destinations {
            if !policy_token(handle)
                || !std::path::Path::new(root).is_absolute()
                || root.len() > 4096
                || root.chars().any(char::is_control)
            {
                return Err("browser_policy_artifact_destination_invalid".into());
            }
        }
        Ok(())
    }
}
fn policy_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}
/// Dispatch-level aliases only. Rules are not a website semantic sandbox.
pub fn canonical_browser_policy_action(action: &str) -> &str {
    match action {
        "browser_navigate" | "browser_session_navigate" | "goto" => "navigate",
        "browser_session_open" => "launch",
        "browser_screenshot" => "screenshot",
        "browser_download" => "download",
        "browser_wait_for_download" => "waitfordownload",
        "browser_snapshot" => "snapshot",
        "browser_evaluate" | "eval" => "evaluate",
        "browser_click" => "click",
        "browser_fill" => "fill",
        "browser_tab_new" | "browser_session_tab_new" => "tab_new",
        "browser_tab_close" | "browser_session_tab_close" => "tab_close",
        "browser_session_close" => "close",
        "browser_session_focus" => "view_focus",
        other => other,
    }
}
pub fn policy_status_action(action: &str) -> bool {
    matches!(
        action,
        "status"
            | "ping"
            | "service_status"
            | "browser_session_status"
            | "browser_session_retention"
            | "browser_control"
            | "browser_policy"
            | "browser_session_policy"
            | "browser_host_status"
            | "browser_host_events"
            | "browser_reconnect"
            | "browser_session_reconnect"
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserPolicyOverride {
    pub task_id: String,
    pub host_id: String,
    pub policy: BrowserTaskPolicy,
    pub action: String,
    pub request_id: String,
    pub revision: u64,
    pub actor: String,
    pub reason: String,
}
/// Active override working-set policy. Archived grants retain exactly the same
/// task/action/request/policy matching and revocation checks as active grants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserOverrideCapacity {
    pub active_limit: usize,
}
impl Default for BrowserOverrideCapacity {
    fn default() -> Self {
        Self { active_limit: 256 }
    }
}
impl BrowserOverrideCapacity {
    pub fn validate(&self) -> Result<(), String> {
        if self.active_limit == 0 {
            return Err("browser_policy_override_capacity_invalid: set capacity.activeLimit to a positive integer; archived grants preserve exact authorization".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserHostPolicies {
    pub revision: u64,
    #[serde(default)]
    pub initialized_hosts: BTreeSet<String>,
    #[serde(default)]
    pub app_defaults: BTreeMap<String, BrowserTaskPolicy>,
    #[serde(default)]
    pub task_overrides: BTreeMap<String, BrowserTaskPolicy>,
    #[serde(default)]
    pub operator_overrides: BTreeMap<String, BrowserPolicyOverride>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub archived_operator_overrides: BTreeMap<String, BrowserPolicyOverride>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub revoked_override_ids: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub app_override_capacities: BTreeMap<String, BrowserOverrideCapacity>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub task_override_capacities: BTreeMap<String, BrowserOverrideCapacity>,
    #[serde(default)]
    pub audit: Vec<BrowserPolicyAudit>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserPolicyAudit {
    pub revision: u64,
    pub task_id: String,
    pub operation: String,
    pub actor: String,
    pub reason: String,
}
impl BrowserHostPolicies {
    pub fn is_empty(&self) -> bool {
        self.revision == 0
            && self.initialized_hosts.is_empty()
            && self.app_defaults.is_empty()
            && self.task_overrides.is_empty()
            && self.operator_overrides.is_empty()
            && self.archived_operator_overrides.is_empty()
            && self.revoked_override_ids.is_empty()
            && self.app_override_capacities.is_empty()
            && self.task_override_capacities.is_empty()
            && self.audit.is_empty()
    }
    pub fn has_rules(&self) -> bool {
        !self.app_defaults.is_empty() || !self.task_overrides.is_empty()
    }
    /// Task-scoped rules expire with their task. Archived closed grants remain
    /// attributable revoked evidence and can never authorize another task.
    pub fn retain_active_tasks(&mut self, active: &BTreeSet<String>) {
        self.task_overrides.retain(|id, _| active.contains(id));
        let closed: Vec<String> = self
            .operator_overrides
            .iter()
            .chain(self.archived_operator_overrides.iter())
            .filter(|(_, grant)| !active.contains(&grant.task_id))
            .map(|(id, _)| id.clone())
            .collect();
        for id in closed {
            self.revoked_override_ids.insert(id.clone());
            if let Some(grant) = self.operator_overrides.remove(&id) {
                self.archived_operator_overrides.insert(id, grant);
            }
        }
        self.task_override_capacities
            .retain(|task, _| active.contains(task));
    }

    pub fn override_capacity(
        &self,
        host: &str,
        task: &str,
    ) -> (BrowserOverrideCapacity, &'static str) {
        if let Some(policy) = self.task_override_capacities.get(task) {
            return (policy.clone(), "task_override");
        }
        if let Some(policy) = self.app_override_capacities.get(host) {
            return (policy.clone(), "app_default");
        }
        (BrowserOverrideCapacity::default(), "system_default")
    }
    #[allow(clippy::too_many_arguments)]
    pub fn configure_override_capacity(
        &mut self,
        expected: u64,
        host: &str,
        task: &str,
        scope: &str,
        capacity: BrowserOverrideCapacity,
        actor: &str,
        reason: &str,
    ) -> Result<(), String> {
        if self.revision != expected {
            return Err("browser_policy_revision_changed: read browser_policy status and retry with its current expectedRevision".into());
        }
        attribution(actor, reason)?;
        self.revision
            .checked_add(1)
            .ok_or("browser_policy_revision_exhausted")?;
        capacity.validate()?;
        match scope {
            "app" => {
                self.app_override_capacities.insert(host.into(), capacity);
            }
            "task" => {
                self.task_override_capacities.insert(task.into(), capacity);
            }
            _ => {
                return Err(
                    "browser_policy_override_capacity_scope_invalid: choose app or task".into(),
                )
            }
        }
        self.advance(task, "override_capacity", actor, reason)
    }

    pub fn effective(&self, host: &str, task: &str) -> Option<&BrowserTaskPolicy> {
        self.task_overrides
            .get(task)
            .or_else(|| self.app_defaults.get(host))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        expected: u64,
        host: &str,
        task: &str,
        operation: &str,
        policy: Option<BrowserTaskPolicy>,
        actor: &str,
        reason: &str,
    ) -> Result<(), String> {
        if self.revision != expected {
            return Err("browser_policy_revision_changed".into());
        }
        attribution(actor, reason)?;
        self.revision
            .checked_add(1)
            .ok_or("browser_policy_revision_exhausted")?;
        if let Some(policy) = &policy {
            policy.validate()?;
        }
        if matches!(operation, "set_app" | "set_task") && policy.is_none() {
            return Err("browser_policy_required".into());
        }
        match operation {
            "set_app" => {
                self.initialized_hosts.insert(host.into());
                self.app_defaults
                    .insert(host.into(), policy.ok_or("browser_policy_required")?);
            }
            "set_task" => {
                self.task_overrides
                    .insert(task.into(), policy.ok_or("browser_policy_required")?);
            }
            "clear_task" => {
                self.task_overrides.remove(task);
            }
            "clear_app" => {
                self.initialized_hosts.insert(host.into());
                self.app_defaults.remove(host);
            }
            _ => return Err("browser_policy_operation_invalid".into()),
        }
        let revoked: Vec<String> = self
            .operator_overrides
            .iter()
            .chain(self.archived_operator_overrides.iter())
            .filter(|(_, grant)| match operation {
                "set_app" | "clear_app" => grant.host_id == host,
                _ => grant.task_id == task,
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in revoked {
            self.revoked_override_ids.insert(id.clone());
            if let Some(grant) = self.operator_overrides.remove(&id) {
                self.archived_operator_overrides.insert(id, grant);
            }
        }
        self.advance(task, operation, actor, reason)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn issue_override(
        &mut self,
        expected: u64,
        token: String,
        host: &str,
        task: &str,
        action: &str,
        request_id: &str,
        actor: &str,
        reason: &str,
    ) -> Result<(), String> {
        if self.revision != expected {
            return Err("browser_policy_revision_changed".into());
        }
        attribution(actor, reason)?;
        if request_id.is_empty()
            || request_id.len() > 512
            || request_id.chars().any(char::is_control)
            || !policy_token(action)
            || token.is_empty()
        {
            return Err("browser_policy_override_invalid".into());
        }
        if self.operator_overrides.contains_key(&token)
            || self.archived_operator_overrides.contains_key(&token)
        {
            return Err("browser_policy_override_id_conflict: inspect the original grant; request a new attributable scoped override rather than replacing its identity".into());
        }
        let (capacity, _) = self.override_capacity(host, task);
        capacity.validate()?;
        if self
            .operator_overrides
            .values()
            .filter(|grant| grant.host_id == host && grant.task_id == task)
            .count()
            >= capacity.active_limit
        {
            let ids: Vec<String> = self
                .operator_overrides
                .iter()
                .filter(|(_, grant)| grant.host_id == host && grant.task_id == task)
                .map(|(id, _)| id.clone())
                .collect();
            for id in ids {
                if let Some(grant) = self.operator_overrides.remove(&id) {
                    self.archived_operator_overrides.insert(id, grant);
                }
            }
        }
        self.operator_overrides.insert(
            token,
            BrowserPolicyOverride {
                task_id: task.into(),
                host_id: host.into(),
                policy: self.effective(host, task).cloned().unwrap_or_default(),
                action: canonical_browser_policy_action(action).into(),
                request_id: request_id.into(),
                revision: self.revision,
                actor: actor.into(),
                reason: reason.into(),
            },
        );
        self.audit.push(BrowserPolicyAudit {
            revision: self.revision,
            task_id: task.into(),
            operation: "override".into(),
            actor: actor.into(),
            reason: reason.into(),
        });
        self.trim_audit();
        Ok(())
    }
    pub fn check(
        &self,
        host: &str,
        task: &str,
        action: &str,
        request_id: &str,
        override_id: Option<&str>,
    ) -> Result<(), String> {
        let action = canonical_browser_policy_action(action);
        if policy_status_action(action) {
            return Ok(());
        }
        let denied = self.effective(host, task).is_some_and(|policy| {
            policy
                .denied_actions
                .iter()
                .any(|denied| canonical_browser_policy_action(denied) == action)
        });
        if !denied {
            return Ok(());
        }
        if override_id
            .filter(|id| !self.revoked_override_ids.contains(*id))
            .and_then(|id| {
                self.operator_overrides
                    .get(id)
                    .or_else(|| self.archived_operator_overrides.get(id))
            })
            .is_some_and(|grant| {
                grant.task_id == task
                    && grant.action == action
                    && grant.request_id == request_id
                    && grant.host_id == host
                    && grant.policy == self.effective(host, task).cloned().unwrap_or_default()
            })
        {
            return Ok(());
        }
        Err(format!("browser_policy_action_denied:{action}: change this task's explicit policy or use an attributable scoped override"))
    }
    fn advance(
        &mut self,
        task: &str,
        operation: &str,
        actor: &str,
        reason: &str,
    ) -> Result<(), String> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or("browser_policy_revision_exhausted")?;
        self.audit.push(BrowserPolicyAudit {
            revision: self.revision,
            task_id: task.into(),
            operation: operation.into(),
            actor: actor.into(),
            reason: reason.into(),
        });
        self.trim_audit();
        Ok(())
    }
    fn trim_audit(&mut self) {
        if self.audit.len() > 128 {
            self.audit.drain(..self.audit.len() - 128);
        }
    }
}
fn attribution(actor: &str, reason: &str) -> Result<(), String> {
    if actor.trim().is_empty()
        || reason.trim().is_empty()
        || actor.len() > 160
        || reason.len() > 320
        || actor.chars().any(char::is_control)
        || reason.chars().any(char::is_control)
    {
        return Err("browser_policy_attribution_required".into());
    }
    Ok(())
}
