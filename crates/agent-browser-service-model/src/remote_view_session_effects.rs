//! Ordinary session effects joined to durable Remote View process admission.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};

/// Composes existing browser effects with public consumer transport and process
/// custody. Assignments are transient provider context, not a reservation ledger.
/// Unresolved claims block construction and later operations until explicit
/// reconciliation. No automatic recovery or new intent bypass is provided.
pub struct RemoteViewSessionEffects<E, T, S> {
    effects: E,
    adapter: RemoteViewApplicationAdapter<T>,
    store: S,
    assignments: BTreeMap<String, RemoteViewAssignmentRecord>,
    pool: Option<RemoteViewSessionPool>,
    next_intent_id: fn() -> String,
    expected: Option<BrowserSessionState>,
    pending: Option<(BrowserLaunchIntent, bool)>,
    view_clock: Option<fn() -> u64>,
}

impl<E, T: RemoteViewApplicationTransport, S: BrowserLaunchCustodyStore>
    RemoteViewSessionEffects<E, T, S>
{
    pub fn new(
        effects: E,
        adapter: RemoteViewApplicationAdapter<T>,
        mut store: S,
        assignments: Vec<RemoteViewAssignmentRecord>,
        next_intent_id: fn() -> String,
    ) -> Result<Self, String> {
        Self::require_clear_custody(&mut store)?;
        let mut by_desktop = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for assignment in assignments {
            let qualification = BrowserLaunchIntent {
                intent_id: "11111111-1111-1111-1111-111111111111".into(),
                profile_id: "assignment-validation".into(),
                assignment: assignment.clone(),
            };
            if qualification.validate().is_err()
                || !ids.insert(assignment.assignment_id.clone())
                || by_desktop
                    .insert(assignment.desktop_id.clone(), assignment)
                    .is_some()
            {
                return Err("remote_view_session_assignment_invalid".into());
            }
        }
        Ok(Self {
            effects,
            adapter,
            store,
            assignments: by_desktop,
            pool: None,
            next_intent_id,
            expected: None,
            pending: None,
            view_clock: None,
        })
    }
    /// Configure demand-driven capacity. This performs no provider request.
    pub fn with_pool(mut self, pool: RemoteViewSessionPool) -> Result<Self, String> {
        if pool.name.is_empty() || pool.desired_desktops == 0 {
            return Err("remote_view_session_pool_invalid".into());
        }
        self.pool = Some(pool);
        Ok(self)
    }

    /// Supply a runtime clock sampled after provider transport returns. Tests
    /// without a runtime clock retain their explicit logical timestamp.
    pub fn with_view_clock(mut self, clock: fn() -> u64) -> Self {
        self.view_clock = Some(clock);
        self
    }

    fn require_clear_custody(store: &mut S) -> Result<(), String> {
        if !store
            .unpublished_launch_records()
            .map_err(|_| "remote_view_session_custody_unavailable".to_string())?
            .is_empty()
        {
            return Err("remote_view_session_launch_readback_required".into());
        }
        Ok(())
    }
    fn require_operation(&self) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("remote_view_session_launch_readback_required".into());
        }
        if self.expected.is_none() {
            return Err("remote_view_session_operation_baseline_missing".into());
        }
        Ok(())
    }
}

impl<
        E: BrowserSessionEffects + RemoteViewBrowserProcessEffects,
        T: RemoteViewApplicationTransport,
        S: BrowserLaunchCustodyStore + RemoteViewPoolRequestStore,
    > BrowserSessionEffects for RemoteViewSessionEffects<E, T, S>
{
    fn select_browser_executable(&mut self, path: String) -> Result<(), String> {
        self.effects.select_browser_executable(path)
    }
    fn resolve_remote_view_tab_view(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        view: &RemoteViewTabView,
        now_ms: u64,
    ) -> Result<RemoteViewApplicationViewIssuance, String> {
        self.require_operation()?;
        let expected = self
            .expected
            .as_ref()
            .ok_or("remote_view_session_operation_baseline_missing")?;
        if expected.browsers.get(&target.browser.id) != Some(&target.browser)
            || expected.tabs.get(&target.tab.id) != Some(&target.tab)
            || expected.sessions.get(&target.session.id) != Some(&target.session)
            || view.idempotency_key.is_empty()
        {
            return Err("remote_view_tab_view_identity_conflict".into());
        }
        if view.issuance.is_some() {
            return resolve_published_remote_view_tab_view(
                &mut self.adapter,
                &mut self.store,
                target,
                view,
                now_ms,
            );
        }
        let assignment = remote_view_tab_assignment(&mut self.adapter, &mut self.store, target)?;
        let clock = self.view_clock;
        self.adapter
            .issue_view(
                &assignment,
                RemoteViewApplicationViewOptions {
                    audience: "remote_view".into(),
                    capability: RemoteViewApplicationViewCapability::Control,
                    lifetime_seconds: 300,
                    idempotency_key: view.idempotency_key.clone(),
                },
                || clock.map_or(now_ms, |clock| clock()),
                &mut self.store,
            )
            .map_err(|_| "remote_view_tab_view_issuance_readback_required".into())
    }

    fn admits_ordinary_sessions(&self) -> bool {
        self.pool.is_some()
    }

    fn remote_view_desktop_candidates(
        &mut self,
    ) -> Result<Option<Vec<RemoteViewDesktopCandidate>>, String> {
        self.require_operation()?;
        let Some(pool) = &self.pool else {
            return Ok(None);
        };
        let prepared = prepare_remote_view_session_pool(&mut self.adapter, &mut self.store, pool)?;
        self.assignments = prepared
            .assignments
            .into_iter()
            .map(|assignment| (assignment.desktop_id.clone(), assignment))
            .collect();
        Ok(Some(prepared.desktops))
    }

    fn begin_operation(&mut self, expected: &BrowserSessionState) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("remote_view_session_launch_readback_required".into());
        }
        Self::require_clear_custody(&mut self.store)?;
        self.effects.begin_operation(expected)?;
        self.expected = Some(expected.clone());
        Ok(())
    }
    fn pending_launch_intent(&self) -> Option<BrowserLaunchIntent> {
        self.pending
            .as_ref()
            .filter(|(_, observed)| *observed)
            .map(|(intent, _)| intent.clone())
    }
    fn acknowledge_launch_publication(&mut self) {
        if self.pending.as_ref().is_some_and(|(_, observed)| !observed) {
            return;
        }
        self.pending = None;
        self.expected = None;
        self.effects.acknowledge_launch_publication();
    }
    fn launch_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        desktop: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserLaunch, String> {
        self.require_operation()?;
        let desktop = desktop.ok_or_else(|| "remote_view_session_desktop_required".to_string())?;
        let assignment = self
            .assignments
            .get(&desktop.desktop_id)
            .filter(|assignment| assignment.generation == desktop.generation)
            .ok_or_else(|| "remote_view_session_assignment_stale".to_string())?
            .clone();
        let intent = BrowserLaunchIntent {
            intent_id: (self.next_intent_id)(),
            profile_id: profile.id.clone(),
            assignment,
        };
        intent
            .validate()
            .map_err(|_| "remote_view_session_intent_invalid".to_string())?;
        self.pending = Some((intent.clone(), false));
        let mut launch = launch_remote_view_browser(
            &mut self.adapter,
            &mut self.store,
            &mut self.effects,
            &intent,
            self.expected.as_ref().expect("qualified baseline"),
            profile,
        )
        .map_err(|_| "remote_view_session_launch_readback_required".to_string())?;
        // Preserve the selected descriptive label for the ordinary manager's
        // full-value comparison. Coordinator custody already qualified UUID and
        // lifecycle generation; the label supplies no placement authority.
        launch.desktop = Some(desktop.clone());
        self.pending = Some((intent, true));
        Ok(launch)
    }
    fn browser_is_live(&mut self, browser: &ManagedBrowserInstance) -> Result<bool, String> {
        self.require_operation()?;
        self.effects.browser_is_live(browser)
    }
    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String> {
        self.require_operation()?;
        self.effects.close_browser(browser)
    }
    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.require_operation()?;
        self.effects
            .acquire_initial_tab(browser, attributed_target_ids)
    }
    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        self.require_operation()?;
        self.effects.create_tab(browser)
    }
    fn close_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<(), String> {
        self.require_operation()?;
        self.effects.close_tab(browser, tab)
    }
    fn navigate(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String> {
        self.require_operation()?;
        self.effects.navigate(browser, tab, url)
    }
    fn focus_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        self.require_operation()?;
        self.effects.focus_browser(browser, tab)
    }
    fn allocate_disposable_profile(
        &mut self,
        policy: &BrowserDisposableProfilePolicy,
        allocation_id: &str,
        session_name: &str,
    ) -> Result<BrowserProfileCatalogEntry, String> {
        self.require_operation()?;
        self.effects
            .allocate_disposable_profile(policy, allocation_id, session_name)
    }
    fn delete_disposable_profile(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<(), String> {
        self.require_operation()?;
        self.effects.delete_disposable_profile(allocation)
    }
    fn disposable_profile_size_bytes(
        &mut self,
        allocation: &ManagedDisposableProfile,
    ) -> Result<u64, String> {
        self.require_operation()?;
        self.effects.disposable_profile_size_bytes(allocation)
    }
}

impl<
        E: ManagedBrowserCommandEffects,
        T: RemoteViewApplicationTransport,
        S: BrowserLaunchCustodyStore,
    > ManagedBrowserCommandEffects for RemoteViewSessionEffects<E, T, S>
{
    fn execute_command(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        session_id: &str,
        session_name: &str,
        command: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.require_operation()?;
        self.effects
            .execute_command(browser, tab, session_id, session_name, command)
    }
}

fn remote_view_tab_assignment<T: RemoteViewApplicationTransport>(
    adapter: &mut RemoteViewApplicationAdapter<T>,
    store: &mut impl BrowserLaunchCustodyStore,
    target: &RemoteViewTabHandoffTarget,
) -> Result<RemoteViewAssignmentRecord, String> {
    let desktop = target
        .browser
        .desktop
        .as_ref()
        .ok_or("remote_view_tab_view_desktop_missing")?;
    let inventory = adapter
        .inventory()
        .map_err(|_| "remote_view_tab_view_inventory_unavailable")?;
    let matching = inventory
        .assignments
        .iter()
        .filter(|assignment| {
            assignment.desktop_id == desktop.desktop_id
                && assignment.generation == desktop.generation
        })
        .collect::<Vec<_>>();
    let assignment = match matching.as_slice() {
        [assignment] => (*assignment).clone(),
        _ => return Err("remote_view_tab_view_assignment_unavailable".into()),
    };
    let published = store
        .published_launch_assignment(&target.browser)
        .map_err(|_| "remote_view_tab_view_launch_custody_unavailable")?;
    if assignment != published {
        return Err("remote_view_tab_view_assignment_conflict".into());
    }
    Ok(assignment)
}

/// Refresh one already-published presentation without issuing or persisting a
/// new grant. Authenticated presentation endpoints use this read-only path.
pub fn resolve_published_remote_view_tab_view<T: RemoteViewApplicationTransport>(
    adapter: &mut RemoteViewApplicationAdapter<T>,
    store: &mut impl BrowserLaunchCustodyStore,
    target: &RemoteViewTabHandoffTarget,
    view: &RemoteViewTabView,
    now_ms: u64,
) -> Result<RemoteViewApplicationViewIssuance, String> {
    if view.issuance.is_none() {
        return Err("remote_view_tab_view_unpublished".into());
    }
    let assignment = remote_view_tab_assignment(adapter, store, target)?;
    if let Some(issuance) = &view.issuance {
        // The durable view binds the client request to this exact issuance.
        // Provider grant keys are registration-scoped opaque identities and
        // cannot be compared with the original client mutation key.
        let resolved = adapter
            .resolve_view(&assignment, &issuance.grant, || now_ms, false)
            .map_err(|_| "remote_view_tab_view_resolution_failed")?;
        issuance
            .validate_target(
                &resolved.request.target,
                &resolved.request.application,
                "remote_view",
                RemoteViewApplicationViewCapability::Control,
                300,
                now_ms,
            )
            .map_err(|_| "remote_view_tab_view_issuance_invalid")?;
        return Ok(issuance.clone());
    }
    Err("remote_view_tab_view_unpublished".into())
}
