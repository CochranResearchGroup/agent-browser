#!/usr/bin/env bash
set -euo pipefail

if [[ $# -eq 0 ]]; then
  echo "Usage: scripts/ci/cargo-safe.sh <cargo arguments...>" >&2
  exit 2
fi

runtime_dir="${XDG_RUNTIME_DIR:-/tmp}"
admission_dir="${AGENT_BROWSER_CARGO_ADMISSION_DIR:-${runtime_dir}/agent-browser-cargo-${UID}}"
claims_dir="${admission_dir}/claims"
admission_lock="${admission_dir}/admission.lock"
build_jobs="${AGENT_BROWSER_CARGO_BUILD_JOBS:-8}"
max_concurrent="${AGENT_BROWSER_CARGO_MAX_CONCURRENT:-2}"
memory_reserve_kib="${AGENT_BROWSER_CARGO_MEMORY_RESERVE_KIB:-16777216}"
memory_claim_kib="${AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB:-14680064}"
minimum_swap_free_kib="${AGENT_BROWSER_CARGO_MINIMUM_SWAP_FREE_KIB:-2097152}"
minimum_disk_free_kib="${AGENT_BROWSER_CARGO_MINIMUM_DISK_FREE_KIB:-20971520}"
poll_seconds="${AGENT_BROWSER_CARGO_ADMISSION_POLL_SECONDS:-2}"
meminfo_file="${AGENT_BROWSER_CARGO_MEMINFO_FILE:-/proc/meminfo}"
probe_only="${AGENT_BROWSER_CARGO_CAPACITY_PROBE_ONLY:-0}"
probe_hold_seconds="${AGENT_BROWSER_CARGO_CAPACITY_HOLD_SECONDS:-0}"
no_wait="${AGENT_BROWSER_CARGO_ADMISSION_NO_WAIT:-0}"
cargo_cache_mode="${AGENT_BROWSER_CARGO_CACHE:-auto}"
fast_linker_mode="${AGENT_BROWSER_CARGO_FAST_LINKER:-auto}"

for numeric in "$build_jobs" "$max_concurrent" "$memory_reserve_kib" "$memory_claim_kib" "$minimum_swap_free_kib" "$minimum_disk_free_kib"; do
  if [[ ! "$numeric" =~ ^[0-9]+$ ]]; then
    echo "Agent Browser Cargo capacity settings must be non-negative integers" >&2
    exit 2
  fi
done
if (( build_jobs == 0 || max_concurrent == 0 || memory_claim_kib == 0 )); then
  echo "Cargo jobs, maximum concurrency, and memory claim must be greater than zero" >&2
  exit 2
fi

case "$cargo_cache_mode" in
  auto|off|required) ;;
  *)
    echo "AGENT_BROWSER_CARGO_CACHE must be auto, off, or required" >&2
    exit 2
    ;;
esac
case "$fast_linker_mode" in
  auto|off|required) ;;
  *)
    echo "AGENT_BROWSER_CARGO_FAST_LINKER must be auto, off, or required" >&2
    exit 2
    ;;
esac

cargo_cache="none"
cargo_cache_path=""
if [[ "$cargo_cache_mode" != "off" ]]; then
  cargo_cache_path="$(command -v sccache 2>/dev/null || true)"
  if [[ -n "$cargo_cache_path" ]]; then
    cargo_cache="sccache"
  elif [[ "$cargo_cache_mode" == "required" ]]; then
    echo "Cargo acceleration unavailable: sccache is required but was not found" >&2
    exit 78
  fi
fi

native_linux_target=0
if [[ "$(uname -s 2>/dev/null || true)" == "Linux" ]]; then
  native_linux_target=1
  next_is_target=0
  for cargo_argument in "$@"; do
    if (( next_is_target == 1 )); then
      [[ "$cargo_argument" == *linux* ]] || native_linux_target=0
      next_is_target=0
      continue
    fi
    case "$cargo_argument" in
      --target) next_is_target=1 ;;
      --target=*)
        target_triple="${cargo_argument#--target=}"
        [[ "$target_triple" == *linux* ]] || native_linux_target=0
        ;;
    esac
  done
fi

fast_linker="none"
if [[ "$fast_linker_mode" != "off" && "$native_linux_target" == "1" ]]; then
  if command -v mold >/dev/null 2>&1; then
    fast_linker="mold"
  elif command -v ld.lld >/dev/null 2>&1; then
    fast_linker="lld"
  elif [[ "$fast_linker_mode" == "required" ]]; then
    echo "Cargo acceleration unavailable: mold or lld is required but neither was found" >&2
    exit 78
  fi
elif [[ "$fast_linker_mode" == "required" ]]; then
  echo "Cargo acceleration unavailable: a fast linker is required for a non-native Linux target" >&2
  exit 78
fi

cargo_environment=(env "CARGO_BUILD_JOBS=$build_jobs")
if [[ "$cargo_cache" == "sccache" ]]; then
  cargo_environment+=("RUSTC_WRAPPER=$cargo_cache_path")
fi
if [[ "$fast_linker" != "none" ]]; then
  linker_flags="-C link-arg=-fuse-ld=$fast_linker"
  if [[ -n "${RUSTFLAGS:-}" ]]; then
    linker_flags="${RUSTFLAGS} ${linker_flags}"
  fi
  cargo_environment+=("RUSTFLAGS=$linker_flags")
fi

mkdir -p "$claims_dir"

process_start_token() {
  local process_id="$1"
  awk '{print $22}' "/proc/${process_id}/stat" 2>/dev/null || true
}

claim_is_live() {
  local claim_path_candidate="$1"
  local claim_pid=""
  local claim_start=""
  while IFS='=' read -r key value; do
    case "$key" in
      pid) claim_pid="$value" ;;
      start) claim_start="$value" ;;
    esac
  done < "$claim_path_candidate"
  [[ "$claim_pid" =~ ^[0-9]+$ ]] || return 1
  [[ -n "$claim_start" ]] || return 1
  [[ "$(process_start_token "$claim_pid")" == "$claim_start" ]]
}

reconcile_claims() {
  local claim_path_candidate
  shopt -s nullglob
  for claim_path_candidate in "$claims_dir"/*.claim; do
    if ! claim_is_live "$claim_path_candidate"; then
      rm -f "$claim_path_candidate"
    fi
  done
  shopt -u nullglob
}

read_meminfo_kib() {
  local field="$1"
  awk -v field="${field}:" '$1 == field {print $2; found=1; exit} END {if (!found) print 0}' "$meminfo_file"
}

active_claim_count() {
  local count=0
  local claim_path_candidate
  shopt -s nullglob
  for claim_path_candidate in "$claims_dir"/*.claim; do
    count=$((count + 1))
  done
  shopt -u nullglob
  echo "$count"
}

disk_available_kib() {
  if [[ -n "${AGENT_BROWSER_CARGO_DISK_AVAILABLE_KIB:-}" ]]; then
    echo "$AGENT_BROWSER_CARGO_DISK_AVAILABLE_KIB"
    return
  fi
  df -Pk "$PWD" | awk 'NR == 2 {print $4}'
}

cpu_count() {
  if [[ -n "${AGENT_BROWSER_CARGO_CPU_COUNT:-}" ]]; then
    echo "$AGENT_BROWSER_CARGO_CPU_COUNT"
    return
  fi
  nproc
}

claim_path=""
release_claim() {
  if [[ -n "$claim_path" ]]; then
    rm -f "$claim_path"
  fi
}
trap release_claim EXIT INT TERM

last_reason=""
while [[ -z "$claim_path" ]]; do
  exec {admission_fd}>"$admission_lock"
  flock "$admission_fd"
  reconcile_claims

  active="$(active_claim_count)"
  available_memory="$(read_meminfo_kib MemAvailable)"
  available_swap="$(read_meminfo_kib SwapFree)"
  available_disk="$(disk_available_kib)"
  available_cpus="$(cpu_count)"
  reserved_for_claims=$((active * memory_claim_kib))
  required_memory=$((memory_reserve_kib + memory_claim_kib + reserved_for_claims))
  swap_shortfall_kib=$((minimum_swap_free_kib > available_swap ? minimum_swap_free_kib - available_swap : 0))
  required_memory_with_swap_shortfall=$((required_memory + swap_shortfall_kib))
  required_cpus=$(((active + 1) * build_jobs))
  reason=""

  if (( active >= max_concurrent )); then
    reason="concurrency_limit"
  elif (( available_memory < required_memory )); then
    reason="memory_pressure"
  elif (( available_swap < minimum_swap_free_kib && available_memory < required_memory_with_swap_shortfall )); then
    reason="swap_pressure"
  elif (( available_disk < minimum_disk_free_kib )); then
    reason="disk_pressure"
  elif (( available_cpus < required_cpus )); then
    reason="cpu_capacity"
  fi

  if [[ -z "$reason" ]]; then
    start_token="$(process_start_token "$$")"
    if [[ -z "$start_token" ]]; then
      flock -u "$admission_fd"
      exec {admission_fd}>&-
      echo "Unable to obtain the current process identity for Cargo admission" >&2
      exit 78
    fi
    claim_path="${claims_dir}/$$-${start_token}.claim"
    umask 077
    printf 'pid=%s\nstart=%s\njobs=%s\nmemory_claim_kib=%s\n' "$$" "$start_token" "$build_jobs" "$memory_claim_kib" > "$claim_path"
  fi

  flock -u "$admission_fd"
  exec {admission_fd}>&-

  if [[ -z "$claim_path" ]]; then
    if [[ "$no_wait" == "1" ]]; then
      echo "Agent Browser Cargo build capacity unavailable: reason=$reason active=$active max=$max_concurrent" >&2
      exit 75
    fi
    if [[ "$reason" != "$last_reason" ]]; then
      echo "Waiting for Agent Browser Cargo build capacity: reason=$reason active=$active max=$max_concurrent" >&2
      last_reason="$reason"
    fi
    sleep "$poll_seconds"
  fi
done

if [[ "$probe_only" == "1" ]]; then
  printf '{"admitted":true,"pid":%s,"jobs":%s,"maxConcurrent":%s,"acceleration":{"cache":"%s","linker":"%s"}}\n' \
    "$$" "$build_jobs" "$max_concurrent" "$cargo_cache" "$fast_linker"
  sleep "$probe_hold_seconds"
  exit 0
fi

kernel_release="$(uname -r 2>/dev/null || true)"
if [[ "${AGENT_BROWSER_CARGO_FORCE_WSL:-0}" == "1" || "$kernel_release" == *microsoft* || "$kernel_release" == *Microsoft* ]]; then
  memory_high="${AGENT_BROWSER_CARGO_MEMORY_HIGH:-20G}"
  memory_max="${AGENT_BROWSER_CARGO_MEMORY_MAX:-24G}"
  swap_max="${AGENT_BROWSER_CARGO_SWAP_MAX:-4G}"
  tasks_max="${AGENT_BROWSER_CARGO_TASKS_MAX:-512}"
  aggregate_memory_high="${AGENT_BROWSER_CARGO_AGGREGATE_MEMORY_HIGH:-28G}"
  aggregate_memory_max="${AGENT_BROWSER_CARGO_AGGREGATE_MEMORY_MAX:-32G}"
  aggregate_swap_max="${AGENT_BROWSER_CARGO_AGGREGATE_SWAP_MAX:-4G}"
  aggregate_tasks_max="${AGENT_BROWSER_CARGO_AGGREGATE_TASKS_MAX:-1024}"
  cargo_slice="${AGENT_BROWSER_CARGO_SLICE:-agent-browser-cargo.slice}"

  if ! command -v systemd-run >/dev/null 2>&1 || ! systemctl --user show-environment >/dev/null 2>&1; then
    echo "Refusing to run uncapped Cargo on WSL: the user systemd manager is unavailable." >&2
    echo "Restore user systemd, or set AGENT_BROWSER_CARGO_ALLOW_UNCAPPED=1 for an explicit one-off override." >&2
    if [[ "${AGENT_BROWSER_CARGO_ALLOW_UNCAPPED:-0}" != "1" ]]; then
      exit 78
    fi
    "${cargo_environment[@]}" cargo "$@"
    exit $?
  fi

  systemctl --user set-property --runtime "$cargo_slice" "MemoryHigh=$aggregate_memory_high" "MemoryMax=$aggregate_memory_max" "MemorySwapMax=$aggregate_swap_max" "TasksMax=$aggregate_tasks_max" >/dev/null

  echo "Running admitted Cargo in a WSL cgroup: jobs=$build_jobs active_capacity=$max_concurrent MemoryHigh=$memory_high MemoryMax=$memory_max aggregate_slice=$cargo_slice cache=$cargo_cache linker=$fast_linker" >&2
  systemd-run \
    --user \
    --scope \
    --quiet \
    --slice="$cargo_slice" \
    --property="MemoryHigh=$memory_high" \
    --property="MemoryMax=$memory_max" \
    --property="MemorySwapMax=$swap_max" \
    --property="TasksMax=$tasks_max" \
    "${cargo_environment[@]}" cargo "$@"
  exit $?
fi

echo "Running admitted Cargo: jobs=$build_jobs active_capacity=$max_concurrent cache=$cargo_cache linker=$fast_linker" >&2
"${cargo_environment[@]}" cargo "$@"
