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
    next_intent_id: fn() -> String,
    expected: Option<BrowserSessionState>,
    pending: Option<(BrowserLaunchIntent, bool)>,
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
            next_intent_id,
            expected: None,
            pending: None,
        })
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
        S: BrowserLaunchCustodyStore,
    > BrowserSessionEffects for RemoteViewSessionEffects<E, T, S>
{
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
