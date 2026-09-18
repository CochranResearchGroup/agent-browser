use agent_browser_service_model::{ServiceEntitySource, ServiceEntitySources};

#[test]
fn source_labels_match_the_existing_model_contract() {
    let cases = [
        (
            ServiceEntitySource::PersistedState,
            "persisted_state",
            false,
        ),
        (ServiceEntitySource::Config, "config", false),
        (ServiceEntitySource::Builtin, "builtin", true),
        (
            ServiceEntitySource::RuntimeObserved,
            "runtime_observed",
            false,
        ),
    ];

    for (source, label, overrideable) in cases {
        assert_eq!(source.as_str(), label);
        assert_eq!(source.overrideable(), overrideable);
    }
}

#[test]
fn provenance_indexes_are_in_memory_maps_with_empty_defaults() {
    let mut sources = ServiceEntitySources::default();
    assert!(sources.profiles.is_empty());
    assert!(sources.site_policies.is_empty());

    sources
        .profiles
        .insert("profile-a".to_string(), ServiceEntitySource::Config);
    sources.site_policies.insert(
        "accounts-google".to_string(),
        ServiceEntitySource::RuntimeObserved,
    );

    assert_eq!(
        sources.profiles.get("profile-a"),
        Some(&ServiceEntitySource::Config)
    );
    assert_eq!(
        sources.site_policies.get("accounts-google"),
        Some(&ServiceEntitySource::RuntimeObserved)
    );
}
