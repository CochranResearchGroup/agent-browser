use agent_browser_service_model::{BrowserProfileCatalog, BrowserProfileKind};

#[test]
fn imports_only_profiles_when_unrelated_legacy_state_is_malformed() {
    let raw = serde_json::json!({
        "profiles": {
            "work": {
                "id": "work",
                "name": "Work",
                "userDataDir": "/managed/work",
                "profileClass": "durable_named"
            }
        },
        "sessions": "not a session map",
        "browsers": ["not a browser record"],
        "leaseAuthority": {"activeClaims": "malformed"},
        "runtimeOwnerRegistry": {"registry": null},
        "profileLeaseSchemaVersion": {"contradictory": true}
    })
    .to_string();

    let imported = BrowserProfileCatalog::import_legacy_service_state_json(&raw);

    assert!(imported.diagnostics.is_empty());
    assert_eq!(imported.catalog.profiles["work"].name, "Work");
    assert_eq!(
        imported.catalog.profiles["work"].kind,
        BrowserProfileKind::Named
    );
}

#[test]
fn skips_one_malformed_profile_and_imports_valid_sibling() {
    let raw = serde_json::json!({
        "profiles": {
            "good": {
                "id": "good",
                "name": "Good",
                "userDataDir": "/managed/good",
                "profileClass": "managed_one_time"
            },
            "bad": {
                "id": "bad",
                "name": "Bad",
                "userDataDir": 42
            }
        }
    })
    .to_string();

    let imported = BrowserProfileCatalog::import_legacy_service_state_json(&raw);

    assert_eq!(imported.catalog.profiles.len(), 1);
    assert_eq!(
        imported.catalog.profiles["good"].kind,
        BrowserProfileKind::Disposable
    );
    assert_eq!(imported.diagnostics.len(), 1);
    assert_eq!(imported.diagnostics[0].profile_key.as_deref(), Some("bad"));
}

#[test]
fn absent_profiles_yields_empty_v1_catalog() {
    let imported = BrowserProfileCatalog::import_legacy_service_state_json(
        r#"{"sessions":{"broken":null},"owner":"ignored"}"#,
    );

    assert_eq!(
        imported.catalog.schema_version,
        "agent-browser.browser-profile-catalog.v1"
    );
    assert!(imported.catalog.profiles.is_empty());
    assert!(imported.diagnostics.is_empty());
}

#[test]
fn missing_required_profile_fields_are_non_fatal_diagnostics() {
    let raw = serde_json::json!({
        "profiles": {
            "missing-name": {
                "id": "missing-name",
                "userDataDir": "/managed/missing-name"
            },
            "missing-path": {
                "id": "missing-path",
                "name": "Missing Path"
            }
        }
    })
    .to_string();

    let imported = BrowserProfileCatalog::import_legacy_service_state_json(&raw);

    assert!(imported.catalog.profiles.is_empty());
    assert_eq!(imported.diagnostics.len(), 2);
}

#[test]
fn profile_class_mapping_is_field_level_and_does_not_import_access_policy() {
    let raw = serde_json::json!({
        "profiles": {
            "operator": {
                "id": "operator",
                "name": "Operator",
                "userDataDir": "/managed/operator",
                "profileClass": "operator_supplied",
                "accessPolicy": "malformed but ignored by catalog"
            }
        }
    })
    .to_string();

    let imported = BrowserProfileCatalog::import_legacy_service_state_json(&raw);

    assert_eq!(
        imported.catalog.profiles["operator"].kind,
        BrowserProfileKind::Named
    );
}
