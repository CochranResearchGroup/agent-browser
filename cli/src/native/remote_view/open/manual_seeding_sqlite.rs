//! Presentation proof for a SQLite-reserved, CDP-free manual-seeding browser.

use agent_browser_service_model::{
    RecordedProcessIdentity, RouteKeeperHandoffBinding, ViewStreamProvider,
};
use serde_json::{json, Value};

use super::RouteBoundOpenAttribution;
use super::{
    ensure_route_display_access, remote_view_open_operator_access_readiness,
    remote_view_open_visible_window_proof, route_binding_with_operator_access,
    snapshot_browser_scene_when_ready,
};
use crate::native::action_runtime::runtime::{
    build_cdp_free_launch_plan_for_sqlite_profile, launch_cdp_free_from_sqlite_profile,
    validate_cdp_free_launch_plan, DaemonState,
};
use crate::native::browser_session_store::{
    BrowserRuntimeSqliteStore, ManualSeedingReservation, ManualSeedingState,
};
use crate::native::remote_view::RemoteViewRouteBinding;
use crate::native::remote_view_handoff::route_bound_manual_seeding_operator_visible;

enum ManualSeedingAcquisition {
    Existing(Value),
    Prove {
        profile_id: String,
        operation_id: String,
        generation: u64,
        binding: Box<RouteKeeperHandoffBinding>,
        process: RecordedProcessIdentity,
    },
}

fn manual_seeding_command_string(command: &Value, key: &str) -> Option<String> {
    command
        .get(key)
        .or_else(|| command.get("params").and_then(|params| params.get(key)))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn manual_seeding_executable_path(command: &Value) -> Result<String, String> {
    let path = manual_seeding_command_string(command, "executablePath")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("AGENT_BROWSER_EXECUTABLE_PATH").map(Into::into))
        .or_else(crate::native::cdp::chrome::find_chrome)
        .ok_or_else(|| "manual_seeding_chrome_executable_unavailable".to_string())?;
    let path = std::fs::canonicalize(&path)
        .map_err(|error| format!("manual_seeding_chrome_executable_invalid:{error}"))?;
    if !path.is_file() {
        return Err("manual_seeding_chrome_executable_not_file".to_string());
    }
    Ok(path.to_string_lossy().into_owned())
}

fn manual_seeding_launch_command(
    request: &ManualSeedingReservation,
    binding: &RouteKeeperHandoffBinding,
) -> Value {
    json!({
        "action": "cdp_free_launch",
        "profileId": request.profile_id,
        "runtimeProfile": request.profile_id,
        "executablePath": request.executable_path,
        "url": request.requested_url,
        "browserHost": "remote_headed",
        "routeId": binding.slot_id,
        "displayAllocationId": binding.display_name,
        "remoteHeadedDisplay": binding.display_name,
        "displayIsolation": "private_virtual_display",
        "viewStreamProvider": "rdp_gateway",
        "controlInput": "manual_attached_desktop",
    })
}

fn ready_manual_seeding_bindings(
    authority: &agent_browser_service_model::RouteKeeperAuthority,
) -> Vec<RouteKeeperHandoffBinding> {
    authority
        .records
        .values()
        .filter_map(|record| {
            let ready = record.protocol_ready.as_ref()?;
            authority
                .ready_handoff_binding(&record.slot_id, &ready.display_name)
                .ok()
        })
        .collect()
}

fn abort_manual_seeding_before_effect(
    store: &mut BrowserRuntimeSqliteStore,
    request: &ManualSeedingReservation,
    generation: u64,
    reason: &str,
) -> Result<(), String> {
    store
        .abort_manual_seeding_before_launch(
            &request.profile_id,
            &request.operation_id,
            generation,
            reason,
        )
        .map(|_| ())
}

fn prepare_sqlite_manual_seeding_acquisition(
    command: &Value,
    state: &DaemonState,
) -> Result<ManualSeedingAcquisition, String> {
    let profile_id = manual_seeding_command_string(command, "profileId")
        .or_else(|| manual_seeding_command_string(command, "runtimeProfile"))
        .ok_or_else(|| "service_profile_manual_seeding_acquire requires profileId".to_string())?;
    let target_service_id =
        manual_seeding_command_string(command, "targetServiceId").ok_or_else(|| {
            "service_profile_manual_seeding_acquire requires targetServiceId".to_string()
        })?;
    let requested_url = manual_seeding_command_string(command, "url");
    let mut store = BrowserRuntimeSqliteStore::default_sqlite()?;
    let catalog = store.load_profile_catalog()?;
    let profile = catalog
        .profiles
        .get(&profile_id)
        .filter(|profile| profile.id == profile_id)
        .ok_or_else(|| format!("manual_seeding_profile_not_registered:{profile_id}"))?;
    let existing = store.load_manual_seeding_record(&profile_id)?;
    let (request, previous) = match existing
        .filter(|record| record.state != ManualSeedingState::Closed)
    {
        Some(record) => {
            // A PID without launch identity is never adoption or kill authority.
            // Terminalize only when that PID has disappeared; the caller must
            // submit a new operation before another launch can be issued.
            if record.state == ManualSeedingState::RecoveryRequired
                && record.process_identity.is_none()
            {
                if let Some(pid) = record.uncertain_launch_pid {
                    store.reconcile_absent_uncertain_manual_seeding_launch(
                        &profile_id,
                        &record.operation_id,
                        record.generation,
                        pid,
                    )?;
                    return Ok(ManualSeedingAcquisition::Existing(json!({
                        "status": "uncertain_launch_process_absent",
                        "resolved": false,
                        "profileId": profile_id,
                        "handoffId": record.handoff_id,
                        "pid": pid,
                        "retryRequiresNewOperation": true,
                        "operatorVisible": {
                            "state": "not_checked",
                            "reason": "no_live_manual_seeding_browser",
                        },
                    })));
                }
            }
            if record.target_service_id != target_service_id
                || record.requested_url != requested_url
            {
                return Err(format!("manual_seeding_profile_busy:{profile_id}"));
            }
            if record.state == ManualSeedingState::Ready {
                let process = record
                    .process_identity
                    .as_ref()
                    .ok_or_else(|| "manual_seeding_ready_process_identity_missing".to_string())?;
                if !crate::process_identity::recorded_process_is_running(process)? {
                    return Err("manual_seeding_ready_process_missing".to_string());
                }
                let handoff = store
                    .load_handoff_registry()?
                    .handoffs
                    .get(&record.handoff_id)
                    .filter(|handoff| handoff.state == "ready")
                    .cloned()
                    .ok_or_else(|| "manual_seeding_ready_handoff_missing".to_string())?;
                return Ok(ManualSeedingAcquisition::Existing(json!({
                    "status": "manual_seeding_in_progress",
                    "reused": true,
                    "profileId": profile_id,
                    "targetServiceId": target_service_id,
                    "pid": process.pid,
                    "handoffId": handoff.id,
                    "handoffUrl": handoff.handoff_url,
                    "operatorVisible": {
                        "state": "not_checked",
                        "reason": "reopen_the_existing_opaque_handoff_for_current_visibility",
                    },
                    "authentication": {
                        "state": "not_probed",
                        "reason": "visibility_is_not_authentication_evidence",
                    },
                })));
            }
            if record.state == ManualSeedingState::LaunchObserved {
                let slot_id = record
                    .route_slot_id
                    .as_deref()
                    .ok_or_else(|| "manual_seeding_route_missing_after_launch".to_string())?;
                let display_name = record
                    .display_name
                    .as_deref()
                    .ok_or_else(|| "manual_seeding_display_missing_after_launch".to_string())?;
                let authority = store.load_route_keeper_authority()?;
                let binding = authority.ready_handoff_binding(slot_id, display_name)?;
                if record.route_fence.as_ref() != Some(&binding.fence) {
                    return Err("manual_seeding_route_binding_changed".to_string());
                }
                let process = record.process_identity.ok_or_else(|| {
                    "manual_seeding_process_identity_missing_after_launch".to_string()
                })?;
                return Ok(ManualSeedingAcquisition::Prove {
                    profile_id,
                    operation_id: record.operation_id,
                    generation: record.generation,
                    binding: Box::new(binding),
                    process,
                });
            }
            if record.state != ManualSeedingState::Reserved {
                return Err(format!(
                    "manual_seeding_recovery_required:{}:{:?}",
                    profile_id, record.state
                ));
            }
            (
                ManualSeedingReservation {
                    operation_id: record.operation_id.clone(),
                    profile_id: profile_id.clone(),
                    target_service_id: target_service_id.clone(),
                    handoff_id: record.handoff_id.clone(),
                    requested_url: requested_url.clone(),
                    executable_path: record.executable_path.clone(),
                },
                Some(record),
            )
        }
        None => (
            ManualSeedingReservation {
                operation_id: manual_seeding_command_string(command, "id")
                    .or_else(|| manual_seeding_command_string(command, "serviceJobId"))
                    .unwrap_or_else(|| {
                        format!("manual-seeding-operation-{}", uuid::Uuid::new_v4())
                    }),
                profile_id: profile_id.clone(),
                target_service_id: target_service_id.clone(),
                handoff_id: manual_seeding_command_string(command, "remoteViewHandoffId")
                    .unwrap_or_else(|| format!("manual-seeding-{}", uuid::Uuid::new_v4().simple())),
                requested_url: requested_url.clone(),
                executable_path: manual_seeding_executable_path(command)?,
            },
            None,
        ),
    };
    let (reserved, _) = store.reserve_manual_seeding(&request)?;
    let authority = store.load_route_keeper_authority()?;
    let mut bindings = ready_manual_seeding_bindings(&authority);
    if let Some(previous) = previous {
        if let Some(slot_id) = previous.route_slot_id.as_deref() {
            bindings.retain(|binding| {
                binding.slot_id == slot_id && previous.route_fence.as_ref() == Some(&binding.fence)
            });
        }
    }
    let mut selected = None;
    for binding in bindings {
        match store.bind_manual_seeding_route(
            &profile_id,
            &request.operation_id,
            reserved.generation,
            &binding,
        ) {
            Ok(_) => {
                selected = Some(binding);
                break;
            }
            Err(error) if error == "manual_seeding_route_busy" => continue,
            Err(error) => {
                abort_manual_seeding_before_effect(
                    &mut store,
                    &request,
                    reserved.generation,
                    "route_binding_failed",
                )?;
                return Err(error);
            }
        }
    }
    let Some(binding) = selected else {
        abort_manual_seeding_before_effect(
            &mut store,
            &request,
            reserved.generation,
            "ready_route_unavailable",
        )?;
        return Err("manual_seeding_ready_route_unavailable".to_string());
    };
    let launch_command = manual_seeding_launch_command(&request, &binding);
    let preflight = build_cdp_free_launch_plan_for_sqlite_profile(&launch_command, profile)
        .and_then(|plan| validate_cdp_free_launch_plan(&plan))
        .and_then(|_| {
            ensure_route_display_access(
                &binding.slot_id,
                &binding.display_name,
                &binding.route_user,
            )
            .map(|_| ())
        });
    if let Err(error) = preflight {
        abort_manual_seeding_before_effect(
            &mut store,
            &request,
            reserved.generation,
            "prelaunch_validation_failed",
        )?;
        return Err(error);
    }
    store.mark_manual_seeding_launch_issued(
        &profile_id,
        &request.operation_id,
        reserved.generation,
        &binding,
    )?;
    let launch = launch_cdp_free_from_sqlite_profile(&launch_command, state, profile)
        .map_err(|error| format!("manual_seeding_launch_recovery_required:{error}"))?;
    let Some(process) = launch.process_identity else {
        store.observe_uncertain_manual_seeding_launch(
            &profile_id,
            &request.operation_id,
            reserved.generation,
            &binding,
            launch.pid,
        )?;
        return Err("manual_seeding_launch_process_identity_uncertain".to_string());
    };
    let (observed, _) = store.observe_manual_seeding_launch(
        &profile_id,
        &request.operation_id,
        reserved.generation,
        &binding,
        &process,
    )?;
    if observed.state != ManualSeedingState::LaunchObserved {
        return Err("manual_seeding_launch_route_recovery_required".to_string());
    }
    Ok(ManualSeedingAcquisition::Prove {
        profile_id,
        operation_id: request.operation_id,
        generation: reserved.generation,
        binding: Box::new(binding),
        process,
    })
}

/// Acquire or resume one SQLite-owned manual-seeding browser and publish a
/// durable handoff only after live presentation proof succeeds.
pub(crate) async fn handle_sqlite_manual_seeding_acquire(
    command: &Value,
    state: &DaemonState,
    attribution: RouteBoundOpenAttribution,
) -> Result<Value, String> {
    if !attribution.authorization.is_authorized() {
        return Err("manual_seeding_authorization_required".to_string());
    }
    let prepared = prepare_sqlite_manual_seeding_acquisition(command, state)?;
    let ManualSeedingAcquisition::Prove {
        profile_id,
        operation_id,
        generation,
        binding,
        process,
    } = prepared
    else {
        let ManualSeedingAcquisition::Existing(response) = prepared else {
            unreachable!()
        };
        return Ok(response);
    };
    let browser_id = format!("manual-seeding:{profile_id}:{generation}");
    let dashboard_generation = attribution
        .dashboard_deployment_generation
        .or_else(|| crate::dashboard_ingress::selected_dashboard_generation().ok())
        .ok_or_else(|| "manual_seeding_dashboard_generation_unavailable".to_string())?;
    let operator_visible =
        prove_sqlite_manual_seeding_presentation(&binding, &process, &browser_id).await?;
    if !crate::process_identity::recorded_process_is_running(&process)? {
        return Err("manual_seeding_process_exited_before_publication".to_string());
    }
    let observed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut store = BrowserRuntimeSqliteStore::default_sqlite()?;
    let (_, operation) = store.publish_manual_seeding_ready(
        &profile_id,
        &operation_id,
        generation,
        &binding,
        &process,
        &operator_visible,
        &dashboard_generation,
        &observed_at,
    )?;
    operation
        .result
        .ok_or_else(|| "manual_seeding_publication_result_missing".to_string())
}

/// Terminate only the process instance journaled by the SQLite reservation,
/// then atomically close its seeding record and durable handoff.
pub(crate) async fn handle_sqlite_manual_seeding_close(command: &Value) -> Result<Value, String> {
    let profile_id = manual_seeding_command_string(command, "profileId")
        .or_else(|| manual_seeding_command_string(command, "runtimeProfile"))
        .ok_or_else(|| "service_profile_manual_seeding_close requires profileId".to_string())?;
    let target_service_id =
        manual_seeding_command_string(command, "targetServiceId").ok_or_else(|| {
            "service_profile_manual_seeding_close requires targetServiceId".to_string()
        })?;
    let handoff_id = manual_seeding_command_string(command, "handoffId")
        .or_else(|| manual_seeding_command_string(command, "remoteViewHandoffId"))
        .ok_or_else(|| "service_profile_manual_seeding_close requires handoffId".to_string())?;
    let expected_pid = command
        .get("pid")
        .or_else(|| command.get("params").and_then(|params| params.get("pid")))
        .and_then(Value::as_u64)
        .and_then(|pid| u32::try_from(pid).ok())
        .ok_or_else(|| "service_profile_manual_seeding_close requires pid".to_string())?;
    tokio::task::spawn_blocking(move || {
        use crate::process_identity::{
            assess_process_ownership, observe_process, LegacyProfileProof, VerifiedProcessSignal,
            VerifiedProcessTermination,
        };
        let mut store = BrowserRuntimeSqliteStore::default_sqlite()?;
        let record = store
            .load_manual_seeding_record(&profile_id)?
            .ok_or_else(|| format!("manual_seeding_reservation_missing:{profile_id}"))?;
        if record.target_service_id != target_service_id || record.handoff_id != handoff_id {
            return Err("manual_seeding_close_identity_mismatch".to_string());
        }
        let process = record
            .process_identity
            .as_ref()
            .ok_or_else(|| "manual_seeding_close_process_identity_missing".to_string())?;
        if process.pid != expected_pid {
            return Err("manual_seeding_close_pid_mismatch".to_string());
        }
        let assessment = assess_process_ownership(
            Some(process),
            observe_process(process.pid),
            LegacyProfileProof::Unproven,
        );
        let mut polite_attempted = false;
        let mut force_attempted = false;
        if assessment.authorizes_adoption() {
            let termination = VerifiedProcessTermination::open(process)?
                .ok_or_else(|| "manual_seeding_close_verified_process_missing".to_string())?;
            if termination.is_running()? {
                polite_attempted = termination.signal(VerifiedProcessSignal::Terminate)?;
                for _ in 0..20 {
                    if !termination.is_running()? {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                if termination.is_running()? {
                    force_attempted = termination.signal(VerifiedProcessSignal::Kill)?;
                    for _ in 0..20 {
                        if !termination.is_running()? {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
                if termination.is_running()? {
                    return Err("manual_seeding_close_process_survived".to_string());
                }
            }
        } else if !assessment.authorizes_cleanup() {
            return Err(format!(
                "manual_seeding_close_process_ambiguous:{}",
                assessment.reason
            ));
        }
        let closed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        store.close_manual_seeding_after_process_exit(
            &profile_id,
            &target_service_id,
            &handoff_id,
            process,
            &closed_at,
        )?;
        Ok(json!({
            "closed": true,
            "profileId": profile_id,
            "targetServiceId": target_service_id,
            "handoffId": handoff_id,
            "pid": expected_pid,
            "shutdown": {
                "politeCloseAttempted": polite_attempted,
                "forceKillAttempted": force_attempted,
                "processExited": true,
            },
            "routeRelease": { "status": "keeper_retained" },
            "attachableRelaunch": {
                "action": "service_profile_acquire",
                "profileId": profile_id,
                "targetServiceIds": [target_service_id],
            },
            "authenticationProbe": {
                "action": "service_profile_freshness_update",
                "state": "separate_required_probe",
                "visibilityAcceptedAsAuthentication": false,
            },
        }))
    })
    .await
    .map_err(|error| format!("manual_seeding_close_join_failed:{error}"))?
}

/// Resolve an opaque manual-seeding handoff from SQLite and re-prove its
/// current process, keeper slot, browser window, and public operator route.
pub(crate) async fn resolve_sqlite_manual_seeding_handoff(
    handoff_id: &str,
) -> Result<Value, String> {
    let (handoff, record, binding) = {
        let store = BrowserRuntimeSqliteStore::default_sqlite()?;
        let handoff = store
            .load_handoff_registry()?
            .handoffs
            .get(handoff_id)
            .cloned()
            .ok_or_else(|| "manual_seeding_handoff_not_found".to_string())?;
        if handoff.intent.get("manualSeeding").and_then(Value::as_bool) != Some(true) {
            return Err("manual_seeding_handoff_identity_mismatch".to_string());
        }
        let profile_id = handoff
            .profile_id
            .as_deref()
            .ok_or_else(|| "manual_seeding_handoff_profile_missing".to_string())?;
        let record = store
            .load_manual_seeding_record(profile_id)?
            .ok_or_else(|| "manual_seeding_record_missing".to_string())?;
        if record.handoff_id != handoff_id
            || handoff.intent.get("operationId").and_then(Value::as_str)
                != Some(record.operation_id.as_str())
        {
            return Err("manual_seeding_handoff_record_mismatch".to_string());
        }
        if record.state == ManualSeedingState::Closed || handoff.state == "closed" {
            return Ok(json!({
                "status": "explicitly_closed",
                "resolved": false,
                "handoffId": handoff_id,
                "profileId": profile_id,
            }));
        }
        if record.state != ManualSeedingState::Ready || handoff.state != "ready" {
            return Err("manual_seeding_handoff_recovery_required".to_string());
        }
        let slot_id = record
            .route_slot_id
            .as_deref()
            .ok_or_else(|| "manual_seeding_handoff_slot_missing".to_string())?;
        let display_name = record
            .display_name
            .as_deref()
            .ok_or_else(|| "manual_seeding_handoff_display_missing".to_string())?;
        let binding = store
            .load_route_keeper_authority()?
            .ready_handoff_binding(slot_id, display_name)?;
        if record.route_fence.as_ref() != Some(&binding.fence)
            || handoff.last_route_id.as_deref() != Some(slot_id)
            || handoff.last_display_allocation_id.as_deref() != Some(display_name)
        {
            return Err("manual_seeding_handoff_route_changed_recovery_required".to_string());
        }
        (handoff, record, binding)
    };
    let process = record
        .process_identity
        .as_ref()
        .ok_or_else(|| "manual_seeding_handoff_process_identity_missing".to_string())?;
    let browser_id = handoff
        .browser_id
        .as_deref()
        .ok_or_else(|| "manual_seeding_handoff_browser_id_missing".to_string())?;
    let visibility =
        prove_sqlite_manual_seeding_presentation(&binding, process, browser_id).await?;
    let store = BrowserRuntimeSqliteStore::default_sqlite()?;
    let current = store
        .load_manual_seeding_record(&record.profile_id)?
        .ok_or_else(|| "manual_seeding_record_missing_after_proof".to_string())?;
    let current_binding = store
        .load_route_keeper_authority()?
        .ready_handoff_binding(&binding.slot_id, &binding.display_name)?;
    if current != record || current_binding.fence != binding.fence {
        return Err("manual_seeding_handoff_changed_during_proof".to_string());
    }
    let mut result = handoff
        .last_resolution
        .ok_or_else(|| "manual_seeding_handoff_result_missing".to_string())?;
    result["operatorVisible"] = visibility;
    Ok(result)
}

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
