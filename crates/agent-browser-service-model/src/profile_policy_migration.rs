//! Passive profile-policy migration records shared by service adapters.

use crate::ProfileAccessMode;
use serde::{Deserialize, Serialize};

pub const PROFILE_POLICY_MIGRATION_SCHEMA_VERSION: &str =
    "agent-browser.profile-policy-migration.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfilePolicyMigrationEntry {
    pub profile_id: String,
    pub classification: String,
    pub target_mode: ProfileAccessMode,
    pub ambiguity: bool,
    pub blocking: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfilePolicyMigrationReport {
    pub schema_version: String,
    pub migration_id: String,
    pub source_revision: u64,
    pub target_revision: u64,
    pub entries: Vec<ProfilePolicyMigrationEntry>,
    pub blocking_issue_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry() -> ProfilePolicyMigrationEntry {
        ProfilePolicyMigrationEntry {
            profile_id: "profile-1".to_string(),
            classification: "proven-strict-compatibility".to_string(),
            target_mode: ProfileAccessMode::Exclusive,
            ambiguity: false,
            blocking: false,
            reason: "A provenance-backed principal held every exclusive legacy session."
                .to_string(),
        }
    }

    fn report() -> ProfilePolicyMigrationReport {
        ProfilePolicyMigrationReport {
            schema_version: PROFILE_POLICY_MIGRATION_SCHEMA_VERSION.to_string(),
            migration_id: "profile-policy-migration-1".to_string(),
            source_revision: 3,
            target_revision: 4,
            entries: vec![entry()],
            blocking_issue_count: 0,
        }
    }

    #[test]
    fn migration_report_round_trips() {
        let original = report();
        let wire = serde_json::to_string(&original).unwrap();
        let decoded: ProfilePolicyMigrationReport = serde_json::from_str(&wire).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn migration_report_uses_camel_case_wire_fields() {
        let wire = serde_json::to_value(report()).unwrap();

        assert!(wire.get("schemaVersion").is_some());
        assert!(wire.get("sourceRevision").is_some());
        assert!(wire.get("targetRevision").is_some());
        assert!(wire.get("blockingIssueCount").is_some());
        assert!(wire["entries"][0].get("profileId").is_some());
        assert!(wire["entries"][0].get("targetMode").is_some());
        assert!(wire["entries"][0].get("profile_id").is_none());
    }

    #[test]
    fn migration_records_reject_unknown_fields() {
        let mut wire = serde_json::to_value(report()).unwrap();
        wire["unexpectedField"] = json!(true);

        let error = serde_json::from_value::<ProfilePolicyMigrationReport>(wire).unwrap_err();

        assert!(error
            .to_string()
            .contains("unknown field `unexpectedField`"));

        let mut wire = serde_json::to_value(report()).unwrap();
        wire["entries"][0]["unexpectedField"] = json!(true);

        let error = serde_json::from_value::<ProfilePolicyMigrationReport>(wire).unwrap_err();

        assert!(error
            .to_string()
            .contains("unknown field `unexpectedField`"));
    }

    #[test]
    fn migration_entry_uses_profile_access_mode_wire_value() {
        let wire = serde_json::to_value(entry()).unwrap();

        assert_eq!(wire["targetMode"], "exclusive");
    }

    #[test]
    fn migration_schema_version_is_stable() {
        assert_eq!(
            PROFILE_POLICY_MIGRATION_SCHEMA_VERSION,
            "agent-browser.profile-policy-migration.v1"
        );
    }
}
