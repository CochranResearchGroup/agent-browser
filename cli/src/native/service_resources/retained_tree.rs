//! Read-only resource correlation for descendants of an exactly observed owner.
//! This grants protection only; browser control and cleanup retain their own
//! current identity and authorization checks.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{command_arg_value, ProcessSample, ResourceCorrelation, ServiceState};
use crate::process_identity::{
    assess_process_ownership, browser_family_for_path, LegacyProfileProof, ObservedProcessIdentity,
    ProcessObservation, RuntimeProcessOwnership,
};
use crate::runtime_owner_transfer::{
    CleanupObligationState, ProfileOwnerState, RuntimeLaneLifecycleState,
};

pub(super) fn correlations(
    state: &ServiceState,
    processes: &[ProcessSample],
    boot_epoch: Option<&str>,
) -> BTreeMap<u32, ResourceCorrelation> {
    let Some(boot_epoch) = boot_epoch.filter(|epoch| epoch.starts_with("linux:")) else {
        return BTreeMap::new();
    };
    let by_pid = processes
        .iter()
        .map(|p| (p.pid, p))
        .collect::<BTreeMap<_, _>>();
    // Conflicting samples cannot establish an ancestry chain.
    if by_pid.len() != processes.len() {
        return BTreeMap::new();
    }
    let roots = state
        .browsers
        .keys()
        .filter_map(|id| verified_root(state, id, &by_pid, boot_epoch))
        .collect::<BTreeMap<_, _>>();
    let mut correlations = BTreeMap::new();
    for process in processes {
        if roots.contains_key(&process.pid) {
            continue;
        }
        let mut current = process;
        let mut seen = BTreeSet::new();
        while seen.insert(current.pid) {
            let Some(parent) = current.ppid.and_then(|pid| by_pid.get(&pid)).copied() else {
                break;
            };
            let (Some(child_start), Some(parent_start)) = (
                start_ticks(current, boot_epoch),
                start_ticks(parent, boot_epoch),
            ) else {
                break;
            };
            if child_start < parent_start
                || current.executable.is_none()
                || current.executable != parent.executable
            {
                break;
            }
            if let Some(correlation) = roots.get(&parent.pid) {
                // A separate profile launch must not inherit the parent's owner.
                let root_profile = correlation.profile_path.as_deref();
                let chain_agrees = seen.iter().all(|pid| {
                    command_arg_value(&by_pid[pid].command, "--user-data-dir")
                        .is_none_or(|path| Some(path.as_str()) == root_profile)
                });
                if chain_agrees {
                    let mut correlation = correlation.clone();
                    correlation.cdp_port = None;
                    correlations.insert(process.pid, correlation);
                }
                break;
            }
            current = parent;
        }
    }
    correlations
}

fn start_ticks(process: &ProcessSample, boot_epoch: &str) -> Option<u64> {
    let (boot, ticks) = process.start_token.as_deref()?.rsplit_once(':')?;
    (boot == boot_epoch).then(|| ticks.parse().ok()).flatten()
}

fn verified_root(
    state: &ServiceState,
    browser_id: &str,
    processes: &BTreeMap<u32, &ProcessSample>,
    boot_epoch: &str,
) -> Option<(u32, ResourceCorrelation)> {
    let browser = state.browsers.get(browser_id)?;
    if browser.health != super::BrowserHealth::Ready {
        return None;
    }
    let root = *processes.get(&browser.pid?)?;
    start_ticks(root, boot_epoch)?;
    let recorded = state.browser_process_identities.get(browser_id)?;
    let profile_path = recorded.user_data_dir.as_deref()?;
    let profile_digest =
        crate::runtime_profile::canonical_profile_identity_digest(Path::new(profile_path)).ok()?;
    let owner = state.runtime_owner_registry.owner(&profile_digest)?;
    let lifecycle = state
        .runtime_owner_registry
        .lifecycle_records
        .get(browser_id)?;
    if owner.state != ProfileOwnerState::Ready
        || owner.pending_transfer.is_some()
        || owner.browser_id != browser_id
        || owner.profile_identity_digest != profile_digest
        || recorded.process_identity.browser_family.as_deref()
            != Some(owner.browser_family.as_str())
        || lifecycle.logical_browser_id != browser_id
        || lifecycle.boot_epoch.as_deref() != Some(boot_epoch)
        || lifecycle.owner_generation != owner.owner_generation
        || lifecycle.profile_identity_digest != profile_digest
        || !matches!(
            lifecycle.lifecycle_state,
            RuntimeLaneLifecycleState::Ready | RuntimeLaneLifecycleState::Retained
        )
        || lifecycle.cleanup_obligation_state != CleanupObligationState::Owned
        || lifecycle.process_group_id.is_none()
        || lifecycle.process_group_id != root.process_group_id
        || owner.process_instance_digest
            != crate::native::runtime_lifecycle::digest_json(&recorded.process_identity).ok()?
        || lifecycle.package_launch_identity_digest.as_deref()
            != Some(
                crate::native::runtime_lifecycle::package_launch_identity_digest(
                    owner,
                    root.process_group_id,
                )
                .ok()?
                .as_str(),
            )
    {
        return None;
    }
    let observed = ObservedProcessIdentity {
        pid: root.pid,
        start_token: root.start_token.clone(),
        executable_path: root.executable.clone(),
        browser_family: browser_family_for_path(root.executable.as_deref().map(Path::new)),
        command_line: Some(root.command.clone()),
    };
    if assess_process_ownership(
        Some(&recorded.process_identity),
        ProcessObservation::Observed(observed),
        LegacyProfileProof::Unproven,
    )
    .ownership
        != RuntimeProcessOwnership::MatchingBrowser
    {
        return None;
    }
    let observed_profile = command_arg_value(&root.command, "--user-data-dir")?;
    if crate::runtime_profile::canonical_profile_identity_digest(Path::new(&observed_profile))
        .ok()?
        != profile_digest
    {
        return None;
    }
    Some((
        root.pid,
        ResourceCorrelation {
            browser_id: Some(browser_id.to_string()),
            profile_id: browser.profile_id.clone(),
            session_ids: browser.active_session_ids.clone(),
            display_allocation_id: browser.display_allocation_id.clone(),
            display_name: browser.display_name.clone(),
            profile_path: Some(observed_profile),
            ..ResourceCorrelation::default()
        },
    ))
}
