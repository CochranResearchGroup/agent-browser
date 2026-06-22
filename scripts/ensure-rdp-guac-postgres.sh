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
    --apply)
      DRY_RUN=0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: bash scripts/ensure-rdp-guac-postgres.sh [--dry-run|--apply]" >&2
      exit 2
      ;;
  esac
done

GUAC_DIR="${AGENT_BROWSER_GUACAMOLE_DIR:-$HOME/.agent-browser/guacamole}"
SECRET_FILE="${AGENT_BROWSER_GUACAMOLE_SECRET_FILE:-$HOME/.agent-browser/secrets/guacamole.env}"
INIT_SQL="${AGENT_BROWSER_GUACAMOLE_INIT_SQL:-$GUAC_DIR/init/001-initdb.sql}"
POSTGRES_CONTAINER="${AGENT_BROWSER_GUACAMOLE_POSTGRES_CONTAINER:-agent-browser-guacamole-postgres}"
POSTGRES_SERVICE="${AGENT_BROWSER_GUACAMOLE_POSTGRES_SERVICE:-postgres}"
POSTGRES_USER="${AGENT_BROWSER_GUACAMOLE_POSTGRES_USER:-guacamole_user}"
POSTGRES_DB="${AGENT_BROWSER_GUACAMOLE_POSTGRES_DB:-guacamole_db}"

REQUIRED_TABLES=(
  guacamole_user
  guacamole_entity
  guacamole_connection
  guacamole_connection_parameter
  guacamole_connection_permission
)

if [[ ! -d "$GUAC_DIR" ]]; then
  echo "Missing Guacamole compose directory: $GUAC_DIR" >&2
  exit 1
fi

if [[ ! -r "$GUAC_DIR/compose.yml" ]]; then
  echo "Missing readable Guacamole compose file: $GUAC_DIR/compose.yml" >&2
  exit 1
fi

if [[ ! -r "$INIT_SQL" ]]; then
  echo "Missing readable Guacamole schema SQL: $INIT_SQL" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

compose_env_args=()
if [[ -r "$GUAC_DIR/.env" ]]; then
  compose_env_args+=(--env-file "$GUAC_DIR/.env")
fi
if [[ -r "$SECRET_FILE" ]]; then
  compose_env_args+=(--env-file "$SECRET_FILE")
fi

compose() {
  (
    cd "$GUAC_DIR"
    docker compose "${compose_env_args[@]}" "$@"
  )
}

postgres_running() {
  docker inspect -f '{{.State.Running}}' "$POSTGRES_CONTAINER" 2>/dev/null | grep -qx true
}

wait_for_postgres() {
  local attempt
  for attempt in $(seq 1 40); do
    if compose exec -T "$POSTGRES_SERVICE" pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  return 1
}

query_present_tables() {
  local table_list
  table_list="$(printf "'%s'," "${REQUIRED_TABLES[@]}")"
  table_list="${table_list%,}"
  compose exec -T "$POSTGRES_SERVICE" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -A -c "
select table_name
from information_schema.tables
where table_schema = 'public'
  and table_name = any(array[$table_list])
order by table_name;
" 2>/dev/null
}

query_guacamole_relation_count() {
  compose exec -T "$POSTGRES_SERVICE" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -A -c "
select count(*)
from pg_class c
join pg_namespace n on n.oid = c.relnamespace
where n.nspname = 'public'
  and c.relname like 'guacamole_%'
  and c.relkind in ('r', 'S', 'v', 'm');
" 2>/dev/null
}

if ! postgres_running; then
  if [[ "$DRY_RUN" == "1" ]]; then
    echo "Guacamole Postgres is not running."
    echo "Would run: docker compose up -d $POSTGRES_SERVICE"
    exit 0
  fi
  echo "Starting Guacamole Postgres..."
  compose up -d "$POSTGRES_SERVICE"
fi

if ! wait_for_postgres; then
  echo "Guacamole Postgres did not become ready through pg_isready." >&2
  exit 1
fi

present_tables="$(query_present_tables || true)"
missing_tables=()
for required in "${REQUIRED_TABLES[@]}"; do
  if ! grep -qx "$required" <<<"$present_tables"; then
    missing_tables+=("$required")
  fi
done

if [[ "${#missing_tables[@]}" == "0" ]]; then
  if [[ "$DRY_RUN" == "1" ]]; then
    echo "Guacamole Postgres schema is ready."
  else
    compose exec -T "$POSTGRES_SERVICE" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 -c "CHECKPOINT;" >/dev/null
    echo "Guacamole Postgres schema is ready; checkpoint completed."
  fi
  exit 0
fi

relation_count="$(query_guacamole_relation_count || echo 0)"
relation_count="${relation_count//$'\n'/}"
relation_count="${relation_count//[[:space:]]/}"
if [[ -z "$relation_count" || ! "$relation_count" =~ ^[0-9]+$ ]]; then
  relation_count=0
fi

if [[ "$relation_count" != "0" ]]; then
  echo "Guacamole schema is partial; refusing automatic full schema import." >&2
  echo "Missing table(s): ${missing_tables[*]}" >&2
  echo "Existing guacamole_* relation count: $relation_count" >&2
  echo "Back up $GUAC_DIR/data/postgres, inspect the database, then rerun after recovery." >&2
  exit 1
fi

if [[ "$DRY_RUN" == "1" ]]; then
  echo "Guacamole Postgres is reachable but schema is absent."
  echo "Would import: $INIT_SQL"
  exit 0
fi

echo "Importing Guacamole schema into empty Postgres database..."
compose exec -T "$POSTGRES_SERVICE" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 < "$INIT_SQL"

present_tables="$(query_present_tables || true)"
missing_after=()
for required in "${REQUIRED_TABLES[@]}"; do
  if ! grep -qx "$required" <<<"$present_tables"; then
    missing_after+=("$required")
  fi
done

if [[ "${#missing_after[@]}" != "0" ]]; then
  echo "Guacamole schema import completed but required table(s) are still missing: ${missing_after[*]}" >&2
  exit 1
fi

compose exec -T "$POSTGRES_SERVICE" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 -c "CHECKPOINT;" >/dev/null
echo "Guacamole Postgres schema is ready; checkpoint completed."
