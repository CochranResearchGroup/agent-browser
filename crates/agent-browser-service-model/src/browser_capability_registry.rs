//! Passive, advisory browser capability registry contracts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Draft browser capability registry carried by service config and service state.
///
/// The nested record arrays intentionally stay JSON-shaped while the registry
/// contract is still evolving. Runtime routing must not depend on these records
/// until the registry graduates from advisory state to authoritative policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserCapabilityRegistry {
    pub browser_hosts: Vec<Value>,
    pub browser_executables: Vec<Value>,
    pub browser_capabilities: Vec<Value>,
    pub profile_compatibility: Vec<Value>,
    pub browser_preference_bindings: Vec<Value>,
    pub validation_evidence: Vec<Value>,
    pub generated_at: Option<String>,
}

impl BrowserCapabilityRegistry {
    pub fn is_empty(&self) -> bool {
        self.browser_hosts.is_empty()
            && self.browser_executables.is_empty()
            && self.browser_capabilities.is_empty()
            && self.profile_compatibility.is_empty()
            && self.browser_preference_bindings.is_empty()
            && self.validation_evidence.is_empty()
            && self.generated_at.is_none()
    }
}

pub fn browser_profile_compatibility_matches(
    compatibility: &Value,
    profile_id: &str,
    host_id: &str,
    executable_id: &str,
) -> bool {
    compatibility.get("profileId").and_then(Value::as_str) == Some(profile_id)
        && compatibility.get("hostId").and_then(Value::as_str) == Some(host_id)
        && compatibility.get("executableId").and_then(Value::as_str) == Some(executable_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_and_each_nonempty_field_preserve_emptiness_contract() {
        let empty = BrowserCapabilityRegistry::default();
        assert!(empty.is_empty());
        assert_eq!(
            serde_json::from_value::<BrowserCapabilityRegistry>(json!({})).unwrap(),
            empty
        );
        assert_eq!(
            serde_json::to_value(&empty).unwrap(),
            json!({
                "browserHosts": [],
                "browserExecutables": [],
                "browserCapabilities": [],
                "profileCompatibility": [],
                "browserPreferenceBindings": [],
                "validationEvidence": [],
                "generatedAt": null
            })
        );
        for field in [
            "browserHosts",
            "browserExecutables",
            "browserCapabilities",
            "profileCompatibility",
            "browserPreferenceBindings",
            "validationEvidence",
        ] {
            let mut value = json!({});
            value[field] = json!([null]);
            let registry: BrowserCapabilityRegistry = serde_json::from_value(value).unwrap();
            assert!(!registry.is_empty(), "{field}");
        }
        for generated_at in ["", "not-a-timestamp"] {
            let registry = BrowserCapabilityRegistry {
                generated_at: Some(generated_at.into()),
                ..Default::default()
            };
            assert!(!registry.is_empty());
            let value = serde_json::to_value(&registry).unwrap();
            assert_eq!(value["generatedAt"], generated_at);
            assert_eq!(
                serde_json::from_value::<BrowserCapabilityRegistry>(value).unwrap(),
                registry
            );
        }
    }

    #[test]
    fn populated_registry_roundtrips_json_arrays_and_accepts_unknown_fields() {
        let known = json!({
            "browserHosts": [{"id": "host", "futureField": {"nested": true}}],
            "browserExecutables": [null, 7, "opaque"],
            "browserCapabilities": [true, {"name": "capability"}],
            "profileCompatibility": [{"profileId": "profile", "hostId": "host", "executableId": "executable"}],
            "browserPreferenceBindings": [["nested-array"]],
            "validationEvidence": [{"arbitrary": [false, 2]}],
            "generatedAt": "2026-09-17T00:00:00Z"
        });
        let mut input = known.clone();
        input["futureTopLevelField"] = json!({"accepted": true});
        let registry: BrowserCapabilityRegistry = serde_json::from_value(input).unwrap();
        assert!(!registry.is_empty());
        // Unknown outer fields are accepted but not retained; nested Values are opaque.
        assert_eq!(serde_json::to_value(&registry).unwrap(), known);
        assert_eq!(
            serde_json::from_value::<BrowserCapabilityRegistry>(known).unwrap(),
            registry
        );
    }

    #[test]
    fn compatibility_matcher_requires_three_exact_strings_only() {
        let exact = json!({
            "profileId": "profile", "hostId": "host", "executableId": "executable",
            "uninterpreted": {"compatible": false}
        });
        assert!(browser_profile_compatibility_matches(
            &exact,
            "profile",
            "host",
            "executable"
        ));
        for field in ["profileId", "hostId", "executableId"] {
            let mut missing = exact.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(!browser_profile_compatibility_matches(
                &missing,
                "profile",
                "host",
                "executable"
            ));
            for wrong in [
                json!(null),
                json!(7),
                json!(true),
                json!([]),
                json!({}),
                json!("different"),
            ] {
                let mut changed = exact.clone();
                changed[field] = wrong;
                assert!(!browser_profile_compatibility_matches(
                    &changed,
                    "profile",
                    "host",
                    "executable"
                ));
            }
        }
        for (profile, host, executable) in [
            ("Profile", "host", "executable"),
            ("profile ", "host", "executable"),
            ("profile", "HOST", "executable"),
            ("profile", "host", " executable"),
        ] {
            assert!(!browser_profile_compatibility_matches(
                &exact, profile, host, executable
            ));
        }
        assert!(browser_profile_compatibility_matches(
            &json!({"profileId": "", "hostId": "", "executableId": ""}),
            "",
            "",
            ""
        ));
        for non_record in [json!(null), json!([]), json!("profile"), json!(1)] {
            assert!(!browser_profile_compatibility_matches(
                &non_record,
                "",
                "",
                ""
            ));
        }
    }
}
