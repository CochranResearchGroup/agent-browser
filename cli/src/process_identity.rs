use serde::{Deserialize, Serialize};
use std::path::Path;
#[cfg(any(target_os = "macos", windows))]
use std::path::PathBuf;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedProcessIdentity {
    pub pid: u32,
    pub start_token: Option<String>,
    pub executable_path: Option<String>,
    pub browser_family: Option<String>,
    pub command_line: Option<Vec<String>>,
}

/// Relationship between retained ephemeral evidence and the current host boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BootEpochStatus {
    Current,
    Prior,
    Missing,
    Unavailable,
}

/// Return one stable epoch for the current operating-system boot when the
/// platform exposes enough process identity evidence.
pub(crate) fn current_boot_epoch() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        return std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(|value| format!("linux:{value}"));
    }
    #[cfg(target_os = "macos")]
    {
        return process_start_epoch(1);
    }
    #[cfg(windows)]
    {
        return process_start_epoch(4);
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(any(target_os = "macos", windows))]
fn process_start_epoch(pid: u32) -> Option<String> {
    match observe_process(pid) {
        ProcessObservation::Observed(observation) => observation
            .start_token
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("host-process:{value}")),
        ProcessObservation::Missing | ProcessObservation::Failed { .. } => None,
    }
}

pub(crate) fn boot_epoch_status(
    recorded_boot_epoch: Option<&str>,
    current_boot_epoch: Option<&str>,
) -> BootEpochStatus {
    match (recorded_boot_epoch, current_boot_epoch) {
        (_, None) => BootEpochStatus::Unavailable,
        (None, Some(_)) => BootEpochStatus::Missing,
        (Some(recorded), Some(current)) if recorded == current => BootEpochStatus::Current,
        (Some(_), Some(_)) => BootEpochStatus::Prior,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProcessOwnership {
    MatchingBrowser,
    Missing,
    ReusedUnrelated,
    AmbiguousLegacyBrowser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessObservation {
    Missing,
    Observed(ObservedProcessIdentity),
    Failed { reason: String },
}

/// Return a browser command-line option from either a conventional argv entry
/// or a platform observation that retained the executable and flags together.
pub(crate) fn command_line_option_value<'a>(
    arguments: &'a [String],
    option: &str,
) -> Option<&'a str> {
    for (index, argument) in arguments.iter().enumerate() {
        if let Some(value) = argument.strip_prefix(&format!("{option}=")) {
            return Some(value);
        }
        if argument == option {
            return arguments.get(index + 1).map(String::as_str);
        }

        let mut search_from = 0;
        while let Some(relative_start) = argument[search_from..].find(option) {
            let start = search_from + relative_start;
            let boundary_is_valid = start == 0
                || argument[..start]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace);
            let suffix = &argument[start + option.len()..];
            if boundary_is_valid {
                let value = suffix
                    .strip_prefix('=')
                    .or_else(|| {
                        suffix
                            .chars()
                            .next()
                            .is_some_and(char::is_whitespace)
                            .then_some(suffix)
                    })
                    .map(str::trim_start);
                if let Some(value) = value.filter(|value| !value.is_empty()) {
                    let value = if let Some(quote) =
                        value.chars().next().filter(|c| matches!(c, '\'' | '"'))
                    {
                        let quoted = &value[quote.len_utf8()..];
                        &quoted[..quoted.find(quote).unwrap_or(quoted.len())]
                    } else {
                        &value[..value.find(char::is_whitespace).unwrap_or(value.len())]
                    };
                    if !value.is_empty() {
                        return Some(value);
                    }
                }
            }
            search_from = start + option.len();
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyProfileProof {
    Unproven,
    ProfileConsistent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProcessAssessment {
    pub ownership: RuntimeProcessOwnership,
    pub observation: ProcessObservation,
    pub reason: &'static str,
}

impl RuntimeProcessAssessment {
    pub fn authorizes_adoption(&self) -> bool {
        self.ownership == RuntimeProcessOwnership::MatchingBrowser
    }

    pub fn authorizes_cleanup(&self) -> bool {
        matches!(
            self.ownership,
            RuntimeProcessOwnership::Missing | RuntimeProcessOwnership::ReusedUnrelated
        )
    }

    pub fn preserves_evidence(&self) -> bool {
        !self.authorizes_cleanup()
    }
}

pub fn capture_process_identity(
    pid: u32,
    expected_executable: Option<&Path>,
    expected_browser_family: Option<&str>,
) -> Option<RecordedProcessIdentity> {
    let mut observed = None;
    for attempt in 0..10 {
        match observe_process(pid) {
            ProcessObservation::Observed(candidate) if candidate.start_token.is_some() => {
                observed = Some(candidate);
                break;
            }
            ProcessObservation::Observed(candidate) => observed = Some(candidate),
            ProcessObservation::Missing | ProcessObservation::Failed { .. } => observed = None,
        }
        if attempt < 9 {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    let observed = observed?;
    let start_token = observed.start_token?;
    let executable_path = observed.executable_path?;
    if expected_executable
        .is_some_and(|expected| !executable_paths_match(expected, Path::new(&executable_path)))
        || expected_browser_family
            .is_some_and(|expected| observed.browser_family.as_deref() != Some(expected))
    {
        return None;
    }
    Some(RecordedProcessIdentity {
        pid,
        start_token,
        executable_path: Some(executable_path),
        browser_family: observed.browser_family,
    })
}

/// Capture the final browser executable after a package launcher or vendor
/// wrapper has completed. A wrapper transition is accepted only for a script
/// and same-family executable in the same canonical install directory.
pub fn capture_launched_browser_identity(
    pid: u32,
    requested_executable: &Path,
    expected_browser_family: Option<&str>,
) -> Option<RecordedProcessIdentity> {
    let identity = capture_process_identity(pid, None, expected_browser_family)?;
    let observed_executable = identity.executable_path.as_deref().map(Path::new)?;
    browser_launch_paths_match(
        requested_executable,
        observed_executable,
        expected_browser_family,
    )
    .then_some(identity)
}

pub fn assess_process_ownership(
    recorded: Option<&RecordedProcessIdentity>,
    observation: ProcessObservation,
    legacy_profile_proof: LegacyProfileProof,
) -> RuntimeProcessAssessment {
    let observed = match &observation {
        ProcessObservation::Missing => {
            return RuntimeProcessAssessment {
                ownership: RuntimeProcessOwnership::Missing,
                observation,
                reason: "process_missing",
            };
        }
        ProcessObservation::Failed { .. } => {
            return RuntimeProcessAssessment {
                ownership: RuntimeProcessOwnership::AmbiguousLegacyBrowser,
                observation,
                reason: "process_observation_failed",
            };
        }
        ProcessObservation::Observed(observed) => observed,
    };

    let (ownership, reason) = if let Some(recorded) = recorded {
        if recorded.pid != observed.pid {
            (
                RuntimeProcessOwnership::ReusedUnrelated,
                "recorded_pid_mismatch",
            )
        } else if observed.start_token.as_deref() != Some(recorded.start_token.as_str()) {
            if observed.start_token.is_some() {
                (
                    RuntimeProcessOwnership::ReusedUnrelated,
                    "process_start_token_mismatch",
                )
            } else {
                (
                    RuntimeProcessOwnership::AmbiguousLegacyBrowser,
                    "observed_start_token_unavailable",
                )
            }
        } else if !recorded_executable_matches(recorded, observed) {
            if observed.executable_path.is_some() {
                (
                    RuntimeProcessOwnership::ReusedUnrelated,
                    "recorded_executable_or_family_mismatch",
                )
            } else {
                (
                    RuntimeProcessOwnership::AmbiguousLegacyBrowser,
                    "observed_executable_unavailable",
                )
            }
        } else {
            (
                RuntimeProcessOwnership::MatchingBrowser,
                "exact_process_identity_match",
            )
        }
    } else if observed.executable_path.is_some() && observed.browser_family.is_none() {
        (
            RuntimeProcessOwnership::ReusedUnrelated,
            "legacy_process_is_not_browser",
        )
    } else if observed.browser_family.is_some()
        && legacy_profile_proof == LegacyProfileProof::ProfileConsistent
    {
        (
            RuntimeProcessOwnership::MatchingBrowser,
            "legacy_profile_consistent_browser",
        )
    } else {
        (
            RuntimeProcessOwnership::AmbiguousLegacyBrowser,
            "legacy_browser_profile_unproven",
        )
    };
    RuntimeProcessAssessment {
        ownership,
        observation,
        reason,
    }
}

pub fn observe_process(pid: u32) -> ProcessObservation {
    platform_process_identity(pid)
}

/// Observe the current Unix process group for launch identity binding.
/// Non-Unix platforms return no process-group evidence.
pub fn observe_process_group_id(pid: u32) -> Option<u32> {
    #[cfg(unix)]
    {
        let process_group_id = unsafe { libc::getpgid(pid as libc::pid_t) };
        (process_group_id > 0).then_some(process_group_id as u32)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        None
    }
}

#[cfg(test)]
pub fn process_exists(pid: u32) -> bool {
    matches!(observe_process(pid), ProcessObservation::Observed(_))
}

/// A termination capability bound to one verified process instance.
///
/// Linux and Windows retain a kernel handle so PID reuse after authorization
/// cannot redirect a signal. macOS lacks an equivalent stable public handle,
/// so attached runtimes cannot be signaled safely by PID and fail closed.
pub struct VerifiedProcessTermination {
    pid: u32,
    #[cfg(target_os = "macos")]
    recorded: RecordedProcessIdentity,
    #[cfg(target_os = "linux")]
    pidfd: std::os::fd::RawFd,
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl VerifiedProcessTermination {
    pub fn open(recorded: &RecordedProcessIdentity) -> Result<Option<Self>, String> {
        platform_open_verified_process(recorded)
    }

    pub fn is_running(&self) -> Result<bool, String> {
        platform_verified_process_is_running(self)
    }

    pub fn signal(&self, signal: VerifiedProcessSignal) -> Result<bool, String> {
        platform_signal_verified_process(self, signal)
    }
}

pub fn recorded_process_is_running(recorded: &RecordedProcessIdentity) -> Result<bool, String> {
    verify_recorded_process_observation(recorded, observe_process(recorded.pid))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifiedProcessSignal {
    Terminate,
    Kill,
}

fn verify_recorded_process_observation(
    recorded: &RecordedProcessIdentity,
    observation: ProcessObservation,
) -> Result<bool, String> {
    let assessment =
        assess_process_ownership(Some(recorded), observation, LegacyProfileProof::Unproven);
    match assessment.ownership {
        RuntimeProcessOwnership::MatchingBrowser => Ok(true),
        RuntimeProcessOwnership::Missing => Ok(false),
        RuntimeProcessOwnership::ReusedUnrelated => Err(format!(
            "Refusing to signal PID {} because it no longer matches the recorded process identity ({})",
            recorded.pid, assessment.reason
        )),
        RuntimeProcessOwnership::AmbiguousLegacyBrowser => Err(format!(
            "Refusing to signal PID {} because recorded process identity is ambiguous ({})",
            recorded.pid, assessment.reason
        )),
    }
}

#[cfg(target_os = "linux")]
fn platform_open_verified_process(
    recorded: &RecordedProcessIdentity,
) -> Result<Option<VerifiedProcessTermination>, String> {
    let pidfd = unsafe { libc::syscall(libc::SYS_pidfd_open, recorded.pid as libc::pid_t, 0) };
    if pidfd < 0 {
        let error = std::io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(None)
        } else {
            Err(format!(
                "Failed to bind runtime browser PID {} to pidfd: {}",
                recorded.pid, error
            ))
        };
    }
    let termination = VerifiedProcessTermination {
        pid: recorded.pid,
        pidfd: pidfd as std::os::fd::RawFd,
    };
    match verify_recorded_process_observation(recorded, observe_process(recorded.pid)) {
        Ok(true) => Ok(Some(termination)),
        Ok(false) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "linux")]
fn platform_verified_process_is_running(
    process: &VerifiedProcessTermination,
) -> Result<bool, String> {
    let mut descriptor = libc::pollfd {
        fd: process.pidfd,
        events: libc::POLLIN,
        revents: 0,
    };
    let result = unsafe { libc::poll(&mut descriptor, 1, 0) };
    if result < 0 {
        Err(format!(
            "Failed to query pidfd for runtime browser PID {}: {}",
            process.pid,
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(result == 0)
    }
}

#[cfg(target_os = "linux")]
fn platform_signal_verified_process(
    process: &VerifiedProcessTermination,
    signal: VerifiedProcessSignal,
) -> Result<bool, String> {
    let signal = match signal {
        VerifiedProcessSignal::Terminate => libc::SIGTERM,
        VerifiedProcessSignal::Kill => libc::SIGKILL,
    };
    let result = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            process.pidfd,
            signal,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    };
    if result == 0 {
        Ok(true)
    } else {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(false)
        } else {
            Err(format!(
                "Failed to signal verified runtime browser PID {}: {}",
                process.pid, error
            ))
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for VerifiedProcessTermination {
    fn drop(&mut self) {
        unsafe { libc::close(self.pidfd) };
    }
}

#[cfg(target_os = "macos")]
fn platform_open_verified_process(
    recorded: &RecordedProcessIdentity,
) -> Result<Option<VerifiedProcessTermination>, String> {
    verify_recorded_process_observation(recorded, observe_process(recorded.pid)).map(|matches| {
        matches.then(|| VerifiedProcessTermination {
            pid: recorded.pid,
            recorded: recorded.clone(),
        })
    })
}

#[cfg(target_os = "macos")]
fn platform_verified_process_is_running(
    process: &VerifiedProcessTermination,
) -> Result<bool, String> {
    verify_recorded_process_observation(&process.recorded, observe_process(process.pid))
}

#[cfg(target_os = "macos")]
fn platform_signal_verified_process(
    _process: &VerifiedProcessTermination,
    _signal: VerifiedProcessSignal,
) -> Result<bool, String> {
    attached_runtime_signal_unavailable()
}

#[cfg(any(target_os = "macos", test))]
fn attached_runtime_signal_unavailable() -> Result<bool, String> {
    Err(
        "Safe termination is unavailable for an attached macOS runtime without an owned process handle"
            .to_string(),
    )
}

#[cfg(windows)]
fn platform_open_verified_process(
    recorded: &RecordedProcessIdentity,
) -> Result<Option<VerifiedProcessTermination>, String> {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_INVALID_PARAMETER};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE,
            0,
            recorded.pid,
        )
    };
    if handle == 0 {
        let error = unsafe { GetLastError() };
        return if error == ERROR_INVALID_PARAMETER {
            Ok(None)
        } else {
            Err(format!(
                "Failed to bind runtime browser PID {} to process handle: os error {}",
                recorded.pid, error
            ))
        };
    }
    let termination = VerifiedProcessTermination {
        pid: recorded.pid,
        handle,
    };
    match verify_recorded_process_observation(
        recorded,
        windows_process_identity_from_handle(recorded.pid, handle),
    ) {
        Ok(true) => Ok(Some(termination)),
        Ok(false) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn platform_verified_process_is_running(
    process: &VerifiedProcessTermination,
) -> Result<bool, String> {
    use windows_sys::Win32::Foundation::{WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::WaitForSingleObject;
    match unsafe { WaitForSingleObject(process.handle, 0) } {
        WAIT_TIMEOUT => Ok(true),
        WAIT_OBJECT_0 => Ok(false),
        WAIT_FAILED => Err(format!(
            "Failed to query runtime browser process handle for PID {}: {}",
            process.pid,
            std::io::Error::last_os_error()
        )),
        status => Err(format!(
            "Unexpected process wait status {} for runtime browser PID {}",
            status, process.pid
        )),
    }
}

#[cfg(windows)]
fn platform_signal_verified_process(
    process: &VerifiedProcessTermination,
    _signal: VerifiedProcessSignal,
) -> Result<bool, String> {
    use windows_sys::Win32::System::Threading::TerminateProcess;
    if !platform_verified_process_is_running(process)? {
        return Ok(false);
    }
    if unsafe { TerminateProcess(process.handle, 1) } != 0 {
        Ok(true)
    } else {
        Err(format!(
            "Failed to terminate verified runtime browser PID {}: {}",
            process.pid,
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(windows)]
impl Drop for VerifiedProcessTermination {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_open_verified_process(
    _recorded: &RecordedProcessIdentity,
) -> Result<Option<VerifiedProcessTermination>, String> {
    Err("verified process termination is unsupported on this platform".to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_verified_process_is_running(
    _process: &VerifiedProcessTermination,
) -> Result<bool, String> {
    Err("verified process termination is unsupported on this platform".to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_signal_verified_process(
    _process: &VerifiedProcessTermination,
    _signal: VerifiedProcessSignal,
) -> Result<bool, String> {
    Err("verified process termination is unsupported on this platform".to_string())
}

fn recorded_executable_matches(
    recorded: &RecordedProcessIdentity,
    observed: &ObservedProcessIdentity,
) -> bool {
    let (Some(recorded_executable), Some(observed_executable)) = (
        recorded.executable_path.as_deref(),
        observed.executable_path.as_deref(),
    ) else {
        return false;
    };
    let family_matches = match recorded.browser_family.as_deref() {
        Some(recorded_family) => observed.browser_family.as_deref() == Some(recorded_family),
        None => true,
    };
    family_matches
        && executable_paths_match(
            Path::new(recorded_executable),
            Path::new(observed_executable),
        )
}

fn executable_paths_match(expected: &Path, observed: &Path) -> bool {
    if expected == observed {
        return true;
    }
    if let (Ok(expected), Ok(observed)) = (
        std::fs::canonicalize(expected),
        std::fs::canonicalize(observed),
    ) {
        return expected == observed;
    }
    #[cfg(windows)]
    {
        return expected
            .to_string_lossy()
            .eq_ignore_ascii_case(&observed.to_string_lossy());
    }
    #[cfg(not(windows))]
    false
}

fn browser_launch_paths_match(
    requested: &Path,
    observed: &Path,
    expected_browser_family: Option<&str>,
) -> bool {
    if executable_paths_match(requested, observed) {
        return true;
    }
    let (Ok(requested), Ok(observed)) = (
        std::fs::canonicalize(requested),
        std::fs::canonicalize(observed),
    ) else {
        return false;
    };
    let requested_family = browser_family_for_path(Some(&requested));
    let observed_family = browser_family_for_path(Some(&observed));
    if requested_family.is_none()
        || requested_family != observed_family
        || expected_browser_family
            .is_some_and(|expected| observed_family.as_deref() != Some(expected))
        || requested.parent() != observed.parent()
    {
        return false;
    }
    std::fs::read(&requested)
        .ok()
        .is_some_and(|contents| contents.starts_with(b"#!"))
}

#[cfg(target_os = "linux")]
fn platform_process_identity(pid: u32) -> ProcessObservation {
    let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ProcessObservation::Missing;
        }
        Err(error) => {
            return ProcessObservation::Failed {
                reason: format!("linux_proc_stat_failed: {error}"),
            };
        }
    };
    if linux_process_state(&stat) == Some("Z") {
        return ProcessObservation::Missing;
    }
    let start_ticks = linux_start_ticks(&stat);
    let boot_id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let start_token = match (boot_id, start_ticks) {
        (Some(boot_id), Some(start_ticks)) => Some(format!("linux:{boot_id}:{start_ticks}")),
        _ => None,
    };
    let executable_path = std::fs::read_link(format!("/proc/{pid}/exe")).ok();
    let command_line = std::fs::read(format!("/proc/{pid}/cmdline"))
        .ok()
        .map(|bytes| {
            bytes
                .split(|byte| *byte == 0)
                .filter(|value| !value.is_empty())
                .map(|value| String::from_utf8_lossy(value).into_owned())
                .collect::<Vec<_>>()
        })
        .filter(|arguments| !arguments.is_empty());
    ProcessObservation::Observed(ObservedProcessIdentity {
        pid,
        start_token,
        browser_family: browser_family_for_path(executable_path.as_deref()),
        executable_path: executable_path.map(|path| path.to_string_lossy().into_owned()),
        command_line,
    })
}

#[cfg(target_os = "linux")]
fn linux_start_ticks(stat: &str) -> Option<&str> {
    linux_stat_fields(stat)?.nth(19)
}

#[cfg(target_os = "linux")]
fn linux_process_state(stat: &str) -> Option<&str> {
    linux_stat_fields(stat)?.next()
}

#[cfg(target_os = "linux")]
fn linux_stat_fields(stat: &str) -> Option<impl Iterator<Item = &str>> {
    Some(stat.rsplit_once(')')?.1.split_whitespace())
}

#[cfg(target_os = "macos")]
fn platform_process_identity(pid: u32) -> ProcessObservation {
    use std::mem::{size_of, MaybeUninit};

    let mut info = MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    let info_size = size_of::<libc::proc_bsdinfo>() as libc::c_int;
    let read = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            info_size,
        )
    };
    if read != info_size {
        let error = std::io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ESRCH) {
            ProcessObservation::Missing
        } else {
            ProcessObservation::Failed {
                reason: format!("macos_proc_pidinfo_failed: {error}"),
            }
        };
    }
    let start_token = {
        let info = unsafe { info.assume_init() };
        Some(format!(
            "macos:{}:{}",
            info.pbi_start_tvsec, info.pbi_start_tvusec
        ))
    };

    let mut path = vec![0u8; 4096];
    let path_len = unsafe {
        libc::proc_pidpath(
            pid as libc::c_int,
            path.as_mut_ptr().cast(),
            path.len() as u32,
        )
    };
    let executable_path = if path_len > 0 {
        path.truncate(path_len as usize);
        Some(PathBuf::from(String::from_utf8_lossy(&path).into_owned()))
    } else {
        None
    };
    let command_line = match macos_process_command_line(pid) {
        Ok(arguments) => Some(arguments),
        Err(reason) => return ProcessObservation::Failed { reason },
    };
    ProcessObservation::Observed(ObservedProcessIdentity {
        pid,
        start_token,
        browser_family: browser_family_for_path(executable_path.as_deref()),
        executable_path: executable_path.map(|path| path.to_string_lossy().into_owned()),
        command_line,
    })
}

#[cfg(target_os = "macos")]
fn macos_process_command_line(pid: u32) -> Result<Vec<String>, String> {
    let arg_max = unsafe { libc::sysconf(libc::_SC_ARG_MAX) };
    if arg_max <= 0 {
        return Err(format!(
            "macos_process_command_line_arg_max_failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let mut buffer = vec![0u8; arg_max as usize];
    let mut buffer_len = buffer.len();
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as libc::c_int];
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            buffer.as_mut_ptr().cast(),
            &mut buffer_len,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return Err(format!(
            "macos_process_command_line_sysctl_failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    buffer.truncate(buffer_len);
    parse_macos_kern_procargs2(&buffer)
        .ok_or_else(|| "macos_process_command_line_parse_failed".to_string())
}

#[cfg(windows)]
fn platform_process_identity(pid: u32) -> ProcessObservation {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_INVALID_PARAMETER};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 {
        let error = unsafe { GetLastError() };
        return if error == ERROR_INVALID_PARAMETER {
            ProcessObservation::Missing
        } else {
            ProcessObservation::Failed {
                reason: format!("windows_open_process_failed: os error {error}"),
            }
        };
    }
    let observation = windows_process_identity_from_handle(pid, handle);
    unsafe { CloseHandle(handle) };
    observation
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_process_identity(_pid: u32) -> ProcessObservation {
    ProcessObservation::Failed {
        reason: "process observation is unsupported on this platform".to_string(),
    }
}

#[cfg(windows)]
fn windows_process_identity_from_handle(
    pid: u32,
    handle: windows_sys::Win32::Foundation::HANDLE,
) -> ProcessObservation {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetProcessTimes, QueryFullProcessImageNameW};

    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = creation;
    let mut kernel = creation;
    let mut user = creation;
    if unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return ProcessObservation::Failed {
            reason: format!(
                "windows_get_process_times_failed: {}",
                std::io::Error::last_os_error()
            ),
        };
    }
    let start_token = Some(format!(
        "windows:{}",
        ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64
    ));
    let mut buffer = vec![0u16; 32768];
    let mut len = buffer.len() as u32;
    let executable_path =
        if unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut len) } != 0 {
            buffer.truncate(len as usize);
            Some(PathBuf::from(String::from_utf16_lossy(&buffer)))
        } else {
            None
        };
    let command_line = match windows_process_command_line(handle) {
        Ok(arguments) => Some(arguments),
        Err(reason) => return ProcessObservation::Failed { reason },
    };
    ProcessObservation::Observed(ObservedProcessIdentity {
        pid,
        start_token,
        browser_family: browser_family_for_path(executable_path.as_deref()),
        executable_path: executable_path.map(|path| path.to_string_lossy().into_owned()),
        command_line,
    })
}

#[cfg(windows)]
fn windows_process_command_line(
    handle: windows_sys::Win32::Foundation::HANDLE,
) -> Result<Vec<String>, String> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use windows_sys::Wdk::System::Threading::{
        NtQueryInformationProcess, ProcessCommandLineInformation,
    };
    use windows_sys::Win32::Foundation::UNICODE_STRING;

    let mut required_len = 0u32;
    unsafe {
        NtQueryInformationProcess(
            handle,
            ProcessCommandLineInformation,
            std::ptr::null_mut(),
            0,
            &mut required_len,
        )
    };
    if required_len < size_of::<UNICODE_STRING>() as u32 {
        return Err("windows_process_command_line_size_query_failed".to_string());
    }

    let word_len = (required_len as usize).div_ceil(size_of::<usize>());
    let mut storage = vec![0usize; word_len];
    let status = unsafe {
        NtQueryInformationProcess(
            handle,
            ProcessCommandLineInformation,
            storage.as_mut_ptr().cast::<c_void>(),
            required_len,
            &mut required_len,
        )
    };
    if status < 0 {
        return Err(format!(
            "windows_process_command_line_query_failed: ntstatus {status:#x}"
        ));
    }

    let info = unsafe { &*storage.as_ptr().cast::<UNICODE_STRING>() };
    let buffer_start = storage.as_ptr() as usize;
    let buffer_end = buffer_start + storage.len() * size_of::<usize>();
    let command_start = info.Buffer as usize;
    let command_end = command_start.saturating_add(info.Length as usize);
    if info.Buffer.is_null()
        || info.Length == 0
        || info.Length % 2 != 0
        || command_start < buffer_start
        || command_end > buffer_end
    {
        return Err("windows_process_command_line_invalid_result".to_string());
    }
    let utf16 = unsafe { std::slice::from_raw_parts(info.Buffer, info.Length as usize / 2) };
    let command_line = String::from_utf16(utf16)
        .map_err(|error| format!("windows_process_command_line_utf16_failed: {error}"))?;
    let arguments = parse_windows_command_line(&command_line);
    if arguments.is_empty() {
        Err("windows_process_command_line_parse_failed".to_string())
    } else {
        Ok(arguments)
    }
}

#[cfg(any(windows, test))]
fn parse_windows_command_line(command_line: &str) -> Vec<String> {
    let characters = command_line.chars().collect::<Vec<_>>();
    let mut arguments = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        while index < characters.len() && characters[index].is_ascii_whitespace() {
            index += 1;
        }
        if index == characters.len() {
            break;
        }

        let mut argument = String::new();
        let mut quoted = false;
        while index < characters.len() && (quoted || !characters[index].is_ascii_whitespace()) {
            if characters[index] == '\\' {
                let slash_start = index;
                while index < characters.len() && characters[index] == '\\' {
                    index += 1;
                }
                let slash_count = index - slash_start;
                if index < characters.len() && characters[index] == '"' {
                    argument.extend(std::iter::repeat_n('\\', slash_count / 2));
                    if slash_count % 2 == 0 {
                        quoted = !quoted;
                    } else {
                        argument.push('"');
                    }
                    index += 1;
                } else {
                    argument.extend(std::iter::repeat_n('\\', slash_count));
                }
            } else if characters[index] == '"' {
                quoted = !quoted;
                index += 1;
            } else {
                argument.push(characters[index]);
                index += 1;
            }
        }
        arguments.push(argument);
    }
    arguments
}

#[cfg(any(target_os = "macos", test))]
fn parse_macos_kern_procargs2(buffer: &[u8]) -> Option<Vec<String>> {
    let argc_bytes = buffer.get(..std::mem::size_of::<libc::c_int>())?;
    let argc = libc::c_int::from_ne_bytes(argc_bytes.try_into().ok()?);
    if argc <= 0 {
        return None;
    }

    let mut index = std::mem::size_of::<libc::c_int>();
    index += buffer.get(index..)?.iter().position(|byte| *byte == 0)? + 1;
    while buffer.get(index) == Some(&0) {
        index += 1;
    }

    let mut arguments = Vec::with_capacity(argc as usize);
    for _ in 0..argc {
        let tail = buffer.get(index..)?;
        let end = tail.iter().position(|byte| *byte == 0)?;
        arguments.push(String::from_utf8_lossy(&tail[..end]).into_owned());
        index += end + 1;
    }
    Some(arguments)
}

pub(crate) fn browser_family_for_path(path: Option<&Path>) -> Option<String> {
    let normalized = path?.to_string_lossy().to_ascii_lowercase();
    for (needle, family) in [
        ("chromium", "chromium"),
        ("google chrome", "chrome"),
        ("google-chrome", "chrome"),
        ("chrome", "chrome"),
        ("brave", "brave"),
        ("msedge", "edge"),
        ("microsoft edge", "edge"),
    ] {
        if normalized.contains(needle) {
            return Some(family.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_epoch_status_distinguishes_current_prior_missing_and_unavailable() {
        assert_eq!(
            boot_epoch_status(Some("boot:a"), Some("boot:a")),
            BootEpochStatus::Current
        );
        assert_eq!(
            boot_epoch_status(Some("boot:a"), Some("boot:b")),
            BootEpochStatus::Prior
        );
        assert_eq!(
            boot_epoch_status(None, Some("boot:b")),
            BootEpochStatus::Missing
        );
        assert_eq!(
            boot_epoch_status(Some("boot:a"), None),
            BootEpochStatus::Unavailable
        );
    }

    fn observed(start_token: Option<&str>, executable: Option<&str>) -> ObservedProcessIdentity {
        ObservedProcessIdentity {
            pid: 7,
            start_token: start_token.map(str::to_string),
            executable_path: executable.map(str::to_string),
            browser_family: browser_family_for_path(executable.map(Path::new)),
            command_line: None,
        }
    }

    fn ownership(
        recorded: Option<&RecordedProcessIdentity>,
        observed: ObservedProcessIdentity,
        proof: LegacyProfileProof,
    ) -> RuntimeProcessOwnership {
        assess_process_ownership(recorded, ProcessObservation::Observed(observed), proof).ownership
    }

    #[test]
    fn exact_start_token_is_the_recorded_process() {
        let recorded = RecordedProcessIdentity {
            pid: 7,
            start_token: "linux:boot:11".to_string(),
            executable_path: Some("/opt/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        assert_eq!(
            ownership(
                Some(&recorded),
                observed(Some("linux:boot:11"), Some("/opt/chrome")),
                LegacyProfileProof::Unproven,
            ),
            RuntimeProcessOwnership::MatchingBrowser
        );
    }

    #[test]
    fn exact_non_browser_process_identity_matches_without_a_browser_family() {
        let recorded = RecordedProcessIdentity {
            pid: 7,
            start_token: "linux:boot:11".to_string(),
            executable_path: Some("/opt/agent-browser".to_string()),
            browser_family: None,
        };
        let observed = ObservedProcessIdentity {
            pid: 7,
            start_token: Some("linux:boot:11".to_string()),
            executable_path: Some("/opt/agent-browser".to_string()),
            browser_family: None,
            command_line: None,
        };

        assert_eq!(
            ownership(Some(&recorded), observed, LegacyProfileProof::Unproven,),
            RuntimeProcessOwnership::MatchingBrowser
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn exact_process_liveness_observation_does_not_require_a_pidfd() {
        let executable = std::env::current_exe().unwrap();
        let recorded = capture_process_identity(std::process::id(), Some(&executable), None)
            .expect("the test process identity should be observable");
        assert!(recorded_process_is_running(&recorded).unwrap());

        let missing = RecordedProcessIdentity {
            pid: u32::MAX,
            start_token: "linux:missing:0".to_string(),
            executable_path: recorded.executable_path,
            browser_family: recorded.browser_family,
        };
        assert!(!recorded_process_is_running(&missing).unwrap());
    }

    #[test]
    fn changed_start_token_is_pid_reuse() {
        let recorded = RecordedProcessIdentity {
            pid: 7,
            start_token: "linux:boot:11".to_string(),
            executable_path: Some("/opt/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        assert_eq!(
            ownership(
                Some(&recorded),
                observed(Some("linux:boot:12"), Some("/opt/chrome")),
                LegacyProfileProof::ProfileConsistent,
            ),
            RuntimeProcessOwnership::ReusedUnrelated
        );
    }

    #[test]
    fn equal_start_token_with_executable_mismatch_is_pid_reuse() {
        let recorded = RecordedProcessIdentity {
            pid: 7,
            start_token: "linux:boot:11".to_string(),
            executable_path: Some("/opt/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        assert_eq!(
            ownership(
                Some(&recorded),
                observed(Some("linux:boot:11"), Some("/opt/chromium")),
                LegacyProfileProof::Unproven,
            ),
            RuntimeProcessOwnership::ReusedUnrelated
        );
    }

    #[test]
    fn equal_start_token_with_browser_family_mismatch_is_pid_reuse() {
        let recorded = RecordedProcessIdentity {
            pid: 7,
            start_token: "linux:boot:11".to_string(),
            executable_path: Some("/opt/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        assert_eq!(
            ownership(
                Some(&recorded),
                observed(Some("linux:boot:11"), Some("/opt/brave")),
                LegacyProfileProof::Unproven,
            ),
            RuntimeProcessOwnership::ReusedUnrelated
        );
    }

    #[test]
    fn browser_wrapper_transition_requires_same_install_root_and_family() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-launch-wrapper-{}",
            uuid::Uuid::new_v4()
        ));
        let install = root.join("google-chrome");
        let unrelated = root.join("unrelated");
        std::fs::create_dir_all(&install).unwrap();
        std::fs::create_dir_all(&unrelated).unwrap();
        let wrapper = install.join("google-chrome");
        let browser = install.join("chrome");
        let unrelated_browser = unrelated.join("chrome");
        std::fs::write(&wrapper, "#!/bin/sh\nexec \"$(dirname \"$0\")/chrome\"\n").unwrap();
        std::fs::write(&browser, "browser").unwrap();
        std::fs::write(&unrelated_browser, "browser").unwrap();

        assert!(browser_launch_paths_match(
            &wrapper,
            &browser,
            Some("chrome")
        ));
        assert!(!browser_launch_paths_match(
            &wrapper,
            &unrelated_browser,
            Some("chrome")
        ));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn equal_start_token_without_observed_executable_is_ambiguous() {
        let recorded = RecordedProcessIdentity {
            pid: 7,
            start_token: "linux:boot:11".to_string(),
            executable_path: Some("/opt/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        assert_eq!(
            ownership(
                Some(&recorded),
                observed(Some("linux:boot:11"), None),
                LegacyProfileProof::Unproven,
            ),
            RuntimeProcessOwnership::AmbiguousLegacyBrowser
        );
    }

    #[test]
    fn observation_failure_is_ambiguous_and_never_missing() {
        let assessment = assess_process_ownership(
            None,
            ProcessObservation::Failed {
                reason: "permission denied".to_string(),
            },
            LegacyProfileProof::Unproven,
        );
        assert_eq!(
            assessment.ownership,
            RuntimeProcessOwnership::AmbiguousLegacyBrowser
        );
        assert_eq!(assessment.reason, "process_observation_failed");
    }

    #[test]
    fn windows_command_line_parser_preserves_profile_and_ephemeral_port_arguments() {
        let arguments = parse_windows_command_line(
            r#""C:\Program Files\Google\Chrome\Application\chrome.exe" --user-data-dir="C:\Users\Agent Browser\profile" --remote-debugging-port=0 "quoted \"value\"""#,
        );

        assert_eq!(
            arguments,
            vec![
                r#"C:\Program Files\Google\Chrome\Application\chrome.exe"#,
                r#"--user-data-dir=C:\Users\Agent Browser\profile"#,
                "--remote-debugging-port=0",
                r#"quoted "value""#,
            ]
        );
    }

    #[test]
    fn macos_kern_procargs_parser_returns_exact_declared_arguments() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&3i32.to_ne_bytes());
        buffer
            .extend_from_slice(b"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome\0\0");
        buffer.extend_from_slice(b"Google Chrome\0");
        buffer.extend_from_slice(b"--user-data-dir=/tmp/Agent Browser/profile\0");
        buffer.extend_from_slice(b"--remote-debugging-port=0\0");

        assert_eq!(
            parse_macos_kern_procargs2(&buffer),
            Some(vec![
                "Google Chrome".to_string(),
                "--user-data-dir=/tmp/Agent Browser/profile".to_string(),
                "--remote-debugging-port=0".to_string(),
            ])
        );
    }

    #[test]
    fn macos_attached_runtime_signal_policy_fails_closed_without_effect() {
        let mut signal_effects = 0;
        if attached_runtime_signal_unavailable() == Ok(true) {
            signal_effects += 1;
        }

        assert_eq!(signal_effects, 0);
        assert!(attached_runtime_signal_unavailable()
            .unwrap_err()
            .contains("Safe termination is unavailable"));
    }

    #[test]
    fn replacement_at_final_signal_boundary_is_refused_before_effect() {
        let recorded = RecordedProcessIdentity {
            pid: 7,
            start_token: "linux:boot:11".to_string(),
            executable_path: Some("/opt/chrome".to_string()),
            browser_family: Some("chrome".to_string()),
        };
        let replacement =
            ProcessObservation::Observed(observed(Some("linux:boot:12"), Some("/opt/chrome")));
        let mut signal_effects = 0;

        if verify_recorded_process_observation(&recorded, replacement) == Ok(true) {
            signal_effects += 1;
        }

        assert_eq!(signal_effects, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn pidfd_signal_is_bound_to_the_captured_test_process() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-pidfd-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let executable = root.join("fixture-chrome");
        std::fs::copy("/bin/sleep", &executable).unwrap();
        let mut child = std::process::Command::new(&executable)
            .arg("30")
            .spawn()
            .unwrap();
        let recorded = capture_process_identity(child.id(), Some(&executable), Some("chrome"))
            .expect("owned browser-looking fixture must be captured");
        let process = VerifiedProcessTermination::open(&recorded)
            .unwrap()
            .expect("owned browser-looking fixture must bind to a pidfd");

        assert!(process.is_running().unwrap());
        assert!(process.signal(VerifiedProcessSignal::Terminate).unwrap());
        for _ in 0..50 {
            if !process.is_running().unwrap() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!process.is_running().unwrap());

        let _ = child.wait();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn consumer_policy_never_adopts_ambiguous_or_deletes_unproven_evidence() {
        for (ownership, adopts, cleans, preserves) in [
            (RuntimeProcessOwnership::MatchingBrowser, true, false, true),
            (RuntimeProcessOwnership::Missing, false, true, false),
            (RuntimeProcessOwnership::ReusedUnrelated, false, true, false),
            (
                RuntimeProcessOwnership::AmbiguousLegacyBrowser,
                false,
                false,
                true,
            ),
        ] {
            let assessment = RuntimeProcessAssessment {
                ownership,
                observation: ProcessObservation::Missing,
                reason: "fixture",
            };
            assert_eq!(assessment.authorizes_adoption(), adopts);
            assert_eq!(assessment.authorizes_cleanup(), cleans);
            assert_eq!(assessment.preserves_evidence(), preserves);
        }
    }

    #[test]
    fn capture_cannot_relabel_current_nonbrowser_from_expected_arguments() {
        assert!(capture_process_identity(std::process::id(), None, Some("chrome")).is_none());
    }

    #[test]
    fn legacy_non_browser_is_pid_reuse() {
        assert_eq!(
            ownership(
                None,
                observed(Some("linux:boot:11"), Some("/usr/bin/codex")),
                LegacyProfileProof::Unproven,
            ),
            RuntimeProcessOwnership::ReusedUnrelated
        );
    }

    #[test]
    fn legacy_browser_without_endpoint_is_ambiguous() {
        assert_eq!(
            ownership(
                None,
                observed(Some("linux:boot:11"), Some("/opt/chrome")),
                LegacyProfileProof::Unproven,
            ),
            RuntimeProcessOwnership::AmbiguousLegacyBrowser
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_stat_parser_handles_spaces_and_parentheses_in_process_name() {
        let mut fields = vec!["S".to_string()];
        fields.extend((4..=21).map(|field| field.to_string()));
        fields.push("4242".to_string());
        let stat = format!("7 (chrome helper) weird) {}", fields.join(" "));
        assert_eq!(linux_process_state(&stat), Some("S"));
        assert_eq!(linux_start_ticks(&stat), Some("4242"));
    }
}
