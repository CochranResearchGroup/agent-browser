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

#[test]
fn native_capture_classifies_same_fenced_frame_without_projecting_private_ocr_text() {
    use base64::Engine;
    use sha2::Digest;
    struct OcrOwner(NativeFixture);
    impl browser_native_control::NativeOwner for OcrOwner {
        fn request(
            &mut self,
            operation: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            if operation == "observe" {
                return self.0.request(operation, body);
            }
            if operation == "control" {
                assert_eq!(body["operation"], "status");
                return Ok(
                    serde_json::json!({"desktopId":"viewing","generation":9,"owner":"agent","epoch":4}),
                );
            }
            assert_eq!(
                body["controlEpoch"], 4,
                "OCR must bind current Native control admission"
            );
            let mut encoded = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgb8(1024, 768)
                .write_to(&mut encoded, image::ImageFormat::Png)
                .unwrap();
            let bytes = encoded.into_inner();
            Ok(serde_json::json!({
                "schemaVersion":1,"desktopId":"viewing","generation":9,"windowId":7,
                "processIdentity":body["expectedProcessIdentity"],
                "image":{"mimeType":"image/png","data":base64::engine::general_purpose::STANDARD.encode(&bytes)},
                "capture":{"sha256":format!("{:x}",sha2::Sha256::digest(&bytes)),"width":1024,"height":768},
                "ocr":{"state":"observed","engine":"tesseract","engineVersion":"fixture","modelSha256":"b".repeat(64),
                    "lines":[{"text":"hCaptcha","confidence":95},{"text":"I am human","confidence":95},
                        {"text":"private unrelated body","confidence":95}]}
            }))
        }
    }
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
    let result = browser_native_control::execute(
        &original,
        &target,
        &serde_json::json!({"action":"native_capture","classifyChallenges":true}),
        &mut OcrOwner(NativeFixture { calls: vec![] }),
    )
    .unwrap();
    assert_eq!(result["challenge"]["kind"], "checkbox");
    assert_eq!(
        result["challenge"]["registeredProfileId"],
        "hcaptcha-checkbox-p181-v2"
    );
    assert_eq!(
        result["challenge"]["frameSha256"],
        result["capture"]["capture"]["sha256"]
    );
    assert!(result["capture"].get("ocr").is_none());
    assert!(!result.to_string().contains("private unrelated body"));
    assert_eq!(result["taskCompleted"], false);
}

fn challenge_observation() -> browser_native_control::NativeChallengeObservation {
    use browser_native_control::{NativeChallengeBounds, NativeChallengeObservation};
    NativeChallengeObservation {
        desktop_id: "viewing".into(),
        generation: 9,
        window_id: 7,
        window: NativeChallengeBounds {
            x: 0,
            y: 0,
            width: 1024,
            height: 768,
        },
        display_width: 1024,
        display_height: 768,
        region: NativeChallengeBounds {
            x: 0,
            y: 0,
            width: 1024,
            height: 768,
        },
        frame_sha256: "a".repeat(64),
        native_control_epoch: 4,
        observed_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        registered_action_id: "hcaptcha-checkbox-p181-v2".into(),
        family: "hcaptcha".into(),
        candidate_sha256: "b".repeat(64),
        x: 12,
        y: 13,
    }
}

#[test]
fn native_challenge_attempt_requires_original_owner_evidence_not_caller_points() {
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
    let result = browser_native_control::execute(
        &original,
        &target,
        &serde_json::json!({"action":"native_input","nativeAttempt":{"observationRequestId":"forged"},"actions":[{"kind":"click","x":900,"y":700,"button":1,"count":1}]}),
        &mut owner,
    );
    assert!(result
        .unwrap_err()
        .contains("original_operation_unavailable"));
    assert!(
        owner.calls.is_empty(),
        "Caller labels cannot reach an observation or input without original owner evidence"
    );
}

#[test]
fn native_challenge_attempt_binds_original_visual_epoch_window_and_single_derived_click() {
    struct AttemptOwner {
        fixture: NativeFixture,
        evidence: browser_native_control::NativeChallengeObservation,
        epoch: u64,
    }
    impl browser_native_control::NativeOwner for AttemptOwner {
        fn native_attempt(
            &mut self,
            _: &serde_json::Value,
        ) -> Result<browser_native_control::NativeChallengeObservation, String> {
            Ok(self.evidence.clone())
        }
        fn request(
            &mut self,
            operation: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            if operation == "control" {
                self.fixture.calls.push((operation.into(), body.clone()));
                return Ok(
                    serde_json::json!({"desktopId":"viewing","generation":9,"owner":"agent","epoch":self.epoch}),
                );
            }
            self.fixture.request(operation, body)
        }
    }
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
    let command = serde_json::json!({"action":"native_input","nativeAttempt":{"observationRequestId":"original"},"controlEpoch":999,"actions":[{"kind":"click","x":900,"y":700,"button":3,"count":2}]});
    for condition in ["current", "changed_control", "changed_window", "expired"] {
        let mut evidence = challenge_observation();
        if condition == "changed_window" {
            evidence.window_id = 99;
        }
        if condition == "expired" {
            evidence.observed_at_ms -= 10_001;
        }
        let mut owner = AttemptOwner {
            fixture: NativeFixture { calls: vec![] },
            evidence,
            epoch: if condition == "changed_control" { 5 } else { 4 },
        };
        let result = browser_native_control::execute(&original, &target, &command, &mut owner);
        let inputs: Vec<_> = owner
            .fixture
            .calls
            .iter()
            .filter(|(operation, _)| operation == "input")
            .collect();
        if condition == "current" {
            assert_eq!(result.unwrap()["delivery"], "dispatched");
            assert_eq!(inputs.len(), 1);
            assert_eq!(
                inputs[0].1["actions"],
                serde_json::json!([{"kind":"click","x":12,"y":13,"button":1,"count":1}])
            );
            assert_eq!(inputs[0].1["controlEpoch"], 4);
            assert_eq!(
                inputs[0].1["expectedVisual"],
                serde_json::json!({"region":{"x":0,"y":0,"width":1024,"height":768},"sha256":"a".repeat(64)})
            );
        } else {
            assert!(result.is_err(), "{condition}");
            assert!(inputs.is_empty(), "{condition}");
        }
    }
}
