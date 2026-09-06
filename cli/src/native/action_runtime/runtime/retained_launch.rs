//! Repair launch projections erased by older shared-profile CDP attachment.
//! This restores descriptive fields only, after existing owner authorization
//! and fresh process/display proof. It never creates or promotes authority.

use crate::native::runtime_lifecycle::{digest_json, RuntimeLifecycleAuthority};
use crate::native::service_model::{BrowserHost, BrowserProcess, DisplayAllocation, ServiceState};
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn failure(predicate: &str) -> String {
    format!("retained_launch_recovery_failed: predicate={predicate}")
}

fn allocation_for_recovery(
    state: &ServiceState,
    browser: &BrowserProcess,
    session: &str,
) -> Result<DisplayAllocation, String> {
    let boot = crate::process_identity::current_boot_epoch();
    if boot.is_none() || browser.boot_epoch != boot {
        return Err(failure("current_boot"));
    }
    if browser.profile_id.is_none()
        || !browser.active_session_ids.iter().any(|id| id == session)
        || !state.sessions.get(session).is_some_and(|row| {
            row.profile_id == browser.profile_id && row.browser_ids.contains(&browser.id)
        })
    {
        return Err(failure("reciprocal_profile_session"));
    }
    let candidates = state
        .display_allocations
        .values()
        .filter(|allocation| {
            allocation.boot_epoch == boot
                && allocation.host == Some(BrowserHost::RemoteHeaded)
                && allocation.owner_browser_id.as_deref() == Some(browser.id.as_str())
                && allocation.owner_session_id.as_deref() == Some(session)
                && allocation.profile_id == browser.profile_id
                && Some(allocation.display_isolation.as_str())
                    == browser.display_isolation.as_deref()
                && allocation
                    .display_name
                    .as_deref()
                    .is_some_and(|name| !name.trim().is_empty())
                && allocation
                    .pid_hints
                    .as_ref()
                    .and_then(|hints| hints.get("browserPid"))
                    .and_then(Value::as_u64)
                    == browser.pid.map(u64::from)
        })
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [allocation] => Ok((*allocation).clone()),
        [] => Err(failure("retained_allocation_missing")),
        _ => Err(failure("retained_allocation_ambiguous")),
    }
}

pub(super) async fn recover(
    session: &str,
    host: BrowserHost,
    command: &Value,
) -> Result<(), String> {
    if host != BrowserHost::RemoteHeaded {
        return Ok(());
    }
    let id = super::capability::service_browser_id(session);
    // Recovery is confined to an explicitly selected existing browser route.
    if command.get("browserId").and_then(Value::as_str) != Some(id.as_str())
        || command.get("sessionName").and_then(Value::as_str) != Some(session)
    {
        return Ok(());
    }
    let repository = LockedServiceStateRepository::default_json()?;
    let snapshot = repository.load_snapshot()?;
    let Some(browser) = snapshot.browsers.get(&id).filter(|browser| {
        browser.host == BrowserHost::AttachedExisting
            && browser.display_name.is_none()
            && browser.display_allocation_id.is_none()
            && browser.display_isolation.is_some()
    }) else {
        return Ok(());
    };
    let allocation = allocation_for_recovery(&snapshot, browser, session)?;
    let mut binding =
        crate::runtime_owner_transfer::owner_binding_for_session(&repository, session)?
            .ok_or_else(|| failure("current_owner_missing"))?;
    let authority = RuntimeLifecycleAuthority::new(&repository);
    authority.authorize_effect(&mut binding)?;
    if binding.claim.logical_browser_id != id || binding.claim.daemon_session_route != session {
        return Err(failure("current_owner_route"));
    }
    let owner = snapshot
        .runtime_owner_registry
        .owner(&binding.claim.profile_identity_digest)
        .ok_or_else(|| failure("current_owner_missing"))?;
    let process = browser
        .pid
        .and_then(|pid| crate::process_identity::capture_process_identity(pid, None, None))
        .ok_or_else(|| failure("process_observation"))?;
    if digest_json(&process)? != binding.claim.process_instance_digest {
        return Err(failure("process_identity"));
    }
    let endpoint = browser
        .cdp_endpoint
        .as_deref()
        .ok_or_else(|| failure("endpoint_missing"))?;
    if format!("{:x}", Sha256::digest(endpoint.as_bytes())) != owner.cdp_endpoint_identity_digest {
        return Err(failure("endpoint_identity"));
    }
    let pid = process.pid;
    let display = allocation
        .display_name
        .clone()
        .expect("selected allocation has a display");
    let observed = tokio::task::spawn_blocking(move || {
        crate::native::x11_scene::observe_browser_scene(pid, &display)
    });
    // Scene observation reads process/window geometry, never titles or pixels.
    tokio::time::timeout(std::time::Duration::from_secs(2), observed)
        .await
        .map_err(|_| failure("display_observation_timeout"))?
        .map_err(|_| failure("display_observer_failed"))?
        .map_err(|_| failure("process_window_on_display"))?;
    if crate::process_identity::capture_process_identity(pid, None, None).as_ref() != Some(&process)
    {
        return Err(failure("process_changed_during_observation"));
    }
    authority.authorize_effect(&mut binding)?;
    repository.mutate(|current| {
        if current.browsers.get(&id) != Some(browser)
            || current.sessions.get(session) != snapshot.sessions.get(session)
            || current.display_allocations != snapshot.display_allocations
            || current.runtime_owner_registry != snapshot.runtime_owner_registry
            || current.browser_process_identities.get(&id)
                != snapshot.browser_process_identities.get(&id)
        {
            return Err(failure("projection_changed_during_observation"));
        }
        let repaired = current
            .browsers
            .get_mut(&id)
            .expect("compared browser remains present");
        repaired.host = BrowserHost::RemoteHeaded;
        repaired.display_name = allocation.display_name.clone();
        repaired.display_allocation_id = Some(allocation.id.clone());
        let repaired = repaired.clone();
        crate::native::service_health::record_browser_launch_recorded_event(
            current,
            &id,
            Some(browser),
            &repaired,
            None,
        );
        // Keep allocation readiness and isolation claims unchanged: finding a
        // window proves location, not exclusivity or operator presentation.
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::BrowserSession;
    use serde_json::json;

    fn fixture() -> (ServiceState, BrowserProcess) {
        let boot = crate::process_identity::current_boot_epoch();
        let browser = BrowserProcess {
            id: "session:retained".to_string(),
            boot_epoch: boot.clone(),
            host: BrowserHost::AttachedExisting,
            profile_id: Some("profile".to_string()),
            active_session_ids: vec!["retained".to_string()],
            pid: Some(1234),
            display_isolation: Some("private_virtual_display".to_string()),
            ..BrowserProcess::default()
        };
        let mut state = ServiceState::default();
        state.sessions.insert(
            "retained".to_string(),
            BrowserSession {
                id: "retained".to_string(),
                profile_id: browser.profile_id.clone(),
                browser_ids: vec![browser.id.clone()],
                ..BrowserSession::default()
            },
        );
        state.display_allocations.insert(
            "display".to_string(),
            DisplayAllocation {
                id: "display".to_string(),
                boot_epoch: boot,
                host: Some(BrowserHost::RemoteHeaded),
                owner_browser_id: Some(browser.id.clone()),
                owner_session_id: Some("retained".to_string()),
                profile_id: browser.profile_id.clone(),
                display_isolation: "private_virtual_display".to_string(),
                display_name: Some(":91".to_string()),
                pid_hints: Some(json!({"browserPid":1234})),
                state: "orphaned".to_string(),
                ..DisplayAllocation::default()
            },
        );
        (state, browser)
    }

    #[test]
    fn recovery_requires_unique_current_boot_reciprocal_allocation() {
        let (state, browser) = fixture();
        assert_eq!(
            allocation_for_recovery(&state, &browser, "retained")
                .unwrap()
                .id,
            "display"
        );
        let before = state.clone();
        for field in [
            "boot",
            "browser",
            "session",
            "profile",
            "pid",
            "display",
            "isolation",
            "host",
        ] {
            let mut drifted = state.clone();
            let allocation = drifted.display_allocations.get_mut("display").unwrap();
            match field {
                "boot" => allocation.boot_epoch = Some("prior-boot".to_string()),
                "browser" => allocation.owner_browser_id = Some("other".to_string()),
                "session" => allocation.owner_session_id = Some("other".to_string()),
                "profile" => allocation.profile_id = Some("other".to_string()),
                "pid" => allocation.pid_hints = Some(json!({"browserPid":5678})),
                "display" => allocation.display_name = None,
                "isolation" => allocation.display_isolation = "shared_display".to_string(),
                "host" => allocation.host = Some(BrowserHost::LocalHeadless),
                _ => unreachable!(),
            }
            assert!(
                allocation_for_recovery(&drifted, &browser, "retained")
                    .unwrap_err()
                    .contains("retained_allocation_missing"),
                "{field}"
            );
        }
        let mut ambiguous = state.clone();
        let mut duplicate = ambiguous.display_allocations["display"].clone();
        duplicate.id = "duplicate".to_string();
        ambiguous
            .display_allocations
            .insert(duplicate.id.clone(), duplicate);
        assert!(allocation_for_recovery(&ambiguous, &browser, "retained")
            .unwrap_err()
            .contains("ambiguous"));
        let mut missing_link = state.clone();
        missing_link
            .sessions
            .get_mut("retained")
            .unwrap()
            .browser_ids
            .clear();
        assert!(allocation_for_recovery(&missing_link, &browser, "retained")
            .unwrap_err()
            .contains("reciprocal_profile_session"));
        assert_eq!(state, before, "candidate inspection must be read-only");
    }
}
