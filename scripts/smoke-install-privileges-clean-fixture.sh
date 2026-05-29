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
  *)
    AGENT_BROWSER_FAKE_ROOT=1 exec "$cmd" "$@"
    ;;
esac
EOF

chmod +x "$FAKE_BIN/getent" "$FAKE_BIN/id" "$FAKE_BIN/sudo" "$FAKE_BIN/visudo"

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
sudo_install_count="$(grep -c '^SUDO install ' "$LOG" || true)"
sudo_groupadd_count="$(grep -c '^SUDO groupadd ' "$LOG" || true)"
sudo_usermod_count="$(grep -c '^SUDO usermod ' "$LOG" || true)"

if [[ "$sudo_v_count" != "1" ]]; then
  echo "Expected exactly one sudo -v during first apply, found $sudo_v_count" >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$sudo_n_count" != "0" ]]; then
  echo "Expected no sudo -n readiness check before first apply, found $sudo_n_count" >&2
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

run_installer >/tmp/agent-browser-install-privileges-clean-fixture-second.out

sudo_v_count_after="$(grep -c '^SUDO -v$' "$LOG" || true)"
sudo_n_count_after="$(grep -c '^SUDO -n ' "$LOG" || true)"
sudo_install_count_after="$(grep -c '^SUDO install ' "$LOG" || true)"

if [[ "$sudo_v_count_after" != "1" ]]; then
  echo "Second apply must not add another sudo -v prompt boundary." >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$sudo_n_count_after" != "1" ]]; then
  echo "Second apply should use exactly one non-interactive helper readiness check." >&2
  cat "$LOG" >&2
  exit 1
fi

if [[ "$sudo_install_count_after" != "$sudo_install_count" ]]; then
  echo "Second apply unexpectedly repeated privileged install commands." >&2
  cat "$LOG" >&2
  exit 1
fi

echo "Install privileges clean-fixture smoke passed"
