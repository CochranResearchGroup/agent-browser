#!/usr/bin/env bash
set -euo pipefail

PORT="${AGENT_BROWSER_DASHBOARD_PORT:-4848}"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
UNIT_PATH="$UNIT_DIR/agent-browser-dashboard.service"
INTERLOCK_UNIT_PATH="$UNIT_DIR/agent-browser-runtime-interlock.service"
INTERLOCK_TIMER_PATH="$UNIT_DIR/agent-browser-runtime-interlock.timer"
POSTGRES_BACKUP_UNIT_PATH="$UNIT_DIR/agent-browser-guacamole-postgres-backup.service"
POSTGRES_BACKUP_TIMER_PATH="$UNIT_DIR/agent-browser-guacamole-postgres-backup.timer"
AGENT_BROWSER_BIN="${AGENT_BROWSER_BIN:-$(command -v agent-browser || true)}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOCAL_LIB_DIR="${AGENT_BROWSER_LOCAL_LIB_DIR:-$HOME/.local/lib/agent-browser}"
POSTGRES_DURABILITY_BIN="$LOCAL_LIB_DIR/guacamole-postgres-durability.sh"
INTERLOCK_INTERVAL="${AGENT_BROWSER_RUNTIME_INTERLOCK_INTERVAL:-5min}"

if [[ -z "$AGENT_BROWSER_BIN" ]]; then
  echo "agent-browser was not found on PATH. Set AGENT_BROWSER_BIN to the installed binary path." >&2
  exit 1
fi

if [[ ! "$INTERLOCK_INTERVAL" =~ ^[1-9][0-9]*(s|min|h)$ ]]; then
  echo "AGENT_BROWSER_RUNTIME_INTERLOCK_INTERVAL must look like 60s, 5min, or 1h." >&2
  exit 2
fi

mkdir -p "$UNIT_DIR"
mkdir -p "$LOCAL_LIB_DIR"
install -m 0755 "$ROOT_DIR/scripts/guacamole-postgres-durability.sh" "$POSTGRES_DURABILITY_BIN"

cat > "$UNIT_PATH" <<UNIT
[Unit]
Description=agent-browser dashboard
Documentation=https://github.com/CochranResearchGroup/agent-browser
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=-$HOME/.agent-browser/.env
Environment=AGENT_BROWSER_DASHBOARD=1
Environment=AGENT_BROWSER_DASHBOARD_PORT=$PORT
ExecStart=$AGENT_BROWSER_BIN
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
UNIT

cat > "$INTERLOCK_UNIT_PATH" <<UNIT
[Unit]
Description=agent-browser runtime health interlock
Documentation=https://github.com/CochranResearchGroup/agent-browser
After=agent-browser-dashboard.service network-online.target
Wants=agent-browser-dashboard.service network-online.target

[Service]
Type=oneshot
Environment=PATH=$PATH
ExecStart=$AGENT_BROWSER_BIN install workstation reconcile --json
TimeoutStartSec=5min

[Install]
WantedBy=default.target
UNIT

cat > "$INTERLOCK_TIMER_PATH" <<UNIT
[Unit]
Description=Periodically reconcile agent-browser runtime health

[Timer]
OnBootSec=20s
OnUnitInactiveSec=$INTERLOCK_INTERVAL
AccuracySec=5s
Persistent=true
Unit=agent-browser-runtime-interlock.service

[Install]
WantedBy=timers.target
UNIT

cat > "$POSTGRES_BACKUP_UNIT_PATH" <<UNIT
[Unit]
Description=Back up agent-browser Guacamole PostgreSQL
Documentation=https://github.com/CochranResearchGroup/agent-browser
After=network-online.target
Wants=network-online.target
StartLimitIntervalSec=30min
StartLimitBurst=4

[Service]
Type=oneshot
Environment=PATH=$PATH
ExecStart=/usr/bin/bash $POSTGRES_DURABILITY_BIN backup
TimeoutStartSec=10min
Restart=on-failure
RestartSec=5min

[Install]
WantedBy=default.target
UNIT

cat > "$POSTGRES_BACKUP_TIMER_PATH" <<UNIT
[Unit]
Description=Daily agent-browser Guacamole PostgreSQL backup

[Timer]
OnCalendar=daily
RandomizedDelaySec=15min
Persistent=true
Unit=agent-browser-guacamole-postgres-backup.service

[Install]
WantedBy=timers.target
UNIT

systemctl --user daemon-reload
systemctl --user enable --now \
  agent-browser-dashboard.service \
  agent-browser-runtime-interlock.timer \
  agent-browser-guacamole-postgres-backup.timer
systemctl --user start agent-browser-runtime-interlock.service
systemctl --user status agent-browser-dashboard.service --no-pager
systemctl --user status agent-browser-runtime-interlock.timer --no-pager
systemctl --user show agent-browser-runtime-interlock.service \
  --property=ActiveState \
  --property=SubState \
  --property=Result \
  --property=ExecMainStatus \
  --no-pager
