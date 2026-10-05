# P51 Daemon Authority Guard Plan

Date: 2026-06-27
State: COMPLETE
Lane: P51
Depends On:
- `docs/dev/plans/0050-2026-06-26-s3-binary-authority-visible-window-plan.md`
- `/tmp/agent-browser-p50-daemon-gate-BmulZ7`

## Purpose

Promote the P50 daemon-listener discovery into the normal automated guard
surface. P50 proved that command-path authority is insufficient because the
default socket can have multiple live daemon listeners, including stale deleted
repo-debug binaries and installed binaries. `agent-browser install doctor`
must report that condition directly before any remote-view stress run relies
on the daemon.

## Non-Negotiable Rules

- Do not rerun `s3-open` or full S3 in this plan.
- Do not kill or restart daemon processes in this plan.
- `install doctor` must expose daemon listener inventory in JSON.
- `install doctor` must emit readiness-impacting issues for:
  - multiple live daemon socket listeners;
  - no listener matching the current executable realpath;
  - any listener running a deleted executable inode.
- Validation must include focused Rust tests and a live doctor readback.

## Goal 1: Install Doctor Listener Inventory

`/goal execute P51 goal 1: make install doctor inventory live agent-browser socket listeners with process identity`

Work:

- Add a daemon listener inventory to `install_doctor_report`.
- Use the user socket directory and `ss -xlpn` on Unix to identify live
  listener PIDs.
- Enrich each listener with `/proc/<pid>/exe`, cwd, cmdline, deleted-executable
  state, and whether it matches the current executable realpath.

Evidence:

- `agent-browser install doctor --json` includes `daemonListenerInventory`.

## Goal 2: Readiness Issues

`/goal execute P51 goal 2: make install doctor fail on ambiguous or stale daemon listeners`

Work:

- Add issue code `daemon_socket_multiple_listeners`.
- Add issue code `daemon_socket_current_executable_mismatch`.
- Add issue code `daemon_socket_deleted_executable`.
- Keep remedies as operator-plan guidance, not automatic process killing.

Evidence:

- Focused Rust tests cover each issue shape.
- Live doctor readback reports current listener ambiguity.

## Goal 3: Preserve P46/P50 Lock

`/goal execute P51 goal 3: record that the next executable step is daemon lifecycle repair, not S3`

Work:

- Keep P46 and P50 locked.
- Record validation results and current live doctor output in this plan.
- Recommend a follow-up daemon lifecycle repair plan that can safely converge
  default socket ownership to one intended binary.

Closeout:

- P51 is complete when install doctor catches the drift surface that let P49
  and P50 run against ambiguous daemon authority, validation passes, and runtime
  state remains clean.

## Execution Log

### 2026-06-27

Implemented:

- `agent-browser install doctor --json` now includes
  `data.daemonListenerInventory`.
- The inventory uses the user socket directory and `ss -xlpn` on Unix to find
  live agent-browser socket listeners.
- Each listener records PID, socket path, executable path, cwd, cmdline,
  deleted-executable state, and whether the executable matches the current
  executable realpath.
- `install doctor` now emits readiness-impacting issues:
  - `daemon_socket_multiple_listeners`;
  - `daemon_socket_current_executable_mismatch`;
  - `daemon_socket_deleted_executable`.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor_flags`
- `cargo build --manifest-path cli/Cargo.toml`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/install.rs docs/dev/plans/0051-2026-06-27-daemon-authority-guard-plan.md`

Live doctor proof:

- Artifact: `/tmp/agent-browser-p51-install-doctor.json`
- Command: `./cli/target/debug/agent-browser --json install doctor`
- Result: `success: false`
- New issue codes present:
  - `daemon_socket_multiple_listeners`;
  - `daemon_socket_current_executable_mismatch`;
  - `daemon_socket_deleted_executable`.
- Live daemon inventory reported:
  - `state: ambiguous`;
  - `listenerCount: 10`;
  - `currentExecutableMatchCount: 0`;
  - `deletedExecutableCount: 7`.

Runtime state after validation:

- No S3 or `s3-open` live proof was rerun.
- No daemon process was killed or restarted.
- The next plan should safely converge daemon lifecycle authority to exactly
  one intended listener before reopening P50 or P46.
