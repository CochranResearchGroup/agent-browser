//! Route-keeper ownership for the provider-neutral Guacamole primary task.

use super::guacamole_primary_transport::PrimaryGuard;
use crate::native::browser_session_store::BrowserRuntimeSqliteStore;
use agent_browser_service_model::{RouteKeeperFence, RouteKeeperPhase};
use std::path::PathBuf;
use std::sync::Arc;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::browser_session_store::LegacyBrowserRuntimeSources;
    use crate::native::stream::guacamole_primary_transport::{PrimaryStatus, PrimaryTask};
    use futures_util::{SinkExt, StreamExt};
    use std::fs;
    use std::path::Path;
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
        (directory, path)
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
