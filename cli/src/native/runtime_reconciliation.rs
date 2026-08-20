use crate::native::service_model::ServiceState;
use crate::process_identity::{
    LegacyProfileProof, ObservedProcessIdentity, ProcessObservation, RuntimeProcessOwnership,
};
use crate::runtime_owner_transfer::{
    CleanupObligationState, ProfileOwnerState, RuntimeLaneLifecycleState,
};
use std::path::Path;

/// One current process observation presented to the runtime resource
/// reconciler. Command and path evidence are supplementary; exact ownership
/// comes from the durable launch, process, profile, and lifecycle identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeProcessEvidence {
    pub(crate) process: ObservedProcessIdentity,
    pub(crate) process_group_id: Option<u32>,
    pub(crate) logical_browser_id: Option<String>,
    pub(crate) profile_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewedProcessTree {
    pub(crate) root_process: crate::process_identity::RecordedProcessIdentity,
    pub(crate) process_group_id: u32,
    pub(crate) logical_browser_id: String,
    pub(crate) profile_identity_digest: String,
    pub(crate) owner_generation: u64,
    pub(crate) package_launch_identity_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeResourceDecision {
    Owned(ReviewedProcessTree),
    Protected { reason: &'static str },
}

/// Deep owner for joining process observation with durable runtime lifecycle
/// authority. Service resource projection and process-tree shutdown consume
/// this decision; neither recreates ownership from command-line shape.
pub(crate) struct RuntimeResourceReconciler<'a> {
    state: &'a ServiceState,
}

impl<'a> RuntimeResourceReconciler<'a> {
    pub(crate) fn new(state: &'a ServiceState) -> Self {
        Self { state }
    }

    pub(crate) fn classify(&self, evidence: RuntimeProcessEvidence) -> RuntimeResourceDecision {
        let Some(logical_browser_id) = evidence.logical_browser_id.as_deref() else {
            return protected("runtime_lifecycle_browser_unproven");
        };
        if !self.state.browsers.contains_key(logical_browser_id) {
            return protected("runtime_lifecycle_browser_unproven");
        }
        let Some(recorded) = self
            .state
            .browser_process_identities
            .get(logical_browser_id)
        else {
            return protected("runtime_lifecycle_process_identity_unproven");
        };
        let assessment = crate::process_identity::assess_process_ownership(
            Some(&recorded.process_identity),
            ProcessObservation::Observed(evidence.process),
            LegacyProfileProof::Unproven,
        );
        if assessment.ownership != RuntimeProcessOwnership::MatchingBrowser {
            return protected("runtime_lifecycle_process_identity_changed");
        }
        let Some(profile_root) = evidence.profile_root.as_deref() else {
            return protected("runtime_lifecycle_profile_identity_unproven");
        };
        if recorded.user_data_dir.as_deref() != Some(profile_root) {
            return protected("runtime_lifecycle_profile_identity_changed");
        }
        let Ok(profile_identity_digest) =
            crate::runtime_profile::canonical_profile_identity_digest(Path::new(profile_root))
        else {
            return protected("runtime_lifecycle_profile_identity_unproven");
        };
        let Some(owner) = self
            .state
            .runtime_owner_registry
            .owner(&profile_identity_digest)
        else {
            return protected("runtime_lifecycle_owner_unproven");
        };
        let Some(lifecycle) = self
            .state
            .runtime_owner_registry
            .lifecycle_records
            .get(logical_browser_id)
        else {
            return protected("runtime_lifecycle_record_unproven");
        };
        let Some(process_group_id) = evidence.process_group_id else {
            return protected("runtime_lifecycle_process_group_unproven");
        };
        let process_instance_digest =
            match crate::native::runtime_lifecycle::digest_json(&recorded.process_identity) {
                Ok(digest) => digest,
                Err(_) => return protected("runtime_lifecycle_process_identity_unproven"),
            };
        let package_launch_identity_digest =
            match crate::native::runtime_lifecycle::package_launch_identity_digest(
                owner,
                Some(process_group_id),
            ) {
                Ok(digest) => digest,
                Err(_) => return protected("runtime_lifecycle_launch_identity_unproven"),
            };
        if owner.state != ProfileOwnerState::Ready {
            return protected("runtime_lifecycle_owner_not_ready");
        }
        if owner.browser_id != logical_browser_id {
            return protected("runtime_lifecycle_owner_browser_changed");
        }
        if owner.process_instance_digest != process_instance_digest {
            return protected("runtime_lifecycle_owner_process_changed");
        }
        if lifecycle.logical_browser_id != logical_browser_id {
            return protected("runtime_lifecycle_record_browser_changed");
        }
        if lifecycle.profile_identity_digest != profile_identity_digest {
            return protected("runtime_lifecycle_record_profile_changed");
        }
        if lifecycle.owner_generation != owner.owner_generation {
            return protected("runtime_lifecycle_owner_generation_changed");
        }
        if lifecycle.lifecycle_state != RuntimeLaneLifecycleState::Closing {
            return protected("runtime_lifecycle_not_closing");
        }
        if lifecycle.cleanup_obligation_state != CleanupObligationState::Owned {
            return protected("runtime_lifecycle_cleanup_not_owned");
        }
        if lifecycle.process_group_id != Some(process_group_id) {
            return protected("runtime_lifecycle_process_group_changed");
        }
        if lifecycle.package_launch_identity_digest.as_deref()
            != Some(package_launch_identity_digest.as_str())
        {
            return protected("runtime_lifecycle_package_launch_changed");
        }

        RuntimeResourceDecision::Owned(ReviewedProcessTree {
            root_process: recorded.process_identity.clone(),
            process_group_id,
            logical_browser_id: logical_browser_id.to_string(),
            profile_identity_digest,
            owner_generation: owner.owner_generation,
            package_launch_identity_digest,
        })
    }
}

fn protected(reason: &'static str) -> RuntimeResourceDecision {
    RuntimeResourceDecision::Protected { reason }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessTreeSignal {
    Terminate,
    Kill,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProcessTreeShutdownOutcome {
    pub(crate) exact_process_exited: bool,
    pub(crate) profile_lock_released: bool,
    pub(crate) terminate_sent: bool,
    pub(crate) kill_sent: bool,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) errors: Vec<String>,
}

/// Internal adapter seam for the two real users of reviewed shutdown: an
/// owned Chrome child and resource GC. Scripted tests use the same seam.
pub(crate) trait ReviewedProcessTreeRuntime {
    fn recheck(&mut self, reviewed: &ReviewedProcessTree) -> Result<(), String>;
    fn signal_group(
        &mut self,
        process_group_id: u32,
        signal: ProcessTreeSignal,
    ) -> Result<(), String>;
    fn wait_after_signal(&mut self);
    fn process_exited(&mut self, root_pid: u32) -> Result<bool, String>;
    fn profile_lock_released(&mut self, profile_root: &Path) -> Result<bool, String>;
}

/// Execute the only reviewed process-tree shutdown protocol. Identity is
/// checked immediately before SIGTERM and again before SIGKILL.
pub(crate) fn shutdown_reviewed_process_tree(
    reviewed: &ReviewedProcessTree,
    profile_root: &Path,
    runtime: &mut impl ReviewedProcessTreeRuntime,
) -> ProcessTreeShutdownOutcome {
    let mut outcome = ProcessTreeShutdownOutcome::default();
    if let Err(reason) = runtime.recheck(reviewed) {
        outcome.blocked_reason = Some(reason);
        return outcome;
    }
    if let Err(error) =
        runtime.signal_group(reviewed.process_group_id, ProcessTreeSignal::Terminate)
    {
        outcome.errors.push(error);
        return outcome;
    }
    outcome.terminate_sent = true;
    runtime.wait_after_signal();
    match runtime.process_exited(reviewed.root_process.pid) {
        Ok(true) => outcome.exact_process_exited = true,
        Ok(false) => {}
        Err(error) => {
            outcome.errors.push(error);
            return outcome;
        }
    }
    if !outcome.exact_process_exited {
        if let Err(reason) = runtime.recheck(reviewed) {
            outcome.blocked_reason = Some(reason);
            return outcome;
        }
        if let Err(error) = runtime.signal_group(reviewed.process_group_id, ProcessTreeSignal::Kill)
        {
            outcome.errors.push(error);
            return outcome;
        }
        outcome.kill_sent = true;
        runtime.wait_after_signal();
        match runtime.process_exited(reviewed.root_process.pid) {
            Ok(exited) => outcome.exact_process_exited = exited,
            Err(error) => outcome.errors.push(error),
        }
    }
    if outcome.exact_process_exited {
        match runtime.profile_lock_released(profile_root) {
            Ok(released) => outcome.profile_lock_released = released,
            Err(error) => outcome.errors.push(error),
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserHealth, BrowserHost, BrowserProcess, ServiceBrowserProcessIdentity,
    };
    use crate::process_identity::RecordedProcessIdentity;
    use crate::runtime_owner_transfer::{
        CleanupObligationState, ProfileOwner, ProfileOwnerState, RuntimeLaneLifecycleState,
        RuntimeLifecycleRecord, RuntimeOwnerRegistry,
    };
    use std::collections::VecDeque;

    struct ScriptedProcessTreeRuntime {
        rechecks: VecDeque<Result<(), String>>,
        exits: VecDeque<bool>,
        signals: Vec<(u32, ProcessTreeSignal)>,
        profile_lock_released: bool,
    }

    impl ReviewedProcessTreeRuntime for ScriptedProcessTreeRuntime {
        fn recheck(&mut self, _reviewed: &ReviewedProcessTree) -> Result<(), String> {
            self.rechecks
                .pop_front()
                .unwrap_or_else(|| Err("unexpected_recheck".to_string()))
        }

        fn signal_group(
            &mut self,
            process_group_id: u32,
            signal: ProcessTreeSignal,
        ) -> Result<(), String> {
            self.signals.push((process_group_id, signal));
            Ok(())
        }

        fn wait_after_signal(&mut self) {}

        fn process_exited(&mut self, _root_pid: u32) -> Result<bool, String> {
            Ok(self.exits.pop_front().unwrap_or(false))
        }

        fn profile_lock_released(&mut self, _profile_root: &Path) -> Result<bool, String> {
            Ok(self.profile_lock_released)
        }
    }

    fn reviewed_tree() -> ReviewedProcessTree {
        ReviewedProcessTree {
            root_process: RecordedProcessIdentity {
                pid: 4200,
                start_token: "linux:fixture:200".to_string(),
                executable_path: Some("/opt/agent-browser/chrome".to_string()),
                browser_family: Some("chrome".to_string()),
            },
            process_group_id: 4200,
            logical_browser_id: "browser-tree".to_string(),
            profile_identity_digest: "a".repeat(64),
            owner_generation: 5,
            package_launch_identity_digest: "b".repeat(64),
        }
    }

    #[test]
    fn reviewed_shutdown_rechecks_and_signals_the_process_group() {
        let mut runtime = ScriptedProcessTreeRuntime {
            rechecks: VecDeque::from([Ok(()), Ok(())]),
            exits: VecDeque::from([false, true]),
            signals: Vec::new(),
            profile_lock_released: true,
        };

        let outcome = shutdown_reviewed_process_tree(
            &reviewed_tree(),
            Path::new("/tmp/agent-browser-reviewed-tree"),
            &mut runtime,
        );

        assert_eq!(
            runtime.signals,
            vec![
                (4200, ProcessTreeSignal::Terminate),
                (4200, ProcessTreeSignal::Kill),
            ]
        );
        assert!(outcome.exact_process_exited);
        assert!(outcome.profile_lock_released);
        assert_eq!(outcome.blocked_reason, None);
    }

    #[test]
    fn reviewed_shutdown_refuses_kill_when_identity_changes_after_terminate() {
        let mut runtime = ScriptedProcessTreeRuntime {
            rechecks: VecDeque::from([
                Ok(()),
                Err("runtime_lifecycle_owner_generation_changed".to_string()),
            ]),
            exits: VecDeque::from([false]),
            signals: Vec::new(),
            profile_lock_released: false,
        };

        let outcome = shutdown_reviewed_process_tree(
            &reviewed_tree(),
            Path::new("/tmp/agent-browser-reviewed-tree"),
            &mut runtime,
        );

        assert_eq!(runtime.signals, vec![(4200, ProcessTreeSignal::Terminate)]);
        assert_eq!(
            outcome.blocked_reason.as_deref(),
            Some("runtime_lifecycle_owner_generation_changed")
        );
        assert!(!outcome.kill_sent);
    }

    #[test]
    fn exact_closing_package_browser_tree_is_owned() {
        let profile_root = std::env::temp_dir().join("agent-browser-reconciler-owned-profile");
        let profile_identity_digest =
            crate::runtime_profile::canonical_profile_identity_digest(&profile_root).unwrap();
        let recorded = RecordedProcessIdentity {
            pid: 4100,
            start_token: "linux:fixture:100".to_string(),
            executable_path: Some("/opt/agent-browser/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        let process_instance_digest =
            crate::native::runtime_lifecycle::digest_json(&recorded).unwrap();
        let owner = ProfileOwner {
            owner_id: "owner-reconciler".to_string(),
            profile_identity_digest: profile_identity_digest.clone(),
            state: ProfileOwnerState::Ready,
            owner_generation: 7,
            browser_id: "browser-reconciler".to_string(),
            daemon_session_route: "reconciler".to_string(),
            process_instance_digest,
            browser_family: "chrome".to_string(),
            cdp_endpoint_identity_digest: "c".repeat(64),
            target_set_digest: "d".repeat(64),
            pending_transfer: None,
            last_transition: None,
        };
        let package_launch_identity_digest =
            crate::native::runtime_lifecycle::package_launch_identity_digest(&owner, Some(4100))
                .unwrap();
        let mut state = ServiceState::default();
        state.runtime_owner_registry = RuntimeOwnerRegistry::from_owner(owner.clone());
        state.runtime_owner_registry.lifecycle_records.insert(
            owner.browser_id.clone(),
            RuntimeLifecycleRecord {
                logical_browser_id: owner.browser_id.clone(),
                profile_identity_digest: profile_identity_digest.clone(),
                owner_generation: owner.owner_generation,
                lifecycle_state: RuntimeLaneLifecycleState::Closing,
                cleanup_obligation_state: CleanupObligationState::Owned,
                process_group_id: Some(4100),
                package_launch_identity_digest: Some(package_launch_identity_digest.clone()),
                terminal_evidence: Vec::new(),
            },
        );
        state.browsers.insert(
            owner.browser_id.clone(),
            BrowserProcess {
                id: owner.browser_id.clone(),
                host: BrowserHost::LocalHeadless,
                health: BrowserHealth::Faulted,
                pid: Some(recorded.pid),
                ..BrowserProcess::default()
            },
        );
        state.browser_process_identities.insert(
            owner.browser_id.clone(),
            ServiceBrowserProcessIdentity {
                process_identity: recorded.clone(),
                user_data_dir: Some(profile_root.to_string_lossy().into_owned()),
                runtime_profile: None,
            },
        );

        let decision = RuntimeResourceReconciler::new(&state).classify(RuntimeProcessEvidence {
            process: ObservedProcessIdentity {
                pid: recorded.pid,
                start_token: Some(recorded.start_token.clone()),
                executable_path: recorded.executable_path.clone(),
                browser_family: recorded.browser_family.clone(),
                command_line: Some(vec![
                    "chrome --user-data-dir=/tmp/agent-browser-reconciler-owned-profile"
                        .to_string(),
                ]),
            },
            process_group_id: Some(4100),
            logical_browser_id: Some(owner.browser_id.clone()),
            profile_root: Some(profile_root.to_string_lossy().into_owned()),
        });

        assert_eq!(
            decision,
            RuntimeResourceDecision::Owned(ReviewedProcessTree {
                root_process: recorded,
                process_group_id: 4100,
                logical_browser_id: owner.browser_id,
                profile_identity_digest,
                owner_generation: owner.owner_generation,
                package_launch_identity_digest,
            })
        );
    }
}
