//! Provider-free orchestration contract for unattended authentication.
//!
//! Sensitive credentials, one-time codes, and verification links belong to a
//! response-only action adapter. The durable run and every outward receipt
//! contain only stable bindings and redacted effect evidence.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const AUTHENTICATION_RUN_SCHEMA_VERSION: &str = "agent-browser.authentication-run.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuthenticationRunBinding {
    pub(crate) service_id: String,
    pub(crate) agent_id: String,
    pub(crate) task_id: String,
    pub(crate) principal_id: String,
    pub(crate) target_service_id: String,
    pub(crate) target_account_ref: String,
    pub(crate) profile_id: String,
    pub(crate) browser_id: String,
    pub(crate) session_name: String,
    pub(crate) login_tab_id: String,
    pub(crate) site_recipe_id: String,
    pub(crate) policy_digest: String,
}

impl AuthenticationRunBinding {
    fn validate(&self) -> Result<(), AuthenticationRunError> {
        let fields = [
            &self.service_id,
            &self.agent_id,
            &self.task_id,
            &self.principal_id,
            &self.target_service_id,
            &self.target_account_ref,
            &self.profile_id,
            &self.browser_id,
            &self.session_name,
            &self.login_tab_id,
            &self.site_recipe_id,
            &self.policy_digest,
        ];
        if fields.iter().any(|field| field.trim().is_empty()) {
            return Err(AuthenticationRunError::StableBindingMissing);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthenticationRunState {
    Ready,
    ObservingDelivery,
    AwaitingCandidate,
    Verifying,
    Authenticated,
    OperatorInterventionRequired,
    Blocked,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthenticationChallengeChannel {
    SmsOtp,
    DeviceVerificationEmail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthenticationActionKind {
    SubmitNativeStoredCredentials,
    SubmitSmsOtp,
    OpenDeviceVerificationLink,
    ConfirmRememberDevice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProviderWatchReceipt {
    pub(crate) provider_id: String,
    pub(crate) watch_id: String,
    pub(crate) delivery_fence_id: String,
    pub(crate) ready_before_delivery: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SameProfileNewTabProof {
    pub(crate) profile_id: String,
    pub(crate) browser_id: String,
    pub(crate) session_name: String,
    pub(crate) tab_id: String,
    pub(crate) opened_in_new_tab: bool,
    pub(crate) navigation_consumed_internally: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuthenticationActionReceipt {
    pub(crate) provider_id: String,
    pub(crate) effect_id: String,
    pub(crate) action: AuthenticationActionKind,
    pub(crate) native_credential_store_used: bool,
    pub(crate) credentials_replayed: bool,
    pub(crate) challenge_material_consumed: bool,
    pub(crate) response_only_material_consumption: bool,
    pub(crate) delivery_watch_ready_before_trigger: bool,
    pub(crate) delivery_fence_id: Option<String>,
    pub(crate) candidate_count: Option<u32>,
    pub(crate) same_profile_new_tab: Option<SameProfileNewTabProof>,
}

/// Executes an authentication effect while retaining sensitive material
/// entirely inside the adapter. Implementations return only a redacted receipt.
pub(crate) trait ResponseOnlyAuthenticationAction {
    fn execute(
        &mut self,
        context: &AuthenticationActionContext<'_>,
    ) -> Result<AuthenticationActionReceipt, AuthenticationActionFailure>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthenticationActionContext<'a> {
    pub(crate) run_id: &'a str,
    pub(crate) operation_id: &'a str,
    pub(crate) binding: &'a AuthenticationRunBinding,
    pub(crate) action: AuthenticationActionKind,
    pub(crate) challenge_id: Option<&'a str>,
    pub(crate) delivery_fence_id: Option<&'a str>,
    pub(crate) candidate_count: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthenticationActionFailure {
    ProviderUnavailable,
    DeliveryExpired,
    EffectRejected,
    EffectUnproven,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuthenticationVerifierReceipt {
    pub(crate) verifier_id: String,
    pub(crate) target_service_id: String,
    pub(crate) target_account_ref: String,
    pub(crate) profile_id: String,
    pub(crate) browser_id: String,
    pub(crate) session_name: String,
    pub(crate) exact_target_authenticated: bool,
}

pub(crate) trait AuthenticationVerifier {
    fn verify(
        &mut self,
        context: &AuthenticationVerificationContext<'_>,
    ) -> Result<AuthenticationVerifierReceipt, AuthenticationVerifierFailure>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthenticationVerificationContext<'a> {
    pub(crate) run_id: &'a str,
    pub(crate) operation_id: &'a str,
    pub(crate) binding: &'a AuthenticationRunBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthenticationVerifierFailure {
    ProviderUnavailable,
    ObservationFailed,
    TargetAmbiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuthenticationTransitionReceipt {
    pub(crate) operation_id: String,
    pub(crate) from_state: AuthenticationRunState,
    pub(crate) to_state: AuthenticationRunState,
    pub(crate) action: String,
    pub(crate) challenge_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActiveAuthenticationChallenge {
    challenge_id: String,
    channel: AuthenticationChallengeChannel,
    watch: ProviderWatchReceipt,
    delivery_triggered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuthenticationRun {
    pub(crate) schema_version: String,
    pub(crate) run_id: String,
    pub(crate) binding: AuthenticationRunBinding,
    pub(crate) state: AuthenticationRunState,
    pub(crate) max_transitions: u32,
    pub(crate) transition_count: u32,
    pub(crate) used_operation_ids: BTreeSet<String>,
    pub(crate) completed_challenge_ids: BTreeSet<String>,
    active_challenge: Option<ActiveAuthenticationChallenge>,
    pub(crate) action_receipts: Vec<AuthenticationActionReceipt>,
    pub(crate) verifier_receipt: Option<AuthenticationVerifierReceipt>,
    pub(crate) transition_receipts: Vec<AuthenticationTransitionReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthenticationRunError {
    StableBindingMissing,
    RunIdMissing,
    OperationIdMissing,
    ChallengeIdMissing,
    ProviderWatchInvalid,
    TransitionBudgetInvalid,
    TransitionBudgetExhausted,
    OperationReplay,
    ChallengeReplay,
    ChallengeAlreadyActive,
    ChallengeMissing,
    ChallengeMismatch,
    DeliveryWatchNotReady,
    DeliveryAlreadyTriggered,
    DeliveryNotTriggered,
    CandidateNotUnique,
    UnexpectedState,
    ActionKindMismatch,
    ActionReceiptInvalid,
    CredentialReplayForbidden,
    SameProfileProofMissing,
    SameProfileProofMismatch,
    ActionFailed,
    VerifierFailed,
    ExactTargetNotAuthenticated,
}

impl AuthenticationRun {
    pub(crate) fn new(
        run_id: impl Into<String>,
        binding: AuthenticationRunBinding,
        max_transitions: u32,
    ) -> Result<Self, AuthenticationRunError> {
        let run_id = run_id.into();
        if run_id.trim().is_empty() {
            return Err(AuthenticationRunError::RunIdMissing);
        }
        binding.validate()?;
        if max_transitions == 0 {
            return Err(AuthenticationRunError::TransitionBudgetInvalid);
        }
        Ok(Self {
            schema_version: AUTHENTICATION_RUN_SCHEMA_VERSION.to_string(),
            run_id,
            binding,
            state: AuthenticationRunState::Ready,
            max_transitions,
            transition_count: 0,
            used_operation_ids: BTreeSet::new(),
            completed_challenge_ids: BTreeSet::new(),
            active_challenge: None,
            action_receipts: Vec::new(),
            verifier_receipt: None,
            transition_receipts: Vec::new(),
        })
    }

    pub(crate) fn submit_native_stored_credentials(
        &mut self,
        operation_id: &str,
        action: &mut impl ResponseOnlyAuthenticationAction,
    ) -> Result<AuthenticationActionReceipt, AuthenticationRunError> {
        self.require_state(&[AuthenticationRunState::Ready])?;
        self.reserve_effect(operation_id)?;
        let context = AuthenticationActionContext {
            run_id: &self.run_id,
            operation_id,
            binding: &self.binding,
            action: AuthenticationActionKind::SubmitNativeStoredCredentials,
            challenge_id: None,
            delivery_fence_id: None,
            candidate_count: None,
        };
        let receipt = match action.execute(&context) {
            Ok(receipt) => receipt,
            Err(_) => {
                self.transition(
                    operation_id,
                    AuthenticationRunState::Blocked,
                    "native_action_failed",
                    None,
                );
                return Err(AuthenticationRunError::ActionFailed);
            }
        };
        if let Err(error) = self.validate_primary_receipt(&receipt) {
            self.transition(
                operation_id,
                AuthenticationRunState::Blocked,
                "native_action_receipt_rejected",
                None,
            );
            return Err(error);
        }
        self.action_receipts.push(receipt.clone());
        self.transition(
            operation_id,
            AuthenticationRunState::Verifying,
            "submit_native_stored_credentials",
            None,
        );
        Ok(receipt)
    }

    pub(crate) fn prepare_watch(
        &mut self,
        operation_id: &str,
        challenge_id: &str,
        channel: AuthenticationChallengeChannel,
        watch: ProviderWatchReceipt,
    ) -> Result<(), AuthenticationRunError> {
        self.require_state(&[
            AuthenticationRunState::Ready,
            AuthenticationRunState::Verifying,
        ])?;
        self.require_operation_available(operation_id)?;
        self.require_transition_available()?;
        if challenge_id.trim().is_empty() {
            return Err(AuthenticationRunError::ChallengeIdMissing);
        }
        if self.completed_challenge_ids.contains(challenge_id) {
            return Err(AuthenticationRunError::ChallengeReplay);
        }
        if self.active_challenge.is_some() {
            return Err(AuthenticationRunError::ChallengeAlreadyActive);
        }
        if !watch.ready_before_delivery
            || [
                watch.provider_id.as_str(),
                watch.watch_id.as_str(),
                watch.delivery_fence_id.as_str(),
            ]
            .iter()
            .any(|value| value.trim().is_empty())
        {
            return Err(AuthenticationRunError::ProviderWatchInvalid);
        }
        self.used_operation_ids.insert(operation_id.to_string());
        self.active_challenge = Some(ActiveAuthenticationChallenge {
            challenge_id: challenge_id.to_string(),
            channel,
            watch,
            delivery_triggered: false,
        });
        self.transition(
            operation_id,
            AuthenticationRunState::ObservingDelivery,
            "prepare_watch",
            Some(challenge_id),
        );
        Ok(())
    }

    pub(crate) fn mark_delivery_triggered(
        &mut self,
        operation_id: &str,
        challenge_id: &str,
    ) -> Result<(), AuthenticationRunError> {
        self.require_state(&[AuthenticationRunState::ObservingDelivery])?;
        self.require_operation_available(operation_id)?;
        self.require_transition_available()?;
        let challenge = self
            .active_challenge
            .as_mut()
            .ok_or(AuthenticationRunError::DeliveryWatchNotReady)?;
        if challenge.challenge_id != challenge_id {
            return Err(AuthenticationRunError::ChallengeMismatch);
        }
        if !challenge.watch.ready_before_delivery {
            return Err(AuthenticationRunError::DeliveryWatchNotReady);
        }
        if challenge.delivery_triggered {
            return Err(AuthenticationRunError::DeliveryAlreadyTriggered);
        }
        challenge.delivery_triggered = true;
        self.used_operation_ids.insert(operation_id.to_string());
        self.transition(
            operation_id,
            AuthenticationRunState::AwaitingCandidate,
            "trigger_delivery",
            Some(challenge_id),
        );
        Ok(())
    }

    pub(crate) fn consume_challenge(
        &mut self,
        operation_id: &str,
        challenge_id: &str,
        candidate_count: usize,
        action: &mut impl ResponseOnlyAuthenticationAction,
    ) -> Result<AuthenticationActionReceipt, AuthenticationRunError> {
        self.require_operation_available(operation_id)?;
        self.require_state(&[AuthenticationRunState::AwaitingCandidate])?;
        self.require_transition_available()?;
        if candidate_count != 1 {
            return Err(AuthenticationRunError::CandidateNotUnique);
        }
        let challenge = self
            .active_challenge
            .as_ref()
            .ok_or(AuthenticationRunError::ChallengeMissing)?;
        if challenge.challenge_id != challenge_id {
            return Err(AuthenticationRunError::ChallengeMismatch);
        }
        if !challenge.delivery_triggered {
            return Err(AuthenticationRunError::DeliveryNotTriggered);
        }
        let expected_action = match challenge.channel {
            AuthenticationChallengeChannel::SmsOtp => AuthenticationActionKind::SubmitSmsOtp,
            AuthenticationChallengeChannel::DeviceVerificationEmail => {
                AuthenticationActionKind::OpenDeviceVerificationLink
            }
        };
        let delivery_fence_id = challenge.watch.delivery_fence_id.clone();
        self.used_operation_ids.insert(operation_id.to_string());
        let context = AuthenticationActionContext {
            run_id: &self.run_id,
            operation_id,
            binding: &self.binding,
            action: expected_action,
            challenge_id: Some(challenge_id),
            delivery_fence_id: Some(&delivery_fence_id),
            candidate_count: Some(candidate_count),
        };
        let receipt = match action.execute(&context) {
            Ok(receipt) => receipt,
            Err(_) => {
                self.transition(
                    operation_id,
                    AuthenticationRunState::Blocked,
                    "challenge_action_failed",
                    Some(challenge_id),
                );
                return Err(AuthenticationRunError::ActionFailed);
            }
        };
        if let Err(error) =
            self.validate_challenge_receipt(expected_action, &delivery_fence_id, &receipt)
        {
            self.transition(
                operation_id,
                AuthenticationRunState::Blocked,
                "challenge_action_receipt_rejected",
                Some(challenge_id),
            );
            return Err(error);
        }
        self.completed_challenge_ids
            .insert(challenge_id.to_string());
        self.active_challenge = None;
        self.action_receipts.push(receipt.clone());
        self.transition(
            operation_id,
            AuthenticationRunState::Verifying,
            "consume_challenge",
            Some(challenge_id),
        );
        Ok(receipt)
    }

    pub(crate) fn verify_exact_target(
        &mut self,
        operation_id: &str,
        verifier: &mut impl AuthenticationVerifier,
    ) -> Result<AuthenticationVerifierReceipt, AuthenticationRunError> {
        self.require_state(&[AuthenticationRunState::Verifying])?;
        self.reserve_effect(operation_id)?;
        let context = AuthenticationVerificationContext {
            run_id: &self.run_id,
            operation_id,
            binding: &self.binding,
        };
        let receipt = match verifier.verify(&context) {
            Ok(receipt) => receipt,
            Err(_) => {
                self.transition(
                    operation_id,
                    AuthenticationRunState::OperatorInterventionRequired,
                    "verifier_failed",
                    None,
                );
                return Err(AuthenticationRunError::VerifierFailed);
            }
        };
        if !receipt.exact_target_authenticated
            || receipt.target_service_id != self.binding.target_service_id
            || receipt.target_account_ref != self.binding.target_account_ref
            || receipt.profile_id != self.binding.profile_id
            || receipt.browser_id != self.binding.browser_id
            || receipt.session_name != self.binding.session_name
            || receipt.verifier_id.trim().is_empty()
        {
            self.transition(
                operation_id,
                AuthenticationRunState::OperatorInterventionRequired,
                "exact_target_not_authenticated",
                None,
            );
            return Err(AuthenticationRunError::ExactTargetNotAuthenticated);
        }
        self.verifier_receipt = Some(receipt.clone());
        self.transition(
            operation_id,
            AuthenticationRunState::Authenticated,
            "verify_exact_target",
            None,
        );
        Ok(receipt)
    }

    fn validate_primary_receipt(
        &self,
        receipt: &AuthenticationActionReceipt,
    ) -> Result<(), AuthenticationRunError> {
        self.validate_common_receipt(receipt)?;
        if receipt.action != AuthenticationActionKind::SubmitNativeStoredCredentials {
            return Err(AuthenticationRunError::ActionKindMismatch);
        }
        if !receipt.native_credential_store_used
            || receipt.challenge_material_consumed
            || receipt.delivery_watch_ready_before_trigger
            || receipt.delivery_fence_id.is_some()
            || receipt.candidate_count.is_some()
            || receipt.same_profile_new_tab.is_some()
        {
            return Err(AuthenticationRunError::ActionReceiptInvalid);
        }
        Ok(())
    }

    fn validate_challenge_receipt(
        &self,
        expected_action: AuthenticationActionKind,
        expected_delivery_fence_id: &str,
        receipt: &AuthenticationActionReceipt,
    ) -> Result<(), AuthenticationRunError> {
        self.validate_common_receipt(receipt)?;
        if receipt.action != expected_action {
            return Err(AuthenticationRunError::ActionKindMismatch);
        }
        if receipt.native_credential_store_used
            || !receipt.challenge_material_consumed
            || !receipt.delivery_watch_ready_before_trigger
            || receipt.delivery_fence_id.as_deref() != Some(expected_delivery_fence_id)
            || receipt.candidate_count != Some(1)
        {
            return Err(AuthenticationRunError::ActionReceiptInvalid);
        }
        match expected_action {
            AuthenticationActionKind::SubmitSmsOtp => {
                if receipt.same_profile_new_tab.is_some() {
                    return Err(AuthenticationRunError::ActionReceiptInvalid);
                }
            }
            AuthenticationActionKind::OpenDeviceVerificationLink => {
                let proof = receipt
                    .same_profile_new_tab
                    .as_ref()
                    .ok_or(AuthenticationRunError::SameProfileProofMissing)?;
                if proof.profile_id != self.binding.profile_id
                    || proof.browser_id != self.binding.browser_id
                    || proof.session_name != self.binding.session_name
                    || proof.tab_id.trim().is_empty()
                    || proof.tab_id == self.binding.login_tab_id
                    || !proof.opened_in_new_tab
                    || !proof.navigation_consumed_internally
                {
                    return Err(AuthenticationRunError::SameProfileProofMismatch);
                }
            }
            _ => return Err(AuthenticationRunError::ActionKindMismatch),
        }
        Ok(())
    }

    fn validate_common_receipt(
        &self,
        receipt: &AuthenticationActionReceipt,
    ) -> Result<(), AuthenticationRunError> {
        if receipt.credentials_replayed {
            return Err(AuthenticationRunError::CredentialReplayForbidden);
        }
        if receipt.provider_id.trim().is_empty()
            || receipt.effect_id.trim().is_empty()
            || !receipt.response_only_material_consumption
        {
            return Err(AuthenticationRunError::ActionReceiptInvalid);
        }
        Ok(())
    }

    fn reserve_effect(&mut self, operation_id: &str) -> Result<(), AuthenticationRunError> {
        self.require_operation_available(operation_id)?;
        self.require_transition_available()?;
        self.used_operation_ids.insert(operation_id.to_string());
        Ok(())
    }

    fn require_operation_available(
        &self,
        operation_id: &str,
    ) -> Result<(), AuthenticationRunError> {
        if operation_id.trim().is_empty() {
            return Err(AuthenticationRunError::OperationIdMissing);
        }
        if self.used_operation_ids.contains(operation_id) {
            return Err(AuthenticationRunError::OperationReplay);
        }
        Ok(())
    }

    fn require_transition_available(&self) -> Result<(), AuthenticationRunError> {
        if self.transition_count >= self.max_transitions {
            return Err(AuthenticationRunError::TransitionBudgetExhausted);
        }
        Ok(())
    }

    fn require_state(
        &self,
        allowed: &[AuthenticationRunState],
    ) -> Result<(), AuthenticationRunError> {
        if allowed.contains(&self.state) {
            Ok(())
        } else {
            Err(AuthenticationRunError::UnexpectedState)
        }
    }

    fn transition(
        &mut self,
        operation_id: &str,
        to_state: AuthenticationRunState,
        action: &str,
        challenge_id: Option<&str>,
    ) {
        let from_state = self.state;
        self.state = to_state;
        self.transition_count += 1;
        self.transition_receipts
            .push(AuthenticationTransitionReceipt {
                operation_id: operation_id.to_string(),
                from_state,
                to_state,
                action: action.to_string(),
                challenge_id: challenge_id.map(str::to_string),
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OTP_CANARY: &str = "SYNTHETIC-OTP-CANARY-NOT-A-REAL-CODE";
    const URL_CANARY: &str = "https://verify.example.invalid/device?token=private-canary";

    fn binding() -> AuthenticationRunBinding {
        AuthenticationRunBinding {
            service_id: "books-receipts".to_string(),
            agent_id: "agent-books".to_string(),
            task_id: "task-july-close".to_string(),
            principal_id: "principal-books".to_string(),
            target_service_id: "bill".to_string(),
            target_account_ref: "soylei".to_string(),
            profile_id: "bill-soylei-chrome".to_string(),
            browser_id: "browser-bill-soylei".to_string(),
            session_name: "bill-soylei".to_string(),
            login_tab_id: "tab-login".to_string(),
            site_recipe_id: "bill-auth-v1".to_string(),
            policy_digest: "policy-digest".to_string(),
        }
    }

    fn watch() -> ProviderWatchReceipt {
        ProviderWatchReceipt {
            provider_id: "im-receipts".to_string(),
            watch_id: "watch-1".to_string(),
            delivery_fence_id: "fence-1".to_string(),
            ready_before_delivery: true,
        }
    }

    struct FakeResponseOnlyAction {
        secret_material: Vec<u8>,
        receipt: AuthenticationActionReceipt,
        calls: usize,
    }

    impl FakeResponseOnlyAction {
        fn sms(secret: &str) -> Self {
            Self {
                secret_material: secret.as_bytes().to_vec(),
                receipt: AuthenticationActionReceipt {
                    provider_id: "im-receipts".to_string(),
                    effect_id: "sms-effect-1".to_string(),
                    action: AuthenticationActionKind::SubmitSmsOtp,
                    native_credential_store_used: false,
                    credentials_replayed: false,
                    challenge_material_consumed: true,
                    response_only_material_consumption: true,
                    delivery_watch_ready_before_trigger: true,
                    delivery_fence_id: Some("fence-1".to_string()),
                    candidate_count: Some(1),
                    same_profile_new_tab: None,
                },
                calls: 0,
            }
        }

        fn email_link(secret: &str, proof: SameProfileNewTabProof) -> Self {
            Self {
                secret_material: secret.as_bytes().to_vec(),
                receipt: AuthenticationActionReceipt {
                    provider_id: "mail-receipts".to_string(),
                    effect_id: "email-effect-1".to_string(),
                    action: AuthenticationActionKind::OpenDeviceVerificationLink,
                    native_credential_store_used: false,
                    credentials_replayed: false,
                    challenge_material_consumed: true,
                    response_only_material_consumption: true,
                    delivery_watch_ready_before_trigger: true,
                    delivery_fence_id: Some("fence-1".to_string()),
                    candidate_count: Some(1),
                    same_profile_new_tab: Some(proof),
                },
                calls: 0,
            }
        }

        fn native() -> Self {
            Self {
                secret_material: Vec::new(),
                receipt: AuthenticationActionReceipt {
                    provider_id: "stock-chrome-native-credential-store".to_string(),
                    effect_id: "native-effect-1".to_string(),
                    action: AuthenticationActionKind::SubmitNativeStoredCredentials,
                    native_credential_store_used: true,
                    credentials_replayed: false,
                    challenge_material_consumed: false,
                    response_only_material_consumption: true,
                    delivery_watch_ready_before_trigger: false,
                    delivery_fence_id: None,
                    candidate_count: None,
                    same_profile_new_tab: None,
                },
                calls: 0,
            }
        }
    }

    impl Drop for FakeResponseOnlyAction {
        fn drop(&mut self) {
            self.secret_material.fill(0);
        }
    }

    impl ResponseOnlyAuthenticationAction for FakeResponseOnlyAction {
        fn execute(
            &mut self,
            context: &AuthenticationActionContext<'_>,
        ) -> Result<AuthenticationActionReceipt, AuthenticationActionFailure> {
            self.calls += 1;
            assert_eq!(context.action, self.receipt.action);
            assert!(!context.run_id.is_empty());
            assert!(!context.operation_id.is_empty());
            assert!(!context.binding.profile_id.is_empty());
            if context.action != AuthenticationActionKind::SubmitNativeStoredCredentials {
                assert!(!self.secret_material.is_empty());
                assert!(context.challenge_id.is_some());
                assert!(context.delivery_fence_id.is_some());
                assert_eq!(context.candidate_count, Some(1));
            }
            Ok(self.receipt.clone())
        }
    }

    struct FakeVerifier {
        receipt: AuthenticationVerifierReceipt,
        calls: usize,
    }

    impl FakeVerifier {
        fn exact() -> Self {
            let binding = binding();
            Self {
                receipt: AuthenticationVerifierReceipt {
                    verifier_id: "bill-exact-target-verifier".to_string(),
                    target_service_id: binding.target_service_id,
                    target_account_ref: binding.target_account_ref,
                    profile_id: binding.profile_id,
                    browser_id: binding.browser_id,
                    session_name: binding.session_name,
                    exact_target_authenticated: true,
                },
                calls: 0,
            }
        }
    }

    impl AuthenticationVerifier for FakeVerifier {
        fn verify(
            &mut self,
            context: &AuthenticationVerificationContext<'_>,
        ) -> Result<AuthenticationVerifierReceipt, AuthenticationVerifierFailure> {
            self.calls += 1;
            assert!(!context.run_id.is_empty());
            assert!(!context.operation_id.is_empty());
            assert_eq!(context.binding.target_service_id, "bill");
            Ok(self.receipt.clone())
        }
    }

    fn prepare_and_trigger(
        run: &mut AuthenticationRun,
        challenge_id: &str,
        channel: AuthenticationChallengeChannel,
    ) {
        run.prepare_watch("op-watch", challenge_id, channel, watch())
            .unwrap();
        run.mark_delivery_triggered("op-trigger", challenge_id)
            .unwrap();
    }

    #[test]
    fn construction_requires_complete_binding_and_positive_budget() {
        let mut missing = binding();
        missing.profile_id.clear();
        assert_eq!(
            AuthenticationRun::new("run-1", missing, 5).unwrap_err(),
            AuthenticationRunError::StableBindingMissing
        );
        assert_eq!(
            AuthenticationRun::new("run-1", binding(), 0).unwrap_err(),
            AuthenticationRunError::TransitionBudgetInvalid
        );
    }

    #[test]
    fn delivery_requires_a_ready_watch_and_unique_candidate() {
        let mut run = AuthenticationRun::new("run-1", binding(), 8).unwrap();
        assert_eq!(
            run.mark_delivery_triggered("op-trigger", "challenge-1"),
            Err(AuthenticationRunError::UnexpectedState)
        );
        prepare_and_trigger(
            &mut run,
            "challenge-1",
            AuthenticationChallengeChannel::SmsOtp,
        );
        let mut action = FakeResponseOnlyAction::sms(OTP_CANARY);
        assert_eq!(
            run.consume_challenge("op-consume-zero", "challenge-1", 0, &mut action),
            Err(AuthenticationRunError::CandidateNotUnique)
        );
        assert_eq!(
            run.consume_challenge("op-consume-many", "challenge-1", 2, &mut action),
            Err(AuthenticationRunError::CandidateNotUnique)
        );
        assert_eq!(action.calls, 0);
    }

    #[test]
    fn sms_material_is_response_only_and_absent_from_durable_projections() {
        let mut run = AuthenticationRun::new("run-sms", binding(), 8).unwrap();
        prepare_and_trigger(
            &mut run,
            "challenge-sms",
            AuthenticationChallengeChannel::SmsOtp,
        );
        let mut action = FakeResponseOnlyAction::sms(OTP_CANARY);
        let receipt = run
            .consume_challenge("op-consume", "challenge-sms", 1, &mut action)
            .unwrap();
        assert_eq!(action.calls, 1);
        assert!(receipt.challenge_material_consumed);
        assert!(receipt.response_only_material_consumption);
        assert!(receipt.delivery_watch_ready_before_trigger);
        assert_eq!(receipt.delivery_fence_id.as_deref(), Some("fence-1"));
        assert_eq!(receipt.candidate_count, Some(1));
        assert!(!receipt.credentials_replayed);
        let projections = format!(
            "{}|{:?}|{:?}",
            serde_json::to_string(&run).unwrap(),
            run,
            receipt
        );
        assert!(!projections.contains(OTP_CANARY));
    }

    #[test]
    fn verification_link_requires_an_exact_same_profile_new_tab_proof() {
        let mut run = AuthenticationRun::new("run-email", binding(), 8).unwrap();
        let mut email_watch = watch();
        email_watch.provider_id = "mail-receipts".to_string();
        run.prepare_watch(
            "op-watch",
            "challenge-email",
            AuthenticationChallengeChannel::DeviceVerificationEmail,
            email_watch,
        )
        .unwrap();
        run.mark_delivery_triggered("op-trigger", "challenge-email")
            .unwrap();
        let mut action = FakeResponseOnlyAction::email_link(
            URL_CANARY,
            SameProfileNewTabProof {
                profile_id: "wrong-profile".to_string(),
                browser_id: "browser-bill-soylei".to_string(),
                session_name: "bill-soylei".to_string(),
                tab_id: "tab-verification".to_string(),
                opened_in_new_tab: true,
                navigation_consumed_internally: true,
            },
        );
        assert_eq!(
            run.consume_challenge("op-consume", "challenge-email", 1, &mut action),
            Err(AuthenticationRunError::SameProfileProofMismatch)
        );
        assert_eq!(run.state, AuthenticationRunState::Blocked);
        assert_eq!(action.calls, 1);
        let projections = format!("{}|{:?}", serde_json::to_string(&run).unwrap(), run);
        assert!(!projections.contains(URL_CANARY));
    }

    #[test]
    fn verification_link_success_retains_no_url() {
        let mut run = AuthenticationRun::new("run-email", binding(), 8).unwrap();
        let mut email_watch = watch();
        email_watch.provider_id = "mail-receipts".to_string();
        run.prepare_watch(
            "op-watch",
            "challenge-email",
            AuthenticationChallengeChannel::DeviceVerificationEmail,
            email_watch,
        )
        .unwrap();
        run.mark_delivery_triggered("op-trigger", "challenge-email")
            .unwrap();
        let mut action = FakeResponseOnlyAction::email_link(
            URL_CANARY,
            SameProfileNewTabProof {
                profile_id: "bill-soylei-chrome".to_string(),
                browser_id: "browser-bill-soylei".to_string(),
                session_name: "bill-soylei".to_string(),
                tab_id: "tab-verification".to_string(),
                opened_in_new_tab: true,
                navigation_consumed_internally: true,
            },
        );
        let receipt = run
            .consume_challenge("op-consume", "challenge-email", 1, &mut action)
            .unwrap();
        let projections = format!(
            "{}|{:?}|{:?}",
            serde_json::to_string(&run).unwrap(),
            run,
            receipt
        );
        assert!(!projections.contains(URL_CANARY));
    }

    #[test]
    fn operation_and_challenge_replay_fail_before_a_second_effect() {
        let mut run = AuthenticationRun::new("run-replay", binding(), 10).unwrap();
        prepare_and_trigger(
            &mut run,
            "challenge-sms",
            AuthenticationChallengeChannel::SmsOtp,
        );
        let mut action = FakeResponseOnlyAction::sms(OTP_CANARY);
        run.consume_challenge("op-consume", "challenge-sms", 1, &mut action)
            .unwrap();
        assert_eq!(
            run.consume_challenge("op-consume", "challenge-sms", 1, &mut action),
            Err(AuthenticationRunError::OperationReplay)
        );
        assert_eq!(action.calls, 1);
        assert_eq!(
            run.prepare_watch(
                "op-watch-again",
                "challenge-sms",
                AuthenticationChallengeChannel::SmsOtp,
                watch(),
            ),
            Err(AuthenticationRunError::ChallengeReplay)
        );
        assert_eq!(action.calls, 1);
    }

    #[test]
    fn transition_budget_exhaustion_fails_closed() {
        let mut run = AuthenticationRun::new("run-budget", binding(), 1).unwrap();
        run.prepare_watch(
            "op-watch",
            "challenge-sms",
            AuthenticationChallengeChannel::SmsOtp,
            watch(),
        )
        .unwrap();
        assert_eq!(
            run.mark_delivery_triggered("op-trigger", "challenge-sms"),
            Err(AuthenticationRunError::TransitionBudgetExhausted)
        );
        assert_eq!(run.state, AuthenticationRunState::ObservingDelivery);
    }

    #[test]
    fn exact_target_verification_is_required_for_success() {
        let mut run = AuthenticationRun::new("run-verify", binding(), 5).unwrap();
        let mut native = FakeResponseOnlyAction::native();
        run.submit_native_stored_credentials("op-native", &mut native)
            .unwrap();
        assert_eq!(run.state, AuthenticationRunState::Verifying);
        let mut verifier = FakeVerifier::exact();
        run.verify_exact_target("op-verify", &mut verifier).unwrap();
        assert_eq!(run.state, AuthenticationRunState::Authenticated);
        assert_eq!(verifier.calls, 1);
    }

    #[test]
    fn a_target_mismatch_requires_operator_intervention() {
        let mut run = AuthenticationRun::new("run-mismatch", binding(), 5).unwrap();
        let mut native = FakeResponseOnlyAction::native();
        run.submit_native_stored_credentials("op-native", &mut native)
            .unwrap();
        let mut verifier = FakeVerifier::exact();
        verifier.receipt.target_account_ref = "different-account".to_string();
        assert_eq!(
            run.verify_exact_target("op-verify", &mut verifier),
            Err(AuthenticationRunError::ExactTargetNotAuthenticated)
        );
        assert_eq!(
            run.state,
            AuthenticationRunState::OperatorInterventionRequired
        );
    }
}
