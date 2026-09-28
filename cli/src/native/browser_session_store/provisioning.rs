//! Durable provider coordinates and bounded one-route provisioning intent.

use super::*;

const DOCUMENT: &str = "presentation_provisioning";
const SCHEMA: &str = "agent-browser.presentation-provisioning.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct PresentationProvisioningConfig {
    pub environment: String,
    pub compose_project: String,
    pub postgres_container: String,
    pub postgres_user: String,
    pub postgres_database: String,
    pub rdp_host: String,
    pub rdp_port: u16,
    pub route_user_prefix: String,
    pub connection_key_prefix: String,
    pub connection_name_prefix: String,
    pub sharing_profile_prefix: String,
    pub maximum_slots: u32,
    pub max_connections: u32,
    pub max_connections_per_user: u32,
}

impl PresentationProvisioningConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        let identifier = |s: &str| {
            !s.is_empty()
                && s.len() <= 128
                && !s.starts_with('-')
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        };
        if !matches!(self.environment.as_str(), "development" | "production")
            || [
                &self.compose_project,
                &self.postgres_container,
                &self.postgres_user,
                &self.postgres_database,
                &self.connection_key_prefix,
            ]
            .iter()
            .any(|s| !identifier(s))
            || !self.route_user_prefix.starts_with("agent-browser-rdp-")
            || !identifier(&self.route_user_prefix)
            || !self
                .route_user_prefix
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
            || self.route_user_prefix.len() + self.maximum_slots.to_string().len() > 48
            || [
                &self.rdp_host,
                &self.connection_name_prefix,
                &self.sharing_profile_prefix,
            ]
            .iter()
            .any(|s| s.is_empty() || s.len() > 256 || s.chars().any(char::is_control))
            || self.rdp_port == 0
            || self.maximum_slots == 0
            || self.maximum_slots > 64
            || self.max_connections == 0
            || self.max_connections > 64
            || self.max_connections_per_user == 0
            || self.max_connections_per_user > 64
        {
            return Err("presentation_provisioning_config_invalid".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationProvisioningOperation {
    pub operation_id: String,
    pub expected_catalog_digest: String,
    pub ordinal: u32,
    pub password: String,
    pub attempts: u32,
    pub failed: bool,
    pub failure_code: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProvisioningState {
    config: Option<PresentationProvisioningConfig>,
    pending: Option<PresentationProvisioningOperation>,
    completed_operation: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationGrowthStatus {
    pub state: &'static str,
    pub desired_maximum: u32,
    pub provisioned_maximum: u32,
    pub operation_id: Option<String>,
    pub attempts: u32,
    pub failure_code: Option<String>,
}

fn load_state(connection: &Connection) -> Result<ProvisioningState, String> {
    load_optional_document(connection, DOCUMENT, SCHEMA)
}

impl BrowserRuntimeSqliteStore {
    /// Import reviewed installer coordinates with the complete catalog in one transaction.
    pub(crate) fn publish_route_keeper_configuration(
        &mut self,
        catalog: RouteKeeperConnectionCatalog,
        provisioning: Option<PresentationProvisioningConfig>,
    ) -> Result<RouteKeeperConnectionCatalogPublication, String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "presentation_provisioning_begin_failed".to_string())?;
        let current = load_route_keeper_authority_document(&transaction)?;
        let mut state = load_state(&transaction)?;
        // The installer replays its original seed after autonomous growth. Only
        // the same imported provisioning configuration can retain a strict,
        // exactly matching seed subset; this never removes provisioned slots.
        let retained_seed = provisioning.as_ref().is_some_and(|config| {
            state.config.as_ref() == Some(config)
                && !catalog.bindings.is_empty()
                && catalog.bindings.len() < current.connection_catalog.bindings.len()
                && catalog.provider_base == current.connection_catalog.provider_base
                && catalog.public_operator_url == current.connection_catalog.public_operator_url
                && catalog.bindings.iter().all(|(slot, binding)| {
                    current.connection_catalog.bindings.get(slot) == Some(binding)
                })
                && (1..=catalog.bindings.len()).all(|ordinal| {
                    catalog
                        .bindings
                        .contains_key(&format!("route-slot-{ordinal:02}"))
                })
        });
        let next = if retained_seed {
            current.clone()
        } else {
            catalog_publication_authority(&current, catalog)?
        };
        let old_config = state.config.clone();
        if let Some(config) = provisioning {
            config.validate()?;
            if config.maximum_slots < next.policy.maximum_slots
                || state.config.as_ref().is_some_and(|old| old != &config)
            {
                return Err("presentation_provisioning_config_conflict".to_string());
            }
            state.config = Some(config);
        }
        if let Some(config) = &state.config {
            if config.maximum_slots < next.policy.maximum_slots {
                return Err("presentation_provisioning_config_conflict".to_string());
            }
            for (index, binding) in next.connection_catalog.bindings.values().enumerate() {
                let ordinal = index + 1;
                if binding.route_user != format!("{}{}", config.route_user_prefix, ordinal)
                    || binding.connection_key
                        != format!("{}{}", config.connection_key_prefix, ordinal)
                    || binding.connection_name
                        != format!("{}{}", config.connection_name_prefix, ordinal)
                {
                    return Err("presentation_provisioning_catalog_namespace_mismatch".to_string());
                }
            }
        }
        if next == current && state.config == old_config {
            return Ok(RouteKeeperConnectionCatalogPublication::Unchanged);
        }
        save_document(
            &transaction,
            ROUTE_KEEPER_AUTHORITY_DOCUMENT,
            ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
            &next,
        )?;
        save_document(&transaction, DOCUMENT, SCHEMA, &state)?;
        transaction
            .commit()
            .map_err(|_| "presentation_provisioning_commit_failed".to_string())?;
        Ok(RouteKeeperConnectionCatalogPublication::Published)
    }

    pub(crate) fn presentation_growth_status(&self) -> Result<PresentationGrowthStatus, String> {
        let state = load_state(&self.connection)?;
        let config = self.load_runtime_config()?;
        let authority = self.load_route_keeper_authority()?;
        Ok(PresentationGrowthStatus {
            state: if state.pending.as_ref().is_some_and(|op| op.failed) {
                "failed"
            } else if state.pending.is_some() {
                "provisioning"
            } else if config.maximum_displays > authority.policy.maximum_slots {
                "pending"
            } else {
                "idle"
            },
            desired_maximum: config.maximum_displays,
            provisioned_maximum: authority.policy.maximum_slots,
            operation_id: state.pending.as_ref().map(|op| op.operation_id.clone()),
            attempts: state.pending.as_ref().map_or(0, |op| op.attempts),
            failure_code: state.pending.and_then(|op| op.failure_code),
        })
    }

    /// Reserve or resume one exact operation. Three attempts is a durable lifetime bound.
    pub(crate) fn reserve_presentation_growth(
        &mut self,
    ) -> Result<
        Option<(
            PresentationProvisioningConfig,
            PresentationProvisioningOperation,
        )>,
        String,
    > {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "presentation_provisioning_begin_failed".to_string())?;
        let mut state = load_state(&transaction)?;
        let Some(config) = state.config.clone() else {
            return Ok(None);
        };
        config.validate()?;
        let authority = load_route_keeper_authority_document(&transaction)?;
        let runtime = load_runtime_config_row(&transaction)?;
        if state.pending.is_none() {
            if runtime.maximum_displays <= authority.policy.maximum_slots {
                return Ok(None);
            }
            let ordinal = authority
                .policy
                .maximum_slots
                .checked_add(1)
                .ok_or_else(|| "presentation_provisioning_limit_exceeded".to_string())?;
            if ordinal > config.maximum_slots {
                return Err("presentation_provisioning_limit_exceeded".to_string());
            }
            state.pending = Some(PresentationProvisioningOperation {
                operation_id: uuid::Uuid::new_v4().to_string(),
                expected_catalog_digest: authority.connection_catalog.digest()?,
                ordinal,
                password: format!(
                    "{}{}",
                    uuid::Uuid::new_v4().simple(),
                    uuid::Uuid::new_v4().simple()
                ),
                attempts: 0,
                failed: false,
                failure_code: None,
            });
        }
        let operation = state.pending.as_mut().unwrap();
        if operation.failed {
            return Ok(None);
        }
        if operation.attempts >= 3 {
            operation.failed = true;
            operation.failure_code =
                Some("presentation_provisioning_attempts_exhausted".to_string());
            save_document(&transaction, DOCUMENT, SCHEMA, &state)?;
            transaction
                .commit()
                .map_err(|_| "presentation_provisioning_commit_failed".to_string())?;
            return Ok(None);
        }
        operation.attempts += 1;
        let operation = operation.clone();
        save_document(&transaction, DOCUMENT, SCHEMA, &state)?;
        transaction
            .commit()
            .map_err(|_| "presentation_provisioning_commit_failed".to_string())?;
        Ok(Some((config, operation)))
    }

    pub(crate) fn fail_presentation_growth(
        &mut self,
        operation: &PresentationProvisioningOperation,
        code: &str,
    ) -> Result<(), String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "presentation_provisioning_begin_failed".to_string())?;
        let mut state = load_state(&transaction)?;
        if state.pending.as_ref() != Some(operation) {
            return Err("presentation_provisioning_operation_stale".to_string());
        }
        let pending = state.pending.as_mut().unwrap();
        pending.failed = true;
        pending.failure_code = Some(
            if code
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
            {
                code.to_string()
            } else {
                "presentation_provisioning_failed".to_string()
            },
        );
        save_document(&transaction, DOCUMENT, SCHEMA, &state)?;
        transaction
            .commit()
            .map_err(|_| "presentation_provisioning_commit_failed".to_string())
    }

    pub(crate) fn complete_presentation_growth(
        &mut self,
        operation: &PresentationProvisioningOperation,
        connection_id: u64,
    ) -> Result<(), String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "presentation_provisioning_begin_failed".to_string())?;
        let mut state = load_state(&transaction)?;
        if state.pending.as_ref() != Some(operation) || connection_id == 0 {
            return Err("presentation_provisioning_operation_stale".to_string());
        }
        let config = state
            .config
            .as_ref()
            .ok_or("presentation_provisioning_unconfigured")?;
        let authority = load_route_keeper_authority_document(&transaction)?;
        if authority.connection_catalog.digest()? != operation.expected_catalog_digest
            || authority.policy.maximum_slots.checked_add(1) != Some(operation.ordinal)
        {
            return Err("presentation_provisioning_catalog_changed".to_string());
        }
        let mut catalog = authority.connection_catalog.clone();
        let slot_id = format!("route-slot-{:02}", operation.ordinal);
        catalog.bindings.insert(
            slot_id.clone(),
            agent_browser_service_model::RouteKeeperConnectionBinding {
                slot_id,
                connection_key: format!("{}{}", config.connection_key_prefix, operation.ordinal),
                connection_name: format!("{}{}", config.connection_name_prefix, operation.ordinal),
                route_user: format!("{}{}", config.route_user_prefix, operation.ordinal),
                guacamole_connection_id: connection_id,
            },
        );
        let mut next = authority;
        next.extend_connection_catalog(catalog, operation.ordinal)?;
        let runtime = load_runtime_config_row(&transaction)?;
        next.policy.minimum_ready = runtime.minimum_ready.min(next.policy.maximum_slots);
        next.policy.warm_target = runtime.warm_target.min(next.policy.maximum_slots);
        next.projection()?;
        state.completed_operation = Some(operation.operation_id.clone());
        state.pending = None;
        save_document(
            &transaction,
            ROUTE_KEEPER_AUTHORITY_DOCUMENT,
            ROUTE_KEEPER_AUTHORITY_SCHEMA_V4,
            &next,
        )?;
        save_document(&transaction, DOCUMENT, SCHEMA, &state)?;
        transaction
            .commit()
            .map_err(|_| "presentation_provisioning_commit_failed".to_string())
    }
}

pub(super) fn authorize_growth_config(
    connection: &Connection,
    maximum: u32,
    physical: u32,
    retry_requested: bool,
) -> Result<(), String> {
    let mut state = load_state(connection)?;
    if maximum > physical {
        let Some(config) = state.config.as_ref() else {
            return Err(format!(
                "browser_runtime_config_provisioned_capacity_exceeded:{maximum}:{physical}"
            ));
        };
        config.validate()?;
        if maximum > config.maximum_slots {
            return Err("presentation_provisioning_limit_exceeded".to_string());
        }
    }
    if retry_requested {
        if let Some(operation) = state
            .pending
            .as_mut()
            .filter(|op| op.failed && op.attempts < 3)
        {
            operation.failed = false;
            operation.failure_code = None;
            save_document(connection, DOCUMENT, SCHEMA, &state)?;
        }
    }
    Ok(())
}
