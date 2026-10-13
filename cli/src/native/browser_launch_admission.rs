//! Provider-neutral host resource admission for browser process launch.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;

const MINIMUM_AVAILABLE_MEMORY_BYTES: u64 = 1024 * 1024 * 1024;
const MINIMUM_AVAILABLE_DISK_BYTES: u64 = 1024 * 1024 * 1024;
const RESERVED_PROCESS_IDENTIFIERS: u64 = 256;
const DEFAULT_MAXIMUM_BROWSER_PROCESSES: u64 = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserLaunchAdmission {
    pub(crate) schema_version: &'static str,
    pub(crate) state: &'static str,
    pub(crate) available_memory_bytes: Option<u64>,
    pub(crate) minimum_available_memory_bytes: u64,
    pub(crate) memory_reserve_source: &'static str,
    pub(crate) available_disk_bytes: Option<u64>,
    pub(crate) minimum_available_disk_bytes: u64,
    pub(crate) host_process_count: Option<u64>,
    pub(crate) host_process_limit: Option<u64>,
    pub(crate) browser_process_count: Option<u64>,
    pub(crate) maximum_browser_processes: u64,
    pub(crate) reasons: Vec<&'static str>,
}

impl BrowserLaunchAdmission {
    pub(crate) fn require_admitted(&self) -> Result<(), String> {
        if self.state == "admitted" {
            Ok(())
        } else {
            Err(format!(
                "browser_launch_resource_pressure:{}",
                self.reasons.join(",")
            ))
        }
    }
}

/// Observe current host capacity without mutating browser or provider state.
pub(crate) fn observe_browser_launch_admission(
    profile_path: &Path,
    maximum_browser_processes: Option<u32>,
) -> BrowserLaunchAdmission {
    let system = sysinfo::System::new_all();
    let available_memory_bytes = Some(system.available_memory());
    // An explicit byte budget overrides the adaptive reserve. Invalid settings
    // remain visible as configuration pressure rather than silently weakening it.
    let configured_reserve = std::env::var_os("AGENT_BROWSER_HOST_MEMORY_RESERVE_BYTES");
    let parsed_reserve = configured_reserve
        .as_ref()
        .and_then(|value| value.to_str())
        .and_then(|value| value.parse::<u64>().ok());
    let invalid_reserve = configured_reserve.is_some() && parsed_reserve.is_none();
    let minimum_available_memory_bytes = parsed_reserve
        .unwrap_or_else(|| MINIMUM_AVAILABLE_MEMORY_BYTES.max(system.total_memory() / 20));
    let available_disk_bytes = nearest_existing_ancestor(profile_path)
        .as_deref()
        .and_then(available_disk_bytes);
    let host_process_count = u64::try_from(system.processes().len()).ok();
    let host_process_limit = process_limit();
    let browser_process_count = u64::try_from(
        system
            .processes()
            .values()
            .filter(|process| is_browser_root_process(process))
            .count(),
    )
    .ok();
    let maximum_browser_processes = maximum_browser_processes
        .map(u64::from)
        .unwrap_or(DEFAULT_MAXIMUM_BROWSER_PROCESSES);
    let mut reasons = Vec::new();
    if invalid_reserve {
        reasons.push("host_memory_reserve_invalid");
    }
    match available_memory_bytes {
        Some(bytes) if bytes < minimum_available_memory_bytes => {
            reasons.push("available_memory_below_floor")
        }
        None => reasons.push("available_memory_unavailable"),
        _ => {}
    }
    match available_disk_bytes {
        Some(bytes) if bytes < MINIMUM_AVAILABLE_DISK_BYTES => {
            reasons.push("available_disk_below_floor")
        }
        None => reasons.push("available_disk_unavailable"),
        _ => {}
    }
    match (host_process_count, host_process_limit) {
        (Some(count), Some(limit))
            if count.saturating_add(RESERVED_PROCESS_IDENTIFIERS) >= limit =>
        {
            reasons.push("host_process_capacity_exhausted")
        }
        (Some(_), _) => {}
        (None, _) => reasons.push("host_process_capacity_unavailable"),
    }
    match browser_process_count {
        Some(count) if count >= maximum_browser_processes => {
            reasons.push("browser_process_capacity_exhausted")
        }
        None => reasons.push("browser_process_count_unavailable"),
        _ => {}
    }
    BrowserLaunchAdmission {
        schema_version: "agent-browser.browser-launch-admission.v1",
        state: if reasons.is_empty() {
            "admitted"
        } else {
            "rejected"
        },
        available_memory_bytes,
        minimum_available_memory_bytes,
        memory_reserve_source: if invalid_reserve {
            "invalid_environment"
        } else if configured_reserve.is_some() {
            "environment"
        } else {
            "adaptive_default"
        },
        available_disk_bytes,
        minimum_available_disk_bytes: MINIMUM_AVAILABLE_DISK_BYTES,
        host_process_count,
        host_process_limit,
        browser_process_count,
        maximum_browser_processes,
        reasons,
    }
}

fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|candidate| candidate.exists())
        .map(Path::to_path_buf)
}

/// New process demand briefly waits for recoverable memory pressure. Observation
/// and existing-browser reuse remain immediate and never terminate current work.
pub(crate) fn wait_for_browser_launch_admission(
    profile: &Path,
    maximum: Option<u32>,
) -> Result<(), String> {
    let budget = match std::env::var("AGENT_BROWSER_LAUNCH_PRESSURE_WAIT_MS") {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| "browser_launch_wait_budget_invalid")?,
        Err(std::env::VarError::NotPresent) => 5000,
        Err(_) => return Err("browser_launch_wait_budget_invalid".into()),
    };
    if budget > 60_000 {
        return Err("browser_launch_wait_budget_invalid".into());
    }
    let started = Instant::now();
    wait_for_admission_with(
        || observe_browser_launch_admission(profile, maximum),
        || started.elapsed(),
        std::thread::sleep,
        Duration::from_millis(budget),
    )
}

fn wait_for_admission_with(
    mut observe: impl FnMut() -> BrowserLaunchAdmission,
    mut elapsed: impl FnMut() -> Duration,
    mut pause: impl FnMut(Duration),
    budget: Duration,
) -> Result<(), String> {
    loop {
        let observation = observe();
        if observation.state == "admitted" {
            return Ok(());
        }
        if observation
            .reasons
            .iter()
            .any(|reason| *reason != "available_memory_below_floor")
        {
            return observation.require_admitted();
        }
        let spent = elapsed();
        if spent >= budget {
            return observation.require_admitted();
        }
        pause(Duration::from_millis(250).min(budget.saturating_sub(spent)));
    }
}

#[cfg(unix)]
fn available_disk_bytes(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stats = MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return None;
    }
    let stats = unsafe { stats.assume_init() };
    statvfs_available_bytes(stats.f_bavail, stats.f_frsize)
}

#[cfg(unix)]
fn statvfs_available_bytes<Blocks, Bytes>(
    available_blocks: Blocks,
    fragment_bytes: Bytes,
) -> Option<u64>
where
    Blocks: TryInto<u64>,
    Bytes: TryInto<u64>,
{
    available_blocks
        .try_into()
        .ok()?
        .checked_mul(fragment_bytes.try_into().ok()?)
}

#[cfg(windows)]
fn available_disk_bytes(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;

    let path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut available = 0_u64;
    let result = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (result != 0).then_some(available)
}

fn is_browser_root_process(process: &sysinfo::Process) -> bool {
    let name = process.name().to_string_lossy().to_ascii_lowercase();
    let command = process
        .cmd()
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    (name.contains("chrome") || name.contains("chromium"))
        && command.contains("--agent-browser-reservation-id=")
        && !command.contains("--type=")
}

#[cfg(target_os = "linux")]
fn process_limit() -> Option<u64> {
    std::fs::read_to_string("/proc/sys/kernel/pid_max")
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(not(target_os = "linux"))]
fn process_limit() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_pressure_wait_rechecks_capacity_and_respects_deadline() {
        use std::cell::Cell;
        use std::time::Duration;
        let mut clear = observe_browser_launch_admission(Path::new("."), None);
        clear.state = "admitted";
        clear.reasons.clear();
        let mut pressure = clear.clone();
        pressure.state = "rejected";
        pressure.reasons = vec!["available_memory_below_floor"];
        let elapsed = Cell::new(Duration::ZERO);
        let observations = Cell::new(0);
        wait_for_admission_with(
            || {
                observations.set(observations.get() + 1);
                if observations.get() < 3 {
                    pressure.clone()
                } else {
                    clear.clone()
                }
            },
            || elapsed.get(),
            |delay| elapsed.set(elapsed.get() + delay),
            Duration::from_millis(500),
        )
        .unwrap();
        assert_eq!(observations.get(), 3);
        assert_eq!(elapsed.get(), Duration::from_millis(500));
        elapsed.set(Duration::ZERO);
        assert!(wait_for_admission_with(
            || pressure.clone(),
            || elapsed.get(),
            |delay| elapsed.set(elapsed.get() + delay),
            Duration::from_millis(500)
        )
        .is_err());
        assert_eq!(elapsed.get(), Duration::from_millis(500));
    }

    #[test]
    fn configured_reserve_is_reported_and_applied_without_browser_effects() {
        let key = "AGENT_BROWSER_HOST_MEMORY_RESERVE_BYTES";
        let previous = std::env::var_os(key);
        std::env::set_var(key, u64::MAX.to_string());
        let admission = observe_browser_launch_admission(Path::new("."), None);
        match previous {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
        assert_eq!(admission.minimum_available_memory_bytes, u64::MAX);
        assert!(admission.reasons.contains(&"available_memory_below_floor"));
    }

    #[test]
    fn current_launch_admission_is_typed_and_complete() {
        let admission = observe_browser_launch_admission(Path::new("."), None);
        assert_eq!(
            admission.schema_version,
            "agent-browser.browser-launch-admission.v1"
        );
        assert!(matches!(admission.state, "admitted" | "rejected"));
        assert_eq!(admission.maximum_browser_processes, 64);
        if admission.state == "admitted" {
            admission.require_admitted().unwrap();
            assert!(admission.available_memory_bytes.is_some());
            assert!(admission.available_disk_bytes.is_some());
            assert!(admission.host_process_count.is_some());
            #[cfg(target_os = "linux")]
            assert!(admission.host_process_limit.is_some());
            assert!(admission.browser_process_count.is_some());
        }
    }

    #[test]
    fn zero_browser_capacity_rejects_without_an_effect() {
        let admission = observe_browser_launch_admission(Path::new("."), Some(0));
        assert_eq!(admission.state, "rejected");
        assert!(admission
            .reasons
            .contains(&"browser_process_capacity_exhausted"));
        assert!(admission.require_admitted().is_err());
    }
}
