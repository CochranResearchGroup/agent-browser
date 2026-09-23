//! Compatibility boundary for commands already routed to a Browser Session
//! Manager lane.
//!
//! Legacy cold-navigation acquisition derived admission from profile and owner
//! records. The ordinary runtime no longer manufactures an admission claim;
//! Browser Session Manager performs session and tab selection directly.

use super::DaemonState;
use crate::native::service_model::ServiceState;
use serde_json::Value;

pub(super) fn admit_cold_navigation(
    _command: &Value,
    _daemon: &DaemonState,
    _snapshot: &ServiceState,
) -> Result<Option<Value>, String> {
    Ok(None)
}
