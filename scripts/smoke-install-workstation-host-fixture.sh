#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

FAKE_BIN="$WORKDIR/bin"
STATE="$WORKDIR/state"
HELPER_DIR="$WORKDIR/usr/local/libexec/agent-browser"
HELPER_PATH="$HELPER_DIR/agent-browser-privileged-helper"
SUDOERS_PATH="$WORKDIR/etc/sudoers.d/agent-browser"
AUTHORITY_SOURCE="$WORKDIR/source/agent-browser"
AUTHORITY_STATE_ROOT="$WORKDIR/var/lib/agent-browser/lease-authority"
APPARMOR_PROFILE_PATH="$WORKDIR/etc/apparmor.d/agent-browser-managed-chrome"
APPARMOR_ENABLED_PATH="$WORKDIR/sys/module/apparmor/parameters/enabled"
APPARMOR_RESTRICTION_PATH="$WORKDIR/proc/sys/kernel/apparmor_restrict_unprivileged_userns"
APPARMOR_PROFILES_PATH="$WORKDIR/sys/kernel/security/apparmor/profiles"
LOG="$WORKDIR/sudo.log"
OPERATOR_USER="${USER:-}"
GROUP_NAME="ab-workstation-fixture"

if [[ -z "$OPERATOR_USER" || "$OPERATOR_USER" == "root" ]]; then
  echo "This fixture needs a non-root USER environment value." >&2
  exit 2
fi

mkdir -p \
  "$FAKE_BIN" \
  "$STATE" \
  "$(dirname "$SUDOERS_PATH")" \
  "$(dirname "$APPARMOR_PROFILE_PATH")" \
  "$(dirname "$APPARMOR_ENABLED_PATH")" \
  "$(dirname "$APPARMOR_RESTRICTION_PATH")" \
  "$(dirname "$APPARMOR_PROFILES_PATH")"
mkdir -p "$(dirname "$AUTHORITY_SOURCE")"
: >"$LOG"
printf 'Y\n' >"$APPARMOR_ENABLED_PATH"
printf '1\n' >"$APPARMOR_RESTRICTION_PATH"
: >"$APPARMOR_PROFILES_PATH"

cat >"$AUTHORITY_SOURCE" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_BOOTSTRAP:-}" == "1" ]]
[[ ! -e "$AGENT_BROWSER_FIXTURE_AUTHORITY_STATE_ROOT" ]]
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
    if [[ -f "$AGENT_BROWSER_FIXTURE_STATE/group-$group" ]]; then
      printf '%s:x:9001:%s\n' "$group" "$AGENT_BROWSER_FIXTURE_OPERATOR_USER"
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
  user="${2:-$AGENT_BROWSER_FIXTURE_OPERATOR_USER}"
  groups=("$user")
  for group in "$AGENT_BROWSER_FIXTURE_GROUP" docker; do
    if [[ -f "$AGENT_BROWSER_FIXTURE_STATE/member-$user-$group" ]]; then
      groups+=("$group")
    fi
  done
  printf '%s\n' "${groups[*]}"
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
if [[ "${1:-}" == "-c" \
   && "${2:-}" == "%U:%G:%a" \
   && "${3:-}" == "${AGENT_BROWSER_PRIVILEGED_HELPER:-}" \
   && -x "${3:-}" ]]; then
  echo root:root:755
  exit 0
fi
if [[ "${1:-}" == "-c" \
   && "${2:-}" == "%U:%G:%a" \
   && "${3:-}" == "${AGENT_BROWSER_CHROME_APPARMOR_PROFILE:-}" \
   && -r "${3:-}" ]]; then
  echo root:root:644
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

cat >"$FAKE_BIN/grep" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${*: -1}" == "$AGENT_BROWSER_APPARMOR_PROFILES_PATH" \
   && -f "$AGENT_BROWSER_FIXTURE_STATE/profiles-read-denied" ]]; then
  echo "grep: $AGENT_BROWSER_APPARMOR_PROFILES_PATH: Permission denied" >&2
  exit 2
fi
exec /usr/bin/grep "$@"
EOF

cat >"$FAKE_BIN/apt-get" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  update)
    exit 0
    ;;
  install)
    if [[ " $* " == *" --simulate "* ]]; then
      echo "0 upgraded, 9 newly installed, 0 to remove"
      exit 0
    fi
    touch "$AGENT_BROWSER_FIXTURE_STATE/deps-installed"
    exit 0
    ;;
esac
exit 2
EOF

cat >"$FAKE_BIN/docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "compose" && "${2:-}" == "version" && -f "$AGENT_BROWSER_FIXTURE_STATE/deps-installed" ]]; then
  echo "Docker Compose version fixture"
  exit 0
fi
if [[ "${1:-}" == "info" && -f "$AGENT_BROWSER_FIXTURE_STATE/deps-installed" ]]; then
  exit 0
fi
exit 1
EOF

for command_name in xrdp openbox-session xhost flock systemctl; do
  cat >"$FAKE_BIN/$command_name" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
done

cat >"$FAKE_BIN/apparmor_parser" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'agent-browser-managed-chrome (unconfined)\n' >"$AGENT_BROWSER_APPARMOR_PROFILES_PATH"
EOF

cat >"$FAKE_BIN/ss" <<'EOF'
#!/usr/bin/env bash
echo "LISTEN 0 128 0.0.0.0:3389 0.0.0.0:*"
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
    touch "$AGENT_BROWSER_FIXTURE_STATE/group-${*: -1}"
    ;;
  usermod)
    [[ "${1:-}" == "-aG" ]]
    touch "$AGENT_BROWSER_FIXTURE_STATE/member-${3:-}-${2:-}"
    ;;
  visudo)
    exec visudo "$@"
    ;;
  apt-get)
    exec apt-get "$@"
    ;;
  env)
    exec env "$@"
    ;;
  systemctl)
    exit 0
    ;;
  loginctl)
    exit 0
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

chmod +x "$FAKE_BIN"/*

run_installer() {
  PATH="$FAKE_BIN:$PATH" \
    AGENT_BROWSER_FIXTURE_LOG="$LOG" \
    AGENT_BROWSER_FIXTURE_STATE="$STATE" \
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
    AGENT_BROWSER_CHROME_APPARMOR_PROFILE="$APPARMOR_PROFILE_PATH" \
    AGENT_BROWSER_APPARMOR_ENABLED_PATH="$APPARMOR_ENABLED_PATH" \
    AGENT_BROWSER_APPARMOR_RESTRICTION_PATH="$APPARMOR_RESTRICTION_PATH" \
    AGENT_BROWSER_APPARMOR_PROFILES_PATH="$APPARMOR_PROFILES_PATH" \
    AGENT_BROWSER_INSTALL_PRIVILEGES_FIXTURE_ROOT="$WORKDIR" \
    AGENT_BROWSER_LEASE_AUTHORITY_BINARY_SOURCE="$AUTHORITY_SOURCE" \
    bash "$ROOT/scripts/install-agent-browser-privileges.sh" \
      --apply \
      --with-workstation-deps
}

run_installer >"$WORKDIR/first.out"

if [[ "$(grep -c '^SUDO -v$' "$LOG" || true)" != "1" ]]; then
  echo "Expected exactly one sudo authorization on first apply." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(grep -c '^SUDO -n apt-get update$' "$LOG" || true)" != "1" ]]; then
  echo "Expected one fail-closed apt update." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(grep -c '^SUDO -n apt-get install --simulate ' "$LOG" || true)" != "1" ]]; then
  echo "Expected one dependency simulation." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(grep -c '^SUDO -n env DEBIAN_FRONTEND=noninteractive apt-get install ' "$LOG" || true)" != "1" ]]; then
  echo "Expected one dependency install." >&2
  cat "$LOG" >&2
  exit 1
fi
if ! grep -q ' x11-utils ' "$LOG"; then
  echo "Expected xdpyinfo provider x11-utils in the workstation dependency set." >&2
  cat "$LOG" >&2
  exit 1
fi
for package_name in imagemagick tesseract-ocr; do
  if ! grep -q " $package_name " "$LOG"; then
    echo "Expected viewer proof dependency $package_name in the workstation dependency set." >&2
    cat "$LOG" >&2
    exit 1
  fi
done
if ! grep -q ' apparmor ' "$LOG"; then
  echo "Expected AppArmor in the workstation dependency set." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ ! -f "$APPARMOR_PROFILE_PATH" ]]; then
  echo "Expected the managed Chrome AppArmor profile to be installed." >&2
  exit 1
fi
if ! grep -q '^  userns,$' "$APPARMOR_PROFILE_PATH"; then
  echo "Expected the managed Chrome AppArmor profile to allow user namespaces." >&2
  cat "$APPARMOR_PROFILE_PATH" >&2
  exit 1
fi
if [[ "$(grep -c "^SUDO -n apparmor_parser -r $APPARMOR_PROFILE_PATH$" "$LOG" || true)" != "1" ]]; then
  echo "Expected the managed Chrome AppArmor profile to be loaded once." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ ! -f "$STATE/member-$OPERATOR_USER-docker" || ! -f "$STATE/deps-installed" ]]; then
  echo "Expected Docker membership and installed dependency state." >&2
  exit 1
fi

first_command_count="$(wc -l <"$LOG" | tr -d ' ')"
printf '\n# compatible local policy annotation\n' >>"$APPARMOR_PROFILE_PATH"
touch "$STATE/profiles-read-denied"
run_installer >"$WORKDIR/second.out"
second_command_count="$(wc -l <"$LOG" | tr -d ' ')"

if [[ "$(grep -c '^SUDO -v$' "$LOG" || true)" != "1" ]]; then
  echo "Idempotent rerun added another sudo authorization." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$second_command_count" != "$((first_command_count + 3))" ]]; then
  echo "Idempotent rerun should add only bounded noninteractive helper checks." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(tail -n 3 "$LOG" | head -n 1)" != "SUDO -n $HELPER_PATH check" \
   || "$(tail -n 3 "$LOG" | tail -n 2 | head -n 1)" != "SUDO -n $HELPER_PATH status-json" \
   || "$(tail -n 1 "$LOG")" != "SUDO -n $HELPER_PATH verify-install --group $GROUP_NAME --sudoers $SUDOERS_PATH --sha256 $(sha256sum "$HELPER_PATH" | awk '{print $1}') --apparmor-profile-name agent-browser-managed-chrome" ]]; then
  echo "Unexpected idempotent rerun command." >&2
  tail -n 4 "$LOG" >&2
  exit 1
fi

rm "$STATE/profiles-read-denied"
printf 'N\n' >"$APPARMOR_ENABLED_PATH"
rm "$APPARMOR_PROFILE_PATH"
: >"$APPARMOR_PROFILES_PATH"
run_installer >"$WORKDIR/wsl-like.out"
third_command_count="$(wc -l <"$LOG" | tr -d ' ')"

if [[ "$(grep -c '^SUDO -v$' "$LOG" || true)" != "1" ]]; then
  echo "A WSL-like AppArmor-disabled rerun added another sudo authorization." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$third_command_count" != "$((second_command_count + 2))" ]]; then
  echo "An AppArmor-disabled rerun should add only helper capability checks." >&2
  cat "$LOG" >&2
  exit 1
fi
echo "Workstation host-provision fixture passed"
