//! Immutable producer identity for failure records surviving runtime replacement.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, sync::OnceLock};

pub(super) fn current() -> Value {
    static IDENTITY: OnceLock<Value> = OnceLock::new();
    IDENTITY
        .get_or_init(|| {
            collect(
                std::env::current_exe().ok().as_deref(),
                option_env!("AGENT_BROWSER_BUILD_SOURCE_REVISION"),
            )
        })
        .clone()
}

fn collect(executable: Option<&Path>, revision: Option<&str>) -> Value {
    let mut unavailable = Vec::new();
    let binary = executable
        .and_then(|path| fs::read(path).ok())
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
    if binary.is_none() {
        unavailable.push("binary_identity_unavailable");
    }
    if revision.is_none() {
        unavailable.push("source_revision_unavailable");
    }
    let support = executable
        .and_then(Path::parent)
        .and_then(Path::parent)
        .and_then(|root| {
            let metadata: Value =
                serde_json::from_slice(&fs::read(root.join("generation.json")).ok()?).ok()?;
            let digest = format!(
                "{:x}",
                Sha256::digest(fs::read(root.join("support/manifest.json")).ok()?)
            );
            if binary.as_deref().is_none()
                || metadata.get("binarySha256")?.as_str() != binary.as_deref()
                || metadata.get("supportManifestSha256")?.as_str() != Some(digest.as_str())
            {
                return None;
            }
            let generation = metadata.get("generationId")?.as_str()?;
            if generation.len() > 128
                || !generation
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
            {
                return None;
            }
            Some((generation.to_string(), digest))
        });
    if support.is_none() {
        unavailable.push("matching_support_generation_unavailable");
    }
    json!({"packageVersion": env!("CARGO_PKG_VERSION"), "sourceRevision": revision,
        "sourceTreeState": option_env!("AGENT_BROWSER_BUILD_SOURCE_TREE_STATE").unwrap_or("unknown"),
        "binarySha256": binary, "supportGenerationId": support.as_ref().map(|s| &s.0),
        "supportManifestSha256": support.as_ref().map(|s| &s.1), "unavailableReasons": unavailable})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_records_do_not_acquire_the_reader_build_identity() {
        let record = super::super::ServiceFailureRecord::new(
            super::super::ServiceFailureCategory::ServiceAction,
            "test",
            "execution",
            "test_failure",
            "test",
        );
        let mut historical = serde_json::to_value(record).unwrap();
        historical.as_object_mut().unwrap().remove("buildIdentity");
        let decoded: super::super::ServiceFailureRecord =
            serde_json::from_value(historical).unwrap();
        assert!(decoded.build_identity.is_none());
        assert!(serde_json::to_value(decoded)
            .unwrap()
            .get("buildIdentity")
            .is_none());
    }
    #[test]
    fn support_identity_must_belong_to_the_producing_binary() {
        let root = std::env::temp_dir().join(format!("failure-build-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join("support")).unwrap();
        let executable = root.join("bin/agent-browser");
        fs::write(&executable, b"producer").unwrap();
        fs::write(root.join("support/manifest.json"), b"support").unwrap();
        let binary = format!("{:x}", Sha256::digest(b"producer"));
        let support = format!("{:x}", Sha256::digest(b"support"));
        let metadata = json!({"generationId":"test-generation", "binarySha256":binary, "supportManifestSha256":support});
        fs::write(root.join("generation.json"), metadata.to_string()).unwrap();
        let observed = collect(Some(&executable), Some("test-revision"));
        assert_eq!(observed["supportGenerationId"], "test-generation");
        fs::write(&executable, b"replacement").unwrap();
        let changed = collect(Some(&executable), Some("test-revision"));
        assert!(changed["supportGenerationId"].is_null());
        assert_ne!(changed["binarySha256"], observed["binarySha256"]);
        assert_eq!(
            changed["unavailableReasons"],
            json!(["matching_support_generation_unavailable"])
        );
        fs::remove_dir_all(root).unwrap();
    }
}
