# Plan 0207 | Tab Handle Refresh Custody Repair

Date: 2026-09-16

Plan version: 3

State: OPEN, SOURCE COMPLETE, BROWSER ACCEPTANCE BLOCKED PRE-LAUNCH

Consolidation: required

Product lane: PL-BUGFIX

Lane: P207

Work item: `CochranResearchGroup/agent-browser#175`

Branch: `fix/p207-tab-handle-refresh-custody`

Target: `main`

Integration: merge through the protected pull-request workflow after provider-free custody regression, public response parity, and changed-surface local validation

Source baseline: `c855fc33af3129bede1947f55d5b7a49a9500fda`

## Objective

Repair `tab_handle_refresh` so a stale caller handle can reuse only a compatible
physical target whose canonical Service State custody belongs to that caller.
When no caller-bound target exists, `open_if_missing` opens and durably records
a replacement target rather than adopting a foreign or unattributed blank tab.
Every successful response preserves canonical caller attribution and states
explicitly whether duplicate or peer cleanup was attempted.

The repair preserves the existing four repair policies and the nested
`duplicateTargetCleanup` object. It adds the minimum explicit response proof
required by strict consumers without creating a second tab-ownership model.

## Current State

Issue #175 records a production consumer failure on Agent Browser 0.28.0. A
released Books Receipts target was refreshed with `open_if_missing`; the
handler returned `reused_compatible_target` for an `about:blank` page whose
retained trace belonged to another caller. The strict consumer refused the
result because caller identity was unproven and the top-level
`duplicateCleanupAttempted: false` proof was absent. No accounting or provider
effect occurred.

The defect was reproduced and repaired at implementation checkpoint
`c3d69f3e`. The completed source change now:

- treats URL compatibility only as a discovery hint and joins reusable targets
  to canonical Service State custody;
- opens and durably persists a canonical replacement instead of adopting a
  foreign or unattributed blank target;
- limits duplicate cleanup to compatible targets with the same caller custody;
- returns canonical service, agent, and task attribution; and
- exposes top-level duplicate and peer cleanup-attempt proof while preserving
  the nested cleanup receipt.

The focused characterization command is fast and provider-free:

```bash
scripts/ci/rust-tests.sh --focused test_tab_handle_refresh_classifies_live_pages_by_origin
```

It currently passes while proving the harmful predicate: an unrelated
`about:blank` page is classified as compatible. There is no correct existing
unit seam for the full caller-A versus caller-B refresh decision. The first
implementation packet must create that seam and then demonstrate the exact
regression as red before changing policy.

P204 owns the active CI-selection transition and PR #179. P205 owns Service
State model extraction and currently edits model and profile-access files.
P197 and its descendant P206 landed their challenge changes through PR #157.
The shared generated service-request, schema, client, help, and documentation
surfaces are therefore available for P207 contract parity. P207 owns only this
repair and will not edit P205 model sources.

## Consolidated Batch

1. Extract one pure refresh planner from the production handler without
   changing behavior, then add a deterministic caller-A versus caller-B test
   that fails on the current URL-only decision.
2. Join each compatible live page to canonical retained Service State evidence
   and classify it as caller-bound, foreign, or unattributed before selection.
3. Reuse only caller-bound targets. Open and persist a new caller-bound target
   for `open_if_missing` when no eligible target exists.
4. Restrict `replace_duplicates` cleanup to compatible targets proven to share
   the caller's custody. Preserve foreign and unattributed peers.
5. Return canonical caller identity plus explicit cleanup-attempt evidence and
   synchronize the public response contract, generated client, tests, and
   user-facing guidance.
6. Qualify the completed batch with focused provider-free tests and the exact
   required Rust quality checks. Do not dispatch, wait on, or rerun full CI.

## Repair Contract

The pure planner consumes a validated stale handle, current caller context, a
Service State snapshot, live page metadata, desired URL, and repair policy. It
returns one typed decision:

- `ExactTarget`: the original target still exists and current validation proves
  the caller retains access;
- `ReuseCallerBound`: a compatible retained target has canonical custody and
  trace identity equal to the current caller;
- `OpenReplacement`: no eligible caller-bound target exists and the policy may
  open one;
- `Reject`: the policy forbids opening or the evidence cannot prove safe reuse.

URL or origin compatibility is necessary but never sufficient for reuse. A
blank target may be reused only when its canonical retained handle passes the
same caller, route, profile-child, and principal checks required for ordinary
handle commands. A live page without a matching retained record is
unattributed and ineligible.

The handler consumes the plan without rediscovering candidates. Reused targets
return their canonical retained handle. Newly opened targets receive a fresh
handle from current validated command context, are persisted through the
existing serialized repository boundary, and return the canonical persisted
projection. The repair must never overwrite a foreign target record with stale
caller metadata.

For `open_if_missing` and `reuse_compatible`, successful responses include:

- `duplicateCleanupAttempted: false`;
- `peerCleanupAttempted: false`;
- `duplicateTargetCleanup.policy: "preserve"`;
- `duplicateTargetCleanup.attempted: false`;
- zero closed targets and zero failed targets.

For `replace_duplicates`, cleanup is attempted only against the planner's exact
caller-bound duplicate target set. The response retains closed and failed
target evidence and reports foreign or unattributed compatible pages as
preserved, not as cleanup candidates.

## Scope And Non-Goals

In scope:

- browser-lifecycle refresh planning and response construction;
- custody-safe use of existing Service State and profile-child validation;
- exact provider-free Rust regression coverage;
- generated response types, direct client tests, CLI help, README, agent skill,
  and docs parity;
- one narrowly targeted disposable local-browser acceptance when the
  provider-free repair is green and an isolated profile is available.

Out of scope:

- changing `tab_handle_release` semantics or lease policy;
- general Service State extraction, profile-access redesign, or a second
  ownership authority;
- broad duplicate-tab cleanup or cleanup of unattributed processes;
- website-specific Books Receipts behavior;
- credentials, authenticated provider actions, accounting effects, production
  or staging installation, release work, or external challenge effects;
- comprehensive CI, cross-platform matrices, or routine GitHub Actions waiting.

## Delivery Sequence And Budget

### P0 | Pure Decision Seam And Red Regression

- Extract the smallest pure refresh planner while preserving current behavior.
- Keep browser switching, opening, closing, repository mutation, clock reads,
  and event emission outside the planner.
- Add
  `test_tab_handle_refresh_open_if_missing_does_not_adopt_peer_blank_and_reports_no_cleanup`
  with a closed caller-A target, a ready caller-B blank target, and no
  caller-A-compatible live target.
- Run the focused test and retain its expected failing output before repair.

Exit: the exact observed defect is deterministic, provider-free, fast, and red
through production decision code.

### P1 | Ownership-Aware Selection And Canonical Handle

- Join pages to retained tabs and validate caller custody before reuse.
- Prefer exact valid target, then a deterministic caller-bound compatible
  target. Never fall back to foreign or unattributed pages.
- Make `open_if_missing` choose `OpenReplacement` for the issue fixture.
- Return an existing canonical handle for reuse and a canonical persisted
  handle for a newly opened target.
- Preserve service, agent, task, principal, profile, route, and child-access
  attribution.

Exit: the red regression passes; same-caller reuse remains green; foreign and
unattributed targets remain untouched.

### P2 | Cleanup Fence And Response Contract

- Change duplicate cleanup to accept the planner's exact caller-bound set.
- Prove `open_if_missing` and `reuse_compatible` issue zero close operations.
- Add the explicit cleanup booleans while retaining the nested proof object.
- Add response-builder tests for all successful repair policies and typed
  rejection evidence for `reuse_compatible` with no eligible target.

Exit: peer cleanup is impossible through the refresh plan and every success has
machine-checkable cleanup evidence.

### P3 | Public Parity And Acceptance

- Update the generator and generated service-request client type, direct client
  fixtures, help, README, skill, and docs.
- Run focused Rust refresh tests, direct service-request client tests, and the
  validation selector for the exact changed surface.
- Run formatting and strict workspace Clippy once because Rust source changed.
- Run at most one disposable local-browser refresh scenario with an isolated
  profile if the targeted acceptance seam exists. Preserve peer survival and
  residue readback. Do not broaden into the full ignored E2E suite.

Exit: provider-free and targeted local-browser evidence agree on one exact
clean commit, with public contract parity and no external effect.

### Bounds

- Maximum implementation attempts per packet: 2.
- Maximum review and rework cycles: 1.
- Maximum consecutive hardening checkpoints: 1.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 240 active minutes through source repair, focused
  local qualification, publication, and handoff.
- First evidence deadline: within 45 active minutes of implementation start,
  publish the pure seam plus the red regression or record the exact seam blocker.
- No full CI, release build, cross-platform matrix, provider canary, or CI wait
  belongs to this plan.

## Parallel Plan And Worker Assignments

The P207 owner retains the critical path, repair contract, overlap arbitration,
integration, validation interpretation, and completion claim. Subagents stay
inside the P207 worktree and receive no branch, runtime, build candidate, forge,
or live-effect custody.

Completed diagnosis workers:

- `/root/p207_repro_root_cause`: requested `gpt-5.6-sol`, medium. It identified
  the pure planner seam, ran the existing focused characterization, minimized
  the two-caller fixture, and ranked causal hypotheses.
- `/root/p207_contract_tests`: requested `gpt-5.6-luna`, medium. It mapped the
  Rust, generated-client, schema, test, help, skill, and docs contract and
  identified current branch overlaps.

Runtime-reported effective model and effort were not independently exposed.
The primary reconciled both evidence packets against current `origin/main` and
retains the causal and acceptance decisions.

Implementation assignments:

| Worker | Requested route | Exact write scope | Evidence and stop condition |
| --- | --- | --- | --- |
| Rust planner worker | `gpt-5.6-sol`, medium | `cli/src/native/browser_lifecycle.rs` and its direct action tests only | pure plan plus red-to-green regression; stop at any required Service-model edit |
| Contract parity worker | `gpt-5.6-luna`, medium | generator, generated client, direct client test, and explicitly assigned docs after P197 checkpoint | exact field parity and direct deterministic tests; stop on unresolved shared-file drift |
| Primary | strongest current session | plan, catalog, Rust integration, overlap reconciliation, validation, commits, publication, and PR | integrate returned evidence without repeating worker investigations |

Workers may not spawn children. Deterministic test execution, formatting,
diff checks, and validation selection use repository tools rather than another
model.

## Evidence And Exit

| Requirement | Evidence | Current state |
| --- | --- | --- |
| Exact defect loop | red regression first selected caller-B's blank handle; final `scripts/ci/rust-tests.sh --focused tab_handle_refresh` passes 8 tests | complete at `c3d69f3e` |
| Caller-safe reuse | pure planner requires canonical same-caller session, browser, principal, work lease, and profile-child evidence | complete |
| Durable replacement | `test_tab_handle_refresh_opened_replacement_is_canonical_and_durable` reads the persisted caller-owned target back | complete |
| Peer preservation | `test_tab_handle_refresh_replace_duplicates_preserves_peer_targets` excludes caller-B from the cleanup set | complete |
| Caller attribution | canonical reused and persisted replacement handles retain service, agent, task, principal, profile, route, and child-access evidence | complete |
| Cleanup proof | every handler response includes explicit duplicate and peer booleans plus the nested cleanup receipt | complete |
| Public parity | generator, generated type, direct client fixture, MCP schema/help, README, skill, and docs agree; direct client, type, parity, and docs checks pass | complete |
| Rust quality | format check, strict workspace Clippy, JavaScript syntax checks, focused tests, and diff hygiene pass | complete |
| Browser acceptance | two disposable attempts stopped before Chrome launch with `stock_chrome_capability_selection_failed: no_matching_preference_binding`; cleanup completed | blocked pre-launch on missing isolated capability-registry binding |
| External effect | no credential, provider, accounting, staging, production, or release effect | required none |

The provider-free source and public contract are complete. Browser acceptance
did not exercise the repaired path because the isolated test Service State had
no reviewed stock-Chrome preference binding. No Chrome process was launched in
either attempt, and both disposable homes were cleaned. This is an acceptance
environment prerequisite, not evidence of a refresh-path failure. Full CI and
further blind browser retries remain excluded.

Completion requires every non-deferred row above to cite exact-head evidence.
A passing helper test alone cannot prove caller binding, and a successful local
browser switch cannot substitute for provider-free ownership and cleanup
invariants.

## Stop Condition

Stop and replan rather than expanding this repair if correctness requires a new
public action, a Service State schema migration, a profile-access redesign, a
second ownership authority, broad peer cleanup, a production install, an
authenticated site action, or comprehensive CI. If P205 moves the required
canonical handle transition before P207 reaches P1, consume its published
interface instead of recreating the transition in the CLI adapter.
