use agent_browser_service_model::{
    BrowserBuild, BrowserProfileCompatibilityEvidence, ProfileAllocationPolicy,
    ProfileKeyringPolicy, ProfileReadinessState, ProfileSeedingMode, ProfileTargetReadiness,
};

#[test]
fn readiness_record_codec_preserves_existing_camel_case_wire_shape_and_defaults() {
    let readiness = ProfileTargetReadiness::decode_json(
        r#"{
            "targetServiceId":"google",
            "loginId":"account@example.test",
            "state":"fresh",
            "manualSeedingRequired":false,
            "evidence":"auth_probe_cookie_present",
            "recommendedAction":"use_profile",
            "seedingMode":"attachable_ok",
            "cdpAttachmentAllowedDuringSeeding":true,
            "preferredKeyring":"real_os_keychain",
            "setupScopes":["signin"],
            "lastVerifiedAt":"2026-09-16T12:00:00Z"
        }"#,
    )
    .unwrap();

    assert_eq!(readiness.state, ProfileReadinessState::Fresh);
    assert_eq!(readiness.seeding_mode, ProfileSeedingMode::AttachableOk);
    assert_eq!(
        readiness.preferred_keyring,
        Some(ProfileKeyringPolicy::RealOsKeychain)
    );
    assert!(readiness.has_explicit_freshness_evidence());

    let encoded: serde_json::Value =
        serde_json::from_str(&readiness.encode_json().unwrap()).unwrap();
    assert_eq!(encoded["targetServiceId"], "google");
    assert_eq!(encoded["preferredKeyring"], "real_os_keychain");
    assert_eq!(encoded["freshnessExpiresAt"], serde_json::Value::Null);
    assert!(encoded.get("target_service_id").is_none());
    assert!(encoded.get("manual_seeding_required").is_none());
    assert!(encoded.get("preferred_keyring").is_none());
}

#[test]
fn compatibility_evidence_codec_preserves_existing_defaults_and_browser_build_wire_name() {
    let evidence = BrowserProfileCompatibilityEvidence::decode_json(
        r#"{
            "browserFamily":"chromium",
            "browserBuild":"stealthcdp_chromium",
            "evidence":"registered",
            "observedAt":"2026-09-16T12:00:00Z"
        }"#,
    )
    .unwrap();

    assert_eq!(
        evidence.browser_build,
        Some(BrowserBuild::StealthcdpChromium)
    );
    assert_eq!(evidence.browser_version, None);
    assert_eq!(evidence.source, None);

    let encoded: serde_json::Value =
        serde_json::from_str(&evidence.encode_json().unwrap()).unwrap();
    assert_eq!(encoded["browserBuild"], "stealthcdp_chromium");
    assert_eq!(encoded["browserVersion"], serde_json::Value::Null);
    assert!(encoded.get("browser_build").is_none());
}

#[test]
fn existing_browser_build_labels_and_policy_defaults_are_retained() {
    assert_eq!(
        BrowserBuild::parse_label("chrome"),
        Some(BrowserBuild::StockChrome)
    );
    assert_eq!(
        BrowserBuild::parse_label("stealth-chromium"),
        Some(BrowserBuild::StealthcdpChromium)
    );
    assert_eq!(
        BrowserBuild::parse_label("cdp-free"),
        Some(BrowserBuild::CdpFreeHeaded)
    );
    assert_eq!(BrowserBuild::parse_label("firefox"), None);
    assert_eq!(
        ProfileAllocationPolicy::default(),
        ProfileAllocationPolicy::SharedService
    );
    assert_eq!(
        ProfileKeyringPolicy::default(),
        ProfileKeyringPolicy::BasicPasswordStore
    );
    assert_eq!(
        ProfileReadinessState::default(),
        ProfileReadinessState::Unknown
    );
}

#[test]
fn profile_readiness_family_uses_the_existing_snake_case_variants() {
    let browser_builds = [
        BrowserBuild::StockChrome,
        BrowserBuild::StealthcdpChromium,
        BrowserBuild::CdpFreeHeaded,
    ];
    assert_eq!(
        serde_json::to_value(browser_builds).unwrap(),
        serde_json::json!(["stock_chrome", "stealthcdp_chromium", "cdp_free_headed"])
    );

    let allocations = [
        ProfileAllocationPolicy::SharedService,
        ProfileAllocationPolicy::PerService,
        ProfileAllocationPolicy::PerSite,
        ProfileAllocationPolicy::PerIdentity,
        ProfileAllocationPolicy::CallerSupplied,
    ];
    assert_eq!(
        serde_json::to_value(allocations).unwrap(),
        serde_json::json!([
            "shared_service",
            "per_service",
            "per_site",
            "per_identity",
            "caller_supplied"
        ])
    );

    let keyrings = [
        ProfileKeyringPolicy::BasicPasswordStore,
        ProfileKeyringPolicy::RealOsKeychain,
        ProfileKeyringPolicy::ManagedVault,
        ProfileKeyringPolicy::ManualLoginProfile,
    ];
    assert_eq!(
        serde_json::to_value(keyrings).unwrap(),
        serde_json::json!([
            "basic_password_store",
            "real_os_keychain",
            "managed_vault",
            "manual_login_profile"
        ])
    );

    let readiness_states = [
        ProfileReadinessState::Unknown,
        ProfileReadinessState::NeedsManualSeeding,
        ProfileReadinessState::SeededUnknownFreshness,
        ProfileReadinessState::Fresh,
        ProfileReadinessState::Stale,
        ProfileReadinessState::BlockedByAttachedDevtools,
    ];
    assert_eq!(
        serde_json::to_value(readiness_states).unwrap(),
        serde_json::json!([
            "unknown",
            "needs_manual_seeding",
            "seeded_unknown_freshness",
            "fresh",
            "stale",
            "blocked_by_attached_devtools"
        ])
    );
}

#[test]
fn explicit_freshness_query_matches_the_existing_model_decision() {
    let cases = [
        (ProfileReadinessState::Unknown, None, None, false),
        (ProfileReadinessState::NeedsManualSeeding, None, None, false),
        (
            ProfileReadinessState::SeededUnknownFreshness,
            None,
            None,
            false,
        ),
        (ProfileReadinessState::Fresh, None, None, true),
        (ProfileReadinessState::Stale, None, None, true),
        (
            ProfileReadinessState::BlockedByAttachedDevtools,
            None,
            None,
            true,
        ),
        (
            ProfileReadinessState::Unknown,
            Some("2026-09-16T12:00:00Z"),
            None,
            true,
        ),
        (
            ProfileReadinessState::Unknown,
            None,
            Some("2026-09-17T12:00:00Z"),
            true,
        ),
    ];
    for (state, last_verified_at, freshness_expires_at, expected) in cases {
        let readiness = ProfileTargetReadiness {
            state,
            last_verified_at: last_verified_at.map(str::to_string),
            freshness_expires_at: freshness_expires_at.map(str::to_string),
            ..ProfileTargetReadiness::default()
        };
        assert_eq!(readiness.has_explicit_freshness_evidence(), expected);
    }
}
