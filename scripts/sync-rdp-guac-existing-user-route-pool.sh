#!/usr/bin/env bash
set -euo pipefail

DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
    --)
      ;;
    --dry-run)
      DRY_RUN=1
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: bash scripts/sync-rdp-guac-existing-user-route-pool.sh [--dry-run]" >&2
      exit 2
      ;;
  esac
done

GUAC_DIR="${AGENT_BROWSER_GUACAMOLE_DIR:-$HOME/.agent-browser/guacamole}"
SECRET_FILE="${AGENT_BROWSER_GUACAMOLE_SECRET_FILE:-$HOME/.agent-browser/secrets/guacamole.env}"
HOSTNAME="${AGENT_BROWSER_RDP_TARGET_HOST:-host.docker.internal}"
PORT="${AGENT_BROWSER_RDP_TARGET_PORT:-3389}"
CONNECTION_A="${AGENT_BROWSER_RDP_ROUTE_A_CONNECTION_NAME:-Agent Browser RDP Existing User Route A}"
CONNECTION_B="${AGENT_BROWSER_RDP_ROUTE_B_CONNECTION_NAME:-Agent Browser RDP Existing User Route B}"
COLOR_DEPTH_A="${AGENT_BROWSER_RDP_ROUTE_A_COLOR_DEPTH:-24}"
COLOR_DEPTH_B="${AGENT_BROWSER_RDP_ROUTE_B_COLOR_DEPTH:-32}"

if [[ ! -d "$GUAC_DIR" ]]; then
  echo "Missing Guacamole compose directory: $GUAC_DIR" >&2
  exit 1
fi

if [[ ! -r "$SECRET_FILE" ]]; then
  echo "Missing readable Guacamole secret file: $SECRET_FILE" >&2
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

read_secret() {
  local key="$1"
  python3 - "$SECRET_FILE" "$key" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
key = sys.argv[2]
for line in path.read_text().splitlines():
    if not line or line.strip().startswith("#") or "=" not in line:
        continue
    name, value = line.split("=", 1)
    if name.strip() == key:
        value = value.strip()
        if (value.startswith('"') and value.endswith('"')) or (value.startswith("'") and value.endswith("'")):
            value = value[1:-1]
        print(value)
        raise SystemExit(0)
raise SystemExit(1)
PY
}

USERNAME="${AGENT_BROWSER_RDP_EXISTING_USERNAME:-${XRDP_AGENT_BROWSER_USERNAME:-}}"
PASSWORD="${AGENT_BROWSER_RDP_EXISTING_PASSWORD:-${XRDP_AGENT_BROWSER_PASSWORD:-}}"
if [[ -z "$USERNAME" ]]; then
  USERNAME="$(read_secret XRDP_AGENT_BROWSER_USERNAME || true)"
fi
if [[ -z "$PASSWORD" ]]; then
  PASSWORD="$(read_secret XRDP_AGENT_BROWSER_PASSWORD || true)"
fi

if [[ -z "$USERNAME" || -z "$PASSWORD" ]]; then
  echo "Missing XRDP_AGENT_BROWSER_USERNAME or XRDP_AGENT_BROWSER_PASSWORD in $SECRET_FILE" >&2
  exit 1
fi

if [[ "$COLOR_DEPTH_A" == "$COLOR_DEPTH_B" ]]; then
  echo "Route A and B color depths must differ so XRDP Policy=Default can allocate distinct sessions." >&2
  exit 1
fi

if [[ "$DRY_RUN" == "1" ]]; then
  cat <<EOF
agent-browser existing-user Guacamole route-pool sync dry run

Guacamole compose directory: $GUAC_DIR
Secret file: $SECRET_FILE
RDP target: $HOSTNAME:$PORT
Existing RDP user: $USERNAME
Route A connection: $CONNECTION_A
Route A color depth: $COLOR_DEPTH_A
Route B connection: $CONNECTION_B
Route B color depth: $COLOR_DEPTH_B

No Guacamole records were changed.
This command does not create Linux users and does not require sudo.
EOF
  exit 0
fi

SQL="$(python3 - "$CONNECTION_A" "$COLOR_DEPTH_A" "$CONNECTION_B" "$COLOR_DEPTH_B" "$USERNAME" "$PASSWORD" "$HOSTNAME" "$PORT" <<'PY'
import sys

connection_a, color_a, connection_b, color_b, username, password, hostname, port = sys.argv[1:]

def quote(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"

def connection_block(label: str, connection_name: str, color_depth: str) -> str:
    params = {
        "hostname": hostname,
        "port": port,
        "username": username,
        "password": password,
        "security": "any",
        "ignore-cert": "true",
        "resize-method": "display-update",
        "color-depth": color_depth,
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

  RAISE NOTICE 'configured existing-user route % as connection % with color-depth %', {quote(label)}, route_connection_id, {quote(color_depth)};
END $$;
""".strip()

print("\n\n".join([
    connection_block("A", connection_a, color_a),
    connection_block("B", connection_b, color_b),
]))
PY
)"

(
  cd "$GUAC_DIR"
  printf '%s\n' "$SQL" | docker compose exec -T postgres psql -U guacamole_user -d guacamole_db
)

echo "Configured two Guacamole RDP connections for existing user $USERNAME."
echo "Next: open both routes in Guacamole, then run pnpm inspect:rdp-route-displays."
