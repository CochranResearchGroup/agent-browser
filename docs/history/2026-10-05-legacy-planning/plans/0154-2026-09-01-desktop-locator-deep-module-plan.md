# Plan 0154: Desktop Locator Deep Module

Date: 2026-09-01

State: CLOSED

Lane: P154

Branch: `architecture/desktop-locator-deep-module`

Target: `main`

Source baseline: `c664c25bb2dac0daee619b82334d708ffa4e998f`

Authority: SOURCE-ONLY

Dependencies: [P110, P131]

Overlaps: []

## Objective

Make `native::desktop_locator` a deep module whose crate-wide interface contains
only the production action handler and the long-lived stream redactor. Remove
its duplicate captured-frame wrapper and use the canonical desktop capture
result across locator and prompt-perception internals without changing any
request, response, detector, receipt, privacy, or failure behavior.

## Current Evidence

- `actions` calls `handle_desktop_locate`, which performs configured capture and
  invokes the deterministic locator core.
- The locator currently declares nineteen `pub(crate)` items, while production
  callers require only `handle_desktop_locate` and
  `redact_desktop_locate_stream_result`.
- `BoundFrame` duplicates the three fields already owned by
  `DesktopCaptureResult` and leaks locator implementation vocabulary into
  `desktop_prompt_perception`.
- P110 and P131 already freeze source behavior and the live-provider boundary.
  This plan changes neither.

## Frozen Decisions

1. Keep the locator inside the existing CLI crate. This slice does not create a
   new crate or top-level workflow.
2. Preserve the two production interface operations exactly:
   `handle_desktop_locate` and `redact_desktop_locate_stream_result`.
3. Use `DesktopCaptureResult` as the single captured-frame representation.
4. Keep detector selection, OCR substitution, fixture rendering, and locator
   result types private implementation details.
5. Preserve every serialized field, stable fixture hash, typed error code,
   bounded-resource limit, and response-only visualization rule.
6. Keep `desktop_prompt_observe` provider-unavailable in configured production.
7. Do not install, launch, capture, or interact with a live browser or desktop.

## Work Units

| Unit | Scope | Depends on | Exit condition |
|---|---|---|---|
| W1 | Register the plan and active lane | none | Plan and lane metadata are published from the source baseline |
| W2 | Replace `BoundFrame` with `DesktopCaptureResult` | W1 | Locator and prompt fixtures use the canonical capture result |
| W3 | Tighten the locator interface | W2 | Exactly two crate-visible locator items remain |
| W4 | Validate and close | W3 | Focused, formatting, Clippy, and selected regression gates pass |

## Acceptance Criteria

1. `desktop_locator.rs` exposes exactly two `pub(crate)` items: the action
   handler and stream redactor.
2. `BoundFrame` and its conversion implementation no longer exist.
3. Locator and prompt-perception fixture code use `DesktopCaptureResult`
   directly.
4. All existing locator fixture observation and visualization hashes remain
   unchanged.
5. Existing request rejection, stale binding, ambiguity, detector failure, and
   stream-redaction tests pass without weakening assertions.
6. Existing prompt-perception tests pass with unchanged observable results.
7. Rust formatting and strict Clippy pass through the WSL Cargo safety wrapper.
8. Validation selection and patch hygiene pass for the complete slice.

## Validation

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_locator -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_prompt -- --test-threads=1
pnpm validation:select -- --base c664c25bb2dac0daee619b82334d708ffa4e998f
git diff --check
```

## Bounds

- Maximum implementation attempts: 2
- Maximum review and remediation cycles: 1
- Maximum no-progress checkpoints: 2
- Checkpoint interval: each completed work unit or 90 minutes

## Non-Goals

- New locator profiles, detectors, OCR providers, or prompt providers
- New service actions, schemas, CLI flags, documentation surfaces, or crates
- Production installation or live desktop validation
- Lease, service-state, runtime-host, capability-rotation, or output changes

## Completion Evidence

Implementation commit: `79cb48da`

Integration receipt: `ae4497119431548434330617ef408ed1f1d98428`

Progress classification: `outcome_progress`

- The locator now has exactly two `pub(crate)` declarations, covering the
  production action handler and stream redactor.
- `BoundFrame` and its conversion implementation were deleted. Locator and
  prompt-perception fixtures now use `DesktopCaptureResult` directly.
- Focused locator validation passed 10 tests, including the frozen observation
  and visualization hashes, stale binding, ambiguity, detector failure, request
  rejection, and stream redaction.
- Focused prompt-perception validation passed 26 tests with unchanged frozen
  corpus and privacy outcomes.
- Workspace formatting and strict workspace Clippy passed through
  `scripts/ci/cargo-safe.sh`.
- `pnpm validation:select` selected workspace formatting, workspace Clippy, a
  focused Rust filter, and patch hygiene. Every selected gate passed.
- No installed runtime, live browser, desktop provider, lease, service state,
  runtime host, capability rotation, or output surface was changed.
