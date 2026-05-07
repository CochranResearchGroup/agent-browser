use serde_json::{json, Value};

pub const SERVICE_CONTRACTS_RESOURCE: &str = "agent-browser://contracts";
pub const SERVICE_CONTRACTS_HTTP_ROUTE: &str = "/api/service/contracts";
pub const SERVICE_REQUEST_HTTP_ROUTE: &str = "/api/service/request";
pub const SERVICE_PROFILE_ALLOCATION_HTTP_ROUTE: &str = "/api/service/profiles/<id>/allocation";
pub const SERVICE_PROFILE_READINESS_HTTP_ROUTE: &str = "/api/service/profiles/<id>/readiness";
pub const SERVICE_PROFILE_LOOKUP_HTTP_ROUTE: &str = "/api/service/profiles/lookup";
pub const SERVICE_ACCESS_PLAN_HTTP_ROUTE: &str = "/api/service/access-plan";
pub const SERVICE_MONITORS_RUN_DUE_HTTP_ROUTE: &str = "/api/service/monitors/run-due";
pub const SERVICE_ACCESS_PLAN_MCP_RESOURCE: &str = "agent-browser://access-plan";
pub const SERVICE_REQUEST_MCP_TOOL_NAME: &str = "service_request";
pub const SERVICE_MONITORS_RUN_DUE_MCP_TOOL_NAME: &str = "service_monitors_run_due";
pub const SERVICE_REQUEST_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-request.v1.schema.json";
pub const SERVICE_REQUEST_MCP_TOOL_CALL_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-request-mcp-tool-call.v1.schema.json";
pub const SERVICE_PROFILE_ALLOCATION_RESPONSE_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-profile-allocation-response.v1.schema.json";
pub const SERVICE_PROFILE_READINESS_RESPONSE_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-profile-readiness-response.v1.schema.json";
pub const SERVICE_PROFILE_LOOKUP_RESPONSE_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-profile-lookup-response.v1.schema.json";
pub const SERVICE_ACCESS_PLAN_RESPONSE_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-access-plan-response.v1.schema.json";
pub const SERVICE_MONITOR_RUN_DUE_RESPONSE_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-monitor-run-due-response.v1.schema.json";
pub const SERVICE_REQUEST_CONTRACT_VERSION: &str = "v1";

pub const SERVICE_REQUEST_ACTIONS: &[&str] = &[
    "navigate",
    "back",
    "forward",
    "reload",
    "tab_new",
    "tab_switch",
    "tab_close",
    "tab_list",
    "url",
    "title",
    "viewport",
    "user_agent",
    "emulatemedia",
    "timezone",
    "locale",
    "geolocation",
    "permissions",
    "cookies_get",
    "cookies_set",
    "cookies_clear",
    "storage_get",
    "storage_set",
    "storage_clear",
    "console",
    "errors",
    "setcontent",
    "headers",
    "offline",
    "dialog",
    "clipboard",
    "upload",
    "download",
    "waitfordownload",
    "pdf",
    "responsebody",
    "har_start",
    "har_stop",
    "route",
    "unroute",
    "requests",
    "request_detail",
    "snapshot",
    "screenshot",
    "click",
    "fill",
    "wait",
    "type",
    "press",
    "hover",
    "select",
    "gettext",
    "inputvalue",
    "isvisible",
    "getattribute",
    "innerhtml",
    "styles",
    "count",
    "boundingbox",
    "isenabled",
    "ischecked",
    "check",
    "uncheck",
    "scroll",
    "scrollintoview",
    "focus",
    "clear",
];

pub fn service_contracts_metadata() -> Value {
    json!({
        "schemaVersion": SERVICE_REQUEST_CONTRACT_VERSION,
        "contracts": {
            "serviceRequest": {
                "version": SERVICE_REQUEST_CONTRACT_VERSION,
                "schemaId": SERVICE_REQUEST_SCHEMA_ID,
                "schemaPath": "docs/dev/contracts/service-request.v1.schema.json",
                "http": {
                    "method": "POST",
                    "route": SERVICE_REQUEST_HTTP_ROUTE,
                },
                "mcp": {
                    "tool": SERVICE_REQUEST_MCP_TOOL_NAME,
                    "argumentsSchemaId": SERVICE_REQUEST_SCHEMA_ID,
                    "toolCallSchemaId": SERVICE_REQUEST_MCP_TOOL_CALL_SCHEMA_ID,
                },
                "actions": SERVICE_REQUEST_ACTIONS,
                "actionCount": SERVICE_REQUEST_ACTIONS.len(),
            },
            "serviceRequestMcpToolCall": {
                "version": SERVICE_REQUEST_CONTRACT_VERSION,
                "schemaId": SERVICE_REQUEST_MCP_TOOL_CALL_SCHEMA_ID,
                "schemaPath": "docs/dev/contracts/service-request-mcp-tool-call.v1.schema.json",
                "tool": SERVICE_REQUEST_MCP_TOOL_NAME,
            },
            "serviceProfileAllocationResponse": {
                "version": SERVICE_REQUEST_CONTRACT_VERSION,
                "schemaId": SERVICE_PROFILE_ALLOCATION_RESPONSE_SCHEMA_ID,
                "schemaPath": "docs/dev/contracts/service-profile-allocation-response.v1.schema.json",
                "http": {
                    "method": "GET",
                    "route": SERVICE_PROFILE_ALLOCATION_HTTP_ROUTE,
                },
            },
            "serviceProfileReadinessResponse": {
                "version": SERVICE_REQUEST_CONTRACT_VERSION,
                "schemaId": SERVICE_PROFILE_READINESS_RESPONSE_SCHEMA_ID,
                "schemaPath": "docs/dev/contracts/service-profile-readiness-response.v1.schema.json",
                "http": {
                    "method": "GET",
                    "route": SERVICE_PROFILE_READINESS_HTTP_ROUTE,
                },
                "client": {
                    "package": "@agent-browser/client/service-observability",
                    "helpers": ["getServiceProfileReadiness", "summarizeServiceProfileReadiness"],
                },
            },
            "serviceProfileLookupResponse": {
                "version": SERVICE_REQUEST_CONTRACT_VERSION,
                "schemaId": SERVICE_PROFILE_LOOKUP_RESPONSE_SCHEMA_ID,
                "schemaPath": "docs/dev/contracts/service-profile-lookup-response.v1.schema.json",
                "http": {
                    "method": "GET",
                    "route": SERVICE_PROFILE_LOOKUP_HTTP_ROUTE,
                },
                "client": {
                    "package": "@agent-browser/client/service-observability",
                    "helpers": ["lookupServiceProfile", "getServiceProfileForIdentity"],
                    "selectionOrder": ["authenticatedServiceIds", "targetServiceIds", "sharedServiceIds"],
                },
            },
            "serviceAccessPlanResponse": {
                "version": SERVICE_REQUEST_CONTRACT_VERSION,
                "schemaId": SERVICE_ACCESS_PLAN_RESPONSE_SCHEMA_ID,
                "schemaPath": "docs/dev/contracts/service-access-plan-response.v1.schema.json",
                "http": {
                    "method": "GET",
                    "route": SERVICE_ACCESS_PLAN_HTTP_ROUTE,
                },
                "mcp": {
                    "resource": SERVICE_ACCESS_PLAN_MCP_RESOURCE,
                },
                "client": {
                    "package": "@agent-browser/client/service-observability",
                    "helpers": ["getServiceAccessPlan"],
                },
            },
            "serviceMonitorRunDueResponse": {
                "version": SERVICE_REQUEST_CONTRACT_VERSION,
                "schemaId": SERVICE_MONITOR_RUN_DUE_RESPONSE_SCHEMA_ID,
                "schemaPath": "docs/dev/contracts/service-monitor-run-due-response.v1.schema.json",
                "http": {
                    "method": "POST",
                    "route": SERVICE_MONITORS_RUN_DUE_HTTP_ROUTE,
                },
                "mcp": {
                    "tool": SERVICE_MONITORS_RUN_DUE_MCP_TOOL_NAME,
                },
                "client": {
                    "package": "@agent-browser/client/service-observability",
                    "helpers": ["runDueServiceMonitors"],
                },
            },
        },
        "http": {
            "contractsRoute": SERVICE_CONTRACTS_HTTP_ROUTE,
            "serviceRequestRoute": SERVICE_REQUEST_HTTP_ROUTE,
            "serviceProfileAllocationRoute": SERVICE_PROFILE_ALLOCATION_HTTP_ROUTE,
            "serviceProfileReadinessRoute": SERVICE_PROFILE_READINESS_HTTP_ROUTE,
            "serviceProfileLookupRoute": SERVICE_PROFILE_LOOKUP_HTTP_ROUTE,
            "serviceAccessPlanRoute": SERVICE_ACCESS_PLAN_HTTP_ROUTE,
            "serviceMonitorsRunDueRoute": SERVICE_MONITORS_RUN_DUE_HTTP_ROUTE,
        },
        "mcp": {
            "contractsResource": SERVICE_CONTRACTS_RESOURCE,
            "serviceRequestTool": SERVICE_REQUEST_MCP_TOOL_NAME,
            "serviceAccessPlanResource": SERVICE_ACCESS_PLAN_MCP_RESOURCE,
            "serviceMonitorsRunDueTool": SERVICE_MONITORS_RUN_DUE_MCP_TOOL_NAME,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_request_contract_metadata_has_stable_ids() {
        let metadata = service_contracts_metadata();

        assert_eq!(metadata["schemaVersion"], SERVICE_REQUEST_CONTRACT_VERSION);
        assert_eq!(
            metadata["contracts"]["serviceRequest"]["schemaId"],
            SERVICE_REQUEST_SCHEMA_ID
        );
        assert_eq!(
            metadata["contracts"]["serviceRequest"]["http"]["route"],
            SERVICE_REQUEST_HTTP_ROUTE
        );
        assert_eq!(
            metadata["contracts"]["serviceRequest"]["mcp"]["tool"],
            SERVICE_REQUEST_MCP_TOOL_NAME
        );
        assert_eq!(
            metadata["contracts"]["serviceRequest"]["actionCount"],
            SERVICE_REQUEST_ACTIONS.len()
        );
        assert_eq!(
            metadata["contracts"]["serviceProfileAllocationResponse"]["schemaId"],
            SERVICE_PROFILE_ALLOCATION_RESPONSE_SCHEMA_ID
        );
        assert_eq!(
            metadata["contracts"]["serviceProfileAllocationResponse"]["http"]["route"],
            SERVICE_PROFILE_ALLOCATION_HTTP_ROUTE
        );
        assert_eq!(
            metadata["contracts"]["serviceProfileReadinessResponse"]["schemaId"],
            SERVICE_PROFILE_READINESS_RESPONSE_SCHEMA_ID
        );
        assert_eq!(
            metadata["contracts"]["serviceProfileReadinessResponse"]["http"]["route"],
            SERVICE_PROFILE_READINESS_HTTP_ROUTE
        );
        assert_eq!(
            metadata["contracts"]["serviceProfileReadinessResponse"]["client"]["helpers"][0],
            "getServiceProfileReadiness"
        );
        assert_eq!(
            metadata["contracts"]["serviceProfileLookupResponse"]["schemaId"],
            SERVICE_PROFILE_LOOKUP_RESPONSE_SCHEMA_ID
        );
        assert_eq!(
            metadata["contracts"]["serviceProfileLookupResponse"]["http"]["route"],
            SERVICE_PROFILE_LOOKUP_HTTP_ROUTE
        );
        assert_eq!(
            metadata["contracts"]["serviceProfileLookupResponse"]["client"]["helpers"][0],
            "lookupServiceProfile"
        );
        assert_eq!(
            metadata["contracts"]["serviceProfileLookupResponse"]["client"]["selectionOrder"][0],
            "authenticatedServiceIds"
        );
        assert_eq!(
            metadata["contracts"]["serviceAccessPlanResponse"]["schemaId"],
            SERVICE_ACCESS_PLAN_RESPONSE_SCHEMA_ID
        );
        assert_eq!(
            metadata["contracts"]["serviceAccessPlanResponse"]["http"]["route"],
            SERVICE_ACCESS_PLAN_HTTP_ROUTE
        );
        assert_eq!(
            metadata["contracts"]["serviceAccessPlanResponse"]["mcp"]["resource"],
            SERVICE_ACCESS_PLAN_MCP_RESOURCE
        );
        assert_eq!(
            metadata["contracts"]["serviceAccessPlanResponse"]["client"]["helpers"][0],
            "getServiceAccessPlan"
        );
        assert_eq!(
            metadata["contracts"]["serviceMonitorRunDueResponse"]["schemaId"],
            SERVICE_MONITOR_RUN_DUE_RESPONSE_SCHEMA_ID
        );
        assert_eq!(
            metadata["contracts"]["serviceMonitorRunDueResponse"]["http"]["route"],
            SERVICE_MONITORS_RUN_DUE_HTTP_ROUTE
        );
        assert_eq!(
            metadata["contracts"]["serviceMonitorRunDueResponse"]["mcp"]["tool"],
            SERVICE_MONITORS_RUN_DUE_MCP_TOOL_NAME
        );
        assert_eq!(
            metadata["contracts"]["serviceMonitorRunDueResponse"]["client"]["helpers"][0],
            "runDueServiceMonitors"
        );
    }
}
