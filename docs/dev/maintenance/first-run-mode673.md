# Fresh-profile CDP-free mode continuity

Product lane: PL-BUGFIX
State: COMPLETE
Work item: CochranResearchGroup/agent-browser#673
Primary writer: MODE-HANDOFF-CONTROL-649
Consolidation: required

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


## Observation-only lifecycle follow-up

PR686 is merged and its exact installed candidate passed guidance and installer
fixtures. Actual public acceptance retained the marker page on the replacement
and independently observed its new Native desktop, with CDP absent and first-run
UI suppressed. Conversion still timed out: the retained operation remained
launch-dispatched with one attempt. This is failed workflow evidence, not a pass.

The mode qualifier still called the guarded-input readiness adapter. A real
owner-HTTP fixture is RED: an observable exact window returned ready=false when
input policy was disabled. Lifecycle completion must observe the window without
consulting input readiness. Factor the existing retained-window observation-only
adapter for reuse by an exactly claimed pending process. Keep persisted-state,
viewing-generation and process-identity checks; ordinary input policy remains
unchanged. Match positive observation and wrong-process rejection, retained-window
fences, mode operation checks and strict quality before final installed acceptance.
The page-continuity and public interactive/return gate remain open.


## Consolidated batch

One fresh-profile mode workflow includes first-run suppression, exact pending
assignment and observation-only lifecycle qualification. The first two are
integrated; this follow-up addresses the remaining Native completion dependency.
Preserve peer placement and configured-build selection. Do not expand to unrelated
input policy, provider capacity, global task closure or new viewer work.

## Delivery sequence and budget

Run the focused lifecycle regression, retained-window and mode checks plus strict
quality; review serially, merge, qualify one frozen production candidate, then
reconcile the original disposable operation and exercise the public workflow.
The resumed window has 70 active minutes remaining including build/observation,
at most one further production build and install. Prior failed attempts count;
this does not reset cumulative accounting. Reassess after two no-outcome
checkpoints or 30 minutes. Another discovered blocker ends this build tactic.

## Worker assignments

The primary owns this checkout, Native host extraction, mode call site, matched
fixtures and guidance. No delegation or parallel source writer is required.

## Evidence and exit

Reuse the unchanged first-run and pending-assignment evidence. New gates cover
the observation-only process boundary, existing retained-window identity/state
fences, mode transitions, strict Clippy/fmt and documentation. Exact-artifact
checks and same-page, same-handoff CDP-free conversion plus interactive return
remain mandatory; no fixture substitutes for public acceptance. Record failures
and preserve pending launch custody rather than issuing a second launch.


## Exact pending desktop publication

A bounded reconciliation readback sharpened the live failure to
browser_session_recovery_identity_conflict. The retained operation already held
qualified replacement evidence, including successful Native observation. A live
owner differential returned enabled input policy and the exact marker window
quickly. Thus input readiness was an unnecessary lifecycle dependency, but did
not explain this publication failure.

The owner replacement seam still passed current_desktop_rebind=false. Its final
recovery guard rejected the new desktop despite qualified pending launch custody.
The model regression is RED at this exact publication seam with the same error.
Allow a desktop rebind only for a valid observed pending intent whose profile,
desktop and generation match the replacement, under the existing operation fence.
Unknown moves, foreign profiles, mismatched generations, invalid intent and a
different operation remain refused without state changes. Ordinary recovery and
input guards are unchanged. Consolidate this demonstrated blocker with the
observation-only change before the single remaining candidate cycle.

Serial Standards review against integrated baseline c4861df82 found no blocker:
the observation adapter is extracted rather than duplicated; custody remains in
the existing model/CLI seam; all user-facing guidance surfaces are updated.
Serial Spec review found the change addresses the actual publication conflict
without authorizing arbitrary movement. Matching pending profile/desktop/generation
and operation fences remain required; real public acceptance remains the exit.

Combined source passes strict workspace Clippy and formatting, 15 focused CLI
host/handoff cases and all 424 service-model tests across 26 suites. Policy,
changed-link, handoff documentation and docs build pass. The zero-match unit
filter for mode-operation integration tests is excluded; the explicit integration
binary passed all 17 tests. No hosted CI ran.


## Installed workflow acceptance

PR688 merged as d217a3820. The sealed production binary
fb29957c2e4034a8bfeedbdb032a44eb20acb416ca0a1452b0e0a7b24e852be3
passed exact-artifact MCP guidance and source-free installer fixtures, including
zero residue from two fixture hosts. The preserving install is accepted. Final
doctor passes with a healthy, fresh, ready monitor; workspace-candidate and
operator-journey receipt warnings remain advisory.

The previously interrupted disposable conversion reconciled to completed with
its original single launch. Its unchanged durable handoff connected and displayed
CDP off without touching its existing operator hold. A fresh disposable public
workflow then passed initial CDP on, interactive takeover, confirmed CDP-off
relaunch, same-handoff reconnection, interactive takeover after conversion and
confirmed return. Native capture showed the unique marker heading and body on
the active page, without onboarding or a blank page. Kernel readback proved the
original absent and replacement running without remote debugging, with native
session restoration and first-run suppression.

Task-scoped close removed the fresh browser. Exact allocation-path and executable
readback found no Chrome or crashpad residue for that profile. An earlier
operator-held synthetic browser and its marker server remain deliberately
retained; their private cleanup obligation is separate from this passed repair.
No original user browser was reopened and no unrelated task was closed. Raw
runtime identities, URLs, captures and receipts stay outside Git. This qualifies
the installed local workflow; it does not claim a hosted or other-platform run.
