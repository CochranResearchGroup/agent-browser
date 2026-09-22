# Plan 0211 | Simple Install, Upgrade, And Remote View

Date: 2026-09-17

Plan version: 65

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P211

Work items: `CochranResearchGroup/agent-browser#181`, `CochranResearchGroup/agent-browser#183`, `CochranResearchGroup/agent-browser#195`

Branch: `platform/p211-simple-cold-upgrade`

Target: `main`

Integration: merge through the protected pull-request workflow after provider-free shutdown, cold-install, restart, remote-view, and documentation validation

Source baseline: `fb616aeed2233aca06b4379d61af5d34609a804d`

## Objective

Replace the default production hot-upgrade path with one bounded cold workflow:
stop all Agent Browser-owned machinery, install the new payload, and start a
clean runtime. The ordinary operator interface must not require transaction
revisions, census digests, replacement-plan hashes, rollback decisions,
draining, runtime adoption, or semantic recovery orchestration.

Provide one idempotent `agent-browser shutdown` command that promptly closes
Agent Browser-owned browsers, stops Agent Browser units and timers, stops owned
presentation containers, releases Agent Browser leases and runtime ownership,
and leaves profile data present but unowned. Coordination metadata may be
reported as residue; it cannot veto an operator-requested shutdown.

Make a clean first install and the restarted runtime useful, not merely
nominally healthy. In the default trusted single-user mode, an ordinary client
must be able to name a profile, self-identify, and obtain a ready durable
`/remote-view/<handoff-id>` without generating identity hashes, capabilities,
repair plans, route identifiers, display identifiers, or recovery tokens.
Multi-user concurrency and adversarial identity are deferred until this
single-user journey is operational.

The ordinary trusted single-user path is recovery-first. A valid configured
profile must not be denied merely because retained session, browser, owner, or
prior-boot identity records collide. Preserve and report those records for
diagnosis, select the requested valid profile explicitly, and continue through
a fresh or requalified connection. Reject only an invalid profile request or a
case where the product truly cannot choose a concrete profile. Collision
telemetry must help improve reconciliation without turning Agent Browser into
a denial-only machine.

Replace the ordinary-path lease-management design rather than continuing to
repair each failure signature independently. The SoyLei Website failure is one
reproducer in a larger issue corpus, not the design target. A new Browser
Session Manager owns the simple profile, browser, session, tab-attribution,
heartbeat, and cleanup lifecycle inside the extracted Service Model crate.
Legacy Lease Authority, principal, owner-generation, sealed-recovery, and
boot-bound coordination modules remain available for historical readback and
deferred hardened modes, but they are not consulted as authority or fallback
by the default trusted single-user path.

Make the presentation provider the sole runtime authority for display
readiness, allocation, release, and handoff resolution. Browser Session
Manager must use one typed in-process provider API in the existing runtime
host; outside callers continue through the existing user-scoped Service
transport. It must not parse route inventories from environment variables or
generated files. Persist configuration, intent, ownership, leases, handoffs,
operations, history, and cleanup obligations in one transactional
service-owned SQLite database. Treat XRDP/Xorg processes, display numbers,
Guacamole connections, and other live process observations as ephemeral
evidence that the provider reconciles against that durable state after every
cold start.

## Version 65 Completion Campaign

Version 65 freezes the objective and replaces revision-driven continuation with
one moderate-length `/goal` completion campaign. Historical versions remain
evidence, but they do not create more work. No further Plan 0211 version is
needed unless the operator changes the objective, scope, or effect authority.
Ordinary implementation discoveries belong in the current milestone record.

Campaign objective: qualify one frozen candidate that completes the simple
cold workflow from owned shutdown through a ready opaque remote-view handoff
for one named profile, then present that candidate for protected integration.
Queueing, capacity optimization, multi-user hardening, retention polish,
quarantine automation, alternative transports, and architectural extraction
are deferred unless a failing campaign acceptance check proves one is an exact
blocker.

The campaign has one cumulative 2,000,000-token ceiling across orchestration,
workers, review, retries, validation, and closeout. The ceiling does not renew
at a sub-milestone, new session, successor packet, model change, or plan edit.
Record actual goal usage at each material checkpoint. Stop before starting a
unit whose estimated completion would exceed the remaining campaign balance.
The historical windows and their overruns remain historical accounting; this
operator-directed completion campaign is a new bounded allocation, not a claim
that the earlier work was efficient or complete.

Use one `/goal` campaign with these sub-milestones:

```text
/goal execute P211 completion campaign: freeze and qualify one simple cold-upgrade candidate through durable custody, atomic desktop-control activation, provider-free final qualification, one isolated installed operator journey, and protected-integration handoff; use at most 2,000,000 cumulative tokens and do not expand architecture unless a failing acceptance check proves an exact blocker
```

### M1 | Recoverable Atomic Candidate

Budget reservation: at most 350,000 cumulative campaign tokens.

Preserve the complete dirty checkpoint in durable custody, repair the accepted
desktop-control atomicity defect, and prove failure, success, replay, stale
generation, and concurrent-transfer behavior at the narrowest useful layer.
Lease publication and guarded focus must have one commit outcome, or an exact
rollback must be unable to overwrite an intervening controller. Compile the
changed Rust boundary and run focused tests, formatting and strict Clippy.

Exit evidence: durable source identity, clean or explicitly preserved custody,
focused green tests and no unresolved accepted review finding. Stop if the
atomic contract cannot be achieved without redesigning unrelated authority.

M1 completed at source checkpoint `cf75aac1`. The original unqualified v63
bundle is durably preserved under the user-scoped P211 campaign directory with
all three bundle hashes verified. The repaired SQLite activation stages the
successor lease, runs guarded focus, saves the resulting Browser Session State,
publishes desktop control and commits them under one IMMEDIATE transaction.
Failed effects roll back control and state; the host restores its cached state.

Qualification passed 14 focused desktop-control tests, three exact host
regressions, the complete `cli-native-other` compartment with 786 passed and 57
ignored, strict workspace Clippy, final formatting, service-client generation
and helpers, documentation links, remote-view docs, policy wiring, the route
handoff audit fixture and diff hygiene. The first compile attempt exposed a
wrong model re-export plus a cache-wrapper compiler-probe failure; the re-export
was repaired and final Rust gates ran with the documented cache opt-out. The
failed attempt remains diagnostic evidence rather than being erased by retry.

Delegation receipt: `/root/atomic_activation_fix` used `gpt-5.6-sol` at medium
effort, produced the single-transaction patch and was interrupted after it
stalled before validation; primary completed integration and qualification.
`/root/m1_closed_review` used `gpt-5.6-luna` at medium effort for closed-world
finding verification. `/root/m1_validation_map` used `gpt-5.6-luna` at low
effort for deterministic gate selection. Both read-only workers completed and
their accepted evidence shaped the final regression set. Goal readback at the
M1 source checkpoint was 326,747 cumulative tokens, within the 350,000 M1
reservation. Progress classification: `outcome_progress`. M2 is next.

### M2 | Frozen Provider-Free Qualification

Budget reservation: at most 500,000 additional tokens; cumulative campaign
usage must remain at or below 850,000.

Freeze one candidate and run the validation selected from the complete P211
batch, including the comprehensive provider-free Rust lane and required client,
contract, dashboard, documentation and planning gates. Repair only failures
causally introduced by P211 or required by its frozen acceptance contract.
Do not add another capability, abstraction, queue, recovery domain or plan
revision to improve the result.

Exit evidence: one immutable candidate identity, exact commands and results,
retained first failures, and a requirement-to-evidence table identifying the
installed-only gates. Any source repair invalidates only causally affected
evidence and returns this same milestone to qualification.

M2 completed at frozen source candidate `8bc9a19b`. The comprehensive Rust
runner passed every compartment except `cli-native-service` on its first run.
That compartment exposed one missing valid fixture for the already-supported
`service_runtime_config_update` action. The failure is retained; the fixture
was added without changing runtime behavior, its exact test passed, and the
complete affected compartment then passed 599 tests. All unaffected
comprehensive compartment evidence remains valid because the repair changed
only that test fixture. Final formatting and strict workspace Clippy pass.

The service client, API/MCP parity, dashboard workspace and inspector suites,
dashboard and documentation builds, documentation links, remote-view handoff
documentation, durable remote-view handoff, policy wiring and route-handoff
audit all pass. Two bounded read-only workers used `gpt-5.6-luna`: low effort
for non-Rust gates and medium effort for the requirement audit. Goal readback
after candidate qualification was 595,176 cumulative tokens, below the
850,000 M2 ceiling. Progress classification: `outcome_progress`.

The following table is the authoritative version 65 completion boundary. The
larger historical `Evidence And Exit` table below records earlier design and
qualification work; its deferred architecture rows are not Plan 0211 version
65 completion gates unless the installed journey proves one is an exact
blocker.

| Version 65 requirement | Current evidence | Remaining boundary |
| --- | --- | --- |
| Recoverable atomic candidate | `cf75aac1` and M1 qualification | complete |
| Frozen provider-free qualification | candidate `8bc9a19b`; comprehensive Rust evidence with repaired 599-test service compartment; client, contract, dashboard, docs, policy and handoff gates green | complete |
| One-command owned shutdown and cold replacement | provider-free shutdown, installer and workstation compartments green | installed development execution in M3 |
| Named-profile ordinary client path | provider-free manager, session, handoff and durable-handoff coverage green | installed development execution in M3 |
| Authenticated ready opaque handoff | contract and provider-free handoff coverage green | actual authenticated `operatorVisible.state=ready` `/remote-view/<handoff-id>` in M3 |
| Close and residue | provider-free ownership and foreign-process preservation coverage green | fresh installed process and residue readback in M3 |
| Protected integration handoff | branch and candidate are attributable | M4 review, ledger reconciliation, draft pull request and parity |

### M3 | Isolated Installed Operator Journey

Budget reservation: at most 800,000 additional tokens; cumulative campaign
usage must remain at or below 1,650,000. This milestone starts only with the
applicable installed-runtime effect custody and a frozen M2 candidate.

Run one isolated development cold-install or replacement-upgrade journey using
the documented development runtime: shutdown owned machinery, install the
candidate, start one runtime generation, open one named profile through the
ordinary client path, and obtain one authenticated ready opaque
`/remote-view/<handoff-id>`. Verify residue and process ownership after close.
Use the same journey for the primary product verdict. A provider, fixture or
environment failure is classified separately and does not authorize more
architecture or an automatic retry.

Exit evidence: one reproducible pass proving the operator journey, or one typed
blocking failure with exact candidate, stage, logs, cleanup and next repair.
One repair and one repeat are the maximum within the remaining campaign budget.

### M4 | Integration And Closeout Packet

Budget reservation: at most 350,000 additional tokens; total campaign usage
must not exceed 2,000,000.

Run one closed-world review limited to accepted blocking findings and critical
regressions introduced by their fixes. Reconcile the plan evidence table,
RUNBOOK, ROADMAP, active-lane catalog, draft pull request and branch parity.
Present the exact protected-integration decision without merging, installing
production, releasing, deleting the branch, or removing the worktree unless
separately authorized.

Exit evidence: either an integration-ready candidate with every Plan 0211
criterion supported at its stated boundary, or a truthful bounded closeout
naming the exact unmet criterion. Deferred hardening becomes separately
triaged follow-up work and cannot keep Plan 0211 open through more revisions.

### Campaign Controls

- The primary owns the critical path, scope, candidate freeze and acceptance.
- Use workers only for disjoint bounded implementation or deterministic
  validation that returns to the primary without separate Git or runtime
  custody.
- Checkpoint only at M1 through M4 exit, a material blocker, an authority gate,
  or the 30-minute cadence backstop. Routine commits do not receive paired plan
  revisions.
- At each checkpoint record state transition, acceptance state, progress class,
  evidence, cumulative tokens, material blockers and next action or stop.
- Two consecutive checkpoints without outcome progress stop the current tactic.
- One broad review has already occurred. M4 is closed-world verification only.
- Documentation work updates the owning facts once per milestone. Documentation
  volume, test count and checkpoint count are not progress.
- The only completion verdict is the ordinary installed operator journey plus
  current source qualification. Provider-free components alone cannot close
  Plan 0211.

## September 22 Version 64 Checkpoint Review Amendment

Version 64 accepts two blocking findings from the read-only review of the
uncommitted version 63 checkpoint. It changes the next packet and custody
requirements without qualifying, compiling, committing, or otherwise advancing
the implementation.

The current focus activation is not atomic. It commits a successor desktop
control lease before entering the separately transacted guarded focus effect.
If cached-session validation or focus fails, the successor remains current and
the prior controller remains fenced even though activation did not complete.
This violates the version 63 atomic-transfer acceptance criterion. Repair the
boundary so lease publication and the guarded effect have one commit outcome,
or use an exact compare-and-swap rollback that cannot revoke an intervening
controller. Tests must prove focus failure preserves the prior controller,
successful activation fences it, and an intervening transfer cannot be rolled
back or overwritten.

The recovery packet is internally consistent but not durable custody. The
manifest covers all 16 changed files, their hashes match the dirty P211 tree,
and the bundle-level `SHA256SUMS` verifies from inside its checkpoint directory.
Both the handoff and recovery packet live under `/tmp`, however, so host cleanup
or reboot can remove the only backup outside the dirty worktree. Before relying
on this checkpoint for another session, preserve the exact unqualified state in
durable non-temporary storage or an explicitly unqualified intermediate custody
commit, then verify its base HEAD, complete file set, hashes and recovery
procedure. Retain the current `/tmp` packet until that readback succeeds.

The next authorized technical sequence, after reconciling the exhausted
allowance, is therefore: establish durable custody; repair atomic activation;
run focused SQLite and journaled-handoff failure/success/concurrency tests; run
the complete changed-surface qualification selected from the v62 ledger base;
then join authenticated viewer and agent input to the same authority. No Rust
build, runtime, browser, provider, ingress, installation, commit, push or PR
mutation occurred during this review amendment. Version 63 remains unqualified,
and the complete Plan 0211 goal remains OPEN.

## September 22 Shared Control Boundary Amendment

Version 63 starts from source `6526ccef` and ledger `9d3bd816`. Fresh local
readback confirms a clean P211 checkout and tracked upstream parity. The previous
read-only handoff turn made no implementation progress. The existing coordinator
serializes process-local effects, while the ordinary viewer resolves into the
workspace and obtains a direct Guacamole sharing iframe. That transport does
not pass viewer input through the CLI coordinator. A SQLite lease or focus lock
alone therefore cannot qualify observer-only control transfer.

The intended outcome remains one controller across handoff activation, focus,
maximize, capture, pointer and keyboard, with connected observers preserved.
Implement the durable SQLite control and exact handoff-effect boundary first;
then route the authenticated live viewer transport through that same authority.
Do not label the first boundary as complete shared control or active-view proof.
The direct-provider viewer path must be replaced for ordinary manager handoffs,
not covered by a UI-only read-only flag.

Primary owns host integration, transport decisions, qualification and plan/docs.
Worker `/root/control_persistence`, requested sol/medium with effective settings
unknown, owns the new SQLite control child and narrowly required parent exports.
It has no runtime, Git or acceptance custody. P215 candidate-event contracts are
outside this write set. No new checkout or runtime is admitted.

At 07:33 UTC, approximately 287 minutes have elapsed since 02:46 UTC, with older
active effort unknown. The inherited 360-minute ceiling, review allowance and
retry accounting remain unchanged. Budget this intermediate boundary at 20
implementation, 15 qualification and five custody minutes, preserving the
remaining allowance for the transport join. Full remaining P211 acceptance is
not claimed achievable in this allowance; installed, quarantine and retention
gates remain open. No provider or installed effect belongs to this packet.

Acceptance for this boundary requires atomic transfer, exact operation replay,
old-controller and stale-generation rejection, and no focus effect outside the
current retained handoff binding. The complete shared-control row stays pending
until authenticated viewer and agent input exercise the same authority.

### Version 63 Operator Checkpoint

The operator requested `checkpoint` before Rust qualification. The working tree
contains the SQLite control domain, journaled handoff focus integration, a
superseded-activation regression, and synchronized user guidance/client typing.
Worker `/root/control_persistence` is terminal. Primary reviewed and corrected
per-display scope, required an exact expected keeper binding before transfer,
and added a cached-session-state comparison inside the effect transaction.
These changes are not yet compiled or Rust-tested and are not merge-ready.

Primary verification completed: service-client suite, documentation links,
remote-view handoff docs, formatting application and `git diff --check` pass.
Rust focused tests, strict Clippy, final formatting check, docs build and planning
audit remain pending. No runtime, browser, provider, ingress or installed effect
occurred. The previous v62 Rust qualification does not cover this dirty source.

At the operator checkpoint, fresh UTC readback is 10:30 on September 22:
464 wall-clock minutes since 02:46, exceeding the inherited 360-minute ceiling
by 104 minutes on the elapsed accounting used in prior checkpoints. Exact active
versus idle attribution is unavailable. This overrun does not renew the ceiling;
no further investigation or build starts at this checkpoint. The goal remains
OPEN, with the complete scope unchanged. Progress is unqualified implementation,
not completed shared-control or installed acceptance.

Retain the dirty P211 checkout for resumption. HEAD and verified remote remain
`9d3bd81685afcfa2e4be0bb05fc0467bc1be4e73`; no v63 source commit or push is
claimed. A patch, complete changed-file archive and manifest are preserved in
`/tmp/agent-browser-p211-v63-checkpoint-2026-09-22/`. The next technical gate is
focused SQLite and handoff recovery qualification, followed by actual viewer and
agent input enforcement. Reconcile the exhausted allowance before starting that
work; do not treat this checkpoint as an allowance reset.

## September 22 Reserved Browser Discovery Amendment

Version 62 starts from source `89432d9b` and ledger `5da931dc`. The production
runtime still inherits the default unproven reserved-launch recovery. Replace
that missing adapter using the selected profile's local DevTools endpoint,
Chrome's reported browser PID and exact cross-platform process observations.
There is no need for an unbounded system-wide process scan. The reservation
marker, selected profile, browser family, executable and process start identity
must all agree, and the endpoint and identity must remain stable across proof.
Missing, malformed, conflicting or ambiguous evidence retains the existing
cleanup obligation without launching or terminating a process.

Primary owns the asynchronous CDP proof, runtime actor integration, plan/docs,
qualification and custody. Existing sol/medium worker `/root/capacity_selection`
owns a pure process-proof child and representative platform argv fixtures.
Effective model settings are unknown. No new worktree or runtime is admitted.
Provider-free local mock CDP and injected process observations exercise the real
adapter; they do not establish native macOS/Windows or installed acceptance.

This independently useful recovery prerequisite budgets 18 implementation,
12 qualification and five custody minutes. At about 07:14 UTC, 268 minutes have
elapsed since 02:46 UTC; older effort is unknown and the inherited ceiling stays
360 minutes. Roughly 92 measured minutes remain. Full persistence, shared-control,
quarantine and installed-matrix acceptance is not claimed achievable within
that estimate. No allowance or retry counter resets; the complete objective
remains unchanged. No installed/provider/production/ingress effect is included.

Acceptance requires an exact reserved browser to recover without a second launch,
wrong or stale PID/profile/marker/endpoint evidence to remain unresolved, bounded
read-only probing, preserved prior open recovery, required Rust quality and docs.

### Version 62 Qualification

Source `6526ccef` replaces the production reserved-browser recovery stub with a
selected-profile DevTools probe. Two read-only Chrome process queries bracket
exact OS observations. The profile endpoint is read again before returning the
proven launch. The existing host journal adopts that observed launch and retains
its no-second-launch rule. Missing or conflicting evidence remains unresolved;
this implementation does not guess a process or terminate one.

Primary qualification:

| Gate | Result | Evidence scope |
| --- | --- | --- |
| Initial Browser Session Runtime focus | 13 passed, 2 ignored | pure Linux/macOS/Windows argv fixtures, mock CDP success/drift/deadline, existing runtime contracts |
| Final CLI native-browser compartment | 163 passed, 2 ignored | includes the added real actor-dispatch fixture, prior host interrupted-open adoption, navigation/handoff recovery, browser and SQLite contracts |
| Strict workspace Clippy | passed | production source unchanged after this gate; the later addition is a test-only actor fixture |
| Final formatting | passed | complete frozen source and tests |
| Docs build, links, handoff docs and active planning audit | passed | all five required user guidance surfaces aligned |

Logs use `/tmp/p211-v62-*`; final browser evidence is `browser-frozen.log`.
Frozen input hashes in `frozen-inputs.json` were rechecked without drift before
source custody. Fixtures and counts are retained here; raw temporary logs remain
reproducible. No failed test or repair retry occurred. The earlier v61 daemon,
stream, model, client and installer evidence remains scoped to unchanged inputs;
this packet changes the reserved-recovery adapter and its focused dependencies,
not those contracts. No comprehensive final-head acceptance is claimed.

Worker `/root/capacity_selection` is terminal. Primary reviewed its pure proof,
added the asynchronous read-only probe and actor join, and ran all Cargo gates.
No native macOS/Windows process test or real Chrome recovery was executed: the
cross-platform claim is implementation plus representative argv fixtures only.
No installed runtime, provider, production, ingress or release effect occurred.
No shared installed skill was overwritten or CI run monitored.

State transition: reserved discovery missing to provider-free qualified.
Progress classification: `outcome_progress` for interrupted launch recovery.
About 280 elapsed minutes since 02:46 UTC are recorded, older effort unknown,
with the inherited 360-minute ceiling unchanged. Roughly 80 measured minutes
remain; full remaining acceptance is not claimed achievable in that estimate.
Next is shared desktop control at the SQLite/keeper and Desktop Services boundary.
The current Desktop Services coordinator is process-local and its CLI adapter
reads legacy Service State; a focus lock alone cannot satisfy the required
single generation-fenced controller and observer-only transfer behavior.
Quarantine reconciliation, bounded persistence/retention, remaining domain joins,
and frozen isolated cold-install/upgrade/ready-remote-view acceptance stay OPEN.

## September 22 Navigation And Handoff Recovery Amendment

Version 61 starts from source `214dfc77` and ledger `ddb847a9`. Verification
corrects the prior next-step assumption: only browser opens currently have the
operation journal integration. Navigation and handoff focus need their own
intent before queue restart can admit them safely. This batch adds that missing
join rather than treating queue re-admission alone as completed recovery.

Navigation records the exact original command, selected session/browser/tab and
issued transition. Interrupted issued navigation must inspect the exact retained
target and desired URL instead of blindly repeating navigation or header effects.
Initial browser creation reuses the existing deterministic open journal. Handoff
resolution retains exact logical handoff identity and requalifies the current
keeper and browser target before repeating idempotent focus. Historical response
replay remains distinct from fresh operator visibility. Missing or mismatched
journals preserve the unresolved obligation.

Primary owns host persistence seams, handoff journaling, daemon routing, docs,
integration and acceptance. Existing terra/high worker `/root/startup_recovery`
owns the new child navigation module; existing sol/medium worker
`/root/capacity_selection` owns the admission matcher, atomic SQLite publication
seam, queue restart fixtures and handoff recovery fixtures. Primary identified
that the open commit contract requires the observed result to equal the final
response; navigation instead uses an exact expected-observation guard in the
shared transaction, preserving both state CAS and generation fencing. Navigation
history must never persist before this transaction. Effective settings are unknown. Worktrees and runtime custody do not change.

Budget 25 implementation, 12 qualification and five custody minutes within the
inherited 360-minute ceiling. Preserve the recorded 241 elapsed minutes and
unknown older effort; this packet begins about 06:49 UTC. Roughly 70 measured
minutes would remain after this packet. Remaining full-plan persistence,
Desktop Services, recovery and installed acceptance are not claimed achievable
from that estimate. No retry, review or milestone allowance resets.

Acceptance is the actual host journal and SQLite queue restart journey under
injected browser effects: exact same request/target, no duplicate navigation,
current keeper readiness, stable handoff URL, and mismatch refusal. Run focused
host/admission and affected Rust gates, strict quality and required docs checks.
No browser, installed runtime, provider, production, ingress or release effect
is included. The full install/upgrade/ready-remote-view goal remains OPEN.

### Version 61 Qualification

Source `89432d9b` joins exact navigation and manager handoff journals to durable
queue restart admission. The navigation fixture crosses queue generation 1 to 2
with one command execution, one history entry, one stable opaque handoff and a
completed queue entry. Issued target mismatch retains the obligation. Executed
recovery requalifies the live target before atomic publication; a successful
redirect need not match the originally requested URL. Handoff replay rechecks
current keeper readiness and exact logical identity before idempotent focus.
A completed queue response remains historical evidence, not fresh visibility.

Primary qualification on frozen source:

| Gate | Result | Evidence scope |
| --- | --- | --- |
| Browser Session Host | 22 passed | new navigation/queue crash join, atomic history, executed-target disappearance, handoff freshness and identity refusal, prior host regressions |
| Browser Runtime SQLite store | 31 passed | shared atomic publication, retained open behavior, generations and base-state CAS |
| Browser Session Runtime | 6 passed, 2 ignored | provider-free adapter/process contracts; real-browser cases not run |
| Presentation admission | 6 passed | exact owner/action/payload joins, missing/mismatched journals, interrupted and committed recovery |
| CLI native-other | 786 passed, 57 ignored | daemon and admission regression compartment |
| CLI native-stream | 253 passed | stream/HTTP consumer regression compartment |
| Format and strict workspace Clippy | passed | final code including publication-input refactor |
| Docs build, links, handoff docs and active planning audit | passed | all five required user guidance surfaces aligned |

Initial strict Clippy failed three argument-count checks. The correction groups
publication inputs and takes the retained operation directly; it preserves exact
observed-result, owner, generation and state guards. The failed log remains
`/tmp/p211-v61-clippy.log`. A 21-test host pass preceded the additional executed
target readiness fix; the final 22-test pass supersedes it. Final focused/quality
logs use `/tmp/p211-v61-*-frozen.log`; broader logs are `other.log` and `stream.log`.
These temporary logs are reproducible through the commands above; the source,
fixture assertions and counts are retained here rather than copying raw logs.
Frozen input hashes are `/tmp/p211-v61-frozen-inputs.json` and were rechecked
without drift before source custody.

Both workers are terminal. Primary accepted their code after diff inspection,
identified the initial publication-result mismatch and executed-target freshness
gap, and ran every reported Cargo gate itself. No installed runtime, browser,
provider, production, ingress or release effect occurred. No shared installed
skill was overwritten. Validation selection overmatches workstation fixtures
through the daemon path; no installer/helper/provisioning input changed, so v60
coverage for those surfaces is retained. Client/schema and model inputs also
remain unchanged. This is not comprehensive final-head or installed acceptance.

State transition: recovery join active to provider-free qualified.
Progress classification: `outcome_progress` for interrupted request recovery,
not full installed operator acceptance. About 265 elapsed minutes since 02:46 UTC
are recorded, with older effort unknown and the inherited 360-minute ceiling
unchanged. About 95 measured minutes remain; this does not establish that all
remaining criteria can fit. Next is causal reserved-browser discovery at the
existing recovery seam. Active-view control priority, quarantine reconciliation,
remaining persistence/retention, Desktop Services and the frozen isolated
cold-install/upgrade/ready-remote-view matrix remain OPEN.

## September 22 Durable Automatic Provisioning Amendment

Version 60 starts from source `d25321dd` and ledger `8cef9eae`. Existing
route-user setup can reset any same-named account, and full Guacamole sync
updates retained connections. Automatic growth must use operation-owned user
creation and insert-only connection provisioning with exact idempotent replay.
Provider coordinates and deterministic naming prefixes are imported by the
installer into SQLite; runtime reconciliation must not rediscover them from
environment or JSON inventories. Public maximum-display changes record desired
capacity, with the supervisor owning bounded one-route provisioning effects.

Primary owns the SQLite intent/configuration import, configured supervisor
adapter, public status/docs and integration. Existing terra/high worker owns
operation-owned user creation and mocked helper fixtures; existing sol/medium
worker owns the one-route SQL renderer. Effective settings are unknown. No
worker may install the helper, call sudo, Docker or a live provider. Runtime
commands are implemented but tested through injected effects and temporary
stores. The existing catalog publication receipt digest must also include
provider URLs, matching the Rust authority rather than hashing bindings alone.

One durable operation ID and secret survive interruption and retry. Provisioning
must refuse foreign account or connection collisions and never rewrite retained
routes. A failure preserves the pending operation and a redacted diagnostic;
it must not shut down working routes. Retries remain bounded and publication
joins exact operation identity to an additive catalog transaction. Logical
settings, physical capacity and readiness are distinct status boundaries.

Budget 35 implementation, 15 qualification and 5 custody minutes within the
inherited 360-minute ceiling, preserving 206 elapsed minutes and unknown older
effort. This packet targets the configured growth journey, not full P211
acceptance; roughly 99 minutes would remain in the measured allowance after a
full packet. Remaining recovery, persistence, Desktop Services and isolated
acceptance still require evidence and are not declared achievable from that
estimate. No local packet or reviewer allowance resets.

Acceptance: strict same-operation helper replay and collision refusal; SQL
new-only behavior; runtime-config to durable intent to injected provisioning to
catalog publication with retained records unchanged; interruption/failure
readback and bounded retry; installer/Rust digest parity; relevant Rust, client,
helper and documentation gates. No installed/provider execution in this packet.

Prerequisite verification found that the ordinary privilege installer and Rust
helper doctor still accepted a helper without owned provisioning and could exit
before replacement. The same batch updates those exact compatibility checks and
mocked fixtures, so normal installation supplies the new capability. This is a
required join for automatic growth, not a separate hardening packet. The existing
worker owns that narrow fix; primary retains integration and acceptance.
A cold-install replay after growth must also retain the expanded catalog when
the exact same provisioning coordinates and unchanged contiguous original seed
are imported again. The batch covers that joined replay without allowing ordinary
catalog shrinking or changed bindings. Otherwise the original smaller installer
descriptor would fail publication after successful automatic growth.

### Version 60 qualification

Source `214dfc77` joins public desired capacity to durable one-route provisioning,
additive publication and configured supervisor reconciliation. The imported
coordinates remain in SQLite. Operation-owned helper accounts, insert-only SQL
and exact replay preserve retained routes. Status separates desired settings,
physical slots and live readiness. Normal helper installation now refreshes a
helper missing the owned-provisioning capability. An unchanged original installer
seed can replay after growth without shrinking the retained catalog.

| Requirement | Evidence and scope | Remaining gate |
| --- | --- | --- |
| Configured growth and interrupted replay | Final nine provisioning tests exercise real SQLite plus injected effects, same operation/password after reopen, redacted failure, three-attempt bound, namespace rejection and seed replay | Actual PostgreSQL/helper execution and route visibility are not exercised |
| Durable publication and identity | Final 31 store tests and three catalog tests pass, including a fixed serialization digest oracle matching URL fields and bindings | Installed importer readback remains in the frozen matrix |
| Current readiness and public config | 16 host and six runtime-config tests pass; full client and API/MCP parity pass | Real operator journey remains unverified |
| Helper and normal installer compatibility | 18 helper-focused tests pass; direct helper, clean privilege-install and workstation-host mocked fixtures pass | No installed helper replacement or root effect occurred |
| Broader regression and quality | 784 native-other with 57 ignored, 253 stream, final 1102 core with one ignored; final strict workspace Clippy, formatting, docs build, links, handoff docs and active planning audit pass | Not the comprehensive runner on final HEAD |
| Full P211 outcome | Still OPEN: interrupted handoff recovery, active-view control priority, causal discovery, quarantine reconciliation, remaining persistence/retention and Desktop Services joins, isolated cold-install/upgrade/remote-view matrix | No merge, production, ingress or release acceptance |

The initial core run had 1086 passes, 16 failures and one ignored: old current-helper
samples omitted the newly required capability. Worker fixture-only corrections
updated the common positive samples and preserved each negative test's original
missing field. The final core rerun passes; the failed log remains at
`/tmp/p211-v60-core.log`. A format check encountered the late seed-replay edit;
final formatting and quality checks pass. Final growth/store/quality logs use
`/tmp/p211-v60-*-frozen.log`; core uses `core-final.log`. All other focused logs
use the same v60 prefix. These are ephemeral local receipts; commands in the
repository runner reproduce them from this source checkpoint.

Broader native-other and stream gates precede the narrow final seed-import join
and helper-compatibility fixture correction. Their keeper and browser behavior
is unchanged; final growth/store tests cover the seed join, and final core plus
helper tests cover compatibility. Retain that scoped evidence without claiming
a comprehensive final-head run. The source-free workstation install fixture also
passed using its pre-existing debug binary; it is supporting fixture evidence,
not qualification of the new embedded helper or an installed candidate. The
shared production skill and installed runtimes were intentionally not changed.

Workers `/root/capacity_selection` (requested sol/medium) and
`/root/startup_recovery` (requested terra/high) are terminal. Effective settings
remain unknown. Primary inspected their consequential diffs, integrated the
runtime and status joins, and independently ran the decisive tests. No worker
or primary performed provider, browser, installed-helper, Service State, ingress
or production mutation. P215's separate dirty worktree remains untouched.

Progress is `outcome_progress` for the provider-free configured capacity-growth
criterion; it is not the leading installed operator outcome. About 241 elapsed
minutes since 02:46 UTC are preserved, with older cumulative effort unknown and
the inherited 360-minute ceiling unchanged. This v60 packet began about 06:13 UTC
and stays within its 55-minute bound. The next bounded delivery work is exact
interrupted handoff/navigation recovery through existing journals, followed by
remaining persistence/Desktop Services integration and the complete isolated
acceptance matrix. The full goal remains active; no allowance is reset.

## September 22 Additive Catalog Publication Amendment

Version 59 starts from source `2f138944` and ledger `21d31c46`. The next
capacity milestone is automatic growth preserving every active route. Current
provisioning derives deterministic route users and connections from an installed
descriptor, but publication requires an equal slot set and all-Absent catalog
replacement. Full connection synchronization can rewrite retained connections.
The first required integration is additive publication through the existing
installer bridge, SQLite and configured start/observe/stop/adopt adapters.

Retain exact historical catalogs by digest and accept their fences only when
all old bindings and provider URLs remain unchanged in the current catalog.
Append contiguous Absent slots without rewriting any retained record or receipt.
Publish the whole extension atomically; keep logical runtime limits separate
until the user configuration changes. Concurrent old effect receipts preserve an independently appended catalog only
after exact retained-state equality; changed route evidence remains a conflict. Rebinding,
removal, unknown digests and malformed slot sets remain errors.

Primary owns SQLite publication, provider guards, integration fixtures, docs and
qualification. Existing sol/medium worker owns the pure authority extension and
its tests. Existing terra/high worker returned the bounded provisioning seam
assessment; no provider effects were performed. Effective settings are unknown.
Budget 20 implementation, 15 qualification and 5 custody minutes within the
unchanged 360-minute ceiling, preserving 185 elapsed minutes and unknown older
effort. No review or retry allowance resets. This removes the active-catalog
publication blocker; it does not by itself prove automatic provisioning or the
installed growth journey. A new-only provisioning operation and runtime-config
trigger remain necessary, with no full-connection rewrite accepted as a shortcut.

Acceptance requires retained Ready/Adopting/Stopping identity through additive
publication, exact new-slot starts, immutable bindings, atomic SQLite rejection,
old-fence configured observations/stops and startup recovery, plus required Rust
quality and documentation gates. The original P211 objective and isolated
acceptance matrix remain unchanged and OPEN.

### Version 59 qualification

Source `d25321dd` removes the active-catalog publication blocker. The installer
publication bridge accepts a complete contiguous larger catalog, retaining old
catalog snapshots, active fences and receipts. New slots are Absent; configured
limits remain unchanged until a separate runtime-config update. Configured
start, observation, adoption and stop resolve old fences through validated
historical bindings. Pristine initial publication can establish provider URLs
and larger capacity before the first host claim without retaining false history.

SQLite merges independently appended catalogs into same-host in-flight effect
receipts only after exact retained-state comparison. Multiple appends preserve
intermediate digests for pristine added slots. Changed route state, binding,
provider URL or host-claim transitions cannot use that merge. A host change must
read fresh authority to rebase the newly added slots correctly.

| Evidence | Result and scope |
| --- | --- |
| Full Service Model package | 318 pass; includes retained route start/stop/adoption, atomic invalid extension refusal, digest forgery rejection and initial unconfigured growth |
| CLI store, final frozen source | 31 pass; active publication, configuration separation, new-slot start, two appends racing with old stop intent, and changed-route/host conflict refusal |
| CLI native-stream | 253 pass; existing configured fixtures now append during observation, adoption and stopping before exact provider-helper calls |
| CLI native-other | 775 pass, zero failures, 57 ignored; includes keeper startup, recovery, publication bridge and lifecycle regression coverage |
| CLI browser-session host | 16 pass |
| Quality and docs | Final strict workspace Clippy, formatting, docs build, documentation links and handoff docs and final active planning audit pass |

Initial store qualification recorded 30 passes and one real multi-append merge
failure: deterministic direct extension used the final digest for a slot that
correctly retained its intermediate catalog. Primary repaired that comparison,
verified the historical binding, and added explicit host-transition exclusion.
The final frozen store run passes all 31 cases. A format check caught the last
closure wrapping; it was applied and the final check passes. Initial receipts
remain under `/tmp/p211-v59-*`; final store/quality logs use `*-frozen.log`.
The model and configured adapter source were unchanged after their passing runs.
Broader native and host results precede only the narrowly scoped SQLite
multi-append/host-match correction and formatting; final store and quality gates
cover that correction. This is selected provider-free qualification, not the
comprehensive runner on final HEAD or installed acceptance. Unchanged v58
client/schema/MCP evidence is retained because those surfaces did not change.

The sol/medium worker supplied the bounded model implementation. Primary added
initial-publication and synthetic-fixture compatibility requirements and used
full historical validation at adoption. The terra/high worker returned the
provisioning seam: descriptor-generated route identities, `ensureRouteUser`,
`syncConnections`, exact connection-ID readback and the publication bridge.
Both workers are terminal; effective settings are unknown. No new reviewer,
worktree, installed candidate, provider or shared-runtime effect was used.

Progress is `blocker_reduction` toward automatic capacity growth. About 206
elapsed minutes since 02:46 UTC are recorded, with older effort unknown and the
inherited 360-minute ceiling unchanged. The next outcome is provisioning only a
new route and joining it to a durable runtime-config growth intent. Existing
full-inventory synchronization may rewrite retained connections and is not an
acceptable substitute. Remaining recovery, persistence/retention, Desktop
Services and the complete frozen isolated acceptance matrix remain OPEN.

## September 22 Reference-Free Cooldown Scale-In Amendment

Version 58 starts from source `b2a3ff39` and ledger `e51c150b`. Current source
shows catalog replacement requires every slot Absent and fences bind the full
catalog digest. Automatic additive growth therefore needs an explicit retained
identity design and provider provisioning join; it remains open. The next
integrated capacity lifecycle outcome is reference-free cooldown scale-in using
the existing exact XRDP stop adapter, rather than changing identity opportunistically.

Primary owns supervisor integration, effect-error handling, public cooldown
configuration and qualification. Existing sol/medium worker owns only a pure
idle-state selector and exports; terra/high worker owns SQLite reference joins,
transactional stop reservation and fixtures. Effective settings are unknown.
No provider, installed runtime or shared state effect is authorized by this
provider-free packet. Budget 20 implementation, 15 qualification and 5 custody
minutes within the inherited 360-minute ceiling, preserving 165 elapsed minutes
and unknown older effort. No review/retry bound resets.

One immediate SQLite transaction checks retained browser/handoff references,
queued/admitted/recovery-required work and pending operation journals, persists
idle evidence, and reserves at most one exact stop after cooldown above the
warm/minimum floor. Generation, fence, clock regression or renewed use resets
idle evidence. Ordinary queue insertion serializes with this reservation; an
admitted browser operation prevents scale-in until its journal/state is durable.
Unknown references and pending route recovery block automatic scale-in.
The supervisor executes only the reserved stop. Unproven or failed cleanup
retains a quarantine obligation without declaring reclamation or broad cleanup.
Public `scaleInCooldownMs` joins the supported atomic configuration patch.

Acceptance: pure cooldown/reset invariants, SQLite obligation/refusal and stop
reservation, supervisor exact stop and failure containment, existing startup
and contract tests, strict quality and public documentation parity. Catalog
expansion, remaining recovery/retention/Desktop Services work and the complete
isolated cold-start matrix remain part of the unchanged objective.

### Version 58 qualification

Source `2f138944` integrates the pure idle selector, durable SQLite evidence and
atomic stop reservation, supervised exact stop, and public `scaleInCooldownMs`.
One route above the warm/minimum floor can retire only after reference-free
cooldown. Pending journals, queue admission/recovery, unknown references and
unresolved route phases clear idle evidence. Failed provider stop is contained
as an exact quarantine obligation; it does not trigger broad supervisor cleanup.

Primary verification:

| Evidence | Result and scope |
| --- | --- |
| Service Model `presentation_scale_in` | Four focused cases pass: cooldown, references/admission, fence/generation/clock reset and target/phase boundaries |
| CLI `browser_session_store` | 29 pass, including real SQLite pending-journal cooldown reset, exact stop reservation and browser/handoff reference refusal |
| CLI native-other | 775 pass, zero failures, 57 ignored; includes successful exact retirement, failed-stop quarantine and no broad shutdown or immediate rewarming |
| CLI `runtime_config` and MCP service-request | Six and five pass; seventh public setting preserves strict schema and atomic patch behavior |
| CLI native-stream | 253 pass, including configured startup/recovery and provider adapter contracts |
| Quality and contracts | Final formatting and strict workspace Clippy pass; full client, API/MCP parity (120 actions), docs build, links, handoff docs and active planning audit pass |

The native-other run precedes only an equivalent iterator flattening in the
store and a fixture assertion correction; final store/config/MCP qualification
and Clippy cover those changes. Unchanged model/client/docs evidence is retained.
This is not a comprehensive all-compartment or installed acceptance claim.
Reproduce with the named focused filters and compartment runner; ephemeral logs
use `/tmp/p211-v58-*`, while source and commands are durable here. The initial
compile rejected a SQLite `u64` count read; primary corrected it to `i64`.
Initial Clippy rejected a manual flatten; the final check passes. Primary also
corrected two fixture expectations before the store run: cooldown starts at the
first post-obligation observation, and an unstarted provisioned slot remains
Absent. Initial failed logs remain, without representing them as test passes.

Workers returned bounded model and store changes, with effective model/effort
unknown. Primary reviewed the joins, required last-observation clock regression
reset, repaired the integration issues above and ran qualification. Both workers
are terminal. No installed/provider/shared runtime mutation occurred. The shared
user skill and P215 work are untouched.

This is `outcome_progress` for the capacity lifecycle criterion, not completion
of P211. About 185 elapsed minutes since 02:46 UTC are recorded; older active
effort remains unknown and the inherited 360-minute ceiling is unchanged.
Automatic catalog expansion still requires retained catalog identity and the
provider provisioning join. Interrupted handoff recovery, causal discovery,
quarantine reconciliation, remaining SQLite/retention and Desktop Services work,
and the complete frozen isolated cold-start matrix remain OPEN.

## September 22 Public Runtime Configuration Amendment

Version 57 continues from qualified source `8ab466e8` and ledger `f2b8677f`.
Expose ordinary `service runtime-config get` and `set <json-object>` plus the
matching generic service-request actions. Partial typed updates need no client
revision, preserve unrelated settings, and commit configuration and keeper
minimum/warm policy together. Existing ready routes survive lowered limits;
allocation observes the lowered cap and stale demand is clamped. The current
physical catalog bounds display growth; automatic catalog expansion and
reference-free scale-in remain required follow-up, not claimed complete.

Primary owns CLI, handlers, integration and acceptance. Existing startup worker
owns only the SQLite patch/transaction/tests (requested terra/high); capacity
worker owns service request schema, action metadata and generated client
(requested sol/medium); documentation worker will own public prose. Effective
models are unknown. No extra worktree, runtime or provider mutation is included.
Budget 20 minutes implementation, 15 qualification and 5 custody inside the
unchanged 360-minute ceiling, preserving about 150 elapsed minutes and unknown
older effort. Acceptance: ordinary no-launch command read/update roundtrip,
atomic rollback, idempotent replay, live keeper policy readback and generated
contract parity. This advances public configuration without reducing the full
cold-install, restart and remote-view acceptance contract.

### Version 57 qualification

Source `b2a3ff39` exposes the six supported runtime settings through CLI and
generic HTTP/MCP service requests. Readback includes revision; partial updates
need no revision token. Configuration and keeper policy commit together.
Concurrent provider receipts preserve live policy changes, and demand writes
clamp against the current SQLite cap even when a waiter read stale settings.
Existing routes and browser assignments remain intact when limits decrease.

Primary qualification passes six focused runtime-config tests, five MCP
service-request tests, full service-client checks, API/MCP parity (120 actions),
final formatting and strict Clippy, docs build, links, handoff docs and active
planning audit. Native-other recorded 773 passes, 57 ignored and one fixture
failure: the old startup test treated warm-target drift as authority drift.
That premise conflicts with the new independent-policy merge. The corrected
fixture changes a retained route phase instead and passes its focused rerun,
preserving exact recovery-evidence rejection. No production source changed
following the broad run; passed cases are retained rather than claiming a full
rerun. The first failure remains in `/tmp/p211-v57-other.log`; the correction
is `/tmp/p211-v57-startup-final.log`. Other logs use `/tmp/p211-v57-*`.

The store and contract workers completed their scoped changes on the requested
terra/high and sol/medium routes, effective settings unknown. Primary added
stale-cap demand fencing and integrated the CLI/no-launch fixture. The prose
worker could not resume because the tool reported its thread limit; primary
completed all mandatory prose surfaces. No replacement worker or extra review
was opened. Draft PR #191 remains OPEN; P215 dirty work was preserved.

This is outcome progress for ordinary public configuration within the existing
physical catalog. Automatic catalog expansion, reference-free scale-in,
interrupted handoff recovery, causal process discovery, quarantine, remaining
persistence and Desktop Services joins, and the complete frozen isolated
acceptance remain OPEN. No installed/provider effect occurred. About 165 elapsed
minutes since 02:46 UTC are recorded, older active effort unknown; inherited
360-minute ceiling and retry/review accounting are unchanged. Next consolidate
catalog expansion and capacity lifecycle with the existing configured keeper.

## September 22 Journal Recovery Admission Amendment

Version 56 continues from source `f3d0a758` and ledger `c48610d8`.
The current checkpoint has the two attributed worker edits for pure queue
reconciliation and the SQLite host fixture. The primary now joins admission to
an exact original open command and `browser-runtime-open` journal owner.
The existing journal, not the queue, determines whether to recover or launch.
Missing or mismatched journals and interrupted navigation or handoff requests
retain their recovery obligation. Full automatic handoff recovery remains open.

The inherited 360-active-minute ceiling, historical unknown effort and last
recorded 138 elapsed minutes are preserved. This packet budgets 15 minutes
implementation, 15 qualification and 5 custody without resetting retries.
The pure model worker supplied exact-entry reconciliation and three negative
or success tests; the host fixture worker supplied the interrupted SQLite case.
Their prior requested routes remain sol/medium and terra/high; effective
settings are unknown. The primary integrates the adapter and changes the host
fixture to exercise queue re-admission, journal recovery and queue completion.
The existing documentation worker handles three public prose surfaces while
primary owns CLI help. No new worktree or installed/provider effect is included.

Acceptance requires a fresh fenced queue attempt after restart, exact journal
recovery to the original ready handoff, one total browser launch, retained
obligations when proof is absent, focused model/adapter/host tests and strict
Rust quality gates. Public configuration, live resizing, scale-in and the
remaining complete isolated acceptance matrix remain part of the full goal.

### Version 56 qualification

Source `8ab466e8` integrates exact journal-backed recovery admission. Primary
qualification passes 12 queue model tests, four SQLite admission tests, the
real interrupted-open journal fixture, and 774 native-other tests with 57
ignored. Formatting, strict workspace Clippy, docs build, documentation links,
remote-view handoff docs and active planning audit pass. Logs are
`/tmp/p211-v56-{model,admission,journal,other,fmt,clippy,docs-build,docs,handoff-docs}.log`
and `/tmp/p211-v56-planning.json`. No failed test or runtime effect occurred.

The host worker's initial fixture retained the queue obligation while invoking
the journal directly. Primary integration changed it to re-admit through the
adapter and complete the new fenced attempt, proving the actual join rather
than claiming the earlier fixture exercised it. One browser launch yields the
original ready handoff. Negative adapter cases retain the full queue unchanged
for missing journal, wrong owner, changed payload and navigation. Pure model
cases cover stale proof, fresh sequence and full-depth immutability.

Unchanged stream, client, schema and store behavior retains v55 evidence;
queue selection and adapter integration were requalified here. The path-based
selector also recommends unrelated workstation fixtures through broad source
rules; no workstation assets, installer or provisioning behavior changed.
The shared installed skill remains untouched under the development boundary.
This is an intermediate provider-free checkpoint, not whole-suite, merge-ready,
or installed acceptance. About 150 elapsed minutes since 02:46 UTC are recorded;
older active effort remains unknown and the 360-minute ceiling is unchanged.
Next is public runtime configuration and keeper policy/catalog resizing, with
interrupted handoff recovery and the full acceptance matrix still open.

## September 21 Durable Admission Queue Amendment

Version 55 continues from qualified source `a9595ee2` and ledger `8e71381a`.
The current worktree is clean at that checkpoint. The next integrated outcome
is durable, bounded admission for ordinary remote opens and manager handoff
access, with exact-request coalescing, generation-fenced completion, deadlines,
and priority with aging. SQLite remains the only persistent queue authority.
The existing host lock remains the browser-effect serialization boundary.

The primary owns daemon integration and the asynchronous admission adapter.
`/root/capacity_selection` owns only the pure Service Model queue and fixtures
on its existing requested sol/medium route. `/root/startup_recovery` owns only
the transactional SQLite document seam and rollback/reopen fixtures on its
existing terra/high route. Effective settings are unknown. No extra worktree,
review discovery, installed publication, or live provider effect is included.
Budget 25 minutes implementation, 15 qualification and 5 custody within the
inherited 360-minute allowance, preserving the recorded 116 elapsed minutes
and unknown older active effort. This does not reset retries or prior review.

Admission must persist before effects. Duplicate operation keys require the
same payload fingerprint and wait for the same result; mismatched replay fails.
At most one admitted queue request may enter the ordinary host effect path.
Pending operations receive recovery priority; existing browser/handoff access
precedes new opens; aged FIFO prevents starvation when capacity is available.
A full display pool must not block an existing-handoff request behind an
ineligible new allocation. Queued deadlines and bounded retained results prevent
unlimited growth. A client cancellation never cancels another duplicate waiter;
abandoned queued entries expire without an executor launching their requests.
After host-generation change, queued entries are retryable and admitted entries
retain an explicit external-effect recovery obligation, never blind replay.

Qualification must cover real SQLite persistence, single-admission and duplicate
results, depth/deadline/priority/aging, generation fencing and interrupted-state
distinctions, then the existing affected runtime tests and strict quality gates.
Public configuration, live keeper resizing, scale-in, active-view control joins,
remaining persistence and complete installed acceptance remain open.

Source `f3d0a758` qualifies this provider-free queue outcome. Ordinary remote
opens and manager handoff resolution enqueue in SQLite before browser effects;
only the exact generation, request sequence, and attempt token may execute or
complete. Duplicate payloads coalesce and replay the prior result, which is not
fresh visibility evidence. The proven keeper startup advances queue generation:
queued work becomes retryable, admitted work retains recovery-required evidence.
The blocking effect task persists its result even when its async caller leaves.
An unstarted permit releases only its exact admission on drop. Safe terminal
records are bounded at 128; recovery obligations are never silently evicted and
new attempts stop at their bound. Status exposes redacted queue counts.

Primary integration corrected two concrete gaps before final qualification:
retrying an old safe entry cannot bypass the recovery-obligation bound, and an
expired waiter cannot claim a same-generation replacement request. The latter
required a sequence check both before and inside the admission transaction and
at effect, completion and cancellation boundaries. Both have regression fixtures.
`/root/queue_docs` completed the narrow four-surface documentation update using
requested luna/medium with effective configuration unknown. All three workers
are terminal. The primary retained integration and acceptance responsibility.

Qualification passes: 308 Service Model tests; final native-other 773 with 57
ignored (56.30 seconds), including all three real-SQLite admission tests;
native-stream 253 (53.42 seconds); host 16, store 26 and output two tests;
formatting and strict workspace Clippy; full client, API/MCP parity, docs build,
links, handoff docs and active planning audit. Logs are
`/tmp/p211-v55-model-final.log`, `/tmp/p211-v55-native-other-final.log`,
`/tmp/p211-v55-{stream,host,store,output,clippy-final,client,parity,docs-build}.log`.
Earlier passing native-other and model receipts are retained separately. The
late sequence guard reopened only affected native admission and strict-quality
checks; unchanged stream, client and document behavior retains its scoped pass.
No Rust test failure, installed publication, browser or provider effect occurred.

This is outcome progress, not full queue recovery or P211 acceptance. Admitted
requests interrupted across restart remain explicitly recovery-required until
the operation-journal reconciliation join is implemented; they are not blindly
replayed. Active-view recovery/control priority still needs the Desktop Services
join. Public live configuration, keeper policy/catalog resizing, reference-free
scale-in, causal browser discovery, quarantine reconciliation, remaining SQLite
and retention domains, and the frozen isolated acceptance matrix remain open.
Next join interrupted queue admission to the existing exact operation journal,
then expose and consume public runtime configuration. Approximately 138 elapsed
minutes are recorded since 02:46 UTC; older active effort remains unknown. No
scope, historical retry count, review allowance or 360-minute ceiling is reset.

## September 21 Capacity Admission Amendment

Version 54 resumes the unfinished capacity changes above source `02ee03a2`
and ledger `1e58ae9f`. Current Git confirms those commits and the uncommitted
worker changes. This packet connects ordinary browser admission to bounded
placement and durable keeper demand: grow before sharing, enforce density,
preserve occupied browsers when limits are lowered, and reuse existing browsers
without consuming another capacity unit. SQLite demand changes must not discard
a concurrent provider receipt or weaken phase and fence conflict checks.

The primary owns integration and qualification. `/root/capacity_selection`
returned the selector using requested `gpt-5.6-sol` medium and now owns narrow
help, documentation, output, and type-fixture synchronization.
`/root/startup_recovery` returned durable demand, exact-field SQLite CAS merge,
and a real repository/reconcile fixture using requested `gpt-5.6-terra` high.
Effective model settings are unknown. No new review pass or worktree is created.

Reserve 20 minutes for integration, 15 for changed-surface qualification, and
10 for custody within the inherited 360-active-minute ceiling. Earlier recorded
94 minutes and unmeasured older effort remain historical, not reset. The current
goal readback reports 2,220,504 tokens and 6,868 seconds with no tool budget;
its thread differs from the historical manually capped second-window thread.
Those counters cannot establish that older window's remaining allowance.
This packet introduces no installed, browser, provider, production, or ingress
effect. Freeze source before compiling; run the Service Model tests, affected
CLI tests, format, strict Clippy, client parity, and documentation checks.

Acceptance requires real model admission and SQLite reconciliation fixtures:
a second browser waits for a second display, demand makes that display Ready,
sharing starts only at the display maximum, and lowering capacity preserves
existing browsers. Status distinguishes unavailable ownership from allocation
pressure. Queue depth, priority, coalescing, durable restart, public configuration,
live keeper resizing, and cooldown scale-in remain part of the full objective.
Passing this packet does not establish those requirements or installed acceptance.

Source checkpoint `a9595ee2` qualifies this intermediate outcome. The model
selects unused healthy displays before sharing, caps density, and refuses new
allocation over lowered targets without moving retained browsers. Ordinary
remote admission publishes bounded additional demand into SQLite and waits
within the request deadline. The supervisor consumes that demand using its
existing reconciliation loop. Concurrent demand-only updates merge into effect
receipt commits; phase, fence, catalog, claims, and competing demand changes
still conflict. The primary additionally validates the merged authority before
persistence. Existing schema-v4 records default missing demand to zero.

Primary qualification passes: Service Model package 299 tests across its unit
and integration binaries; native-other 770 tests with 57 ignored (56.54 seconds);
native-stream 253 (53.10 seconds); 16 browser-host, 25 store, and two output tests;
formatting, strict workspace Clippy, full service-client plus final type fixture,
API/MCP parity, final docs build, links, handoff docs, and repo-local active
planning audit. Logs are `/tmp/p211-v54-{model,native-other,stream,host,store,output}.log`
and `/tmp/p211-v54-{clippy,client,types,parity,docs-build-final,links-final,handoff-docs-final}.log`.
The first planning command used the shared auditor and reported legacy archival
findings; the repository-owned active auditor passes, with both receipts retained.
No Rust test failure or provider retry occurred. The docs build was repeated
because the worker clarified count provenance after the first build started.

This is outcome progress at the provider-free allocation boundary. It is not a
full suite, live-process census, queue implementation, or installed acceptance.
The broad validation selector's lease/installer matches do not indicate changes
to those implementations; their prior scoped receipts remain historical.
Recovery of already observed reserved launches still preserves the exact browser
instead of treating adoption as a second allocation. A racing request is checked
again under the host lock and may receive a typed capacity error; fair requeueing
remains unimplemented. Timed-out demand may still finish warming a route; demand
reduction and reference-free cooldown scale-in remain open. Approximately 116
elapsed minutes are now recorded since 02:46 UTC, with older active effort
unknown. No installed or live provider effect occurred. The next critical path
is durable bounded queue admission and public configuration, then the remaining
recovery, Desktop Services, persistence, and isolated acceptance gates.

## September 21 Supervisor Readiness Amendment

Version 53 continues from source `f651384f` and ledger `c1d7c08f`. Fresh fetch
confirms exact remote parity and open draft PR #191. The handoff matches those
locators; historical test results remain scoped receipts. Two unfinished
worker patches provide a live supervisor probe and the optional status contract.
The current implementation gap is verified: the daemon consumes persisted Ready
records without observing supervisor termination until shutdown.

This consolidated packet joins live supervisor health, host generation, and
fresh SQLite route receipts in ordinary remote admission, manager handoff
resolution, and `service_status.presentationKeeper`. Recovering opens wait
within the existing configured request deadline; failed ownership cannot
advertise ready capacity. Keep legacy `presentationCapacity` semantics explicit
rather than presenting that compatibility projection as keeper readiness.
Full allocation/queue capacity replacement and the broader installed journey
remain open. No runtime, provider, ingress, or release effect is included.

The primary owns the shared projection, fixtures, documentation, validation,
and integration. `/root/startup_recovery` owns only daemon integration after
returning its stream health API; its existing requested route is
`gpt-5.6-terra` high, with effective configuration unknown.
`/root/keeper_contract` returned the schema and generated client patch using
requested `gpt-5.6-luna` medium; effective configuration is unknown. No new
worktree or review discovery pass is introduced. Budget up to 20 minutes for
integration, 15 for affected tests and strict quality, and 10 for custody,
within the inherited 360-active-minute ceiling. The prior approximately 70
elapsed minutes and unknown older cumulative effort are not reset.

Acceptance must show that failed, stopped, missing, or wrong-generation owners
cannot reuse retained readiness; pending recovery can become admissible without
launching before readiness; and service status preserves diagnostics when the
keeper is unavailable. Run changed native compartments, client contract checks,
formatting, strict Clippy, and relevant docs checks. This is an intermediate
source milestone, not complete P211 acceptance or a frozen installed candidate.

Source checkpoint `02ee03a2` qualifies this packet. The supervisor probe detects
failed, stopped, and panicked tasks without consuming their shutdown join;
repeated shutdown preserves terminal health. The daemon joins the probe to a
fresh SQLite snapshot after taking the browser-host lock. Retained navigation
and interrupted-open intents must also select a route from the refreshed usable
set. No stale ready receipt can independently authorize those effects. The
bounded read-only waiter covers recovery-to-ready, immediate terminal failure,
expired deadlines, and an observation that never completes. Ordinary status
adds the redacted typed projection and the CLI prints its effective readiness.

Primary qualification on the final source: native-other 769 passed with 57
ignored (56.25 seconds), native-stream 253 passed (53.06 seconds), all 16
browser-host tests, both status-format tests, formatting, and strict workspace
Clippy pass. Full service-client, API/MCP parity, docs build, documentation links,
handoff docs, and active planning audit pass. Logs are
`/tmp/p211-v53-final-native-{other,stream}.log`,
`/tmp/p211-v53-browser-host.log`, `/tmp/p211-v53-output.log`,
`/tmp/p211-v53-clippy.log`, and `/tmp/p211-v53-service-client-final.log`.
The first compile exposed a fixture import error, corrected before final
qualification. An intermediate stream run failed a self-spawn fixture while a
concurrent rebuild replaced its executable; the final frozen run passed without
concurrent recompilation. Both failed receipts remain in `/tmp/p211-v53-*`.
Two P157 oracle locators still pointed to pre-extraction source. The check now
follows the actual provenance helper call and the shared failure classifier,
including its CLI adapter; the original assertions remain in force and the
full aggregate client gate passes.

The final source milestone is outcome progress, not full plan acceptance.
Unchanged model, workstation, browser transport, and installer behavior retain
prior scoped evidence; no comprehensive or installed result is attributed to
this checkpoint. The broad selector also names installer, lease, and live
streaming checks through module-path matches: those implementation surfaces
were not changed, and this packet introduces no live acceptance or skill
publication. Full capacity allocation/queueing, public live configuration,
cross-platform process discovery, quarantine reconciliation, complete handoff
recovery, remaining SQLite domains, Desktop Services joins, and the frozen
isolated-development matrix remain open. Approximately 94 elapsed minutes have
accumulated since the recorded 02:46 UTC continuation start; older cumulative
active effort remains unknown. The 360-active-minute ceiling and inherited
review/retry history remain unchanged. No installed, provider, browser,
production, ingress, or release effect occurred.

## September 21 Whole-Phase Startup Recovery Amendment

Version 52 continues the same full objective from qualified source `d0fb7abe`
and ledger `50db55d6`, with clean remote parity and open draft PR #191 verified.
The preceding startup packet is outcome progress; it does not satisfy the
remaining cold-start matrix or installed acceptance. The next bounded outcome
is recovery of `Starting`, `Observing`, `RecoveryFailed`, and `Stopping` in the
same configured startup path. Quarantined ownership remains preserved and
explicitly unavailable; this packet does not silently clear that obligation.

Before provider work, classify every record and prove exit of its interrupted
host. A retained ready witness additionally requires exit of its original host.
Pending starts without a witness receive a newer host and operation fence,
then recreate transport and observe real readiness. Pending or failed recovery
with a witness enters adoption without discarding that witness. Interrupted
adoption retains its monotonic operation number even after several recovery
attempts. Pending stop intent is preserved: a separate guarded connector seam
uses the exact durable XRDP witness without creating a tunnel, and publishes
stop completion plus Absent-slot successor rebasing in one compare-and-swap.
Transient errors retain intent; contradictory stop evidence is quarantined.
No old-generation Absent slot is exposed to ordinary reconciliation.

The primary owns the pure Service Model transitions and fixtures, all docs,
validation and custody. The existing `/root/startup_recovery` worker owns the
two CLI keeper modules and their fixtures under that frozen model contract.
Its requested model remains `gpt-5.6-terra`, high effort; effective configuration
is unknown. No new broad review, checkout, or runtime effect is introduced.
Budget this successor within the inherited ceiling: up to 25 minutes for the
parallel implementation, 15 for focused/model and quality checks, and 10 for
integration and documentation. The approximately 54 prior elapsed minutes and
unknown older cumulative consumption are retained, not reset. This packet can
advance source recovery, not finish the whole P211 outcome. Candidate build,
installed matrix, cross-platform browser discovery, readiness/capacity,
remaining SQLite domains, and Desktop Services joins remain separate gates.

Acceptance requires mixed-phase recovery through the real orchestrator,
exact-fence and dual-host refusal without provider access, retained witness
adoption, atomic stopped-slot rebasing, and cancellation/failure preservation.
Run the model integration tests, focused keeper tests and strict quality gates;
retain unrelated passed evidence only with an explicit dependency rationale.
Source checkpoint `f651384f` implements and qualifies this provider-free phase-recovery packet. The worker
returned the CLI classifier, reservation and dispatch paths, configured
retained-stop seam, and mixed-phase fixtures. Primary integration requires an
exact captured record before resumed effects, in addition to fence and phase;
a substituted predecessor receipt cannot reuse earlier process proof. The
primary also added stop-negative cases for unproven ownership, mismatched
stopped receipts, transient errors and changed candidates. These retain stop
intent or quarantine without exposing a successor-owned free slot.

The complete Service Model package and all 18 route-keeper integration tests
pass. The initial 57-test keeper run, docs links, handoff docs, docs build, and
six installer/provider fixtures pass. After the record-binding correction,
CLI native-other passes 765 tests with 57 ignored in 55.08 seconds; native-stream
passes all 250 tests in 53.80 seconds. Workspace formatting and strict Clippy
pass on the final source. Documentation links and the active planning audit
also pass. Local reproducible logs are `/tmp/p211-v52-model-package.log`,
`/tmp/p211-v52-record-bound-native-{other,stream}.log`, and
`/tmp/p211-v52-record-bound-clippy.log`. This is changed-surface qualification
with the earlier comprehensive baseline retained: workstation, service,
browser, transport and integration code are unchanged by this packet, and no
whole-suite or installed-candidate result is attributed to `f651384f`.
The six fixtures
use the existing local debug CLI plus unchanged fixture inputs and are retained
installer-boundary evidence, not final-candidate installed acceptance.
This is outcome progress: configured startup now processes every supported
non-quarantined phase through the same proof-bound path. Roughly 70 elapsed
minutes have accumulated since this continuation began, including prior
qualification; older cumulative active effort remains unknown. The next
outcome is truthful supervisor health/readiness and capacity use, followed by
the remaining broader P211 gates. No optimized candidate or runtime effect
occurred, and no new review or attempt allowance was created.

## September 21 Handoff Verification And Startup Recovery Amendment

Version 51 preserves the complete install, upgrade, and remote-view objective.
The startup packet is an intermediate provider-free milestone, not plan
completion or installed acceptance. Fresh fetch and readback establish clean
P211 custody at `8b7366017cc360752e29571368e150aa45f75efe`, exact remote parity,
and open draft PR #191 with that head. Issues #181, #183, and #195 remain open.
Five checkouts exist; P215 has unrelated dirty Desktop Services work, which
this packet preserves. No new checkout is admitted. P207 remains at its known
head with draft PR #184, so no new overlapping interface publication was found.
Graphiti returned four unrelated historical facts and supplies no current
P211 evidence. The handoff's focused and comprehensive test counts remain
historical receipts until fresh checks run against the changed candidate.

The handoff overstates interrupted cold-start recovery. Checkpoint `45124f99`
reconstructs an `Adopting` proof only when the requested successor generation
already equals the interrupted fence. Its fixture replays that generation;
a genuinely new process registers a newer generation and is refused. The
startup packet must therefore prove exit of both the original ready host and
the interrupted adopter before refencing the same logical adoption to the new
registered host. It must preserve original route evidence and reject stale
adopter terminal events. A same-generation replay alone cannot close this row.

The exact packet performs complete provider-free route classification before
provider access, prepares all required exit proofs, recovers candidates in
stable slot order, and only then enters ordinary supervisor reconciliation.
Unsupported phases, malformed records, live or ambiguous predecessors, and
quarantine remain explicit refusals. Partial initialization must shut down only
its owned primary tasks and persist their terminal evidence without attempting
to stop an unadopted predecessor route. Pending observation is not readiness.
The pure refence transition retains the logical operation number and original
ready receipt while changing the host-bound operation ID; the CLI owns process
proof construction and compare-and-swap publication.

Closed-world review dispositions: `P211-H51-1` is blocking for true interrupted
startup acceptance and is addressed by the two-host exit proof and refence
packet. `P211-H51-2` is blocking for current-state claims; reconcile the
acceptance row and catalog now, and replace them with final candidate evidence
at custody. `P211-H51-3` corrects the handoff effect boundary without expanding
current effects. `P211-H51-4` records unavailable historical effort counters and
observable local bounds; no reset or new broad review is authorized. The
reviewer's proposed documentation-first ordering is not an additional gate:
these amendments proceed alongside disjoint provider-free implementation.

Current work ownership: the primary owns model transitions, model fixtures,
plan/docs reconciliation, Git transitions, and final acceptance.
`/root/startup_recovery` owns only the two CLI keeper modules and their focused
provider-free fixtures, requested `gpt-5.6-terra` high effort.
`/root/plan_review` performs a read-only, closed-world handoff/plan consistency
check, requested `gpt-5.6-sol` medium effort. Effective model identities are
not reported by the runtime. These workers share the admitted P211 lane and
have no independent Git, runtime, or delegation authority.

Source checkpoint `ef43de73` implements the version 51 startup packet;
`7634325d` completes retained route-user validation before provider access and
adds the successful two-route case alongside partial-failure coverage.

Both workers completed. The primary accepted the closed-world documentation
review and its two residual corrections, then integrated the startup worker's
55-test intermediate result. Primary integration made reserved startup state a
separate opaque packet, removed the duplicate asynchronous refence path,
required one whole-snapshot compare-and-swap before handle exposure, and tested
shutdown between route attempts. The primary independently reran all 55
route-keeper tests successfully on the final source; workspace formatting and
strict Clippy also pass. The comprehensive provider-free runner passed both
lanes in 842 seconds on source `ef43de73` with ledger `37dc032e`. The final
`7634325d` delta changes only predecessor-proof validation and its keeper
fixtures. All 55 focused keeper tests, formatting, and strict workspace Clippy
pass again; CLI native-other also passes 761 tests with 57 ignored in 52.16 seconds. Unchanged
Service Model, workstation, integration, and source-free fixture evidence is
retained because that delta does not modify their code or dependencies. This
is combined comprehensive-baseline and focused-final evidence, not a claim
that the entire comprehensive runner ran on `7634325d`. Logs are intentionally
local under `/tmp/p211-full-rust-v51.log` and
`/tmp/p211-route-identity-{tests,clippy,native-other}.log`; the commands and
commit identities here make the checks reproducible. These checks establish
provider-free configured startup recovery, not live provider acceptance.

The startup worker's bounded follow-up found Absent-slot generation rollover
already implemented in host registration. The primary verified the model
transition and CLI CAS loop, then accepted the worker's extension of the
existing mixed-authority fixture. It proves cleared Absent operation fences,
a real successor-generation start, and exact-host replay preserving the full
active authority and two host claims. Whole-authority phase recovery remains
a separate open criterion. Test-only checkpoint `d0fb7abe` passes the focused
registration fixture, formatting, and strict workspace Clippy. Its unchanged
production code retains the prior validation evidence; no broad rerun is
needed for this assertion-only extension.
This continuation has used approximately 54 elapsed minutes through this
checkpoint, including the comprehensive run, with no installed candidate
build or provider attempt. Historical cumulative active effort remains unknown.

Execution inherits the existing 360-active-minute ceiling, three-attempt and
one-review limits, and prior evidence; this revision does not reset them.
This continuation began at approximately 2026-09-22 02:46 UTC. Historical
active-time consumption is not reconstructible from commit timestamps, so
remaining cumulative allowance is unverified. Bound the current startup
implementation to 35 minutes, reserve up to 30 minutes for comprehensive
validation and 15 minutes for integration/documentation, and reconcile effort
before another substantive packet or expensive acceptance cycle. These are
local caps within the existing ceiling, not additional allowance. No optimized
candidate build or installed/provider acceptance occurs in this packet.

The handoff's blanket runtime exclusion describes this source packet. The
canonical Stop Condition separately retains conditional isolated-development
authority after the provider-free candidate freeze and fresh production
readback; it does not authorize production, ingress publication, or release.
The stale active-lane `536d58a9` checkpoint and validation fields must be
reconciled at source custody, preserving that comprehensive result as
historical evidence rather than attributing it to later source.

## Current State

The installed recovery on 2026-09-17 required assisted transaction resume,
manual quarantine of a token-only socket, manual repair of ingress fallback
metadata, direct viewer authentication repair, finalize, and generation GC.
The accepted runtime eventually converged, but the process reproduced issues
#181 and #183 and did not meet unattended-install expectations. Subsequent
browser recovery proved that the named profile and Chrome CDP endpoint could be
healthy while remote view still failed. Prior-boot presentation inventory,
orphaned route and display ownership, and profile-identity proof rejected the
ordinary request. The supported browser-reattach path then terminated the
runtime host and timed out. This is the open #195 acceptance boundary.

At P211 admission, `origin/main@fb616aee` defaulted workstation installation
to the preserve-mode hot transaction. Its full-shutdown alternative required a dry-run plan,
a caller-supplied SHA-256 digest, supervisor-takeover census, exact selected
process identity, and the absence of active drains and transactions. Those
preconditions make the terminal operator action subordinate to stale
coordination state.

P211 was admitted from `origin/main@fb616aee`. Issues #181, #183, and #195 are
the primary outcome items. Issues #189 and #190 are required remote-view
regression subcases under #195, not additional implementation lanes. At
admission, P207 was the primary writer for overlapping CLI help, README, agent
skill, service docs, and generated service contracts. On 2026-09-18, the
operator established that P211 is the only active agent and assigned P211
primary write custody for those shared documentation surfaces. P207 retains
its tab-refresh source, branch history, and feature-specific documentation
intent; P211 will not mutate or discard that checkout. P205 completed as Plan
0216 and merged its service-model extraction
into `main`; the extracted Service Model and Lease Authority crates now own the
pure cold-shutdown state transitions, while the CLI retains repository and
effect adapters.

Graphiti discovery was healthy but returned no source-backed prior decision
for a simple cold upgrade. Plan 0116 and the current hot-upgrade implementation
are advisory history, not constraints on this replacement contract.

Checkpoint `b0ea9cc8` establishes the provider-free cold-shutdown controller.
Its external interface has six fixed phases and accepts no transaction,
admission, census, rollback, hash, or token input. It always executes ownership
release and final verification even when an earlier effect fails. Four focused
tests prove fixed ordering, continuation after failure, foreign-process
preservation, and owned-residue failure. Focused tests, workspace Clippy with
warnings denied, and formatting pass. Platform effect adapters and the public
command remain the next slice.

Checkpoint `b36bf40b` removes the prior daemon-shutdown exception that detached
a live browser when owner-authority metadata was stale. Daemon termination now
always selects browser close. Its focused stale-owner regression, the four
controller tests, strict workspace Clippy, and formatting pass.

P211 merged `origin/main@a3848e16` at checkpoint `0e28e44c` after Plan 0216
landed. The only source conflict was the stale-owner daemon-shutdown regression;
the resolution retains P211's terminal close behavior while using the extracted
Service Model types. Focused validation for
`shutdown_closes_browser_when_owner_authority_is_stale` passes. At that
checkpoint, P207 remained open in pull request #184, so P211 excluded its help
and documentation surfaces.

Checkpoint `6b04975b` adds the first post-extraction shutdown adapter seam.
Lease Authority can now atomically fence and release every active resource
claim while retaining events and fencing high-water marks. Its runtime-owner
kernel can remove all current owners and principal bindings while terminalizing
retained lifecycle history. Service Model joins those operations with session,
viewer-lease, and pending-acquisition release without deleting profile records
or profile paths. The CLI repository adapter commits that pure transition under
the canonical Service State lock. Two focused Lease Authority tests, one
Service Model test, one CLI repository test, formatting, and diff hygiene pass.
Process, user-unit, container, transient-metadata, verification, and public
command adapters remain in Slice 2.

Checkpoint `766cde6b` completes the source-only Slice 2 platform adapter and
public routing boundary. `agent-browser shutdown` now enters the fixed
six-phase controller without accepting transaction, admission, census, digest,
rollback, or target-selection input. The live adapter uses only exact
Service-State browser identities, authenticated daemon lanes, the fixed
workstation user-unit set, and the three fixed Agent Browser Guacamole
container names. Every daemon and subprocess wait consumes the controller's
phase deadline; terminal verification reloads Service State and independently
checks recorded browsers, installed units, running containers, runtime owners,
and active resource claims. Stale close-command errors do not veto a shutdown
whose exact process postcondition is already terminal. Two provider-free
adapter tests prove phase mapping, escalation receipt propagation, and
continuation through ownership release and verification after an effect
failure. All 12 focused shutdown tests, all 118 Lease Authority tests, the
Lease Authority architecture guard, formatting, strict workspace Clippy, and
diff hygiene pass. This checkpoint was pushed without running the shutdown
command or mutating an installed runtime. P207-controlled help and
documentation, cold-install routing, joined restart acceptance, and the
remote-view slices remain open.

Checkpoint `34c6a0e3` adds a black-box CLI integration fixture for the public
`agent-browser shutdown --json` route. The fixture runs the real candidate
binary twice against one disposable home, workstation root, runtime directory,
and socket directory. Its `PATH` contains only fake `docker` and `systemctl`
commands, so it cannot reach host services or containers. Both runs return the
same successful six-phase receipt with zero owned residue and no upgrade
transaction fields. The only subprocess observations are the three fixed
Agent Browser Guacamole container names in the effect and verification phases;
no user unit is inspected when no owned unit is installed. The focused process
fixture, formatting, and diff hygiene pass. Shutdown fixtures for populated,
stale, failed-upgrade, and interrupted states remain open with cold install,
restart, remote view, documentation, and separately authorized installed
acceptance.

Checkpoint `839f8cf8` extends the same black-box boundary with a populated
retained-profile case. The public command changes an exclusive retained
session to `released`, reports zero runtime-owner and active-lease residue,
preserves the named profile record and a physical marker inside its profile
directory, then succeeds without changes on replay. Both disposable process
fixtures pass with formatting and diff hygiene. Active protected authority
claims, interrupted phases, and stale upgrade-sidecar cases remain to be added
before the shutdown acceptance row is complete.

Checkpoint `83e23eb2` addresses the broader valid-profile refusal represented
by `existing_session_profile_identity_inconsistent`, observed most recently by
the SoyLei Website workflow. In trusted single-user shared-local mode, a valid
explicitly requested profile now wins over contradictory retained session and
browser history. The selector returns `ExplicitProfile`, which prevents the
wrong retained browser from qualifying for reuse and sends the request through
the fresh or requalified profile path. Conflicting records remain available
for diagnosis. Registered-capability callers and requests without the
shared-local self-declared identity contract retain their stricter behavior.
The exact regression was red before the change and green afterward; six
existing-session tests, nine shared-local tests, formatting, strict workspace
Clippy, and diff hygiene pass. End-to-end connection proof and a typed
collision-observation surface remain open.

Checkpoint `0bebb42c` establishes the independent provider-free Browser Session
Manager and Browser Profile Catalog seams. The manager admits exact-profile
requests without consulting Lease Authority, principals, owner generations,
boot identity, or legacy Service State. Thirteen focused tests prove one
browser per exact profile, Alice and Bob sharing that browser through separate
named sessions, repeated-command heartbeat refresh, the same session name
opening a different exact profile without an identity-conflict denial,
expiration before a replacement epoch, last-session browser closure, bounded
unresponsive-browser replacement without command replay, bootstrap-tab
adoption, navigation without tab growth, explicit-only tab creation,
most-recent remaining-tab selection, and session-owned tab cleanup on a shared
browser. Five catalog tests prove tolerant field-level profile import despite
malformed unrelated legacy state and malformed sibling profiles. Focused tests,
package Clippy with warnings denied, workspace formatting, and diff hygiene
pass. Filesystem persistence, Service hosting, concrete PID and CDP adapters,
disposable allocation and cleanup, terminal tab history, and CLI routing remain
open.

The checkpoint used three bounded workers after the primary froze the model
boundary. `/root/session_seam_inventory` identified reusable browser process,
CDP, and atomic-store leaf mechanisms while confirming that legacy session and
action-runtime records remain lease-shaped. `/root/display_routing_inventory`
located static route, display-owner, focus, and dashboard projection seams and
confirmed that no least-crowded selector or third route exists yet.
`/root/profile_catalog_inventory` implemented only the catalog importer and its
five tests after the manager gate passed. The primary owned the manager
interface, integrated the disjoint catalog patch, and ran the combined gates.

Checkpoint `d510a054` completes the provider-free persistence and disposable
lifecycle portion of Slice 4. Browser Session State and the Browser Profile
Catalog now use private atomic JSON files that are separate from legacy
`state.json`; the first catalog load imports legacy profile fields once, after
which the independent catalog is authoritative. Repeated replacement uses the
repository's Unix rename and Windows replace-existing patterns. Live tab
removal appends terminal tab metadata, and completed
navigation records retain profile, session, browser, tab, target, URL, and
timestamp attribution. Serialized state resumes the same healthy session and
browser after a simulated Service restart. Disposable intent allocates one
managed profile per named session and policy, reuses it for that session,
isolates another named session, closes its final browser, and deletes only the
recorded managed directory through the reaper after the policy-configured
delay. Exact-profile intent rejects a disposable definition. Seventeen manager
tests, five catalog tests, two CLI store tests, workspace formatting, strict
workspace Clippy, and diff hygiene pass. Service hosting, concrete browser and
filesystem effects, CLI routing, and process-restart fixtures remain open.

Checkpoint `833da2a5` adds the concrete BrowserManager effect adapter without
switching any public route or launching Chrome. One serialized worker owns all
manager instances and maps launch, recorded-PID plus CDP liveness, exact close,
bootstrap adoption, session-initial tab creation, explicit tab creation, and
target close to existing BrowserManager primitives. Disposable filesystem
effects create a private direct child of the configured absolute root and
delete only that recorded child. The multi-session tab contract now handles
the real Chrome bootstrap constraint: Alice adopts the unowned bootstrap; Bob
gets exactly one session-initial tab when no unattributed target remains; and
closing Alice's only tab first provisions Bob's required survivor tab before
physical close. Repeated commands still reuse the session-current tab. Eighteen
manager tests, the focused adapter filesystem test, workspace formatting,
strict workspace Clippy, and diff hygiene pass. The adapter is not yet hosted
by the Service and restart-time CDP reattachment remains open.

Checkpoint `578abe9a` hosts the manager lazily inside the existing runtime-host
daemon and routes the first ordinary public operation through it. An explicit
`--session <name> open <url>` now selects the independent manager when runtime
host admission is enabled. An explicitly selected named profile is exact;
omitting the profile selects the configured disposable policy. The host loads
the independent catalog and state, injects configurable five-minute session
and disposable-cleanup defaults, persists every successful transition, and
serializes browser effects through one BrowserManager worker. Navigation
acquires or reuses the session-current tab, switches the exact Chrome target,
performs the navigation, and records immutable attribution history. An
explicitly named `close` ends only that manager session and leaves a shared
browser alive while another session remains. Name-only close succeeds only
when it identifies one active session; otherwise it returns a typed ambiguity
instead of guessing. Provider-free restart and close, three public-routing,
thirteen CLI host/store/runtime, eighteen manager, and five catalog tests pass
with workspace formatting, diff hygiene, and strict Clippy. No live browser or
installed runtime was used. Concrete process-restart CDP reattachment remains
open because the new BrowserManager worker does not yet adopt a recorded live
endpoint after its own process restarts.

Checkpoint `c3ce1b97` closes the concrete restart gap. A graceful runtime-worker
shutdown now relinquishes its launched Chrome process instead of treating
Service lifetime as browser lifetime. The replacement worker first verifies
the persisted PID, reconnects to the recorded browser-wide CDP endpoint,
rediscovers current targets, and requires a responsive CDP command before
reusing the browser. Final session close sends `Browser.close` through the
reattached manager and requires the recorded PID to disappear within five
seconds. If a live recorded process cannot be requalified through its recorded
endpoint, the worker reports it as nonresponsive but will not claim it was
closed or launch a competing profile browser after an unproven close. A real
ignored acceptance test launched Chrome against a disposable profile,
relinquished it with the first worker, reattached from persisted identity in a
second worker, and closed the exact process. The focused restart test, fourteen
CLI browser-session tests, formatting, strict workspace Clippy, diff hygiene,
and a fresh residue readback pass; the readback found no remaining fixture
browser process.

Checkpoint `8c9b181c` implements the first simple presentation calculation and
joins it to browser launch. The provider-free selector accepts only configured
desktop routes and current live-browser display placements. It excludes `:0`,
excludes routes whose startup health probe failed, chooses the smallest live
browser count, breaks ties by configured route order, and rejects duplicate
route or display identities. Retained route leases, prior allocations, and
display-owner history are not inputs. The Service host derives candidates from
the existing RDP route inventory and display-socket probe. The manager stores
the chosen route, display, and selection count on the independent browser
record; the BrowserManager adapter launches headed on that exact display.
Four selector tests and a nineteenth manager lifecycle test prove `:10` then
`:11` assignment for distinct profile browsers, deterministic ties, local and
unhealthy exclusion, and duplicate rejection. Fourteen CLI browser-session
tests, formatting, strict workspace Clippy, and diff hygiene also pass.
Dashboard projection, focus-on-selection, and durable remote-view handoff are
still open.

Checkpoint `26743c65` makes independent Browser Session State visible without
making legacy state authoritative. Service Status now adds a top-level
`browserSessionState` snapshot from the live host or independent atomic store.
An unreadable snapshot adds `browserSessionStateError` while preserving the
successful legacy status response. The dashboard converts those browser,
session, tab, desktop, and navigation records into its existing read model and
merges by stable ID. One concrete browser process produces one live parent
tile; named sessions and tabs contribute attribution and counts rather than
extra browser tiles. Latest navigation history supplies the tab URL. Two Rust
status-join tests, the dashboard workspace-node smoke, selected-context,
workspace-view, and navigator tests, the production dashboard build,
formatting, strict workspace Clippy, and diff hygiene pass. The tile has its
configured route and display identity, but focus-on-selection and a ready
durable handoff remain open.

Checkpoint `a4c08010` routes the explicit tab-growth operations through the
same public named-session boundary. `--session <name> tab new [url]` creates
exactly one explicitly attributed tab, optionally navigates that tab, and
records its history. `--session <name> tab close` closes only the session's
current tab and selects its most-recent remaining tab through the existing
manager rule. Indexed tab close stays on the legacy path because the simple
prototype does not reinterpret a legacy numeric index as attributed identity.
The host restart fixture now proves open, explicit new tab, navigation,
current-tab close, and final session close in sequence. The focused host and
public-routing tests, formatting, strict workspace Clippy, and diff hygiene
pass.

Checkpoint `a9d3887e` activates the Browser Session Manager's existing bounded
cleanup policy instead of requiring an operator command to trigger it. Each
Unix and Windows runtime-host server starts one lazy reaper on the existing
Service reconciliation cadence, defaulting to 30 seconds. The task does not
initialize an unused Browser Session host, skips missed ticks rather than
bursting, and is aborted and joined when the server exits. Once the host has
been used, each tick expires idle sessions, closes newly sessionless browsers,
and applies the manager's exact disposable-profile eligibility rules. The
focused interval test, all 17 executed CLI browser-session tests, formatting,
strict workspace Clippy, and diff hygiene pass; the disposable real-Chrome
restart test remains intentionally ignored in this provider-free lane.

Checkpoint `f5a0c287` connects manager-owned browser tiles to the established
dashboard focus request without changing the public action contract. The
runtime host claims `view_focus` only when its stable browser ID exists in
independent Browser Session State; foreign and legacy IDs continue through the
legacy handler. The manager proves current PID and CDP liveness, selects the
attributed target when supplied, refreshes that session heartbeat, and uses
BrowserManager's existing bring-to-front and native maximize operation. A
missing target is rejected rather than redirected to another tab. All 20
manager lifecycle tests, the host restart/focus fixture, the manager-ownership
routing test, all 17 executed CLI browser-session tests, formatting, strict
workspace Clippy, and diff hygiene pass. Static viewer lookup and durable
handoff issuance remain open.

Checkpoint `4177d983` completes static viewer selection for manager-owned
browser tiles. The dashboard treats the persisted desktop `routeId` as the
configured route-pool entry identity, resolves that entry's concrete route,
and attaches the already-published Service view stream to the browser read
model. The join is read-only, deterministic, and preserves the route-pool
entry identity for diagnostics; it neither checks out a legacy lease nor
constructs a provider URL. Workspace nodes, view projection, selected-context,
and navigator tests plus the optimized dashboard build pass. The tile can now
select its desktop viewer and route focus through the manager; creation of a
ready durable opaque handoff for a newly opened manager browser remains open.

Checkpoint `95b5758f` routes both ordinary workstation apply and reviewed
candidate apply through the concrete cold controller before any prior upgrade
transaction convergence. Stop executes the public owned shutdown adapter;
replace stages and atomically selects one immutable generation; start performs
transaction-free workstation reconciliation; readiness proves the selected
generation and binary digest, then runs the final doctors on a real host.
Reviewed-candidate artifact identity is stored in the immutable generation
manifest and contributes to its generation identity without creating a hot
upgrade transaction. A black-box isolated apply succeeds with contradictory
retained transaction and admission-drain files present, emits only stop,
replace, start, and readiness, and leaves those diagnostic files untouched. A
second injected selector-commit failure executes one rollback and restores the
exact prior selector. The four controller fixtures, both reviewed-candidate
tests, both shutdown process fixtures, formatting, strict workspace Clippy,
and diff hygiene pass. A broad 181-test installer filter completed before the
final success-return refactor; the two directly affected reviewed-candidate
tests were rerun after that refactor. Filesystem replacement is not yet an
interruptible operation, so hard deadline enforcement and installed readiness
acceptance remain open.

Checkpoint `c754180f` routes ordinary tab-bound commands through the Browser
Session Manager after a simple named-session open. Each managed browser now
retains one full command state beside its BrowserManager, preserving snapshot
references and other per-session command context across separate CLI calls.
The runtime selects the session's attributed current target before each
command, refreshes the session heartbeat, and bypasses only the legacy
runtime-owner, admission-drain, manual-seeding, and profile-mismatch gates that
the independent manager replaces. A manager-owned state cannot fall back to
legacy auto-launch when its browser is unavailable. Browser and tab lifecycle
operations remain manager-owned rather than entering the generic dispatcher.
The host fixture proves exact Alice routing and Bob noncapture. A disposable
real-Chrome restart fixture proves PID/CDP reattachment, navigation, ordinary
title and snapshot calls, and exact browser close. All 19 executed focused
browser-session tests, the one opt-in real-Chrome test, formatting, strict
workspace Clippy, and diff hygiene pass. Durable opaque handoff publication
and a full daemon-process client journey remain open.

Checkpoint `ca74617e` publishes and resolves a durable opaque handoff for a
manager-owned browser without creating a route lease or consulting legacy
browser ownership. A successful managed navigation on a ready assigned static
desktop persists one stable UUID handoff, returns only the authenticated
dashboard `/remote-view/<handoff-id>` URL, and updates the desired URL while
retaining that handoff across later same-session navigation. Resolution
revalidates the live session record, heartbeat, exact PID, responsive CDP
endpoint, attributed target, static display binding, route-pool readiness, and
route readiness before raising the browser. A stale proof returns a terminal
typed unavailable response and never enters the dashboard's automatic retry
loop or falls through to legacy retained-browser adoption. The dashboard
accepts the manager-specific receipt from its
authenticated Service response while preserving the stricter legacy receipt
checks for legacy handoffs. Both manager handoff fixtures, all 19 other
executed browser-session tests, all 14 durable-handoff regressions, the route
confusion gates, dashboard durable-handoff, workspace-navigator, and inspector
action smokes, the optimized dashboard build, formatting, strict workspace
Clippy, and diff hygiene pass. A real login redirect and full
daemon-to-dashboard acceptance remain open.

The 2026-09-18 design interview generalized that repair into the first Browser
Session Manager prototype. One Service process owns browser-session decisions;
`--session` is the caller-visible attribution identity rather than a daemon
routing alias or authenticated principal. Multiple named sessions may share
one browser for one exact named profile. Exact-profile requests compare
profile identity, while disposable requests compare allocation policy and
reuse only the current compatible allocation for that named session. Activity
refreshes a configurable five-minute idle heartbeat. A session is live only
while that heartbeat is fresh, its browser PID exists, and its CDP endpoint
responds. The last ended session closes its browser, and a proactive reaper
removes eligible disposable profile data. Browser Session State and the Browser
Profile Catalog are independent of legacy Service State so retained lease or
owner records cannot veto current work.

The same checkpoint simplified presentation. Configured Guacamole routes map
directly to virtual desktops: Guac A to `:10`, Guac B to `:11`, and a future
third route to `:12`. A remotely viewable browser launches on the healthy
virtual desktop with the fewest live browsers; `:0` is reserved for explicitly
local on-screen browsers. Dashboard active tiles represent concrete browser
processes. Selecting a tile chooses the viewer for that browser's desktop and
raises its primary browser window. Tree expansion from browser to sessions to
tabs remains a later UX improvement.

The tab follow-up closes the accidental-growth loophole. Each session has at
most one current tab. Its first tab-requiring command adopts an unattributed
Chrome bootstrap tab when one is available; otherwise it creates exactly one
session-initial tab. Ordinary `open` and navigation commands then reuse the
current tab; only an explicit new-tab operation increases the tab count within
that session. Closing the current tab selects the session's most recently used
remaining tab, or leaves the session tabless until another command needs one.
Session termination closes its live tabs, while historical tab metadata
remains queryable. The prototype records tab-count and cleanup telemetry but
does not add a tab cap or silent least-recently-used eviction.

The 2026-09-18 execution readback synchronized the P211 branch and its draft
pull request #191 at `81699174` before implementation. P207 remains open in
pull request #184 and retains help and documentation custody. P211 therefore
keeps this packet in new lane-owned modules until those dependencies integrate.
Graphiti was healthy but returned only older remote-view history. Current
source, focused tests, and Git evidence remain authoritative.

## Second-Window Review And Remediation Freeze

The second execution window started from clean local and remote candidate
`6f8dccd112be3c10d72e75455d53fdf3c371d8f1` against
`origin/main@a3848e16f75769f31e6855a7e77f9ce9bdb8967f`. Pull request #191 remained
open and draft. Pull request #184 remained open and draft at `838b771a`, so
P207 still owns overlapping help, README, skill, generated-client, and service
documentation surfaces. CodeGraph was not initialized in this worktree and
was not initialized without operator authority. Graphiti was healthy but had
no Plan 0211-specific source-backed recall.

The operator authorized one additional execution window capped at 2,000,000
tokens. Goal-control thread `01a0b5a3-edfc-7a91-94de-e6bf2f1804e3` did not
expose a writable token-budget field, so this plan records the cap as the
manual hard ceiling. The prior window consumed 1,988,143 tokens and 11,388
seconds. The new allowance does not reset the one-review limit, retry counts,
evidence history, or the 360-active-minute provider-free ceiling. This window
can finish the provider-free source objective if the accepted remediation and
P207 documentation join fit the remaining bounds. It cannot establish the
separately authorized installed clean-install and replacement-upgrade result.

The fresh review was one parallel two-axis pass. The primary adjudicated every
candidate as follows:

| Finding | Disposition | Adjudication |
| --- | --- | --- |
| `P211-S1` | `blocking` | `presentation_requalification.rs` has no production caller and duplicates the presentation design. Delete the unused module and its isolated tests. |
| `P211-S2` | `blocking` | Public shutdown, manager behavior, and configuration are not documented on all mandatory surfaces. Perform the parity update only after P207 overlap custody is clear. |
| `P211-S3` | `needs_evidence` | RUNBOOK and the active-lane entry are stale, but both are shared surfaces currently overlapping P207 and P214. Reconcile one writer before refreshing them. |
| `P211-S4` | `nonblocking_backlog` | Duplicate controller cases and test-harness setup raise maintenance cost but do not falsify current behavior. Consolidation is outside this remediation cycle. |
| `P211-R201` | `blocking` | Shutdown and residue verification omit independent manager state, while runtime-host teardown relinquishes manager browsers. Add exact manager-process custody, terminal state transition, and replay proof. |
| `P211-R202` | `blocking` | One `DaemonState` per browser shares refs, frame state, confirmations, and request state across named sessions. Separate session command context from browser lifecycle custody. |
| `P211-R203` | `blocking` | Managed navigation preserves `success: true` when required handoff publication fails. The ordinary ready-remote-view operation must fail as a whole when its handoff is unavailable. |
| `P211-R204` | `blocking` | Managed handoff readiness currently treats retained dynamic pool and allocation fields as authority. Bind publication and resolution to the configured static viewer plus current browser, display, route, and control observations without importing legacy leases. |
| `P211-R205` | `blocking` | Handoff resolution bypasses manager serialization, does not persist heartbeat activity, and can race close or reap. Route it through the live host and its existing persistence lock. |
| `P211-R206` | `blocking` in part | Hard-coded ready/active projection from an unreconciled persisted manager record is invalid. Preserve legacy browser history as a separately classified compatibility source; reject the candidate suggestion to remove all legacy rows from the dashboard. |
| `P211-R207` | `rejected` | The exact catalog ID is the profile selector; `name` is a display label. Adding display-name alias selection would weaken the frozen exact-profile rule and create new ambiguity behavior. |
| `P211-R208` | `blocking` | Several shutdown, replacement, start, readiness, rollback, filesystem, and lock effects only report a deadline without enforcing it. Add injected bounded execution at the effect seam. |
| `P211-R209` | `rejected` | Indexed close and arbitrary tab switching were explicitly deferred. The prototype owns explicit new-tab and current-tab close only; unsupported indexed operations must not be reinterpreted. |
| `P211-R210` | `needs_evidence` | The final daemon, dashboard, shutdown, redirect, and joined install fixtures are still absent. This is the acceptance-evidence packet after source repair, not a separate defect. |

The internal `Converging` variant carrying terminal `unavailable` data and the
constant managed presentation generation are accepted as nonblocking cleanup
inside the handoff repair only if removing them simplifies the joined
interface. They do not justify a new generation or recovery subsystem.

The single remediation batch is frozen in this order:

1. Add one red public shutdown fixture for a manager-only browser, then persist
   exact process identity, close or safely preserve it, terminalize manager
   sessions, verify independent-state residue, and prove idempotent replay.
2. Add one two-session command-context regression, then split session-owned
   command state from the shared browser transport and lifecycle owner.
3. Add red handoff publication, close-race, heartbeat, and current-readiness
   fixtures, then route publication and resolution through the serialized host
   and one current configured presentation observation.
4. Add injected non-completing effect checks, then enforce the existing phase
   deadlines without creating a second transaction or rollback design.
5. Delete the unused presentation-requalification module and retain only tests
   that cross a production interface.
6. Run the missing daemon-process, dashboard handoff/control, same-site login
   redirect, shutdown-state, and joined fresh-install/restart fixtures against
   one frozen provider-free candidate.
7. After P207 custody clears, synchronize CLI help, README, Agent Browser skill,
   docs site, inline docs, RUNBOOK, and the P211 active-lane record. Run the
   changed-surface selector, required format and strict Clippy gates, focused
   contracts, and one broad provider-free lane once at the final batch.

Budget reservation for this window is 12 percent review and adjudication, 43
percent implementation and red-green fixtures, 25 percent selective and broad
validation, 12 percent documentation and P207 reconciliation, and 8 percent
publication and closeout. If accepted remediation consumes the documentation
or final-validation reserve, stop lower-value cleanup and report the exact
provider-free gate reached. No broad review reopens after this freeze;
verification is closed-world against the accepted findings and regressions
introduced by their remediation.

The version 28 remediation checkpoint implements the frozen source repairs.
Shutdown now treats independently persisted Browser Session Manager processes
as owned only when their exact process identity verifies, terminalizes the
manager state after exit proof, preserves named profile data, survives replay,
and leaves an unreferenced foreign process running in the public command
fixture. Browser command state is owned per named session while lifecycle and
transport custody remain shared per browser. Managed handoff publication now
requires one configured static viewer plus current display, route, control,
browser, session, and tab evidence; publication failure fails navigation.
Resolution runs under the manager host lock, focuses the exact target, and
persists the session heartbeat. Status reconciles process and CDP liveness
before projection, and the dashboard treats current manager state as the
active inventory when that state is available. Shutdown and cold-install
controllers detect injected deadline overruns after every effect boundary;
the measured filesystem replacement allowance is 60 seconds and the other
phase allowances remain 30 seconds or less. The unused presentation
requalification module is removed.

The external-process Chrome fixture then exposed one additional routing defect
inside the already frozen `P211-R210` evidence packet. A mutating command such
as `click` for an existing manager session entered the legacy prestart launch
path and was rejected as duplicate profile pressure before the daemon could
route it to the manager. The CLI now detects a persisted live named manager
session before prestart, sends its ordinary command directly to the shared
runtime host, and fails rather than falling back to a legacy launch if that
host is unreachable. The ignored Linux fixture crosses the compiled CLI and
runtime-host daemon with disposable Xvfb and Chrome. Alice and Bob share one
named-profile browser while retaining distinct sessions, tabs, targets, and
independent `e1` snapshot references; Alice's close preserves Bob and Bob's
final close terminates the exact browser process. The same fixture follows a
real loopback HTTP redirect from `/protected` to `/login`, then reaches
`/account` on Alice's existing tab while retaining the same opaque handoff ID
and leaving Bob unaffected. A fresh process census found no fixture browser or
daemon residue.

Focused receipts at this checkpoint are 25 executed browser-session tests with
two explicit Chrome tests excluded from the default filter, the explicit
external-process Chrome/Xvfb fixture, three public shutdown process fixtures,
231 stream tests, three CDP transport tests with compiler caching disabled,
the dashboard durable-handoff contract, and diff hygiene. The stream rerun also
replaced a stale exported-dashboard assertion: current `main` server-renders
the stable dashboard shell and `Restoring session` before client
authentication, so the route test now checks the shell plus injected section
rather than later client-only login copy. Full authenticated dashboard render
and control, joined cold-install restart use, documentation parity, broad final
validation, and separately authorized installed acceptance remain open.

The version 29 validation checkpoint freezes the remediated source candidate.
The first comprehensive runner invocation lost its controlling session during
the serial workstation compartment after recording green results for the
native action, native service, native other, and CLI core compartments. The
unfinished and supporting lanes were then rerun against the unchanged Rust
source: all 214 workstation tests, all CLI integration tests, 99 native browser
tests with two explicit Chrome tests ignored, 118 Lease Authority tests, the
complete 221-test Service Model unit lane and its integration binaries, and
the candidate, challenge-control, desktop-services, and CDP transport crates
passed. The 231-test native stream lane had already passed against the same
source. Strict workspace Clippy, formatting, the dashboard production build,
dashboard action and durable-handoff contracts, route-confusion gates,
workstation host and VM harness contracts, Guacamole and PostgreSQL fixtures,
service API and MCP parity, generated-client checks, the cold-install fixture,
and the Service collection no-launch smoke also passed. This is final
provider-free validation for the frozen remediation batch, not proof of the
still-open authenticated dashboard-control, joined install-to-use,
documentation, or installed-runtime acceptance gates.

The version 30 continuation preserves the bounded authenticated-dashboard
attempt without promoting it to acceptance. Manager handoff resolution now
returns the dashboard's required compatibility presentation generation and an
exact ready receipt binding the logical browser, target, required and observed
stream provider, and Browser Session Manager source. Generation `1` is a
compatibility sentinel, not lease, owner-generation, or monotonic authority.
The focused handoff-resolution test, dashboard workspace-node projection
smoke, formatting, strict workspace Clippy, all four nonignored workstation
shutdown integration tests, and diff hygiene pass.

Three bounded read-only workers supported this continuation without source,
Git, runtime, or acceptance authority. `/root/p211_efficiency_audit` used the
requested `gpt-5.6-luna` low route to assess the prior attempt economics;
`/root/p211_plan_audit` used `gpt-5.6-terra` medium to reconcile the plan and
lane catalog; and `/root/p211_validation_audit` used `gpt-5.6-luna` medium to
select the cheapest discriminating checks. The runtime did not expose their
effective model identities. The primary accepted the bounded state and test
evidence, retained integration and acceptance, and rejected the plan worker's
broader joined-install next step because the fresh handoff made this preserved
dashboard fixture the immediate packet.

The existing compiled-CLI, runtime-host, Xvfb, and Chrome fixture was extended
to authenticate through the real dashboard endpoint, reopen Alice's same
opaque handoff, and inspect the rendered workspace control viewport against a
static fake viewer. Its static Service State now includes the required
route-pool-entry to concrete-route mapping. The one permitted replay produced
no terminal verdict: after more than three minutes it still owned its isolated
daemon, two Chrome profiles, and Xvfb process, so the run was cancelled and
only those exact task-owned processes were stopped. No second browser replay
was started. The failed fixture directory remains at
`/tmp/agent-browser-workstation-shutdown-71248-1789766339538183572` as
ephemeral diagnostic evidence.

A new focused regression then proved the fixture's local auth server could
block teardown indefinitely after accepting an idle browser connection. That
test failed before the repair and passed after accepted connections received a
100-millisecond read timeout. This repairs evidence visibility but does not
reconstruct the hidden dashboard assertion or prove authenticated rendering
and control. Those acceptance claims remain open. One initial focused Cargo
invocation also failed before compilation in the optional compiler-cache
wrapper; the same manager test and subsequent Rust gates passed with compiler
caching disabled.

The version 31 continuation closes the remaining provider-free input variants
for the public shutdown fixture without changing production logic. A compiled
CLI case now persists an active protected profile claim, invokes
`agent-browser shutdown --json`, proves zero active claims in both the receipt
and reloaded Service State, and proves an unchanged successful replay. A second
case retains contradictory failed-upgrade transaction and admission-drain
sidecars while proving that they neither veto shutdown nor enter its receipt.
A third case represents a prior shutdown interrupted after ownership release:
the session is already released, profile data remains present, and stale
session-stream and dashboard PID metadata remain. The next public shutdown
removes only that transient metadata, preserves the profile marker, and then
replays without changes. All seven nonignored workstation shutdown integration
tests pass; the disposable Chrome/Xvfb browser journey remains intentionally
ignored in this provider-free batch. Formatting passes. Installed shutdown
acceptance remains open and no installed or shared runtime was mutated.

The version 32 continuation closes the remaining injected-clock platform
timeout evidence. The real platform-effects adapter is composed with the
controller's injected clock while its browser phase consumes the exact
controller-owned allowance. The resulting receipt attributes the browser
deadline overrun, retains the platform's changed and escalation evidence, and
still records ownership release, transient cleanup, final verification, and
zero residue. This is a deterministic no-sleep fixture; concrete daemon,
systemd, and container subprocess effects remain independently bounded by
their existing adapters.

The version 33 continuation joins a disposable fresh cold install to first
Service use without launching Chrome or mutating an installed runtime. The
fixture executes the selected installed binary, starts one runtime host and
its embedded dashboard, receives HTTP 200 from the dashboard authentication
status endpoint, completes two fresh Service status requests, and proves that
the repeated dashboard and Service calls retain the same runtime-host process
identity and stream port. The runtime host's recorded executable resolves to
the selected installed generation. The installed binary then performs a
successful cold shutdown, removes its process metadata, and terminates that
exact process. Both cold-install integration tests pass together, and a fresh
process and filesystem readback finds no fixture residue. The first draft of
the fixture correctly unwound through its shutdown guard but failed because it
expected separate dashboard PID files; the dashboard on this path is embedded
in the runtime host. The repaired assertion uses the reachable dashboard
endpoint as the logical dashboard identity. The fixture also makes the
cold-install harness remove read-only generation directories during cleanup.

The version 34 continuation closes two manager-host lifecycle gaps. The hosted
reaper now has an end-to-end provider-free fixture across the host,
persistence, manager, and concrete filesystem adapter: an ended disposable
session loses only its recorded direct-child profile directory, a foreign
sibling and marker remain, and the persisted disposable allocation is gone.
The external unresponsive-CDP path now falls back from a typed CDP attach
failure to bounded exact-process termination only when the retained process
identity opens a kernel-backed termination capability and its PID matches the
browser record. The red fixture first failed with
`browser_session_reattach_failed`; after repair, a live disposable external
process with a deterministic HTTP 503 CDP endpoint is terminated and the
adapter can complete manager retirement. A paired negative fixture proves the
same CDP failure preserves a live process when exact identity is absent. An
initial loopback-refusal draft exposed the adapter's long connection timeout
and was cancelled; its process left no residue, and the deterministic local
503 fixture replaced that tactic.

The version 35 continuation closes hosted multi-display persistence and the
remaining legacy-containment display path. A host-level fixture imports only
two valid named profile definitions from a legacy Service State whose session,
runtime-owner, and display-allocation fields are deliberately contradictory.
Two independent browser opens select the least-crowded healthy routes in
configured order, assigning `:10` and then `:11`. A restarted host reloads the
same independent browser and desktop assignments, while the legacy diagnostic
file remains semantically unchanged and never participates in selection.

The version 36 continuation closes Service startup joining for the independent
Browser Session State and Profile Catalog. `service status` remains a local
no-launch read when no daemon exists. When the selected runtime host is already
ready, the same command now joins that host instead of bypassing it, so status
is serialized with the live Browser Session Manager. The joined disposable
cold-install fixture first failed with a missing `browserSessionState`; after
the routing repair it returns `browser-session-state.v1`, persists
`browser-profile-catalog.v1` with the default disposable policy and no invented
named profiles, retains the same host identity and dashboard endpoint on a
second read, and shuts down without process or filesystem residue.

The version 37 continuation bounds recorded-browser CDP reattachment itself.
The earlier loopback-refusal diagnostic showed that the generic WebSocket
handshake could outlive the manager's recovery budget before exact-process
fallback was reached. The manager-specific reattach path now wraps connection,
target discovery, and responsiveness verification in one five-second
production deadline. A deterministic stalled-handshake fixture injects a
50-millisecond allowance and receives the typed
`browser_session_reattach_timeout` result in under 500 milliseconds. Ordinary
CDP connection behavior outside manager recovery is unchanged.

The version 38 checkpoint freezes the completed continuation at its current
provider-free boundary. The repository selector chose the focused tier from
checkpoint `8ee0d806`: formatting, strict workspace Clippy, active planning
audit, diff hygiene, the source-free workstation-install fixture, workstation
host-provision contract, fresh-workstation VM harness, Guacamole asset and
PostgreSQL durability contracts, and route-specific Guacamole user-sync
contract all pass. The compartmented Rust runner's `browser_session` filter
passes 30 tests; only the two explicit disposable-Chrome cases remain ignored
there, and their dedicated acceptance evidence predates this continuation.
The current worktree and remote branch are synchronized with no uncommitted
changes before this plan-only checkpoint.

Three gates remain intentionally outside this provider-free continuation.
P207 PR #184 is still draft, merge-conflicted, and owns the overlapping help,
README, skill, docs-site, and generated-client surfaces. The authenticated
dashboard rendering/control replay exhausted its bounded attempt and cannot be
reconstructed from the repaired teardown regression. Production or staging
install, shutdown, provider, credential, and installed acceptance effects
still require explicit live-effect custody. None of those gates is promoted to
success by the provider-free receipts above.

The version 39 continuation closes the provider-free doctor disagreement for
ordinary Browser Session Manager handoffs. Before the repair, a ready opaque
manager handoff could be selected and resolved through its static route while
`doctor remote-view --session <name> --runtime-profile <profile>` returned
`unavailable`: the doctor consulted only legacy session bindings and treated a
stale legacy route-pool allocation as authority. The new requested-scope join
accepts only an exact ready manager handoff whose opaque URL, concrete route,
display, configured static route entry, profile, browser, and latest
`operatorVisible.state=ready` resolution agree. It then reports that same
session, profile, browser, display, and route as ready without promoting the
legacy allocation state. Missing or non-ready manager resolution still fails
closed. The regression was red with `unavailable` before the repair and green
afterward; all 54 remote-view doctor module tests, strict workspace Clippy,
workspace formatting, diff hygiene, and every focused workstation contract
selected for the changed doctor surface pass. No installed or shared runtime
was inspected or mutated. Installed doctor agreement and authenticated
dashboard rendering and control remain open.

The version 40 continuation records the operator-directed documentation
custody transition. Fresh readback proved P211 and P207 clean and synchronized
at `4843c4e2` and `838b771a`, respectively. Pull request #191 remained open,
draft, and clean. Pull request #184 remained open, draft, and conflicting.
Direct diff review showed that P207's pending user-facing prose documents its
tab-handle refresh custody behavior in CLI help, README, the Agent Browser
skill, and the service-mode page. P211 now owns edits to the shared files for
the cold install and remote-view journey. It must preserve P207's feature
intent for later source integration and must not import that prose ahead of
its unmerged implementation. This transition grants no custody over P207's
source branch and no installed or shared-runtime effect authority.

The version 41 checkpoint `3758f8df` completes repository documentation
parity under that transferred custody. CLI help, README, the Agent Browser
skill, installation and remote-view docs, and the workstation install module
documentation now lead with the fixed cold apply, idempotent shutdown, and
ordinary session-plus-profile remote-view journey. Existing transaction,
census, handoff, and explicit route controls are labeled as legacy recovery or
advanced compatibility surfaces. Compiled help readback, remote-view
documentation contracts, documentation links, the production docs build,
workstation fixture suites, the 181-test focused Rust lane, formatting, and
strict workspace Clippy pass. The goal-scoped planning audit passes. The
repo-wide active-plan audit remains red on pre-existing historical plan wiring
and state findings and reports no P211 finding. The shared user-scoped skill
was not overwritten from this experimental checkout.

The version 42 continuation records the operator-authorized isolated
development-runtime acceptance and its first installed blocker. Candidate
SHA-256 `8682d47487252e77e07c7eeeff28f97b5ff1b4b069673c14f1f6efb2bd5e4534`
installed as generation `0.28.0-8682d4748725`; all development units became
ready, the development pseudo-home skill matched repository source, and the
installer's before-and-after guard proved production unchanged. Provider
planning, staging, and preflight passed with reviewed public origin
`https://agent-browser-dev.ecochran.dyndns.org`, Cooper revision
`e70368ddbb2e61ae26a25072975c2953754b7479`, and binding digest
`4f24eefcac1008871c90c8e41804029aff8747c9ee0bc13ad7ebe58ad0539c4d`.
The first deferred-ingress apply then quarantined at request `r488783` with
`service_tab_target_unproven`; its durable receipt is
`~/.local/share/agent-browser-dev/presentation-provider/receipts/apply-1789782627538-20403.json`.
No blind retry is permitted. Source diagnosis proved that the provider's
header-bearing initial navigation was excluded from Browser Session Manager
routing, which launched a legacy browser while independent manager state
remained empty. The accepted remediation keeps arbitrary launch arguments
excluded, removes the provider's redundant `--no-sandbox` override, and sends
the required `Remote-User` header through the manager-owned tab. A public
routing regression was red before the repair and is green afterward; a host
fixture also proves that the header reaches the managed command executor.
One new candidate build, development install, and provider apply are allowed
only after focused, formatting, strict Clippy, and changed-surface validation
pass. The original quarantine receipt and exact process residue remain
evidence until replacement install or exact task-owned cleanup proves them
gone. Production and shared user-scoped runtime effects remain excluded.

The version 43 continuation records the first repaired candidate and the
second bounded provider blocker. Source `8e31daf2` built as development
generation `0.28.0-c40bd61ec18f` with SHA-256
`c40bd61ec18f3c63f8e63c97a0936219f682e9af85529f4be9b40b19372d7601`.
Replacement install removed the exact Chrome residue from the first
quarantine, restored all three development units, kept the development skill
current, and again proved production unchanged. The development browser smoke
then exposed a stale pre-manager harness assumption; checkpoint `2d0eb8ca`
now uses disposable manager sessions, accepts only the typed missing-desktop
visibility result while the optional provider is absent, and proves three
open, URL-read, exact-close, no-process-residue iterations with production
unchanged. The second provider plan, stage, and preflight passed, but the one
post-fix deferred-ingress apply quarantined on request `r152796` with
`CDP command timed out: Page.navigate`. Its receipt is
`~/.local/share/agent-browser-dev/presentation-provider/receipts/apply-1789783822885-83753.json`.
The manager recorded and exactly closed the route-1 browser, which proves the
first routing defect stayed repaired. Diagnosis found that attached
session-command state enabled Fetch interception for origin headers without
starting its paused-request consumer. The accepted repair initializes event,
Fetch, and dialog handlers for each manager-owned command context. The ignored
real-Chrome restart fixture now proves a header-bearing navigation after
manager reattachment reaches a local HTTP server with `Remote-User: operator`
and closes the exact browser. Two of the plan's maximum three provider attempts
have been consumed. One final candidate build, replacement install, preflight,
and apply are allowed after focused and strict validation; another quarantine
ends the live acceptance packet without retry.

The version 44 checkpoint closes this bounded development-runtime acceptance
packet without accepting the installed journey. Final source `975147a3` built
as generation `0.28.0-fae441ed9a8f` with SHA-256
`fae441ed9a8f8b324a1cbffb69e606e8e30b128b2cf200437708970bc1a794d3`.
Replacement install and the final plan, stage, and preflight succeeded with
production unchanged and no retained viewer process. The third and final
deferred-ingress apply quarantined at request `r97691`; its receipt is
`~/.local/share/agent-browser-dev/presentation-provider/receipts/apply-1789784327708-12563.json`.
This time header-bearing navigation completed and persisted the exact route-1
browser, session, tab, target, and Guacamole URL, proving both earlier defects
fixed. Handoff publication then failed with
`browser_session_handoff_desktop_missing`. The provider bootstraps each warm
Guacamole viewer before its display binding exists, while ordinary Browser
Session Manager navigation now requires that same display binding to publish a
remote-view handoff. This creates presentation-bootstrap recursion for the
internal viewer process. Quarantine exactly closed the browser and session,
stopped all three provider containers and listeners, retained no viewer
process, and reported `productionUnchanged: true`. All three allowed provider
attempts are consumed. Further provider apply, candidate rebuild, installed
shutdown, or replacement-journey execution requires a new reconciled plan
version and explicit continuation; no blind retry is allowed.

The operator explicitly authorized version 45 to amend the plan, repair the
presentation bootstrap boundary, and perform one additional development-only
retry. The ordinary handoff remains a lookup and focus operation: resolve the
session's current tab and browser, read the browser's configured display,
select the Guacamole route for that display, focus the tab, raise and maximize
the browser, and return the opaque handoff URL. Production routes A, B, and C
currently resolve to `:10`, `:11`, and `:12`; the isolated development provider
must continue using its own route users and displays rather than borrowing
those production resources. Development provider startup alone receives one
typed internal bootstrap marker. It still opens the exact registered viewer
profile through Browser Session Manager, applies the header-auth navigation,
records exact process and tab custody, and closes exactly on failure, but it
does not publish a remote-view handoff for the infrastructure viewer that is
creating the route display. After the display is observed and written into the
configured provider inventory, every ordinary open retains the strict ready
handoff requirement. The marker must fail closed outside the development
runtime and outside exact provider viewer session/profile identity.
Provider-free tests must prove ordinary missing-display navigation still
fails, the exact internal viewer bootstrap skips only handoff publication,
production cannot request the marker, and both warm and elastic provider
launch paths set it. One optimized candidate build, replacement development
install, green plan, stage, and preflight, and one deferred-ingress apply are
authorized. Any new quarantine stops without retry.

Version 45's authorized provider apply succeeded with receipt
`apply-1789788918798-73459.json`: isolated warm displays `:13` through `:16`
became ready, provider doctor passed, ingress remained deferred, and the receipt
proved production unchanged. The first ordinary post-bootstrap open then
failed closed with `browser_session_handoff_desktop_missing`. The runtime host
had initialized Browser Session Manager while the provider inventory was still
empty and retained that startup snapshot after bootstrap published the four
routes. Version 46 therefore authorizes one provider-free repair that refreshes
the manager's route choices from the current authoritative inventory before an
ordinary new browser allocation, while forcing the exact typed internal viewer
bootstrap to use no desktop route. Existing browser custody and display
assignments must remain unchanged. One replacement development candidate and
install are authorized, followed by an ordinary open that must return a ready
opaque handoff and exact session cleanup. The already-ready provider must not
be reapplied or published through ingress. A failed ordinary retry ends this
packet.

Version 46 installed development generation `0.28.0-30cfdf91ee18` with binary
SHA-256 `30cfdf91ee18414f2925e9d445ac6c21af3d094659b380b15a77341225281cef`.
Its single ordinary retry again failed closed with
`browser_session_handoff_desktop_missing`, and exact close removed the test
session and browser. Readback isolated the remaining adapter mismatch:
`load_current_remote_desktop_routes()` reads `AGENT_BROWSER_RDP_ROUTE_POOL_JSON`
or the legacy two-route environment, while the development unit intentionally
publishes the ready provider through
`AGENT_BROWSER_PRESENTATION_PROVIDER_INVENTORY_PATH`. The current provider
inventory contains ready `development-route-1` through
`development-route-4` on `:13` through `:16`, but that typed inventory never
reaches Browser Session Manager's launch choices. A successor repair must
adapt only ready entries from `PresentationProviderInventory` into
`BrowserDesktopRoute` choices, preserve the explicit empty choice for internal
viewer bootstrap, and retain the legacy static adapter when no provider path is
configured. No further build, install, provider apply, ingress publication, or
ordinary retry is authorized by this packet. The development provider remains
ready with all three containers and ports healthy; production remained
unchanged.

On 2026-09-19 the operator rejected the version 47 adapter direction. Version
48 supersedes the proposed direct
`AGENT_BROWSER_PRESENTATION_PROVIDER_INVENTORY_PATH` to
`BrowserDesktopRoute` adapter. The generated inventory file may remain a
diagnostic receipt during migration, but it is not an allocation API or a
runtime authority. `AGENT_BROWSER_RDP_ROUTE_POOL_JSON` and its legacy two-route
environment are confined to one compatibility ingress at the provider
boundary, never read by Browser Session Manager, and removed after the current
production routes have migrated and passed cold-start acceptance. Changing
display numbers are expected: route configuration and custody are durable,
while displays are recreated and rebound through the provider API. Environment
variables are limited to immutable bootstrap configuration such as runtime
identity, local provider endpoint, capacity policy, and store location. The
internal viewer-bootstrap exception becomes a typed private provider request,
not a caller-set environment switch.

Version 49 supersedes version 48's compatibility ingress, typed internal
viewer request, fragmented JSON persistence, and rollback-to-old-generation
direction. The operator completed a one-question-at-a-time architecture review
and selected a forward-only cutover. The new runtime has no route-inventory,
provider-inventory, or internal-viewer environment input and no legacy runtime
fallback. A cold upgrade imports valid legacy configuration and history into
one user-private SQLite database before the new runtime starts, archives every
source and rejected record with typed reasons, removes the old unit wiring and
owned processes, and proceeds with the new generation. Invalid legacy state
cannot veto the cutover. Failure leaves the new generation diagnostic and
repair-forward; it never restores the broken runtime architecture.

Hidden Chrome viewers are removed. The existing runtime host owns supervised,
in-process Guacamole tunnel keepers that establish and verify XRDP sessions
through the same Guacamole path used by operator handoffs. Keepers have no
browser profile, tab, Browser Session Manager session, or handoff. The first
verified route satisfies `minimumReady=1`; the provider warms toward the
user-configured target in the background and creates further displays just in
time. One browser is placed on each display while capacity can grow. At the
configured display maximum, additional browsers may share the least-loaded
display up to the configured density. Desktop Services owns the single fenced
display-control lease used by handoff activation, focus, maximize, capture,
and input, so shared-display interference is explicit and stale commands
cannot affect a replacement generation.

Version 50 corrects one disproven version 49 identity assumption. Apache
Guacamole assigns a new UUID to each newly created websocket tunnel, and the
configured provider exposes no operation that can reattach a successor process
to the predecessor's exact tunnel UUID. The durable route identity is therefore
the digest-fenced catalog binding plus the exact XRDP ownership witness. The
Guacamole UUID identifies one transport occurrence under one keeper fence. A
cold successor may publish a new tunnel occurrence only after exact predecessor
process exit is proved, the same catalog binding is revalidated before the
provider effect, and the complete XRDP witness is reobserved unchanged. Same
process replay reuses the one retained task; a later process needs a new exact
exit proof. Stale terminal events remain occurrence-bound and cannot degrade a
newer tunnel. This correction does not authorize configured startup recovery,
whole-authority recovery, provider effects, or runtime acceptance by itself.

## Frozen Interface Packet

The shutdown module exposes one operation that receives one effects adapter.
The controller owns the fixed phase order and short deadline for each phase;
the adapter receives only the selected phase and its deadline. The operation
accepts no transaction, drain, census, revision, rollback, hash, capability,
or repair-token input. Every phase returns a typed step receipt. Verification
returns independently attributable Agent Browser-owned residue plus separately
reported foreign-process observations. A failed phase does not suppress later
ownership release, transient cleanup, or final verification.

The cold-install module exposes one forward-only operation with the fixed
sequence `stop`, `migrate`, `replace`, `start`, and `readiness`. Exact
Agent Browser ownership evidence controls process cleanup; stale lease, owner,
transaction, or route metadata cannot veto it. Migration imports valid fields,
archives original sources and typed rejections, removes legacy unit inputs, and
commits the new SQLite schema before start. A replace, start, or readiness
failure leaves the selected new generation installed in a typed diagnostic
state for forward repair. The operation never restores the old runtime, route
database, unit definition, or legacy authority. Success requires all five
phases and at least one verified presentation route. Legacy hot-upgrade
transaction state is not an input to this interface.

The Browser Session Manager is a deep module inside the extracted Service
Model crate. Its interface admits a named session against an exact-profile or
disposable-profile intent, records activity and tab attribution, closes a
session, reconciles current PID and CDP observations, and runs one reaper tick.
The existing user-scoped Service process hosts it. CLI commands automatically
start that Service once when needed and never fall back to legacy lease
authority. Existing browser launch, termination, PID and CDP observation,
atomic persistence, and event adapters remain mechanisms behind the new seam.

The runtime host owns one user-private SQLite database as the only durable
authority for browser sessions, profiles, logical tabs, handoffs, presentation
intent, capacity policy, operation records, generations, Desktop Services
lease joins, cleanup obligations, history, and provider credentials. WAL,
durable transactions, integrity checks, incremental vacuum, and one rotating
verified online backup protect it. The live database has a provisional 96 MiB
cap, including 64 MiB of exact URL history, and database, WAL, plus compressed
backup have a 128 MiB routine budget. Repeated URL events are coalesced; old
exact URL events compact into daily first, last, count, session, profile, and
recovery summaries. Material lifecycle and recovery history is retained, and
compaction is itself auditable. Corruption preserves the damaged database and
automatically restores the verified backup. If neither copy is valid, the
runtime starts diagnostics and fresh presentation capacity without inventing
browser or handoff identity.

Legacy environment and JSON knowledge exists only in the cold-upgrade
migrator. It performs tolerant field-level import in one idempotent
transaction, records source hashes and an import receipt, archives source data
read-only, and never dual-writes. Valid named profiles and history import
independently. Invalid records are retained verbatim with typed reasons; safe
capacity defaults and a disposable profile keep unrelated service available.
The runtime itself understands only the SQLite schema and returns a typed
installation error if obsolete inputs remain.

An active browser is one concrete process attached to one profile. Multiple
named sessions may share it and route requests through its one CDP endpoint.
Each session owns attribution, not permanent tab control. Every live tab has at
least one active session reference and one session-current pointer is retained
when that session has tabs. A session adopts an available unattributed Chrome
bootstrap tab, or receives one session-initial tab when none remains. Ordinary
navigation reuses the current tab; explicit new-tab is the only additional
tab-creation command within a session. Closing the current tab selects the most
recently used remaining tab without eagerly creating a replacement. Session
termination closes its live tabs and preserves only historical metadata. Tab
sharing, tab handoff, authenticated principals, identity-restricted profiles,
desktop-exclusive leases, per-session timeout overrides, tab caps, and silent
tab eviction are deferred. `session_idle_timeout_ms` defaults to `300000`;
activity is a heartbeat. Explicit close, heartbeat expiry, browser termination,
or failed bounded liveness recovery ends a session. The browser closes when its
last session ends. Interrupted commands are never replayed automatically.

Presentation has no configured display-to-viewer inventory. User-scoped typed
settings in SQLite provide `presentation.warm-target` (default 4),
`presentation.maximum-displays` (default 6),
`presentation.maximum-browsers-per-display` (default 4), queue size (default
32), request deadline (provisionally 90 seconds), and scale-in cooldown
(provisionally 10 minutes). `agent-browser service runtime-config get/set` and the Service API
change them transactionally and trigger live reconciliation without
recompilation, reinstallation, or normally a restart. Lowering a limit below
current usage is non-destructive: status becomes `over-target`, new allocation
cannot worsen it, and the provider converges as resources become idle.

The provider creates deterministic logical slots, route users, credentials,
Guacamole connections, and supervised protocol-level keepers. Physical
display numbers and Guacamole connection IDs are ephemeral observations. The
provider stays diagnostic when capacity is absent, reports progress, and makes
ordinary opens wait up to their deadline without launching Chrome before a
verified display exists. Active-view recovery has priority, followed by access
to an existing handoff and then aged FIFO new opens. Duplicate requests
coalesce. Merely queued requests become retryable after restart; operations
that began external effects are reconciled or compensated. Empty,
reference-free displays above the warm target scale in only after cooldown and
a final database, browser, handoff, Desktop Services, keeper, and live-process
reference check.

Each browser retains its display binding for its lifetime. New browsers receive
unused displays first, then the least-loaded healthy display after the display
maximum. Multiple viewers may observe a shared display, but only one holds the
Desktop Services control lease. Activating another handoff transfers control,
focuses and maximizes its exact browser and logical tab, and makes prior clients
view-only. The authenticated remote-view connection and live Guacamole tunnel,
not a stored page or flag, prove active viewing. Every host start advances a
SQLite generation; provider work, Desktop Services leases, browser launches,
callbacks, and operation completion carry generation plus operation ID, and
stale effects cannot publish readiness or act on a replacement display.

A durable handoff stores an opaque ID and logical session, browser, profile,
and tab target, never a raw provider URL or absolute public URL. Repeated open
for the same logical session and tab returns the same handoff. Resolution uses
the current public origin and provider binding, may recreate a missing tab in
the same healthy browser, and may launch at most one proven replacement browser
at the last committed top-level URL. It never replays clicks, forms, downloads,
or other commands. Active-view failure retries with bounded backoff for up to
90 seconds while dormant recovery remains lazy. Named-profile handoffs have no
default time expiry. Disposable handoffs, sessions, and profiles expire after
24 hours of inactivity by default and are additionally bounded by 20 retained
profiles and 10 GiB; all limits are live user settings. Cleanup is
oldest-inactive-first, never affects active or named profiles, and returns a
typed quota error if it cannot make room. Profile promotion is deferred.

This packet declares the following disjoint write scopes before fan-out:

- Primary: `cli/src/workstation_shutdown.rs`,
  `cli/src/workstation_cold_install.rs`, and the smallest command adapter in
  `cli/src/main.rs`; `cli/src/workstation_install.rs` remains deferred until
  P205 reconciliation.
- Provider-free fixture worker:
  `cli/src/workstation_shutdown_contract_tests.rs` and
  `cli/src/workstation_cold_install_contract_tests.rs` only. The primary owns
  module declarations and production logic.
- Remote-view implementation worker:
  `cli/src/native/presentation_requalification.rs` only, including its local
  unit tests. The primary owns module registration and later integration with
  presentation inventory and route selection.
- The former P205 Service State exclusion ended when Plan 0216 merged. P211 may
  now add narrowly scoped cold-shutdown transitions to the extracted crates,
  while P207-controlled help and documentation files remain excluded.

The first packet completed its bounded fan-out. Fixture worker
`/root/p211_fixture_contracts` added only the two declared contract-test files.
Remote-view worker `/root/p211_presentation_requalification` added only the
declared requalification module and its local tests. The primary reconciled the
boot-epoch type with the repository's string identity, registered the modules,
and implemented the shutdown-deadline and cold-install controllers. The
shutdown deadline tracer failed first because `deadline_ms` was absent, then
passed after the controller owned and normalized every deadline. The
cold-install tracer failed first because its module was absent, then all four
contract cases passed after the fixed sequence and rollback result landed.

Focused primary validation is green for five shutdown-controller tests, four
shutdown-contract tests, four cold-install contract tests, and eight
presentation-requalification tests. Strict workspace Clippy with warnings
denied, workspace formatting, and `git diff --check` pass. The validation
selector also named broad workstation and Guacamole lanes, but those remain
deferred because this checkpoint changes no platform adapter, installer route,
embedded asset, or live runtime. Production shutdown routing, platform effects,
cold installer integration, profile release, and remote-view integration remain
open.

## Consolidated Batch

1. Add a deep cold-shutdown module with one external operation and injected
   process, unit, container, and state adapters for provider-free tests.
2. Route `agent-browser shutdown` through that module. Use fixed short phase
   deadlines, polite termination followed by exact owned-process escalation,
   and idempotent postcondition checks.
3. On shutdown, close every Agent Browser-owned browser, release every runtime
   owner and lease, mark retained profile definitions unowned, preserve profile
   directories, and remove only Agent Browser-owned transient coordination
   artifacts.
4. Make ordinary workstation apply and reviewed-candidate install use cold
   shutdown, payload replacement, unit activation, and a bounded readiness
   probe. Do not create a new hot-upgrade transaction.
5. Park hot-upgrade mutation commands as legacy recovery/readback surfaces.
   They may inspect or close old transactions but are not selected by the
   default install or upgrade command.
6. Add the Browser Session Manager, independent Browser Session State, and
   independent Browser Profile Catalog. The ordinary path must not call Lease
   Authority, principal, owner-generation, sealed-recovery, or boot-identity
   modules as authority or fallback.
7. Import only valid legacy profile definitions when the new catalog is absent.
   Keep all other legacy state as diagnostic history. Make Service State
   projection best effort and nonblocking.
8. Route all ordinary CLI requests through the one user-scoped Service. Share
   one live browser per exact named profile across named sessions, use a
   session-scoped allocation for disposable profile intent, refresh heartbeats
   from activity, and proactively reap expired sessions, sessionless browsers,
   and exactly proven disposable profile directories.
9. Give each session one current tab. Adopt an unattributed Chrome bootstrap
   tab or create one session-initial tab, reuse the current tab for ordinary
   navigation, create further tabs only on explicit request, select the most
   recently used remaining tab after close, and close live tabs when their
   session ends without deleting historical metadata.
10. Replace fragmented runtime JSON authorities with one user-private SQLite
    database owned by the runtime host. Persist typed user configuration,
    profiles, sessions, browsers, logical tabs, handoffs, provider intent,
    operation records, generations, cleanup obligations, credentials, and
    compact history transactionally. Keep JSON only as migration input or an
    explicitly requested diagnostic export.
11. Make one in-process presentation-provider control plane the only authority
    for readiness, allocation, release, capacity, and handoff resolution.
    Browser Session Manager calls that typed API and never parses route JSON,
    a generated inventory, or an internal-bootstrap environment switch.
12. Remove infrastructure viewer browsers. Supervise lightweight in-process
    Guacamole tunnel keepers that establish XRDP sessions through the exact
    operator presentation path without Browser Session Manager, Chrome,
    profiles, tabs, or handoffs.
13. Reconstruct presentation eagerly after cold start. One verified route makes
    service usable, the provider warms toward the configured target in the
    background, and extra displays are created only as needed. Use one browser
    per display until maximum displays, then permit bounded least-loaded
    overflow sharing under the single Desktop Services control lease.
14. Make all capacity, queue, timeout, cooldown, history, and disposable-profile
    retention limits typed user-scoped settings that apply live through Service
    API and CLI configuration. Do not encode mutable policy in environment
    variables, generated files, or installer-only constants.
15. Make browser open a crash-recoverable operation: commit pending session,
    browser, slot, and handoff intent before effects; establish display and
    Chrome; then atomically publish the observed browser, tab, display, and
    handoff as ready. Fence every effect by host generation and operation ID.
16. Route display focus, maximize, capture, pointer, and keyboard through the
    existing Desktop Services fenced control authority. Multiple clients may
    observe a display, but only one controls it; activating a different handoff
    transfers control and makes prior clients view-only.
17. Persist only logical durable handoffs. Resolve current provider bindings at
    access time, lazily recover dormant browsers, eagerly recover active views,
    recreate a missing logical tab or at most one proven replacement browser at
    its last committed URL, and never replay page actions.
18. Perform one forward-only cold-upgrade migration. Import valid legacy fields
    transactionally, archive source hashes and typed rejections, remove legacy
    units, variables, runtime readers, docs, and tests, rebuild the owned
    Guacamole database as derived infrastructure, and never roll back to or
    fall back on the old architecture.
19. Return success only when `operatorVisible.state=ready` and the durable
    opaque handoff resolves. Doctor, service status, capacity, preflight, and
    checkout must agree on the same effective readiness result.
20. Update CLI help, README, agent skill, documentation site, and inline docs to
    present the one-command workflow, typed configuration, durable-handoff
    recovery, and forward-only migration while removing obsolete environment
    and hot-upgrade ceremony.

## Scope And Effect Boundary

Expected implementation writes are limited to the CLI command router and help,
a focused cold-shutdown/cold-install module, the smallest required adapters in
the workstation installer, a new Browser Session Manager and Browser Profile
Catalog in the extracted Service Model crate, their CLI persistence and effect
adapters, native runtime, profile acquisition, display selection, dashboard
workspace projection, and remote-view handoff paths, provider-free
fixtures, README, the Agent Browser skill, installation and remote-view
documentation, this plan, and P211's active-lane projection.

The presentation successor may add one provider-neutral in-process
control-plane contract, one SQLite persistence adapter, one Guacamole protocol
adapter, and the smallest shared Desktop Services lease join. It removes the
generated inventory, route-pool environment, internal bootstrap switch, and
fragmented JSON authorities from runtime use. It must not create a second
daemon, database, coordinator, allocation authority, infrastructure browser,
or operator-visible recovery workflow.

The command may affect only Agent Browser-owned user units, timers, browsers,
runtime hosts, dashboard processes, MCP processes, presentation processes and
containers, leases, ownership records, and transient runtime metadata. It must
not kill unrelated browsers or containers. Unknown foreign processes are
reported and left alone; stale Agent Browser bookkeeping does not block the
owned shutdown set.

This plan authorizes isolated development provider credentials, provider
mutation, development browser/profile effects, and development-only cold
upgrade acceptance after source qualification. It does not authorize a
production install, production shutdown, production credential or provider
use, external ingress publication, release, broad process cleanup, or mutation
of unrelated worktrees. The product contract is intentionally one private
trusted-user runtime; this plan does not claim adversarial multi-user
isolation.

The current delivery defers authenticated principals, identity-restricted
profiles, adversarial or distributed fencing, cross-profile tab sharing,
profile promotion, a dashboard settings editor, per-session timeout overrides,
tab caps and silent eviction, multi-window management, and a remote viewer for
`:0`. Extension points may be retained, but no deferred concept may appear in
the ordinary interface or live authority calculation.

## Delivery Sequence And Budget

- Optimization target: balanced wall-clock and token efficiency.
- Active-agent concurrency: one. The operator explicitly assigned this lane to
  the primary alone; no subagent, auxiliary worktree, or independent reviewer
  is part of the version 49 batch.
- Critical path: frozen lifecycle and trusted-single-user contracts, public
  shutdown, forward-only migrator, SQLite authority, route keeper, Desktop
  Services fencing, capacity reconciliation, durable handoff recovery, and
  isolated development cold-upgrade acceptance.
- Completed foundation: public shutdown, cold-install routing, Browser Session
  Manager, profile catalog, shared named-profile browsers, logical tab
  attribution, durable handoff projection, documentation parity, and isolated
  development provider staging remain reusable only where their contracts do
  not conflict with version 49.
- Slice 1: freeze the SQLite schema, typed configuration, operation journal,
  generation fencing, history compaction, backup, and forward-only import
  contracts. Add provider-free red tests for migration tolerance, corruption
  recovery, pending-operation replay, and legacy-input rejection.
- Slice 2: implement the SQLite repository and move Browser Session Manager,
  profiles, tabs, handoffs, presentation intent, configuration, and history
  behind it. Retire runtime JSON reads and dual-write projections.
- Slice 3: change cold upgrade to `stop`, `migrate`, `replace`, `start`, and
  `readiness`; remove rollback to the old generation; delete legacy unit
  inputs, route readers, provider inventory authority, and the caller-set
  bootstrap switch. Prove exact owned cleanup and foreign preservation.
- Slice 4: implement the in-process Guacamole tunnel keeper and provider state
  machine. Prove `minimumReady=1`, background warm target, just-in-time scale
  out, reference-free scale in, zero privileged calls on a healthy cold start,
  and exact privileged repair receipts when drift exists.
- Slice 5: implement one-browser-per-display preference, bounded overflow
  sharing, capacity queueing and priority, live configuration changes, memory
  admission, and the shared Desktop Services fenced control lease.
- Slice 6: make handoff resolution idempotent and recovery-aware. Prove active
  eager recovery, dormant lazy recovery, same-URL reuse, missing-tab recreation,
  at most one browser replacement, exact last-committed URL tracking, and no
  command replay.
- Slice 7: implement bounded history and disposable retention: 96 MiB live
  database, 128 MiB routine total, 24-hour inactivity, 20 retained disposable
  profiles, 10 GiB disposable storage, and oldest-inactive cleanup. Expose
  read-only doctor plus status, config, recovery, queue, and repair receipts.
- Slice 8: synchronize CLI help, README, Agent Browser skill, docs site, inline
  docs, generated service contracts, development scripts, and fixtures. Run
  changed-surface provider-free validation once against the frozen candidate.
- Slice 9: install the frozen candidate only in the isolated development
  runtime and run the accepted cold-start and fault-injection matrix. Keep
  external ingress deferred and prove production unchanged after every effect.
- Maximum work-unit attempts: 3 per slice.
- Maximum review and rework cycles: 1.
- Maximum consecutive hardening checkpoints: 2.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Version 49 implementation and development-acceptance ceiling: 360 active
  minutes. This does not erase prior P211 effort or authorize production.
- Next outcome artifact: provider-free red tests plus a typed schema packet for
  the SQLite authority, forward-only migrator, Guacamole keeper lifecycle, and
  Desktop Services control join. No optimized build or provider retry occurs
  before those cheaper seams are green.

Checkpoint `726563fb` completes the first version 49 implementation slice.
The default runtime host and daemon admission paths now read Browser Session
State and Browser Profile Catalog state only from a private WAL-backed SQLite
database. The cold installer executes `stop`, `migrate`, `replace`, `start`,
and `readiness`; no post-migration failure path restores the old generation.
Migration hashes and read-only archives the three legacy JSON inputs, builds
the database under a unique staging name, checkpoints it, and atomically
publishes it. A narrowly named shutdown adapter may read or terminalize legacy
JSON only when no SQLite database exists; a present but invalid database fails
closed instead of falling back. This checkpoint does not yet make the reserved
configuration, handoff, presentation, operation, credential, generation, or
history tables authoritative, record typed field rejections, remove legacy
provider assets, or implement backup recovery. Those remain in Slices 2
through 7.

Checkpoint `c0ff2768` completes the tolerant-import and typed-configuration
part of the schema packet. Missing legacy files now produce defaults without
inventing source history. Present sources are hashed and archived whether or
not they contribute state. Invalid session or catalog records and rejected
legacy profile fields no longer veto migration; the database records their
source, stable code, and diagnostic detail, and valid sibling profiles still
import. The database also seeds one validated runtime-configuration aggregate
with the frozen capacity, deadline, cooldown, inactivity, retention, and byte
limits. Revisioned compare-and-swap makes internal updates durable and rejects
stale or invalid replacements. Default host timeout and disposable policy
projection now reads this aggregate and no longer reads the three ad hoc
session/disposable environment variables. The Service API and CLI mutation
surface plus live in-process refresh remain pending, so this is not yet public
configuration acceptance.

Checkpoint `759f12e9` adds the durable operation-journal kernel. An exact
operation ID and request replay the same prepared or committed record. Each
logical owner advances its own generation, so a later reservation fences an
earlier effect from committing while unrelated owners remain independent.
Exact committed results replay after database reopen; changed requests or
results fail closed. Browser open, presentation allocation, and handoff code
do not consume this kernel yet, so crash-consistent open remains an integration
gate rather than an accepted behavior.

Checkpoint `56a0558e` completes the first journal-to-browser transaction
slice. A named-profile remote open now persists its exact session, browser,
display-slot, and handoff intent before browser effects. Durable observations
advance through `launch_started`, `browser_opened`, `tab_acquired`, and `ready`;
ready Browser Session State and the logical handoff publish in one immediate
SQLite transaction. Exact replay returns the committed response without
reselecting from multiple routes. Recovery resumes a current `Prepared`
operation, reuses a positively observed browser without another launch, and
fails closed with `browser_runtime_open_effect_recovery_required` when launch
started but its outcome is ambiguous. The operation captures its exact base
session state, checks it before resumed effects, and checks it again inside the
publish transaction, so an intervening ordinary manager write cannot be
overwritten. Fresh explicit-display opens retain the preexisting path, while
an already journaled operation remains recoverable. Legacy Service State
projection occurs only after the SQLite commit and is not transaction
authority. Automatic reconciliation of an ambiguous launch, protocol keepers,
provider-owned route authority, full handoff recovery, and the development
cold-start matrix remain open.

Checkpoint `e1a3edcd` retains a durable disposition for the ambiguous interval
between browser process creation and `browser_opened`. Reserved browser
launches now carry the exact deterministic browser identity as a causal process
marker. The model accepts an exact observed launch without running another
launch effect, publishes only after session, browser, desktop, and healthy
reserved-route validation, and does not recompute least-loaded placement. An
unproven recovery, failed probe, or rejected observed launch records one
`launch_cleanup_required` obligation with exact operation, generation,
browser, profile, route, display, and reason fields. Repeated replay neither
launches nor probes again. Provider-free fixtures prove both exact adoption and
durable cleanup. The production runtime intentionally returns unproven until a
cross-platform discovery adapter can bind the causal marker, canonical profile
directory, root PID, current process identity, CDP endpoint, and listener
ownership. Thus automatic real-process adoption and exact cleanup remain open.

Checkpoint `6390a48a` completes the provider-neutral route-keeper lifecycle
kernel and durable authority packet. The pure model owns a bounded six-slot
inventory, reconciles `minimumReady=1` before the warm target of four, and has
no browser, profile, tab, Browser Session Manager, or handoff input. Protocol
readiness is an exact Guacamole connection, XRDP session, display, keeper,
slot, host-generation, and operation-generation receipt. A keeper disconnect
can start a fenced replacement or enter an explicit adoption phase; adoption
cannot use the ordinary ready transition and must preserve the prior protocol
resources exactly. Exact stop clears only the recorded route. Foreign or
ambiguous observations quarantine the record and retain a cleanup obligation
instead of retrying destructive cleanup. Phase-aware validation prevents
malformed persisted records from projecting false readiness. The runtime
SQLite database seeds this authority independently of session and handoff
documents, loads a default for pre-checkpoint databases, and publishes changes
through an immediate compare-and-swap transaction that rejects stale or
generation-regressing writers. This is not the protocol-keeper runtime
adapter: no Guacamole tunnel, XRDP session, display, browser, or provider was
started, and provider-owned readiness plus durable cold-start reconstruction
remain open.

Checkpoint `89c05d7e` adds the provider-free runtime adapter for that
authority. It persists `Starting`, `Adopting`, or `Stopping` before invoking an
injected connector, then publishes only an exact observation through SQLite
compare-and-swap. Cancellation after start-effect creation and a conflict
while publishing `Observing` both leave durable `Starting`; a fresh adapter
selects `Observe` and reaches readiness without a second start. Disconnect is
durable before recovery. Adoption and stop drive the exact prepared action
rather than another priority selection. Ready, adoption, and stop receipts are
bound to the dispatched slot and fence; ready and adoption also bind the
keeper, and adoption binds the previous host generation. A foreign keeper
reported for the dispatched stop is quarantined, while an unrelated slot or
fence is rejected without mutation. The SQLite repository opens a connection
per operation instead of retaining one across an asynchronous connector wait.
This remains a provider-free seam with an in-memory fixture connector. It does
not yet own `PrimaryTask`, connect the Guacamole protocol transport, observe
XRDP, schedule the host loop, or project actual provider readiness.

Checkpoint `ab05e2d3` connects the authority boundary to the existing
receive-only Guacamole transport without using the legacy binding. The generic
`PrimaryGuard` and fresh authority check now belong to `PrimaryTask` transport.
A keeper-specific guard reloads SQLite before every transport effect and
requires the exact slot, keeper, host generation, operation ID, and operation
generation in `Starting`, `Observing`, or `Ready`. A duplex-websocket fixture
proves the task reaches protocol readiness while current and closes before
acknowledging a later frame after its fence is superseded. This is still not
the concrete provider connector: no provider authentication, Guacamole URL,
XRDP observer, task registry, host-loop scheduler, or live process is present.

Checkpoint `a481d7e1` adds the process-local connector and task registry while
keeping every provider effect injected. A successful start places the real
`PrimaryTask` under connector custody synchronously, so caller cancellation
cannot detach it. Exact replay retains the same occurrence; another keeper or
fence cannot replace it. Readiness joins the task's Guacamole UUID with an
exact injected XRDP session and display receipt and rechecks the task after the
observer await. Stop retains custody across cancellation and observer errors,
uses a fresh exact SQLite `Stopping` guard immediately before the destructive
observer, and caches the terminal receipt for replay after a final publication
conflict. A terminal callback only enqueues bounded identity and cause
evidence. The connector refuses adoption because an in-memory task cannot
survive a cold process restart. This is provider-free evidence: concrete
provider authentication and task creation, XRDP observation and stop effects,
terminal-event reconciliation, host-loop scheduling, and live readiness are
still absent.

Checkpoint `1d17f471` adds the provider-free supervisor and durable terminal
reconciliation above that connector. Startup reconciles only to
`minimumReady` before timed ticks grow the warm pool one action at a time.
Shutdown interrupts a pending connector await, closes every retained primary,
drains the resulting exact terminal events, persists their SQLite transitions,
and acknowledges task release only after successful compare-and-swap. A
pre-ready termination returns the exact attempt to `Absent`; a failed recovery
enters `RecoveryFailed` while preserving its prior Guacamole, XRDP, and display
receipt; a ready disconnect becomes degraded; and a terminal exact stop
quarantines the retained identities and cleanup obligation. Changed occurrence
or fence evidence is discarded without durable mutation, while publication
failure restores the event for replay. The full Service Model package, six
focused model tests, eleven adapter tests, ten connector tests, all 21
Guacamole-primary tests, formatting, strict workspace Clippy, diff hygiene,
and closed-world review pass. Concrete provider-authenticated task creation and
connection catalog authority, XRDP observation and exact stop effects,
RuntimeHostRouter integration, and live provider readiness remain absent.

Checkpoint `e9593b5b` extracts the Guacamole connection identity from the
legacy browser binding into a browser-independent specification and adds a
concrete route-keeper primary factory. The specification accepts only a
nonempty connection ID and the exact credential-free, query-free,
literal-loopback `/guacamole/` provider path. The factory selects by exact
keeper slot, rejects an absent catalog entry before provider access, uses the
existing bounded header-authenticated connection flow, keeps tokens transient,
and carries the same fresh SQLite guard through connection and task custody.
The legacy browser-bound primary remains on the same validated path. This is
still source-only: no authoritative slot-to-connection catalog populates the
factory, RuntimeHostRouter does not install it, and XRDP observation and exact
stop remain injected.

Checkpoint `18c790bd` removes the legacy hidden-viewer bootstrap and its
environment input. Browser Session Host desktop placement now comes only from
`Ready` SQLite route-keeper receipts. Until the keeper-backed handoff join is
implemented, remote-required manager requests fail before Browser Session Host
loading or browser effects with a typed unavailable or integration-pending
result. Development provider preflight and the real system-effects adapter
also reject before mutation, and the old hidden Chrome launcher is removed.
The independent review found an initial base-adapter bypass; the corrected
adapter and regression prove zero command calls before rejection. The provider
fixture, six workstation and Guacamole fixtures, thirteen Browser Session Host
tests, the daemon fail-before-effects test, documentation contract, production
docs build, formatting, strict workspace Clippy, and diff hygiene pass. Help,
README, the repository skill, the docs site, and the forward-only controller
now agree that failures at or after migration never restore the old generation.
No provider, browser, installed-runtime, production, ingress, or release effect
occurred.

Checkpoint `d402f55b` adds the v2 route-keeper connection catalog to the same
SQLite authority as keeper lifecycle. Each binding carries the exact slot,
stable provider connection key and name, and positive numeric Guacamole
connection ID. Its canonical SHA-256 digest is embedded in every keeper fence,
which binds all reconcile actions and receipts to one catalog snapshot. Empty
or undersized catalogs block new starts. Replacement requires every slot to be
`Absent`, and the SQLite compare-and-swap enforces that boundary even if a
caller bypasses the model helper. The concrete primary factory reloads SQLite,
checks the action fence and digest, and only then constructs the transient
provider connection from the catalog's numeric ID. Absent v1 documents migrate
through an exact schema-plus-JSON compare-and-swap; a stale reader reloads
instead of overwriting a newer operation. Any active v1 document fails closed
without rewriting its phase or receipts. Independent review found the initial
stale-write and active-readiness migration hazards and passed the corrected
regressions. The full 221-test Service Model package and integration tests, 26
focused keeper, store, and connector tests, thirteen Browser Session Host
tests, formatting, strict workspace Clippy, diff hygiene, and changed-surface
selection pass. Provider-side stable-name catalog population remains absent,
as do exact XRDP observation and stop, runtime installation, keeper-backed
handoff resolution, and live readiness. No provider, browser,
installed-runtime, production, ingress, or release effect occurred.

Checkpoint `f494f273` closes provider-side stable-name catalog population.
After connection synchronization, the development provider reads the exact
stable connection names, route users, and numeric IDs from PostgreSQL. It
rejects foreign, duplicate, incomplete, noncanonical, zero, or unsafe
identities and constructs all hard-maximum slot bindings in descriptor order.
A bounded stdin-only development CLI bridge publishes the catalog into the
user-private SQLite authority before Guacamole starts. Every adapter must
provide both the readback and publication effects before any provider effect
can run. Exact replay is unchanged only after the complete slot set is proved;
changed catalogs remain absent-only; and the system-effects adapter verifies
that the receipt digest equals the canonical submitted catalog. The provider
fixture, ten focused catalog tests, three hidden-bridge tests, the 118-test
Lease Authority package, architecture guard, six selected workstation and
Guacamole fixtures, formatting, strict workspace Clippy, diff hygiene, and
independent re-review pass. Exact XRDP ownership observation and stop,
RuntimeHostRouter installation, keeper-backed handoff resolution, and live
readiness remain absent. No provider, browser, installed-runtime, production,
ingress, or release effect occurred.

Checkpoint `d0aa34de` extends the keeper protocol receipt with a typed XRDP
ownership witness. The Service Model validates and persists the exact boot,
route user and UID, logind session and scope, scope invocation, cgroup
device/inode/path, leader and Xorg process identities, display, and X11 socket
inode. The route-keeper connector carries that evidence from observation into
the durable ready receipt and requires it again for exact stop. This checkpoint
does not observe or terminate a real XRDP session by itself.

Checkpoint `9b96e44f` qualifies the concrete provider-free XRDP observer and
exact-stop source packet. The privileged helper now selects only X11 listener
rows whose `/proc/net/unix` flags include `0x10000`, proves every selected
listener inode belongs to the chosen Xorg, and rejects a bound non-listening
socket. Observation commands are bounded and status-checked. Exact stop
reobserves the complete durable witness, opens the exact cgroup directory,
rechecks the retained device and inode, writes only descriptor-relative
`cgroup.kill`, and treats unreadable post-kill process identity as incomplete.
The public helper no longer exposes broad route-user termination, and the
legacy development lifecycle fails closed until the keeper owns stop
integration. Closed-world verification passed after adversarial listener plus
connected-row and non-listener-only regressions. The helper, provider,
installer, route-confusion, observer, witness-wire, helper-contract, doctor,
workstation, Guacamole, validation-selection, formatting, strict workspace
Clippy, diff-hygiene, and complete two-lane provider-free Rust suite pass; the
comprehensive Rust run completed in 829 seconds. RuntimeHostRouter does not yet
install the configured observer or supervisor, keeper-backed handoff resolution
is absent, and no live logind, XRDP, Xorg, Guacamole, browser, installed-runtime,
production, ingress, or release effect occurred.

Checkpoint `d79a387d` installs the configured route-keeper supervisor as one
host-level owner in `RuntimeHostRouter`. The development provider publishes its
validated loopback Guacamole base inside the same SQLite connection catalog as
the stable connection bindings, so the canonical digest fences both selection
inputs. Startup reloads and validates that exact catalog, refuses every durable
non-`Absent` route until cold-process task recovery exists, and starts one
background minimum-before-warm reconciliation loop. Configured shutdown and
every supervisor error path exactly stop each `Ready` XRDP route through its
durable witness before closing the owned Guacamole primary. Observation error
retains primary custody for retry. The host awaits the supervisor once, rejects
and closes a duplicate owner, and tears down already preloaded lanes if keeper
installation fails. Independent closed-world review first found reversed stop
ordering and lost exact-stop policy on error exits; both passed the bounded
re-review. A second focused review found partial startup cleanup and passed the
rollback repair. The development provider fixture, 221-test Service Model
suite plus integrations, 28 focused supervisor and connector tests, host
ownership and initialization rollback regressions, formatting, strict
workspace Clippy, validation tooling, diff hygiene, and complete two-lane
provider-free Rust suite pass; the final comprehensive run completed in 725
seconds. This remains source qualification. Cold-process adoption of retained
keeper tasks, live supervisor health projection, keeper-backed handoff
resolution, and live Guacamole/XRDP readiness remain absent. No provider,
browser, installed-runtime, production, ingress, or release effect occurred.

Implementation checkpoint `da9438a7` joins manager handoff preparation and
resolution to the current SQLite route-keeper authority. The connection catalog
digest now binds the reviewed public operator origin with the provider base and
exact connection bindings. Every projected route is fully keeper-bound and its
opaque URL is validated before a journaled browser launch or ordinary
navigation. Journaled open preflights the reserved slot, display, and handoff,
then atomically commits manager state and the SQLite handoff. Resolution loads
the handoff registry and current keeper authority before the host, validates the
session, browser, tab, display, route, fence, and opaque URL, and only then
focuses the browser. Public responses contain the durable
`/remote-view/<handoff-id>` URL and provider-neutral presentation semantics,
never raw Guacamole, provider, route-binding, or loopback URLs.

Qualification checkpoint `3922f138` adds a daemon-boundary regression proving
SQLite registry lookup, current keeper reload, and failure before focus. That
focused test, formatting, strict workspace Clippy, the prior focused keeper,
host, store, model, and provider-fixture gates, and the complete two-lane
provider-free Rust suite pass; the comprehensive run completed in 818 seconds
with both lanes at zero. This is source qualification only. Cold-process
adoption or recovery of retained keeper tasks, live supervisor health and
readiness, shared Desktop Services control, and the isolated cold-start matrix
remain open. No provider, browser, Service State, installed-runtime,
production, ingress, or release effect occurred.

Checkpoint `b0a82574` adds the first bounded cold-process recovery kernel but
does not enable configured startup recovery. A private predecessor-exit proof
binds one complete prior ready receipt to a strictly newer successor host
generation; SQLite persistence cannot construct that proof. The adapter
durably degrades the exact receipt, prepares or replays only its deterministic
adoption action, and accepts readiness only when the connector preserves the
Guacamole UUID, XRDP session, and display. Exact compare-and-swap rejects a
rebound candidate, full operation-ID matching rejects a foreign prepared
action, connector failure leaves replayable `Adopting` state, and missing
observation returns explicit `Pending`. The first broader tracer was rejected
after review because it manufactured disconnect evidence and could strand
partial whole-authority recovery. All 42 route-keeper tests, formatting, strict
workspace Clippy, diff hygiene, and closed-world re-review pass.

This is provider-free source qualification only. Construction of exact
predecessor process-exit proof, configured Guacamole connector adoption,
recovery of the remaining durable phases, absent-slot successor-generation
rollover, quarantine blocking, runtime-host integration, and live readiness
remain open. No provider, browser, Service State, installed-runtime,
production, ingress, or release effect occurred.

Checkpoint `1ebfa757` advances the SQLite keeper authority to v3 with
append-only host-generation claims. Each claim binds a boot epoch and the
complete recorded host-process identity. A successor may rebase only `Absent`
slots, preserving a retained active predecessor under its original claim.
The provider-free predecessor-exit proof factory first validates the durable
ready receipt and both claims. A different predecessor boot proves exit; on
the same boot, only a missing process or an exact observation that classifies
the recorded PID as reused by an unrelated process proves exit. Exact-live,
ambiguous, incomplete, and failed observations return no proof. Configured
startup records its exact current process claim but still rejects retained
non-`Absent` recovery because the configured Guacamole connector has no
retained-primary adoption implementation.

Diff hygiene, formatting, strict workspace Clippy, and all 44 focused
route-keeper tests pass. The complete provider-free runner ended nonzero after
815 seconds in its support lane. A slow isolated workstation diagnostic rerun
was deliberately stopped without a failure diagnosis, so the broad support
lane remains an explicit validation gate. This checkpoint performs no provider,
browser, Service State, installed-runtime, production, ingress, or release
effect. The next bounded packet must first reproduce or clear that support-lane
failure before joining configured connector adoption or broader runtime proof.

Checkpoint `536d58a9` clears that validation gate. The broad support-lane
failure reproduced as six browser handoff and host fixtures that constructed a
v3 route-keeper authority without registering the exact host-process claim for
their active generation. Production validation correctly rejected those
fixtures with `route_keeper_host_process_claim_missing`. The fixtures now bind
their generation to a complete recorded process identity before starting a
keeper. The isolated workstation compartment passes all 215 tests, the
repaired browser compartment passes 140 active tests with two browser-launch
tests ignored, strict workspace Clippy passes, and the complete provider-free
runner passes both lanes in 720 seconds. The next bounded provider-free packet
may join configured connector adoption with the already constructed exact
predecessor-exit evidence. Whole-authority phase recovery, quarantine handling,
runtime-host startup integration, and live readiness remain separate gates.
No provider, browser, Service State, installed-runtime, production, ingress, or
release effect occurred.

Plan version 50 follows a failed configured-adoption implementation preflight.
The current configured factory can only authenticate and open a new Guacamole
websocket tunnel. The tunnel protocol then reports a newly generated UUID,
while the version 49 model rejects adoption unless that UUID equals the
predecessor's. No adapter can satisfy both contracts truthfully. Three
independent interface studies compared strict retained-tunnel resume, a broad
provider-capability interface, and a common-caller route-continuation seam.
The selected design keeps the existing keeper seam and separates durable route
identity from transport occurrence: the catalog connection and exact XRDP
ownership witness remain invariant, while a successor tunnel UUID may change
under the new fence. The next packet is provider-free model and recovery
evidence only. It must retain the prior occurrence in the adoption receipt,
reject every catalog or XRDP witness drift, and prove that a stale predecessor
terminal event cannot affect the successor. Configured connector work follows
only after that packet passes.

Checkpoint `11f606a4` completes that provider-free model packet. Route Keeper
Authority schema v4 now retains predecessor and current Guacamole tunnel
occurrences separately, migrates historical v3 adoption evidence from the old
same-UUID invariant, and accepts a successor occurrence only when the exact
catalog digest, route user, and complete XRDP ownership witness remain stable.
Disconnect processing remains bound to the current fence and occurrence, so a
predecessor terminal event cannot degrade the successor. The focused service
model suites, 44-test CLI route-keeper lane, v1-to-v4 SQLite migration test,
format check, and strict workspace Clippy pass. Configured connector adoption
joined to exact predecessor-exit proof is the next bounded source packet.
Whole-authority recovery and runtime acceptance remain later gates. No runtime
effect occurred.

Checkpoint `8f60434f` implements the configured Guacamole adoption adapter
without enabling startup recovery. An exact `Adopting` action revalidates the
SQLite catalog and fence before provider access, opens one fresh tunnel task,
reuses that task for same-process replay, observes XRDP under the configured
route user, and returns the schema-v4 predecessor/current occurrence receipt.
The 18-test focused Guacamole keeper lane, format check, strict workspace
Clippy, and diff check pass. Startup still fails closed on retained state. The
next packet must reconstruct predecessor-exit proof for exact `Ready`,
`Degraded`, and interrupted `Adopting` records and give an adoption task's
terminal event a durable failure transition before startup recovery is enabled.
No runtime effect occurred.

Checkpoint `45124f99` closes the provider-free proof and terminal-state gaps
identified after configured adoption. Exact predecessor-exit proof can now be
reconstructed from retained `Ready`, `Degraded`, or deterministic interrupted
`Adopting` state. A current adoption task terminal event moves durably to
`RecoveryFailed`, retains the predecessor ready receipt, and rejects stale
fences or predecessor occurrences. The combined 50-test route-keeper lane,
format check, and strict workspace Clippy pass. Configured startup recovery and
multi-route orchestration remain the next bounded packet; no runtime effect
occurred.

## Worker Assignments

For the exact XRDP source packet, the operator authorized bounded parallel
support. `/root/p211_listener_fixture` used requested `gpt-5.6-luna` low effort
for read-only adversarial fixture design; `/root/p211_validation_selection`
used requested `gpt-5.6-luna` medium effort for read-only validation selection;
and `/root/p211_xrdp_contract` used requested `gpt-5.6-sol` medium effort for
one closed-world preparation pass and the single post-repair verification.
The runtime did not report effective model identities. All three returned
successfully without edits or effects. The primary wrote the tests and source,
ran every validation gate, adjudicated the finding, and owned both Git
transitions. The final closed-world verdict for `P211-XRDP-LISTENER` is `PASS`.

For the RuntimeHostRouter packet, the same three authorized workers remained
read-only. `/root/p211_xrdp_contract` found the stop-order and configured-error
policy defects, then returned `PASS` after the primary repaired and tested both.
`/root/p211_listener_fixture` found the partial-initialization cleanup defect,
then returned `PASS` after the primary added rollback and its regression.
`/root/p211_validation_selection` supplied changed-surface guidance. The
primary independently ran every reported gate, the strict workspace checks,
and the complete provider-free Rust suite. No worker performed edits, Git
transitions, or runtime effects.

For the catalog-population packet, one bounded read-only worker specified the
provider readback and publication seam, one bounded read-only worker audited
the existing XRDP ownership and exact-stop gap for the next packet, and the
existing transaction reviewer performed one repair re-review. The primary
retained all source writes, validation, Git transitions, and runtime-effect
custody. No worker received live-effect authority.

The operator explicitly authorized subagents for Plan 0211 parallelism and
model-choice optimization for the current packet. The primary retains
architecture, overlapping-source custody, Git transitions, runtime effects,
finding disposition, integration, and final acceptance. Bounded workers used
the shared admitted P211 worktree: `gpt-5.6-luna` at low effort audited
acceptance text, `gpt-5.6-sol` at medium effort designed red cases and wrote the
disjoint SQLite transaction surface, `gpt-5.6-terra` at medium effort wrote the
disjoint pure handoff preparation surface, and `gpt-6-astra` at high effort
performed the closed-world transaction review. The primary integrated and
reworked the packet, and the original reviewer verified the four exact
findings once. No worker received Git or runtime-effect authority.

For the version 50 correction, `/root/configured_adoption_impl` used
`gpt-5.6-terra` at high effort and stopped without edits after proving the
configured provider cannot preserve a predecessor tunnel UUID. Three
read-only Design It Twice workers then explored strict resume, flexible
provider capability, and common-caller continuation interfaces. The completed
`gpt-5.6-luna`, `gpt-5.6-terra`, and replacement `gpt-5.6-luna` studies all
identified the catalog binding and XRDP witness as the durable identity; the
original common-caller worker was interrupted after failing to return within
the bounded design window, and none of its unreported work was used. The
primary selected the narrow route-continuation correction and retains all
source, Git, validation, and runtime-effect custody.

For the keeper-handoff qualification closeout, `/root/p211_quality_gates` used
requested `gpt-5.6-luna` low effort to run formatting and strict Clippy;
`/root/p211_doc_reconcile` used requested `gpt-5.6-sol` medium effort for a
read-only fact-owner audit; and `/root/p211_test_coverage` used requested
`gpt-5.6-terra` medium effort for closed-world changed-surface coverage mapping.
The runtime did not report effective model identities. All three completed
without repository edits, Git transitions, or runtime effects. The primary
accepted the qualification receipts, adjudicated the daemon-boundary coverage
gap as blocking, added and ran its focused regression, ran the comprehensive
suite, and retained integration and acceptance custody. During documentation
closeout, the same workers ran the production docs build, handoff and link
contracts, six selector-recommended workstation and Guacamole fixtures, and a
closed-world parity review. The review found and the primary corrected a
staging-versus-apply attribution error plus stale static-viewer guidance before
the primary reran the affected documentation gates.

For the proof-bound cold-recovery tracer, `/root/p211_doc_reconcile` used
requested `gpt-5.6-sol` medium effort to extract the frozen phase contract and
perform two closed-world invariant reviews; `/root/p211_test_coverage` used
requested `gpt-5.6-terra` medium effort to identify the public recovery seam
and perform concurrency review; and `/root/p211_quality_gates` used requested
`gpt-5.6-luna` low effort for focused validation inventory. The runtime did not
report effective model identities. All work remained read-only. The primary
discarded the first unsafe tracer, implemented the proof-bound replacement,
owned every test and source edit, ran the selected gates, and retained Git and
integration custody.

The assignments below are completed historical packets and grant no current
worker or write custody.

The Browser Session Manager prototype began as a serialized primary-owned
packet. After its first five provider-free interface tests passed, the primary
delegated only the disjoint tolerant profile-catalog importer. Read-only seam
inventories ran in parallel. None of the workers received Git authority,
runtime effects, or overlapping source custody.

The earlier shutdown packet used these completed disjoint assignments:

| Worker | Initial route | Exact scope | Return and stop condition |
| --- | --- | --- | --- |
| Provider-free fixture worker | `gpt-5.6-luna`, medium | Only primary-declared test modules for shutdown, clean install, interruption/replay, profile release, restart, and the joined end-to-end fixture; no production logic or module declarations | Return a patch, commands, and exact failing invariant within 30 active minutes or after one failed implementation attempt; stop if production edits or a contract change are required |
| Remote-view implementation worker | `gpt-5.6-terra`, medium | One primary-declared post-boot requalification and route-admission module plus its unit tests; no installer, shutdown, docs, Git, or live-runtime writes | Return a patch and focused validation within 45 active minutes; stop after two cross-lane clarifications, one failed approach, or any need to change the frozen interface |

The primary implements the shutdown and cold-install critical path while those
workers run. At the source join, the primary inspects and integrates returned
diffs without repeating accepted investigation, runs focused tests, and allows
at most one repair handback per worker. If interface churn makes either lane
coordination-heavy, cancel that worker and absorb the work into the critical
path.

The operator-directed version 40 transition makes P211 the primary writer for
documentation parity. The primary performs this packet without delegation and
limits writes to `cli/src/output.rs`, `README.md`,
`skills/agent-browser/SKILL.md`, the relevant installation and remote-view MDX
pages, inline documentation, RUNBOOK, this plan, and P211's catalog entry.
P207's feature-specific prose remains evidence for later reconciliation, not
content to publish before its corresponding source integrates.

Version 49 normally keeps review primary-owned, but the operator's newer
instruction authorized the bounded closed-world review above. It did not
authorize another worktree, live runtime effects, or broader discovery.

P205 completed the Service Model extraction and its integrated result is the
starting seam for this packet. P211 owns CLI help, README, the Agent Browser
skill, installation and remote-view documentation, and inline documentation
for this packet. P207 retains its unmerged tab-refresh source and generated
service-contract changes. P211 owns the new Browser Session Manager, Browser
Profile Catalog, independent state codec, CLI and process adapters, cold-shutdown module,
workstation-install routing, display selection, dashboard projection,
remote-view joining logic, and their tests; it will rebase after P207 before
integrating any P207 implementation.

## Evidence And Exit

| Requirement | Acceptance evidence | Current state |
| --- | --- | --- |
| One-command shutdown | `agent-browser shutdown` fixture returns success from healthy, drained, failed-upgrade, and partial-prior-run inputs | public route, empty-workstation idempotence, retained-profile release, protected-claim release, failed-upgrade independence, partial-prior-run completion, manager-only state, exact process termination, foreign-process preservation, and replay fixtures green; installed acceptance pending |
| Bounded completion | injected-clock tests prove fixed phase deadlines and exact escalation without production-scale sleeps | fixed phase deadlines, injected-clock controller and platform-adapter overruns, bounded daemon and command waits, and provider-free escalation receipt mapping are green |
| Complete owned shutdown | receipt proves owned units, timers, browsers, runtime hosts, dashboard, MCP, and owned containers are stopped | exact browser/daemon, fixed user-unit, fixed container, state-release, metadata, and verification adapters implemented; installed residue proof pending |
| Profiles become unowned | fixture proves profile data remains while runtime owners and leases are released | authority kernels and repository fixture green; public process fixtures release retained sessions and an active protected claim while preserving profile records and physical data |
| Metadata cannot veto | the controller interface accepts no coordination inputs and the fixed-sequence test passes | controller and platform adapter green; installed stale-metadata acceptance pending |
| Cold replacement | workstation and reviewed-candidate apply execute stop, migrate, replace, start, and readiness in that order | checkpoint `726563fb` makes both source routes forward-only after migration; all 181 affected installer tests and the source-free apply fixture are green; installed acceptance pending |
| Clean restart | post-start fixture proves one selected generation, one runtime host, one dashboard, and clients can make a fresh service request | joined disposable cold-install-to-first-use proves the selected generation, one reused runtime-host identity, one reachable reused dashboard endpoint, repeated fresh Service requests, and exact installed shutdown; independent host serialization, provider-free restart reuse, concrete disposable-Chrome worker reattachment, and the compiled-CLI shared-daemon title, snapshot, click, redirect, and close journey are also green |
| Independent profile catalog | cold migration imports only legacy profile definitions into the SQLite authority; malformed or contradictory legacy lease state cannot block lookup | the tolerant field-level importer is retained at the migration boundary, checkpoint `726563fb` makes SQLite authoritative for subsequent host loads and saves, and checkpoint `c0ff2768` durably archives typed non-vetoing rejection records |
| Shared browser sessions | Alice and Bob use one named-profile browser through independent named sessions; activity refreshes each heartbeat and ending either session preserves the other | manager, independent persistence, concrete adapter, lazy Service host, public named-session lifecycle routing, generic current-tab command routing, and the two-session compiled-CLI/runtime-host Chrome fixture are green |
| Disposable lifecycle | one named session reuses its compatible disposable allocation; another session receives another allocation; the final session closes the browser and the reaper removes only an exactly proven managed disposable directory | provider-free allocation, reuse, isolation, final close, configurable-delay reaping, hosted exact recorded filesystem deletion with foreign-sibling preservation, default host policy, and exact hosted-Chrome final-process termination are green |
| Bounded tab lifecycle | ordinary navigation reuses one session-current tab; first use adopts an unattributed bootstrap or creates one session-initial tab; explicit new-tab is the only further growth path within that session; close selects the most recently used remainder; session end removes live tabs | provider-free lifecycle, concrete adapter, real-Chrome restart and ordinary-command fixture, public named-session new/current-close routing, and the two-session daemon-process tab and close fixture are green |
| Current liveness | active requires a fresh heartbeat, existing recorded PID, and responsive CDP; bounded recovery ends dead sessions without replaying the interrupted command | heartbeat, bounded-recovery model, recorded-PID plus CDP checks, Service hosting, five-second manager-specific reattach timeout, concrete restart reattachment, full daemon-process command routing, and bounded exact-process recovery for a verified external unresponsive-CDP browser are green; unverified processes remain untouched |
| Legacy containment | ordinary session, browser, profile, tab, and display decisions remain unchanged when legacy lease, principal, owner, generation, and recovery records are contradictory | catalog import ignores unrelated malformed legacy state, the manager has no legacy-authority input, and hosted persistence plus multi-display selection remain independent of contradictory legacy session, owner, and display records |
| Trusted single-user profile | `--session` alone supplies attribution; a named profile reuses one healthy matching browser and requires no principal, hash, capability, sealed plan, or repair token | selector collision regressions, independent manager proof, named-session lifecycle routing, persistent generic command state, real-Chrome title and snapshot calls, and the full daemon-process command journey are green |
| SQLite runtime authority | one user-private transactional database owns configuration, profiles, sessions, browsers, tabs, handoffs, presentation intent, operations, generations, credentials, history, and cleanup; JSON is migration input or diagnostic export only | checkpoints `726563fb`, `c0ff2768`, `6390a48a`, and `d402f55b` make Browser Session State and Browser Profile Catalog reads and writes SQLite-only, add validated configuration, persist the compare-and-swap route-keeper authority plus connection catalog, and migrate absent v1 keeper state with exact-document fencing; public config mutation and provisioning coordinates/operation credentials are now SQLite-owned through `214dfc77`; remaining credential domains, bounded history, cleanup integration, integrity, backup, and cross-domain joins are pending |
| Forward-only legacy cutover | cold upgrade imports valid fields, archives typed rejections and source hashes, removes old units, variables, readers, processes, and owned Guacamole state, and never restores or falls back to the old architecture | checkpoints `726563fb`, `c0ff2768`, and `18c790bd` add the explicit migration phase, source hashes and read-only archive, typed non-vetoing rejection records, exact missing-source history, atomic database publication, no JSON fallback after database creation, no old-generation rollback, and removal of the hidden-viewer bootstrap variable and launcher; exact legacy provider cleanup remains pending |
| Protocol-level route keeper | the existing runtime host establishes warm XRDP sessions through supervised in-process Guacamole tunnels with no Chrome, profile, tab, manager session, or handoff | checkpoints through `536d58a9` establish the provider-free lifecycle, durable authority, exact XRDP proof, host claims, and complete runner baseline; `11f606a4` adds v4 route/transport identity, `8f60434f` adds configured connector adoption, and `45124f99` reconstructs exact predecessor-exit proof plus same-generation interrupted replay and durable adoption termination. Checkpoint `ef43de73` joins true interrupted-host restart and deterministic configured startup, with both comprehensive lanes passing in 842 seconds. Final source `7634325d` adds pre-provider route-user validation and successful two-route coverage; 55 focused keeper tests, format and strict Clippy pass again, and CLI native-other passes 761 tests with 57 ignored. Unchanged Service Model and other comprehensive evidence is retained. `f651384f` extends recovery to pending starts, failed recovery and retained stops with 765 native-other and 250 native-stream tests plus model/quality gates passing. `02ee03a2` qualifies live-supervisor health projection and current-generation remote admission with bounded pending-readiness waits; 769 native-other, 253 stream, 16 browser-host and two output tests plus strict quality/client/docs gates pass. Quarantine reconciliation, full capacity allocation and live acceptance remain open |
| Configurable capacity | `minimumReady=1`, warm target 4, maximum displays 6, density 4, queue 32, 90-second request deadline, and 10-minute scale-in cooldown are live user settings; lowering limits is non-destructive | checkpoint `b2a3ff39` exposes six atomic CLI/HTTP/MCP settings and live keeper policy synchronization, preserving existing records and fencing stale demand; checkpoint `2f138944` adds provider-free qualified cooldown retirement and its public setting. Checkpoint `d25321dd` qualifies additive publication preserving active evidence; `214dfc77` qualifies durable automatic one-route provisioning through injected effects. Installed capacity acceptance remains pending |
| Display allocation and overflow | one browser per display while capacity can grow; after maximum displays, new browsers use the least-loaded display up to density; occupied browsers are never routinely migrated | `a9595ee2` qualifies bounded admission and durable demand; `f3d0a758` adds durable queue admission; `8ab466e8` joins exact journal recovery. Installed overflow and complete recovery acceptance remain open |
| Durable admission queue | bounded depth and deadline, priority with aging, duplicate coalescing, restart distinguishes queued from effects already started | f3d0a758 qualifies SQLite admission, bounded retention, generation/sequence/attempt fencing and duplicate results; `8ab466e8` qualifies exact interrupted-open journal reconciliation. `89432d9b` qualifies navigation and handoff journal recovery with exact payload/target evidence. Active-view control priority and installed recovery acceptance remain open |
| Shared desktop control | handoff activation, focus, maximize, capture, pointer, and keyboard share one generation-fenced Desktop Services control lease; observers remain connected and prior controllers become view-only on transfer | version 49 contract frozen; implementation pending |
| Crash-consistent open | one operation durably reserves session, browser, slot, and handoff intent before effects and publishes the observed browser, tab, display, and handoff atomically afterward; stale-generation effects cannot commit | checkpoints `759f12e9`, `56a0558e`, `e1a3edcd`, and `da9438a7` prove durable exact intent, per-owner fencing, browser and tab observations, keeper-bound slot and opaque-URL preflight, atomic session-plus-handoff publication, exact multi-route replay, durable `Prepared` recovery, exact observed-launch adoption without relaunch, base-state conflict rejection, and a replay-stable cleanup obligation for unproven launch recovery; `6526ccef` implements selected-profile causal discovery with exact CDP PID and process identity proof; native-platform and installed recovery acceptance remain pending |
| Durable cold-start reconstruction | from zero provider, Guacamole, XRDP/Xorg, route-keeper, and browser processes, one verified route makes service usable, remaining warm routes reconcile in background, and an ordinary request receives a ready opaque handoff without operator repair | not yet implemented; version 45/46 evidence proves the hidden-viewer and split-inventory architecture is insufficient |
| Bounded persistence and history | live SQLite stays within 96 MiB, exact URL history within 64 MiB, routine database/WAL/backup within 128 MiB, summaries retain long-term lifecycle evidence, and verified backup recovery is automatic | version 49 contract frozen; development size and corruption tests pending |
| Disposable retention | default 24-hour inactivity, 20 profiles, and 10 GiB are live settings; oldest inactive sessions expire first and active or named profiles are never evicted | version 49 contract frozen; implementation and quota tests pending |
| Dashboard browser identity | each active tile represents one concrete browser and selects its desktop viewer while raising its primary window | independent status projection, browser-parent tile identity, static desktop-viewer join, and manager-owned focus and maximize routing green |
| Login handoff | #190 regression proves a normal same-site authentication redirect leaves a usable durable handoff or typed authentication-required state | the compiled-CLI Chrome fixture follows a same-site `/protected` to `/login` redirect and retains the same ready opaque handoff through `/account`; the dashboard same-origin post-auth return contract is green; one authenticated dashboard replay ended without a terminal verdict and its teardown-hang regression is repaired, so authenticated rendering remains pending |
| Ready remote view | an ordinary route-free open returns `operatorVisible.state=ready` and an opaque `/remote-view/<handoff-id>`; repeated open is idempotent; doctor, status, capacity, preflight, and checkout agree | checkpoint `18c790bd` removes static-route and hidden-viewer fallback; checkpoints `f494f273`, `d0aa34de`, `9b96e44f`, `d79a387d`, and `da9438a7` qualify the catalog, exact XRDP source seams, runtime-host supervisor, and keeper-backed opaque manager handoff join, but the shared control lease and true cold-start and live acceptance remain required before this row can become green |
| Simple interface | default operator path requires no preflight digest, transaction ID, revision, census code, rollback choice, or manual recovery command | the full compiled CLI and runtime-host journey shares one browser, drives independent commands, returns the ready handoff, and closes cleanly with none of those inputs; compiled help and repository documentation expose the same ordinary path |
| Legacy hot-upgrade containment | hot transaction mutation is not reachable from the default install or upgrade path | default apply bypasses prior transaction convergence and creates no transaction; explicit legacy inspection and recovery commands remain |
| Documentation parity | CLI help, README, Agent Browser skill, docs site, and inline comments describe the same workflow | the post-`3922f138` closeout reconciles every required repository surface with SQLite `Ready` route-keeper handoff semantics and failure before browser effects; the remote-view documentation contract, link checker, and production docs build pass |

Exit requires all rows green against one frozen source candidate. Provider-free
tests include the existing shutdown, session, browser, tab, profile, redirect,
and foreign-process invariants plus forward-only field-level migration,
SQLite integrity and backup recovery, history compaction, live configuration,
operation replay at every pending-effect boundary, generation fencing,
protocol-keeper supervision, minimum-ready versus warm-target projection,
capacity queue priority and restart behavior, overflow display sharing,
Desktop Services lease transfer, missing-tab recreation, single browser
replacement, disposable quota cleanup, and proof that runtime source contains
no legacy route, inventory, or bootstrap-variable reader.

Development acceptance uses one frozen installed candidate and includes:

1. Three consecutive isolated cold starts from zero provider, Guacamole,
   XRDP/Xorg, route-keeper, and browser processes. Each first ordinary open
   reaches a ready opaque handoff within the provisional 90-second deadline,
   with no manual action, stale ready state, production mutation, or unexplained
   residue.
2. Runtime-host restart with the same handoff; keeper disconnect; active-display
   loss with at most one browser replacement; and a failed 90-second recovery
   that proves no Chrome launch before display readiness.
3. Maximum-display overflow sharing, automatic focus/control transfer through
   Desktop Services, live capacity-setting changes, and crash recovery from
   every pending-operation phase.
4. Named-profile browser authentication state retained through relaunch;
   missing-tab recovery at the last committed top-level URL; no replay of
   clicks, forms, downloads, or other side effects; and idempotent repeated
   `remote_view_open`.
5. Disposable inactivity, retained-count, and byte-quota cleanup; SQLite size,
   WAL checkpoint, compaction, backup restore, and corrupted-primary evidence.
6. A fresh OS process and resource census after every provider or browser run,
   with exact task-owned cleanup and production unchanged.

The final journey fails if an operator must choose a route, desktop, or
display; generate or copy a hash, capability, token, code, or sealed plan; edit
runtime state; run a repair command; or interpret raw Guacamole state. It also
fails if Browser Session Manager reads a legacy environment variable or
generated inventory, an infrastructure Chrome viewer exists, a stale display
is reported ready, a changed physical display requires operator action, or an
old installation can veto or roll back the forward cutover.

## Stop Condition

Stop before production installation or shutdown, production profile-data
deletion, unscoped process or container termination, external ingress
publication, release, or mutation of another lane's checkout. Development-only
provider credentials, processes, profiles, and isolated runtime replacement are
authorized after the provider-free candidate freeze and current production
readback. Stop and reconcile if P207 publishes an overlapping interface change
before P211 integration. Stop an implementation tactic after its stated bound
and retain partial evidence; do not silently increase concurrency, retry count,
or the cumulative plan budget.
