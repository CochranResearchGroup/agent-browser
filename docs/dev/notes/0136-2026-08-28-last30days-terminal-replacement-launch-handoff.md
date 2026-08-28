# Last30days Terminal Owner Replacement Launch Handoff

Date: 2026-08-28

Note ID: 0136

Status: LIVE REPRODUCER CONFIRMED | NO REPLACEMENT LAUNCHED

Scope: Agent Browser service acquisition, terminal lifecycle replacement, and
transactional runtime admission

Consumer: Last30days X and LinkedIn direct home-feed retrieval

Authority: DIAGNOSIS AND HANDOFF ONLY | NO UPGRADE TAKEOVER OR FURTHER LIVE
LAUNCH AUTHORITY

## Purpose

Record the most recent failure to launch a replacement browser for the durable
authenticated profile `last30days-facebook`. Last30days needs one retained
browser for direct X and LinkedIn home-feed scraping. The target acceptance is
20 items from X and 20 items from LinkedIn in one bounded tick.

No target page was reached in this incident. The current failure is in Agent
Browser lifecycle and service acquisition, before X or LinkedIn authentication,
page rendering, scrolling, extraction, or filtering.

## Executive Summary

The selected profile is correct and has no live browser, active lease, or live
profile process. Its retained owner is generation 55. The matching lifecycle is
terminal, cleanup is satisfied, the old PID is absent, the stale profile lock is
absent, and the browser projection is absent.

Despite those terminal facts, `service access-plan` refuses replacement with:

```text
terminal_replacement_route_inconsistent
```

The current owner binds these two identities:

```text
browserId=session:last30days-facebook--last30days-facebook
daemonSessionRoute=handoff-a79ef2887412addf
```

`lifecycle_replacement_decision()` currently treats a terminal owner as
replaceable only when `owner.browser_id` equals
`session:{owner.daemon_session_route}`. That equality is false for the current
durable-browser plus daemon-alias representation, even though the owner,
profile, generation, lifecycle, and cleanup evidence agree.

A forced service-owned launch was explicitly authorized by the operator. Two
requests using existing session identities were rejected before effect as
`existing_session_profile_identity_unproven`. A third request used a fresh
service session bound to the same durable profile. It reached Agent Browser but
was rejected before effect because a workstation upgrade transaction was then
draining runtime admission.

That transaction subsequently failed safely and preserved the old generation.
Admission is no longer draining. A fresh no-launch access plan still reports
the original terminal replacement route inconsistency. No forced launch was
retried after the transaction became terminal because the active task changed
to writing this handoff.

## Current Installed And Source Identity

Installed command:

```text
agent-browser 0.28.0
/home/ecochran76/.local/bin/agent-browser
sha256=92d2015dd76cf65c880d6876a6de0fc9e400b67afcafd8eb806d529c2d14b529
installedGeneration=0.28.0-92d2015dd76c-d017d3f4db8a
```

Agent Browser repository:

```text
repo=/home/ecochran76/workspace.local/agent-browser
branch=main
HEAD=b87a85db8b7017357058352eb14983901a5401df
origin/main=b87a85db8b7017357058352eb14983901a5401df
```

The Agent Browser worktree already contained unrelated uncommitted changes
before this note was created:

```text
cli/src/runtime_adoption.rs
packages/dashboard/src/components/service-panel.tsx
packages/dashboard/src/components/workspace-navigator.tsx
packages/dashboard/src/lib/service-browser-row-actions.ts
scripts/test-dashboard-browser-row-actions-render.js
scripts/test-dashboard-browser-table.js
scripts/test-dashboard-workspace-navigator.js
```

Do not absorb, reset, or overwrite those files. Reconcile the active lane or use
an isolated worktree before implementation.

Last30days consumer repository:

```text
repo=/home/ecochran76/workspace.local/last30days-skill
branch=fix/x-linkedin-failure-cause-evidence
HEAD=df7babdd304915f6a05ecfe93c72b9175a99dbc9
plan=docs/dev/plans/0056-2026-08-25-x-linkedin-target-auth-reliability-repair.md
```

## Exact Current No-Launch Reproducer

Run from any directory with the installed binary on `PATH`:

```bash
agent-browser --json service access-plan \
  --service-name last30days \
  --agent-name x-scraper \
  --task-name x-feed \
  --target-service-id x \
  --url https://x.com/home \
  --runtime-profile last30days-facebook \
  --browser-build stealthcdp_chromium \
  --browser-host remote_headed \
  --view-stream-provider rdp_gateway \
  --control-input-provider manual_attached_desktop \
  --display-isolation shared_display
```

Current decisive fields:

```json
{
  "selectedProfile": "last30days-facebook",
  "recommendedAction": "reconcile_lifecycle_owner_for_tab_acquisition",
  "profileReuse": {
    "recommendedAction": "blocked_by_lifecycle_owner",
    "compatibleLiveBrowserCount": 0,
    "sameProfileLiveBrowserCount": 0,
    "activeLeaseCount": 0,
    "reusableBrowserId": null,
    "reusableSessionName": null
  },
  "lifecycleReplacement": {
    "available": true,
    "profileId": "last30days-facebook",
    "ownerId": "owner-9bcae5c95b4ed0e2b978",
    "ownerGeneration": 55,
    "ownerState": "ready",
    "logicalBrowserId": "session:last30days-facebook--last30days-facebook",
    "lifecycleState": "terminal",
    "cleanupObligationState": "satisfied",
    "replacementEligible": false,
    "replacementBrowserId": null,
    "replacementSessionName": null,
    "reason": "terminal_replacement_route_inconsistent",
    "requiredAction": "inspect_lifecycle_owner",
    "registryRevision": 1469,
    "terminalEvidence": [
      "service_reconcile_process_group_absent:95745",
      "service_reconcile_profile_lock_stale_pid_absent:95745",
      "service_reconcile_browser_projection_absent"
    ]
  },
  "serviceRequest": {
    "available": false,
    "blockedByAcquisition": true,
    "acquisitionBlocker": "lifecycle_owner_blocks_replacement"
  }
}
```

The same command with LinkedIn caller labels and
`https://www.linkedin.com/feed/` returns the same profile-level blocker.

## Exact Retained Owner Contradiction

The authoritative user-scoped Service State currently records:

```json
{
  "ownerId": "owner-9bcae5c95b4ed0e2b978",
  "state": "ready",
  "ownerGeneration": 55,
  "browserId": "session:last30days-facebook--last30days-facebook",
  "daemonSessionRoute": "handoff-a79ef2887412addf"
}
```

The generation-55 owner was produced by a cooperative transfer from generation
54. That prior owner used the same durable browser ID with daemon route
`handoff-d2625aa8dab27020`. The transfer therefore preserved durable browser
identity while advancing the daemon session alias.

Current source in `cli/src/native/service_access.rs`, function
`lifecycle_replacement_decision()`, computes:

```text
expected_browser_id = session:{owner.daemon_session_route}
replacement_route exists only when owner.browser_id == expected_browser_id
```

For the current owner, the computed expected browser ID is
`session:handoff-a79ef2887412addf`, not the retained durable browser ID. The
access planner therefore emits `terminal_replacement_route_inconsistent` even
though terminal cleanup is satisfied.

This looks like a contract mismatch between durable browser identity and the
current daemon command route, not proof of a live competing browser. Verify that
interpretation against the intended Plan 0130 owner and route semantics before
changing code.

Relevant existing authority:

```text
docs/dev/plans/0130-2026-08-24-access-plan-owner-reuse-coherence.md
docs/dev/notes/0128-2-2026-08-23-runtime-source-session-selection-hotfix.md
docs/dev/notes/0134-2026-08-26-exclusive-profile-lease-holder-reuse-divergence.md
```

Do not recreate their completed work. This note records the new terminal
replacement case exposed after later cooperative owner transfers.

## Chronology Of The Forced Replacement Attempt

### 1. Supported reconciliation did not repair the owner route

The operator explicitly requested forced replacement. Before launch, the
supported reconciliation command ran:

```bash
agent-browser --json service reconcile
```

Result:

```text
success=true
reconciled=true
browserCount=5
changedBrowsers=0
expiredSessionLeaseCount=0
```

It repaired 11 unrelated orphaned display-allocation records. It did not change
the Last30days owner or create a browser. The subsequent access plan returned
the same terminal replacement blocker.

### 2. Reviewed cleanup surfaces had no applicable candidate

The following dry runs found no Last30days browser replacement candidate:

```bash
agent-browser --json service prune-retained --dry-run \
  --process-exited-browsers \
  --released-sessions \
  --abandoned-sessions \
  --display-allocations

agent-browser --json service gc --dry-run
```

GC reported `candidateCount=0`. No cleanup apply ran.

### 3. Existing session identities failed before launch

The explicitly authorized service request used the same durable runtime profile,
`allowDuplicateProfileLane=true`, and `action=tab_new`. Omitting `sessionName`
defaulted to session `default` and returned:

```text
existing_session_profile_identity_unproven
```

Repeating with historical session
`last30days-facebook--last30days-facebook` returned the same typed error. Both
responses had `data=null` and `success=false`. Neither request launched a
browser.

The profile lease readback separately reports:

```text
leaseId=profile-lease-v1:c53de05eadf2eb8ef67973a2
profileId=last30days-facebook
state=identity_reconciliation_required
principalProvenance=unproven_legacy
blockingIdentityAxes=[legacy_principal_unproven]
recourse=reconcile_principal_identity
authorizedActions=[list, inspect, explain, doctor, watch, reconcile_plan]
```

This is a second identity seam. Do not collapse it into the terminal route
predicate without proving whether the two defects share one cause.

### 4. Fresh replacement session was transiently blocked by upgrade admission

A third service request used fresh session
`last30days-social-replacement-20260828`, still bound to the same durable
profile. Agent Browser returned before effect:

```text
runtime_admission_draining: transaction
'upgrade-9fa6ac5c-4ee4-4028-9172-8f12c779b685' is transferring runtime
ownership at revision 10
```

At that observation, the transaction was `candidate_ready` with three
outstanding owner obligations. No attempt was made to bypass admission, resume
the upgrade, or roll it back.

The transaction later became terminal:

```text
transactionId=upgrade-9fa6ac5c-4ee4-4028-9172-8f12c779b685
state=failed_preserved_old_generation
revision=12
stopReason=candidate_dashboard_presentation_unproven
terminalResult=old_generation_preserved
oldGenerationId=0.28.0-92d2015dd76c-d017d3f4db8a
candidateGenerationId=0.28.0-171ae667834b-7d81e8dd43a6
outstandingOwnerObligationCount=0
safeActions=[inspect]
```

Current workstation status reports `admissionDraining=false`, `ready=true`,
and selected generation `0.28.0-92d2015dd76c-d017d3f4db8a`.

The active upgrade was a transient launch blocker, not the root cause of the
current no-launch access-plan failure.

## Verified Zero-Effect Outcome

Final readback after the failed forced requests showed:

```json
{
  "socialBrowsers": [],
  "replacementSessions": []
}
```

There is no retained browser whose `profileId` is `last30days-facebook`, and no
session named `last30days-social-replacement-20260828`. The current access plan
still has `serviceRequest.available=false`.

Therefore:

- no X or LinkedIn page was opened;
- no browser process was launched for the profile;
- no replacement generation was committed;
- no scraper retry budget was consumed;
- no authenticated profile directory or cookie store was copied, renamed,
  cleared, or deleted; and
- no direct X or LinkedIn 20-item retrieval attempt ran.

## Required Repair Properties

The smallest coherent Agent Browser repair should satisfy all of these
properties:

1. Preserve separate durable browser identity and daemon session route identity.
2. Permit replacement of an exact cleanup-satisfied terminal owner when the
   profile digest, owner generation, lifecycle generation, durable browser ID,
   command route, and terminal evidence form one unambiguous collision-free
   lineage.
3. Do not require a durable browser ID to be the string encoding of a daemon
   alias when cooperative transfer intentionally preserves one and advances the
   other.
4. Keep genuinely inconsistent, missing, ambiguous, live, locked, or
   generation-mismatched owners fail-closed.
5. Keep replacement as an idempotent compare-and-swap transition. Do not add a
   generic force unlock or state-file editing workflow.
6. Keep active workstation upgrade admission authoritative. A launch must still
   fail before effect while `admissionDraining=true`.
7. Make `service access-plan`, MCP and HTTP `service_request`, retained profile
   lease projection, and actual launch admission agree on the same replacement
   route.
8. Do not make `allowDuplicateProfileLane=true` the normal recovery path for a
   terminal owner. The product should expose the exact safe replacement route.
9. Preserve the authenticated profile directory and one-process-per-profile
   invariant.
10. Return enough typed evidence for a consumer to distinguish terminal route
    repair, legacy principal reconciliation, upgrade admission, profile lock,
    and target-site authentication.

## Suggested Red And Green Tests

Use isolated Service State fixtures only. Do not touch the real
`last30days-facebook` profile during source validation.

Add or extend tests for:

- a cleanup-satisfied terminal owner whose durable browser ID differs from its
  valid current daemon session alias after cooperative transfer;
- replacement eligibility for that exact unambiguous lineage;
- a true route collision or unrelated daemon alias remaining blocked as
  `terminal_replacement_route_inconsistent`;
- a missing lifecycle record remaining blocked;
- owner-generation mismatch remaining blocked;
- active upgrade admission rejecting the otherwise valid replacement before
  effect;
- access-plan and service-request route parity for the eligible replacement;
- a legacy unproven profile lease not being silently converted into proven
  principal authority; and
- no duplicate browser process when the terminal owner has no live projection,
  PID, lock, or lease.

Relevant source seam:

```text
cli/src/native/service_access.rs::lifecycle_replacement_decision
```

Use CodeGraph for callers, impact, and the complete owner-transfer flow before
editing. The equality predicate alone is not enough context for a safe repair.

## Validation And Installed Acceptance

On WSL, route every compiling Cargo command through the repository wrapper.
At minimum:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  lifecycle_replacement
pnpm validation:select -- --base <last-known-green-ref>
```

Widen focused tests according to CodeGraph impact and the validation selector.
If any service contract shape changes, apply the complete contract and
documentation matrix in `AGENTS.md`.

Installed acceptance must use a reviewed exact candidate and transactional
installation. After the workstation transaction is terminal and doctor reports
steady state:

1. run the exact no-launch X and LinkedIn access plans;
2. require an eligible service-owned replacement route for the same profile;
3. request one harmless `about:blank` tab through the service control plane;
4. verify one browser process, exact profile ID, owner generation advance,
   usable daemon route, and a valid service tab handle;
5. release that exact tab handle;
6. verify tab release preserved the browser and profile;
7. run final install doctor and a fresh OS process/resource readback; and
8. return control to Last30days for the separately authorized 20 plus 20 feed
   tick.

Do not use X or LinkedIn provider pages as Agent Browser repair fixtures.

## Hard Stops

Stop before any action that would:

- edit `~/.agent-browser/service/state.json` by hand;
- delete, rename, clear, copy, or replace the authenticated
  `last30days-facebook` profile directory;
- kill a process based only on historical PID 95745;
- create a new authentication profile;
- bypass an active workstation upgrade admission gate;
- resume or roll back an upgrade transaction owned by another lane;
- apply broad GC or retained-state pruning without exact reviewed candidates;
- launch a duplicate same-profile browser as a product workaround;
- open X or LinkedIn or consume a Last30days feed retry without current
  consumer authority;
- absorb or reset the pre-existing dirty Agent Browser worktree changes; or
- release, push, merge, or open a pull request without explicit authority.

## Suggested Skills

- `graphiti-discovery` for source-backed prior decisions in group
  `agent_browser_main` when the MCP endpoint is healthy;
- `codegraph-workspace` for owner-transfer, access-plan, request-routing, and
  impact analysis;
- `diagnosing-bugs` for a bounded causal investigation;
- `tdd` for the terminal-owner and route-alias regression fixtures;
- `agent-browser-service` for installed access-plan and service-owned acceptance;
- `repo-policy-selector` before implementation, integration, or release work;
  and
- `handoff` when returning the repaired installed runtime to Last30days.

## Graphiti Discovery Status

At note creation, `graphiti-runtime doctor` reported:

```text
doctor=degraded
mcp_http=down
falkordb=healthy
falkordb_persistence=healthy
inspector_local_api=healthy
```

No Graphiti repair was authorized or attempted, and no Graphiti result was used
as current runtime evidence. Retry discovery against group `agent_browser_main`
when the MCP endpoint is healthy.

## Next Recommended Action

Create or select an isolated Agent Browser worktree after reconciling the active
dirty lane. Use CodeGraph to trace terminal owner transfer through access-plan
request normalization and launch admission. Add the durable-browser plus daemon
alias fixture first, observe it fail with
`terminal_replacement_route_inconsistent`, and repair the smallest shared
identity predicate without weakening generation, cleanup, lease, or upgrade
admission checks.
