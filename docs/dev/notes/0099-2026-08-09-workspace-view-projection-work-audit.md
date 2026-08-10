# Plan 0099 Work Audit Cycle 1 | Workspace View Projection

Review mode: `drift_discovery`

Review cycle: 1 of 2

Reviewer role: independent work auditor

Branch: `architecture-deepening-20260809`

Repository base: `ae36b272327982e3227f4dc7c5d6dc5b4b16350c`

Frozen plan:
`docs/dev/plans/0099-2026-08-09-workspace-view-projection-deepening-plan.md`

Frozen plan SHA-256:
`698052922136ce5608f3e54c2bfad3ebc6999ff6c698dbde31ae4cab52fa6514`

Reported Candidate 2 content-manifest identity:
`b351b81e8f8b4c6f7b0c9310f80d332248a9cb69af573e6f0068afb6f062076d`

Audit date: 2026-08-09

## Scope And Method

This review was limited to the reported 19-path Candidate 2 dashboard, test,
package-script, and P12/P45 documentation diff. Concurrent Candidate 1 Rust,
schema, generated-client, and service-request changes were excluded. The audit
did not edit implementation, plans, existing audits, runtime state, commits, or
installed services.

CodeGraph was healthy at review time with 421 indexed files, 14,472 nodes, and
44,162 edges. CodeGraph structural reads covered the projection seam, authority
ledger, selected-context hook, preference controller, workspace-node caller,
viewport caller, and Service inspector caller. Direct reads were limited to
the current edited files surfaced by CodeGraph, the frozen plan and audit,
literal guard searches, the exact Candidate 2 diff, and the unchanged
cross-seam fixture and Service Status schema.

The review verified the following positive evidence:

- one exported `projectWorkspaceViews` computation exists;
- the dashboard-level preference controller owns one storage snapshot and
  increments a same-document revision on writes;
- the selected-context hook supplies `remoteViewRoutes` and projection inputs
  from one `serviceStatus` object;
- selected-only `view-provider` and browser-keyed tile preferences are modeled
  separately, and the direct two-browser fixture passes;
- the direct manual-runtime RDP and public-ingress route fixture passes;
- foreign CDP remains detected and lifecycle read-only, while Borrow input is
  still gated in the viewport renderer;
- both shallow modules are deleted, and literal guard searches find no imports
  or wrappers for them;
- the old named `workspaceViewStreamScore`, `readinessState`, and
  `browserPrimaryViewStream` symbols are absent;
- the viewport no longer fetches its own Service Status snapshot, restores a
  route from `remoteViewRoutes`, or owns local-storage preference state;
- Candidate 2 changes do not alter a Rust or public wire contract;
- P12 and P45 describe the intended single projection path.

Those positive facts do not close the architectural and behavioral findings
below.

## Findings

### `P0099-W1-01` | blocking | The authority ledger is still derived from presentation capability

Criterion: Plan 0099 acceptance criteria 2 and 3 require raw canonical Service
Status facts to become an authority ledger before presentation selection. A
stream URL, provider, readiness, or control input may reduce presentation
availability but may not create lifecycle state or inventory class.

Evidence:

- `deriveWorkspaceViewAuthorityLedger` reads `browser.viewStreams`, calls
  `canOpenViewStream`, `canOpenControlViewStream`, and `routeProofState`, then
  uses those answers to assign `controllable`, `view-only`, or diagnostic state
  and inventory class
  (`packages/dashboard/src/lib/service-workspaces.ts:464-575`).
- The implementation comment says this classification occurs before stream
  selection, but presentation capability is still the classifier input
  (`service-workspaces.ts:459-462,487-551`).
- A direct reproducer using the same browser health and identity produced
  `service-owned-controllable-browser` and `controllable` for a ready CDP
  stream, then `service-owned-diagnostic-browser` and `needs-attention` when
  only that stream readiness changed to `unreachable`.

Consequence: presentation evidence remains an ownership and lifecycle engine.
A ready-looking stream can manufacture a stronger canonical inventory class,
and a presentation failure can rewrite lifecycle authority. This contradicts
the frozen source-authority ledger and the P45 authority direction.

Reproducer:

1. Call `deriveWorkspaceViewAuthorityLedger` with one healthy service browser
   and one ready `cdp_screencast` stream.
2. Repeat with identical browser facts and only stream readiness changed to
   `unreachable`.
3. Compare `inventoryClass` and `lifecycle.state`; both change.

Suggested disposition: accept as blocking. In the one bounded remediation,
make the compatibility classifier depend only on raw lifecycle, inventory,
session authority, proof, route-ownership, incident, job, allocation, and
explicit canonical fields. Move provider URL, readiness, and control-input
interpretation behind the projection seam. Add the crossed ready-looking
presentation versus diagnostic-authority fixture through the real
`deriveWorkspaceNodes` integration path.

Confidence: high.

### `P0099-W1-02` | blocking | Callers use the projection as a shallow selector and reconstruct the answer

Criterion: acceptance criteria 1, 4, 6, 7, and 15 require one deep projection
interface for selection, route, capability, readiness, and presentation. The
workspace-node builder and Service inspector must consume the resulting
projection rather than wrap it only to select a stream and then reinterpret
that stream.

Evidence:

- `deriveWorkspaceNodes` creates an authority ledger, but
  `createBrowserWorkspaceNode` does not receive a projected view. It calls
  `selectPrimaryWorkspaceViewStream`, then independently applies CDP fallback,
  proof gating, route-bound ownership, attention, state, process, and action
  derivation (`service-workspaces.ts:634-725,1170-1366`).
- `selectPrimaryWorkspaceViewStream` creates a synthetic browser and a
  permissive synthetic authority entry, calls `projectWorkspaceViews`, and
  returns only the chosen stream (`service-workspaces.ts:2709-2750`). This is a
  shallow wrapper, not the planned raw snapshot to ledger to projection to node
  call order.
- `service-workspaces.ts` still parses readiness and route proof through
  `viewStreamReadinessState`, `readinessReason`, `routeProofState`, and local
  readiness labeling after the new seam. Renaming or exporting the old
  recursive parser did not remove the competing interpretation.
- The Service inspector repeats the same pattern: `projectedBrowserViewStream`
  fabricates permissive authority and returns only a stream
  (`packages/dashboard/src/components/service-panel.tsx:2572-2607`), while the
  inspector separately derives view readiness, control readiness, route,
  lease, and readiness labels from the raw stream
  (`service-panel.tsx:2752-2799`).
- The projection declares transient preflight, frame, focus, takeover, and
  foreign-Borrow inputs, but the only production constructions in the hook,
  workspace-node wrapper, and inspector do not pass `transient`. The viewport
  instead composes readiness in `deriveWorkspaceViewportReadiness`
  (`packages/dashboard/src/components/workspace-remote-viewport.tsx:1365-1401`).
  The separate viewport-controller seam is legitimate, but the current
  projection interface claims transient ownership that no caller uses.

Consequence: the deletion test is not met. Deleting the new module would move
only stream ranking back into the callers; authority, proof, readiness,
capability, and action complexity already remains there. Workspace nodes,
viewport readiness, and the Service inspector can still report different
answers. A terminal browser with a stale openable stream can also reach the
inspector's fabricated live authority instead of the real ledger.

Reproducer or check:

- Inspect every production `projectWorkspaceViews` call. Two construct
  synthetic permissive authority and discard all output except `stream`; the
  selected-context hook constructs the only full source projection.
- Run the literal guard for `selectPrimaryWorkspaceViewStream`,
  `viewStreamReadinessState`, `readinessReason`, and
  `deriveWorkspaceViewportReadiness`; the competing caller knowledge remains.

Suggested disposition: accept as blocking. In the same bounded remediation as
`P0099-W1-01`, project service browsers once from the real ledger, pass the
projected record into node construction and inspector rendering, and delete
the synthetic selector adapters and caller-local presentation interpretation.
Resolve the unused transient contract explicitly: either construct the one
projection where correlated viewport transient facts are available, or narrow
the projection to source readiness and leave transient readiness solely behind
the already-real viewport-controller seam. Do not keep both claims.

Confidence: high.

### `P0099-W1-03` | blocking | Explicit live blank-tab selection regressed

Criterion: acceptance criterion 16 requires selected-tab and stale-target
recovery behavior to remain covered. The prior viewport behavior honored an
explicitly selected live blank tab while marking it as stale-recovery evidence;
it did not silently switch to another live tab.

Evidence:

- The new `selectTab` defines a selected tab as usable only when it is live and
  nonblank, then chooses the highest-scored alternative when the selected live
  tab is blank (`packages/dashboard/src/lib/workspace-view-projection.ts:476-498`).
- The removed viewport implementation used the explicitly selected tab when it
  was live, independently of blankness, while still recording blankness as
  stale evidence. The removed test asserted that exact behavior.
- A direct projection reproducer with selected active tab `blank` at
  `about:blank` and another active content tab returned:
  `{"selected":"content","recovered":true,"stale":"blank"}`.
- The replacement projection fixture covers a closed stale tab but not an
  explicitly selected live blank tab
  (`scripts/test-dashboard-workspace-view-projection.js:60-82`).

Consequence: a user-selected live target can be replaced by another tab before
focus or control dispatch. The URL is then rewritten to the alternate tab by
the existing stale-selection recovery effect.

Reproducer: project one browser with two active tabs, select the blank tab by
ID, and compare the returned `tabSelection.tab.id` with the explicit selection.

Suggested disposition: accept as blocking. Preserve an explicitly selected
live tab, including a blank one, while retaining the stale/recovery marker if
that is needed for UI evidence. Add a projection-interface regression fixture
for this exact case.

Confidence: high.

### `P0099-W1-04` | blocking | Replace-not-layer tests do not exercise the real authority and caller seams

Criterion: acceptance criterion 17 requires behavior tests through the
projection interface and replacement of obsolete helper tests without losing
the behaviors they protected. The test matrix must also cross the real
authority and caller integration paths.

Evidence:

- The new projection test constructs hand-authored authority objects and calls
  `projectWorkspaceViews` directly. It never calls
  `deriveWorkspaceViewAuthorityLedger` or `deriveWorkspaceNodes`, so it cannot
  detect `P0099-W1-01` or the synthetic selector in `P0099-W1-02`
  (`scripts/test-dashboard-workspace-view-projection.js:17-205`).
- Its caller coverage is limited to source-text assertions that the parent
  passes a projection and that the viewport lacks `/status`, route lookup, and
  local storage (`test-dashboard-workspace-view-projection.js:207-214`). It
  does not prove node, context, viewport, and inspector parity from one fixture.
- The direct foreign-CDP case supplies `foreignBorrowActive: true` but asserts
  that projection control remains false, so it does not exercise how Borrow
  changes only transient input operations (`test-dashboard-workspace-view-projection.js:156-177`).
- The prior view-stream test removed the live blank-tab assertion and many
  route, recovery, readiness, and rendering regressions. Passing reduced tests
  therefore did not detect `P0099-W1-03`.

Consequence: the reported green focused tests validate isolated happy paths
and wiring tokens while the frozen crossed authority, caller parity, Borrow,
and selected-tab contracts can fail.

Reproducer or check: run the new projection test, then run the independent
authority and live blank-tab reproducers from `P0099-W1-01` and
`P0099-W1-03`; the test passes while both contract violations remain.

Suggested disposition: accept as blocking. Extend the replacement packet with
behavioral fixtures that cross the real ledger to projection to node path,
same-snapshot context to viewport path, Service inspector parity, selected-only
preference versus per-browser tiles, live blank-tab selection, manual-runtime
RDP, and foreign Borrow without lifecycle upgrade. Keep private helpers out of
the test interface.

Confidence: high.

### `P0099-W1-05` | blocking validation fixture | The required cross-seam fixture is stale, and its repair belongs in this validation slice

Criterion: acceptance criterion 19 and the frozen validation packet require
`pnpm test:cross-seam-interlocks` to pass or receive an evidence-backed
adjudication. Candidate 2 depends on this shared test for authority and
manual-browser interlocks.

Evidence:

- The command fails before any workspace assertion with
  `cross-seam service status fixture missing schema field manualBrowsers` at
  `scripts/test-cross-seam-interlocks.js:213`.
- The contract requires both `manualBrowsers` and `closedTabProjection`
  (`docs/dev/contracts/service-status-response.v1.schema.json:7-12`). The
  fixture ends its top-level record without either field
  (`scripts/test-cross-seam-interlocks.js:70-211`).
- `scripts/test-cross-seam-interlocks.js`,
  `docs/dev/contracts/service-status-response.v1.schema.json`, and
  `scripts/smoke-schema-utils.js` are byte-identical to `HEAD`; their current
  and `HEAD` SHA-256 values match. Candidate 2 did not cause the drift.
- Git history shows the status contract gained closed-tab and manual-browser
  requirements after the cross-seam fixture's original stabilization, which
  explains the baseline failure but does not validate current Candidate 2
  behavior.

Consequence: the required shared authority test never reaches the Candidate 2
workspace assertions. Leaving it as an unrelated baseline failure would remove
the only named cross-seam evidence from this architecture slice.

Reproducer: run `pnpm test:cross-seam-interlocks` at the reviewed worktree. It
exits 1 at schema validation before `deriveWorkspaceNodes` executes.

Suggested disposition: accept as a blocking validation-fixture repair within
Candidate 2 scope, while explicitly recording that it is not a Candidate 2
implementation regression. Add the exact current empty `manualBrowsers` array
and a valid zero-count `closedTabProjection` record to this fixture, rerun the
test, and adjudicate any subsequent workspace assertion on its own merits.
This is a narrow shared-fixture repair, not permission to change the Service
Status wire contract.

Confidence: high.

## Validation Receipt

The following focused checks passed against the reviewed worktree:

- `pnpm test:dashboard-workspace-view-projection`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-inspector-tab`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:workspace-viewport-controller`
- `pnpm build:dashboard`

The build completed successfully. Next.js emitted the existing static-export
rewrite warnings; TypeScript compilation and static page generation passed.

The following required check failed:

- `pnpm test:cross-seam-interlocks`, exit 1, before workspace execution because
  the unchanged fixture omits required `manualBrowsers`; the same fixture also
  omits required `closedTabProjection`.

Literal Plan 0099 guard results:

- no old named score, readiness, primary-stream, or temporary-wrapper symbols;
- no import or reference to either deleted shallow module;
- no viewport-local selected-context route lookup;
- `remoteViewRoutes` is present in the hook and selected-context input shape.

The broader literal inspection also found the replacement synthetic selector
and remaining caller-local readiness knowledge described in
`P0099-W1-02`.

## Cycle 1 Verdict

Candidate 2 is **not implementation-ready after work review Cycle 1**.

Blocking findings:

- `P0099-W1-01`
- `P0099-W1-02`
- `P0099-W1-03`
- `P0099-W1-04`
- `P0099-W1-05`

Nonblocking findings: none.

Needs-evidence findings: none.

Use one consolidated remediation pass. The bounded packet is:

1. make authority classification presentation-independent;
2. pass real projections into workspace nodes and Service inspection, deleting
   synthetic stream-only wrappers and competing presentation interpretation;
3. restore explicit live blank-tab behavior;
4. replace the missing crossed behavioral coverage;
5. update only the stale shared cross-seam fixture fields required by the
   current schema and rerun the named packet.

Cycle 2 must be `closed_world` and limited to these five IDs plus critical
regressions introduced by their fixes. It must not reopen broad architecture
discovery.

## Audit Effects

- Implementation changes: none.
- Plan or prior-audit changes: none.
- Runtime, browser, tenant, provider, scheduler, installed-service, release,
  commit, and network effects: none.
- Artifact written: this work-audit note only.

## Cycle 2 Closed-World Work Audit

Review mode: `closed_world`

Review cycle: 2 of 2

Review date: 2026-08-10

This terminal review was limited to `P0099-W1-01` through `P0099-W1-05`, the
seven reported remediation paths, and critical regressions introduced by that
remediation. It did not reopen broad discovery. The frozen Plan 0099 SHA-256
remains
`698052922136ce5608f3e54c2bfad3ebc6999ff6c698dbde31ae4cab52fa6514`.

### Cycle 2 Target Identity

The audited Candidate 2 target is the original reported 19-path implementation
with two additional shared test paths changed by remediation:
`scripts/test-cross-seam-interlocks.js` and
`scripts/test-dashboard-workspace-nodes.js`. The original reported
content-manifest identity remains
`b351b81e8f8b4c6f7b0c9310f80d332248a9cb69af573e6f0068afb6f062076d`.

The terminal 21-path manifest is path-sorted. Each present path is represented
as `F<TAB>path<TAB>current-file-sha256`; each deleted tracked path is
represented as `D<TAB>path<TAB>HEAD-file-sha256`. The newline-terminated TSV
contains 21 rows and has SHA-256
`ffe37d3647e2bb45860565f7a3a906c499440aa008563d6b19e8918ba02570b7`.
This representation binds both deleted module identities without adding plan,
audit, or concurrent Candidate 1 paths.

### `P0099-W1-01` | pass | Presentation-independent authority is repaired

The compatibility authority classifier no longer reads `viewStreams` or calls
presentation capability helpers. It derives lifecycle, inventory, proof,
route ownership, diagnostics, and action ceilings from the raw browser,
session, tab, allocation, job, incident, and browser-session-authority inputs.
A direct crossed reproducer that changed only a CDP stream from ready to
unreachable returned identical authority in both cases:
`{"equal":true,"ready":{"inventoryClass":"service-owned-controllable-browser","state":"controllable"},"blocked":{"inventoryClass":"service-owned-controllable-browser","state":"controllable"}}`.

The formerly failing terminal-only selected-context fixture now reports
lifecycle state `controllable` while correctly disabling View and Control with
the source-backed terminal-only reason. The fixture's old expectation that a
presentation blocker rewrite lifecycle state to `needs-attention` is obsolete
under the frozen authority rule.

Residual disposition: closed. The remaining route-ownership assertion failure
is a stale replacement fixture under `P0099-W1-04`: it places
`routeBoundOwnership` inside a `ViewStream`, although the current Rust
`ViewStream` wire model has no such field and the remediated classifier accepts
explicit authority only on the canonical browser record.

### `P0099-W1-02` | fail | Deep consumers landed, but their integration does not compile

The structural remediation is present. `deriveWorkspaceNodes` builds one real
ledger, calls `projectServiceWorkspaceViews`, and passes the resulting complete
projection into browser-node construction. The Service inspector likewise
projects the real status browser/session/tab/allocation/job/incident snapshot
and consumes `canView`, `canControl`, readiness, route summary, and action
reasons from that result. The synthetic `selectPrimaryWorkspaceViewStream` and
`projectedBrowserViewStream` wrappers are deleted. Source readiness belongs to
the projection; viewer-local transient readiness remains behind the separate
viewport-controller seam.

The final `pnpm build:dashboard` nevertheless fails in TypeScript at
`packages/dashboard/src/lib/service-workspaces.ts:821`: the implementation
passes `input.remoteViewRoutes` to the projection, but `WorkspaceNodeInput`
does not declare `remoteViewRoutes`. The same-snapshot hook and selected-context
types do carry the route map, so this is a bounded type-integration omission in
the new real consumer path.

Reproducer: run `pnpm build:dashboard`. Next.js completes compilation and then
TypeScript reports `Property 'remoteViewRoutes' does not exist on type
'WorkspaceNodeInput'`.

Consequence: the dashboard cannot produce a deployable build, and acceptance
criteria 4, 14, and 19 are not satisfied even though the intended caller
topology is visible in source.

Residual disposition: blocking critical regression, attached to this existing
finding rather than opening a new architecture finding.

### `P0099-W1-03` | pass | Live blank-tab compatibility is restored

`selectTab` now preserves an explicitly selected live blank tab, returns
`selectionEvidence: "selected-live-blank"`, and does not mark the selection as
recovered from a closed tab. The replacement projection test contains and
passes the exact live blank-tab case.

Residual disposition: closed.

### `P0099-W1-04` | fail | Crossed coverage exists, but replacement gates remain stale

The new projection test now crosses the real ledger, node, selected-context,
viewport-readiness, inspector, route-ingress, preference, tile, manual-runtime,
foreign-CDP, readiness, and live blank-tab seams. It proves that ready and
unreachable presentation streams preserve identical ledger and node authority
while their presentation actions differ. That focused test passes.

Five required replacement gates still fail because their fixtures or
source-text assertions were not migrated with the removed helper topology:

- `test:dashboard-workspace-nodes` expects stream-nested synthetic
  `routeBoundOwnership` to become canonical browser authority;
- `test:dashboard-selected-workspace-context` expects a terminal-only
  presentation blocker to rewrite lifecycle state to `needs-attention`;
- `test:dashboard-workspace-inspector-tab` requires the obsolete phrase
  `No embeddable`, while the disabled View action correctly reports
  `No workspace view stream is available.`;
- `test:dashboard-inspector-actions` requires the deleted
  `canOpenControlViewStream(primaryViewStream)` caller reconstruction instead
  of `projectedView.canControl`;
- `test:dashboard-browser-table` requires the deleted
  `projectedBrowserViewStream(browser)` wrapper instead of the real
  `projectedViewByBrowserId` consumer.

Direct readback confirmed the no-stream and terminal-only cases remain
fail-closed for View and Control. These failures do not justify restoring the
old reconstruction; they show that the replace-not-layer test migration is
unfinished.

Consequence: acceptance criteria 17 and 19 fail, and required focused gates do
not protect the new public seams without contradictory legacy assertions.

Residual disposition: blocking validation residual.

### `P0099-W1-05` | pass | Shared cross-seam fixture is repaired

The shared Service Status fixture now contains the required empty
`manualBrowsers` collection and a valid zero-count `closedTabProjection`. The
command reaches the workspace assertions and
`pnpm test:cross-seam-interlocks` passes.

Residual disposition: closed. The Cycle 1 adjudication was correct: this was a
required bounded shared-fixture repair, not a Candidate 2 implementation
regression or permission to change the wire contract.

### Cycle 2 Validation Receipt

Passed named checks:

- `pnpm test:dashboard-workspace-view-projection`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:workspace-viewport-controller`
- `pnpm test:cross-seam-interlocks`
- Candidate 2 scoped `git diff --check`

Failed named checks:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-workspace-inspector-tab`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-browser-table`
- `pnpm build:dashboard`

All four literal Plan 0099 guards passed: no old named score/readiness/primary
stream wrapper remains; neither deleted shallow module is referenced; the
viewport has no selected-route reconstruction; and the hook and context types
carry `remoteViewRoutes` from the same status snapshot.

`pnpm validation:select -- --base
ae36b272327982e3227f4dc7c5d6dc5b4b16350c` exited zero. Because that base also
contains concurrent Candidate 1 Rust, schema, client, documentation, and plan
changes, its broad recommendations are not Candidate 2 work-audit scope. No
unrelated selected suites, live checks, runtime publication, or installed-skill
checks were run.

### Cycle 2 Verdict

Final work acceptance: **no**.

Passed stable findings: `P0099-W1-01`, `P0099-W1-03`, `P0099-W1-05`.

Residual blocking findings: `P0099-W1-02`, `P0099-W1-04`.

Nonblocking findings: none.

Needs-evidence findings: none.

This is the terminal work audit. There is no Cycle 3. One bounded remediation
remains: declare and plumb `remoteViewRoutes` on `WorkspaceNodeInput`, update
only the five listed replacement fixtures or assertions to the new authority
and projection contracts, then rerun the six failed named commands. Do not
restore stream-derived lifecycle authority or either deleted wrapper.

### Cycle 2 Audit Effects

- Implementation, plan, runtime, browser, tenant, provider, scheduler,
  installed-service, release, commit, and network effects: none.
- Validation effects: read-only local tests, source guards, manifest hashing,
  whitespace checking, validation selection, and a failed local dashboard
  build attempt. No dashboard runtime was published.
- Artifact effect: this Cycle 2 section was appended to the existing work-audit
  note only.
