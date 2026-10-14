//! Durable application effect custody. Recording an intent or attempt grants
//! no close authority; runtime owners retain exact physical and activity checks.
use crate::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserIdleRetirementStoreError {
    Unavailable,
    InvalidRecord,
    Conflict,
}
type Result<T> = std::result::Result<T, BrowserIdleRetirementStoreError>;

pub fn state_digest(state: &BrowserSessionState) -> Result<String> {
    serde_json::to_vec(state)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|_| BrowserIdleRetirementStoreError::InvalidRecord)
}
fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserIdleRetirementScope {
    pub browser: ManagedBrowserInstance,
    pub tabs: Vec<ManagedBrowserTab>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserIdleRetirementIntent {
    pub id: String,
    pub baseline_digest: String,
    pub provider: RemoteViewIdleRetirementIntent,
    pub scopes: Vec<BrowserIdleRetirementScope>,
}
impl BrowserIdleRetirementIntent {
    pub fn new(
        id: &str,
        provider: RemoteViewIdleRetirementIntent,
        state: &BrowserSessionState,
        browser_ids: &[String],
    ) -> Result<Self> {
        let mut scopes = Vec::new();
        for id in browser_ids {
            let browser = state
                .browsers
                .get(id)
                .ok_or(BrowserIdleRetirementStoreError::Conflict)?
                .clone();
            let tabs = state
                .tabs
                .values()
                .filter(|tab| tab.browser_id == *id)
                .cloned()
                .collect();
            scopes.push(BrowserIdleRetirementScope { browser, tabs });
        }
        scopes.sort_by(|a, b| a.browser.id.cmp(&b.browser.id));
        let intent = Self {
            id: id.into(),
            baseline_digest: state_digest(state)?,
            provider,
            scopes,
        };
        intent.validate()?;
        Ok(intent)
    }
    pub fn validate(&self) -> Result<()> {
        self.provider
            .validate()
            .map_err(|_| BrowserIdleRetirementStoreError::InvalidRecord)?;
        if !crate::remote_view_application_response::uuid(&self.id)
            || !digest(&self.baseline_digest)
            || self.scopes.is_empty()
        {
            return Err(BrowserIdleRetirementStoreError::InvalidRecord);
        }
        let mut browsers = BTreeSet::new();
        let mut tabs = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for scope in &self.scopes {
            let browser = &scope.browser;
            let desktop = browser
                .desktop
                .as_ref()
                .ok_or(BrowserIdleRetirementStoreError::InvalidRecord)?;
            if browser.id.is_empty()
                || browser.profile_id.is_empty()
                || browser.pid == 0
                || browser.cdp_endpoint.is_empty()
                || !browsers.insert(&browser.id)
                || desktop.desktop_id != self.provider.assignment.desktop_id
                || desktop.generation != self.provider.assignment.generation
            {
                return Err(BrowserIdleRetirementStoreError::InvalidRecord);
            }
            for tab in &scope.tabs {
                if tab.browser_id != browser.id
                    || tab.id.is_empty()
                    || tab.target_id.is_empty()
                    || !tabs.insert(&tab.id)
                    || !targets.insert(&tab.target_id)
                {
                    return Err(BrowserIdleRetirementStoreError::InvalidRecord);
                }
            }
        }
        Ok(())
    }
    pub fn matches_baseline(&self, state: &BrowserSessionState) -> Result<()> {
        if Self::new(
            &self.id,
            self.provider.clone(),
            state,
            &self
                .scopes
                .iter()
                .map(|scope| scope.browser.id.clone())
                .collect::<Vec<_>>(),
        )? != *self
        {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BrowserIdleRetirementEffect {
    CloseTab {
        browser_id: String,
        tab_id: String,
        target_id: String,
    },
    CloseBrowser {
        browser_id: String,
    },
}
impl BrowserIdleRetirementEffect {
    fn key(&self) -> String {
        match self {
            Self::CloseTab { tab_id, .. } => format!("tab:{tab_id}"),
            Self::CloseBrowser { browser_id } => format!("browser:{browser_id}"),
        }
    }
    pub fn browser_id(&self) -> &str {
        match self {
            Self::CloseTab { browser_id, .. } | Self::CloseBrowser { browser_id } => browser_id,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Attempt {
    effect: BrowserIdleRetirementEffect,
    settled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserIdleRetirementEffectAdmission {
    New,
    PendingReadback,
    AlreadySettled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserIdleRetirementRecord {
    pub intent: BrowserIdleRetirementIntent,
    preparation_started: bool,
    deferred: bool,
    pub prepared: Option<RemoteViewIdleRetirementReceipt>,
    effects: Vec<Attempt>,
    pub published_state_digest: Option<String>,
    pub completed: Option<RemoteViewIdleRetirementReceipt>,
}
impl BrowserIdleRetirementRecord {
    pub fn new(intent: BrowserIdleRetirementIntent) -> Result<Self> {
        intent.validate()?;
        Ok(Self {
            intent,
            preparation_started: false,
            deferred: false,
            prepared: None,
            effects: vec![],
            published_state_digest: None,
            completed: None,
        })
    }
    pub fn is_terminal(&self) -> bool {
        self.deferred || self.completed.is_some()
    }
    pub fn validate(&self) -> Result<()> {
        self.intent.validate()?;
        let invalid = BrowserIdleRetirementStoreError::InvalidRecord;
        if self.deferred
            && (!self.preparation_started
                || self.prepared.is_some()
                || !self.effects.is_empty()
                || self.published_state_digest.is_some()
                || self.completed.is_some())
        {
            return Err(invalid);
        }
        if let Some(prepared) = &self.prepared {
            if !self.preparation_started {
                return Err(invalid);
            }
            prepared
                .validate(
                    &self.intent.provider,
                    RemoteViewIdleRetirementReceiptState::Prepared,
                )
                .map_err(|_| invalid)?;
        } else if !self.effects.is_empty()
            || self.published_state_digest.is_some()
            || self.completed.is_some()
        {
            return Err(invalid);
        }
        let mut keys = BTreeSet::new();
        for attempt in &self.effects {
            self.validate_effect(&attempt.effect)?;
            if !keys.insert(attempt.effect.key()) {
                return Err(invalid);
            }
        }
        if self
            .published_state_digest
            .as_ref()
            .is_some_and(|value| !digest(value))
            || (self.published_state_digest.is_some()
                && self.effects.iter().any(|effect| !effect.settled))
        {
            return Err(invalid);
        }
        if let Some(completed) = &self.completed {
            if self.published_state_digest.is_none() {
                return Err(invalid);
            }
            completed
                .validate(
                    &self.intent.provider,
                    RemoteViewIdleRetirementReceiptState::Completed,
                )
                .map_err(|_| invalid)?;
            if Some(&completed.fence) != self.prepared.as_ref().map(|prepared| &prepared.fence) {
                return Err(invalid);
            }
        }
        Ok(())
    }
    fn validate_effect(&self, effect: &BrowserIdleRetirementEffect) -> Result<()> {
        let scope = self
            .intent
            .scopes
            .iter()
            .find(|scope| scope.browser.id == effect.browser_id())
            .ok_or(BrowserIdleRetirementStoreError::InvalidRecord)?;
        if let BrowserIdleRetirementEffect::CloseTab {
            tab_id, target_id, ..
        } = effect
        {
            if !scope
                .tabs
                .iter()
                .any(|tab| tab.id == *tab_id && tab.target_id == *target_id)
            {
                return Err(BrowserIdleRetirementStoreError::InvalidRecord);
            }
        }
        Ok(())
    }
    pub fn begin_preparation(&mut self) -> Result<()> {
        self.validate()?;
        if self.preparation_started {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        self.preparation_started = true;
        Ok(())
    }
    pub fn defer_preparation(&mut self) -> Result<()> {
        self.validate()?;
        if !self.preparation_started || self.prepared.is_some() {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        self.deferred = true;
        Ok(())
    }
    pub fn record_prepared(&mut self, prepared: RemoteViewIdleRetirementReceipt) -> Result<()> {
        self.validate()?;
        if !self.preparation_started
            || self.deferred
            || self.prepared.as_ref().is_some_and(|old| *old != prepared)
        {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        prepared
            .validate(
                &self.intent.provider,
                RemoteViewIdleRetirementReceiptState::Prepared,
            )
            .map_err(|_| BrowserIdleRetirementStoreError::InvalidRecord)?;
        self.prepared = Some(prepared);
        Ok(())
    }
    pub fn begin_effect(
        &mut self,
        effect: BrowserIdleRetirementEffect,
    ) -> Result<BrowserIdleRetirementEffectAdmission> {
        self.validate()?;
        self.validate_effect(&effect)?;
        if self.prepared.is_none() || self.published_state_digest.is_some() || self.is_terminal() {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        if let Some(old) = self
            .effects
            .iter()
            .find(|old| old.effect.key() == effect.key())
        {
            if old.effect != effect {
                return Err(BrowserIdleRetirementStoreError::Conflict);
            }
            return Ok(if old.settled {
                BrowserIdleRetirementEffectAdmission::AlreadySettled
            } else {
                BrowserIdleRetirementEffectAdmission::PendingReadback
            });
        }
        self.effects.push(Attempt {
            effect,
            settled: false,
        });
        Ok(BrowserIdleRetirementEffectAdmission::New)
    }
    pub fn settle_effect(&mut self, effect: &BrowserIdleRetirementEffect) -> Result<()> {
        self.validate()?;
        if self.published_state_digest.is_some() {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        let attempt = self
            .effects
            .iter_mut()
            .find(|old| old.effect == *effect)
            .ok_or(BrowserIdleRetirementStoreError::Conflict)?;
        attempt.settled = true;
        Ok(())
    }
    /// Used only by the SQLite publisher in the same transaction as session
    /// state. Generic record updates must reject this transition.
    pub fn record_publication(&mut self, state: &BrowserSessionState) -> Result<()> {
        self.validate()?;
        if self.prepared.is_none()
            || self.is_terminal()
            || self.effects.iter().any(|effect| !effect.settled)
        {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        for attempt in &self.effects {
            if match &attempt.effect {
                BrowserIdleRetirementEffect::CloseTab { tab_id, .. } => {
                    state.tabs.contains_key(tab_id)
                }
                BrowserIdleRetirementEffect::CloseBrowser { browser_id } => {
                    state.browsers.contains_key(browser_id)
                }
            } {
                return Err(BrowserIdleRetirementStoreError::Conflict);
            }
        }
        let digest = state_digest(state)?;
        if self
            .published_state_digest
            .as_ref()
            .is_some_and(|old| *old != digest)
        {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        self.published_state_digest = Some(digest);
        Ok(())
    }
    pub fn record_completed(&mut self, completed: RemoteViewIdleRetirementReceipt) -> Result<()> {
        self.validate()?;
        if self.published_state_digest.is_none()
            || self.completed.as_ref().is_some_and(|old| *old != completed)
        {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        completed
            .validate(
                &self.intent.provider,
                RemoteViewIdleRetirementReceiptState::Completed,
            )
            .map_err(|_| BrowserIdleRetirementStoreError::InvalidRecord)?;
        if Some(&completed.fence) != self.prepared.as_ref().map(|old| &old.fence) {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        self.completed = Some(completed);
        Ok(())
    }
    /// A CAS can persist one monotonic step. Publication has its own atomic
    /// session-state boundary and cannot be forged through a generic update.
    pub fn validate_successor(&self, next: &Self) -> Result<()> {
        self.validate()?;
        next.validate()?;
        if self.intent != next.intent || self.published_state_digest != next.published_state_digest
        {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        if self == next {
            return Ok(());
        }
        let mut candidate = self.clone();
        if !self.preparation_started && next.preparation_started {
            candidate.begin_preparation()?;
        } else if !self.deferred && next.deferred {
            candidate.defer_preparation()?;
        } else if self.prepared.is_none() && next.prepared.is_some() {
            candidate.record_prepared(next.prepared.clone().unwrap())?;
        } else if next.effects.len() == self.effects.len() + 1 {
            candidate.begin_effect(next.effects.last().unwrap().effect.clone())?;
        } else if next.effects.len() == self.effects.len() {
            let changed = self
                .effects
                .iter()
                .zip(&next.effects)
                .filter(|(a, b)| a != b)
                .collect::<Vec<_>>();
            if let [(old, new)] = changed.as_slice() {
                if old.effect == new.effect && !old.settled && new.settled {
                    candidate.settle_effect(&old.effect)?;
                }
            } else if self.completed.is_none() && next.completed.is_some() {
                candidate.record_completed(next.completed.clone().unwrap())?;
            }
        }
        if candidate != *next {
            return Err(BrowserIdleRetirementStoreError::Conflict);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrowserIdleRetirementAdmission {
    New,
    Existing(Box<BrowserIdleRetirementRecord>),
}
pub trait BrowserIdleRetirementCustodyStore {
    fn idle_retirement_records(&mut self) -> Result<Vec<BrowserIdleRetirementRecord>>;
    fn claim_idle_retirement(
        &mut self,
        intent: &BrowserIdleRetirementIntent,
        expected: &BrowserSessionState,
    ) -> Result<BrowserIdleRetirementAdmission>;
    fn update_idle_retirement(
        &mut self,
        previous: &BrowserIdleRetirementRecord,
        next: &BrowserIdleRetirementRecord,
    ) -> Result<()>;
    fn publish_idle_retirement_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        ids: &[String],
    ) -> Result<()>;
}
