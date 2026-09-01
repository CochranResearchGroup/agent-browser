use super::{
    LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION,
    LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
};
use crate::native::service_lease_authority::{
    ActiveLeaseClaim, LeaseClaimMode, LeaseEffectAuthorization, LeaseResourceKey,
};
use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::time::Duration;

const PROTECTED_LEASE_AUTHORITY_CLIENT_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) struct ProtectedProfileEnrollmentRequest {
    pub(crate) raw_capability: String,
    pub(crate) profile_id: String,
    pub(crate) profile_path: String,
    pub(crate) idempotency_key: String,
}

impl std::fmt::Debug for ProtectedProfileEnrollmentRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtectedProfileEnrollmentRequest")
            .field("raw_capability", &"[REDACTED]")
            .field("profile_id", &self.profile_id)
            .field("profile_path", &"[REDACTED]")
            .field("idempotency_key", &self.idempotency_key)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProtectedProfileEnrollment {
    pub(crate) principal_id: String,
    pub(crate) capability_id: String,
    pub(crate) capability_revision: u64,
    pub(crate) resource_revision: u64,
}

pub(crate) struct ProtectedEphemeralProfileClaimRequest {
    pub(crate) raw_capability: String,
    pub(crate) profile_id: String,
    pub(crate) idempotency_key: String,
}

impl std::fmt::Debug for ProtectedEphemeralProfileClaimRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtectedEphemeralProfileClaimRequest")
            .field("raw_capability", &"[REDACTED]")
            .field("profile_id", &self.profile_id)
            .field("idempotency_key", &self.idempotency_key)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProtectedEphemeralProfileClaim {
    pub(crate) resource: LeaseResourceKey,
    pub(crate) claim_id: String,
    pub(crate) principal_id: String,
    pub(crate) capability_id: String,
    pub(crate) capability_revision: u64,
    pub(crate) claim_revision: u64,
    pub(crate) fencing_token: u64,
    pub(crate) expires_at: String,
}

pub(crate) struct ProtectedBrowserLaunchRequest {
    pub(crate) raw_capability: String,
    pub(crate) resource: LeaseResourceKey,
    pub(crate) claim_id: String,
    pub(crate) claim_revision: u64,
    pub(crate) fencing_token: u64,
    pub(crate) audience: String,
    pub(crate) idempotency_key: String,
}

impl std::fmt::Debug for ProtectedBrowserLaunchRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtectedBrowserLaunchRequest")
            .field("raw_capability", &"[REDACTED]")
            .field("resource", &self.resource)
            .field("claim_id", &self.claim_id)
            .field("claim_revision", &self.claim_revision)
            .field("fencing_token", &self.fencing_token)
            .field("audience", &self.audience)
            .field("idempotency_key", &self.idempotency_key)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProtectedBrowserLaunchPermit {
    pub(crate) receipt_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProtectedBrowserOwner {
    pub(crate) owner_id: String,
    pub(crate) owner_generation: u64,
    pub(crate) logical_browser_id: String,
    pub(crate) daemon_session_route: String,
    pub(crate) process_instance_digest: String,
    pub(crate) process_pid: u32,
    pub(crate) revision: u64,
}

pub(crate) struct ProtectedBrowserOwnerReconciliationRequest {
    pub(crate) raw_capability: String,
    pub(crate) profile_id: String,
    pub(crate) expected_owner_id: String,
    pub(crate) expected_owner_generation: u64,
    pub(crate) idempotency_key: String,
}

impl std::fmt::Debug for ProtectedBrowserOwnerReconciliationRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtectedBrowserOwnerReconciliationRequest")
            .field("raw_capability", &"[REDACTED]")
            .field("profile_id", &self.profile_id)
            .field("expected_owner_id", &self.expected_owner_id)
            .field("expected_owner_generation", &self.expected_owner_generation)
            .field("idempotency_key", &self.idempotency_key)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProtectedBrowserOwnerReconciliation {
    pub(crate) receipt_id: String,
    pub(crate) owner_id: String,
    pub(crate) owner_generation: u64,
    pub(crate) evidence_digest: String,
    pub(crate) authority_revision: u64,
    pub(crate) owner_revision: u64,
    pub(crate) replayed: bool,
}

pub(crate) fn enroll_protected_profile(
    request: &ProtectedProfileEnrollmentRequest,
) -> Result<ProtectedProfileEnrollment, String> {
    let encoded = encode_protected_profile_enrollment_request(request)?;
    let response = exchange_with_protected_lease_authority(&encoded)?;
    decode_protected_profile_enrollment_response(&response, request)
}

pub(crate) fn acquire_protected_ephemeral_profile_claim(
    request: &ProtectedEphemeralProfileClaimRequest,
) -> Result<ProtectedEphemeralProfileClaim, String> {
    let encoded = encode_protected_ephemeral_profile_claim_request(request)?;
    let response = exchange_with_protected_lease_authority(&encoded)?;
    decode_protected_ephemeral_profile_claim_response(&response, request)
}

pub(crate) fn authorize_protected_browser_launch(
    request: &ProtectedBrowserLaunchRequest,
) -> Result<ProtectedBrowserLaunchPermit, String> {
    let encoded = encode_protected_browser_launch_request(request)?;
    let response = exchange_with_protected_lease_authority(&encoded)?;
    decode_protected_browser_launch_response(&response, request)
}

pub(crate) fn mark_protected_browser_launch_uncertain(
    permit: &ProtectedBrowserLaunchPermit,
    completion_evidence_digest: &str,
    completion_idempotency_key: &str,
) -> Result<(), String> {
    if permit.receipt_id.trim().is_empty()
        || !super::valid_sha256_digest(completion_evidence_digest)
        || completion_idempotency_key.trim().is_empty()
    {
        return Err("lease_authority_effect_completion_invalid".to_string());
    }
    let encoded = serde_json::to_vec(&serde_json::json!({
        "schemaVersion": LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION,
        "operation": "complete_effect",
        "payload": {
            "receiptId": permit.receipt_id,
            "result": "uncertain",
            "completionEvidenceDigest": completion_evidence_digest,
            "completionIdempotencyKey": completion_idempotency_key,
        }
    }))
    .map_err(|_| "lease_authority_effect_completion_encode_failed".to_string())?;
    let response = exchange_with_protected_lease_authority(&encoded)?;
    decode_protected_browser_launch_uncertainty(
        &response,
        permit,
        completion_evidence_digest,
        completion_idempotency_key,
    )
}

pub(crate) fn complete_protected_browser_launch_success(
    permit: &ProtectedBrowserLaunchPermit,
    browser_pid: u32,
    completion_idempotency_key: &str,
) -> Result<ProtectedBrowserOwner, String> {
    if permit.receipt_id.trim().is_empty()
        || browser_pid <= 1
        || completion_idempotency_key.trim().is_empty()
    {
        return Err("lease_authority_browser_launch_completion_invalid".to_string());
    }
    let encoded = serde_json::to_vec(&serde_json::json!({
        "schemaVersion": LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION,
        "operation": "complete_browser_launch",
        "payload": {
            "receiptId": permit.receipt_id,
            "browserPid": browser_pid,
            "completionIdempotencyKey": completion_idempotency_key,
        }
    }))
    .map_err(|_| "lease_authority_browser_launch_completion_encode_failed".to_string())?;
    let response = exchange_with_protected_lease_authority(&encoded)?;
    decode_protected_browser_launch_success(&response, permit, browser_pid)
}

pub(crate) fn reconcile_protected_browser_owner(
    request: &ProtectedBrowserOwnerReconciliationRequest,
) -> Result<ProtectedBrowserOwnerReconciliation, String> {
    let encoded = encode_protected_browser_owner_reconciliation_request(request)?;
    let response = exchange_with_protected_lease_authority(&encoded)?;
    decode_protected_browser_owner_reconciliation_response(&response, request)
}

fn exchange_with_protected_lease_authority(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let state_root = super::service::fixed_state_root();
    let socket_path = super::service::fixed_socket_path();
    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|_| "lease_authority_service_unavailable".to_string())?;
    stream
        .set_read_timeout(Some(PROTECTED_LEASE_AUTHORITY_CLIENT_TIMEOUT))
        .and_then(|_| stream.set_write_timeout(Some(PROTECTED_LEASE_AUTHORITY_CLIENT_TIMEOUT)))
        .map_err(|_| "lease_authority_service_timeout_configuration_failed".to_string())?;
    let socket_group_id = std::fs::symlink_metadata(&socket_path)
        .map_err(|_| "lease_authority_service_socket_identity_unavailable".to_string())?
        .gid();
    super::custody::inspect_linux_authority_endpoint(
        &state_root,
        &socket_path,
        &stream,
        socket_group_id,
    )
    .map_err(|_| "lease_authority_service_identity_unproven".to_string())?;
    exchange_framed(&mut stream, encoded)
}

fn exchange_framed<S: Read + Write>(stream: &mut S, encoded: &[u8]) -> Result<Vec<u8>, String> {
    super::write_lease_authority_frame(stream, encoded)
        .map_err(|error| error.code().to_string())?;
    super::read_lease_authority_frame(stream).map_err(|error| error.code().to_string())
}

fn encode_protected_profile_enrollment_request(
    request: &ProtectedProfileEnrollmentRequest,
) -> Result<Vec<u8>, String> {
    if request.raw_capability.trim().is_empty()
        || crate::runtime_profile::validate_runtime_profile_name(&request.profile_id).is_err()
        || request.profile_path.trim().is_empty()
        || request.idempotency_key.trim().is_empty()
    {
        return Err("lease_authority_profile_enrollment_request_invalid".to_string());
    }
    serde_json::to_vec(&serde_json::json!({
        "schemaVersion": LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION,
        "operation": "enroll_profile",
        "payload": {
            "rawCapability": request.raw_capability.as_bytes(),
            "profileId": request.profile_id,
            "profilePath": request.profile_path,
            "expectedResourceRevision": 0,
            "idempotencyKey": request.idempotency_key,
        }
    }))
    .map_err(|_| "lease_authority_profile_enrollment_request_encode_failed".to_string())
}

fn decode_protected_profile_enrollment_response(
    encoded: &[u8],
    request: &ProtectedProfileEnrollmentRequest,
) -> Result<ProtectedProfileEnrollment, String> {
    let response = decode_success_response(
        encoded,
        "profile_enrolled",
        "lease_authority_profile_enrollment",
    )?;
    let receipt = response
        .pointer("/payload/receipt")
        .ok_or_else(|| "lease_authority_profile_enrollment_receipt_invalid".to_string())?;
    let profile_id = required_response_string(
        receipt,
        "profileId",
        "lease_authority_profile_enrollment_receipt_invalid",
    )?;
    let principal_id = required_response_string(
        receipt,
        "principalId",
        "lease_authority_profile_enrollment_receipt_invalid",
    )?;
    let capability_id = required_response_string(
        receipt,
        "capabilityId",
        "lease_authority_profile_enrollment_receipt_invalid",
    )?;
    let capability_revision = required_response_u64(
        receipt,
        "capabilityRevision",
        "lease_authority_profile_enrollment_receipt_invalid",
    )?;
    let resource_revision = required_response_u64(
        receipt,
        "resourceRevision",
        "lease_authority_profile_enrollment_receipt_invalid",
    )?;
    if profile_id != request.profile_id {
        return Err("lease_authority_profile_enrollment_receipt_mismatch".to_string());
    }
    Ok(ProtectedProfileEnrollment {
        principal_id,
        capability_id,
        capability_revision,
        resource_revision,
    })
}

fn encode_protected_ephemeral_profile_claim_request(
    request: &ProtectedEphemeralProfileClaimRequest,
) -> Result<Vec<u8>, String> {
    if request.raw_capability.trim().is_empty()
        || crate::runtime_profile::validate_runtime_profile_name(&request.profile_id).is_err()
        || request.idempotency_key.trim().is_empty()
    {
        return Err("lease_authority_acquire_request_invalid".to_string());
    }
    serde_json::to_vec(&serde_json::json!({
        "schemaVersion": LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION,
        "operation": "acquire",
        "payload": {
            "rawCapability": request.raw_capability.as_bytes(),
            "resource": LeaseResourceKey::profile(&request.profile_id),
            "parentClaimId": null,
            "mode": "ephemeral",
            "idempotencyKey": request.idempotency_key,
            "recoveryControllerId": null,
        }
    }))
    .map_err(|_| "lease_authority_acquire_request_encode_failed".to_string())
}

fn decode_protected_ephemeral_profile_claim_response(
    encoded: &[u8],
    request: &ProtectedEphemeralProfileClaimRequest,
) -> Result<ProtectedEphemeralProfileClaim, String> {
    let response = decode_success_response(encoded, "acquired", "lease_authority_acquire")?;
    let claim_value = response
        .pointer("/payload/claim")
        .filter(|value| !value.is_null())
        .ok_or_else(|| "lease_authority_acquisition_replay_without_current_claim".to_string())?;
    let claim: ActiveLeaseClaim = serde_json::from_value(claim_value.clone())
        .map_err(|_| "lease_authority_acquire_claim_invalid".to_string())?;
    let expected_resource = LeaseResourceKey::profile(&request.profile_id);
    if claim.resource != expected_resource || claim.mode() != LeaseClaimMode::Ephemeral {
        return Err("lease_authority_acquire_claim_mismatch".to_string());
    }
    Ok(ProtectedEphemeralProfileClaim {
        resource: claim.resource.clone(),
        claim_id: claim.claim_id().to_string(),
        principal_id: claim.principal_id().to_string(),
        capability_id: claim.capability_id().to_string(),
        capability_revision: claim.capability_revision,
        claim_revision: claim.revision(),
        fencing_token: claim.fencing_token(),
        expires_at: claim.expires_at().to_string(),
    })
}

fn decode_success_response(
    encoded: &[u8],
    expected_outcome: &str,
    error_prefix: &str,
) -> Result<Value, String> {
    let response: Value =
        serde_json::from_slice(encoded).map_err(|_| format!("{error_prefix}_response_invalid"))?;
    if response.get("schemaVersion").and_then(Value::as_str)
        != Some(LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION)
    {
        return Err(format!("{error_prefix}_response_schema_invalid"));
    }
    if response.get("outcome").and_then(Value::as_str) == Some("error") {
        return Err(response
            .pointer("/error/code")
            .and_then(Value::as_str)
            .unwrap_or("lease_authority_request_failed")
            .to_string());
    }
    if response.get("outcome").and_then(Value::as_str) != Some(expected_outcome) {
        return Err(format!("{error_prefix}_response_outcome_invalid"));
    }
    Ok(response)
}

fn required_response_string(value: &Value, field: &str, code: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| code.to_string())
}

fn required_response_u64(value: &Value, field: &str, code: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| code.to_string())
}

fn encode_protected_browser_launch_request(
    request: &ProtectedBrowserLaunchRequest,
) -> Result<Vec<u8>, String> {
    if request.raw_capability.trim().is_empty()
        || request.claim_id.trim().is_empty()
        || request.claim_revision == 0
        || request.fencing_token == 0
        || request.audience.trim().is_empty()
        || request.idempotency_key.trim().is_empty()
    {
        return Err("lease_authority_effect_request_invalid".to_string());
    }
    serde_json::to_vec(&serde_json::json!({
        "schemaVersion": LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION,
        "operation": "authorize_effect",
        "payload": {
            "rawCapability": request.raw_capability.as_bytes(),
            "resource": request.resource,
            "claimId": request.claim_id,
            "claimRevision": request.claim_revision,
            "fencingToken": request.fencing_token,
            "actionClass": "browser_launch",
            "audience": request.audience,
            "idempotencyKey": request.idempotency_key,
        }
    }))
    .map_err(|_| "lease_authority_effect_request_encode_failed".to_string())
}

fn decode_protected_browser_launch_response(
    encoded: &[u8],
    request: &ProtectedBrowserLaunchRequest,
) -> Result<ProtectedBrowserLaunchPermit, String> {
    let response: Value = serde_json::from_slice(encoded)
        .map_err(|_| "lease_authority_effect_response_invalid".to_string())?;
    if response.get("schemaVersion").and_then(Value::as_str)
        != Some(LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION)
    {
        return Err("lease_authority_effect_response_schema_invalid".to_string());
    }
    if response.get("outcome").and_then(Value::as_str) == Some("error") {
        return Err(response
            .pointer("/error/code")
            .and_then(Value::as_str)
            .unwrap_or("lease_authority_effect_request_failed")
            .to_string());
    }
    if response.get("outcome").and_then(Value::as_str) != Some("effect_authorized") {
        return Err("lease_authority_effect_response_outcome_invalid".to_string());
    }
    let payload = response
        .get("payload")
        .ok_or_else(|| "lease_authority_effect_response_invalid".to_string())?;
    if payload.get("replayed").and_then(Value::as_bool) != Some(false)
        || payload.get("authorization").is_none_or(Value::is_null)
    {
        return Err("lease_authority_effect_uncertain_inspect_before_retry".to_string());
    }
    let authorization: LeaseEffectAuthorization =
        serde_json::from_value(payload["authorization"].clone())
            .map_err(|_| "lease_authority_effect_authorization_invalid".to_string())?;
    if authorization.resource != request.resource
        || authorization.claim_id != request.claim_id
        || authorization.claim_revision != request.claim_revision
        || authorization.fencing_token != request.fencing_token
        || authorization.action_class != "browser_launch"
        || authorization.audience != request.audience
        || authorization.operation_idempotency_key != request.idempotency_key
    {
        return Err("lease_authority_effect_authorization_mismatch".to_string());
    }
    let receipt = payload
        .get("receipt")
        .ok_or_else(|| "lease_authority_effect_receipt_invalid".to_string())?;
    if receipt.get("state").and_then(Value::as_str) != Some("consumed") {
        return Err("lease_authority_effect_receipt_invalid".to_string());
    }
    let receipt_id = receipt
        .get("receiptId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "lease_authority_effect_receipt_invalid".to_string())?;
    Ok(ProtectedBrowserLaunchPermit {
        receipt_id: receipt_id.to_string(),
    })
}

fn decode_protected_browser_launch_uncertainty(
    encoded: &[u8],
    permit: &ProtectedBrowserLaunchPermit,
    _completion_evidence_digest: &str,
    _completion_idempotency_key: &str,
) -> Result<(), String> {
    let response: Value = serde_json::from_slice(encoded)
        .map_err(|_| "lease_authority_effect_completion_response_invalid".to_string())?;
    if response.get("schemaVersion").and_then(Value::as_str)
        != Some(LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION)
    {
        return Err("lease_authority_effect_completion_schema_invalid".to_string());
    }
    if response.get("outcome").and_then(Value::as_str) == Some("error") {
        return Err(response
            .pointer("/error/code")
            .and_then(Value::as_str)
            .unwrap_or("lease_authority_effect_completion_failed")
            .to_string());
    }
    if response.get("outcome").and_then(Value::as_str) != Some("effect_uncertain")
        || response
            .pointer("/payload/receipt/receiptId")
            .and_then(Value::as_str)
            != Some(permit.receipt_id.as_str())
        || response
            .pointer("/payload/receipt/state")
            .and_then(Value::as_str)
            != Some("uncertain")
    {
        return Err("lease_authority_effect_completion_response_mismatch".to_string());
    }
    Ok(())
}

fn decode_protected_browser_launch_success(
    encoded: &[u8],
    permit: &ProtectedBrowserLaunchPermit,
    browser_pid: u32,
) -> Result<ProtectedBrowserOwner, String> {
    let response = decode_success_response(
        encoded,
        "browser_launch_completed",
        "lease_authority_browser_launch_completion",
    )?;
    if response
        .pointer("/payload/receipt/receiptId")
        .and_then(Value::as_str)
        != Some(permit.receipt_id.as_str())
        || response
            .pointer("/payload/receipt/state")
            .and_then(Value::as_str)
            != Some("completed")
    {
        return Err("lease_authority_browser_launch_completion_response_mismatch".to_string());
    }
    let owner = response
        .pointer("/payload/owner")
        .ok_or_else(|| "lease_authority_browser_launch_owner_invalid".to_string())?;
    let process_pid = owner
        .get("processPid")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 1)
        .ok_or_else(|| "lease_authority_browser_launch_owner_invalid".to_string())?;
    let process_instance_digest = required_response_string(
        owner,
        "processInstanceDigest",
        "lease_authority_browser_launch_owner_invalid",
    )?;
    if process_pid != browser_pid || !super::valid_sha256_digest(&process_instance_digest) {
        return Err("lease_authority_browser_launch_owner_mismatch".to_string());
    }
    Ok(ProtectedBrowserOwner {
        owner_id: required_response_string(
            owner,
            "ownerId",
            "lease_authority_browser_launch_owner_invalid",
        )?,
        owner_generation: required_response_u64(
            owner,
            "ownerGeneration",
            "lease_authority_browser_launch_owner_invalid",
        )?,
        logical_browser_id: required_response_string(
            owner,
            "logicalBrowserId",
            "lease_authority_browser_launch_owner_invalid",
        )?,
        daemon_session_route: required_response_string(
            owner,
            "daemonSessionRoute",
            "lease_authority_browser_launch_owner_invalid",
        )?,
        process_instance_digest,
        process_pid,
        revision: required_response_u64(
            owner,
            "revision",
            "lease_authority_browser_launch_owner_invalid",
        )?,
    })
}

fn encode_protected_browser_owner_reconciliation_request(
    request: &ProtectedBrowserOwnerReconciliationRequest,
) -> Result<Vec<u8>, String> {
    if request.raw_capability.trim().is_empty()
        || crate::runtime_profile::validate_runtime_profile_name(&request.profile_id).is_err()
        || request.expected_owner_id.trim().is_empty()
        || request.expected_owner_generation == 0
        || request.idempotency_key.trim().is_empty()
    {
        return Err("lease_authority_owner_reconciliation_request_invalid".to_string());
    }
    serde_json::to_vec(&serde_json::json!({
        "schemaVersion": LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION,
        "operation": "reconcile_browser_owner",
        "payload": {
            "rawCapability": request.raw_capability.as_bytes(),
            "resource": LeaseResourceKey::profile(&request.profile_id),
            "expectedOwnerId": request.expected_owner_id,
            "expectedOwnerGeneration": request.expected_owner_generation,
            "idempotencyKey": request.idempotency_key,
        }
    }))
    .map_err(|_| "lease_authority_owner_reconciliation_request_encode_failed".to_string())
}

fn decode_protected_browser_owner_reconciliation_response(
    encoded: &[u8],
    request: &ProtectedBrowserOwnerReconciliationRequest,
) -> Result<ProtectedBrowserOwnerReconciliation, String> {
    let response = decode_success_response(
        encoded,
        "browser_owner_reconciled",
        "lease_authority_owner_reconciliation",
    )?;
    let receipt = response
        .pointer("/payload/receipt")
        .ok_or_else(|| "lease_authority_owner_reconciliation_receipt_invalid".to_string())?;
    let resource: LeaseResourceKey = serde_json::from_value(
        receipt
            .get("resource")
            .cloned()
            .ok_or_else(|| "lease_authority_owner_reconciliation_receipt_invalid".to_string())?,
    )
    .map_err(|_| "lease_authority_owner_reconciliation_receipt_invalid".to_string())?;
    let owner_id = required_response_string(
        receipt,
        "ownerId",
        "lease_authority_owner_reconciliation_receipt_invalid",
    )?;
    let owner_generation = required_response_u64(
        receipt,
        "ownerGeneration",
        "lease_authority_owner_reconciliation_receipt_invalid",
    )?;
    let evidence_digest = required_response_string(
        receipt,
        "evidenceDigest",
        "lease_authority_owner_reconciliation_receipt_invalid",
    )?;
    if resource != LeaseResourceKey::profile(&request.profile_id)
        || owner_id != request.expected_owner_id
        || owner_generation != request.expected_owner_generation
        || !super::valid_sha256_digest(&evidence_digest)
    {
        return Err("lease_authority_owner_reconciliation_receipt_mismatch".to_string());
    }
    Ok(ProtectedBrowserOwnerReconciliation {
        receipt_id: required_response_string(
            receipt,
            "receiptId",
            "lease_authority_owner_reconciliation_receipt_invalid",
        )?,
        owner_id,
        owner_generation,
        evidence_digest,
        authority_revision: required_response_u64(
            receipt,
            "authorityRevision",
            "lease_authority_owner_reconciliation_receipt_invalid",
        )?,
        owner_revision: required_response_u64(
            receipt,
            "ownerRevision",
            "lease_authority_owner_reconciliation_receipt_invalid",
        )?,
        replayed: response
            .pointer("/payload/replayed")
            .and_then(Value::as_bool)
            .ok_or_else(|| "lease_authority_owner_reconciliation_receipt_invalid".to_string())?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_lease_authority::LeaseResourceKey;

    fn request() -> ProtectedBrowserLaunchRequest {
        ProtectedBrowserLaunchRequest {
            raw_capability: "capability-secret".to_string(),
            resource: LeaseResourceKey::profile("last30days-social"),
            claim_id: "claim-1".to_string(),
            claim_revision: 3,
            fencing_token: 7,
            audience: "daemon-session:last30days".to_string(),
            idempotency_key: "launch:last30days:tick-1".to_string(),
        }
    }

    #[test]
    fn ordinary_profile_enrollment_and_acquisition_require_no_lease_choreography() {
        let enrollment_request = ProtectedProfileEnrollmentRequest {
            raw_capability: "capability-secret".to_string(),
            profile_id: "last30days-social".to_string(),
            profile_path: "/private/profile/path".to_string(),
            idempotency_key: "enroll:last30days:v1".to_string(),
        };
        let encoded = encode_protected_profile_enrollment_request(&enrollment_request).unwrap();
        let encoded: Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(encoded["operation"], "enroll_profile");
        assert_eq!(encoded["payload"]["expectedResourceRevision"], 0);
        assert!(encoded["payload"].get("principalId").is_none());
        assert!(encoded["payload"].get("physicalIdentityDigest").is_none());
        assert!(encoded["payload"].get("operatorUid").is_none());
        assert!(encoded["payload"].get("occurredAt").is_none());
        let debug = format!("{enrollment_request:?}");
        assert!(!debug.contains("capability-secret"));
        assert!(!debug.contains("/private/profile/path"));

        let enrollment_response = serde_json::json!({
            "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
            "outcome": "profile_enrolled",
            "payload": {
                "receipt": {
                    "profileId": "last30days-social",
                    "principalId": "principal:local-uid:1000:profile:last30days-social",
                    "capabilityId": "capability:last30days-social",
                    "capabilityRevision": 1,
                    "resourceRevision": 1
                },
                "replayed": false
            }
        });
        let enrollment = decode_protected_profile_enrollment_response(
            &serde_json::to_vec(&enrollment_response).unwrap(),
            &enrollment_request,
        )
        .unwrap();
        assert_eq!(enrollment.resource_revision, 1);

        let acquire_request = ProtectedEphemeralProfileClaimRequest {
            raw_capability: "capability-secret".to_string(),
            profile_id: "last30days-social".to_string(),
            idempotency_key: "acquire:last30days:worker-2".to_string(),
        };
        let encoded = encode_protected_ephemeral_profile_claim_request(&acquire_request).unwrap();
        let encoded: Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(encoded["operation"], "acquire");
        assert_eq!(encoded["payload"]["mode"], "ephemeral");
        assert!(encoded["payload"].get("expectedClaimRevision").is_none());
        assert!(encoded["payload"].get("expiresAt").is_none());
        assert!(encoded["payload"].get("ownerGeneration").is_none());
        assert!(encoded["payload"].get("sessionName").is_none());
        assert!(!format!("{acquire_request:?}").contains("capability-secret"));

        let acquired_response = serde_json::json!({
            "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
            "outcome": "acquired",
            "payload": {
                "claim": {
                    "schemaVersion": "agent-browser.lease-authority.v1",
                    "claimId": "lease-claim-v1:abc",
                    "resource": {"kind": "profile", "id": "last30days-social"},
                    "parentClaimId": null,
                    "principalId": "principal:local-uid:1000:profile:last30days-social",
                    "capabilityId": "capability:last30days-social",
                    "capabilityRevision": 1,
                    "mode": "ephemeral",
                    "revision": 1,
                    "fencingToken": 7,
                    "idempotencyKey": "acquire:last30days:worker-1",
                    "acquiredAt": "2026-09-01T12:00:00Z",
                    "heartbeatAt": "2026-09-01T12:00:00Z",
                    "expiresAt": "2026-09-01T12:05:00Z",
                    "transitionDeadline": null,
                    "recoveryControllerId": null,
                    "bootEpoch": "boot-1",
                    "ownerGeneration": null
                },
                "receipt": {},
                "replayed": false
            }
        });
        let claim = decode_protected_ephemeral_profile_claim_response(
            &serde_json::to_vec(&acquired_response).unwrap(),
            &acquire_request,
        )
        .unwrap();
        assert_eq!(claim.fencing_token, 7);
        assert_eq!(claim.expires_at, "2026-09-01T12:05:00Z");

        let expired_replay = serde_json::json!({
            "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
            "outcome": "acquired",
            "payload": {"claim": null, "receipt": {}, "replayed": true}
        });
        assert_eq!(
            decode_protected_ephemeral_profile_claim_response(
                &serde_json::to_vec(&expired_replay).unwrap(),
                &acquire_request,
            ),
            Err("lease_authority_acquisition_replay_without_current_claim".to_string())
        );
    }

    #[test]
    fn protected_launch_request_is_closed_and_replay_is_not_effect_capable() {
        let encoded = encode_protected_browser_launch_request(&request()).unwrap();
        let encoded: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(encoded["operation"], "authorize_effect");
        assert_eq!(
            encoded["payload"]["rawCapability"],
            serde_json::json!(b"capability-secret")
        );
        assert!(encoded["payload"].get("issuedAt").is_none());
        assert!(encoded["payload"].get("executorUid").is_none());
        assert!(encoded["payload"].get("executorIdentityDigest").is_none());

        let replay = serde_json::json!({
            "schemaVersion": "agent-browser.lease-authority-response.v1",
            "outcome": "effect_authorized",
            "payload": {
                "authorization": null,
                "receipt": {
                    "receiptId": "effect-receipt:abc",
                    "state": "consumed"
                },
                "replayed": true
            }
        });
        let error = decode_protected_browser_launch_response(
            &serde_json::to_vec(&replay).unwrap(),
            &request(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            "lease_authority_effect_uncertain_inspect_before_retry"
        );
        let debug = format!("{:?}", request());
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("capability-secret"));
    }

    #[test]
    fn owner_reconciliation_request_contains_no_caller_process_assertion() {
        let request = ProtectedBrowserOwnerReconciliationRequest {
            raw_capability: "capability-secret".to_string(),
            profile_id: "last30days-social".to_string(),
            expected_owner_id: "owner:abc".to_string(),
            expected_owner_generation: 7,
            idempotency_key: "reconcile:last30days:owner-7".to_string(),
        };
        let encoded = encode_protected_browser_owner_reconciliation_request(&request).unwrap();
        let encoded: Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(encoded["operation"], "reconcile_browser_owner");
        assert!(encoded["payload"].get("processPid").is_none());
        assert!(encoded["payload"].get("processInstanceDigest").is_none());
        assert!(encoded["payload"].get("evidenceDigest").is_none());
        assert!(encoded["payload"].get("observedAt").is_none());
        assert!(!format!("{request:?}").contains("capability-secret"));

        let response = serde_json::json!({
            "schemaVersion": LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
            "outcome": "browser_owner_reconciled",
            "payload": {
                "receipt": {
                    "receiptId": "owner-reconciliation:abc",
                    "resource": {"kind": "profile", "id": "last30days-social"},
                    "ownerId": "owner:abc",
                    "ownerGeneration": 7,
                    "evidenceDigest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "authorityRevision": 9,
                    "ownerRevision": 8
                },
                "replayed": false
            }
        });
        let outcome = decode_protected_browser_owner_reconciliation_response(
            &serde_json::to_vec(&response).unwrap(),
            &request,
        )
        .unwrap();
        assert!(!outcome.replayed);
        assert_eq!(outcome.owner_generation, 7);
    }

    #[test]
    fn first_launch_delivery_and_exact_completion_are_typed() {
        let request = request();
        let response = serde_json::json!({
            "schemaVersion": "agent-browser.lease-authority-response.v1",
            "outcome": "effect_authorized",
            "payload": {
                "authorization": {
                    "schemaVersion": "agent-browser.lease-effect-authorization.v5",
                    "signingKeyId": "lease-authority-ed25519-verification-key-v1:key",
                    "signingKeyEpoch": 1,
                    "resource": {"kind": "profile", "id": "last30days-social"},
                    "claimId": "claim-1",
                    "principalId": "principal-1",
                    "capabilityId": "capability-1",
                    "capabilityRevision": 1,
                    "claimRevision": 3,
                    "fencingToken": 7,
                    "ownerGeneration": null,
                    "executorIdentityDigest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "actionClass": "browser_launch",
                    "audience": "daemon-session:last30days",
                    "operationIdempotencyKey": "launch:last30days:tick-1",
                    "issuedAt": "2026-09-01T12:00:00Z",
                    "authorizationExpiresAt": "2026-09-01T12:01:00Z",
                    "proof": "00"
                },
                "receipt": {
                    "receiptId": "effect-receipt:abc",
                    "state": "consumed"
                },
                "replayed": false
            }
        });
        let permit = decode_protected_browser_launch_response(
            &serde_json::to_vec(&response).unwrap(),
            &request,
        )
        .unwrap();
        assert_eq!(permit.receipt_id, "effect-receipt:abc");

        let completion = serde_json::json!({
            "schemaVersion": "agent-browser.lease-authority-response.v1",
            "outcome": "effect_uncertain",
            "payload": {
                "receipt": {
                    "receiptId": "effect-receipt:abc",
                    "state": "uncertain"
                },
                "replayed": false
            }
        });
        decode_protected_browser_launch_uncertainty(
            &serde_json::to_vec(&completion).unwrap(),
            &permit,
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "complete:launch:last30days:tick-1",
        )
        .unwrap();

        let completed = serde_json::json!({
            "schemaVersion": "agent-browser.lease-authority-response.v1",
            "outcome": "browser_launch_completed",
            "payload": {
                "receipt": {
                    "receiptId": "effect-receipt:abc",
                    "state": "completed"
                },
                "owner": {
                    "ownerId": "owner:abc",
                    "ownerGeneration": 1,
                    "logicalBrowserId": "browser:abc",
                    "daemonSessionRoute": "last30days",
                    "processInstanceDigest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                    "processPid": 4242,
                    "revision": 1
                },
                "replayed": false
            }
        });
        let owner = decode_protected_browser_launch_success(
            &serde_json::to_vec(&completed).unwrap(),
            &permit,
            4242,
        )
        .unwrap();
        assert_eq!(owner.owner_id, "owner:abc");
        assert_eq!(owner.logical_browser_id, "browser:abc");
    }
}
