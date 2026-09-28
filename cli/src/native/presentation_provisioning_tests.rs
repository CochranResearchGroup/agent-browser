use super::browser_session_store::{
    BrowserRuntimeConfigPatch, BrowserRuntimeSqliteStore, LegacyBrowserRuntimeSources,
    PresentationProvisioningConfig, PresentationProvisioningOperation,
    RouteKeeperConnectionCatalogPublication,
};
use super::presentation_provisioning::{
    reconcile_presentation_growth, PresentationProvisioningEffects,
};
use agent_browser_service_model::{RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog};
use std::fs;
use std::path::PathBuf;

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "agent-browser-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn catalog(slots: u32) -> RouteKeeperConnectionCatalog {
    RouteKeeperConnectionCatalog::new((1..=slots).map(|ordinal| RouteKeeperConnectionBinding {
        slot_id: format!("route-slot-{ordinal:02}"),
        connection_key: format!("route-{ordinal}"),
        connection_name: format!("Agent Browser Route {ordinal}"),
        route_user: format!("agent-browser-rdp-{ordinal}"),
        guacamole_connection_id: u64::from(ordinal),
    }))
    .unwrap()
}

fn provisioning_config() -> PresentationProvisioningConfig {
    PresentationProvisioningConfig {
        environment: "development".to_string(),
        compose_project: "agent_browser_dev_presentation".to_string(),
        postgres_container: "agent-browser-dev-postgres".to_string(),
        postgres_user: "agent_browser_dev".to_string(),
        postgres_database: "agent_browser_dev_guacamole".to_string(),
        rdp_host: "host.docker.internal".to_string(),
        rdp_port: 3389,
        route_user_prefix: "agent-browser-rdp-".to_string(),
        connection_key_prefix: "route-".to_string(),
        connection_name_prefix: "Agent Browser Route ".to_string(),
        sharing_profile_prefix: "Agent Browser Shared Session ".to_string(),
        maximum_slots: 7,
        max_connections: 8,
        max_connections_per_user: 8,
    }
}

fn configured_store(label: &str) -> (TempDirectory, PathBuf, BrowserRuntimeSqliteStore) {
    let directory = TempDirectory::new(label);
    let database_path = directory.0.join("runtime.sqlite3");
    BrowserRuntimeSqliteStore::migrate_from_legacy(
        &database_path,
        LegacyBrowserRuntimeSources {
            session_state_path: &directory.0.join("browser-sessions.json"),
            profile_catalog_path: &directory.0.join("browser-profiles.json"),
            service_state_path: &directory.0.join("state.json"),
        },
    )
    .unwrap();
    let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
    assert_eq!(
        store
            .publish_route_keeper_configuration(catalog(6), Some(provisioning_config()))
            .unwrap(),
        RouteKeeperConnectionCatalogPublication::Published
    );
    (directory, database_path, store)
}

#[derive(Default)]
struct FakeProvisioningEffects {
    outcomes: Vec<Result<u64, String>>,
    operations: Vec<(String, String, u32)>,
}

#[async_trait::async_trait]
impl PresentationProvisioningEffects for FakeProvisioningEffects {
    async fn provision(
        &mut self,
        _config: &PresentationProvisioningConfig,
        operation: &PresentationProvisioningOperation,
    ) -> Result<u64, String> {
        self.operations.push((
            operation.operation_id.clone(),
            operation.password.clone(),
            operation.ordinal,
        ));
        self.outcomes.remove(0)
    }
}

#[tokio::test]
async fn provisioning_growth_extends_sqlite_authority_without_rewriting_existing_routes() {
    let (_directory, database_path, mut store) =
        configured_store("presentation-provisioning-growth");
    let before = store.load_route_keeper_authority().unwrap();
    store
        .update_runtime_config(BrowserRuntimeConfigPatch {
            maximum_displays: Some(7),
            warm_target: Some(7),
            ..Default::default()
        })
        .unwrap();
    let mut effects = FakeProvisioningEffects {
        outcomes: vec![Ok(700)],
        ..Default::default()
    };

    reconcile_presentation_growth(&database_path, &mut effects)
        .await
        .unwrap();

    let after = store.load_route_keeper_authority().unwrap();
    assert_eq!(after.policy.maximum_slots, 7);
    assert_eq!(after.policy.warm_target, 7);
    assert_eq!(
        after.records["route-slot-07"].phase,
        agent_browser_service_model::RouteKeeperPhase::Absent
    );
    for (slot_id, record) in before.records {
        assert_eq!(after.records[&slot_id], record);
    }
    assert_eq!(
        after.connection_catalog.bindings["route-slot-07"].guacamole_connection_id,
        700
    );
    assert_eq!(store.load_runtime_config().unwrap().maximum_displays, 7);
    assert_eq!(effects.operations.len(), 1);
    assert_eq!(effects.operations[0].2, 7);
    // Cold installation replays the same seed and coordinates after growth.
    // It must retain the added connection, while an ordinary shrinking catalog
    // remains invalid.
    assert_eq!(
        store
            .publish_route_keeper_configuration(catalog(6), Some(provisioning_config()))
            .unwrap(),
        RouteKeeperConnectionCatalogPublication::Unchanged
    );
    assert_eq!(store.load_route_keeper_authority().unwrap(), after);
    assert!(store
        .publish_route_keeper_connection_catalog(catalog(6))
        .is_err());
}

#[tokio::test]
async fn interrupted_growth_replays_the_same_operation_and_password_after_reopen() {
    let (_directory, database_path, mut store) =
        configured_store("presentation-provisioning-replay");
    store
        .update_runtime_config(BrowserRuntimeConfigPatch {
            maximum_displays: Some(7),
            ..Default::default()
        })
        .unwrap();
    let (_, reserved) = store.reserve_presentation_growth().unwrap().unwrap();
    drop(store);

    let mut effects = FakeProvisioningEffects {
        outcomes: vec![Ok(701)],
        ..Default::default()
    };
    reconcile_presentation_growth(&database_path, &mut effects)
        .await
        .unwrap();

    assert_eq!(
        effects.operations,
        vec![(reserved.operation_id, reserved.password, 7)]
    );
    let reopened = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
    assert_eq!(
        reopened
            .load_route_keeper_authority()
            .unwrap()
            .policy
            .maximum_slots,
        7
    );
    assert_eq!(reopened.presentation_growth_status().unwrap().state, "idle");
}

#[tokio::test]
async fn failed_growth_is_redacted_and_only_explicit_config_updates_rearm_its_bounded_attempts() {
    let (_directory, database_path, mut store) =
        configured_store("presentation-provisioning-failure");
    store
        .update_runtime_config(BrowserRuntimeConfigPatch {
            maximum_displays: Some(7),
            ..Default::default()
        })
        .unwrap();
    let mut effects = FakeProvisioningEffects {
        outcomes: vec![Err("password=untrusted-provider-detail".to_string())],
        ..Default::default()
    };
    reconcile_presentation_growth(&database_path, &mut effects)
        .await
        .unwrap();
    let first = store.presentation_growth_status().unwrap();
    assert_eq!(first.state, "failed");
    assert_eq!(first.attempts, 1);
    assert_eq!(
        first.failure_code.as_deref(),
        Some("presentation_provisioning_provider_failed")
    );
    assert!(!format!("{:?}", first.failure_code).contains("untrusted"));

    store
        .update_runtime_config(BrowserRuntimeConfigPatch {
            maximum_displays: Some(7),
            ..Default::default()
        })
        .unwrap();
    effects
        .outcomes
        .push(Err("another-provider-detail".to_string()));
    reconcile_presentation_growth(&database_path, &mut effects)
        .await
        .unwrap();
    let second = store.presentation_growth_status().unwrap();
    assert_eq!(second.state, "failed");
    assert_eq!(second.attempts, 2);
    assert_eq!(effects.operations.len(), 2);

    store
        .update_runtime_config(BrowserRuntimeConfigPatch {
            maximum_displays: Some(7),
            ..Default::default()
        })
        .unwrap();
    effects
        .outcomes
        .push(Err("third-provider-detail".to_string()));
    reconcile_presentation_growth(&database_path, &mut effects)
        .await
        .unwrap();
    assert_eq!(store.presentation_growth_status().unwrap().attempts, 3);

    store
        .update_runtime_config(BrowserRuntimeConfigPatch {
            maximum_displays: Some(7),
            ..Default::default()
        })
        .unwrap();
    reconcile_presentation_growth(&database_path, &mut effects)
        .await
        .unwrap();
    assert_eq!(store.presentation_growth_status().unwrap().attempts, 3);
    assert_eq!(effects.operations.len(), 3);
}

#[test]
fn growth_publication_rejects_a_catalog_changed_after_the_exact_reservation() {
    let (_directory, _database_path, mut store) =
        configured_store("presentation-provisioning-catalog");
    store
        .update_runtime_config(BrowserRuntimeConfigPatch {
            maximum_displays: Some(7),
            ..Default::default()
        })
        .unwrap();
    let (_, operation) = store.reserve_presentation_growth().unwrap().unwrap();
    let authority = store.load_route_keeper_authority().unwrap();
    let mut changed = authority.connection_catalog.clone();
    changed
        .bindings
        .get_mut("route-slot-01")
        .unwrap()
        .route_user = "agent-browser-rdp-other".to_string();
    assert_eq!(
        store.publish_route_keeper_connection_catalog(changed.clone()),
        Err("presentation_provisioning_catalog_namespace_mismatch".to_string())
    );
    let mut replacement = authority.clone();
    replacement.replace_connection_catalog(changed).unwrap();
    store
        .compare_and_swap_route_keeper_authority(&authority, &replacement)
        .unwrap();

    assert_eq!(
        store.complete_presentation_growth(&operation, 702),
        Err("presentation_provisioning_catalog_changed".to_string())
    );
    let after = store.load_route_keeper_authority().unwrap();
    assert_eq!(after.policy.maximum_slots, 6);
    assert!(!after.records.contains_key("route-slot-07"));
}
