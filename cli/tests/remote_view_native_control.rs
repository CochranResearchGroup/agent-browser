// The durable round helpers are tested through the production MCP/store seam.
#[path = "../src/native/browser_native_control.rs"]
mod browser_native_control;
#[allow(dead_code)]
#[path = "../src/native/browser_native_image.rs"]
mod browser_native_image;
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

struct ImagePuzzleOwner {
    fixture: NativeFixture,
}
impl browser_native_control::NativeOwner for ImagePuzzleOwner {
    fn request(
        &mut self,
        operation: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        use base64::Engine;
        use sha2::Digest;
        if operation == "observe" {
            return self.fixture.request(operation, body);
        }
        if operation == "control" {
            return Ok(
                serde_json::json!({"desktopId":"viewing","generation":9,"owner":"agent","epoch":4}),
            );
        }
        if operation == "input" {
            return self.fixture.request(operation, body);
        }
        assert_eq!(operation, "ocr");
        let mut image = image::RgbImage::from_pixel(1024, 768, image::Rgb([255, 255, 255]));
        for row in 0..3 {
            for column in 0..3 {
                let (left, top) = (100 + column * 66, 200 + row * 66);
                for y in top..top + 64 {
                    for x in left..left + 64 {
                        if x < left + 2 || x >= left + 62 || y < top + 2 || y >= top + 62 {
                            image.put_pixel(x, y, image::Rgb([20, 20, 20]));
                        }
                    }
                }
            }
        }
        for y in 414..444 {
            for x in 100..180 {
                if x < 102 || x >= 178 || y < 416 || y >= 442 {
                    image.put_pixel(x, y, image::Rgb([20, 20, 20]));
                }
            }
        }
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let bytes = encoded.into_inner();
        Ok(
            serde_json::json!({"schemaVersion":1,"desktopId":"viewing","generation":9,"windowId":7,
            "processIdentity":body["expectedProcessIdentity"],"region":body["region"],
            "image":{"mimeType":"image/png","data":base64::engine::general_purpose::STANDARD.encode(&bytes)},
            "capture":{"sha256":format!("{:x}",sha2::Sha256::digest(&bytes)),"width":1024,"height":768},
            "ocr":{"state":"observed","engine":"tesseract","modelSha256":"b".repeat(64),"lines":[
                {"text":"Select all images containing buses","confidence":95,"bounds":{"x":100,"y":160,"width":350,"height":20}},
                {"text":"hCaptcha","confidence":95,"bounds":{"x":400,"y":170,"width":80,"height":20}},
                {"text":"Verify","confidence":95,"bounds":{"x":114,"y":421,"width":50,"height":16}}
            ]}}),
        )
    }
}

#[test]
fn native_image_capture_registers_same_frame_candidates_without_input() {
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
    let mut owner = ImagePuzzleOwner {
        fixture: NativeFixture { calls: vec![] },
    };
    let result=browser_native_control::execute(&original,&target,&serde_json::json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":"hcaptcha-image-grid-3x3-v1"}),&mut owner).expect("supported static image grid must produce owner evidence");
    assert_eq!(result["challenge"]["kind"], "image_puzzle");
    assert_eq!(
        result["nativeImageObservation"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    assert_eq!(
        result["nativeImageObservation"]["frameSha256"],
        result["capture"]["capture"]["sha256"]
    );
    assert!(!owner
        .fixture
        .calls
        .iter()
        .any(|(operation, _)| operation == "input"));
    assert!(result["capture"].get("ocr").is_none());
}

#[test]
fn native_image_attempt_derives_only_current_candidate_and_verify_points() {
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
    let capture=browser_native_control::execute(&original,&target,&serde_json::json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":browser_native_image::PROFILE_ID}),&mut ImagePuzzleOwner{fixture:NativeFixture{calls:vec![]}}).unwrap();
    let observed: browser_native_image::NativeImageObservation =
        serde_json::from_value(capture["nativeImageObservation"].clone()).unwrap();
    struct Owner {
        fixture: NativeFixture,
        evidence: browser_native_image::NativeImageObservation,
        epoch: u64,
    }
    impl browser_native_control::NativeOwner for Owner {
        fn native_image_attempt(
            &mut self,
            command: &serde_json::Value,
        ) -> Result<browser_native_image::NativeImageObservation, String> {
            let selected: Vec<String> =
                serde_json::from_value(command["imageAttempt"]["selectedCandidateIds"].clone())
                    .unwrap();
            let now = self.evidence.current_time_ms()?;
            let plan = browser_native_image::plan(
                "task",
                "attempt",
                &self.evidence,
                &selected,
                &serde_json::json!({"providerId":"native-image-provider","digest":"c".repeat(64)}),
                now,
            )?;
            assert_eq!(plan.policy.max_rounds, 1);
            assert_eq!(plan.intent.planned_pointer_events, 9);
            assert_eq!(plan.snapshot.total_pointer_events, 0);
            Ok(self.evidence.clone())
        }
        fn request(
            &mut self,
            operation: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            if operation == "control" {
                return Ok(
                    serde_json::json!({"desktopId":"viewing","generation":9,"owner":"agent","epoch":self.epoch}),
                );
            }
            self.fixture.request(operation, body)
        }
    }
    let selected = vec![
        observed.candidates[0].candidate_id.clone(),
        observed.candidates[2].candidate_id.clone(),
    ];
    let mut owner = Owner {
        fixture: NativeFixture { calls: vec![] },
        evidence: observed.clone(),
        epoch: 4,
    };
    let response=browser_native_control::execute(&original,&target,&serde_json::json!({"action":"native_input","imageAttempt":{"observationRequestId":"capture","selectedCandidateIds":selected}}),&mut owner).unwrap();
    assert_eq!(response["delivery"], "dispatched");
    let inputs = owner
        .fixture
        .calls
        .iter()
        .filter(|(operation, _)| operation == "input")
        .collect::<Vec<_>>();
    assert_eq!(inputs.len(), 1);
    assert_eq!(
        inputs[0].1["actions"],
        serde_json::json!([{"kind":"click","x":132,"y":232,"button":1,"count":1},{"kind":"click","x":264,"y":232,"button":1,"count":1},{"kind":"click","x":140,"y":429,"button":1,"count":1}])
    );
    assert_eq!(
        inputs[0].1["expectedVisual"]["sha256"],
        observed.frame_sha256
    );
    assert_eq!(inputs[0].1["controlEpoch"], 4);
    for condition in [
        "changed_control",
        "changed_window",
        "expired",
        "unknown_candidate",
    ] {
        let mut evidence = observed.clone();
        if condition == "changed_window" {
            evidence.window_id += 1;
        }
        if condition == "expired" {
            evidence.clock = None;
            evidence.observed_at_ms -= 120_000;
        }
        let mut selected = selected.clone();
        if condition == "unknown_candidate" {
            selected[0] = "tile:unknown:aaaaaaaaaaaaaaaa".into();
        }
        let mut owner = Owner {
            fixture: NativeFixture { calls: vec![] },
            evidence,
            epoch: if condition == "changed_control" { 5 } else { 4 },
        };
        let result = browser_native_control::execute(
            &original,
            &target,
            &serde_json::json!({"action":"native_input","imageAttempt":{"observationRequestId":"capture","selectedCandidateIds":selected}}),
            &mut owner,
        );
        assert!(result.is_err(), "{condition}");
        assert!(
            !owner
                .fixture
                .calls
                .iter()
                .any(|(operation, _)| operation == "input"),
            "{condition}"
        );
    }
}

#[test]
fn native_image_malformed_attempt_never_falls_back_to_legacy_input() {
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
    let valid = serde_json::json!({"observationRequestId":"capture","selectedCandidateIds":["tile:1:aaaaaaaaaaaaaaaa"]});
    let mut cases=vec![serde_json::Value::Null,serde_json::json!(false),serde_json::json!("caller"),serde_json::json!({}),serde_json::json!({"observationRequestId":"capture","selectedCandidateIds":[]}),serde_json::json!({"observationRequestId":"capture","selectedCandidateIds":["one","one"]}),serde_json::json!({"observationRequestId":"capture","selectedCandidateIds":["one"],"x":99})].into_iter().map(|attempt|serde_json::json!({"action":"native_input","imageAttempt":attempt,"actions":[{"kind":"click","x":900,"y":700,"button":1,"count":1}]})).collect::<Vec<_>>();
    for (key, value) in [
        (
            "actions",
            serde_json::json!([{ "kind":"click","x":900,"y":700,"button":1,"count":1}]),
        ),
        (
            "nativeAttempt",
            serde_json::json!({"observationRequestId":"capture","registeredActionId":"hcaptcha-checkbox-p181-v2"}),
        ),
        ("controlEpoch", serde_json::json!(4)),
    ] {
        let mut command = serde_json::json!({"action":"native_input","imageAttempt":valid});
        command[key] = value;
        cases.push(command);
    }
    let bare = cases
        .iter()
        .map(|command| {
            let mut command = command.clone();
            command.as_object_mut().unwrap().remove("actions");
            command
        })
        .collect::<Vec<_>>();
    cases.extend(bare);
    for command in cases {
        let mut owner = NativeFixture { calls: vec![] };
        assert!(
            browser_native_control::execute(&original, &target, &command, &mut owner).is_err(),
            "{command}"
        );
        assert!(
            owner.calls.is_empty(),
            "Malformed/mixed image input cannot reach legacy Native transport"
        );
    }
}

#[test]
fn native_image_capture_offsets_absolute_ocr_and_local_pixels_exactly_once() {
    struct OffsetOwner(ImagePuzzleOwner);
    impl browser_native_control::NativeOwner for OffsetOwner {
        fn request(
            &mut self,
            operation: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            let mut value = self.0.request(operation, body)?;
            if operation == "observe" {
                value["geometry"]["current"] = serde_json::json!({"width":1200,"height":900});
                value["windows"]["windows"][0]["x"] = serde_json::json!(20);
                value["windows"]["windows"][0]["y"] = serde_json::json!(30);
            }
            if operation == "ocr" {
                for line in value["ocr"]["lines"].as_array_mut().unwrap() {
                    line["bounds"]["x"] =
                        serde_json::json!(line["bounds"]["x"].as_u64().unwrap() + 20);
                    line["bounds"]["y"] =
                        serde_json::json!(line["bounds"]["y"].as_u64().unwrap() + 30);
                }
            }
            Ok(value)
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
    let capture=browser_native_control::execute(&original,&target,&serde_json::json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":browser_native_image::PROFILE_ID}),&mut OffsetOwner(ImagePuzzleOwner{fixture:NativeFixture{calls:vec![]}})).unwrap();
    let observation: browser_native_image::NativeImageObservation =
        serde_json::from_value(capture["nativeImageObservation"].clone()).unwrap();
    observation.validate_current().unwrap();
    assert_eq!(observation.candidates[0].bounds.x, 120);
    assert_eq!(observation.candidates[0].bounds.y, 230);
    let actions = observation
        .actions(&[observation.candidates[0].candidate_id.clone()])
        .unwrap();
    assert_eq!(actions[0]["x"], 152);
    assert_eq!(actions[0]["y"], 262);
    assert_eq!(actions[1]["x"], 160);
    assert_eq!(actions[1]["y"], 459);
}

#[test]
fn native_image_capture_refuses_unsupported_ambiguous_partial_or_changed_pixels() {
    struct ChangedOwner {
        inner: ImagePuzzleOwner,
        case: &'static str,
    }
    impl browser_native_control::NativeOwner for ChangedOwner {
        fn request(
            &mut self,
            operation: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            use base64::Engine;
            use sha2::Digest;
            let mut value = self.inner.request(operation, body)?;
            if operation != "ocr" {
                return Ok(value);
            }
            if self.case == "unsupported" {
                value["ocr"]["lines"][0]["text"] =
                    serde_json::json!("Select all images containing cars");
            }
            if self.case == "low-score" {
                value["ocr"]["lines"][0]["confidence"] = serde_json::json!(69);
            }
            if self.case == "dynamic" {
                value["ocr"]["lines"].as_array_mut().unwrap().push(serde_json::json!({"text":"New images replace selected images","confidence":95,"bounds":{"x":100,"y":460,"width":250,"height":18}}));
            }
            if self.case == "wrong-frame" {
                value["capture"]["sha256"] = serde_json::json!("f".repeat(64));
            }
            if matches!(self.case, "duplicate-grid" | "occluded" | "transparent") {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(value["image"]["data"].as_str().unwrap())
                    .unwrap();
                let mut pixels = image::load_from_memory(&bytes).unwrap().to_rgba8();
                if self.case == "duplicate-grid" {
                    for row in 0..3 {
                        for column in 0..3 {
                            let (left, top) = (500 + column * 66, 200 + row * 66);
                            for y in top..top + 64 {
                                for x in left..left + 64 {
                                    if x < left + 2
                                        || x >= left + 62
                                        || y < top + 2
                                        || y >= top + 62
                                    {
                                        pixels.put_pixel(x, y, image::Rgba([20, 20, 20, 255]));
                                    }
                                }
                            }
                        }
                    }
                }
                if self.case == "occluded" {
                    for y in 200..202 {
                        for x in 100..164 {
                            pixels.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
                        }
                    }
                }
                if self.case == "transparent" {
                    pixels.put_pixel(100, 200, image::Rgba([20, 20, 20, 0]));
                }
                let mut encoded = std::io::Cursor::new(Vec::new());
                image::DynamicImage::ImageRgba8(pixels)
                    .write_to(&mut encoded, image::ImageFormat::Png)
                    .unwrap();
                let bytes = encoded.into_inner();
                value["image"]["data"] =
                    serde_json::json!(base64::engine::general_purpose::STANDARD.encode(&bytes));
                value["capture"]["sha256"] =
                    serde_json::json!(format!("{:x}", sha2::Sha256::digest(&bytes)));
            }
            Ok(value)
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
    for case in [
        "unsupported",
        "low-score",
        "dynamic",
        "wrong-frame",
        "duplicate-grid",
        "occluded",
        "transparent",
        "partial",
    ] {
        let mut owner = ChangedOwner {
            inner: ImagePuzzleOwner {
                fixture: NativeFixture { calls: vec![] },
            },
            case,
        };
        let mut command = serde_json::json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":browser_native_image::PROFILE_ID});
        if case == "partial" {
            command["region"] = serde_json::json!({"x":0,"y":0,"width":500,"height":480});
        }
        let result = browser_native_control::execute(&original, &target, &command, &mut owner);
        if case == "wrong-frame" {
            assert!(result.is_err());
        } else {
            assert!(
                result.unwrap().get("nativeImageObservation").is_none(),
                "{case}"
            );
        }
        assert!(
            !owner
                .inner
                .fixture
                .calls
                .iter()
                .any(|(operation, _)| operation == "input"),
            "{case}"
        );
    }
}

#[test]
fn native_image_complete_puzzle_region_is_explicit_fresh_and_fully_contained() {
    struct RegionOwner(ImagePuzzleOwner);
    impl browser_native_control::NativeOwner for RegionOwner {
        fn request(
            &mut self,
            operation: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            use base64::Engine;
            use sha2::Digest;
            let mut reply = self.0.request(operation, body)?;
            if operation == "ocr" {
                let r = &body["region"];
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(reply["image"]["data"].as_str().unwrap())
                    .unwrap();
                let image = image::load_from_memory(&bytes).unwrap().crop_imm(
                    r["x"].as_u64().unwrap() as u32,
                    r["y"].as_u64().unwrap() as u32,
                    r["width"].as_u64().unwrap() as u32,
                    r["height"].as_u64().unwrap() as u32,
                );
                let mut out = std::io::Cursor::new(Vec::new());
                image.write_to(&mut out, image::ImageFormat::Png).unwrap();
                let bytes = out.into_inner();
                reply["image"]["data"] =
                    serde_json::json!(base64::engine::general_purpose::STANDARD.encode(&bytes));
                reply["capture"] = serde_json::json!({"sha256":format!("{:x}",sha2::Sha256::digest(&bytes)),"width":r["width"],"height":r["height"]});
            }
            Ok(reply)
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
    let region = serde_json::json!({"x":80,"y":140,"width":440,"height":340});
    let command = serde_json::json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":browser_native_image::PROFILE_ID,"imageCaptureScope":"complete_puzzle_region","region":region});
    for bad in [
        serde_json::json!({"imageCaptureScope":null}),
        serde_json::json!({"imageCaptureScope":"complete_window"}),
        serde_json::json!({"imageCaptureScope":"complete_puzzle_region","action":"native_input"}),
        serde_json::json!({"imageCaptureScope":"complete_puzzle_region","registeredActionId":"hcaptcha-checkbox-p181-v2"}),
    ] {
        let mut invalid = command.clone();
        for (key, value) in bad.as_object().unwrap() {
            invalid[key] = value.clone();
        }
        let mut owner = RegionOwner(ImagePuzzleOwner {
            fixture: NativeFixture { calls: vec![] },
        });
        assert!(browser_native_control::execute(&original, &target, &invalid, &mut owner).is_err());
        assert!(owner.0.fixture.calls.is_empty());
    }
    let result = browser_native_control::execute(
        &original,
        &target,
        &command,
        &mut RegionOwner(ImagePuzzleOwner {
            fixture: NativeFixture { calls: vec![] },
        }),
    )
    .unwrap();
    assert_eq!(result["partialWindow"], true);
    assert_eq!(result["challenge"]["completeWindow"], false);
    assert_eq!(
        result["nativeImageObservation"]["captureScope"],
        "complete_puzzle_region"
    );
    assert_eq!(result["nativeImageObservation"]["region"], region);
    let obs: browser_native_image::NativeImageObservation =
        serde_json::from_value(result["nativeImageObservation"].clone()).unwrap();
    obs.validate_current().unwrap();
    struct InputOwner {
        inner: RegionOwner,
        evidence: browser_native_image::NativeImageObservation,
    }
    impl browser_native_control::NativeOwner for InputOwner {
        fn native_image_attempt(
            &mut self,
            _: &serde_json::Value,
        ) -> Result<browser_native_image::NativeImageObservation, String> {
            Ok(self.evidence.clone())
        }
        fn request(
            &mut self,
            operation: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            self.inner.request(operation, body)
        }
    }
    for case in ["current", "control", "window", "stale", "frame"] {
        let mut evidence = obs.clone();
        match case {
            "control" => evidence.native_control_epoch += 1,
            "window" => evidence.window_id += 1,
            "stale" => {
                evidence.clock = None;
                evidence.observed_at_ms -= 120_000;
            }
            "frame" => evidence.frame_sha256 = "f".repeat(64),
            _ => {}
        }
        let mut owner = InputOwner {
            inner: RegionOwner(ImagePuzzleOwner {
                fixture: NativeFixture { calls: vec![] },
            }),
            evidence,
        };
        let input = serde_json::json!({"action":"native_input","imageAttempt":{"observationRequestId":"roi-original","selectedCandidateIds":[obs.candidates[0].candidate_id]}});
        let reply = browser_native_control::execute(&original, &target, &input, &mut owner);
        let inputs = owner
            .inner
            .0
            .fixture
            .calls
            .iter()
            .filter(|(op, _)| op == "input")
            .collect::<Vec<_>>();
        if case == "current" {
            assert!(reply.is_ok());
            assert_eq!(inputs.len(), 1);
            assert_eq!(inputs[0].1["expectedVisual"]["region"], region);
            assert_eq!(inputs[0].1["expectedVisual"]["sha256"], obs.frame_sha256);
            assert_eq!(inputs[0].1["expectedWindow"], result["windowBounds"]);
            assert_eq!(inputs[0].1["actions"][0]["x"], 132);
            assert_eq!(inputs[0].1["actions"][0]["y"], 232);
        } else {
            assert!(reply.is_err(), "{case}");
            assert!(inputs.is_empty(), "{case}");
        }
    }
    for field in ["x", "width"] {
        let mut bad = result["nativeImageObservation"].clone();
        bad["region"][field] = serde_json::json!(2000);
        let obs: browser_native_image::NativeImageObservation =
            serde_json::from_value(bad).unwrap();
        assert!(obs.validate_current().is_err());
    }
    let mut legacy = result["nativeImageObservation"].clone();
    legacy.as_object_mut().unwrap().remove("captureScope");
    let obs: browser_native_image::NativeImageObservation = serde_json::from_value(legacy).unwrap();
    assert!(obs.validate_current().is_err());
    for region in [
        serde_json::json!({"x":80,"y":180,"width":440,"height":300}),
        serde_json::json!({"x":80,"y":140,"width":300,"height":340}),
        serde_json::json!({"x":80,"y":140,"width":440,"height":290}),
    ] {
        let mut command = command.clone();
        command["region"] = region;
        let result = browser_native_control::execute(
            &original,
            &target,
            &command,
            &mut RegionOwner(ImagePuzzleOwner {
                fixture: NativeFixture { calls: vec![] },
            }),
        )
        .unwrap();
        assert!(result.get("nativeImageObservation").is_none());
    }
}

#[test]
fn native_image_rounds_owner_hashes_interiors_and_vetoes_rejection_at_any_confidence() {
    struct RoundOwner {
        inner: ImagePuzzleOwner,
        case: &'static str,
    }
    impl browser_native_control::NativeOwner for RoundOwner {
        fn request(
            &mut self,
            operation: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            use base64::Engine;
            use sha2::Digest;
            let mut value = self.inner.request(operation, body)?;
            if operation != "ocr" {
                return Ok(value);
            }
            if self.case == "rejection" {
                value["ocr"]["lines"].as_array_mut().unwrap().push(serde_json::json!({"text":"Please try again","confidence":12,"bounds":{"x":100,"y":450,"width":180,"height":18}}));
            }
            if matches!(self.case, "border" | "interiors") {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(value["image"]["data"].as_str().unwrap())
                    .unwrap();
                let mut pixels = image::load_from_memory(&bytes).unwrap().to_rgb8();
                for row in 0..3 {
                    for column in 0..3 {
                        let inset = if self.case == "border" { 4 } else { 20 };
                        pixels.put_pixel(
                            100 + column * 66 + inset,
                            200 + row * 66 + inset,
                            image::Rgb([220, 220, 220]),
                        );
                    }
                }
                let mut encoded = std::io::Cursor::new(Vec::new());
                image::DynamicImage::ImageRgb8(pixels)
                    .write_to(&mut encoded, image::ImageFormat::Png)
                    .unwrap();
                let bytes = encoded.into_inner();
                value["image"]["data"] =
                    serde_json::json!(base64::engine::general_purpose::STANDARD.encode(&bytes));
                value["capture"]["sha256"] =
                    serde_json::json!(format!("{:x}", sha2::Sha256::digest(&bytes)));
            }
            Ok(value)
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
    let capture = |case, profile| {
        browser_native_control::execute(&original,&target,&serde_json::json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":profile}),&mut RoundOwner{inner:ImagePuzzleOwner{fixture:NativeFixture{calls:vec![]}},case}).unwrap()
    };
    let baseline = capture("baseline", browser_native_image::ROUNDS_PROFILE_ID);
    let border = capture("border", browser_native_image::ROUNDS_PROFILE_ID);
    let interiors = capture("interiors", browser_native_image::ROUNDS_PROFILE_ID);
    let hashes = baseline["nativeImageObservation"]["tileImageSha256s"]
        .as_array()
        .unwrap();
    assert_eq!(hashes.len(), 9);
    assert_eq!(
        baseline["nativeImageObservation"]["tileImageSha256s"],
        border["nativeImageObservation"]["tileImageSha256s"]
    );
    assert!(hashes
        .iter()
        .zip(
            interiors["nativeImageObservation"]["tileImageSha256s"]
                .as_array()
                .unwrap()
        )
        .all(|(a, b)| a != b));
    let first: browser_native_image::NativeImageObservation =
        serde_json::from_value(baseline["nativeImageObservation"].clone()).unwrap();
    let mut fresh: browser_native_image::NativeImageObservation =
        serde_json::from_value(interiors["nativeImageObservation"].clone()).unwrap();
    fresh.observed_at_ms = if fresh.clock.is_some() {
        first.observed_at_ms.saturating_sub(3_000)
    } else {
        first.observed_at_ms + 1
    };
    let binding =
        serde_json::json!({"digest":"d".repeat(64),"providerId":"explicit-two-round-provider"});
    let selected = vec![first.candidates[0].candidate_id.clone()];
    let plan = browser_native_image::plan(
        "task",
        "original",
        &first,
        &selected,
        &binding,
        first.current_time_ms().unwrap(),
    )
    .unwrap();
    let (_, finished) = browser_native_image::finish_round(
        &browser_native_image::retain_plan(&plan).unwrap(),
        true,
        "original",
    )
    .unwrap();
    let selected = vec![fresh.candidates[0].candidate_id.clone()];
    let continued = browser_native_image::plan_following(
        "task",
        "original",
        &fresh,
        &selected,
        &binding,
        fresh.current_time_ms().unwrap(),
        (&finished, &first),
    )
    .unwrap();
    assert_eq!(continued.policy.deadline_at_ms, plan.policy.deadline_at_ms);
    assert_eq!(continued.snapshot.attempt_id, "original");
    assert!(browser_native_image::plan_following(
        "task",
        "original",
        &fresh,
        &selected,
        &binding,
        plan.policy.deadline_at_ms,
        (&finished, &first)
    )
    .is_err());
    assert!(
        capture("rejection", browser_native_image::ROUNDS_PROFILE_ID)
            .get("nativeImageObservation")
            .is_none()
    );
    assert!(
        capture("rejection", browser_native_image::PROFILE_ID)["nativeImageObservation"]
            .get("tileImageSha256s")
            .is_none()
    );
}

#[test]
fn native_image_capture_completes_after_wall_clock_steps_back() {
    use browser_native_image::{
        NativeImageClockReading, NativeImageElapsedClock, NativeImageObservation,
    };
    struct ClockOwner(ImagePuzzleOwner);
    impl browser_native_control::NativeOwner for ClockOwner {
        fn image_observation_clock(&mut self) -> Result<NativeImageClockReading, String> {
            Ok(NativeImageClockReading {
                wall_time_ms: 1_000_000,
                elapsed: Some(NativeImageElapsedClock {
                    kind: "linux-boottime-v1".into(),
                    boot_id: "11111111-1111-4111-8111-111111111111".into(),
                    observed_at_ms: 200_000,
                }),
            })
        }
        fn request(
            &mut self,
            operation: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            self.0.request(operation, body)
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
    let mut owner = ClockOwner(ImagePuzzleOwner {
        fixture: NativeFixture { calls: vec![] },
    });
    let result = browser_native_control::execute(&original, &target, &serde_json::json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":browser_native_image::PROFILE_ID}), &mut owner).unwrap();
    let observation: NativeImageObservation =
        serde_json::from_value(result["nativeImageObservation"].clone()).unwrap();
    assert_eq!(observation.observed_at_ms, 1_000_000);
    let completion_clock = NativeImageClockReading {
        wall_time_ms: 997_000,
        elapsed: Some(NativeImageElapsedClock {
            kind: "linux-boottime-v1".into(),
            boot_id: "11111111-1111-4111-8111-111111111111".into(),
            observed_at_ms: 200_010,
        }),
    };
    observation
        .validate_at(&completion_clock)
        .expect("Same-boot capture aged10ms must complete despite a3second wall-clock correction");
    let mut forward = completion_clock.clone();
    forward.wall_time_ms = 1_120_000;
    observation
        .validate_at(&forward)
        .expect("A forward wall correction cannot expire10ms elapsed evidence");
    for case in ["expired", "future", "boot", "domain", "missing"] {
        let mut clock = completion_clock.clone();
        match case {
            "expired" => clock.elapsed.as_mut().unwrap().observed_at_ms = 260_001,
            "future" => clock.elapsed.as_mut().unwrap().observed_at_ms = 199_999,
            "boot" => {
                clock.elapsed.as_mut().unwrap().boot_id =
                    "22222222-2222-4222-8222-222222222222".into()
            }
            "domain" => clock.elapsed.as_mut().unwrap().kind = "unknown".into(),
            "missing" => clock.elapsed = None,
            _ => unreachable!(),
        }
        assert!(
            observation.validate_at(&clock).is_err(),
            "{case} must never downgrade to wall time"
        );
    }
    let mut legacy = observation.clone();
    legacy.clock = None;
    assert!(
        legacy.validate_at(&completion_clock).is_err(),
        "Legacy wall-time future evidence remains refused"
    );

    assert!(!owner
        .0
        .fixture
        .calls
        .iter()
        .any(|(operation, _)| operation == "input"));
}
