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
    let browser_family = observed.browser_family?;
    if expected_executable
        .is_some_and(|expected| !executable_paths_match(expected, Path::new(&executable_path)))
        || expected_browser_family.is_some_and(|expected| expected != browser_family)
    {
        return None;
    }
    Some(RecordedProcessIdentity {
        pid,
        start_token,
        executable_path: Some(executable_path),
        browser_family: Some(browser_family),
    })
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

#[cfg(test)]
pub fn process_exists(pid: u32) -> bool {
    matches!(observe_process(pid), ProcessObservation::Observed(_))
}

/// A termination capability bound to one verified process instance.
///
/// Linux and Windows retain a kernel handle so PID reuse after authorization
/// cannot redirect a signal. macOS lacks an equivalent stable public handle,
/// so every signal is preceded by a conservative identity recheck; the final
/// metadata-check-to-signal interval is an unavoidable kernel boundary there.
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
            "Refusing to signal PID {} because it no longer matches the recorded runtime browser identity ({})",
            recorded.pid, assessment.reason
        )),
        RuntimeProcessOwnership::AmbiguousLegacyBrowser => Err(format!(
            "Refusing to signal PID {} because runtime browser identity is ambiguous ({})",
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
    process: &VerifiedProcessTermination,
    signal: VerifiedProcessSignal,
) -> Result<bool, String> {
    if !verify_recorded_process_observation(&process.recorded, observe_process(process.pid))? {
        return Ok(false);
    }
    let signal = match signal {
        VerifiedProcessSignal::Terminate => libc::SIGTERM,
        VerifiedProcessSignal::Kill => libc::SIGKILL,
    };
    let result = unsafe { libc::kill(process.pid as libc::pid_t, signal) };
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

#[cfg(windows)]
fn platform_open_verified_process(
    recorded: &RecordedProcessIdentity,
) -> Result<Option<VerifiedProcessTermination>, String> {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_INVALID_PARAMETER};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, SYNCHRONIZE,
    };
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | SYNCHRONIZE,
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
    let (Some(recorded_family), Some(observed_family)) = (
        recorded.browser_family.as_deref(),
        observed.browser_family.as_deref(),
    ) else {
        return false;
    };
    recorded_family == observed_family
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
    ProcessObservation::Observed(ObservedProcessIdentity {
        pid,
        start_token,
        browser_family: browser_family_for_path(executable_path.as_deref()),
        executable_path: executable_path.map(|path| path.to_string_lossy().into_owned()),
        command_line: None,
    })
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
    ProcessObservation::Observed(ObservedProcessIdentity {
        pid,
        start_token,
        browser_family: browser_family_for_path(executable_path.as_deref()),
        executable_path: executable_path.map(|path| path.to_string_lossy().into_owned()),
        command_line: None,
    })
}

fn browser_family_for_path(path: Option<&Path>) -> Option<String> {
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

    #[test]
    fn browser_ownership_consumers_do_not_restore_pid_only_liveness() {
        for (name, source, required_interface) in [
            (
                "runtime_profile",
                include_str!("runtime_profile.rs"),
                "evaluate_runtime_process(",
            ),
            (
                "chrome_profile_lock",
                include_str!("native/cdp/chrome.rs"),
                "profile_lock_process_assessment(",
            ),
            (
                "service_config",
                include_str!("native/service_config.rs"),
                "runtime_process_assessment(",
            ),
            (
                "service_health",
                include_str!("native/service_health.rs"),
                "runtime_process_assessment(",
            ),
            (
                "remote_view",
                include_str!("native/remote_view.rs"),
                "runtime_process_assessment(",
            ),
            (
                "runtime_navigation",
                include_str!("native/action_runtime/runtime/navigation.rs"),
                "runtime_process_assessment(",
            ),
            (
                "runtime_recovery",
                include_str!("native/action_runtime/runtime/recovery.rs"),
                "runtime_process_assessment(",
            ),
            (
                "runtime_launch",
                include_str!("native/action_runtime/runtime/launch.rs"),
                "VerifiedProcessTermination::open(",
            ),
        ] {
            assert!(
                source.contains(required_interface),
                "{name} must use its declared shared process identity interface"
            );
            assert!(
                !source.contains("pid_is_running"),
                "{name} must use the shared process identity decision"
            );
            assert!(
                !source.contains("runtime_process_ownership"),
                "{name} must not restore the raw ownership facade"
            );
            assert!(
                !source.contains("profile_endpoint_reachable"),
                "{name} must not pass divergent endpoint evidence"
            );
            assert!(
                !source.contains("libc::kill(pid as i32, 0)")
                    && !source.contains("libc::kill(pid, 0)"),
                "{name} must not observe ownership through signal zero"
            );
            assert!(
                !source.contains("Command::new(\"taskkill\")"),
                "{name} must not terminate an unbound Windows PID"
            );
        }
        let identity_implementation = include_str!("process_identity.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(!identity_implementation.contains("libc::kill(pid, 0)"));
        assert!(!identity_implementation.contains("libc::kill(pid as i32, 0)"));
    }
}
