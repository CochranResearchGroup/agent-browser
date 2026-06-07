#!/usr/bin/env bash
set -euo pipefail

unit_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

systemctl --user disable --now agent-browser-resource-monitor.timer 2>/dev/null || true
rm -f "$unit_dir/agent-browser-resource-monitor.timer"
rm -f "$unit_dir/agent-browser-resource-monitor.service"
systemctl --user daemon-reload
systemctl --user reset-failed agent-browser-resource-monitor.service 2>/dev/null || true
echo "Removed agent-browser-resource-monitor user timer"
