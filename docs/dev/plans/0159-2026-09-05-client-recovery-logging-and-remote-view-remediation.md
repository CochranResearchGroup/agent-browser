# Plan 0159: Client Recovery, Logging, and Remote-View Remediation

Date: 2026-09-05

State: OPEN

Execution state: `recovery_verified_logging_and_presentation_active`

Lane: P157

Branch: plan/profile-permissions-and-request-provenance

Target: main

Integration: merge

Source baseline: `0aba2f496c21541fbf31800acaca99bb2887357d`

Dependencies: [P157, P158]

Overlaps: [P144]

## Authority and objective

The operator instructed: write Plan 0159 and execute. This supersedes the
postmortem's discussion-only pause for successor work. The primary owns this
plan and continues ordinary implementation, focused validation, isolated
development publication and synthetic outcome verification without repeatedly
asking for approval. P158 calibration and its old automatic sequence remain
paused. No new broad discovery campaign is part of this plan.

Deliver the three original priorities: authorized clients reliably use their
Profiles and retained handles; failures leave durable correlated evidence and
useful recourse; ordinary authenticated durable remote-view URLs show the
correct browser, accept input, and work when reopened. Consolidate verified
repairs into a concrete production-update decision instead of leaving source
validation disconnected from delivery.

The [postmortem](../notes/0152-2026-09-05-plan-0158-postmortem.md) and
[D01–D15 register](../notes/0152-2026-09-05-plan-0158-defect-register.json)
are the fixed starting inventory. Preserve their historical status and failed
attempts; record new dispositions here. Completing this successor does not
retroactively pass P158's unexecuted full campaign, performance or endurance
criteria. Missing coverage does not become an automatic work queue.

Production replacement remains a separately reviewed delivery decision as
established in the handoff. Prepare the exact candidate, dependencies, rollback,
and installed outcome checks before that decision. No credential entry, private
page capture, production ACL mutation, client eviction, or formal release is
included. The existing terminal-target replacement proposal still requires
explicit reopening confirmation; ordinary reopen of a live retained browser
is a different behavior. External synthetic proof uses the existing protected,
manually dispatched workflow with exact identity/capture bindings and no retry.

## Acceptance contract

1. Two independently authenticated clients resume control through their
   ORIGINAL authorized handles after an unexpected disposable host stop with
   unchanged Chrome process, owner and target identities. No extra acquisition,
   replacement tab, broader grant, or foreign-client access masks the defect.
   Genuine missing/changed owner identity fails before browser effects with
   actionable evidence. Existing valid attachment and peer behavior remain safe.
2. Recovery and denial failures retain typed cause, phase/axis, authenticated
   provenance, correct effect certainty, and safe recourse across applicable
   response/job/event/trace/journal projections. Expected, observed, missing,
   duplicate and conflicting counts are explicit for selected failures.
   Existing journal interruption and actor fixes remain included in the repair
   set; their scoped proof does not become a power-loss or universal coverage claim.
3. Ordinary authenticated durable URLs, without a calibration anchor, show
   the exact synthetic browser, accept input, reopen the same URL, and preserve
   identity under selected reconnect/concurrent-view transitions. Require
   `operatorVisible.state=ready` and external pixels/input, not HTTP 200 or a
   stored ready flag. P/Q and other failed epochs remain visible. A terminal
   replacement cannot count as retained-browser continuity.
4. A reviewed repair inventory separates source, built, isolated-installed,
   production-installed and user-outcome states. The production proposal names
   exact candidate/dependencies, rollback, compatibility-override disposition,
   and outcome checks. Record deployment if authorized and performed; otherwise
   leave that decision explicitly pending without claiming production success.
5. Evidence is private, durable and source-bound; cleanup separately accounts
   for Service resources and exact owned processes. Any retained obligation
   has an explicit disposition. No foreign cleanup or raw private evidence
   enters the product repository.

## Work units

| Unit | Defects and work | Exit evidence | Dependency |
| --- | --- | --- | --- |
| W1 | D01/D02: fix authenticated retained-handle attachment and failure recourse | Correct-seam red/green regression, required Rust checks, one isolated original-handle repair verification with authorization/identity counterexamples and correlated failures | Existing red evidence |
| W2 | D05–D10: consolidate the already verified denial, journal, actor, drain, advice and startup fixes; close logging gaps exposed by W1 | Explicit fix/dependency inventory and selected causal joins; no repeat of completed live cycles without regression evidence | W1 results where related |
| W3 | D03/D14/D15: diagnose retained frame lifecycle from P/Q/K/W; repair the actual ordinary-link seam | Focused red/green proof, isolated installed check, protected ordinary same-URL external pixels/input/reconnect evidence | Source/evidence reconciliation; W1 only if attachment blocks selected link |
| W4 | D11/D12/D13: correct only harness/custody/cleanup issues required by the selected W1/W3 probe | Supported native transport and listener readiness; durable bounded observations and required reports; exact process cleanup or retained obligation | Applies to each selected probe |
| W5 | Consolidate delivery and closeout | Acceptance-by-acceptance audit, exact candidate/update/rollback proposal; production readback only if deployment is authorized | W1–W4 outcome evidence |

This is a dependency graph, not a requirement to use parallel agents. The
primary executes locally. No subagents are requested. A blocked presentation
unit does not block safe source or logging work. W4 must not turn into another
general harness-hardening prerequisite campaign.

## Bounds and inherited disposition

- D01 has two consumed diagnostics: listener race, then 0/2 original-handle
  recovery with listener ready and both Chrome processes preserved. The next
  live action is a separately source-bound repair verification, not a third
  unchanged diagnosis. Use existing evidence and a provider-free regression
  to identify the seam first.
- D03 retains failed P/Q and W. Do not rerun unchanged presentation diagnostics.
  Reframe from their evidence to a specific failing seam before another effect.
- Completed D04–D09 live repair cycles stay closed unless new regression
  evidence changes their disposition. D10's compatibility repair is not a
  source-fix deployment claim.
- Each selected unresolved defect has at most two diagnostics and one repair
  verification before written disposition. Historical attempts count. A failed
  verification ends that loop and requires a changed hypothesis/seam or scoped
  block, not new labels. Safe independent work continues under this objective.
- Two consecutive preparation/hardening checkpoints without outcome progress
  trigger work-selection reassessment. Checkpoint material transitions and at
  least every 60 minutes. Broad calibration, stress, new coverage producers and
  whole-branch promotion remain outside the active queue.
- Preserve uncertain effects and reconcile the exact operation before any
  retry. Each live probe has a deadline, private append-only ledger, positive
  fixture ownership, terminal evidence and narrow cleanup. Do not infer live
  terminality from observation timeout.

## Validation and documentation

Use the recorded failing command as historical red evidence; build a regression
at the real dispatch/attachment seam before changing behavior. Reproduce its
failure on the unchanged source, then fix and run focused relevant tests.
On WSL every compiling Cargo invocation uses `scripts/ci/cargo-safe.sh`.
Run Rust formatting and workspace clippy with warnings denied for Rust changes.
Use `pnpm build:development-candidate` for the isolated candidate and
`pnpm validation:select` against the slice baseline before integration.

Update help, README, repository agent skill, docs site and inline comments for
user-facing behavior changes. Keep generated service contracts aligned if their
shape changes. Do not rerun broad suites just to create activity. Exact native
integration and original-handle proof, not test counts, accept W1. External
ordinary interaction, not conditional anchor proof, accepts W3.

## Initial checkpoint

Transition: `postmortem_complete -> retained_handle_repair_active`.
All five acceptance criteria are initially incomplete. Historical D01 evidence
provides a red client-level oracle, and D03 evidence provides failed frame
lifecycles. No new live attempt has run. The plan supersedes discussion-only
successor instructions while preserving P158 history and action-specific gates.
Next: trace handle-bearing dispatch to attachment, add the smallest real-seam
regression and repair D01/D02 before the one source-bound live verification.

## Checkpoint 1: original-handle recovery verified

Date: 2026-09-06 UTC. Progress: `outcome_progress`.
Transition: `retained_handle_repair_active -> recovery_verified_logging_and_presentation_active`.

Source repair `894076b42bdbc013a5db8601f8ffb983218d80f8` reconnects only the
original target after child authorization, exact owner/process/endpoint checks,
and a second ownership fence. Missing targets do not select a peer or create a
tab. Recovery failures have typed recourse and preserve uncertain attachment
effects. Help, README, repository skill, service docs and inline comments changed
with the implementation.

The real-dispatch regression reproduced the original rejection before repair.
Both recovery/absent-target tests then passed; the dispatch module passed 13
cases, failure recourse 14, and bounded evaluation one. Workspace clippy with
warnings denied, format check, docs build and remote-view handoff documentation
checks passed. These are focused checks, not a full-suite claim.

The one source-bound disposable repair verification passed: two authenticated
clients evaluated through their ORIGINAL handles before and after supervisor
SIGKILL; both original Chrome process identities remained unchanged. No extra
acquisition masked recovery. The foreign-client attempt failed with
`profile_child_subject_mismatch`, `child_admission`, `no_effect`, and own-handle
recourse. Its response, trace job, terminal event and journal join each contain
one expected occurrence: missing 0, duplicates 0, conflicts 0. Actor provenance
matches across response/job/event; the journal joins it by request ID rather
than embedding the actor. This is selected-denial evidence, not universal logging
coverage or live negative-owner proof.

Private source-bound evidence: operator state `campaigns/p159/retained-repair-HvlahC/`,
including `evidence-manifest.json`, ledger, probe source, trace, journal, snapshots
and cleanup receipt. Candidate SHA-256:
`6439f9f866e7516ac1b29d2fd9e02d8b5f934ea3700d6fbeddaa87907b0e4602`.
The top-level evidence scan found no raw capability matches. This candidate ran
in a disposable supervisor; it was not published to the installed development
runtime or production.

Cleanup separately proves zero Service browsers/sessions/tabs, host exit 0,
and eight captured ancillary identities terminal or absent through exact
identity/pidfd accounting. A subsequent readable-process environment scan found
zero fixture HOME matches; inaccessible foreign environments are not a global
process absence proof. The inherited P158 ancillary obligation is separately
accounted for by `campaigns/p159/inherited-residue-cleanup-v4.jsonl`; retain the
preceding partial receipts. No broad process cleanup was used.

Two additional journal failures preceded the interruption: automatic
`stream_enable` reported already-enabled state as a generic uncertain failure,
with unknown actor. They do not falsify the retained-handle outcome, but remain
an explicit W2 logging gap to trace and adjudicate. D01's live loop is complete;
do not repeat it. Genuine owner/identity counterexamples and comprehensive W1/W2
acceptance still require scope review. W3 ordinary external pixels/input/reopen
and W5 delivery remain unaccepted. Next: inspect the recorded presentation
failures and the duplicate stream-enable caller before selecting the next repair.

## Checkpoint 2: observer admission detached the presentation frame

Date: 2026-09-06 UTC. Progress: `blocker_reduction`.
Execution remains `recovery_verified_logging_and_presentation_active`.

Re-read P/Q/K/W artifacts rather than repeating their live diagnostics. P showed
correct primary/shared pixels and failed reconnect; Q detached its initial frame.
Both P and Q retained the observer-lease success message. K's missing reconnect
iframe and W's public transport failures remain distinct observations, not a
single proven cause.

A mounted React/Chrome regression now demonstrates a specific current source
cause: automatic observer admission runs concurrently with Guacamole frame
connection. When its response arrives, `reconnectWorkspaceViewer` changed the
iframe key through `streamRefreshNonce`, destroying the existing frame despite
unchanged browser, route and URL. Removing that nonce change preserves the exact
frame node. Explicit Reload view still replaces it. No timeout increase, extra
anchor, broader retry or replacement target is part of this fix.

`pnpm test:dashboard-observer-frame-lifecycle` mounts the actual component with
real React reconciliation and synthetic provider/Service responses. The original
remount was restored temporarily to prove the final regression red, then removed
to prove green. Private receipts and source/test hashes are retained under
`campaigns/p159/observer-frame-regression/`. Decorative UI/store atoms are mocked;
connection planning, effects and iframe rendering execute from current source.
This is a deterministic product regression, not proof that every historical
external failure had this cause or that ordinary external reconnect now passes.

Dashboard sharing, view-stream, navigator and handoff-doc checks pass. Dashboard
and docs builds, Rust format and workspace clippy with warnings denied pass.
README, help, repository skill, remote-view docs and inline behavior comments
are updated. Validation-selector installer/release recommendations stem from
broad help/package mappings; installer and release behavior did not change.
Production/default-dashboard publication and shared-skill overwrite suggestions
are superseded by the explicit isolated-development boundary.

W3 remains unaccepted pending source-bound isolated installation and the one
protected ordinary external verification of pixels, input and same-URL reopen.
Do not repeat P/Q unchanged or dispatch calibration. W2's duplicate stream-enable
logging gap and negative-owner recourse scope remain open. Next: bind the built
candidate and served dashboard asset to this repair, reconcile the isolated
provider/target state, and prepare the ordinary external verification within the
existing synthetic capture and no-retry controls. The terminal-target replacement
proposal remains unexecuted and is not the continuity oracle.

## Checkpoint 3: isolated candidate installed and served

Date: 2026-09-06 UTC. Progress: `blocker_reduction`.

Source `c1434e57dda060da85922dcb44be851c80dd8c0f` built successfully and is
installed only in the existing isolated P158 lane as generation
`0.28.0-8f2e24a62491`. Binary SHA-256:
`8f2e24a624918118faf1d20f7e4500f402d345f3cbd89687e03d91427bdb22c5`.
Before installation, authoritative lifecycle readback showed 26 terminal records
with satisfied cleanup, no viable retained browser, and four provider slots.
The previous generation and private Service snapshot are preserved. The installer
verified production and default development unchanged, synchronized only the
isolated skill, and reported the new lane ready. Required development doctor and
three disposable browser launch/URL-read/close/residue checks passed.

The installed dashboard served the exact built JavaScript bytes containing the
viewport implementation. Asset SHA-256:
`8db63419295968e07ea66a393b6c53cc467c1f1907bbe73f5b0cecf987d30bf6`.
Private candidate, installation, guards, doctor, launch checks and served-asset
receipts are under `campaigns/p159/frame-candidate-install/`.

This advances isolated-installed custody for the recovery and frame repairs;
it does not prove remote desktop pixels/input or ordinary external reconnect.
No external workflow, terminal-target replacement, production update or P158
calibration was executed. Next: obtain the supported access plan for the
synthetic ordinary-link fixture, preserve the terminal proposal boundary,
and bind one protected external verification to the exact installed candidate.
W2 logging and W5 production decision remain open.

## Checkpoint 4: ordinary open lost the access-plan session

Date: 2026-09-06 UTC. Progress: `blocker_reduction`.

The first local ordinary durable-URL preparation failed before an iframe was
rendered. The supported open reported operator-visible readiness on route 1,
but durable resolution selected route 2 and could not find the same browser's
window there. Service returned a failed job and rolled back to the original
display allocation. No external workflow was dispatched and no unchanged retry
was made. This is a distinct failure before the repaired frame-lifecycle seam.

The retained request and authoritative state identify the cause: the access
plan selected a named session, but `createServiceRemoteViewOpenRequest` moved
`sessionName` into action parameters and deleted the top-level daemon selector.
HTTP correctly left this request on its default daemon. Open then persisted the
requested session in handoff/route/display metadata while the actual browser
belonged to the default session. Durable resolution used the current owner
session, making the original route fail its exact-owner checks.

The helper now preserves explicit top-level browser/session routing alongside
matching action metadata. The unchanged helper failed a routing regression;
the repair passes that test, including a session-only access-plan shape,
conflicting nested metadata overridden by explicit routing, and serialized HTTP
request custody. The full service-client gate, Rust format and workspace clippy
with warnings denied, and docs build pass. All five documentation surfaces are
updated. This is source/client-contract evidence, not a repaired stored handoff
or successful external presentation claim.

Private evidence remains under `campaigns/p159/`: `ordinary-access-plan.json`,
`ordinary-open-ledger.jsonl`, `ordinary-open.json`, `ordinary-local-preparation.json`,
`ordinary-jobs-after-local.json`, `ordinary-state-after-local.json`, and the
synthetic scene captures. The failed handoff and original synthetic browser are
retained for exact reconciliation, with the fixture server still running.
They are explicit live cleanup obligations, not zero-resource cleanup. The
terminal-target replacement proposal remains unexecuted.

The earlier full Rust suite's tool handle is now absent and its final result
was not recovered; do not claim that suite passed. No active Cargo test process
was found at this checkpoint. Focused prior results retain their stated scope.
Next: reconcile the malformed handoff through supported exact-owner semantics
or explicitly dispose of that failed fixture before a changed-source ordinary
verification. A new fixture cannot count as recovery of the failed URL. W2's
logging gap, external W3 acceptance and W5 delivery decision remain open.

## Checkpoint 5: presentation reacquisition erased client access

Date: 2026-09-06 UTC. Progress: `blocker_reduction`.

Exact fixture reconciliation used supported route release with display release,
then browser reattachment to its original route/display. Before effects, the
route had no active viewer or controller, and the browser process start identity
and physical display matched the captured fixture. Both Service actions passed;
Chrome's process identity and original target remained present. No new browser,
replacement target or handoff was acquired. Private receipts are
`ordinary-route-release.json`, `ordinary-route-reattach.json`, their ledgers,
`ordinary-before-route-repair-identity.json`, and
`ordinary-after-route-repair-state.json` under `campaigns/p159/`.

The changed-state same-URL local verification reached the embedded frame, but
showed Guacamole's login screen and timed out before connected pixels. The
browser context closed. This advances route reconciliation only; it does not
accept ordinary external presentation, and no external run was dispatched.
The previous local failure and this new result remain separate receipts.

An original-handle check then failed because its stored Profile child access
record was missing. The pre-reconciliation snapshot already lacked that record,
so the loss preceded route release/reattachment. Source tracing and a real
persistence regression identify a second seam: durable target reacquisition
called the new-tab persistence function with a synthetic handle containing no
client grant. It overwrote the existing child record while merely resolving
presentation. The original implementation reproduced that overwrite.

Presentation-only reacquisition now skips new-tab persistence. The regression
passes with the entire original tab record and event count unchanged. Missing
child-record failures also retain the typed code
`profile_child_access_record_missing`, `profile_access`, `child_admission`,
`no_effect`, and trace-inspection recourse. The original generic classification
failed its regression; the repaired classifier passes. Direct admission tests
cover both missing-tab and missing-grant records, proving no state mutation and
no reconstruction from the caller's handle. Wrapped or unknown errors retain
uncertain effect classification.

Focused custody tests (2), the remote-view open module (see retained test log),
failure-recourse module and direct child-admission tests pass. Workspace clippy
with warnings denied, formatting, handoff docs, failure-journal/dashboard
observation contracts and docs build pass. Red/green and validation logs are
retained in `campaigns/p159/child-custody-repair/`.

The erased live grant has not been recreated. The original fixture remains a
failed acceptance attempt and a cleanup obligation; its process is still live.
This source repair prevents the demonstrated overwrite but has not been
installed or verified live. Next: account for that failed fixture, publish the
changed-source isolated candidate, and verify that ordinary URL resolution
preserves a newly authorized original handle. Investigate the Guacamole login
boundary from source/evidence before another presentation attempt. W2's earlier
stream-enable logging gap and W5 delivery decision remain open.

### Checkpoint 5 cleanup addendum

The failed fixture was subsequently closed through `service_browser_close` on
its exact original daemon/browser, after rechecking the captured process start
identity. Service reported success. All 14 captured Chrome tree identities were
absent or terminal afterward, and the readable namespace scan found no remaining
Chrome or crashpad process. The authoritative lifecycle projection reports 29
terminal records, all with satisfied cleanup. Four provider placeholder browser
rows and their historical sessions/tabs remain; these are not zero-row evidence.
Receipts: `ordinary-close-ledger.jsonl`, `ordinary-close-owned-processes.json`,
`ordinary-close-process-readback.json`, and `ordinary-after-close-state.json`.
The synthetic fixture HTTP server is intentionally retained for the next
source-bound verification and remains an explicit final cleanup obligation.
The failed URL is now terminal and will not count as successful original-handle
recovery or be reopened without an explicit, applicable reopening decision.

## Checkpoint 6: custody candidate installed and denial evidence joined

Date: 2026-09-06 UTC. Progress: `outcome_progress` for selected failure custody;
ordinary external presentation remains unaccepted.

Runtime source `6efa26f1` built and installed only in the existing isolated P158
lane as `0.28.0-7df13c4085f9`. Binary SHA-256:
`7df13c4085f9b2bf235e0326be76d3171f83c7e6dcd9488efdc4bd7841c131c2`.
Pre-install authority showed all 29 lifecycle records terminal with satisfied
cleanup. The private Service backup and prior generation are retained under
`campaigns/p159/child-candidate-install/`. Installation verified production and
default development unchanged. Development doctor and the three disposable
launch/URL-read/close/residue checks passed.

One deliberately closed-handle admission request against that installed binary
returned `profile_child_access_record_missing`, `child_admission`, and
`no_effect`. This is an expected negative case, not an attempt to revive the
terminal fixture. Response, trace job, terminal event, trace activity and journal
each contain one matching occurrence: expected 1 each, missing 0, duplicates 0,
conflicts 0. Actor matches response/job/event; the journal joins through request
ID. Its code and effect certainty agree with the response and job. Receipts and
hashes are in `closed-handle-correlation.json` beside the private raw projections.
This accepts that selected installed failure projection only; the live
reacquisition-preserves-grant repair verification is still required.

The full Rust script passed its CDP, parallel-safe and preceding serial
partitions, then hit a stack overflow in the long Service configuration action
scenario. A debugger reproduced the failure during its first profile upsert.
The test now boxes each real dispatcher future through a local test helper;
all commands and assertions remain intact, and the default stack size is
unchanged. The three configuration tests pass. The remaining 35 serial
partitions then passed under the standard wrapper, with their exact filters and
results retained in `child-custody-repair/remaining-rust-partitions.json`.
The original failed run is preserved; this is completed partitioned validation,
not a claim that the original uninterrupted invocation passed. The test-only
change does not alter the installed runtime candidate.

The local Guacamole login observation is not yet an external authentication
failure: source shows that the public forward-auth route supplies the provider's
`Remote-User` identity, while the localhost check bypassed that ingress. Treat
that as a scope limitation and verify through the required protected public
workflow; do not add a guessed authentication repair or repeat localhost as the
external oracle. The synthetic HTTP server remains intentionally retained.

## Checkpoint 7: live custody passes; external verification remains incomplete

Progress: `outcome_progress`. Execution remains
`recovery_verified_logging_and_presentation_active`.

The installed custody candidate preserved a newly authorized original handle:
evaluation succeeded before and after supported durable handoff resolution,
which returned ready for the same browser/session/target. The access-plan session
also survived the client helper and HTTP routing. Private source-bound receipts,
the append-only ledger and preserved probe sources are under
`campaigns/p159/custody-outcome/`. This verifies the live grant-preservation seam;
it does not replace the earlier two-client host-restart proof.

Protected manual readiness run
[34009455909](https://github.com/CochranResearchGroup/agent-browser/actions/runs/34009455909)
finished with failure against source `ba55a7f7` and installed binary
`7df13c4085f9b2bf235e0326be76d3171f83c7e6dcd9488efdc4bd7841c131c2`.
The human-paced client passed ordinary public pixels and same-URL reconnect,
with unchanged expected identity and zero physical browser launch delta. Its
action manifest was absent and action observations empty: remote input is NOT
proven by that pass.

The slow concurrent client captured initial, concurrent and reconnect pixel
markers matching the independently frozen expected hash, but its dashboard
oracle rejected one console error. All 146 captured Guacamole HTTP responses
were 200, with zero failed or pending requests. These facts do not waive the
console failure or establish complete concurrent acceptance. The error was
unclassified; its location hash joins to the shared-view Guacamole
`guacamole-common-js/all.min.js` asset, whose request succeeded. Raw error text
was not retained, so the causal diagnosis remains incomplete. Preserve this
logging limitation rather than inventing a transport cause from the hash.

Both clients report zero retries. No calibration or unchanged rerun occurred.
Private downloaded artifacts and their adjudication are retained in
`custody-outcome/external/`; the aggregate failure remains authoritative.
The synthetic browser and fixture server are retained cleanup obligations.
W3 remains open for the console failure and actual remote input. W2's duplicate
stream-enable failure and owner-counterexample scope, and W5's concrete
production proposal, remain open. Next: resolve the selected console evidence
gap before any changed-seam external verification, while continuing independent
logging work. Production replacement remains pending.

### Checkpoint 7 diagnostic custody repair

The actual external-runner console callback now preserves page ID, numeric
source coordinates, a hashed WebSocket endpoint and a bounded failure reason.
Guacamole WebSocket failures are distinguishable from other console errors;
raw console text and token-bearing URLs remain excluded. Normalization retains
these fields through artifact projection. Every error still fails the dashboard
oracle unless an existing, independently supported classification applies.
This does not identify the historical hash-only error as a WebSocket failure.

A regression invokes the actual capture callback and final normalizer. It failed
on missing page attribution before the repair, then passed source coordinates,
endpoint hashing, reason custody, normalization idempotence, unknown-error
rejection and secret-exclusion assertions. The full external-runner,
dashboard-oracle and external-handoff-oracle provider-free checks pass. Private
red/green logs are retained under `custody-outcome/console-custody-repair/`.
This closes a demonstrated evidence-loss seam, not W3's runtime failure or input
acceptance. No external rerun or runtime publication accompanies this change.

Input-path review found that the optional H01 manifest does not supply the
missing remote-control proof: `buildW8H01ActionObservations` derives its interact
observation from `initial.firstUsablePixelsAt`, while `humanPacedObservation`
only traverses dashboard focus and moves the mouse. The existing controller and
interaction-coordinate helpers have no production callers. Do not dispatch
that manifest and count its projected interaction as accepted remote input.
The next W3 change must exercise actual synthetic desktop input and verify a
target-side response, then bind the interaction receipt to that observation.

## Checkpoint 8: remote input verification implemented

Progress: `blocker_reduction`. W3 remains unaccepted externally.

The synthetic fixture now acknowledges a trusted marker click by turning the
marker white and acknowledges a subsequent trusted Enter key by restoring its
original pixels. Scripted DOM events cannot advance the acknowledgment. The
attestation binds this opt-in protocol. Only the readiness human-controller lane
uses it; calibration and the concurrent observer do not inject input.

After Service confirms controller takeover, the external runner clicks inside
the frozen synthetic marker region and requires all-white rendered pixels. It
then sends Enter and requires the original exact pixel hash. Separate mouse and
keyboard screenshot receipts become `remoteInteraction` in the human receipt.
No remote DOM mutation or direct target API supplies the acknowledgment. A
rejected Service takeover stops before input even when HTTP status is 200.
The cursor is moved outside the crop before visual comparison.

`pnpm test:p159-synthetic-remote-input` exercises the actual fixture and verifier
in disposable real Chrome. Removing the fixture acknowledgment reproduced a
missing-mouse failure; the completed implementation passes. Blocked mouse input,
blocked keyboard input, wrong baseline pixels and scripted clicks cannot pass.
The external-runner and synthetic-fixture suites also pass. This proves the
verification mechanism locally, not remote transport acceptance. Private logs
and source hashes are retained under
`campaigns/p159/custody-outcome/input-verifier/`.

No installed runtime, current retained browser, fixture server or protected
secret has changed in this slice. The existing server still serves its earlier
fixture bytes. Next: refresh that exact owned synthetic server/page, bind the new
attestation and unchanged browser identity, and perform one protected verification
with the new diagnostic custody and actual input oracle. This is a changed
verification seam; historical external failure remains unaccepted. W2 logging,
owner counterexamples and W5 delivery work remain open.

## Checkpoint 9: concurrent viewing passes; controller input remains unproven

Progress: `outcome_progress` for the selected concurrent-view transitions;
overall W3 remains unaccepted.

The exact synthetic server was refreshed and the original authorized handle
reloaded its existing fixture page. Current owner generation and process digest
still match the retained browser. Served fixture attestation matched the new
source bytes before updating the protected attestation secret. No replacement
browser or handoff was acquired. The installed binary remains `7df13c4085f9`.

Protected readiness run
[34010337271](https://github.com/CochranResearchGroup/agent-browser/actions/runs/34010337271)
finished with failure against verifier commit `d75d79ca`. The concurrent client
passed initial, concurrent and reconnect observations with unchanged expected
identity and zero physical launch delta. The prior console error did not recur;
this is not proof that its unknown cause was repaired.

The human client passed initial pixels and received successful controller
takeover, then failed the verifier's immediate baseline-pixel comparison before
any mouse or keyboard action. Its receipt records a second WebSocket connection.
The old verifier neither waited for post-takeover pixels nor retained the failing
crop, so transient reconnection is a hypothesis, not an established cause.
No input acceptance or overall external pass is claimed. Both clients report
zero retries; original failed artifacts remain under
`campaigns/p159/custody-outcome/input-external/`.

A real-Chrome regression with delayed baseline restoration reproduces the
premature failure. The verifier now waits within its existing deadline for the
exact baseline before sending the first input. On failure it retains the last
crop, full page screenshot and a phase-specific failure code. Pixel polling
does not repeat controller takeover or input. The real-Chrome verifier and
external-runner suites pass. This is a scoped diagnostic/readiness correction,
not proof that the failed external transition is repaired.

The selected fixture server identity is captured in the private post-run
owned-state receipt; revalidate before cleanup. The original
synthetic browser and server remain explicit cleanup obligations. W2 repair is
underway independently; no candidate publication accompanied this external run.

## Checkpoint 10: repeated stream enable no longer creates a false failure

Progress: `blocker_reduction`; installed logging verification remains pending.

Source tracing confirmed that automatic named-session recovery invokes
`stream enable`, including when Profile configuration requires a lane refresh
and the listener is already running. The handler previously rejected that
successful existing state, producing the two generic uncertain failures noted
in checkpoint 1.

`stream enable` now returns current status for an omitted port, zero, or the
existing bound port. The real-listener regression failed on the original
already-enabled rejection and now passes, proving the same listener allocation,
port and metadata bytes survive repeated calls. A different explicit port still
fails, and malformed/out-of-range explicit ports cannot become implicit reuse.
This eliminates the selected false failure at its cause without assigning a
guessed actor or rewriting historical journal entries.

The focused Rust test, workspace clippy with warnings denied, format check and
docs build pass. Help, README, repository skill, streaming docs and inline
comments describe the behavior. Private red/green, clippy and docs logs are under
`custody-outcome/input-external/stream-repair/`. The earlier baseline-verifier
repair logs are under `baseline-repair/`.

This repair is source-verified and has not been built into or published as an
installed candidate. The active external browser remains on the prior binary.
Finish the selected W3 verification before controlled fixture cleanup and any
candidate replacement; retain the exact browser/process cleanup obligations.
W1 owner counterexamples and the consolidated W5 production proposal remain
required work. Production replacement has not been performed.

## Checkpoint 11: input reaches a primary-connection lifetime defect

Progress: `blocker_reduction` through a specific causal finding; W3 is open.

Protected readiness run
[34010718428](https://github.com/CochranResearchGroup/agent-browser/actions/runs/34010718428)
finished with failure against verifier `d46ddc71` and unchanged installed binary
`7df13c4085f9`. The original handle and synthetic fixture were checked beforehand;
no replacement, calibration anchor or secret change accompanied dispatch.
The slow client passed. The human client passed its post-takeover baseline,
sent its one mouse action, and failed `synthetic_remote_input_mouse_missing`.
Keyboard input was not sent. The retained failure screenshot shows Guacamole's
disconnected overlay. Extending the pixel wait again is not the selected repair.

The console error's exact SHA-256 matches the fixed browser message
`WebSocket is already in CLOSING or CLOSED state.` Source coordinates point to
the Guacamole WebSocket tunnel's send call. This also identifies the formerly
unclassified error in run 34009455909; it does not retroactively accept that run.
Selected provider logs show the authenticated primary disconnecting, then both
shared tunnels disconnecting within six milliseconds. A later primary connection
starts another RDP client. Raw provider logs, screenshots, redacted transport
receipts and adjudication remain private under
`custody-outcome/baseline-external/`.

Current `guacamole-connection-sharing.ts` elects a viewer's direct frame as the
primary and obtains restricted sharing keys from it. Closing that viewer can
therefore invalidate peers even while their browser, route and Service leases
remain valid. The previous calibration anchor hid this lifetime dependency.
The selected next W3 unit is product-level primary-connection ownership and
peer continuity. Establish that lifetime at the existing provider/Service
boundary, bind it to the exact browser/route and cleanup obligations, and verify
primary-viewer departure before another external attempt. Do not add a harness
anchor, waive closed-socket errors, change the expected pixel oracle, or repeat
the failed external loop unchanged. Any new endpoint or worker must have its
ownership and shutdown contract recorded here before implementation.

Separately, the real-dispatch retained-handle fixture now tests missing owner
registry evidence and missing process evidence as well as a changed endpoint.
All return their expected typed recovery failures, `no_effect` and executable
recourse before any CDP method, without installing a browser manager. Restoring
valid evidence still recovers the original target; an absent target still cannot
create or select a peer. Both focused tests, Rust format and workspace clippy
with warnings denied pass. Private logs are in `baseline-external/owner-counterexamples/`.
This closes the selected guard-level counterexample gap; it is not a new live
owner-mutation campaign or a universal logging claim.

W2's compatible stream-enable repair still needs installed custody. The
synthetic browser/server remain retained obligations. The provider ownership
repair and W5 consolidated delivery decision remain required; production is
unchanged and the old terminal-target proposal remains unexecuted.

### W3 implementation contract: backend-owned provider primary

Guacamole 1.5.5's [sharing contract](https://guacamole.apache.org/doc/1.5.5/gug/using-guacamole.html#sharing-the-connection)
explicitly ties shared access to the original connection. The repair will own
that connection in the dashboard backend, using the existing authenticated
primary-claim endpoint and trusted local provider configuration. This is product
connection ownership, not a calibration client or additional browser process.

Before provider effects, resolve the exact route/connection/browser from current
Service authority and validate the original process/owner binding. Never accept
an arbitrary caller URL or provider credential. Serialize startup per exact
binding and let every viewer use a restricted sharing connection. A viewer page
must no longer receive a direct-primary election grant. Connection tokens stay
in memory and are excluded from response, logs and persisted evidence.

The owner consumes only provider display/protocol traffic; it sends no desktop
input and creates no browser or target. Its task belongs to the backend process,
has bounded startup and shutdown, and is cancelled when the exact route/browser
binding becomes invalid or terminal. A failed owner is an explicit failed state,
not an automatic reconnect loop. Stale or foreign bindings fail before effects.
Tests must prove that dropping viewer requests does not drop the primary,
concurrent requests do not start duplicate primaries, and binding invalidation
does close it. Installed proof must then cover peer departure and actual input
through the same durable URL before W3 acceptance.

Implementation in progress: the receive-only protocol and backend task modules
are currently compiled only for tests. Eight focused tests pass, including the
two existing primary-election tests. The new tests exercise real WebSocket
framing over an in-memory transport, cancellation of a pending viewer wait,
binding invalidation, stale readiness after task abortion, and bounded protocol
parsing without desktop input. Workspace clippy with warnings denied and the
format check pass; clippy's ordinary workspace selection does not lint these
test-only modules. Logs are `/tmp/p159-primary-protocol-tests.log` and
`/tmp/p159-primary-clippy.log`.

The authenticated endpoint, exact Service-binding admission, duplicate-start
serialization, local provider connection and shared-only frontend remain to be
wired. These tests do not establish installed provider ownership, peer continuity,
or external input acceptance. No provider dispatch or runtime publication was
performed for this implementation step.

The exact-binding module now reuses `RuntimeLifecycleAuthority` and validates
the route, connection, original process digest, endpoint digest and display
ownership. Its regression uses current process evidence and the real lifecycle
gate against an in-memory Service repository. Viewer/controller lease changes
preserve the binding; missing process evidence, changed endpoint, display
reassignment, missing lifecycle/owner evidence and foreign route/connection
selectors fail. Provider origins require a literal loopback address and the
expected Guacamole path, with no URL credentials or query. Ten focused tests,
workspace clippy with warnings denied and format check pass. Current logs are
`/tmp/p159-primary-binding-tests-final.log` and
`/tmp/p159-primary-binding-clippy.log`. All three new modules remain test-only.
The logs and a source-hash receipt are preserved in the private P159 campaign
directory `primary-owner-source-validation/` for durable evidence custody.

Integration must also replace the frontend's oldest-eligible-connection
selection with the backend owner's exact active-connection identity. A lingering
viewer-owned primary is not an acceptable sharing-key donor just because its
connection ID matches. Resolve the provider's tunnel-to-active-row identity
contract before wiring key issuance. Startup must belong to the backend task
even while authentication/connection is pending; request cancellation must not
cancel that startup or permit duplicate admission. These are remaining
implementation requirements, not reasons to dispatch another external run.

## Checkpoint 12: backend primary ownership integrated in source

The primary modules now compile in the ordinary runtime. The authenticated
primary endpoint accepts `ensure`, resolves exact Service authority, and retains
one backend task per provider origin/connection. Authentication and WebSocket
startup belong to that task, so cancelling a pending viewer request cannot
cancel startup. The task has one total startup deadline, bounded sends/shutdown,
and periodic exact-binding checks. Failed entries are retained without automatic
retry; a changed binding must wait for the prior owner to stop before replacement.
Legacy viewer-election and connected-confirmation requests fail before effects.

Guacamole 1.5.5's WebSocket endpoint emits the tunnel UUID in its initial internal
instruction. JDBC `ActiveConnectionRecord` preserves that UUID on the tunnel and
`TrackedActiveConnection` exposes it as the active-row identifier. The source
readback is retained privately; upstream references are
[WebSocket endpoint](https://github.com/apache/guacamole-client/blob/1.5.5/guacamole-common/src/main/java/org/apache/guacamole/websocket/GuacamoleWebSocketTunnelEndpoint.java)
and [tracked active connection](https://github.com/apache/guacamole-client/blob/1.5.5/extensions/guacamole-auth-jdbc/modules/guacamole-auth-jdbc-base/src/main/java/org/apache/guacamole/auth/jdbc/activeconnection/TrackedActiveConnection.java).
The protocol validates and freezes that UUID. Readiness requires it plus a
consumed sync frame. The frontend obtains keys only from that exact UUID and
never falls back to an older or viewer-owned primary. Backend failure codes
survive into the existing dashboard error-reporting path without raw provider
responses or credentials.

Ten focused Rust tests pass, including cancelled pending startup, exact identity
and lifecycle guards, immutable tunnel identity, terminal task status and legacy
endpoint rejection. Workspace clippy now covers the enabled modules and passes
with warnings denied. Format check, dashboard type check, dashboard build, docs
build, resolver custody tests and the mounted observer-frame regression pass.
The latter proves that observer acknowledgement preserves the same iframe and
explicit reload replaces it. Legacy election cases were replaced with exact
backend-donor, concurrent/reopen, maturity, missing/vanishing donor, rejected key,
authorization failure and cancellation coverage. The first mounted-fixture
attempt still supplied a direct-election grant; its next revision changed the
primary start timestamp on each read and correctly failed stability checks.
The final fixture freezes the timestamp and supplies backend ownership. Those
failed attempts remain diagnostic evidence. Two clippy findings in the newly
enabled transport were fixed before the passing check.
Moving the authority gate before connection startup also changed the invalid-
binding fixture from a WebSocket close frame to transport closure; its assertion
now accepts closure while still rejecting every provider acknowledgement.
Logs, upstream source readbacks and the source-hash receipt are preserved under
the private P159 campaign directory `primary-owner-integration/`.

This is source integration, not W3 acceptance. Remaining immediate gates are
the provider adapter's authentication/handshake and registry duplicate-start
boundary tests, exact retained-fixture disposition, isolated installation and
real provider peer-departure/input proof. No new external dispatch, provider
mutation or candidate publication accompanied this checkpoint. Production and
the retained browser/server remain unchanged by this slice. W2 installed
verification and W5 delivery/cleanup obligations remain open.

## Checkpoint 13: provider and registry boundaries verified

Twelve focused Rust tests pass. A disposable loopback HTTP/WebSocket server now
verifies the real adapter's header principal, exact connection/data-source query,
Guacamole subprotocol and tunnel UUID readiness. A foreign authenticated
principal is rejected before WebSocket startup. The provider receives only
protocol acknowledgements/keepalive, with no desktop input. The registry test
uses concurrent admissions: both receive the same pending task, a failed owner
does not restart for the same binding, and reassignment cannot start a new task
until the old task closes. This is synthetic protocol-boundary proof, not a
real Guacamole or RDP acceptance result. Workspace clippy with warnings denied
also passes.

Read-only development status still selects `0.28.0-7df13c4085f9` and reports ready.
The retained synthetic Chrome and fixture-server PIDs still exist; that alone
does not re-prove their start tokens, handles or cleanup. The next operation is
candidate build, followed by exact fixture disposition and isolated installation.
Status custody is under private `primary-owner-candidate/`; boundary proof logs
are preserved under `primary-owner-boundary/`. No installation or live provider
effect has occurred at this checkpoint.

## Checkpoint 14: isolated candidate installed; selected W2 repair verified

The optimized candidate built from `76997cda` is installed only in P158 as
`0.28.0-1698c2f6c4a9`, SHA-256
`1698c2f6c4a9a08486a7c208e9349bc83636550dd395c5ee003a56ae16033fd5`.
The installer verifies production and default-development custody unchanged.
The exact development doctor and three disposable browser-launch smokes pass.

Before installation, the retained synthetic browser answered through its
original authorized handle. A fresh target inventory contained the synthetic
fixture, blank/new-tab pages and Chrome internal UI. The exact browser was then
closed through `service_browser_close` with its original handle/session and
labels. Its Service row is absent afterward; all fourteen captured process
start identities are gone. This is a deliberate fixture closure, not a
same-browser upgrade or same-URL reopen pass. The synthetic fixture server
remains retained for subsequent provider verification. Captured-tree cleanup
does not assert that every namespace-wide ancillary process is reclaimed.

On the installed candidate, two successive `stream enable` requests against
the existing `development-default-p158` lane succeed, both reporting enabled
port 5151. The supervisor manifest hash remains identical. Together with the
earlier real-listener identity regression, this closes the selected W2
compatible-stream-enable installation gap. It does not establish universal
failure-log coverage or erase the historical uncertain failures.

Private `primary-owner-candidate/` contains build/candidate identity, before
status, original-handle readback, target/process inventory, durable close intent
and response, cleanup readback, installation receipt, doctor, launch smokes and
stream-enable readbacks. W3 still requires a new explicitly scoped synthetic
fixture, actual provider primary/peer-departure/input evidence and the protected
external outcome. No new external dispatch was made. W5 repair inventory,
production-update decision and final cleanup remain open.

## Checkpoint 15: live adapter finding and authentication-source correction

A new synthetic fixture was acquired through a fresh broker access plan on the
installed candidate. An invented explicit session alias was rejected during
read-only planning; the executable broker-selected session was used instead.
The fixture is operator-ready and its original authorized handle answers before
and after the provider startup rejection below. It remains a retained cleanup
obligation under private `primary-owner-live/`.

The first diagnostic ownership request supplied HTTP Basic authentication,
which this dashboard does not accept; the next used the wrong login path.
Both stopped at dashboard authentication. Using the actual dashboard login
endpoint and returned session cookie reached the primary endpoint. The route
had zero matching provider connections before this request. Startup failed
`guacamole_primary_provider_auth_identity_mismatch` before WebSocket creation.
No unchanged owner restart or external dispatch followed.

The live header-auth response has the correct username, `dataSource=header`,
and `availableDataSources=[postgresql,postgresql-shared]`. The adapter had
incorrectly required the authentication extension itself to be `postgresql`.
It now keeps the exact username check and independently requires `postgresql`
among available connection directories. The connection query remains pinned to
that directory and the exact Service connection ID. Missing directory access
returns `guacamole_primary_provider_data_source_unavailable`.

The protocol-boundary regression was red on the actual header-auth shape before
the correction and is green afterward, including foreign-principal and missing-
directory negatives. All twelve focused Rust tests and workspace clippy with
warnings denied pass. Red/green/clippy evidence is preserved in private
`primary-owner-auth-source-repair/`. The next step is a corrected candidate build
and isolated publication with explicit retained-fixture disposition. Real
primary lifetime, peer departure and external input acceptance remain open.


## Checkpoint 16: primary startup survives the dashboard ingress budget

The authentication-source correction from `40023d88` was built and installed
only in P158 as `0.28.0-a2aa0e84cf34`, SHA-256
`a2aa0e84cf3415b28badbaa50b5cf5b2b1d39d5a74fa4614879770d9b6e06ef3`.
Its installation, doctor and disposable launch receipts are retained under
private `primary-owner-auth-fix-candidate/`. The preceding synthetic browser
was deliberately closed before installation; the new fixture and its startup
ledger are under `primary-owner-verified-live/`.

The authenticated primary request returned HTTP 502 `mutation_outcome_unknown`
with `retrySafe=false`. Provider reconciliation found one connectable primary;
a fresh readback during this repair still shows the same UUID and start time.
No unchanged startup POST was repeated. This proves retained provider connection
identity, not visible pixels, input or healthy original-handle recovery.

The cause is a mismatched timeout contract: `PrimaryTask::ready` can wait sixteen
seconds, while ingress gave this POST the generic two-second first-response
budget. The existing request-specific ingress policy now allows twenty-one
seconds for the exact primary endpoint, including five seconds of response
grace. Mutation retry/fallback rules are unchanged. This does not extend pixel
verification or authorize another protected external attempt.

The existing delayed-presentation proxy test now covers both durable-handoff
resolution and primary startup through the real ingress. It reproduced the
502 before the repair and passes afterward. All 29 focused ingress tests pass,
including delivered-mutation uncertainty and no-fallback-replay cases. Workspace
clippy with warnings denied, format check and docs build pass. Private
`primary-owner-ingress-repair/` retains red, green, clippy and docs logs. Help,
README, the repository skill, remote-view docs and inline guidance describe the
bounded startup response and reconciliation requirement.

This source repair has not yet been installed. Next, build the corrected
candidate, disposition the exact retained fixture before publication, and
verify primary/peer continuity and actual input through the intended installed
surface. Review selected primary-endpoint failure custody before claiming W2
complete. W3 external outcome evidence and W5 production proposal/final cleanup
remain open. Production was not changed by this repair.


## Checkpoint 17: installed primary startup passes through stable ingress

Candidate `9b975aec` is installed only in P158 as `0.28.0-0552574f397a`,
SHA-256 `0552574f397a95b63b561a5454ddf34b6c77fd72e1513da4c84546090e654c39`.
The optimized build, isolated installer, doctor and three disposable launch
smokes pass. The installer retains production and default-development custody.

Before publication, the previous synthetic browser answered through its
original authorized handle. Its exact process identity and target inventory
were checked, then Service closed that browser. The Service row is absent and
all fourteen captured process start identities are gone. This captured-tree
receipt does not claim namespace-wide ancillary cleanup. Evidence is under
private `primary-owner-ingress-candidate/`.

A fresh broker access plan selected the next synthetic session; remote-view
open returned operator-ready and its original handle answered. Before the one
primary-start POST, the provider had zero matching connections. Through stable
ingress port 5148, that POST returned HTTP 200, `primaryOwned=true` and
`granted=false`. The returned UUID matches the provider's active connection.
The durable ledger records 2.610 seconds from intent to response, exceeding
the former two-second ingress limit. No retry was used. Evidence, full handles
and the retained synthetic fixture obligation are private under
`primary-owner-ingress-live/`.

This closes the installed primary-start response defect. It does not establish
shared-viewer lifetime, actual pixels/input, same-URL reopening or W3 acceptance.
Next bind the retained synthetic fixture and exact candidate to the protected
external readiness proof, preserving prior failed attempts and the frozen pixel
oracle. No external workflow was dispatched in this checkpoint. The selected
primary-endpoint failure-custody review and W5 production proposal/final cleanup
remain open.

## Checkpoint 18: external proof exposes idle primary loss

Protected readiness run `34014574632`, workflow source `4c902d8b`, is FAILED.
It used installed candidate `0552574f397a`, the exact retained synthetic browser,
fresh original-handle/input-ready and durable-resolution identity checks, and
the existing historical pixel oracle. Six protected environment secrets were
bound from private sources. No calibration, anchor or retry was used.

Both external clients failed `external_stream_not_embeddable`, with zero
iframes and zero provider WebSockets observed. The screenshots show the ordinary
authenticated dashboard reporting Stream unavailable. The primary endpoint
returned 503. Neither client reached pixel/input/peer-departure acceptance.
No successful aggregate can be inferred from these partial artifacts.

Read-only provider reconciliation found no matching primary. Provider logs show
its disconnect at 05:39:37 UTC after 126216 milliseconds, with guacd reporting
"User is not responding". This precedes the external clients' startup around
05:41:51 UTC. The primary-start response pass remains valid, but it does not
prove idle lifetime. Root cause is not yet established. No unchanged primary
POST, backend restart or further external run followed this result.

The original authorized handle still answers after the failed run, with the
synthetic input marker still ready. This isolates the observed failure to
presentation rather than proving a browser-control failure. The exact fixture
remains retained with a cleanup obligation. Private `primary-owner-ingress-live/
external/` retains bindings, secret-update metadata, dispatch intent/receipt,
run status, client artifacts, screenshot review, provider/backend logs,
post-run original-handle result and adjudication.

Source review also confirms missing product failure custody at the new primary
endpoint: it returns a typed error without appending its own correlated record.
The client journal retained generic HTTP 503 and connection-sharing failures,
which do not establish the owner's terminal cause. Next repair primary lifetime
observability and verify the idle keepalive/acknowledgement path at the protocol
boundary before another installed or external attempt. Apache Guacamole 1.5.5
Client.js confirms that its client uses five-second nop keepalives; matching
that instruction alone does not prove our task continues delivering them.
Relevant upstream source snapshots are retained with the private evidence.
W3 remains unaccepted; W2 selected custody and W5 delivery/cleanup remain open.


## Checkpoint 19: primary terminal custody and idle transport boundary

The existing transport lifetime test now waits for two provider-visible nop
keepalives after startup waiters have returned, with only the sync acknowledgement
allowed alongside them. That idle boundary passes without an opcode or timing
repair. This narrows the investigation: the transport can keep a quiet mocked
provider alive, but the real 126-second disconnect is still unexplained.

Production registry admission now installs a terminal observer on its owned
task. Normal termination persists a private failure-journal record before
publishing Closed. The record contains the static typed cause, elapsed lifetime,
and route/session/display references. Provider credentials, URLs and display
payloads are not included. The observer also records task cancellation or
unwinding once; SIGKILL and process abort cannot run this observer. Existing
failed-owner stickiness and binding invalidation semantics are unchanged.

The lifetime test verifies terminal observation precedes Closed, and the
existing aborted-owner test verifies cancellation custody alongside stale-ready
rejection. No new live primary, installation or external retry occurred in this
slice. The existing retained browser remains a cleanup obligation. This repair
covers owner terminal custody; immediate endpoint admission failures and response
correlation still require their selected W2 review. Installed idle lifetime,
external W3 outcomes and W5 delivery remain open.

All twelve focused primary tests pass, including the idle and cancellation
boundaries. Workspace clippy with warnings denied, format check, docs build and
remote-view handoff documentation checks pass. Logs are retained privately in
`primary-terminal-custody/`. Next bind the terminal record to the endpoint's
failed response and complete the selected custody regression before building
and installing another candidate. No live lifetime repair is claimed here.


## Checkpoint 20: primary responses correlate to endpoint and owner custody

Each failed authenticated primary request now persists a request observation
before returning its `occurrenceId`. The record retains the typed code, route
reference, authenticated actor hash, reconciliation recourse and `retrySafe=false`.
Invalid or retired admission requests also receive records. Successful responses
do not create failures, and request credentials or raw actor names are excluded.

A backend owner allocates one terminal occurrence ID before startup. Its
terminal observer uses that ID, and responses from the failed owner include it
as `terminalOccurrenceId`. Separate requests get separate request occurrence IDs
while referring to the same owner event. A waiter timeout does not assert owner
termination. Existing failed-owner admission remains sticky without restart.

All thirteen focused primary tests pass. The extended lifetime test matches the
observer's ID to its owner, and the endpoint custody test covers repeated request
correlation, malformed admission, success exclusion and private-field exclusion.
Workspace clippy with warnings denied, format check, docs build and remote-view
handoff documentation checks pass. Private `primary-response-correlation/`
retains validation logs. All five documentation surfaces describe the correlation
fields. No new live request, candidate publication or external retry occurred.

Next build and install this combined custody candidate after exact disposition
of the retained synthetic browser, then perform one bounded isolated primary
lifetime observation. Use the owner record to adjudicate any closure before a
new external run. The real disconnect's cause and W3 acceptance remain unresolved;
W2 installed custody verification and W5 delivery/final cleanup remain open.


## Checkpoint 21: installed custody isolates handoff resolution as the trigger

Candidate `b5faee54` is installed only in P158 as `0.28.0-3161b1e55833`,
SHA-256 `3161b1e5583344b27a96ae2dd828c97f39fd5d6b80351429988fd77f6babe5aa`.
The build, isolated installer, doctor and three launch smokes pass. Before
publication, the preceding retained browser answered its original handle, then
was closed through Service after exact identity/target checks. Its row is absent
and all fourteen captured process identities are gone. This is captured-tree
cleanup, not namespace-wide ancillary clearance. Receipts are private under
`primary-custody-candidate/`.

The fresh broker-selected synthetic browser and original handle passed. Its
one primary startup returned HTTP 200 with an exact provider UUID. A bounded
read-only provider/journal observation retained that UUID as connectable for
150299 milliseconds, with no terminal record. Thus idle time alone did not
reproduce the preceding disconnect.

One supported resolution of the same live durable handoff then returned ready
with the same browser and target identities. During that operation, the primary
terminated. Provider readback found it absent, and the new private owner record
reports `guacamole_primary_binding_changed`, lifetime 206720 milliseconds.
Expected terminal records: 1; observed: 1; missing: 0; duplicates: 0. This is
installed terminal-custody evidence. Endpoint response correlation still needs
its selected installed check.

The temporal comparison now points to handoff resolution invalidating the primary
binding, rather than elapsed idle time. The exact changed/rejected binding field
is not yet adjudicated; final route/display summaries retain the same identity.
Inspect resolution's intermediate authority and route mutations before changing
the guard. Preserve exact owner/process/display protections; do not waive the
binding check or restart the sticky owner to hide this failure.

Private `primary-custody-live/` retains open/start receipts, the 150-second
observation ledger, same-handoff resolve intent/response, terminal record and
adjudication. No external dispatch or startup retry occurred. The new synthetic
browser remains retained for exact disposition; W3 and W5 remain open.


## Checkpoint 22: retained primary continuity across acquisition reservation

The coordinator always calls `begin_route_bound_handoff_plan_acquisition` during
durable resolution. That function atomically records the previous ready route
and display, creates a pending acquisition lease and marks the current route
pending. The primary's readiness-only guard treated this temporary reservation
as binding loss. The retained live receipt confirms its prior route was ready.

New-primary admission still requires ready state. Existing-primary continuity
now separately accepts a pending route only when its readiness locator names
a current-boot active acquisition with the exact browser/session/route/display
and previous browser-display binding. The lease must retain the exact prior
ready provider connection/origin and owned display, with no terminal timestamp.
The usual live owner-generation, process, endpoint and display checks still run.
Released routes, missing or failed leases, and changed identity remain fenced.
Existing registry lookup uses this continuity check before new-owner admission,
so a concurrent viewer may reuse the owner during proven revalidation.

A regression invokes the actual acquisition reservation on a disposable JSON
Service repository. It failed before the repair because the unchanged retained
primary was invalidated, and passes afterward while new-primary admission is
still rejected. Negative cases cover absent/failed lease proof, foreign prior
route, foreign display, changed owner generation and released route. Registry
coverage preserves exact existing-task reuse and rejects a noncurrent or foreign
route. All fourteen focused primary tests, workspace clippy with warnings denied,
format check, docs build and handoff documentation checks pass. Private
`primary-reservation-continuity/` retains red/green and validation evidence.

All five guidance surfaces describe retained continuity versus new admission.
No guard waiver, primary restart or external attempt accompanied this source
repair. Next build and install after exact fixture disposition, then verify the
same primary UUID survives supported same-handoff resolution on the installed
candidate. W3 external input/reopen evidence, selected installed W2 correlation,
and W5 production delivery/final cleanup remain open.

## Checkpoint 23: installed reservation repair fails at the inventory boundary

Source `13706923` was built and installed only in P158 as
`0.28.0-f9d0f9092662`, binary SHA-256
`f9d0f90926620f747225a5aecf28a6970163887c8b9bd2ed05942283baeded38`.
Private `primary-continuity-candidate/` retains installation, doctor and three
successful launch smokes, plus the preceding fixture's exact disposition.

Private `primary-continuity-live/` proves one authenticated malformed primary
request returned HTTP 400 with exactly one matching journal occurrence:
expected 1, observed 1, missing 0, duplicates 0. This accepts the selected
installed admission-correlation check, not every W2 failure path.

The new synthetic browser's primary startup succeeded. One same-handoff
resolution returned ready with the same browser and target, but the owner
terminated with `guacamole_primary_binding_changed` after 14285 milliseconds.
The single terminal record falls between acquisition reservation and completion.
The provider subsequently reported no matching connection. The installed
continuity requirement therefore failed despite the preceding focused test pass.
No new external dispatch or sticky-primary restart followed this failure.

Source inspection found that JSON repository reads apply provider inventory
after loading durable state. That overlay preserves browser/session custody
only for ready routes; it rebuilds pending reservations as unowned ready routes.
The previous regression exercised the actual reservation but omitted this
installed read boundary. Extend that regression with the real inventory overlay
before accepting another repair. This is a source-backed explanation to test,
not yet a passing installed outcome. W3 and W5 remain open.

## Checkpoint 24: inventory overlay regression and reservation repair

The extended existing primary regression invoked the real acquisition and then
the real provider inventory overlay. Before repair it failed in 0.02 seconds
with `provider inventory erased retained reservation custody`. The initial
invocation with an incomplete exact test name selected zero tests and is not
counted as evidence; the corrected filtered invocation executed one failing test.

Inventory refresh now preserves route ownership and pending state for an active
current-boot acquisition whose browser/session/route/display and readiness
locator match exactly. Terminal timestamps and unsupported phases reject that
proof. The display and occupied pool slot retain their pending states as well.
Provider availability cannot promote the reservation to ready. New-primary
admission and the existing primary's owner/process/display guards remain intact.

The final focused primary run passes all fourteen tests, including the actual
reservation/overlay boundary, pending slot preservation and missing, failed,
stale-boot, foreign-session and terminal-phase lease rejection. All eight
inventory tests, workspace clippy with warnings denied, format check, docs build
and handoff documentation check pass. All five guidance surfaces are updated.
Private `primary-inventory-reservation/` retains validation receipts.

A fresh read through the retained synthetic browser's original handle still
returns the expected fixture and ready input marker after the primary failure.
That isolates the observed failure to presentation continuity; it does not prove
remote-view pixels or input. This repair is source-verified only. Next dispose
of that exact fixture, publish the isolated candidate, and verify the same
primary UUID survives same-handoff resolution before any external readiness
attempt. W3 external input/reopen and W5 delivery/final cleanup remain open.

## Checkpoint 25: primary continuity survives but checkout loses capacity

Candidate `3ad29c44` is installed only in P158 with SHA-256
`dca7acc08af9ec8fb2b439fa68dd0e0ab5ca874756cfb9074c231266fb7bac02`.
Build, installer, doctor and three launch smokes passed. The preceding fixture
closed through Service after current process and synthetic target checks; its
browser row is absent and all fourteen captured identities are gone. Private
`primary-inventory-candidate/` retains these receipts.

Fresh broker-selected acquisition and the original handle passed. One primary
startup returned HTTP 200. The single same-handoff resolution then failed with
`checkout_failed`, underlying `presentation_bound_slot_missing`. Its exact
acquisition rolled back, restoring the previous route, display and pool row.
The provider still reports the original primary UUID connectable, with zero
owner terminal records for this session. This advances primary continuity but
does not accept the failed resolution. No resolution retry or external dispatch
occurred. The synthetic browser remains retained.

The inventory overlay rebuilds capacity from ready rows before copying prior
slot details. Preserving pending reservation rows therefore excludes the slot
from that reconstruction; checkout later cannot find it. The regression now
executes the same capacity activation called by checkout. It reproduces
`presentation_bound_slot_missing` in 0.01 seconds before repair. The bounded
repair preserves only an existing exact browser/route/display/provider-slot
binding during a current acquisition, including its scene generation and lease.
It does not create missing capacity. Source validation and installed verification
must finish before this new checkout outcome can be accepted. Private live
evidence is under `primary-inventory-live/`.

The capacity repair passes fourteen focused primary tests and fifty-five
selected presentation tests. It preserves the prior scene generation and
recovery lease, and rejects missing capacity and foreign browser/display/route/
slot bindings. Workspace clippy with warnings denied, format, docs build and
handoff documentation checks pass. Clippy initially rejected a redundant clone
on a Copy type; that was removed before the final passing run. Private
`primary-capacity-reservation/` retains red/green and validation receipts.

Before the next isolated publication, the failed-checkout fixture was closed
through Service after exact current process and synthetic target checks. Its
row is absent and all fourteen captured process identities are gone. Receipts
are under `primary-capacity-candidate/`. This is captured-tree disposition;
the fixture server and other retained cleanup obligations remain explicit.

## Checkpoint 26: installed resolution and shared primary continuity pass

Source `d0b5f298` is installed only in P158 with binary SHA-256
`e12e7f4da2dcdc61d1d89d0048bbef8d4d6c72d399dc27a080c8c9f18a1b0a0e`.
The optimized build, isolated installer, doctor and three launch smokes pass.
Private `primary-capacity-candidate/` retains the exact candidate and receipts.

Fresh broker-selected synthetic acquisition, operator readiness and the original
handle pass. One primary startup returned HTTP 200. One resolution of the same
live durable handoff returned ready with the same browser and target. Provider
readback retained the original connection UUID as connectable; two concurrent
authenticated primary requests then reused that exact UUID. There are zero
owner terminal records for the selected session. This passes the combined
installed resolution/primary-continuity gate, not external pixels or input.

The next protected readiness attempt is run
[34017270853](https://github.com/CochranResearchGroup/agent-browser/actions/runs/34017270853),
dispatched once against exact source `d0b5f298` after that installed proof.
Its bindings retain the historical pixel oracle, exact synthetic identity,
synthetic-only capture attestation and readiness mode without a calibration
anchor. Six protected-environment secrets were updated privately. Dispatch
intent and readback are under `primary-capacity-live/external/`. Its initial
readback was queued; external acceptance remains pending both clients and the
aggregate receipt. The browser and synthetic server are intentionally retained
for this run. No production replacement or calibration occurred.

## Checkpoint 27: initial external pixels pass; takeover/concurrent transition fails

Protected run 34017270853 completed with both client jobs and the aggregate
failed, with zero runner retries or repairs. Both clients initially rendered
the historical pixel marker exactly: SHA-256
`7f642adcc83d962dcf542faedfee0a7bd9027bd45aa1bcba2fe6842c1d6ac527`.
All five human and seven slow-client artifact digests and sizes match their
receipts. Direct review of the human initial screenshot shows the synthetic
browser inside the ordinary authenticated dashboard. Initial visibility passed;
the full W3 outcome did not.

The primary terminated at 06:45:36.912 UTC, lifetime 146161 milliseconds, with
`guacamole_primary_binding_changed`. This falls during the human controller
takeover request and the slow client's second-page handoff resolution. The
human then failed with `Synthetic input requires exactly one remote frame`;
the slow client failed to embed its concurrent page. Provider readback afterward
has no matching primary. Input, concurrent continuity and reopen remain
unaccepted. No unchanged dispatch or sticky-owner restart followed the failure.

The retained acquisition timestamps do not show overlapping pending leases at
termination: the preceding acquisition completed at 06:45:31.402 and the next
began at 06:45:37.755. Do not attribute this failure to nested reservations.
Controller takeover normally changes viewer/controller authority, which the
binding contract excludes from primary identity. The exact rejected guard
condition remains unknown. In particular, `is_current` currently collapses
repository-read errors and identity rejection into the same boolean; the
repository has a one-second default lock budget. Contention is a hypothesis,
not a confirmed cause or permission to waive guards or increase timeouts.

The journal retains exactly one owner terminal occurrence and two distinct
endpoint occurrences referencing it. This is installed owner-to-endpoint
causal custody; the redacted external HAR does not retain response occurrence
fields, so it is not a full external response-body correlation proof.

Private `primary-capacity-live/external/` retains the two client artifacts,
aggregate, owner/endpoint records, provider readbacks and hash verification.
Next distinguish typed primary guard rejection causes with a local regression
before selecting another live repair verification. The synthetic browser and
fixture server remain retained cleanup obligations. W3 and W5 remain open.

## Checkpoint 28: preserve the specific primary guard rejection

The primary's boolean guard discarded the reason for rejection before transport
termination. The existing transport regression now supplies a lock-timeout
rejection after proven startup and continued keepalive. Before repair, its
waiting caller received `guacamole_primary_binding_changed` instead of the
supplied cause. The regression failed in 5.25 seconds.

The provider and transport now carry a typed, bounded guard result. Repository
state-read and authority-read timeout/unavailability remain distinct from owner
or binding change through the terminal observer and waiter response. Raw paths,
holder details and provider evidence are excluded. Guard rejection still stops
the primary; no check, timeout or sticky-failure rule is relaxed. Additional
coverage faults each of the two real repository-read boundaries and verifies
their distinct safe codes. This repairs diagnostic custody; it does not yet
identify the cause of run 34017270853 or repair that external transition.

All fifteen focused primary tests, workspace clippy with warnings denied,
format, docs build and handoff documentation checks pass. All five guidance
surfaces are updated. Private `primary-guard-custody/` retains red/green and
validation receipts. The preceding synthetic browser was closed through Service
after exact process/target checks; its row is absent and all fourteen captured
process identities are gone, recorded under `primary-guard-candidate/`.

Next publish this isolated diagnostic candidate and exercise one bounded local
takeover/concurrent-resolution transition with the new terminal custody. Do not
claim the historical guard failure was contention without a typed live record.
The synthetic server remains retained. External W3 and production delivery W5
remain open.

## Checkpoint 29: installed typed guard and local transition diagnostic

Source `3428a7a7` is installed only in P158 with binary SHA-256
`35cb5a62b307f8ea3e2a463479ef5c248caeac62323455a922d04a5ef5ffd28b`.
Build, installer, doctor and three launch smokes pass. A fresh broker-selected
synthetic browser and its original handle pass; one primary startup returned
HTTP 200 with exact provider identity. Private receipts are under
`primary-guard-candidate/` and `primary-guard-live/`.

The first local concurrent probe supplied convenience takeover fields to the
client helper, which retained them both at top level and in params. HTTP rejected
that envelope with `unknown_field` before takeover execution. The peer resolution
succeeded and the primary survived. Preserve this as a probe/client-envelope
finding, not a failed product takeover or proof about the external transition.
The corrected probe puts action-specific fields only in params. Its one takeover
and one concurrent resolution both succeeded, with the same browser/target and
primary UUID, and zero owner terminal records. No repeated local diagnostic was
run after that passing result. The helper envelope defect remains a separately
identified delivery risk; the valid params form is used by this probe.

The local transition did not reproduce the external failure. Protected run
[34018112693](https://github.com/CochranResearchGroup/agent-browser/actions/runs/34018112693)
was therefore dispatched once in readiness mode against this exact instrumented
source to distinguish the external guard rejection. This is a changed custody
diagnostic, not a claim that timeout or identity policy was repaired. It retains
the historical pixel oracle and synthetic capture attestation. Its initial
readback was in progress; external outcome and specific live rejection cause
remain pending. No automatic retry or production replacement occurred.

## Checkpoint 30: diagnostic stopped at a scaled capture-region mismatch

Run 34018112693 completed failed. Both clients had one Guacamole iframe,
successful transport requests and the same observed pixel hash
`13f238a8d65c66de0325dafcded7fefed3e7e954f03ab2ad080e6a75d6ec24e0`.
Both stopped at `external_stream_identity_marker_missing` before takeover or
input. All five artifacts from each client match their recorded byte counts
and digests. No runner repair or retry occurred.

Direct review of the initial screenshot shows the expected synthetic page in
the remote browser, scaled down within the iframe. Direct review of the saved
marker crop shows the orange strip and surrounding page below the blue marker.
The source currently applies the frozen x/y/width/height as iframe CSS pixels;
the backend primary requests a 1920-by-1080 desktop. The crop is therefore not
sampling the intended marker in this rendered geometry. This is a capture-
geometry failure, not evidence that a different browser was displayed. Do not
replace the expected pixel hash with the observed orange-strip hash.

Provider readback still finds the original primary UUID connectable, and the
journal has zero terminal owner records for this session. This diagnostic did
not reach the transition that failed in run 34017270853 and cannot identify or
clear that historical guard failure. The next bounded W4 slice must reconcile
the declared marker coordinates with rendered remote-display geometry, prove
the crop/input mapping locally, and preserve the independent synthetic oracle
before another external diagnostic can reach takeover. The client-helper
envelope finding and W5 production proposal remain explicit delivery risks.

Private `primary-guard-live/external/` retains the failed artifacts, aggregate,
hash verification, terminal-record readback and provider observations. The
synthetic browser and fixture server remain intentionally retained. W3 is not
accepted, production is unchanged, and the goal remains open.

## Checkpoint 31: map the synthetic sample through rendered display geometry

Read-only inspection of the installed Guacamole 1.5.5 WAR confirms that the
client centers its display inside the iframe. `Display.getElement()` returns a
scaled bounding div; its first child retains native desktop dimensions with a
CSS scale transform. The capture/input producer now reads those dimensions and
the rendered bounds. The new `remote-view-display` region describes a native
marker rectangle, including browser chrome, with fixed CSS `sampleWidth` and
`sampleHeight`. Both consumers center the unchanged-size sample inside that
rectangle. Missing geometry waits; ambiguous desktops fail; an undersized
rendered marker cannot pass by shrinking the sample. Input rechecks geometry
after baseline and retains crop coordinates with acknowledgment artifacts.

The existing real-Chrome fixture regression now reproduces a desktop scaled to
0.56 inside an offset, letterboxed iframe. It proves the old fixed crop misses
the marker. Before input integration it failed at the unsupported geometry
boundary; after repair it passes baseline, trusted mouse white acknowledgment,
and Enter restoration with the same 400-by-100 local baseline digest. Existing
negative input cases and the oversized-sample refusal pass. Configuration and
external runner checks, handoff documentation checks, docs build, workspace
clippy with warnings denied, and format pass. All five guidance surfaces are
updated. These are focused local checks, not remote-view acceptance.

A supplementary assertion comparing the direct-Chrome baseline to the historic
RDP digest failed and was adjudicated outside that local test's scope: the source
fixture declares RGB 18/92/142, while the independently read retained RDP PNG
begins with RGB 16/94/140. The historical 400-by-100 RDP artifact still hashes to
`7f642adcc83d962dcf542faedfee0a7bd9027bd45aa1bcba2fe6842c1d6ac527`.
No external oracle, secret, installed runtime or live fixture was changed.

This is W4 blocker reduction. Before one changed external diagnostic, verify
the retained handle and native marker/chrome geometry, bind the producer source
and installed candidate explicitly, and retain the historical RDP oracle. The
test's 86-pixel chrome offset is a local fixture construction, not a live
readback. W3 takeover/reopen/concurrency and the W5 production proposal remain
open; the retained synthetic browser and server remain cleanup obligations.

## Checkpoint 32: external pixel mapping passes; resized sample guard corrected

Live original-handle readback measured a 1920-by-1080 browser with a 1920-by-993
content viewport, zero screen offsets and device scale one. The prepared native
marker is therefore x=240, y=207, width=960, height=320, with the historical
400-by-100 sample. Source `846ec620` was dispatched once in protected readiness
run [34019097339](https://github.com/CochranResearchGroup/agent-browser/actions/runs/34019097339).
Its binding separately identifies the unchanged installed `3428a7a7` binary
`35cb5a62b307f8ea3e2a463479ef5c248caeac62323455a922d04a5ef5ffd28b`.
The intervening changes were reviewed as producer/docs plus one CLI help line;
no runtime implementation, install or browser replacement occurred.

Both clients' initial crops now match the independent historical RDP hash
`7f642adcc83d962dcf542faedfee0a7bd9027bd45aa1bcba2fe6842c1d6ac527`.
The human passed controller takeover, then failed at native geometry validation
before input acknowledgment. The slow client failed in its second page's
geometry validation. Both jobs and the aggregate failed; retries remain zero.
All five human and six slow artifacts match their declared bytes and hashes.
Private `primary-guard-live/external-geometry/` retains the binding, dispatch,
receipts, artifacts, verification and production/development identity readbacks.
The same primary remains connectable with zero terminal owner records.

The first hypothesis was a transient undersized desktop. A fresh original-handle
readback instead proves a persistent 1108-by-633 browser after takeover, with
1108-by-546 content and the same marker rectangle and ready input state. The
full marker extends beyond the resized desktop, while its centered 400-by-100
sample remains visible. Requiring the entire marker to fit was an overly broad
producer guard. Waiting for the full marker would not fix this settled geometry.

The guard now requires the full sample inside the declared marker, actual
rendered desktop and iframe. It still refuses malformed coordinates and never
shrinks the oracle. A sample that cannot fit waits within the existing deadline.
The real-Chrome regression was red on the old guard. It now proves bounded
waiting through a too-small display, rejects negative coordinates, and passes
trusted click/Enter at both the scaled geometry and the settled 1108-by-633
geometry with a clipped marker edge. Focused input and runner checks and diff
validation pass. This is the corrected resized-sample seam, not permission to
repeat the prior full-marker rule or relax browser/primary identity guards.

Progress is external initial-pixel acceptance plus W4 blocker reduction. Full W3
is still unaccepted. Next use this changed guard for one source-bound external
verification with the same retained handle and historical oracle; keep the
failed runs counted. W5 production delivery remains pending, production hashes
are unchanged, and the synthetic browser/server remain retained obligations.
