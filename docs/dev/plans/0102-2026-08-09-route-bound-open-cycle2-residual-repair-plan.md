# Plan 0102 | Route-Bound Open Cycle 2 Residual Repair

Date: 2026-08-09

State: ACCEPTED BOUNDED REPAIR OVERLAY

Lane: P102, corrective overlay for P101 only

Authority:

- `docs/dev/plans/0101-2026-08-09-route-bound-open-actions-deepening-plan.md`
- `docs/dev/notes/0101-2026-08-09-route-bound-open-actions-plan-audit.md`
- terminal findings `P0101-A1-02`, `P0101-A1-03`, and `P0101-A1-04`

## Purpose And Bound

Plan 0101 exhausted its two plan-audit cycles with three proven blockers. This
packet resolves only those blockers and the clerical P0099 dependency path. It
does not reopen architecture discovery, create a third audit cycle, authorize
source movement before the P0101 predecessor gate, or change P0101's final
monolith budgets.

This overlay supersedes only the conflicting timing, outcome, fallback, and
packet-target statements identified below. Every other P0101 requirement,
including the transaction ledger, sole `remote_view.rs` root, temporary runtime
adapter, 615-definition inventory, 81 atomic checkpoints, predecessor hard
stop, architecture gates, safety invariants, and validation matrix remains in
force.

The correct P0099 dependency is:
`docs/dev/plans/0099-2026-08-09-workspace-view-projection-deepening-plan.md`.

## Repair R01 | Total Deadline And Bounded Compensation

Finding: `P0101-A1-02`.

`jobTimeoutMs` remains the total daemon-side command deadline. It does not
become a forward-effect deadline and it gains no unbounded post-deadline join.
The current outer-caller rule remains unchanged: the caller deadline must be
longer than `jobTimeoutMs` only by its existing transport margin.

For a positive total budget `T` milliseconds, route-bound execution reserves:

```text
compensation_reserve_ms = min(15_000, max(250, floor(T / 5)))
forward_deadline = command_started_at + max(0, T - compensation_reserve_ms)
total_deadline = command_started_at + T
```

If `T` is no larger than the reserve, the coordinator performs no mutation and
returns the existing timeout envelope by the total deadline. Before every
forward effect, it checks the forward deadline. Timeout or cancellation stops
new forward work and begins compensation immediately.

Every compensation method receives the remaining total-deadline budget and
must be independently bounded. The supervisor never starts a cleanup effect
that cannot finish within that remaining budget. The last bounded repository
step records one of two terminal states:

- `rolled_back`: all owned effects were compensated and the exact prior
  persisted state was restored;
- `rollback_incomplete`: no cleanup future remains, active checkout is removed,
  the affected profile or acquisition identity remains fail-closed and
  quarantined against duplicate acquisition, and exact unconfirmed external
  effects are recorded for explicit recovery.

At `total_deadline`, the coordinator has returned one of those terminal states.
No task, blocking command, browser close, or rollback future continues in the
background. The worker emits the existing timeout or cancellation envelope and
releases the serialized queue. Response and queue release are therefore bounded
by `jobTimeoutMs` plus ordinary scheduler and serialization jitter, not by an
additional fifteen seconds.

Failed-transaction compensation may still close only the tab or browser
created by this transaction when the remaining budget permits confirmation.
If confirmation cannot fit, the resource is quarantined and preserved rather
than closed speculatively. Reused and established resources always survive.
Plan 0097's process-exit-only cleanup rule for an established reachable
`BrowserManager` remains unchanged.

Slice A must add fake-clock and scripted-adapter tests for:

- success within the forward budget;
- cancellation at every mutating phase;
- forward timeout beginning compensation before the reserve;
- a cleanup effect that consumes the remaining budget;
- `rollback_incomplete` quarantine with no duplicate lane;
- response and queue release by the total deadline;
- no spawned task or effect invocation after the returned outcome.

Inline comments must state that the compensation reserve is internal to the
existing total deadline. No CLI help, README, skill, or docs-site contract
change is required because the public deadline meaning is preserved. If
implementation cannot meet the total bound without changing that meaning, the
route-bound Slice A packet stops and requires explicit public contract review.

## Repair R02 | Complete Typed Outcome And Authorization Boundary

Finding: `P0101-A1-03`.

The frozen outcome enum is corrected to:

```text
RouteBoundOpenOutcome
  Planned { plan }
  NotFound
  ExplicitlyClosed
  Reopened { opened }
  Opened { opened }
  RolledBack { blocker, compensation }
  ProviderFallback { fallback }
```

`DirectOpen` with dry run returns `Planned` and performs no mutation. Durable
resolution may return `Planned` only when its derived request explicitly
requests dry run; it never reuses `Opened` as a planning side channel.

Caller authentication and transport authorization stay outside the
coordinator. Dashboard, HTTP, MCP, CLI, and direct daemon entry adapters each
must authorize the caller according to their existing contract before they can
construct `RouteBoundOpenInvocation`. The coordinator receives typed caller
and service-job attribution, not cookies, headers, sessions, or transport
identity.

The provider-fallback eligibility ledger therefore removes the
dashboard-specific caller-authentication predicate. It retains all of these
closed, transport-neutral predicates:

1. an immutable durable-handoff resolution snapshot exists;
2. the handoff was not explicitly closed unless authorized reopen is active;
3. the retained provider is exactly RDP and the opaque handoff remains the
   public identity;
4. reacquisition failed with typed
   `RequestedProfileInUseByPid` for the expected retained ownership lane;
5. the retained route snapshot is current for the invocation and contains a
   bounded provider-access target;
6. the bounded operator-access observation succeeds;
7. the result is typed `best_effort`, never normal managed control;
8. no new browser, profile lane, checkout, or lifecycle ownership is created;
9. the retained browser and unrelated tabs remain unchanged.

Ingress parity fixtures prove unauthorized requests never reach the
coordinator and every authorized ingress produces the same invocation facts,
dry-run `Planned` outcome, blocker mapping, and fallback decision. The
coordinator never infers authorization from provider URL presence or operator
access.

## Repair R03 | Atomic Packets Into Cohesive Deep Modules

Finding: `P0101-A1-04`.

The 81 packet and commit checkpoints remain atomic. Packet granularity is a
review and rollback boundary, not a requirement for one target module per
packet. The following target corrections are mandatory:

| Packet | Rejected shallow target | Adopted cohesive owner |
| --- | --- | --- |
| `P0101-E05-03` | `browser_page_content.rs` | `browser_inspection.rs`, shared with URL, title, content, and page inspection operations |
| `P0101-E05-05` | `browser_console.rs` | `browser_inspection.rs`, whose read-side interface owns console and page-error observation |
| `P0101-E11-03` | `browser_styles.rs` | `browser_inspection.rs`, whose typed element-inspection result owns computed style |
| `P0101-E11-05` | `browser_dialog.rs` | existing `interaction.rs`, whose operation interface owns page interaction state and dialog response |
| `P0101-E11-06` | `browser_upload.rs` | existing `interaction.rs`, with the existing allowed-path authority injected at its upload operation |
| `P0101-E16-01` | `network_response.rs` | existing `network.rs`, whose request identity and body retrieval interface owns response bodies |
| `P0101-E16-05` | `browser_video.rs` | existing `recording.rs`, whose lifecycle interface owns recording and video start, stop, and evidence |

The executor must apply the same deletion test to every remaining new target
before its packet begins. A new module is permitted only when its interface
hides a multi-step invariant used by at least two operations or production
call sites. Otherwise the packet deepens the nearest cohesive existing module.
The packet row, stable inventory IDs, expected delta, tests, rollback boundary,
and checkpoint remain distinct even when several packets accumulate into one
deep module.

The architecture checker gains a target-depth ledger with, for each new
module, its owned invariant, external interface operations, production callers,
and deletion-test statement. It fails when a target is a one-handler wrapper,
when actions dispatch coordinates its private steps, or when several new
targets split one cohesive invariant merely to satisfy line budgets.

## Execution Authorization And Receipt

Plan 0101 source execution remains blocked until P0098, P0099, and P0100 are
landed, validated, and recorded by P0101 Preflight P0. Once that gate passes,
the executor treats this overlay as the final disposition of
`P0101-A1-02`, `P0101-A1-03`, and `P0101-A1-04`.

There is no third plan audit. The independent implementation audit will verify
these repair criteria as part of its ordinary two-cycle work review. A failure
there is an implementation finding, not a reason to reopen plan discovery.

Effects of this packet: one planning artifact only. No source, runtime,
installation, browser, tenant, commit, push, release, or live-system effect is
authorized.
