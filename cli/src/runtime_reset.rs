//! Local production maintenance, deliberately outside daemon dispatch: a reset
//! must be able to stop its runtime writer. Profiles and Remote View services
//! retain their independent ownership; there is no profile-data reset here.
use crate::candidate_coordination::{CandidateCoordinationStore, CandidateInstallCustody};
use crate::native::browser_session_store::{
    commit_reset, reset_backup, reset_backup_digest, reset_preview, reset_result,
};
use crate::process_identity::RecordedProcessIdentity;
#[cfg(target_os = "linux")]
use crate::process_identity::VerifiedProcessTermination;
#[cfg(target_os = "linux")]
use crate::process_identity::{capture_process_identity, VerifiedProcessSignal};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

const HOST: &str = "agent-browser-runtime-host.service";
const UNITS: &[&str] = &[
    "agent-browser-runtime-interlock.timer",
    "agent-browser-dashboard.service",
    "agent-browser-dashboard-backend.service",
    HOST,
];

#[derive(Default, Debug, PartialEq, Eq)]
struct Options {
    apply: bool,
    resume: Option<String>,
}
fn options(args: &[String]) -> Result<Options, String> {
    let mut result = Options::default();
    let mut index = 0;
    let mut dry = false;
    while index < args.len() {
        match args[index].as_str() {
            "--apply" if !result.apply => result.apply = true,
            "--dry-run" if !dry => dry = true,
            "--resume" if result.resume.is_none() => {
                index += 1;
                let id = args.get(index).ok_or("runtime_reset_resume_id_required")?;
                uuid::Uuid::parse_str(id).map_err(|_| "runtime_reset_resume_id_invalid")?;
                result.resume = Some(id.clone());
            }
            _ => return Err(format!("runtime_reset_argument_invalid:{}", args[index])),
        }
        index += 1;
    }
    if (dry && result.apply) || (result.resume.is_some() && !result.apply) {
        return Err("runtime_reset_arguments_conflict".into());
    }
    Ok(result)
}

/// A preview is read-only; apply/resume are explicit whole-production effects.
/// Never route this through a browser command or a remotely callable action.
pub(crate) fn run(args: &[String]) -> Result<Value, String> {
    let options = options(args)?;
    let home = dirs::home_dir().ok_or("runtime_reset_home_missing")?;
    if options.apply {
        validate_production_home(&home)?;
    }
    if options.apply
        && options.resume.is_none()
        && !crate::native::browser_session_store::BrowserSessionSqliteStore::default_sqlite_path()?
            .exists()
    {
        return Err("runtime_reset_database_missing: no reset effects applied".into());
    }
    let adapter = LinuxEffects;
    let coordination = CandidateCoordinationStore::production(&home);
    if !options.apply {
        return Ok(
            json!({"schemaVersion":"agent-browser.runtime-reset-plan.v1","dryRun":true,
            "scope":"production","liveStateWillEnd":true,"profilesPreserved":true,"journalsPreserved":true,
            "activeRecords":reset_preview()?,"ownedBrowserProcesses":adapter.inventory(&adapter.group()?)?.len(),
            "activeOperation":coordination.read()?.active(),"units":UNITS,
            "nextAction":"agent-browser runtime reset --apply"}),
        );
    }
    let id = options
        .resume
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let directory = home.join(".agent-browser/service/runtime-resets").join(&id);
    let path = directory.join("receipt.json");
    let mut receipt = if options.resume.is_some() {
        let receipt: Receipt = serde_json::from_slice(
            &fs::read(&path).map_err(|e| format!("runtime_reset_resume_read:{e}"))?,
        )
        .map_err(|e| format!("runtime_reset_resume_invalid:{e}"))?;
        if receipt.id != id {
            return Err("runtime_reset_receipt_identity_mismatch".into());
        }
        if receipt.phase == "completed" {
            finish_completed(&coordination, &receipt)?;
            return Ok(
                json!({"receipt":path,"result":receipt.result,"state":"completed","replayed":true}),
            );
        }
        receipt
    } else {
        Receipt {
            id: id.clone(),
            phase: "prepared".into(),
            restore_units: adapter.active_units()?,
            cgroup: adapter.group()?,
            executor: None,
            custody: None,
            result: None,
            backup_sha256: None,
            error: None,
        }
    };
    // Persist the intended restoration set before acquiring coordination. A
    // crash between acquisition and executor admission can rejoin this ID.
    if receipt.custody.is_none() {
        persist(&path, &receipt)?;
    }
    let custody = match receipt.custody.as_ref() {
        Some(custody) => custody.clone(),
        None => coordination.start_or_join_install(
            &format!("runtime-reset-{id}-{}", uuid::Uuid::new_v4()),
            &format!("runtime-reset-{id}"),
            "runtime-reset-v1",
        ).map_err(|error| format!("runtime_reset_operation_conflict:{error}; no reset effects applied; inspect agent-browser candidate status; receipt={}",path.display()))?,
    };
    receipt.custody = Some(custody.clone());
    // Bounded atomic executor admission prevents two processes joining one
    // operation from performing its side effects concurrently. No file lock
    // spans process waits, backups, or service stop/start commands.
    coordination.with_active_install(&custody, |_| {
        if path.exists() {
            let existing: Receipt =
                serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if let Some(executor) = existing.executor {
                if crate::process_identity::recorded_process_is_running(&executor)? {
                    return Err(format!("runtime_reset_executor_active:{}", executor.pid));
                }
            }
        }
        receipt.executor =
            crate::process_identity::capture_process_identity(std::process::id(), None, None);
        if receipt.executor.is_none() {
            return Err("runtime_reset_executor_identity_unavailable".into());
        }
        persist(&path, &receipt)
    })?;
    let result = execute(&adapter, &mut receipt, &path, &coordination, &custody);
    if let Err(error) = &result {
        receipt.error = Some(error.clone());
        receipt.executor = None;
        let _ = coordination.with_active_install(&custody, |_| persist(&path, &receipt));
        return Err(format!(
            "{error}; receipt={}; resume: agent-browser runtime reset --apply --resume {id}",
            path.display()
        ));
    }
    result
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Receipt {
    id: String,
    phase: String,
    restore_units: Vec<String>,
    cgroup: String,
    executor: Option<RecordedProcessIdentity>,
    custody: Option<CandidateInstallCustody>,
    result: Option<Value>,
    backup_sha256: Option<String>,
    error: Option<String>,
}

fn persist(path: &Path, receipt: &Receipt) -> Result<(), String> {
    #[cfg(unix)]
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let parent = path
        .parent()
        .ok_or("runtime_reset_receipt_parent_missing")?;
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(".receipt-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&temporary).map_err(|e| e.to_string())?;
    file.write_all(
        serde_json::to_string_pretty(receipt)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(&temporary, path).map_err(|e| e.to_string())?;
    fs::File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())
}

trait Effects {
    fn published(&self, id: &str) -> Result<Option<Value>, String>;
    fn group(&self) -> Result<String, String>;
    fn inventory(&self, group: &str) -> Result<Vec<RecordedProcessIdentity>, String>;
    fn backup(&self, path: &Path) -> Result<Option<String>, String>;
    fn publish(
        &self,
        id: &str,
        now: u64,
        coordination: &CandidateCoordinationStore,
        custody: &CandidateInstallCustody,
    ) -> Result<Value, String>;
    fn active_units(&self) -> Result<Vec<String>, String>;
    fn unit(&self, action: &str, unit: &str) -> Result<(), String>;
    fn quiescent(&self) -> Result<(), String>;
    fn close(
        &self,
        processes: &[RecordedProcessIdentity],
        coordination: &CandidateCoordinationStore,
        custody: &CandidateInstallCustody,
    ) -> Result<(), String>;
}
struct LinuxEffects;

fn finish_completed(
    coordination: &CandidateCoordinationStore,
    receipt: &Receipt,
) -> Result<(), String> {
    let custody = receipt
        .custody
        .as_ref()
        .ok_or("runtime_reset_custody_missing")?;
    let ledger = coordination.read()?;
    if ledger.receipts().iter().any(|r| {
        r.operation_id.as_deref() == Some(custody.operation_id.as_str())
            && r.outcome == agent_browser_candidate::CoordinationOutcome::Completed
    }) {
        return Ok(());
    }
    coordination.complete_active_install(
        custody,
        &format!("runtime-reset-complete-{}", receipt.id),
        |_| Ok(()),
    )?;
    Ok(())
}

fn execute(
    effects: &impl Effects,
    receipt: &mut Receipt,
    path: &Path,
    coordination: &CandidateCoordinationStore,
    custody: &CandidateInstallCustody,
) -> Result<Value, String> {
    if effects.group()? != receipt.cgroup {
        return Err("runtime_reset_receipt_cgroup_invalid".into());
    }
    let checkpoint =
        |receipt: &Receipt| coordination.with_active_install(custody, |_| persist(path, receipt));
    if receipt.phase != "committed" && receipt.phase != "restored" {
        if let Some(result) = effects.published(&receipt.id)? {
            receipt.result = Some(result);
            receipt.phase = "committed".into();
            checkpoint(receipt)?;
        }
    }
    if receipt.phase != "committed" && receipt.phase != "restored" {
        // Restoring the original set is idempotent after interruption. Only
        // these immutable allowlisted units may be affected by a receipt.
        if receipt
            .restore_units
            .iter()
            .any(|u| !UNITS.contains(&u.as_str()))
        {
            return Err("runtime_reset_receipt_unit_invalid".into());
        }
        receipt.phase = "stopping".into();
        checkpoint(receipt)?;
        for unit in UNITS {
            coordination.with_active_install(custody, |_| Ok(()))?;
            effects.unit("stop", unit)?;
        }
        effects.quiescent()?;
        let backup = path
            .parent()
            .ok_or("runtime_reset_receipt_parent_missing")?
            .join("runtime.before.sqlite3");
        let digest = effects.backup(&backup)?;
        if receipt.backup_sha256.is_some() && receipt.backup_sha256 != digest {
            return Err("runtime_reset_backup_digest_changed".into());
        }
        receipt.backup_sha256 = digest;
        receipt.phase = "closing".into();
        checkpoint(receipt)?;
        let processes = effects.inventory(&receipt.cgroup)?;
        effects.close(&processes, coordination, custody)?;
        effects.quiescent()?;
        if !effects.inventory(&receipt.cgroup)?.is_empty() {
            return Err("runtime_reset_owned_processes_remain".into());
        }
        receipt.phase = "publishing".into();
        checkpoint(receipt)?;
        let now = chrono::Utc::now()
            .timestamp_millis()
            .try_into()
            .map_err(|_| "runtime_reset_clock_invalid")?;
        receipt.result = Some(effects.publish(&receipt.id, now, coordination, custody)?);
        receipt.phase = "committed".into();
        checkpoint(receipt)?;
    }
    if receipt
        .restore_units
        .iter()
        .any(|u| !UNITS.contains(&u.as_str()))
    {
        return Err("runtime_reset_receipt_unit_invalid".into());
    }
    for unit in receipt.restore_units.iter().rev() {
        coordination.with_active_install(custody, |_| Ok(()))?;
        effects.unit("start", unit)?;
    }
    receipt.phase = "restored".into();
    checkpoint(receipt)?;
    // Mark completed and release coordination in one short commit. A crash
    // after this point replays the receipt and never closes newly started work.
    coordination.complete_active_install(
        custody,
        &format!("runtime-reset-complete-{}", receipt.id),
        |_| {
            receipt.phase = "completed".into();
            receipt.executor = None;
            receipt.error = None;
            persist(path, receipt)
        },
    )?;
    Ok(
        json!({"state":"completed","receipt":path,"result":receipt.result,"profilesPreserved":true,"journalsPreserved":true}),
    )
}

#[cfg(target_os = "linux")]
fn production_cgroup() -> String {
    // SAFETY: geteuid has no pointer arguments or mutable process state.
    let uid = unsafe { libc::geteuid() };
    format!("/user.slice/user-{uid}.slice/user@{uid}.service/app.slice/{HOST}")
}
#[cfg(target_os = "linux")]
fn validate_production_home(home: &Path) -> Result<(), String> {
    // SAFETY: this synchronous CLI copies pw_dir before any other libc call.
    let account = unsafe { libc::getpwuid(libc::geteuid()) };
    if account.is_null() {
        return Err("runtime_reset_account_home_unavailable".into());
    }
    // SAFETY: getpwuid returned a valid passwd record and NUL-terminated pw_dir.
    let configured = unsafe { std::ffi::CStr::from_ptr((*account).pw_dir) };
    let expected = Path::new(
        std::str::from_utf8(configured.to_bytes())
            .map_err(|_| "runtime_reset_account_home_invalid")?,
    );
    if home != expected {
        return Err("runtime_reset_requires_production_home".into());
    }
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn validate_production_home(_: &Path) -> Result<(), String> {
    Err("runtime_reset_requires_linux_user_systemd".into())
}

#[cfg(target_os = "linux")]
fn systemctl(args: &[&str]) -> Result<String, String> {
    let mut child = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("runtime_reset_systemd:{e}"))?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("runtime_reset_systemd_timeout".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "runtime_reset_systemd_failed:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(target_os = "linux")]
impl Effects for LinuxEffects {
    fn published(&self, id: &str) -> Result<Option<Value>, String> {
        reset_result(id)
    }
    fn group(&self) -> Result<String, String> {
        // An inactive unit can report an empty ControlGroup even while its
        // KillMode=process children survive. Keep the exact known unit path.
        let expected = production_cgroup();
        let current = systemctl(&["show", HOST, "--property=ControlGroup", "--value"])?;
        if !current.is_empty() && current != expected {
            return Err("runtime_reset_unexpected_production_cgroup".into());
        }
        Ok(expected)
    }
    fn backup(&self, path: &Path) -> Result<Option<String>, String> {
        if !crate::native::browser_session_store::BrowserSessionSqliteStore::default_sqlite_path()?
            .exists()
        {
            return Err("runtime_reset_database_missing".into());
        }
        if !path.exists() {
            reset_backup(path)?;
        }
        Ok(Some(reset_backup_digest(path)?))
    }
    fn publish(
        &self,
        id: &str,
        now: u64,
        coordination: &CandidateCoordinationStore,
        custody: &CandidateInstallCustody,
    ) -> Result<Value, String> {
        commit_reset(id, now, coordination, custody)
    }
    fn inventory(&self, group: &str) -> Result<Vec<RecordedProcessIdentity>, String> {
        if group != production_cgroup() {
            return Err("runtime_reset_receipt_cgroup_invalid".into());
        }
        if group.is_empty() {
            return Ok(Vec::new());
        }
        let membership = Path::new("/sys/fs/cgroup")
            .join(group.trim_start_matches('/'))
            .join("cgroup.procs");
        let members = match fs::read_to_string(&membership) {
            Ok(members) => members,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(format!(
                    "runtime_reset_cgroup_inventory_unavailable:{error}"
                ))
            }
        };
        let mut result = Vec::new();
        for member in members.lines() {
            let pid = member
                .parse::<u32>()
                .map_err(|_| "runtime_reset_cgroup_pid_invalid")?;
            let path = Path::new("/proc").join(pid.to_string());
            let cgroup = match fs::read_to_string(path.join("cgroup")) {
                Ok(group) => group,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(format!(
                        "runtime_reset_process_cgroup_unavailable:{pid}:{error}"
                    ))
                }
            };
            if cgroup.trim() != format!("0::{group}") {
                continue;
            }
            let executable = match fs::read_link(path.join("exe")) {
                Ok(exe) => exe,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(format!("runtime_reset_process_unverified:{pid}:{e}")),
            };
            if !browser_executable(&executable) {
                continue;
            }
            let identity = capture_process_identity(pid, Some(&executable), None)
                .ok_or_else(|| format!("runtime_reset_process_identity_unavailable:{pid}"))?;
            result.push(identity);
        }
        result.sort_by_key(|p| p.pid);
        Ok(result)
    }
    fn active_units(&self) -> Result<Vec<String>, String> {
        let mut active = Vec::new();
        for unit in UNITS {
            let state = systemctl(&["show", unit, "--property=ActiveState", "--value"])?;
            match state.as_str() {
                "active" => active.push((*unit).into()),
                "inactive" | "failed" => {}
                _ => return Err(format!("runtime_reset_unit_transitioning:{unit}:{state}")),
            }
        }
        Ok(active)
    }
    fn unit(&self, action: &str, unit: &str) -> Result<(), String> {
        systemctl(&[action, unit]).map(|_| ())
    }
    fn quiescent(&self) -> Result<(), String> {
        let state = systemctl(&["show", HOST, "--property=MainPID", "--value"])?;
        if state != "0" {
            return Err("runtime_reset_writer_still_running".into());
        }
        Ok(())
    }
    fn close(
        &self,
        processes: &[RecordedProcessIdentity],
        coordination: &CandidateCoordinationStore,
        custody: &CandidateInstallCustody,
    ) -> Result<(), String> {
        let mut handles = Vec::new();
        for identity in processes {
            if let Some(process) = VerifiedProcessTermination::open(identity)? {
                handles.push(process);
            }
        }
        for process in &handles {
            coordination.with_active_install(custody, |_| {
                process.signal(VerifiedProcessSignal::Terminate)
            })?;
        }
        wait(&handles, Duration::from_secs(10))?;
        for process in &handles {
            if process.is_running()? {
                coordination.with_active_install(custody, |_| {
                    process.signal(VerifiedProcessSignal::Kill)
                })?;
            }
        }
        wait(&handles, Duration::from_secs(5))?;
        for process in &handles {
            if process.is_running()? {
                return Err("runtime_reset_process_exit_unproven".into());
            }
        }
        Ok(())
    }
}
fn browser_executable(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("chrome" | "chromium" | "chromium-browser" | "chrome_crashpad_handler" | "lightpanda")
    )
}
#[cfg(target_os = "linux")]
fn wait(handles: &[VerifiedProcessTermination], timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let running = handles
            .iter()
            .map(VerifiedProcessTermination::is_running)
            .collect::<Result<Vec<_>, _>>()?;
        if running.iter().all(|running| !*running) {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(())
}
#[cfg(not(target_os = "linux"))]
impl Effects for LinuxEffects {
    fn published(&self, _: &str) -> Result<Option<Value>, String> {
        Err("runtime_reset_requires_linux_user_systemd".into())
    }
    fn group(&self) -> Result<String, String> {
        Err("runtime_reset_requires_linux_user_systemd".into())
    }
    fn backup(&self, _: &Path) -> Result<Option<String>, String> {
        Err("runtime_reset_requires_linux_user_systemd".into())
    }
    fn publish(
        &self,
        _: &str,
        _: u64,
        _: &CandidateCoordinationStore,
        _: &CandidateInstallCustody,
    ) -> Result<Value, String> {
        Err("runtime_reset_requires_linux_user_systemd".into())
    }
    fn inventory(&self, _: &str) -> Result<Vec<RecordedProcessIdentity>, String> {
        Err("runtime_reset_requires_linux_user_systemd".into())
    }
    fn active_units(&self) -> Result<Vec<String>, String> {
        Err("runtime_reset_requires_linux_user_systemd".into())
    }
    fn unit(&self, _: &str, _: &str) -> Result<(), String> {
        Err("runtime_reset_requires_linux_user_systemd".into())
    }
    fn quiescent(&self) -> Result<(), String> {
        Err("runtime_reset_requires_linux_user_systemd".into())
    }
    fn close(
        &self,
        _: &[RecordedProcessIdentity],
        _: &CandidateCoordinationStore,
        _: &CandidateInstallCustody,
    ) -> Result<(), String> {
        Err("runtime_reset_requires_linux_user_systemd".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_reset_requires_explicit_apply_and_validates_resume() {
        assert_eq!(options(&[]).unwrap(), Options::default());
        for args in [
            vec!["--apply", "--dry-run"],
            vec!["--resume", "../../other"],
            vec!["--apply", "--pid", "1"],
            vec!["--resume"],
        ] {
            assert!(options(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).is_err());
        }
        let id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            options(&["--apply".into(), "--resume".into(), id.clone()]).unwrap(),
            Options {
                apply: true,
                resume: Some(id)
            }
        );
    }
    #[test]
    fn runtime_reset_executable_selection_excludes_daemon_and_foreign_tools() {
        assert!(browser_executable(Path::new("/installed/chrome")));
        assert!(!browser_executable(Path::new("/installed/agent-browser")));
        assert!(!browser_executable(Path::new(
            "/installed/chrome-helper-untrusted"
        )));
    }
    #[derive(Default)]
    struct FakeEffects {
        calls: std::cell::RefCell<Vec<String>>,
        fail_start: std::cell::Cell<bool>,
        published: std::cell::Cell<bool>,
    }
    impl Effects for FakeEffects {
        fn published(&self, _: &str) -> Result<Option<Value>, String> {
            Ok(self
                .published
                .get()
                .then(|| json!({"after":{"sessions":0}})))
        }
        fn group(&self) -> Result<String, String> {
            Ok("fixture".into())
        }
        fn inventory(&self, _: &str) -> Result<Vec<RecordedProcessIdentity>, String> {
            self.calls.borrow_mut().push("inventory".into());
            Ok(vec![])
        }
        fn active_units(&self) -> Result<Vec<String>, String> {
            Ok(vec![HOST.into()])
        }
        fn unit(&self, action: &str, _: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(action.into());
            if action == "start" && self.fail_start.replace(false) {
                return Err("fixture_start_failure".into());
            }
            Ok(())
        }
        fn quiescent(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("quiescent".into());
            Ok(())
        }
        fn close(
            &self,
            _: &[RecordedProcessIdentity],
            _: &CandidateCoordinationStore,
            _: &CandidateInstallCustody,
        ) -> Result<(), String> {
            self.calls.borrow_mut().push("close".into());
            Ok(())
        }
        fn backup(&self, _: &Path) -> Result<Option<String>, String> {
            self.calls.borrow_mut().push("backup".into());
            Ok(Some("fixture-digest".into()))
        }
        fn publish(
            &self,
            _: &str,
            _: u64,
            _: &CandidateCoordinationStore,
            _: &CandidateInstallCustody,
        ) -> Result<Value, String> {
            self.calls.borrow_mut().push("publish".into());
            self.published.set(true);
            Ok(json!({"after":{"sessions":0}}))
        }
    }
    #[test]
    fn runtime_reset_resume_after_publication_never_repeats_closure() {
        let root =
            std::env::temp_dir().join(format!("runtime-reset-test-{}", uuid::Uuid::new_v4()));
        let store = CandidateCoordinationStore::production(&root);
        let custody = store
            .start_or_join_install("reset-test", "reset-test", "fixture")
            .unwrap();
        let path = root.join("receipt.json");
        let mut receipt = Receipt {
            id: "fixture".into(),
            phase: "prepared".into(),
            restore_units: vec![HOST.into()],
            cgroup: "fixture".into(),
            executor: None,
            custody: Some(custody.clone()),
            result: None,
            backup_sha256: None,
            error: None,
        };
        let effects = FakeEffects::default();
        effects.fail_start.set(true);
        assert_eq!(
            execute(&effects, &mut receipt, &path, &store, &custody).unwrap_err(),
            "fixture_start_failure"
        );
        assert_eq!(receipt.phase, "committed");
        assert!(store.read().unwrap().active().is_some());
        // Simulate a crash after the database committed but before the
        // external receipt advanced. New work must not be closed on resume.
        receipt.phase = "publishing".into();
        effects.calls.borrow_mut().clear();
        execute(&effects, &mut receipt, &path, &store, &custody).unwrap();
        assert_eq!(*effects.calls.borrow(), vec!["start"]);
        assert_eq!(receipt.phase, "completed");
        assert!(store.read().unwrap().active().is_none());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn runtime_reset_rejects_receipt_foreign_units_before_effects() {
        let root =
            std::env::temp_dir().join(format!("runtime-reset-test-{}", uuid::Uuid::new_v4()));
        let store = CandidateCoordinationStore::production(&root);
        let custody = store
            .start_or_join_install("reset-test", "reset-test", "fixture")
            .unwrap();
        let path = root.join("receipt.json");
        let mut receipt = Receipt {
            id: "fixture".into(),
            phase: "prepared".into(),
            restore_units: vec!["im-receipts.service".into()],
            cgroup: "fixture".into(),
            executor: None,
            custody: Some(custody.clone()),
            result: None,
            backup_sha256: None,
            error: None,
        };
        let effects = FakeEffects::default();
        assert_eq!(
            execute(&effects, &mut receipt, &path, &store, &custody).unwrap_err(),
            "runtime_reset_receipt_unit_invalid"
        );
        assert!(effects.calls.borrow().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn runtime_reset_completed_receipt_releases_interrupted_coordination_without_effects() {
        let root =
            std::env::temp_dir().join(format!("runtime-reset-test-{}", uuid::Uuid::new_v4()));
        let store = CandidateCoordinationStore::production(&root);
        let custody = store
            .start_or_join_install("reset-test", "reset-test", "fixture")
            .unwrap();
        let receipt = Receipt {
            id: "fixture".into(),
            phase: "completed".into(),
            restore_units: vec![HOST.into()],
            cgroup: "fixture".into(),
            executor: None,
            custody: Some(custody),
            result: Some(json!({"after":{"sessions":0}})),
            backup_sha256: None,
            error: None,
        };
        finish_completed(&store, &receipt).unwrap();
        assert!(store.read().unwrap().active().is_none());
        finish_completed(&store, &receipt).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn runtime_reset_kernel_handle_closes_only_the_selected_process() {
        let mut owned = Command::new("sleep").arg("30").spawn().unwrap();
        let mut peer = Command::new("sleep").arg("30").spawn().unwrap();
        let identity = capture_process_identity(owned.id(), None, None).unwrap();
        let root =
            std::env::temp_dir().join(format!("runtime-reset-test-{}", uuid::Uuid::new_v4()));
        let store = CandidateCoordinationStore::production(&root);
        let custody = store
            .start_or_join_install("process-test", "process-test", "fixture")
            .unwrap();
        let result = LinuxEffects.close(&[identity], &store, &custody);
        let peer_live = peer.try_wait().unwrap().is_none();
        let _ = owned.kill();
        let _ = owned.wait();
        let _ = peer.kill();
        let _ = peer.wait();
        result.unwrap();
        assert!(peer_live);
        fs::remove_dir_all(root).unwrap();
    }
}
