//! Configured application routing over existing browser and Native ownership.
use std::collections::BTreeMap;

use agent_browser_service_model::{BrowserHostBinding, BrowserSessionState};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HostRegistry {
    version: u32,
    hosts: BTreeMap<String, HostConfiguration>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HostConfiguration {
    pub(crate) origin: String,
    pub(crate) provider_application: String,
}

/// The app supplies opaque identities, not a viewer URL or a private launch environment.
pub(crate) fn configured_host(binding: &BrowserHostBinding) -> Result<HostConfiguration, String> {
    binding.validate()?;
    let path = std::env::var_os("AGENT_BROWSER_HOSTS_FILE")
        .ok_or("browser_host_configuration_missing: configure AGENT_BROWSER_HOSTS_FILE")?;
    let bytes = std::fs::read(path).map_err(|_| "browser_host_configuration_unavailable")?;
    if bytes.len() > 65_536 {
        return Err("browser_host_configuration_too_large".into());
    }
    let mut registry: HostRegistry =
        serde_json::from_slice(&bytes).map_err(|_| "browser_host_configuration_invalid")?;
    if registry.version != 1 {
        return Err("browser_host_configuration_version_unsupported".into());
    }
    let host = registry
        .hosts
        .remove(&binding.host_id)
        .ok_or("browser_host_not_configured")?;
    validate_host(&host)?;
    Ok(host)
}

fn validate_host(host: &HostConfiguration) -> Result<(), String> {
    let url = super::browser_session_remote_view::public_origin(&host.origin)?;
    if url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || host.provider_application.is_empty()
        || host.provider_application.len() > 64
        || !host
            .provider_application
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return Err("browser_host_configuration_invalid".into());
    }
    Ok(())
}

/// The configured provider application remains the assignment's resource owner.
/// A host name never grants cross-application access to another desktop.
pub(crate) fn validate_provider(binding: &BrowserHostBinding) -> Result<(), String> {
    let configured = configured_host(binding)?;
    let application = std::env::var("AGENT_BROWSER_REMOTE_VIEW_APPLICATION")
        .unwrap_or_else(|_| "agent-browser".into());
    if configured.provider_application != application {
        return Err("browser_host_provider_mismatch: use the host's configured Remote View application context".into());
    }
    Ok(())
}

pub(crate) fn handoff_url(binding: &BrowserHostBinding, id: &str) -> Result<String, String> {
    let host = configured_host(binding)?;
    uuid::Uuid::parse_str(id).map_err(|_| "browser_host_handoff_identity_invalid")?;
    let origin = super::browser_session_remote_view::public_origin(&host.origin)?;
    origin
        .join(&format!("/agent-browser/handoff/{id}"))
        .map(|url| url.to_string())
        .map_err(|_| "browser_host_handoff_url_invalid".into())
}

/// Compact metadata-only projection. It does not focus, acquire, reconcile,
/// launch, capture, or assert that a recorded process is a currently ready viewer.
pub(crate) fn snapshot(state: &BrowserSessionState, task_id: &str) -> Result<Value, String> {
    let binding = state
        .host_bindings
        .get(task_id)
        .ok_or("browser_host_task_not_found")?;
    let task = state.sessions.get(task_id);
    let browser = task.and_then(|task| state.browsers.get(&task.browser_id));
    Ok(json!({
        "version":1,"taskId":task_id,"hostId":binding.request.host_id,
        "participantId":binding.request.participant_id,"sessionId":binding.request.session_id,
        "browserId":task.map(|task| &task.browser_id),"profileId":task.map(|task| &task.profile_id),
        "tabId":task.and_then(|task| task.current_tab_id.as_ref()),
        "desktop":{"desktopId":binding.desktop.desktop_id,"generation":binding.desktop.generation},
        "taskState":if task.is_some() {"recorded"} else {"ended"},
        "browserState":if browser.is_some() {"unverified"} else {"absent_from_state"},
        "viewerState":"unverified","controllerState":task.map(|task|super::browser_host_control::metadata(&task.browser_id)).transpose()?,"accountState":"unverified",
        "capabilities":{"placement":true,"hostedHandoff":true,"metadataRead":true,"statusEvents":true,"attachment":false,"relocation":false}
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_routing_rejects_embedded_credentials_paths_and_redirect_material() {
        for origin in [
            "http://host.test",
            "https://user:pass@host.test",
            "https://host.test/path",
            "https://host.test/?redirect=elsewhere",
            "https://host.test/#fragment",
        ] {
            assert!(
                validate_host(&HostConfiguration {
                    origin: origin.into(),
                    provider_application: "bots".into()
                })
                .is_err(),
                "{origin}"
            );
        }
        assert!(validate_host(&HostConfiguration {
            origin: "https://bots.example.test".into(),
            provider_application: "codex-bots".into()
        })
        .is_ok());
    }
}
