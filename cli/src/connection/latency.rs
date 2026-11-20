//! Opt-in phase sampling at the StableTime service measurement instants.
//!
//! Only the operator-selected phase contacts the supported time client. Ordinary
//! requests do not acquire time. A missing sample never blocks browser work.
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Clone)]
struct Client {
    binary: String,
    config: PathBuf,
}

impl Client {
    fn configured() -> Result<Self, &'static str> {
        let config = std::env::var_os("AGENT_BROWSER_LATENCY_STABLE_TIME_CLIENT")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or("stable_time_client_configuration_missing")?;
        Ok(Self {
            binary: std::env::var("AGENT_BROWSER_LATENCY_STABLE_TIME_BINARY")
                .unwrap_or_else(|_| "stable-time".into()),
            config,
        })
    }

    fn observe(&self) -> Result<Observation, &'static str> {
        // Credential reading, pinned HTTPS and response validation remain in the
        // installed supported client. No private counter or clock bridge starts.
        let mut child = Command::new(&self.binary)
            .arg("mcp")
            .arg(&self.config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "stable_time_client_start_failed")?;
        let requests = concat!(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"clientInfo\":{\"name\":\"agent-browser-latency\",\"version\":\"1\"}}}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"get_time\",\"arguments\":{}}}\n"
        );
        let written = child
            .stdin
            .take()
            .ok_or("stable_time_client_input_unavailable")?
            .write_all(requests.as_bytes());
        if written.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err("stable_time_client_input_failed");
        }
        let output = child
            .wait_with_output()
            .map_err(|_| "stable_time_client_response_unavailable")?;
        if !output.status.success() || output.stdout.len() > 131_072 {
            return Err("stable_time_client_response_invalid");
        }
        let response = output
            .stdout
            .split(|byte| *byte == b'\n')
            .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
            .find(|response| response["id"] == 2)
            .ok_or("stable_time_client_response_invalid")?;
        if response["error"].is_object() || response["result"]["isError"] == true {
            return Err("stable_time_client_request_failed");
        }
        let text = response["result"]["content"]
            .as_array()
            .and_then(|blocks| blocks.iter().find_map(|block| block["text"].as_str()))
            .ok_or("stable_time_client_response_invalid")?;
        let observation: Observation =
            serde_json::from_str(text).map_err(|_| "stable_time_client_response_invalid")?;
        observation.validate()?;
        Ok(observation)
    }
}

#[derive(Deserialize)]
struct Observation {
    contract_version: String,
    epoch: String,
    status: String,
    bound: String,
    authentication: String,
    measurement: Measurement,
    counter_policy: Option<Policy>,
    estimate: Option<Estimate>,
}

#[derive(Deserialize)]
struct Measurement {
    counter: String,
    frequency_hz: String,
}

#[derive(Deserialize)]
struct Policy {
    rate_ppm: u32,
    maximum_age_ns: u64,
}

#[derive(Deserialize)]
struct Estimate {
    earliest_utc: String,
    latest_utc: String,
}

impl Observation {
    fn validate(&self) -> Result<(), &'static str> {
        if self.contract_version != "2.0"
            || self.bound != "finite"
            || !matches!(
                self.status.as_str(),
                "qualified_live" | "qualified_holdover"
            )
            || self.authentication != "NTS-verified"
        {
            return Err("stable_time_service_unqualified");
        }
        if uuid::Uuid::parse_str(&self.epoch).is_err()
            || self.measurement.counter.parse::<u64>().is_err()
            || self
                .measurement
                .frequency_hz
                .parse::<u64>()
                .ok()
                .unwrap_or(0)
                == 0
            || !self
                .counter_policy
                .as_ref()
                .is_some_and(|policy| policy.rate_ppm < 1_000_000 && policy.maximum_age_ns > 0)
        {
            return Err("stable_time_client_response_invalid");
        }
        let estimate = self
            .estimate
            .as_ref()
            .ok_or("stable_time_client_response_invalid")?;
        if utc_ns(&estimate.earliest_utc)? > utc_ns(&estimate.latest_utc)? {
            return Err("stable_time_client_response_invalid");
        }
        Ok(())
    }
}

fn utc_ns(value: &str) -> Result<i64, &'static str> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .and_then(|time| time.timestamp_nanos_opt())
        .ok_or("stable_time_client_response_invalid")
}

fn service_span(before: &Observation, after: &Observation) -> Result<Value, &'static str> {
    if before.epoch != after.epoch
        || before.measurement.frequency_hz != after.measurement.frequency_hz
    {
        return Err("stable_time_measurement_continuity_changed");
    }
    let first = before.measurement.counter.parse::<u64>().unwrap();
    let last = after.measurement.counter.parse::<u64>().unwrap();
    let ticks = last
        .checked_sub(first)
        .ok_or("stable_time_measurement_continuity_changed")?;
    let frequency = before.measurement.frequency_hz.parse::<u64>().unwrap();
    let first_policy = before.counter_policy.as_ref().unwrap();
    let last_policy = after.counter_policy.as_ref().unwrap();
    let rate = first_policy.rate_ppm.max(last_policy.rate_ppm);
    // StableTime's documented conditional rate envelope divides by the fast/
    // slow frequency extremes, rounded outward. Account for endpoint tick phase.
    let denominator = u128::from(frequency) * u128::from(1_000_000 - rate);
    let lower = u128::from(ticks.saturating_sub(1)) * 1_000_000_000_000_000
        / (u128::from(frequency) * u128::from(1_000_000 + rate));
    let upper = ((u128::from(ticks) + 1) * 1_000_000_000_000_000).div_ceil(denominator);
    if upper > u128::from(first_policy.maximum_age_ns.min(last_policy.maximum_age_ns)) {
        return Err("stable_time_measurement_horizon_exceeded");
    }
    let before_utc = before.estimate.as_ref().unwrap();
    let after_utc = after.estimate.as_ref().unwrap();
    let utc_lower = (i128::from(utc_ns(&after_utc.earliest_utc)?)
        - i128::from(utc_ns(&before_utc.latest_utc)?))
    .max(0);
    let utc_upper =
        i128::from(utc_ns(&after_utc.latest_utc)?) - i128::from(utc_ns(&before_utc.earliest_utc)?);
    if utc_upper < 0 || lower > utc_upper as u128 || utc_lower as u128 > upper {
        return Err("stable_time_measurement_evidence_disagrees");
    }
    Ok(json!({
        "epoch":before.epoch,"conditionalRatePpm":rate,
        "serviceSpanNs":{"lower":lower.to_string(),"upper":upper.to_string()},
        "utcServiceSpanNs":{"lower":utc_lower.to_string(),"upper":utc_upper.to_string()},
        // Time between service observations contains response delivery and the
        // next request. Without a delivery bound only an upper phase bound exists.
        "operationElapsedNs":{"lower":"0","upper":upper.to_string()},
        "includesSamplingDelivery":true,
        "basis":"StableTime service measurement counter under declared conditional rate policy"
    }))
}

/// Capture one explicitly selected phase. Finish locally with the operation's
/// result and attach the returned diagnostic; never use it as dispatch authority.
pub(crate) struct PhaseCapture {
    phase: &'static str,
    active: Option<(Client, Result<Observation, &'static str>)>,
    configuration_error: Option<&'static str>,
}

impl PhaseCapture {
    pub(crate) fn start(phase: &'static str) -> Self {
        let mut capture = Self {
            phase,
            active: None,
            configuration_error: None,
        };
        if std::env::var("AGENT_BROWSER_LATENCY_PHASE").ok().as_deref() != Some(phase) {
            return capture;
        }
        match Client::configured() {
            Ok(client) => {
                let before = client.observe();
                capture.active = Some((client, before));
            }
            Err(reason) => capture.configuration_error = Some(reason),
        }
        capture
    }

    pub(crate) fn finish(self) -> Option<Value> {
        let result = if let Some(reason) = self.configuration_error {
            Err(reason)
        } else {
            let (client, before) = self.active?;
            before.and_then(|before| {
                client
                    .observe()
                    .and_then(|after| service_span(&before, &after))
            })
        };
        Some(match result {
            Ok(span) => {
                json!({"schemaVersion":"agent-browser.latency-phase.v1","phase":self.phase,"status":"measured","measurement":span})
            }
            Err(reason) => {
                json!({"schemaVersion":"agent-browser.latency-phase.v1","phase":self.phase,"status":"unavailable","reason":reason,"guestClockSubstituted":false})
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(counter: &str, utc: &str) -> Observation {
        serde_json::from_value(
            json!({"contract_version":"2.0","epoch":"00000000-0000-0000-0000-000000000001",
            "status":"qualified_live","bound":"finite","authentication":"NTS-verified",
            "measurement":{"counter":counter,"frequency_hz":"10000000"},
            "counter_policy":{"rate_ppm":5000,"maximum_age_ns":300000000000u64},
            "estimate":{"earliest_utc":utc,"latest_utc":utc}}),
        )
        .unwrap()
    }

    #[test]
    fn service_bracket_does_not_pretend_to_bound_delivery_corrected_phase_from_below() {
        let first = observation("10000000", "2026-11-20T00:00:00Z");
        let last = observation("20000000", "2026-11-20T00:00:01Z");
        first.validate().unwrap();
        last.validate().unwrap();
        let span = service_span(&first, &last).unwrap();
        assert_eq!(span["serviceSpanNs"]["lower"], "995024776");
        assert_eq!(span["serviceSpanNs"]["upper"], "1005025227");
        assert_eq!(span["operationElapsedNs"]["lower"], "0");
    }

    #[test]
    fn discontinuity_and_unbounded_service_evidence_remain_unavailable() {
        let first = observation("10000000", "2026-11-20T00:00:00Z");
        let mut last = observation("20000000", "2026-11-20T00:00:01Z");
        last.epoch = "00000000-0000-0000-0000-000000000002".into();
        assert_eq!(
            service_span(&first, &last).unwrap_err(),
            "stable_time_measurement_continuity_changed"
        );
        last.bound = "unbounded".into();
        assert_eq!(
            last.validate().unwrap_err(),
            "stable_time_service_unqualified"
        );
    }
}
