use super::*;

#[test]
fn native_owner_allowance_uses_private_service_admission_only_for_input() {
    use crate::connection::stable_time::{fixture_stamp, Fixture};
    let stamp = fixture_stamp(123_000);
    let _time = Fixture::set(Ok(stamp.clone()));
    task_operations::transport(
        json!({"action":"native_input","ownerTimeoutMs":1000,
        "_nativeOwnerAdmittedServiceTime":{"forged":true},"_nativeOwnerAdmittedAtMs":0}),
        |command| {
            assert_eq!(
                command["_nativeOwnerAdmittedServiceTime"],
                serde_json::to_value(stamp).unwrap()
            );
            assert!(command.get("_nativeOwnerAdmittedAtMs").is_none());
            Ok(crate::connection::Response {
                success: true,
                ..Default::default()
            })
        },
    )
    .unwrap();
    let _outage = Fixture::set(Err("stable_time_client_start_failed".into()));
    task_operations::transport(
        json!({"action":"native_capture","ownerTimeoutMs":1000}),
        |command| {
            assert!(command.get("_nativeOwnerAdmittedServiceTime").is_none());
            Ok(crate::connection::Response {
                success: true,
                ..Default::default()
            })
        },
    )
    .unwrap();
    assert!(task_operations::transport(
        json!({"action":"native_input","ownerTimeoutMs":1000}),
        |_| { panic!("Unqualified input allowance cannot dispatch") }
    )
    .unwrap_err()
    .contains("stable_time_client_start_failed"));
}

#[test]
fn public_original_task_operation_readback_does_not_start_transport() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use agent_browser_service_model::BrowserSessionState;
    let directory =
        std::env::temp_dir().join(format!("task-operation-public-{}", uuid::Uuid::new_v4()));
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_SOCKET_DIR",
    ]);
    guard.set("HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    guard.set(
        "AGENT_BROWSER_SOCKET_DIR",
        directory.join("never-started").to_str().unwrap(),
    );
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let mut value = serde_json::to_value(BrowserSessionState::default()).unwrap();
    value["taskOperations"] = json!({"session:alice":{"lost-fill":{
        "requestId":"lost-fill","taskId":"session:alice","profileId":"profile","browserId":"browser",
        "tabId":"tab","targetId":"target","processPid":42,"controlEpoch":3,"action":"fill",
        "fingerprint":"private-command-mac","state":"pending","createdAtMs":10,
        "completedAtMs":null,"evidenceReferences":{}
    }}});
    value["sessions"] = json!({"session:alice":{"id":"session:alice","name":"alice",
        "profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,
        "expiresAtMs":999999,"currentTabId":null}});
    let state: BrowserSessionState = serde_json::from_value(value).unwrap();
    store.save_session_state(&state).unwrap();
    let request = json!({"jsonrpc":"2.0","id":468,"method":"tools/call",
        "params":{"name":"browser_command","arguments":{"taskId":"session:alice",
            "action":"task_operation","params":{"requestId":"lost-fill"}}}});
    let response = handle_jsonrpc_line(&request.to_string(), "unused").unwrap();
    assert!(response.get("error").is_none(), "{response}");
    let payload: Value =
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload["data"]["operation"]["requestId"], "lost-fill");
    assert_eq!(payload["data"]["operation"]["state"], "pending");
    assert_eq!(payload["data"]["operation"]["retryAllowed"], false);
    assert!(!response.to_string().contains("private-command-mac"));
    assert!(!directory.join("never-started").exists());
    assert_eq!(store.load_session_state().unwrap(), state);
    let key_path = BrowserSessionSqliteStore::default_sqlite_path()
        .unwrap()
        .parent()
        .unwrap()
        .join("task-operation-mac.key");
    assert!(!key_path.exists());
    let same_original = json!({"jsonrpc":"2.0","id":470,"method":"tools/call",
        "params":{"name":"browser_fill","arguments":{"taskId":"session:alice",
        "requestId":"lost-fill","selector":"#draft","value":"never replay"}}});
    let missing_key = handle_jsonrpc_line(&same_original.to_string(), "unused").unwrap();
    assert!(missing_key
        .to_string()
        .contains("browser_task_operation_key_unavailable"));
    assert!(
        !key_path.exists(),
        "Original history must never cause key regeneration"
    );
    assert_eq!(store.load_session_state().unwrap(), state);
    let invalid = json!({"jsonrpc":"2.0","id":469,"method":"tools/call",
        "params":{"name":"browser_fill","arguments":{"taskId":"session:alice",
        "requestId":"invalid identifier", "selector":"#draft", "value":"must not be persisted"}}});
    let invalid_response = handle_jsonrpc_line(&invalid.to_string(), "unused").unwrap();
    assert!(invalid_response.get("error").is_some());
    assert_eq!(store.load_session_state().unwrap(), state);
    assert!(!directory.join("never-started").exists());
    let unscoped = json!({"jsonrpc":"2.0","id":471,"method":"tools/call",
        "params":{"name":"browser_fill","arguments":{"requestId":"existing legacy identifier",
        "value":"not sent because selector is missing"}}});
    let legacy_validation = handle_jsonrpc_line(&unscoped.to_string(), "unused").unwrap();
    assert!(
        legacy_validation.to_string().contains("taskId"),
        "{legacy_validation}"
    );
    assert!(!legacy_validation.to_string().contains("requestId must"));
    let recovered = handle_jsonrpc_line(&request.to_string(), "unused").unwrap();
    assert!(
        recovered.get("error").is_none(),
        "Failed ingress must not poison subsequent readback"
    );
    drop(store);
    fn unseal_owned_directories(path: &std::path::Path) {
        if !std::fs::symlink_metadata(path).unwrap().is_dir() {
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                unseal_owned_directories(&entry.path());
            }
        }
    }
    unseal_owned_directories(&directory);
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn native_capture_completion_failure_keeps_pending_original_without_replay() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use agent_browser_service_model::BrowserSessionState;
    struct OwnedProcess(std::process::Child);
    impl Drop for OwnedProcess {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let directory =
        std::env::temp_dir().join(format!("native-completion-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let chrome = directory.join("chrome");
    std::fs::copy("/bin/sleep", &chrome).unwrap();
    let child = OwnedProcess(
        std::process::Command::new(&chrome)
            .arg("60")
            .spawn()
            .unwrap(),
    );
    let crate::process_identity::ProcessObservation::Observed(process) =
        crate::process_identity::observe_process(child.0.id())
    else {
        panic!("Owned fixture process unavailable")
    };
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
    ]);
    guard.set("HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    let mut value = serde_json::to_value(BrowserSessionState::default()).unwrap();
    value["sessions"] = json!({"task":{"id":"task","name":"capture-lane","profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":0,"currentTabId":"tab"}});
    value["browsers"] = json!({"browser":{"id":"browser","profileId":"profile","pid":child.0.id(),"cdpEndpoint":"","desktop":null,"activeSessionIds":["task"]}});
    value["tabs"] = json!({"tab":{"id":"tab","targetId":"target","browserId":"browser","sessionId":"task","createdAtMs":1,"lastActivityAtMs":1}});
    value["sharedBrowserIds"] = json!(["browser"]);
    value["adoptedBrowserProcessIdentities"] = json!({"browser":{"profileId":"profile","cdpEndpoint":"","process":{"pid":process.pid,"startToken":process.start_token,"executablePath":process.executable_path,"browserFamily":process.browser_family}}});
    let state: BrowserSessionState = serde_json::from_value(value).unwrap();
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    store.save_session_state(&state).unwrap();
    let params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_capture","requestId":"original-capture","params":{"classifyChallenges":true,"registeredActionId":"hcaptcha-checkbox-p181-v2"}}});
    let command = json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":"hcaptcha-checkbox-p181-v2"});
    let mut dispatched = 0;
    let mut failure = None;
    task_operations::with_context(Some(&params), || {
        failure = Some(
            task_operations::transport(command.clone(), |_| {
                dispatched += 1;
                Ok(crate::connection::Response {
                    success: true,
                    data: Some(json!({"nativeObservation":{"private":"canary"}})),
                    ..Default::default()
                })
            })
            .err()
            .expect("Invalid observation must fail completion"),
        );
        Ok(Value::Null)
    })
    .unwrap();
    let failure = failure.unwrap();
    assert!(
        failure.contains("cause=browser_native_original_observation_invalid"),
        "{failure}"
    );
    assert!(failure.contains("original-capture:inspect original; no retry"));
    assert!(!failure.contains("canary"));
    let lost_params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_capture","requestId":"lost-transport-capture","params":{"classifyChallenges":true,"registeredActionId":"hcaptcha-checkbox-p181-v2"}}});
    task_operations::with_context(Some(&lost_params), || {
        let failure = task_operations::transport(command.clone(), |_| {
            Err("Transport closed: https://private.invalid/?token=never-publish".into())
        })
        .err()
        .expect("lost acknowledgement must remain uncertain");
        assert!(failure.contains("cause=transport_closed"));
        assert!(failure.contains("lost-transport-capture"));
        assert!(!failure.contains("never-publish"));
        let original = task_operations::lookup("task", "lost-transport-capture").unwrap();
        assert_eq!(original["operation"]["state"], "uncertain");
        assert_eq!(original["operation"]["retryAllowed"], false);
        let replay = task_operations::transport(command.clone(), |_| {
            panic!("lost acknowledgement must never redispatch")
        })
        .unwrap();
        assert_eq!(replay.data.unwrap()["dispatchAttempted"], false);
        Ok(Value::Null)
    })
    .unwrap();
    let readback = task_operations::lookup("task", "original-capture").unwrap();
    assert_eq!(readback["operation"]["state"], "pending");
    assert_eq!(readback["operation"]["retryAllowed"], false);
    task_operations::with_context(Some(&params), || {
        let replay = task_operations::transport(command, |_| {
            dispatched += 1;
            panic!("Original capture replayed")
        })
        .unwrap();
        assert!(!replay.success);
        assert_eq!(replay.data.unwrap()["dispatchAttempted"], false);
        Ok(Value::Null)
    })
    .unwrap();
    assert_eq!(dispatched, 1);
}

#[test]
fn native_completion_failure_causes_never_expose_unreviewed_details() {
    for (raw, expected) in [
        (
            "browser_native_original_observation_invalid:private-canary",
            "browser_native_original_observation_invalid",
        ),
        (
            "browser_native_original_observation_expired:human assistance required",
            "browser_native_original_observation_expired",
        ),
        (
            "browser_task_operation_original_missing",
            "browser_task_operation_original_missing",
        ),
        (
            "browser_task_operation_completion_conflict",
            "browser_task_operation_completion_conflict",
        ),
        (
            "browser_session_mutation_begin_failed:SELECT private-canary FROM /private/path",
            "store_unavailable",
        ),
        (
            "browser_runtime_document_invalid:browser_session_state:private-canary",
            "store_unavailable",
        ),
        (
            "terminal_history_archive_write_failed:private-canary",
            "store_unavailable",
        ),
        (
            "private-canary:browser_native_original_observation_invalid",
            "unclassified",
        ),
        (
            "browser_native_original_observation_invalid_private-canary",
            "unclassified",
        ),
        (
            "browser_runtime_database_unreviewed_private-canary",
            "unclassified",
        ),
    ] {
        let cause = task_operations::completion_failure_cause(raw);
        assert_eq!(cause, expected);
        assert!(!cause.contains("private-canary"));
        assert!(!cause.contains("SELECT"));
        assert!(!cause.contains('/'));
    }
}

#[test]
fn native_image_public_original_reserves_one_taskwide_attempt_and_never_replays() {
    native_image_public_completion_case(false, false, None, false, None);
}

#[test]
fn native_image_public_acknowledged_original_retains_typed_batch_budget() {
    native_image_public_completion_case(true, false, None, false, None);
}

#[test]
fn native_image_public_region_original_retains_scope_and_one_attempt() {
    native_image_public_completion_case(true, true, None, false, None);
    native_image_public_completion_case(false, true, None, false, None);
}
#[test]
#[cfg(target_os = "linux")]
fn native_owner_no_effect_reconciles_failed_original_and_preserves_archive_replay() {
    native_image_public_completion_case(false, true, None, false, Some("no_effect"));
    native_image_public_completion_case(false, true, None, false, Some("no_effect_bootstrap"));
}

#[test]
#[cfg(target_os = "linux")]
fn native_image_public_completion_and_input_survive_wall_clock_correction() {
    native_image_public_completion_case(true, true, Some("wall_correction"), false, None);
    native_image_public_completion_case(true, true, Some("wall_correction"), true, None);
}

#[test]
fn native_image_public_legacy_receipt_requires_qualified_recapture() {
    native_image_public_completion_case(true, true, Some("legacy"), false, None);
}

#[test]
fn native_image_public_expired_capture_remains_acknowledged_but_input_is_refused() {
    for reason in ["future_timestamp", "age_exceeded"] {
        native_image_public_completion_case(false, true, Some(reason), false, None);
    }
}
fn native_image_public_completion_case(
    acknowledged: bool,
    region: bool,
    clock_fault: Option<&str>,
    rounds: bool,
    round_fault: Option<&str>,
) {
    use crate::native::{
        browser_native_image as image, browser_session_store::BrowserSessionSqliteStore,
    };
    use agent_browser_service_model::BrowserSessionState;
    let profile = if rounds {
        "hcaptcha-image-grid-3x3-two-round-v1"
    } else {
        image::PROFILE_ID
    };
    let config = if rounds {
        "agent-browser:remote-view-native-image-static-3x3-two-round:v1"
    } else {
        image::CONFIG_REF
    };
    struct OwnedProcess(std::process::Child);
    impl Drop for OwnedProcess {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let directory = std::env::temp_dir().join(format!("image-original-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let chrome = directory.join("chrome");
    std::fs::copy("/bin/sleep", &chrome).unwrap();
    let child = OwnedProcess(
        std::process::Command::new(chrome)
            .arg("60")
            .spawn()
            .unwrap(),
    );
    let crate::process_identity::ProcessObservation::Observed(process) =
        crate::process_identity::observe_process(child.0.id())
    else {
        panic!("fixture process missing")
    };
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_CONFIG",
    ]);
    guard.set("HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    guard.remove("AGENT_BROWSER_CONFIG");
    let mut value = serde_json::to_value(BrowserSessionState::default()).unwrap();
    value["sessions"] = json!({"task":{"id":"task","name":"image-lane","profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":0,"currentTabId":"tab"}});
    value["browsers"] = json!({"browser":{"id":"browser","profileId":"profile","pid":child.0.id(),"cdpEndpoint":"","desktop":null,"activeSessionIds":["task"]}});
    value["tabs"] = json!({"tab":{"id":"tab","targetId":"target","browserId":"browser","sessionId":"task","createdAtMs":1,"lastActivityAtMs":1}});
    value["sharedBrowserIds"] = json!(["browser"]);
    value["adoptedBrowserProcessIdentities"] = json!({"browser":{"profileId":"profile","cdpEndpoint":"","process":{"pid":process.pid,"startToken":process.start_token,"executablePath":process.executable_path,"browserFamily":process.browser_family}}});
    let mut store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    store
        .save_session_state(&serde_json::from_value(value).unwrap())
        .unwrap();
    use crate::native::service_store::{JsonServiceStateStore, ServiceStateStore};
    let profile_path = directory.join("profile");
    std::fs::create_dir_all(&profile_path).unwrap();
    let catalog=serde_json::from_value(json!({"schemaVersion":"agent-browser.browser-profile-catalog.v1","profiles":{"profile":{"id":"profile","name":"Image fixture","userDataDir":profile_path,"kind":"named"}}})).unwrap();
    store.save_profile_catalog(&catalog).unwrap();
    let mut service =
        serde_json::to_value(agent_browser_service_model::ServiceState::default()).unwrap();
    service["profiles"] = json!({"profile":{"id":"profile","name":"Image fixture","userDataDir":profile_path,"sitePolicyIds":["image-site"]}});
    service["sitePolicies"] = json!({"image-site":{"id":"image-site","originPattern":"https://owned.example.test","challengePolicy":"provider_allowed","allowedChallengeProviders":["image-provider"]}});
    service["providers"] = json!({"image-provider":{"id":"image-provider","kind":"captcha","enabled":true,"capabilities":["captcha_solve"],"configRef":config}});
    JsonServiceStateStore::new(crate::native::service_store::default_service_state_path().unwrap())
        .save(&serde_json::from_value(service).unwrap())
        .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let _service_time = crate::connection::stable_time::Fixture::observe_with(move || {
        // This second connection must acquire the writer while time acquisition
        // runs. Calling the clock inside the admission transaction deadlocks or
        // fails here; no elapsed-time/sleep assertion or new fixture is needed.
        BrowserSessionSqliteStore::default_sqlite()?.mutate_session_state(|_| Ok(()))?;
        Ok(crate::connection::stable_time::fixture_stamp(now))
    });
    let candidates=(0..9).map(|index|json!({"candidateId":format!("tile:{}:{}",index+1,"a".repeat(16)),"bounds":{"x":100+(index%3)*66,"y":200+(index/3)*66,"width":64,"height":64}})).collect::<Vec<_>>();
    let mut observation = json!({"desktopId":"viewing","generation":9,"windowId":7,"processDigest":"b".repeat(64),"window":{"x":0,"y":0,"width":640,"height":480},"displayWidth":640,"displayHeight":480,"region":{"x":0,"y":0,"width":640,"height":480},"frameSha256":"a".repeat(64),"nativeControlEpoch":4,"observedAtMs":now,"registeredActionId":profile,"puzzleSha256":"","candidates":candidates,"verify":{"x":100,"y":414,"width":80,"height":30}});
    if rounds {
        observation["tileImageSha256s"] = json!(vec!["a".repeat(64); 9]);
    }
    if region {
        observation["captureScope"] = json!("complete_puzzle_region");
        observation["region"] = json!({"x":80,"y":140,"width":440,"height":320});
    }
    observation["puzzleSha256"] = json!(image::digest(&(
        &observation["frameSha256"],
        &observation["candidates"],
        &observation["verify"],
        profile
    ))
    .unwrap());
    let capture_params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_capture","requestId":"image-observation","params":{"classifyChallenges":true,"registeredActionId":profile}}});
    observation["serviceTime"] =
        serde_json::to_value(crate::connection::stable_time::fixture_stamp(now)).unwrap();
    if clock_fault == Some("wall_correction") {
        observation["observedAtMs"] = json!(now + 3_000);
    }
    if let Some(reason) = clock_fault.filter(|reason| *reason != "wall_correction") {
        observation["serviceTime"] = if reason == "legacy" {
            Value::Null
        } else {
            serde_json::to_value(crate::connection::stable_time::fixture_stamp(
                if reason == "future_timestamp" {
                    now + 120_000
                } else {
                    now - 120_000
                },
            ))
            .unwrap()
        };
        task_operations::with_context(Some(&capture_params), || {
            let completed = task_operations::transport(json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":profile}), |_| Ok(crate::connection::Response { success:true, data:Some(json!({"nativeImageObservation":observation})), ..Default::default() })).unwrap();
            assert!(completed.success, "Late age must not discard an acknowledged capture");
            Ok(Value::Null)
        }).unwrap();
        assert_eq!(
            task_operations::lookup("task", "image-observation").unwrap()["operation"]["state"],
            "owner_acknowledged"
        );
        let params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_input","requestId":"stale-image-input"}});
        task_operations::with_context(Some(&params), || {
            let error = task_operations::transport(json!({"action":"native_input","imageAttempt":{"observationRequestId":"image-observation","selectedCandidateIds":[format!("tile:1:{}","a".repeat(16))]}}), |_| panic!("Unqualified age must never dispatch input")).err().expect("Input needs fresh reliable evidence");
            assert!(error.contains("browser_native_time_") || error.contains("browser_native_observation_expired"), "{error}");
            Ok(Value::Null)
        }).unwrap();
        return;
    }
    task_operations::with_context(Some(&capture_params),||{
        let response=task_operations::transport(json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":profile}),|_|Ok(crate::connection::Response{success:true,data:Some(json!({"nativeImageObservation":observation})),..Default::default()})).expect("public image observation must complete");
        assert!(response.success);Ok(Value::Null)
    }).unwrap();
    let mut input = json!({"action":"native_input","imageAttempt":{"observationRequestId":"image-observation","selectedCandidateIds":[format!("tile:1:{}","a".repeat(16)),format!("tile:3:{}","a".repeat(16))]}});
    if rounds {
        input["imageAttempt"]["originalRequestId"] = json!("image-attempt");
    }
    if matches!(round_fault, Some("no_effect" | "no_effect_bootstrap")) {
        input["ownerDeadlineAtMs"] = json!(now + 60_000);
        input["ownerTimeoutMs"] = json!(60_000);
    }
    let mut wrong_profile_envelope = input.clone();
    if rounds {
        wrong_profile_envelope["imageAttempt"]
            .as_object_mut()
            .unwrap()
            .remove("originalRequestId");
    } else {
        wrong_profile_envelope["imageAttempt"]["originalRequestId"] = json!("image-attempt");
    }
    let wrong_profile_params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_input","requestId":"image-attempt","params":wrong_profile_envelope}});
    task_operations::with_context(Some(&wrong_profile_params), || {
        assert!(
            task_operations::transport(wrong_profile_envelope.clone(), |_| panic!(
                "profile authority crossed envelope boundary"
            ))
            .is_err()
        );
        Ok(Value::Null)
    })
    .unwrap();
    let params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_input","requestId":"image-attempt","params":input}});
    let mut dispatched = 0;
    let mut admission_detail = String::new();
    task_operations::with_context(Some(&params), || {
        let response = task_operations::transport(input.clone(), |command| {
            dispatched += 1;
            if matches!(round_fault, Some("no_effect" | "no_effect_bootstrap")) {
                assert_eq!(command["_nativeOwnerAdmittedServiceTime"],
                    serde_json::to_value(crate::connection::stable_time::fixture_stamp(now)).unwrap());
                assert!(command.get("_nativeOwnerAdmittedAtMs").is_none());
            }
            let binding = crate::native::browser_task_operations::native_site_policy(
                &command,
                "https://owned.example.test",
            ).map_err(|error| {admission_detail=format!("policy:{} clock={:?}",error.split(':').next().unwrap_or("unknown"),task_operations::completion_failure_clock(&error));error})?;
            crate::native::browser_task_operations::retain_native_site_policy(&command, &binding).map_err(|error| {admission_detail=format!("publication:{} clock={:?}",error.split(':').next().unwrap_or("unknown"),task_operations::completion_failure_clock(&error));error})?;
            if matches!(round_fault, Some("no_effect" | "no_effect_bootstrap")) {
                struct RefusingOwner(&'static str);
                impl crate::native::browser_native_control::NativeOwner for RefusingOwner {
                    fn image_observation_clock(&mut self) -> Result<crate::native::browser_native_image::NativeImageClockReading, String> {
                        Err(self.0.into())
                    }
                    fn request(&mut self, _: &str, _: &Value) -> Result<Value,String> { panic!("Refused before any physical attempt") }
                    fn record_pre_input_refusal(&mut self, command: &Value, refusal: &crate::native::browser_native_control::PreInputRefusal) -> Result<(),String> {
                        crate::native::browser_task_operations::record_native_no_effect(command, refusal)
                    }
                }
                let original = agent_browser_service_model::RecordedProcessIdentity { pid:process.pid,
                    start_token:process.start_token.clone().unwrap(), executable_path:process.executable_path.clone(), browser_family:process.browser_family.clone() };
                let target = agent_browser_service_model::RemoteViewApplicationTarget { assignment_id:"fixture".into(),registration_id:"fixture".into(),
                    desktop_id:"lifecycle".into(),lifecycle_generation:1,viewing_desktop_id:"viewing".into(),viewing_generation:9 };
                let cause = if round_fault == Some("no_effect_bootstrap") { "browser_native_owner_unconfigured" } else { "browser_native_time_unavailable:cause=stable_time_client_start_failed" };
                let error = crate::native::browser_native_control::execute(&original, &target, &command, &mut RefusingOwner(cause)).unwrap_err();
                return Ok(crate::connection::Response { success:false, error:Some(error), ..Default::default() });
            }
            Ok(crate::connection::Response {
                success: acknowledged,
                data: Some(json!({"source":"remote_view_native","delivery":"dispatched"})),
                error: (!acknowledged).then(|| "external Native reply unavailable".into()),
                ..Default::default()
            })
        })
        .unwrap_or_else(|error|panic!("completed original must remain readable: {error}; owner admission={admission_detail}"));
        assert_eq!(response.success, acknowledged);
        Ok(Value::Null)
    })
    .unwrap();
    let readback = task_operations::lookup("task", "image-attempt").unwrap();
    if matches!(round_fault, Some("no_effect" | "no_effect_bootstrap")) {
        assert_eq!(readback["operation"]["state"], "uncertain");
        let original_failure = if round_fault == Some("no_effect_bootstrap") {
            "browser_native_owner_unconfigured"
        } else {
            "browser_native_time_unavailable:cause=stable_time_client_start_failed"
        };
        let signature = store.load_session_state().unwrap().task_operations["task"]
            ["image-attempt"]
            .evidence_references["nativeOwnerNoEffectSignature"]
            .clone();
        store
            .mutate_session_state(|state| {
                state
                    .task_operations
                    .get_mut("task")
                    .unwrap()
                    .get_mut("image-attempt")
                    .unwrap()
                    .evidence_references
                    .insert("nativeOwnerNoEffectSignature".into(), "00".into());
                Ok(())
            })
            .unwrap();
        assert!(
            crate::native::browser_task_operations::reconcile_native_no_effect(
                "task",
                "image-attempt"
            )
            .unwrap_err()
            .contains("proof_invalid")
        );
        store
            .mutate_session_state(|state| {
                state
                    .task_operations
                    .get_mut("task")
                    .unwrap()
                    .get_mut("image-attempt")
                    .unwrap()
                    .evidence_references
                    .insert("nativeOwnerNoEffectSignature".into(), signature.clone());
                state.next_session_sequence += 1;
                state.browsers.get_mut("browser").unwrap().pid = u32::MAX;
                Ok(())
            })
            .unwrap();
        let advanced = crate::native::browser_host_control::change(
            "browser",
            "takeover",
            0,
            "human",
            "Fixture epoch advance after refusal",
            false,
        )
        .unwrap();
        let public = task_operations::call(&json!({"taskId":"task","params":{"operation":"reconcile","requestId":"image-attempt","claim":"no_effect"}}), "fixture").unwrap();
        let settled: Value =
            serde_json::from_str(public["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            settled["data"]["operation"]["state"],
            "owner_refused_no_effect"
        );
        assert_eq!(
            settled["data"]["operation"]["originalResponse"]["success"],
            false
        );
        assert_eq!(settled["data"]["operation"]["effectStatus"], "no_effect");
        assert_eq!(settled["data"]["currentContext"], "changed_or_missing");
        assert!(!settled.to_string().contains("fingerprint"));
        task_operations::with_context(Some(&params), || {
            let replay = task_operations::transport(input.clone(), |_| {
                panic!("Settled failed original must never replay")
            })
            .unwrap();
            assert!(!replay.success);
            assert_eq!(replay.error.as_deref(), Some(original_failure));
            assert_eq!(replay.data.unwrap()["dispatchAttempted"], false);
            Ok(Value::Null)
        })
        .unwrap();
        let mut changed = input.clone();
        changed["imageAttempt"]["selectedCandidateIds"] = json!(["different-intent"]);
        task_operations::with_context(Some(&params), || {
            assert!(task_operations::transport(changed, |_| panic!(
                "Changed original arguments cannot dispatch"
            ))
            .err()
            .unwrap()
            .contains("request_conflict"));
            Ok(Value::Null)
        })
        .unwrap();
        store
            .mutate_session_state(|state| {
                state.browsers.get_mut("browser").unwrap().pid = process.pid;
                Ok(())
            })
            .unwrap();
        let returned = crate::native::browser_host_control::change(
            "browser",
            "return",
            advanced,
            "human",
            "Fixture viewer return",
            false,
        )
        .unwrap();
        let fresh_control = crate::native::browser_host_control::admit(
            "browser",
            "native_capture",
            "fresh-after-no-effect",
        )
        .unwrap()
        .unwrap();
        let mut reopened = BrowserSessionSqliteStore::default_sqlite().unwrap();
        reopened
            .task_operation_capacity(
                "task",
                "task",
                Some(
                    serde_json::from_value(
                        json!({"initialCapacity":1,"extensionIncrement":1,"maximumCapacity":1}),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        let mut fresh_params = capture_params.clone();
        fresh_params["arguments"]["requestId"] = json!("fresh-after-no-effect");
        task_operations::with_context(Some(&fresh_params), || {
            task_operations::transport(json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":profile}), |_| Ok(crate::connection::Response { success:true,
                data:Some(json!({"nativeImageObservation":observation})), ..Default::default() })).unwrap(); Ok(Value::Null)
        }).unwrap();
        fresh_control.finish(true).unwrap();
        assert_eq!(
            crate::native::browser_host_control::read("browser")
                .unwrap()
                .epoch,
            returned
        );
        let db =
            rusqlite::Connection::open(BrowserSessionSqliteStore::default_sqlite_path().unwrap())
                .unwrap();
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            7,
            "Archival cannot downgrade the failed-terminal reader guard"
        );
        assert_eq!(
            reopened
                .archived_task_operation("task", "image-attempt")
                .unwrap()
                .unwrap()
                .state,
            agent_browser_service_model::BrowserTaskOperationState::OwnerRefusedNoEffect
        );
        let mut next = input.clone();
        next["imageAttempt"]["observationRequestId"] = json!("fresh-after-no-effect");
        let next_params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_input","requestId":"new-explicit-intent","params":next}});
        task_operations::with_context(Some(&next_params), || {
            let response = task_operations::transport(next.clone(), |command| {
                let binding = crate::native::browser_task_operations::native_site_policy(
                    &command,
                    "https://owned.example.test",
                )?;
                crate::native::browser_task_operations::retain_native_site_policy(
                    &command, &binding,
                )?;
                Ok(crate::connection::Response {
                    success: true,
                    data: Some(json!({"source":"remote_view_native","delivery":"dispatched"})),
                    ..Default::default()
                })
            })
            .unwrap();
            assert!(response.success);
            assert_eq!(
                response.data.as_ref().unwrap()["taskOperation"]["state"],
                "owner_acknowledged"
            );
            Ok(Value::Null)
        })
        .unwrap();
        task_operations::with_context(Some(&params), || {
            let replay = task_operations::transport(input.clone(), |_| {
                panic!("Archived failed original must never replay")
            })
            .unwrap();
            assert!(!replay.success);
            assert_eq!(replay.error.as_deref(), Some(original_failure));
            Ok(Value::Null)
        })
        .unwrap();
        // Exercise the actual public facade after archival, with no current
        // routing authority and both unavailable and certainly expired time.
        reopened
            .mutate_session_state(|state| {
                state.browsers.get_mut("browser").unwrap().pid = u32::MAX;
                Ok(())
            })
            .unwrap();
        for clock in [
            Err("stable_time_client_start_failed".to_owned()),
            Ok(crate::connection::stable_time::fixture_stamp(now + 120_000)),
        ] {
            let _clock = crate::connection::stable_time::Fixture::set(clock);
            let response = handle_jsonrpc_line(
                &json!({
                    "jsonrpc":"2.0","id":762,"method":"tools/call","params":params
                })
                .to_string(),
                "never-started",
            )
            .unwrap();
            assert_eq!(
                response["error"]["data"]["originalReplay"], true,
                "{response}"
            );
            assert_eq!(
                response["error"]["data"]["dispatchAttempted"], false,
                "{response}"
            );
            assert!(
                response.to_string().contains(original_failure),
                "{response}"
            );
            assert!(!response.to_string().contains("callerCommandFingerprint"));
            let mut changed = params.clone();
            changed["arguments"]["params"]["ownerDeadlineAtMs"] = json!(now + 180_000);
            let conflict = handle_jsonrpc_line(
                &json!({"jsonrpc":"2.0","id":763,
                "method":"tools/call","params":changed})
                .to_string(),
                "never-started",
            )
            .unwrap();
            assert!(
                conflict
                    .to_string()
                    .contains("browser_task_operation_request_conflict"),
                "{conflict}"
            );
            let mut unknown = params.clone();
            unknown["arguments"]["requestId"] = json!("unknown-original");
            let refused = handle_jsonrpc_line(
                &json!({"jsonrpc":"2.0","id":764,
                "method":"tools/call","params":unknown})
                .to_string(),
                "never-started",
            )
            .unwrap();
            assert!(refused.get("error").is_some(), "{refused}");
            assert_ne!(
                refused["error"]["data"]["originalReplay"], true,
                "{refused}"
            );
        }
        return;
    }
    if !acknowledged {
        assert!(
            crate::native::browser_task_operations::reconcile_native_no_effect(
                "task",
                "image-attempt"
            )
            .unwrap_err()
            .contains("proof_missing"),
            "Post-dispatch unknown input cannot be settled from a caller claim or error text"
        );
    }
    assert_eq!(
        readback["operation"]["state"],
        if acknowledged {
            "owner_acknowledged"
        } else {
            "uncertain"
        }
    );
    if let Some(elapsed) = observation["serviceTime"]["earliestUtcNs"]
        .as_str()
        .and_then(|value| value.parse::<i64>().ok())
        .map(|ns| ns as u64 / 1_000_000)
    {
        assert_eq!(
            readback["operation"]["evidenceReferences"]["nativeChallengeDeadlineAtMs"],
            (elapsed + 60_000).to_string(),
            "Original deadline is elapsed authority, not its diagnostic wall timestamp"
        );
    }
    if clock_fault == Some("legacy") {
        let captured = task_operations::lookup("task", "image-observation").unwrap();
        let evidence: Value = serde_json::from_str(
            captured["operation"]["evidenceReferences"]["nativeImageObservation"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert!(
            evidence.get("clock").is_none(),
            "Legacy serialized receipt remains unchanged"
        );
    }
    let round: Value = serde_json::from_str(
        readback["operation"]["evidenceReferences"]["nativeImageRound"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(round["snapshot"]["totalPointerEvents"], 9);
    assert_eq!(round["snapshot"]["totalSelections"], 2);
    assert_eq!(
        readback["operation"]["evidenceReferences"]["nativeImageBudgetAccounting"],
        if acknowledged {
            "derived_batch_acknowledged"
        } else {
            "reserved_maximum_on_uncertain"
        }
    );
    assert_eq!(readback["operation"]["retryAllowed"], false);
    task_operations::with_context(Some(&params), || {
        let replay = task_operations::transport(input.clone(), |_| {
            dispatched += 1;
            panic!("image original replayed")
        })
        .unwrap();
        assert_eq!(replay.data.unwrap()["dispatchAttempted"], false);
        Ok(Value::Null)
    })
    .unwrap();
    assert_eq!(dispatched, 1);
    if rounds && acknowledged {
        let first_round = readback["operation"].clone();
        if matches!(round_fault, Some("revision" | "policy")) {
            let service_store = JsonServiceStateStore::new(
                crate::native::service_store::default_service_state_path().unwrap(),
            );
            let mut updated = service_store.load().unwrap();
            updated.state_revision = updated.state_revision().checked_add(1).unwrap();
            if round_fault == Some("policy") {
                updated
                    .site_policies
                    .get_mut("image-site")
                    .unwrap()
                    .challenge_policy =
                    agent_browser_service_model::ChallengePolicy::ProviderPreferred;
            }
            service_store.save(&updated).unwrap();
        }

        let mut fresh = observation.clone();
        fresh["tileImageSha256s"] = json!(vec!["c".repeat(64); 9]);
        fresh["frameSha256"] = json!("c".repeat(64));
        fresh["observedAtMs"] = json!(now + 1);
        fresh["serviceTime"] =
            serde_json::to_value(crate::connection::stable_time::fixture_stamp(now + 1)).unwrap();
        let _next_time = crate::connection::stable_time::Fixture::set(Ok(
            crate::connection::stable_time::fixture_stamp(now + 2),
        ));
        for (index, candidate) in fresh["candidates"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            candidate["candidateId"] = json!(format!("tile:{}:{}", index + 1, "c".repeat(16)));
        }
        fresh["puzzleSha256"] = json!(image::digest(&(
            &fresh["frameSha256"],
            &fresh["candidates"],
            &fresh["verify"],
            profile
        ))
        .unwrap());
        if round_fault == Some("partial") {
            fresh["tileImageSha256s"][4] = json!("a".repeat(64));
        }
        if round_fault == Some("context") {
            fresh["windowId"] = json!(8);
        }
        let mut second_capture = capture_params.clone();
        second_capture["arguments"]["requestId"] = json!("image-observation-2");
        task_operations::with_context(Some(&second_capture), || {
            let response = task_operations::transport(json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":profile}), |_| Ok(crate::connection::Response { success:true, data:Some(json!({"nativeImageObservation":fresh})), ..Default::default() })).unwrap();
            assert!(response.success);
            Ok(Value::Null)
        }).unwrap();
        let mut second_input = json!({"action":"native_input","imageAttempt":{"originalRequestId":"image-attempt","observationRequestId":"image-observation-2","selectedCandidateIds":[format!("tile:2:{}", "c".repeat(16))]}});
        if round_fault == Some("stale") {
            second_input["imageAttempt"]["selectedCandidateIds"] =
                input["imageAttempt"]["selectedCandidateIds"].clone();
        }
        if round_fault == Some("original") {
            second_input["imageAttempt"]["originalRequestId"] = json!("different-attempt");
        }
        let second_params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_input","requestId":"image-attempt-2","params":second_input}});
        task_operations::with_context(Some(&second_params), || {
            let response = task_operations::transport(second_input.clone(), |command| {
                let binding = crate::native::browser_task_operations::native_site_policy(
                    &command,
                    "https://owned.example.test",
                )?;
                if matches!(round_fault, Some("revision" | "policy")) {
                    let prior: Value = serde_json::from_str(
                        first_round["evidenceReferences"]["nativeSitePolicy"]
                            .as_str()
                            .unwrap(),
                    )
                    .unwrap();
                    assert_ne!(
                        prior["observedServiceRevision"],
                        binding["observedServiceRevision"]
                    );
                    assert_eq!(
                        prior["digest"] == binding["digest"],
                        round_fault == Some("revision")
                    );
                }
                crate::native::browser_task_operations::retain_native_site_policy(
                    &command, &binding,
                )?;
                dispatched += 1;
                Ok(crate::connection::Response {
                    success: true,
                    data: Some(json!({"source":"remote_view_native","delivery":"dispatched"})),
                    ..Default::default()
                })
            });
            if round_fault.is_some() && round_fault != Some("revision") {
                assert!(response.is_err());
            } else {
                assert!(response.unwrap().success);
            }
            Ok(Value::Null)
        })
        .unwrap();
        if round_fault.is_some() && round_fault != Some("revision") {
            assert_eq!(dispatched, 1);
            assert_eq!(
                task_operations::lookup("task", "image-attempt").unwrap()["operation"],
                first_round
            );
            return;
        }
        assert_eq!(dispatched, 2);
        assert_eq!(
            task_operations::lookup("task", "image-attempt").unwrap()["operation"],
            first_round
        );
        let second = task_operations::lookup("task", "image-attempt-2").unwrap();
        let second_round: Value = serde_json::from_str(
            second["operation"]["evidenceReferences"]["nativeImageRound"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(second_round["snapshot"]["attemptId"], "image-attempt");
        assert_eq!(second_round["snapshot"]["totalPointerEvents"], 15);
        assert_eq!(second_round["snapshot"]["totalSelections"], 3);
        assert_eq!(second_round["deadlineAtMs"], round["deadlineAtMs"]);
        task_operations::with_context(Some(&second_params), || {
            let replay = task_operations::transport(second_input.clone(), |_| {
                panic!("second round replayed")
            })
            .unwrap();
            assert_eq!(replay.data.unwrap()["dispatchAttempted"], false);
            Ok(Value::Null)
        })
        .unwrap();
    }
    let mut another = params.clone();
    another["arguments"]["requestId"] = json!("new-image-attempt");
    task_operations::with_context(Some(&another), || {
        let failure =
            task_operations::transport(input.clone(), |_| panic!("taskwide image budget reset"))
                .err()
                .unwrap();
        assert!(failure.contains("attempt_budget_exhausted"), "{failure}");
        Ok(Value::Null)
    })
    .unwrap();
}

#[test]
fn native_image_rounds_public_original_retains_shared_budget() {
    native_image_public_completion_case(true, true, None, true, None);
}

#[test]
fn native_image_rounds_public_policy_revision_drift_preserves_stable_authority() {
    native_image_public_completion_case(true, true, None, true, Some("revision"));
    native_image_public_completion_case(true, true, None, true, Some("policy"));
}

#[test]
fn native_image_rounds_public_refusals_preserve_first_original() {
    for fault in ["partial", "context", "stale", "original"] {
        native_image_public_completion_case(true, true, None, true, Some(fault));
    }
    native_image_public_completion_case(false, true, None, true, None);
}

#[test]
fn native_image_completion_diagnostics_discard_every_private_suffix() {
    for code in [
        "browser_native_image_observation_invalid",
        "browser_native_image_observation_expired",
        "browser_native_image_round_invalid",
        "browser_native_image_round_changed",
        "browser_native_image_round_missing",
    ] {
        assert_eq!(task_operations::completion_failure_cause(code), code);
        assert_eq!(
            task_operations::completion_failure_cause(&format!(
                "{code}:SELECT password FROM private; /private/path"
            )),
            code
        );
        assert_eq!(
            task_operations::completion_failure_cause(&format!("unknown-{code}:private")),
            "unclassified"
        );
    }
}

#[test]
fn native_image_completion_clock_diagnostics_are_exact_and_content_free() {
    let future =
        json!({"reason":"future_timestamp","observedAtMs":100,"validationNowMs":99,"ageMs":0});
    let old =
        json!({"reason":"age_exceeded","observedAtMs":100,"validationNowMs":60101,"ageMs":60001});
    for valid in [&future, &old] {
        assert_eq!(
            task_operations::completion_failure_clock(&format!(
                "browser_native_image_observation_expired:{valid}"
            )),
            Some(valid.clone())
        );
    }
    let mut bads = vec![
        json!({}),
        json!("/private/path SELECT password"),
        json!({"reason":"future_timestamp","observedAtMs":100,"validationNowMs":101,"ageMs":0}),
        json!({"reason":"age_exceeded","observedAtMs":100,"validationNowMs":60100,"ageMs":60000}),
    ];
    for key in ["reason", "observedAtMs", "validationNowMs", "ageMs"] {
        let mut bad = future.clone();
        bad[key] = json!("/private/path SELECT password");
        bads.push(bad);
    }
    let mut extra = future.clone();
    extra["privatePath"] = json!("/private/path SELECT password");
    bads.push(extra);
    let mut inconsistent = old.clone();
    inconsistent["ageMs"] = json!(60002);
    bads.push(inconsistent);
    for bad in bads {
        assert!(task_operations::completion_failure_clock(&format!(
            "browser_native_image_observation_expired:{bad}"
        ))
        .is_none());
    }
    for bad in [
        format!("browser_native_image_observation_expired:{future}:SELECT password"),
        format!("unknown:{future}"),
        "browser_native_image_observation_expired:/private/path SELECT password".into(),
    ] {
        assert!(task_operations::completion_failure_clock(&bad).is_none());
    }
}

#[test]
fn public_task_capacity_can_change_without_transport_or_losing_pending_history() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    let directory =
        std::env::temp_dir().join(format!("task-capacity-public-{}", uuid::Uuid::new_v4()));
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_SOCKET_DIR",
    ]);
    guard.set("HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    guard.set(
        "AGENT_BROWSER_SOCKET_DIR",
        directory.join("never-started").to_str().unwrap(),
    );
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let mut value =
        serde_json::to_value(agent_browser_service_model::BrowserSessionState::default()).unwrap();
    value["sessions"] = json!({"task":{"id":"task","name":"capacity","profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":0,"currentTabId":null}});
    value["taskOperations"] = json!({"task":{"pending":{"requestId":"pending","taskId":"task","profileId":"profile","browserId":"browser","tabId":"tab","targetId":"target","processPid":42,"controlEpoch":3,"action":"fill","fingerprint":"private-mac","state":"pending","createdAtMs":10,"completedAtMs":null,"evidenceReferences":{}}}});
    let state = serde_json::from_value(value).unwrap();
    store.save_session_state(&state).unwrap();
    let invoke = |params: Value| {
        let request = json!({"jsonrpc":"2.0","id":751,"method":"tools/call","params":{"name":"browser_command","arguments":{"taskId":"task","action":"task_operation","params":params}}});
        let response = handle_jsonrpc_line(&request.to_string(), "unused").unwrap();
        assert!(response.get("error").is_none(), "{response}");
        let payload: Value =
            serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        payload["data"].clone()
    };
    let initial = invoke(json!({"operation":"capacity"}));
    assert_eq!(initial["policy"]["initialCapacity"], 256);
    assert_eq!(initial["activeCount"], 1);
    let updated = invoke(
        json!({"operation":"capacity","policy":{"initialCapacity":512,"extensionIncrement":128,"maximumCapacity":1024}}),
    );
    assert_eq!(updated["policySource"], "task_override");
    assert_eq!(updated["capacity"], 512);
    drop(store);
    let reopened = invoke(json!({"operation":"capacity"}));
    assert_eq!(reopened["policy"]["maximumCapacity"], 1024);
    let original = invoke(json!({"requestId":"pending"}));
    assert_eq!(original["operation"]["state"], "pending");
    assert!(!directory.join("never-started").exists());
    let defaults = invoke(
        json!({"operation":"capacity","scope":"defaults","policy":{"initialCapacity":128,"extensionIncrement":64,"maximumCapacity":2048}}),
    );
    assert_eq!(
        defaults["policy"]["initialCapacity"], 512,
        "Existing task override survives a defaults change"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn public_task_history_archives_and_originals_still_replay_after_restart() {
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use agent_browser_service_model::BrowserSessionState;
    struct OwnedProcess(std::process::Child);
    impl Drop for OwnedProcess {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let directory =
        std::env::temp_dir().join(format!("task-history-capacity-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let chrome = directory.join("chrome");
    std::fs::copy("/bin/sleep", &chrome).unwrap();
    let child = OwnedProcess(
        std::process::Command::new(&chrome)
            .arg("120")
            .spawn()
            .unwrap(),
    );
    let crate::process_identity::ProcessObservation::Observed(process) =
        crate::process_identity::observe_process(child.0.id())
    else {
        panic!("Owned process unavailable")
    };
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
    ]);
    guard.set("HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", directory.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    let mut value = serde_json::to_value(BrowserSessionState::default()).unwrap();
    value["sessions"] = json!({"task":{"id":"task","name":"capacity-lane","profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":0,"currentTabId":"tab"}});
    value["browsers"] = json!({"browser":{"id":"browser","profileId":"profile","pid":child.0.id(),"cdpEndpoint":"","desktop":null,"activeSessionIds":["task"]}});
    value["tabs"] = json!({"tab":{"id":"tab","targetId":"target","browserId":"browser","sessionId":"task","createdAtMs":1,"lastActivityAtMs":1}});
    value["sharedBrowserIds"] = json!(["browser"]);
    value["adoptedBrowserProcessIdentities"] = json!({"browser":{"profileId":"profile","cdpEndpoint":"","process":{"pid":process.pid,"startToken":process.start_token,"executablePath":process.executable_path,"browserFamily":process.browser_family}}});
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    store
        .save_session_state(&serde_json::from_value(value).unwrap())
        .unwrap();
    drop(store);
    for index in 0..260 {
        let params = json!({"name":"browser_click","arguments":{"taskId":"task","requestId":format!("step-{index}"),"selector":"#next"}});
        task_operations::with_context(Some(&params), || {
            let reply =
                task_operations::transport(json!({"action":"click","selector":"#next"}), |_| {
                    Ok(crate::connection::Response {
                        success: true,
                        ..Default::default()
                    })
                })
                .unwrap();
            assert!(reply.success);
            Ok(Value::Null)
        })
        .unwrap();
    }
    task_operations::call(&json!({"taskId":"task","params":{"operation":"capacity","policy":{"initialCapacity":2,"extensionIncrement":2,"maximumCapacity":4}}}), "unused").unwrap();
    for index in 0..4 {
        let params = json!({"name":"browser_click","arguments":{"taskId":"task","requestId":format!("unresolved-{index}"),"selector":"#next"}});
        task_operations::with_context(Some(&params), || {
            let outcome =
                task_operations::transport(json!({"action":"click","selector":"#next"}), |_| {
                    Err("transport lost".into())
                });
            assert!(outcome.err().unwrap().contains("uncertain"));
            Ok(Value::Null)
        })
        .unwrap();
    }
    let full = json!({"name":"browser_click","arguments":{"taskId":"task","requestId":"after-ceiling","selector":"#next"}});
    task_operations::with_context(Some(&full), || {
        let outcome =
            task_operations::transport(json!({"action":"click","selector":"#next"}), |_| {
                panic!("Capacity exhaustion dispatched an effect")
            });
        let message = outcome.err().unwrap();
        assert!(message.contains("capacity_reached"));
        assert!(message.contains("maximumCapacity"));
        Ok(Value::Null)
    })
    .unwrap();
    task_operations::call(&json!({"taskId":"task","params":{"operation":"capacity","policy":{"initialCapacity":2,"extensionIncrement":2,"maximumCapacity":6}}}), "unused").unwrap();
    task_operations::with_context(Some(&full), || {
        let outcome =
            task_operations::transport(json!({"action":"click","selector":"#next"}), |_| {
                Ok(crate::connection::Response {
                    success: true,
                    ..Default::default()
                })
            })
            .unwrap();
        assert!(outcome.success);
        Ok(Value::Null)
    })
    .unwrap();
    let first = task_operations::lookup("task", "step-0").unwrap();
    assert_eq!(first["operation"]["state"], "owner_acknowledged");
    let capacity = task_operations::call(
        &json!({"taskId":"task","params":{"operation":"capacity"}}),
        "unused",
    )
    .unwrap();
    let payload: Value =
        serde_json::from_str(capacity["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(payload["data"]["archivedCount"].as_u64().unwrap() > 0);
    let params = json!({"name":"browser_click","arguments":{"taskId":"task","requestId":"step-0","selector":"#next"}});
    task_operations::with_context(Some(&params), || {
        let reply =
            task_operations::transport(json!({"action":"click","selector":"#next"}), |_| {
                panic!("Archived original redispatched")
            })
            .unwrap();
        assert!(reply.success);
        assert_eq!(reply.data.unwrap()["dispatchAttempted"], false);
        let conflict =
            task_operations::transport(json!({"action":"click","selector":"#changed"}), |_| {
                panic!("Conflicting original redispatched")
            });
        assert!(conflict.err().unwrap().contains("request_conflict"));
        Ok(Value::Null)
    })
    .unwrap();
}
