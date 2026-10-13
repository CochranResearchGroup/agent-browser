/// Opt-in source-service focus effects on two independently owned X11 desktops.
/// HTTP authentication is supplied as a synthetic authenticated operator here;
/// this does not qualify an installed HTTP server or native viewer rendering.
#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "requires the isolated P237 service-selection driver"]
async fn physical_service_selection_preserves_distinct_desktops() {
    use crate::native::action_runtime::runtime::DaemonState;
    use crate::native::browser::BrowserManager;
    use crate::native::cdp::chrome::LaunchOptions;
    use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};

    fn native_observation(display: &str) -> Result<Value, String> {
        let run = |args: &[&str]| -> Result<String, String> {
            let output = std::process::Command::new(args[0])
                .args(&args[1..])
                .env("DISPLAY", display)
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).into_owned());
            }
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        };
        let active = run(&["xdotool", "getactivewindow"])?;
        Ok(json!({"display":display,"activeWindow":active,
            "inputWindow":run(&["xdotool","getwindowfocus"])?,
            "title":run(&["xdotool","getwindowname",&active])?,
            "properties":run(&["xprop","-id",&active,"_NET_WM_STATE","_NET_WM_PID"])?,
            "stacking":run(&["xprop","-root","_NET_CLIENT_LIST_STACKING"])?}))
    }

    let root = std::path::PathBuf::from(std::env::var("P237_NATIVE_SELECTION_ROOT").unwrap());
    let displays = [
        std::env::var("DISPLAY").unwrap(),
        std::env::var("P237_NATIVE_SELECTION_PEER_DISPLAY").unwrap(),
    ];
    assert_ne!(displays[0], displays[1]);
    let executable = std::env::var("P237_NATIVE_SELECTION_CHROME").unwrap();
    let auth = root.join("operator-auth.json");
    // Dedicated process and private home: exercise real synthetic login.
    std::env::set_var("AGENT_BROWSER_DASHBOARD_AUTH_FILE", &auth);
    std::env::set_var("AGENT_BROWSER_DASHBOARD_AUTH_DIR", &root);
    let credentials = dashboard_auth::ensure_dashboard_auth_config()
        .unwrap()
        .bootstrap_credential_file;
    let retained = std::env::var_os("P237_NATIVE_SELECTION_RETAINED").is_some();
    let native_dispatch = std::env::var_os("P237_NATIVE_SELECTION_DISPATCH").is_some();
    assert!(
        !(retained && native_dispatch),
        "retained and native-dispatch modes qualify separate boundaries"
    );
    let mut launch_owners = Vec::new();
    let mut daemons = Vec::new();
    let mut targets = Vec::new();
    let mut service = ServiceState::default();
    for (index, display) in displays.iter().enumerate() {
        let profile = root.join(format!("service-browser-{index}"));
        let mut manager = BrowserManager::launch(
            LaunchOptions {
                headless: false,
                executable_path: Some(executable.clone()),
                profile: Some(profile.to_string_lossy().into_owned()),
                display: Some(display.clone()),
                args: vec![
                    "--no-sandbox".into(),
                    "--no-proxy-server".into(),
                    "--disable-background-networking".into(),
                ],
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();
        let pid = manager.browser_pid().unwrap();
        let mut browser_targets = Vec::new();
        for tab_index in 0..2 {
            if tab_index != 0 {
                manager.tab_new(None).await.unwrap();
            }
            let target = manager.active_target_id().unwrap().to_owned();
            let title = format!("P237 desktop {index} tab {tab_index}");
            manager
                .evaluate(
                    &format!("document.title={}", serde_json::to_string(&title).unwrap()),
                    None,
                )
                .await
                .unwrap();
            browser_targets.push((target.clone(), title));
            service.tabs.insert(format!("target:{target}"), serde_json::from_value(json!({
                "id":format!("target:{target}"),"browserId":format!("browser-{index}"),"targetId":target,
                "sessionId":format!("session-{index}"),"ownerSessionId":format!("session-{index}"),"lifecycle":"ready"
            })).unwrap());
        }
        if retained {
            // Provider normally prepares window bounds before native-only
            // focus. Seed those bounds here without attributing them to the
            // native-only command under test.
            manager.focus_for_view(true).await.unwrap();
        }
        service.profiles.insert(
            format!("profile-{index}"),
            serde_json::from_value(json!({"id":format!("profile-{index}"),"userDataDir":profile}))
                .unwrap(),
        );
        service.browsers.insert(format!("browser-{index}"),serde_json::from_value(json!({
            "id":format!("browser-{index}"),"profileId":format!("profile-{index}"),"health":"ready","pid":pid,
            "activeSessionIds":[format!("session-{index}")],"displayAllocationId":format!("display-{index}"),
            "cdpEndpoint":manager.get_cdp_url()
        })).unwrap());
        service.remote_view_routes.insert(format!("route-{index}"),serde_json::from_value(json!({
            "id":format!("route-{index}"),"browserId":format!("browser-{index}"),"sessionId":format!("session-{index}"),
            "displayAllocationId":format!("display-{index}"),"state":"ready","readOnly":false,
            "controllerLeaseId":format!("controller-{index}"),"controllerEpoch":1,"viewerLeaseIds":[format!("controller-{index}")]
        })).unwrap());
        service.viewer_leases.insert(format!("controller-{index}"),serde_json::from_value(json!({
            "id":format!("controller-{index}"),"routeId":format!("route-{index}"),"browserId":format!("browser-{index}"),
            "viewerId":"dashboard:admin","viewerRole":"controller","state":"controlling","expiresAt":"2099-01-01T00:00:00Z"
        })).unwrap());
        let mut daemon =
            DaemonState::new_for_session_with_stream(&format!("session-{index}"), None, None);
        if retained {
            launch_owners.push(manager);
        } else {
            daemon.browser = Some(manager);
        }
        daemons.push(daemon);
        targets.push(browser_targets);
    }
    std::fs::write(root.join("browser-identity.json"),serde_json::to_vec(&json!({
        "browsers":daemons.iter().enumerate().map(|(index,_daemon)|json!({"pid":service.browsers[&format!("browser-{index}")].pid,"display":displays[index]})).collect::<Vec<_>>()
    })).unwrap()).unwrap();
    let repository = LockedServiceStateRepository::default_json().unwrap();
    repository
        .mutate(|state| {
            *state = service.clone();
            Ok(())
        })
        .unwrap();
    if retained || native_dispatch {
        use crate::native::runtime_lifecycle::{digest_json, RuntimeLifecycleAuthority};
        use crate::runtime_owner_transfer::{ProfileOwner, ProfileOwnerState};
        let managers: Vec<&BrowserManager> = if retained {
            launch_owners.iter().collect()
        } else {
            daemons
                .iter()
                .map(|state| state.browser.as_ref().unwrap())
                .collect()
        };
        for (index, manager) in managers.iter().enumerate() {
            let profile = manager.browser_user_data_dir().unwrap();
            let process = crate::process_identity::capture_process_identity(
                manager.browser_pid().unwrap(),
                None,
                None,
            )
            .unwrap();
            RuntimeLifecycleAuthority::new(&repository)
                .register_current_owner(ProfileOwner {
                    owner_id: format!("fixture-owner-{index}"),
                    profile_identity_digest:
                        agent_browser_lease_authority::canonical_profile_identity_digest(profile)
                            .unwrap(),
                    state: ProfileOwnerState::Ready,
                    owner_generation: 1,
                    browser_id: format!("browser-{index}"),
                    daemon_session_route: format!("session-{index}"),
                    process_instance_digest: digest_json(&process).unwrap(),
                    browser_family: "chrome".into(),
                    cdp_endpoint_identity_digest: format!(
                        "{:x}",
                        Sha256::digest(manager.get_cdp_url().as_bytes())
                    ),
                    target_set_digest: digest_json(
                        &targets[index]
                            .iter()
                            .map(|(target, _)| target)
                            .collect::<Vec<_>>(),
                    )
                    .unwrap(),
                    pending_transfer: None,
                    last_transition: None,
                })
                .unwrap();
        }
    }
    if native_dispatch {
        for daemon in &mut daemons {
            daemon.runtime_owner_binding =
                crate::runtime_owner_transfer::owner_binding_for_session(
                    &repository,
                    &daemon.session_id,
                )
                .unwrap();
        }
    }
    let daemons = Arc::new(tokio::sync::Mutex::new(daemons));
    let http = if std::env::var_os("P237_NATIVE_SELECTION_HTTP").is_some() {
        Some(P237SelectionHttp::start(daemons.clone(), &credentials).await)
    } else {
        None
    };
    let run = async {
        for (index, tab_index) in [(0,0),(1,0),(0,1),(1,1),(0,0)] {
            let peer = 1-index;
            let before_peer = native_observation(&displays[peer])?;
            let (target,title) = &targets[index][tab_index];
            if retained {
                let mut guard = daemons.lock().await;
                if let Some(manager) = guard[index].browser.as_mut() { manager.relinquish_browser_for_handoff(); }
                crate::native::service_probe::reattach_verified_retained_target(
                    &format!("browser-{index}"), &format!("profile-{index}"), target, &mut guard[index]
                ).await?;
            }
            if retained {
                // Durable handoff asks for native-only window operation. A
                // minimized exact PID window makes missing native effects
                // independently visible instead of checking reply shape alone.
                let pid = service.browsers[&format!("browser-{index}")].pid.unwrap();
                let windows = std::process::Command::new("xdotool").args(["search", "--pid", &pid.to_string(), "--class", "Chrome"]).env("DISPLAY", &displays[index]).output().map_err(|e|e.to_string())?;
                if !windows.status.success() { return Err("retained native probe cannot locate its exact PID window".into()); }
                let window = String::from_utf8(windows.stdout).map_err(|e|e.to_string())?.lines().next().ok_or("retained window missing")?.to_owned();
                let minimized = std::process::Command::new("xdotool").args(["windowminimize", "--sync", &window]).env("DISPLAY", &displays[index]).status().map_err(|e|e.to_string())?;
                if !minimized.success() { return Err("retained native probe could not minimize its exact PID window".into()); }
            }
            let body = json!({"action":"view_focus","serviceName":"fixture","agentName":"admin","taskName":"select","jobTimeoutMs":5000,
                "browserId":format!("browser-{index}"),"sessionName":format!("session-{index}"),
                "params":{"operatorFocus":true,"targetId":target,"routeId":format!("route-{index}"),
                "controllerLeaseId":format!("controller-{index}"),"maximize":true,"nativeFocusOnly":retained}}).to_string();
            let command = service_request_command_with_dashboard_generation(&body,Some(&repository.load_snapshot()?),"admin",&format!("session-{index}"),None).map_err(|e|format!("{e:?}"))?;
            let reply = if let Some(http) = http.as_ref() {
                http.focus(&body).await?
            } else {
                crate::native::browser_lifecycle::handle_view_focus(&command,&mut daemons.lock().await[index]).await?
            };
            let expected_pid = service.browsers[&format!("browser-{index}")].pid.unwrap();
            let deadline = std::time::Instant::now()+Duration::from_secs(3);
            let observation = loop {
                let observed = native_observation(&displays[index])?;
                let properties = observed["properties"].as_str().unwrap();
                let owned = properties.lines().any(|line| line == format!("_NET_WM_PID(CARDINAL) = {expected_pid}"));
                let topmost = observed["activeWindow"].as_str().unwrap().parse::<u64>().ok().is_some_and(|id|
                    observed["stacking"].as_str().unwrap().split(',').next_back().is_some_and(|last| last.trim().ends_with(&format!("0x{id:x}"))));
                if observed["title"].as_str().unwrap().contains(title) && observed["activeWindow"]==observed["inputWindow"]
                    && owned && topmost && properties.contains("_NET_WM_STATE_MAXIMIZED_HORZ") && properties.contains("_NET_WM_STATE_MAXIMIZED_VERT") { break observed; }
                if std::time::Instant::now()>=deadline { return Err(format!("physical service selection failed: expected={title}, observed={observed}, reply={reply}")); }
                tokio::time::sleep(Duration::from_millis(20)).await;
            };
            if !retained && (reply["nativeWindowFocus"]["focused"] != true || reply["nativeWindowFocus"]["display"] != displays[index]) {
                return Err(format!("service native focus used wrong desktop: expected={}, reply={reply}, observed={observation}",displays[index]));
            }
            let after_peer = native_observation(&displays[peer])?;
            if after_peer != before_peer { return Err(format!("service focus changed peer desktop: before={before_peer}, after={after_peer}")); }
            println!("physical-service-selection: {}",json!({"observation":observation,"reply":reply,"peer":after_peer}));
        }
        if let Some(runner) = std::env::var_os("P237_NATIVE_SELECTION_UI_RUNNER") {
            if !native_dispatch { return Err("UI selection fixture requires actual native dispatch".into()); }
            let http = http.as_ref().ok_or("UI selection fixture requires HTTP")?;
            let config = root.join("ui-selection-config.json");
            std::fs::write(&config, json!({
                "origin":format!("http://{}",http.address),"cookie":http.cookie,
                "chrome":executable,"root":root,
                "bundle":std::env::var("P237_NATIVE_SELECTION_UI_BUNDLE").map_err(|e|e.to_string())?,
                "browserId":"browser-0","sessionName":"session-0",
                "routeId":"route-0","controllerLeaseId":"controller-0",
                "displayAllocationId":service.browsers["browser-0"].display_allocation_id,
                "tabs":targets[0],
            }).to_string()).map_err(|e|e.to_string())?;
            let before_peer = native_observation(&displays[1])?;
            let output = tokio::time::timeout(Duration::from_secs(25),
                tokio::process::Command::new("node").kill_on_drop(true).arg(runner).arg(&config).output())
                .await.map_err(|_|"UI selection runner exceeded its fixture deadline")?
                .map_err(|e|e.to_string())?;
            println!("physical-ui-selection-driver: {}",String::from_utf8_lossy(&output.stdout));
            if !output.status.success() {
                return Err(format!("UI selection setup/transport failed: {}",String::from_utf8_lossy(&output.stderr)));
            }
            let observation = native_observation(&displays[0])?;
            let expected_title = &targets[0][1].1;
            let after_peer = native_observation(&displays[1])?;
            if after_peer != before_peer { return Err(format!("UI selection changed peer desktop: before={before_peer}, after={after_peer}")); }
            println!("physical-ui-selection-final: {}",json!({"expectedTitle":expected_title,"observation":observation,"peer":after_peer}));
            if !observation["title"].as_str().unwrap().contains(expected_title) {
                return Err(format!("latest UI selection lost: expected={expected_title}, observed={observation}"));
            }
            let properties = observation["properties"].as_str().unwrap();
            let expected_pid = service.browsers["browser-0"].pid.unwrap();
            let owned = properties.lines().any(|line|line == format!("_NET_WM_PID(CARDINAL) = {expected_pid}"));
            let topmost = observation["activeWindow"].as_str().unwrap().parse::<u64>().ok().is_some_and(|id|
                observation["stacking"].as_str().unwrap().split(',').next_back().is_some_and(|last|last.trim().ends_with(&format!("0x{id:x}"))));
            if !owned || !topmost || observation["activeWindow"] != observation["inputWindow"]
                || !properties.contains("_NET_WM_STATE_MAXIMIZED_HORZ") || !properties.contains("_NET_WM_STATE_MAXIMIZED_VERT") {
                return Err(format!("latest UI selection lacks exact physical focus: {observation}"));
            }
        }
        // An already-issued proof must fail at daemon execution after a
        // controller-generation change, without switching either desktop.
        let target = &targets[0][1].0;
        let body = json!({"action":"view_focus","serviceName":"fixture","agentName":"admin","taskName":"stale-select","jobTimeoutMs":5000,
            "browserId":"browser-0","sessionName":"session-0","params":{"operatorFocus":true,"targetId":target,
            "routeId":"route-0","controllerLeaseId":"controller-0","maximize":true}}).to_string();
        let command = service_request_command_with_dashboard_generation(&body,Some(&repository.load_snapshot()?),"admin","session-0",None).map_err(|e|format!("{e:?}"))?;
        let before = [native_observation(&displays[0])?,native_observation(&displays[1])?];
        let rejected = if let Some(http) = http.as_ref() {
            http.advance_controller_before_execution.store(true, std::sync::atomic::Ordering::SeqCst);
            let result = http.focus(&body).await;
            if http.advance_controller_before_execution.load(std::sync::atomic::Ordering::SeqCst)
                || repository.load_snapshot()?.remote_view_routes["route-0"].controller_epoch != 2 {
                return Err("HTTP stale-generation probe did not reach its native execution boundary".into());
            }
            result
        } else {
            repository.mutate(|state| {state.remote_view_routes.get_mut("route-0").unwrap().controller_epoch += 1; Ok(())})?;
            crate::native::browser_lifecycle::handle_view_focus(&command,&mut daemons.lock().await[0]).await
        };
        if !rejected.as_ref().err().is_some_and(|error|error.contains("operator_focus_authority_unproven")) {
            return Err(format!("stale controller generation did not return the expected authority rejection: {rejected:?}"));
        }
        let after = [native_observation(&displays[0])?,native_observation(&displays[1])?];
        if before != after { return Err(format!("rejected stale focus changed native desktops: before={before:?}, after={after:?}")); }
        for (index,daemon) in daemons.lock().await.iter().enumerate() {
            let observer;
            let client = if native_dispatch {
                observer = agent_browser_cdp::client::CdpClient::connect(service.browsers[&format!("browser-{index}")].cdp_endpoint.as_deref().unwrap()).await?;
                &observer
            } else { daemon.browser.as_ref().unwrap().client.as_ref() };
            let inventory = client.send_command("Target.getTargets",None,None).await?;
            if !targets[index].iter().all(|(target,_)|inventory["targetInfos"].as_array().unwrap().iter().any(|entry| entry["targetId"].as_str()==Some(target.as_str()))) {
                return Err(format!("physical service selection removed a peer target on desktop {index}"));
            }
        }
        println!("physical-service-rejected-stale: {}",json!({"error":rejected.unwrap_err(),"desktops":after,"targetsPreserved":true}));
        Ok::<(),String>(())
    }.await;
    if let Err(error) = &run {
        std::fs::write(
            root.join("physical-failure.json"),
            json!({"error":error}).to_string(),
        )
        .unwrap();
    }
    if native_dispatch {
        http.as_ref().unwrap().close_dispatchers().await;
    }
    for daemon in daemons.lock().await.iter_mut() {
        if let Some(manager) = daemon.browser.as_mut() {
            if retained {
                manager.relinquish_browser_for_handoff();
            } else {
                manager.close().await.unwrap();
            }
        }
    }
    for manager in &mut launch_owners {
        manager.close().await.unwrap();
    }
    run.unwrap();
}
