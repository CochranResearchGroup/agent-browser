//! Trusted Linux process/profile observation. Public requests supply no proofs.
use super::{BrowserProfileCatalogEntry, LostTaskRetirementObservation, RecordedProcessIdentity};
use std::path::Path;

pub(super) struct LinuxOwner {
    #[cfg(test)]
    roots: Option<std::path::PathBuf>,
    browser_id: String,
    binding_only: bool,
    control: crate::native::browser_host_control::RetirementGuard,
}
impl LinuxOwner {
    pub(super) fn acquire_binding(browser_id: &str) -> Result<Self, String> {
        Ok(Self {
            #[cfg(test)]
            roots: None,
            browser_id: browser_id.into(),
            binding_only: true,
            control: crate::native::browser_host_control::binding_guard(browser_id)?,
        })
    }
    #[cfg(test)]
    pub(super) fn fixture(browser_id: &str, root: &Path) -> Result<Self, String> {
        Ok(Self {
            browser_id: browser_id.into(),
            binding_only: false,
            control: crate::native::browser_host_control::fixture_guard(
                &root.join("control.json"),
            )?,
            roots: Some(root.into()),
        })
    }
    pub(super) fn acquire(browser_id: &str) -> Result<Self, String> {
        Ok(Self {
            #[cfg(test)]
            roots: None,
            browser_id: browser_id.into(),
            binding_only: false,
            control: crate::native::browser_host_control::retirement_guard(browser_id)?,
        })
    }
}
impl super::OwnerObserver for LinuxOwner {
    fn control_epoch(&self, browser_id: &str) -> Result<u64, String> {
        if browser_id != self.browser_id {
            return Err(super::held());
        }
        Ok(self.control.epoch())
    }
    fn observe_loss(
        &self,
        profile: &BrowserProfileCatalogEntry,
        original: &RecordedProcessIdentity,
        epoch: u64,
    ) -> Result<LostTaskRetirementObservation, String> {
        if self.binding_only || epoch != self.control.epoch() {
            return Err(super::held());
        }
        #[cfg(test)]
        if let Some(root) = &self.roots {
            // SAFETY: geteuid has no arguments or failure path.
            return observe_at(
                profile,
                original,
                epoch,
                &root.join("proc"),
                &root.join("boot"),
                &root.join("hostname"),
                unsafe { libc::geteuid() },
            );
        }
        // SAFETY: geteuid has no arguments or failure path.
        observe_at(
            profile,
            original,
            epoch,
            Path::new("/proc"),
            Path::new("/proc/sys/kernel/random/boot_id"),
            Path::new("/proc/sys/kernel/hostname"),
            unsafe { libc::geteuid() },
        )
    }
}

struct CensusProcess {
    parent: u32,
    birth: String,
    command: Vec<u8>,
    profile: Option<std::path::PathBuf>,
}
fn census_stat(stat: &str, pid: u32) -> Result<(String, u32, String), String> {
    let held = || "browser_task_retirement_kernel_evidence_unverified".to_string();
    if stat
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        != Some(pid)
    {
        return Err(held());
    }
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .ok_or_else(held)?
        .1
        .split_whitespace()
        .collect();
    let state = fields.first().ok_or_else(held)?;
    let parent = fields
        .get(1)
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(held)?;
    let birth = fields
        .get(19)
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(held)?;
    Ok((state.to_string(), parent, birth.to_string()))
}

// Explicit roots are private to this module and its disposable filesystem tests.
// Production construction will bind /proc and kernel boot/hostname files.
fn observe_at(
    profile: &BrowserProfileCatalogEntry,
    original: &RecordedProcessIdentity,
    epoch: u64,
    proc_root: &Path,
    boot_file: &Path,
    hostname_file: &Path,
    uid: u32,
) -> Result<LostTaskRetirementObservation, String> {
    use std::os::unix::fs::MetadataExt;
    let held = || "browser_task_retirement_kernel_evidence_unverified".to_string();
    let boot = std::fs::read_to_string(boot_file).map_err(|_| held())?;
    let boot = boot.trim();
    let current = uuid::Uuid::parse_str(boot).map_err(|_| held())?;
    let mut token = original.start_token.split(':');
    if token.next() != Some("linux") || original.pid <= 1 || uid == 0 {
        return Err(held());
    }
    let previous = uuid::Uuid::parse_str(token.next().ok_or_else(held)?).map_err(|_| held())?;
    let ticks = token.next().ok_or_else(held)?;
    if ticks.is_empty()
        || !ticks.bytes().all(|b| b.is_ascii_digit())
        || token.next().is_some()
        || previous == current
    {
        return Err(held());
    }
    let absent = || match std::fs::symlink_metadata(proc_root.join(original.pid.to_string())) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(held()), // Any reused PID is held, even on a different boot.
    };
    absent()?;
    let path = Path::new(&profile.user_data_dir);
    let canonical = std::fs::canonicalize(path).map_err(|_| held())?;
    let metadata = std::fs::symlink_metadata(path).map_err(|_| held())?;
    if !path.is_absolute()
        || path != canonical
        || !metadata.is_dir()
        || metadata.uid() != uid
        || metadata.mode() & 0o022 != 0
    {
        return Err(held());
    }
    let hostname = std::fs::read_to_string(hostname_file).map_err(|_| held())?;
    let hostname = hostname.trim();
    if hostname.is_empty() || hostname.contains('/') || hostname.chars().any(char::is_control) {
        return Err(held());
    }
    let lock_path = path.join("SingletonLock");
    let lock = std::fs::symlink_metadata(&lock_path).map_err(|_| held())?;
    let expected_lock = format!("{hostname}-{}", original.pid);
    if !lock.file_type().is_symlink()
        || lock.uid() != uid
        || std::fs::read_link(&lock_path).map_err(|_| held())? != Path::new(&expected_lock)
    {
        return Err(held());
    }
    // No command lines leave this function. Only same-user browser roots need
    // profile classification; child renderers do not own SingletonLock.
    let mut census = std::collections::BTreeMap::<u32, CensusProcess>::new();
    for entry in std::fs::read_dir(proc_root).map_err(|_| held())? {
        let entry = entry.map_err(|_| held())?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        let process = entry.path();
        let info = match std::fs::metadata(&process) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(held()),
        };
        if info.uid() != uid {
            continue;
        }
        let comm = match std::fs::read_to_string(process.join("comm")) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !process.exists() => continue,
            Err(_) => return Err(held()),
        };
        let name = comm.trim().to_ascii_lowercase();
        if !["chrome", "chromium", "lightpanda"]
            .iter()
            .any(|s| name.contains(s))
        {
            continue;
        }
        let stat = std::fs::read_to_string(process.join("stat")).map_err(|_| held())?;
        let (state, parent, birth) = census_stat(&stat, pid)?;
        if state == "Z" {
            continue;
        }
        // Chromium's crash handler does not own a browser profile. Linux comm
        // truncates its executable name to chrome_crashpad (15 bytes).
        if matches!(name.as_str(), "chrome_crashpad" | "chrome_crashpad_handler") {
            continue;
        }
        let raw = std::fs::read(process.join("cmdline")).map_err(|_| held())?;
        if raw.len() > 1024 * 1024 {
            return Err(held());
        }
        let args: Vec<&str> = raw
            .split(|b| *b == 0)
            .filter(|b| !b.is_empty())
            .map(std::str::from_utf8)
            .collect::<Result<_, _>>()
            .map_err(|_| held())?;
        let mut paths: Vec<String> = Vec::new();
        let mut child = false;
        let child_kind = |v: &str| {
            matches!(
                v,
                "renderer" | "gpu-process" | "utility" | "zygote" | "broker"
            )
        };
        if args.len() == 1 {
            // A Chromium process title may replace argv with a single string.
            // Flag boundaries, rather than whitespace, delimit paths with spaces.
            let title = args[0];
            let bytes = title.as_bytes();
            let mut flags = Vec::new();
            for start in 0..bytes.len().saturating_sub(2) {
                if (start == 0 || bytes[start - 1].is_ascii_whitespace())
                    && bytes[start..].starts_with(b"--")
                    && bytes[start + 2].is_ascii_alphabetic()
                {
                    let mut end = start + 2;
                    while end < bytes.len()
                        && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-')
                    {
                        end += 1;
                    }
                    if end == bytes.len() || bytes[end] == b'=' || bytes[end].is_ascii_whitespace()
                    {
                        flags.push((start, if end < bytes.len() { end + 1 } else { end }));
                    }
                }
            }
            for (i, (start, end)) in flags.iter().enumerate() {
                let flag = &title[*start..*end];
                let value =
                    title[*end..flags.get(i + 1).map_or(title.len(), |(start, _)| *start)].trim();
                if flag == "--user-data-dir=" || flag.trim_end() == "--user-data-dir" {
                    paths.push(value.into());
                }
                if flag == "--type=" {
                    child = child_kind(value);
                }
            }
        } else {
            for (i, arg) in args.iter().enumerate() {
                if let Some(p) = arg.strip_prefix("--user-data-dir=") {
                    paths.push(p.into());
                }
                if *arg == "--user-data-dir" {
                    paths.push((*args.get(i + 1).ok_or_else(held)?).into());
                }
                if let Some(kind) = arg.strip_prefix("--type=") {
                    child = child_kind(kind);
                }
            }
        }
        // An explicit profile is always checked, even on a child process.
        if paths.is_empty() && child {
            census.insert(
                pid,
                CensusProcess {
                    parent,
                    birth,
                    command: raw,
                    profile: None,
                },
            );
            continue;
        }
        if paths.len() != 1 || paths[0].is_empty() {
            return Err(held());
        }
        let other = std::fs::canonicalize(&paths[0]).map_err(|_| held())?;
        if other == canonical {
            return Err(held());
        }
        census.insert(
            pid,
            CensusProcess {
                parent,
                birth,
                command: raw,
                profile: Some(other),
            },
        );
    }
    // A surviving child without its own profile is safe only when its current
    // ancestry reaches a positively classified different-profile browser root.
    for (pid, process) in &census {
        if process.profile.is_none() {
            let mut parent = process.parent;
            let mut visited = std::collections::BTreeSet::from([*pid]);
            loop {
                if !visited.insert(parent) {
                    return Err(held());
                }
                let ancestor = census.get(&parent).ok_or_else(held)?;
                if ancestor.profile.is_some() {
                    break;
                }
                parent = ancestor.parent;
            }
        }
    }
    // No PID/parent/birth/argv changes may convert this census into a different
    // owner's proof while the owning store prepares its atomic publication.
    for (pid, process) in &census {
        let root = proc_root.join(pid.to_string());
        if std::fs::metadata(&root).map_err(|_| held())?.uid() != uid {
            return Err(held());
        }
        let stat = std::fs::read_to_string(root.join("stat")).map_err(|_| held())?;
        let (state, parent, birth) = census_stat(&stat, *pid)?;
        if state == "Z"
            || parent != process.parent
            || birth != process.birth
            || std::fs::read(root.join("cmdline")).map_err(|_| held())? != process.command
        {
            return Err(held());
        }
        if let Some(profile) = &process.profile {
            if std::fs::canonicalize(profile).map_err(|_| held())? != *profile
                || profile == &canonical
            {
                return Err(held());
            }
        }
    }
    // Recheck mutable physical witnesses before returning. Owning-store launch
    // fences/control locking remain independent requirements of publication.
    absent()?;
    let after = std::fs::symlink_metadata(path).map_err(|_| held())?;
    let lock_after = std::fs::symlink_metadata(&lock_path).map_err(|_| held())?;
    if (after.dev(), after.ino(), after.uid(), after.mode())
        != (
            metadata.dev(),
            metadata.ino(),
            metadata.uid(),
            metadata.mode(),
        )
        || (lock_after.dev(), lock_after.ino(), lock_after.uid())
            != (lock.dev(), lock.ino(), lock.uid())
        || std::fs::read_link(&lock_path).map_err(|_| held())? != Path::new(&expected_lock)
        || std::fs::read_to_string(boot_file)
            .map_err(|_| held())?
            .trim()
            != boot
    {
        return Err(held());
    }
    Ok(LostTaskRetirementObservation {
        original_process: original.clone(),
        current_boot_id: boot.into(),
        original_pid_absent: true,
        profile_unoccupied: true,
        control_epoch: epoch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::BrowserProfileKind;
    use std::os::unix::fs::{symlink, PermissionsExt};
    #[test]
    fn lost_task_retirement_linux_observer_crossboot_profile_evidence() {
        let dir = std::env::temp_dir().join(format!("auth650-linux-{}", uuid::Uuid::new_v4()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                std::fs::remove_dir_all(&self.0).unwrap();
            }
        }
        std::fs::create_dir(&dir).unwrap();
        let _cleanup = Cleanup(dir.clone());
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let proc_root = dir.join("proc");
        std::fs::create_dir(&proc_root).unwrap();
        let profile_dir = dir.join("profile");
        std::fs::create_dir(&profile_dir).unwrap();
        std::fs::set_permissions(&profile_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let boot = dir.join("boot");
        let hostname = dir.join("hostname");
        std::fs::write(&boot, "00000000-0000-0000-0000-000000000002\n").unwrap();
        std::fs::write(&hostname, "synthetic-host\n").unwrap();
        symlink("synthetic-host-4100", profile_dir.join("SingletonLock")).unwrap();
        let profile = BrowserProfileCatalogEntry {
            id: "profile-a".into(),
            name: "Synthetic".into(),
            user_data_dir: profile_dir.to_string_lossy().into_owned(),
            kind: BrowserProfileKind::Named,
            manual_login_url: None,
        };
        let original = RecordedProcessIdentity {
            pid: 4100,
            start_token: "linux:00000000-0000-0000-0000-000000000001:17".into(),
            executable_path: Some("/synthetic/chromium".into()),
            browser_family: None,
        };
        // SAFETY: geteuid has no arguments or failure path.
        let uid = unsafe { libc::geteuid() };
        let observation =
            observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).unwrap();
        assert_eq!(observation.original_process, original);
        assert!(observation.original_pid_absent && observation.profile_unoccupied);
        assert_eq!(observation.control_epoch, 3);
        assert_eq!(
            std::fs::read_link(profile_dir.join("SingletonLock")).unwrap(),
            Path::new("synthetic-host-4100")
        );
        std::fs::create_dir(proc_root.join("4100")).unwrap();
        assert!(observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_err());
        std::fs::remove_dir(proc_root.join("4100")).unwrap();
        std::fs::write(&boot, "00000000-0000-0000-0000-000000000001").unwrap();
        assert!(observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_err());
        std::fs::write(&boot, "00000000-0000-0000-0000-000000000002").unwrap();
        std::fs::set_permissions(&profile_dir, std::fs::Permissions::from_mode(0o770)).unwrap();
        assert!(observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_err());
        std::fs::set_permissions(&profile_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::remove_file(profile_dir.join("SingletonLock")).unwrap();
        symlink("other-host-4100", profile_dir.join("SingletonLock")).unwrap();
        assert!(observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_err());
        std::fs::remove_file(profile_dir.join("SingletonLock")).unwrap();
        symlink("synthetic-host-4100", profile_dir.join("SingletonLock")).unwrap();
        let browser = proc_root.join("4200");
        std::fs::create_dir(&browser).unwrap();
        std::fs::write(
            browser.join("stat"),
            format!("4200 (chromium) S 1 {}17", "0 ".repeat(17)),
        )
        .unwrap();
        std::fs::write(browser.join("comm"), "chromium\n").unwrap();
        let args = format!("chromium\0--user-data-dir={}\0", profile_dir.display());
        std::fs::write(browser.join("cmdline"), args.as_bytes()).unwrap();
        assert!(observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_err());
        std::fs::remove_file(browser.join("cmdline")).unwrap();
        assert!(observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_err());
        std::fs::remove_dir_all(&browser).unwrap();
        let other_profile = dir.join("other-profile");
        std::fs::create_dir(&other_profile).unwrap();
        std::fs::create_dir(&browser).unwrap();
        std::fs::write(
            browser.join("stat"),
            format!("4200 (chromium) S 1 {}17", "0 ".repeat(17)),
        )
        .unwrap();
        std::fs::write(browser.join("comm"), "chromium\n").unwrap();
        let title = format!(
            "chromium --user-data-dir={} --disable-gpu\0",
            other_profile.display()
        );
        std::fs::write(browser.join("cmdline"), title.as_bytes()).unwrap();
        assert!(
            observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_ok(),
            "unrelated root with Chromium rewritten title must be classified"
        );
        std::fs::write(browser.join("comm"), "chrome_crashpad\n").unwrap();
        std::fs::write(
            browser.join("cmdline"),
            b"chrome_crashpad_handler\0--monitor-self\0",
        )
        .unwrap();
        assert!(
            observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_ok(),
            "crash handler is not a profile root"
        );
        std::fs::write(browser.join("comm"), "chromium\n").unwrap();
        std::fs::write(browser.join("cmdline"), b"chromium\0--type=renderer\0").unwrap();
        assert!(
            observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_err(),
            "unattributed surviving child cannot prove profile vacancy"
        );
        let parent = proc_root.join("4300");
        std::fs::create_dir(&parent).unwrap();
        std::fs::write(parent.join("comm"), "chromium").unwrap();
        std::fs::write(
            parent.join("stat"),
            format!("4300 (chromium) S 1 {}21", "0 ".repeat(17)),
        )
        .unwrap();
        std::fs::write(
            parent.join("cmdline"),
            format!("chromium\0--user-data-dir={}\0", other_profile.display()),
        )
        .unwrap();
        std::fs::write(
            browser.join("stat"),
            format!("4200 (chromium) S 4300 {}17", "0 ".repeat(17)),
        )
        .unwrap();
        assert!(
            observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_ok(),
            "child of positively classified other-profile root is independent"
        );
        std::fs::write(
            browser.join("stat"),
            format!("4200 (chromium) S 4200 {}17", "0 ".repeat(17)),
        )
        .unwrap();
        assert!(
            observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_err(),
            "cyclic ancestry stays held"
        );
        std::fs::remove_dir_all(&parent).unwrap();

        std::fs::remove_dir_all(&browser).unwrap();
        std::fs::write(&boot, "malformed").unwrap();
        assert!(observe_at(&profile, &original, 3, &proc_root, &boot, &hostname, uid).is_err());
    }
}
