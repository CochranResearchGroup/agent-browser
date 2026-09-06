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
