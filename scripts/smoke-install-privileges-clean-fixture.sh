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
exec /usr/bin/stat "$@"
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

chmod +x "$FAKE_BIN/getent" "$FAKE_BIN/id" "$FAKE_BIN/stat" "$FAKE_BIN/sudo" "$FAKE_BIN/visudo"

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
    bash "$ROOT/scripts/install-agent-browser-privileges.sh" --apply
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

if [[ "$sudo_n_count" != "9" ]]; then
  echo "Expected nine noninteractive privileged commands after authorization, found $sudo_n_count" >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$sudo_install_count" != "3" || "$sudo_groupadd_count" != "1" || "$sudo_usermod_count" != "1" ]]; then
  echo "Unexpected first-apply privileged command shape." >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ ! -x "$HELPER_PATH" || ! -f "$SUDOERS_PATH" ]]; then
  echo "Fixture install did not create helper and sudoers artifacts." >&2
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

if [[ "$sudo_n_count_after" != "11" ]]; then
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
