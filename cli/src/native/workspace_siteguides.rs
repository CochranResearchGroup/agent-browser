//! Authenticated workspace composition over the installed teaching support assets.
//! Profile policy remains in the existing owner; source/approval paths never come
//! from dashboard JSON. This adapter does not launch browsers or acquire media.
use super::service_store::ServiceStateRepository;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
        "request_agent" | "resume_agent" | "import_narration" | "transcribe" | "stage_draft"
        | "save" | "prepare_questions" | "answer" | "record_decision" | "record_repeat"
        | "promote" => Ok(true),
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
        "priorGuideId",
        "rootGuideId",
        "expectedSha256",
        "operationId",
        "kind",
        "sourceKey",
        "requestId",
        "bundle",
        "speechSource",
        "segments",
        "sitePolicyId",
        "account",
        "organization",
        "range",
        "segment",
        "guide",
        "questions",
        "questionId",
        "purpose",
        "reviewedGuideId",
        "reviewedSha256",
        "contentSha256",
        "answeredGuideId",
        "answeredSha256",
        "questionSha256",
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
    let repository = super::service_store::LockedServiceStateRepository::default_json()?;
    let mut profile_state = repository.load_snapshot()?;
    profile_state
        .overlay_configured_entities(crate::flags::load_config(&[])?.service_state_snapshot());
    profile_state.apply_builtin_site_policies();
    let config = std::env::var_os("AGENT_BROWSER_SITEGUIDES_CONFIG")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or(
            "siteguides_configuration_required: configure private AGENT_BROWSER_SITEGUIDES_CONFIG",
        )?;
    let sites = json!(profile_state
        .site_policies
        .values()
        .map(|site| {
            let mut origins = site
                .origins
                .iter()
                .chain(std::iter::once(&site.origin_pattern))
                .filter_map(|origin| url::Url::parse(origin).ok())
                .filter(|origin| {
                    matches!(origin.scheme(), "http" | "https")
                        && !origin.host_str().unwrap_or("").contains('*')
                        && origin.username().is_empty()
                        && origin.password().is_none()
                })
                .map(|origin| origin.origin().ascii_serialization())
                .collect::<Vec<_>>();
            origins.sort();
            origins.dedup();
            json!({"id":site.id,"allowedOrigins":origins})
        })
        .collect::<Vec<_>>());
    let retained = invoke_installed_host(
        &config,
        username,
        confirmation_directory.as_deref(),
        &json!({}),
        &sites,
        &json!({"operation":"retained_contexts"}),
    )
    .await?;
    let mut allowed_tasks = serde_json::Map::new();
    if let Some(managed) =
        super::browser_session_store::BrowserSessionSqliteStore::default_session_state_read_only()?
    {
        let mut tasks = managed.sessions.values().cloned().collect::<Vec<_>>();
        tasks.extend(
            managed
                .session_history
                .iter()
                .filter(|task| !managed.sessions.contains_key(&task.id))
                .map(|task| agent_browser_service_model::ManagedBrowserSession {
                    id: task.id.clone(),
                    name: task.name.clone(),
                    profile_id: task.profile_id.clone(),
                    browser_id: task.browser_id.clone(),
                    created_at_ms: task.created_at_ms,
                    last_activity_at_ms: task.last_activity_at_ms,
                    expires_at_ms: task.ended_at_ms,
                    current_tab_id: None,
                }),
        );
        for session in &tasks {
            let task_id = &session.id;
            let row = visible
                .iter()
                .find(|row| row["profile"]["id"].as_str() == Some(session.profile_id.as_str()));
            if row.is_none() {
                let captured_disposable = retained["contexts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|record| {
                        record["taskId"] == session.id
                            && record["context"]["profileId"] == session.profile_id
                            && record["context"]["profileKind"] == "disposable"
                    });
                let retained_default = captured_disposable
                    && !profile_state.profiles.contains_key(&session.profile_id);
                let read = (retained_default
                    && default_profile_access(
                        &session.profile_id,
                        &principal,
                        agent_browser_service_model::ProfilePermission::PolicyRead,
                    ))
                    || disposable_default_access(
                        &managed,
                        &session,
                        profile_state.profiles.contains_key(&session.profile_id),
                        &principal,
                        agent_browser_service_model::ProfilePermission::PolicyRead,
                    );
                let use_profile = (retained_default
                    && default_profile_access(
                        &session.profile_id,
                        &principal,
                        agent_browser_service_model::ProfilePermission::ProfileUse,
                    ))
                    || disposable_default_access(
                        &managed,
                        &session,
                        profile_state.profiles.contains_key(&session.profile_id),
                        &principal,
                        agent_browser_service_model::ProfilePermission::ProfileUse,
                    );
                if read {
                    allowed_tasks.insert(task_id.clone(),json!({"read":true,"use":use_profile,"profileId":session.profile_id,"profileKind":"disposable","currentBrowser":managed.sessions.contains_key(task_id)&&managed.browsers.get(&session.browser_id).is_some_and(|browser|browser.profile_id==session.profile_id),"siteAssociations":[]}));
                }
                continue;
            }
            let selection = super::workspace_profile_library::handle_workspace_profile_library(&json!({
                "operation":"select","profileId":session.profile_id,"servicePrincipalId":principal,"servicePrincipalProvenance":"authenticated_transport"
            })).await?;
            let profile = visible
                .iter()
                .find(|row| row["profile"]["id"].as_str() == Some(session.profile_id.as_str()))
                .map(|row| row["profile"].clone())
                .unwrap_or(Value::Null);
            allowed_tasks.insert(task_id.clone(), json!({"read":true,"use":selection["profileId"].as_str()==Some(session.profile_id.as_str()),"profileId":session.profile_id,"profileKind":"named","currentBrowser":managed.sessions.contains_key(task_id)&&managed.browsers.get(&session.browser_id).is_some_and(|browser|browser.profile_id==session.profile_id),"siteAssociations":profile["siteAssociations"]}));
        }
    }
    let tasks = Value::Object(allowed_tasks);
    if operation == "request_agent" {
        if let Err(reason) = super::stream::chat::teaching_agent_preflight() {
            return Ok(
                json!({"status":"unavailable","reason":reason,"retryAuthority":"not_inferred"}),
            );
        }
    }
    if matches!(operation, "request_agent" | "resume_agent") {
        let config = &config;
        let tasks = &tasks;
        let sites = &sites;
        let confirmation_directory = confirmation_directory.as_deref();
        return compose_agent_operation(
            &request,
            |command| async move {
                invoke_installed_host(
                    config,
                    username,
                    confirmation_directory,
                    tasks,
                    sites,
                    &command,
                )
                .await
            },
            |packet| async move { super::stream::chat::interpret_teaching_packet(&packet).await },
        )
        .await;
    }
    invoke_installed_host(
        &config,
        username,
        confirmation_directory.as_deref(),
        &tasks,
        &sites,
        &request,
    )
    .await
}

/// Host and model calls are existing transport boundaries. Only the first atomic
/// admission can invoke the model; resume consumes original persisted output.
async fn compose_agent_operation<H, HF, I, IF>(
    request: &Value,
    host: H,
    interpret: I,
) -> Result<Value, String>
where
    H: Fn(Value) -> HF,
    HF: std::future::Future<Output = Result<Value, String>>,
    I: FnOnce(Value) -> IF,
    IF: std::future::Future<Output = Result<Value, String>>,
{
    let command = |operation: &str| {
        let mut value = request.clone();
        value["operation"] = json!(operation);
        value
    };
    let admitted = if request["operation"] == "resume_agent" {
        host(command("read_agent")).await?
    } else {
        host(command("begin_agent")).await?
    };
    if admitted["status"] != "created" {
        return if admitted["status"] == "resolved" {
            host(command("stage_agent_result")).await
        } else {
            Ok(admitted)
        };
    }
    let packet = match host(command("prepare_agent_media")).await {
        Ok(packet) => packet,
        Err(reason) => {
            let response =
                json!({"status":"unavailable","reason":reason,"retryAuthority":"not_inferred"});
            let mut record = command("record_agent_result");
            record["response"] = response.clone();
            host(record).await?;
            return Ok(response);
        }
    };
    let mut binding = command("bind_agent_media");
    binding["packet"] = packet.clone();
    host(binding).await?;
    let response = match interpret(packet).await {
        Ok(response) => response,
        Err(reason) => {
            json!({"status":if reason=="teaching_agent_outcome_unknown"{"unknown"}else{"unavailable"},"reason":reason,"retryAuthority":"not_inferred"})
        }
    };
    let mut record = command("record_agent_result");
    record["response"] = response.clone();
    host(record).await?;
    if response.get("status").is_some() {
        return Ok(response);
    }
    host(command("stage_agent_result")).await
}

async fn invoke_installed_host(
    config: &Path,
    username: &str,
    confirmation_directory: Option<&Path>,
    allowed_tasks: &Value,
    known_site_policies: &Value,
    request: &Value,
) -> Result<Value, String> {
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
    let mut command = tokio::process::Command::new("node");
    command
        .arg(host)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.as_std_mut().process_group(0);
    }
    let mut child = command.spawn().map_err(|_| "siteguides_node_18_required")?;
    let mut group = OwnedTeachingHostGroup(child.id());
    let payload = serde_json::to_vec(&json!({"configPath":config,"browserExecutable":executable,"actor":username,"ledgerDirectory":confirmation_directory,"allowedTasks":allowed_tasks,"knownSitePolicies":known_site_policies,"request":request})).map_err(|error|error.to_string())?;
    child
        .stdin
        .take()
        .ok_or("siteguides_transport_unavailable")?
        .write_all(&payload)
        .await
        .map_err(|_| "siteguides_transport_unavailable")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("siteguides_transport_unavailable")?;
    let reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout
            .take(1048577)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| "siteguides_transport_unavailable")?;
        Ok::<_, &str>(bytes)
    });
    if tokio::time::timeout(std::time::Duration::from_secs(180), child.wait())
        .await
        .is_err()
    {
        group.signal(false);
        if tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
            .await
            .is_err()
        {
            group.signal(true);
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        let _ = reader.await;
        return Err("siteguides_original_operation_unconfirmed".into());
    }
    group.signal(false);
    let bytes = reader
        .await
        .map_err(|_| "siteguides_transport_unavailable")?
        .map_err(str::to_string)?;
    group.0 = None;
    if bytes.len() > 1048576 {
        return Err("siteguides_response_too_large".into());
    }
    let response: Value =
        serde_json::from_slice(&bytes).map_err(|_| "siteguides_original_operation_unconfirmed")?;
    if response["success"] != true {
        return Err(response["error"]
            .as_str()
            .unwrap_or("siteguides_unavailable")
            .to_string());
    }
    Ok(response["data"].clone())
}

/// Only the newly spawned, isolated host group is signaled. Graceful timeout
/// lets its media finally blocks remove temporary input before the hard bound.
struct OwnedTeachingHostGroup(Option<u32>);
impl OwnedTeachingHostGroup {
    fn signal(&self, hard: bool) {
        #[cfg(unix)]
        if let Some(pid) = self.0 {
            unsafe {
                libc::kill(
                    -(pid as i32),
                    if hard { libc::SIGKILL } else { libc::SIGTERM },
                );
            }
        }
        #[cfg(not(unix))]
        let _ = hard;
    }
}
impl Drop for OwnedTeachingHostGroup {
    fn drop(&mut self) {
        self.signal(false);
    }
}

/// A positive native disposable allocation uses the existing SharedLocal default
/// policy only when no persistent/configured profile of that identity exists.
/// A hidden restricted profile can never enter this fallback by caller assertion.
fn disposable_default_access(
    managed: &agent_browser_service_model::BrowserSessionState,
    session: &agent_browser_service_model::ManagedBrowserSession,
    registered: bool,
    subject: &str,
    permission: agent_browser_service_model::ProfilePermission,
) -> bool {
    if registered
        || !managed
            .disposable_profiles
            .get(&session.profile_id)
            .is_some_and(|allocation| {
                allocation.profile.id == session.profile_id
                    && allocation.profile.kind
                        == agent_browser_service_model::BrowserProfileKind::Disposable
                    && allocation.session_name == session.name
            })
    {
        return false;
    }
    default_profile_access(&session.profile_id, subject, permission)
}
fn default_profile_access(
    profile_id: &str,
    subject: &str,
    permission: agent_browser_service_model::ProfilePermission,
) -> bool {
    agent_browser_service_model::evaluate_profile_access(
        agent_browser_service_model::ProfileAccessEvaluation {
            profile_id,
            explicit_policy: None,
            subject_id: Some(subject.to_string()),
            assurance: agent_browser_service_model::ProfileIdentityAssurance::AuthenticatedIngress,
            connection_instance_id: None,
            permission,
            operation: "workspace_teaching",
            incompatible_occupancy: vec![],
        },
    )
    .1
    .allowed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn siteguides_agent_original_result_and_unknown_never_replay_model() {
        use std::sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc, Mutex,
        };
        let state = Arc::new(Mutex::new((false, None::<Value>)));
        let host_state = state.clone();
        let host = move |command: Value| {
            let state = host_state.clone();
            async move {
                let mut state = state.lock().unwrap();
                match command["operation"].as_str().unwrap() {
                    "begin_agent" => {
                        if !state.0 {
                            state.0 = true;
                            Ok(json!({"status":"created"}))
                        } else {
                            Ok(
                                json!({"status":if state.1.is_some(){"resolved"}else{"pending"},"retryAuthority":"not_inferred"}),
                            )
                        }
                    }
                    "read_agent" => Ok(
                        json!({"status":if state.1.is_some(){"resolved"}else{"pending"},"retryAuthority":"not_inferred"}),
                    ),
                    "prepare_agent_media" => {
                        Ok(json!({"schemaVersion":"agent-browser.teaching-agent-media.v1"}))
                    }
                    "bind_agent_media" => Ok(json!({"bound":true})),
                    "record_agent_result" => {
                        state.1 = Some(command["response"].clone());
                        Ok(json!({"status":"resolved"}))
                    }
                    "stage_agent_result" => Ok(
                        if state
                            .1
                            .as_ref()
                            .is_some_and(|result| result.get("status").is_some())
                        {
                            state.1.clone().unwrap()
                        } else {
                            json!({"status":"saved","guideId":"original"})
                        },
                    ),
                    _ => Err("unexpected host operation".into()),
                }
            }
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let unknown = Arc::new(AtomicBool::new(false));
        let model_calls = calls.clone();
        let model_unknown = unknown.clone();
        let model = move |_: Value| {
            model_calls.fetch_add(1, Ordering::SeqCst);
            let unknown = model_unknown.load(Ordering::SeqCst);
            async move {
                if unknown {
                    Err("teaching_agent_outcome_unknown".into())
                } else {
                    Ok(json!({"guide":{"guideId":"original"},"questions":[]}))
                }
            }
        };
        let request = json!({"operation":"request_agent","taskId":"task","rootGuideId":"original","operationId":"original"});
        assert_eq!(
            compose_agent_operation(&request, &host, &model)
                .await
                .unwrap()["status"],
            "saved"
        );
        assert_eq!(
            compose_agent_operation(&request, &host, &model)
                .await
                .unwrap()["status"],
            "saved"
        );
        assert_eq!(
            compose_agent_operation(&json!({"operation":"resume_agent"}), &host, &model)
                .await
                .unwrap()["status"],
            "saved"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        *state.lock().unwrap() = (false, None);
        calls.store(0, Ordering::SeqCst);
        unknown.store(true, Ordering::SeqCst);
        assert_eq!(
            compose_agent_operation(&request, &host, &model)
                .await
                .unwrap()["status"],
            "unknown"
        );
        assert_eq!(
            compose_agent_operation(&request, &host, &model)
                .await
                .unwrap()["status"],
            "unknown"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        *state.lock().unwrap() = (true, None);
        calls.store(0, Ordering::SeqCst);
        assert_eq!(
            compose_agent_operation(&request, &host, &model)
                .await
                .unwrap()["status"],
            "pending"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn siteguides_disposable_uses_owned_allocation_and_never_shadows_private_profile() {
        use agent_browser_service_model::*;
        let session = ManagedBrowserSession {
            id: "task".into(),
            name: "disposable-task".into(),
            profile_id: "disposable-1".into(),
            browser_id: "browser".into(),
            created_at_ms: 0,
            last_activity_at_ms: 0,
            expires_at_ms: 1000,
            current_tab_id: None,
        };
        let mut managed = BrowserSessionState::default();
        assert!(!disposable_default_access(
            &managed,
            &session,
            false,
            "dashboard:operator",
            ProfilePermission::PolicyRead
        ));
        managed.disposable_profiles.insert(
            "disposable-1".into(),
            ManagedDisposableProfile {
                profile: BrowserProfileCatalogEntry {
                    id: "disposable-1".into(),
                    name: "Disposable".into(),
                    user_data_dir: "/fixture-only".into(),
                    kind: BrowserProfileKind::Disposable,
                    manual_login_url: None,
                },
                policy_id: "existing".into(),
                session_name: "disposable-task".into(),
                user_data_root: "/fixture-only".into(),
                created_at_ms: 0,
                cleanup_delay_ms: 0,
                cleanup_eligible_at_ms: None,
            },
        );
        assert!(disposable_default_access(
            &managed,
            &session,
            false,
            "dashboard:operator",
            ProfilePermission::PolicyRead
        ));
        assert!(disposable_default_access(
            &managed,
            &session,
            false,
            "dashboard:operator",
            ProfilePermission::ProfileUse
        ));
        assert!(
            !disposable_default_access(
                &managed,
                &session,
                true,
                "dashboard:operator",
                ProfilePermission::ProfileUse
            ),
            "registered policy remains authoritative even if a native allocation collides"
        );
        managed
            .disposable_profiles
            .get_mut("disposable-1")
            .unwrap()
            .session_name = "peer".into();
        assert!(!disposable_default_access(
            &managed,
            &session,
            false,
            "dashboard:operator",
            ProfilePermission::ProfileUse
        ));
    }
    #[test]
    fn siteguides_operation_policy_is_not_blanket_read_only() {
        assert!(!operation_is_write("list").unwrap());
        for operation in [
            "request_agent",
            "resume_agent",
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
