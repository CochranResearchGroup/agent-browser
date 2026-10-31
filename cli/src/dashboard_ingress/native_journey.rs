//! Selected-generation acceptance from owner-qualified Native viewer use.
//! This companion receipt keeps legacy selector and rollback bytes readable.
use super::*;
use crate::native::browser_session_remote_view::NativeHandoffJourney;
use agent_browser_service_model::RemoteViewApplicationTarget;

const SCHEMA: &str = "agent-browser.native-operator-journey.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeJourneyReceipt {
    schema_version: String,
    receipt_id: String,
    dashboard_deployment_generation: String,
    backend: DashboardBackend,
    state: String,
    handoff_id: String,
    target_digest: String,
    process_instance_digest: String,
    provider_target: RemoteViewApplicationTarget,
    correlation_id: String,
    viewer_connection_digest: String,
    frame_sync: u64,
    observed_at_ms: u64,
}

fn receipt_path(ingress: &Path) -> PathBuf {
    ingress.with_extension("native-journey.json")
}

fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl NativeJourneyReceipt {
    fn qualifies(&self, backend: &DashboardBackend) -> bool {
        self.schema_version == SCHEMA
            && self.state == "ready"
            && self.backend == *backend
            && self.dashboard_deployment_generation == backend.generation_id
            && !self.receipt_id.is_empty()
            && !self.handoff_id.is_empty()
            && digest(&self.target_digest)
            && digest(&self.process_instance_digest)
            && digest(&self.correlation_id)
            && digest(&self.viewer_connection_digest)
            && self.observed_at_ms > 0
            && self.provider_target.lifecycle_generation > 0
            && self.provider_target.viewing_generation > 0
    }

    fn from_use(
        backend: DashboardBackend,
        handoff_id: &str,
        correlation: &str,
        proof: NativeHandoffJourney,
        process_instance_digest: String,
    ) -> Result<Self, String> {
        let observation = proof.observation;
        let used = observation
            .used
            .as_ref()
            .ok_or("native_journey_use_pending")?;
        let context = &used.evidence.context;
        let desktop = proof
            .target
            .browser
            .desktop
            .as_ref()
            .ok_or("native_journey_desktop_missing")?;
        let target_digest = agent_browser_service_model::remote_view_native_target_digest(
            handoff_id,
            &proof.target,
        )?;
        if !digest(correlation)
            || observation.correlation_id != correlation
            || context.correlation_id != correlation
            || observation.target_digest != target_digest
            || context.target_digest != target_digest
            || context.operator.trim().is_empty()
            || context.viewer_connection_id.trim().is_empty()
            || observation.target.desktop_id != desktop.desktop_id
            || observation.target.lifecycle_generation != desktop.generation
            || context.desktop_id != observation.target.viewing_desktop_id
            || context.lifecycle_generation != observation.target.lifecycle_generation
            || context.viewing_generation != observation.target.viewing_generation
        {
            return Err("native_journey_identity_changed".into());
        }
        let receipt_id = format!(
            "native-handoff-{}",
            hex_sha256(
                &serde_json::to_vec(&(
                    &backend,
                    handoff_id,
                    correlation,
                    &target_digest,
                    &used.evidence,
                ))
                .map_err(|_| "native_journey_evidence_invalid")?
            )
        );
        let receipt = Self {
            schema_version: SCHEMA.into(),
            receipt_id,
            dashboard_deployment_generation: backend.generation_id.clone(),
            backend,
            state: "ready".into(),
            handoff_id: handoff_id.into(),
            target_digest,
            process_instance_digest,
            provider_target: observation.target.clone(),
            correlation_id: correlation.into(),
            viewer_connection_digest: hex_sha256(context.viewer_connection_id.as_bytes()),
            frame_sync: used.evidence.frame_sync,
            observed_at_ms: used.recorded_at_ms,
        };
        if !receipt.qualifies(&receipt.backend) {
            return Err("native_journey_evidence_invalid".into());
        }
        Ok(receipt)
    }
}

pub(super) fn selected_receipt(
    ingress: &Path,
    backend: &DashboardBackend,
) -> Option<serde_json::Value> {
    let receipt: NativeJourneyReceipt =
        serde_json::from_slice(&fs::read(receipt_path(ingress)).ok()?).ok()?;
    receipt
        .qualifies(backend)
        .then(|| serde_json::to_value(receipt).ok())
        .flatten()
}

/// Capture only the executing selected dashboard's identity, not a staged backend.
pub(crate) fn selected_generation() -> Option<String> {
    authenticated_selected_dashboard_generation()
}

/// A private owner response only triggers a fresh provider read; it is not evidence.
pub(crate) fn record(generation: &str, handoff_id: &str, correlation: &str) -> Result<(), String> {
    if selected_generation().as_deref() != Some(generation) {
        return Err("native_journey_dashboard_changed".into());
    }
    let path = DashboardIngressRepository::default_path();
    let registry = DashboardIngressRepository::new(&path).load()?;
    let backend = registry.selected_backend().clone();
    if backend.generation_id != generation {
        return Err("native_journey_dashboard_changed".into());
    }
    // Network and provider waits never hold the ingress writer lock.
    validate_dashboard_backend(&backend)?;
    let proof =
        crate::native::browser_session_remote_view::resolve_native_handoff_journey(handoff_id)
            .map_err(|_| "native_journey_provider_evidence_unavailable")?;
    let identity = crate::process_identity::capture_process_identity(
        proof.target.browser.pid,
        None,
        Some("chrome"),
    )
    .ok_or("native_journey_process_unavailable")?;
    let process_digest = hex_sha256(
        &serde_json::to_vec(&identity).map_err(|_| "native_journey_process_unavailable")?,
    );
    let receipt =
        NativeJourneyReceipt::from_use(backend, handoff_id, correlation, proof, process_digest)?;
    store(&path, &receipt)
}

fn store(path: &Path, receipt: &NativeJourneyReceipt) -> Result<(), String> {
    let _lock = acquire_ingress_lock(path)?;
    let registry = load_registry(path)?;
    if !receipt.qualifies(registry.selected_backend()) {
        return Err("native_journey_dashboard_changed".into());
    }
    let path = receipt_path(path);
    if fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<NativeJourneyReceipt>(&bytes).ok())
        .as_ref()
        == Some(receipt)
    {
        return Ok(());
    }
    let staged = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged)
            .map_err(|_| "native_journey_stage_failed")?;
        set_private_file(&staged)?;
        file.write_all(&serde_json::to_vec(receipt).map_err(|_| "native_journey_encode_failed")?)
            .and_then(|_| file.sync_all())
            .map_err(|_| "native_journey_persist_failed")?;
        fs::rename(&staged, &path).map_err(|_| "native_journey_commit_failed")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(staged);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::{
        ManagedBrowserInstance, ManagedBrowserSession, ManagedBrowserTab, RemoteViewFixedDesktop,
        RemoteViewTabHandoffTarget,
    };

    fn proof() -> NativeHandoffJourney {
        let target = RemoteViewTabHandoffTarget {
            session: ManagedBrowserSession {
                id: "task".into(),
                name: "task".into(),
                profile_id: "profile".into(),
                browser_id: "browser".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
                expires_at_ms: 1000,
                current_tab_id: Some("tab".into()),
            },
            tab: ManagedBrowserTab {
                id: "tab".into(),
                target_id: "target".into(),
                browser_id: "browser".into(),
                session_id: "task".into(),
                created_at_ms: 1,
                last_activity_at_ms: 1,
            },
            browser: ManagedBrowserInstance {
                id: "browser".into(),
                profile_id: "profile".into(),
                pid: 42,
                cdp_endpoint: "http://synthetic.invalid".into(),
                desktop: Some(RemoteViewFixedDesktop {
                    desktop_id: "desktop".into(),
                    friendly_route_label: "synthetic".into(),
                    generation: 2,
                }),
                active_session_ids: vec!["task".into()],
            },
        };
        let digest = agent_browser_service_model::remote_view_native_target_digest(
            "handoff-native",
            &target,
        )
        .unwrap();
        let observation = serde_json::from_value(serde_json::json!({
            "correlationId":"c".repeat(64),"requestDigest":"e".repeat(64),"application":"agent-browser",
            "target":{"assignmentId":"assignment","registrationId":"registration","desktopId":"desktop","lifecycleGeneration":2,"viewingDesktopId":"view","viewingGeneration":3},
            "targetDigest":digest,"clock":{"bootId":"synthetic-boot","elapsedMs":100},"lifetimeMs":300000,
            "used":{"recordedAtMs":150,"evidence":{"frameSync":7,"context":{
                "correlationId":"c".repeat(64),"operator":"synthetic-operator","viewerConnectionId":"synthetic-viewer",
                "desktopId":"view","lifecycleGeneration":2,"viewingGeneration":3,"targetDigest":digest
            }}}
        })).unwrap();
        NativeHandoffJourney {
            target,
            observation,
        }
    }

    fn receipt(proof: NativeHandoffJourney) -> Result<NativeJourneyReceipt, String> {
        NativeJourneyReceipt::from_use(
            DashboardBackend::new("selected-native", 4849, "selected-manifest"),
            "handoff-native",
            &"c".repeat(64),
            proof,
            "b".repeat(64),
        )
    }

    #[test]
    fn native_journey_rejects_pending_stale_or_incomplete_private_evidence() {
        assert!(receipt(proof()).is_ok());
        let mut first_frame = proof();
        first_frame
            .observation
            .used
            .as_mut()
            .unwrap()
            .evidence
            .frame_sync = 0;
        assert!(
            receipt(first_frame).is_ok(),
            "the provider's first acknowledged frame may have sequence zero"
        );
        for condition in 0..9 {
            let mut p = proof();
            match condition {
                0 => p.observation.used = None,
                1 => p.observation.correlation_id = "d".repeat(64),
                2 => p.target.tab.target_id = "replacement-target".into(),
                3 => p.target.browser.desktop.as_mut().unwrap().generation += 1,
                4 => p.observation.target.viewing_generation += 1,
                5 => p
                    .observation
                    .used
                    .as_mut()
                    .unwrap()
                    .evidence
                    .context
                    .operator
                    .clear(),
                6 => p.observation.used.as_mut().unwrap().recorded_at_ms = 0,
                7 => p
                    .observation
                    .used
                    .as_mut()
                    .unwrap()
                    .evidence
                    .context
                    .viewer_connection_id
                    .clear(),
                _ => {
                    p.observation
                        .used
                        .as_mut()
                        .unwrap()
                        .evidence
                        .context
                        .target_digest = "d".repeat(64)
                }
            }
            assert!(
                receipt(p).is_err(),
                "condition {condition} accepted unqualified Native evidence"
            );
        }
    }

    #[test]
    fn native_journey_commit_keeps_selection_and_fences_changed_backends() {
        let root =
            std::env::temp_dir().join(format!("native-journey-store-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("dashboard-ingress.json");
        let receipt = receipt(proof()).unwrap();
        let repo = DashboardIngressRepository::new(&path);
        let initial = repo.initialize(receipt.backend.clone()).unwrap();
        repo.stage_candidate(
            initial.revision,
            DashboardBackend::new("staged", 4850, "staged-manifest"),
        )
        .unwrap();
        let before = fs::read(&path).unwrap();
        store(&path, &receipt).unwrap();
        let first = fs::read(receipt_path(&path)).unwrap();
        store(&path, &receipt).unwrap();
        assert_eq!(fs::read(receipt_path(&path)).unwrap(), first);
        assert_eq!(
            fs::read(&path).unwrap(),
            before,
            "receipt may not change staged, fallback or rollback custody"
        );
        assert_eq!(
            dashboard_ingress_status_for_path(&path)["operatorJourneyReady"],
            true
        );
        let mut changed = receipt.clone();
        changed.backend.runtime_manifest_sha256 = "changed".into();
        assert!(store(&path, &changed).is_err());
        assert_eq!(fs::read(receipt_path(&path)).unwrap(), first);
        assert_eq!(fs::read(&path).unwrap(), before);
        // Old readers still parse the unchanged selector. Unknown/malformed sidecars fail closed.
        assert_eq!(
            load_registry(&path)
                .unwrap()
                .candidate_backend()
                .unwrap()
                .generation_id,
            "staged"
        );
        fs::write(receipt_path(&path), b"{}").unwrap();
        assert_eq!(
            dashboard_ingress_status_for_path(&path)["operatorJourneyReady"],
            false
        );
        fs::remove_dir_all(root).unwrap();
    }
}
