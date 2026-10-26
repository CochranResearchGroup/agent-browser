//! Read-only interpretation of bounded native OCR evidence, without authority.
use serde::Serialize;

pub const CHALLENGE_CORPUS_VERSION: &str = "native-challenge-corpus.v1";

#[derive(Debug)]
pub struct NativeChallengeText<'a> {
    pub text: &'a str,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeChallengeKind {
    PassiveCheck,
    Checkbox,
    ImagePuzzle,
    AuthenticationRequired,
    Loading,
    AccessDenied,
    Unknown,
    Ambiguous,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeChallengeClassification {
    pub corpus_version: &'static str,
    pub kind: NativeChallengeKind,
    pub family: Option<&'static str>,
    pub registered_profile_id: Option<&'static str>,
    pub rule_ids: Vec<&'static str>,
    pub frame_sha256: String,
    pub complete_window: bool,
    pub minimum_ocr_confidence: u8,
    pub limitation: &'static str,
    pub automated_attempt_authorized: bool,
}

/// Interpret positive text signatures only. Unknown is never challenge clearance.
/// OCR scores are engine scores, not probabilities, and no attempt is authorized.
pub fn classify_native_challenge(
    lines: &[NativeChallengeText<'_>],
    frame_sha256: &str,
    fresh: bool,
    complete_window: bool,
) -> NativeChallengeClassification {
    let mut result = NativeChallengeClassification {
        corpus_version: CHALLENGE_CORPUS_VERSION,
        kind: NativeChallengeKind::Unknown,
        family: None,
        registered_profile_id: None,
        rule_ids: Vec::new(),
        frame_sha256: frame_sha256.into(),
        complete_window,
        minimum_ocr_confidence: 70,
        limitation: "English OCR text-signature candidates only; unknown/partial is not clearance or site admission; OCR scores are not probabilities",
        automated_attempt_authorized: false,
    };
    if !fresh || frame_sha256.len() != 64 || !frame_sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        result.kind = NativeChallengeKind::Stale;
        return result;
    }
    if lines.len() > 4096
        || lines.iter().any(|line| {
            line.text.len() > 2048
                || !line.confidence.is_finite()
                || !(0.0..=100.0).contains(&line.confidence)
        })
    {
        result.rule_ids.push("evidence_limit_or_invalid");
        return result;
    }
    let text = lines
        .iter()
        .filter(|line| line.confidence >= 70.0)
        .map(|line| {
            line.text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase()
        })
        .collect::<Vec<_>>();
    let starts = |prefix: &str| text.iter().any(|line| line.starts_with(prefix));
    let exact = |phrase: &str| {
        text.iter()
            .any(|line| line.trim_end_matches(['.', '!', '?']) == phrase)
    };
    let brand = |name: &str| text.iter().any(|line| line.contains(name));
    let family = if brand("hcaptcha") {
        Some("hcaptcha")
    } else if brand("cloudflare") {
        Some("turnstile")
    } else {
        None
    };
    let mut matches = Vec::new();
    let image =
        starts("select all images") || starts("click each image") || starts("select all squares");
    if image {
        matches.push((NativeChallengeKind::ImagePuzzle, "image_instruction_v1"));
    }
    if !image
        && ((family == Some("hcaptcha") && starts("i am human"))
            || (family == Some("turnstile") && starts("verify you are human")))
    {
        matches.push((NativeChallengeKind::Checkbox, "registered_checkbox_text_v1"));
    }
    if starts("checking your browser") || starts("verifying your browser") || exact("just a moment")
    {
        matches.push((
            NativeChallengeKind::PassiveCheck,
            "passive_browser_check_v1",
        ));
    }
    if exact("sign in") && (starts("password") || starts("email")) {
        matches.push((
            NativeChallengeKind::AuthenticationRequired,
            "login_form_text_v1",
        ));
    }
    if exact("loading") {
        matches.push((NativeChallengeKind::Loading, "loading_text_v1"));
    }
    if starts("access denied") || starts("403 forbidden") || starts("you have been blocked") {
        matches.push((NativeChallengeKind::AccessDenied, "denied_text_v1"));
    }
    result.rule_ids = matches.iter().map(|(_, rule)| *rule).collect();
    result.kind = match matches.as_slice() {
        [] => NativeChallengeKind::Unknown,
        [(kind, _)] => kind.clone(),
        _ => NativeChallengeKind::Ambiguous,
    };
    if matches!(
        result.kind,
        NativeChallengeKind::Checkbox | NativeChallengeKind::ImagePuzzle
    ) {
        result.family = family;
    }
    if result.kind == NativeChallengeKind::Checkbox {
        result.registered_profile_id = match family {
            Some("hcaptcha") => Some(crate::HCAPTCHA_PROFILE.profile_id),
            Some("turnstile") => Some(crate::TURNSTILE_PROFILE.profile_id),
            _ => None,
        };
    }
    result
}
