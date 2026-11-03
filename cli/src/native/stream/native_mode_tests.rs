//! Real private HTTP admission, no browser or provider is started.
use super::*;
use crate::test_utils::EnvGuard;

#[tokio::test]
async fn native_mode_callback_requires_gateway_capability_and_rejects_browser_selected_scope() {
    let directory = std::env::temp_dir().join(format!("native-mode-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let env = EnvGuard::new(&["AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE"]);
    let token = directory.join("token");
    std::fs::write(&token, b"synthetic-native-mode-capability-123456789").unwrap();
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    env.set("AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE", token.to_str().unwrap());
    let valid = r#"{"slot":1,"generation":1,"choice":"inspect"}"#;
    for (headers, body, status) in [
        ("", valid, 403),
        ("Authorization: Bearer wrong-capability\r\nX-Remote-View-Operator: operator\r\n", valid, 403),
        ("Authorization: Bearer synthetic-native-mode-capability-123456789\r\nX-Remote-View-Operator: operator\r\nCookie: forged=1\r\n", valid, 403),
        ("Authorization: Bearer synthetic-native-mode-capability-123456789\r\nX-Remote-View-Operator: operator\r\n", r#"{"slot":1,"generation":1,"choice":"prepare","sessionId":"foreign","operationId":"op","mode":"cdp"}"#, 400),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { let (stream, _) = listener.accept().await.unwrap(); handle_dashboard_connection(stream).await; });
        let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
        client.write_all(format!("POST /api/native-remote-view/retained-task/mode HTTP/1.1\r\nHost: localhost\r\n{headers}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        let mut reply = Vec::new(); timeout(Duration::from_secs(3), client.read_to_end(&mut reply)).await.unwrap().unwrap(); task.await.unwrap();
        assert_eq!(http_response_status(&reply), Some(status), "{headers}");
    }
    std::fs::remove_dir_all(directory).unwrap();
}
