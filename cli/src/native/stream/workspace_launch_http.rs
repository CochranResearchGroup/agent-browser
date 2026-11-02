//! Authenticated compact-launch adapter. Caller identity fields never enter the
//! owner request; existing profile choice uses the public profile policy owner.
use super::*;

pub(super) async fn handle(
    stream: &mut tokio::net::TcpStream,
    method: &str,
    query: Option<&str>,
    body: &str,
    username: &str,
    headers: &[(String, String)],
    secure_cookie: bool,
) {
    if method != "GET" && method != "POST" {
        write_json_error(
            stream,
            "405 Method Not Allowed",
            "workspace_launch_method_invalid",
        )
        .await;
        return;
    }
    let username = if method == "POST" {
        match dashboard_auth::require_superuser(headers, secure_cookie) {
            Ok(identity) => identity.username,
            Err(response) => {
                let _ = stream.write_all(&response.into_http_bytes()).await;
                return;
            }
        }
    } else {
        username.to_string()
    };
    let principal = format!("dashboard:{}", username.trim());
    let mut handed_to_owner = false;
    let result: Result<Value,String>=async {
        if method=="GET" {
            if let Some(id)=query.and_then(|q|url::form_urlencoded::parse(q.as_bytes()).find(|(k,_)|k=="operationId").map(|(_,v)|v.into_owned())) {
                let username=username.to_string();
                return tokio::task::spawn_blocking(move || crate::native::browser_session_host::workspace_browser_launch::inspect(&username,&id)).await.map_err(|_|"workspace_launch_read_failed".to_string())?;
            }
            let library=crate::native::workspace_profile_library::handle_workspace_profile_library(&json!({"operation":"list","servicePrincipalId":principal,"servicePrincipalProvenance":"authenticated_transport"})).await?;
            return tokio::task::spawn_blocking(move ||crate::native::browser_session_host::workspace_browser_launch::catalog(library)).await.map_err(|_|"workspace_catalog_read_failed".to_string())?;
        }
        let intent:Value=serde_json::from_str(body).map_err(|_|"workspace_launch_body_invalid")?;
        let operation=intent["operationId"].as_str().ok_or("workspace_launch_id_required")?;
        uuid::Uuid::parse_str(operation).map_err(|_|"workspace_launch_id_invalid")?;
        if intent["profileMode"]=="existing" {
            let selected=crate::native::workspace_profile_library::handle_workspace_profile_library(&json!({"operation":"select","profileId":intent["profileId"],"servicePrincipalId":principal,"servicePrincipalProvenance":"authenticated_transport"})).await?;
            if selected["state"]!="selected" || selected["profileId"]!=intent["profileId"] {return Err("workspace_profile_unavailable".into());}
        }
        let cmd=json!({"action":"browser_session_workspace_open","operationId":operation,"operator":username,"intent":intent});
        ensure_service_daemon_session(DASHBOARD_SERVICE_BACKEND_SESSION,Some(&cmd)).await?;
        handed_to_owner=true;
        let response=relay_command_to_daemon(DASHBOARD_SERVICE_BACKEND_SESSION,&cmd.to_string()).await?;
        let response:Value=serde_json::from_str(&response).map_err(|_|"workspace_launch_response_invalid")?;
        if response["success"]!=true {return Err(response["error"].as_str().unwrap_or("workspace_launch_unknown").to_string());}
        Ok(response["data"].clone())
    }.await;
    match result {
        Ok(data) => write_json_value(stream, "200 OK", json!({"success":true,"data":data})).await,
        Err(error) => {
            let outcome = if (method == "POST" && !handed_to_owner)
                || error.starts_with("workspace_launch_refused_no_effect:")
            {
                "refused_no_effect"
            } else {
                "unknown"
            };
            write_json_value(
                stream,
                "409 Conflict",
                json!({"success":false,"error":error,"outcome":outcome}),
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_launcher_http_auth_policy_and_readback_no_launch() {
        const CHILD: &str = "P300_LAUNCH_HTTP_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let root =
                std::env::temp_dir().join(format!("p300-launch-http-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            let config = root.join("config.json");
            std::fs::write(&config, "{}").unwrap();
            let status=std::process::Command::new(std::env::current_exe().unwrap())
                .args(["native::stream::dashboard::workspace_launch_http::tests::workspace_launcher_http_auth_policy_and_readback_no_launch","--exact","--nocapture"])
                .env(CHILD,"1").env("HOME",&root).env("AGENT_BROWSER_HOME",root.join(".agent-browser"))
                .env("AGENT_BROWSER_CONFIG",config).env("AGENT_BROWSER_DASHBOARD_AUTH_DIR",&root)
                .env("AGENT_BROWSER_DASHBOARD_AUTH_FILE",root.join("dashboard-auth.json"))
                .env("AGENT_BROWSER_SOCKET_DIR",root.join("sockets"))
                .env_remove("AGENT_BROWSER_DASHBOARD_INGRESS_VERIFY_URL")
                .env_remove("AGENT_BROWSER_DASHBOARD_INGRESS_WORKSPACE_URL")
                .env_remove("AGENT_BROWSER_DASHBOARD_INGRESS_USERS")
                .env("AGENT_BROWSER_EXECUTABLE_PATH",root.join("must-not-launch"))
                .status().unwrap();
            let _ = std::fs::remove_dir_all(&root);
            assert!(status.success());
            return;
        }
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            async fn exchange(method:&str,path:&str,cookie:&str,body:Value)->(String,Value) {
                use tokio::io::{AsyncReadExt,AsyncWriteExt};
                let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let address=listener.local_addr().unwrap();
                let server=tokio::spawn(async move {let(stream,_)=listener.accept().await.unwrap();handle_dashboard_connection(stream).await;});
                let mut client=tokio::net::TcpStream::connect(address).await.unwrap();let body=body.to_string();
                client.write_all(format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nCookie: {cookie}\r\nRemote-User: admin\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
                let mut bytes=vec![];tokio::time::timeout(std::time::Duration::from_secs(20),client.read_to_end(&mut bytes)).await.unwrap().unwrap();server.await.unwrap();
                let raw=String::from_utf8(bytes).unwrap();let(head,body)=raw.split_once("\r\n\r\n").unwrap();(head.lines().next().unwrap().into(),serde_json::from_str(body).unwrap())
            }
            let auth=dashboard_auth::ensure_dashboard_auth_config().unwrap();
            let credentials=std::fs::read_to_string(auth.bootstrap_credential_file).unwrap();
            let password=credentials.lines().find_map(|line|line.strip_prefix("AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD=\"").and_then(|s|s.strip_suffix('"'))).unwrap();
            let login=String::from_utf8(dashboard_auth::login_response(&[],&json!({"username":"admin","password":password}).to_string(),false).into_http_bytes()).unwrap();
            let cookie=login.lines().find_map(|line|line.strip_prefix("Set-Cookie: ").and_then(|s|s.split(';').next())).unwrap();
            let endpoint="/api/dashboard/browser-launch";
            assert!(exchange("GET","/api/dashboard/profile-library","",Value::Null).await.0.contains("401"));
            let (profile_status,profiles)=exchange("GET","/api/dashboard/profile-library",cookie,Value::Null).await;
            assert!(profile_status.contains("200"),"{profile_status} {profiles}");
            assert!(profiles["data"]["profiles"].as_array().unwrap().is_empty());
            assert!(exchange("GET",endpoint,"",Value::Null).await.0.contains("401"),"caller header is not identity");
            let(status,catalog)=exchange("GET",endpoint,cookie,Value::Null).await;
            assert!(status.contains("200"),"{status} {catalog}");
            assert_eq!(catalog["data"]["schemaVersion"],"agent-browser.workspace-browser-catalog.v1");
            assert!(catalog["data"]["profiles"].as_array().unwrap().is_empty());
            let(status,refused)=exchange("POST",endpoint,cookie,json!({"operationId":"not-an-id","operator":"forged","profileMode":"disposable"})).await;
            assert!(status.contains("409"));assert_eq!(refused["outcome"],"refused_no_effect");
            let operation="c871220c-496c-4722-b772-08ad5781a78d";
            let(status,refused)=exchange("POST",endpoint,cookie,json!({"operationId":operation,"profileMode":"existing","profileId":"private","servicePrincipalId":"forged"})).await;
            assert!(status.contains("409"));assert_eq!(refused["outcome"],"refused_no_effect");
            let(status,unknown)=exchange("GET",&format!("{endpoint}?operationId={operation}"),cookie,Value::Null).await;
            assert!(status.contains("409"));assert_eq!(unknown["outcome"],"unknown");
            let store=crate::native::browser_session_store::BrowserSessionSqliteStore::default_sqlite().unwrap();
            let state=store.load_session_state().unwrap();assert!(state.browsers.is_empty());assert!(state.sessions.is_empty());assert!(state.managed_tab_requests.is_empty());
            assert!(!crate::connection::daemon_ready(DASHBOARD_SERVICE_BACKEND_SESSION));
        });
    }
}
