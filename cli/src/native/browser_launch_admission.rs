use std::path::{Path, PathBuf};

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

pub(crate) fn observe_browser_launch_admission(
    profile_path: &Path,
    maximum_browser_processes: Option<u32>,
) -> BrowserLaunchAdmission {
    let system = sysinfo::System::new_all();
    let available_memory_bytes = Some(system.available_memory());
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
    match available_memory_bytes {
        Some(bytes) if bytes < MINIMUM_AVAILABLE_MEMORY_BYTES => {
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
        minimum_available_memory_bytes: MINIMUM_AVAILABLE_MEMORY_BYTES,
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
