//! Durable host for the ordinary Browser Session Manager path.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_browser_service_model::{
    BrowserDesktopRoute, BrowserDisposableProfilePolicy, BrowserProfileCatalog,
    BrowserSessionEffects, BrowserSessionManager, BrowserSessionManagerConfig, BrowserSessionState,
    CloseBrowserSessionResult, CloseBrowserTabResult, OpenBrowserSession, OpenBrowserSessionResult,
    ReapBrowserSessionsResult, SessionEndReason,
};

use super::browser_session_runtime::{
    BrowserManagerRuntime, BrowserManagerRuntimeConfig, BrowserSessionEffectAdapter,
};
use super::browser_session_store::{BrowserProfileCatalogLoad, BrowserSessionJsonStore};

const DEFAULT_SESSION_IDLE_TIMEOUT_MS: u64 = 300_000;
const DEFAULT_DISPOSABLE_CLEANUP_DELAY_MS: u64 = 300_000;
const DEFAULT_DISPOSABLE_POLICY_ID: &str = "default";

pub(crate) type DefaultBrowserSessionHost =
    BrowserSessionHost<BrowserSessionJsonStore, BrowserSessionEffectAdapter<BrowserManagerRuntime>>;

pub(crate) fn load_default_browser_session_host() -> Result<DefaultBrowserSessionHost, String> {
    let legacy_state_path = super::service_store::default_service_state_path()?;
    let store = BrowserSessionJsonStore::default_json()?;
    let session_idle_timeout_ms = configured_u64(
        "AGENT_BROWSER_SESSION_IDLE_TIMEOUT_MS",
        DEFAULT_SESSION_IDLE_TIMEOUT_MS,
    )?;
    let cleanup_delay_ms = configured_u64(
        "AGENT_BROWSER_DISPOSABLE_CLEANUP_DELAY_MS",
        DEFAULT_DISPOSABLE_CLEANUP_DELAY_MS,
    )?;
    let disposable_root = match std::env::var_os("AGENT_BROWSER_DISPOSABLE_PROFILE_ROOT") {
        Some(value) => PathBuf::from(value),
        None => legacy_state_path
            .parent()
            .ok_or_else(|| "browser_session_service_directory_missing".to_string())?
            .join("disposable-profiles"),
    };
    if !disposable_root.is_absolute() {
        return Err("browser_disposable_root_not_absolute".to_string());
    }
    let display = std::env::var("AGENT_BROWSER_SESSION_DISPLAY").ok();
    let remote_desktop_routes = if display.is_some() {
        Vec::new()
    } else {
        super::presentation_inventory::StaticRouteInventory::from_environment()?
            .routes()
            .iter()
            .filter_map(|route| {
                let display_name = route.display_name.as_ref()?;
                Some(BrowserDesktopRoute {
                    id: route.id.clone(),
                    display_name: display_name.clone(),
                    healthy: super::remote_view::route_display_socket_available(display_name),
                })
            })
            .collect()
    };
    let runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
        headless: display.is_none(),
        executable_path: std::env::var("AGENT_BROWSER_EXECUTABLE_PATH").ok(),
        display,
        remote_headed: false,
    })?;
    BrowserSessionHost::load(
        store,
        BrowserSessionEffectAdapter::new(runtime),
        &legacy_state_path,
        BrowserSessionHostConfig {
            session_idle_timeout_ms,
            remote_desktop_routes,
            default_disposable_policy: Some(BrowserDisposableProfilePolicy {
                id: DEFAULT_DISPOSABLE_POLICY_ID.to_string(),
                user_data_root: disposable_root.to_string_lossy().into_owned(),
                cleanup_delay_ms,
            }),
        },
    )
}

fn configured_u64(name: &str, default: u64) -> Result<u64, String> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|error| format!("browser_session_config_invalid:{name}:{error}")),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(format!("browser_session_config_invalid:{name}:{error}")),
    }
}

pub(crate) trait BrowserSessionPersistence {
    fn load_session_state(&self) -> Result<BrowserSessionState, String>;
    fn save_session_state(&self, state: &BrowserSessionState) -> Result<(), String>;
    fn load_or_import_profile_catalog(
        &self,
        legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String>;
    fn save_profile_catalog(&self, catalog: &BrowserProfileCatalog) -> Result<(), String>;
}

impl BrowserSessionPersistence for BrowserSessionJsonStore {
    fn load_session_state(&self) -> Result<BrowserSessionState, String> {
        BrowserSessionJsonStore::load_session_state(self)
    }

    fn save_session_state(&self, state: &BrowserSessionState) -> Result<(), String> {
        BrowserSessionJsonStore::save_session_state(self, state)
    }

    fn load_or_import_profile_catalog(
        &self,
        legacy_service_state_path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String> {
        BrowserSessionJsonStore::load_or_import_profile_catalog(self, legacy_service_state_path)
    }

    fn save_profile_catalog(&self, catalog: &BrowserProfileCatalog) -> Result<(), String> {
        BrowserSessionJsonStore::save_profile_catalog(self, catalog)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserSessionHostConfig {
    pub(crate) session_idle_timeout_ms: u64,
    pub(crate) remote_desktop_routes: Vec<BrowserDesktopRoute>,
    pub(crate) default_disposable_policy: Option<BrowserDisposableProfilePolicy>,
}

pub(crate) struct BrowserSessionHost<P, E> {
    persistence: P,
    effects: E,
    catalog: BrowserProfileCatalog,
    state: BrowserSessionState,
    manager_config: BrowserSessionManagerConfig,
}

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects> BrowserSessionHost<P, E> {
    pub(crate) fn load(
        persistence: P,
        effects: E,
        legacy_service_state_path: &Path,
        config: BrowserSessionHostConfig,
    ) -> Result<Self, String> {
        let state = persistence.load_session_state()?;
        let mut catalog_load =
            persistence.load_or_import_profile_catalog(legacy_service_state_path)?;
        let mut catalog_changed = false;
        if let Some(policy) = config.default_disposable_policy {
            if !catalog_load
                .catalog
                .disposable_policies
                .contains_key(&policy.id)
            {
                catalog_load
                    .catalog
                    .disposable_policies
                    .insert(policy.id.clone(), policy);
                catalog_changed = true;
            }
        }
        if catalog_changed {
            persistence.save_profile_catalog(&catalog_load.catalog)?;
        }
        Ok(Self {
            persistence,
            effects,
            catalog: catalog_load.catalog,
            state,
            manager_config: BrowserSessionManagerConfig {
                session_idle_timeout_ms: config.session_idle_timeout_ms,
                remote_desktop_routes: config.remote_desktop_routes,
            },
        })
    }

    pub(crate) fn open(
        &mut self,
        request: OpenBrowserSession,
    ) -> Result<OpenBrowserSessionResult, String> {
        let result = self.manager().open(request)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn tab_for_navigation(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserTabAcquisition, String> {
        let result = self
            .manager()
            .tab_for_navigation(session_id, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn record_navigation(
        &mut self,
        session_id: &str,
        url: &str,
        visited_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserNavigationRecord, String> {
        let result = self
            .manager()
            .record_navigation(session_id, url, visited_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn navigate(
        &mut self,
        session_id: &str,
        url: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserNavigationRecord, String> {
        let result = self.manager().navigate(session_id, url, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn new_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<agent_browser_service_model::BrowserTabAcquisition, String> {
        let result = self.manager().new_tab(session_id, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn close_current_tab(
        &mut self,
        session_id: &str,
        activity_at_ms: u64,
    ) -> Result<CloseBrowserTabResult, String> {
        let result = self
            .manager()
            .close_current_tab(session_id, activity_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn close_session(
        &mut self,
        session_id: &str,
        reason: SessionEndReason,
        ended_at_ms: u64,
    ) -> Result<CloseBrowserSessionResult, String> {
        let result = self
            .manager()
            .close_session(session_id, reason, ended_at_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn reap(&mut self, now_ms: u64) -> Result<ReapBrowserSessionsResult, String> {
        let result = self.manager().reap(now_ms)?;
        self.commit_state()?;
        Ok(result)
    }

    pub(crate) fn state(&self) -> &BrowserSessionState {
        &self.state
    }

    pub(crate) fn handle_command(&mut self, command: &serde_json::Value) -> serde_json::Value {
        let id = command
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let result = self.handle_command_result(command);
        match result {
            Ok(data) => serde_json::json!({ "id": id, "success": true, "data": data }),
            Err(error) => serde_json::json!({ "id": id, "success": false, "error": error }),
        }
    }

    fn handle_command_result(
        &mut self,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let action = command
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "browser_session_action_missing".to_string())?;
        let now_ms = command
            .get("activityAtMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(current_unix_ms);
        match action {
            "browser_session_open" => {
                let opened = self.open_request_from_command(command, now_ms)?;
                Ok(serde_json::json!({
                    "sessionId": opened.session_id,
                    "sessionName": opened.session_name,
                    "profileId": opened.profile_id,
                    "browserId": opened.browser_id,
                    "browserDisposition": format!("{:?}", opened.disposition).to_lowercase(),
                    "sessionDisposition": format!("{:?}", opened.session_disposition).to_lowercase(),
                }))
            }
            "browser_session_close" => {
                let session_id = self.session_id_from_command(command)?;
                let closed =
                    self.close_session(&session_id, SessionEndReason::ExplicitClose, now_ms)?;
                Ok(serde_json::json!({
                    "closed": true,
                    "sessionId": closed.session_id,
                    "browserId": closed.browser_id,
                    "disposition": session_close_disposition_name(closed.disposition),
                }))
            }
            "browser_session_navigate" => {
                let opened = if optional_string(command, "sessionId").is_none() {
                    Some(self.open_request_from_command(command, now_ms)?)
                } else {
                    None
                };
                let session_id = optional_string(command, "sessionId")
                    .map(str::to_string)
                    .or_else(|| opened.as_ref().map(|result| result.session_id.clone()))
                    .ok_or_else(|| "browser_session_field_missing:sessionId".to_string())?;
                let url = required_string(command, "url")?;
                let navigation = self.navigate(&session_id, url, now_ms)?;
                Ok(serde_json::json!({
                    "sessionId": session_id,
                    "profileId": navigation.profile_id,
                    "browserId": navigation.browser_id,
                    "tabId": navigation.tab_id,
                    "targetId": navigation.target_id,
                    "url": navigation.url,
                    "visitedAtMs": navigation.visited_at_ms,
                }))
            }
            "browser_session_tab_new" => {
                let session_id = self.session_id_from_command(command)?;
                let tab = self.new_tab(&session_id, now_ms)?;
                let navigation = optional_string(command, "url")
                    .map(|url| self.navigate(&session_id, url, now_ms))
                    .transpose()?;
                Ok(serde_json::json!({
                    "sessionId": session_id,
                    "tabId": tab.tab_id,
                    "targetId": tab.target_id,
                    "source": "explicit_new",
                    "url": navigation.map(|record| record.url),
                }))
            }
            "browser_session_tab_close" => {
                let session_id = self.session_id_from_command(command)?;
                let closed = self.close_current_tab(&session_id, now_ms)?;
                Ok(serde_json::json!({
                    "sessionId": session_id,
                    "closed": true,
                    "closedTabId": closed.closed_tab_id,
                    "currentTabId": closed.current_tab_id,
                }))
            }
            "browser_session_reap" => {
                let reaped = self.reap(now_ms)?;
                Ok(serde_json::json!({
                    "expiredSessionIds": reaped.expired_session_ids,
                    "closedBrowserIds": reaped.closed_browser_ids,
                    "deletedDisposableProfileIds": reaped.deleted_disposable_profile_ids,
                }))
            }
            "browser_session_status" => serde_json::to_value(self.state())
                .map_err(|error| format!("browser_session_status_serialize_failed:{error}")),
            _ => Err(format!("browser_session_action_unsupported:{action}")),
        }
    }

    fn manager(&mut self) -> BrowserSessionManager<'_, E> {
        BrowserSessionManager::new(
            &mut self.state,
            &self.catalog,
            &mut self.effects,
            self.manager_config.clone(),
        )
    }

    fn commit_state(&self) -> Result<(), String> {
        self.persistence.save_session_state(&self.state)
    }

    fn open_request_from_command(
        &mut self,
        command: &serde_json::Value,
        activity_at_ms: u64,
    ) -> Result<OpenBrowserSessionResult, String> {
        let session_name = required_string(command, "sessionName")?;
        let request = if let Some(profile_id) = optional_string(command, "profileId") {
            OpenBrowserSession::exact_profile(session_name, profile_id, activity_at_ms)
        } else {
            OpenBrowserSession::disposable(
                session_name,
                optional_string(command, "disposablePolicyId")
                    .unwrap_or(DEFAULT_DISPOSABLE_POLICY_ID),
                activity_at_ms,
            )
        };
        self.open(request)
    }

    fn session_id_from_command(&self, command: &serde_json::Value) -> Result<String, String> {
        if let Some(session_id) = optional_string(command, "sessionId") {
            return Ok(session_id.to_string());
        }
        let session_name = required_string(command, "sessionName")?;
        let profile_id = optional_string(command, "profileId");
        let matching_ids: Vec<&str> = self
            .state
            .sessions
            .values()
            .filter(|session| {
                session.name == session_name
                    && profile_id.is_none_or(|profile_id| session.profile_id == profile_id)
            })
            .map(|session| session.id.as_str())
            .collect();
        match matching_ids.as_slice() {
            [session_id] => Ok((*session_id).to_string()),
            [] => Err(format!("browser_session_not_found_by_name:{session_name}")),
            _ => Err(format!("browser_session_name_ambiguous:{session_name}")),
        }
    }
}

fn session_close_disposition_name(
    disposition: agent_browser_service_model::SessionCloseDisposition,
) -> &'static str {
    match disposition {
        agent_browser_service_model::SessionCloseDisposition::BrowserPreserved => {
            "browser_preserved"
        }
        agent_browser_service_model::SessionCloseDisposition::BrowserClosed => "browser_closed",
    }
}

fn required_string<'a>(command: &'a serde_json::Value, field: &str) -> Result<&'a str, String> {
    optional_string(command, field).ok_or_else(|| format!("browser_session_field_missing:{field}"))
}

fn optional_string<'a>(command: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    command
        .get(field)
        .or_else(|| command.get("params").and_then(|params| params.get(field)))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::browser_session_runtime::{
        BrowserRuntimeDriver, BrowserSessionEffectAdapter,
    };
    use agent_browser_service_model::{
        BrowserLaunch, BrowserProfileCatalogEntry, BrowserTabAcquisition, ManagedBrowserInstance,
        ManagedBrowserTab,
    };
    use std::fs;
    use std::path::PathBuf;

    #[derive(Default)]
    struct FixtureRuntime {
        launches: usize,
        live: bool,
        next_tab: usize,
    }

    impl BrowserRuntimeDriver for FixtureRuntime {
        fn browser_is_live(&mut self, _browser: &ManagedBrowserInstance) -> Result<bool, String> {
            Ok(self.live)
        }

        fn launch_browser(
            &mut self,
            profile: &BrowserProfileCatalogEntry,
            desktop: Option<&agent_browser_service_model::BrowserDesktopAssignment>,
        ) -> Result<BrowserLaunch, String> {
            self.launches += 1;
            Ok(BrowserLaunch {
                browser_id: format!("browser:{}:{}", profile.id, self.launches),
                pid: 4242,
                cdp_endpoint: "ws://127.0.0.1:9422/devtools/browser/test".to_string(),
                desktop: desktop.cloned(),
            })
        }

        fn close_browser(&mut self, _browser: &ManagedBrowserInstance) -> Result<(), String> {
            Ok(())
        }

        fn acquire_initial_tab(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _attributed_target_ids: &[String],
        ) -> Result<BrowserTabAcquisition, String> {
            Err("unused".to_string())
        }

        fn create_tab(
            &mut self,
            _browser: &ManagedBrowserInstance,
        ) -> Result<BrowserTabAcquisition, String> {
            self.next_tab += 1;
            Ok(BrowserTabAcquisition {
                tab_id: format!("tab-{}", self.next_tab),
                target_id: format!("target-{}", self.next_tab),
                source: agent_browser_service_model::BrowserTabSource::ExplicitNew,
            })
        }

        fn close_tab(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: &ManagedBrowserTab,
        ) -> Result<(), String> {
            Ok(())
        }

        fn navigate(
            &mut self,
            _browser: &ManagedBrowserInstance,
            _tab: &ManagedBrowserTab,
            _url: &str,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agent-browser-session-host-{}-{}",
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
    fn restart_loads_independent_state_and_reuses_healthy_session() {
        let directory = TempDirectory::new();
        let legacy_path = directory.0.join("state.json");
        fs::write(
            &legacy_path,
            serde_json::json!({
                "profiles": {
                    "work": {
                        "id": "work",
                        "name": "Work",
                        "userDataDir": directory.0.join("work"),
                        "profileClass": "durable_named"
                    }
                },
                "sessions": "ignored legacy contradiction"
            })
            .to_string(),
        )
        .unwrap();
        let config = BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_desktop_routes: Vec::new(),
            default_disposable_policy: None,
        };
        let first = {
            let store = BrowserSessionJsonStore::new(&directory.0);
            let effects = BrowserSessionEffectAdapter::new(FixtureRuntime::default());
            let mut host =
                BrowserSessionHost::load(store, effects, &legacy_path, config.clone()).unwrap();
            host.open(OpenBrowserSession::exact_profile("alice", "work", 1_000))
                .unwrap()
        };

        let store = BrowserSessionJsonStore::new(&directory.0);
        let effects = BrowserSessionEffectAdapter::new(FixtureRuntime {
            launches: 0,
            live: true,
            next_tab: 0,
        });
        let mut restarted = BrowserSessionHost::load(store, effects, &legacy_path, config).unwrap();
        let resumed = restarted
            .open(OpenBrowserSession::exact_profile("alice", "work", 2_000))
            .unwrap();

        assert_eq!(resumed.session_id, first.session_id);
        assert_eq!(resumed.browser_id, first.browser_id);
        assert_eq!(restarted.state().sessions.len(), 1);

        let new_tab = restarted.handle_command(&serde_json::json!({
            "id": "new-alice-tab",
            "action": "browser_session_tab_new",
            "sessionName": "alice",
            "profileId": "work",
            "url": "https://example.test/next",
            "activityAtMs": 2_500
        }));
        assert_eq!(new_tab["success"], true);
        assert_eq!(new_tab["data"]["source"], "explicit_new");
        assert_eq!(new_tab["data"]["url"], "https://example.test/next");
        assert_eq!(restarted.state().tabs.len(), 1);
        assert_eq!(restarted.state().navigation_history.len(), 1);

        let close_tab = restarted.handle_command(&serde_json::json!({
            "id": "close-alice-tab",
            "action": "browser_session_tab_close",
            "sessionName": "alice",
            "profileId": "work",
            "activityAtMs": 2_750
        }));
        assert_eq!(close_tab["success"], true);
        assert_eq!(close_tab["data"]["closedTabId"], "tab-1");
        assert!(restarted.state().tabs.is_empty());

        let response = restarted.handle_command(&serde_json::json!({
            "id": "close-alice",
            "action": "browser_session_close",
            "sessionName": "alice",
            "profileId": "work",
            "activityAtMs": 3_000
        }));
        assert_eq!(response["success"], true);
        assert_eq!(response["data"]["sessionId"], first.session_id);
        assert_eq!(response["data"]["disposition"], "browser_closed");
        assert!(restarted.state().sessions.is_empty());
    }
}
