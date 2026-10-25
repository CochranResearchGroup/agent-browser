#[path = "../src/native/browser_native_control.rs"]
mod browser_native_control;
#[path = "../src/native/remote_view_application_http.rs"]
mod remote_view_application_http;
#[path = "../src/test_utils.rs"]
mod test_utils;

struct NativeFixture {
    calls: Vec<(String, serde_json::Value)>,
}
impl browser_native_control::NativeOwner for NativeFixture {
    fn request(
        &mut self,
        operation: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.calls.push((operation.into(), body.clone()));
        Ok(if operation == "observe" {
            serde_json::json!({"desktopId":"viewing","generation":9,"geometry":{"current":{"width":1024,"height":768}},"windows":{"windows":[{"id":7,"pid":42,"x":0,"y":0,"width":1024,"height":768,"active":true,"processIdentity":{"pid":42,"boot_id":"boot","start_token":"11","executable":"/synthetic/chrome","uid":1000,"cgroup":"fixture"}}]}})
        } else {
            serde_json::json!({"delivery":"dispatched","taskCompleted":false})
        })
    }
}
#[test]
fn native_task_input_joins_original_process_and_current_viewing_generation() {
    use agent_browser_service_model::{RecordedProcessIdentity, RemoteViewApplicationTarget};
    let original = RecordedProcessIdentity {
        pid: 42,
        start_token: "linux:boot:11".into(),
        executable_path: Some("/synthetic/chrome".into()),
        browser_family: Some("chrome".into()),
    };
    let target = RemoteViewApplicationTarget {
        assignment_id: "assignment".into(),
        registration_id: "registration".into(),
        desktop_id: "lifecycle".into(),
        lifecycle_generation: 2,
        viewing_desktop_id: "viewing".into(),
        viewing_generation: 9,
    };
    let mut owner = NativeFixture { calls: vec![] };
    let result = browser_native_control::execute(
        &original,
        &target,
        &serde_json::json!({"action":"native_input","actions":[{"kind":"key","chord":"Tab"}]}),
        &mut owner,
    )
    .unwrap();
    assert_eq!(result["taskCompleted"], false);
    assert_eq!(owner.calls.len(), 2);
    assert_eq!(owner.calls[1].0, "input");
    assert_eq!(
        owner.calls[1].1["expectedGeneration"], 9,
        "viewing generation, not lifecycle generation"
    );
    assert_eq!(owner.calls[1].1["windowId"], 7);
    assert_eq!(
        owner.calls[1].1["expectedProcessIdentity"]["start_token"],
        "11"
    );
    assert_eq!(owner.calls[1].1["expectedWidth"], 1024);
    assert_eq!(owner.calls[1].1["actions"][0]["chord"], "Tab");
}

#[test]
fn native_task_input_never_dispatches_to_a_replaced_process() {
    use agent_browser_service_model::{RecordedProcessIdentity, RemoteViewApplicationTarget};
    let original = RecordedProcessIdentity {
        pid: 42,
        start_token: "linux:boot:replaced".into(),
        executable_path: Some("/synthetic/chrome".into()),
        browser_family: Some("chrome".into()),
    };
    let target = RemoteViewApplicationTarget {
        assignment_id: "assignment".into(),
        registration_id: "registration".into(),
        desktop_id: "lifecycle".into(),
        lifecycle_generation: 2,
        viewing_desktop_id: "viewing".into(),
        viewing_generation: 9,
    };
    let mut owner = NativeFixture { calls: vec![] };
    let result = browser_native_control::execute(
        &original,
        &target,
        &serde_json::json!({"action":"native_input","actions":[{"kind":"key","chord":"Return"}]}),
        &mut owner,
    );
    assert!(result.unwrap_err().contains("window_unverified"));
    assert_eq!(
        owner.calls.len(),
        1,
        "process mismatch must stop before input dispatch"
    );
    assert_eq!(owner.calls[0].0, "observe");
}

#[test]
fn native_task_capture_is_fenced_to_the_browser_window_without_input() {
    let original = agent_browser_service_model::RecordedProcessIdentity {
        pid: 42,
        start_token: "linux:boot:11".into(),
        executable_path: Some("/synthetic/chrome".into()),
        browser_family: Some("chrome".into()),
    };
    let target = agent_browser_service_model::RemoteViewApplicationTarget {
        assignment_id: "assignment".into(),
        registration_id: "registration".into(),
        desktop_id: "lifecycle".into(),
        lifecycle_generation: 2,
        viewing_desktop_id: "viewing".into(),
        viewing_generation: 9,
    };
    let mut owner = NativeFixture { calls: vec![] };
    let reply = browser_native_control::execute(
        &original,
        &target,
        &serde_json::json!({"action":"native_capture"}),
        &mut owner,
    )
    .unwrap();
    assert_eq!(reply["source"], "remote_view_native");
    assert_eq!(owner.calls[1].0, "capture_region");
    assert_eq!(owner.calls[1].1["windowId"], 7);
    assert!(owner.calls[1].1.get("actions").is_none());
    assert_eq!(
        owner.calls[1].1["region"],
        serde_json::json!({"x":0,"y":0,"width":1024,"height":768})
    );
}

#[test]
fn native_task_window_remediation_uses_only_the_verified_window() {
    let original = agent_browser_service_model::RecordedProcessIdentity {
        pid: 42,
        start_token: "linux:boot:11".into(),
        executable_path: Some("/synthetic/chrome".into()),
        browser_family: Some("chrome".into()),
    };
    let target = agent_browser_service_model::RemoteViewApplicationTarget {
        assignment_id: "assignment".into(),
        registration_id: "registration".into(),
        desktop_id: "lifecycle".into(),
        lifecycle_generation: 2,
        viewing_desktop_id: "viewing".into(),
        viewing_generation: 9,
    };
    for (action, expected) in [("native_close", "close"), ("native_focus", "activate")] {
        let mut owner = NativeFixture { calls: vec![] };
        let result = browser_native_control::execute(
            &original,
            &target,
            &serde_json::json!({"action":action}),
            &mut owner,
        )
        .unwrap();
        assert_eq!(result["taskCompleted"], false);
        if action == "native_close" {
            assert_eq!(
                owner.calls[1].0, "input",
                "graceful close must never destroy the native window"
            );
            assert_eq!(
                owner.calls[1].1["actions"],
                serde_json::json!([{"kind":"key","chord":"alt+F4"}])
            );
            assert_eq!(
                owner.calls[1].1["expectedProcessIdentity"]["start_token"],
                "11"
            );
        } else {
            assert_eq!(owner.calls[1].0, "window");
            assert_eq!(
                owner.calls[1].1,
                serde_json::json!({"desktopId":"viewing","expectedGeneration":9,"windowId":7,"expectedProcessIdentity":{"pid":42,"boot_id":"boot","start_token":"11","executable":"/synthetic/chrome","uid":1000,"cgroup":"fixture"},"action":expected})
            );
        }
        assert!(
            result.get("processExited").is_none(),
            "dispatch does not prove exit"
        );
    }
}
