use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{digest_serializable, validate_nonempty, validate_sha256, CandidateError};

pub const DEVELOPMENT_NAMESPACE_SCHEMA_VERSION: &str = "agent-browser.development-namespace.v1";
pub const TEST_RUN_IDENTITY_SCHEMA_VERSION: &str = "agent-browser.test-run-identity.v1";

const NAMESPACE_SLOT_COUNT: u16 = 512;
const PORT_BLOCK_BASE: u16 = 32_000;
const PORT_BLOCK_WIDTH: u16 = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DevelopmentNamespace {
    pub schema_version: String,
    pub lane_id: String,
    pub namespace_id: String,
    pub slot: u16,
    pub install_root: String,
    pub pseudo_home: String,
    pub runtime_directory: String,
    pub socket_directory: String,
    pub profile_root: String,
    pub browser_state_root: String,
    pub output_root: String,
    pub provider_namespace: String,
    pub ports: BTreeMap<String, u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamespaceAllocation {
    Existing(DevelopmentNamespace),
    Allocated(DevelopmentNamespace),
}

pub fn allocate_development_namespace(
    lane_id: &str,
    existing: &[DevelopmentNamespace],
) -> Result<NamespaceAllocation, CandidateError> {
    validate_lane_id(lane_id)?;
    let matches = existing
        .iter()
        .filter(|namespace| namespace.lane_id == lane_id)
        .collect::<Vec<_>>();
    if let Some(first) = matches.first() {
        if matches.iter().any(|namespace| *namespace != *first) {
            return Err(CandidateError::new(
                "namespace_state_conflict",
                format!("lane {lane_id} has conflicting namespace records"),
            ));
        }
        return Ok(NamespaceAllocation::Existing((*first).clone()));
    }

    let occupied = existing
        .iter()
        .map(|namespace| namespace.slot)
        .collect::<BTreeSet<_>>();
    let lane_digest = digest_serializable(&lane_id);
    let initial = u16::from_str_radix(&lane_digest[..4], 16).expect("digest prefix is hexadecimal")
        % NAMESPACE_SLOT_COUNT;
    let slot = (0..NAMESPACE_SLOT_COUNT)
        .map(|offset| (initial + offset) % NAMESPACE_SLOT_COUNT)
        .find(|candidate| !occupied.contains(candidate))
        .ok_or_else(|| {
            CandidateError::new(
                "namespace_capacity_exhausted",
                "no development namespace slot is available",
            )
        })?;
    let normalized_lane = lane_id.to_ascii_lowercase();
    let namespace_id = format!("{}-{}", normalized_lane, &lane_digest[..12]);
    let root = format!("development/{namespace_id}");
    let port_base = PORT_BLOCK_BASE + slot * PORT_BLOCK_WIDTH;
    let ports = BTreeMap::from([
        ("daemon".to_string(), port_base),
        ("dashboard".to_string(), port_base + 1),
        ("provider".to_string(), port_base + 2),
        ("remote_view".to_string(), port_base + 3),
    ]);
    Ok(NamespaceAllocation::Allocated(DevelopmentNamespace {
        schema_version: DEVELOPMENT_NAMESPACE_SCHEMA_VERSION.to_string(),
        lane_id: lane_id.to_string(),
        namespace_id: namespace_id.clone(),
        slot,
        install_root: format!("{root}/install"),
        pseudo_home: format!("{root}/home"),
        runtime_directory: format!("{root}/runtime"),
        socket_directory: format!("{root}/sockets"),
        profile_root: format!("{root}/profiles"),
        browser_state_root: format!("{root}/browser-state"),
        output_root: format!("{root}/outputs"),
        provider_namespace: format!("agent-browser-{namespace_id}"),
        ports,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestResourceClass {
    IsolatedProviderFree,
    Shared(String),
    ProviderBacked(String),
}

impl TestResourceClass {
    fn validate(&self) -> Result<(), CandidateError> {
        match self {
            Self::IsolatedProviderFree => Ok(()),
            Self::Shared(key) | Self::ProviderBacked(key) => {
                validate_nonempty("test_resource_key", key)
            }
        }
    }

    fn shared_key(&self) -> Option<&str> {
        match self {
            Self::IsolatedProviderFree => None,
            Self::Shared(key) | Self::ProviderBacked(key) => Some(key),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestRunIdentity {
    pub schema_version: String,
    pub candidate_manifest_sha256: String,
    pub binary_sha256: String,
    pub test_suite_revision: String,
    pub selection: Vec<String>,
    pub fixture_sha256: String,
    pub target: String,
    pub runtime_capability_sha256: String,
    pub environment_input_sha256: String,
    pub resource_class: TestResourceClass,
}

impl TestRunIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        candidate_manifest_sha256: String,
        binary_sha256: String,
        test_suite_revision: impl Into<String>,
        mut selection: Vec<String>,
        fixture_sha256: String,
        target: impl Into<String>,
        runtime_capability_sha256: String,
        environment_input_sha256: String,
        resource_class: TestResourceClass,
    ) -> Result<Self, CandidateError> {
        validate_sha256("candidate_manifest_sha256", &candidate_manifest_sha256)?;
        validate_sha256("binary_sha256", &binary_sha256)?;
        validate_sha256("fixture_sha256", &fixture_sha256)?;
        validate_sha256("runtime_capability_sha256", &runtime_capability_sha256)?;
        validate_sha256("environment_input_sha256", &environment_input_sha256)?;
        let test_suite_revision = test_suite_revision.into();
        let target = target.into();
        validate_nonempty("test_suite_revision", &test_suite_revision)?;
        validate_nonempty("target", &target)?;
        resource_class.validate()?;
        selection.sort();
        selection.dedup();
        if selection.is_empty() || selection.iter().any(|item| item.trim().is_empty()) {
            return Err(CandidateError::new(
                "invalid_test_selection",
                "test selection must contain at least one nonempty item",
            ));
        }
        Ok(Self {
            schema_version: TEST_RUN_IDENTITY_SCHEMA_VERSION.to_string(),
            candidate_manifest_sha256,
            binary_sha256,
            test_suite_revision,
            selection,
            fixture_sha256,
            target,
            runtime_capability_sha256,
            environment_input_sha256,
            resource_class,
        })
    }

    pub fn digest(&self) -> String {
        digest_serializable(self)
    }

    pub fn validate(&self) -> Result<(), CandidateError> {
        if self.schema_version != TEST_RUN_IDENTITY_SCHEMA_VERSION {
            return Err(CandidateError::new(
                "unsupported_test_run_identity_schema",
                "test-run identity schema is not supported",
            ));
        }
        let canonical = Self::new(
            self.candidate_manifest_sha256.clone(),
            self.binary_sha256.clone(),
            self.test_suite_revision.clone(),
            self.selection.clone(),
            self.fixture_sha256.clone(),
            self.target.clone(),
            self.runtime_capability_sha256.clone(),
            self.environment_input_sha256.clone(),
            self.resource_class.clone(),
        )?;
        if canonical != *self {
            return Err(CandidateError::new(
                "noncanonical_test_run_identity",
                "test-run identity must use canonical selection and resource values",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRunState {
    Active,
    Passed,
    Failed,
    Cancelled,
    TimedOut,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestRunRecord {
    pub run_id: String,
    pub identity: TestRunIdentity,
    pub state: TestRunState,
    pub hermetic: bool,
    pub terminal_cleanup_proven: bool,
    pub receipt_locator: Option<String>,
}

impl TestRunRecord {
    pub fn active(run_id: impl Into<String>, identity: TestRunIdentity) -> Self {
        Self {
            run_id: run_id.into(),
            identity,
            state: TestRunState::Active,
            hermetic: false,
            terminal_cleanup_proven: false,
            receipt_locator: None,
        }
    }

    pub fn validate(&self) -> Result<(), CandidateError> {
        validate_nonempty("test_run_id", &self.run_id)?;
        self.identity.validate()?;
        if self.state == TestRunState::Active
            && (self.hermetic || self.terminal_cleanup_proven || self.receipt_locator.is_some())
        {
            return Err(CandidateError::new(
                "active_test_run_has_terminal_evidence",
                "an active test run cannot claim terminal receipt or cleanup evidence",
            ));
        }
        if self
            .receipt_locator
            .as_ref()
            .is_some_and(|locator| locator.trim().is_empty())
        {
            return Err(CandidateError::new(
                "invalid_test_receipt_locator",
                "test receipt locator must be nonempty when present",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRunDecision {
    JoinActive {
        run_id: String,
    },
    ReuseReceipt {
        run_id: String,
        receipt_locator: String,
    },
    WaitForSharedResource {
        resource_key: String,
        active_run_id: String,
    },
    StartIsolated {
        output_directory: String,
    },
}

pub fn coordinate_test_run(
    requested: &TestRunIdentity,
    active: &[TestRunRecord],
    completed: &[TestRunRecord],
) -> Result<TestRunDecision, CandidateError> {
    requested.validate()?;
    for run in active.iter().chain(completed) {
        run.validate()?;
    }
    if let Some(run) = active
        .iter()
        .find(|run| run.state == TestRunState::Active && run.identity == *requested)
    {
        return Ok(TestRunDecision::JoinActive {
            run_id: run.run_id.clone(),
        });
    }
    if let Some(resource_key) = requested.resource_class.shared_key() {
        if let Some(run) = active.iter().find(|run| {
            run.state == TestRunState::Active
                && run.identity.resource_class.shared_key() == Some(resource_key)
        }) {
            return Ok(TestRunDecision::WaitForSharedResource {
                resource_key: resource_key.to_string(),
                active_run_id: run.run_id.clone(),
            });
        }
    }
    if let Some(run) = completed.iter().find(|run| {
        run.identity == *requested
            && run.state == TestRunState::Passed
            && run.hermetic
            && run.terminal_cleanup_proven
            && run
                .receipt_locator
                .as_ref()
                .is_some_and(|locator| !locator.trim().is_empty())
    }) {
        return Ok(TestRunDecision::ReuseReceipt {
            run_id: run.run_id.clone(),
            receipt_locator: run.receipt_locator.clone().expect("checked above"),
        });
    }
    Ok(TestRunDecision::StartIsolated {
        output_directory: format!("candidate-tests/{}", &requested.digest()[..32]),
    })
}

fn validate_lane_id(lane_id: &str) -> Result<(), CandidateError> {
    if lane_id.trim().is_empty()
        || !lane_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(CandidateError::new(
            "invalid_lane_id",
            "lane ID must contain only ASCII letters, numbers, hyphens, or underscores",
        ));
    }
    Ok(())
}
