//! Independent durable storage for the ordinary Browser Session Manager path.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use agent_browser_service_model::{
    BrowserProfileCatalog, BrowserProfileCatalogDiagnostic, BrowserSessionState,
    BROWSER_PROFILE_CATALOG_SCHEMA_V1, BROWSER_SESSION_STATE_SCHEMA_V1,
};
use serde::{de::DeserializeOwned, Serialize};

use super::service_store::default_service_state_path;

const BROWSER_SESSION_STATE_FILENAME: &str = "browser-session-state.json";
const BROWSER_PROFILE_CATALOG_FILENAME: &str = "browser-profile-catalog.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserProfileCatalogLoad {
    pub(crate) catalog: BrowserProfileCatalog,
    pub(crate) diagnostics: Vec<BrowserProfileCatalogDiagnostic>,
    pub(crate) imported_legacy_profiles: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserSessionJsonStore {
    session_state_path: PathBuf,
    profile_catalog_path: PathBuf,
}

impl BrowserSessionJsonStore {
    pub(crate) fn new(service_directory: impl Into<PathBuf>) -> Self {
        let service_directory = service_directory.into();
        Self {
            session_state_path: service_directory.join(BROWSER_SESSION_STATE_FILENAME),
            profile_catalog_path: service_directory.join(BROWSER_PROFILE_CATALOG_FILENAME),
        }
    }

    pub(crate) fn default_json() -> Result<Self, String> {
        let legacy_state_path = default_service_state_path()?;
        let service_directory = legacy_state_path
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?;
        Ok(Self::new(service_directory))
    }

    pub(crate) fn load_session_state(&self) -> Result<BrowserSessionState, String> {
        let state: BrowserSessionState = read_json_or_default(&self.session_state_path)?;
        if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                state.schema_version
            ));
        }
        Ok(state)
    }

    pub(crate) fn save_session_state(&self, state: &BrowserSessionState) -> Result<(), String> {
        if state.schema_version != BROWSER_SESSION_STATE_SCHEMA_V1 {
            return Err(format!(
                "browser_session_state_schema_unsupported:{}",
                state.schema_version
            ));
        }
        write_private_json_atomic(&self.session_state_path, state)
    }

    pub(crate) fn load_or_import_profile_catalog(
        &self,
        legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String> {
        if self.profile_catalog_path.exists() {
            let catalog: BrowserProfileCatalog = read_json(&self.profile_catalog_path)?;
            validate_catalog_schema(&catalog)?;
            return Ok(BrowserProfileCatalogLoad {
                catalog,
                diagnostics: Vec::new(),
                imported_legacy_profiles: false,
            });
        }

        let legacy_raw = match fs::read_to_string(legacy_service_state_path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => "{}".to_string(),
            Err(error) => {
                return Err(format!(
                    "browser_profile_legacy_state_read_failed:{}:{error}",
                    legacy_service_state_path.display()
                ))
            }
        };
        serde_json::from_str::<serde_json::Value>(&legacy_raw).map_err(|error| {
            format!(
                "browser_profile_legacy_state_json_invalid:{}:{error}",
                legacy_service_state_path.display()
            )
        })?;
        let imported = BrowserProfileCatalog::import_legacy_service_state_json(&legacy_raw);
        write_private_json_atomic(&self.profile_catalog_path, &imported.catalog)?;
        Ok(BrowserProfileCatalogLoad {
            catalog: imported.catalog,
            diagnostics: imported.diagnostics,
            imported_legacy_profiles: true,
        })
    }

    pub(crate) fn save_profile_catalog(
        &self,
        catalog: &BrowserProfileCatalog,
    ) -> Result<(), String> {
        validate_catalog_schema(catalog)?;
        write_private_json_atomic(&self.profile_catalog_path, catalog)
    }
}

fn validate_catalog_schema(catalog: &BrowserProfileCatalog) -> Result<(), String> {
    if catalog.schema_version != BROWSER_PROFILE_CATALOG_SCHEMA_V1 {
        return Err(format!(
            "browser_profile_catalog_schema_unsupported:{}",
            catalog.schema_version
        ));
    }
    Ok(())
}

fn read_json_or_default<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw)
            .map_err(|error| format!("browser_session_json_invalid:{}:{error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(format!(
            "browser_session_json_read_failed:{}:{error}",
            path.display()
        )),
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "browser_session_json_read_failed:{}:{error}",
            path.display()
        )
    })?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("browser_session_json_invalid:{}:{error}", path.display()))
}

fn write_private_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "browser_session_json_parent_missing".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "browser_session_json_directory_failed:{}:{error}",
            parent.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|error| {
            format!("browser_session_json_directory_permissions_failed:{error}")
        })?;
    }
    let staged = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let body = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("browser_session_json_serialize_failed:{error}"))?;
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged)
            .map_err(|error| format!("browser_session_json_stage_failed:{error}"))?;
        file.write_all(&body)
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("browser_session_json_write_failed:{error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&staged, fs::Permissions::from_mode(0o600))
                .map_err(|error| format!("browser_session_json_permissions_failed:{error}"))?;
        }
        atomic_replace(&staged, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination)
        .map_err(|error| format!("browser_session_json_commit_failed:{error}"))
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(format!(
            "browser_session_json_commit_failed:{}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_browser_service_model::BrowserProfileKind;

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "agent-browser-{label}-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn session_state_round_trips_in_its_independent_file() {
        let directory = TempDirectory::new("browser-session-store");
        let store = BrowserSessionJsonStore::new(&directory.0);
        let mut state = BrowserSessionState::default();
        state.next_session_sequence = 42;

        store.save_session_state(&state).unwrap();
        state.next_session_sequence = 43;
        store.save_session_state(&state).unwrap();
        let loaded = store.load_session_state().unwrap();

        assert_eq!(loaded, state);
        assert!(directory.0.join(BROWSER_SESSION_STATE_FILENAME).exists());
        assert!(!directory.0.join("state.json").exists());
    }

    #[test]
    fn first_catalog_load_imports_profiles_once_and_ignores_other_legacy_fields() {
        let directory = TempDirectory::new("browser-profile-catalog");
        let store = BrowserSessionJsonStore::new(&directory.0);
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({
                "profiles": {
                    "work": {
                        "id": "work",
                        "name": "Work",
                        "userDataDir": "/managed/work",
                        "profileClass": "durable_named"
                    }
                },
                "sessions": "malformed but irrelevant"
            })
            .to_string(),
        )
        .unwrap();

        let imported = store.load_or_import_profile_catalog(&legacy_path).unwrap();
        assert!(imported.imported_legacy_profiles);
        assert_eq!(
            imported.catalog.profiles["work"].kind,
            BrowserProfileKind::Named
        );

        fs::write(&legacy_path, r#"{"profiles":{}}"#).unwrap();
        let authoritative = store.load_or_import_profile_catalog(&legacy_path).unwrap();
        assert!(!authoritative.imported_legacy_profiles);
        assert!(authoritative.catalog.profiles.contains_key("work"));
    }
}
