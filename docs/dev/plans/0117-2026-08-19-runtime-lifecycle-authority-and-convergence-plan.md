# Plan 0117: Runtime Lifecycle Authority And Convergence

Date: 2026-08-19

State: IN PROGRESS — SLICE A SOURCE ACCEPTED

Lane: P117

Source baseline: `93503441a735accab2d2414c170f84ff4bab22b7`

Slice A receipt:
`docs/dev/notes/0117-1-2026-08-20-runtime-lifecycle-slice-a-source-acceptance.md`

Depends on:

- `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`
- `docs/dev/plans/0029-2026-06-07-live-retained-pressure-cleanup-plan.md`
- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
- `docs/dev/plans/0070-2026-07-09-browser-session-authority-plan.md`
- `docs/dev/plans/0108-2026-08-10-runtime-process-identity-pid-reuse-repair-plan.md`
- `docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md`
- `docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md`

## Goal

Make one dashboard process, one runtime host, and one current immutable runtime
generation the normal workstation state while preserving every valid retained
browser across upgrades. Put launch, transfer, adoption, close, reclamation,
profile retention, and generation retention under deep concrete owner modules
so stale resources cannot accumulate merely because different workflows hold
partial lifecycle facts.

The implementation must converge automatically after ordinary operation and
after successful upgrades. A bounded runtime convergence window may contain
one old and one candidate runtime host, but only one owner generation may
execute effects for a runtime lane. Outside that window, extra daemons,
executable generations, package-owned stale browser process trees, expired
ephemeral profiles, and unreferenced rollback payloads are drift that the
system must either reclaim safely or report as a typed blocking incident.

## Maintainer Decisions

The maintainer accepted the following decisions on 2026-08-19:

1. Steady state contains one dashboard process and one runtime-host daemon
   process. Both execute the same current immutable generation.
2. An upgrade may temporarily run one old and one candidate runtime host only
   inside a durable transaction with a deadline and deterministic rollback or
   convergence.
3. Unattended garbage collection may terminate a package-owned stale browser
   process group only when exact ownership, terminal lifecycle, expired lease,
   unreachable CDP, process identity, and grace-period evidence agree.
   Ambiguous resources remain protected and require reviewed cleanup.
4. Ephemeral profiles become automatically reclaimable after 24 hours in a
   terminal state with no process, lease, browser, transaction, handoff, or
   rollback reference. Failed or quarantined profiles are retained for seven
   days before reviewed or policy-approved reclamation. Named persistent
   profiles are never age-deleted automatically.
5. Healthy accepted upgrades automatically become retirable after a 24-hour
   rollback window. The current and immediately previous rollback generation
   remain protected; historical transaction metadata must not pin older
   binary generations.
6. The plan ends with controlled convergence of the current workstation only
   after the new authority, reconciliation, retention, and runtime-host paths
   pass source, fixture, disposable installed, and live dry-run gates.

## Incident Evidence

The 2026-08-19 workstation audit found:

- 226 Chrome processes across 14 process groups using about 13.1 GiB RSS;
- at least five Chrome groups using `.agent-browser` runtime profiles and
  about 6.5 GiB RSS;
- seven `agent-browser` processes using about 1.9 GiB RSS;
- nine Xvfb processes;
- 218 runtime-profile directories using about 58 GiB, including 204 older
  than seven days and 96 older than thirty days;
- 6.1 GiB of retired runtime profiles and 7.8 GiB under the package temporary
  root;
- 21 immutable runtime generations, all protected and none reclaimable;
- 49 upgrade transactions, including 26 `blocked_ambiguous_runtime`, 20
  `failed_preserved_old_generation`, two accepted, and one
  `blocked_inflight_effect`;
- a healthy dashboard user unit whose main process is
  `/home/ecochran76/.local/bin/agent-browser`, alongside multiple daemon
  processes from current and older generation paths; and
- no installed recurring resource-monitor timer.

`agent-browser service resources` observed roughly 15.8 GB across 268 relevant
processes, while `agent-browser service gc --dry-run` returned zero candidates.
The collector therefore exposed pressure without being able to reconcile it
to safe effects.

## Confirmed Source Gaps

### Runtime lifecycle authority is distributed

`RuntimeOwnerRegistry` deeply protects owner transfer, compare-and-swap,
candidate commit, reverse transfer, and generation fencing. It does not own the
entire lifecycle. `ChromeProcess` owns process-group shutdown, the adoption
transaction owns upgrade classification, `service_resources` independently
classifies stale processes, retained-state pruning removes Service State
records, and generation GC computes a separate protection graph.

Handoff deliberately sets `ChromeProcess.owns_process` false and clears its
temporary-profile cleanup. That correctly preserves a retained browser while a
candidate daemon attaches, but the cleanup obligation does not move through
the same durable owner transition. A browser can therefore survive transfer
without one authority remaining accountable for its eventual close or profile
reclamation.

### Live process observation disagrees with fixtures

Linux process collection splits `/proc/<pid>/cmdline` on NUL boundaries.
Current Chrome roots can rewrite their arguments into one flattened command
string plus a trailing NUL. `command_arg_value` expects discrete argument
tokens, so live profile and CDP evidence can disappear even though fixture
tests pass.

The collector scans the user process table and currently lacks an exact
package launch marker for every managed browser tree. Fixing only command-line
parsing would risk treating unrelated Chrome processes as package-owned.

### Garbage collection effect semantics are shallower than normal close

GC candidate identity records the process group ID, but apply calls
`terminate_pid` on only the recorded PID. Normal owned Chrome close tracks and
terminates the full process group. Inventory, review identity, and effect
execution therefore do not preserve one process-tree invariant.

The live GC smoke launches and reclaims Xvfb. It does not reproduce a managed
Chrome root, rewritten command line, helper processes, process-group cleanup,
profile locks, or package-ownership evidence.

### Retention rules cannot reach filesystem convergence

`service prune-retained --orphaned-profiles` removes Service State profile
records only when its current policy permits. It does not reclaim present
runtime-profile directories. Named and persistent profile protection is also
used as a resource-classification signal, so record cleanup and filesystem
cleanup can disagree.

Generation GC protects the selected generation, live process executables,
supervisor manifests, rollback evidence, and transaction references. Those
safety checks are correct, but every transaction not exactly
`old_generation_retirable` continues to reference old or candidate
generations. Historical blocked and failed transactions can therefore retain
all generations forever.

### The named session is still a daemon process key

`ensure_daemon(session, ...)` checks, versions, authenticates, starts, and
connects to one daemon per named session. Dashboard discovery and relay code
also resolve per-session socket and port files. Hot ownership transfer exists,
but normal routing still permits an unbounded set of session daemon processes.

CodeGraph impact analysis shows that changing the runtime-owner authority
touches 163 symbols across launch, recovery, daemon execution, handoff,
connection, dashboard relay, supervisors, installation, adoption, ingress, and
Service State persistence. The migration must therefore be staged behind
preserved behavior rather than attempted as one mechanical rewrite.

### Recurring route-user reconciliation can cross an interactive PAM boundary

The five-minute runtime interlock reconciles RDP route-user passwords through
the fixed root-owned helper. The formerly accepted helper used bare
`chpasswd`, which invokes the host PAM password stack and can raise a GNOME
Keyring unlock prompt during unattended maintenance. Chrome's
`basic_password_store` posture does not govern that Linux account-maintenance
path.

Before P117 re-enables automatic convergence, helper compatibility must require
an advertised non-PAM password-update contract and preserve compatibility with
the selected installed generation. A prompt-producing legacy helper is unsafe,
not merely byte-different, and must fail closed at the installer, doctor,
remote-view preflight, and scheduled reconciliation boundaries.

## Relationship To Existing Plans

### Plans 0026 and 0029

P26 established conservative inventory, dry-run review tokens, PID rechecks,
operator visibility, and an optional read-only timer. P29 proved retained-state
cleanup could reduce records without force-killing ambiguous processes. P117
keeps those safety properties but closes the missing ownership, process-tree,
profile-filesystem, and unattended-convergence gaps.

P117 does not reinterpret P26's zero-candidate result as a healthy workstation.
It treats a high-pressure zero-candidate result as unknown reconciliation that
must remain visible and block a healthy multiplicity claim.

### Plans 0069, 0070, and 0111

P117 preserves canonical profile identity, browser-session authority, owner
generations, and effect fencing. It must not create a second owner registry or
route commands around the existing owner generation. The new lifecycle owner
deepens that authority by carrying cleanup obligations and terminal states
through the same transitions.

### Plan 0108

Exact process start tokens, executable identity, browser-family evidence, and
PID-reuse rejection remain mandatory. P117 adds package launch identity and
process-group identity; it does not weaken the existing process assessment.

### Plan 0116

P116 proved immutable generation staging, closed-world census, cooperative
transfer, verified orphan adoption, continuous dashboard ingress, rollback,
and durable handoff preservation. P117 consumes those mechanisms and tightens
their terminal condition: accepted transactions converge automatically after
the rollback window, runtime lanes move into one host, and retained resources
must retain or discharge explicit cleanup obligations.

P117 is normal roadmap checkpoint work, not a formal release. The release gate
in `AGENTS.md` remains unchanged.

## Frozen Terminology

P117 adds the following domain language to `CONTEXT.md`:

- **Runtime host**: the singular active authority that executes daemon commands
  for all runtime lanes in one user installation.
- **Runtime lane**: a logical serialized command and ownership scope for one
  named browser session within the runtime host.
- **Cleanup obligation**: the durable duty to preserve a valid retained
  resource or reclaim it after its lifecycle becomes terminal.
- **Reclaimable runtime resource**: a package-owned resource with terminal
  lifecycle and no active ownership, lease, handoff, rollback, or process
  reference.
- **Runtime convergence window**: the bounded upgrade interval in which old
  and candidate runtime generations may coexist.

Use **retained browser** for a valid browser that survives daemon or route
transitions. Never label it orphaned merely because its original daemon exited.
Use **reclaimable runtime resource** only after package ownership and terminal
evidence are proven.

## Product Invariants

1. Every managed browser lane has one logical identity, one profile identity,
   one current owner generation, and one cleanup obligation.
2. A cleanup obligation is never silently discarded. It is transferred,
   satisfied, or quarantined with typed evidence.
3. Exactly one owner generation may execute effects for a runtime lane.
4. A retained browser may outlive any daemon process without losing its
   logical identity, profile identity, targets, route-bound handoff, or cleanup
   obligation.
5. Steady state contains exactly one dashboard process and one runtime host.
6. Every steady-state `agent-browser` process executes the selected immutable
   generation. Stable selectors and wrapper links do not count as separate
   generations.
7. Named sessions remain public logical identities but never require their own
   daemon process.
8. A runtime convergence window contains at most one old and one candidate
   runtime host. It has one transaction, one deadline, and one terminal result.
9. The dashboard remains reachable throughout runtime-host transfer and
   dashboard process replacement.
10. A candidate host observes and validates every runtime lane before any owner
    commit.
11. Ownership commit is compare-and-swap, idempotent, and reversible until the
    source finalizes or a typed recovery-only state is entered.
12. Package ownership is proven by durable launch identity plus exact process
    identity. Process name, executable basename, command shape, age, or profile
    path alone is insufficient.
13. Browser reclamation targets the reviewed process group and rechecks its
    exact identity immediately before every signal.
14. Unrelated Chrome, Chromium, Brave, Edge, Electron, Xvfb, and user-owned
    processes are never candidates merely because they resemble managed
    resources.
15. Reclamation must prove exact root exit, helper-tree exit, profile-lock
    release, and cleanup-obligation satisfaction.
16. An ambiguous resource remains protected and creates a typed incident. It
    cannot disappear from pressure summaries.
17. Ephemeral profiles become reclaimable only after 24 hours terminal and no
    active references.
18. Failed or quarantined profiles retain seven days of evidence before
    reviewed or policy-approved reclamation.
19. Named persistent profiles are never automatically deleted by age or quota.
20. Healthy accepted upgrades automatically advance to retirable after a
    24-hour rollback window and current readiness proof.
21. Only the current and immediately previous healthy rollback generation are
    retained by ordinary policy after convergence.
22. Historical transaction metadata remains durable without retaining binary
    generation directories after its rollback references close.
23. Resource pressure monitoring is installed and current before the runtime
    can claim healthy convergence.
24. Garbage collection is idempotent, bounded, observable, and safe to rerun
    after interruption.
25. Live cleanup occurs only after source, fixture, disposable installed, and
    live dry-run evidence identify the exact resources to be changed.

## Target Architecture

### 1. Deep runtime lifecycle authority

Add one concrete owner module under `cli/src/native/` that owns managed browser
lane lifecycle. Its narrow interface accepts lifecycle intentions such as
launch, attach, transfer, adopt, close, and reconcile. Its implementation owns
the rules that coordinate:

- logical browser and runtime-lane identity;
- canonical profile identity;
- current owner ID and generation;
- exact browser-root and process-group identity;
- daemon session-route compatibility metadata;
- retained-browser state;
- cleanup obligation and terminal evidence; and
- Service State projection.

The module wraps and then absorbs the authoritative behavior currently split
between `runtime_owner_transfer`, `ChromeProcess`, adoption workflows, service
resource classification, and retained-state cleanup. Command dispatch remains
shallow. It requests a lifecycle transition and renders the result; it does not
reconstruct ownership rules.

The existing owner registry remains the durable authority during migration.
Schema evolution extends it rather than creating a parallel registry. Service
State, handoff presentation state, and lifecycle authority continue to commit
under the current locked repository transaction.

### 2. Ownership-backed resource reconciliation

Add a deep resource reconciliation module with two real adapter families:

- a production OS adapter for process tables, process groups, start tokens,
  executable paths, command lines, RSS, profile locks, CDP reachability, and
  signaling; and
- a deterministic scripted adapter that reproduces live `/proc` bytes,
  rewritten Chrome command lines, PID reuse, partial reads, permission errors,
  and process-tree races.

The reconciler compares desired lifecycle records with observed resources and
emits one evidence model: protected, observed-ambiguous, reclaimable, reclaiming,
satisfied, or quarantined. CLI GC, automatic GC, install doctor, dashboard
pressure, and installed acceptance consume this model rather than implementing
their own classification.

Every package-launched browser root receives a durable, non-secret launch
identity tied to the lifecycle record. The design must work even when Chrome
rewrites argv, so launch identity cannot depend solely on later command-line
reconstruction. Platform-specific implementation may use inherited environment,
a private descriptor, a launcher-owned sidecar keyed by exact process identity,
or an equivalent reviewed mechanism. The selected mechanism must survive
daemon transfer without becoming forgeable by unrelated user processes.

Reclamation calls the same process-tree shutdown implementation as normal
owned close. Review tokens bind the exact root PID, start token, executable,
process group, lifecycle generation, profile identity, action, and observation
digest.

### 3. Deep retention authority

Add one retention module that calculates protection references and terminal
policy for:

- ephemeral runtime profiles;
- failed and quarantined profiles;
- retired profile roots and temporary artifacts;
- immutable runtime generations;
- upgrade transactions and rollback receipts;
- supervisor and live-process executable references; and
- cleanup evidence needed for review and audit.

Artifact-specific policy remains explicit, but callers cannot independently
decide that an artifact is safe to delete. The module returns a retention plan
with protected reasons, reclaimable reasons, age deadlines, projected bytes,
and exact paths rooted under validated package directories.

Accepted transactions keep rollback references for 24 hours after their
accepted timestamp and only while readiness remains healthy. At expiry, the
transaction automatically advances to the existing retirable terminal state.
Its compact metadata and receipts remain, while generation references are
closed. Failed and ambiguous transactions stay review-gated but must identify
which exact recovery reference prevents retirement; a generic nonterminal
state cannot protect every historical generation indefinitely.

Filesystem effects use a local-substitutable adapter. Apply rechecks every
reference and moves eligible directories into a package quarantine before
final deletion when atomic same-filesystem rename is available. Quarantine
manifests make interrupted cleanup and bounded restoration observable.

### 4. Single runtime host

Replace one-daemon-per-session routing with one user-scoped runtime host. The
host owns a map of runtime lanes and gives each lane serialized command
execution, owner-generation fencing, browser attachment, and cancellation.
Cross-lane work may proceed concurrently where existing state locks and browser
effects permit, but one slow lane cannot block unrelated lanes merely because
they share the host.

The dashboard remains a separate supervised process for failure isolation. In
steady state the dashboard and runtime host execute the same selected
generation. The dashboard relays named-session commands to the host with the
runtime-lane identity rather than discovering a per-session daemon port.

A transitional compatibility adapter accepts existing session socket and
metadata conventions and forwards them to logical lanes. It is temporary. The
final architecture gate deletes per-session daemon launch, PID, version,
executable-SHA, authentication, port, and supervisor authority after every
caller uses the runtime host.

### 5. Transaction-bounded hot convergence

Extend the P116 transaction so a candidate runtime host can attach to all
runtime lanes, validate their browser and target evidence, commit owner
generations, and take admission while the old host remains rollback-capable.
The transaction owns the runtime convergence deadline and the complete lane
set. It may not silently omit a lane discovered after the first census; a
changed set restarts observation or fails closed before commit.

After every lane commits and candidate readiness passes, the dashboard process
is replaced through the existing stable-ingress mechanism. Selection commits
only when the dashboard and runtime host both execute the candidate generation
and every durable handoff resolves to the same logical browser, target, route,
display, and requested provider.

The old host then finalizes, exits, and discharges its transferred cleanup
obligations. The convergence window closes only after exact process-exit and
multiplicity proof. Automatic rollback restores old-host admission and owner
generations before its deadline when reversal remains possible. Irreversible
legacy revocation keeps P116's typed operator-recovery state.

### 6. Continuous pressure control

Install a user-scoped read-and-reconcile timer as part of normal workstation
installation. Each run:

1. refreshes the complete resource and retention inventory;
2. records current pressure and multiplicity summaries;
3. applies only already-proven unattended reclamation classes;
4. leaves ambiguous or review-gated resources untouched;
5. emits compact lifecycle and retention receipts; and
6. alerts on stale monitor age, quota pressure, repeated reclamation failure,
   or multiplicity drift.

The timer does not become an independent authority. It calls the same lifecycle,
resource-reconciliation, and retention modules as operator commands.

## State Models

### Runtime lane lifecycle

```text
planned
  -> launching
  -> ready
  -> transferring
  -> ready
  -> closing
  -> terminal

launching | transferring | closing
  -> quarantined

ready
  -> retained
  -> transferring | closing

retained
  -> ready | transferring | closing | quarantined
```

Only `ready` and `retained` lanes may hold an effect-capable owner. A
`transferring` lane preserves the old effect-capable owner until candidate
commit. `terminal` means the browser process tree is gone, profile locks are
released, external route and display effects are terminal, and the cleanup
obligation is satisfied or separately retained by policy.

### Cleanup obligation

```text
owned
  -> transferring
  -> owned
  -> reclaimable
  -> reclaiming
  -> satisfied

owned | transferring | reclaimable | reclaiming
  -> quarantined
```

Transfer commit moves the obligation atomically with owner generation.
`reclaimable` requires the complete evidence predicate. `quarantined` preserves
the resource and exposes the missing or conflicting proof.

### Runtime multiplicity

```text
steady_current
  -> observing_candidate
  -> transferring_lanes
  -> candidate_current
  -> rollback_window
  -> steady_current

observing_candidate | transferring_lanes | candidate_current
  -> rolling_back
  -> steady_current | operator_recovery_required
```

`steady_current` proves one dashboard process, one runtime host, one selected
generation, and no legacy session daemon. The intermediate states require one
durable transaction and permit no more than two runtime hosts or two executable
generations.

## Implementation Slices

### Slice A | Red fixtures and lifecycle ledger

- Capture sanitized live process fixtures that preserve raw `/proc` command
  bytes, rewritten Chrome argv, process groups, helper trees, start tokens,
  executable generations, and profile roots.
- Add deterministic red tests proving the current zero-candidate defect, root
  PID versus process-group mismatch, dropped handoff cleanup obligation,
  profile-directory retention, transaction generation pinning, and
  per-session daemon multiplicity.
- Add a read-only multiplicity report covering dashboard processes, runtime
  hosts, legacy daemons, executable paths, generation IDs, and transaction
  windows.
- Extend the durable owner schema with cleanup-obligation and lifecycle fields
  behind backward-compatible defaults. Do not change effects yet.
- Add no-launch projection tests proving old state loads conservatively and
  cannot accidentally create reclaimable resources.

Terminal condition: all six current gaps have deterministic red proofs and the
new lifecycle schema round-trips without changing live behavior.

### Slice B | Deep runtime lifecycle authority

- Introduce the concrete runtime lifecycle owner module.
- Route launch registration, current-owner registration, cooperative transfer,
  verified orphan adoption, reverse transfer, finalize, owned close, and
  retained-browser preservation through it.
- Move cleanup obligations atomically with owner generations.
- Project lifecycle state into Service State and resource summaries without
  allowing those projections to become independent writers.
- Adapt `ChromeProcess` to execute lifecycle-approved close and relinquish
  decisions. Its drop implementation must no longer be the sole owner of
  profile cleanup.
- Keep a temporary facade only where a one-slice caller migration is required;
  enumerate and delete every facade before Slice G closes.

Terminal condition: every managed browser lane has one durable lifecycle
record and no launch, handoff, adoption, close, or recovery path can bypass its
owner-generation and cleanup-obligation checks.

### Slice C | Ownership-backed resource reconciliation

- Add production and scripted process-observation adapters.
- Parse raw Linux command data faithfully, including one-field rewritten
  Chrome command lines, but treat parsing as supplementary evidence.
- Add durable package launch identity and join it with P108 process identity,
  lifecycle generation, profile identity, and process group.
- Replace independent resource classification with the reconciler evidence
  model.
- Make normal close and GC use one reviewed process-tree shutdown
  implementation.
- Recheck root PID, start token, executable, process group, lifecycle
  generation, and profile identity before SIGTERM and again before SIGKILL.
- Replace the Xvfb-only live smoke with a disposable managed-Chrome smoke that
  proves helper-tree exit, profile-lock release, unrelated-Chrome protection,
  and candidate identity drift rejection.
- Preserve dry-run review tokens and explicit force semantics for manual
  review-gated cleanup.

Terminal condition: the captured live stale Chrome fixtures become candidates,
unrelated Chrome fixtures remain protected, and a disposable managed Chrome
tree is reclaimed completely through the same implementation as normal close.

### Slice D | Deep retention authority

- Add the retention module and filesystem adapter.
- Join Service State, lifecycle authority, process observation, handoff and
  route references, supervisor manifests, transaction state, rollback state,
  selected generation, and filesystem inventory.
- Implement the accepted 24-hour ephemeral, seven-day failed or quarantined,
  and never-automatic persistent-profile policies.
- Add automatic accepted-transaction finalization after a healthy 24-hour
  rollback window.
- Decouple durable transaction metadata from binary-generation protection after
  rollback references close.
- Retain only current and previous healthy rollback generations under ordinary
  policy.
- Add quarantine manifests, interrupted-apply recovery, restored-reference
  rejection, exact-root validation, projected bytes, and idempotent replay.
- Make retained-state pruning consume retention decisions rather than treating
  record removal as filesystem reclamation.

Terminal condition: fixture history matching the live 49-transaction and
21-generation shape converges to current plus previous generation without
losing durable transaction metadata, and eligible profile directories become
reviewable or automatically reclaimable under the accepted policy.

### Slice E | Single runtime host foundation

- Introduce one user-scoped runtime-host identity, socket, authentication
  record, process identity, executable generation, and supervisor manifest.
- Move named session serialization into logical runtime lanes within the host.
- Add bounded cross-lane scheduling so one blocked CDP command or remote-view
  operation does not starve unrelated lanes.
- Preserve per-lane owner fencing, cancellation, timeout, profile routing,
  browser attachment, tab state, stream delivery, and event attribution.
- Change CLI, HTTP, MCP, dashboard relay, stream discovery, remote-headed
  workflows, and supervisors to address runtime lanes through the host.
- Add a compatibility adapter for old per-session sockets and metadata. It may
  forward only; it cannot launch a new legacy daemon after host admission is
  enabled.
- Add stress fixtures for concurrent lanes, one stalled lane, host restart,
  cancellation, duplicate session names, and state-lock contention.

Terminal condition: a fixture with multiple named sessions uses one runtime
host process, preserves independent serialized lanes, and passes existing
command and browser-session behavior without launching per-session daemons.

### Slice F | Transaction-bounded hot host convergence

- Extend the P116 census and transaction with the complete runtime-lane set and
  runtime-host identities.
- Start one candidate host from the sealed candidate generation and attach it
  observation-only to every lane.
- Commit lanes through the lifecycle authority with rollback receipts and
  owner-generation fencing.
- Drain old-host admission, transfer queued work deterministically, and reject
  new legacy daemon creation.
- Replace the dashboard process through stable ingress only after candidate
  host and presentation readiness pass.
- Commit the current selector when dashboard and host executable evidence both
  match the candidate generation.
- Finalize and exit the old host, prove exact exit, and close the convergence
  window.
- Enforce one transaction deadline and automatic rollback or typed recovery
  for every interruption point.

Terminal condition: a disposable installed upgrade preserves multiple live
browser lanes and durable handoffs while observed multiplicity never exceeds
one dashboard, two transaction-bound hosts, and two generations, then returns
to one dashboard, one host, and one current generation.

### Slice G | Automatic reconciliation and compatibility deletion

- Install and enable the user-scoped pressure and convergence timer by default.
- Route scheduled and operator-triggered runs through the same reconciler and
  retention authority.
- Automatically reclaim only the accepted unattended classes.
- Add backoff and a typed incident after repeated effect failure.
- Make install doctor fail when the monitor is stale, multiplicity is outside
  its allowed state, generations drift, cleanup obligations are missing, or
  pressure is high but ownership remains unknown.
- Add dashboard summaries for steady-state multiplicity, convergence windows,
  protected versus reclaimable bytes, cleanup obligations, and blocking
  incidents.
- Delete per-session daemon launch and ownership code, obsolete metadata
  writers, permanent compatibility facades, and tests that assert process-per-
  session behavior.
- Replace shallow tests with tests at the lifecycle, reconciliation, retention,
  and runtime-host interfaces.

Terminal condition: ordinary use and completed upgrades converge without an
operator running cleanup commands, while ambiguous effects remain visible and
untouched.

### Slice H | Public parity and operator documentation

- Keep CLI, HTTP, MCP, generated client, dashboard, and Service State contracts
  aligned for multiplicity, lifecycle, reconciliation, retention, and incident
  readback.
- Update `cli/src/output.rs`, `README.md`, `skills/agent-browser/SKILL.md`, and
  `docs/src/app/` for every user-facing command, flag, behavior, environment
  variable, or policy.
- Document the two-process steady state, bounded convergence window, automatic
  GC predicates, profile retention classes, generation rollback window,
  review-gated cleanup, and recovery procedures.
- Update `ROADMAP.md`, the operator runbook, release diagnostics, and relevant
  contract schemas.
- Keep docs-site tables in HTML and CLI flags in kebab-case.

Terminal condition: all public ingresses expose one semantic model and an
operator can distinguish a retained browser, a reclaimable runtime resource,
an ambiguous protected resource, and an active convergence window without
inspecting `ps` or private files.

### Slice I | Controlled installed convergence

This slice requires explicit live authorization at execution time even though
the plan is authorized now.

1. Install the validated candidate generation through the transactional
   workstation path.
2. Prove the dashboard and runtime host are current while all retained browsers,
   targets, routes, displays, leases, and durable handoffs remain unchanged.
3. Run resource and retention inventory and record the exact protected,
   ambiguous, review-gated, and unattended candidates.
4. Refuse apply if unrelated user-owned resources appear in any candidate set.
5. Let policy reclaim proven unattended stale process trees and expired
   ephemeral profiles.
6. Review and apply the exact remaining eligible profile, transaction, and
   generation plan.
7. Preserve named persistent profiles and all ambiguous resources.
8. Re-run reconciliation, doctor, dashboard readiness, handoff resolution,
   profile locks, generation inventory, disk usage, process RSS, and
   multiplicity proof.
9. Record rollback and recovery instructions for every retained ambiguity.

Terminal condition: the workstation has one healthy dashboard process, one
healthy runtime host, one current executable generation plus at most one
healthy rollback generation on disk, no unreferenced package-owned browser
process tree, no overdue reclaimable ephemeral profile, a current monitor
summary, and zero unexplained readiness drift.

## Migration Strategy

1. Add authority and evidence before changing effects.
2. Route current paths through the lifecycle owner while preserving old public
   behavior.
3. Make reconciliation and retention correct under the current daemon topology.
4. Introduce the runtime host and forward legacy session routing into it.
5. Prove hot convergence with the compatibility adapter still present.
6. Enable default automatic reconciliation.
7. Delete legacy daemon creation and obsolete shallow tests.
8. Perform controlled live convergence last.

Every slice must be independently revertible before the next slice deletes its
old path. No migration step may rewrite private Service State by hand.

## Failure And Rollback Contract

### Before any lane owner commit

- terminate the candidate host process tree;
- remove only candidate-owned temporary artifacts;
- preserve old-host admission and every browser;
- leave the selected generation unchanged; and
- persist the failed observation receipt without adding generation retention
  beyond the normal evidence window.

### After some lane commits

- stop new candidate effects;
- reverse committed lanes with receipt-bearing owner generations;
- restore old-host admission;
- prove every lane's effect-capable owner and cleanup obligation; and
- quarantine any lane whose reversal cannot be proven before the deadline.

### After selector and dashboard commit

- prefer forward recovery while the candidate host and dashboard remain ready;
- otherwise restore the authenticated old dashboard and host generation;
- reverse ownership only while the old host is still valid rollback authority;
- never claim rollback when legacy daemon revocation or old-host finalization
  made restoration impossible; and
- preserve the typed operator-recovery state from P116.

### During reclamation

- stop the current candidate on any identity or reference mismatch;
- never widen from one reviewed process group or filesystem root to siblings;
- record partial effects and retry only idempotent remaining actions;
- restore quarantined directories only when no newer resource owns the target;
  and
- surface unreleased locks, surviving helpers, or failed deletion as blocking
  incidents rather than a successful cleanup.

## Required Deterministic Tests

1. Rewritten single-field Chrome argv retains supplementary profile and CDP
   evidence.
2. Exact launch identity proves a package browser when argv is unavailable.
3. Similar unrelated Chrome with the same profile-shape strings is protected.
4. PID reuse, process-group reuse, executable change, and lifecycle-generation
   change invalidate a review token.
5. GC terminates a disposable Chrome process tree, not only its root PID.
6. Normal close and GC produce the same process-exit and profile-lock proof.
7. Handoff transfers the cleanup obligation without killing the browser.
8. Reverse transfer restores owner and cleanup obligation with a newer
   generation.
9. Daemon loss preserves a retained browser and keeps one accountable cleanup
   obligation.
10. Ephemeral profiles younger than 24 hours remain protected.
11. Terminal unreferenced ephemeral profiles older than 24 hours are
    reclaimable.
12. Failed or quarantined profiles younger than seven days remain protected.
13. Named persistent profiles remain protected regardless of age or pressure.
14. A newly restored lease or handoff reference invalidates a retention plan.
15. Accepted transactions keep rollback for 24 hours and then become retirable
    only while readiness stays healthy.
16. Historical transaction metadata survives after binary references close.
17. Generation GC retains current and immediate rollback but reclaims older
    unreferenced generations.
18. Multiple named sessions execute through one runtime host process.
19. One stalled lane does not block unrelated lanes.
20. Per-lane command ordering remains serialized.
21. Candidate observation performs no browser, navigation, route, display, or
    profile mutation.
22. Partial host transfer reverses all committed lanes or enters typed recovery.
23. Dashboard ingress remains available throughout host and dashboard
    replacement.
24. Durable handoffs resolve the same logical browser, target, route, display,
    and provider before and after convergence.
25. Runtime multiplicity never exceeds the transaction-bound maximum and
    returns to steady state after success, rollback, timeout, and crash.
26. An old client using session metadata reaches a logical lane without
    launching a legacy daemon.
27. Automatic GC applies only unattended classes and leaves review-gated and
    ambiguous classes untouched.
28. Repeated cleanup failure creates one bounded incident without a retry
    storm.
29. Interrupted filesystem quarantine is resumable and does not escape the
    validated package root.
30. Install doctor rejects stale monitor state, unexplained daemon count,
    executable-generation drift, and missing cleanup obligations.
31. A legacy helper that lacks the non-PAM route-user password capability is
    rejected by every compatibility consumer even when its older desktop and
    display capabilities remain ready.
32. A compatible helper updates route-user passwords through the advertised
    SHA-512 crypt path without invoking PAM, remains accepted by the selected
    installed generation, and produces no password or keyring prompt during one
    bounded interlock reconciliation.

## Validation Strategy

### Per-slice source gates

Run the selector first against the slice baseline:

```bash
pnpm validation:select -- --base <last-green-ref>
```

Every Rust slice runs:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml <focused-filter>
```

Run the full Rust suite at each cohesive architecture checkpoint:

```bash
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml
```

Contract or dashboard slices also run the selected generated-client, parity,
dashboard inspector, browser-table, workspace-context, docs-build, and no-launch
service smokes required by `AGENTS.md`.

### Browser and process gates

- Extend the ignored E2E suite with a temporary profile and serial real-Chrome
  lifecycle test.
- Run real-Chrome tests with `--test-threads=1`.
- Add a disposable managed-Chrome GC smoke that verifies root and helper-tree
  exit, profile-lock release, profile policy, and unrelated-browser protection.
- Add a disposable multi-lane runtime-host smoke with at least three sessions,
  concurrent commands, one stalled lane, restart, and cancellation.
- Capture aggregate process counts and RSS before and after; do not persist raw
  private command lines or browser data.

### Upgrade and retention gates

- Run the source-free workstation failure matrix at every convergence
  transition.
- Run a disposable installed upgrade with multiple retained browsers, one
  injected rollback, stable authenticated dashboard ingress, and the same
  durable handoff URLs.
- Advance a fake clock through the 24-hour rollback and profile windows.
- Prove current plus previous generation retention and historical transaction
  metadata preservation.
- Prove an ambiguous transaction names its exact blocking reference instead of
  retaining all generations generically.

### Installed gate

Before live convergence:

- `agent-browser install doctor` must pass in the disposable installed target;
- resource and retention dry-runs must match the expected closed candidate set;
- the resource monitor must be installed, current, and read-only before apply;
- dashboard and host generation evidence must agree; and
- an independent closed-world review must find no candidate path capable of
  targeting unrelated Chrome or a named persistent profile.

After live convergence, bind evidence to the installed generation digest,
dashboard main PID and executable, runtime-host PID and executable, selected and
rollback generation IDs, transaction ID, resource summary timestamp, and
retention receipt.

## Documentation Contract

Any user-facing behavior introduced by this plan updates all required surfaces
in the same slice:

1. `cli/src/output.rs`
2. `README.md`
3. `skills/agent-browser/SKILL.md`
4. `docs/src/app/`
5. inline source documentation

Service request actions, schemas, HTTP, MCP, generated clients, JavaScript
types, and examples remain synchronized. Documentation must distinguish the
runtime host from a runtime lane, hot runtime convergence from a durable
remote-view handoff, and a retained browser from a reclaimable runtime
resource.

## Architecture Guards

Reject or stop implementation when:

- a second lifecycle or owner registry is introduced;
- a command path mutates lifecycle state without the concrete owner module;
- cleanup eligibility depends on process name, age, command line, or profile
  path without exact package ownership;
- GC signals a root PID while leaving its reviewed process group unmanaged;
- handoff drops a cleanup obligation;
- retained-state record deletion is reported as profile filesystem reclamation;
- a generic transaction state retains generations without an exact active
  recovery reference;
- a named session launches its own daemon after runtime-host admission begins;
- a transitional facade gains business logic or survives Slice G;
- dashboard and runtime host execute different generations outside a runtime
  convergence window;
- a convergence window lacks one durable transaction, deadline, complete lane
  set, or rollback result;
- automatic GC can target a named persistent profile or ambiguous process;
- tests mock the lifecycle owner instead of exercising its real interface;
- a compatibility test preserves obsolete daemon-per-session architecture
  instead of public named-session behavior; or
- live cleanup begins before the exact candidate set passes the disposable
  installed gate.

## Hard Stops

- Stop before process signaling if exact root, start token, executable,
  process-group, owner generation, launch identity, or profile identity changes.
- Stop before profile movement or deletion if any browser, process, lease,
  handoff, transaction, rollback, or lock reference remains.
- Stop before generation deletion if any live executable, selected link,
  supervisor, active convergence window, or rollback reference remains.
- Stop host commit if any runtime lane is missing, changing, ambiguous, or
  attached to the wrong browser or profile.
- Stop dashboard replacement if authenticated candidate ingress and durable
  handoff presentation are not ready.
- Stop automatic finalization if readiness regresses during the rollback
  window.
- Stop live convergence if unrelated resources enter the candidate set or the
  rollback path cannot preserve every retained browser.

## Review Bounds

- Use one broad fresh-context architecture review after Slices B through F are
  source complete.
- Adjudicate findings against this frozen goal, invariants, non-goals, and hard
  stops.
- Accept at most one bounded remediation pass for blocking findings before
  splitting or reframing the affected slice.
- Use closed-world verification for accepted blocking findings and critical
  regressions only.
- Do not reopen daemon topology, automatic cleanup, profile windows, generation
  windows, or live convergence scope without a maintainer decision and plan
  revision.

## Non-Goals

- Do not reduce the number of valid retained browser process trees to one.
  The multiplicity contract governs dashboard, daemon, and binary generations,
  not the number of intentionally retained browsers.
- Do not terminate or adopt unrelated user-owned browsers.
- Do not automatically delete named persistent profiles.
- Do not expose raw process command lines, private profile paths, auth state,
  cookies, or handoff provider URLs in public summaries.
- Do not replace durable opaque remote-view handoffs with daemon or provider
  addresses.
- Do not make every runtime lane globally serial; preserve safe cross-lane
  concurrency.
- Do not keep a permanent compatibility facade or dual daemon topology.
- Do not perform formal release work under this plan.
- Do not make live cleanup part of an ordinary source checkpoint.

## Acceptance Criteria

Plan 0117 is complete only when current source, disposable installed, and
authorized live evidence prove all of the following:

- one concrete lifecycle owner governs launch, transfer, adoption, close,
  retained-browser preservation, and cleanup obligations;
- every managed browser lane has one logical identity, profile identity, owner
  generation, exact process identity, and cleanup obligation;
- resource reconciliation identifies the captured stale managed Chrome groups
  while protecting unrelated browsers;
- GC and normal close share full process-tree and profile-lock semantics;
- automatic GC is enabled and applies only closed unattended classes;
- retention policy covers profiles, transactions, rollback records, and
  generations through one evidence model;
- healthy accepted transactions automatically become retirable after 24 hours;
- ordinary generation retention converges to current plus immediate rollback;
- historical transaction metadata no longer pins old generation directories;
- named sessions execute as runtime lanes in one runtime host;
- the dashboard remains a separate process and shares the selected generation
  with the host in steady state;
- hot upgrade preserves all retained browsers, targets, profiles, routes,
  displays, leases, and durable handoffs;
- runtime multiplicity stays within the bounded convergence window and returns
  to one dashboard, one host, and one current generation;
- per-session daemon launch and permanent compatibility code are deleted;
- install doctor and dashboard expose current multiplicity, pressure,
  retention, cleanup-obligation, and incident evidence;
- scheduled route-user reconciliation uses a generation-compatible non-PAM
  helper contract and cannot raise an interactive password or keyring prompt;
- the controlled workstation convergence removes only proven package-owned
  stale resources and preserves every named persistent or ambiguous resource;
- all required Rust, client, dashboard, docs, E2E, disposable installed, and
  live gates pass; and
- no formal release is claimed or performed.

## First Execution Slice

The compatibility-safe route-user PAM bypass is a bounded operational
prerequisite to P117, not a substitute for its lifecycle architecture. Install
and verify that helper before restarting the existing interlock timer. Its
source and live proof become input evidence for Slice A and the installed gate.

Begin with Slice A only. Do not signal processes, move profiles, finalize
transactions, delete generations, enable automatic GC, change daemon routing,
or touch the live workstation.

The first checkpoint should contain:

1. sanitized deterministic fixtures for the six confirmed gaps;
2. red tests bound to current source seams;
3. a read-only multiplicity report;
4. backward-compatible lifecycle and cleanup-obligation schema additions; and
5. focused no-launch validation proving old state remains protected.

Slice A exists to make the unsafe and missing seams executable before the deep
modules change behavior.
