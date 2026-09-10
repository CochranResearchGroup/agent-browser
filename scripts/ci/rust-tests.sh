#!/usr/bin/env bash
set -euo pipefail

manifest_path="${CARGO_MANIFEST_PATH:-cli/Cargo.toml}"
profile="${CARGO_TEST_PROFILE:-}"
test_stack_bytes="${RUST_TEST_STACK_SIZE_BYTES:-16777216}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"

export RUST_MIN_STACK="$test_stack_bytes"

if [[ ! "$RUST_MIN_STACK" =~ ^[0-9]+$ ]] ||
  ((RUST_MIN_STACK < 2097152 || RUST_MIN_STACK > 67108864)); then
  echo "RUST_TEST_STACK_SIZE_BYTES must be an integer from 2097152 through 67108864" >&2
  exit 2
fi

cargo_test=("$script_dir/cargo-safe.sh" test --manifest-path "$manifest_path")
cdp_cargo_test=("$script_dir/cargo-safe.sh" test -p agent-browser-cdp --manifest-path "$repo_root/Cargo.toml")
if [[ -n "$profile" ]]; then
  cargo_test+=(--profile "$profile")
  cdp_cargo_test+=(--profile "$profile")
fi

usage() {
  echo "Usage: scripts/ci/rust-tests.sh [--focused <filter> | --compartment <name> | --list-compartments]"
  echo "Compartments: transport, cli-native, cli-workstation, cli-core, cli-integration"
}

run_transport() {
  echo "Running Rust compartment: transport"
  "${cdp_cargo_test[@]}"
}

run_cli_native() {
  echo "Running Rust compartment: cli-native"
  "${cargo_test[@]}" --bin agent-browser native:: -- --test-threads=1
}

run_cli_core() {
  echo "Running Rust compartment: cli-core"
  "${cargo_test[@]}" --bin agent-browser -- \
    --skip native:: \
    --skip workstation \
    --test-threads=1
}

run_cli_workstation() {
  echo "Running Rust compartment: cli-workstation"
  "${cargo_test[@]}" --bin agent-browser workstation -- --test-threads=1
}

run_cli_integration() {
  echo "Running Rust compartment: cli-integration"
  "${cargo_test[@]}" \
    --test close_scope \
    --test remote_view_doctor_scope \
    --test service_state_validation \
    --test session_supervisor \
    -- --test-threads=1
}

run_compartment() {
  case "$1" in
    transport) run_transport ;;
    cli-native) run_cli_native ;;
    cli-workstation) run_cli_workstation ;;
    cli-core) run_cli_core ;;
    cli-integration) run_cli_integration ;;
    *)
      echo "Unknown Rust test compartment: $1" >&2
      usage >&2
      return 2
      ;;
  esac
}

run_and_record() {
  local name="$1" runner="$2" log_root="$3" started_at status
  started_at=$SECONDS
  status=0
  "$runner" >"$log_root/$name.log" 2>&1 || status=$?
  printf '%s=%s elapsedSeconds=%s\n' "$name" "$status" "$((SECONDS - started_at))" \
    >"$log_root/$name.result"
  return "$status"
}

run_comprehensive() {
  local log_root native_pid support_pid native_status support_status started_at
  log_root="$(mktemp -d "${TMPDIR:-/tmp}/agent-browser-rust-tests.XXXXXX")"
  trap 'rm -rf "$log_root"' RETURN
  started_at=$SECONDS

  echo "Running comprehensive Rust tests in two isolated serial compartments"
  echo "Rust test thread stack: $RUST_MIN_STACK bytes"

  set +e
  run_and_record cli-native run_cli_native "$log_root" &
  native_pid=$!
  (
    lane_status=0
    run_and_record cli-workstation run_cli_workstation "$log_root" || lane_status=1
    run_and_record cli-core run_cli_core "$log_root" || lane_status=1
    run_and_record transport run_transport "$log_root" || lane_status=1
    run_and_record cli-integration run_cli_integration "$log_root" || lane_status=1
    exit "$lane_status"
  ) &
  support_pid=$!

  wait "$native_pid"
  native_status=$?
  wait "$support_pid"
  support_status=$?
  set -e

  for compartment in cli-native cli-workstation cli-core transport cli-integration; do
    cat "$log_root/$compartment.log"
    cat "$log_root/$compartment.result"
  done
  printf 'Rust comprehensive result: nativeLane=%s supportLane=%s elapsedSeconds=%s\n' \
    "$native_status" "$support_status" "$((SECONDS - started_at))"

  if [[ "$native_status" -ne 0 || "$support_status" -ne 0 ]]; then
    return 1
  fi
}

case "${1:-}" in
  "")
    run_comprehensive
    ;;
  --focused)
    if [[ $# -ne 2 || -z "$2" || "$2" == -* ]]; then
      usage >&2
      exit 2
    fi
    echo "Running focused CLI Rust tests: $2"
    "${cargo_test[@]}" --bin agent-browser "$2" -- --test-threads=1
    ;;
  --compartment)
    if [[ $# -ne 2 || -z "$2" ]]; then
      usage >&2
      exit 2
    fi
    run_compartment "$2"
    ;;
  --list-compartments)
    printf '%s\n' transport cli-native cli-workstation cli-core cli-integration
    ;;
  -h|--help)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
