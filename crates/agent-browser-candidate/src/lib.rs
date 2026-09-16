//! Pure candidate identity and advisory primitives.
//!
//! Repository inspection, Cargo execution, artifact storage, and runtime
//! mutation stay in adapters. This crate accepts already reviewed input
//! digests and produces deterministic identities without reading host state.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

mod advice;
mod build;
mod coordination;
mod development;
mod jam;
mod promotion;
mod transition;

pub use advice::*;
pub use build::*;
pub use coordination::*;
pub use development::*;
pub use jam::*;
pub use promotion::*;
pub use transition::*;

pub const EXECUTABLE_INPUT_CLOSURE_SCHEMA_VERSION: &str =
    "agent-browser.executable-input-closure.v1";
pub const CANDIDATE_MANIFEST_SCHEMA_VERSION: &str = "agent-browser.candidate-manifest.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateError {
    code: &'static str,
    detail: String,
}

impl CandidateError {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl Display for CandidateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl Error for CandidateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputCategory {
    RustSource,
    CargoManifest,
    CargoLock,
    BuildScript,
    EmbeddedDashboard,
    EmbeddedAsset,
    PackageVersion,
    ToolchainConfiguration,
    /// Reader compatibility for executable-input-closure v1 artifacts.
    /// Current collectors omit source-control provenance from executable input.
    SourceControlMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutableInput {
    pub path: String,
    pub sha256: String,
    pub category: InputCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutableInputContext {
    pub target: String,
    pub toolchain: String,
    pub cargo_profile: String,
    pub resolved_build_profile: BuildProfileConfiguration,
    pub features: Vec<String>,
    /// Values are SHA-256 digests of reviewed build-affecting values. Raw
    /// environment values, which may contain secrets, are never accepted.
    pub reviewed_environment_inputs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutableInputClosure {
    pub schema_version: String,
    pub context: ExecutableInputContext,
    pub inputs: Vec<ExecutableInput>,
}

impl ExecutableInputClosure {
    pub fn new(
        context: ExecutableInputContext,
        inputs: Vec<ExecutableInput>,
    ) -> Result<Self, CandidateError> {
        Self::canonicalize(context, inputs, false)
    }

    fn canonicalize(
        mut context: ExecutableInputContext,
        mut inputs: Vec<ExecutableInput>,
        allow_legacy_source_control_metadata: bool,
    ) -> Result<Self, CandidateError> {
        validate_nonempty("target", &context.target)?;
        validate_nonempty("toolchain", &context.toolchain)?;
        validate_nonempty("cargo_profile", &context.cargo_profile)?;
        context.resolved_build_profile.validate()?;

        context.features.sort();
        context.features.dedup();
        if context
            .features
            .iter()
            .any(|feature| feature.trim().is_empty())
        {
            return Err(CandidateError::new(
                "empty_feature",
                "feature names must not be empty",
            ));
        }
        for (name, digest) in &context.reviewed_environment_inputs {
            validate_nonempty("reviewed_environment_input_name", name)?;
            validate_sha256("reviewed_environment_input_sha256", digest)?;
        }

        inputs.sort_by(|left, right| left.path.cmp(&right.path));
        let mut paths = BTreeSet::new();
        for input in &inputs {
            if input.category == InputCategory::SourceControlMetadata
                && !allow_legacy_source_control_metadata
            {
                return Err(CandidateError::new(
                    "legacy_source_control_metadata_not_emittable",
                    "new executable-input closures must keep source-control provenance outside the functional input list",
                ));
            }
            validate_input_path(&input.path)?;
            validate_sha256("input_sha256", &input.sha256)?;
            if !paths.insert(&input.path) {
                return Err(CandidateError::new(
                    "duplicate_input_path",
                    format!("input path appears more than once: {}", input.path),
                ));
            }
        }
        if inputs.is_empty() {
            return Err(CandidateError::new(
                "empty_input_closure",
                "at least one executable input is required",
            ));
        }

        Ok(Self {
            schema_version: EXECUTABLE_INPUT_CLOSURE_SCHEMA_VERSION.to_string(),
            context,
            inputs,
        })
    }

    pub fn digest(&self) -> String {
        digest_serializable(self)
    }

    pub fn validate(&self) -> Result<(), CandidateError> {
        if self.schema_version != EXECUTABLE_INPUT_CLOSURE_SCHEMA_VERSION {
            return Err(CandidateError::new(
                "unsupported_input_closure_schema",
                "executable-input closure schema is not supported",
            ));
        }
        let canonical = Self::canonicalize(self.context.clone(), self.inputs.clone(), true)?;
        if canonical != *self {
            return Err(CandidateError::new(
                "noncanonical_input_closure",
                "features and executable inputs must use canonical order without duplicates",
            ));
        }
        Ok(())
    }

    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.digest() == other.digest()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactClass {
    FastIteration,
    ProductionShaped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildProfileConfiguration {
    pub opt_level: String,
    pub lto: String,
    pub codegen_units: u32,
    pub strip: bool,
}

impl BuildProfileConfiguration {
    pub fn production_release() -> Self {
        Self {
            opt_level: "3".to_string(),
            lto: "fat".to_string(),
            codegen_units: 1,
            strip: true,
        }
    }

    pub fn digest(&self) -> String {
        digest_serializable(self)
    }

    fn validate(&self) -> Result<(), CandidateError> {
        validate_nonempty("profile_opt_level", &self.opt_level)?;
        validate_nonempty("profile_lto", &self.lto)?;
        if self.codegen_units == 0 {
            return Err(CandidateError::new(
                "invalid_codegen_units",
                "build profile codegen units must be positive",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTreeState {
    Clean,
    Dirty,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceProvenance {
    pub commit: String,
    pub tree: String,
    pub state: SourceTreeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidateManifest {
    pub schema_version: String,
    pub candidate_id: String,
    pub source: SourceProvenance,
    pub executable_input_sha256: String,
    pub target: String,
    pub toolchain: String,
    pub cargo_profile: String,
    pub features: Vec<String>,
    pub reviewed_environment_input_sha256: String,
    pub embedded_dashboard_sha256: String,
    pub embedded_asset_digests: BTreeMap<String, String>,
    pub binary_sha256: String,
    pub support_manifest_sha256: String,
    pub created_at: String,
    pub artifact_class: ArtifactClass,
    pub resolved_build_profile: BuildProfileConfiguration,
    pub resolved_build_profile_sha256: String,
    pub validation_receipts: Vec<String>,
}

impl CandidateManifest {
    pub fn new(
        source: SourceProvenance,
        closure: &ExecutableInputClosure,
        artifact_class: ArtifactClass,
        binary_sha256: String,
        support_manifest_sha256: String,
        created_at: String,
    ) -> Result<Self, CandidateError> {
        validate_git_commit(&source.commit)?;
        validate_sha256("source_tree", &source.tree)?;
        validate_sha256("binary_sha256", &binary_sha256)?;
        validate_sha256("support_manifest_sha256", &support_manifest_sha256)?;
        if !created_at.contains('T') || !created_at.ends_with('Z') {
            return Err(CandidateError::new(
                "invalid_creation_time",
                "creation time must be an RFC3339 UTC value",
            ));
        }

        let dashboard_inputs = category_inputs(closure, InputCategory::EmbeddedDashboard);
        let embedded_asset_digests = closure
            .inputs
            .iter()
            .filter(|input| input.category == InputCategory::EmbeddedAsset)
            .map(|input| (input.path.clone(), input.sha256.clone()))
            .collect::<BTreeMap<_, _>>();
        if artifact_class == ArtifactClass::ProductionShaped
            && (dashboard_inputs.is_empty() || embedded_asset_digests.is_empty())
        {
            return Err(CandidateError::new(
                "production_embedded_output_incomplete",
                "production-shaped candidates must bind dashboard output and embedded support assets",
            ));
        }

        let executable_input_sha256 = closure.digest();
        let embedded_dashboard_sha256 = digest_serializable(&dashboard_inputs);
        let reviewed_environment_input_sha256 =
            digest_serializable(&closure.context.reviewed_environment_inputs);
        let candidate_id = format!(
            "candidate-{}-{}",
            &executable_input_sha256[..16],
            &binary_sha256[..16]
        );

        Ok(Self {
            schema_version: CANDIDATE_MANIFEST_SCHEMA_VERSION.to_string(),
            candidate_id,
            source,
            executable_input_sha256,
            target: closure.context.target.clone(),
            toolchain: closure.context.toolchain.clone(),
            cargo_profile: closure.context.cargo_profile.clone(),
            features: closure.context.features.clone(),
            reviewed_environment_input_sha256,
            embedded_dashboard_sha256,
            embedded_asset_digests,
            binary_sha256,
            support_manifest_sha256,
            created_at,
            artifact_class,
            resolved_build_profile: closure.context.resolved_build_profile.clone(),
            resolved_build_profile_sha256: closure.context.resolved_build_profile.digest(),
            validation_receipts: Vec::new(),
        })
    }

    pub fn can_reuse_artifact_for(&self, other: &Self) -> bool {
        self.executable_input_sha256 == other.executable_input_sha256
            && self.target == other.target
            && self.toolchain == other.toolchain
            && self.cargo_profile == other.cargo_profile
            && self.features == other.features
            && self.reviewed_environment_input_sha256 == other.reviewed_environment_input_sha256
            && self.artifact_class == other.artifact_class
    }

    pub fn validate_against_closure(
        &self,
        closure: &ExecutableInputClosure,
    ) -> Result<(), CandidateError> {
        self.validate_internal()?;
        closure.validate()?;
        let mut canonical = Self::new(
            self.source.clone(),
            closure,
            self.artifact_class,
            self.binary_sha256.clone(),
            self.support_manifest_sha256.clone(),
            self.created_at.clone(),
        )?;
        canonical.validation_receipts = self.validation_receipts.clone();
        if canonical != *self {
            return Err(CandidateError::new(
                "candidate_manifest_inconsistent",
                "candidate manifest does not match its executable-input closure",
            ));
        }
        Ok(())
    }

    pub fn validate_internal(&self) -> Result<(), CandidateError> {
        if self.schema_version != CANDIDATE_MANIFEST_SCHEMA_VERSION {
            return Err(CandidateError::new(
                "unsupported_candidate_manifest_schema",
                "candidate manifest schema is not supported",
            ));
        }
        validate_git_commit(&self.source.commit)?;
        validate_sha256("source_tree", &self.source.tree)?;
        validate_sha256("executable_input_sha256", &self.executable_input_sha256)?;
        validate_sha256(
            "reviewed_environment_input_sha256",
            &self.reviewed_environment_input_sha256,
        )?;
        validate_sha256("embedded_dashboard_sha256", &self.embedded_dashboard_sha256)?;
        validate_sha256("binary_sha256", &self.binary_sha256)?;
        validate_sha256("support_manifest_sha256", &self.support_manifest_sha256)?;
        validate_sha256(
            "resolved_build_profile_sha256",
            &self.resolved_build_profile_sha256,
        )?;
        validate_nonempty("target", &self.target)?;
        validate_nonempty("toolchain", &self.toolchain)?;
        validate_nonempty("cargo_profile", &self.cargo_profile)?;
        self.resolved_build_profile.validate()?;
        if self.resolved_build_profile.digest() != self.resolved_build_profile_sha256 {
            return Err(CandidateError::new(
                "build_profile_digest_mismatch",
                "resolved build-profile fields do not match their digest",
            ));
        }
        let expected_candidate_id = format!(
            "candidate-{}-{}",
            &self.executable_input_sha256[..16],
            &self.binary_sha256[..16]
        );
        if self.candidate_id != expected_candidate_id {
            return Err(CandidateError::new(
                "candidate_id_mismatch",
                "candidate ID does not match executable-input and binary digests",
            ));
        }
        if self.features.windows(2).any(|pair| pair[0] >= pair[1])
            || self
                .features
                .iter()
                .any(|feature| feature.trim().is_empty())
        {
            return Err(CandidateError::new(
                "noncanonical_features",
                "features must be nonempty, sorted, and unique",
            ));
        }
        for (path, digest) in &self.embedded_asset_digests {
            validate_input_path(path)?;
            validate_sha256("embedded_asset_sha256", digest)?;
        }
        if self.artifact_class == ArtifactClass::ProductionShaped
            && self.embedded_asset_digests.is_empty()
        {
            return Err(CandidateError::new(
                "production_embedded_output_incomplete",
                "production-shaped candidates must bind embedded support assets",
            ));
        }
        if !self.created_at.contains('T') || !self.created_at.ends_with('Z') {
            return Err(CandidateError::new(
                "invalid_creation_time",
                "creation time must be an RFC3339 UTC value",
            ));
        }
        let mut canonical_receipts = self.validation_receipts.clone();
        canonical_receipts.sort();
        canonical_receipts.dedup();
        if canonical_receipts != self.validation_receipts
            || canonical_receipts
                .iter()
                .any(|locator| locator.trim().is_empty())
        {
            return Err(CandidateError::new(
                "noncanonical_validation_receipts",
                "validation receipt locators must be nonempty, sorted, and unique",
            ));
        }
        Ok(())
    }
}

fn validate_nonempty(field: &'static str, value: &str) -> Result<(), CandidateError> {
    if value.trim().is_empty() {
        return Err(CandidateError::new(
            "empty_field",
            format!("{field} must not be empty"),
        ));
    }
    Ok(())
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), CandidateError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CandidateError::new(
            "invalid_sha256",
            format!("{field} must be a lowercase hexadecimal SHA-256 digest"),
        ));
    }
    Ok(())
}

fn validate_git_commit(value: &str) -> Result<(), CandidateError> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CandidateError::new(
            "invalid_source_commit",
            "source commit must be a lowercase 40-character Git object ID",
        ));
    }
    Ok(())
}

fn validate_input_path(path: &str) -> Result<(), CandidateError> {
    let invalid = path.trim().is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..");
    if invalid {
        return Err(CandidateError::new(
            "invalid_input_path",
            format!("input path must be normalized and repository-relative: {path}"),
        ));
    }
    Ok(())
}

fn category_inputs(
    closure: &ExecutableInputClosure,
    category: InputCategory,
) -> Vec<&ExecutableInput> {
    closure
        .inputs
        .iter()
        .filter(|input| input.category == category)
        .collect()
}

fn digest_serializable(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("validated candidate values are serializable");
    format!("{:x}", Sha256::digest(bytes))
}
