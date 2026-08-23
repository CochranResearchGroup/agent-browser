#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
      echo "Usage: bash scripts/sync-rdp-guac-route-specific-user-pool.sh [--dry-run]" >&2
      exit 2
      ;;
  esac
done

GUAC_DIR="${AGENT_BROWSER_GUACAMOLE_DIR:-$HOME/.agent-browser/guacamole}"
SECRET_FILE="${AGENT_BROWSER_GUACAMOLE_SECRET_FILE:-$HOME/.agent-browser/secrets/guacamole.env}"
HOSTNAME="${AGENT_BROWSER_RDP_TARGET_HOST:-host.docker.internal}"
PORT="${AGENT_BROWSER_RDP_TARGET_PORT:-3389}"
POSTGRES_CONTAINER="${AGENT_BROWSER_GUACAMOLE_POSTGRES_CONTAINER:-agent-browser-guacamole-postgres}"
ROUTE_USER_HELPER="$SCRIPT_DIR/lib/rdp-route-user-pool.py"

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

compose_env_args=()
if [[ -r "$GUAC_DIR/.env" ]]; then
  compose_env_args+=(--env-file "$GUAC_DIR/.env")
fi
compose_env_args+=(--env-file "$SECRET_FILE")

compose_project_args=()
if docker inspect "$POSTGRES_CONTAINER" >/dev/null 2>&1; then
  retained_project="$(docker inspect -f '{{index .Config.Labels "com.docker.compose.project"}}' "$POSTGRES_CONTAINER")"
  if [[ ! "$retained_project" =~ ^[a-z0-9][a-z0-9_-]*$ ]]; then
    echo "Retained Guacamole PostgreSQL container has no usable Compose project label." >&2
    exit 1
  fi
  compose_project_args+=(--project-name "$retained_project")
fi

compose() {
  (
    cd "$GUAC_DIR"
    docker compose "${compose_project_args[@]}" "${compose_env_args[@]}" "$@"
  )
}

ensure_guacamole_postgres() {
  bash "$SCRIPT_DIR/ensure-rdp-guac-postgres.sh" --apply
}

ROUTE_USER_POOL_JSON="$(python3 "$ROUTE_USER_HELPER" resolve --secret-file "$SECRET_FILE")"
ROUTE_COUNT="$(python3 -c 'import json,sys; print(len(json.load(sys.stdin)))' <<<"$ROUTE_USER_POOL_JSON")"
ROUTE_SUMMARY="$(python3 -c 'import json,sys; [print("{}\t{}\t{}".format(route["id"], route["routeUser"], route["connectionName"])) for route in json.load(sys.stdin)]' <<<"$ROUTE_USER_POOL_JSON")"

while IFS=$'\t' read -r route_id route_user connection_name; do
  [[ -n "$route_id" ]] || continue
  if ! getent passwd "$route_user" >/dev/null; then
    echo "Route Linux user does not exist: $route_id ($route_user)" >&2
    exit 1
  fi
done <<<"$ROUTE_SUMMARY"

if [[ "$DRY_RUN" == "1" ]]; then
  cat <<EOF
agent-browser route-specific Guacamole route-pool sync dry run

Guacamole compose directory: $GUAC_DIR
Secret file: $SECRET_FILE
RDP target: $HOSTNAME:$PORT
Configured routes: $ROUTE_COUNT
$ROUTE_SUMMARY

No Guacamole records were changed.
This command does not create Linux users, rotate passwords, change XRDP policy,
restart XRDP, or require sudo.
EOF
  exit 0
fi

ensure_guacamole_postgres

SQL="$(printf '%s' "$ROUTE_USER_POOL_JSON" | python3 "$ROUTE_USER_HELPER" sql --hostname "$HOSTNAME" --port "$PORT")"

printf '%s\n' "$SQL" |
  compose exec -T postgres psql -U guacamole_user -d guacamole_db -v ON_ERROR_STOP=1
compose exec -T postgres psql -U guacamole_user -d guacamole_db \
  -v ON_ERROR_STOP=1 -c "CHECKPOINT;" >/dev/null

echo "Configured $ROUTE_COUNT canonical Guacamole RDP routes with distinct route-specific users."
echo "Guacamole Postgres route writes checkpoint completed."
echo "Next: open every configured route in Guacamole, then run node scripts/inspect-rdp-route-displays.js."
