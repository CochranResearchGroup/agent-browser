# CI Evaluation Boundary

## Decision

CI evaluation is now treated as a separate task. Agents should not actively
watch GitHub Actions, analyze CI logs, rerun jobs, or continue CI tuning during
ordinary implementation closeout unless the maintainer explicitly asks for CI
evaluation or release gating.

## Ordinary Closeout Behavior

- Run relevant local validation for the changed surface before commit.
- Push when the work is ready for remote backup or shared continuity.
- Perform at most one lazy CI status check if the pushed run should already
  have completed.
- If CI is still queued or in progress, report the run URL and current state
  without waiting.
- Do not convert normal feature work into CI polish just because a run is slow.

## Context

The CI shortening pass produced useful changes, including splitting Rust
quality checks from Rust tests, partitioning serial env-mutating tests in
`scripts/ci/rust-tests.sh`, removing retry sleeps from install retry tests, and
using Cargo's default test profile for ordinary Rust CI. The final observed
ordinary CI run for commit `4997504` completed successfully, with the Rust job
at 5m10s versus the prior optimized-profile run at 8m48s.

That result is good enough for ordinary commits. Further CI optimization should
be scheduled as a dedicated task with explicit acceptance criteria.
