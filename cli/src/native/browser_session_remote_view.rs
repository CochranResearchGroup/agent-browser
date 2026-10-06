//! Runtime construction from the public Remote View application inventory.
use super::browser_session_runtime::{BrowserManagerRuntime, BrowserSessionEffectAdapter};
use super::browser_session_store::BrowserSessionSqliteStore;
use super::remote_view_application_http::RemoteViewApplicationHttp;
use agent_browser_service_model::*;
use std::time::Duration;

type LocalEffects = BrowserSessionEffectAdapter<BrowserManagerRuntime>;
type RemoteEffects =
    RemoteViewSessionEffects<LocalEffects, RemoteViewApplicationHttp, BrowserSessionSqliteStore>;

pub(crate) enum RuntimeSessionEffects {
    Local(LocalEffects),
    Remote(Box<RemoteEffects>),
}

pub(super) struct RemoteViewRuntimeContext {
    adapter: RemoteViewApplicationAdapter<RemoteViewApplicationHttp>,
    pool: RemoteViewSessionPool,
}

/// Detect an explicitly selected Remote View lane before loading the host.
/// Partial settings enter validation rather than falling back to local launch.
pub(crate) fn remote_view_settings_present() -> bool {
    [
        "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
        "AGENT_BROWSER_REMOTE_VIEW_POOL",
        "AGENT_BROWSER_REMOTE_VIEW_APPLICATION",
        "AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
}

/// Runtime setup validates explicit settings without reading or mutating the
/// provider. Capacity preparation belongs to a new browser's placement demand.
pub(super) fn configured_remote_view() -> Result<Option<RemoteViewRuntimeContext>, String> {
    runtime_context(
        setting("AGENT_BROWSER_REMOTE_VIEW_ORIGIN")?,
        setting("AGENT_BROWSER_REMOTE_VIEW_POOL")?,
        setting("AGENT_BROWSER_REMOTE_VIEW_APPLICATION")?,
        setting("AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT")?,
    )
}

fn runtime_context(
    origin: Option<String>,
    pool: Option<String>,
    application: Option<String>,
    count: Option<String>,
) -> Result<Option<RemoteViewRuntimeContext>, String> {
    let (origin, pool, application, count) = match (origin, pool, application, count) {
        (None, None, None, None) => return Ok(None),
        (Some(origin), Some(pool), application, count) if !pool.is_empty() => {
            (origin, pool, application, count)
        }
        _ => return Err("remote_view_runtime_config_incomplete".into()),
    };
    let desired_desktops = count
        .as_deref()
        .unwrap_or("1")
        .parse::<u32>()
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| "remote_view_runtime_desktop_count_invalid".to_string())?;
    let transport = RemoteViewApplicationHttp::new(&origin, Duration::from_secs(15))
        .map_err(|_| "remote_view_runtime_origin_invalid".to_string())?;
    let adapter = RemoteViewApplicationAdapter::new(
        application.unwrap_or_else(|| "agent-browser".into()),
        transport,
    )
    .map_err(|_| "remote_view_runtime_application_invalid".to_string())?;
    Ok(Some(RemoteViewRuntimeContext {
        adapter,
        pool: RemoteViewSessionPool {
            name: pool,
            desired_desktops,
        },
    }))
}

fn setting(name: &str) -> Result<Option<String>, String> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(_) => Err("remote_view_runtime_config_invalid".into()),
    }
}

pub(super) fn runtime_effects(
    mut local: LocalEffects,
    context: Option<RemoteViewRuntimeContext>,
) -> Result<RuntimeSessionEffects, String> {
    match context {
        None => Ok(RuntimeSessionEffects::Local(local)),
        Some(context) => {
            let mut store = BrowserSessionSqliteStore::default_sqlite()?;
            let pending = store
                .unpublished_launch_records()
                .map_err(|_| "remote_view_session_custody_unavailable")?;
            if !pending.is_empty() {
                let state = store.load_session_state()?;
                let catalog = store.load_profile_catalog()?.catalog;
                for claim in pending {
                    let browser = state
                        .browsers
                        .values()
                        .find(|browser| browser.profile_id == claim.intent.profile_id)
                        .ok_or("remote_view_session_launch_readback_required")?;
                    let profile = catalog
                        .profiles
                        .get(&browser.profile_id)
                        .or_else(|| {
                            state
                                .disposable_profiles
                                .get(&browser.profile_id)
                                .map(|p| &p.profile)
                        })
                        .ok_or("remote_view_session_launch_readback_required")?;
                    local
                        .prove_recovery_absence(browser, profile)
                        .map_err(|_| "remote_view_session_launch_readback_required")?;
                    store
                        .reconcile_absent_unobserved_recovery(&claim, &state, browser)
                        .map_err(|_| "remote_view_session_launch_readback_required")?;
                }
            }
            Ok(RuntimeSessionEffects::Remote(Box::new(
                RemoteViewSessionEffects::new(local, context.adapter, store, Vec::new(), || {
                    uuid::Uuid::new_v4().to_string()
                })?
                .with_pool(context.pool)?
                .with_view_clock(current_view_time_ms),
            )))
        }
    }
}

impl BrowserSessionEffects for RuntimeSessionEffects {
    fn recover_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        tabs: &[ManagedBrowserTab],
        navigation: &[BrowserNavigationRecord],
    ) -> Result<BrowserRecovery, String> {
        match self {
            Self::Local(effects) => effects.recover_browser(browser, profile, tabs, navigation),
            Self::Remote(effects) => effects.recover_browser(browser, profile, tabs, navigation),
        }
    }

    fn select_browser_executable(&mut self, path: String) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.select_browser_executable(path),
            Self::Remote(effects) => effects.select_browser_executable(path),
        }
    }

    fn resolve_remote_view_tab_view(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        view: &RemoteViewTabView,
        now_ms: u64,
    ) -> Result<RemoteViewApplicationViewIssuance, String> {
        match self {
            Self::Local(effects) => effects.resolve_remote_view_tab_view(target, view, now_ms),
            Self::Remote(effects) => effects.resolve_remote_view_tab_view(target, view, now_ms),
        }
    }
    fn observe_remote_view_tab_view(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        issuance: &RemoteViewApplicationViewIssuance,
        now_ms: u64,
    ) -> Result<RemoteViewApplicationViewObservation, String> {
        match self {
            Self::Local(effects) => effects.observe_remote_view_tab_view(target, issuance, now_ms),
            Self::Remote(effects) => effects.observe_remote_view_tab_view(target, issuance, now_ms),
        }
    }
    fn admits_ordinary_sessions(&self) -> bool {
        match self {
            Self::Local(effects) => effects.admits_ordinary_sessions(),
            Self::Remote(effects) => effects.admits_ordinary_sessions(),
        }
    }
    fn remote_view_desktop_candidates(
        &mut self,
    ) -> Result<Option<Vec<RemoteViewDesktopCandidate>>, String> {
        match self {
            Self::Local(effects) => effects.remote_view_desktop_candidates(),
            Self::Remote(effects) => effects.remote_view_desktop_candidates(),
        }
    }
    fn begin_operation(&mut self, expected: &BrowserSessionState) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.begin_operation(expected),
            Self::Remote(effects) => effects.begin_operation(expected),
        }
    }
    fn pending_launch_intent(&self) -> Option<BrowserLaunchIntent> {
        match self {
            Self::Local(effects) => effects.pending_launch_intent(),
            Self::Remote(effects) => effects.pending_launch_intent(),
        }
    }
    fn acknowledge_launch_publication(&mut self) {
        match self {
            Self::Local(effects) => effects.acknowledge_launch_publication(),
            Self::Remote(effects) => effects.acknowledge_launch_publication(),
        }
    }
    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String> {
        match self {
            Self::Local(effects) => effects.browser_is_live(browser),
            Self::Remote(effects) => effects.browser_is_live(browser),
        }
    }
    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        match self {
            Self::Local(effects) => effects.launch_browser(profile, desktop),
            Self::Remote(effects) => effects.launch_browser(profile, desktop),
        }
    }
    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.close_browser(browser),
            Self::Remote(effects) => effects.close_browser(browser),
        }
    }
    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        match self {
            Self::Local(effects) => effects.acquire_initial_tab(browser, attributed_target_ids),
            Self::Remote(effects) => effects.acquire_initial_tab(browser, attributed_target_ids),
        }
    }
    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        match self {
            Self::Local(effects) => effects.create_tab(browser),
            Self::Remote(effects) => effects.create_tab(browser),
        }
    }
    fn close_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.close_tab(browser, tab),
            Self::Remote(effects) => effects.close_tab(browser, tab),
        }
    }
    fn navigate(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.navigate(browser, tab, url),
            Self::Remote(effects) => effects.navigate(browser, tab, url),
        }
    }
    fn focus_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.focus_browser(browser, tab),
            Self::Remote(effects) => effects.focus_browser(browser, tab),
        }
    }
    fn allocate_disposable_profile(
        &mut self,
        policy: &BrowserDisposableProfilePolicy,
        allocation_id: &str,
        session_name: &str,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        match self {
            Self::Local(effects) => {
                effects.allocate_disposable_profile(policy, allocation_id, session_name)
            }
            Self::Remote(effects) => {
                effects.allocate_disposable_profile(policy, allocation_id, session_name)
            }
        }
    }
    fn delete_disposable_profile(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<(), String> {
        match self {
            Self::Local(effects) => effects.delete_disposable_profile(allocation),
            Self::Remote(effects) => effects.delete_disposable_profile(allocation),
        }
    }
    fn disposable_profile_size_bytes(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<u64, String> {
        match self {
            Self::Local(effects) => effects.disposable_profile_size_bytes(allocation),
            Self::Remote(effects) => effects.disposable_profile_size_bytes(allocation),
        }
    }
}
impl ManagedBrowserCommandEffects for RuntimeSessionEffects {
    fn execute_command(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        session_id: &str,
        session_name: &str,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match self {
            Self::Local(effects) => {
                effects.execute_command(browser, tab, session_id, session_name, command)
            }
            Self::Remote(effects) => {
                effects.execute_command(browser, tab, session_id, session_name, command)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_session_runtime_configuration_performs_no_provider_request() {
        assert!(runtime_context(None, None, None, None).unwrap().is_none());
        let origin = Some("http://127.0.0.1:1".into());
        let pool = Some("main".into());
        let context = runtime_context(origin.clone(), pool.clone(), None, None)
            .unwrap()
            .unwrap();
        assert_eq!(context.pool.desired_desktops, 1);
        assert_eq!(context.pool.name, "main");
        assert!(runtime_context(origin.clone(), None, None, None).is_err());
        assert!(runtime_context(None, pool.clone(), None, None).is_err());
        assert!(runtime_context(None, None, None, Some("1".into())).is_err());
        for count in ["0", "-1", "private-secret", "4294967296"] {
            let error = runtime_context(origin.clone(), pool.clone(), None, Some(count.into()))
                .err()
                .unwrap();
            assert_eq!(error, "remote_view_runtime_desktop_count_invalid");
        }
        assert!(runtime_context(Some("http://example.test".into()), pool, None, None).is_err());
    }
}

/// Resolve the owned desktop's provider route without issuing a viewer grant.
/// Remote View owns viewer authentication and lifetime; this read only joins
/// the retained handoff to its current external desktop assignment.
pub(crate) fn resolve_handoff_desktop_route(id: &str) -> Result<String, String> {
    let store = BrowserSessionSqliteStore::default_sqlite()?;
    let state = store.load_session_state()?;
    state
        .remote_view_tab_handoffs
        .get(id)
        .ok_or("remote_view_tab_handoff_missing")?
        .check_link_at(current_view_time_ms())?;
    let target = resolve_remote_view_tab_handoff(&state, id)?;
    let desktop = target
        .browser
        .desktop
        .as_ref()
        .ok_or("remote_view_handoff_desktop_missing")?;
    let mut context = configured_remote_view()?.ok_or("remote_view_runtime_config_missing")?;
    let inventory = context
        .adapter
        .inventory()
        .map_err(|_| "remote_view_runtime_inventory_unavailable")?;
    owned_desktop_route(&inventory, &desktop.desktop_id, desktop.generation)
}

fn owned_desktop_route(
    inventory: &RemoteViewApplicationInventory,
    desktop_id: &str,
    generation: u64,
) -> Result<String, String> {
    if !inventory.assignments.iter().any(|assignment| {
        assignment.desktop_id == desktop_id
            && assignment.generation == generation
            && assignment.state == RemoteViewAssignmentState::Active
    }) {
        return Err("remote_view_handoff_desktop_not_owned".into());
    }
    let desktop = inventory
        .desktops
        .iter()
        .find(|desktop| desktop.desktop_id == desktop_id)
        .ok_or("remote_view_handoff_desktop_missing")?;
    let lifecycle = desktop
        .lifecycle
        .as_ref()
        .ok_or("remote_view_handoff_desktop_unavailable")?;
    let viewing = desktop
        .viewing
        .as_ref()
        .ok_or("remote_view_handoff_desktop_unavailable")?;
    if lifecycle.generation != generation
        || lifecycle.state != RemoteViewResourceState::Ready
        || lifecycle.readiness_scope != RemoteViewApplicationReadinessScope::LiveResource
        || viewing.lifecycle_generation != generation
        || viewing.desktop_id.is_empty()
        || viewing.generation == 0
    {
        return Err("remote_view_handoff_desktop_unavailable".into());
    }
    Ok(viewing.public_route.clone())
}

#[cfg(test)]
mod desktop_handoff_tests {
    use super::*;

    #[test]
    fn desktop_handoff_requires_current_owned_assignment_and_viewing_generation() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        let mut inventory: RemoteViewApplicationInventory =
            serde_json::from_value(fixture["inventory"].clone()).unwrap();
        let id = inventory.desktops[0].desktop_id.clone();
        inventory.desktops[0].viewing = Some(RemoteViewApplicationDesktopViewing {
            lifecycle_generation: 7,
            desktop_id: "33333333-3333-4333-8333-333333333333".into(),
            generation: 4,
            public_route: "/1".into(),
        });
        assert_eq!(owned_desktop_route(&inventory, &id, 7).unwrap(), "/1");
        assert!(owned_desktop_route(&inventory, &id, 8).is_err());
        inventory.desktops[0]
            .viewing
            .as_mut()
            .unwrap()
            .lifecycle_generation = 8;
        assert!(owned_desktop_route(&inventory, &id, 7).is_err());
        inventory.desktops[0]
            .viewing
            .as_mut()
            .unwrap()
            .lifecycle_generation = 7;
        inventory.assignments.clear();
        assert!(owned_desktop_route(&inventory, &id, 7).is_err());
    }
}

fn current_view_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

pub(crate) fn public_origin(value: &str) -> Result<url::Url, String> {
    let origin = url::Url::parse(value).map_err(|_| "remote_view_public_origin_invalid")?;
    if origin.scheme() != "https"
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.query().is_some()
        || origin.fragment().is_some()
        || origin.path() != "/"
        || origin.port() == Some(0)
        || origin
            .host_str()
            .is_none_or(|host| host == "localhost" || host.ends_with(".localhost"))
        || match origin.host() {
            Some(url::Host::Ipv4(ip)) => ip.is_loopback() || ip.is_unspecified(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback() || ip.is_unspecified(),
            _ => false,
        }
    {
        return Err("remote_view_public_origin_invalid".into());
    }
    Ok(origin)
}
