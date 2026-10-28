//! Ordinary session effects joined to durable Remote View process admission.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Weak};

struct IdleRetirementGuard {
    _local: Box<dyn BrowserIdleRetirementGuard>,
    _scope: Arc<()>,
}
impl BrowserIdleRetirementGuard for IdleRetirementGuard {}

/// Composes existing browser effects with public consumer transport and process
/// custody. Assignments are transient provider context, not a reservation ledger.
/// Current browser observations qualify ordinary reuse independently of launch
/// history. Interrupted observed publication joins the ordinary atomic commit.
/// Durable admission still deduplicates new process effects.
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
    retirements: BTreeMap<String, BrowserIdleRetirementRecord>,
    active_retirements: BTreeMap<String, Weak<()>>,
    operation_retirements: BTreeSet<String>,
    activities: BTreeSet<String>,
}

impl<
        E,
        T: RemoteViewApplicationTransport,
        S: BrowserLaunchCustodyStore + BrowserIdleRetirementCustodyStore,
    > RemoteViewSessionEffects<E, T, S>
{
    pub fn new(
        effects: E,
        adapter: RemoteViewApplicationAdapter<T>,
        store: S,
        assignments: Vec<RemoteViewAssignmentRecord>,
        next_intent_id: fn() -> String,
    ) -> Result<Self, String> {
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
            retirements: BTreeMap::new(),
            active_retirements: BTreeMap::new(),
            operation_retirements: BTreeSet::new(),
            activities: BTreeSet::new(),
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

    fn with_browser_activity<R>(
        &mut self,
        browser: &ManagedBrowserInstance,
        effect: impl FnOnce(&mut E) -> Result<R, String>,
    ) -> Result<R, String> {
        self.require_clear_profile_custody(&browser.profile_id)?;
        // Unpublished new roots already have exclusive launch custody. Their
        // initial target work is published atomically with that launch.
        if self
            .pending
            .as_ref()
            .is_some_and(|(intent, _)| intent.profile_id == browser.profile_id)
        {
            return effect(&mut self.effects);
        }
        let expected = self
            .expected
            .as_ref()
            .ok_or("browser_session_activity_unavailable")?;
        let id = (self.next_intent_id)();
        self.store
            .admit_browser_activity(&id, browser, expected)
            .map_err(|_| "browser_session_activity_admission_conflict")?;
        // Unknown effects retain the durable veto and are never replayed here.
        let result = effect(&mut self.effects)?;
        self.store
            .acknowledge_browser_activity(&id)
            .map_err(|_| "browser_session_activity_readback_required")?;
        self.activities.insert(id);
        Ok(result)
    }

    fn require_clear_profile_custody(&mut self, profile_id: &str) -> Result<(), String> {
        if self
            .store
            .idle_retirement_records()
            .map_err(|_| "browser_session_idle_custody_unavailable")?
            .iter()
            .any(|record| {
                !record.is_terminal()
                    && record
                        .intent
                        .scopes
                        .iter()
                        .any(|scope| scope.browser.profile_id == profile_id)
            })
        {
            return Err("browser_session_idle_readback_required".into());
        }
        // Historical launch observations do not veto an ordinary operation on
        // a currently verified live browser. New process effects still use
        // durable launch admission; current retirement fences remain above.
        Ok(())
    }
    fn update_retirement(
        &mut self,
        previous: &BrowserIdleRetirementRecord,
        next: BrowserIdleRetirementRecord,
    ) -> Result<(), String> {
        self.store
            .update_idle_retirement(previous, &next)
            .map_err(|_| "browser_session_idle_custody_unavailable")?;
        self.retirements.insert(next.intent.id.clone(), next);
        Ok(())
    }

    fn qualify_idle_viewers(
        &mut self,
        browsers: &[ManagedBrowserInstance],
        acquire_guard: impl FnOnce(
            &mut E,
        )
            -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String>,
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        if self.pool.is_none() || browsers.is_empty() {
            return acquire_guard(&mut self.effects);
        }
        let expected = self
            .expected
            .clone()
            .ok_or("browser_session_idle_custody_unavailable")?;
        let mut groups = BTreeMap::new();
        for browser in browsers {
            if self.borrows_browser_without_native_desktop(browser) {
                continue;
            }
            let baseline = expected
                .browsers
                .get(&browser.id)
                .ok_or("browser_session_idle_custody_unavailable")?;
            if baseline.pid != browser.pid
                || baseline.cdp_endpoint != browser.cdp_endpoint
                || baseline.profile_id != browser.profile_id
                || baseline.desktop != browser.desktop
            {
                return Err("browser_session_idle_custody_identity_changed".into());
            }
            let assignment = self
                .store
                .published_launch_assignment(baseline)
                .map_err(|_| "browser_session_idle_custody_unavailable")?;
            groups.insert(assignment.assignment_id.clone(), assignment);
        }
        // Qualify all new groups before any claim or provider mutation. A
        // deterministic legacy deferral must not leave an earlier group fenced.
        // Existing operations retain their exact immutable readback obligation.
        let mut original_identities = BTreeMap::new();
        for assignment in groups.values() {
            if self.retirements.values().any(|record| {
                !record.is_terminal() && record.intent.provider.assignment == *assignment
            }) {
                continue;
            }
            for browser in expected.browsers.values().filter(|browser| {
                browser.desktop.as_ref().is_some_and(|desktop| {
                    desktop.desktop_id == assignment.desktop_id
                        && desktop.generation == assignment.generation
                })
            }) {
                let Some(identity) = self
                    .store
                    .published_launch_process_identity(browser)
                    .map_err(|_| "browser_session_idle_custody_unavailable")?
                else {
                    return Ok(None);
                };
                original_identities.insert(browser.id.clone(), identity);
            }
        }
        // Provider reads can wait on network or its own desktop census. They
        // grant no effect authority and must not exclude foreground Service
        // State requests. Recheck current local authority after those reads;
        // native prepare still CAS-checks the captured viewer/revision fence.
        let mut observations = BTreeMap::new();
        for assignment in groups.values() {
            if self.retirements.values().any(|record| {
                !record.is_terminal() && record.intent.provider.assignment == *assignment
            }) {
                continue;
            }
            let idle = self
                .adapter
                .observe_idle(assignment)
                .map_err(|_| "browser_session_idle_viewer_observation_unavailable")?;
            if idle.viewer_active {
                return Ok(None);
            }
            let admission = self
                .adapter
                .read_idle_retirement(assignment)
                .map_err(|_| "browser_session_idle_readback_required")?;
            observations.insert(assignment.assignment_id.clone(), (idle, admission));
        }
        let Some(guard) = acquire_guard(&mut self.effects)? else {
            return Ok(None);
        };
        for assignment in groups.into_values() {
            let existing = self
                .retirements
                .values()
                .find(|record| {
                    !record.is_terminal() && record.intent.provider.assignment == assignment
                })
                .cloned();
            let mut record = if let Some(record) = existing {
                if record.published_state_digest.is_some() {
                    return Err("browser_session_idle_readback_required".into());
                }
                record
            } else {
                let (idle, admission) = observations
                    .remove(&assignment.assignment_id)
                    .ok_or("browser_session_idle_readback_required")?;
                let id = (self.next_intent_id)();
                let provider =
                    RemoteViewIdleRetirementIntent::new(&assignment, &idle, &admission, &id)
                        .map_err(|_| "browser_session_idle_readback_required")?;
                // Freeze every published peer on this assignment once. Sequential
                // tab/session closes reuse one native operation and slot revision.
                let ids = expected
                    .browsers
                    .values()
                    .filter(|browser| {
                        browser.desktop.as_ref().is_some_and(|desktop| {
                            desktop.desktop_id == assignment.desktop_id
                                && desktop.generation == assignment.generation
                        })
                    })
                    .map(|browser| browser.id.clone())
                    .collect::<Vec<_>>();
                let mut intent = BrowserIdleRetirementIntent::new(&id, provider, &expected, &ids)
                    .map_err(|_| "browser_session_idle_custody_unavailable")?;
                for scope in &mut intent.scopes {
                    scope.process_identity = Some(
                        original_identities
                            .get(&scope.browser.id)
                            .cloned()
                            .ok_or("browser_session_idle_custody_unavailable")?,
                    );
                }
                match self.store.claim_idle_retirement(&intent, &expected) {
                    Ok(BrowserIdleRetirementAdmission::New) => {
                        BrowserIdleRetirementRecord::new(intent)
                            .map_err(|_| "browser_session_idle_custody_unavailable")?
                    }
                    Ok(BrowserIdleRetirementAdmission::Existing(record)) => *record,
                    Err(BrowserIdleRetirementStoreError::ActivityActive) => return Ok(None),
                    Err(_) => return Err("browser_session_idle_readback_required".into()),
                }
            };
            self.operation_retirements.insert(record.intent.id.clone());
            let (current, new_preparation) = self
                .store
                .admit_idle_retirement_preparation(&record.intent)
                .map_err(|_| "browser_session_idle_readback_required")?;
            record = current;
            self.retirements
                .insert(record.intent.id.clone(), record.clone());
            if new_preparation {
                match self
                    .adapter
                    .prepare_idle_retirement(&record.intent.provider)
                {
                    Ok(prepared) => {
                        let previous = record.clone();
                        record
                            .record_prepared(prepared)
                            .map_err(|_| "browser_session_idle_readback_required")?;
                        self.update_retirement(&previous, record.clone())?;
                    }
                    Err(RemoteViewApplicationAdapterError::Transport(
                        RemoteViewApplicationTransportError::IdleRetirementDeferred,
                    )) => {
                        let previous = record.clone();
                        record
                            .defer_preparation()
                            .map_err(|_| "browser_session_idle_readback_required")?;
                        self.update_retirement(&previous, record)?;
                        return Ok(None);
                    }
                    Err(_) => return Err("browser_session_idle_readback_required".into()),
                }
            } else {
                let prepared = self
                    .adapter
                    .reconcile_idle_retirement(&record.intent.provider, record.prepared.as_ref())
                    .map_err(|_| "browser_session_idle_readback_required")?;
                if prepared.state != RemoteViewIdleRetirementReceiptState::Prepared {
                    return Err("browser_session_idle_readback_required".into());
                }
                if record.prepared.is_none() {
                    let previous = record.clone();
                    record
                        .record_prepared(prepared)
                        .map_err(|_| "browser_session_idle_readback_required")?;
                    self.update_retirement(&previous, record.clone())?;
                }
            }
            self.retirements.insert(record.intent.id.clone(), record);
        }
        let scope = Arc::new(());
        for browser in browsers {
            self.active_retirements
                .insert(browser.id.clone(), Arc::downgrade(&scope));
        }
        Ok(Some(Box::new(IdleRetirementGuard {
            _local: guard,
            _scope: scope,
        })))
    }

    /// Borrowed task tabs do not grant ownership of their browser process or
    /// a Native assignment. Their retirement still requires local activity
    /// exclusion, and the manager preserves the shared browser and peer tabs.
    fn borrows_browser_without_native_desktop(&self, browser: &ManagedBrowserInstance) -> bool {
        browser.desktop.is_none()
            && self.expected.as_ref().is_some_and(|expected| {
                expected.shared_browser_ids.contains(&browser.id)
                    && expected.browsers.get(&browser.id) == Some(browser)
            })
    }

    fn begin_retirement_effect(
        &mut self,
        browser: &ManagedBrowserInstance,
        effect: BrowserIdleRetirementEffect,
    ) -> Result<(Option<BrowserIdleRetirementRecord>, bool), String>
    where
        E: BrowserSessionEffects,
    {
        let Some(record) = self
            .retirements
            .values()
            .find(|record| {
                !record.is_terminal()
                    && record
                        .intent
                        .scopes
                        .iter()
                        .any(|scope| scope.browser.id == browser.id)
            })
            .cloned()
        else {
            return Ok((None, true));
        };
        if self
            .active_retirements
            .get(&browser.id)
            .is_none_or(|guard| guard.upgrade().is_none())
        {
            return Err("browser_session_idle_readback_required".into());
        }
        let scope = record
            .intent
            .scopes
            .iter()
            .find(|scope| scope.browser.id == browser.id)
            .unwrap();
        if scope.browser.pid != browser.pid
            || scope.browser.cdp_endpoint != browser.cdp_endpoint
            || scope.browser.profile_id != browser.profile_id
            || scope.browser.desktop != browser.desktop
        {
            return Err("browser_session_idle_custody_identity_changed".into());
        }
        let (current, admission) = self
            .store
            .admit_idle_retirement_effect(&record.intent, effect.clone())
            .map_err(|_| "browser_session_idle_readback_required")?;
        self.retirements
            .insert(current.intent.id.clone(), current.clone());
        match admission {
            BrowserIdleRetirementEffectAdmission::New => Ok((Some(current), true)),
            BrowserIdleRetirementEffectAdmission::AlreadySettled => Ok((None, false)),
            BrowserIdleRetirementEffectAdmission::PendingReadback => {
                let tab = match &effect {
                    BrowserIdleRetirementEffect::CloseTab { tab_id, .. } => {
                        scope.tabs.iter().find(|tab| tab.id == *tab_id)
                    }
                    BrowserIdleRetirementEffect::CloseBrowser { .. } => None,
                };
                if !self.effects.observe_idle_retirement_effect(scope, tab)? {
                    return Err("browser_session_idle_readback_required".into());
                }
                self.settle_retirement_effect(current, &effect)?;
                Ok((None, false))
            }
        }
    }

    fn settle_retirement_effect(
        &mut self,
        record: BrowserIdleRetirementRecord,
        effect: &BrowserIdleRetirementEffect,
    ) -> Result<(), String> {
        let mut next = record.clone();
        next.settle_effect(effect)
            .map_err(|_| "browser_session_idle_readback_required")?;
        self.update_retirement(&record, next)
    }

    fn require_operation(&self) -> Result<(), String> {
        if self.expected.is_none() {
            return Err("remote_view_session_operation_baseline_missing".into());
        }
        Ok(())
    }
}

impl<
        E: BrowserSessionEffects + RemoteViewBrowserProcessEffects,
        T: RemoteViewApplicationTransport,
        S: BrowserLaunchCustodyStore + RemoteViewPoolRequestStore + BrowserIdleRetirementCustodyStore,
    > BrowserSessionEffects for RemoteViewSessionEffects<E, T, S>
{
    fn attach_app_browser(
        &mut self,
        request: &BrowserAttachmentRequest,
        profile: &BrowserProfileCatalogEntry,
        desktop: &RemoteViewFixedDesktop,
    ) -> Result<BrowserAttachmentObservation, String> {
        self.require_operation()?;
        let assignment = self
            .assignments
            .get(&desktop.desktop_id)
            .filter(|row| row.generation == desktop.generation)
            .cloned()
            .ok_or("browser_attachment_desktop_observation_unavailable")?;
        self.adapter
            .observe_assignment(&assignment)
            .map_err(|_| "browser_attachment_desktop_stale")?;
        let matches = self
            .adapter
            .windows(&assignment)
            .map_err(|_| "browser_attachment_window_observation_unavailable")?
            .windows
            .iter()
            .any(|window| {
                window.pid == Some(request.process.pid) && window.width > 0 && window.height > 0
            });
        if !matches {
            return Err("browser_attachment_desktop_process_mismatch".into());
        }
        let mut observed = self.effects.attach_app_browser(request, profile, desktop)?;
        self.adapter
            .observe_assignment(&assignment)
            .map_err(|_| "browser_attachment_desktop_changed")?;
        if !self
            .adapter
            .windows(&assignment)
            .map_err(|_| "browser_attachment_window_observation_unavailable")?
            .windows
            .iter()
            .any(|window| {
                window.pid == Some(request.process.pid) && window.width > 0 && window.height > 0
            })
        {
            return Err("browser_attachment_desktop_process_changed".into());
        }
        observed.assignment = Some(assignment);
        Ok(observed)
    }
    fn native_handoff_assignment_target(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
    ) -> Result<RemoteViewApplicationTarget, String> {
        self.require_operation()?;
        let expected = self
            .expected
            .as_ref()
            .ok_or("remote_view_session_operation_baseline_missing")?;
        if expected.browsers.get(&target.browser.id) != Some(&target.browser)
            || expected.tabs.get(&target.tab.id) != Some(&target.tab)
            || expected.sessions.get(&target.session.id) != Some(&target.session)
        {
            return Err("remote_view_native_target_changed".into());
        }
        let assignment = remote_view_tab_assignment(&mut self.adapter, &mut self.store, target)?;
        self.adapter
            .observe_assignment(&assignment)
            .map(|observed| observed.target)
            .map_err(|_| "remote_view_native_target_unavailable".into())
    }
    fn register_native_handoff_observation(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        provider_target: &RemoteViewApplicationTarget,
        target_digest: &str,
        idempotency_key: &str,
    ) -> Result<RemoteViewNativeHandoffObservation, String> {
        self.require_operation()?;
        let expected = self
            .expected
            .as_ref()
            .ok_or("remote_view_session_operation_baseline_missing")?;
        if expected.browsers.get(&target.browser.id) != Some(&target.browser)
            || expected.tabs.get(&target.tab.id) != Some(&target.tab)
            || expected.sessions.get(&target.session.id) != Some(&target.session)
        {
            return Err("remote_view_native_target_changed".into());
        }
        let assignment = remote_view_tab_assignment(&mut self.adapter, &mut self.store, target)?;
        if self
            .adapter
            .observe_assignment(&assignment)
            .map_err(|_| "remote_view_native_target_unavailable")?
            .target
            != *provider_target
        {
            return Err("remote_view_native_target_changed".into());
        }
        let observation = self
            .adapter
            .register_native_handoff_observation(
                &assignment,
                target_digest.into(),
                idempotency_key.into(),
                300_000,
            )
            .map_err(|_| "remote_view_native_registration_unavailable")?;
        if observation.target != *provider_target {
            return Err("remote_view_native_target_changed".into());
        }
        Ok(observation)
    }

    fn observe_native_handoff_use(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        registered: &RemoteViewNativeHandoffObservation,
    ) -> Result<RemoteViewNativeHandoffUseRead, String> {
        self.require_operation()?;
        let expected = self
            .expected
            .as_ref()
            .ok_or("remote_view_session_operation_baseline_missing")?;
        if expected.browsers.get(&target.browser.id) != Some(&target.browser)
            || expected.tabs.get(&target.tab.id) != Some(&target.tab)
            || expected.sessions.get(&target.session.id) != Some(&target.session)
        {
            return Err("remote_view_native_target_changed".into());
        }
        let assignment = remote_view_tab_assignment(&mut self.adapter, &mut self.store, target)?;
        self.adapter
            .observe_native_handoff_use_state(&assignment, registered)
            .map_err(|_| "remote_view_native_use_unavailable".into())
    }
    fn reconcile_profile_browser(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        browser: Option<&ManagedBrowserInstance>,
        tabs: &[ManagedBrowserTab],
        navigation: &[BrowserNavigationRecord],
    ) -> Result<Option<BrowserRecovery>, String> {
        self.require_operation()?;
        let Some(mut recovered) = self
            .effects
            .reconcile_profile_browser(profile, browser, tabs, navigation)?
        else {
            if browser.is_none()
                && self
                    .store
                    .unpublished_launch_records()
                    .map_err(|_| "remote_view_session_launch_readback_required")?
                    .iter()
                    .any(|record| record.intent.profile_id == profile.id)
            {
                // A missing current attachment is not proof that an attempted
                // initial launch had no effect. Explicit exact-absence settlement
                // must precede a new launch through the original operation.
                return Err("remote_view_session_launch_readback_required".into());
            }
            return Ok(None);
        };
        // Physical reuse does not inherit an old presentation location. Join
        // only current native window evidence for this exact browser process.
        recovered.launch.desktop = None;
        if let Ok(inventory) = self.adapter.inventory() {
            let mut matches = Vec::new();
            for assignment in inventory
                .assignments
                .into_iter()
                .filter(|assignment| assignment.state == RemoteViewAssignmentState::Active)
            {
                if self.adapter.windows(&assignment).is_ok_and(|windows| {
                    windows
                        .windows
                        .iter()
                        .any(|window| window.pid == Some(recovered.launch.pid))
                }) {
                    matches.push(assignment);
                }
            }
            if let [assignment] = matches.as_slice() {
                recovered.launch.desktop = Some(RemoteViewFixedDesktop {
                    desktop_id: assignment.desktop_id.clone(),
                    generation: assignment.generation,
                    friendly_route_label: "Existing profile browser".into(),
                });
                self.assignments
                    .insert(assignment.desktop_id.clone(), assignment.clone());
            }
        }
        // Direct process/profile/CDP and native window observations establish
        // the browser we can use now. An interrupted publication is completed
        // with that observation; historical claims never decide permission.
        if let Ok(records) = self.store.unpublished_launch_records() {
            if let Some(record) = records.into_iter().find(|record| {
                record.intent.profile_id == profile.id
                    && record.observed_browser_id.as_deref()
                        == Some(recovered.launch.browser_id.as_str())
                    && record.observed_pid == Some(recovered.launch.pid)
                    && recovered.launch.desktop.as_ref().is_some_and(|desktop| {
                        desktop.desktop_id == record.intent.assignment.desktop_id
                            && desktop.generation == record.intent.assignment.generation
                    })
            }) {
                self.pending = Some((record.intent, true));
            }
        }
        Ok(Some(recovered))
    }

    fn recover_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        tabs: &[ManagedBrowserTab],
        navigation: &[BrowserNavigationRecord],
    ) -> Result<BrowserRecovery, String> {
        self.recover_browser_with_target(browser, profile, tabs, navigation, None)
    }
    fn recover_browser_at(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        tabs: &[ManagedBrowserTab],
        navigation: &[BrowserNavigationRecord],
        desktop: &RemoteViewFixedDesktop,
    ) -> Result<BrowserRecovery, String> {
        self.recover_browser_with_target(browser, profile, tabs, navigation, Some(desktop))
    }
    fn select_profile_browser_executable(
        &mut self,
        profile_id: String,
        path: String,
    ) -> Result<(), String> {
        self.effects
            .select_profile_browser_executable(profile_id, path)
    }
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
        if let Some(issuance) = &view.issuance {
            if issuance.grant.expires_at <= now_ms {
                let assignment =
                    remote_view_tab_assignment(&mut self.adapter, &mut self.store, target)?;
                let observed = self
                    .adapter
                    .observe_assignment(&assignment)
                    .map_err(|_| "remote_view_tab_view_resolution_failed")?;
                // Validate the retained issuance at admission time before using
                // its expiry. Revocation and changed target identity fail closed.
                issuance
                    .validate_target(
                        &observed.target,
                        &self.adapter.application,
                        "remote_view",
                        RemoteViewApplicationViewCapability::Control,
                        300,
                        issuance.grant.issued_at,
                    )
                    .map_err(|_| "remote_view_tab_view_issuance_invalid")?;
                return Err("remote_view_tab_view_grant_terminal".into());
            }
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
            .map_err(|error| match error {
                RemoteViewApplicationAdapterError::ViewGrantTerminal => {
                    "remote_view_tab_view_grant_terminal".into()
                }
                _ => "remote_view_tab_view_issuance_readback_required".into(),
            })
    }

    fn observe_remote_view_tab_view(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        issuance: &RemoteViewApplicationViewIssuance,
        now_ms: u64,
    ) -> Result<RemoteViewApplicationViewObservation, String> {
        self.require_operation()?;
        let expected = self
            .expected
            .as_ref()
            .ok_or("remote_view_session_operation_baseline_missing")?;
        if expected.browsers.get(&target.browser.id) != Some(&target.browser)
            || expected.tabs.get(&target.tab.id) != Some(&target.tab)
            || expected.sessions.get(&target.session.id) != Some(&target.session)
        {
            return Err("remote_view_tab_view_identity_conflict".into());
        }
        let assignment = remote_view_tab_assignment(&mut self.adapter, &mut self.store, target)?;
        let clock = self.view_clock;
        self.adapter
            .observe_view(&assignment, &issuance.grant, || {
                clock.map_or(now_ms, |clock| clock())
            })
            .map_err(|_| "remote_view_tab_transport_readback_required".into())
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

    fn remote_view_placement_candidates(
        &mut self,
        placement: &BrowserPlacement,
    ) -> Result<Option<Vec<RemoteViewDesktopCandidate>>, String> {
        self.require_operation()?;
        // Explicit placement reads existing host-owned assignments only. It does
        // not acquire spare capacity or reject a desktop for having app windows.
        let inventory = self
            .adapter
            .inventory()
            .map_err(|_| "browser_placement_inventory_unavailable")?;
        let pool = self
            .pool
            .as_ref()
            .ok_or("browser_placement_provider_unconfigured")?;
        let pool_id = inventory
            .pools
            .iter()
            .find(|candidate| candidate.name == pool.name)
            .ok_or("browser_placement_pool_unavailable")?
            .pool_id
            .clone();
        let requested = match placement {
            BrowserPlacement::Required { desktop } => vec![desktop],
            BrowserPlacement::Preferred {
                desktop,
                alternatives,
            } => std::iter::once(desktop).chain(alternatives).collect(),
        };
        let mut candidates = Vec::new();
        for desktop in requested {
            let Some(assignment) = inventory.assignments.iter().find(|assignment| {
                assignment.pool_id == pool_id
                    && assignment.state == RemoteViewAssignmentState::Active
                    && assignment.desktop_id == desktop.desktop_id
                    && assignment.generation == desktop.generation
            }) else {
                continue;
            };
            if self.adapter.observe_assignment(assignment).is_err() {
                continue;
            }
            self.assignments
                .insert(assignment.desktop_id.clone(), assignment.clone());
            candidates.push(RemoteViewDesktopCandidate {
                ready: true,
                desktop: RemoteViewFixedDesktop {
                    desktop_id: assignment.desktop_id.clone(),
                    friendly_route_label: assignment.desktop_id.clone(),
                    generation: assignment.generation,
                },
            });
        }
        Ok(Some(candidates))
    }

    fn begin_operation(&mut self, expected: &BrowserSessionState) -> Result<(), String> {
        // A failed earlier operation leaves its observation in the durable
        // journal. It must not poison another task through process-local state.
        // Current profile reconciliation can join that observation again.
        self.pending = None;
        let activities = self
            .store
            .browser_activity_records()
            .map_err(|_| "browser_session_activity_readback_unavailable")?;
        // Only atomic publication removes transient acknowledged rows. A lost
        // reply may leave local IDs after that commit, even with unchanged
        // session JSON. Ended resources likewise keep history, not local vetoes.
        self.activities.retain(|id| {
            activities
                .iter()
                .any(|record| &record.id == id && !record.original_ended)
        });
        // Several interrupted commands can retain the same original process.
        // Share its observation only inside this operation and only for an
        // exactly equal scope. Unknown evidence still retains every obligation;
        // positive original exit settles each durable activity independently.
        let mut observations: Vec<(BrowserIdleRetirementScope, bool)> = Vec::new();
        for activity in activities
            .into_iter()
            .filter(|activity| !activity.original_ended)
        {
            let scope = BrowserIdleRetirementScope {
                browser: activity.browser.clone(),
                process_identity: activity.process_identity.clone(),
                tabs: Vec::new(),
            };
            // No target inventory, attachment or effect. Unavailable evidence
            // retains the veto; only positive original exit settles resources.
            let original_ended = match observations.iter().find(|(seen, _)| seen == &scope) {
                Some((_, ended)) => *ended,
                None => {
                    let ended =
                        self.effects.observe_idle_retirement_effect(&scope, None) == Ok(true);
                    observations.push((scope, ended));
                    ended
                }
            };
            if original_ended {
                match self.store.settle_browser_activity_resource(&activity) {
                    Ok(()) | Err(BrowserIdleRetirementStoreError::Conflict) => (),
                    Err(_) => return Err("browser_session_activity_readback_unavailable".into()),
                }
            }
        }
        self.retirements = self
            .store
            .idle_retirement_records()
            .map_err(|_| "browser_session_idle_custody_unavailable")?
            .into_iter()
            .filter(|record| !record.is_terminal())
            .map(|record| (record.intent.id.clone(), record))
            .collect();
        // Recover exact native preparation before attempting any new effect.
        // A known prepared operation with no admitted effects can publish an
        // unchanged state atomically. That publication fences a delayed actor's
        // future effect admission before native exclusion is completed.
        let reconcilable = self
            .retirements
            .values()
            .filter(|record| {
                record.published_state_digest.is_some()
                    || (record.preparation_started() && !record.has_effect_attempts())
            })
            .cloned()
            .collect::<Vec<_>>();
        for original in reconcilable {
            let Ok(observed) = self
                .adapter
                .reconcile_idle_retirement(&original.intent.provider, original.prepared.as_ref())
            else {
                continue;
            };
            let mut record = self
                .store
                .idle_retirement_records()
                .map_err(|_| "browser_session_idle_custody_unavailable")?
                .into_iter()
                .find(|record| record.intent == original.intent)
                .ok_or("browser_session_idle_readback_required")?;
            self.retirements
                .insert(record.intent.id.clone(), record.clone());
            if record.is_terminal() {
                continue;
            }
            if record.published_state_digest.is_none() {
                if record.has_effect_attempts()
                    || observed.state != RemoteViewIdleRetirementReceiptState::Prepared
                {
                    continue;
                }
                if record.prepared.is_none() {
                    let mut next = record.clone();
                    next.record_prepared(observed.clone())
                        .map_err(|_| "browser_session_idle_readback_required")?;
                    match self.store.update_idle_retirement(&record, &next) {
                        Ok(()) => record = next,
                        Err(BrowserIdleRetirementStoreError::Conflict) => continue,
                        Err(_) => return Err("browser_session_idle_custody_unavailable".into()),
                    }
                }
                match self.store.publish_idle_retirement_state(
                    expected,
                    expected,
                    std::slice::from_ref(&record.intent.id),
                ) {
                    Ok(()) => (),
                    Err(BrowserIdleRetirementStoreError::Conflict) => continue,
                    Err(_) => return Err("browser_session_idle_custody_unavailable".into()),
                }
                record = self
                    .store
                    .idle_retirement_records()
                    .map_err(|_| "browser_session_idle_custody_unavailable")?
                    .into_iter()
                    .find(|current| current.intent == record.intent)
                    .ok_or("browser_session_idle_readback_required")?;
            }
            let completed = if observed.state == RemoteViewIdleRetirementReceiptState::Completed {
                Ok(observed)
            } else {
                self.adapter.complete_idle_retirement(
                    &record.intent.provider,
                    record
                        .prepared
                        .as_ref()
                        .ok_or("browser_session_idle_readback_required")?,
                    true,
                )
            };
            if let Ok(completed) = completed {
                let mut next = record.clone();
                next.record_completed(completed)
                    .map_err(|_| "browser_session_idle_readback_required")?;
                self.update_retirement(&record, next)?;
            }
        }
        self.operation_retirements.clear();

        self.effects.begin_operation(expected)?;
        self.expected = Some(expected.clone());
        Ok(())
    }
    fn pending_browser_activity_ids(&self) -> Vec<String> {
        self.activities.iter().cloned().collect()
    }
    fn pending_idle_retirement_ids(&self) -> Vec<String> {
        self.operation_retirements
            .iter()
            .filter(|id| {
                self.retirements.get(*id).is_some_and(|record| {
                    !record.is_terminal() && record.published_state_digest.is_none()
                })
            })
            .cloned()
            .collect()
    }
    fn finish_idle_retirement_publication(&mut self, ids: &[String]) -> Result<(), String> {
        for id in ids {
            let record = self
                .store
                .idle_retirement_records()
                .map_err(|_| "browser_session_idle_custody_unavailable")?
                .into_iter()
                .find(|record| record.intent.id == *id)
                .ok_or("browser_session_idle_custody_unavailable")?;
            if record.published_state_digest.is_none() {
                return Err("browser_session_idle_readback_required".into());
            }
            if record.completed.is_some() {
                continue;
            }
            let completed = self
                .adapter
                .complete_idle_retirement(
                    &record.intent.provider,
                    record
                        .prepared
                        .as_ref()
                        .ok_or("browser_session_idle_readback_required")?,
                    true,
                )
                .map_err(|_| "browser_session_idle_readback_required")?;
            let mut next = record.clone();
            next.record_completed(completed)
                .map_err(|_| "browser_session_idle_readback_required")?;
            self.update_retirement(&record, next)?;
        }
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
        self.activities.clear();
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
        .map_err(|error| match error {
            RemoteViewBrowserLaunchError::Preflight(reason) => {
                // No durable claim or process effect exists for this attempt.
                self.pending = None;
                reason
            }
            _ => "remote_view_session_launch_readback_required".to_string(),
        })?;
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
    fn permits_idle_browser_close(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<bool, String> {
        self.require_operation()?;
        // Provider idleness cannot override current controller or operation
        // protection supplied by the browser runtime authority.
        if !self.effects.permits_idle_browser_close(browser)? {
            return Ok(false);
        }
        if self.pool.is_none() || self.borrows_browser_without_native_desktop(browser) {
            return Ok(true);
        }
        let assignment = self
            .store
            .published_launch_assignment(browser)
            .map_err(|_| "browser_session_idle_custody_unavailable")?;
        let observation = self
            .adapter
            .observe_idle(&assignment)
            .map_err(|_| "browser_session_idle_viewer_observation_unavailable")?;
        Ok(!observation.viewer_active)
    }
    fn acquire_profile_cleanup_guard(
        &mut self,
        allocation: &crate::ManagedDisposableProfile,
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        self.require_operation()?;
        self.effects.acquire_profile_cleanup_guard(allocation)
    }
    fn acquire_idle_retirement_guard(
        &mut self,
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        self.require_operation()?;
        self.qualify_idle_viewers(browsers, |effects| {
            effects.acquire_idle_retirement_guard(browsers)
        })
    }
    fn acquire_disposable_retirement_guard(
        &mut self,
        allocations: &[crate::ManagedDisposableProfile],
        browsers: &[ManagedBrowserInstance],
    ) -> Result<Option<Box<dyn BrowserIdleRetirementGuard>>, String> {
        self.require_operation()?;
        self.qualify_idle_viewers(browsers, |effects| {
            effects.acquire_disposable_retirement_guard(allocations, browsers)
        })
    }
    fn close_browser(&mut self, browser: &ManagedBrowserInstance) -> Result<(), String> {
        self.require_operation()?;
        let effect = BrowserIdleRetirementEffect::CloseBrowser {
            browser_id: browser.id.clone(),
        };
        let (record, apply) = self.begin_retirement_effect(browser, effect.clone())?;
        if apply {
            if let Some(record) = record.as_ref() {
                let scope = record
                    .intent
                    .scopes
                    .iter()
                    .find(|scope| scope.browser.id == browser.id)
                    .ok_or("browser_session_idle_custody_identity_changed")?;
                self.effects.apply_idle_retirement_effect(scope, None)?;
            } else {
                self.effects.close_browser(browser)?;
            }
        }
        if let Some(record) = record {
            self.settle_retirement_effect(record, &effect)?;
        }
        Ok(())
    }
    fn acquire_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.require_operation()?;
        self.require_clear_profile_custody(&browser.profile_id)?;
        self.with_browser_activity(browser, |effects| {
            effects.acquire_initial_tab(browser, attributed_target_ids)
        })
    }
    fn acquire_session_initial_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        session: &ManagedBrowserSession,
        attributed_target_ids: &[String],
    ) -> Result<BrowserTabAcquisition, String> {
        self.require_operation()?;
        self.require_clear_profile_custody(&browser.profile_id)?;
        self.with_browser_activity(browser, |effects| {
            effects.acquire_session_initial_tab(browser, profile, session, attributed_target_ids)
        })
    }
    fn create_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
    ) -> Result<BrowserTabAcquisition, String> {
        self.require_operation()?;
        self.require_clear_profile_custody(&browser.profile_id)?;
        self.with_browser_activity(browser, |effects| effects.create_tab(browser))
    }
    fn close_tab(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
    ) -> Result<(), String> {
        self.require_operation()?;
        let effect = BrowserIdleRetirementEffect::CloseTab {
            browser_id: browser.id.clone(),
            tab_id: tab.id.clone(),
            target_id: tab.target_id.clone(),
        };
        let (record, apply) = self.begin_retirement_effect(browser, effect.clone())?;
        if apply {
            if let Some(record) = record.as_ref() {
                let scope = record
                    .intent
                    .scopes
                    .iter()
                    .find(|scope| scope.browser.id == browser.id)
                    .ok_or("browser_session_idle_custody_identity_changed")?;
                self.effects
                    .apply_idle_retirement_effect(scope, Some(tab))?;
            } else {
                self.effects.close_tab(browser, tab)?;
            }
        }
        if let Some(record) = record {
            self.settle_retirement_effect(record, &effect)?;
        }
        Ok(())
    }
    fn navigate(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: &ManagedBrowserTab,
        url: &str,
    ) -> Result<(), String> {
        self.require_operation()?;
        self.require_clear_profile_custody(&browser.profile_id)?;
        self.with_browser_activity(browser, |effects| effects.navigate(browser, tab, url))
    }
    fn focus_browser(
        &mut self,
        browser: &ManagedBrowserInstance,
        tab: Option<&ManagedBrowserTab>,
    ) -> Result<(), String> {
        self.require_operation()?;
        self.require_clear_profile_custody(&browser.profile_id)?;
        let desktop = browser
            .desktop
            .as_ref()
            .ok_or("remote_view_focus_desktop_missing")?;
        let assignment = self
            .store
            .published_presentation_assignment(browser)
            .map_err(|_| "remote_view_focus_launch_custody_unavailable")?;
        if assignment.desktop_id != desktop.desktop_id
            || assignment.generation != desktop.generation
        {
            return Err("remote_view_focus_assignment_conflict".into());
        }
        // CDP selects the addressed target. Provider observations then fence the
        // current desktop generations and prove the owned process is foreground.
        // This is window proof only; it does not establish viewer connectivity.
        self.with_browser_activity(browser, |effects| effects.focus_browser(browser, tab))?;
        let windows = self
            .adapter
            .windows(&assignment)
            .map_err(|_| "remote_view_focus_window_readback_required")?;
        let count = windows
            .windows
            .iter()
            .filter(|window| {
                window.pid == Some(browser.pid)
                    && window.active
                    && window.width > 0
                    && window.height > 0
            })
            .count();
        if browser.pid == 0 || count != 1 {
            return Err("remote_view_focus_owned_window_unproven".into());
        }
        Ok(())
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
        S: BrowserLaunchCustodyStore + BrowserIdleRetirementCustodyStore,
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
        self.require_clear_profile_custody(&browser.profile_id)?;
        self.with_browser_activity(browser, |effects| {
            effects.execute_command(browser, tab, session_id, session_name, command)
        })
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
                // Released history cannot provide current presentation custody.
                && assignment.state == RemoteViewAssignmentState::Active
        })
        .collect::<Vec<_>>();
    let assignment = match matching.as_slice() {
        [assignment] => (*assignment).clone(),
        _ => return Err("remote_view_tab_view_assignment_unavailable".into()),
    };
    let published = store
        .published_presentation_assignment(&target.browser)
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

impl<
        E: BrowserSessionEffects + RemoteViewBrowserProcessEffects,
        T: RemoteViewApplicationTransport,
        S: BrowserLaunchCustodyStore + RemoteViewPoolRequestStore + BrowserIdleRetirementCustodyStore,
    > RemoteViewSessionEffects<E, T, S>
{
    fn recover_browser_with_target(
        &mut self,
        browser: &ManagedBrowserInstance,
        profile: &BrowserProfileCatalogEntry,
        tabs: &[ManagedBrowserTab],
        navigation: &[BrowserNavigationRecord],
        target: Option<&RemoteViewFixedDesktop>,
    ) -> Result<BrowserRecovery, String> {
        self.require_operation()?;
        self.effects
            .prove_recovery_absence(browser, profile)
            .map_err(|_| "browser_session_recovery_absence_unproven")?;
        // Startup may have observed this profile live. Recheck its interrupted
        // launches when absence becomes provable in a long-lived host, before
        // reserving a replacement. No other profile's custody is changed.
        let expected = self.expected.as_ref().expect("qualified baseline");
        for claim in self
            .store
            .unpublished_launch_records()
            .map_err(|_| "browser_session_recovery_readback_required")?
            .into_iter()
            .filter(|claim| claim.intent.profile_id == profile.id)
        {
            let mut observed = browser.clone();
            if let Some(pid) = claim.observed_pid {
                observed.pid = pid;
            }
            self.effects
                .prove_recovery_absence(&observed, profile)
                .map_err(|_| "browser_session_recovery_absence_unproven")?;
            self.store
                .reconcile_absent_recovery(&claim, expected, browser)
                .map_err(|_| "browser_session_recovery_readback_required")?;
        }
        let borrowed = self
            .expected
            .as_ref()
            .expect("qualified baseline")
            .shared_browser_ids
            .contains(&browser.id);
        let (mut assignment, mut desktop) = if let Some(desktop) = target {
            let placement = BrowserPlacement::Required {
                desktop: BrowserDesktopBinding {
                    desktop_id: desktop.desktop_id.clone(),
                    generation: desktop.generation,
                },
            };
            let candidates = self
                .remote_view_placement_candidates(&placement)?
                .ok_or("browser_reconnect_recovery_target_unavailable")?;
            if !candidates.iter().any(|c| {
                c.ready
                    && c.desktop.desktop_id == desktop.desktop_id
                    && c.desktop.generation == desktop.generation
            }) {
                return Err("browser_reconnect_recovery_target_unavailable".into());
            }
            let assignment = self
                .assignments
                .get(&desktop.desktop_id)
                .filter(|a| a.generation == desktop.generation)
                .cloned()
                .ok_or("browser_reconnect_recovery_target_unavailable")?;
            (assignment, desktop.clone())
        } else if let Some(desktop) = browser.desktop.as_ref().filter(|_| !borrowed) {
            let assignment = self
                .store
                .published_launch_assignment(browser)
                .map_err(|_| "browser_session_recovery_prior_custody_unavailable")?;
            (assignment, desktop.clone())
        } else {
            // Local and borrowed browsers have no native launch custody to
            // inherit. Current profile absence qualifies a fresh placement.
            let pool = self
                .pool
                .as_ref()
                .ok_or("browser_session_recovery_pool_missing")?;
            let prepared =
                prepare_remote_view_session_pool(&mut self.adapter, &mut self.store, pool)?;
            let desktop = prepared
                .desktops
                .first()
                .ok_or("browser_session_recovery_desktop_unavailable")?
                .desktop
                .clone();
            let assignment = prepared
                .assignments
                .into_iter()
                .find(|assignment| {
                    assignment.desktop_id == desktop.desktop_id
                        && assignment.generation == desktop.generation
                })
                .ok_or("browser_session_recovery_assignment_unavailable")?;
            (assignment, desktop)
        };
        if assignment.desktop_id != desktop.desktop_id
            || assignment.generation != desktop.generation
        {
            return Err("browser_session_recovery_assignment_conflict".into());
        }
        if self
            .store
            .assignment_release_confirmed(&assignment)
            .map_err(|_| "browser_session_recovery_prior_custody_unavailable")?
        {
            if target.is_some() {
                return Err("browser_reconnect_recovery_target_released".into());
            }
            let pool = self
                .pool
                .as_ref()
                .ok_or("browser_session_recovery_pool_missing")?;
            let prepared =
                prepare_remote_view_session_pool(&mut self.adapter, &mut self.store, pool)?;
            desktop = prepared
                .desktops
                .first()
                .ok_or("browser_session_recovery_desktop_unavailable")?
                .desktop
                .clone();
            assignment = prepared
                .assignments
                .into_iter()
                .find(|assignment| {
                    assignment.desktop_id == desktop.desktop_id
                        && assignment.generation == desktop.generation
                })
                .ok_or("browser_session_recovery_assignment_unavailable")?;
        }
        let intent = BrowserLaunchIntent {
            intent_id: (self.next_intent_id)(),
            profile_id: profile.id.clone(),
            assignment,
        };
        // Provider read failures precede process admission. No launch intent
        // should survive when recovery never reached a process effect.
        let environment = self
            .adapter
            .launch_environment(&intent.assignment)
            .map_err(|_| "browser_session_recovery_environment_unavailable")?;
        let observation = self
            .adapter
            .observe_assignment(&intent.assignment)
            .map_err(|_| "browser_session_recovery_assignment_unavailable")?;
        environment
            .validate_observation(&observation, &intent.assignment)
            .map_err(|_| "browser_session_recovery_assignment_conflict")?;
        match self
            .store
            .admit_recovery_launch_intent(
                &intent,
                self.expected.as_ref().expect("qualified baseline"),
                browser,
            )
            .map_err(|_| "browser_session_recovery_custody_conflict")?
        {
            LaunchCustodyAdmission::New => (),
            LaunchCustodyAdmission::Existing(_) => {
                return Err("browser_session_recovery_readback_required".into())
            }
        }
        self.pending = Some((intent.clone(), false));
        let mut launch = self
            .effects
            .recover_launch(browser, profile, &intent, environment, &observation)
            .map_err(|_| "browser_session_recovery_launch_readback_required")?;
        if launch.browser_id != browser.id {
            return Err("browser_session_recovery_identity_conflict".into());
        }
        launch.desktop = Some(desktop.clone());
        self.store
            .observe_launch_intent(&intent, &launch)
            .map_err(|_| "browser_session_recovery_observation_readback_required")?;
        let mut restored_browser = browser.clone();
        restored_browser.pid = launch.pid;
        restored_browser.cdp_endpoint = launch.cdp_endpoint.clone();
        restored_browser.desktop = launch.desktop.clone();
        let mut target_ids = BTreeMap::new();
        if launch.cdp_endpoint.is_empty() {
            // Native routing handles denote the common browser context, never
            // invented CDP pages. No old website navigation is replayed.
            for tab in tabs {
                target_ids.insert(
                    tab.id.clone(),
                    format!("native-context:{}:{}", browser.id, tab.id),
                );
            }
            self.pending = Some((intent, true));
            return Ok(BrowserRecovery { launch, target_ids });
        }
        // Custody remains unpublished throughout restoration. A crash or failed
        // restoration cannot authorize another process or duplicate tab replay.
        for (index, tab) in tabs.iter().enumerate() {
            let acquired = if index == 0 {
                self.effects.acquire_initial_tab(&restored_browser, &[])?
            } else {
                self.effects.create_tab(&restored_browser)?
            };
            let mut restored_tab = tab.clone();
            restored_tab.target_id = acquired.target_id.clone();
            let url = navigation
                .iter()
                .filter(|row| {
                    row.tab_id == tab.id
                        && row.session_id == tab.session_id
                        && row.browser_id == browser.id
                })
                .max_by_key(|row| row.visited_at_ms)
                .map_or("about:blank", |row| row.url.as_str());
            self.effects
                .navigate(&restored_browser, &restored_tab, url)?;
            target_ids.insert(tab.id.clone(), acquired.target_id);
        }
        self.pending = Some((intent, true));
        Ok(BrowserRecovery { launch, target_ids })
    }
}
