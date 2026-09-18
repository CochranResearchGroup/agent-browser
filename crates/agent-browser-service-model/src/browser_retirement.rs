//! Provider-free records for retiring inert browser records.

use serde::{Deserialize, Serialize};

pub const BROWSER_RETIREMENT_PLAN_SCHEMA_V1: &str = "agent-browser.browser-retirement-plan.v1";
pub const BROWSER_RETIREMENT_RECEIPT_SCHEMA_V1: &str =
    "agent-browser.browser-retirement-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserRetirementPlan {
    pub schema_version: String,
    pub plan_id: String,
    pub browser_id: String,
    pub record_revision: u64,
    pub evidence_digest: String,
    pub created_at: String,
    pub expires_at: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserRetirementReceipt {
    pub schema_version: String,
    pub plan_id: String,
    pub browser_id: String,
    pub record_revision: u64,
    pub evidence_digest: String,
    pub terminal_result: String,
    pub applied_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContaminationReport {
    pub schema_version: String,
    pub inert_browser_ids: Vec<String>,
    pub review_browser_ids: Vec<String>,
    pub diagnostic_display_allocation_count: usize,
    pub default_effect: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde_json::{json, Value};

    fn assert_round_trip<T>(record: &T)
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let wire = serde_json::to_value(record).unwrap();
        let decoded = serde_json::from_value(wire).unwrap();
        assert_eq!(*record, decoded);
    }

    fn plan() -> BrowserRetirementPlan {
        BrowserRetirementPlan {
            schema_version: BROWSER_RETIREMENT_PLAN_SCHEMA_V1.to_string(),
            plan_id: "browser-retirement-plan:test".to_string(),
            browser_id: "browser:legacy".to_string(),
            record_revision: 11,
            evidence_digest: "sha256:evidence".to_string(),
            created_at: "2026-09-16T12:00:00Z".to_string(),
            expires_at: "2026-09-16T13:00:00Z".to_string(),
            reasons: vec!["inert_record".to_string(), "unreferenced".to_string()],
        }
    }

    fn receipt() -> BrowserRetirementReceipt {
        BrowserRetirementReceipt {
            schema_version: BROWSER_RETIREMENT_RECEIPT_SCHEMA_V1.to_string(),
            plan_id: "browser-retirement-plan:test".to_string(),
            browser_id: "browser:legacy".to_string(),
            record_revision: 11,
            evidence_digest: "sha256:evidence".to_string(),
            terminal_result: "retired".to_string(),
            applied_at: "2026-09-16T12:01:00Z".to_string(),
        }
    }

    fn contamination_report() -> BrowserContaminationReport {
        BrowserContaminationReport {
            schema_version: "agent-browser.browser-contamination-report.v1".to_string(),
            inert_browser_ids: vec!["browser:legacy".to_string()],
            review_browser_ids: vec!["browser:review".to_string()],
            diagnostic_display_allocation_count: 2,
            default_effect: "review".to_string(),
        }
    }

    #[test]
    fn records_round_trip_over_the_wire() {
        assert_round_trip(&plan());
        assert_round_trip(&receipt());
        assert_round_trip(&contamination_report());
    }

    #[test]
    fn records_use_camel_case_wire_fields() {
        let cases = [
            (
                serde_json::to_value(plan()).unwrap(),
                [
                    "schemaVersion",
                    "planId",
                    "browserId",
                    "recordRevision",
                    "evidenceDigest",
                    "createdAt",
                    "expiresAt",
                ]
                .as_slice(),
            ),
            (
                serde_json::to_value(receipt()).unwrap(),
                [
                    "schemaVersion",
                    "planId",
                    "browserId",
                    "recordRevision",
                    "evidenceDigest",
                    "terminalResult",
                    "appliedAt",
                ]
                .as_slice(),
            ),
            (
                serde_json::to_value(contamination_report()).unwrap(),
                [
                    "schemaVersion",
                    "inertBrowserIds",
                    "reviewBrowserIds",
                    "diagnosticDisplayAllocationCount",
                    "defaultEffect",
                ]
                .as_slice(),
            ),
        ];

        for (wire, fields) in cases {
            let object = wire.as_object().unwrap();
            for field in fields {
                assert!(
                    object.contains_key(*field),
                    "missing camelCase field {field}"
                );
            }
        }
    }

    #[test]
    fn deny_unknown_fields_applies_to_plan_and_receipt() {
        let mut plan_wire = serde_json::to_value(plan()).unwrap();
        plan_wire
            .as_object_mut()
            .unwrap()
            .insert("unexpectedField".to_string(), json!(true));
        assert!(serde_json::from_value::<BrowserRetirementPlan>(plan_wire).is_err());

        let mut receipt_wire = serde_json::to_value(receipt()).unwrap();
        receipt_wire
            .as_object_mut()
            .unwrap()
            .insert("unexpectedField".to_string(), json!(true));
        assert!(serde_json::from_value::<BrowserRetirementReceipt>(receipt_wire).is_err());
    }

    #[test]
    fn schema_constants_match_wire_values() {
        assert_eq!(
            BROWSER_RETIREMENT_PLAN_SCHEMA_V1,
            "agent-browser.browser-retirement-plan.v1"
        );
        assert_eq!(
            BROWSER_RETIREMENT_RECEIPT_SCHEMA_V1,
            "agent-browser.browser-retirement-receipt.v1"
        );
        assert_eq!(
            serde_json::to_value(plan()).unwrap()["schemaVersion"],
            Value::String(BROWSER_RETIREMENT_PLAN_SCHEMA_V1.to_string())
        );
        assert_eq!(
            serde_json::to_value(receipt()).unwrap()["schemaVersion"],
            Value::String(BROWSER_RETIREMENT_RECEIPT_SCHEMA_V1.to_string())
        );
    }
}
