//! One-route provisioning from durable SQLite intent. Provider failures retain
//! the exact operation for bounded replay and do not retire working routes.

use super::browser_session_store::{
    BrowserRuntimeSqliteStore, PresentationProvisioningConfig, PresentationProvisioningOperation,
};
use super::presentation_provisioning_sql::{render_route_provision_sql, RouteProvisionSqlInput};
use serde_json::Value;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

#[async_trait::async_trait]
pub(crate) trait PresentationProvisioningEffects: Send {
    async fn provision(
        &mut self,
        config: &PresentationProvisioningConfig,
        operation: &PresentationProvisioningOperation,
    ) -> Result<u64, String>;
}

pub(crate) async fn reconcile_presentation_growth(
    database_path: &Path,
    effects: &mut impl PresentationProvisioningEffects,
) -> Result<(), String> {
    let mut store = BrowserRuntimeSqliteStore::open(database_path)?;
    let Some((config, operation)) = store.reserve_presentation_growth()? else {
        return Ok(());
    };
    // No transaction is held across provider access. Completion checks the exact
    // attempt and catalog again before publishing its additive binding.
    let result = effects.provision(&config, &operation).await;
    match result {
        Ok(connection_id) => {
            if store
                .complete_presentation_growth(&operation, connection_id)
                .is_err()
            {
                store.fail_presentation_growth(
                    &operation,
                    "presentation_provisioning_publication_failed",
                )?;
            }
        }
        Err(code) => {
            let code = match code.as_str() {
                "presentation_provisioning_helper_unsupported"
                | "presentation_provisioning_helper_invalid"
                | "presentation_provisioning_container_invalid"
                | "presentation_provisioning_container_ownership_mismatch"
                | "presentation_provisioning_command_timeout"
                | "presentation_provisioning_receipt_invalid" => code.as_str(),
                _ => "presentation_provisioning_provider_failed",
            };
            store.fail_presentation_growth(&operation, code)?;
        }
    }
    Ok(())
}

pub(crate) struct InstalledProvisioningEffects;

/// Secrets travel only through stdin. Command errors and output are never
/// propagated into public diagnostics; subprocess lifetime and output are bounded.
async fn command_output(program: &str, args: &[&str], input: &[u8]) -> Result<Vec<u8>, String> {
    let operation = async {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| "presentation_provisioning_command_failed".to_string())?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or("presentation_provisioning_stdin_failed")?;
        stdin
            .write_all(input)
            .await
            .map_err(|_| "presentation_provisioning_stdin_failed")?;
        drop(stdin);
        let stdout = child
            .stdout
            .take()
            .ok_or("presentation_provisioning_stdout_failed")?;
        let mut output = Vec::new();
        stdout
            .take(65537)
            .read_to_end(&mut output)
            .await
            .map_err(|_| "presentation_provisioning_command_failed")?;
        if output.len() > 65536 {
            return Err("presentation_provisioning_command_failed".to_string());
        }
        let status = child
            .wait()
            .await
            .map_err(|_| "presentation_provisioning_command_failed")?;
        if !status.success() {
            return Err("presentation_provisioning_command_failed".to_string());
        }
        Ok(output)
    };
    tokio::time::timeout(Duration::from_secs(30), operation)
        .await
        .map_err(|_| "presentation_provisioning_command_timeout".to_string())?
}

#[async_trait::async_trait]
impl PresentationProvisioningEffects for InstalledProvisioningEffects {
    async fn provision(
        &mut self,
        config: &PresentationProvisioningConfig,
        operation: &PresentationProvisioningOperation,
    ) -> Result<u64, String> {
        config.validate()?;
        let helper = crate::remote_view_helper_contract::FIXED_HELPER_PATH;
        let status: Value =
            serde_json::from_slice(&command_output(helper, &["status-json"], b"").await?)
                .map_err(|_| "presentation_provisioning_helper_invalid")?;
        for field in ["supported", "gecosOperationMarker", "retrySafe"] {
            if status["routeUserOwnedProvisioning"][field] != true {
                return Err("presentation_provisioning_helper_unsupported".to_string());
            }
        }
        let inspected: Value = serde_json::from_slice(
            &command_output("docker", &["inspect", &config.postgres_container], b"").await?,
        )
        .map_err(|_| "presentation_provisioning_container_invalid")?;
        let labels = &inspected[0]["Config"]["Labels"];
        if labels["com.docker.compose.project"].as_str() != Some(config.compose_project.as_str())
            || labels["agent-browser.environment"].as_str() != Some(config.environment.as_str())
            || inspected[0]["State"]["Running"] != true
        {
            return Err("presentation_provisioning_container_ownership_mismatch".to_string());
        }
        let route_user = format!("{}{}", config.route_user_prefix, operation.ordinal);
        command_output(
            "sudo",
            &[
                "-n",
                helper,
                "ensure-rdp-route-user-owned",
                "--user",
                &route_user,
                "--operation-id",
                &operation.operation_id,
            ],
            format!("{}\n", operation.password).as_bytes(),
        )
        .await?;
        let sql = render_route_provision_sql(&RouteProvisionSqlInput {
            connection_name: &format!("{}{}", config.connection_name_prefix, operation.ordinal),
            route_user: &route_user,
            password: &operation.password,
            sharing_profile_name: &format!(
                "{}{}",
                config.sharing_profile_prefix, operation.ordinal
            ),
            hostname: &config.rdp_host,
            port: config.rdp_port,
            max_connections: config.max_connections,
            max_connections_per_user: config.max_connections_per_user,
        })?;
        let output = command_output(
            "docker",
            &[
                "exec",
                "-i",
                &config.postgres_container,
                "psql",
                "-U",
                &config.postgres_user,
                "-d",
                &config.postgres_database,
                "-v",
                "ON_ERROR_STOP=1",
                "-t",
                "-A",
                "-q",
            ],
            sql.as_bytes(),
        )
        .await?;
        let receipt: Value = serde_json::from_slice(&output)
            .map_err(|_| "presentation_provisioning_receipt_invalid")?;
        receipt["connectionId"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or_else(|| "presentation_provisioning_receipt_invalid".to_string())
    }
}
