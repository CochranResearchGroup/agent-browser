use agent_browser_service_model::{
    BrowserBuild, BrowserHost, BrowserProfile, BrowserProfileCompatibilityEvidence,
    BrowserProfileRegistration, ProfileAllocationPolicy, ProfileClass, ProfileKeyringPolicy,
    ProfileOrigin, ProfileReadinessState, ProfileSeedingMode, ProfileSourceRecord,
    ProfileTargetReadiness, ServiceProfileAccessPolicy, SitePolicySourceRecord,
    SERVICE_BROWSER_HOST_VALUES, SERVICE_PROFILE_CLASS_VALUES,
};

#[test]
fn browser_profile_default_preserves_existing_record_defaults() {
    let profile = BrowserProfile::default();

    assert!(profile.id.is_empty());
    assert!(profile.target_readiness.is_empty());
    assert_eq!(profile.profile_origin, ProfileOrigin::AgentBrowserOwned);
    assert_eq!(profile.profile_class, ProfileClass::DurableNamed);
    assert_eq!(profile.allocation, ProfileAllocationPolicy::SharedService);
    assert_eq!(profile.keyring, ProfileKeyringPolicy::BasicPasswordStore);
    assert!(profile.access_policy.is_none());
}

#[test]
fn external_profile_round_trips_access_policy_and_readiness_with_existing_wire_names() {
    let profile = BrowserProfile {
        id: "external-google".to_string(),
        name: "External Google".to_string(),
        profile_origin: ProfileOrigin::ExternalByop,
        profile_class: ProfileClass::OperatorSupplied,
        access_policy: Some(ServiceProfileAccessPolicy::shared_local_default(
            "external-google",
        )),
        default_browser_host: Some(BrowserHost::AttachedExisting),
        browser_build: Some(BrowserBuild::StockChrome),
        allocation: ProfileAllocationPolicy::CallerSupplied,
        keyring: ProfileKeyringPolicy::RealOsKeychain,
        target_readiness: vec![ProfileTargetReadiness {
            target_service_id: "google".to_string(),
            state: ProfileReadinessState::Fresh,
            evidence: "auth_probe_cookie_present".to_string(),
            recommended_action: "use_profile".to_string(),
            seeding_mode: ProfileSeedingMode::AttachableOk,
            cdp_attachment_allowed_during_seeding: true,
            ..ProfileTargetReadiness::default()
        }],
        registration: Some(BrowserProfileRegistration {
            source: Some("operator".to_string()),
            ..BrowserProfileRegistration::default()
        }),
        browser_compatibility_evidence: vec![BrowserProfileCompatibilityEvidence {
            browser_family: Some("chromium".to_string()),
            browser_build: Some(BrowserBuild::StockChrome),
            evidence: "operator_observed".to_string(),
            ..BrowserProfileCompatibilityEvidence::default()
        }],
        persistent: true,
        ..BrowserProfile::default()
    };

    let encoded = serde_json::to_value(&profile).unwrap();
    assert_eq!(encoded["profileOrigin"], "external_byop");
    assert_eq!(encoded["profileClass"], "operator_supplied");
    assert_eq!(encoded["defaultBrowserHost"], "attached_existing");
    assert_eq!(encoded["accessPolicy"]["profileId"], "external-google");
    assert_eq!(
        encoded["targetReadiness"][0]["seedingMode"],
        "attachable_ok"
    );
    assert_eq!(encoded["registration"]["source"], "operator");
    assert!(encoded.get("profile_origin").is_none());
    assert!(encoded.get("access_policy").is_none());
    assert!(encoded.get("target_readiness").is_none());

    let decoded: BrowserProfile = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, profile);
}

#[test]
fn browser_host_and_profile_class_constants_match_enum_wire_values() {
    let hosts = [
        BrowserHost::LocalHeadless,
        BrowserHost::LocalHeaded,
        BrowserHost::DockerHeaded,
        BrowserHost::RemoteHeaded,
        BrowserHost::CloudProvider,
        BrowserHost::AttachedExisting,
    ];
    assert_eq!(
        serde_json::to_value(hosts).unwrap(),
        serde_json::json!(SERVICE_BROWSER_HOST_VALUES)
    );

    let classes = [
        ProfileClass::Default,
        ProfileClass::ManagedOneTime,
        ProfileClass::DurableNamed,
        ProfileClass::OperatorSupplied,
    ];
    assert_eq!(
        serde_json::to_value(classes).unwrap(),
        serde_json::json!(SERVICE_PROFILE_CLASS_VALUES)
    );

    let origins = [
        ProfileOrigin::AgentBrowserOwned,
        ProfileOrigin::ExternalByop,
        ProfileOrigin::ExternalObserved,
    ];
    assert_eq!(
        serde_json::to_value(origins).unwrap(),
        serde_json::json!(["agent_browser_owned", "external_byop", "external_observed"])
    );
}

#[test]
fn source_projection_records_preserve_camel_case_wire_names() {
    let profile = ProfileSourceRecord {
        id: "profile-a".to_string(),
        source: "persisted_state".to_string(),
        overrideable: false,
        precedence: vec!["persisted_state".to_string(), "config".to_string()],
    };
    let site_policy = SitePolicySourceRecord {
        id: "accounts-google".to_string(),
        source: "config".to_string(),
        overrideable: true,
        precedence: vec!["persisted_state".to_string(), "config".to_string()],
    };

    let profile_json = serde_json::to_value(&profile).unwrap();
    let site_policy_json = serde_json::to_value(&site_policy).unwrap();
    assert_eq!(profile_json["overrideable"], false);
    assert_eq!(site_policy_json["overrideable"], true);
    assert_eq!(profile_json["precedence"][0], "persisted_state");
    assert_eq!(site_policy_json["precedence"][1], "config");
}
