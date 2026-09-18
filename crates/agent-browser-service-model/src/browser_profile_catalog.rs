//! Minimal profile catalog for the ordinary browser-session path.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::ProfileClass;

pub const BROWSER_PROFILE_CATALOG_SCHEMA_V1: &str = "agent-browser.browser-profile-catalog.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserProfileKind {
    Named,
    Disposable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfileCatalogEntry {
    pub id: String,
    pub name: String,
    pub user_data_dir: String,
    pub kind: BrowserProfileKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfileCatalog {
    pub schema_version: String,
    pub profiles: BTreeMap<String, BrowserProfileCatalogEntry>,
}

/// A non-fatal observation made while importing the legacy profile map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserProfileCatalogDiagnostic {
    /// The key in the legacy `profiles` object, when one was available.
    pub profile_key: Option<String>,
    pub reason: String,
}

/// Result of the field-level legacy import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserProfileCatalogImport {
    pub catalog: BrowserProfileCatalog,
    pub diagnostics: Vec<BrowserProfileCatalogDiagnostic>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LegacyProfileCatalogFields {
    id: String,
    name: String,
    user_data_dir: Option<String>,
    profile_class: ProfileClass,
}

impl Default for BrowserProfileCatalog {
    fn default() -> Self {
        Self {
            schema_version: BROWSER_PROFILE_CATALOG_SCHEMA_V1.to_string(),
            profiles: BTreeMap::new(),
        }
    }
}

impl BrowserProfileCatalog {
    /// Import only `profiles` from a legacy Service State JSON document.
    ///
    /// The document is intentionally inspected as a JSON value instead of
    /// being decoded as `ServiceState`. This keeps malformed or contradictory
    /// sessions, browsers, leases, and owner records outside this catalog's
    /// authority boundary. Each profile is decoded independently, so one bad
    /// profile cannot prevent valid siblings from being imported.
    pub fn import_legacy_service_state_json(raw: &str) -> BrowserProfileCatalogImport {
        let mut imported = BrowserProfileCatalogImport {
            catalog: BrowserProfileCatalog::default(),
            diagnostics: Vec::new(),
        };
        let value: Value = match serde_json::from_str(raw) {
            Ok(value) => value,
            Err(error) => {
                imported.diagnostics.push(BrowserProfileCatalogDiagnostic {
                    profile_key: None,
                    reason: format!("invalid legacy JSON: {error}"),
                });
                return imported;
            }
        };

        let Some(profiles) = value.get("profiles") else {
            return imported;
        };
        let Some(profiles) = profiles.as_object() else {
            imported.diagnostics.push(BrowserProfileCatalogDiagnostic {
                profile_key: None,
                reason: "legacy profiles field is not an object".to_string(),
            });
            return imported;
        };

        for (legacy_key, value) in profiles {
            let profile = match serde_json::from_value::<LegacyProfileCatalogFields>(value.clone())
            {
                Ok(profile) => profile,
                Err(error) => {
                    imported.diagnostics.push(BrowserProfileCatalogDiagnostic {
                        profile_key: Some(legacy_key.clone()),
                        reason: format!("profile could not be decoded: {error}"),
                    });
                    continue;
                }
            };
            let id = non_empty(profile.id.as_str())
                .unwrap_or(legacy_key)
                .to_string();
            let Some(name) = non_empty(profile.name.as_str()) else {
                imported.diagnostics.push(BrowserProfileCatalogDiagnostic {
                    profile_key: Some(legacy_key.clone()),
                    reason: "profile name is empty".to_string(),
                });
                continue;
            };
            let Some(user_data_dir) = profile.user_data_dir.as_deref().and_then(non_empty) else {
                imported.diagnostics.push(BrowserProfileCatalogDiagnostic {
                    profile_key: Some(legacy_key.clone()),
                    reason: "profile userDataDir is missing or empty".to_string(),
                });
                continue;
            };

            let entry = BrowserProfileCatalogEntry {
                id: id.clone(),
                name: name.to_string(),
                user_data_dir: user_data_dir.to_string(),
                kind: profile_kind(profile.profile_class),
            };
            imported.catalog.profiles.insert(id, entry);
        }
        imported
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn profile_kind(profile_class: ProfileClass) -> BrowserProfileKind {
    match profile_class {
        crate::ProfileClass::ManagedOneTime => BrowserProfileKind::Disposable,
        crate::ProfileClass::Default
        | crate::ProfileClass::DurableNamed
        | crate::ProfileClass::OperatorSupplied => BrowserProfileKind::Named,
    }
}
