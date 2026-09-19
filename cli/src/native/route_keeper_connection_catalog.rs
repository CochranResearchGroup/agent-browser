//! Bounded provider-to-SQLite publication for the route-keeper connection catalog.

use super::browser_session_store::{
    BrowserRuntimeSqliteStore, RouteKeeperConnectionCatalogPublication,
};
use agent_browser_service_model::{RouteKeeperConnectionBinding, RouteKeeperConnectionCatalog};
use serde::{Deserialize, Serialize};
use std::io::Read;

const ENTRY: &str = "--internal-route-keeper-connection-catalog-publish";
const PUBLICATION_SCHEMA: &str = "agent-browser.route-keeper-connection-catalog-publication.v1";
const RECEIPT_SCHEMA: &str = "agent-browser.route-keeper-connection-catalog-publication-receipt.v1";
const MAX_DOCUMENT_BYTES: u64 = 64 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PublicationDocument {
    schema_version: String,
    bindings: Vec<RouteKeeperConnectionBinding>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationReceipt {
    schema_version: &'static str,
    outcome: &'static str,
    catalog_digest: String,
    binding_count: usize,
}

fn publish_document(
    bytes: &[u8],
    store: &mut BrowserRuntimeSqliteStore,
) -> Result<PublicationReceipt, String> {
    let document: PublicationDocument = serde_json::from_slice(bytes)
        .map_err(|_| "route_keeper_connection_catalog_publication_invalid".to_string())?;
    if document.schema_version != PUBLICATION_SCHEMA {
        return Err("route_keeper_connection_catalog_publication_schema_invalid".to_string());
    }
    let catalog = RouteKeeperConnectionCatalog::new(document.bindings)?;
    let catalog_digest = catalog.digest()?;
    let binding_count = catalog.bindings.len();
    let outcome = match store.publish_route_keeper_connection_catalog(catalog)? {
        RouteKeeperConnectionCatalogPublication::Published => "published",
        RouteKeeperConnectionCatalogPublication::Unchanged => "unchanged",
    };
    Ok(PublicationReceipt {
        schema_version: RECEIPT_SCHEMA,
        outcome,
        catalog_digest,
        binding_count,
    })
}

/// Dispatch the hidden stdin-only publication bridge before normal CLI parsing.
pub(crate) fn run_entry(args: &[String]) -> Option<Result<(), String>> {
    if args.get(1).map(String::as_str) != Some(ENTRY) {
        return None;
    }
    Some((|| {
        if args.len() != 2 {
            return Err(
                "route_keeper_connection_catalog_publication_arguments_invalid".to_string(),
            );
        }
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(MAX_DOCUMENT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "route_keeper_connection_catalog_publication_read_failed".to_string())?;
        if bytes.is_empty() || bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err("route_keeper_connection_catalog_publication_size_invalid".to_string());
        }
        let mut store = BrowserRuntimeSqliteStore::default_sqlite()?;
        let receipt = publish_document(&bytes, &mut store)?;
        println!(
            "{}",
            serde_json::to_string(&receipt)
                .map_err(|_| "route_keeper_connection_catalog_receipt_encode_failed".to_string())?
        );
        Ok(())
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::browser_session_store::{
        BrowserRuntimeSqliteStore, LegacyBrowserRuntimeSources,
    };
    use std::fs;
    use std::path::PathBuf;

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agent-browser-route-keeper-catalog-publication-{}-{}",
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

    #[test]
    fn publication_document_commits_a_complete_catalog_without_echoing_bindings() {
        let directory = TempDirectory::new();
        let database_path = directory.0.join("runtime.sqlite3");
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join("browser-session-state.json"),
                profile_catalog_path: &directory.0.join("browser-profile-catalog.json"),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let document = serde_json::json!({
            "schemaVersion": "agent-browser.route-keeper-connection-catalog-publication.v1",
            "bindings": (1_u32..=6).map(|sequence| serde_json::json!({
                "slotId": format!("route-slot-{sequence:02}"),
                "connectionKey": format!("route-{sequence:02}"),
                "connectionName": format!("Agent Browser Route {sequence:02}"),
                "routeUser": format!("agent-browser-rdp-{sequence}"),
                "guacamoleConnectionId": sequence,
            })).collect::<Vec<_>>()
        });

        let receipt = publish_document(document.to_string().as_bytes(), &mut store).unwrap();
        assert_eq!(receipt.outcome, "published");
        assert_eq!(receipt.binding_count, 6);
        assert_eq!(receipt.catalog_digest.len(), 64);
        let encoded = serde_json::to_string(&receipt).unwrap();
        assert!(!encoded.contains("Agent Browser Route"));
        assert!(!encoded.contains("route-slot"));
    }

    #[test]
    fn publication_rejects_the_wrong_schema_without_mutating_sqlite() {
        let directory = TempDirectory::new();
        let database_path = directory.0.join("runtime.sqlite3");
        BrowserRuntimeSqliteStore::migrate_from_legacy(
            &database_path,
            LegacyBrowserRuntimeSources {
                session_state_path: &directory.0.join("browser-session-state.json"),
                profile_catalog_path: &directory.0.join("browser-profile-catalog.json"),
                service_state_path: &directory.0.join("state.json"),
            },
        )
        .unwrap();
        let mut store = BrowserRuntimeSqliteStore::open(&database_path).unwrap();
        let before = store.load_route_keeper_authority().unwrap();
        let document = serde_json::json!({
            "schemaVersion": "agent-browser.route-keeper-connection-catalog-publication.v0",
            "bindings": []
        });

        assert_eq!(
            publish_document(document.to_string().as_bytes(), &mut store).unwrap_err(),
            "route_keeper_connection_catalog_publication_schema_invalid"
        );
        assert_eq!(store.load_route_keeper_authority().unwrap(), before);
    }

    #[test]
    fn hidden_entry_rejects_trailing_arguments_before_reading_stdin() {
        let args = vec![
            "agent-browser".to_string(),
            ENTRY.to_string(),
            "unexpected".to_string(),
        ];
        assert_eq!(
            run_entry(&args).unwrap(),
            Err("route_keeper_connection_catalog_publication_arguments_invalid".to_string())
        );
    }
}
