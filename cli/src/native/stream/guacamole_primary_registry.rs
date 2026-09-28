//! Backend-scoped primary ownership. Failed attempts remain sticky until an
//! explicit recovery names the terminal occurrence and revalidates its binding.

use super::guacamole_primary_binding::PrimaryBinding;
use super::guacamole_primary_provider;
use super::guacamole_primary_transport::{PrimaryGuard, PrimaryStatus, PrimaryTask};
use crate::native::browser_session_store::BrowserRuntimeSqliteStore;
use crate::native::service_model::{ServiceState, ViewStreamProvider};
use crate::native::service_store::{
    JsonServiceStateStore, LockedServiceStateRepository, ServiceStateRepository,
};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

#[derive(Default)]
struct Registry {
    owners: HashMap<(String, String), (PrimaryBinding, Arc<PrimaryTask>)>,
}

pub(super) struct PrimaryFailure {
    pub code: &'static str,
    pub terminal_occurrence_id: Option<String>,
}

impl From<&'static str> for PrimaryFailure {
    fn from(code: &'static str) -> Self {
        Self {
            code,
            terminal_occurrence_id: None,
        }
    }
}

pub(super) async fn ensure(route_id: &str, connection_id: &str) -> Result<String, PrimaryFailure> {
    primary(route_id, connection_id, None).await
}

/// Recover only the named terminal attempt after fresh ownership verification.
pub(super) async fn recover(
    route_id: &str,
    connection_id: &str,
    expected_terminal_occurrence_id: &str,
) -> Result<String, PrimaryFailure> {
    primary(
        route_id,
        connection_id,
        Some(expected_terminal_occurrence_id.to_owned()),
    )
    .await
}

async fn primary(
    route_id: &str,
    connection_id: &str,
    expected_terminal: Option<String>,
) -> Result<String, PrimaryFailure> {
    let keeper_route_id = route_id.to_owned();
    let keeper_connection_id = connection_id.to_owned();
    if let Some(active_connection_id) = tokio::task::spawn_blocking(move || {
        runtime_keeper_primary_identity(&keeper_route_id, &keeper_connection_id)
    })
    .await
    .map_err(|_| "guacamole_primary_keeper_observation_failed")??
    {
        return Ok(active_connection_id);
    }
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    let route_id = route_id.to_owned();
    let connection_id = connection_id.to_owned();
    // Admission includes synchronous repository and process reads. Keep the
    // registry decision atomic without blocking unrelated dashboard requests.
    let task = tokio::task::spawn_blocking(move || -> Result<Arc<PrimaryTask>, &'static str> {
        let repository = LockedServiceStateRepository::default_json()
            .map_err(|_| "guacamole_primary_state_unavailable")?;
        let mut registry = REGISTRY
            .get_or_init(|| Mutex::new(Registry::default()))
            .blocking_lock();
        if let Some(expected) = expected_terminal {
            let binding = PrimaryBinding::resolve(&repository, &route_id, &connection_id)?;
            registry.recover(binding, &expected, |binding| {
                start_primary(binding, repository)
            })
        } else if let Some(task) = registry.retained(&route_id, &connection_id, |binding| {
            binding.is_current(&repository)
        })? {
            Ok(task)
        } else {
            let binding = PrimaryBinding::resolve(&repository, &route_id, &connection_id)?;
            registry.admit(binding, |binding| start_primary(binding, repository))
        }
    })
    .await
    .map_err(|_| "guacamole_primary_admission_task_failed")??;
    let result = task.ready().await.and_then(|()| match task.status() {
        PrimaryStatus::Ready(id) => Ok(id),
        PrimaryStatus::Closed(code) => Err(code),
        PrimaryStatus::Starting => Err("guacamole_primary_not_ready"),
    });
    result.map_err(|code| PrimaryFailure {
        code,
        // A waiter timeout does not prove the owner has terminated.
        terminal_occurrence_id: matches!(task.status(), PrimaryStatus::Closed(_))
            .then(|| task.occurrence_id.clone()),
    })
}

fn runtime_keeper_primary_identity(
    route_id: &str,
    connection_id: &str,
) -> Result<Option<String>, &'static str> {
    let service = LockedServiceStateRepository::default_json()
        .map_err(|_| "guacamole_primary_state_unavailable")?
        .load_snapshot()
        .map_err(|_| "guacamole_primary_state_unavailable")?;
    let authority = BrowserRuntimeSqliteStore::default_sqlite()
        .and_then(|store| store.load_route_keeper_authority())
        .map_err(|_| "guacamole_primary_keeper_state_unavailable")?;
    runtime_keeper_primary_identity_from(&service, &authority, route_id, connection_id, |claim| {
        crate::process_identity::current_boot_epoch().as_deref() == Some(&claim.boot_epoch)
            && crate::process_identity::recorded_process_is_running(&claim.process_identity)
                .unwrap_or(false)
    })
}

fn runtime_keeper_primary_identity_from(
    service: &ServiceState,
    authority: &agent_browser_service_model::RouteKeeperAuthority,
    route_id: &str,
    connection_id: &str,
    host_is_current: impl Fn(&agent_browser_service_model::RouteKeeperHostProcessClaim) -> bool,
) -> Result<Option<String>, &'static str> {
    let Some(service_route) = service.remote_view_routes.get(route_id) else {
        return Ok(None);
    };
    if service_route.id != route_id
        || service_route.connection_id.as_deref() != Some(connection_id)
        || service_route.provider != ViewStreamProvider::RdpGateway
        || service_route.provider_mode != "simultaneous_view"
    {
        return Ok(None);
    }
    let mut matching_bindings = authority
        .connection_catalog
        .bindings
        .values()
        .filter(|binding| binding.guacamole_connection_id.to_string() == connection_id);
    let Some(binding) = matching_bindings.next() else {
        return Ok(None);
    };
    if matching_bindings.next().is_some() {
        return Err("guacamole_primary_keeper_binding_ambiguous");
    }
    let record = authority
        .records
        .get(&binding.slot_id)
        .ok_or("guacamole_primary_keeper_route_missing")?;
    if record.phase != agent_browser_service_model::RouteKeeperPhase::Ready {
        return Err("guacamole_primary_keeper_route_unavailable");
    }
    let claim = authority
        .host_process_claims
        .get(&record.fence.host_generation)
        .filter(|claim| host_is_current(claim))
        .ok_or("guacamole_primary_keeper_owner_stale")?;
    if claim.host_generation != record.fence.host_generation {
        return Err("guacamole_primary_keeper_owner_stale");
    }
    let ready = record
        .protocol_ready
        .as_ref()
        .filter(|ready| {
            ready.slot_id == record.slot_id
                && ready.keeper_id == record.keeper_id
                && ready.fence == record.fence
                && ready.xrdp_ownership.is_some()
                && !ready.guacamole_connection_uuid.is_empty()
        })
        .ok_or("guacamole_primary_keeper_receipt_unavailable")?;
    Ok(Some(ready.guacamole_connection_uuid.clone()))
}

fn start_primary(
    binding: &PrimaryBinding,
    repository: LockedServiceStateRepository<JsonServiceStateStore>,
) -> PrimaryTask {
    let expected = binding.clone();
    let is_current: PrimaryGuard = Arc::new(move || expected.verify_current(&repository));
    let evidence_binding = binding.clone();
    PrimaryTask::connect_observed(
        guacamole_primary_provider::connect(binding.clone(), is_current.clone()),
        is_current,
        move |occurrence_id, code, elapsed_ms| {
            evidence_binding.record_terminal(occurrence_id, code, elapsed_ms)
        },
    )
}

impl Registry {
    fn recover(
        &mut self,
        binding: PrimaryBinding,
        expected_terminal: &str,
        start: impl FnOnce(&PrimaryBinding) -> PrimaryTask,
    ) -> Result<Arc<PrimaryTask>, &'static str> {
        let key = (
            binding.provider_base.to_string(),
            binding.connection_id.clone(),
        );
        let (previous, task) = self
            .owners
            .get(&key)
            .ok_or("guacamole_primary_recovery_owner_missing")?;
        if previous != &binding {
            return Err("guacamole_primary_recovery_binding_changed");
        }
        // Duplicate explicit requests coalesce with a starting or live owner.
        // Never stop a live primary as a consequence of viewer retry.
        if !matches!(task.status(), PrimaryStatus::Closed(_)) {
            return Ok(task.clone());
        }
        if task.occurrence_id != expected_terminal {
            return Err("guacamole_primary_recovery_superseded");
        }
        self.owners.remove(&key);
        self.admit(binding, start)
    }

    fn retained(
        &self,
        route_id: &str,
        connection_id: &str,
        is_current: impl Fn(&PrimaryBinding) -> bool,
    ) -> Result<Option<Arc<PrimaryTask>>, &'static str> {
        let mut matches = self.owners.values().filter(|(binding, _)| {
            binding.route_id == route_id
                && binding.connection_id == connection_id
                && is_current(binding)
        });
        let task = matches.next().map(|(_, task)| task.clone());
        if matches.next().is_some() {
            return Err("guacamole_primary_owner_ambiguous");
        }
        Ok(task)
    }

    fn admit(
        &mut self,
        binding: PrimaryBinding,
        start: impl FnOnce(&PrimaryBinding) -> PrimaryTask,
    ) -> Result<Arc<PrimaryTask>, &'static str> {
        // Called under the backend registry mutex; no await separates lookup
        // from insertion. Route reassignment shares the provider-connection key.
        let key = (
            binding.provider_base.to_string(),
            binding.connection_id.clone(),
        );
        if let Some((previous, task)) = self.owners.get(&key) {
            if previous != &binding {
                task.stop();
                if !matches!(task.status(), PrimaryStatus::Closed(_)) {
                    return Err("guacamole_primary_previous_binding_stopping");
                }
                self.owners.remove(&key);
            }
        }
        if let Some((_, task)) = self.owners.get(&key) {
            return Ok(task.clone());
        }
        let task = Arc::new(start(&binding));
        self.owners.insert(key, (binding, task.clone()));
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::RemoteViewRoute;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn pending_owner(starts: &AtomicUsize) -> PrimaryTask {
        starts.fetch_add(1, Ordering::SeqCst);
        PrimaryTask::connect(
            std::future::pending::<
                Result<tokio_tungstenite::WebSocketStream<tokio::io::DuplexStream>, &'static str>,
            >(),
            Arc::new(|| Ok(())),
        )
    }

    #[test]
    fn current_keeper_primary_satisfies_viewer_claim_without_second_start() {
        let mut service = ServiceState::default();
        service.remote_view_routes.insert(
            "provider-route-1".to_string(),
            RemoteViewRoute {
                id: "provider-route-1".to_string(),
                connection_id: Some("41".to_string()),
                provider: ViewStreamProvider::RdpGateway,
                provider_mode: "simultaneous_view".to_string(),
                ..RemoteViewRoute::default()
            },
        );
        let mut authority = agent_browser_service_model::RouteKeeperAuthority::new(7).unwrap();
        authority.connection_catalog =
            agent_browser_service_model::RouteKeeperConnectionCatalog::new([
                agent_browser_service_model::RouteKeeperConnectionBinding {
                    slot_id: "route-slot-01".to_string(),
                    connection_key: "connection-key-1".to_string(),
                    connection_name: "Connection 1".to_string(),
                    route_user: "route-user-1".to_string(),
                    guacamole_connection_id: 41,
                },
            ])
            .unwrap();
        authority.host_process_claims.insert(
            7,
            agent_browser_service_model::RouteKeeperHostProcessClaim {
                host_generation: 7,
                boot_epoch: "boot-fixture".to_string(),
                process_identity: agent_browser_service_model::RecordedProcessIdentity {
                    pid: 4100,
                    start_token: "start-fixture".to_string(),
                    executable_path: Some("/fixture/agent-browser".to_string()),
                    browser_family: None,
                },
            },
        );
        let record = authority.records.get_mut("route-slot-01").unwrap();
        record.phase = agent_browser_service_model::RouteKeeperPhase::Ready;
        record.protocol_ready = Some(
            agent_browser_service_model::RouteKeeperProtocolReadyReceipt {
                slot_id: record.slot_id.clone(),
                keeper_id: record.keeper_id.clone(),
                fence: record.fence.clone(),
                guacamole_connection_uuid: "active-connection-41".to_string(),
                xrdp_session_id: "c41".to_string(),
                display_name: ":41".to_string(),
                xrdp_ownership: Some(
                    agent_browser_service_model::RouteKeeperXrdpOwnershipWitness {
                        schema_version: "agent-browser.route-keeper-xrdp-ownership.v1".to_string(),
                        boot_id: "boot-fixture".to_string(),
                        route_user: "route-user-1".to_string(),
                        route_uid: 2001,
                        session_id: "c41".to_string(),
                        session_service: "xrdp-sesman".to_string(),
                        session_scope: "session-c41.scope".to_string(),
                        scope_invocation_id: "invocation-fixture".to_string(),
                        cgroup_path: "/user.slice/user-2001.slice/session-c41.scope".to_string(),
                        cgroup_device: 28,
                        cgroup_inode: 1001,
                        leader_pid: 4101,
                        leader_start_ticks: 5101,
                        x_server_pid: 4102,
                        x_server_start_ticks: 5102,
                        display_name: ":41".to_string(),
                        x11_socket_inode: 6101,
                    },
                ),
                observed_at: "2026-09-26T21:00:00Z".to_string(),
            },
        );

        assert_eq!(
            runtime_keeper_primary_identity_from(
                &service,
                &authority,
                "provider-route-1",
                "41",
                |_| true,
            )
            .unwrap()
            .as_deref(),
            Some("active-connection-41")
        );
        assert_eq!(
            runtime_keeper_primary_identity_from(
                &service,
                &authority,
                "provider-route-1",
                "41",
                |_| false,
            ),
            Err("guacamole_primary_keeper_owner_stale")
        );
        assert_eq!(
            runtime_keeper_primary_identity_from(
                &service,
                &authority,
                "unknown-route",
                "41",
                |_| true,
            )
            .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn explicit_recovery_is_bound_to_one_terminal_and_preserves_live_owners() {
        let binding = PrimaryBinding::synthetic_fixture();
        let mut registry = Registry::default();
        let starts = AtomicUsize::new(0);
        assert_eq!(
            registry
                .recover(binding.clone(), "missing", |_| pending_owner(&starts))
                .err(),
            Some("guacamole_primary_recovery_owner_missing")
        );
        let first = registry
            .admit(binding.clone(), |_| pending_owner(&starts))
            .unwrap();
        let live = registry
            .recover(binding.clone(), "stale", |_| pending_owner(&starts))
            .unwrap();
        assert!(Arc::ptr_eq(&first, &live));
        let mut changed = binding.clone();
        changed.route_id = "changed".into();
        assert_eq!(
            registry
                .recover(changed, &first.occurrence_id, |_| pending_owner(&starts))
                .err(),
            Some("guacamole_primary_recovery_binding_changed")
        );
        first.stop();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !matches!(first.status(), PrimaryStatus::Closed(_)) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            registry
                .recover(binding.clone(), "stale", |_| pending_owner(&starts))
                .err(),
            Some("guacamole_primary_recovery_superseded")
        );
        let next = registry
            .recover(binding.clone(), &first.occurrence_id, |_| {
                pending_owner(&starts)
            })
            .unwrap();
        let duplicate = registry
            .recover(binding.clone(), &first.occurrence_id, |_| {
                pending_owner(&starts)
            })
            .unwrap();
        assert!(Arc::ptr_eq(&next, &duplicate));
        assert!(!Arc::ptr_eq(&first, &next));
        next.stop();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !matches!(next.status(), PrimaryStatus::Closed(_)) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            registry
                .recover(binding, &first.occurrence_id, |_| pending_owner(&starts))
                .err(),
            Some("guacamole_primary_recovery_superseded")
        );
        assert_eq!(starts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn duplicate_requests_share_startup_and_failed_owner_cannot_restart() {
        let binding = PrimaryBinding::synthetic_fixture();
        let registry = Mutex::new(Registry::default());
        let starts = AtomicUsize::new(0);
        let acquire = || async {
            registry
                .lock()
                .await
                .admit(binding.clone(), |_| pending_owner(&starts))
                .unwrap()
        };
        let (first, second) = tokio::join!(acquire(), acquire());
        assert!(Arc::ptr_eq(&first, &second));
        let locked = registry.lock().await;
        assert!(Arc::ptr_eq(
            &first,
            &locked.retained("route", "1", |_| true).unwrap().unwrap()
        ));
        assert!(locked.retained("route", "1", |_| false).unwrap().is_none());
        assert!(locked
            .retained("foreign-route", "1", |_| true)
            .unwrap()
            .is_none());
        drop(locked);
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        drop(second);
        let mut replacement = binding.clone();
        replacement.route_id = "replacement-route".into();
        assert_eq!(
            registry
                .lock()
                .await
                .admit(replacement.clone(), |_| pending_owner(&starts))
                .err(),
            Some("guacamole_primary_previous_binding_stopping")
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            while !matches!(first.status(), PrimaryStatus::Closed(_)) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let failed = acquire().await;
        assert!(Arc::ptr_eq(&first, &failed));
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        let next = registry
            .lock()
            .await
            .admit(replacement, |_| pending_owner(&starts))
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &next));
        assert_eq!(starts.load(Ordering::SeqCst), 2);
        next.stop();
    }
}
