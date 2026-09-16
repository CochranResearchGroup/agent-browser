use serde::{Deserialize, Serialize};

use crate::{
    digest_serializable, validate_nonempty, validate_sha256, ArtifactClass,
    BuildProfileConfiguration, CandidateError, CandidateManifest, ExecutableInputClosure,
};

pub const BUILD_IDENTITY_SCHEMA_VERSION: &str = "agent-browser.candidate-build-identity.v1";
pub const SEALED_ARTIFACT_SCHEMA_VERSION: &str = "agent-browser.sealed-artifact.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildIdentity {
    pub schema_version: String,
    pub executable_input_sha256: String,
    pub artifact_class: ArtifactClass,
    pub target: String,
    pub toolchain: String,
    pub cargo_profile: String,
    pub features: Vec<String>,
    pub reviewed_environment_input_sha256: String,
    pub resolved_build_profile: BuildProfileConfiguration,
    pub resolved_build_profile_sha256: String,
}

impl BuildIdentity {
    pub fn new(closure: &ExecutableInputClosure, artifact_class: ArtifactClass) -> Self {
        Self {
            schema_version: BUILD_IDENTITY_SCHEMA_VERSION.to_string(),
            executable_input_sha256: closure.digest(),
            artifact_class,
            target: closure.context.target.clone(),
            toolchain: closure.context.toolchain.clone(),
            cargo_profile: closure.context.cargo_profile.clone(),
            features: closure.context.features.clone(),
            reviewed_environment_input_sha256: digest_serializable(
                &closure.context.reviewed_environment_inputs,
            ),
            resolved_build_profile: closure.context.resolved_build_profile.clone(),
            resolved_build_profile_sha256: closure.context.resolved_build_profile.digest(),
        }
    }

    pub fn digest(&self) -> String {
        digest_serializable(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildOperationState {
    Building,
    Sealed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildOperation {
    pub operation_id: String,
    pub identity: BuildIdentity,
    pub state: BuildOperationState,
    pub output_directory: String,
}

impl BuildOperation {
    pub fn new(operation_id: impl Into<String>, identity: BuildIdentity) -> Self {
        let output_directory = isolated_output_directory(&identity);
        Self {
            operation_id: operation_id.into(),
            identity,
            state: BuildOperationState::Building,
            output_directory,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SealedArtifact {
    pub schema_version: String,
    pub operation_id: String,
    pub identity: BuildIdentity,
    pub binary_sha256: String,
    pub support_manifest_sha256: String,
    pub candidate_manifest_sha256: String,
    pub seal_sha256: String,
}

impl SealedArtifact {
    pub fn new(
        operation_id: impl Into<String>,
        identity: BuildIdentity,
        binary_sha256: String,
        support_manifest_sha256: String,
        candidate_manifest_sha256: String,
    ) -> Result<Self, CandidateError> {
        let operation_id = operation_id.into();
        validate_nonempty("operation_id", &operation_id)?;
        validate_sha256("binary_sha256", &binary_sha256)?;
        validate_sha256("support_manifest_sha256", &support_manifest_sha256)?;
        validate_sha256("candidate_manifest_sha256", &candidate_manifest_sha256)?;
        let seal_sha256 = artifact_seal_digest(
            &operation_id,
            &identity,
            &binary_sha256,
            &support_manifest_sha256,
            &candidate_manifest_sha256,
        );
        Ok(Self {
            schema_version: SEALED_ARTIFACT_SCHEMA_VERSION.to_string(),
            operation_id,
            identity,
            binary_sha256,
            support_manifest_sha256,
            candidate_manifest_sha256,
            seal_sha256,
        })
    }

    pub fn verify(&self) -> bool {
        self.schema_version == SEALED_ARTIFACT_SCHEMA_VERSION
            && self.seal_sha256
                == artifact_seal_digest(
                    &self.operation_id,
                    &self.identity,
                    &self.binary_sha256,
                    &self.support_manifest_sha256,
                    &self.candidate_manifest_sha256,
                )
    }

    /// Verify that one exact serialized candidate manifest is the document
    /// sealed by this artifact and that both describe the same build inputs
    /// and payload digests.
    pub fn validate_candidate_manifest(
        &self,
        manifest: &CandidateManifest,
        closure: &ExecutableInputClosure,
        observed_manifest_sha256: &str,
    ) -> Result<(), CandidateError> {
        validate_sha256("candidate_manifest_sha256", observed_manifest_sha256)?;
        if !self.verify() {
            return Err(CandidateError::new(
                "sealed_artifact_tampered",
                "sealed artifact fields do not match its seal digest",
            ));
        }
        if self.candidate_manifest_sha256 != observed_manifest_sha256 {
            return Err(CandidateError::new(
                "candidate_manifest_digest_mismatch",
                "candidate manifest bytes do not match the sealed digest",
            ));
        }
        manifest.validate_against_closure(closure)?;
        if self.identity != BuildIdentity::new(closure, manifest.artifact_class) {
            return Err(CandidateError::new(
                "sealed_artifact_identity_mismatch",
                "sealed build identity does not match the candidate manifest input closure",
            ));
        }
        if self.binary_sha256 != manifest.binary_sha256
            || self.support_manifest_sha256 != manifest.support_manifest_sha256
        {
            return Err(CandidateError::new(
                "sealed_artifact_payload_mismatch",
                "sealed payload digests do not match the candidate manifest",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildDecision {
    JoinActive {
        operation_id: String,
    },
    ReuseSealed {
        operation_id: String,
        binary_sha256: String,
    },
    StartIsolated {
        output_directory: String,
    },
}

pub fn coordinate_build(
    requested: &BuildIdentity,
    active: &[BuildOperation],
    sealed: &[SealedArtifact],
) -> Result<BuildDecision, CandidateError> {
    if let Some(operation) = active.iter().find(|operation| {
        operation.state == BuildOperationState::Building && operation.identity == *requested
    }) {
        return Ok(BuildDecision::JoinActive {
            operation_id: operation.operation_id.clone(),
        });
    }
    if let Some(artifact) = sealed
        .iter()
        .find(|artifact| artifact.identity == *requested)
    {
        if !artifact.verify() {
            return Err(CandidateError::new(
                "sealed_artifact_tampered",
                format!(
                    "sealed artifact for operation {} failed digest verification",
                    artifact.operation_id
                ),
            ));
        }
        return Ok(BuildDecision::ReuseSealed {
            operation_id: artifact.operation_id.clone(),
            binary_sha256: artifact.binary_sha256.clone(),
        });
    }
    Ok(BuildDecision::StartIsolated {
        output_directory: isolated_output_directory(requested),
    })
}

fn isolated_output_directory(identity: &BuildIdentity) -> String {
    format!("candidate-builds/{}", &identity.digest()[..32])
}

fn artifact_seal_digest(
    operation_id: &str,
    identity: &BuildIdentity,
    binary_sha256: &str,
    support_manifest_sha256: &str,
    candidate_manifest_sha256: &str,
) -> String {
    digest_serializable(&(
        SEALED_ARTIFACT_SCHEMA_VERSION,
        operation_id,
        identity,
        binary_sha256,
        support_manifest_sha256,
        candidate_manifest_sha256,
    ))
}
