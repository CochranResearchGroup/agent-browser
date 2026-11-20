//! Immutable provenance for a relative lifetime minted from StableTime bounds.
//! This record contains no clock, extrapolation, client or I/O implementation.
use serde::{Deserialize, Serialize};

pub const SERVICE_TIME_ORIGIN_SCHEMA_V1: &str = "agent-browser.service-time-origin.v1";

/// Service UTC bounds at the documented observation instant. A missing origin
/// on a legacy relative lifetime is not trusted time evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceTimeOrigin {
    pub schema_version: String,
    pub epoch: String,
    pub earliest_utc_ms: u64,
    pub latest_utc_ms: u64,
}

impl ServiceTimeOrigin {
    pub fn new(
        epoch: String,
        earliest_utc_ms: u64,
        latest_utc_ms: u64,
    ) -> Result<Self, &'static str> {
        let origin = Self {
            schema_version: SERVICE_TIME_ORIGIN_SCHEMA_V1.into(),
            epoch,
            earliest_utc_ms,
            latest_utc_ms,
        };
        origin.validate()?;
        Ok(origin)
    }

    /// Validate only provenance shape. The owning adapter obtains it from the
    /// supported authority; caller-supplied timestamps cannot mint this record.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != SERVICE_TIME_ORIGIN_SCHEMA_V1
            || self.epoch.is_empty()
            || self.epoch.len() > 128
            || self.epoch.chars().any(char::is_control)
            || self.latest_utc_ms < self.earliest_utc_ms
            || self.latest_utc_ms > (i64::MAX as u64) / 1_000_000
        {
            return Err("service_time_origin_invalid");
        }
        Ok(())
    }

    /// Choose the conservative earliest-origin deadline, never a later grant.
    /// If uncertainty already spans that window, no new lifetime is qualified.
    pub fn deadline_ms(&self, lifetime_ms: u64) -> Result<u64, &'static str> {
        self.validate()?;
        let deadline = self
            .earliest_utc_ms
            .checked_add(lifetime_ms)
            .filter(|_| lifetime_ms > 0)
            .ok_or("service_time_origin_deadline_invalid")?;
        if self.latest_utc_ms >= deadline {
            return Err("service_time_origin_window_uncertain");
        }
        Ok(deadline)
    }
}
