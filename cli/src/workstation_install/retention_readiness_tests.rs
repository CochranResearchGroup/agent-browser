use super::*;

#[cfg(unix)]
#[test]
fn generation_gc_preserves_latest_terminal_rollback_readiness_without_pinning_older_history() {
    use crate::runtime_adoption::UpgradeTransactionState;
    use std::os::unix::fs::symlink;

    let root = env::temp_dir().join(format!("rollback-readiness-{}", uuid::Uuid::new_v4()));
    let paths = install_paths(&root);
    for id in ["selected", "required-old", "older-history"] {
        let generation = paths.generations_dir.join(id);
        fs::create_dir_all(&generation).unwrap();
        fs::write(generation.join("payload"), "sealed fixture").unwrap();
        seal_generation_tree(&generation).unwrap();
    }
    symlink("generations/selected", &paths.current_selector).unwrap();

    let mut latest = new_upgrade_transaction(
        &paths,
        "required-old".to_string(),
        "a".repeat(64),
        "b".repeat(64),
    );
    latest.old_generation_id = Some("required-old".to_string());
    latest.state = UpgradeTransactionState::FailedPreservedOldGeneration;
    latest.checkpoints[0].recorded_at = "2026-09-07T00:00:00Z".to_string();
    let mut older = latest.clone();
    older.transaction_id = "older-transaction".to_string();
    older.old_generation_id = Some("older-history".to_string());
    older.candidate_generation_id = "older-history".to_string();
    older.checkpoints[0].recorded_at = "2026-09-06T00:00:00Z".to_string();
    for transaction in [&latest, &older] {
        write_private_json_atomic(
            &transaction_path(&root, &transaction.transaction_id),
            transaction,
        )
        .unwrap();
    }

    let readiness = || {
        workstation_upgrade_readiness(
            &paths,
            Some("selected"),
            Some(&latest),
            false,
            &serde_json::json!({}),
        )["rollbackReady"]
            .clone()
    };
    assert_eq!(readiness(), true);
    let preview = workstation_generation_gc_locked(&root, &paths, InstallMode::DryRun).unwrap();
    assert_eq!(preview["candidates"], serde_json::json!(["older-history"]));
    let applied = workstation_generation_gc_locked(&root, &paths, InstallMode::Apply).unwrap();
    assert_eq!(applied["removed"], preview["candidates"]);
    assert_eq!(readiness(), true);
    assert!(applied["previousHealthyGenerationId"].is_null());
    assert!(paths.generations_dir.join("required-old").exists());
    assert!(!paths.generations_dir.join("older-history").exists());
    let retained = applied["retained"].as_array().unwrap();
    assert!(retained.iter().any(|row| {
        row["generationId"] == "required-old"
            && row["reasonCodes"] == serde_json::json!(["latest_transaction_rollback_readiness"])
    }));
    remove_generation_tree(&paths.generations_dir.join("required-old")).unwrap();
    assert_eq!(
        readiness(),
        false,
        "missing required rollback must still fail doctor"
    );
    remove_generation_tree(&paths.generations_dir.join("selected")).unwrap();
    fs::remove_dir_all(root).unwrap();
}
