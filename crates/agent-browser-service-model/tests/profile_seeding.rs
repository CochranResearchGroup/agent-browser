use agent_browser_service_model::{
    profile_seeding_handoff_id, ProfileSeedingHandoffRecord, ProfileSeedingHandoffState,
    ProfileSeedingMode,
};

#[test]
fn handoff_record_codec_preserves_existing_camel_case_wire_shape_and_defaults() {
    let decoded = ProfileSeedingHandoffRecord::decode_json(
        r#"{
            "id":"profile-a:google",
            "profileId":"profile-a",
            "targetServiceId":"google",
            "state":"seeding_waiting_for_close",
            "pid":123,
            "declaredCompleteAt":"2026-09-16T12:00:00Z"
        }"#,
    )
    .unwrap();

    assert_eq!(
        decoded.state,
        ProfileSeedingHandoffState::SeedingWaitingForClose
    );
    assert_eq!(decoded.pid, Some(123));
    assert_eq!(decoded.started_at, None);
    assert_eq!(decoded.canonical_id(), "profile-a:google");

    let encoded: serde_json::Value = serde_json::from_str(&decoded.encode_json().unwrap()).unwrap();
    assert_eq!(encoded["profileId"], "profile-a");
    assert_eq!(encoded["targetServiceId"], "google");
    assert_eq!(encoded["declaredCompleteAt"], "2026-09-16T12:00:00Z");
    assert_eq!(encoded["startedAt"], serde_json::Value::Null);
    assert!(encoded.get("profile_id").is_none());
    assert!(encoded.get("target_service_id").is_none());
    assert!(encoded.get("declared_complete_at").is_none());
}

#[test]
fn lifecycle_decisions_are_complete_and_provider_free() {
    let cases = [
        (ProfileSeedingHandoffState::NotRequired, "info", false),
        (
            ProfileSeedingHandoffState::NeedsManualSeeding,
            "action_required",
            true,
        ),
        (
            ProfileSeedingHandoffState::SeedingLaunchedDetached,
            "attention",
            true,
        ),
        (
            ProfileSeedingHandoffState::SeedingWaitingForClose,
            "action_required",
            true,
        ),
        (
            ProfileSeedingHandoffState::CompletionDeclaredWaitingForClose,
            "action_required",
            true,
        ),
        (
            ProfileSeedingHandoffState::SeedingClosedUnverified,
            "attention",
            false,
        ),
        (
            ProfileSeedingHandoffState::VerificationPending,
            "attention",
            false,
        ),
        (ProfileSeedingHandoffState::Fresh, "info", false),
        (ProfileSeedingHandoffState::Failed, "danger", false),
        (ProfileSeedingHandoffState::Abandoned, "danger", false),
    ];
    for (state, severity, blocks_profile_lease) in cases {
        assert_eq!(state.intervention_severity(), severity);
        assert_eq!(state.blocks_profile_lease(), blocks_profile_lease);
        assert!(!state.intervention_message().is_empty());
    }
}

#[test]
fn durable_key_and_launch_posture_match_the_existing_contract() {
    assert_eq!(
        profile_seeding_handoff_id("profile:a", "google"),
        "profile:a:google"
    );
    assert_eq!(
        ProfileSeedingMode::default(),
        ProfileSeedingMode::NotRequired
    );
    assert_eq!(
        serde_json::to_string(&ProfileSeedingMode::DetachedHeadedNoCdp).unwrap(),
        r#""detached_headed_no_cdp""#
    );
}
