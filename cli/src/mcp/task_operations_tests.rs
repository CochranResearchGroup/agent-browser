use super::*;

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
    native_image_public_completion_case(false, false, None);
}

#[test]
fn native_image_public_acknowledged_original_retains_typed_batch_budget() {
    native_image_public_completion_case(true, false, None);
}

#[test]
fn native_image_public_region_original_retains_scope_and_one_attempt() {
    native_image_public_completion_case(true, true, None);
    native_image_public_completion_case(false, true, None);
}
#[test]
fn native_image_public_capture_expiry_reports_only_bounded_clock_values() {
    for reason in ["future_timestamp", "age_exceeded"] {
        native_image_public_completion_case(false, true, Some(reason));
    }
}
fn native_image_public_completion_case(
    acknowledged: bool,
    region: bool,
    clock_fault: Option<&str>,
) {
    use crate::native::{
        browser_native_image as image, browser_session_store::BrowserSessionSqliteStore,
    };
    use agent_browser_service_model::BrowserSessionState;
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
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
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
    service["providers"] = json!({"image-provider":{"id":"image-provider","kind":"captcha","enabled":true,"capabilities":["captcha_solve"],"configRef":image::CONFIG_REF}});
    JsonServiceStateStore::new(crate::native::service_store::default_service_state_path().unwrap())
        .save(&serde_json::from_value(service).unwrap())
        .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let candidates=(0..9).map(|index|json!({"candidateId":format!("tile:{}:{}",index+1,"a".repeat(16)),"bounds":{"x":100+(index%3)*66,"y":200+(index/3)*66,"width":64,"height":64}})).collect::<Vec<_>>();
    let mut observation = json!({"desktopId":"viewing","generation":9,"windowId":7,"processDigest":"b".repeat(64),"window":{"x":0,"y":0,"width":640,"height":480},"displayWidth":640,"displayHeight":480,"region":{"x":0,"y":0,"width":640,"height":480},"frameSha256":"a".repeat(64),"nativeControlEpoch":4,"observedAtMs":now,"registeredActionId":image::PROFILE_ID,"puzzleSha256":"","candidates":candidates,"verify":{"x":100,"y":414,"width":80,"height":30}});
    if region {
        observation["captureScope"] = json!("complete_puzzle_region");
        observation["region"] = json!({"x":80,"y":140,"width":440,"height":320});
    }
    observation["puzzleSha256"] = json!(image::digest(&(
        &observation["frameSha256"],
        &observation["candidates"],
        &observation["verify"],
        image::PROFILE_ID
    ))
    .unwrap());
    let capture_params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_capture","requestId":"image-observation","params":{"classifyChallenges":true,"registeredActionId":image::PROFILE_ID}}});
    if let Some(reason) = clock_fault {
        let observed = if reason == "future_timestamp" {
            now + 120_000
        } else {
            now - 120_000
        };
        observation["observedAtMs"] = json!(observed);
        let mut failure = String::new();
        task_operations::with_context(Some(&capture_params),||{
            failure=match task_operations::transport(json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":image::PROFILE_ID}),|_|Ok(crate::connection::Response{success:true,data:Some(json!({"nativeImageObservation":observation})),..Default::default()})){Err(error)=>error,Ok(_)=>panic!("expired image observation must refuse completion")};Ok(Value::Null)
        }).unwrap();
        assert!(failure.contains("cause=browser_native_image_observation_expired"));
        let safe: Value = serde_json::from_str(
            failure
                .split(";observationClock=")
                .nth(1)
                .expect("public completion must retain only bounded image clock diagnostics"),
        )
        .unwrap();
        assert_eq!(safe["reason"], reason);
        assert_eq!(safe["observedAtMs"], observed);
        assert_eq!(safe.as_object().unwrap().len(), 4);
        let validated = safe["validationNowMs"].as_u64().unwrap();
        assert_eq!(safe["ageMs"], validated.saturating_sub(observed));
        assert_eq!(reason == "future_timestamp", validated < observed);
        assert_eq!(
            task_operations::lookup("task", "image-observation").unwrap()["operation"]["state"],
            "pending"
        );
        task_operations::with_context(Some(&capture_params),||{
            let repeated=task_operations::transport(json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":image::PROFILE_ID}),|_|panic!("expired original must never redispatch capture")).unwrap();
            assert!(!repeated.success);assert_eq!(repeated.data.unwrap()["replayed"],true);Ok(Value::Null)
        }).unwrap();
        return;
    }
    task_operations::with_context(Some(&capture_params),||{
        let response=task_operations::transport(json!({"action":"native_capture","classifyChallenges":true,"registeredActionId":image::PROFILE_ID}),|_|Ok(crate::connection::Response{success:true,data:Some(json!({"nativeImageObservation":observation})),..Default::default()})).expect("public image observation must complete");
        assert!(response.success);Ok(Value::Null)
    }).unwrap();
    let input = json!({"action":"native_input","imageAttempt":{"observationRequestId":"image-observation","selectedCandidateIds":[format!("tile:1:{}","a".repeat(16)),format!("tile:3:{}","a".repeat(16))]}});
    let params = json!({"name":"browser_command","arguments":{"taskId":"task","action":"native_input","requestId":"image-attempt","params":input}});
    let mut dispatched = 0;
    let mut admission_detail = String::new();
    task_operations::with_context(Some(&params), || {
        let response = task_operations::transport(input.clone(), |command| {
            dispatched += 1;
            let binding = crate::native::browser_task_operations::native_site_policy(
                &command,
                "https://owned.example.test",
            ).map_err(|error| {admission_detail=format!("policy:{} clock={:?}",error.split(':').next().unwrap_or("unknown"),task_operations::completion_failure_clock(&error));error})?;
            crate::native::browser_task_operations::retain_native_site_policy(&command, &binding).map_err(|error| {admission_detail=format!("publication:{} clock={:?}",error.split(':').next().unwrap_or("unknown"),task_operations::completion_failure_clock(&error));error})?;
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
    assert_eq!(
        readback["operation"]["state"],
        if acknowledged {
            "owner_acknowledged"
        } else {
            "uncertain"
        }
    );
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
