# P294: Authenticated dashboard browser commands

Product lane: PL-BUGFIX
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/451
Baseline: `c5063a3f84712b53b00448e182c1ced4b1db32e3`

## Failure and causal evidence

The installed read-only chooser probe returned HTTP 200 `text/html` for POST
`/api/command`, followed by JSON parse failure. The same symptom reproduced
inside the private real-browser fixture on binary `e7a233070ded`. Both the
production probe and disposable fixture left no observed owned-process residue.

Ranked explanations were missing top-level route, authentication redirect and
wrong backend routing. The real-listener regression established the first:
authentication rejects anonymous requests, login succeeds, but authenticated
inspection falls through to the embedded static document before the backend
sees the command. The per-session backend already implements this command path.

## Red-green and seam

The regression drives the actual dashboard TCP handler, creates disposable
credentials through ordinary login and connects an isolated HTTP backend. It
checks response JSON, unchanged exact scope/confirmation payloads, rejection
before backend contact, and JSON failures for invalid commands and methods.
This proves ingress behavior, not actual owner effects or rendered UI.

`scripts/ci/rust-tests.sh --focused authenticated_browser_commands_cross_the_real_dashboard_listener`
failed before the repair with `dashboard command must return backend JSON, not
the static HTML document`, then passed after it. The first invocation stalled in
`sccache` before compilation and was stopped with exit 143; its log is retained
separately. Subsequent invocations disable caching and use the existing bounded
four-job, one-invocation, 2 GiB host-reserve and 16 GiB cgroup settings.

The repair forwards only the three existing browser build/mode/copy actions
through the authenticated dashboard service proxy. It retains body fields and
owner validation and uses a bounded observation timeout without mutation replay.
No CLI, MCP or service response contract changes.

## Qualification in progress

Strict format and workspace clippy pass; all 70 focused dashboard Rust tests pass.
The public duplication fixture now has an explicit private-dashboard lane that
exercises mode changes, build inspection and independent profile copy through
this listener. A reviewed read-only Node chooser probe can run against that
same private dashboard before production effects. Exact binary and installed
qualification remain pending.

Independent Standards and Spec reviewers reported zero confirmed source
findings. The primary verified their conclusions against the frozen baseline
and retains actual mutation, rendered chooser and installed readback as pending
gates. Review is not runtime qualification.


## Private workflow setup corrections

The first optimized development run failed before dashboard ingress with
`manual_login_native_window_unavailable`; one browser process but no X11 window
was observed. A bounded second run passed that initial setup but exposed an
incomplete fixture payload: mode preparation lacked the exact session name and
was rejected with `browser_operation_task_binding_changed`. The ordinary dialog
already supplies this field. The fixture now reads the addressed task name from
public build inspection before constructing the same request. Owner validation
was not weakened. Both failures and their independent zero-residue receipts are
retained; the initial Native-window failure has no established cause and is not
claimed repaired by dashboard routing.


## Owner-lane qualification finding

The corrected request reached the owner and dispatched replacement, but exact
posture qualification failed with `mode_operation_replacement_posture_unverified`.
The failed browser was not accepted and cleanup again found zero namespace
residue. Source tracing showed that backend relay stamps its own runtime lane;
forwarding every command through the general backend can therefore substitute
its configuration despite retained task identity in the body. This requires a
second regression: named browser commands must select their existing owned
session HTTP route, and a missing route must return a typed unavailable result
without fallback. Profile-only requests retain the ordinary service backend.
This finding remains under #451's original owner-routing requirement.


The owned-lane regression failed before correction: expected `exact-owner`,
received no owner identity from the general backend. After selecting the existing
owned route and rejecting unavailable named owners, all 70 dashboard tests and
strict format/clippy pass again. Both independent review axes report no confirmed
findings in this delta. Operational qualification is still required.

A test-only debug replay expired its 15-second MCP response deadline while
runtime startup logged 16.7–17.0 seconds. It never reached browser creation or the
repaired route. Its independent cleanup receipt is empty. This is not optimized
candidate acceptance; final private qualification uses the repository's optimized
CI profile and production qualification uses the sealed release profile.


## Fixture qualification corrections

The owner-port change satisfies transport identity independently; it did not
establish the cause of subsequent Native readiness failures. Inspection of the
provider's earlier exception identified a missing fixture operation: the copy
provider handled keys/text but rejected the pointer move required by mode
qualification. The handler now performs the same real X11 move used by the
existing mode fixture, after its unchanged process/window/geometry fences.

A separate sub-second X11 rehearsal demonstrated loss of a root-window marker
when the last client disconnected, and retention with `-noreset`. Both private
Xvfb displays now disable that reset. This addresses the fixture's observed
window-manager setup race without changing production display services.

The routing acceptance workflow starts with an automation task, explicitly
selects the reviewed source build, converts to No CDP and back, reads the chooser
without effects, and copies to the other reviewed build. It retains the earlier
cold manual-start failures and does not claim universal cold-start acceptance.
The first completed conversion exposed an incorrect fixture assertion:
normal completion publishes `observedMode`; `effectiveMode` belongs to recovery.
The corrected assertion is followed by independent public mode inspection.
All failed attempts remain retained with fresh zero namespace/process residue.


## Optimized development acceptance

Optimized CI-profile test binary
`44027ca44896b0c6998078c2590728f1ed33c2165beb4e6da1180f50f137d06d`
passes the complete private dashboard workflow. Its Native source diff matches
the committed repair byte-for-byte. The workflow performs eleven authenticated
dashboard commands: selected-build inspection, both mode directions and
independent profile copy. It verifies terminal observed mode through an
independent public read, two original mode closes, one copy close and replay
without another launch or close. The rendered chooser passes the reviewed
read-only probe with no page errors or mutation attempts. Copy/profile and
synthetic-cookie checks pass across Chrome153 and Chromium150. Fresh owned
namespace and fixture-tag scans report zero residue.

The final fixture correction scopes close-count assertions to their phases:
two legitimate mode closes are checked separately before the existing copy
replay oracle requires exactly one close. This preserves the no-replay invariant.
All earlier failed attempts remain retained. Production build, exact sealed
qualification, preserve installation and public installed chooser inspection
remain pending.
