//! Supported StableTime service consumer shared by Native freshness and opt-in diagnostics.
//! OS waits bound the helper only; they never prove observation age.
use serde::{Deserialize, Serialize};
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
pub(super) struct Client {
    binary: String,
    config: PathBuf,
}

impl Client {
    pub(super) fn configured() -> Result<Self, &'static str> {
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

    #[cfg(not(test))]
    fn native_configured() -> Result<Self, &'static str> {
        let config = std::env::var_os("AGENT_BROWSER_STABLE_TIME_CLIENT")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|home| PathBuf::from(home).join(".local/share/stable-time/client.json"))
            })
            .filter(|path| path.is_absolute())
            .ok_or("stable_time_client_configuration_missing")?;
        Ok(Self {
            binary: std::env::var("AGENT_BROWSER_STABLE_TIME_BINARY")
                .unwrap_or_else(|_| "stable-time".into()),
            config,
        })
    }

    pub(super) fn observe(&self) -> Result<Observation, &'static str> {
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
pub(super) fn exchange(
    child: Child,
    input: &[u8],
    timeout: Duration,
) -> Result<Vec<u8>, &'static str> {
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
pub(super) fn exchange(
    child: Child,
    _input: &[u8],
    _timeout: Duration,
) -> Result<Vec<u8>, &'static str> {
    let _owned = ClientChild(child);
    Err("stable_time_client_bounded_transport_unsupported")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Observation {
    pub(super) contract_version: String,
    pub(super) epoch: String,
    pub(super) status: String,
    pub(super) bound: String,
    pub(super) authentication: String,
    pub(super) measurement: Measurement,
    pub(super) counter_policy: Option<Policy>,
    pub(super) estimate: Option<Estimate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Measurement {
    pub(super) counter: String,
    pub(super) frequency_hz: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Policy {
    pub(super) rate_ppm: u32,
    pub(super) maximum_age_ns: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Estimate {
    pub(super) earliest_utc: String,
    pub(super) latest_utc: String,
}

impl Observation {
    pub(super) fn validate(&self) -> Result<(), &'static str> {
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

pub(super) fn utc_ns(value: &str) -> Result<i64, &'static str> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .and_then(|time| time.timestamp_nanos_opt())
        .ok_or("stable_time_client_response_invalid")
}

/// Trusted service observation instant, carried with owner-minted image evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stamp {
    pub epoch: String,
    #[serde(with = "decimal")]
    pub counter: u64,
    #[serde(with = "decimal")]
    pub frequency_hz: u64,
    #[serde(with = "decimal")]
    pub earliest_utc_ns: i64,
    #[serde(with = "decimal")]
    pub latest_utc_ns: i64,
}
mod decimal {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<T: std::fmt::Display, S: Serializer>(
        value: &T,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, T: std::str::FromStr, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<T, D::Error>
    where
        T::Err: std::fmt::Display,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
impl Stamp {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if uuid::Uuid::parse_str(&self.epoch).is_err()
            || self.frequency_hz == 0
            || self.earliest_utc_ns < 0
            || self.latest_utc_ns < self.earliest_utc_ns
        {
            return Err(
                "browser_native_time_evidence_invalid: obtain a fresh native_capture".into(),
            );
        }
        Ok(())
    }
    pub(crate) fn earliest_ms(&self) -> u64 {
        self.earliest_utc_ns as u64 / 1_000_000
    }
    pub(crate) fn latest_ms(&self) -> u64 {
        (self.latest_utc_ns as u64).div_ceil(1_000_000)
    }
    pub(crate) fn fresh_at(&self, now: &Self, maximum_ms: u64) -> Result<(), String> {
        self.validate()?;
        now.validate()?;
        if self.epoch != now.epoch
            || self.frequency_hz != now.frequency_hz
            || now.counter < self.counter
        {
            return Err("browser_native_time_continuity_changed: obtain a fresh native_capture using qualified StableTime".into());
        }
        let limit = i128::from(maximum_ms) * 1_000_000;
        let upper = i128::from(now.latest_utc_ns) - i128::from(self.earliest_utc_ns);
        let lower = i128::from(now.earliest_utc_ns) - i128::from(self.latest_utc_ns);
        if upper < 0 {
            return Err(
                "browser_native_time_continuity_changed: obtain a fresh native_capture".into(),
            );
        }
        if lower > limit {
            return Err("browser_native_observation_expired: obtain a fresh native_capture; do not replay original input".into());
        }
        if upper > limit {
            return Err("browser_native_time_uncertain: StableTime bounds cannot qualify observation age; repair time qualification or obtain a fresh native_capture".into());
        }
        Ok(())
    }
}

/// Called only for Native freshness-dependent operations, never ordinary CDP.
pub(crate) fn native_now() -> Result<Stamp, String> {
    #[cfg(test)]
    return NATIVE_FIXTURE.with(|source| {
        source
            .borrow()
            .clone()
            .unwrap_or_else(|| Err("stable_time_client_fixture_missing".into()))
    });
    #[cfg(not(test))]
    {
        let observed = Client::native_configured().and_then(|client| client.observe())
            .map_err(|error| format!("browser_native_time_unavailable:cause={error}; repair the supported StableTime client/service and obtain a fresh native_capture; independent status/readback remains available"))?;
        let estimate = observed
            .estimate
            .as_ref()
            .ok_or("browser_native_time_evidence_invalid")?;
        let stamp = Stamp {
            epoch: observed.epoch.clone(),
            counter: observed
                .measurement
                .counter
                .parse()
                .map_err(|_| "browser_native_time_evidence_invalid")?,
            frequency_hz: observed
                .measurement
                .frequency_hz
                .parse()
                .map_err(|_| "browser_native_time_evidence_invalid")?,
            earliest_utc_ns: utc_ns(&estimate.earliest_utc).map_err(str::to_owned)?,
            latest_utc_ns: utc_ns(&estimate.latest_utc).map_err(str::to_owned)?,
        };
        stamp.validate()?;
        Ok(stamp)
    }
}
#[cfg(test)]
thread_local! { static NATIVE_FIXTURE: std::cell::RefCell<Option<Result<Stamp,String>>> = const { std::cell::RefCell::new(None) }; }
#[cfg(test)]
pub(crate) struct Fixture(Option<Result<Stamp, String>>);
#[cfg(test)]
impl Fixture {
    pub(crate) fn set(reading: Result<Stamp, String>) -> Self {
        Self(NATIVE_FIXTURE.with(|source| source.replace(Some(reading))))
    }
}
#[cfg(test)]
impl Drop for Fixture {
    fn drop(&mut self) {
        NATIVE_FIXTURE.with(|source| source.replace(self.0.take()));
    }
}

#[cfg(test)]
pub(crate) fn fixture_stamp(ms: u64) -> Stamp {
    Stamp {
        epoch: "11111111-1111-4111-8111-111111111111".into(),
        counter: ms,
        frequency_hz: 1000,
        earliest_utc_ns: (ms * 1_000_000) as i64,
        latest_utc_ns: (ms * 1_000_000) as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_age_uses_service_bounds_and_never_guest_wall_or_boot_time() {
        let captured = fixture_stamp(1_000_000);
        let current = fixture_stamp(1_000_010);
        // Neither a forward nor backward guest-wall reading participates.
        captured.fresh_at(&current, 10_000).unwrap();
        let mut uncertain = fixture_stamp(1_005_000);
        uncertain.latest_utc_ns = 1_015_000_000_000;
        assert!(captured
            .fresh_at(&uncertain, 10_000)
            .unwrap_err()
            .starts_with("browser_native_time_uncertain"));
        assert!(captured
            .fresh_at(&fixture_stamp(1_020_000), 10_000)
            .unwrap_err()
            .starts_with("browser_native_observation_expired"));
        assert!(captured
            .fresh_at(&fixture_stamp(999_999), 10_000)
            .unwrap_err()
            .starts_with("browser_native_time_continuity_changed"));
        let mut epoch = current;
        epoch.epoch = "22222222-2222-4222-8222-222222222222".into();
        assert!(captured
            .fresh_at(&epoch, 10_000)
            .unwrap_err()
            .starts_with("browser_native_time_continuity_changed"));
    }
    #[test]
    fn helper_failure_and_service_unqualified_have_distinct_causes() {
        let client = Client {
            binary: "/missing/qualified-stable-time-client".into(),
            config: "/unused/private/config.json".into(),
        };
        assert_eq!(
            client.observe().unwrap_err(),
            "stable_time_client_start_failed"
        );
        let observation: Observation = serde_json::from_value(json!({"contract_version":"2.0","epoch":"11111111-1111-4111-8111-111111111111","status":"unavailable","bound":"unbounded","authentication":"NTS-verified","measurement":{"counter":"1","frequency_hz":"1000"},"estimate":null})).unwrap();
        assert_eq!(
            observation.validate().unwrap_err(),
            "stable_time_service_unqualified"
        );
        let _unavailable = Fixture::set(Err(
            "browser_native_time_unavailable:cause=stable_time_client_start_failed".into(),
        ));
        assert!(native_now().unwrap_err().contains("client_start_failed"));
    }
}
