# P07 Release Dry Run Cross-Target Fix

Date: 2026-05-29
Lane: P07
Status: IN PROGRESS

## Scope

Record the first `Release` workflow dry-run failure and the cross-target source
fix before retrying the release gate.

## Failed Gate

- Workflow: `Release`
- Run: `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26647064696`
- Ref: `main`
- Input: `dry_run=true`
- Result: failed

The release-state precheck passed, then all platform build jobs failed before
the release creation stage. The relevant Rust compile error was:

- `cli/src/native/cdp/chrome.rs` referenced
  `private_remote_display_allowed`, `is_wsl_mounted_windows_executable`, and
  `start_remote_headed_virtual_display` from non-Linux targets.

Those helpers are Linux-only because private remote-headed virtual displays are
implemented through Xvfb and WSL-mounted Windows executable translation.

## Fix

`try_launch_chrome` now keeps the private virtual-display fallback entirely
inside the `target_os = "linux"` cfg block. Non-Linux targets keep
`remote_headed_display` as `None`.

## Local Validation

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml private_remote_display -- --test-threads=1`

All passed.

## Cross-Target Check

`cargo check --manifest-path cli/Cargo.toml --target x86_64-pc-windows-gnu`
advanced past the previous missing-symbol errors, then stopped because the
local workstation does not have `x86_64-w64-mingw32-gcc` for the `ring` build
script. The release workflow retry remains the authoritative cross-platform
validation.
