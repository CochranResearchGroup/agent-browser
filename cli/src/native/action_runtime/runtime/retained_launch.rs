//! Legacy retained-owner launch repair is intentionally absent from the
//! ordinary runtime. Browser Session Manager reconstructs session state from
//! SQLite instead of repairing historical owner projections.

use serde_json::Value;

pub(super) async fn recover(_session: &str, _command: &Value) -> Result<(), String> {
    Ok(())
}
