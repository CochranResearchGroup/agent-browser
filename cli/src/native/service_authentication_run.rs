//! Durable public custody for response-only authentication runs.
//!
//! This module accepts only opaque references and exact retained-tab identity.
//! It intentionally does not accept selectors, usernames, passwords, one-time
//! codes, message bodies, verification URLs, or generic UI recipes.

use super::action_runtime::runtime::{
    validate_service_tab_handle_for_current_session, DaemonState,
};
use super::authentication_run::{
    AuthenticationActionFailure, AuthenticationActionKind, AuthenticationActionReceipt,
    AuthenticationChallengeChannel, AuthenticationRun, AuthenticationRunBinding,
    AuthenticationRunState, AuthenticationVerificationContext, AuthenticationVerifier,
    AuthenticationVerifierFailure, AuthenticationVerifierReceipt, ProviderWatchReceipt,
    ResponseOnlyAuthenticationAction, ResponseOnlySiteLoginAction, SiteLoginActionContext,
    SiteLoginActionReceipt, SiteLoginObservationReceipt, SiteLoginState,
};
use super::service_model::{LeaseState, ServiceState, ServiceTabHandle};
use super::service_store::{
    JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
};
use super::service_trace::service_now_timestamp;
use super::site_login_recipe::{
    classify_site_page, load_site_login_recipe, site_login_recipe_digest, SiteLoginRecipe,
    SitePageEvidence,
};
use super::{auth, interaction};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::time::Duration as StdDuration;
use url::Url;

pub(crate) const SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION: &str =
    "agent-browser.service-authentication-run.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ServiceAuthenticationRunRecord {
    pub(crate) schema_version: String,
    pub(crate) request_sha256: String,
    pub(crate) idempotency_key_sha256: String,
    pub(crate) created_at: String,
    pub(crate) deadline_at: String,
    pub(crate) service_tab_handle: ServiceTabHandle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_effect: Option<PendingAuthenticationEffect>,
    pub(crate) run: AuthenticationRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PendingAuthenticationEffect {
    operation_id: String,
    action: AuthenticationActionKind,
    state_instance_id: String,
    reserved_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthenticationRunStartIntent {
    service_name: String,
    agent_name: String,
    task_name: String,
    principal_id: String,
    target_service_id: String,
    account_ref: String,
    organization_ref: String,
    challenge_provider_id: String,
    challenge_provider_tenant_ref: String,
    challenge_provider_account_ref: String,
    profile_id: String,
    browser_id: String,
    session_name: String,
    site_recipe_id: String,
    policy_digest: String,
    idempotency_key: String,
    deadline_ms: u64,
    max_transitions: u32,
    supplied_handle: ServiceTabHandle,
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("authentication_run_hash_failed:{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn required_string(command: &Value, field: &str) -> Result<String, String> {
    command
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("authentication_run_{field}_required"))
}

fn caller_principal(command: &Value) -> Result<String, String> {
    command
        .get("clientSubjectId")
        .or_else(|| command.get("callerId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "authentication_run_caller_principal_required".to_string())
}

fn parse_start_intent(command: &Value) -> Result<AuthenticationRunStartIntent, String> {
    for forbidden in [
        "username",
        "password",
        "otp",
        "code",
        "messageBody",
        "verificationUrl",
        "selector",
        "uiAction",
        "expression",
        "script",
    ] {
        if command.get(forbidden).is_some() {
            return Err(format!("authentication_run_forbidden_field:{forbidden}"));
        }
    }
    let policy_digest = required_string(command, "policyDigest")?;
    if policy_digest.len() != 64 || !policy_digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("authentication_run_policy_digest_invalid".to_string());
    }
    let deadline_ms = command
        .get("deadlineMs")
        .and_then(Value::as_u64)
        .filter(|value| (1_000..=600_000).contains(value))
        .ok_or_else(|| "authentication_run_deadline_invalid".to_string())?;
    let max_transitions = command
        .get("maxTransitions")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| (1..=64).contains(value))
        .ok_or_else(|| "authentication_run_transition_budget_invalid".to_string())?;
    let supplied_handle: ServiceTabHandle = serde_json::from_value(
        command
            .get("serviceTabHandle")
            .cloned()
            .ok_or_else(|| "authentication_run_service_tab_handle_required".to_string())?,
    )
    .map_err(|error| format!("authentication_run_service_tab_handle_invalid:{error}"))?;
    let intent = AuthenticationRunStartIntent {
        service_name: required_string(command, "serviceName")?,
        agent_name: required_string(command, "agentName")?,
        task_name: required_string(command, "taskName")?,
        principal_id: caller_principal(command)?,
        target_service_id: required_string(command, "targetServiceId")?,
        account_ref: required_string(command, "accountRef")?,
        organization_ref: required_string(command, "organizationRef")?,
        challenge_provider_id: required_string(command, "challengeProviderId")?,
        challenge_provider_tenant_ref: required_string(command, "challengeProviderTenantRef")?,
        challenge_provider_account_ref: required_string(command, "challengeProviderAccountRef")?,
        profile_id: required_string(command, "profileId")?,
        browser_id: required_string(command, "browserId")?,
        session_name: required_string(command, "sessionName")?,
        site_recipe_id: required_string(command, "siteRecipeId")?,
        policy_digest: policy_digest.to_ascii_lowercase(),
        idempotency_key: required_string(command, "idempotencyKey")?,
        deadline_ms,
        max_transitions,
        supplied_handle,
    };
    let recipe = load_site_login_recipe(&intent.site_recipe_id)?;
    if recipe.target_service_id != intent.target_service_id {
        return Err("authentication_run_recipe_target_mismatch".to_string());
    }
    if intent.challenge_provider_id != "im-receipts" {
        return Err("authentication_run_challenge_provider_unsupported".to_string());
    }
    if site_login_recipe_digest(&intent.site_recipe_id)? != intent.policy_digest {
        return Err("authentication_run_policy_digest_mismatch".to_string());
    }
    Ok(intent)
}

fn exact_handle_binding(
    intent: &AuthenticationRunStartIntent,
    current: &ServiceTabHandle,
) -> Result<AuthenticationRunBinding, String> {
    let supplied = &intent.supplied_handle;
    let checks = [
        (supplied.valid, "supplied_valid"),
        (current.valid, "current_valid"),
        (
            supplied.browser_id == intent.browser_id,
            "requested_browser_id",
        ),
        (
            supplied.session_name.as_deref() == Some(intent.session_name.as_str()),
            "requested_session_name",
        ),
        (
            supplied.profile_id.as_deref() == Some(intent.profile_id.as_str()),
            "requested_profile_id",
        ),
        (supplied.browser_id == current.browser_id, "browser_id"),
        (
            supplied.session_name == current.session_name,
            "session_name",
        ),
        (supplied.tab_id == current.tab_id, "tab_id"),
        (supplied.target_id == current.target_id, "target_id"),
        (supplied.profile_id == current.profile_id, "profile_id"),
        (supplied.lease_id == current.lease_id, "lease_id"),
        (
            supplied.owner_session_id == current.owner_session_id,
            "owner_session_id",
        ),
        (
            current.lease_state == Some(LeaseState::Exclusive),
            "exclusive_lease",
        ),
        (
            current.trace_filter.service_name.as_deref() == Some(&intent.service_name),
            "service_name",
        ),
        (
            current.trace_filter.agent_name.as_deref() == Some(&intent.agent_name),
            "agent_name",
        ),
        (
            current.trace_filter.task_name.as_deref() == Some(&intent.task_name),
            "task_name",
        ),
        (
            current
                .profile_access
                .as_ref()
                .and_then(|access| access.subject_id.as_deref())
                == Some(&intent.principal_id),
            "principal_id",
        ),
    ];
    if let Some((_, axis)) = checks.into_iter().find(|(matched, _)| !matched) {
        return Err(format!(
            "authentication_run_service_tab_handle_mismatch:{axis}"
        ));
    }
    Ok(AuthenticationRunBinding {
        service_id: intent.service_name.clone(),
        agent_id: intent.agent_name.clone(),
        task_id: intent.task_name.clone(),
        principal_id: intent.principal_id.clone(),
        target_service_id: intent.target_service_id.clone(),
        target_account_ref: intent.account_ref.clone(),
        target_organization_ref: intent.organization_ref.clone(),
        challenge_provider_id: intent.challenge_provider_id.clone(),
        challenge_provider_tenant_ref: intent.challenge_provider_tenant_ref.clone(),
        challenge_provider_account_ref: intent.challenge_provider_account_ref.clone(),
        profile_id: current
            .profile_id
            .clone()
            .ok_or_else(|| "authentication_run_profile_missing".to_string())?,
        browser_id: current.browser_id.clone(),
        session_name: current
            .session_name
            .clone()
            .ok_or_else(|| "authentication_run_session_missing".to_string())?,
        login_tab_id: current.tab_id.clone(),
        site_recipe_id: intent.site_recipe_id.clone(),
        policy_digest: intent.policy_digest.clone(),
    })
}

fn start_run_in_state(
    state: &mut ServiceState,
    intent: AuthenticationRunStartIntent,
    created_at: &str,
) -> Result<(ServiceAuthenticationRunRecord, bool), String> {
    let current = state
        .service_tab_handle(&intent.supplied_handle.tab_id)
        .ok_or_else(|| "authentication_run_service_tab_handle_missing".to_string())?;
    let binding = exact_handle_binding(&intent, &current)?;
    let idempotency_key_sha256 = canonical_sha256(&intent.idempotency_key)?;
    let request_sha256 = canonical_sha256(&(
        &binding,
        &idempotency_key_sha256,
        intent.deadline_ms,
        intent.max_transitions,
    ))?;
    if let Some(existing) = state
        .authentication_runs
        .values()
        .find(|record| record.idempotency_key_sha256 == idempotency_key_sha256)
    {
        if existing.request_sha256 != request_sha256 {
            return Err("authentication_run_idempotency_conflict".to_string());
        }
        return Ok((existing.clone(), true));
    }
    let run_id = format!("authrun-{}", &request_sha256[..24]);
    let created = DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| "authentication_run_created_at_invalid".to_string())?
        .with_timezone(&Utc);
    let deadline_delta = i64::try_from(intent.deadline_ms)
        .map_err(|_| "authentication_run_deadline_invalid".to_string())?;
    let deadline_at = (created + Duration::milliseconds(deadline_delta)).to_rfc3339();
    let record = ServiceAuthenticationRunRecord {
        schema_version: SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION.to_string(),
        request_sha256,
        idempotency_key_sha256,
        created_at: created.to_rfc3339(),
        deadline_at,
        service_tab_handle: current,
        pending_effect: None,
        run: AuthenticationRun::new(run_id.clone(), binding, intent.max_transitions)
            .map_err(|error| format!("authentication_run_invalid:{error:?}"))?,
    };
    state.authentication_runs.insert(run_id, record.clone());
    Ok((record, false))
}

fn caller_owns_run(command: &Value, record: &ServiceAuthenticationRunRecord) -> Result<(), String> {
    let binding = &record.run.binding;
    let matches = required_string(command, "serviceName")? == binding.service_id
        && required_string(command, "agentName")? == binding.agent_id
        && required_string(command, "taskName")? == binding.task_id
        && caller_principal(command)? == binding.principal_id;
    if matches {
        Ok(())
    } else {
        Err("authentication_run_caller_mismatch".to_string())
    }
}

fn run_projection(record: &ServiceAuthenticationRunRecord, replayed: bool) -> Value {
    json!({
        "schemaVersion": record.schema_version,
        "runId": record.run.run_id,
        "state": record.run.state,
        "createdAt": record.created_at,
        "deadlineAt": record.deadline_at,
        "requestSha256": record.request_sha256,
        "accountRefSha256": canonical_sha256(&record.run.binding.target_account_ref).ok(),
        "organizationRefSha256": canonical_sha256(&record.run.binding.target_organization_ref).ok(),
        "challengeProviderId": record.run.binding.challenge_provider_id,
        "challengeProviderTenantRefSha256": canonical_sha256(&record.run.binding.challenge_provider_tenant_ref).ok(),
        "challengeProviderAccountRefSha256": canonical_sha256(&record.run.binding.challenge_provider_account_ref).ok(),
        "targetServiceId": record.run.binding.target_service_id,
        "profileId": record.run.binding.profile_id,
        "browserId": record.run.binding.browser_id,
        "sessionName": record.run.binding.session_name,
        "tabId": record.run.binding.login_tab_id,
        "transitionCount": record.run.transition_count,
        "actionReceiptCount": record.run.action_receipts.len() + record.run.site_action_receipts.len(),
        "observationReceiptCount": record.run.site_observation_receipts.len(),
        "effectPending": record.pending_effect.is_some(),
        "replayed": replayed,
    })
}

fn require_live_run(record: &ServiceAuthenticationRunRecord) -> Result<(), String> {
    let deadline = DateTime::parse_from_rfc3339(&record.deadline_at)
        .map_err(|_| "authentication_run_deadline_invalid".to_string())?
        .with_timezone(&Utc);
    if Utc::now() > deadline {
        return Err("authentication_run_deadline_expired".to_string());
    }
    if record.pending_effect.is_some() {
        return Err("authentication_run_effect_outcome_unknown".to_string());
    }
    Ok(())
}

fn exact_current_handle(
    state: &ServiceState,
    record: &ServiceAuthenticationRunRecord,
) -> Result<ServiceTabHandle, String> {
    let current = state
        .service_tab_handle(&record.run.binding.login_tab_id)
        .ok_or_else(|| "authentication_run_service_tab_handle_missing".to_string())?;
    let expected = &record.service_tab_handle;
    let matches = current.valid
        && current.browser_id == expected.browser_id
        && current.session_name == expected.session_name
        && current.tab_id == expected.tab_id
        && current.target_id == expected.target_id
        && current.profile_id == expected.profile_id
        && current.lease_id == expected.lease_id
        && current.owner_session_id == expected.owner_session_id
        && current.lease_state == Some(LeaseState::Exclusive);
    if !matches {
        return Err("authentication_run_service_tab_handle_changed".to_string());
    }
    Ok(current)
}

async fn select_exact_run_target(
    daemon: &mut DaemonState,
    handle: &ServiceTabHandle,
) -> Result<(), String> {
    let handle_value = serde_json::to_value(handle)
        .map_err(|error| format!("authentication_run_handle_encode_failed:{error}"))?;
    let handle_map = handle_value
        .as_object()
        .ok_or_else(|| "authentication_run_handle_invalid".to_string())?;
    validate_service_tab_handle_for_current_session(handle_map, &daemon.session_id)?;
    let target_id = handle
        .target_id
        .as_deref()
        .ok_or_else(|| "authentication_run_target_missing".to_string())?;
    let manager = daemon
        .browser
        .as_mut()
        .ok_or_else(|| "authentication_run_retained_browser_not_running".to_string())?;
    if manager.active_target_id().ok() != Some(target_id) {
        manager.tab_switch_target_id(target_id).await?;
    }
    if manager.active_target_id().ok() != Some(target_id) {
        return Err("authentication_run_target_selection_unproven".to_string());
    }
    Ok(())
}

async fn observe_page(
    daemon: &mut DaemonState,
    recipe: &SiteLoginRecipe,
) -> Result<SitePageEvidence, String> {
    let selectors = json!({
        "identifier": recipe.identifier.field_selectors,
        "password": recipe.password.field_selectors,
        "smsOtp": recipe.sms_otp.field_selectors,
        "buttonLabels": recipe.identifier.submit_labels.iter()
            .chain(recipe.password.submit_labels.iter())
            .chain(recipe.sms_otp.submit_labels.iter())
            .cloned().collect::<Vec<_>>(),
    });
    let selectors_json = serde_json::to_string(&selectors)
        .map_err(|error| format!("authentication_run_probe_encode_failed:{error}"))?;
    let script = format!(
        r#"(() => {{
          const recipe = {selectors_json};
          const visible = (el) => {{
            if (!el) return false;
            const r = el.getBoundingClientRect();
            const s = getComputedStyle(el);
            return r.width > 0 && r.height > 0 && s.display !== 'none' &&
              s.visibility !== 'hidden' && !el.disabled;
          }};
          const first = (items) => items.find((selector) => {{
            try {{ return visible(document.querySelector(selector)); }} catch {{ return false; }}
          }}) || null;
          const allowed = new Set(recipe.buttonLabels);
          const labels = [...document.querySelectorAll('button,input[type=submit],[role=button]')]
            .filter(visible)
            .map((el) => (el.innerText || el.value || el.getAttribute('aria-label') || '').trim())
            .filter((label) => allowed.has(label));
          return {{
            url: location.href,
            title: document.title,
            identifierSelector: first(recipe.identifier),
            passwordSelector: first(recipe.password),
            smsOtpSelector: first(recipe.smsOtp),
            visibleButtonLabels: [...new Set(labels)].sort()
          }};
        }})()"#
    );
    let manager = daemon
        .browser
        .as_ref()
        .ok_or_else(|| "authentication_run_retained_browser_not_running".to_string())?;
    let value = manager.evaluate_with_timeout(&script, 5_000).await?;
    serde_json::from_value(value)
        .map_err(|error| format!("authentication_run_probe_invalid:{error}"))
}

async fn click_exact_label(daemon: &mut DaemonState, labels: &[String]) -> Result<(), String> {
    let labels_json = serde_json::to_string(labels)
        .map_err(|error| format!("authentication_run_labels_encode_failed:{error}"))?;
    let script = format!(
        r#"(() => {{
          const allowed = new Set({labels_json});
          const visible = (el) => {{
            const r = el.getBoundingClientRect(); const s = getComputedStyle(el);
            return r.width > 0 && r.height > 0 && s.display !== 'none' &&
              s.visibility !== 'hidden' && !el.disabled;
          }};
          const matches = [...document.querySelectorAll('button,input[type=submit],[role=button]')]
            .filter(visible)
            .filter((el) => allowed.has((el.innerText || el.value || el.getAttribute('aria-label') || '').trim()));
          if (matches.length !== 1) return {{clicked: false, matchCount: matches.length}};
          matches[0].focus(); matches[0].click();
          return {{clicked: true, matchCount: 1}};
        }})()"#
    );
    let value = daemon
        .browser
        .as_ref()
        .ok_or_else(|| "authentication_run_retained_browser_not_running".to_string())?
        .evaluate_with_timeout(&script, 5_000)
        .await?;
    if value.get("clicked").and_then(Value::as_bool) != Some(true) {
        return Err(format!(
            "authentication_run_submit_control_ambiguous:{}",
            value.get("matchCount").and_then(Value::as_u64).unwrap_or(0)
        ));
    }
    Ok(())
}

async fn fill_and_submit(
    daemon: &mut DaemonState,
    selector: &str,
    value: &str,
    labels: &[String],
) -> Result<(), String> {
    let manager = daemon
        .browser
        .as_ref()
        .ok_or_else(|| "authentication_run_retained_browser_not_running".to_string())?;
    let session_id = manager.active_session_id()?.to_string();
    interaction::fill(
        &manager.client,
        &session_id,
        &daemon.ref_map,
        selector,
        value,
        &daemon.iframe_sessions,
    )
    .await?;
    click_exact_label(daemon, labels).await
}

struct OneShotSiteAction(Result<SiteLoginActionReceipt, AuthenticationActionFailure>);
impl ResponseOnlySiteLoginAction for OneShotSiteAction {
    fn execute(
        &mut self,
        _context: &SiteLoginActionContext<'_>,
    ) -> Result<SiteLoginActionReceipt, AuthenticationActionFailure> {
        std::mem::replace(
            &mut self.0,
            Err(AuthenticationActionFailure::EffectRejected),
        )
    }
}

struct OneShotAuthenticationAction(
    Result<AuthenticationActionReceipt, AuthenticationActionFailure>,
);
impl ResponseOnlyAuthenticationAction for OneShotAuthenticationAction {
    fn execute(
        &mut self,
        _context: &super::authentication_run::AuthenticationActionContext<'_>,
    ) -> Result<AuthenticationActionReceipt, AuthenticationActionFailure> {
        std::mem::replace(
            &mut self.0,
            Err(AuthenticationActionFailure::EffectRejected),
        )
    }
}

struct OneShotVerifier(Result<AuthenticationVerifierReceipt, AuthenticationVerifierFailure>);
impl AuthenticationVerifier for OneShotVerifier {
    fn verify(
        &mut self,
        _context: &AuthenticationVerificationContext<'_>,
    ) -> Result<AuthenticationVerifierReceipt, AuthenticationVerifierFailure> {
        std::mem::replace(
            &mut self.0,
            Err(AuthenticationVerifierFailure::ObservationFailed),
        )
    }
}

fn deterministic_watch_id(run_id: &str) -> String {
    let digest = Sha256::digest(format!("{run_id}:im-receipts:sms-otp").as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes(bytes[0..4].try_into().expect("four bytes")),
        u16::from_be_bytes(bytes[4..6].try_into().expect("two bytes")),
        u16::from_be_bytes(bytes[6..8].try_into().expect("two bytes")),
        u16::from_be_bytes(bytes[8..10].try_into().expect("two bytes")),
        u64::from_be_bytes([
            0, 0, bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ])
    )
}

fn im_receipts_url(binding: &AuthenticationRunBinding, path: &str) -> Result<Url, String> {
    let raw = env::var("IM_RECEIPTS_LOCAL_API_BASE_URL")
        .map_err(|_| "authentication_run_im_receipts_base_url_missing".to_string())?;
    let mut base = Url::parse(&raw)
        .map_err(|_| "authentication_run_im_receipts_base_url_invalid".to_string())?;
    let loopback = base
        .host_str()
        .is_some_and(|host| matches!(host, "127.0.0.1" | "localhost" | "::1"));
    if base.scheme() != "http"
        || !loopback
        || !base.username().is_empty()
        || base.password().is_some()
    {
        return Err("authentication_run_im_receipts_base_url_not_loopback".to_string());
    }
    base.set_path(path);
    base.set_query(None);
    base.query_pairs_mut()
        .append_pair("tenant", &binding.challenge_provider_tenant_ref)
        .append_pair("service", "google_messages")
        .append_pair("account", &binding.challenge_provider_account_ref);
    Ok(base)
}

async fn im_receipts_request(
    method: reqwest::Method,
    binding: &AuthenticationRunBinding,
    path: &str,
    body: Option<Value>,
    sealed: bool,
) -> Result<Value, String> {
    let url = im_receipts_url(binding, path)?;
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(10))
        .build()
        .map_err(|_| "authentication_run_im_receipts_client_failed".to_string())?;
    let mut request = client.request(method, url);
    if let Some(body) = body {
        request = request.json(&body);
    }
    if sealed {
        let capability = env::var("IM_RECEIPTS_SEALED_AUTH_CAPABILITY")
            .map_err(|_| "authentication_run_im_receipts_capability_missing".to_string())?;
        if capability.len() < 32 {
            return Err("authentication_run_im_receipts_capability_invalid".to_string());
        }
        request = request.bearer_auth(capability);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "authentication_run_im_receipts_transport_failed".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "authentication_run_im_receipts_rejected:{}",
            response.status().as_u16()
        ));
    }
    response
        .json::<Value>()
        .await
        .map_err(|_| "authentication_run_im_receipts_response_invalid".to_string())
}

fn watch_from_response(value: &Value) -> Result<&Value, String> {
    value
        .pointer("/data/watch")
        .ok_or_else(|| "authentication_run_im_receipts_watch_missing".to_string())
}

async fn arm_im_receipts_watch(
    record: &ServiceAuthenticationRunRecord,
) -> Result<ProviderWatchReceipt, String> {
    let watch_id = deterministic_watch_id(&record.run.run_id);
    let path = "/v1/sync/urgent-auth-code-watches";
    im_receipts_request(
        reqwest::Method::POST,
        &record.run.binding,
        path,
        Some(json!({
            "watchId": watch_id,
            "supervisorRuntimeMs": 300_000,
        })),
        false,
    )
    .await?;
    let status_path = format!("/v1/sync/urgent-auth-code-watches/{watch_id}");
    loop {
        let status = im_receipts_request(
            reqwest::Method::GET,
            &record.run.binding,
            &status_path,
            None,
            false,
        )
        .await?;
        let watch = watch_from_response(&status)?;
        match watch.get("phase").and_then(Value::as_str) {
            Some("live_armed") => {
                return Ok(ProviderWatchReceipt {
                    provider_id: "im-receipts".to_string(),
                    watch_id: watch_id.clone(),
                    delivery_fence_id: format!("im-watch:{watch_id}"),
                    ready_before_delivery: true,
                });
            }
            Some("terminal") => {
                return Err("authentication_run_im_receipts_watch_terminal_before_ready".to_string())
            }
            _ => tokio::time::sleep(StdDuration::from_millis(250)).await,
        }
        require_live_run(record)?;
    }
}

async fn fence_im_receipts_watch(
    binding: &AuthenticationRunBinding,
    watch_id: &str,
) -> Result<Value, String> {
    im_receipts_request(
        reqwest::Method::POST,
        binding,
        &format!("/v1/sync/urgent-auth-code-watches/{watch_id}/fence"),
        None,
        false,
    )
    .await
}

async fn im_receipts_watch_status(
    binding: &AuthenticationRunBinding,
    watch_id: &str,
) -> Result<Value, String> {
    im_receipts_request(
        reqwest::Method::GET,
        binding,
        &format!("/v1/sync/urgent-auth-code-watches/{watch_id}"),
        None,
        false,
    )
    .await
}

async fn consume_im_receipts_code(
    record: &ServiceAuthenticationRunRecord,
    watch_id: &str,
    delivery_fence_sequence: u64,
) -> Result<String, String> {
    let response = im_receipts_request(
        reqwest::Method::POST,
        &record.run.binding,
        &format!("/v1/internal/sealed-auth/urgent-auth-code-watches/{watch_id}/consume"),
        Some(json!({
            "consumerRef": record.run.run_id,
            "deliveryFenceSequence": delivery_fence_sequence,
        })),
        true,
    )
    .await?;
    response
        .pointer("/data/code")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "authentication_run_im_receipts_code_missing".to_string())
}

fn effect_id(run_id: &str, operation_id: &str, action: AuthenticationActionKind) -> String {
    canonical_sha256(&(run_id, operation_id, action))
        .map(|digest| format!("auth-effect-{}", &digest[..24]))
        .unwrap_or_else(|_| "auth-effect-hash-failed".to_string())
}

fn reserve_effect(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    command: &Value,
    run_id: &str,
    operation_id: &str,
    action: AuthenticationActionKind,
    state_instance_id: &str,
) -> Result<(), String> {
    let reserved_at = service_now_timestamp();
    repository.mutate(|state| {
        let record = state
            .authentication_runs
            .get_mut(run_id)
            .ok_or_else(|| "authentication_run_not_found".to_string())?;
        caller_owns_run(command, record)?;
        require_live_run(record)?;
        if record.run.used_operation_ids.contains(operation_id) {
            return Err("authentication_run_operation_replay".to_string());
        }
        record.pending_effect = Some(PendingAuthenticationEffect {
            operation_id: operation_id.to_string(),
            action,
            state_instance_id: state_instance_id.to_string(),
            reserved_at: reserved_at.clone(),
        });
        Ok(())
    })
}

async fn resume_authentication_run(
    command: &Value,
    daemon: &mut DaemonState,
) -> Result<Value, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    let run_id = required_string(command, "authenticationRunId")?;
    let operation_id = required_string(command, "operationId")?;
    let snapshot = repository.load_snapshot()?;
    let record = snapshot
        .authentication_runs
        .get(&run_id)
        .cloned()
        .ok_or_else(|| "authentication_run_not_found".to_string())?;
    caller_owns_run(command, &record)?;
    require_live_run(&record)?;
    let current_handle = exact_current_handle(&snapshot, &record)?;
    let recipe = load_site_login_recipe(&record.run.binding.site_recipe_id)?;
    if site_login_recipe_digest(&recipe.recipe_id)? != record.run.binding.policy_digest {
        return Err("authentication_run_policy_digest_changed".to_string());
    }
    select_exact_run_target(daemon, &current_handle).await?;

    match record.run.state {
        AuthenticationRunState::Ready => {
            if record
                .run
                .site_observation_receipts
                .last()
                .is_some_and(|receipt| receipt.site_state == SiteLoginState::SmsOtpForm)
            {
                return Err("authentication_run_sms_provider_unavailable".to_string());
            }
            let evidence = observe_page(daemon, &recipe).await?;
            let classified = classify_site_page(
                &recipe,
                &record.run.binding.target_organization_ref,
                &evidence,
            )?;
            let receipt = SiteLoginObservationReceipt {
                observer_id: "agent-browser.bill-login-v1.page-observer".to_string(),
                target_service_id: record.run.binding.target_service_id.clone(),
                target_account_ref: record.run.binding.target_account_ref.clone(),
                profile_id: record.run.binding.profile_id.clone(),
                browser_id: record.run.binding.browser_id.clone(),
                session_name: record.run.binding.session_name.clone(),
                tab_id: record.run.binding.login_tab_id.clone(),
                origin_verified: classified.origin_verified,
                page_state_fresh: true,
                site_state: classified.state,
                state_instance_id: classified.state_instance_id,
                exact_account_authenticated: classified.exact_organization_authenticated,
                browser_chrome_semantics_used: false,
            };
            let updated = repository.mutate(|state| {
                let current = state
                    .authentication_runs
                    .get_mut(&run_id)
                    .ok_or_else(|| "authentication_run_not_found".to_string())?;
                caller_owns_run(command, current)?;
                require_live_run(current)?;
                current
                    .run
                    .observe_site_login_state(&operation_id, receipt.clone())
                    .map_err(|error| format!("authentication_run_observe_failed:{error:?}"))?;
                Ok(current.clone())
            })?;
            Ok(run_projection(&updated, false))
        }
        AuthenticationRunState::AwaitingIdentifier => {
            let evidence = observe_page(daemon, &recipe).await?;
            let classified = classify_site_page(
                &recipe,
                &record.run.binding.target_organization_ref,
                &evidence,
            )?;
            let expected = record
                .run
                .site_observation_receipts
                .last()
                .ok_or_else(|| "authentication_run_site_observation_missing".to_string())?;
            if classified.state != SiteLoginState::IdentifierForm
                || classified.state_instance_id != expected.state_instance_id
            {
                return Err("authentication_run_site_state_changed".to_string());
            }
            let selector = evidence
                .identifier_selector
                .ok_or_else(|| "authentication_run_identifier_field_missing".to_string())?;
            let credentials = auth::credentials_get_full(&record.run.binding.target_account_ref)?;
            reserve_effect(
                &repository,
                command,
                &run_id,
                &operation_id,
                AuthenticationActionKind::SubmitAccountIdentifier,
                &classified.state_instance_id,
            )?;
            let outcome = fill_and_submit(
                daemon,
                &selector,
                &credentials.username,
                &recipe.identifier.submit_labels,
            )
            .await;
            let action_result = outcome.map(|_| SiteLoginActionReceipt {
                provider_id: "agent-browser.bill-login-v1.vault".to_string(),
                effect_id: effect_id(
                    &run_id,
                    &operation_id,
                    AuthenticationActionKind::SubmitAccountIdentifier,
                ),
                action: AuthenticationActionKind::SubmitAccountIdentifier,
                state_instance_id: classified.state_instance_id.clone(),
                response_only_material_consumption: true,
                secret_material_exposed: false,
                browser_chrome_semantics_used: false,
                persistence_verified: false,
            });
            complete_site_action(
                &repository,
                command,
                &run_id,
                &operation_id,
                AuthenticationActionKind::SubmitAccountIdentifier,
                action_result,
            )
        }
        AuthenticationRunState::AwaitingPassword => {
            let evidence = observe_page(daemon, &recipe).await?;
            let classified = classify_site_page(
                &recipe,
                &record.run.binding.target_organization_ref,
                &evidence,
            )?;
            let expected = record
                .run
                .site_observation_receipts
                .last()
                .ok_or_else(|| "authentication_run_site_observation_missing".to_string())?;
            if classified.state != SiteLoginState::PasswordForm
                || classified.state_instance_id != expected.state_instance_id
            {
                return Err("authentication_run_site_state_changed".to_string());
            }
            let watch = arm_im_receipts_watch(&record).await?;
            let challenge_id = format!("sms:{}", watch.watch_id);
            let updated = repository.mutate(|state| {
                let current = state
                    .authentication_runs
                    .get_mut(&run_id)
                    .ok_or_else(|| "authentication_run_not_found".to_string())?;
                caller_owns_run(command, current)?;
                require_live_run(current)?;
                current
                    .run
                    .prepare_watch(
                        &operation_id,
                        &challenge_id,
                        AuthenticationChallengeChannel::SmsOtp,
                        watch.clone(),
                    )
                    .map_err(|error| {
                        format!("authentication_run_watch_prepare_failed:{error:?}")
                    })?;
                Ok(current.clone())
            })?;
            Ok(run_projection(&updated, false))
        }
        AuthenticationRunState::ObservingDelivery => {
            let (challenge_id, channel, watch, delivery_triggered) = record
                .run
                .active_challenge_binding()
                .ok_or_else(|| "authentication_run_active_challenge_missing".to_string())?;
            if channel != AuthenticationChallengeChannel::SmsOtp || delivery_triggered {
                return Err("authentication_run_active_challenge_invalid".to_string());
            }
            let evidence = observe_page(daemon, &recipe).await?;
            let classified = classify_site_page(
                &recipe,
                &record.run.binding.target_organization_ref,
                &evidence,
            )?;
            let expected = record
                .run
                .site_observation_receipts
                .last()
                .ok_or_else(|| "authentication_run_site_observation_missing".to_string())?;
            if classified.state != SiteLoginState::PasswordForm
                || classified.state_instance_id != expected.state_instance_id
            {
                return Err("authentication_run_site_state_changed".to_string());
            }
            let selector = evidence
                .password_selector
                .ok_or_else(|| "authentication_run_password_field_missing".to_string())?;
            let credentials = auth::credentials_get_full(&record.run.binding.target_account_ref)?;
            reserve_effect(
                &repository,
                command,
                &run_id,
                &operation_id,
                AuthenticationActionKind::SubmitNativeStoredCredentials,
                &classified.state_instance_id,
            )?;
            let outcome = match fill_and_submit(
                daemon,
                &selector,
                &credentials.password,
                &recipe.password.submit_labels,
            )
            .await
            {
                Ok(()) => fence_im_receipts_watch(&record.run.binding, &watch.watch_id)
                    .await
                    .map(|_| ()),
                Err(error) => Err(error),
            };
            let action_result = outcome.map(|_| AuthenticationActionReceipt {
                provider_id: "agent-browser.bill-login-v1.vault+im-receipts".to_string(),
                effect_id: effect_id(
                    &run_id,
                    &operation_id,
                    AuthenticationActionKind::SubmitNativeStoredCredentials,
                ),
                action: AuthenticationActionKind::SubmitNativeStoredCredentials,
                native_credential_store_used: true,
                credentials_replayed: false,
                challenge_material_consumed: false,
                response_only_material_consumption: true,
                delivery_watch_ready_before_trigger: true,
                delivery_fence_id: Some(watch.delivery_fence_id.clone()),
                candidate_count: None,
                same_profile_new_tab: None,
            });
            complete_delivery_trigger_action(
                &repository,
                command,
                &run_id,
                &operation_id,
                &challenge_id,
                action_result,
            )
        }
        AuthenticationRunState::AwaitingCandidate => {
            let (challenge_id, channel, watch, delivery_triggered) = record
                .run
                .active_challenge_binding()
                .ok_or_else(|| "authentication_run_active_challenge_missing".to_string())?;
            if channel != AuthenticationChallengeChannel::SmsOtp || !delivery_triggered {
                return Err("authentication_run_active_challenge_invalid".to_string());
            }
            let status = im_receipts_watch_status(&record.run.binding, &watch.watch_id).await?;
            let status_watch = watch_from_response(&status)?;
            if status_watch.get("phase").and_then(Value::as_str) != Some("terminal") {
                return Err("authentication_run_sms_candidate_pending".to_string());
            }
            if status_watch.get("terminalReason").and_then(Value::as_str) != Some("candidate_found")
                || status_watch.get("candidateCount").and_then(Value::as_u64) != Some(1)
            {
                return Err("authentication_run_sms_candidate_not_unique".to_string());
            }
            let delivery_fence_sequence = status_watch
                .pointer("/cursors/deliveryFenceSequence")
                .and_then(Value::as_u64)
                .ok_or_else(|| "authentication_run_sms_delivery_fence_missing".to_string())?;
            let evidence = observe_page(daemon, &recipe).await?;
            let classified = classify_site_page(
                &recipe,
                &record.run.binding.target_organization_ref,
                &evidence,
            )?;
            if classified.state != SiteLoginState::SmsOtpForm {
                return Err("authentication_run_sms_form_missing".to_string());
            }
            let selector = evidence
                .sms_otp_selector
                .ok_or_else(|| "authentication_run_sms_field_missing".to_string())?;
            reserve_effect(
                &repository,
                command,
                &run_id,
                &operation_id,
                AuthenticationActionKind::SubmitSmsOtp,
                &classified.state_instance_id,
            )?;
            let outcome =
                match consume_im_receipts_code(&record, &watch.watch_id, delivery_fence_sequence)
                    .await
                {
                    Ok(code) => {
                        fill_and_submit(daemon, &selector, &code, &recipe.sms_otp.submit_labels)
                            .await
                    }
                    Err(error) => Err(error),
                };
            let action_result = outcome.map(|_| AuthenticationActionReceipt {
                provider_id: "im-receipts+agent-browser.bill-login-v1".to_string(),
                effect_id: effect_id(
                    &run_id,
                    &operation_id,
                    AuthenticationActionKind::SubmitSmsOtp,
                ),
                action: AuthenticationActionKind::SubmitSmsOtp,
                native_credential_store_used: false,
                credentials_replayed: false,
                challenge_material_consumed: true,
                response_only_material_consumption: true,
                delivery_watch_ready_before_trigger: true,
                delivery_fence_id: Some(watch.delivery_fence_id.clone()),
                candidate_count: Some(1),
                same_profile_new_tab: None,
            });
            complete_challenge_action(
                &repository,
                command,
                &run_id,
                &operation_id,
                &challenge_id,
                action_result,
            )
        }
        AuthenticationRunState::Verifying => {
            let evidence = observe_page(daemon, &recipe).await?;
            let classified = classify_site_page(
                &recipe,
                &record.run.binding.target_organization_ref,
                &evidence,
            )?;
            if classified.state != SiteLoginState::Authenticated
                || !classified.exact_organization_authenticated
            {
                return Err("authentication_run_verification_pending".to_string());
            }
            let receipt = Ok(AuthenticationVerifierReceipt {
                verifier_id: "agent-browser.bill-login-v1.company-route".to_string(),
                target_service_id: record.run.binding.target_service_id.clone(),
                target_account_ref: record.run.binding.target_account_ref.clone(),
                profile_id: record.run.binding.profile_id.clone(),
                browser_id: record.run.binding.browser_id.clone(),
                session_name: record.run.binding.session_name.clone(),
                exact_target_authenticated: true,
            });
            let (updated, transition_error) = repository.mutate(|state| {
                let current = state
                    .authentication_runs
                    .get_mut(&run_id)
                    .ok_or_else(|| "authentication_run_not_found".to_string())?;
                caller_owns_run(command, current)?;
                let mut verifier = OneShotVerifier(receipt.clone());
                let result = current
                    .run
                    .verify_exact_target(&operation_id, &mut verifier);
                Ok((current.clone(), result.err()))
            })?;
            if let Some(error) = transition_error {
                return Err(format!("authentication_run_verify_failed:{error:?}"));
            }
            Ok(run_projection(&updated, false))
        }
        AuthenticationRunState::Authenticated
        | AuthenticationRunState::OperatorInterventionRequired
        | AuthenticationRunState::Blocked
        | AuthenticationRunState::Failed
        | AuthenticationRunState::Cancelled => Ok(run_projection(&record, true)),
        AuthenticationRunState::AwaitingPasswordManagerDecision => {
            Err("authentication_run_browser_chrome_provider_unavailable".to_string())
        }
    }
}

fn complete_site_action(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    command: &Value,
    run_id: &str,
    operation_id: &str,
    expected_action: AuthenticationActionKind,
    outcome: Result<SiteLoginActionReceipt, String>,
) -> Result<Value, String> {
    let adapter_outcome = outcome.map_err(|_| AuthenticationActionFailure::EffectUnproven);
    let (updated, transition_error) = repository.mutate(|state| {
        let current = state
            .authentication_runs
            .get_mut(run_id)
            .ok_or_else(|| "authentication_run_not_found".to_string())?;
        caller_owns_run(command, current)?;
        let pending = current
            .pending_effect
            .take()
            .ok_or_else(|| "authentication_run_pending_effect_missing".to_string())?;
        if pending.operation_id != operation_id || pending.action != expected_action {
            return Err("authentication_run_pending_effect_mismatch".to_string());
        }
        let mut adapter = OneShotSiteAction(adapter_outcome.clone());
        let result = current
            .run
            .submit_account_identifier(operation_id, &mut adapter);
        Ok((current.clone(), result.err()))
    })?;
    if let Some(error) = transition_error {
        return Err(format!("authentication_run_site_action_failed:{error:?}"));
    }
    Ok(run_projection(&updated, false))
}

fn complete_delivery_trigger_action(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    command: &Value,
    run_id: &str,
    operation_id: &str,
    challenge_id: &str,
    outcome: Result<AuthenticationActionReceipt, String>,
) -> Result<Value, String> {
    let adapter_outcome = outcome.map_err(|_| AuthenticationActionFailure::EffectUnproven);
    let (updated, transition_error) = repository.mutate(|state| {
        let current = state
            .authentication_runs
            .get_mut(run_id)
            .ok_or_else(|| "authentication_run_not_found".to_string())?;
        caller_owns_run(command, current)?;
        let pending = current
            .pending_effect
            .take()
            .ok_or_else(|| "authentication_run_pending_effect_missing".to_string())?;
        if pending.operation_id != operation_id
            || pending.action != AuthenticationActionKind::SubmitNativeStoredCredentials
        {
            return Err("authentication_run_pending_effect_mismatch".to_string());
        }
        let mut adapter = OneShotAuthenticationAction(adapter_outcome.clone());
        let result = current.run.submit_credentials_and_trigger_delivery(
            operation_id,
            challenge_id,
            &mut adapter,
        );
        Ok((current.clone(), result.err()))
    })?;
    if let Some(error) = transition_error {
        return Err(format!(
            "authentication_run_credential_action_failed:{error:?}"
        ));
    }
    Ok(run_projection(&updated, false))
}

fn complete_challenge_action(
    repository: &LockedServiceStateRepository<JsonServiceStateStore>,
    command: &Value,
    run_id: &str,
    operation_id: &str,
    challenge_id: &str,
    outcome: Result<AuthenticationActionReceipt, String>,
) -> Result<Value, String> {
    let adapter_outcome = outcome.map_err(|_| AuthenticationActionFailure::EffectUnproven);
    let (updated, transition_error) = repository.mutate(|state| {
        let current = state
            .authentication_runs
            .get_mut(run_id)
            .ok_or_else(|| "authentication_run_not_found".to_string())?;
        caller_owns_run(command, current)?;
        let pending = current
            .pending_effect
            .take()
            .ok_or_else(|| "authentication_run_pending_effect_missing".to_string())?;
        if pending.operation_id != operation_id
            || pending.action != AuthenticationActionKind::SubmitSmsOtp
        {
            return Err("authentication_run_pending_effect_mismatch".to_string());
        }
        let mut adapter = OneShotAuthenticationAction(adapter_outcome.clone());
        let result = current
            .run
            .consume_challenge(operation_id, challenge_id, 1, &mut adapter);
        Ok((current.clone(), result.err()))
    })?;
    if let Some(error) = transition_error {
        return Err(format!(
            "authentication_run_challenge_action_failed:{error:?}"
        ));
    }
    Ok(run_projection(&updated, false))
}

pub(crate) async fn handle_service_authentication_run(
    command: &Value,
    daemon: &mut DaemonState,
) -> Result<Value, String> {
    let action = command
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| "authentication_run_action_required".to_string())?;
    let repository = LockedServiceStateRepository::default_json()?;
    match action {
        "service_authentication_run_start" => {
            let intent = parse_start_intent(command)?;
            let created_at = service_now_timestamp();
            let (record, replayed) =
                repository.mutate(|state| start_run_in_state(state, intent, &created_at))?;
            Ok(run_projection(&record, replayed))
        }
        "service_authentication_run_status" => {
            let run_id = required_string(command, "authenticationRunId")?;
            let state = repository.load_snapshot()?;
            let record = state
                .authentication_runs
                .get(&run_id)
                .ok_or_else(|| "authentication_run_not_found".to_string())?;
            caller_owns_run(command, record)?;
            Ok(run_projection(record, false))
        }
        "service_authentication_run_cancel" => {
            let run_id = required_string(command, "authenticationRunId")?;
            let operation_id = required_string(command, "operationId")?;
            let record = repository.mutate(|state| {
                let record = state
                    .authentication_runs
                    .get_mut(&run_id)
                    .ok_or_else(|| "authentication_run_not_found".to_string())?;
                caller_owns_run(command, record)?;
                record
                    .run
                    .cancel(&operation_id)
                    .map_err(|error| format!("authentication_run_cancel_failed:{error:?}"))?;
                Ok(record.clone())
            })?;
            Ok(run_projection(&record, false))
        }
        "service_authentication_run_resume" => resume_authentication_run(command, daemon).await,
        "service_authentication_recipe_status" => {
            let recipe_id = required_string(command, "siteRecipeId")?;
            let target_service_id = required_string(command, "targetServiceId")?;
            let recipe = load_site_login_recipe(&recipe_id)?;
            if recipe.target_service_id != target_service_id {
                return Err("authentication_run_recipe_target_mismatch".to_string());
            }
            Ok(json!({
                "schemaVersion": recipe.schema_version,
                "siteRecipeId": recipe.recipe_id,
                "targetServiceId": recipe.target_service_id,
                "policyDigest": site_login_recipe_digest(&recipe_id)?,
            }))
        }
        _ => Err(format!("authentication_run_action_unsupported:{action}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserHealth, BrowserProcess, BrowserSession, BrowserTab, ProfileOrigin,
        ServiceTabHandleTraceFilter, TabLifecycle,
    };
    use crate::native::service_profile_access_policy::{
        ProfileChildAccess, ProfileConnectionState, ProfileIdentityAssurance,
    };

    fn handle() -> ServiceTabHandle {
        ServiceTabHandle {
            browser_id: "browser-1".to_string(),
            session_name: Some("session-1".to_string()),
            tab_id: "tab-1".to_string(),
            target_id: Some("target-1".to_string()),
            profile_id: Some("profile-1".to_string()),
            profile_origin: ProfileOrigin::AgentBrowserOwned,
            lease_id: Some("session-1".to_string()),
            lease_state: Some(LeaseState::Exclusive),
            owner_session_id: Some("session-1".to_string()),
            profile_access: Some(ProfileChildAccess {
                schema_version: "agent-browser.profile-child-access.v1".to_string(),
                parent_policy_revision: 1,
                access_decision_id: "decision-1".to_string(),
                subject_id: Some("principal-1".to_string()),
                identity_assurance: ProfileIdentityAssurance::AuthenticatedIngress,
                connection_instance_id: Some("connection-1".to_string()),
                connection_state: ProfileConnectionState::Active,
                permissions: Vec::new(),
            }),
            trace_filter: ServiceTabHandleTraceFilter {
                browser_id: Some("browser-1".to_string()),
                profile_id: Some("profile-1".to_string()),
                session_id: Some("session-1".to_string()),
                service_name: Some("books-receipts".to_string()),
                agent_name: Some("closeout-worker".to_string()),
                task_name: Some("bill-auth".to_string()),
            },
            valid: true,
            ..ServiceTabHandle::default()
        }
    }

    fn service_state() -> ServiceState {
        let handle = handle();
        let mut state = ServiceState::default();
        state.browsers.insert(
            "browser-1".to_string(),
            BrowserProcess {
                id: "browser-1".to_string(),
                profile_id: Some("profile-1".to_string()),
                health: BrowserHealth::Ready,
                active_session_ids: vec!["session-1".to_string()],
                ..BrowserProcess::default()
            },
        );
        state.sessions.insert(
            "session-1".to_string(),
            BrowserSession {
                id: "session-1".to_string(),
                service_name: Some("books-receipts".to_string()),
                agent_name: Some("closeout-worker".to_string()),
                task_name: Some("bill-auth".to_string()),
                profile_id: Some("profile-1".to_string()),
                lease: LeaseState::Exclusive,
                ..BrowserSession::default()
            },
        );
        state.tabs.insert(
            "tab-1".to_string(),
            BrowserTab {
                id: "tab-1".to_string(),
                browser_id: "browser-1".to_string(),
                target_id: Some("target-1".to_string()),
                session_id: Some("session-1".to_string()),
                lifecycle: TabLifecycle::Ready,
                owner_session_id: Some("session-1".to_string()),
                profile_access: handle.profile_access,
                ..BrowserTab::default()
            },
        );
        state
    }

    fn start_command(state: &ServiceState) -> Value {
        let current = state.service_tab_handle("tab-1").unwrap();
        json!({
            "action": "service_authentication_run_start",
            "serviceName": "books-receipts",
            "agentName": "closeout-worker",
            "taskName": "bill-auth",
            "clientSubjectId": "principal-1",
            "targetServiceId": "bill",
            "accountRef": "opaque-account-1",
            "organizationRef": "opaque-organization-1",
            "challengeProviderId": "im-receipts",
            "challengeProviderTenantRef": "opaque-tenant-1",
            "challengeProviderAccountRef": "opaque-provider-account-1",
            "profileId": "profile-1",
            "browserId": "browser-1",
            "sessionName": "session-1",
            "siteRecipeId": "bill-login-v1",
            "policyDigest": site_login_recipe_digest("bill-login-v1").unwrap(),
            "idempotencyKey": "auth-idempotency-1",
            "deadlineMs": 120000,
            "maxTransitions": 32,
            "serviceTabHandle": current,
        })
    }

    #[test]
    fn start_is_exact_handle_bound_idempotent_and_secret_free() {
        let mut state = service_state();
        let command = start_command(&state);
        let intent = parse_start_intent(&command).unwrap();
        let (first, replayed) =
            start_run_in_state(&mut state, intent, "2026-09-10T12:00:00Z").unwrap();
        assert!(!replayed);
        let replay_intent = parse_start_intent(&command).unwrap();
        let (second, replayed) =
            start_run_in_state(&mut state, replay_intent, "2026-09-10T12:01:00Z").unwrap();
        assert!(replayed);
        assert_eq!(first, second);
        assert_eq!(state.authentication_runs.len(), 1);
        let projection = run_projection(&first, false).to_string();
        assert!(!projection.contains("opaque-account-1"));
        assert!(!projection.contains("auth-idempotency-1"));

        let encoded = serde_json::to_value(&state).unwrap();
        let decoded: ServiceState = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.authentication_runs, state.authentication_runs);
    }

    #[test]
    fn idempotency_key_reuse_with_changed_request_is_rejected() {
        let mut state = service_state();
        let command = start_command(&state);
        start_run_in_state(
            &mut state,
            parse_start_intent(&command).unwrap(),
            "2026-09-10T12:00:00Z",
        )
        .unwrap();

        let mut changed = command;
        changed["deadlineMs"] = json!(121000);
        assert_eq!(
            start_run_in_state(
                &mut state,
                parse_start_intent(&changed).unwrap(),
                "2026-09-10T12:01:00Z",
            )
            .unwrap_err(),
            "authentication_run_idempotency_conflict"
        );
        assert_eq!(state.authentication_runs.len(), 1);
    }

    #[test]
    fn wrong_principal_or_stale_handle_fails_before_record_creation() {
        let mut state = service_state();
        let mut wrong_principal = start_command(&state);
        wrong_principal["clientSubjectId"] = json!("different-principal");
        assert_eq!(
            start_run_in_state(
                &mut state,
                parse_start_intent(&wrong_principal).unwrap(),
                "2026-09-10T12:00:00Z",
            )
            .unwrap_err(),
            "authentication_run_service_tab_handle_mismatch:principal_id"
        );
        let mut stale = start_command(&state);
        stale["serviceTabHandle"]["valid"] = json!(false);
        assert_eq!(
            start_run_in_state(
                &mut state,
                parse_start_intent(&stale).unwrap(),
                "2026-09-10T12:00:00Z",
            )
            .unwrap_err(),
            "authentication_run_service_tab_handle_mismatch:supplied_valid"
        );
        assert!(state.authentication_runs.is_empty());
    }

    #[test]
    fn forbidden_secret_and_generic_input_fields_are_rejected() {
        let state = service_state();
        for field in ["username", "password", "otp", "uiAction", "selector"] {
            let mut command = start_command(&state);
            command[field] = json!("synthetic-secret-canary");
            assert_eq!(
                parse_start_intent(&command).unwrap_err(),
                format!("authentication_run_forbidden_field:{field}")
            );
        }
    }
}

pub(crate) fn authentication_run_map_is_empty(
    value: &BTreeMap<String, ServiceAuthenticationRunRecord>,
) -> bool {
    value.is_empty()
}
