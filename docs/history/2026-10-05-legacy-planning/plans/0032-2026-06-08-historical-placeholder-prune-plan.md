# Plan 0032: Historical Placeholder Prune

## Goal

Finish stopping service-state hoarding by removing historical browser placeholders that are not explicit `process_exited` rows. Empty `not_started`, failed, and no-runtime unreachable rows should not remain operational browser/session state when they have no live process, CDP endpoint, live tabs, or usable stream.

## Scope

1. Add a shared historical-placeholder classifier for reconciliation cleanup.
2. Widen retained-state prune so safe placeholders with stale active-session ids, profile ids, or stale streams are still removable.
3. Keep records with real runtime evidence out of this cleanup path.
4. Validate with focused Rust tests and Rust quality gates.
5. Install the updated local runtime and clean current retained placeholders.

## Validation

- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml prune_retained_service_state -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm validation:select -- --base HEAD`
- `git diff --check`

## Result

Implemented.

- Reconciliation now removes empty historical browser placeholders after recording terminal health evidence.
- Retention prune now treats safe `not_started`, `process_exited`, `unreachable`, and `faulted` placeholders as removable when they have no live tabs.
- Released and abandoned session pruning can remove stale retained view-stream metadata with the pruned browser/session pair instead of keeping left-rail history operational.
- Direct browser pruning still keeps records that are tied to existing session rows unless the session itself is eligible for pruning.

## Live Cleanup

Installed the updated local runtime with:

```bash
pnpm publish:local-dashboard -- --skip-browser --expect-marker "All records"
```

The runtime was installed to `/home/ecochran76/.local/bin/agent-browser`, backup `/home/ecochran76/.local/bin/agent-browser.pre-local-dashboard-20260608170156`, service PID `90345`.

Applied the final retained-state prune with:

```bash
agent-browser --json service prune-retained --process-exited-browsers --not-started-browsers --released-sessions --abandoned-sessions --abandoned-session-min-age-minutes 0 --apply
```

Live state changed from 70 browsers, 70 sessions, and 2 tabs to 3 browsers, 3 sessions, and 2 tabs. The prune removed 67 stale browsers and 67 stale sessions. A follow-up dry run returned zero browser and session candidates.

`agent-browser --json service resources` now reports zero GC candidates and still warns about two live Odollo sessions sharing `stealthcdp-default`: `odollo-carrier-ups` and `odollo-carrier-usps`. Those are live lease/profile reuse warnings, not historical retained-state hoarding.

## Validation Result

- Passed: `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- Passed: `cargo test --manifest-path cli/Cargo.toml prune_retained_service_state -- --test-threads=1`
- Passed: `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- Passed: `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- Passed: `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- Passed: `git diff --check`
- Ran: `pnpm validation:select -- --base HEAD`
- Blocked before assertion: `pnpm test:service-cdp-tab-streaming-live`

The live CDP tab-streaming smoke could not launch Chrome in WSL. Chrome exited before exposing DevTools and stderr included `UtilAcceptVsock: accept4 failed 110`; this did not reach an agent-browser code assertion.
