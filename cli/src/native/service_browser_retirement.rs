//! Exact, receipt-backed retirement of inert legacy browser records.
//!
//! Retirement is distinct from browser close. It accepts only a PID-less,
//! process-unproven, managed-runtime-unproven, and session-unreferenced record.
//! Planning is read-only and application compare-and-swaps the exact record
//! revision and evidence digest before removing that one browser row.

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::service_model::{BrowserProcess, ServiceState};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use super::service_trace::service_commands::service_now_timestamp;
use serde_json::{json, Value};

pub(crate) const BROWSER_RETIREMENT_PLAN_SCHEMA_V1: &str =
    "agent-browser.browser-retirement-plan.v1";
pub(crate) const BROWSER_RETIREMENT_RECEIPT_SCHEMA_V1: &str =
    "agent-browser.browser-retirement-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BrowserRetirementPlan {
    pub(crate) schema_version: String,
    pub(crate) plan_id: String,
    pub(crate) browser_id: String,
    pub(crate) record_revision: u64,
    pub(crate) evidence_digest: String,
    pub(crate) created_at: String,
    pub(crate) expires_at: String,
    pub(crate) reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BrowserRetirementReceipt {
    pub(crate) schema_version: String,
    pub(crate) plan_id: String,
    pub(crate) browser_id: String,
    pub(crate) record_revision: u64,
    pub(crate) evidence_digest: String,
    pub(crate) terminal_result: String,
    pub(crate) applied_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserContaminationReport {
    pub(crate) schema_version: String,
    pub(crate) inert_browser_ids: Vec<String>,
    pub(crate) review_browser_ids: Vec<String>,
    pub(crate) diagnostic_display_allocation_count: usize,
    pub(crate) default_effect: String,
}

pub(crate) fn handle_service_browser_retirement_command(command: &Value) -> Result<Value, String> {
    let action = command
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| "browser_retirement_action_required".to_string())?;
    let repository = LockedServiceStateRepository::default_json()?;
    match action {
        "service_browser_contamination_report" => Ok(json!({
            "report": detect_browser_contamination(&repository.load_snapshot()?),
        })),
        "service_browser_retirement_plan" => {
            let browser_id = required_string(command, "browserId")?;
            let created_at = service_now_timestamp();
            let expires_at = required_string(command, "expiresAt")?;
            Ok(json!({
                "plan": plan_browser_retirement(
                    &repository.load_snapshot()?,
                    browser_id,
                    &created_at,
                    expires_at,
                )?,
            }))
        }
        "service_browser_retirement_apply" => {
            let plan = command
                .get("plan")
                .cloned()
                .ok_or_else(|| "browser_retirement_plan_required".to_string())?;
            let plan: BrowserRetirementPlan = serde_json::from_value(plan)
                .map_err(|error| format!("browser_retirement_plan_invalid:{error}"))?;
            let (receipt, replayed) =
                apply_browser_retirement(&repository, &plan, &service_now_timestamp())?;
            Ok(json!({ "receipt": receipt, "replayed": replayed }))
        }
        _ => Err(format!("Unsupported browser retirement action: {action}")),
    }
}

pub(crate) fn plan_browser_retirement(
    state: &ServiceState,
    browser_id: &str,
    created_at: &str,
    expires_at: &str,
) -> Result<BrowserRetirementPlan, String> {
    let browser = state
        .browsers
        .get(browser_id)
        .ok_or_else(|| "browser_retirement_browser_missing".to_string())?;
    require_inert_browser(state, browser)?;
    let record_revision = browser
        .record_provenance
        .as_ref()
        .map(|provenance| provenance.record_revision)
        .unwrap_or(0);
    let evidence_digest = browser_evidence_digest(browser)?;
    let plan_id = digest_json(&(
        BROWSER_RETIREMENT_PLAN_SCHEMA_V1,
        browser_id,
        record_revision,
        &evidence_digest,
        created_at,
        expires_at,
    ))?;
    Ok(BrowserRetirementPlan {
        schema_version: BROWSER_RETIREMENT_PLAN_SCHEMA_V1.to_string(),
        plan_id,
        browser_id: browser_id.to_string(),
        record_revision,
        evidence_digest,
        created_at: created_at.to_string(),
        expires_at: expires_at.to_string(),
        reasons: vec![
            "pid_absent".to_string(),
            "process_identity_absent".to_string(),
            "managed_runtime_evidence_absent".to_string(),
            "session_reference_absent".to_string(),
        ],
    })
}

pub(crate) fn apply_browser_retirement(
    repository: &impl ServiceStateRepository,
    plan: &BrowserRetirementPlan,
    now: &str,
) -> Result<(BrowserRetirementReceipt, bool), String> {
    if plan.schema_version != BROWSER_RETIREMENT_PLAN_SCHEMA_V1 {
        return Err("browser_retirement_plan_schema_unsupported".to_string());
    }
    let now_parsed = DateTime::parse_from_rfc3339(now)
        .map_err(|_| "browser_retirement_now_invalid".to_string())?;
    let expires_at = DateTime::parse_from_rfc3339(&plan.expires_at)
        .map_err(|_| "browser_retirement_expiry_invalid".to_string())?;
    if now_parsed >= expires_at {
        return Err("browser_retirement_plan_expired".to_string());
    }
    repository.mutate(|state| {
        if let Some(receipt) = state.browser_retirement_receipts.get(&plan.plan_id) {
            return Ok((receipt.clone(), true));
        }
        let current = state
            .browsers
            .get(&plan.browser_id)
            .ok_or_else(|| "browser_retirement_plan_stale".to_string())?;
        require_inert_browser(state, current)?;
        let current_revision = current
            .record_provenance
            .as_ref()
            .map(|provenance| provenance.record_revision)
            .unwrap_or(0);
        if current_revision != plan.record_revision
            || browser_evidence_digest(current)? != plan.evidence_digest
        {
            return Err("browser_retirement_plan_stale".to_string());
        }
        state.browsers.remove(&plan.browser_id);
        let receipt = BrowserRetirementReceipt {
            schema_version: BROWSER_RETIREMENT_RECEIPT_SCHEMA_V1.to_string(),
            plan_id: plan.plan_id.clone(),
            browser_id: plan.browser_id.clone(),
            record_revision: plan.record_revision,
            evidence_digest: plan.evidence_digest.clone(),
            terminal_result: "retired".to_string(),
            applied_at: now.to_string(),
        };
        state
            .browser_retirement_receipts
            .insert(plan.plan_id.clone(), receipt.clone());
        Ok((receipt, false))
    })
}

pub(crate) fn detect_browser_contamination(state: &ServiceState) -> BrowserContaminationReport {
    let mut inert_browser_ids = Vec::new();
    let mut review_browser_ids = Vec::new();
    for (browser_id, browser) in &state.browsers {
        if require_inert_browser(state, browser).is_ok() {
            inert_browser_ids.push(browser_id.clone());
        } else if browser.pid.is_none()
            && !state.browser_process_identities.contains_key(browser_id)
        {
            review_browser_ids.push(browser_id.clone());
        }
    }
    BrowserContaminationReport {
        schema_version: "agent-browser.browser-contamination-report.v1".to_string(),
        inert_browser_ids,
        review_browser_ids,
        diagnostic_display_allocation_count: state.display_allocations.len(),
        default_effect: "none".to_string(),
    }
}

fn require_inert_browser(state: &ServiceState, browser: &BrowserProcess) -> Result<(), String> {
    if browser.pid.is_some()
        || state.browser_process_identities.contains_key(&browser.id)
        || state
            .runtime_owner_registry
            .lifecycle_records
            .contains_key(&browser.id)
    {
        return Err("browser_retirement_live_authority_present".to_string());
    }
    let referenced = !browser.active_session_ids.is_empty()
        || state
            .sessions
            .values()
            .any(|session| session.browser_ids.iter().any(|id| id == &browser.id))
        || browser.display_allocation_id.is_some()
        || !browser.view_streams.is_empty();
    if referenced {
        return Err("browser_retirement_record_referenced".to_string());
    }
    Ok(())
}

fn browser_evidence_digest(browser: &BrowserProcess) -> Result<String, String> {
    let mut evidence = browser.clone();
    evidence.record_provenance = None;
    digest_json(&evidence)
}

fn digest_json(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| format!("browser_retirement_encode_failed:{error}"))
}

fn required_string<'a>(command: &'a Value, field: &str) -> Result<&'a str, String> {
    command
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("browser_retirement_{field}_required"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct MemoryRepository(Arc<Mutex<ServiceState>>);

    impl ServiceStateRepository for MemoryRepository {
        fn load_snapshot(&self) -> Result<ServiceState, String> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn mutate<R>(
            &self,
            mutator: impl FnOnce(&mut ServiceState) -> Result<R, String>,
        ) -> Result<R, String> {
            mutator(&mut self.0.lock().unwrap())
        }
    }

    fn contaminated_state() -> ServiceState {
        ServiceState {
            browsers: BTreeMap::from([
                (
                    "browser-cdp".to_string(),
                    BrowserProcess {
                        id: "browser-cdp".to_string(),
                        ..BrowserProcess::default()
                    },
                ),
                (
                    "session:odollo-carrier-ups".to_string(),
                    BrowserProcess {
                        id: "session:odollo-carrier-ups".to_string(),
                        ..BrowserProcess::default()
                    },
                ),
            ]),
            ..ServiceState::default()
        }
    }

    #[test]
    fn detector_classifies_fixture_shaped_rows_without_touching_displays() {
        let mut state = contaminated_state();
        state.display_allocations = (0..36)
            .map(|index| {
                let id = format!("display-{index}");
                (
                    id.clone(),
                    super::super::service_model::DisplayAllocation {
                        id,
                        ..Default::default()
                    },
                )
            })
            .collect();
        let report = detect_browser_contamination(&state);
        assert_eq!(
            report.inert_browser_ids,
            vec![
                "browser-cdp".to_string(),
                "session:odollo-carrier-ups".to_string()
            ]
        );
        assert_eq!(report.diagnostic_display_allocation_count, 36);
        assert_eq!(report.default_effect, "none");
        assert_eq!(state.display_allocations.len(), 36);
    }

    #[test]
    fn exact_retirement_persists_receipt_and_replays_without_broad_cleanup() {
        let repository = MemoryRepository(Arc::new(Mutex::new(contaminated_state())));
        let plan = plan_browser_retirement(
            &repository.load_snapshot().unwrap(),
            "browser-cdp",
            "2026-08-28T13:00:00Z",
            "2026-08-28T13:05:00Z",
        )
        .unwrap();
        let (receipt, replayed) =
            apply_browser_retirement(&repository, &plan, "2026-08-28T13:01:00Z").unwrap();
        assert!(!replayed);
        assert_eq!(receipt.terminal_result, "retired");
        let state = repository.load_snapshot().unwrap();
        assert!(!state.browsers.contains_key("browser-cdp"));
        assert!(state.browsers.contains_key("session:odollo-carrier-ups"));

        let (replayed_receipt, replayed) =
            apply_browser_retirement(&repository, &plan, "2026-08-28T13:02:00Z").unwrap();
        assert!(replayed);
        assert_eq!(replayed_receipt, receipt);
    }

    #[test]
    fn retirement_fails_closed_when_process_or_reference_appears() {
        let mut state = contaminated_state();
        let plan = plan_browser_retirement(
            &state,
            "browser-cdp",
            "2026-08-28T13:00:00Z",
            "2026-08-28T13:05:00Z",
        )
        .unwrap();
        state.browsers.get_mut("browser-cdp").unwrap().pid = Some(7137);
        let repository = MemoryRepository(Arc::new(Mutex::new(state)));
        assert_eq!(
            apply_browser_retirement(&repository, &plan, "2026-08-28T13:01:00Z").unwrap_err(),
            "browser_retirement_live_authority_present"
        );
    }
}
