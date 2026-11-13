# Fresh-profile CDP-free mode continuity

Product lane: PL-BUGFIX
State: OPEN
Work item: CochranResearchGroup/agent-browser#673
Primary writer: MODE-HANDOFF-CONTROL-649

## Bounded outcome

Keep Chrome first-run onboarding and default-browser prompts from replacing the
intended page when a fresh automation profile becomes CDP-free. Preserve native
session restoration, initial URL behavior, exact mode scope, process custody and
no-CDP posture. Reuse the existing checkout and retained candidate custody.
This lane owns the manual Chrome launch argument list and matched regressions;
peer placement and MCP default-mode work retain their existing ownership.

## Diagnosis

A disposable Chrome/X11 loop first visits a loopback marker with first-run UI
suppressed, closes gracefully, then requests native session restoration. Omitting
first-run suppression in the replacement reliably opens the generic Chrome
welcome window and loses the visible marker. Adding only the two suppression
flags restores the marker with the same profile, executable and saved session.
Both runs exit every owned process. This narrows the defect to manual startup
posture, without account, credential or production-browser effects.

Add a failing argument-builder regression before changing production code. Run
matched Rust checks, strict workspace Clippy/formatting, required guidance and
exact-artifact qualification. Freeze source during production build. Actual
acceptance must keep the same page and handoff through mode conversion and prove
interactive return, then clean only the disposable browser. Source, installed
and actual workflow proof remain separate.

## Source qualification

The new argument-builder regression failed on the missing first-run flag before
production changed. After the narrow manual/no-debugging branch fix, 21 launch
argument tests, 3 ordinary recovery tests, 28 mode tests and the manual-login
guard passed. Strict workspace Clippy and formatting passed. Policy wiring (11),
changed links, handoff documentation, docs build, host provisioning, VM harness,
viewer assets, database durability and route-user contracts passed locally.
The root guide is 9,992 bytes; independently edited shared guidance is preserved.
No hosted CI ran. Production and installed workflow qualification remain open.

Serial Standards review found no blocking breach: no new API, viewer, CDP,
private-input reuse or process-identity bypass. Serial Spec review found the
change matches the isolated first-run failure while retaining the distinct
CDP-backed manual-login posture. Actual page continuity through the public mode
toggle remains a required acceptance gate, not inferred from argument tests.

Main placement repair PR675 is integrated. Combined-source strict Clippy and
formatting pass; all 3 ordinary recovery and 2 replacement-placement cases pass.
The placement and startup guide intents remain in one linked reference.

## Installed attempt and bounded follow-up

PR678 is merged and production binary
`21bd2f5eebcde1fe5021d59aa80c700fb86db467ebe57418ee681fd5e1db13c4`
passed exact-artifact guidance/installer fixtures and preserving installation.
Release compilation took 30m31 after a resource-admission wait; source stayed
frozen. First doctor observed monitor lock-timeout backoff, later healthy.

Actual disposable acceptance is FAILED, not complete. A first authenticated
handoff returned503 before effects; the same-link retry loaded. Initial CDP mode
and interactive control passed. Confirmed conversion launched a CDP-free process
with session restore and first-run suppression, but stalled at
`mode_operation_native_unavailable` and viewer rejoin timed out. The mode journal
retains one launch and replacement-pending state. The disposable task later had
an explicit-close history entry before reconciliation; the test did not submit a
cleanup request. Preserve this uncertainty and do not attribute the close without
further evidence. Private runtime IDs/receipts stay outside Git.

The combined replacement-placement path needs a bounded fixture: qualify Native
readiness against the exact observed unpublished replacement assignment, rather
than the retained original presentation. Preserve logical target, original
absence, pending launch identity and process checks. No general control or
publication guard may be relaxed. Compare changed-assignment, unchanged-assignment
and changed/foreign pending identity cases before another installed candidate.
The public same-page conversion remains a required gate.


## Pending-assignment repair qualification

The actual recovery-adapter fixture went RED by returning the original desktop
instead of the newly selected pending desktop. The first fixture compile failed
on private host-field access; the corrected fixture constructs the real adapter
without exposing production fields. Both changed-default and unchanged-explicit
assignment cases now pass. They reject missing pending custody, a changed PID and
foreign profile and prove no publication during observation.

An explicit replacement-only Native target method checks the original logical
target, observed pending intent, launch desktop and durable unpublished record's
browser/PID/process identity before fresh provider observation. Mode qualification
uses that method; ordinary Native input still uses published assignment custody.
No new viewer, control grant or lifecycle retry is added.

Strict Clippy/fmt and 37 focused executions passed (2 pending assignment, 30 mode,
2 placement, 3 ordinary recovery). The complete service-model package, its
integration binaries and doc tests passed. Policy/link/handoff docs and docs build
passed. Installed acceptance remains open; the first failed attempt is retained.

Final integration preserves PR679 configured-build defaults and its installed
source. Combined strict Clippy/fmt and both pending-assignment cases pass. The
initial configured-build filter matched zero tests and is excluded; the corrected
StealthCDP filter passed 9 tests with 1 ignored browser e2e. Both managed default
launch and required-build refusal cases ran. The merged docs build passes. Peer
configured-build guidance is retained verbatim in the linked readiness reference;
the compact root guide is 9,993 bytes. No shared user-scoped guide was overwritten.
