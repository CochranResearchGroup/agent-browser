# Plan 0177 | Development Governance And Repository Readiness

Date: 2026-09-13

State: PLANNED

Consolidation: required

Lane: P177

Product lane: PL-PLATFORM

Branch: `platform/plan-0177-development-readiness`

Target: `main`

Integration: merge through the repository review workflow

## Objective

Establish issue-backed multi-session development governance, reconcile every
current worktree and active planning authority, and leave the repository plus
its installed and development runtime boundaries in a truthful maintenance
state before serious feature development resumes.

## Current State

The repository has stable product lanes and mature planning, Git, validation,
runtime, and multi-agent policies, but it lacks one operating model that assigns
top-level session ownership and one coordinator role across those policies.
Last30days policy commit `901c5216fb0045961d29ba877f9c45acd94e7107`
provides a useful multi-session model. Agent Browser needs a local adaptation
that preserves its five product lanes, six-worktree portfolio limit, shared
runtime constraints, and separate production-effect gates.

The installed policy-selector bundle is `v0.1.24` at source commit
`9caf5708e5aa48cc0cf6264baa7065fc335d193e`. The latest reviewed release is
`v0.1.25`; its only functional change repairs installation of a referenced
model-selection policy. The current selector classifies Agent Browser as an
`operations-platform` repository in `patch-missing` mode and recommends the
missing `forge-issue-reporting` and `github-issue-operations` modules. The
unreleased shared `collaborative-development-workflow` module at
`8f4273d2b4ab6f755fa0586fc57ae896a885bf3f` assumes multiple accountable human
contributors and a fully active shared tracker, so it remains deferred.

GitHub Issues are disabled on `CochranResearchGroup/agent-browser`. The current
authenticated actor has `ADMIN` repository capability, but capability does not
establish operator authority for changing repository settings, labels, or
issues. The repository has only GitHub's default labels and no issue templates
or repo-local target registry.

Current Git custody is:

| Worktree or ref | Current evidence | Required disposition |
| --- | --- | --- |
| Canonical path `/home/ecochran76/workspace.local/agent-browser` | Topic branch `fix/plan-0240-noop-service-state-revision` at `2d1cc3ff`; PR 63 merged it to `origin/main` as `d32919f8`; one untracked profile-selection field note remains | Preserve and route the note, restore the canonical path to clean current `main`, then retire the merged topic refs after exact ancestry and remote checks |
| `/home/ecochran76/workspace.local/agent-browser-turnstile` | `feature/turnstile-desktop-challenge` at `17791566`; six commits ahead and 51 behind current `origin/main`; one local commit is not on the remote | Preserve a remote checkpoint, reconcile current `main`, retain P169 as the only plan identity, then reach either integration-ready or paused-ref custody without adding new feature scope |
| Plan 0177 worktree | Clean branch from `origin/main` at `d32919f8` | Publish this plan for review without modifying the other worktrees |

The active-lane auditor reports that P169 and the Turnstile branch's conflicting
P173 identity are absent from the canonical default-ref catalog. The active
planning audit reports 37 findings: stale open-plan state, missing current-state
or roadmap/runbook wiring, one missing Plan 0162 worker assignment, and open
roadmap lanes P13 and P69 without actionable plan coverage. These are
housekeeping inputs, not permission to rewrite historical outcomes.

Graphiti discovery was healthy in `agent_browser_main` but returned no current
facts for this campaign. Git refs, GitHub readback, policies, plans,
`ROADMAP.md`, `RUNBOOK.md`, and `docs/dev/active-lanes.yaml` are authoritative.

## Consolidated batch

1. Adopt the missing multi-session, forge-reporting, and GitHub issue-operation
   policy contracts from reviewed sources.
2. Establish GitHub Issues as the shared work-item and dependency surface while
   preserving plans, active lanes, Git, tests, receipts, and runtime readback as
   separate authorities.
3. Reconcile the merged no-op Service State worktree and the divergent
   Turnstile worktree without losing the untracked field note or unpublished
   Turnstile checkpoint.
4. Triage the 37 active planning-audit findings and reconcile every retained
   actionable plan into a truthful current state and issue-backed locator.
5. Compact stale branch and catalog custody, verify canonical repository and
   runtime readiness, and publish one final maintenance receipt.

## Non-Goals

- Implementing new CAPTCHA, authentication, disposable-profile, recipe, or
  architecture behavior
- Repairing new Turnstile findings inside this campaign
- Retrying protected providers, authenticated consumers, CAPTCHA challenges,
  X evaluations, BILL operations, or other tenant workflows
- Installing, replacing, restarting, or releasing the production binary
- Resetting profiles, credentials, cookies, browser data, or Service State
- Deleting an unmerged ref, dirty worktree, untracked artifact, or runtime
  process merely to obtain a clean report
- Migrating every closed historical plan into GitHub Issues
- Adopting the unreleased shared multi-user collaboration module before its
  human-ownership assumptions are true and a reviewed release includes it

## Authority and effect boundaries

Writing, reviewing, and merging this plan does not authorize GitHub provider
mutations. Execution must obtain one explicit operator decision covering the
exact repository-setting, label, issue, assignment, and Project actions to be
performed. Read-only GitHub capability inspection and duplicate searches may
proceed before that decision.

After that gate, issue enablement, creation, labeling, editing, and closure are
serialized through one coordinator. Every mutation uses an exact target,
idempotency marker, and post-write readback. An ambiguous response is reconciled
by search before any retry.

Local Git cleanup may remove a branch or worktree only after tracked and
untracked cleanliness, published custody, target ancestry or patch-equivalence,
and the intended disposition are proven. Remote branch deletion remains a
separate explicit cleanup action recorded in the execution checkpoint.

Runtime inspection is read-only. Any required install, supervisor, browser,
profile, process, or state mutation becomes a separately scoped action with its
applicable runtime authority and evidence; this campaign does not infer it from
housekeeping authority.

## Delivery sequence and budget

Critical path:

```text
W0 freeze and census
  -> W1 policy adoption
  -> W2 issue foundation and migration
  -> W3 canonical worktree closure
  -> W4 Turnstile disposition
  -> W5 planning and catalog reconciliation
  -> W6 repository and runtime readiness
  -> W7 final review and closeout
```

W3 and the read-only portion of W5 may prepare in parallel after W2 defines
stable work-item locators. Shared roadmap, runbook, policy, catalog, and issue
metadata changes remain coordinator-owned and serialized. Turnstile source
changes occur only in its existing worktree.

Overall active-effort ceiling: six hours across at most three execution
sessions. Checkpoint at every work-unit boundary or after 30 minutes of active
work. Stop a tactic after two checkpoints without outcome progress. Permit one
policy review/rework cycle, one issue-taxonomy correction cycle, and one
Turnstile integration review. Run at most one expensive Rust presubmit cycle,
and only if the existing Turnstile implementation is selected for integration.
No production build, installation, provider replay, or live browser acceptance
is budgeted.

If the Turnstile branch needs substantive product repair, preserve it as
`PAUSED_REF`, create or update its issue with the missing acceptance evidence,
and finish this campaign without performing that feature work. A safely paused
and discoverable feature does not block repository readiness.

## Work units

### W0 | Freeze and authoritative census

1. Pause new substantive development intake.
2. Fetch the configured origin without deleting refs and record default branch,
   worktree, local/remote branch, pull request, status, and divergence evidence.
3. Freeze the Plan 0177 baseline and verify that no worktree changed during the
   census.
4. Record exact dirty paths and unpublished commits. Never use timestamps or
   branch names as proof of ownership.
5. Run the policy selector, goal audit, active planning audit, and bounded
   active-lane audit. Preserve their initial findings as the campaign ledger.

Exit: every current worktree, actionable branch, planning finding, and external
effect gate has an explicit owner and proposed disposition.

### W1 | Policy source and multi-session operating model

1. Upgrade the installed selector from reviewed release `v0.1.24` to
   `v0.1.25`, verify its release manifest, and rerun deterministic selection.
2. Add one repo-local multi-session operating-model policy, expected as
   `0047-multi-session-development-operating-model.md`, adapted from Last30days
   commit `901c5216`. It must define:
   - one top-level session per substantive product lane;
   - one separate coordinator for intake, priority, shared contracts,
     dependency joins, active-lane reconciliation, and final integration;
   - one session and one branch per worktree;
   - coordinator ownership of roadmap, runbook, catalog, and shared schemas;
   - routine delegation depth of one unless a bounded plan proves otherwise;
   - the existing five product lanes and six-worktree portfolio limit;
   - per-lane development-runtime isolation as the target, with serialized
     shared-runtime use until isolation is proven; and
   - serialized authenticated or provider canaries.
3. Adopt the pinned `forge-issue-reporting` and `github-issue-operations`
   modules as the next unique repo-local policy identities.
4. Wire all three policies into `AGENTS.md` with exact read triggers. Do not
   duplicate their bodies in the entrypoint.
5. Add or update deterministic policy identity and wire-in tests. Record the
   Last30days source, selector release, local overrides, and deferred shared
   collaboration module in one dated adoption note.

Exit: exactly one active file owns each policy identity, selector validation is
clean, and policy installation is distinguished from later enacted evidence.

### W2 | GitHub issue foundation and bounded migration

1. Preflight `github.com/CochranResearchGroup/agent-browser`, current actor,
   admin capability, Issues availability, security route, templates, labels,
   and duplicate-search behavior.
2. After the explicit provider-mutation gate, enable Issues for this repository.
3. Add a non-secret target registry with the exact owned repository, allowed
   actions, security route, and normalized label mappings.
4. Add concise issue forms or templates for defects, product work, operational
   gates, and governance work. Keep plan bodies in plans rather than copying
   them into issue descriptions.
5. Establish labels for the five product lanes, work-item state, work type, and
   live-effect posture. Reuse suitable default GitHub labels; create only the
   missing reviewed labels authorized by the operator.
6. Create the minimum initial issue set after duplicate searches:
   - P177 governance and repository readiness;
   - P169 Turnstile challenge work;
   - the unauthenticated access-plan profile-selection defect;
   - each still-actionable operational or product gate retained by W5.
7. Do not manufacture retroactive issues for closed work whose PR and plan
   already provide sufficient traceability.

Exit: every retained active item has one stable GitHub locator, owning product
lane, state, dependencies, effect boundary, and verified creation receipt.

### W3 | Canonical worktree and merged no-op repair closure

1. Verify PR 63, merge commit `d32919f8`, required CI checks, and exact ancestry
   of `fix/plan-0240-noop-service-state-revision`.
2. Review the untracked profile-selection note as separate user-owned custody.
   Preserve it on an issue-bound branch or reviewed documentation change before
   switching the canonical checkout. Split its stock-Chrome capability and
   challenge observations into linked platform or challenge items when they do
   not belong to the profile-selection defect.
3. Restore `/home/ecochran76/workspace.local/agent-browser` to local `main`,
   fast-forward it to exact `origin/main`, and verify a clean status.
4. Remove the merged no-op local worktree custody and local or remote topic refs
   only after the note is preserved and ancestry plus remote readback pass.
5. Record the no-op repair as `PL-BUGFIX` with `PL-PLATFORM` as the affected
   component. Do not create a new active issue solely for its already-completed
   implementation.

Exit: the canonical path is clean current `main`; the field note is recoverable
and issue-routed; no merged topic worktree or ambiguous branch custody remains.

### W4 | Turnstile branch reconciliation and disposition

1. Preserve commit `17791566` under an exact recoverable remote ref before
   history or integration work.
2. Make P169 and Plan 0169 the sole active identity. Remove or supersede the
   conflicting CAPTCHA P173 references without altering canonical closed P173.
3. Merge current `origin/main` into the already-published Turnstile branch.
   Resolve shared roadmap, runbook, catalog, service-contract, client, and
   documentation surfaces under coordinator arbitration.
4. Review the complete diff against the P169 issue, plan, product-lane boundary,
   current platform contracts, and changed-surface validation selector.
5. Choose exactly one disposition:
   - `INTEGRATION_READY` when existing behavior is coherent and all required
     checks pass;
   - `PAUSED_REF` when substantive feature repair or missing acceptance remains;
   - `ARCHIVED` only when current `main` already represents all useful work and
     exact evidence supports retirement.
6. If integration-ready, publish the reconciled checkpoint and open one PR. If
   paused, publish the checkpoint and issue evidence without adding new feature
   behavior.

Exit: one issue, one plan identity, one published checkpoint, one truthful
custody state, and no hidden local-only commit.

### W5 | Planning, roadmap, runbook, and catalog reconciliation

1. Adjudicate all 37 initial active-planning findings. For each plan, inspect
   current Git and acceptance evidence and choose `OPEN`, `BLOCKED`, `CLOSED`,
   `CANCELLED`, or an explicit successor. Do not infer closure from source
   integration alone.
2. Add truthful `Current State` and canonical wiring only for plans that remain
   actionable. Close or supersede stale plans instead of preserving artificial
   activity.
3. Repair Plan 0162's missing worker assignment and reconcile Plans 0162, 0163,
   and 0165 with their real current gates.
4. Resolve roadmap lanes P13 and P69 through an actionable retained plan or an
   explicit closed, cancelled, or superseded disposition.
5. Reconcile P144, P157, and P165 so their open operational gates remain
   distinct from already-integrated source and historical branch custody.
6. Keep `docs/dev/active-lanes.yaml` limited to currently active, paused, or
   integration-ready Git custody. Remove fully closed integrated projections
   only after their plans retain durable receipts.
7. Add work-item locators to every retained substantive lane. Change catalog
   work-item tracking to required only after the migration is complete.
8. Compact `RUNBOOK.md` under 200 lines while preserving current stops, evidence
   links, and archive behavior.

Exit: the active-only planning and active-lane audits pass without hiding new
findings behind a broad legacy baseline.

### W6 | Repository and runtime readiness

1. Verify canonical `main` equals `origin/main`, every remaining worktree is
   clean or explicitly paused with published custody, and every remaining local
   or remote branch has a documented disposition.
2. Inspect open pull requests, completed CI for integrated housekeeping, stale
   remote branches, and current branch protection or review behavior. Do not
   delete a ref solely because it is old.
3. Run read-only production and development install doctor, service status,
   supervisor identity, listener, browser/process census, and development
   namespace checks. Treat service status alone as insufficient process proof.
4. Confirm there is no false mid-install state, stale transaction owner,
   unmanaged development service, or undocumented installed-source divergence.
5. If runtime coherence fails, create a typed issue with exact evidence and
   quarantine the affected runtime path. Do not repair, restart, reinstall, or
   clean processes inside this campaign.
6. Confirm normal development may resume without borrowing production profiles,
   credentials, browser identity, or provider capacity.

Exit: repository custody is coherent and runtime state is either read-only
healthy or explicitly quarantined behind a named issue that does not endanger
isolated development.

### W7 | Review, integration, and closure

1. Freeze the final documentation and housekeeping candidate.
2. Run policy, planning, lane, link, YAML/JSON, issue-receipt, and patch-hygiene
   checks. Run code gates only for code or executable configuration actually
   selected for integration.
3. Perform one closed-world review against this plan's acceptance table.
4. Merge through the repository review workflow and verify exact `origin/main`.
5. Close P177 and its issue only after policy wiring, issue migration, Git
   custody, planning status, and runtime-readiness claims agree.
6. Record the first enacted multi-session policy feedback after the campaign,
   then consider source-backed Graphiti memory under the existing memory policy.

Exit: serious development intake may resume under issue-backed, lane-owned,
coordinator-reconciled custody.

## Worker assignments

| Owner | Scope | Write boundary | Evidence and stop condition |
| --- | --- | --- | --- |
| Coordinator | Critical path, policy adoption, issue mutations, shared authorities, final integration | Plan 0177 branch plus coordinator-owned roadmap, runbook, catalog, policy, target-registry, and issue-template surfaces | Stops at any unapproved provider mutation or contradictory custody claim |
| Git custody reviewer | Read-only worktree, ref, PR, ancestry, and divergence inventory | None | Returns exact refs and discrepancies; never merges, deletes, switches, or cleans |
| Planning reviewer | Classify the 37 findings and propose state dispositions | None during review; coordinator applies accepted shared-doc changes | Stops when evidence cannot distinguish open, closed, or superseded state |
| Turnstile lane owner | P169 branch reconciliation and validation | Existing Turnstile worktree only | Stops at substantive new feature work and publishes either integration-ready or paused custody |
| Runtime reviewer | Fresh read-only production and development census | None | Stops before any install, restart, process cleanup, profile, or state mutation |

No subagent or parallel worker was used to author this plan. Future execution
may use separate top-level lane sessions after W1 is integrated. Each worker
must receive a frozen packet with exact scope, evidence, and stop condition;
the coordinator retains issue effects, shared-doc writes, integration, and
acceptance.

## Evidence and exit

| Requirement | Required evidence | Completion condition |
| --- | --- | --- |
| Policy provenance | Selector `v0.1.25` manifest, Last30days `901c5216`, unique local policy identities, AGENTS wiring, adoption note | Multi-session plus issue modules are installed and actively wired; unreleased collaboration module remains explicitly deferred |
| Issue foundation | Enabled GitHub Issues readback, exact target registry, reviewed labels/templates, mutation receipts | Tracker is usable without becoming the sole implementation or completion authority |
| Work-item migration | Canonical issue URLs and readback for P177, P169, the profile-selection defect, and retained actionable gates | Every active item has one stable locator and no duplicate creation |
| No-op worktree | PR 63 checks, merge `d32919f8`, ancestry, clean canonical `main`, note custody receipt | Merged branch and worktree are closed without losing the field note |
| Turnstile worktree | P169-only identity, published checkpoint, current-main reconciliation, selected validation, issue state | Branch is integration-ready, paused, or archived with no local-only work |
| Planning debt | Finding-by-finding disposition and fresh audits | Active planning and lane audits pass without masking new findings |
| Catalog and runbook | Work-item-linked active entries and runbook at or below 200 lines | Current authorities are compact, consistent, and source-backed |
| Repository readiness | Clean/equal canonical main, branch/worktree disposition table, PR and CI readback | No hidden dirty state or ambiguous Git custody remains |
| Runtime readiness | Fresh doctor, supervisor, listener, process, installed identity, and development-isolation readback | Runtime is coherent or explicitly quarantined without unsafe cleanup |
| Final integration | Review packet, validation table, merged commit, current remote readback | P177 and its issue close truthfully; serious development may resume |

## Definition of done

This plan is complete only when issue-backed development governance is enacted,
the canonical checkout is clean current `main`, every other worktree has one
published and truthful disposition, all retained active plans and lanes have
stable issue locators, current planning and lane audits pass, and runtime
readiness has fresh source-bound evidence or an explicit safe quarantine.

Policy files, issue creation, a clean Git report, or a healthy service status
cannot establish completion independently. The plan may close with Turnstile
paused rather than merged, but it may not close with an unpublished commit,
duplicate P173 identity, unowned dirty note, or ambiguous runtime state.

## Plan-authoring checkpoint P0177-C01 | 2026-09-13

State remains `PLANNED`. This plan was authored in a clean dedicated worktree
from `origin/main` at `d32919f8` without modifying the merged no-op or Turnstile
worktrees. GitHub Issues remain disabled and no setting, label, issue, branch
cleanup, runtime, browser, profile, provider, installation, or tenant mutation
occurred.

Next action: review and merge this planning packet. Execution begins with W0
and must stop before W2 provider mutations until the operator grants the exact
GitHub issue-setting and issue-operation authority.
