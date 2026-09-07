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
