#[test]
fn profile_library_real_http_auth_write_read_and_no_launch() {
    const CHILD: &str = "P300_PROFILE_LIBRARY_HTTP_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let root = std::env::temp_dir().join(format!("p300-library-http-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let config = root.join("config.json");
        std::fs::write(&config, "{}").unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["native::stream::http::tests::profile_library_real_http_auth_write_read_and_no_launch", "--exact", "--nocapture"])
            .env(CHILD,"1").env("HOME",&root).env("AGENT_BROWSER_HOME",root.join(".agent-browser"))
            .env("AGENT_BROWSER_CONFIG",&config).env("AGENT_BROWSER_DASHBOARD_AUTH_DIR",&root)
            .env("AGENT_BROWSER_DASHBOARD_AUTH_FILE",root.join("dashboard-auth.json"))
            .env("AGENT_BROWSER_SOCKET_DIR",root.join("sockets"))
            .env("AGENT_BROWSER_EXECUTABLE_PATH",root.join("browser-must-not-launch")).status().unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            status.success(),
            "isolated real HTTP profile library failed"
        );
        return;
    }
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        async fn exchange(method:&str,cookie:&str,body:Value)->(String,Value) {
            exchange_at(method,"/api/dashboard/profile-library",cookie,body).await
        }
        async fn exchange_at(method:&str,path:&str,cookie:&str,body:Value)->(String,Value) {
            use tokio::io::{AsyncReadExt,AsyncWriteExt};
            let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address=listener.local_addr().unwrap();
            let server=tokio::spawn(async move {
                let (stream,_)=listener.accept().await.unwrap();
                let mut peeked=vec![0;16384];let count=stream.peek(&mut peeked).await.unwrap();
                handle_http_request(stream,&peeked[..count],&Arc::new(RwLock::new(vec![])),&Arc::new(RwLock::new("chrome".into())),"p300-no-browser").await;
            });
            let body=body.to_string();
            let mut client=tokio::net::TcpStream::connect(address).await.unwrap();
            client.write_all(format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nCookie: {cookie}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            let mut bytes=vec![];tokio::time::timeout(std::time::Duration::from_secs(10),client.read_to_end(&mut bytes)).await.unwrap().unwrap();server.await.unwrap();
            let raw=String::from_utf8(bytes).unwrap();let (head,body)=raw.split_once("\r\n\r\n").unwrap();
            (head.lines().next().unwrap().into(),serde_json::from_str(body).unwrap_or(Value::Null))
        }
        let auth=dashboard_auth::ensure_dashboard_auth_config().unwrap();
        let credentials=std::fs::read_to_string(&auth.bootstrap_credential_file).unwrap();
        let password=credentials.lines().find_map(|line|line.strip_prefix("AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD=\"").and_then(|s|s.strip_suffix('"'))).unwrap();
        let auth_path=std::env::var("AGENT_BROWSER_DASHBOARD_AUTH_FILE").unwrap();
        let mut store:Value=serde_json::from_slice(&std::fs::read(&auth_path).unwrap()).unwrap();
        let mut operator=store["users"][0].clone();operator["username"]=json!("operator");operator["displayName"]=json!("Operator");operator["bootstrap"]=json!(false);store["users"].as_array_mut().unwrap().push(operator);std::fs::write(&auth_path,serde_json::to_vec(&store).unwrap()).unwrap();
        let login=dashboard_auth::login_response(&[],&json!({"username":"operator","password":password}).to_string(),false);
        let login=String::from_utf8(login.into_http_bytes()).unwrap();
        let cookie=login.lines().find_map(|line|line.strip_prefix("Set-Cookie: ").and_then(|s|s.split(';').next())).unwrap();
        assert!(exchange("GET","",Value::Null).await.0.contains("401"));
        assert!(exchange_at("POST", "/api/command", "", json!({"action":"workspace_profile_library","operation":"replace_associations","profileId":"work","servicePrincipalId":"forged"})).await.0.contains("401"));
        let repository=crate::native::service_store::LockedServiceStateRepository::default_json().unwrap();
        use crate::native::service_store::ServiceStateRepository;
        repository.mutate(|state| {
            state.site_policies.insert("site".into(),SitePolicy{id:"site".into(),..Default::default()});
            state.profiles.insert("work".into(),BrowserProfile{id:"work".into(),name:"Work".into(),..Default::default()});
            let mut private=BrowserProfile{id:"private".into(),name:"Hidden".into(),..Default::default()};
            let mut policy=agent_browser_service_model::ServiceProfileAccessPolicy::shared_local_default("private");
            policy.mode=agent_browser_service_model::ProfileAccessMode::Exclusive;policy.default_permissions.clear();
            policy.grants.push(agent_browser_service_model::ProfileAccessGrant{subject_id:"forged".into(),minimum_assurance:agent_browser_service_model::ProfileIdentityAssurance::RegisteredCapability,permissions:agent_browser_service_model::ProfileAccessPreset::Administrator.permissions()});private.access_policy=Some(policy);
            state.profiles.insert("private".into(),private);Ok(())
        }).unwrap();
        let (status,list)=exchange("GET",cookie,Value::Null).await;assert!(status.contains("200"));assert_eq!(list["data"]["profiles"].as_array().unwrap().len(),1);
        let write=json!({"operation":"replace_associations","profileId":"work","associations":[{"sitePolicyId":"site","account":"a","organization":"org"}],"expectedAssociations":[],"servicePrincipalId":"forged","identityAssurance":"operator","actor":{"subjectId":"forged"}});
        assert!(exchange("POST",cookie,write.clone()).await.0.contains("200"));
        let (_,list)=exchange("GET",cookie,Value::Null).await;assert_eq!(list["data"]["profiles"][0]["profile"]["siteAssociations"][0]["organization"],"org");
        assert!(exchange("POST",cookie,write.clone()).await.0.contains("409"));
        let (_,choice)=exchange("POST",cookie,json!({"operation":"select","sitePolicyId":"site","account":"a","organization":"org"})).await;assert_eq!(choice["data"]["profileId"],"work");
        let (_,context_free)=exchange("POST",cookie,json!({"operation":"select","profileId":"work"})).await;assert_eq!(context_free["data"]["profileId"],"work");
        let denied=exchange("POST",cookie,json!({"operation":"select","sitePolicyId":"site","profileId":"private","servicePrincipalId":"forged"})).await.1;assert_eq!(denied["data"]["profileId"],Value::Null);
        for id in ["private", "missing"] {
            let (status, denied) = exchange("POST", cookie, json!({"operation":"replace_associations","profileId":id,"associations":[],"expectedAssociations":[],"servicePrincipalId":"forged","servicePrincipalProvenance":"registered_capability"})).await;
            assert!(status.contains("403"));
            assert_eq!(denied["error"], "Profile unavailable or permission denied");
        }
        assert!(exchange("POST", "", json!({"operation":"replace_associations","profileId":"work","associations":[],"expectedAssociations":[]})).await.0.contains("401"));
        let mut store:Value=serde_json::from_slice(&std::fs::read(&auth_path).unwrap()).unwrap();
        store["users"].as_array_mut().unwrap().iter_mut().find(|user|user["username"]=="operator").unwrap()["role"]=json!("observer");std::fs::write(&auth_path,serde_json::to_vec(&store).unwrap()).unwrap();
        assert!(exchange("POST",cookie,write.clone()).await.0.contains("401"),"role change invalidates the old session");
        let observer=String::from_utf8(dashboard_auth::login_response(&[],&json!({"username":"operator","password":password}).to_string(),false).into_http_bytes()).unwrap();
        let observer_cookie=observer.lines().find_map(|line|line.strip_prefix("Set-Cookie: ").and_then(|s|s.split(';').next())).unwrap();
        assert!(exchange("GET",observer_cookie,Value::Null).await.0.contains("200"));
        assert!(exchange("POST",observer_cookie,write).await.0.contains("403"));
        assert!(crate::native::actions::action_skips_browser_launch("workspace_profile_library"));
        let result=execute_command(&json!({"id":"p300","action":"workspace_profile_library","operation":"list"}),&mut DaemonState::new()).await;assert_eq!(result["success"],true);
        let public=json!({"action":"workspace_profile_library","serviceName":"Profiles","agentName":"fixture","taskName":"Select","params":{"operation":"select","sitePolicyId":"site","profileId":"private","servicePrincipalId":"forged","servicePrincipalProvenance":"registered_capability","actor":{"assurance":"operator"}}});
        let normalized=normalize_service_request(ServiceRequestNormalization{request:&public,service_state:None,authenticated_principal:None,fallback_principal:None,request_id:"p300-mcp",effective_session:Some("p300-no-browser")}).unwrap();
        assert!(normalized.command.get("servicePrincipalId").is_none());
        let dispatched=execute_command(&normalized.command,&mut DaemonState::new()).await;assert_eq!(dispatched["success"],true);assert_eq!(dispatched["data"]["profileId"],Value::Null);
        let mutation=json!({"action":"workspace_profile_library","params":{"operation":"replace_associations","profileId":"work","associations":[],"expectedAssociations":[]}}).to_string();
        assert!(service_request_command_with_state_and_principal(&mutation,None,Some("operator"),"p300-no-browser").is_err(),"revoked operator cannot bypass dashboard role through generic service request");
        assert!(repository.load_snapshot().unwrap().browsers.is_empty());
    });
}
