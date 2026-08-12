//! Canonical service-request normalization shared by HTTP and MCP ingress.
//!
//! Transports retain parsing, request identifiers, error envelopes, relay
//! selection, and queue I/O. This module owns the public top-level field
//! ledger, validation, merge precedence, trace projection, and route-hint
//! ordering. The HTTP-only top-level `args` compatibility overlay is
//! deliberately handled by the HTTP adapter and is not canonical here.

use serde_json::{json, Map, Value};
use std::fmt;

use crate::native::remote_view_handoff::apply_remote_view_handoff_route_hints;
use crate::native::service_access::apply_shared_profile_route_hints_for_service_request;
use crate::native::service_contracts::{DESKTOP_CAPTURE_HARD_MAX_BYTES, SERVICE_REQUEST_ACTIONS};
use crate::native::service_model::ServiceState;

const PROFILE_LEASE_POLICIES: &[&str] = &["reject", "wait"];
const REPAIR_POLICIES: &[&str] = &[
    "reject_only",
    "reuse_compatible",
    "open_if_missing",
    "replace_duplicates",
];
const BROWSER_BUILDS: &[&str] = &["stock_chrome", "stealthcdp_chromium", "cdp_free_headed"];
const BROWSER_HOSTS: &[&str] = &[
    "local_headless",
    "local_headed",
    "docker_headed",
    "remote_headed",
    "cloud_provider",
    "attached_existing",
];
const VIEW_STREAM_PROVIDERS: &[&str] = &[
    "cdp_screencast",
    "chrome_tab_webrtc",
    "virtual_display_webrtc",
    "novnc",
    "rdp_gateway",
    "external_url",
];
const CONTROL_INPUT_PROVIDERS: &[&str] = &[
    "cdp_input",
    "webrtc_input",
    "vnc_input",
    "manual_attached_desktop",
];
const DISPLAY_ISOLATIONS: &[&str] = &[
    "private_virtual_display",
    "shared_display",
    "ambient_display",
];
const PROFILE_CLASSES: &[&str] = &[
    "default",
    "managed_one_time",
    "durable_named",
    "operator_supplied",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RouteHintStage {
    RetainedHandoff,
    SharedProfile,
}

const ROUTE_HINT_ORDER: [RouteHintStage; 2] = [
    RouteHintStage::RetainedHandoff,
    RouteHintStage::SharedProfile,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldKind {
    String,
    PositiveInteger,
    Boolean,
    StringArray,
    Object,
    Enum(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
struct ServiceRequestFieldSpec {
    name: &'static str,
    kind: FieldKind,
    command: bool,
    trace: bool,
    routing: bool,
    structural: bool,
}

impl ServiceRequestFieldSpec {
    const fn field(
        name: &'static str,
        kind: FieldKind,
        command: bool,
        trace: bool,
        routing: bool,
    ) -> Self {
        Self {
            name,
            kind,
            command,
            trace,
            routing,
            structural: false,
        }
    }

    const fn structural(name: &'static str, kind: FieldKind, command: bool) -> Self {
        Self {
            name,
            kind,
            command,
            trace: false,
            routing: false,
            structural: true,
        }
    }
}

// This is the single Rust authority for canonical field recognition and
// top-level role projection. Keep it aligned with service-request.v1.schema.json.
const SERVICE_REQUEST_FIELDS: &[ServiceRequestFieldSpec] = &[
    ServiceRequestFieldSpec::structural("action", FieldKind::String, true),
    ServiceRequestFieldSpec::structural("params", FieldKind::Object, false),
    ServiceRequestFieldSpec::field(
        "jobTimeoutMs",
        FieldKind::PositiveInteger,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "profileLeasePolicy",
        FieldKind::Enum(PROFILE_LEASE_POLICIES),
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "profileLeaseWaitTimeoutMs",
        FieldKind::PositiveInteger,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "blockedByManualAction",
        FieldKind::Boolean,
        false,
        false,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "manualSeedingRequired",
        FieldKind::Boolean,
        false,
        false,
        false,
    ),
    ServiceRequestFieldSpec::field("allowManualAction", FieldKind::Boolean, false, false, false),
    ServiceRequestFieldSpec::field(
        "monitorRunDueSummary",
        FieldKind::Object,
        false,
        false,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "allowMonitorFreshnessRisk",
        FieldKind::Boolean,
        false,
        false,
        false,
    ),
    ServiceRequestFieldSpec::field("requiresCdpFree", FieldKind::Boolean, true, true, false),
    ServiceRequestFieldSpec::field(
        "cdpAttachmentAllowed",
        FieldKind::Boolean,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field("serviceTabHandle", FieldKind::Object, true, true, true),
    ServiceRequestFieldSpec::field("targetId", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("script", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("expression", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("returnByValue", FieldKind::Boolean, true, true, false),
    ServiceRequestFieldSpec::field("timeoutMs", FieldKind::PositiveInteger, true, true, false),
    ServiceRequestFieldSpec::field(
        "maxReturnBytes",
        FieldKind::PositiveInteger,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "maxTextBytes",
        FieldKind::PositiveInteger,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "maxBodyBytes",
        FieldKind::PositiveInteger,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field("includeScreenshot", FieldKind::Boolean, true, true, false),
    ServiceRequestFieldSpec::field("screenshotDir", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field(
        "maxConsoleEntries",
        FieldKind::PositiveInteger,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "maxErrorEntries",
        FieldKind::PositiveInteger,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "maxRequestEntries",
        FieldKind::PositiveInteger,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "captureEvidenceOnFailure",
        FieldKind::Boolean,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field("probe", FieldKind::Object, true, true, false),
    ServiceRequestFieldSpec::field("uiAction", FieldKind::Object, true, true, false),
    ServiceRequestFieldSpec::field("networkCapture", FieldKind::Object, true, true, false),
    ServiceRequestFieldSpec::field("fileTransfer", FieldKind::Object, true, true, false),
    ServiceRequestFieldSpec::field(
        "repairPolicy",
        FieldKind::Enum(REPAIR_POLICIES),
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "browserBuild",
        FieldKind::Enum(BROWSER_BUILDS),
        true,
        true,
        true,
    ),
    ServiceRequestFieldSpec::field(
        "browserHost",
        FieldKind::Enum(BROWSER_HOSTS),
        true,
        true,
        true,
    ),
    ServiceRequestFieldSpec::field(
        "viewStreamProvider",
        FieldKind::Enum(VIEW_STREAM_PROVIDERS),
        true,
        true,
        true,
    ),
    ServiceRequestFieldSpec::field(
        "controlInputProvider",
        FieldKind::Enum(CONTROL_INPUT_PROVIDERS),
        true,
        true,
        true,
    ),
    ServiceRequestFieldSpec::field(
        "displayIsolation",
        FieldKind::Enum(DISPLAY_ISOLATIONS),
        true,
        true,
        true,
    ),
    ServiceRequestFieldSpec::field("serviceName", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("agentName", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("taskName", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("targetServiceId", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("targetService", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("targetServiceIds", FieldKind::StringArray, true, true, true),
    ServiceRequestFieldSpec::field("targetServices", FieldKind::StringArray, true, true, true),
    ServiceRequestFieldSpec::field("siteId", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("siteIds", FieldKind::StringArray, true, true, true),
    ServiceRequestFieldSpec::field("loginId", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("loginIds", FieldKind::StringArray, true, true, true),
    ServiceRequestFieldSpec::field("accountId", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("accountIds", FieldKind::StringArray, true, true, true),
    ServiceRequestFieldSpec::field("url", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("desiredUrl", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("profile", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("profileId", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("runtimeProfile", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field(
        "profileClass",
        FieldKind::Enum(PROFILE_CLASSES),
        true,
        true,
        true,
    ),
    ServiceRequestFieldSpec::field("cdpUrl", FieldKind::String, true, false, false),
    ServiceRequestFieldSpec::field("cdpPort", FieldKind::PositiveInteger, true, false, false),
    ServiceRequestFieldSpec::field("browserId", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("sessionName", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field("format", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("maxBytes", FieldKind::PositiveInteger, true, true, false),
    ServiceRequestFieldSpec::field("locator", FieldKind::Object, true, true, false),
    ServiceRequestFieldSpec::field(
        "includeVisualization",
        FieldKind::Boolean,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field(
        "allowDuplicateProfileLane",
        FieldKind::Boolean,
        true,
        true,
        true,
    ),
    ServiceRequestFieldSpec::field("manualLoginLaunch", FieldKind::Boolean, true, false, false),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ServiceRequestIssueKind {
    InvalidRequest,
    MissingAction,
    UnsupportedAction,
    UnknownField,
    InvalidFieldType,
    InvalidFieldValue,
    BlockedManualAction,
    StaleMonitorEvidence,
    ForbiddenCdpExecution,
    InvalidServiceTabHandle,
    InvalidBoundedRecipe,
    RouteHintFailure,
    MissingAccountablePrincipal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceRequestIssue {
    pub kind: ServiceRequestIssueKind,
    message: String,
}

impl ServiceRequestIssue {
    fn new(kind: ServiceRequestIssueKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ServiceRequestIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

pub(crate) struct ServiceRequestNormalization<'a> {
    pub request: &'a Value,
    pub service_state: Option<&'a ServiceState>,
    pub fallback_principal: Option<ServiceRequestFallbackPrincipal<'a>>,
    pub request_id: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ServiceRequestPrincipalSource {
    ExplicitLabels,
    AuthenticatedDashboard,
    LocalProcess,
}

impl ServiceRequestPrincipalSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitLabels => "explicit_labels",
            Self::AuthenticatedDashboard => "authenticated_dashboard",
            Self::LocalProcess => "local_process",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ServiceRequestFallbackPrincipal<'a> {
    pub source: ServiceRequestPrincipalSource,
    pub principal: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceRequestAttribution {
    pub source: ServiceRequestPrincipalSource,
    pub principal: String,
    pub request_id: String,
}

#[derive(Debug)]
pub(crate) struct NormalizedServiceRequest {
    pub command: Value,
    pub trace: Value,
    pub attribution: ServiceRequestAttribution,
}

/// Normalize one schema-backed service request without transport or queue I/O.
pub(crate) fn normalize_service_request(
    input: ServiceRequestNormalization<'_>,
) -> Result<NormalizedServiceRequest, ServiceRequestIssue> {
    let request = input.request.as_object().ok_or_else(|| {
        ServiceRequestIssue::new(
            ServiceRequestIssueKind::InvalidRequest,
            "service request body must be a JSON object",
        )
    })?;
    let action = request
        .get("action")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ServiceRequestIssue::new(
                ServiceRequestIssueKind::MissingAction,
                "service request requires action",
            )
        })?;
    if !SERVICE_REQUEST_ACTIONS.contains(&action) {
        return Err(ServiceRequestIssue::new(
            ServiceRequestIssueKind::UnsupportedAction,
            format!("service request action '{action}' is not supported"),
        ));
    }

    validate_canonical_fields(request)?;
    validate_safety_gates(action, request)?;
    let attribution = derive_service_request_attribution(&input, request)?;

    let mut command = json!({ "action": action });
    if let Some(params) = request.get("params") {
        let params = params.as_object().ok_or_else(|| {
            ServiceRequestIssue::new(
                ServiceRequestIssueKind::InvalidFieldType,
                "params must be a JSON object",
            )
        })?;
        for (key, value) in params {
            if key != "id" && key != "action" {
                command[key] = value.clone();
            }
        }
    }

    let mut trace = json!({
        "serviceName": Value::Null,
        "agentName": Value::Null,
        "taskName": Value::Null,
    });
    for spec in SERVICE_REQUEST_FIELDS {
        if spec.structural {
            continue;
        }
        let Some(value) = request.get(spec.name) else {
            continue;
        };
        if spec.command {
            command[spec.name] = value.clone();
        }
        if spec.trace {
            trace[spec.name] = value.clone();
        }
    }

    if let Some(service_state) = input.service_state {
        for stage in ROUTE_HINT_ORDER {
            match stage {
                RouteHintStage::RetainedHandoff => {
                    apply_remote_view_handoff_route_hints(service_state, &mut command);
                }
                RouteHintStage::SharedProfile => {
                    apply_shared_profile_route_hints_for_service_request(
                        service_state,
                        &mut command,
                    )
                    .map_err(|message| {
                        ServiceRequestIssue::new(ServiceRequestIssueKind::RouteHintFailure, message)
                    })?;
                }
            }
        }
    }

    Ok(NormalizedServiceRequest {
        command,
        trace,
        attribution,
    })
}

/// Attach transport-proven request identity after canonical request projection.
///
/// These fields are internal command metadata rather than public schema inputs,
/// so callers cannot forge them through the service-request contract.
pub(crate) fn apply_service_request_attribution(
    command: &mut Value,
    attribution: &ServiceRequestAttribution,
) {
    command["callerId"] = json!(attribution.principal);
    command["requestId"] = json!(attribution.request_id);
    command["requestPrincipalSource"] = json!(attribution.source.as_str());
}

fn derive_service_request_attribution(
    input: &ServiceRequestNormalization<'_>,
    request: &Map<String, Value>,
) -> Result<ServiceRequestAttribution, ServiceRequestIssue> {
    if input.request_id.trim().is_empty() {
        return Err(ServiceRequestIssue::new(
            ServiceRequestIssueKind::MissingAccountablePrincipal,
            "effect-capable service request requires a nonempty request ID",
        ));
    }
    let explicit = ["serviceName", "agentName", "taskName"]
        .map(|field| request.get(field).and_then(Value::as_str).map(str::trim));
    let explicit_complete = explicit
        .iter()
        .all(|value| value.is_some_and(|value| !value.is_empty()));
    let (source, principal) = if explicit_complete {
        (
            ServiceRequestPrincipalSource::ExplicitLabels,
            format!(
                "service:{}/agent:{}/task:{}",
                explicit[0].unwrap(),
                explicit[1].unwrap(),
                explicit[2].unwrap()
            ),
        )
    } else if let Some(fallback) = input.fallback_principal.filter(|fallback| {
        !fallback.principal.trim().is_empty() && !input.request_id.trim().is_empty()
    }) {
        (fallback.source, fallback.principal.trim().to_string())
    } else {
        return Err(ServiceRequestIssue::new(
            ServiceRequestIssueKind::MissingAccountablePrincipal,
            "effect-capable service request requires serviceName, agentName, and taskName, or an authenticated/local principal with request ID",
        ));
    };
    Ok(ServiceRequestAttribution {
        source,
        principal,
        request_id: input.request_id.to_string(),
    })
}

fn validate_canonical_fields(request: &Map<String, Value>) -> Result<(), ServiceRequestIssue> {
    for (name, value) in request {
        let Some(spec) = SERVICE_REQUEST_FIELDS.iter().find(|spec| spec.name == name) else {
            return Err(ServiceRequestIssue::new(
                ServiceRequestIssueKind::UnknownField,
                format!("unknown service request field: {name}"),
            ));
        };
        if spec.name == "params" {
            continue;
        }
        validate_field(spec, value)?;
    }
    Ok(())
}

fn validate_field(
    spec: &ServiceRequestFieldSpec,
    value: &Value,
) -> Result<(), ServiceRequestIssue> {
    if value.is_null() {
        return Err(ServiceRequestIssue::new(
            ServiceRequestIssueKind::InvalidFieldType,
            format!("{} must not be null", spec.name),
        ));
    }
    let valid = match spec.kind {
        FieldKind::String => value.is_string(),
        FieldKind::PositiveInteger => value.as_u64().is_some_and(|value| value > 0),
        FieldKind::Boolean => value.is_boolean(),
        FieldKind::StringArray => value
            .as_array()
            .is_some_and(|values| values.iter().all(Value::is_string)),
        FieldKind::Object => value.is_object(),
        FieldKind::Enum(values) => value.as_str().is_some_and(|value| values.contains(&value)),
    };
    if valid {
        return Ok(());
    }

    let message = match spec.kind {
        FieldKind::String => format!("{} must be a string", spec.name),
        FieldKind::PositiveInteger => format!("{} must be a positive integer", spec.name),
        FieldKind::Boolean => format!("{} must be a boolean", spec.name),
        FieldKind::StringArray => format!("{} must be an array of strings", spec.name),
        FieldKind::Object => format!("{} must be a JSON object", spec.name),
        FieldKind::Enum(values) => format!("{} must be one of {}", spec.name, values.join(", ")),
    };
    let kind = if matches!(spec.kind, FieldKind::Enum(_)) {
        ServiceRequestIssueKind::InvalidFieldValue
    } else {
        ServiceRequestIssueKind::InvalidFieldType
    };
    Err(ServiceRequestIssue::new(kind, message))
}

fn validate_safety_gates(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    reject_blocked_manual_service_request(request)?;
    reject_cdp_free_service_request(action, request)?;
    reject_cdp_attach_service_request(action, request)?;
    reject_external_byop_adopt_request(action, request)?;
    reject_bounded_evaluate_service_request(action, request)?;
    reject_service_diagnostics_request(action, request)?;
    reject_desktop_capture_request(action, request)?;
    reject_desktop_locate_request(action, request)?;
    reject_service_probe_request(action, request)?;
    reject_tab_handle_refresh_request(action, request)?;
    reject_service_ui_action_request(action, request)?;
    reject_service_network_capture_request(action, request)?;
    reject_service_file_transfer_request(action, request)?;
    reject_stale_monitor_service_request(request)
}

fn issue(kind: ServiceRequestIssueKind, message: impl Into<String>) -> ServiceRequestIssue {
    ServiceRequestIssue::new(kind, message)
}

fn reject_blocked_manual_service_request(
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if request
        .get("blockedByManualAction")
        .and_then(Value::as_bool)
        == Some(true)
        && request
            .get("manualSeedingRequired")
            .and_then(Value::as_bool)
            == Some(true)
        && request.get("allowManualAction").and_then(Value::as_bool) != Some(true)
    {
        return Err(issue(
            ServiceRequestIssueKind::BlockedManualAction,
            "service request is blocked by manual profile seeding; complete seeding or set allowManualAction=true to override",
        ));
    }
    Ok(())
}

fn reject_stale_monitor_service_request(
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    let Some(summary) = request.get("monitorRunDueSummary") else {
        return Ok(());
    };
    if request
        .get("allowMonitorFreshnessRisk")
        .and_then(Value::as_bool)
        == Some(true)
    {
        return Ok(());
    }
    let summary = summary
        .as_object()
        .expect("canonical object validation ran first");
    let expired = summary_string_array(summary.get("expiredTargetServiceIds"));
    if !expired.is_empty() {
        return Err(issue(
            ServiceRequestIssueKind::StaleMonitorEvidence,
            format!(
                "service monitor run-due found expired profile freshness before service request: {}",
                expired.join(",")
            ),
        ));
    }
    let unverified = summary_string_array(summary.get("unverifiedTargetServiceIds"));
    if !unverified.is_empty() {
        return Err(issue(
            ServiceRequestIssueKind::StaleMonitorEvidence,
            format!(
                "service monitor run-due could not verify profile freshness before service request: {}",
                unverified.join(",")
            ),
        ));
    }
    let matched = summary.get("matched").and_then(Value::as_u64).unwrap_or(0);
    let failed = summary.get("failed").and_then(Value::as_bool) == Some(true);
    let recommended_action = summary
        .get("recommendedAction")
        .and_then(Value::as_str)
        .unwrap_or("inspect_monitor_results");
    if matched == 0 || (failed && recommended_action != "use_selected_profile") {
        return Err(issue(
            ServiceRequestIssueKind::StaleMonitorEvidence,
            format!(
                "service monitor run-due requires inspection before service request: {recommended_action}"
            ),
        ));
    }
    Ok(())
}

fn summary_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn reject_cdp_free_service_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "cdp_free_launch"
        && request.get("requiresCdpFree").and_then(Value::as_bool) == Some(true)
        && request.get("cdpAttachmentAllowed").and_then(Value::as_bool) != Some(true)
    {
        return Err(issue(
            ServiceRequestIssueKind::ForbiddenCdpExecution,
            "service request requires CDP-free browser operation; non-CDP service request execution is not implemented yet",
        ));
    }
    Ok(())
}

fn reject_cdp_attach_service_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "cdp_attach" {
        return Ok(());
    }
    if request.get("cdpAttachmentAllowed").and_then(Value::as_bool) != Some(true) {
        return Err(issue(
            ServiceRequestIssueKind::ForbiddenCdpExecution,
            "cdp_attach requires cdpAttachmentAllowed=true from the access-plan decision",
        ));
    }
    validate_service_tab_handle(request, action, true)
}

fn reject_external_byop_adopt_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "external_byop_adopt" {
        return Ok(());
    }
    if request
        .get("runtimeProfile")
        .or_else(|| request.get("profileId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "external_byop_adopt requires runtimeProfile or profileId",
        ));
    }
    let has_cdp_url = request
        .get("cdpUrl")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let has_cdp_port = request.get("cdpPort").and_then(Value::as_u64).is_some();
    if has_cdp_url == has_cdp_port {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "external_byop_adopt requires exactly one of cdpUrl or cdpPort",
        ));
    }
    Ok(())
}

fn reject_bounded_evaluate_service_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "evaluate" {
        return Ok(());
    }
    validate_service_tab_handle(request, action, true)?;
    if request
        .get("script")
        .or_else(|| request.get("expression"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "evaluate requires script or expression",
        ));
    }
    if request.get("returnByValue").and_then(Value::as_bool) == Some(false) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "evaluate requires returnByValue=true so results can be capped",
        ));
    }
    for field in ["timeoutMs", "maxReturnBytes"] {
        if request.get(field).and_then(Value::as_u64).is_none() {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                format!("evaluate requires positive {field}"),
            ));
        }
    }
    Ok(())
}

fn reject_service_diagnostics_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action == "diagnostics" {
        validate_service_tab_handle(request, action, false)?;
    }
    Ok(())
}

fn reject_desktop_capture_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "desktop_capture" {
        return Ok(());
    }
    let params = request.get("params").and_then(Value::as_object);
    const DESKTOP_CAPTURE_FIELDS: &[&str] = &["browserId", "sessionName", "format", "maxBytes"];
    if let Some(params) = params {
        if let Some(field) = params
            .keys()
            .find(|field| !DESKTOP_CAPTURE_FIELDS.contains(&field.as_str()))
        {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                format!("desktop_capture does not accept params.{field}"),
            ));
        }
        for field in DESKTOP_CAPTURE_FIELDS {
            if let (Some(top_level), Some(nested)) = (request.get(*field), params.get(*field)) {
                if top_level != nested {
                    return Err(issue(
                        ServiceRequestIssueKind::InvalidBoundedRecipe,
                        format!("desktop_capture has conflicting {field} values"),
                    ));
                }
            }
        }
    }
    let capture_field = |field: &str| {
        request
            .get(field)
            .or_else(|| params.and_then(|p| p.get(field)))
    };
    if capture_field("browserId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_capture requires browserId",
        ));
    }
    if let Some(format) = capture_field("format") {
        if format.as_str() != Some("png") {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "desktop_capture format must be png",
            ));
        }
    }
    if capture_field("sessionName").is_some_and(|value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    }) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_capture sessionName must be a nonempty string",
        ));
    }
    if let Some(max_bytes) = capture_field("maxBytes") {
        let Some(max_bytes) = max_bytes.as_u64().filter(|value| *value > 0) else {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "desktop_capture maxBytes must be a positive integer",
            ));
        };
        if max_bytes > DESKTOP_CAPTURE_HARD_MAX_BYTES {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                format!(
                    "desktop_capture maxBytes must not exceed {DESKTOP_CAPTURE_HARD_MAX_BYTES}"
                ),
            ));
        }
    }
    Ok(())
}

fn reject_desktop_locate_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "desktop_locate" {
        return Ok(());
    }
    const TOP_LEVEL_FIELDS: &[&str] = &[
        "action",
        "browserId",
        "sessionName",
        "locator",
        "includeVisualization",
        "jobTimeoutMs",
        "serviceName",
        "agentName",
        "taskName",
        "params",
    ];
    if let Some(field) = request
        .keys()
        .find(|field| !TOP_LEVEL_FIELDS.contains(&field.as_str()))
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            format!("desktop_locate does not accept {field}"),
        ));
    }
    let params = request.get("params").and_then(Value::as_object);
    const DESKTOP_LOCATE_FIELDS: &[&str] = &[
        "browserId",
        "sessionName",
        "locator",
        "includeVisualization",
    ];
    if let Some(params) = params {
        if let Some(field) = params
            .keys()
            .find(|field| !DESKTOP_LOCATE_FIELDS.contains(&field.as_str()))
        {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                format!("desktop_locate does not accept params.{field}"),
            ));
        }
        for field in DESKTOP_LOCATE_FIELDS {
            if let (Some(top_level), Some(nested)) = (request.get(*field), params.get(*field)) {
                if top_level != nested {
                    return Err(issue(
                        ServiceRequestIssueKind::InvalidBoundedRecipe,
                        format!("desktop_locate has conflicting {field} values"),
                    ));
                }
            }
        }
    }
    let locate_field = |field: &str| {
        request
            .get(field)
            .or_else(|| params.and_then(|values| values.get(field)))
    };
    if locate_field("browserId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_locate requires browserId",
        ));
    }
    if locate_field("sessionName").is_some_and(|value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    }) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_locate sessionName must be a nonempty string",
        ));
    }
    let locator = locate_field("locator")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "desktop_locate requires locator",
            )
        })?;
    const LOCATOR_FIELDS: &[&str] = &["locatorId", "maxCandidates"];
    if let Some(field) = locator
        .keys()
        .find(|field| !LOCATOR_FIELDS.contains(&field.as_str()))
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            format!("desktop_locate locator does not accept {field}"),
        ));
    }
    if locator
        .get("locatorId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_locate locator requires locatorId",
        ));
    }
    if let Some(max_candidates) = locator.get("maxCandidates") {
        if !max_candidates
            .as_u64()
            .is_some_and(|value| (1..=32).contains(&value))
        {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "desktop_locate locator.maxCandidates must be an integer between 1 and 32",
            ));
        }
    }
    if locate_field("includeVisualization").is_some_and(|value| !value.is_boolean()) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_locate includeVisualization must be a boolean",
        ));
    }
    Ok(())
}

fn validate_service_tab_handle(
    request: &Map<String, Value>,
    action: &str,
    require_target: bool,
) -> Result<(), ServiceRequestIssue> {
    let Some(handle) = request.get("serviceTabHandle").and_then(Value::as_object) else {
        return Err(issue(
            ServiceRequestIssueKind::InvalidServiceTabHandle,
            format!("{action} requires serviceTabHandle"),
        ));
    };
    if handle.get("valid").and_then(Value::as_bool) != Some(true) {
        let stale_reason = handle
            .get("staleReason")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return Err(issue(
            ServiceRequestIssueKind::InvalidServiceTabHandle,
            format!("service tab handle is stale: {stale_reason}"),
        ));
    }
    if handle.get("tabId").and_then(Value::as_str).is_none() {
        return Err(issue(
            ServiceRequestIssueKind::InvalidServiceTabHandle,
            "serviceTabHandle.tabId is required",
        ));
    }
    if require_target && handle.get("targetId").and_then(Value::as_str).is_none() {
        return Err(issue(
            ServiceRequestIssueKind::InvalidServiceTabHandle,
            format!("{action} requires serviceTabHandle.targetId"),
        ));
    }
    Ok(())
}

fn reject_service_probe_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "probe" {
        return Ok(());
    }
    validate_service_tab_handle(request, action, true)?;
    let recipe = request
        .get("probe")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "probe requires probe recipe object",
            )
        })?;
    if recipe
        .get("detectors")
        .and_then(Value::as_array)
        .filter(|detectors| !detectors.is_empty())
        .is_none()
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "probe requires at least one detector",
        ));
    }
    for field in ["timeoutMs", "maxReturnBytes"] {
        let value = request
            .get(field)
            .or_else(|| recipe.get(field))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if value == 0 {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                format!("probe requires positive {field}"),
            ));
        }
    }
    if let Some(record) = recipe.get("recordFreshness") {
        let record = record.as_object().ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "probe.recordFreshness must be an object",
            )
        })?;
        for field in ["targetServiceId", "accountId"] {
            if record
                .get(field)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                return Err(issue(
                    ServiceRequestIssueKind::InvalidBoundedRecipe,
                    format!("probe.recordFreshness requires {field}"),
                ));
            }
        }
    }
    Ok(())
}

fn reject_tab_handle_refresh_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "tab_handle_refresh" {
        return Ok(());
    }
    let Some(handle) = request.get("serviceTabHandle").and_then(Value::as_object) else {
        return Err(issue(
            ServiceRequestIssueKind::InvalidServiceTabHandle,
            "tab_handle_refresh requires serviceTabHandle",
        ));
    };
    if handle.get("tabId").and_then(Value::as_str).is_none() {
        return Err(issue(
            ServiceRequestIssueKind::InvalidServiceTabHandle,
            "serviceTabHandle.tabId is required",
        ));
    }
    Ok(())
}

fn reject_service_ui_action_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "ui_action" {
        return Ok(());
    }
    validate_service_tab_handle(request, action, true)?;
    let recipe = request
        .get("uiAction")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "ui_action requires uiAction object",
            )
        })?;
    let steps = recipe
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "ui_action requires uiAction.steps array",
            )
        })?;
    if steps.is_empty() {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "ui_action requires at least one step",
        ));
    }
    if request
        .get("timeoutMs")
        .or_else(|| recipe.get("timeoutMs"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "ui_action requires positive timeoutMs",
        ));
    }
    Ok(())
}

fn reject_service_network_capture_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "network_capture" {
        return Ok(());
    }
    validate_service_tab_handle(request, action, true)?;
    let recipe = request
        .get("networkCapture")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "network_capture requires networkCapture object",
            )
        })?;
    let timeout_ms = request
        .get("timeoutMs")
        .or_else(|| recipe.get("timeoutMs"))
        .or_else(|| recipe.get("maxDurationMs"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if timeout_ms == 0 {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "network_capture requires positive timeoutMs",
        ));
    }
    if recipe
        .get("maxEvents")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .is_none()
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "network_capture requires positive networkCapture.maxEvents",
        ));
    }
    if recipe.get("captureBodies").and_then(Value::as_bool) == Some(true)
        && recipe
            .get("maxBodyBytes")
            .or_else(|| request.get("maxBodyBytes"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "network_capture captureBodies requires positive maxBodyBytes",
        ));
    }
    Ok(())
}

fn reject_service_file_transfer_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "file_transfer" {
        return Ok(());
    }
    validate_service_tab_handle(request, action, true)?;
    let recipe = request
        .get("fileTransfer")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "file_transfer requires fileTransfer object",
            )
        })?;
    if request
        .get("timeoutMs")
        .or_else(|| recipe.get("timeoutMs"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "file_transfer requires positive timeoutMs",
        ));
    }
    if recipe.get("upload").is_none() && recipe.get("download").is_none() {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "file_transfer requires upload or download recipe",
        ));
    }
    if let Some(upload) = recipe.get("upload") {
        let upload = upload.as_object().ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "file_transfer upload must be an object",
            )
        })?;
        reject_file_transfer_upload_recipe(upload)?;
    }
    if let Some(download) = recipe.get("download") {
        let download = download.as_object().ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "file_transfer download must be an object",
            )
        })?;
        reject_file_transfer_download_recipe(download)?;
    }
    Ok(())
}

fn reject_file_transfer_upload_recipe(
    upload: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
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
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "file_transfer upload requires selector or labelText",
        ));
    }
    let files = upload
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "file_transfer upload requires files array",
            )
        })?;
    if files.is_empty()
        || !files
            .iter()
            .all(|file| file.as_str().is_some_and(|value| !value.trim().is_empty()))
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "file_transfer upload files must be nonempty strings",
        ));
    }
    let max_files = upload.get("maxFiles").and_then(Value::as_u64).unwrap_or(0);
    if max_files == 0 {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "file_transfer upload requires positive maxFiles",
        ));
    }
    if files.len() as u64 > max_files {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            format!(
                "file_transfer upload file count {} exceeds maxFiles {}",
                files.len(),
                max_files
            ),
        ));
    }
    reject_nonempty_string_array(
        upload.get("allowedPaths"),
        "file_transfer upload allowedPaths",
    )
}

fn reject_file_transfer_download_recipe(
    download: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if download
        .get("selector")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "file_transfer download requires selector",
        ));
    }
    if download
        .get("directory")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "file_transfer download requires directory",
        ));
    }
    reject_nonempty_string_array(
        download.get("allowedDirectories"),
        "file_transfer download allowedDirectories",
    )?;
    if download.get("maxBytes").and_then(Value::as_u64) == Some(0) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "file_transfer download maxBytes must be positive",
        ));
    }
    Ok(())
}

fn reject_nonempty_string_array(
    value: Option<&Value>,
    label: &str,
) -> Result<(), ServiceRequestIssue> {
    let valid = value
        .and_then(Value::as_array)
        .filter(|items| {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| item.as_str().is_some_and(|text| !text.trim().is_empty()))
        })
        .is_some();
    if valid {
        Ok(())
    } else {
        Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            format!("{label} must be a nonempty string array"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize(request: Value) -> Result<NormalizedServiceRequest, ServiceRequestIssue> {
        normalize_service_request(ServiceRequestNormalization {
            request: &request,
            service_state: None,
            fallback_principal: Some(ServiceRequestFallbackPrincipal {
                source: ServiceRequestPrincipalSource::LocalProcess,
                principal: "local:test-normalizer",
            }),
            request_id: "test-normalizer-request",
        })
    }

    #[test]
    fn effectful_request_without_labels_or_fallback_principal_fails_closed() {
        let request = json!({"action": "navigate", "params": {"url": "https://example.com"}});
        let error = normalize_service_request(ServiceRequestNormalization {
            request: &request,
            service_state: None,
            fallback_principal: None,
            request_id: "http-service-request-navigate-test",
        })
        .unwrap_err();

        assert_eq!(
            error.kind,
            ServiceRequestIssueKind::MissingAccountablePrincipal
        );
        assert_eq!(
            error.message(),
            "effect-capable service request requires serviceName, agentName, and taskName, or an authenticated/local principal with request ID"
        );
    }

    #[test]
    fn complete_explicit_labels_are_the_preferred_accountable_principal() {
        let request = json!({
            "action": "navigate",
            "serviceName": "JournalDownloader",
            "agentName": "codex",
            "taskName": "probeACSwebsite"
        });
        let mut normalized = normalize_service_request(ServiceRequestNormalization {
            request: &request,
            service_state: None,
            fallback_principal: Some(ServiceRequestFallbackPrincipal {
                source: ServiceRequestPrincipalSource::AuthenticatedDashboard,
                principal: "dashboard-admin",
            }),
            request_id: "request-42",
        })
        .unwrap();

        assert_eq!(
            normalized.attribution,
            ServiceRequestAttribution {
                source: ServiceRequestPrincipalSource::ExplicitLabels,
                principal: "service:JournalDownloader/agent:codex/task:probeACSwebsite".to_string(),
                request_id: "request-42".to_string(),
            }
        );
        apply_service_request_attribution(&mut normalized.command, &normalized.attribution);
        assert_eq!(
            normalized.command["callerId"],
            normalized.attribution.principal
        );
        assert_eq!(normalized.command["requestId"], "request-42");
        assert_eq!(
            normalized.command["requestPrincipalSource"],
            "explicit_labels"
        );
    }

    fn sorted_names(values: impl IntoIterator<Item = String>) -> Vec<String> {
        let mut values = values.into_iter().collect::<Vec<_>>();
        values.sort();
        values
    }

    fn ledger_role_names(role: &str) -> Vec<String> {
        let contract: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-request-field-roles.v1.json"
        ))
        .unwrap();
        sorted_names(
            contract["roles"][role]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap().to_string()),
        )
    }

    fn spec_role_names(predicate: impl Fn(&ServiceRequestFieldSpec) -> bool) -> Vec<String> {
        sorted_names(
            SERVICE_REQUEST_FIELDS
                .iter()
                .filter(|spec| predicate(spec))
                .map(|spec| spec.name.to_string()),
        )
    }

    #[test]
    fn canonical_field_ledger_matches_schema_constraints_and_every_role_set() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-request.v1.schema.json"
        ))
        .unwrap();
        let role_contract: Value = serde_json::from_str(include_str!(
            "../../../docs/dev/contracts/service-request-field-roles.v1.json"
        ))
        .unwrap();
        let properties = schema["properties"].as_object().unwrap();
        let canonical_names = sorted_names(properties.keys().cloned());
        let spec_names = spec_role_names(|_| true);

        assert_eq!(canonical_names.len(), 64);
        assert_eq!(canonical_names, spec_names);
        assert_eq!(role_contract["canonicalPropertyCount"], 64);
        assert_eq!(role_contract["transportLegacy"], json!(["args"]));
        assert!(!properties.contains_key("args"));

        assert_eq!(
            spec_role_names(|spec| spec.structural),
            ledger_role_names("structural")
        );
        assert_eq!(
            spec_role_names(|spec| spec.command),
            ledger_role_names("command")
        );
        assert_eq!(
            spec_role_names(|spec| spec.trace),
            ledger_role_names("trace")
        );
        assert_eq!(
            spec_role_names(|spec| spec.routing),
            ledger_role_names("routing")
        );
        assert_eq!(
            spec_role_names(|spec| !spec.structural && !spec.command && !spec.trace),
            ledger_role_names("validationOnly")
        );

        assert!(SERVICE_REQUEST_FIELDS
            .iter()
            .filter(|spec| spec.routing)
            .all(|spec| spec.command));
        assert!(SERVICE_REQUEST_FIELDS
            .iter()
            .filter(|spec| spec.trace)
            .all(|spec| spec.command));
        assert!(SERVICE_REQUEST_FIELDS
            .iter()
            .all(|spec| !(spec.trace && !spec.command) && !(spec.routing && !spec.command)));
        let routing = ledger_role_names("routing");
        for field in crate::native::service_access::SERVICE_REQUEST_ACCESS_PLAN_ROUTING_FIELDS {
            assert!(
                routing.iter().any(|candidate| candidate == field),
                "{field}"
            );
        }
        for (field, _) in crate::native::stream::SERVICE_REQUEST_HTTP_RELAY_CANONICAL_POINTERS {
            assert!(
                routing.iter().any(|candidate| candidate == field),
                "{field}"
            );
        }

        for spec in SERVICE_REQUEST_FIELDS {
            let property = &properties[spec.name];
            match spec.kind {
                FieldKind::String => assert_eq!(property["type"], "string", "{}", spec.name),
                FieldKind::PositiveInteger => {
                    assert_eq!(property["type"], "integer", "{}", spec.name);
                    assert_eq!(property["minimum"], 1, "{}", spec.name);
                }
                FieldKind::Boolean => assert_eq!(property["type"], "boolean", "{}", spec.name),
                FieldKind::StringArray => {
                    assert_eq!(property["type"], "array", "{}", spec.name);
                    assert_eq!(property["items"]["type"], "string", "{}", spec.name);
                }
                FieldKind::Object => assert!(
                    property["type"] == "object" || property["$ref"].is_string(),
                    "{}",
                    spec.name
                ),
                FieldKind::Enum(values) => {
                    assert_eq!(property["type"], "string", "{}", spec.name);
                    assert_eq!(property["enum"], json!(values), "{}", spec.name);
                }
            }
        }
        assert_eq!(properties["action"]["enum"], json!(SERVICE_REQUEST_ACTIONS));
    }

    #[test]
    fn params_are_flattened_with_reserved_fields_protected_and_top_level_precedence() {
        let normalized = normalize(json!({
            "action": "navigate",
            "params": {
                "id": "caller-id",
                "action": "screenshot",
                "url": "https://params.example",
                "args": ["--from-params"]
            },
            "url": "https://top.example"
        }))
        .unwrap();
        assert!(normalized.command.get("id").is_none());
        assert_eq!(normalized.command["action"], "navigate");
        assert_eq!(normalized.command["url"], "https://top.example");
        assert_eq!(normalized.command["args"], json!(["--from-params"]));
    }

    #[test]
    fn validation_only_safety_fields_do_not_leak_into_command_or_trace() {
        let normalized = normalize(json!({
            "action": "navigate",
            "blockedByManualAction": true,
            "manualSeedingRequired": true,
            "allowManualAction": true,
            "monitorRunDueSummary": {"matched": 1},
            "allowMonitorFreshnessRisk": true
        }))
        .unwrap();
        for field in [
            "blockedByManualAction",
            "manualSeedingRequired",
            "allowManualAction",
            "monitorRunDueSummary",
            "allowMonitorFreshnessRisk",
        ] {
            assert!(
                normalized.command.get(field).is_none(),
                "command leaked {field}"
            );
            assert!(
                normalized.trace.get(field).is_none(),
                "trace leaked {field}"
            );
        }
    }

    #[test]
    fn account_and_routing_fields_are_preserved_in_command_and_trace() {
        let normalized = normalize(json!({
            "action": "navigate",
            "accountId": "acct",
            "accountIds": ["acct", "backup"],
            "browserHost": "remote_headed",
            "viewStreamProvider": "rdp_gateway",
            "controlInputProvider": "manual_attached_desktop"
        }))
        .unwrap();
        for field in [
            "accountId",
            "accountIds",
            "browserHost",
            "viewStreamProvider",
            "controlInputProvider",
        ] {
            assert_eq!(normalized.command[field], normalized.trace[field]);
        }
    }

    #[test]
    fn canonical_types_nulls_enums_and_unknown_fields_are_rejected() {
        let fixtures = [
            (
                json!({"action": "navigate", "jobTimeoutMs": "10"}),
                ServiceRequestIssueKind::InvalidFieldType,
                "jobTimeoutMs must be a positive integer",
            ),
            (
                json!({"action": "navigate", "serviceName": 3}),
                ServiceRequestIssueKind::InvalidFieldType,
                "serviceName must be a string",
            ),
            (
                json!({"action": "navigate", "allowDuplicateProfileLane": "true"}),
                ServiceRequestIssueKind::InvalidFieldType,
                "allowDuplicateProfileLane must be a boolean",
            ),
            (
                json!({"action": "navigate", "accountIds": ["ok", 3]}),
                ServiceRequestIssueKind::InvalidFieldType,
                "accountIds must be an array of strings",
            ),
            (
                json!({"action": "navigate", "params": []}),
                ServiceRequestIssueKind::InvalidFieldType,
                "params must be a JSON object",
            ),
            (
                json!({"action": "navigate", "browserHost": "moon"}),
                ServiceRequestIssueKind::InvalidFieldValue,
                "browserHost must be one of local_headless, local_headed, docker_headed, remote_headed, cloud_provider, attached_existing",
            ),
            (
                json!({"action": "navigate", "serviceName": null}),
                ServiceRequestIssueKind::InvalidFieldType,
                "serviceName must not be null",
            ),
            (
                json!({"action": "navigate", "surprise": true}),
                ServiceRequestIssueKind::UnknownField,
                "unknown service request field: surprise",
            ),
        ];
        for (request, expected_kind, expected_message) in fixtures {
            let error = normalize(request).unwrap_err();
            assert_eq!(error.kind, expected_kind);
            assert_eq!(error.message(), expected_message);
        }

        let invalid_body = normalize(Value::Array(vec![])).unwrap_err();
        assert_eq!(invalid_body.kind, ServiceRequestIssueKind::InvalidRequest);
        assert_eq!(
            invalid_body.message(),
            "service request body must be a JSON object"
        );
        let missing_action = normalize(json!({})).unwrap_err();
        assert_eq!(missing_action.kind, ServiceRequestIssueKind::MissingAction);
        assert_eq!(missing_action.message(), "service request requires action");
        let unsupported = normalize(json!({"action": "surprise"})).unwrap_err();
        assert_eq!(unsupported.kind, ServiceRequestIssueKind::UnsupportedAction);
        assert_eq!(
            unsupported.message(),
            "service request action 'surprise' is not supported"
        );
    }

    #[test]
    fn external_byop_adopt_has_one_endpoint_rule() {
        for request in [
            json!({"action": "external_byop_adopt", "profileId": "p", "cdpUrl": "http://127.0.0.1:9222"}),
            json!({"action": "external_byop_adopt", "runtimeProfile": "p", "cdpPort": 9222}),
        ] {
            assert!(normalize(request).is_ok());
        }
        assert!(normalize(json!({
            "action": "external_byop_adopt",
            "profileId": "p",
            "cdpUrl": "http://127.0.0.1:9222",
            "cdpPort": 9222
        }))
        .is_err());
    }

    #[test]
    fn desktop_capture_rejects_unbound_or_unsafe_request_fields() {
        let fixtures = [
            (
                json!({"action": "desktop_capture", "format": "png"}),
                "desktop_capture requires browserId",
            ),
            (
                json!({"action": "desktop_capture", "browserId": "browser-1", "format": "jpeg"}),
                "desktop_capture format must be png",
            ),
            (
                json!({"action": "desktop_capture", "browserId": "browser-1", "maxBytes": DESKTOP_CAPTURE_HARD_MAX_BYTES + 1}),
                "desktop_capture maxBytes must not exceed 16777216",
            ),
            (
                json!({"action": "desktop_capture", "browserId": "browser-1", "params": {"displayName": ":99"}}),
                "desktop_capture does not accept params.displayName",
            ),
            (
                json!({"action": "desktop_capture", "browserId": "browser-1", "maxBytes": 1024, "params": {"maxBytes": 2048}}),
                "desktop_capture has conflicting maxBytes values",
            ),
        ];

        for (request, expected_message) in fixtures {
            let error = normalize(request).unwrap_err();
            assert_eq!(error.message(), expected_message);
        }

        assert!(normalize(json!({
            "action": "desktop_capture",
            "params": {
                "browserId": "browser-1",
                "sessionName": "default",
                "format": "png",
                "maxBytes": 1024
            }
        }))
        .is_ok());
    }

    #[test]
    fn desktop_locate_accepts_only_named_bounded_locator_requests() {
        let normalized = normalize(json!({
            "action": "desktop_locate",
            "browserId": "browser-1",
            "sessionName": "rdp-1",
            "locator": {
                "locatorId": "p110-control-v1",
                "maxCandidates": 8
            },
            "includeVisualization": true
        }))
        .unwrap();
        assert_eq!(normalized.command["action"], "desktop_locate");
        assert_eq!(
            normalized.command["locator"]["locatorId"],
            "p110-control-v1"
        );

        let fixtures = [
            (
                json!({"action": "desktop_locate", "locator": {"locatorId": "p110-control-v1"}}),
                "desktop_locate requires browserId",
            ),
            (
                json!({"action": "desktop_locate", "browserId": "browser-1"}),
                "desktop_locate requires locator",
            ),
            (
                json!({"action": "desktop_locate", "browserId": "browser-1", "locator": {"locatorId": ""}}),
                "desktop_locate locator requires locatorId",
            ),
            (
                json!({"action": "desktop_locate", "browserId": "browser-1", "locator": {"locatorId": "p110-control-v1", "maxCandidates": 33}}),
                "desktop_locate locator.maxCandidates must be an integer between 1 and 32",
            ),
            (
                json!({"action": "desktop_locate", "browserId": "browser-1", "locator": {"locatorId": "p110-control-v1", "threshold": 900}}),
                "desktop_locate locator does not accept threshold",
            ),
            (
                json!({"action": "desktop_locate", "browserId": "browser-1", "locator": {"locatorId": "p110-control-v1"}, "params": {"imageBase64": "pixels"}}),
                "desktop_locate does not accept params.imageBase64",
            ),
            (
                json!({"action": "desktop_locate", "browserId": "browser-1", "locator": {"locatorId": "p110-control-v1"}, "cdpUrl": "http://caller.invalid"}),
                "desktop_locate does not accept cdpUrl",
            ),
            (
                json!({"action": "desktop_locate", "browserId": "browser-1", "locator": {"locatorId": "p110-control-v1"}, "uiAction": {"type": "click"}}),
                "desktop_locate does not accept uiAction",
            ),
        ];
        for (request, expected_message) in fixtures {
            let error = normalize(request).unwrap_err();
            assert_eq!(error.message(), expected_message);
        }
    }

    fn test_tab_handle(valid: bool) -> Value {
        json!({
            "browserId": "session:default",
            "sessionName": "default",
            "tabId": "target:target-1",
            "targetId": "target-1",
            "valid": valid,
            "staleReason": if valid { Value::Null } else { json!("tab_closed") }
        })
    }

    fn valid_request_for_action(action: &str) -> Value {
        let mut request = json!({"action": action});
        match action {
            "external_byop_adopt" => {
                request["runtimeProfile"] = json!("external-work");
                request["cdpPort"] = json!(9222);
            }
            "cdp_attach" => {
                request["cdpAttachmentAllowed"] = json!(true);
                request["serviceTabHandle"] = test_tab_handle(true);
            }
            "evaluate" => {
                request["serviceTabHandle"] = test_tab_handle(true);
                request["script"] = json!("document.title");
                request["timeoutMs"] = json!(1000);
                request["maxReturnBytes"] = json!(128);
            }
            "diagnostics" => {
                request["serviceTabHandle"] = test_tab_handle(true);
            }
            "desktop_capture" => {
                request["browserId"] = json!("browser:desktop-fixture");
                request["format"] = json!("png");
                request["maxBytes"] = json!(4 * 1024 * 1024);
            }
            "probe" => {
                request["serviceTabHandle"] = test_tab_handle(true);
                request["probe"] = json!({"detectors": [{"type": "url_title"}]});
                request["timeoutMs"] = json!(1000);
                request["maxReturnBytes"] = json!(128);
            }
            "tab_handle_refresh" => {
                request["serviceTabHandle"] = test_tab_handle(true);
                request["repairPolicy"] = json!("reject_only");
            }
            "ui_action" => {
                request["serviceTabHandle"] = test_tab_handle(true);
                request["uiAction"] = json!({"steps": [{"type": "find"}]});
                request["timeoutMs"] = json!(1000);
            }
            "network_capture" => {
                request["serviceTabHandle"] = test_tab_handle(true);
                request["networkCapture"] = json!({"maxEvents": 1});
                request["timeoutMs"] = json!(1000);
            }
            "file_transfer" => {
                request["serviceTabHandle"] = test_tab_handle(true);
                request["fileTransfer"] = json!({
                    "upload": {
                        "selector": "#file",
                        "files": ["/tmp/report.txt"],
                        "allowedPaths": ["/tmp"],
                        "maxFiles": 1
                    }
                });
                request["timeoutMs"] = json!(1000);
            }
            _ => {}
        }
        request
    }

    #[test]
    fn every_supported_action_has_one_valid_normalizer_fixture() {
        for action in SERVICE_REQUEST_ACTIONS {
            let normalized = normalize(valid_request_for_action(action))
                .unwrap_or_else(|error| panic!("{action}: {error}"));
            assert_eq!(normalized.command["action"], *action);
        }
    }

    #[test]
    fn action_rejections_have_exact_issue_kind_and_message() {
        let handle = test_tab_handle(true);
        let fixtures = vec![
            (
                json!({"action":"tab_new","blockedByManualAction":true,"manualSeedingRequired":true}),
                ServiceRequestIssueKind::BlockedManualAction,
                "service request is blocked by manual profile seeding; complete seeding or set allowManualAction=true to override",
            ),
            (
                json!({"action":"tab_new","requiresCdpFree":true,"cdpAttachmentAllowed":false}),
                ServiceRequestIssueKind::ForbiddenCdpExecution,
                "service request requires CDP-free browser operation; non-CDP service request execution is not implemented yet",
            ),
            (
                json!({"action":"cdp_attach","cdpAttachmentAllowed":false,"serviceTabHandle":handle.clone()}),
                ServiceRequestIssueKind::ForbiddenCdpExecution,
                "cdp_attach requires cdpAttachmentAllowed=true from the access-plan decision",
            ),
            (
                json!({"action":"cdp_attach","cdpAttachmentAllowed":true,"serviceTabHandle":test_tab_handle(false)}),
                ServiceRequestIssueKind::InvalidServiceTabHandle,
                "service tab handle is stale: tab_closed",
            ),
            (
                json!({"action":"external_byop_adopt","cdpPort":9222}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "external_byop_adopt requires runtimeProfile or profileId",
            ),
            (
                json!({"action":"external_byop_adopt","profileId":"p","cdpUrl":"http://127.0.0.1:9222","cdpPort":9222}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "external_byop_adopt requires exactly one of cdpUrl or cdpPort",
            ),
            (
                json!({"action":"evaluate","script":"document.title","timeoutMs":1000,"maxReturnBytes":128}),
                ServiceRequestIssueKind::InvalidServiceTabHandle,
                "evaluate requires serviceTabHandle",
            ),
            (
                json!({"action":"evaluate","serviceTabHandle":handle.clone(),"timeoutMs":1000,"maxReturnBytes":128}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "evaluate requires script or expression",
            ),
            (
                json!({"action":"evaluate","serviceTabHandle":handle.clone(),"script":"x","returnByValue":false,"timeoutMs":1000,"maxReturnBytes":128}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "evaluate requires returnByValue=true so results can be capped",
            ),
            (
                json!({"action":"diagnostics"}),
                ServiceRequestIssueKind::InvalidServiceTabHandle,
                "diagnostics requires serviceTabHandle",
            ),
            (
                json!({"action":"probe","serviceTabHandle":handle.clone(),"probe":{"detectors":[]},"timeoutMs":1000,"maxReturnBytes":128}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "probe requires at least one detector",
            ),
            (
                json!({"action":"probe","serviceTabHandle":handle.clone(),"probe":{"detectors":[{}]},"timeoutMs":1000}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "probe requires positive maxReturnBytes",
            ),
            (
                json!({"action":"tab_handle_refresh"}),
                ServiceRequestIssueKind::InvalidServiceTabHandle,
                "tab_handle_refresh requires serviceTabHandle",
            ),
            (
                json!({"action":"ui_action","serviceTabHandle":handle.clone(),"uiAction":{"steps":[]},"timeoutMs":1000}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "ui_action requires at least one step",
            ),
            (
                json!({"action":"ui_action","serviceTabHandle":handle.clone(),"uiAction":{"steps":[{}]}}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "ui_action requires positive timeoutMs",
            ),
            (
                json!({"action":"network_capture","serviceTabHandle":handle.clone(),"networkCapture":{"maxEvents":1}}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "network_capture requires positive timeoutMs",
            ),
            (
                json!({"action":"network_capture","serviceTabHandle":handle.clone(),"networkCapture":{},"timeoutMs":1000}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "network_capture requires positive networkCapture.maxEvents",
            ),
            (
                json!({"action":"network_capture","serviceTabHandle":handle.clone(),"networkCapture":{"maxEvents":1,"captureBodies":true},"timeoutMs":1000}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "network_capture captureBodies requires positive maxBodyBytes",
            ),
            (
                json!({"action":"file_transfer","serviceTabHandle":handle.clone(),"fileTransfer":{"upload":{}},"timeoutMs":1000}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "file_transfer upload requires selector or labelText",
            ),
            (
                json!({"action":"file_transfer","serviceTabHandle":handle.clone(),"fileTransfer":{"upload":{"selector":"#file","files":["/tmp/a"],"maxFiles":1}},"timeoutMs":1000}),
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "file_transfer upload allowedPaths must be a nonempty string array",
            ),
            (
                json!({"action":"tab_new","monitorRunDueSummary":{"matched":1,"expiredTargetServiceIds":["acs"]}}),
                ServiceRequestIssueKind::StaleMonitorEvidence,
                "service monitor run-due found expired profile freshness before service request: acs",
            ),
            (
                json!({"action":"tab_new","monitorRunDueSummary":{"matched":1,"unverifiedTargetServiceIds":["acs"]}}),
                ServiceRequestIssueKind::StaleMonitorEvidence,
                "service monitor run-due could not verify profile freshness before service request: acs",
            ),
            (
                json!({"action":"tab_new","monitorRunDueSummary":{"matched":0,"recommendedAction":"inspect_monitor_results"}}),
                ServiceRequestIssueKind::StaleMonitorEvidence,
                "service monitor run-due requires inspection before service request: inspect_monitor_results",
            ),
        ];
        for (request, expected_kind, expected_message) in fixtures {
            let error = normalize(request).unwrap_err();
            assert_eq!(error.kind, expected_kind);
            assert_eq!(error.message(), expected_message);
        }
    }

    #[test]
    fn common_safety_gate_order_is_stable() {
        let canonical_before_safety = normalize(json!({
            "action": "tab_new",
            "jobTimeoutMs": "1000",
            "blockedByManualAction": true,
            "manualSeedingRequired": true
        }))
        .unwrap_err();
        assert_eq!(
            canonical_before_safety.kind,
            ServiceRequestIssueKind::InvalidFieldType
        );

        let manual_before_cdp = normalize(json!({
            "action": "tab_new",
            "blockedByManualAction": true,
            "manualSeedingRequired": true,
            "requiresCdpFree": true,
            "cdpAttachmentAllowed": false
        }))
        .unwrap_err();
        assert_eq!(
            manual_before_cdp.kind,
            ServiceRequestIssueKind::BlockedManualAction
        );

        let manual_before_structural_params = normalize(json!({
            "action": "tab_new",
            "params": [],
            "blockedByManualAction": true,
            "manualSeedingRequired": true
        }))
        .unwrap_err();
        assert_eq!(
            manual_before_structural_params.kind,
            ServiceRequestIssueKind::BlockedManualAction
        );

        let cdp_before_action = normalize(json!({
            "action": "cdp_attach",
            "requiresCdpFree": true,
            "cdpAttachmentAllowed": false,
            "serviceTabHandle": test_tab_handle(false),
            "monitorRunDueSummary": {"matched": 0}
        }))
        .unwrap_err();
        assert_eq!(
            cdp_before_action.kind,
            ServiceRequestIssueKind::ForbiddenCdpExecution
        );
        assert_eq!(
            cdp_before_action.message(),
            "service request requires CDP-free browser operation; non-CDP service request execution is not implemented yet"
        );
    }

    fn sample_value(spec: &ServiceRequestFieldSpec) -> Value {
        match spec.kind {
            FieldKind::String => json!(format!("{}-value", spec.name)),
            FieldKind::PositiveInteger => json!(1),
            FieldKind::Boolean => json!(false),
            FieldKind::StringArray => json!([format!("{}-value", spec.name)]),
            FieldKind::Object => json!({}),
            FieldKind::Enum(values) => json!(values[0]),
        }
    }

    #[test]
    fn command_and_trace_projection_exactly_match_the_role_ledger() {
        let mut request = Map::new();
        for spec in SERVICE_REQUEST_FIELDS {
            if spec.name == "params" {
                continue;
            }
            request.insert(spec.name.to_string(), sample_value(spec));
        }
        request.insert("action".to_string(), json!("navigate"));
        request.insert("monitorRunDueSummary".to_string(), json!({"matched": 1}));

        let normalized = normalize(Value::Object(request)).unwrap();
        assert_eq!(
            sorted_names(normalized.command.as_object().unwrap().keys().cloned()),
            ledger_role_names("command")
        );
        assert_eq!(
            sorted_names(normalized.trace.as_object().unwrap().keys().cloned()),
            ledger_role_names("trace")
        );
    }

    fn shared_profile_state() -> ServiceState {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            BrowserHealth, BrowserHost, BrowserProcess, BrowserProfile, ControlInputProvider,
            ViewStream, ViewStreamProvider,
        };

        ServiceState {
            profiles: BTreeMap::from([(
                "shared-social".to_string(),
                BrowserProfile {
                    id: "shared-social".to_string(),
                    name: "Shared social".to_string(),
                    target_service_ids: vec!["x".to_string()],
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-social".to_string(),
                BrowserProcess {
                    id: "browser-social".to_string(),
                    profile_id: Some("shared-social".to_string()),
                    host: BrowserHost::RemoteHeaded,
                    health: BrowserHealth::Ready,
                    display_isolation: Some("private_virtual_display".to_string()),
                    view_streams: vec![ViewStream {
                        provider: ViewStreamProvider::RdpGateway,
                        control_input: Some(ControlInputProvider::ManualAttachedDesktop),
                        ..ViewStream::default()
                    }],
                    active_session_ids: vec!["operator-social".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            ..ServiceState::default()
        }
    }

    fn normalize_with_state(request: Value, state: &ServiceState) -> NormalizedServiceRequest {
        normalize_service_request(ServiceRequestNormalization {
            request: &request,
            service_state: Some(state),
            fallback_principal: Some(ServiceRequestFallbackPrincipal {
                source: ServiceRequestPrincipalSource::LocalProcess,
                principal: "local:test-normalizer",
            }),
            request_id: "test-normalizer-request",
        })
        .unwrap()
    }

    #[test]
    fn retained_handoff_and_shared_profile_route_order_and_short_circuits_are_owned_here() {
        use std::collections::BTreeMap;

        use crate::native::service_model::RemoteViewHandoff;

        assert_eq!(
            ROUTE_HINT_ORDER,
            [
                RouteHintStage::RetainedHandoff,
                RouteHintStage::SharedProfile
            ]
        );

        let mut state = shared_profile_state();
        state.remote_view_handoffs = BTreeMap::from([(
            "handoff-a".to_string(),
            RemoteViewHandoff {
                id: "handoff-a".to_string(),
                browser_id: Some("session:original-lane".to_string()),
                session_name: Some("original-lane".to_string()),
                ..RemoteViewHandoff::default()
            },
        )]);
        let handoff = normalize_with_state(
            json!({
                "action": "service_remote_view_handoff_resolve",
                "params": {"handoffId": "handoff-a"}
            }),
            &state,
        );
        assert_eq!(handoff.command["browserId"], "session:original-lane");
        assert_eq!(handoff.command["sessionName"], "original-lane");

        let base = json!({
            "action": "tab_new",
            "runtimeProfile": "shared-social",
            "siteId": "x",
            "browserHost": "remote_headed",
            "viewStreamProvider": "rdp_gateway",
            "controlInputProvider": "manual_attached_desktop",
            "displayIsolation": "private_virtual_display",
            "serviceName": "SocialService",
            "agentName": "agent-a",
            "taskName": "openSocial"
        });
        let selected = normalize_with_state(base.clone(), &state);
        assert_eq!(selected.command["browserId"], "browser-social");
        assert_eq!(selected.command["sessionName"], "operator-social");

        for overlay in [
            json!({"browserId": "session:explicit"}),
            json!({"sessionName": "explicit"}),
            json!({"allowDuplicateProfileLane": true}),
        ] {
            let mut request = base.clone();
            request
                .as_object_mut()
                .unwrap()
                .extend(overlay.as_object().unwrap().clone());
            let normalized = normalize_with_state(request, &state);
            assert_ne!(normalized.command["browserId"], "browser-social");
            assert_ne!(normalized.command["sessionName"], "operator-social");
        }
    }

    fn without_transport_id(mut command: Value) -> Value {
        for field in ["id", "callerId", "requestId", "requestPrincipalSource"] {
            command.as_object_mut().unwrap().remove(field);
        }
        command
    }

    #[test]
    fn cross_adapter_matrix_preserves_commands_and_exact_error_envelopes() {
        let valid = [
            json!({"action":"navigate","params":{"url":"https://params.example"},"url":"https://top.example"}),
            valid_request_for_action("cdp_free_launch"),
            valid_request_for_action("evaluate"),
            valid_request_for_action("file_transfer"),
        ];
        for request in valid {
            let body = serde_json::to_string(&request).unwrap();
            let http = crate::native::stream::service_request_adapter_fixture(&body).unwrap();
            let mcp = crate::mcp::service_request_adapter_fixture(&request).unwrap();
            assert_eq!(without_transport_id(http), without_transport_id(mcp));
        }

        let invalid = [
            json!({"action":"navigate","jobTimeoutMs":"1000"}),
            json!({"action":"navigate","surprise":true}),
            json!({"action":"tab_new","blockedByManualAction":true,"manualSeedingRequired":true}),
            json!({"action":"evaluate","script":"document.title","timeoutMs":1000,"maxReturnBytes":128}),
            json!({"action":"network_capture","serviceTabHandle":test_tab_handle(true),"networkCapture":{"maxEvents":1}}),
        ];
        for request in invalid {
            let message = normalize(request.clone())
                .unwrap_err()
                .message()
                .to_string();
            let body = serde_json::to_string(&request).unwrap();
            assert_eq!(
                crate::native::stream::service_request_adapter_fixture(&body).unwrap_err(),
                json!({
                    "status": "400 Bad Request",
                    "body": {"success": false, "error": message}
                })
            );
            assert_eq!(
                crate::mcp::service_request_adapter_fixture(&request).unwrap_err(),
                json!({
                    "jsonrpc": "2.0",
                    "id": "fixture",
                    "error": {
                        "code": -32602,
                        "message": "Invalid params",
                        "data": {"message": message}
                    }
                })
            );
        }
    }

    #[test]
    fn http_and_mcp_profile_selection_reach_the_shared_mismatch_guard_unchanged() {
        let request = json!({
            "action": "navigate",
            "runtimeProfile": "last30days-facebook",
            "serviceName": "Last30Days",
            "agentName": "collector",
            "taskName": "facebook-search"
        });
        let body = serde_json::to_string(&request).unwrap();
        let commands = [
            crate::native::stream::service_request_adapter_fixture(&body).unwrap(),
            crate::mcp::service_request_adapter_fixture(&request).unwrap(),
        ];

        for command in commands {
            assert_eq!(command["runtimeProfile"], "last30days-facebook");
            let mismatch =
                crate::native::action_runtime::runtime::active_browser_profile_mismatch_message(
                    command.get("runtimeProfile").and_then(Value::as_str),
                    command.get("profile").and_then(Value::as_str),
                    Some("default"),
                    Some(std::path::Path::new(
                        "/tmp/agent-browser/runtime-profiles/default/user-data",
                    )),
                    "default",
                )
                .expect("named profile must not cross-attach to the default profile");
            assert!(mismatch.contains("selected profile mismatch"));
        }
    }
}
