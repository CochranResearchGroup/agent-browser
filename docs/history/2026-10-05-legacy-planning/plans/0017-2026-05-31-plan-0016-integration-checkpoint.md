# Plan 0016 Integration Checkpoint

Date: 2026-05-31
State: COMPLETE
Lane: integration hygiene
Depends On:
- `docs/dev/plans/0016-2026-05-31-effective-stealth-remote-default-launch-plan.md`

## Purpose

Make the completed Plan 0016 runtime posture work reviewable and durable
without hiding unrelated dashboard, retained-cleanup, or App Intelligence work
inside the same checkpoint.

The active worktree contains several overlapping lanes. Plan 0016 is complete
as a runtime and hosted-dashboard validation result, but a clean integration
slice still needs to separate the effective stealth remote default launch work
from pre-existing dirty files before committing.

## Scope

In scope:

- effective launch defaults for ordinary launches
- manifest executable source handling
- Google Sheets built-in site policy
- Plan 0016 validation evidence
- authenticated dashboard runtime smoke harness support needed to prove the
  hosted UX without competing for the managed default runtime profile
- docs and skill updates that describe the stealth remote default behavior

Out of scope:

- retained orphan profile cleanup
- needs-attention and retained pool redesign
- broad inspector pane implementation
- contextual Chat/App Intelligence provider work beyond smoke evidence that
  already exists
- package version changes or formal release preparation

## Execution Plan

1. Classify dirty files into Plan 0016, prerequisite smoke harness, and
   unrelated prior lanes.
2. Stage only clean Plan 0016 hunks and required plan evidence.
3. Leave mixed files unstaged when clean hunk staging would risk dropping or
   misattributing prior work.
4. Run validation that covers staged Rust, docs, skill, and smoke-script
   changes.
5. Commit the checkpoint only if the staged diff is coherent and validation
   proves the committed surface.

## Validation

Minimum validation before commit:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml native::actions -- --nocapture --test-threads=1
node --check scripts/smoke-local-dashboard-runtime.js
git diff --check
```

Runtime evidence can be reused from Plan 0016 only if the installed binary and
hosted smoke have not drifted since that plan was marked complete.

## Completion Criteria

- The repo contains this plan and completed Plan 0016 evidence.
- A focused commit exists for the Plan 0016 integration checkpoint, or the plan
  records the exact file overlap that prevents a safe commit.
- Validation evidence is recorded in this file.
- Unrelated dirty work is preserved.

## Execution Result

Collected on 2026-05-31:

- Graphiti discovery was healthy, but returned no newer Plan 0016 authority
  than the repo-local plan and source files.
- `pnpm validation:select -- --base HEAD` reported 56 changed files and
  recommended broad dashboard, service-client, Rust, docs, and local publish
  gates for the whole dirty tree.
- The current dirty worktree spans multiple lanes:
  - Plan 0016 effective stealth remote default launch work in
    `cli/src/main.rs`, `cli/src/native/actions.rs`,
    `cli/src/native/service_model.rs`, `README.md`,
    `docs/src/app/service-mode/page.mdx`, `skills/agent-browser/SKILL.md`,
    `scripts/smoke-local-dashboard-runtime.js`, and Plan 0016 itself.
  - retained orphan profile cleanup in `cli/src/native/actions.rs`,
    `cli/src/output.rs`, `README.md`, `docs/src/app/service-mode/page.mdx`,
    and `skills/agent-browser/SKILL.md`.
  - workspace navigator, CDP stream, inspector, and contextual Chat work across
    dashboard source, stream backend source, service request contracts, scripts,
    and Plans 0011 through 0015.
- `cli/src/native/actions.rs` contains both the Plan 0016 effective default
  launch helper/tests and unrelated orphaned-profile prune logic/tests. Because
  that is the core Plan 0016 source file, a whole-file commit would be
  misleading and a partial commit should be done only in a clean integration
  branch or side worktree.

Validation run on the current integrated state:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed.
- `node --check scripts/smoke-local-dashboard-runtime.js` passed.
- `git diff --check` passed.
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
  passed with 27 tests.
- `cargo test --manifest-path cli/Cargo.toml native::actions -- --nocapture
  --test-threads=1` passed with 177 tests.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.

Decision:

- Do not create a Plan 0016 commit directly from this dirty worktree.
- Preserve all existing dirty work.
- Create a clean side worktree from `main`, re-apply only Plan 0016 hunks plus
  the smoke harness option, run validation there, and commit from that isolated
  worktree.

Side-worktree execution:

- Created branch `plan0016-integration` in
  `/home/ecochran76/workspace.local/agent-browser-plan0016-integration`.
- Re-applied the Plan 0016 source, docs, skill, plan, and smoke-harness
  changes without the retained orphan profile cleanup or broad dashboard/App
  Intelligence work.
- Verified no retained-orphan cleanup strings remained in the isolated Plan
  0016 candidate.
- Committed the isolated checkpoint on branch `plan0016-integration`.

Side-worktree validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed after applying
  rustfmt to the isolated worktree.
- `node --check scripts/smoke-local-dashboard-runtime.js` passed.
- `git diff --check` passed.
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
  passed with 27 tests.
- `cargo test --manifest-path cli/Cargo.toml native::actions -- --nocapture
  --test-threads=1` passed with 176 tests. The retained orphan profile cleanup
  test is intentionally absent from the isolated Plan 0016 branch.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.
- `pnpm --dir docs build` passed after installing the side-worktree docs
  dependencies. A temporary symlink attempt failed because Turbopack rejects
  `docs/node_modules` symlinks that point outside the workspace root; the
  symlink was removed before the successful install and build.
