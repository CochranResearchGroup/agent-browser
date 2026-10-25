#[path = "../src/native/browser_mode_operation.rs"]
mod browser_mode_operation;
use browser_mode_operation::*;

fn request() -> Request {
    Request {
        operation_id: "convert-1".into(),
        browser_id: "browser-1".into(),
        profile_id: "profile-1".into(),
        original_process: "boot:pid:start".into(),
        executable: "/synthetic/chrome".into(),
        executable_sha256: None,
        task_ids: vec!["task-1".into(), "peer-task".into()],
        requested_mode: Mode::CdpFree,
    }
}
#[test]
fn close_prompt_is_durable_and_offers_owner_remediation_without_replacing_identity() {
    let directory = std::env::temp_dir().join(format!("mode-operation-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("operations.sqlite3");
    let mut store = OperationStore::open(&path).unwrap();
    let initial = store.request(request()).unwrap();
    assert_eq!(
        initial.actions(),
        vec![Action::GracefulClose, Action::Pause, Action::Cancel]
    );
    store
        .observe("convert-1", "boot:pid:start", Observation::ClosePrompt)
        .unwrap();
    drop(store);
    let mut reopened = OperationStore::open(&path).unwrap();
    let retained = reopened.request(request()).unwrap();
    assert_eq!(retained.phase, Phase::ClosePrompt);
    assert_eq!(retained.request.task_ids, vec!["task-1", "peer-task"]);
    assert_eq!(
        retained.actions(),
        vec![
            Action::ReturnToBrowser,
            Action::RetryGracefulClose,
            Action::ConfirmForceClose,
            Action::Cancel
        ]
    );
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}

struct SyntheticRuntime {
    process: String,
    mode: Mode,
    closes: usize,
    launches: usize,
    published: bool,
    lose_launch_reply: bool,
    fail_before_launch: bool,
    native_unready: bool,
    lose_restore_reply: bool,
    prompt: bool,
    uncertain_close: bool,
    forces: usize,
    focuses: usize,
}
impl ModeRuntime for SyntheticRuntime {
    fn validate_executable(&mut self, request: &Request) -> Result<(), String> {
        if request.executable == "/synthetic/unavailable" {
            Err("browser_build_unavailable".into())
        } else {
            Ok(())
        }
    }
    fn original_status(&mut self, _: &Request) -> Result<OriginalStatus, String> {
        Ok(if self.process == "boot:pid:start" {
            OriginalStatus::Live
        } else {
            OriginalStatus::Exited
        })
    }
    fn graceful_close(&mut self, _: &Request) -> Result<CloseOutcome, String> {
        self.closes += 1;
        if self.prompt {
            return Ok(CloseOutcome::Prompt);
        }
        self.process.clear();
        if self.uncertain_close {
            return Ok(CloseOutcome::Uncertain);
        }
        Ok(CloseOutcome::Exited)
    }
    fn cancel_graceful_close(&mut self, _: &Request) -> Result<bool, String> {
        let was_prompt = self.prompt;
        self.prompt = false;
        Ok(was_prompt && self.process == "boot:pid:start")
    }
    fn force_close(&mut self, _: &Request) -> Result<CloseOutcome, String> {
        self.forces += 1;
        self.process.clear();
        Ok(CloseOutcome::Exited)
    }
    fn focus(&mut self, _: &Request) -> Result<(), String> {
        self.focuses += 1;
        Ok(())
    }
    fn launch(&mut self, request: &Request) -> Result<Replacement, String> {
        if self.fail_before_launch {
            self.fail_before_launch = false;
            return Err("synthetic_pre_spawn_failure".into());
        }
        if request.executable.starts_with("/synthetic/versioned-") {
            assert_eq!(
                request.executable,
                if request.requested_mode == Mode::Cdp {
                    "/synthetic/versioned-previous"
                } else {
                    "/synthetic/versioned-target"
                },
                "Previous mode recovery must select the retained previous executable"
            );
        }
        self.launches += 1;
        self.process = "boot:replacement:start".into();
        self.mode = request.requested_mode.clone();
        if self.lose_launch_reply {
            self.lose_launch_reply = false;
            return Err("synthetic_launch_reply_lost".into());
        }
        if self.native_unready {
            return Err("mode_operation_native_unavailable".into());
        }
        Ok(Replacement {
            process_identity: self.process.clone(),
            mode: self.mode.clone(),
            debugging: self.mode == Mode::Cdp,
            native_observation: true,
            native_input: true,
        })
    }
    fn reconcile_replacement(&mut self, _: &Request) -> Result<Option<Replacement>, String> {
        if self.native_unready {
            return Err("mode_operation_native_unavailable".into());
        }
        Ok(
            (self.process == "boot:replacement:start").then(|| Replacement {
                process_identity: self.process.clone(),
                mode: self.mode.clone(),
                debugging: self.mode == Mode::Cdp,
                native_observation: true,
                native_input: true,
            }),
        )
    }
    fn replacement_absent(&mut self, _: &Request) -> Result<bool, String> {
        Ok(self.process.is_empty())
    }
    fn restore_previous(&mut self, _: &Request) -> Result<Mode, String> {
        if !self.process.is_empty() {
            self.closes += 1;
            self.process.clear();
        }
        self.native_unready = false;
        if self.lose_restore_reply {
            self.lose_restore_reply = false;
            return Err("synthetic_restore_reply_lost".into());
        }
        Ok(Mode::Cdp)
    }
    fn previous_executable(&self, request: &Request) -> Option<String> {
        request
            .executable
            .starts_with("/synthetic/versioned-")
            .then(|| "/synthetic/versioned-previous".into())
    }
    fn publish(&mut self, _: &Request, _: &Replacement) -> Result<(), String> {
        self.published = true;
        Ok(())
    }
}
#[test]
fn mode_conversion_completes_same_task_and_profile_after_verified_original_exit() {
    let directory = std::env::temp_dir().join(format!("mode-roundtrip-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let mut store = OperationStore::open(&directory.join("operations.sqlite3")).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        fail_before_launch: false,
        native_unready: false,
        lose_restore_reply: false,
        lose_launch_reply: false,
        prompt: false,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    let result = store
        .advance(
            "convert-1",
            Choice::Relaunch { confirmed: true },
            &mut runtime,
        )
        .unwrap();
    assert_eq!(result.phase, Phase::Completed);
    assert_eq!(result.request.task_ids, vec!["task-1", "peer-task"]);
    assert_eq!(result.request.profile_id, "profile-1");
    assert_eq!(result.replacement.as_ref().unwrap().mode, Mode::CdpFree);
    assert!(runtime.published);
    assert_eq!((runtime.closes, runtime.launches), (1, 1));
    store
        .advance("convert-1", Choice::Resume, &mut runtime)
        .unwrap();
    assert_eq!(
        (runtime.closes, runtime.launches),
        (1, 1),
        "completed replay does not repeat process effects"
    );
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn lost_launch_reply_resumes_original_operation_without_launch_or_site_effect_replay() {
    let directory = std::env::temp_dir().join(format!("mode-lost-reply-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("operations.sqlite3");
    let mut store = OperationStore::open(&path).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        fail_before_launch: false,
        native_unready: false,
        lose_restore_reply: false,
        lose_launch_reply: true,
        prompt: false,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    assert_eq!(
        store
            .advance(
                "convert-1",
                Choice::Relaunch { confirmed: true },
                &mut runtime
            )
            .unwrap_err(),
        "synthetic_launch_reply_lost"
    );
    assert_eq!(
        store.read("convert-1").unwrap().unwrap().phase,
        Phase::LaunchDispatched
    );
    drop(store);
    let mut resumed = OperationStore::open(&path).unwrap();
    let done = resumed
        .advance("convert-1", Choice::Resume, &mut runtime)
        .unwrap();
    assert_eq!(done.phase, Phase::Completed);
    assert_eq!((runtime.closes, runtime.launches), (1, 1));
    assert!(runtime.published);
    drop(resumed);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn close_prompt_offers_effective_focus_and_explicit_force_with_data_loss_receipt() {
    let directory = std::env::temp_dir().join(format!("mode-prompt-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let mut store = OperationStore::open(&directory.join("operations.sqlite3")).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        fail_before_launch: false,
        native_unready: false,
        lose_restore_reply: false,
        lose_launch_reply: false,
        prompt: true,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    assert!(store
        .advance(
            "convert-1",
            Choice::Relaunch { confirmed: false },
            &mut runtime
        )
        .unwrap_err()
        .contains("confirmation_required"));
    assert_eq!(
        runtime.closes, 0,
        "peer warning must precede browser-wide effects"
    );
    assert_eq!(
        store
            .advance(
                "convert-1",
                Choice::Relaunch { confirmed: true },
                &mut runtime
            )
            .unwrap()
            .phase,
        Phase::ClosePrompt
    );
    store
        .advance("convert-1", Choice::ReturnToBrowser, &mut runtime)
        .unwrap();
    assert_eq!(runtime.focuses, 1);
    assert_eq!(runtime.launches, 0);
    assert!(store
        .advance(
            "convert-1",
            Choice::ForceClose { confirmed: false },
            &mut runtime
        )
        .is_err());
    let forced = store
        .advance(
            "convert-1",
            Choice::ForceClose { confirmed: true },
            &mut runtime,
        )
        .unwrap();
    assert_eq!(forced.phase, Phase::Completed);
    assert_eq!(forced.close_method, Some(CloseMethod::Forced));
    assert_eq!((runtime.forces, runtime.launches), (1, 1));
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn retrying_graceful_close_requires_an_explicit_choice_and_can_finish_same_operation() {
    let directory = std::env::temp_dir().join(format!("mode-retry-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let mut store = OperationStore::open(&directory.join("operations.sqlite3")).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        fail_before_launch: false,
        native_unready: false,
        lose_restore_reply: false,
        lose_launch_reply: false,
        prompt: true,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    store
        .advance(
            "convert-1",
            Choice::Relaunch { confirmed: true },
            &mut runtime,
        )
        .unwrap();
    store
        .advance("convert-1", Choice::Resume, &mut runtime)
        .unwrap();
    assert_eq!(
        runtime.closes, 1,
        "read/resume does not repeat a pending close effect"
    );
    runtime.prompt = false;
    let done = store
        .advance("convert-1", Choice::RetryGracefulClose, &mut runtime)
        .unwrap();
    assert_eq!(done.phase, Phase::Completed);
    assert_eq!(done.close_method, Some(CloseMethod::Graceful));
    assert_eq!(
        (runtime.closes, runtime.forces, runtime.launches),
        (2, 0, 1)
    );
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cancelling_before_dispatch_preserves_original_and_releases_only_operation_custody() {
    let directory = std::env::temp_dir().join(format!("mode-cancel-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let mut store = OperationStore::open(&directory.join("operations.sqlite3")).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        fail_before_launch: false,
        native_unready: false,
        lose_restore_reply: false,
        lose_launch_reply: false,
        prompt: false,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    assert_eq!(
        store
            .advance("convert-1", Choice::Cancel, &mut runtime)
            .unwrap()
            .phase,
        Phase::Cancelled
    );
    assert_eq!(runtime.process, "boot:pid:start");
    assert_eq!(
        (runtime.closes, runtime.launches, runtime.forces),
        (0, 0, 0)
    );
    let mut next = request();
    next.operation_id = "convert-after-cancel".into();
    assert_eq!(store.request(next).unwrap().phase, Phase::Prepared);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn simultaneous_owner_choices_cannot_dispatch_two_effects_for_one_operation() {
    let directory = std::env::temp_dir().join(format!("mode-concurrent-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("operations.sqlite3");
    let mut store = OperationStore::open(&path).unwrap();
    store.request(request()).unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    struct PausedRuntime {
        inner: SyntheticRuntime,
        started: std::sync::mpsc::Sender<()>,
        release: std::sync::mpsc::Receiver<()>,
    }
    impl ModeRuntime for PausedRuntime {
        fn original_status(&mut self, r: &Request) -> Result<OriginalStatus, String> {
            self.inner.original_status(r)
        }
        fn graceful_close(&mut self, r: &Request) -> Result<CloseOutcome, String> {
            self.started.send(()).unwrap();
            self.release
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            self.inner.graceful_close(r)
        }
        fn force_close(&mut self, r: &Request) -> Result<CloseOutcome, String> {
            self.inner.force_close(r)
        }
        fn focus(&mut self, r: &Request) -> Result<(), String> {
            self.inner.focus(r)
        }
        fn launch(&mut self, r: &Request) -> Result<Replacement, String> {
            self.inner.launch(r)
        }
        fn reconcile_replacement(&mut self, r: &Request) -> Result<Option<Replacement>, String> {
            self.inner.reconcile_replacement(r)
        }
        fn publish(&mut self, r: &Request, p: &Replacement) -> Result<(), String> {
            self.inner.publish(r, p)
        }
    }
    let other_path = path.clone();
    let worker = std::thread::spawn(move || {
        let mut owner = OperationStore::open(&other_path).unwrap();
        let mut runtime = PausedRuntime {
            inner: SyntheticRuntime {
                process: "boot:pid:start".into(),
                mode: Mode::Cdp,
                closes: 0,
                launches: 0,
                published: false,
                fail_before_launch: false,
                native_unready: false,
                lose_restore_reply: false,
                lose_launch_reply: false,
                prompt: false,
                uncertain_close: false,
                forces: 0,
                focuses: 0,
            },
            started: started_tx,
            release: release_rx,
        };
        owner.advance(
            "convert-1",
            Choice::Relaunch { confirmed: true },
            &mut runtime,
        )
    });
    started_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let mut competing = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        fail_before_launch: false,
        native_unready: false,
        lose_restore_reply: false,
        lose_launch_reply: false,
        prompt: false,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    let result = store.advance(
        "convert-1",
        Choice::ForceClose { confirmed: true },
        &mut competing,
    );
    release_tx.send(()).unwrap();
    let first = worker.join().unwrap();
    assert!(result.unwrap_err().contains("busy"));
    assert_eq!((competing.forces, competing.launches), (0, 0));
    assert_eq!(first.unwrap().phase, Phase::Completed);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn unknown_close_reply_requires_original_exit_readback_without_repeating_close() {
    let directory =
        std::env::temp_dir().join(format!("mode-close-unknown-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let mut store = OperationStore::open(&directory.join("operations.sqlite3")).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        fail_before_launch: false,
        native_unready: false,
        lose_restore_reply: false,
        lose_launch_reply: false,
        prompt: false,
        uncertain_close: true,
        forces: 0,
        focuses: 0,
    };
    assert_eq!(
        store
            .advance(
                "convert-1",
                Choice::Relaunch { confirmed: true },
                &mut runtime
            )
            .unwrap()
            .phase,
        Phase::Closing
    );
    assert_eq!(
        runtime.launches, 0,
        "unknown close dispatch is not exit evidence"
    );
    assert_eq!(
        store
            .advance("convert-1", Choice::Resume, &mut runtime)
            .unwrap()
            .phase,
        Phase::Completed
    );
    assert_eq!((runtime.closes, runtime.launches), (1, 1));
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn definitively_unstarted_replacement_retries_after_restart_using_original_operation() {
    let directory = std::env::temp_dir().join(format!("mode-pre-spawn-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("operations.sqlite3");
    let mut store = OperationStore::open(&path).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        lose_launch_reply: false,
        fail_before_launch: true,
        native_unready: false,
        lose_restore_reply: false,
        prompt: false,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    assert_eq!(
        store
            .advance(
                "convert-1",
                Choice::Relaunch { confirmed: true },
                &mut runtime
            )
            .unwrap_err(),
        "synthetic_pre_spawn_failure"
    );
    assert_eq!(runtime.launches, 0);
    drop(store);
    let mut restarted = OperationStore::open(&path).unwrap();
    let recovered = restarted
        .advance(
            "convert-1",
            Choice::Relaunch { confirmed: true },
            &mut runtime,
        )
        .unwrap();
    assert_eq!(
        recovered.phase,
        Phase::Completed,
        "positive absence and explicit retry must recover the original operation"
    );
    assert_eq!(recovered.request.operation_id, "convert-1");
    assert_eq!((runtime.closes, runtime.launches), (1, 1));
    assert!(runtime.published);
    drop(restarted);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn prior_configuration_restores_same_operation_after_unstarted_conversion() {
    let directory = std::env::temp_dir().join(format!("mode-restore-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("operations.sqlite3");
    let mut store = OperationStore::open(&path).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        lose_launch_reply: false,
        fail_before_launch: true,
        native_unready: false,
        lose_restore_reply: false,
        prompt: false,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    assert!(store
        .advance(
            "convert-1",
            Choice::Relaunch { confirmed: true },
            &mut runtime
        )
        .is_err());
    drop(store);
    let mut store = OperationStore::open(&path).unwrap();
    assert!(store
        .advance(
            "convert-1",
            Choice::RestorePrevious { confirmed: false },
            &mut runtime
        )
        .is_err());
    assert_eq!(runtime.launches, 0);
    let restored = store
        .advance(
            "convert-1",
            Choice::RestorePrevious { confirmed: true },
            &mut runtime,
        )
        .unwrap();
    assert_eq!(restored.phase, Phase::Completed);
    assert_eq!(restored.request.operation_id, "convert-1");
    assert_eq!(restored.request.requested_mode, Mode::CdpFree);
    assert_eq!(restored.replacement.unwrap().mode, Mode::Cdp);
    assert_eq!((runtime.closes, runtime.launches), (1, 1));
    assert!(runtime.published);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_unready_replacement_and_lost_restore_reply_recover_original_configuration_after_restart()
{
    let directory =
        std::env::temp_dir().join(format!("mode-native-restore-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("operations.sqlite3");
    let mut store = OperationStore::open(&path).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        lose_launch_reply: false,
        fail_before_launch: false,
        native_unready: true,
        lose_restore_reply: true,
        prompt: false,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    assert_eq!(
        store
            .advance(
                "convert-1",
                Choice::Relaunch { confirmed: true },
                &mut runtime
            )
            .unwrap_err(),
        "mode_operation_native_unavailable"
    );
    let pending = store.read("convert-1").unwrap().unwrap();
    assert_eq!(pending.phase, Phase::ReplacementPending);
    assert!(pending.actions().contains(&Action::RestorePrevious));
    assert!(!runtime.published);
    assert_eq!(
        store
            .advance(
                "convert-1",
                Choice::RestorePrevious { confirmed: true },
                &mut runtime
            )
            .unwrap_err(),
        "synthetic_restore_reply_lost"
    );
    assert!(store.read("convert-1").unwrap().unwrap().restoring_previous);
    drop(store);
    let mut store = OperationStore::open(&path).unwrap();
    let restored = store
        .advance("convert-1", Choice::Resume, &mut runtime)
        .unwrap();
    assert_eq!(restored.phase, Phase::Completed);
    assert_eq!(restored.effective_mode, Some(Mode::Cdp));
    assert_eq!(restored.launch_attempts, 2);
    assert_eq!((runtime.closes, runtime.launches), (2, 2));
    assert!(runtime.published);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cancelling_an_observed_close_prompt_keeps_original_and_never_launches() {
    let directory =
        std::env::temp_dir().join(format!("mode-cancel-prompt-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let mut store = OperationStore::open(&directory.join("operations.sqlite3")).unwrap();
    store.request(request()).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        fail_before_launch: false,
        native_unready: false,
        lose_restore_reply: false,
        lose_launch_reply: false,
        prompt: true,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    assert_eq!(
        store
            .advance(
                "convert-1",
                Choice::Relaunch { confirmed: true },
                &mut runtime
            )
            .unwrap()
            .phase,
        Phase::ClosePrompt
    );
    runtime.prompt = false; // A lost/unproven dismissal must retain custody.
    assert!(store
        .advance("convert-1", Choice::Cancel, &mut runtime)
        .unwrap_err()
        .contains("cancel_unproven"));
    assert_eq!(
        store.read("convert-1").unwrap().unwrap().phase,
        Phase::ClosePrompt
    );
    runtime.prompt = true;
    let cancelled = store
        .advance("convert-1", Choice::Cancel, &mut runtime)
        .unwrap();
    assert_eq!(cancelled.phase, Phase::Cancelled);
    assert_eq!(runtime.process, "boot:pid:start");
    assert_eq!(
        (runtime.closes, runtime.forces, runtime.launches),
        (1, 0, 0)
    );
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn pause_is_durable_until_explicit_relaunch_or_cancel_without_implicit_effects() {
    for cancel in [false, true] {
        let directory = std::env::temp_dir().join(format!("mode-paused-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("operations.sqlite3");
        let mut store = OperationStore::open(&path).unwrap();
        store.request(request()).unwrap();
        let mut runtime = SyntheticRuntime {
            process: "boot:pid:start".into(),
            mode: Mode::Cdp,
            closes: 0,
            launches: 0,
            published: false,
            fail_before_launch: false,
            native_unready: false,
            lose_restore_reply: false,
            lose_launch_reply: false,
            prompt: false,
            uncertain_close: false,
            forces: 0,
            focuses: 0,
        };
        assert_eq!(
            store
                .advance("convert-1", Choice::Pause, &mut runtime)
                .unwrap()
                .phase,
            Phase::Paused
        );
        assert_eq!((runtime.closes, runtime.launches), (0, 0));
        drop(store);
        let mut reopened = OperationStore::open(&path).unwrap();
        assert_eq!(
            reopened
                .advance("convert-1", Choice::Resume, &mut runtime)
                .unwrap()
                .phase,
            Phase::Paused
        );
        assert_eq!((runtime.closes, runtime.launches), (0, 0));
        let choice = if cancel {
            Choice::Cancel
        } else {
            Choice::Relaunch { confirmed: true }
        };
        let outcome = reopened.advance("convert-1", choice, &mut runtime).unwrap();
        assert_eq!(
            outcome.phase,
            if cancel {
                Phase::Cancelled
            } else {
                Phase::Completed
            }
        );
        assert_eq!(
            (runtime.closes, runtime.launches),
            if cancel { (0, 0) } else { (1, 1) }
        );
        drop(reopened);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn prior_executable_selection_recovers_without_claiming_profile_rollback() {
    let directory = std::env::temp_dir().join(format!("mode-restore-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("operations.sqlite3");
    let mut store = OperationStore::open(&path).unwrap();
    let mut selected = request();
    selected.executable = "/synthetic/versioned-target".into();
    store.request(selected).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        lose_launch_reply: false,
        fail_before_launch: true,
        native_unready: false,
        lose_restore_reply: false,
        prompt: false,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    assert!(store
        .advance(
            "convert-1",
            Choice::Relaunch { confirmed: true },
            &mut runtime
        )
        .is_err());
    drop(store);
    let mut store = OperationStore::open(&path).unwrap();
    assert!(store
        .advance(
            "convert-1",
            Choice::RestorePrevious { confirmed: false },
            &mut runtime
        )
        .is_err());
    assert_eq!(runtime.launches, 0);
    let restored = store
        .advance(
            "convert-1",
            Choice::RestorePrevious { confirmed: true },
            &mut runtime,
        )
        .unwrap();
    assert_eq!(restored.phase, Phase::Completed);
    assert_eq!(restored.request.operation_id, "convert-1");
    assert_eq!(restored.request.requested_mode, Mode::CdpFree);
    assert_eq!(restored.replacement.unwrap().mode, Mode::Cdp);
    assert_eq!((runtime.closes, runtime.launches), (1, 1));
    assert!(runtime.published);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn unavailable_build_keeps_original_and_allows_explicit_same_operation_selection() {
    let directory =
        std::env::temp_dir().join(format!("mode-build-selection-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("operations.sqlite3");
    let mut store = OperationStore::open(&path).unwrap();
    let mut selected = request();
    selected.executable = "/synthetic/unavailable".into();
    selected.executable_sha256 = Some("reviewed-missing-build".into());
    store.request(selected).unwrap();
    let mut runtime = SyntheticRuntime {
        process: "boot:pid:start".into(),
        mode: Mode::Cdp,
        closes: 0,
        launches: 0,
        published: false,
        lose_launch_reply: false,
        fail_before_launch: true,
        native_unready: false,
        lose_restore_reply: false,
        prompt: false,
        uncertain_close: false,
        forces: 0,
        focuses: 0,
    };
    assert!(store
        .advance(
            "convert-1",
            Choice::Relaunch { confirmed: true },
            &mut runtime
        )
        .is_err());
    assert_eq!(
        store.read("convert-1").unwrap().unwrap().phase,
        Phase::Prepared,
        "Unavailable target must be rejected before a close is dispatched"
    );
    assert_eq!((runtime.closes, runtime.launches), (0, 0));
    assert!(store
        .advance(
            "convert-1",
            Choice::SelectExecutable {
                executable: "/synthetic/alternative".into(),
                sha256: "alternative-sha".into(),
                confirmed: false
            },
            &mut runtime
        )
        .is_err());
    let reviewed = store
        .advance(
            "convert-1",
            Choice::SelectExecutable {
                executable: "/synthetic/alternative".into(),
                sha256: "alternative-sha".into(),
                confirmed: true,
            },
            &mut runtime,
        )
        .unwrap();
    assert_eq!(
        reviewed.effective_executable.as_deref(),
        Some("/synthetic/alternative")
    );
    assert_eq!(
        (runtime.closes, runtime.launches),
        (0, 0),
        "Selecting a build alone does not relaunch"
    );
    assert!(store
        .advance(
            "convert-1",
            Choice::Relaunch { confirmed: true },
            &mut runtime
        )
        .is_err());
    drop(store);
    let mut store = OperationStore::open(&path).unwrap();
    let absent = store
        .advance("convert-1", Choice::Resume, &mut runtime)
        .unwrap();
    assert_eq!(absent.phase, Phase::LaunchAbsent);
    let next = store
        .advance(
            "convert-1",
            Choice::SelectExecutable {
                executable: "/synthetic/another-build".into(),
                sha256: "another-sha".into(),
                confirmed: true,
            },
            &mut runtime,
        )
        .unwrap();
    assert_eq!(
        next.request.executable, "/synthetic/unavailable",
        "Original request history remains unchanged"
    );
    assert_eq!(
        next.effective_executable.as_deref(),
        Some("/synthetic/another-build")
    );
    assert_eq!((runtime.closes, runtime.launches), (1, 0));
    let completed = store
        .advance(
            "convert-1",
            Choice::Relaunch { confirmed: true },
            &mut runtime,
        )
        .unwrap();
    assert_eq!(completed.phase, Phase::Completed);
    assert_eq!((runtime.closes, runtime.launches), (1, 1));
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}
