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
    #[serde(default)]
    pub native_binary: Option<bool>,
}
impl ExecutableEvidence {
    pub fn public(&self) -> Value {
        json!({"id":self.id,"name":self.name,"family":self.family,"version":self.version,
            "sha256":self.sha256,"available":self.available,"nativeBinary":self.native_binary,
            "familyEvidence":if self.version.is_some(){"version_output"}else{"filename_hint_or_unknown"}})
    }
}

fn native_binary(path: &Path) -> Option<bool> {
    #[cfg(target_os = "linux")]
    {
        let mut magic = [0; 4];
        Some(
            std::fs::File::open(path)
                .and_then(|mut file| file.read_exact(&mut magic))
                .is_ok()
                && magic == *b"\x7fELF",
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        None
    }
}
pub(super) fn build_id(path: &str, sha256: &str) -> String {
    format!(
        "build:{:x}",
        Sha256::digest(format!("{path}\0{sha256}").as_bytes())
    )
}
fn candidate_paths(command: &Value, additional: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut paths = std::collections::BTreeSet::new();
    if let Some(path) = optional_string(command, "executablePath") {
        if !Path::new(path).is_absolute() {
            return Err("browser_build_absolute_path_required".into());
        }
        paths.insert(PathBuf::from(path));
    } else {
        for path in additional {
            if Path::new(path).is_absolute() {
                paths.insert(PathBuf::from(path));
            }
        }
        if let Ok(path) = std::env::var("AGENT_BROWSER_EXECUTABLE_PATH") {
            if Path::new(&path).is_absolute() {
                paths.insert(PathBuf::from(path));
            }
        }
        if let Some(path) = crate::native::cdp::chrome::find_chrome() {
            paths.insert(path);
        }
        for path in [
            "/opt/google/chrome/chrome",
            "/usr/lib/chromium/chromium",
            "/usr/lib/chromium-browser/chromium-browser",
            // Inspect the native executable directly. The snap launcher can
            // change desktop settings and is never a discovery probe.
            "/snap/chromium/current/usr/lib/chromium-browser/chrome",
        ] {
            if Path::new(path).is_file() {
                paths.insert(PathBuf::from(path));
            }
        }
    }
    Ok(paths
        .into_iter()
        .map(|path| path.canonicalize().unwrap_or(path))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect())
}
pub(super) fn compatibility(
    profile_version: Option<&str>,
    source_family: Option<&str>,
    build: &ExecutableEvidence,
) -> Value {
    let major = |value: Option<&str>| {
        value
            .and_then(|version| version.split('.').next())
            .and_then(|part| part.parse::<u32>().ok())
    };
    let relation = match (major(profile_version), major(build.version.as_deref())) {
        (Some(profile), Some(target)) if target < profile => "older_major",
        (Some(profile), Some(target)) if target > profile => "newer_major",
        (Some(_), Some(_)) => "same_major_unverified_storage",
        _ => "unverified",
    };
    json!({"profileDataVersion":profile_version,"versionRelation":relation,"familyRelation":match(source_family,build.family.as_deref()){(Some(source),Some(target)) if source!=target=>"different",(Some(_),Some(_))=>"same_family_unverified_storage",_=>"unverified"},"familyPolicy":"advisory","storageFormat":"unverified","credentialStorage":"unverified","authenticationContinuity":"unverified","profileDataReadable":"unverified","options":["duplicate_and_test","choose_another_build","confirmed_original_profile_attempt","cancel"]})
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
    let native_binary = native_binary(Path::new(&path));
    ExecutableEvidence {
        path,
        id,
        name,
        family,
        version,
        available: executable && sha256.is_some() && stable,
        native_binary,
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
    fn profile_build_paths(&self, state: &BrowserSessionState, profile_id: &str) -> Vec<String> {
        use agent_browser_service_model::BrowserLaunchCustodyStore;
        let mut paths = state
            .profile_executables
            .get(profile_id)
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();
        let mut store = self
            .persistence
            .runtime_database_path()
            .ok()
            .and_then(|path| BrowserSessionSqliteStore::open_read_only(&path).ok());
        for browser in state
            .browsers
            .values()
            .filter(|browser| browser.profile_id == profile_id)
        {
            let process = state
                .app_browser_attachments
                .get(&browser.id)
                .map(|attached| attached.request.process.clone())
                .or_else(|| {
                    store.as_mut().and_then(|store| {
                        store
                            .published_launch_process_identity(browser)
                            .ok()
                            .flatten()
                    })
                });
            if let Some(path) = process.and_then(|process| process.executable_path) {
                paths.push(path);
            }
        }
        paths
    }
    pub(super) fn requested_profile_build(
        &self,
        command: &Value,
        profile_id: &str,
    ) -> Result<Option<ExecutableEvidence>, String> {
        let extra = if optional_string(command, "executablePath").is_some() {
            vec![]
        } else {
            let state = self.persistence.load_session_state()?;
            self.profile_build_paths(&state, profile_id)
        };
        requested_build(command, &extra)
    }
    pub(crate) fn browser_build_select(&mut self, command: &Value) -> Result<Value, String> {
        let choice = optional_string(command, "choice").unwrap_or("inspect");
        if choice == "open_selected" {
            return self.open_selected_build(command);
        }
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
        let extra = self.profile_build_paths(&state, profile_id);
        let paths = candidate_paths(command, &extra)?;
        let rows = paths
            .into_iter()
            .map(|path| inspect_executable(&path))
            .collect::<Vec<_>>();
        let pin = state.profile_executables.get(profile_id).map(|path| {
            rows.iter()
                .find(|row| Path::new(&row.path) == Path::new(path))
                .cloned()
                .unwrap_or_else(|| inspect_executable(Path::new(path)))
        });
        let profile_version = profile_data_version(profile);
        let source_family = pin.as_ref().and_then(|pin| pin.family.as_deref());
        let available = rows
            .iter()
            .map(|row| {
                let mut value = row.public();
                value["selection"] = json!({"buildId":row.id,"expectedSha256":row.sha256});
                value["source"] = json!(if pin.as_ref().is_some_and(|pin| pin.id == row.id) {
                    "profile_pin"
                } else {
                    "installed_or_configured"
                });
                value["compatibility"] =
                    compatibility(profile_version.as_deref(), source_family, row);
                value
            })
            .collect::<Vec<_>>();
        let current_selection = if let Some(pin) = &pin {
            BrowserSessionSqliteStore::open_read_only(&self.persistence.runtime_database_path()?)?
                .current_profile_build_selection(
                profile_id,
                &pin.id,
                if profile.manual_login_url.is_some() {
                    "cdp_free"
                } else {
                    "cdp"
                },
            )?
        } else {
            None
        };
        let active_tasks = state
            .sessions
            .values()
            .filter(|task| task.profile_id == profile_id)
            .map(|task| json!({"id":task.id,"name":task.name,"browserId":task.browser_id}))
            .collect::<Vec<_>>();
        Ok(
            json!({"schemaVersion":"agent-browser.build-selection.v1","profileId":profile.id,"profileName":profile.name,
            "profileDataVersion":profile_version,"profileKind":profile.kind,"nextLaunchMode":if profile.manual_login_url.is_some(){"cdp_free"}else{"cdp"},"nextLaunchBuild":pin.as_ref().map(ExecutableEvidence::public),"currentSelectionOperationId":current_selection,"activeTasks":active_tasks,"activeOperationId":state.profile_operation_ids.get(profile_id),"credentialStorage":"unverified","profileDataReadable":"unverified",
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
        let executable = optional_string(command, "executablePath");
        let requested_id = optional_string(command, "buildId");
        if executable.is_none() && requested_id.is_none() {
            return Err("browser_build_selection_required".into());
        }
        let mode = required_string(command, "mode")?;
        if !matches!(mode, "cdp" | "cdp_free") {
            return Err("browser_build_mode_invalid".into());
        }
        if executable.is_some_and(|path| !Path::new(path).is_absolute()) {
            return Err("browser_build_absolute_path_required".into());
        }
        if command["ownerConfirmed"] != true {
            return Err("browser_build_owner_confirmation_required:review migration and login risk; duplicate-and-test, choose another build or cancel".into());
        }
        let expected_sha = required_string(command, "expectedSha256")?;
        let mut intent_value = json!({"profileId":profile_id,"mode":mode,"sha256":expected_sha});
        if let Some(path) = executable {
            intent_value["executablePath"] = json!(path);
        }
        if let Some(id) = requested_id {
            intent_value["buildId"] = json!(id);
        }
        let intent =
            serde_json::to_string(&intent_value).map_err(|_| "browser_build_intent_invalid")?;
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
        let selected = self
            .requested_profile_build(command, profile_id)?
            .ok_or("browser_build_selection_required")?;
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
pub(super) fn requested_build(
    command: &Value,
    additional: &[String],
) -> Result<Option<ExecutableEvidence>, String> {
    let requested_id = optional_string(command, "buildId");
    if optional_string(command, "executablePath").is_none() && requested_id.is_none() {
        return Ok(None);
    }
    let expected = required_string(command, "expectedSha256")?;
    let path = candidate_paths(command, additional)?
        .into_iter()
        .find(|path| {
            requested_id.is_none_or(|id| build_id(&path.to_string_lossy(), expected) == id)
        })
        .ok_or("browser_build_unavailable:refresh discovery or choose another build")?;
    let selected = inspect_executable(&path);
    if !selected.available {
        return Err("browser_build_unavailable:choose another build or cancel".into());
    }
    if selected.sha256.as_deref() != Some(expected) {
        return Err("browser_build_changed:inspect and confirm the current build".into());
    }
    if selected.native_binary == Some(false) {
        return Err(
            "browser_build_launcher_unverified:select the actual browser executable".into(),
        );
    }
    Ok(Some(selected))
}
/// Verify before close and launch. Confirmation never substitutes for file evidence.
pub(super) fn verify_build(path: &str, expected: Option<&str>) -> Result<(), String> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let path = Path::new(path);
    if native_binary(path) == Some(false) {
        return Err(
            "browser_build_launcher_unverified:select the actual browser executable".into(),
        );
    }
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

/// Installed build choices without allocating a profile or opening a browser.
pub(super) fn workspace_builds() -> Result<Vec<Value>, String> {
    Ok(candidate_paths(&json!({}), &[])?
        .into_iter()
        .map(|path| inspect_executable(&path))
        .filter(|row| {
            row.available
                && row.native_binary != Some(false)
                && matches!(row.family.as_deref(), Some("chrome" | "chromium"))
        })
        .map(|row| {
            let mut public = row.public();
            public["selection"] = json!({"buildId":row.id,"expectedSha256":row.sha256});
            public
        })
        .collect())
}
