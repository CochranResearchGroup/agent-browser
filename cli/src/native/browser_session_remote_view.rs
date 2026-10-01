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
    local: LocalEffects,
    context: Option<RemoteViewRuntimeContext>,
) -> Result<RuntimeSessionEffects, String> {
    match context {
        None => Ok(RuntimeSessionEffects::Local(local)),
        Some(context) => Ok(RuntimeSessionEffects::Remote(Box::new(
            RemoteViewSessionEffects::new(
                local,
                context.adapter,
                BrowserSessionSqliteStore::default_sqlite()?,
                Vec::new(),
                || uuid::Uuid::new_v4().to_string(),
            )?
            .with_pool(context.pool)?,
        ))),
    }
}

impl BrowserSessionEffects for RuntimeSessionEffects {
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
