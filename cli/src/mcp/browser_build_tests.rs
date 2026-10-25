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
