//! Opt-in phase sampling at the StableTime service measurement instants.
//!
//! Only the operator-selected phase contacts the supported time client. Ordinary
//! requests do not acquire time. A missing sample never blocks browser work.
use serde::Deserialize;
use serde_json::{json, Value};
#[cfg(unix)]
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
#[cfg(unix)]
use std::time::Instant;

const MAX_RESPONSE_BYTES: usize = 131_072;
const CLIENT_WAIT: Duration = Duration::from_secs(5);

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
        let child = Command::new(&self.binary)
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
        let output = exchange(child, requests.as_bytes(), CLIENT_WAIT)?;
        let response = output
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

/// Own only the helper spawned for this observation. Never signal a discovered
/// service or a process selected by a stale PID readback.
struct ClientChild(Child);

impl Drop for ClientChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

#[cfg(unix)]
fn exchange(child: Child, input: &[u8], timeout: Duration) -> Result<Vec<u8>, &'static str> {
    use std::os::fd::AsRawFd;
    fn nonblocking(pipe: &impl AsRawFd) -> Result<(), &'static str> {
        // SAFETY: the descriptor belongs to this live owned child pipe. Flags
        // change only this observation's pipe and are checked before I/O.
        let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
        {
            return Err("stable_time_client_transport_unavailable");
        }
        Ok(())
    }
    let mut child = ClientChild(child);
    let stdin = child
        .0
        .stdin
        .take()
        .ok_or("stable_time_client_input_unavailable")?;
    let mut stdout = child
        .0
        .stdout
        .take()
        .ok_or("stable_time_client_response_unavailable")?;
    nonblocking(&stdin)?;
    nonblocking(&stdout)?;
    let deadline = Instant::now() + timeout;
    let mut stdin = Some(stdin);
    let mut written = 0;
    let mut bytes = Vec::new();
    let mut eof = false;
    loop {
        if Instant::now() >= deadline {
            return Err("stable_time_client_timeout");
        }
        let mut progressed = false;
        if let Some(pipe) = &mut stdin {
            match pipe.write(&input[written..]) {
                Ok(count) => {
                    written += count;
                    progressed = count > 0;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return Err("stable_time_client_input_failed"),
            }
            if written == input.len() {
                stdin.take();
            }
        }
        if !eof {
            let mut chunk = [0; 4096];
            // The extra byte detects overflow while allocation stays bounded.
            let available = (MAX_RESPONSE_BYTES + 1 - bytes.len()).min(chunk.len());
            match stdout.read(&mut chunk[..available]) {
                Ok(0) => eof = true,
                Ok(count) => {
                    bytes.extend_from_slice(&chunk[..count]);
                    if bytes.len() > MAX_RESPONSE_BYTES {
                        return Err("stable_time_client_response_too_large");
                    }
                    progressed = true;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return Err("stable_time_client_response_unavailable"),
            }
        }
        match child.0.try_wait() {
            Ok(Some(status)) if !status.success() => {
                return Err("stable_time_client_response_invalid")
            }
            Ok(Some(_)) if eof => return Ok(bytes),
            Ok(_) => {}
            Err(_) => return Err("stable_time_client_wait_failed"),
        }
        if !progressed {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[cfg(not(unix))]
fn exchange(child: Child, _input: &[u8], _timeout: Duration) -> Result<Vec<u8>, &'static str> {
    let _owned = ClientChild(child);
    Err("stable_time_client_bounded_transport_unsupported")
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

    pub(crate) fn append_response(data: &mut Option<Value>, samples: Vec<Value>) {
        if !samples.is_empty() {
            append(data.get_or_insert(Value::Null), samples);
        }
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

/// Attach diagnostics only to the response envelope, never request identity.
pub(crate) fn append(data: &mut Value, samples: impl IntoIterator<Item = Value>) {
    let samples: Vec<_> = samples.into_iter().collect();
    if samples.is_empty() {
        return;
    }
    if data.is_null() {
        *data = json!({});
    }
    if let Some(object) = data.as_object_mut() {
        let phases = object.entry("latencyPhases").or_insert_with(|| json!([]));
        if let Some(phases) = phases.as_array_mut() {
            phases.extend(samples);
        }
    }
}

pub(crate) fn finish(capture: PhaseCapture, scope: &'static str, samples: &mut Vec<Value>) {
    if let Some(mut sample) = capture.finish() {
        sample["scope"] = json!(scope);
        samples.push(sample);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn empty_response_diagnostics_preserve_absence_and_selected_samples_are_response_only() {
        let command = serde_json::json!({"id":"same-id","action":"evaluate","script":"1"});
        let original = command.clone();
        let mut data = None;
        super::append_response(&mut data, Vec::new());
        assert!(data.is_none());
        super::append_response(
            &mut data,
            vec![serde_json::json!({"phase":"routing","measurement":1})],
        );
        super::append_response(
            &mut data,
            vec![serde_json::json!({"phase":"routing","measurement":2})],
        );
        assert_eq!(
            data.as_ref().unwrap()["latencyPhases"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(command, original);
    }

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

    #[cfg(target_os = "linux")]
    #[test]
    fn timing_client_bounds_output_and_reaps_its_hung_helper() {
        let spawn = |binary: &str, arguments: &[&str]| {
            Command::new(binary)
                .args(arguments)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
        };
        let child = spawn("/bin/sleep", &["30"]);
        let pid = child.id();
        assert_eq!(
            exchange(child, b"request", Duration::from_millis(50)).unwrap_err(),
            "stable_time_client_timeout"
        );
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
        let child = spawn("/usr/bin/head", &["-c", "131073", "/dev/zero"]);
        let pid = child.id();
        assert_eq!(
            exchange(child, b"", Duration::from_secs(2)).unwrap_err(),
            "stable_time_client_response_too_large"
        );
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
        let child = spawn("/bin/cat", &[]);
        assert_eq!(
            exchange(child, b"complete", Duration::from_secs(2)).unwrap(),
            b"complete"
        );
    }
}
