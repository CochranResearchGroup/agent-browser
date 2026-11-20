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

/// Record renewal once per frozen reconstruction, including across a later
/// explicit return. A retry acknowledges the old acceptance without taking
/// human control again or treating that receipt as current physical proof.
pub(crate) fn begin_reopened_handoff(
    browser: &str,
    id: &str,
    digest: &str,
    request: &str,
) -> Result<(), String> {
    transaction(&path(browser)?, |control| {
        begin_reopen(control, id, digest, request)
    })
}

pub(super) fn permits_reopen(control: &Control, id: &str) -> bool {
    !control.human
        || invitation(control, id).is_some_and(|previous| {
            previous.epoch == control.epoch && previous.returned_by.is_none()
        })
}

fn begin_reopen(
    control: &mut Control,
    id: &str,
    digest: &str,
    request: &str,
) -> Result<(), String> {
    if request.is_empty() || request.len() > 128 {
        return Err("browser_handoff_reopen_request_invalid".into());
    }
    if let Some(accepted) = control
        .audit
        .iter()
        .find(|event| event["handoffReopenRequestId"] == request)
    {
        return if accepted["handoffId"] == id {
            Ok(())
        } else {
            Err("browser_handoff_reopen_request_conflict".into())
        };
    }
    // A frozen explicit reconstruction may replace this same invitation's
    // closed tab while its browser and peer tabs survive. This is not an
    // automatic physical rebind or permission to take another current hold.
    if let Some(previous) = invitation(control, id).filter(|previous| {
        control.human
            && previous.epoch == control.epoch
            && previous.returned_by.is_none()
            && previous.target_digest != digest
    }) {
        control.epoch = control
            .epoch
            .checked_add(1)
            .ok_or("browser_control_epoch_exhausted")?;
        control.pause_reads = true;
        control.awaiting_observation = false;
        let next = HumanHandoff {
            target_digest: digest.into(),
            epoch: control.epoch,
            ..previous
        };
        control.audit.push(json!({"epoch":control.epoch,"operation":"human_handoff_reopened_target","humanHandoff":next}));
    } else {
        begin(control, id, digest)?;
    }
    control.audit.push(
        json!({"operation":"human_handoff_reopen_accepted", "epoch":control.epoch,
        "handoffReopenRequestId":request, "handoffId":id, "targetDigest":digest}),
    );
    Ok(())
}

fn begin(control: &mut Control, id: &str, digest: &str) -> Result<(), String> {
    if intent(control, id, digest) == "interactive" {
        return Ok(());
    }
    if control.human {
        return Err("browser_handoff_control_already_held".into());
    }
    // Interactive capability is an invitation, not evidence of human activity.
    // A Native viewer's explicit takeover claims control through the private callback.
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
        handoff.id == id && handoff.target_digest == digest && handoff.returned_by.is_none()
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

/// The authenticated Native owner calls this only after explicit Take control.
/// Opening or reconnecting an interactive-capable viewer never calls it.
pub(crate) fn take_handoff(
    browser: &str,
    id: &str,
    digest: &str,
    request: &str,
) -> Result<(), String> {
    transaction(&path(browser)?, |control| {
        claim(control, id, digest, request, true)
    })
}

/// A real Native viewer event pauses automation without asserting an explicit
/// indefinite takeover. The provider owns the qualified quiet continuation.
pub(crate) fn activity_handoff(
    browser: &str,
    id: &str,
    digest: &str,
    request: &str,
) -> Result<(), String> {
    transaction(&path(browser)?, |control| {
        claim(control, id, digest, request, false)
    })
}

pub(crate) fn return_activity(
    browser: &str,
    id: &str,
    digest: &str,
    request: &str,
) -> Result<(), String> {
    transaction(&path(browser)?, |control| {
        let (viewer, _) = viewer_request(request).ok_or("browser_activity_viewer_required")?;
        let latest = control
            .audit
            .iter()
            .rev()
            .find(|event| event["humanViewerId"] == viewer)
            .ok_or("browser_activity_claim_missing")?;
        if latest["humanViewerReturnRequestId"] == request {
            return Ok(());
        }
        if latest["humanViewerOperation"] != "activity"
            || latest["humanHandoffClaimRequestId"] != request
        {
            return Err("browser_activity_claim_promoted_or_changed".into());
        }
        relinquish(control, id, digest, request)?;
        // Quiet activity returns automation, but does not revoke this viewer's
        // interactive capability. A later real event starts a new soft pause.
        if let Some(mut handoff) = invitation(control, id) {
            handoff.returned_by = None;
            control.audit.push(
                json!({"epoch":control.epoch,"operation":"human_activity_quiet",
                "humanHandoff":handoff}),
            );
        }
        Ok(())
    })
}

pub(super) fn coordination_metadata(control: &Control) -> Value {
    let mut modes = BTreeMap::new();
    for event in &control.audit {
        if let (Some(viewer), Some(operation)) = (
            event["humanViewerId"].as_str(),
            event["humanViewerOperation"].as_str(),
        ) {
            modes.insert(viewer, operation);
        }
    }
    json!({"explicitViewerCount":modes.values().filter(|mode| **mode == "takeover").count(),
        "activityViewerCount":modes.values().filter(|mode| **mode == "activity").count(),
        "scope":"instrumented_native_viewer_input"})
}

fn viewer_request(request: &str) -> Option<(&str, &str)> {
    let (viewer, operation) = request.split_once(':')?;
    (!viewer.is_empty()
        && viewer.len() <= 64
        && !operation.is_empty()
        && viewer
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    .then_some((viewer, operation))
}

fn viewer_claims(control: &Control) -> BTreeMap<String, (String, String)> {
    let mut claims = BTreeMap::new();
    for event in &control.audit {
        let Some(viewer) = event["humanViewerId"].as_str() else {
            continue;
        };
        match event["humanViewerOperation"].as_str() {
            Some("takeover" | "activity") => {
                if let (Some(id), Some(digest)) =
                    (event["handoffId"].as_str(), event["targetDigest"].as_str())
                {
                    claims.insert(viewer.into(), (id.into(), digest.into()));
                }
            }
            Some("return") => {
                claims.remove(viewer);
            }
            _ => {}
        }
    }
    claims
}

fn claim(
    control: &mut Control,
    id: &str,
    digest: &str,
    request: &str,
    explicit: bool,
) -> Result<(), String> {
    if request.is_empty() || request.len() > 128 {
        return Err("browser_handoff_takeover_request_invalid".into());
    }
    if control
        .audit
        .iter()
        .any(|event| event["humanHandoffClaimRequestId"] == request)
    {
        return if control.human && intent(control, id, digest) == "interactive" {
            Ok(())
        } else {
            Err("browser_handoff_takeover_request_stale".into())
        };
    }
    if intent(control, id, digest) != "interactive" {
        return Err("browser_handoff_target_changed".into());
    }
    if control.human && viewer_claims(control).is_empty() {
        return Err("browser_handoff_control_already_held".into());
    }
    // A newly admitted real activity sample supersedes only the earlier soft
    // activity pause. Explicit independent viewer claims are never discarded.
    if !explicit {
        let current_viewer = viewer_request(request).map(|(viewer, _)| viewer);
        let mut latest = BTreeMap::new();
        for event in &control.audit {
            if let (Some(viewer), Some(mode)) = (
                event["humanViewerId"].as_str(),
                event["humanViewerOperation"].as_str(),
            ) {
                latest.insert(viewer.to_owned(), mode.to_owned());
            }
        }
        if current_viewer
            .is_some_and(|viewer| latest.get(viewer).is_some_and(|mode| mode == "takeover"))
        {
            return Ok(());
        }
        for (viewer, mode) in latest {
            if mode == "activity" && current_viewer != Some(viewer.as_str()) {
                control
                    .audit
                    .push(json!({"operation":"human_activity_superseded",
                    "humanViewerId":viewer,"humanViewerOperation":"return"}));
            }
        }
    }
    let mut handoff = invitation(control, id).ok_or("browser_handoff_target_changed")?;
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
    control.pause_reads = false;
    control.awaiting_observation = false;
    handoff.epoch = control.epoch;
    control.audit.push(
        json!({"epoch":control.epoch,"operation":"human_handoff_takeover",
        "humanHandoffClaimRequestId":request,"humanHandoff":handoff,
        "humanViewerId":viewer_request(request).map(|(viewer, _)| viewer),
        "humanViewerOperation":if explicit {"takeover"} else {"activity"},"handoffId":id,"targetDigest":digest}),
    );
    Ok(())
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
    if let Some((viewer, _)) = viewer_request(request) {
        if control
            .audit
            .iter()
            .any(|event| event["humanViewerReturnRequestId"] == request)
        {
            return Ok(());
        }
        let mut claims = viewer_claims(control);
        let returning_legacy = claims.is_empty() && control.human && handoff.epoch == control.epoch;
        if !returning_legacy
            && claims.remove(viewer).as_ref() != Some(&(id.to_string(), digest.to_string()))
        {
            return Err("browser_handoff_viewer_claim_missing".into());
        }
        control.epoch = control
            .epoch
            .checked_add(1)
            .ok_or("browser_control_epoch_exhausted")?;
        control.human = !claims.is_empty();
        control.pause_reads = false;
        control.awaiting_observation = !control.human;
        handoff.epoch = control.epoch;
        handoff.returned_by =
            (!claims.values().any(|(claim, _)| claim == id)).then(|| request.into());
        control.audit.push(json!({"epoch":control.epoch,"operation":"human_handoff_return",
            "humanViewerReturnRequestId":request,"humanViewerId":viewer,"humanViewerOperation":"return",
            "humanHandoff":handoff}));
        return Ok(());
    }
    if !viewer_claims(control).is_empty() {
        return Err("browser_handoff_viewer_identity_required".into());
    }
    // A proof-backed replacement can retain a returned, passive invitation.
    // A later Native-only return has no cooperative hold to clear. Require its
    // exact recorded rebind; unqualified stale holds stay fenced.
    if !control.human
        && !control.pause_reads
        && control.audit.iter().rev().any(|event| {
            matches!(
                event["operation"].as_str(),
                Some("human_handoff_mode_replacement" | "human_handoff_recovery")
            ) && event["humanHandoff"]["id"] == id
                && event["humanHandoff"]["targetDigest"] == digest
        })
    {
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
    // Returning Native input proves ownership, not fresh page contents.
    control.awaiting_observation = true;
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
    Ok(invitation(&read(browser)?, id).is_some_and(|handoff| handoff.target_digest != digest))
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
    let interactive = intent(&before, id, &original.target_digest) == "interactive";
    if !interactive && (before.human || before.pause_reads) {
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
    let mode_proof =
        crate::native::browser_session_host::browser_mode_host::completed_mode_handoff_proof(
            &transaction,
            &current,
            id,
            &original.target_digest,
        )?;
    if let Some(proof) = mode_proof {
        super::transaction(&path(&target.browser.id)?, |control| {
            if invitation(control, id).as_ref() != Some(&original)
                || control.epoch != before.epoch
                || control.human != before.human
                || control.pause_reads != before.pause_reads
            {
                return Err("browser_handoff_control_changed".into());
            }
            let mut next = original.clone();
            next.target_digest = digest.clone();
            control.audit.push(json!({"epoch":control.epoch,"operation":"human_handoff_mode_replacement","previousTargetDigest":original.target_digest,"recovery":proof,"humanHandoff":next}));
            Ok(())
        })?;
    } else if interactive {
        let proof = crate::native::browser_launch_custody_store::retained_handoff_recovery_proof(
            &transaction,
            &current,
            id,
            &original.target_digest,
        )?;
        transaction_control_rebind(&target.browser.id, &original, &digest, proof)?;
    } else if let Ok(proof) =
        crate::native::browser_launch_custody_store::retained_handoff_recovery_proof(
            &transaction,
            &current,
            id,
            &original.target_digest,
        )
    {
        // Keep a qualified returned invitation passive across ordinary Native
        // recovery. Never clear a hold or infer a missing historical mapping.
        super::transaction(&path(&target.browser.id)?, |control| {
            if invitation(control, id).as_ref() != Some(&original)
                || control.epoch != before.epoch
                || control.human
                || control.pause_reads
            {
                return Err("browser_handoff_control_changed".into());
            }
            let mut next = original.clone();
            next.target_digest = digest.clone();
            control.audit.push(json!({"epoch":control.epoch,"operation":"human_handoff_recovery","previousTargetDigest":original.target_digest,"recovery":proof,"humanHandoff":next}));
            Ok(())
        })?;
    } // Unqualified passive history retains the existing no-rebind behavior.

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
    fn opening_interactive_handoff_does_not_claim_human_control() {
        let mut state = Control::default();
        state.flights.insert("original".into(), "dispatched".into());
        begin(&mut state, "invitation", "target").unwrap();
        assert_eq!(intent(&state, "invitation", "target"), "interactive");
        assert!(!state.human);
        assert!(!state.pause_reads);
        assert_eq!(state.epoch, 0);
        assert_eq!(state.flights["original"], "dispatched");
        assert!(permits(&state, "click").is_ok());
        begin(&mut state, "invitation", "target").unwrap();
        assert_eq!(state.epoch, 0);
    }

    #[test]
    fn returning_one_viewer_preserves_another_viewers_explicit_claim() {
        let mut state = Control::default();
        begin(&mut state, "invitation", "target").unwrap();
        claim(&mut state, "invitation", "target", "viewer-a:take-a", true).unwrap();
        claim(&mut state, "invitation", "target", "viewer-b:take-b", true).unwrap();
        relinquish(&mut state, "invitation", "target", "viewer-a:return-a").unwrap();
        assert!(state.human);
        assert!(permits(&state, "click").is_err());
        assert_eq!(viewer_claims(&state).len(), 1);
        relinquish(&mut state, "invitation", "target", "viewer-b:return-b").unwrap();
        assert!(!state.human);
        assert!(state.awaiting_observation);
        assert!(permits(&state, "click").is_err());
    }

    #[test]
    fn newer_soft_viewer_activity_supersedes_only_soft_claims() {
        let mut state = Control::default();
        begin(&mut state, "invitation", "target").unwrap();
        claim(
            &mut state,
            "invitation",
            "target",
            "viewer-a:activity-a",
            false,
        )
        .unwrap();
        claim(
            &mut state,
            "invitation",
            "target",
            "viewer-b:activity-b",
            false,
        )
        .unwrap();
        assert_eq!(viewer_claims(&state).len(), 1);
        assert_eq!(coordination_metadata(&state)["activityViewerCount"], 1);
        claim(&mut state, "invitation", "target", "viewer-a:take-a", true).unwrap();
        claim(
            &mut state,
            "invitation",
            "target",
            "viewer-b:activity-c",
            false,
        )
        .unwrap();
        assert_eq!(coordination_metadata(&state)["explicitViewerCount"], 1);
        assert_eq!(coordination_metadata(&state)["activityViewerCount"], 1);
        assert!(state.human);
    }

    #[test]
    fn explicit_reopen_rebinds_its_own_hold_without_taking_another_hold() {
        let mut state = Control::default();
        begin(&mut state, "same-link", "closed-target").unwrap();
        claim(&mut state, "same-link", "closed-target", "takeover-1", true).unwrap();
        let epoch = state.epoch;
        begin_reopen(&mut state, "same-link", "reopened-target", "reopen-request").unwrap();
        assert_eq!(state.epoch, epoch + 1);
        assert_eq!(
            intent(&state, "same-link", "reopened-target"),
            "interactive"
        );
        assert_eq!(intent(&state, "same-link", "closed-target"), "view");
        let before = serde_json::to_value(&state).unwrap();
        assert!(begin_reopen(
            &mut state,
            "another-link",
            "another-target",
            "another-request"
        )
        .is_err());
        assert_eq!(serde_json::to_value(&state).unwrap(), before);
    }

    #[test]
    fn reopen_retry_after_return_never_reasserts_human_control() {
        let mut state = Control::default();
        begin_reopen(&mut state, "same-link", "target", "reopen-request").unwrap();
        relinquish(&mut state, "same-link", "target", "human-return").unwrap();
        let returned = serde_json::to_value(&state).unwrap();
        let mut restarted: Control = serde_json::from_value(returned.clone()).unwrap();
        begin_reopen(&mut restarted, "same-link", "target", "reopen-request").unwrap();
        assert_eq!(serde_json::to_value(&restarted).unwrap(), returned);
        assert_eq!(intent(&restarted, "same-link", "target"), "view");
        assert!(begin_reopen(&mut restarted, "another-link", "target", "reopen-request").is_err());
        begin_reopen(&mut restarted, "same-link", "target", "new-reopen-request").unwrap();
        assert_eq!(intent(&restarted, "same-link", "target"), "interactive");
    }

    #[test]
    fn explicit_takeover_pauses_same_browser_and_return_is_idempotent() {
        let mut state = Control::default();
        state
            .flights
            .insert("pending-operation".into(), "dispatched".into());
        begin(&mut state, "invitation", "target").unwrap();
        assert_eq!(intent(&state, "invitation", "target"), "interactive");
        claim(&mut state, "invitation", "target", "takeover-1", true).unwrap();
        assert_eq!(state.flights["pending-operation"], "uncertain_in_flight");
        assert_eq!(intent(&state, "passive", "target"), "view");
        assert_eq!(intent(&state, "invitation", "replacement"), "view");
        assert!(permits(&state, "click").is_err());
        assert!(permits(&state, "url").is_ok());
        let epoch = state.epoch;
        begin(&mut state, "invitation", "target").unwrap();
        assert_eq!(state.epoch, epoch);
        let serialized = serde_json::to_vec(&state).unwrap();
        let mut state: Control = serde_json::from_slice(&serialized).unwrap();
        relinquish(&mut state, "invitation", "target", "return-1").unwrap();
        assert_eq!(intent(&state, "invitation", "target"), "view");
        assert!(!state.human);
        assert_eq!(state.flights["pending-operation"], "uncertain_in_flight");
        assert!(state.awaiting_observation);
        assert!(permits(&state, "click").is_err());
        assert!(permits(&state, "url").is_ok());
        relinquish(&mut state, "invitation", "target", "return-1").unwrap();
        assert_eq!(state.epoch, epoch + 1);
        begin(&mut state, "next-invitation", "target").unwrap();
        relinquish(&mut state, "invitation", "target", "return-1").unwrap();
        assert_eq!(intent(&state, "next-invitation", "target"), "interactive");
    }
}
