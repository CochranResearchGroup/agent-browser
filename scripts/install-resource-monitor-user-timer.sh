#!/usr/bin/env bash
set -euo pipefail

interval="${AGENT_BROWSER_RESOURCE_MONITOR_INTERVAL:-5min}"
bin="${AGENT_BROWSER_BIN:-$(command -v agent-browser || true)}"

if [[ -z "$bin" ]]; then
  echo "agent-browser binary not found; set AGENT_BROWSER_BIN=/path/to/agent-browser" >&2
  exit 1
fi

unit_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
mkdir -p "$unit_dir"

cat >"$unit_dir/agent-browser-resource-monitor.service" <<SERVICE
[Unit]
Description=Agent Browser read-only resource monitor

[Service]
Type=oneshot
ExecStart=$bin service resources --write-monitor-summary --json
SERVICE

cat >"$unit_dir/agent-browser-resource-monitor.timer" <<TIMER
[Unit]
Description=Run Agent Browser read-only resource monitor

[Timer]
OnBootSec=2min
OnUnitActiveSec=$interval
Unit=agent-browser-resource-monitor.service

[Install]
WantedBy=timers.target
TIMER

systemctl --user daemon-reload
systemctl --user enable --now agent-browser-resource-monitor.timer
systemctl --user status agent-browser-resource-monitor.timer --no-pager
