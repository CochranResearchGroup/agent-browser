use agent_browser_challenge_control::{
    classify_native_challenge, NativeChallengeKind, NativeChallengeText, HCAPTCHA_PROFILE,
};

#[test]
fn native_corpus_recognizes_a_registered_checkbox_without_granting_an_attempt() {
    let lines = [
        NativeChallengeText {
            text: "hCaptcha",
            confidence: 94.0,
        },
        NativeChallengeText {
            text: "I am human",
            confidence: 91.0,
        },
    ];
    let result = classify_native_challenge(&lines, &"a".repeat(64), true, true);
    assert_eq!(result.kind, NativeChallengeKind::Checkbox);
    assert_eq!(
        result.registered_profile_id,
        Some(HCAPTCHA_PROFILE.profile_id)
    );
    assert!(!result.automated_attempt_authorized);
}

#[test]
fn declared_native_corpus_distinguishes_obstruction_without_claiming_site_access() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/native-challenge-corpus-v1.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let lines = case["text"]
            .as_array()
            .unwrap()
            .iter()
            .map(|text| NativeChallengeText {
                text: text.as_str().unwrap(),
                confidence: case["confidence"].as_f64().unwrap(),
            })
            .collect::<Vec<_>>();
        let result = classify_native_challenge(
            &lines,
            &"a".repeat(64),
            case["fresh"].as_bool().unwrap(),
            case["complete"].as_bool().unwrap(),
        );
        assert_eq!(
            serde_json::to_value(&result.kind).unwrap(),
            case["expected"],
            "{}",
            case["id"]
        );
        assert_eq!(result.corpus_version, corpus["version"]);
        assert!(!result.automated_attempt_authorized);
    }
}
