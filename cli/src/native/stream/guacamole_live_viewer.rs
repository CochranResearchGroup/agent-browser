//! Authenticated dashboard viewer lifecycle joined to live Guacamole state.
//!
//! Dashboard presence alone is insufficient. Connect and heartbeat renewals
//! re-observe a restricted Guacamole sharing tunnel before SQLite control
//! authority is granted or extended.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::guacamole_primary_provider::{
    observe_shared_connection_count, GuacamolePrimaryConnectSpec,
};
use crate::native::browser_session_store::{
    BrowserRuntimeSqliteStore, LiveViewerActivationRequest, LiveViewerHeartbeatRequest,
};
use crate::native::service_model::ViewStreamProvider;
use crate::native::service_store::{LockedServiceStateRepository, ServiceStateRepository};

const MAX_ID_LENGTH: usize = 512;

fn string_field<'a>(body: &'a Value, field: &str) -> Result<&'a str, &'static str> {
    body.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| {
            !value.is_empty()
                && value.len() <= MAX_ID_LENGTH
                && !value.chars().any(char::is_control)
        })
        .ok_or("live_viewer_request_invalid")
}

fn connection_id(body: &Value) -> Result<u64, &'static str> {
    string_field(body, "connectionId")?
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or("live_viewer_connection_id_invalid")
}

fn now_ms() -> Result<u64, &'static str> {
    use std::time::{SystemTime, UNIX_EPOCH};
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "live_viewer_clock_invalid")?
            .as_millis(),
    )
    .map_err(|_| "live_viewer_clock_invalid")
}

fn provider_spec(
    route_id: &str,
    connection_id: u64,
) -> Result<GuacamolePrimaryConnectSpec, &'static str> {
    let state = LockedServiceStateRepository::default_json()
        .map_err(|_| "live_viewer_state_unavailable")?
        .load_snapshot()
        .map_err(|_| "live_viewer_state_unavailable")?;
    let expected = connection_id.to_string();
    let route = state
        .remote_view_routes
        .get(route_id)
        .filter(|route| {
            route.id == route_id
                && route.connection_id.as_deref() == Some(expected.as_str())
                && route.provider == ViewStreamProvider::RdpGateway
                && route.provider_mode == "simultaneous_view"
        })
        .ok_or("live_viewer_route_binding_invalid")?;
    let local_embed = route
        .route_descriptor
        .as_ref()
        .and_then(|descriptor| descriptor.get("localEmbedUrl"))
        .and_then(Value::as_str)
        .ok_or("live_viewer_provider_unavailable")?;
    GuacamolePrimaryConnectSpec::from_local_embed(local_embed, expected)
        .map_err(|_| "live_viewer_provider_unavailable")
}

fn operation_id(principal: &str, handoff_id: &str, attempt_id: &str) -> String {
    let digest = Sha256::digest(format!("{principal}\0{handoff_id}\0{attempt_id}").as_bytes());
    format!("viewer-{}", hex::encode(&digest[..16]))
}

pub(super) async fn response(body: &str, principal: &str) -> (&'static str, Value) {
    match response_inner(body, principal).await {
        Ok(value) => ("200 OK", value),
        Err(code) => (
            if code.ends_with("_invalid") {
                "400 Bad Request"
            } else if matches!(
                code,
                "live_viewer_lease_inactive"
                    | "live_viewer_authority_mismatch"
                    | "live_viewer_tunnel_absent"
            ) {
                "409 Conflict"
            } else {
                "503 Service Unavailable"
            },
            json!({ "success": false, "code": code }),
        ),
    }
}

async fn response_inner(body: &str, principal: &str) -> Result<Value, &'static str> {
    if principal.trim().is_empty() || principal.len() > MAX_ID_LENGTH {
        return Err("live_viewer_principal_invalid");
    }
    let body: Value = serde_json::from_str(body).map_err(|_| "live_viewer_request_invalid")?;
    match string_field(&body, "operation")? {
        "connect" => {
            let handoff_id = string_field(&body, "handoffId")?;
            let attempt_id = string_field(&body, "attemptId")?;
            let route_id = string_field(&body, "routeId")?;
            let connection_id = connection_id(&body)?;
            let count =
                observe_shared_connection_count(provider_spec(route_id, connection_id)?).await?;
            if count == 0 {
                return Err("live_viewer_tunnel_absent");
            }
            let operation_id = operation_id(principal, handoff_id, attempt_id);
            let lease = BrowserRuntimeSqliteStore::default_sqlite()
                .map_err(|_| "live_viewer_state_unavailable")?
                .activate_live_viewer(&LiveViewerActivationRequest {
                    operation_id: operation_id.clone(),
                    handoff_id: handoff_id.to_string(),
                    client_connection_id: attempt_id.to_string(),
                    authenticated_principal: principal.to_string(),
                    provider_route_id: route_id.to_string(),
                    guacamole_connection_id: connection_id,
                    observed_shared_connection_count: count,
                    observed_at_ms: now_ms()?,
                })
                .map_err(map_store_error)?;
            Ok(json!({
                "success": true,
                "state": "controlling",
                "leaseId": format!("live-viewer:{operation_id}"),
                "controllerEpoch": lease.epoch,
                "heartbeatIntervalMs": 5000,
                "expiresAfterMs": 15000
            }))
        }
        "heartbeat" => {
            let lease_id = string_field(&body, "leaseId")?;
            let route_id = string_field(&body, "routeId")?;
            let connection_id = connection_id(&body)?;
            let count =
                observe_shared_connection_count(provider_spec(route_id, connection_id)?).await?;
            if count == 0 {
                return Err("live_viewer_tunnel_absent");
            }
            let lease = BrowserRuntimeSqliteStore::default_sqlite()
                .map_err(|_| "live_viewer_state_unavailable")?
                .heartbeat_live_viewer(&LiveViewerHeartbeatRequest {
                    lease_id: lease_id.to_string(),
                    authenticated_principal: principal.to_string(),
                    provider_route_id: route_id.to_string(),
                    guacamole_connection_id: connection_id,
                    observed_shared_connection_count: count,
                    observed_at_ms: now_ms()?,
                })
                .map_err(map_store_error)?;
            Ok(json!({
                "success": true,
                "state": "controlling",
                "leaseId": lease_id,
                "controllerEpoch": lease.epoch,
                "heartbeatIntervalMs": 5000,
                "expiresAfterMs": 15000
            }))
        }
        "disconnect" => {
            let lease_id = string_field(&body, "leaseId")?;
            BrowserRuntimeSqliteStore::default_sqlite()
                .map_err(|_| "live_viewer_state_unavailable")?
                .disconnect_live_viewer(lease_id, principal, now_ms()?)
                .map_err(map_store_error)?;
            Ok(json!({ "success": true, "state": "disconnected", "leaseId": lease_id }))
        }
        _ => Err("live_viewer_operation_invalid"),
    }
}

fn map_store_error(error: String) -> &'static str {
    match error.split(':').next().unwrap_or_default() {
        "live_viewer_lease_inactive" => "live_viewer_lease_inactive",
        "live_viewer_authority_mismatch" => "live_viewer_authority_mismatch",
        "live_viewer_tunnel_absent" => "live_viewer_tunnel_absent",
        "live_viewer_connection_binding_mismatch" => "live_viewer_connection_binding_invalid",
        "desktop_control_handoff_missing" | "desktop_control_handoff_not_ready" => {
            "live_viewer_handoff_invalid"
        }
        _ => "live_viewer_state_unavailable",
    }
}
