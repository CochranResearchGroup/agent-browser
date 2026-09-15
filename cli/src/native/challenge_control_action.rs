//! No-launch Service adapter for provider-free challenge lifecycle scenarios.

use agent_browser_challenge_control::{
    challenge_profile, evaluate_provider_free_scenario, ProviderFreeScenarioOutcome,
};
use serde_json::{json, Value};

pub(crate) fn handle_challenge_control_evaluate(cmd: &Value) -> Result<Value, String> {
    let profile_id = cmd
        .get("challengeProfileId")
        .and_then(Value::as_str)
        .ok_or_else(|| "challenge_control_evaluate requires challengeProfileId".to_string())?;
    let profile = challenge_profile(profile_id).ok_or_else(|| {
        format!("challenge_control_evaluate profile {profile_id} is not repository-owned")
    })?;
    let outcome = cmd
        .get("scenarioOutcome")
        .cloned()
        .ok_or_else(|| "challenge_control_evaluate requires scenarioOutcome".to_string())?;
    let outcome: ProviderFreeScenarioOutcome = serde_json::from_value(outcome)
        .map_err(|_| "challenge_control_evaluate scenarioOutcome is unsupported".to_string())?;
    let receipt = evaluate_provider_free_scenario(*profile, outcome)
        .map_err(|error| format!("challenge_control_evaluate failed: {error:?}"))?;
    Ok(json!({ "compositeReceipt": receipt }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_effect_free_receipts_for_both_registered_profiles() {
        for profile_id in ["turnstile-checkbox-p169-v1", "hcaptcha-checkbox-p181-v2"] {
            let result = handle_challenge_control_evaluate(&json!({
                "challengeProfileId": profile_id,
                "scenarioOutcome": "passed",
            }))
            .unwrap();
            assert_eq!(
                result["compositeReceipt"]["evidenceClass"],
                "provider_free_scenario"
            );
            assert_eq!(result["compositeReceipt"]["state"], "passed");
            assert_eq!(result["compositeReceipt"]["emittedEffects"], false);
        }
    }

    #[test]
    fn rejects_unregistered_profiles_and_outcomes() {
        for request in [
            json!({
                "challengeProfileId": "caller-profile",
                "scenarioOutcome": "passed",
            }),
            json!({
                "challengeProfileId": "turnstile-checkbox-p169-v1",
                "scenarioOutcome": "retry",
            }),
        ] {
            assert!(handle_challenge_control_evaluate(&request).is_err());
        }
    }
}
