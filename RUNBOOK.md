# Runbook

Fresh-session entry: [compact P222 handoff](P222-HANDOFF.md). The operator
resumed full Plan 0222 delivery on 2026-10-03, including in-scope repair/retest.
The operator resumed with 50000 additional tokens on 2026-10-03.
Retain the prior stop/pause accounting of 511993; checkpoint before 561993
on the same cumulative meter, or before 50000 new tokens if it stays paused.
Historical consumption baseline 1040108 remains preserved; the new goal-meter
ceiling does not erase earlier effort. Single primary, p221 runtime only.
Check consumption at each substantive packet and before expensive follow-up;
retain the stricter 25000-token no-blocker-removal checkpoint and 30-minute
no-outcome reassessment. First-client view/control/automation/close precedes
later milestones; all five milestones remain required and unaccepted.

Current execution record. [Preserved history through the P222 foreground repair](RUNBOOK-history-2026-10-03-through-p222-focus.md) retains prior evidence, earlier archive links, and failed attempts without alteration.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)
- [P222](docs/dev/plans/0222-2026-10-02-real-client-browser-delivery.md); [P221 superseded history](docs/dev/plans/0221-2026-10-01-browser-product-acceptance.md); [P220 preserved history](docs/dev/plans/0220-2026-09-28-remote-view-consumer-integration.md)

## P222 current checkpoint | 2026-10-03

### Partial publication guard | 2026-10-03 23:42 UTC

Diagnosing-bugs fixture reproduces the mismatch trigger through the actual
installDevelopmentRuntime publisher with a disposable executable and injected
unit observations. Before the fix, non-activating publication admitted a
replacement while one old dashboard/host member remained live. Red command:
pnpm test:development-runtime, Missing expected exception. The minimized seam
requires only one live unit and one replacement binary.

The publisher now refuses non-activating installation before any write when
any development unit has a PID, is running, transitioning or has unknown state.
Inactive installations remain supported. Ordinary coordinated activation and
live-executable doctor checks are unchanged. Tests address each of the three
unit roles and unknown state, assert selection and all executable-bound files
unchanged, and retain existing activation ordering checks. Focused fixture
passed. Live p221 guard refused all three active units and preserved bindings;
private evidence script p222-live-partial-publication-guard.mjs. No binary was
published, service restarted or browser launched by this verification.
Documentation, script help and CLI help describe the new guard. Docs build,
syntax and format checks passed; strict workspace Clippy passed in 16.81
seconds with cache disabled. The first Clippy attempt was cancelled after cache
startup delay (exit 143); it is not a passed check. This closes
the demonstrated non-activating publication trigger; handoff browser-closure
recovery and explicit TTL semantics remain pending independently.
Memory disposition queued: 20261003T234422Z-remember.json, job
553b0001-9def-49c7-934e-c8001d5eb45a; queue acceptance is not retrieval proof.

### Dashboard generation recovery | 2026-10-03 23:36 UTC

Authenticated POST /api/service/request reproduced HTTP 502 and
Durable handoff owner preparation failed for dashboard-service-backend.
Both dashboard processes still ran 91d28506c445 while the host and installed
manifest selected a0a18b271f31. Child preparation uses current_exe.
Restarted only p221 dashboard backend and ingress units. Current processes
93104 and 93105 now use a0a18b271f31; host 95196 and browser 97181 survived.
The identical authenticated request returned HTTP 200, success true, opened,
and operatorVisible ready. Private evidence:
evidence/p222-dashboard-generation-recovery.json. This restores HTTP handoff
resolution but does not establish public pixels/input or correct TTL semantics.

### Operator contract correction | 2026-10-03

Operator rejected indefinite browser retention in 8b596861. The default
five-minute inactivity rule was correct. Durable handoffs must survive browser
closure; a requestor may explicitly extend TTL. Hours, days and until handoff
closure are candidate settings, not implemented or validated options.
Browser closure and handoff closure must be separate lifecycle operations.
The prior idle-survival proof demonstrates the implemented behavior only and
is withdrawn as acceptance of the intended contract. The p221 candidate still
contains the incorrect unconditional retention; production is unchanged.
Next repair must restore default reaping and prove the same handoff recovers
its logical profile/tab intent after browser closure, with bounded explicit
retention and separate handoff expiration or closure. No runtime effects were
performed during this correction readback. Memory disposition queued:
20261003T233352Z-remember.json, job 0b981e5a-3584-4021-bd00-980620d8d224;
queue acceptance is not retrieval proof.

### Idle retention repair | 2026-10-03 22:23 UTC

Diagnosing-bugs loop reproduced remote_view_tab_handoff_session_unavailable for
handoff 46e0c328-5220-466e-b0df-148c2be57404. Exact session history proves
heartbeat_expired, 311600 milliseconds after the final recorded activity.
The temporary diagnostic keeper ended and ordinary five-minute cleanup removed
the session/browser. Repeated replacement links did not satisfy stable operation.

Source 8b596861 fixes the owning Browser Session Manager. A valid exact handoff
join retains its session against inactivity and quota eviction; reconnect uses
the original live session even past its idle deadline. Explicit close still
releases the browser. Invalid, dangling and closed historical bindings confer
no retention or additional access. The existing handoff join remains identity
and access evidence, not automatic permission to recreate closed browsers.
CONTEXT, help, README, agent skill and remote-view docs describe the behavior.

Regression advances the real manager clock past expiry, reaps, reconnects the
same session/tab after serialized state reload, explicitly closes, and checks
that an invalid dangling handoff does not pin the browser. Original red:
retained operator session was reaped. Cargo Signal red job 220906-a8e1aab209b1;
27 focused manager tests passed in green job 221240-d0e8b34c0c97. Strict workspace
Clippy 221352-9e3262c484eb, format check, handoff docs and docs build passed.
Qualified candidate build 221536-c1f13d77b826 passed in 196.787 seconds.
Private logs and receipts remain under the retained p221 evidence/cargo-signal.

Published only p221 generation 0.28.0-a0a18b271f31, SHA256
 a0a18b271f31a206a45bab2244b0626e6afeac4f8dc856958490ac6a59c9d2a9.
Evidence/p222-handoff-retention-install.json confirms production/default unchanged.
Old synthetic task session closed normally, with zero-browser/session/profile
process census. Its keeper stopped with the intended refusal to cold-launch.
First host restart used stale loaded unit metadata, causing executable_drift and
systemd start-limit refusal. The manifest on disk already matched the new binary;
daemon-reload and reset-failed restored the empty namespaced host without manual
manifest edits or production unit changes. Two pre-launch failures are retained
in p222-retention-live-launch.log and p222-retention-live-launch-2.log.
Host 95196 live executable SHA matches the published candidate.

Actual normal AuraCall launch p222-retention-live-launch-3.log passed. Browser
9b763bf0-edf9-45b3-9558-b680c41d045e, PID 97181, CDP port 37595,
handoff d31f27d9-fa86-4891-8a62-db8ddeaee290. Evidence/p222-retention-live-client.json
owns the full returned identity. No keeper was started for this browser.
Live proof exec session 40903 reads SQLite in read-only mode, checks unchanged
activity and exact process/session identity until 65 seconds past recorded idle
expiry, then resolves the same handoff once. Bound 430 seconds plus 90-second
resolve. Passed at 22:28 UTC: 66393 milliseconds beyond the original idle
deadline, unchanged last activity before resolve, same browser/session/handoff,
and operatorVisible ready. Session ordinal 22. Receipt:
evidence/p222-retention-live-proof.json. The browser remains intentionally live
for the operator; no keeper is running. Public operator pixels/input remain unproved.
All five milestones remain unaccepted; production and operator state preserved.
Memory disposition queued: 20261003T222833Z-remember.json, job
3d1c044f-069b-4bd7-8972-295b485fef1f. Queue acceptance is not retrieval proof.


### Operator workflow resumed | 2026-10-03 20:00 UTC

Operator accepted the recommendation to resume with 50000 additional tokens.
Full Plan 0222 scope and all five unaccepted milestones remain unchanged.
The goal API still reports paused at 511993 after explicit user resumption;
this is accounting state, not a fresh allowance. Preserve that baseline and
bound this continuation by 50000 additional tokens. Prior recommendation-only
turn was no_progress; this run advances installed client evidence.

Fresh census found the historical process IDs absent and p221 browser/session/tab
state empty following machine restart. Restored the retained p221 desktop using
normal start 1, installed control HTTP on 19103, gateway/verifier units, and
Agent Browser dashboard units. No production unit was started, stopped or changed.
The generated host unit lacked its previously standalone Remote View binding;
a p221-only systemd drop-in now supplies the same origin/pool/public origin.
Restarted only the empty p221 host. Provider local doctor passes, including
native control, audio, private Guacamole verification and exact executable adoption.
Public operator acceptance remains unproved.

Actual unmodified AuraCall launcher passed with canonical CDP port 37291,
PID 47160, browser 72c447c5-ee8e-48c6-ba66-d0ab22945e4f and session ordinal 19.
Same qualified binaries: Agent Browser 91d28506c445 and Remote View f7869224d96e.
Private evidence: p222-operator-run-client.json and p222-operator-run-census.json
under the retained task evidence root. A synthetic BLUE 472 page/input test is
prepared on the addressed Example Domain tab. Fresh authenticated opaque handoff
46e0c328-5220-466e-b0df-148c2be57404 was supplied for operator sign-in/control.
Unauthenticated public HTTP returned 401; this does not prove viewer failure.

Bounded keeper exec session 18897 checks exact live browser ownership before
profile-bound get url, refuses cold launch, and ends after 27 ticks/1175 seconds.
Await operator observation, read the synthetic input result through the actual
client CDP dependency, then continue automation and normal close with fresh census.
Do not accept milestone 1 before the complete workflow succeeds.

Memory disposition: queued; installed restart recovery and actual client evidence.
Receipt /home/ecochran76/.graphiti-openclaw/state/closeout-memory/20261003T202218Z-remember.json,
job 2b0817f8-9961-486b-a707-4875e8364a6c. Queue acceptance only; no delivery polling.


### Requested spending checkpoint | 2026-10-03 19:31 UTC

Execution is stopping at the operator-requested checkpoint before the active
500000-token meter ceiling. Last source-checkpoint readback: 476924. The goal
pause receipt owns final accounting; preserve the historical 1040108 baseline.
Full Plan 0222 scope is unchanged. No milestone is accepted. Resume requires
operator direction and a new spending bound; do not continue effects merely
because the previous goal was larger than this completed repair.

| Milestone | Current evidence and remaining gate |
| --- | --- |
| 1 actual AuraCall browser | Normal launcher, canonical CDP attachment, retained CLI resolution, continued synthetic automation, normal close and clean browser census passed. Public authenticated viewer pixels/input are unproved. |
| 2 persistent login | Manual account login and restart persistence unproved. |
| 3 second client | Shared/separate tabs, multiple profiles, replay and peer-preserving closure unproved. |
| 4 recovery/capacity/input | Real grant expiry renewed under the same handoff/session/tab. Other disruptions, revocation, operator stop, capacity growth/shrink and desktop/mobile input unproved. |
| 5 reproducible delivery | Qualified p221 candidates published with protected production/default identities. Clean installation/update/doctor, target platforms and protected integration remain incomplete. |

Source: Agent Browser 2b4fad71 on platform/p220-remote-view-consumer adds normal
remote-view resolve for the existing handler with allowReopenClosed=false.
AuraCall 939e7eda8 on fix/p222-agent-browser-cdp-endpoint adds bounded retained
readiness waiting, preserving initial build proof and handoff identity.
Compiled launcher SHA256:
2e75ecb0848592f141e75da8c493cf93774c18179ac455aa36b7e980dc78bf37.
Original regressions were red. Current gates: 21 AuraCall launcher tests,
typecheck/compilation, 48 focused Agent Browser remote-view tests, strict
workspace Clippy, format check, handoff docs check and docs build all passed.
Earlier unchanged inventory and 19 pool/expiry gates retain their own receipts.

P221 installed generation 0.28.0-91d28506c445, SHA256
91d28506c445887a99a3c09092406de2be23ee2996c2020f6bd3d9c603144aa2.
Task host 102413 owns 5051. Provider source/config/credentials are preserved.
Publication receipt evidence/p222-readiness-resolve-install.json proves
production and default development unchanged. The first publication failed its
production guard because a systemd unit read returned a transport error;
fresh productionSnapshot exactly reconciled with the original pre-publication
snapshot before one guarded retry. Preserve failed receipt and reconciliation
artifact evidence/p222-production-guard-reconciliation.json. No guard bypass.
An initial retest was refused before browser effects by stale daemon custody;
replaced only the positively identified empty task host after zero-process census.

Actual normal-client job-20261003T192642Z-a097b9158c16 passed in 11.041 seconds.
Browser browser:p221-auracall:a231fd91-e910-492f-8d8b-a7274e7efe2a returned canonical
CDP port 55557 and durable handoff 41bd8345-8883-40e3-b0ca-7050d7340906.
Normal CLI resolution passed for that same identity; real AuraCall CDP dependency
attached, checked Example Domain and set/read a synthetic DOM marker.
Task receipts: evidence/p222-normal-resolve-result.json,
evidence/p222-client-continuation.json and evidence/p222-final-client-close.json.
Normal close returned browser_closed for the exact session. Fresh OS and SQLite
census evidence/p222-final-client-census.json shows zero browsers/sessions/tabs,
no profile process residue and addressed PID absent. This closes the task browser,
not the whole provider installation. The bounded keepalive also finished; no
client/helper build job remains running. The now-closed operator URL is historical
evidence and must not be presented as a live viewer link.

Next bounded continuation: use the same qualified client path, establish actual
operator authentication/viewing/input, then continuation/closure and acceptance
for milestone 1. Preserve all remaining milestones and target p221 only. Do not
change production, reset profiles, repeat completed source gates without cause,
or open a successor plan to erase the unmet acceptance scope.

Memory disposition: queued; qualified source and installed actual-client repair
checkpoint. Receipt /home/ecochran76/.graphiti-openclaw/state/closeout-memory/20261003T193411Z-remember.json,
job 7615e63e-0e6d-4511-abd6-294974aa09db. Queue acceptance only; persistence and
retrieval were not polled. Do not retry this write without receipt reconciliation.


### Expiry repair installed and readiness blocker | 2026-10-03 19:01 UTC

Source repair 09e2d8ea validates an expired published issuance against the current
assignment at admission time, then permits the session manager's single bounded
replacement. It preserves the handoff/session/tab identity and issuance history.
Revoked, mismatched and invalid grants fail closed; dashboard reads remain
read-only. Original real host/SQLite/provider regression failed with
remote_view_tab_view_resolution_failed. After repair, all 19 pool tests passed,
including restart renewal, bounded replacement failure and invalid/revoked
issuance rejection. Strict workspace Clippy, format check, remote-view handoff
documentation check and docs build passed. Candidate build passed in 203 seconds.
Cargo resource configuration used four jobs, 10 GiB admission claim/process cap
and 8 GiB MemoryHigh with the existing 16 GiB host reserve unchanged.

Only p221 published: 0.28.0-35c7d911ab0e, SHA256
35c7d911ab0ed996aaee767fb38998687172789b9b5b900db29ffff2dd77b5b5.
Install receipt evidence/p222-expiry-install.json proves production and default
development unchanged. Host 4113086 owns 5051 and its live executable digest
matches the candidate. Old host 3637734 stopped after exact ownership and an
empty browser/profile process census. Retained task provider is unchanged.

Cleanup preflight found task disposable utility PID 4011042, start token
24852233, orphaned after the earlier incorrect profile-free CLI probe. SIGKILL
was pending while it was pinned to CPU 13. Cgroup was not frozen. Restoring this
exact task process's affinity to the available CPUs caused immediate exit.
Underlying CPU scheduling cause is not established. Probe GPU/crashpad also
exited; fresh census was clear before host replacement. No production or
operator process was targeted.

New real AuraCall fixture job-20261003T185636Z-c84bf906725a failed because AuraCall
rejects status=converging immediately. It retained browser
browser:p221-auracall:b03cd54b-608e-40d7-abe0-515d9b234d02, PID 4115214, session
session:auracall-p221-chatgpt:p221-auracall:17, handoff
56f33e34-d813-4c63-bc08-5d5b9bc74e8d. Local authenticated resolution later passed:
status=opened and addressed-tab operatorVisible.state=ready. Profile-bound
keepalive job-20261003T185831Z-83d1ad675259 verifies the same live browser and
Example Domain every 45 seconds, with 27 ticks total and a 1250-second command
bound. Session expiry readback confirms activity refresh; this keeper refuses a
cold launch when the addressed browser is missing. It is diagnostic custody,
not a successful AuraCall launch result or clean final shutdown.

Public operator viewing/input remain unproved. The updated authenticated handoff
was supplied for operator sign-in. Actual elapsed-time expiry renewal PASSED:
evidence/p222-expiry-live-renewal.json records old expiry 1791054100894 and new
expiry 1791054404500 with the same handoff/session/tab, ready local resolution,
and the exact installed source/digest. This is the original expiry symptom's
installed green readback, not public pixel/input acceptance.

Next source repair is a bounded same-handoff readiness wait in AuraCall,
preserving initial build proof and verifying response identity. The CLI has no
service request subcommand; an attempted call was rejected before effects.
Expose the already-supported service_remote_view_handoff_resolve through a
normal remote-view resolve CLI entry point, then use it to await readiness.
Do not repeat remote-view open to poll readiness. This parser/client repair can
use the existing live host's compatible resolve handler without restarting the
retained browser. Check normal command round trip before another actual-client
run. No milestone is accepted; all five remain open.

Active goal meter at this checkpoint: 283710; stop before 500000. Keepalive
remains a bounded diagnostic worker and must be polled by its exact job handle.
Memory disposition: queued, source-backed expiry repair and installed elapsed-time
renewal proof. Receipt /home/ecochran76/.graphiti-openclaw/state/closeout-memory/20261003T190725Z-remember.json,
job 908cac61-0ec4-4aed-ac24-b9d23c9ff4ab. Queue acceptance only; persistence and
retrieval were not polled.

Readiness repair in progress: the AuraCall regression reproduces the real
status=converging rejection in six milliseconds; its fix is not applied yet.
CLI retained-resolution regression also went red. A narrow remote-view resolve
parser entry now addresses the existing handler with allowReopenClosed=false;
focused green confirmation passed: one test, job-20261003T190904Z-8226733210b3.
This new command remains unqualified/unpublished: all five required documentation
surfaces, malformed-ID regression, final format/Clippy/build and normal retained
command round trip remain pending. Current installed runtime remains the fully
qualified 09e2d8ea expiry candidate; do not confuse it with this parser draft.


### Full-goal client checkpoint | 2026-10-03 18:35 UTC

The active goal meter was 138453 tokens; the historical 1040108 baseline is
unchanged. All five milestones remain open. AuraCall normal-client fixture
job-20261003T181549Z-ab2b9726eb74 passed and returned the canonical CDP endpoint,
selected stealth Chromium build, and handoff 070a42bd-268c-4c4e-ae93-b65c8c1055ca.
Authenticated public pixels/input were not proved. The task viewer encountered
the public authentication gateway; local authenticated resolve subsequently
failed remote_view_tab_view_resolution_failed after the published grant expired.
Later readback reports remote_view_tab_handoff_session_unavailable; the earlier
operator link is no longer acceptance evidence. Retain both failed receipts.

Diagnosis: published-grant resolution rejects expiry before provider resolve;
manager renewal currently handles pending provider-terminal grants only.
The read-only dashboard must stay read-only. A source expiry regression at the
real host/SQLite/provider seam is pending in browser_session_pool_tests.rs.
Cargo Signal job-20261003T183128Z-ee8388bcab42 waits for memory admission; no red
or green test verdict exists yet. Candidate repair is a private draft only,
not applied or published. Its exact assignment/application validation still
needs qualification. No gate or runtime acceptance is claimed from that draft.

Reassessment: simplify to this regression and expiry repair; do not branch into
later milestones. Preserve the prior issuance ledger and handoff identity;
validate current assignment and retained grant before bounded renewal. Next
actual-client run must bind the viewer check and keepalive to the actual client,
then prove automation and normal close. The current close command is pending;
fresh cleanup/census is required, including retained task probe GPU/crashpad.

Memory disposition: not_durable; incomplete diagnosis and unvalidated draft.


### Gateway/auth service recovery | 2026-10-03

Operator explicitly requested restoration. Linked and started the existing
installation-generated p221 gateway/verifier user units without regenerating
config, credentials, desktop, ledger or provider bytes. Gateway PID 3714419 owns
19102 and authentication 19104; verifier PID 3714420 is active. Both exact unit
cgroups and live executable SHA256 match installed provider f7869224. Control
PID 3681458 remains on 19103. Production control remains separately on 19096.
Readback artifact: task evidence/p222-gateway-auth-recovery.json.
Installed `doctor --json` returned exit 0 and ok=true: config/ownership,
three runtime-adoption checks, journal/registry, local desktop, native control,
audio/package, private stack/permissions, Guacamole and gateway pass.
Public routing remains unknown/not requested; authenticated viewer remains
unknown/acceptance receipt required. Inventory probe passes with one retained
assignment. Fresh managed state still has zero browsers and zero sessions.
No client-launch fixture repeated; its prior single-run instruction remains
respected. Next: a newly authorized actual client launch through this restored
provider, then exact addressed-tab/viewer/input/automation/close evidence.
This is local service readiness, not outcome-1 or full-plan acceptance.

Memory disposition: not_durable; routine task-process recovery with transient readiness and no client/viewer acceptance. Receipt /home/ecochran76/.graphiti-openclaw/state/closeout-memory/20261003T174418Z-remember.json.

### Diagnosing-bugs inventory loop | 2026-10-03

Applied the diagnosing-bugs skill. Original symptom is retained service job
r538507 remote_view_runtime_inventory_unavailable. Minimized loop sends the same
read-only application inventory envelope to the same /v1/consumer endpoint;
no acquire/view/launch request is repeated. Command:
python3 /home/ecochran76/.local/share/agent-browser-dev-p221/evidence/p222-inventory-probe.py.
Two pre-fix runs failed in under one second with connection_refused.
Ranked predictions: absent task control process, wrong bind/config, installed
startup failure. Census found no task listener/process on 19103; production
control 19096 remains distinct. Installed p221 digest still matches f7869224.
Only variable changed: started retained installed control server with existing
config/control home/token and capacity 930:60030 through host-adapter installed.
Process 3681458 owns 19103. Two post-fix runs passed, observing p221 pool and one
retained assignment. No identity, credentials, desktop or pending key replaced.
Immediate cause confirmed: absent task control server. Why the previous server
exited is NOT established; this recovery is not a persistence/clean-install gate.

Regression seam is the retained live read-only inventory probe; no synthetic
unit test was added for missing live process. Probe intentionally retained as a
clearly named diagnostic artifact outside Git. No tagged instrumentation added.
Original browser launch has NOT been repeated because the one-run boundary was
consumed. Thus the provider inventory boundary is repaired, while original full
client workflow remains unverified. Gateway/auth 19102/19104 remain absent;
generated gateway systemd unit is inactive. Next: restore those retained task
services, verify live readiness, and request another client run only after that
concrete prerequisite is proved. Zero browser/session state remains retained.

Memory disposition: queued; receipt /home/ecochran76/.graphiti-openclaw/state/closeout-memory/20261003T173815Z-remember.json, job 113a71b5-ca27-4f5b-911e-d4a5f9b4d485. Queue acceptance only.

### Packet publication and single-run result | 2026-10-03T17:27Z

Existing candidate job job-20261003T165755Z-3973d85a7739 passed at 17:11:23Z
(816.599 seconds including capacity wait). Installed source eefc26d1:
0.28.0-5474ca235ca9, SHA256
5474ca235ca92ea54ee7890033614515c6e3aa06b261107dadcc803deb34788a.
Publication receipt evidence/p222-inventory-install.json proves production and
default development unchanged. Successful publication job
job-20261003T172504Z-f6f17dd10d32 used prior no-activation publication mode.
First automatic-activation job job-20261003T172359Z-a7b799b9678e failed the
systemd listener ownership gate and restored prior generation. Disabled/stopped
only its p221 systemd units; retained standalone dashboard listeners preserved.
After exact executable/home identity and read-only zero-browser checks, stopped
old task host 3071968 and launched installed candidate as host 3637734.
Port 5051 and fresh service response provenance bind to that exact new host.

The existing actual-client fixture ran exactly once with the specified provider
environment: job-20261003T172606Z-11e710aea592, exit 1. Service job r538507 failed
at 17:26:06Z with remote_view_runtime_inventory_unavailable. Full failure receipt
is evidence/p222-inventory-client-failure.json. No canonical attachment or ready
handoff was reached. Provider ports 19102–19104 remain absent. Fresh post-run
read-only state: zero browsers, zero sessions, no p221 profile process residue.
Do not retry this fixture without reconciling the provider blocker. Next action:
restore retained task control/gateway/auth through their supported entry points,
verify exact installed provider identity/readiness, then obtain authority for
another actual-client run if the single-run instruction still bounds the packet.
No milestone accepted. Prior token baseline remains 1040108 plus new work;
accounting is not reset by the publication or this checkpoint.

Memory disposition: queued; receipt /home/ecochran76/.graphiti-openclaw/state/closeout-memory/20261003T173010Z-remember.json, job da2a56db-46cb-47dc-8485-f60d9f7480fc. Queue acceptance only.

### Bounded packet current evidence | 2026-10-03T17:06Z

User explicitly authorized this packet. Retained cumulative token baseline remains
1040108; the 25000 additional-token no-progress checkpoint still applies.
AuraCall commit a74d06db7 consumes canonical CDP endpoint with legacy fallback
only when absent. Tests cover precedence, explicit ports, prohibited protocol,
credentials/query/fragment and malformed endpoints. All 17 focused tests passed;
typecheck and tsconfig.build.json compilation passed. Initial typecheck caught a
new logger mock type error; corrected before final qualification. Compiled launcher
SHA256 b2e535e074e693465c959d8ecc3c97678412db7947735937246557a3ba770919.
Unrelated AuraCall .tmp/ preserved; no user runtime installed.

Agent Browser source remains eefc26d1 without Rust changes. Reuse the qualified
20 service_inventory tests, format and strict workspace Clippy at that identity.
Candidate build job job-20261003T165755Z-3973d85a7739 remains LIVE, waiting at
Cargo admission reason memory_pressure, active=0. Host available memory roughly
27 GiB versus required 30 GiB. Poll that same job; do not restart on timeout or
bypass admission. Its 1800-second command timeout remains authoritative.
Logs/receipts: task evidence/cargo-signal. No new inventory candidate installed.

Fresh read-only managed store contains zero browsers; no p221-profile browser
process was observed. Old host/dashboard PIDs 3071968/3071904/3071905 remain live.
Provider control/gateway/auth ports 19103/19102/19104 have no listeners; earlier
exec handles are stale locators. Retained desktop resources remain untouched.
Prepared task evidence/p222-publish-inventory.mjs uses the prior publication's
p221 namespace, paths and seven explicit ports, with unchanged-production/default
checks. Run only after build succeeds and fresh ownership readback.
The actual-client fixture has NOT run in this packet: publishing and running an
old binary would not test the requested repair. Preserve the single-run boundary.
Next: poll build, publish qualified p221 candidate, run fixture once and retain
its full result. No outcome or milestone accepted.
Memory disposition: queued, source-backed AuraCall endpoint repair;
receipt /home/ecochran76/.graphiti-openclaw/state/closeout-memory/20261003T170928Z-remember.json,
job 4f9c7c80-55aa-4b3a-aac5-fea2f2dbeb99. Queue acceptance is not retrieval proof.


Plan 0222 remains OPEN; none of five milestones accepted. Product lane PL-PLATFORM.
Custody: agent-browser-p220, platform/p220-remote-view-consumer. No delegation.
Previous checkpoint is preserved verbatim in
[readiness resume history](RUNBOOK-history-2026-10-03-p222-readiness-resume.md).

Operator resumed the same delivery sequence, requested Cargo Signal and progress
checks every 25000 additional tokens, and a stop before 1000000 tokens. Latest
new tracker 285778 plus prior retained checkpoint 748065 equals 1033843 before
closeout overhead. The cumulative crossing was missed between reads; stop now.
Do not reset this accounting. No runtime effects after the boundary read.

| Requirement | Current evidence | Remaining gate |
| --- | --- | --- |
| Actual AuraCall Stealth/view/control/close | AuraCall a74d06db7 qualified/compiled; Agent Browser eefc26d1 installed p221 0.28.0-5474ca235ca9; single actual-client run failed before launch with remote_view_runtime_inventory_unavailable (r538507) | Provider inventory and p221 gateway/auth restored; local doctor and inventory probe pass; public routing/authenticated viewer remain unknown; another client run remains pending; viewer/input and close unproved |
| Persistent authentication | NOT RUN | Reviewed login, automation and restart/reconnect with same handoff |
| Shared operation | NOT RUN through real clients | Second client, stable-key replay, peer-preserving and final close |
| Recovery/capacity/mobile | NOT RUN | Observed planned disruptions, capacity, retained URL and focus/input |
| Install/update/doctor/platform/integration | Candidate publication and provider local doctor pass; launch smoke FAIL | Disposable profile smoke preparation, full clean-install/update/platform/integration proof |

Installed Agent Browser: source 9c499716, generation 0.28.0-d0c5bd7be0cd,
SHA256 d0c5bd7be0cd62c189adca40d5a9d6ba8d6420ae0015feab25165fd372cb3a3a.
Publication confirms production and default development unchanged. Candidate
Cargo Signal job job-20261003T153354Z-966a2a8f3f47 passed in 212 seconds.
First job stopped before compilation because MCP environment lacked user bus;
explicit XDG runtime and DBUS address corrected that admission failure.

Installed Remote View: source 7a1dfe05d48e3ab227323b56fdd62b35e78ca2a0,
SHA256 f7869224d96e56464153ec00e06d4ee334b35826c5b5c7ed0a5e4125acd0c685.
Supported install and exact monitor adoption complete; package/local/native
control/Guacamole/gateway doctor checks pass. Authenticated viewer remains unknown.
Control exec handle 20721, gateway 90032, verifier 90894. Verify process identity
before any restart. Desktop, audio and Guacamole resources are intentional.
Task dashboard backend 3071904, frontend 3071905; public origin configured.

State/evidence root: /home/ecochran76/.local/share/agent-browser-dev-p221.
Wrapper: /home/ecochran76/.local/bin/agent-browser-dev-p221.
Provider public origin: https://remote-view-dev-p221.ecochran.dyndns.org.
Dashboard public origin: https://agent-browser-dev-p221.ecochran.dyndns.org.
Share only returned authenticated /remote-view/<handoff-id> links when ready.
Preserve private credentials, profiles, pending keys, ledgers and retained resources.

Actual launcher job job-20261003T154305Z-142a89f8273e failed after open at inventory:
agent-browser browser inventory did not identify one exact opened browser.
Direct service browsers read returned zero although managed SQLite held the exact
browser, proving the legacy-only collection gap. The task browser may have expired
through normal 300-second idle cleanup; take a fresh census before resumption.

Source inventory repair in this checkpoint reads existing managed SQLite through
read-only open, projects exact IDs/PIDs/canonical cdpEndpoint and preserves legacy
records. Metadata does not claim fresh health or effect authority. New source is
not installed. Focused service_inventory 20 passed; formatting and strict workspace
Clippy passed. Cargo Signal jobs job-20261003T155536Z-4d5a1bb1dedc and
job-20261003T155634Z-267a9e861015 retain logs and receipts privately.

AuraCall custody: auracall, fix/p222-agent-browser-cdp-endpoint. Unrelated .tmp/
remains untouched. Expanded existing regression covers canonical cdpEndpoint and
legacy cdpHost/cdpPort. Canonical case fails before fix, retained job
job-20261003T155634Z-be788b1ab289. No production client runtime replaced.
Next exact client repair: normalize canonical cdpEndpoint into host/port while
retaining the existing compatibility shape, then run focused tests/typecheck.
Publish the qualified browser inventory candidate and compiled task client, inspect
fresh task ownership, and rerun the actual fixture without an injected runner.

Required development launch smoke job job-20261003T154044Z-2b2d697d8aeb failed
before browser launch with browser_profile_not_found for its disposable profile.
Retain this installation gate; do not substitute its failure for actual-client
profile readiness. The stopped smoke host and orphan tokens were preserved after
positive empty-state and executable-identity checks. No broad process cleanup ran.

Cargo Signal is callable and used for builds/tests. Raw logs stay under the task
evidence/cargo-signal tree; inspect only bounded diagnostics and relevant excerpts.
Do not broaden implementation beyond the active first-client acceptance failure.

## Historical anchor compatibility

<a id="p222-continuation--2026-10-03--presentation-path-qualification"></a>[P222 continuation | 2026-10-03 | Presentation path qualification](RUNBOOK-history-2026-10-03-through-p222-focus.md#p222-continuation--2026-10-03--presentation-path-qualification)
<a id="turn-439--2026-10-02--p222-execution-resumed"></a>[Turn 439 | 2026-10-02 | P222 execution resumed](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-439--2026-10-02--p222-execution-resumed)
<a id="turn-438--2026-10-02--product-delivery-successor"></a>[Turn 438 | 2026-10-02 | Product delivery successor](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-438--2026-10-02--product-delivery-successor)
<a id="turn-437--2026-10-02--installed-browser-selection-and-shared-session-workflow"></a>[Turn 437 | 2026-10-02 | Installed browser selection and shared-session workflow](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-437--2026-10-02--installed-browser-selection-and-shared-session-workflow)
<a id="turn-436--2026-10-02--requested-pre-500k-stop-checkpoint"></a>[Turn 436 | 2026-10-02 | Requested pre-500k stop checkpoint](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-436--2026-10-02--requested-pre-500k-stop-checkpoint)
<a id="turn-435--2026-10-02--installed-view-attempt-exposes-browser-id-collision"></a>[Turn 435 | 2026-10-02 | Installed view attempt exposes browser ID collision](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-435--2026-10-02--installed-view-attempt-exposes-browser-id-collision)
<a id="turn-434--2026-10-02--installed-ordinary-reads-work-viewing-setup-ready"></a>[Turn 434 | 2026-10-02 | Installed ordinary reads work; viewing setup ready](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-434--2026-10-02--installed-ordinary-reads-work-viewing-setup-ready)
<a id="turn-433--2026-10-02--ordinary-reuse-routing-repaired-task-gateway-setup"></a>[Turn 433 | 2026-10-02 | Ordinary reuse routing repaired; task gateway setup](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-433--2026-10-02--ordinary-reuse-routing-repaired-task-gateway-setup)
<a id="turn-432--2026-10-02--installed-navigation-works-client-presentation-incomplete"></a>[Turn 432 | 2026-10-02 | Installed navigation works; client presentation incomplete](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-432--2026-10-02--installed-navigation-works-client-presentation-incomplete)
<a id="turn-431--2026-10-02--actual-client-launches-chrome-navigation-join-repair"></a>[Turn 431 | 2026-10-02 | Actual client launches Chrome; navigation join repair](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-431--2026-10-02--actual-client-launches-chrome-navigation-join-repair)
<a id="turn-430--2026-10-02--desktop-automation-no-longer-depends-on-gateway"></a>[Turn 430 | 2026-10-02 | Desktop automation no longer depends on gateway](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-430--2026-10-02--desktop-automation-no-longer-depends-on-gateway)
<a id="turn-429--2026-10-02--installed-client-advances-to-desktop-observation"></a>[Turn 429 | 2026-10-02 | Installed client advances to desktop observation](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-429--2026-10-02--installed-client-advances-to-desktop-observation)
<a id="turn-428--2026-10-02--pending-acquisition-recovery-implemented"></a>[Turn 428 | 2026-10-02 | Pending acquisition recovery implemented](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-428--2026-10-02--pending-acquisition-recovery-implemented)
<a id="turn-427--2026-10-02--exact-provider-recovery-unblocks-acquisition"></a>[Turn 427 | 2026-10-02 | Exact provider recovery unblocks acquisition](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-427--2026-10-02--exact-provider-recovery-unblocks-acquisition)
<a id="turn-426--2026-10-02--real-client-exposes-placement-and-acquisition-gaps"></a>[Turn 426 | 2026-10-02 | Real client exposes placement and acquisition gaps](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-426--2026-10-02--real-client-exposes-placement-and-acquisition-gaps)
<a id="turn-425--2026-10-02--current-candidate-installed-for-client-workflow"></a>[Turn 425 | 2026-10-02 | Current candidate installed for client workflow](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-425--2026-10-02--current-candidate-installed-for-client-workflow)
<a id="turn-424--2026-10-02--product-delivery-correction"></a>[Turn 424 | 2026-10-02 | Product delivery correction](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-424--2026-10-02--product-delivery-correction)
<a id="turn-423--2026-10-01--simple-product-acceptance-successor"></a>[Turn 423 | 2026-10-01 | Simple product acceptance successor](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-423--2026-10-01--simple-product-acceptance-successor)
<a id="turn-422--2026-10-01--requested-resumption-checkpoint"></a>[Turn 422 | 2026-10-01 | Requested resumption checkpoint](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-422--2026-10-01--requested-resumption-checkpoint)
<a id="turn-421--2026-10-01"></a>[Turn 421 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-421--2026-10-01)
<a id="turn-420--2026-10-01"></a>[Turn 420 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-420--2026-10-01)
<a id="turn-419--2026-10-01"></a>[Turn 419 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-419--2026-10-01)
<a id="turn-418--2026-10-01"></a>[Turn 418 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-418--2026-10-01)
<a id="turn-417--2026-10-01"></a>[Turn 417 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-417--2026-10-01)
<a id="turn-416--2026-10-01"></a>[Turn 416 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-416--2026-10-01)
<a id="turn-415--2026-10-01"></a>[Turn 415 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-415--2026-10-01)
<a id="turn-414--2026-10-01"></a>[Turn 414 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-414--2026-10-01)
<a id="turn-413--2026-10-01"></a>[Turn 413 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-413--2026-10-01)
<a id="turn-412--2026-10-01"></a>[Turn 412 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-412--2026-10-01)
<a id="turn-411--2026-10-01"></a>[Turn 411 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-411--2026-10-01)
<a id="turn-410--2026-10-01"></a>[Turn 410 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-410--2026-10-01)
<a id="turn-409--2026-10-01"></a>[Turn 409 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-409--2026-10-01)
<a id="turn-408--2026-10-01"></a>[Turn 408 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-408--2026-10-01)
<a id="turn-407--2026-10-01"></a>[Turn 407 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-407--2026-10-01)
<a id="turn-406--2026-09-29"></a>[Turn 406 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-406--2026-09-29)
<a id="turn-405--2026-09-29"></a>[Turn 405 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-405--2026-09-29)
<a id="turn-404--2026-09-29"></a>[Turn 404 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-404--2026-09-29)
<a id="turn-403--2026-09-29"></a>[Turn 403 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-403--2026-09-29)
<a id="turn-402--2026-09-29"></a>[Turn 402 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-402--2026-09-29)
<a id="turn-401--2026-09-29"></a>[Turn 401 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-401--2026-09-29)
<a id="turn-400--2026-09-29"></a>[Turn 400 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-400--2026-09-29)
<a id="turn-399--2026-09-29"></a>[Turn 399 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-399--2026-09-29)
<a id="turn-398--2026-09-29"></a>[Turn 398 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-398--2026-09-29)
<a id="turn-397--2026-09-29"></a>[Turn 397 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-397--2026-09-29)
<a id="turn-396--2026-09-29"></a>[Turn 396 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-396--2026-09-29)
<a id="turn-395--2026-09-29"></a>[Turn 395 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-395--2026-09-29)
<a id="turn-394--2026-09-28"></a>[Turn 394 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-394--2026-09-28)
<a id="turn-393--2026-09-28"></a>[Turn 393 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-393--2026-09-28)
<a id="turn-392--2026-09-28"></a>[Turn 392 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-392--2026-09-28)
<a id="turn-391--2026-09-28"></a>[Turn 391 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-391--2026-09-28)
<a id="turn-390--2026-09-28"></a>[Turn 390 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-390--2026-09-28)
<a id="turn-389--2026-09-28"></a>[Turn 389 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-389--2026-09-28)
<a id="turn-388--2026-09-28"></a>[Turn 388 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-388--2026-09-28)
<a id="turn-387--2026-09-17"></a>[Turn 387 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-387--2026-09-17)
<a id="turn-386--2026-09-17"></a>[Turn 386 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-386--2026-09-17)
<a id="turn-385--2026-09-17"></a>[Turn 385 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-385--2026-09-17)
<a id="turn-384--2026-09-17"></a>[Turn 384 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-384--2026-09-17)
<a id="turn-383--2026-09-17"></a>[Turn 383 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-383--2026-09-17)
<a id="turn-382--2026-09-17"></a>[Turn 382 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-382--2026-09-17)
<a id="turn-381--2026-09-17"></a>[Turn 381 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-381--2026-09-17)
<a id="turn-380--2026-09-17"></a>[Turn 380 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-380--2026-09-17)
<a id="turn-380--2026-09-17-1"></a>[Turn 380 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-380--2026-09-17-1)
<a id="turn-379--2026-09-17"></a>[Turn 379 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-379--2026-09-17)
<a id="turn-378--2026-09-17"></a>[Turn 378 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-378--2026-09-17)
<a id="turn-377--2026-09-17"></a>[Turn 377 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-377--2026-09-17)
<a id="turn-376--2026-09-17"></a>[Turn 376 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-376--2026-09-17)
<a id="turn-375--2026-09-17"></a>[Turn 375 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-375--2026-09-17)
<a id="turn-374--2026-09-17"></a>[Turn 374 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-374--2026-09-17)
<a id="turn-373--2026-09-17"></a>[Turn 373 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-373--2026-09-17)
<a id="turn-376--2026-09-17-1"></a>[Turn 376 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-376--2026-09-17-1)
<a id="turn-375--2026-09-17-1"></a>[Turn 375 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-375--2026-09-17-1)
<a id="turn-373--2026-09-17-1"></a>[Turn 373 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-373--2026-09-17-1)
<a id="turn-374--2026-09-17-1"></a>[Turn 374 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-374--2026-09-17-1)
<a id="turn-373--2026-09-17-2"></a>[Turn 373 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-373--2026-09-17-2)
<a id="turn-372--2026-09-16"></a>[Turn 372 | 2026-09-16](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-372--2026-09-16)
<a id="turn-371--2026-09-16"></a>[Turn 371 | 2026-09-16](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-371--2026-09-16)
<a id="turn-370--2026-09-16"></a>[Turn 370 | 2026-09-16](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-370--2026-09-16)
<a id="turn-369--2026-09-16"></a>[Turn 369 | 2026-09-16](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-369--2026-09-16)
