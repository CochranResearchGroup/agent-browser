//! Explicit attachment borrows a current app browser without lifecycle custody.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserHostIdentity {
    pub host_id: String,
    pub participant_id: String,
    pub session_id: String,
}
impl From<&BrowserHostBinding> for BrowserHostIdentity {
    fn from(value: &BrowserHostBinding) -> Self {
        Self {
            host_id: value.host_id.clone(),
            participant_id: value.participant_id.clone(),
            session_id: value.session_id.clone(),
        }
    }
}
/// Exact process, profile (provided separately), endpoint and existing page.
/// Only the original owner may change the explicit sharing list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserAttachmentRequest {
    pub version: u32,
    pub process: RecordedProcessIdentity,
    pub profile_path: String,
    pub cdp_endpoint: String,
    pub target_id: String,
    #[serde(default)]
    pub shared_with: Vec<BrowserHostIdentity>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserAttachmentObservation {
    pub launch: BrowserLaunch,
    pub assignment: Option<RemoteViewAssignmentRecord>,
    pub target_id: String,
}
/// Durable borrowed custody; target IDs here are physical CDP identities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserAppAttachment {
    pub request: BrowserAttachmentRequest,
    pub owner: BrowserHostIdentity,
    pub borrowed_tab_ids: BTreeSet<String>,
    pub profile_id: String,
    pub desktop: RemoteViewFixedDesktop,
    pub assignment: Option<RemoteViewAssignmentRecord>,
}
impl BrowserAttachmentRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.process.pid == 0
            || self.process.start_token.is_empty()
            || self
                .process
                .executable_path
                .as_ref()
                .is_none_or(|path| !std::path::Path::new(path).is_absolute())
            || self.target_id.is_empty()
            || self.target_id.len() > 512
            || self.target_id.chars().any(char::is_control)
            || self.cdp_endpoint.len() > 4096
            || self.shared_with.len() > 64
        {
            return Err("browser_attachment_identity_invalid".into());
        }
        // The runtime additionally joins the endpoint to the exact local process.
        if !self
            .cdp_endpoint
            .strip_prefix("ws://127.0.0.1:")
            .is_some_and(|suffix| {
                suffix
                    .split_once("/devtools/browser/")
                    .is_some_and(|(port, id)| {
                        port.parse::<u16>().is_ok_and(|port| port > 0)
                            && !id.is_empty()
                            && id.bytes().all(|byte| {
                                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
                            })
                    })
            })
        {
            return Err("browser_attachment_capability_unavailable: a local CDP endpoint with exact process evidence is required; keep native app tools otherwise".into());
        }
        for identity in &self.shared_with {
            for value in [
                &identity.host_id,
                &identity.participant_id,
                &identity.session_id,
            ] {
                if value.is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
                    return Err("browser_attachment_sharing_invalid".into());
                }
            }
        }
        Ok(())
    }
}

/// Observe first, then publish only logical task attribution. Never launch,
/// navigate, recover, close, move, or acquire a replacement page here.
#[allow(clippy::too_many_arguments)]
pub fn attach_app_browser_task<E: BrowserSessionEffects>(
    state: &mut BrowserSessionState,
    effects: &mut E,
    profile: &BrowserProfileCatalogEntry,
    session_name: &str,
    binding: &BrowserHostBinding,
    request: &BrowserAttachmentRequest,
    now_ms: u64,
    idle_timeout_ms: u64,
) -> Result<OpenBrowserSessionResult, String> {
    binding.validate()?;
    request.validate()?;
    if session_name.is_empty()
        || !std::path::Path::new(&profile.user_data_dir).is_absolute()
        || request.profile_path != profile.user_data_dir
    {
        return Err("browser_attachment_profile_invalid".into());
    }
    let identity = BrowserHostIdentity::from(binding);
    let previous = state
        .app_browser_attachments
        .values()
        .find(|row| row.profile_id == profile.id)
        .cloned();
    if let Some(row) = &previous {
        if row.request.process != request.process
            || row.request.cdp_endpoint != request.cdp_endpoint
        {
            return Err("browser_attachment_profile_conflict: retain the current process, attach its exact binding, or explicitly relocate; never launch the same profile twice".into());
        }
        if row.owner != identity && !row.request.shared_with.contains(&identity) {
            return Err("browser_attachment_sharing_required: the owning app must explicitly share this browser with the exact recipient".into());
        }
        if row.owner != identity && row.request.shared_with != request.shared_with {
            return Err("browser_attachment_sharing_owner_required".into());
        }
        if !binding.placement.accepts(Some(&row.desktop)) {
            return Err("browser_attachment_placement_conflict: retain current desktop or request explicit relocation; another profile is an alternative".into());
        }
    } else if state
        .browsers
        .values()
        .any(|row| row.profile_id == profile.id || row.pid == request.process.pid)
    {
        return Err("browser_attachment_owner_conflict: the current owner must explicitly share; attachment does not transfer custody".into());
    }
    let replay = state
        .sessions
        .values()
        .find(|row| row.name == session_name)
        .cloned();
    if let Some(task) = &replay {
        let retained = state
            .host_bindings
            .get(&task.id)
            .ok_or("browser_attachment_task_conflict")?;
        if task.profile_id != profile.id
            || retained.request != *binding
            || state.attached_task_targets.get(&task.id) != Some(&request.target_id)
        {
            return Err("browser_attachment_task_conflict".into());
        }
    }
    let candidates = effects
        .remote_view_placement_candidates(&binding.placement)?
        .ok_or("browser_attachment_desktop_observation_unavailable")?;
    // Attachment observes where the existing process already is. Try only
    // explicitly permitted candidates, and only after a conclusive wrong-window
    // read. An ambiguous provider/CDP read never authorizes another attempt.
    binding.placement.select(&candidates)?;
    let allowed = match &binding.placement {
        BrowserPlacement::Required {desktop} => vec![desktop],
        BrowserPlacement::Preferred {desktop, alternatives} => std::iter::once(desktop).chain(alternatives).collect(),
    };
    let mut selected = None;
    for requested in allowed {
        let Some(candidate) = candidates.iter().find(|candidate| candidate.ready
            && candidate.desktop.desktop_id==requested.desktop_id && candidate.desktop.generation==requested.generation) else {continue};
        if previous.as_ref().is_some_and(|row| row.desktop.desktop_id!=candidate.desktop.desktop_id || row.desktop.generation!=candidate.desktop.generation) {continue;}
        match effects.attach_app_browser(request,profile,&candidate.desktop) {
            Ok(observed) => {selected=Some((candidate.desktop.clone(),observed));break;},
            Err(error) if error=="browser_attachment_desktop_process_mismatch" => continue,
            Err(error) => return Err(error),
        }
    }
    let (desktop,observed)=selected.ok_or("browser_attachment_desktop_process_mismatch: none of the authorized desktops currently contains this browser")?;
    if observed.launch.process_identity.as_ref() != Some(&request.process)
        || observed.launch.pid != request.process.pid
        || observed.launch.cdp_endpoint != request.cdp_endpoint
        || observed.target_id != request.target_id
        || observed.launch.desktop.as_ref() != Some(&desktop)
    {
        return Err("browser_attachment_observation_mismatch".into());
    }
    let browser_id = observed.launch.browser_id;
    if browser_id.is_empty()
        || state.browsers.get(&browser_id).is_some_and(|row| {
            row.profile_id != profile.id
                || row.pid != request.process.pid
                || row.cdp_endpoint != request.cdp_endpoint
                || row.desktop.as_ref() != Some(&desktop)
        })
    {
        return Err("browser_attachment_observation_mismatch".into());
    }
    if let Some(row) = &previous {
        if state.app_browser_attachments.get(&browser_id) != Some(row) {
            return Err("browser_attachment_browser_identity_changed".into());
        }
    }
    let expires = now_ms
        .checked_add(idle_timeout_ms)
        .ok_or("browser_session_expiry_exhausted")?;
    let (task_id, disposition) = if let Some(task) = replay {
        (task.id, SessionRecordDisposition::Reused)
    } else {
        state.next_session_sequence = state
            .next_session_sequence
            .checked_add(1)
            .ok_or("browser_session_sequence_exhausted")?;
        (
            format!(
                "session:{session_name}:{}:{}",
                profile.id, state.next_session_sequence
            ),
            SessionRecordDisposition::Created,
        )
    };
    let tab_id = format!("borrowed:{task_id}:{}", request.target_id);
    let browser =
        state
            .browsers
            .entry(browser_id.clone())
            .or_insert_with(|| ManagedBrowserInstance {
                id: browser_id.clone(),
                profile_id: profile.id.clone(),
                pid: request.process.pid,
                cdp_endpoint: request.cdp_endpoint.clone(),
                desktop: Some(desktop.clone()),
                active_session_ids: Vec::new(),
            });
    if !browser.active_session_ids.contains(&task_id) {
        browser.active_session_ids.push(task_id.clone());
    }
    let record = state
        .app_browser_attachments
        .entry(browser_id.clone())
        .or_insert_with(|| BrowserAppAttachment {
            request: request.clone(),
            owner: identity,
            borrowed_tab_ids: BTreeSet::new(),
            profile_id: profile.id.clone(),
            desktop: desktop.clone(),
            assignment: observed.assignment.clone(),
        });
    record.request.shared_with = request.shared_with.clone();
    record.borrowed_tab_ids.insert(request.target_id.clone());
    state.shared_browser_ids.insert(browser_id.clone());
    state
        .attached_task_targets
        .insert(task_id.clone(), request.target_id.clone());
    state
        .tabs
        .entry(tab_id.clone())
        .or_insert_with(|| ManagedBrowserTab {
            id: tab_id.clone(),
            target_id: request.target_id.clone(),
            browser_id: browser_id.clone(),
            session_id: task_id.clone(),
            created_at_ms: now_ms,
            last_activity_at_ms: now_ms,
        });
    let task = state
        .sessions
        .entry(task_id.clone())
        .or_insert_with(|| ManagedBrowserSession {
            id: task_id.clone(),
            name: session_name.into(),
            profile_id: profile.id.clone(),
            browser_id: browser_id.clone(),
            created_at_ms: now_ms,
            last_activity_at_ms: now_ms,
            expires_at_ms: expires,
            current_tab_id: Some(tab_id),
        });
    task.last_activity_at_ms = now_ms;
    task.expires_at_ms = expires;
    state.host_bindings.insert(
        task_id.clone(),
        BrowserHostTaskBinding {
            request: binding.clone(),
            desktop,
        },
    );
    Ok(OpenBrowserSessionResult {
        session_id: task_id,
        session_name: session_name.into(),
        profile_id: profile.id.clone(),
        browser_id,
        disposition: SessionBrowserDisposition::Reused,
        session_disposition: disposition,
    })
}
