# Plan 0160: Production Profile Identity and Operational Readiness

Date: 2026-09-06

State: OPEN

Consolidation: required

Current execution status: [RUNBOOK.md](../../../RUNBOOK.md). Historical
execution checkpoints below are retained evidence, not current status.

Lane: P157

Branch: plan/profile-permissions-and-request-provenance

Target: main

Integration: merge

Source baseline: `51da03ee`

Dependencies: [P159]

Overlaps: [P157, P158]

## Objective and authority

Make the production candidate operational across all four outstanding readiness
items: reliable browser control, the authenticated operator journey, maintenance
and ownership reconciliation, and unattended operation. Profile ownership and
identity proof blockers are the highest priority. Causal error tracing is part
of repairing those blockers, starting with the first reproduction.

The operator requested this plan and subsequently instructed execution. The
primary now executes the work units under that authority. Existing authorization
for candidate installation and the reviewed 77 policy writes remains recorded
in the [delivery proposal](../notes/0154-2026-09-06-plan-0159-production-delivery-proposal.md).
During execution, apply that authority to its actual scope without requesting
it again. Do not infer authority for unrelated client eviction, broader grants,
private content capture, new external systems, or formal release.

Operator clarification: self-identification is the default for ordinary
permitted use under `shared-local`. Stale registered-capability or principal
binding warnings must not impose stronger credentials on those clients. Resolve
or classify each warning against that permission contract. Physical browser,
profile, session, process and target identity checks remain required; they are
distinct from proving the client's declared identity.

The operator also supplied [incident 0156](../notes/0156-2026-09-06-tab-release-skipped-close-and-wrong-tab-close.md).
It reports successful logical release despite skipped physical cleanup, followed
by a close operation that apparently removed a different tab. Treat this as an
A1/AX investigation case: establish the actual serialized selector and historical
build, then reproduce detach/release and exact-target close with three disposable
targets. The cause remains unverified. Do not replay the incident identifiers or
perform cleanup in its shared browser.

## Current state and reconciliation

Current state: OPEN; the runbook acceptance table distinguishes completed proof
from the remaining consolidated batch and blocked runtime execution.

### Consolidation amendment, 2026-09-08

This amendment replaces the previous active repair ordering and implements the
operator-approved consolidation and task-based delegation policies. The frozen
A1–A4/AX contract below is unchanged. RUNBOOK.md owns current results and the
single requirement-to-evidence table; historical checkpoints are not next steps.
This request authorizes policy/plan changes, not a restart or renewal of the
expired runtime observation window. All prior effort and publication counts remain.

### Consolidated batch

Deliver one accepted production configuration with usable retained ownership,
remote-view continuity and diagnosable failures. Group the known remaining work
around that outcome before another build or deployment:

- Reconcile the ownership/control matrix and all seven selected AX failure cases
  against existing receipts. Preserve accepted original-client self-declared
  recovery and local viewer/input evidence. Produce missing-proof lists rather
  than replaying already accepted tests. Historical wrong-tab request/build
  attribution remains a distinct unresolved investigation, not a current repro.
- Resolve delivery of the committed process-only supervisor default: installed
  override, binary template and future supported unit rewrite must have an
  explicit consistent disposition. Freeze the binary, support and unit settings
  accepted by the batch; an override alone does not mean source was published.
- Qualify scheduled cadence, preservation of browser roots and descendants,
  and original-handle/durable-link continuity through the guarded host restart.
  Include the completed fixture cleanup in readiness evidence; do not repeat it.
- Intake new consumer reports once against the frozen contract, including
  note0160's native-confirm timeout. Determine whether they contradict accepted
  A1/AX behavior before reopening that row or adding repairs to the batch.
  Do not run consumer actions merely to investigate a report.

Keep actual consumer CSV acceptance explicit and separate from synthetic file
transfer. Private payment/business operations, formal release, Plan0158 external
vantage, provider polish and unrelated application cleanup remain outside this
batch. Product recovery-surface gaps are evaluated against A1/A3, not silently
removed from scope. The full goal stays open when a dependency cannot be resolved.

### Delivery sequence and budget

1. Reconcile existing evidence and new report scope in parallel with primary
   candidate/configuration review. No browser launches are needed for receipt
   extraction. Exit with one missing-proof list and a consolidated repair set.
2. Decide once whether source changes are necessary. If so, qualify the combined
   repair batch in isolation, rehearse fixtures and cleanup, and identify all
   changed-surface gates before proposing a final build/publication. If not,
   preserve the installed candidate and record why its evidence remains valid.
3. Freeze the accepted configuration and checks. Require current doctor readiness,
   available original-client inputs, no active-work conflict, correct stop mode,
   and exact retained process/display census before the restart.
4. Observe three consecutive normal five-minute cycles, perform one guarded host
   restart, then observe the next cycle and verify original handles and durable
   link. Check monotonic interval and timer provenance; manual refreshes do not
   count. A failed precondition stops the dependent action, not unrelated work.
5. Join full A1–A4/AX evidence, integrate through the existing branch strategy,
   and report committed, installed, user-verified and incomplete outcomes.

This documentation slice is bounded to 15 minutes and includes one narrow worker,
focused audit updates and commit/push. It does not consume a new runtime allowance.
A future execution window must budget the entire remaining sequence: provisionally
15 minutes for parallel evidence reconciliation, 10 for configuration/preflight,
30 reserved for A4 and 10 for integration/closeout, plus 10 contingency (75 total).
This is an estimate awaiting an execution allowance, not a claim of approval or a
whole-plan finish guarantee. Compilation, publication, new source defects or
missing consumer evidence may exceed it; disclose that at the first join before
spending the reserved observation time. The earlier requested 30 minutes covers
A4 only. No successor, worker or model change resets cumulative bounds.

### Worker assignments

| Lane | Route and owner | Inputs / exact write scope | Output, dependency and stop |
| --- | --- | --- | --- |
| Evidence reconciliation | Luna low/medium worker; deterministic extraction first | Existing Plan0160 receipts and notes; private evidence table draft only | One row per A1/AX case with identity, applicability and missing proof; no live calls or source edits; one pass, 15 minutes. Primary adjudicates before repairs. |
| Fixture readiness | Independent Luna medium worker when a specific missing check is identified | Named existing fixture and its test only; explicit filenames assigned before launch | Rehearse setup/oracle/cleanup at cheapest layer; one bounded attempt, at most 10 minutes inside the evidence stage. No production effects or duplicate discovery. |
| Bounded implementation, conditional | Sol/Terra medium worker after primary freezes a concrete defect and files | Exact disjoint source/test paths in the accepted repair set | Patch and decisive regression under remaining allowance; return after one failed attempt for diagnosis, no self-expanded investigation. |
| Integration and operations | Primary, current strongest available model | Candidate disposition, ownership evidence, guarded runtime actions, runbook and final integration | Own critical path, conflicting-evidence decisions and completion; join worker evidence without repeating its investigation. |

Do not spawn all lanes by default. Start useful independent work when it can run
alongside primary work; keep dependent steps and trivial extraction local. Use
compact briefs, record actual handles, requested/effective model when available,
accepted outputs, wall time and rework. Model mapping is provisional; no measured
cost saving or automated budget enforcement is claimed.

### Evidence and exit

The RUNBOOK.md table is the sole current acceptance ledger. Each incomplete row
must identify missing evidence and next action; artifacts retain exact source,
binary/support/configuration identity and applicable scope. New defects reopen
only affected rows and required changed-surface gates. Neither doctor success,
worker completion nor an accepted local viewer test closes the full plan.

Policy adoption feedback: local overrides of the pinned selector v0.1.24
(source 9caf5708e5aa48cc0cf6264baa7065fc335d193e), retaining this repo's custom
composition. Modules 0010/0021/0028/0042/0043/0044/0045 are merged locally; no
profile replacement or bundled-library update. The installed audit helper has
a scoped local extension for consolidation declarations. Prior rules named consolidation
but did not require the batch decision; stale status and fixture cleanup caused
rework. The new audit checks declared plan structure, not truthful acceptance,
model quality or runtime stopping. This slice exercises bounded delegation and
policy wiring; operational effectiveness remains unverified. These changes are
reusable upstream feedback; no cross-repo publication is performed here.

### Historical strategy amendment, 2026-09-07

Production presentation recovery requires a supported production inventory
adapter qualified before provider-backed acceptance. Keep production and
development contracts explicit; do not relabel production as development or
remove the environment guard. Bind the production contract to the actual runtime
environment and provider ownership evidence.
Preserve incumbent route, display, browser, viewer and controller custody across
projection, including orphaned records. Available recovery capacity must not
assert that an operator presentation is already ready. Qualify the transition
from a disconnected retained browser through recovered browser control, stale
presentation records, guarded same-route reattach, and the original durable URL.
Deny foreign or ambiguous owners and environment mismatches before state effects.
Use existing production provider routes; this work does not authorize expansion
or borrowing development resources. Include namespace-aware display UID evidence
and original browser/process continuity in the acceptance receipt.

Production inventory failures must fence presentation effects without disabling
Service State reads or ordinary browser diagnostics. Preserve incumbent custody
and reservations, expose the validation cause, and require successful inventory
requalification before admitting new presentation work. Include provider-outage
and restored-inventory cases before enabling this adapter in production.

Qualification must also exercise a real pending handoff acquisition through
inventory admission, health reconciliation, capacity reload and activation.
Preserve exact in-flight custody without declaring the pending route ready;
missing displays and foreign, prior-boot, completed or mismatched acquisitions
must still refuse. Test this sequence against the built candidate in isolated
state before installation, then exercise the original durable URL.
Keep synthetic input markers visible within the actual remote viewport. If a
second viewer changes its dimensions, record that behavior separately from
pixel-marker and input delivery results; fixture adjustments do not establish
correct multi-viewer sizing or waive reconnect and input acceptance.

For stranded legacy connections, use preserved host-replacement custody and a
fresh census of connection-producing processes, scoped to the same Service State
root. Require an exact unchanged child-access record and affected-tab set under
the native repository transaction lock. A local operator maintenance path may
mark the proven dead transport disconnected while preserving subjects,
permissions, profiles, leases, browser processes and targets. Unknown producers,
state roots or changed custody must refuse the update. Validate the transition
in isolation before applying it; keep it outside ordinary consumer request
admission. Do not edit the primary JSON independently of its transaction store.

Operator follow-up: complete MCP request correlation and structured failure
propagation first, then use that evidence for unresolved consumer ownership
refusals. Owner/lease usability remains ahead of remote-view work. For the
unresolved Guacamole input observation, check refresh/reconnect, effective
settings, and scoped browser/provider service recovery before assuming a code
defect. Use disposable sessions and preserve ongoing consumer browsers.

Prioritize the remaining A1 ownership and identity failures before rollback
retention work. Use the current installed candidate and isolated consumer
simulations to distinguish a real action refusal from an owner-maintenance
warning. Preserve ordinary self-identified use under shared-local and exact
physical custody checks. Do not weaken recovery, transfer or cleanup authority
to clear a doctor finding.

The next bounded repair batch must identify the affected action, reproduce its
failure, repair its cause, and verify the original action plus foreign-owner
rejection. Diagnostics must identify missing proof and supported recourse, with
correlation sufficient to trace the failure. Keep earlier viewer acceptance as
historical evidence. Do not replay a failed current viewer journey without a
causal change or instrumentation that can explain its input path.

Allocate the next hour first to completing the prepared candidate, then to the
highest-impact remaining identity refusal: approximately 20 minutes for exact
candidate qualification and one installation decision, 30 minutes for one causal
identity investigation and repair, and ten minutes for focused verification and
checkpointing. Qualification must simulate host interruption and prove original
handles, complete identity attestation and writable private temporary storage
survive together before production replacement. Candidate publication retains
its existing execution deadline; this allocation does not silently extend it.
Use two independently authenticated clients sharing one profile for this
interruption test. Each must recover diagnostics and control with its original
handle and credential, retain its permissions, and refuse the other client's
handle before page effects. Two handles owned by one principal are insufficient.
Distinguish an action refusal from an incorrect maintenance diagnostic before
choosing a repair. Reassess at 30 minutes without A1 outcome progress and change
the unsuccessful approach instead of repeating unchanged verification. All prior
effort remains cumulative; this is a tactic change, not a reset of the full goal
or its acceptance requirements. Keep mouse delivery, supported rollback retention
and scheduled maintenance as subsequent work unless evidence makes one a direct
dependency of the identity repair.

The retained-runtime boundary also includes temporary filesystem custody. A
2026-09-07 disposable systemd reproduction showed that retiring a PrivateTmp
service unlinks its backing directory while a retained child remains alive.
A1/A3 acceptance must include usable temporary storage after host retirement,
not only unchanged browser PID and tab targets. Preserve private isolation with
storage whose lifetime covers retained browsers. Do not retire another legacy
PrivateTmp host over retained browsers until that dependency is preserved.
Existing deleted namespaces require a separate ownership-preserving recovery;
a new unit template cannot repair their mounted inode. Browser-native download
and artifact retrieval must pass before calling the consumer download resolved.

Browser-mode capture must preserve an existing browser context's download
policy and deliver only the authorized target's artifact. Passive observation on
a newly attached CDP connection does not satisfy this requirement: Chromium binds
download-event subscription to each BrowserHandler, while SetDownloadBehavior
both enables those events and mutates the addressed context policy. An observer that
set the policy cannot stand in for a reconnected Service consumer in acceptance.
Independent Page events may supply completion without policy mutation, but a
read-only join to Chrome's live History database is not yet a usable artifact
resolver. Do not bypass its locking or treat an inconsistent copied database as
proof of the completed file. Bound further artifact-path investigation separately.
Qualified retained-handle ownership repairs and accurate failure reporting may
advance through installed acceptance independently; keep consumer CSV acceptance
explicitly open instead of withholding those repairs behind the separate capture
design. This does not relax A1–A4/AX completion requirements.
Use the independently qualified subscription strategy: create a new, empty,
operation-owned browser context on the observation connection, enable Browser
download events with behavior deny addressed only to that context, and dispose
that exact context before clicking the authorized target. The prototype proves
that Browser events remain available on that connection after disposal and report
the completed path without changing the shared/default context's peer policy.
Never create tabs or copy authentication into the temporary context. Require
positive context ownership, disposal confirmation and unchanged pre-existing
context identities; creation, subscription or cleanup uncertainty must return
traceable failure before the download click. Connection detach must reclaim any
operation-owned context left by a failed setup. Qualify the real Service request
across retained host restart, with concurrent peer download, namespace-separated
source, exact primary bytes and zero owned residue. The existing prototype is
not product acceptance. Do not infer policy from directory contents, reset the
shared context, or bypass Chrome History locking. New owned-launch defaults may
be established before exposing a browser; retained peer settings remain intact.
Preserve process-bound namespace delivery, frame/GUID attribution and destination
checks as reusable components; qualify them through the real Service request.
When a restarted user-systemd namespace cannot open the retained browser's
process-root link, translation must prove an equivalent filesystem location from
both kernel mount tables, preserving device and filesystem-relative path.
Never assume the same absolute path identifies the same file across namespaces;
unknown or shadowed mappings must fail with causal identity evidence.
Include ordinary user download-directory aliases in qualification, including a
home Downloads symlink into a Windows mount. Prove the browser-visible alias and
resolved source identity; blanket rejection of directory aliases does not satisfy
usable artifact delivery. Preserve leaf-file substitution and overwrite defenses.
When an installed retained target produces no matching download completion,
distinguish an untriggered or policy-blocked download from missing event delivery
before changing code or repeating the request. Compare the exact retained target
with an isolated headed browser without changing its default-context policy.
Record actual click activation and first/repeated native download outcomes;
untrusted DOM activation alone is not sufficient evidence of the cause.
Do not replace the retained browser, borrow another client's capability, or reset
unknown context policy to obtain a passing artifact check.
Failed recipes must report failed response, job and event outcomes with the same
causal error, correlation and accurate effect state, even when transport succeeded.
Validate shared directories and retained PrivateTmp separately, including foreign
events, changed process identity and unsafe paths. The
[CDP download contract](https://chromedevtools.github.io/devtools-protocol/tot/Browser/#method-setDownloadBehavior)
scopes download settings to a browser context, not an individual target.

After the identity batch, repair supported rollback retention before maintenance
can resume. For a reviewed generation installed outside an accepted upgrade
transaction, add explicit hash-bound operator retention under the workstation
lock. It must protect the sealed payload from GC without synthesizing an accepted
transaction or healthy-generation claim. Require explicit release, retained audit
evidence, no-effect preview and refusal on changed payloads. Validate the real
GC join in isolation before using it for the production rollback generation.
Bound this repair to one source batch, one candidate build and one installation
attempt within 45 minutes; preserve prior milestone effort and acceptance gaps. Complete A1/A3 dispositions and doctor acceptance before A4's actual
scheduled cycles. A2 and AX retain their existing scope; the full A1–A4/AX
contract remains open until each requirement has current evidence.

[Plan 0159](0159-2026-09-05-client-recovery-logging-and-remote-view-remediation.md)
closed a bounded repair/delivery contract; it did not establish whole production
readiness. Its [acceptance audit](../notes/0155-2026-09-06-plan-0159-acceptance-audit.md)
and delivery proposal preserve that distinction. This plan cannot close merely
because source tests, an older candidate, or a subset of user journeys pass.

Last recorded production generation is
`0.28.0-e4243235af0c-072303ae3e67`, containing profile repair `8bd7f58a`.
The exact release passed isolated original-two-client retained-handle recovery
and installed custom-profile headless control, interaction, close and reopen.
The ambient remote-headed reopen failed when Xvfb could not start display `:91`.
Headless success does not clear that failure. Refresh live identity at execution
start; these are source-backed historical checkpoints, not a fresh live census.

The last doctor report includes 11 owner/generation/binding mismatches, five
missing principal bindings, one unproven session authority, unknown resource
ownership, monitor readiness, operator journey and selected-generation acceptance
findings. A stopped synthetic supervisor finding may already have been removed;
verify instead of carrying it forward blindly. The reconciliation timer was
enabled but inactive. Neither historical lease rows nor failed upgrade receipts
may be erased to manufacture readiness.

## Frozen acceptance contract

| Criterion | Required outcome on the final installed candidate |
| --- | --- |
| A1: reliable profile control, highest priority | Every current profile ownership/identity blocker has an evidence-backed disposition. Authorized named-profile, custom-directory and session-only control passes launch, navigation, interaction, retained reuse, close and reopen in both supported headless and remote-headed production paths. Two independently authenticated clients retain their original authorized handles through an exact disposable host interruption; legitimate foreign, ambiguous or changed identities are denied before effects with useful recourse. No unintended browser, tab, profile or grant replacement masks a pass. |
| A2: authenticated operator journey | The ordinary authenticated dashboard selects the intended synthetic browser. Its durable remote-view URL reaches `operatorVisible.state=ready`, shows verified synthetic pixels, accepts mouse and keyboard input, and reconnects using the same URL with two concurrent authorized viewers. Browser, profile, target and route ownership stay attributable across transitions; unauthorized viewers are denied. |
| A3: maintenance and readiness | Every refreshed doctor finding and ownership-related pressure finding is repaired or explicitly classified with current evidence, consequence and disposition. No unresolved actionable production blocker remains. Supported doctor returns zero, and selected generation, payload, ingress, convergence, operator journey, monitor and rollback readiness reflect real observed state. Historical failures remain inspectable without falsely blocking current readiness. |
| A4: unattended operation | After A1–A3 pass, restore the intended timer and observe three consecutive normal scheduled cycles plus one controlled runtime restart and its next scheduled cycle. All complete without new ownership/identity errors, route loss, duplicated owners, unsafe cleanup or accumulating owned residue. Original authorized handles and durable URL remain usable after restart. |
| AX: diagnosis across all four | Each selected failure is traceable from its returned correlation identifier to the first causal decision, relevant ownership evidence, source/build identity, affected operation and safe repair action. Expected projections have zero unexplained missing, duplicate or conflicting records. An operator can reconstruct the cause using the documented diagnosis surface without searching unrelated logs or accessing secrets. |

A genuine foreign resource may remain present when it is positively classified,
excluded from service cleanup and does not block an authorized user workflow.
Unexplained identity gaps, blanket historical exemptions and false-positive
errors in ordinary authorized use do not satisfy A1 or A3.

## Work units and dependencies

The primary owns integration, production mutation and acceptance. One economical
worker may run a bounded independent evidence or disposable-fixture lane under
policies 0021/0045 while the primary advances separate work. Shared production
mutation remains serial; primary reviews decisive evidence before accepting it.

| Unit | Work and expected write surface | Exit evidence | Dependency |
| --- | --- | --- | --- |
| W0 | Refresh exact installed identity, doctor, leases, principal bindings, processes, namespaces, ingress, provider and timer state. Create a private blocker ledger and deployment/evidence manifest. | Every finding has a stable key, affected workflow, provenance and A1–A4 mapping; final-candidate mismatch is explicit. | Planning complete |
| W1 | Repair profile ownership and identity proof at the actual selection, attachment, relaunch or reconciliation seam. Add causal logging at the failing seam immediately. Likely surfaces: native action runtime, Service ownership/lease model, request diagnostics and focused regressions. | A1 and its AX cases pass; custom profile repair stays covered; evidence binds principal, process and target before effects. | W0 |
| W2 | Fix remote-headed launch/Xvfb ownership and ordinary durable remote-view failures. Update provider/launch and handoff surfaces only where the reproducer establishes the cause. | A2 passes on the installed candidate, including remote-headed close/reopen; synthetic external evidence and exact cleanup accounted for. | W1; Xvfb ownership diagnosis begins in W0/W1 if it blocks A1 |
| W3 | Reconcile remaining maintenance records, monitor readiness and selected-generation/upgrade acceptance through supported product paths. Fix false-positive classification and broken transition software instead of hand-editing green receipts. | A3 passes; every ledger item is resolved or justified; retained failure history remains readable. | W1; A2 for operator acceptance |
| W4 | Restore timer, observe scheduled cycles, perform controlled restart and verify client/operator continuity. | A4 and associated AX cases pass with before/after census and bounded resource comparison. | A1–A3 |
| W5 | Join all evidence against final binary and support identities, perform closed-world acceptance audit, update plan and delivery status. | A1–A4 and AX all pass on one accepted final candidate; no unowned cleanup obligations hidden. | W1–W4 |

W1 takes precedence over unrelated maintenance or presentation polish. Ownership
findings from W3 that affect profile usability are pulled into W1 immediately.
Read-only evidence collection for later units may proceed without delaying W1.

## Strategy amendment, 2026-09-07

Preserve A1, A2, A3, A4 and AX in full. Execute consumer recovery before further
presentation, maintenance, or unattended work. The next diagnosis milestone is an isolated simulation of the blocked recovery
transition with a test-owned credential, profile, browser, and original handles.
Do not interrupt an active consumer merely to diagnose a reproducible mechanism.
Original-consumer recovery remains final acceptance evidence; it is not a
prerequisite for isolated investigation. Simulation must state which identity
and custody conditions it reproduces and which remain unproven.

1. Read current installed recovery and ownership evidence, then reproduce the
   relevant state in a disposable isolated runtime using the exact candidate.
   Exercise the supported rejoin path with the original test-owned capability.
   Preserve process, target, and handle identity; verify cleanup. Never borrow
   production capabilities or mutate a consumer browser for simulation.
2. If blocked, reproduce the exact failed transition and join requested profile,
   physical profile, lease, live owner, and principal evidence into one causal
   explanation before changing code. Ordinary shared-local self-identification
   remains the default; physical resource ownership checks remain mandatory.
3. Repair one coherent batch only when that explanation supports the repair.
   Include causal diagnostics at the failed decision. Use focused checks during
   repair and required integration gates once before governed publication.
4. After consumer recovery, resume cold operator access, maintenance, and
   unattended acceptance in the existing dependency order.

The operator authorized the proposed additional 30-minute round to establish
consumer recovery or a reproduced causal defect, then explicitly directed using
simulation to avoid interrupting ongoing consumer work. Count diagnosis, validation,
waiting, and documentation within it. RUNBOOK.md records its exact start and
end. Retain the approximately 12 hours already reported; no successor resets
that effort. An inconclusive round does not automatically trigger a rebuild,
new live attempt, or further allowance. A reproduced defect is a diagnosis
milestone, not completion of A1 or the whole plan.

## W1 ownership proof and failure matrix

For each blocker, distinguish requested identity, inherited/default selection,
stored authority and live observation. Record canonical profile identity,
logical browser/session and target, authenticated principal and grant reference,
lease/fence, owner generation, executable identity, PID plus process start/boot
identity, CDP endpoint and relevant namespace. A PID or directory match alone
is insufficient. Verify real authorization; do not weaken checks to reduce
error counts or create replacement profiles as recovery.

Cover positive named and custom profiles, session-only reuse, two authorized
clients, clean terminal close/reopen and retained host recovery. Exercise
diagnostics, UI actions, network capture and file transfer directly
after another authorized client's retained target was selected. Verify each
operation reaches its own target and foreign requests leave both tabs unchanged.
Identity counterexamples must cover conflicting explicit profile, foreign
principal, missing binding, ambiguous owner, changed generation, PID reuse and
stale endpoint/target.
Use focused deterministic tests for identity counterexamples and disposable
installed fixtures for process/transport claims. Each negative case must prove
no browser effects and identify the safe next action.

For the Xvfb failure, inspect the runtime host's actual mount/process namespace,
display allocation record, child exit status and bounded stderr before deciding
whether a lock is stale. Caller-visible `/tmp` is insufficient under PrivateTmp.
Reclaim only positively owned released resources; never unlink an unknown lock
or terminate an unrelated browser/display to make a probe pass.

## Causal logging and backtrace contract

A failure envelope and its durable diagnostic record must provide:

- Stable error code, first cause, causal parent/span identifiers, operation,
  phase/axis, UTC time and ordered attempt sequence; wrappers and retries retain
  the original cause and distinguish their own failure from it.
- Request, trace, job and event correlation; session/profile/logical browser,
  owner generation, lease/fence and principal-binding references when applicable.
  Include requested versus inherited profile-selection provenance and the exact
  proof predicate that failed, with expected and observed safe summaries.
- Exact binary/source and support generation; process start/boot identity and
  endpoint/target references where relevant. Mark unavailable evidence explicitly
  rather than inventing values or omitting the reason it is unavailable.
- Effect certainty, retry safety, cleanup obligations and concrete recourse.
  Link the decision to its source component and relevant regression/reproducer.
- For unexpected internal failures, bounded symbolizable stack backtraces or
  source locations plus the error cause chain. Expected authorization denials
  need the decision trace; a noisy stack alone is not an explanation.

Expose a documented lookup from the returned correlation ID through existing
CLI/doctor/diagnostic surfaces where practical. Add a product lookup only if
existing surfaces cannot reconstruct the chain. Keep logs structured, bounded,
redacted and durably correlated across response, job, event, trace and journal;
record which projections are applicable and why. Record rotation/retention and
lookup behavior so a restart does not sever the diagnostic trail. Logging
failure must not turn an error into success or lose the first cause silently.

Selected AX fixtures: profile-selection conflict, missing principal binding,
owner-generation mismatch, stale transport/target, Xvfb startup failure,
journal append interruption, and failed maintenance/restart transition. Preserve
red evidence, then show the repaired case or safe typed denial. For each, produce
one end-to-end causal reconstruction from the returned ID, including restart
lookup for the journal case, with explicit expected/observed/missing/duplicate/
conflicting counts. Historical P159 logging proofs may be reused as scoped
regression evidence but do not replace new-seam or final-installed proof.

Never log credentials, raw capabilities, cookies, authenticated page content or
unredacted private payloads. Keep access-controlled raw diagnostics and identity
manifests outside the product repository; tracked summaries contain only safe
conclusions, hashes and source references.

## Installation, operational checks and evidence

Validate changes first in the isolated development runtime. Use the optimized
candidate build and focused smoke there; use the full release build for the
production candidate. Preserve unrelated production browsers and the retained
P158 development fixture. The known old upgrade prerequisite failure must be
reproduced and corrected through a supported path before that transition is
called operational. A successful controlled publication does not itself prove
upgrade recovery or clear selected-generation acceptance.

Before publication, record exact binary/support hashes, compatibility, owned
resources, rollback target and preserved clients. After publication, read back
all three production unit executables, current selector, ingress, provider
assets and doctor. Do not fabricate acceptance receipts. If the candidate
changes during validation, invalidate affected checks and repeat the dependent
acceptance slices against the new identity before the final join.

Production user-journey fixtures use synthetic content and dedicated identities.
External visual evidence uses the protected manually dispatched
`p158-external-vantage.yml` lane with its required synthetic-only bindings,
credentials and exact retained identity. No automatic trigger or unchanged
retry. Historical run 34022233372 is baseline evidence, not proof of this
production candidate. Keep preparation and credentials out of tracked artifacts.

W4 records the timer's intended schedule before enabling it, then observes three
actual scheduled cycles at that cadence, not three manually forced invocations.
Bound observation to five scheduled opportunities and an explicit deadline
calculated from the refreshed schedule. Restart only the exact owned runtime
unit after recording rollback and protected client identities; then observe the
next scheduled cycle and recheck original handles, remote-view and doctor.
A new ownership error ends unattended acceptance and returns its causal evidence
to W1 or W3. Preserve timer state and the reason for any safety stop.

## Validation, bounds and completion

For changed Rust surfaces, run focused regressions during the repair batch;
run repository-safe format and workspace clippy at the completed batch boundary
before integration or governed runtime effects, following policy 0042. Run Service/client contract parity checks when
models, schemas or diagnostics change; update CLI help, README, repository skill,
docs site and inline contracts for changed user-facing behavior. Use
`pnpm validation:select` against the slice baseline. Documentation-only planning
requires link, whitespace and acceptance/dependency review; it does not require
browser launches or Rust builds.

max_work_unit_attempts: 3
max_review_rework_cycles: 1
max_hardening_checkpoints: 2
checkpoint_interval: 30 minutes
max_review_discovery_passes: 1
review_verification_mode: closed_world_if_reviewed
authorization_gate: material_departure_or_explicit_action_gate_only
continuation_default: execute_obvious_in_scope_low_risk
bound_exhaustion_mode: local_replan_before_escalation
checkpoint_mode: material_boundary_with_cadence_backstop

The primary controls each reproduce/repair/retest loop. Three materially distinct
attempts per unit is the hard local bound; do not repeat unchanged failed live
operations. The external lane retains its stricter no-retry rule. At a bound,
record the causal result and end that unsuccessful approach. Any different
approach must have evidence of acceptance progress within the remaining overall
effort ceiling, following policy 0028. Successors do not reset cumulative effort,
retry counts, or the default two-checkpoint/30-minute no-outcome-progress bound.
Set a finite overall ceiling in the existing runbook control record before
further sustained execution; retain prior effort rather than granting a fresh
allowance through this policy update. A bound does not authorize broader
mutation or erase a blocked criterion.
One final acceptance review checks the frozen criteria; one rework cycle may
address accepted blocking findings. Other concerns go to an explicit backlog.

Checkpoints record `state_transition`, `acceptance_state`,
`progress_classification`, `evidence`, `material_blockers`, and
`next_action_or_stop_reason`. Unit states are ready, active, awaiting-review,
awaiting-gate, blocked, complete, failed or cancelled. Classify progress as
outcome_progress, blocker_reduction, hardening, no_progress or regression.

Close only when all five acceptance rows pass against the final installed
candidate and private evidence manifests account for owned-process cleanup and
retained obligations. Publish a concise criterion-by-criterion verdict with
current doctor and scheduled-cycle results. If anything remains blocked, retain
OPEN execution status and name it; do not call production fully operational.

## Planning checkpoint

At the planning checkpoint all execution units were unstarted. This plan covers the four requested readiness
items and makes profile ownership proof and causal diagnosis the critical path.
The next execution action is W0's current census followed immediately by W1's
highest-impact reproducible ownership blocker.

## Execution checkpoint 1

state_transition: ready → active

acceptance_state: A1–A4 and AX incomplete

progress_classification: blocker_reduction

evidence: Fresh private doctor, Service status, lease census and blocker ledger
under the user-scoped `campaigns/p160/census` directory. The selected production
unit still runs generation `0.28.0-e4243235af0c-072303ae3e67`. The interlock timer
is enabled and inactive with a five-minute schedule. The stopped synthetic
supervisor finding is absent from the fresh doctor.

material_blockers: Seventeen lease findings remain. Thirteen affected lease
rows have matching terminal lifecycle records with satisfied cleanup, while the
projection still treats retained owner evidence as current. Two unbound
capabilities and one live owner with two findings remain separate investigations.
No classification has been changed in production.

next_action_or_stop_reason: Run the deterministic terminal-owner projection
regression, then repair the exact historical/current distinction with negative
controls for active work and mismatched generation. Continue live-owner diagnosis
and causal diagnostics; terminal history alone cannot satisfy A1.

## Execution checkpoint 2: occupied profile without reusable browser

state_transition: active → active

acceptance_state: A1–A4 and AX incomplete

progress_classification: blocker_reduction

evidence: The operator supplied an Odollo blocked-submission example. A read-only
production access plan reproduced active lease conflict with zero compatible
browsers. Persisted health says process identity is ambiguous while the Service
status projection says ready. Disposable user-systemd probes isolated executable
observation permission denial across PrivateTmp user namespaces: the isolated
probe cannot read the retained browser executable; the namespace-neutral probe
with NoNewPrivileges still enabled reads the exact recorded executable, boot
and start identity. Private diagnosis and both probe receipts are in the census
evidence manifest. No live profile, lease, browser or unit was changed.

material_blockers: Process observation discards the underlying executable-read
error; health then discards the assessment reason, and reuse reports only no
compatible browser. These causal losses obstruct diagnosis. The terminal-lease
regression separately went red at the real projection seam, reporting identity
reconciliation instead of history despite exact terminal cleanup and no work.

next_action_or_stop_reason: Prioritize the live occupied-profile case. Repair
trusted process observation across namespaces without weakening runtime isolation
or trusting PID alone. Carry typed observation failure and causal provenance
through health and acquisition diagnostics. Keep terminal-history regression
and its active-work/generation counterexamples in the same W1 repair queue.
The internal submission itself and unrelated Ads operations are outside this
Agent Browser repair; do not submit or mutate those workflows as a browser test.

### W1 bounded implementation decision: namespace-neutral observation

Keep runtime PrivateTmp isolation. On an executable-read permission denial only,
use the same binary through a bounded read-only user-systemd helper outside the
private namespace. The helper accepts one PID, reads only process identity,
never loads runtime configuration or issues browser effects, and cannot recurse.
Bind its answer to locally observed process start identity before and after the
call. Do not cache effect authority. Helper failure retains the typed reason and
leaves ownership unproven. Add adversarial identity-join tests and a disposable
namespace integration probe before publication. This is an internal observation
transport, not a new browser acquisition policy or a privileged helper.

## Execution checkpoint 3: namespace observation repair qualified in development

state_transition: active → active

acceptance_state: A1–A4 and AX incomplete; production publication pending

progress_classification: outcome_progress

evidence: Optimized candidate SHA256
`4a9a4a41563747b2254967ffa5ee961448246686c5e4e4425a04c5f9502264a7`
was installed in the separate p160 development namespace. Publisher readback
proved production and default development unchanged. The disposable namespace
smoke distinguishes the baseline's false browser absence from the candidate's
exact match and rejects wrong-start and wrong-executable cases; its owned process
was stopped with zero remaining MainPID. A read-only comparison also recognized
the original retained production browser without changing its lease or process.
The installed candidate passed disposable custom-profile continuity.

Focused validation: 30 process-identity tests, 23 profile-lease tests, 44
access-plan tests, one occupied-profile causal diagnostic regression, and 86
Service health tests passed. Workspace clippy with warnings denied, format
check, Service client suite, API/MCP parity, docs build and handoff docs checks
passed. The terminal-history regression was red before the repair. The new
namespace test uses a synthetic executable and does not claim browser or CDP
authorization coverage. Private receipts remain under campaigns/p160.

material_blockers: The standard development browser-launch smoke failed before
launch because a provider-optional namespace exports an unstaged presentation
inventory path. Preserve that fixture/bootstrap failure; the separate disposable
custom-profile pass does not clear it. Production still runs the baseline. The
live principal binding, unbound capabilities, Xvfb allocation and remaining
acceptance axes are not cleared by these source/isolated results.

next_action_or_stop_reason: Correct optional development bootstrap, finish
installed profile proof, then build and publish the exact production candidate
and verify the original occupied-profile workflow and all remaining W1 cases.
Continue W2–W5; do not call the runtime operational from this checkpoint.

## Execution checkpoint 4: optional development bootstrap repaired

state_transition: active → active

acceptance_state: A1–A4 and AX incomplete

progress_classification: blocker_reduction

evidence: The optional-provider installer regression reproduced missing inventory
and passed after initialization of an empty inventory only for an unconfigured
optional provider with no retained routes. Existing inventories and staged
providers are preserved. Reinstallation in p160 preserved production/default
development identity, and the standard three-iteration development browser
launch/read/close/residue smoke passed. Development doctor remains nonzero only
for the new namespace's local-hostname ingress; the direct dashboard port is
reachable. This is not production operator-journey acceptance.

material_blockers: Production still runs the baseline binary. Source repair
`81d2936e` is pushed; the full release build is active. The optional-bootstrap
follow-up is being validated. The remaining profile/principal and complete
production acceptance matrix stays open.

next_action_or_stop_reason: Finish the active release build and its exact-binary
qualification, publish within existing authority with preserved browser owners,
and verify original profile reuse. Do not restart the build merely because an
observation window ends. Retain the original failed development smoke receipt.

## Execution checkpoint 5: production observation repair installed, next identity blocker reproduced

state_transition: active → active

acceptance_state: A1–A4 and AX incomplete

progress_classification: blocker_reduction

evidence: Source `6d741162` was built with the full release profile. Exact binary
SHA256 `0904e93f590bb05dd8434a137dfbabe025557821e8d83adecbd1aed958843bc0`
passed the namespace identity probe, source-free installer fixture and disposable
two-authenticated-client retained-handle recovery after host interruption.
Production generation `0.28.0-0904e93f590b-df8df00a157c` was selected under existing
installation authority. Host and both dashboard executables matched that hash;
all four browser processes present at activation retained their exact identities.
The previous immutable generation remains available for rollback. Private
activation, qualification and request receipts are under campaigns/p160.

After supported Service reconciliation, the original occupied profile reported
its original browser ready and reusable. An authenticated dashboard API test
then requested only an owned blank tab in that exact browser. It failed before
effects with `existing_session_profile_identity_inconsistent`; no owned tab
handle was returned. The retained browser/session profile association agrees,
but its current owner generation is 2 while the registered principal binding
still names generation 1. This is the next concrete admission blocker, not a
successful end-to-end reuse result. A newer automatically selected profile is
not accepted as proof that the original profile works.

material_blockers: Access-plan reuse and execution admission still disagree.
The failure has a job identifier and provenance but reports an unknown subject
inside failure recourse while terminal provenance contains the declared subject;
its advertised recovery-plan request omits the required profile and capability
inputs. Preserve these actionable diagnostic gaps for AX. Doctor remains
nonzero, remote-headed reopen and the operator journey remain unproved, and the
reconciliation timer remains inactive pending A1–A3. The supported upgrade dry
run remains unready; controlled publication does not establish A3 convergence.

next_action_or_stop_reason: Trace owner-generation advancement and principal
binding retirement/continuity, add the exact stale-binding regression with
changed-owner denial counterexamples, and repair the authority transition without
manually upgrading a stale principal grant. Align acquisition and failure
recourse with actual admission. Retain the no-effect failed request as the red
production case; repeat only after a material repair. Continue W1 then W2–W5.

### W1 admission decision: separate current profile policy from stale capability authority

The observed supersession and terminal-replacement transitions replace the
runtime owner but retain its prior principal binding. That binding is correctly
unproven for the new generation. The defect is treating it as a veto over a new
shared-local subject, after proving the current owner, reciprocal browser/session
membership and canonical profile digest. The existing profile policy already
admits that subject independently. Preserve the old binding and guarded rejoin
for registered capability users; do not promote or delete it. Allow independent
shared-local profile selection only for an older generation, matching profile,
stable permitted subject and non-capability request. Current policy restriction,
future bindings and inconsistent identities continue to deny admission.

The existing exact-owner selection regression was extended with this production
shape and failed with `existing_session_profile_identity_inconsistent` before
the repair. Its first repaired run and the three adjacent identity-conflict tests
passed. Additional route and lint validation is in progress; this source result
has not yet repaired the installed production request. The policy predicate is
shared with the existing shared-local continuity path, without moving ownership
or effect authorization into that predicate.

W1 follow-up validation: all 92 route-host tests passed serially, including the
added future-generation denial. Workspace clippy with warnings denied, format
check, documentation build and remote-view handoff documentation checks passed.
The optimized development candidate build is active. Its next proof uses a
fully disposable shared-local fixture with exact retained browser processes and
an explicitly constructed older capability binding, then exercises acquisition,
blank-tab control and handle release through the Service API. No production
state or capability is copied into that fixture.

## Execution checkpoint 6: shared-local admission repair qualified

state_transition: active → active

acceptance_state: A1–A4 and AX incomplete

progress_classification: blocker_reduction

evidence: Repair `c1acd4c7` is committed and pushed. Optimized binary SHA256
`4bfecff9b205401a7525dde3d662e93836b610ae8f1e92711650d8f662e8588a`
passed the disposable full Service API reproduction with two independently
permitted shared-local clients. Both reused their respective original browser
processes, controlled only owned blank tabs and released their handles. Exact
older capability bindings remained unchanged. Final fixture state contained zero
browsers and zero nonterminal tabs; the subsequent scoped process census found
no residue. The same candidate was published in the separate p160 development
namespace and passed the standard three-iteration browser-launch smoke.

Validation retains the first two harness failures: unsupported CLI flag placement
prevented initial registration, and a JSON-only fixture edit was superseded by
the authoritative owner registry. The third bounded fixture attempt corrected
both setup defects and checked retained binding equality against the resulting
state. Its private evidence is campaigns/p160/shared-local-repair-tcqAHT; earlier
attempts remain diagnostic evidence. The synthetic state edit affected only a
disposable home with its host stopped. Constructed bindings were removed only
during fixture teardown after the acceptance assertions, before disposing its
browsers. No production registry or capability was edited.

material_blockers: The installed production binary still predates this second
repair. Its original blank-tab request remains a failed no-effect receipt.
The new full release build is active and must finish before exact-binary
qualification and controlled publication. Wider profile/principal projection,
remote-headed reopen, operator journey, upgrade readiness, diagnostic recourse
and unattended-cycle criteria remain open.

next_action_or_stop_reason: Resume the existing full release build, recorded in
campaigns/p160/census/stale-binding-release-build.log. Do not start another build
because a polling window ends. Qualify that exact binary, publish while preserving
current browser identities, then repeat the original profile's blank-tab test
once against the changed production binary. Continue the remaining W1–W5 work.

### W2 read-only diagnosis during the W1 release build

The current Linux socket census confirms the earlier remote-headed `:91`
failure has an independent allocation cause. An abstract X11 socket for that
display is live in the network namespace while its filesystem socket is absent
from the caller's `/tmp`; the visible lock names a PID that no longer exists.
The allocator in `cdp/chrome.rs` classifies only filesystem sockets and lock PID
command lines. It can therefore treat this occupied display as a stale lock and
select it. The private abstract-socket census is under campaigns/p160/census.
No lock was removed and no X server or browser was changed by this diagnosis.
Next W2 repair must account for abstract socket occupancy, preserve unknown
owners and retain useful Xvfb startup diagnostics. This evidence does not close
remote-headed launch/reopen or operator presentation acceptance.

## Execution checkpoint 7: original production profile reuse passes

state_transition: active → active

acceptance_state: Original occupied-profile blank-tab reproduction passes; A1–A4
and AX remain incomplete

progress_classification: outcome_progress

evidence: Full release build completed in 11m 41s. Binary SHA256
`c1b4ca7fe648d6bc56b3a82c892fe14554bdf80502c944f71a724342d3e56667`
passed the shared-local retained-browser fixture, two authenticated clients'
original-handle recovery after disposable host interruption, and source-free
installer fixture. Production generation `0.28.0-c1b4ca7fe648-711195022d0a` was
installed with all four browser process identities preserved. Host and both
dashboard executables independently matched that binary hash.

The original profile's authenticated dashboard API request reused its exact
retained browser, created only an owned blank tab, evaluated the expected
synthetic result and released the handle successfully. This follows the prior
no-effect `existing_session_profile_identity_inconsistent` failure on the old
binary. The older registered capability binding was not refreshed or deleted.
Receipts and reproducible probe source are in campaigns/p160/publication-shared-local.
No internal submission or other client business operation was performed.

material_blockers: Doctor remains nonzero for the operator journey, upgrade
readiness, monitor and resource-ownership findings; principal-binding warnings
also remain for separate disposition. The timer remains inactive. The release
fixtures required explicit exact-identity disposal of 27 still-present synthetic
processes after their API checks, with no signaled processes remaining. That
teardown is recorded separately and does not establish automatic residue-free
cleanup acceptance. Full remote-headed reopen and A2–A4 remain unproved.

next_action_or_stop_reason: Repair abstract X11 socket occupancy handling with a
red/green allocation regression and useful bounded Xvfb startup diagnostics.
Validate remote-headed launch/reopen, then complete the current profile/doctor
finding dispositions, causal-tracing contract and remaining W2–W5 acceptance.

### W2 implementation and focused validation

The allocator now observes abstract X11 sockets in the current Linux network
namespace as well as filesystem sockets. It refuses allocation and stale-lock
cleanup when an abstract socket is live; an unreadable socket census is unknown.
The real abstract-socket regression reported `Free` before the repair. After
correcting the fixture's directory setup, all 84 Chrome/display tests passed,
including lock preservation while the abstract listener remains live and reuse
only after it closes. Test setup/compile failures remain in the private logs.

Xvfb writes stderr to a private file so retained display ownership does not depend
on a daemon logging pipe. Startup exit/timeout errors include a bounded first
4096-byte sample and the log locator; inability to create the log does not itself
prevent launch. A focused diagnostic regression proves first-cause retention and
bounded inline output. The existing Xvfb tests now isolate their log HOME; their
final focused run passed. Workspace clippy, formatting, documentation build and
handoff documentation checks passed. The optimized candidate build completed;
installed development remote-headed launch/control/close/reopen is next.

## Execution checkpoint 8: remote-headed development continuity passes

state_transition: active → active

acceptance_state: A1–A4 and AX incomplete; original production reuse remains
qualified by checkpoint 7

progress_classification: blocker_reduction

evidence: Repair `50540506` was committed and pushed. Installed development
binary SHA256 `b750b542c5e5710d604b48ddc2e9b7ab4ea3e4c689697386d7ee2d5336150ce8`
passed explicit custom-directory remote-headed launch, URL read, synthetic click
and result read, close, reopen, second URL read and final close. Browser State
confirmed `remote_headed`, `private_virtual_display` and display `:90` for both
launches, with a new browser PID after the exact close. The final state had zero
browsers and zero nonterminal tabs; a subsequent process census by fixture HOME,
arguments and open-file ownership found no residue. Private durable receipts,
state snapshots and probe source are in campaigns/p160/remote-headed-development-proof.
The two Xvfb startup logs were created in the disposable HOME.

material_blockers: This direct headed-control proof has no selected presentation
route and does not establish A2 pixels, input, durable URL or multi-viewer
acceptance. Production still runs the checkpoint 7 binary without the display
repair. The display-fix full release build is now active. All remaining profile,
doctor, diagnostics and timer criteria stay open.

next_action_or_stop_reason: Resume the existing build recorded at
campaigns/p160/census/x-display-release-build.log, qualify the exact release and
publish within existing authority while preserving current browser owners.
Repeat the required final-candidate production matrix, then complete W2–W5.

## Execution checkpoint 9: display repair published; restart reuse fails

state_transition: active → active

acceptance_state: A1–A4 and AX incomplete. Checkpoint 7's single production
reuse pass does not prove continuity after a subsequent host replacement.

progress_classification: regression

evidence: Source `50540506` produced release binary SHA256
`6ba6ba3a9d4dc9929bfde5a5e1191781b6e3ffa1027c97c7f0e510b9c6c8a3d3`.
Exact-release disposable remote-headed control/close/reopen, shared-local reuse,
two authenticated retained-handle clients after host interruption, foreign-handle
denial and source-free installer fixtures passed. The retained-handle fixture's
23 remaining synthetic processes were disposed by exact process identity;
recovered handle connections deliberately use detach semantics, so this is
separate fixture teardown, not proof of automatic browser destruction.

The release is installed as generation `0.28.0-6ba6ba3a9d4d-9807bdc21eb8`.
All three production executables matched its SHA256, and all four original
browser process identities survived activation. The prior generation remains
available for rollback. Evidence is private under campaigns/p160/publication-x-display;
qualification receipts are under remote-headed-release-proof,
shared-local-release-J0Wvn6 and retained-repair-06K3TH.

The original-profile synthetic blank-tab request failed after this activation:
access planning selected the existing browser, but execution attempted another
launch against its occupied profile. The original browser remained alive. Its
record now had host `attached_existing` and null display name/allocation.
Source tracing found that shared-profile attachment persists that host together
with replacement launch metadata; the next remote-headed selection rejects it.
This newly exposed continuity defect is an A1 blocker. The failed request and
its diagnostics are preserved in publication-x-display/original-reuse-open.json.
No business submission was attempted and no production metadata was manually
repaired to force acceptance.

material_blockers: The returned failure still lacks a useful first predicate
and causal identity comparison. A read-only audit of the earlier failure found
one job and one journal record; its event had aged out of the 100-event window,
so the missing event is explained by retention, not proven delivery loss.
The journal's terminal details alone are insufficient for AX. Doctor, external
presentation, monitor and timer acceptance remain open; the timer is inactive.

next_action_or_stop_reason: Reproduce remote-headed metadata loss in an isolated
runtime, preserve verified launch identity during reuse, and test a second host
replacement. Diagnose failures through the existing durable journal and trusted
request provenance. Continue remaining A1–A4 and AX without weakening identity
checks or treating detached client connections as new lifecycle authority.

### W1 restart-continuity repair qualification

The isolated remote-headed shared-local fixture reproduced launch-record loss
with the installed release: the first reuse succeeded, then its assertion found
`attached_existing` instead of `remote_headed`. Its private evidence is in
campaigns/p160/shared-headed-restart-XcM0ac. Exact fixture cleanup found no residue.

The repair treats an attachment to the same current-boot process, endpoint and
profile as a health observation. It preserves launch posture, display allocation,
launch/session metadata and recorded profile path rather than replacing them with
incomplete CDP manager metadata. This changes projection persistence, not profile
admission or lifecycle authority. A changed process start identity receives fresh
metadata. The focused persistence regression and all 87 health tests passed;
workspace clippy, formatting, docs build and handoff documentation checks passed.
An optimized candidate build is active in census/retained-launch-candidate-build.log;
the prepared isolated probe covers two clients across two host replacements.
Already-damaged production metadata still requires verified recovery. The separate
display allocation remains present, but its existence alone is not live proof.

## Execution checkpoint 10: repeated headed reuse preserves launch identity

state_transition: active → active

acceptance_state: A1–A4 and AX incomplete; recurrence prevention is qualified
on an isolated optimized candidate, not yet installed in production.

progress_classification: blocker_reduction

evidence: Repair `29cb8739` is committed and pushed. Optimized candidate SHA256
`77e724c3d9c7eb68f10ec43d26771ae97c0a2816833bc3ceebc6875254eb02c6`
passed two-client remote-headed reuse across two exact host replacements.
All four acquisitions and synthetic evaluations passed, with both original
Chrome process identities, profile IDs, host types, display names and display
allocation IDs preserved. Historical capability bindings remained unchanged.
Final state had zero browser records, and exact fixture census found no residue.
Private receipts and reproducible probe are in campaigns/p160/shared-headed-restart-4zYWjF;
the terminal log is census/retained-launch-two-restarts.log.

Read-only production X11 inspection found windows declaring the original
browser PID on its retained allocation's display. Only window IDs and PID
properties were retained, with no titles, screenshots or page content. The
same display also contains windows for another retained browser, so this is
display-location evidence and does not establish exclusive display isolation.
Evidence is census/original-display-process-proof.json. Production state was
not changed by these probes.

material_blockers: Preventing future overwrite does not restore the already
damaged production record. A bounded recovery must join exact current process,
owner/profile and unique retained allocation evidence before updating that
projection. A1 remains open until final installed production reuse passes;
display isolation, external presentation, doctor and AX remain separate open
criteria. The production timer remains inactive.

next_action_or_stop_reason: Implement and qualify recovery of the damaged
launch projection without granting new profile or lifecycle authority, then
publish the combined repair and repeat final-candidate production acceptance.
Continue causal diagnostics and W2–W5 under the existing goal authority.

### W1 bounded recovery implementation and admission mismatch

A focused recovery module now restores only erased host, display name and
allocation ID for an explicitly selected retained route. It requires a unique
current-boot allocation with reciprocal browser/profile/session/PID linkage,
current lifecycle owner authorization, fresh process digest and endpoint digest,
a process-bound X11 window observation, a second process check and a locked state
comparison. It does not promote permissions, change readiness, assert display
exclusivity, navigate pages or replace processes. Its typed error names the failed
predicate. Allocation mismatch and ambiguity tests passed.

The synthetic damaged-record fixture on the prior prevention-only candidate
failed earlier than daemon launch with `service_access_plan_route_browser_conflict`.
Its access plan selected the original browser but its generated request copied
replacement posture defaults into explicit constraints, changing admission's
answer against the same state. The generator now preserves only caller-supplied
posture constraints for reuse. The existing route-hint test now covers this
round trip and passed; explicit route-conflict tests remain in place.

The failed fixture and cleanup receipts remain private in
campaigns/p160/shared-headed-recovery-QeTms3 and census/retained-recovery-red-cleanup.json.
No fixture residue remained. Broad access tests and an optimized candidate build
are active; no production recovery or publication has occurred in this slice.

The 66 access-planning tests, service-client suite, final workspace clippy,
formatting, docs build and handoff documentation checks passed. The final
candidate rebuild includes the synchronized help text; installed recovery proof
is still pending. A negative X11 observation currently returns its named
predicate; preserving its underlying cause and complete trusted request/build
context remains part of AX and must not be inferred from these focused tests.

The first recovery candidate `c8dac8ee` admitted and executed synthetic reuse,
but failed the assertion that the damaged host field was restored. Receipt root:
campaigns/p160/shared-headed-recovery-pv8KEp. Exact teardown found no residue.
The recovery entry point still depended on the replacement launch's resolved
host, which can be headless after removing default constraints from reuse.
Recovery now keys solely on the explicitly selected route and the damaged
retained-record pattern; all allocation, process, owner and display proof gates
remain. This is the third distinct verification attempt for this work unit;
failure will trigger local reframe rather than another unchanged retry.
All 92 route/host tests passed before this small entry-gate correction. The
corrected optimized candidate build is active in
census/retained-recovery-route-gate-build.log.

## Execution checkpoint 11: damaged headed records recover and remain reusable

state_transition: active → active

acceptance_state: A1–A4 and AX incomplete. Damaged-record recovery and repeat
reuse are qualified on a disposable optimized candidate; production remains on
the checkpoint 9 generation.

progress_classification: blocker_reduction

evidence: Corrected source `f2786e1a` is committed and pushed. Candidate SHA256
`56399d594144fb6e7e6ab055b8a517dec78bf13d2c7bc97ae1c3d3667d6dcb55`
passed the third bounded verification attempt. Two deliberately damaged headed
records recovered their original host, display name and allocation IDs through
ordinary planned tab requests, with original process identities and historical
capability bindings unchanged. Four acquisitions/evaluations passed across two
host replacements. Independent snapshot comparison confirmed the restored
fields; final state had zero browsers and exact fixture census found no residue.
Evidence and probe source are private in campaigns/p160/shared-headed-recovery-lZDjAE;
terminal and cleanup receipts are census/retained-recovery-route-gate-proof.log
and census/retained-recovery-qualified-cleanup.json. Prior failed attempts remain
available. Corrected entry-gate clippy and formatting passed.

material_blockers: Production recovery and exact-release acceptance remain
unproved. The full release build is active in census/retained-recovery-release-build.log.
Runtime ownership warnings, causal diagnostics, external presentation, doctor,
monitor and timer criteria remain open. Synthetic recovery proves neither
exclusive display isolation nor operator-visible presentation.

next_action_or_stop_reason: Resume the existing release build, qualify its exact
binary with the damaged-record/two-restart fixture and authenticated ownership
checks, then publish while preserving all current production browsers. Run the
original-profile synthetic request to exercise the installed recovery before
claiming A1 progress in production. Continue AX and W2–W5.

## Execution checkpoint 12: production original-profile recovery passes

state_transition: active → active

acceptance_state: A1–A4 and AX remain incomplete. The original occupied-profile
workflow now passes on the installed release, with its damaged launch projection
restored through the product path.

progress_classification: blocker_reduction

evidence: Full release SHA256
`d870205e3bed66e882a40b4d17a54d153e183eb8fcb2e2dd1f45c90f87e597e3`
from source `f2786e1a` passed damaged-record recovery across two host replacements,
two independently authenticated clients retaining their original handles,
foreign-handle denial (`profile_child_subject_mismatch`, `no_effect`) and the
source-free installer fixture. The authenticated fixture's 21 deliberately
retained processes were disposed by exact fixture identity; none remained.
The shared-headed recovery fixture left no residue.

The candidate is installed as `0.28.0-d870205e3bed-796e97509b3e`, with support
manifest SHA256 `796e97509b3efe1c4613c418d2e4c59e6c14cb987cfd35d19e23f452f80c252c`.
All three production executables independently matched the binary hash. All
four original browser process identities survived activation and the subsequent
probe. The original-profile access plan selected the retained browser; ordinary
synthetic blank-tab acquisition, evaluation and release passed. Its host,
display name and allocation linkage were restored. No business operation,
browser replacement or manual metadata repair was used for this result.
Publication, rollback material, exact executable readback, probe and doctor
receipts are private under campaigns/p160/publication-retained-recovery.
Exact-release fixture roots are shared-headed-recovery-release-ceKnhk and
retained-repair-eyys6m.

material_blockers: Doctor remains nonzero, with the same 13 issue rows before
and after publication: operator journey, upgrade readiness, retained terminal
history warning, eight principal/binding warning rows, monitor readiness and
unknown pressure ownership. Their individual dispositions and remaining A1
matrix are unfinished. No A2 visibility or display exclusivity is claimed.
The interlock timer remains inactive. Causal lookup, first-cause/source/build
diagnostics and AX reconstruction remain unfinished.

next_action_or_stop_reason: Complete causal diagnostics for profile failures,
then finish current ownership dispositions and the final installed control
matrix. Continue the operator journey, supported maintenance readiness and
scheduled-operation acceptance without treating this reuse pass as whole-plan
completion.

## Execution checkpoint 13: tab identity and cleanup failures reproduced

state_transition: active → active

acceptance_state: A1–A4 and AX remain incomplete. Incident 0156 now has an
isolated reproduction of both reported failure patterns on the exact installed
candidate binary; no product repair is claimed.

progress_classification: outcome_progress

evidence: The updated incident note records the exact binary, saved request,
before/after CDP target sets, controlled disposable host interruption, source
causes and hashed private receipts. Explicit target close removed another target.
Release after host interruption returned successful verified effect despite
skipping physical closure and leaving the intended target alive. Three fixture
attempts included two preserved setup failures; all fixture resources were
disposed with exact ownership checks. Production browsers were untouched.

material_blockers: Close execution ignores target selection and discards CDP
close errors; release omits retained-target recovery and marks skipped physical
cleanup closed. Existing regression coverage encodes that incorrect lifecycle.
Named-profile inference during direct tab control and its unhelpful failure
classification also remain A1/AX cases. Historical incident request/build
provenance remains missing, distinct from the confirmed current reproduction.

next_action_or_stop_reason: Repair exact-target selection and authorization,
retained release recovery, physical target-set verification and truthful cleanup
outcomes. Reuse the disposable reproducer for green verification, including
negative selectors and unrelated-target preservation. Continue remaining A1/AX,
then A2–A4; no broad incident-profile cleanup or new approval is needed.

## Execution checkpoint 14: exact cleanup and surviving-handle control repaired

state_transition: active → active

acceptance_state: A1–A4 and AX remain incomplete. The tab-cleanup repair passes
isolated candidate verification; it has not passed the production publication
gate or resolved the consumer's separate lease/handoff attestation blocker.

progress_classification: blocker_reduction

evidence: Optimized candidate SHA256
`7bcaad1737cba146cdfa85a1a147ae00220042c6fd9f95326d7bd26b228638e1`
passed the disposable HTTP/CDP fixture in campaigns/p160/tab-cleanup-green-CORyLt.
Four negative requests (conflicting selectors, empty target, another declared
subject, and an unattributed bootstrap target) preserved every physical target.
Authorized close removed only its explicit target. Release after an exact host
interruption recovered and closed only its target. Another original handle then
evaluated 42 on its surviving target. No fixture residue remained after teardown.
The private ledger SHA256 is
`6c6fed2a5f5d75d6298edb1934869ef33a2aa26bb4cab6fc4aab13849c53a4e0`;
the saved fixture source SHA256 is
`121630d38610be2c2cc0b9f68604af952508521b40f1ed26cf3eebbe65072082`.

Close now resolves target, tab and handle selectors without active-tab fallback,
requires current attributed child close authority for service requests, checks
the physical browser target census, sends one close command and waits at most
two seconds for verified removal and peer survival. Local page state is retained
when verification fails. Release preserves the handle when requested physical
cleanup is unverified. Retained recovery can switch a borrowed connection to
another original authorized target after rerunning its identity fence; it never
drops an owned browser process to achieve that switch. The request schema, field
role ledger and generated client now recognize top-level tabId. Failure records
correlate typed close decisions to the request and source location, and terminal
provenance includes the requested tab. Full AX build and cross-surface proof
remains outstanding.

Validation: the new real-CDP-transport regression failed on the old last-tab
check and passes with acknowledged removal, delayed destruction, refusal,
target-still-present and peer-loss cases. Twenty lifecycle tests, 27 request
contract tests, three provenance tests, 14 recourse tests and three owner-guard
tests passed. The client suite, generated contracts, handoff documentation,
docs build, Rust formatting and workspace clippy passed. The final API/MCP
parity readback caught a missing MCP tabId schema field; the schema follow-up
adds that field and passes the full API/MCP parity check, workspace clippy and
format check. The isolated HTTP/CDP evidence above predates only that MCP schema
addition; final combined-candidate publication remains outstanding. These
are focused and isolated-live checks, not whole-production acceptance.

Preserved failed verification: the first candidate needed bounded observation
after Chrome's asynchronous close acknowledgement. A later continuity extension
showed that the borrowed manager retained only the released target; that defect
was locally reframed as surviving-handle recovery and repaired before the final
pass. No failed result was erased or treated as an unchanged retry. Private
roots kDDpyT, CktyTy and g0SGcY retain these distinct candidate results and exact
cleanup receipts.

material_blockers: The newly appended SoyLei r2 report remains a priority W1
dependency. Its diagnostics show an exclusive session but a shared handle lease;
the current attestation requires both to be exclusive. It also has no handoff
receipt. Registration's raw relative userDataDir versus named-profile resolution
is a separately reported identity-digest mismatch requiring source verification
and reproduction. No consumer reconciliation or payment input is authorized by
the incomplete attestation, and no such action was performed here.

next_action_or_stop_reason: Repair the lease/handle attestation contract and
registration's canonical profile identity, then qualify and publish the combined
candidate under existing authority. Continue full A1/AX and A2–A4 acceptance;
do not bypass the consumer's identity gate or claim Plan 0160 complete.

## Checkpoint 15: named-profile identity reconciliation

Integrated consumer contribution 3cb446bd2f3906c7cf58bcbec15caf23d3bd3813
through the primary Plan0160 branch. Registration, rotation, unbound lease
projection and recovery now resolve profile names through the same resolver as
browser acquisition before hashing the physical directory. An independently
written regression failed on the prior implementation with different digests
for one named profile, then passed with the contribution. All 25 profile-lease
tests passed, including guarded rejoin with an existing capability and denial
of a conflicting foreign principal. Workspace Clippy passed. Evidence logs:
/tmp/p160-r2-profile-identity-red.log, /tmp/p160-named-profile-lease-green.log
and /tmp/p160-named-profile-clippy.log. These are local source qualification;
production publication and consumer readback remain outstanding.

The separate diagnostics repair remains in progress: the caller handle carries
older lease metadata, while the current snapshot derives an exclusive lease.
The current owner has no transfer receipt; its separately persisted lifecycle
record carries Ready/Owned custody and a package-launch identity digest. The
repair must verify that custody against the exact owner generation and process,
without manufacturing a handoff or weakening foreign-owner rejection.

## Checkpoint 16: current lease and launch-custody attestation

Diagnostics now join the supplied active handle with current tab/session lease
records. A configured runtime name can identify its catalog profile without
being mistaken for a different physical profile. Released handles, foreign
profiles/targets and conflicting current records remain denied. Custody accepts
a current transfer receipt or an exact Ready/Owned managed-launch lifecycle
record bound to the current boot, generation and process digest. No handoff
receipt is manufactured. The ownerCustody projection identifies the basis and
individual launch predicates for diagnosis. Client types and all five guidance
surfaces describe the corrected contract.

Two focused regressions reproduced the old lease-snapshot and missing-handoff
failures. Six diagnostics tests now pass, including configured-alias acceptance
and foreign-alias denial, stale generation/digest/boot/cleanup evidence, missing
custody and released handles. Workspace Clippy, formatting, client suite and
documentation build passed. Private logs use the /tmp/p160-attestation prefix;
final source tests are p160-attestation-alias-tests.log and Clippy is
p160-attestation-alias-clippy.log.

The optimized combined candidate SHA256 is
2cd3bc25245d43622b27e0ebaa6286b4e31402733e2f0f2911ba5824144f6d51.
The disposable named-profile fixture attestation-green-Ji8iGE passed initial
and post-host-restart attestation, four no-effect negative cleanup requests,
exact-target close, retained release, peer preservation and original-handle
evaluation returning 42. Fixture cleanup reports zero residue. Private evidence
under the P160 campaign has ledger SHA256
8b48a0c137bab29bff328667165e3b7899fc6079b9d595be7d5b0585379aaab4
and probe-source SHA256
b329c995b0c5627544be7063ae5dab74683a8d6b43adcdaf0b5db1b69fc78599.

Preserved limitations: atPhai reproduced the diagnostics catalog/runtime-name
mismatch and led to the configured-alias correction. YtBRkU and 0dy9Uh then
passed alias diagnostics but exposed contradictory raw tab-switch profile
guards when catalog ID and runtime name differ. After that bounded sequence,
the unit was reframed: Ji8iGE qualifies matching names, as in consumer Default.
The raw alias-routing defect remains a W1/A1 blocker and is not counted as
resolved or hidden by the passing fixture. All failed fixture roots and cleanup
receipts remain private. Production publication, consumer readback, complete
A1/AX and A2–A4 acceptance remain outstanding.

## Checkpoint 17: production publication and Default-owner readback

Published source 495649c6 as production generation
0.28.0-787159fbcae7-a58bd361158f. Binary SHA256:
787159fbcae76a2cde1bccf51bc131300cea9abae28d4fd829b7a3b282ab0bec.
Support-manifest SHA256:
a58bd361158fe251b78df97b7ec497c49841511f76ecb8556db9390d1592b2be.
The full release build passed. Its disposable named-profile fixture w5m1qI
passed the same attestation, restart, exact cleanup and surviving-handle checks
as the CI candidate. Release-fixture ledger SHA256:
358ec376b04c63c630bf07c6f320948c2ea7724d71a72beba5512247b8177c46.
All fixture residue checks passed.

The controlled host replacement used the standing publication authority and
retained the previous immutable generation for rollback. Fresh admission found
no active jobs. All four preexisting browser process identities survived; the
new host PID is 57482. The publication receipt and before/after configuration,
validation logs, doctor results and rollback scripts are private under
campaigns/p160/publication-attestation. Shared installed skill guidance now
reflects the deployed attestation and verified-cleanup contracts.

A separate authenticated P160 client was correctly refused when it attempted
diagnostics through the consumer's still-active child connection:
owner_connection_still_active. That is not consumer-side readback and was not
bypassed. A new P160-owned synthetic blank tab in the same existing Default
browser then returned complete=true, managed_launch custody, owner generation24
and bounded evaluation result42. Its release physically closed that exact tab.
Post-probe readback preserved the consumer target, its complete child-access
record, all three original active target records and all four browser processes.
No payment input or business submission occurred. The consumer must recheck
through its original connection; its own capability's fresh guarded rejoin
projection is available for evaluation but no consumer lease was applied here.

Install doctor remains nonzero with 12 issues: operator journey and upgrade
readiness, retained terminal-history warning, seven principal/lease warning rows,
runtime-monitor readiness and unknown pressure ownership. This publication
therefore does not establish full operational readiness or complete Plan0160.
The maintenance timer remains inactive.

Next W1 unit: fix the already-reproduced catalog/runtime-name routing conflict.
apply_existing_session_profile_selection in runtime/daemon.rs compares an
explicit runtimeProfile with the catalog ID, while active_browser_profile_mismatch
in runtime/profile_lease.rs compares it with the active runtime name. Join both
through the exact configured physical identity; retain foreign-profile denial.
Use the preserved alias counterexamples, not another broad failure review.
Then continue remaining A1/AX and A2–A4 evidence and final installed qualification.

## Checkpoint 18: alias routing and explicit-selector fencing

state_transition: W1 alias repair active to isolated qualification complete

acceptance_state: A1–A4 and AX remain incomplete pending final installed proof

progress_classification: blocker_reduction

The catalog ID and configured runtime name now pass the same current-owner
selection. Active-browser alias reconciliation additionally checks the physical
directory and current endpoint identity. Retained-tab recovery resolves and
verifies the configured physical directory against the owner digest, accepts
only its exact configured alias and preserves that verified profile metadata on
the borrowed browser connection. Process ownership and child permission checks
remain required; borrowed metadata never grants process cleanup ownership.

The first live alias verification exposed a separate explicit-selector gap:
tab_close could discard a conflicting runtimeProfile despite closing only an
authorized target. The shared handle validator now checks every explicit
runtimeProfile, profileId and profile field, including nested params, before
child admission or effects. service_tab_profile_selector_conflict identifies
the field and source function, classifies no_effect and directs comparison of
the requested profile with the current owner. This also protects other handle
actions that intentionally skip browser auto-launch.

evidence: The original alias-selection unit test failed before repair. The
93 runtime routing tests, four child/selector tests, 14 recourse tests, 20
lifecycle tests and exact-close transport regression pass, as do workspace
Clippy, formatting and documentation build. Private logs use the
/tmp/p160-routing prefix. Final optimized candidate SHA256:
ada9cae92dd710fbf1bac53b32007aa7894f0f8381cea4ec382afb6925c990aa.
Disposable fixture routing-alias-green-Uu3rMr passes both raw alias spellings,
five no-effect negative selectors/subjects, exact target close, retained release
after host interruption, original peer-handle evaluation42 and complete
attestation before and after restart. Ledger SHA256:
deb851eb440aa6fd1dbad90db50f2df36ab27743929e16169fc3562e77d71ac3.
Probe source SHA256:
b5b1f7671f9efab6457aecabb4c9472f9ea1d02796051edff79d924257caae69.

The three materially distinct fixture attempts are preserved: apVr58 exposed
the discarded conflicting selector; DdPtDm passed that denial but exposed the
retained alias comparison; Uu3rMr passes the repaired sequence. All exact
fixture cleanup receipts report no remaining owned residue. No production
browser was used for these failure reproductions.

material_blockers: This source is not installed yet. Whole A1 positive/negative
coverage, two independently authenticated original handles, AX reconstruction,
operator journey, doctor readiness and scheduled cycles remain required.

next_action_or_stop_reason: Freeze and qualify the full release binary, publish
under standing authority, and continue the remaining frozen acceptance rows.

## Checkpoint 19: session inference and native child custody

state_transition: W1 session-only reproduction to repaired isolated control

acceptance_state: A1–A4 and AX remain incomplete

progress_classification: blocker_reduction

A session-only tab switch reproduced explicit_profile_conflicts_with_current_owner
because owner selection compared the host's inherited startup default with the
session's verified profile. Selection now checks explicit request selectors,
including every nested field, while inheriting the exact current owner when no
selector is supplied. Conflicts report the field, source function and no_effect
recourse. The focused regression failed before repair. The 94 routing tests and
14 recourse tests pass. Fixture session-only-check-XGOgjL passes session-only
control, exact cleanup, retained release, original peer control, complete
attestation, and session-only/catalog/runtime-alias control after restart.
Its ledger SHA256 is
afb28b1666da029caf53cc7512654502c000eb1285cb68c6a52351976f27e518.

The expanded navigation/input workflow exposed two additional blockers:

- persist_service_owned_navigate_tab used the new-tab writer with no newly
  issued child grant. That overwrote the existing tab's profileAccess with null.
  Navigation now preserves existing custody and lease metadata while updating
  page observations. The original handle remains usable after navigation.
- Raw native fill accepted another registered client's handle and changed that
  client's synthetic input. Fixture a1-raw-input-664nE9 records the successful
  forbidden request and both clients' independent readback. Native commands
  carrying handles now validate current child permission, select the exact
  authorized target, reject conflicting selectors and avoid auto-launch around
  a refusal. Specialized handle actions retain their existing checks.

A separate reconciliation race also reproduced in a unit test: an async probe
could replace a newer tab grant or resurrect a removed tab. Tab merge now applies
an observation only if the current record still matches the probe's starting
snapshot. This race repair alone did not fix navigation: live attempt
a1-operational-Ig2J5F still failed, which led to the navigation writer diagnosis.
The first race-test invocation had a test-fixture type error; the corrected test
then failed on the actual erased grant before the fix. All 69 service-health
tests and 21 tab-lifecycle tests now pass. These results are separate from the
live navigation root cause, not an attribution of that failure to the race.

Final optimized candidate SHA256:
2f014ee7fc20b70a8959a582651b59553718b036aacaaefd83fe84d103ca18ea.
Its full source patch and binary are retained privately. Two independent
registered clients with named and custom-directory profiles each navigate to a
synthetic page, fill a field, click a button and verify the result before and
after one exact disposable host interruption. Both original handle objects and
both original browser process identities survive. Cross-client evaluate and
raw fill are denied; field readback proves no foreign input effect.

Headless fixture a1-operational-Dc2M4z ledger SHA256:
1070507403b047a28d4e1f8673d00fce00eb4035a782a4342341d369976f8111.
Remote-headed fixture a1-operational-uGQtdX ledger SHA256:
9cfa26c5b9baa00857a5cd67e0596307319236ea22b6f2907c80716f6d4aa471.
Both pass. Own-handle releases succeed; exact residue cleanup reports zero
remaining processes, including two headed-fixture helpers removed by pidfd.
All failed fixtures are retained and their exact cleanup is complete. Earlier
working candidates are identified by digest and working-source label, not a
frozen commit; their saved probe receipts do not claim an archived binary.

Workspace Clippy, formatting, docs build, route-confusion gates and remote-view
handoff documentation checks pass. The first full Rust run passed the CDP tests
but failed the parallel CLI partition: 2006 passed, one named-profile identity
test failed, 57 ignored. That regression read HOME without an environment guard
while tests in its module changed HOME. The module was absent from the serial
partition list. It now runs in that existing serial lane, and the regression
holds the environment guard. The first failure is retained; the corrected full
suite is running. Private validation artifacts, the source patch and a manifest
are under campaigns/p160/session-native-custody-validation; fixture manifests
bind each ledger, probe source and saved state. No production publication or
consumer business input occurred in this checkpoint.

material_blockers: Whole A1 still needs terminal browser close/reopen and final
installed qualification. Raw commands without handles need explicit ownership
coverage. AX causal reconstruction, the A2 authenticated external operator
journey, A3 supported upgrade/doctor readiness and A4 scheduled cycles remain.

next_action_or_stop_reason: Finish the running full Rust suite, freeze the
validated source and qualify the release binary. Continue the named remaining
A1 cases before final installed acceptance; do not treat successful synthetic
input as full operational readiness or retry business payment work here.

## Checkpoint 20: implicit native commands respect child custody

state_transition: A1 no-handle ownership bypass reproduced and repaired

acceptance_state: A1–A4 and AX remain incomplete

progress_classification: blocker_reduction

Two self-declared clients sharing one profile reproduced a raw fill without a
serviceTabHandle changing the other client's synthetic input. The failed fixture
a1-shared-native-Ov2Mvo is retained; its ledger SHA256 is
1bc449fe10f4a90097d3d27db665b673af4dc8ca67194228ab2b77e2c1266056.
Native Service commands now resolve their target against current child custody,
then use the existing handle authorization before effects. An implicit command
uses the caller's owned active target or sole owned target. Missing, ambiguous,
conflicting or foreign targets fail without selecting a peer. New-child creation
retains its broker admission path.

The optimized working candidate SHA256 is
9f11ce76c135fc101c0c9a141fb558052d6a0d8494b0f86aae1169340eb59c0d.
Its binary is archived under the private campaign retained-candidates directory.
Headless fixture a1-shared-native-o7e7vq passes, ledger SHA256
da97818af8a32fca4f251ca791db1d54028c7d4f404bd0d359aa941b46909bd7.
Remote-headed fixture a1-shared-native-6Vkia0 passes, ledger SHA256
e51c8f6f568dec049808188b33c289f6a686d0e1ff51043501f6725ef1e1f044.
Each verifies foreign input denial, successful implicit own input while a peer
was active, independent field readback and conflicting-selector no_effect.
Manifests bind candidate, probe, ledger and saved state. All three exact fixture
cleanup receipts report zero remaining owned processes.

The 95 routing tests pass. The corrected checkpoint19 full Rust run subsequently
failed two retained-browser dispatch tests because their fixture omitted the
profile record required by recovery's physical-identity check. The fixture now
includes that record; all 13 focused dispatch tests pass. The original failures
remain preserved. Full Rust validation and workspace Clippy are running for the
new dispatcher source. The earlier full release build finished, but source
advanced during that build; it is not qualified for publication as this repair.

material_blockers: This repair is not installed. Terminal browser close/reopen,
final installed A1 qualification, AX reconstruction, A2 external operator journey,
A3 supported upgrade and doctor readiness, and A4 scheduled cycles remain open.

next_action_or_stop_reason: Finish the current validation handles, test the
supported terminal close/reopen lifecycle, then freeze and build the production
candidate. Preserve the consumer's original capability and browser throughout.

Terminal lifecycle follow-up: fixture a1-terminal-reopen-qiS1yo reaches successful
original-handle navigation and input after host restart, but service_browser_close
fails with "Can only leave open a launched managed runtime profile". Its subsequent
own-handle release fails runtime_owner_generation_stale; peer release succeeds.
The terminal-close failure is currently classified unknown/effect_uncertain.
Source inspection shows retained recovery sets CloseBehavior::Detach, while the
explicit service close handler delegates to handle_close without changing that
intent. The common close path first preserves the owner, then attempts managed
detach on a borrowed manager. This is a concrete A1/AX repair lead, not green
terminal-close evidence. Ledger SHA256:
3732c1af56729daa48b4b0825ed2f270a47acabb278fd7dcf5fd6333f4829a0c.
The saved probe and state are bound by its evidence manifest. Exact disposable
cleanup signaled 13 positively identified processes through pidfd; none remain.
No production process was involved. Reopen was not reached.

## Checkpoint 21: terminal close and replacement admission

state_transition: supported terminal close repaired; replacement admission in validation

acceptance_state: A1–A4 and AX remain incomplete

progress_classification: blocker_reduction

Explicit service_browser_close now selects terminal shutdown rather than the
detach default installed by retained-tab recovery. Service callers must pass the
current profile full_shutdown permission before any close transition, followed
by the existing lifecycle-owner fence. A denial names the missing permission,
profile, policy revision and source, with no_effect recourse. Ordinary retained
connection teardown continues to preserve the browser.

Fixture a1-terminal-reopen-Ommtbg verifies participant denial with continuing
original-handle usability, authorized terminal close, exact original process
absence, stale-handle denial and preservation of the other browser and handle.
Reopen then fails runtime_owner_generation_stale. Ledger SHA256:
98396fc5abe3353a382f8ba1bbcb993fd0b4ef23c5b7026f9495e9c42aeb1d4c.
Its archived candidate SHA256 is
3f121e588cf483290b5a54687dcc23918195468487095210d39247cd135159ae.
The fixture manifest binds source, ledger and saved state; exact cleanup found
zero remaining owned processes. This is partial close proof, not a reopen pass.

The lifecycle admission code reloads retained owner history for a new request.
It previously released a proven terminal binding only for remote_view_open.
Tab and window creation now use that same exact-session, current-owner,
no-pending-transfer, terminal-state and satisfied-cleanup gate before launch.
Registration must still atomically establish the replacement owner. Navigation
and mismatched sessions remain denied. The existing focused regression was
extended: it fails before repair and passes with the lifecycle suite afterward.
The replacement candidate build is still running.

The full Rust script completed successfully. Source advanced during its serial
partitions, so this is not represented as one frozen-source run. Subsequent
focused service-health tests (7), recourse tests (14), lifecycle tests and
workspace Clippy pass for the affected repairs. Docs build and remote-view
handoff documentation checks also pass. Private validation is under
campaigns/p160/terminal-close-validation. No production publication occurred.

material_blockers: Reopen live verification and final installed A1 qualification
remain open, along with A2, A3, A4 and AX. Stale-owner errors still need the AX
classification and causal reconstruction checks already required by the plan.

next_action_or_stop_reason: Test the completed replacement candidate in the
isolated terminal close/reopen workflow, freeze the validated source, and proceed
to production candidate qualification under the existing authority.

Replacement follow-up: a1-terminal-reopen-KzstjR still fails reopen. Its separate
runtime-lifecycle-registry.json proves the closed browser remains closing with
cleanup owned; the replacement gate correctly refuses that state. The ordinary
state.json projection omits these lifecycle records and cannot establish this
proof by itself. Ledger SHA256:
9e8561268e8adf40a21200ae05d3b1f42f28a3f3eadb961cb0a4f0da7c6905f5.
Archived candidate SHA256:
e012ddfc25139e6616f702231a396bfae7abefca26c332f0731daced14c81592.
Both owner and lifecycle registries are now included in the fixture manifest.
Exact disposable cleanup reports zero remaining processes.

Source diagnosis: terminate_runtime_browser returns default shutdown evidence
when the process is already absent, and its other exit branches never populate
exact_process_exited or profile_lock_released. The borrowed manager cannot supply
owned-child shutdown evidence either. Consequently the common close path skips
CompleteClose but still returns closed:true. The third materially distinct
terminal fixture ends this work unit. Reframe the next unit around the missing
retained-process shutdown barrier: capture exact process/profile evidence before
CDP close, verify exit and lock release afterward, commit terminal cleanup only
on that evidence, and refuse a success claim when terminal proof is incomplete.
Do not weaken the replacement gate or manufacture cleanup evidence. The 16
lifecycle tests, final Clippy and formatting checks pass; live reopen remains red.

## Checkpoint 22: retained terminal shutdown and reopen verified

state_transition: missing retained shutdown proof repaired with live close/reopen

acceptance_state: isolated terminal close/reopen passes; installed A1–A4 and AX remain open

progress_classification: outcome_progress

The common close path now pins the recovered browser's exact process before CDP
close, comparing process and physical profile digests with its lifecycle owner.
It observes that pinned process and physical SingletonLock afterward. Only exact
exit plus absent lock permits terminal cleanup completion; an incomplete barrier
returns browser_terminal_close_unproven instead of closed:true. No lock deletion
or foreign process termination is introduced. Pre-effect identity refusal and
post-effect incomplete cleanup have distinct effect certainty and source-linked
recourse. Existing owned-child shutdown continues to supply its own evidence.

Headless fixture a1-terminal-reopen-6i1zZp passes, ledger SHA256:
6ce209f13b675dc56fc77c0016fbc9b179addacc80c2f2dcfcbfb77287b4d7d6.
Remote-headed fixture a1-terminal-reopen-bs8we0 passes, ledger SHA256:
73e9411a95655b231f351aff9e6faa1d6a9081fac3990d5025462d0d52efd875.
Both use archived optimized candidate SHA256:
302a59f7a68772b7892796b4225fcf134e199f598b358d91a28b393299e1ff12.
Each verifies original registered-client handles and synthetic input before and
after host restart, participant shutdown denial without lost access, authorized
terminal close, exact process absence, terminal/satisfied lifecycle state, old
handle rejection, reopen with the same capability and incremented owner
generation, complete diagnostics and working input, and peer preservation.
Both cleanup censuses found zero owned residue without sending any signals.
Fixture manifests include the separate owner and lifecycle registries.

The 14 close/launch tests, 14 recourse tests, formatting, docs build and workspace
Clippy pass. Initial Clippy rejected an unnecessary unwrap; the equivalent
pattern match passes. That style correction occurred during the optimized build,
so the digest is working-source evidence, not a frozen-commit build claim.
Private validation is under campaigns/p160/retained-shutdown-validation. The
next full release build must use frozen source. No production state changed.

material_blockers: Final installed A1 qualification, A2 authenticated external
operator journey, A3 supported upgrade and doctor readiness, A4 scheduled cycles
and AX causal reconstruction remain required.

next_action_or_stop_reason: Freeze and build the full production candidate,
qualify the exact binary, and publish through the already-authorized controlled
replacement path while preserving original production browsers.

## Checkpoint 23: frozen candidate installed with browser preservation

state_transition: qualified ec04ad55 release binary installed in production

acceptance_state: installed own-profile smoke passes; A1–A4 and AX remain incomplete

progress_classification: outcome_progress

The full release build completed in 8m50s from ec04ad55 with source unchanged.
Binary SHA256:
390ce3eb6ad4e0c2285a17cbc1c36183c06d53b5867c8234127df6f1b5a6d5e1.
The copied distribution binary has the same digest. Three fixtures qualify this
exact release binary: headless terminal close/reopen bU9U6z (ledger
3d3715daa29f361f40d11fe2478c1ec416a945f7214024f49c7b2bab6638e219),
headed terminal close/reopen qXATl7 (ledger
7e74cfbca55d23126b380b5dadf2e81e6d0e41dbc5696943e8bacffc969a3566),
and same-profile self-declared no-handle isolation ODDmJ4 (ledger
3b0e35e36bbafd054a52a68cbbd3d3973a9137793c0585911d96088d2632a588).
All pass and leave zero owned residue without cleanup signals. Full fixture
names, probes, state, owner/lifecycle registries and hashes are retained privately.

Controlled activation installed generation
0.28.0-390ce3eb6ad4-4bf6a8301e06 with support manifest SHA256
4bf6a8301e068d413d20a6f12338c5037715c48e083cd6744f24a70bf2259a47.
Bundled support assets and unit templates were verified unchanged. Fresh gates
found no active jobs and verified the old host identity and four browser
identities outside its cgroup. The procedure stopped dashboard services,
terminated only the old host, selected the sealed generation, and rebound
ingress from the observed new host and socket identity. Rollback configuration
and process evidence were retained. All four original browser identities and all
45 original tab custody projections remained unchanged after activation and smoke.

The installed smoke opened its own blank tab in the existing Default browser,
evaluated 42, obtained controlPlaneAttestation.complete=true with managed_launch
custody at the existing generation, and released only that tab. It did not use
the consumer's payment tab or capability. A narrow shared-skill paragraph now
documents the installed ownership behavior, retaining the complete=true business
input gate and a backup of the previous guidance.

Doctor still exits1 with 12 findings: operator journey, upgrade readiness,
retained terminal history, seven profile/principal lease findings, stale monitor,
and unknown pressure ownership. The workspace-candidate mismatch is resolved.
Controlled activation is not supported upgrade acceptance. The maintenance timer
remains off. Consumer original-connection acceptance is not claimed.

Private evidence: campaigns/p160/publication-ec04ad55, including the release
qualification, sealed generation, activation and installed-readback receipts,
doctor before/after, original tab custody comparison, and shared-skill receipt.
An AX preparation check found one journal record, one job and one event for a
returned fixture denial ID, with cause, permission, policy revision and source.
That journal record omits build identity; full AX reconstruction remains open.

material_blockers: Remaining installed A1/consumer proof, A2 authenticated
external operator journey, A3 supported upgrade and doctor dispositions, A4
scheduled cycles, and AX build-bound causal reconstruction remain required.

next_action_or_stop_reason: Continue the outstanding installed acceptance and
diagnostic repairs against the selected generation; do not mark Plan0160 complete
or enable the maintenance timer from this successful publication alone.

## Checkpoint 24: installed lease disposition and producer build identity

state_transition: installed recovery path refreshed; AX build-identity repair validated in source

acceptance_state: A1–A4 and AX remain incomplete

progress_classification: blocker_reduction

Installed lease readback still has seven findings across six records: two
legacy unproven profiles, three registered capabilities without owner binding,
and one record with generation and session-authority mismatches. They have not
been dismissed as historical or automatically adopted. The consumer's registered
capability now resolves to the current owner's physical profile digest and its
installed explanation offers rejoin_owned_browser with authorized rejoin. That
is a repaired available path, not original-connection acceptance. No consumer
capability was used or rebound. Private readback and disposition are under
campaigns/p160/installed-lease-disposition.

New failure records now capture producer buildIdentity with package version,
compiled source revision and repository tree state, exact executable digest,
and support generation/manifest digest only when the adjacent immutable manifest
matches that executable. Missing fields carry explicit reasons. The cached
producer identity never follows the later current-generation selector. Build
metadata refreshes on source and Git revision changes. Historical records retain
an absent identity when read; the reader does not invent their provenance.
Authenticated client observations cannot supply producer buildIdentity.

The optional record schema extension preserves historical v1 records. The
contract check, journal tests including mismatched support identity and historical
readback, workspace Clippy and docs build pass. Source build identity is not yet
installed or proven through a new live journal occurrence. Validation is retained
under campaigns/p160/build-identity-validation. Production remains on ec04ad55.

material_blockers: Consumer original-connection acceptance, remaining lease
dispositions, installed build-bound AX lookup and other selected causal cases,
A2 operator journey, A3 doctor/upgrade readiness and A4 cycles remain open.

next_action_or_stop_reason: Qualify the new record through an isolated producer
and restart readback, then include it in the next frozen candidate while pursuing
the remaining installed acceptance gates. No ownership checks were weakened.


## Checkpoint 25: live producer identity and restart readback

state_transition: AX producer build attribution verified through authenticated journal readback

acceptance_state: A1–A4 and full AX remain incomplete

progress_classification: blocker_reduction

Candidate 16963b880ec6ae07bbe67a2281301e2b8ad526c6 built successfully with
optimized binary SHA256
2047021bf935a7fac92cdd4b452bfc33c1b007cb35e8c0faf8b6b22efe4fb75b.
A disposable two-client browser fixture denied foreign input before effects.
Its returned provenance request ID resolves to exactly one journal occurrence
through the authenticated dashboard failure endpoint after the producer host
stopped. Two separately started dashboard readers return the identical record,
including the producing executable digest and compiled source revision.
The source tree is honestly reported dirty because the consumer note was modified.
Uninstalled support identity is absent with an explicit unavailable reason.
This proves persistence across producer exit and reader restart with the same
candidate; it does not claim an installed-generation or different-build reader test.

Two setup failures remain retained: the first probe selected a nonexistent
response correlation field; the second queried the browser stream port rather
than the dashboard journal endpoint. The investigation was reframed to an
isolated authenticated dashboard reading the existing occurrence, without
another browser launch. Both browser fixtures had zero owned residue at census.
No production or consumer browser was changed.

Evidence is private under campaigns/p160/ax-build-restart-SwcQyY and
campaigns/p160/build-identity-validation/live-readback-manifest.json. Readback
receipt SHA256: 5e0d85099166380d21627c02ef01ce472b735661bc7154c6384295c933a340e0.

material_blockers: The subject-mismatch record lacks expected/observed ownership
evidence and the precise source decision location. Build attribution alone does
not satisfy AX. Production still runs ec04ad55; consumer original-connection
acceptance and A2–A4 remain open.

next_action_or_stop_reason: Repair causal evidence at child ownership denial,
then verify returned-ID reconstruction and the remaining selected AX cases
before freezing the next installed candidate.


## Checkpoint 26: child denial causal evidence preserved

state_transition: child authorization now carries bounded decision evidence into recourse and journal

acceptance_state: source checks pass; installed A1–A4 and full AX remain incomplete

progress_classification: blocker_reduction

The policy evaluator captures expected/observed subject and connection hashes,
owner/caller assurance, requested permission, inherited and current permission
checks, connection state, reconnect intent, parent/current policy revisions and
the deciding source function. The existing native string-error transport carries
only this typed bounded evidence alongside its recognized denial reason.
Recourse preserves it in subject, and the terminal journal projects the same
validated fields as details.childAccessEvidence with the recommended repair.
Raw identity labels, connection identifiers and arbitrary recourse subjects are
excluded. Historical exact denial messages remain supported. Malformed evidence
does not gain a no-effect classification. Authorization behavior is unchanged.

The existing wrong-subject regression now verifies evidence round-trip,
privacy, malformed-input refusal and absence of denial evidence on successful
reconnect. Policy tests, caller ownership regression, failure-recourse tests,
workspace Clippy and docs build pass. All five user documentation surfaces are
updated. Private validation logs are under campaigns/p160/child-causal-evidence.

material_blockers: New live returned-ID causal reconstruction remains to be
verified on the frozen candidate. These changes are not installed. Consumer
original-connection proof, other selected AX failures and A2–A4 remain open.

next_action_or_stop_reason: Build the frozen optimized candidate and read one
actual denied cross-client input occurrence through authenticated dashboard
readback, including reader restart and exact build identity.


Checkpoint 26 live qualification:

The frozen optimized candidate passed the disposable two-client native input
fixture a1-shared-native-45iYfe. Foreign input was denied, own input succeeded,
and conflicting targets were denied. The returned request ID resolves through
authenticated dashboard readback to exactly one journal occurrence, one job and
one terminal event. All three carry the same child comparison evidence, including
unequal expected/observed subject hashes, the requested permission, both permission
checks, the deciding source function and the recommended use-own-handle action.
Two separate dashboard reader processes return the identical journal record after
the producer host exited. Exact source/build identity matches the frozen candidate.
No owned fixture residue remained; no production browser was changed.

Binary SHA256: a51db766584dcb25f8c22b95dcdc034a3c7fdace5088203f656afff3e5a79be6.
Readback receipt SHA256: dc676286cfcbff668d1262ad668307c76fe2586bc2dc068646b2ce2e81265ac9.
Private receipt: campaigns/p160/a1-shared-native-45iYfe/dashboard-readback-receipt.json.

This closes live causal reconstruction for the selected subject-mismatch case on
the isolated candidate. Installed qualification, other AX cases, consumer original
connection acceptance and A2–A4 remain required. The production candidate is still
ec04ad55.


## Checkpoint 27: explicit unknown profile fallback reproduced and repaired

state_transition: production no-launch selection exposed a further A1 defect; source repair validated

acceptance_state: A1–A4 and full AX remain incomplete

progress_classification: blocker_reduction

Fresh installed doctor still reports 12 findings. Its selected-generation axis
compares the current sealed payload with a historical failed transaction naming
a different generation. The installed payload, ingress and runtime convergence
axes are ready, but supported transaction and operator-journey acceptance remain
unproven. No readiness receipt was fabricated or historical transaction removed.

While preparing the A2 synthetic browser through the operating guide and Service
access-plan path, an explicit uncataloged runtime profile was replaced by the
automatically ranked existing default profile, including that profile's directory
and retained browser reuse hints. The returned plan was not executed. This is a
profile selection defect independent of the already-repaired child authority
checks. Private doctor, lease, workstation status and exact no-launch response
are under campaigns/p160/readiness-refresh-637c6077.

The access-plan selector now excludes automatic catalog candidates that differ
from an explicit runtime profile. Explicit identity also controls default
readiness selection, and profile-source provenance comes from the actual selected
profile. Existing cataloged explicit profiles retain their configured directory;
uncataloged explicit names remain the requested launch intent without borrowing
another profile or directory. No profile or browser was created for the repair.

The regression initially lacked a catalog candidate with any ranking match. After
adding a matching target identity, it reproduced the unwanted selection before
repair. All 56 service access-plan tests pass after repair, together with workspace
Clippy and docs build. All five user documentation surfaces are updated.

material_blockers: Candidate interface readback and installation of this repair
remain pending. The synthetic operator journey has not started. Original consumer
connection proof, selected AX cases and A2–A4 remain open.

next_action_or_stop_reason: Verify the frozen candidate's no-launch access-plan
response, then include this profile-selection repair in installed qualification.


Checkpoint 27 interface qualification:

The optimized candidate passed no-launch fixture a1-explicit-profile-8ljzVK.
The fixture first proves that its existing catalog profile wins automatic ranking,
then sends an explicit different profile through the service client and HTTP
access-plan path. The response preserves the requested name, has no selected
foreign profile/source/match, no borrowed directory or browser/session route, and
readiness names the requested profile. Service State contains zero browsers and
zero tabs. The fixture host exited normally.

Candidate binary SHA256: 58477aae1a965ba93acbb0669a2858ac3b637b047b4f485dfe8f92a174cefa12.
Ledger SHA256: cbff78248e4515a88250998190d7be3a18f76be4a405ee1028db7c0a83cd8289.
Private evidence: campaigns/p160/a1-explicit-profile-8ljzVK.

Next: qualify the combined source through the full Rust gate and final production
candidate build, then preserve original production browser identities during
installation and resume installed A1/A2 acceptance. No formal release is planned.


## Checkpoint 28: combined production candidate installed; dashboard ownership gap remains

state_transition: ebf0f0d0 release candidate installed with exact browser preservation

acceptance_state: installed cross-client isolation fails; A1–A4 and full AX remain incomplete

progress_classification: blocker_reduction

The frozen full Rust gate passed. The final release build completed in 8m58s.
Exact release-binary qualification passed headless and remote-headed original
registered-handle continuity through host interruption, terminal close/reopen,
foreign input denial, explicit unknown-profile preservation and build-bound
causal readback through two authenticated dashboard reader processes. All four
disposable fixtures had zero owned residue.

Installed generation: 0.28.0-8cc912b4d251-ea005893e8fb.
Binary SHA256:
8cc912b4d251375aca532209937df4445821159fab408387cd79079c51a2f9bf.
Support-manifest SHA256:
ea005893e8fb6709d66598ba0d05745d16cabbbd110500193295bdea4a74acf4.

Controlled activation preserved all four original browser PID/start/executable/
cgroup identities and all 47 pre-existing tab custody records. The runtime host
changed to PID30808; no active jobs existed at activation. Maintenance remains
disabled. Rollback payload and configuration backups are retained. Activation
does not constitute supported-upgrade or operator-journey acceptance.

The first installed two-client blank-tab smoke produced complete browser
attestation but no child grants; a cross-client evaluation of 6*7 succeeded.
Dashboard processes inherited unsafe_claim_any from the private EnvironmentFile,
overriding their fail_open_ephemeral systemd setting. The host already used
fail_open_ephemeral. The environment file was backed up and aligned with the
configured host mode; only the dashboard services restarted. Fresh process
readback confirms all three now agree.

The second smoke assigned and retained distinct child grants, but cross-client
evaluation still succeeded. This is unresolved production request-path behavior,
not a passing isolation result. Both attempts used only newly opened synthetic
blank tabs, and all four test tabs were physically closed with browser preservation.
Final verification confirms the same four browser identities and all 47 original
tab custody projections remain unchanged. No consumer page or payment input was used.

Private evidence: campaigns/p160/publication-ebf0f0d0, including qualification,
activation, first-installed-smoke, dashboard-mode-reconciliation, second smoke,
post-install doctor and original-custody records. Doctor still has 12 findings.

material_blockers: Cross-client evaluation is still allowed through the installed
dashboard path despite distinct grants. The isolated direct service path denies
it. Original consumer acceptance, installed AX denial reconstruction, A2 operator
journey, A3 doctor/supported upgrade and A4 scheduled cycles remain open.

next_action_or_stop_reason: Reproduce the dashboard-to-runtime relay path in the
isolated fixture and trace the effective command identity at child authorization.
Do not repeat production smoke until the remaining bypass is explained and repaired.

## Checkpoint 29: client handle identity borrowing repaired

state_transition: workspace client repair verified against the installed ebf0f0d0 runtime

acceptance_state: cross-client denial passes; journal read reliability and full A1–A4/AX remain open

progress_classification: blocker_reduction

The request helper copied clientSubjectId and identityAssurance from a handle's
profileAccess grant when the caller omitted those fields. Consequently, a second
client's labels appeared in attribution while authorization used the owner's
subject. The common handle-routing helper now inherits only target routing;
caller identity and assurance come only from explicit request fields or normal
server self-identification. Explicit caller identity remains supported.

An existing evaluate-builder regression was extended with an owner-bearing handle
and different caller labels. It failed before the fix with client:owner and passed
afterward. The full service-client suite, Rust format and workspace clippy gates
passed. The documentation build and remote-view guidance check passed. The isolated
authenticated dashboard fixture also denied foreign-handle
evaluation and raw foreign input while preserving ordinary own-client input.
A fixture setup attempt used a relative executable path and was rejected before
browser launch; the corrected absolute-path fixture passed. Both were terminal
and their owned-process census found zero residue.

The third installed smoke used the corrected workspace client against the existing
8cc912b4d251 binary. Both callers obtained complete attestation. Foreign-handle
evaluation returned profile_child_subject_mismatch with no_effect and expected/
observed hashed identity evidence. Both newly created blank tabs were physically
closed, with unrelated targets preserved. All four original process identities
and all 47 original tab custody records remain unchanged.

The immediate journal request returned HTTP 500, so the overall smoke failed and
its subsequent own-tab evaluation checks did not run. A separate read-only follow-up
returned HTTP 200 and exactly one occurrence for the denied request. Its source
revision, binary and support-manifest digests, child-access evidence and recommended
action matched the response and installed generation. This confirms durable causal
recording for this denial but does not excuse the unexplained initial read failure.

Private evidence: campaigns/p160/a1-dashboard-isolation-jsRqKb and
campaigns/p160/publication-ebf0f0d0, including second-installed-smoke preservation,
third-installed-journal-matched.json and third-installed-readback.json.
The installed executable was not replaced in this checkpoint. The corrected
client is the workspace package imported by these tests; propagation to other
consumer copies is not yet established. No consumer payment input occurred.

next_action_or_stop_reason: Diagnose the journal HTTP 500 without another browser
attempt; complete consumer identity continuity and client propagation, then the
remaining installed A1/AX and A2–A4 gates. Maintenance remains disabled.

## Checkpoint 30: bounded journal read contention repair

state_transition: source repair qualified; optimized HTTP acceptance pending

acceptance_state: full installed A1–A4/AX remain incomplete

progress_classification: blocker_reduction

An isolated no-browser dashboard fixture using the installed release binary
reproduced HTTP 500 within 2 ms when a writer held the journal lock. Both a short
write and a retained lock produced the same read-lock failure. This establishes
a concrete failure path, but the third installed smoke did not capture the first
500 response body, so its exact historical cause remains unproven.

The reader now allows up to 250 ms for concurrent writers before returning a
lock error. Shared-lock admission still protects a coherent journal snapshot;
no unlocked partial read or fabricated empty result is used. Dashboard journal
reads execute on a blocking worker so disk I/O and bounded lock admission do not
stall asynchronous request workers. Persistent lock contention still fails.

The existing malformed-line/latest-record regression was extended to hold an
exclusive writer lock while a reader starts, then release it and require a
successful coherent read. It failed before the repair and passed afterward.
All 13 journal tests passed, including pending-custody preservation under a held
lock. Journal contract and dashboard observation checks, workspace clippy,
format and documentation build passed. No production executable was replaced.

Private red HTTP evidence: campaigns/p160/journal-http-O4Ornq. The fixture launched
only an isolated dashboard and a positively identified lock-holder process; both
exited. Its synthetic journal and red response bodies are retained.

next_action_or_stop_reason: Build the optimized candidate and prove short-write
HTTP success plus persistent-lock refusal without browsers; then qualify the
combined production candidate and resume the original acceptance contract.

## Checkpoint 31: combined identity and journal candidate installed and verified

state_transition: 2b9640b8 release candidate selected in production

acceptance_state: installed two-client control and causal denial pass; full A1–A4/AX remain incomplete

progress_classification: blocker_reduction

The full Rust gate passed. The frozen release build completed in 9m10s.
The exact release binary passed headless and remote-headed two-client original
registered-handle continuity through disposable host interruption, terminal
close/reopen, dashboard foreign-handle evaluation denial, raw foreign-input
denial, explicit profile preservation and causal readback across two restarted
authenticated dashboard readers. Owned-process cleanup censuses found zero
residue. The release journal fixture returned HTTP 200 after 83 ms for a short
writer hold and HTTP 500 after 253 ms for a persistent lock, preserving the
coherent record and bounded refusal contract.

Installed generation: 0.28.0-bce7a26617d2-3039ae26bf90.
Source: 2b9640b86b9dc3998c8ef4f68498adbd34f1be7e.
Binary SHA256:
bce7a26617d25859651ce82144b67246fed6a4f5279e1d16cbd916fa0c776a6f.
Support-manifest SHA256:
3039ae26bf90f64f94129a058da54d35916bc946aebc3e86889550dc9474a507.

All 26 support assets and unit files matched their immutable manifest. Controlled
activation found no active production jobs and preserved all four original
browser process identities. All 54 preactivation tab custody projections,
including the 47 original protected records, remained unchanged. Fresh readback
verified all three production unit executables, both dashboard manifest endpoints
and consistent fail_open_ephemeral configuration. The new runtime host PID was
26518 at activation. The previous installed payload and configuration backups
remain available for rollback; no formal release was performed.

The installed two-client blank-tab test passed in full. Both clients obtained
controlPlaneAttestation.complete=true and evaluated their own blank tabs.
Cross-client evaluation returned profile_child_subject_mismatch with no_effect.
The immediate authenticated journal read succeeded and returned exactly one
occurrence joined by the request ID, with matching source, binary, support
generation, hashed causal comparison and recommended action. Both newly created
tabs were physically removed with target-removal proof and unrelated targets
preserved. Final readback preserved all four original browsers and 47 original
custody records. No business input or consumer capability use occurred.

The installed shared skill received a scoped identity and journal guidance update,
with its previous bytes backed up and other guidance preserved. The client fix
is active in the workspace package used by the smoke; other consumer copies and
the original consumer connection still require their own acceptance evidence.
The source tree state remains honestly dirty because the separately maintained
consumer incident note was modified during the frozen build.

Doctor still returns nonzero with 12 findings: operator journey, selected-generation
readiness, retained transaction history, seven profile/principal observations,
stale monitor and unknown process-memory ownership. The profile findings remain
warnings and have not been adopted, erased or treated as authority for cleanup.
Maintenance remains inactive. Controlled publication does not prove supported
upgrade acceptance or unattended operation.

Private evidence: campaigns/p160/publication-2b9640b8 contains qualification,
support-source review, activation backups and receipt, installed smoke, shared
skill publication hashes, and before/after doctor readback. Release fixtures:
journal-http-kOCD9g, a1-terminal-reopen-VZ06Rm, a1-terminal-reopen-4NkKZo,
a1-dashboard-isolation-KVoFHb and a1-explicit-profile-Mpc1F0.

next_action_or_stop_reason: Complete remaining profile dispositions and original
consumer continuity, run the dedicated synthetic authenticated operator journey,
then reconcile supported generation/monitor/resource readiness and perform the
scheduled A4 cycles. The full AX fixture matrix remains required.

## Checkpoint 32: production viewer configuration repaired; sharing ingress absent

state_transition: Missing sharing configuration repaired for the owned synthetic route;
the next viewer failure is isolated to the sharing origin.

acceptance_state: A1–A4 and AX remain incomplete. Original consumer connection
acceptance remains the highest-priority outstanding ownership proof.

progress_classification: blocker_reduction

The dedicated p160-operator-journey browser and its original durable handoff
remain retained. Authenticated handoff resolution returns the intended browser
and target. Anonymous access presents login without a remote frame. These checks
use the public origin from this host and are not external-vantage acceptance.

An instrumented viewer diagnosis established two distinct failures. The dashboard
automatically submits view_focus under its own caller identity; child custody
rejects subject_mismatch against the synthetic tab owner. Separately, the route3
sharingProfiles API returned an empty collection. Direct database readback
confirmed that connection3 had no sharing profile. The latter prevents the
dashboard from obtaining its shared iframe URL; the focus denial alone does not
explain the missing iframe.

A bounded production configuration transaction created the missing sharing
profile for connection3, after verifying its exact connection name and RDP user.
It used the runtime's expected name, Agent Browser Shared Session guacamole:3,
and inherited READ membership only from that connection's existing READ grants.
It changed no connection parameters, browser, route allocation, or other route.
The previous sharing tables were exported before the transaction. The SQL and
apply receipt are retained privately. Authenticated API readback then returned
the exact sharing profile, and the same handoff produced one Guacamole iframe.
This is a provider configuration repair, not an acceptance-receipt edit.

The rendered synthetic-pixel check still failed. A separate owner-authorized
view_focus succeeded but did not repair the pixel failure. The next iframe
capture showed a browser load-error surface. Certificate-verifying curl to the
derived production sharing origin failed with exit60: the certificate does not
cover that hostname. Current Cooper generated routes contain development sharing
origins but no production sharing-origin route. No TLS bypass was used. The
dashboard nevertheless displayed Ready while this iframe was unusable.

Three bounded viewer attempts are retained: initial missing iframe, iframe with
missing pixels after the sharing repair, and the owner-focused diagnostic with
the same pixel failure. Neither remote input nor the second authenticated viewer
was reached. Disposable viewer process readback found zero matching residue.
The owned remote browser stays available for the next changed-condition test.

Private evidence: campaigns/p160/a2-operator-journey/checkpoint32-evidence.json
binds the diagnostic events, owner-focus receipt, transaction and TLS readback.
route3-sharing-repair contains the pre-change export and reviewable repair SQL.
The consumer's incident note was preserved without modification by this slice.

next_action_or_stop_reason: Repair the production sharing-origin ingress through
the governed Cooper path, including valid TLS and the correct sharing backend;
then resume the same handoff. Reconcile provider provisioning because its route
inventory labels differ from runtime route IDs. Independently repair the
operator focus authorization flow and iframe readiness/error reporting without
borrowing tab-owner identity. Original consumer acceptance and A3/A4/AX remain
required; no unchanged fourth viewer attempt is admitted by this checkpoint.

## Checkpoint 33: sharing HTTPS repaired; cross-environment display proof fails

state_transition: Production sharing ingress is reachable with valid TLS;
physical window evidence exposes an incorrect production display binding.

acceptance_state: A1–A4 and AX remain incomplete. Display ownership and false
complete attestation are immediate A1/AX blockers. Original consumer connection
acceptance remains open.

progress_classification: blocker_reduction

Cooper inventory now includes the production sharing origin on pinned port8092.
Its external route strips Remote-User and related identity headers; it does not
forward dashboard identity onto a sharing-key connection. Inventory validation
and rendering passed. Existing Traefik file watchers loaded the local and bastion
routes without restarting either shared proxy. Public certificate-verifying HTTPS
returns 200 for the Guacamole shell and 403 for anonymous token requests, forged
identity headers, and an invalid sharing key. Cooper commit c50abc8 is pushed;
bastion commit 2842790 records the generated route. The pre-existing dirty
CODEX_LOG.md remains dirty with an appended operational note. Other pre-existing
Cooper edits were preserved.

The changed-condition viewer attempt local-viewers-mJIcQG reached the Guacamole
iframe but still failed synthetic pixels before remote input or a second viewer.
The frame now shows the Guacamole surface rather than a TLS load error.

An extension check initially inspected the unused legacy runtime directory. Its
old JAR is not the running mount. A mount guard rejected replacement before any
file write, but a following shell command still restarted the exact Guacamole
container. That sequencing error is retained in the private receipt. Actual mount
inspection then proved that the older generation path contains the exact same
JAR and script bytes as the selected candidate; no extension replacement was
needed or performed. The Guacamole asset test passed. All five known browser
processes remained present after that restart. Do not attribute the pixel failure
to the unused legacy JAR or treat the restart as an asset repair.

Fresh own-handle diagnostics still reported controlPlaneAttestation.complete=true
and display :12. Physical process readback showed that :12 belongs to the retained
P158 development route user, while production route3's user owns Xorg :14.
Read-only X window properties on :12 independently bound window0x600003 to our
synthetic Chrome PID45678. This confirms an actual misrouted production browser,
not merely stale display text. Own-handle evaluation still read the expected
synthetic page, demonstrating that correct tab content and complete current
attestation did not establish correct environment/display ownership.

The misrouted synthetic browser was contained through its exact PID descriptor,
after rechecking recorded start time, executable and dedicated profile path.
SIGTERM removed that browser and its window. Readback preserved the development
X server identity, all other X windows, and all four original production browser
identities. No remaining process carried the exact synthetic profile evidence.
No remote input was sent. No development server, retained development browser,
original production browser, route allocation or lease was cleaned up. The
synthetic handoff is retained as failed evidence; its browser is now absent and
must not be reported usable or silently replaced to claim continuity.

Private evidence: campaigns/p160/a2-operator-journey/production-share-ingress
contains public-readback.json, actual-mounted-extension-and-restart.json,
display-process-ownership.txt, synthetic-browser-physical-display.json,
misrouted-browser-containment.json and misrouted-browser-residue.json.

next_action_or_stop_reason: Reproduce the mismatched route-user/display binding
in isolation and repair launch admission plus control-plane attestation so a
production route cannot borrow a development X server. Then reconcile the exact
production route to its live display through supported ownership checks. Resume
operator qualification only on positively proved production resources, retaining
this failed fixture and the failed continuity evidence. Focus authorization,
iframe readiness reporting, provider provisioning, original consumer acceptance,
A3/A4 and the remaining AX matrix still require completion.

## Checkpoint 34: kernel display-owner admission and attestation repair

state_transition: False-complete attestation reproduced in a provider-free test;
shared live display-owner proof now gates route launch, preflight and diagnostics.

acceptance_state: Source qualification passes; installed A1–A4 and AX remain open.

progress_classification: blocker_reduction

The launch seam returned already_ready immediately after successful xdpyinfo,
before requiring a route user. Control-plane attestation checked browser/process,
profile lease and owner custody but did not check display ownership. A regression
with otherwise complete custody and an unproved remote display returned
complete=true on the prior source, then passed after the repair.

The new remote_view/display_owner module compares the expected route account UID
with SO_PEERCRED from the actual X server socket. It probes the abstract socket
and filesystem socket without an X authentication grant, using nonblocking
connections. Inaccessible, conflicting or ambiguous peers do not establish
ownership. This also works when the service's private temporary namespace hides
the filesystem socket. A provider-free abstract socket fixture proves matching
UID success and rejection of a different UID despite socket accessibility.

Route-bound launch checks ownership before xdpyinfo, privileged display access
or Chrome launch. Preflight reports the same ownership refusal. Diagnostics add
displayOwner and require its proof for remote-headed complete attestation;
missing or conflicting evidence adds display_owner to missingProofs. Standalone
private displays require the runtime account's socket ownership; route-bound
allocations cannot fall back to that account when provider identity is missing.
Typed presentation/launch-admission failures direct callers to
repair_route_display_binding and preserve uncertainty about reservation cleanup
until the enclosing operation reports its terminal result.

Validation: the false-complete regression failed before repair with observed
true versus expected false. Two focused display tests passed; all 121 affected
remote-view, diagnostics and failure-recourse tests passed serially. Workspace
Clippy with warnings denied, formatting check and diff check passed. README,
CLI help, repository skill, docs site and source comments describe the new proof.
Private source hashes and validation results are retained at
campaigns/p160/display-owner-repair/source-validation.json.

The installed production candidate is unchanged. No production browser was
launched, no display grants were changed, and the contained misrouted synthetic
handoff was not reopened in this slice. Original-consumer acceptance remains
unproved. Source checks do not establish full display or runtime acceptance.

next_action_or_stop_reason: Build the optimized development candidate and verify
the exact kernel-observed correct-owner and wrong-owner route cases through the
real preflight/launch boundary in isolation. Reconcile production route3 with its
own live X server through the supported provider workflow before another browser
launch. Qualify and install the resulting production candidate under standing
authority, then resume the original consumer and all remaining A1–A4/AX gates.


## Checkpoint 35: live display boundary and private-allocation regression

state_transition: Candidate 621b8db8 passed isolated route-owner admission, but
real remote-headed continuity exposed a private-allocation attestation regression.

acceptance_state: Repair passes focused source checks and rebuilt isolated browser
qualification. Production publication remains pending. A1–A4 and AX remain open.

progress_classification: blocker_reduction

The optimized candidate's full Rust suite passed. The isolated boundary fixture
at campaigns/p160/display-owner-boundary-49RTCg proved matching-owner display
preflight ready, wrong-owner preflight blocked, and actual wrong-owner open denied
before any browser or tab existed. Earlier setup failures remain retained.

The remote-headed fixture a1-terminal-reopen-zgVHUg then failed before continuity:
its own live private X server matched the runtime UID, but complete attestation
was false because the browser carried a valid private display allocation ID.
The initial proof incorrectly required that ID to be absent. The fixture released
its own tab and stopped its host. Exact fixture process inspection found no
remaining owned process to signal.

The correction accepts an allocation only when it names the same browser and
display, is ready and private, and has no provider routes. Any associated remote
route, including one missing from the route pool, prevents fallback to the runtime
account. Kernel socket ownership remains required. A provider-free regression
failed on 621b8db8 with observed false versus expected true, then passed after
repair, also rejecting foreign browser ownership and an unresolved provider route.
All nine focused display-owner and diagnostics tests and workspace Clippy passed.
Logs and hashes are retained at campaigns/p160/display-owner-repair/private-allocation.

next_action_or_stop_reason: Qualify the rebuilt candidate against real private
remote-headed continuity and wrong-owner admission, then reconcile the production
route display and publish under standing authority. The original consumer's own
connection acceptance, physical tab safety, operator journey, installed doctor,
monitor continuity and complete causal error matrix remain required.

Checkpoint 35 live readback: source 789cb780, binary SHA256
`d064563c630f963c1f1427d3f3c6b01f5ad3b33194ff1473172c050ccd6bd1d9`,
passed a1-terminal-reopen-1GGVSX with original authenticated handles surviving
host interruption, terminal close/reopen, synthetic interaction and peer checks.
The same binary passed display-owner-boundary-mLkGHI: correct-owner preflight
ready and wrong-owner preflight/open denied with zero browsers and tabs.
Both fixtures exited successfully. Exact continuity-fixture residue inspection
found zero owned processes to signal; the boundary fixture stopped its own host
and Xvfb unit. Candidate metadata and hashed live ledgers are bound in
campaigns/p160/display-owner-repair/private-allocation/live-validation.json.
Production remains unchanged. The next step is the production release build and
controlled installation, with exact route-display reconciliation before launch.


## Checkpoint 36: display-owner candidate installed with original custody preserved

state_transition: Release candidate 789cb780 qualified and installed; production
route3 canonical configuration and Service projection corrected to its own X server.

acceptance_state: Installed own-tab attestation and causal denial proof pass.
Original-consumer acceptance, operator journey, A3/A4 and full AX remain open.

progress_classification: blocker_reduction

The release build completed successfully. Binary SHA256
`e7d3e8ea4384f0d435ea3d1baa6d26b180cf99002d45829b98bdb55ae5f15253`
passed isolated real remote-headed original-handle continuity and terminal
close/reopen at a1-terminal-reopen-9BXeCb. The same binary passed correct-owner
preflight and wrong-owner preflight/open rejection at display-owner-boundary-xyPalX.
Both fixtures exited successfully; exact continuity-fixture inspection found no
owned process residue. Release provenance binds source 789cb780.

Installed generation is 0.28.0-e7d3e8ea4384-5afafcf688e5. Support manifest SHA256 is
`5afafcf688e533d344fc3509e1772a9e4ef6588403521b80b9459245ba72c29a`.
Controller/provider payloads and units were verified unchanged from the preceding
installation. Controlled host replacement preserved all four original browser
process identities and all 47 original tab-custody records. Rollback retains the
previous generation and configuration backup. The monitor timer remains inactive.

Kernel SO_PEERCRED independently proved production route3's user owns display :14.
The stored :12 binding was corrected in the canonical environment with an unused
route guard. Restart alone did not refresh the retained Service entry, so the
supported authoritative-route-pool reconciliation applied the single route3 entry.
It reported one update and no active conflicts. Readback shows :14 and available;
original browser and tab custody remained intact after reconciliation. The other
two production route accounts have no live X server, so their stale display hints
are not ownership evidence. Their recovery remains an operational requirement.

Installed authenticated HTTP smoke acquired two own blank tabs on the existing
Default browser, obtained complete attestation, and evaluated both successfully.
Cross-client use was denied with profile_child_subject_mismatch and no_effect.
Exactly one journal occurrence joined the returned request ID to matching binary,
source revision, support generation/hash and child-access evidence. Both own test
tabs were released. Shared installed skill guidance now includes display-owner
proof; its previous contents are backed up.

Install doctor exited1. The 12 findings concern authenticated operator-journey
readiness, selected-generation readiness, retained transaction history, seven
legacy lease observations, stale monitor state and unknown resource ownership.
These findings remain retained and do not constitute full operational acceptance.
The misrouted synthetic handoff has not been reopened or replaced; no payment,
remote input, original consumer capability use or development cleanup occurred.

Private publication receipts, hashes, activation rollback, canonical environment
backup, route projection, installed smoke and doctor output are retained at
campaigns/p160/publication-789cb780. installed-validation.json records the bounded
result and explicitly leaves operationalAcceptanceComplete false.

next_action_or_stop_reason: Prove the installed route3 preflight and complete the
synthetic operator journey on positively owned production resources, preserving
the failed prior fixture and its continuity evidence. Complete original-consumer
acceptance on its own connection, repair focus/iframe readiness and remaining
provider readiness, then finish A3/A4 and the full causal-error matrix.


## Checkpoint 37: installed namespace boundary rejects the correct route owner

state_transition: Installed operator preflight and launch reveal distinct stale
allocation and user-namespace proof failures. No new browser was launched.

acceptance_state: A2 remains failed; A1 display proof needs namespace correction.
The previous headless/private-display installed checks remain bounded evidence.

progress_classification: blocker_reduction

A new synthetic profile p160-operator-journey-r2 was registered, preserving the
prior misrouted fixture. Initial explicit route3 preflight failed with
route_pool_target_mismatch because the orphaned original display allocation still
retains :12 while the corrected route-pool entry targets :14. Request ID
http-service-remote-view-route-preflight-1913dd34-6b29-4dd8-b470-b85d4d1b95f6
returned generic service_operation_failed/unknown classification. Inactive
allocation reconciliation and causal classification remain product defects to
resolve, not acceptance passes.

The supported explicit displayAllocationId selected a new allocation
p160-operator-display-r2 for the new fixture. Preflight reported preflight_ready
and dry-run succeeded on :14. This permits a distinct new fixture without
rewriting the old failed allocation; it does not repair automatic stale-allocation
recovery or establish continuity for the old fixture.

Actual open failed with route_display_owner_mismatch. Outside user isolation,
SO_PEERCRED observed Xorg PID60951 as UID1010, matching the route account. Inside
the installed runtime host, the same socket peer PID was UID65534. The host's
uid_map contains only 1000 to1000, length1. Other host users are unmapped there;
comparing the namespace-visible overflow UID with host account UID1010 therefore
produces a false mismatch. Earlier isolated fixtures did not exercise this
production user-namespace boundary.

Request http-service-request-remote_view_open-8abf61ae-b19a-474c-9257-36db5ac3b6c1
retains typed launch-admission failure and display-owner evidence. Compensation
reported skipped_before_browser_launch and rolled_back. Fresh readback found zero
new-fixture browsers and route3 available. No viewer, screenshot or remote-input
attempt followed the failed launch. Private evidence and guarded drivers are under
campaigns/p160/a2-operator-journey-r2 and /tmp/p160-a2-r2-*.mjs; the live viewer
driver has not run. The earlier fixture remains failed evidence.

next_action_or_stop_reason: Extend display-owner observation with a bounded
read-only namespace-neutral probe, following the existing process-observer
transport pattern. Bind the returned socket peer to the locally observed PID and
stable process instance; never accept UID65534 as the route user. Add installed
PrivateUsers coverage proving the correct host route user passes while a different
host user fails. Preserve runtime isolation. Then qualify/publish the correction,
repair automatic inactive-allocation reconciliation, and resume the complete
operator journey, original-consumer acceptance, A3/A4 and AX.


## Checkpoint 38: namespace-neutral display-owner source repair

state_transition: Implemented bounded namespace-neutral X socket ownership proof
for host route UIDs not represented by the installed runtime's uid_map.

acceptance_state: Eleven focused tests and workspace Clippy pass. Real namespace
qualification and publication remain pending; A1–A4 and AX remain open.

progress_classification: blocker_reduction

The display-owner module now delegates only when the expected host UID lacks an
identity mapping. A generated user-manager unit runs the same executable with
PrivateUsers and PrivateTmp disabled for a read-only socket observation. The
calling runtime retains its isolation. The helper has a two-second runtime limit,
one-second stop limit, four-second parent deadline and 8192-byte output bound.
It grants no display access and launches no browser. Its host-only mode rejects
an unmapped UID instead of recursively delegating.

Acceptance requires the same display and expected UID, matching local/delegated
socket peer PIDs and process start ticks, and unchanged local process instances
after the helper returns. Different visibility of abstract/filesystem socket
addresses is accepted only for the same server instances. Changed or absent
process evidence remains unproved. A delegated negative owner result remains
negative; overflow UID65534 is never interpreted as the configured route account.
Failures report namespaceObservationError while successful delegated observations
identify their transport and retain the namespace-local peer evidence.

Focused tests cover UID mapping, exact instance agreement, negative result
preservation, duplicate socket spellings and changed-instance rejection alongside
existing ownership and attestation checks. README, CLI help, repository skill,
docs site and inline source describe the behavior. Private source validation is
retained at campaigns/p160/display-namespace-repair. The installed production
binary has not changed in this slice.

next_action_or_stop_reason: Build the optimized candidate and run the prepared
read-only /tmp/p160-display-namespace-live.py probe with PrivateUsers/PrivateTmp,
proving correct host-owner acceptance and wrong-owner rejection on the exact
production X socket without browser/provider mutation. Then complete candidate
continuity qualification, production publication, stale-allocation repair and the
remaining operator, original-consumer, doctor, monitor and AX gates.


## Checkpoint 39: real namespace and continuity qualification pass

state_transition: Optimized namespace-aware candidate passes the previously
missing production-style isolation boundary and real browser continuity.

acceptance_state: Candidate qualification passes; release build and installation
are pending. A1–A4 and AX remain open on the final production candidate.

progress_classification: blocker_reduction

Source 8e0fd8b8 produced optimized binary SHA256
`2459a5fc10c749d4af2b829c44a0746924b61be1a23ec4e335bf6263196da894`.
The read-only display-namespace-f333ca91 probe ran this executable in generated
user units with PrivateUsers=true and PrivateTmp=true. It observed production
route3's X socket only: no browser, provider session or display grant was created.
The local namespace saw overflow UID65534; delegated observation matched the same
PID/start instance and correctly accepted host UID1010. A separate expected-root
observation correctly returned route_display_owner_mismatch for that same server.
Both probe units exited successfully. Isolation remained enabled on the callers.

The same binary passed real remote-headed a1-terminal-reopen-MBgXpp: original
independently authenticated handles survived host interruption, and terminal
close/reopen plus synthetic interaction passed. The host exited successfully;
exact fixture residue inspection found zero owned processes to signal.
Hashed namespace and continuity evidence is bound in
campaigns/p160/display-namespace-repair/optimized-validation.json.

The production release build is running as the existing admitted Cargo invocation.
Publication preparation at campaigns/p160/publication-8e0fd8b8 preserves the prior
generation and binds all 47 unchanged original tab-custody records. Production
still selects the preceding e7d3e8ea4384 candidate. No acceptance receipt was
rewritten and the monitor timer remains inactive.

next_action_or_stop_reason: Resume the existing release build, qualify its exact
binary through the namespace and continuity probes, then stage and install with
original browser/custody preservation. Resume the existing empty r2 fixture using
its explicit allocation only after installed namespace proof passes, retaining
both prior failures. Automatic inactive-allocation recovery, complete operator
journey, original-consumer acceptance, doctor/monitor readiness and AX remain due.


## Checkpoint 40: namespace repair installed; display grant boundary diagnosed

state_transition: Namespace-aware release installed and proved through the
previous ownership gate. Operator launch now reaches a separate privileged grant
failure, with no browser launch and completed reservation rollback.

acceptance_state: Installed own-tab checks pass; A2 and full A1/A3/A4/AX remain open.

progress_classification: blocker_reduction

Source 8e0fd8b8 release binary SHA256
`e00cc6254ed18f85db4d4ebd108d6914ac59c92158e022a3a5820f8b086c8104`
passed namespace probe display-namespace-17918018, real remote-headed continuity
and terminal reopen a1-terminal-reopen-urUrss, and actual wrong-owner launch denial
at display-owner-boundary-dqREg1. All exited successfully. Fixture residue was zero.

Selected generation is 0.28.0-e00cc6254ed1-5ad5a9320e6f with support manifest SHA256
`5ad5a9320e6f1d4f9baee0c2bf20222008c718e26de8d1a523d8a4c7855f0973`.
Controlled replacement preserved all four original browser process identities and
47 original tab-custody records. Installed authenticated own-tab smoke passed two
complete attestations, successful evaluation, cross-client denial and exactly one
build/source/support-bound causal journal occurrence. Both own test tabs were
released. Shared skill namespace guidance was synchronized with backup. Doctor
still exited1; its remaining findings are retained, not converted into acceptance.

The empty r2 profile was reused for one changed-binary operator attempt in
campaigns/p160/a2-operator-journey-r2/namespace-repair-attempt. Preflight and dry-run
passed. Actual open passed display-owner proof but failed display_access_grant_failed
before launching Chrome. Request
http-service-request-remote_view_open-d511875a-6f1e-44a9-a7d8-a5af7b7c0962
reported skipped_before_browser_launch and rolled_back. Fresh readback found zero
r2 browsers, route3 available, and original browser/tab custody unchanged. No
viewer or remote-input attempt followed.

Source inspection found that the grant path invokes sudo directly inside the
runtime sandbox, with stdout and stderr discarded. A generated read-only sudo
listing probe under NoNewPrivileges and PrivateUsers reproduced exit1: root-owned
sudo.conf appears as UID65534, and NoNewPrivileges prevents root elevation.
The existing formatter accepts stderr but its caller passes an empty string,
hiding the actionable cause. This probe did not attempt a display grant.
Private installed and diagnostic evidence is retained at
campaigns/p160/publication-8e0fd8b8.

next_action_or_stop_reason: Repair the privileged display-grant execution boundary
using the existing restricted grant helper in a separate bounded supervised
operation, while keeping runtime isolation enabled. Retain bounded causal stderr
and namespace/elevation evidence. Qualify the real grant path and its failure
recourse, then resume operator acceptance and all remaining A1–A4/AX requirements.
Automatic inactive-allocation recovery remains separately unresolved.


## Checkpoint 41: supervised display grant repair and bounded causal errors

state_transition: Implemented supervised restricted-helper grant transport and
proved its real display access effect while the caller retains isolation.

acceptance_state: Source checks and scoped operational grant proof pass; candidate
build/publication and complete operator journey remain pending. A1–A4/AX remain open.

progress_classification: blocker_reduction

Linux display grants now run the existing sudoers-restricted privileged helper in
a generated user-manager unit outside the caller's user/temp namespace and
NoNewPrivileges restriction. The browser runtime stays isolated. The grant keeps
its two-second timeout, the supervised unit has a four-second lifetime plus
one-second stop limit, and an outer six-second timeout bounds the transport.
The unit uses control-group termination. No sudoers policy or root helper
capability changed; existing exact user/display validation remains in force.

Grant stderr is retained in the returned error, stripped of unsafe control
characters and limited to 1024 characters. Classification preserves
 display_access_grant_failed or display_access_grant_timeout at launch admission
with inspect_privileged_display_grant recourse, ahead of generic reservation
rollback classification. Grant effects remain uncertain on failure because an X
access grant may finish before a timeout even when no browser was launched.

All 118 selected remote-view and failure-classification tests passed. The added
checks cover bounded diagnostics and retention of the original grant cause and
uncertainty despite browser rollback metadata. Workspace Clippy and format check
passed; all five user-facing/source documentation surfaces were updated.

A nested read-only status probe first demonstrated that the restricted helper can
run through this supervised boundary while its caller has PrivateUsers, PrivateTmp
and NoNewPrivileges enabled. A subsequent guarded operational probe invoked the
same command recipe for production route3 only, after fresh kernel UID/PID/start
proof for its X server and an available-route check. The exact grant to operator
ecochran76 on :14 succeeded. A fresh equally isolated xdpyinfo probe then succeeded
on :14. The X-server process instance was preserved. These probes did not launch
a browser, change a development display, or disable runtime isolation. This is a
scoped environment repair plus transport proof, not yet a native-candidate cold
grant-path acceptance claim. Installed production binary is unchanged in this slice.

Private command intent, stdout/stderr, readback and hashes are retained at
campaigns/p160/display-grant-repair/validation.json. No prior failed attempt or
acceptance receipt was overwritten.

next_action_or_stop_reason: Build and qualify the native candidate, retain the
supervised grant proof, and publish with original custody preservation. The now
repaired production display permits a separately recorded operator attempt using
the existing empty r2 profile. Complete that journey, automatic inactive-allocation
recovery, original-consumer acceptance, doctor/monitor readiness and the full AX
matrix on the final accepted installed candidate.

## Checkpoint 42: local operator acceptance and native grant failure proof

state_transition: Verified the local synthetic operator journey after the scoped
display grant and qualified the optimized supervised-grant candidate.

acceptance_state: Local operator evidence passes on the installed 8e0fd8b8
generation. Candidate be6748fd remains unpublished; A1–A4/AX remain open.

progress_classification: blocker_reduction

The r2 access-grant-attempt opened the intended synthetic browser with complete
attestation. Two authenticated local viewers saw the synthetic marker, trusted
mouse and keyboard input changed its pixels, and reconnect reused the same
durable handoff while the other viewer remained visible. An anonymous viewer
was denied. Independent fixture readback recorded exactly one trusted mouse and
one keyboard event. The server browser and durable handoff remain available for
continuity validation; the disposable client browser closed.

Private evidence is retained under
campaigns/p160/a2-operator-journey-r2/access-grant-attempt, including
local-viewers-g7IBrz/events.json and trusted-input-readback.json. The preceding
local-viewers-51BHR6 attempt stopped before remote input because its baseline
crop mixed white and blue after the frame moved. The private driver now requires
a fully blue crop and stable frame geometry before accepting that baseline.
Both attempts remain inspectable. Local acceptance does not satisfy the protected
external-vantage gate or final-candidate acceptance.

Optimized candidate be6748fd85bb77f0c4977bcefc0c5071ba4ba8e0, binary SHA256
f380956ce4d1397ff53c3d54c527ccf54564b77616b014f4212e1f4da45d003d,
passed the namespace probe and remote-headed terminal-reopen fixture. The latter
retained the existing capability, rejected the old handle, proved the old process
absent, exercised the new target and preserved the peer. Exact fixture cleanup
inspection found no remaining owned process to signal.

The native grant failure fixture display-grant-boundary-5dWekv used its own
authenticated Xvfb display. Display ownership passed, then the restricted helper
rejected the intentionally unsupported route username. The response preserved
display_access_grant_failed at launch_admission, effect_uncertain, and the helper's
specific username rejection instead of replacing it with rollback metadata.
No browser or tab was created. The fixture host and Xvfb stopped successfully.
This proves native failure transport and first-cause preservation; the positive
grant effect remains the separate scoped operational proof in Checkpoint41.

next_action_or_stop_reason: Finish the existing release build, qualify and publish
that exact binary with original-browser custody preserved, and complete consumer
identity acceptance, inactive-allocation repair, readiness and scheduled-cycle
requirements. No production replacement occurred for this checkpoint.

## Checkpoint 43: qualified grant candidate installed with original custody

state_transition: Completed release qualification and controlled production host
replacement with the supervised display-grant repair.

acceptance_state: Installed own-tab identity and causal denial checks pass;
doctor exits1. Final A1–A4/AX acceptance remains open.

progress_classification: blocker_reduction

The release build completed successfully in 9m31s and embeds source
be6748fd85bb77f0c4977bcefc0c5071ba4ba8e0. Its archived binary SHA256 is
f0a1edfb7b808a1b54429656ade24b75d278968b1a5209c70f8bcf4a8d4e5043.
Exact-release namespace proof, remote-headed original-client continuity and
terminal reopen, and restricted-grant failure transport all passed. The
continuity fixture left no owned process residue. Evidence hashes are bound in
campaigns/p160/publication-be6748fd/qualification-status.json.

Production now selects generation 0.28.0-f0a1edfb7b80-879b8a263f21, support hash
879b8a263f217f76223b2316bbb15b383ee41c2e53904513c9bf5242e972e4f9.
Verified controller/provider assets and unit payloads were reused unchanged.
Controlled host replacement preserved all five live browser process identities,
including the retained synthetic operator browser. All 47 original tab-custody
records remained unchanged. The activation receipt binds observed host PID92837,
socket identity, ingress and executable generation; it is not a readiness receipt.

Installed own-tab smoke opened two authorized blank tabs in the retained Default
browser, obtained complete attestations and evaluated both successfully. A
cross-client operation was denied before effects, and exactly one causal journal
occurrence matched the installed binary, source and support identity. Both own
test tabs were released. Supported doctor still exited1; its full current result
is retained in the publication directory as doctor-after.json.

next_action_or_stop_reason: Revalidate the retained operator handoff on this
installed candidate, synchronize installed guidance, resolve current consumer
identity and automatic allocation blockers, and complete doctor/monitor and
scheduled-cycle acceptance. Preserve the historical failures and original custody.

## Checkpoint 44: installed retained-browser proof and visual-oracle failure

state_transition: Refreshed installed consumer lease recourse and exercised the
retained operator browser after host replacement.

acceptance_state: Retained synthetic attestation and input delivery pass; the
complete current-candidate viewer oracle did not pass. A1–A4/AX remain open.

progress_classification: blocker_reduction

Read-only installed lease explanation still offers rejoin_owned_browser for the
consumer's existing capability and identifies runtime_owner_principal_binding_missing.
The named-profile mismatch reported earlier is no longer the observed blocker.
No consumer capability was read or used, and no consumer binding was changed.
Original-connection acceptance remains required. Current readback is retained at
publication-be6748fd/consumer-lease-explain-current.json. The preceding malformed
read-only CLI argument attempt is retained separately; it performed no mutation.

The retained synthetic handle obtained complete attestation on the newly installed
binary, and its original input counters remained one each before the viewer test.
The installed shared skill received the exact grant-diagnosis paragraph with a
backup and before/after hashes in the publication directory.

Current-candidate viewer attempt local-viewers-47eJUu resolved the same durable
handoff for both authenticated viewers, displayed the synthetic browser, and
denied the anonymous viewer. Its mouse pixel acknowledgment passed, but its
keyboard pixel hash timed out before the final reconnect checks. Both client
contexts closed. This attempt remains failed; no unchanged retry followed.

Subsequent exact-target readback retained complete attestation and counted two
trusted mouse events and two keyboard events, proving that both new inputs were
delivered. Direct inspection of the failed crop showed blue marker pixels and
white lettering instead of the required solid-blue baseline. The full synthetic
page screenshot also showed the blue marker. This is a visual-oracle discrepancy,
not evidence that keyboard delivery failed; the precise geometry/focus transition
still needs diagnosis before another attempt. The second viewer independently
displayed profile_child_subject_mismatch for tab_control_own despite a ready
viewport. That authorization failure remains an A1/A2/AX defect to disposition.

next_action_or_stop_reason: Fix the dashboard operator focus/child-authorization
path and diagnose the unstable synthetic sample before repeating the installed
journey. Preserve original browser/target identity and all failed evidence.
Complete consumer acceptance, allocation recovery, doctor and scheduled cycles.

## Checkpoint 45: operator focus authorization seam established

state_transition: Traced the observed dashboard subject mismatch to the automatic
focus request and the daemon's child-access decision.

acceptance_state: Diagnosis established; operator-focus repair and its regression
proof are pending. A1–A4/AX remain open.

progress_classification: blocker_reduction

workspace-remote-viewport.tsx queues view_focus in control mode when the selected
browser projection says canControl. That projection is not the current viewer's
controller lease. The request uses dashboard service/agent/task attribution, while
cdp_free_execute.rs::authorize_profile_child_access requires TabControlOwn for
view_focus and compares that subject with the existing tab child. Both identities
are legitimate but represent different authority. Changing the caller labels to
the tab owner would conceal this mismatch and is not an acceptable repair.

The HTTP broker already authenticates dashboard credentials separately, but only
uses that identity as fallback request attribution when caller labels are absent.
The actual focus authorization therefore has no distinct authenticated operator
proof. Existing browser reattach performs route recovery and checkout effects;
it is not a suitable replacement for ordinary tab focus. The native focus handler
also permits index fallback after an explicit target fails, which must not be
used to establish exact operator target selection.

Bounded repair contract for this A1/A2/AX seam:

- Preserve transport-authenticated operator identity separately from caller
  labels. Public request fields cannot manufacture or override that proof.
- Permit operator focus only after current authorization for the exact browser,
  profile, target and controller/viewer role is established. An observer must not
  automatically issue a control operation merely because the browser supports it.
- Keep the tab's existing child owner and agent capability binding unchanged.
  Ordinary agent control continues to require the current child-access contract.
- Reject a missing or changed explicit target before focus; never substitute an
  index. Return a typed causal failure identifying the failed authority seam.
- Verify an authenticated authorized controller can focus an agent-owned target,
  while anonymous, forged metadata, observer-only, expired and mismatched
  browser/target cases are refused before effects. Include a two-viewer installed
  check and before/after child-owner evidence.

next_action_or_stop_reason: Implement this bounded operator authority path through
the existing view_focus transport and execution seams, with regression proof.
Then repair the independent synthetic sampling instability and resume final
installed acceptance. No browser, lease or production binary changed here.

Implementation follow-up: the archived installed binary rejected an isolated
missing-target-plus-index request at child admission with service_tab_target_unproven
and no_effect. This qualifies the existing public admission guard, not the native
handler's later race. Private fixture focus-exact-target-E04kod retained its
request/response and completed cleanup with no owned residue. A preceding driver
syntax error occurred before any fixture execution and was corrected before use.
The native handler now propagates exact-target switch failure rather than falling
back to an index if the target disappears after admission. All five documentation
surfaces reflect exact selection. Formatting passed; Clippy and focused view_focus
tests are running. The operator-authority path remains unimplemented, and this
source change is not installed.

The eight existing view_focus tests and workspace Clippy passed for the exact
target change. The operator proof primitive is now implemented under
stream/dashboard_auth/operator_focus.rs and is compiled only in tests until its
admission integration is ready. It signs a purpose-separated, 30-second binding
to request ID, browser, session, profile, target and browser PID with the existing
dashboard secret. Verification rechecks the current account's superuser role,
signature, full binding and time bounds. The focused test covers changed fields,
tampering, future/expired proof, unknown user, observer issuance and role revocation.
That test is running; no proof is issued or accepted by production code yet.
Integration must still bind current profile policy and operator/controller
permissions, strip caller-authored proof fields at transport normalization, avoid
persisting the transient token, preserve child ownership, and qualify the complete
operator path. Passing the primitive alone cannot satisfy operator acceptance.

The signed-proof test passed. The pending admission implementation now also
recomputes profile view permission using AuthenticatedIngress assurance and checks
the exact ready browser/target/session, PID, route/display binding, controller
lease, controller epoch, authenticated dashboard viewer identity and parsed lease
expiry. It takes an immutable state reference and does not rebind child ownership.
A focused state test covers policy denial, controller/target drift, observer and
released leases, invalid expiry and unchanged original state. Its first compile
found that this workspace enables formatting but not parsing in the time crate;
the implementation now uses the existing chrono RFC3339 parser. Both corrected
operator-focus tests passed. This code remains test-only until HTTP, daemon and dashboard
integration is complete. Existing viewer issuance accepts caller viewerId labels;
integration must derive the dashboard viewer identity from authenticated ingress,
and the UI must focus only after its own explicit controller acquisition succeeds.


Integration follow-up: operator focus is now wired through authenticated HTTP
issuance, daemon admission, and the native focus handler. The handler holds the
route controller fence while revalidating the signed binding and focusing the
exact target. Dashboard viewer identity is derived from authenticated ingress;
the RDP workspace waits for its own explicit controller acquisition before
requesting operator focus. These changes remain uncommitted and uninstalled.
The earlier test-only descriptions above record intermediate states.

The first integrated stream selection finished with 217 passes and one failure:
a routing-only test injected the fictional authenticated account test-adapter,
which the new current-superuser check rejected. That routing test now uses an
attributed service request without pretending to authenticate a dashboard
account. The full stream selection is running again, including the added forged
operator-proof rejection and typed no-effect recourse test. Formatting was
corrected after its check found two unformatted assertions; Clippy is running.

Dashboard durable-handoff and authentication-status checks passed. The workspace
view-projection check failed at scripts/test-dashboard-workspace-view-projection.js:283:
its loopback /route-b fixture expects a public URL, but the existing helper only
rebases /guacamole paths and returns null. The test, projection and URL-helper
files are unchanged from HEAD. This failure needs a separate disposition; it is
not a successful frontend regression run. No installed acceptance is claimed.


Verification readback: the second stream run finished with 218 passes and one
routing-fixture failure because removing fictional authentication exposed the
fixture's missing agent/task attribution. After supplying all three attribution
fields, that exact routing regression passed (one selected test, zero failures).
The forged operator-proof HTTP regression passed in the 218-test result. Workspace
Clippy passed before the final test-data-only edit, and git diff --check passed.
The two failed selections remain recorded rather than being described as a clean
full-suite pass. Positive HTTP-to-daemon operator integration, token-retention
inspection, isolated real-browser qualification and installed two-viewer acceptance
remain required before publication. A1–A4/AX remain open.


Execution-fence correction: review found that the pending operator-focus handler
used begin_service_controller_mutation, which cancels existing desktop claims
before its final proof check. That contradicts no-effect rejection. Focus now
uses a non-cancelling interaction claim and event guard together with the same
external route fence. It verifies proof before acquisition and again under the
fence; rejection drops only its own guards. Competing desktop interaction is
refused without cancelling the original claim. Controller takeover retains its
existing mutation semantics. A focused coordinator regression exercises retained
claim usability and local/external fence release. The first compile failed on
missing test imports; those were corrected and the selection is running.
Workspace Clippy passed for the corrected production path. This source is not
installed and does not establish final operator or consumer acceptance.


The corrected coordinator selection passed all five tests, including rejected
focus preserving the original interaction and releasing its own external fence.
Formatting check passed. The source checkpoint includes the authenticated
operator-focus integration and the non-cancelling execution guard; it is a
qualification checkpoint, not production publication or completed A1/A2 evidence.
Next: run positive authenticated HTTP-to-daemon focus against an isolated browser,
verify no transient proof appears in retained records, then qualify the complete
candidate and its dashboard assets before controlled production replacement.


Candidate and integration readback: optimized build 708bfd4b passed and was
archived with its source commit and binary SHA256 under the private P160 campaign
candidate-708bfd4b directory. It has not been installed. A subprocess-isolated
regression now passes authenticated request construction through the same proof
verifier used by daemon admission. It preserves the original tab owner, strips
caller-authored proof, rejects changed request/browser/session/target/route/lease
and controller epoch, and excludes the signed token from retained provenance.
This uses modeled service state, not a real browser or provider, and therefore
does not satisfy installed A1/A2 acceptance.

Follow-up AX repair: successful focus now stamps the authenticated dashboard
account and authenticated-ingress assurance into command provenance separately
from caller service/agent/task labels. The regression now checks that distinction;
the updated isolated regression passed, as did formatting and diff checks.
This source change is newer than archived 708bfd4b
and requires a newly bound final candidate before production acceptance.


Real-browser qualification: candidate 09fd769f passed operator-focus-live-lsrOHb
using a disposable headed Chrome, private Xvfb display and window manager,
registered original agent capability, supported route checkout, and an isolated
dashboard account. Controller acquisition derived its viewer identity from the
account rather than the supplied label. Operator focus succeeded; original tab
principal/session custody remained unchanged, and the original agent still
passed evaluation and complete control-plane attestation afterward. Retained
Service State contained no operatorFocusProofToken field. The trace recorded
clientSubjectId dashboard:admin with authenticated-ingress assurance. This
qualifies real browser focus, not Guacamole transport or production acceptance.

The preceding operator-focus-live-oBdcxI attempt stopped after browser/route
setup because the fixture expected an auth credential file before initializing
dashboard auth. The fixture now invokes the supported auth-status endpoint
first. Both fixtures completed cleanup with zero remaining owned processes.
The first failure remains retained. No production browser was touched.

The dashboard production build passed. Full release compilation of 09fd769f is
running after that build, so its embedded dashboard will contain the changed UI.
External controller/provider assets and unit templates are unchanged from
be6748fd; publication helpers and asset hashes are prepared under private
publication-09fd769f. Production readback still showed five live browsers, zero
active jobs, and nonzero doctor. Selection remains unchanged pending release
qualification and fresh preservation checks.

The pending two-viewer driver now waits for the successful operator-focus
response after takeover and maps its synthetic pixel sample into native desktop
coordinates using the existing geometry helper. It chooses an interior uniform
patch. These remove focus-completion and iframe-scaling ambiguities; the previous
crop shift's exact cause is not claimed as proven. No new viewer attempt has run.


Production activation 09fd769f: the exact release binary passed operator focus,
headless two-client interruption/terminal-reopen, and remote-headed equivalents.
All fixtures completed with zero owned residue. Generation
0.28.0-00a9aac42f64-c367eed6e549 is now selected; activation preserved all five
browser processes and 68 tab-custody records. Installed binary and dashboard
focus-asset hashes matched; retained synthetic attestation remained complete and
its two prior mouse/keyboard events were unchanged. Shared guidance was synced.
Doctor still exits 1. The interlock timer remains inactive. A1–A4/AX remain open.

Two-viewer attempt local-viewers-pjKrTk reached the correct Ready browser but its
new 40-pixel uniform patch exceeded the scaled marker height. No input was sent;
the client closed. After correcting that sampling-size error, local-viewers-ssDlh0
verified both viewers' pixels but received profile_child_subject_mismatch during
operator focus, before input. Its dashboard-view-focus request ID and preserved
self-declared backend attribution identify the legacy dashboard focus adapter.
That adapter rebuilt the command while dropping operator authority fields. The
isolated prior fixture exercised the lane HTTP broker, not this dashboard branch.

The pending fix bypasses the legacy adapter for operatorFocus requests so the
existing authenticated canonical dashboard path handles them. Its regression now
checks that routing choice and uses the dashboard generation-aware normalizer.
The current ordinary controller lease also has expiresAt null. The pending proof
verifier now honors that existing active-until-release/replacement contract;
explicit invalid or expired lease dates remain denied, and the signed request
still expires after 30 seconds. This follow-up is source-only; a full stream
selection is running. The first intermediary routing regression passed before
simplifying the fix to use the existing canonical fallback. No passing installed
operator journey or post-fix browser test is claimed yet.


The final focused stream selection passed all 220 tests, including authenticated
operator routing and controller lease expiry cases. Formatting and workspace
Clippy passed. The standalone-dashboard fixture operator-dashboard-focus-8IDXUE
reproduced the installed failure before the fix: controller acquisition succeeded,
but focus arrived as self-declared and failed profile_child_subject_mismatch.
The original client retained custody and released its exact target; dashboard,
host, display and window manager exited, with zero owned process residue.
The preceding 60W88o fixture failed before focus because its dashboard environment
omitted shared-host relay mode. That setup error was corrected and retained.
A new optimized candidate is building for the identical standalone-dashboard
fixture. No post-fix installed acceptance is claimed.


Production follow-up 5e0abfb1: the exact release binary passed the standalone
dashboard operator-focus fixture and both headless and remote-headed original
handle interruption/terminal-reopen fixtures. All three left zero owned residue.
Generation 0.28.0-a7fd467a5548-5d9a9891ddf6 is selected, with all five live browser
identities and 68 tab-custody projections preserved. Installed embedded dashboard
assets matched and shared guidance was synced. Doctor remains nonzero.

The first installed attempt, local-viewers-GHmvpT, failed before input because
the durable-handoff dashboard shortcut used a retained HTTP stream port without
preparing the owner lane after the host restart. Journal occurrence IDs
90266e16-a458-4ae4-b45e-6f4d0aad00c0 and b056b906-e0b2-4945-a9eb-8703aefd4a31
bind backend_unavailable at connect to this build. The returned fixture projection
lost the gateway error detail; later attempts now retain that detail. A supported
diagnostic read of the original synthetic handle restored its lane and proved
complete attestation with unchanged input counters. This is a warm-lane workaround,
not cold-start acceptance. The pending source correction prepares the exact lane
before the shortcut sends its request. Stream regression and Clippy are running.

Warm-lane attempt local-viewers-aoKCek resolved the correct browser and target but
could not see the blue marker: the page viewport was 142 pixels high and the
marker's top was 159 pixels below its top. No input occurred. The adjusted attempt
local-viewers-bBbB2V resolved both authenticated viewers, denied anonymous access,
and successfully completed operator focus with dashboard:admin and authenticated-
ingress provenance. Mouse input produced the white acknowledgement. The final
keyboard crop did not match its solid-blue baseline, so the test remains failed.
Readback of the same original handle proved both trusted counters increased from
two to three and attestation remained complete. The failed keyboard crop contains
blue plus a small part of marker lettering; the page's scroll position changed to
33 and its viewport became 462 pixels high. That supports a moving-sample diagnosis
but does not establish a passing fixed-region pixel protocol. Both viewers closed.
No additional input retry has run. Reconnect and full A2 acceptance remain open.

Read-only pressure attribution is retained in publication-5e0abfb1. Nineteen
observed stock-Chrome browser roots include other application profiles, retained
P158/P159/P160 campaign fixtures and the separately preserved P158 development
runtime. Several Chrome processes expose a flattened command line and scrubbed
HOME rather than ordinary argv/environment fields. Profile arguments and cgroups
provide attribution evidence, not cleanup authority. No process was signaled and
no blanket doctor exemption was applied. A1, A2, A3, A4 and AX remain open.

The cold-handoff follow-up passed the focused stream selection: 220 tests, zero
failures, 43.14 seconds after compilation. Final help-text formatting and
workspace Clippy readback passed. This coverage includes
the existing live-listener readiness predicate and authenticated handoff contracts,
but does not yet prove the newly connected shortcut on a cold real browser lane.
That installed regression remains an explicit next gate.


Installed warm-lane A2 readback now passes in local-viewers-vXO6zP. The driver
chooses a fixed solid-blue controller crop with 40 pixels of vertical blue margin,
excluding marker lettering and accommodating the previously observed 33-pixel
focus scroll. The existing fixed-region hash protocol was unchanged. Both
concurrent authenticated viewers showed synthetic pixels, anonymous access was
denied, trusted mouse and keyboard acknowledgements passed, and the original
durable URL reconnected while the peer remained usable. Subsequent original-
handle diagnostics remained complete and both trusted counters were four. This
proves the installed warm journey, not cold host recovery or P158 external vantage.

Cold routing regression: operator-cold-handoff-Wi50im failed on installed-source
5e0abfb1 with connection refused at the retained owner HTTP port after a disposable
host interruption. The same fixture passed on optimized candidate 6d4e0f15 in
operator-cold-handoff-pVUdQN. It creates and closes a disposable peer tab through
supported actions, then seeds only an isolated handoff hint referencing that
closed peer while its host is stopped. No owner or permission record is authored.
The dashboard restores the exact listener, returns status closed without reopening
the peer, and preserves the original live browser process and complete original-
handle attestation. This is a routing regression, not a ready-provider handoff.
The earlier e78cwG setup passed the old binary because its closed target sent the
request down the already-correct canonical fallback; it is not red evidence.
All three fixtures left zero owned residue. The full 6d4e0f15 release is building.

Readiness remains incomplete despite the warm A2 pass: the synthetic handoff has
a fresh ready presentation receipt in Service State, while dashboard ingress has
no lastPresentationReceipt for its already-selected backend. Workstation status
still reports selectedGenerationReady false against the old failed-preserved
transaction. These are separate remaining reconciliation gaps. No acceptance
receipt or transaction history was hand-edited. The consumer's own rejoin and
subsequent attestation result has been requested while independent work continues.


Cold installed handoff follow-up on 6d4e0f15: the exact release passed the
isolated cold-listener fixture and both original-handle continuity fixtures.
Generation 0.28.0-a2f1ce276fbf-cc8aafde7920 was activated with all five browser
process identities and 68 tab-custody projections preserved. The activation
helpers have run and must not be replayed from the earlier continuation text.

The first valid retained-handoff journey after activation, local-viewers-bvPpHr,
failed before page input. The listener now starts, but adoption rejects the
existing Ready owner with runtime_handoff_orphan_owner_present. Directed response
readback is retained in publication-6d4e0f15/cold-handoff-response.json under
request http-service-request-service_remote_view_handoff_resolve-324d0831-05c7-47bb-892c-8e5123883a8a.
It incorrectly returns success true, status converging and retryable true. This
is an A1/A2 recovery defect and an AX causal-classification defect. The original
warm journey on 5e0abfb1 does not establish cold recovery on this candidate.

Source review locates the wrong branch: the remote-view runtime adapter always
calls runtime handoff resume; without a transfer descriptor that handler attempts
orphan adoption, which correctly refuses a Ready owner. Original service-tab
recovery already verifies the handle, canonical profile, process, endpoint and
owner fence before attaching the exact retained target. The next repair must
share the appropriate existing-owner verification without weakening orphan or
foreign-owner rejection, replacing tabs, or promoting observation to authority.
The cold-listener fixture references a closed hint and cannot cover this valid
retained-browser branch; add coverage at that actual seam.

The refreshed consumer report at note0156 confirms that registration and a Ready
owner did not prove profile_lease or handoff_receipt. Its named-profile correction
is integrated, but a successful rejoin and complete attestation from the original
consumer remain unverified. Self-identification alone cannot satisfy those proofs.
Keep that consumer acceptance first, together with exact tab release, cold retained
recovery and causal error reporting. Payment tests, joined emails, CSV/browser
checks and final consumer cleanup remain downstream; no business action or
consumer-capability use was performed during this review. A1 through A4 and AX
remain open.


Cold-owner repair candidate: durable resolution now passes the retained handoff
identity to the runtime adapter. For an exact current Ready owner, it shares the
original service-handle profile/process/endpoint/owner fence and attaches only
the recorded target. It preserves the ownership generation; actual orphan and
transfer paths retain their guards. A post-attachment process identity check
also prevents installing a connection after process replacement.

The production-source isolated reproduction operator-cold-ready-handoff-p3PbXn
failed with the same owner-present refusal. It seeds only an isolated handoff
reference to its original live target, without authoring owner or permission
records, and left zero owned residue. The coordinator regression independently
failed because that refusal became Converging. After the error-path correction,
all 29 remote-view coordinator tests and 15 failure-classification tests passed;
workspace Clippy and formatting checks passed. Existing-owner refusal retains
uncertain effects because the old orphan path may already have attached CDP;
the earlier pre-attachment hint guard remains no-effect. Optimized real-browser
qualification and production publication of this repair remain pending.


Optimized cold-owner candidate 4549eecd1abcba545663bf8e685c7ddcd559a22787caf2ec8c4ea9156557519c
passed operator-cold-ready-handoff-0SY9PJ: a cold authenticated dashboard resolve
reattaches the exact original target and preserves the owner registry generation.
The deliberately unproven local display is refused with geometry_not_authorized;
route compensation is verified rolled_back, the closed peer stays closed, and
original-client diagnostics remain complete. This qualifies owner recovery and
refusal preservation, not a ready remote presentation. Headless ztL8Ww and
remote-headed OSVkkH also passed both independently authenticated original-client
interruption, foreign-denial and terminal-reopen checks. All left zero owned
residue. Source and binary binding is retained in candidate-cold-ready-owner.

Retained failed fixture attempts remain part of the evidence: qkxW6K omitted
retained route selectors and reached route_pool_exhausted; aJ0TXL and NSqgey
exhausted the inherited ten-second closed-hint budget. A 45-second disposable
run, 50Vzcp, exposed the actual presentation refusal, geometry_not_authorized,
with route rollback. No product timeout or presentation gate was relaxed. The
final fixture explicitly tests that refusal, original owner continuity and
complete original-client attestation. Old-source CFdNvs still fails earlier at
runtime_handoff_orphan_owner_present. Production A2 must independently pass the
real authorized display, synthetic input, concurrent viewers and durable reconnect.


## Checkpoint 20476d9a: installed cold owner recovery and operator journey

The release build completed in 9m 27s. Production generation
0.28.0-0031818eb0a0-9982d6cba7ee now runs source 20476d9a with binary SHA256
0031818eb0a0e463f2186eeb88dacf3ad90483af0ddffd846e21bb457459286c.
Support SHA256 is 9982d6cba7eedea75eaa20b602a7b217ee08af4ee9705e615efe0417cb64008d.
All five live browser process identities and 68 tab-custody projections survived
controlled activation. Installed embedded dashboard assets and shared guidance
match. Publication helpers have run and must not be replayed.

Exact-release isolated qualifications passed: cold owner recovery/refusal 41QdYN,
headless two-client continuity 4tara8, remote-headed continuity Mmm3Mq, session-only
headless poaDKW and remote-headed JtWyhf. The session-only cases include exact tab
closure, five foreign/ambiguous no-effect denials, retained release, peer control
and complete attestation after interruption. All fixture residue is clear. One
cleanup invocation failed because the default Python lacks PID-descriptor APIs;
the system Python supports them and the subsequent exact-fixture census was clear.
The original cleanup failure is retained separately from test success.

Installed attempt local-viewers-4cwtMJ, before any diagnostic warmup, resolved the
original browser and target for both authenticated viewers. Thus the cold Ready
owner path is verified on production. Operator focus retained authenticated
account provenance. Both trusted counters advanced from four to five and original
handle attestation stayed complete, but the keyboard pixel oracle failed. The
failed sample was white while the full synthetic frame showed a blue marker.
The vertical-only crop selector allowed a sample near the marker's right edge;
the precise failed clip movement was not recorded and is not claimed proven.

The corrected sampler requires 35 pixels of solid-blue clearance on both axes
and records changed sample coordinates/hashes. It leaves the fixed-region input
protocol unchanged and does not alter the remote DOM. Attempt local-viewers-z1j8Ma
passed two simultaneous authenticated viewers, anonymous denial, synthetic pixels,
trusted mouse and keyboard, same-URL reconnect and continued peer usability.
All three acknowledgment samples used the identical 20x20 clip at 388,716; the
baseline and keyboard hashes matched, and mouse was verified white. Original
handle attestation remains complete; both counters are now six. Both viewer
clients closed. This is a pass after a retained failed pixel attempt, not a
first-attempt clean result or P158 external-vantage proof.

Evidence is retained in publication-20476d9a, including qualification-status,
activation-receipt, tab-custody readback, local-operator-acceptance and both viewer
attempts. Doctor still exits 1: the verified selected dashboard lacks its ingress
operator-journey acceptance projection, selected-generation readiness still
compares against older failed-preserved history, consumer principal binding and
other lease findings remain, monitor is stale, and observed-pressure ownership
needs disposition. The original consumer's own rejoin and attestation result is
still pending. A1, A3, A4 and AX remain open; local A2 journey evidence now passes
on this installed candidate. No timer restoration or full-plan acceptance is
claimed.

## Selected dashboard acceptance repair

state_transition: Source repair validated for missing selected-backend journey acceptance.
acceptance_state: A1, A3, A4 and AX remain open; installed acceptance of this repair is pending.
progress_classification: progress

The authenticated durable-handoff ingress hook previously returned without
recording acceptance whenever the requesting dashboard was already selected.
Controlled production activation therefore left operatorJourneyReady false even
after the retained synthetic viewer journey passed. This is distinct from the
older transaction's selected-generation comparison and from consumer ownership.

The hook now derives selected-backend evidence through the same owner, route,
display, target, provider and generation validator used by candidate selection.
It rechecks the selected backend's live manifest under the ingress revision
lock, then updates only its presentation receipt and revision. Repeated identical
evidence is idempotent. Candidate, fallback and rollback custody remain intact;
stale dashboard generations cannot record acceptance. The hook's boolean still
means candidate selection, preserving its existing contract.

Focused dashboard-ingress validation passed all 29 tests. The extended existing
repository fixture covers missing acceptance with another candidate staged,
repeat resolution, preserved complete registry custody, a stale generation, and
refusal after the route becomes orphaned. No red-before-fix execution is claimed.
Workspace Clippy with warnings denied, formatting check, remote-view documentation
check and the docs production build passed. Help, README, Service skill, docs site
and inline comments describe the behavior. No production state or consumer
capability was changed, and the consumer investigation note remains separate.

Next: qualify and publish the acceptance repair, verify the original retained
handoff through the authenticated installed dashboard, and reconcile the separate
selected-generation/history readiness comparison using current installation
proof. Consumer original-connection attestation, remaining doctor findings,
scheduled cycles and the full causal-error matrix remain required.

### Ordinary dashboard startup hook correction

state_transition: Live readback exposed a second acceptance gap; source correction validated.
acceptance_state: Installed acceptance remains pending; A1, A3, A4 and AX remain open.
progress_classification: progress

The optimized 7849e18e build passed in 2m 19s and is archived with SHA256
3fa39956b466b5a4d3817e1aafb8b6a44afa914366b1e814a15efd639d84a11c.
It was not published. Fresh read-only production inspection found that the
ordinary dashboard has no AGENT_BROWSER_DASHBOARD_GENERATION variable. Its ingress
and retained handoff use dashboard-0.28.0, while its manifest executable is the
exact selected 20476d9a production binary. The HTTP hook previously required an
explicit generation variable, so the earlier repository-level acceptance repair
alone would not run on this actual production path.

The ordinary HTTP path now resolves its generation only when its own runtime
manifest hash equals the selected ingress backend's manifest hash. An explicit
shadow generation keeps the existing path. A stale process cannot borrow the
selected generation by reading the registry. The existing ingress fixture now
also proves matching-manifest resolution and stale-manifest denial. The 29
focused ingress tests, Clippy with warnings denied, remote-view documentation
check and docs production build passed after this correction. No red-before-fix
execution is claimed. All five documentation/help surfaces were updated.

Private readback is under selected-dashboard-acceptance. The initial exploratory
comparison treated the boot-qualified process start token as a bare kernel
counter; the corrected comparison confirms exact host PID, boot and start-token
identity. The dashboard's generic generation label genuinely differs from the
installation generation, so the remaining selected-generation readiness repair
must join exact binary/manifest evidence instead of assuming those labels match.
Production and the consumer's capability remain untouched. Next: qualify this
combined acceptance repair on the installed runtime, then continue current-state
readiness reconciliation without discarding historical failure evidence.

## Installed c249c531 acceptance checkpoint

state_transition: Installed authenticated journey acceptance is now projected into doctor.
acceptance_state: A1, A3, A4 and AX remain open; first cold viewer startup failure remains unclassified.
progress_classification: progress

The full release build passed in 10m 03s. Exact binary SHA256 is
e7c871b3f26b26f28fb8b8574fbd1edfbc9155ce01342b3c6c7b241a788b6849.
Sequential exact-release cases passed: cold owner i7n71F, headless two-client
Vzedsf, remote-headed two-client mupgRW, headless session-only UB6vna and headed
session-only xMdLLM. Every case's exact disposable cleanup passed.

Controlled activation selected generation 0.28.0-e7c871b3f26b-d87931582350 with
support SHA256 d879315823506b99aa8601516f93b9ca6fb78f43572b641ded942052e0fe6bd4.
Five browser process identities and all 68 tab-custody rows survived. The new
host PID was 41744 at activation. Shared guidance was synchronized only after
verifying that its prior bytes matched the previously installed source.
Activation helpers have executed and must not be replayed.

The first authenticated cold attempt, local-viewers-PmQcm0, resolved the original
browser and target and wrote the new ready ingress receipt. It then failed its
30-second iframe check: the viewport stayed at Checking stream with zero
iframes. It did not reach input. Its client closed, and the failed attempt is
retained. A subsequent one-viewer diagnostic with request tracing reached the
iframe without input or repair; this does not establish the first failure's
cause. That diagnostic also retained an HTTP502 session-tabs read followed by
successful reads. Startup and causal-diagnostic disposition remain required.

The complete traced attempt local-viewers-UMFmI4 passed two authorized viewers,
anonymous denial, original target identity, synthetic pixels, trusted mouse and
keyboard, same-URL reconnect and peer continuity. Both viewers verified the
selected ingress acceptance receipt. Original-handle diagnostics afterward
remained complete, input counters were 7/7, and installed dashboard asset hashes
matched. This is a pass after the retained initial startup failure, not a clean
first-attempt result. All viewer clients closed.

Doctor before and after still exits1, but operatorJourneyReady changed from
false to true and dashboard_operator_journey_not_ready disappeared. The
remaining selectedGenerationReady=false still references old
failed_preserved_old_generation history. Principal/lease findings, stale monitor
and observed-pressure ownership remain. No history or green acceptance record
was manually synthesized, no consumer capability was borrowed, and no scheduled
timer was restored. Evidence is under publication-c249c531, including both doctor
snapshots, exact qualification, activation, custody comparison, viewer attempts,
original attestation and local-operator-acceptance.json.

Next: repair the separate current-generation readiness comparison with exact
selected payload, live host and dashboard manifest evidence while retaining
historical failures. Continue consumer original-connection acceptance, cold
viewer startup diagnosis, remaining ownership dispositions and A4/AX gates.

## Current-selection readiness proof repair

state_transition: Source repair validated for unrelated failed upgrade history.
acceptance_state: Current candidate readback and installed qualification remain pending; A1, A3, A4 and AX remain open.
progress_classification: progress

The selected-generation axis previously required the current selector to equal
the old or candidate generation of the latest upgrade transaction. That cannot
recognize a subsequent controlled installation even when its actual payload,
host and authenticated dashboard are healthy. The original terminal transaction
must remain history rather than being rewritten into an invented successful
upgrade.

A focused workstation_install/current_selection module now provides read-only
Linux proof when the latest transaction is closed zero-effect, rolled back
before commit, or failed with its old generation preserved, and the current
selection differs from both historical generations. Active admission drains,
active transactions and uncertain failures cannot use this path. It verifies the
stable selector and entrypoint, sealed generation and binary/support manifest
hashes, executable-bound live dashboard manifest and existing accepted journey,
current-boot settled host ingress, process start/executable identity, live binary
hash and socket identity. A final observation fences host and selector changes.
The ordinary dashboard's package-level label need not equal the installation ID;
its executable manifest supplies the identity join. No selector, transaction,
browser, owner or acceptance receipt is mutated by this proof.

Readiness adds currentSelectionEvidence with ready and error fields for this
specific observation; it is null when the path does not apply. Refusals expose
bounded diagnostic codes. All original readiness axes and terminal history are
preserved. Help, README, skill, installation docs and inline source guidance now
explain this behavior.

The focused regression uses a disposable process, socket and local manifest
server. It verifies positive current proof, preserved transaction content, an
active drain, active and uncertain transaction states, stale dashboard manifest,
missing accepted journey, mismatched payload hash, prior-boot host record and a
dead process. It passed. The existing seven-axis active-transaction regression
also passed, as did workspace Clippy with warnings denied, formatting and the
docs production build. No red-before-fix execution is claimed. An optimized
candidate build is running for read-only validation against the actual installed
runtime; production still runs c249c531.

Current-selection live readback: the optimized candidate build passed in 2m 13s.
The installed c249c531 status command reports selectedGenerationReady=false and
overall workstation-upgrade readiness=false. The candidate's read-only status
command against that same production installation reports
currentSelectionEvidence.ready=true, selectedGenerationReady=true and overall
workstation-upgrade readiness=true. Both retain
upgradeTransactionState=failed_preserved_old_generation. Hash comparisons prove
that neither ingress registry nor any existing transaction file changed, and
the installation selector is unchanged. This proves the new observation against
current production evidence; it does not publish the candidate, clear the other
doctor findings, or complete the full plan. Evidence and the optimized binary
are archived under current-selection-proof. The optimized build began on base
795d7c18 with the source changes later committed as 8c3ee439; final publication
must use a fresh release build from committed source.

## Installed 35611c64 current-selection checkpoint

state_transition: Current-selection proof is installed and local operator acceptance passed.
acceptance_state: A1, A3, A4 and AX remain open; full-plan acceptance is not established.
progress_classification: progress

The committed-source release build passed in 9m 48s. Its binary SHA256 is
9429d20fd5a55e6b7b71d4d7bdbc87e066363af801c061d040fc73ab03a98ce2.
All five exact-release qualification cases passed with successful disposable
cleanup: cold owner, two-client headless and remote-headed, and session-only
headless and headed. Controlled activation selected generation
0.28.0-9429d20fd5a5-457dd13afe4f. Five existing browser process identities and
all 68 tab-custody rows were preserved. Activation has executed and must not be
replayed.

The first traced cold viewer attempt, local-viewers-Otd4LZ, passed two
authenticated viewers, anonymous denial, synthetic pixels, trusted mouse and
keyboard, same durable URL reconnect and peer continuity. Its client closed.
The original synthetic handle subsequently reported complete control-plane
attestation, mouse/keyboard counters 8/8 and matching installed asset hashes.
This does not prove the consumer's distinct lease or explain the earlier
c249c531 cold startup failure.

Installed doctor still reports success=false, but the current-selection
readiness blocker is gone. Historical terminal upgrade evidence remains as a
warning. Remaining findings include consumer and legacy principal bindings,
owner-generation/session-authority mismatches, stale runtime monitoring and
observed process pressure with unknown ownership. No scheduled timer was restored
and no consumer capability was used. Private evidence is retained under
campaigns/p160/publication-35611c64, including qualification-status.json,
activation-receipt.json, the viewer event ledger, original-handle readback and
doctor-after.json.

Current pressure investigation found a concrete diagnostic classification defect:
Chrome executables below the agent-browser browser installation directory match
the agent-browser substring before browser classification. Browser children
therefore receive misleading agent-browser process reasons. Correcting executable
classification must preserve unknown ownership and must not grant cleanup
authority. Descendant ownership correlation and the stale monitor require their
own evidence; changing a resource label alone cannot satisfy A3.

### Executable classification source repair

state_transition: Misleading process-kind classification repaired and validated in source.
acceptance_state: Installed verification remains pending; A1, A3, A4 and AX remain open.
progress_classification: progress

Process classification now recognizes the agent-browser executable names and
uses the browser executable basename, rather than matching either program name
against the entire installation path. Chrome below .agent-browser remains a
browser; unrelated Node in an agent-browser fixture directory is not projected
as an agent-browser daemon. Browser classification alone retains observed
disposition and no GC action when ownership is unproven.

The new regression failed before repair because an unrelated executable was
included through its directory name. After repair it proves both managed Chrome
paths classify as browsers, an actual agent-browser executable below a
chrome-named directory remains a daemon, the unrelated program is excluded, and
no cleanup candidates are introduced. All 24 focused resource tests and four
browser-session-authority tests passed serially. Formatting, workspace Clippy
with warnings denied and the docs production build passed. These are source
checks, not installed or consumer acceptance. All five documentation surfaces
were updated; the shared installed skill remains bound to the installed source.

Production still runs 35611c64. No process cleanup, consumer capability use or
timer restoration occurred in this slice. Next: establish positive retained
browser descendant evidence, resolve current observed-pressure dispositions and
monitor freshness, then qualify the consolidated repair. Consumer original-handle
attestation and the remaining A4/AX acceptance contract still govern completion.

### Retained browser descendant resource proof

state_transition: Exact retained-browser descendant protection implemented and source validated.
acceptance_state: Current-process candidate readback and installed qualification remain pending; A1, A3, A4 and AX remain open.
progress_classification: progress

Fresh process and sidecar inspection confirms the retained Default root and its
Chrome children are still alive. The root's recorded process identity, Ready
owner generation and current-boot lifecycle agree, while the inventory's original
PID-only browser correlation leaves children unowned. This is resource accounting
evidence, not consumer lease authority.

A focused service_resources/retained_tree module now derives read-only child and
grandchild correlation from an exact current-boot retained root. Root admission
requires Ready browser and owner, matching physical profile, process identity,
owner generation, lifecycle, process group and package-launch digest. The sampled
ancestry must have matching observed executables, consistent boot/start tokens,
no missing parents or cycles, and no conflicting child profile. Only previously
observed browser rows become protected. Existing candidates and protected rows
are unchanged; the proof grants neither browser control nor GC authority.
The Linux collector also leaves failed executable observations absent rather
than substituting command-line text as executable identity proof.

The same regression failed on e09051ad with two descendants incorrectly observed,
then passed with the repair. It covers direct children, grandchildren, protected
RSS accounting, absent cleanup authority and twelve negative identity, lifecycle,
profile and ancestry cases. All 25 resource tests passed. Formatting, workspace
Clippy with warnings denied and the docs production build passed. Help, README,
skill, docs site and inline comments describe the boundary. Private red/green
logs and continuation evidence are under campaigns/p160/retained-tree-proof.
The pre-repair parent used for the red test has been restored to the integrated
source. Production remains on 35611c64, and no browser or lease was changed.

Next: build the consolidated candidate and compare its no-launch resource
projection with the installed command against current production evidence.
Unproven foreign processes, stale monitoring, the consumer's original connection
and remaining A4/AX gates still require their own disposition and acceptance.

### Live descendant projection and unexpected read-command persistence

state_transition: Live candidate projection passed; file-access audit found a separate read-command write defect.
acceptance_state: Publication held for diagnosis of unintended Service persistence; full plan remains open.
progress_classification: progress

The committed ed4552c3 optimized candidate built in 2m 23s, SHA256
4b834895733bcb36cf1157b7cb9cc38188871396819df0be36488678e2d34797.
The four browser-session-authority tests also passed. Comparing the installed
and candidate resource commands proved that the same retained Default children
change from observed agent-browser processes to protected browser descendants
with the exact retained browser correlation and no GC action. Observed process
identities remained unchanged. Both commands reported zero cleanup candidates;
candidate observed RSS remained approximately 6.82 GB, so pressure was not cleared.

The comparison's Service evidence hashes changed. A subsequent syscall trace
of the candidate command confirmed writes by that command itself: temporary
Service State, handoff, presentation, owner and lifecycle files plus transaction
publication and renames. Thirteen potential Service write operations excluding
the lock file were captured. The classifier result is valid, but the command
cannot be accepted as read-only. This observation does not establish whether
the unintended persistence predates the descendant repair, nor which state
fields changed during the first comparison.

Private evidence under retained-tree-proof now separates descendantProjectionPassed
from the failed readOnlyComparisonPassed assertion. The original comparison
receipt is retained as readback-before-file-access-audit.json; the correction,
candidate-resource-file-access.trace and file-access-readback.json are preserved.
The candidate has not been installed. Next: diagnose and remove the unintended
write path for the read command, verify that caller and owner state are preserved,
then resume consolidated qualification and the remaining A1/A3/A4/AX work.

### Read-path seeding persistence repair

state_transition: Unintended read-path persistence traced to the shared seeding refresh hook and repaired in source.
acceptance_state: Fresh candidate syscall verification and installed qualification remain pending; full plan remains open.
progress_classification: progress

The CLI unconditionally called refresh_persisted_profile_seeding_handoffs before
parsing every service/runtime command, including help. Three HTTP read helpers
called the same function. It always opened a repository mutation, so even zero
changed handoffs incremented the state revision and rewrote the full authority
sidecars. This path predates the descendant repair. The isolated CLI regression
reproduced a revision change from 7 to 8 on service resources without any seeding
handoff present; it failed before the repair.

The CLI and HTTP read hooks and their unconditional persistence helper are now
removed. Service reconciliation owns seeding-browser exit observation, including
the existing background reconciliation cycle. Its event records
closedSeedingHandoffCount. Reconciliation merges a changed handoff only when the
current record still equals the record it observed, preserving concurrent
re-seeding, updates and deletion. No new lifecycle or cleanup authority is granted.

All three Service State integration tests passed, including exact-byte preservation
and no authority-sidecar or daemon creation for resource reads, Service help and
runtime listing. The extended reconciliation summary test proves missing seeding
browser closure; the extended repository merge test proves persisted closure and
preservation of newer or deleted records. Both passed. Formatting, workspace
Clippy with warnings denied and the docs production build passed. All five help
and documentation surfaces describe the explicit persistence boundary. Direct
HTTP read-path runtime verification has not yet run. Private continuation evidence
is under campaigns/p160/read-command-proof; production remains on 35611c64.

Next: build committed source and repeat the candidate resource syscall audit,
then resume consolidated qualification. Consumer original-connection attestation,
remaining pressure and monitoring findings, and A4/AX acceptance remain required.

### HTTP status disconnect persistence repair

state_transition: Candidate CLI syscall audit passed; native HTTP status exposed a second unconditional persistence path, now repaired in source.
acceptance_state: Source regression and storage validation pass; consolidated candidate HTTP verification and installed qualification remain pending.
progress_classification: progress

The optimized 167f9246 candidate resource command issued zero Service authority
file writes in its syscall trace, excluding the lock file. Other production
processes still changed the shared files during that observation, so this does
not establish global state immutability. A corrected disposable native HTTP
fixture then preserved exact authority bytes for profiles and access-plan reads,
but status advanced stateRevision from 7 to 8 and created authority sidecars.
The earlier fixture stopped before requests because its setup wrongly expected
the host to create an initial state file. Both fixture hosts terminated with no
matching process residue. Evidence is under campaigns/p160/read-command-proof.

The daemon connection disconnect guard always called a repository mutation,
including a diagnostic connection with no active profile children. This explains
the status response retaining revision7 while the subsequent persisted state had
revision8. The new regression reproduced this unconditional rewrite before the
repair. Disconnect persistence now evaluates matching active child ownership
under the same exclusive repository lock as the update. A false predicate skips
both save and revision advancement. Real disconnects continue marking only their
own active children disconnected; ordinary transaction recovery is unchanged.

The focused regression passes for a read-only connection, a real child disconnect,
foreign child preservation and a repeated disconnect. All 29 existing storage
tests passed serially. Formatting, workspace Clippy with warnings denied and the
docs production build passed. Help, README, repository skill, docs site and inline
comments describe the persistence boundary. No production generation, browser,
consumer capability or retained synthetic handle was changed.

Next: build the committed candidate, repeat the native HTTP fixture including a
post-shutdown authority snapshot, then continue consolidated installed qualification
and the full A1/A2/A3/A4/AX acceptance contract. Original-consumer attestation is
still unproven; this read-path correction does not establish consumer recovery.
