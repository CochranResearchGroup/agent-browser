//! Purpose-bound attestation for an internal cooperative return observation.
//!
//! This authenticates internal request provenance only. The CLI must still
//! enforce current principal policy, exact scope, target identity, and epoch.
//! An attestation never confers effect authority. Serialization contains secret
//! key material and belongs only in the CLI's private local sidecar adapter,
//! never in public metadata. This module performs no filesystem operations.

use ring::hmac;
use serde::{Deserialize, Serialize};

const DOMAIN: &[u8] = b"agent-browser.host-control.return-observation.v1\0";

/// Secret key for internal read provenance, with fully redacted debug output.
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CooperativeReadKey([u8; 32]);

impl std::fmt::Debug for CooperativeReadKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CooperativeReadKey([REDACTED])")
    }
}

impl CooperativeReadKey {
    /// Generate one secret key. Entropy failures disclose no underlying data.
    pub fn new() -> Result<Self, String> {
        let mut secret = [0u8; 32];
        getrandom::getrandom(&mut secret)
            .map_err(|_| "cooperative_read_key_entropy_unavailable".to_string())?;
        Ok(Self(secret))
    }

    /// Authenticate exact payload bytes for this internal read purpose only.
    pub fn attest(&self, payload: &[u8]) -> String {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.0);
        hex::encode(hmac::sign(&key, &purpose_payload(payload)).as_ref())
    }

    /// Verify a fixed-size hex MAC using ring's constant-time MAC comparison.
    /// Malformed attestations fail closed without exposing key material.
    pub fn verify(&self, payload: &[u8], attestation: &str) -> bool {
        if attestation.len() != 64 {
            return false;
        }
        let mut mac = [0u8; 32];
        if hex::decode_to_slice(attestation, &mut mac).is_err() {
            return false;
        }
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.0);
        hmac::verify(&key, &purpose_payload(payload), &mac).is_ok()
    }
}

fn purpose_payload(payload: &[u8]) -> Vec<u8> {
    let mut message = DOMAIN.to_vec();
    message.extend_from_slice(payload);
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_roundtrips_exact_payload_and_empty_payload() {
        let key = CooperativeReadKey::new().unwrap();
        for payload in [b"explicit scoped return".as_slice(), b"".as_slice()] {
            let attestation = key.attest(payload);
            assert_eq!(attestation.len(), 64);
            assert!(key.verify(payload, &attestation));
        }
    }

    #[test]
    fn payload_mutation_and_mac_mutation_are_rejected() {
        let key = CooperativeReadKey([0x5a; 32]);
        let payload = b"task=one;epoch=7;target=retained";
        let attestation = key.attest(payload);
        assert!(!key.verify(b"task=one;epoch=8;target=retained", &attestation));
        assert!(!key.verify(b"task=two;epoch=7;target=retained", &attestation));
        assert!(!key.verify(b"", &attestation));
        let mut mac = hex::decode(&attestation).unwrap();
        mac[0] ^= 1;
        assert!(!key.verify(payload, &hex::encode(mac)));
    }

    #[test]
    fn another_secret_cannot_verify_the_attestation() {
        let key = CooperativeReadKey([0x5a; 32]);
        let other = CooperativeReadKey([0xa5; 32]);
        assert!(!other.verify(b"payload", &key.attest(b"payload")));
    }

    #[test]
    fn malformed_or_wrong_size_hex_never_verifies() {
        let key = CooperativeReadKey([0x5a; 32]);
        for proof in [
            String::new(),
            "00".repeat(31),
            "00".repeat(33),
            "g0".repeat(32),
            "\u{00e9}".repeat(32),
            format!(" {}", "0".repeat(63)),
        ] {
            assert!(!key.verify(b"payload", &proof));
        }
    }

    #[test]
    fn attestation_is_bound_to_exact_private_purpose_domain() {
        let secret = [0x5a; 32];
        let key = CooperativeReadKey(secret);
        let ring_key = hmac::Key::new(hmac::HMAC_SHA256, &secret);
        let payload = b"payload";
        let mut expected = b"agent-browser.host-control.return-observation.v1\0".to_vec();
        expected.extend_from_slice(payload);
        assert_eq!(
            key.attest(payload),
            hex::encode(hmac::sign(&ring_key, &expected).as_ref())
        );
        assert!(!key.verify(
            payload,
            &hex::encode(hmac::sign(&ring_key, payload).as_ref())
        ));
        let mut foreign = b"agent-browser.host-control.effect.v1\0".to_vec();
        foreign.extend_from_slice(payload);
        assert!(!key.verify(
            payload,
            &hex::encode(hmac::sign(&ring_key, &foreign).as_ref())
        ));
    }

    #[test]
    fn private_serialization_roundtrips_and_enforces_exact_key_length() {
        let key = CooperativeReadKey([0x5a; 32]);
        let serialized = serde_json::to_string(&key).unwrap();
        let bytes: Vec<u8> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(bytes, vec![0x5a; 32]);
        let restored: CooperativeReadKey = serde_json::from_str(&serialized).unwrap();
        assert!(restored.verify(b"payload", &key.attest(b"payload")));
        for length in [0, 31, 33] {
            assert!(
                serde_json::from_value::<CooperativeReadKey>(serde_json::json!(vec![90; length]))
                    .is_err()
            );
        }
        assert!(
            serde_json::from_value::<CooperativeReadKey>(serde_json::json!("5a".repeat(32)))
                .is_err()
        );
        assert!(
            serde_json::from_value::<CooperativeReadKey>(serde_json::json!(vec![256; 32])).is_err()
        );
    }

    #[test]
    fn debug_is_fully_redacted_and_clone_preserves_provenance() {
        let key = CooperativeReadKey([0x5a; 32]);
        assert_eq!(format!("{key:?}"), "CooperativeReadKey([REDACTED])");
        assert_eq!(format!("{key:#?}"), "CooperativeReadKey([REDACTED])");
        assert!(key.clone().verify(b"payload", &key.attest(b"payload")));
    }
}
