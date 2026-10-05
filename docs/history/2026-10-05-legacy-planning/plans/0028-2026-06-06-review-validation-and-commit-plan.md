# Review, Validation, And Commit Plan

Date: 2026-06-06
State: CLOSED
Lane: P13
Depends On:
- `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`
- `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`

## Purpose

Turn the post-implementation recommendations into a reviewable, committed
change set with current validation and live workstation readback evidence.

## Scope

- Review the current dirty worktree by coherent boundary:
  - Plan 0026 resource monitor, resource GC, and retained resource docs.
  - Plan 0027 access-plan profile reuse, route hints, duplicate-lane guard, and
    doctor duplicate-pressure visibility.
  - Dashboard selected-workspace and launcher visibility that consumes those
    service surfaces.
- Run `pnpm validation:select -- --base HEAD` and execute the relevant selected
  no-launch gates.
- Run live workstation readbacks:
  - `agent-browser service resources --json`
  - `agent-browser install doctor --json`
  - one real `agent-browser service access-plan ... --json` against the profile
    lane that was previously multiplying browsers.
- Commit in reviewable chunks after the worktree proves coherent.

## Validation Checklist

Required no-launch checks:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_resources -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_profile_lease_gate`
- `cargo test --manifest-path cli/Cargo.toml service_request_`
- `cargo test --manifest-path cli/Cargo.toml install_doctor_flags`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `node scripts/dev/select-validation.js --base HEAD --json`

Live or operator-visible checks:

- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm publish:local-dashboard -- --expect-marker <changed-ui-marker>`
- Live resource, doctor, and access-plan readbacks listed above.

Live checks can be skipped only with a recorded reason. No-launch checks should
pass before commits are made.

## Commit Plan

Use coherent commits, adjusting if the diff boundaries prove different after
review:

1. Resource monitor and conservative GC.
2. Minimal runtime profile reuse and duplicate-lane enforcement.
3. Dashboard workspace and launcher visibility.
4. Documentation, skills, and validation-plan closeout.

## Closeout Contract

Close this plan only after the selected validation evidence and live readbacks
are recorded, the worktree is committed in coherent chunks, and any skipped
live checks have explicit rationale.

## Closeout

Completed on 2026-06-06.

No-launch validation passed:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_resources -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_profile_lease_gate`
- `cargo test --manifest-path cli/Cargo.toml service_request_`
- `cargo test --manifest-path cli/Cargo.toml install_doctor_flags`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `node scripts/dev/select-validation.js --base HEAD --json`

Live validation passed:

- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm publish:local-dashboard -- --expect-marker "Duplicate profile pressure detected" --json`

Live readbacks:

- `agent-browser service resources --json` reported `candidateCount: 0`,
  `protectedCount: 1`, and six duplicate-profile-pressure warnings after the
  local dashboard runtime publish.
- `agent-browser install doctor --json` reported the service ready and
  zero resource cleanup candidates. It remained nonzero because the local debug
  runtime intentionally differs from the pnpm global and workspace release
  binaries, and because duplicate profile pressure is still present in live
  retained state.
- `agent-browser service access-plan --login-id canva --json` exposed the new
  `decision.profileReuse` surface with `recommendedAction:
  register_or_select_profile` because no managed Canva profile is registered.

The implementation spans shared service access, resource monitoring, dashboard
visibility, generated clients, docs, skills, and live-runtime publishing
surfaces. The overlapping files make artificial commit splitting higher risk
than a single coherent checkpoint, so the slice is committed as one reviewable
P13 stabilization commit.
