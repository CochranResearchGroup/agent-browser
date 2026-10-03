//! Revision 3 public application requests, independent of provider internals.
//!
//! This is the external lifecycle subset of `remote_view_consumer`. Agent
//! Browser owns launch and stop, so managed placement verbs are excluded.
use serde::{Deserialize, Serialize};

pub const REMOTE_VIEW_APPLICATION_SOURCE_CHECKPOINT: &str =
    "da540f22a6ff851272c5e9b91d7e3117a28bf6cd";

/// Resource selection is not a credential or an authenticated principal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteViewApplicationEnvelope {
    pub application: String,
    pub request: RemoteViewApplicationRequest,
}

/// Exact published external-mode request shape. Requests use snake_case;
/// the nested release acknowledgement deliberately uses camelCase.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteViewApplicationRequest {
    Inventory {},
    Acquire {
        pool_name: String,
        idempotency_key: String,
    },
    ObserveAssignment {
        assignment_id: String,
        expected_generation: u64,
    },
    LaunchEnvironment {
        assignment_id: String,
        expected_generation: u64,
        expected_viewing_generation: u64,
    },
    Windows {
        assignment_id: String,
        expected_generation: u64,
        expected_viewing_generation: u64,
    },
    Activate {
        assignment_id: String,
        expected_generation: u64,
        expected_viewing_generation: u64,
        window_id: u32,
        idempotency_key: String,
    },
    Events {
        assignment_id: String,
        expected_generation: u64,
        after: Option<u64>,
        limit: usize,
    },
    IssueView {
        assignment_id: String,
        expected_generation: u64,
        expected_viewing_generation: u64,
        audience: String,
        capability: RemoteViewApplicationViewCapability,
        lifetime_seconds: u32,
        idempotency_key: String,
    },
    ResolveView {
        route_id: String,
        audience: String,
    },
    /// Current installed transport observation; grant validity alone is insufficient.
    ObserveView {
        route_id: String,
        audience: String,
    },
    ResolveEmbedView {
        route_id: String,
    },
    RevokeView {
        route_id: String,
        audience: String,
    },
    Release {
        assignment_id: String,
        expected_generation: u64,
        idempotency_key: String,
        cleanup: RemoteViewApplicationCleanup,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteViewApplicationViewCapability {
    Observe,
    Control,
}

/// Acknowledgement of application-owned obligations, never provider cleanup.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationCleanup {
    pub schema_version: u32,
    pub assignment_id: String,
    pub lifecycle_generation: u64,
    pub application_references_clear: bool,
    pub pending_recovery_clear: bool,
    pub foreground_leases_clear: bool,
    pub cleanup_tasks_clear: bool,
}

impl RemoteViewApplicationCleanup {
    /// Check every acknowledgement against the exact release target.
    /// Runtime adapters must derive these assertions from current Agent Browser
    /// evidence; a retained-peer count alone cannot establish this result.
    pub fn validates_target(&self, assignment_id: &str, lifecycle_generation: u64) -> bool {
        self.schema_version == 1
            && !assignment_id.is_empty()
            && lifecycle_generation > 0
            && self.assignment_id == assignment_id
            && self.lifecycle_generation == lifecycle_generation
            && self.application_references_clear
            && self.pending_recovery_clear
            && self.foreground_leases_clear
            && self.cleanup_tasks_clear
    }
}
