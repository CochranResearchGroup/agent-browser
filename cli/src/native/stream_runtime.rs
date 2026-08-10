#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::connection::get_socket_dir;
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use crate::native::stream::{self, StreamServer};
    use serde_json::{json, Map, Value};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    pub(crate) fn stream_file_path(session_id: &str) -> PathBuf {
        get_socket_dir().join(format!("{}.stream", session_id))
    }
    pub(crate) fn write_stream_file(session_id: &str, port: u16) -> Result<(), String> {
        let path = stream_file_path(session_id);
        fs::write(&path, port.to_string()).map_err(|e| {
            format!(
                "Failed to write stream metadata '{}': {}",
                path.display(),
                e
            )
        })
    }
    pub(crate) fn remove_stream_file(session_id: &str) -> Result<(), String> {
        let path = stream_file_path(session_id);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!(
                "Failed to remove stream metadata '{}': {}",
                path.display(),
                err
            )),
        }
    }
    pub(crate) fn engine_file_path(session_id: &str) -> PathBuf {
        get_socket_dir().join(format!("{}.engine", session_id))
    }
    pub(crate) fn write_engine_file(session_id: &str, engine: &str) {
        let _ = fs::write(engine_file_path(session_id), engine);
    }
    pub(crate) fn remove_engine_file(session_id: &str) {
        let _ = fs::remove_file(engine_file_path(session_id));
    }
    pub(crate) fn provider_file_path(session_id: &str) -> PathBuf {
        get_socket_dir().join(format!("{}.provider", session_id))
    }
    pub(crate) fn write_provider_file(session_id: &str, provider: &str) {
        let _ = fs::write(provider_file_path(session_id), provider);
    }
    pub(crate) fn remove_provider_file(session_id: &str) {
        let _ = fs::remove_file(provider_file_path(session_id));
    }
    pub(crate) fn extensions_file_path(session_id: &str) -> PathBuf {
        get_socket_dir().join(format!("{}.extensions", session_id))
    }
    pub(crate) fn write_extensions_file(session_id: &str) {
        if let Ok(val) = env::var("AGENT_BROWSER_EXTENSIONS") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                let _ = fs::write(extensions_file_path(session_id), trimmed);
                return;
            }
        }
        let _ = fs::remove_file(extensions_file_path(session_id));
    }
    pub(crate) fn remove_extensions_file(session_id: &str) {
        let _ = fs::remove_file(extensions_file_path(session_id));
    }
    pub(crate) async fn current_stream_status(state: &DaemonState) -> Value {
        debug_assert_eq!(
            state.stream_server.is_some(),
            state.stream_client.is_some(),
            "stream server and stream client slot should be set together"
        );
        let connected = match state.browser.as_ref() {
            Some(mgr) => mgr.is_connection_alive().await,
            None => false,
        };
        let runtime_screencasting = match state.stream_server.as_ref() {
            Some(server) => server.is_screencasting().await,
            None => false,
        };
        json!(
            { "enabled" : state.stream_server.is_some(), "port" : state.stream_server
            .as_ref().map(| server | Value::from(server.port())).unwrap_or(Value::Null),
            "connected" : connected, "screencasting" : connected && (state.screencasting
            || runtime_screencasting), }
        )
    }
    pub(crate) async fn handle_stream_enable(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        if state.stream_server.is_some() {
            return Err("Streaming is already enabled for this session".to_string());
        }
        let requested_port = match cmd.get("port").and_then(|value| value.as_u64()) {
            Some(raw) => u16::try_from(raw)
                .map_err(|_| format!("Invalid stream port '{}': expected 0-65535", raw))?,
            None => 0,
        };
        let (server, client_slot) =
            StreamServer::start_without_client(requested_port, state.session_id.clone(), false)
                .await?;
        let port = server.port();
        if let Err(err) = write_stream_file(&state.session_id, port) {
            server.shutdown().await;
            return Err(err);
        }
        state.stream_client = Some(client_slot);
        state.stream_server = Some(Arc::new(server));
        state.request_tracking = true;
        if state.screencasting {
            if let Some(ref server) = state.stream_server {
                server.set_screencasting(true).await;
            }
        }
        state.update_stream_client().await;
        Ok(current_stream_status(state).await)
    }
    pub(crate) async fn handle_stream_disable(state: &mut DaemonState) -> Result<Value, String> {
        let Some(server) = state.stream_server.clone() else {
            return Err("Streaming is not enabled for this session".to_string());
        };
        server.shutdown().await;
        state.stream_server = None;
        state.stream_client = None;
        remove_stream_file(&state.session_id)?;
        remove_engine_file(&state.session_id);
        remove_provider_file(&state.session_id);
        Ok(json!({ "disabled" : true }))
    }
    pub(crate) async fn handle_stream_status(state: &DaemonState) -> Result<Value, String> {
        Ok(current_stream_status(state).await)
    }

    pub(crate) async fn handle_screencast_start(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let manager = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = manager.active_session_id()?.to_string();
        if state.screencasting {
            return Err("Screencast already active".to_string());
        }
        let (default_width, default_height) = if let Some(ref server) = state.stream_server {
            server.viewport().await
        } else {
            (1280, 720)
        };
        let format = cmd.get("format").and_then(Value::as_str).unwrap_or("jpeg");
        let quality = cmd.get("quality").and_then(Value::as_i64).unwrap_or(80) as i32;
        let max_width = cmd
            .get("maxWidth")
            .and_then(Value::as_i64)
            .unwrap_or(i64::from(default_width)) as i32;
        let max_height = cmd
            .get("maxHeight")
            .and_then(Value::as_i64)
            .unwrap_or(i64::from(default_height)) as i32;
        stream::start_screencast(
            &manager.client,
            &session_id,
            format,
            quality,
            max_width,
            max_height,
        )
        .await?;
        state.screencasting = true;
        if let Some(ref server) = state.stream_server {
            server.set_screencasting(true).await;
            server
                .broadcast_status(
                    true,
                    true,
                    max_width as u32,
                    max_height as u32,
                    &state.engine,
                )
                .await;
        }
        Ok(json!({ "started": true }))
    }

    pub(crate) async fn handle_screencast_stop(state: &mut DaemonState) -> Result<Value, String> {
        let manager = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = manager.active_session_id()?;
        if !state.screencasting {
            return Err("No screencast active".to_string());
        }
        stream::stop_screencast(&manager.client, session_id).await?;
        state.screencasting = false;
        if let Some(ref server) = state.stream_server {
            server.set_screencasting(false).await;
            let (viewport_width, viewport_height) = server.viewport().await;
            server
                .broadcast_status(true, false, viewport_width, viewport_height, &state.engine)
                .await;
        }
        Ok(json!({ "stopped": true }))
    }
}
pub(crate) use action_commands::*;
#[cfg(test)]
mod action_tests;
