#!/usr/bin/env bash
set -euo pipefail

APPLY=0
WITH_WORKSTATION_DEPS=0
GROUP_NAME="${AGENT_BROWSER_PRIVILEGED_GROUP:-agent-browser}"
OPERATOR_USER="${AGENT_BROWSER_PRIVILEGED_USER:-${SUDO_USER:-${USER:-}}}"
HELPER_SOURCE="${AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE:-scripts/libexec/agent-browser-privileged-helper}"
HELPER_DIR="${AGENT_BROWSER_PRIVILEGED_HELPER_DIR:-/usr/local/libexec/agent-browser}"
HELPER_PATH="${AGENT_BROWSER_PRIVILEGED_HELPER:-$HELPER_DIR/agent-browser-privileged-helper}"
EXPECTED_HELPER_SHA256="${AGENT_BROWSER_PRIVILEGED_HELPER_SHA256:-}"
SUDOERS_PATH="${AGENT_BROWSER_PRIVILEGED_SUDOERS:-/etc/sudoers.d/agent-browser}"
INSTALL_FIXTURE_ROOT="${AGENT_BROWSER_INSTALL_PRIVILEGES_FIXTURE_ROOT:-}"
LEASE_AUTHORITY_BINARY_SOURCE="${AGENT_BROWSER_LEASE_AUTHORITY_BINARY_SOURCE:-}"
LEASE_AUTHORITY_ROOT="$INSTALL_FIXTURE_ROOT/usr/local/libexec/agent-browser/lease-authority"
LEASE_AUTHORITY_GENERATIONS_ROOT="$LEASE_AUTHORITY_ROOT/generations"
LEASE_AUTHORITY_STATE_PARENT="$INSTALL_FIXTURE_ROOT/var/lib/agent-browser"
LEASE_AUTHORITY_STATE_ROOT="$LEASE_AUTHORITY_STATE_PARENT/lease-authority"
LEASE_AUTHORITY_SERVICE_UNIT="$INSTALL_FIXTURE_ROOT/etc/systemd/system/agent-browser-lease-authority.service"
LEASE_AUTHORITY_SOCKET_UNIT="$INSTALL_FIXTURE_ROOT/etc/systemd/system/agent-browser-lease-authority.socket"
LEASE_AUTHORITY_SYSTEMD_UNIT_DIR="$(dirname "$LEASE_AUTHORITY_SERVICE_UNIT")"
LEASE_AUTHORITY_SOCKET_PATH="$INSTALL_FIXTURE_ROOT/run/agent-browser/lease-authority.sock"
APPARMOR_PROFILE_PATH="${AGENT_BROWSER_CHROME_APPARMOR_PROFILE:-/etc/apparmor.d/agent-browser-managed-chrome}"
APPARMOR_PROFILE_NAME="agent-browser-managed-chrome"
APPARMOR_ENABLED_PATH="${AGENT_BROWSER_APPARMOR_ENABLED_PATH:-/sys/module/apparmor/parameters/enabled}"
APPARMOR_RESTRICTION_PATH="${AGENT_BROWSER_APPARMOR_RESTRICTION_PATH:-/proc/sys/kernel/apparmor_restrict_unprivileged_userns}"
APPARMOR_PROFILES_PATH="${AGENT_BROWSER_APPARMOR_PROFILES_PATH:-/sys/kernel/security/apparmor/profiles}"
APPARMOR_TMP=""
SUDOERS_TMP=""
LEASE_AUTHORITY_SERVICE_TMP=""
LEASE_AUTHORITY_SOCKET_TMP=""

cleanup_temp_files() {
  [[ -z "$APPARMOR_TMP" ]] || rm -f "$APPARMOR_TMP"
  [[ -z "$SUDOERS_TMP" ]] || rm -f "$SUDOERS_TMP"
  [[ -z "$LEASE_AUTHORITY_SERVICE_TMP" ]] || rm -f "$LEASE_AUTHORITY_SERVICE_TMP"
  [[ -z "$LEASE_AUTHORITY_SOCKET_TMP" ]] || rm -f "$LEASE_AUTHORITY_SOCKET_TMP"
}
trap cleanup_temp_files EXIT

usage() {
  cat <<'EOF'
Usage: bash scripts/install-agent-browser-privileges.sh [--dry-run|--apply] [--with-workstation-deps]

Installs the narrow root-owned helper and protected lease-authority service.
The helper is protected by a sudoers rule for the agent-browser group so later
route-user and display-access maintenance can run without repeated prompts.
The optional workstation dependency phase is Ubuntu 24.04 amd64 only and
installs Docker, Compose, XRDP, XorgXRDP, Openbox, and required host tools.
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
    --with-workstation-deps)
      WITH_WORKSTATION_DEPS=1
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

if [[ -z "$LEASE_AUTHORITY_BINARY_SOURCE" ]]; then
  if [[ -r "$LEASE_AUTHORITY_SERVICE_UNIT" ]]; then
    INSTALLED_LEASE_AUTHORITY_BINARY="$(sed -n 's/^ExecStart=//p' "$LEASE_AUTHORITY_SERVICE_UNIT")"
    if [[ -x "$INSTALLED_LEASE_AUTHORITY_BINARY" ]]; then
      LEASE_AUTHORITY_BINARY_SOURCE="$INSTALLED_LEASE_AUTHORITY_BINARY"
    fi
  fi
  if [[ -z "$LEASE_AUTHORITY_BINARY_SOURCE" && -x "cli/target/release/agent-browser" ]]; then
    LEASE_AUTHORITY_BINARY_SOURCE="cli/target/release/agent-browser"
  elif [[ -z "$LEASE_AUTHORITY_BINARY_SOURCE" ]] && command -v agent-browser >/dev/null 2>&1; then
    LEASE_AUTHORITY_BINARY_SOURCE="$(command -v agent-browser)"
  fi
fi
if [[ -z "$LEASE_AUTHORITY_BINARY_SOURCE" || ! -x "$LEASE_AUTHORITY_BINARY_SOURCE" ]]; then
  echo "Set AGENT_BROWSER_LEASE_AUTHORITY_BINARY_SOURCE to an executable reviewed agent-browser binary." >&2
  exit 1
fi
LEASE_AUTHORITY_BINARY_SHA256="$(sha256sum "$LEASE_AUTHORITY_BINARY_SOURCE" | awk '{print $1}')"
if [[ ! "$LEASE_AUTHORITY_BINARY_SHA256" =~ ^[a-f0-9]{64}$ ]]; then
  echo "Lease-authority binary SHA-256 is invalid." >&2
  exit 1
fi
LEASE_AUTHORITY_GENERATION="sha256-$LEASE_AUTHORITY_BINARY_SHA256"
LEASE_AUTHORITY_BANKED_BINARY="$LEASE_AUTHORITY_GENERATIONS_ROOT/$LEASE_AUTHORITY_GENERATION/agent-browser"
if [[ -z "$EXPECTED_HELPER_SHA256" ]]; then
  EXPECTED_HELPER_SHA256="$(sha256sum "$HELPER_SOURCE" | awk '{print $1}')"
fi
if [[ ! "$EXPECTED_HELPER_SHA256" =~ ^[a-f0-9]{64}$ ]]; then
  echo "Expected helper SHA-256 must be 64 lowercase hexadecimal characters." >&2
  exit 2
fi
if [[ "$(sha256sum "$HELPER_SOURCE" | awk '{print $1}')" != "$EXPECTED_HELPER_SHA256" ]]; then
  echo "Helper source SHA-256 does not match the embedded installer manifest." >&2
  exit 1
fi

expected_sudoers_content() {
  cat <<EOF
# agent-browser narrow privileged helper
%$GROUP_NAME ALL=(root) NOPASSWD: $HELPER_PATH
EOF
}

lease_authority_service_unit_content() {
  local banked_binary="${1:-$LEASE_AUTHORITY_BANKED_BINARY}"
  cat <<EOF
[Unit]
Description=Agent Browser protected lease authority
Requires=agent-browser-lease-authority.socket
After=agent-browser-lease-authority.socket

[Service]
Type=simple
ExecStart=$banked_binary
Environment=AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_SERVICE=1
User=root
Group=root
NoNewPrivileges=true
CapabilityBoundingSet=
DevicePolicy=closed
IPAddressDeny=any
LockPersonality=true
MemoryDenyWriteExecute=true
MemoryMax=256M
PrivateDevices=true
PrivateTmp=true
ProtectHome=true
ProtectControlGroups=true
ProtectKernelModules=true
ProtectKernelTunables=true
ProtectSystem=strict
ReadWritePaths=$LEASE_AUTHORITY_STATE_ROOT
RestrictAddressFamilies=AF_UNIX
RestrictSUIDSGID=true
SystemCallArchitectures=native
TasksMax=16
UMask=0077

[Install]
WantedBy=multi-user.target
EOF
}

lease_authority_socket_unit_content() {
  cat <<EOF
[Unit]
Description=Agent Browser protected lease authority socket

[Socket]
ListenStream=$LEASE_AUTHORITY_SOCKET_PATH
SocketUser=root
SocketGroup=$GROUP_NAME
SocketMode=0660
RemoveOnStop=true

[Install]
WantedBy=sockets.target
EOF
}

lease_authority_artifacts_ready() {
  [[ -r "$LEASE_AUTHORITY_SERVICE_UNIT" && -r "$LEASE_AUTHORITY_SOCKET_UNIT" ]] || return 1
  local installed_binary
  installed_binary="$(sed -n 's/^ExecStart=//p' "$LEASE_AUTHORITY_SERVICE_UNIT")"
  local installed_generation installed_sha256
  [[ "$(dirname "$(dirname "$installed_binary")")" == "$LEASE_AUTHORITY_GENERATIONS_ROOT" ]] || return 1
  [[ "$(basename "$installed_binary")" == "agent-browser" ]] || return 1
  installed_generation="$(basename "$(dirname "$installed_binary")")"
  [[ "$installed_generation" =~ ^sha256-[a-f0-9]{64}$ ]] || return 1
  [[ -x "$installed_binary" && ! -L "$installed_binary" ]] || return 1
  installed_sha256="$(sha256sum "$installed_binary" 2>/dev/null | awk '{print $1}')"
  [[ "sha256-$installed_sha256" == "$installed_generation" ]] || return 1
  [[ "$(stat -c '%U:%G:%a' "$installed_binary" 2>/dev/null)" == "root:root:755" ]] || return 1
  [[ "$(stat -c '%U:%G:%a' "$LEASE_AUTHORITY_STATE_ROOT" 2>/dev/null)" == "root:root:700" ]] || return 1
  [[ "$(stat -c '%U:%G:%a' "$LEASE_AUTHORITY_SERVICE_UNIT" 2>/dev/null)" == "root:root:644" ]] || return 1
  [[ "$(stat -c '%U:%G:%a' "$LEASE_AUTHORITY_SOCKET_UNIT" 2>/dev/null)" == "root:root:644" ]] || return 1
  lease_authority_service_unit_content "$installed_binary" | diff -q - "$LEASE_AUTHORITY_SERVICE_UNIT" >/dev/null 2>&1 || return 1
  lease_authority_socket_unit_content | diff -q - "$LEASE_AUTHORITY_SOCKET_UNIT" >/dev/null 2>&1 || return 1
}

lease_authority_contract_ready() {
  lease_authority_artifacts_ready || return 1
  systemctl is-enabled --quiet agent-browser-lease-authority.socket || return 1
  systemctl is-active --quiet agent-browser-lease-authority.socket || return 1
}

operator_home() {
  getent passwd "$OPERATOR_USER" | awk -F: '{print $6}'
}

apparmor_profile_content() {
  local home_dir
  home_dir="$(operator_home)"
  local chrome_path="$home_dir/.agent-browser/browsers/**/chrome"
  chrome_path="${chrome_path//\\/\\\\}"
  chrome_path="${chrome_path//\"/\\\"}"
  cat <<EOF
abi <abi/4.0>,
include <tunables/global>

profile $APPARMOR_PROFILE_NAME "$chrome_path" flags=(unconfined) {
  userns,

  include if exists <local/$APPARMOR_PROFILE_NAME>
}
EOF
}

# The managed Chrome policy is only needed on kernels that both enable
# AppArmor and restrict unprivileged user namespaces. WSL kernels commonly
# expose AppArmor as disabled, where installing or starting the service cannot
# make the policy effective and must not turn an otherwise-ready rerun into a
# new sudo prompt.
apparmor_policy_required() {
  [[ -r "$APPARMOR_ENABLED_PATH" ]] || return 1
  [[ -r "$APPARMOR_RESTRICTION_PATH" ]] || return 1
  [[ "$(tr -d '[:space:]' <"$APPARMOR_ENABLED_PATH")" =~ ^[Yy]$ ]] || return 1
  [[ "$(tr -d '[:space:]' <"$APPARMOR_RESTRICTION_PATH")" == "1" ]]
}

apparmor_profile_ready() {
  apparmor_policy_required || return 0
  command -v apparmor_parser >/dev/null 2>&1 || return 1
  systemctl is-active --quiet apparmor || return 1
  [[ -r "$APPARMOR_PROFILE_PATH" ]] || return 1
  [[ "$(stat -c '%U:%G:%a' "$APPARMOR_PROFILE_PATH" 2>/dev/null)" == "root:root:644" ]] || return 1

  local home_dir chrome_path profile_header
  home_dir="$(operator_home)"
  chrome_path="$home_dir/.agent-browser/browsers/**/chrome"
  profile_header="profile $APPARMOR_PROFILE_NAME \"$chrome_path\" flags=(unconfined) {"
  grep -Fqx "$profile_header" "$APPARMOR_PROFILE_PATH" || return 1
  grep -Eq '^[[:space:]]*userns,[[:space:]]*$' "$APPARMOR_PROFILE_PATH" || return 1
  if [[ -r "$APPARMOR_PROFILES_PATH" ]] \
    && grep -Fqx "$APPARMOR_PROFILE_NAME (unconfined)" "$APPARMOR_PROFILES_PATH" 2>/dev/null; then
    return 0
  fi

  # Ubuntu protects the loaded-profile registry from unprivileged readers.
  # Reuse the already-authorized narrow helper to verify that root-only fact
  # without turning an idempotent rerun into another interactive sudo prompt.
  [[ -x "$HELPER_PATH" ]] || return 1
  local installed_helper_sha256
  installed_helper_sha256="$(sha256sum "$HELPER_PATH" | awk '{print $1}')"
  [[ "$installed_helper_sha256" =~ ^[a-f0-9]{64}$ ]] || return 1
  sudo -n "$HELPER_PATH" verify-install \
    --group "$GROUP_NAME" \
    --sudoers "$SUDOERS_PATH" \
    --sha256 "$installed_helper_sha256" \
    --apparmor-profile-name "$APPARMOR_PROFILE_NAME" >/dev/null 2>&1
}

workstation_packages() {
  printf '%s\n' \
    apparmor \
    docker.io \
    docker-compose-v2 \
    xrdp \
    xorgxrdp \
    openbox \
    x11-utils \
    x11-xserver-utils \
    imagemagick \
    tesseract-ocr \
    curl \
    python3 \
    nodejs \
    util-linux \
    ca-certificates \
    ssl-cert \
    freerdp2-x11 \
    xvfb \
    xauth \
    dbus-x11 \
    iproute2 \
    libxcb-shm0 \
    libx11-xcb1 \
    libx11-6 \
    libxcb1 \
    libxext6 \
    libxrandr2 \
    libxcomposite1 \
    libxcursor1 \
    libxdamage1 \
    libxfixes3 \
    libxi6 \
    libgtk-3-0t64 \
    libpangocairo-1.0-0 \
    libpango-1.0-0 \
    libatk1.0-0t64 \
    libcairo-gobject2 \
    libcairo2 \
    libgdk-pixbuf-2.0-0 \
    libxrender1 \
    libasound2t64 \
    libfreetype6 \
    libfontconfig1 \
    libdbus-1-3 \
    libnss3 \
    libnspr4 \
    libatk-bridge2.0-0t64 \
    libdrm2 \
    libxkbcommon0 \
    libatspi2.0-0t64 \
    libcups2t64 \
    libxshmfence1 \
    libgbm1 \
    fonts-noto-color-emoji \
    fonts-noto-cjk \
    fonts-freefont-ttf
}

workstation_deps_ready() {
  [[ "$(uname -m)" == "x86_64" ]] || return 1
  command -v apt-get >/dev/null 2>&1 || return 1
  command -v docker >/dev/null 2>&1 || return 1
  docker compose version >/dev/null 2>&1 || return 1
  command -v xrdp >/dev/null 2>&1 || [[ -x /usr/sbin/xrdp ]] || return 1
  command -v openbox-session >/dev/null 2>&1 || return 1
  command -v xhost >/dev/null 2>&1 || return 1
  command -v flock >/dev/null 2>&1 || return 1
  apparmor_profile_ready || return 1
  getent group docker >/dev/null 2>&1 || return 1
  id -nG "$OPERATOR_USER" 2>/dev/null | tr ' ' '\n' | grep -Fx docker >/dev/null || return 1
}

current_install_ready() {
  getent group "$GROUP_NAME" >/dev/null 2>&1 || return 1
  id -nG "$OPERATOR_USER" 2>/dev/null | tr ' ' '\n' | grep -Fx "$GROUP_NAME" >/dev/null || return 1
  helper_contract_ready || return 1
  lease_authority_contract_ready || return 1
  if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
    workstation_deps_ready || return 1
  fi
}

helper_contract_ready() {
  [[ -x "$HELPER_PATH" ]] || return 1
  [[ "$(stat -c '%U:%G:%a' "$HELPER_PATH" 2>/dev/null)" == "root:root:755" ]] || return 1
  sudo -n "$HELPER_PATH" check >/dev/null 2>&1 || return 1

  local helper_status compact_status required
  helper_status="$(sudo -n "$HELPER_PATH" status-json 2>/dev/null)" || return 1
  compact_status="$(printf '%s' "$helper_status" | tr -d '[:space:]')"
  for required in \
    '"schemaVersion":1' \
    '"state":"browser_control_ready_template"' \
    '"startsWindowManager":true' \
    '"keepsSessionAlive":true' \
    '"routeSessionTermination":{' \
    '"supported":true' \
    '"exactRouteUser":true' \
    '"idempotentWhenAbsent":true' \
    '"supportsFilesystemX11Socket":true' \
    '"supportsAbstractX11Socket":true' \
    '"boundedXhostTimeoutSeconds":2' \
    '"routeUserCredentialUpdate":{' \
    '"pamBypassed":true' \
    '"cryptMethod":"SHA512"' \
    '"shaRounds":100000'; do
    [[ "$compact_status" == *"$required"* ]] || return 1
  done
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
      echo "  helper provenance: bundled helper matches installed helper"
    elif helper_contract_ready; then
      echo "  helper provenance: bundled helper differs; compatible installed helper will be retained"
    else
      echo "  helper provenance: installed helper differs and lacks the required runtime contract"
    fi
  elif [[ -e "$HELPER_PATH" ]]; then
    echo "  helper provenance: present but not executable"
  else
    echo "  helper provenance: missing"
  fi

  if [[ -r "$SUDOERS_PATH" ]] && expected_sudoers_content | diff -q - "$SUDOERS_PATH" >/dev/null 2>&1; then
    echo "  sudoers: ready"
  elif [[ -e "$SUDOERS_PATH" ]]; then
    echo "  sudoers: protected; helper verification required"
  else
    echo "  sudoers: protected or missing; helper verification required"
  fi

  if helper_contract_ready; then
    echo "  passwordless helper contract: ready"
  else
    echo "  passwordless helper contract: not ready"
  fi

  if lease_authority_contract_ready; then
    echo "  protected lease authority: ready"
  else
    echo "  protected lease authority: not ready"
  fi

  if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
    if workstation_deps_ready; then
      echo "  workstation dependencies: ready"
    else
      echo "  workstation dependencies: missing or operator docker membership is stale"
    fi
    if apparmor_policy_required; then
      if apparmor_profile_ready; then
        echo "  managed Chrome AppArmor policy: ready"
      else
        echo "  managed Chrome AppArmor policy: required but not ready"
      fi
    else
      echo "  managed Chrome AppArmor policy: not required by this kernel"
    fi
  fi
}

if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
  if [[ "$(uname -m)" != "x86_64" ]]; then
    echo "Workstation installation currently supports Linux x86_64 only." >&2
    exit 1
  fi
  if [[ ! -r /etc/os-release ]]; then
    echo "Unable to verify Ubuntu release from /etc/os-release." >&2
    exit 1
  fi
  # shellcheck disable=SC1091
  . /etc/os-release
  if [[ "${ID:-}" != "ubuntu" || "${VERSION_ID:-}" != "24.04" ]]; then
    echo "Workstation installation currently supports Ubuntu 24.04 only." >&2
    exit 1
  fi
  if ! command -v apt-get >/dev/null 2>&1 || ! command -v apt-cache >/dev/null 2>&1; then
    echo "apt-get and apt-cache are required for workstation dependency installation." >&2
    exit 1
  fi
fi

if [[ "$APPLY" != "1" ]]; then
  cat <<EOF
agent-browser privileged helper install dry run

Group: $GROUP_NAME
Operator user: $OPERATOR_USER
Helper source: $HELPER_SOURCE
Installed helper: $HELPER_PATH
Sudoers file: $SUDOERS_PATH
Lease-authority source: $LEASE_AUTHORITY_BINARY_SOURCE
Lease-authority generation: $LEASE_AUTHORITY_GENERATION
Lease-authority state: $LEASE_AUTHORITY_STATE_ROOT

Would run with one privileged authorization:
  sudo install -d -o root -g root -m 0755 $HELPER_DIR
  sudo install -o root -g root -m 0755 $HELPER_SOURCE $HELPER_PATH
  sudo groupadd --force $GROUP_NAME
  sudo usermod -aG $GROUP_NAME $OPERATOR_USER
  sudo install validated sudoers policy at $SUDOERS_PATH
  sudo install immutable lease-authority binary at $LEASE_AUTHORITY_BANKED_BINARY
  sudo install fixed systemd units at $LEASE_AUTHORITY_SERVICE_UNIT and $LEASE_AUTHORITY_SOCKET_UNIT
  sudo initialize absent lease-authority state exactly once
  sudo systemctl enable --now agent-browser-lease-authority.socket
EOF
  if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
    echo "  sudo apt-get update"
    echo "  sudo apt-get install after a no-removal simulation:"
    workstation_packages | sed 's/^/    /'
    echo "  sudo usermod -aG docker $OPERATOR_USER"
    if apparmor_policy_required; then
      echo "  sudo install and load managed Chrome AppArmor policy at $APPARMOR_PROFILE_PATH"
    else
      echo "  managed Chrome AppArmor policy is not required by this kernel"
    fi
    echo "  sudo systemctl enable --now docker xrdp"
  fi
  cat <<EOF

After applying, log out and back in or reboot so group membership is active.
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

if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
  mapfile -t WORKSTATION_PACKAGES < <(workstation_packages)
  sudo -n apt-get update
  for package_name in "${WORKSTATION_PACKAGES[@]}"; do
    if ! apt-cache show "$package_name" >/dev/null 2>&1; then
      echo "Required workstation package has no apt candidate after updating indexes: $package_name" >&2
      exit 1
    fi
  done
  SIMULATION_OUTPUT="$(sudo -n apt-get install --simulate "${WORKSTATION_PACKAGES[@]}" 2>&1)" || {
    printf '%s\n' "$SIMULATION_OUTPUT" >&2
    echo "Workstation dependency simulation failed." >&2
    exit 1
  }
  if grep -q '^Remv ' <<<"$SIMULATION_OUTPUT"; then
    printf '%s\n' "$SIMULATION_OUTPUT" >&2
    echo "Workstation dependency installation would remove packages." >&2
    exit 1
  fi
  sudo -n env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends --no-remove \
    "${WORKSTATION_PACKAGES[@]}"
  if apparmor_policy_required; then
    APPARMOR_TMP="$(mktemp)"
    apparmor_profile_content >"$APPARMOR_TMP"
    sudo -n install -o root -g root -m 0644 "$APPARMOR_TMP" "$APPARMOR_PROFILE_PATH"
    sudo -n apparmor_parser -r "$APPARMOR_PROFILE_PATH"
    rm -f "$APPARMOR_TMP"
    APPARMOR_TMP=""
  fi
  sudo -n groupadd --force docker
  sudo -n usermod -aG docker "$OPERATOR_USER"
  sudo -n loginctl enable-linger "$OPERATOR_USER"
  if apparmor_policy_required; then
    sudo -n systemctl enable --now apparmor docker xrdp xrdp-sesman
  else
    sudo -n systemctl enable --now docker xrdp xrdp-sesman
  fi
  sudo -n docker info >/dev/null
  sudo -n docker compose version >/dev/null
  if apparmor_policy_required; then
    sudo -n systemctl is-active --quiet apparmor docker xrdp xrdp-sesman
  else
    sudo -n systemctl is-active --quiet docker xrdp xrdp-sesman
  fi
  sudo -n ss -ltn | grep -Eq '(^|[[:space:]])[^[:space:]]*:3389[[:space:]]'
fi

SUDOERS_TMP="$(mktemp)"
expected_sudoers_content >"$SUDOERS_TMP"
LEASE_AUTHORITY_SERVICE_TMP="$(mktemp)"
lease_authority_service_unit_content >"$LEASE_AUTHORITY_SERVICE_TMP"
LEASE_AUTHORITY_SOCKET_TMP="$(mktemp)"
lease_authority_socket_unit_content >"$LEASE_AUTHORITY_SOCKET_TMP"

sudo -n visudo -cf "$SUDOERS_TMP" >/dev/null
sudo -n install -d -o root -g root -m 0755 "$HELPER_DIR"
sudo -n install -o root -g root -m 0755 "$HELPER_SOURCE" "$HELPER_PATH"
sudo -n groupadd --force "$GROUP_NAME"
sudo -n usermod -aG "$GROUP_NAME" "$OPERATOR_USER"

if ! lease_authority_contract_ready; then
  if [[ -e "$LEASE_AUTHORITY_STATE_ROOT" ]]; then
    if ! lease_authority_artifacts_ready; then
      echo "Existing lease-authority state has untrusted installation artifacts and will not be overwritten." >&2
      exit 1
    fi
    sudo -n systemctl daemon-reload
    sudo -n systemctl enable --now agent-browser-lease-authority.socket
    lease_authority_contract_ready || {
      echo "Protected lease-authority socket did not recover to exact readiness." >&2
      exit 1
    }
  else
    sudo -n install -d -o root -g root -m 0755 "$LEASE_AUTHORITY_GENERATIONS_ROOT/$LEASE_AUTHORITY_GENERATION"
    sudo -n install -o root -g root -m 0755 "$LEASE_AUTHORITY_BINARY_SOURCE" "$LEASE_AUTHORITY_BANKED_BINARY"
    sudo -n install -d -o root -g root -m 0755 "$LEASE_AUTHORITY_STATE_PARENT"
    sudo -n install -d -o root -g root -m 0755 "$LEASE_AUTHORITY_SYSTEMD_UNIT_DIR"
    sudo -n install -o root -g root -m 0644 "$LEASE_AUTHORITY_SERVICE_TMP" "$LEASE_AUTHORITY_SERVICE_UNIT"
    sudo -n install -o root -g root -m 0644 "$LEASE_AUTHORITY_SOCKET_TMP" "$LEASE_AUTHORITY_SOCKET_UNIT"
    OPERATOR_GROUP_ID="$(getent group "$GROUP_NAME" | awk -F: '{print $3}')"
    if [[ ! "$OPERATOR_GROUP_ID" =~ ^[1-9][0-9]*$ ]]; then
      echo "Unable to resolve the protected lease-authority operator group id." >&2
      exit 1
    fi
    sudo -n env \
      AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_BOOTSTRAP=1 \
      AGENT_BROWSER_INTERNAL_LEASE_AUTHORITY_OPERATOR_GROUP_ID="$OPERATOR_GROUP_ID" \
      "$LEASE_AUTHORITY_BANKED_BINARY"
    sudo -n systemctl daemon-reload
    sudo -n systemctl enable --now agent-browser-lease-authority.socket
    lease_authority_contract_ready || {
      echo "Protected lease-authority installation did not pass exact readiness verification." >&2
      exit 1
    }
  fi
fi

sudo -n install -o root -g root -m 0440 "$SUDOERS_TMP" "$SUDOERS_PATH"
sudo -n visudo -cf "$SUDOERS_PATH" >/dev/null
sudo -n test "$(stat -c '%U:%G:%a' "$HELPER_PATH")" = "root:root:755"
INSTALLED_HELPER_SHA256="$(sudo -n sha256sum "$HELPER_PATH" | awk '{print $1}')"
if [[ "$INSTALLED_HELPER_SHA256" != "$EXPECTED_HELPER_SHA256" ]]; then
  echo "Installed helper SHA-256 does not match the embedded installer manifest." >&2
  exit 1
fi

echo "Installed agent-browser privileged helper at $HELPER_PATH."
echo "Added $OPERATOR_USER to group $GROUP_NAME."
echo "Installed sudoers policy at $SUDOERS_PATH."
if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
  echo "Added $OPERATOR_USER to group docker."
fi
echo "Log out and back in or reboot so group membership is active."
