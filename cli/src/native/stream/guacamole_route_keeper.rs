//! Route-keeper ownership for the provider-neutral Guacamole primary task.

use super::guacamole_primary_provider::{connect_spec, GuacamolePrimaryConnectSpec};
use super::guacamole_primary_transport::{
    check_primary_authority, PrimaryGuard, PrimaryStatus, PrimaryTask,
};
use crate::native::browser_session_store::BrowserRuntimeSqliteStore;
use crate::native::presentation_route_keeper::{
    prepare_configured_cold_process_recovery, recover_prepared_cold_process_routes_until_shutdown,
    register_route_keeper_host_process, reserve_prepared_cold_process_routes,
    run_configured_route_keeper_supervisor, ConfiguredColdProcessRecoveryOutcome,
    PresentationRouteConnector, RouteKeeperAdoptionObservation, RouteKeeperConnectorObservation,
    RouteKeeperStopObservation, RouteKeeperTerminalEvent, SqliteRouteKeeperRepository,
    SupervisedPresentationRouteConnector,
};
use agent_browser_service_model::{
    RecordedProcessIdentity, RouteKeeperAdoptionReceipt, RouteKeeperAuthority, RouteKeeperFence,
    RouteKeeperPhase, RouteKeeperProtocolReadyReceipt, RouteKeeperReconcileAction,
    RouteKeeperStopReceipt, RouteKeeperXrdpOwnershipWitness,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::watch;

type PrimaryTerminalSink = Box<dyn FnOnce(&str, &'static str, u64) + Send>;

pub(crate) struct ConfiguredRouteKeeperSupervisorHandle {
    shutdown_tx: watch::Sender<bool>,
    join: Option<tokio::task::JoinHandle<Result<(), String>>>,
}

impl ConfiguredRouteKeeperSupervisorHandle {
    pub(crate) fn start_default() -> Result<Self, String> {
        const HELPER_PATH: &str =
            "/usr/local/libexec/agent-browser/agent-browser-privileged-helper";
        let database_path = BrowserRuntimeSqliteStore::default_sqlite_path()?;
        let repository = SqliteRouteKeeperRepository::new(&database_path);
        let current_executable = std::env::current_exe()
            .map_err(|error| format!("route_keeper_host_executable_unavailable:{error}"))?;
        let process_identity = crate::process_identity::capture_process_identity(
            std::process::id(),
            Some(&current_executable),
            None,
        )
        .ok_or_else(|| "route_keeper_host_process_identity_unavailable".to_string())?;
        let boot_epoch = crate::process_identity::current_boot_epoch()
            .ok_or_else(|| "route_keeper_host_boot_epoch_unavailable".to_string())?;
        let (authority, successor_host_generation) =
            register_route_keeper_host_process(&repository, &boot_epoch, process_identity)?;
        let prepared = prepare_configured_route_keeper_startup(
            &authority,
            successor_host_generation,
            &boot_epoch,
            |process_identity| crate::process_identity::observe_process(process_identity.pid),
            validate_configured_provider_catalog,
        )?;
        let prepared = reserve_prepared_cold_process_routes(&repository, &authority, prepared)?;

        let factory = ConfiguredRouteKeeperPrimaryFactory::new(database_path.clone());
        let observer = ConfiguredXrdpRouteKeeperObserver::new(
            database_path.clone(),
            InstalledXrdpHelperTransport::new(PathBuf::from(HELPER_PATH)),
        );
        let mut connector = GuacamoleRouteKeeperConnector::new(database_path, factory, observer);
        let (tick_tx, mut ticks) = mpsc::channel(1);
        let (shutdown_tx, mut shutdown) = watch::channel(false);
        let mut ticker_shutdown = shutdown.clone();
        let ticker = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval.tick().await;
            loop {
                tokio::select! {
                    changed = ticker_shutdown.changed() => {
                        if changed.is_err() || *ticker_shutdown.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        let _ = tick_tx.try_send(());
                    }
                }
            }
        });
        let join = tokio::spawn(async move {
            let result = match recover_prepared_cold_process_routes_until_shutdown(
                &repository,
                &mut connector,
                &prepared,
                &mut shutdown,
            )
            .await
            {
                Ok(ConfiguredColdProcessRecoveryOutcome::Recovered) => {
                    // The configured supervisor owns all subsequent reconciliation
                    // only after every retained candidate was started under the
                    // pre-provider proof packet above.
                    run_configured_route_keeper_supervisor(
                        &repository,
                        &mut connector,
                        &mut ticks,
                        &mut shutdown,
                    )
                    .await
                }
                Ok(ConfiguredColdProcessRecoveryOutcome::Cancelled) => Ok(()),
                Err(error) => Err(error),
            };
            ticker.abort();
            let _ = ticker.await;
            result
        });
        Ok(Self {
            shutdown_tx,
            join: Some(join),
        })
    }

    pub(crate) async fn shutdown(&mut self) -> Result<(), String> {
        let _ = self.shutdown_tx.send(true);
        let Some(join) = self.join.take() else {
            return Ok(());
        };
        join.await
            .map_err(|error| format!("route_keeper_supervisor_join_failed:{error}"))?
    }
}

fn prepare_configured_route_keeper_startup<F, V>(
    authority: &RouteKeeperAuthority,
    successor_host_generation: u64,
    boot_epoch: &str,
    observe_process: F,
    validate_provider: V,
) -> Result<Vec<crate::native::presentation_route_keeper::PreparedRouteKeeperColdRecovery>, String>
where
    F: FnMut(&RecordedProcessIdentity) -> crate::process_identity::ProcessObservation,
    V: FnOnce(&RouteKeeperAuthority) -> Result<(), String>,
{
    let prepared = prepare_configured_cold_process_recovery(
        authority,
        successor_host_generation,
        boot_epoch,
        observe_process,
    )?;
    validate_provider(authority)?;
    Ok(prepared)
}

fn validate_configured_provider_catalog(authority: &RouteKeeperAuthority) -> Result<(), String> {
    authority.projection()?;
    let provider_base = authority
        .connection_catalog
        .provider_base
        .as_deref()
        .ok_or_else(|| "route_keeper_primary_provider_unconfigured".to_string())?;
    GuacamolePrimaryConnectSpec::from_local_embed(provider_base, "catalog-validation")
        .map_err(str::to_string)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn configured_route_keeper_supervisor_fixture() -> (
    ConfiguredRouteKeeperSupervisorHandle,
    Arc<std::sync::atomic::AtomicUsize>,
) {
    let shutdowns = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = shutdowns.clone();
    let (shutdown_tx, mut shutdown) = watch::channel(false);
    let join = tokio::spawn(async move {
        while shutdown.changed().await.is_ok() {
            if *shutdown.borrow() {
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return Ok(());
            }
        }
        Err("route_keeper_supervisor_fixture_shutdown_closed".to_string())
    });
    (
        ConfiguredRouteKeeperSupervisorHandle {
            shutdown_tx,
            join: Some(join),
        },
        shutdowns,
    )
}

pub(super) trait RouteKeeperPrimaryFactory: Send {
    fn start(
        &mut self,
        action: &RouteKeeperReconcileAction,
        guard: PrimaryGuard,
        on_closed: PrimaryTerminalSink,
    ) -> Result<PrimaryTask, String>;
}

/// SQLite-resolved provider factory for browser-independent route keepers.
///
/// The connection catalog is read from the same durable authority as the
/// action fence. Provider access begins only after the exact catalog digest,
/// slot binding, keeper identity, and operation fence all agree.
pub(super) struct ConfiguredRouteKeeperPrimaryFactory {
    database_path: PathBuf,
}

impl ConfiguredRouteKeeperPrimaryFactory {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }
}

impl RouteKeeperPrimaryFactory for ConfiguredRouteKeeperPrimaryFactory {
    fn start(
        &mut self,
        action: &RouteKeeperReconcileAction,
        guard: PrimaryGuard,
        on_closed: PrimaryTerminalSink,
    ) -> Result<PrimaryTask, String> {
        let (slot_id, keeper_id, fence, expected_phase) = primary_start_identity(action)?;
        let authority =
            BrowserRuntimeSqliteStore::open(&self.database_path)?.load_route_keeper_authority()?;
        authority.projection()?;
        let digest = authority.connection_catalog.digest()?;
        if fence.connection_catalog_digest != digest {
            return Err("route_keeper_primary_catalog_fence_stale".to_string());
        }
        let record = authority
            .records
            .get(slot_id)
            .ok_or_else(|| "route_keeper_primary_fence_stale".to_string())?;
        if record.keeper_id != keeper_id || record.fence != *fence || record.phase != expected_phase
        {
            return Err("route_keeper_primary_fence_stale".to_string());
        }
        let binding = authority
            .connection_catalog
            .bindings
            .get(slot_id)
            .ok_or_else(|| "route_keeper_primary_connection_unconfigured".to_string())?;
        let provider_base = authority
            .connection_catalog
            .provider_base
            .as_deref()
            .ok_or_else(|| "route_keeper_primary_provider_unconfigured".to_string())?;
        let spec = GuacamolePrimaryConnectSpec::from_local_embed(
            provider_base,
            binding.guacamole_connection_id.to_string(),
        )
        .map_err(str::to_string)?;
        Ok(PrimaryTask::connect_observed(
            connect_spec(spec, guard.clone()),
            guard,
            on_closed,
        ))
    }
}

#[async_trait::async_trait]
pub(super) trait RouteKeeperProtocolObserver: Send {
    async fn observe_xrdp(
        &mut self,
        action: &RouteKeeperReconcileAction,
        guacamole_connection_uuid: &str,
    ) -> Result<Option<RouteKeeperProtocolReadyReceipt>, String>;

    async fn stop_exact(
        &mut self,
        action: &RouteKeeperReconcileAction,
        ready: &RouteKeeperProtocolReadyReceipt,
    ) -> Result<RouteKeeperStopObservation, String>;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XrdpHelperResponse {
    schema_version: u32,
    state: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    witness: Option<RouteKeeperXrdpOwnershipWitness>,
    #[serde(default)]
    witness_digest: Option<String>,
    #[serde(default)]
    verification: Option<XrdpStopVerification>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XrdpStopVerification {
    session_instance_absent: bool,
    x_server_instance_absent: bool,
    owned_scope_empty_or_absent: bool,
}

#[derive(Debug, Clone)]
enum XrdpHelperObservation {
    Pending,
    Ready(Box<RouteKeeperXrdpOwnershipWitness>),
    OwnershipUnproven(String),
}

#[derive(Debug, Clone)]
enum XrdpHelperStop {
    Stopped,
    OwnershipUnproven(String),
}

#[async_trait::async_trait]
trait XrdpHelperTransport: Send {
    async fn observe(&mut self, route_user: &str) -> Result<XrdpHelperObservation, String>;

    async fn stop_exact(
        &mut self,
        witness: &RouteKeeperXrdpOwnershipWitness,
    ) -> Result<XrdpHelperStop, String>;
}

pub(super) struct InstalledXrdpHelperTransport {
    helper_path: PathBuf,
    timeout: Duration,
}

impl InstalledXrdpHelperTransport {
    pub(super) fn new(helper_path: PathBuf) -> Self {
        Self {
            helper_path,
            timeout: Duration::from_secs(15),
        }
    }

    async fn run(&self, args: &[String]) -> Result<XrdpHelperResponse, String> {
        let mut command = tokio::process::Command::new("sudo");
        command
            .arg("-n")
            .arg(&self.helper_path)
            .args(args)
            .kill_on_drop(true);
        let output = tokio::time::timeout(self.timeout, command.output())
            .await
            .map_err(|_| "route_keeper_xrdp_helper_timeout".to_string())?
            .map_err(|_| "route_keeper_xrdp_helper_spawn_failed".to_string())?;
        if !output.status.success() {
            return Err("route_keeper_xrdp_helper_failed".to_string());
        }
        if output.stdout.is_empty() || output.stdout.len() > 64 * 1024 {
            return Err("route_keeper_xrdp_helper_response_size_invalid".to_string());
        }
        parse_xrdp_helper_response(&output.stdout)
    }
}

fn parse_xrdp_helper_response(stdout: &[u8]) -> Result<XrdpHelperResponse, String> {
    serde_json::from_slice(stdout)
        .map_err(|_| "route_keeper_xrdp_helper_response_invalid".to_string())
}

#[async_trait::async_trait]
impl XrdpHelperTransport for InstalledXrdpHelperTransport {
    async fn observe(&mut self, route_user: &str) -> Result<XrdpHelperObservation, String> {
        let response = self
            .run(&[
                "observe-rdp-route-session".to_string(),
                "--user".to_string(),
                route_user.to_string(),
            ])
            .await?;
        if response.schema_version != 1 {
            return Err("route_keeper_xrdp_helper_schema_invalid".to_string());
        }
        match response.state.as_str() {
            "pending" => Ok(XrdpHelperObservation::Pending),
            "ready" => response
                .witness
                .map(Box::new)
                .map(XrdpHelperObservation::Ready)
                .ok_or_else(|| "route_keeper_xrdp_helper_witness_missing".to_string()),
            "ownership_unproven" | "unsupported" => Ok(XrdpHelperObservation::OwnershipUnproven(
                response
                    .code
                    .unwrap_or_else(|| "route_keeper_xrdp_ownership_unproven".to_string()),
            )),
            _ => Err("route_keeper_xrdp_helper_state_invalid".to_string()),
        }
    }

    async fn stop_exact(
        &mut self,
        witness: &RouteKeeperXrdpOwnershipWitness,
    ) -> Result<XrdpHelperStop, String> {
        let args = vec![
            "terminate-rdp-route-session-exact".to_string(),
            "--user".to_string(),
            witness.route_user.clone(),
            "--session-id".to_string(),
            witness.session_id.clone(),
            "--boot-id".to_string(),
            witness.boot_id.clone(),
            "--scope-invocation-id".to_string(),
            witness.scope_invocation_id.clone(),
            "--cgroup-device".to_string(),
            witness.cgroup_device.to_string(),
            "--cgroup-inode".to_string(),
            witness.cgroup_inode.to_string(),
            "--leader-pid".to_string(),
            witness.leader_pid.to_string(),
            "--leader-start-ticks".to_string(),
            witness.leader_start_ticks.to_string(),
            "--x-server-pid".to_string(),
            witness.x_server_pid.to_string(),
            "--x-server-start-ticks".to_string(),
            witness.x_server_start_ticks.to_string(),
            "--display".to_string(),
            witness.display_name.clone(),
            "--x11-socket-inode".to_string(),
            witness.x11_socket_inode.to_string(),
        ];
        let response = self.run(&args).await?;
        if response.schema_version != 1 {
            return Err("route_keeper_xrdp_helper_schema_invalid".to_string());
        }
        match response.state.as_str() {
            "stopped" => {
                let verification = response
                    .verification
                    .ok_or_else(|| "route_keeper_xrdp_stop_verification_missing".to_string())?;
                let expected_witness_digest = xrdp_witness_digest(witness);
                if !verification.session_instance_absent
                    || !verification.x_server_instance_absent
                    || !verification.owned_scope_empty_or_absent
                    || response.witness_digest.as_deref() != Some(expected_witness_digest.as_str())
                {
                    return Err("route_keeper_xrdp_stop_verification_invalid".to_string());
                }
                Ok(XrdpHelperStop::Stopped)
            }
            "ownership_unproven" | "incomplete" | "unsupported" => {
                Ok(XrdpHelperStop::OwnershipUnproven(
                    response
                        .code
                        .unwrap_or_else(|| "route_keeper_xrdp_stop_unproven".to_string()),
                ))
            }
            _ => Err("route_keeper_xrdp_helper_state_invalid".to_string()),
        }
    }
}

fn xrdp_witness_digest(witness: &RouteKeeperXrdpOwnershipWitness) -> String {
    let canonical = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
        witness.schema_version,
        witness.boot_id,
        witness.route_user,
        witness.route_uid,
        witness.session_id,
        witness.session_service,
        witness.session_scope,
        witness.scope_invocation_id,
        witness.cgroup_path,
        witness.cgroup_device,
        witness.cgroup_inode,
        witness.leader_pid,
        witness.leader_start_ticks,
        witness.x_server_pid,
        witness.x_server_start_ticks,
        witness.display_name,
    );
    let canonical = format!("{canonical}{}\n", witness.x11_socket_inode);
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}

pub(super) struct ConfiguredXrdpRouteKeeperObserver<T> {
    database_path: PathBuf,
    helper: T,
}

impl<T> ConfiguredXrdpRouteKeeperObserver<T> {
    pub(super) fn new(database_path: PathBuf, helper: T) -> Self {
        Self {
            database_path,
            helper,
        }
    }

    fn binding_for_action(
        &self,
        action: &RouteKeeperReconcileAction,
        expected_phase: RouteKeeperPhase,
    ) -> Result<
        (
            agent_browser_service_model::RouteKeeperConnectionBinding,
            Option<RouteKeeperProtocolReadyReceipt>,
        ),
        String,
    > {
        let (slot_id, keeper_id, fence) = match action {
            RouteKeeperReconcileAction::Observe {
                slot_id,
                keeper_id,
                fence,
            }
            | RouteKeeperReconcileAction::Adopt {
                slot_id,
                keeper_id,
                fence,
                ..
            }
            | RouteKeeperReconcileAction::Stop {
                slot_id,
                keeper_id,
                fence,
            } => (slot_id, keeper_id, fence),
            _ => return Err("route_keeper_xrdp_action_invalid".to_string()),
        };
        let authority =
            BrowserRuntimeSqliteStore::open(&self.database_path)?.load_route_keeper_authority()?;
        authority.projection()?;
        let record = authority
            .records
            .get(slot_id)
            .ok_or_else(|| "route_keeper_xrdp_slot_missing".to_string())?;
        if record.keeper_id != *keeper_id
            || record.fence != *fence
            || record.phase != expected_phase
            || authority.connection_catalog.digest()? != fence.connection_catalog_digest
        {
            return Err("route_keeper_xrdp_fence_stale".to_string());
        }
        let binding = authority
            .connection_catalog
            .bindings
            .get(slot_id)
            .cloned()
            .ok_or_else(|| "route_keeper_xrdp_route_user_missing".to_string())?;
        Ok((binding, record.protocol_ready.clone()))
    }
}

#[async_trait::async_trait]
impl<T> RouteKeeperProtocolObserver for ConfiguredXrdpRouteKeeperObserver<T>
where
    T: XrdpHelperTransport,
{
    async fn observe_xrdp(
        &mut self,
        action: &RouteKeeperReconcileAction,
        guacamole_connection_uuid: &str,
    ) -> Result<Option<RouteKeeperProtocolReadyReceipt>, String> {
        let expected_phase = match action {
            RouteKeeperReconcileAction::Observe { .. } => RouteKeeperPhase::Observing,
            RouteKeeperReconcileAction::Adopt { .. } => RouteKeeperPhase::Adopting,
            _ => return Err("route_keeper_xrdp_action_invalid".to_string()),
        };
        let (binding, _) = self.binding_for_action(action, expected_phase)?;
        let (slot_id, keeper_id, fence) = observe_identity(action)?;
        match self.helper.observe(&binding.route_user).await? {
            XrdpHelperObservation::Pending => Ok(None),
            XrdpHelperObservation::OwnershipUnproven(code) => {
                Err(format!("route_keeper_xrdp_ownership_unproven:{code}"))
            }
            XrdpHelperObservation::Ready(witness) => {
                if witness.route_user != binding.route_user {
                    return Err("route_keeper_xrdp_route_user_mismatch".to_string());
                }
                Ok(Some(RouteKeeperProtocolReadyReceipt {
                    slot_id: slot_id.to_string(),
                    keeper_id: keeper_id.to_string(),
                    fence: fence.clone(),
                    guacamole_connection_uuid: guacamole_connection_uuid.to_string(),
                    xrdp_session_id: witness.session_id.clone(),
                    display_name: witness.display_name.clone(),
                    xrdp_ownership: Some(*witness),
                    observed_at: chrono::Utc::now().to_rfc3339(),
                }))
            }
        }
    }

    async fn stop_exact(
        &mut self,
        action: &RouteKeeperReconcileAction,
        ready: &RouteKeeperProtocolReadyReceipt,
    ) -> Result<RouteKeeperStopObservation, String> {
        let (binding, durable_ready) =
            self.binding_for_action(action, RouteKeeperPhase::Stopping)?;
        if durable_ready.as_ref() != Some(ready) {
            return Ok(RouteKeeperStopObservation::OwnershipUnproven {
                preserved_observed_keeper_id: "route_keeper_xrdp_ready_receipt_mismatch"
                    .to_string(),
            });
        }
        let witness = ready
            .xrdp_ownership
            .as_ref()
            .ok_or_else(|| "route_keeper_xrdp_stop_ownership_missing".to_string())?;
        if witness.route_user != binding.route_user {
            return Ok(RouteKeeperStopObservation::OwnershipUnproven {
                preserved_observed_keeper_id: "route_keeper_xrdp_route_user_mismatch".to_string(),
            });
        }
        match self.helper.stop_exact(witness).await? {
            XrdpHelperStop::Stopped => Ok(RouteKeeperStopObservation::Stopped(
                RouteKeeperStopReceipt {
                    slot_id: ready.slot_id.clone(),
                    keeper_id: ready.keeper_id.clone(),
                    fence: ready.fence.clone(),
                    guacamole_connection_uuid: Some(ready.guacamole_connection_uuid.clone()),
                    xrdp_session_id: Some(ready.xrdp_session_id.clone()),
                    stopped_at: chrono::Utc::now().to_rfc3339(),
                },
            )),
            XrdpHelperStop::OwnershipUnproven(code) => {
                Ok(RouteKeeperStopObservation::OwnershipUnproven {
                    preserved_observed_keeper_id: code,
                })
            }
        }
    }
}

struct OwnedPrimary {
    keeper_id: String,
    fence: RouteKeeperFence,
    occurrence_id: String,
    task: PrimaryTask,
    task_closed: bool,
    ready: Option<RouteKeeperProtocolReadyReceipt>,
}

#[derive(Clone)]
struct CompletedStop {
    keeper_id: String,
    fence: RouteKeeperFence,
    observation: RouteKeeperStopObservation,
}

/// Process-local custody for exact route-keeper protocol tasks.
///
/// Provider authentication, connection lookup, XRDP inspection, and exact
/// process stop mechanics remain injected. This owner retains only the live
/// Guacamole protocol task and fenced receipt evidence.
pub(super) struct GuacamoleRouteKeeperConnector<F, O> {
    database_path: PathBuf,
    factory: F,
    observer: O,
    tasks: BTreeMap<String, OwnedPrimary>,
    completed_stops: BTreeMap<String, CompletedStop>,
    shutting_down: bool,
    terminal_tx: mpsc::UnboundedSender<RouteKeeperTerminalEvent>,
    terminal_rx: mpsc::UnboundedReceiver<RouteKeeperTerminalEvent>,
}

impl<F, O> GuacamoleRouteKeeperConnector<F, O> {
    pub(super) fn new(database_path: PathBuf, factory: F, observer: O) -> Self {
        let (terminal_tx, terminal_rx) = mpsc::unbounded_channel();
        Self {
            database_path,
            factory,
            observer,
            tasks: BTreeMap::new(),
            completed_stops: BTreeMap::new(),
            shutting_down: false,
            terminal_tx,
            terminal_rx,
        }
    }

    fn drain_terminal_events(&mut self) -> Vec<RouteKeeperTerminalEvent> {
        let mut events = Vec::new();
        while let Ok(mut event) = self.terminal_rx.try_recv() {
            if let Some(owned) = self.tasks.get(&event.slot_id).filter(|owned| {
                owned.keeper_id == event.keeper_id
                    && owned.fence == event.fence
                    && owned.occurrence_id == event.occurrence_id
            }) {
                event.guacamole_connection_uuid = owned
                    .ready
                    .as_ref()
                    .map(|ready| ready.guacamole_connection_uuid.clone());
            }
            events.push(event);
        }
        events
    }

    fn adoption_predecessor(
        &self,
        action: &RouteKeeperReconcileAction,
    ) -> Result<RouteKeeperProtocolReadyReceipt, String> {
        let (slot_id, keeper_id, fence, previous_host_generation) = adoption_identity(action)?;
        let authority =
            BrowserRuntimeSqliteStore::open(&self.database_path)?.load_route_keeper_authority()?;
        authority.projection()?;
        if authority.connection_catalog.digest()? != fence.connection_catalog_digest {
            return Err("route_keeper_primary_catalog_fence_stale".to_string());
        }
        let record = authority
            .records
            .get(slot_id)
            .ok_or_else(|| "route_keeper_primary_fence_stale".to_string())?;
        if record.keeper_id != keeper_id
            || record.fence != *fence
            || record.phase != RouteKeeperPhase::Adopting
        {
            return Err("route_keeper_primary_fence_stale".to_string());
        }
        let predecessor = record
            .protocol_ready
            .clone()
            .ok_or_else(|| "route_keeper_primary_adoption_source_missing".to_string())?;
        if predecessor.fence.host_generation != previous_host_generation
            || predecessor.guacamole_connection_uuid.is_empty()
        {
            return Err("route_keeper_primary_adoption_source_stale".to_string());
        }
        Ok(predecessor)
    }
}

#[async_trait::async_trait]
impl<F, O> PresentationRouteConnector for GuacamoleRouteKeeperConnector<F, O>
where
    F: RouteKeeperPrimaryFactory,
    O: RouteKeeperProtocolObserver,
{
    async fn start(&mut self, action: &RouteKeeperReconcileAction) -> Result<(), String> {
        self.shutting_down = false;
        let (slot_id, keeper_id, fence) = start_identity(action)?;
        if let Some(existing) = self.tasks.get(slot_id) {
            if existing.keeper_id == keeper_id && existing.fence == *fence {
                return Ok(());
            }
            return Err("route_keeper_primary_slot_owned_by_other_fence".to_string());
        }
        self.completed_stops.remove(slot_id);

        let guard = sqlite_route_keeper_guard(
            self.database_path.clone(),
            slot_id.to_string(),
            keeper_id.to_string(),
            fence.clone(),
        );
        let terminal_tx = self.terminal_tx.clone();
        let event_slot_id = slot_id.to_string();
        let event_keeper_id = keeper_id.to_string();
        let event_fence = fence.clone();
        let task = self.factory.start(
            action,
            guard,
            Box::new(move |occurrence_id, code, elapsed_ms| {
                let _ = terminal_tx.send(RouteKeeperTerminalEvent {
                    slot_id: event_slot_id,
                    keeper_id: event_keeper_id,
                    fence: event_fence,
                    occurrence_id: occurrence_id.to_string(),
                    guacamole_connection_uuid: None,
                    code,
                    elapsed_ms,
                });
            }),
        )?;
        let occurrence_id = task.occurrence_id.clone();
        self.tasks.insert(
            slot_id.to_string(),
            OwnedPrimary {
                keeper_id: keeper_id.to_string(),
                fence: fence.clone(),
                occurrence_id,
                task,
                task_closed: false,
                ready: None,
            },
        );
        Ok(())
    }

    async fn observe(
        &mut self,
        action: &RouteKeeperReconcileAction,
    ) -> Result<RouteKeeperConnectorObservation, String> {
        let (slot_id, keeper_id, fence) = observe_identity(action)?;
        let owned = self
            .tasks
            .get_mut(slot_id)
            .ok_or_else(|| "route_keeper_primary_task_missing".to_string())?;
        require_owned_primary(owned, keeper_id, fence)?;
        match owned.task.status() {
            PrimaryStatus::Starting => Ok(RouteKeeperConnectorObservation::Pending),
            PrimaryStatus::Ready(guacamole_connection_uuid) => {
                if let Some(ready) = &owned.ready {
                    if ready.guacamole_connection_uuid != guacamole_connection_uuid {
                        return Err("route_keeper_primary_connection_identity_changed".to_string());
                    }
                    return Ok(RouteKeeperConnectorObservation::Ready(Box::new(
                        ready.clone(),
                    )));
                }
                let Some(ready) = self
                    .observer
                    .observe_xrdp(action, &guacamole_connection_uuid)
                    .await?
                else {
                    return Ok(RouteKeeperConnectorObservation::Pending);
                };
                match owned.task.status() {
                    PrimaryStatus::Ready(current) if current == guacamole_connection_uuid => {}
                    PrimaryStatus::Closed(_) => {
                        return Ok(RouteKeeperConnectorObservation::Pending);
                    }
                    _ => {
                        return Err("route_keeper_primary_connection_identity_changed".to_string());
                    }
                }
                if ready.slot_id != slot_id
                    || ready.keeper_id != keeper_id
                    || ready.fence != *fence
                    || ready.guacamole_connection_uuid != guacamole_connection_uuid
                {
                    return Err("route_keeper_primary_observation_mismatch".to_string());
                }
                owned.ready = Some(ready.clone());
                Ok(RouteKeeperConnectorObservation::Ready(Box::new(ready)))
            }
            PrimaryStatus::Closed(_) => Ok(RouteKeeperConnectorObservation::Pending),
        }
    }

    async fn adopt(
        &mut self,
        action: &RouteKeeperReconcileAction,
    ) -> Result<RouteKeeperAdoptionObservation, String> {
        self.shutting_down = false;
        let (slot_id, keeper_id, fence, previous_host_generation) = adoption_identity(action)?;
        let predecessor = self.adoption_predecessor(action)?;
        if predecessor.fence.host_generation != previous_host_generation {
            return Err("route_keeper_primary_adoption_source_stale".to_string());
        }

        if let Some(existing) = self.tasks.get(slot_id) {
            if existing.keeper_id != keeper_id || existing.fence != *fence {
                return Err("route_keeper_primary_slot_owned_by_other_fence".to_string());
            }
        } else {
            self.completed_stops.remove(slot_id);
            let guard = sqlite_route_keeper_guard(
                self.database_path.clone(),
                slot_id.to_string(),
                keeper_id.to_string(),
                fence.clone(),
            );
            let terminal_tx = self.terminal_tx.clone();
            let event_slot_id = slot_id.to_string();
            let event_keeper_id = keeper_id.to_string();
            let event_fence = fence.clone();
            let task = self.factory.start(
                action,
                guard,
                Box::new(move |occurrence_id, code, elapsed_ms| {
                    let _ = terminal_tx.send(RouteKeeperTerminalEvent {
                        slot_id: event_slot_id,
                        keeper_id: event_keeper_id,
                        fence: event_fence,
                        occurrence_id: occurrence_id.to_string(),
                        guacamole_connection_uuid: None,
                        code,
                        elapsed_ms,
                    });
                }),
            )?;
            let occurrence_id = task.occurrence_id.clone();
            self.tasks.insert(
                slot_id.to_string(),
                OwnedPrimary {
                    keeper_id: keeper_id.to_string(),
                    fence: fence.clone(),
                    occurrence_id,
                    task,
                    task_closed: false,
                    ready: None,
                },
            );
        }

        let owned = self
            .tasks
            .get_mut(slot_id)
            .ok_or_else(|| "route_keeper_primary_task_missing".to_string())?;
        require_owned_primary(owned, keeper_id, fence)?;
        let ready = match owned.task.status() {
            PrimaryStatus::Starting | PrimaryStatus::Closed(_) => {
                return Ok(RouteKeeperAdoptionObservation::Pending);
            }
            PrimaryStatus::Ready(guacamole_connection_uuid) => {
                if let Some(ready) = &owned.ready {
                    if ready.guacamole_connection_uuid != guacamole_connection_uuid {
                        return Err("route_keeper_primary_connection_identity_changed".to_string());
                    }
                    ready.clone()
                } else {
                    let Some(ready) = self
                        .observer
                        .observe_xrdp(action, &guacamole_connection_uuid)
                        .await?
                    else {
                        return Ok(RouteKeeperAdoptionObservation::Pending);
                    };
                    match owned.task.status() {
                        PrimaryStatus::Ready(current) if current == guacamole_connection_uuid => {}
                        PrimaryStatus::Closed(_) => {
                            return Ok(RouteKeeperAdoptionObservation::Pending)
                        }
                        _ => {
                            return Err(
                                "route_keeper_primary_connection_identity_changed".to_string()
                            );
                        }
                    }
                    if ready.slot_id != slot_id
                        || ready.keeper_id != keeper_id
                        || ready.fence != *fence
                        || ready.guacamole_connection_uuid != guacamole_connection_uuid
                    {
                        return Err("route_keeper_primary_observation_mismatch".to_string());
                    }
                    owned.ready = Some(ready.clone());
                    ready
                }
            }
        };
        if ready.guacamole_connection_uuid == predecessor.guacamole_connection_uuid {
            return Err("route_keeper_primary_adoption_transport_not_fresh".to_string());
        }
        Ok(RouteKeeperAdoptionObservation::Adopted(Box::new(
            RouteKeeperAdoptionReceipt {
                previous_host_generation,
                previous_guacamole_connection_uuid: predecessor.guacamole_connection_uuid,
                ready,
                adopted_at: chrono::Utc::now().to_rfc3339(),
            },
        )))
    }

    async fn stop(
        &mut self,
        action: &RouteKeeperReconcileAction,
    ) -> Result<RouteKeeperStopObservation, String> {
        let (slot_id, keeper_id, fence) = stop_identity(action)?;
        if let Some(completed) = self.completed_stops.get(slot_id) {
            if completed.keeper_id == keeper_id && completed.fence == *fence {
                return Ok(completed.observation.clone());
            }
            return Ok(RouteKeeperStopObservation::OwnershipUnproven {
                preserved_observed_keeper_id: completed.keeper_id.clone(),
            });
        }
        let Some(owned) = self.tasks.get_mut(slot_id) else {
            return Ok(RouteKeeperStopObservation::OwnershipUnproven {
                preserved_observed_keeper_id: "route_keeper_primary_task_absent".to_string(),
            });
        };
        if let Err(error) = require_owned_primary(owned, keeper_id, fence) {
            let preserved_observed_keeper_id = owned.keeper_id.clone();
            if error == "route_keeper_primary_slot_owned_by_other_fence" {
                return Ok(RouteKeeperStopObservation::OwnershipUnproven {
                    preserved_observed_keeper_id,
                });
            }
            return Err(error);
        }
        let Some(ready) = owned.ready.clone() else {
            return Ok(RouteKeeperStopObservation::OwnershipUnproven {
                preserved_observed_keeper_id: keeper_id.to_string(),
            });
        };

        let stop_guard = sqlite_route_keeper_stop_guard(
            self.database_path.clone(),
            slot_id.to_string(),
            keeper_id.to_string(),
            fence.clone(),
        );
        check_primary_authority(&stop_guard)
            .await
            .map_err(str::to_string)?;
        let observation = self.observer.stop_exact(action, &ready).await?;
        if let Some(owned) = self.tasks.get_mut(slot_id) {
            if !owned.task_closed {
                owned.task.close().await;
                owned.task_closed = true;
            }
        }
        self.completed_stops.insert(
            slot_id.to_string(),
            CompletedStop {
                keeper_id: keeper_id.to_string(),
                fence: fence.clone(),
                observation: observation.clone(),
            },
        );
        self.tasks.remove(slot_id);
        Ok(observation)
    }

    async fn resume_stop(
        &mut self,
        action: &RouteKeeperReconcileAction,
        ready: &RouteKeeperProtocolReadyReceipt,
    ) -> Result<RouteKeeperStopObservation, String> {
        // A cold successor cannot establish ownership of a predecessor's
        // Guacamole primary.  The durable Stopping fence and retained XRDP
        // witness are therefore the only authority for this exact cleanup.
        // Do not create a transport or touch `tasks` here.
        let (slot_id, keeper_id, fence) = stop_identity(action)?;
        let stop_guard = sqlite_route_keeper_stop_guard(
            self.database_path.clone(),
            slot_id.to_string(),
            keeper_id.to_string(),
            fence.clone(),
        );
        check_primary_authority(&stop_guard)
            .await
            .map_err(str::to_string)?;
        self.observer.stop_exact(action, ready).await
    }
}

#[async_trait::async_trait]
impl<F, O> SupervisedPresentationRouteConnector for GuacamoleRouteKeeperConnector<F, O>
where
    F: RouteKeeperPrimaryFactory,
    O: RouteKeeperProtocolObserver,
{
    fn take_terminal_events(&mut self) -> Vec<RouteKeeperTerminalEvent> {
        self.drain_terminal_events()
    }

    fn terminal_event_is_current(&self, event: &RouteKeeperTerminalEvent) -> bool {
        self.tasks.get(&event.slot_id).is_some_and(|owned| {
            owned.keeper_id == event.keeper_id
                && owned.fence == event.fence
                && owned.occurrence_id == event.occurrence_id
                && (self.shutting_down || !owned.task_closed)
                && matches!(owned.task.status(), PrimaryStatus::Closed(_))
        })
    }

    fn restore_terminal_event(&mut self, event: RouteKeeperTerminalEvent) {
        let _ = self.terminal_tx.send(event);
    }

    fn acknowledge_terminal_event(&mut self, event: &RouteKeeperTerminalEvent) {
        if self.terminal_event_is_current(event) {
            self.tasks.remove(&event.slot_id);
        }
    }

    async fn shutdown_primaries(&mut self) {
        self.shutting_down = true;
        for owned in self.tasks.values_mut() {
            if !owned.task_closed {
                owned.task.close().await;
                owned.task_closed = true;
            }
        }
    }
}

fn start_identity(
    action: &RouteKeeperReconcileAction,
) -> Result<(&str, &str, &RouteKeeperFence), String> {
    match action {
        RouteKeeperReconcileAction::Start {
            slot_id,
            keeper_id,
            fence,
            ..
        } => Ok((slot_id, keeper_id, fence)),
        _ => Err("route_keeper_primary_start_action_required".to_string()),
    }
}

fn observe_identity(
    action: &RouteKeeperReconcileAction,
) -> Result<(&str, &str, &RouteKeeperFence), String> {
    match action {
        RouteKeeperReconcileAction::Observe {
            slot_id,
            keeper_id,
            fence,
        }
        | RouteKeeperReconcileAction::Adopt {
            slot_id,
            keeper_id,
            fence,
            ..
        } => Ok((slot_id, keeper_id, fence)),
        _ => Err("route_keeper_primary_observe_action_required".to_string()),
    }
}

fn primary_start_identity(
    action: &RouteKeeperReconcileAction,
) -> Result<(&str, &str, &RouteKeeperFence, RouteKeeperPhase), String> {
    match action {
        RouteKeeperReconcileAction::Start {
            slot_id,
            keeper_id,
            fence,
            ..
        } => Ok((slot_id, keeper_id, fence, RouteKeeperPhase::Starting)),
        RouteKeeperReconcileAction::Adopt {
            slot_id,
            keeper_id,
            fence,
            ..
        } => Ok((slot_id, keeper_id, fence, RouteKeeperPhase::Adopting)),
        _ => Err("route_keeper_primary_start_action_required".to_string()),
    }
}

fn adoption_identity(
    action: &RouteKeeperReconcileAction,
) -> Result<(&str, &str, &RouteKeeperFence, u64), String> {
    match action {
        RouteKeeperReconcileAction::Adopt {
            slot_id,
            keeper_id,
            fence,
            previous_host_generation,
        } => Ok((slot_id, keeper_id, fence, *previous_host_generation)),
        _ => Err("route_keeper_primary_adoption_action_required".to_string()),
    }
}

fn stop_identity(
    action: &RouteKeeperReconcileAction,
) -> Result<(&str, &str, &RouteKeeperFence), String> {
    match action {
        RouteKeeperReconcileAction::Stop {
            slot_id,
            keeper_id,
            fence,
        } => Ok((slot_id, keeper_id, fence)),
        _ => Err("route_keeper_primary_stop_action_required".to_string()),
    }
}

fn require_owned_primary(
    owned: &OwnedPrimary,
    keeper_id: &str,
    fence: &RouteKeeperFence,
) -> Result<(), String> {
    if owned.keeper_id != keeper_id
        || owned.fence != *fence
        || owned.occurrence_id != owned.task.occurrence_id
    {
        return Err("route_keeper_primary_slot_owned_by_other_fence".to_string());
    }
    Ok(())
}

pub(super) fn sqlite_route_keeper_guard(
    database_path: PathBuf,
    slot_id: String,
    keeper_id: String,
    fence: RouteKeeperFence,
) -> PrimaryGuard {
    Arc::new(move || {
        let authority = BrowserRuntimeSqliteStore::open(&database_path)
            .and_then(|store| store.load_route_keeper_authority())
            .map_err(|_| "guacamole_route_keeper_authority_unavailable")?;
        authority
            .projection()
            .map_err(|_| "guacamole_route_keeper_authority_invalid")?;
        let record = authority
            .records
            .get(&slot_id)
            .ok_or("guacamole_route_keeper_fence_stale")?;
        if record.slot_id != slot_id
            || record.keeper_id != keeper_id
            || record.fence != fence
            || !matches!(
                record.phase,
                RouteKeeperPhase::Starting
                    | RouteKeeperPhase::Observing
                    | RouteKeeperPhase::Adopting
                    | RouteKeeperPhase::Ready
            )
        {
            return Err("guacamole_route_keeper_fence_stale");
        }
        Ok(())
    })
}

fn sqlite_route_keeper_stop_guard(
    database_path: PathBuf,
    slot_id: String,
    keeper_id: String,
    fence: RouteKeeperFence,
) -> PrimaryGuard {
    Arc::new(move || {
        let authority = BrowserRuntimeSqliteStore::open(&database_path)
            .and_then(|store| store.load_route_keeper_authority())
            .map_err(|_| "guacamole_route_keeper_authority_unavailable")?;
        authority
            .projection()
            .map_err(|_| "guacamole_route_keeper_authority_invalid")?;
        let record = authority
            .records
            .get(&slot_id)
            .ok_or("guacamole_route_keeper_stop_fence_stale")?;
        if record.slot_id != slot_id
            || record.keeper_id != keeper_id
            || record.fence != fence
            || record.phase != RouteKeeperPhase::Stopping
        {
            return Err("guacamole_route_keeper_stop_fence_stale");
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::browser_session_store::LegacyBrowserRuntimeSources;
    use crate::native::presentation_route_keeper::{
        reconcile_once, run_route_keeper_supervisor, stop_once, RouteKeeperRepository,
        SqliteRouteKeeperRepository,
    };
    use crate::process_identity::ProcessObservation;
    use agent_browser_service_model::{
        RecordedProcessIdentity, RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog,
        RouteKeeperHostProcessClaim, RouteKeeperXrdpOwnershipWitness,
    };
    use futures_util::{SinkExt, StreamExt};
    use std::collections::VecDeque;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

    struct TempDirectory(PathBuf);

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn host_process_claim() -> RouteKeeperHostProcessClaim {
        RouteKeeperHostProcessClaim {
            host_generation: 1,
            boot_epoch: "linux:boot:fixture".to_string(),
            process_identity: RecordedProcessIdentity {
                pid: 4_001,
                start_token: "linux:start:1".to_string(),
                executable_path: Some("/opt/agent-browser".to_string()),
                browser_family: None,
            },
        }
    }

    fn database() -> (TempDirectory, PathBuf) {
        let directory = TempDirectory(std::env::temp_dir().join(format!(
            "agent-browser-route-keeper-primary-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        )));
        fs::create_dir_all(&directory.0).unwrap();
        let path = directory.0.join("runtime.sqlite3");
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join("browser-session-state.json"),
                profile_catalog_path: &directory.0.join("browser-profile-catalog.json"),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&path).unwrap();
        let original = store.load_route_keeper_authority().unwrap();
        let mut configured = original.clone();
        configured
            .register_host_process_claim(host_process_claim())
            .unwrap();
        configured
            .replace_connection_catalog(
                RouteKeeperConnectionCatalog::with_provider_base(
                    "http://127.0.0.1:8193/guacamole/",
                    (1_u32..=6).map(|sequence| RouteKeeperConnectionBinding {
                        slot_id: format!("route-slot-{sequence:02}"),
                        connection_key: format!("route-{sequence:02}"),
                        connection_name: format!("Agent Browser Route {sequence:02}"),
                        route_user: format!("agent-browser-rdp-{sequence}"),
                        guacamole_connection_id: u64::from(sequence),
                    }),
                )
                .unwrap(),
            )
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&original, &configured)
            .unwrap();
        (directory, path)
    }

    #[test]
    fn configured_startup_refuses_ambiguous_retained_route_before_provider_validation() {
        let (_directory, path) = database();
        let (_action, _predecessor) = adoption_action(&path);
        let mut store = BrowserRuntimeSqliteStore::open(&path).unwrap();
        let authority = store.load_route_keeper_authority().unwrap();
        let mut with_successor = authority.clone();
        let mut successor = host_process_claim();
        successor.host_generation = 3;
        successor.process_identity.pid = 4_003;
        with_successor
            .register_host_process_claim(successor)
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&authority, &with_successor)
            .unwrap();
        let provider_calls = AtomicUsize::new(0);

        let result = prepare_configured_route_keeper_startup(
            &with_successor,
            3,
            "linux:boot:fixture",
            |_| ProcessObservation::Failed {
                reason: "fixture process census ambiguous".to_string(),
            },
            |_| {
                provider_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(ref error)
                if error == "route_keeper_cold_process_interrupted_adopter_exit_unproven:route-slot-01"
        ));
        assert_eq!(provider_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            BrowserRuntimeSqliteStore::open(&path)
                .unwrap()
                .load_route_keeper_authority()
                .unwrap()
                .records["route-slot-01"]
                .fence,
            with_successor.records["route-slot-01"].fence
        );
    }

    struct DuplexPrimaryFactory {
        socket: Option<WebSocketStream<tokio::io::DuplexStream>>,
        starts: Arc<AtomicUsize>,
    }

    impl RouteKeeperPrimaryFactory for DuplexPrimaryFactory {
        fn start(
            &mut self,
            _action: &RouteKeeperReconcileAction,
            guard: PrimaryGuard,
            on_closed: PrimaryTerminalSink,
        ) -> Result<PrimaryTask, String> {
            let socket = self
                .socket
                .take()
                .ok_or_else(|| "fixture_primary_socket_missing".to_string())?;
            self.starts.fetch_add(1, Ordering::SeqCst);
            Ok(PrimaryTask::connect_observed(
                async move { Ok::<_, &'static str>(socket) },
                guard,
                on_closed,
            ))
        }
    }

    struct SequencedDuplexPrimaryFactory {
        sockets: VecDeque<WebSocketStream<tokio::io::DuplexStream>>,
        starts: Arc<AtomicUsize>,
    }

    impl RouteKeeperPrimaryFactory for SequencedDuplexPrimaryFactory {
        fn start(
            &mut self,
            _action: &RouteKeeperReconcileAction,
            guard: PrimaryGuard,
            on_closed: PrimaryTerminalSink,
        ) -> Result<PrimaryTask, String> {
            let socket = self
                .sockets
                .pop_front()
                .ok_or_else(|| "fixture_primary_socket_missing".to_string())?;
            self.starts.fetch_add(1, Ordering::SeqCst);
            Ok(PrimaryTask::connect_observed(
                async move { Ok::<_, &'static str>(socket) },
                guard,
                on_closed,
            ))
        }
    }

    #[derive(Default)]
    struct ExactProtocolObserver {
        observations: usize,
        stops: usize,
        fail_stop_once: bool,
        block_stop: bool,
        stop_entered: Option<Arc<tokio::sync::Notify>>,
    }

    #[async_trait::async_trait]
    impl RouteKeeperProtocolObserver for ExactProtocolObserver {
        async fn observe_xrdp(
            &mut self,
            action: &RouteKeeperReconcileAction,
            guacamole_connection_uuid: &str,
        ) -> Result<Option<RouteKeeperProtocolReadyReceipt>, String> {
            self.observations += 1;
            let (slot_id, keeper_id, fence) = observe_identity(action)?;
            let xrdp_session_id = format!("xrdp-{slot_id}");
            Ok(Some(RouteKeeperProtocolReadyReceipt {
                slot_id: slot_id.to_string(),
                keeper_id: keeper_id.to_string(),
                fence: fence.clone(),
                guacamole_connection_uuid: guacamole_connection_uuid.to_string(),
                xrdp_session_id: xrdp_session_id.clone(),
                display_name: ":10".to_string(),
                xrdp_ownership: Some(RouteKeeperXrdpOwnershipWitness {
                    schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
                    boot_id: "boot-fixture".to_string(),
                    route_user: "agent-browser-rdp-1".to_string(),
                    route_uid: 2001,
                    session_id: xrdp_session_id.clone(),
                    session_service: "xrdp-sesman".to_string(),
                    session_scope: format!("session-{xrdp_session_id}.scope"),
                    scope_invocation_id: "invocation-fixture".to_string(),
                    cgroup_path: format!(
                        "/user.slice/user-2001.slice/session-{xrdp_session_id}.scope"
                    ),
                    cgroup_device: 28,
                    cgroup_inode: 1001,
                    leader_pid: 4101,
                    leader_start_ticks: 5101,
                    x_server_pid: 4102,
                    x_server_start_ticks: 5102,
                    display_name: ":10".to_string(),
                    x11_socket_inode: 6101,
                }),
                observed_at: "2026-09-19T20:00:00Z".to_string(),
            }))
        }

        async fn stop_exact(
            &mut self,
            action: &RouteKeeperReconcileAction,
            ready: &RouteKeeperProtocolReadyReceipt,
        ) -> Result<RouteKeeperStopObservation, String> {
            self.stops += 1;
            if self.fail_stop_once {
                self.fail_stop_once = false;
                return Err("fixture_stop_observation_failed".to_string());
            }
            if let Some(entered) = &self.stop_entered {
                entered.notify_one();
            }
            if self.block_stop {
                std::future::pending().await
            }
            let (slot_id, keeper_id, fence) = stop_identity(action)?;
            Ok(RouteKeeperStopObservation::Stopped(
                agent_browser_service_model::RouteKeeperStopReceipt {
                    slot_id: slot_id.to_string(),
                    keeper_id: keeper_id.to_string(),
                    fence: fence.clone(),
                    guacamole_connection_uuid: Some(ready.guacamole_connection_uuid.clone()),
                    xrdp_session_id: Some(ready.xrdp_session_id.clone()),
                    stopped_at: "2026-09-19T20:01:00Z".to_string(),
                },
            ))
        }
    }

    #[derive(Clone)]
    struct FakeXrdpHelperTransport {
        calls: Arc<std::sync::Mutex<Vec<String>>>,
        observation: XrdpHelperObservation,
        stop: XrdpHelperStop,
    }

    #[async_trait::async_trait]
    impl XrdpHelperTransport for FakeXrdpHelperTransport {
        async fn observe(&mut self, route_user: &str) -> Result<XrdpHelperObservation, String> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("observe:{route_user}"));
            Ok(self.observation.clone())
        }

        async fn stop_exact(
            &mut self,
            witness: &RouteKeeperXrdpOwnershipWitness,
        ) -> Result<XrdpHelperStop, String> {
            self.calls.lock().unwrap().push(format!(
                "stop:{}:{}:{}",
                witness.route_user, witness.session_id, witness.x_server_start_ticks
            ));
            Ok(self.stop.clone())
        }
    }

    fn exact_xrdp_witness() -> RouteKeeperXrdpOwnershipWitness {
        RouteKeeperXrdpOwnershipWitness {
            schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
            boot_id: "11111111-2222-3333-4444-555555555555".to_string(),
            route_user: "agent-browser-rdp-1".to_string(),
            route_uid: 2001,
            session_id: "c42".to_string(),
            session_service: "xrdp-sesman".to_string(),
            session_scope: "session-c42.scope".to_string(),
            scope_invocation_id: "0123456789abcdef0123456789abcdef".to_string(),
            cgroup_path: "/user.slice/user-2001.slice/session-c42.scope".to_string(),
            cgroup_device: 28,
            cgroup_inode: 1001,
            leader_pid: 4101,
            leader_start_ticks: 5101,
            x_server_pid: 4102,
            x_server_start_ticks: 5102,
            display_name: ":21".to_string(),
            x11_socket_inode: 6101,
        }
    }

    #[test]
    fn installed_xrdp_helper_ready_wire_requires_typed_witness_schema() {
        let ready = serde_json::json!({
            "schemaVersion": 1,
            "state": "ready",
            "witness": exact_xrdp_witness(),
        });
        let parsed = parse_xrdp_helper_response(ready.to_string().as_bytes()).unwrap();
        assert_eq!(
            parsed.witness.unwrap().schema_version,
            "agent-browser.route-keeper-xrdp-ownership.v1"
        );

        let mut missing_schema = ready;
        missing_schema["witness"]
            .as_object_mut()
            .unwrap()
            .remove("schemaVersion");
        assert_eq!(
            parse_xrdp_helper_response(missing_schema.to_string().as_bytes()).unwrap_err(),
            "route_keeper_xrdp_helper_response_invalid"
        );
    }

    #[tokio::test]
    async fn configured_xrdp_observer_uses_catalog_user_and_exact_witness_for_stop() {
        let (_directory, database_path) = database();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let original = store.load_route_keeper_authority().unwrap();
        let mut starting = original.clone();
        let start_action = starting.next_reconcile_action().unwrap();
        let (slot_id, _, fence) = start_identity(&start_action).unwrap();
        let slot_id = slot_id.to_string();
        let fence = fence.clone();
        store
            .compare_and_swap_route_keeper_authority(&original, &starting)
            .unwrap();
        let mut observing = starting.clone();
        observing.record_observing(&slot_id, &fence).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&starting, &observing)
            .unwrap();
        let observe_action = observing.next_reconcile_action().unwrap();
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let helper = FakeXrdpHelperTransport {
            calls: calls.clone(),
            observation: XrdpHelperObservation::Ready(Box::new(exact_xrdp_witness())),
            stop: XrdpHelperStop::Stopped,
        };
        let mut observer = ConfiguredXrdpRouteKeeperObserver::new(database_path.clone(), helper);

        let ready = observer
            .observe_xrdp(&observe_action, "guacamole-connection-42")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ready.xrdp_session_id, "c42");
        assert_eq!(ready.display_name, ":21");
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["observe:agent-browser-rdp-1"]
        );

        let mut ready_authority = observing.clone();
        ready_authority
            .record_protocol_ready(ready.clone())
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&observing, &ready_authority)
            .unwrap();
        let mut stopping = ready_authority.clone();
        let stop_action = stopping.begin_stop(&slot_id, &fence).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&ready_authority, &stopping)
            .unwrap();
        let starts = Arc::new(AtomicUsize::new(0));
        let mut connector = GuacamoleRouteKeeperConnector::new(
            database_path,
            DuplexPrimaryFactory {
                socket: None,
                starts: starts.clone(),
            },
            observer,
        );
        let stopped = connector.resume_stop(&stop_action, &ready).await.unwrap();
        assert!(matches!(stopped, RouteKeeperStopObservation::Stopped(_)));
        assert_eq!(starts.load(Ordering::SeqCst), 0);
        assert!(connector.tasks.is_empty());
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "observe:agent-browser-rdp-1",
                "stop:agent-browser-rdp-1:c42:5102"
            ]
        );
    }

    #[tokio::test]
    async fn configured_xrdp_observer_observes_successor_witness_while_adopting() {
        let (_directory, database_path) = database();
        let (action, predecessor) = adoption_action(&database_path);
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let helper = FakeXrdpHelperTransport {
            calls: calls.clone(),
            observation: XrdpHelperObservation::Ready(Box::new(
                predecessor.xrdp_ownership.clone().unwrap(),
            )),
            stop: XrdpHelperStop::Stopped,
        };
        let mut observer = ConfiguredXrdpRouteKeeperObserver::new(database_path, helper);

        let ready = observer
            .observe_xrdp(&action, "00000000-0000-4000-8000-000000000220")
            .await
            .unwrap()
            .unwrap();
        let (_, _, fence, _) = adoption_identity(&action).unwrap();
        assert_eq!(ready.fence, *fence);
        assert_ne!(
            ready.guacamole_connection_uuid,
            predecessor.guacamole_connection_uuid
        );
        assert_eq!(ready.xrdp_ownership, predecessor.xrdp_ownership);
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["observe:agent-browser-rdp-1"]
        );
    }

    #[tokio::test]
    async fn configured_xrdp_observer_preserves_pending_and_unproven_stop() {
        let (_directory, database_path) = database();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let original = store.load_route_keeper_authority().unwrap();
        let mut starting = original.clone();
        let start_action = starting.next_reconcile_action().unwrap();
        let (slot_id, _, fence) = start_identity(&start_action).unwrap();
        let slot_id = slot_id.to_string();
        let fence = fence.clone();
        store
            .compare_and_swap_route_keeper_authority(&original, &starting)
            .unwrap();
        let mut observing = starting.clone();
        observing.record_observing(&slot_id, &fence).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&starting, &observing)
            .unwrap();
        let observe_action = observing.next_reconcile_action().unwrap();
        let pending_calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let helper = FakeXrdpHelperTransport {
            calls: pending_calls.clone(),
            observation: XrdpHelperObservation::Pending,
            stop: XrdpHelperStop::OwnershipUnproven("scope_not_empty".to_string()),
        };
        let mut observer = ConfiguredXrdpRouteKeeperObserver::new(database_path.clone(), helper);
        assert!(observer
            .observe_xrdp(&observe_action, "guacamole-connection-42")
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            pending_calls.lock().unwrap().as_slice(),
            ["observe:agent-browser-rdp-1"]
        );

        let witness = exact_xrdp_witness();
        let ready = RouteKeeperProtocolReadyReceipt {
            slot_id: slot_id.clone(),
            keeper_id: observing.records[&slot_id].keeper_id.clone(),
            fence: fence.clone(),
            guacamole_connection_uuid: "guacamole-connection-42".to_string(),
            xrdp_session_id: witness.session_id.clone(),
            display_name: witness.display_name.clone(),
            xrdp_ownership: Some(witness),
            observed_at: "2026-09-19T20:00:00Z".to_string(),
        };
        let mut ready_authority = observing.clone();
        ready_authority
            .record_protocol_ready(ready.clone())
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&observing, &ready_authority)
            .unwrap();
        let mut stopping = ready_authority.clone();
        let stop_action = stopping.begin_stop(&slot_id, &fence).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&ready_authority, &stopping)
            .unwrap();

        let stop_calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let helper = FakeXrdpHelperTransport {
            calls: stop_calls.clone(),
            observation: XrdpHelperObservation::Pending,
            stop: XrdpHelperStop::OwnershipUnproven("scope_not_empty".to_string()),
        };
        let mut observer = ConfiguredXrdpRouteKeeperObserver::new(database_path.clone(), helper);
        let result = observer.stop_exact(&stop_action, &ready).await.unwrap();
        assert!(matches!(
            result,
            RouteKeeperStopObservation::OwnershipUnproven {
                preserved_observed_keeper_id
            } if preserved_observed_keeper_id == "scope_not_empty"
        ));
        assert_eq!(
            stop_calls.lock().unwrap().as_slice(),
            ["stop:agent-browser-rdp-1:c42:5102"]
        );

        let mut mismatched_ready = ready;
        mismatched_ready.guacamole_connection_uuid = "guacamole-replacement".to_string();
        let mismatch_calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let helper = FakeXrdpHelperTransport {
            calls: mismatch_calls.clone(),
            observation: XrdpHelperObservation::Pending,
            stop: XrdpHelperStop::Stopped,
        };
        let mut observer = ConfiguredXrdpRouteKeeperObserver::new(database_path, helper);
        let result = observer
            .stop_exact(&stop_action, &mismatched_ready)
            .await
            .unwrap();
        assert!(matches!(
            result,
            RouteKeeperStopObservation::OwnershipUnproven {
                preserved_observed_keeper_id
            } if preserved_observed_keeper_id == "route_keeper_xrdp_ready_receipt_mismatch"
        ));
        assert!(mismatch_calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn configured_xrdp_observer_rejects_stale_catalog_fence_before_helper_call() {
        let (_directory, database_path) = database();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let original = store.load_route_keeper_authority().unwrap();
        let mut starting = original.clone();
        let start_action = starting.next_reconcile_action().unwrap();
        let (slot_id, _, fence) = start_identity(&start_action).unwrap();
        let slot_id = slot_id.to_string();
        let fence = fence.clone();
        store
            .compare_and_swap_route_keeper_authority(&original, &starting)
            .unwrap();
        let mut observing = starting.clone();
        observing.record_observing(&slot_id, &fence).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&starting, &observing)
            .unwrap();
        let mut stale_action = observing.next_reconcile_action().unwrap();
        if let RouteKeeperReconcileAction::Observe { fence, .. } = &mut stale_action {
            fence.connection_catalog_digest = "0".repeat(64);
        } else {
            panic!("expected observe action");
        }
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let helper = FakeXrdpHelperTransport {
            calls: calls.clone(),
            observation: XrdpHelperObservation::Ready(Box::new(exact_xrdp_witness())),
            stop: XrdpHelperStop::Stopped,
        };
        let mut observer = ConfiguredXrdpRouteKeeperObserver::new(database_path, helper);
        assert_eq!(
            observer
                .observe_xrdp(&stale_action, "guacamole-connection-42")
                .await
                .unwrap_err(),
            "route_keeper_xrdp_fence_stale"
        );
        assert!(calls.lock().unwrap().is_empty());
    }

    async fn duplex_pair() -> (
        WebSocketStream<tokio::io::DuplexStream>,
        WebSocketStream<tokio::io::DuplexStream>,
    ) {
        let (client, server) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(
            client,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;
        let server = WebSocketStream::from_raw_socket(
            server,
            tokio_tungstenite::tungstenite::protocol::Role::Server,
            None,
        )
        .await;
        (client, server)
    }

    #[tokio::test]
    async fn configured_factory_resolves_only_the_exact_sqlite_catalog_fence() {
        let (_directory, path) = database();
        let mut store = BrowserRuntimeSqliteStore::open(&path).unwrap();
        let expected = store.load_route_keeper_authority().unwrap();
        let mut starting = expected.clone();
        let action = starting.next_reconcile_action().unwrap();
        store
            .compare_and_swap_route_keeper_authority(&expected, &starting)
            .unwrap();
        let mut factory = ConfiguredRouteKeeperPrimaryFactory::new(path);
        let stale_action = match action.clone() {
            RouteKeeperReconcileAction::Start {
                slot_id,
                keeper_id,
                mut fence,
                priority,
            } => {
                fence.connection_catalog_digest = "0".repeat(64);
                RouteKeeperReconcileAction::Start {
                    slot_id,
                    keeper_id,
                    fence,
                    priority,
                }
            }
            other => panic!("expected start action, got {other:?}"),
        };
        let guard: PrimaryGuard = Arc::new(|| Ok(()));
        assert_eq!(
            factory
                .start(&stale_action, guard.clone(), Box::new(|_, _, _| {}))
                .err(),
            Some("route_keeper_primary_catalog_fence_stale".to_string())
        );
        let mut task = factory
            .start(&action, guard, Box::new(|_, _, _| {}))
            .unwrap();
        task.close().await;
    }

    async fn wait_for_primary_ready<F, O>(
        connector: &GuacamoleRouteKeeperConnector<F, O>,
        slot_id: &str,
    ) where
        F: RouteKeeperPrimaryFactory,
        O: RouteKeeperProtocolObserver,
    {
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if matches!(
                    connector.tasks[slot_id].task.status(),
                    PrimaryStatus::Ready(_)
                ) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    fn protocol_ready(
        slot_id: &str,
        keeper_id: &str,
        fence: RouteKeeperFence,
        guacamole_connection_uuid: &str,
    ) -> RouteKeeperProtocolReadyReceipt {
        let xrdp_session_id = format!("xrdp-{slot_id}");
        RouteKeeperProtocolReadyReceipt {
            slot_id: slot_id.to_string(),
            keeper_id: keeper_id.to_string(),
            fence,
            guacamole_connection_uuid: guacamole_connection_uuid.to_string(),
            xrdp_session_id: xrdp_session_id.clone(),
            display_name: ":10".to_string(),
            xrdp_ownership: Some(RouteKeeperXrdpOwnershipWitness {
                schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
                boot_id: "boot-fixture".to_string(),
                route_user: "agent-browser-rdp-1".to_string(),
                route_uid: 2001,
                session_id: xrdp_session_id.clone(),
                session_service: "xrdp-sesman".to_string(),
                session_scope: format!("session-{xrdp_session_id}.scope"),
                scope_invocation_id: "invocation-fixture".to_string(),
                cgroup_path: format!("/user.slice/user-2001.slice/session-{xrdp_session_id}.scope"),
                cgroup_device: 28,
                cgroup_inode: 1001,
                leader_pid: 4101,
                leader_start_ticks: 5101,
                x_server_pid: 4102,
                x_server_start_ticks: 5102,
                display_name: ":10".to_string(),
                x11_socket_inode: 6101,
            }),
            observed_at: "2026-09-21T12:00:00Z".to_string(),
        }
    }

    fn adoption_action(
        database_path: &Path,
    ) -> (RouteKeeperReconcileAction, RouteKeeperProtocolReadyReceipt) {
        let mut store = BrowserRuntimeSqliteStore::open(database_path).unwrap();
        let original = store.load_route_keeper_authority().unwrap();
        let mut starting = original.clone();
        let start = starting.next_reconcile_action().unwrap();
        let (slot_id, keeper_id, start_fence) = start_identity(&start).unwrap();
        let slot_id = slot_id.to_string();
        let keeper_id = keeper_id.to_string();
        let start_fence = start_fence.clone();
        starting.record_observing(&slot_id, &start_fence).unwrap();
        let predecessor = protocol_ready(
            &slot_id,
            &keeper_id,
            start_fence.clone(),
            "00000000-0000-4000-8000-000000000218",
        );
        starting.record_protocol_ready(predecessor.clone()).unwrap();
        starting
            .record_disconnect(
                &slot_id,
                &start_fence,
                &predecessor.guacamole_connection_uuid,
            )
            .unwrap();
        let mut successor_claim = host_process_claim();
        successor_claim.host_generation = 2;
        successor_claim.process_identity.pid = 4_002;
        starting
            .register_host_process_claim(successor_claim)
            .unwrap();
        let action = starting.begin_adoption(&slot_id, 2).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&original, &starting)
            .unwrap();
        (action, predecessor)
    }

    #[tokio::test]
    async fn adoption_reuses_one_task_and_returns_fresh_transport_occurrence() {
        let (_directory, path) = database();
        let (action, predecessor) = adoption_action(&path);
        let (client, mut server) = duplex_pair().await;
        let starts = Arc::new(AtomicUsize::new(0));
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path.clone(),
            SequencedDuplexPrimaryFactory {
                sockets: VecDeque::from([client]),
                starts: starts.clone(),
            },
            ExactProtocolObserver::default(),
        );

        assert_eq!(
            connector.adopt(&action).await.unwrap(),
            RouteKeeperAdoptionObservation::Pending
        );
        connector.adopt(&action).await.unwrap();
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000219;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        wait_for_primary_ready(&connector, "route-slot-01").await;
        let receipt = match connector.adopt(&action).await.unwrap() {
            RouteKeeperAdoptionObservation::Adopted(receipt) => *receipt,
            RouteKeeperAdoptionObservation::Pending => panic!("expected adopted receipt"),
        };
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert_eq!(
            receipt.previous_guacamole_connection_uuid,
            predecessor.guacamole_connection_uuid
        );
        assert_ne!(
            receipt.ready.guacamole_connection_uuid,
            receipt.previous_guacamole_connection_uuid
        );
        assert_eq!(
            receipt.ready.xrdp_ownership, predecessor.xrdp_ownership,
            "the service model receives the exact successor witness for continuity validation"
        );

        let mut store = BrowserRuntimeSqliteStore::open(&path).unwrap();
        let adopting = store.load_route_keeper_authority().unwrap();
        let mut adopted = adopting.clone();
        adopted.adopt(receipt).unwrap();
        store
            .compare_and_swap_route_keeper_authority(&adopting, &adopted)
            .unwrap();
    }

    #[tokio::test]
    async fn stale_adoption_action_cannot_start_provider_transport() {
        let (_directory, path) = database();
        let (mut action, _) = adoption_action(&path);
        if let RouteKeeperReconcileAction::Adopt { fence, .. } = &mut action {
            fence.connection_catalog_digest = "0".repeat(64);
        }
        let starts = Arc::new(AtomicUsize::new(0));
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path,
            DuplexPrimaryFactory {
                socket: None,
                starts: starts.clone(),
            },
            ExactProtocolObserver::default(),
        );

        assert_eq!(
            connector.adopt(&action).await,
            Err("route_keeper_primary_catalog_fence_stale".to_string())
        );
        assert_eq!(starts.load(Ordering::SeqCst), 0);
        assert!(connector.tasks.is_empty());
    }

    #[tokio::test]
    async fn connector_retains_real_primary_until_exact_protocol_ready_and_stop() {
        let (_directory, path) = database();
        let repository = SqliteRouteKeeperRepository::new(&path);
        let (client, mut server) = duplex_pair().await;
        let starts = Arc::new(AtomicUsize::new(0));
        let factory = DuplexPrimaryFactory {
            socket: Some(client),
            starts: starts.clone(),
        };
        let mut connector =
            GuacamoleRouteKeeperConnector::new(path, factory, ExactProtocolObserver::default());

        let observing = reconcile_once(&repository, &mut connector).await.unwrap();
        assert_eq!(observing.ready_count, 0);
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert_eq!(connector.tasks.len(), 1);
        let occurrence_id = connector.tasks["route-slot-01"].occurrence_id.clone();

        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000211;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        wait_for_primary_ready(&connector, "route-slot-01").await;
        let ready = reconcile_once(&repository, &mut connector).await.unwrap();
        assert_eq!(ready.ready_count, 1);
        assert!(ready.minimum_satisfied);
        assert_eq!(connector.observer.observations, 1);
        let receipt = repository.load_route_keeper_authority().unwrap().records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        assert_eq!(
            receipt.guacamole_connection_uuid,
            "00000000-0000-4000-8000-000000000211"
        );
        assert_eq!(receipt.xrdp_session_id, "xrdp-route-slot-01");

        let stopped = stop_once(&repository, &mut connector, "route-slot-01")
            .await
            .unwrap();
        assert_eq!(stopped.keeper_count, 0);
        assert!(connector.tasks.is_empty());
        assert_eq!(connector.observer.stops, 1);
        let events = connector.take_terminal_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].slot_id, "route-slot-01");
        assert_eq!(events[0].occurrence_id, occurrence_id);
        assert!(matches!(
            events[0].code,
            "guacamole_primary_stopped" | "guacamole_route_keeper_fence_stale"
        ));
    }

    struct RejectSecondCasRepository {
        inner: SqliteRouteKeeperRepository,
        calls: AtomicUsize,
    }

    impl RouteKeeperRepository for RejectSecondCasRepository {
        fn load_route_keeper_authority(
            &self,
        ) -> Result<agent_browser_service_model::RouteKeeperAuthority, String> {
            self.inner.load_route_keeper_authority()
        }

        fn compare_and_swap_route_keeper_authority(
            &self,
            expected: &agent_browser_service_model::RouteKeeperAuthority,
            next: &agent_browser_service_model::RouteKeeperAuthority,
        ) -> Result<(), String> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
                return Err("route_keeper_authority_compare_and_swap_conflict".to_string());
            }
            self.inner
                .compare_and_swap_route_keeper_authority(expected, next)
        }
    }

    #[tokio::test]
    async fn completed_stop_replays_exact_receipt_after_final_cas_conflict() {
        let (_directory, path) = database();
        let repository = SqliteRouteKeeperRepository::new(&path);
        let (client, mut server) = duplex_pair().await;
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path.clone(),
            DuplexPrimaryFactory {
                socket: Some(client),
                starts: Arc::new(AtomicUsize::new(0)),
            },
            ExactProtocolObserver::default(),
        );
        reconcile_once(&repository, &mut connector).await.unwrap();
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000213;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        wait_for_primary_ready(&connector, "route-slot-01").await;
        reconcile_once(&repository, &mut connector).await.unwrap();

        let conflicting = RejectSecondCasRepository {
            inner: SqliteRouteKeeperRepository::new(&path),
            calls: AtomicUsize::new(0),
        };
        assert_eq!(
            stop_once(&conflicting, &mut connector, "route-slot-01").await,
            Err("route_keeper_authority_compare_and_swap_conflict".to_string())
        );
        assert_eq!(connector.observer.stops, 1);
        assert!(connector.tasks.is_empty());
        assert_eq!(connector.completed_stops.len(), 1);
        assert_eq!(
            conflicting.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Stopping
        );

        let projection = reconcile_once(&conflicting, &mut connector).await.unwrap();
        assert_eq!(projection.keeper_count, 0);
        assert_eq!(connector.observer.stops, 1);
        assert_eq!(
            conflicting.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Absent
        );
    }

    #[tokio::test]
    async fn stop_observation_error_retries_without_joining_primary_twice() {
        let (_directory, path) = database();
        let repository = SqliteRouteKeeperRepository::new(&path);
        let (client, mut server) = duplex_pair().await;
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path,
            DuplexPrimaryFactory {
                socket: Some(client),
                starts: Arc::new(AtomicUsize::new(0)),
            },
            ExactProtocolObserver::default(),
        );
        reconcile_once(&repository, &mut connector).await.unwrap();
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000215;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        wait_for_primary_ready(&connector, "route-slot-01").await;
        reconcile_once(&repository, &mut connector).await.unwrap();
        connector.observer.fail_stop_once = true;

        assert_eq!(
            stop_once(&repository, &mut connector, "route-slot-01").await,
            Err("fixture_stop_observation_failed".to_string())
        );
        assert!(
            !connector.tasks["route-slot-01"].task_closed,
            "the primary must remain owned until exact XRDP stop succeeds"
        );
        assert_eq!(connector.observer.stops, 1);
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Stopping
        );

        let projection = reconcile_once(&repository, &mut connector).await.unwrap();
        assert_eq!(projection.keeper_count, 0);
        assert_eq!(connector.observer.stops, 2);
        assert!(connector.tasks.is_empty());
    }

    #[tokio::test]
    async fn stale_stop_action_cannot_reach_destructive_protocol_observer() {
        let (_directory, path) = database();
        let repository = SqliteRouteKeeperRepository::new(&path);
        let (client, mut server) = duplex_pair().await;
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path,
            DuplexPrimaryFactory {
                socket: Some(client),
                starts: Arc::new(AtomicUsize::new(0)),
            },
            ExactProtocolObserver::default(),
        );
        reconcile_once(&repository, &mut connector).await.unwrap();
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000214;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        wait_for_primary_ready(&connector, "route-slot-01").await;
        reconcile_once(&repository, &mut connector).await.unwrap();

        let ready = repository.load_route_keeper_authority().unwrap();
        let mut stopping = ready.clone();
        let ready_receipt = stopping.records["route-slot-01"]
            .protocol_ready
            .clone()
            .unwrap();
        let action = stopping
            .begin_stop("route-slot-01", &ready_receipt.fence)
            .unwrap();
        repository
            .compare_and_swap_route_keeper_authority(&ready, &stopping)
            .unwrap();
        let mut absent = stopping.clone();
        absent
            .record_stopped(agent_browser_service_model::RouteKeeperStopReceipt {
                slot_id: ready_receipt.slot_id.clone(),
                keeper_id: ready_receipt.keeper_id.clone(),
                fence: ready_receipt.fence.clone(),
                guacamole_connection_uuid: Some(ready_receipt.guacamole_connection_uuid.clone()),
                xrdp_session_id: Some(ready_receipt.xrdp_session_id.clone()),
                stopped_at: "2026-09-19T20:02:00Z".to_string(),
            })
            .unwrap();
        repository
            .compare_and_swap_route_keeper_authority(&stopping, &absent)
            .unwrap();

        assert_eq!(
            connector.stop(&action).await,
            Err("guacamole_route_keeper_stop_fence_stale".to_string())
        );
        assert_eq!(connector.observer.stops, 0);
        assert!(connector.completed_stops.is_empty());
        assert_eq!(
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].phase,
            RouteKeeperPhase::Absent
        );
    }

    #[tokio::test]
    async fn exact_start_replay_reuses_task_and_conflicting_fence_preserves_owner() {
        let (_directory, path) = database();
        let mut store = BrowserRuntimeSqliteStore::open(Path::new(&path)).unwrap();
        let expected = store.load_route_keeper_authority().unwrap();
        let mut starting = expected.clone();
        let action = starting.next_reconcile_action().unwrap();
        store
            .compare_and_swap_route_keeper_authority(&expected, &starting)
            .unwrap();
        let (client, _server) = duplex_pair().await;
        let starts = Arc::new(AtomicUsize::new(0));
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path,
            DuplexPrimaryFactory {
                socket: Some(client),
                starts: starts.clone(),
            },
            ExactProtocolObserver::default(),
        );

        connector.start(&action).await.unwrap();
        let occurrence_id = connector.tasks["route-slot-01"].occurrence_id.clone();
        connector.start(&action).await.unwrap();
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert_eq!(
            connector.tasks["route-slot-01"].occurrence_id,
            occurrence_id
        );

        let mut conflicting = action.clone();
        if let RouteKeeperReconcileAction::Start { fence, .. } = &mut conflicting {
            fence.operation_generation += 1;
        }
        assert_eq!(
            connector.start(&conflicting).await,
            Err("route_keeper_primary_slot_owned_by_other_fence".to_string())
        );
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert_eq!(
            connector.tasks["route-slot-01"].occurrence_id,
            occurrence_id
        );
        connector
            .tasks
            .get_mut("route-slot-01")
            .unwrap()
            .task
            .close()
            .await;
    }

    #[tokio::test]
    async fn closed_primary_emits_one_terminal_event_and_never_republishes_cached_ready() {
        let (_directory, path) = database();
        let repository = SqliteRouteKeeperRepository::new(&path);
        let (client, mut server) = duplex_pair().await;
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path,
            DuplexPrimaryFactory {
                socket: Some(client),
                starts: Arc::new(AtomicUsize::new(0)),
            },
            ExactProtocolObserver::default(),
        );
        reconcile_once(&repository, &mut connector).await.unwrap();
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000212;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        wait_for_primary_ready(&connector, "route-slot-01").await;
        reconcile_once(&repository, &mut connector).await.unwrap();
        server.close(None).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if matches!(
                    connector.tasks["route-slot-01"].task.status(),
                    PrimaryStatus::Closed(_)
                ) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let record =
            repository.load_route_keeper_authority().unwrap().records["route-slot-01"].clone();
        let action = RouteKeeperReconcileAction::Observe {
            slot_id: record.slot_id,
            keeper_id: record.keeper_id,
            fence: record.fence,
        };
        assert_eq!(
            connector.observe(&action).await,
            Ok(RouteKeeperConnectorObservation::Pending)
        );
        let events = connector.take_terminal_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].guacamole_connection_uuid.as_deref(),
            Some("00000000-0000-4000-8000-000000000212")
        );
        let mut stale = events[0].clone();
        stale.occurrence_id = "stale-occurrence".to_string();
        connector.acknowledge_terminal_event(&stale);
        assert_eq!(connector.tasks.len(), 1);
        connector.acknowledge_terminal_event(&events[0]);
        assert!(connector.tasks.is_empty());
        assert!(connector.take_terminal_events().is_empty());
    }

    #[tokio::test]
    async fn supervisor_shutdown_closes_owned_primary_once_without_xrdp_stop() {
        let (_directory, path) = database();
        let repository = SqliteRouteKeeperRepository::new(&path);
        let (client, _server) = duplex_pair().await;
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path,
            DuplexPrimaryFactory {
                socket: Some(client),
                starts: Arc::new(AtomicUsize::new(0)),
            },
            ExactProtocolObserver::default(),
        );
        reconcile_once(&repository, &mut connector).await.unwrap();
        assert_eq!(connector.tasks.len(), 1);

        connector.shutdown_primaries().await;
        connector.shutdown_primaries().await;
        assert_eq!(connector.tasks.len(), 1);
        assert_eq!(connector.observer.stops, 0);
        let events = connector.take_terminal_events();
        assert_eq!(events.len(), 1);
        assert!(events[0].guacamole_connection_uuid.is_none());
        connector.acknowledge_terminal_event(&events[0]);
        assert!(connector.tasks.is_empty());
    }

    #[tokio::test]
    async fn supervisor_shutdown_persists_ready_primary_disconnect_before_release() {
        let (_directory, path) = database();
        let repository = SqliteRouteKeeperRepository::new(&path);
        let (client, mut server) = duplex_pair().await;
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path,
            DuplexPrimaryFactory {
                socket: Some(client),
                starts: Arc::new(AtomicUsize::new(0)),
            },
            ExactProtocolObserver::default(),
        );
        reconcile_once(&repository, &mut connector).await.unwrap();
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000216;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        wait_for_primary_ready(&connector, "route-slot-01").await;
        reconcile_once(&repository, &mut connector).await.unwrap();

        let (_tick_tx, mut ticks) = mpsc::channel(1);
        let (shutdown_tx, mut shutdown) = tokio::sync::watch::channel(false);
        let signal = tokio::spawn(async move {
            tokio::task::yield_now().await;
            shutdown_tx.send(true).unwrap();
        });
        run_route_keeper_supervisor(&repository, &mut connector, &mut ticks, &mut shutdown)
            .await
            .unwrap();
        signal.await.unwrap();

        assert!(connector.tasks.is_empty());
        assert_eq!(connector.observer.stops, 0);
        let authority = repository.load_route_keeper_authority().unwrap();
        assert_eq!(
            authority.records["route-slot-01"].phase,
            RouteKeeperPhase::Degraded
        );
        assert_eq!(authority.projection().unwrap().ready_count, 0);
    }

    #[tokio::test]
    async fn shutdown_during_exact_stop_quarantines_before_releasing_task() {
        let (_directory, path) = database();
        let repository = SqliteRouteKeeperRepository::new(&path);
        let (client, mut server) = duplex_pair().await;
        let stop_entered = Arc::new(tokio::sync::Notify::new());
        let mut connector = GuacamoleRouteKeeperConnector::new(
            path,
            DuplexPrimaryFactory {
                socket: Some(client),
                starts: Arc::new(AtomicUsize::new(0)),
            },
            ExactProtocolObserver {
                block_stop: true,
                stop_entered: Some(stop_entered.clone()),
                ..ExactProtocolObserver::default()
            },
        );
        reconcile_once(&repository, &mut connector).await.unwrap();
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000217;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        wait_for_primary_ready(&connector, "route-slot-01").await;
        reconcile_once(&repository, &mut connector).await.unwrap();
        let ready = repository.load_route_keeper_authority().unwrap();
        let mut stopping = ready.clone();
        let fence = stopping.records["route-slot-01"].fence.clone();
        stopping.begin_stop("route-slot-01", &fence).unwrap();
        repository
            .compare_and_swap_route_keeper_authority(&ready, &stopping)
            .unwrap();

        let (_tick_tx, mut ticks) = mpsc::channel(1);
        let (shutdown_tx, mut shutdown) = tokio::sync::watch::channel(false);
        let signal = tokio::spawn(async move {
            stop_entered.notified().await;
            shutdown_tx.send(true).unwrap();
        });
        run_route_keeper_supervisor(&repository, &mut connector, &mut ticks, &mut shutdown)
            .await
            .unwrap();
        signal.await.unwrap();

        assert!(connector.tasks.is_empty());
        let authority = repository.load_route_keeper_authority().unwrap();
        let record = &authority.records["route-slot-01"];
        assert_eq!(record.phase, RouteKeeperPhase::Quarantined);
        let obligation = record.cleanup_obligation.as_ref().unwrap();
        assert_eq!(obligation.reason, "route_keeper_stop_ownership_unproven");
        assert!(obligation
            .preserved_observed_keeper_id
            .contains("guacamole=00000000-0000-4000-8000-000000000217"));
        assert!(obligation
            .preserved_observed_keeper_id
            .contains("xrdp=xrdp-route-slot-01"));
    }

    #[tokio::test]
    async fn primary_task_writes_only_while_exact_sqlite_keeper_fence_is_current() {
        let (_directory, path) = database();
        let mut store = BrowserRuntimeSqliteStore::open(Path::new(&path)).unwrap();
        let expected = store.load_route_keeper_authority().unwrap();
        let mut starting = expected.clone();
        let action = starting.next_reconcile_action().unwrap();
        let (slot_id, keeper_id, fence) = match action {
            agent_browser_service_model::RouteKeeperReconcileAction::Start {
                slot_id,
                keeper_id,
                fence,
                ..
            } => (slot_id, keeper_id, fence),
            other => panic!("expected start, got {other:?}"),
        };
        store
            .compare_and_swap_route_keeper_authority(&expected, &starting)
            .unwrap();

        let guard =
            sqlite_route_keeper_guard(path.clone(), slot_id.clone(), keeper_id, fence.clone());
        let (client, server) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(
            client,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;
        let mut server = WebSocketStream::from_raw_socket(
            server,
            tokio_tungstenite::tungstenite::protocol::Role::Server,
            None,
        )
        .await;
        let mut task = PrimaryTask::spawn(client, guard);
        server
            .send(Message::Text(
                "0.,36.00000000-0000-4000-8000-000000000211;4.sync,1.1;".into(),
            ))
            .await
            .unwrap();
        task.ready().await.unwrap();
        assert_eq!(
            task.status(),
            PrimaryStatus::Ready("00000000-0000-4000-8000-000000000211".to_string())
        );

        let expected = store.load_route_keeper_authority().unwrap();
        let mut superseding = expected.clone();
        let record = superseding.records.get_mut(&slot_id).unwrap();
        record.fence.operation_generation += 1;
        record.fence.operation_id = format!(
            "route-keeper:{}:{}:{}",
            record.fence.host_generation, record.slot_id, record.fence.operation_generation
        );
        store
            .compare_and_swap_route_keeper_authority(&expected, &superseding)
            .unwrap();
        server
            .send(Message::Text("4.sync,1.2;".into()))
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while !matches!(task.status(), PrimaryStatus::Closed(_)) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            task.status(),
            PrimaryStatus::Closed("guacamole_route_keeper_fence_stale")
        );
        while let Ok(Some(Ok(message))) =
            tokio::time::timeout(std::time::Duration::from_millis(25), server.next()).await
        {
            if let Message::Text(text) = message {
                assert_ne!(text, "4.sync,1.2;");
            }
        }
        task.close().await;
    }
}
