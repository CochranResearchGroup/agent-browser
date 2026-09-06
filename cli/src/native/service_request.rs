//! Canonical service-request normalization shared by HTTP and MCP ingress.
//!
//! Transports retain parsing, request identifiers, error envelopes, relay
//! selection, and queue I/O. This module owns the public top-level field
//! ledger, validation, merge precedence, trace projection, and route-hint
//! ordering. The HTTP-only top-level `args` compatibility overlay is
//! deliberately handled by the HTTP adapter and is not canonical here.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fmt;

use crate::native::remote_view_handoff::apply_remote_view_handoff_route_hints;
use crate::native::service_access::apply_shared_profile_route_hints_with_decision;
use crate::native::service_contracts::{DESKTOP_CAPTURE_HARD_MAX_BYTES, SERVICE_REQUEST_ACTIONS};
use crate::native::service_failure_journal::{
    ServiceFailureCategory, ServiceFailureRecord, ServiceFailureReferences,
};
use crate::native::service_model::ServiceState;
use crate::native::service_principal::AuthenticatedServicePrincipal;
use crate::native::service_profile_access_policy::ServiceProfileAccessDecision;

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
const IDENTITY_ASSURANCE_LEVELS: &[&str] = &[
    "self-declared",
    "authenticated-ingress",
    "registered-capability",
    "operator",
    "unknown",
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
        "serviceStateLockTimeoutMs",
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
    ServiceRequestFieldSpec::field("clientSubjectId", FieldKind::String, true, true, true),
    ServiceRequestFieldSpec::field(
        "identityAssurance",
        FieldKind::Enum(IDENTITY_ASSURANCE_LEVELS),
        true,
        true,
        true,
    ),
    ServiceRequestFieldSpec::field(
        "policyRevision",
        FieldKind::PositiveInteger,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field("accessDecisionId", FieldKind::String, true, true, false),
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
    ServiceRequestFieldSpec::field("evidenceSurface", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("episodeId", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("includeFrame", FieldKind::Boolean, true, true, false),
    ServiceRequestFieldSpec::field(
        "includeVisualization",
        FieldKind::Boolean,
        true,
        true,
        false,
    ),
    ServiceRequestFieldSpec::field("promptProfileId", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("controllerLeaseId", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("operationId", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("recipe", FieldKind::Object, true, true, false),
    ServiceRequestFieldSpec::field("handoffId", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("remoteViewHandoffId", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("pid", FieldKind::PositiveInteger, true, true, false),
    ServiceRequestFieldSpec::field("expiresAt", FieldKind::String, true, true, false),
    ServiceRequestFieldSpec::field("plan", FieldKind::Object, true, true, false),
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ServiceRequestIssue {
    pub kind: ServiceRequestIssueKind,
    message: String,
    access_decision: Option<Box<ServiceProfileAccessDecision>>,
}

impl ServiceRequestIssue {
    pub(crate) fn new(kind: ServiceRequestIssueKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            access_decision: None,
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn code(&self) -> &str {
        if self.access_decision.is_some() {
            return "profile_access_denied";
        }
        match self.kind {
            ServiceRequestIssueKind::InvalidRequest => "invalid_request",
            ServiceRequestIssueKind::MissingAction => "missing_action",
            ServiceRequestIssueKind::UnsupportedAction => "unsupported_action",
            ServiceRequestIssueKind::UnknownField => "unknown_field",
            ServiceRequestIssueKind::InvalidFieldType => "invalid_field_type",
            ServiceRequestIssueKind::InvalidFieldValue => "invalid_field_value",
            ServiceRequestIssueKind::BlockedManualAction => "blocked_manual_action",
            ServiceRequestIssueKind::StaleMonitorEvidence => "stale_monitor_evidence",
            ServiceRequestIssueKind::ForbiddenCdpExecution => "forbidden_cdp_execution",
            ServiceRequestIssueKind::InvalidServiceTabHandle => "invalid_service_tab_handle",
            ServiceRequestIssueKind::InvalidBoundedRecipe => "invalid_bounded_recipe",
            ServiceRequestIssueKind::RouteHintFailure => match self.message.as_str() {
                "service_access_plan_route_browser_conflict"
                | "service_access_plan_route_session_conflict" => self.message.as_str(),
                _ => "route_hint_failure",
            },
            ServiceRequestIssueKind::MissingAccountablePrincipal => "missing_accountable_principal",
        }
    }
}

/// Retains the typed pre-dispatch failure and its journal correlation across
/// transport adapters. No browser effect or job exists at this boundary.
#[derive(Debug)]
pub(crate) struct ServiceRequestRejection {
    issue: ServiceRequestIssue,
    request_id: String,
}

impl ServiceRequestRejection {
    pub(crate) fn record(
        source: &str,
        action: Option<&str>,
        request_id: &str,
        session_id: &str,
        issue: ServiceRequestIssue,
    ) -> Self {
        let record = service_request_rejection_failure_record(
            source, action, request_id, session_id, &issue,
        );
        #[cfg(not(test))]
        crate::native::service_failure_journal::append_service_failure_best_effort(&record);
        #[cfg(test)]
        let _ = record;
        Self {
            issue,
            request_id: request_id.to_string(),
        }
    }

    pub(crate) fn response(&self) -> Value {
        use crate::native::service_failure::{
            classify_service_failure, ServiceEffectState, ServiceFailureAxis, ServiceFailurePhase,
            ServiceRetryDisposition,
        };
        let mut failure = classify_service_failure(self.issue.message());
        if failure.axis == ServiceFailureAxis::Unknown {
            failure.axis = ServiceFailureAxis::Request;
            failure.recommended_action = "correct_service_request".to_string();
            failure.safe_next_actions = vec!["inspect_service_request_schema".to_string()];
            failure.retry_disposition = ServiceRetryDisposition::DoNotRetry;
        }
        if let Some(decision) = self.issue.access_decision.as_ref() {
            failure.axis = ServiceFailureAxis::ProfileAccess;
            failure.subject = Some(json!(decision.subject));
            failure.missing_permission = decision.missing_permission.clone();
            failure.recommended_action = decision.next_action.action.clone();
            failure.executable_next_action = Some(json!(decision.next_action));
            failure.safe_next_actions = vec![decision.next_action.action.clone()];
            failure.retry_disposition = ServiceRetryDisposition::InspectBeforeRetry;
            failure.hard_stops = vec![
                "blind_retry".to_string(),
                "impersonate_profile_subject".to_string(),
            ];
        }
        failure.code = self.issue.code().to_string();
        failure.phase = ServiceFailurePhase::IngressValidation;
        failure.effect_state = ServiceEffectState::NoEffect;
        let mut response = json!({ "success": false, "id": self.request_id,
            "error": self.issue.message(), "failure": failure });
        if let Some(decision) = self.issue.access_decision.as_ref() {
            response["profileAccessDecision"] = json!(decision);
        }
        response
    }
}

impl fmt::Display for ServiceRequestRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.issue.message())
    }
}

/// Build privacy-bounded evidence for a request rejected before job creation.
pub(crate) fn service_request_rejection_failure_record(
    source: &str,
    action: Option<&str>,
    request_id: &str,
    session_id: &str,
    issue: &ServiceRequestIssue,
) -> ServiceFailureRecord {
    let mut record = ServiceFailureRecord::new(
        ServiceFailureCategory::ServiceAction,
        source,
        "ingress_validation",
        issue.code(),
        issue.message(),
    )
    .with_references(ServiceFailureReferences {
        request_id: Some(request_id.to_string()),
        session_id: Some(session_id.to_string()),
        profile_id: issue
            .access_decision
            .as_ref()
            .and_then(|decision| decision.resource.profile_id.clone()),
        ..ServiceFailureReferences::default()
    });
    if let Some(decision) = issue.access_decision.as_ref() {
        record = record.with_details(json!({
            "profileAccessDecision": decision,
            "effectState": "no_effect",
            "retryDisposition": "inspect_before_retry",
        }));
        if record.details.is_none() {
            record = record.with_details(json!({
                "profileAccessDecisionOmitted": "record_size_limit",
                "effectState": "no_effect",
                "retryDisposition": "inspect_before_retry",
            }));
        }
    }
    if let Some(action) = action {
        record = record.with_action(action);
    }
    record
}

impl fmt::Display for ServiceRequestIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

pub(crate) struct ServiceRequestNormalization<'a> {
    pub request: &'a Value,
    pub service_state: Option<&'a ServiceState>,
    /// Transport-authenticated profile authority. Public request fields cannot
    /// populate this value.
    pub authenticated_principal: Option<&'a AuthenticatedServicePrincipal>,
    pub fallback_principal: Option<ServiceRequestFallbackPrincipal<'a>>,
    pub request_id: &'a str,
    pub effective_session: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ServiceRequestPrincipalSource {
    ExplicitLabels,
    AttributionTupleV1,
    AuthenticatedDashboard,
    LocalProcess,
}

impl ServiceRequestPrincipalSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitLabels => "explicit_labels",
            Self::AttributionTupleV1 => "attribution_tuple_v1",
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
    pub principal_authority: Option<AuthenticatedServicePrincipal>,
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
    let explicit_profile_routing = ["runtimeProfile", "profileId", "profile"]
        .iter()
        .any(|field| {
            request
                .get(*field)
                .or_else(|| request.get("params").and_then(|params| params.get(*field)))
                .and_then(Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
        });
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
    if action == "view_focus" {
        command[crate::runtime_host::SERVICE_REQUEST_EXPLICIT_PROFILE_ROUTING_FIELD] =
            json!(explicit_profile_routing);
    }
    if let Some(params) = request.get("params") {
        let params = params.as_object().ok_or_else(|| {
            ServiceRequestIssue::new(
                ServiceRequestIssueKind::InvalidFieldType,
                "params must be a JSON object",
            )
        })?;
        for (key, value) in params {
            if !matches!(
                key.as_str(),
                "id" | "action" | "connectionInstanceId" | "profileChildAccess"
            ) {
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

    if action == "desktop_interact" && command.get("sessionName").is_none() {
        let effective_session = input
            .effective_session
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ServiceRequestIssue::new(
                    ServiceRequestIssueKind::InvalidBoundedRecipe,
                    "desktop_interact requires an effective session route",
                )
            })?;
        command["sessionName"] = json!(effective_session);
    }

    if let Some(service_state) = input.service_state {
        for stage in ROUTE_HINT_ORDER {
            match stage {
                RouteHintStage::RetainedHandoff => {
                    apply_remote_view_handoff_route_hints(service_state, &mut command);
                }
                RouteHintStage::SharedProfile => {
                    apply_shared_profile_route_hints_with_decision(
                        service_state,
                        &mut command,
                        input.authenticated_principal,
                    )
                    .map_err(|failure| ServiceRequestIssue {
                        kind: ServiceRequestIssueKind::RouteHintFailure,
                        message: failure.message,
                        access_decision: failure.access_decision,
                    })?;
                }
            }
        }
    }

    if action == "desktop_interact" {
        command["operationPrincipalId"] = json!(desktop_interact_operation_principal_id(request));
    }

    let principal_authority = input.authenticated_principal.cloned();
    if let Some(authority) = &principal_authority {
        command["servicePrincipalId"] = json!(authority.principal_id);
        command["servicePrincipalProvenance"] = json!(authority.provenance.as_str());
        command["serviceProfileCapabilityId"] = json!(authority.capability_id);
        command["serviceProfileCapabilityRevision"] = json!(authority.capability_revision);
    }

    Ok(NormalizedServiceRequest {
        command,
        trace,
        attribution,
        principal_authority,
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
            if request.get("action").and_then(Value::as_str) == Some("desktop_interact") {
                ServiceRequestPrincipalSource::AttributionTupleV1
            } else {
                ServiceRequestPrincipalSource::ExplicitLabels
            },
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

fn desktop_interact_operation_principal_id(request: &Map<String, Value>) -> String {
    let attribution_tuple = ["serviceName", "agentName", "taskName"].map(|field| {
        request
            .get(field)
            .and_then(Value::as_str)
            .expect("validated desktop interaction attribution")
    });
    let canonical = serde_json::to_string(&attribution_tuple)
        .expect("desktop interaction attribution tuple serializes");
    format!(
        "operation-principal-v1:{:x}",
        Sha256::digest(canonical.as_bytes())
    )
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
    reject_unexecutable_tab_new_route_intent(action, request)?;
    reject_bounded_evaluate_service_request(action, request)?;
    reject_service_diagnostics_request(action, request)?;
    reject_desktop_capture_request(action, request)?;
    reject_desktop_locate_request(action, request)?;
    reject_desktop_evidence_observe_request(action, request)?;
    reject_desktop_prompt_observe_request(action, request)?;
    reject_desktop_interact_request(action, request)?;
    reject_service_probe_request(action, request)?;
    reject_tab_handle_refresh_request(action, request)?;
    reject_service_ui_action_request(action, request)?;
    reject_service_network_capture_request(action, request)?;
    reject_service_file_transfer_request(action, request)?;
    reject_stale_monitor_service_request(request)
}

/// Reject route-bound intent before a generic tab job can be enqueued.
///
/// `tab_new` uses the generic browser auto-launch path, which has no authority
/// to reserve a presentation route. Callers must use the route-aware
/// `remote_view_open` action, whose successful result includes a tab handle.
fn reject_unexecutable_tab_new_route_intent(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "tab_new" {
        return Ok(());
    }
    const ROUTE_FIELDS: &[&str] = &[
        "routePoolEntryId",
        "remoteViewRouteId",
        "routeId",
        "viewStreamRouteId",
        "displayAllocationId",
        "displayName",
    ];
    let params = request.get("params").and_then(Value::as_object);
    if ROUTE_FIELDS.iter().any(|field| {
        request.contains_key(*field) || params.is_some_and(|value| value.contains_key(*field))
    }) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "tab_new cannot execute remote-view route intent; use authenticated remote_view_open to acquire the route and serviceTabHandle",
        ));
    }
    Ok(())
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

fn reject_desktop_interact_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "desktop_interact" {
        return Ok(());
    }
    const TOP_LEVEL_FIELDS: &[&str] = &[
        "action",
        "browserId",
        "sessionName",
        "controllerLeaseId",
        "operationId",
        "recipe",
        "serviceName",
        "agentName",
        "taskName",
        "jobTimeoutMs",
    ];
    if let Some(field) = request
        .keys()
        .find(|field| !TOP_LEVEL_FIELDS.contains(&field.as_str()))
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            format!("desktop_interact does not accept {field}"),
        ));
    }
    for field in [
        "browserId",
        "controllerLeaseId",
        "operationId",
        "serviceName",
        "agentName",
        "taskName",
    ] {
        if request
            .get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                format!("desktop_interact requires {field}"),
            ));
        }
    }
    if request.get("sessionName").is_some_and(|value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    }) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_interact sessionName must be a nonempty string",
        ));
    }
    let recipe = request
        .get("recipe")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "desktop_interact requires recipe",
            )
        })?;
    if let Some(field) = recipe.keys().find(|field| field.as_str() != "recipeId") {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            format!("desktop_interact recipe does not accept {field}"),
        ));
    }
    if !matches!(
        recipe.get("recipeId").and_then(Value::as_str),
        Some("p110-pointer-keyboard-v1" | "p110-foundation-stress-v1" | "p131-controlled-x11-v1")
    ) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_interact requires a supported recipeId",
        ));
    }
    Ok(())
}

fn reject_desktop_evidence_observe_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "desktop_evidence_observe" {
        return Ok(());
    }
    const TOP_LEVEL_FIELDS: &[&str] = &[
        "action",
        "browserId",
        "sessionName",
        "episodeId",
        "evidenceSurface",
        "includeFrame",
        "serviceTabHandle",
        "uiAction",
        "serviceName",
        "agentName",
        "taskName",
        "jobTimeoutMs",
    ];
    if let Some(field) = request
        .keys()
        .find(|field| !TOP_LEVEL_FIELDS.contains(&field.as_str()))
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            format!("desktop_evidence_observe does not accept {field}"),
        ));
    }
    for field in [
        "browserId",
        "episodeId",
        "serviceName",
        "agentName",
        "taskName",
    ] {
        if request
            .get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                format!("desktop_evidence_observe requires {field}"),
            ));
        }
    }
    if request.get("sessionName").is_some_and(|value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    }) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_evidence_observe sessionName must be a nonempty string",
        ));
    }
    match request.get("evidenceSurface").and_then(Value::as_str) {
        Some("stacking_or_occlusion") => {
            if request.contains_key("serviceTabHandle") || request.contains_key("uiAction") {
                return Err(issue(
                    ServiceRequestIssueKind::InvalidBoundedRecipe,
                    "desktop_evidence_observe stacking_or_occlusion does not accept serviceTabHandle or uiAction",
                ));
            }
        }
        Some("passkey_chooser") => {
            if !request
                .get("serviceTabHandle")
                .is_some_and(Value::is_object)
            {
                return Err(issue(
                    ServiceRequestIssueKind::InvalidBoundedRecipe,
                    "desktop_evidence_observe passkey_chooser requires serviceTabHandle",
                ));
            }
            let valid_trigger = request
                .get("uiAction")
                .and_then(Value::as_object)
                .is_some_and(|action| {
                    action
                        .keys()
                        .all(|field| matches!(field.as_str(), "steps" | "maxActions"))
                        && action
                            .get("maxActions")
                            .map(Value::as_u64)
                            .unwrap_or(Some(1))
                            == Some(1)
                        && action
                            .get("steps")
                            .and_then(Value::as_array)
                            .filter(|steps| steps.len() == 1)
                            .and_then(|steps| steps[0].as_object())
                            .is_some_and(|step| {
                                step.keys()
                                    .all(|field| matches!(field.as_str(), "type" | "selector"))
                                    && step.get("type").and_then(Value::as_str) == Some("click")
                                    && step
                                        .get("selector")
                                        .and_then(Value::as_str)
                                        .map(str::trim)
                                        .is_some_and(|value| !value.is_empty())
                            })
                });
            if !valid_trigger {
                return Err(issue(
                    ServiceRequestIssueKind::InvalidBoundedRecipe,
                    "desktop_evidence_observe passkey_chooser requires exactly one selector-based uiAction click",
                ));
            }
        }
        _ => {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                "desktop_evidence_observe evidenceSurface must be stacking_or_occlusion or passkey_chooser",
            ));
        }
    }
    if request
        .get("includeFrame")
        .is_some_and(|value| !value.is_boolean())
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_evidence_observe includeFrame must be a boolean",
        ));
    }
    Ok(())
}

fn reject_desktop_prompt_observe_request(
    action: &str,
    request: &Map<String, Value>,
) -> Result<(), ServiceRequestIssue> {
    if action != "desktop_prompt_observe" {
        return Ok(());
    }
    const TOP_LEVEL_FIELDS: &[&str] = &[
        "action",
        "browserId",
        "sessionName",
        "promptProfileId",
        "includeVisualization",
        "serviceName",
        "agentName",
        "taskName",
        "jobTimeoutMs",
    ];
    if let Some(field) = request
        .keys()
        .find(|field| !TOP_LEVEL_FIELDS.contains(&field.as_str()))
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            format!("desktop_prompt_observe does not accept {field}"),
        ));
    }
    for field in ["browserId", "serviceName", "agentName", "taskName"] {
        if request
            .get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            return Err(issue(
                ServiceRequestIssueKind::InvalidBoundedRecipe,
                format!("desktop_prompt_observe requires {field}"),
            ));
        }
    }
    if request.get("sessionName").is_some_and(|value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    }) {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_prompt_observe sessionName must be a nonempty string",
        ));
    }
    if request.get("promptProfileId").and_then(Value::as_str) != Some("p110-external-prompt-v1") {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_prompt_observe requires promptProfileId p110-external-prompt-v1",
        ));
    }
    if request
        .get("includeVisualization")
        .is_some_and(|value| !value.is_boolean())
    {
        return Err(issue(
            ServiceRequestIssueKind::InvalidBoundedRecipe,
            "desktop_prompt_observe includeVisualization must be a boolean",
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
            authenticated_principal: None,
            fallback_principal: Some(ServiceRequestFallbackPrincipal {
                source: ServiceRequestPrincipalSource::LocalProcess,
                principal: "local:test-normalizer",
            }),
            request_id: "test-normalizer-request",
            effective_session: Some("test-normalizer"),
        })
    }

    #[test]
    fn view_focus_records_whether_profile_routing_was_caller_authored() {
        let inherited = normalize(json!({
            "action": "view_focus",
            "browserId": "session:retained",
            "sessionName": "retained",
            "params": {"targetId": "target-1", "index": 1, "maximize": true}
        }))
        .unwrap();
        assert_eq!(
            inherited.command[crate::runtime_host::SERVICE_REQUEST_EXPLICIT_PROFILE_ROUTING_FIELD],
            false
        );

        let explicit = normalize(json!({
            "action": "view_focus",
            "browserId": "session:retained",
            "sessionName": "retained",
            "runtimeProfile": "caller-selected-profile",
            "params": {"targetId": "target-1", "index": 1, "maximize": true}
        }))
        .unwrap();
        assert_eq!(
            explicit.command[crate::runtime_host::SERVICE_REQUEST_EXPLICIT_PROFILE_ROUTING_FIELD],
            true
        );
    }

    #[test]
    fn effectful_request_without_labels_or_fallback_principal_fails_closed() {
        let request = json!({"action": "navigate", "params": {"url": "https://example.com"}});
        let error = normalize_service_request(ServiceRequestNormalization {
            request: &request,
            service_state: None,
            authenticated_principal: None,
            fallback_principal: None,
            request_id: "http-service-request-navigate-test",
            effective_session: Some("test-normalizer"),
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
            authenticated_principal: None,
            fallback_principal: Some(ServiceRequestFallbackPrincipal {
                source: ServiceRequestPrincipalSource::AuthenticatedDashboard,
                principal: "dashboard-admin",
            }),
            request_id: "request-42",
            effective_session: Some("test-normalizer"),
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

    #[test]
    fn authenticated_profile_authority_is_separate_from_caller_labels() {
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };

        let request = json!({
            "action": "navigate",
            "serviceName": "CallerSuppliedLabel",
            "agentName": "caller-agent",
            "taskName": "caller-task"
        });
        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:registered-service".to_string(),
            profile_id: "registered-profile".to_string(),
            capability_id: "profile-capability-v1:fixture".to_string(),
            capability_revision: 3,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        let normalized = normalize_service_request(ServiceRequestNormalization {
            request: &request,
            service_state: None,
            authenticated_principal: Some(&authority),
            fallback_principal: None,
            request_id: "request-with-registered-authority",
            effective_session: Some("registered-session"),
        })
        .unwrap();

        assert_eq!(
            normalized.attribution.principal,
            "service:CallerSuppliedLabel/agent:caller-agent/task:caller-task"
        );
        assert_eq!(normalized.principal_authority.as_ref(), Some(&authority));
        assert_eq!(
            normalized.command["servicePrincipalId"],
            "principal:registered-service"
        );
        assert_eq!(
            normalized.command["servicePrincipalProvenance"],
            "registered_capability"
        );
        assert!(normalized.command.get("profileCapability").is_none());

        let forged = normalize(json!({
            "action": "navigate",
            "serviceName": "CallerSuppliedLabel",
            "agentName": "caller-agent",
            "taskName": "caller-task",
            "servicePrincipalId": "principal:registered-service"
        }))
        .unwrap_err();
        assert_eq!(forged.kind, ServiceRequestIssueKind::UnknownField);
        assert_eq!(
            forged.message(),
            "unknown service request field: servicePrincipalId"
        );

        let forged_route_authorization = normalize(json!({
            "action": "tab_new",
            "serviceProfileRouteAuthorization": {
                "schemaVersion": "agent-browser.profile-launch-route-authorization.v1",
                "kind": "authenticated_cold"
            }
        }))
        .unwrap_err();
        assert_eq!(
            forged_route_authorization.kind,
            ServiceRequestIssueKind::UnknownField
        );
        assert_eq!(
            forged_route_authorization.message(),
            "unknown service request field: serviceProfileRouteAuthorization"
        );
    }

    #[test]
    fn request_admission_uses_authenticated_principal_for_profile_reuse() {
        use std::collections::BTreeMap;

        use crate::native::service_model::{
            BrowserHealth, BrowserProcess, BrowserProfile, BrowserSession, LeaseState,
        };
        use crate::native::service_principal::{
            AuthenticatedServicePrincipal, ServicePrincipalProvenance,
        };

        let authority = AuthenticatedServicePrincipal {
            principal_id: "principal:foreign-service".to_string(),
            profile_id: "odollo-fedex".to_string(),
            capability_id: "profile-capability-v1:foreign-fedex".to_string(),
            capability_revision: 1,
            provenance: ServicePrincipalProvenance::RegisteredCapability,
        };
        let mut state = ServiceState {
            profiles: BTreeMap::from([(
                "odollo-fedex".to_string(),
                BrowserProfile {
                    id: "odollo-fedex".to_string(),
                    target_service_ids: vec!["fedex".to_string()],
                    authenticated_service_ids: vec!["fedex".to_string()],
                    access_policy: Some(
                        crate::native::service_profile_access_policy::ServiceProfileAccessPolicy {
                            profile_id: "odollo-fedex".to_string(),
                            mode: crate::native::service_profile_access_policy::ProfileAccessMode::Restricted,
                            default_permissions: vec![
                                crate::native::service_profile_access_policy::ProfilePermission::TabCreate,
                            ],
                            ..crate::native::service_profile_access_policy::ServiceProfileAccessPolicy::default()
                        },
                    ),
                    ..BrowserProfile::default()
                },
            )]),
            browsers: BTreeMap::from([(
                "browser-fedex".to_string(),
                BrowserProcess {
                    id: "browser-fedex".to_string(),
                    profile_id: Some("odollo-fedex".to_string()),
                    health: BrowserHealth::Ready,
                    active_session_ids: vec!["session-fedex-owner".to_string()],
                    ..BrowserProcess::default()
                },
            )]),
            sessions: BTreeMap::from([(
                "session-fedex-owner".to_string(),
                BrowserSession {
                    id: "session-fedex-owner".to_string(),
                    principal_id: Some("principal:odollo-fulfillment".to_string()),
                    principal_provenance: Some(ServicePrincipalProvenance::RegisteredCapability),
                    profile_id: Some("odollo-fedex".to_string()),
                    browser_ids: vec!["browser-fedex".to_string()],
                    lease: LeaseState::Exclusive,
                    ..BrowserSession::default()
                },
            )]),
            ..ServiceState::default()
        };
        let request = json!({
            "action": "tab_new",
            "runtimeProfile": "odollo-fedex",
            "targetServiceIds": ["fedex"],
            "serviceName": "OdolloFulfillment",
            "agentName": "worker",
            "taskName": "lookup-fedex-tracking"
        });

        let error = normalize_service_request(ServiceRequestNormalization {
            request: &request,
            service_state: Some(&state),
            authenticated_principal: Some(&authority),
            fallback_principal: None,
            request_id: "request-foreign-fedex",
            effective_session: Some("foreign-fedex"),
        })
        .unwrap_err();

        assert_eq!(error.kind, ServiceRequestIssueKind::RouteHintFailure);
        assert_eq!(
            error.message(),
            "service_access_plan_request_unavailable:foreign_principal_profile_lease"
        );

        // A real denied policy decision must survive request admission rather
        // than sending an authenticated caller to request-schema repair.
        state
            .profiles
            .get_mut("odollo-fedex")
            .unwrap()
            .access_policy
            .as_mut()
            .unwrap()
            .default_permissions
            .clear();
        let denied = normalize_service_request(ServiceRequestNormalization {
            request: &request,
            service_state: Some(&state),
            authenticated_principal: Some(&authority),
            fallback_principal: None,
            request_id: "request-denied-profile",
            effective_session: Some("foreign-fedex"),
        })
        .unwrap_err();
        let record = service_request_rejection_failure_record(
            "http_service_request",
            Some("tab_new"),
            "request-denied-profile",
            "foreign-fedex",
            &denied,
        );
        let response = ServiceRequestRejection::record(
            "http_service_request",
            Some("tab_new"),
            "request-denied-profile",
            "foreign-fedex",
            denied,
        )
        .response();
        assert_eq!(response["failure"]["code"], "profile_access_denied");
        assert_eq!(response["failure"]["axis"], "profile_access");
        assert_eq!(response["failure"]["effectState"], "no_effect");
        assert_eq!(
            response["failure"]["subject"]["subjectId"],
            authority.principal_id
        );
        assert_eq!(
            response["failure"]["subject"]["assurance"],
            "registered-capability"
        );
        assert_eq!(response["failure"]["missingPermission"], "tab_create");
        assert_eq!(
            response["failure"]["recommendedAction"],
            "inspect_profile_access_policy"
        );
        assert_eq!(response["profileAccessDecision"]["allowed"], false);
        assert_eq!(response["profileAccessDecision"]["policyRevision"], 1);
        assert_eq!(record.code, "profile_access_denied");
        assert_eq!(
            record.details.as_ref().unwrap()["profileAccessDecision"],
            response["profileAccessDecision"]
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

        assert_eq!(canonical_names.len(), 83);
        assert_eq!(canonical_names, spec_names);
        assert_eq!(
            role_contract["canonicalPropertyCount"].as_u64(),
            Some(SERVICE_REQUEST_FIELDS.len() as u64)
        );
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
                "connectionInstanceId": "caller-connection",
                "profileChildAccess": {
                    "subjectId": "caller-subject",
                    "permissions": ["profile-admin"]
                },
                "url": "https://params.example",
                "args": ["--from-params"]
            },
            "url": "https://top.example"
        }))
        .unwrap();
        assert!(normalized.command.get("id").is_none());
        assert!(normalized.command.get("connectionInstanceId").is_none());
        assert!(normalized.command.get("profileChildAccess").is_none());
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
    fn route_pool_repair_params_project_exact_acquisition_lease_into_command() {
        let normalized = normalize(json!({
            "action": "service_route_pool_repair",
            "params": {
                "acquisitionLeaseId": "lease-terminal",
                "apply": false,
                "staleCheckouts": false,
                "stalePendingAcquisitions": true
            }
        }))
        .unwrap();

        assert_eq!(normalized.command["acquisitionLeaseId"], "lease-terminal");
        assert_eq!(normalized.command["apply"], false);
        assert_eq!(normalized.command["staleCheckouts"], false);
        assert_eq!(normalized.command["stalePendingAcquisitions"], true);
    }

    #[test]
    fn service_state_lock_timeout_is_a_canonical_command_and_trace_field() {
        let normalized = normalize(json!({
            "action": "service_remote_view_handoff_resolve",
            "serviceStateLockTimeoutMs": 30_000,
            "params": { "handoffId": "handoff-a" }
        }))
        .unwrap();

        assert_eq!(normalized.command["serviceStateLockTimeoutMs"], 30_000);
        assert_eq!(normalized.trace["serviceStateLockTimeoutMs"], 30_000);
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
    fn route_bearing_tab_new_is_rejected_before_job_creation() {
        let error = normalize(json!({
            "action": "tab_new",
            "params": {
                "url": "https://example.test/",
                "routePoolEntryId": "guacamole-rdp-b"
            }
        }))
        .unwrap_err();

        assert_eq!(error.kind, ServiceRequestIssueKind::InvalidBoundedRecipe);
        assert_eq!(
            error.message(),
            "tab_new cannot execute remote-view route intent; use authenticated remote_view_open to acquire the route and serviceTabHandle"
        );
        let record = service_request_rejection_failure_record(
            "mcp_service_request",
            Some("tab_new"),
            "mcp-service-request-tab-new-fixture",
            "default",
            &error,
        );
        assert_eq!(record.category, ServiceFailureCategory::ServiceAction);
        assert_eq!(record.stage, "ingress_validation");
        assert_eq!(record.code, "invalid_bounded_recipe");
        assert_eq!(record.action.as_deref(), Some("tab_new"));
        assert_eq!(
            record.references.request_id.as_deref(),
            Some("mcp-service-request-tab-new-fixture")
        );
    }

    #[test]
    fn route_conflict_failure_record_preserves_the_exact_actionable_code() {
        for code in [
            "service_access_plan_route_browser_conflict",
            "service_access_plan_route_session_conflict",
        ] {
            let issue = ServiceRequestIssue::new(ServiceRequestIssueKind::RouteHintFailure, code);
            let record = service_request_rejection_failure_record(
                "http_service_request",
                Some("tab_new"),
                "request-route-conflict",
                "shared-profile-route",
                &issue,
            );

            assert_eq!(issue.code(), code);
            assert_eq!(record.code, code);
        }
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

    #[test]
    fn desktop_interact_requires_one_attributed_named_recipe_without_raw_input() {
        let normalized = normalize(json!({
            "action": "desktop_interact",
            "browserId": "browser-1",
            "sessionName": "rdp-1",
            "controllerLeaseId": "lease-1",
            "operationId": "operation-1",
            "recipe": { "recipeId": "p110-pointer-keyboard-v1" },
            "serviceName": "DesktopInteractor",
            "agentName": "fixture-agent",
            "taskName": "verify-synthetic-control"
        }))
        .unwrap();
        assert_eq!(normalized.command["action"], "desktop_interact");
        assert_eq!(normalized.command["controllerLeaseId"], "lease-1");
        assert_eq!(normalized.command["operationId"], "operation-1");
        let controlled = normalize(json!({
            "action": "desktop_interact",
            "browserId": "browser-1",
            "sessionName": "rdp-1",
            "controllerLeaseId": "lease-1",
            "operationId": "controlled-operation-1",
            "recipe": { "recipeId": "p131-controlled-x11-v1" },
            "serviceName": "DesktopInteractor",
            "agentName": "fixture-agent",
            "taskName": "verify-controlled-x11"
        }))
        .unwrap();
        assert_eq!(
            controlled.command["recipe"]["recipeId"],
            "p131-controlled-x11-v1"
        );
        assert!(normalized.command["operationPrincipalId"]
            .as_str()
            .is_some_and(|value| value.starts_with("operation-principal-v1:")));
        assert_eq!(
            normalized.attribution.source,
            ServiceRequestPrincipalSource::AttributionTupleV1
        );
        let same_scope = normalize(json!({
            "action": "desktop_interact",
            "browserId": "browser-1",
            "sessionName": "rdp-1",
            "controllerLeaseId": "lease-1",
            "operationId": "another-operation",
            "recipe": { "recipeId": "p110-foundation-stress-v1" },
            "serviceName": "DesktopInteractor",
            "agentName": "fixture-agent",
            "taskName": "verify-synthetic-control"
        }))
        .unwrap();
        assert_eq!(
            normalized.command["operationPrincipalId"],
            same_scope.command["operationPrincipalId"]
        );
        let changed_principal = normalize(json!({
            "action": "desktop_interact",
            "browserId": "browser-1",
            "sessionName": "rdp-1",
            "controllerLeaseId": "lease-1",
            "operationId": "operation-1",
            "recipe": { "recipeId": "p110-pointer-keyboard-v1" },
            "serviceName": "DesktopInteractor",
            "agentName": "fixture-agent",
            "taskName": "different-task"
        }))
        .unwrap();
        assert_ne!(
            normalized.command["operationPrincipalId"],
            changed_principal.command["operationPrincipalId"]
        );
        for (request, expected_message) in [
            (
                json!({"action":"desktop_interact","browserId":"browser-1","controllerLeaseId":"lease-1","operationId":"operation-1","recipe":{"recipeId":"p110-pointer-keyboard-v1"},"serviceName":"DesktopInteractor","agentName":"fixture-agent"}),
                "desktop_interact requires taskName",
            ),
            (
                json!({"action":"desktop_interact","browserId":"browser-1","controllerLeaseId":"lease-1","operationId":"operation-1","recipe":{"recipeId":"wrong"},"serviceName":"DesktopInteractor","agentName":"fixture-agent","taskName":"verify"}),
                "desktop_interact requires a supported recipeId",
            ),
            (
                json!({"action":"desktop_interact","browserId":"browser-1","controllerLeaseId":"lease-1","operationId":"operation-1","recipe":{"recipeId":"p110-pointer-keyboard-v1"},"serviceName":"DesktopInteractor","agentName":"fixture-agent","taskName":"verify","params":{}}),
                "desktop_interact does not accept params",
            ),
            (
                json!({"action":"desktop_interact","browserId":"browser-1","controllerLeaseId":"lease-1","operationId":"operation-1","recipe":{"recipeId":"p110-pointer-keyboard-v1","text":"secret"},"serviceName":"DesktopInteractor","agentName":"fixture-agent","taskName":"verify"}),
                "desktop_interact recipe does not accept text",
            ),
        ] {
            let error = normalize(request).unwrap_err();
            assert_eq!(error.message(), expected_message);
        }
    }

    #[test]
    fn desktop_evidence_observe_requires_attributed_named_evidence_need() {
        let normalized = normalize(json!({
            "action": "desktop_evidence_observe",
            "browserId": "browser-1",
            "episodeId": "episode-1",
            "evidenceSurface": "stacking_or_occlusion",
            "includeFrame": false,
            "serviceName": "DesktopEvidence",
            "agentName": "fixture-agent",
            "taskName": "inspect-stacking"
        }))
        .unwrap();
        assert_eq!(normalized.command["action"], "desktop_evidence_observe");
        assert_eq!(normalized.command["episodeId"], "episode-1");

        let passkey = normalize(json!({
            "action": "desktop_evidence_observe",
            "browserId": "browser-1",
            "episodeId": "episode-2",
            "evidenceSurface": "passkey_chooser",
            "serviceTabHandle": {"browserId":"browser-1","tabId":"tab-1","targetId":"target-1","valid":true},
            "uiAction": {"maxActions":1,"steps":[{"type":"click","selector":"#show-passkeys"}]},
            "serviceName": "DesktopEvidence",
            "agentName": "fixture-agent",
            "taskName": "inspect-passkey-chooser"
        }))
        .unwrap();
        assert_eq!(passkey.command["evidenceSurface"], "passkey_chooser");

        for (request, expected) in [
            (
                json!({"action":"desktop_evidence_observe","browserId":"browser-1","episodeId":"episode-1","evidenceSurface":"stacking_or_occlusion","serviceName":"DesktopEvidence","agentName":"fixture-agent"}),
                "desktop_evidence_observe requires taskName",
            ),
            (
                json!({"action":"desktop_evidence_observe","browserId":"browser-1","episodeId":"episode-1","evidenceSurface":"browser_external_prompt","serviceName":"DesktopEvidence","agentName":"fixture-agent","taskName":"inspect"}),
                "desktop_evidence_observe evidenceSurface must be stacking_or_occlusion or passkey_chooser",
            ),
            (
                json!({"action":"desktop_evidence_observe","browserId":"browser-1","episodeId":"episode-1","evidenceSurface":"stacking_or_occlusion","serviceName":"DesktopEvidence","agentName":"fixture-agent","taskName":"inspect","params":{}}),
                "desktop_evidence_observe does not accept params",
            ),
            (
                json!({"action":"desktop_evidence_observe","browserId":"browser-1","episodeId":"episode-1","evidenceSurface":"stacking_or_occlusion","serviceName":"DesktopEvidence","agentName":"fixture-agent","taskName":"inspect","serviceJobId":"caller-job"}),
                "unknown service request field: serviceJobId",
            ),
        ] {
            assert_eq!(normalize(request).unwrap_err().message(), expected);
        }
    }

    #[test]
    fn desktop_prompt_observe_requires_attributed_top_level_named_profile() {
        let normalized = normalize(json!({
            "action": "desktop_prompt_observe",
            "browserId": "browser-1",
            "sessionName": "rdp-1",
            "promptProfileId": "p110-external-prompt-v1",
            "includeVisualization": true,
            "serviceName": "DesktopPromptObserver",
            "agentName": "fixture-agent",
            "taskName": "observe-synthetic-prompt"
        }))
        .unwrap();
        assert_eq!(normalized.command["action"], "desktop_prompt_observe");
        assert_eq!(
            normalized.command["promptProfileId"],
            "p110-external-prompt-v1"
        );

        for (request, expected_message) in [
            (
                json!({"action":"desktop_prompt_observe","browserId":"browser-1","promptProfileId":"p110-external-prompt-v1","serviceName":"DesktopPromptObserver","agentName":"fixture-agent"}),
                "desktop_prompt_observe requires taskName",
            ),
            (
                json!({"action":"desktop_prompt_observe","browserId":"browser-1","promptProfileId":"wrong","serviceName":"DesktopPromptObserver","agentName":"fixture-agent","taskName":"observe"}),
                "desktop_prompt_observe requires promptProfileId p110-external-prompt-v1",
            ),
            (
                json!({"action":"desktop_prompt_observe","browserId":"browser-1","promptProfileId":"p110-external-prompt-v1","serviceName":"DesktopPromptObserver","agentName":"fixture-agent","taskName":"observe","params":{}}),
                "desktop_prompt_observe does not accept params",
            ),
            (
                json!({"action":"desktop_prompt_observe","browserId":"browser-1","promptProfileId":"p110-external-prompt-v1","serviceName":"DesktopPromptObserver","agentName":"fixture-agent","taskName":"observe","promptText":"secret"}),
                "unknown service request field: promptText",
            ),
        ] {
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
            "desktop_locate" => {
                request["browserId"] = json!("browser:desktop-fixture");
                request["locator"] = json!({
                    "locatorId": "p110-control-v1",
                    "maxCandidates": 8
                });
            }
            "desktop_evidence_observe" => {
                request["browserId"] = json!("browser:desktop-fixture");
                request["episodeId"] = json!("episode-fixture");
                request["evidenceSurface"] = json!("stacking_or_occlusion");
                request["serviceName"] = json!("DesktopEvidence");
                request["agentName"] = json!("fixture-agent");
                request["taskName"] = json!("inspect-stacking");
            }
            "desktop_interact" => {
                request["browserId"] = json!("browser:desktop-fixture");
                request["controllerLeaseId"] = json!("controller-lease-fixture");
                request["operationId"] = json!("operation-fixture");
                request["recipe"] = json!({"recipeId": "p110-pointer-keyboard-v1"});
                request["serviceName"] = json!("DesktopInteractor");
                request["agentName"] = json!("fixture-agent");
                request["taskName"] = json!("verify-synthetic-control");
            }
            "desktop_prompt_observe" => {
                request["browserId"] = json!("browser:desktop-fixture");
                request["promptProfileId"] = json!("p110-external-prompt-v1");
                request["serviceName"] = json!("DesktopPromptObserver");
                request["agentName"] = json!("fixture-agent");
                request["taskName"] = json!("observe-synthetic-prompt");
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

    fn normalize_with_state(
        request: Value,
        state: &ServiceState,
    ) -> Result<NormalizedServiceRequest, ServiceRequestIssue> {
        normalize_service_request(ServiceRequestNormalization {
            request: &request,
            service_state: Some(state),
            authenticated_principal: None,
            fallback_principal: Some(ServiceRequestFallbackPrincipal {
                source: ServiceRequestPrincipalSource::LocalProcess,
                principal: "local:test-normalizer",
            }),
            request_id: "test-normalizer-request",
            effective_session: Some("test-normalizer"),
        })
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
        )
        .unwrap();
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
        let selected = normalize_with_state(base.clone(), &state).unwrap();
        assert_eq!(selected.command["browserId"], "browser-social");
        assert_eq!(selected.command["sessionName"], "operator-social");

        for overlay in [
            json!({"browserId": "session:explicit"}),
            json!({"sessionName": "explicit"}),
        ] {
            let mut request = base.clone();
            request
                .as_object_mut()
                .unwrap()
                .extend(overlay.as_object().unwrap().clone());
            let error = normalize_with_state(request, &state).unwrap_err();
            assert!(error.message().contains("service_access_plan_"));
        }

        let mut duplicate_request = base;
        duplicate_request["allowDuplicateProfileLane"] = json!(true);
        let duplicate = normalize_with_state(duplicate_request, &state).unwrap();
        assert!(duplicate.command.get("browserId").is_none());
        assert!(duplicate.command.get("sessionName").is_none());
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
            json!({
                "action": "desktop_interact",
                "browserId": "browser-rdp-1",
                "sessionName": "rdp-1",
                "controllerLeaseId": "lease-1",
                "operationId": "operation-stress-1",
                "recipe": { "recipeId": "p110-foundation-stress-v1" },
                "serviceName": "DesktopInteractor",
                "agentName": "fixture-agent",
                "taskName": "stress-synthetic-foundation"
            }),
            json!({
                "action": "desktop_prompt_observe",
                "browserId": "browser-rdp-1",
                "sessionName": "rdp-1",
                "promptProfileId": "p110-external-prompt-v1",
                "includeVisualization": false,
                "serviceName": "DesktopPromptObserver",
                "agentName": "fixture-agent",
                "taskName": "observe-synthetic-prompt"
            }),
        ];
        for request in valid {
            let body = serde_json::to_string(&request).unwrap();
            let http = crate::native::stream::service_request_adapter_fixture(&body).unwrap();
            let mcp = crate::mcp::service_request_adapter_fixture(&request).unwrap();
            if request["action"] == "desktop_interact" {
                let dedicated = crate::mcp::desktop_interact_adapter_fixture(&json!({
                    "browserId": request["browserId"],
                    "sessionName": request["sessionName"],
                    "controllerLeaseId": request["controllerLeaseId"],
                    "operationId": request["operationId"],
                    "recipeId": request["recipe"]["recipeId"],
                    "serviceName": request["serviceName"],
                    "agentName": request["agentName"],
                    "taskName": request["taskName"]
                }))
                .unwrap();
                for command in [&http, &mcp, &dedicated] {
                    assert_eq!(command["sessionName"], "rdp-1");
                    assert_eq!(command["requestPrincipalSource"], "attribution_tuple_v1");
                    assert_eq!(
                        command["operationPrincipalId"],
                        http["operationPrincipalId"]
                    );
                }
                assert_eq!(
                    without_transport_id(http.clone()),
                    without_transport_id(dedicated)
                );
            }
            assert_eq!(without_transport_id(http), without_transport_id(mcp));
        }

        let invalid = [
            json!({"action":"navigate","jobTimeoutMs":"1000"}),
            json!({"action":"navigate","surprise":true}),
            json!({"action":"tab_new","blockedByManualAction":true,"manualSeedingRequired":true}),
            json!({"action":"evaluate","script":"document.title","timeoutMs":1000,"maxReturnBytes":128}),
            json!({"action":"network_capture","serviceTabHandle":test_tab_handle(true),"networkCapture":{"maxEvents":1}}),
            json!({"action":"desktop_interact","browserId":"browser-rdp-1","controllerLeaseId":"lease-1","recipe":{"recipeId":"p110-foundation-stress-v1"},"serviceName":"DesktopInteractor","agentName":"fixture-agent","taskName":"stress"}),
            json!({"action":"desktop_prompt_observe","browserId":"browser-rdp-1","promptProfileId":"p110-external-prompt-v1","serviceName":"DesktopPromptObserver","agentName":"fixture-agent","taskName":"observe","params":{}}),
            json!({"action":"desktop_prompt_observe","browserId":"browser-rdp-1","promptProfileId":"wrong","serviceName":"DesktopPromptObserver","agentName":"fixture-agent","taskName":"observe"}),
        ];
        for request in invalid {
            let issue = normalize(request.clone()).unwrap_err();
            let body = serde_json::to_string(&request).unwrap();
            let http = crate::native::stream::service_request_adapter_fixture(&body).unwrap_err();
            let mcp = crate::mcp::service_request_adapter_fixture(&request).unwrap_err();
            assert_eq!(http["status"], "400 Bad Request");
            assert_eq!(http["body"]["success"], false);
            assert_eq!(http["body"]["error"], issue.message());
            assert_eq!(http["body"]["failure"]["code"], issue.code());
            assert_eq!(http["body"]["failure"]["phase"], "ingress_validation");
            assert_eq!(http["body"]["failure"]["effectState"], "no_effect");
            assert!(http["body"]["id"]
                .as_str()
                .unwrap()
                .starts_with("http-service-request-"));
            assert_eq!(mcp["error"]["code"], -32602);
            assert_eq!(mcp["error"]["message"], "Invalid params");
            assert_eq!(mcp["error"]["data"]["message"], issue.message());
            assert_eq!(mcp["error"]["data"]["failure"], http["body"]["failure"]);
            assert!(mcp["error"]["data"]["requestId"]
                .as_str()
                .unwrap()
                .starts_with("mcp-service-request-"));
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
