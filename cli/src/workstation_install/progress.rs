//! Opt-in observations of the installer, separate from durable work progress.
//!
//! The observer keeps one current snapshot. It cannot mutate a transaction,
//! extend its deadline, or block an installer on a slow stderr consumer.

use serde_json::{json, Value};
use std::cell::RefCell;
use std::io::Write;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const OBSERVATION_INTERVAL: Duration = Duration::from_secs(1);
const FINISH_BUDGET: Duration = Duration::from_millis(250);

thread_local! {
    static CURRENT: RefCell<Option<Arc<Mutex<Value>>>> = const { RefCell::new(None) };
}

pub(super) struct Observer {
    stop: mpsc::Sender<()>,
    result: mpsc::Receiver<Result<u64, ()>>,
    worker: Option<thread::JoinHandle<()>>,
    started: Instant,
}

impl Observer {
    /// Starts before expensive preflight or staging. Only this CLI invocation
    /// owns the observer; an observation is not evidence of a new checkpoint.
    pub(super) fn start() -> Result<Self, String> {
        Self::start_with_output(std::io::stderr())
    }

    fn start_with_output(mut output: impl Write + Send + 'static) -> Result<Self, String> {
        if CURRENT.with(|slot| slot.borrow().is_some()) {
            return Err("install_progress_observer_already_active".into());
        }
        let started = Instant::now();
        let snapshot = Arc::new(Mutex::new(invocation_phase("preflight")));
        let observed = snapshot.clone();
        let (stop, stopping) = mpsc::channel();
        let (completed, result) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("install-progress".into())
            .spawn(move || {
                let outcome = observe(&observed, started, &stopping, &mut output);
                let _ = completed.send(outcome);
            })
            .map_err(|_| "install_progress_observer_start_failed".to_string())?;
        CURRENT.with(|slot| *slot.borrow_mut() = Some(snapshot));
        Ok(Self {
            stop,
            result,
            worker: Some(worker),
            started,
        })
    }

    /// Backpressure affects telemetry, never a committed install. A blocked
    /// observer is detached after this budget and dies with the CLI process.
    pub(super) fn finish(mut self) -> Value {
        CURRENT.with(|slot| *slot.borrow_mut() = None);
        let _ = self.stop.send(());
        let outcome = self.result.recv_timeout(FINISH_BUDGET);
        let (state, count) = match outcome {
            Ok(Ok(count)) => ("finished", Some(count)),
            Ok(Err(())) => ("output_failed", None),
            Err(mpsc::RecvTimeoutError::Timeout) => ("output_backpressure", None),
            Err(mpsc::RecvTimeoutError::Disconnected) => ("observer_failed", None),
        };
        if let Some(worker) = self.worker.take() {
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
        json!({
            "state": state,
            "observationCount": count,
            "applyElapsedMs": millis(self.started.elapsed()),
            "timingScope": "workstation_apply_invocation",
            "buildIncluded": false,
        })
    }
}

impl Drop for Observer {
    fn drop(&mut self) {
        CURRENT.with(|slot| *slot.borrow_mut() = None);
        let _ = self.stop.send(());
        // No join: stderr may be blocked. Process exit also ends the observer
        // on existing installer failure paths which call std::process::exit.
    }
}

fn invocation_phase(phase: &str) -> Value {
    json!({
        "phase": phase,
        "phaseScope": "invocation",
        "phaseStartedAt": super::runtime_adoption_timestamp(),
        "lastProgressAt": null,
        "deadlineAt": null,
        "deadlineScope": null,
    })
}

pub(super) fn phase(phase: &str) {
    replace(invocation_phase(phase));
}

/// Called only after the exact transaction checkpoint was durably written.
pub(super) fn transaction(transaction: &crate::runtime_adoption::UpgradeTransaction) {
    if !CURRENT.with(|slot| slot.borrow().is_some()) {
        return;
    }
    let recorded =
        super::upgrade_transaction_progress(transaction, time::OffsetDateTime::now_utc());
    replace(json!({
        "phase": transaction.state,
        "phaseScope": "transaction",
        "transactionId": transaction.transaction_id,
        "transactionRevision": transaction.revision,
        "phaseStartedAt": recorded["phaseStartedAt"],
        "lastProgressAt": recorded["lastProgressAt"],
        "deadlineAt": recorded["deadlineAt"],
        "deadlineScope": "runtime_host_convergence",
    }));
}

fn replace(value: Value) {
    CURRENT.with(|slot| {
        if let Some(snapshot) = slot.borrow().as_ref() {
            if let Ok(mut current) = snapshot.lock() {
                *current = value;
            }
        }
    });
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn event(
    snapshot: &Value,
    kind: &str,
    sequence: u64,
    elapsed: Duration,
    now: time::OffsetDateTime,
) -> Value {
    let mut value = snapshot.clone();
    let phase_elapsed = snapshot["phaseStartedAt"]
        .as_str()
        .and_then(|start| chrono::DateTime::parse_from_rfc3339(start).ok())
        .and_then(|start| {
            u64::try_from(
                (now.unix_timestamp_nanos() / 1_000_000 - i128::from(start.timestamp_millis()))
                    .max(0),
            )
            .ok()
        });
    value["schemaVersion"] = json!("agent-browser.workstation-install-observation.v1");
    value["event"] = json!(kind);
    value["sequence"] = json!(sequence);
    value["elapsedMs"] = json!(millis(elapsed));
    value["phaseElapsedMs"] = json!(phase_elapsed);
    value["observedAt"] = json!(now
        .format(&time::format_description::well_known::Rfc3339)
        .ok());
    value["heartbeatScope"] = json!("installer_observer");
    value
}

fn observe(
    snapshot: &Mutex<Value>,
    started: Instant,
    stopping: &mpsc::Receiver<()>,
    output: &mut impl Write,
) -> Result<u64, ()> {
    let mut sequence = 0;
    let mut kind = "started";
    let mut previous = Value::Null;
    loop {
        let current = snapshot.lock().map_err(|_| ())?.clone();
        if kind == "heartbeat" && current != previous {
            kind = "phase_observed";
        }
        let observed = event(
            &current,
            kind,
            sequence,
            started.elapsed(),
            time::OffsetDateTime::now_utc(),
        );
        let line = serde_json::to_vec(&observed).map_err(|_| ())?;
        output.write_all(&line).map_err(|_| ())?;
        output.write_all(b"\n").map_err(|_| ())?;
        output.flush().map_err(|_| ())?;
        sequence = sequence.saturating_add(1);
        previous = current;
        if kind == "observer_finished" {
            return Ok(sequence);
        }
        kind = match stopping.recv_timeout(OBSERVATION_INTERVAL) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => "observer_finished",
            Err(mpsc::RecvTimeoutError::Timeout) => "heartbeat",
        };
    }
}

#[cfg(test)]
pub(super) fn capture<T>(action: impl FnOnce() -> T) -> (T, Value) {
    let snapshot = Arc::new(Mutex::new(Value::Null));
    CURRENT.with(|slot| *slot.borrow_mut() = Some(snapshot.clone()));
    let result = action();
    CURRENT.with(|slot| *slot.borrow_mut() = None);
    let captured = snapshot.lock().unwrap().clone();
    (result, captured)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BlockedOutput {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
        dropped: mpsc::Sender<()>,
        blocked_once: bool,
    }

    impl Write for BlockedOutput {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if !self.blocked_once {
                self.blocked_once = true;
                self.entered.send(()).unwrap();
                self.release.recv().unwrap();
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Drop for BlockedOutput {
        fn drop(&mut self) {
            let _ = self.dropped.send(());
        }
    }

    #[test]
    fn progress_output_backpressure_cannot_hold_install_completion() {
        let (entered, writing) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let (dropped, retired) = mpsc::channel();
        let observer = Observer::start_with_output(BlockedOutput {
            entered,
            release: blocked,
            dropped,
            blocked_once: false,
        })
        .unwrap();
        writing.recv_timeout(Duration::from_secs(2)).unwrap();
        let started = Instant::now();
        let receipt = observer.finish();
        let elapsed = started.elapsed();
        release.send(()).unwrap();
        retired.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(receipt["state"], "output_backpressure");
        assert!(elapsed < Duration::from_secs(1));
        assert!(!CURRENT.with(|slot| slot.borrow().is_some()));
    }

    #[test]
    fn observer_heartbeat_does_not_advance_recorded_progress_or_deadline() {
        let snapshot = json!({"phase":"post_commit_validating","phaseScope":"transaction",
            "phaseStartedAt":"2026-10-11T04:00:00Z","lastProgressAt":"2026-10-11T04:00:00Z",
            "deadlineAt":"2026-10-11T04:01:30Z","deadlineScope":"runtime_host_convergence"});
        let before = snapshot.clone();
        let now = time::OffsetDateTime::from_unix_timestamp(1791691260).unwrap();
        let first = event(&snapshot, "heartbeat", 1, Duration::from_secs(60), now);
        let later = event(
            &snapshot,
            "heartbeat",
            2,
            Duration::from_secs(61),
            now + time::Duration::seconds(1),
        );
        assert_eq!(first["phaseElapsedMs"], 60_000);
        assert_eq!(later["phaseElapsedMs"], 61_000);
        assert_eq!(first["lastProgressAt"], later["lastProgressAt"]);
        assert_eq!(first["deadlineAt"], later["deadlineAt"]);
        assert_ne!(first["observedAt"], later["observedAt"]);
        assert_eq!(snapshot, before);
    }
}
