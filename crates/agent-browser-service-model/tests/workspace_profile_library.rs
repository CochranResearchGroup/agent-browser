use agent_browser_service_model::*;
use serde_json::json;

#[test]
fn intended_associations_round_trip_without_asserting_or_replacing_authentication() {
    let mut state = ServiceState::default();
    state.site_policies.insert(
        "accounts".into(),
        SitePolicy {
            id: "accounts".into(),
            ..Default::default()
        },
    );
    state.profiles.insert("work".into(), serde_json::from_value(json!({"id":"work","name":"Work","targetReadiness":[{"targetServiceId":"accounts","state":"stale","evidence":"old independent probe"}]})).unwrap());
    let expected = state.profiles["work"].target_readiness.clone();
    replace_profile_site_associations(
        &mut state,
        "work",
        vec![ProfileSiteAssociation {
            site_policy_id: "accounts".into(),
            account: "person@example.test".into(),
            organization: "Research".into(),
        }],
        &[],
        "2026-11-01T12:00:00Z",
    )
    .unwrap();
    let restored: ServiceState =
        serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    assert_eq!(
        restored.profiles["work"].site_associations[0].organization,
        "Research"
    );
    assert_eq!(restored.profiles["work"].target_readiness, expected);
    assert!(restored.profiles["work"]
        .authenticated_service_ids
        .is_empty());
    assert_eq!(
        restored.profiles["work"]
            .site_associations_updated_at
            .as_deref(),
        Some("2026-11-01T12:00:00Z")
    );
}

#[test]
fn public_selection_requires_exact_context_and_preserves_explicit_choice() {
    let mut state = ServiceState::default();
    state.site_policies.insert(
        "accounts".into(),
        SitePolicy {
            id: "accounts".into(),
            ..Default::default()
        },
    );
    for (id, org) in [("research", "Research"), ("other", "Other")] {
        state.profiles.insert(id.into(),serde_json::from_value(json!({"id":id,"siteAssociations":[{"sitePolicyId":"accounts","account":"same@example.test","organization":org}],"browserBuild":"stock_chrome"})).unwrap());
    }
    let request: ProfileLibrarySelectionRequest=serde_json::from_value(json!({"sitePolicyId":"accounts","account":"same@example.test","organization":"Research","browserBuild":"stock_chrome"})).unwrap();
    assert_eq!(
        select_profile_for_site_identity(&state, &request)
            .profile_id
            .as_deref(),
        Some("research")
    );
    let mut explicit = request.clone();
    explicit.profile_id = Some("other".into());
    assert_eq!(
        select_profile_for_site_identity(&state, &explicit)
            .profile_id
            .as_deref(),
        Some("other")
    );
    state
        .profiles
        .insert("duplicate".into(), state.profiles["research"].clone());
    state.profiles.get_mut("duplicate").unwrap().id = "duplicate".into();
    let ambiguous = select_profile_for_site_identity(&state, &request);
    assert_eq!(ambiguous.state, "choice_required");
    assert_eq!(ambiguous.profile_id, None);
    assert!(ambiguous.identity_verification_required);
    explicit.browser_build = Some(BrowserBuild::CdpFreeHeaded);
    assert_eq!(
        select_profile_for_site_identity(&state, &explicit).state,
        "conflict"
    );
    explicit.profile_id = Some("missing".into());
    assert_eq!(
        select_profile_for_site_identity(&state, &explicit).profile_id,
        None
    );
    let mut missing = request;
    missing.organization = "Missing".into();
    assert_eq!(
        select_profile_for_site_identity(&state, &missing).state,
        "choice_required"
    );
}

#[test]
fn public_library_handler_filters_policy_and_denies_untrusted_or_unknown_writes() {
    let mut state = ServiceState::default();
    state.site_policies.insert(
        "accounts".into(),
        SitePolicy {
            id: "accounts".into(),
            ..Default::default()
        },
    );
    let mut private = BrowserProfile {
        id: "private".into(),
        name: "Private account".into(),
        ..Default::default()
    };
    let mut policy = ServiceProfileAccessPolicy::shared_local_default("private");
    policy.mode = ProfileAccessMode::Restricted;
    policy.default_permissions.clear();
    private.access_policy = Some(policy);
    state.profiles.insert("private".into(), private);
    state.profiles.insert(
        "public".into(),
        BrowserProfile {
            id: "public".into(),
            name: "Public profile".into(),
            ..Default::default()
        },
    );
    let actor = ProfileLibraryActor {
        subject_id: Some("caller".into()),
        assurance: ProfileIdentityAssurance::SelfDeclared,
    };
    let listing = profile_library_request(
        &mut state,
        &json!({"operation":"list"}),
        &actor,
        "2026-11-01T12:00:00Z",
    )
    .unwrap();
    assert_eq!(listing["profiles"].as_array().unwrap().len(), 1);
    assert_eq!(listing["profiles"][0]["profile"]["id"], "public");
    let original = state.clone();
    for id in ["private", "missing"] {
        let denied = profile_library_request(
            &mut state,
            &json!({"operation":"replace_associations","profileId":id,"associations":[],"expectedAssociations":[]}),
            &actor,
            "2026-11-01T12:00:00Z",
        );
        assert_eq!(
            denied.unwrap_err(),
            "Profile unavailable or permission denied"
        );
    }
    assert_eq!(state, original);
    let choice = profile_library_request(
        &mut state,
        &json!({"operation":"select","sitePolicyId":"accounts","profileId":"private"}),
        &actor,
        "2026-11-01T12:00:00Z",
    )
    .unwrap();
    assert_eq!(choice["profileId"], serde_json::Value::Null);
    assert!(choice["candidates"].as_array().unwrap().is_empty());
    let write = json!({"operation":"replace_associations","profileId":"public","associations":[{"sitePolicyId":"accounts","account":"person@example.test","organization":"Research"}],"expectedAssociations":[]});
    profile_library_request(&mut state, &write, &actor, "2026-11-01T12:00:00Z").unwrap();
    let read = profile_library_request(
        &mut state,
        &json!({"operation":"list"}),
        &actor,
        "2026-11-01T12:00:00Z",
    )
    .unwrap();
    assert_eq!(
        read["profiles"][0]["profile"]["siteAssociations"][0]["organization"],
        "Research"
    );
    assert!(
        profile_library_request(&mut state, &write, &actor, "2026-11-01T12:00:01Z")
            .unwrap_err()
            .contains("changed; reload")
    );
}

#[test]
fn profile_choice_respects_current_mode_without_conflating_chrome_build_and_cdp() {
    let mut state = ServiceState::default();
    state.site_policies.insert(
        "accounts".into(),
        SitePolicy {
            id: "accounts".into(),
            requires_cdp_free: true,
            ..Default::default()
        },
    );
    state.profiles.insert("work".into(),serde_json::from_value(json!({"id":"work","browserBuild":"stock_chrome","siteAssociations":[{"sitePolicyId":"accounts","account":"a","organization":"org"}]})).unwrap());
    state.browsers.insert(
        "running".into(),
        BrowserProcess {
            id: "running".into(),
            profile_id: Some("work".into()),
            health: BrowserHealth::Ready,
            cdp_endpoint: None,
            ..Default::default()
        },
    );
    let request = ProfileLibrarySelectionRequest {
        site_policy_id: "accounts".into(),
        account: "a".into(),
        organization: "org".into(),
        cdp_connected: Some(false),
        ..Default::default()
    };
    assert_eq!(
        select_profile_for_site_identity(&state, &request)
            .profile_id
            .as_deref(),
        Some("work")
    );
    state.browsers.get_mut("running").unwrap().cdp_endpoint = Some("http://127.0.0.1:9000".into());
    assert_eq!(
        select_profile_for_site_identity(&state, &request).state,
        "conflict"
    );
    assert_eq!(
        profile_library_snapshot(&state)["profiles"][0]["currentUse"][0]["mode"],
        "automation_connected"
    );
}

#[test]
fn configured_profile_keeps_owner_edited_intent_and_current_config_policy() {
    let mut state = ServiceState::default();
    state.site_policies.insert(
        "accounts".into(),
        SitePolicy {
            id: "accounts".into(),
            ..Default::default()
        },
    );
    let mut configured = ServiceState::default();
    configured.profiles.insert(
        "work".into(),
        BrowserProfile {
            id: "work".into(),
            name: "Configured work".into(),
            browser_build: Some(BrowserBuild::StockChrome),
            ..Default::default()
        },
    );
    state.overlay_configured_entities(configured.clone());
    let actor = ProfileLibraryActor {
        subject_id: None,
        assurance: ProfileIdentityAssurance::Unknown,
    };
    profile_library_request(&mut state,&json!({"operation":"replace_associations","profileId":"work","associations":[{"sitePolicyId":"accounts","account":"a","organization":"org"}],"expectedAssociations":[]}),&actor,"2026-11-01T12:00:00Z").unwrap();
    let persisted: ServiceState =
        serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    let mut reopened = persisted;
    configured.profiles.get_mut("work").unwrap().name = "New configured name".into();
    reopened.overlay_configured_entities(configured);
    let listing = profile_library_request(
        &mut reopened,
        &json!({"operation":"list"}),
        &actor,
        "2026-11-01T12:00:01Z",
    )
    .unwrap();
    assert_eq!(
        listing["profiles"][0]["profile"]["name"],
        "New configured name"
    );
    assert_eq!(
        listing["profiles"][0]["profile"]["siteAssociations"][0]["organization"],
        "org"
    );
}

#[test]
fn explicit_profile_can_be_authorized_without_inventing_a_site_context() {
    let mut state = ServiceState::default();
    state.profiles.insert(
        "work".into(),
        BrowserProfile {
            id: "work".into(),
            ..Default::default()
        },
    );
    let actor = ProfileLibraryActor {
        subject_id: Some("dashboard:operator".into()),
        assurance: ProfileIdentityAssurance::AuthenticatedIngress,
    };
    let choice = profile_library_request(
        &mut state,
        &json!({"operation":"select","profileId":"work"}),
        &actor,
        "2026-11-01T12:00:00Z",
    )
    .unwrap();
    assert_eq!(choice["profileId"], "work");
    let invalid = profile_library_request(
        &mut state,
        &json!({"operation":"select","profileId":"work","sitePolicyId":"unknown"}),
        &actor,
        "2026-11-01T12:00:00Z",
    )
    .unwrap();
    assert_eq!(invalid["state"], "conflict");
    let auto = profile_library_request(
        &mut state,
        &json!({"operation":"select"}),
        &actor,
        "2026-11-01T12:00:00Z",
    )
    .unwrap();
    assert_ne!(auto["state"], "selected");
}
