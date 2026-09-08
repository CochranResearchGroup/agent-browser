//! Explicit operator retention, independent of upgrade acceptance or health.
use super::{
    install_paths, selected_generation_id, workstation_root, write_private_json_atomic,
    InstallPaths, WorkstationLock,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Hold {
    schema_version: String,
    generation_id: String,
    payload_digest: String,
    reason: String,
    retained_at: String,
    released_at: Option<String>,
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id == "."
        || id == ".."
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("generation_retention_invalid_id".into());
    }
    Ok(())
}

/// Hash every regular file and its relative path; links and writable entries
/// cannot supply immutable rollback custody. This is retention, not readiness.
fn payload_digest(generation: &Path) -> Result<String, String> {
    fn walk(
        root: &Path,
        path: &Path,
        entries: &mut BTreeMap<String, String>,
    ) -> Result<(), String> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|e| format!("generation_retention_payload_unavailable: {e}"))?;
        if metadata.file_type().is_symlink() {
            return Err("generation_retention_symlink_rejected".into());
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                walk(root, &entry.map_err(|e| e.to_string())?.path(), entries)?;
            }
        } else if metadata.is_file() {
            entries.insert(
                path.strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .into_owned(),
                super::workstation_file_sha256(path)?,
            );
        } else {
            return Err("generation_retention_special_file_rejected".into());
        }
        Ok(())
    }
    super::validate_sealed_generation_tree(generation)?;
    let mut entries = BTreeMap::new();
    walk(generation, generation, &mut entries)?;
    for required in [
        "generation.json",
        "bin/agent-browser",
        "support/manifest.json",
    ] {
        if !entries.contains_key(required) {
            return Err(format!("generation_retention_payload_missing:{required}"));
        }
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&entries).map_err(|e| e.to_string())?)
    ))
}

fn hold_path(root: &Path, id: &str) -> std::path::PathBuf {
    root.join(".agent-browser/runtime-adoption/generation-retention")
        .join(format!("{id}.json"))
}

fn read_hold(path: &Path) -> Result<Option<Hold>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("generation_retention_read_failed: {e}")),
    };
    let hold: Hold =
        serde_json::from_slice(&bytes).map_err(|e| format!("generation_retention_invalid: {e}"))?;
    validate_id(&hold.generation_id)?;
    if hold.schema_version != "agent-browser.generation-retention.v1"
        || hold.reason.trim().is_empty()
    {
        return Err("generation_retention_invalid".into());
    }
    Ok(Some(hold))
}

fn change(
    root: &Path,
    paths: &InstallPaths,
    id: &str,
    reason: &str,
    release: bool,
    apply: bool,
) -> Result<serde_json::Value, String> {
    validate_id(id)?;
    if reason.trim().is_empty() || reason.len() > 1024 {
        return Err("generation_retention_reason_required".into());
    }
    let generation = paths.generations_dir.join(id);
    let digest = payload_digest(&generation)?;
    let path = hold_path(root, id);
    let previous = read_hold(&path)?;
    if let Some(previous) = &previous {
        if previous.generation_id != id || previous.payload_digest != digest {
            return Err("generation_retention_payload_changed".into());
        }
    } else if release {
        return Err("generation_retention_hold_missing".into());
    }
    let now = super::runtime_adoption_timestamp();
    let hold = Hold {
        schema_version: "agent-browser.generation-retention.v1".into(),
        generation_id: id.into(),
        payload_digest: digest,
        reason: reason.into(),
        retained_at: previous
            .as_ref()
            .map(|h| h.retained_at.clone())
            .unwrap_or_else(|| now.clone()),
        released_at: release.then_some(now),
    };
    if apply {
        write_private_json_atomic(&path, &hold)?;
    }
    Ok(
        serde_json::json!({"success":true,"mode":if apply {"apply"} else {"dry-run"},"retained":!release,"hold":hold,"selectedGenerationId":selected_generation_id(paths),"healthAttested":false,"generationRemoved":false}),
    )
}

pub(super) fn run(args: &[String], json: bool) {
    let result = (|| {
        let start = args
            .iter()
            .position(|a| a == "retain-generation")
            .ok_or("generation_retention_command_missing")?;
        let id = args
            .get(start + 1)
            .ok_or("generation_retention_id_required")?;
        let mut reason = None;
        let mut apply = false;
        let mut dry = false;
        let mut release = false;
        let mut index = start + 2;
        while index < args.len() {
            match args[index].as_str() {
                "--reason" => {
                    index += 1;
                    reason = Some(
                        args.get(index)
                            .ok_or("generation_retention_reason_required")?
                            .as_str(),
                    );
                }
                "--apply" => apply = true,
                "--dry-run" => dry = true,
                "--release" => release = true,
                "--json" => {}
                _ => return Err("generation_retention_unknown_argument".into()),
            }
            index += 1;
        }
        if apply == dry {
            return Err("generation_retention_choose_dry_run_or_apply".into());
        }
        let root = workstation_root()?;
        let paths = install_paths(&root);
        let _lock = if apply {
            Some(WorkstationLock::acquire(&root)?)
        } else {
            None
        };
        change(
            &root,
            &paths,
            id,
            reason.ok_or("generation_retention_reason_required")?,
            release,
            apply,
        )
    })();
    match result {
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Err(error) => super::fail(&error, json),
    }
}

/// Join holds before any retention finalization or generation removal. Invalid
/// retained evidence blocks GC; a release is explicit and leaves its receipt.
pub(super) fn references(
    root: &Path,
    paths: &InstallPaths,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let dir = root.join(".agent-browser/runtime-adoption/generation-retention");
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(e.to_string()),
    };
    let mut references = BTreeMap::new();
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let hold = read_hold(&path)?.ok_or("generation_retention_hold_disappeared")?;
        if path != hold_path(root, &hold.generation_id) {
            return Err("generation_retention_filename_mismatch".into());
        }
        if hold.released_at.is_some() {
            continue;
        }
        if payload_digest(&paths.generations_dir.join(&hold.generation_id))? != hold.payload_digest
        {
            return Err("generation_retention_payload_changed".into());
        }
        references.insert(hold.generation_id, vec!["operator_retention_hold".into()]);
    }
    Ok(references)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (std::path::PathBuf, InstallPaths) {
        let root = std::env::temp_dir().join(format!("generation-hold-{}", uuid::Uuid::new_v4()));
        let paths = install_paths(&root);
        let generation = paths.generations_dir.join("reviewed-rollback");
        fs::create_dir_all(generation.join("bin")).unwrap();
        fs::create_dir_all(generation.join("support")).unwrap();
        for (path, bytes) in [
            ("generation.json", "{}"),
            ("bin/agent-browser", "binary"),
            ("support/manifest.json", "{}"),
        ] {
            fs::write(generation.join(path), bytes).unwrap();
        }
        super::super::seal_generation_tree(&generation).unwrap();
        (root, paths)
    }
    #[test]
    fn explicit_hold_protects_gc_without_inventing_a_healthy_generation() {
        let (root, paths) = fixture();
        let preview = change(
            &root,
            &paths,
            "reviewed-rollback",
            "reviewed rollback",
            false,
            false,
        )
        .unwrap();
        assert_eq!(preview["healthAttested"], false);
        assert!(!hold_path(&root, "reviewed-rollback").exists());
        change(
            &root,
            &paths,
            "reviewed-rollback",
            "reviewed rollback",
            false,
            true,
        )
        .unwrap();
        let gc = super::super::workstation_generation_gc_locked(
            &root,
            &paths,
            super::super::InstallMode::DryRun,
        )
        .unwrap();
        assert_eq!(gc["candidates"], serde_json::json!([]));
        assert_eq!(
            gc["retained"][0]["reasonCodes"],
            serde_json::json!(["operator_retention_hold"])
        );
        assert!(gc["previousHealthyGenerationId"].is_null());
        assert!(!root
            .join(".agent-browser/runtime-adoption/transactions")
            .exists());
        change(
            &root,
            &paths,
            "reviewed-rollback",
            "reviewed release",
            true,
            true,
        )
        .unwrap();
        let gc = super::super::workstation_generation_gc_locked(
            &root,
            &paths,
            super::super::InstallMode::DryRun,
        )
        .unwrap();
        assert_eq!(gc["candidates"], serde_json::json!(["reviewed-rollback"]));
        assert!(read_hold(&hold_path(&root, "reviewed-rollback"))
            .unwrap()
            .unwrap()
            .released_at
            .is_some());
        super::super::remove_generation_tree(&root).unwrap();
    }
    #[test]
    fn changed_payload_or_invalid_hold_blocks_gc_before_effects() {
        let (root, paths) = fixture();
        change(
            &root,
            &paths,
            "reviewed-rollback",
            "reviewed rollback",
            false,
            true,
        )
        .unwrap();
        let path = hold_path(&root, "reviewed-rollback");
        let mut hold = read_hold(&path).unwrap().unwrap();
        hold.payload_digest = "changed".into();
        write_private_json_atomic(&path, &hold).unwrap();
        let error = super::super::workstation_generation_gc_locked(
            &root,
            &paths,
            super::super::InstallMode::Apply,
        )
        .unwrap_err();
        assert!(error.contains("generation_retention_payload_changed"));
        assert!(paths
            .generations_dir
            .join("reviewed-rollback/bin/agent-browser")
            .exists());
        assert!(change(&root, &paths, "../escape", "reason", false, true).is_err());
        fs::write(&path, "invalid JSON").unwrap();
        assert!(references(&root, &paths)
            .unwrap_err()
            .contains("generation_retention_invalid"));
        super::super::remove_generation_tree(&root).unwrap();
    }
}
