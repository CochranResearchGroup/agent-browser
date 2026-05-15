use crate::color;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const DEFAULT_SCAN_PORTS: &[u16] = &[
    22, 80, 443, 631, 2222, 3017, 3308, 3389, 4173, 4318, 4369, 4777, 4778, 6389, 8081, 8444, 8765,
    8787, 8791, 8817, 8818, 9090, 9222, 9223, 9333, 11434, 18095, 18789, 18791, 19080,
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorArgs {
    port: u16,
    host: Option<String>,
    scan_ports: bool,
    firewall: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EndpointCandidate {
    host: String,
    port: u16,
    source: String,
}

pub fn run_windows_browser_doctor(clean: &[String], json_mode: bool) {
    let args = parse_doctor_args(clean);
    let report = windows_browser_doctor_report(&args);

    if json_mode {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| {
                r#"{"success":false,"error":"Failed to serialize doctor"}"#.to_string()
            })
        );
        return;
    }

    print_windows_browser_doctor_report(&report);
}

pub(crate) fn windows_browser_doctor_report_for_setup(
    port: u16,
    host: Option<String>,
) -> serde_json::Value {
    windows_browser_doctor_report(&DoctorArgs {
        port,
        host,
        scan_ports: false,
        firewall: false,
    })
}

fn parse_doctor_args(clean: &[String]) -> DoctorArgs {
    let mut port = 9222_u16;
    let mut host = None;
    let mut scan_ports = false;
    let mut firewall = false;
    let mut i = 0;
    while i < clean.len() {
        match clean[i].as_str() {
            "--scan-ports" => scan_ports = true,
            "--firewall" => firewall = true,
            "--port" => {
                if let Some(value) = clean.get(i + 1).and_then(|value| value.parse::<u16>().ok()) {
                    port = value;
                    i += 1;
                }
            }
            "--host" => {
                if let Some(value) = clean.get(i + 1) {
                    if !value.trim().is_empty() {
                        host = Some(value.clone());
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    DoctorArgs {
        port,
        host,
        scan_ports,
        firewall,
    }
}

fn windows_browser_doctor_report(args: &DoctorArgs) -> serde_json::Value {
    let is_wsl = detect_wsl();
    let wslconfig = find_wslconfig()
        .map(|path| {
            let settings = parse_wslconfig(&fs::read_to_string(&path).unwrap_or_default());
            json!({
                "path": path.display().to_string(),
                "exists": true,
                "settings": settings,
            })
        })
        .unwrap_or_else(|| {
            json!({
                "path": null,
                "exists": false,
                "settings": {}
            })
        });
    let networking_mode = wslconfig
        .pointer("/settings/wsl2/networkingMode")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let windows_host_ip = default_route_host_ip();
    let manifest = find_stable_stealthcdp_manifest();
    let executable_path = manifest
        .as_ref()
        .and_then(|manifest| manifest.parent())
        .map(|dir| dir.join("chrome.exe"));
    let executable_on_windows_mount = executable_path
        .as_ref()
        .is_some_and(|path| is_wsl_windows_mount_path(path));
    let path_translation_applies = is_wsl && executable_on_windows_mount;
    let no_sandbox_applies = path_translation_applies;
    let profile_smoke = profile_smoke_recommendation(
        is_wsl,
        manifest.as_ref(),
        executable_path.as_ref(),
        executable_on_windows_mount,
    );
    let candidates = candidate_endpoints(args, &networking_mode, windows_host_ip.as_deref());
    let probes: Vec<_> = candidates.iter().map(probe_endpoint).collect();
    let recommended_route = recommend_route(&networking_mode, &probes);
    let recommended_action = recommend_action(is_wsl, &networking_mode, &probes);
    let port_scan = if args.scan_ports {
        Some(scan_windows_browser_ports(
            args,
            windows_host_ip.as_deref(),
            &networking_mode,
        ))
    } else {
        None
    };
    let firewall_state = if args.firewall {
        Some(query_windows_firewall_state(args.port))
    } else {
        None
    };

    json!({
        "success": true,
        "data": {
            "isWsl": is_wsl,
            "wslconfig": wslconfig,
            "networkingMode": networking_mode,
            "windowsHostIp": windows_host_ip,
            "stealthCdpChromium": {
                "manifestPath": manifest.as_ref().map(|path| path.display().to_string()),
                "manifestVisible": manifest.as_ref().is_some_and(|path| path.is_file()),
                "executablePath": executable_path.as_ref().map(|path| path.display().to_string()),
                "executableOnWindowsMount": executable_on_windows_mount,
                "pathTranslationApplies": path_translation_applies,
                "noSandboxApplies": no_sandbox_applies,
            },
            "profileSmoke": profile_smoke,
            "candidateEndpoints": candidates.iter().map(|candidate| json!({
                "host": candidate.host,
                "port": candidate.port,
                "source": candidate.source,
            })).collect::<Vec<_>>(),
            "probes": probes,
            "portScan": port_scan,
            "recommendedRoute": recommended_route,
            "firewall": firewall_recommendation(&networking_mode, &recommended_action),
            "firewallState": firewall_state,
            "recommendedAction": recommended_action,
        }
    })
}

fn print_windows_browser_doctor_report(report: &serde_json::Value) {
    let data = &report["data"];
    println!(
        "{} agent-browser doctor windows-browser",
        color::success_indicator()
    );
    println!("is WSL: {}", value_or_unknown(&data["isWsl"]));
    println!(
        "networking mode: {}",
        value_or_unknown(&data["networkingMode"])
    );
    println!(
        "windows host IP: {}",
        value_or_unknown(&data["windowsHostIp"])
    );
    println!(
        "stealth manifest: {}",
        value_or_unknown(&data["stealthCdpChromium"]["manifestPath"])
    );
    println!(
        "path translation: {}",
        value_or_unknown(&data["stealthCdpChromium"]["pathTranslationApplies"])
    );
    println!(
        "no-sandbox for WSL Windows executable: {}",
        value_or_unknown(&data["stealthCdpChromium"]["noSandboxApplies"])
    );
    println!(
        "profile smoke available: {}",
        value_or_unknown(&data["profileSmoke"]["available"])
    );
    println!(
        "profile smoke command: {}",
        value_or_unknown(&data["profileSmoke"]["command"])
    );
    println!(
        "profile smoke reason: {}",
        value_or_unknown(&data["profileSmoke"]["reason"])
    );
    println!("candidate endpoints:");
    if let Some(probes) = data["probes"].as_array() {
        for probe in probes {
            let reachable = probe
                .get("reachable")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let marker = if reachable {
                color::success_indicator()
            } else {
                color::warning_indicator()
            };
            println!(
                "  {marker} {}:{} source={} reachable={} http={}",
                value_or_unknown(&probe["host"]),
                value_or_unknown(&probe["port"]),
                value_or_unknown(&probe["source"]),
                value_or_unknown(&probe["reachable"]),
                value_or_unknown(&probe["httpJsonVersion"])
            );
        }
    }
    if let Some(scan) = data["portScan"].as_object() {
        println!("port scan:");
        if let Some(open_ports) = scan.get("openPorts").and_then(|value| value.as_array()) {
            for port in open_ports {
                println!(
                    "  {} {}:{} source={} cdp={}",
                    color::success_indicator(),
                    value_or_unknown(&port["host"]),
                    value_or_unknown(&port["port"]),
                    value_or_unknown(&port["source"]),
                    value_or_unknown(&port["httpJsonVersion"])
                );
            }
        }
    }
    if !data["firewallState"].is_null() {
        println!(
            "firewall state: {}",
            value_or_unknown(&data["firewallState"]["status"])
        );
        if let Some(summary) = data["firewallState"].get("summary") {
            println!("firewall summary: {}", value_or_unknown(summary));
        }
    }
    println!(
        "recommended route: {}",
        value_or_unknown(&data["recommendedRoute"])
    );
    println!(
        "recommended action: {}",
        value_or_unknown(&data["recommendedAction"])
    );
}

fn profile_smoke_recommendation(
    is_wsl: bool,
    manifest: Option<&PathBuf>,
    executable_path: Option<&PathBuf>,
    executable_on_windows_mount: bool,
) -> serde_json::Value {
    let command = "pnpm test:wsl-windows-chromium-profile-live";
    let available = is_wsl
        && manifest.is_some_and(|path| path.is_file())
        && executable_path.is_some_and(|path| path.is_file())
        && executable_on_windows_mount;
    let reason = if available {
        "ready_to_validate_wsl_windows_profile_launch"
    } else if !is_wsl {
        "not_wsl"
    } else if manifest.is_none_or(|path| !path.is_file()) {
        "stealthcdp_manifest_not_visible"
    } else if executable_path.is_none_or(|path| !path.is_file()) {
        "stealthcdp_executable_not_visible"
    } else if !executable_on_windows_mount {
        "stealthcdp_executable_not_on_windows_mount"
    } else {
        "unavailable"
    };
    json!({
        "available": available,
        "command": command,
        "reason": reason,
        "description": "Launches Windows chromium-stealthcdp from WSL with an isolated daemon socket and Windows-mounted profile, then verifies profile writes and Chrome stderr path hygiene.",
    })
}

fn value_or_unknown(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Null => "unknown".to_string(),
        _ => value.to_string(),
    }
}

fn detect_wsl() -> bool {
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .or_else(|_| fs::read_to_string("/proc/version"))
        .map(|text| text.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
        || std::env::var_os("WSL_INTEROP").is_some()
}

fn find_wslconfig() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        candidates.push(PathBuf::from(userprofile).join(".wslconfig"));
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(PathBuf::from(home).join(".wslconfig"));
    }
    if let Ok(entries) = fs::read_dir("/mnt/c/Users") {
        for entry in entries.filter_map(Result::ok) {
            candidates.push(entry.path().join(".wslconfig"));
        }
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn parse_wslconfig(text: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut sections: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut current = String::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        sections
            .entry(current.clone())
            .or_default()
            .insert(key.trim().to_string(), value.trim().to_string());
    }
    sections
}

fn default_route_host_ip() -> Option<String> {
    let route = fs::read_to_string("/proc/net/route").ok()?;
    for line in route.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() > 2 && fields[1] == "00000000" {
            let raw = u32::from_str_radix(fields[2], 16).ok()?;
            let bytes = raw.to_le_bytes();
            return Some(format!(
                "{}.{}.{}.{}",
                bytes[0], bytes[1], bytes[2], bytes[3]
            ));
        }
    }
    None
}

fn find_stable_stealthcdp_manifest() -> Option<PathBuf> {
    if let Ok(root) = std::env::var("AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT") {
        let path = PathBuf::from(root).join("current").join("manifest.json");
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let path = PathBuf::from(local_app_data)
            .join("chromium-stealthcdp")
            .join("current")
            .join("manifest.json");
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(entries) = fs::read_dir("/mnt/c/Users") {
        let mut candidates = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            let manifest = entry
                .path()
                .join("AppData")
                .join("Local")
                .join("chromium-stealthcdp")
                .join("current")
                .join("manifest.json");
            if manifest.is_file() {
                candidates.push(manifest);
            }
        }
        candidates.sort();
        return candidates.into_iter().next();
    }
    None
}

fn is_wsl_windows_mount_path(path: &Path) -> bool {
    let mut components = path.components();
    if !matches!(components.next(), Some(std::path::Component::RootDir)) {
        return false;
    }
    match components.next() {
        Some(std::path::Component::Normal(value)) if value == "mnt" => {}
        _ => return false,
    }
    match components.next() {
        Some(std::path::Component::Normal(value)) => {
            let drive = value.to_string_lossy();
            drive.len() == 1 && drive.as_bytes()[0].is_ascii_alphabetic()
        }
        _ => false,
    }
}

fn candidate_endpoints(
    args: &DoctorArgs,
    networking_mode: &str,
    windows_host_ip: Option<&str>,
) -> Vec<EndpointCandidate> {
    let mut candidates = Vec::new();
    if let Some(host) = &args.host {
        candidates.push(EndpointCandidate {
            host: host.clone(),
            port: args.port,
            source: "explicit_host".to_string(),
        });
        return candidates;
    }

    candidates.push(EndpointCandidate {
        host: "127.0.0.1".to_string(),
        port: args.port,
        source: if networking_mode.eq_ignore_ascii_case("mirrored") {
            "mirrored_or_ssh"
        } else {
            "localhost_or_ssh"
        }
        .to_string(),
    });
    if let Some(host) = windows_host_ip {
        if host != "127.0.0.1" {
            candidates.push(EndpointCandidate {
                host: host.to_string(),
                port: args.port,
                source: "nat_default_route".to_string(),
            });
        }
    }
    candidates
}

fn probe_endpoint(candidate: &EndpointCandidate) -> serde_json::Value {
    let reachable = tcp_reachable(&candidate.host, candidate.port);
    let http_json_version = if reachable {
        probe_json_version(&candidate.host, candidate.port).is_some()
    } else {
        false
    };
    json!({
        "host": candidate.host,
        "port": candidate.port,
        "source": candidate.source,
        "reachable": reachable,
        "httpJsonVersion": http_json_version,
    })
}

fn tcp_reachable(host: &str, port: u16) -> bool {
    let Ok(mut addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    addrs.any(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok())
}

fn probe_json_version(host: &str, port: u16) -> Option<String> {
    let mut addrs = (host, port).to_socket_addrs().ok()?;
    let addr = addrs.next()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(700)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(700)));
    let request =
        format!("GET /json/version HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut buf = [0_u8; 4096];
    let n = stream.read(&mut buf).ok()?;
    let response = String::from_utf8_lossy(&buf[..n]).to_string();
    let is_ok = response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200");
    let is_cdp_version = response.contains("\"Browser\"")
        && (response.contains("\"Protocol-Version\"")
            || response.contains("\"webSocketDebuggerUrl\""));
    if is_ok && is_cdp_version {
        Some(response)
    } else {
        None
    }
}

fn scan_windows_browser_ports(
    args: &DoctorArgs,
    windows_host_ip: Option<&str>,
    networking_mode: &str,
) -> serde_json::Value {
    let mut localhost_ports = DEFAULT_SCAN_PORTS.to_vec();
    localhost_ports.push(args.port);
    localhost_ports.extend(proc_net_tcp_listeners_for_localhost());
    localhost_ports.sort_unstable();
    localhost_ports.dedup();

    let mut targets = vec![(
        "127.0.0.1".to_string(),
        "localhost_or_mirrored",
        localhost_ports,
    )];
    if let Some(host) = windows_host_ip {
        if host != "127.0.0.1" {
            let mut host_ports = DEFAULT_SCAN_PORTS.to_vec();
            host_ports.push(args.port);
            host_ports.sort_unstable();
            host_ports.dedup();
            targets.push((host.to_string(), "nat_default_route", host_ports));
        }
    }
    if let Some(host) = &args.host {
        if host != "127.0.0.1" && windows_host_ip != Some(host.as_str()) {
            targets.push((host.clone(), "explicit_host", vec![args.port]));
        }
    }

    let mut open_ports = Vec::new();
    let mut scanned = 0_usize;
    for (host, source, ports) in targets {
        for port in ports {
            scanned += 1;
            if !tcp_reachable_with_timeout(&host, port, Duration::from_millis(250)) {
                continue;
            }
            let http_json_version = probe_json_version(&host, port).is_some();
            open_ports.push(json!({
                "host": host,
                "port": port,
                "source": source,
                "httpJsonVersion": http_json_version,
            }));
        }
    }

    json!({
        "enabled": true,
        "mutated": false,
        "networkingMode": networking_mode,
        "scannedCount": scanned,
        "openPorts": open_ports,
    })
}

fn proc_net_tcp_listeners_for_localhost() -> Vec<u16> {
    let mut ports = Vec::new();
    for path in ["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines().skip(1) {
                if let Some(port) = parse_proc_net_tcp_listener_port(line) {
                    ports.push(port);
                }
            }
        }
    }
    ports.sort_unstable();
    ports.dedup();
    ports
}

fn parse_proc_net_tcp_listener_port(line: &str) -> Option<u16> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let local = *fields.get(1)?;
    let state = *fields.get(3)?;
    if state != "0A" {
        return None;
    }
    let (addr, port_hex) = local.split_once(':')?;
    let is_localhost = matches!(
        addr,
        "0100007F"
            | "00000000"
            | "00000000000000000000000001000000"
            | "00000000000000000000000000000000"
    );
    if !is_localhost {
        return None;
    }
    u16::from_str_radix(port_hex, 16).ok()
}

fn tcp_reachable_with_timeout(host: &str, port: u16, timeout: Duration) -> bool {
    let Ok(mut addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    addrs.any(|addr| TcpStream::connect_timeout(&addr, timeout).is_ok())
}

fn query_windows_firewall_state(port: u16) -> serde_json::Value {
    let Some(powershell) = find_windows_powershell() else {
        return json!({
            "status": "unavailable",
            "mutated": false,
            "error": "Windows PowerShell executable was not found from WSL",
        });
    };
    let script = firewall_query_script(port);
    let output = Command::new(&powershell)
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .output();
    let Ok(output) = output else {
        return json!({
            "status": "error",
            "mutated": false,
            "powershellPath": powershell.display().to_string(),
            "error": "failed to execute Windows PowerShell",
        });
    };
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return json!({
            "status": "error",
            "mutated": false,
            "powershellPath": powershell.display().to_string(),
            "exitCode": output.status.code(),
            "stdout": stdout,
            "stderr": stderr,
        });
    }
    let parsed = serde_json::from_str::<serde_json::Value>(&stdout).ok();
    json!({
        "status": if parsed.is_some() { "ok" } else { "parse_error" },
        "mutated": false,
        "powershellPath": powershell.display().to_string(),
        "summary": parsed.unwrap_or_else(|| json!({
            "raw": stdout,
            "stderr": stderr,
        })),
    })
}

fn find_windows_powershell() -> Option<PathBuf> {
    [
        PathBuf::from("/mnt/c/Program Files/PowerShell/7/pwsh.exe"),
        PathBuf::from("/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe"),
        PathBuf::from("powershell.exe"),
        PathBuf::from("pwsh.exe"),
    ]
    .into_iter()
    .find(|path| path.is_file() || path.components().count() == 1)
}

fn firewall_query_script(port: u16) -> String {
    format!(
        r#"
$ErrorActionPreference = 'SilentlyContinue'
$VmCreatorId = '{{40E0AC32-46A5-438A-A0B2-2B479E8F2E90}}'
$Port = {port}
$HyperVAvailable = [bool](Get-Command Get-NetFirewallHyperVRule -ErrorAction SilentlyContinue)
$HyperVRules = @()
if ($HyperVAvailable) {{
  $HyperVRules = Get-NetFirewallHyperVRule |
    Where-Object {{ $_.VMCreatorId -eq $VmCreatorId -or $_.Name -like '*agent-browser*' -or $_.DisplayName -like '*agent-browser*' -or $_.LocalPorts -contains $Port }} |
    Select-Object Name,DisplayName,Direction,Action,Enabled,LocalPorts,VMCreatorId
}}
$FirewallRules = Get-NetFirewallRule -ErrorAction SilentlyContinue |
  Where-Object {{ $_.DisplayName -like '*agent-browser*' -or $_.Name -like '*agent-browser*' }} |
  Select-Object Name,DisplayName,Enabled,Direction,Action,Profile
[pscustomobject]@{{
  hyperVAvailable = $HyperVAvailable
  vmCreatorId = $VmCreatorId
  port = $Port
  hyperVRuleCount = @($HyperVRules).Count
  firewallRuleCount = @($FirewallRules).Count
  hyperVRules = @($HyperVRules)
  firewallRules = @($FirewallRules)
}} | ConvertTo-Json -Depth 5 -Compress
"#
    )
}

fn recommend_route(networking_mode: &str, probes: &[serde_json::Value]) -> String {
    if let Some(probe) = probes
        .iter()
        .find(|probe| probe["httpJsonVersion"].as_bool() == Some(true))
    {
        return format!(
            "{}:{}",
            probe["host"].as_str().unwrap_or("unknown"),
            probe["port"].as_u64().unwrap_or_default()
        );
    }
    if networking_mode.eq_ignore_ascii_case("mirrored") {
        "127.0.0.1 or ssh-forwarded localhost".to_string()
    } else {
        "default-route-host-ip or ssh-forwarded localhost".to_string()
    }
}

fn recommend_action(is_wsl: bool, networking_mode: &str, probes: &[serde_json::Value]) -> String {
    if !is_wsl {
        return "not_wsl_no_windows_browser_network_setup_needed".to_string();
    }
    if probes
        .iter()
        .any(|probe| probe["httpJsonVersion"].as_bool() == Some(true))
    {
        return "cdp_endpoint_ready".to_string();
    }
    if networking_mode.eq_ignore_ascii_case("mirrored") {
        "launch_agent_browser_owned_windows_browser_or_open_scoped_firewall_for_fixed_port"
            .to_string()
    } else {
        "use_default_route_host_ip_or_configure_mirrored_networking_or_ssh_tunnel".to_string()
    }
}

fn firewall_recommendation(networking_mode: &str, recommended_action: &str) -> serde_json::Value {
    let action = if recommended_action == "cdp_endpoint_ready" {
        "none_probe_passed"
    } else if networking_mode.eq_ignore_ascii_case("mirrored") {
        "scoped_hyper_v_rule_for_operator_owned_fixed_ports"
    } else {
        "prefer_ssh_tunnel_or_scoped_windows_firewall_rule_for_fixed_ports"
    };
    json!({
        "recommendedAction": action,
        "mutated": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    use std::net::TcpListener;

    #[test]
    fn parses_wslconfig_sections() {
        let parsed = parse_wslconfig(
            r#"
[wsl2]
networkingMode=mirrored
firewall=true

[experimental]
autoMemoryReclaim=disabled
"#,
        );

        assert_eq!(parsed["wsl2"]["networkingMode"], "mirrored");
        assert_eq!(parsed["wsl2"]["firewall"], "true");
        assert_eq!(parsed["experimental"]["autoMemoryReclaim"], "disabled");
    }

    #[test]
    fn parses_doctor_port_and_host() {
        let args = parse_doctor_args(&[
            "doctor".to_string(),
            "windows-browser".to_string(),
            "--host".to_string(),
            "192.168.1.2".to_string(),
            "--port".to_string(),
            "9333".to_string(),
        ]);

        assert_eq!(
            args,
            DoctorArgs {
                port: 9333,
                host: Some("192.168.1.2".to_string()),
                scan_ports: false,
                firewall: false,
            }
        );
    }

    #[test]
    fn parses_scan_ports_and_firewall_flags() {
        let args = parse_doctor_args(&[
            "doctor".to_string(),
            "windows-browser".to_string(),
            "--scan-ports".to_string(),
            "--firewall".to_string(),
        ]);

        assert_eq!(
            args,
            DoctorArgs {
                port: 9222,
                host: None,
                scan_ports: true,
                firewall: true,
            }
        );
    }

    #[test]
    fn candidate_endpoints_prefers_explicit_host() {
        let args = DoctorArgs {
            port: 9333,
            host: Some("10.0.0.2".to_string()),
            scan_ports: false,
            firewall: false,
        };

        assert_eq!(
            candidate_endpoints(&args, "mirrored", Some("192.168.50.1")),
            vec![EndpointCandidate {
                host: "10.0.0.2".to_string(),
                port: 9333,
                source: "explicit_host".to_string(),
            }]
        );
    }

    #[test]
    fn candidate_endpoints_includes_localhost_and_nat_host() {
        let args = DoctorArgs {
            port: 9222,
            host: None,
            scan_ports: false,
            firewall: false,
        };

        assert_eq!(
            candidate_endpoints(&args, "nat", Some("192.168.50.1")),
            vec![
                EndpointCandidate {
                    host: "127.0.0.1".to_string(),
                    port: 9222,
                    source: "localhost_or_ssh".to_string(),
                },
                EndpointCandidate {
                    host: "192.168.50.1".to_string(),
                    port: 9222,
                    source: "nat_default_route".to_string(),
                }
            ]
        );
    }

    #[test]
    fn json_version_probe_accepts_response_without_waiting_for_eof() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 512];
            let _ = stream.read(&mut request);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 39\r\n\r\n{\"Browser\":\"x\",\"Protocol-Version\":\"1.3\"}",
                )
                .unwrap();
            std::thread::sleep(Duration::from_millis(200));
        });

        assert!(probe_json_version("127.0.0.1", port).is_some());
        handle.join().unwrap();
    }

    #[test]
    fn json_version_probe_rejects_plain_http_200() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 512];
            let _ = stream.read(&mut request);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 12\r\n\r\n<html></html>",
                )
                .unwrap();
        });

        assert!(probe_json_version("127.0.0.1", port).is_none());
        handle.join().unwrap();
    }

    #[test]
    fn parses_proc_net_tcp_localhost_listener_port() {
        let line = "   1: 0100007F:2406 00000000:0000 0A 00000000:00000000 00:00000000 00000000";
        assert_eq!(parse_proc_net_tcp_listener_port(line), Some(9222));
    }

    #[test]
    fn ignores_non_listener_proc_net_tcp_port() {
        let line = "   1: 0100007F:2406 00000000:0000 01 00000000:00000000 00:00000000 00000000";
        assert_eq!(parse_proc_net_tcp_listener_port(line), None);
    }

    #[test]
    fn firewall_query_script_contains_port_and_vm_creator_id() {
        let script = firewall_query_script(9333);
        assert!(script.contains("$Port = 9333"));
        assert!(script.contains("{40E0AC32-46A5-438A-A0B2-2B479E8F2E90}"));
        assert!(script.contains("ConvertTo-Json"));
    }

    #[test]
    fn detects_wsl_windows_mount_path() {
        assert!(is_wsl_windows_mount_path(Path::new(
            "/mnt/c/Users/ecoch/AppData/Local/chrome.exe"
        )));
        assert!(!is_wsl_windows_mount_path(Path::new("/tmp/chrome.exe")));
    }

    #[test]
    fn profile_smoke_recommendation_requires_visible_windows_mounted_artifact() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-windows-profile-smoke-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&dir).unwrap();
        let manifest = dir.join("manifest.json");
        let executable = dir.join("chrome.exe");
        fs::write(&manifest, "{}").unwrap();
        fs::write(&executable, "").unwrap();

        let available =
            profile_smoke_recommendation(true, Some(&manifest), Some(&executable), true);
        assert_eq!(available["available"], true);
        assert_eq!(
            available["command"],
            "pnpm test:wsl-windows-chromium-profile-live"
        );

        let missing = profile_smoke_recommendation(true, Some(&manifest), Some(&executable), false);
        assert_eq!(missing["available"], false);
        assert_eq!(
            missing["reason"],
            "stealthcdp_executable_not_on_windows_mount"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stable_manifest_honors_install_root_override() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-windows-doctor-manifest-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        let current = dir.join("current");
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join("manifest.json"), "{}").unwrap();
        let guard = EnvGuard::new(&["AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT"]);
        guard.set(
            "AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT",
            dir.to_str().unwrap(),
        );

        assert_eq!(
            find_stable_stealthcdp_manifest(),
            Some(current.join("manifest.json"))
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
