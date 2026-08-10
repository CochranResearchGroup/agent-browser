use std::collections::BTreeMap;

use serde::Serialize;

use super::service_model::{BrowserHealth, ServiceState};
use super::service_resources::{
    service_resource_authority_snapshot, ResourceAuthoritySnapshot, ResourceDisposition,
    ResourceKind,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct BrowserSessionAuthoritySnapshot {
    pub(crate) schema_version: u8,
    pub(crate) availability: BrowserSessionAuthorityAvailability,
    pub(crate) summary: BrowserSessionAuthoritySummary,
    pub(crate) resource_pressure: BrowserSessionResourcePressure,
    pub(crate) browser_verdicts: Vec<BrowserSessionAuthorityVerdict>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct BrowserSessionAuthoritySummary {
    pub(crate) modeled_browser_count: usize,
    pub(crate) viable_browser_count: usize,
    pub(crate) attention_browser_count: usize,
    pub(crate) non_viable_browser_count: usize,
    pub(crate) unknown_browser_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct BrowserSessionResourcePressure {
    pub(crate) state: BrowserSessionResourcePressureState,
    pub(crate) total_process_count: usize,
    pub(crate) correlated_process_count: usize,
    pub(crate) candidate_count: usize,
    pub(crate) protected_count: usize,
    pub(crate) observed_count: usize,
    pub(crate) observed_unowned_agent_browser_process_count: usize,
    pub(crate) candidate_rss_bytes: u64,
    pub(crate) total_rss_bytes: u64,
    pub(crate) reasons: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct BrowserSessionAuthorityVerdict {
    pub(crate) key: String,
    pub(crate) browser_id: String,
    pub(crate) state: BrowserSessionAuthorityVerdictState,
    pub(crate) viable: bool,
    pub(crate) needs_attention: bool,
    pub(crate) reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BrowserSessionAuthorityAvailability {
    Available,
    Partial,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BrowserSessionResourcePressureState {
    Clear,
    Pressure,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BrowserSessionAuthorityVerdictState {
    Viable,
    Attention,
    #[default]
    NonViable,
}

impl BrowserSessionAuthoritySnapshot {
    pub(crate) fn validate(
        &self,
    ) -> Result<(), super::service_status_projection::ServiceStatusProjectionError> {
        let summary_total = self.summary.viable_browser_count
            + self.summary.attention_browser_count
            + self.summary.non_viable_browser_count
            + self.summary.unknown_browser_count;
        if self.schema_version != 1 {
            return Err(
                super::service_status_projection::ServiceStatusProjectionError::InvalidAuthority(
                    "browserSessionAuthority.schemaVersion must equal 1".to_string(),
                ),
            );
        }
        if summary_total != self.summary.modeled_browser_count {
            return Err(
                super::service_status_projection::ServiceStatusProjectionError::InvalidAuthority(
                    "browserSessionAuthority summary counts do not reconcile".to_string(),
                ),
            );
        }
        if self.browser_verdicts.len() + self.summary.unknown_browser_count
            != self.summary.modeled_browser_count
        {
            return Err(
                super::service_status_projection::ServiceStatusProjectionError::InvalidAuthority(
                    "browserSessionAuthority verdict and unknown counts do not reconcile"
                        .to_string(),
                ),
            );
        }
        if (self.availability == BrowserSessionAuthorityAvailability::Available
            && self.summary.unknown_browser_count != 0)
            || (self.availability == BrowserSessionAuthorityAvailability::Unknown
                && !self.browser_verdicts.is_empty())
        {
            return Err(
                super::service_status_projection::ServiceStatusProjectionError::InvalidAuthority(
                    "browserSessionAuthority availability contradicts verdict coverage".to_string(),
                ),
            );
        }
        if self.resource_pressure.correlated_process_count
            > self.resource_pressure.total_process_count
            || self.resource_pressure.candidate_count > self.resource_pressure.total_process_count
            || self.resource_pressure.protected_count > self.resource_pressure.total_process_count
            || self.resource_pressure.observed_count > self.resource_pressure.total_process_count
        {
            return Err(
                super::service_status_projection::ServiceStatusProjectionError::InvalidAuthority(
                    "browserSessionAuthority resource counts exceed totalProcessCount".to_string(),
                ),
            );
        }
        let mut ids = std::collections::BTreeSet::new();
        for verdict in &self.browser_verdicts {
            if verdict.browser_id.trim().is_empty()
                || verdict.key != verdict.browser_id
                || !ids.insert(verdict.browser_id.as_str())
            {
                return Err(
                    super::service_status_projection::ServiceStatusProjectionError::InvalidAuthority(
                        "browserSessionAuthority verdict IDs are invalid or duplicated".to_string(),
                    ),
                );
            }
            let booleans_match = match verdict.state {
                BrowserSessionAuthorityVerdictState::Viable => {
                    verdict.viable && !verdict.needs_attention
                }
                BrowserSessionAuthorityVerdictState::Attention
                | BrowserSessionAuthorityVerdictState::NonViable => {
                    !verdict.viable && verdict.needs_attention
                }
            };
            if !booleans_match {
                return Err(
                    super::service_status_projection::ServiceStatusProjectionError::InvalidAuthority(
                        "browserSessionAuthority verdict booleans contradict state".to_string(),
                    ),
                );
            }
        }
        Ok(())
    }
}

pub(crate) fn browser_session_authority_snapshot(
    state: &ServiceState,
) -> BrowserSessionAuthoritySnapshot {
    browser_session_authority_snapshot_from_resources(
        state,
        service_resource_authority_snapshot(state),
    )
}

pub(crate) fn browser_session_authority_snapshot_from_resources(
    state: &ServiceState,
    resources: ResourceAuthoritySnapshot,
) -> BrowserSessionAuthoritySnapshot {
    if !state.browsers.is_empty()
        && resources.resources.is_empty()
        && !resources.collection_warnings.is_empty()
    {
        return unavailable_browser_session_authority(state.browsers.len());
    }
    let resource_pressure = resource_pressure(&resources);
    let candidate_reasons_by_browser = candidate_reasons_by_browser(&resources);
    let collection_partial = !resources.collection_warnings.is_empty();
    let evidence_by_browser = resources
        .resources
        .iter()
        .filter_map(|resource| resource.correlation.browser_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    let mut browser_verdicts = state
        .browsers
        .iter()
        .filter_map(|(browser_id, browser)| {
            if collection_partial
                && !evidence_by_browser.contains(browser_id.as_str())
                && !browser_health_non_viable(browser.health)
            {
                return None;
            }
            let mut reasons = Vec::new();
            if let Some(candidate_reasons) = candidate_reasons_by_browser.get(browser_id) {
                reasons.push("cleanup_candidate_process_correlates_to_browser".to_string());
                reasons.extend(candidate_reasons.iter().cloned());
            }
            if browser_health_non_viable(browser.health) {
                reasons.push(format!(
                    "browser_health_{}",
                    browser_health_label(browser.health)
                ));
            }
            let verdict_state = if reasons
                .iter()
                .any(|reason| reason == "cleanup_candidate_process_correlates_to_browser")
                || browser_health_non_viable(browser.health)
            {
                BrowserSessionAuthorityVerdictState::NonViable
            } else if browser.pid.is_none() && browser_health_expects_process(browser.health) {
                reasons.push("live_browser_missing_pid".to_string());
                BrowserSessionAuthorityVerdictState::Attention
            } else {
                BrowserSessionAuthorityVerdictState::Viable
            };
            Some(BrowserSessionAuthorityVerdict {
                key: browser_id.clone(),
                browser_id: browser_id.clone(),
                state: verdict_state,
                viable: verdict_state == BrowserSessionAuthorityVerdictState::Viable,
                needs_attention: verdict_state != BrowserSessionAuthorityVerdictState::Viable,
                reasons,
            })
        })
        .collect::<Vec<_>>();
    browser_verdicts.sort_by(|left, right| left.browser_id.cmp(&right.browser_id));

    let summary = BrowserSessionAuthoritySummary {
        modeled_browser_count: state.browsers.len(),
        viable_browser_count: browser_verdicts
            .iter()
            .filter(|verdict| verdict.state == BrowserSessionAuthorityVerdictState::Viable)
            .count(),
        attention_browser_count: browser_verdicts
            .iter()
            .filter(|verdict| verdict.state == BrowserSessionAuthorityVerdictState::Attention)
            .count(),
        non_viable_browser_count: browser_verdicts
            .iter()
            .filter(|verdict| verdict.state == BrowserSessionAuthorityVerdictState::NonViable)
            .count(),
        unknown_browser_count: state.browsers.len().saturating_sub(browser_verdicts.len()),
    };

    BrowserSessionAuthoritySnapshot {
        schema_version: 1,
        availability: if resources.collection_warnings.is_empty() {
            BrowserSessionAuthorityAvailability::Available
        } else if browser_verdicts.is_empty() {
            BrowserSessionAuthorityAvailability::Unknown
        } else {
            BrowserSessionAuthorityAvailability::Partial
        },
        summary,
        resource_pressure,
        browser_verdicts,
    }
}

pub(crate) fn unavailable_browser_session_authority(
    modeled_browser_count: usize,
) -> BrowserSessionAuthoritySnapshot {
    BrowserSessionAuthoritySnapshot {
        schema_version: 1,
        availability: BrowserSessionAuthorityAvailability::Unknown,
        summary: BrowserSessionAuthoritySummary {
            modeled_browser_count,
            unknown_browser_count: modeled_browser_count,
            ..BrowserSessionAuthoritySummary::default()
        },
        resource_pressure: BrowserSessionResourcePressure {
            state: BrowserSessionResourcePressureState::Unknown,
            reasons: vec!["process_inventory_unavailable".to_string()],
            ..BrowserSessionResourcePressure::default()
        },
        browser_verdicts: Vec::new(),
    }
}

fn resource_pressure(resources: &ResourceAuthoritySnapshot) -> BrowserSessionResourcePressure {
    let observed_unowned_agent_browser_process_count = resources
        .resources
        .iter()
        .filter(|resource| {
            resource.kind == ResourceKind::AgentBrowser
                && resource.disposition == ResourceDisposition::Observed
                && resource.correlation.browser_id.is_none()
                && resource
                    .reasons
                    .iter()
                    .any(|reason| reason == "agent_browser_process_unowned_by_service_state")
        })
        .count();
    let mut reasons = Vec::new();
    if resources.summary.candidate_count > 0 {
        reasons.push("cleanup_candidates_present".to_string());
    }
    if observed_unowned_agent_browser_process_count > 0 {
        reasons.push("unowned_agent_browser_processes_observed".to_string());
    }
    if !resources.collection_warnings.is_empty() {
        reasons.push("process_collection_warnings_present".to_string());
    }
    BrowserSessionResourcePressure {
        state: if reasons.is_empty() {
            BrowserSessionResourcePressureState::Clear
        } else {
            BrowserSessionResourcePressureState::Pressure
        },
        total_process_count: resources.summary.total_processes,
        correlated_process_count: resources.summary.correlated_processes,
        candidate_count: resources.summary.candidate_count,
        protected_count: resources.summary.protected_count,
        observed_count: resources.summary.observed_count,
        observed_unowned_agent_browser_process_count,
        candidate_rss_bytes: resources.summary.candidate_rss_bytes,
        total_rss_bytes: resources.summary.total_rss_bytes,
        reasons,
    }
}

fn candidate_reasons_by_browser(
    resources: &ResourceAuthoritySnapshot,
) -> BTreeMap<String, Vec<String>> {
    let mut reasons_by_browser: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for resource in &resources.resources {
        if resource.disposition != ResourceDisposition::Candidate {
            continue;
        }
        let Some(browser_id) = resource.correlation.browser_id.as_ref() else {
            continue;
        };
        reasons_by_browser
            .entry(browser_id.clone())
            .or_default()
            .extend(resource.reasons.iter().cloned());
    }
    reasons_by_browser
}

fn browser_health_non_viable(health: BrowserHealth) -> bool {
    matches!(
        health,
        BrowserHealth::NotStarted
            | BrowserHealth::ProcessExited
            | BrowserHealth::Closing
            | BrowserHealth::Faulted
            | BrowserHealth::Unreachable
            | BrowserHealth::CdpDisconnected
            | BrowserHealth::Degraded
    )
}

fn browser_health_expects_process(health: BrowserHealth) -> bool {
    !browser_health_non_viable(health)
}

fn browser_health_label(health: BrowserHealth) -> &'static str {
    match health {
        BrowserHealth::NotStarted => "not_started",
        BrowserHealth::Launching => "launching",
        BrowserHealth::Ready => "ready",
        BrowserHealth::Unreachable => "unreachable",
        BrowserHealth::CdpDisconnected => "cdp_disconnected",
        BrowserHealth::Reconnecting => "reconnecting",
        BrowserHealth::ProcessExited => "process_exited",
        BrowserHealth::Closing => "closing",
        BrowserHealth::Faulted => "faulted",
        BrowserHealth::Degraded => "degraded",
    }
}

#[cfg(test)]
mod tests {
    use super::super::service_model::{BrowserHost, BrowserProcess};
    use super::super::service_resources::{
        service_resource_authority_snapshot_from_samples, ProcessSample,
    };
    use super::*;

    fn sample(pid: u32, command: &[&str], age_seconds: Option<u64>) -> ProcessSample {
        ProcessSample {
            pid,
            command: command.iter().map(|value| value.to_string()).collect(),
            executable: command.first().map(|value| value.to_string()),
            age_seconds,
            rss_bytes: Some(10),
            ..ProcessSample::default()
        }
    }

    #[test]
    fn browser_session_authority_reports_unowned_agent_browser_pressure() {
        let state = ServiceState::default();
        let resources = service_resource_authority_snapshot_from_samples(
            &state,
            vec![sample(101, &["agent-browser", "daemon"], Some(3600))],
            Vec::new(),
        );

        let authority = browser_session_authority_snapshot_from_resources(&state, resources);

        assert_eq!(
            authority.resource_pressure.state,
            BrowserSessionResourcePressureState::Pressure
        );
        assert_eq!(
            authority
                .resource_pressure
                .observed_unowned_agent_browser_process_count,
            1
        );
        assert!(authority
            .resource_pressure
            .reasons
            .contains(&"unowned_agent_browser_processes_observed".to_string()));
    }

    #[test]
    fn browser_session_authority_marks_process_exited_browser_non_viable() {
        let mut state = ServiceState::default();
        state.browsers.insert(
            "browser-dead".to_string(),
            BrowserProcess {
                id: "browser-dead".to_string(),
                host: BrowserHost::LocalHeaded,
                health: BrowserHealth::ProcessExited,
                ..BrowserProcess::default()
            },
        );
        let resources =
            service_resource_authority_snapshot_from_samples(&state, Vec::new(), Vec::new());

        let authority = browser_session_authority_snapshot_from_resources(&state, resources);

        assert_eq!(authority.summary.non_viable_browser_count, 1);
        assert_eq!(authority.browser_verdicts[0].browser_id, "browser-dead");
        assert_eq!(
            authority.browser_verdicts[0].state,
            BrowserSessionAuthorityVerdictState::NonViable
        );
        assert_eq!(authority.browser_verdicts[0].viable, false);
    }

    #[test]
    fn browser_session_authority_keeps_ready_browser_viable_without_pressure() {
        let mut state = ServiceState::default();
        state.browsers.insert(
            "browser-ready".to_string(),
            BrowserProcess {
                id: "browser-ready".to_string(),
                host: BrowserHost::LocalHeaded,
                health: BrowserHealth::Ready,
                pid: Some(202),
                ..BrowserProcess::default()
            },
        );
        let resources = service_resource_authority_snapshot_from_samples(
            &state,
            vec![sample(202, &["chrome"], Some(60))],
            Vec::new(),
        );

        let authority = browser_session_authority_snapshot_from_resources(&state, resources);

        assert_eq!(
            authority.resource_pressure.state,
            BrowserSessionResourcePressureState::Clear
        );
        assert_eq!(authority.summary.viable_browser_count, 1);
        assert_eq!(
            authority.browser_verdicts[0].state,
            BrowserSessionAuthorityVerdictState::Viable
        );
    }

    #[test]
    fn browser_session_authority_keeps_uncorrelated_browser_unknown_on_partial_collection() {
        let mut state = ServiceState::default();
        for (id, pid) in [("browser-observed", 202), ("browser-unknown", 303)] {
            state.browsers.insert(
                id.to_string(),
                BrowserProcess {
                    id: id.to_string(),
                    host: BrowserHost::LocalHeaded,
                    health: BrowserHealth::Ready,
                    pid: Some(pid),
                    ..BrowserProcess::default()
                },
            );
        }
        let mut resources = service_resource_authority_snapshot_from_samples(
            &state,
            vec![sample(202, &["chrome"], Some(60))],
            Vec::new(),
        );
        resources
            .collection_warnings
            .push("process inventory was partial".to_string());

        let authority = browser_session_authority_snapshot_from_resources(&state, resources);

        assert_eq!(
            authority.availability,
            BrowserSessionAuthorityAvailability::Partial
        );
        assert_eq!(authority.summary.modeled_browser_count, 2);
        assert_eq!(authority.summary.viable_browser_count, 1);
        assert_eq!(authority.summary.unknown_browser_count, 1);
        assert_eq!(authority.browser_verdicts.len(), 1);
        authority.validate().unwrap();
    }
}
