#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

FAKE_BIN="$WORKDIR/bin"
STATE_DIR="$WORKDIR/state"
HELPER_DIR="$WORKDIR/usr/local/libexec/agent-browser"
HELPER_PATH="$HELPER_DIR/agent-browser-privileged-helper"
SUDOERS_PATH="$WORKDIR/etc/sudoers.d/agent-browser"
LEGACY_ROOT="$HELPER_DIR/lease-authority"
LEGACY_STATE="$WORKDIR/var/lib/agent-browser/lease-authority"
LEGACY_SERVICE="$WORKDIR/etc/systemd/system/agent-browser-lease-authority.service"
LEGACY_SOCKET_UNIT="$WORKDIR/etc/systemd/system/agent-browser-lease-authority.socket"
LEGACY_SOCKET="$WORKDIR/run/agent-browser/lease-authority.sock"
ARCHIVE_ROOT="$HELPER_DIR/retired-lease-authority"
ARCHIVE_STATE="$WORKDIR/var/lib/agent-browser/retired-lease-authority"
ARCHIVE_UNITS="$WORKDIR/var/lib/agent-browser/retired-lease-authority-units"
LOG="$WORKDIR/sudo.log"
GROUP_NAME="agent-browser-fixture-$$"
OPERATOR_USER="${USER:-}"

if [[ -z "$OPERATOR_USER" || "$OPERATOR_USER" == "root" ]]; then
  echo "This smoke needs a non-root USER environment value." >&2
  exit 2
fi

mkdir -p "$FAKE_BIN" "$STATE_DIR" "$(dirname "$SUDOERS_PATH")"
: >"$LOG"

cat >"$FAKE_BIN/getent" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  passwd) exec /usr/bin/getent "$@" ;;
  group)
    group="${2:-}"
    [[ -f "$AGENT_BROWSER_FIXTURE_STATE/group-$group" ]] || exit 2
    printf '%s:x:9001:%s\n' "$group" "$AGENT_BROWSER_FIXTURE_OPERATOR_USER"
    ;;
  *) exec /usr/bin/getent "$@" ;;
esac
EOF

cat >"$FAKE_BIN/id" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-u" && "${AGENT_BROWSER_FAKE_ROOT:-0}" == "1" ]]; then
  echo 0
  exit 0
fi
if [[ "${1:-}" == "-nG" ]]; then
  user="${2:-$AGENT_BROWSER_FIXTURE_OPERATOR_USER}"
  if [[ -f "$AGENT_BROWSER_FIXTURE_STATE/member-$user-$AGENT_BROWSER_FIXTURE_GROUP" ]]; then
    echo "$user $AGENT_BROWSER_FIXTURE_GROUP"
  else
    echo "$user"
  fi
  exit 0
fi
exec /usr/bin/id "$@"
EOF

cat >"$FAKE_BIN/visudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "-cf" && -f "${2:-}" ]]
EOF

cat >"$FAKE_BIN/stat" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-c" && "${2:-}" == "%U:%G:%a" ]]; then
  if [[ "${3:-}" == "${AGENT_BROWSER_PRIVILEGED_HELPER:-}" && -x "${3:-}" ]]; then
    echo root:root:755
    exit 0
  fi
  if [[ "${3:-}" == "${AGENT_BROWSER_PRIVILEGED_SUDOERS:-}" && -f "${3:-}" ]]; then
    echo root:root:440
    exit 0
  fi
fi
exec /usr/bin/stat "$@"
EOF

cat >"$FAKE_BIN/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  is-active)
    unit="${3:-${2:-}}"
    [[ -f "$AGENT_BROWSER_FIXTURE_STATE/active-$unit" ]]
    ;;
  is-enabled)
    unit="${3:-${2:-}}"
    [[ -f "$AGENT_BROWSER_FIXTURE_STATE/enabled-$unit" ]]
    ;;
  disable)
    unit="${3:-${2:-}}"
    rm -f "$AGENT_BROWSER_FIXTURE_STATE/active-$unit" "$AGENT_BROWSER_FIXTURE_STATE/enabled-$unit"
    if [[ "$unit" == "agent-browser-lease-authority.socket" ]]; then
      rm -f "$AGENT_BROWSER_INSTALL_PRIVILEGES_FIXTURE_ROOT/run/agent-browser/lease-authority.sock"
    fi
    ;;
  stop)
    rm -f "$AGENT_BROWSER_FIXTURE_STATE/active-${2:-}"
    ;;
  daemon-reload) ;;
  *) exit 1 ;;
esac
EOF

cat >"$FAKE_BIN/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'SUDO' >>"$AGENT_BROWSER_FIXTURE_LOG"
for arg in "$@"; do printf ' %q' "$arg" >>"$AGENT_BROWSER_FIXTURE_LOG"; done
printf '\n' >>"$AGENT_BROWSER_FIXTURE_LOG"
if [[ "${1:-}" == "-v" ]]; then exit 0; fi
if [[ "${1:-}" == "-n" ]]; then shift; fi
cmd="${1:-}"
shift || true
case "$cmd" in
  install)
    args=()
    while [[ $# -gt 0 ]]; do
      case "$1" in -o|-g) shift 2 ;; *) args+=("$1"); shift ;; esac
    done
    exec /usr/bin/install "${args[@]}"
    ;;
  groupadd) touch "$AGENT_BROWSER_FIXTURE_STATE/group-${*: -1}" ;;
  usermod)
    [[ "${1:-}" == "-aG" ]]
    touch "$AGENT_BROWSER_FIXTURE_STATE/member-${3:-}-${2:-}"
    ;;
  visudo) exec visudo "$@" ;;
  systemctl) exec systemctl "$@" ;;
  mv) exec /usr/bin/mv "$@" ;;
  chmod) exec /usr/bin/chmod "$@" ;;
  test) exit 0 ;;
  sha256sum) exec /usr/bin/sha256sum "$@" ;;
  *)
    if [[ "$cmd" == "${AGENT_BROWSER_PRIVILEGED_HELPER:-}" && "${1:-}" == "verify-install" ]]; then
      exit 0
    fi
    AGENT_BROWSER_FAKE_ROOT=1 exec "$cmd" "$@"
    ;;
esac
EOF

chmod +x "$FAKE_BIN"/*

run_installer() {
  PATH="$FAKE_BIN:$PATH" \
    AGENT_BROWSER_FIXTURE_LOG="$LOG" \
    AGENT_BROWSER_FIXTURE_STATE="$STATE_DIR" \
    AGENT_BROWSER_FIXTURE_GROUP="$GROUP_NAME" \
    AGENT_BROWSER_FIXTURE_OPERATOR_USER="$OPERATOR_USER" \
    AGENT_BROWSER_PRIVILEGED_GROUP="$GROUP_NAME" \
    AGENT_BROWSER_PRIVILEGED_USER="$OPERATOR_USER" \
    AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE="$ROOT/scripts/libexec/agent-browser-privileged-helper" \
    AGENT_BROWSER_PRIVILEGED_HELPER_DIR="$HELPER_DIR" \
    AGENT_BROWSER_PRIVILEGED_HELPER="$HELPER_PATH" \
    AGENT_BROWSER_PRIVILEGED_SUDOERS="$SUDOERS_PATH" \
    AGENT_BROWSER_INSTALL_PRIVILEGES_FIXTURE_ROOT="$WORKDIR" \
    bash "$ROOT/scripts/install-agent-browser-privileges.sh" \
      --sealed-plan-digest "$(printf 'b%.0s' {1..64})" \
      --sealed-plan-actions retire_legacy_lease_authority,ensure_privileged_helper \
      "$@"
}

run_installer --apply >"$WORKDIR/first.out"
grep -q '"schemaVersion":"agent-browser.privileged-host-effect-receipt.v3"' "$WORKDIR/first.out"
grep -q '"legacyLeaseAuthorityRetired":true' "$WORKDIR/first.out"
[[ ! -e "$LEGACY_ROOT" && ! -e "$LEGACY_SERVICE" && ! -e "$LEGACY_SOCKET_UNIT" ]]
[[ "$(grep -c '^SUDO -v$' "$LOG" || true)" == "1" ]]

: >"$LOG"
run_installer --apply >"$WORKDIR/healthy.out"
grep -q 'No privileged calls or changes were needed.' "$WORKDIR/healthy.out"
grep -q '"outcome":"already_ready"' "$WORKDIR/healthy.out"
[[ ! -s "$LOG" ]]

# Compatible byte-only helper drift stays on the zero-sudo path.
printf '\n# compatible fixture drift\n' >>"$HELPER_PATH"
compatible_sha="$(sha256sum "$HELPER_PATH" | awk '{print $1}')"
: >"$LOG"
run_installer --apply >"$WORKDIR/compatible.out"
[[ ! -s "$LOG" ]]
[[ "$(sha256sum "$HELPER_PATH" | awk '{print $1}')" == "$compatible_sha" ]]

# Missing runtime capabilities must refresh the helper through one explicit
# authorization rather than taking the compatible healthy-rerun path.
for capability in routeUserOwnedProvisioning routeSessionTermination routeUserCredentialUpdate; do
  sed -i "s/$capability/${capability}Legacy/" "$HELPER_PATH"
  if "$HELPER_PATH" status-json | grep -q "\"$capability\""; then
    echo "Stale-helper fixture still advertises $capability." >&2
    exit 1
  fi
  : >"$LOG"
  run_installer --apply >"$WORKDIR/stale-$capability.out"
  [[ "$(grep -c '^SUDO -v$' "$LOG" || true)" == "1" ]]
  cmp -s "$ROOT/scripts/libexec/agent-browser-privileged-helper" "$HELPER_PATH"
done

mkdir -p "$LEGACY_ROOT/generations/sha256-fixture" "$LEGACY_STATE/store" \
  "$(dirname "$LEGACY_SERVICE")" "$(dirname "$LEGACY_SOCKET")"
printf 'legacy-binary\n' >"$LEGACY_ROOT/generations/sha256-fixture/agent-browser"
printf '[Service]\nExecStart=%s\n' "$LEGACY_ROOT/generations/sha256-fixture/agent-browser" >"$LEGACY_SERVICE"
printf '[Socket]\nListenStream=%s\n' "$LEGACY_SOCKET" >"$LEGACY_SOCKET_UNIT"
printf 'socket\n' >"$LEGACY_SOCKET"
touch "$STATE_DIR/active-agent-browser-lease-authority.service"
touch "$STATE_DIR/active-agent-browser-lease-authority.socket"
touch "$STATE_DIR/enabled-agent-browser-lease-authority.socket"
: >"$LOG"

run_installer --apply >"$WORKDIR/retire.out"
grep -q '"legacyLeaseAuthorityRetired":true' "$WORKDIR/retire.out"
[[ -d "$ARCHIVE_ROOT" && -d "$ARCHIVE_STATE" ]]
[[ -f "$ARCHIVE_UNITS/agent-browser-lease-authority.service" ]]
[[ -f "$ARCHIVE_UNITS/agent-browser-lease-authority.socket" ]]
[[ ! -e "$LEGACY_ROOT" && ! -e "$LEGACY_STATE" && ! -e "$LEGACY_SOCKET" ]]
[[ ! -e "$LEGACY_SERVICE" && ! -e "$LEGACY_SOCKET_UNIT" ]]
grep -q '^SUDO -n systemctl disable --now agent-browser-lease-authority.socket$' "$LOG"
grep -q '^SUDO -n systemctl stop agent-browser-lease-authority.service$' "$LOG"

mkdir -p "$LEGACY_ROOT"
: >"$LOG"
if run_installer --apply >"$WORKDIR/conflict.out" 2>"$WORKDIR/conflict.err"; then
  echo "Conflicting live and archived authority roots must fail closed." >&2
  exit 1
fi
grep -q 'Conflicting live and archived legacy lease-authority artifacts require manual review.' "$WORKDIR/conflict.err"
[[ ! -s "$LOG" ]]

rm -rf "$LEGACY_ROOT"
ln -s /tmp "$LEGACY_ROOT"
: >"$LOG"
if run_installer --apply >"$WORKDIR/symlink.out" 2>"$WORKDIR/symlink.err"; then
  echo "Symbolic-link legacy authority paths must fail closed." >&2
  exit 1
fi
grep -q 'retirement refuses symbolic-link paths' "$WORKDIR/symlink.err"
[[ ! -s "$LOG" ]]
rm -f "$LEGACY_ROOT"

if run_installer --apply --upgrade-lease-authority >"$WORKDIR/obsolete.out" 2>"$WORKDIR/obsolete.err"; then
  echo "Removed lease-authority upgrade flag must be rejected." >&2
  exit 1
fi
grep -q 'Unknown argument: --upgrade-lease-authority' "$WORKDIR/obsolete.err"

echo "Privileged helper clean-install and legacy-authority retirement fixture passed"
