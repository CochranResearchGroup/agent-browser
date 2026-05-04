use serde_json::{json, Value};

pub const SERVICE_CONTRACTS_RESOURCE: &str = "agent-browser://contracts";
pub const SERVICE_CONTRACTS_HTTP_ROUTE: &str = "/api/service/contracts";
pub const SERVICE_REQUEST_HTTP_ROUTE: &str = "/api/service/request";
pub const SERVICE_PROFILE_ALLOCATION_HTTP_ROUTE: &str = "/api/service/profiles/<id>/allocation";
pub const SERVICE_PROFILE_READINESS_HTTP_ROUTE: &str = "/api/service/profiles/<id>/readiness";
pub const SERVICE_REQUEST_MCP_TOOL_NAME: &str = "service_request";
pub const SERVICE_REQUEST_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-request.v1.schema.json";
pub const SERVICE_REQUEST_MCP_TOOL_CALL_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-request-mcp-tool-call.v1.schema.json";
pub const SERVICE_PROFILE_ALLOCATION_RESPONSE_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-profile-allocation-response.v1.schema.json";
pub const SERVICE_PROFILE_READINESS_RESPONSE_SCHEMA_ID: &str =
    "https://agent-browser.local/contracts/service-profile-readiness-response.v1.schema.json";
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
            },
        },
        "http": {
            "contractsRoute": SERVICE_CONTRACTS_HTTP_ROUTE,
            "serviceRequestRoute": SERVICE_REQUEST_HTTP_ROUTE,
            "serviceProfileAllocationRoute": SERVICE_PROFILE_ALLOCATION_HTTP_ROUTE,
            "serviceProfileReadinessRoute": SERVICE_PROFILE_READINESS_HTTP_ROUTE,
        },
        "mcp": {
            "contractsResource": SERVICE_CONTRACTS_RESOURCE,
            "serviceRequestTool": SERVICE_REQUEST_MCP_TOOL_NAME,
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
    }
}
