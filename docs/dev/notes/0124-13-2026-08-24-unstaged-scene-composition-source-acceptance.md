# Plan 0124 Unstaged Scene Composition Source Acceptance

Date: 2026-08-24

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: SOURCE-ONLY

Status: ACCEPTED

## Accepted Boundary

The Desktop Evidence Coordinator now has a distinct stacking-or-occlusion
scene request. It uses desktop presentation capacity without claiming
browser-external prompt perception, paired page absence, a popup trigger, or a
staging effect.

Configured observation-only adapters now provide:

- exact authority snapshot and re-read;
- scene-generation verification;
- no-op restoration only when the episode never staged the desktop;
- an existing durable-handoff reference when one already belongs to the
  browser;
- deterministic cleanup receipt identity;
- response-only retention of the configured frame until a product caller
  consumes or drops it.

Scene snapshot, verification, current-authority, restore, and frame operations
are fallible. Failures after reservation remain terminal and still attempt
slot release and cleanup. Configured staging explicitly returns
`desktop_scene_staging_provider_unavailable` until exact native window-state
snapshot and restoration are implemented.

The existing `desktop_prompt_observe` action remains unchanged and fail-closed.
It is a Plan 0110 synthetic prompt-perception contract and is not reused as a
generic Plan 0124 scene action.

## Validation

- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_evidence::tests -- --nocapture`
  passed 21 tests.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_evidence_configured::tests -- --nocapture`
  passed 10 tests.
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
  passed.
- Both touched Rust files were formatted directly with `rustfmt` while another
  independent packet had uncommitted Rust changes in the shared worktree.

## Remaining Boundary

No task-shaped product action invokes the configured scene composition yet.
The presentation-capacity adapter also needs an exact active-presentation lease
that releases without parking the retained browser. Paired CDP evidence and
real staged snapshot and restoration remain separate unfinished boundaries.
No installed runtime or live browser state changed in this slice.
