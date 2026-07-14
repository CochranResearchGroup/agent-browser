#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DRY_RUN=0
FORCE=0
for arg in "$@"; do
  case "$arg" in
    --)
      ;;
    --dry-run)
      DRY_RUN=1
      ;;
    --force)
      FORCE=1
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: bash scripts/setup-rdp-guac-route-pool.sh [--dry-run] [--force]" >&2
      exit 2
      ;;
  esac
done

GUAC_DIR="${AGENT_BROWSER_GUACAMOLE_DIR:-$HOME/.agent-browser/guacamole}"
SECRET_FILE="${AGENT_BROWSER_GUACAMOLE_SECRET_FILE:-$HOME/.agent-browser/secrets/guacamole.env}"
HOSTNAME="${AGENT_BROWSER_RDP_TARGET_HOST:-host.docker.internal}"
PORT="${AGENT_BROWSER_RDP_TARGET_PORT:-3389}"
USER_A="${AGENT_BROWSER_RDP_ROUTE_A_USERNAME:-agent-browser-rdp-a}"
USER_B="${AGENT_BROWSER_RDP_ROUTE_B_USERNAME:-agent-browser-rdp-b}"
CONNECTION_A="${AGENT_BROWSER_RDP_ROUTE_A_CONNECTION_NAME:-Agent Browser RDP Route A}"
CONNECTION_B="${AGENT_BROWSER_RDP_ROUTE_B_CONNECTION_NAME:-Agent Browser RDP Route B}"
PRIVILEGED_HELPER="${AGENT_BROWSER_PRIVILEGED_HELPER:-/usr/local/libexec/agent-browser/agent-browser-privileged-helper}"

if [[ ! -d "$GUAC_DIR" ]]; then
  echo "Missing Guacamole compose directory: $GUAC_DIR" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required" >&2
  exit 1
fi

ensure_guacamole_postgres() {
  bash "$SCRIPT_DIR/ensure-rdp-guac-postgres.sh" --apply
}

read_secret() {
  local key="$1"
  if [[ ! -f "$SECRET_FILE" ]]; then
    return 1
  fi

  python3 - "$SECRET_FILE" "$key" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
key = sys.argv[2]
prefix = key + "="

for line in path.read_text().splitlines():
    if line.startswith(prefix):
        print(line[len(prefix):])
        sys.exit(0)

sys.exit(1)
PY
}

privileged_helper_available() {
  [[ -x "$PRIVILEGED_HELPER" ]] && sudo -n "$PRIVILEGED_HELPER" check >/dev/null 2>&1
}

display_gate_report() {
  if [[ -f "scripts/inspect-rdp-route-displays.js" ]] && command -v node >/dev/null 2>&1; then
    node scripts/inspect-rdp-route-displays.js 2>/dev/null || true
  fi
}

display_gate_allows_route_users() {
  local report="$1"

  if [[ -z "$report" ]]; then
    return 1
  fi

  ROUTE_DISPLAY_REPORT="$report" python3 - <<'PY'
import json
import os
import sys

try:
    report = json.loads(os.environ["ROUTE_DISPLAY_REPORT"])
except Exception:
    sys.exit(1)

if report.get("success") is True:
    sys.exit(1)

route_specific = report.get("routeSpecificUsers") or {}
for route in ("A", "B"):
    if ((route_specific.get(route) or {}).get("displayName")):
        sys.exit(1)

next_step = str(report.get("nextStep") or "")
if (
    "collapsing existing-user routes" in next_step
    or "existing agent-browser-rdp user has one active display only" in next_step
):
    sys.exit(0)

sys.exit(1)
PY
}

DISPLAY_GATE_REPORT="$(display_gate_report)"
DISPLAY_GATE_STATUS="unavailable"
if display_gate_allows_route_users "$DISPLAY_GATE_REPORT"; then
  DISPLAY_GATE_STATUS="allows_route_specific_fallback"
else
  DISPLAY_GATE_STATUS="not_proven"
fi

if [[ "$DRY_RUN" == "1" ]]; then
  cat <<EOF
agent-browser RDP Guacamole route-pool setup dry run

Guacamole compose directory: $GUAC_DIR
Secret file: $SECRET_FILE
RDP target: $HOSTNAME:$PORT
Route A user: $USER_A
Route A connection: $CONNECTION_A
Route B user: $USER_B
Route B connection: $CONNECTION_B
Display isolation gate: $DISPLAY_GATE_STATUS
Privileged helper: $PRIVILEGED_HELPER

No users, secrets, Guacamole records, or services were changed.
Install one-time privileges with:
  pnpm install:privileges -- --apply

Then run without --dry-run. If the privileged helper is not installed, run from
an interactive terminal to allow sudo prompts.

Important: this host-XRDP-user bootstrap only creates distinct RDP sessions.
P03 is complete only after the many-to-many live gate proves Browser A and
Browser B are actually visible through those routes at the same time.
After opening both RDP sessions, run: pnpm inspect:rdp-route-displays
EOF
  exit 0
fi

if [[ "$FORCE" != "1" && "$DISPLAY_GATE_STATUS" != "allows_route_specific_fallback" ]]; then
  cat >&2 <<EOF
Refusing to create route-specific RDP users without route-display evidence.

Run:
  agent-browser doctor remote-view
  pnpm inspect:rdp-route-displays

This setup command is allowed only after the display inspector proves the
existing agent-browser-rdp route topology collapsed to one display. Use
--force only for a reviewed operator override.
EOF
  exit 2
fi

EXISTING_PASS_A="$(read_secret XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD || true)"
EXISTING_PASS_B="$(read_secret XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD || true)"
REUSE_EXISTING_ROUTE_USERS=0
if getent passwd "$USER_A" >/dev/null \
  && getent passwd "$USER_B" >/dev/null \
  && [[ -n "$EXISTING_PASS_A" ]] \
  && [[ -n "$EXISTING_PASS_B" ]]; then
  REUSE_EXISTING_ROUTE_USERS=1
fi

USE_PRIVILEGED_HELPER=0
if [[ "$REUSE_EXISTING_ROUTE_USERS" != "1" ]]; then
  if privileged_helper_available; then
    USE_PRIVILEGED_HELPER=1
  elif ! sudo -v; then
    echo "This setup needs sudo to create/update local XRDP users." >&2
    echo "Install the one-time helper from an interactive terminal:" >&2
    echo "  pnpm install:privileges -- --apply" >&2
    echo "Or run this setup from an interactive terminal where sudo can prompt." >&2
    exit 2
  fi
fi

PASSWORDS="{}"
if [[ "$REUSE_EXISTING_ROUTE_USERS" != "1" ]]; then
  PASSWORDS="$(python3 - "$USER_A" "$USER_B" <<'PY'
import json
import secrets
import string
import sys

alphabet = string.ascii_letters + string.digits + "-_."
users = sys.argv[1:]
print(json.dumps({
    user: "".join(secrets.choice(alphabet) for _ in range(32))
    for user in users
}))
PY
)"
fi

setup_user() {
  local user_name="$1"
  local password="$2"

  if [[ "$USE_PRIVILEGED_HELPER" == "1" ]]; then
    printf '%s\n' "$password" | sudo -n "$PRIVILEGED_HELPER" ensure-rdp-route-user --user "$user_name"
    return
  fi

  if ! getent passwd "$user_name" >/dev/null; then
    sudo useradd --create-home --shell /bin/bash --comment "agent-browser route-pool RDP session" "$user_name"
  fi

  printf '%s:%s\n' "$user_name" "$password" | sudo chpasswd
  sudo usermod -aG ssl-cert xrdp >/dev/null 2>&1 || true

  sudo -u "$user_name" mkdir -p "/home/$user_name/.config/openbox"
  sudo tee "/home/$user_name/.xsession" >/dev/null <<'EOF'
#!/bin/sh
xsetroot -solid '#20252b' 2>/dev/null || true
if command -v openbox-session >/dev/null 2>&1; then
  openbox-session &
fi
while true; do
  sleep 3600
done
EOF
  sudo chmod 700 "/home/$user_name/.xsession"
  sudo chown "$user_name:$user_name" "/home/$user_name/.xsession"
}

if [[ "$REUSE_EXISTING_ROUTE_USERS" == "1" ]]; then
  PASS_A="$EXISTING_PASS_A"
  PASS_B="$EXISTING_PASS_B"
else
  PASS_A="$(python3 - "$PASSWORDS" "$USER_A" <<'PY'
import json
import sys

print(json.loads(sys.argv[1])[sys.argv[2]])
PY
)"
  PASS_B="$(python3 - "$PASSWORDS" "$USER_B" <<'PY'
import json
import sys

print(json.loads(sys.argv[1])[sys.argv[2]])
PY
)"

  setup_user "$USER_A" "$PASS_A"
  setup_user "$USER_B" "$PASS_B"
fi

mkdir -p "$(dirname "$SECRET_FILE")"
python3 - "$SECRET_FILE" "$USER_A" "$PASS_A" "$USER_B" "$PASS_B" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
user_a = sys.argv[2]
pass_a = sys.argv[3]
user_b = sys.argv[4]
pass_b = sys.argv[5]
remove_prefixes = (
    "XRDP_AGENT_BROWSER_ROUTE_A_USERNAME=",
    "XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD=",
    "XRDP_AGENT_BROWSER_ROUTE_B_USERNAME=",
    "XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD=",
)
text = path.read_text() if path.exists() else ""
lines = [
    line for line in text.splitlines()
    if not any(line.startswith(prefix) for prefix in remove_prefixes)
]
lines.extend([
    f"XRDP_AGENT_BROWSER_ROUTE_A_USERNAME={user_a}",
    f"XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD={pass_a}",
    f"XRDP_AGENT_BROWSER_ROUTE_B_USERNAME={user_b}",
    f"XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD={pass_b}",
])
path.write_text("\n".join(lines) + "\n")
path.chmod(0o600)
PY

ensure_guacamole_postgres

SQL="$(python3 - "$CONNECTION_A" "$USER_A" "$PASS_A" "$CONNECTION_B" "$USER_B" "$PASS_B" "$HOSTNAME" "$PORT" <<'PY'
import sys

connection_a, user_a, pass_a, connection_b, user_b, pass_b, hostname, port = sys.argv[1:]

def quote(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"

def connection_block(label: str, connection_name: str, username: str, password: str) -> str:
    params = {
        "hostname": hostname,
        "port": port,
        "username": username,
        "password": password,
        "security": "any",
        "ignore-cert": "true",
        "resize-method": "display-update",
        "enable-audio-input": "false",
        "enable-drive": "false",
        "enable-theming": "false",
        "enable-wallpaper": "false",
    }
    values = ",\n".join(
        f"    (route_connection_id, {quote(name)}, {quote(value)})"
        for name, value in params.items()
    )
    return f"""
DO $$
DECLARE
  route_connection_id integer;
BEGIN
  SELECT c.connection_id INTO route_connection_id
  FROM guacamole_connection c
  WHERE c.connection_name = {quote(connection_name)}
    AND c.parent_id IS NULL
  ORDER BY c.connection_id
  LIMIT 1;

  IF route_connection_id IS NULL THEN
    INSERT INTO guacamole_connection (
      connection_name,
      protocol,
      max_connections,
      max_connections_per_user
    )
    VALUES ({quote(connection_name)}, 'rdp', 4, 2)
    RETURNING guacamole_connection.connection_id INTO route_connection_id;
  ELSE
    UPDATE guacamole_connection
    SET protocol = 'rdp',
        max_connections = 4,
        max_connections_per_user = 2
    WHERE guacamole_connection.connection_id = route_connection_id;
  END IF;

  INSERT INTO guacamole_connection_parameter (connection_id, parameter_name, parameter_value)
  VALUES
{values}
  ON CONFLICT (connection_id, parameter_name) DO UPDATE
  SET parameter_value = EXCLUDED.parameter_value;

  INSERT INTO guacamole_connection_permission (entity_id, connection_id, permission)
  SELECT entity.entity_id, route_connection_id, 'READ'::guacamole_object_permission_type
  FROM guacamole_entity entity
  WHERE entity.type = 'USER'
  ON CONFLICT DO NOTHING;

  RAISE NOTICE 'configured % route % as connection %', {quote(label)}, {quote(connection_name)}, route_connection_id;
END $$;
""".strip()

print("\n\n".join([
    connection_block("A", connection_a, user_a, pass_a),
    connection_block("B", connection_b, user_b, pass_b),
]))
PY
)"

(
  cd "$GUAC_DIR"
  printf '%s\n' "$SQL" | docker compose exec -T postgres psql -U guacamole_user -d guacamole_db -v ON_ERROR_STOP=1
  docker compose exec -T postgres psql -U guacamole_user -d guacamole_db -v ON_ERROR_STOP=1 -c "CHECKPOINT;" >/dev/null
)

if [[ "$REUSE_EXISTING_ROUTE_USERS" != "1" ]]; then
  if [[ "$USE_PRIVILEGED_HELPER" == "1" ]]; then
    sudo -n "$PRIVILEGED_HELPER" restart-xrdp
  else
    sudo systemctl restart xrdp-sesman xrdp
  fi
fi

echo "Configured two Guacamole RDP route-pool users and connections."
if [[ "$REUSE_EXISTING_ROUTE_USERS" == "1" ]]; then
  echo "Reused existing route-specific XRDP users and stored route secrets."
fi
echo "Guacamole Postgres route writes checkpoint completed."
echo "Secrets were stored in $SECRET_FILE."
echo "Next: pnpm test:rdp-guac-route-pool-readiness"
echo "After opening both RDP sessions, run: pnpm inspect:rdp-route-displays"
