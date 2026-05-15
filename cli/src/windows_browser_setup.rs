use crate::color;
use serde_json::json;
use std::process::exit;

const HYPER_V_WSL_VM_CREATOR_ID: &str = "{40E0AC32-46A5-438A-A0B2-2B479E8F2E90}";

#[derive(Debug, Clone, PartialEq, Eq)]
struct SetupArgs {
    print_powershell: bool,
    port: u16,
    mode: SetupMode,
    windows_user: Option<String>,
    windows_host: Option<String>,
    rule_name: String,
    apply_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SetupMode {
    Mirrored,
    Nat,
    Ssh,
}

impl SetupMode {
    fn as_str(&self) -> &'static str {
        match self {
            SetupMode::Mirrored => "mirrored",
            SetupMode::Nat => "nat",
            SetupMode::Ssh => "ssh",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "mirrored" => Some(SetupMode::Mirrored),
            "nat" => Some(SetupMode::Nat),
            "ssh" => Some(SetupMode::Ssh),
            _ => None,
        }
    }
}

pub fn run_windows_browser_setup(clean: &[String], json_mode: bool) {
    match parse_setup_args(clean) {
        Ok(args) => {
            if args.apply_requested {
                exit_with_error(
                    "agent-browser setup windows-browser is preview-only; use --print-powershell and review the generated script before running it on Windows",
                    json_mode,
                );
            }
            if !args.print_powershell {
                exit_with_error(
                    "Usage: agent-browser setup windows-browser --print-powershell [--port <port>] [--mode mirrored|nat|ssh]",
                    json_mode,
                );
            }

            let script = render_powershell_setup(&args);
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "success": true,
                        "data": {
                            "mutated": false,
                            "kind": "windows_browser_setup_powershell",
                            "port": args.port,
                            "mode": args.mode.as_str(),
                            "ruleName": args.rule_name,
                            "vmCreatorId": HYPER_V_WSL_VM_CREATOR_ID,
                            "script": script,
                        }
                    }))
                    .unwrap_or_else(|_| {
                        r#"{"success":false,"error":"Failed to serialize setup script"}"#
                            .to_string()
                    })
                );
            } else {
                print!("{script}");
            }
        }
        Err(error) => exit_with_error(error, json_mode),
    }
}

fn exit_with_error(message: impl AsRef<str>, json_mode: bool) -> ! {
    if json_mode {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "success": false,
                "error": message.as_ref(),
            }))
            .unwrap_or_else(|_| r#"{"success":false,"error":"setup failed"}"#.to_string())
        );
    } else {
        eprintln!("{} {}", color::error_indicator(), message.as_ref());
    }
    exit(1);
}

fn parse_setup_args(clean: &[String]) -> Result<SetupArgs, String> {
    let mut print_powershell = false;
    let mut port = 9222_u16;
    let mut mode = SetupMode::Mirrored;
    let mut windows_user = None;
    let mut windows_host = None;
    let mut rule_name = None;
    let mut apply_requested = false;
    let mut i = 0;

    while i < clean.len() {
        match clean[i].as_str() {
            "--print-powershell" => print_powershell = true,
            "--apply" => apply_requested = true,
            "--port" => {
                let value = clean
                    .get(i + 1)
                    .ok_or_else(|| "--port requires a value".to_string())?;
                port = value
                    .parse::<u16>()
                    .map_err(|_| format!("Invalid --port value: {value}"))?;
                i += 1;
            }
            "--mode" => {
                let value = clean
                    .get(i + 1)
                    .ok_or_else(|| "--mode requires a value".to_string())?;
                mode = SetupMode::parse(value).ok_or_else(|| {
                    format!("Invalid --mode value: {value}; expected mirrored, nat, or ssh")
                })?;
                i += 1;
            }
            "--windows-user" => {
                let value = clean
                    .get(i + 1)
                    .ok_or_else(|| "--windows-user requires a value".to_string())?;
                if !value.trim().is_empty() {
                    windows_user = Some(value.clone());
                }
                i += 1;
            }
            "--windows-host" => {
                let value = clean
                    .get(i + 1)
                    .ok_or_else(|| "--windows-host requires a value".to_string())?;
                if !value.trim().is_empty() {
                    windows_host = Some(value.clone());
                }
                i += 1;
            }
            "--rule-name" => {
                let value = clean
                    .get(i + 1)
                    .ok_or_else(|| "--rule-name requires a value".to_string())?;
                if !value.trim().is_empty() {
                    rule_name = Some(value.clone());
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    Ok(SetupArgs {
        print_powershell,
        port,
        mode,
        windows_user,
        windows_host,
        rule_name: rule_name.unwrap_or_else(|| format!("agent-browser-cdp-{port}")),
        apply_requested,
    })
}

fn render_powershell_setup(args: &SetupArgs) -> String {
    let rule_name = powershell_single_quote(&args.rule_name);
    let windows_user =
        powershell_single_quote(args.windows_user.as_deref().unwrap_or("<windows-user>"));
    let windows_host =
        powershell_single_quote(args.windows_host.as_deref().unwrap_or("<windows-host>"));
    let recommended_route = match args.mode {
        SetupMode::Mirrored => "Use 127.0.0.1 from WSL when mirrored networking is active.",
        SetupMode::Nat => "Use the Windows host IP from the WSL default route when mirrored networking is unavailable.",
        SetupMode::Ssh => "Use an SSH local tunnel and connect agent-browser to 127.0.0.1.",
    };
    let launch_hint = format!(
        "Start the Windows browser with --remote-debugging-address=127.0.0.1 --remote-debugging-port={}",
        args.port
    );

    format!(
        r#"# agent-browser Windows browser setup preview
# Generated by: agent-browser setup windows-browser --print-powershell
# This script is dry-run by default. Re-run with -Apply after reviewing it.

param(
    [switch]$Apply
)

$ErrorActionPreference = 'Stop'
$Port = {port}
$RuleName = '{rule_name}'
$DisplayName = "agent-browser CDP $Port"
$VmCreatorId = '{vm_creator_id}'
$WslConfigPath = Join-Path $env:USERPROFILE '.wslconfig'
$RecommendedWslConfig = @'
[wsl2]
networkingMode=mirrored
dnsTunneling=true
firewall=true
autoProxy=true
localhostForwarding=true
'@

Write-Host 'agent-browser Windows browser setup'
Write-Host "Mode: {mode}"
Write-Host "CDP port: $Port"
Write-Host '{recommended_route}'
Write-Host '{launch_hint}'
Write-Host ''

if (-not (Test-Path $WslConfigPath)) {{
    Write-Warning ".wslconfig was not found at $WslConfigPath"
    Write-Host 'Recommended .wslconfig content:'
    Write-Host $RecommendedWslConfig
}} else {{
    Write-Host "Existing .wslconfig: $WslConfigPath"
    $ExistingWslConfig = Get-Content -Raw -Path $WslConfigPath
    if ($ExistingWslConfig -notmatch '(?im)^\s*networkingMode\s*=\s*mirrored\s*$') {{
        Write-Warning 'Mirrored networking is not present. Review the recommended settings before editing .wslconfig.'
        Write-Host $RecommendedWslConfig
    }} else {{
        Write-Host 'Mirrored networking appears to be configured.'
    }}
}}

if ($Apply) {{
    if (Get-Command New-NetFirewallHyperVRule -ErrorAction SilentlyContinue) {{
        $ExistingRule = Get-NetFirewallHyperVRule -Name $RuleName -ErrorAction SilentlyContinue
        if ($null -eq $ExistingRule) {{
            New-NetFirewallHyperVRule `
                -Name $RuleName `
                -DisplayName $DisplayName `
                -Direction Inbound `
                -VMCreatorId $VmCreatorId `
                -Protocol TCP `
                -LocalPorts $Port `
                -Action Allow
            Write-Host "Created scoped Hyper-V firewall rule: $RuleName"
        }} else {{
            Write-Host "Scoped Hyper-V firewall rule already exists: $RuleName"
        }}
    }} else {{
        Write-Warning 'New-NetFirewallHyperVRule is unavailable on this Windows build.'
        Write-Warning 'Fallback, if policy permits: New-NetFirewallRule -DisplayName $DisplayName -Direction Inbound -Action Allow -Protocol TCP -LocalPort $Port -Profile Private'
    }}
}} else {{
    Write-Host 'Dry run only. No firewall, WSL, SSH, or browser state was changed.'
    Write-Host "To apply only the scoped Hyper-V firewall rule, save this script and run it with -Apply from an elevated PowerShell session."
}}

Write-Host ''
Write-Host 'SSH tunnel fallback from WSL, when Windows OpenSSH server is enabled:'
Write-Host 'ssh -N -L 127.0.0.1:{port}:127.0.0.1:{port} {windows_user}@{windows_host}'
Write-Host ''
Write-Host 'After .wslconfig changes, run from Windows PowerShell: wsl --shutdown'
Write-Host 'Then restart WSL and run: agent-browser doctor windows-browser --port {port}'
Write-Host ''
Write-Host 'Rollback commands:'
Write-Host "Remove-NetFirewallHyperVRule -Name '$RuleName' -ErrorAction SilentlyContinue"
Write-Host "Remove-NetFirewallRule -DisplayName '$DisplayName' -ErrorAction SilentlyContinue"
"#,
        port = args.port,
        rule_name = rule_name,
        vm_creator_id = HYPER_V_WSL_VM_CREATOR_ID,
        mode = args.mode.as_str(),
        recommended_route = powershell_single_quote(recommended_route),
        launch_hint = powershell_single_quote(&launch_hint),
        windows_user = windows_user,
        windows_host = windows_host,
    )
}

fn powershell_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_setup_defaults() {
        let args = parse_setup_args(&[
            "setup".to_string(),
            "windows-browser".to_string(),
            "--print-powershell".to_string(),
        ])
        .unwrap();

        assert_eq!(
            args,
            SetupArgs {
                print_powershell: true,
                port: 9222,
                mode: SetupMode::Mirrored,
                windows_user: None,
                windows_host: None,
                rule_name: "agent-browser-cdp-9222".to_string(),
                apply_requested: false,
            }
        );
    }

    #[test]
    fn parses_setup_overrides() {
        let args = parse_setup_args(&[
            "setup".to_string(),
            "windows-browser".to_string(),
            "--print-powershell".to_string(),
            "--port".to_string(),
            "9333".to_string(),
            "--mode".to_string(),
            "ssh".to_string(),
            "--windows-user".to_string(),
            "ecoch".to_string(),
            "--windows-host".to_string(),
            "winhost".to_string(),
            "--rule-name".to_string(),
            "custom-rule".to_string(),
        ])
        .unwrap();

        assert_eq!(args.port, 9333);
        assert_eq!(args.mode, SetupMode::Ssh);
        assert_eq!(args.windows_user, Some("ecoch".to_string()));
        assert_eq!(args.windows_host, Some("winhost".to_string()));
        assert_eq!(args.rule_name, "custom-rule");
    }

    #[test]
    fn rejects_invalid_mode() {
        let error = parse_setup_args(&[
            "setup".to_string(),
            "windows-browser".to_string(),
            "--mode".to_string(),
            "bridge".to_string(),
        ])
        .unwrap_err();

        assert!(error.contains("Invalid --mode value"));
    }

    #[test]
    fn powershell_contains_firewall_and_rollback() {
        let args = SetupArgs {
            print_powershell: true,
            port: 9333,
            mode: SetupMode::Ssh,
            windows_user: Some("ecoch".to_string()),
            windows_host: Some("winhost".to_string()),
            rule_name: "agent-browser-cdp-9333".to_string(),
            apply_requested: false,
        };
        let script = render_powershell_setup(&args);

        assert!(script.contains("New-NetFirewallHyperVRule"));
        assert!(script.contains(HYPER_V_WSL_VM_CREATOR_ID));
        assert!(script.contains("ssh -N -L 127.0.0.1:9333:127.0.0.1:9333 ecoch@winhost"));
        assert!(script.contains("Remove-NetFirewallHyperVRule"));
        assert!(script.contains("agent-browser doctor windows-browser --port 9333"));
    }

    #[test]
    fn powershell_escapes_single_quotes() {
        assert_eq!(powershell_single_quote("agent's rule"), "agent''s rule");
    }
}
