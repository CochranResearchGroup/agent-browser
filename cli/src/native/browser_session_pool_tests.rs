//! Capacity demand through the actual host, pool coordinator and SQLite owner.
use super::super::browser_session_host::BrowserSessionPersistence;
use super::super::browser_session_store::BrowserProfileCatalogLoad;
use super::*;
use std::cell::RefCell;
use std::path::Path;

#[test]
fn cdp_free_idle_retirement_os_close_does_not_request_native_window_context() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_open","sessionName":"Alice","profileId":"profile-a",
        "url":"https://synthetic.example/","manualLogin":true,"activityAtMs":100,
        "serviceName":"fixture","agentName":"fixture","taskName":"OS idle close"
    }));
    assert_eq!(opened["success"], true, "{opened}");
    consumer
        .tab_for_navigation(opened["data"]["sessionId"].as_str().unwrap(), 101)
        .unwrap();
    let mut baseline = consumer.state().clone();
    let browser = baseline.browsers.values().next().unwrap().clone();
    baseline.tabs.values_mut().next().unwrap().target_id = format!("manual-window:{}", browser.id);
    drop(consumer);
    fixture.store(false).save_session_state(&baseline).unwrap();
    let mut effects = RemoteViewSessionEffects::new(
        Process {
            mode: Mode::Success,
            root: fixture.root.clone(),
            calls,
        },
        adapter(&fixture, provider),
        fixture.store(false),
        vec![],
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    effects.begin_operation(&baseline).unwrap();
    let _guard = effects
        .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
        .unwrap()
        .unwrap();
    std::fs::write(
        fixture.root.join("native-close-context-unavailable"),
        "unavailable",
    )
    .unwrap();
    std::fs::write(fixture.root.join("count-retirement-closes"), "count").unwrap();
    effects
        .close_browser(&browser)
        .expect("automatic OS close cannot depend on Native window-close context");
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("retirement-effect-count")).unwrap(),
        "1"
    );
    assert!(fixture.store(false).idle_retirement_records().unwrap()[0]
        .has_settled_browser_close(&browser.id));
}

#[test]
fn cdp_free_idle_retirement_live_original_handoff_supersedes_pending_close() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_open","sessionName":"Alice","profileId":"profile-a",
        "url":"https://synthetic.example/","manualLogin":true,"activityAtMs":100,
        "serviceName":"fixture","agentName":"fixture","taskName":"Human handoff"
    }));
    assert_eq!(opened["success"], true, "{opened}");
    let session_id = opened["data"]["sessionId"].as_str().unwrap();
    consumer.tab_for_navigation(session_id, 101).unwrap();
    let mut baseline = consumer.state().clone();
    let browser = baseline.browsers.values().next().unwrap().clone();
    let tab = baseline.tabs.values_mut().next().unwrap();
    tab.target_id = format!("manual-window:{}", browser.id);
    let tab = tab.clone();
    let link =
        retain_remote_view_tab_handoff_at(&mut baseline, "cdp-free-live", session_id, &tab.id, 101)
            .unwrap();
    drop(consumer);
    fixture.store(false).save_session_state(&baseline).unwrap();
    let intent = claim_fixture_retirement(&fixture, provider.clone(), &baseline, &browser);
    fixture
        .store(false)
        .admit_idle_retirement_preparation(&intent)
        .unwrap();
    let prepared = adapter(&fixture, provider.clone())
        .prepare_idle_retirement(&intent.provider)
        .unwrap();
    let mut record = fixture
        .store(false)
        .idle_retirement_records()
        .unwrap()
        .remove(0);
    let previous = record.clone();
    record.record_prepared(prepared).unwrap();
    fixture
        .store(false)
        .update_idle_retirement(&previous, &record)
        .unwrap();
    fixture
        .store(false)
        .admit_idle_retirement_effect(
            &intent,
            BrowserIdleRetirementEffect::CloseBrowser {
                browser_id: browser.id.clone(),
            },
        )
        .unwrap();
    let attempted = fixture
        .store(false)
        .idle_retirement_records()
        .unwrap()
        .remove(0);
    provider.borrow_mut().window_pid = Some(browser.pid);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let command = serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":link.id,
        "nativeDesktop":true,"activityAtMs":102
    });
    for blocked in [
        "handoff-cleanup-worker-active",
        "activity-readback-unavailable",
    ] {
        std::fs::write(fixture.root.join(blocked), "blocked").unwrap();
        std::fs::write(fixture.root.join("activity-original-readback"), "original").unwrap();
        let refused = restarted.handle_command(&command);
        assert_eq!(refused["success"], false, "{blocked}: {refused}");
        assert_eq!(
            fixture.store(false).idle_retirement_records().unwrap()[0],
            attempted
        );
        std::fs::remove_file(fixture.root.join(blocked)).unwrap();
    }
    let response = restarted.handle_command(&command);
    assert_eq!(
        response["success"], true,
        "live original handoff must recover: {response}"
    );
    assert_eq!(response["data"]["operatorVisible"]["state"], "ready");
    assert_eq!(response["data"]["browserId"], browser.id);
    assert_eq!(response["data"]["tabId"], tab.id);
    assert_eq!(calls.get(), 1, "no replacement launch");
    let completed = fixture
        .store(false)
        .idle_retirement_records()
        .unwrap()
        .remove(0);
    assert!(completed.is_terminal());
    assert!(!completed.has_settled_browser_close(&browser.id));
    assert_eq!(
        serde_json::to_value(&attempted).unwrap()["effects"],
        serde_json::to_value(&completed).unwrap()["effects"],
        "original close outcome remains recorded, not relabeled as no effect"
    );
    assert!(restarted.state().idle_closed_browsers.is_empty());
    let mut delayed = attempted.clone();
    delayed
        .settle_effect(&BrowserIdleRetirementEffect::CloseBrowser {
            browser_id: browser.id.clone(),
        })
        .unwrap();
    assert_eq!(
        fixture
            .store(false)
            .update_idle_retirement(&attempted, &delayed),
        Err(BrowserIdleRetirementStoreError::Conflict),
        "late original cannot overwrite explicit retention"
    );
    assert_eq!(
        restarted.handle_command(&command)["success"],
        true,
        "same-link reconnect remains available"
    );
}

#[test]
fn native_policy_publication_focus_reuses_exact_owner_activity_snapshot() {
    native_policy_focus_cases(false);
}

#[test]
fn native_image_policy_publication_focus_reuses_exact_owner_activity_snapshot() {
    native_policy_focus_cases(true);
}

fn native_policy_focus_cases(image: bool) {
    use super::super::browser_task_operations as operations;
    for case in ["own", "foreign_before", "foreign_after", "epoch_after"] {
        let root =
            std::env::temp_dir().join(format!("native-focus-policy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join(".agent-browser/service")).unwrap();
        let environment = crate::test_utils::EnvGuard::new(&[
            "HOME",
            "AGENT_BROWSER_HOME",
            "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
            "AGENT_BROWSER_CONFIG",
        ]);
        environment.set("HOME", root.to_str().unwrap());
        environment.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
        environment.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
        environment.remove("AGENT_BROWSER_CONFIG");
        let fixture = Fixture::with_root(root.join(".agent-browser/service"), true);
        assert_eq!(
            BrowserSessionSqliteStore::default_sqlite_path().unwrap(),
            fixture.root.join("runtime.sqlite3")
        );
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let opened = consumer
            .open(OpenBrowserSession::exact_profile(
                "Native",
                "profile-a",
                100,
            ))
            .unwrap();
        consumer
            .tab_for_navigation(&opened.session_id, 101)
            .unwrap();
        let mut state = consumer.state().clone();
        drop(consumer);
        let session = state.sessions[&opened.session_id].clone();
        let browser = state.browsers[&opened.browser_id].clone();
        let tab = state.tabs[session.current_tab_id.as_ref().unwrap()].clone();
        let epoch = super::super::browser_host_control::read(&browser.id)
            .unwrap()
            .epoch;
        let identity = fixture
            .store(false)
            .original_browser_identity(&state, &browser);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let mut observation = serde_json::json!({"desktopId":"viewing","generation":9,"windowId":7,"window":{"x":0,"y":0,"width":320,"height":200},"displayWidth":320,"displayHeight":200,"region":{"x":0,"y":0,"width":320,"height":200},"frameSha256":"a".repeat(64),"nativeControlEpoch":4,"observedAtMs":now,"registeredActionId":"hcaptcha-checkbox-p181-v2","family":"hcaptcha","candidateSha256":"b".repeat(64),"x":50,"y":50});
        if image {
            let candidates=(0..9).map(|index|serde_json::json!({"candidateId":format!("tile:{}:{}",index+1,"a".repeat(16)),"bounds":{"x":100+(index%3)*66,"y":200+(index/3)*66,"width":64,"height":64}})).collect::<Vec<_>>();
            observation = serde_json::json!({"desktopId":"viewing","generation":9,"windowId":7,"processDigest":"b".repeat(64),"window":{"x":0,"y":0,"width":640,"height":480},"displayWidth":640,"displayHeight":480,"region":{"x":0,"y":0,"width":640,"height":480},"frameSha256":"a".repeat(64),"nativeControlEpoch":4,"observedAtMs":now,"registeredActionId":super::super::browser_native_image::PROFILE_ID,"puzzleSha256":"","candidates":candidates,"verify":{"x":100,"y":414,"width":80,"height":30}});
            observation["puzzleSha256"] =
                serde_json::json!(super::super::browser_native_image::digest(&(
                    &observation["frameSha256"],
                    &observation["candidates"],
                    &observation["verify"],
                    super::super::browser_native_image::PROFILE_ID
                ))
                .unwrap());
        }
        let row = |id: &str, action: &str, status: &str, refs: Value| {
            serde_json::from_value::<BrowserTaskOperation>(serde_json::json!({"requestId":id,"taskId":session.id,"profileId":session.profile_id,"browserId":browser.id,"tabId":tab.id,"targetId":tab.target_id,"processPid":browser.pid,"processIdentity":identity,"controlEpoch":epoch,"action":action,"fingerprint":"private-fixture","state":status,"createdAtMs":now,"completedAtMs":null,"evidenceReferences":refs})).unwrap()
        };
        state.task_operations.insert(session.id.clone(),std::collections::BTreeMap::from([
        ("capture".into(),row("capture","native_capture","owner_acknowledged",if image {serde_json::json!({"nativeImageObservation":observation.to_string()})} else {serde_json::json!({"nativeObservation":observation.to_string()})})),
        ("attempt".into(),row("attempt","native_input","pending",serde_json::json!({"nativeObservationRequestId":"capture","nativeRegisteredActionId":if image {super::super::browser_native_image::PROFILE_ID}else{"hcaptcha-checkbox-p181-v2"}})))
    ]));
        fixture.store(false).save_session_state(&state).unwrap();
        provider.borrow_mut().window_pid = Some(browser.pid);
        let launches_before = calls.get();
        let mut effects = RemoteViewSessionEffects::new(
            Process {
                mode: Mode::Success,
                root: fixture.root.clone(),
                calls: calls.clone(),
            },
            adapter(&fixture, provider),
            fixture.store(false),
            Vec::new(),
            || uuid::Uuid::new_v4().to_string(),
        )
        .unwrap();
        effects.begin_operation(&state).unwrap();
        let mut consumer = BrowserSessionHost::load(
            fixture.store(false),
            effects,
            &fixture.root.join("unused.json"),
            BrowserSessionHostConfig {
                session_idle_timeout_ms: 300_000,
                remote_view_desktops: Vec::new(),
                default_disposable_policy: None,
                exact_url_history_maximum_bytes: 1024,
            },
        )
        .unwrap();
        let mut command = serde_json::json!({"id":"attempt","action":"native_input","sessionId":session.id,"nativeAttempt":{"observationRequestId":"capture","registeredActionId":"hcaptcha-checkbox-p181-v2"}});
        let binding = serde_json::json!({"sitePolicyId":"owned-site","digest":"d".repeat(64),"providerId":"owned-native-provider","family":"hcaptcha","registeredActionId":if image {super::super::browser_native_image::PROFILE_ID}else{"hcaptcha-checkbox-p181-v2"}});
        if image {
            command = serde_json::json!({"id":"attempt","action":"native_input","sessionId":session.id,"imageAttempt":{"observationRequestId":"capture","selectedCandidateIds":[format!("tile:1:{}","a".repeat(16))]}});
        }
        if case == "foreign_before" {
            fixture
                .store(false)
                .mutate_session_state(|current| {
                    current.tabs.get_mut(&tab.id).unwrap().target_id = "competing-target".into();
                    Ok(())
                })
                .unwrap();
            assert!(operations::retain_native_site_policy_for_owner(
                &mut fixture.store(false),
                &command,
                &binding,
                &state
            )
            .err()
            .unwrap()
            .contains("publication_conflict"));
            let current = fixture.store(false).load_session_state().unwrap();
            assert!(!current.task_operations[&session.id]["attempt"]
                .evidence_references
                .contains_key("nativeSitePolicy"));
            assert_eq!(calls.get(), launches_before);
            continue;
        }
        let publication = operations::retain_native_site_policy_for_owner(
            &mut fixture.store(false),
            &command,
            &binding,
            &state,
        )
        .unwrap();
        if case == "foreign_after" {
            fixture
                .store(false)
                .mutate_session_state(|current| {
                    current.tabs.get_mut(&tab.id).unwrap().target_id = "competing-target".into();
                    Ok(())
                })
                .unwrap();
        }
        if case == "epoch_after" {
            super::super::browser_host_control::change(
                &browser.id,
                "takeover",
                epoch,
                "owned-test",
                "foreign control change after policy publication",
                false,
            )
            .unwrap();
        }
        let target = RemoteViewTabHandoffTarget {
            session,
            browser,
            tab,
        };
        let result = consumer.focus_native_policy_publication(&target, &publication);
        if case == "own" {
            assert!(result.is_ok(),"Exact own policy publication must not invalidate legitimate focus admission: {result:?}");
        } else {
            assert!(
                result.unwrap_err().contains("publication_conflict"),
                "{case}"
            );
            assert!(
                fixture
                    .store(false)
                    .browser_activity_records()
                    .unwrap()
                    .is_empty(),
                "A rejected context cannot gain focus activity"
            );
        }
        assert_eq!(
            calls.get(),
            launches_before,
            "Native focus must not launch or replace a browser"
        );
        let current = fixture.store(false).load_session_state().unwrap();
        assert_eq!(
            current.task_operations[&target.session.id]["attempt"].state,
            BrowserTaskOperationState::Pending
        );
        assert_eq!(
            current.task_operations[&target.session.id]["attempt"].evidence_references
                ["nativeChallengeState"],
            "solving"
        );
    }
}

#[derive(Default)]
struct Provider {
    assignments: Vec<RemoteViewAssignmentRecord>,
    acquisitions: Vec<String>,
    operations: Vec<&'static str>,
    lose_reply: bool,
    view_requests: Vec<String>,
    lose_view_reply: bool,
    competing_view_publication: bool,
    delayed_scoped_view: bool,
    view_issued_at: Option<u64>,
    terminal_view_key: Option<String>,
    all_views_terminal: bool,
    unavailable_assignment: Option<String>,
    denied_assignment: Option<String>,
    empty_desktop: Option<String>,
    occupied_desktops: std::collections::BTreeSet<String>,
    fail_new_windows: bool,
    pool_maximum: Option<u32>,
    window_pid: Option<u32>,
    inactive_window: bool,
    viewer_active_desktops: std::collections::BTreeSet<String>,
    defer_idle_release: bool,
    lose_idle_release_reply: bool,
    idle_release_requests: Vec<String>,
    native_observations: std::collections::BTreeMap<String, Value>,
    native_used_at_ms: Option<u64>,
    native_observation_expired: bool,
    lose_native_registration_reply: bool,
    native_viewing_generation: Option<u64>,
    native_competing_publication: bool,
    native_bridge: Option<NativeOwnerBridge>,
    retirement_receipts: std::collections::BTreeMap<String, Value>,
    lose_retirement_completion: bool,
    lose_retirement_preparation: bool,
    unavailable_idle_retirement_assignment: Option<String>,
}
struct NativeOwnerBridge {
    owner_url: String,
    inventory: Value,
    assignment_observation: Value,
}
struct PoolTransport {
    provider: Rc<RefCell<Provider>>,
    root: PathBuf,
}
impl RemoteViewApplicationTransport for PoolTransport {
    fn request(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError> {
        let mut provider = self.provider.borrow_mut();
        assert_eq!(envelope.application, "agent-browser");
        let mut fixture = wire();
        if let Some(bridge) = &provider.native_bridge {
            if matches!(
                envelope.request,
                RemoteViewApplicationRequest::RegisterNativeHandoffObservation { .. }
                    | RemoteViewApplicationRequest::ObserveNativeHandoffUse { .. }
            ) {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                return runtime.block_on(async {
                    let response = reqwest::Client::builder()
                        .no_proxy()
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .unwrap()
                        .post(&bridge.owner_url)
                        .json(envelope)
                        .send()
                        .await
                        .map_err(|_| RemoteViewApplicationTransportError::Unavailable)?;
                    if !response.status().is_success() {
                        return Err(RemoteViewApplicationTransportError::Unavailable);
                    }
                    response
                        .json()
                        .await
                        .map_err(|_| RemoteViewApplicationTransportError::Unavailable)
                });
            }
            fixture["inventory"] = bridge.inventory.clone();
            fixture["assignmentObservation"] = bridge.assignment_observation.clone();
            fixture["launchEnvironment"]["target"] =
                bridge.assignment_observation["target"].clone();
            fixture["launchEnvironment"]["environment"]["REMOTE_VIEW_SLOT_GENERATION"] = bridge
                .assignment_observation["target"]["viewingGeneration"]
                .to_string()
                .into();
            fixture["windows"]["generation"] =
                bridge.assignment_observation["target"]["lifecycleGeneration"].clone();
            fixture["windows"]["viewingGeneration"] =
                bridge.assignment_observation["target"]["viewingGeneration"].clone();
        }
        match &envelope.request {
            RemoteViewApplicationRequest::RegisterNativeHandoffObservation {
                assignment_id,
                expected_generation,
                expected_viewing_generation,
                target_digest,
                idempotency_key,
                lifetime_ms,
            } => {
                use sha2::{Digest, Sha256};
                provider
                    .operations
                    .push("register_native_handoff_observation");
                let state = open_store(&self.root, false).load_session_state().unwrap();
                assert!(
                    state.remote_view_tab_handoffs.values().any(|link| link
                        .native_observation
                        .as_ref()
                        .is_some_and(|binding| binding.idempotency_key == *idempotency_key
                            && binding.target_digest == *target_digest)),
                    "provider registration must follow durable intent"
                );
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|a| a.assignment_id == *assignment_id)
                    .unwrap();
                let mut target = fixture["assignmentObservation"]["target"].clone();
                target["assignmentId"] = assignment.assignment_id.clone().into();
                target["desktopId"] = assignment.desktop_id.clone().into();
                if assignment.desktop_id != fixture["assignment"]["desktopId"] {
                    target["viewingDesktopId"] = assignment.desktop_id.clone().into();
                }
                if let Some(generation) = provider.native_viewing_generation {
                    target["viewingGeneration"] = generation.into();
                }
                assert_eq!(target["lifecycleGeneration"], *expected_generation);
                assert_eq!(target["viewingGeneration"], *expected_viewing_generation);
                let mut hash = Sha256::new();
                hash.update((envelope.application.len() as u64).to_be_bytes());
                hash.update(envelope.application.as_bytes());
                hash.update(idempotency_key.as_bytes());
                let record = provider.native_observations.entry(idempotency_key.clone()).or_insert_with(|| serde_json::json!({
                    "correlationId":format!("{:x}", Sha256::digest(idempotency_key.as_bytes())),
                    "requestDigest":format!("{:x}",hash.finalize()),"application":envelope.application,
                    "target":target,"targetDigest":target_digest,"clock":{"bootId":"synthetic-boot","elapsedMs":100},
                    "lifetimeMs":lifetime_ms,"used":null,
                })).clone();
                if provider.lose_native_registration_reply {
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                Ok(serde_json::json!({"schemaVersion":1,"observation":record}))
            }
            RemoteViewApplicationRequest::ObserveNativeHandoffUse {
                correlation_id,
                target_digest,
            } => {
                provider.operations.push("observe_native_handoff_use");
                let used_at = provider.native_used_at_ms;
                let expired = provider.native_observation_expired;
                if provider.native_competing_publication {
                    provider.native_competing_publication = false;
                    let mut store = open_store(&self.root, false);
                    let before = store.load_session_state().unwrap();
                    let mut after = before.clone();
                    after.next_session_sequence += 1;
                    store
                        .compare_and_save_session_state(&before, &after)
                        .unwrap();
                }
                let record = provider
                    .native_observations
                    .values_mut()
                    .find(|record| record["correlationId"] == *correlation_id)
                    .ok_or(RemoteViewApplicationTransportError::NativeObservationMissing)?;
                assert_eq!(record["targetDigest"], *target_digest);
                if record["used"].is_null() && !expired {
                    if let Some(at) = used_at {
                        record["used"] = serde_json::json!({"recordedAtMs":at,"evidence":{"frameSync":12,"context":{
                            "correlationId":correlation_id,"operator":"native-operator","viewerConnectionId":"native-connection",
                            "desktopId":record["target"]["viewingDesktopId"],"lifecycleGeneration":record["target"]["lifecycleGeneration"],
                            "viewingGeneration":record["target"]["viewingGeneration"],"targetDigest":target_digest
                        }}});
                    }
                }
                Ok(
                    serde_json::json!({"schemaVersion":1,"state":if !record["used"].is_null() {"used"} else if expired {"expired"} else {"pending"},"observation":record}),
                )
            }
            RemoteViewApplicationRequest::ReadIdleRetirement { assignment_id, .. } => {
                provider.operations.push("read_idle_retirement");
                if provider.unavailable_idle_retirement_assignment.as_ref() == Some(assignment_id) {
                    return Err(RemoteViewApplicationTransportError::Unavailable);
                }
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|a| &a.assignment_id == assignment_id)
                    .unwrap();
                let mut target = fixture["assignmentObservation"]["target"].clone();
                target["assignmentId"] = assignment.assignment_id.clone().into();
                target["desktopId"] = assignment.desktop_id.clone().into();
                if assignment.desktop_id != fixture["assignment"]["desktopId"] {
                    target["viewingDesktopId"] = assignment.desktop_id.clone().into();
                }
                if let Some(generation) = provider.native_viewing_generation {
                    target["viewingGeneration"] = generation.into();
                }
                let retirement = provider.retirement_receipts.get(assignment_id).map_or_else(
                    || serde_json::json!({"revision":0,"fence":null,"completed":true}),
                    |receipt| {
                        serde_json::json!({"revision":receipt["fence"]["revision"],
                        "fence":receipt["fence"],"completed":receipt["state"] == "completed"})
                    },
                );
                Ok(
                    serde_json::json!({"schemaVersion":1,"target":target,"readinessScope":"native_admission","retirement":retirement}),
                )
            }
            RemoteViewApplicationRequest::PrepareIdleRetirement {
                assignment_id,
                idempotency_key,
                expected_revision,
                expected_retirement,
                ..
            } => {
                use sha2::{Digest, Sha256};
                provider.operations.push("prepare_idle_retirement");
                let records = fixture_retirement_records(&self.root);
                let record = records
                    .iter()
                    .find(|r| r.intent.provider.idempotency_key == *idempotency_key)
                    .unwrap();
                assert!(
                    record.preparation_started(),
                    "intent and attempt must precede provider prepare"
                );
                assert!(record.prepared.is_none());
                let target = &record.intent.provider.target;
                let key = format!(
                    "consumer-{:x}",
                    Sha256::digest(
                        format!("{}:{}", target.registration_id, idempotency_key).as_bytes()
                    )
                );
                let operation = format!(
                    "{:x}",
                    Sha256::digest(
                        serde_json::to_vec(&(key, target, expected_retirement)).unwrap()
                    )
                );
                let receipt = serde_json::json!({"schemaVersion":1,"target":target,"readinessScope":"native_admission",
                    "state":"prepared","fence":{"schema_version":1,"installation_id":"0123456789abcdef",
                        "config_hash":"a".repeat(64),"slot":2,"generation":target.viewing_generation,
                        "operation_id":operation,"revision":expected_revision + 1}});
                provider
                    .retirement_receipts
                    .insert(assignment_id.clone(), receipt.clone());
                if self.root.join("require-retirement-custody").exists() {
                    std::fs::write(self.root.join("native-retirement-fence"), "held").unwrap();
                }
                if provider.lose_retirement_preparation {
                    provider.lose_retirement_preparation = false;
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                Ok(receipt)
            }
            RemoteViewApplicationRequest::CompleteIdleRetirement {
                assignment_id,
                idempotency_key,
                fence,
                effects_settled,
                ..
            } => {
                provider.operations.push("complete_idle_retirement");
                assert!(*effects_settled);
                let records = fixture_retirement_records(&self.root);
                let record = records
                    .iter()
                    .find(|r| r.intent.provider.idempotency_key == *idempotency_key)
                    .unwrap();
                assert!(
                    record.published_state_digest.is_some(),
                    "native completion preceded atomic publication"
                );
                assert_eq!(&record.prepared.as_ref().unwrap().fence, fence);
                let receipt = provider.retirement_receipts.get_mut(assignment_id).unwrap();
                assert_eq!(receipt["fence"], serde_json::to_value(fence).unwrap());
                receipt["state"] = "completed".into();
                if self.root.join("require-retirement-custody").exists() {
                    std::fs::remove_file(self.root.join("native-retirement-fence")).unwrap();
                }
                let receipt = receipt.clone();
                if provider.lose_retirement_completion {
                    provider.lose_retirement_completion = false;
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                Ok(receipt)
            }
            RemoteViewApplicationRequest::ObserveIdle { assignment_id, .. } => {
                if self.root.join("check-retirement-read-exclusion").exists() {
                    use super::super::service_store::{
                        LockedServiceStateRepository, ServiceStateRepository,
                    };
                    let repository = LockedServiceStateRepository::default_json()
                        .unwrap()
                        .with_lock_timeout(std::time::Duration::from_millis(30));
                    repository.load_snapshot().expect(
                        "native observation must not hold foreground Service State exclusion",
                    );
                    if let Ok(browser_id) = std::fs::read_to_string(
                        self.root.join("publish-controller-during-idle-read"),
                    ) {
                        repository
                            .mutate(|state| {
                                state.viewer_leases.insert(
                                    "late-current-controller".into(),
                                    agent_browser_service_model::ViewerLease {
                                        id: "late-current-controller".into(),
                                        browser_id: Some(browser_id.clone()),
                                        viewer_role: "controller".into(),
                                        state: "active".into(),
                                        ..Default::default()
                                    },
                                );
                                Ok(())
                            })
                            .unwrap();
                    }
                }
                provider.operations.push("observe_idle");
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|a| &a.assignment_id == assignment_id)
                    .unwrap();
                let mut observed = fixture["assignmentObservation"].clone();
                observed["target"]["assignmentId"] = assignment.assignment_id.clone().into();
                observed["target"]["desktopId"] = assignment.desktop_id.clone().into();
                if assignment.desktop_id != fixture["assignment"]["desktopId"] {
                    observed["target"]["viewingDesktopId"] = assignment.desktop_id.clone().into();
                }
                let viewer = provider
                    .viewer_active_desktops
                    .contains(&assignment.desktop_id);
                observed["retirement"] = serde_json::json!({"schemaVersion":1,"desktopId":assignment.desktop_id,
                    "generation":assignment.generation,"routes":[],"sessions":[]});
                observed["idle"] = serde_json::json!(
                    !viewer
                        && !provider.occupied_desktops.contains(&assignment.desktop_id)
                        && provider.window_pid.is_none()
                );
                observed["viewerActive"] = viewer.into();
                Ok(observed)
            }
            RemoteViewApplicationRequest::ReleaseIdle {
                assignment_id,
                expected_retirement,
                idempotency_key,
                ..
            } => {
                provider.operations.push("release_idle");
                provider.idle_release_requests.push(idempotency_key.clone());
                let defer = provider.defer_idle_release;
                let assignment = provider
                    .assignments
                    .iter_mut()
                    .find(|a| &a.assignment_id == assignment_id)
                    .unwrap();
                if defer {
                    return Ok(
                        serde_json::json!({"schemaVersion":1,"operation":"release_idle","state":"retained",
                        "assignmentId":assignment_id,"generation":assignment.generation}),
                    );
                }
                assignment.state = RemoteViewAssignmentState::Released;
                let outcome =
                    serde_json::json!({"assignment":assignment,"retirement":expected_retirement});
                if provider.lose_idle_release_reply {
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                Ok(outcome)
            }
            RemoteViewApplicationRequest::Inventory {} => {
                provider.operations.push("inventory");
                let mut inventory = fixture["inventory"].clone();
                if let Some(maximum) = provider.pool_maximum {
                    inventory["policy"]["pools"]["main"]["maximum_reserved"] = maximum.into();
                }
                inventory["assignments"] = serde_json::to_value(&provider.assignments).unwrap();
                inventory["pools"][0]["desktopMembers"] = serde_json::json!(provider
                    .assignments
                    .iter()
                    .map(|assignment| &assignment.desktop_id)
                    .collect::<Vec<_>>());
                Ok(inventory)
            }
            RemoteViewApplicationRequest::Acquire {
                pool_name,
                idempotency_key,
            } => {
                assert_eq!(pool_name, "main");
                let mut store = open_store(&self.root, false);
                let pending = store
                    .pending_pool_acquisitions("agent-browser", "main")
                    .unwrap();
                assert_eq!(pending.len(), 1);
                assert_eq!(pending[0].envelope, *envelope);
                provider.operations.push("acquire");
                if let Some(index) = provider
                    .acquisitions
                    .iter()
                    .position(|key| key == idempotency_key)
                {
                    if provider.lose_reply {
                        return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                    }
                    return Ok(serde_json::to_value(&provider.assignments[index]).unwrap());
                }
                provider.acquisitions.push(idempotency_key.clone());
                let mut assignment: RemoteViewAssignmentRecord =
                    serde_json::from_value(fixture["assignment"].clone()).unwrap();
                if !provider.assignments.is_empty() {
                    assignment.assignment_id =
                        format!("assignment-extra-{}", provider.assignments.len());
                    assignment.desktop_id = uuid::Uuid::new_v4().to_string();
                }
                provider.assignments.push(assignment.clone());
                if provider.lose_reply {
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                Ok(serde_json::to_value(assignment).unwrap())
            }
            RemoteViewApplicationRequest::ObserveAssignment {
                assignment_id,
                expected_generation,
            } => {
                provider.operations.push("observe_assignment");
                if provider.denied_assignment.as_ref() == Some(assignment_id) {
                    return Err(RemoteViewApplicationTransportError::OwnerAccessDenied);
                }
                if provider.unavailable_assignment.as_ref() == Some(assignment_id) {
                    return Err(RemoteViewApplicationTransportError::Unavailable);
                }
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|assignment| assignment.assignment_id == *assignment_id)
                    .unwrap();
                assert_eq!(assignment.generation, *expected_generation);
                if assignment.state != RemoteViewAssignmentState::Active {
                    return Err(RemoteViewApplicationTransportError::OwnerAccessDenied);
                }
                let mut observation = fixture["assignmentObservation"].clone();
                observation["target"]["assignmentId"] = assignment.assignment_id.clone().into();
                observation["target"]["desktopId"] = assignment.desktop_id.clone().into();
                if assignment.desktop_id != fixture["assignment"]["desktopId"] {
                    observation["target"]["viewingDesktopId"] =
                        assignment.desktop_id.clone().into();
                }
                if let Some(generation) = provider.native_viewing_generation {
                    observation["target"]["viewingGeneration"] = generation.into();
                }
                Ok(observation)
            }
            RemoteViewApplicationRequest::Windows { assignment_id, .. } => {
                provider.operations.push("windows");
                if provider.fail_new_windows
                    && provider
                        .assignments
                        .first()
                        .is_some_and(|first| first.assignment_id != *assignment_id)
                {
                    return Err(RemoteViewApplicationTransportError::Unavailable);
                }
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|assignment| assignment.assignment_id == *assignment_id)
                    .unwrap();
                let mut windows = fixture["windows"].clone();
                windows["desktopId"] = assignment.desktop_id.clone().into();
                #[cfg(target_os = "linux")]
                if self.root.join("copy-process-fixture").exists() {
                    let observed: Option<Value> = std::fs::read(self.root.join(format!(
                        "copy-process-observation-{}.json",
                        assignment.desktop_id
                    )))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok());
                    let identity = observed
                        .as_ref()
                        .and_then(|value| value.get("processIdentity"));
                    let pid = identity
                        .and_then(|value| value["pid"].as_u64())
                        .map(|pid| pid as u32);
                    if observed
                        .as_ref()
                        .is_some_and(|value| value["desktopId"] == assignment.desktop_id)
                        && pid.is_some_and(|pid| {
                            matches!(
                                crate::process_identity::observe_process(pid),
                                crate::process_identity::ProcessObservation::Observed(_)
                            )
                        })
                    {
                        windows["windows"][0]["pid"] = serde_json::json!(pid.unwrap());
                    } else {
                        windows["windows"] = serde_json::json!([]);
                    }
                    return Ok(windows);
                }

                if let Some(generation) = provider.native_viewing_generation {
                    windows["viewingGeneration"] = generation.into();
                }
                let composed_pid = provider.native_bridge.as_ref().and_then(|_| {
                    std::fs::read_to_string(self.root.join("native-composition-browser-pid"))
                        .ok()
                        .map(|pid| pid.parse::<u32>().unwrap())
                });
                if let Some(pid) = composed_pid.or(provider.window_pid) {
                    windows["windows"][0]["pid"] = serde_json::json!(pid);
                }
                if provider.inactive_window {
                    windows["windows"][0]["active"] = serde_json::json!(false);
                }
                if provider.empty_desktop.as_ref() == Some(&assignment.desktop_id)
                    || (provider.window_pid.is_none()
                        && !provider.occupied_desktops.contains(&assignment.desktop_id))
                {
                    windows["windows"] = serde_json::json!([]);
                }
                Ok(windows)
            }
            RemoteViewApplicationRequest::LaunchEnvironment { assignment_id, .. } => {
                provider.operations.push("launch_environment");
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|assignment| assignment.assignment_id == *assignment_id)
                    .unwrap();
                let mut environment = fixture["launchEnvironment"].clone();
                environment["target"]["assignmentId"] = assignment.assignment_id.clone().into();
                environment["target"]["desktopId"] = assignment.desktop_id.clone().into();
                if assignment.desktop_id != fixture["assignment"]["desktopId"] {
                    environment["target"]["viewingDesktopId"] =
                        assignment.desktop_id.clone().into();
                }
                Ok(environment)
            }
            RemoteViewApplicationRequest::IssueView {
                assignment_id,
                idempotency_key,
                audience,
                capability,
                lifetime_seconds,
                ..
            } => {
                assert_eq!(audience, "remote_view");
                assert_eq!(*capability, RemoteViewApplicationViewCapability::Control);
                assert_eq!(*lifetime_seconds, 300);
                let store = open_store(&self.root, false);
                assert!(store
                    .load_session_state()
                    .unwrap()
                    .remote_view_tab_handoffs
                    .values()
                    .any(|record| record
                        .view
                        .as_ref()
                        .is_some_and(|view| view.idempotency_key == *idempotency_key
                            && view.issuance.is_none())));
                provider.operations.push("issue_view");
                provider.view_requests.push(idempotency_key.clone());
                if provider.competing_view_publication {
                    competing_write(&self.root);
                }
                if provider.all_views_terminal
                    || provider.terminal_view_key.as_ref() == Some(idempotency_key)
                {
                    return Err(RemoteViewApplicationTransportError::ViewGrantTerminal);
                }
                if provider.lose_view_reply {
                    return Err(RemoteViewApplicationTransportError::OutcomeUnknown);
                }
                let mut issuance = fixture["viewIssuance"].clone();
                let assignment = provider
                    .assignments
                    .iter()
                    .find(|assignment| assignment.assignment_id == *assignment_id)
                    .unwrap();
                let target = &mut issuance["grant"]["request"]["target"];
                target["assignmentId"] = assignment.assignment_id.clone().into();
                target["desktopId"] = assignment.desktop_id.clone().into();
                if assignment.desktop_id != fixture["assignment"]["desktopId"] {
                    target["viewingDesktopId"] = assignment.desktop_id.clone().into();
                }
                issuance["grant"]["request"]["idempotencyKey"] = if provider.delayed_scoped_view {
                    format!("provider-scoped-{idempotency_key}")
                } else {
                    idempotency_key.clone()
                }
                .into();
                if provider.delayed_scoped_view {
                    issuance["grant"]["issuedAt"] = 2001.into();
                    issuance["grant"]["expiresAt"] = 302001.into();
                }
                if let Some(now) = provider.view_issued_at {
                    issuance["grant"]["issuedAt"] = now.into();
                    issuance["grant"]["expiresAt"] = (now + 300_000).into();
                }
                Ok(issuance)
            }
            RemoteViewApplicationRequest::ResolveView { route_id, audience } => {
                provider.operations.push("resolve_view");
                assert_eq!(audience, "remote_view");
                assert_eq!(route_id, fixture["grant"]["routeId"].as_str().unwrap());
                let mut grant = fixture["grant"].clone();
                let key = provider.view_requests.last().unwrap();
                grant["request"]["idempotencyKey"] = if provider.delayed_scoped_view {
                    format!("provider-scoped-{key}")
                } else {
                    key.clone()
                }
                .into();
                if provider.delayed_scoped_view {
                    grant["issuedAt"] = 2001.into();
                    grant["expiresAt"] = 302001.into();
                }
                Ok(grant)
            }
            _ => panic!("unexpected provider mutation during capacity demand"),
        }
    }
}
fn claim_fixture_retirement(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
    baseline: &BrowserSessionState,
    browser: &ManagedBrowserInstance,
) -> BrowserIdleRetirementIntent {
    let intent = fixture_retirement_intent(fixture, provider, baseline, browser);
    fixture
        .store(false)
        .claim_idle_retirement(&intent, baseline)
        .unwrap();
    intent
}

fn fixture_retirement_intent(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
    baseline: &BrowserSessionState,
    browser: &ManagedBrowserInstance,
) -> BrowserIdleRetirementIntent {
    let assignment = fixture
        .store(false)
        .published_launch_assignment(browser)
        .unwrap();
    let mut coordinator = adapter(fixture, provider);
    let idle = coordinator.observe_idle(&assignment).unwrap();
    let admission = coordinator.read_idle_retirement(&assignment).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let native = RemoteViewIdleRetirementIntent::new(&assignment, &idle, &admission, &id).unwrap();
    let mut intent =
        BrowserIdleRetirementIntent::new(&id, native, baseline, std::slice::from_ref(&browser.id))
            .unwrap();
    for scope in &mut intent.scopes {
        scope.process_identity = fixture
            .store(false)
            .published_launch_process_identity(&scope.browser)
            .unwrap();
    }
    intent
}

fn fixture_retirement_records(root: &Path) -> Vec<BrowserIdleRetirementRecord> {
    open_store(root, false).idle_retirement_records().unwrap()
}
fn adapter(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
) -> RemoteViewApplicationAdapter<PoolTransport> {
    RemoteViewApplicationAdapter::new(
        "agent-browser".into(),
        PoolTransport {
            provider,
            root: fixture.root.clone(),
        },
    )
    .unwrap()
}
fn host(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
    calls: Rc<Cell<u32>>,
) -> BrowserSessionHost<
    BrowserSessionSqliteStore,
    RemoteViewSessionEffects<Process, PoolTransport, BrowserSessionSqliteStore>,
> {
    host_mode(fixture, provider, calls, Mode::Success)
}
fn host_mode(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
    calls: Rc<Cell<u32>>,
    mode: Mode,
) -> BrowserSessionHost<
    BrowserSessionSqliteStore,
    RemoteViewSessionEffects<Process, PoolTransport, BrowserSessionSqliteStore>,
> {
    host_mode_clock(fixture, provider, calls, mode, || 2001)
}
fn host_mode_clock(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
    calls: Rc<Cell<u32>>,
    mode: Mode,
    clock: fn() -> u64,
) -> BrowserSessionHost<
    BrowserSessionSqliteStore,
    RemoteViewSessionEffects<Process, PoolTransport, BrowserSessionSqliteStore>,
> {
    host_with_persistence(fixture, provider, calls, mode, clock, fixture.store(false))
}
fn host_with_persistence<P: BrowserSessionPersistence>(
    fixture: &Fixture,
    provider: Rc<RefCell<Provider>>,
    calls: Rc<Cell<u32>>,
    mode: Mode,
    clock: fn() -> u64,
    persistence: P,
) -> BrowserSessionHost<
    P,
    RemoteViewSessionEffects<Process, PoolTransport, BrowserSessionSqliteStore>,
> {
    let effects = RemoteViewSessionEffects::new(
        Process {
            mode,
            root: fixture.root.clone(),
            calls,
        },
        adapter(fixture, provider),
        fixture.store(false),
        Vec::new(),
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap()
    .with_pool(RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    })
    .unwrap()
    .with_view_clock(clock);
    BrowserSessionHost::load(
        persistence,
        effects,
        &fixture.root.join("unused.json"),
        BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: Vec::new(),
            default_disposable_policy: Some(BrowserDisposableProfilePolicy {
                id: "default".into(),
                user_data_root: fixture
                    .root
                    .join("disposable")
                    .to_string_lossy()
                    .into_owned(),
                cleanup_delay_ms: 300_000,
                maximum_retained_profiles: 20,
                maximum_total_bytes: 1024 * 1024,
            }),
            exact_url_history_maximum_bytes: 1024,
        },
    )
    .unwrap()
}

#[test]
fn pool_demand_empty_open_acquires_once_and_status_reuse_restart_do_not_allocate() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    assert!(provider.borrow().operations.is_empty());
    let status = consumer.handle_command(&serde_json::json!({"action":"browser_session_status"}));
    assert_eq!(status["success"], true);
    assert!(provider.borrow().operations.is_empty());
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert_eq!(calls.get(), 1);
    let requests = provider.borrow().operations.len();
    let reused = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-a", 101))
        .unwrap();
    assert_eq!(reused.browser_id, opened.browser_id);
    assert_eq!(provider.borrow().operations.len(), requests);
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    restarted
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 102))
        .unwrap();
    assert_eq!(provider.borrow().operations.len(), requests);
    assert_eq!(calls.get(), 1);
    let first_desktop = provider.borrow().assignments[0].desktop_id.clone();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(first_desktop);
    let other = restarted
        .open(OpenBrowserSession::exact_profile("Other", "profile-b", 103))
        .unwrap();
    assert_ne!(
        restarted.state().browsers[&opened.browser_id].desktop,
        restarted.state().browsers[&other.browser_id].desktop
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    assert_eq!(calls.get(), 2);
    let mut store = fixture.store(false);
    assert!(store
        .pending_pool_acquisitions("agent-browser", "main")
        .unwrap()
        .is_empty());
    assert!(store.unpublished_launch_records().unwrap().is_empty());
}

#[test]
fn pool_demand_lost_acquire_reply_recovers_same_assignment_after_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        lose_reply: true,
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        consumer
            .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
            .unwrap_err(),
        "remote_view_pool_acquisition_readback_required"
    );
    assert_eq!(provider.borrow().assignments.len(), 1);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    drop(consumer);
    provider.borrow_mut().lose_reply = false;
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    restarted
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 101))
        .unwrap();
    assert_eq!(provider.borrow().assignments.len(), 1);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert_eq!(calls.get(), 1);
    assert!(fixture
        .store(false)
        .pending_pool_acquisitions("agent-browser", "main")
        .unwrap()
        .is_empty());
}

#[test]
fn pool_demand_reconciles_lost_then_released_acquisition_before_launch() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        lose_reply: true,
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        consumer
            .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
            .unwrap_err(),
        "remote_view_pool_acquisition_readback_required"
    );
    assert_eq!(calls.get(), 0);
    drop(consumer);
    let original = fixture
        .store(false)
        .pending_pool_acquisitions("agent-browser", "main")
        .unwrap()
        .remove(0);
    provider.borrow_mut().assignments[0].state = RemoteViewAssignmentState::Released;
    provider.borrow_mut().lose_reply = false;
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    restarted
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 101))
        .unwrap();
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    assert_ne!(
        provider.borrow().acquisitions[0],
        provider.borrow().acquisitions[1]
    );
    assert_eq!(
        calls.get(),
        1,
        "only the new active assignment may launch a browser"
    );
    let mut store = fixture.store(false);
    assert!(store
        .pending_pool_acquisitions("agent-browser", "main")
        .unwrap()
        .is_empty());
    let terminal = store.read(&original.envelope).unwrap().unwrap();
    assert_eq!(terminal.envelope, original.envelope);
    assert!(
        matches!(terminal.outcome, Some(RemoteViewApplicationMutationOutcome::Acquire(assignment)) if assignment == provider.borrow().assignments[0] && assignment.state == RemoteViewAssignmentState::Released)
    );
}

#[test]
fn pool_demand_policy_bounds_growth_and_completed_absence_requires_release_proof() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    assert_eq!(
        prepare_remote_view_session_pool(
            &mut adapter,
            &mut store,
            &RemoteViewSessionPool {
                name: "main".into(),
                desired_desktops: 3
            }
        )
        .err()
        .unwrap(),
        "remote_view_pool_demand_exceeds_policy"
    );
    assert!(provider.borrow().acquisitions.is_empty());
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 2,
    };
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(prepared.desktops.len(), 2);
    let keys = provider.borrow().acquisitions.clone();
    assert_eq!(keys.len(), 2);
    assert_ne!(keys[0], keys[1]);
    drop(store);
    let mut store = fixture.store(false);
    prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(provider.borrow().acquisitions, keys);
    provider.borrow_mut().assignments.clear();
    assert_eq!(
        prepare_remote_view_session_pool(&mut adapter, &mut store, &pool)
            .err()
            .unwrap(),
        "remote_view_pool_acquisition_readback_required"
    );
    assert_eq!(provider.borrow().acquisitions, keys);
}

#[test]
fn pool_demand_acquires_a_free_desktop_for_an_independent_browser() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let first = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    let occupied = first.desktops[0].desktop.desktop_id.clone();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(occupied.clone());
    let second = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(
        provider.borrow().acquisitions.len(),
        2,
        "independent browser demand must grow beyond the warm minimum"
    );
    assert!(second
        .desktops
        .iter()
        .any(|candidate| candidate.desktop.desktop_id != occupied));
    provider.borrow_mut().occupied_desktops.extend(
        second
            .assignments
            .iter()
            .map(|assignment| assignment.desktop_id.clone()),
    );
    let overflow = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(
        overflow.desktops.len(),
        2,
        "at the growth limit occupied healthy desktops remain eligible for wrap-around"
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn pool_demand_unknown_new_desktop_reuses_healthy_peer_without_another_acquisition() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        pool_maximum: Some(3),
        ..Provider::default()
    }));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let first = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(first.assignments[0].desktop_id.clone());
    provider.borrow_mut().fail_new_windows = true;
    let continued = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(continued.desktops.len(), 1);
    assert_eq!(
        continued.desktops[0].desktop.desktop_id,
        first.assignments[0].desktop_id
    );
    assert_eq!(
        provider.borrow().acquisitions.len(),
        2,
        "failed new observation must not acquire a third desktop"
    );
}

#[test]
fn pool_request_heads_share_unsubmitted_and_pending_identity_across_connections() {
    let fixture = Fixture::new();
    let mut first = fixture.store(false);
    let mut peer = fixture.store(false);
    let key = first
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .unwrap();
    assert_eq!(
        peer.pool_acquisition_request_key("agent-browser", "main", &[])
            .unwrap(),
        key
    );
    let envelope = RemoteViewApplicationEnvelope {
        application: "agent-browser".into(),
        request: RemoteViewApplicationRequest::Acquire {
            pool_name: "main".into(),
            idempotency_key: key.clone(),
        },
    };
    assert_eq!(
        first.claim(&envelope).unwrap(),
        RemoteViewApplicationMutationClaim::New
    );
    assert_eq!(
        peer.pool_acquisition_request_key("agent-browser", "main", &[])
            .unwrap(),
        key
    );
    assert!(matches!(
        peer.claim(&envelope).unwrap(),
        RemoteViewApplicationMutationClaim::Existing(_)
    ));
    assert_eq!(
        peer.pending_pool_acquisitions("agent-browser", "main")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn pool_request_advances_after_exact_completed_retirement_and_preserves_history() {
    use RemoteViewApplicationObligationInventory as Inventory;
    let fixture = Fixture::new();
    let mut store = fixture.store(false);
    let key = store
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .unwrap();
    let envelope = RemoteViewApplicationEnvelope {
        application: "agent-browser".into(),
        request: RemoteViewApplicationRequest::Acquire {
            pool_name: "main".into(),
            idempotency_key: key.clone(),
        },
    };
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(wire()["assignment"].clone()).unwrap();
    store.claim(&envelope).unwrap();
    store
        .complete(
            &envelope,
            &RemoteViewApplicationMutationOutcome::Acquire(assignment.clone()),
        )
        .unwrap();
    assert!(store
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .is_err());
    let target = RemoteViewApplicationReleaseTarget {
        assignment: assignment.clone(),
        route_ids: Vec::new(),
        viewer_session_ids: Vec::new(),
    };
    let snapshot = RemoteViewApplicationCleanupSnapshot {
        schema_version: 1,
        assignment_id: assignment.assignment_id.clone(),
        desktop_id: assignment.desktop_id.clone(),
        lifecycle_generation: assignment.generation,
        pending_recovery: Inventory::Complete(Vec::new()),
        foreground_leases: Inventory::Complete(Vec::new()),
        cleanup_tasks: Inventory::Complete(Vec::new()),
    };
    let release_id = uuid::Uuid::new_v4().to_string();
    store
        .admit_assignment_release(&target, &snapshot, &release_id)
        .unwrap();
    assert!(store
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .is_err());
    let mut released = assignment.clone();
    released.state = RemoteViewAssignmentState::Released;
    let outcome = RemoteViewJoinedReleaseOutcome {
        assignment: released,
        retirement: RemoteViewDesktopViewingRetirement {
            schema_version: 1,
            desktop_id: assignment.desktop_id,
            generation: assignment.generation,
            routes: Vec::new(),
            sessions: Vec::new(),
        },
    };
    store
        .complete_assignment_release(&target, &release_id, &outcome)
        .unwrap();
    drop(store);
    let mut restarted = fixture.store(false);
    let next = restarted
        .pool_acquisition_request_key("agent-browser", "main", &[])
        .unwrap();
    assert_ne!(next, key);
    assert!(
        matches!(restarted.claim(&envelope).unwrap(), RemoteViewApplicationMutationClaim::Existing(record) if record.outcome.is_some())
    );
}

#[test]
fn pool_demand_excludes_occupied_desktops_and_retains_their_assignments() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 2,
    };
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    let first = prepared.assignments[0].clone();
    let second = prepared.assignments[1].clone();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(first.desktop_id.clone());
    provider.borrow_mut().empty_desktop = Some(second.desktop_id.clone());
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(prepared.desktops[0].desktop.desktop_id, second.desktop_id);
    assert_eq!(prepared.desktops.len(), 1);
    assert_eq!(prepared.assignments.len(), 2);
    provider.borrow_mut().empty_desktop = None;
    provider.borrow_mut().unavailable_assignment = Some(first.assignment_id);
    let prepared = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_eq!(prepared.desktops.len(), 1);
    assert_eq!(prepared.desktops[0].desktop.desktop_id, second.desktop_id);
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn pool_demand_skips_pending_retirement_without_settling_its_reservation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let baseline = consumer.state().clone();
    let browser = baseline.browsers[&opened.browser_id].clone();
    drop(consumer);
    let intent = claim_fixture_retirement(&fixture, provider.clone(), &baseline, &browser);
    let before = fixture_retirement_records(&fixture.root);
    let mut coordinator = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let prepared = prepare_remote_view_session_pool(
        &mut coordinator,
        &mut store,
        &RemoteViewSessionPool {
            name: "main".into(),
            desired_desktops: 2,
        },
    )
    .unwrap();
    assert_eq!(
        prepared.assignments.len(),
        2,
        "reservation retains its assignment"
    );
    assert_eq!(
        prepared.desktops.len(),
        1,
        "only the unreserved desktop is eligible"
    );
    assert_ne!(
        prepared.desktops[0].desktop.desktop_id,
        intent.provider.assignment.desktop_id
    );
    assert_eq!(fixture_retirement_records(&fixture.root), before);
    assert_eq!(calls.get(), 1, "pool selection performs no browser effects");
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    assert!(
        super::super::browser_idle_retirement_store::reject_retirement_admission(
            store.remote_view_mutation_connection(),
            "profile-b",
            &intent.provider.assignment,
        )
        .is_err(),
        "final admission still fences the reserved desktop"
    );
}

#[test]
fn ordinary_live_profile_reuse_joins_current_native_desktop_without_launch() {
    let fixture = Fixture::new();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(wire()["assignment"].clone()).unwrap();
    let provider = Rc::new(RefCell::new(Provider {
        assignments: vec![assignment.clone()],
        window_pid: Some(91),
        ..Default::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::LiveProfileReuse,
    );
    let command =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(calls.get(), 0);
    assert!(provider.borrow().acquisitions.is_empty());
    let browser = consumer.state().browsers.values().next().unwrap();
    assert_eq!(browser.pid, 91);
    let desktop = browser
        .desktop
        .as_ref()
        .expect("current native window must identify its desktop");
    assert_eq!(desktop.desktop_id, assignment.desktop_id);
    assert_eq!(desktop.generation, assignment.generation);
}

#[test]
fn ordinary_read_recovers_exited_borrowed_browser_with_fresh_native_custody() {
    let fixture = Fixture::new();
    let assignment: RemoteViewAssignmentRecord =
        serde_json::from_value(wire()["assignment"].clone()).unwrap();
    let provider = Rc::new(RefCell::new(Provider {
        assignments: vec![assignment],
        window_pid: Some(91),
        ..Default::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let command =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::LiveProfileReuse,
    );
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(calls.get(), 0);
    let identity = opened["browserSession"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    assert!(consumer.state().shared_browser_ids.contains(browser_id));
    drop(consumer);
    std::fs::write(fixture.root.join("idle-process-absent"), "absent").unwrap();
    let mut consumer = host_mode(&fixture, provider, calls.clone(), Mode::IdleRecovery);
    let resumed = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(resumed["success"], true);
    assert_eq!(
        resumed["browserSession"]["sessionId"],
        identity["sessionId"]
    );
    assert_eq!(
        resumed["browserSession"]["browserId"],
        identity["browserId"]
    );
    assert_eq!(calls.get(), 1);
    assert!(!consumer.state().shared_browser_ids.contains(browser_id));
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
}

#[test]
fn ordinary_read_recovers_retained_local_browser_on_native_desktop() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let command =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    drop(consumer);
    let mut store = fixture.store(false);
    let before = store.load_session_state().unwrap();
    let mut local = before.clone();
    for browser in local.browsers.values_mut() {
        browser.desktop = None;
    }
    local
        .leave_open_session_ids
        .extend(local.sessions.keys().cloned());
    store
        .compare_and_save_session_state(&before, &local)
        .unwrap();
    store
        .remote_view_mutation_connection()
        .execute(
            "DELETE FROM state_documents WHERE kind = 'browser_launch_custody'",
            [],
        )
        .unwrap();
    drop(store);
    std::fs::write(fixture.root.join("idle-process-absent"), "absent").unwrap();
    let mut resumed = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let result = resumed
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(result["success"], true);
    assert_eq!(result["browserSession"]["sessionId"], identity["sessionId"]);
    assert_eq!(calls.get(), 2);
    assert!(resumed
        .state()
        .browsers
        .values()
        .all(|browser| browser.desktop.is_some()));
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
}

#[test]
fn ordinary_read_recovers_idle_retained_browser_without_opening_handoff() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    let original_target = opened["targetId"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    let reaped = consumer.reap(300_101).unwrap();
    assert_eq!(reaped.closed_browser_ids, vec![browser_id]);
    assert!(consumer
        .state()
        .idle_closed_browsers
        .contains_key(browser_id));
    drop(consumer);

    let mut restarted = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let resumed = restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_102
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(resumed["success"], true);
    let mut recovered_identity = resumed["browserSession"].clone();
    recovered_identity["targetId"] = identity["targetId"].clone();
    assert_eq!(recovered_identity, identity);
    assert_eq!(resumed["browserSession"]["targetId"], resumed["targetId"]);
    assert_ne!(resumed["targetId"], original_target);
    assert_eq!(calls.get(), 2);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert!(provider.borrow().view_requests.is_empty());
    assert!(restarted.state().idle_closed_browsers.is_empty());
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        *restarted.state()
    );
    restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_103
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(calls.get(), 2);
}

#[test]
fn ordinary_idle_recovery_preserves_unrelated_unfinished_launch() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    let original_target = opened["targetId"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    let reaped = consumer.reap(300_101).unwrap();
    assert_eq!(reaped.closed_browser_ids, vec![browser_id]);
    assert!(consumer
        .state()
        .idle_closed_browsers
        .contains_key(browser_id));
    drop(consumer);

    let mut store = fixture.store(false);
    let assignment = provider.borrow().assignments[0].clone();
    let unrelated = BrowserLaunchIntent {
        intent_id: uuid::Uuid::new_v4().to_string(),
        profile_id: "profile-b".into(),
        assignment,
    };
    let baseline = store.load_session_state().unwrap();
    store.admit_launch_intent(&unrelated, &baseline).unwrap();
    let retained_claim = store.unpublished_launch_records().unwrap();
    crate::native::browser_session_remote_view::reconcile_pending_recoveries(
        &mut Process {
            mode: Mode::IdleRecovery,
            root: fixture.root.clone(),
            calls: calls.clone(),
        },
        &mut store,
    )
    .unwrap();
    assert_eq!(store.unpublished_launch_records().unwrap(), retained_claim);

    let mut restarted = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let resumed = restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_102
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(resumed["success"], true);
    let mut recovered_identity = resumed["browserSession"].clone();
    recovered_identity["targetId"] = identity["targetId"].clone();
    assert_eq!(recovered_identity, identity);
    assert_eq!(resumed["browserSession"]["targetId"], resumed["targetId"]);
    assert_ne!(resumed["targetId"], original_target);
    let handoff_id = identity["handoffId"].as_str().unwrap();
    provider.borrow_mut().window_pid = Some(restarted.state().browsers[browser_id].pid);
    let handoff = restarted.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":handoff_id,
        "nativeDesktop":true, "activityAtMs":300_103,
    }));
    assert_eq!(handoff["success"], true, "{handoff}");
    assert_eq!(handoff["data"]["operatorVisible"]["state"], "ready");
    assert_eq!(handoff["data"]["targetId"], resumed["targetId"]);
    assert_eq!(calls.get(), 2);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert!(provider.borrow().view_requests.is_empty());
    assert!(restarted.state().idle_closed_browsers.is_empty());
    assert_eq!(
        fixture.store(false).unpublished_launch_records().unwrap(),
        retained_claim
    );
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        *restarted.state()
    );
    restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_103
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(calls.get(), 2);
}

#[test]
fn ordinary_idle_recovery_refuses_unproven_absence_before_launch_or_activity_refresh() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    consumer.reap(300_101).unwrap();
    let before = consumer.state().clone();
    std::fs::write(fixture.root.join("recovery-absence-unknown"), "unknown").unwrap();
    assert_eq!(
        consumer
            .execute_managed_command(
                "Alice",
                &serde_json::json!({
                    "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":300_102
                })
            )
            .unwrap_err(),
        "browser_session_recovery_absence_unproven"
    );
    assert_eq!(consumer.state(), &before);
    assert_eq!(fixture.store(false).load_session_state().unwrap(), before);
    assert_eq!(calls.get(), 1);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
}

#[test]
fn ordinary_first_open_admits_exact_profile_and_reuses_tab_with_navigation_history() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let request = serde_json::json!({"action": "navigate", "runtimeProfile": "profile-a", "sessionName":"Alice",
        "params":{"url": "https://synthetic.example/first", "headers": {"X-Test":"fixture"}, "waitUntil":"domcontentloaded", "activityAtMs": 100}});
    let command =
        crate::native::stream::service_request_adapter_fixture(&request.to_string()).unwrap();
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(opened["id"], command["id"]);
    assert_eq!(opened["data"]["headers"], command["headers"]);
    assert_eq!(opened["data"]["waitUntil"], "domcontentloaded");
    assert_eq!(opened["browserSession"]["profileId"], "profile-a");
    let handoff_id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let persisted = fixture.store(false).load_session_state().unwrap();
    assert_eq!(persisted.remote_view_tab_handoffs.len(), 1);
    assert_eq!(
        opened["browserSession"]["handoffUrl"],
        format!("/remote-view/{handoff_id}")
    );
    let retained =
        agent_browser_service_model::resolve_remote_view_tab_handoff(&persisted, handoff_id)
            .unwrap();
    assert_eq!(retained.tab.id, opened["browserSession"]["tabId"]);
    assert!(opened.get("handoffUrl").is_none());
    assert_eq!(consumer.state().navigation_history.len(), 1);
    assert_eq!(
        consumer.state().navigation_history[0].url,
        "https://synthetic.example/first"
    );
    let mut second = crate::mcp::service_request_adapter_fixture_for_session(
        &serde_json::json!({
            "action":"navigate", "sessionName":"Alice",
            "profile":fixture.root.join("profile-a").to_string_lossy().into_owned(),
            "params":{"url":"https://synthetic.example/second", "activityAtMs":101},
        }),
        "service-default",
    )
    .unwrap();
    let reopened = consumer
        .execute_managed_command("Alice", &second)
        .unwrap()
        .unwrap();
    assert_eq!(reopened["browserSession"], opened["browserSession"]);
    assert_eq!(calls.get(), 1);
    assert_eq!(provider.borrow().acquisitions.len(), 1);
    assert_eq!(consumer.state().navigation_history.len(), 2);
    second["url"] = "https://synthetic.example/failure".into();
    let failed = consumer
        .execute_managed_command("Alice", &second)
        .unwrap()
        .unwrap();
    assert_eq!(failed["success"], false);
    assert_eq!(consumer.state().navigation_history.len(), 2);
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let result = restarted
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(result["browserSession"], opened["browserSession"]);
    assert_eq!(calls.get(), 1);
}

#[test]
fn named_session_navigation_registers_explicit_profile_before_open() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider, calls.clone());
    let path = fixture.root.join("cli-fresh-profile");
    std::fs::create_dir_all(&path).unwrap();
    let response = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_navigate", "sessionName":"Alice", "profileId":"cli-fresh",
        "runtimeProfile":"cli-fresh", "profile":path, "url":"https://synthetic.example/", "activityAtMs":100
    }));
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["data"]["profileId"], "cli-fresh");
    assert_eq!(calls.get(), 1);
}

#[test]
fn ordinary_open_registers_explicit_profile_and_rejects_identity_rebinding() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let profile_path = fixture.root.join("fresh-profile");
    std::fs::create_dir_all(&profile_path).unwrap();
    let command = serde_json::json!({"action":"navigate", "url":"https://synthetic.example/",
        "runtimeProfile":"fresh", "profile":profile_path, "activityAtMs":100});
    let mut conflict = command.clone();
    conflict["profileId"] = serde_json::json!("different");
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &conflict)
            .unwrap_err(),
        "browser_session_profile_selector_conflict"
    );
    assert!(!fixture
        .store(false)
        .load_profile_catalog()
        .unwrap()
        .catalog
        .profiles
        .contains_key("fresh"));
    assert_eq!(calls.get(), 0);
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(calls.get(), 1);
    let persisted = fixture.store(false).load_profile_catalog().unwrap();
    assert_eq!(
        persisted.catalog.profiles["fresh"].user_data_dir,
        profile_path.canonicalize().unwrap().to_string_lossy()
    );
    let other_path = fixture.root.join("other-profile");
    std::fs::create_dir_all(&other_path).unwrap();
    let mut rebound = command.clone();
    rebound["profile"] = serde_json::json!(other_path);
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &rebound)
            .unwrap_err(),
        "browser_session_profile_selector_conflict"
    );
    let mut alias = command;
    alias["runtimeProfile"] = serde_json::json!("alias");
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &alias)
            .unwrap_err(),
        "browser_session_profile_selector_conflict"
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn ordinary_profile_selector_conflict_precedes_allocation_and_default_is_disposable() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let command = serde_json::json!({"action":"navigate", "url":"https://synthetic.example/", "activityAtMs":100,
        "runtimeProfile":"profile-a", "profileId":"profile-b"});
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &command)
            .unwrap_err(),
        "browser_session_profile_selector_conflict"
    );
    assert!(provider.borrow().operations.is_empty());
    assert_eq!(calls.get(), 0);
    for (invalid, expected) in [
        (
            serde_json::json!({"action":"navigate", "url":"https://synthetic.example/", "profileId":4}),
            "browser_session_profile_selector_invalid",
        ),
        (
            serde_json::json!({"action":"navigate", "url":"https://synthetic.example/", "profileId":"unknown"}),
            "browser_session_profile_selector_unknown",
        ),
        (
            serde_json::json!({"action":"navigate", "profileId":"profile-a"}),
            "browser_session_field_missing:url",
        ),
    ] {
        assert_eq!(
            consumer
                .execute_managed_command("Alice", &invalid)
                .unwrap_err(),
            expected
        );
        assert!(provider.borrow().operations.is_empty());
    }
    let command = serde_json::json!({"id":"default-open", "action":"navigate", "url":"https://synthetic.example/", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &command)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(consumer.state().disposable_profiles.len(), 1);
    assert_eq!(calls.get(), 1);
}

#[test]
fn native_handoff_assignment_history_preserves_exact_target_and_rejects_active_ambiguity() {
    for history_count in [1, 3] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/history", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
        let id = opened["browserSession"]["handoffId"].as_str().unwrap();
        let browser_id = opened["browserSession"]["browserId"].as_str().unwrap();
        provider.borrow_mut().window_pid = Some(consumer.state().browsers[browser_id].pid);
        let active = provider.borrow().assignments[0].clone();
        for index in 0..history_count {
            let mut released = active.clone();
            released.assignment_id = format!("released-history-{index}");
            released.state = RemoteViewAssignmentState::Released;
            provider.borrow_mut().assignments.push(released);
        }
        drop(consumer);
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let issued = consumer
            .resolve_tab_handoff_view(id, 2000)
            .expect("released history must not obscure the active assignment");
        let command = serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "nativeDesktop":true, "activityAtMs":2001,
        });
        let response = consumer.handle_command(&command);
        assert_eq!(response["success"], true, "{response}");
        assert_eq!(response["data"]["operatorVisible"]["state"], "ready");
        assert_eq!(
            response["data"]["targetId"],
            opened["browserSession"]["targetId"]
        );
        assert_eq!(
            provider.borrow().view_requests.len(),
            1,
            "native resolution adds no grant after compatibility issuance"
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(consumer.state().navigation_history.len(), 1);

        let baseline = fixture.store(false).load_session_state().unwrap();
        // A second active assignment is genuine ambiguity, even if custody names one.
        let mut competing = active.clone();
        competing.assignment_id = "competing-active".into();
        provider.borrow_mut().assignments.push(competing);
        assert_eq!(
            consumer.resolve_tab_handoff_view(id, 2001).unwrap_err(),
            "remote_view_tab_view_assignment_unavailable"
        );
        assert_eq!(fixture.store(false).load_session_state().unwrap(), baseline);
        provider.borrow_mut().assignments.pop();
        provider.borrow_mut().assignments[0].state = RemoteViewAssignmentState::Released;
        assert_eq!(
            consumer.resolve_tab_handoff_view(id, 2001).unwrap_err(),
            "remote_view_tab_view_assignment_unavailable"
        );
        assert_eq!(fixture.store(false).load_session_state().unwrap(), baseline);
        provider.borrow_mut().assignments[0] = active;
        assert_eq!(consumer.resolve_tab_handoff_view(id, 2001).unwrap(), issued);
        let recovered = consumer.handle_command(&command);
        assert_eq!(recovered["success"], true, "{recovered}");
        assert_eq!(
            recovered["data"]["targetId"],
            opened["browserSession"]["targetId"]
        );
        assert_eq!(provider.borrow().view_requests.len(), 1);
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn native_handoff_resolution_survives_restart_without_viewer_grants_or_navigation_replay() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    provider.borrow_mut().all_views_terminal = true;
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_string();
    let browser_id = opened["browserSession"]["browserId"].as_str().unwrap();
    provider.borrow_mut().window_pid = Some(consumer.state().browsers[browser_id].pid);
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let command = serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "nativeDesktop":true, "activityAtMs":2001,
    });
    for _ in 0..2 {
        let response = restarted.handle_command(&command);
        assert_eq!(response["success"], true, "{response}");
        assert_eq!(response["data"]["operatorVisible"]["state"], "ready");
        assert_eq!(
            response["data"]["targetId"],
            opened["browserSession"]["targetId"]
        );
    }
    assert!(provider.borrow().view_requests.is_empty());
    assert_eq!(calls.get(), 1);
    assert_eq!(restarted.state().navigation_history.len(), 1);
    let session_id = opened["browserSession"]["sessionId"].as_str().unwrap();
    restarted
        .close_session(session_id, SessionEndReason::ExplicitClose, 2002)
        .unwrap();
    let response = restarted.handle_command(&command);
    assert_eq!(response["success"], false);
    assert_eq!(calls.get(), 1);
}

#[test]
fn handoff_view_issuance_and_resolution_survive_host_restart_without_launch_or_navigation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    provider.borrow_mut().delayed_scoped_view = true;
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let issuance = consumer.resolve_tab_handoff_view(id, 2000).unwrap();
    assert_eq!(issuance.presentation_state, "grant_issued");
    let stored = fixture.store(false).load_session_state().unwrap();
    assert_eq!(
        stored.remote_view_tab_handoffs[id]
            .view
            .as_ref()
            .unwrap()
            .issuance
            .as_ref(),
        Some(&issuance)
    );
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let response = restarted.handle_command(&serde_json::json!({
        "id":"resolve", "action":"service_remote_view_handoff_resolve", "handoffId":id, "activityAtMs":2001,
    }));
    assert_eq!(response["success"], true);
    assert_eq!(response["data"]["status"], "converging");
    assert_eq!(response["data"]["operatorVisible"]["state"], "pending");
    assert_eq!(
        response["data"]["targetId"],
        opened["browserSession"]["targetId"]
    );
    assert!(response["data"].get("providerExternalUrl").is_none());
    assert_eq!(provider.borrow().view_requests.len(), 1);
    assert_eq!(calls.get(), 1);
    assert_eq!(restarted.state().navigation_history.len(), 1);
    let before_view = fixture.store(false).load_session_state().unwrap();
    let target =
        agent_browser_service_model::resolve_remote_view_tab_handoff(&before_view, id).unwrap();
    let view = before_view.remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap();
    let read_only = agent_browser_service_model::resolve_published_remote_view_tab_view(
        &mut adapter(&fixture, provider.clone()),
        &mut fixture.store(false),
        &target,
        view,
        2001,
    )
    .unwrap();
    assert_eq!(read_only, issuance);
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        before_view
    );
    assert_eq!(provider.borrow().view_requests.len(), 1);
    let reused = restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "activityAtMs":2001,
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(reused["browserSession"]["handoffId"], id);
    let original_assignment = provider.borrow().assignments[0].assignment_id.clone();
    provider.borrow_mut().assignments[0].assignment_id = "replacement-assignment".into();
    assert!(restarted.resolve_tab_handoff_view(id, 2001).is_err());
    assert_eq!(provider.borrow().view_requests.len(), 1);
    provider.borrow_mut().assignments[0].assignment_id = original_assignment;
    let session_id = opened["browserSession"]["sessionId"].as_str().unwrap();
    restarted.close_current_tab(session_id, 2002).unwrap();
    let operations = provider.borrow().operations.len();
    assert!(restarted.resolve_tab_handoff_view(id, 2003).is_err());
    assert_eq!(provider.borrow().operations.len(), operations);
}

#[test]
fn handoff_lost_view_reply_keeps_one_request_across_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    provider.borrow_mut().lose_view_reply = true;
    assert!(consumer.resolve_tab_handoff_view(id, 2000).is_err());
    let before = fixture
        .store(false)
        .load_session_state()
        .unwrap()
        .remote_view_tab_handoffs[id]
        .view
        .clone();
    assert!(before.as_ref().unwrap().issuance.is_none());
    drop(consumer);
    provider.borrow_mut().lose_view_reply = false;
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    restarted.resolve_tab_handoff_view(id, 2001).unwrap();
    assert_eq!(
        restarted.state().remote_view_tab_handoffs[id]
            .view
            .as_ref()
            .unwrap()
            .idempotency_key,
        before.as_ref().unwrap().idempotency_key
    );
    assert_eq!(provider.borrow().view_requests.len(), 2);
    assert_eq!(
        provider.borrow().view_requests[0],
        provider.borrow().view_requests[1]
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn published_view_expiry_renews_after_restart_without_relaunch_or_navigation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let issuance = consumer.resolve_tab_handoff_view(id, 2000).unwrap();
    let before = consumer.state().remote_view_tab_handoffs[id].clone();
    drop(consumer);
    provider.borrow_mut().view_issued_at = Some(issuance.grant.expires_at);
    let mut restarted = host_mode_clock(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::Success,
        || 301000,
    );
    let replacement = restarted
        .resolve_tab_handoff_view(id, issuance.grant.expires_at)
        .unwrap();
    let after = &restarted.state().remote_view_tab_handoffs[id];
    assert_eq!(after.id, before.id);
    assert_eq!(after.session_id, before.session_id);
    assert_eq!(after.tab_id, before.tab_id);
    assert_ne!(
        after.view.as_ref().unwrap().idempotency_key,
        before.view.as_ref().unwrap().idempotency_key
    );
    assert!(replacement.grant.expires_at > issuance.grant.expires_at);
    assert_eq!(provider.borrow().view_requests.len(), 2);
    assert_eq!(calls.get(), 1);
    assert_eq!(restarted.state().navigation_history.len(), 1);
}

#[test]
fn published_view_expiry_rejects_invalid_or_revoked_issuance() {
    for invalid in ["revoked", "application", "target"] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
            "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
        })).unwrap().unwrap();
        let id = opened["browserSession"]["handoffId"].as_str().unwrap();
        let issuance = consumer.resolve_tab_handoff_view(id, 2000).unwrap();
        let before = consumer.state().clone();
        drop(consumer);
        let mut changed = before.clone();
        let grant = &mut changed
            .remote_view_tab_handoffs
            .get_mut(id)
            .unwrap()
            .view
            .as_mut()
            .unwrap()
            .issuance
            .as_mut()
            .unwrap()
            .grant;
        match invalid {
            "revoked" => grant.revoked = true,
            "application" => grant.request.application = "foreign".into(),
            "target" => grant.request.target.viewing_generation += 1,
            _ => unreachable!(),
        }
        fixture
            .store(false)
            .compare_and_save_session_state(&before, &changed)
            .unwrap();
        let mut restarted = host(&fixture, provider.clone(), calls.clone());
        assert_eq!(
            restarted
                .resolve_tab_handoff_view(id, issuance.grant.expires_at)
                .unwrap_err(),
            "remote_view_tab_view_issuance_invalid"
        );
        assert_eq!(restarted.state(), &changed);
        assert_eq!(provider.borrow().view_requests.len(), 1);
    }
}

#[test]
fn published_view_expiry_preserves_handoff_when_renewal_cannot_complete() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let issuance = consumer.resolve_tab_handoff_view(id, 2000).unwrap();
    let retained = consumer.state().remote_view_tab_handoffs[id].clone();
    provider.borrow_mut().all_views_terminal = true;
    assert_eq!(
        consumer
            .resolve_tab_handoff_view(id, issuance.grant.expires_at)
            .unwrap_err(),
        "remote_view_tab_view_grant_terminal"
    );
    assert_eq!(
        consumer.state().remote_view_tab_handoffs[id].id,
        retained.id
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn terminal_pending_view_renews_once_preserving_handoff_and_mutation_history() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    provider.borrow_mut().lose_view_reply = true;
    assert!(consumer.resolve_tab_handoff_view(id, 2000).is_err());
    let old_key = consumer.state().remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap()
        .idempotency_key
        .clone();
    drop(consumer);
    provider.borrow_mut().lose_view_reply = false;
    provider.borrow_mut().terminal_view_key = Some(old_key.clone());
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    restarted.resolve_tab_handoff_view(id, 2001).unwrap();
    let new_key = &restarted.state().remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap()
        .idempotency_key;
    assert_ne!(*new_key, old_key);
    assert_eq!(
        provider.borrow().view_requests,
        vec![old_key.clone(), old_key.clone(), new_key.clone()]
    );
    let mut connection = fixture.store(false);
    let terminal_records: i64 = connection
        .remote_view_mutation_connection()
        .query_row(
            "SELECT count(*) FROM state_documents WHERE json LIKE '%issue_view_terminal%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(terminal_records, 1);
    assert_eq!(calls.get(), 1);
}

#[test]
fn terminal_view_renewal_stops_after_one_replacement() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    provider.borrow_mut().all_views_terminal = true;
    assert_eq!(
        consumer.resolve_tab_handoff_view(id, 2000).unwrap_err(),
        "remote_view_tab_view_grant_terminal"
    );
    assert_eq!(provider.borrow().view_requests.len(), 2);
    assert!(consumer.state().remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap()
        .issuance
        .is_none());
    assert_eq!(calls.get(), 1);
}

#[test]
fn handoff_consumers_reconcile_peer_publication_before_entry() {
    for operation in ["view", "inspect", "native"] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
            "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
        })).unwrap().unwrap();
        let id = opened["browserSession"]["handoffId"].as_str().unwrap();
        let browser_id = opened["browserSession"]["browserId"].as_str().unwrap();
        provider.borrow_mut().window_pid = Some(consumer.state().browsers[browser_id].pid);
        let sequence = consumer.state().next_session_sequence;
        competing_write(&fixture.root);
        if operation == "view" {
            let issued = consumer
                .resolve_tab_handoff_view(id, 2000)
                .expect("an independent publication must not veto view issuance");
            assert_eq!(issued.presentation_state, "grant_issued");
        } else {
            let response = consumer.handle_command(&serde_json::json!({
                "action":"service_remote_view_handoff_resolve", "handoffId":id,
                "handoffOperation":if operation == "inspect" { "inspect" } else { "open" },
                "nativeDesktop":operation == "native", "activityAtMs":2000,
            }));
            assert_eq!(response["success"], true, "{operation}: {response}");
        }
        assert_eq!(consumer.state().next_session_sequence, sequence + 1);
        assert_eq!(
            fixture
                .store(false)
                .load_session_state()
                .unwrap()
                .next_session_sequence,
            sequence + 1
        );
        assert_eq!(calls.get(), 1, "no browser launch replay");
        assert_eq!(
            provider.borrow().view_requests.len(),
            usize::from(operation == "view")
        );
    }
}

#[test]
fn completed_view_reconciles_peer_publication_and_reuses_ledger_after_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    provider.borrow_mut().competing_view_publication = true;
    let issued = consumer.resolve_tab_handoff_view(id, 2000).unwrap();
    assert_eq!(issued.presentation_state, "grant_issued");
    assert!(fixture
        .store(false)
        .load_session_state()
        .unwrap()
        .remote_view_tab_handoffs[id]
        .view
        .as_ref()
        .unwrap()
        .issuance
        .is_some());
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let recovered = restarted.resolve_tab_handoff_view(id, 2001).unwrap();
    assert_eq!(recovered.presentation_state, "grant_issued");
    assert_eq!(provider.borrow().view_requests.len(), 1);
    assert_eq!(calls.get(), 1);
}

#[tokio::test]
async fn queued_action_executor_resolves_managed_handoff_through_the_registered_host() {
    use crate::native::browser_session_remote_view::RuntimeSessionEffects;
    use crate::native::browser_session_runtime::{
        BrowserManagerRuntime, BrowserManagerRuntimeConfig, BrowserSessionEffectAdapter,
    };
    use crate::native::remote_view_application_http::RemoteViewApplicationHttp;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let fixture = Fixture::new();
    let cdp_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cdp_endpoint = format!(
        "ws://{}/devtools/browser/fixture",
        cdp_listener.local_addr().unwrap()
    );
    std::fs::write(
        fixture.root.join("live-cdp-endpoint.json"),
        serde_json::json!({"pid":std::process::id(), "endpoint":cdp_endpoint}).to_string(),
    )
    .unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut source = host(&fixture, provider.clone(), calls.clone());
    let opened = source.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100,
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_string();
    drop(source);
    let before = fixture.store(false).load_session_state().unwrap();
    let target = before.tabs.values().next().unwrap().target_id.clone();
    let cdp_server = tokio::spawn(async move {
        use futures_util::{SinkExt, StreamExt};
        let (stream, _) = cdp_listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        while let Some(Ok(message)) = socket.next().await {
            let tokio_tungstenite::tungstenite::Message::Text(text) = message else {
                continue;
            };
            let request: Value = serde_json::from_str(&text).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "SystemInfo.getProcessInfo" => serde_json::json!({"processInfo":[{
                    "type":"browser", "id":std::process::id()
                }]}),
                "Target.getTargets" => serde_json::json!({"targetInfos":[{
                    "targetId":target, "type":"page", "title":"Fixture",
                    "url":"https://synthetic.example/", "attached":false
                }]}),
                "Browser.getWindowForTarget" => serde_json::json!({"windowId":42}),
                "Target.attachToTarget" => serde_json::json!({"sessionId":"fixture-session"}),
                "Runtime.evaluate" => {
                    serde_json::json!({"result":{"type":"string", "value":"https://synthetic.example/"}})
                }
                _ => serde_json::json!({}),
            };
            socket
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    serde_json::json!({"id":request["id"], "result":result})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let fixture = wire();
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let (header_end, length) = loop {
                let mut chunk = [0_u8; 4096];
                let count = stream.read(&mut chunk).await.unwrap();
                assert!(count > 0 && bytes.len() + count < 65536);
                bytes.extend_from_slice(&chunk[..count]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let header = std::str::from_utf8(&bytes[..end]).unwrap();
                    let length = header
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break (end + 4, length);
                    }
                }
            };
            let envelope: Value =
                serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
            let response = match envelope["request"]["operation"].as_str().unwrap() {
                "inventory" => fixture["inventory"].clone(),
                "observe_assignment" => fixture["assignmentObservation"].clone(),
                "windows" => {
                    let mut windows = fixture["windows"].clone();
                    windows["windows"][0]["pid"] = serde_json::json!(std::process::id());
                    windows
                }
                "activate" => fixture["activation"].clone(),
                _ => panic!("queued resolver must not launch or acquire"),
            }
            .to_string();
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).as_bytes()).await.unwrap();
        }
    });
    let runtime = BrowserManagerRuntime::start(BrowserManagerRuntimeConfig {
        headless: true,
        executable_path: None,
        display: None,
        remote_headed: false,
        maximum_browser_processes: None,
        remote_view_desktop_contexts: Vec::new(),
    })
    .unwrap();
    let transport =
        RemoteViewApplicationHttp::new(&origin, std::time::Duration::from_secs(2)).unwrap();
    let effects = RemoteViewSessionEffects::new(
        BrowserSessionEffectAdapter::new(runtime),
        RemoteViewApplicationAdapter::new("agent-browser".into(), transport).unwrap(),
        fixture.store(false),
        Vec::new(),
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap()
    .with_pool(RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    })
    .unwrap();
    let registered = BrowserSessionHost::load(
        fixture.store(false),
        RuntimeSessionEffects::Remote(Box::new(effects)),
        &fixture.root.join("unused.json"),
        BrowserSessionHostConfig {
            session_idle_timeout_ms: 300_000,
            remote_view_desktops: Vec::new(),
            default_disposable_policy: None,
            exact_url_history_maximum_bytes: 1024,
        },
    )
    .unwrap();
    let shared = std::sync::Arc::new(std::sync::Mutex::new(Some(registered)));
    let mut state =
        crate::native::action_runtime::DaemonState::new().with_managed_session_host(shared.clone());
    let response = crate::native::actions::execute_command(
        &serde_json::json!({
            "id":"queued-resolve", "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "activityAtMs":2001,
        }),
        &mut state,
    )
    .await;
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["data"]["handoffId"], id);
    assert_eq!(response["data"]["operatorVisible"]["state"], "ready");
    let after = fixture.store(false).load_session_state().unwrap();
    assert!(after.remote_view_tab_handoffs[&id].view.is_none());
    assert_eq!(after.browsers, before.browsers);
    let mut expected_sessions = before.sessions.clone();
    for session in expected_sessions.values_mut() {
        session.last_activity_at_ms = 2001;
        session.expires_at_ms = 302001;
    }
    assert_eq!(after.sessions, expected_sessions);
    let mut expected_tabs = before.tabs.clone();
    for tab in expected_tabs.values_mut() {
        tab.last_activity_at_ms = 2001;
    }
    assert_eq!(after.tabs, expected_tabs);
    assert_eq!(after.navigation_history, before.navigation_history);
    assert_eq!(shared.lock().unwrap().as_ref().unwrap().state(), &after);
    let repeated = crate::native::actions::execute_command(
        &serde_json::json!({
            "id":"queued-repeat", "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "activityAtMs":2001,
        }),
        &mut state,
    )
    .await;
    assert_eq!(repeated["success"], true, "{repeated}");
    assert_eq!(fixture.store(false).load_session_state().unwrap(), after);
    server.abort();
    cdp_server.abort();
    let cold = serde_json::json!({
        "id":"queued-cold", "action":"snapshot", "runtimeProfile":"missing-profile",
        "sessionName":"ColdService", "serviceName":"synthetic-service", "agentName":"fixture", "taskName":"selector-check",
    });
    state.confirm_actions = Some(crate::native::policy::ConfirmActions {
        categories: std::collections::HashSet::from(["snapshot".to_string()]),
    });
    let confirmation = crate::native::actions::execute_command(&cold, &mut state).await;
    assert_eq!(confirmation["data"]["confirmation_required"], true);
    state.confirm_actions = None;
    let policy_path = fixture.root.join("deny-snapshot.json");
    std::fs::write(&policy_path, r#"{"default":"allow","deny":["snapshot"]}"#).unwrap();
    state.policy =
        Some(crate::native::policy::ActionPolicy::load(policy_path.to_str().unwrap()).unwrap());
    let denied = crate::native::actions::execute_command(&cold, &mut state).await;
    assert_eq!(denied["success"], false);
    assert!(denied["error"]
        .as_str()
        .unwrap()
        .contains("denied by policy"));
    state.policy = None;
    let queue = crate::native::control_plane::ControlPlaneWorker::start(state);
    let rejected = queue.submit(cold).await;
    assert_eq!(
        rejected["error"], "browser_session_profile_selector_unknown",
        "{rejected}"
    );
    let request = serde_json::json!({
        "action":"snapshot", "runtimeProfile":"missing-profile", "sessionName":"ColdService",
        "serviceName":"synthetic-service", "agentName":"fixture", "taskName":"selector-check",
    });
    let http =
        crate::native::stream::service_request_adapter_fixture(&request.to_string()).unwrap();
    let mcp = crate::mcp::service_request_adapter_fixture_for_session(&request, "service-default")
        .unwrap();
    for command in [http, mcp] {
        assert_eq!(command["runtimeProfile"], "missing-profile");
        assert_eq!(command["sessionName"], "ColdService");
        assert_eq!(command["taskName"], "selector-check");
        let id = command["id"].clone();
        let rejected = queue.submit(command).await;
        assert_eq!(rejected["id"], id);
        assert_eq!(
            rejected["error"], "browser_session_profile_selector_unknown",
            "{rejected}"
        );
    }
    let mut tab_request = request.clone();
    tab_request["action"] = "tab_new".into();
    tab_request["params"] = serde_json::json!({"tabRequestId":"stable-client-retry"});
    let saved_origin = std::env::var_os("AGENT_BROWSER_REMOTE_VIEW_ORIGIN");
    std::env::set_var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN", "http://127.0.0.1:1");
    let service_state = ServiceState::default();
    let normalized = crate::native::service_request::normalize_service_request(
        crate::native::service_request::ServiceRequestNormalization {
            request: &tab_request,
            service_state: Some(&service_state),
            authenticated_principal: None,
            fallback_principal: None,
            request_id: "normalized-tab-request",
            effective_session: Some("service-default"),
        },
    );
    match saved_origin {
        Some(value) => std::env::set_var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN", value),
        None => std::env::remove_var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN"),
    }
    let mut normalized = normalized.unwrap().command;
    normalized["id"] = "normalized-tab-request".into();
    assert_eq!(normalized["runtimeProfile"], "missing-profile");
    assert_eq!(normalized["sessionName"], "ColdService");
    assert_eq!(normalized["tabRequestId"], "stable-client-retry");
    let rejected = queue.submit(normalized).await;
    assert_eq!(
        rejected["error"], "browser_session_profile_selector_unknown",
        "{rejected}"
    );
    assert_eq!(fixture.store(false).load_session_state().unwrap(), after);
    queue.shutdown().await;
}

#[test]
fn ordinary_tab_creation_replays_exact_result_and_retains_unknown_outcome_across_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let first = serde_json::json!({"id":"tab-first", "action":"tab_new", "runtimeProfile":"profile-a",
        "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &first)
        .unwrap()
        .unwrap();
    assert_eq!(opened["success"], true);
    assert_eq!(consumer.state().tabs.len(), 1);
    assert!(consumer.state().navigation_history.is_empty());
    let mut next = first.clone();
    next["id"] = "tab-next".into();
    next["tabRequestId"] = "stable-next".into();
    next["url"] = "https://synthetic.example/second".into();
    next["activityAtMs"] = 101.into();
    let second = consumer
        .execute_managed_command("Alice", &next)
        .unwrap()
        .unwrap();
    assert_eq!(second["success"], true);
    assert_ne!(
        opened["browserSession"]["tabId"],
        second["browserSession"]["tabId"]
    );
    assert_ne!(
        opened["browserSession"]["handoffId"],
        second["browserSession"]["handoffId"]
    );
    assert_eq!(consumer.state().tabs.len(), 2);
    assert_eq!(calls.get(), 1);
    let state = consumer.state().clone();
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap()
            .unwrap(),
        second
    );
    assert_eq!(restarted.state(), &state);
    next["id"] = "transport-retry".into();
    let mut replay = second.clone();
    replay["id"] = "transport-retry".into();
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap()
            .unwrap(),
        replay
    );
    assert_eq!(restarted.state(), &state);
    let session_id = second["browserSession"]["sessionId"].as_str().unwrap();
    restarted.close_current_tab(session_id, 102).unwrap();
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap_err(),
        "browser_session_tab_request_target_closed"
    );
    assert_eq!(restarted.state().tabs.len(), 1);
    next["url"] = "https://synthetic.example/changed".into();
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap_err(),
        "browser_session_tab_request_conflict"
    );
    drop(restarted);
    let mut unknown = host_mode(&fixture, provider.clone(), calls.clone(), Mode::UnknownTab);
    next["id"] = "tab-unknown".into();
    next.as_object_mut().unwrap().remove("tabRequestId");
    assert_eq!(
        unknown.execute_managed_command("Alice", &next).unwrap_err(),
        "synthetic_tab_reply_lost"
    );
    let pending = fixture.store(false).load_session_state().unwrap();
    assert!(pending.managed_tab_requests["tab-unknown"]
        .response
        .is_none());
    assert_eq!(pending.tabs.len(), 1);
    drop(unknown);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap_err(),
        "browser_session_tab_creation_readback_required"
    );
    next["id"] = "another-attempt".into();
    assert_eq!(
        restarted
            .execute_managed_command("Alice", &next)
            .unwrap_err(),
        "browser_session_tab_creation_readback_required"
    );
    assert_eq!(
        restarted.execute_managed_command("Bob", &next).unwrap_err(),
        "browser_session_tab_creation_readback_required"
    );
    assert_eq!(fixture.store(false).load_session_state().unwrap(), pending);
    assert_eq!(calls.get(), 1);
}

#[test]
fn remote_view_focus_requires_the_owned_active_window() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut consumer = host(&fixture, provider.clone(), Rc::new(Cell::new(0)));
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let pid = consumer.state().browsers[&opened.browser_id].pid;
    provider.borrow_mut().window_pid = Some(pid.saturating_add(1));
    assert_eq!(
        consumer
            .focus_browser(&opened.browser_id, None, 101)
            .unwrap_err(),
        "remote_view_focus_owned_window_unproven"
    );
    provider.borrow_mut().window_pid = Some(pid);
    provider.borrow_mut().inactive_window = true;
    assert_eq!(
        consumer
            .focus_browser(&opened.browser_id, None, 102)
            .unwrap_err(),
        "remote_view_focus_owned_window_unproven"
    );
    provider.borrow_mut().inactive_window = false;
    consumer
        .focus_browser(&opened.browser_id, None, 103)
        .unwrap();
}

#[test]
fn handoff_link_retention_extension_and_expiry_do_not_change_browser_or_issue_viewer_grants() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a",
                "activityAtMs":100,"handoffTtlMs":10,
            }),
        )
        .unwrap()
        .unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(opened["browserSession"]["handoffExpiresAtMs"], 110);
    let browsers = consumer.state().browsers.clone();
    let sessions = consumer.state().sessions.clone();
    let tabs = consumer.state().tabs.clone();
    let history = consumer.state().navigation_history.clone();
    let extended = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,
        "handoffOperation":"extend","ttlMs":20,"activityAtMs":105,
    }));
    assert_eq!(extended["success"], true, "{extended}");
    assert_eq!(extended["data"]["expiresAtMs"], 125);
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    assert_eq!(
        consumer.state().remote_view_tab_handoffs[&id].expires_at_ms,
        Some(125)
    );
    for operation in [None, Some("extend")] {
        let mut request = serde_json::json!({"action":"service_remote_view_handoff_resolve",
            "handoffId":id,"nativeDesktop":true,"activityAtMs":125,"ttlMs":20});
        if let Some(operation) = operation {
            request["handoffOperation"] = serde_json::json!(operation);
        }
        let expired = consumer.handle_command(&request);
        assert_eq!(expired["success"], false);
        assert_eq!(expired["error"], "remote_view_handoff_expired");
    }
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,
        "handoffOperation":"inspect","activityAtMs":125}));
    assert_eq!(inspected["data"]["expired"], true);
    assert_eq!(consumer.state().browsers, browsers);
    assert_eq!(consumer.state().sessions, sessions);
    assert_eq!(consumer.state().tabs, tabs);
    assert_eq!(consumer.state().navigation_history, history);
    assert!(provider.borrow().view_requests.is_empty());
    assert_eq!(calls.get(), 1);
    let reopened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate","url":"https://synthetic.example/", "profileId":"profile-a",
                "activityAtMs":126,"handoffTtlMs":10,
            }),
        )
        .unwrap()
        .unwrap();
    assert_ne!(reopened["browserSession"]["handoffId"], id);
    assert_eq!(
        reopened["browserSession"]["targetId"],
        opened["browserSession"]["targetId"]
    );
    assert_eq!(reopened["browserSession"]["handoffExpiresAtMs"], 136);
    assert!(consumer.state().remote_view_tab_handoffs[&id]
        .check_link_at(126)
        .is_err());
}

#[test]
fn durable_named_handoff_has_no_default_expiry_across_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(opened["browserSession"]["handoffExpiresAtMs"].is_null());
    drop(consumer);
    let consumer = host(&fixture, provider, calls);
    let link = &consumer.state().remote_view_tab_handoffs[&id];
    assert_eq!(link.expires_at_ms, None);
    link.check_link_at(86_400_101).unwrap();
}

#[test]
fn disposable_handoff_defaults_perpetual_across_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    assert!(
        opened["browserSession"]["handoffExpiresAtMs"].is_null(),
        "{opened}"
    );
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    drop(consumer);
    let mut consumer = host(&fixture, provider, calls.clone());
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":2_678_400_100_u64
    }));
    assert_eq!(inspected["success"], true, "{inspected}");
    assert_eq!(inspected["data"]["expired"], false);
    assert!(inspected["data"]["expiresAtMs"].is_null());
    assert_eq!(calls.get(), 1);
}

#[test]
fn legacy_handoff_without_expiry_stays_perpetual_after_upgrade() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    drop(consumer);
    // Supply the historical document shape to the persisted-state boundary.
    // The old record has no creation marker and no configured link expiry.
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut legacy = serde_json::to_value(&original).unwrap();
    let link = legacy["remoteViewTabHandoffs"][&id]
        .as_object_mut()
        .unwrap();
    link.remove("createdAtMs");
    link.remove("expiresAtMs");
    store
        .compare_and_save_session_state(&original, &serde_json::from_value(legacy).unwrap())
        .unwrap();
    for _ in 0..2 {
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let inspected = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"inspect", "activityAtMs":u64::MAX
        }));
        assert_eq!(inspected["success"], true, "{inspected}");
        assert_eq!(inspected["data"]["expired"], false, "{inspected}");
        assert!(inspected["data"]["expiresAtMs"].is_null(), "{inspected}");
    }
    assert_eq!(calls.get(), 1);
}

#[test]
fn configured_durable_retention_survives_restart_and_applies_to_new_links() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = serde_json::to_value(&original).unwrap();
    configured["retentionPolicy"] = serde_json::json!({"durableHandoffTtlMs":10, "disposableHandoffTtlMs":20, "disposableInactivityMs":30});
    let configured: BrowserSessionState = serde_json::from_value(configured).unwrap();
    store
        .compare_and_save_session_state(&original, &configured)
        .unwrap();
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(opened["browserSession"]["handoffExpiresAtMs"], 110);
    drop(consumer);
    let consumer = host(&fixture, provider, calls);
    assert_eq!(
        consumer.state().remote_view_tab_handoffs[&id].expires_at_ms,
        Some(110)
    );
    assert_eq!(
        consumer.state().remote_view_tab_handoffs[&id]
            .check_link_at(110)
            .unwrap_err(),
        "remote_view_handoff_expired"
    );
}

#[test]
fn valid_handoff_recreates_expired_task_tab_after_host_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = original.clone();
    configured.retention_policy.disposable_handoff_ttl_ms = Some(1_000);
    configured.retention_policy.disposable_inactivity_ms = 30;
    store
        .compare_and_save_session_state(&original, &configured)
        .unwrap();
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/expired-task",
                "activityAtMs":100, "purpose":"Original synthetic task", "agentName":"fixture-origin"
            }),
        )
        .unwrap()
        .unwrap();
    let handoff_id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    let session_id = opened["browserSession"]["sessionId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        consumer.reap(130).unwrap().expired_session_ids,
        vec![session_id]
    );
    let expired = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":handoff_id,
        "handoffOperation":"inspect", "activityAtMs":130
    }));
    assert_eq!(expired["success"], true, "{expired}");
    assert_eq!(
        expired["data"]["taskMetadata"]["purpose"], "Original synthetic task",
        "{expired}"
    );
    // The external provider observes the second synthetic process launch.
    // A valid handoff still requires an owned, active native window.
    provider.borrow_mut().window_pid = Some(44);
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    // Both callers cache the expired aggregate before either starts recovery.
    let mut stale = host(&fixture, provider.clone(), calls.clone());
    let reopen = serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":handoff_id,
        "nativeDesktop":true, "activityAtMs":131
    });
    let first_reopen = consumer.handle_command(&reopen);
    assert_eq!(first_reopen["success"], true, "{first_reopen}");
    // Native selection and compatibility view issuance are separate contracts.
    // Lose the latter reply only after the recreated task is durable.
    provider.borrow_mut().view_issued_at = Some(131);
    provider.borrow_mut().lose_view_reply = true;
    assert!(consumer.resolve_tab_handoff_view(&handoff_id, 131).is_err());
    let recreated = fixture.store(false).load_session_state().unwrap();
    assert_eq!(recreated.tabs.len(), 1);
    let recreated_link = recreated.remote_view_tab_handoffs[&handoff_id].clone();
    assert!(!recreated_link.target_expired);
    assert!(recreated_link.recreation.is_none());
    let launches = calls.get();
    let requests = provider.borrow().view_requests.clone();
    assert_eq!(requests.len(), 1);
    let stale_result = stale.handle_command(&reopen);
    // Native presentation refreshes the durable recreated target independently
    // of the lost compatibility-view reply. It may focus that same owned window
    // without relaunching or replaying compatibility-view issuance.
    assert_eq!(stale_result["success"], true, "{stale_result}");
    assert_eq!(stale_result["data"]["operatorVisible"]["state"], "ready");
    assert_eq!(stale_result["data"]["handoffId"], handoff_id);
    assert_eq!(
        stale_result["data"]["browserId"],
        recreated.tabs[&recreated_link.tab_id].browser_id
    );
    assert_eq!(stale_result["data"]["tabId"], recreated_link.tab_id);
    assert_eq!(
        stale_result["data"]["targetId"],
        recreated.tabs[&recreated_link.tab_id].target_id
    );
    assert_eq!(calls.get(), launches, "stale reopen must not launch");
    assert_eq!(provider.borrow().view_requests, requests);
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        recreated
    );
    drop(stale);
    drop(consumer);
    provider.borrow_mut().lose_view_reply = false;
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    consumer.resolve_tab_handoff_view(&handoff_id, 131).unwrap();
    let reopened = consumer.handle_command(&reopen);
    assert_eq!(
        calls.get(),
        launches,
        "restart must reuse the recreated target"
    );
    assert_eq!(provider.borrow().view_requests.len(), 2);
    assert_eq!(
        provider.borrow().view_requests[0],
        provider.borrow().view_requests[1]
    );
    let settled = &consumer.state().remote_view_tab_handoffs[&handoff_id];
    assert_eq!(settled.session_id, recreated_link.session_id);
    assert_eq!(settled.tab_id, recreated_link.tab_id);
    assert_eq!(consumer.state().tabs, recreated.tabs);
    assert_eq!(reopened["success"], true, "{reopened}");
    assert_eq!(reopened["data"]["handoffId"], handoff_id);
    assert_eq!(reopened["data"]["operatorVisible"]["state"], "ready");
    let restored = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":handoff_id,
        "handoffOperation":"inspect", "activityAtMs":132
    }));
    assert_eq!(restored["success"], true, "{restored}");
    assert_eq!(
        restored["data"]["taskMetadata"], expired["data"]["taskMetadata"],
        "original metadata survives recreation"
    );
    assert_eq!(restored["data"]["taskMetadata"]["createdAtMs"], 100);
}

#[test]
fn expired_or_deleted_link_cannot_recreate_missing_target_after_restart() {
    for deleted in [false, true] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut owner = host(&fixture, provider.clone(), calls.clone());
        let alice = owner.execute_managed_command("Alice", &serde_json::json!({
            "action":"navigate", "url":"https://synthetic.example/revoked-recovery", "profileId":"profile-a", "activityAtMs":100
        })).unwrap().unwrap();
        let bob = owner.execute_managed_command("Bob", &serde_json::json!({
            "action":"navigate", "url":"https://synthetic.example/preserved-peer", "profileId":"profile-a", "activityAtMs":101
        })).unwrap().unwrap();
        let id = alice["browserSession"]["handoffId"].as_str().unwrap();
        let configured = owner.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"set_tab_lifetime", "tabInactivityMs":30, "activityAtMs":101
        }));
        assert_eq!(configured["success"], true, "{configured}");
        if !deleted {
            let finite = owner.handle_command(&serde_json::json!({
                "action":"service_remote_view_handoff_resolve", "handoffId":id,
                "handoffOperation":"set_link_lifetime", "ttlMs":30, "activityAtMs":100
            }));
            assert_eq!(finite["data"]["expiresAtMs"], 130, "{finite}");
        }
        let expired = owner.handle_command(&serde_json::json!({
            "action":"browser_session_reap", "activityAtMs":130
        }));
        assert_eq!(expired["success"], true, "{expired}");
        assert_eq!(
            expired["data"]["expiredTabIds"],
            serde_json::json!([alice["browserSession"]["tabId"]])
        );
        if deleted {
            let revoked = owner.handle_command(&serde_json::json!({
                "action":"service_remote_view_handoff_resolve", "handoffId":id,
                "handoffOperation":"delete", "activityAtMs":130
            }));
            assert_eq!(revoked["data"]["status"], "deleted", "{revoked}");
        }
        drop(owner);
        let mut restarted = host(&fixture, provider.clone(), calls.clone());
        let before = fixture.store(false).load_session_state().unwrap();
        let effects = provider.borrow().operations.clone();
        let views = provider.borrow().view_requests.clone();
        let launches = calls.get();
        let denied = restarted.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "nativeDesktop":true, "activityAtMs":131
        }));
        assert_eq!(denied["success"], false, "{denied}");
        assert_eq!(
            denied["error"],
            if deleted {
                "remote_view_tab_handoff_missing"
            } else {
                "remote_view_handoff_expired"
            },
            "{denied}"
        );
        assert_eq!(
            calls.get(),
            launches,
            "invalid retained link must not launch"
        );
        assert_eq!(provider.borrow().operations, effects);
        assert_eq!(provider.borrow().view_requests, views);
        assert_eq!(fixture.store(false).load_session_state().unwrap(), before);
        let peer = restarted.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":bob["browserSession"]["handoffId"],
            "handoffOperation":"inspect", "activityAtMs":131
        }));
        assert_eq!(peer["success"], true, "{peer}");
        assert_eq!(peer["data"]["tabId"], bob["browserSession"]["tabId"]);
    }
}

#[test]
fn valid_handoff_can_extend_after_target_expiry_without_recreation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = original.clone();
    configured.retention_policy.disposable_handoff_ttl_ms = Some(1_000);
    configured.retention_policy.disposable_inactivity_ms = 30;
    store
        .compare_and_save_session_state(&original, &configured)
        .unwrap();
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/expired-task", "activityAtMs":100
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(consumer.reap(130).unwrap().expired_session_ids.len(), 1);
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let extended = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"extend", "ttlMs":2_000, "activityAtMs":131
    }));
    assert_eq!(extended["success"], true, "{extended}");
    assert_eq!(extended["data"]["handoffId"], id);
    assert_eq!(extended["data"]["expiresAtMs"], 2_131);
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":132
    }));
    assert_eq!(inspected["data"]["expiresAtMs"], 2_131);
    let state_before_edit = consumer.state().clone();
    let shortened = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_link_lifetime", "ttlMs":10, "activityAtMs":132
    }));
    assert_eq!(shortened["success"], true, "{shortened}");
    assert_eq!(shortened["data"]["expiresAtMs"], 142, "{shortened}");
    let perpetual = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_link_lifetime", "ttlMs":0, "activityAtMs":133
    }));
    assert_eq!(perpetual["success"], true, "{perpetual}");
    assert!(perpetual["data"]["expiresAtMs"].is_null());
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":5000
    }));
    assert_eq!(inspected["data"]["expired"], false, "{inspected}");
    assert!(inspected["data"]["expiresAtMs"].is_null());
    assert_eq!(consumer.state().tabs, state_before_edit.tabs);
    assert_eq!(consumer.state().browsers, state_before_edit.browsers);
    assert_eq!(consumer.state().sessions, state_before_edit.sessions);
    assert_eq!(calls.get(), 1, "link management must not launch a browser");
    assert!(provider.borrow().view_requests.is_empty());
}

#[test]
fn invalid_pending_human_deadline_rejects_before_browser_effects() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let error = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100,
        "humanHandoff":true, "pendingHumanRetentionMs":u64::MAX
    })).unwrap_err();
    assert_eq!(error, "remote_view_pending_human_retention_invalid");
    assert_eq!(calls.get(), 0, "invalid deadline must not launch a browser");
    assert!(
        provider.borrow().operations.is_empty(),
        "invalid deadline must not acquire a desktop"
    );
    let status = consumer.handle_command(&serde_json::json!({"action":"browser_session_status"}));
    assert_eq!(status["data"]["browsers"], serde_json::json!({}));
}

#[test]
fn explicit_human_handoff_prepares_configured_hold_and_agent_work_does_not() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider, calls);
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100,
        "humanHandoff":true, "pendingHumanRetentionMs":50
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":110
    }));
    assert_eq!(
        inspected["data"]["pendingHuman"]["awaitingUse"], true,
        "{inspected}"
    );
    assert_eq!(inspected["data"]["pendingHuman"]["expiresAtMs"], 150);
    let agent = consumer.execute_managed_command("Bob", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/bob", "profileId":"profile-a", "activityAtMs":110
    })).unwrap().unwrap();
    let agent_link = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":agent["browserSession"]["handoffId"],
        "handoffOperation":"inspect", "activityAtMs":110
    }));
    assert!(agent_link["data"]["pendingHuman"].is_null(), "{agent_link}");
}

#[test]
fn public_human_use_request_preserves_pending_hold() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        window_pid: Some(43),
        ..Provider::default()
    }));
    let mut consumer = host(&fixture, provider, Rc::new(Cell::new(0)));
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a",
        "activityAtMs":100, "humanHandoff":true, "pendingHumanRetentionMs":50
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let request = serde_json::json!({
        "action":"service_remote_view_handoff_resolve",
        "params":{
            "handoffId":id, "handoffOperation":"confirm_human_use", "activityAtMs":110,
            "humanUseEvidence":{
                "source":"authenticated_presentation_navigation", "operator":"fabricated-operator",
                "tabId":opened["browserSession"]["tabId"], "targetId":opened["browserSession"]["targetId"]
            }
        }
    });
    if let Ok(command) = crate::native::stream::service_request_adapter_fixture(
        &serde_json::to_string(&request).unwrap(),
    ) {
        consumer.handle_command(&command);
    }
    let retained = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":111
    }));
    assert_eq!(
        retained["data"]["pendingHuman"]["awaitingUse"], true,
        "public request consumed pending human hold: {retained}"
    );
    assert!(
        retained["data"]["taskMetadata"]["firstHumanUseAtMs"].is_null(),
        "public request fabricated first human use: {retained}"
    );
}

#[test]
fn human_use_releases_pending_hold_but_agent_preparation_does_not() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        window_pid: Some(43),
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let configured = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":30, "activityAtMs":100
    }));
    assert_eq!(configured["success"], true, "{configured}");
    let prepared = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"prepare_human", "pendingHumanRetentionMs":0, "activityAtMs":110
    }));
    assert_eq!(prepared["success"], true, "{prepared}");
    let agent_preparation = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "nativeDesktop":true, "activityAtMs":140
    }));
    assert_eq!(
        agent_preparation["data"]["operatorVisible"]["state"], "ready",
        "{agent_preparation}"
    );
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":140
    }));
    assert_eq!(inspected["data"]["pendingHuman"]["awaitingUse"], true);
    assert!(
        inspected["data"]["taskMetadata"]["firstHumanUseAtMs"].is_null(),
        "agent readiness must not record first use"
    );
    for (source, operator, target) in [
        (
            "authenticated_presentation_navigation",
            "synthetic-operator",
            "stale-target",
        ),
        (
            "agent_preparation",
            "synthetic-operator",
            opened["browserSession"]["targetId"].as_str().unwrap(),
        ),
        (
            "authenticated_presentation_navigation",
            "",
            opened["browserSession"]["targetId"].as_str().unwrap(),
        ),
    ] {
        let rejected = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"confirm_human_use", "activityAtMs":145,
            "humanUseEvidence":{"source":source, "operator":operator,
                "tabId":opened["browserSession"]["tabId"], "targetId":target}
        }));
        assert_eq!(rejected["success"], false, "{rejected}");
        let retained = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"inspect", "activityAtMs":145
        }));
        assert_eq!(
            retained["data"]["pendingHuman"]["awaitingUse"], true,
            "{retained}"
        );
        assert!(
            retained["data"]["taskMetadata"]["firstHumanUseAtMs"].is_null(),
            "rejected acknowledgement must not record use"
        );
    }
    let registered = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"prepare_native_use",
        "expectedTabId":opened["browserSession"]["tabId"],"expectedTargetId":opened["browserSession"]["targetId"],"activityAtMs":145
    }));
    assert_eq!(registered["success"], true, "{registered}");
    let pending = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"observe_native_use","activityAtMs":145
    }));
    assert_eq!(pending["data"]["status"], "native_use_pending", "{pending}");
    assert!(consumer
        .state()
        .pending_human_tabs
        .contains_key(opened["browserSession"]["tabId"].as_str().unwrap()));
    provider.borrow_mut().native_used_at_ms = Some(150);
    provider.borrow_mut().window_pid = Some(99);
    let denied = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"observe_native_use", "activityAtMs":146
    }));
    assert_eq!(
        denied["success"], false,
        "foreign native window must deny acknowledgement: {denied}"
    );
    let retained = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":147
    }));
    assert_eq!(retained["data"]["pendingHuman"]["awaitingUse"], true);
    assert!(retained["data"]["taskMetadata"]["firstHumanUseAtMs"].is_null());
    provider.borrow_mut().window_pid = Some(43);
    let used = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"observe_native_use", "activityAtMs":150
    }));
    assert_eq!(used["success"], true, "{used}");
    assert_eq!(used["data"]["status"], "human_use_confirmed", "{used}");
    let first_use = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":151
    }));
    assert_eq!(first_use["success"], true, "{first_use}");
    assert_eq!(
        first_use["data"]["taskMetadata"]["firstHumanUseAtMs"], 150,
        "first human use must remain attributable in Properties: {first_use}"
    );
    let duplicate = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"observe_native_use", "activityAtMs":170
    }));
    assert_eq!(duplicate["success"], true, "{duplicate}");
    assert_eq!(
        duplicate["data"]["status"], "human_use_already_confirmed",
        "{duplicate}"
    );
    let before = consumer
        .handle_command(&serde_json::json!({"action":"browser_session_reap", "activityAtMs":179}));
    assert_eq!(before["data"]["expiredTabIds"], serde_json::json!([]));
    let expired = consumer
        .handle_command(&serde_json::json!({"action":"browser_session_reap", "activityAtMs":180}));
    assert_eq!(
        expired["data"]["expiredTabIds"],
        serde_json::json!([opened["browserSession"]["tabId"]])
    );
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":180
    }));
    assert_eq!(inspected["data"]["expired"], false);
    drop(consumer);
    let mut consumer = host(&fixture, provider, calls);
    let restarted = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":181
    }));
    assert_eq!(restarted["success"], true, "{restarted}");
    assert_eq!(
        restarted["data"]["taskMetadata"]["firstHumanUseAtMs"], 150,
        "{restarted}"
    );
    assert_eq!(
        restarted["data"]["taskMetadata"]["attribution"], "unknown",
        "human use does not invent originating identity"
    );
    assert_eq!(
        inspected["data"]["taskMetadata"]["firstHumanUseAtMs"], 150,
        "duplicate use and target expiry must preserve first use: {inspected}"
    );
}

#[test]
fn first_human_use_without_pending_hold_survives_reprepare_and_legacy_restart() {
    for (legacy, reprepare) in [(false, false), (false, true), (true, false), (true, true)] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider {
            window_pid: Some(43),
            ..Provider::default()
        }));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
            "action":"navigate", "url":"https://synthetic.example/first-use", "profileId":"profile-a", "activityAtMs":100
        })).unwrap().unwrap();
        let id = opened["browserSession"]["handoffId"].as_str().unwrap();
        let configured = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id, "handoffOperation":"set_tab_lifetime",
            "tabInactivityMs":100, "activityAtMs":100
        }));
        assert_eq!(configured["success"], true, "{configured}");
        if legacy {
            drop(consumer);
            let mut store = fixture.store(false);
            let before = store.load_session_state().unwrap();
            let mut old = before.clone();
            old.tab_metadata.clear();
            old.remote_view_tab_handoffs.get_mut(id).unwrap().resume = None;
            store.compare_and_save_session_state(&before, &old).unwrap();
            consumer = host(&fixture, provider.clone(), calls.clone());
        }
        let inspect = |consumer: &mut BrowserSessionHost<_, _>, at| {
            consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id, "handoffOperation":"inspect", "activityAtMs":at
        }))
        };
        let before = inspect(&mut consumer, 110);
        assert_eq!(before["success"], true, "{before}");
        assert!(before["data"]["pendingHuman"].is_null());
        assert!(before["data"]["taskMetadata"]["firstHumanUseAtMs"].is_null());
        let acknowledge = |consumer: &mut BrowserSessionHost<_, _>, at| {
            consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id, "handoffOperation":"observe_native_use", "activityAtMs":at
        }))
        };
        let registered = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"prepare_native_use",
            "expectedTabId":opened["browserSession"]["tabId"],"expectedTargetId":opened["browserSession"]["targetId"],"activityAtMs":149
        }));
        assert_eq!(registered["success"], true, "{registered}");
        provider.borrow_mut().native_used_at_ms = Some(150);
        let first = acknowledge(&mut consumer, 150);
        assert_eq!(first["success"], true, "{first}");
        assert_eq!(
            first["data"]["status"], "human_use_confirmed",
            "unheld handoff still has a first use: {first}"
        );
        let properties = inspect(&mut consumer, 151);
        assert_eq!(
            properties["data"]["taskMetadata"]["firstHumanUseAtMs"], 150,
            "{properties}"
        );
        assert_eq!(
            properties["data"]["taskMetadata"]["attribution"], "unknown",
            "{properties}"
        );
        if legacy {
            assert!(
                properties["data"]["taskMetadata"]["createdAtMs"].is_null(),
                "first use must not invent missing creation history: {properties}"
            );
        }
        if reprepare {
            let hold = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id, "handoffOperation":"prepare_human",
            "pendingHumanRetentionMs":0, "activityAtMs":160
        }));
            assert_eq!(hold["success"], true, "{hold}");
            let stale = acknowledge(&mut consumer, 161);
            assert_eq!(
                stale["success"], false,
                "old native observation consumed a newly prepared hold: {stale}"
            );
            assert_eq!(
                inspect(&mut consumer, 161)["data"]["pendingHuman"]["awaitingUse"],
                true
            );
            let fresh = consumer.handle_command(&serde_json::json!({
                "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"prepare_native_use",
                "expectedTabId":opened["browserSession"]["tabId"],"expectedTargetId":opened["browserSession"]["targetId"],"activityAtMs":162
            }));
            assert_eq!(fresh["success"], true, "{fresh}");
            provider.borrow_mut().native_used_at_ms = Some(170);
            let repeated = acknowledge(&mut consumer, 170);
            assert_eq!(
                repeated["data"]["status"], "human_use_confirmed",
                "{repeated}"
            );
            assert!(inspect(&mut consumer, 171)["data"]["pendingHuman"].is_null());
        }
        let expiry = if reprepare { 270 } else { 250 };
        consumer.reap(expiry).unwrap();
        assert!(!consumer
            .state()
            .tabs
            .contains_key(opened["browserSession"]["tabId"].as_str().unwrap()));
        drop(consumer);
        let mut consumer = host(&fixture, provider, calls.clone());
        let retained = inspect(&mut consumer, expiry + 1);
        assert_eq!(retained["success"], true, "{retained}");
        assert_eq!(
            retained["data"]["taskMetadata"]["firstHumanUseAtMs"], 150,
            "legacy={legacy}, reprepare={reprepare}: {retained}"
        );
        assert_eq!(
            retained["data"]["taskMetadata"]["attribution"], "unknown",
            "{retained}"
        );
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn unused_human_handoff_protects_configured_tab_expiry_across_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let configured = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":30, "activityAtMs":100
    }));
    assert_eq!(configured["success"], true, "{configured}");
    let prepared = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"prepare_human", "pendingHumanRetentionMs":0, "activityAtMs":110
    }));
    assert_eq!(prepared["success"], true, "{prepared}");
    assert_eq!(prepared["data"]["status"], "pending_human", "{prepared}");
    drop(consumer);
    let mut consumer = host(&fixture, provider, calls.clone());
    let reaped = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_reap", "activityAtMs":130
    }));
    assert_eq!(reaped["success"], true, "{reaped}");
    assert_eq!(reaped["data"]["expiredTabIds"], serde_json::json!([]));
    assert_eq!(reaped["data"]["closedBrowserIds"], serde_json::json!([]));
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":130
    }));
    assert_eq!(inspected["data"]["pendingHuman"]["awaitingUse"], true);
    assert_eq!(inspected["data"]["expired"], false);
    assert_eq!(calls.get(), 1);
}

#[test]
fn finite_pending_human_deadline_expires_exact_target_after_restart() {
    for retention_ms in [10 * 60 * 1_000_u64, 4 * 60 * 60 * 1_000] {
        let deadline = 110 + retention_ms;
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
        let id = opened["browserSession"]["handoffId"].as_str().unwrap();
        let configured = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"set_tab_lifetime", "tabInactivityMs":30, "activityAtMs":100
        }));
        assert_eq!(configured["success"], true, "{configured}");
        let prepared = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"prepare_human", "pendingHumanRetentionMs":retention_ms, "activityAtMs":110
    }));
        assert_eq!(prepared["success"], true, "{prepared}");
        assert_eq!(prepared["data"]["status"], "pending_human", "{prepared}");
        drop(consumer);
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let reaped = consumer.handle_command(&serde_json::json!({
            "action":"browser_session_reap", "activityAtMs":deadline - 1
        }));
        assert_eq!(reaped["success"], true, "{reaped}");
        assert_eq!(reaped["data"]["expiredTabIds"], serde_json::json!([]));
        assert_eq!(reaped["data"]["closedBrowserIds"], serde_json::json!([]));
        let inspected = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"inspect", "activityAtMs":deadline - 1
        }));
        assert_eq!(inspected["data"]["pendingHuman"]["awaitingUse"], true);
        assert_eq!(inspected["data"]["expired"], false);
        assert_eq!(inspected["data"]["pendingHuman"]["expiresAtMs"], deadline);
        drop(consumer);
        let mut consumer = host(&fixture, provider, calls.clone());
        let expired = consumer.handle_command(&serde_json::json!({
            "action":"browser_session_reap", "activityAtMs":deadline
        }));
        assert_eq!(expired["success"], true, "{expired}");
        assert_eq!(
            expired["data"]["expiredTabIds"],
            serde_json::json!([opened["browserSession"]["tabId"]]),
            "{expired}"
        );
        let link = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"inspect", "activityAtMs":deadline
        }));
        assert_eq!(link["data"]["expired"], false, "{link}");
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn existing_link_lifetime_switches_finite_and_perpetual_without_target_effects() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100,
        "humanHandoff":true, "pendingHumanRetentionMs":0
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let before = consumer.state().clone();
    let operations = provider.borrow().operations.clone();
    let finite = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_link_lifetime", "ttlMs":20, "activityAtMs":100
    }));
    assert_eq!(finite["success"], true, "{finite}");
    assert_eq!(
        finite["data"]["status"], "link_lifetime_updated",
        "{finite}"
    );
    assert_eq!(finite["data"]["expiresAtMs"], 120);
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let invalid = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_link_lifetime", "ttlMs":u64::MAX, "activityAtMs":110
    }));
    assert_eq!(invalid["success"], false, "{invalid}");
    let unchanged = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":110
    }));
    assert_eq!(unchanged["data"]["expiresAtMs"], 120, "{unchanged}");
    let perpetual = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_link_lifetime", "ttlMs":0, "activityAtMs":110
    }));
    assert_eq!(perpetual["success"], true, "{perpetual}");
    assert_eq!(
        perpetual["data"]["status"], "link_lifetime_updated",
        "{perpetual}"
    );
    assert!(perpetual["data"]["expiresAtMs"].is_null());
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":200
    }));
    assert_eq!(inspected["success"], true, "{inspected}");
    assert_eq!(inspected["data"]["expired"], false, "{inspected}");
    assert!(inspected["data"]["expiresAtMs"].is_null());
    let finite_again = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_link_lifetime", "ttlMs":10, "activityAtMs":200
    }));
    assert_eq!(finite_again["success"], true, "{finite_again}");
    assert_eq!(finite_again["data"]["expiresAtMs"], 210);
    let elapsed = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_link_lifetime", "ttlMs":0, "activityAtMs":210
    }));
    assert_eq!(elapsed["success"], false, "{elapsed}");
    let expired = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":210
    }));
    assert_eq!(expired["data"]["expired"], true, "{expired}");
    assert_eq!(expired["data"]["expiresAtMs"], 210);
    assert_eq!(consumer.state().tabs, before.tabs);
    assert_eq!(consumer.state().browsers, before.browsers);
    assert_eq!(consumer.state().sessions, before.sessions);
    assert_eq!(
        consumer.state().pending_human_tabs,
        before.pending_human_tabs
    );
    assert_eq!(consumer.state().tab_lifetimes, before.tab_lifetimes);
    assert_eq!(provider.borrow().operations, operations);
    assert_eq!(calls.get(), 1);
}

#[test]
fn expired_link_deletion_fences_stale_policy_and_recovery_publication() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut winner = host(&fixture, provider.clone(), calls.clone());
    let alice = winner.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100,
        "serviceName":"Original service", "agentName":"Original agent", "purpose":"Preserve original task context"
    })).unwrap().unwrap();
    let bob = winner.execute_managed_command("Bob", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/bob", "profileId":"profile-a", "activityAtMs":101
    })).unwrap().unwrap();
    let id = alice["browserSession"]["handoffId"].as_str().unwrap();
    let finite = winner.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_link_lifetime", "ttlMs":10, "activityAtMs":100
    }));
    assert_eq!(finite["data"]["expiresAtMs"], 110, "{finite}");
    let mut stale = host(&fixture, provider.clone(), calls.clone());
    let before = winner.state().clone();
    let effects_before = provider.borrow().operations.clone();
    let launches_before = calls.get();
    let deleted = winner.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"delete", "activityAtMs":110
    }));
    assert_eq!(deleted["data"]["status"], "deleted", "{deleted}");
    for params in [
        serde_json::json!({"handoffOperation":"set_link_lifetime","ttlMs":0}),
        serde_json::json!({"handoffOperation":"set_tab_lifetime","tabInactivityMs":0}),
        serde_json::json!({"handoffOperation":"prepare_human","pendingHumanRetentionMs":0}),
        serde_json::json!({}),
    ] {
        let mut command = serde_json::json!({"action":"service_remote_view_handoff_resolve","handoffId":id,"activityAtMs":111});
        command
            .as_object_mut()
            .unwrap()
            .extend(params.as_object().unwrap().clone());
        let rejected = stale.handle_command(&command);
        assert_eq!(rejected["success"], false, "{rejected}");
        assert_eq!(
            rejected["error"], "remote_view_tab_handoff_missing",
            "{rejected}"
        );
    }
    drop(stale);
    drop(winner);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let missing = restarted.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"inspect","activityAtMs":112
    }));
    assert_eq!(
        missing["error"], "remote_view_tab_handoff_missing",
        "{missing}"
    );
    assert!(restarted
        .state()
        .remote_view_tab_handoffs
        .contains_key(bob["browserSession"]["handoffId"].as_str().unwrap()));
    assert_eq!(restarted.state().tabs, before.tabs);
    assert_eq!(restarted.state().browsers, before.browsers);
    assert_eq!(restarted.state().sessions, before.sessions);
    assert_eq!(restarted.state().tab_metadata, before.tab_metadata);
    assert_eq!(restarted.state().tab_lifetimes, before.tab_lifetimes);
    assert_eq!(
        restarted.state().pending_human_tabs,
        before.pending_human_tabs
    );
    assert_eq!(provider.borrow().operations, effects_before);
    assert_eq!(calls.get(), launches_before);
}

#[test]
fn competing_link_delete_and_policy_write_publish_one_restart_safe_outcome() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut initial = host(&fixture, provider, calls);
    let opened = initial.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/concurrent","profileId":"profile-a","activityAtMs":100
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    let before = initial.state().clone();
    drop(initial);
    let results = std::thread::scope(|scope| {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let mut starts = Vec::new();
        let mut workers = Vec::new();
        for operation in ["delete", "set_link_lifetime"] {
            let fixture = &fixture;
            let id = id.clone();
            let ready = ready_tx.clone();
            let (start_tx, start_rx) = std::sync::mpsc::channel();
            starts.push(start_tx);
            workers.push(scope.spawn(move || {
                let provider = Rc::new(RefCell::new(Provider::default()));
                let calls = Rc::new(Cell::new(0));
                let mut owner = host(fixture, provider.clone(), calls.clone());
                ready.send(()).unwrap();
                start_rx
                    .recv_timeout(std::time::Duration::from_secs(10))
                    .expect("bounded owner start");
                let response = owner.handle_command(&serde_json::json!({
                    "action":"service_remote_view_handoff_resolve","handoffId":id,
                    "handoffOperation":operation,"ttlMs":1000,"activityAtMs":150
                }));
                assert!(provider.borrow().operations.is_empty());
                assert_eq!(calls.get(), 0);
                (operation, response)
            }));
        }
        // A failed setup cannot strand its peer on an unbounded barrier.
        for _ in 0..2 {
            ready_rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("bounded owner readiness");
        }
        for start in starts {
            start.send(()).unwrap();
        }
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    // Overlapping operations may linearize as lifetime update then deletion.
    // Both acknowledgements are valid; the final deletion must remain terminal.
    assert!(
        results
            .iter()
            .any(|(_, response)| response["success"] == true),
        "{results:?}"
    );
    for (operation, response) in results
        .iter()
        .filter(|(_, response)| response["success"] != true)
    {
        assert!(
            response["error"] == "browser_session_publication_conflict"
                || (*operation == "set_link_lifetime"
                    && response["error"] == "remote_view_tab_handoff_missing"
                    && results
                        .iter()
                        .any(|(op, reply)| *op == "delete" && reply["success"] == true)),
            "{operation}: {response}"
        );
    }
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    let readback = restarted.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"inspect","activityAtMs":151
    }));
    let delete_won = results
        .iter()
        .any(|(operation, response)| *operation == "delete" && response["success"] == true);
    if delete_won {
        assert_eq!(
            readback["error"], "remote_view_tab_handoff_missing",
            "{readback}"
        );
    } else {
        assert_eq!(readback["success"], true, "{readback}");
        assert_eq!(readback["data"]["expiresAtMs"], 1150, "{readback}");
    }
    assert_eq!(restarted.state().tabs, before.tabs);
    assert_eq!(restarted.state().browsers, before.browsers);
    assert_eq!(restarted.state().sessions, before.sessions);
    assert_eq!(restarted.state().tab_metadata, before.tab_metadata);
    assert!(provider.borrow().operations.is_empty());
    assert_eq!(calls.get(), 0);
}

#[test]
fn independent_handoff_policy_deadlines_and_creation_context_survive_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut owner = host(&fixture, provider.clone(), calls.clone());
    let opened = owner.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/policies","profileId":"profile-a","activityAtMs":100,
        "serviceName":"Original service","agentName":"Original agent","humanName":"Supplied operator",
        "originKind":"agent","purpose":"Original purpose","requestId":"creation-request"
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let before = owner.state().clone();
    let effects = provider.borrow().operations.clone();
    let launches = calls.get();
    for params in [
        serde_json::json!({"handoffOperation":"set_link_lifetime","ttlMs":1000,"activityAtMs":110}),
        serde_json::json!({"handoffOperation":"set_tab_lifetime","tabInactivityMs":10000,"activityAtMs":120}),
        serde_json::json!({"handoffOperation":"prepare_human","pendingHumanRetentionMs":50,"activityAtMs":130}),
    ] {
        let mut command = serde_json::json!({"action":"service_remote_view_handoff_resolve","handoffId":id,
            "serviceName":"Later editor","agentName":"Later editor","purpose":"Do not replace origin"});
        command
            .as_object_mut()
            .unwrap()
            .extend(params.as_object().unwrap().clone());
        let response = owner.handle_command(&command);
        assert_eq!(response["success"], true, "{response}");
    }
    drop(owner);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    for (now, retained) in [(179, true), (180, false)] {
        let readback = restarted.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"inspect","activityAtMs":now
        }));
        assert_eq!(readback["success"], true, "{readback}");
        assert_eq!(readback["data"]["createdAtMs"], 100);
        assert_eq!(readback["data"]["expiresAtMs"], 1110);
        assert_eq!(readback["data"]["tabLifetime"]["inactivityMs"], 10000);
        assert_eq!(readback["data"]["pendingHuman"]["requestedAtMs"], 130);
        assert_eq!(readback["data"]["pendingHuman"]["expiresAtMs"], 180);
        assert_eq!(readback["data"]["pendingHuman"]["retained"], retained);
        assert_eq!(
            readback["data"]["taskMetadata"]["agentName"],
            "Original agent"
        );
        assert_eq!(
            readback["data"]["taskMetadata"]["purpose"],
            "Original purpose"
        );
        assert_eq!(
            readback["data"]["taskMetadata"]["requestId"],
            "creation-request"
        );
    }
    let expired = restarted.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"activityAtMs":1110
    }));
    assert_eq!(expired["error"], "remote_view_handoff_expired", "{expired}");
    assert!(restarted.state().remote_view_tab_handoffs.contains_key(id));
    assert_eq!(restarted.state().tabs, before.tabs);
    assert_eq!(restarted.state().browsers, before.browsers);
    assert_eq!(restarted.state().sessions, before.sessions);
    assert_eq!(restarted.state().tab_metadata, before.tab_metadata);
    assert_eq!(provider.borrow().operations, effects);
    assert_eq!(calls.get(), launches);
}

#[test]
fn deleting_handoff_revokes_only_link_and_preserves_target_across_restart() {
    for link_ttl in [None, Some(5)] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100, "humanHandoff":true, "pendingHumanRetentionMs":0
    })).unwrap().unwrap();
        let id = opened["browserSession"]["handoffId"].as_str().unwrap();
        if let Some(ttl) = link_ttl {
            let limited = consumer.handle_command(&serde_json::json!({
                "action":"service_remote_view_handoff_resolve", "handoffId":id,
                "handoffOperation":"set_link_lifetime", "ttlMs":ttl, "activityAtMs":100
            }));
            assert_eq!(limited["success"], true, "{limited}");
        }
        let before = consumer.state().clone();
        let deleted = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"delete", "activityAtMs":110
        }));
        assert_eq!(deleted["success"], true, "{deleted}");
        assert_eq!(deleted["data"]["status"], "deleted", "{deleted}");
        assert_eq!(consumer.state().tabs, before.tabs);
        assert_eq!(consumer.state().browsers, before.browsers);
        assert_eq!(consumer.state().sessions, before.sessions);
        assert_eq!(
            consumer.state().pending_human_tabs,
            before.pending_human_tabs
        );
        drop(consumer);
        let mut consumer = host(&fixture, provider, calls.clone());
        let revoked = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"inspect", "activityAtMs":111
        }));
        assert_eq!(revoked["success"], false, "{revoked}");
        assert_eq!(consumer.state().tabs, before.tabs);
        assert_eq!(consumer.state().browsers, before.browsers);
        assert_eq!(consumer.state().sessions, before.sessions);
        assert_eq!(
            consumer.state().pending_human_tabs,
            before.pending_human_tabs
        );
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn handoff_properties_preserve_supplied_origin_and_purpose_across_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let alice = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/review", "activityAtMs":100,
        "serviceName":"Synthetic Review", "agentName":"fixture-agent", "taskName":"Review synthetic document",
        "purpose":"Human review of the synthetic document", "requestId":"fixture-request"
    })).unwrap().unwrap();
    let id = alice["browserSession"]["handoffId"].as_str().unwrap();
    drop(consumer);
    let mut consumer = host(&fixture, provider, calls.clone());
    let properties = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":110
    }));
    assert_eq!(properties["success"], true, "{properties}");
    let metadata = &properties["data"]["taskMetadata"];
    assert_eq!(metadata["attribution"], "supplied", "{properties}");
    assert_eq!(metadata["agentName"], "fixture-agent", "{properties}");
    assert_eq!(metadata["serviceName"], "Synthetic Review", "{properties}");
    assert_eq!(
        metadata["purpose"], "Human review of the synthetic document",
        "{properties}"
    );
    assert_eq!(metadata["requestId"], "fixture-request", "{properties}");
    assert_eq!(metadata["createdAtMs"], 100, "{properties}");
    assert_eq!(calls.get(), 1);
    consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/next", "activityAtMs":120,
                "agentName":"later-client", "purpose":"Replacement context"
            }),
        )
        .unwrap()
        .unwrap();
    let later = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":130
    }));
    assert_eq!(later["success"], true, "{later}");
    assert_eq!(
        later["data"]["taskMetadata"], *metadata,
        "original context must not be overwritten"
    );
    let bob = consumer
        .execute_managed_command(
            "Bob",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/unknown", "activityAtMs":140
            }),
        )
        .unwrap()
        .unwrap();
    let unknown = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":bob["browserSession"]["handoffId"],
        "handoffOperation":"inspect", "activityAtMs":150
    }));
    assert_eq!(unknown["success"], true, "{unknown}");
    assert_eq!(
        unknown["data"]["taskMetadata"]["attribution"], "unknown",
        "{unknown}"
    );
    for field in [
        "agentName",
        "humanName",
        "originKind",
        "purpose",
        "requestId",
        "jobId",
    ] {
        assert!(
            unknown["data"]["taskMetadata"][field].is_null(),
            "{unknown}"
        );
    }
    assert_eq!(
        unknown["data"]["taskMetadata"]["createdAtMs"], 140,
        "{unknown}"
    );
}

#[test]
fn pressure_configuration_is_opt_in_and_invalid_edits_preserve_state() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let alice = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/alice", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    let id = alice["browserSession"]["handoffId"].as_str().unwrap();
    let tab_id = alice["browserSession"]["tabId"].as_str().unwrap();
    assert!(consumer
        .state()
        .tab_lifetimes
        .get(tab_id)
        .is_none_or(|policy| policy.pressure_inactivity_ms.is_none()));
    for invalid in [
        serde_json::json!(-1),
        serde_json::json!("30"),
        serde_json::json!(u64::MAX),
    ] {
        let before = consumer.state().clone();
        let edit = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"set_tab_lifetime", "tabInactivityMs":0,
            "pressureInactivityMs":invalid, "activityAtMs":100
        }));
        assert_eq!(edit["success"], false, "{edit}");
        assert_eq!(
            edit["error"], "browser_tab_pressure_lifetime_invalid",
            "{edit}"
        );
        assert_eq!(consumer.state(), &before);
        assert_eq!(calls.get(), 1);
    }
    let edit = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":0,
        "pressureInactivityMs":30, "activityAtMs":100
    }));
    assert_eq!(edit["success"], true, "{edit}");
    drop(consumer);
    let mut consumer = host(&fixture, provider, calls.clone());
    let edit = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":60, "activityAtMs":110
    }));
    assert_eq!(edit["success"], true, "{edit}");
    assert_eq!(
        consumer.state().tab_lifetimes[tab_id].pressure_inactivity_ms,
        Some(30)
    );
    let edit = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":60,
        "pressureInactivityMs":0, "activityAtMs":110
    }));
    assert_eq!(edit["success"], true, "{edit}");
    assert_eq!(
        consumer.state().tab_lifetimes[tab_id].pressure_inactivity_ms,
        None
    );
    assert_eq!(
        consumer.state().tab_lifetimes[tab_id].inactivity_ms,
        Some(60)
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn configured_pressure_retires_inactive_tab_without_revoking_its_link() {
    let fixture = Fixture::new();
    let store = fixture.store(false);
    let mut catalog = store.load_profile_catalog().unwrap().catalog;
    catalog.disposable_policies.insert(
        "default".into(),
        BrowserDisposableProfilePolicy {
            id: "default".into(),
            user_data_root: fixture
                .root
                .join("disposable")
                .to_string_lossy()
                .into_owned(),
            cleanup_delay_ms: 300_000,
            maximum_retained_profiles: 1,
            maximum_total_bytes: 1024 * 1024,
        },
    );
    store.save_profile_catalog(&catalog).unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let alice = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/alice", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    let id = alice["browserSession"]["handoffId"].as_str().unwrap();
    let configured = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":0,
        "pressureInactivityMs":30, "activityAtMs":100
    }));
    assert_eq!(configured["success"], true, "{configured}");
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let attempt = |consumer: &mut BrowserSessionHost<_, _>, at| {
        consumer.execute_managed_command(
            "Bob",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/bob", "activityAtMs":at
            }),
        )
    };
    assert_eq!(
        attempt(&mut consumer, 129).unwrap_err(),
        "browser_disposable_profile_count_quota_protected"
    );
    let hold = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"prepare_human", "pendingHumanRetentionMs":50, "activityAtMs":100
    }));
    assert_eq!(hold["success"], true, "{hold}");
    assert_eq!(
        attempt(&mut consumer, 130).unwrap_err(),
        "browser_disposable_profile_count_quota_protected"
    );
    let desktop = provider.borrow().assignments[0].desktop_id.clone();
    provider
        .borrow_mut()
        .viewer_active_desktops
        .insert(desktop.clone());
    assert_eq!(
        attempt(&mut consumer, 150).unwrap_err(),
        "browser_disposable_profile_count_quota_protected"
    );
    assert!(consumer
        .state()
        .tabs
        .contains_key(alice["browserSession"]["tabId"].as_str().unwrap()));
    assert_eq!(calls.get(), 1);
    provider
        .borrow_mut()
        .viewer_active_desktops
        .remove(&desktop);
    let next = consumer.execute_managed_command(
        "Bob",
        &serde_json::json!({
            "action":"navigate", "url":"https://synthetic.example/bob", "activityAtMs":151
        }),
    );
    assert!(
        next.is_ok(),
        "eligible inactive task must free configured quota: {next:?}"
    );
    let bob = next.unwrap().unwrap();
    assert_ne!(
        bob["browserSession"]["browserId"],
        alice["browserSession"]["browserId"]
    );
    assert!(!consumer
        .state()
        .tabs
        .contains_key(alice["browserSession"]["tabId"].as_str().unwrap()));
    drop(consumer);
    let mut consumer = host(&fixture, provider, calls.clone());
    let link = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":152
    }));
    assert_eq!(link["success"], true, "{link}");
    assert_eq!(link["data"]["expired"], false, "{link}");
    assert!(link["data"]["expiresAtMs"].is_null());
    assert_eq!(calls.get(), 2);
}

#[test]
fn automatic_last_tab_expiry_closes_owned_browser_with_empty_peer_session() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let alice = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let id = alice["browserSession"]["handoffId"].as_str().unwrap();
    let configured = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":30, "activityAtMs":100
    }));
    assert_eq!(configured["success"], true, "{configured}");
    let peer = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-a", 110))
        .unwrap();
    assert!(consumer.state().sessions[&peer.session_id]
        .current_tab_id
        .is_none());
    assert!(!consumer
        .state()
        .tabs
        .values()
        .any(|tab| tab.session_id == peer.session_id));
    let desktop = provider.borrow().assignments[0].desktop_id.clone();
    provider
        .borrow_mut()
        .viewer_active_desktops
        .insert(desktop.clone());
    let protected = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_reap", "activityAtMs":130
    }));
    assert_eq!(protected["success"], true, "{protected}");
    assert_eq!(
        protected["data"]["expiredTabIds"],
        serde_json::json!([]),
        "{protected}"
    );
    assert_eq!(
        protected["data"]["closedBrowserIds"],
        serde_json::json!([]),
        "{protected}"
    );
    assert!(consumer.state().sessions[&peer.session_id]
        .current_tab_id
        .is_none());
    provider
        .borrow_mut()
        .viewer_active_desktops
        .remove(&desktop);
    let reaped = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_reap", "activityAtMs":131
    }));
    assert_eq!(reaped["success"], true, "{reaped}");
    assert_eq!(
        reaped["data"]["expiredTabIds"],
        serde_json::json!([alice["browserSession"]["tabId"]]),
        "{reaped}"
    );
    assert_eq!(
        reaped["data"]["closedBrowserIds"],
        serde_json::json!([alice["browserSession"]["browserId"]]),
        "{reaped}"
    );
    assert!(consumer.state().sessions.is_empty());
    assert!(consumer.state().browsers.is_empty());
    drop(consumer);
    let mut consumer = host(&fixture, provider, calls.clone());
    let link = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":132
    }));
    assert_eq!(link["success"], true, "{link}");
    assert_eq!(link["data"]["expired"], false, "{link}");
    assert_eq!(calls.get(), 1);
}

#[test]
fn closing_last_live_tab_closes_browser_without_deleting_handoff() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let closed = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_tab_close", "sessionName":"Alice", "profileId":"profile-a", "activityAtMs":110
    }));
    assert_eq!(closed["success"], true, "{closed}");
    let status = consumer.handle_command(&serde_json::json!({"action":"browser_session_status"}));
    assert_eq!(
        status["data"]["browsers"],
        serde_json::json!({}),
        "{status}"
    );
    drop(consumer);
    let mut consumer = host(&fixture, provider, calls);
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":opened["browserSession"]["handoffId"],
        "handoffOperation":"inspect", "activityAtMs":111
    }));
    assert_eq!(inspected["success"], true, "{inspected}");
    assert_eq!(inspected["data"]["expired"], false);
    assert!(inspected["data"]["expiresAtMs"].is_null());
}

#[test]
fn configured_tab_lifetime_expires_only_its_target_and_keeps_the_link() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let alice = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/alice", "profileId":"profile-a", "activityAtMs":100
    })).unwrap().unwrap();
    let bob = consumer.execute_managed_command("Bob", &serde_json::json!({
        "action":"navigate", "url":"https://synthetic.example/bob", "profileId":"profile-a", "activityAtMs":120
    })).unwrap().unwrap();
    let id = alice["browserSession"]["handoffId"]
        .as_str()
        .unwrap()
        .to_owned();
    let configured = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":30, "activityAtMs":120
    }));
    assert_eq!(configured["success"], true, "{configured}");
    assert_eq!(
        configured["data"]["status"], "tab_lifetime_updated",
        "{configured}"
    );
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let before = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_reap", "activityAtMs":129
    }));
    assert_eq!(before["data"]["expiredTabIds"], serde_json::json!([]));
    let expired = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_reap", "activityAtMs":130
    }));
    assert_eq!(expired["success"], true, "{expired}");
    assert_eq!(
        expired["data"]["expiredTabIds"],
        serde_json::json!([alice["browserSession"]["tabId"]])
    );
    assert_eq!(expired["data"]["closedBrowserIds"], serde_json::json!([]));
    let inspected = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"inspect", "activityAtMs":130
    }));
    assert_eq!(inspected["data"]["expired"], false);
    assert!(inspected["data"]["expiresAtMs"].is_null());
    let attempts = || {
        std::fs::read_to_string(fixture.root.join("tab-acquisition-attempts"))
            .unwrap()
            .parse::<u32>()
            .unwrap()
    };
    let before_recreation = attempts();
    drop(consumer);
    let mut unknown = host_mode(&fixture, provider.clone(), calls.clone(), Mode::UnknownTab);
    let reopen = serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "nativeDesktop":true, "activityAtMs":131
    });
    let interrupted = unknown.handle_command(&reopen);
    assert_eq!(interrupted["success"], false, "{interrupted}");
    assert!(
        interrupted["error"]
            .as_str()
            .unwrap()
            .contains("synthetic_tab_reply_lost"),
        "{interrupted}"
    );
    assert_eq!(attempts(), before_recreation + 1);
    let pending = fixture.store(false).load_session_state().unwrap();
    let request_id = pending.remote_view_tab_handoffs[&id]
        .recreation
        .as_ref()
        .unwrap()
        .request_id
        .clone();
    assert!(pending.managed_tab_requests[&request_id].response.is_none());
    assert_eq!(pending.tabs.len(), 1);
    let requests = provider.borrow().view_requests.clone();
    drop(unknown);
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    for now in [132, 133] {
        let mut retry = reopen.clone();
        retry["activityAtMs"] = now.into();
        let blocked = consumer.handle_command(&retry);
        assert_eq!(blocked["success"], false, "{blocked}");
        assert!(
            blocked["error"]
                .as_str()
                .unwrap()
                .contains("browser_session_tab_creation_readback_required"),
            "{blocked}"
        );
        assert_eq!(fixture.store(false).load_session_state().unwrap(), pending);
        assert_eq!(
            calls.get(),
            1,
            "unknown recreation must not launch a replacement"
        );
        assert_eq!(provider.borrow().view_requests, requests);
        assert_eq!(
            attempts(),
            before_recreation + 1,
            "unknown acquisition must not be replayed"
        );
    }
    let continued = consumer
        .execute_managed_command(
            "Bob",
            &serde_json::json!({
                "action":"get_url", "profileId":"profile-a", "activityAtMs":134
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        continued["browserSession"]["tabId"],
        bob["browserSession"]["tabId"]
    );
    assert_eq!(
        continued["browserSession"]["targetId"],
        bob["browserSession"]["targetId"]
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn disposable_inactivity_expires_work_and_reaps_only_unreferenced_profile_data() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = serde_json::to_value(&original).unwrap();
    configured["retentionPolicy"] = serde_json::json!({"durableHandoffTtlMs":null, "disposableHandoffTtlMs":20, "disposableInactivityMs":30});
    store
        .compare_and_save_session_state(&original, &serde_json::from_value(configured).unwrap())
        .unwrap();
    let mut consumer = host(&fixture, provider, calls);
    let opened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    let identity = &opened["browserSession"];
    let session = identity["sessionId"].as_str().unwrap().to_owned();
    let profile = identity["profileId"].as_str().unwrap().to_owned();
    assert_eq!(identity["handoffExpiresAtMs"], 120);
    assert!(consumer.reap(129).unwrap().expired_session_ids.is_empty());
    assert_eq!(
        consumer.reap(130).unwrap().expired_session_ids,
        vec![session]
    );
    assert!(consumer.state().disposable_profiles.contains_key(&profile));
    assert!(consumer
        .reap(300_129)
        .unwrap()
        .deleted_disposable_profile_ids
        .is_empty());
    assert_eq!(
        consumer
            .reap(300_130)
            .unwrap()
            .deleted_disposable_profile_ids,
        vec![profile]
    );
    assert!(consumer.state().disposable_profiles.is_empty());
}

#[test]
fn native_overflow_wraps_after_restart_without_same_profile_advancing_cursor() {
    let fixture = Fixture::new();
    let mut catalog = fixture.store(false).load_profile_catalog().unwrap().catalog;
    for id in ["profile-c", "profile-d", "profile-e"] {
        catalog.profiles.insert(
            id.into(),
            BrowserProfileCatalogEntry {
                manual_login_url: None,
                id: id.into(),
                name: id.into(),
                user_data_dir: fixture.root.join(id).to_string_lossy().into_owned(),
                kind: BrowserProfileKind::Named,
            },
        );
    }
    fixture.store(false).save_profile_catalog(&catalog).unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let mut opened = Vec::new();
    for (i, profile) in ["profile-a", "profile-b", "profile-c", "profile-d"]
        .iter()
        .enumerate()
    {
        let item = consumer
            .open(OpenBrowserSession::exact_profile(
                format!("Task{i}"),
                *profile,
                100 + i as u64,
            ))
            .unwrap();
        let desktop = consumer.state().browsers[&item.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id
            .clone();
        provider
            .borrow_mut()
            .occupied_desktops
            .insert(desktop.clone());
        opened.push((item, desktop));
    }
    assert_ne!(opened[0].1, opened[1].1);
    assert_eq!(opened[2].1, opened[0].1);
    assert_eq!(opened[3].1, opened[1].1);
    consumer
        .close_session(
            &opened[1].0.session_id,
            SessionEndReason::ExplicitClose,
            110,
        )
        .unwrap();
    let peer = consumer
        .open(OpenBrowserSession::exact_profile("Peer", "profile-a", 111))
        .unwrap();
    assert_eq!(peer.browser_id, opened[0].0.browser_id);
    drop(consumer);
    let mut consumer = host(&fixture, provider.clone(), calls);
    let next = consumer
        .open(OpenBrowserSession::exact_profile("Next", "profile-e", 112))
        .unwrap();
    assert_eq!(
        consumer.state().browsers[&next.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id,
        opened[0].1
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn background_pool_replenishes_one_spare_without_launching_or_duplicating() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let mut maintenance = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    assert!(
        replenish_remote_view_session_pool(&mut maintenance, &mut store, &pool)
            .unwrap()
            .is_none()
    );
    assert!(provider.borrow().acquisitions.is_empty());
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let first = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let desktop = consumer.state().browsers[&first.browser_id]
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider.borrow_mut().occupied_desktops.insert(desktop);
    let spare = replenish_remote_view_session_pool(&mut maintenance, &mut store, &pool)
        .unwrap()
        .unwrap();
    assert_eq!(spare.assignments.len(), 2);
    assert_eq!(spare.desktops.len(), 1);
    assert_eq!(calls.get(), 1);
    let second = replenish_remote_view_session_pool(&mut maintenance, &mut store, &pool)
        .unwrap()
        .unwrap();
    assert_eq!(second.desktops, spare.desktops);
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    let next = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 101))
        .unwrap();
    assert_eq!(
        consumer.state().browsers[&next.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id,
        spare.desktops[0].desktop.desktop_id
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn unknown_spare_acquisition_cannot_block_a_separately_confirmed_free_desktop() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let first = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let desktop = consumer.state().browsers[&first.browser_id]
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(desktop.clone());
    provider.borrow_mut().lose_reply = true;
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    assert!(replenish_remote_view_session_pool(
        &mut adapter(&fixture, provider.clone()),
        &mut fixture.store(false),
        &pool
    )
    .is_err());
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    consumer
        .close_session(&first.session_id, SessionEndReason::ExplicitClose, 101)
        .unwrap();
    provider.borrow_mut().occupied_desktops.remove(&desktop);
    let next = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 102))
        .unwrap();
    assert_eq!(
        consumer.state().browsers[&next.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id,
        desktop
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    assert_eq!(
        fixture
            .store(false)
            .pending_pool_acquisitions("agent-browser", "main")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn configured_handoff_overflow_is_rejected_before_profile_or_browser_effects() {
    let fixture = Fixture::new();
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = original.clone();
    configured.retention_policy.durable_handoff_ttl_ms = Some(u64::MAX);
    store
        .compare_and_save_session_state(&original, &configured)
        .unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    assert!(consumer.execute_managed_command("Alice",&serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/","profileId":"profile-a","activityAtMs":100
    })).is_err());
    assert_eq!(calls.get(), 0);
    assert!(provider.borrow().acquisitions.is_empty());
    assert!(consumer.state().sessions.is_empty());
}

#[test]
fn native_idle_pool_returns_capacity_and_recovers_same_logical_handoff() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        pool_maximum: Some(3),
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let mut opened = Vec::new();
    for (name, profile) in [("Alice", "profile-a"), ("Bob", "profile-b")] {
        let response = consumer
            .execute_managed_command(
                name,
                &serde_json::json!({"action":"navigate",
            "url":"https://synthetic.example/","profileId":profile,"activityAtMs":100}),
            )
            .unwrap()
            .unwrap();
        let browser = response["browserSession"]["browserId"].as_str().unwrap();
        provider.borrow_mut().occupied_desktops.insert(
            consumer.state().browsers[browser]
                .desktop
                .as_ref()
                .unwrap()
                .desktop_id
                .clone(),
        );
        opened.push(response);
    }
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    replenish_remote_view_session_pool(
        &mut adapter(&fixture, provider.clone()),
        &mut fixture.store(false),
        &pool,
    )
    .unwrap();
    assert_eq!(provider.borrow().acquisitions.len(), 3);
    assert_eq!(consumer.reap(300_100).unwrap().closed_browser_ids.len(), 2);
    provider.borrow_mut().occupied_desktops.clear();
    super::super::browser_session_pool_maintenance::maintain_pool(
        &mut adapter(&fixture, provider.clone()),
        &mut fixture.store(false),
        &pool,
    )
    .unwrap();
    assert_eq!(
        provider
            .borrow()
            .assignments
            .iter()
            .filter(|a| a.state == RemoteViewAssignmentState::Active)
            .count(),
        1
    );
    drop(consumer);
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let resumed = consumer
        .execute_managed_command(
            "Bob",
            &serde_json::json!({"action":"get_url","activityAtMs":300_101}),
        )
        .unwrap()
        .unwrap();
    for field in ["handoffId", "browserId", "sessionId", "tabId"] {
        assert_eq!(
            resumed["browserSession"][field],
            opened[1]["browserSession"][field]
        );
    }
    assert_ne!(
        resumed["browserSession"]["targetId"],
        opened[1]["browserSession"]["targetId"]
    );
    assert_eq!(calls.get(), 3);
}

#[test]
fn native_idle_cleanup_keeps_viewer_attached_browser_running() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(&fixture, provider.clone(), calls, Mode::IdleRecovery);
    consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({"action":"navigate","profileId":"profile-a",
        "url":"https://synthetic.example/","activityAtMs":100}),
        )
        .unwrap()
        .unwrap();
    let desktop = consumer
        .state()
        .browsers
        .values()
        .next()
        .unwrap()
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider
        .borrow_mut()
        .viewer_active_desktops
        .insert(desktop.clone());
    assert!(consumer
        .reap(300_100)
        .unwrap()
        .closed_browser_ids
        .is_empty());
    assert!(!fixture.root.join("idle-process-absent").exists());
    provider
        .borrow_mut()
        .viewer_active_desktops
        .remove(&desktop);
    assert_eq!(consumer.reap(300_101).unwrap().closed_browser_ids.len(), 1);
}

#[test]
fn native_idle_release_deferral_and_lost_reply_preserve_exact_custody() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        pool_maximum: Some(3),
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls);
    let first = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    provider.borrow_mut().occupied_desktops.insert(
        consumer.state().browsers[&first.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id
            .clone(),
    );
    let second = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 100))
        .unwrap();
    let desktop = consumer.state().browsers[&second.browser_id]
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider
        .borrow_mut()
        .occupied_desktops
        .insert(desktop.clone());
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let mut coordinator = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    replenish_remote_view_session_pool(&mut coordinator, &mut store, &pool).unwrap();
    consumer
        .close_session(&second.session_id, SessionEndReason::ExplicitClose, 101)
        .unwrap();
    provider.borrow_mut().occupied_desktops.remove(&desktop);
    provider.borrow_mut().defer_idle_release = true;
    super::super::browser_session_pool_maintenance::maintain_pool(
        &mut coordinator,
        &mut store,
        &pool,
    )
    .unwrap();
    assert!(store.idle_release_obligations().unwrap().is_empty());
    assert_eq!(
        provider
            .borrow()
            .assignments
            .iter()
            .filter(|a| a.state == RemoteViewAssignmentState::Active)
            .count(),
        3
    );
    provider.borrow_mut().defer_idle_release = false;
    provider.borrow_mut().lose_idle_release_reply = true;
    assert_eq!(
        super::super::browser_session_pool_maintenance::maintain_pool(
            &mut coordinator,
            &mut store,
            &pool
        )
        .unwrap_err(),
        "remote_view_release_readback_required"
    );
    assert_eq!(store.idle_release_obligations().unwrap().len(), 1);
    provider.borrow_mut().lose_idle_release_reply = false;
    super::super::browser_session_pool_maintenance::maintain_pool(
        &mut coordinator,
        &mut store,
        &pool,
    )
    .unwrap();
    assert!(store.idle_release_obligations().unwrap().is_empty());
    let requests = &provider.borrow().idle_release_requests;
    assert_eq!(requests.len(), 3);
    assert_ne!(requests[0], requests[1]);
    assert_eq!(requests[1], requests[2]);
    assert_eq!(provider.borrow().acquisitions.len(), 3);
}

#[test]
fn canceled_idle_release_permit_cannot_submit_after_work_resumes() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut coordinator = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let prepared = prepare_remote_view_session_pool(
        &mut coordinator,
        &mut store,
        &RemoteViewSessionPool {
            name: "main".into(),
            desired_desktops: 1,
        },
    )
    .unwrap();
    let assignment = &prepared.assignments[0];
    let observation = coordinator.observe_idle(assignment).unwrap();
    let target = observation.release_target(assignment).unwrap();
    let clear = RemoteViewApplicationObligationInventory::Complete(Vec::new());
    let snapshot = RemoteViewApplicationCleanupSnapshot {
        schema_version: 1,
        assignment_id: assignment.assignment_id.clone(),
        desktop_id: assignment.desktop_id.clone(),
        lifecycle_generation: assignment.generation,
        pending_recovery: clear.clone(),
        foreground_leases: clear.clone(),
        cleanup_tasks: clear,
    };
    let release_id = uuid::Uuid::new_v4().to_string();
    let permit = store
        .admit_idle_assignment_release(&target, &snapshot, &release_id)
        .unwrap();
    assert!(!store.pool_assignment_available(assignment).unwrap());
    store
        .cancel_unsubmitted_or_deferred_idle_release(&release_id)
        .unwrap();
    assert!(store.pool_assignment_available(assignment).unwrap());
    assert!(coordinator
        .release_idle(
            assignment,
            &observation,
            permit,
            release_id.into(),
            &mut store
        )
        .is_err());
    assert!(provider.borrow().idle_release_requests.is_empty());
    assert_eq!(
        provider.borrow().assignments[0].state,
        RemoteViewAssignmentState::Active
    );
}

#[test]
fn native_idle_viewer_protects_disposable_expiry_and_profile_data() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut store = fixture.store(false);
    let original = store.load_session_state().unwrap();
    let mut configured = serde_json::to_value(&original).unwrap();
    configured["retentionPolicy"] = serde_json::json!({
        "durableHandoffTtlMs":null, "disposableHandoffTtlMs":20, "disposableInactivityMs":30});
    store
        .compare_and_save_session_state(&original, &serde_json::from_value(configured).unwrap())
        .unwrap();
    let mut consumer = host_mode(&fixture, provider.clone(), calls, Mode::IdleRecovery);
    let opened = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    let browser_id = opened["browserSession"]["browserId"].as_str().unwrap();
    let desktop = consumer.state().browsers[browser_id]
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider
        .borrow_mut()
        .viewer_active_desktops
        .insert(desktop.clone());
    assert!(consumer.reap(130).unwrap().expired_session_ids.is_empty());
    assert!(!fixture.root.join("idle-process-absent").exists());
    assert_eq!(consumer.state().disposable_profiles.len(), 1);
    provider
        .borrow_mut()
        .viewer_active_desktops
        .remove(&desktop);
    assert_eq!(consumer.reap(131).unwrap().expired_session_ids.len(), 1);
    assert_eq!(
        consumer
            .reap(300_131)
            .unwrap()
            .deleted_disposable_profile_ids
            .len(),
        1
    );
}

#[test]
fn pool_demand_regrows_after_exact_native_release_without_local_release_receipt() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut adapter = adapter(&fixture, provider.clone());
    let mut store = fixture.store(false);
    let pool = RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    };
    let first = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    provider.borrow_mut().assignments[0].state = RemoteViewAssignmentState::Released;
    let next = prepare_remote_view_session_pool(&mut adapter, &mut store, &pool).unwrap();
    assert_ne!(
        first.assignments[0].assignment_id,
        next.assignments[0].assignment_id
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
    let history = store
        .confirmed_pool_acquisitions("agent-browser", "main")
        .unwrap();
    assert_eq!(history[0].state, RemoteViewAssignmentState::Active);
    let mut changed = provider.borrow_mut();
    changed.assignments[1].state = RemoteViewAssignmentState::Released;
    changed.assignments[1].generation += 1;
    drop(changed);
    assert_eq!(
        prepare_remote_view_session_pool(&mut adapter, &mut store, &pool)
            .err()
            .unwrap(),
        "remote_view_pool_acquisition_readback_required"
    );
    assert_eq!(provider.borrow().acquisitions.len(), 2);
}

#[test]
fn reaper_observation_failure_does_not_starve_unrelated_expiry() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        window_pid: Some(43),
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls);
    let held = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/held",
                "profileId":"profile-a", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    let id = held["browserSession"]["handoffId"].as_str().unwrap();
    let held_tab = held["browserSession"]["tabId"].as_str().unwrap();
    let prepared = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":id,
        "handoffOperation":"prepare_human", "pendingHumanRetentionMs":0, "activityAtMs":100
    }));
    assert_eq!(prepared["success"], true, "{prepared}");
    let peer = consumer
        .execute_managed_command(
            "Bob",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/expired",
                "profileId":"profile-b", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    let configured = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":peer["browserSession"]["handoffId"],
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":30, "activityAtMs":100
    }));
    assert_eq!(configured["success"], true, "{configured}");
    provider.borrow_mut().lose_native_registration_reply = true;
    let error = consumer
        .reap(130)
        .expect_err("provider failure must remain observable");
    assert_eq!(error, "remote_view_native_registration_unavailable");
    assert!(consumer.state().pending_human_tabs.contains_key(held_tab));
    let durable = fixture.store(false).load_session_state().unwrap();
    assert!(!durable
        .sessions
        .contains_key(peer["browserSession"]["sessionId"].as_str().unwrap()));
}

#[test]
fn native_use_owner_reconciles_registration_restart_generation_and_publication_conflict() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider {
        window_pid: Some(43),
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/native-use","profileId":"profile-a",
        "activityAtMs":100,"humanHandoff":false,"pendingHumanRetentionMs":0
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let tab = opened["browserSession"]["tabId"].as_str().unwrap();
    let prepare = |host: &mut BrowserSessionHost<_, _>, at| {
        host.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"prepare_native_use",
        "expectedTabId":tab,"expectedTargetId":opened["browserSession"]["targetId"],"activityAtMs":at
    }))
    };
    let observe = |host: &mut BrowserSessionHost<_, _>, at| {
        host.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"observe_native_use","activityAtMs":at
    }))
    };
    let held = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,
        "handoffOperation":"prepare_human","pendingHumanRetentionMs":0,"activityAtMs":105
    }));
    assert_eq!(held["success"], true);
    provider.borrow_mut().lose_native_registration_reply = true;
    let lost = prepare(&mut consumer, 110);
    assert_eq!(lost["success"], false, "{lost}");
    let intent = consumer.state().remote_view_tab_handoffs[id]
        .native_observation
        .clone()
        .unwrap();
    assert!(intent.observation.is_none());
    assert_eq!(provider.borrow().native_observations.len(), 1);
    drop(consumer);
    provider.borrow_mut().lose_native_registration_reply = false;
    consumer = host(&fixture, provider.clone(), calls.clone());
    let resumed = prepare(&mut consumer, 111);
    assert_eq!(resumed["success"], true, "{resumed}");
    assert_eq!(
        consumer.state().remote_view_tab_handoffs[id]
            .native_observation
            .as_ref()
            .unwrap()
            .idempotency_key,
        intent.idempotency_key
    );
    assert_eq!(
        provider.borrow().native_observations.len(),
        1,
        "lost reply must replay the exact registration"
    );
    assert_eq!(
        observe(&mut consumer, 112)["data"]["status"],
        "native_use_pending"
    );
    assert!(consumer.state().pending_human_tabs.contains_key(tab));
    // Producer retention can remove the context entirely. The owner must
    // recreate only that short context, preserving the exact task and hold.
    let link_before_missing = consumer.state().remote_view_tab_handoffs[id].clone();
    let hold_before_missing = consumer.state().pending_human_tabs[tab].clone();
    provider.borrow_mut().native_observations.clear();
    consumer.reap(112).unwrap();
    let refreshed = prepare(&mut consumer, 112);
    assert_eq!(refreshed["success"], true, "{refreshed}");
    assert_ne!(
        refreshed["data"]["correlationId"],
        resumed["data"]["correlationId"]
    );
    let link_after_missing = &consumer.state().remote_view_tab_handoffs[id];
    assert_eq!(link_after_missing.tab_id, link_before_missing.tab_id);
    assert_eq!(
        link_after_missing.created_at_ms,
        link_before_missing.created_at_ms
    );
    assert_eq!(
        link_after_missing.expires_at_ms,
        link_before_missing.expires_at_ms
    );
    assert_eq!(
        consumer.state().pending_human_tabs[tab],
        hold_before_missing
    );
    assert_eq!(calls.get(), 1, "refresh must not launch another browser");
    provider.borrow_mut().native_observation_expired = true;
    assert_eq!(
        observe(&mut consumer, 113)["data"]["status"],
        "native_use_expired"
    );
    assert!(consumer.state().pending_human_tabs.contains_key(tab));
    // The real owner reaper renews short provider context without extending
    // the perpetual link or releasing a pending-human hold.
    let held_before_refresh = consumer.state().pending_human_tabs[tab].clone();
    consumer.reap(114).unwrap();
    assert_eq!(
        consumer.state().pending_human_tabs[tab],
        held_before_refresh
    );
    provider.borrow_mut().native_observation_expired = false;
    let fresh = prepare(&mut consumer, 115);
    assert_eq!(fresh["success"], true, "{fresh}");
    assert_ne!(
        fresh["data"]["correlationId"],
        resumed["data"]["correlationId"]
    );
    provider.borrow_mut().native_observation_expired = false;
    provider.borrow_mut().native_viewing_generation = Some(999);
    let stale = observe(&mut consumer, 115);
    assert_eq!(stale["success"], false, "{stale}");
    assert!(consumer.state().pending_human_tabs.contains_key(tab));
    let relocated = prepare(&mut consumer, 116);
    assert_eq!(relocated["success"], true, "{relocated}");
    assert_ne!(
        relocated["data"]["correlationId"],
        fresh["data"]["correlationId"]
    );
    assert!(consumer.state().remote_view_tab_handoffs[id]
        .expires_at_ms
        .is_none());
    provider.borrow_mut().native_used_at_ms = Some(150);
    provider.borrow_mut().native_competing_publication = true;
    let conflict = observe(&mut consumer, 151);
    assert_eq!(conflict["success"], false, "{conflict}");
    let durable = fixture.store(false).load_session_state().unwrap();
    assert!(durable.pending_human_tabs.contains_key(tab));
    assert!(durable.tab_metadata[tab].first_human_use_at_ms.is_none());
    drop(consumer);
    consumer = host(&fixture, provider.clone(), calls.clone());
    consumer.reap(160).unwrap();
    assert!(!consumer.state().pending_human_tabs.contains_key(tab));
    assert_eq!(
        consumer.state().tab_metadata[tab].first_human_use_at_ms,
        Some(150)
    );
    let duplicate = observe(&mut consumer, 170);
    assert_eq!(
        duplicate["data"]["status"], "human_use_already_confirmed",
        "{duplicate}"
    );
    assert_eq!(
        consumer.state().tab_metadata[tab].first_human_use_at_ms,
        Some(150)
    );
    assert!(
        provider.borrow().view_requests.is_empty(),
        "ordinary native use must issue no viewer grant"
    );
    assert_eq!(calls.get(), 1);
}

/// Inject only the publication reply boundary; all database operations and
/// provider custody continue through the actual SQLite store.
struct NativeUseReplyFailure {
    store: BrowserSessionSqliteStore,
    mode: Rc<Cell<u8>>,
    attempts: Rc<Cell<u32>>,
}
impl BrowserSessionPersistence for NativeUseReplyFailure {
    fn publish_session_effects(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        intent: Option<&BrowserLaunchIntent>,
        retirement_ids: &[String],
        activity_ids: &[String],
    ) -> Result<(), String> {
        if self.mode.get() == 5 && !retirement_ids.is_empty() {
            self.mode.set(0);
            self.attempts.set(self.attempts.get() + 1);
            return Err("synthetic_retirement_publication_unavailable".into());
        }
        if self.mode.get() == 3 && !activity_ids.is_empty() {
            self.mode.set(0);
            self.attempts.set(self.attempts.get() + 1);
            return Err("synthetic_activity_publication_unavailable".into());
        }
        if self.mode.get() == 4 && !activity_ids.is_empty() {
            self.mode.set(0);
            self.attempts.set(self.attempts.get() + 1);
            BrowserSessionPersistence::publish_session_effects(
                &mut self.store,
                expected,
                state,
                intent,
                retirement_ids,
                activity_ids,
            )?;
            return Err("synthetic_activity_publication_reply_lost".into());
        }
        let publishes_use = state.tab_metadata.iter().any(|(id, metadata)| {
            metadata.first_human_use_at_ms.is_some()
                && expected
                    .tab_metadata
                    .get(id)
                    .is_none_or(|old| old.first_human_use_at_ms.is_none())
        });
        if publishes_use && self.mode.get() != 0 {
            let mode = self.mode.replace(0);
            self.attempts.set(self.attempts.get() + 1);
            if mode == 2 {
                BrowserSessionPersistence::publish_session_effects(
                    &mut self.store,
                    expected,
                    state,
                    intent,
                    retirement_ids,
                    activity_ids,
                )?;
                return Err("synthetic_native_use_reply_lost_after_commit".into());
            }
            return Err("synthetic_native_use_publication_unavailable".into());
        }
        BrowserSessionPersistence::publish_session_effects(
            &mut self.store,
            expected,
            state,
            intent,
            retirement_ids,
            activity_ids,
        )
    }
    fn publish_session_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
        intent: Option<&BrowserLaunchIntent>,
    ) -> Result<(), String> {
        self.publish_session_effects(expected, state, intent, &[], &[])
    }
    fn load_session_state(&self) -> Result<BrowserSessionState, String> {
        self.store.load_session_state()
    }
    fn compare_and_save_session_state(
        &mut self,
        expected: &BrowserSessionState,
        state: &BrowserSessionState,
    ) -> Result<(), String> {
        self.store.compare_and_save_session_state(expected, state)
    }
    fn load_or_import_profile_catalog(
        &self,
        _path: &Path,
    ) -> Result<BrowserProfileCatalogLoad, String> {
        self.store.load_profile_catalog()
    }
    fn save_profile_catalog(&self, catalog: &BrowserProfileCatalog) -> Result<(), String> {
        self.store.save_profile_catalog(catalog)
    }
}

#[test]
fn native_use_publication_failure_preserves_durable_truth_and_exact_retry() {
    for fault in [1, 2] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider {
            window_pid: Some(43),
            ..Provider::default()
        }));
        let calls = Rc::new(Cell::new(0));
        let mode = Rc::new(Cell::new(0));
        let attempts = Rc::new(Cell::new(0));
        let mut consumer = host_with_persistence(
            &fixture,
            provider.clone(),
            calls.clone(),
            Mode::Success,
            || 2001,
            NativeUseReplyFailure {
                store: fixture.store(false),
                mode: mode.clone(),
                attempts: attempts.clone(),
            },
        );
        let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
            "action":"navigate","url":"https://synthetic.example/publication","profileId":"profile-a",
            "activityAtMs":100,"humanHandoff":true,"pendingHumanRetentionMs":0
        })).unwrap().unwrap();
        let id = opened["browserSession"]["handoffId"].as_str().unwrap();
        let tab = opened["browserSession"]["tabId"].as_str().unwrap();
        let registered = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"prepare_native_use",
            "expectedTabId":tab,"expectedTargetId":opened["browserSession"]["targetId"],"activityAtMs":110
        }));
        assert_eq!(registered["success"], true, "{registered}");
        provider.borrow_mut().native_used_at_ms = Some(150);
        mode.set(fault);
        let first = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"observe_native_use","activityAtMs":160
        }));
        assert_eq!(
            attempts.get(),
            1,
            "fault injection must reach first-use publication after focus"
        );
        let durable = fixture.store(false).load_session_state().unwrap();
        if fault == 1 {
            assert_eq!(first["success"], false, "{first}");
            assert!(durable.pending_human_tabs.contains_key(tab));
            assert!(durable.tab_metadata[tab].first_human_use_at_ms.is_none());
        } else {
            assert_eq!(
                first["success"], true,
                "exact durable readback must reconcile a lost publication reply: {first}"
            );
            assert!(!durable.pending_human_tabs.contains_key(tab));
            assert_eq!(durable.tab_metadata[tab].first_human_use_at_ms, Some(150));
        }
        let inspected = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"inspect","activityAtMs":161
        }));
        assert_eq!(inspected["success"], true, "{inspected}");
        assert_eq!(
            inspected["data"]["taskMetadata"]["firstHumanUseAtMs"],
            serde_json::to_value(durable.tab_metadata[tab].first_human_use_at_ms).unwrap(),
            "uncommitted first-use evidence leaked into Properties: {inspected}"
        );
        assert_eq!(
            inspected["data"]["pendingHuman"].is_null(),
            !durable.pending_human_tabs.contains_key(tab),
            "uncommitted hold release leaked into Properties"
        );
        let retry = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"observe_native_use","activityAtMs":170
        }));
        assert_eq!(retry["success"], true, "{retry}");
        let durable = fixture.store(false).load_session_state().unwrap();
        assert_eq!(durable.tab_metadata[tab].first_human_use_at_ms, Some(150));
        assert!(!durable.pending_human_tabs.contains_key(tab));
        assert!(durable.remote_view_tab_handoffs[id].expires_at_ms.is_none());
        assert_eq!(
            provider.borrow().native_observations.len(),
            1,
            "publication failure must not register another viewer attempt"
        );
        assert_eq!(calls.get(), 1);
        let continued = consumer.execute_managed_command("Alice", &serde_json::json!({
            "action":"navigate", "url":"https://synthetic.example/after-publication-readback",
            "profileId":"profile-a", "activityAtMs":180
        })).expect("publication readback must permit continued automation").unwrap();
        assert_eq!(continued["success"], true, "{continued}");
        assert_eq!(
            fixture.store(false).load_session_state().unwrap(),
            *consumer.state()
        );
    }
}

#[test]
fn native_use_processing_delay_does_not_replace_actual_use_with_poll_time() {
    for (newer_activity, peer_activity) in [(None, false), (Some(180), false), (None, true)] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider {
            window_pid: Some(43),
            ..Provider::default()
        }));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls);
        let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/delayed-use","profileId":"profile-a",
        "activityAtMs":100,"humanHandoff":true,"pendingHumanRetentionMs":0
    })).unwrap().unwrap();
        let id = opened["browserSession"]["handoffId"].as_str().unwrap();
        let tab = opened["browserSession"]["tabId"].as_str().unwrap();
        let registered = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"prepare_native_use",
        "expectedTabId":tab,"expectedTargetId":opened["browserSession"]["targetId"],"activityAtMs":110
    }));
        assert_eq!(registered["success"], true, "{registered}");
        if let Some(at_ms) = newer_activity {
            let session_id = consumer.state().tabs[tab].session_id.clone();
            consumer
                .navigate(&session_id, "https://synthetic.example/delayed-use", at_ms)
                .unwrap();
        }
        let session_id = consumer.state().tabs[tab].session_id.clone();
        let peer = if peer_activity {
            let created = consumer.new_tab(&session_id, 180).unwrap();
            consumer
                .navigate(&session_id, "https://synthetic.example/peer", 190)
                .unwrap();
            Some(created.tab_id)
        } else {
            None
        };
        let session_before = consumer.state().sessions[&session_id].clone();
        provider.borrow_mut().native_used_at_ms = Some(150);
        let used = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"observe_native_use","activityAtMs":200
    }));
        assert_eq!(used["success"], true, "{used}");
        assert_eq!(
            consumer.state().tabs[tab].last_activity_at_ms,
            newer_activity.unwrap_or(150),
            "private observation processing time replaced actual native human-use time"
        );
        assert_eq!(
            consumer.state().tab_metadata[tab].first_human_use_at_ms,
            Some(150)
        );
        let session = &consumer.state().sessions[&session_id];
        assert_eq!(session.current_tab_id.as_deref(), Some(tab));
        assert_eq!(
            session.last_activity_at_ms,
            session_before.last_activity_at_ms.max(150)
        );
        assert!(session.expires_at_ms >= session_before.expires_at_ms);
        if let Some(peer) = peer {
            assert_eq!(consumer.state().tabs[&peer].last_activity_at_ms, 190);
        }
    }
}

/// Opt-in cross-repository fixture. The native viewer and private producer run
/// in Remote View's isolated test process; physical browser custody is synthetic.
#[cfg(unix)]
#[test]
#[ignore = "requires the task-owned Remote View native viewer fixture"]
fn native_viewer_use_composes_with_actual_provider_owner() {
    let descriptor_path = std::env::var("P237_NATIVE_PRODUCER_DESCRIPTOR")
        .expect("synthetic producer descriptor required");
    let descriptor: Value =
        serde_json::from_slice(&std::fs::read(descriptor_path).unwrap()).unwrap();
    let status_path = std::path::PathBuf::from(descriptor["statusPath"].as_str().unwrap());
    let now = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    };
    // The public presentation handler opens the default SQLite database. Join
    // that exact database here rather than copying the owner's state into a
    // second store. This ignored fixture runs in its own synthetic child.
    assert_ne!(
        std::env::var("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME")
            .ok()
            .as_deref(),
        Some("1")
    );
    let database_path = BrowserSessionSqliteStore::default_sqlite_path().unwrap();
    assert!(
        !database_path.exists(),
        "composed child database already exists"
    );
    let root = database_path.parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&root).unwrap();
    // Use the production initialization boundary, including its migration
    // receipt, before adding the fixture's synthetic profiles and owner state.
    drop(BrowserSessionSqliteStore::default_sqlite().unwrap());
    let fixture = Fixture::with_root(root, false);
    let mut expected_environment = wire()["launchEnvironment"]["environment"].clone();
    expected_environment["REMOTE_VIEW_SLOT_GENERATION"] = descriptor["assignmentObservation"]
        ["target"]["viewingGeneration"]
        .to_string()
        .into();
    std::fs::write(
        fixture
            .root
            .join("native-composition-launch-environment.json"),
        serde_json::to_vec(&expected_environment).unwrap(),
    )
    .unwrap();
    let provider = Rc::new(RefCell::new(Provider {
        assignments: vec![serde_json::from_value(descriptor["assignment"].clone()).unwrap()],
        window_pid: Some(43),
        native_bridge: Some(NativeOwnerBridge {
            owner_url: descriptor["ownerUrl"].as_str().unwrap().into(),
            inventory: descriptor["inventory"].clone(),
            assignment_observation: descriptor["assignmentObservation"].clone(),
        }),
        ..Provider::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/composed-native-use","profileId":"profile-a",
        "activityAtMs":now(),"humanHandoff":true,"pendingHumanRetentionMs":0
    })).unwrap().unwrap();
    let id = opened["browserSession"]["handoffId"].as_str().unwrap();
    let tab = opened["browserSession"]["tabId"].as_str().unwrap();
    assert_eq!(
        BrowserSessionSqliteStore::default_sqlite()
            .unwrap()
            .load_session_state()
            .unwrap(),
        fixture.store(false).load_session_state().unwrap(),
        "public presentation reader must observe the actual owner database"
    );
    // The explicit last-browser mode requires complete target/browser expiry.
    // The default recovery case keeps a peer and proves it survives unchanged.
    let last_browser = descriptor["publicLastBrowser"].as_bool().unwrap_or(false);
    let recovery_peer = (descriptor["publicRecovery"].as_bool().unwrap_or(false) && !last_browser).then(|| {
        let peer = consumer.execute_managed_command("Bob", &serde_json::json!({
            "action":"navigate", "url":"https://synthetic.example/recovery-peer", "profileId":"profile-a",
            "activityAtMs":now()
        })).unwrap().unwrap();
        assert_eq!(peer["browserSession"]["browserId"], opened["browserSession"]["browserId"]);
        peer
    });
    let public_http = descriptor["publicHttp"].as_bool().unwrap_or(false);
    let public_locator = descriptor["publicLocator"].as_bool().unwrap_or(false);
    let public_html = descriptor["publicHtml"].as_bool().unwrap_or(false);
    assert!(!public_html || (public_http && public_locator));
    // Acceptance uses context produced by the actual authorized handoff,
    // never a test-side registration or diagnostic URL rewrite.
    let locator_prepared = public_locator.then(|| {
        let observation = consumer.state().remote_view_tab_handoffs[id]
            .native_observation
            .as_ref()
            .and_then(|binding| binding.observation.as_ref())
            .expect("native handoff must prepare observation before publishing its link");
        serde_json::json!({"success":true,"data":{"correlationId":observation.correlation_id}})
    });

    let mut public = public_http.then(|| {
        NativePublicHarness::start(
            &fixture.root,
            descriptor["ownerUrl"].as_str().unwrap(),
            "Alice",
            id,
            public_locator,
            public_html,
            descriptor["publicOrigin"].as_str(),
        )
    });
    let mut presentation_path = None;
    let registered = if public_html {
        locator_prepared.expect("public browser handoff must already have native context")
    } else if let Some(harness) = public.as_mut() {
        let mut prepared = locator_prepared;
        loop {
            match harness.next() {
                NativePublicEvent::Command(command, reply) => {
                    let result = consumer.handle_command(&command);
                    assert_eq!(result["success"], true, "{result}");
                    if command["handoffOperation"] == "prepare_native_use" {
                        prepared = Some(result.clone());
                    }
                    reply.send(result).unwrap();
                }
                NativePublicEvent::Presented(location) => {
                    let prepared =
                        prepared.expect("HTTP redirect did not register with actual owner");
                    let correlation = prepared["data"]["correlationId"].as_str().unwrap();
                    let route = reqwest::Url::parse(&location)
                        .expect("public presentation location must be absolute");
                    assert_eq!(route.host_str(), Some("desktops.example.invalid"));
                    assert_eq!(
                        route
                            .query_pairs()
                            .find(|(key, _)| key == "handoff-observation")
                            .map(|(_, value)| value.into_owned()),
                        Some(correlation.to_owned())
                    );
                    presentation_path = Some(format!(
                        "{}{}",
                        route.path(),
                        route
                            .query()
                            .map(|query| format!("?{query}"))
                            .unwrap_or_default()
                    ));
                    break prepared;
                }
                NativePublicEvent::Failed(reason) => {
                    panic!("actual public HTTP request failed: {reason}")
                }
            }
        }
    } else {
        consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"prepare_native_use",
        "expectedTabId":tab,"expectedTargetId":opened["browserSession"]["targetId"],"activityAtMs":now()
        }))
    };
    assert_eq!(registered["success"], true, "{registered}");
    let correlation = registered["data"]["correlationId"].as_str().unwrap();
    let pending = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"observe_native_use","activityAtMs":now()
    }));
    assert_eq!(pending["data"]["status"], "native_use_pending", "{pending}");
    assert!(fixture
        .store(false)
        .load_session_state()
        .unwrap()
        .pending_human_tabs
        .contains_key(tab));
    std::fs::write(&status_path,serde_json::to_vec(&serde_json::json!({"stage":"pending","correlationId":correlation,"handoffId":id,"pendingHuman":true,"presentationPath":presentation_path,"publicEndpoint":public.as_ref().map(|harness|harness.origin.as_str()),"handoffPath":format!("/remote-view/{id}")})).unwrap()).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let result = if public_locator {
            consumer
                .reap(now())
                .expect("public handoff owner reaping failed");
            serde_json::json!({"success":true,"data":{"status":if consumer.state().pending_human_tabs.contains_key(tab) {"native_use_pending"} else {"human_use_confirmed"}}})
        } else if let Some(harness) = public.as_mut() {
            let NativePublicEvent::Command(command, reply) = harness.next() else {
                panic!("unexpected HTTP fixture event after presentation");
            };
            assert_eq!(command["handoffOperation"], "observe_native_use");
            let result = consumer.handle_command(&command);
            reply.send(result.clone()).unwrap();
            result
        } else {
            consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"observe_native_use","activityAtMs":now()
            }))
        };
        assert_eq!(result["success"], true, "{result}");
        if result["data"]["status"] == "human_use_confirmed" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "actual viewer evidence never reached browser owner: {result}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let durable = fixture.store(false).load_session_state().unwrap();
    assert!(!durable.pending_human_tabs.contains_key(tab));
    let first = durable.tab_metadata[tab].first_human_use_at_ms.unwrap();
    let observation = durable.remote_view_tab_handoffs[id]
        .native_observation
        .as_ref()
        .unwrap()
        .observation
        .as_ref()
        .unwrap();
    assert_eq!(observation.correlation_id, correlation);
    assert_eq!(observation.used.as_ref().unwrap().recorded_at_ms, first);
    let replay = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve","handoffId":id,"handoffOperation":"observe_native_use","activityAtMs":now()
    }));
    assert_eq!(
        replay["data"]["status"], "human_use_already_confirmed",
        "{replay}"
    );
    assert_eq!(
        fixture
            .store(false)
            .load_session_state()
            .unwrap()
            .tab_metadata[tab]
            .first_human_use_at_ms,
        Some(first)
    );
    assert_eq!(calls.get(), 1);
    assert!(provider.borrow().view_requests.is_empty());
    std::fs::write(&status_path,serde_json::to_vec(&serde_json::json!({"stage":"confirmed","correlationId":correlation,"handoffId":id,"pendingHuman":false,"firstHumanUseAtMs":first,"providerUseAtMs":observation.used.as_ref().unwrap().recorded_at_ms,"replayStatus":replay["data"]["status"]})).unwrap()).unwrap();
    if descriptor["publicRecovery"].as_bool().unwrap_or(false) {
        assert!(public_html);
        let configured = consumer.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":id,
            "handoffOperation":"set_tab_lifetime", "tabInactivityMs":1,"activityAtMs":now()
        }));
        assert_eq!(configured["success"], true, "{configured}");
        std::thread::sleep(std::time::Duration::from_millis(5));
        let expired = consumer.reap(now()).unwrap();
        assert!(expired.expired_tab_ids.contains(&tab.to_owned()));
        let state = fixture.store(false).load_session_state().unwrap();
        assert!(!state.tabs.contains_key(tab));
        if let Some(peer) = &recovery_peer {
            let peer_tab = peer["browserSession"]["tabId"].as_str().unwrap();
            assert!(state.tabs.contains_key(peer_tab));
            assert_eq!(
                state.tabs[peer_tab].target_id,
                peer["browserSession"]["targetId"].as_str().unwrap()
            );
        } else {
            assert!(
                state.tabs.is_empty(),
                "last-browser fixture retained a live tab"
            );
            assert!(
                state.browsers.is_empty(),
                "last-browser fixture retained a browser"
            );
        }
        state.remote_view_tab_handoffs[id]
            .check_link_at(now())
            .unwrap();
        let current_tab = &consumer.state().remote_view_tab_handoffs[id].tab_id;
        std::fs::write(
            &status_path,
            serde_json::to_vec(&serde_json::json!({
                "stage":"expired", "handoffId":id, "linkValid":true,
                "tabPresentAtExpiry":false, "firstHumanUseAtMs":first,

                "peerTabPresent":recovery_peer.is_some(),
                "lastBrowser":last_browser,
                "currentTabPresent":consumer.state().tabs.contains_key(current_tab)
            }))
            .unwrap(),
        )
        .unwrap();
        // Keep the real public HTTP reader and owner alive for the next opaque
        // navigation. No test-side resolve or context preparation repairs it.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(35);
        while std::time::Instant::now() < deadline {
            if status_path.with_extension("done").exists() {
                let recovered = fixture.store(false).load_session_state().unwrap();
                let current_tab = &recovered.remote_view_tab_handoffs[id].tab_id;
                assert!(recovered.tabs.contains_key(current_tab));
                assert_eq!(
                    recovered.tab_metadata[current_tab].first_human_use_at_ms,
                    Some(first)
                );
                if let Some(peer) = &recovery_peer {
                    let peer_tab = peer["browserSession"]["tabId"].as_str().unwrap();
                    assert_eq!(
                        recovered.tabs[peer_tab].target_id,
                        peer["browserSession"]["targetId"].as_str().unwrap()
                    );
                }
                assert_eq!(
                    calls.get(),
                    if last_browser { 2 } else { 1 },
                    "recovery browser launch count changed"
                );
                if !recovered.pending_human_tabs.contains_key(current_tab) {
                    std::fs::write(
                        &status_path,
                        serde_json::to_vec(&serde_json::json!({
                            "stage":"confirmed","correlationId":correlation,"handoffId":id,
                            "pendingHuman":false,"firstHumanUseAtMs":first,
                            "providerUseAtMs":observation.used.as_ref().unwrap().recorded_at_ms,
                            "replayStatus":replay["data"]["status"],
                            "recoveredTargetPresent":true
                        }))
                        .unwrap(),
                    )
                    .unwrap();
                    return;
                }
            }
            if let Ok(NativePublicEvent::Command(command, reply)) = public
                .as_ref()
                .unwrap()
                .events
                .recv_timeout(std::time::Duration::from_millis(100))
            {
                let result = consumer.handle_command(&command);
                reply.send(result).unwrap();
            }
        }
        panic!("retained opaque URL never recovered its expired target");
    }
}

#[cfg(unix)]
enum NativePublicEvent {
    Command(Value, tokio::sync::oneshot::Sender<Value>),
    Presented(String),
    Failed(String),
}

#[cfg(unix)]
struct NativePublicHarness {
    origin: String,
    events: std::sync::mpsc::Receiver<NativePublicEvent>,
    _runtime: tokio::runtime::Runtime,
    _readiness: std::net::TcpListener,
}

#[cfg(unix)]
impl NativePublicHarness {
    fn start(
        root: &Path,
        owner_url: &str,
        session: &str,
        handoff: &str,
        public_locator: bool,
        server_only: bool,
        public_origin: Option<&str>,
    ) -> Self {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let sockets = root.join("public-sockets");
        std::fs::create_dir(&sockets).unwrap();
        std::env::set_var("AGENT_BROWSER_SOCKET_DIR", &sockets);
        std::env::set_var("AGENT_BROWSER_DASHBOARD_AUTH_DIR", root.join("public-auth"));
        std::env::remove_var("AGENT_BROWSER_DASHBOARD_AUTH_FILE");
        std::env::set_var(
            "AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN",
            public_origin.unwrap_or("https://desktops.example.invalid"),
        );
        std::env::set_var(
            "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
            owner_url.strip_suffix("/v1/consumer").unwrap(),
        );
        std::env::set_var("AGENT_BROWSER_REMOTE_VIEW_APPLICATION", "agent-browser");
        std::env::set_var("AGENT_BROWSER_REMOTE_VIEW_POOL", "main");
        std::env::remove_var("AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT");
        if public_locator {
            // Exercise the actual public reader against retained registration
            // states before any renderer can publish use. Each rejected read
            // must leave owner state untouched.
            let mut store = BrowserSessionSqliteStore::default_sqlite().unwrap();
            let original = store.load_session_state().unwrap();
            for partial in [false, true] {
                let mut rejected = original.clone();
                let binding = rejected
                    .remote_view_tab_handoffs
                    .get_mut(handoff)
                    .unwrap()
                    .native_observation
                    .as_mut()
                    .unwrap();
                let expected = if partial {
                    binding.observation = None;
                    "remote_view_native_registration_missing"
                } else {
                    binding.target_digest = "0".repeat(64);
                    "remote_view_human_use_target_changed"
                };
                store
                    .compare_and_save_session_state(&original, &rejected)
                    .unwrap();
                assert_eq!(
                    crate::native::browser_session_remote_view::resolve_handoff_desktop_presentation(handoff)
                        .unwrap_err(),
                    expected
                );
                assert_eq!(store.load_session_state().unwrap(), rejected);
                store
                    .compare_and_save_session_state(&rejected, &original)
                    .unwrap();
            }
            let (_, correlation) =
                crate::native::browser_session_remote_view::resolve_handoff_desktop_presentation(
                    handoff,
                )
                .unwrap();
            assert_eq!(
                correlation.as_deref(),
                original.remote_view_tab_handoffs[handoff]
                    .native_observation
                    .as_ref()
                    .unwrap()
                    .observation
                    .as_ref()
                    .map(|observation| observation.correlation_id.as_str())
            );
            assert_eq!(store.load_session_state().unwrap(), original);
        }
        let credentials = crate::native::stream::native_handoff_http_fixture_credentials();
        let credentials = std::fs::read_to_string(credentials).unwrap();
        let password = credentials
            .lines()
            .find_map(|line| {
                line.strip_prefix("AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD=\"")
                    .and_then(|value| value.strip_suffix('"'))
            })
            .unwrap()
            .to_owned();
        let token = uuid::Uuid::new_v4().to_string();
        crate::connection::write_daemon_auth_token(session, &token).unwrap();
        let readiness = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        std::fs::write(
            sockets.join(format!("{session}.stream")),
            readiness.local_addr().unwrap().port().to_string(),
        )
        .unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let (sender, events) = std::sync::mpsc::channel();
        let handoff = handoff.to_owned();
        let origin = runtime.block_on(async {
            let unix = tokio::net::UnixListener::bind(crate::connection::get_socket_path(session))
                .unwrap();
            let commands = sender.clone();
            tokio::spawn(async move {
                loop {
                    let (socket, _) = unix.accept().await.unwrap();
                    let mut stream = tokio::io::BufReader::new(socket);
                    let mut line = String::new();
                    // Broker readiness checks connect without sending a command.
                    if stream.read_line(&mut line).await.unwrap() == 0 {
                        continue;
                    }
                    let command: Value = serde_json::from_str(&line).unwrap();
                    assert!(
                        command["_agentBrowserAuthToken"].as_str() == Some(token.as_str()),
                        "synthetic daemon authentication failed"
                    );
                    let (reply, result) = tokio::sync::oneshot::channel();
                    if commands
                        .send(NativePublicEvent::Command(command, reply))
                        .is_err()
                    {
                        break;
                    }
                    let result = result.await.unwrap();
                    stream
                        .get_mut()
                        .write_all(format!("{result}\n").as_bytes())
                        .await
                        .unwrap();
                }
            });
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let origin = format!("http://{}", listener.local_addr().unwrap());
            tokio::spawn(crate::native::stream::run_native_handoff_http_fixture(
                listener,
            ));
            if server_only {
                if std::env::var_os("AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE").is_some() {
                    let before = BrowserSessionSqliteStore::default_sqlite()
                        .unwrap()
                        .load_session_state()
                        .unwrap();
                    let token = std::fs::read_to_string(
                        std::env::var_os("AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE").unwrap(),
                    )
                    .unwrap();
                    let client = reqwest::Client::builder()
                        .no_proxy()
                        .redirect(reqwest::redirect::Policy::none())
                        .build()
                        .unwrap();
                    for case in 0..3 {
                        let mut request = client
                            .post(format!("{origin}/api/native-remote-view/{handoff}/resume"))
                            .header("content-length", "0")
                            .bearer_auth(if case == 0 {
                                "incorrect-synthetic-capability-1234567890"
                            } else {
                                token.trim_end()
                            });
                        if case != 2 {
                            request = request.header("x-remote-view-operator", "operator-example");
                        }
                        if case == 1 {
                            request = request.header("cookie", "fixture_session=operator");
                        }
                        assert_eq!(
                            request.send().await.unwrap().status().as_u16(),
                            403,
                            "invalid private callback reached recovery"
                        );
                        assert_eq!(
                            BrowserSessionSqliteStore::default_sqlite()
                                .unwrap()
                                .load_session_state()
                                .unwrap(),
                            before
                        );
                        assert!(
                            events.try_recv().is_err(),
                            "denied callback reached the daemon"
                        );
                    }
                }
                return origin;
            }
            let server_origin = origin.clone();
            tokio::spawn(async move {
                let result = async {
                    let client = reqwest::Client::builder()
                        .no_proxy()
                        .redirect(reqwest::redirect::Policy::none())
                        .timeout(std::time::Duration::from_secs(25))
                        .build()
                        .unwrap();
                    let request = if public_locator {
                        client.get(format!("{origin}/remote-view/{handoff}"))
                    } else {
                        let login = client
                            .post(format!("{origin}/api/dashboard-auth/login"))
                            .json(&serde_json::json!({"username":"admin","password":password}))
                            .send()
                            .await
                            .map_err(|e| e.to_string())?;
                        if login.status().as_u16() != 200 {
                            return Err("synthetic HTTP login failed".to_string());
                        }
                        let cookie = login
                            .headers()
                            .get("set-cookie")
                            .ok_or("synthetic signed cookie missing")?
                            .to_str()
                            .map_err(|e| e.to_string())?
                            .split(';')
                            .next()
                            .unwrap()
                            .to_owned();
                        client
                            .get(format!("{origin}/api/remote-view/{handoff}/presentation"))
                            .header("cookie", cookie)
                    };
                    let presented = request
                        .header("sec-fetch-site", "same-origin")
                        .header("sec-fetch-mode", "navigate")
                        .header("sec-fetch-dest", "document")
                        .send()
                        .await
                        .map_err(|e| e.to_string())?;
                    if presented.status().as_u16() != 303 {
                        return Err(format!("HTTP presentation status {}", presented.status()));
                    }
                    Ok(presented
                        .headers()
                        .get("location")
                        .ok_or("presentation location missing")?
                        .to_str()
                        .map_err(|e| e.to_string())?
                        .to_owned())
                }
                .await;
                let event = match result {
                    Ok(location) => NativePublicEvent::Presented(location),
                    Err(reason) => NativePublicEvent::Failed(reason),
                };
                let _ = sender.send(event);
            });
            server_origin
        });
        Self {
            origin,
            events,
            _runtime: runtime,
            _readiness: readiness,
        }
    }

    fn next(&self) -> NativePublicEvent {
        self.events
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("public HTTP/owner continuation made no progress")
    }
}

#[test]
fn automatic_retirement_publishes_custody_before_native_completion() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("require-retirement-custody"), "check").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let opened = consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/","profileId":"profile-a","activityAtMs":100
    })).unwrap().unwrap();
    let browser_id = opened["browserSession"]["browserId"].as_str().unwrap();
    std::fs::write(
        fixture.root.join("check-retirement-read-exclusion"),
        "check",
    )
    .unwrap();
    assert_eq!(
        consumer.reap(300_100).unwrap().closed_browser_ids,
        vec![browser_id]
    );
    let mut store = fixture.store(false);
    let records = store.idle_retirement_records().unwrap();
    assert_eq!(
        records.len(),
        1,
        "physical retirement must have durable original-operation custody"
    );
    assert!(
        records[0].completed.is_some(),
        "native fence completes only after session publication"
    );
    assert!(records[0].published_state_digest.is_some());
    assert!(store
        .load_session_state()
        .unwrap()
        .idle_closed_browsers
        .contains_key(browser_id));
    let operations = &provider.borrow().operations;
    assert_eq!(
        operations
            .iter()
            .filter(|op| **op == "prepare_idle_retirement")
            .count(),
        1
    );
    assert_eq!(
        operations
            .iter()
            .filter(|op| **op == "complete_idle_retirement")
            .count(),
        1
    );
    assert_eq!(calls.get(), 1, "retirement must not launch a browser");
}

#[test]
fn automatic_retirement_rechecks_current_authority_after_unlocked_provider_read() {
    use super::super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
    let repository = LockedServiceStateRepository::default_json().unwrap();
    let prior = repository.load_snapshot().unwrap();
    struct Restore(agent_browser_service_model::ServiceState);
    impl Drop for Restore {
        fn drop(&mut self) {
            use super::super::service_store::{
                LockedServiceStateRepository, ServiceStateRepository,
            };
            let prior = self.0.clone();
            LockedServiceStateRepository::default_json()
                .unwrap()
                .mutate(|state| {
                    let revision = state.state_revision;
                    *state = prior.clone();
                    state.state_revision = revision;
                    Ok(())
                })
                .unwrap();
        }
    }
    let _restore = Restore(prior);
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(&fixture, provider.clone(), calls, Mode::IdleRecovery);
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    std::fs::write(
        fixture.root.join("check-retirement-read-exclusion"),
        "check",
    )
    .unwrap();
    std::fs::write(
        fixture.root.join("publish-controller-during-idle-read"),
        &opened.browser_id,
    )
    .unwrap();
    assert!(
        consumer
            .reap(300_100)
            .unwrap()
            .closed_browser_ids
            .is_empty(),
        "newly published controller must veto retirement"
    );
    assert!(provider.borrow().operations.contains(&"observe_idle"));
    assert!(
        !provider
            .borrow()
            .operations
            .contains(&"prepare_idle_retirement"),
        "no native mutation may precede current authority admission"
    );
    assert!(fixture
        .store(false)
        .idle_retirement_records()
        .unwrap()
        .is_empty());
}

#[test]
fn automatic_retirement_lost_completion_reconciles_original_operation_after_restart() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/","profileId":"profile-a","activityAtMs":100
    })).unwrap().unwrap();
    provider.borrow_mut().lose_retirement_completion = true;
    assert_eq!(
        consumer.reap(300_100).unwrap_err(),
        "browser_session_idle_readback_required"
    );
    let records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].published_state_digest.is_some());
    assert!(records[0].completed.is_none());
    let id = records[0].intent.id.clone();
    drop(consumer);
    let mut restarted = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    restarted.reap(300_101).unwrap();
    let records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].intent.id, id);
    assert!(
        records[0].completed.is_some(),
        "readback must reconcile the already completed original fence"
    );
    assert_eq!(
        provider
            .borrow()
            .operations
            .iter()
            .filter(|op| **op == "complete_idle_retirement")
            .count(),
        1
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn automatic_retirement_lost_close_reads_original_effect_without_repeating_close() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("require-retirement-custody"), "check").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    consumer.execute_managed_command("Alice", &serde_json::json!({
        "action":"navigate","url":"https://synthetic.example/","profileId":"profile-a","activityAtMs":100
    })).unwrap().unwrap();
    std::fs::write(fixture.root.join("lose-retirement-close-reply"), "lose").unwrap();
    assert_eq!(
        consumer.reap(300_100).unwrap_err(),
        "synthetic_close_reply_lost"
    );
    let records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].prepared.is_some());
    assert!(records[0].published_state_digest.is_none());
    assert!(records[0].completed.is_none());
    let id = records[0].intent.id.clone();
    drop(consumer);
    let mut restarted = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    assert_eq!(restarted.reap(300_101).unwrap().closed_browser_ids.len(), 1);
    let records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].intent.id, id);
    assert!(records[0].completed.is_some());
    assert_eq!(
        provider
            .borrow()
            .operations
            .iter()
            .filter(|op| **op == "prepare_idle_retirement")
            .count(),
        1
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("retirement-close-count")).unwrap(),
        "1"
    );
}

#[test]
fn automatic_retirement_publication_precedes_same_command_logical_identity_reuse() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let first = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let old_pid = consumer.state().browsers[&first.browser_id].pid;
    let second = consumer
        .open(OpenBrowserSession::exact_profile(
            "Alice",
            "profile-a",
            300_101,
        ))
        .unwrap();
    assert_ne!(second.session_id, first.session_id);
    assert_eq!(
        second.browser_id, first.browser_id,
        "the fixture deliberately reuses the logical browser ID"
    );
    assert_ne!(consumer.state().browsers[&second.browser_id].pid, old_pid);
    assert_eq!(calls.get(), 2);
    let records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].completed.is_some());
    assert_eq!(records[0].intent.scopes[0].browser.pid, old_pid);
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        *consumer.state()
    );
}

#[test]
fn automatic_retirement_stale_snapshot_cannot_repeat_an_unknown_close() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("require-retirement-custody"), "check").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let baseline = consumer.state().clone();
    let browser = baseline.browsers[&opened.browser_id].clone();
    drop(consumer);
    claim_fixture_retirement(&fixture, provider.clone(), &baseline, &browser);
    let make_effects = || {
        RemoteViewSessionEffects::new(
            Process {
                mode: Mode::IdleRecovery,
                root: fixture.root.clone(),
                calls: calls.clone(),
            },
            adapter(&fixture, provider.clone()),
            fixture.store(false),
            vec![],
            || uuid::Uuid::new_v4().to_string(),
        )
        .unwrap()
        .with_pool(RemoteViewSessionPool {
            name: "main".into(),
            desired_desktops: 1,
        })
        .unwrap()
    };
    let mut first = make_effects();
    let mut stale = make_effects();
    first.begin_operation(&baseline).unwrap();
    stale.begin_operation(&baseline).unwrap();
    let first_guard = first
        .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
        .unwrap()
        .unwrap();
    let stale_guard = stale
        .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
        .unwrap()
        .unwrap();
    std::fs::write(fixture.root.join("lose-retirement-close-reply"), "lose").unwrap();
    assert_eq!(
        first.close_browser(&browser).unwrap_err(),
        "synthetic_close_reply_lost"
    );
    drop(first_guard);
    stale.close_browser(&browser).unwrap();
    drop(stale_guard);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("retirement-close-count")).unwrap(),
        "1",
        "a matching CAS replay is not fresh physical-effect admission"
    );
    assert_eq!(
        fixture
            .store(false)
            .idle_retirement_records()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn automatic_retirement_stale_preparation_snapshot_reads_back_without_mutation_replay() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let baseline = consumer.state().clone();
    let browser = baseline.browsers[&opened.browser_id].clone();
    drop(consumer);
    let intent = claim_fixture_retirement(&fixture, provider.clone(), &baseline, &browser);
    let id = intent.id;
    let make_effects = || {
        RemoteViewSessionEffects::new(
            Process {
                mode: Mode::IdleRecovery,
                root: fixture.root.clone(),
                calls: calls.clone(),
            },
            adapter(&fixture, provider.clone()),
            fixture.store(false),
            vec![],
            || uuid::Uuid::new_v4().to_string(),
        )
        .unwrap()
        .with_pool(RemoteViewSessionPool {
            name: "main".into(),
            desired_desktops: 1,
        })
        .unwrap()
    };
    let mut first = make_effects();
    let mut stale = make_effects();
    first.begin_operation(&baseline).unwrap();
    stale.begin_operation(&baseline).unwrap();
    provider.borrow_mut().lose_retirement_preparation = true;
    assert!(
        matches!(first.acquire_idle_retirement_guard(std::slice::from_ref(&browser)),
        Err(error) if error == "browser_session_idle_readback_required")
    );
    let guard = stale
        .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
        .unwrap()
        .unwrap();
    drop(guard);
    assert_eq!(
        provider
            .borrow()
            .operations
            .iter()
            .filter(|op| **op == "prepare_idle_retirement")
            .count(),
        1,
        "a preparation CAS replay must require original-operation readback"
    );
    let records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].intent.id, id);
    assert!(records[0].prepared.is_some());
}

#[test]
fn automatic_retirement_prepared_without_effects_reconciles_after_restart() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("require-retirement-custody"), "check").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let baseline = consumer.state().clone();
    let browser = baseline.browsers[&opened.browser_id].clone();
    drop(consumer);
    let mut interrupted = RemoteViewSessionEffects::new(
        Process {
            mode: Mode::Success,
            root: fixture.root.clone(),
            calls: calls.clone(),
        },
        adapter(&fixture, provider.clone()),
        fixture.store(false),
        vec![],
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap()
    .with_pool(RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    })
    .unwrap();
    interrupted.begin_operation(&baseline).unwrap();
    let guard = interrupted
        .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
        .unwrap()
        .unwrap();
    assert!(fixture.root.join("native-retirement-fence").exists());
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    assert!(restarted.reap(101).unwrap().closed_browser_ids.is_empty());
    let records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(records.len(), 1);
    assert!(
        records[0].completed.is_some(),
        "no admitted effect remains to justify retaining native exclusion"
    );
    assert!(!fixture.root.join("native-retirement-fence").exists());
    assert_eq!(
        interrupted.close_browser(&browser).unwrap_err(),
        "browser_session_idle_readback_required",
        "a delayed actor cannot admit its old effect after no-effect publication"
    );
    drop(guard);
    drop(interrupted);
    assert!(!fixture.root.join("retirement-close-count").exists());
    assert_eq!(restarted.state().browsers, baseline.browsers);
    assert_eq!(restarted.state().tabs, baseline.tabs);
    assert!(restarted
        .execute_managed_command(
            "Alice",
            &serde_json::json!({"action":"get_url","profileId":"profile-a","activityAtMs":102})
        )
        .unwrap()
        .is_some());
    assert_eq!(calls.get(), 1);
}

#[test]
fn automatic_retirement_absent_fence_after_unknown_prepare_remains_unresolved() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let baseline = consumer.state().clone();
    let browser = baseline.browsers[&opened.browser_id].clone();
    drop(consumer);
    let mut interrupted = RemoteViewSessionEffects::new(
        Process {
            mode: Mode::Success,
            root: fixture.root.clone(),
            calls: calls.clone(),
        },
        adapter(&fixture, provider.clone()),
        fixture.store(false),
        vec![],
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap()
    .with_pool(RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    })
    .unwrap();
    interrupted.begin_operation(&baseline).unwrap();
    provider.borrow_mut().lose_retirement_preparation = true;
    assert!(
        matches!(interrupted.acquire_idle_retirement_guard(std::slice::from_ref(&browser)),
        Err(error) if error == "browser_session_idle_readback_required")
    );
    drop(interrupted);
    // This observation is compatible with a prepare that never arrived or is
    // still delayed. It is deliberately not a cancellation acknowledgement.
    provider.borrow_mut().retirement_receipts.clear();
    let original = fixture.store(false).idle_retirement_records().unwrap();
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    assert!(restarted.reap(101).unwrap().closed_browser_ids.is_empty());
    assert_eq!(
        fixture.store(false).idle_retirement_records().unwrap(),
        original
    );
    assert!(original[0].prepared.is_none());
    assert!(original[0].published_state_digest.is_none());
    assert!(original[0].completed.is_none());
    assert_eq!(
        provider
            .borrow()
            .operations
            .iter()
            .filter(|op| **op == "prepare_idle_retirement")
            .count(),
        1
    );
    assert_eq!(
        provider
            .borrow()
            .operations
            .iter()
            .filter(|op| **op == "complete_idle_retirement")
            .count(),
        0
    );
    assert_eq!(restarted.state().browsers, baseline.browsers);
    assert_eq!(restarted.state().tabs, baseline.tabs);
}

#[test]
fn automatic_retirement_pending_effect_denies_conflicting_browser_commands() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    consumer
        .navigate(&opened.session_id, "https://synthetic.example/", 101)
        .unwrap();
    let peer = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 102))
        .unwrap();
    consumer
        .navigate(&peer.session_id, "https://synthetic.example/peer", 103)
        .unwrap();
    let baseline = consumer.state().clone();
    let browser = baseline.browsers[&opened.browser_id].clone();
    let tab = baseline
        .tabs
        .values()
        .find(|tab| tab.browser_id == browser.id)
        .unwrap()
        .clone();
    drop(consumer);
    let intent = claim_fixture_retirement(&fixture, provider.clone(), &baseline, &browser);
    fixture
        .store(false)
        .admit_idle_retirement_preparation(&intent)
        .unwrap();
    let prepared = adapter(&fixture, provider.clone())
        .prepare_idle_retirement(&intent.provider)
        .unwrap();
    let mut record = fixture
        .store(false)
        .idle_retirement_records()
        .unwrap()
        .remove(0);
    let previous = record.clone();
    record.record_prepared(prepared).unwrap();
    fixture
        .store(false)
        .update_idle_retirement(&previous, &record)
        .unwrap();
    fixture
        .store(false)
        .admit_idle_retirement_effect(
            &intent,
            BrowserIdleRetirementEffect::CloseBrowser {
                browser_id: browser.id.clone(),
            },
        )
        .unwrap();
    let pending = fixture.store(false).idle_retirement_records().unwrap();
    let mut effects = RemoteViewSessionEffects::new(
        Process {
            mode: Mode::Success,
            root: fixture.root.clone(),
            calls: calls.clone(),
        },
        adapter(&fixture, provider),
        fixture.store(false),
        vec![],
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    effects.begin_operation(&baseline).unwrap();
    assert_eq!(
        effects
            .navigate(&browser, &tab, "https://synthetic.example/")
            .unwrap_err(),
        "browser_session_idle_readback_required"
    );
    assert_eq!(
        effects.create_tab(&browser).unwrap_err(),
        "browser_session_idle_readback_required"
    );
    assert_eq!(
        effects.focus_browser(&browser, Some(&tab)).unwrap_err(),
        "browser_session_idle_readback_required"
    );
    assert_eq!(
        effects
            .execute_command(
                &browser,
                &tab,
                &opened.session_id,
                "Alice",
                &serde_json::json!({"action":"navigate","url":"https://synthetic.example/"})
            )
            .unwrap_err(),
        "browser_session_idle_readback_required"
    );
    assert!(effects.browser_is_live(&browser).unwrap());
    let peer_browser = &baseline.browsers[&peer.browser_id];
    let peer_tab = baseline
        .tabs
        .values()
        .find(|tab| tab.browser_id == peer_browser.id)
        .unwrap();
    effects
        .navigate(
            peer_browser,
            peer_tab,
            "https://synthetic.example/continued",
        )
        .unwrap();

    assert_eq!(
        fixture.store(false).idle_retirement_records().unwrap(),
        pending
    );
}

#[test]
fn automatic_retirement_freezes_the_published_launch_kernel_identity() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(&fixture, provider, calls, Mode::IdleRecovery);
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let original = fixture
        .store(false)
        .published_launch_process_identity(&consumer.state().browsers[&opened.browser_id])
        .unwrap()
        .unwrap();
    assert_eq!(
        consumer.reap(300_100).unwrap().closed_browser_ids,
        vec![opened.browser_id]
    );
    let record = fixture
        .store(false)
        .idle_retirement_records()
        .unwrap()
        .remove(0);
    assert_eq!(record.intent.scopes[0].process_identity, Some(original));
    assert!(record.completed.is_some());
}

#[test]
fn automatic_retirement_reused_pid_never_reaches_the_ordinary_close_effect() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("require-retirement-custody"), "synthetic").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(&fixture, provider, calls, Mode::IdleRecovery);
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    std::fs::write(
        fixture.root.join("replacement-kernel-observed"),
        "original ended",
    )
    .unwrap();
    assert_eq!(
        consumer.reap(300_100).unwrap().closed_browser_ids,
        vec![opened.browser_id]
    );
    assert!(
        !fixture.root.join("retirement-close-count").exists(),
        "a reused PID must not reach the ordinary physical close delegate"
    );
    assert!(fixture.store(false).idle_retirement_records().unwrap()[0]
        .completed
        .is_some());
}

#[test]
fn automatic_retirement_legacy_identity_defers_before_native_preparation() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("legacy-launch-no-kernel"), "profile-a").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let alice = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let baseline = consumer.state().clone();
    assert!(fixture
        .store(false)
        .published_launch_process_identity(&baseline.browsers[&alice.browser_id])
        .unwrap()
        .is_none());
    drop(consumer);
    let mut restarted = host(&fixture, provider.clone(), calls.clone());
    assert!(restarted
        .reap(300_100)
        .unwrap()
        .closed_browser_ids
        .is_empty());
    assert_eq!(restarted.state().browsers, baseline.browsers);
    assert_eq!(restarted.state().tabs, baseline.tabs);
    assert!(fixture
        .store(false)
        .idle_retirement_records()
        .unwrap()
        .is_empty());
    assert!(!fixture.root.join("native-retirement-fence").exists());
    assert!(!provider
        .borrow()
        .operations
        .contains(&"prepare_idle_retirement"));
    let bob = restarted
        .open(OpenBrowserSession::exact_profile(
            "Bob",
            "profile-b",
            300_101,
        ))
        .unwrap();
    assert_ne!(alice.browser_id, bob.browser_id);
    assert_eq!(
        restarted.state().browsers[&alice.browser_id],
        baseline.browsers[&alice.browser_id]
    );
    assert!(fixture
        .store(false)
        .published_launch_process_identity(&restarted.state().browsers[&bob.browser_id])
        .unwrap()
        .is_some());
    assert_eq!(calls.get(), 2);
}

#[test]
fn automatic_retirement_legacy_group_preflight_precedes_every_native_fence() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("legacy-launch-no-kernel"), "profile-b").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let alice = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let occupied = consumer.state().browsers[&alice.browser_id]
        .desktop
        .as_ref()
        .unwrap()
        .desktop_id
        .clone();
    provider.borrow_mut().occupied_desktops.insert(occupied);
    let bob = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 101))
        .unwrap();
    provider.borrow_mut().occupied_desktops.clear();
    let baseline = consumer.state().clone();
    let first = fixture
        .store(false)
        .published_launch_assignment(&baseline.browsers[&alice.browser_id])
        .unwrap();
    let second = fixture
        .store(false)
        .published_launch_assignment(&baseline.browsers[&bob.browser_id])
        .unwrap();
    assert!(
        first.assignment_id < second.assignment_id,
        "fixture must encounter the known group before the legacy group"
    );
    drop(consumer);
    let mut effects = RemoteViewSessionEffects::new(
        Process {
            mode: Mode::Success,
            root: fixture.root.clone(),
            calls,
        },
        adapter(&fixture, provider.clone()),
        fixture.store(false),
        vec![],
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap()
    .with_pool(RemoteViewSessionPool {
        name: "main".into(),
        desired_desktops: 1,
    })
    .unwrap();
    effects.begin_operation(&baseline).unwrap();
    let browsers = vec![
        baseline.browsers[&alice.browser_id].clone(),
        baseline.browsers[&bob.browser_id].clone(),
    ];
    let reason = match effects.acquire_idle_retirement_guard(&browsers) {
        Err(reason) => reason,
        Ok(_) => panic!("legacy identity deferral must retain an inspectable reason"),
    };
    assert_eq!(reason, "browser_session_idle_original_identity_missing");
    assert!(
        fixture
            .store(false)
            .idle_retirement_records()
            .unwrap()
            .is_empty(),
        "known local deferral must precede every claim and native preparation"
    );
    assert!(!provider
        .borrow()
        .operations
        .contains(&"prepare_idle_retirement"));
    assert_eq!(fixture.store(false).load_session_state().unwrap(), baseline);
}

#[test]
fn admitted_navigation_prevents_competing_retirement_before_its_effect() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls);
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    consumer
        .navigate(&opened.session_id, "https://synthetic.example/before", 101)
        .unwrap();
    let baseline = consumer.state().clone();
    let intent = fixture_retirement_intent(
        &fixture,
        provider,
        &baseline,
        &baseline.browsers[&opened.browser_id],
    );
    intent.matches_baseline(&baseline).unwrap();
    assert_eq!(fixture.store(false).load_session_state().unwrap(), baseline);
    std::fs::write(
        fixture.root.join("command-retirement-race.json"),
        serde_json::to_vec(&serde_json::json!({"intent":intent,"expected":baseline})).unwrap(),
    )
    .unwrap();
    let navigation = consumer.navigate(
        &opened.session_id,
        "https://synthetic.example/admitted",
        102,
    );
    assert!(
        fixture
            .store(false)
            .idle_retirement_records()
            .unwrap()
            .is_empty(),
        "a retirement acquired custody while the already-admitted navigation was executing"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("racing-navigation-effect")).unwrap(),
        "once"
    );
    navigation.unwrap();
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        *consumer.state()
    );
}

#[test]
fn unknown_command_activity_recovers_only_after_original_process_readback() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    assert!(consumer
        .state()
        .remote_view_tab_handoffs
        .values()
        .any(|link| link.session_id == opened.session_id));
    std::fs::write(
        fixture.root.join("command-activity-unknown-profile"),
        "profile-a",
    )
    .unwrap();
    assert_eq!(
        consumer
            .navigate(&opened.session_id, "https://synthetic.example/unknown", 101)
            .unwrap_err(),
        "synthetic_navigation_outcome_unknown"
    );
    drop(consumer);
    std::fs::write(
        fixture.root.join("activity-original-readback"),
        "observe only",
    )
    .unwrap();

    let mut resumed = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    resumed
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 102))
        .unwrap();
    assert_eq!(calls.get(), 2, "unrelated profile must remain usable");
    let baseline = resumed.state().clone();
    let intent = fixture_retirement_intent(
        &fixture,
        provider,
        &baseline,
        &baseline.browsers[&opened.browser_id],
    );
    assert_eq!(
        fixture
            .store(false)
            .claim_idle_retirement(&intent, &baseline),
        Err(BrowserIdleRetirementStoreError::ActivityActive)
    );
    std::fs::write(fixture.root.join("idle-process-absent"), "absent").unwrap();
    std::fs::write(
        fixture.root.join("activity-readback-unavailable"),
        "unknown",
    )
    .unwrap();
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":103});
    assert!(resumed.execute_managed_command("Alice", &read).is_err());
    assert_eq!(
        calls.get(),
        2,
        "unresolved original activity must deny replacement before launch"
    );
    std::fs::remove_file(fixture.root.join("activity-readback-unavailable")).unwrap();
    let recovered = resumed
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    assert_eq!(recovered["success"], true, "{recovered}");
    assert_eq!(recovered["browserSession"]["sessionId"], opened.session_id);
    assert_eq!(calls.get(), 3);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("unknown-navigation-count")).unwrap(),
        "1",
        "unknown navigation must not replay during recovery"
    );
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        *resumed.state()
    );
    let records = fixture.store(false).browser_activity_records().unwrap();
    assert_eq!(
        records.len(),
        1,
        "successful transient activities must be removed"
    );
    assert!(records[0].original_ended);
    assert!(
        !records[0].acknowledged,
        "resource recovery must not settle unknown command outcome"
    );
    assert_eq!(records[0].browser.id, opened.browser_id);
    assert!(records[0].process_identity.is_some());
}

#[test]
fn acknowledged_activity_exit_clears_local_ids_and_preserves_history() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mode = Rc::new(Cell::new(0));
    let attempts = Rc::new(Cell::new(0));
    let mut consumer = host_with_persistence(
        &fixture,
        provider,
        calls.clone(),
        Mode::IdleRecovery,
        || 2001,
        NativeUseReplyFailure {
            store: fixture.store(false),
            mode: mode.clone(),
            attempts: attempts.clone(),
        },
    );

    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let session_id = opened["browserSession"]["sessionId"].as_str().unwrap();
    mode.set(3);
    assert_eq!(
        consumer
            .navigate(session_id, "https://synthetic.example/acknowledged", 101)
            .unwrap_err(),
        "synthetic_activity_publication_unavailable"
    );
    assert_eq!(attempts.get(), 1);
    let record = fixture
        .store(false)
        .browser_activity_records()
        .unwrap()
        .remove(0);
    assert!(record.acknowledged);
    assert!(!record.original_ended);
    std::fs::write(fixture.root.join("idle-process-absent"), "absent").unwrap();
    let mut continued = read.clone();
    continued["activityAtMs"] = serde_json::json!(102);
    let resumed = consumer
        .execute_managed_command("Alice", &continued)
        .expect("original exit must retire stale process-local publication IDs")
        .unwrap();
    assert_eq!(resumed["success"], true, "{resumed}");
    assert_eq!(calls.get(), 2);
    let records = fixture.store(false).browser_activity_records().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, record.id);
    assert!(records[0].acknowledged && records[0].original_ended);
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        *consumer.state()
    );
}

#[test]
fn sqlite_activity_publication_storage_failure_preserves_its_typed_cause_after_dispatch() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider, calls.clone());
    consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    std::fs::write(
        fixture
            .root
            .join("fail-activity-publication-after-dispatch"),
        "armed",
    )
    .unwrap();
    let error = consumer.execute_managed_command("Alice", &serde_json::json!({
        "id":"original-dispatched-action", "action":"navigate",
        "url":"https://synthetic.example/activity", "runtimeProfile":"profile-a", "activityAtMs":101
    })).unwrap_err();
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("command-dispatch-count")).unwrap_or_default(),
        "1",
        "fixture must reach exactly one original dispatch before publication fails: {error}"
    );
    let baseline: BrowserSessionState = serde_json::from_slice(
        &std::fs::read(fixture.root.join("activity-publication-baseline.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        baseline,
        "failed SQLite transaction cannot publish staged state"
    );
    let activities = fixture.store(false).browser_activity_records().unwrap();
    assert_eq!(activities.len(), 1);
    assert!(
        activities[0].acknowledged && !activities[0].original_ended,
        "original dispatch receipt remains protected"
    );
    assert_eq!(
        error, "browser_session_idle_publication_unavailable",
        "storage failure must not be mislabeled as a publication conflict"
    );
}

#[test]
fn changed_activity_publication_lost_reply_allows_next_command_without_replay() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mode = Rc::new(Cell::new(0));
    let attempts = Rc::new(Cell::new(0));
    let mut consumer = host_with_persistence(
        &fixture,
        provider,
        calls.clone(),
        Mode::Success,
        || 2001,
        NativeUseReplyFailure {
            store: fixture.store(false),
            mode: mode.clone(),
            attempts: attempts.clone(),
        },
    );
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    std::fs::write(fixture.root.join("count-navigation-dispatches"), "armed").unwrap();
    mode.set(4);
    let navigate = serde_json::json!({"action":"navigate", "runtimeProfile":"profile-a", "activityAtMs":101, "url":"https://synthetic.example/once"});
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &navigate)
            .unwrap_err(),
        "synthetic_activity_publication_reply_lost"
    );
    assert_eq!(attempts.get(), 1);
    let published = fixture.store(false).load_session_state().unwrap();
    assert_eq!(published.navigation_history.len(), 1);
    assert!(fixture
        .store(false)
        .browser_activity_records()
        .unwrap()
        .is_empty());
    let mut peer = published.clone();
    peer.next_session_sequence += 10;
    fixture
        .store(false)
        .compare_and_save_session_state(&published, &peer)
        .unwrap();
    consumer
        .execute_managed_command("Alice", &read)
        .expect("completed SQLite publication must not poison an independent next command")
        .unwrap();
    assert_eq!(
        fixture
            .store(false)
            .load_session_state()
            .unwrap()
            .navigation_history
            .len(),
        1,
        "original navigation is never replayed"
    );
    assert_eq!(
        consumer.state().next_session_sequence,
        peer.next_session_sequence,
        "independent peer metadata survives reconciliation"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("navigation-dispatch-count")).unwrap(),
        "1",
        "the original browser effect executes exactly once"
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn activity_publication_receipts_preserve_unknown_and_current_identity_fences() {
    let fixture = Fixture::new();
    let mut consumer = host(
        &fixture,
        Rc::new(RefCell::new(Provider::default())),
        Rc::new(Cell::new(0)),
    );
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let mut store = fixture.store(false);
    let state = store.load_session_state().unwrap();
    let json: String = store
        .remote_view_mutation_connection()
        .query_row(
            "SELECT json FROM state_documents WHERE kind='browser_activity_publications'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let receipts: Vec<BrowserActivityRecord> = serde_json::from_str(&json).unwrap();
    let id = receipts.last().unwrap().id.clone();
    // Exact durable completion is idempotent, even after its active row is gone.
    BrowserSessionPersistence::publish_session_effects(
        &mut store,
        &state,
        &state,
        None,
        &[],
        std::slice::from_ref(&id),
    )
    .unwrap();
    let unknown = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        BrowserSessionPersistence::publish_session_effects(
            &mut store,
            &state,
            &state,
            None,
            &[],
            std::slice::from_ref(&unknown)
        )
        .unwrap_err(),
        "browser_session_idle_publication_conflict"
    );
    let browser = state.browsers.values().next().unwrap().clone();
    let mut changed_root = state.clone();
    changed_root.browsers.get_mut(&browser.id).unwrap().pid += 1;
    assert_eq!(
        BrowserSessionPersistence::publish_session_effects(
            &mut store,
            &state,
            &changed_root,
            None,
            &[],
            std::slice::from_ref(&id)
        )
        .unwrap_err(),
        "browser_session_idle_publication_conflict"
    );
    store
        .admit_browser_activity(&unknown, &browser, &state)
        .unwrap();
    assert_eq!(
        BrowserSessionPersistence::publish_session_effects(
            &mut store,
            &state,
            &state,
            None,
            &[],
            std::slice::from_ref(&unknown)
        )
        .unwrap_err(),
        "browser_session_idle_publication_conflict"
    );
    assert!(
        !store
            .browser_activity_records()
            .unwrap()
            .iter()
            .find(|record| record.id == unknown)
            .unwrap()
            .acknowledged
    );
    let mut replacement = state.clone();
    replacement.browsers.get_mut(&browser.id).unwrap().pid += 1;
    assert_eq!(
        BrowserSessionPersistence::publish_session_effects(
            &mut store,
            &state,
            &replacement,
            None,
            &[],
            std::slice::from_ref(&id)
        )
        .unwrap_err(),
        "browser_session_idle_publication_activity_active"
    );
    assert_eq!(store.load_session_state().unwrap(), state);
}

#[test]
fn activity_publication_receipt_window_and_atomic_failure_keep_exact_evidence() {
    let fixture = Fixture::new();
    let mut consumer = host(
        &fixture,
        Rc::new(RefCell::new(Provider::default())),
        Rc::new(Cell::new(0)),
    );
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let mut store = fixture.store(false);
    let state = store.load_session_state().unwrap();
    let json: String = store
        .remote_view_mutation_connection()
        .query_row(
            "SELECT json FROM state_documents WHERE kind='browser_activity_publications'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut receipts: Vec<BrowserActivityRecord> = serde_json::from_str(&json).unwrap();
    let oldest = receipts[0].id.clone();
    while receipts.len() < 4096 {
        let mut receipt = receipts[0].clone();
        receipt.id = uuid::Uuid::new_v4().to_string();
        receipts.push(receipt);
    }
    let seeded = serde_json::to_string(&receipts).unwrap();
    store
        .remote_view_mutation_connection()
        .execute(
            "UPDATE state_documents SET json=?1 WHERE kind='browser_activity_publications'",
            [&seeded],
        )
        .unwrap();
    let next = uuid::Uuid::new_v4().to_string();
    store
        .admit_browser_activity(&next, state.browsers.values().next().unwrap(), &state)
        .unwrap();
    store.acknowledge_browser_activity(&next).unwrap();
    store
        .remote_view_mutation_connection()
        .execute_batch(
            "CREATE TRIGGER fail_activity_receipt BEFORE INSERT ON state_documents
         WHEN NEW.kind='browser_activity_publications'
         BEGIN SELECT RAISE(ABORT, 'fixture receipt failure'); END;",
        )
        .unwrap();
    let mut staged = state.clone();
    staged.next_session_sequence += 1;
    assert_eq!(
        BrowserSessionPersistence::publish_session_effects(
            &mut store,
            &state,
            &staged,
            None,
            &[],
            std::slice::from_ref(&next)
        )
        .unwrap_err(),
        "browser_session_idle_publication_unavailable"
    );
    assert_eq!(
        store.load_session_state().unwrap(),
        state,
        "receipt failure rolls back the preceding state write"
    );
    assert!(store
        .browser_activity_records()
        .unwrap()
        .iter()
        .any(|record| record.id == next && record.acknowledged));
    store
        .remote_view_mutation_connection()
        .execute_batch("DROP TRIGGER fail_activity_receipt")
        .unwrap();
    BrowserSessionPersistence::publish_session_effects(
        &mut store,
        &state,
        &staged,
        None,
        &[],
        std::slice::from_ref(&next),
    )
    .unwrap();
    let json: String = store
        .remote_view_mutation_connection()
        .query_row(
            "SELECT json FROM state_documents WHERE kind='browser_activity_publications'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let receipts: Vec<BrowserActivityRecord> = serde_json::from_str(&json).unwrap();
    assert_eq!(receipts.len(), 4096);
    assert_eq!(receipts.last().unwrap().id, next);
    assert!(!receipts.iter().any(|record| record.id == oldest));
    assert_eq!(
        BrowserSessionPersistence::publish_session_effects(
            &mut store,
            &staged,
            &staged,
            None,
            &[],
            &[oldest]
        )
        .unwrap_err(),
        "browser_session_idle_publication_conflict",
        "expired receipt cannot be inferred from an absent activity row"
    );
    BrowserSessionPersistence::publish_session_effects(
        &mut store,
        &staged,
        &staged,
        None,
        &[],
        &[next],
    )
    .unwrap();
}

#[test]
fn unchanged_activity_publication_lost_reply_does_not_block_next_command() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mode = Rc::new(Cell::new(0));
    let attempts = Rc::new(Cell::new(0));
    let mut consumer = host_with_persistence(
        &fixture,
        provider,
        calls.clone(),
        Mode::Success,
        || 2001,
        NativeUseReplyFailure {
            store: fixture.store(false),
            mode: mode.clone(),
            attempts: attempts.clone(),
        },
    );
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let before = consumer.state().clone();
    mode.set(4);
    assert_eq!(
        consumer
            .execute_managed_command("Alice", &read)
            .unwrap_err(),
        "synthetic_activity_publication_reply_lost"
    );
    assert_eq!(attempts.get(), 1);
    assert_eq!(fixture.store(false).load_session_state().unwrap(), before);
    assert_eq!(*consumer.state(), before);
    assert!(fixture
        .store(false)
        .browser_activity_records()
        .unwrap()
        .is_empty());
    let continued = consumer
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/after-unchanged-publication",
                "runtimeProfile":"profile-a", "activityAtMs":101,
            }),
        )
        .expect("already-published activity ID must not obstruct the next command")
        .unwrap();
    assert_eq!(continued["success"], true, "{continued}");
    assert_eq!(calls.get(), 1);
    assert!(fixture
        .store(false)
        .browser_activity_records()
        .unwrap()
        .is_empty());
    assert_eq!(
        fixture.store(false).load_session_state().unwrap(),
        *consumer.state()
    );
}

#[test]
fn ordinary_continuation_publishes_observed_live_launch_without_replacement() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    drop(consumer);
    let mut store = fixture.store(false);
    let baseline = store.load_session_state().unwrap();
    let original = baseline.browsers[browser_id].clone();
    let intent = BrowserLaunchIntent {
        intent_id: uuid::Uuid::new_v4().to_string(),
        profile_id: original.profile_id.clone(),
        assignment: provider.borrow().assignments[0].clone(),
    };
    store
        .admit_recovery_launch_intent(&intent, &baseline, &original)
        .unwrap();
    store
        .observe_launch_intent(
            &intent,
            &BrowserLaunch {
                browser_id: original.id.clone(),
                pid: 91,
                cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/synthetic".into(),
                desktop: original.desktop.clone(),
                process_identity: None,
            },
        )
        .unwrap();
    drop(store);
    // The persisted PID is gone; direct profile and native window observations
    // see the already-running replacement. No process launch is necessary.
    std::fs::write(fixture.root.join("idle-process-absent"), "old PID absent").unwrap();
    provider.borrow_mut().window_pid = Some(91);
    let mut resumed = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::LiveProfileReuse,
    );
    for now in [101, 102] {
        let result = resumed
            .execute_managed_command(
                "Alice",
                &serde_json::json!({
                    "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":now
                }),
            )
            .unwrap()
            .unwrap();
        assert_eq!(result["success"], true, "{result}");
        assert_eq!(result["browserSession"]["sessionId"], identity["sessionId"]);
        assert_eq!(result["browserSession"]["tabId"], identity["tabId"]);
        assert_eq!(resumed.state().browsers[browser_id].pid, 91);
    }
    assert_eq!(
        calls.get(),
        1,
        "continuation must not launch a second browser"
    );
    assert!(fixture
        .store(false)
        .unpublished_launch_records()
        .unwrap()
        .is_empty());
}

#[test]
fn ordinary_dead_observed_launch_recovers_after_current_profile_absence() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    drop(consumer);
    let mut store = fixture.store(false);
    let baseline = store.load_session_state().unwrap();
    let original = baseline.browsers[browser_id].clone();
    let intent = BrowserLaunchIntent {
        intent_id: uuid::Uuid::new_v4().to_string(),
        profile_id: original.profile_id.clone(),
        assignment: provider.borrow().assignments[0].clone(),
    };
    store
        .admit_recovery_launch_intent(&intent, &baseline, &original)
        .unwrap();
    store
        .observe_launch_intent(
            &intent,
            &BrowserLaunch {
                browser_id: original.id.clone(),
                pid: 91,
                cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/synthetic".into(),
                desktop: original.desktop.clone(),
                process_identity: None,
            },
        )
        .unwrap();
    drop(store);
    std::fs::write(
        fixture.root.join("idle-process-absent"),
        "whole profile absent",
    )
    .unwrap();
    let mut store = fixture.store(false);
    crate::native::browser_session_remote_view::reconcile_pending_recoveries(
        &mut Process {
            mode: Mode::IdleRecovery,
            root: fixture.root.clone(),
            calls: calls.clone(),
        },
        &mut store,
    )
    .unwrap();
    assert!(store.unpublished_launch_records().unwrap().is_empty());
    let mut resumed = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let result = resumed
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":101
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(result["success"], true, "{result}");
    assert_eq!(result["browserSession"]["sessionId"], identity["sessionId"]);
    assert_eq!(
        calls.get(),
        2,
        "exactly one replacement after proven absence"
    );
}

#[test]
fn ordinary_long_lived_host_rechecks_dead_launch_before_recovery() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    drop(consumer);
    let mut store = fixture.store(false);
    let baseline = store.load_session_state().unwrap();
    let original = baseline.browsers[browser_id].clone();
    let intent = BrowserLaunchIntent {
        intent_id: uuid::Uuid::new_v4().to_string(),
        profile_id: original.profile_id.clone(),
        assignment: provider.borrow().assignments[0].clone(),
    };
    store
        .admit_recovery_launch_intent(&intent, &baseline, &original)
        .unwrap();
    store
        .observe_launch_intent(
            &intent,
            &BrowserLaunch {
                browser_id: original.id.clone(),
                pid: 91,
                cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/synthetic".into(),
                desktop: original.desktop.clone(),
                process_identity: None,
            },
        )
        .unwrap();
    drop(store);
    // The host was already running when the old launch was still unresolved.
    let mut resumed = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    std::fs::write(
        fixture.root.join("idle-process-absent"),
        "whole profile absent",
    )
    .unwrap();
    let result = resumed
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":101
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(result["success"], true, "{result}");
    assert_eq!(result["browserSession"]["sessionId"], identity["sessionId"]);
    assert_eq!(
        calls.get(),
        2,
        "exactly one replacement after proven absence"
    );
}

#[test]
fn ordinary_live_profile_ignores_mismatched_historical_launch_observation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let read =
        serde_json::json!({"action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100});
    let opened = consumer
        .execute_managed_command("Alice", &read)
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    drop(consumer);
    let mut store = fixture.store(false);
    let baseline = store.load_session_state().unwrap();
    let original = baseline.browsers[browser_id].clone();
    let intent = BrowserLaunchIntent {
        intent_id: uuid::Uuid::new_v4().to_string(),
        profile_id: original.profile_id.clone(),
        assignment: provider.borrow().assignments[0].clone(),
    };
    store
        .admit_recovery_launch_intent(&intent, &baseline, &original)
        .unwrap();
    store
        .observe_launch_intent(
            &intent,
            &BrowserLaunch {
                browser_id: original.id.clone(),
                pid: 99,
                cdp_endpoint: "ws://127.0.0.1:9222/devtools/browser/synthetic".into(),
                desktop: original.desktop.clone(),
                process_identity: None,
            },
        )
        .unwrap();
    drop(store);
    // The persisted PID is gone; direct profile and native window observations
    // see the already-running replacement. No process launch is necessary.
    std::fs::write(fixture.root.join("idle-process-absent"), "old PID absent").unwrap();
    provider.borrow_mut().window_pid = Some(91);
    let mut resumed = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::LiveProfileReuse,
    );
    for now in [101, 102] {
        let result = resumed
            .execute_managed_command(
                "Alice",
                &serde_json::json!({
                    "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":now
                }),
            )
            .unwrap()
            .unwrap();
        assert_eq!(result["success"], true, "{result}");
        assert_eq!(result["browserSession"]["sessionId"], identity["sessionId"]);
        assert_eq!(result["browserSession"]["tabId"], identity["tabId"]);
        assert_eq!(resumed.state().browsers[browser_id].pid, 91);
    }
    assert_eq!(
        calls.get(),
        1,
        "continuation must not launch a second browser"
    );
    let retained = fixture.store(false).unpublished_launch_records().unwrap();
    assert_eq!(retained.len(), 1, "mismatched history remains diagnostic");
    assert_eq!(retained[0].observed_pid, Some(99));
    // Historical publication for the previous PID is not a prerequisite for
    // later recovery after this current browser exits.
    drop(resumed);
    provider.borrow_mut().window_pid = None;
    let mut store = fixture.store(false);
    crate::native::browser_session_remote_view::reconcile_pending_recoveries(
        &mut Process {
            mode: Mode::IdleRecovery,
            root: fixture.root.clone(),
            calls: calls.clone(),
        },
        &mut store,
    )
    .unwrap();
    assert!(store.unpublished_launch_records().unwrap().is_empty());
    let mut recovered = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    let result = recovered
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":103
            }),
        )
        .unwrap()
        .unwrap();
    assert_eq!(result["success"], true, "{result}");
    assert_eq!(result["browserSession"]["sessionId"], identity["sessionId"]);
    assert_eq!(
        calls.get(),
        2,
        "recovery launches once after current absence"
    );
}

#[test]
fn manual_native_context_adopts_current_kernel_identity_and_rejects_replaced_pid() {
    let root =
        std::env::temp_dir().join(format!("manual-native-adoption-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join(".agent-browser/service")).unwrap();
    let environment_guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_CONFIG",
    ]);
    environment_guard.set("HOME", root.to_str().unwrap());
    environment_guard.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    environment_guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    environment_guard.remove("AGENT_BROWSER_CONFIG");
    let parent_env = [std::env::var_os("DISPLAY"), std::env::var_os("XAUTHORITY")];
    let fixture = Fixture::with_root(root.join(".agent-browser/service"), true);
    std::fs::write(fixture.root.join("real-native-context-adoption"), "enabled").unwrap();
    let crate::process_identity::ProcessObservation::Observed(current) =
        crate::process_identity::observe_process(std::process::id())
    else {
        panic!("fixture process must be observable");
    };
    let identity = RecordedProcessIdentity {
        pid: current.pid,
        start_token: current.start_token.unwrap(),
        executable_path: current.executable_path,
        browser_family: None,
    };
    std::fs::write(
        fixture.root.join("live-cdp-endpoint.json"),
        serde_json::json!({"pid":identity.pid, "processIdentity":identity}).to_string(),
    )
    .unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    provider.borrow_mut().window_pid = Some(std::process::id());
    let launches = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), launches.clone());
    let opened = consumer.handle_command(&serde_json::json!({"action":"browser_session_open", "sessionName":"Native adoption", "profileId":"profile-a", "url":"about:blank", "manualLogin":true, "activityAtMs":100, "serviceName":"fixture", "agentName":"fixture", "taskName":"Native adoption"}));
    assert_eq!(opened["success"], true, "{opened}");
    let browser = consumer.state().browsers.values().next().unwrap().clone();
    drop(consumer);
    let mut adopted = host(&fixture, provider.clone(), launches.clone());
    adopted.focus_browser(&browser.id, None, 101).unwrap();
    assert_eq!(
        launches.get(),
        1,
        "adoption binds current provider inputs without launching or navigating"
    );
    assert_eq!(
        parent_env,
        [std::env::var_os("DISPLAY"), std::env::var_os("XAUTHORITY")]
    );
    let assignment = fixture
        .store(false)
        .published_launch_assignment(&browser)
        .unwrap();
    let mut owner = adapter(&fixture, provider);
    let private = owner.launch_environment(&assignment).unwrap();
    let observation = owner.observe_assignment(&assignment).unwrap();
    let mut wrong = browser.clone();
    wrong.pid = u32::MAX;
    let mut runtime =
        super::super::browser_session_runtime::BrowserManagerRuntime::start(Default::default())
            .unwrap();
    assert_eq!(
        runtime.refresh_native_environment(&wrong, &assignment, private, &observation),
        Err(RemoteViewBrowserProcessError::Rejected),
        "binding cannot adopt another process identity"
    );
    assert_eq!(
        parent_env,
        [std::env::var_os("DISPLAY"), std::env::var_os("XAUTHORITY")]
    );
    drop(adopted);
    drop(runtime);
    drop(fixture);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn manual_native_context_refreshes_after_host_restart_and_fences_generation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let launches = Rc::new(Cell::new(0));
    let command = serde_json::json!({"action":"browser_session_open", "sessionName":"Native auth", "profileId":"profile-a", "url":"about:blank", "manualLogin":true, "activityAtMs":100, "serviceName":"fixture", "agentName":"fixture", "taskName":"Native auth"});
    let mut original = host(&fixture, provider.clone(), launches.clone());
    let opened = original.handle_command(&command);
    assert_eq!(opened["success"], true, "{opened}");
    let browser = original.state().browsers.values().next().unwrap().clone();
    provider.borrow_mut().window_pid = Some(browser.pid);
    let session = opened["data"]["sessionId"].as_str().unwrap().to_owned();
    let reads = || {
        provider
            .borrow()
            .operations
            .iter()
            .filter(|operation| **operation == "launch_environment")
            .count()
    };
    original.focus_browser(&browser.id, None, 101).unwrap();
    assert_eq!(
        reads(),
        2,
        "manual focus needs fresh owner context before local focus"
    );
    drop(original);
    let mut adopted = host(&fixture, provider.clone(), launches.clone());
    let before = reads();
    adopted.focus_browser(&browser.id, None, 102).unwrap();
    assert_eq!(
        reads(),
        before + 1,
        "adoption refreshes from the exact current owner"
    );
    assert_eq!(launches.get(), 1, "focus must not relaunch or navigate");
    let effects =
        || std::fs::read_to_string(fixture.root.join("manual-native-focus-count")).unwrap();
    let before_refusal = effects();
    provider.borrow_mut().native_viewing_generation = Some(2);
    assert!(
        adopted.focus_browser(&browser.id, None, 103).is_err(),
        "stale owner context must be refused"
    );
    assert_eq!(
        effects(),
        before_refusal,
        "generation refusal precedes the local effect"
    );
    let mut strict_effects = RemoteViewSessionEffects::new(
        Process {
            mode: Mode::Success,
            root: fixture.root.clone(),
            calls: launches.clone(),
        },
        adapter(&fixture, provider.clone()),
        fixture.store(false),
        Vec::new(),
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    strict_effects.begin_operation(adopted.state()).unwrap();
    assert!(strict_effects
        .dispatch_graceful_browser_close(&browser)
        .is_err());
    assert!(
        !fixture.root.join("manual-strict-close-count").exists(),
        "strict close generation refusal precedes physical effect"
    );
    provider.borrow_mut().native_viewing_generation = None;
    let assignment_id = provider.borrow().assignments[0].assignment_id.clone();
    provider.borrow_mut().unavailable_assignment = Some(assignment_id);
    assert!(adopted.focus_browser(&browser.id, None, 104).is_err());
    assert_eq!(
        effects(),
        before_refusal,
        "unavailable owner refuses before the local effect"
    );
    assert!(strict_effects
        .dispatch_graceful_browser_close(&browser)
        .is_err());
    assert!(
        !fixture.root.join("manual-strict-close-count").exists(),
        "unavailable owner cannot close"
    );
    provider.borrow_mut().unavailable_assignment = None;
    let before_strict = reads();
    strict_effects
        .dispatch_graceful_browser_close(&browser)
        .unwrap();
    assert_eq!(
        reads(),
        before_strict + 1,
        "strict close refreshes current adopted owner context"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("manual-strict-close-count")).unwrap(),
        "1"
    );
    let before = reads();
    adopted
        .close_session(&session, SessionEndReason::ExplicitClose, 105)
        .unwrap();
    assert!(
        reads() > before,
        "clean close also needs fresh exact owner context"
    );
    assert_eq!(launches.get(), 1);
}

#[test]
fn manual_login_preserves_posture_through_managed_owner_replay_and_conflict() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut host = host(&fixture, provider.clone(), calls.clone());
    let command = serde_json::json!({"action":"browser_session_open", "sessionName":"Alice",
        "profileId":"profile-a", "url":"https://accounts.google.com/", "manualLogin":true,
        "activityAtMs":100, "serviceName":"fixture", "agentName":"fixture", "taskName":"Manual login"});
    let mut invalid = command.clone();
    invalid["url"] = "--disable-web-security".into();
    let refused = host.handle_command(&invalid);
    assert_eq!(refused["success"], false, "{refused}");
    assert!(refused.to_string().contains("manual_login_url_invalid"));
    assert_eq!(calls.get(), 0, "invalid initial URLs cannot launch Chrome");
    let mut attached = command.clone();
    attached["attachment"] = serde_json::json!({});
    let refused = host.handle_command(&attached);
    assert_eq!(refused["success"], false, "{refused}");
    assert!(refused
        .to_string()
        .contains("manual_login_requires_owned_browser"));
    assert_eq!(calls.get(), 0, "manual login cannot attach a CDP browser");
    let first = host.handle_command(&command);
    assert_eq!(first["success"], true, "{first}");
    let launch: Value = serde_json::from_slice(
        &std::fs::read(fixture.root.join("manual-launch-profile.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(launch["manualLoginUrl"], command["url"]);
    let browser = host.state().browsers.values().next().unwrap();
    assert!(browser.cdp_endpoint.is_empty());
    let id = first["data"]["sessionId"].as_str().unwrap().to_owned();
    assert_eq!(host.handle_command(&command)["data"]["sessionId"], id);
    assert_eq!(calls.get(), 1, "same login task must not launch again");
    let before_refusal = host.state().clone();
    let refused = host
        .execute_managed_command(
            "Alice",
            &serde_json::json!({"action":"snapshot", "profileId":"profile-a", "activityAtMs":100}),
        )
        .unwrap_err();
    assert!(refused.starts_with("manual_login_automation_unavailable"));
    assert_eq!(host.state(), &before_refusal);
    for patch in [
        serde_json::json!({"manualLogin":false}),
        serde_json::json!({"sessionName":"Bob"}),
    ] {
        let mut incompatible = command.clone();
        incompatible
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let refused = host.handle_command(&incompatible);
        assert_eq!(refused["success"], false, "{refused}");
        assert_eq!(calls.get(), 1);
    }
    host.close_session(&id, SessionEndReason::ExplicitClose, 101)
        .unwrap();
    let mut automation = command.clone();
    automation["manualLogin"] = false.into();
    let reopened = host.handle_command(&automation);
    assert_eq!(reopened["success"], true, "{reopened}");
    assert!(!host
        .state()
        .browsers
        .values()
        .next()
        .unwrap()
        .cdp_endpoint
        .is_empty());
    assert_eq!(calls.get(), 2);
}

#[test]
fn activity_readback_observes_each_exact_scope_once_per_operation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let alice = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let bob = consumer
        .open(OpenBrowserSession::exact_profile("Bob", "profile-b", 101))
        .unwrap();
    let baseline = consumer.state().clone();
    drop(consumer);
    for browser_id in [&alice.browser_id, &bob.browser_id] {
        for _ in 0..4 {
            fixture
                .store(false)
                .admit_browser_activity(
                    &uuid::Uuid::new_v4().to_string(),
                    &baseline.browsers[browser_id],
                    &baseline,
                )
                .unwrap();
        }
    }
    // A retained original generation with the same PID/browser is a distinct
    // observation scope. This fixture edit never changes the published launch.
    let connection = rusqlite::Connection::open(fixture.root.join("runtime.sqlite3")).unwrap();
    let source: String = connection
        .query_row(
            "SELECT json FROM state_documents WHERE kind='browser_idle_retirement_custody'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut ledger: Value = serde_json::from_str(&source).unwrap();
    let first = ledger["activities"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap();
    first["processIdentity"]["startToken"] = "synthetic-other-original".into();
    let second = ledger["activities"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .nth(1)
        .unwrap();
    second.as_object_mut().unwrap().remove("processIdentity");
    connection
        .execute(
            "UPDATE state_documents SET json=?1 WHERE kind='browser_idle_retirement_custody'",
            [ledger.to_string()],
        )
        .unwrap();
    drop(connection);
    let count_path = fixture.root.join("activity-observation-count");
    std::fs::write(&count_path, "0").unwrap();
    let count = || {
        std::fs::read_to_string(&count_path)
            .unwrap()
            .parse::<u32>()
            .unwrap()
    };
    let mut effects = RemoteViewSessionEffects::new(
        Process {
            mode: Mode::Success,
            root: fixture.root.clone(),
            calls,
        },
        adapter(&fixture, provider),
        fixture.store(false),
        vec![],
        || uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    effects.begin_operation(&baseline).unwrap();
    assert_eq!(
        count(),
        4,
        "eight rows name four exact scopes, including legacy missing identity"
    );
    assert!(fixture
        .store(false)
        .browser_activity_records()
        .unwrap()
        .iter()
        .all(|row| !row.original_ended));
    std::fs::write(
        fixture.root.join("activity-count-readback-unavailable"),
        "unknown",
    )
    .unwrap();
    effects.begin_operation(&baseline).unwrap();
    assert_eq!(
        count(),
        8,
        "uncertain observation is shared only within this operation"
    );
    assert!(fixture
        .store(false)
        .browser_activity_records()
        .unwrap()
        .iter()
        .all(|row| !row.original_ended));
    std::fs::remove_file(fixture.root.join("activity-count-readback-unavailable")).unwrap();
    std::fs::write(fixture.root.join("idle-process-absent"), "absent").unwrap();
    let connection = rusqlite::Connection::open(fixture.root.join("runtime.sqlite3")).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_activity_settlement BEFORE UPDATE OF json ON state_documents WHEN NEW.kind='browser_idle_retirement_custody' BEGIN SELECT RAISE(ABORT, 'synthetic settlement unavailable'); END;").unwrap();
    assert_eq!(
        effects.begin_operation(&baseline).unwrap_err(),
        "browser_session_activity_readback_unavailable"
    );
    assert_eq!(
        count(),
        9,
        "failed settlement stops without inventing completion"
    );
    assert!(fixture
        .store(false)
        .browser_activity_records()
        .unwrap()
        .iter()
        .all(|row| !row.original_ended));
    connection
        .execute_batch("DROP TRIGGER fail_activity_settlement;")
        .unwrap();
    drop(connection);
    effects.begin_operation(&baseline).unwrap();
    assert_eq!(
        count(),
        13,
        "a later operation refreshes all original scopes"
    );
    let rows = fixture.store(false).browser_activity_records().unwrap();
    assert_eq!(
        rows.len(),
        8,
        "every durable activity history remains retained"
    );
    assert!(rows
        .iter()
        .all(|row| row.original_ended && !row.acknowledged));
    effects.begin_operation(&baseline).unwrap();
    assert_eq!(
        count(),
        13,
        "positively ended history needs no further observation"
    );
}

#[test]
fn completed_peer_retirement_does_not_poison_an_ordinary_host_command() {
    peer_retirement_publication_cases(false);
}

#[test]
fn unknown_peer_retirement_completion_does_not_veto_an_independent_task() {
    peer_retirement_publication_cases(true);
}

fn peer_retirement_publication_cases(completion_unknown: bool) {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("count-retirement-closes"), "check").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let failure = Rc::new(Cell::new(0));
    let mut consumer = host_with_persistence(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
        || 2001,
        NativeUseReplyFailure {
            store: fixture.store(false),
            mode: failure.clone(),
            attempts: Rc::new(Cell::new(0)),
        },
    );
    let original = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    failure.set(5);
    assert_eq!(
        consumer.reap(300_101).unwrap_err(),
        "synthetic_retirement_publication_unavailable"
    );
    let pending = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].has_settled_browser_close(&original.browser_id));
    assert!(pending[0].published_state_digest.is_none());

    let mut peer = host_mode(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::IdleRecovery,
    );
    if completion_unknown {
        provider.borrow_mut().lose_retirement_completion = true;
        assert_eq!(
            peer.reap(300_102).unwrap_err(),
            "browser_session_idle_readback_required"
        );
        let unknown = fixture.store(false).idle_retirement_records().unwrap();
        assert!(unknown[0].published_state_digest.is_some());
        assert!(unknown[0].completed.is_none());
        let continued = consumer.handle_command(&serde_json::json!({
            "action":"browser_session_open", "sessionName":"Bob", "profileId":"profile-b",
            "activityAtMs":300_103
        }));
        assert_eq!(
            continued["success"], true,
            "independent task must continue: {continued}"
        );
        assert_eq!(
            calls.get(),
            2,
            "only the independent requested browser may launch"
        );
        let retained = fixture.store(false).idle_retirement_records().unwrap();
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].intent, unknown[0].intent);
        assert_eq!(
            std::fs::read_to_string(fixture.root.join("retirement-effect-count")).unwrap(),
            "1",
            "reconciliation must not repeat the settled original close"
        );
        assert!(
            retained[0].completed.is_some(),
            "original owner readback must complete the original journal"
        );
        return;
    }
    peer.reap(300_104).unwrap();
    let completed = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].intent, pending[0].intent);
    assert!(completed[0].completed.is_some());
    let response = consumer.handle_command(&serde_json::json!({
        "action":"browser_session_open", "sessionName":"Bob", "profileId":"profile-b",
        "activityAtMs":300_103
    }));
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(
        calls.get(),
        2,
        "only the independent requested browser may launch"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("retirement-effect-count")).unwrap(),
        "1",
        "the settled original close must never replay"
    );
    assert_eq!(
        fixture.store(false).idle_retirement_records().unwrap(),
        completed
    );
}

#[test]
fn failed_retirement_publication_does_not_poison_an_unrelated_task() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("count-retirement-closes"), "check").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mode = Rc::new(Cell::new(0));
    let attempts = Rc::new(Cell::new(0));
    let mut consumer = host_with_persistence(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::Success,
        || 2001,
        NativeUseReplyFailure {
            store: fixture.store(false),
            mode: mode.clone(),
            attempts: attempts.clone(),
        },
    );
    let first = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let before = fixture.store(false).load_session_state().unwrap();
    mode.set(5);
    assert_eq!(
        consumer.reap(300_101).unwrap_err(),
        "synthetic_retirement_publication_unavailable"
    );
    assert_eq!(attempts.get(), 1);
    let records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].has_settled_browser_close(&first.browser_id));
    assert!(records[0].published_state_digest.is_none());
    assert_eq!(
        *consumer.state(),
        before,
        "failed cleanup cannot leak unpublished state into another task"
    );
    provider.borrow_mut().window_pid = Some(44);
    let peer = consumer
        .open(OpenBrowserSession::exact_profile(
            "Bob",
            "profile-b",
            300_102,
        ))
        .unwrap();
    assert_ne!(peer.browser_id, first.browser_id);
    assert!(fixture
        .store(false)
        .load_session_state()
        .unwrap()
        .sessions
        .contains_key(&peer.session_id));
    assert_eq!(
        fixture.store(false).idle_retirement_records().unwrap(),
        records,
        "unrelated publication must preserve the original settled close journal"
    );
    let opened = consumer
        .execute_managed_command(
            "Bob",
            &serde_json::json!({
                "action":"navigate", "url":"https://synthetic.example/independent-expiry",
                "profileId":"profile-b", "activityAtMs":300_102
            }),
        )
        .unwrap()
        .unwrap();
    let lifetime = consumer.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":opened["browserSession"]["handoffId"],
        "handoffOperation":"set_tab_lifetime", "tabInactivityMs":30, "activityAtMs":300_102
    }));
    assert_eq!(lifetime["success"], true, "{lifetime}");
    let reaped = consumer.reap(300_132).unwrap();
    assert!(reaped.expired_session_ids.contains(&peer.session_id));
    let durable = fixture.store(false).load_session_state().unwrap();
    assert!(!durable.sessions.contains_key(&first.session_id));
    assert!(!durable.sessions.contains_key(&peer.session_id));
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("retirement-effect-count")).unwrap(),
        "2",
        "each browser closes once; recovery must not replay the settled original close"
    );
    let completed = fixture.store(false).idle_retirement_records().unwrap();
    assert!(completed
        .iter()
        .find(|r| r.intent.id == records[0].intent.id)
        .unwrap()
        .completed
        .is_some());
}

#[test]
fn native_readback_failure_preserves_its_journal_and_publishes_borrowed_expiry() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mode = Rc::new(Cell::new(0));
    let mut native = host_with_persistence(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::Success,
        || 2001,
        NativeUseReplyFailure {
            store: fixture.store(false),
            mode: mode.clone(),
            attempts: Rc::new(Cell::new(0)),
        },
    );
    let old = native
        .open(OpenBrowserSession::exact_profile(
            "Old native",
            "profile-a",
            100,
        ))
        .unwrap();
    mode.set(5);
    assert_eq!(
        native.reap(300_101).unwrap_err(),
        "synthetic_retirement_publication_unavailable"
    );
    let old_records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(old_records.len(), 1);
    assert!(old_records[0].has_settled_browser_close(&old.browser_id));
    assert!(old_records[0].published_state_digest.is_none());
    drop(native);
    let mut borrowed = host_mode(&fixture, provider.clone(), calls, Mode::LiveProfileReuse);
    let task = borrowed
        .open(OpenBrowserSession::exact_profile(
            "Borrowed",
            "profile-b",
            300_102,
        ))
        .unwrap();
    let own = borrowed
        .tab_for_navigation(&task.session_id, 300_102)
        .unwrap();
    let protected = borrowed
        .open(OpenBrowserSession::exact_profile(
            "Protected",
            "profile-b",
            300_102,
        ))
        .unwrap();
    let peer = borrowed
        .tab_for_navigation(&protected.session_id, 300_102)
        .unwrap();
    assert!(borrowed
        .state()
        .shared_browser_ids
        .contains(&task.browser_id));
    assert!(borrowed.state().browsers[&task.browser_id]
        .desktop
        .is_none());
    drop(borrowed);
    let mut store = fixture.store(false);
    let before = store.load_session_state().unwrap();
    let mut configured = before.clone();
    for (id, ttl) in [(&own.tab_id, 30), (&peer.tab_id, 600_000)] {
        configured.tab_lifetimes.insert(
            id.clone(),
            BrowserTabLifetime {
                inactivity_ms: Some(ttl),
                pressure_inactivity_ms: None,
            },
        );
    }
    store
        .compare_and_save_session_state(&before, &configured)
        .unwrap();
    provider.borrow_mut().unavailable_idle_retirement_assignment = Some(
        old_records[0]
            .intent
            .provider
            .assignment
            .assignment_id
            .clone(),
    );
    let mut borrowed = host_mode(
        &fixture,
        provider.clone(),
        Rc::new(Cell::new(0)),
        Mode::LiveProfileReuse,
    );
    let result = borrowed
        .reap(300_132)
        .expect("later Native readback cannot discard independent borrowed retirement");
    assert_eq!(
        result
            .deferred_browser_errors
            .get(&old.browser_id)
            .map(String::as_str),
        Some("browser_session_idle_readback_required")
    );
    let durable = fixture.store(false).load_session_state().unwrap();
    assert!(!durable.sessions.contains_key(&task.session_id));
    assert!(!durable.tabs.contains_key(&own.tab_id));
    assert!(durable.tab_history.iter().any(|tab| tab.id == own.tab_id));
    assert_eq!(
        durable.sessions[&protected.session_id],
        configured.sessions[&protected.session_id]
    );
    assert_eq!(durable.tabs[&peer.tab_id], configured.tabs[&peer.tab_id]);
    assert_eq!(
        durable.browsers[&old.browser_id],
        configured.browsers[&old.browser_id]
    );
    assert_eq!(
        durable.sessions[&old.session_id],
        configured.sessions[&old.session_id]
    );
    assert_eq!(
        fixture.store(false).idle_retirement_records().unwrap(),
        old_records,
        "failed Native readback preserves the exact settled, unpublished original journal"
    );
    assert_eq!(
        provider
            .borrow()
            .operations
            .iter()
            .filter(|op| **op == "complete_idle_retirement")
            .count(),
        0
    );
}

#[test]
fn late_maintenance_failures_preserve_committed_borrowed_expiry_and_failed_scope() {
    for failure in ["native_close", "profile_cleanup", "borrowed_close"] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut native = host(&fixture, provider.clone(), calls.clone());
        let first = native
            .open(OpenBrowserSession::exact_profile(
                "Native A",
                "profile-a",
                100,
            ))
            .unwrap();
        let first_tab = native.tab_for_navigation(&first.session_id, 100).unwrap();
        let second = native
            .open(OpenBrowserSession::exact_profile(
                "Native B",
                "profile-a",
                100,
            ))
            .unwrap();
        let second_tab = native.tab_for_navigation(&second.session_id, 100).unwrap();
        let native_desktop = native.state().browsers[&first.browser_id]
            .desktop
            .as_ref()
            .unwrap()
            .desktop_id
            .clone();
        drop(native);
        let mut borrowed = host_mode(&fixture, provider.clone(), calls, Mode::LiveProfileReuse);
        let a = borrowed
            .open(OpenBrowserSession::exact_profile(
                "Borrowed A",
                "profile-b",
                100,
            ))
            .unwrap();
        let a_tab = borrowed.tab_for_navigation(&a.session_id, 100).unwrap();
        let b = borrowed
            .open(OpenBrowserSession::exact_profile(
                "Borrowed B",
                "profile-b",
                100,
            ))
            .unwrap();
        let b_tab = borrowed.tab_for_navigation(&b.session_id, 100).unwrap();
        let protected = borrowed
            .open(OpenBrowserSession::exact_profile(
                "Protected",
                "profile-b",
                100,
            ))
            .unwrap();
        let peer = borrowed
            .tab_for_navigation(&protected.session_id, 100)
            .unwrap();
        drop(borrowed);
        let mut store = fixture.store(false);
        let old = store.load_session_state().unwrap();
        let mut configured = old.clone();
        for id in [
            &first_tab.tab_id,
            &second_tab.tab_id,
            &a_tab.tab_id,
            &b_tab.tab_id,
        ] {
            configured.tab_lifetimes.insert(
                id.clone(),
                BrowserTabLifetime {
                    inactivity_ms: Some(30),
                    pressure_inactivity_ms: None,
                },
            );
        }
        configured.tab_lifetimes.insert(
            peer.tab_id.clone(),
            BrowserTabLifetime {
                inactivity_ms: Some(600_000),
                pressure_inactivity_ms: None,
            },
        );
        if failure == "profile_cleanup" {
            configured.disposable_profiles.insert(
                "late-profile".into(),
                ManagedDisposableProfile {
                    profile: BrowserProfileCatalogEntry {
                        id: "late-profile".into(),
                        name: "Late profile".into(),
                        user_data_dir: fixture.root.join("late-profile").to_string_lossy().into(),
                        kind: BrowserProfileKind::Disposable,
                        manual_login_url: None,
                    },
                    policy_id: "default".into(),
                    session_name: "old".into(),
                    user_data_root: fixture.root.to_string_lossy().into(),
                    created_at_ms: 1,
                    cleanup_delay_ms: 0,
                    cleanup_eligible_at_ms: Some(2),
                },
            );
            std::fs::write(fixture.root.join("fail-profile-delete"), "fail").unwrap();
        } else if failure == "native_close" {
            std::fs::write(fixture.root.join("count-retirement-closes"), "count").unwrap();
            std::fs::write(fixture.root.join("fail-retirement-close"), "fail").unwrap();
        } else {
            provider
                .borrow_mut()
                .viewer_active_desktops
                .insert(native_desktop);
            std::fs::write(
                fixture.root.join("fail-task-close-target"),
                &b_tab.target_id,
            )
            .unwrap();
        }
        store
            .compare_and_save_session_state(&old, &configured)
            .unwrap();
        let mut owner = host_mode(
            &fixture,
            provider.clone(),
            Rc::new(Cell::new(0)),
            Mode::LiveProfileReuse,
        );
        let result = owner.reap(130);
        if failure == "borrowed_close" {
            assert_eq!(
                result
                    .as_ref()
                    .unwrap()
                    .deferred_browser_errors
                    .get(&a.browser_id)
                    .map(String::as_str),
                Some("synthetic_task_close_outcome_unknown")
            );
        } else {
            let expected = if failure == "native_close" {
                "synthetic_close_outcome_unknown"
            } else {
                "synthetic_profile_delete_failed"
            };
            assert_eq!(result.unwrap_err(), expected);
        }
        let durable = fixture.store(false).load_session_state().unwrap();
        assert!(
            !durable.sessions.contains_key(&a.session_id),
            "{failure}: completed independent expiry must be durable"
        );
        assert!(durable.tab_history.iter().any(|tab| tab.id == a_tab.tab_id));
        assert_eq!(
            durable.browsers[&first.browser_id],
            configured.browsers[&first.browser_id]
        );
        assert_eq!(
            durable.sessions[&first.session_id],
            configured.sessions[&first.session_id]
        );
        assert_eq!(
            durable.sessions[&second.session_id],
            configured.sessions[&second.session_id]
        );
        assert_eq!(
            durable.tabs[&first_tab.tab_id],
            configured.tabs[&first_tab.tab_id]
        );
        assert_eq!(
            durable.tabs[&second_tab.tab_id],
            configured.tabs[&second_tab.tab_id]
        );
        assert_eq!(
            durable.sessions[&protected.session_id],
            configured.sessions[&protected.session_id]
        );
        assert_eq!(durable.tabs[&peer.tab_id], configured.tabs[&peer.tab_id]);
        let records = fixture.store(false).idle_retirement_records().unwrap();
        assert!(records
            .iter()
            .all(|r| r.published_state_digest.is_none() && r.completed.is_none()));
        if failure == "borrowed_close" {
            assert_eq!(
                durable.sessions[&b.session_id],
                configured.sessions[&b.session_id]
            );
            assert_eq!(durable.tabs[&b_tab.tab_id], configured.tabs[&b_tab.tab_id]);
            assert_eq!(
                std::fs::read_to_string(fixture.root.join("failed-task-close-count")).unwrap(),
                "1",
                "a deferred unknown effect cannot be retried in the second phase"
            );
            assert!(records.is_empty());
        } else {
            assert!(!durable.sessions.contains_key(&b.session_id));
            assert_eq!(records.len(), 1);
            let wire = serde_json::to_value(&records[0]).unwrap();
            assert!(wire["effects"]
                .as_array()
                .unwrap()
                .iter()
                .any(|effect| effect["settled"] == true));
            if failure == "native_close" {
                assert!(wire["effects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|effect| effect["settled"] == false));
                assert_eq!(
                    std::fs::read_to_string(fixture.root.join("retirement-effect-count")).unwrap(),
                    "1"
                );
            } else {
                assert_eq!(
                    durable.disposable_profiles["late-profile"],
                    configured.disposable_profiles["late-profile"]
                );
            }
        }
    }
}

#[test]
fn current_retirement_excludes_historical_peers_through_sqlite_custody() {
    for retired_without_identity in [true, false] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host(&fixture, provider.clone(), calls.clone());
        let opened = consumer
            .open(OpenBrowserSession::exact_profile(
                "current",
                "profile-a",
                100,
            ))
            .unwrap();
        let mut state = consumer.state().clone();
        let browser = state.browsers[&opened.browser_id].clone();
        drop(consumer);
        let mut historical = browser.clone();
        historical.id = "historical-browser".into();
        historical.profile_id = "profile-b".into();
        historical.pid += 100;
        historical.active_session_ids.clear();
        state
            .browsers
            .insert(historical.id.clone(), historical.clone());
        if retired_without_identity {
            state.idle_closed_browsers.insert(historical.id.clone(), 1);
        } else {
            // Synthetic process boundary qualifies the earlier original as ended.
            std::fs::write(fixture.root.join("idle-process-absent"), "ended").unwrap();
        }
        let mut store = fixture.store(false);
        store.save_session_state(&state).unwrap();
        let connection = store.remote_view_mutation_connection();
        let raw: String = connection
            .query_row(
                "SELECT json FROM state_documents WHERE kind='browser_launch_custody'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut ledger: Value = serde_json::from_str(&raw).unwrap();
        let mut old = ledger["records"]
            .as_object()
            .unwrap()
            .values()
            .find(|r| r["observedBrowserId"] == browser.id)
            .unwrap()
            .clone();
        let id = uuid::Uuid::new_v4().to_string();
        old["intent"]["intentId"] = id.clone().into();
        old["intent"]["profileId"] = historical.profile_id.clone().into();
        old["observedBrowserId"] = historical.id.clone().into();
        old["observedPid"] = historical.pid.into();
        if retired_without_identity {
            old.as_object_mut()
                .unwrap()
                .remove("observedProcessIdentity");
        } else {
            old["observedProcessIdentity"]["pid"] = historical.pid.into();
            old["intent"]["assignment"]["assignmentId"] = "historical-allocation".into();
        }
        ledger["records"][&id] = old;
        connection
            .execute(
                "UPDATE state_documents SET json=?1 WHERE kind='browser_launch_custody'",
                [ledger.to_string()],
            )
            .unwrap();
        drop(store);
        let mut effects = RemoteViewSessionEffects::new(
            Process {
                mode: Mode::Success,
                root: fixture.root.clone(),
                calls,
            },
            adapter(&fixture, provider),
            fixture.store(false),
            Vec::new(),
            || uuid::Uuid::new_v4().to_string(),
        )
        .unwrap()
        .with_pool(RemoteViewSessionPool {
            name: "main".into(),
            desired_desktops: 1,
        })
        .unwrap();
        effects.begin_operation(&state).unwrap();
        assert!(effects
            .acquire_idle_retirement_guard(std::slice::from_ref(&browser))
            .unwrap()
            .is_some());
        let records = fixture.store(false).idle_retirement_records().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].intent.scopes.len(), 1);
        assert_eq!(records[0].intent.scopes[0].browser, browser);
        assert_eq!(
            fixture.store(false).load_session_state().unwrap(),
            state,
            "admission does not erase history or close targets"
        );
    }
}

#[test]
fn recovery522_exact_owner_release_restores_retained_tasks_without_fabricating_cleanup() {
    for case in [
        "exact",
        "missing",
        "generation",
        "desktop",
        "registration",
        "pool",
        "pending",
        "denied",
        "absence",
    ] {
        let fixture = Fixture::new();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mut consumer = host_mode(
            &fixture,
            provider.clone(),
            calls.clone(),
            Mode::IdleRecovery,
        );
        let alice = consumer
            .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
            .unwrap();
        let bob = consumer
            .open(OpenBrowserSession::exact_profile("Bob", "profile-a", 101))
            .unwrap();
        let mut before = consumer.state().clone();
        let original = before.browsers[&alice.browser_id].clone();
        let prior = fixture
            .store(false)
            .published_launch_assignment(&original)
            .unwrap();
        assert!(!fixture
            .store(false)
            .assignment_release_confirmed(&prior)
            .unwrap());
        drop(consumer);
        before
            .idle_closed_browsers
            .insert(alice.browser_id.clone(), 101);
        before
            .leave_open_session_ids
            .insert(alice.session_id.clone());
        fixture.store(false).save_session_state(&before).unwrap();
        {
            let mut owner = provider.borrow_mut();
            owner.assignments[0].state = RemoteViewAssignmentState::Released;
            match case {
                "missing" => {
                    owner.assignments.clear();
                    owner.unavailable_assignment = Some(prior.assignment_id.clone());
                }
                "generation" => {
                    owner.assignments[0].generation += 1;
                    owner.unavailable_assignment = Some(prior.assignment_id.clone());
                }
                "desktop" => owner.assignments[0].desktop_id = uuid::Uuid::new_v4().to_string(),
                "registration" => owner.assignments[0].registration_id.push_str("-other"),
                "pool" => owner.assignments[0].pool_id.push_str("-other"),
                "pending" => owner.assignments[0].state = RemoteViewAssignmentState::ReleasePending,
                "denied" => {
                    owner.assignments[0].state = RemoteViewAssignmentState::Active;
                    owner.denied_assignment = Some(prior.assignment_id.clone());
                }
                _ => (),
            }
        }
        std::fs::write(
            fixture.root.join("idle-process-absent"),
            "exact synthetic absence",
        )
        .unwrap();
        if case == "absence" {
            std::fs::write(fixture.root.join("recovery-absence-unknown"), "unknown").unwrap();
        }
        let mut restarted = host_mode(
            &fixture,
            provider.clone(),
            calls.clone(),
            Mode::IdleRecovery,
        );
        let result = restarted.execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "sessionId":alice.session_id, "activityAtMs":102
            }),
        );
        if case != "exact" {
            assert!(
                result.is_err(),
                "unqualified terminal evidence {case} must not replace a browser"
            );
            assert_eq!(calls.get(), 1, "{case}");
            assert_eq!(*restarted.state(), before, "{case}");
            assert_eq!(provider.borrow().acquisitions.len(), 1, "{case}");
            continue;
        }
        let result = result
            .expect("owner-terminal assignment must not strand retained recovery")
            .unwrap();
        assert_eq!(result["success"], true, "{result}");
        assert_eq!(result["browserSession"]["sessionId"], alice.session_id);
        let after = restarted.state();
        assert!(after.sessions.contains_key(&bob.session_id));
        assert_eq!(after.sessions.len(), before.sessions.len());
        assert_ne!(after.browsers[&alice.browser_id].desktop, original.desktop);
        assert_eq!(calls.get(), 2);
        assert_eq!(provider.borrow().acquisitions.len(), 2);
        assert!(provider.borrow().view_requests.is_empty());
        assert!(
            !fixture
                .store(false)
                .assignment_release_confirmed(&prior)
                .unwrap(),
            "owner terminal evidence is not a fabricated joined cleanup receipt"
        );
        assert!(fixture
            .store(false)
            .unpublished_launch_records()
            .unwrap()
            .is_empty());
    }
}

#[test]
fn recovery522_owner_terminal_evidence_survives_restart_and_fences_stale_admission() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider, calls);
    let opened = consumer
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    let before = consumer.state().clone();
    let browser = before.browsers[&opened.browser_id].clone();
    let prior = fixture
        .store(false)
        .published_launch_assignment(&browser)
        .unwrap();
    drop(consumer);
    let mut released = prior.clone();
    released.state = RemoteViewAssignmentState::Released;
    let mut wrong = released.clone();
    wrong.generation += 1;
    assert_eq!(
        fixture
            .store(false)
            .retain_owner_released_assignment(&browser, &before, &wrong),
        Err(LaunchCustodyStoreError::Conflict)
    );
    let mut stale = before.clone();
    stale.next_session_sequence += 1;
    assert_eq!(
        fixture
            .store(false)
            .retain_owner_released_assignment(&browser, &stale, &released),
        Err(LaunchCustodyStoreError::Conflict)
    );
    fixture
        .store(false)
        .retain_owner_released_assignment(&browser, &before, &released)
        .unwrap();
    let mut restarted = fixture.store(false);
    assert_eq!(
        restarted.published_launch_assignment(&browser).unwrap(),
        prior
    );
    assert!(!restarted.assignment_release_confirmed(&prior).unwrap());
    let mut fresh = prior.clone();
    fresh.assignment_id.push_str("-fresh");
    fresh.generation += 1;
    let intent = BrowserLaunchIntent {
        intent_id: uuid::Uuid::new_v4().to_string(),
        profile_id: browser.profile_id.clone(),
        assignment: fresh,
    };
    assert_eq!(
        restarted
            .admit_recovery_launch_intent(&intent, &before, &browser)
            .unwrap(),
        LaunchCustodyAdmission::New
    );
    drop(restarted);
    assert!(matches!(
        fixture
            .store(false)
            .admit_recovery_launch_intent(&intent, &before, &browser)
            .unwrap(),
        LaunchCustodyAdmission::Existing(_)
    ));
    fixture.store(false).save_session_state(&stale).unwrap();
    assert_eq!(
        fixture
            .store(false)
            .admit_recovery_launch_intent(&intent, &before, &browser),
        Err(LaunchCustodyStoreError::Conflict)
    );
}

#[test]
fn workspace_manual_disposable_uses_native_owner_and_retains_original_allocation() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut host = host(&fixture, provider, calls.clone());
    let command = serde_json::json!({"action":"browser_session_open","sessionName":"manual-disposable",
        "url":"about:blank","manualLogin":true,"leaveOpen":true,"activityAtMs":100,
        "serviceName":"fixture","agentName":"operator","originKind":"human","taskName":"Manual browsing"});
    let first = host.handle_command(&command);
    assert_eq!(first["success"], true, "{first}");
    let id = first["data"]["sessionId"].as_str().unwrap().to_owned();
    let profile_id = first["data"]["profileId"].as_str().unwrap().to_owned();
    let browser = host.state().browsers.values().next().unwrap();
    assert!(
        browser.cdp_endpoint.is_empty(),
        "CDP-free cannot acquire a debugging endpoint"
    );
    assert_eq!(
        host.state().disposable_profiles[&profile_id]
            .profile
            .manual_login_url
            .as_deref(),
        Some("about:blank")
    );
    assert!(host.state().leave_open_session_ids.contains(&id));
    assert_eq!(host.handle_command(&command)["data"]["sessionId"], id);
    assert_eq!(
        calls.get(),
        1,
        "reopening the same original task cannot launch again"
    );
    assert_eq!(host.state().disposable_profiles.len(), 1);
}

#[test]
fn workspace_manual_launch_retains_original_request_and_inspection_never_replays() {
    let fixture = Fixture::new();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), calls.clone());
    let inspect = consumer.handle_command(&serde_json::json!({"action":"browser_session_workspace_open", "operation":"inspect", "operationId":"8e54f8ab-91ed-4bcb-91a2-88ab0df1a001", "operator":"operator"}));
    assert!(
        inspect["error"]
            .as_str()
            .unwrap_or("")
            .contains("launch_unknown"),
        "{inspect}"
    );
    assert_eq!(calls.get(), 0);
    let executable = fixture.root.join("chrome");
    std::fs::copy("/bin/true", &executable).unwrap();
    use sha2::Digest;
    let digest = format!(
        "{:x}",
        sha2::Sha256::digest(std::fs::read(&executable).unwrap())
    );
    let operation = "8e54f8ab-91ed-4bcb-91a2-88ab0df1a001";
    let mut command = serde_json::json!({"action":"browser_session_workspace_open","operationId":operation,"operator":"operator","intent":{"operationId":operation,"profileMode":"disposable","mode":"cdp_free","url":"about:blank","leaveOpen":true,"buildSelection":{"executablePath":executable,"expectedSha256":digest}}});
    let opened = consumer.handle_command(&command);
    assert_eq!(opened["success"], true, "{opened}");
    let inspected=consumer.handle_command(&serde_json::json!({"action":"browser_session_workspace_open","operation":"inspect","operationId":operation,"operator":"operator"}));
    assert_eq!(inspected["data"], opened["data"]);
    assert_eq!(calls.get(), 1);
    let mut pending = consumer.state().clone();
    pending
        .managed_tab_requests
        .values_mut()
        .find(|row| row.session_name == opened["data"]["sessionName"].as_str().unwrap())
        .unwrap()
        .response = None;
    drop(consumer);
    fixture.store(false).save_session_state(&pending).unwrap();
    let mut consumer = host(&fixture, provider, calls.clone());
    let recovered=consumer.handle_command(&serde_json::json!({"action":"browser_session_workspace_open","operation":"inspect","operationId":operation,"operator":"operator"}));
    assert_eq!(recovered["success"], true, "{recovered}");
    assert_eq!(recovered["data"]["taskId"], opened["data"]["taskId"]);
    assert_eq!(recovered["data"]["initialNavigation"], "unconfirmed");
    assert_eq!(calls.get(), 1);
    command["intent"]["url"] = serde_json::json!("https://example.test/");
    let changed = consumer.handle_command(&command);
    assert!(
        changed["error"]
            .as_str()
            .unwrap_or("")
            .contains("intent_changed"),
        "{changed}"
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn lifecycle_window_readiness_uses_authenticated_observe_only_and_exact_identity() {
    use std::io::{Read, Write};
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("live-cdp-endpoint.json"), serde_json::to_vec(&serde_json::json!({
        "pid":42,"endpoint":"ws://127.0.0.1:9222","processIdentity":{"pid":42,"startToken":"linux:fixture-boot:99","executablePath":"/synthetic/chrome"}
    })).unwrap()).unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let mut consumer = host(&fixture, provider.clone(), Rc::new(Cell::new(0)));
    let opened = consumer.handle_command(&serde_json::json!({"action":"browser_session_open","sessionName":"Window readiness","profileId":"profile-a","manualLogin":true,"url":"https://example.test/login","activityAtMs":100,"serviceName":"fixture","agentName":"fixture","taskName":"Window readiness"}));
    assert_eq!(opened["success"], true, "{opened}");
    provider.borrow_mut().window_pid = Some(42);
    // Mirror copy launch: acquire this task's first tab before window proof.
    consumer
        .tab_for_navigation(opened["data"]["sessionId"].as_str().unwrap(), 101)
        .unwrap();
    let task = consumer.state().sessions.values().next().unwrap().clone();
    let target = RemoteViewTabHandoffTarget {
        browser: consumer.state().browsers[&task.browser_id].clone(),
        tab: consumer.state().tabs[task.current_tab_id.as_ref().unwrap()].clone(),
        session: task,
    };
    let token = fixture.root.join("window-owner.token");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(&token)
        .unwrap()
        .write_all(b"synthetic-window-owner-token-530")
        .unwrap();
    let environment = crate::test_utils::EnvGuard::new(&[
        "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
        "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
    ]);
    environment.set(
        "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
        token.to_str().unwrap(),
    );
    let binding = wire()["assignmentObservation"]["target"].clone();
    for case in [
        "ready_input_disabled",
        "changed_generation",
        "changed_identity",
    ] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        environment.set(
            "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
            &format!("http://{}", listener.local_addr().unwrap()),
        );
        let binding = binding.clone();
        let server = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "owner observation not requested"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(error) => panic!("owner fixture accept failed: {error}"),
                }
            };
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut bytes = Vec::new();
            let at = loop {
                let mut chunk = [0u8; 4096];
                let size = socket.read(&mut chunk).unwrap();
                assert!(size > 0);
                bytes.extend_from_slice(&chunk[..size]);
                assert!(bytes.len() <= 8192);
                if let Some(at) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..at]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap();
                    if bytes.len() >= at + 4 + length {
                        break at;
                    }
                }
            };
            let headers = String::from_utf8_lossy(&bytes[..at]);
            assert_eq!(
                headers.lines().next().unwrap(),
                "POST /v1/agent-desktop/observe HTTP/1.1"
            );
            assert!(
                headers.lines().any(|line| line.eq_ignore_ascii_case(
                    "authorization: Bearer synthetic-window-owner-token-530"
                )),
                "owner authentication missing"
            );
            let body: Value = serde_json::from_slice(&bytes[at + 4..]).unwrap();
            assert_eq!(body["desktopId"], binding["viewingDesktopId"]);
            assert_eq!(body["expectedGeneration"], binding["viewingGeneration"]);
            let generation = binding["viewingGeneration"].as_u64().unwrap()
                + u64::from(case == "changed_generation");
            let start = if case == "changed_identity" {
                "100"
            } else {
                "99"
            };
            let reply = serde_json::json!({"desktopId":binding["viewingDesktopId"],"generation":generation,"inputCapability":{"policyState":"disabled"},"windows":{"windows":[{"id":5,"pid":42,"active":true,"x":0,"y":0,"width":400,"height":300,"processIdentity":{"pid":42,"executable":"/synthetic/chrome","boot_id":"fixture-boot","start_token":start}}]}});
            let encoded = serde_json::to_vec(&reply).unwrap();
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",encoded.len()).unwrap();
            socket.write_all(&encoded).unwrap();
        });
        let result = consumer.native_browser_window_ready(&target);
        server.join().unwrap();
        if case == "ready_input_disabled" {
            let ready = result.unwrap();
            assert_eq!(ready["ready"], true);
            assert!(ready.get("inputReady").is_none());
            assert!(ready.get("inputReadiness").is_none());
        } else {
            assert!(result.is_err(), "{case} cannot qualify a window");
        }
    }
    let mut changed = target.clone();
    changed.browser.pid += 1;
    assert!(consumer
        .native_browser_window_ready(&changed)
        .unwrap_err()
        .starts_with("browser_native_target_changed"));
    let mut changed = consumer.state().clone();
    changed.next_session_sequence += 1;
    fixture.store(false).save_session_state(&changed).unwrap();
    assert!(consumer
        .native_browser_window_ready(&target)
        .unwrap_err()
        .starts_with("browser_native_target_changed"));
}

#[cfg(target_os = "linux")]
struct CopyDisposalHttp {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    server: Option<std::thread::JoinHandle<()>>,
}
#[cfg(target_os = "linux")]
impl CopyDisposalHttp {
    fn new(root: &Path, environment: &crate::test_utils::EnvGuard<'_>) -> Self {
        use std::io::{Read, Write};
        let token = root.join("copy-disposal-owner.token");
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&token)
            .unwrap()
            .write_all(b"synthetic-copy-disposal-owner-token")
            .unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        environment.set(
            "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
            &format!("http://{}", listener.local_addr().unwrap()),
        );
        environment.set(
            "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
            token.to_str().unwrap(),
        );
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopping = stop.clone();
        let root = root.to_path_buf();
        let server = std::thread::spawn(move || {
            while !stopping.load(std::sync::atomic::Ordering::SeqCst) {
                let mut socket = match listener.accept() {
                    Ok((socket, _)) => socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("copy owner fixture accept failed: {error}"),
                };
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let at = loop {
                    let mut chunk = [0u8; 4096];
                    let count = socket.read(&mut chunk).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&chunk[..count]);
                    assert!(bytes.len() <= 8192);
                    if let Some(at) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..at]);
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                            })
                            .unwrap();
                        if bytes.len() >= at + 4 + length {
                            break at;
                        }
                    }
                };
                let headers = String::from_utf8_lossy(&bytes[..at]);
                assert_eq!(
                    headers.lines().next().unwrap(),
                    "POST /v1/agent-desktop/observe HTTP/1.1"
                );
                assert!(headers.lines().any(|line| line.eq_ignore_ascii_case(
                    "authorization: Bearer synthetic-copy-disposal-owner-token"
                )));
                let body: Value = serde_json::from_slice(&bytes[at + 4..]).unwrap();
                let observed: Value = serde_json::from_slice(
                    &std::fs::read(root.join("copy-process-observation.json")).unwrap(),
                )
                .unwrap();
                let identity = &observed["processIdentity"];
                let token = identity["startToken"].as_str().unwrap();
                let (boot, start) = token
                    .strip_prefix("linux:")
                    .unwrap()
                    .rsplit_once(':')
                    .unwrap();
                let reply = serde_json::json!({"desktopId":body["desktopId"],"generation":body["expectedGeneration"],"windows":{"windows":[{"id":1,"pid":identity["pid"],"active":true,"x":0,"y":0,"width":400,"height":300,"processIdentity":{"pid":identity["pid"],"executable":identity["executablePath"],"boot_id":boot,"start_token":start}}]}});
                let encoded = serde_json::to_vec(&reply).unwrap();
                write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",encoded.len()).unwrap();
                socket.write_all(&encoded).unwrap();
            }
        });
        Self {
            stop,
            server: Some(server),
        }
    }
}
#[cfg(target_os = "linux")]
impl Drop for CopyDisposalHttp {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        self.server.take().unwrap().join().unwrap();
    }
}

#[test]
#[cfg(target_os = "linux")]
fn copy_disposal_public_completed_close_reload_dispose_preserves_source_and_peer() {
    copy_disposal_case("normal");
}
#[test]
#[cfg(target_os = "linux")]
fn copy_disposal_retained_owner_readback_refreshes_closed_copy_snapshot() {
    copy_disposal_case("retained_snapshot");
}
#[cfg(target_os = "linux")]
fn copy_disposal_case(case: &str) {
    let root = std::env::temp_dir().join(format!("copy-disposal-public-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join(".agent-browser/service")).unwrap();
    let environment = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_CONFIG",
        "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
        "AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE",
    ]);
    environment.set("HOME", root.to_str().unwrap());
    environment.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    environment.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    environment.remove("AGENT_BROWSER_CONFIG");
    let fixture = Fixture::with_root(root.join(".agent-browser/service"), true);
    for (id, bytes) in [
        ("profile-a", b"source bytes".as_slice()),
        ("profile-b", b"peer bytes".as_slice()),
    ] {
        std::fs::create_dir_all(fixture.root.join(id)).unwrap();
        std::fs::write(fixture.root.join(id).join("retained"), bytes).unwrap();
    }
    let processes = CopyProcessFixture::new(&fixture.root);
    let owner = CopyDisposalHttp::new(&fixture.root, &environment);
    let provider = Rc::new(RefCell::new(Provider {
        pool_maximum: Some(4),
        ..Provider::default()
    }));
    let launches = Rc::new(Cell::new(0));
    let mut consumer = host(&fixture, provider.clone(), launches.clone());
    let peer=consumer.handle_command(&serde_json::json!({"action":"browser_session_open","profileId":"profile-b","sessionName":"Peer retained work","manualLogin":true,"url":"https://peer.example.test/login","activityAtMs":100,"serviceName":"fixture","agentName":"fixture","taskName":"Peer retained work"}));
    assert_eq!(peer["success"], true, "{peer}");
    let peer_task = peer["data"]["sessionId"].as_str().unwrap().to_owned();
    consumer.tab_for_navigation(&peer_task, 101).unwrap();
    let peer_browser =
        consumer.state().browsers[peer["data"]["browserId"].as_str().unwrap()].clone();
    let peer_identity = crate::process_identity::observe_process(peer_browser.pid);
    let opened=consumer.handle_command(&serde_json::json!({"action":"browser_session_open","profileId":"profile-a","sessionName":"Source copy","manualLogin":true,"url":"https://example.test/login","activityAtMs":100,"serviceName":"fixture","agentName":"fixture","taskName":"Source copy"}));
    assert_eq!(opened["success"], true, "{opened}");
    consumer
        .tab_for_navigation(opened["data"]["sessionId"].as_str().unwrap(), 101)
        .unwrap();
    let original_catalog = fixture.store(false).load_profile_catalog().unwrap().catalog;
    let prepare=consumer.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","sourceProfileId":"profile-a","destinationName":"Synthetic independent copy","choice":"prepare","launch":true}));
    assert_eq!(prepare["success"], true, "{prepare}");
    let completed=consumer.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","choice":"close_and_duplicate","ownerConfirmed":true}));
    assert_eq!(
        completed["data"]["phase"], "completed",
        "public hosted copy must really complete: {completed}"
    );
    let destination = completed["data"]["destination"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let copied_path = fixture
        .store(false)
        .load_profile_catalog()
        .unwrap()
        .catalog
        .profiles[&destination]
        .user_data_dir
        .clone();
    if case == "live" {
        let refused=consumer.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","choice":"dispose_copy","ownerConfirmed":true}));
        assert_eq!(refused["success"], false, "{refused}");
        assert!(Path::new(&copied_path).is_dir());
    }
    let closed=consumer.handle_command(&serde_json::json!({"action":"browser_session_close","sessionId":completed["data"]["launch"]["taskId"],"scope":"browser","activityAtMs":200,"actor":"synthetic-owner","reason":"close the completed synthetic copy before disposal"}));
    assert_eq!(closed["data"]["result"]["closed"], true, "{closed}");
    assert_eq!(closed["data"]["residue"]["originalProcess"], "absent");
    let launch_count = launches.get();
    drop(consumer);
    let mut retained_owner = host(&fixture, provider.clone(), launches.clone());
    let mut reloaded = host(&fixture, provider.clone(), launches.clone());
    // OS process enumeration is an external boundary. The exact published child
    // identities above still require real kernel exit; unrelated census is synthetic.
    let census = super::super::service_resources::CleanupCensusFixture::new(Vec::new());
    let disposal_directory = fixture.root.join("profile-duplication");
    let marker = disposal_directory.join("fixture-disposal-stop-after");
    let request = serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","choice":"dispose_copy","ownerConfirmed":true});
    let mut restore_catalog = None;
    let mut restore_fence = None;
    let mut moved_payload = None;
    match case {
        "confirmation" => {
            let mut bad = request.clone();
            bad["ownerConfirmed"] = false.into();
            assert_eq!(reloaded.handle_command(&bad)["success"], false);
        }
        "forged_target" => {
            for field in [
                "destinationProfileId",
                "path",
                "actor",
                "requestPrincipalSource",
            ] {
                let mut bad = request.clone();
                bad[field] = "profile-a".into();
                assert_eq!(reloaded.handle_command(&bad)["success"], false, "{field}");
            }
        }
        "pending_intent" => {
            let pending=reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"other-copy-intent","sourceProfileId":destination,"choice":"prepare","launch":false}));
            assert_eq!(pending["success"], true, "{pending}");
        }
        "lock" => std::fs::write(
            Path::new(&copied_path).join("SingletonLock"),
            b"synthetic lock",
        )
        .unwrap(),
        "unknown_census" => census.fail("synthetic_census_unavailable"),
        "foreign_fence" => {
            let mut store = fixture.store(false);
            let before = store.load_session_state().unwrap();
            let mut next = before.clone();
            next.profile_operation_ids
                .insert(destination.clone(), "foreign-operation".into());
            store
                .compare_and_save_session_state(&before, &next)
                .unwrap();
            restore_fence = Some(before);
        }
        "catalog_alias" => {
            let store = fixture.store(false);
            let before = store.load_profile_catalog().unwrap().catalog;
            let mut next = before.clone();
            let mut alias = next.profiles[&destination].clone();
            alias.id = "foreign-alias".into();
            next.profiles.insert(alias.id.clone(), alias);
            store
                .compare_and_save_profile_catalog(&before, &next)
                .unwrap();
            restore_catalog = Some(before);
        }
        "lineage" => {
            rusqlite::Connection::open(fixture.root.join("runtime.sqlite3")).unwrap().execute("UPDATE browser_profile_duplication_lineage SET operation_id='foreign-operation' WHERE profile_id=?1",[&destination]).unwrap();
        }
        "symlink" => std::os::unix::fs::symlink(
            fixture.root.join("profile-a/retained"),
            Path::new(&copied_path).join("foreign-link"),
        )
        .unwrap(),
        "root_symlink" => {
            let moved = fixture.root.join("held-copy");
            std::fs::rename(&copied_path, &moved).unwrap();
            std::os::unix::fs::symlink(&moved, &copied_path).unwrap();
            moved_payload = Some(moved);
        }
        _ => {}
    }
    if matches!(
        case,
        "pending_intent"
            | "lock"
            | "unknown_census"
            | "foreign_fence"
            | "catalog_alias"
            | "lineage"
            | "symlink"
            | "root_symlink"
    ) {
        let blocked = reloaded.handle_command(&request);
        assert_eq!(blocked["success"], false, "{case}: {blocked}");
        assert!(Path::new(&copied_path).exists());
        if case == "foreign_fence" {
            let cancelled=reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","choice":"cancel"}));
            assert_eq!(cancelled["data"]["phase"], "completed");
            assert_eq!(
                fixture
                    .store(false)
                    .load_session_state()
                    .unwrap()
                    .profile_operation_ids[&destination],
                "foreign-operation"
            );
        }
        match case {
            "pending_intent"=>assert_eq!(reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"other-copy-intent","choice":"cancel"}))["data"]["phase"],"cancelled"),
            "lock"=>std::fs::remove_file(Path::new(&copied_path).join("SingletonLock")).unwrap(),
            "unknown_census"=>census.restore(),
            "foreign_fence"=>{let mut store=fixture.store(false);let now=store.load_session_state().unwrap();store.compare_and_save_session_state(&now,&restore_fence.take().unwrap()).unwrap();},
            "catalog_alias"=>{let store=fixture.store(false);let now=store.load_profile_catalog().unwrap().catalog;store.compare_and_save_profile_catalog(&now,&restore_catalog.take().unwrap()).unwrap();},
            "lineage"=>{rusqlite::Connection::open(fixture.root.join("runtime.sqlite3")).unwrap().execute("UPDATE browser_profile_duplication_lineage SET operation_id='copy-disposal-public' WHERE profile_id=?1",[&destination]).unwrap();},
            "symlink"=>std::fs::remove_file(Path::new(&copied_path).join("foreign-link")).unwrap(),
            "root_symlink"=>{std::fs::remove_file(&copied_path).unwrap();std::fs::rename(moved_payload.take().unwrap(),&copied_path).unwrap();},
            _=>{}
        }
    }
    let stop = case
        .strip_prefix("restart_")
        .or_else(|| case.strip_prefix("missing_"));
    if let Some(stop) = stop {
        std::fs::write(&marker, stop).unwrap();
    }
    if case == "foreign_after_intent" {
        std::fs::write(&marker, "prepared").unwrap();
    }
    let mut disposed = reloaded.handle_command(&request);
    if stop.is_some() || case == "foreign_after_intent" {
        assert_eq!(
            disposed["data"]["state"], completed["data"]["state"],
            "cleanup must preserve public copy state"
        );
        assert_eq!(
            disposed["data"]["sourceObservation"], completed["data"]["sourceObservation"],
            "cleanup must preserve retained source observation"
        );
        assert_eq!(
            disposed["data"]["cleanup"]["state"], "readback_required",
            "{case}: {disposed}"
        );
        let checkpoint = disposed["data"]["cleanup"]["checkpoint"].clone();
        let inspect=reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","sourceProfileId":"profile-a","choice":"inspect"}));
        let view = &inspect["data"]["operations"][0];
        assert_eq!(view["cleanup"]["checkpoint"], checkpoint, "{inspect}");
        assert_eq!(
            view["actions"],
            serde_json::json!([{"action":"reconcile","requiresConfirmation":false}])
        );
        std::fs::remove_file(&marker).unwrap();
        drop(reloaded);
        reloaded = host(&fixture, provider.clone(), launches.clone());
        assert_eq!(
            launches.get(),
            launch_count,
            "restart loads retained closed history without a dummy browser"
        );
        if case.starts_with("missing_") {
            let payload = if case == "missing_prepared" {
                PathBuf::from(&copied_path)
            } else {
                std::fs::read_dir(disposal_directory.join("disposal-quarantine"))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path()
            };
            std::fs::remove_dir_all(payload).unwrap();
            let unknown=reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","choice":"reconcile"}));
            assert_eq!(
                unknown["data"]["cleanup"]["state"], "readback_required",
                "{unknown}"
            );
            assert_eq!(
                unknown["data"]["cleanup"]["deleteAcknowledged"],
                Value::Null
            );
            assert_eq!(
                std::fs::read(fixture.root.join("profile-a/retained")).unwrap(),
                b"source bytes"
            );
            assert_eq!(
                std::fs::read(fixture.root.join("profile-b/retained")).unwrap(),
                b"peer bytes"
            );
            return;
        }
        if case == "foreign_after_intent" {
            let mut store = fixture.store(false);
            let before = store.load_session_state().unwrap();
            let mut next = before.clone();
            next.profile_operation_ids
                .insert(destination.clone(), "foreign-operation".into());
            store
                .compare_and_save_session_state(&before, &next)
                .unwrap();
            reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","choice":"cancel"}));
            let refusal=reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","choice":"reconcile"}));
            assert_eq!(refusal["data"]["cleanup"]["state"], "readback_required");
            assert_eq!(
                store.load_session_state().unwrap().profile_operation_ids[&destination],
                "foreign-operation"
            );
            let now = store.load_session_state().unwrap();
            store.compare_and_save_session_state(&now, &before).unwrap();
        }
        disposed=reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","choice":"reconcile"}));
    }
    assert_eq!(disposed["data"]["state"], completed["data"]["state"]);
    assert_eq!(
        disposed["data"]["sourceObservation"],
        completed["data"]["sourceObservation"]
    );
    assert_eq!(
        disposed["data"]["cleanup"]["state"], "completed",
        "{case}: {disposed}"
    );
    assert_eq!(
        disposed["data"]["cleanup"]["deleteAcknowledged"],
        case != "restart_deleted"
    );
    if case == "restart_deleted" {
        assert_eq!(
            disposed["data"]["cleanup"]["completionEvidence"],
            "absent_after_retained_delete_intent"
        );
    }
    let inspect=reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","sourceProfileId":"profile-a","choice":"inspect"}));
    let view = &inspect["data"]["operations"][0];
    assert_eq!(view["cleanup"]["state"], "completed", "{inspect}");
    assert_eq!(view["actions"], serde_json::json!([]));
    for field in [
        "phase",
        "source",
        "destination",
        "launch",
        "completionOutcome",
        "authenticationContinuity",
    ] {
        assert_eq!(
            view[field], completed["data"][field],
            "original history {field}"
        );
    }
    for choice in [
        "prepare",
        "reconcile",
        "close_and_duplicate",
        "cancel",
        "focus_source",
        "retry_launch",
        "finish_without_opening",
        "open_source",
    ] {
        let replay=reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","sourceProfileId":"profile-a","choice":choice,"ownerConfirmed":true}));
        assert_eq!(
            replay["data"]["cleanup"]["state"], "completed",
            "{choice}: {replay}"
        );
        assert_eq!(replay["data"]["actions"], serde_json::json!([]));
    }
    let wrong=reloaded.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","choice":"close_and_duplicate","sourceProfileId":"profile-b","ownerConfirmed":true}));
    assert_eq!(wrong["success"], false);
    reloaded.tab_for_navigation(&peer_task, 202).unwrap();
    assert_eq!(
        crate::process_identity::observe_process(peer_browser.pid),
        peer_identity,
        "active peer kernel identity preserved"
    );
    let after_catalog = fixture.store(false).load_profile_catalog().unwrap().catalog;
    assert_eq!(
        after_catalog.profiles["profile-a"],
        original_catalog.profiles["profile-a"]
    );
    assert_eq!(
        after_catalog.profiles["profile-b"],
        original_catalog.profiles["profile-b"]
    );
    assert_eq!(
        launches.get(),
        launch_count,
        "disposal after normal close never opens a dummy or source browser"
    );
    if case == "retained_snapshot" {
        let readback=retained_owner.handle_command(&serde_json::json!({"action":"browser_session_profile_duplicate","operationId":"copy-disposal-public","sourceProfileId":"profile-a","choice":"prepare"}));
        assert_eq!(readback["data"]["cleanup"]["state"], "completed");
        let refusal = retained_owner
            .open(OpenBrowserSession::exact_profile(
                "disposed profile must not open",
                &destination,
                201,
            ))
            .unwrap_err();
        assert!(
            refusal.starts_with("browser_profile_not_found:"),
            "retained catalog/state must refresh before the next owner call: {refusal}"
        );
        assert_eq!(launches.get(), launch_count);
    }
    assert!(!Path::new(&copied_path).exists());
    assert!(!fixture
        .store(false)
        .load_profile_catalog()
        .unwrap()
        .catalog
        .profiles
        .contains_key(&destination));
    assert_eq!(
        std::fs::read(fixture.root.join("profile-a/retained")).unwrap(),
        b"source bytes"
    );
    assert_eq!(
        std::fs::read(fixture.root.join("profile-b/retained")).unwrap(),
        b"peer bytes"
    );
    assert!(
        fixture
            .store(false)
            .load_profile_catalog()
            .unwrap()
            .catalog
            .profiles
            .contains_key("profile-a")
            && fixture
                .store(false)
                .load_profile_catalog()
                .unwrap()
                .catalog
                .profiles
                .contains_key("profile-b")
    );
    let repository =
        super::super::service_store::LockedServiceStateRepository::default_json().unwrap();
    assert!(
        !super::super::service_store::ServiceStateRepository::load_snapshot(&repository)
            .unwrap()
            .profiles
            .contains_key(&destination),
        "Native copy/open did not persist a legacy profile projection"
    );
    drop(reloaded);
    drop(retained_owner);
    drop(owner);
    drop(processes);
    drop(fixture);
    drop(environment);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn copy_disposal_rejects_live_references_untrusted_targets_and_foreign_metadata() {
    for case in [
        "live",
        "confirmation",
        "forged_target",
        "pending_intent",
        "lock",
        "unknown_census",
        "foreign_fence",
        "catalog_alias",
        "lineage",
        "symlink",
        "root_symlink",
        "foreign_after_intent",
    ] {
        copy_disposal_case(case);
    }
}
#[test]
#[cfg(target_os = "linux")]
fn copy_disposal_resumes_only_original_receipt_across_owner_restarts() {
    for case in [
        "restart_prepared",
        "restart_renamed",
        "restart_quarantined",
        "restart_catalog_removed",
        "restart_deleting",
        "restart_deleted",
    ] {
        copy_disposal_case(case);
    }
}
#[test]
#[cfg(target_os = "linux")]
fn copy_disposal_missing_unacknowledged_payload_never_becomes_terminal() {
    for case in [
        "missing_prepared",
        "missing_quarantined",
        "missing_catalog_removed",
    ] {
        copy_disposal_case(case);
    }
}

#[test]
fn requested_profile_recovers_its_settled_unpublished_retirement_after_scope_ended() {
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("count-retirement-closes"), "check").unwrap();
    let provider = Rc::new(RefCell::new(Provider {
        pool_maximum: Some(4),
        ..Default::default()
    }));
    let calls = Rc::new(Cell::new(0));
    let mode = Rc::new(Cell::new(0));
    let mut original = host_with_persistence(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::Success,
        || 2001,
        NativeUseReplyFailure {
            store: fixture.store(false),
            mode: mode.clone(),
            attempts: Rc::new(Cell::new(0)),
        },
    );
    let first = original
        .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
        .unwrap();
    mode.set(5);
    assert_eq!(
        original.reap(300_101).unwrap_err(),
        "synthetic_retirement_publication_unavailable"
    );
    provider.borrow_mut().window_pid = Some(44);
    let peer = original
        .open(OpenBrowserSession::exact_profile(
            "Bob",
            "profile-b",
            300_102,
        ))
        .unwrap();
    mode.set(5);
    assert_eq!(
        original.reap(600_103).unwrap_err(),
        "synthetic_retirement_publication_unavailable"
    );
    let records = fixture.store(false).idle_retirement_records().unwrap();
    assert_eq!(records.len(), 2);
    let own = records
        .iter()
        .find(|r| r.has_settled_browser_close(&first.browser_id))
        .unwrap()
        .clone();
    let unrelated = records
        .iter()
        .find(|r| r.has_settled_browser_close(&peer.browser_id))
        .unwrap()
        .clone();
    assert!(own.published_state_digest.is_none() && unrelated.published_state_digest.is_none());
    let before = fixture.store(false).load_session_state().unwrap();
    // Model an external operator ending logical inventory after the recorded
    // physical closes. Keep real SQLite custody and both original Native fences.
    let ended = BrowserSessionState {
        next_session_sequence: before.next_session_sequence,
        retention_policy: before.retention_policy.clone(),
        ..Default::default()
    };
    fixture.store(false).save_session_state(&ended).unwrap();
    drop(original);
    provider.borrow_mut().window_pid = Some(45);
    let mut restarted = host(&fixture, provider, calls.clone());
    let fresh = restarted
        .open(OpenBrowserSession::exact_profile(
            "Alice",
            "profile-a",
            600_104,
        ))
        .expect("requested profile must finish its settled transaction before new admission");
    assert_eq!(fresh.profile_id, "profile-a");
    assert_eq!(calls.get(), 3);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("retirement-effect-count")).unwrap(),
        "2",
        "each original physical close executes once"
    );
    let records = fixture.store(false).idle_retirement_records().unwrap();
    let completed = records.iter().find(|r| r.intent == own.intent).unwrap();
    assert!(completed.published_state_digest.is_some() && completed.is_terminal());
    assert_eq!(
        records
            .iter()
            .find(|r| r.intent == unrelated.intent)
            .unwrap(),
        &unrelated,
        "another profiles prepared retirement remains untouched"
    );
}

#[test]
fn scoped_settled_retirement_reconciliation_preserves_unqualified_evidence() {
    for case in ["unknown_effect", "provider_unavailable", "changed_identity"] {
        let fixture = Fixture::new();
        std::fs::write(fixture.root.join("count-retirement-closes"), "check").unwrap();
        let provider = Rc::new(RefCell::new(Provider::default()));
        let calls = Rc::new(Cell::new(0));
        let mode = Rc::new(Cell::new(0));
        let mut original = host_with_persistence(
            &fixture,
            provider.clone(),
            calls.clone(),
            Mode::Success,
            || 2001,
            NativeUseReplyFailure {
                store: fixture.store(false),
                mode: mode.clone(),
                attempts: Rc::new(Cell::new(0)),
            },
        );
        let first = original
            .open(OpenBrowserSession::exact_profile("Alice", "profile-a", 100))
            .unwrap();
        if case == "unknown_effect" {
            std::fs::write(fixture.root.join("fail-retirement-close"), "unknown").unwrap();
        } else {
            mode.set(5);
        }
        let _ = original.reap(300_101);
        let records = fixture.store(false).idle_retirement_records().unwrap();
        assert_eq!(records.len(), 1, "{case}");
        assert!(
            records[0].has_effect_attempts(),
            "fixture must reach the original effect: {case}"
        );
        assert_eq!(
            records[0].has_settled_browser_close(&first.browser_id),
            case != "unknown_effect"
        );
        let before = fixture.store(false).load_session_state().unwrap();
        let mut current = BrowserSessionState {
            next_session_sequence: before.next_session_sequence,
            retention_policy: before.retention_policy.clone(),
            ..Default::default()
        };
        if case == "changed_identity" {
            current = before;
            current.browsers.get_mut(&first.browser_id).unwrap().pid += 100;
        }
        fixture.store(false).save_session_state(&current).unwrap();
        drop(original);
        if case == "provider_unavailable" {
            provider.borrow_mut().unavailable_idle_retirement_assignment =
                Some(records[0].intent.provider.assignment.assignment_id.clone());
        }
        let profile = fixture
            .store(false)
            .load_profile_catalog()
            .unwrap()
            .catalog
            .profiles["profile-a"]
            .clone();
        let mut effects = RemoteViewSessionEffects::new(
            Process {
                mode: Mode::Success,
                root: fixture.root.clone(),
                calls,
            },
            adapter(&fixture, provider),
            fixture.store(false),
            Vec::new(),
            || uuid::Uuid::new_v4().to_string(),
        )
        .unwrap()
        .with_pool(RemoteViewSessionPool {
            name: "main".into(),
            desired_desktops: 1,
        })
        .unwrap();
        effects.begin_operation(&current).unwrap();
        let _ = effects.reconcile_profile_browser(
            &profile,
            current.browsers.get(&first.browser_id),
            &[],
            &[],
        );
        assert_eq!(
            fixture.store(false).idle_retirement_records().unwrap(),
            records,
            "unqualified evidence must not publish or complete: {case}"
        );
        assert_eq!(
            fixture.store(false).load_session_state().unwrap(),
            current,
            "{case}"
        );
        assert_eq!(
            std::fs::read_to_string(fixture.root.join("retirement-effect-count")).unwrap(),
            "1",
            "no physical close replay: {case}"
        );
    }
}

#[test]
fn retained_handoff_recovers_after_settled_close_publication_was_interrupted() {
    retained_handoff_retirement_fixture(false, None);
}

#[test]
fn retained_interactive_handoff_recovers_with_original_invitation() {
    retained_handoff_retirement_fixture(true, None);
}

#[test]
fn retained_interactive_recovery_rejects_unqualified_scope_and_control() {
    for case in [
        "missing_prior",
        "ambiguous_launch",
        "nonterminal_retirement",
        "changed_tab",
        "target_swap",
        "legacy_mapping",
        "original_live",
        "stale_control",
        "foreign_hold",
        "stale_snapshot",
    ] {
        retained_handoff_retirement_fixture(true, Some(case));
    }
}

fn retained_handoff_retirement_fixture(interactive: bool, refusal: Option<&str>) {
    let fixture = Fixture::new();
    // Synthetic browser IDs are stable across fixture instances; their control
    // journal must have the same per-fixture isolation as the SQLite database.
    let environment_guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_CONFIG",
        "XDG_CONFIG_HOME",
        "XDG_STATE_HOME",
        "XDG_CACHE_HOME",
    ]);
    environment_guard.set("HOME", fixture.root.to_str().unwrap());
    environment_guard.set("AGENT_BROWSER_HOME", fixture.root.to_str().unwrap());
    environment_guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    environment_guard.remove("AGENT_BROWSER_CONFIG");
    environment_guard.set(
        "XDG_CONFIG_HOME",
        fixture.root.join("config").to_str().unwrap(),
    );
    environment_guard.set(
        "XDG_STATE_HOME",
        fixture.root.join("state").to_str().unwrap(),
    );
    environment_guard.set(
        "XDG_CACHE_HOME",
        fixture.root.join("cache").to_str().unwrap(),
    );
    std::fs::create_dir_all(fixture.root.join(".agent-browser/service")).unwrap();
    std::fs::write(fixture.root.join("count-retirement-closes"), "check").unwrap();
    let provider = Rc::new(RefCell::new(Provider::default()));
    let calls = Rc::new(Cell::new(0));
    let mode = Rc::new(Cell::new(0));
    let mut original = host_with_persistence(
        &fixture,
        provider.clone(),
        calls.clone(),
        Mode::Success,
        || 2001,
        NativeUseReplyFailure {
            store: fixture.store(false),
            mode: mode.clone(),
            attempts: Rc::new(Cell::new(0)),
        },
    );
    let opened = original
        .execute_managed_command(
            "Alice",
            &serde_json::json!({
                "action":"get_url", "runtimeProfile":"profile-a", "activityAtMs":100
            }),
        )
        .unwrap()
        .unwrap();
    let identity = opened["browserSession"].clone();
    let browser_id = identity["browserId"].as_str().unwrap();
    let handoff_id = identity["handoffId"].as_str().unwrap();
    mode.set(5);
    assert_eq!(
        original.reap(300_101).unwrap_err(),
        "synthetic_retirement_publication_unavailable"
    );
    let before = fixture.store(false).load_session_state().unwrap();
    let retirement = fixture
        .store(false)
        .idle_retirement_records()
        .unwrap()
        .remove(0);
    assert!(retirement.has_settled_browser_close(browser_id));
    assert!(retirement.published_state_digest.is_none());
    assert!(!before.idle_closed_browsers.contains_key(browser_id));
    assert!(before.remote_view_tab_handoffs.contains_key(handoff_id));
    let original_digest = if interactive {
        let target =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&before, handoff_id)
                .unwrap();
        let digest =
            agent_browser_service_model::remote_view_native_target_digest(handoff_id, &target)
                .unwrap();
        let control_path = crate::native::browser_host_control::path(browser_id).unwrap();
        std::fs::create_dir_all(control_path.parent().unwrap()).unwrap();
        if refusal.is_none() {
            crate::native::browser_host_control::begin_handoff(browser_id, handoff_id, &digest)
                .unwrap();
        }
        Some(digest)
    } else {
        None
    };
    drop(original);
    std::fs::write(fixture.root.join("idle-process-absent"), "absent").unwrap();
    provider.borrow_mut().window_pid = Some(44);
    let mut restarted = host_mode(&fixture, provider, calls.clone(), Mode::IdleRecovery);
    if let Some(case) = refusal {
        // Use the exact public Native recovery path proven by the positive case.
        // Receipt publication is independent of interactive control. Reconstruct
        // the old invitation afterward only for adversarial guard qualification;
        // the positive case retains invitation-before-recovery ordering.
        let prepared = restarted.handle_command(&serde_json::json!({
            "action":"service_remote_view_handoff_resolve", "handoffId":handoff_id,
            "nativeDesktop":true, "activityAtMs":300_102
        }));
        assert_eq!(prepared["success"], true, "{case}: {prepared}");
        assert_eq!(prepared["data"]["operatorVisible"]["state"], "ready");
        crate::native::browser_host_control::begin_handoff(
            browser_id,
            handoff_id,
            original_digest.as_deref().unwrap(),
        )
        .unwrap();
        let database = fixture.store(false).runtime_database_path().unwrap();
        let connection = rusqlite::Connection::open(&database).unwrap();
        let read_document = |kind: &str| -> Value {
            serde_json::from_str(
                &connection
                    .query_row(
                        "SELECT json FROM state_documents WHERE kind=?1",
                        [kind],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap(),
            )
            .unwrap()
        };
        let published = fixture.store(false).load_session_state().unwrap();
        let target =
            agent_browser_service_model::resolve_remote_view_tab_handoff(&published, handoff_id)
                .unwrap();
        let digest =
            agent_browser_service_model::remote_view_native_target_digest(handoff_id, &target)
                .unwrap();
        let mappings = connection
            .prepare(
                "SELECT json FROM state_documents WHERE kind LIKE 'handoff_recovery_binding:%'",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let matching = mappings
            .iter()
            .map(|raw| serde_json::from_str::<Value>(raw).unwrap())
            .filter(|receipt| receipt["handoffId"] == handoff_id)
            .collect::<Vec<_>>();
        assert_eq!(
            matching.len(),
            1,
            "{case}: replacement mapping must be durably published before adverse mutation"
        );
        assert_eq!(
            matching[0]["previousTargetDigest"],
            original_digest.as_deref().unwrap(),
            "{case}"
        );
        assert_eq!(matching[0]["targetDigest"], digest, "{case}");
        assert_eq!(
            matching[0]["recovery"]["retirementId"], retirement.intent.id,
            "{case}"
        );
        if case == "missing_prior" || case == "ambiguous_launch" {
            let mut ledger = read_document("browser_launch_custody");
            let records = ledger["records"].as_object_mut().unwrap();
            if case == "missing_prior" {
                records
                    .retain(|_, record| record["observedPid"] != before.browsers[browser_id].pid);
            } else {
                let mut record = records
                    .values()
                    .find(|record| record["observedPid"] != before.browsers[browser_id].pid)
                    .unwrap()
                    .clone();
                let id = uuid::Uuid::new_v4().to_string();
                record["intent"]["intentId"] = id.clone().into();
                records.insert(id, record);
            }
            connection
                .execute(
                    "UPDATE state_documents SET json=?1 WHERE kind='browser_launch_custody'",
                    [ledger.to_string()],
                )
                .unwrap();
        }
        if case == "nonterminal_retirement" {
            let mut ledger = read_document("browser_idle_retirement_custody");
            for record in ledger["records"].as_object_mut().unwrap().values_mut() {
                record["completed"] = Value::Null;
                record["publishedStateDigest"] = Value::Null;
            }
            connection.execute("UPDATE state_documents SET json=?1 WHERE kind='browser_idle_retirement_custody'", [ledger.to_string()]).unwrap();
        }
        let stale = fixture.store(false).load_session_state().unwrap();
        if case == "legacy_mapping" {
            connection
                .execute(
                    "DELETE FROM state_documents WHERE kind LIKE 'handoff_recovery_binding:%'",
                    [],
                )
                .unwrap();
        }
        if case == "changed_tab" || case == "target_swap" || case == "stale_snapshot" {
            let mut next = stale.clone();
            let tab = next
                .tabs
                .get_mut(identity["tabId"].as_str().unwrap())
                .unwrap();
            if case == "changed_tab" {
                tab.created_at_ms += 1;
            } else if case == "target_swap" {
                tab.target_id = "unrelated-target".into();
            } else {
                tab.last_activity_at_ms += 1;
            }
            fixture
                .store(false)
                .compare_and_save_session_state(&stale, &next)
                .unwrap();
        }
        let control_path = crate::native::browser_host_control::path(browser_id).unwrap();
        if case == "stale_control" || case == "foreign_hold" {
            let mut control: Value =
                serde_json::from_slice(&std::fs::read(&control_path).unwrap()).unwrap();
            control["epoch"] = (control["epoch"].as_u64().unwrap() + 1).into();
            if case == "foreign_hold" {
                let epoch = control["epoch"].clone();
                control["audit"].as_array_mut().unwrap().push(serde_json::json!({"epoch":epoch,"operation":"human_handoff","humanHandoff":{"id":"foreign","targetDigest":"a".repeat(64),"epoch":epoch,"returnedBy":null}}));
            }
            std::fs::write(&control_path, serde_json::to_vec(&control).unwrap()).unwrap();
        }
        if case == "original_live" {
            let crate::process_identity::ProcessObservation::Observed(observed) =
                crate::process_identity::observe_process(std::process::id())
            else {
                panic!("fixture process must be observable");
            };
            let live_identity = RecordedProcessIdentity {
                pid: observed.pid,
                start_token: observed.start_token.unwrap(),
                executable_path: observed.executable_path,
                browser_family: None,
            };
            let mut retirements = read_document("browser_idle_retirement_custody");
            let record = retirements["records"]
                .as_object_mut()
                .unwrap()
                .values_mut()
                .next()
                .unwrap();
            let scope = &mut record["intent"]["scopes"][0];
            scope["browser"]["pid"] = live_identity.pid.into();
            scope["processIdentity"] = serde_json::to_value(&live_identity).unwrap();
            let original_target = agent_browser_service_model::RemoteViewTabHandoffTarget {
                session: stale.sessions[identity["sessionId"].as_str().unwrap()].clone(),
                tab: serde_json::from_value(
                    scope["tabs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|tab| tab["id"] == identity["tabId"])
                        .unwrap()
                        .clone(),
                )
                .unwrap(),
                browser: serde_json::from_value(scope["browser"].clone()).unwrap(),
            };
            let previous_digest = agent_browser_service_model::remote_view_native_target_digest(
                handoff_id,
                &original_target,
            )
            .unwrap();
            connection.execute("UPDATE state_documents SET json=?1 WHERE kind='browser_idle_retirement_custody'", [retirements.to_string()]).unwrap();
            let mut launches = read_document("browser_launch_custody");
            let prior = launches["records"]
                .as_object_mut()
                .unwrap()
                .values_mut()
                .find(|record| record["observedPid"] == before.browsers[browser_id].pid)
                .unwrap();
            prior["observedPid"] = live_identity.pid.into();
            prior["observedProcessIdentity"] = serde_json::to_value(&live_identity).unwrap();
            connection
                .execute(
                    "UPDATE state_documents SET json=?1 WHERE kind='browser_launch_custody'",
                    [launches.to_string()],
                )
                .unwrap();
            let mut control: Value =
                serde_json::from_slice(&std::fs::read(&control_path).unwrap()).unwrap();
            for event in control["audit"].as_array_mut().unwrap() {
                if event["humanHandoff"]["id"] == handoff_id {
                    event["humanHandoff"]["targetDigest"] = previous_digest.clone().into();
                }
            }
            std::fs::write(&control_path, serde_json::to_vec(&control).unwrap()).unwrap();
            let rows = connection.prepare("SELECT kind,json FROM state_documents WHERE kind LIKE 'handoff_recovery_binding:%'").unwrap().query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
            for (kind, raw) in rows {
                let mut receipt: Value = serde_json::from_str(&raw).unwrap();
                if receipt["handoffId"] == handoff_id {
                    receipt["previousTargetDigest"] = previous_digest.clone().into();
                    connection
                        .execute(
                            "UPDATE state_documents SET json=?1 WHERE kind=?2",
                            [receipt.to_string(), kind],
                        )
                        .unwrap();
                }
            }
        }
        let expected = fixture.store(false).load_session_state().unwrap();
        let control_before = std::fs::read(&control_path).unwrap();
        let ledger_before = read_document("browser_launch_custody");
        let error = crate::native::browser_host_control::reconcile_recovered_handoff(
            &database,
            if case == "stale_snapshot" {
                &stale
            } else {
                &expected
            },
            handoff_id,
            300_102,
        )
        .unwrap_err();
        let expected_error = match case {
            "target_swap" | "legacy_mapping" => "browser_handoff_recovery_mapping_unrecorded",
            "stale_control" | "foreign_hold" => "browser_handoff_control_changed",
            "stale_snapshot" => "browser_handoff_recovery_publication_conflict",
            _ => "browser_handoff_recovery_chain_unconfirmed",
        };
        assert_eq!(error, expected_error, "{case}");
        assert_eq!(
            std::fs::read(&control_path).unwrap(),
            control_before,
            "{case}"
        );
        assert_eq!(
            fixture.store(false).load_session_state().unwrap(),
            expected,
            "{case}"
        );
        assert_eq!(
            read_document("browser_launch_custody"),
            ledger_before,
            "{case}"
        );
        assert_eq!(calls.get(), 2, "{case}: no additional launch");
        return;
    }
    let resumed = restarted.handle_command(&serde_json::json!({
        "action":"service_remote_view_handoff_resolve", "handoffId":handoff_id,
        "nativeDesktop":true, "activityAtMs":300_102
    }));
    assert_eq!(
        resumed["success"], true,
        "same retained handoff must recover: {resumed}"
    );
    assert_eq!(resumed["data"]["operatorVisible"]["state"], "ready");
    assert!(restarted
        .state()
        .sessions
        .contains_key(identity["sessionId"].as_str().unwrap()));
    assert_eq!(resumed["data"]["browserId"], identity["browserId"]);
    assert_eq!(resumed["data"]["tabId"], identity["tabId"]);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("retirement-effect-count")).unwrap(),
        "1"
    );
    assert_eq!(
        calls.get(),
        2,
        "one original launch plus one qualified recovery"
    );
    let completed = fixture
        .store(false)
        .idle_retirement_records()
        .unwrap()
        .remove(0);
    assert_eq!(completed.intent, retirement.intent);
    assert!(completed.is_terminal() && completed.published_state_digest.is_some());
    if interactive {
        let target = agent_browser_service_model::resolve_remote_view_tab_handoff(
            restarted.state(),
            handoff_id,
        )
        .unwrap();
        let digest =
            agent_browser_service_model::remote_view_native_target_digest(handoff_id, &target)
                .unwrap();
        assert_eq!(
            crate::native::browser_host_control::handoff_intent(browser_id, handoff_id, &digest)
                .unwrap(),
            "interactive",
            "qualified recovery must retain the original interactive invitation"
        );
        let control = crate::native::browser_host_control::metadata(browser_id).unwrap();
        assert_eq!(control["owner"], "human");
        assert_eq!(control["epoch"], 1);
        assert_eq!(control["pauseReads"], true);
    }
}
