//! Backend-scoped primary ownership. Failed attempts remain explicit and sticky.

use super::guacamole_primary_binding::PrimaryBinding;
use super::guacamole_primary_provider;
use super::guacamole_primary_transport::{PrimaryStatus, PrimaryTask};
use crate::native::service_store::LockedServiceStateRepository;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

#[derive(Default)]
struct Registry {
    owners: HashMap<(String, String), (PrimaryBinding, Arc<PrimaryTask>)>,
}

pub(super) async fn ensure(route_id: &str, connection_id: &str) -> Result<String, &'static str> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    let repository = LockedServiceStateRepository::default_json()
        .map_err(|_| "guacamole_primary_state_unavailable")?;
    let binding = PrimaryBinding::resolve(&repository, route_id, connection_id)?;
    let task = {
        let mut registry = REGISTRY
            .get_or_init(|| Mutex::new(Registry::default()))
            .lock()
            .await;
        registry.admit(binding, |binding| {
            let expected = binding.clone();
            let is_current: Arc<dyn Fn() -> bool + Send + Sync> =
                Arc::new(move || expected.is_current(&repository));
            PrimaryTask::connect(
                guacamole_primary_provider::connect(binding.clone(), is_current.clone()),
                is_current,
            )
        })?
    };
    task.ready().await?;
    match task.status() {
        PrimaryStatus::Ready(id) => Ok(id),
        PrimaryStatus::Closed(code) => Err(code),
        PrimaryStatus::Starting => Err("guacamole_primary_not_ready"),
    }
}

impl Registry {
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
            Arc::new(|| true),
        )
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
