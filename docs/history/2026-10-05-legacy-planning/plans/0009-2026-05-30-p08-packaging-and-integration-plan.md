# P08 Packaging And Integration Plan

Date: 2026-05-30
State: CLOSED
Lane: P08
Outcome: COMPLETE

## Purpose

Package the completed Plan 0008 CDP tab streaming implementation into a
durable, reviewable repo checkpoint, then push it so remote backup and CI can
reason from the current validated state.

## Policy Basis

- `docs/dev/policies/0001-policy-management.md`: re-read relevant repo-local
  policy before non-trivial work and treat `AGENTS.md` as the policy-loading
  contract.
- `docs/dev/policies/0004-git-worktree-hygiene.md`: start branch-sensitive
  work with `git status`, preserve the bounded lane scope, and do not call the
  work merge-ready while intended changes are uncommitted.
- `docs/dev/policies/0005-commit-history-discipline.md`: make a coherent,
  truthful commit that represents one tightly related slice.
- `docs/dev/policies/0007-commit-and-push-cadence.md`: commit at a meaningful
  slice boundary and push when remote backup, CI, or continuity matters.
- `docs/dev/policies/0010-validation-and-handoff.md`: include concrete
  validation evidence and record what live smoke proved.

## Current Worktree Scope

`git status --short --branch` shows local `main` even with `origin/main` and a
bounded dirty P08 slice:

- CDP stream service/runtime changes under `cli/src/native/`
- CLI, README, docs site, and installed skill documentation updates
- dashboard view-stream helper and smoke coverage updates
- Plan 0008 closeout and a dated contract-audit note
- new live smoke script for service-owned CDP tab streaming

No unrelated dirty files are present in the current status output.

## Execution Steps

1. Confirm the final diff is the expected bounded P08 feature slice.
2. Run final hygiene checks after writing this plan.
3. Stage the full P08 slice, including this integration plan and the new
   audit and live-smoke files.
4. Commit with a truthful feature-slice subject:
   `Add service CDP tab streaming for non-remote browsers`.
5. Push `main` to `origin` because this is validated end-of-slice work and
   the repo guidance says normal project work lands in this fork's `origin`
   repository.
6. Do one lazy CI status check after push and report the current state without
   active babysitting.

## Validation Evidence To Preserve

Already passed before this packaging plan:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml set_cdp_session_id -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm test:dashboard-view-streams`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `node scripts/dev/select-validation.js --base HEAD --json`

The live smoke proved that a service-owned local headless browser can expose a
`cdp_screencast` stream, accept a WebSocket viewer, receive frames, focus from
page A to page B through `view_focus`, refocus page A, and produce distinct
frames after each tab switch.

## Completion Criteria

- This plan file exists in `docs/dev/plans/`.
- The P08 slice is committed as one coherent local commit.
- The commit is pushed to `origin/main`.
- One post-push CI status check is recorded in closeout.

## Execution Evidence

- Plan file exists at
  `docs/dev/plans/0009-2026-05-30-p08-packaging-and-integration-plan.md`.
- The bounded P08 slice was committed as
  `79bb481f Add service CDP tab streaming for non-remote browsers`.
- `git rev-parse HEAD origin/main` returned the same commit:
  `79bb481f25165d34e79df3330b7d5d5540202f1c`.
- `git status --short --branch` reported `## main...origin/main` with no
  dirty files after the P08 push.
- A post-push CI status check found the CI run for commit `79bb481f` in
  progress:
  `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26688779709`.

The P08 implementation commit remains the coherent feature-slice checkpoint.
This plan-status update is a follow-up execution closeout for Plan 0009.
