//! Runtime construction from the public Remote View application inventory.
use super::browser_session_runtime::{BrowserManagerRuntime, BrowserSessionEffectAdapter};
use super::browser_session_store::BrowserSessionSqliteStore;
use super::remote_view_application_http::RemoteViewApplicationHttp;
use agent_browser_service_model::*;
use std::time::Duration;

type ExistingPool = (
    Vec<RemoteViewAssignmentRecord>,
    Vec<RemoteViewDesktopCandidate>,
);

type LocalEffects = BrowserSessionEffectAdapter<BrowserManagerRuntime>;
type RemoteEffects =
    RemoteViewSessionEffects<LocalEffects, RemoteViewApplicationHttp, BrowserSessionSqliteStore>;

pub(crate) enum RuntimeSessionEffects {
    Local(LocalEffects),
    Remote(Box<RemoteEffects>),
}

pub(super) struct RemoteViewRuntimeContext {
    adapter: RemoteViewApplicationAdapter<RemoteViewApplicationHttp>,
    assignments: Vec<RemoteViewAssignmentRecord>,
    pub(super) desktops: Vec<RemoteViewDesktopCandidate>,
}

/// Explicit origin and pool are required together. Application defaults to
/// agent-browser. Configuration errors never echo supplied values.
pub(super) fn configured_remote_view() -> Result<Option<RemoteViewRuntimeContext>, String> {
    let origin = setting("AGENT_BROWSER_REMOTE_VIEW_ORIGIN")?;
    let pool = setting("AGENT_BROWSER_REMOTE_VIEW_POOL")?;
    let application = setting("AGENT_BROWSER_REMOTE_VIEW_APPLICATION")?;
    let (origin, pool) = match (origin, pool, application.as_ref()) {
        (None, None, None) => return Ok(None),
        (Some(origin), Some(pool), _) if !pool.is_empty() => (origin, pool),
        _ => return Err("remote_view_runtime_config_incomplete".into()),
    };
    let transport = RemoteViewApplicationHttp::new(&origin, Duration::from_secs(15))
        .map_err(|_| "remote_view_runtime_origin_invalid".to_string())?;
    let mut adapter = RemoteViewApplicationAdapter::new(
        application.unwrap_or_else(|| "agent-browser".into()),
        transport,
    )
    .map_err(|_| "remote_view_runtime_application_invalid".to_string())?;
    let (assignments, desktops) = existing_pool(&mut adapter, &pool)?;
    Ok(Some(RemoteViewRuntimeContext {
        adapter,
        assignments,
        desktops,
    }))
}

fn setting(name: &str) -> Result<Option<String>, String> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(_) => Err("remote_view_runtime_config_invalid".into()),
    }
}

/// Read current assignments only. Empty pools require durable acquisition by
/// the capacity owner; runtime setup never fabricates an assignment or display.
fn existing_pool<T: RemoteViewApplicationTransport>(
    adapter: &mut RemoteViewApplicationAdapter<T>,
    pool_name: &str,
) -> Result<ExistingPool, String> {
    let inventory = adapter
        .inventory()
        .map_err(|_| "remote_view_runtime_inventory_unavailable".to_string())?;
    let pool = inventory
        .pools
        .iter()
        .find(|pool| pool.name == pool_name)
        .ok_or_else(|| "remote_view_runtime_pool_missing".to_string())?;
    let mut assignments = Vec::new();
    let mut desktops = Vec::new();
    for assignment in inventory.assignments.into_iter().filter(|assignment| {
        assignment.pool_id == pool.pool_id && assignment.state == RemoteViewAssignmentState::Active
    }) {
        adapter
            .observe_assignment(&assignment)
            .map_err(|_| "remote_view_runtime_assignment_unavailable".to_string())?;
        desktops.push(RemoteViewDesktopCandidate {
            ready: true,
            desktop: RemoteViewFixedDesktop {
                desktop_id: assignment.desktop_id.clone(),
                generation: assignment.generation,
                friendly_route_label: assignment.desktop_id.clone(),
            },
        });
        assignments.push(assignment);
    }
    if assignments.is_empty() {
        return Err("remote_view_runtime_assignment_required".into());
    }
    Ok((assignments, desktops))
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
                context.assignments,
                || uuid::Uuid::new_v4().to_string(),
            )?,
        ))),
    }
}

impl BrowserSessionEffects for RuntimeSessionEffects {
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
    use serde_json::Value;

    struct InventoryTransport {
        inventory: Value,
        observation: Value,
        operations: Vec<&'static str>,
    }
    impl RemoteViewApplicationTransport for InventoryTransport {
        fn request(
            &mut self,
            envelope: &RemoteViewApplicationEnvelope,
        ) -> Result<Value, RemoteViewApplicationTransportError> {
            assert_eq!(envelope.application, "agent-browser");
            match envelope.request {
                RemoteViewApplicationRequest::Inventory {} => {
                    self.operations.push("inventory");
                    Ok(self.inventory.clone())
                }
                RemoteViewApplicationRequest::ObserveAssignment { .. } => {
                    self.operations.push("observe_assignment");
                    Ok(self.observation.clone())
                }
                _ => {
                    panic!("runtime setup must not mutate provider or request private environment")
                }
            }
        }
    }
    fn adapter(
        change: impl FnOnce(&mut Value),
    ) -> RemoteViewApplicationAdapter<InventoryTransport> {
        let mut fixture: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/remote-view-application-r3.v1.fixture.json"
        ))
        .unwrap();
        change(&mut fixture);
        RemoteViewApplicationAdapter::new(
            "agent-browser".into(),
            InventoryTransport {
                inventory: fixture["inventory"].clone(),
                observation: fixture["assignmentObservation"].clone(),
                operations: Vec::new(),
            },
        )
        .unwrap()
    }
    #[test]
    fn browser_session_runtime_pool_requires_current_exact_public_assignment() {
        let (assignments, candidates) = existing_pool(&mut adapter(|_| {}), "main").unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(candidates[0].desktop.desktop_id, assignments[0].desktop_id);
        assert_eq!(candidates[0].desktop.generation, assignments[0].generation);
        assert!(candidates[0].ready);
        assert_eq!(
            existing_pool(&mut adapter(|_| {}), "other").unwrap_err(),
            "remote_view_runtime_pool_missing"
        );
        assert_eq!(
            existing_pool(
                &mut adapter(|f| f["inventory"]["assignments"] = serde_json::json!([])),
                "main"
            )
            .unwrap_err(),
            "remote_view_runtime_assignment_required"
        );
        assert_eq!(
            existing_pool(
                &mut adapter(
                    |f| f["assignmentObservation"]["target"]["lifecycleGeneration"] = 8.into()
                ),
                "main"
            )
            .unwrap_err(),
            "remote_view_runtime_assignment_unavailable"
        );
        assert_eq!(
            existing_pool(
                &mut adapter(
                    |f| f["assignmentObservation"]["readinessScope"] = "provider_record".into()
                ),
                "main"
            )
            .unwrap_err(),
            "remote_view_runtime_assignment_unavailable"
        );
    }
}
