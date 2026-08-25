//! Controlled desktop input provider internals.
//!
//! This module owns the operating-system-visible effect fence, private effect
//! journal, and closed X11 event adapter used by the existing
//! `desktop_interact` daemon command. Callers provide service-owned opaque
//! identities. They never provide display names, paths, executables, URLs, or
//! arbitrary event payloads.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteEffectFenceIdentity<'a> {
    pub environment_id: &'a str,
    pub route_id: &'a str,
    pub display_allocation_id: &'a str,
}

#[derive(Debug)]
pub(crate) struct RouteEffectFence {
    file: File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopInputProviderError {
    code: &'static str,
}

impl DesktopInputProviderError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }
}

impl RouteEffectFence {
    pub(crate) fn acquire(
        runtime_state_root: &Path,
        identity: &RouteEffectFenceIdentity<'_>,
        deadline: Duration,
    ) -> Result<Self, DesktopInputProviderError> {
        if identity.environment_id.is_empty()
            || identity.route_id.is_empty()
            || identity.display_allocation_id.is_empty()
        {
            return Err(DesktopInputProviderError::new(
                "desktop_input_route_fence_identity_invalid",
            ));
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

            let directory = runtime_state_root.join("service/desktop-input/fences");
            fs::create_dir_all(&directory).map_err(|_| {
                DesktopInputProviderError::new("desktop_input_route_fence_unavailable")
            })?;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).map_err(|_| {
                DesktopInputProviderError::new("desktop_input_route_fence_unavailable")
            })?;
            let digest = Sha256::digest(
                format!(
                    "{}\0{}\0{}",
                    identity.environment_id, identity.route_id, identity.display_allocation_id
                )
                .as_bytes(),
            );
            let path = directory.join(format!("{}.lock", hex::encode(digest)));
            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .mode(0o600)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .open(path)
                .map_err(|_| {
                    DesktopInputProviderError::new("desktop_input_route_fence_unavailable")
                })?;
            let started = Instant::now();
            loop {
                // SAFETY: `file` owns a valid descriptor for the duration of
                // the call. No pointer or borrowed memory crosses this seam.
                let result =
                    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
                if result == 0 {
                    return Ok(Self { file });
                }
                let error = std::io::Error::last_os_error();
                let would_block = error
                    .raw_os_error()
                    .is_some_and(|code| code == libc::EWOULDBLOCK || code == libc::EAGAIN);
                if !would_block {
                    return Err(DesktopInputProviderError::new(
                        "desktop_input_route_fence_unavailable",
                    ));
                }
                let elapsed = started.elapsed();
                if elapsed >= deadline {
                    return Err(DesktopInputProviderError::new(
                        "desktop_input_route_fence_contended",
                    ));
                }
                std::thread::sleep((deadline - elapsed).min(Duration::from_millis(5)));
            }
        }

        #[cfg(not(unix))]
        {
            let _ = (runtime_state_root, identity, deadline);
            Err(DesktopInputProviderError::new(
                "desktop_input_provider_unsupported",
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderEffectDescriptor<'a> {
    pub provider_generation: &'a str,
    pub event_kind: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EffectJournalDecision {
    Prepared,
    Acknowledged { acknowledgement_id: String },
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum PersistedEffectState {
    Prepared,
    Acknowledged { acknowledgement_id: String },
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedEffectRecord {
    provider_generation_sha256: String,
    event_kind: String,
    state: PersistedEffectState,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedEffectJournal {
    #[serde(default = "effect_journal_schema")]
    schema_version: String,
    #[serde(default)]
    records: BTreeMap<String, PersistedEffectRecord>,
}

impl Default for PersistedEffectJournal {
    fn default() -> Self {
        Self {
            schema_version: effect_journal_schema(),
            records: BTreeMap::new(),
        }
    }
}

fn effect_journal_schema() -> String {
    "agent-browser.desktop-input-effect-journal.v1".to_string()
}

#[derive(Debug)]
pub(crate) struct ProviderEffectJournal {
    path: std::path::PathBuf,
    journal: PersistedEffectJournal,
}

impl ProviderEffectJournal {
    pub(crate) fn open(
        runtime_state_root: &Path,
        identity: &RouteEffectFenceIdentity<'_>,
    ) -> Result<Self, DesktopInputProviderError> {
        if identity.environment_id.is_empty()
            || identity.route_id.is_empty()
            || identity.display_allocation_id.is_empty()
        {
            return Err(DesktopInputProviderError::new(
                "desktop_input_effect_journal_identity_invalid",
            ));
        }
        let directory = runtime_state_root.join("service/desktop-input/journals");
        fs::create_dir_all(&directory).map_err(|_| {
            DesktopInputProviderError::new("desktop_input_effect_journal_unavailable")
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).map_err(|_| {
                DesktopInputProviderError::new("desktop_input_effect_journal_unavailable")
            })?;
        }
        let identity_digest = digest_text(&format!(
            "{}\0{}\0{}",
            identity.environment_id, identity.route_id, identity.display_allocation_id
        ));
        let path = directory.join(format!("{identity_digest}.json"));
        let journal = match fs::read_to_string(&path) {
            Ok(serialized) => {
                let journal: PersistedEffectJournal =
                    serde_json::from_str(&serialized).map_err(|_| {
                        DesktopInputProviderError::new("desktop_input_effect_journal_invalid")
                    })?;
                if journal.schema_version != effect_journal_schema() {
                    return Err(DesktopInputProviderError::new(
                        "desktop_input_effect_journal_invalid",
                    ));
                }
                journal
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                PersistedEffectJournal::default()
            }
            Err(_) => {
                return Err(DesktopInputProviderError::new(
                    "desktop_input_effect_journal_unavailable",
                ))
            }
        };
        let mut opened = Self { path, journal };
        let mut recovered = false;
        for record in opened.journal.records.values_mut() {
            if matches!(record.state, PersistedEffectState::Prepared) {
                record.state = PersistedEffectState::Uncertain;
                recovered = true;
            }
        }
        if recovered {
            opened.save()?;
        }
        Ok(opened)
    }

    pub(crate) fn prepare(
        &mut self,
        effect_key: &str,
        descriptor: &ProviderEffectDescriptor<'_>,
    ) -> Result<EffectJournalDecision, DesktopInputProviderError> {
        let key = digest_text(effect_key);
        let provider_generation_sha256 = digest_text(descriptor.provider_generation);
        if let Some(record) = self.journal.records.get(&key) {
            if record.provider_generation_sha256 != provider_generation_sha256
                || record.event_kind != descriptor.event_kind
            {
                return Err(DesktopInputProviderError::new(
                    "desktop_input_effect_journal_conflict",
                ));
            }
            return Ok(match &record.state {
                PersistedEffectState::Prepared => EffectJournalDecision::Prepared,
                PersistedEffectState::Acknowledged { acknowledgement_id } => {
                    EffectJournalDecision::Acknowledged {
                        acknowledgement_id: acknowledgement_id.clone(),
                    }
                }
                PersistedEffectState::Uncertain => EffectJournalDecision::Uncertain,
            });
        }
        self.transition(|journal| {
            journal.records.insert(
                key,
                PersistedEffectRecord {
                    provider_generation_sha256,
                    event_kind: descriptor.event_kind.to_string(),
                    state: PersistedEffectState::Prepared,
                },
            );
            Ok(())
        })?;
        Ok(EffectJournalDecision::Prepared)
    }

    pub(crate) fn acknowledge(
        &mut self,
        effect_key: &str,
        acknowledgement_id: &str,
    ) -> Result<(), DesktopInputProviderError> {
        let key = digest_text(effect_key);
        self.transition(|journal| {
            let record = journal.records.get_mut(&key).ok_or_else(|| {
                DesktopInputProviderError::new("desktop_input_effect_journal_transition_invalid")
            })?;
            if !matches!(record.state, PersistedEffectState::Prepared) {
                return Err(DesktopInputProviderError::new(
                    "desktop_input_effect_journal_transition_invalid",
                ));
            }
            record.state = PersistedEffectState::Acknowledged {
                acknowledgement_id: acknowledgement_id.to_string(),
            };
            Ok(())
        })
    }

    fn transition(
        &mut self,
        mutation: impl FnOnce(&mut PersistedEffectJournal) -> Result<(), DesktopInputProviderError>,
    ) -> Result<(), DesktopInputProviderError> {
        let previous = serde_json::to_vec(&self.journal)
            .map_err(|_| DesktopInputProviderError::new("desktop_input_effect_journal_invalid"))?;
        mutation(&mut self.journal)?;
        if let Err(error) = self.save() {
            self.journal = serde_json::from_slice(&previous).map_err(|_| {
                DesktopInputProviderError::new("desktop_input_effect_journal_invalid")
            })?;
            return Err(error);
        }
        Ok(())
    }

    fn save(&self) -> Result<(), DesktopInputProviderError> {
        use std::io::Write;

        let parent = self.path.parent().ok_or_else(|| {
            DesktopInputProviderError::new("desktop_input_effect_journal_save_failed")
        })?;
        let serialized = serde_json::to_vec(&self.journal)
            .map_err(|_| DesktopInputProviderError::new("desktop_input_effect_journal_invalid"))?;
        let temporary = parent.join(format!(
            ".journal-{}-{}.tmp",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
        }
        let mut file = options.open(&temporary).map_err(|_| {
            DesktopInputProviderError::new("desktop_input_effect_journal_save_failed")
        })?;
        let result = (|| {
            file.write_all(&serialized)?;
            file.sync_all()?;
            fs::rename(&temporary, &self.path)?;
            #[cfg(unix)]
            File::open(parent)?.sync_all()?;
            Ok::<(), std::io::Error>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(DesktopInputProviderError::new(
                "desktop_input_effect_journal_save_failed",
            ));
        }
        Ok(())
    }
}

fn digest_text(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

#[cfg(unix)]
use std::os::fd::AsRawFd;

#[cfg(unix)]
impl Drop for RouteEffectFence {
    fn drop(&mut self) {
        // SAFETY: `self.file` remains valid until after `drop` returns. The
        // descriptor is not exposed outside this module.
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "agent-browser-desktop-provider-{name}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn same_route_contends_while_unrelated_route_progresses() {
        let root = test_root("route-fence");
        let route_a = RouteEffectFenceIdentity {
            environment_id: "development",
            route_id: "route-a",
            display_allocation_id: "display-a",
        };
        let route_b = RouteEffectFenceIdentity {
            environment_id: "development",
            route_id: "route-b",
            display_allocation_id: "display-b",
        };

        let held = RouteEffectFence::acquire(&root, &route_a, Duration::ZERO).unwrap();
        let contention = RouteEffectFence::acquire(&root, &route_a, Duration::ZERO).unwrap_err();
        assert_eq!(contention.code(), "desktop_input_route_fence_contended");
        let unrelated = RouteEffectFence::acquire(&root, &route_b, Duration::ZERO).unwrap();

        drop(unrelated);
        drop(held);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn acknowledged_effect_replays_without_new_emission_authority() {
        let root = test_root("effect-journal-acknowledged");
        let identity = RouteEffectFenceIdentity {
            environment_id: "development",
            route_id: "route-a",
            display_allocation_id: "display-a",
        };
        let descriptor = ProviderEffectDescriptor {
            provider_generation: "generation-a",
            event_kind: "left_down",
        };
        let mut journal = ProviderEffectJournal::open(&root, &identity).unwrap();
        assert_eq!(
            journal.prepare("effect-a", &descriptor).unwrap(),
            EffectJournalDecision::Prepared
        );
        journal
            .acknowledge("effect-a", "acknowledgement-a")
            .unwrap();
        drop(journal);

        let mut reloaded = ProviderEffectJournal::open(&root, &identity).unwrap();
        assert_eq!(
            reloaded.prepare("effect-a", &descriptor).unwrap(),
            EffectJournalDecision::Acknowledged {
                acknowledgement_id: "acknowledgement-a".to_string(),
            }
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn abandoned_prepared_effect_reopens_as_uncertain() {
        let root = test_root("effect-journal-abandoned");
        let identity = RouteEffectFenceIdentity {
            environment_id: "development",
            route_id: "route-a",
            display_allocation_id: "display-a",
        };
        let descriptor = ProviderEffectDescriptor {
            provider_generation: "generation-a",
            event_kind: "left_down",
        };
        let mut journal = ProviderEffectJournal::open(&root, &identity).unwrap();
        assert_eq!(
            journal.prepare("effect-a", &descriptor).unwrap(),
            EffectJournalDecision::Prepared
        );
        drop(journal);

        let mut reloaded = ProviderEffectJournal::open(&root, &identity).unwrap();
        assert_eq!(
            reloaded.prepare("effect-a", &descriptor).unwrap(),
            EffectJournalDecision::Uncertain
        );
        fs::remove_dir_all(root).unwrap();
    }
}
