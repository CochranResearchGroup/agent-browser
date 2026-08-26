# Plan 0133 | Operator-Visible Window Focus Postcondition

Date: 2026-08-25

State: DEVELOPMENT ACCEPTED

Execution state: `development_accepted`

Lane: P133

Source baseline: `d07b481159a0de8a3e69b0dd7a327d8fb7fef8e9`

Branch: `fix/operator-visible-window-focus-postcondition`

Worktree: `/home/ecochran76/workspace.local/agent-browser-plan0133`

Authority: SOURCE AND DEVELOPMENT-RUNTIME REPAIR. PRODUCTION READ-ONLY.

Depends on:

- `docs/dev/notes/0133-2026-08-25-operator-visible-window-focus-gap-handoff.md`;
- accepted Plan 0124 presentation evidence;
- accepted Plan 0131 controlled development X11 provider;
- the existing browser, profile, session, tab, route, display, viewer, controller,
  and durable handoff identities.

## Objective

Make operator-visible readiness a verified postcondition for the exact retained
browser. Reattachment and durable handoff resolution may restore presentation
only when current authority permits it, and they must independently re-observe
the desktop after any focus or maximize request.

## Ranked Diagnostic Hypotheses

1. `visible_browser_window_proof` accepts a coarse
   `browser_window_visible` string and discards native scene state.
2. Reattachment and durable checkout convert that coarse proof directly into
   `ready` without a focus-and-reobserve phase.
3. Capture-ready X11 evidence already contains most required scene semantics,
   but the operator handoff path does not consume it.
4. Presentation-capacity policy protects active human use, but automatic
   window restoration is not currently admitted through that policy.

## Frozen Contract

Ready requires process-bound proof that the retained browser window is mapped,
non-minimized, on the visible workspace, active or topmost, within approved
geometry, and unobscured. Missing evidence is a blocker, not implied success.

Automatic focus and maximize may occur only for the same retained browser when
viewer and controller authority permit presentation staging. Active human
control wins. A focus acknowledgement is never readiness evidence. The desktop
must be observed again and the independent postcondition must pass.

Browser acquisition, profile ownership, session, tab, route, display, and
durable handoff identity must remain unchanged. No duplicate browser is an
allowed recovery mechanism.

## Execution Slices

### Slice A | Red Fixtures

- Reject mapped but minimized evidence.
- Reject a browser on a non-visible workspace.
- Preserve typed failure when required process-bound evidence is missing.

### Slice B | Process-Bound Proof

- Adapt the Plan 0124 X11 scene model for operator presentation.
- Bind observation to the selected browser process and route display.
- Project a typed operator-presentation blocker instead of `ready` on failure.

### Slice C | Authorized Focus And Re-observation

- Admit restoration through current viewer and controller authority.
- Focus and maximize the exact retained window only when staging is allowed.
- Re-observe independently after the mutation.
- Keep a successful focus followed by failed observation in a blocked state.

### Slice D | Convergence And Identity

- Apply the same postcondition to reattachment and durable handoff resolution.
- Keep standalone `view_focus` as an explicit repair action.
- Prove browser, profile, session, tab, route, display, and handoff identities
  unchanged.

### Slice E | Validation And Development Acceptance

- Run focused Rust tests through `scripts/ci/cargo-safe.sh`.
- Run formatting, Clippy, selected contract checks, and documentation checks.
- Use one disposable retained browser in the isolated development runtime.
- Record pre-repair blocked evidence and post-repair independent readiness.

## Hard Stops

- No production runtime install, restart, or provider mutation.
- No duplicate browser, profile replacement, or unrelated route cleanup.
- No automated staging while an active human controller owns the scene.
- No private page content, credentials, tenant identity, or raw provider URL in
  fixtures or evidence.
- No readiness based on a focus command acknowledgement alone.

## Completion Gate

Plan 0133 completes only when provider-free tests and one development-runtime
acceptance prove the stronger postcondition, identity invariants, human control
priority, and cleanup. Production remains unchanged.

## Acceptance

Source validation and isolated development-runtime acceptance passed. The
qualified runtime artifact, exact retained identity readbacks, strict
operator-presentation predicates, reattach convergence, cleanup proof, and
production boundary are recorded in
`docs/dev/notes/0133-1-2026-08-25-operator-visible-window-focus-development-acceptance.md`.
