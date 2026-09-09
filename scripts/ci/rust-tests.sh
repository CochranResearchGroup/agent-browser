#!/usr/bin/env bash
set -euo pipefail

manifest_path="${CARGO_MANIFEST_PATH:-cli/Cargo.toml}"
profile="${CARGO_TEST_PROFILE:-}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"

cargo_test=("$script_dir/cargo-safe.sh" test --manifest-path "$manifest_path")
cdp_cargo_test=("$script_dir/cargo-safe.sh" test -p agent-browser-cdp --manifest-path "$repo_root/Cargo.toml")
if [[ -n "$profile" ]]; then
  cargo_test+=(--profile "$profile")
  cdp_cargo_test+=(--profile "$profile")
fi

echo "Running CDP transport crate tests"
"${cdp_cargo_test[@]}"

# Some CLI tests mutate process-global environment variables or user-scoped
# runtime state. One serial harness invocation preserves their isolation and
# compiles the large CLI test executable only once on uncached runners.
echo "Running CLI Rust tests serially"
"${cargo_test[@]}" -- --test-threads=1
