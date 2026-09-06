# Plan 0160: Production Profile Identity and Operational Readiness

Date: 2026-09-06

State: OPEN

Execution state: `active_profile_ownership_diagnosis`

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

## Current state and reconciliation

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

The primary agent owns all units and integration. Active-agent concurrency is
one; no delegated discovery or parallel runtime mutation is planned.

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

## W1 ownership proof and failure matrix

For each blocker, distinguish requested identity, inherited/default selection,
stored authority and live observation. Record canonical profile identity,
logical browser/session and target, authenticated principal and grant reference,
lease/fence, owner generation, executable identity, PID plus process start/boot
identity, CDP endpoint and relevant namespace. A PID or directory match alone
is insufficient. Verify real authorization; do not weaken checks to reduce
error counts or create replacement profiles as recovery.

Cover positive named and custom profiles, session-only reuse, two authorized
clients, clean terminal close/reopen and retained host recovery. Negative cases
must cover conflicting explicit profile, foreign principal, missing binding,
ambiguous owner, changed generation, PID reuse and stale endpoint/target.
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

For changed Rust surfaces, run focused regressions plus repository-safe format
and workspace clippy checks. Run Service/client contract parity checks when
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
record the causal result, split or reframe locally, and continue unaffected safe
work. A bound does not authorize broader mutation or erase a blocked criterion.
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
