//! Read original task operations without starting a daemon or browser.
use super::{required_string_argument, JsonRpcError};
use crate::native::browser_session_store::BrowserSessionSqliteStore;
use agent_browser_service_model::BrowserTaskOperation;
use serde_json::{json, Value};

pub(super) fn projection(operation: &BrowserTaskOperation) -> Value {
    json!({"requestId":operation.request_id,"taskId":operation.task_id,
        "profileId":operation.profile_id,"browserId":operation.browser_id,
        "tabId":operation.tab_id,"targetId":operation.target_id,
        "processPid":operation.process_pid,"controlEpoch":operation.control_epoch,
        "action":operation.action,"state":operation.state,
        "createdAtMs":operation.created_at_ms,"completedAtMs":operation.completed_at_ms,
        "evidenceReferences":operation.evidence_references,"retryAllowed":false,
        "siteOutcome":"unverified"})
}

pub(super) fn lookup(task_id: &str, request_id: &str) -> Result<Value, String> {
    let path = BrowserSessionSqliteStore::default_sqlite_path()?;
    let store = BrowserSessionSqliteStore::open_read_only(&path)?;
    let state = store.load_session_state()?;
    let operation = state
        .task_operations
        .get(task_id)
        .and_then(|rows| rows.get(request_id))
        .filter(|row| row.task_id == task_id && row.request_id == request_id)
        .ok_or("browser_task_operation_not_found")?;
    let current = state.sessions.get(task_id);
    let unchanged = current.is_some_and(|task| {
        task.browser_id == operation.browser_id
            && task.profile_id == operation.profile_id
            && task.current_tab_id.as_deref() == Some(&operation.tab_id)
    }) && state
        .browsers
        .get(&operation.browser_id)
        .is_some_and(|browser| {
            browser.pid == operation.process_pid
                && operation.process_identity.is_some()
                && store.original_browser_identity(&state, browser) == operation.process_identity
        })
        && state
            .tabs
            .get(&operation.tab_id)
            .is_some_and(|tab| tab.target_id == operation.target_id)
        && crate::native::browser_host_control::read(&operation.browser_id)?.epoch
            == operation.control_epoch;
    Ok(
        json!({"schemaVersion":"agent-browser.task-operation.v1","operation":projection(operation),
        "currentContext":if unchanged {"retained_matches"} else {"changed_or_missing"},
        "liveObservation":"not_performed","nextAction":"inspect_current_task_do_not_replay_original"}),
    )
}

pub(super) fn call(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let task_id = required_string_argument(arguments, "taskId")?;
    let request_id = required_string_argument(&arguments["params"], "requestId")?;
    let data = lookup(task_id, request_id).map_err(|error| JsonRpcError::invalid_params(&error))?;
    Ok(super::tool_response_from_payload(
        "browser_command",
        session,
        json!({}),
        data,
        false,
    ))
}

#[derive(Clone)]
struct RequestContext {
    task_id: String,
    request_id: Option<String>,
}
thread_local! {
    static REQUEST_CONTEXT: std::cell::RefCell<Option<RequestContext>> = const { std::cell::RefCell::new(None) };
}
struct RestoreContext(Option<RequestContext>);
impl Drop for RestoreContext {
    fn drop(&mut self) {
        REQUEST_CONTEXT.with(|context| {
            context.replace(self.0.take());
        });
    }
}

/// Scoped synchronous dispatcher context only. Restoration also covers errors
/// and nested calls; no borrow or context guard crosses an async yield.
pub(super) fn with_context(
    params: Option<&Value>,
    run: impl FnOnce() -> Result<Value, JsonRpcError>,
) -> Result<Value, JsonRpcError> {
    let name = params
        .and_then(|value| value["name"].as_str())
        .unwrap_or_default();
    let arguments = params
        .map(|value| &value["arguments"])
        .unwrap_or(&Value::Null);
    let eligible = matches!(
        name,
        "browser_navigate"
            | "browser_click"
            | "browser_fill"
            | "browser_screenshot"
            | "browser_artifact"
    ) || name == "browser_command"
        && crate::native::browser_task_operations::eligible_action(
            arguments["action"].as_str().unwrap_or_default(),
        );
    let native = name == "browser_command"
        && crate::native::browser_task_operations::eligible_native(
            arguments["action"].as_str().unwrap_or_default(),
            &arguments["params"],
        );
    let eligible = eligible || native;
    let addressed = arguments["taskId"]
        .as_str()
        .is_some_and(|id| !id.trim().is_empty());
    if native && !addressed {
        return Err(JsonRpcError::invalid_params(
            "Native challenge observation/attempt requires taskId",
        ));
    }
    let next = if eligible && addressed {
        let request_id = match arguments.get("requestId") {
            None => None,
            Some(value) => Some(
                value
                    .as_str()
                    .filter(|id| crate::native::browser_task_operations::valid_request_id(id))
                    .ok_or_else(|| {
                        JsonRpcError::invalid_params(
                            "requestId must be an opaque 1-128 character identifier",
                        )
                    })?
                    .into(),
            ),
        };
        if native && request_id.is_none() {
            return Err(JsonRpcError::invalid_params(
                "Native challenge observation/attempt requires stable requestId",
            ));
        }
        arguments["taskId"]
            .as_str()
            .filter(|id| !id.trim().is_empty())
            .map(|task_id| RequestContext {
                task_id: task_id.into(),
                request_id,
            })
    } else {
        None
    };
    let _restore = RestoreContext(REQUEST_CONTEXT.with(|context| context.replace(next)));
    run()
}

fn response_data(
    response: &mut crate::connection::Response,
    operation: &BrowserTaskOperation,
    replayed: bool,
) {
    let receipt = projection(operation);
    let mut data = match response.data.take() {
        Some(Value::Object(object)) => object,
        other => serde_json::Map::from_iter([("result".into(), other.unwrap_or(Value::Null))]),
    };
    data.insert("taskOperation".into(), receipt);
    data.insert("replayed".into(), json!(replayed));
    data.insert("dispatchAttempted".into(), json!(!replayed));
    response.data = Some(Value::Object(data));
}

/// Return only reviewed static completion diagnostics. Store errors can contain
/// SQL, paths or private values; their details never enter the public envelope.
pub(super) fn completion_failure_cause(error: &str) -> &'static str {
    match error.split(':').next().unwrap_or_default() {
        "browser_native_original_observation_invalid" => {
            "browser_native_original_observation_invalid"
        }
        "browser_native_original_observation_expired" => {
            "browser_native_original_observation_expired"
        }
        "browser_task_operation_original_missing" => "browser_task_operation_original_missing",
        "browser_task_operation_completion_conflict" => {
            "browser_task_operation_completion_conflict"
        }
        "browser_task_operation_clock_unavailable" => "browser_task_operation_clock_unavailable",
        "browser_session_publication_idle_retirement_fenced" => {
            "browser_session_publication_idle_retirement_fenced"
        }
        "browser_session_mutation_begin_failed"
        | "browser_session_mutation_commit_failed"
        | "browser_runtime_database_open_failed"
        | "browser_runtime_database_read_only_unavailable"
        | "browser_runtime_database_read_only_open_failed"
        | "browser_runtime_database_busy_timeout_failed"
        | "browser_runtime_database_pragma_failed"
        | "browser_runtime_database_schema_failed"
        | "browser_runtime_database_schema_read_failed"
        | "browser_runtime_document_serialize_failed"
        | "browser_runtime_document_save_failed"
        | "browser_runtime_document_read_failed"
        | "browser_runtime_document_invalid"
        | "browser_runtime_document_schema_unsupported"
        | "terminal_history_previous_read_failed"
        | "terminal_history_previous_invalid"
        | "terminal_history_archive_unavailable"
        | "terminal_history_archive_write_failed"
        | "terminal_history_document_invalid"
        | "terminal_history_document_serialize_failed" => "store_unavailable",
        _ => "unclassified",
    }
}

/// The marker is durable before this closure can ensure a transport or send.
pub(super) fn transport(
    mut command: Value,
    run: impl FnOnce(Value) -> Result<crate::connection::Response, String>,
) -> Result<crate::connection::Response, String> {
    use crate::native::browser_task_operations::{self as operations, Admission};
    let context = REQUEST_CONTEXT.with(|context| context.borrow().clone());
    let Some(context) = context.filter(|_| {
        operations::eligible_action(command["action"].as_str().unwrap_or_default())
            || operations::eligible_native(command["action"].as_str().unwrap_or_default(), &command)
    }) else {
        return run(command);
    };
    if let Some(request_id) = context.request_id {
        command["id"] = json!(request_id);
    }
    super::workflow::address_task_command(&mut command, &context.task_id)?;
    let admitted =
        if operations::eligible_native(command["action"].as_str().unwrap_or_default(), &command) {
            operations::begin_native(&context.task_id, &mut command)?
        } else {
            operations::begin(&context.task_id, &mut command)?
        };
    match admitted {
        Admission::Existing(original) => {
            let mut response = crate::connection::Response {
                success: original.state
                    == agent_browser_service_model::BrowserTaskOperationState::OwnerAcknowledged,
                id: Some(original.request_id.clone()),
                error: (original.state
                    != agent_browser_service_model::BrowserTaskOperationState::OwnerAcknowledged)
                    .then(|| "browser_task_operation_uncertain: inspect original; no retry".into()),
                ..Default::default()
            };
            response_data(&mut response, &original, true);
            Ok(response)
        }
        Admission::Dispatch(original) => match run(command) {
            Ok(mut response) => {
                let completed=operations::finish(&original,Some(&response)).map_err(|error|format!(
                        "browser_task_operation_completion_unavailable:{}:inspect original; no retry;cause={}",original.request_id,completion_failure_cause(&error)))?;
                response_data(&mut response, &completed, false);
                Ok(response)
            }
            Err(_) => {
                let _ = operations::finish(&original, None);
                Err(format!(
                    "browser_task_operation_uncertain:{}:inspect original; no retry",
                    original.request_id
                ))
            }
        },
    }
}

pub(super) fn send(mut command: Value, route: &str) -> Result<crate::connection::Response, String> {
    let context = REQUEST_CONTEXT.with(|context| context.borrow().clone());
    let task_id = context
        .as_ref()
        .map(|context| context.task_id.as_str())
        .or_else(|| route.starts_with("session:").then_some(route));
    let lane = if let Some(task_id) = task_id {
        super::workflow::address_task_command(&mut command, task_id)?
    } else {
        route.into()
    };
    transport(command, |command| {
        crate::connection::send_command(command, &lane)
    })
}
