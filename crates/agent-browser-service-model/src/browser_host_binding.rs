//! App context is task intent, not proof of desktop or browser authority.
use serde::{Deserialize, Serialize};

use crate::{BrowserPlacement, RemoteViewFixedDesktop};

/// Versioned app context. Routing is configured by the host, never supplied as a URL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserHostBinding {
    pub version: u32,
    pub host_id: String,
    pub participant_id: String,
    pub session_id: String,
    pub placement: BrowserPlacement,
}

impl BrowserHostBinding {
    /// Identifiers are opaque and bounded. Current provider observations separately
    /// qualify the requested placement; attribution never substitutes for them.
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("browser_host_binding_version_unsupported".into());
        }
        for value in [&self.host_id, &self.participant_id, &self.session_id] {
            if value.is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
                return Err("browser_host_binding_identity_invalid".into());
            }
        }
        Ok(())
    }
}

/// The actual accepted desktop is retained independently of requested placement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserHostTaskBinding {
    pub request: BrowserHostBinding,
    pub desktop: RemoteViewFixedDesktop,
}
