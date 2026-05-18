#!/usr/bin/env bash
set -euo pipefail

PORT="${AGENT_BROWSER_DASHBOARD_PORT:-4848}"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
UNIT_PATH="$UNIT_DIR/agent-browser-dashboard.service"
AGENT_BROWSER_BIN="${AGENT_BROWSER_BIN:-$(command -v agent-browser || true)}"

if [[ -z "$AGENT_BROWSER_BIN" ]]; then
  echo "agent-browser was not found on PATH. Set AGENT_BROWSER_BIN to the installed binary path." >&2
  exit 1
fi

mkdir -p "$UNIT_DIR"

cat > "$UNIT_PATH" <<UNIT
[Unit]
Description=agent-browser dashboard
Documentation=https://github.com/CochranResearchGroup/agent-browser
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
Environment=AGENT_BROWSER_DASHBOARD=1
Environment=AGENT_BROWSER_DASHBOARD_PORT=$PORT
ExecStart=$AGENT_BROWSER_BIN
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
UNIT

systemctl --user daemon-reload
systemctl --user enable --now agent-browser-dashboard.service
systemctl --user status agent-browser-dashboard.service --no-pager
