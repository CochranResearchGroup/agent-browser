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
    // Serialize by provider connection, including when a route is reassigned.
    let key = (binding.provider_base.to_string(), connection_id.to_owned());
    let task = {
        let mut registry = REGISTRY
            .get_or_init(|| Mutex::new(Registry::default()))
            .lock()
            .await;
        if let Some((previous, task)) = registry.owners.get(&key) {
            if previous != &binding {
                task.stop();
                if !matches!(task.status(), PrimaryStatus::Closed(_)) {
                    return Err("guacamole_primary_previous_binding_stopping");
                }
                registry.owners.remove(&key);
            }
        }
        if let Some((_, task)) = registry.owners.get(&key) {
            task.clone()
        } else {
            let expected = binding.clone();
            let is_current: Arc<dyn Fn() -> bool + Send + Sync> =
                Arc::new(move || expected.is_current(&repository));
            let task = Arc::new(PrimaryTask::connect(
                guacamole_primary_provider::connect(binding.clone(), is_current.clone()),
                is_current,
            ));
            registry.owners.insert(key, (binding, task.clone()));
            task
        }
    };
    task.ready().await?;
    match task.status() {
        PrimaryStatus::Ready(id) => Ok(id),
        PrimaryStatus::Closed(code) => Err(code),
        PrimaryStatus::Starting => Err("guacamole_primary_not_ready"),
    }
}
