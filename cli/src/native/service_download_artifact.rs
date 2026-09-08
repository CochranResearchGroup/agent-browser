//! Exact-target download observation and process-bound artifact delivery.

use crate::process_identity::{capture_process_identity, RecordedProcessIdentity};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Component, Path};

/// Subscribe on a dedicated connection without changing any existing context's
/// policy. The empty context is reclaimed on detach even if setup is cancelled;
/// explicit disposal must succeed before the caller may click its target.
pub(crate) async fn subscribe(
    endpoint: &str,
) -> Result<agent_browser_cdp::client::CdpClient, String> {
    use agent_browser_cdp::client::CdpClient;
    use serde_json::json;
    let client = CdpClient::connect(endpoint)
        .await
        .map_err(|e| format!("download_subscription_failed: {e}"))?;
    let before = client
        .send_command("Target.getBrowserContexts", None, None)
        .await
        .map_err(|e| format!("download_subscription_failed: {e}"))?;
    let before = before["browserContextIds"]
        .as_array()
        .ok_or("download_subscription_failed: context census missing")?;
    let created = client
        .send_command(
            "Target.createBrowserContext",
            Some(json!({"disposeOnDetach": true})),
            None,
        )
        .await
        .map_err(|e| format!("download_subscription_failed: {e}"))?;
    let context = created["browserContextId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or("download_subscription_failed: created context identity missing")?;
    if before.iter().any(|id| id.as_str() == Some(context)) {
        return Err("download_subscription_failed: context identity was already present".into());
    }
    let subscription = client
        .send_command(
            "Browser.setDownloadBehavior",
            Some(json!({
                "browserContextId": context, "behavior": "deny", "eventsEnabled": true
            })),
            None,
        )
        .await;
    client
        .send_command(
            "Target.disposeBrowserContext",
            Some(json!({"browserContextId": context})),
            None,
        )
        .await
        .map_err(|e| format!("download_subscription_cleanup_failed: {e}"))?;
    subscription.map_err(|e| format!("download_subscription_failed: {e}"))?;
    let remaining = client
        .send_command("Target.getBrowserContexts", None, None)
        .await
        .map_err(|e| format!("download_subscription_cleanup_failed: {e}"))?;
    let contexts = remaining["browserContextIds"]
        .as_array()
        .ok_or("download_subscription_cleanup_failed: context census missing")?;
    if contexts.iter().any(|id| id.as_str() == Some(context)) {
        return Err("download_subscription_cleanup_failed: owned context remains".into());
    }
    Ok(client)
}

#[derive(Default)]
pub(crate) struct DownloadObservation {
    pub frames: HashSet<String>,
    pub guid: Option<String>,
    pub filename: Option<String>,
    pub url: Option<String>,
    pub path: Option<String>,
}

impl DownloadObservation {
    pub fn from_frame_tree(tree: &Value) -> Result<Self, String> {
        fn collect(tree: &Value, frames: &mut HashSet<String>) {
            if let Some(id) = tree["frame"]["id"].as_str().filter(|id| !id.is_empty()) {
                frames.insert(id.to_string());
            }
            if let Some(children) = tree["childFrames"].as_array() {
                for child in children {
                    collect(child, frames);
                }
            }
        }
        let mut observation = Self::default();
        collect(&tree["frameTree"], &mut observation.frames);
        if observation.frames.is_empty() {
            return Err("download_target_unproven: authorized frame tree is empty".into());
        }
        Ok(observation)
    }

    /// Browser events are global. Only a begin event from this target's frame
    /// tree can establish the GUID accepted by subsequent progress events.
    pub fn observe(&mut self, method: &str, params: &Value) -> Result<bool, String> {
        if method == "Browser.downloadWillBegin" {
            if !params["frameId"]
                .as_str()
                .is_some_and(|id| self.frames.contains(id))
            {
                return Ok(false);
            }
            let guid = params["guid"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or("download_event_unproven: matching begin event lacks GUID")?;
            if self
                .guid
                .as_deref()
                .is_some_and(|existing| existing != guid)
            {
                return Err(
                    "download_event_ambiguous: multiple downloads began in the authorized target"
                        .into(),
                );
            }
            self.guid = Some(guid.into());
            self.filename = params["suggestedFilename"].as_str().map(str::to_string);
            self.url = params["url"].as_str().map(str::to_string);
        }
        if method != "Browser.downloadProgress"
            || self.guid.is_none()
            || params["guid"].as_str() != self.guid.as_deref()
        {
            return Ok(false);
        }
        match params["state"].as_str() {
            Some("canceled") => {
                Err("download_canceled: the browser canceled the exact requested download".into())
            }
            Some("completed") => {
                let path = params["filePath"].as_str().filter(|path| !path.is_empty())
                    .ok_or("download_completion_path_missing: browser did not report the completed artifact path")?;
                self.path = Some(path.into());
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

pub(crate) fn verify_process(expected: &RecordedProcessIdentity) -> Result<(), String> {
    if capture_process_identity(expected.pid, None, None).as_ref() != Some(expected) {
        return Err("download_source_identity_unproven: browser process identity changed".into());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct Mount {
    device: String,
    root: std::path::PathBuf,
    point: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
fn mounts(text: &str) -> Result<Vec<Mount>, String> {
    fn decode(value: &str) -> Result<std::path::PathBuf, String> {
        use std::os::unix::ffi::OsStringExt;
        let mut bytes = Vec::new();
        let mut input = value.as_bytes().iter().copied();
        while let Some(byte) = input.next() {
            if byte != b'\\' {
                bytes.push(byte);
                continue;
            }
            let digits: Vec<_> = input.by_ref().take(3).collect();
            if digits.len() != 3 || !digits.iter().all(|n| (b'0'..=b'7').contains(n)) {
                return Err("download_source_identity_unproven: invalid mount escape".into());
            }
            let value = u16::from(digits[0] - b'0') * 64
                + u16::from(digits[1] - b'0') * 8
                + u16::from(digits[2] - b'0');
            bytes.push(
                u8::try_from(value)
                    .map_err(|_| "download_source_identity_unproven: invalid mount byte")?,
            );
        }
        Ok(std::ffi::OsString::from_vec(bytes).into())
    }
    text.lines()
        .map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() < 10 || !fields.contains(&"-") {
                return Err("download_source_identity_unproven: invalid mount census".into());
            }
            let mount = Mount {
                device: fields[2].into(),
                root: decode(fields[3])?,
                point: decode(fields[4])?,
            };
            path_components(&mount.root)?;
            path_components(&mount.point)?;
            Ok(mount)
        })
        .collect()
}

/// A restarted user-systemd host can read mountinfo while its user namespace
/// cannot follow another generation's /proc/PID/root. Translate only when both
/// kernel mount tables prove the same device and filesystem-relative path.
#[cfg(target_os = "linux")]
fn mapped_source(
    source: &Path,
    browser: &[Mount],
    local: &[Mount],
) -> Result<std::path::PathBuf, String> {
    path_components(source)?;
    let covering = |path: &Path, entries: &[Mount]| {
        entries
            .iter()
            .enumerate()
            .filter(|(_, m)| path.starts_with(&m.point))
            .max_by_key(|(_, m)| m.point.components().count())
            .map(|(index, _)| index)
    };
    let origin = covering(source, browser)
        .map(|i| &browser[i])
        .ok_or("download_source_identity_unproven: source mount missing")?;
    let relative = origin.root.join(
        source
            .strip_prefix(&origin.point)
            .map_err(|_| "download_source_identity_unproven: mount prefix mismatch")?,
    );
    for mount in local.iter().filter(|m| m.device == origin.device) {
        let Ok(suffix) = relative.strip_prefix(&mount.root) else {
            continue;
        };
        let candidate = mount.point.join(suffix);
        let Some(index) = covering(&candidate, local) else {
            continue;
        };
        let effective = &local[index];
        if effective.device == origin.device
            && effective
                .root
                .join(candidate.strip_prefix(&effective.point).unwrap())
                == relative
        {
            return Ok(candidate);
        }
    }
    Err("download_source_identity_unproven: no equivalent local mount path".into())
}

#[cfg(unix)]
fn path_components(path: &Path) -> Result<Vec<&std::ffi::OsStr>, String> {
    if !path.is_absolute() {
        return Err("download_artifact_path_unsafe: path must be absolute".into());
    }
    path.components()
        .filter(|part| !matches!(part, Component::RootDir))
        .map(|part| match part {
            Component::Normal(name) => Ok(name),
            _ => Err("download_artifact_path_unsafe: path contains traversal or a prefix".into()),
        })
        .collect()
}

#[cfg(unix)]
fn open_beneath(
    root: &std::fs::File,
    path: &Path,
    directory: bool,
) -> Result<std::fs::File, String> {
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    let parts = path_components(path)?;
    let mut current = root
        .try_clone()
        .map_err(|e| format!("download_artifact_path_unsafe: {e}"))?;
    for (index, part) in parts.iter().enumerate() {
        let name = std::ffi::CString::new(part.as_bytes())
            .map_err(|_| "download_artifact_path_unsafe: embedded NUL")?;
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if directory || index + 1 < parts.len() {
                libc::O_DIRECTORY
            } else {
                0
            };
        let fd = unsafe { libc::openat(current.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(format!(
                "download_artifact_path_unsafe: {}",
                std::io::Error::last_os_error()
            ));
        }
        current = unsafe { std::fs::File::from_raw_fd(fd) };
    }
    Ok(current)
}

/// Copy the completed file without consuming its source or overwriting a peer.
/// Directory descriptors keep path resolution anchored across concurrent renames.
#[cfg(unix)]
pub(crate) fn deliver(
    owner: &RecordedProcessIdentity,
    source_path: &Path,
    destination: &Path,
    max_bytes: Option<u64>,
) -> Result<u64, String> {
    use std::fs::File;
    use std::io::Read;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    verify_process(owner)?;
    #[cfg(target_os = "linux")]
    let root_path = format!("/proc/{}/root", owner.pid);
    #[cfg(not(target_os = "linux"))]
    let root_path = "/".to_string();
    #[cfg(target_os = "linux")]
    let mut resolved_source = source_path.to_path_buf();
    #[cfg(not(target_os = "linux"))]
    let resolved_source = source_path.to_path_buf();
    let browser_root = match File::open(root_path) {
        Ok(root) => root,
        #[cfg(target_os = "linux")]
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            let read = |path: String| {
                std::fs::read_to_string(path)
                    .map_err(|e| format!("download_source_identity_unproven: mount census: {e}"))
            };
            let browser = mounts(&read(format!("/proc/{}/mountinfo", owner.pid))?)?;
            let local = mounts(&read("/proc/self/mountinfo".into())?)?;
            resolved_source = mapped_source(source_path, &browser, &local)?;
            File::open("/").map_err(|e| format!("download_source_identity_unproven: {e}"))?
        }
        Err(error) => return Err(format!("download_source_identity_unproven: {error}")),
    };
    verify_process(owner)?;
    let source = open_beneath(&browser_root, &resolved_source, false)?;
    let metadata = source
        .metadata()
        .map_err(|e| format!("download_artifact_path_unsafe: {e}"))?;
    if !metadata.is_file() || metadata.uid() != unsafe { libc::geteuid() } || metadata.nlink() != 1
    {
        return Err(
            "download_artifact_path_unsafe: source must be a singly linked owned regular file"
                .into(),
        );
    }
    if max_bytes.is_some_and(|limit| metadata.len() > limit) {
        return Err("download_artifact_limit_exceeded: completed file exceeds maxBytes".into());
    }
    path_components(destination)?;
    let parent = destination
        .parent()
        .ok_or("download_artifact_path_unsafe: destination has no parent")?;
    let name = std::ffi::CString::new(
        destination
            .file_name()
            .ok_or("download_artifact_path_unsafe: destination has no filename")?
            .as_bytes(),
    )
    .map_err(|_| "download_artifact_path_unsafe: embedded NUL")?;
    let local_root = File::open("/").map_err(|e| format!("download_artifact_path_unsafe: {e}"))?;
    let directory = open_beneath(&local_root, parent, true)?;
    let fd = directory.as_raw_fd();
    let existing = unsafe {
        libc::openat(
            fd,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if existing >= 0 {
        let existing = unsafe { File::from_raw_fd(existing) }
            .metadata()
            .map_err(|e| format!("download_artifact_path_unsafe: {e}"))?;
        if existing.is_file()
            && existing.dev() == metadata.dev()
            && existing.ino() == metadata.ino()
        {
            verify_process(owner)?;
            return Ok(metadata.len());
        }
        return Err("download_destination_exists: preserve the existing destination".into());
    } else if std::io::Error::last_os_error().raw_os_error() != Some(libc::ENOENT) {
        return Err("download_destination_exists: destination cannot be safely created".into());
    }
    let temporary =
        std::ffi::CString::new(format!(".agent-browser-download-{}", uuid::Uuid::new_v4()))
            .unwrap();
    let output_fd = unsafe {
        libc::openat(
            fd,
            temporary.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600 as libc::mode_t,
        )
    };
    if output_fd < 0 {
        return Err(format!(
            "download_artifact_delivery_failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let result = (|| {
        let mut output = unsafe { File::from_raw_fd(output_fd) };
        let limit = max_bytes
            .map(|limit| limit.saturating_add(1))
            .unwrap_or(u64::MAX);
        let size = std::io::copy(&mut (&source).take(limit), &mut output)
            .map_err(|e| format!("download_artifact_delivery_failed: {e}"))?;
        if max_bytes.is_some_and(|limit| size > limit) || size != metadata.len() {
            return Err(
                "download_artifact_limit_exceeded: source size changed or exceeds maxBytes".into(),
            );
        }
        let after = source
            .metadata()
            .map_err(|e| format!("download_artifact_delivery_failed: {e}"))?;
        if after.len() != metadata.len()
            || after.mtime() != metadata.mtime()
            || after.mtime_nsec() != metadata.mtime_nsec()
            || after.ctime() != metadata.ctime()
            || after.ctime_nsec() != metadata.ctime_nsec()
        {
            return Err("download_artifact_delivery_failed: source changed during delivery".into());
        }
        output
            .sync_all()
            .map_err(|e| format!("download_artifact_delivery_failed: {e}"))?;
        verify_process(owner)?;
        if unsafe { libc::linkat(fd, temporary.as_ptr(), fd, name.as_ptr(), 0) } != 0 {
            return Err(format!(
                "download_artifact_delivery_failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(size)
    })();
    unsafe {
        libc::unlinkat(fd, temporary.as_ptr(), 0);
    }
    result
}

#[cfg(windows)]
pub(crate) fn deliver(
    owner: &RecordedProcessIdentity,
    source_path: &Path,
    destination: &Path,
    max_bytes: Option<u64>,
) -> Result<u64, String> {
    use std::fs::{File, OpenOptions};
    use std::io::Read;
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    // Holding every directory without FILE_SHARE_DELETE prevents replacement
    // of an ancestor while the final file is opened and delivered.
    fn guard_parents(path: &Path) -> Result<Vec<File>, String> {
        if !path.is_absolute() {
            return Err("download_artifact_path_unsafe: absolute path required".into());
        }
        let parent = path
            .parent()
            .ok_or("download_artifact_path_unsafe: parent required")?;
        let mut current = std::path::PathBuf::new();
        let mut handles = Vec::new();
        for part in parent.components() {
            if matches!(part, Component::ParentDir | Component::CurDir) {
                return Err("download_artifact_path_unsafe: traversal refused".into());
            }
            current.push(part.as_os_str());
            if !current.is_absolute() {
                continue;
            }
            let handle = OpenOptions::new()
                .read(true)
                .share_mode(3)
                .custom_flags(0x02000000 | 0x00200000)
                .open(&current)
                .map_err(|e| format!("download_artifact_path_unsafe: {e}"))?;
            let metadata = handle
                .metadata()
                .map_err(|e| format!("download_artifact_path_unsafe: {e}"))?;
            if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
                return Err("download_artifact_path_unsafe: reparse directory refused".into());
            }
            handles.push(handle);
        }
        Ok(handles)
    }
    verify_process(owner)?;
    let _source_parents = guard_parents(source_path)?;
    let _destination_parents = guard_parents(destination)?;
    let source = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .custom_flags(0x00200000)
        .open(source_path)
        .map_err(|e| format!("download_artifact_path_unsafe: {e}"))?;
    let metadata = source
        .metadata()
        .map_err(|e| format!("download_artifact_path_unsafe: {e}"))?;
    if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
        return Err("download_artifact_path_unsafe: regular non-reparse source required".into());
    }
    if max_bytes.is_some_and(|limit| metadata.len() > limit) {
        return Err("download_artifact_limit_exceeded: completed file exceeds maxBytes".into());
    }
    if source_path == destination {
        verify_process(owner)?;
        return Ok(metadata.len());
    }
    if destination.exists() {
        return Err("download_destination_exists: preserve existing destination".into());
    }
    let temporary =
        destination.with_file_name(format!(".agent-browser-download-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| format!("download_artifact_delivery_failed: {e}"))?;
        let size = std::io::copy(
            &mut source.take(max_bytes.map(|n| n.saturating_add(1)).unwrap_or(u64::MAX)),
            &mut output,
        )
        .map_err(|e| format!("download_artifact_delivery_failed: {e}"))?;
        if size != metadata.len() || max_bytes.is_some_and(|limit| size > limit) {
            return Err("download_artifact_limit_exceeded: source size changed".into());
        }
        output
            .sync_all()
            .map_err(|e| format!("download_artifact_delivery_failed: {e}"))?;
        drop(output);
        let _sealed = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&temporary)
            .map_err(|e| format!("download_artifact_delivery_failed: {e}"))?;
        verify_process(owner)?;
        std::fs::hard_link(&temporary, destination)
            .map_err(|e| format!("download_artifact_delivery_failed: {e}"))?;
        Ok(size)
    })();
    let _ = std::fs::remove_file(&temporary);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[cfg(target_os = "linux")]
    #[test]
    fn mount_translation_preserves_backing_path_and_rejects_shadowed_mounts() {
        let browser = mounts(
            "1 0 8:1 / / rw - ext4 /dev/x rw\n2 1 8:1 /owned\\040storage /tmp rw - ext4 /dev/x rw",
        )
        .unwrap();
        let local =
            mounts("3 0 8:1 / / rw - ext4 /dev/x rw\n4 3 8:1 /different /tmp rw - ext4 /dev/x rw")
                .unwrap();
        assert_eq!(
            mapped_source(Path::new("/tmp/file.csv"), &browser, &local).unwrap(),
            Path::new("/owned storage/file.csv")
        );
        let shadowed = mounts(
            "3 0 8:1 / / rw - ext4 /dev/x rw\n4 3 8:2 / /owned\\040storage rw - ext4 /dev/y rw",
        )
        .unwrap();
        assert!(mapped_source(Path::new("/tmp/file.csv"), &browser, &shadowed).is_err());
        assert!(mapped_source(Path::new("/tmp/../file.csv"), &browser, &local).is_err());
        assert!(mounts("malformed").is_err());
    }

    #[test]
    fn observation_requires_owned_frame_then_matching_guid_and_completion_path() {
        let mut capture =
            DownloadObservation::from_frame_tree(&json!({"frameTree":{"frame":{"id":"owned"}}}))
                .unwrap();
        assert!(!capture
            .observe(
                "Browser.downloadWillBegin",
                &json!({"frameId":"peer","guid":"foreign"})
            )
            .unwrap());
        assert!(!capture
            .observe(
                "Browser.downloadProgress",
                &json!({"guid":"foreign","state":"completed","filePath":"/secret"})
            )
            .unwrap());
        capture
            .observe(
                "Browser.downloadWillBegin",
                &json!({"frameId":"owned","guid":"mine","suggestedFilename":"file.csv"}),
            )
            .unwrap();
        assert!(!capture
            .observe(
                "Browser.downloadProgress",
                &json!({"guid":"foreign","state":"canceled"})
            )
            .unwrap());
        assert!(capture
            .observe(
                "Browser.downloadProgress",
                &json!({"guid":"mine","state":"completed"})
            )
            .unwrap_err()
            .starts_with("download_completion_path_missing:"));
        assert!(capture
            .observe(
                "Browser.downloadProgress",
                &json!({"guid":"mine","state":"completed","filePath":"/download/file.csv"})
            )
            .unwrap());
        assert_eq!(capture.path.as_deref(), Some("/download/file.csv"));
        assert!(capture
            .observe(
                "Browser.downloadWillBegin",
                &json!({"frameId":"owned","guid":"second"})
            )
            .unwrap_err()
            .starts_with("download_event_ambiguous:"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn delivery_preserves_source_and_refuses_wrong_owner_symlinks_overwrite_and_limits() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!("p160-download-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let destination = root.join("destination");
        std::fs::write(&source, b"exact artifact").unwrap();
        let identity = capture_process_identity(std::process::id(), None, None).unwrap();
        let mut wrong = identity.clone();
        wrong.start_token.push_str("-changed");
        assert!(deliver(&wrong, &source, &destination, None)
            .unwrap_err()
            .starts_with("download_source_identity_unproven:"));
        assert!(deliver(&identity, &source, &destination, Some(2))
            .unwrap_err()
            .starts_with("download_artifact_limit_exceeded:"));
        symlink(&source, root.join("link")).unwrap();
        assert!(deliver(&identity, &root.join("link"), &destination, None).is_err());
        symlink(&root, root.join("directory-link")).unwrap();
        assert!(deliver(
            &identity,
            &root.join("directory-link/source"),
            &destination,
            None
        )
        .is_err());
        assert!(deliver(&identity, &root.join("../outside"), &destination, None).is_err());
        assert!(!destination.exists());
        assert_eq!(
            deliver(&identity, &source, &destination, Some(100)).unwrap(),
            14
        );
        assert_eq!(
            std::fs::read(&source).unwrap(),
            std::fs::read(&destination).unwrap()
        );
        assert!(deliver(&identity, &source, &destination, None)
            .unwrap_err()
            .starts_with("download_destination_exists:"));
        assert_eq!(deliver(&identity, &source, &source, None).unwrap(), 14);
        assert!(!std::fs::read_dir(&root).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".agent-browser-download-")));
        std::fs::remove_dir_all(root).unwrap();
    }
}
