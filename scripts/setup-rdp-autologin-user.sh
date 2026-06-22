#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
USER_NAME="${AGENT_BROWSER_RDP_USERNAME:-agent-browser-rdp}"
GUAC_DIR="${AGENT_BROWSER_GUACAMOLE_DIR:-$HOME/.agent-browser/guacamole}"
SECRET_FILE="${AGENT_BROWSER_GUACAMOLE_SECRET_FILE:-$HOME/.agent-browser/secrets/guacamole.env}"
CONNECTION_ID="${AGENT_BROWSER_GUACAMOLE_CONNECTION_ID:-1}"

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

PASSWORD="$(python3 - <<'PY'
import secrets
import string

alphabet = string.ascii_letters + string.digits + "-_."
print("".join(secrets.choice(alphabet) for _ in range(32)))
PY
)"

if ! sudo -v; then
  echo "This setup needs sudo to create/update the local XRDP user." >&2
  echo "Run it from an interactive terminal where sudo can prompt for your password:" >&2
  echo "  bash scripts/setup-rdp-autologin-user.sh" >&2
  exit 2
fi

if ! getent passwd "$USER_NAME" >/dev/null; then
  sudo useradd --create-home --shell /bin/bash --comment "agent-browser RDP viewer session" "$USER_NAME"
fi

printf '%s:%s\n' "$USER_NAME" "$PASSWORD" | sudo chpasswd
sudo usermod -aG ssl-cert xrdp >/dev/null 2>&1 || true

sudo -u "$USER_NAME" mkdir -p "/home/$USER_NAME/.config/openbox"
sudo tee "/home/$USER_NAME/.xsession" >/dev/null <<'EOF'
#!/bin/sh
xsetroot -solid '#20252b' 2>/dev/null || true
xterm -geometry 100x28+40+40 -title 'agent-browser RDP session' &
exec openbox-session
EOF
sudo chmod 700 "/home/$USER_NAME/.xsession"
sudo chown "$USER_NAME:$USER_NAME" "/home/$USER_NAME/.xsession"

mkdir -p "$(dirname "$SECRET_FILE")"
python3 - "$SECRET_FILE" "$USER_NAME" "$PASSWORD" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
user = sys.argv[2]
password = sys.argv[3]
text = path.read_text() if path.exists() else ""
lines = [
    line for line in text.splitlines()
    if not line.startswith("XRDP_AGENT_BROWSER_USERNAME=")
    and not line.startswith("XRDP_AGENT_BROWSER_PASSWORD=")
]
lines.append(f"XRDP_AGENT_BROWSER_USERNAME={user}")
lines.append(f"XRDP_AGENT_BROWSER_PASSWORD={password}")
path.write_text("\n".join(lines) + "\n")
path.chmod(0o600)
PY

ensure_guacamole_postgres

SQL="$(python3 - "$CONNECTION_ID" "$USER_NAME" "$PASSWORD" <<'PY'
import sys

connection_id = int(sys.argv[1])
username = sys.argv[2]
password = sys.argv[3]

def quote(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"

rows = {
    "username": username,
    "password": password,
    "enable-drive": "false",
    "enable-audio-input": "false",
}

values = ",\n".join(
    f"  ({connection_id}, {quote(name)}, {quote(value)})"
    for name, value in rows.items()
)
print(f"""
INSERT INTO guacamole_connection_parameter (connection_id, parameter_name, parameter_value)
VALUES
{values}
ON CONFLICT (connection_id, parameter_name) DO UPDATE
SET parameter_value = EXCLUDED.parameter_value;

INSERT INTO guacamole_connection_permission (entity_id, connection_id, permission)
SELECT entity_id, {connection_id}, 'READ'::guacamole_object_permission_type
FROM guacamole_entity
WHERE type = 'USER'
ON CONFLICT DO NOTHING;
""".strip())
PY
)"

(
  cd "$GUAC_DIR"
  printf '%s\n' "$SQL" | docker compose exec -T postgres psql -U guacamole_user -d guacamole_db
)

sudo systemctl restart xrdp-sesman xrdp

echo "Configured Guacamole connection $CONNECTION_ID for XRDP autologin as $USER_NAME."
echo "The generated password was stored in $SECRET_FILE."
