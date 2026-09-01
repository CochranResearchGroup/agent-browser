use super::{
    LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION,
    LEASE_AUTHORITY_PROTOCOL_RESPONSE_SCHEMA_VERSION,
};
use crate::native::service_lease_authority::{LeaseEffectAuthorization, LeaseResourceKey};
use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::time::Duration;

const PROTECTED_LEASE_AUTHORITY_CLIENT_TIMEOUT: Duration = Duration::from_secs(2);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProtectedEffectCompletion {
    Completed,
    Uncertain,
}

impl ProtectedEffectCompletion {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Uncertain => "uncertain",
        }
    }
}

pub(crate) fn authorize_protected_browser_launch(
    request: &ProtectedBrowserLaunchRequest,
) -> Result<ProtectedBrowserLaunchPermit, String> {
    let encoded = encode_protected_browser_launch_request(request)?;
    let response = exchange_with_protected_lease_authority(&encoded)?;
    decode_protected_browser_launch_response(&response, request)
}

pub(crate) fn complete_protected_browser_launch(
    permit: &ProtectedBrowserLaunchPermit,
    completion: ProtectedEffectCompletion,
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
            "result": completion.as_str(),
            "completionEvidenceDigest": completion_evidence_digest,
            "completionIdempotencyKey": completion_idempotency_key,
        }
    }))
    .map_err(|_| "lease_authority_effect_completion_encode_failed".to_string())?;
    let response = exchange_with_protected_lease_authority(&encoded)?;
    decode_protected_browser_launch_completion(
        &response,
        permit,
        completion,
        completion_evidence_digest,
        completion_idempotency_key,
    )
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

fn decode_protected_browser_launch_completion(
    encoded: &[u8],
    permit: &ProtectedBrowserLaunchPermit,
    completion: ProtectedEffectCompletion,
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
    let expected_outcome = match completion {
        ProtectedEffectCompletion::Completed => "effect_completed",
        ProtectedEffectCompletion::Uncertain => "effect_uncertain",
    };
    if response.get("outcome").and_then(Value::as_str) != Some(expected_outcome)
        || response
            .pointer("/payload/receipt/receiptId")
            .and_then(Value::as_str)
            != Some(permit.receipt_id.as_str())
        || response
            .pointer("/payload/receipt/state")
            .and_then(Value::as_str)
            != Some(completion.as_str())
    {
        return Err("lease_authority_effect_completion_response_mismatch".to_string());
    }
    Ok(())
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
        decode_protected_browser_launch_completion(
            &serde_json::to_vec(&completion).unwrap(),
            &permit,
            ProtectedEffectCompletion::Uncertain,
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "complete:launch:last30days:tick-1",
        )
        .unwrap();
    }
}
