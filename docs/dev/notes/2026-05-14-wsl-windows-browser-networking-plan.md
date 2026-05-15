# WSL Windows Browser Networking Plan

Date: 2026-05-14

## Why This Matters

agent-browser can run from WSL while managing Windows-native browsers. That is a
valuable posture for users who prefer Windows Chrome, Edge, or a Windows
`chromium-stealthcdp` build, but the CDP control path depends on WSL networking
mode, Windows firewall state, profile path placement, and whether the browser is
started by agent-browser or brought by an operator.

The service should eventually own this diagnosis instead of making operators
guess whether `127.0.0.1`, the WSL default-route host IP, a fixed forwarded port,
or an SSH tunnel is the correct route.

## Current Local Finding

This workstation is WSL 2 with mirrored networking enabled:

```text
/mnt/c/Users/ecoch/.wslconfig
[wsl2]
networkingMode=mirrored
dnsTunneling=true
firewall=true
autoProxy=true
localhostForwarding=true
```

The successful live smoke launched:

```text
/mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe
```

agent-browser read `DevToolsActivePort` from a Windows-mounted profile path and
connected to:

```text
ws://127.0.0.1:<ephemeral-port>/<devtools-browser-path>
```

That means the observed local route was:

```text
WSL agent-browser -> 127.0.0.1:<ephemeral CDP port> -> Windows Chrome DevTools listener
```

## WSL Modes To Support

### Mirrored Networking

Microsoft documents mirrored networking as the preferred mode on Windows 11
22H2 and later when WSL and Windows need to connect to each other through
`127.0.0.1`. In this mode agent-browser should try `127.0.0.1` first for
Windows-hosted CDP ports.

Recommended `.wslconfig` shape:

```ini
[wsl2]
networkingMode=mirrored
dnsTunneling=true
firewall=true
autoProxy=true
localhostForwarding=true
```

For inbound WSL traffic and some Hyper-V filtered traffic, Windows may also need
Hyper-V firewall rules. Microsoft documents both a broad setting and scoped
rules using the WSL VM creator ID:

```powershell
Set-NetFirewallHyperVVMSetting -Name '{40E0AC32-46A5-438A-A0B2-2B479E8F2E90}' -DefaultInboundAction Allow
New-NetFirewallHyperVRule -Name 'agent-browser-cdp' -DisplayName 'agent-browser CDP' -Direction Inbound -VMCreatorId '{40E0AC32-46A5-438A-A0B2-2B479E8F2E90}' -Protocol TCP -LocalPorts 9222
```

agent-browser should prefer scoped rules over broad allow in generated scripts.

### NAT Networking

In NAT mode, Microsoft documents that Linux programs generally reach Windows
host services through the Windows host IP shown by the WSL default route:

```bash
ip route show | grep -i default | awk '{ print $3 }'
```

For Windows browsers started outside agent-browser, a fixed CDP port must be
reachable from WSL at that host IP. The browser must bind to a reachable
address, not just a Windows-only loopback address when NAT prevents loopback
sharing. If the browser cannot or should not expose a LAN-reachable listener,
use SSH port forwarding instead.

### SSH Port Forwarding

SSH forwarding is the safest explicit fallback when direct WSL-to-Windows CDP
probing is blocked or policy forbids opening Windows firewall ports. The
operator can bind a local WSL port and forward it to the Windows CDP listener:

```bash
ssh -N -L 127.0.0.1:9222:127.0.0.1:9222 <windows-user>@<windows-host>
```

agent-browser should treat this as an operator-managed route with a doctor probe
that validates `http://127.0.0.1:9222/json/version` and the resulting WebSocket
URL.

## Proposed Doctor Surface

Add:

```bash
agent-browser install doctor --windows-browser-network
```

or a focused command:

```bash
agent-browser doctor windows-browser
```

The doctor should be read-only by default and report:

- WSL detection from `/proc/sys/kernel/osrelease` and `WSL_INTEROP`.
- `.wslconfig` path, parsed `networkingMode`, `firewall`, `dnsTunneling`,
  `autoProxy`, `hostAddressLoopback`, and `ignoredPorts`.
- Windows host IP from the WSL default route for NAT fallback.
- Whether `%LOCALAPPDATA%\chromium-stealthcdp\current\manifest.json` is visible
  through `/mnt/c`.
- Whether the selected Windows browser executable is on a Windows-mounted path.
- Whether agent-browser will translate mounted paths for Chrome arguments.
- Whether agent-browser will add `--no-sandbox` for WSL-launched Windows
  executables.
- Candidate CDP endpoints in priority order:
  - `127.0.0.1:<port>` for mirrored mode and SSH-forwarded mode.
  - `<default-route-host-ip>:<port>` for NAT mode.
  - any explicit `--cdp` or configured browser host endpoint.
- Probe result for `/json/version`, `/json/list`, and direct WebSocket fallback.
- Firewall recommendation:
  - no firewall change needed for agent-browser-owned ephemeral localhost CDP
    under mirrored mode when the probe passes
  - scoped fixed-port rule needed for operator-owned fixed CDP ports
  - prefer SSH tunnel when policy forbids opening ports

Output should include JSON fields so agents and software can consume it:

```json
{
  "isWsl": true,
  "networkingMode": "mirrored",
  "windowsHostIp": "192.168.50.1",
  "recommendedRoute": "localhost",
  "candidateEndpoints": [
    { "host": "127.0.0.1", "port": 9222, "source": "mirrored_or_ssh" },
    { "host": "192.168.50.1", "port": 9222, "source": "nat_default_route" }
  ],
  "firewall": {
    "mode": "hyper_v_firewall_enabled",
    "recommendedAction": "none_for_ephemeral_agent_browser_owned_launch"
  }
}
```

## Proposed Setup Wizard

Add a read-only preview first:

```bash
agent-browser setup windows-browser --print-powershell --port 9222
```

Then an apply mode only after review:

```bash
agent-browser setup windows-browser --apply --port 9222
```

The wizard should produce:

- `.wslconfig` patch guidance for mirrored mode.
- A reminder that `wsl --shutdown` is required after changing `.wslconfig`.
- A scoped Hyper-V firewall rule for the selected fixed CDP port.
- A standard Windows Defender firewall rule only when a non-Hyper-V path needs it.
- An SSH tunnel recipe when the user selects tunnel mode.
- A rollback script for every firewall rule it creates.

## Security Defaults

- Do not expose CDP on `0.0.0.0` by default.
- Prefer ephemeral CDP ports for agent-browser-owned launches.
- Prefer `127.0.0.1` and SSH tunnels over LAN-reachable fixed ports.
- Require explicit operator consent before authoring firewall changes.
- Label fixed CDP ports as sensitive because CDP is effectively browser control.

## Next Slice

Implement the read-only doctor first. It should not edit `.wslconfig`, firewall
state, SSH config, or browser profiles. Once the doctor can explain the current
route and recommend the safest next action, add `--print-powershell` generation
for users who want agent-browser to author the setup script.

## Sources Checked

- Microsoft Learn, Accessing network applications with WSL:
  https://learn.microsoft.com/en-us/windows/wsl/networking
- Microsoft Learn, Advanced settings configuration in WSL:
  https://learn.microsoft.com/en-us/windows/wsl/wsl-config
