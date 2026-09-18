# Plan 0211 | Simple Install, Upgrade, And Remote View

Date: 2026-09-17

Plan version: 10

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

Current `origin/main` defaults workstation installation to the preserve-mode
hot transaction. Its full-shutdown alternative still requires a dry-run plan,
a caller-supplied SHA-256 digest, supervisor-takeover census, exact selected
process identity, and the absence of active drains and transactions. Those
preconditions make the terminal operator action subordinate to stale
coordination state.

P211 was admitted from `origin/main@fb616aee`. Issues #181, #183, and #195 are
the primary outcome items. Issues #189 and #190 are required remote-view
regression subcases under #195, not additional implementation lanes. P207
remains the primary writer for overlapping CLI help,
README, agent skill, service docs, and generated service contracts until it
integrates; P211 will rebase before editing those shared documentation
surfaces. P205 completed as Plan 0216 and merged its service-model extraction
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
`shutdown_closes_browser_when_owner_authority_is_stale` passes. P207 remains
open in pull request #184, so P211 still excludes its help and documentation
surfaces.

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

The fresh-context startup readback on 2026-09-17 found the P211 worktree clean
and synchronized with `origin/platform/p211-simple-cold-upgrade@4b9edcca`.
Current `origin/main` is 17 commits ahead and P211 is 6 commits ahead of its
merge base. P205 remains active and modifies the installer, Service State,
presentation inventory, profile acquisition, and remote-view coordination.
P207 remains open in pull request #184 and retains help and documentation
custody. P211 therefore keeps this packet in new lane-owned modules and the
small command adapter until those dependencies integrate. Graphiti was healthy
but returned only older remote-view history. CodeGraph was unavailable because
this worktree has no local index, so current source and Git evidence remain the
authority.

## Frozen Interface Packet

The shutdown module exposes one operation that receives one effects adapter.
The controller owns the fixed phase order and short deadline for each phase;
the adapter receives only the selected phase and its deadline. The operation
accepts no transaction, drain, census, revision, rollback, hash, capability,
or repair-token input. Every phase returns a typed step receipt. Verification
returns independently attributable Agent Browser-owned residue plus separately
reported foreign-process observations. A failed phase does not suppress later
ownership release, transient cleanup, or final verification.

The cold-install module exposes one operation with the fixed sequence `stop`,
`replace`, `start`, and `readiness`. Stop must return a successful shutdown
receipt before replacement begins. Any replace, start, or readiness failure
executes one bounded rollback phase and reports both the original failure and
rollback integrity. A successful result requires all four phases and a ready
probe from the newly selected generation. Legacy hot-upgrade transaction state
is not an input to this interface.

The post-boot presentation module accepts stable configured route and display
descriptors, prior transient claims, the current boot epoch, and one observation
adapter. It returns admitted current-boot bindings and typed exclusions.
Prior-boot claims are historical evidence only. A route is admitted only when
the current observation proves its configured route, display, and ownership
relationship; quarantined, orphaned, or mismatched inventory is excluded.

Trusted single-user acquisition keeps the existing named-profile request seam.
A stable self-declared subject is sufficient in shared-local mode. One healthy
retained browser is reused; otherwise one browser is launched. Failure releases
tentative ownership so the profile and browser are immediately reusable.
Remote-view success requires both `operatorVisible.state=ready` and a resolving
opaque `/remote-view/<handoff-id>`. A normal same-site authentication redirect
retains that durable handoff and returns a typed authentication-required state.
Raw provider, route-binding, embed, dashboard, or health URLs are never the
operator result.

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
6. On startup, treat prior-boot process, lease, browser-owner, route, display,
   controller, and viewer claims as historical. Re-observe stable configured
   presentation resources and admit each independently healthy route for the
   current boot without reinstalling or editing primary Service State.
7. In trusted single-user mode, accept the caller's stable self-declared
   identity for a named profile. Reuse one healthy matching retained browser
   when present; otherwise launch or requalify one. Retained identity
   collisions are diagnostic observations, not denials: select the valid
   requested profile explicitly, preserve the conflicting evidence, and
   continue. A failed attempt must leave the profile and browser immediately
   reusable rather than retaining an unmatched owner.
8. Make ordinary route selection exclude orphaned, quarantined, or
   route/display-mismatched inventory. A normal same-site authentication
   redirect must preserve a usable login handoff or return a typed
   authentication-required handoff rather than closing the browser.
9. Return success only when `operatorVisible.state=ready` and the durable
   opaque handoff resolves. Doctor, service status, capacity, preflight, and
   checkout must agree on the same effective readiness result.
10. Update CLI help, README, agent skill, documentation site, and inline docs to
   present the one-command workflow and remove hot-upgrade ceremony from the
   ordinary path.

## Scope And Effect Boundary

Expected implementation writes are limited to the CLI command router and help,
a focused cold-shutdown/cold-install module, the smallest required adapters in
the workstation installer, native runtime, profile acquisition, presentation
inventory, route selection, and remote-view handoff paths, provider-free
fixtures, README, the Agent Browser skill, installation and remote-view
documentation, this plan, and P211's active-lane projection.

The command may affect only Agent Browser-owned user units, timers, browsers,
runtime hosts, dashboard processes, MCP processes, presentation processes and
containers, leases, ownership records, and transient runtime metadata. It must
not kill unrelated browsers or containers. Unknown foreign processes are
reported and left alone; stale Agent Browser bookkeeping does not block the
owned shutdown set.

This plan does not authorize a production install, production shutdown,
credential or provider use, release, broad process cleanup, deletion of profile
data, or mutation of unrelated worktrees. Installed validation remains a
separate user-directed effect after source qualification. The product contract
is intentionally single trusted user; this plan does not weaken identity or
ownership behavior for a future multi-user mode.

## Delivery Sequence And Budget

- Optimization target: balanced wall-clock and token efficiency.
- Active-agent concurrency: at most 3 total, including the primary. Delegation
  is one level deep; workers cannot spawn workers.
- Critical path: frozen lifecycle and trusted-single-user contracts, public
  shutdown, profile release, cold installer routing, restart requalification,
  ordinary remote-view open, installed acceptance, documentation.
- Slice 1: freeze the shutdown and cold-upgrade result contracts and add red
  provider-free fixtures for broken transaction state, active drain, stale
  metadata, partial retry, and a clean machine.
- Slice 2: implement `agent-browser shutdown` and the bounded owned-target
  adapters.
- Slice 3: route workstation and reviewed-candidate apply through
  stop-replace-start while leaving legacy hot transaction inspection intact.
- Slice 4: implement startup requalification, single-user named-profile
  acquisition, healthy-browser reuse, safe route selection, and durable
  remote-view handoff fixtures for #195, including #189 and #190.
- Slice 5: join the cold-install and remote-view paths in one provider-free
  fresh-install and reboot acceptance fixture.
- Slice 6: synchronize all required documentation and run changed-surface
  validation once against the consolidated candidate.
- Slice 7, separately authorized after source qualification: run one installed
  clean-state journey and one replacement-upgrade journey through a ready
  remote-view handoff.
- Maximum work-unit attempts: 3 per slice.
- Maximum review and rework cycles: 1.
- Maximum consecutive hardening checkpoints: 2.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 360 active minutes through a provider-free qualified
  candidate. Installed-runtime validation is excluded until separately
  directed.
- First outcome artifact: a provider-free fixture proving stale drain and
  failed transaction metadata cannot block shutdown.

## Worker Assignments

The P211 lane owner retains architecture, source custody, Git transitions,
shared-contract decisions, runtime effects, finding disposition, integration,
and final acceptance. The primary uses the strongest available tier for
consequential architecture and integration, currently `gpt-6-astra` at high
reasoning. Deterministic repository and test tools remain the first choice.

After the primary freezes the shutdown result, cold-install sequencing,
startup-requalification, trusted-single-user identity, and remote-view
readiness contracts, it may fan out two disjoint workers:

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

After P205 and P207 integrate, all workers pause while the primary refreshes
the worktree inventory, rebases, and reconciles shared Service State and
documentation surfaces. One freed slot may then run a documentation-parity
worker on `gpt-5.6-luna` at low reasoning, limited to `cli/src/output.rs`,
`README.md`, `skills/agent-browser/SKILL.md`, and the relevant installation and
remote-view MDX pages. It gets 25 active minutes and one correction pass.

After the candidate is frozen and deterministic validation finishes, one freed
slot runs a fresh read-only review on `gpt-5.6-sol` at high reasoning for at
most 20 active minutes. The packet contains the exact candidate and base,
acceptance table, changed files, validation receipts, effect exclusions, and
stable finding IDs. The reviewer may return no findings and cannot broaden
scope or claim acceptance. No worker receives an auxiliary worktree,
independent branch, commit authority, production effect, or nested delegation.

P205 remains primary writer for the service-model extraction and its migrated
Service State types. P207 remains primary writer for its current CLI help,
README, skill, service documentation, and generated-client changes. P211 owns
the new cold-shutdown module, workstation-install routing, post-boot
requalification adapter, trusted-single-user acquisition, remote-view joining
logic, and its tests; it will rebase after those checkpoints before touching
overlapping surfaces.

## Evidence And Exit

| Requirement | Acceptance evidence | Current state |
| --- | --- | --- |
| One-command shutdown | `agent-browser shutdown` fixture returns success from healthy, drained, failed-upgrade, and partial-prior-run inputs | public route and empty-workstation idempotence fixture green; populated, failed-upgrade, partial-prior-run, and installed acceptance pending |
| Bounded completion | injected-clock tests prove fixed phase deadlines and exact escalation without production-scale sleeps | fixed phase deadlines, bounded daemon/command waits, and provider-free escalation receipt mapping are green; injected-clock platform timeout fixture pending |
| Complete owned shutdown | receipt proves owned units, timers, browsers, runtime hosts, dashboard, MCP, and owned containers are stopped | exact browser/daemon, fixed user-unit, fixed container, state-release, metadata, and verification adapters implemented; installed residue proof pending |
| Profiles become unowned | fixture proves profile data remains while runtime owners and leases are released | authority kernels and repository fixture green; public process fixture releases the retained session and preserves the profile record and physical data; active protected-claim process coverage remains |
| Metadata cannot veto | the controller interface accepts no coordination inputs and the fixed-sequence test passes | controller and platform adapter green; installed stale-metadata acceptance pending |
| Cold replacement | workstation and reviewed-candidate apply execute stop, replace, start, and readiness in that order | not implemented |
| Clean restart | post-start fixture proves one selected generation, one runtime host, one dashboard, and clients can make a fresh service request | not implemented |
| Current-boot presentation | startup fixture invalidates prior-boot claims, re-observes configured routes and displays, and admits each healthy current-boot route without repair input | not implemented |
| Trusted single-user profile | a named profile accepts stable self-identification, reuses one healthy matching browser, and requires no hash, capability, sealed plan, or repair token; retained identity collisions choose the valid profile and remain observable | source selector and provider-free collision regressions green; joined launch and remote-view proof plus typed collision telemetry pending |
| Safe route selection | #189 regression proves quarantined, orphaned, and mismatched routes are repaired or excluded before preflight reports ready | not implemented |
| Login handoff | #190 regression proves a normal same-site authentication redirect leaves a usable durable handoff or typed authentication-required state | not implemented |
| Ready remote view | an ordinary route-free open returns `operatorVisible.state=ready` and an opaque `/remote-view/<handoff-id>`; doctor, status, capacity, preflight, and checkout agree | not implemented |
| Simple interface | default operator path requires no preflight digest, transaction ID, revision, census code, rollback choice, or manual recovery command | not implemented |
| Legacy containment | hot transaction mutation is not reachable from the default install or upgrade path | routing change pending |
| Documentation parity | CLI help, README, Agent Browser skill, docs site, and inline comments describe the same workflow | not implemented |

Exit requires all rows green against one frozen source candidate. Provider-free
tests must include idempotent replay, a shutdown interrupted after each phase,
stale PID metadata, exact foreign-process preservation, owned container
cleanup, browser close escalation, ownership release, and restart readiness.
It must also include changed-boot requalification, partial route recovery,
healthy retained-browser reuse, unmatched-owner rollback prevention,
quarantined-route exclusion, a protected URL redirecting to login, and
runtime-host survival during reattach.

The final installed acceptance is one ordinary user journey against one frozen
candidate: clean install; bounded start and passing doctor; named-profile open
using self-identification; ready durable remote view; one-command shutdown with
profiles unowned; replacement install; bounded restart; and a second ready
remote view from the same named profile. It fails if the operator must choose a
route, desktop, or display; generate or copy a hash, capability, token, code,
or sealed plan; edit Service State; run a repair command; or interpret raw
Guacamole state.

## Stop Condition

Stop before production installation or shutdown, profile-data deletion,
unscoped process or container termination, live provider or credential use,
release, or mutation of another lane's checkout. Stop and reconcile if P205 or
P207 publishes an overlapping interface change before P211's corresponding
adapter or documentation work begins. Stop the affected worker after its stated
bound and return partial evidence; do not silently increase concurrency,
reasoning tier, retry count, or the cumulative plan budget.
