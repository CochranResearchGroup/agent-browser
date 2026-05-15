#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${AGENT_BROWSER_WINDOWS_SETUP_SMOKE_PORT:-9222}"
MODE="${AGENT_BROWSER_WINDOWS_SETUP_SMOKE_MODE:-mirrored}"
WINDOWS_USER="${AGENT_BROWSER_WINDOWS_SETUP_SMOKE_USER:-<windows-user>}"
WINDOWS_HOST="${AGENT_BROWSER_WINDOWS_SETUP_SMOKE_HOST:-<windows-host>}"
RULE_NAME="${AGENT_BROWSER_WINDOWS_SETUP_SMOKE_RULE:-agent-browser-cdp-${PORT}}"
RUN_SH="$ROOT/scripts/windows-debug/run.sh"
INSTANCE_FILE="$ROOT/scripts/windows-debug/.instance"

if [[ ! -x "$RUN_SH" ]]; then
  echo "Error: Windows debug harness is not executable: $RUN_SH" >&2
  exit 1
fi

if [[ ! -f "$INSTANCE_FILE" ]]; then
  echo "Error: No Windows debug instance provisioned. Run ./scripts/windows-debug/provision.sh first." >&2
  exit 1
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

script_path="$tmpdir/windows-browser-setup.ps1"
cargo run --quiet --manifest-path "$ROOT/cli/Cargo.toml" -- \
  setup windows-browser \
  --print-powershell \
  --port "$PORT" \
  --mode "$MODE" \
  --windows-user "$WINDOWS_USER" \
  --windows-host "$WINDOWS_HOST" \
  --rule-name "$RULE_NAME" \
  > "$script_path"

script_b64="$(python3 - "$script_path" <<'PY'
import base64
import pathlib
import sys

print(base64.b64encode(pathlib.Path(sys.argv[1]).read_bytes()).decode("ascii"))
PY
)"

remote_command='$ScriptBytes = [Convert]::FromBase64String("'$script_b64'");
$ScriptText = [Text.Encoding]::UTF8.GetString($ScriptBytes);
$ScriptPath = Join-Path $env:TEMP "agent-browser-windows-browser-setup-dry-run.ps1";
Set-Content -Path $ScriptPath -Value $ScriptText -Encoding UTF8;
$Output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath 2>&1 | Out-String;
Write-Output $Output;
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
if ($Output -notmatch "Dry run only") { throw "Dry-run marker missing from generated setup script output" }
if ($Output -match "Created scoped Hyper-V firewall rule") { throw "Generated setup script created a firewall rule during dry-run smoke" }
if ($Output -notmatch "Rollback commands") { throw "Rollback commands missing from generated setup script output" }
Remove-Item -Path $ScriptPath -Force -ErrorAction SilentlyContinue;
Write-Output "agent-browser Windows browser setup PowerShell dry-run smoke passed"'

"$RUN_SH" "$remote_command"
