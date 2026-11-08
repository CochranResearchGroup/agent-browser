#[test]
fn siteguides_real_http_auth_classification_and_no_launch() {
    const CHILD: &str = "P300_SITEGUIDES_HTTP_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let root = std::env::temp_dir().join(format!("p300-library-http-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let config = root.join("config.json");
        std::fs::write(&config, "{}").unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["native::stream::http::tests::siteguides_real_http_auth_classification_and_no_launch", "--exact", "--nocapture"])
            .env(CHILD,"1").env("HOME",&root).env("AGENT_BROWSER_HOME",root.join(".agent-browser"))
            .env("AGENT_BROWSER_CONFIG",&config).env("AGENT_BROWSER_DASHBOARD_AUTH_DIR",&root)
            .env("AGENT_BROWSER_DASHBOARD_AUTH_FILE",root.join("dashboard-auth.json"))
            .env("AGENT_BROWSER_SOCKET_DIR",root.join("sockets"))
            .env("AGENT_BROWSER_EXECUTABLE_PATH",root.join("browser-must-not-launch")).status().unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            status.success(),
            "isolated real HTTP SiteGuides failed"
        );
        return;
    }
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        async fn exchange(method:&str,cookie:&str,body:Value)->(String,Value) {
            exchange_at(method,"/api/dashboard/siteguides",cookie,body).await
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
        dashboard_auth::ensure_dashboard_auth_config().unwrap();
        let auth_path=std::env::var("AGENT_BROWSER_DASHBOARD_AUTH_FILE").unwrap();
        let mut store:Value=serde_json::from_slice(&std::fs::read(&auth_path).unwrap()).unwrap();
        let mut operator=store["users"][0].clone();operator["username"]=json!("operator");operator["displayName"]=json!("Operator");operator["bootstrap"]=json!(false);store["users"].as_array_mut().unwrap().push(operator);std::fs::write(&auth_path,serde_json::to_vec(&store).unwrap()).unwrap();
        let ingress=dashboard_auth::test_ingress::TestIngress::start();
        let cookie_owned=ingress.cookie("operator");
        let cookie=cookie_owned.as_str();
        assert!(exchange("GET","",Value::Null).await.0.contains("401"));
        assert!(exchange_at("POST", "/api/command", "", json!({"action":"workspace_profile_library","operation":"replace_associations","profileId":"work","servicePrincipalId":"forged"})).await.0.contains("401"));
        let (status,_) = exchange("GET","",Value::Null).await; assert!(status.contains("401"));
        let (status,_) = exchange("POST",cookie,json!({"operation":"browser_open","actor":{"username":"operator"}})).await; assert!(status.contains("400"));
        let (status,read)=exchange("GET",cookie,Value::Null).await; assert!(status.contains("503")); assert!(read["error"].as_str().unwrap().starts_with("siteguides_configuration_required"));
        let (status,write)=exchange("POST",cookie,json!({"operation":"stage_draft","taskId":"forged","actor":{"username":"operator"},"configPath":"/forged","guide":{}})).await; assert!(status.contains("503")); assert!(write["error"].as_str().unwrap().starts_with("siteguides_configuration_required"));
        // Revoke the current role without changing the session cookie. Existing
        // authenticated reads remain reads; a stale role cannot write teaching.
        let mut store:Value=serde_json::from_slice(&std::fs::read(&auth_path).unwrap()).unwrap();
        store["users"].as_array_mut().unwrap().iter_mut().find(|user|user["username"]=="operator").unwrap()["role"]=json!("observer");
        std::fs::write(&auth_path,serde_json::to_vec(&store).unwrap()).unwrap();
        let (status,_) = exchange("POST",cookie,json!({"operation":"record_decision","servicePrincipalId":"forged","role":"superuser"})).await; assert!(status.contains("403"));
        let (status,_) = exchange("GET",cookie,Value::Null).await; assert!(status.contains("503"));
        assert!(!std::path::Path::new(&std::env::var("AGENT_BROWSER_EXECUTABLE_PATH").unwrap()).exists());
    });
}
