//! Backend-scoped primary ownership. Failed attempts remain sticky until an
//! explicit recovery names the terminal occurrence and revalidates its binding.

use super::guacamole_primary_binding::{PrimaryBinding, PrimaryGuard};
use super::guacamole_primary_provider;
use super::guacamole_primary_transport::{PrimaryStatus, PrimaryTask};
use crate::native::service_store::{JsonServiceStateStore, LockedServiceStateRepository};
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
