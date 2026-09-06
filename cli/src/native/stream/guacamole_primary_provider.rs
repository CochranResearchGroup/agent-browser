//! Local Guacamole authentication and receive-only tunnel startup.
//! Provider tokens never leave this module's transient connection setup.

use super::guacamole_primary_binding::PrimaryBinding;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::client::IntoClientRequest, MaybeTlsStream, WebSocketStream};

pub(super) async fn connect(
    binding: PrimaryBinding,
    is_current: Arc<dyn Fn() -> bool + Send + Sync>,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, &'static str> {
    if !is_current() {
        return Err("guacamole_primary_binding_changed");
    }
    let principal = std::env::var("AGENT_BROWSER_GUACAMOLE_HEADER_USER")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or("guacamole_primary_provider_principal_missing")?;
    connect_with_principal(binding, is_current, principal).await
}

async fn connect_with_principal(
    binding: PrimaryBinding,
    is_current: Arc<dyn Fn() -> bool + Send + Sync>,
    principal: String,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, &'static str> {
    if !is_current() {
        return Err("guacamole_primary_binding_changed");
    }
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| "guacamole_primary_provider_client_failed")?;
    let mut response = client
        .post(
            binding
                .provider_base
                .join("api/tokens")
                .map_err(|_| "guacamole_primary_provider_invalid")?,
        )
        .header("Remote-User", &principal)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("")
        .send()
        .await
        .map_err(|_| "guacamole_primary_provider_auth_failed")?;
    if !response.status().is_success() {
        return Err("guacamole_primary_provider_auth_failed");
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "guacamole_primary_provider_auth_failed")?
    {
        if body.len().saturating_add(chunk.len()) > 64 * 1024 {
            return Err("guacamole_primary_provider_auth_invalid");
        }
        body.extend_from_slice(&chunk);
    }
    let auth: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| "guacamole_primary_provider_auth_invalid")?;
    if auth["username"].as_str() != Some(principal.as_str()) {
        return Err("guacamole_primary_provider_auth_identity_mismatch");
    }
    // dataSource identifies the authentication extension (e.g. header), not
    // the connection directory. Require the exact PostgreSQL directory among
    // the data sources available to this authenticated principal.
    if !auth["availableDataSources"]
        .as_array()
        .is_some_and(|sources| {
            sources
                .iter()
                .any(|source| source.as_str() == Some("postgresql"))
        })
    {
        return Err("guacamole_primary_provider_data_source_unavailable");
    }
    let token = auth["authToken"]
        .as_str()
        .filter(|value| !value.is_empty() && value.len() <= 8192)
        .ok_or("guacamole_primary_provider_auth_invalid")?;
    if !is_current() {
        return Err("guacamole_primary_binding_changed");
    }
    let mut url = binding
        .provider_base
        .join("websocket-tunnel")
        .map_err(|_| "guacamole_primary_provider_invalid")?;
    let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
    url.set_scheme(scheme)
        .map_err(|_| "guacamole_primary_provider_invalid")?;
    url.query_pairs_mut().extend_pairs([
        ("token", token),
        ("GUAC_DATA_SOURCE", "postgresql"),
        ("GUAC_ID", binding.connection_id.as_str()),
        ("GUAC_TYPE", "c"),
        ("GUAC_WIDTH", "1920"),
        ("GUAC_HEIGHT", "1080"),
        ("GUAC_DPI", "96"),
        ("GUAC_TIMEZONE", "UTC"),
        ("GUAC_IMAGE", "image/png"),
    ]);
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|_| "guacamole_primary_provider_request_invalid")?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", "guacamole".parse().unwrap());
    let config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
        max_message_size: Some(2 * 1024 * 1024),
        max_frame_size: Some(2 * 1024 * 1024),
        ..Default::default()
    };
    let (socket, _) = tokio::time::timeout(
        Duration::from_secs(5),
        tokio_tungstenite::connect_async_with_config(request, Some(config), false),
    )
    .await
    .map_err(|_| "guacamole_primary_provider_connect_timeout")?
    .map_err(|_| "guacamole_primary_provider_connect_failed")?;
    if !is_current() {
        return Err("guacamole_primary_binding_changed");
    }
    Ok(socket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::stream::guacamole_primary_transport::{PrimaryStatus, PrimaryTask};
    use futures_util::{SinkExt, StreamExt};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::{
        handshake::server::{Request, Response},
        Message,
    };

    #[tokio::test]
    async fn header_authentication_binds_the_exact_websocket_and_rejects_foreign_principal() {
        for (foreign, postgres_available) in [(false, true), (true, true), (false, false)] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let mut binding = PrimaryBinding::synthetic_fixture();
            binding.provider_base = reqwest::Url::parse(&format!(
                "http://{}/guacamole/",
                listener.local_addr().unwrap()
            ))
            .unwrap();
            let server = tokio::spawn(async move {
                let (mut http, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    assert!(request.len() < 8192);
                    request.push(http.read_u8().await.unwrap());
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with("POST /guacamole/api/tokens HTTP/1.1\r\n"));
                assert!(request
                    .to_ascii_lowercase()
                    .contains("remote-user: synthetic-operator\r\n"));
                let body = serde_json::json!({
                    "authToken": "synthetic-token", "dataSource": "header",
                    "availableDataSources": if postgres_available { vec!["postgresql", "postgresql-shared"] } else { vec!["postgresql-shared"] },
                    "username": if foreign { "foreign-operator" } else { "synthetic-operator" }
                })
                .to_string();
                http.write_all(format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body).as_bytes()).await.unwrap();
                drop(http);
                if foreign || !postgres_available {
                    return;
                }
                let (tcp, _) = listener.accept().await.unwrap();
                let mut socket = tokio_tungstenite::accept_hdr_async(
                    tcp,
                    |request: &Request, response: Response| {
                        let url = reqwest::Url::parse(&format!("http://fixture{}", request.uri()))
                            .unwrap();
                        assert_eq!(url.path(), "/guacamole/websocket-tunnel");
                        let query = url
                            .query_pairs()
                            .collect::<std::collections::HashMap<_, _>>();
                        assert_eq!(query.get("token").unwrap(), "synthetic-token");
                        assert_eq!(query.get("GUAC_ID").unwrap(), "1");
                        assert_eq!(query.get("GUAC_DATA_SOURCE").unwrap(), "postgresql");
                        assert_eq!(query.get("GUAC_TYPE").unwrap(), "c");
                        assert_eq!(request.headers()["Sec-WebSocket-Protocol"], "guacamole");
                        let mut response = response;
                        response
                            .headers_mut()
                            .insert("Sec-WebSocket-Protocol", "guacamole".parse().unwrap());
                        Ok(response)
                    },
                )
                .await
                .unwrap();
                socket
                    .send(Message::Text(
                        "0.,36.00000000-0000-4000-8000-000000000001;4.sync,1.1;".into(),
                    ))
                    .await
                    .unwrap();
                while let Some(message) = socket.next().await {
                    match message.unwrap() {
                        Message::Text(value) => {
                            assert!(value == "3.nop;" || value == "4.sync,1.1;")
                        }
                        Message::Close(_) => break,
                        other => panic!("unexpected provider message: {other:?}"),
                    }
                }
            });
            let guard: Arc<dyn Fn() -> bool + Send + Sync> = Arc::new(|| true);
            let mut owner = PrimaryTask::connect(
                connect_with_principal(binding, guard.clone(), "synthetic-operator".into()),
                guard,
            );
            if foreign {
                assert_eq!(
                    owner.ready().await,
                    Err("guacamole_primary_provider_auth_identity_mismatch")
                );
            } else if !postgres_available {
                assert_eq!(
                    owner.ready().await,
                    Err("guacamole_primary_provider_data_source_unavailable")
                );
            } else {
                owner.ready().await.unwrap();
                assert_eq!(
                    owner.status(),
                    PrimaryStatus::Ready("00000000-0000-4000-8000-000000000001".into())
                );
            }
            owner.close().await;
            tokio::time::timeout(Duration::from_secs(2), server)
                .await
                .unwrap()
                .unwrap();
        }
    }
}
