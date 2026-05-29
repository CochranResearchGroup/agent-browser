#!/usr/bin/env bash
set -euo pipefail

APPLY=0
for arg in "$@"; do
  case "$arg" in
    --)
      ;;
    --apply)
      APPLY=1
      ;;
    --dry-run)
      APPLY=0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: bash scripts/grant-rdp-route-display-access.sh [--dry-run|--apply]" >&2
      exit 2
      ;;
  esac
done

if ! command -v node >/dev/null 2>&1; then
  echo "node is required" >&2
  exit 1
fi

PRIVILEGED_HELPER="${AGENT_BROWSER_PRIVILEGED_HELPER:-/usr/local/libexec/agent-browser/agent-browser-privileged-helper}"

privileged_helper_available() {
  [[ -x "$PRIVILEGED_HELPER" ]] && sudo -n "$PRIVILEGED_HELPER" check >/dev/null 2>&1
}

USE_PRIVILEGED_HELPER=0
if privileged_helper_available; then
  USE_PRIVILEGED_HELPER=1
elif ! command -v xhost >/dev/null 2>&1; then
  echo "xhost is required when the privileged helper is not installed." >&2
  exit 1
fi

OPERATOR_USER="${AGENT_BROWSER_RDP_DISPLAY_ACCESS_USER:-${SUDO_USER:-${USER:-}}}"
if [[ -z "$OPERATOR_USER" || "$OPERATOR_USER" == "root" ]]; then
  echo "Set AGENT_BROWSER_RDP_DISPLAY_ACCESS_USER to the non-root user that launches agent-browser." >&2
  exit 2
fi

REPORT="$(node scripts/inspect-rdp-route-displays.js 2>/dev/null || true)"
if [[ -z "$REPORT" ]]; then
  echo "Could not inspect route displays. Open both route-specific RDP sessions first." >&2
  exit 1
fi

ROUTES="$(ROUTE_DISPLAY_REPORT="$REPORT" python3 - <<'PY'
import json
import os
import sys

try:
    report = json.loads(os.environ["ROUTE_DISPLAY_REPORT"])
except Exception as exc:
    print(f"failed to parse display report: {exc}", file=sys.stderr)
    sys.exit(1)

routes = report.get("routeSpecificUsers") or {}
for label in ("A", "B"):
    route = routes.get(label) or {}
    user = route.get("user")
    display = route.get("displayName")
    if not user or not display:
        continue
    print(f"{label}\t{user}\t{display}")
PY
)"

if [[ -z "$ROUTES" ]]; then
  echo "No active route-specific displays found. Open routes A and B first, then rerun." >&2
  exit 1
fi

echo "agent-browser RDP route display access"
echo "Operator user: $OPERATOR_USER"
echo "Mode: $([[ "$APPLY" == "1" ]] && echo apply || echo dry-run)"
echo "Privileged helper: $([[ "$USE_PRIVILEGED_HELPER" == "1" ]] && echo "$PRIVILEGED_HELPER" || echo not-installed)"

while IFS=$'\t' read -r label route_user display_name; do
  [[ -n "$label" ]] || continue
  xauthority="/home/$route_user/.Xauthority"
  echo "Route $label: user=$route_user display=$display_name xauthority=$xauthority"
  if [[ "$APPLY" != "1" ]]; then
    if [[ "$USE_PRIVILEGED_HELPER" == "1" ]]; then
      echo "  would run: sudo -n $PRIVILEGED_HELPER grant-display-access --operator-user $OPERATOR_USER --route-user $route_user --display $display_name"
    else
      echo "  would run: sudo -u $route_user env DISPLAY=$display_name XAUTHORITY=$xauthority xhost +SI:localuser:$OPERATOR_USER"
    fi
    continue
  fi
  if [[ "$USE_PRIVILEGED_HELPER" == "1" ]]; then
    sudo -n "$PRIVILEGED_HELPER" grant-display-access \
      --operator-user "$OPERATOR_USER" \
      --route-user "$route_user" \
      --display "$display_name"
  else
    sudo -u "$route_user" env DISPLAY="$display_name" XAUTHORITY="$xauthority" \
      xhost "+SI:localuser:$OPERATOR_USER"
  fi
done <<<"$ROUTES"

if [[ "$APPLY" == "1" ]]; then
  echo "Granted local X access for $OPERATOR_USER on active route displays."
else
  echo "No display access was changed. Rerun with --apply after installing one-time privileges."
fi
