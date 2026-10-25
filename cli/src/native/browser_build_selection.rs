//! Profile-scoped executable evidence. Discovery never opens a browser profile;
//! credential compatibility and profile readability need independent observation.
use super::*;
use agent_browser_service_model::BrowserProfileCatalogEntry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ExecutableEvidence {
    pub path: String,
    pub id: String,
    pub name: String,
    pub family: Option<String>,
    pub version: Option<String>,
    pub sha256: Option<String>,
    pub available: bool,
}
impl ExecutableEvidence {
    pub fn public(&self) -> Value {
        json!({"id":self.id,"name":self.name,"family":self.family,"version":self.version,
            "sha256":self.sha256,"available":self.available,
            "familyEvidence":if self.version.is_some(){"version_output"}else{"filename_hint_or_unknown"}})
    }
}

pub(super) fn executable_digest(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|_| "browser_build_unavailable")?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| "browser_build_unreadable")?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn numeric_version(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() > 64 {
        return None;
    }
    let parts: Vec<_> = value.split('.').collect();
    (parts.len() >= 2
        && parts.len() <= 5
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.parse::<u32>().is_ok()))
    .then(|| value.to_owned())
}
#[cfg(unix)]
fn version_output(path: &Path) -> Option<(String, String)> {
    use std::os::unix::{
        fs::{DirBuilderExt, PermissionsExt},
        process::CommandExt,
    };
    use std::process::{Command, Stdio};
    let root = std::env::temp_dir().join(format!(
        "agent-browser-build-probe-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::DirBuilder::new().mode(0o700).create(&root).ok()?;
    struct Guard(PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Guard(root.clone());
    let output_path = root.join("version.txt");
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_path)
        .ok()?;
    output
        .set_permissions(std::fs::Permissions::from_mode(0o600))
        .ok()?;
    let mut command = Command::new(path);
    command
        .arg("--version")
        .arg(format!(
            "--user-data-dir={}",
            root.join("profile").display()
        ))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .env("HOME", &root)
        .env("XDG_CONFIG_HOME", &root)
        .env("XDG_CACHE_HOME", &root)
        .env("XDG_DATA_HOME", &root)
        .env("XDG_RUNTIME_DIR", &root)
        .env_remove("DISPLAY")
        .env_remove("XAUTHORITY")
        .env_remove("DBUS_SESSION_BUS_ADDRESS")
        .stdin(Stdio::null())
        .stdout(output)
        .stderr(Stdio::null())
        .process_group(0);
    let mut child = command.spawn().ok()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                break;
            }
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            Ok(None) => {
                // The unreaped leader reserves this newly created process group.
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                return None;
            }
            Err(_) => return None,
        }
    }
    let mut text = String::new();
    std::fs::File::open(output_path)
        .ok()?
        .take(4096)
        .read_to_string(&mut text)
        .ok()?;
    let family = if text.starts_with("Chromium ") {
        "chromium"
    } else if text.starts_with("Google Chrome ") || text.starts_with("Chrome for Testing ") {
        "chrome"
    } else {
        return None;
    };
    let version = text.split_whitespace().find_map(numeric_version)?;
    Some((family.into(), version))
}
#[cfg(not(unix))]
fn version_output(_: &Path) -> Option<(String, String)> {
    None
}

pub(super) fn inspect_executable(path: &Path) -> ExecutableEvidence {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let hint = path.to_string_lossy().to_ascii_lowercase();
    let family = if hint.contains("chromium") {
        Some("chromium".into())
    } else if hint.contains("chrome") {
        Some("chrome".into())
    } else {
        None
    };
    let executable = path.is_absolute() && path.is_file() && {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            path.metadata()
                .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            true
        }
    };
    let sha256 = executable.then(|| executable_digest(&path).ok()).flatten();
    // Unknown programs are not executed merely to discover their identity.
    let observed = (executable && family.is_some())
        .then(|| version_output(&path))
        .flatten();
    let version = observed.as_ref().map(|(_, v)| v.clone());
    let family = observed.map(|(f, _)| f).or(family);
    let stable = sha256
        .as_ref()
        .is_none_or(|digest| executable_digest(&path).as_ref() == Ok(digest));
    let path = path.to_string_lossy().into_owned();
    let id = format!(
        "build:{:x}",
        Sha256::digest(
            format!("{path}\0{}", sha256.as_deref().unwrap_or("unavailable")).as_bytes()
        )
    );
    ExecutableEvidence {
        path,
        id,
        name,
        family,
        version,
        available: executable && sha256.is_some() && stable,
        sha256,
    }
}
pub(super) fn profile_data_version(profile: &BrowserProfileCatalogEntry) -> Option<String> {
    let file = std::fs::File::open(Path::new(&profile.user_data_dir).join("Last Version")).ok()?;
    let mut text = String::new();
    file.take(128).read_to_string(&mut text).ok()?;
    numeric_version(&text)
}
impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    BrowserSessionHost<P, E>
{
    pub(crate) fn browser_build_select(&mut self, command: &Value) -> Result<Value, String> {
        let choice = optional_string(command, "choice").unwrap_or("inspect");
        if choice == "select_for_next_launch" {
            return self.select_offline_build(command);
        }
        if choice != "inspect" {
            return Err("browser_build_choice_unsupported".into());
        }
        let (state, catalog) = self.persistence.load_owner_snapshot()?;
        let task = optional_string(command, "sessionId")
            .map(|id| state.sessions.get(id).ok_or("browser_build_task_missing"))
            .transpose()?;
        let profile_id = optional_string(command, "profileId")
            .or_else(|| task.map(|t| t.profile_id.as_str()))
            .ok_or("browser_build_profile_required")?;
        if task.is_some_and(|t| {
            t.profile_id != profile_id
                || optional_string(command, "sessionName").is_some_and(|n| n != t.name)
        }) {
            return Err("browser_build_task_profile_conflict".into());
        }
        let profile = catalog
            .profiles
            .get(profile_id)
            .or_else(|| {
                state
                    .disposable_profiles
                    .get(profile_id)
                    .map(|p| &p.profile)
            })
            .ok_or("browser_build_profile_missing")?;
        let mut paths = std::collections::BTreeSet::new();
        if let Some(p) = optional_string(command, "executablePath") {
            if !Path::new(p).is_absolute() {
                return Err("browser_build_absolute_path_required".into());
            }
            paths.insert(PathBuf::from(p));
        }
        if let Some(p) = state.profile_executables.get(profile_id) {
            paths.insert(PathBuf::from(p));
        }
        if let Ok(p) = std::env::var("AGENT_BROWSER_EXECUTABLE_PATH") {
            paths.insert(PathBuf::from(p));
        }
        if let Some(p) = crate::native::cdp::chrome::find_chrome() {
            paths.insert(p);
        }
        for p in [
            "/opt/google/chrome/chrome",
            "/usr/lib/chromium/chromium",
            "/usr/lib/chromium-browser/chromium-browser",
        ] {
            if Path::new(p).is_file() {
                paths.insert(PathBuf::from(p));
            }
        }
        let available = paths
            .into_iter()
            .map(|p| inspect_executable(&p).public())
            .collect::<Vec<_>>();
        Ok(
            json!({"schemaVersion":"agent-browser.build-selection.v1","profileId":profile.id,"profileName":profile.name,
            "profileDataVersion":profile_data_version(profile),"credentialStorage":"unverified","profileDataReadable":"unverified",
            "availableBuilds":available,"observedAtMs":current_unix_ms(),
            "warning":"A build change may migrate profile data or lose login. Previous executable selection does not restore profile bytes."}),
        )
    }
    fn select_offline_build(&mut self, command: &Value) -> Result<Value, String> {
        let operation_id = required_string(command, "operationId")?;
        if operation_id.len() > 128 {
            return Err("browser_build_operation_id_invalid".into());
        }
        let profile_id = required_string(command, "profileId")?;
        let executable = required_string(command, "executablePath")?;
        let mode = required_string(command, "mode")?;
        if !matches!(mode, "cdp" | "cdp_free") {
            return Err("browser_build_mode_invalid".into());
        }
        if !Path::new(executable).is_absolute() {
            return Err("browser_build_absolute_path_required".into());
        }
        if command["ownerConfirmed"] != true {
            return Err("browser_build_owner_confirmation_required:review migration and login risk; duplicate-and-test, choose another build or cancel".into());
        }
        let expected_sha = required_string(command, "expectedSha256")?;
        let intent=serde_json::to_string(&json!({"profileId":profile_id,"executablePath":executable,"mode":mode,"sha256":expected_sha})).map_err(|_|"browser_build_intent_invalid")?;
        let database = self.persistence.runtime_database_path()?;
        let retained = BrowserSessionSqliteStore::open_read_only(&database)?
            .profile_build_selection_receipt(operation_id, &intent)?;
        if let Some(receipt) = retained {
            return Ok(receipt);
        }
        let (state, catalog) = self.persistence.load_owner_snapshot()?;
        let profile = catalog
            .profiles
            .get(profile_id)
            .or_else(|| {
                state
                    .disposable_profiles
                    .get(profile_id)
                    .map(|p| &p.profile)
            })
            .ok_or("browser_build_profile_missing")?;
        let selected = inspect_executable(Path::new(executable));
        if !selected.available {
            return Err("browser_build_unavailable:choose another build or cancel".into());
        }
        if selected.sha256.as_deref() != Some(expected_sha) {
            return Err("browser_build_changed:inspect and confirm the current build".into());
        }
        let mut store = BrowserSessionSqliteStore::open(
            database.parent().ok_or("browser_build_store_unavailable")?,
        )?;
        let receipt = store.select_profile_build(
            super::super::browser_session_store::profile_build::ProfileBuildSelection {
                operation_id,
                profile,
                expected_pin: state
                    .profile_executables
                    .get(profile_id)
                    .map(String::as_str),
                intent: &intent,
                executable: &selected.path,
                mode,
                public_build: selected.public(),
            },
            || {
                if executable_digest(Path::new(&selected.path))?.as_str() != expected_sha {
                    return Err(
                        "browser_build_changed:inspect and confirm the current build".into(),
                    );
                }
                Ok(())
            },
        )?;
        let (state, catalog) = self.persistence.load_owner_snapshot()?;
        self.persisted_state = state.clone();
        self.state = state;
        self.catalog = catalog;
        Ok(receipt)
    }
}

/// Review exact executable bytes without asserting profile or credential portability.
pub(super) fn requested_build(command: &Value) -> Result<Option<ExecutableEvidence>, String> {
    let Some(path) = optional_string(command, "executablePath") else {
        return Ok(None);
    };
    if !Path::new(path).is_absolute() {
        return Err("browser_build_absolute_path_required".into());
    }
    let expected = required_string(command, "expectedSha256")?;
    let selected = inspect_executable(Path::new(path));
    if !selected.available {
        return Err("browser_build_unavailable:choose another build or cancel".into());
    }
    if selected.sha256.as_deref() != Some(expected) {
        return Err("browser_build_changed:inspect and confirm the current build".into());
    }
    Ok(Some(selected))
}
/// Verify before close and launch. Confirmation never substitutes for file evidence.
pub(super) fn verify_build(path: &str, expected: Option<&str>) -> Result<(), String> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let path = Path::new(path);
    let executable = path.is_file() && {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            path.metadata()
                .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            true
        }
    };
    if !executable {
        return Err("browser_build_unavailable:choose another build or cancel".into());
    }
    if executable_digest(path)?.as_str() != expected {
        return Err("browser_build_changed:choose another build and review again".into());
    }
    Ok(())
}
