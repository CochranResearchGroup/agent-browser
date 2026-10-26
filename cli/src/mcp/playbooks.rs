//! Shared website routine discovery over the existing browser-task interface.
//! This layer does not acquire a second browser or own credentials.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::JsonRpcError;
mod retrieval;
mod teaching;

pub(super) const OPERATIONS: &[&str] = &[
    "siteguides",
    "inspect_guide",
    "verify_login",
    "prepare_login",
    "validate_plan",
    "collect_posts",
];

pub(super) fn schema(operation: &str) -> Result<Value, JsonRpcError> {
    let mut properties = match operation {
        "siteguides" => json!({}),
        "inspect_guide" => json!({"guideId":{"type":"string"}}),
        "collect_posts" => {
            json!({"guideId":{"type":"string"},"maxItems":{"type":"integer","minimum":1,"maximum":200},
            "maxPages":{"type":"integer","minimum":1,"maximum":10},"maxScrolls":{"type":"integer","minimum":0,"maximum":10},
            "maxObservationAgeMs":{"type":"integer","minimum":1,"maximum":60000}})
        }
        "verify_login" | "prepare_login" | "validate_plan" => json!({"guideId":{"type":"string"},
            "accountRef":{"type":"string"},"organizationRef":{"type":"string"},
            "maxObservationAgeMs":{"type":"integer","minimum":1,"maximum":60000}}),
        _ => return Err(JsonRpcError::invalid_params("Unknown playbook operation")),
    };
    if operation == "validate_plan" {
        properties["plan"] =
            json!({"type":"object","description":"The unchanged plan returned by prepare_login."});
    }
    let required = match operation {
        "siteguides" => json!([]),
        "inspect_guide" => json!(["guideId"]),
        "collect_posts" => json!(["guideId"]),
        "validate_plan" => json!(["guideId", "accountRef", "organizationRef", "plan"]),
        _ => json!(["guideId", "accountRef", "organizationRef"]),
    };
    Ok(json!({"type":"object","additionalProperties":false,
        "properties":properties,"required":required}))
}

pub(super) fn discover(arguments: &Value, session: &str) -> Result<Value, JsonRpcError> {
    let data = match super::optional_string_argument(arguments, "operation")? {
        Some(operation) => json!({"category":"playbooks","operation":operation,
            "inputSchema":schema(operation)?,
            "execution":{"tool":"browser_command","action":"playbook",
                "operation":operation,"arguments":"Supply taskId and put operation plus routine arguments in params."}}),
        None => json!({"category":"playbooks","operations":OPERATIONS,
            "next":"Request one operation for its schema."}),
    };
    Ok(super::tool_response_from_payload(
        "browser_discover",
        session,
        json!({}),
        data,
        false,
    ))
}

use crate::native::browser_session_store::BrowserSessionSqliteStore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Instant;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IdentityField {
    selector: String,
    attribute: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageState {
    state: String,
    selector: String,
}

/// Reviewed semantic controls consumed by a trusted private receiving adapter.
/// They identify fields and buttons, never credential values or account choice.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LoginControls {
    identifier: String,
    password: String,
    submit: String,
    account_scope: String,
    identifier_submit: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SiteGuide {
    schema_version: String,
    guide_id: String,
    name: String,
    allowed_origins: Vec<String>,
    authenticated_selector: Option<String>,
    account: Option<IdentityField>,
    organization: Option<IdentityField>,
    states: Vec<PageState>,
    controls: Option<LoginControls>,
    retrieval: Option<retrieval::RetrievalGuide>,
    /// Inert agent-prepared teaching knowledge; inspection is not execution approval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    teaching: Option<teaching::TeachingDraft>,
}

/// A bounded observation plan is task-bound metadata, not an access grant.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreparedLoginPlan {
    plan_id: String,
    guide_id: String,
    guide_digest: String,
    binding_digest: String,
    control_fingerprint: String,
    expected_state: String,
    task_id: String,
    profile_id: String,
    browser_id: String,
    tab_id: String,
    prepared_at_ms: u64,
    maximum_age_ms: u64,
}

fn digest(value: &impl Serialize) -> Result<String, JsonRpcError> {
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(value).map_err(|_| fault("playbook_metadata_invalid"))?,
    )))
}

fn now_ms() -> Result<u64, JsonRpcError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| fault("playbook_clock_invalid"))?;
    u64::try_from(elapsed.as_millis()).map_err(|_| fault("playbook_clock_invalid"))
}

fn fault(reason: &str) -> JsonRpcError {
    JsonRpcError::invalid_params(reason)
}

fn load_guide(guide_id: &str) -> Result<SiteGuide, JsonRpcError> {
    if guide_id.is_empty()
        || guide_id.len() > 120
        || !guide_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(fault("siteguide_id_invalid"));
    }
    let bytes = if let Some(directory) = std::env::var_os("AGENT_BROWSER_SITEGUIDE_DIRECTORY") {
        let path = std::path::PathBuf::from(directory).join(format!("{guide_id}.json"));
        let metadata = fs::symlink_metadata(&path).map_err(|_| fault("siteguide_not_found"))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 65_536 {
            return Err(fault("siteguide_file_invalid"));
        }
        fs::read(path).map_err(|_| fault("siteguide_read_failed"))?
    } else if guide_id == "bill-session-v1" {
        include_bytes!("../../../config/siteguides/bill-session-v1.json").to_vec()
    } else if guide_id == "qbo-session-v1" {
        include_bytes!("../../../config/siteguides/qbo-session-v1.json").to_vec()
    } else if guide_id == "reddit-feed-v1" {
        include_bytes!("../../../config/siteguides/reddit-feed-v1.json").to_vec()
    } else {
        return Err(fault("siteguide_not_found"));
    };
    let guide: SiteGuide =
        serde_json::from_slice(&bytes).map_err(|_| fault("siteguide_invalid"))?;
    if guide.schema_version != "agent-browser.siteguide.v1"
        || guide.guide_id != guide_id
        || guide.allowed_origins.is_empty()
        || guide.allowed_origins.len() > 16
        || guide.states.len() > 16
        || guide.name.is_empty()
        || guide.name.len() > 256
    {
        return Err(fault("siteguide_contract_invalid"));
    }
    for origin in &guide.allowed_origins {
        let url = url::Url::parse(origin).map_err(|_| fault("siteguide_origin_invalid"))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.origin().ascii_serialization() != *origin
        {
            return Err(fault("siteguide_origin_invalid"));
        }
    }
    if guide.retrieval.is_none()
        && guide.teaching.is_none()
        && (guide.authenticated_selector.is_none()
            || guide.account.is_none()
            || guide.organization.is_none())
    {
        return Err(fault("siteguide_contract_invalid"));
    }
    if let Some(teaching) = &guide.teaching {
        teaching.validate()?;
    }
    if let Some(retrieval) = &guide.retrieval {
        retrieval.validate()?;
    }
    let selectors = guide
        .authenticated_selector
        .as_deref()
        .into_iter()
        .chain(guide.account.iter().map(|field| field.selector.as_str()))
        .chain(
            guide
                .organization
                .iter()
                .map(|field| field.selector.as_str()),
        )
        .chain(guide.states.iter().map(|state| state.selector.as_str()));
    if selectors
        .into_iter()
        .any(|selector| selector.is_empty() || selector.len() > 512)
    {
        return Err(fault("siteguide_selector_invalid"));
    }
    if let Some(controls) = &guide.controls {
        let selectors = [
            &controls.identifier,
            &controls.password,
            &controls.submit,
            &controls.account_scope,
        ];
        if selectors
            .iter()
            .any(|selector| selector.is_empty() || selector.len() > 512)
            || controls
                .identifier_submit
                .as_ref()
                .is_some_and(|selector| selector.is_empty() || selector.len() > 512)
        {
            return Err(fault("siteguide_control_invalid"));
        }
    }
    for field in guide.account.iter().chain(guide.organization.iter()) {
        if field.attribute.as_ref().is_some_and(|attribute| {
            attribute.len() > 120
                || !(attribute.starts_with("data-")
                    || attribute == "aria-label"
                    || attribute == "title")
        }) {
            return Err(fault("siteguide_identity_attribute_invalid"));
        }
    }
    if guide.states.iter().any(|state| {
        !matches!(
            state.state.as_str(),
            "identifier_required"
                | "password_required"
                | "mfa_required"
                | "login_required"
                | "blocked"
        )
    }) {
        return Err(fault("siteguide_state_invalid"));
    }
    Ok(guide)
}

fn task_context(task_id: &str) -> Result<Value, JsonRpcError> {
    let path = BrowserSessionSqliteStore::default_sqlite_path()
        .map_err(|_| fault("playbook_task_store_unavailable"))?;
    let store = BrowserSessionSqliteStore::open_read_only(&path)
        .map_err(|_| fault("playbook_task_store_unavailable"))?;
    let state = store
        .load_session_state()
        .map_err(|_| fault("playbook_task_store_unavailable"))?;
    let task = state
        .sessions
        .get(task_id)
        .ok_or_else(|| fault("browser_task_not_found"))?;
    let tab_id = task
        .current_tab_id
        .as_deref()
        .filter(|id| state.tabs.contains_key(*id))
        .ok_or_else(|| fault("browser_task_current_tab_missing"))?;
    Ok(
        json!({"taskId":task_id,"profileId":task.profile_id,"browserId":task.browser_id,
        "tabId":tab_id,"sessionName":task.name}),
    )
}

fn required_reference(arguments: &Value, name: &str) -> Result<String, JsonRpcError> {
    let value = super::required_string_argument(arguments, name)?;
    if value.len() > 256 {
        return Err(fault("playbook_identity_invalid"));
    }
    Ok(value.to_string())
}

/// Observe only identity predicates and state markers. Passwords, challenge
/// values, full page bodies and account text never cross the routine boundary.
pub(super) fn call(arguments: &Value, transport_session: &str) -> Result<Value, JsonRpcError> {
    let task_id = super::required_string_argument(arguments, "taskId")?;
    let params = arguments
        .get("params")
        .ok_or_else(|| fault("playbook_params_required"))?;
    let operation = super::required_string_argument(params, "operation")?;
    let contract = schema(operation)?;
    let properties = contract["properties"]
        .as_object()
        .expect("schema properties");
    for key in params
        .as_object()
        .ok_or_else(|| fault("playbook_params_invalid"))?
        .keys()
    {
        if key != "operation" && !properties.contains_key(key) {
            return Err(fault("playbook_argument_unknown"));
        }
    }
    task_context(task_id)?;
    if operation == "siteguides" {
        let data = json!({"guides":guide_names()?});
        return Ok(super::tool_response_from_payload(
            "browser_command",
            transport_session,
            json!({}),
            data,
            false,
        ));
    }
    if operation == "inspect_guide" {
        let guide = load_guide(super::required_string_argument(params, "guideId")?)?;
        let data = serde_json::to_value(guide).map_err(|_| fault("siteguide_metadata_invalid"))?;
        return Ok(super::tool_response_from_payload(
            "browser_command",
            transport_session,
            json!({}),
            data,
            false,
        ));
    }
    let guide = load_guide(super::required_string_argument(params, "guideId")?)?;
    if guide.teaching.is_some() {
        return Err(fault("teaching_draft_not_executable"));
    }
    if operation == "collect_posts" {
        return retrieval::collect(&guide, params, task_id, transport_session);
    }
    let authenticated = guide
        .authenticated_selector
        .as_ref()
        .ok_or_else(|| fault("siteguide_login_contract_missing"))?;
    let account_field = guide
        .account
        .as_ref()
        .ok_or_else(|| fault("siteguide_login_contract_missing"))?;
    let organization_field = guide
        .organization
        .as_ref()
        .ok_or_else(|| fault("siteguide_login_contract_missing"))?;
    let account = required_reference(params, "accountRef")?;
    let organization = required_reference(params, "organizationRef")?;
    let maximum_age = match params.get("maxObservationAgeMs") {
        Some(value) => value
            .as_u64()
            .filter(|age| (1..=60_000).contains(age))
            .ok_or_else(|| fault("playbook_observation_age_invalid"))?,
        None => 10_000,
    };
    let binding = task_context(task_id)?;
    let guide_digest = digest(&guide)?;
    let binding_digest = digest(&json!([binding, account, organization]))?;
    let descriptor = json!({"origins":guide.allowed_origins,"authenticated":authenticated,
        "account":{"selector":account_field.selector,"attribute":account_field.attribute,"expected":account},
        "organization":{"selector":organization_field.selector,"attribute":organization_field.attribute,"expected":organization},
        "states":guide.states.iter().map(|state|json!({"state":state.state,"selector":state.selector})).collect::<Vec<_>>(),
        "controls":guide.controls});
    let script = format!(
        r#"(() => {{
        const guide = {descriptor};
        const origin = location.origin;
        if (!guide.origins.includes(origin)) return {{origin, originAllowed:false}};
        const matchIdentity = field => {{
            const elements = document.querySelectorAll(field.selector);
            if (elements.length !== 1 || elements[0].matches('input[type=password]')) return false;
            const value = field.attribute ? elements[0].getAttribute(field.attribute) : elements[0].textContent;
            return typeof value === 'string' && value.trim() === field.expected;
        }};
        const visible = element => element && element.getClientRects().length > 0 && getComputedStyle(element).visibility !== 'hidden';
        const applications = [...document.querySelectorAll(guide.authenticated)].filter(visible);
        const authenticated = applications.length === 1;
        const state = guide.states.find(state => [...document.querySelectorAll(state.selector)].some(visible));
        const controlState = Object.entries(guide.controls || {{}}).filter(([,selector])=>typeof selector === 'string').map(([name,selector])=>{{
            const elements = document.querySelectorAll(selector);
            const element = elements.length === 1 ? elements[0] : null;
            return {{name,count:elements.length,tag:element ? element.tagName : null,type:element && typeof element.type === 'string' ? element.type : null}};
        }});
        return {{origin,originAllowed:true,authenticated,
            accountMatched:authenticated && matchIdentity(guide.account),
            organizationMatched:authenticated && matchIdentity(guide.organization),
            state:state ? state.state : 'site_changed',controlState}};
    }})()"#
    );
    let started = Instant::now();
    let observed = super::workflow::execute(
        json!({"id":uuid::Uuid::new_v4().to_string(),
        "action":"evaluate","script":script,"sessionName":binding["sessionName"],"sessionId":task_id}),
        transport_session,
    )?;
    let current = task_context(task_id)?;
    let evidence = observed
        .get("result")
        .filter(|value| value.is_object())
        .ok_or_else(|| fault("playbook_observation_invalid"))?;
    let (mut status, mut reason) = if current != binding {
        ("adaptation_required", "task_target_changed")
    } else if started.elapsed().as_millis() > u128::from(maximum_age) {
        ("adaptation_required", "observation_stale")
    } else if evidence["originAllowed"] != true {
        ("failed", "origin_mismatch")
    } else if evidence["authenticated"] == true && evidence["accountMatched"] != true {
        ("failed", "account_mismatch")
    } else if evidence["authenticated"] == true && evidence["organizationMatched"] != true {
        ("failed", "organization_mismatch")
    } else if evidence["authenticated"] == true {
        ("completed", "authenticated")
    } else {
        match evidence["state"].as_str() {
            Some("identifier_required") => ("waiting", "identifier_required"),
            Some("password_required") => ("waiting", "password_required"),
            Some("mfa_required") => ("waiting", "mfa_required"),
            Some("login_required") => ("waiting", "login_required"),
            Some("blocked") => ("failed", "site_blocked"),
            _ => ("adaptation_required", "site_changed"),
        }
    };
    let fingerprint = digest(&evidence["controlState"])?;
    let mut prepared = None;
    if operation == "prepare_login"
        && status == "waiting"
        && matches!(
            reason,
            "identifier_required" | "password_required" | "login_required"
        )
    {
        let required = if reason == "identifier_required"
            && guide
                .controls
                .as_ref()
                .is_some_and(|controls| controls.identifier_submit.is_some())
        {
            vec!["identifier", "identifierSubmit", "accountScope"]
        } else if reason == "identifier_required" {
            vec!["identifier", "password", "submit", "accountScope"]
        } else {
            vec!["password", "submit", "accountScope"]
        };
        let masked = evidence["controlState"].as_array().is_some_and(|controls| {
            controls.iter().any(|control| {
                control["name"] == "password"
                    && control["count"] == 1
                    && control["tag"] == "INPUT"
                    && control["type"] == "password"
            })
        });
        let password_required = required.contains(&"password");
        let ready = (!password_required || masked)
            && guide.controls.is_some()
            && evidence["controlState"].as_array().is_some_and(|controls| {
                required.iter().all(|name| {
                    controls
                        .iter()
                        .any(|control| control["name"] == *name && control["count"] == 1)
                })
            });
        if !ready {
            status = "adaptation_required";
            reason = "prepared_controls_missing";
        } else {
            let mut plan = PreparedLoginPlan {
                plan_id: String::new(),
                guide_id: guide.guide_id.clone(),
                guide_digest: guide_digest.clone(),
                binding_digest: binding_digest.clone(),
                control_fingerprint: fingerprint.clone(),
                expected_state: reason.into(),
                task_id: task_id.into(),
                profile_id: binding["profileId"].as_str().unwrap_or_default().into(),
                browser_id: binding["browserId"].as_str().unwrap_or_default().into(),
                tab_id: binding["tabId"].as_str().unwrap_or_default().into(),
                prepared_at_ms: now_ms()?,
                maximum_age_ms: maximum_age,
            };
            plan.plan_id = digest(&plan)?;
            prepared = Some(plan);
            status = "completed";
            reason = "plan_prepared";
        }
    }
    if operation == "validate_plan" {
        let mut plan: PreparedLoginPlan = serde_json::from_value(
            params
                .get("plan")
                .cloned()
                .ok_or_else(|| fault("playbook_plan_required"))?,
        )
        .map_err(|_| fault("playbook_plan_invalid"))?;
        let supplied = std::mem::take(&mut plan.plan_id);
        if supplied != digest(&plan)? || !(1..=60_000).contains(&plan.maximum_age_ms) {
            return Err(fault("playbook_plan_invalid"));
        }
        if status != "failed" {
            (status, reason) =
                if plan.guide_id != guide.guide_id || plan.guide_digest != guide_digest {
                    ("adaptation_required", "prepared_guide_changed")
                } else if plan.binding_digest != binding_digest
                    || plan.task_id != task_id
                    || current != binding
                    || binding["profileId"] != plan.profile_id
                    || binding["browserId"] != plan.browser_id
                    || binding["tabId"] != plan.tab_id
                {
                    ("adaptation_required", "task_target_changed")
                } else if plan.control_fingerprint != fingerprint {
                    ("adaptation_required", "prepared_controls_changed")
                } else if evidence["state"] != plan.expected_state {
                    ("adaptation_required", "prepared_state_changed")
                } else if now_ms()?
                    .checked_sub(plan.prepared_at_ms)
                    .is_none_or(|age| age > plan.maximum_age_ms)
                {
                    ("adaptation_required", "plan_stale")
                } else if started.elapsed().as_millis() > u128::from(maximum_age) {
                    ("adaptation_required", "observation_stale")
                } else {
                    ("completed", "plan_current")
                };
        }
    }
    let data = json!({"schemaVersion":"agent-browser.routine-result.v1","guideId":guide.guide_id,
        "routine":operation,"status":status,"reasonCode":reason,"effectState":"no_effect",
        "taskId":task_id,"profileId":binding["profileId"],"browserId":binding["browserId"],"tabId":binding["tabId"],
        "accountVerified":reason=="authenticated","organizationVerified":reason=="authenticated",
        "guideDigest":guide_digest,"plan":prepared,
        "observationElapsedMs":started.elapsed().as_millis()});
    Ok(super::tool_response_from_payload(
        "browser_command",
        transport_session,
        json!({}),
        data,
        false,
    ))
}

fn guide_names() -> Result<Vec<String>, JsonRpcError> {
    let Some(directory) = std::env::var_os("AGENT_BROWSER_SITEGUIDE_DIRECTORY") else {
        return Ok(vec![
            "bill-session-v1".into(),
            "qbo-session-v1".into(),
            "reddit-feed-v1".into(),
        ]);
    };
    let mut names = Vec::new();
    for entry in fs::read_dir(directory).map_err(|_| fault("siteguide_directory_unavailable"))? {
        let entry = entry.map_err(|_| fault("siteguide_directory_unavailable"))?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            if let Some(name) = path.file_stem().and_then(|name| name.to_str()) {
                load_guide(name)?;
                names.push(name.to_string());
            }
        }
        if names.len() > 64 {
            return Err(fault("siteguide_directory_limit_exceeded"));
        }
    }
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod teaching_tests;
