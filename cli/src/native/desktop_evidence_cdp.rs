//! Focus-preserving CDP evidence and page-trigger provider for desktop episodes.
//!
//! The provider resolves an exact service-owned tab handle, connects directly
//! to that page target without activating it, and returns only digest-bearing
//! receipts. Page HTML and screenshots remain process-local and ephemeral.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::runtime::Handle;

use super::desktop_evidence::BrowserExternalSurface;
use super::service_model::{BrowserHealth, ServiceState, ServiceTabHandle, TabLifecycle};
use agent_browser_cdp::client::CdpClient;

const CDP_HTTP_TIMEOUT: Duration = Duration::from_secs(2);
const CDP_COMMAND_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_TARGET_LIST_BYTES: usize = 1024 * 1024;
const MAX_PAGE_HTML_BYTES: usize = 1024 * 1024;
const MAX_SCREENSHOT_BASE64_BYTES: usize = 32 * 1024 * 1024;
const MAX_TRIGGER_SELECTOR_BYTES: usize = 1024;
const POST_TRIGGER_SETTLE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfiguredCdpTarget {
    pub(crate) browser_id: String,
    pub(crate) tab_id: String,
    pub(crate) target_id: String,
    endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfiguredCdpFailure {
    pub(crate) code: &'static str,
    pub(crate) detail: String,
}

impl ConfiguredCdpFailure {
    pub(crate) fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

#[cfg(test)]
impl ConfiguredCdpTarget {
    pub(crate) fn fixture(browser_id: &str) -> Self {
        Self {
            browser_id: browser_id.to_string(),
            tab_id: "tab-fixture".to_string(),
            target_id: "target-fixture".to_string(),
            endpoint: "ws://127.0.0.1:9222/devtools/browser/fixture".to_string(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ConfiguredCdpProvider {
    runtime: Handle,
}

pub(crate) trait CdpEpisodeProvider {
    fn confirm_browser_external_absent(
        &self,
        target: &ConfiguredCdpTarget,
        surface: BrowserExternalSurface,
    ) -> Result<String, ConfiguredCdpFailure>;
    fn trigger(
        &self,
        target: &ConfiguredCdpTarget,
        selector: &str,
        effect_key: &str,
    ) -> Result<String, ConfiguredCdpFailure>;
}

impl ConfiguredCdpProvider {
    pub(crate) fn new(runtime: Handle) -> Self {
        Self { runtime }
    }
}

impl CdpEpisodeProvider for ConfiguredCdpProvider {
    fn confirm_browser_external_absent(
        &self,
        target: &ConfiguredCdpTarget,
        surface: BrowserExternalSurface,
    ) -> Result<String, ConfiguredCdpFailure> {
        self.runtime
            .block_on(confirm_browser_external_absent(target, surface))
    }

    fn trigger(
        &self,
        target: &ConfiguredCdpTarget,
        selector: &str,
        effect_key: &str,
    ) -> Result<String, ConfiguredCdpFailure> {
        self.runtime
            .block_on(trigger_page_control(target, selector, effect_key))
    }
}

pub(crate) fn resolve_configured_cdp_target(
    state: &ServiceState,
    browser_id: &str,
    handle_value: &Value,
) -> Result<ConfiguredCdpTarget, ConfiguredCdpFailure> {
    let handle: ServiceTabHandle = serde_json::from_value(handle_value.clone()).map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_handle_invalid",
            "serviceTabHandle does not match the service-owned tab-handle contract",
        )
    })?;
    if !handle.valid {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_handle_stale",
            "serviceTabHandle is stale and cannot authorize paired page evidence",
        ));
    }
    if handle.browser_id != browser_id || handle.tab_id.is_empty() {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_handle_identity_mismatch",
            "serviceTabHandle does not identify the requested browser and one service tab",
        ));
    }
    let target_id = handle
        .target_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_target_missing",
                "serviceTabHandle has no current CDP target identity",
            )
        })?;
    let tab = state.tabs.get(&handle.tab_id).ok_or_else(|| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_tab_missing",
            "the service-owned tab record referenced by serviceTabHandle is missing",
        )
    })?;
    if tab.browser_id != browser_id
        || tab.target_id.as_deref() != Some(target_id)
        || !matches!(tab.lifecycle, TabLifecycle::Ready)
    {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_tab_binding_drift",
            "the service tab no longer has the exact ready browser and target binding",
        ));
    }
    let browser = state.browsers.get(browser_id).ok_or_else(|| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_browser_missing",
            "the requested service browser is missing",
        )
    })?;
    if browser.health != BrowserHealth::Ready {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_browser_not_ready",
            "the requested browser is not ready for paired CDP evidence",
        ));
    }
    if let Some(session_name) = handle.session_name.as_deref() {
        if !browser
            .active_session_ids
            .iter()
            .any(|session_id| session_id == session_name)
        {
            return Err(ConfiguredCdpFailure::new(
                "desktop_cdp_session_binding_drift",
                "serviceTabHandle session is no longer active on the requested browser",
            ));
        }
    }
    let pid = browser.pid.ok_or_else(|| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_process_identity_missing",
            "the requested browser has no current process identity",
        )
    })?;
    if state
        .browser_process_identities
        .get(browser_id)
        .is_none_or(|identity| identity.process_identity.pid != pid)
    {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_process_identity_drift",
            "the requested browser process generation is missing or stale",
        ));
    }
    let endpoint = browser
        .cdp_endpoint
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_endpoint_missing",
                "the requested browser has no service-owned CDP endpoint",
            )
        })?;
    Ok(ConfiguredCdpTarget {
        browser_id: browser_id.to_string(),
        tab_id: handle.tab_id,
        target_id: target_id.to_string(),
        endpoint: endpoint.to_string(),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CdpHttpTarget {
    id: String,
    #[serde(rename = "type")]
    target_type: String,
    web_socket_debugger_url: Option<String>,
}

async fn connect_exact_page(
    target: &ConfiguredCdpTarget,
) -> Result<CdpClient, ConfiguredCdpFailure> {
    let list_url = cdp_list_url(&target.endpoint)?;
    let client = reqwest::Client::builder()
        .timeout(CDP_HTTP_TIMEOUT)
        .build()
        .map_err(|_| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_client_unavailable",
                "could not construct the bounded CDP target client",
            )
        })?;
    let response = client.get(list_url).send().await.map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_target_inventory_unavailable",
            "the exact browser CDP target inventory could not be read",
        )
    })?;
    if !response.status().is_success() {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_target_inventory_unavailable",
            "the exact browser CDP target inventory was not ready",
        ));
    }
    let bytes = response.bytes().await.map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_target_inventory_unavailable",
            "the exact browser CDP target inventory response could not be read",
        )
    })?;
    if bytes.len() > MAX_TARGET_LIST_BYTES {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_target_inventory_oversized",
            "the browser CDP target inventory exceeded the configured evidence bound",
        ));
    }
    let targets: Vec<CdpHttpTarget> = serde_json::from_slice(&bytes).map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_target_inventory_invalid",
            "the browser CDP target inventory did not match the expected contract",
        )
    })?;
    let page = targets
        .into_iter()
        .find(|candidate| {
            candidate.id == target.target_id
                && matches!(candidate.target_type.as_str(), "page" | "webview")
        })
        .ok_or_else(|| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_target_binding_drift",
                "the exact service-owned page target disappeared",
            )
        })?;
    let websocket_url = page.web_socket_debugger_url.ok_or_else(|| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_target_unattachable",
            "the exact service-owned page target has no debugger endpoint",
        )
    })?;
    ensure_same_endpoint_authority(&target.endpoint, &websocket_url)?;
    let cdp = CdpClient::connect(&websocket_url).await.map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_target_unattachable",
            "the exact service-owned page target could not be observed",
        )
    })?;
    cdp.send_command_with_timeout("Page.enable", None, None, CDP_COMMAND_TIMEOUT)
        .await
        .map_err(|_| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_page_unavailable",
                "the exact page target did not enable bounded page observation",
            )
        })?;
    cdp.send_command_with_timeout("Runtime.enable", None, None, CDP_COMMAND_TIMEOUT)
        .await
        .map_err(|_| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_runtime_unavailable",
                "the exact page target did not enable bounded DOM observation",
            )
        })?;
    Ok(cdp)
}

async fn confirm_browser_external_absent(
    target: &ConfiguredCdpTarget,
    surface: BrowserExternalSurface,
) -> Result<String, ConfiguredCdpFailure> {
    let cdp = connect_exact_page(target).await?;
    let expression = match surface {
        BrowserExternalSurface::PasskeyChooser => passkey_absence_expression(),
    };
    let value = evaluate_value(&cdp, &expression, "paired page evidence").await?;
    let record = value.as_object().ok_or_else(|| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_absence_evidence_invalid",
            "paired page evidence did not return the expected bounded record",
        )
    })?;
    let candidate_count = record
        .get("candidateCount")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_absence_evidence_invalid",
                "paired page evidence omitted the page-owned candidate count",
            )
        })?;
    if candidate_count != 0 {
        return Err(ConfiguredCdpFailure::new(
            "desktop_page_contains_matching_surface",
            "the requested passkey chooser is represented by page-owned modal evidence, so desktop classification is not established",
        ));
    }
    let html = record.get("html").and_then(Value::as_str).ok_or_else(|| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_absence_evidence_invalid",
            "paired page evidence omitted the bounded DOM sample",
        )
    })?;
    if html.len() > MAX_PAGE_HTML_BYTES {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_dom_evidence_oversized",
            "paired page DOM evidence exceeded the configured evidence bound",
        ));
    }
    let screenshot = cdp
        .send_command_with_timeout(
            "Page.captureScreenshot",
            Some(json!({
                "format": "png",
                "fromSurface": true,
                "captureBeyondViewport": false,
            })),
            None,
            CDP_COMMAND_TIMEOUT,
        )
        .await
        .map_err(|_| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_screenshot_unavailable",
                "the exact page screenshot could not be collected for paired evidence",
            )
        })?;
    let screenshot_base64 = screenshot
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_screenshot_invalid",
                "paired CDP screenshot evidence omitted its bounded image payload",
            )
        })?;
    if screenshot_base64.len() > MAX_SCREENSHOT_BASE64_BYTES {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_screenshot_oversized",
            "paired CDP screenshot evidence exceeded the configured evidence bound",
        ));
    }
    let screenshot_bytes = BASE64_STANDARD.decode(screenshot_base64).map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_screenshot_invalid",
            "paired CDP screenshot evidence was not valid base64",
        )
    })?;
    let digest = digest_parts(&[
        target.browser_id.as_bytes(),
        target.tab_id.as_bytes(),
        target.target_id.as_bytes(),
        html.as_bytes(),
        &screenshot_bytes,
        b"candidate-count:0",
    ]);
    Ok(format!("paired-cdp-absence:{digest}"))
}

async fn trigger_page_control(
    target: &ConfiguredCdpTarget,
    selector: &str,
    effect_key: &str,
) -> Result<String, ConfiguredCdpFailure> {
    let selector = selector.trim();
    if selector.is_empty() || selector.len() > MAX_TRIGGER_SELECTOR_BYTES || selector.contains('\0')
    {
        return Err(ConfiguredCdpFailure::new(
            "desktop_trigger_selector_invalid",
            "the bounded page trigger selector is empty or outside configured limits",
        ));
    }
    let cdp = connect_exact_page(target).await?;
    let selector_json = serde_json::to_string(selector).map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_trigger_selector_invalid",
            "the bounded page trigger selector could not be encoded",
        )
    })?;
    let expression = format!(
        "(() => {{ const element = document.querySelector({selector_json}); if (!element) return {{status:'not_found'}}; const rect = element.getBoundingClientRect(); const style = getComputedStyle(element); if (rect.width <= 0 || rect.height <= 0 || style.visibility === 'hidden' || style.display === 'none' || element.disabled === true) return {{status:'not_actionable'}}; element.click(); return {{status:'clicked'}}; }})()"
    );
    let effect_digest = digest_parts(&[effect_key.as_bytes()]);
    let value = evaluate_trigger_value(&cdp, &expression, &effect_digest).await?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("invalid");
    if status != "clicked" {
        return Err(ConfiguredCdpFailure::new(
            "desktop_external_trigger_no_effect",
            "the exact page trigger target was absent or not actionable",
        ));
    }
    tokio::time::sleep(POST_TRIGGER_SETTLE).await;
    let digest = digest_parts(&[
        target.browser_id.as_bytes(),
        target.tab_id.as_bytes(),
        target.target_id.as_bytes(),
        effect_key.as_bytes(),
        selector.as_bytes(),
    ]);
    Ok(format!("cdp-page-trigger:{digest}"))
}

async fn evaluate_trigger_value(
    cdp: &CdpClient,
    expression: &str,
    effect_digest: &str,
) -> Result<Value, ConfiguredCdpFailure> {
    let response = cdp
        .send_command_with_timeout(
            "Runtime.evaluate",
            Some(json!({
                "expression": expression,
                "returnByValue": true,
                "awaitPromise": true,
            })),
            None,
            CDP_COMMAND_TIMEOUT,
        )
        .await
        .map_err(|_| {
            ConfiguredCdpFailure::new(
                "desktop_external_trigger_outcome_unknown",
                format!(
                    "the page trigger outcome is unknown after dispatch; reconcile effect {effect_digest} before retry"
                ),
            )
        })?;
    if response.get("exceptionDetails").is_some() {
        return Err(ConfiguredCdpFailure::new(
            "desktop_external_trigger_outcome_unknown",
            format!(
                "the page trigger returned an exception with an uncertain effect; reconcile effect {effect_digest} before retry"
            ),
        ));
    }
    response
        .get("result")
        .and_then(|result| result.get("value"))
        .cloned()
        .ok_or_else(|| {
            ConfiguredCdpFailure::new(
                "desktop_external_trigger_outcome_unknown",
                format!(
                    "the page trigger returned no bounded outcome; reconcile effect {effect_digest} before retry"
                ),
            )
        })
}

async fn evaluate_value(
    cdp: &CdpClient,
    expression: &str,
    label: &'static str,
) -> Result<Value, ConfiguredCdpFailure> {
    let response = cdp
        .send_command_with_timeout(
            "Runtime.evaluate",
            Some(json!({
                "expression": expression,
                "returnByValue": true,
                "awaitPromise": true,
            })),
            None,
            CDP_COMMAND_TIMEOUT,
        )
        .await
        .map_err(|_| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_evaluation_unavailable",
                format!("{label} could not be evaluated on the exact page target"),
            )
        })?;
    if response.get("exceptionDetails").is_some() {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_evaluation_failed",
            format!("{label} failed on the exact page target"),
        ));
    }
    response
        .get("result")
        .and_then(|result| result.get("value"))
        .cloned()
        .ok_or_else(|| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_evaluation_invalid",
                format!("{label} returned no bounded by-value result"),
            )
        })
}

fn passkey_absence_expression() -> String {
    format!(
        "(() => {{ const visible = element => {{ const rect = element.getBoundingClientRect(); const style = getComputedStyle(element); return rect.width > 0 && rect.height > 0 && style.visibility !== 'hidden' && style.display !== 'none'; }}; const candidates = [...document.querySelectorAll('dialog,[role=dialog],[aria-modal=true]')].filter(visible).filter(element => {{ const text = (element.innerText || element.textContent || '').toLowerCase(); const choices = element.querySelectorAll('button,[role=option],[role=listitem],input[type=radio]').length; return text.includes('passkey') && choices > 0; }}); const html = (document.documentElement?.outerHTML || '').slice(0, {MAX_PAGE_HTML_BYTES}); return {{candidateCount:candidates.length,html}}; }})()"
    )
}

fn cdp_list_url(endpoint: &str) -> Result<url::Url, ConfiguredCdpFailure> {
    let mut url = url::Url::parse(endpoint).map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_endpoint_invalid",
            "the service-owned CDP endpoint is not a valid URL",
        )
    })?;
    match url.scheme() {
        "ws" => url.set_scheme("http").map_err(|_| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_endpoint_invalid",
                "the service-owned CDP endpoint scheme could not be normalized",
            )
        })?,
        "wss" => url.set_scheme("https").map_err(|_| {
            ConfiguredCdpFailure::new(
                "desktop_cdp_endpoint_invalid",
                "the service-owned CDP endpoint scheme could not be normalized",
            )
        })?,
        "http" | "https" => {}
        _ => {
            return Err(ConfiguredCdpFailure::new(
                "desktop_cdp_endpoint_invalid",
                "the service-owned CDP endpoint uses an unsupported scheme",
            ))
        }
    }
    url.set_path("/json/list");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn ensure_same_endpoint_authority(
    endpoint: &str,
    websocket_url: &str,
) -> Result<(), ConfiguredCdpFailure> {
    let endpoint = url::Url::parse(endpoint).map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_endpoint_invalid",
            "the service-owned CDP endpoint is not a valid URL",
        )
    })?;
    let websocket = url::Url::parse(websocket_url).map_err(|_| {
        ConfiguredCdpFailure::new(
            "desktop_cdp_target_endpoint_invalid",
            "the exact page target returned an invalid debugger endpoint",
        )
    })?;
    let same_host = endpoint.host_str() == websocket.host_str()
        || endpoint.host_str().is_some_and(is_loopback_host)
            && websocket.host_str().is_some_and(is_loopback_host);
    if !same_host || endpoint.port_or_known_default() != websocket.port_or_known_default() {
        return Err(ConfiguredCdpFailure::new(
            "desktop_cdp_target_endpoint_drift",
            "the exact page target debugger endpoint left the service-owned CDP authority",
        ));
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn digest_parts(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{BrowserProcess, BrowserTab, ServiceBrowserProcessIdentity};
    use crate::process_identity::RecordedProcessIdentity;
    use std::collections::BTreeMap;

    fn ready_state() -> (ServiceState, Value) {
        let handle = ServiceTabHandle {
            browser_id: "browser-1".to_string(),
            session_name: Some("session-1".to_string()),
            tab_id: "tab-1".to_string(),
            target_id: Some("target-1".to_string()),
            valid: true,
            ..ServiceTabHandle::default()
        };
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            health: BrowserHealth::Ready,
            pid: Some(4242),
            cdp_endpoint: Some("ws://127.0.0.1:9222/devtools/browser/browser-1".to_string()),
            active_session_ids: vec!["session-1".to_string()],
            ..BrowserProcess::default()
        };
        let tab = BrowserTab {
            id: "tab-1".to_string(),
            browser_id: "browser-1".to_string(),
            target_id: Some("target-1".to_string()),
            lifecycle: TabLifecycle::Ready,
            service_tab_handle: Some(handle.clone()),
            ..BrowserTab::default()
        };
        let state = ServiceState {
            browsers: BTreeMap::from([("browser-1".to_string(), browser)]),
            tabs: BTreeMap::from([("tab-1".to_string(), tab)]),
            browser_process_identities: BTreeMap::from([(
                "browser-1".to_string(),
                ServiceBrowserProcessIdentity {
                    process_identity: RecordedProcessIdentity {
                        pid: 4242,
                        start_token: "start-1".to_string(),
                        executable_path: None,
                        browser_family: None,
                    },
                    user_data_dir: None,
                    runtime_profile: None,
                },
            )]),
            ..ServiceState::default()
        };
        (state, serde_json::to_value(handle).unwrap())
    }

    #[test]
    fn configured_target_requires_exact_live_handle_tab_and_process_binding() {
        let (state, handle) = ready_state();

        let target = resolve_configured_cdp_target(&state, "browser-1", &handle).unwrap();

        assert_eq!(target.browser_id, "browser-1");
        assert_eq!(target.tab_id, "tab-1");
        assert_eq!(target.target_id, "target-1");
    }

    #[test]
    fn configured_target_rejects_stale_or_cross_browser_handles() {
        let (state, mut handle) = ready_state();
        handle["valid"] = Value::Bool(false);
        let stale = resolve_configured_cdp_target(&state, "browser-1", &handle).unwrap_err();
        assert_eq!(stale.code, "desktop_cdp_handle_stale");

        handle["valid"] = Value::Bool(true);
        handle["browserId"] = Value::String("browser-2".to_string());
        let drift = resolve_configured_cdp_target(&state, "browser-1", &handle).unwrap_err();
        assert_eq!(drift.code, "desktop_cdp_handle_identity_mismatch");
    }

    #[test]
    fn target_inventory_url_and_authority_are_bounded_to_the_service_endpoint() {
        assert_eq!(
            cdp_list_url("ws://127.0.0.1:9222/devtools/browser/abc")
                .unwrap()
                .as_str(),
            "http://127.0.0.1:9222/json/list"
        );
        ensure_same_endpoint_authority(
            "ws://localhost:9222/devtools/browser/abc",
            "ws://127.0.0.1:9222/devtools/page/target-1",
        )
        .unwrap();
        let failure = ensure_same_endpoint_authority(
            "ws://127.0.0.1:9222/devtools/browser/abc",
            "ws://127.0.0.1:9333/devtools/page/target-1",
        )
        .unwrap_err();
        assert_eq!(failure.code, "desktop_cdp_target_endpoint_drift");
    }

    #[test]
    fn passkey_absence_probe_returns_no_raw_request_data() {
        let expression = passkey_absence_expression();
        assert!(expression.contains("candidateCount"));
        assert!(expression.contains("outerHTML"));
        assert!(!expression.contains("account"));
        assert!(!expression.contains("credential"));
    }
}
