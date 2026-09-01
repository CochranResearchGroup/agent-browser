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
AUTHORITY_SOURCE="$WORKDIR/source/agent-browser"
AUTHORITY_STATE_ROOT="$WORKDIR/var/lib/agent-browser/lease-authority"
LOG="$WORKDIR/sudo.log"
GROUP_NAME="agent-browser-fixture-$$"
OPERATOR_USER="${USER:-}"

if [[ -z "$OPERATOR_USER" || "$OPERATOR_USER" == "root" ]]; then
  echo "This smoke needs a non-root USER environment value." >&2
  exit 2
fi

mkdir -p "$FAKE_BIN" "$STATE_DIR" "$(dirname "$SUDOERS_PATH")" "$(dirname "$AUTHORITY_SOURCE")"
: >"$LOG"

cat >"$AUTHORITY_SOURCE" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_BOOTSTRAP:-}" != "1" ]]; then
  echo "fixture authority binary accepts bootstrap only" >&2
  exit 2
fi
if [[ -e "$AGENT_BROWSER_FIXTURE_AUTHORITY_STATE_ROOT" ]]; then
  echo "lease_authority_bootstrap_state_exists" >&2
  exit 1
fi
mkdir -p "$AGENT_BROWSER_FIXTURE_AUTHORITY_STATE_ROOT/store/generations"
mkdir -p "$AGENT_BROWSER_FIXTURE_AUTHORITY_STATE_ROOT/trust/generations"
chmod 0700 "$AGENT_BROWSER_FIXTURE_AUTHORITY_STATE_ROOT"
EOF
chmod +x "$AUTHORITY_SOURCE"

cat >"$FAKE_BIN/getent" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  passwd)
    exec /usr/bin/getent "$@"
    ;;
  group)
    group="${2:-}"
    if [[ -n "$group" && -f "$AGENT_BROWSER_FIXTURE_STATE/group-$group" ]]; then
      printf '%s:x:9001:%s\n' "$group" "${AGENT_BROWSER_FIXTURE_OPERATOR_USER:-operator}"
      exit 0
    fi
    exit 2
    ;;
  *)
    exec /usr/bin/getent "$@"
    ;;
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
  user="${2:-${USER:-}}"
  group="${AGENT_BROWSER_FIXTURE_GROUP:-agent-browser-fixture}"
  if [[ -f "$AGENT_BROWSER_FIXTURE_STATE/member-$user-$group" ]]; then
    echo "$user $group"
    exit 0
  fi
fi
exec /usr/bin/id "$@"
EOF

cat >"$FAKE_BIN/visudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-cf" && -f "${2:-}" ]]; then
  exit 0
fi
echo "fake visudo expected -cf <file>" >&2
exit 2
EOF

cat >"$FAKE_BIN/stat" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-c" \
   && "${2:-}" == "%U:%G:%a" \
   && "${3:-}" == "${AGENT_BROWSER_PRIVILEGED_HELPER:-}" \
   && -x "${3:-}" ]]; then
  echo root:root:755
  exit 0
fi
if [[ "${1:-}" == "-c" \
   && "${2:-}" == "%U:%G:%a" \
   && "${3:-}" == "$AGENT_BROWSER_FIXTURE_AUTHORITY_STATE_ROOT" \
   && -d "${3:-}" ]]; then
  echo root:root:700
  exit 0
fi
if [[ "${1:-}" == "-c" \
   && "${2:-}" == "%U:%G:%a" \
   && "${3:-}" == "$AGENT_BROWSER_FIXTURE_ROOT"/usr/local/libexec/agent-browser/lease-authority/generations/*/agent-browser \
   && -x "${3:-}" ]]; then
  echo root:root:755
  exit 0
fi
if [[ "${1:-}" == "-c" \
   && "${2:-}" == "%U:%G:%a" \
   && ( "${3:-}" == "$AGENT_BROWSER_FIXTURE_ROOT/etc/systemd/system/agent-browser-lease-authority.service" \
     || "${3:-}" == "$AGENT_BROWSER_FIXTURE_ROOT/etc/systemd/system/agent-browser-lease-authority.socket" ) \
   && -f "${3:-}" ]]; then
  echo root:root:644
  exit 0
fi
exec /usr/bin/stat "$@"
EOF

cat >"$FAKE_BIN/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  daemon-reload)
    exit 0
    ;;
  stop)
    if [[ "${2:-}" == "agent-browser-lease-authority.service" ]]; then
      exit 0
    fi
    ;;
  enable)
    if [[ "${2:-}" == "--now" && "${3:-}" == "agent-browser-lease-authority.socket" ]]; then
      touch "$AGENT_BROWSER_FIXTURE_STATE/lease-authority-socket-enabled"
      exit 0
    fi
    ;;
  is-enabled|is-active)
    if [[ "${2:-}" == "--quiet" && "${3:-}" == "agent-browser-lease-authority.socket" \
       && -f "$AGENT_BROWSER_FIXTURE_STATE/lease-authority-socket-enabled" ]]; then
      exit 0
    fi
    ;;
esac
exit 1
EOF

cat >"$FAKE_BIN/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'SUDO' >>"$AGENT_BROWSER_FIXTURE_LOG"
for arg in "$@"; do
  printf ' %q' "$arg" >>"$AGENT_BROWSER_FIXTURE_LOG"
done
printf '\n' >>"$AGENT_BROWSER_FIXTURE_LOG"

if [[ "${1:-}" == "-v" ]]; then
  exit 0
fi

if [[ "${1:-}" == "-n" ]]; then
  shift
fi

cmd="${1:-}"
shift || true
case "$cmd" in
  install)
    args=()
    while [[ $# -gt 0 ]]; do
      case "$1" in
        -o|-g)
          shift 2
          ;;
        *)
          args+=("$1")
          shift
          ;;
      esac
    done
    exec /usr/bin/install "${args[@]}"
    ;;
  groupadd)
    group="${*: -1}"
    touch "$AGENT_BROWSER_FIXTURE_STATE/group-$group"
    ;;
  usermod)
    if [[ "${1:-}" == "-aG" ]]; then
      group="${2:-}"
      user="${3:-}"
      touch "$AGENT_BROWSER_FIXTURE_STATE/member-$user-$group"
    fi
    ;;
  visudo)
    exec visudo "$@"
    ;;
  test)
    exit 0
    ;;
  *)
    if [[ "$cmd" == "${AGENT_BROWSER_PRIVILEGED_HELPER:-}" && "${1:-}" == "verify-install" ]]; then
      exit 0
    fi
    AGENT_BROWSER_FAKE_ROOT=1 exec "$cmd" "$@"
    ;;
esac
EOF

chmod +x "$FAKE_BIN/getent" "$FAKE_BIN/id" "$FAKE_BIN/stat" "$FAKE_BIN/sudo" "$FAKE_BIN/systemctl" "$FAKE_BIN/visudo"

run_installer_mode() {
  PATH="$FAKE_BIN:$PATH" \
    AGENT_BROWSER_FIXTURE_LOG="$LOG" \
    AGENT_BROWSER_FIXTURE_STATE="$STATE_DIR" \
    AGENT_BROWSER_FIXTURE_GROUP="$GROUP_NAME" \
    AGENT_BROWSER_FIXTURE_OPERATOR_USER="$OPERATOR_USER" \
    AGENT_BROWSER_FIXTURE_ROOT="$WORKDIR" \
    AGENT_BROWSER_FIXTURE_AUTHORITY_STATE_ROOT="$AUTHORITY_STATE_ROOT" \
    AGENT_BROWSER_PRIVILEGED_GROUP="$GROUP_NAME" \
    AGENT_BROWSER_PRIVILEGED_USER="$OPERATOR_USER" \
    AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE="$ROOT/scripts/libexec/agent-browser-privileged-helper" \
    AGENT_BROWSER_PRIVILEGED_HELPER_DIR="$HELPER_DIR" \
    AGENT_BROWSER_PRIVILEGED_HELPER="$HELPER_PATH" \
    AGENT_BROWSER_PRIVILEGED_SUDOERS="$SUDOERS_PATH" \
    AGENT_BROWSER_INSTALL_PRIVILEGES_FIXTURE_ROOT="$WORKDIR" \
    AGENT_BROWSER_LEASE_AUTHORITY_BINARY_SOURCE="$AUTHORITY_SOURCE" \
    bash "$ROOT/scripts/install-agent-browser-privileges.sh" "$@"
}

run_installer() {
  run_installer_mode --apply
}

run_installer >/tmp/agent-browser-install-privileges-clean-fixture-first.out

sudo_v_count="$(grep -c '^SUDO -v$' "$LOG" || true)"
sudo_n_count="$(grep -c '^SUDO -n ' "$LOG" || true)"
sudo_install_count="$(grep -c '^SUDO -n install ' "$LOG" || true)"
sudo_groupadd_count="$(grep -c '^SUDO -n groupadd ' "$LOG" || true)"
sudo_usermod_count="$(grep -c '^SUDO -n usermod ' "$LOG" || true)"

if [[ "$sudo_v_count" != "1" ]]; then
  echo "Expected exactly one sudo -v during first apply, found $sudo_v_count" >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$sudo_n_count" != "18" ]]; then
  echo "Expected eighteen noninteractive privileged commands after authorization, found $sudo_n_count" >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$sudo_install_count" != "9" || "$sudo_groupadd_count" != "1" || "$sudo_usermod_count" != "1" ]]; then
  echo "Unexpected first-apply privileged command shape." >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ ! -x "$HELPER_PATH" || ! -f "$SUDOERS_PATH" ]]; then
  echo "Fixture install did not create helper and sudoers artifacts." >&2
  exit 1
fi

if [[ ! -d "$AUTHORITY_STATE_ROOT" \
   || ! -f "$WORKDIR/etc/systemd/system/agent-browser-lease-authority.service" \
   || ! -f "$WORKDIR/etc/systemd/system/agent-browser-lease-authority.socket" ]]; then
  echo "Fixture install did not create the protected lease-authority artifacts." >&2
  exit 1
fi

if grep -Eq 'lease-authority|bootstrap|sign|upgrade' "$SUDOERS_PATH"; then
  echo "Lease-authority mutation unexpectedly entered the passwordless sudoers surface." >&2
  exit 1
fi

# A compatible installed helper may differ byte-for-byte from the newly bundled
# helper. Repeat installation must use its bounded runtime contract instead of
# requiring an interactive root-owned file refresh solely for provenance drift.
printf '\n# compatible fixture provenance drift\n' >>"$HELPER_PATH"
helper_sha_before_second_apply="$(sha256sum "$HELPER_PATH" | awk '{print $1}')"

run_installer >/tmp/agent-browser-install-privileges-clean-fixture-second.out

sudo_v_count_after="$(grep -c '^SUDO -v$' "$LOG" || true)"
sudo_n_count_after="$(grep -c '^SUDO -n ' "$LOG" || true)"
sudo_install_count_after="$(grep -c '^SUDO -n install ' "$LOG" || true)"
helper_sha_after_second_apply="$(sha256sum "$HELPER_PATH" | awk '{print $1}')"

if [[ "$sudo_v_count_after" != "1" ]]; then
  echo "Second apply must not add another sudo -v prompt boundary." >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$sudo_n_count_after" != "20" ]]; then
  echo "Second apply should add exactly two non-interactive helper capability checks." >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$(grep -c "^SUDO -n $HELPER_PATH check$" "$LOG" || true)" != "1" \
   || "$(grep -c "^SUDO -n $HELPER_PATH status-json$" "$LOG" || true)" != "1" ]]; then
  echo "Second apply must probe the bounded helper check and status-json contracts." >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$sudo_install_count_after" != "$sudo_install_count" ]]; then
  echo "Second apply unexpectedly repeated privileged install commands." >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$helper_sha_after_second_apply" != "$helper_sha_before_second_apply" ]]; then
  echo "Second apply unexpectedly replaced the compatible installed helper." >&2
  exit 1
fi

# A stopped or disabled socket is a bounded service-lifecycle repair. Existing
# authority state and its banked executable must be retained, and bootstrap
# must never run a second time.
AUTHORITY_SERVICE_UNIT="$WORKDIR/etc/systemd/system/agent-browser-lease-authority.service"
AUTHORITY_BANKED_BINARY="$(sed -n 's/^ExecStart=//p' "$AUTHORITY_SERVICE_UNIT")"
authority_sha_before_recovery="$(sha256sum "$AUTHORITY_BANKED_BINARY" | awk '{print $1}')"
rm -f "$STATE_DIR/lease-authority-socket-enabled"
sudo_v_count_before_socket_recovery="$(grep -c '^SUDO -v$' "$LOG" || true)"
run_installer >/tmp/agent-browser-install-privileges-clean-fixture-socket-recovery.out
sudo_v_count_after_socket_recovery="$(grep -c '^SUDO -v$' "$LOG" || true)"
if [[ "$sudo_v_count_after_socket_recovery" != "$((sudo_v_count_before_socket_recovery + 1))" ]]; then
  echo "Socket recovery must cross exactly one explicit sudo boundary." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(grep -c 'AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_BOOTSTRAP=1' "$LOG" || true)" != "1" ]]; then
  echo "Socket recovery unexpectedly repeated authority bootstrap." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(sha256sum "$AUTHORITY_BANKED_BINARY" | awk '{print $1}')" != "$authority_sha_before_recovery" ]]; then
  echo "Socket recovery unexpectedly replaced the banked authority binary." >&2
  exit 1
fi

# The exact previous ProtectHome=true unit has one guarded migration. It must
# preserve state and the banked binary, stop only the protected service, and
# replace only the service unit with read-only home visibility.
sed -i 's/^ProtectHome=read-only$/ProtectHome=true/' "$AUTHORITY_SERVICE_UNIT"
home_migration_dry_run="$(run_installer_mode --dry-run)"
if ! grep -q 'migrate only the exact legacy ProtectHome=true service unit to ProtectHome=read-only' \
  <<<"$home_migration_dry_run"; then
  echo "Lease-authority home-visibility dry run did not describe the exact migration." >&2
  printf '%s\n' "$home_migration_dry_run" >&2
  exit 1
fi
if grep -q 'initialize absent lease-authority state exactly once' <<<"$home_migration_dry_run"; then
  echo "Lease-authority home-visibility dry run falsely described a fresh bootstrap." >&2
  printf '%s\n' "$home_migration_dry_run" >&2
  exit 1
fi
sudo_v_count_before_home_migration="$(grep -c '^SUDO -v$' "$LOG" || true)"
run_installer >/tmp/agent-browser-install-privileges-clean-fixture-home-migration.out
sudo_v_count_after_home_migration="$(grep -c '^SUDO -v$' "$LOG" || true)"
if [[ "$sudo_v_count_after_home_migration" != "$((sudo_v_count_before_home_migration + 1))" ]]; then
  echo "Lease-authority home-visibility migration must cross one explicit sudo boundary." >&2
  cat "$LOG" >&2
  exit 1
fi
if ! grep -q '^ProtectHome=read-only$' "$AUTHORITY_SERVICE_UNIT"; then
  echo "Lease-authority home-visibility migration did not publish the exact current unit." >&2
  exit 1
fi
if [[ "$(grep -c 'AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_BOOTSTRAP=1' "$LOG" || true)" != "1" ]]; then
  echo "Lease-authority home-visibility migration unexpectedly repeated bootstrap." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(sha256sum "$AUTHORITY_BANKED_BINARY" | awk '{print $1}')" != "$authority_sha_before_recovery" ]]; then
  echo "Lease-authority home-visibility migration unexpectedly replaced the banked binary." >&2
  exit 1
fi

# Modified authority units are not a service-lifecycle repair. With protected
# state present, the installer must fail without overwriting state, units, or
# the banked binary and without invoking bootstrap again.
cp "$AUTHORITY_SERVICE_UNIT" "$AUTHORITY_SERVICE_UNIT.fixture-backup"
printf '\nProtectKernelTunables=false\n' >>"$AUTHORITY_SERVICE_UNIT"
sudo_v_count_before_tamper="$(grep -c '^SUDO -v$' "$LOG" || true)"
if run_installer >/tmp/agent-browser-install-privileges-clean-fixture-tamper.out 2>&1; then
  echo "Tampered authority unit unexpectedly passed installer readiness." >&2
  exit 1
fi
sudo_v_count_after_tamper="$(grep -c '^SUDO -v$' "$LOG" || true)"
if [[ "$sudo_v_count_after_tamper" != "$((sudo_v_count_before_tamper + 1))" ]]; then
  echo "Tampered authority handling must cross exactly one explicit sudo boundary." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(grep -c 'AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_BOOTSTRAP=1' "$LOG" || true)" != "1" ]]; then
  echo "Tampered authority handling unexpectedly repeated bootstrap." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(sha256sum "$AUTHORITY_BANKED_BINARY" | awk '{print $1}')" != "$authority_sha_before_recovery" ]]; then
  echo "Tampered authority handling unexpectedly replaced the banked binary." >&2
  exit 1
fi
mv "$AUTHORITY_SERVICE_UNIT.fixture-backup" "$AUTHORITY_SERVICE_UNIT"

# An installed helper without exact, idempotent route-session termination is
# not compatible with elastic presentation lifecycle. It must be replaced
# before scale-out can create resources that the runtime cannot reclaim.
sed -i \
  's/,"routeSessionTermination":{"supported":true,"exactRouteUser":true,"idempotentWhenAbsent":true}//' \
  "$HELPER_PATH"
if "$HELPER_PATH" status-json | grep -q 'routeSessionTermination'; then
  echo "Stale-helper fixture still advertises route-session termination." >&2
  exit 1
fi

sudo_v_count_before_stale_apply="$(grep -c '^SUDO -v$' "$LOG" || true)"
run_installer >/tmp/agent-browser-install-privileges-clean-fixture-stale.out
sudo_v_count_after_stale_apply="$(grep -c '^SUDO -v$' "$LOG" || true)"

if [[ "$sudo_v_count_after_stale_apply" != "$((sudo_v_count_before_stale_apply + 1))" ]]; then
  echo "Stale helper replacement must cross exactly one sudo -v boundary." >&2
  cat "$LOG" >&2
  exit 1
fi

if ! cmp -s "$ROOT/scripts/libexec/agent-browser-privileged-helper" "$HELPER_PATH"; then
  echo "Stale helper without route-session termination was not replaced." >&2
  exit 1
fi

# A formerly compatible helper that advertises the non-PAM fields below the
# redaction-triggering legacy object name is unsafe. It must cross the
# intentional authorization boundary and be replaced, even when its remaining
# capabilities still pass.
sed -i \
  's/"routeUserCredentialUpdate"/"routeUserPasswordUpdate"/' \
  "$HELPER_PATH"
if "$HELPER_PATH" status-json | grep -q 'routeUserCredentialUpdate'; then
  echo "Legacy-helper fixture still advertises the current credential-update contract." >&2
  exit 1
fi

sudo_v_count_before_legacy_apply="$(grep -c '^SUDO -v$' "$LOG" || true)"
run_installer >/tmp/agent-browser-install-privileges-clean-fixture-legacy.out
sudo_v_count_after_legacy_apply="$(grep -c '^SUDO -v$' "$LOG" || true)"

if [[ "$sudo_v_count_after_legacy_apply" != "$((sudo_v_count_before_legacy_apply + 1))" ]]; then
  echo "Legacy helper replacement must cross exactly one sudo -v boundary." >&2
  cat "$LOG" >&2
  exit 1
fi

if ! cmp -s "$ROOT/scripts/libexec/agent-browser-privileged-helper" "$HELPER_PATH"; then
  echo "Legacy prompt-producing helper was not replaced by the bundled helper." >&2
  exit 1
fi

echo "Install privileges clean-fixture smoke passed"
