// Included inside browser_host_control::integration_tests. These fixtures never
// launch a browser or authorize recovery; all identities are synthetic records.

fn return_observation_state() -> BrowserSessionState {
    let mut state = BrowserSessionState::default();
    state.sessions.insert(
        "task".into(),
        serde_json::from_value(json!({
            "id":"task","name":"hosted-return","profileId":"profile",
            "browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,
            "expiresAtMs":u64::MAX,"currentTabId":"tab"
        }))
        .unwrap(),
    );
    state.browsers.insert(
        "browser".into(),
        serde_json::from_value(json!({
            "id":"browser","profileId":"profile","pid":999999,
            "cdpEndpoint":"ws://127.0.0.1:1","activeSessionIds":["task"]
        }))
        .unwrap(),
    );
    state.tabs.insert(
        "tab".into(),
        serde_json::from_value(json!({
            "id":"tab","targetId":"raw-target","browserId":"browser",
            "sessionId":"task","createdAtMs":1,"lastActivityAtMs":1
        }))
        .unwrap(),
    );
    state.host_bindings.insert(
        "task".into(),
        serde_json::from_value(json!({
            "request":{"version":1,"hostId":"app","participantId":"participant",
                "sessionId":"app-session","placement":{"mode":"required",
                    "desktop":{"desktopId":"desktop","generation":1}}},
            "desktop":{"desktop_id":"desktop","friendly_route_label":"Desktop 1","generation":1}
        }))
        .unwrap(),
    );
    state
}

fn return_observation_command() -> Value {
    let mut command = json!({
        "id":"return-observation","action":"url","hostControlFreshObservation":true,
        "sessionName":"hosted-return","sessionId":"task","targetId":"raw-target",
        "browserId":"browser","profileId":"profile",
        "hostControlReturnObservation":{
            "browserId":"browser","taskId":"task","profileId":"profile",
            "targetId":"raw-target","expectedEpoch":1,"actor":"returning operator",
            "reason":"Explicit same-target return observation"
        }
    });
    command["hostControlReturnObservation"] = attest_return_observation(command["hostControlReturnObservation"].clone()).unwrap();
    command
}

#[test]
fn host_control_return_exact_observation_is_narrow_and_does_not_change_ownership() {
    fixture(|| {
        let state = return_observation_state();
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let before = metadata("browser").unwrap();
        let command = return_observation_command();
        assert!(check("browser", "url").is_err());
        let mut ordinary = command.clone();
        ordinary.as_object_mut().unwrap().remove("hostControlReturnObservation");
        assert!(check_state(&state, "hosted-return", &ordinary).is_err());
        assert!(authorize_return_observation(&state, &command).is_ok());
        assert!(check_state(&state, "hosted-return", &command).is_ok());
        assert_eq!(metadata("browser").unwrap(), before);
        assert!(check("browser", "click").is_err());
        assert!(check("browser", "screenshot").is_err());
        let mut minimally_addressed = command;
        minimally_addressed.as_object_mut().unwrap().remove("browserId");
        minimally_addressed.as_object_mut().unwrap().remove("profileId");
        assert!(authorize_return_observation(&state, &minimally_addressed).is_ok());
    });
}

#[test]
fn host_control_return_observation_rejects_forged_or_malformed_proof() {
    fixture(|| {
        let state = return_observation_state();
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let original = return_observation_command();
        for (field, value) in [
            ("browserId", json!("foreign-browser")),
            ("taskId", json!("foreign-task")),
            ("profileId", json!("foreign-profile")),
            ("targetId", json!("foreign-target")),
            ("expectedEpoch", json!(0)),
            ("expectedEpoch", json!(-1)),
            ("expectedEpoch", json!("1")),
            ("actor", json!("")),
            ("actor", json!("   ")),
            ("actor", json!("a".repeat(161))),
            ("actor", json!(17)),
            ("reason", json!("")),
            ("reason", json!("   ")),
            ("reason", json!("r".repeat(321))),
            ("reason", json!(false)),
            ("unexpectedAuthority", json!(true)),
        ] {
            let mut forged = original.clone();
            forged["hostControlReturnObservation"][field] = value;
            assert!(authorize_return_observation(&state, &forged).is_err(), "{field}");
            assert!(check_state(&state, "hosted-return", &forged).is_err(), "{field}");
        }
        for field in ["browserId", "taskId", "profileId", "targetId", "expectedEpoch", "actor", "reason"] {
            let mut forged = original.clone();
            forged["hostControlReturnObservation"].as_object_mut().unwrap().remove(field);
            assert!(authorize_return_observation(&state, &forged).is_err(), "missing {field}");
        }
        for malformed in [Value::Null, json!(true), json!([]), json!("proof")] {
            let mut forged = original.clone();
            forged["hostControlReturnObservation"] = malformed;
            assert!(authorize_return_observation(&state, &forged).is_err());
        }
        assert_eq!(metadata("browser").unwrap()["epoch"], 1);
        assert_eq!(metadata("browser").unwrap()["owner"], "human");
    });
}

#[test]
fn host_control_return_proof_never_bypasses_other_actions_or_cross_addressing() {
    fixture(|| {
        let state = return_observation_state();
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let original = return_observation_command();
        for action in ["click", "evaluate", "navigate", "snapshot", "screenshot", "browser_session_attach", "browser_session_reconnect"] {
            let mut command = original.clone();
            command["action"] = json!(action);
            assert!(authorize_return_observation(&state, &command).is_err(), "{action}");
            assert!(check_state(&state, "hosted-return", &command).is_err(), "{action}");
        }
        for (field, value) in [
            ("sessionName", json!("foreign-lane")),
            ("sessionId", json!("foreign-task")),
            ("targetId", json!("foreign-target")),
            ("browserId", json!("foreign-browser")),
            ("profileId", json!("foreign-profile")),
            ("hostControlFreshObservation", json!(false)),
            ("hostControlFreshObservation", json!("true")),
        ] {
            let mut command = original.clone();
            command[field] = value;
            assert!(authorize_return_observation(&state, &command).is_err(), "{field}");
        }
    });
}

#[test]
fn host_control_return_proof_requires_current_hosted_task_browser_profile_and_tab_records() {
    fixture(|| {
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let original = return_observation_state();
        let command = return_observation_command();
        for scenario in 0..11 {
            let mut state = original.clone();
            match scenario {
                0 => { state.host_bindings.remove("task"); }
                1 => { state.sessions.remove("task"); }
                2 => { state.browsers.remove("browser"); }
                3 => { state.tabs.remove("tab"); }
                4 => state.sessions.get_mut("task").unwrap().profile_id = "foreign".into(),
                5 => state.sessions.get_mut("task").unwrap().current_tab_id = None,
                6 => state.browsers.get_mut("browser").unwrap().profile_id = "foreign".into(),
                7 => state.tabs.get_mut("tab").unwrap().target_id = "foreign".into(),
                8 => state.tabs.get_mut("tab").unwrap().browser_id = "foreign".into(),
                9 => state.tabs.get_mut("tab").unwrap().session_id = "foreign".into(),
                _ => state.sessions.get_mut("task").unwrap().browser_id = "foreign".into(),
            }
            assert!(authorize_return_observation(&state, &command).is_err(), "scenario {scenario}");
        }
    });
}

#[test]
fn host_control_return_commit_is_one_attributable_transition_after_paused_observation() {
    fixture(|| {
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        assert!(eligible_return_observation("browser", 1, "returner", "Resume").unwrap());
        assert!(check("browser", "url").is_err());
        assert!(check("browser", "fill").is_err());
        assert_eq!(metadata("browser").unwrap()["epoch"], 1);
        assert_eq!(commit_observed_return("browser", 1, "returner", "Resume").unwrap(), 2);
        let control = read("browser").unwrap();
        assert!(!control.human && !control.pause_reads && !control.awaiting_observation);
        assert_eq!(control.actor, "returner");
        assert_eq!(control.reason, "Resume");
        assert_eq!(control.audit.len(), 2);
        assert_eq!(control.audit[1], json!({"epoch":2,"operation":"return","actor":"returner","reason":"Resume"}));
        assert!(check("browser", "fill").is_ok());
        assert!(check("browser", "url").is_ok());
        assert!(commit_observed_return("browser", 1, "returner", "Resume").is_err());
        assert_eq!(read("browser").unwrap().audit.len(), 2);
    });
}

#[test]
fn host_control_return_commit_cannot_overwrite_concurrent_takeover_or_new_flight() {
    fixture(|| {
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        assert!(eligible_return_observation("browser", 1, "returner", "Resume").unwrap());
        change("browser", "takeover", 1, "other human", "New intent", true).unwrap();
        assert!(commit_observed_return("browser", 1, "returner", "Resume").is_err());
        assert_eq!(metadata("browser").unwrap()["owner"], "human");
        assert_eq!(metadata("browser").unwrap()["actor"], "other human");
        assert_eq!(read("browser").unwrap().audit.len(), 2);
        assert!(eligible_return_observation("browser", 2, "returner", "Resume").unwrap());
        transaction(&path("browser").unwrap(), |control| {
            control.flights.insert("late-effect".into(), "uncertain_in_flight".into());
            Ok(())
        }).unwrap();
        assert!(commit_observed_return("browser", 2, "returner", "Resume").is_err());
        assert_eq!(metadata("browser").unwrap()["epoch"], 2);
        assert!(check("browser", "click").is_err());
    });
}

#[test]
fn host_control_return_concurrent_commits_have_one_cas_winner() {
    fixture(|| {
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let workers = (0..2).map(|_| std::thread::spawn(|| {
            commit_observed_return("browser", 1, "returner", "Resume")
        })).collect::<Vec<_>>();
        let results = workers.into_iter().map(|worker| worker.join().unwrap()).collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(metadata("browser").unwrap()["epoch"], 2);
        assert_eq!(read("browser").unwrap().audit.len(), 2);
    });
}

#[test]
fn host_control_return_rejects_stale_epoch_and_invalid_attribution_without_writing() {
    fixture(|| {
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let before = std::fs::read(path("browser").unwrap()).unwrap();
        for (actor, reason) in [("", "Resume"), ("  ", "Resume"), ("operator", ""), ("operator", "  ")] {
            assert!(eligible_return_observation("browser", 1, actor, reason).is_err());
            assert!(commit_observed_return("browser", 1, actor, reason).is_err());
        }
        assert!(eligible_return_observation("browser", 0, "operator", "Resume").is_err());
        assert!(commit_observed_return("browser", 0, "operator", "Resume").is_err());
        assert_eq!(std::fs::read(path("browser").unwrap()).unwrap(), before);
    });
}

#[test]
fn host_control_return_unresolved_flights_and_legacy_awaiting_keep_existing_reconcile_lane() {
    fixture(|| {
        for (index, outcome) in ["dispatched", "uncertain_in_flight"].iter().enumerate() {
            let browser = format!("pending-{index}");
            change(&browser, "takeover", 0, "human", "Assist", true).unwrap();
            transaction(&path(&browser).unwrap(), |control| {
                control.flights.insert("effect".into(), (*outcome).into());
                Ok(())
            }).unwrap();
            assert!(!eligible_return_observation(&browser, 1, "operator", "Resume").unwrap());
            assert!(commit_observed_return(&browser, 1, "operator", "Resume").is_err());
            let epoch = change(&browser, "return", 1, "operator", "Legacy return", false).unwrap();
            assert!(!eligible_return_observation(&browser, epoch, "operator", "Resume").unwrap());
            assert!(observed(&browser, epoch).is_err());
            assert_eq!(metadata(&browser).unwrap()["owner"], "awaiting_observation");
        }
        change("legacy", "takeover", 0, "human", "Assist", true).unwrap();
        transaction(&path("legacy").unwrap(), |control| {
            control.flights.insert("uncertain".into(), "uncertain_no_replay".into());
            Ok(())
        }).unwrap();
        let epoch = change("legacy", "return", 1, "operator", "Legacy return", false).unwrap();
        assert!(!eligible_return_observation("legacy", epoch, "operator", "Resume").unwrap());
        assert!(commit_observed_return("legacy", epoch, "operator", "Resume").is_err());
        reconcile("legacy", epoch, "uncertain", "operator", "Observed uncertainty").unwrap();
        observed("legacy", epoch).unwrap();
        assert_eq!(read("legacy").unwrap().flights["uncertain"], "observed_uncertain_no_replay");
    });
}

#[test]
fn host_control_return_terminal_uncertainty_is_eligible_without_erasing_no_replay_history() {
    fixture(|| {
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        transaction(&path("browser").unwrap(), |control| {
            control.flights.insert("uncertain".into(), "uncertain_no_replay".into());
            Ok(())
        }).unwrap();
        assert!(eligible_return_observation("browser", 1, "operator", "Resume").unwrap());
        commit_observed_return("browser", 1, "operator", "Resume").unwrap();
        assert_eq!(read("browser").unwrap().flights["uncertain"], "uncertain_no_replay");
        assert!(admit("browser", "click", "uncertain").is_err(), "terminal uncertainty never permits request replay");
    });
}

#[test]
fn host_control_return_admission_accepts_only_the_exact_explicit_observation() {
    fixture(|| {
        let state = return_observation_state();
        BrowserSessionSqliteStore::default_sqlite().unwrap().save_session_state(&state).unwrap();
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let command = return_observation_command();
        for check_only in [false, true] {
            assert!(admit_command(Some("hosted-return"), None, &command, check_only).unwrap().is_none());
            let mut ordinary = command.clone();
            ordinary.as_object_mut().unwrap().remove("hostControlReturnObservation");
            assert!(admit_command(Some("hosted-return"), None, &ordinary, check_only).is_err());
        }
        assert!(read("browser").unwrap().flights.is_empty(), "explicit observation grants no effect admission");
        assert_eq!(metadata("browser").unwrap()["owner"], "human");
    });
}

#[test]
fn host_control_return_unsigned_metadata_cannot_forge_observation() {
    fixture(|| {
        let state = return_observation_state();
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let mut command = return_observation_command();
        command["hostControlReturnObservation"].as_object_mut().unwrap().remove("attestation");
        assert!(authorize_return_observation(&state, &command).is_err(), "status metadata alone cannot authorize an internal return read");
        assert!(check("browser", "url").is_err());
    });
}

#[test]
fn host_control_return_missing_or_nonprivate_key_keeps_legacy_control_available() {
    fixture(|| {
        use std::os::unix::fs::PermissionsExt;
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let key_path = return_key_path("browser").unwrap();
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!eligible_return_observation("browser", 1, "operator", "Resume").unwrap());
        std::fs::remove_file(&key_path).unwrap();
        assert!(!eligible_return_observation("browser", 1, "operator", "Resume").unwrap());
        let epoch = change("browser", "return", 1, "operator", "Resume", false).unwrap();
        observed("browser", epoch).unwrap();
        assert!(!read("browser").unwrap().human);
        assert!(!metadata("browser").unwrap().to_string().contains("return-key"));
    });
}

#[test]
fn host_control_return_another_key_cannot_attest_the_same_status_metadata() {
    fixture(|| {
        let state = return_observation_state();
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let mut command = return_observation_command();
        let proof: ReturnObservation = serde_json::from_value(command["hostControlReturnObservation"].clone()).unwrap();
        let other = agent_browser_lease_authority::CooperativeReadKey::new().unwrap();
        command["hostControlReturnObservation"]["attestation"] = json!(other.attest(&return_observation_payload(&proof).unwrap()));
        assert!(authorize_return_observation(&state, &command).is_err());
        assert!(check("browser", "url").is_err());
        let private_key = serde_json::to_string(&load_return_key("browser").unwrap()).unwrap();
        assert!(!metadata("browser").unwrap().to_string().contains(&private_key));
    });
}

#[cfg(target_os = "linux")]
#[test]
fn host_control_return_fifo_sidecar_cannot_block_takeover_or_legacy_fallback() {
    fixture(|| {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::OpenOptionsExt;
        change("browser", "takeover", 0, "human", "Assist", true).unwrap();
        let key_path = return_key_path("browser").unwrap();
        std::fs::remove_file(&key_path).unwrap();
        let name = std::ffi::CString::new(key_path.as_os_str().as_bytes()).unwrap();
        // SAFETY: this is the exact private disposable fixture path, NUL-terminated.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            sender.send(change("browser", "takeover", 1, "next operator", "Explicit takeover", true)).unwrap();
        });
        let outcome = receiver.recv_timeout(std::time::Duration::from_secs(2));
        // Release the old blocking implementation before asserting RED, so the
        // regression itself never leaves a blocked thread/control lock behind.
        let unblock = if outcome.is_err() {
            Some(OpenOptions::new().read(true).write(true).custom_flags(libc::O_NONBLOCK).open(&key_path).unwrap())
        } else { None };
        worker.join().unwrap();
        drop(unblock);
        assert_eq!(outcome.expect("optional FIFO key must not block the control lock").unwrap(), 2);
        assert!(!eligible_return_observation("browser", 2, "operator", "Resume").unwrap());
        let epoch = change("browser", "return", 2, "operator", "Resume", false).unwrap();
        observed("browser", epoch).unwrap();
        assert!(!read("browser").unwrap().human);
    });
}
