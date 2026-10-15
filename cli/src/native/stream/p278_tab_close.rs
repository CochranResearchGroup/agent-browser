#[test]
fn operator_tab_close_is_explicit_and_requires_one_exact_selector() {
    let body = json!({"action":"tab_close","browserId":"browser:managed-a","sessionName":"task-a",
        "serviceName":"fixture","agentName":"agent","taskName":"Close owned task",
        "params":{"targetId":"target-a","operatorTabCloseProofToken":"forged"}});
    let ordinary = service_request_command_with_state_and_authority(&body.to_string(), None, None, "task-a", None).unwrap();
    assert_eq!(ordinary["action"], "tab_close");
    assert!(ordinary.get("operatorClose").is_none(), "ordinary agent requests retain their existing admission path");
    assert!(ordinary.get("operatorTabCloseProofToken").is_none());
    for (field, value, expected) in [("operatorClose", json!("true"), "invalid_field_type"),
        ("index", json!(0), "invalid_request")] {
        let mut invalid = body.clone(); invalid["params"]["operatorClose"] = json!(true);
        invalid["params"][field] = value;
        let rejection = service_request_command_with_state_and_authority(&invalid.to_string(), None, None, "task-a", None).unwrap_err().response();
        assert_eq!(rejection["failure"]["code"], expected, "{rejection}");
        assert_eq!(rejection["failure"]["effectState"], "no_effect");
    }
}

#[test]
fn operator_tab_close_binds_headless_owner_target_and_current_close_permission() {
    const CHILD: &str = "P278_OPERATOR_TAB_CLOSE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let root = std::env::temp_dir().join(format!("p278-close-proof-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let auth = root.join("dashboard-auth.json");
        std::fs::write(&auth, serde_json::to_vec(&json!({
            "version":1,"createdAt":"fixture","sessionSecret":"ERERERERERERERERERERERERERERERERERERERERERE",
            "users":[{"username":"operator","displayName":"Operator","role":"superuser",
            "passwordHash":"","createdAt":"fixture","bootstrap":false}]
        })).unwrap()).unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["native::stream::http::tests::operator_tab_close_binds_headless_owner_target_and_current_close_permission", "--exact", "--nocapture"])
            .env(CHILD, "1").env("HOME", &root)
            .env("AGENT_BROWSER_HOME", root.join(".agent-browser"))
            .env("AGENT_BROWSER_DASHBOARD_AUTH_FILE", &auth)
            .env("AGENT_BROWSER_DASHBOARD_AUTH_DIR", &root)
            .status().unwrap();
        assert!(status.success(), "isolated close proof fixture failed: {}", root.display());
        std::fs::remove_dir_all(root).unwrap();
        return;
    }
    use agent_browser_service_model::{BrowserProfileCatalog, BrowserSessionState, ProfileAccessGrant,
        ProfileIdentityAssurance, ProfilePermission, ServiceProfileAccessPolicy};
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let mut managed = BrowserSessionState::default();
    managed.browsers.insert("browser:managed-a".into(), serde_json::from_value(json!({
        "id":"browser:managed-a","profileId":"profile-a","pid":100,
        "cdpEndpoint":"http://127.0.0.1:1","activeSessionIds":["session-a"]
    })).unwrap());
    managed.sessions.insert("session-a".into(), serde_json::from_value(json!({
        "id":"session-a","name":"task-a","profileId":"profile-a","browserId":"browser:managed-a",
        "createdAtMs":100,"lastActivityAtMs":100,"expiresAtMs":300000,"currentTabId":"tab-a"
    })).unwrap());
    managed.tabs.insert("tab-a".into(), serde_json::from_value(json!({
        "id":"tab-a","targetId":"target-a","browserId":"browser:managed-a","sessionId":"session-a",
        "createdAtMs":100,"lastActivityAtMs":100
    })).unwrap());
    store.save_session_state(&managed).unwrap();
    let mut catalog = BrowserProfileCatalog::default();
    catalog.profiles.insert("profile-a".into(), serde_json::from_value(json!({
        "id":"profile-a","name":"Profile A","kind":"named","userDataDir":"/synthetic/profile-a"
    })).unwrap());
    store.save_profile_catalog(&catalog).unwrap();
    let mut state: ServiceState = serde_json::from_value(json!({
        "profiles":{"profile-a":{"id":"profile-a"}}
    })).unwrap();
    let mut policy = ServiceProfileAccessPolicy::shared_local_default("profile-a");
    policy.grants.push(ProfileAccessGrant {
        subject_id: "dashboard:operator".into(), minimum_assurance: ProfileIdentityAssurance::AuthenticatedIngress,
        permissions: vec![ProfilePermission::TabCloseAny],
    });
    state.profiles.get_mut("profile-a").unwrap().access_policy = Some(policy);
    let original = serde_json::to_value(&state).unwrap();
    let body = json!({"action":"tab_close","browserId":"browser:managed-a","sessionName":"task-a",
        "serviceName":"fixture","agentName":"caller-label","taskName":"Close selected task",
        "params":{"targetId":"target-a","operatorClose":true,"operatorTabCloseProofToken":"caller-forged-close"}}).to_string();
    let command = service_request_command_with_dashboard_generation(&body, Some(&state), "operator", "task-a", None).unwrap();
    assert_ne!(command["operatorTabCloseProofToken"], "caller-forged-close");
    dashboard_auth::verify_operator_tab_close(&state, &command).unwrap();
    assert_eq!(serde_json::to_value(&state).unwrap(), original);
    assert_eq!(store.load_session_state().unwrap(), managed, "proof issuance must not mutate the owner");
    assert!(dashboard_auth::verify_operator_focus(&state, &command).is_err(), "close authority is not focus authority");
    for (field, value) in [("id", json!("other-request")), ("targetId", json!("other-target")),
        ("sessionName", json!("other-task")), ("operatorTabCloseProofToken", json!("forged"))] {
        let mut changed = command.clone(); changed[field] = value;
        assert!(dashboard_auth::verify_operator_tab_close(&state, &changed).is_err(), "{field}");
    }
    let mut changed_owner = managed.clone();
    changed_owner.browsers.get_mut("browser:managed-a").unwrap().pid = 101;
    store.save_session_state(&changed_owner).unwrap();
    assert!(dashboard_auth::verify_operator_tab_close(&state, &command).is_err());
    store.save_session_state(&managed).unwrap();
    let mut denied = state.clone();
    denied.profiles.get_mut("profile-a").unwrap().access_policy.as_mut().unwrap().grants.clear();
    let rejection = service_request_command_with_dashboard_generation(&body, Some(&denied), "operator", "task-a", None).unwrap_err().response();
    assert_eq!(rejection["failure"]["code"], "operator_tab_close_profile_policy_denied", "{rejection}");
    assert_eq!(rejection["failure"]["missingPermission"], "tab_close_any");
    assert_eq!(rejection["failure"]["effectState"], "no_effect");
    assert!(dashboard_auth::verify_operator_tab_close(&denied, &command).is_err(), "revoked close permission must invalidate prior issuance");
}

/// Opt-in physical composition through the public normalizer and actual owner.
/// A single private session makes failure cleanup use its existing final-owner close.
#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "requires the isolated P278 headless Chrome driver"]
async fn operator_tab_close_real_headless_owner_preserves_peer_and_closes_last_browser() {
    const CHILD: &str = "P278_REAL_TAB_CLOSE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("p278-real-close-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let auth = root.join("dashboard-auth.json");
        std::fs::write(&auth, serde_json::to_vec(&json!({
            "version":1,"createdAt":"fixture","sessionSecret":"ERERERERERERERERERERERERERERERERERERERERERE",
            "users":[{"username":"operator","displayName":"Operator","role":"superuser",
            "passwordHash":"","createdAt":"fixture","bootstrap":false}]
        })).unwrap()).unwrap();
        for name in ["config", "data", "state", "cache", "runtime", "sockets"] {
            let path = root.join(name); std::fs::create_dir_all(&path).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child.args(["native::stream::http::tests::operator_tab_close_real_headless_owner_preserves_peer_and_closes_last_browser",
            "--exact", "--ignored", "--nocapture"])
            .env(CHILD, "1").env("HOME", &root).env("P278_FIXTURE_ROOT", &root)
            .env("AGENT_BROWSER_HOME", root.join(".agent-browser"))
            .env("XDG_CONFIG_HOME", root.join("config")).env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_STATE_HOME", root.join("state")).env("XDG_CACHE_HOME", root.join("cache"))
            .env("XDG_RUNTIME_DIR", root.join("runtime")).env("AGENT_BROWSER_SOCKET_DIR", root.join("sockets"))
            .env("AGENT_BROWSER_DASHBOARD_AUTH_FILE", &auth)
            .env("AGENT_BROWSER_DASHBOARD_AUTH_DIR", &root)
            .env("AGENT_BROWSER_EXECUTABLE_PATH", std::env::var("P278_TEST_CHROME").expect("reviewed Chrome path required"))
            .env("AGENT_BROWSER_ARGS", "--no-sandbox,--no-first-run,--disable-dev-shm-usage")
            .env("AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY", "disabled");
        for name in ["DISPLAY", "XAUTHORITY", "AGENT_BROWSER_SESSION_DISPLAY", "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
            "AGENT_BROWSER_REMOTE_VIEW_APPLICATION", "AGENT_BROWSER_REMOTE_VIEW_POOL", "AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN",
            "AGENT_BROWSER_PROFILE", "AGENT_BROWSER_USER_DATA_DIR", "AGENT_BROWSER_RUNTIME_PROFILE", "AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE"] {
            child.env_remove(name);
        }
        let status = child.status().unwrap();
        // Keep a failed run's private source receipts. Never erase a potentially
        // live profile to manufacture cleanup or acceptance.
        let profile = root.join("owned-profile");
        let mut retained = Vec::new();
        for entry in std::fs::read_dir("/proc").unwrap().flatten() {
            if !entry.file_name().to_string_lossy().bytes().all(|value| value.is_ascii_digit()) { continue; }
            if let Ok(command) = std::fs::read(entry.path().join("cmdline")) {
                if command.windows(profile.as_os_str().as_encoded_bytes().len()).any(|part| part == profile.as_os_str().as_encoded_bytes()) {
                    retained.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
        std::fs::write(root.join("cleanup-census.json"), serde_json::to_vec(&json!({"ownedProcesses":retained})).unwrap()).unwrap();
        assert!(retained.is_empty(), "owned runtime residue retained at {}", root.display());
        assert!(status.success(), "isolated headless close failed; retained {}", root.display());
        eprintln!("P278 physical close receipts: {}", root.display());
        return;
    }
    use agent_browser_service_model::{BrowserProfileCatalog, OpenBrowserSession, ProfileAccessGrant,
        ProfileIdentityAssurance, ProfilePermission, ServiceProfileAccessPolicy, SessionEndReason};
    use crate::native::browser_session_host::{close_shared_operator_tab, load_default_browser_session_host, SharedBrowserSessionHost};
    use crate::native::browser_session_store::BrowserSessionSqliteStore;
    use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};
    let root = std::path::PathBuf::from(std::env::var_os("P278_FIXTURE_ROOT").unwrap());
    let profile = root.join("owned-profile"); std::fs::create_dir_all(&profile).unwrap();
    let store = BrowserSessionSqliteStore::default_sqlite().unwrap();
    let mut catalog = BrowserProfileCatalog::default();
    catalog.profiles.insert("profile-a".into(), serde_json::from_value(json!({
        "id":"profile-a","name":"Private close fixture","kind":"named","userDataDir":profile
    })).unwrap());
    store.save_profile_catalog(&catalog).unwrap();
    let mut policy = ServiceProfileAccessPolicy::shared_local_default("profile-a");
    policy.grants.push(ProfileAccessGrant { subject_id:"dashboard:operator".into(),
        minimum_assurance:ProfileIdentityAssurance::AuthenticatedIngress, permissions:vec![ProfilePermission::TabCloseAny] });
    let state: ServiceState = serde_json::from_value(json!({"profiles":{"profile-a":{"id":"profile-a","accessPolicy":policy}}})).unwrap();
    LockedServiceStateRepository::default_json().unwrap().mutate(|snapshot| { *snapshot = state.clone(); Ok(()) }).unwrap();
    let shared: SharedBrowserSessionHost = Arc::new(std::sync::Mutex::new(Some(load_default_browser_session_host().unwrap())));
    struct Cleanup(SharedBrowserSessionHost);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let mut host = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(host) = host.as_mut() {
                let ids = host.state().sessions.keys().cloned().collect::<Vec<_>>();
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                for id in ids { let _ = host.close_session(&id, SessionEndReason::ExplicitClose, now); }
            }
        }
    }
    let cleanup = Cleanup(shared.clone());
    let (browser, first, second, session_name) = {
        let mut guard = shared.lock().unwrap(); let host = guard.as_mut().unwrap();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        let opened = host.open(OpenBrowserSession::exact_profile("Alice", "profile-a", now)).unwrap();
        let first = host.tab_for_navigation(&opened.session_id, now + 1).unwrap();
        let second = host.new_tab(&opened.session_id, now + 2).unwrap();
        (host.state().browsers[&opened.browser_id].clone(), first, second, "Alice")
    };
    let pid = browser.pid;
    std::fs::write(root.join("owned-root.json"), serde_json::to_vec(&json!({"pid":pid,"profile":profile})).unwrap()).unwrap();
    for (target, expected_current) in [(&first, Some(second.tab_id.as_str())), (&second, None)] {
        let body = json!({"action":"tab_close","browserId":browser.id,"sessionName":session_name,
            "serviceName":"private-close-fixture","agentName":"operator","taskName":"Close exact owned tab",
            "params":{"targetId":target.target_id,"operatorClose":true}}).to_string();
        let command = service_request_command_with_dashboard_generation(&body, Some(&state), "operator", session_name, None).unwrap();
        let result = close_shared_operator_tab(shared.clone(), command).await.unwrap();
        assert_eq!(result["closedTabId"], target.tab_id);
        assert_eq!(result["currentTabId"].as_str(), expected_current);
        let retained = store.load_session_state().unwrap();
        assert!(!retained.tabs.contains_key(&target.tab_id));
        if expected_current.is_some() {
            assert!(retained.tabs.contains_key(&second.tab_id));
            let mut inventory = url::Url::parse(&browser.cdp_endpoint).unwrap();
            inventory.set_scheme("http").unwrap(); inventory.set_path("/json/list"); inventory.set_query(None);
            let pages: Value = reqwest::get(inventory).await.unwrap().json().await.unwrap();
            assert!(pages.as_array().unwrap().iter().any(|page| page["id"] == second.target_id));
            assert!(!pages.as_array().unwrap().iter().any(|page| page["id"] == first.target_id));
        } else {
            assert!(retained.browsers.is_empty());
        }
    }
    drop(cleanup);
    for _ in 0..50 {
        if !Path::new(&format!("/proc/{pid}")).exists() { break; }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!Path::new(&format!("/proc/{pid}")).exists(), "exact owned Chrome root survived final-tab close");
    std::fs::write(root.join("physical-close-receipt.json"), b"{\"exactTargetClosed\":true,\"peerPreserved\":true,\"lastOwnedBrowserClosed\":true}").unwrap();
}
