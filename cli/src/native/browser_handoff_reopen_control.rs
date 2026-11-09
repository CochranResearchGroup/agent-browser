//! Private, exact-target admission for a frozen explicit reconstruction. It
//! permits only its initial URL command; it never returns cooperative control.
use super::*;

fn payload(command: &Value, epoch: u64) -> Value {
    json!({"domain":"agent-browser-handoff-reopen-v1", "epoch":epoch,
        "action":command["action"], "url":command["url"], "id":command["id"],
        "tabRequestId":command["tabRequestId"], "resumeHandoffId":command["resumeHandoffId"],
        "browserId":command["browserId"], "sessionId":command["sessionId"],
        "sessionName":command["sessionName"], "targetId":command["targetId"],
        "profileId":command["profileId"]})
}

fn validate<'a>(state: &'a BrowserSessionState, command: &Value) -> Result<&'a str, String> {
    let invalid = || "browser_handoff_reopen_scope_invalid".to_string();
    let id = command["resumeHandoffId"].as_str().ok_or_else(invalid)?;
    let link = state.remote_view_tab_handoffs.get(id).ok_or_else(invalid)?;
    let choice = link.reopen.as_ref().ok_or_else(invalid)?;
    let pending = link.recreation.as_ref().ok_or_else(invalid)?;
    let request = state
        .managed_tab_requests
        .get(&choice.request_id)
        .ok_or_else(invalid)?;
    let task = state
        .sessions
        .get(command["sessionId"].as_str().ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    let tab = task
        .current_tab_id
        .as_ref()
        .and_then(|id| state.tabs.get(id))
        .ok_or_else(invalid)?;
    let intent = link.resume.as_ref().ok_or_else(invalid)?;
    let browser = state.browsers.get(&task.browser_id).ok_or_else(invalid)?;
    if pending.request_id != choice.request_id
        || request.response.is_some()
        || command["id"] != choice.request_id
        || command["tabRequestId"] != choice.request_id
        || request.session_name != task.name
        || intent.session_name != task.name
        || command["sessionName"] != task.name
        || command["browserId"] != task.browser_id
        || command["profileId"] != task.profile_id
        || command["targetId"] != tab.target_id
        || tab.session_id != task.id
        || tab.browser_id != task.browser_id
        || tab.id == link.tab_id
        || browser.profile_id != task.profile_id
        || browser.desktop.is_none()
        || (command["action"] != "navigate" && command["action"] != "get_url")
        || command["url"] != json!(intent.url)
        || (command["action"] == "navigate") != intent.url.is_some()
    {
        return Err(invalid());
    }
    Ok(&task.browser_id)
}

pub(crate) fn preflight(browser: &str, handoff: &str) -> Result<(), String> {
    let control = read(browser)?;
    if !human_handoff::permits_reopen(&control, handoff)
        || control.awaiting_observation
        || !flight_clear(&control)
    {
        return Err("browser_handoff_reopen_control_unavailable".into());
    }
    Ok(())
}

/// Called only after the owner freezes the choice and admits its new tab.
/// The private key prevents public request fields from claiming this exception.
pub(crate) fn attest_reopen_command(
    state: &BrowserSessionState,
    command: &mut Value,
) -> Result<(), String> {
    // Ordinary reconstruction needs no exception. A concurrent takeover still
    // meets the ordinary guard below; only an existing human hold is attested.
    let browser_id = command["browserId"]
        .as_str()
        .ok_or("browser_handoff_reopen_scope_invalid")?;
    if !read(browser_id)?.human {
        return Ok(());
    }
    let browser = validate(state, command)?.to_owned();
    let epoch = transaction(&path(&browser)?, |control| {
        if !human_handoff::permits_reopen(
            control,
            command["resumeHandoffId"].as_str().unwrap_or_default(),
        ) || control.awaiting_observation
            || !flight_clear(control)
        {
            return Err("browser_handoff_reopen_control_unavailable".into());
        }
        prepare_return_key(&browser)?;
        Ok(control.epoch)
    })?;
    let key = load_return_key(&browser).ok_or("browser_handoff_reopen_key_unavailable")?;
    let bytes = serde_json::to_vec(&payload(command, epoch))
        .map_err(|_| "browser_handoff_reopen_scope_invalid")?;
    command["hostControlReopen"] = json!({"epoch":epoch,"attestation":key.attest(&bytes)});
    Ok(())
}

pub(super) fn authorize(state: &BrowserSessionState, command: &Value) -> Result<(), String> {
    let invalid = || "browser_handoff_reopen_scope_invalid".to_string();
    let browser = validate(state, command)?;
    let epoch = command["hostControlReopen"]["epoch"]
        .as_u64()
        .ok_or_else(invalid)?;
    let signature = command["hostControlReopen"]["attestation"]
        .as_str()
        .ok_or_else(invalid)?;
    let key = load_return_key(browser).ok_or_else(invalid)?;
    let bytes = serde_json::to_vec(&payload(command, epoch)).map_err(|_| invalid())?;
    if !key.verify(&bytes, signature) {
        return Err(invalid());
    }
    let control = read(browser)?;
    if control.epoch != epoch
        || control.awaiting_observation
        || !flight_clear(&control)
        || !human_handoff::permits_reopen(
            &control,
            command["resumeHandoffId"].as_str().unwrap_or_default(),
        )
    {
        return Err("browser_handoff_reopen_control_unavailable".into());
    }
    Ok(())
}
