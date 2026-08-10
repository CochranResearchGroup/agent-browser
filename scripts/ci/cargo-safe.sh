#!/usr/bin/env bash
set -euo pipefail

if [[ $# -eq 0 ]]; then
  echo "Usage: scripts/ci/cargo-safe.sh <cargo arguments...>" >&2
  exit 2
fi

runtime_dir="${XDG_RUNTIME_DIR:-/tmp}"
lock_file="${AGENT_BROWSER_CARGO_LOCK_FILE:-${runtime_dir}/agent-browser-cargo-${UID}.lock}"
build_jobs="${AGENT_BROWSER_CARGO_BUILD_JOBS:-4}"

exec 9>"$lock_file"
if ! flock -n 9; then
  echo "Waiting for the serialized Agent Browser Cargo build lock: $lock_file" >&2
  flock 9
fi

kernel_release="$(uname -r 2>/dev/null || true)"
if [[ "${AGENT_BROWSER_CARGO_FORCE_WSL:-0}" == "1" || "$kernel_release" == *microsoft* || "$kernel_release" == *Microsoft* ]]; then
  memory_high="${AGENT_BROWSER_CARGO_MEMORY_HIGH:-20G}"
  memory_max="${AGENT_BROWSER_CARGO_MEMORY_MAX:-24G}"
  swap_max="${AGENT_BROWSER_CARGO_SWAP_MAX:-4G}"
  tasks_max="${AGENT_BROWSER_CARGO_TASKS_MAX:-512}"

  if ! command -v systemd-run >/dev/null 2>&1 || ! systemctl --user show-environment >/dev/null 2>&1; then
    echo "Refusing to run uncapped Cargo on WSL: the user systemd manager is unavailable." >&2
    echo "Restore user systemd, or set AGENT_BROWSER_CARGO_ALLOW_UNCAPPED=1 for an explicit one-off override." >&2
    if [[ "${AGENT_BROWSER_CARGO_ALLOW_UNCAPPED:-0}" != "1" ]]; then
      exit 78
    fi
    exec env CARGO_BUILD_JOBS="$build_jobs" cargo "$@"
  fi

  echo "Running serialized Cargo in a WSL cgroup: jobs=$build_jobs MemoryHigh=$memory_high MemoryMax=$memory_max MemorySwapMax=$swap_max" >&2
  exec systemd-run \
    --user \
    --scope \
    --quiet \
    --property="MemoryHigh=$memory_high" \
    --property="MemoryMax=$memory_max" \
    --property="MemorySwapMax=$swap_max" \
    --property="TasksMax=$tasks_max" \
    env CARGO_BUILD_JOBS="$build_jobs" cargo "$@"
fi

exec env CARGO_BUILD_JOBS="$build_jobs" cargo "$@"
