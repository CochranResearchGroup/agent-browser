//! Route-keeper ownership for the provider-neutral Guacamole primary task.

use super::guacamole_primary_provider::{connect_spec, GuacamolePrimaryConnectSpec};
use super::guacamole_primary_transport::{
    check_primary_authority, PrimaryGuard, PrimaryStatus, PrimaryTask,
};
use crate::native::browser_session_store::BrowserRuntimeSqliteStore;
use crate::native::presentation_route_keeper::{
    PresentationRouteConnector, RouteKeeperAdoptionObservation, RouteKeeperConnectorObservation,
    RouteKeeperStopObservation, RouteKeeperTerminalEvent, SupervisedPresentationRouteConnector,
};
use agent_browser_service_model::{
    RouteKeeperFence, RouteKeeperPhase, RouteKeeperProtocolReadyReceipt, RouteKeeperReconcileAction,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

type PrimaryTerminalSink = Box<dyn FnOnce(&str, &'static str, u64) + Send>;

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
    provider_base: String,
}

impl ConfiguredRouteKeeperPrimaryFactory {
    pub fn new(database_path: PathBuf, provider_base: &str) -> Result<Self, String> {
        GuacamolePrimaryConnectSpec::from_local_embed(provider_base, "catalog-validation")
            .map_err(str::to_string)?;
        Ok(Self {
            database_path,
            provider_base: provider_base.to_string(),
        })
    }
}

impl RouteKeeperPrimaryFactory for ConfiguredRouteKeeperPrimaryFactory {
    fn start(
        &mut self,
        action: &RouteKeeperReconcileAction,
        guard: PrimaryGuard,
        on_closed: PrimaryTerminalSink,
    ) -> Result<PrimaryTask, String> {
        let (slot_id, keeper_id, fence) = start_identity(action)?;
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
        if record.keeper_id != keeper_id
            || record.fence != *fence
            || record.phase != RouteKeeperPhase::Starting
        {
            return Err("route_keeper_primary_fence_stale".to_string());
        }
        let binding = authority
            .connection_catalog
            .bindings
            .get(slot_id)
            .ok_or_else(|| "route_keeper_primary_connection_unconfigured".to_string())?;
        let spec = GuacamolePrimaryConnectSpec::from_local_embed(
            &self.provider_base,
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
    ) -> Result<RouteKeeperProtocolReadyReceipt, String>;

    async fn stop_exact(
        &mut self,
        action: &RouteKeeperReconcileAction,
        ready: &RouteKeeperProtocolReadyReceipt,
    ) -> Result<RouteKeeperStopObservation, String>;
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
                let ready = self
                    .observer
                    .observe_xrdp(action, &guacamole_connection_uuid)
                    .await?;
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
        _action: &RouteKeeperReconcileAction,
    ) -> Result<RouteKeeperAdoptionObservation, String> {
        Err("route_keeper_primary_adoption_not_supported".to_string())
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

        if !owned.task_closed {
            owned.task.close().await;
            owned.task_closed = true;
        }
        let stop_guard = sqlite_route_keeper_stop_guard(
            self.database_path.clone(),
            slot_id.to_string(),
            keeper_id.to_string(),
            fence.clone(),
        );
        check_primary_authority(&stop_guard)
            .await
            .map_err(str::to_string)?;
        match self.observer.stop_exact(action, &ready).await {
            Ok(observation) => {
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
            Err(error) => Err(error),
        }
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
        } => Ok((slot_id, keeper_id, fence)),
        _ => Err("route_keeper_primary_observe_action_required".to_string()),
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
                RouteKeeperPhase::Starting | RouteKeeperPhase::Observing | RouteKeeperPhase::Ready
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
    use agent_browser_service_model::{RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog};
    use futures_util::{SinkExt, StreamExt};
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
            .replace_connection_catalog(
                RouteKeeperConnectionCatalog::new((1_u32..=6).map(|sequence| {
                    RouteKeeperConnectionBinding {
                        slot_id: format!("route-slot-{sequence:02}"),
                        connection_key: format!("route-{sequence:02}"),
                        connection_name: format!("Agent Browser Route {sequence:02}"),
                        guacamole_connection_id: u64::from(sequence),
                    }
                }))
                .unwrap(),
            )
            .unwrap();
        store
            .compare_and_swap_route_keeper_authority(&original, &configured)
            .unwrap();
        (directory, path)
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
        ) -> Result<RouteKeeperProtocolReadyReceipt, String> {
            self.observations += 1;
            let (slot_id, keeper_id, fence) = observe_identity(action)?;
            Ok(RouteKeeperProtocolReadyReceipt {
                slot_id: slot_id.to_string(),
                keeper_id: keeper_id.to_string(),
                fence: fence.clone(),
                guacamole_connection_uuid: guacamole_connection_uuid.to_string(),
                xrdp_session_id: format!("xrdp-{slot_id}"),
                display_name: ":10".to_string(),
                observed_at: "2026-09-19T20:00:00Z".to_string(),
            })
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
        let mut factory =
            ConfiguredRouteKeeperPrimaryFactory::new(path, "http://127.0.0.1:8193/guacamole/")
                .unwrap();
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
        assert!(connector.tasks["route-slot-01"].task_closed);
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
