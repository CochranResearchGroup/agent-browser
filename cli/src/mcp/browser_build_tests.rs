use super::*;

#[test]
fn build_selection_profile_dispatch_does_not_require_an_unrelated_task() {
    let request = json!({"name":"browser_command","arguments":{"action":"browser_build","params":{"choice":"inspect"}}});
    let error =
        call_service_mcp_tool(Some(&request), "default", &ServiceState::default()).unwrap_err();
    let detail = native_request_error(error);
    assert!(
        !detail.contains("taskId"),
        "Profile discovery cannot require a task: {}",
        detail
    );
    assert!(
        detail.contains("profileId"),
        "Missing profile is validated before any transport effect: {}",
        detail
    );
}

#[test]
fn build_selection_cli_validates_json_before_any_owner_transport() {
    let args = vec!["mcp".into(), "browser-build".into(), "{".into()];
    let error = mcp_command_response(&args).unwrap_err();
    assert_eq!(error, "browser_build_request_invalid");
}

#[test]
fn build_selection_profile_parameter_reaches_profile_adapter_before_transport() {
    let request = json!({"name":"browser_command","arguments":{"action":"browser_build","params":{"profileId":"reviewed-profile","choice":"select_for_next_launch"}}});
    let error =
        call_service_mcp_tool(Some(&request), "default", &ServiceState::default()).unwrap_err();
    let detail = native_request_error(error);
    assert!(detail.contains("operationId"), "The profile adapter must validate its operation before transport, rather than reject its own profile parameter: {detail}");
}

#[test]
fn build_selection_open_metadata_reaches_adapter_before_transport() {
    let request = json!({"name":"browser_command","arguments":{"action":"browser_build","params":{"profileId":"reviewed-profile","choice":"open_selected","serviceName":"fixture","agentName":"fixture","taskName":"Reviewed build"}}});
    let error =
        call_service_mcp_tool(Some(&request), "default", &ServiceState::default()).unwrap_err();
    assert!(native_request_error(error).contains("operationId"));
}

#[test]
fn build_selection_does_not_relax_transport_or_page_profile_reservations() {
    for field in ["sessionName", "browserId", "requestPrincipalSource"] {
        let mut request = json!({"action":"browser_build","params":{"profileId":"reviewed-profile","choice":"open_selected"}});
        request["params"][field] = json!("untrusted");
        assert!(
            native_request_error(validate_browser_command_action(&request).unwrap_err())
                .contains("reserved")
        );
    }
    let page = json!({"action":"reload","params":{"profileId":"untrusted"}});
    assert!(
        native_request_error(validate_browser_command_action(&page).unwrap_err())
            .contains("reserved")
    );
}

#[test]
fn build_selection_discovery_retains_only_its_declared_profile_context() {
    let build = catalog::browser_argument_schema("browser_build").unwrap();
    for field in ["profileId", "serviceName", "agentName", "taskName"] {
        assert!(
            build["properties"].get(field).is_some(),
            "Missing build context: {field}"
        );
    }
    for field in ["sessionName", "browserId", "requestPrincipalSource"] {
        assert!(build["properties"].get(field).is_none());
    }
    let page = catalog::browser_argument_schema("reload").unwrap();
    assert!(page["properties"].get("profileId").is_none());
}
