# Operator-Visible Window Focus Gap Handoff

Date: 2026-08-25

Status: OPEN FIELD DEFECT

Scope: Agent Browser operator presentation only

Authority: SOURCE AND DEVELOPMENT-RUNTIME REPAIR | PRODUCTION READ-ONLY UNLESS SEPARATELY AUTHORIZED

## Purpose

Continue one bounded repair for a retained remote-headed browser whose route,
display, browser, and session were reported ready while the human operator
could not see the browser window. The next agent should strengthen the
operator-presentation contract without launching a replacement browser,
changing profile ownership, or treating the incident as a browser-acquisition
or route-capacity failure.

Tenant, credential, page-content, raw provider URL, and private profile details
are intentionally omitted. The live consumer was BooksReceipts, but the defect
is provider-neutral Agent Browser behavior.

## Executive Summary

An installed Agent Browser `0.28.0` retained browser was healthy and attached
to its existing RDP route. During a human login attempt, the browser window was
minimized or displaced from the operator's visible desktop. A no-launch
`service_remote_view_browser_reattach` returned all of the following:

- `status: reattached`;
- `attachability.state: attached_ready`;
- `attachability.proofState: ready`;
- `displayContentState: browser_window_visible`;
- route, display, browser, and session agreement;
- an active viewer.

Those fields did not match the human-visible result. A subsequent no-launch
`view_focus` against the same retained browser and daemon session returned:

```json
{
  "broughtToFront": true,
  "maximizeRequested": true,
  "maximized": true
}
```

The operator-visible browser was restored. No duplicate browser was launched,
no profile was replaced, no route switch was required, and no other workload
was parked or closed.

## Why This Matters

This is a fresh installed-runtime reproducer of a known semantic gap, not a new
route-pool or lifecycle-owner incident.

- `docs/dev/notes/2026-06-22-rdp-browser-determinism-audit.md` already records
  that `browser_window_visible` does not prove that the intended browser is
  topmost, focused, non-minimized, or visible through the actual operator
  client.
- `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`
  keeps `browser_window_visible` as operator-view evidence while explicitly
  requiring stronger active-window, topmost, geometry, and occlusion evidence
  for capture-ready scenes.
- The field incident proves that the weaker operator handoff and reattachment
  path can still declare readiness before the human has a usable browser
  surface.

The immediate workaround is `view_focus`, but requiring the operator or a
consumer agent to discover and issue that second action violates the intended
"open or reconnect the durable handoff" contract.

## Current Source Boundary

CodeGraph readback at main commit `a54b0f976fb20e801d8e09e844708753c80ac79d`
identified the relevant seams:

- `cli/src/native/remote_view.rs::visible_browser_window_proof` accepts
  `displayContent.state == "browser_window_visible"` as `state: ready` without
  binding active-window, workspace, minimized, maximize, or occlusion proof.
- `cli/src/native/browser_lifecycle.rs::handle_view_focus` selects the intended
  target when supplied and invokes native focus plus maximize by default.
- `cli/src/native/browser.rs::focus_for_view` and its native-window helpers are
  the existing repair machinery. The successful live result confirms that
  this path can restore the retained window without relaunching Chrome.
- `cli/src/native/x11_scene.rs` already has stronger process-bound scene
  evidence for active window, topmost ownership, authorized geometry, and
  occlusion. That model should be reused or deliberately adapted instead of
  creating a second unrelated definition of operator visibility.
- `cli/src/native/remote_view/open/coordinator.rs` owns route-bound open and
  durable-resolution sequencing. Reattachment and durable handoff resolution
  must converge on the same usable-window postcondition.

The exact mechanism that displaced the window is not yet proven. Candidate
causes include minimization, Openbox workspace displacement, another window
being raised, or scene restoration that did not recognize newer human intent.
Do not select one cause without a bounded trace or deterministic fixture.

## Required Repair Contract

The next implementation packet should satisfy all of these conditions:

1. A ready operator handoff means the exact retained browser window is mapped,
   non-minimized, on the operator-visible workspace, active or topmost as the
   selected policy requires, and at the expected maximized or approved
   geometry.
2. Reattachment restores or focuses the exact browser only when current viewer
   and controller authority allow that presentation mutation.
3. An active human controller has priority. Automated staging or restoration
   must not move a window out from under ongoing human input.
4. After focus or restoration, the service re-reads the desktop state. A
   successful command result alone is not the readiness proof.
5. If focus is disallowed or the postcondition cannot be proved, return a typed
   operator-presentation blocker. Do not report `attached_ready` or
   `operatorVisible.state: ready`.
6. Browser acquisition, profile ownership, tabs, and durable handoff identity
   remain unchanged throughout the repair.

## Suggested Implementation Packet

1. Add a fixture for a route-bound retained browser that is mapped but
   minimized or assigned to a non-visible Openbox workspace. Confirm the
   current visible-window proof incorrectly accepts the scene or lacks the
   evidence needed to reject it.
2. Extend the operator-visible proof with process-bound active-window,
   minimized, workspace, geometry, and occlusion evidence. Reuse the controlled
   X11 scene provider introduced by Plan 0131 and the scene semantics from Plan
   0124 where their authority and lifecycle match.
3. Make `service_remote_view_browser_reattach` and durable handoff resolution
   establish the stronger postcondition. Prefer an internal focus-and-verify
   phase over requiring callers to queue a separate `view_focus` request.
4. Preserve the existing standalone `view_focus` action as an explicit repair
   and dashboard operation.
5. Add no-launch tests for:
   - minimized browser rejected before repair;
   - wrong-workspace browser rejected before repair;
   - permitted focus and maximize followed by ready proof;
   - focus success followed by failed re-observation remaining not ready;
   - active human control preventing disruptive automatic staging;
   - retained browser, profile, session, tab, route, and handoff identities
     remaining unchanged.
6. Run one development-runtime provider acceptance with a disposable retained
   browser. Production input remains out of scope until separately authorized.

## Bounded Reproducer

Use the service control plane and preserve its selected profile and browser
route hints:

1. Read `agent-browser://operating-guide`.
2. Call `service_access_plan` for an existing remote-headed retained browser.
3. Call `service_remote_view_browser_reattach` for that same browser and
   session. Record route/display agreement and visible-window evidence.
4. Minimize or move the browser only in an isolated development fixture with
   explicit presentation authority.
5. Resolve or reopen the same durable `/remote-view/<handoff-id>`.
6. Confirm the pre-fix path can report ready while the browser is not usable.
7. Queue `view_focus` for the same browser/session and confirm the response
   reports `broughtToFront`, `maximizeRequested`, and `maximized` as true.
8. Re-read the process-bound desktop scene and confirm the exact browser is on
   the visible workspace, active or topmost, non-minimized, and unobscured.

Do not preserve or publish raw Guacamole URLs. The only operator-facing link is
the durable authenticated `/remote-view/<handoff-id>` URL.

## Validation Expectations

At minimum, validate the focused Rust tests for visible-window proof,
reattachment, route-bound open, native focus, and X11 scene observation through
`scripts/ci/cargo-safe.sh`. Use `pnpm validation:select -- --base <known-green-ref>`
to select any wider checks required by the final diff. If service request
contract shapes change, also run the service contract parity and generated
client checks named in `AGENTS.md`.

The live acceptance must record separately:

- browser acquisition: reused existing retained browser;
- operator presentation before repair: not usable despite route attachment;
- operator presentation after repair: exact browser focused, maximized, and
  independently re-observed;
- runtime maintenance: advisory unless a scoped issue blocks the test;
- effects not taken: no replacement browser, profile replacement, unrelated
  route parking, broad cleanup, or private-site mutation.

## Hard Stops

- Do not initiate another workstation or production Agent Browser upgrade.
- Do not launch a duplicate browser to escape the presentation defect.
- Do not close, park, release, or take over another workload to obtain a route.
- Do not capture credentials, private page content, or tenant identifiers in
  fixtures, notes, logs, or screenshots.
- Do not treat a mapped Chrome window or `browser_window_visible` alone as
  human-visible readiness.
- Do not perform production browser input or provider mutation without a new,
  explicit authorization.

## Suggested Skills

- `agent-browser-service` for access-plan, retained-browser, route, and durable
  handoff boundaries.
- `codegraph-workspace` for structural tracing from reattachment and durable
  resolution into visible-window proof and native focus.
- `diagnosing-bugs` for the bounded minimized/workspace reproducer and root
  cause separation.
- `tdd` for the failing operator-visible postcondition tests before the repair.

## Best Next Action

Open one short-lived development worktree for this defect, anchor it to this
note and Plans 0124 and 0131, and implement the minimized/wrong-workspace
fixture first. Do not modify the installed production runtime until the
provider-free failure and repaired postcondition are both demonstrated.
