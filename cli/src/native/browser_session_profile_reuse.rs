//! Current physical profile evidence and task-scoped acquisition replay.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::Path;

use agent_browser_service_model::{
    BrowserProfileCatalogEntry, BrowserTabAcquisition, BrowserTabSource, ManagedBrowserInstance,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::browser::BrowserManager;
use crate::process_identity::{observe_process, ObservedProcessIdentity, ProcessObservation};
mod profile_file_proof;
use profile_file_proof::qualify_profile_files;

/// Qualify the current lock holder from kernel and exact open-profile evidence.
/// Stale locks referring to dead or positively unrelated processes confer no custody.
pub(crate) fn current_profile_browser_identity(
    path: &Path,
) -> Result<Option<ObservedProcessIdentity>, String> {
    if !path.is_absolute() {
        return Err("browser_session_profile_path_not_absolute".into());
    }
    let lock = match fs::read_link(path.join("SingletonLock")) {
        Ok(lock) => lock,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("browser_session_profile_lock_unavailable".into()),
    };
    let lock = lock.to_string_lossy();
    let (host, pid) = lock
        .rsplit_once('-')
        .ok_or("browser_session_profile_lock_invalid")?;
    if sysinfo::System::host_name().as_deref() != Some(host) {
        return Err("browser_session_profile_locked_on_another_host".into());
    }
    let pid: u32 = pid
        .parse()
        .map_err(|_| "browser_session_profile_lock_invalid")?;
    let before = match observe_process(pid) {
        ProcessObservation::Missing => return Ok(None),
        ProcessObservation::Observed(identity)
            if identity.executable_path.is_some() && identity.browser_family.is_none() =>
        {
            return Ok(None)
        }
        ProcessObservation::Observed(identity)
            if identity.browser_family.is_some() && identity.start_token.is_some() =>
        {
            identity
        }
        _ => return Err("browser_session_profile_process_identity_unavailable".into()),
    };
    let expected = path
        .canonicalize()
        .map_err(|_| "browser_session_profile_unavailable")?;
    let files = fs::read_dir(format!("/proc/{pid}/fd"))
        .map_err(|_| "browser_session_profile_process_identity_unavailable")?
        .flatten()
        .filter_map(|entry| fs::read_link(entry.path()).ok())
        .collect::<Vec<_>>();
    if observe_process(pid) != ProcessObservation::Observed(before.clone()) {
        return Err("browser_session_profile_process_identity_unavailable".into());
    }
    if let Some(actual) = before.command_line.as_ref().and_then(|arguments| {
        crate::process_identity::command_line_option_value(arguments, "--user-data-dir")
    }) {
        if fs::canonicalize(actual).ok().as_ref() != Some(&expected) {
            return Ok(None);
        }
    }
    let matched = qualify_profile_files(&expected, files, || {
        #[cfg(target_os = "linux")]
        {
            crate::process_identity::observe_profile_files(&before, &expected)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err("browser_session_profile_process_identity_unavailable".into())
        }
    })?;
    Ok(matched.then_some(before))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskClaim {
    task_key: String,
    marker_url: String,
    target_id: Option<String>,
}

fn journal(profile: &Path) -> Option<File> {
    let directory = profile.join(".agent-browser-session-state");
    match fs::DirBuilder::new().mode(0o700).create(&directory) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(_) => return None,
    }
    let metadata = fs::symlink_metadata(&directory).ok()?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return None;
    }
    let file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(directory.join("task-targets.jsonl"))
        .ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return None;
    }
    // File closure releases this process's acquisition lock, including on failure.
    (unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0).then_some(file)
}

fn read_claim(file: &mut File, task_key: &str) -> Option<TaskClaim> {
    // Bound history reads; malformed or unrelated rows cannot veto acquisition.
    let length = file.metadata().ok()?.len().min(1024 * 1024);
    file.seek(SeekFrom::End(-(length as i64))).ok()?;
    BufReader::new(file)
        .take(1024 * 1024)
        .split(b'\n')
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_slice::<TaskClaim>(&line).ok())
        .filter(|claim| {
            claim.task_key == task_key
                && claim
                    .marker_url
                    .strip_prefix("about:blank#agent-browser-task-")
                    .is_some_and(|nonce| uuid::Uuid::parse_str(nonce).is_ok())
        })
        .last()
}

fn append_claim(file: &mut File, claim: &TaskClaim) -> bool {
    let Ok(record) = serde_json::to_string(claim) else {
        return false;
    };
    file.write_all(format!("\n{record}\n").as_bytes())
        .and_then(|()| file.sync_all())
        .is_ok()
}

pub(crate) struct TaskTabAcquisition {
    pub(crate) acquisition: BrowserTabAcquisition,
    pub(crate) created: bool,
}

/// Reuse only this current physical browser's observed named-task target.
/// A private birth marker covers creation before target observation is journaled.
/// Journal failures are diagnostic; they never deny an otherwise valid open.
pub(crate) async fn acquire_task_tab(
    manager: &mut BrowserManager,
    browser: &ManagedBrowserInstance,
    profile: &BrowserProfileCatalogEntry,
    task_kind: &str,
    session_name: &str,
    attributed: &[String],
) -> Result<TaskTabAcquisition, String> {
    if browser.profile_id != profile.id {
        return Err("browser_session_recovery_identity_conflict".into());
    }
    let identity = current_profile_browser_identity(Path::new(&profile.user_data_dir))?
        .filter(|identity| identity.pid == browser.pid)
        .ok_or("browser_session_profile_process_identity_unavailable")?;
    let task_key = hex::encode(Sha256::digest(format!(
        "{}\0{}\0{}\0{}\0{}",
        identity.pid,
        identity.start_token.as_deref().unwrap_or_default(),
        profile.id,
        task_kind,
        session_name
    )));
    let mut journal = journal(Path::new(&profile.user_data_dir));
    let mut claim = journal
        .as_mut()
        .and_then(|file| read_claim(file, &task_key))
        .unwrap_or_else(|| TaskClaim {
            task_key,
            marker_url: format!("about:blank#agent-browser-task-{}", uuid::Uuid::new_v4()),
            target_id: None,
        });
    let targets = manager
        .client
        .send_command_no_params("Target.getTargets", None)
        .await?;
    let targets = targets["targetInfos"]
        .as_array()
        .ok_or("browser_session_target_inventory_unavailable")?;
    let target = targets
        .iter()
        .filter(|target| target["type"] == "page")
        .filter(|target| {
            target["targetId"]
                .as_str()
                .is_some_and(|id| !attributed.iter().any(|other| other == id))
        })
        .find(|target| {
            target["targetId"].as_str() == claim.target_id.as_deref()
                || target["url"].as_str() == Some(claim.marker_url.as_str())
        })
        .and_then(|target| target["targetId"].as_str())
        .map(str::to_owned);
    let created = target.is_none();
    let target_id = if let Some(target) = target {
        if !manager.has_target(&target) {
            *manager = super::browser_session_runtime::attach_verified_browser_endpoint(
                manager.get_cdp_url(),
                browser.pid,
                identity.clone(),
            )
            .await?;
        }
        target
    } else {
        if claim.target_id.is_some() {
            claim.marker_url = format!("about:blank#agent-browser-task-{}", uuid::Uuid::new_v4());
        }
        claim.target_id = None;
        if let Some(file) = journal.as_mut() {
            let _ = append_claim(file, &claim);
        }
        let created = manager.tab_new_background(Some(&claim.marker_url)).await?;
        created["targetId"]
            .as_str()
            .ok_or("browser_session_tab_target_missing")?
            .to_owned()
    };
    claim.target_id = Some(target_id.clone());
    let persisted = journal
        .as_mut()
        .is_some_and(|file| append_claim(file, &claim));
    if persisted {
        let _ = manager
            .clear_initial_task_marker(&target_id, &claim.marker_url)
            .await;
    } else {
        eprintln!("browser_session_profile_claim_persistence_unavailable: acquisition continues");
    }
    Ok(TaskTabAcquisition {
        acquisition: BrowserTabAcquisition {
            tab_id: target_id.clone(),
            target_id,
            source: BrowserTabSource::SessionInitial,
        },
        created,
    })
}
