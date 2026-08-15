# Plan 0116: Runtime Adoption And Transactional Upgrade

Date: 2026-08-15

State: OPEN

Lane: P116

Source baseline: `190338dde2e5efb997e1d92e5a7a3647189c9646`

Depends on:

- `docs/dev/plans/0035-2026-06-15-external-byop-browser-adoption-plan.md`
- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
- `docs/dev/plans/0070-2026-07-09-browser-session-authority-plan.md`
- `docs/dev/plans/0091-2026-08-03-systemd-interlock-self-quiesce-repair-plan.md`
- `docs/dev/plans/0096-2026-08-07-durable-remote-view-handoff-plan.md`
- `docs/dev/plans/0108-2026-08-10-runtime-process-identity-pid-reuse-repair-plan.md`
- `docs/dev/plans/0109-2026-08-11-runtime-dependability-handoff-remediation-plan.md`
- `docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md`

## Goal

Make fresh installation, executable replacement, workstation reconciliation,
daemon loss, and durable remote-view recovery converge through one
generation-aware runtime authority without corrupting or abandoning a live
browser.

An installation or upgrade is complete only when every discovered live
runtime has one proven disposition:

- transferred cooperatively to the candidate generation;
- adopted directly from verified surviving process and CDP evidence;
- preserved as a manual or externally owned runtime without claiming new
  automation authority;
- retired because it owns no live browser or protected state;
- or rejected as an explicit blocking ambiguity.

Copying files, restarting a dashboard, receiving a ready route record, or
passing a component doctor is not completion by itself.

## Maintainer Direction

Agent Browser should treat daemons as replaceable adapters around durable
logical browser identities. A daemon executable is not the browser identity,
and loss of one daemon must not make a live, verifiable browser unreachable.

When a retained browser PID still names the same process instance, its
DevTools endpoint is reachable, its profile identity agrees, and its display
and route evidence remain valid, a fresh Agent Browser generation should
reconstruct a safe attachment rather than launch another browser or require
the old daemon to remain healthy.

Fresh installation and upgrade must use that same adoption capability. They
must not overwrite the active runtime first and hope that later reconciliation
can stitch the pieces back together.

## Incident Evidence

### Installed upgrade failure

Plan 0091 recorded the existing unsafe ordering directly:

- workstation apply materialized a replacement payload before an interactive
  privilege gate stopped reconciliation;
- twenty active daemons remained bound to the prior executable;
- unit restoration recovered only service active states, not prior payload or
  browser ownership;
- one retained browser later required a separately authorized route recovery;
- the recurring interlock remained disabled because coordinated handoff of the
  remaining runtimes was not proven.

### Durable handoff failures on 2026-08-15

Read-only and controlled live diagnosis observed:

- durable QuickBooks and Google Messages handoffs reported unavailable or
  timed out while their browser PIDs and DevTools endpoints remained live;
- one handoff initially rendered a CDP stream despite requesting RDP, then the
  unchanged URL later converged to the correct Guacamole stream;
- a fresh verifier daemon lost its browser attachment while the browser and
  profile lock survived;
- the recurring workstation interlock intentionally stopped the dashboard,
  causing public login and service-proxy failures during reconciliation;
- `agent-browser install doctor --json` could report success while the public
  dashboard was inactive and the interlock was still reconciling.

These are not independent route, profile, dashboard, or binary defects. They
are consequences of several partial state authorities advancing without one
transaction or completion barrier.

## Confirmed Source Gaps

### Payload commit precedes runtime reconciliation

`run_workstation_install()` quiesces selected user units and calls
`materialize_payload()` before host preparation and runtime reconciliation.
`materialize_payload()` commits support assets, the stable installed binary,
systemd units, and workstation state without first inventorying or
transferring live daemon and browser ownership.

The payload commit is not atomic as a set. Existing destinations are removed
before staged files or directories are renamed into place. A failure after
some destinations change can leave a mixed generation.

### Failure restoration is service-state-only

`restore_previously_active_user_units()` restores the prior active or inactive
state of selected units. It does not restore:

- the prior binary;
- prior support assets;
- prior unit contents;
- daemon process ownership;
- browser CDP attachments;
- display and route bindings;
- or durable handoff presentation state.

Restarting a previously active unit after payload replacement may therefore
start the candidate generation while surviving session daemons still execute
deleted or older inodes.

### Cooperative handoff is not adoption

`runtime_handoff_prepare` requires the old daemon's process-local
`BrowserManager`, a live CDP connection, and a successfully written handoff
descriptor. `runtime_handoff_resume` requires that prepared descriptor.

If the old daemon is dead, too old, missing metadata, or no longer owns its
process-local manager, the new daemon does not reconstruct attachment from
the surviving browser's process identity, profile, CDP endpoint, target set,
display, and service-state evidence.

### The development publisher is incomplete authority

`scripts/publish-local-dashboard-runtime.js` improves the cooperative case by
preparing socket-discovered sessions before binary replacement. It still:

- discovers sessions from socket filenames rather than a closed-world runtime
  census;
- cannot adopt an orphan browser when the old daemon cannot prepare;
- makes the old daemon relinquish before the replacement proves ownership;
- stops the public dashboard during transfer;
- and cannot fully roll back after one or more handoffs have started.

### Durable handoff resolution replays effects

The durable handoff resolver rebuilds `remote_view_open`, may reacquire routes,
switch targets, navigate back to the originally expected URL, and wait for
target readiness. It is not a read-mostly resolution of one logical browser
and presentation capability.

The dashboard also treats the requested stream provider as a preference. If
the requested RDP stream is not present in the current projection, automatic
selection can fall back to CDP and render an apparently successful but wrong
view.

### Readiness has no end-to-end generation

Route, display, browser, proof, dashboard, daemon, authenticated ingress, and
Guacamole readiness are individually observable, but no receipt proves that
they describe one current presentation generation. The dashboard begins
rendering as soon as the resolver reports ready, before service-state and
viewport projection necessarily contain the requested stream.

## Relationship To Existing Plans

### Plan 0111

P111 owns canonical profile identity, one root browser owner, shared agent
participation, and resource-scoped mutation authority. P116 consumes those
concepts and owns transfer of a browser owner across executable generations.

P116 must not create a competing profile-owner registry. If P111 lands first,
P116 uses its `ProfileIdentity`, `ProfileOwner`, and owner generation directly.
If P116 begins first, its interim adoption records must be deliberately
compatible with those frozen concepts and migrate without changing identity.

### Plan 0096

P96 established opaque authenticated durable handoff URLs. P116 preserves that
public identity but changes resolution from effectful replay toward logical
browser and required-presentation reconciliation.

### Plan 0108

P108 established process-instance evidence that rejects PID reuse. P116 must
use that shared evidence rather than adding another PID liveness rule.

### Plans 0091 and 0109

P91 and P109 document executable drift, interlock, named-daemon supervision,
and installed-canary boundaries. P116 supersedes their maintenance-window
assumption only after the transactional upgrade and adoption gates pass. Until
then, their fail-closed installed-runtime guidance remains in force.

## Scope

- immutable generation staging and atomic candidate selection;
- closed-world runtime discovery before install mutation;
- verified direct adoption of surviving Agent Browser-owned browsers;
- two-phase cooperative daemon transfer without an ownership gap;
- generation-bound browser, profile, tab, display, route, and presentation
  receipts;
- continuous public dashboard availability during reconciliation and upgrade;
- read-mostly durable handoff resolution with hard requested-provider
  semantics;
- full rollback before old-generation retirement;
- installation, reconciliation, doctor, dashboard, HTTP, MCP, CLI, and
  generated-client contract alignment where new public surfaces are required;
- deterministic source fixtures plus a separately authorized disposable live
  upgrade gate.

## Non-Goals

- adopting an arbitrary process merely because it exposes a DevTools port;
- taking ownership of a browser whose process, profile, or endpoint evidence
  is ambiguous;
- copying a live writable profile to simulate a successful migration;
- terminating an externally owned or ambiguous browser;
- navigating, authenticating, or changing private site state during adoption;
- weakening P111's one-root-per-writable-profile invariant;
- changing formal release version or publishing a formal release in the first
  implementation slices;
- using QuickBooks, Google Messages, Facebook, or another retained private
  browser as the first live canary;
- claiming zero-downtime browser effects while ownership is deliberately
  frozen for compare-and-swap;
- preserving obsolete executable generations indefinitely after acceptance
  and rollback windows close.

## Frozen Terminology

### Runtime generation

`RuntimeGeneration` identifies one immutable binary and support-asset set. It
contains:

- generation ID;
- package version;
- binary SHA-256;
- support-manifest SHA-256;
- controller and schema compatibility versions;
- immutable installation path;
- creation and acceptance timestamps;
- state: `staged`, `validating`, `candidate`, `current`, `rollback`,
  `retired`, or `failed`.

One stable selector points at the current accepted generation. Running
processes always execute from an immutable generation path, never from a file
that an upgrade will unlink underneath them.

### Logical browser identity

`LogicalBrowserIdentity` names the durable browser owner independently of its
daemon or executable generation. It binds:

- browser ID;
- canonical profile identity digest;
- browser family and build identity;
- process-instance identity when live;
- owner generation;
- current daemon attachment, if any;
- target-set generation;
- display and route posture when applicable.

### Runtime observation

`RuntimeObservation` is read-only evidence from one source such as service
state, runtime state, supervisor state, socket metadata, process inspection,
profile lock, CDP, display allocation, or route provider. An observation may
support a decision but does not independently grant ownership.

### Browser adoption receipt

`BrowserAdoptionReceipt` is the authorization proof for attaching a new daemon
generation to a surviving browser. It records:

- logical browser and canonical profile digests;
- browser PID plus recorded process-instance evidence;
- executable family and observed executable identity;
- CDP endpoint identity and `Browser.getVersion` digest;
- target-set digest and selected-target identity;
- runtime-profile and service-state agreement;
- display, geometry, route, and stream agreement when present;
- previous owner generation and candidate owner generation;
- adoption mode: `cooperative_transfer`, `orphan_adoption`, or
  `manual_preservation`;
- decision, typed reasons, and timestamp;
- retention and redaction posture.

No endpoint credentials, raw profile paths, private target URLs, page content,
or provider secrets appear in durable public projections.

### Upgrade transaction

`UpgradeTransaction` is the durable host-level record for one candidate
installation. It contains:

- transaction ID and request attribution;
- old and candidate runtime generations;
- exact candidate artifact hashes;
- runtime-census digest;
- per-runtime migration records;
- current transaction state and monotonic revision;
- commit and rollback checkpoints;
- dashboard and presentation validation summaries;
- terminal result and typed stop reason.

### Presentation receipt

`PresentationReceipt` proves one authenticated operator view against one
current generation. It binds:

- dashboard deployment generation;
- selected coordinator and daemon generation;
- logical browser and process identity;
- selected target generation without exposing its private URL;
- required view-stream provider;
- display allocation and geometry epoch;
- route and Guacamole connection generation;
- authenticated ingress probe time;
- iframe or external-view load result;
- state: `ready`, `converging`, `blocked`, or `wrong_provider`.

## Product Invariants

1. No install or upgrade mutates the current selected generation before a
   complete runtime census and candidate preflight succeed.
2. One logical browser has at most one effect-capable daemon owner generation.
3. A candidate daemon may attach observation-only before ownership transfer,
   but it cannot issue browser effects until compare-and-swap commits.
4. Failure before owner commit leaves the old daemon authoritative.
5. Failure after owner commit produces a receipt-bearing rollback or a typed
   operator-required state. It never silently allows both generations to act.
6. A missing or incompatible old daemon does not prevent direct adoption when
   independent process, profile, CDP, and service evidence proves the same
   logical browser.
7. An open DevTools port alone never authorizes adoption.
8. A process identity mismatch, browser-family mismatch, profile mismatch,
   ambiguous owner, or unexpected target endpoint blocks adoption.
9. Fresh install performs the same census and adoption decisions as upgrade.
10. The default upgrade never terminates a live browser merely to release its
    profile lock.
11. A controlled shutdown fallback applies only to a proven Agent
    Browser-owned browser and must close it normally, verify exact process
    exit, and verify lock release before relaunch.
12. Dashboard ingress remains available during routine reconciliation and
    upgrade. Mutation admission may be temporarily frozen and visibly
    reported.
13. `install workstation reconcile` repairs the selected generation. It does
    not replace payload or silently change runtime generation.
14. A durable handoff requiring RDP waits for or reports RDP. It never renders
    CDP as a silent substitute.
15. Handoff resolution does not navigate or relaunch a retained browser unless
    the operator requested a separately named reopen or recovery action.
16. Installation success requires a generation-bound operator-journey receipt,
    not only component doctors.
17. Old generations remain available until the acceptance and rollback window
    closes.
18. Cleanup deletes only generations and metadata proven unreferenced by live
    processes, rollback state, supervisors, and retained transactions.

## Target Architecture

### 1. Immutable generation store

Use a layout equivalent to:

```text
~/.local/lib/agent-browser/generations/<generation-id>/bin/agent-browser
~/.local/lib/agent-browser/generations/<generation-id>/support/
~/.local/lib/agent-browser/current -> generations/<accepted-generation-id>
~/.local/bin/agent-browser -> ../lib/agent-browser/current/bin/agent-browser
```

The exact selector mechanism may be a symlink or an equally atomic launcher
manifest. The invariant is that staged and running generation files are
immutable and candidate selection changes through one atomic rename.

Systemd unit templates must not embed a support path that can be removed while
a live generation references it. Generation cleanup consults live process and
transaction references first.

### 2. Host runtime registry

Introduce one host-level `RuntimeRegistry` as the authoritative join across
logical browsers, profile owners, daemon attachments, generation ownership,
displays, routes, and presentation state.

The registry uses a process-safe repository and monotonic revisions. Individual
daemons publish observations and request compare-and-swap transitions. They do
not infer global authority from their own process-local `BrowserManager`.

The registry does not replace detailed service state. It owns the narrow
cross-generation authority that service state currently cannot prove.

### 3. Closed-world runtime census

Before candidate activation, inspect all supported sources:

```text
service browser records
  + runtime-profile state
  + profile-owner reservations
  + named session supervisors
  + daemon socket, port, PID, token, and executable metadata
  + operating-system process identity
  + profile lock and DevToolsActivePort evidence
  + bounded CDP browser and target observations
  + display allocations and visible-window proof
  + view streams, route pool, Guacamole, and handoff records
```

The census is stable only if a second observation confirms that relevant
process identities and registry revisions did not change while classification
was computed.

Each runtime is classified as:

- `cooperative_live_owner`;
- `orphan_adoptable`;
- `manual_preserve_only`;
- `idle_daemon`;
- `stale_metadata`;
- `external_observed`;
- `conflicting_owner`;
- or `insufficient_evidence`.

The last two states block default activation.

### 4. Admission drain rather than service outage

The coordinator announces `upgrade_pending`, stops admission of new effectful
jobs for affected logical browsers, and allows bounded in-flight work to
finish. Reads and dashboard status remain available.

After the drain deadline, unresolved effects produce a typed block. Upgrade
does not kill a daemon in the middle of an unreceipted effect merely to meet a
timer.

### 5. Two-phase cooperative transfer

For a responsive old daemon:

```text
old owner remains effect-capable
  -> candidate daemon attaches observation-only
  -> candidate proves browser, profile, targets, display, and route
  -> both bind the same proposed owner generation and transfer nonce
  -> registry compare-and-swap commits the new owner generation
  -> candidate becomes effect-capable
  -> old daemon acknowledges commit, drains, and exits
```

The current prepare behavior that clears `state.browser` before candidate
proof must not remain the final protocol.

### 6. Orphan adoption

When no cooperative daemon exists, the coordinator may authorize direct
adoption only when the evidence bundle proves one current logical browser.

The candidate daemon connects to the exact verified CDP endpoint, enumerates
targets, compares the target-set digest, restores event subscriptions, and
publishes a candidate attachment. The registry then performs the same owner
generation compare-and-swap used by cooperative transfer.

Adoption never launches Chrome and never removes a profile lock. The browser
continues with the same PID and process group.

### 7. Manual preservation and controlled shutdown

A matching browser without an adoptable automation endpoint remains
`manual_preserve_only`. Its process, profile lock, display, and operator route
remain protected, but the new generation does not claim CDP control.

If policy explicitly selects controlled restart for a proven service-owned
browser:

1. drain work and capture a restart plan;
2. request a normal browser close through the current owner;
3. verify the exact process identity exited;
4. verify the canonical profile lock disappeared;
5. activate the candidate generation;
6. relaunch the exact browser family and profile;
7. restore only approved target and presentation intent;
8. issue a restart receipt.

Signal-based termination is reserved for separately reviewed emergency
cleanup and is not the upgrade default.

### 8. Candidate deployment and commit

Stage the candidate generation and run source-free preflight before draining
runtime work. Start candidate coordinator, daemon, and dashboard backends on
generation-specific sockets or ports.

Only after all required runtime migrations and candidate presentation probes
succeed does the coordinator atomically change the current-generation
selector and unit generation metadata.

The old generation remains intact as rollback authority until post-commit
validation succeeds.

### 9. Continuous dashboard ingress

The public dashboard listener must not be the generation-specific process that
the upgrade stops. Introduce a stable ingress boundary, such as a small local
proxy or socket-activated front service, that forwards to one validated
generation backend.

Upgrade behavior becomes:

```text
old dashboard backend serves traffic
  -> candidate backend starts on a shadow endpoint
  -> authenticated smoke and runtime-manifest checks pass
  -> ingress backend selection changes atomically
  -> old backend drains
```

During browser mutation drain, the dashboard presents typed upgrade progress
and retry timing. It does not return an unexplained 502.

### 10. Read-mostly durable handoff reconciliation

Resolve `/remote-view/<handoff-id>` to:

- the durable logical browser;
- a retained or compatible target identity;
- the required presentation provider;
- and the current generation's presentation receipt.

If the browser is live but its daemon attachment is absent, request adoption.
If its RDP route is stale, request bounded route reconciliation. Neither action
navigates the retained target.

Explicit target reopen, URL navigation, browser relaunch, provider downgrade,
and profile replacement remain separately named operations with their own
authority and receipts.

### 11. End-to-end readiness

`operatorVisible.state=ready` remains useful route and display evidence, but
P116 adds a higher presentation gate. A durable handoff is ready only when one
`PresentationReceipt` proves the authenticated front door, selected dashboard
backend, service coordinator, daemon owner generation, browser, requested
stream provider, route, and rendered operator surface agree.

The dashboard waits for this receipt or shows `converging`. It must not render
an automatic alternate provider while waiting.

### 12. Doctor and status semantics

Separate these axes:

- `payloadReady`;
- `selectedGenerationReady`;
- `runtimeConvergenceReady`;
- `upgradeTransactionState`;
- `dashboardIngressReady`;
- `operatorJourneyReady`;
- `rollbackReady`.

Doctor returns nonzero or explicitly degraded when the public dashboard is
inactive, the selected generation disagrees with live daemon owners, an
upgrade is stranded, rollback material is missing, or a required operator
journey cannot complete.

An intentionally active, healthy upgrade may report `converging` rather than
generic failure, but it cannot report overall success before commit and final
validation.

## Upgrade Transaction State Machine

```text
planned
  -> candidate_staged
  -> candidate_preflight_ready
  -> census_stable
  -> admission_draining
  -> runtimes_transferring
  -> presentations_rebinding
  -> candidate_ready
  -> generation_committed
  -> post_commit_validating
  -> accepted
  -> old_generation_retirable
```

Typed terminal or recoverable states include:

- `blocked_ambiguous_runtime`;
- `blocked_inflight_effect`;
- `blocked_candidate_incompatible`;
- `rollback_before_commit`;
- `rollback_after_commit`;
- `operator_recovery_required`;
- `failed_preserved_old_generation`;
- `failed_effect_uncertain`.

Every transition uses the transaction revision and expected registry
generation. Replay of the same transaction ID returns the current durable
state rather than repeating transfers or browser effects.

## Failure And Rollback Contract

### Before generation commit

- candidate daemons release observation-only attachments;
- old daemons retain effect authority;
- old dashboard backend remains selected;
- current-generation selector remains unchanged;
- candidate files remain staged or become eligible for cleanup;
- no browser is relaunched.

### After one browser-owner commit but before deployment commit

- migrate that owner back through a receipt-bearing reverse compare-and-swap,
  or keep it on the candidate generation while the old deployment remains
  selected and record the mixed but authoritative recovery state;
- never authorize both owners;
- preserve both immutable generations;
- require explicit recovery if reverse transfer cannot be proved.

### After deployment commit

- shift stable ingress and generation selector back to the old accepted
  generation;
- reverse-transfer committed browser owners;
- validate old presentation receipts;
- retain the failed candidate and transaction evidence until adjudicated.

Rollback success means the old operator journey works, not merely that a
symlink changed back.

## Public And Operator Surfaces

The first slices should keep upgrade internals private while contracts settle.
Before installed execution, expose a coherent public surface:

- CLI preview and apply use the same transaction engine;
- a read-only upgrade status command reports transaction and runtime
  classifications without private paths or endpoint credentials;
- dashboard shows selected and candidate generations, migration progress,
  blockers, rollback readiness, and affected logical browsers;
- service status projects only safe generation and adoption summaries;
- HTTP, MCP, and generated client expose read-only status only if software
  orchestration is required;
- user-facing help, README, repository skill, docs site, inline comments,
  ROADMAP, and RUNBOOK describe the same semantics.

No public caller supplies a raw PID, CDP URL, profile path, display, route,
owner generation, or adoption decision. Those are service-resolved evidence.

## Implementation Slices

### Slice A | Red Fixtures And Authority Ledger

- Freeze the runtime census source ledger and classification matrix.
- Add fixture models for cooperative owner, orphan adoptable browser, manual
  browser, idle daemon, stale metadata, PID reuse, wrong profile, wrong browser
  family, wrong endpoint, conflicting owners, and insufficient evidence.
- Add a red upgrade fixture proving payload commit currently precedes runtime
  preservation.
- Add a red orphan fixture proving a fresh daemon cannot currently adopt a
  verified surviving browser without a prepared handoff descriptor.
- Freeze `RuntimeGeneration`, `UpgradeTransaction`, `BrowserAdoptionReceipt`,
  and `PresentationReceipt` schemas before production effects.

Terminal condition: deterministic fixtures express the complete authority
decision and the current unsafe seams fail for the expected reasons.

### Slice B | Immutable Generation Staging

- Introduce immutable generation paths and an atomic current selector.
- Stage binary, support, manifests, and unit templates under one generation.
- Make preflight failure leave the selected generation byte-identical.
- Retain old-generation assets through rollback acceptance.
- Split payload installation from runtime reconciliation.
- Make `install workstation reconcile` incapable of payload replacement.

Terminal condition: injected failure at every staging and selector boundary
leaves one complete selected generation and no mixed payload.

### Slice C | Runtime Census And Adoption Decision

- Implement the closed-world observation adapters.
- Use P108 process-instance evidence and P111 profile identity.
- Reconcile duplicated and missing observations without granting authority.
- Add bounded CDP identity and target-set probes.
- Persist census digest and per-runtime classification in the transaction.

Terminal condition: every supported live runtime is classified exactly once,
and ambiguity blocks before admission drain or payload commit.

### Slice D | Two-Phase Transfer And Orphan Adoption

- Replace relinquish-first handoff with candidate observation attachment and
  registry compare-and-swap.
- Implement verified orphan adoption through the same owner-generation seam.
- Preserve manual and external browsers without effect authority.
- Add idempotent replay and reverse-transfer rollback.
- Integrate active session supervisors without relying on socket enumeration
  alone.

Terminal condition: cooperative and orphan fixtures preserve the same browser
PID, profile, targets, and logical identity with exactly one effect-capable
owner at every boundary.

### Slice E | Continuous Dashboard And Presentation Commit

- Add the stable dashboard ingress boundary.
- Start and validate a shadow candidate dashboard backend.
- Add `PresentationReceipt` production derivation.
- Keep authenticated status available during mutation drain.
- Swap the backend only after candidate operator-journey proof.
- Make doctor observe ingress and operator-journey state.

Terminal condition: injected dashboard/backend failure never produces a front
door outage or selects an unvalidated backend.

### Slice F | Durable Handoff Self-Healing

- Resolve durable identity without replaying navigation.
- Request adoption when the logical browser survives without a daemon.
- Reconcile only the requested presentation capability.
- Make requested provider selection fail closed.
- Wait for one matching presentation generation before rendering.
- Keep explicit reopen, navigate, relaunch, and provider-change actions
  separate.

Terminal condition: the same opaque handoff resolves after daemon loss,
executable replacement, route replacement, and dashboard generation change
without launching a duplicate browser or navigating the retained target.

### Slice G | Transactional Installer Integration

- Route fresh install and upgrade through the same transaction engine.
- Add admission drain, candidate activation, final commit, rollback, and
  generation garbage collection.
- Reconcile privilege and dependency gates before payload selection whenever
  possible.
- When a precondition requires operator action, retain the old selected
  generation and all live ownership unchanged.
- Replace the development publisher's private lifecycle algorithm with the
  canonical engine or reduce it to a thin builder and client.

Terminal condition: every installer exit state has one durable transaction,
one selected generation, and a proven disposition for every discovered
runtime.

### Slice H | Public Parity And Documentation

- Add status schemas and generated types only after the native state model is
  stable.
- Align CLI, HTTP, MCP, dashboard, service status, doctor, help, README,
  repository skill, docs site, inline comments, ROADMAP, and RUNBOOK.
- Add architectural guards against direct stable-binary overwrite,
  relinquish-first transfer, hard provider fallback, and navigation inside
  ordinary handoff resolution.

Terminal condition: every ingress reports one transaction and readiness model
without exposing private runtime evidence.

### Slice I | Controlled Installed Acceptance

This slice requires separate explicit live authorization.

- Use two disposable named profiles and multiple tabs on both RDP routes.
- Preserve browser PIDs, process identities, profile identities, targets,
  displays, routes, and handoff URLs across one candidate upgrade.
- Kill one old daemon while preserving its browser and prove orphan adoption.
- Inject one candidate failure and prove full old-generation rollback.
- Open the same handoff from two fresh authenticated clients.
- Observe at least one recurring reconciliation interval without dashboard
  unavailability.
- Run final doctor and require all readiness axes to agree.

Only after the disposable gate passes may a separately authorized retained
private browser be used as a preservation canary.

## Required Deterministic Tests

1. Fresh install discovers a live workspace-started Agent Browser daemon and
   browser before any payload mutation.
2. Upgrade discovers runtimes represented only in service state, runtime
   state, supervisor state, socket metadata, process evidence, or profile-lock
   evidence and reconciles the union without double counting.
3. A live unrelated process reusing a recorded PID cannot be adopted.
4. A reachable DevTools endpoint owned by the wrong browser process or profile
   cannot be adopted.
5. A matching orphan browser with valid process, profile, CDP, and target
   evidence is adopted without launch or navigation.
6. A manual browser without CDP is preserved and never advertised as
   automation-ready.
7. An old daemon lacking the cooperative handoff protocol can still yield to
   verified orphan adoption after its effect authority is revoked.
8. Two-phase transfer failure before compare-and-swap leaves the old owner
   effect-capable.
9. Transfer failure after compare-and-swap produces a receipt-bearing reverse
   transfer or explicit recovery state with no double owner.
10. Concurrent client replay of one upgrade transaction emits no duplicate
    transfer, launch, shutdown, selector commit, or cleanup effect.
11. Failure at every payload staging boundary leaves the selected generation
    complete and byte-identical.
12. Failure at generation-selector commit rolls back to one complete old
    generation.
13. Failure after selector commit restores the authenticated old-generation
    operator journey.
14. Active profile locks are never removed during adoption or ordinary
    upgrade.
15. Controlled shutdown waits for exact process exit and lock disappearance
    before relaunch.
16. Dashboard ingress remains responsive while old and candidate backends
    start, validate, swap, drain, fail, and roll back.
17. Doctor cannot report overall success while dashboard ingress is down, a
    runtime owner executes the wrong generation, or an upgrade transaction is
    stranded.
18. Required RDP handoff never falls back to CDP.
19. Ordinary durable handoff resolution performs no navigation, browser
    launch, profile replacement, target creation, or provider downgrade.
20. Resolver waits for a matching `PresentationReceipt` rather than treating
    backend route readiness alone as rendered readiness.
21. The same durable handoff survives daemon loss, route replacement,
    dashboard generation swap, and new authenticated client attachment.
22. Old-generation garbage collection refuses any generation referenced by a
    live process, supervisor, rollback record, or unclosed transaction.
23. Persisted transaction, adoption, and presentation projections contain no
    raw profile paths, CDP credentials, target URLs, page content, cookies,
    tokens, or provider secrets.
24. P111 shared-browser acquisition continues routing many agents and tabs
    through one canonical profile owner after generation transfer.

## Validation Strategy

### Source gates

- focused native generation, census, adoption, transfer, presentation, and
  installer tests;
- workstation install fixture with injected failures at every transaction
  boundary;
- runtime handoff and process-identity regressions;
- shared-profile owner and duplicate-pressure regressions;
- durable handoff, route, display, and attachability regressions;
- dashboard projection, authenticated handoff, and ingress-swap tests;
- service contract, API/MCP parity, generated client, and no-launch gates when
  public contracts change;
- Rust formatting and strict Clippy through `scripts/ci/cargo-safe.sh` on WSL;
- dashboard and docs production builds;
- `pnpm validation:select -- --base <last-green-commit>` plus every selected
  gate;
- diff, architectural guard, and secret/path redaction checks.

### Installed gate

The installed gate is separate from source acceptance and requires:

- immutable old and candidate generation provenance;
- a stable runtime census before mutation;
- two disposable live browsers with multiple tabs;
- cooperative transfer and forced orphan adoption;
- one injected rollback;
- continuous local and public dashboard availability;
- same-PID and same-profile proof where adoption applies;
- same durable handoff URLs before and after;
- exact RDP provider rendering from two fresh clients;
- zero duplicate profile-owner roots;
- final transaction state `accepted`;
- final doctor success across all readiness axes;
- and retained rollback material until the reviewed cleanup step.

Source acceptance does not authorize this installed gate automatically.

## Architecture Guards

Add structural checks that fail when:

- workstation apply directly replaces the selected generation before a stable
  census and transaction record;
- a running generation's binary or support directory is removed;
- production upgrade enumerates only socket-discovered sessions;
- cooperative transfer clears the old owner before candidate proof and owner
  compare-and-swap;
- orphan adoption bypasses process-instance or canonical-profile validation;
- ordinary handoff resolution invokes navigation or browser launch;
- a required provider silently falls back to another provider;
- dashboard reconciliation stops the stable public ingress;
- doctor success ignores active upgrade, runtime-generation, or operator
  journey failure;
- cleanup deletes a generation that still has a live or rollback reference.

## Hard Stops

- Stop before source implementation if P111 and P116 define incompatible
  profile-owner identities or generations.
- Stop before payload mutation if the runtime census is incomplete or changes
  during classification.
- Stop before adoption if process, profile, browser family, CDP endpoint, or
  target identity is ambiguous.
- Stop before owner commit if candidate observation does not match the frozen
  adoption receipt.
- Stop before generation commit if any required runtime lacks a safe
  disposition.
- Stop before dashboard swap if authenticated candidate presentation proof is
  absent.
- Stop rather than terminate an external or ambiguous browser.
- Stop installed validation on the first unreceipted browser effect, duplicate
  profile root, wrong-provider render, dashboard outage, or failed rollback.
- Do not use a retained private browser to compensate for a disposable gate
  failure.

## Review Bounds

- One broad architecture drift review after Slice A freezes the authority
  ledger.
- One consolidated blocking-finding set.
- One bounded closed-world remediation pass against accepted blockers.
- A failed second verification causes the affected slice to split or remain
  blocked rather than reopening broad design indefinitely.
- Live installed acceptance uses one candidate upgrade, one injected rollback,
  and one post-commit observation interval unless a new critical safety defect
  requires stopping.

## Acceptance Criteria

Plan 0116 is complete only when current evidence proves all of the following:

- fresh install and upgrade share one durable transaction engine;
- installed files are immutable by generation and current selection is atomic;
- all supported runtime sources participate in a stable pre-mutation census;
- cooperative transfer and orphan adoption preserve the logical browser with
  exactly one effect-capable owner;
- manual and external browsers are preserved without false ownership;
- every failure boundary has deterministic rollback or a typed preserved
  recovery state;
- dashboard ingress stays available throughout routine upgrade and
  reconciliation;
- durable handoffs recover the same logical browser and required RDP
  presentation without navigation, duplicate launch, or silent CDP fallback;
- presentation readiness is bound to one authenticated end-to-end generation;
- doctor distinguishes payload, selected generation, runtime convergence,
  dashboard ingress, operator journey, and rollback readiness;
- source, schemas, generated clients, dashboard, help, README, repository
  skill, docs site, ROADMAP, RUNBOOK, and inline documentation agree;
- the disposable installed gate passes cooperative transfer, orphan adoption,
  rollback, two-client handoff, and continuous dashboard availability;
- installed and source artifact identities are recorded separately;
- no retained private browser, profile, authentication state, route, or
  operator handoff is mutated without separate explicit authority;
- and old-generation cleanup occurs only after acceptance and rollback review.

The objective is not satisfied by a green source suite, a successful file
copy, a cooperative happy-path handoff, a route-ready receipt, or a dashboard
that eventually recovers after operator-visible failure.

## First Execution Slice

Execute Slice A only.

The first change should be a provider-free, no-browser fixture package that
freezes the runtime census ledger and produces two intentional red results:

1. current workstation apply can commit candidate payload before preserving a
   live runtime;
2. a fresh daemon cannot adopt a fully verified surviving browser without the
   old daemon's prepared handoff descriptor.

Do not change installer behavior, replace the installed binary, enable the
recurring interlock, stop a live daemon, adopt a live browser, or run a private
handoff during Slice A.

## Execution Status

### 2026-08-15 | Slice A Source Implementation

Acceptance state: `IMPLEMENTED_NOT_ACCEPTED`

Implemented:

- one internal, provider-free runtime-adoption authority module with no
  installer, daemon, browser, dashboard, or public-contract integration;
- a closed ten-source census ledger covering service state, runtime state,
  profile ownership, supervisors, daemon metadata, process identity, profile
  lock and DevTools evidence, CDP browser and targets, display proof, and the
  view-stream, route-pool, Guacamole, and handoff join;
- thirteen deterministic fixtures covering all eight classifications plus PID
  reuse, wrong profile, wrong browser family, wrong endpoint, missing target
  evidence, conflicting owners, external observation, and an unstable census;
- frozen `RuntimeGeneration`, `UpgradeTransaction`,
  `BrowserAdoptionReceipt`, and `PresentationReceipt` serialization samples
  with private runtime evidence excluded from receipts and projections;
- two intentional red results for payload commit before runtime preservation
  and descriptor-bound orphan adoption, plus source-order guards tying those
  results to the current implementation.

Architecture drift review:

- completed the one broad Slice A review allowed by the plan;
- consolidated blocking-finding set: empty;
- P108 process-instance evidence and P111 profile identity digest and owner
  generation remain the compatible upstream vocabulary;
- the module creates no competing owner registry and does not alter existing
  `BrowserSessionAuthoritySnapshot`, `RuntimeState`, or
  `RuntimeHandoffDescriptor` behavior.

Validation:

- `runtime_adoption::tests`: 5 passed;
- JSON fixture parsing: passed;
- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`:
  passed;
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D
  warnings`: passed;
- broad Rust run: 2,025 passed, 57 ignored, and 6 failed;
- isolated serial reruns cleared three race-sensitive failures;
- the following three failures reproduce with the Slice A module registration
  removed, proving they are current baseline failures rather than Slice A
  regressions:
  - `test_confirm_executes_once_and_restores_confirmation_gate`;
  - `test_service_status_leaves_guacamole_root_without_route`;
  - `test_service_status_repairs_stale_guacamole_view_url`.

No live or installed validation was authorized or attempted. Slice A remains
not accepted until the three baseline failures are reconciled and the broad
source gate is green.

## Planning Evidence

- CodeGraph was healthy on 2026-08-15 with 553 files, 19,654 nodes, and 68,296
  edges.
- Structural tracing confirmed payload commit, unit restoration, cooperative
  handoff, publisher session discovery, dashboard backend selection, durable
  handoff replay, and provider fallback behavior against current source.
- Graphiti `agent_browser_main` returned source-linked prior direction that the
  service should own browser lifecycle and that service state must become
  authoritative before client surfaces imply stronger capability. Current
  source and plan artifacts remain authoritative over that advisory memory.
- This planning slice performs no browser, profile, daemon, route, display,
  dashboard, installed-payload, supervisor, or external-state mutation.
