//! Existing owner stores are the policy boundary; no provider or browser starts.
use super::super::service_store::{JsonServiceStateStore, ServiceStateStore};
use super::*;

#[test]
fn native_challenge_owner_policy_requires_current_profile_origin_and_enabled_bound_provider() {
    let root = std::env::temp_dir().join(format!("native-policy-{}", uuid::Uuid::new_v4()));
    let profile_path = root.join("profile");
    std::fs::create_dir_all(&profile_path).unwrap();
    let guard = crate::test_utils::EnvGuard::new(&[
        "HOME",
        "AGENT_BROWSER_HOME",
        "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
        "AGENT_BROWSER_CONFIG",
    ]);
    guard.set("HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_HOME", root.to_str().unwrap());
    guard.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
    guard.remove("AGENT_BROWSER_CONFIG");
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let epoch = super::super::browser_host_control::read("browser")
        .unwrap()
        .epoch;
    let observation = json!({"desktopId":"desktop","generation":9,"windowId":7,"window":{"x":0,"y":0,"width":320,"height":200},
        "displayWidth":320,"displayHeight":200,"region":{"x":0,"y":0,"width":320,"height":200},"frameSha256":"a".repeat(64),
        "nativeControlEpoch":4,"observedAtMs":now_ms().unwrap(),"registeredActionId":"hcaptcha-checkbox-p181-v2","family":"hcaptcha","candidateSha256":"b".repeat(64),"x":50,"y":50});
    let row = |request: &str, action: &str, state: &str, refs: Value| {
        json!({"requestId":request,"taskId":"task","profileId":"profile","browserId":"browser","tabId":"tab","targetId":"target",
        "processPid":std::process::id(),"processIdentity":null,"controlEpoch":epoch,"action":action,"fingerprint":"synthetic-private-mac","state":state,"createdAtMs":1,"completedAtMs":null,"evidenceReferences":refs})
    };
    let mut value = serde_json::to_value(BrowserSessionState::default()).unwrap();
    value["sessions"] = json!({"task":{"id":"task","name":"task-lane","profileId":"profile","browserId":"browser","createdAtMs":1,"lastActivityAtMs":1,"expiresAtMs":0,"currentTabId":"tab"}});
    value["browsers"] = json!({"browser":{"id":"browser","profileId":"profile","pid":std::process::id(),"cdpEndpoint":"","desktop":null,"activeSessionIds":["task"]}});
    value["tabs"] = json!({"tab":{"id":"tab","targetId":"target","browserId":"browser","sessionId":"task","createdAtMs":1,"lastActivityAtMs":1}});
    value["taskOperations"] = json!({"task":{
        "capture":row("capture","native_capture","owner_acknowledged",json!({"nativeObservation":observation.to_string()})),
        "attempt":row("attempt","native_input","pending",json!({"nativeObservationRequestId":"capture","nativeRegisteredActionId":"hcaptcha-checkbox-p181-v2"}))}});
    let base: BrowserSessionState = serde_json::from_value(value.clone()).unwrap();
    store.save_session_state(&base).unwrap();
    let catalog:agent_browser_service_model::BrowserProfileCatalog=serde_json::from_value(json!({"schemaVersion":"agent-browser.browser-profile-catalog.v1","profiles":{"profile":{"id":"profile","name":"Fixture","userDataDir":profile_path,"kind":"named"}}})).unwrap();
    store.save_profile_catalog(&catalog).unwrap();
    let mut service =
        serde_json::to_value(agent_browser_service_model::ServiceState::default()).unwrap();
    service["profiles"] = json!({"profile":{"id":"profile","name":"Fixture","userDataDir":profile_path,"sitePolicyIds":["owned-site"]}});
    service["sitePolicies"] = json!({"owned-site":{"id":"owned-site","originPattern":"https://owned.example.test","challengePolicy":"provider_allowed","allowedChallengeProviders":["native-provider"]}});
    service["providers"] = json!({"native-provider":{"id":"native-provider","kind":"captcha","enabled":true,"capabilities":["captcha_solve"],"configRef":"agent-browser:remote-view-native:v1"}});
    let service_store = JsonServiceStateStore::new(
        super::super::service_store::default_service_state_path().unwrap(),
    );
    let save = |value: &Value| {
        service_store
            .save(&serde_json::from_value(value.clone()).unwrap())
            .unwrap()
    };
    save(&service);
    let command = json!({"id":"attempt","action":"native_input","sessionId":"task","nativeAttempt":{"observationRequestId":"capture","registeredActionId":"hcaptcha-checkbox-p181-v2"}});
    let mut outcomes = Vec::new();
    for case in [
        "current",
        "wrong-origin",
        "unbound-profile",
        "ambiguous-site",
        "manual-only",
        "cdp-free-required",
        "manual-mode",
        "manual-login",
        "profile-manual",
        "denied-provider",
        "disabled-provider",
        "missing-capability",
        "wrong-binding",
        "ambiguous-provider",
    ] {
        store.save_session_state(&base).unwrap();
        let mut source = service.clone();
        match case {
            "unbound-profile" => source["profiles"]["profile"]["sitePolicyIds"] = json!([]),
            "ambiguous-site" => {
                source["profiles"]["profile"]["sitePolicyIds"] = json!(["owned-site", "other"]);
                source["sitePolicies"]["other"] = source["sitePolicies"]["owned-site"].clone();
                source["sitePolicies"]["other"]["id"] = json!("other");
            }
            "cdp-free-required" => {
                source["sitePolicies"]["owned-site"]["requiresCdpFree"] = json!(true)
            }
            "manual-mode" => {
                source["sitePolicies"]["owned-site"]["interactionMode"] = json!("manual")
            }
            "manual-login" => {
                source["sitePolicies"]["owned-site"]["manualLoginPreferred"] = json!(true)
            }
            "profile-manual" => source["profiles"]["profile"]["manualLoginPreferred"] = json!(true),
            "manual-only" => {
                source["sitePolicies"]["owned-site"]["challengePolicy"] = json!("manual_only")
            }
            "denied-provider" => {
                source["sitePolicies"]["owned-site"]["allowedChallengeProviders"] =
                    json!(["other-provider"])
            }
            "disabled-provider" => source["providers"]["native-provider"]["enabled"] = json!(false),
            "missing-capability" => {
                source["providers"]["native-provider"]["capabilities"] = json!(["human_approval"])
            }
            "wrong-binding" => {
                source["providers"]["native-provider"]["configRef"] = json!("vendor-other")
            }
            "ambiguous-provider" => {
                source["sitePolicies"]["owned-site"]["allowedChallengeProviders"] = json!([]);
                source["providers"]["another"] = source["providers"]["native-provider"].clone();
                source["providers"]["another"]["id"] = json!("another");
            }
            _ => {}
        }
        save(&source);
        outcomes.push((
            case,
            native_site_policy(
                &command,
                if case == "wrong-origin" {
                    "https://other.example.test"
                } else {
                    "https://owned.example.test"
                },
            ),
        ));
    }
    store.save_session_state(&base).unwrap();
    save(&service);
    let binding = native_site_policy(&command, "https://owned.example.test").unwrap();
    let original = base.task_operations["task"]["attempt"].clone();
    retain_native_site_policy(&command, &binding).unwrap();
    let response = crate::connection::Response {
        success: true,
        data: Some(json!({"source":"remote_view_native","delivery":"dispatched"})),
        ..Default::default()
    };
    let acknowledged = finish(&original, Some(&response)).unwrap();
    store.save_session_state(&base).unwrap();
    retain_native_site_policy(&command, &binding).unwrap();
    let uncertain = finish(&original, None).unwrap();
    let mut budget = Vec::new();
    for state_name in ["pending", "uncertain", "owner_acknowledged"] {
        let mut state = value.clone();
        state["taskOperations"]["task"]["attempt"]["state"] = json!(state_name);
        let mut fresh = observation.clone();
        fresh["registeredActionId"] = json!("turnstile-checkbox-p169-v1");
        fresh["family"] = json!("turnstile");
        fresh["observedAtMs"] = json!(now_ms().unwrap());
        state["taskOperations"]["task"]["fresh-capture"] = row(
            "fresh-capture",
            "native_capture",
            "owner_acknowledged",
            json!({"nativeObservation":fresh.to_string()}),
        );
        store
            .save_session_state(&serde_json::from_value(state).unwrap())
            .unwrap();
        let mut request = json!({"id":"different-attempt","action":"native_input","nativeAttempt":{"observationRequestId":"fresh-capture","registeredActionId":"turnstile-checkbox-p169-v1"}});
        budget.push((state_name, begin_native("task", &mut request).err()));
    }
    drop(service_store);
    drop(store);
    fn unseal(path: &std::path::Path) {
        if !path.is_dir() {
            return;
        }
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        for e in std::fs::read_dir(path).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                unseal(&e.path());
            }
        }
    }
    unseal(&root);
    std::fs::remove_dir_all(&root).unwrap();
    for (case, result) in outcomes {
        if matches!(case, "current" | "manual-login" | "profile-manual") {
            let result = result.unwrap();
            assert_eq!(result["providerId"], "native-provider");
            assert_eq!(result["family"], "hcaptcha");
            assert_eq!(result["registeredActionId"], "hcaptcha-checkbox-p181-v2");
        } else {
            assert!(result.is_err(), "{case}:{result:?}");
        }
    }
    assert_eq!(
        acknowledged.state,
        BrowserTaskOperationState::OwnerAcknowledged
    );
    assert_eq!(
        acknowledged.evidence_references["nativeChallengeState"],
        "verification_in_progress"
    );
    assert!(
        acknowledged
            .evidence_references
            .contains_key("nativeSitePolicy"),
        "Completion preserves policy bound after the original pending marker"
    );
    assert_eq!(uncertain.state, BrowserTaskOperationState::Uncertain);
    assert_eq!(
        uncertain.evidence_references["nativeChallengeState"],
        "human_intervention_required"
    );
    for (state, error) in budget {
        assert!(error.unwrap().contains("budget_exhausted"), "{state}");
    }
}
