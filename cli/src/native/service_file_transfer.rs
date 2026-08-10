#![allow(unused_imports)]
use super::action_runtime::common::*;
use super::action_runtime::runtime::{
    service_browser_id, validate_service_tab_handle_for_current_session, DaemonState,
    RuntimeHandoffDescriptor, TrackedRequest,
};
use super::browser_navigation::handle_reload;
use super::interaction::{
    handle_clear, handle_click, handle_dialog, handle_fill, handle_focus, handle_select,
    handle_type, handle_wait,
};
use super::network::matches_status_filter;
use super::service_diagnostics::handle_service_diagnostics;
use super::service_probe::probe_recipe_fingerprint;
use super::service_ui_action::{service_ui_caller, service_ui_current_page};
pub(crate) async fn handle_service_file_transfer(
    cmd: &Value,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let handle = cmd
        .get("serviceTabHandle")
        .and_then(Value::as_object)
        .ok_or_else(|| "file_transfer requires serviceTabHandle".to_string())?;
    validate_service_tab_handle_for_current_session(handle, &state.session_id)?;
    let transfer = cmd
        .get("fileTransfer")
        .and_then(Value::as_object)
        .ok_or_else(|| "file_transfer requires fileTransfer object".to_string())?;
    let timeout_ms = cmd
        .get("timeoutMs")
        .and_then(Value::as_u64)
        .or_else(|| transfer.get("timeoutMs").and_then(Value::as_u64))
        .ok_or_else(|| "file_transfer requires positive timeoutMs".to_string())?;
    if timeout_ms == 0 {
        return Err("file_transfer requires positive timeoutMs".to_string());
    }
    if transfer.get("upload").is_none() && transfer.get("download").is_none() {
        return Err("file_transfer requires upload or download recipe".to_string());
    }
    validate_service_file_transfer_recipe(transfer)?;
    let target_id = handle
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer requires serviceTabHandle.targetId".to_string())?;
    {
        let mgr = state
            .browser
            .as_mut()
            .ok_or_else(|| {
                "Cannot run file_transfer: target browser session is not running; request a service tab first"
                    .to_string()
            })?;
        if mgr.active_target_id().ok() != Some(target_id) {
            let _ = mgr.tab_switch_target_id(target_id).await?;
        }
    }
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let before = service_ui_current_page(state).await;
    let mut upload_result = Value::Null;
    let mut download_result = Value::Null;
    if let Some(upload) = transfer.get("upload").and_then(Value::as_object) {
        match run_service_file_upload(upload, state).await {
            Ok(result) => upload_result = result,
            Err(error) => {
                return service_file_transfer_failure(
                    cmd,
                    state,
                    ServiceFileTransferFailure {
                        handle,
                        transfer,
                        target_id,
                        observed_at: &observed_at,
                        before,
                        phase: "upload",
                        error,
                    },
                )
                .await;
            }
        }
    }
    if let Some(download) = transfer.get("download").and_then(Value::as_object) {
        match run_service_download_capture(download, state, timeout_ms).await {
            Ok(result) => download_result = result,
            Err(error) => {
                return service_file_transfer_failure(
                    cmd,
                    state,
                    ServiceFileTransferFailure {
                        handle,
                        transfer,
                        target_id,
                        observed_at: &observed_at,
                        before,
                        phase: "download",
                        error,
                    },
                )
                .await;
            }
        }
    }
    let after = service_ui_current_page(state).await;
    Ok(json!(
        { "ok" : true, "action" : "file_transfer", "observedAt" : observed_at,
        "targetId" : target_id, "tabId" : handle.get("tabId").cloned()
        .unwrap_or(Value::Null), "profileId" : handle.get("profileId").cloned()
        .unwrap_or(Value::Null), "serviceTabHandle" : cmd.get("serviceTabHandle")
        .cloned().unwrap_or(Value::Null), "traceFilter" : handle.get("traceFilter")
        .cloned().unwrap_or(Value::Null), "fileTransfer" :
        service_file_transfer_summary(transfer, timeout_ms), "before" : before,
        "after" : after, "upload" : upload_result, "download" : download_result,
        "caller" : service_ui_caller(cmd), }
    ))
}
pub(crate) struct ServiceFileTransferFailure<'a> {
    pub(crate) handle: &'a Map<String, Value>,
    pub(crate) transfer: &'a Map<String, Value>,
    pub(crate) target_id: &'a str,
    pub(crate) observed_at: &'a str,
    pub(crate) before: Value,
    pub(crate) phase: &'a str,
    pub(crate) error: String,
}
pub(crate) async fn service_file_transfer_failure(
    cmd: &Value,
    state: &mut DaemonState,
    failure: ServiceFileTransferFailure<'_>,
) -> Result<Value, String> {
    let after = service_ui_current_page(state).await;
    let diagnostics = if failure
        .transfer
        .get("includeDiagnosticsOnFailure")
        .or_else(|| cmd.get("captureEvidenceOnFailure"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        handle_service_diagnostics(cmd, state)
            .await
            .unwrap_or_else(|diagnostic_error| json!({ "ok" : false, "error" : diagnostic_error, }))
    } else {
        Value::Null
    };
    Ok(json!(
        { "ok" : false, "action" : "file_transfer", "observedAt" : failure
        .observed_at, "failedPhase" : failure.phase, "error" : failure.error,
        "targetId" : failure.target_id, "tabId" : failure.handle.get("tabId")
        .cloned().unwrap_or(Value::Null), "profileId" : failure.handle
        .get("profileId").cloned().unwrap_or(Value::Null), "serviceTabHandle" : cmd
        .get("serviceTabHandle").cloned().unwrap_or(Value::Null), "traceFilter" :
        failure.handle.get("traceFilter").cloned().unwrap_or(Value::Null),
        "fileTransfer" : service_file_transfer_summary(failure.transfer, cmd
        .get("timeoutMs").and_then(Value::as_u64).or_else(|| failure.transfer
        .get("timeoutMs").and_then(Value::as_u64)).unwrap_or(0),), "before" : failure
        .before, "after" : after, "diagnostics" : diagnostics, "caller" :
        service_ui_caller(cmd), }
    ))
}
pub(crate) fn service_file_transfer_summary(
    transfer: &Map<String, Value>,
    timeout_ms: u64,
) -> Value {
    json!(
        { "hasUpload" : transfer.get("upload").is_some(), "hasDownload" : transfer
        .get("download").is_some(), "timeoutMs" : timeout_ms, "recipeId" : transfer
        .get("recipeId").cloned().unwrap_or(Value::Null), "recipeFingerprint" :
        probe_recipe_fingerprint(transfer), }
    )
}
pub(crate) fn validate_service_file_transfer_recipe(
    transfer: &Map<String, Value>,
) -> Result<(), String> {
    if let Some(upload) = transfer.get("upload") {
        let upload = upload
            .as_object()
            .ok_or_else(|| "file_transfer upload must be an object".to_string())?;
        validate_service_file_upload_recipe(upload)?;
    }
    if let Some(download) = transfer.get("download") {
        let download = download
            .as_object()
            .ok_or_else(|| "file_transfer download must be an object".to_string())?;
        validate_service_download_recipe(download)?;
    }
    Ok(())
}
pub(crate) fn validate_service_file_upload_recipe(
    upload: &Map<String, Value>,
) -> Result<(), String> {
    if upload
        .get("selector")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_none()
        && upload
            .get("labelText")
            .or_else(|| upload.get("label"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return Err("file_transfer upload requires selector or labelText".to_string());
    }
    let files = upload
        .get("files")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "file_transfer upload requires files array".to_string())?;
    if !files
        .iter()
        .all(|file| file.as_str().is_some_and(|value| !value.trim().is_empty()))
    {
        return Err("file_transfer upload files must be nonempty strings".to_string());
    }
    let max_files = upload
        .get("maxFiles")
        .and_then(Value::as_u64)
        .ok_or_else(|| "file_transfer upload requires positive maxFiles".to_string())?;
    if max_files == 0 {
        return Err("file_transfer upload requires positive maxFiles".to_string());
    }
    if files.len() as u64 > max_files {
        return Err(format!(
            "file_transfer upload file count {} exceeds maxFiles {}",
            files.len(),
            max_files
        ));
    }
    validate_nonempty_string_array(
        upload.get("allowedPaths"),
        "file_transfer upload allowedPaths",
    )
}
pub(crate) fn validate_service_download_recipe(
    download: &Map<String, Value>,
) -> Result<(), String> {
    if download
        .get("selector")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err("file_transfer download requires selector".to_string());
    }
    if download
        .get("directory")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err("file_transfer download requires directory".to_string());
    }
    validate_nonempty_string_array(
        download.get("allowedDirectories"),
        "file_transfer download allowedDirectories",
    )?;
    if let Some(max_bytes) = download.get("maxBytes").and_then(Value::as_u64) {
        if max_bytes == 0 {
            return Err("file_transfer download maxBytes must be positive".to_string());
        }
    }
    Ok(())
}
pub(crate) fn validate_nonempty_string_array(
    value: Option<&Value>,
    label: &str,
) -> Result<(), String> {
    let valid = value
        .and_then(Value::as_array)
        .filter(|items| {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| item.as_str().is_some_and(|text| !text.trim().is_empty()))
        })
        .is_some();
    if !valid {
        return Err(format!("{label} must be a nonempty string array"));
    }
    Ok(())
}
pub(crate) async fn run_service_file_upload(
    upload: &Map<String, Value>,
    state: &mut DaemonState,
) -> Result<Value, String> {
    let selector = resolve_service_file_input_selector(upload, state).await?;
    let allowed_paths = service_canonical_allowed_paths(upload.get("allowedPaths"))?;
    let files = upload
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "file_transfer upload requires files array".to_string())?;
    let mut resolved_files = Vec::new();
    let mut file_items = Vec::new();
    for file in files {
        let raw = file
            .as_str()
            .ok_or_else(|| "file_transfer upload files must be strings".to_string())?;
        let path = service_existing_path(raw)?;
        service_require_allowed_path(&path, &allowed_paths, "upload file")?;
        let metadata = fs::metadata(&path)
            .map_err(|err| format!("Failed to read upload file metadata: {err}"))?;
        resolved_files.push(path.to_string_lossy().to_string());
        file_items.push(json!(
            { "name" : path.file_name().and_then(| value | value.to_str())
            .unwrap_or(""), "path" : path.to_string_lossy().to_string(), "size" :
            metadata.len(), }
        ));
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    mgr.upload_files(
        &selector,
        &resolved_files,
        &state.ref_map,
        &state.iframe_sessions,
    )
    .await?;
    let selected = if upload
        .get("verifySelectedNames")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        service_file_input_selected_names(&selector, state).await?
    } else {
        Value::Null
    };
    Ok(json!(
        { "ok" : true, "selector" : selector, "uploaded" : resolved_files.len(),
        "files" : file_items, "selectedFileNames" : selected, }
    ))
}
pub(crate) async fn resolve_service_file_input_selector(
    upload: &Map<String, Value>,
    state: &mut DaemonState,
) -> Result<String, String> {
    if let Some(selector) = upload
        .get("selector")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(selector.to_string());
    }
    let label_text = upload
        .get("labelText")
        .or_else(|| upload.get("label"))
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer upload requires selector or labelText".to_string())?;
    let label_json = serde_json::to_string(label_text).unwrap_or_else(|_| "\"\"".to_string());
    let expression = format!(
        r#"(() => {{
const expected = String({label_json}).trim().toLowerCase();
for (const input of Array.from(document.querySelectorAll('input[type="file"]'))) {{
  const labels = Array.from(input.labels || []);
  const text = labels.map((label) => String(label.innerText || label.textContent || '')).join(' ').replace(/\s+/g, ' ').trim().toLowerCase();
  if (text.includes(expected)) {{
    const token = input.getAttribute('data-agent-browser-file-input-id') || `file-input-${{Date.now()}}-${{Math.random().toString(16).slice(2)}}`;
    input.setAttribute('data-agent-browser-file-input-id', token);
    return `[data-agent-browser-file-input-id="${{token}}"]`;
  }}
}}
return null;
}})()"#
    );
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let result = mgr.evaluate(&expression, None).await?;
    result
        .pointer("/result/value")
        .or_else(|| result.pointer("/result/result/value"))
        .or_else(|| result.get("value"))
        .or(Some(&result))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            format!(
                "file_transfer upload could not resolve labelText to file input: {}",
                result
            )
        })
}
pub(crate) async fn service_file_input_selected_names(
    selector: &str,
    state: &DaemonState,
) -> Result<Value, String> {
    let selector_json = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string());
    let expression = format!(
        r#"(() => {{
const input = document.querySelector({selector_json});
if (!input || !input.files) return [];
return Array.from(input.files).map((file) => file.name);
}})()"#
    );
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let result = mgr.evaluate(&expression, None).await?;
    Ok(result
        .pointer("/result/value")
        .or_else(|| result.pointer("/result/result/value"))
        .or_else(|| result.get("value"))
        .or(Some(&result))
        .cloned()
        .unwrap_or_else(|| json!([])))
}
pub(crate) async fn run_service_download_capture(
    download: &Map<String, Value>,
    state: &mut DaemonState,
    timeout_ms: u64,
) -> Result<Value, String> {
    let selector = download
        .get("selector")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer download requires selector".to_string())?;
    let directory = download
        .get("directory")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer download requires directory".to_string())?;
    let download_dir = service_prepare_allowed_download_dir(directory, download)?;
    let download_dir_str = download_dir
        .to_str()
        .ok_or("Download directory path is not valid UTF-8")?;
    let max_bytes = download.get("maxBytes").and_then(Value::as_u64);
    if download
        .get("captureMode")
        .and_then(Value::as_str)
        .unwrap_or("fetch")
        != "browser"
    {
        return run_service_download_fetch_capture(download, state, &download_dir, max_bytes).await;
    }
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms.min(1000)),
        mgr.set_download_behavior(download_dir_str),
    )
    .await
    .map_err(|_| "file_transfer set download behavior timed out".to_string())??;
    let mut rx = mgr.client.subscribe();
    tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms.min(1000)),
        interaction::click(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            "left",
            1,
            &state.iframe_sessions,
        ),
    )
    .await
    .map_err(|_| "file_transfer download click timed out".to_string())??;
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    let mut downloaded_guid: Option<String> = None;
    let mut source_url: Option<String> = None;
    let mut canceled_event = false;
    let mut suggested_filename = download
        .get("expectedFileName")
        .or_else(|| download.get("expectedFilename"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("Timeout waiting for file_transfer download".to_string());
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(event)) => {
                let is_page_session = event.session_id.as_deref() == Some(&session_id);
                let is_download_event = |method: &str, browser_method: &str, page_method: &str| {
                    method == browser_method || (method == page_method && is_page_session)
                };
                if is_download_event(
                    &event.method,
                    "Browser.downloadWillBegin",
                    "Page.downloadWillBegin",
                ) {
                    if let Some(guid) = event.params.get("guid").and_then(Value::as_str) {
                        downloaded_guid = Some(guid.to_string());
                    }
                    if source_url.is_none() {
                        source_url = event
                            .params
                            .get("url")
                            .and_then(Value::as_str)
                            .map(ToString::to_string);
                    }
                    if suggested_filename.is_none() {
                        suggested_filename = event
                            .params
                            .get("suggestedFilename")
                            .and_then(Value::as_str)
                            .map(ToString::to_string);
                    }
                }
                if is_download_event(
                    &event.method,
                    "Browser.downloadProgress",
                    "Page.downloadProgress",
                ) {
                    match event.params.get("state").and_then(Value::as_str) {
                        Some("completed") => break,
                        Some("canceled") => {
                            canceled_event = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(_)) => return Err("Event stream closed".to_string()),
            Err(_) => {
                return Err("Timeout waiting for file_transfer download".to_string());
            }
        }
    }
    let file_name = suggested_filename
        .as_deref()
        .and_then(service_safe_file_name)
        .ok_or_else(|| "file_transfer download could not determine safe file name".to_string())?;
    let dest = download_dir.join(&file_name);
    if let Some(guid) = downloaded_guid.as_deref() {
        let guid_path = download_dir.join(guid);
        for _ in 0..10 {
            if guid_path.exists() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        if guid_path.exists() && guid_path != dest {
            fs::rename(&guid_path, &dest)
                .map_err(|err| format!("Failed to rename downloaded file: {err}"))?;
        }
    }
    if !dest.exists() && canceled_event {
        return Err("Download was canceled".to_string());
    }
    if !dest.exists() {
        return Err("Downloaded file not found at captured path".to_string());
    }
    let metadata = fs::metadata(&dest)
        .map_err(|err| format!("Failed to read downloaded file metadata: {err}"))?;
    if let Some(max_bytes) = max_bytes {
        if metadata.len() > max_bytes {
            return Err(format!(
                "Downloaded file size {} exceeds maxBytes {}",
                metadata.len(),
                max_bytes
            ));
        }
    }
    Ok(json!(
        { "ok" : true, "selector" : selector, "localPath" : dest.to_string_lossy()
        .to_string(), "fileName" : file_name, "size" : metadata.len(), "mimeType" :
        service_guess_mime_type(& dest), "sourceUrl" : source_url, "timedOut" :
        false, "canceledEvent" : canceled_event, "maxBytes" : max_bytes, }
    ))
}
pub(crate) fn service_canonical_allowed_paths(
    value: Option<&Value>,
) -> Result<Vec<PathBuf>, String> {
    let items = value
        .and_then(Value::as_array)
        .ok_or_else(|| "allowed paths must be an array".to_string())?;
    items
        .iter()
        .map(|item| {
            let path = item
                .as_str()
                .ok_or_else(|| "allowed paths must be strings".to_string())?;
            service_existing_path(path)
        })
        .collect()
}
pub(crate) fn service_existing_path(path: &str) -> Result<PathBuf, String> {
    let raw = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        env::current_dir()
            .map_err(|err| format!("Failed to get current directory: {err}"))?
            .join(path)
    };
    raw.canonicalize()
        .map_err(|err| format!("Failed to resolve path '{}': {err}", raw.display()))
}
pub(crate) fn service_require_allowed_path(
    path: &Path,
    allowed_paths: &[PathBuf],
    label: &str,
) -> Result<(), String> {
    if allowed_paths
        .iter()
        .any(|allowed| path == allowed || path.starts_with(allowed))
    {
        Ok(())
    } else {
        Err(format!("{label} is outside allowedPaths"))
    }
}
pub(crate) fn service_prepare_allowed_download_dir(
    directory: &str,
    download: &Map<String, Value>,
) -> Result<PathBuf, String> {
    let raw = if Path::new(directory).is_absolute() {
        PathBuf::from(directory)
    } else {
        env::current_dir()
            .map_err(|err| format!("Failed to get current directory: {err}"))?
            .join(directory)
    };
    fs::create_dir_all(&raw)
        .map_err(|err| format!("Failed to create download directory: {err}"))?;
    let canonical_dir = raw
        .canonicalize()
        .map_err(|err| format!("Failed to resolve download directory: {err}"))?;
    let allowed = service_canonical_allowed_paths(download.get("allowedDirectories"))?;
    if allowed.iter().any(|item| canonical_dir.starts_with(item)) {
        Ok(canonical_dir)
    } else {
        Err("download directory is outside allowedDirectories".to_string())
    }
}
pub(crate) fn service_safe_file_name(value: &str) -> Option<String> {
    let path = Path::new(value);
    let name = path.file_name()?.to_str()?.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        None
    } else {
        Some(name.to_string())
    }
}
pub(crate) async fn run_service_download_fetch_capture(
    download: &Map<String, Value>,
    state: &DaemonState,
    download_dir: &Path,
    max_bytes: Option<u64>,
) -> Result<Value, String> {
    let selector = download
        .get("selector")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer download requires selector".to_string())?;
    let max_fetch_bytes = max_bytes.unwrap_or(10 * 1024 * 1024).min(10 * 1024 * 1024);
    let expected_file_name = download
        .get("expectedFileName")
        .or_else(|| download.get("expectedFilename"))
        .and_then(Value::as_str);
    let selector_json =
        serde_json::to_string(selector).map_err(|err| format!("Invalid selector: {err}"))?;
    let expected_json = serde_json::to_string(&expected_file_name)
        .map_err(|err| format!("Invalid file name: {err}"))?;
    let script = format!(
        r#"(async () => {{
const node = document.querySelector({selector_json});
if (!node) throw new Error('download selector not found');
const rawUrl = node.href || node.getAttribute('href');
if (!rawUrl) throw new Error('download selector has no href');
const url = new URL(rawUrl, window.location.href).toString();
const response = await fetch(url, {{ credentials: 'include' }});
const buffer = await response.arrayBuffer();
if (buffer.byteLength > {max_fetch_bytes}) throw new Error(`download exceeds maxBytes ${{buffer.byteLength}}`);
let binary = '';
const bytes = new Uint8Array(buffer);
const chunkSize = 0x8000;
for (let i = 0; i < bytes.length; i += chunkSize) {{
  binary += String.fromCharCode(...bytes.slice(i, i + chunkSize));
}}
const expected = {expected_json};
const attrName = node.getAttribute('download');
const pathName = new URL(response.url || url).pathname.split('/').filter(Boolean).pop();
return {{
  sourceUrl: response.url || url,
  status: response.status,
  ok: response.ok,
  fileName: expected || attrName || pathName || 'download',
  mimeType: response.headers.get('content-type'),
  size: buffer.byteLength,
  bodyBase64: btoa(binary),
}};
}})()"#
    );
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let result = tokio::time::timeout(
        tokio::time::Duration::from_millis(
            download
                .get("fetchTimeoutMs")
                .and_then(Value::as_u64)
                .unwrap_or(10_000)
                .clamp(1, 60_000),
        ),
        mgr.evaluate(&script, None),
    )
    .await
    .map_err(|_| "file_transfer fetch download timed out".to_string())??;
    let payload = service_extract_evaluate_value(&result)
        .ok_or_else(|| format!("file_transfer fetch download returned no payload: {result}"))?;
    let file_name = payload
        .get("fileName")
        .and_then(Value::as_str)
        .and_then(service_safe_file_name)
        .ok_or_else(|| "file_transfer download could not determine safe file name".to_string())?;
    let body = payload
        .get("bodyBase64")
        .and_then(Value::as_str)
        .ok_or_else(|| "file_transfer fetch download returned no body".to_string())?;
    let bytes = BASE64_STANDARD
        .decode(body)
        .map_err(|err| format!("file_transfer fetch download body was invalid base64: {err}"))?;
    if let Some(max_bytes) = max_bytes {
        if bytes.len() as u64 > max_bytes {
            return Err(format!(
                "Downloaded file size {} exceeds maxBytes {}",
                bytes.len(),
                max_bytes
            ));
        }
    }
    let dest = download_dir.join(&file_name);
    fs::write(&dest, &bytes).map_err(|err| format!("Failed to write downloaded file: {err}"))?;
    Ok(json!(
        { "ok" : true, "selector" : selector, "captureMode" : "fetch", "localPath" :
        dest.to_string_lossy().to_string(), "fileName" : file_name, "size" : bytes
        .len(), "mimeType" : payload.get("mimeType").cloned().unwrap_or(Value::Null),
        "sourceUrl" : payload.get("sourceUrl").cloned().unwrap_or(Value::Null),
        "status" : payload.get("status").cloned().unwrap_or(Value::Null), "timedOut"
        : false, "maxBytes" : max_bytes, }
    ))
}
pub(crate) fn service_extract_evaluate_value(result: &Value) -> Option<&Value> {
    result
        .pointer("/result/value")
        .or_else(|| result.pointer("/result/result/value"))
        .or_else(|| result.get("value"))
        .or(Some(result))
}
pub(crate) fn service_guess_mime_type(path: &Path) -> Value {
    let mime = match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "txt" | "text" => "text/plain",
        "json" => "application/json",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "html" | "htm" => "text/html",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        _ => return Value::Null,
    };
    json!(mime)
}
