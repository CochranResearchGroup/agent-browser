//! Service-generated connection identity binds custody to a host lifetime.
//! Unknown legacy or unreadable process evidence never authorizes reconnect.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::process_identity::{current_boot_epoch, observe_process, ProcessObservation};

const PREFIX: &str = "connection-v2-";

#[derive(Clone, Serialize, Deserialize)]
struct HostLifetime {
    boot: String,
    namespace: String,
    pid: u32,
    start: String,
}

#[derive(Serialize, Deserialize)]
struct ConnectionLifetime {
    host: HostLifetime,
    nonce: String,
}

fn process_namespace() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_link("/proc/self/ns/pid")
            .ok()
            .map(|path| path.to_string_lossy().into_owned())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Some("native".to_string())
    }
}

/// Minted only by the service; caller-supplied connection IDs are overwritten.
pub(crate) fn new_connection_id() -> String {
    static HOST: OnceLock<Option<HostLifetime>> = OnceLock::new();
    let host = HOST.get_or_init(|| {
        let ProcessObservation::Observed(process) = observe_process(std::process::id()) else {
            return None;
        };
        Some(HostLifetime {
            boot: current_boot_epoch()?,
            namespace: process_namespace()?,
            pid: process.pid,
            start: process.start_token?,
        })
    });
    let nonce = uuid::Uuid::new_v4().to_string();
    match host {
        Some(host) => {
            let identity = ConnectionLifetime {
                host: host.clone(),
                nonce,
            };
            format!(
                "{PREFIX}{}",
                URL_SAFE_NO_PAD.encode(
                    serde_json::to_vec(&identity).expect("connection lifetime is serializable")
                )
            )
        }
        None => format!("connection-{nonce}"),
    }
}

/// A different boot, a missing PID, or a changed start token proves the old
/// host ended. Namespace differences and failed observations are inconclusive.
pub(crate) fn connection_host_ended(connection: &str) -> bool {
    let Some(encoded) = connection.strip_prefix(PREFIX) else {
        return false;
    };
    let Some(identity) = URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ConnectionLifetime>(&bytes).ok())
    else {
        return false;
    };
    let (Some(boot), Some(namespace)) = (current_boot_epoch(), process_namespace()) else {
        return false;
    };
    host_ended(&identity.host, &boot, &namespace, || {
        observe_process(identity.host.pid)
    })
}

fn host_ended(
    host: &HostLifetime,
    boot: &str,
    namespace: &str,
    observe: impl FnOnce() -> ProcessObservation,
) -> bool {
    if host.boot.is_empty() || host.start.is_empty() || host.pid == 0 {
        return false;
    }
    if host.boot != boot {
        return true;
    }
    if host.namespace != namespace {
        return false;
    }
    match observe() {
        ProcessObservation::Missing => true,
        ProcessObservation::Observed(process) => {
            process.start_token.is_some_and(|start| start != host.start)
        }
        ProcessObservation::Failed { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_identity::ObservedProcessIdentity;

    #[test]
    fn host_lifetime_requires_positive_death_evidence() {
        let host = HostLifetime {
            boot: "boot-a".into(),
            namespace: "ns-a".into(),
            pid: 7,
            start: "start-a".into(),
        };
        let observed = |start| {
            ProcessObservation::Observed(ObservedProcessIdentity {
                pid: 7,
                start_token: start,
                executable_path: None,
                browser_family: None,
                command_line: None,
            })
        };
        assert!(!host_ended(&host, "boot-a", "ns-a", || observed(Some(
            "start-a".into()
        ))));
        assert!(host_ended(&host, "boot-a", "ns-a", || observed(Some(
            "start-b".into()
        ))));
        assert!(host_ended(&host, "boot-a", "ns-a", || {
            ProcessObservation::Missing
        }));
        assert!(host_ended(&host, "boot-b", "ns-b", || panic!(
            "old boot needs no PID observation"
        )));
        assert!(!host_ended(&host, "boot-a", "ns-b", || panic!(
            "foreign namespace must remain unknown"
        )));
        assert!(!host_ended(&host, "boot-a", "ns-a", || observed(None)));
        assert!(!host_ended(&host, "boot-a", "ns-a", || {
            ProcessObservation::Failed {
                reason: "unreadable".into(),
            }
        }));
        assert!(!connection_host_ended("connection-legacy"));
        assert!(!connection_host_ended("connection-v2-invalid"));
        assert!(!connection_host_ended(&new_connection_id()));
    }
}
