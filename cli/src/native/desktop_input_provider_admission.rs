//! Installed-generation admission for the controlled desktop input provider.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const DEVELOPMENT_RUNTIME_SCHEMA: &str = "agent-browser.development-runtime.v1";
const PRODUCTION_RUNTIME_SCHEMA: &str = "agent-browser.runtime-generation.v1";
const PROVIDER_ID: &str = "controlled-x11-xtest";
const PROVIDER_CAPABILITY: &str = "guarded_pointer_keyboard_v1";
const RECIPE_ID: &str = "p131-controlled-x11-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderAdmission {
    pub(crate) runtime_environment: String,
    pub(crate) generation_id: String,
    pub(crate) generation_sha256: String,
    pub(crate) provider_id: String,
    pub(crate) capability: String,
    pub(crate) recipe_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerationManifest {
    schema_version: String,
    environment: String,
    generation_id: String,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    binary_sha256: Option<String>,
    desktop_input_provider: ProviderManifest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderManifest {
    enabled: bool,
    provider_id: String,
    capability: String,
    recipe_id: String,
}

pub(crate) fn current_provider_admission() -> Result<ProviderAdmission, String> {
    let environment = std::env::var("AGENT_BROWSER_RUNTIME_ENVIRONMENT").unwrap_or_default();
    let executable = std::env::current_exe()
        .map_err(|_| "desktop_input_provider_generation_unavailable".to_string())?;
    verify_provider_admission(&environment, &executable)
}

pub(crate) fn revalidate_current_provider_admission(
    expected: &ProviderAdmission,
) -> Result<(), String> {
    let environment = std::env::var("AGENT_BROWSER_RUNTIME_ENVIRONMENT").unwrap_or_default();
    let executable = std::env::current_exe()
        .map_err(|_| "desktop_input_provider_generation_changed".to_string())?;
    revalidate_provider_admission(expected, &environment, &executable)
}

fn revalidate_provider_admission(
    expected: &ProviderAdmission,
    runtime_environment: &str,
    executable: &Path,
) -> Result<(), String> {
    let current = verify_provider_admission(runtime_environment, executable)
        .map_err(|_| "desktop_input_provider_generation_changed".to_string())?;
    if current != *expected {
        return Err("desktop_input_provider_generation_changed".to_string());
    }
    Ok(())
}

fn verify_provider_admission(
    runtime_environment: &str,
    executable: &Path,
) -> Result<ProviderAdmission, String> {
    let (expected_schema, expected_environment, digest_field) = match runtime_environment {
        "development" => (DEVELOPMENT_RUNTIME_SCHEMA, "development", "sha256"),
        "" | "production" => (PRODUCTION_RUNTIME_SCHEMA, "production", "binarySha256"),
        _ => {
            return Err("desktop_input_provider_unavailable".to_string());
        }
    };
    let generation_dir = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "desktop_input_provider_generation_unavailable".to_string())?;
    let generation_name = generation_dir
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "desktop_input_provider_generation_unavailable".to_string())?;
    let raw = fs::read(generation_dir.join("generation.json"))
        .map_err(|_| "desktop_input_provider_generation_unavailable".to_string())?;
    let manifest: GenerationManifest = serde_json::from_slice(&raw)
        .map_err(|_| "desktop_input_provider_generation_invalid".to_string())?;
    let provider = manifest.desktop_input_provider;
    if manifest.schema_version != expected_schema
        || manifest.environment != expected_environment
        || manifest.generation_id != generation_name
        || !provider.enabled
        || provider.provider_id != PROVIDER_ID
        || provider.capability != PROVIDER_CAPABILITY
        || provider.recipe_id != RECIPE_ID
    {
        return Err("desktop_input_provider_generation_invalid".to_string());
    }
    let executable_bytes = fs::read(executable)
        .map_err(|_| "desktop_input_provider_generation_unavailable".to_string())?;
    let executable_sha256 = format!("{:x}", Sha256::digest(&executable_bytes));
    let manifest_sha256 = match digest_field {
        "sha256" => manifest.sha256.as_deref(),
        "binarySha256" => manifest.binary_sha256.as_deref(),
        _ => None,
    };
    if manifest_sha256.is_none_or(|value| value.len() != 64 || value != executable_sha256) {
        return Err("desktop_input_provider_generation_mismatch".to_string());
    }
    Ok(ProviderAdmission {
        runtime_environment: expected_environment.to_string(),
        generation_id: manifest.generation_id,
        generation_sha256: executable_sha256,
        provider_id: provider.provider_id,
        capability: provider.capability,
        recipe_id: provider.recipe_id,
    })
}

#[cfg(test)]
mod tests {
    use super::{revalidate_provider_admission, verify_provider_admission};
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::PathBuf;

    fn fixture(case: &str, enabled: bool, sha256: Option<&str>) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-desktop-input-admission-{case}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let generation = root.join("generations").join("0.28.0-fixture");
        let binary = generation.join("bin").join("agent-browser");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"controlled-provider-fixture").unwrap();
        let digest = format!("{:x}", Sha256::digest(b"controlled-provider-fixture"));
        fs::write(
            generation.join("generation.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": "agent-browser.development-runtime.v1",
                "environment": "development",
                "generationId": "0.28.0-fixture",
                "sha256": sha256.unwrap_or(&digest),
                "desktopInputProvider": {
                    "enabled": enabled,
                    "providerId": "controlled-x11-xtest",
                    "capability": "guarded_pointer_keyboard_v1",
                    "recipeId": "p131-controlled-x11-v1"
                }
            }))
            .unwrap(),
        )
        .unwrap();
        binary
    }

    #[test]
    fn exact_development_generation_is_admitted() {
        let binary = fixture("accepted", true, None);
        let admission = verify_provider_admission("development", &binary).unwrap();
        assert_eq!(admission.provider_id, "controlled-x11-xtest");
        assert_eq!(admission.runtime_environment, "development");
        assert_eq!(admission.recipe_id, "p131-controlled-x11-v1");
        assert_eq!(admission.generation_id, "0.28.0-fixture");
    }

    #[test]
    fn exact_production_generation_is_admitted() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-desktop-input-admission-production-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let generation = root.join("generations").join("0.28.0-production");
        let binary = generation.join("bin").join("agent-browser");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"controlled-production-provider-fixture").unwrap();
        let digest = format!(
            "{:x}",
            Sha256::digest(b"controlled-production-provider-fixture")
        );
        fs::write(
            generation.join("generation.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": "agent-browser.runtime-generation.v1",
                "environment": "production",
                "generationId": "0.28.0-production",
                "binarySha256": digest,
                "desktopInputProvider": {
                    "enabled": true,
                    "providerId": "controlled-x11-xtest",
                    "capability": "guarded_pointer_keyboard_v1",
                    "recipeId": "p131-controlled-x11-v1"
                }
            }))
            .unwrap(),
        )
        .unwrap();

        for runtime_environment in ["", "production"] {
            let admission = verify_provider_admission(runtime_environment, &binary).unwrap();
            assert_eq!(admission.provider_id, "controlled-x11-xtest");
            assert_eq!(admission.runtime_environment, "production");
            assert_eq!(admission.recipe_id, "p131-controlled-x11-v1");
            assert_eq!(admission.generation_id, "0.28.0-production");
        }
    }

    #[test]
    fn unmanifested_environment_and_hash_drift_fail_closed() {
        let binary = fixture("unknown-environment", true, None);
        assert_eq!(
            verify_provider_admission("staging", &binary).unwrap_err(),
            "desktop_input_provider_unavailable"
        );
        let drifted = fixture("drifted", true, Some(&"0".repeat(64)));
        assert_eq!(
            verify_provider_admission("development", &drifted).unwrap_err(),
            "desktop_input_provider_generation_mismatch"
        );
    }

    #[test]
    fn effect_boundary_rejects_a_different_valid_generation_identity() {
        let binary = fixture("effect-boundary-drift", true, None);
        let admitted = verify_provider_admission("development", &binary).unwrap();

        fs::write(&binary, b"replacement-controlled-provider-fixture").unwrap();
        let replacement_digest = format!(
            "{:x}",
            Sha256::digest(b"replacement-controlled-provider-fixture")
        );
        let manifest_path = binary
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("generation.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["sha256"] = serde_json::Value::String(replacement_digest);
        fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();

        assert_eq!(
            revalidate_provider_admission(&admitted, "development", &binary).unwrap_err(),
            "desktop_input_provider_generation_changed"
        );
    }
}
