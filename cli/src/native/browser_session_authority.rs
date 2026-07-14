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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct BrowserSessionResourcePressure {
    pub(crate) state: String,
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
    pub(crate) state: String,
    pub(crate) viable: bool,
    pub(crate) needs_attention: bool,
    pub(crate) reasons: Vec<String>,
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
    let resource_pressure = resource_pressure(&resources);
    let candidate_reasons_by_browser = candidate_reasons_by_browser(&resources);
    let mut browser_verdicts = state
        .browsers
        .iter()
        .map(|(browser_id, browser)| {
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
            let state = if reasons
                .iter()
                .any(|reason| reason == "cleanup_candidate_process_correlates_to_browser")
                || browser_health_non_viable(browser.health)
            {
                "non_viable"
            } else if browser.pid.is_none() && browser_health_expects_process(browser.health) {
                reasons.push("live_browser_missing_pid".to_string());
                "attention"
            } else {
                "viable"
            };
            BrowserSessionAuthorityVerdict {
                key: browser_id.clone(),
                browser_id: browser_id.clone(),
                state: state.to_string(),
                viable: state == "viable",
                needs_attention: state != "viable",
                reasons,
            }
        })
        .collect::<Vec<_>>();
    browser_verdicts.sort_by(|left, right| left.browser_id.cmp(&right.browser_id));

    let summary = BrowserSessionAuthoritySummary {
        modeled_browser_count: browser_verdicts.len(),
        viable_browser_count: browser_verdicts
            .iter()
            .filter(|verdict| verdict.state == "viable")
            .count(),
        attention_browser_count: browser_verdicts
            .iter()
            .filter(|verdict| verdict.state == "attention")
            .count(),
        non_viable_browser_count: browser_verdicts
            .iter()
            .filter(|verdict| verdict.state == "non_viable")
            .count(),
    };

    BrowserSessionAuthoritySnapshot {
        schema_version: 1,
        summary,
        resource_pressure,
        browser_verdicts,
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
            "clear".to_string()
        } else {
            "pressure".to_string()
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

        assert_eq!(authority.resource_pressure.state, "pressure");
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
        assert_eq!(authority.browser_verdicts[0].state, "non_viable");
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

        assert_eq!(authority.resource_pressure.state, "clear");
        assert_eq!(authority.summary.viable_browser_count, 1);
        assert_eq!(authority.browser_verdicts[0].state, "viable");
    }
}
