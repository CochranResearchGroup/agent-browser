#!/usr/bin/env bash
set -euo pipefail

APPLY=0
GROUP_NAME="${AGENT_BROWSER_PRIVILEGED_GROUP:-agent-browser}"
OPERATOR_USER="${AGENT_BROWSER_PRIVILEGED_USER:-${SUDO_USER:-${USER:-}}}"
HELPER_SOURCE="${AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE:-scripts/libexec/agent-browser-privileged-helper}"
HELPER_DIR="${AGENT_BROWSER_PRIVILEGED_HELPER_DIR:-/usr/local/libexec/agent-browser}"
HELPER_PATH="${AGENT_BROWSER_PRIVILEGED_HELPER:-$HELPER_DIR/agent-browser-privileged-helper}"
SUDOERS_PATH="${AGENT_BROWSER_PRIVILEGED_SUDOERS:-/etc/sudoers.d/agent-browser}"

usage() {
  cat <<'EOF'
Usage: bash scripts/install-agent-browser-privileges.sh [--dry-run|--apply]

Installs the narrow root-owned helper used by agent-browser RDP/Guacamole setup.
The helper is protected by a sudoers rule for the agent-browser group so later
route-user and display-access maintenance can run without repeated prompts.
EOF
}

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
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$OPERATOR_USER" || "$OPERATOR_USER" == "root" ]]; then
  echo "Set AGENT_BROWSER_PRIVILEGED_USER to the non-root user that runs agent-browser." >&2
  exit 2
fi

if [[ ! "$OPERATOR_USER" =~ ^[a-z_][a-z0-9_-]{0,31}$ ]]; then
  echo "Operator user must be a local username." >&2
  exit 2
fi

if ! getent passwd "$OPERATOR_USER" >/dev/null; then
  echo "Operator user does not exist: $OPERATOR_USER" >&2
  exit 2
fi

if [[ ! "$GROUP_NAME" =~ ^[a-z_][a-z0-9_-]{0,31}$ ]]; then
  echo "Privileged group must be a local group name." >&2
  exit 2
fi

if [[ "$HELPER_PATH" != /* ]]; then
  echo "Installed helper path must be absolute." >&2
  exit 2
fi

if [[ ! -f "$HELPER_SOURCE" ]]; then
  echo "Missing helper source: $HELPER_SOURCE" >&2
  exit 1
fi

expected_sudoers_content() {
  cat <<EOF
# agent-browser narrow privileged helper
%$GROUP_NAME ALL=(root) NOPASSWD: $HELPER_PATH
EOF
}

current_install_ready() {
  getent group "$GROUP_NAME" >/dev/null 2>&1 || return 1
  id -nG "$OPERATOR_USER" 2>/dev/null | tr ' ' '\n' | grep -Fx "$GROUP_NAME" >/dev/null || return 1
  [[ -x "$HELPER_PATH" ]] || return 1
  cmp -s "$HELPER_SOURCE" "$HELPER_PATH" || return 1
  [[ -f "$SUDOERS_PATH" ]] || return 1
  if [[ -r "$SUDOERS_PATH" ]]; then
    expected_sudoers_content | diff -q - "$SUDOERS_PATH" >/dev/null 2>&1 || return 1
  fi
  sudo -n "$HELPER_PATH" check >/dev/null 2>&1 || return 1
}

print_install_status() {
  echo "Current readiness:"
  if getent group "$GROUP_NAME" >/dev/null 2>&1; then
    echo "  group: ready"
  else
    echo "  group: missing"
  fi

  if id -nG "$OPERATOR_USER" 2>/dev/null | tr ' ' '\n' | grep -Fx "$GROUP_NAME" >/dev/null; then
    echo "  membership: ready"
  else
    echo "  membership: $OPERATOR_USER is not in $GROUP_NAME"
  fi

  if [[ -x "$HELPER_PATH" ]]; then
    if cmp -s "$HELPER_SOURCE" "$HELPER_PATH"; then
      echo "  helper: ready"
    else
      echo "  helper: installed helper differs from bundled helper and must be refreshed"
    fi
  elif [[ -e "$HELPER_PATH" ]]; then
    echo "  helper: present but not executable"
  else
    echo "  helper: missing"
  fi

  if [[ -f "$SUDOERS_PATH" ]]; then
    if [[ -r "$SUDOERS_PATH" ]] && expected_sudoers_content | diff -q - "$SUDOERS_PATH" >/dev/null 2>&1; then
      echo "  sudoers: ready"
    elif [[ -r "$SUDOERS_PATH" ]]; then
      echo "  sudoers: policy differs from expected rule"
    else
      echo "  sudoers: present but not readable by current user"
    fi
  else
    echo "  sudoers: missing"
  fi

  if [[ -x "$HELPER_PATH" ]] && sudo -n "$HELPER_PATH" check >/dev/null 2>&1; then
    echo "  sudo helper check: ready"
  else
    echo "  sudo helper check: not ready"
  fi
}

if [[ "$APPLY" != "1" ]]; then
  cat <<EOF
agent-browser privileged helper install dry run

Group: $GROUP_NAME
Operator user: $OPERATOR_USER
Helper source: $HELPER_SOURCE
Installed helper: $HELPER_PATH
Sudoers file: $SUDOERS_PATH

Would run with one privileged authorization:
  sudo install -d -o root -g root -m 0755 $HELPER_DIR
  sudo install -o root -g root -m 0755 $HELPER_SOURCE $HELPER_PATH
  sudo groupadd --force $GROUP_NAME
  sudo usermod -aG $GROUP_NAME $OPERATOR_USER
  sudo install validated sudoers policy at $SUDOERS_PATH

After applying, open a new shell or run: newgrp $GROUP_NAME
EOF
  print_install_status
  exit 0
fi

if current_install_ready; then
  echo "agent-browser privileged helper is already ready."
  echo "No privileged changes were needed."
  exit 0
fi

if ! command -v visudo >/dev/null 2>&1; then
  echo "visudo is required to validate the sudoers policy." >&2
  exit 1
fi

print_install_status
sudo -v

SUDOERS_TMP="$(mktemp)"
trap 'rm -f "$SUDOERS_TMP"' EXIT
expected_sudoers_content >"$SUDOERS_TMP"

sudo visudo -cf "$SUDOERS_TMP" >/dev/null
sudo install -d -o root -g root -m 0755 "$HELPER_DIR"
sudo install -o root -g root -m 0755 "$HELPER_SOURCE" "$HELPER_PATH"
sudo groupadd --force "$GROUP_NAME"
sudo usermod -aG "$GROUP_NAME" "$OPERATOR_USER"
sudo install -o root -g root -m 0440 "$SUDOERS_TMP" "$SUDOERS_PATH"
sudo visudo -cf "$SUDOERS_PATH" >/dev/null

echo "Installed agent-browser privileged helper at $HELPER_PATH."
echo "Added $OPERATOR_USER to group $GROUP_NAME."
echo "Installed sudoers policy at $SUDOERS_PATH."
echo "Open a new shell or run: newgrp $GROUP_NAME"
