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

## Second Dry Run

- Workflow: `Release`
- Run: `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26647678335`
- Ref: `main`
- Input: `dry_run=true`
- Result: failed

The cfg leak fix worked for non-Linux targets: Windows x64, macOS x64, and
macOS ARM64 passed. Linux targets failed later during zigbuild linking because
`cli/src/native/browser.rs` linked directly against `libX11`.

## Linux Link Fix

The X11 browser-focus helper now loads `libX11` dynamically with `dlopen` and
resolves the required symbols with `dlsym`. This keeps the runtime focus
behavior available on Linux hosts that have libX11 installed, while removing
the release-time `-lX11` linker requirement for Linux, musl, and ARM release
targets.

Additional validation after this fix:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml browser -- --test-threads=1`
- `git diff --check`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `rg -n "#\\[link\\(name = \\\"X11\\\"\\)|-lX11" cli/src`

All completed cleanly. The local machine does not have `cargo-zigbuild`, so
the release workflow retry remains the authoritative Linux cross-target
validation.
