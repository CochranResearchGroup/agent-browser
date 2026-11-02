//! Authenticated workspace composition over the installed teaching support assets.
//! Profile policy remains in the existing owner; source/approval paths never come
//! from dashboard JSON. This adapter does not launch browsers or acquire media.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

pub(crate) fn operation_is_write(operation: &str) -> Result<bool, String> {
    match operation {
        "list"
        | "open"
        | "lookup_operation"
        | "list_sources"
        | "read_video"
        | "inspect_source"
        | "read_transcript"
        | "agent_packet"
        | "reconcile_narration" => Ok(false),
        "import_narration" | "transcribe" | "stage_draft" | "save" | "prepare_questions"
        | "answer" | "record_decision" | "record_repeat" | "promote" => Ok(true),
        _ => Err("siteguides_operation_unsupported".into()),
    }
}

fn installed_host(executable: &Path) -> Result<PathBuf, String> {
    let support = executable
        .parent()
        .filter(|parent| parent.file_name().is_some_and(|name| name == "bin"))
        .and_then(Path::parent)
        .ok_or("siteguides_installed_support_required")?
        .join("support");
    for (relative, expected, _) in crate::workstation_install::CONTROLLER_ASSETS {
        if !relative.starts_with("teaching/") {
            continue;
        }
        let path = support.join(relative);
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|_| "siteguides_installed_support_required")?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("siteguides_installed_support_invalid".into());
        }
        let bytes = std::fs::read(path).map_err(|_| "siteguides_installed_support_required")?;
        if Sha256::digest(&bytes) != Sha256::digest(expected.as_bytes()) {
            return Err("siteguides_installed_support_hash_mismatch".into());
        }
    }
    Ok(support.join("teaching/workspace-siteguides-host.js"))
}

/// The server obtains subject/role from dashboard authentication. Client actor,
/// assurance, expected source binding, runtime paths and endpoints are stripped.
pub(crate) async fn handle_workspace_siteguides(
    input: &Value,
    username: &str,
    confirmation_directory: Option<PathBuf>,
) -> Result<Value, String> {
    let operation = input
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("list");
    operation_is_write(operation)?;
    let mut request = json!({"operation":operation});
    for key in [
        "taskId",
        "guideId",
        "rootGuideId",
        "expectedSha256",
        "operationId",
        "sourceKey",
        "requestId",
        "bundle",
        "range",
        "segment",
        "guide",
        "questions",
        "questionId",
        "answerRef",
        "revisedGuide",
        "decisionRef",
        "validationRef",
        "requestedStartDate",
        "requestedEndDate",
        "reportId",
    ] {
        if let Some(value) = input.get(key) {
            request[key] = value.clone();
        }
    }
    let principal = format!("dashboard:{}", username.trim());
    let list = super::workspace_profile_library::handle_workspace_profile_library(&json!({
        "operation":"list","servicePrincipalId":principal,"servicePrincipalProvenance":"authenticated_transport"
    })).await?;
    let visible = list["profiles"]
        .as_array()
        .ok_or("siteguides_profile_readback_required")?;
    let mut allowed_tasks = serde_json::Map::new();
    if let Some(managed) =
        super::browser_session_store::BrowserSessionSqliteStore::default_session_state_read_only()?
    {
        for (task_id, session) in managed.sessions {
            if !visible
                .iter()
                .any(|row| row["profile"]["id"].as_str() == Some(session.profile_id.as_str()))
            {
                continue;
            }
            let selection = super::workspace_profile_library::handle_workspace_profile_library(&json!({
                "operation":"select","profileId":session.profile_id,"servicePrincipalId":principal,"servicePrincipalProvenance":"authenticated_transport"
            })).await?;
            allowed_tasks.insert(task_id, json!({"read":true,"use":selection["profileId"].as_str()==Some(session.profile_id.as_str())}));
        }
    }
    let config = std::env::var_os("AGENT_BROWSER_SITEGUIDES_CONFIG")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or(
            "siteguides_configuration_required: configure private AGENT_BROWSER_SITEGUIDES_CONFIG",
        )?;
    let executable = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|_| "siteguides_installed_support_required")?;
    let host = installed_host(&executable)?;
    let version = tokio::process::Command::new("node")
        .arg("--version")
        .output()
        .await
        .map_err(|_| "siteguides_node_18_required: install Node.js 18 or newer")?;
    let major = String::from_utf8_lossy(&version.stdout)
        .trim()
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|part| part.parse::<u32>().ok());
    if !version.status.success() || major.is_none_or(|major| major < 18) {
        return Err("siteguides_node_18_required".into());
    }
    let mut child = tokio::process::Command::new("node")
        .arg(host)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| "siteguides_node_18_required")?;
    let payload = serde_json::to_vec(&json!({"configPath":config,"actor":username,"ledgerDirectory":confirmation_directory,"allowedTasks":allowed_tasks,"request":request})).map_err(|error|error.to_string())?;
    child
        .stdin
        .take()
        .ok_or("siteguides_transport_unavailable")?
        .write_all(&payload)
        .await
        .map_err(|_| "siteguides_transport_unavailable")?;
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| "siteguides_original_operation_unconfirmed")?
    .map_err(|_| "siteguides_original_operation_unconfirmed")?;
    if output.stdout.len() > 1048576 {
        return Err("siteguides_response_too_large".into());
    }
    let response: Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| "siteguides_original_operation_unconfirmed")?;
    if response["success"] != true {
        return Err(response["error"]
            .as_str()
            .unwrap_or("siteguides_unavailable")
            .to_string());
    }
    Ok(response["data"].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn siteguides_operation_policy_is_not_blanket_read_only() {
        assert!(!operation_is_write("list").unwrap());
        for operation in [
            "import_narration",
            "transcribe",
            "stage_draft",
            "save",
            "prepare_questions",
            "answer",
            "record_decision",
            "record_repeat",
            "promote",
        ] {
            assert!(operation_is_write(operation).unwrap());
        }
        assert!(operation_is_write("browser_open").is_err());
        assert!(installed_host(Path::new("/source/cli/target/debug/agent-browser")).is_err());
    }
}
