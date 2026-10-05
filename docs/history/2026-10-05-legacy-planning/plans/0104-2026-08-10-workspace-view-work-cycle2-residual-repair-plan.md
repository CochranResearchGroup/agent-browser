# Plan 0104 | Workspace View Work Cycle 2 Residual Repair

Date: 2026-08-10

State: APPLIED BOUNDED REPAIR

Authority:

- `docs/dev/plans/0099-2026-08-09-workspace-view-projection-deepening-plan.md`
- `docs/dev/notes/0099-2026-08-09-workspace-view-projection-work-audit.md`
- terminal findings `P0099-W1-02` and `P0099-W1-04`

## Bound

Candidate 2 exhausted its two implementation-audit cycles with two narrow
blocking residuals. The deep projection was already the authority for browser
view capability and actions, but one input type omitted the route descriptor
map and five source-contract tests still asserted deleted selection topology or
the prior presentation-derived authority shape.

This packet resolves only those closed findings. It is not a third audit
cycle, does not reopen the projection architecture, and does not add a second
view-selection path.

## Applied Correction

1. Add `remoteViewRoutes` to `WorkspaceNodeInput` so the node, inspector, and
   selected-context callers can project route-only descriptors from the same
   immutable status snapshot.
2. Normalize nullable workspace route records at the projection boundary into
   the stricter `ServiceViewStream` input shape.
3. Update authority fixtures to place `routeBoundOwnership` and
   `operatorVisibleProof` on the canonical browser authority record rather than
   under `viewStreams`.
4. Update caller contract tests to require the real
   `projectedViewByBrowserId` and `ProjectedWorkspaceView` capability fields.
5. Preserve an explicitly selected live blank tab as `selected-live-blank`,
   while retaining the real provider URL and controllable projected state.
6. Update the readiness-strip assertion to require the projected view input,
   not the deleted primary-stream selector.
7. Preserve unresolved incident precedence in the selected-workspace chat
   packet fixture instead of allowing stream presentation to overwrite
   `needs-attention` with `controllable`.

The repair intentionally keeps source readiness inside the projector and
transient interaction readiness inside the viewport controller.

## Verification And Handoff

The bounded correction passed:

- `pnpm test:dashboard-workspace-nodes`;
- `pnpm test:dashboard-selected-workspace-context`;
- `pnpm test:dashboard-workspace-inspector-tab`;
- `pnpm test:dashboard-inspector-actions`;
- `pnpm test:dashboard-browser-table`;
- `pnpm test:dashboard-selected-workspace-chat-packet`;
- `pnpm build:dashboard`;
- `git diff --check`.

The distinct Candidate 2 tester must re-run the complete Plan 0099 targeted
packet, its deletion guards, the cross-seam interlock, the dashboard build, and
an exact scoped content identity. No further work audit is authorized. A test
failure is reported as a test receipt or a bounded implementation blocker, not
as a reason to restart architecture review.

Effects: the scoped dashboard projection, caller, test-contract, and planning
files only. No runtime, browser, installation, tenant, commit, push, release,
or live-system effect occurred.
