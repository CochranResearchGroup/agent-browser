use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::{
    AcquireLeaseClaimRequest, LeaseAuthorityState, LeaseClaimAcquisitionOutcome, LeaseClaimMode,
    LeaseResourceKey, LeaseResourceKind,
};
use crate::native::service_principal::{authenticate_profile_capability, ServicePrincipalRegistry};

const LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-request.v1";
const LEASE_AUTHORITY_PROTECTED_STATE_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-protected-state.v1";
const LEASE_AUTHORITY_RESOURCE_REGISTRY_SCHEMA_VERSION: &str =
    "agent-browser.lease-authority-resource-registry.v1";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityProtocolEnvelope {
    schema_version: String,
    operation: String,
    payload: Value,
}

#[derive(Deserialize, PartialEq, Eq)]
struct LeaseAuthoritySecret(Vec<u8>);

impl LeaseAuthoritySecret {
    fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for LeaseAuthoritySecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl Drop for LeaseAuthoritySecret {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AcquireLeaseAuthorityPayload {
    raw_capability: LeaseAuthoritySecret,
    resource: LeaseResourceKey,
    parent_claim_id: Option<String>,
    mode: LeaseClaimMode,
    expected_authority_revision: u64,
    idempotency_key: String,
    now: String,
    expires_at: String,
    transition_deadline: Option<String>,
    recovery_controller_id: Option<String>,
    boot_epoch: Option<String>,
    owner_generation: Option<u64>,
}

#[derive(PartialEq, Eq)]
enum LeaseAuthorityProtocolRequest {
    Acquire(Box<AcquireLeaseAuthorityPayload>),
    AuthorizeEffect(Value),
    Release(Value),
    Recover(Value),
    Revoke(Value),
    Inspect(Value),
}

enum LeaseAuthorityProtocolResponse {
    Acquired(LeaseClaimAcquisitionOutcome),
}

struct LeaseAuthorityProtocolKernel {
    state: LeaseAuthorityProtectedState,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityProtectedState {
    schema_version: String,
    authority: LeaseAuthorityState,
    principals: ServicePrincipalRegistry,
    resources: LeaseAuthorityResourceRegistry,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityResourceRegistration {
    resource: LeaseResourceKey,
    physical_identity_digest: String,
    revision: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaseAuthorityResourceRegistry {
    schema_version: String,
    revision: u64,
    registrations: BTreeMap<String, LeaseAuthorityResourceRegistration>,
}

impl LeaseAuthorityProtocolKernel {
    fn new(authority: LeaseAuthorityState, principals: ServicePrincipalRegistry) -> Self {
        Self {
            state: LeaseAuthorityProtectedState {
                schema_version: LEASE_AUTHORITY_PROTECTED_STATE_SCHEMA_VERSION.to_string(),
                authority,
                principals,
                resources: LeaseAuthorityResourceRegistry {
                    schema_version: LEASE_AUTHORITY_RESOURCE_REGISTRY_SCHEMA_VERSION.to_string(),
                    revision: 0,
                    registrations: BTreeMap::new(),
                },
            },
        }
    }

    fn bootstrap_profile_resource(
        &mut self,
        profile_id: &str,
        physical_identity_digest: &str,
    ) -> Result<(), LeaseAuthorityProtocolError> {
        if profile_id.trim().is_empty()
            || !valid_sha256_digest(physical_identity_digest)
            || self
                .state
                .resources
                .registrations
                .values()
                .any(|registration| {
                    registration.physical_identity_digest == physical_identity_digest
                        && registration.resource.id != profile_id
                })
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_resource_registration_invalid",
            });
        }
        let resource = LeaseResourceKey::profile(profile_id);
        let storage_key = resource.storage_key();
        if let Some(existing) = self.state.resources.registrations.get(&storage_key) {
            return (existing.resource == resource
                && existing.physical_identity_digest == physical_identity_digest)
                .then_some(())
                .ok_or(LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_resource_registration_conflict",
                });
        }
        let revision = self
            .state
            .resources
            .revision
            .checked_add(1)
            .filter(|revision| *revision > 0)
            .ok_or(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_resource_revision_exhausted",
            })?;
        self.state.resources.registrations.insert(
            storage_key,
            LeaseAuthorityResourceRegistration {
                resource,
                physical_identity_digest: physical_identity_digest.to_string(),
                revision,
            },
        );
        self.state.resources.revision = revision;
        Ok(())
    }

    fn encode_protected_state(&self) -> Result<Vec<u8>, LeaseAuthorityProtocolError> {
        serde_json::to_vec_pretty(&self.state).map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_state_encode_failed",
        })
    }

    fn from_protected_state(encoded: &[u8]) -> Result<Self, LeaseAuthorityProtocolError> {
        let state: LeaseAuthorityProtectedState =
            serde_json::from_slice(encoded).map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_state_invalid",
            })?;
        if state.schema_version != LEASE_AUTHORITY_PROTECTED_STATE_SCHEMA_VERSION {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_state_schema_unsupported",
            });
        }
        validate_protected_state(&state)?;
        Ok(Self { state })
    }

    fn execute(
        &mut self,
        request: LeaseAuthorityProtocolRequest,
    ) -> Result<LeaseAuthorityProtocolResponse, LeaseAuthorityProtocolError> {
        match request {
            LeaseAuthorityProtocolRequest::Acquire(request) => self.acquire(*request),
            LeaseAuthorityProtocolRequest::AuthorizeEffect(_)
            | LeaseAuthorityProtocolRequest::Release(_)
            | LeaseAuthorityProtocolRequest::Recover(_)
            | LeaseAuthorityProtocolRequest::Revoke(_)
            | LeaseAuthorityProtocolRequest::Inspect(_) => Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_operation_not_implemented",
            }),
        }
    }

    fn acquire(
        &mut self,
        request: AcquireLeaseAuthorityPayload,
    ) -> Result<LeaseAuthorityProtocolResponse, LeaseAuthorityProtocolError> {
        if request.resource.kind != LeaseResourceKind::Profile {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_resource_unsupported",
            });
        }
        if !self
            .state
            .resources
            .registrations
            .contains_key(&request.resource.storage_key())
        {
            return Err(LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_resource_unregistered",
            });
        }
        let raw_capability =
            std::str::from_utf8(request.raw_capability.expose()).map_err(|_| {
                LeaseAuthorityProtocolError {
                    code: "lease_authority_protocol_capability_invalid",
                }
            })?;
        let authenticated = authenticate_profile_capability(
            &self.state.principals,
            raw_capability,
            Some(&request.resource.id),
        )
        .map_err(|error| LeaseAuthorityProtocolError {
            code: error.code.as_str(),
        })?;
        let outcome = self
            .state
            .authority
            .acquire_with_receipt(AcquireLeaseClaimRequest {
                resource: request.resource,
                parent_claim_id: request.parent_claim_id,
                principal_id: authenticated.principal_id,
                capability_id: authenticated.capability_id,
                capability_revision: authenticated.capability_revision,
                mode: request.mode,
                expected_authority_revision: request.expected_authority_revision,
                idempotency_key: request.idempotency_key,
                now: request.now,
                expires_at: request.expires_at,
                transition_deadline: request.transition_deadline,
                recovery_controller_id: request.recovery_controller_id,
                boot_epoch: request.boot_epoch,
                owner_generation: request.owner_generation,
            })
            .map_err(|error| LeaseAuthorityProtocolError {
                code: error.as_str(),
            })?;
        Ok(LeaseAuthorityProtocolResponse::Acquired(outcome))
    }
}

fn validate_protected_state(
    state: &LeaseAuthorityProtectedState,
) -> Result<(), LeaseAuthorityProtocolError> {
    let invalid = || LeaseAuthorityProtocolError {
        code: "lease_authority_protocol_state_invalid",
    };
    if state.resources.schema_version != LEASE_AUTHORITY_RESOURCE_REGISTRY_SCHEMA_VERSION {
        return Err(invalid());
    }

    let mut physical_identities = BTreeSet::new();
    let mut highest_revision = 0;
    for (storage_key, registration) in &state.resources.registrations {
        if registration.resource.kind != LeaseResourceKind::Profile
            || storage_key != &registration.resource.storage_key()
            || !valid_sha256_digest(&registration.physical_identity_digest)
            || registration.revision == 0
            || registration.revision > state.resources.revision
            || !physical_identities.insert(&registration.physical_identity_digest)
        {
            return Err(invalid());
        }
        highest_revision = highest_revision.max(registration.revision);
    }

    if highest_revision != state.resources.revision {
        return Err(invalid());
    }
    Ok(())
}

fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LeaseAuthorityProtocolError {
    code: &'static str,
}

impl LeaseAuthorityProtocolError {
    fn code(self) -> &'static str {
        self.code
    }
}

fn decode_lease_authority_request(
    encoded: &[u8],
) -> Result<LeaseAuthorityProtocolRequest, LeaseAuthorityProtocolError> {
    let envelope: LeaseAuthorityProtocolEnvelope =
        serde_json::from_slice(encoded).map_err(|_| LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_request_invalid",
        })?;
    if envelope.schema_version != LEASE_AUTHORITY_PROTOCOL_REQUEST_SCHEMA_VERSION {
        return Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_schema_unsupported",
        });
    }
    match envelope.operation.as_str() {
        "acquire" => serde_json::from_value(envelope.payload)
            .map(Box::new)
            .map(LeaseAuthorityProtocolRequest::Acquire)
            .map_err(|_| LeaseAuthorityProtocolError {
                code: "lease_authority_protocol_request_invalid",
            }),
        "authorize_effect" => Ok(LeaseAuthorityProtocolRequest::AuthorizeEffect(
            envelope.payload,
        )),
        "release" => Ok(LeaseAuthorityProtocolRequest::Release(envelope.payload)),
        "recover" => Ok(LeaseAuthorityProtocolRequest::Recover(envelope.payload)),
        "revoke" => Ok(LeaseAuthorityProtocolRequest::Revoke(envelope.payload)),
        "inspect" => Ok(LeaseAuthorityProtocolRequest::Inspect(envelope.payload)),
        _ => Err(LeaseAuthorityProtocolError {
            code: "lease_authority_protocol_operation_unsupported",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_rejects_generic_signing_and_state_mutation_oracles() {
        for encoded in [
            br#"{"schemaVersion":"agent-browser.lease-authority-request.v1","operation":"sign","payload":{}}"#.as_slice(),
            br#"{"schemaVersion":"agent-browser.lease-authority-request.v1","operation":"mutate_state","payload":{}}"#.as_slice(),
        ] {
            let error = match decode_lease_authority_request(encoded) {
                Ok(_) => panic!("generic authority oracle must be rejected"),
                Err(error) => error,
            };
            assert_eq!(error.code(), "lease_authority_protocol_operation_unsupported");
        }
    }

    #[test]
    fn acquire_request_is_typed_and_redacts_the_profile_capability() {
        let encoded = br#"{
            "schemaVersion":"agent-browser.lease-authority-request.v1",
            "operation":"acquire",
            "payload":{
                "rawCapability":[108,97,115,116,51,48,100,97,121,115,45,115,101,99,114,101,116],
                "resource":{"kind":"profile","id":"last30days-social"},
                "parentClaimId":null,
                "mode":"ephemeral",
                "expectedAuthorityRevision":0,
                "idempotencyKey":"acquire:last30days:tick-1",
                "now":"2026-08-31T12:00:00Z",
                "expiresAt":"2026-08-31T12:05:00Z",
                "transitionDeadline":null,
                "recoveryControllerId":null,
                "bootEpoch":"boot-1",
                "ownerGeneration":null
            }
        }"#;
        let request = match decode_lease_authority_request(encoded) {
            Ok(request) => request,
            Err(error) => panic!("typed acquire request must decode: {}", error.code()),
        };
        let LeaseAuthorityProtocolRequest::Acquire(acquire) = request else {
            panic!("decoded operation must remain acquire");
        };
        assert_eq!(
            acquire.resource,
            super::super::LeaseResourceKey::profile("last30days-social")
        );
        assert_eq!(acquire.mode, super::super::LeaseClaimMode::Ephemeral);
        assert_eq!(acquire.raw_capability.expose(), b"last30days-secret");
        assert!(!format!("{acquire:?}").contains("last30days-secret"));
    }

    #[test]
    fn authenticated_acquire_derives_holder_identity_inside_the_kernel() {
        let raw_capability = "last30days-profile-capability-secret-v1";
        let mut principals = crate::native::service_principal::ServicePrincipalRegistry::default();
        let registered = crate::native::service_principal::register_profile_capability(
            &mut principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "principal:last30days".to_string(),
                display_name: Some("Last30days".to_string()),
                profile_id: "last30days-social".to_string(),
                registered_at: Some("2026-08-31T12:00:00Z".to_string()),
                registered_by: Some("authority-bootstrap".to_string()),
            },
            raw_capability,
        )
        .unwrap();
        let encoded = serde_json::to_vec(&serde_json::json!({
            "schemaVersion": "agent-browser.lease-authority-request.v1",
            "operation": "acquire",
            "payload": {
                "rawCapability": raw_capability.as_bytes(),
                "resource": {"kind": "profile", "id": "last30days-social"},
                "parentClaimId": null,
                "mode": "ephemeral",
                "expectedAuthorityRevision": 0,
                "idempotencyKey": "acquire:last30days:tick-1",
                "now": "2026-08-31T12:00:00Z",
                "expiresAt": "2026-08-31T12:05:00Z",
                "transitionDeadline": null,
                "recoveryControllerId": null,
                "bootEpoch": "boot-1",
                "ownerGeneration": null
            }
        }))
        .unwrap();
        let request = match decode_lease_authority_request(&encoded) {
            Ok(request) => request,
            Err(error) => panic!("typed acquire request must decode: {}", error.code()),
        };
        let mut kernel = LeaseAuthorityProtocolKernel::new(
            super::super::LeaseAuthorityState::default(),
            principals,
        );
        kernel
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();

        let response = match kernel.execute(request) {
            Ok(response) => response,
            Err(error) => panic!("authenticated acquire must succeed: {}", error.code()),
        };
        let LeaseAuthorityProtocolResponse::Acquired(outcome) = response;
        let claim = outcome.claim.expect("fresh acquisition returns a claim");
        assert_eq!(claim.principal_id(), registered.principal.principal_id);
        assert_eq!(claim.capability_id(), registered.capability.capability_id);
    }

    #[test]
    fn protected_state_round_trip_preserves_replay_without_persisting_the_bearer() {
        let raw_capability = "last30days-profile-capability-secret-v1";
        let mut principals = crate::native::service_principal::ServicePrincipalRegistry::default();
        crate::native::service_principal::register_profile_capability(
            &mut principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "principal:last30days".to_string(),
                display_name: Some("Last30days".to_string()),
                profile_id: "last30days-social".to_string(),
                registered_at: Some("2026-08-31T12:00:00Z".to_string()),
                registered_by: Some("authority-bootstrap".to_string()),
            },
            raw_capability,
        )
        .unwrap();
        let encode_request = || {
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": "agent-browser.lease-authority-request.v1",
                "operation": "acquire",
                "payload": {
                    "rawCapability": raw_capability.as_bytes(),
                    "resource": {"kind": "profile", "id": "last30days-social"},
                    "parentClaimId": null,
                    "mode": "ephemeral",
                    "expectedAuthorityRevision": 0,
                    "idempotencyKey": "acquire:last30days:tick-1",
                    "now": "2026-08-31T12:00:00Z",
                    "expiresAt": "2026-08-31T12:05:00Z",
                    "transitionDeadline": null,
                    "recoveryControllerId": null,
                    "bootEpoch": "boot-1",
                    "ownerGeneration": null
                }
            }))
            .unwrap()
        };
        let request = decode_lease_authority_request(&encode_request())
            .unwrap_or_else(|error| panic!("typed acquire request must decode: {}", error.code()));
        let mut kernel = LeaseAuthorityProtocolKernel::new(
            super::super::LeaseAuthorityState::default(),
            principals,
        );
        kernel
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
        let first = kernel
            .execute(request)
            .unwrap_or_else(|error| panic!("fresh acquire must succeed: {}", error.code()));
        let LeaseAuthorityProtocolResponse::Acquired(first) = first;
        assert!(!first.replayed);

        let protected = kernel.encode_protected_state().unwrap();
        assert!(!String::from_utf8_lossy(&protected).contains(raw_capability));
        let mut restarted = LeaseAuthorityProtocolKernel::from_protected_state(&protected).unwrap();
        let replay_request = decode_lease_authority_request(&encode_request())
            .unwrap_or_else(|error| panic!("replay acquire request must decode: {}", error.code()));
        let replay = restarted
            .execute(replay_request)
            .unwrap_or_else(|error| panic!("replay must succeed: {}", error.code()));
        let LeaseAuthorityProtocolResponse::Acquired(replay) = replay;
        assert!(replay.replayed);
        assert_eq!(replay.claim, first.claim);
    }

    #[test]
    fn authenticated_acquire_cannot_invent_an_unregistered_profile_resource() {
        let raw_capability = "unregistered-profile-capability-secret-v1";
        let mut principals = crate::native::service_principal::ServicePrincipalRegistry::default();
        crate::native::service_principal::register_profile_capability(
            &mut principals,
            crate::native::service_principal::ServicePrincipalRegistrationRequest {
                principal_id: "principal:unregistered".to_string(),
                display_name: None,
                profile_id: "unregistered-profile".to_string(),
                registered_at: Some("2026-08-31T12:00:00Z".to_string()),
                registered_by: Some("authority-bootstrap".to_string()),
            },
            raw_capability,
        )
        .unwrap();
        let encoded = serde_json::to_vec(&serde_json::json!({
            "schemaVersion": "agent-browser.lease-authority-request.v1",
            "operation": "acquire",
            "payload": {
                "rawCapability": raw_capability.as_bytes(),
                "resource": {"kind": "profile", "id": "unregistered-profile"},
                "parentClaimId": null,
                "mode": "ephemeral",
                "expectedAuthorityRevision": 0,
                "idempotencyKey": "acquire:unregistered:tick-1",
                "now": "2026-08-31T12:00:00Z",
                "expiresAt": "2026-08-31T12:05:00Z",
                "transitionDeadline": null,
                "recoveryControllerId": null,
                "bootEpoch": "boot-1",
                "ownerGeneration": null
            }
        }))
        .unwrap();
        let request = decode_lease_authority_request(&encoded)
            .unwrap_or_else(|error| panic!("request must decode: {}", error.code()));
        let mut kernel = LeaseAuthorityProtocolKernel::new(
            super::super::LeaseAuthorityState::default(),
            principals,
        );

        let error = match kernel.execute(request) {
            Ok(_) => panic!("unregistered profile must not become authority"),
            Err(error) => error,
        };
        assert_eq!(
            error.code(),
            "lease_authority_protocol_resource_unregistered"
        );
    }

    #[test]
    fn protected_state_rejects_two_profile_ids_for_one_physical_identity() {
        let mut kernel = LeaseAuthorityProtocolKernel::new(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        kernel
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap();
        let mut protected: serde_json::Value =
            serde_json::from_slice(&kernel.encode_protected_state().unwrap()).unwrap();
        let first = protected
            .pointer("/resources/registrations/profile:last30days-social")
            .cloned()
            .expect("registered profile is present");
        let mut alias = first;
        alias["resource"]["id"] = serde_json::Value::String("last30days-alias".to_string());
        alias["revision"] = serde_json::Value::from(2);
        protected["resources"]["revision"] = serde_json::Value::from(2);
        protected["resources"]["registrations"]["profile:last30days-alias"] = alias;
        let encoded = serde_json::to_vec(&protected).unwrap();

        let error = match LeaseAuthorityProtocolKernel::from_protected_state(&encoded) {
            Ok(_) => panic!("one physical profile must not load as two resources"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_state_invalid");
    }

    #[test]
    fn protected_state_rejects_noncanonical_physical_identity_digest() {
        let mut kernel = LeaseAuthorityProtocolKernel::new(
            super::super::LeaseAuthorityState::default(),
            crate::native::service_principal::ServicePrincipalRegistry::default(),
        );
        kernel
            .bootstrap_profile_resource(
                "last30days-social",
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap();
        let mut protected: serde_json::Value =
            serde_json::from_slice(&kernel.encode_protected_state().unwrap()).unwrap();
        protected["resources"]["registrations"]["profile:last30days-social"]
            ["physicalIdentityDigest"] = serde_json::Value::String(
            "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
        );
        let protected = serde_json::to_vec(&protected).unwrap();

        let error = match LeaseAuthorityProtocolKernel::from_protected_state(&protected) {
            Ok(_) => panic!("physical identity digests must have one canonical spelling"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "lease_authority_protocol_state_invalid");
    }
}
