# Plan 0168: Worktree, Branch, Merge, and Closure Campaign

Date: 2026-09-11

State: CLOSED

Consolidation: required

Lane: P168

Branch: `maintenance/plan-0168-repository-closeout`

Target: `main`

Integration: merge

Integration model: short-lived branch through the protected `main` workflow

Related plans: [Plan 0144](0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), [Plan 0155](0155-2026-09-02-durable-handoff-resume-intent-plan.md), [Plan 0156](0156-2026-09-02-full-runtime-shutdown-replacement-plan.md), [Plan 0157](0157-2026-09-02-profile-permissions-and-request-provenance-plan.md), [Plan 0161](0161-2026-09-09-first-class-profile-repair-and-reset-plan.md), [Plan 0162](0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [Plan 0165](0165-2026-09-11-bill-identifier-form-drift-repair.md), [Plan 0166](0166-2026-09-11-repository-custody-and-timed-out-connection-release.md), and [Plan 0167](0167-2026-09-11-production-scale-service-state-lock-attribution-and-critical-section-repair.md)

Current execution status: [RUNBOOK.md](../../../RUNBOOK.md)

## Objective and authority

Reconcile every current Agent Browser worktree and bounded local or remote
branch, merge only reviewed residual work, and close Git custody only when
cleanliness, publication, ancestry, plan state, catalog state, and integration
receipts agree. The campaign must reduce repository-maintenance debt without
silently discarding unique commits or converting source integration into a
claim of installed or operational acceptance.

This planning turn authorizes the plan artifact only. Future execution may
perform repository-local review, create the proposed maintenance branch and
worktree, update plans and the active-lane catalog, run provider-free tests,
open normal pull requests against this repository's `main`, and remove exact
worktrees or refs only after their closure gates pass.

The campaign does not authorize production installation, browser or profile
mutation, X or RuFresh retries, BILL authentication resume, credential or Auth
Vault mutation, provider effects, tenant effects, formal release work, broad
process cleanup, forced Git updates, or deletion of a ref that retains
unresolved unique work. Remote branch deletion remains a separately verified
closure step for the exact named ref.

## Current state

The snapshot below is planning evidence, not future execution authority. Every
value must be refreshed after `git fetch origin --prune` before a disposition.

- `origin/main` and the canonical `main` ref were
  `2618ddb2c97cedbcc453a9f9584063a7c721eca3`.
- Three worktrees existed. The Plan 0165 and P0240 worktrees were clean at
  `5ee95d670e52c1157f0f94b90d176e4066d1cd2f` and
  `0bb7fc5682816ee2f789adfee5fcfcc61428cc20`, respectively.
- Plan 0165 had zero commits not in `origin/main`; `origin/main` was seven
  commits ahead. Its source custody was integrated, but its plan remained open
  for the missing sealed Auth Vault entry and exact authentication resume.
- P0240 retained eight commits not in `origin/main` and was 79 commits behind.
  It was not represented in `docs/dev/active-lanes.yaml`.
- P157 retained four commits not in `origin/main` and was 71 commits behind.
  Plan 0161 was source-closed while Plan 0162 and broader installed acceptance
  remained open.
- The two documentation branches retained one and two unique commits,
  respectively. They had no worktrees.
- P144, P155, and P156 recorded checkpoints were ancestors of `origin/main`,
  while their catalog custody or plan projections did not describe that Git
  state consistently.
- The catalog-only active-lane auditor returned `ok: false` with 19 problems,
  including stale checkpoints, missing registered refs, metadata drift,
  incomplete integration evidence, unknown dependencies, and overlap
  dispositions that did not satisfy the auditor's exact contract.
- The active-only planning auditor accepted this plan's filename, state,
  current-state section, consolidation sections, roadmap link, and runbook
  link. Its lane parser accepts only two-digit identifiers and therefore
  rejects `P168`, while the same roadmap already contains `P167`. W1 must
  reconcile that deterministic-audit contract before a final green claim.
- No stale administrative worktree entries were reported by a dry-run prune.
- After that readback, the canonical worktree became dirty in
  `cli/src/commands.rs` and
  `cli/src/native/action_runtime/runtime/navigation.rs`. The owner and intended
  lane were not established. The latter path overlaps preserved branch work,
  so campaign execution must stop at W0 until custody is explicit.
- Graphiti was healthy, but the focused discovery returned no current facts for
  these lanes. Git, plans, the active-lane catalog, and the runbook remain the
  authorities for this campaign.
- P168 is intentionally not registered as active custody during this planning
  turn. W0 creates and registers it only after a clean dedicated worktree can
  be based on refreshed `origin/main` without absorbing foreign dirty work.

## W0 custody checkpoint

W0 is complete. The unexpected Rust edits were positively attributed to a
still-running Codex process whose parent workspace is Odollo and whose exact
child performed the observed Agent Browser workstation installation. That
install child exited. The foreign parent remains active, so its edits stay
untouched in the canonical worktree.

P168 now owns only
`/home/ecochran76/workspace.local/agent-browser-p0168-closeout` on branch
`maintenance/plan-0168-repository-closeout`, created from refreshed
`origin/main` at `2618ddb2c97cedbcc453a9f9584063a7c721eca3`. No foreign file was
stashed, reset, committed, copied, or overwritten. The P168 worktree contains
only its plan, roadmap, and runbook changes at this checkpoint.

## W1 and W2 closure checkpoint

W1 and W2 are complete. Checkpoint `9a61432a` published the closed-world
review, corrected the planning and lane metadata parsers, removed closed P155,
P156, P166, and P167 entries from the active catalog, and separated integrated
Git custody from the still-open P144, P157, and P165 plan outcomes.

After a fresh cleanliness, tip, process-cwd, patch-equivalence, and ancestry
readback, the campaign removed the clean Plan 0165 worktree; deleted the exact
local P165, P157, Last30days X, and Reddit branches; and deleted their matching
remote refs. It also deleted the already-integrated remote-only
`feature/p0240-sealed-auth`, `integration/plan-0161`,
`maintenance/consolidate-field-notes-20260908`, and
`plan/lease-authority-coordination` refs. P0240 runtime-compatible custody was
not changed.

P165 remains open for its sealed authentication gate. P144 remains open for
public mutation and daemon effect admission. Plan 0158 remains open under its
existing postmortem-only authority. Plan 0162 remains open and now explicitly
requires a fresh current-main branch if execution resumes.

## W3 residual integration checkpoint

W3 classified all eight P0240 commits. The first three are patch-equivalent to
`main`; `45360140` is formatting-only and superseded; and `67a2b98f` plus
`0bb7fc56` are historical candidate notes tied to an obsolete installed
baseline. The only current source residuals were `1104e16e` and `022d820d`.

Fresh branch `integration/p0168-p0240-residual` reconstructs those residuals
from current `origin/main`. Commit `2a5545db` disambiguates the geometry-epoch
test literal. Commit `7f2b7083` adds an exact `semantic_click` step, requires an
explicit exact role/name locator, rejects zero or multiple accessibility-tree
matches, validates recipes before queueing, and documents every required
user-facing surface. Pull request 30 merged as `de614fbe`, the protected-main
integration receipt.

Selected validation passed: Rust formatting; workspace clippy with warnings
denied; 2 semantic-click, 26 desktop-capture, and 62 service-request focused
tests; service API/MCP parity; generated service-client contracts and type
coverage; no-launch service collections; the docs production build;
remote-view documentation; and all six workstation fixtures selected by the
changed-path auditor. Initial eight-job Cargo attempts could not create worker
threads at 949 of 1,024 shared slice tasks. The exact gates passed with one
build job and compiler caching disabled. The installed shared skill remains
unchanged until normal source integration and publication; no runtime effect
was used as acceptance evidence.

After that merge, a fresh cleanliness, tip, ancestry, and process-cwd preflight
found one task-owned debug daemon left by the passing no-launch fixture. PID
94587 was bound to isolated home `/tmp/ab-service-collections-no-launch-95VDIZ`
and exact session `service-collections-no-launch-94578`; it terminated cleanly
on `SIGTERM`. The preflight then found both P0240 worktrees clean and idle. The
campaign removed both worktrees, their local branches, and their matching
remote branches. Their named commits and PR 30 remain the recovery locators.

PR 31 merged the P168 custody record as
`9f711ebcccf6393fcf1ff22778fba4b3450c3224`. The final closeout packet removes
P168 from the active catalog, moves the surviving open lanes' plan authority to
`origin/main`, and closes this plan through the protected-main workflow.

## W5 closure

W5 is complete subject only to mechanical deletion of this exact closeout
worktree and branch after the final protected-main merge receipt exists. The
final branch-local audit is green after P168 leaves the catalog. Surviving P165,
P157, and P144 entries describe integrated Git custody and separate open plan
outcomes from `origin/main`; they require no historical branch or worktree.

The final intended inventory is canonical `main` only. Its two Odollo-owned
uncommitted Rust paths remain attributable foreign custody and are not modified
by this campaign. Pull request 32 is the final plan-state, catalog, roadmap,
runbook, and closeout-note integration receipt. After it merges, the exact P168
worktree and local and remote maintenance refs are eligible for deletion.

## Frozen decisions

### Review is closed-world and deletion is last

The authorized population is the refreshed worktree inventory, all local
branches, the exact `origin/*` branches named by the refreshed review, and every
lane in `docs/dev/active-lanes.yaml`. Each item receives one disposition:

- `integrate`: unique residual work is accepted and must pass its changed-surface gates;
- `preserve`: the work remains valid but is blocked, paused, or owned by an open plan;
- `archive`: history is retained under an explicit durable ref without an active worktree;
- `close`: work is integrated or explicitly superseded and all closure evidence passes;
- `discard_proposed`: unique work appears obsolete or unwanted, but deletion waits for explicit maintainer approval naming the exact ref.

No clean status, old handoff, merged-looking subject, or absent worktree is
sufficient by itself. For each close action, record the local tip, remote tip,
upstream relation, unique-commit count, target ancestry, dirty status, plan
state, custody state, integration receipt, and overlap disposition immediately
before mutation.

### Plan state and Git custody remain separate

Plan 0165 is the primary example. Its branch has no unique source work, so Git
custody may become `INTEGRATED` and its worktree may close while the plan stays
`OPEN` for the governed live acceptance gate. P144 and P156 require the same
distinction to be resolved from their own exit evidence. An ancestor commit
proves source inclusion only; it does not prove installed, provider, tenant, or
operational acceptance.

### Published divergent branches are reconciled without history rewriting

Do not rebase or force-push the published P0240, P157, or documentation
branches. Review them with `git cherry`, patch identity, file-level history,
and a no-commit or merge-tree conflict simulation. Land accepted residual work
on a fresh short-lived integration branch from current `origin/main`, preserving
original commit attribution where practical. Patch-equivalent, superseded, and
still-needed commits must be named separately in the review receipt.

### Source-bearing lanes are serialized

P0240, P157, and the currently dirty canonical worktree intersect on runtime or
navigation surfaces. They are one serialized critical path. Catalog-only and
already-integrated custody corrections may proceed together only after W0
proves that their files do not overlap active uncommitted work.

## Consolidated batch

The campaign contains three outcome groups:

1. establish truthful current custody and close already-integrated worktrees,
   refs, and stale catalog projections;
2. review every unique branch and integrate only the residual commits that are
   still correct on current `main`; and
3. leave the repository with no unexplained worktree, no silently abandoned
   unique commit, a clean active-lane audit or an exact bounded residual ledger,
   and a final remote readback.

The batch deliberately includes catalog repair and Git closure together because
either one alone would leave misleading custody. It does not absorb the live
acceptance work of Plans 0144, 0157, 0162, or 0165. Any source defect found
during reconciliation becomes a bounded repair inside the owning source packet
only when required to integrate already-approved behavior. New features,
architecture changes, and runtime qualification are deferred to their owning
plans.

## Delivery sequence and budget

The campaign has a four-hour active-work ceiling, excluding CI queue time. It
allows at most two source-bearing integration candidates, one conflict-repair
cycle per candidate, and one final closure batch. A failed validation or an
unexplained ownership change consumes the relevant allowance; it is not retried
under a renamed branch.

1. **W0, custody freeze, 20 minutes.** Refresh refs and inventory; identify the
   owner, branch, and intended disposition of every dirty file. Stop without a
   branch, stash, reset, commit, checkout, or overwrite if ownership is not
   explicit. Create and register P168 only from a clean current
   `origin/main`-based worktree.
2. **W1, integrated-custody reconciliation, 40 minutes.** Review P165, P166,
   P167, P144, P155, and P156 against plan exits, ancestry, and receipts. Correct
   catalog metadata and plan state without upgrading source inclusion into live
   acceptance. Reconcile the planning auditor's two-digit lane parser with the
   repository's established three-digit lanes. P165 may close its Git worktree
   and refs only after its catalog custody is truthful on `main`.
3. **W2, unique documentation review, 30 minutes.** Review
   `docs/last30days-x-identity-rejection-20260903` and
   `docs/reddit-handoff-errors-20260902`. Integrate still-current source-backed
   documentation, archive it with an explicit locator, or retain an exact
   `discard_proposed` decision for maintainer approval. Do not blend stale
   operational instructions into the current runbook.
4. **W3, P0240 reconciliation, 80 minutes.** Classify its eight unique commits
   against current `main`, especially the newer Plan 0165 authentication work.
   Build one minimal residual candidate on a fresh integration branch. If no
   residual remains, record patch equivalence or supersession rather than
   manufacturing a merge. Preserve the original branch until the review and
   integration receipt are durable.
5. **W4, P157 reconciliation, 60 minutes.** Classify its four unique commits and
   reconcile the `navigation.rs` overlap after W0 and W3. Merge independently
   useful current behavior only. Keep Plan 0162 and any unproven installed
   acceptance open on a refreshed published custody ref when work remains.
6. **W5, closure, 30 minutes.** Re-run all audits, verify target and remote
   ancestry, remove only exact approved worktrees and refs, prune stale
   administrative metadata, and record the final inventory. One lazy CI status
   read is allowed after integration; active CI babysitting is outside this
   campaign unless separately requested.

If W3 or W4 cannot fit its budget or requires a new semantic design, stop that
packet as `preserve` with a precise blocker. Continue only with disjoint
already-integrated closure work; do not expand the campaign.

## Worker assignments

The primary agent owns W0 through W5 and every final disposition. No worker is
assigned because dirty-work ownership, shared-path reconciliation, integration,
and ref deletion form one serialized custody boundary. A future executor may
use deterministic scripts for mechanical inventory, but may not delegate
acceptance, discard decisions, production effects, or final closure authority.

## Work units and exact gates

| Unit | Scope | Exit evidence | Hard stop |
| --- | --- | --- | --- |
| W0 | Ref refresh, worktree status, dirty-file ownership, lane registration | Clean dedicated P168 worktree from current `origin/main`; exact inventory receipt | Any unexplained dirty file, moving ref, active foreign owner, or overlapping uncommitted work |
| W1 | P165, P166, P167, P144, P155, P156 plan and custody truth | Per-lane plan/custody decision; verified checkpoint ancestry; corrected catalog | Missing plan exit evidence, unverified receipt, or source inclusion confused with live acceptance |
| W2 | Two unique documentation branches | Per-commit relevance decision and link/source check | Private, stale, or unauthoritative operational material cannot be safely integrated |
| W3 | P0240 eight-commit review and residual candidate | Commit disposition matrix; conflict simulation; selected checks; reviewed integration receipt or preserve decision | New product design, unclear consumer authority, second failed candidate, or runtime effect required |
| W4 | P157 four-commit review and residual candidate | Commit disposition matrix; overlap reconciliation; selected checks; refreshed open-lane custody or reviewed integration receipt | Plan 0162 scope expansion, current dirty overlap, second failed candidate, or installed acceptance required |
| W5 | Exact worktree/ref cleanup and final catalog/runbook closeout | Clean intended worktrees; remote readback; ancestry receipts; auditor result; retained residual ledger | Unique commits, dirty state, missing upstream proof, catalog disagreement, or absent deletion authority |

## Review procedure

For each branch under review:

1. capture `git status --short --branch`, `HEAD`, upstream, ahead and behind
   counts, merge base, and local and remote tips;
2. list commits in `origin/main..branch` and classify each as unique,
   patch-equivalent, superseded, documentation-only, or unresolved;
3. compare changed paths against every dirty or active lane before simulating
   integration;
4. inspect the owning plan, runbook entry, catalog record, and any integration
   receipt from their authoritative refs;
5. simulate conflicts without mutating the source branch;
6. freeze one candidate SHA and run `pnpm validation:select -- --base <base>`;
7. run every selected gate and the mandatory language or contract gates below;
8. integrate through the normal reviewed `main` workflow and verify the remote
   target SHA; and
9. only then update custody and consider exact worktree or ref deletion.

No branch is deleted in the same unverified step that merges it. Refresh
`origin/main`, prove the reviewed tip is an ancestor or has an explicit approved
non-integration disposition, and re-check that its worktree is clean first.

## Validation

Every documentation or catalog packet must run:

```text
python .codex/skills/repo-policy-selector/scripts/audit_active_lanes.py \
  --repo-root /home/ecochran76/workspace.local/agent-browser \
  --default-ref refs/remotes/origin/main \
  --catalog-only --json
python .codex/skills/repo-policy-selector/scripts/audit_planning_contract.py \
  --repo-root /home/ecochran76/workspace.local/agent-browser \
  --active-only --json
pnpm validation:select -- --base <packet-base>
```

Documentation checks must also confirm that current operational instructions
remain in `RUNBOOK.md`, priority stays in `ROADMAP.md`, and historical detail
remains in plans or notes. Keep `RUNBOOK.md` at or below 200 lines.

If any candidate changes Rust under `cli/src/` or `crates/`, run:

```text
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
scripts/ci/rust-tests.sh
```

Also run focused tests selected from the changed service, authentication,
workstation-install, navigation, service-request, generated-client, dashboard,
and documentation surfaces. If a service schema, model, formatter, request
metadata, or generated client changes, run its focused contract and generation
parity gates. A prior pass may be reused only when policy 0042 proves the frozen
candidate and all intervening commits were non-invalidating.

## Evidence and exit

| Requirement | Evidence | Exit condition |
| --- | --- | --- |
| Custody is attributable | Refreshed worktree inventory, dirty-file owner, lane and branch mapping | No work begins from unexplained dirty state |
| Integrated lanes are truthful | Plan exit review, checkpoint ancestry, integration receipts, catalog diff | P165, P166, P167, P144, P155, and P156 state source inclusion and residual live gates accurately |
| Unique work is preserved | Per-commit matrix for P0240, P157, and both documentation branches | Every unique commit is integrated, preserved, archived, or explicitly proposed for discard |
| Merges are current | Frozen candidate SHA, conflict receipt, selected validation, reviewed integration receipt | Accepted residual work is on current `origin/main` and passes every touched-surface gate |
| Cleanup is safe | Clean status, zero unresolved unique commits or explicit approved disposition, target ancestry, catalog custody, remote readback | Only exact eligible worktrees and refs are removed |
| Repository is maintainable | Final worktree and branch inventory, active-lane audit, planning audit, bounded residual ledger | Audits pass or every remaining finding has one named owner, reason, and next review condition |
| Runtime boundary is preserved | Negative-effect statement in closeout | No production, browser, profile, credential, provider, tenant, X, RuFresh, or formal release effect occurred |

The campaign closes only when W5 records the exact surviving worktrees and
branches, the exact deleted refs, the integration receipts for accepted work,
the remaining open operational gates, and the final remote readback. A smaller
successful closure batch does not falsely close preserved P0240, P157, Plan
0162, Plan 0165, or another plan whose acceptance remains open.
