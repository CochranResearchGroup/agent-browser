//! Explicit human assistance shares the cooperative control journal. Viewer
//! connection/input ownership remains Remote View's responsibility.
use super::*;

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HumanHandoff {
    id: String,
    target_digest: String,
    epoch: u64,
    returned_by: Option<String>,
}

pub(crate) fn begin_handoff(browser: &str, id: &str, digest: &str) -> Result<(), String> {
    transaction(&path(browser)?, |control| begin(control, id, digest))
}

fn begin(control: &mut Control, id: &str, digest: &str) -> Result<(), String> {
    if intent(control, id, digest) == "interactive" {
        return Ok(());
    }
    if control.human {
        return Err("browser_handoff_control_already_held".into());
    }
    // Match ordinary cooperative takeover: preserve dispatched uncertainty;
    // human assistance may be needed to finish that very operation.
    for outcome in control
        .flights
        .values_mut()
        .filter(|state| state.as_str() == "dispatched")
    {
        *outcome = "uncertain_in_flight".into();
    }
    control.epoch = control
        .epoch
        .checked_add(1)
        .ok_or("browser_control_epoch_exhausted")?;
    control.human = true;
    control.awaiting_observation = false;
    control.pause_reads = true;
    control.actor = "browser_handoff".into();
    control.reason = "Explicit human assistance requested".into();
    let handoff = HumanHandoff {
        id: id.into(),
        target_digest: digest.into(),
        epoch: control.epoch,
        returned_by: None,
    };
    control
        .audit
        .push(json!({"epoch":control.epoch,"operation":"human_handoff","humanHandoff":handoff}));
    Ok(())
}

// Keep extension data inside the existing extensible audit envelope so older
// cooperating runtimes can still read human/pauseReads and observe the return.
fn invitation(control: &Control, id: &str) -> Option<HumanHandoff> {
    control
        .audit
        .iter()
        .rev()
        .filter_map(|event| event.get("humanHandoff"))
        .find(|value| value["id"].as_str() == Some(id))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
}

fn intent(control: &Control, id: &str, digest: &str) -> &'static str {
    if invitation(control, id).is_some_and(|handoff| {
        handoff.id == id
            && handoff.target_digest == digest
            && handoff.epoch == control.epoch
            && handoff.returned_by.is_none()
            && control.human
            && control.pause_reads
    }) {
        "interactive"
    } else {
        "view"
    }
}

pub(crate) fn handoff_intent(
    browser: &str,
    id: &str,
    digest: &str,
) -> Result<&'static str, String> {
    Ok(intent(&read(browser)?, id, digest))
}

pub(crate) fn return_handoff(
    browser: &str,
    id: &str,
    digest: &str,
    request: &str,
) -> Result<(), String> {
    transaction(&path(browser)?, |control| {
        relinquish(control, id, digest, request)
    })
}

fn relinquish(control: &mut Control, id: &str, digest: &str, request: &str) -> Result<(), String> {
    let Some(mut handoff) = invitation(control, id) else {
        return Ok(());
    };
    if handoff.id != id || handoff.target_digest != digest {
        return Err("browser_handoff_target_changed".into());
    }
    if handoff.returned_by.as_deref() == Some(request) {
        return Ok(());
    }
    if intent(control, id, digest) != "interactive" {
        return Err("browser_handoff_control_changed".into());
    }
    control.epoch = control
        .epoch
        .checked_add(1)
        .ok_or("browser_control_epoch_exhausted")?;
    control.human = false;
    control.pause_reads = false;
    // The private callback freshly rejoined the same process and target.
    // Native input is relinquished before this cooperative resume.
    control.awaiting_observation = false;
    handoff.returned_by = Some(request.into());
    control.audit.push(
        json!({"epoch":control.epoch,"operation":"human_handoff_return","requestId":request,"humanHandoff":handoff}),
    );
    Ok(())
}

/// Retain a purpose-specific invitation without promoting an existing passive
/// recipient. Publication uses the owner's normal SQLite reconciliation fences.
pub(crate) fn prepare_handoff(
    state: &BrowserSessionState,
    task_id: &str,
    interaction: &str,
    now_ms: u64,
) -> Result<agent_browser_service_model::RemoteViewTabHandoff, String> {
    let task = state
        .sessions
        .get(task_id)
        .ok_or("browser_task_not_found_after_handoff")?;
    let tab_id = task
        .current_tab_id
        .as_deref()
        .ok_or("browser_task_current_tab_missing")?;
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    let template = state
        .remote_view_tab_handoffs
        .values()
        .find(|link| {
            !link.id.starts_with("human-")
                && link.session_id == task_id
                && link.tab_id == tab_id
                && link.check_link_at(now_ms).is_ok()
        })
        .ok_or("remote_view_tab_handoff_missing")?;
    let retained = if interaction == "interactive" {
        let active = state.remote_view_tab_handoffs.values().find(|link| {
            link.session_id == task_id
                && link.tab_id == tab_id
                && link.check_link_at(now_ms).is_ok()
                && agent_browser_service_model::resolve_remote_view_tab_handoff(state, &link.id)
                    .and_then(|target| {
                        agent_browser_service_model::remote_view_native_target_digest(
                            &link.id, &target,
                        )
                    })
                    .and_then(|digest| {
                        handoff_intent(&task.browser_id, &link.id, &digest).map(str::to_owned)
                    })
                    .is_ok_and(|intent| intent == "interactive")
        });
        if let Some(link) = active {
            link.clone()
        } else {
            let mut link = template.clone();
            link.id = format!("human-{}", uuid::Uuid::new_v4());
            link.native_observation = None;
            link.view = None;
            let mut updated = state.clone();
            updated
                .remote_view_tab_handoffs
                .insert(link.id.clone(), link.clone());
            BrowserSessionSqliteStore::open(
                path.parent().ok_or("browser_runtime_directory_missing")?,
            )?
            .compare_and_save_session_state(state, &updated)?;
            link
        }
    } else {
        template.clone()
    };
    Ok(retained)
}

/// A physical replacement never promotes a passive or different invitation.
pub(crate) fn recovery_rebind_pending(
    browser: &str,
    id: &str,
    digest: &str,
) -> Result<bool, String> {
    Ok(invitation(&read(browser)?, id)
        .is_some_and(|handoff| handoff.returned_by.is_none() && handoff.target_digest != digest))
}

/// The SQLite writer fence stays held through the existing atomic control-journal
/// replacement. No aggregate fields change and neither physical input nor control
/// release occurs. A lost reply is idempotent against the same current digest.
pub(crate) fn reconcile_recovered_handoff(
    database: &Path,
    expected: &BrowserSessionState,
    id: &str,
    now_ms: u64,
) -> Result<(), String> {
    use crate::native::browser_session_store::{load_optional_document, SESSION_STATE_DOCUMENT};
    use agent_browser_service_model::{
        remote_view_native_target_digest, resolve_remote_view_tab_handoff,
        BROWSER_SESSION_STATE_SCHEMA_V1,
    };
    let target = resolve_remote_view_tab_handoff(expected, id)?;
    let digest = remote_view_native_target_digest(id, &target)?;
    let before = read(&target.browser.id)?;
    let Some(original) = invitation(&before, id) else {
        return Ok(());
    };
    if original.target_digest == digest {
        return Ok(());
    }
    if intent(&before, id, &original.target_digest) != "interactive" {
        return Err("browser_handoff_control_changed".into());
    }
    let mut connection = rusqlite::Connection::open_with_flags(
        database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .map_err(|_| "browser_handoff_recovery_store_unavailable")?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "browser_handoff_recovery_store_unavailable")?;
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|_| "browser_handoff_recovery_store_unavailable")?;
    let current: BrowserSessionState = load_optional_document::<Option<BrowserSessionState>>(
        &transaction,
        SESSION_STATE_DOCUMENT,
        BROWSER_SESSION_STATE_SCHEMA_V1,
    )?
    .ok_or("browser_handoff_recovery_state_missing")?;
    if current != *expected {
        return Err("browser_handoff_recovery_publication_conflict".into());
    }
    current
        .remote_view_tab_handoffs
        .get(id)
        .ok_or("remote_view_tab_handoff_missing")?
        .check_link_at(now_ms)?;
    let proof = crate::native::browser_launch_custody_store::retained_handoff_recovery_proof(
        &transaction,
        &current,
        id,
        &original.target_digest,
    )?;
    transaction_control_rebind(&target.browser.id, &original, &digest, proof)?;
    transaction
        .commit()
        .map_err(|_| "browser_handoff_recovery_store_unavailable".into())
}

fn transaction_control_rebind(
    browser: &str,
    expected: &HumanHandoff,
    digest: &str,
    proof: Value,
) -> Result<(), String> {
    transaction(&path(browser)?, |control| {
        if intent(control, &expected.id, digest) == "interactive" {
            return Ok(());
        }
        if invitation(control, &expected.id).as_ref() != Some(expected)
            || intent(control, &expected.id, &expected.target_digest) != "interactive"
        {
            return Err("browser_handoff_control_changed".into());
        }
        let mut next = expected.clone();
        next.target_digest = digest.into();
        control.audit.push(json!({"epoch":control.epoch,"operation":"human_handoff_recovery","previousTargetDigest":expected.target_digest,"recovery":proof,"humanHandoff":next}));
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invitation_pauses_same_browser_and_return_is_idempotent() {
        let mut state = Control::default();
        state
            .flights
            .insert("pending-operation".into(), "dispatched".into());
        begin(&mut state, "invitation", "target").unwrap();
        assert_eq!(intent(&state, "invitation", "target"), "interactive");
        assert_eq!(state.flights["pending-operation"], "uncertain_in_flight");
        assert_eq!(intent(&state, "passive", "target"), "view");
        assert_eq!(intent(&state, "invitation", "replacement"), "view");
        assert!(permits(&state, "click").is_err());
        assert!(permits(&state, "url").is_err());
        let epoch = state.epoch;
        begin(&mut state, "invitation", "target").unwrap();
        assert_eq!(state.epoch, epoch);
        let serialized = serde_json::to_vec(&state).unwrap();
        let mut state: Control = serde_json::from_slice(&serialized).unwrap();
        relinquish(&mut state, "invitation", "target", "return-1").unwrap();
        assert_eq!(intent(&state, "invitation", "target"), "view");
        assert!(!state.human);
        assert_eq!(state.flights["pending-operation"], "uncertain_in_flight");
        assert!(permits(&state, "click").is_ok());
        assert!(permits(&state, "url").is_ok());
        relinquish(&mut state, "invitation", "target", "return-1").unwrap();
        assert_eq!(state.epoch, epoch + 1);
        begin(&mut state, "next-invitation", "target").unwrap();
        relinquish(&mut state, "invitation", "target", "return-1").unwrap();
        assert_eq!(intent(&state, "next-invitation", "target"), "interactive");
    }
}
