#!/usr/bin/env bash
set -euo pipefail

COMMAND="${1:-status}"
if [[ "$#" -gt 0 ]]; then
  shift
fi

GUAC_DIR="${AGENT_BROWSER_GUACAMOLE_DIR:-$HOME/.agent-browser/guacamole}"
BACKUP_DIR="${AGENT_BROWSER_GUACAMOLE_BACKUP_DIR:-$HOME/.agent-browser/backups/guacamole-postgres}"
STATE_DIR="${AGENT_BROWSER_GUACAMOLE_STATE_DIR:-$HOME/.agent-browser/state}"
IDENTITY_FILE="${AGENT_BROWSER_GUACAMOLE_IDENTITY_FILE:-$STATE_DIR/guacamole-postgres-identity.json}"
POSTGRES_CONTAINER="${AGENT_BROWSER_GUACAMOLE_POSTGRES_CONTAINER:-agent-browser-guacamole-postgres}"
POSTGRES_USER="${AGENT_BROWSER_GUACAMOLE_POSTGRES_USER:-guacamole_user}"
POSTGRES_DB="${AGENT_BROWSER_GUACAMOLE_POSTGRES_DB:-guacamole_db}"
RETENTION_COUNT="${AGENT_BROWSER_GUACAMOLE_BACKUP_RETENTION:-14}"
BACKUP_WAIT_ATTEMPTS="${AGENT_BROWSER_GUACAMOLE_BACKUP_WAIT_ATTEMPTS:-30}"
REQUIRE_CONTINUITY=0
ALLOW_STALE_SOURCE=0
BACKUP_PATH=""

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --)
      ;;
    --require-continuity)
      REQUIRE_CONTINUITY=1
      ;;
    --allow-stale-source)
      ALLOW_STALE_SOURCE=1
      ;;
    --backup)
      shift
      BACKUP_PATH="${1:-}"
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 2
      ;;
  esac
  shift
done

if [[ ! "$RETENTION_COUNT" =~ ^[1-9][0-9]*$ ]]; then
  echo "AGENT_BROWSER_GUACAMOLE_BACKUP_RETENTION must be a positive integer." >&2
  exit 2
fi
if [[ ! "$BACKUP_WAIT_ATTEMPTS" =~ ^[1-9][0-9]*$ ]]; then
  echo "AGENT_BROWSER_GUACAMOLE_BACKUP_WAIT_ATTEMPTS must be a positive integer." >&2
  exit 2
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

postgres_running() {
  docker inspect -f '{{.State.Running}}' "$POSTGRES_CONTAINER" 2>/dev/null |
    grep -qx true
}

require_postgres() {
  if ! postgres_running; then
    echo "Guacamole PostgreSQL container is not running: $POSTGRES_CONTAINER" >&2
    exit 1
  fi
  if ! docker exec "$POSTGRES_CONTAINER" pg_isready \
    -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null 2>&1; then
    echo "Guacamole PostgreSQL is not ready." >&2
    exit 1
  fi
}

wait_for_postgres() {
  local attempt
  for attempt in $(seq 1 "$BACKUP_WAIT_ATTEMPTS"); do
    if postgres_running &&
      docker exec "$POSTGRES_CONTAINER" pg_isready \
        -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  return 1
}

query_scalar() {
  docker exec "$POSTGRES_CONTAINER" psql \
    -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 -Atc "$1"
}

system_identifier() {
  query_scalar "select system_identifier from pg_control_system();"
}

mount_line() {
  docker exec "$POSTGRES_CONTAINER" sh -lc \
    "grep ' /var/lib/postgresql/data ' /proc/self/mountinfo | head -n 1"
}

mount_fstype() {
  local line
  line="$(mount_line)"
  printf '%s\n' "${line#* - }" | awk '{print $1}'
}

inspect_mount_type() {
  docker inspect "$POSTGRES_CONTAINER" \
    --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Type}}{{end}}{{end}}'
}

inspect_mount_source() {
  docker inspect "$POSTGRES_CONTAINER" \
    --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Source}}{{end}}{{end}}'
}

recorded_system_identifier() {
  if [[ ! -r "$IDENTITY_FILE" ]]; then
    return 0
  fi
  node -e '
    const fs = require("node:fs");
    const value = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    process.stdout.write(String(value.systemIdentifier || ""));
  ' "$IDENTITY_FILE"
}

recorded_identity_field() {
  local field="$1"
  if [[ ! -r "$IDENTITY_FILE" ]]; then
    return 0
  fi
  node -e '
    const fs = require("node:fs");
    const [path, field] = process.argv.slice(1);
    const value = JSON.parse(fs.readFileSync(path, "utf8"));
    process.stdout.write(String(value[field] || ""));
  ' "$IDENTITY_FILE" "$field"
}

status_command() {
  require_postgres
  local current_id fstype declared_type source recorded_id
  local recorded_type recorded_fstype recorded_source continuity issue
  current_id="$(system_identifier)"
  fstype="$(mount_fstype)"
  declared_type="$(inspect_mount_type)"
  source="$(inspect_mount_source)"
  recorded_id="$(recorded_system_identifier)"
  recorded_type="$(recorded_identity_field declaredMountType)"
  recorded_fstype="$(recorded_identity_field runningMountFilesystem)"
  recorded_source="$(recorded_identity_field mountSource)"
  continuity="ready"
  issue="none"

  if [[ "$fstype" == "tmpfs" && "$declared_type" == "bind" ]]; then
    continuity="blocked"
    issue="stale_wsl_bind_mount"
  elif [[ -n "$recorded_id" && "$recorded_id" != "$current_id" ]]; then
    continuity="blocked"
    issue="cluster_identity_mismatch"
  elif [[ -n "$recorded_type" &&
    ("$recorded_type" != "$declared_type" ||
      "$recorded_fstype" != "$fstype" ||
      "$recorded_source" != "$source") ]]; then
    continuity="blocked"
    issue="mount_identity_mismatch"
  fi

  printf 'status=%s\n' "$continuity"
  printf 'issue=%s\n' "$issue"
  printf 'system_identifier=%s\n' "$current_id"
  printf 'recorded_system_identifier=%s\n' "${recorded_id:-none}"
  printf 'recorded_mount_type=%s\n' "${recorded_type:-none}"
  printf 'recorded_mount_filesystem=%s\n' "${recorded_fstype:-none}"
  printf 'recorded_mount_source=%s\n' "${recorded_source:-none}"
  printf 'declared_mount_type=%s\n' "$declared_type"
  printf 'running_mount_filesystem=%s\n' "$fstype"
  printf 'mount_source=%s\n' "$source"
  printf 'identity_file=%s\n' "$IDENTITY_FILE"

  if [[ "$continuity" != "ready" ]]; then
    if [[ "$REQUIRE_CONTINUITY" == "1" ]]; then
      echo "Guacamole PostgreSQL continuity check failed: $issue" >&2
    fi
    return 1
  fi
}

backup_command() {
  if ! wait_for_postgres; then
    echo "Guacamole PostgreSQL did not become ready for backup." >&2
    exit 1
  fi
  if [[ "$ALLOW_STALE_SOURCE" != "1" ]]; then
    if ! status_command >/dev/null; then
      echo "Refusing backup from a database that failed continuity checks." >&2
      echo "Use --allow-stale-source only for a reviewed one-time migration capture." >&2
      exit 1
    fi
  fi
  mkdir -p "$BACKUP_DIR"
  chmod 700 "$BACKUP_DIR"
  if ! command -v flock >/dev/null 2>&1; then
    echo "flock is required for concurrency-safe backup publication." >&2
    exit 1
  fi
  exec 9>"$BACKUP_DIR/.backup.lock"
  if ! flock --nonblock 9; then
    echo "Another Guacamole PostgreSQL backup is already running." >&2
    exit 1
  fi

  local timestamp prefix temp_dump dump_path temp_manifest manifest_path
  local current_id connections permissions dump_sha dump_size catalog_lines
  timestamp="$(date -u +%Y%m%dT%H%M%S-%NZ)"
  prefix="$BACKUP_DIR/guacamole-postgres-$timestamp"
  temp_dump="$(mktemp "$BACKUP_DIR/.guacamole-postgres.XXXXXX.dump")"
  temp_manifest="$(mktemp "$BACKUP_DIR/.guacamole-postgres.XXXXXX.json")"
  dump_path="$prefix.dump"
  manifest_path="$prefix.json"
  chmod 600 "$temp_dump" "$temp_manifest"

  cleanup_backup_temps() {
    rm -f "$temp_dump" "$temp_manifest"
  }
  trap cleanup_backup_temps EXIT

  docker exec "$POSTGRES_CONTAINER" pg_dump \
    -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
    --format=custom --no-owner --no-privileges >"$temp_dump"
  catalog_lines="$(
    docker exec -i "$POSTGRES_CONTAINER" pg_restore --list <"$temp_dump" |
      wc -l | tr -d ' '
  )"
  if [[ -z "$catalog_lines" || "$catalog_lines" -lt 1 ]]; then
    echo "pg_restore --list returned an empty catalog." >&2
    exit 1
  fi

  current_id="$(system_identifier)"
  connections="$(query_scalar "select count(*) from guacamole_connection;")"
  permissions="$(query_scalar "select count(*) from guacamole_connection_permission;")"
  dump_sha="$(sha256sum "$temp_dump" | awk '{print $1}')"
  dump_size="$(stat -c '%s' "$temp_dump")"

  node -e '
    const fs = require("node:fs");
    const [path, createdAt, systemIdentifier, sha256, size, catalogLines, connections, permissions] =
      process.argv.slice(1);
    const value = {
      schemaVersion: "guacamole-postgres-backup.v1",
      createdAt,
      database: "guacamole_db",
      systemIdentifier,
      sha256,
      sizeBytes: Number(size),
      catalogLines: Number(catalogLines),
      expected: {
        connections: Number(connections),
        connectionPermissions: Number(permissions),
      },
    };
    fs.writeFileSync(path, JSON.stringify(value, null, 2) + "\n", { mode: 0o600 });
  ' "$temp_manifest" "$timestamp" "$current_id" "$dump_sha" "$dump_size" \
    "$catalog_lines" "$connections" "$permissions"

  mv "$temp_dump" "$dump_path"
  mv "$temp_manifest" "$manifest_path"
  trap - EXIT

  mapfile -t published_dumps < <(
    find "$BACKUP_DIR" -maxdepth 1 -type f \
      -name 'guacamole-postgres-*.dump' -printf '%p\n' | sort -r
  )
  local published_dump retained_unprotected=0
  for published_dump in "${published_dumps[@]}"; do
    [[ -n "$published_dump" ]] || continue
    if [[ -r "${published_dump%.dump}.keep" ]]; then
      continue
    fi
    if [[ "$retained_unprotected" -lt "$RETENTION_COUNT" ]]; then
      ((retained_unprotected += 1))
      continue
    fi
    rm -f "$published_dump" "${published_dump%.dump}.json"
  done

  printf 'backup=%s\n' "$dump_path"
  printf 'manifest=%s\n' "$manifest_path"
  printf 'sha256=%s\n' "$dump_sha"
  printf 'catalog_lines=%s\n' "$catalog_lines"
}

latest_backup() {
  local candidate
  while IFS= read -r candidate; do
    candidate="${candidate#* }"
    if [[ -r "${candidate%.dump}.json" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done < <(
    find "$BACKUP_DIR" -maxdepth 1 -type f \
      -name 'guacamole-postgres-*.dump' -printf '%T@ %p\n' | sort -nr
  )
  return 1
}

restore_drill_command() {
  require_postgres
  if [[ -z "$BACKUP_PATH" ]]; then
    BACKUP_PATH="$(latest_backup)"
  fi
  if [[ -z "$BACKUP_PATH" || ! -r "$BACKUP_PATH" ]]; then
    echo "No readable Guacamole PostgreSQL backup was found." >&2
    exit 1
  fi
  local manifest expected_sha actual_sha expected_connections expected_permissions
  manifest="${BACKUP_PATH%.dump}.json"
  if [[ ! -r "$manifest" ]]; then
    echo "Missing backup manifest: $manifest" >&2
    exit 1
  fi
  expected_sha="$(node -e 'const v=require(process.argv[1]); process.stdout.write(v.sha256)' "$manifest")"
  actual_sha="$(sha256sum "$BACKUP_PATH" | awk '{print $1}')"
  if [[ "$actual_sha" != "$expected_sha" ]]; then
    echo "Backup checksum mismatch." >&2
    exit 1
  fi
  docker exec -i "$POSTGRES_CONTAINER" pg_restore --list <"$BACKUP_PATH" >/dev/null

  expected_connections="$(
    node -e 'const v=require(process.argv[1]); process.stdout.write(String(v.expected.connections))' "$manifest"
  )"
  expected_permissions="$(
    node -e 'const v=require(process.argv[1]); process.stdout.write(String(v.expected.connectionPermissions))' "$manifest"
  )"

  local drill_db required_count actual_connections actual_permissions
  drill_db="guacamole_restore_drill_$(date -u +%Y%m%d%H%M%S)_$$"
  cleanup_drill_best_effort() {
    docker exec "$POSTGRES_CONTAINER" dropdb \
      -U "$POSTGRES_USER" --if-exists "$drill_db" >/dev/null 2>&1 || true
  }
  trap cleanup_drill_best_effort EXIT
  docker exec "$POSTGRES_CONTAINER" createdb -U "$POSTGRES_USER" "$drill_db"
  docker exec -i "$POSTGRES_CONTAINER" pg_restore \
    -U "$POSTGRES_USER" -d "$drill_db" \
    --no-owner --no-privileges --exit-on-error <"$BACKUP_PATH"

  required_count="$(
    docker exec "$POSTGRES_CONTAINER" psql \
      -U "$POSTGRES_USER" -d "$drill_db" -v ON_ERROR_STOP=1 -Atc "
select count(*)
from information_schema.tables
where table_schema = 'public'
  and table_name = any(array[
    'guacamole_user',
    'guacamole_entity',
    'guacamole_connection',
    'guacamole_connection_parameter',
    'guacamole_connection_permission'
  ]);"
  )"
  actual_connections="$(
    docker exec "$POSTGRES_CONTAINER" psql \
      -U "$POSTGRES_USER" -d "$drill_db" -v ON_ERROR_STOP=1 \
      -Atc "select count(*) from guacamole_connection;"
  )"
  actual_permissions="$(
    docker exec "$POSTGRES_CONTAINER" psql \
      -U "$POSTGRES_USER" -d "$drill_db" -v ON_ERROR_STOP=1 \
      -Atc "select count(*) from guacamole_connection_permission;"
  )"

  if [[ "$required_count" != "5" ||
    "$actual_connections" != "$expected_connections" ||
    "$actual_permissions" != "$expected_permissions" ]]; then
    echo "Restore drill invariant mismatch." >&2
    exit 1
  fi
  docker exec "$POSTGRES_CONTAINER" dropdb \
    -U "$POSTGRES_USER" --if-exists "$drill_db" >/dev/null
  local residual_drill_count
  residual_drill_count="$(
    docker exec "$POSTGRES_CONTAINER" psql \
      -U "$POSTGRES_USER" -d postgres -v ON_ERROR_STOP=1 -Atc "
select count(*) from pg_database where datname = '$drill_db';"
  )"
  if [[ "$residual_drill_count" != "0" ]]; then
    echo "Restore drill cleanup failed; temporary database remains." >&2
    exit 1
  fi
  trap - EXIT
  printf 'restore_drill=passed\n'
  printf 'backup=%s\n' "$BACKUP_PATH"
  printf 'required_tables=%s\n' "$required_count"
  printf 'connections=%s\n' "$actual_connections"
  printf 'connection_permissions=%s\n' "$actual_permissions"
}

record_identity_command() {
  require_postgres
  local current_id fstype declared_type source temp_identity
  current_id="$(system_identifier)"
  fstype="$(mount_fstype)"
  declared_type="$(inspect_mount_type)"
  source="$(inspect_mount_source)"
  if [[ -r "$IDENTITY_FILE" ]]; then
    if ! status_command >/dev/null; then
      echo "Refusing to overwrite a discontinuous recorded database identity." >&2
      exit 1
    fi
  fi
  if [[ "$fstype" == "tmpfs" && "$declared_type" == "bind" ]]; then
    echo "Refusing to record identity for stale_wsl_bind_mount." >&2
    exit 1
  fi
  mkdir -p "$STATE_DIR"
  chmod 700 "$STATE_DIR"
  temp_identity="$(mktemp "$STATE_DIR/.guacamole-postgres-identity.XXXXXX.json")"
  chmod 600 "$temp_identity"
  node -e '
    const fs = require("node:fs");
    const [path, systemIdentifier, declaredMountType, runningMountFilesystem, mountSource] =
      process.argv.slice(1);
    fs.writeFileSync(path, JSON.stringify({
      schemaVersion: "guacamole-postgres-identity.v1",
      recordedAt: new Date().toISOString(),
      systemIdentifier,
      declaredMountType,
      runningMountFilesystem,
      mountSource,
    }, null, 2) + "\n", { mode: 0o600 });
  ' "$temp_identity" "$current_id" "$declared_type" "$fstype" "$source"
  mv "$temp_identity" "$IDENTITY_FILE"
  printf 'identity=%s\n' "$IDENTITY_FILE"
  printf 'system_identifier=%s\n' "$current_id"
}

case "$COMMAND" in
  status)
    status_command
    ;;
  backup)
    backup_command
    ;;
  restore-drill)
    restore_drill_command
    ;;
  record-identity)
    record_identity_command
    ;;
  *)
    echo "Usage: bash scripts/guacamole-postgres-durability.sh {status|backup|restore-drill|record-identity}" >&2
    exit 2
    ;;
esac
