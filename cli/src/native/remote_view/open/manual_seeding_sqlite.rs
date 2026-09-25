//! Presentation proof for a SQLite-reserved, CDP-free manual-seeding browser.

use agent_browser_service_model::{
    RecordedProcessIdentity, RouteKeeperHandoffBinding, ViewStreamProvider,
};
use serde_json::{json, Value};

use super::{
    ensure_route_display_access, remote_view_open_operator_access_readiness,
    remote_view_open_visible_window_proof, route_binding_with_operator_access,
    snapshot_browser_scene_when_ready,
};
use crate::native::remote_view::RemoteViewRouteBinding;
use crate::native::remote_view_handoff::route_bound_manual_seeding_operator_visible;

/// Project the exact keeper-owned slot into the existing presentation probes.
/// The public URL is probed here; only the durable opaque URL is returned to
/// the operator after SQLite publication.
#[allow(dead_code)]
pub(crate) fn sqlite_manual_seeding_route_binding(
    binding: &RouteKeeperHandoffBinding,
) -> RemoteViewRouteBinding {
    RemoteViewRouteBinding {
        route_id: binding.slot_id.clone(),
        route_pool_entry_id: None,
        display_allocation_id: binding.display_name.clone(),
        route_pool_entry_state: Some("available".to_string()),
        current_route_allocation_id: Some(binding.slot_id.clone()),
        display_name: Some(binding.display_name.clone()),
        launch_display_name: Some(binding.display_name.clone()),
        display_isolation: "dedicated".to_string(),
        route_user: Some(binding.route_user.clone()),
        display_access: None,
        provider: ViewStreamProvider::RdpGateway,
        provider_mode: "single_controller".to_string(),
        connection_id: Some(binding.guacamole_connection_id.to_string()),
        connection_name: None,
        frame_url: None,
        external_url: None,
        route_descriptor: Some(json!({
            "publicOperatorUrl": binding.public_operator_url,
            "guacamoleConnectionId": binding.guacamole_connection_id,
            "guacamoleConnectionUuid": binding.guacamole_connection_uuid,
        })),
        readiness: Some(json!({
            "state": "ready",
            "source": "route_keeper_protocol_ready",
            "keeperId": binding.keeper_id,
            "hostGeneration": binding.fence.host_generation,
            "operationGeneration": binding.fence.operation_generation,
        })),
    }
}

/// Stage and observe a process-owned window, then probe the public operator
/// route. The caller must recheck the keeper fence and process identity before
/// committing the resulting proof in SQLite.
#[allow(dead_code)]
pub(crate) async fn prove_sqlite_manual_seeding_presentation(
    binding: &RouteKeeperHandoffBinding,
    process: &RecordedProcessIdentity,
    browser_id: &str,
) -> Result<Value, String> {
    if !crate::process_identity::recorded_process_is_running(process)? {
        return Err("manual_seeding_process_exited_before_proof".to_string());
    }
    ensure_route_display_access(&binding.slot_id, &binding.display_name, &binding.route_user)?;
    let scene = snapshot_browser_scene_when_ready(process.pid, &binding.display_name)
        .await
        .map_err(|issue| issue.compatibility_message().to_string())?;
    crate::native::x11_scene::stage_browser_scene(&scene)?;
    let route = sqlite_manual_seeding_route_binding(binding);
    let window_proof = remote_view_open_visible_window_proof(&route, Some(process.pid))?;
    let operator_access = remote_view_open_operator_access_readiness(&route)
        .await
        .ok_or_else(|| "manual_seeding_public_operator_probe_unavailable".to_string())?;
    let route = route_binding_with_operator_access(route, Some(operator_access));
    if !crate::process_identity::recorded_process_is_running(process)? {
        return Err("manual_seeding_process_exited_during_proof".to_string());
    }
    let operator_visible = route_bound_manual_seeding_operator_visible(
        &route,
        browser_id,
        "manual-seeding",
        Some(process.pid),
        Some(&window_proof),
    );
    if operator_visible.get("state").and_then(Value::as_str) != Some("ready") {
        return Err(format!(
            "manual_seeding_operator_visibility_unproven:{}",
            operator_visible
                .pointer("/notVisible/code")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ));
    }
    Ok(operator_visible)
}

#[cfg(test)]
mod tests {
    use agent_browser_service_model::RouteKeeperFence;

    use super::*;

    #[test]
    fn keeper_binding_projects_public_probe_without_provider_route_url() {
        let keeper = RouteKeeperHandoffBinding {
            slot_id: "slot-a".to_string(),
            keeper_id: "keeper-a".to_string(),
            fence: RouteKeeperFence {
                host_generation: 4,
                operation_id: "start-a".to_string(),
                operation_generation: 7,
                connection_catalog_digest: "digest-a".to_string(),
            },
            route_user: "route-a".to_string(),
            display_name: ":3".to_string(),
            guacamole_connection_id: 12,
            guacamole_connection_uuid: "connection-a".to_string(),
            public_operator_url: "https://dashboard.example/remote-view".to_string(),
        };
        let route = sqlite_manual_seeding_route_binding(&keeper);
        assert_eq!(route.route_id, "slot-a");
        assert_eq!(route.launch_display_name.as_deref(), Some(":3"));
        assert!(route.frame_url.is_none());
        assert!(route.external_url.is_none());
        assert_eq!(
            route.route_descriptor.as_ref().unwrap()["publicOperatorUrl"],
            "https://dashboard.example/remote-view"
        );
        let ready = route_binding_with_operator_access(
            route,
            Some(json!({"state":"ready","httpStatus":200})),
        );
        let visible = route_bound_manual_seeding_operator_visible(
            &ready,
            "manual-seeding:work:1",
            "manual-seeding",
            Some(4242),
            Some(&json!({"state":"ready","displayContent":{"state":"browser_visible"}})),
        );
        assert_eq!(visible["state"], "ready");
        assert_eq!(visible["manualSeedingProcess"]["pid"], 4242);
    }
}
