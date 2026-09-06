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
    if auth["username"].as_str() != Some(principal.as_str())
        || auth["dataSource"].as_str() != Some("postgresql")
    {
        return Err("guacamole_primary_provider_auth_identity_mismatch");
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
