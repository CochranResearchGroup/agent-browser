use super::app_intelligence_schema::{
    observation_output_schema, reject_sensitive_markers, validate_observation,
    CODEX_WORKSPACE_OBSERVATION_VERSION, CONTEXTUAL_CHAT_PROVIDER_ID,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const DEFAULT_TURN_TIMEOUT_SECONDS: u64 = 75;

#[derive(Debug, Clone)]
pub(crate) struct InspectionInput {
    pub(crate) run_id: String,
    pub(crate) created_at: String,
    pub(crate) prompt: String,
    pub(crate) packet: Value,
    pub(crate) packet_hash: String,
    pub(crate) workspace_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct InspectionSuccess {
    pub(crate) observation: Value,
    pub(crate) ledger: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct InspectionFailure {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) ledger: Option<Value>,
}

pub(crate) fn inspect_with_supervisor(
    input: InspectionInput,
) -> Result<InspectionSuccess, InspectionFailure> {
    let mut run = InspectionRun::create(&input).map_err(|message| InspectionFailure {
        code: "ledger_write_failed",
        message,
        ledger: None,
    })?;
    run.write_request(&input)
        .map_err(|message| run.failure("ledger_write_failed", message))?;

    let mode = env::var("AGENT_BROWSER_APP_INTELLIGENCE_MODE").unwrap_or_else(|_| {
        if cfg!(test) {
            "deterministic".to_string()
        } else {
            "codex".to_string()
        }
    });
    let result = if mode == "deterministic" {
        deterministic_inspection(&input, &mut run)
    } else {
        run_codex_inspection(&input, &mut run)
    };

    match result {
        Ok(mut success) => {
            if let Err(message) = run.write_observation(&success.observation) {
                return Err(run.failure("ledger_write_failed", message));
            }
            if let Ok(ledger) = run.finalize("succeeded", &success.ledger) {
                success.ledger = ledger;
            }
            Ok(success)
        }
        Err(mut failure) => {
            if failure.ledger.is_none() {
                failure.ledger = run.finalize("failed", &json!({})).ok();
            }
            Err(failure)
        }
    }
}

fn run_codex_inspection(
    input: &InspectionInput,
    run: &mut InspectionRun,
) -> Result<InspectionSuccess, InspectionFailure> {
    let codex_bin = resolve_codex_bin();
    let cli_version = codex_cli_version(&codex_bin);
    let mut client = CodexAppServerClient::start(&codex_bin, run)
        .map_err(|message| run.failure("codex_unavailable", message))?;

    let initialize = client
        .request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "agent-browser",
                    "title": "Agent Browser",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "experimentalApi": true,
                    "requestAttestation": false,
                    "optOutNotificationMethods": [],
                },
            }),
            Duration::from_secs(10),
        )
        .map_err(|message| run.failure("protocol_error", message))?;
    client
        .notify("initialized", json!(null))
        .map_err(|message| run.failure("protocol_error", message))?;

    let thread = client
        .request(
            "thread/start",
            json!({
                "cwd": env::current_dir().ok().and_then(|path| path.to_str().map(str::to_string)),
                "approvalPolicy": "never",
                "sandbox": "read-only",
                "serviceName": "agent-browser-app-intelligence",
                "baseInstructions": "You are a read-only browser workspace inspection adapter. Do not run commands, call tools, edit files, mutate browser state, mutate service state, request approvals, or use private data outside the supplied packet.",
                "developerInstructions": "Return only the requested JSON object. Do not include markdown fences or prose outside JSON.",
                "ephemeral": true,
            }),
            Duration::from_secs(20),
        )
        .map_err(|message| run.failure("protocol_error", message))?;
    let thread_id = thread
        .pointer("/thread/id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let thread_id_value = thread_id.clone().ok_or_else(|| {
        run.failure(
            "protocol_error",
            "thread/start did not return a thread id".to_string(),
        )
    })?;

    let turn = client
        .request(
            "turn/start",
            json!({
                "threadId": thread_id_value,
                "input": [
                    {
                        "type": "text",
                        "text": build_prompt(input),
                        "text_elements": [],
                    }
                ],
                "approvalPolicy": "never",
                "sandboxPolicy": {
                    "type": "readOnly",
                    "networkAccess": false,
                },
                "outputSchema": observation_output_schema(),
            }),
            Duration::from_secs(20),
        )
        .map_err(|message| run.failure("protocol_error", message))?;
    let turn_id = turn
        .pointer("/turn/id")
        .and_then(Value::as_str)
        .map(str::to_string);

    let timeout = Duration::from_secs(
        env::var("AGENT_BROWSER_APP_INTELLIGENCE_TURN_TIMEOUT_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TURN_TIMEOUT_SECONDS),
    );
    let output = client
        .collect_turn_output(thread_id.as_deref(), turn_id.as_deref(), timeout, run)
        .map_err(|failure| run.failure(failure.0, failure.1))?;
    let observation = parse_observation_output(&output).map_err(|message| {
        run.write_rejected_output(&output).ok();
        run.failure("invalid_observation", message)
    })?;
    validate_observation(&observation, &input.packet, &input.run_id).map_err(|message| {
        run.write_rejected_output(&output).ok();
        run.failure("invalid_observation", message)
    })?;
    reject_policy_violations(&observation)
        .map_err(|message| run.failure("policy_violation", message))?;

    Ok(InspectionSuccess {
        observation,
        ledger: json!({
            "threadId": thread_id,
            "turnId": turn_id,
            "transport": "stdio-jsonl",
            "cliVersion": cli_version,
            "initialize": initialize,
        }),
    })
}

fn deterministic_inspection(
    input: &InspectionInput,
    run: &mut InspectionRun,
) -> Result<InspectionSuccess, InspectionFailure> {
    let observation = build_deterministic_observation(input);
    validate_observation(&observation, &input.packet, &input.run_id)
        .map_err(|message| run.failure("invalid_observation", message))?;
    let ledger = json!({
            "threadId": Value::Null,
            "turnId": Value::Null,
            "transport": "deterministic-test",
            "cliVersion": codex_cli_version(&resolve_codex_bin()),
    });
    Ok(InspectionSuccess {
        observation,
        ledger,
    })
}

struct InspectionRun {
    root: PathBuf,
    run_json: PathBuf,
    request_json: PathBuf,
    codex_events_jsonl: PathBuf,
    events_jsonl: PathBuf,
    observation_json: PathBuf,
    rejected_output_json: PathBuf,
    input_summary: Value,
}

impl InspectionRun {
    fn create(input: &InspectionInput) -> Result<Self, String> {
        let root = app_intelligence_run_root()
            .ok_or_else(|| "Cannot resolve app intelligence run root.".to_string())?
            .join(&input.run_id);
        fs::create_dir_all(&root)
            .map_err(|err| format!("Failed to create run directory: {err}"))?;
        let run = Self {
            run_json: root.join("run.json"),
            request_json: root.join("request.json"),
            codex_events_jsonl: root.join("codex-events.jsonl"),
            events_jsonl: root.join("events.jsonl"),
            observation_json: root.join("observation.json"),
            rejected_output_json: root.join("rejected-output.json"),
            input_summary: json!({
                "runId": input.run_id,
                "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
                "mode": "read-only-inspection",
                "createdAt": input.created_at,
                "workspaceId": input.workspace_id,
                "contextPacketHash": input.packet_hash,
            }),
            root,
        };
        run.write_event("run_created", json!({}))?;
        atomic_write_json(&run.run_json, &run.ledger("created", json!({})))?;
        Ok(run)
    }

    fn write_request(&self, input: &InspectionInput) -> Result<(), String> {
        let request = json!({
            "prompt": input.prompt,
            "packet": input.packet,
            "packetHash": input.packet_hash,
        });
        reject_sensitive_markers(&request, "App Intelligence request artifact")?;
        atomic_write_json(&self.request_json, &request)
    }

    fn write_observation(&self, observation: &Value) -> Result<(), String> {
        atomic_write_json(&self.observation_json, observation)
    }

    fn write_rejected_output(&self, output: &str) -> Result<(), String> {
        atomic_write_json(&self.rejected_output_json, &json!({ "output": output }))
    }

    fn append_codex_event(&self, value: &Value) -> Result<(), String> {
        append_jsonl(&self.codex_events_jsonl, value)
    }

    fn write_event(&self, event: &str, data: Value) -> Result<(), String> {
        append_jsonl(
            &self.events_jsonl,
            &json!({
                "timestamp": Utc::now().to_rfc3339(),
                "event": event,
                "data": data,
            }),
        )
    }

    fn ledger(&self, status: &str, codex: Value) -> Value {
        json!({
            "runId": self.input_summary["runId"],
            "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
            "mode": "read-only-inspection",
            "createdAt": self.input_summary["createdAt"],
            "workspaceId": self.input_summary["workspaceId"],
            "contextPacketHash": self.input_summary["contextPacketHash"],
            "status": status,
            "codex": codex,
            "policy": {
                "readOnly": true,
                "allowedActions": ["inspect_context", "summarize", "recommend_next_inspection"],
                "forbiddenActions": ["browser_mutation", "service_mutation", "file_write", "shell_command"],
            },
            "artifacts": {
                "runPath": self.run_json,
                "requestPath": self.request_json,
                "eventLogPath": self.codex_events_jsonl,
                "normalizedEventLogPath": self.events_jsonl,
                "observationPath": self.observation_json,
            },
        })
    }

    fn finalize(&self, status: &str, codex: &Value) -> Result<Value, String> {
        let ledger = self.ledger(status, codex.clone());
        atomic_write_json(&self.run_json, &ledger)?;
        self.write_event("run_finished", json!({ "status": status }))?;
        Ok(ledger)
    }

    fn failure(&self, code: &'static str, message: String) -> InspectionFailure {
        self.write_event("failure", json!({ "code": code, "message": message }))
            .ok();
        InspectionFailure {
            code,
            message,
            ledger: self.finalize("failed", &json!({})).ok(),
        }
    }
}

struct CodexAppServerClient {
    child: Child,
    stdin: ChildStdin,
    rx: mpsc::Receiver<LineEvent>,
}

enum LineEvent {
    Stdout(Value),
    Stderr(String),
    Eof,
}

impl CodexAppServerClient {
    fn start(bin: &str, run: &InspectionRun) -> Result<Self, String> {
        let mut child = Command::new(bin)
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| format!("Failed to start codex app-server: {err}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to open codex app-server stdin.".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to open codex app-server stdout.".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to open codex app-server stderr.".to_string())?;
        let (tx, rx) = mpsc::channel();
        let stdout_tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                match serde_json::from_str::<Value>(&line) {
                    Ok(value) => {
                        let _ = stdout_tx.send(LineEvent::Stdout(value));
                    }
                    Err(_) => {
                        let _ = stdout_tx.send(LineEvent::Stderr(format!(
                            "Non-JSON stdout from codex app-server: {line}"
                        )));
                    }
                }
            }
            let _ = stdout_tx.send(LineEvent::Eof);
        });
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let _ = tx.send(LineEvent::Stderr(line));
            }
        });
        run.write_event("codex_started", json!({ "bin": bin })).ok();
        Ok(Self { child, stdin, rx })
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        let mut notification = json!({ "method": method });
        if !params.is_null() {
            notification["params"] = params;
        }
        self.write_message(&notification)
    }

    fn request(&mut self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.write_message(&json!({
            "id": id,
            "method": method,
            "params": params,
        }))?;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!(
                    "Timed out waiting for app-server response to {method}"
                ));
            }
            match self.rx.recv_timeout(remaining) {
                Ok(LineEvent::Stdout(value)) => {
                    if value.get("id").and_then(Value::as_str) == Some(id.as_str()) {
                        if let Some(error) = value.get("error") {
                            return Err(format!("{method} failed: {error}"));
                        }
                        return Ok(value.get("result").cloned().unwrap_or(Value::Null));
                    }
                }
                Ok(LineEvent::Stderr(line)) => {
                    if !line.trim().is_empty() {
                        // Keep stderr available through the raw event stream.
                    }
                }
                Ok(LineEvent::Eof) => {
                    return Err("codex app-server exited before responding".to_string())
                }
                Err(_) => {
                    return Err(format!(
                        "Timed out waiting for app-server response to {method}"
                    ))
                }
            }
        }
    }

    fn collect_turn_output(
        &mut self,
        thread_id: Option<&str>,
        turn_id: Option<&str>,
        timeout: Duration,
        run: &InspectionRun,
    ) -> Result<String, (&'static str, String)> {
        let deadline = Instant::now() + timeout;
        let mut output = String::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err((
                    "timeout",
                    "Timed out waiting for Codex turn completion.".to_string(),
                ));
            }
            match self.rx.recv_timeout(remaining) {
                Ok(LineEvent::Stdout(value)) => {
                    run.append_codex_event(&value).ok();
                    if let Some(text) = response_item_output_text(&value, thread_id, turn_id) {
                        output.push_str(&text);
                    }
                    if is_turn_completed(&value, thread_id, turn_id) {
                        return Ok(output);
                    }
                    if is_policy_event(&value) {
                        return Err((
                            "policy_violation",
                            format!("Codex attempted a forbidden event: {value}"),
                        ));
                    }
                }
                Ok(LineEvent::Stderr(line)) => {
                    run.append_codex_event(&json!({ "stderr": line })).ok();
                }
                Ok(LineEvent::Eof) => {
                    return Err((
                        "protocol_error",
                        "codex app-server exited before turn completion.".to_string(),
                    ));
                }
                Err(_) => {
                    return Err((
                        "timeout",
                        "Timed out waiting for Codex turn event.".to_string(),
                    ));
                }
            }
        }
    }

    fn write_message(&mut self, value: &Value) -> Result<(), String> {
        let line = serde_json::to_string(value).map_err(|err| err.to_string())?;
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|err| format!("Failed to write to codex app-server: {err}"))
    }
}

impl Drop for CodexAppServerClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn build_prompt(input: &InspectionInput) -> String {
    let schema = observation_output_schema();
    format!(
        "Inspect the selected browser workspace using only the redacted packet below.\n\
         Return exactly one JSON object matching the observation schema. No markdown.\n\
         Read-only policy: do not run commands, call tools, edit files, mutate browser state, mutate service state, inspect storage secrets, or request approvals.\n\
         runId: {}\ncreatedAt: {}\nworkspaceId: {}\noperatorRequest: {}\n\
         Observation schema:\n{}\n\
         Selected workspace packet:\n{}",
        input.run_id,
        input.created_at,
        input.workspace_id.as_deref().unwrap_or("null"),
        input.prompt,
        serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string_pretty(&input.packet).unwrap_or_else(|_| "{}".to_string()),
    )
}

fn response_item_output_text(
    value: &Value,
    thread_id: Option<&str>,
    turn_id: Option<&str>,
) -> Option<String> {
    let method = value.get("method").and_then(Value::as_str)?;
    if let Some(expected) = thread_id {
        if value.pointer("/params/threadId").and_then(Value::as_str) != Some(expected) {
            return None;
        }
    }
    if let Some(expected) = turn_id {
        if value.pointer("/params/turnId").and_then(Value::as_str) != Some(expected) {
            return None;
        }
    }
    match method {
        "rawResponseItem/completed" => {
            let item = value.pointer("/params/item")?;
            if item.get("type").and_then(Value::as_str) != Some("message") {
                return None;
            }
            let mut output = String::new();
            for content in item.get("content").and_then(Value::as_array)? {
                if content.get("type").and_then(Value::as_str) == Some("output_text") {
                    if let Some(text) = content.get("text").and_then(Value::as_str) {
                        output.push_str(text);
                    }
                }
            }
            Some(output)
        }
        "item/agentMessage/delta" => value
            .pointer("/params/delta")
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

fn is_turn_completed(value: &Value, thread_id: Option<&str>, turn_id: Option<&str>) -> bool {
    if value.get("method").and_then(Value::as_str) != Some("turn/completed") {
        return false;
    }
    if let Some(expected) = thread_id {
        if value.pointer("/params/threadId").and_then(Value::as_str) != Some(expected) {
            return false;
        }
    }
    if let Some(expected) = turn_id {
        if value.pointer("/params/turn/id").and_then(Value::as_str) != Some(expected) {
            return false;
        }
    }
    true
}

fn is_policy_event(value: &Value) -> bool {
    matches!(
        value.get("method").and_then(Value::as_str),
        Some("item/commandExecution/requestApproval")
            | Some("item/fileChange/requestApproval")
            | Some("item/commandExecution/outputDelta")
            | Some("item/fileChange/outputDelta")
            | Some("command/exec/outputDelta")
            | Some("process/outputDelta")
    )
}

fn parse_observation_output(output: &str) -> Result<Value, String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err("Codex returned an empty observation.".to_string());
    }
    serde_json::from_str(trimmed).or_else(|_| {
        let start = trimmed
            .find('{')
            .ok_or_else(|| "Codex output did not contain JSON.".to_string())?;
        let end = trimmed
            .rfind('}')
            .ok_or_else(|| "Codex output did not contain a complete JSON object.".to_string())?;
        serde_json::from_str(&trimmed[start..=end])
            .map_err(|err| format!("Codex observation JSON was invalid: {err}"))
    })
}

fn reject_policy_violations(observation: &Value) -> Result<(), String> {
    let text = serde_json::to_string(observation)
        .unwrap_or_default()
        .to_lowercase();
    for marker in [
        "click ",
        "navigate",
        "delete",
        "clear storage",
        "run command",
        "execute",
        "service request",
        "write file",
    ] {
        if text.contains(marker) {
            return Err(format!(
                "Observation contains action-like text forbidden in this slice: {marker}"
            ));
        }
    }
    Ok(())
}

fn build_deterministic_observation(input: &InspectionInput) -> Value {
    let workspace = input.packet.get("workspace").unwrap_or(&Value::Null);
    let runtime = input.packet.get("runtime").unwrap_or(&Value::Null);
    let label = workspace
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("selected workspace");
    let state = workspace
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let live = workspace
        .get("live")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let viewable = workspace
        .get("viewable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let controllable = workspace
        .get("controllable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let pid = runtime
        .get("pid")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string());
    let stream_port = runtime
        .get("streamPort")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string());
    json!({
        "version": CODEX_WORKSPACE_OBSERVATION_VERSION,
        "provider": CONTEXTUAL_CHAT_PROVIDER_ID,
        "runId": input.run_id,
        "threadId": Value::Null,
        "createdAt": input.created_at,
        "workspaceId": input.workspace_id,
        "summary": format!(
            "Codex app server supervised read-only inspection for {label}: state={state}, live={live}, viewable={viewable}, controllable={controllable}, pid={pid}, streamPort={stream_port}, request={}",
            input.prompt
        ),
        "detectedState": state,
        "blockers": [{
            "severity": if live { "info" } else { "blocked" },
            "summary": if live { "No selected-workspace blocker is visible in the redacted context packet." } else { "The selected workspace is not reporting a live browser." },
            "evidenceIds": ["workspace.summary"],
        }],
        "risks": [{
            "summary": "Only redacted selected-workspace evidence is available; later evidence providers remain placeholders.",
            "evidenceIds": ["activity.unavailable", "console.unavailable", "network.unavailable", "storage.unavailable", "extensions.unavailable"],
        }],
        "suggestedNextInspections": [{
            "label": "Review workspace readiness",
            "reason": "The redacted workspace packet is the enabled evidence source for this inspection.",
            "evidenceIds": ["workspace.summary"],
        }],
        "unsupportedActions": [{
            "label": "Mutate browser or service state",
            "reason": "This App Intelligence supervisor is read-only.",
        }],
        "confidence": if live { "medium" } else { "low" },
    })
}

pub(crate) fn resolve_codex_bin() -> String {
    if let Ok(path) = env::var("AGENT_BROWSER_CODEX_BIN") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if command_exists("codex") {
        return "codex".to_string();
    }

    let mut candidates = Vec::new();
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local/bin/codex"));
        candidates.push(home.join(".openclaw/extensions/codex/node_modules/.bin/codex"));
        candidates.push(home.join(".codex/bin/codex"));
    }

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| "codex".to_string())
}

fn command_exists(name: &str) -> bool {
    env::var_os("PATH")
        .map(|paths| {
            env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(name);
                candidate.is_file()
            })
        })
        .unwrap_or(false)
}

fn codex_cli_version(codex_bin: &str) -> String {
    Command::new(codex_bin)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn app_intelligence_run_root() -> Option<PathBuf> {
    if let Ok(path) = env::var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    dirs::home_dir().map(|home| home.join(".agent-browser/app-intelligence/runs"))
}

fn atomic_write_json(path: &Path, value: &Value) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(|err| err.to_string())?;
    fs::write(&tmp, bytes).map_err(|err| format!("Failed to write {}: {err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "Failed to move {} to {}: {err}",
            tmp.display(),
            path.display()
        )
    })
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("Failed to open {}: {err}", path.display()))?;
    let line = serde_json::to_string(value).map_err(|err| err.to_string())?;
    writeln!(file, "{line}").map_err(|err| format!("Failed to append {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_input(root: &Path) -> InspectionInput {
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT", root);
        env::set_var("AGENT_BROWSER_APP_INTELLIGENCE_MODE", "deterministic");
        InspectionInput {
            run_id: format!("codex-inspect-{}", uuid::Uuid::new_v4()),
            created_at: "2026-05-31T00:00:00Z".to_string(),
            prompt: "inspect".to_string(),
            packet_hash: "abc123".to_string(),
            workspace_id: Some("browser:session:default".to_string()),
            packet: json!({
                "workspace": {
                    "id": "browser:session:default",
                    "label": "default",
                    "state": "active",
                    "live": true,
                    "viewable": true,
                    "controllable": true
                },
                "runtime": {"pid": 123, "streamPort": 38395},
                "evidence": [
                    {"id": "workspace.summary"},
                    {"id": "activity.unavailable"},
                    {"id": "console.unavailable"},
                    {"id": "network.unavailable"},
                    {"id": "storage.unavailable"},
                    {"id": "extensions.unavailable"}
                ]
            }),
        }
    }

    #[test]
    fn deterministic_mode_writes_run_directory() {
        let root = env::temp_dir().join(format!(
            "agent-browser-app-intel-supervisor-{}",
            uuid::Uuid::new_v4()
        ));
        let input = fixture_input(&root);
        let run_id = input.run_id.clone();
        let result = inspect_with_supervisor(input).expect("deterministic inspection");
        assert_eq!(result.observation["provider"], CONTEXTUAL_CHAT_PROVIDER_ID);
        let run_root = root.join(run_id);
        assert!(run_root.join("run.json").exists());
        assert!(run_root.join("request.json").exists());
        assert!(run_root.join("events.jsonl").exists());
        assert!(run_root.join("observation.json").exists());
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT");
        env::remove_var("AGENT_BROWSER_APP_INTELLIGENCE_MODE");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parses_raw_response_item_output() {
        let value = json!({
            "method": "rawResponseItem/completed",
            "params": {
                "threadId": "t1",
                "turnId": "u1",
                "item": {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "{\"ok\":true}"}]
                }
            }
        });
        assert_eq!(
            response_item_output_text(&value, Some("t1"), Some("u1")).as_deref(),
            Some("{\"ok\":true}")
        );
    }
}
