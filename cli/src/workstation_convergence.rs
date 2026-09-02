//! Canonical workstation convergence planning and health projection.
//!
//! This module owns the join from desired installation state and normalized
//! runtime observations to one sealed plan and final receipt. Profile access
//! findings and request-scoped acquisition findings are projected on separate
//! axes and cannot alter runtime or installation convergence readiness.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const WORKSTATION_CONVERGENCE_DESIRED_SCHEMA: &str =
    "agent-browser.workstation-convergence-desired.v1";
const WORKSTATION_CONVERGENCE_OBSERVED_SCHEMA: &str =
    "agent-browser.workstation-convergence-observed.v1";
const WORKSTATION_CONVERGENCE_PLAN_SCHEMA: &str = "agent-browser.workstation-convergence-plan.v1";
const WORKSTATION_CONVERGENCE_RECEIPT_SCHEMA: &str =
    "agent-browser.workstation-convergence-receipt.v1";
const DASHBOARD_HEALTH_SCHEMA: &str = "agent-browser.dashboard-health.v1";
const PRIVILEGED_EFFECT_PLAN_SCHEMA: &str = "agent-browser.privileged-host-effect-plan.v1";
const PRIVILEGED_EFFECT_RECEIPT_SCHEMA: &str = "agent-browser.privileged-host-effect-receipt.v1";
const PRIVILEGED_EFFECT_RECEIPT_PREFIX: &str = "AGENT_BROWSER_PRIVILEGED_EFFECT_RECEIPT=";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkstationConvergenceDesiredState {
    schema_version: String,
    selected_generation_id: Option<String>,
    require_runtime_host: bool,
    require_runtime_monitor: bool,
    require_dashboard_ingress: bool,
    require_operator_journey: bool,
}

impl WorkstationConvergenceDesiredState {
    fn installed(selected_generation_id: Option<String>) -> Self {
        Self {
            schema_version: WORKSTATION_CONVERGENCE_DESIRED_SCHEMA.to_string(),
            selected_generation_id,
            require_runtime_host: true,
            require_runtime_monitor: true,
            require_dashboard_ingress: true,
            require_operator_journey: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkstationHealthFinding {
    pub(crate) code: String,
    pub(crate) blocking: bool,
    pub(crate) message: String,
}

impl WorkstationHealthFinding {
    fn blocking(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            blocking: true,
            message: message.to_string(),
        }
    }

    #[cfg(test)]
    fn advisory(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            blocking: false,
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkstationConvergenceObservedState {
    schema_version: String,
    runtime_inventory_ready: bool,
    supervisor_ready: bool,
    runtime_multiplicity_ready: bool,
    runtime_monitor_ready: bool,
    selected_generation_ready: bool,
    transaction_terminal: bool,
    dashboard_ingress_ready: bool,
    operator_journey_ready: bool,
    access_findings: Vec<WorkstationHealthFinding>,
    acquisition_findings: Vec<WorkstationHealthFinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WorkstationConvergenceEffect {
    ReconcileRuntimeInventory,
    RestartRuntimeSupervisor,
    ReconcileRuntimeMultiplicity,
    RunRuntimeMonitor,
    ResumeWorkstationTransaction,
    RepairDashboardIngress,
    ReproveOperatorJourney,
}

impl WorkstationConvergenceEffect {
    fn executable_action(self) -> &'static str {
        match self {
            Self::ReconcileRuntimeInventory => "reconcile_runtime_inventory",
            Self::RestartRuntimeSupervisor => "restart_runtime_supervisor",
            Self::ReconcileRuntimeMultiplicity => "reconcile_runtime_multiplicity",
            Self::RunRuntimeMonitor => "run_runtime_reconciliation",
            Self::ResumeWorkstationTransaction => "resume_workstation_transaction",
            Self::RepairDashboardIngress => "repair_dashboard_ingress",
            Self::ReproveOperatorJourney => "reprove_operator_journey",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkstationConvergencePlanMaterial<'a> {
    desired: &'a WorkstationConvergenceDesiredState,
    observed: &'a WorkstationConvergenceObservedState,
    required_effects: &'a [WorkstationConvergenceEffect],
    runtime_findings: &'a [WorkstationHealthFinding],
    convergence_findings: &'a [WorkstationHealthFinding],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkstationConvergencePlan {
    schema_version: String,
    pub(crate) plan_digest: String,
    desired: WorkstationConvergenceDesiredState,
    observed: WorkstationConvergenceObservedState,
    required_effects: Vec<WorkstationConvergenceEffect>,
    runtime_findings: Vec<WorkstationHealthFinding>,
    convergence_findings: Vec<WorkstationHealthFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkstationRuntimeHealthAxis {
    pub(crate) state: String,
    pub(crate) ready: bool,
    pub(crate) findings: Vec<WorkstationHealthFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkstationAdvisoryHealthAxis {
    pub(crate) state: String,
    pub(crate) findings: Vec<WorkstationHealthFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkstationAcquisitionHealthAxis {
    pub(crate) state: String,
    pub(crate) request_scoped: bool,
    pub(crate) findings: Vec<WorkstationHealthFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkstationDashboardHealth {
    pub(crate) schema_version: String,
    pub(crate) runtime: WorkstationRuntimeHealthAxis,
    pub(crate) convergence: WorkstationRuntimeHealthAxis,
    pub(crate) access: WorkstationAdvisoryHealthAxis,
    pub(crate) acquisition: WorkstationAcquisitionHealthAxis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkstationConvergenceReceipt {
    pub(crate) schema_version: String,
    pub(crate) receipt_id: String,
    pub(crate) plan_digest: String,
    pub(crate) ready: bool,
    pub(crate) state: String,
    pub(crate) executable_next_action: Option<String>,
    pub(crate) dashboard_health: WorkstationDashboardHealth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrivilegedHostEffect {
    LeaseAuthority,
    PrivilegedHelper,
    WorkstationDependencies,
}

impl PrivilegedHostEffect {
    fn as_str(self) -> &'static str {
        match self {
            Self::LeaseAuthority => "ensure_lease_authority",
            Self::PrivilegedHelper => "ensure_privileged_helper",
            Self::WorkstationDependencies => "ensure_workstation_dependencies",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrivilegedHostEffectPlanMaterial<'a> {
    schema_version: &'a str,
    actions: &'a [PrivilegedHostEffect],
    helper_sha256: &'a str,
    lease_authority_sha256: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PrivilegedHostEffectPlan {
    pub(crate) schema_version: String,
    pub(crate) plan_digest: String,
    actions: Vec<PrivilegedHostEffect>,
    helper_sha256: String,
    lease_authority_sha256: String,
}

impl PrivilegedHostEffectPlan {
    pub(crate) fn seal(
        with_workstation_dependencies: bool,
        helper_sha256: &str,
        lease_authority_sha256: &str,
    ) -> Result<Self, String> {
        if !is_sha256(helper_sha256) || !is_sha256(lease_authority_sha256) {
            return Err("privileged_host_effect_plan_identity_invalid".to_string());
        }
        let mut actions = vec![
            PrivilegedHostEffect::LeaseAuthority,
            PrivilegedHostEffect::PrivilegedHelper,
        ];
        if with_workstation_dependencies {
            actions.push(PrivilegedHostEffect::WorkstationDependencies);
        }
        let material = PrivilegedHostEffectPlanMaterial {
            schema_version: PRIVILEGED_EFFECT_PLAN_SCHEMA,
            actions: &actions,
            helper_sha256,
            lease_authority_sha256,
        };
        Ok(Self {
            schema_version: PRIVILEGED_EFFECT_PLAN_SCHEMA.to_string(),
            plan_digest: digest_serializable(&material),
            actions,
            helper_sha256: helper_sha256.to_string(),
            lease_authority_sha256: lease_authority_sha256.to_string(),
        })
    }

    pub(crate) fn action_csv(&self) -> String {
        self.actions
            .iter()
            .copied()
            .map(PrivilegedHostEffect::as_str)
            .collect::<Vec<_>>()
            .join(",")
    }

    fn validate(&self) -> Result<(), String> {
        let material = PrivilegedHostEffectPlanMaterial {
            schema_version: &self.schema_version,
            actions: &self.actions,
            helper_sha256: &self.helper_sha256,
            lease_authority_sha256: &self.lease_authority_sha256,
        };
        if self.schema_version != PRIVILEGED_EFFECT_PLAN_SCHEMA
            || !is_sha256(&self.helper_sha256)
            || !is_sha256(&self.lease_authority_sha256)
            || digest_serializable(&material) != self.plan_digest
        {
            return Err("privileged_host_effect_plan_digest_mismatch".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PrivilegedHostEffectReceipt {
    pub(crate) schema_version: String,
    pub(crate) plan_digest: String,
    pub(crate) actions: String,
    pub(crate) outcome: String,
    pub(crate) helper_ready: bool,
    pub(crate) lease_authority_ready: bool,
    pub(crate) workstation_dependencies_ready: bool,
}

pub(crate) fn validate_privileged_effect_adapter_receipt(
    plan: &PrivilegedHostEffectPlan,
    stdout: &str,
) -> Result<PrivilegedHostEffectReceipt, String> {
    plan.validate()?;
    let encoded = stdout
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(PRIVILEGED_EFFECT_RECEIPT_PREFIX))
        .ok_or_else(|| "privileged_host_effect_receipt_missing".to_string())?;
    let receipt: PrivilegedHostEffectReceipt = serde_json::from_str(encoded)
        .map_err(|_| "privileged_host_effect_receipt_invalid".to_string())?;
    if receipt.schema_version != PRIVILEGED_EFFECT_RECEIPT_SCHEMA
        || receipt.plan_digest != plan.plan_digest
        || receipt.actions != plan.action_csv()
        || !receipt.helper_ready
        || !receipt.lease_authority_ready
        || (plan
            .actions
            .contains(&PrivilegedHostEffect::WorkstationDependencies)
            && !receipt.workstation_dependencies_ready)
        || !matches!(
            receipt.outcome.as_str(),
            "already_ready" | "effects_applied"
        )
    {
        return Err("privileged_host_effect_receipt_mismatch".to_string());
    }
    Ok(receipt)
}

pub(crate) struct WorkstationConvergenceOwner {
    desired: WorkstationConvergenceDesiredState,
}

impl WorkstationConvergenceOwner {
    pub(crate) fn installed(selected_generation_id: Option<String>) -> Self {
        Self {
            desired: WorkstationConvergenceDesiredState::installed(selected_generation_id),
        }
    }

    pub(crate) fn plan(
        &self,
        observed: WorkstationConvergenceObservedState,
    ) -> WorkstationConvergencePlan {
        let mut required_effects = Vec::new();
        let mut runtime_findings = Vec::new();
        let mut convergence_findings = Vec::new();

        if !observed.runtime_inventory_ready {
            required_effects.push(WorkstationConvergenceEffect::ReconcileRuntimeInventory);
            runtime_findings.push(WorkstationHealthFinding::blocking(
                "runtime_inventory_not_ready",
                "The selected runtime has stale or incomplete executable evidence.",
            ));
        }
        if !observed.supervisor_ready {
            required_effects.push(WorkstationConvergenceEffect::RestartRuntimeSupervisor);
            runtime_findings.push(WorkstationHealthFinding::blocking(
                "runtime_supervisor_not_ready",
                "The selected runtime supervisor is not active and ready.",
            ));
        }
        if !observed.runtime_multiplicity_ready {
            required_effects.push(WorkstationConvergenceEffect::ReconcileRuntimeMultiplicity);
            runtime_findings.push(WorkstationHealthFinding::blocking(
                "runtime_multiplicity_not_ready",
                "Runtime process multiplicity is outside the selected steady state.",
            ));
        }
        if self.desired.require_runtime_monitor && !observed.runtime_monitor_ready {
            required_effects.push(WorkstationConvergenceEffect::RunRuntimeMonitor);
            runtime_findings.push(WorkstationHealthFinding::blocking(
                "runtime_monitor_not_ready",
                "The runtime reconciliation monitor has no current healthy receipt.",
            ));
        }
        if !observed.selected_generation_ready || !observed.transaction_terminal {
            required_effects.push(WorkstationConvergenceEffect::ResumeWorkstationTransaction);
            convergence_findings.push(WorkstationHealthFinding::blocking(
                "workstation_transaction_not_converged",
                "The selected generation and workstation transaction have not converged.",
            ));
        }
        if self.desired.require_dashboard_ingress && !observed.dashboard_ingress_ready {
            required_effects.push(WorkstationConvergenceEffect::RepairDashboardIngress);
            convergence_findings.push(WorkstationHealthFinding::blocking(
                "dashboard_ingress_not_ready",
                "The stable dashboard ingress is not bound to a ready generation.",
            ));
        }
        if self.desired.require_operator_journey && !observed.operator_journey_ready {
            required_effects.push(WorkstationConvergenceEffect::ReproveOperatorJourney);
            convergence_findings.push(WorkstationHealthFinding::blocking(
                "operator_journey_not_ready",
                "The selected dashboard generation lacks a current operator-journey receipt.",
            ));
        }

        let material = WorkstationConvergencePlanMaterial {
            desired: &self.desired,
            observed: &observed,
            required_effects: &required_effects,
            runtime_findings: &runtime_findings,
            convergence_findings: &convergence_findings,
        };
        let plan_digest = digest_serializable(&material);
        WorkstationConvergencePlan {
            schema_version: WORKSTATION_CONVERGENCE_PLAN_SCHEMA.to_string(),
            plan_digest,
            desired: self.desired.clone(),
            observed,
            required_effects,
            runtime_findings,
            convergence_findings,
        }
    }

    pub(crate) fn settle(
        &self,
        plan: WorkstationConvergencePlan,
    ) -> Result<WorkstationConvergenceReceipt, String> {
        if plan.schema_version != WORKSTATION_CONVERGENCE_PLAN_SCHEMA
            || plan.desired != self.desired
        {
            return Err("workstation_convergence_plan_owner_mismatch".to_string());
        }
        let material = WorkstationConvergencePlanMaterial {
            desired: &plan.desired,
            observed: &plan.observed,
            required_effects: &plan.required_effects,
            runtime_findings: &plan.runtime_findings,
            convergence_findings: &plan.convergence_findings,
        };
        if digest_serializable(&material) != plan.plan_digest {
            return Err("workstation_convergence_plan_digest_mismatch".to_string());
        }
        let runtime_ready = plan.runtime_findings.is_empty();
        let convergence_ready = runtime_ready && plan.convergence_findings.is_empty();
        let access_state = if plan.observed.access_findings.is_empty() {
            "allowed"
        } else if plan
            .observed
            .access_findings
            .iter()
            .any(|finding| finding.blocking)
        {
            "denied"
        } else {
            "attention"
        };
        let acquisition_state = if plan.observed.acquisition_findings.is_empty() {
            "available"
        } else if plan
            .observed
            .acquisition_findings
            .iter()
            .any(|finding| finding.blocking)
        {
            "denied"
        } else {
            "waiting"
        };
        let executable_next_action = plan
            .required_effects
            .first()
            .copied()
            .map(WorkstationConvergenceEffect::executable_action)
            .map(str::to_string);
        let receipt_id = format!(
            "workstation-convergence-receipt:{}",
            &plan.plan_digest[..24]
        );
        Ok(WorkstationConvergenceReceipt {
            schema_version: WORKSTATION_CONVERGENCE_RECEIPT_SCHEMA.to_string(),
            receipt_id,
            plan_digest: plan.plan_digest,
            ready: convergence_ready,
            state: if convergence_ready {
                "converged".to_string()
            } else {
                "action_required".to_string()
            },
            executable_next_action,
            dashboard_health: WorkstationDashboardHealth {
                schema_version: DASHBOARD_HEALTH_SCHEMA.to_string(),
                runtime: WorkstationRuntimeHealthAxis {
                    state: if runtime_ready { "ready" } else { "degraded" }.to_string(),
                    ready: runtime_ready,
                    findings: plan.runtime_findings,
                },
                convergence: WorkstationRuntimeHealthAxis {
                    state: if convergence_ready {
                        "ready"
                    } else {
                        "blocked"
                    }
                    .to_string(),
                    ready: convergence_ready,
                    findings: plan.convergence_findings,
                },
                access: WorkstationAdvisoryHealthAxis {
                    state: access_state.to_string(),
                    findings: plan.observed.access_findings,
                },
                acquisition: WorkstationAcquisitionHealthAxis {
                    state: acquisition_state.to_string(),
                    request_scoped: true,
                    findings: plan.observed.acquisition_findings,
                },
            },
        })
    }
}

fn digest_serializable(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn bool_at(value: &Value, pointer: &str) -> bool {
    value.pointer(pointer).and_then(Value::as_bool) == Some(true)
}

fn terminal_upgrade_state(value: &Value) -> bool {
    matches!(
        value
            .pointer("/latestTransaction/state")
            .and_then(Value::as_str),
        None | Some("accepted") | Some("old_generation_retirable")
    )
}

pub(crate) fn observe_installed_workstation(
    runtime_inventory: &Value,
    session_supervisors: &Value,
    runtime_multiplicity: &Value,
    runtime_monitor: &Value,
    workstation_upgrade: &Value,
    dashboard_ingress: &Value,
) -> WorkstationConvergenceObservedState {
    WorkstationConvergenceObservedState {
        schema_version: WORKSTATION_CONVERGENCE_OBSERVED_SCHEMA.to_string(),
        runtime_inventory_ready: runtime_inventory
            .get("staleCount")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0,
        supervisor_ready: bool_at(session_supervisors, "/ready"),
        runtime_multiplicity_ready: bool_at(runtime_multiplicity, "/steadyState")
            && runtime_multiplicity
                .pointer("/counts/runtimeHosts")
                .and_then(Value::as_u64)
                == Some(1)
            && runtime_multiplicity
                .pointer("/counts/legacyDaemons")
                .and_then(Value::as_u64)
                == Some(0),
        runtime_monitor_ready: bool_at(runtime_monitor, "/ready"),
        selected_generation_ready: bool_at(
            workstation_upgrade,
            "/readiness/selectedGenerationReady",
        ),
        transaction_terminal: terminal_upgrade_state(workstation_upgrade),
        dashboard_ingress_ready: bool_at(dashboard_ingress, "/dashboardIngressReady"),
        operator_journey_ready: bool_at(dashboard_ingress, "/operatorJourneyReady"),
        access_findings: Vec::new(),
        acquisition_findings: Vec::new(),
    }
}

pub(crate) fn convergence_receipt_from_runtime_health(
    runtime_inventory: &Value,
    session_supervisors: &Value,
    runtime_multiplicity: &Value,
    runtime_monitor: &Value,
    workstation_upgrade: &Value,
    dashboard_ingress: &Value,
) -> Result<WorkstationConvergenceReceipt, String> {
    let owner = WorkstationConvergenceOwner::installed(
        workstation_upgrade
            .get("selectedGenerationId")
            .and_then(Value::as_str)
            .map(str::to_string),
    );
    let observed = observe_installed_workstation(
        runtime_inventory,
        session_supervisors,
        runtime_multiplicity,
        runtime_monitor,
        workstation_upgrade,
        dashboard_ingress,
    );
    let plan = owner.plan(observed);
    owner.settle(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ready_observation() -> WorkstationConvergenceObservedState {
        WorkstationConvergenceObservedState {
            schema_version: WORKSTATION_CONVERGENCE_OBSERVED_SCHEMA.to_string(),
            runtime_inventory_ready: true,
            supervisor_ready: true,
            runtime_multiplicity_ready: true,
            runtime_monitor_ready: true,
            selected_generation_ready: true,
            transaction_terminal: true,
            dashboard_ingress_ready: true,
            operator_journey_ready: true,
            access_findings: Vec::new(),
            acquisition_findings: Vec::new(),
        }
    }

    #[test]
    fn access_ambiguity_cannot_change_runtime_or_convergence_readiness() {
        let owner = WorkstationConvergenceOwner::installed(Some("generation-a".to_string()));
        let mut observed = ready_observation();
        observed
            .access_findings
            .push(WorkstationHealthFinding::advisory(
                "legacy_profile_identity_ambiguous",
                "Historical profile ownership could not be proven.",
            ));

        let receipt = owner.settle(owner.plan(observed)).unwrap();

        assert!(receipt.ready);
        assert!(receipt.dashboard_health.runtime.ready);
        assert!(receipt.dashboard_health.convergence.ready);
        assert_eq!(receipt.dashboard_health.access.state, "attention");
        assert!(!receipt.dashboard_health.access.findings[0].blocking);
        assert_eq!(receipt.dashboard_health.acquisition.state, "available");
        assert!(receipt.dashboard_health.acquisition.request_scoped);
    }

    #[test]
    fn sealed_plan_selects_one_executable_convergence_action() {
        let owner = WorkstationConvergenceOwner::installed(Some("generation-a".to_string()));
        let mut observed = ready_observation();
        observed.supervisor_ready = false;
        observed.dashboard_ingress_ready = false;
        let mut plan = owner.plan(observed);
        let original_digest = plan.plan_digest.clone();

        let receipt = owner.settle(plan.clone()).unwrap();
        assert!(!receipt.ready);
        assert_eq!(
            receipt.executable_next_action.as_deref(),
            Some("restart_runtime_supervisor")
        );
        assert_eq!(receipt.plan_digest, original_digest);

        plan.required_effects.clear();
        assert_eq!(
            owner.settle(plan).unwrap_err(),
            "workstation_convergence_plan_digest_mismatch"
        );
    }

    #[test]
    fn current_single_host_runtime_observation_converges_without_default_socket() {
        let inventory = json!({"staleCount": 0});
        let supervisors = json!({"ready": true});
        let multiplicity = json!({
            "steadyState": true,
            "counts": {"runtimeHosts": 1, "legacyDaemons": 0}
        });
        let monitor = json!({"ready": true});
        let upgrade = json!({
            "selectedGenerationId": "generation-a",
            "readiness": {"selectedGenerationReady": true},
            "latestTransaction": {"state": "accepted"}
        });
        let ingress = json!({"dashboardIngressReady": true, "operatorJourneyReady": true});

        let receipt = convergence_receipt_from_runtime_health(
            &inventory,
            &supervisors,
            &multiplicity,
            &monitor,
            &upgrade,
            &ingress,
        )
        .unwrap();

        assert!(receipt.ready);
        assert_eq!(receipt.state, "converged");
        assert_eq!(receipt.dashboard_health.runtime.state, "ready");
    }

    #[test]
    fn privileged_effect_adapter_is_bound_to_the_exact_rust_plan() {
        let plan = PrivilegedHostEffectPlan::seal(true, &"a".repeat(64), &"b".repeat(64)).unwrap();
        let stdout = format!(
            "installer output\n{PRIVILEGED_EFFECT_RECEIPT_PREFIX}{{\"schemaVersion\":\"{PRIVILEGED_EFFECT_RECEIPT_SCHEMA}\",\"planDigest\":\"{}\",\"actions\":\"{}\",\"outcome\":\"effects_applied\",\"helperReady\":true,\"leaseAuthorityReady\":true,\"workstationDependenciesReady\":true}}\n",
            plan.plan_digest,
            plan.action_csv(),
        );

        let receipt = validate_privileged_effect_adapter_receipt(&plan, &stdout).unwrap();
        assert_eq!(receipt.plan_digest, plan.plan_digest);
        assert_eq!(receipt.actions, plan.action_csv());

        let mismatched = stdout.replace(&plan.plan_digest, &"c".repeat(64));
        assert_eq!(
            validate_privileged_effect_adapter_receipt(&plan, &mismatched).unwrap_err(),
            "privileged_host_effect_receipt_mismatch"
        );
    }
}
