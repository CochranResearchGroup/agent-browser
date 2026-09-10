//! Read-only joined diagnosis for one stored Service profile.
//!
//! This projection consumes the existing profile, lifecycle owner, lease,
//! process, readiness, and presentation authorities. It does not create a
//! second owner evaluator and never mutates Service State or the profile path.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use super::super::service_model::ServiceState;
use super::super::service_profile_access_policy::ProfileAccessMode;
use super::super::service_profile_lease::profile_leases_for_state;
use crate::process_identity::{observe_process, recorded_process_is_running, ProcessObservation};
use crate::runtime_owner_transfer::ProfileOwnerState;

pub(crate) const PROFILE_DIAGNOSIS_SCHEMA_V1: &str = "agent-browser.profile-diagnosis.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProfileDiagnosisState {
    Ready,
    Repairable,
    ManualAction,
    Blocked,
    ResetAvailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileDiagnosisFinding {
    pub(crate) code: String,
    pub(crate) axis: String,
    pub(crate) severity: String,
    pub(crate) blocking: bool,
    pub(crate) message: String,
    pub(crate) evidence: Vec<String>,
    pub(crate) recourse: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ServiceProfileDiagnosis {
    pub(crate) schema_version: String,
    pub(crate) diagnosis_id: String,
    pub(crate) observed_at: String,
    pub(crate) state: ProfileDiagnosisState,
    pub(crate) profile: Value,
    pub(crate) runtime: Value,
    pub(crate) ownership: Value,
    pub(crate) chrome_locks: Value,
    pub(crate) readiness: Value,
    pub(crate) presentation: Value,
    pub(crate) decision: Value,
    pub(crate) trace: Value,
    pub(crate) findings: Vec<ProfileDiagnosisFinding>,
}

pub(crate) fn diagnose_service_profile(
    state: &ServiceState,
    profile_id: &str,
    observed_at: &str,
    correlation_id: &str,
) -> Result<ServiceProfileDiagnosis, String> {
    let profile = state
        .profiles
        .get(profile_id)
        .ok_or_else(|| "service_profile_diagnosis_profile_missing".to_string())?;
    let profile_identity_digest = super::recovery::recovery_profile_identity_digest(profile)?;
    let owner = state.runtime_owner_registry.owner(&profile_identity_digest);
    let logical_browser_id = owner.map(|owner| owner.browser_id.as_str());
    let browser = logical_browser_id.and_then(|browser_id| state.browsers.get(browser_id));
    let process_identity =
        logical_browser_id.and_then(|browser_id| state.browser_process_identities.get(browser_id));
    let process_state = match (browser.and_then(|browser| browser.pid), process_identity) {
        (_, Some(identity)) => match recorded_process_is_running(&identity.process_identity) {
            Ok(true) => "current",
            Ok(false) => "missing_or_changed",
            Err(_) => "observation_failed",
        },
        (Some(pid), None) => match observe_process(pid) {
            ProcessObservation::Observed(_) => "live_unproven",
            ProcessObservation::Missing => "missing",
            ProcessObservation::Failed { .. } => "observation_failed",
        },
        (None, _) => "not_recorded",
    };

    let mut session_ids = state
        .sessions
        .values()
        .filter(|session| {
            session.profile_id.as_deref() == Some(profile_id)
                || logical_browser_id
                    .is_some_and(|browser_id| session.browser_ids.iter().any(|id| id == browser_id))
        })
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    session_ids.sort();
    session_ids.dedup();
    let mut tab_ids = state
        .tabs
        .values()
        .filter(|tab| logical_browser_id == Some(tab.browser_id.as_str()))
        .map(|tab| tab.id.clone())
        .collect::<Vec<_>>();
    tab_ids.sort();
    let mut target_ids = state
        .tabs
        .values()
        .filter(|tab| logical_browser_id == Some(tab.browser_id.as_str()))
        .filter_map(|tab| tab.target_id.clone())
        .collect::<Vec<_>>();
    target_ids.sort();
    target_ids.dedup();

    let binding = state
        .runtime_owner_registry
        .principal_bindings
        .get(&profile_identity_digest);
    let leases = profile_leases_for_state(state, observed_at)
        .into_iter()
        .filter(|lease| lease.profile_id == profile_id)
        .collect::<Vec<_>>();
    let access_mode = profile
        .access_policy
        .as_ref()
        .map(|policy| policy.mode)
        .unwrap_or(ProfileAccessMode::SharedLocal);
    let shared_local = access_mode == ProfileAccessMode::SharedLocal;
    let lock_diagnosis = inspect_chrome_locks(
        profile.user_data_dir.as_deref(),
        browser.and_then(|browser| browser.pid),
    );

    let mut findings = Vec::new();
    if let Some(owner) = owner {
        if owner.state == ProfileOwnerState::Ready && browser.is_none() {
            findings.push(finding(
                "runtime_browser_record_missing",
                "runtime",
                "error",
                true,
                "The current lifecycle owner has no matching Service browser record.",
                vec![owner.browser_id.clone()],
                "plan_preserving_profile_repair",
            ));
        }
        if let Some(browser) = browser {
            match browser.profile_id.as_deref() {
                None => findings.push(finding(
                    "subordinate_browser_profile_missing",
                    "ownership",
                    "error",
                    true,
                    "The current browser record is missing its canonical profile join.",
                    vec![browser.id.clone(), profile_id.to_string()],
                    "plan_preserving_profile_repair",
                )),
                Some(current) if current != profile_id => findings.push(finding(
                    "subordinate_browser_profile_mismatch",
                    "ownership",
                    "error",
                    true,
                    "The current browser record names a different profile.",
                    vec![
                        browser.id.clone(),
                        current.to_string(),
                        profile_id.to_string(),
                    ],
                    "inspect_foreign_or_changed_owner",
                )),
                _ => {}
            }
        }
        if matches!(process_state, "missing_or_changed" | "observation_failed")
            && owner.state == ProfileOwnerState::Ready
        {
            findings.push(finding(
                "owner_process_identity_unproven",
                "runtime",
                "error",
                true,
                "The ready owner does not have current matching process evidence.",
                vec![owner.browser_id.clone(), process_state.to_string()],
                "inspect_exact_process_before_repair",
            ));
        }
    }
    if binding.is_none() {
        findings.push(finding(
            "runtime_owner_principal_binding_missing",
            "ownership",
            if shared_local { "advisory" } else { "error" },
            !shared_local,
            if shared_local {
                "No registered principal binding exists; shared-local self-identification remains available for ordinary permitted use."
            } else {
                "The current owner lacks the principal binding required by this restricted profile."
            },
            vec![profile_id.to_string(), profile_identity_digest.clone()],
            if shared_local {
                "continue_with_self_declared_access"
            } else {
                "plan_preserving_principal_repair"
            },
        ));
    }
    if lock_diagnosis.get("verdict").and_then(Value::as_str) == Some("proven_stale") {
        findings.push(finding(
            "chrome_singleton_lock_proven_stale",
            "chrome_locks",
            "error",
            true,
            "The Chrome singleton lock names a process that is proven absent.",
            Vec::new(),
            "plan_preserving_stale_lock_repair",
        ));
    } else if lock_diagnosis
        .get("verdict")
        .and_then(Value::as_str)
        .is_some_and(|verdict| verdict == "occupied_unproven" || verdict == "observation_failed")
    {
        findings.push(finding(
            "chrome_singleton_lock_ownership_unproven",
            "chrome_locks",
            "error",
            true,
            "Chrome singleton lock ownership is live or could not be proven.",
            Vec::new(),
            "inspect_exact_lock_process_without_removal",
        ));
    }

    let missing_auth_targets = profile
        .target_service_ids
        .iter()
        .filter(|target| !profile.authenticated_service_ids.contains(target))
        .cloned()
        .collect::<Vec<_>>();
    let has_blocking = findings.iter().any(|finding| finding.blocking);
    let finding_is_repairable = |finding: &ProfileDiagnosisFinding| {
        finding.blocking
            && matches!(
                finding.code.as_str(),
                "runtime_browser_record_missing"
                    | "subordinate_browser_profile_missing"
                    | "chrome_singleton_lock_proven_stale"
                    | "runtime_owner_principal_binding_missing"
            )
    };
    let repairable = findings.iter().any(finding_is_repairable);
    let unrepairable_blocking = findings
        .iter()
        .any(|finding| finding.blocking && !finding_is_repairable(finding));
    let diagnosis_state = if unrepairable_blocking {
        ProfileDiagnosisState::Blocked
    } else if has_blocking && repairable {
        ProfileDiagnosisState::Repairable
    } else if has_blocking {
        ProfileDiagnosisState::Blocked
    } else if !missing_auth_targets.is_empty() {
        ProfileDiagnosisState::ManualAction
    } else {
        ProfileDiagnosisState::Ready
    };
    let eligible_action = match diagnosis_state {
        ProfileDiagnosisState::Ready => "normal_launch",
        ProfileDiagnosisState::Repairable => "service_profile_repair_plan",
        ProfileDiagnosisState::ManualAction => "service_profile_reset_plan_authentication",
        ProfileDiagnosisState::Blocked => "none",
        ProfileDiagnosisState::ResetAvailable => "service_profile_reset_plan",
    };
    let dominant_blocker = findings
        .iter()
        .find(|finding| finding.blocking)
        .map(|finding| finding.code.clone());

    let handoffs = state
        .remote_view_handoffs
        .values()
        .filter(|handoff| logical_browser_id == handoff.browser_id.as_deref())
        .map(|handoff| json!({"id": handoff.id, "state": handoff.state}))
        .collect::<Vec<_>>();
    let streams = browser
        .map(|browser| {
            browser
                .view_streams
                .iter()
                .map(|stream| {
                    json!({
                        "id": stream.id,
                        "provider": stream.provider,
                        "readiness": stream.readiness,
                        "routeId": stream.route_id,
                        "displayAllocationId": stream.display_allocation_id,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let diagnosis_id = digest_json(&(
        PROFILE_DIAGNOSIS_SCHEMA_V1,
        profile_id,
        state.state_revision,
        state.runtime_owner_registry.revision,
        observed_at,
        correlation_id,
    ))?;

    Ok(ServiceProfileDiagnosis {
        schema_version: PROFILE_DIAGNOSIS_SCHEMA_V1.to_string(),
        diagnosis_id,
        observed_at: observed_at.to_string(),
        state: diagnosis_state,
        profile: json!({
            "id": profile.id,
            "stateRevision": state.state_revision,
            "policyRevision": profile.access_policy.as_ref().map(|policy| policy.revision),
            "userDataPathDigest": profile_identity_digest,
            "allocation": profile.allocation,
            "browserBuild": profile.browser_build,
            "accessMode": access_mode,
            "sharedServiceIds": profile.shared_service_ids,
            "targetServiceIds": profile.target_service_ids,
        }),
        runtime: json!({
            "browserId": logical_browser_id,
            "daemonSessionRoute": owner.map(|owner| owner.daemon_session_route.as_str()),
            "health": browser.map(|browser| browser.health),
            "pid": browser.and_then(|browser| browser.pid),
            "processState": process_state,
            "processStartToken": process_identity.map(|identity| identity.process_identity.start_token.as_str()),
            "executablePath": process_identity.and_then(|identity| identity.process_identity.executable_path.as_deref()),
            "executableDigest": Value::Null,
            "executableDigestUnavailableReason": "not_retained_in_service_process_identity",
            "bootEpoch": browser.and_then(|browser| browser.boot_epoch.as_deref()),
            "sessionIds": session_ids,
            "tabIds": tab_ids,
            "targetIds": target_ids,
            "cdpAttached": browser.and_then(|browser| browser.cdp_endpoint.as_ref()).is_some(),
        }),
        ownership: json!({
            "ownerId": owner.map(|owner| owner.owner_id.as_str()),
            "ownerGeneration": owner.map(|owner| owner.owner_generation),
            "ownerState": owner.map(|owner| owner.state),
            "profileIdentityDigest": profile_identity_digest,
            "principalId": binding.map(|binding| binding.principal_id.as_str()),
            "principalProvenance": binding.map(|binding| binding.provenance),
            "capabilityId": binding.map(|binding| binding.capability_id.as_str()),
            "leaseRecords": leases,
        }),
        chrome_locks: lock_diagnosis,
        readiness: json!({
            "authenticatedServiceIds": profile.authenticated_service_ids,
            "missingAuthenticationTargetIds": missing_auth_targets,
            "targetReadiness": profile.target_readiness,
            "manualSeedingRequired": profile.target_readiness.iter().any(|row| row.manual_seeding_required),
            "monitorEvidence": [],
            "monitorEvidenceUnavailableReason": "profile_monitor_join_not_yet_modeled",
        }),
        presentation: json!({
            "browserHost": browser.map(|browser| browser.host),
            "displayIsolation": browser.and_then(|browser| browser.display_isolation.as_deref()),
            "displayName": browser.and_then(|browser| browser.display_name.as_deref()),
            "displayAllocationId": browser.and_then(|browser| browser.display_allocation_id.as_deref()),
            "streams": streams,
            "handoffs": handoffs,
        }),
        decision: json!({
            "state": diagnosis_state,
            "dominantBlocker": dominant_blocker,
            "eligibleAction": eligible_action,
            "proposedEffects": [],
            "preservedData": ["profile_directory", "cookies", "credentials", "extensions", "authenticated_site_state"],
            "manualStep": if diagnosis_state == ProfileDiagnosisState::ManualAction { Some("complete_detached_manual_seeding") } else { None },
            "recourse": eligible_action,
        }),
        trace: json!({
            "correlationId": correlation_id,
            "sourceComponent": "service_profile_diagnosis.rs::diagnose_service_profile",
            "serviceStateRevision": state.state_revision,
            "runtimeOwnerRegistryRevision": state.runtime_owner_registry.revision,
            "buildIdentity": Value::Null,
            "buildIdentityUnavailableReason": "producer_build_identity_not_exposed_to_profile_diagnosis",
        }),
        findings,
    })
}

fn inspect_chrome_locks(user_data_dir: Option<&str>, current_browser_pid: Option<u32>) -> Value {
    let Some(user_data_dir) = user_data_dir else {
        return json!({
            "verdict": "unavailable",
            "unavailableReason": "profile_user_data_directory_missing",
            "locks": [],
        });
    };
    let mut locks = Vec::new();
    let mut verdict = "released";
    for name in ["SingletonLock", "SingletonSocket", "SingletonCookie"] {
        let path = Path::new(user_data_dir).join(name);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                let target = fs::read_link(&path).ok();
                let target_digest = target
                    .as_ref()
                    .map(|target| digest_text(&target.to_string_lossy()));
                let parsed_pid = (name == "SingletonLock")
                    .then(|| {
                        target
                            .as_ref()
                            .and_then(|target| parse_singleton_lock_pid(target))
                    })
                    .flatten();
                let process_state = parsed_pid.map(|pid| match observe_process(pid) {
                    ProcessObservation::Missing => "missing",
                    ProcessObservation::Observed(_) if Some(pid) == current_browser_pid => {
                        "current_browser"
                    }
                    ProcessObservation::Observed(_) => "other_live_process",
                    ProcessObservation::Failed { .. } => "observation_failed",
                });
                if name == "SingletonLock" {
                    verdict = match process_state {
                        Some("missing") => "proven_stale",
                        Some("current_browser") => "current_owner",
                        Some("other_live_process") | None => "occupied_unproven",
                        Some("observation_failed") => "observation_failed",
                        Some(_) => "occupied_unproven",
                    };
                }
                locks.push(json!({
                    "name": name,
                    "present": true,
                    "kind": if metadata.file_type().is_symlink() { "symlink" } else if metadata.is_file() { "file" } else { "other" },
                    "targetDigest": target_digest,
                    "parsedPid": parsed_pid,
                    "processState": process_state,
                }));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => locks.push(json!({
                "name": name,
                "present": false,
                "kind": Value::Null,
                "targetDigest": Value::Null,
                "parsedPid": Value::Null,
                "processState": Value::Null,
            })),
            Err(_) => {
                verdict = "observation_failed";
                locks.push(json!({
                    "name": name,
                    "present": Value::Null,
                    "kind": Value::Null,
                    "targetDigest": Value::Null,
                    "parsedPid": Value::Null,
                    "processState": "observation_failed",
                }));
            }
        }
    }
    json!({"verdict": verdict, "locks": locks})
}

fn parse_singleton_lock_pid(target: &Path) -> Option<u32> {
    target
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.rsplit_once('-').map(|(_, pid)| pid))
        .and_then(|pid| pid.parse::<u32>().ok())
        .filter(|pid| *pid > 0)
}

fn finding(
    code: &str,
    axis: &str,
    severity: &str,
    blocking: bool,
    message: &str,
    evidence: Vec<String>,
    recourse: &str,
) -> ProfileDiagnosisFinding {
    ProfileDiagnosisFinding {
        code: code.to_string(),
        axis: axis.to_string(),
        severity: severity.to_string(),
        blocking,
        message: message.to_string(),
        evidence,
        recourse: recourse.to_string(),
    }
}

fn digest_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn digest_json(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|encoded| format!("{:x}", Sha256::digest(encoded)))
        .map_err(|error| format!("service_profile_diagnosis_encode_failed:{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserProcess, BrowserProfile, BrowserSession, LeaseState, ProfileTargetReadiness,
    };
    use crate::runtime_owner_transfer::{ProfileOwner, RuntimeOwnerRegistry};
    use std::collections::BTreeMap;

    fn profile(path: &Path) -> BrowserProfile {
        BrowserProfile {
            id: "shared-profile".to_string(),
            user_data_dir: Some(path.to_string_lossy().to_string()),
            target_service_ids: vec!["bill".to_string()],
            authenticated_service_ids: vec!["bill".to_string()],
            ..BrowserProfile::default()
        }
    }

    fn transferred_state(path: &Path, browser_profile_id: Option<&str>) -> ServiceState {
        let profile = profile(path);
        let digest = super::super::recovery::recovery_profile_identity_digest(&profile).unwrap();
        let browser_id = "session:original-browser".to_string();
        ServiceState {
            state_revision: 9,
            profiles: BTreeMap::from([(profile.id.clone(), profile)]),
            browsers: BTreeMap::from([(
                browser_id.clone(),
                BrowserProcess {
                    id: browser_id.clone(),
                    profile_id: browser_profile_id.map(str::to_string),
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "handoff-current".to_string(),
                BrowserSession {
                    id: "handoff-current".to_string(),
                    profile_id: Some("shared-profile".to_string()),
                    browser_ids: vec![browser_id.clone()],
                    lease: LeaseState::Shared,
                    ..BrowserSession::default()
                },
            )]),
            runtime_owner_registry: RuntimeOwnerRegistry::from_owner(ProfileOwner {
                owner_id: "owner-current".to_string(),
                profile_identity_digest: digest.clone(),
                state: ProfileOwnerState::Ready,
                owner_generation: 6,
                browser_id,
                daemon_session_route: "handoff-current".to_string(),
                process_instance_digest: digest,
                browser_family: "chrome".to_string(),
                cdp_endpoint_identity_digest: "c".repeat(64),
                target_set_digest: "d".repeat(64),
                pending_transfer: None,
                last_transition: None,
            }),
            ..ServiceState::default()
        }
    }

    #[test]
    fn transferred_owner_missing_profile_join_is_repairable() {
        let root = std::env::temp_dir().join(format!("profile-diagnosis-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let diagnosis = diagnose_service_profile(
            &transferred_state(&root, None),
            "shared-profile",
            "2026-09-10T10:00:00Z",
            "request-1",
        )
        .unwrap();
        assert_eq!(diagnosis.state, ProfileDiagnosisState::Repairable);
        assert_eq!(diagnosis.runtime["browserId"], "session:original-browser");
        assert_eq!(diagnosis.runtime["daemonSessionRoute"], "handoff-current");
        assert!(diagnosis.findings.iter().any(|finding| {
            finding.code == "subordinate_browser_profile_missing" && finding.blocking
        }));
        assert!(diagnosis.findings.iter().any(|finding| {
            finding.code == "runtime_owner_principal_binding_missing" && !finding.blocking
        }));
        assert_eq!(
            diagnosis.decision["eligibleAction"],
            "service_profile_repair_plan"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn healthy_shared_local_profile_does_not_require_registered_identity() {
        let root = std::env::temp_dir().join(format!("profile-diagnosis-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let diagnosis = diagnose_service_profile(
            &transferred_state(&root, Some("shared-profile")),
            "shared-profile",
            "2026-09-10T10:00:00Z",
            "request-2",
        )
        .unwrap();
        assert_eq!(diagnosis.state, ProfileDiagnosisState::Ready);
        assert_eq!(diagnosis.decision["eligibleAction"], "normal_launch");
        assert!(diagnosis.findings.iter().all(|finding| !finding.blocking));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_authentication_routes_to_manual_action_without_ownership_error() {
        let root = std::env::temp_dir().join(format!("profile-diagnosis-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let mut state = transferred_state(&root, Some("shared-profile"));
        state
            .profiles
            .get_mut("shared-profile")
            .unwrap()
            .authenticated_service_ids
            .clear();
        state
            .profiles
            .get_mut("shared-profile")
            .unwrap()
            .target_readiness = vec![ProfileTargetReadiness {
            target_service_id: "bill".to_string(),
            manual_seeding_required: true,
            ..ProfileTargetReadiness::default()
        }];
        let diagnosis = diagnose_service_profile(
            &state,
            "shared-profile",
            "2026-09-10T10:00:00Z",
            "request-3",
        )
        .unwrap();
        assert_eq!(diagnosis.state, ProfileDiagnosisState::ManualAction);
        assert_eq!(
            diagnosis.readiness["missingAuthenticationTargetIds"][0],
            "bill"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn stale_singleton_lock_is_reported_without_removal() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!("profile-diagnosis-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        symlink("host-4294967294", root.join("SingletonLock")).unwrap();
        let diagnosis = diagnose_service_profile(
            &transferred_state(&root, Some("shared-profile")),
            "shared-profile",
            "2026-09-10T10:00:00Z",
            "request-4",
        )
        .unwrap();
        assert_eq!(diagnosis.chrome_locks["verdict"], "proven_stale");
        assert_eq!(diagnosis.state, ProfileDiagnosisState::Repairable);
        assert!(
            root.join("SingletonLock").exists()
                || fs::symlink_metadata(root.join("SingletonLock")).is_ok()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn foreign_profile_mismatch_dominates_a_repairable_stale_lock() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!("profile-diagnosis-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        symlink("host-4294967294", root.join("SingletonLock")).unwrap();
        let diagnosis = diagnose_service_profile(
            &transferred_state(&root, Some("another-profile")),
            "shared-profile",
            "2026-09-10T10:00:00Z",
            "request-5",
        )
        .unwrap();
        assert_eq!(diagnosis.state, ProfileDiagnosisState::Blocked);
        assert_eq!(diagnosis.decision["eligibleAction"], "none");
        assert!(diagnosis.findings.iter().any(|finding| {
            finding.code == "subordinate_browser_profile_mismatch" && finding.blocking
        }));
        assert!(fs::symlink_metadata(root.join("SingletonLock")).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
}
