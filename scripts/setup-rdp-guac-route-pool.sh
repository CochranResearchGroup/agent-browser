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
PRIVILEGED_HELPER="${AGENT_BROWSER_PRIVILEGED_HELPER:-/usr/local/libexec/agent-browser/agent-browser-privileged-helper}"
ROUTE_USER_HELPER="$SCRIPT_DIR/lib/rdp-route-user-pool.py"

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

route_inventory = report.get("routeInventory") or []
for route in route_inventory:
    if ((route.get("target") or {}).get("displayName")):
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

DESIRED_ROUTE_USER_POOL_JSON="$(python3 "$ROUTE_USER_HELPER" resolve \
  --secret-file "$SECRET_FILE" --allow-missing-passwords)"
ROUTE_COUNT="$(python3 -c 'import json,sys; print(len(json.load(sys.stdin)))' \
  <<<"$DESIRED_ROUTE_USER_POOL_JSON")"
ROUTE_SUMMARY="$(python3 -c 'import json,sys; print("\n".join("  {}: {} -> {}".format(r["id"], r["routeUser"], r["connectionName"]) for r in json.load(sys.stdin)))' \
  <<<"$DESIRED_ROUTE_USER_POOL_JSON")"

if [[ "$DRY_RUN" == "1" ]]; then
  cat <<EOF
agent-browser RDP Guacamole route-pool setup dry run

Guacamole compose directory: $GUAC_DIR
Secret file: $SECRET_FILE
RDP target: $HOSTNAME:$PORT
Configured routes: $ROUTE_COUNT
$ROUTE_SUMMARY
Display isolation gate: $DISPLAY_GATE_STATUS
Privileged helper: $PRIVILEGED_HELPER

No users, secrets, Guacamole records, or services were changed.
Install one-time privileges with:
  pnpm install:privileges -- --apply

Then run without --dry-run. If the privileged helper is not installed, run from
an interactive terminal to complete the one-time workstation bootstrap. All
later route maintenance uses the passwordless helper and never prompts.

Important: this host-XRDP-user bootstrap only creates distinct RDP sessions.
P03 is complete only after the many-to-many live gate proves distinct browsers
are actually visible through all configured routes at the same time.
After opening the RDP sessions, run: pnpm inspect:rdp-route-displays
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

REUSE_EXISTING_ROUTE_USERS=0
if ROUTE_USER_POOL_JSON="$DESIRED_ROUTE_USER_POOL_JSON" python3 - <<'PY'
import json
import os
import pwd
import sys

routes = json.loads(os.environ["ROUTE_USER_POOL_JSON"])
for route in routes:
    if not route.get("password"):
        sys.exit(1)
    try:
        pwd.getpwnam(route["routeUser"])
    except KeyError:
        sys.exit(1)
PY
then
  REUSE_EXISTING_ROUTE_USERS=1
fi

if [[ "$REUSE_EXISTING_ROUTE_USERS" != "1" ]]; then
  if ! privileged_helper_available; then
    echo "The passwordless agent-browser helper is required to create or update XRDP route users." >&2
    echo "Complete the one-time workstation bootstrap from an interactive terminal:" >&2
    echo "  agent-browser install workstation --apply" >&2
    exit 2
  fi
fi

setup_user() {
  local user_name="$1"
  local password="$2"

  printf '%s\n' "$password" \
    | sudo -n "$PRIVILEGED_HELPER" ensure-rdp-route-user --user "$user_name"
}

ROUTE_USER_POOL_JSON="$(python3 "$ROUTE_USER_HELPER" resolve \
  --secret-file "$SECRET_FILE" --generate-passwords)"

if [[ "$REUSE_EXISTING_ROUTE_USERS" != "1" ]]; then
  while IFS=$'\t' read -r route_user route_password; do
    setup_user "$route_user" "$route_password"
  done < <(ROUTE_USER_POOL_JSON="$ROUTE_USER_POOL_JSON" python3 - <<'PY'
import json
import os

for route in json.loads(os.environ["ROUTE_USER_POOL_JSON"]):
    print(f'{route["routeUser"]}\t{route["password"]}')
PY
)
fi

ensure_guacamole_postgres

SQL="$(printf '%s\n' "$ROUTE_USER_POOL_JSON" \
  | python3 "$ROUTE_USER_HELPER" sql --hostname "$HOSTNAME" --port "$PORT")"

(
  cd "$GUAC_DIR"
  printf '%s\n' "$SQL" | docker compose exec -T postgres psql -U guacamole_user -d guacamole_db -v ON_ERROR_STOP=1
  docker compose exec -T postgres psql -U guacamole_user -d guacamole_db -v ON_ERROR_STOP=1 -c "CHECKPOINT;" >/dev/null
)

echo "Configured $ROUTE_COUNT Guacamole RDP route-pool users and connections."
if [[ "$REUSE_EXISTING_ROUTE_USERS" == "1" ]]; then
  echo "Reused existing route-specific XRDP users and stored route secrets."
fi
echo "Preserved any live XRDP route desktops; new users and credentials apply at their next login."
echo "Guacamole Postgres route writes checkpoint completed."
echo "Secrets were stored in $SECRET_FILE."
echo "Next: pnpm test:rdp-guac-route-pool-readiness"
echo "After opening the RDP sessions, run: pnpm inspect:rdp-route-displays"
