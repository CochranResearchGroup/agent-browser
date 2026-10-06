//! Read-only diagnostics bound to a published isolated installation.
use super::{binary_fingerprint, find_path_command, sha256_bytes};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{fs, path::Path, time::Duration};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Binding {
    schema_version: u32,
    command_path: std::path::PathBuf,
    command_sha256: String,
    dashboard_port: u16,
}

impl Binding {
    pub(super) fn command_fingerprint(&self, executable: &Value) -> Value {
        let mut result = executable.clone();
        result["path"] = json!(self.command_path);
        result["launcherSha256"] = json!(self.command_sha256);
        result["scope"] = json!("development_selected_generation");
        result
    }

    pub(super) fn dashboard_target(&self) -> (String, String, u16) {
        (
            format!(
                "http://127.0.0.1:{}/api/runtime/manifest",
                self.dashboard_port
            ),
            format!("127.0.0.1:{}", self.dashboard_port),
            self.dashboard_port,
        )
    }
}

/// A manifest is evidence only after its binary, selected pointer and exact
/// launcher bytes match. Missing bindings never fall back to production.
pub(super) fn load_binding(
    executable: &Path,
    namespace: Option<&str>,
) -> Result<Binding, &'static str> {
    load_binding_with_command(executable, namespace, find_path_command)
}

fn load_binding_with_command(
    executable: &Path,
    namespace: Option<&str>,
    lookup: impl Fn(&str) -> Option<std::path::PathBuf>,
) -> Result<Binding, &'static str> {
    let generation = executable
        .parent()
        .and_then(Path::parent)
        .ok_or("development_generation_unavailable")?;
    let root = generation
        .parent()
        .and_then(Path::parent)
        .ok_or("development_install_root_unavailable")?;
    let manifest: Value = serde_json::from_slice(
        &fs::read(generation.join("generation.json"))
            .map_err(|_| "development_manifest_unavailable")?,
    )
    .map_err(|_| "development_manifest_invalid")?;
    if manifest.get("schemaVersion").and_then(Value::as_str)
        != Some("agent-browser.development-runtime.v1")
        || manifest.get("environment").and_then(Value::as_str) != Some("development")
        || manifest.get("namespace").and_then(Value::as_str) != namespace
        || manifest.get("generationId").and_then(Value::as_str)
            != generation.file_name().and_then(|n| n.to_str())
        || manifest.get("sha256").and_then(Value::as_str).is_none()
        || manifest.get("sha256")
            != binary_fingerprint(Some(executable.to_path_buf())).get("sha256")
        || fs::canonicalize(root.join("current/bin/agent-browser"))
            .ok()
            .as_deref()
            != Some(executable)
    {
        return Err("development_generation_identity_mismatch");
    }
    let binding: Binding = serde_json::from_value(
        manifest
            .get("installDoctor")
            .cloned()
            .ok_or("development_doctor_binding_missing")?,
    )
    .map_err(|_| "development_doctor_binding_invalid")?;
    let launcher =
        fs::read(&binding.command_path).map_err(|_| "development_launcher_unavailable")?;
    let path_command = binding
        .command_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(lookup)
        .and_then(|path| fs::canonicalize(path).ok());
    if binding.schema_version != 1
        || binding.dashboard_port == 0
        || !binding.command_path.is_absolute()
        || sha256_bytes(&launcher) != binding.command_sha256
        || path_command != fs::canonicalize(&binding.command_path).ok()
    {
        return Err("development_launcher_identity_mismatch");
    }
    Ok(binding)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    struct Fixture {
        root: std::path::PathBuf,
        binary: std::path::PathBuf,
        command: std::path::PathBuf,
        manifest: Value,
    }

    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("development-doctor-{}", uuid::Uuid::new_v4()));
            let generation = root.join("generations/fixture");
            let binary = generation.join("bin/agent-browser");
            fs::create_dir_all(binary.parent().unwrap()).unwrap();
            fs::write(&binary, b"fixture binary; must never execute").unwrap();
            let command = root.join("agent-browser-dev-fixture");
            fs::write(&command, b"fixture launcher; must never execute").unwrap();
            std::os::unix::fs::symlink(&generation, root.join("current")).unwrap();
            let manifest = json!({
                "schemaVersion":"agent-browser.development-runtime.v1",
                "environment":"development", "namespace":"fixture", "generationId":"fixture",
                "sha256":sha256_bytes(&fs::read(&binary).unwrap()),
                "installDoctor": {"schemaVersion":1,"commandPath":command,
                    "commandSha256":sha256_bytes(&fs::read(&command).unwrap()),"dashboardPort":5048}
            });
            let fixture = Self {
                root,
                binary,
                command,
                manifest,
            };
            fixture.save();
            fixture
        }
        fn save(&self) {
            fs::write(
                self.binary
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("generation.json"),
                serde_json::to_vec(&self.manifest).unwrap(),
            )
            .unwrap();
        }
        fn load(&self) -> Result<Binding, &'static str> {
            load_binding_with_command(&self.binary, Some("fixture"), |_| {
                Some(self.command.clone())
            })
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn isolated_doctor_binds_selected_binary_launcher_and_port_without_execution() {
        let fixture = Fixture::new();
        let binding = fixture.load().unwrap();
        assert_eq!(binding.dashboard_target().2, 5048);
        let fingerprint =
            binding.command_fingerprint(&binary_fingerprint(Some(fixture.binary.clone())));
        assert_eq!(fingerprint["path"], json!(fixture.command));
        assert_eq!(fingerprint["sha256"], fixture.manifest["sha256"]);
    }

    #[test]
    fn isolated_doctor_rejects_cross_namespace_and_unselected_binary() {
        let fixture = Fixture::new();
        assert!(
            load_binding_with_command(&fixture.binary, Some("foreign"), |_| Some(
                fixture.command.clone()
            ))
            .is_err()
        );
        fs::remove_file(fixture.root.join("current")).unwrap();
        std::os::unix::fs::symlink(fixture.root.join("other"), fixture.root.join("current"))
            .unwrap();
        assert!(fixture.load().is_err());
    }

    #[test]
    fn isolated_doctor_rejects_launcher_drift_and_missing_path_command() {
        let fixture = Fixture::new();
        assert!(load_binding_with_command(&fixture.binary, Some("fixture"), |_| None).is_err());
        fs::write(&fixture.command, b"unreviewed launcher").unwrap();
        assert!(fixture.load().is_err());
    }

    #[test]
    fn isolated_doctor_rejects_missing_digest_binding_and_invalid_port() {
        let mut fixture = Fixture::new();
        fixture.manifest["sha256"] = Value::Null;
        fixture.save();
        assert!(fixture.load().is_err());
        fixture.manifest["sha256"] = json!(sha256_bytes(&fs::read(&fixture.binary).unwrap()));
        fixture.manifest["installDoctor"]["dashboardPort"] = json!(0);
        fixture.save();
        assert!(fixture.load().is_err());
        fixture
            .manifest
            .as_object_mut()
            .unwrap()
            .remove("installDoctor");
        fixture.save();
        assert!(fixture.load().is_err());
    }
}

/// Probe the selected provider's public, no-effect inventory. This proves
/// configuration admission, not visible pixels, controls or authentication.
pub(super) fn native_provider_readiness() -> Value {
    let result = (|| {
        use agent_browser_service_model::RemoteViewApplicationAdapter;
        let origin =
            std::env::var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN").map_err(|_| "origin_missing")?;
        let pool = std::env::var("AGENT_BROWSER_REMOTE_VIEW_POOL").map_err(|_| "pool_missing")?;
        let application = std::env::var("AGENT_BROWSER_REMOTE_VIEW_APPLICATION")
            .unwrap_or_else(|_| "agent-browser".into());
        let transport =
            crate::native::remote_view_application_http::RemoteViewApplicationHttp::new(
                &origin,
                Duration::from_secs(5),
            )
            .map_err(|_| "origin_invalid")?;
        let mut adapter = RemoteViewApplicationAdapter::new(application, transport)
            .map_err(|_| "application_invalid")?;
        let inventory = adapter
            .inventory()
            .map_err(|_| "inventory_unavailable_or_invalid")?;
        if !inventory.policy.pools.contains_key(&pool) {
            return Err("pool_not_admitted");
        }
        Ok(pool)
    })();
    match result {
        Ok(pool) => json!({"mode":"native_remote_view","ready":true,"helperRequired":false,
            "proofScope":"provider_inventory","pool":pool}),
        Err(reason) => json!({"mode":"native_remote_view","ready":false,"helperRequired":false,
            "proofScope":"provider_inventory","reason":reason}),
    }
}
