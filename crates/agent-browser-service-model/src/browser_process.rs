//! Provider-free durable browser-process records and record invariants.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{BrowserHost, ServiceTabHandle, ViewStream};

pub const SERVICE_BROWSER_HEALTH_VALUES: [&str; 10] = [
    "not_started",
    "launching",
    "ready",
    "degraded",
    "unreachable",
    "process_exited",
    "cdp_disconnected",
    "reconnecting",
    "closing",
    "faulted",
];

/// Origin of a browser record retained by the service model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserRecordSource {
    CallerProjection,
    PersistedState,
    RuntimeObserved,
    ManagedRuntime,
}

/// Evidence source that may establish authority for a browser record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserRecordAuthoritySource {
    CallerProjection,
    LegacyUnproven,
    ProcessIdentity,
    ManagedRuntime,
}

/// Current lifecycle interpretation of a retained browser record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserRecordLifecycleClassification {
    Live,
    Reattachable,
    InertLegacy,
    ReviewRequired,
}

/// Durable provenance and lifecycle evidence for a browser record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserRecordProvenance {
    pub source: BrowserRecordSource,
    pub authority_source: BrowserRecordAuthoritySource,
    pub created_at: Option<String>,
    pub last_observed_at: Option<String>,
    pub lifecycle_classification: BrowserRecordLifecycleClassification,
    pub recommended_action: String,
    pub record_revision: u64,
    pub evidence_digest: String,
}

impl Default for BrowserRecordProvenance {
    fn default() -> Self {
        Self {
            source: BrowserRecordSource::PersistedState,
            authority_source: BrowserRecordAuthoritySource::LegacyUnproven,
            created_at: None,
            last_observed_at: None,
            lifecycle_classification: BrowserRecordLifecycleClassification::ReviewRequired,
            recommended_action: "review".to_string(),
            record_revision: 0,
            evidence_digest: String::new(),
        }
    }
}

/// Browser process health as seen by the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserHealth {
    NotStarted,
    Launching,
    Ready,
    Degraded,
    Unreachable,
    ProcessExited,
    CdpDisconnected,
    Reconnecting,
    Closing,
    Faulted,
}

/// Latest service-owned browser health evidence retained on the browser record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserHealthObservation {
    pub observed_at: String,
    pub health: BrowserHealth,
    pub reason_kind: Option<String>,
    pub failure_class: Option<String>,
    pub process_exit_cause: Option<String>,
    pub message: Option<String>,
    pub details: Option<Value>,
}

impl Default for BrowserHealthObservation {
    fn default() -> Self {
        Self {
            observed_at: String::new(),
            health: BrowserHealth::NotStarted,
            reason_kind: None,
            failure_class: None,
            process_exit_cause: None,
            message: None,
            details: None,
        }
    }
}

/// A supervised or attached browser process known to the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowserProcess {
    pub id: String,
    /// Host boot that authenticated process-scoped observations on this row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_epoch: Option<String>,
    pub profile_id: Option<String>,
    pub host: BrowserHost,
    pub health: BrowserHealth,
    pub display_isolation: Option<String>,
    pub display_name: Option<String>,
    pub display_allocation_id: Option<String>,
    pub pid: Option<u32>,
    pub cdp_endpoint: Option<String>,
    pub view_streams: Vec<ViewStream>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachability: Option<Value>,
    pub active_session_ids: Vec<String>,
    pub tab_handles: Vec<ServiceTabHandle>,
    pub last_error: Option<String>,
    pub last_health_observation: Option<BrowserHealthObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_provenance: Option<BrowserRecordProvenance>,
}

impl Default for BrowserProcess {
    fn default() -> Self {
        Self {
            id: String::new(),
            boot_epoch: None,
            profile_id: None,
            host: BrowserHost::LocalHeaded,
            health: BrowserHealth::NotStarted,
            display_isolation: None,
            display_name: None,
            display_allocation_id: None,
            pid: None,
            cdp_endpoint: None,
            view_streams: Vec::new(),
            attachability: None,
            active_session_ids: Vec::new(),
            tab_handles: Vec::new(),
            last_error: None,
            last_health_observation: None,
            record_provenance: None,
        }
    }
}

/// Receipt-linked observation of a browser owner committed by the protected
/// lease authority. This projection is never operational authority: callers
/// must revalidate every axis with the protected service before any effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedBrowserOwnerObservation {
    pub schema_version: String,
    pub source: String,
    #[serde(deserialize_with = "deserialize_protected_observation_authority")]
    pub operational_authority: bool,
    pub authority_receipt_id: String,
    pub owner_id: String,
    pub owner_generation: u64,
    pub logical_browser_id: String,
    pub daemon_session_route: String,
    pub process_instance_digest: String,
    pub process_pid: u32,
    pub owner_revision: u64,
    pub observed_at: String,
    pub freshness_expires_at: String,
}

fn deserialize_protected_observation_authority<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = bool::deserialize(deserializer)?;
    if value {
        return Err(<D::Error as serde::de::Error>::custom(
            "protected_browser_owner_observation_invalid",
        ));
    }
    Ok(false)
}

/// Durable, provider-free process identity evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordedProcessIdentity {
    pub pid: u32,
    pub start_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_family: Option<String>,
}

/// Durable process-instance evidence for a service browser.
///
/// `profile_id` remains a service-domain identity. Only `runtime_profile`
/// names the runtime-profile subsystem, so custom service profiles are never
/// reinterpreted as runtime profiles during health assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceBrowserProcessIdentity {
    pub process_identity: RecordedProcessIdentity,
    pub user_data_dir: Option<String>,
    pub runtime_profile: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn protected_owner_json(operational_authority: bool) -> Value {
        json!({
            "schemaVersion": "agent-browser.protected-owner-observation.v1",
            "source": "lease_authority",
            "operationalAuthority": operational_authority,
            "authorityReceiptId": "receipt-1",
            "ownerId": "owner-1",
            "ownerGeneration": 3,
            "logicalBrowserId": "browser-1",
            "daemonSessionRoute": "session-1",
            "processInstanceDigest": "sha256:digest",
            "processPid": 42,
            "ownerRevision": 7,
            "observedAt": "2026-09-16T00:00:00Z",
            "freshnessExpiresAt": "2026-09-16T00:01:00Z"
        })
    }

    #[test]
    fn browser_health_values_match_wire_variants() {
        let values = [
            BrowserHealth::NotStarted,
            BrowserHealth::Launching,
            BrowserHealth::Ready,
            BrowserHealth::Degraded,
            BrowserHealth::Unreachable,
            BrowserHealth::ProcessExited,
            BrowserHealth::CdpDisconnected,
            BrowserHealth::Reconnecting,
            BrowserHealth::Closing,
            BrowserHealth::Faulted,
        ];

        assert_eq!(
            serde_json::to_value(values).unwrap(),
            json!(SERVICE_BROWSER_HEALTH_VALUES)
        );
    }

    #[test]
    fn browser_process_defaults_and_wire_omissions_are_stable() {
        let browser = BrowserProcess::default();
        assert_eq!(browser.host, BrowserHost::LocalHeaded);
        assert_eq!(browser.health, BrowserHealth::NotStarted);
        assert!(browser.view_streams.is_empty());
        assert!(browser.tab_handles.is_empty());

        let encoded = serde_json::to_value(browser).unwrap();
        assert_eq!(encoded["profileId"], Value::Null);
        assert_eq!(encoded["host"], "local_headed");
        assert_eq!(encoded["health"], "not_started");
        assert_eq!(encoded["viewStreams"], json!([]));
        assert_eq!(encoded["tabHandles"], json!([]));
        assert!(encoded.get("bootEpoch").is_none());
        assert!(encoded.get("attachability").is_none());
        assert!(encoded.get("recordProvenance").is_none());
        assert!(encoded.get("profile_id").is_none());

        let decoded: BrowserProcess = serde_json::from_value(json!({"id": "browser-1"})).unwrap();
        assert_eq!(decoded.id, "browser-1");
        assert_eq!(decoded.host, BrowserHost::LocalHeaded);
        assert_eq!(decoded.health, BrowserHealth::NotStarted);
    }

    #[test]
    fn provenance_and_health_observation_defaults_preserve_legacy_decode() {
        let provenance: BrowserRecordProvenance = serde_json::from_value(json!({})).unwrap();
        assert_eq!(provenance.source, BrowserRecordSource::PersistedState);
        assert_eq!(
            provenance.authority_source,
            BrowserRecordAuthoritySource::LegacyUnproven
        );
        assert_eq!(
            provenance.lifecycle_classification,
            BrowserRecordLifecycleClassification::ReviewRequired
        );
        assert_eq!(provenance.recommended_action, "review");
        let encoded = serde_json::to_value(&provenance).unwrap();
        assert_eq!(encoded["source"], "persisted_state");
        assert_eq!(encoded["authoritySource"], "legacy_unproven");
        assert_eq!(encoded["lifecycleClassification"], "review_required");
        assert!(encoded.get("authority_source").is_none());

        let observation: BrowserHealthObservation = serde_json::from_value(json!({})).unwrap();
        assert_eq!(observation.health, BrowserHealth::NotStarted);
        assert!(observation.observed_at.is_empty());
    }

    #[test]
    fn protected_owner_observation_is_never_operational_authority() {
        let accepted: ProtectedBrowserOwnerObservation =
            serde_json::from_value(protected_owner_json(false)).unwrap();
        assert!(!accepted.operational_authority);

        let error =
            serde_json::from_value::<ProtectedBrowserOwnerObservation>(protected_owner_json(true))
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("protected_browser_owner_observation_invalid"));

        let mut unknown = protected_owner_json(false);
        unknown["unexpectedAuthority"] = json!(false);
        assert!(serde_json::from_value::<ProtectedBrowserOwnerObservation>(unknown).is_err());
    }

    #[test]
    fn recorded_process_identity_preserves_provider_free_wire_shape() {
        let identity = RecordedProcessIdentity {
            pid: 42,
            start_token: "start-1".to_string(),
            executable_path: None,
            browser_family: Some("chromium".to_string()),
        };
        let service_identity = ServiceBrowserProcessIdentity {
            process_identity: identity.clone(),
            user_data_dir: Some("/profiles/work".to_string()),
            runtime_profile: None,
        };

        assert_eq!(
            serde_json::to_value(&identity).unwrap(),
            json!({
                "pid": 42,
                "startToken": "start-1",
                "browserFamily": "chromium"
            })
        );
        let encoded = serde_json::to_value(&service_identity).unwrap();
        assert_eq!(encoded["processIdentity"]["startToken"], "start-1");
        assert_eq!(encoded["userDataDir"], "/profiles/work");
        assert_eq!(encoded["runtimeProfile"], Value::Null);
        assert_eq!(
            serde_json::from_value::<ServiceBrowserProcessIdentity>(encoded).unwrap(),
            service_identity
        );
    }
}
