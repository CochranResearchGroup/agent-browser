# Plan 0100 Cycle 1 Audit: Service Status Projection Authority

Date: 2026-08-09
Review mode: `drift_discovery`
Review cycle: 1 of at most 2
Reviewed artifact: `docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md`
Reviewer runtime: `/root/audit_plan_status_projection`
Source or runtime mutation: none

## Verdict

Implementation-ready: **No**.

The plan identifies the right architectural seam and is directionally strong:
Service State remains the persisted authority, one projector should own the
complete response, host-local evidence crosses a substitutable adapter, and the
dashboard gateway stops rewriting arbitrary JSON. The deletion target and the
legacy timeout and cache protections are also correctly identified.

Six unresolved choices are still material enough that different implementers
could produce incompatible authority, concurrency, locality, and wire-contract
behavior. One evidence item and one process item should also be closed in the
same bounded remediation pass. No source extraction should begin before the
blocking ledger below is resolved.

## Evidence Reviewed

- Plan 0100 in full, including its interface, deployment gate, slices, tests,
  risks, non-goals, and completion criteria.
- Current `service_status_projection`, control-plane status, dashboard proxy,
  dashboard JSON repair, display observation, Browser Session Authority, and
  workspace-consumer paths using CodeGraph first, then bounded direct reads for
  the intentionally oversized `actions.rs` and literal contract fields.
- The current service-status JSON Schema and generated client type.
- Plans 0099 and 0101 for cross-candidate ownership and ordering.
- Repo policies for worktree hygiene, validation and handoff, CodeGraph use,
  documentation control, and bounded independent review.

The worktree was already dirty only with the orchestrated plan and audit
artifacts. This audit added only this note.

## Finding Ledger

### P0100-A1-01 — `blocking` — Browser Session Authority crosses the proposed authority boundary

Criterion: persisted and reconciled Service State owns browser, inventory, and
actionability decisions; an observation may not alter inventory or enabled
actions; existing public behavior remains compatible.

Evidence:

- Plan 0100 says Service State owns browser, proof, inventory, and actionability
  and observations cannot change them
  (`docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md:160-173`,
  `:223-242`, `:307-316`).
- The target layout nevertheless puts Browser Session Authority inputs in
  `local_observation.rs` (`:295-298`) and then requires adapter output not to
  change dashboard actionability (`:436-441`).
- Today `browser_session_authority_snapshot` obtains process/resource evidence
  and can classify a modeled browser `non_viable`
  (`cli/src/native/browser_session_authority.rs:55-105`).
- The dashboard consumes that verdict as an authority input, changes `live` to
  false, disables the stream, and changes workspace state and actions
  (`packages/dashboard/src/lib/service-workspaces.ts:1088-1124`,
  `:1251-1268`).
- Manual runtime browsers also combine host registry observation with
  service-owned route and control fields
  (`cli/src/native/service_status_projection.rs:11-46`).

Consequence: an implementer must choose between preserving the current
process-backed viability gate and obeying the new rule that observation cannot
change inventory or actionability. The unavailable-adapter branch is especially
ambiguous: an empty required `manualBrowsers` array or absent process verdicts
can be mistaken for negative proof.

Reproducer: provide a modeled live browser plus resource evidence that marks its
process as a cleanup candidate. The current Browser Session Authority snapshot
returns `non_viable`, after which workspace projection treats the browser as not
live. Repeat with an unavailable observation source and there is no specified
equivalent outcome.

Confidence: high.

Suggested disposition: add a field-role ledger for every existing status
derivation and legacy field. Classify each as reconciled authority,
authority-derived response, host observation, compatibility mirror, or
transport envelope. Decide explicitly whether Browser Session Authority is a
preprojection authority input that may gate actionability or an observation that
may only annotate it. Freeze unavailable behavior for `manualBrowsers`, process
stats, Browser Session Authority, profile readiness, URL refresh,
attachability, and display content. Do not leave this decision to Slice 3.

### P0100-A1-02 — `blocking` — The public projector and observation adapter have no executable async contract

Criterion: the module has one small interface, blocking host inspection is
bounded, no async lock is held during blocking work, and callers have stable
error behavior.

Evidence:

- The proposed public shape is synchronous:
  `project(StatusAuthorityInput) -> ServiceStatusResponse`
  (`docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md:175-188`).
- The same plan requires the local adapter to run display inspection through a
  bounded blocking task and emit timeout and unavailable states (`:295-298`,
  `:425-434`).
- The current display helper is synchronous and holds its cache mutex while
  calling the blocking inspector
  (`cli/src/native/remote_view.rs:1314-1332`).
- Action and control-plane status paths currently have different failure
  envelopes for invalid state, which the plan says must remain intact
  (`docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md:397-414`).

Consequence: a synchronous projector cannot directly await `spawn_blocking`,
timeouts, or an asynchronous adapter without blocking a Tokio worker or hiding
a runtime bridge. Error, cancellation, concurrency, and partial-observation
behavior remain implementation choices.

Reproducer: attempt to implement `LocalStatusObservationAdapter` with the
proposed method signature while preserving the two-second display-command
bound. Either the method becomes async, the caller gathers observations before
projection, or it blocks inside the supposedly bounded interface. The plan does
not select one.

Confidence: high.

Suggested disposition: freeze the exact Rust seam, including asyncness,
ownership, lifetimes, cancellation, and error mapping. A viable shape is one
async projector call that asks an async adapter for a typed snapshot, with
`spawn_blocking` and timeout owned by the local adapter and pure serialization
afterward. State which errors fail the whole status read and which become typed
partial or unavailable observations. Preserve the current invalid-state
envelopes at the entry adapters.

### P0100-A1-03 — `blocking` — Cache coalescing and lock invariants contradict the current gateway mechanism

Criterion: concurrent status polls coalesce, successful responses remain cached
for five seconds, projection does not run under the gateway cache lock, and the
status timeout retains its exact established meaning.

Evidence:

- The current gateway intentionally holds `DashboardServiceStatusCache`'s
  async mutex across the complete backend request to coalesce polls
  (`cli/src/native/stream/dashboard.rs:935-951`).
- Projection runs in that backend request. Plan 0100 requires that no
  observation or projection run while the backend cache lock is held, while
  also retaining coalescing (`docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md:450-468`).
- The current ten-second value is passed separately to connect, write, and read
  phases (`cli/src/native/stream/dashboard.rs:851-905`), so the source does not
  establish a single ten-second end-to-end deadline even though the plan says a
  slow status call times out at ten seconds.
- Only successful 2xx bytes are cached today
  (`cli/src/native/stream/dashboard.rs:949-958`).

Consequence: retaining the existing lock implementation violates the new
invariant. Releasing it before the request loses coalescing unless a single-flight
mechanism is designed. Timeout and concurrent-failure behavior can also change
silently.

Reproducer: start two uncached requests for the same backend port and complete
path. In the current implementation the second waits on the mutex while the
first backend projection executes under that mutex. A three-phase slow backend
can also consume more than ten seconds because each phase receives the full
duration.

Confidence: high.

Suggested disposition: specify a single-flight cache state machine. The mutex
may guard only cache and in-flight registration, never the awaited request.
Freeze the key, waiter behavior, success-only five-second retention,
failure-sharing or retry semantics, cancellation cleanup, and maximum entry
count. Decide whether ten seconds is an end-to-end deadline or the existing
per-phase bound, then name tests that prove that exact choice without weakening
other gateway timeouts.

### P0100-A1-04 — `blocking` — Observation and compatibility wire contracts are not frozen deeply enough

Criterion: HTTP, MCP, CLI, generated clients, and dashboard consumers receive a
stable typed response with deterministic source, freshness, availability, and
legacy behavior.

Evidence:

- The proposed `statusProjection` shape is prose only. It does not specify
  required versus optional fields, nullability, key encoding, exact state and
  error enums, source-host identity, or timestamp rules
  (`docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md:202-221`).
- The plan says a compatibility projection "may" mirror observed URL and
  display data and later deletes the mirror once installed consumers have
  migrated (`:223-230`, `:503-526`). There is no request-time version
  negotiation or a rule saying whether the v1 response retains those legacy
  fields permanently.
- The current schema requires `manualBrowsers`, permits open Service State, and
  has no way for an old consumer to distinguish observation unavailable from no
  manual browser (`docs/dev/contracts/service-status-response.v1.schema.json:5-13`,
  `:55-101`).
- A five-second display observation cache followed by a five-second completed
  response cache can expose an observation nearly ten seconds after collection,
  but no freshness thresholds or recalculation rule are defined.

Consequence: independently correct Rust, schema, client, and dashboard changes
can disagree on whether missing means unavailable, how freshness expires, and
when legacy `frameUrl`, `externalUrl`, `displayContent`, `manualBrowsers`, and
process fields may disappear. Deleting a mirror based only on the installed
dashboard risks breaking other v1 clients.

Reproducer: collect a display result immediately before its observation-cache
TTL expires, project and cache the response, then read the response just before
the gateway-cache TTL expires. Ask whether the observation state is fresh,
stale, or observed, and whether an old v1 consumer still receives legacy stream
fields. The plan permits multiple answers.

Confidence: high.

Suggested disposition: add a complete wire ledger and representative JSON for
observed, stale, timed-out, unsupported-host, partial, and unavailable cases.
Define identifiers, required fields, timestamp source, maximum-age calculation,
gateway-cache interaction, and error vocabulary. Freeze a compatibility matrix
for old server, current server, old client, and current client. Either retain
legacy v1 fields for the v1 lifetime or add explicit version negotiation before
deleting them.

### P0100-A1-05 — `blocking` — The deployment locality failure branch is a second undefined architecture

Criterion: the projector remains locally substitutable and transport-pure in
single-host, unavailable-locality, and future multi-host deployments without an
unbounded implementation branch.

Evidence:

- The plan says implementation may proceed only if the dedicated backend has
  the same current environment and display-host access
  (`docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md:244-267`).
- If that fails, it calls for a typed host-observation read between the gateway
  host and projector but defines no endpoint, authentication, caller, timeout,
  cache, source identity, failure model, or ownership (`:269-273`).
- Slice 0 simultaneously requires a written locality verdict while requiring no
  live display (`:332-362`), and the plan later acknowledges that current live
  process locality and X11 access were not proven.

Consequence: Slice 0 can select either an in-process adapter or a new
interprocess observation service. Those have different security, transport,
deployment, and failure surfaces. A source-only fixture cannot truthfully prove
current X11 capability.

Reproducer: configure the dedicated backend without an X11 socket while the
dashboard gateway can inspect the display. The plan forbids gateway JSON repair
but provides neither a complete cross-host read nor a frozen unavailable
response for this case.

Confidence: high.

Suggested disposition: select one bounded P100 behavior now. The lowest-scope
choice is an in-process local adapter with a runtime capability check and a
typed unavailable result when locality is missing; defer a cross-host
observation transport to a separate authorized plan. If behavior compatibility
requires the cross-host path now, specify its complete authenticated contract
and validation surface in P100. Split the locality verdict into static
deployment wiring evidence and optional read-only runtime capability evidence;
do not claim the latter from a no-launch fixture.

### P0100-A1-06 — `blocking` — Cross-candidate write ownership and migration order are unresolved

Criterion: all four architecture candidates land without duplicate decision
logic, unsafe overlapping writes, or a module importing a successor monolith.

Evidence:

- Plan 0099 owns the workspace node, selected context, viewport, tile mode, and
  Service inspector projection and explicitly says it does not change the
  service-status wire contract
  (`docs/dev/plans/0099-2026-08-09-workspace-view-projection-deepening-plan.md:590-609`,
  `:686-700`).
- Plan 0100 Slice 5 also claims workspace, viewport, inspector, and
  selected-context migration
  (`docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md:471-501`).
- Plan 0101 requires P0100 to land before its Service State extraction and owns
  the final removal of status logic from `actions.rs`
  (`docs/dev/plans/0101-2026-08-09-route-bound-open-actions-deepening-plan.md:350-358`,
  `:490-510`, `:650-673`).

Consequence: P0099 and P0100 can both rewrite the same dashboard consumers, and
P0100 and P0101 can both move status logic out of `actions.rs`. Parallel or
out-of-order execution can create competing projectors, reintroduce local
selection, or lose changes in a dirty shared worktree.

Reproducer: execute P0099 Slice 3 or 4 concurrently with P0100 Slice 5, or run
P0101 Slice F before P0100's final module path and interface are stable. The
plans name the same files and responsibilities without a handoff contract.

Confidence: high.

Suggested disposition: add an inter-plan dependency and write-ownership ledger.
A coherent order is P0099 first for the dashboard projection seam, P0100 next
for the wire and observation input feeding that public seam, then P0101 for
residual `actions.rs` extraction and final dependency fences. P0100 should
change P0099's public input adapter and observation consumption tests, not
recreate workspace selection. P0101 should adopt the landed P0100 module and
own the final monolith budget only.

### P0100-A1-07 — `needs_evidence` — Exact transport inventory and parity boundary are deferred

Criterion: the plan's claim that CLI, HTTP, MCP, generated client, and dashboard
all receive one canonical answer is bound to every actual ingress and envelope.

Evidence:

- The plan itself says the exact MCP resource and tool set still must be
  enumerated in Slice 0.
- Current control-plane status wraps `id`, `success`, and `data`, whereas the
  action handler returns the data body for its caller to wrap. Dashboard can
  also fall back to a CLI-produced response. The intended parity boundary is
  described as `data`, but the required envelope assertions are not enumerated.

Consequence: a green parity fixture can cover one MCP tool while leaving a
resource read or fallback path divergent.

Reproducer: enumerate every `service_status` producer and every MCP resource or
tool that exposes status, then compare whether the fixture currently reaches
each one and whether it compares envelopes or only canonical data.

Confidence: high.

Suggested disposition: complete the transport matrix in Slice 0 before Slice 1.
Name each ingress, status producer, fallback, envelope owner, canonical data
path, and exact parity assertion. Keep transport-specific `id`, success, error,
and status-code mapping outside the projector.

### P0100-A1-08 — `nonblocking` — Review bounds and whole-surface validation should be explicit

Criterion: the user's five-role process is bounded to two review cycles, and a
cross-cutting Rust and dashboard change has proportional final validation.

Evidence:

- Completion criterion 10 asks for an independent implementation audit, but the
  plan does not record the Cycle 1 drift-discovery, one consolidated remediation,
  Cycle 2 closed-world rule, or stable finding ledger.
- The suggested commands are strong focused gates but omit the full Rust unit
  suite despite changes across actions, control plane, dashboard gateway,
  remote view, contracts, and generated consumers.

Consequence: the orchestrator can still enforce the process, but the durable
plan does not preserve it, and focused tests alone may miss a cross-module Rust
regression.

Reproducer: compare the plan's completion workflow and validation list with the
user's required role and review protocol and the repo's proportional-validation
policy.

Confidence: high.

Suggested disposition: append a bounded review section using these stable IDs.
After one consolidated remediation, Cycle 2 must be `closed_world` over accepted
blocking findings and critical regressions introduced by the remediation. Add
the full Rust unit gate, the validation selector bound to the frozen base, and
the exact no-launch HTTP and MCP smokes selected by P0100-A1-07. Record any
remaining nonblocking issue once instead of reopening discovery.

## Bounded Remediation Required Before Cycle 2

Perform one consolidated plan revision only:

1. Add the current-field authority and compatibility ledger, and resolve
   P0100-A1-01 without weakening current Browser Session Authority behavior by
   accident.
2. Freeze the executable async projector and adapter contract, error mapping,
   cancellation, and lock ownership for P0100-A1-02.
3. Define the gateway single-flight state machine and exact timeout semantics
   for P0100-A1-03.
4. Add the complete observation JSON, freshness, version, and legacy-client
   compatibility matrix for P0100-A1-04.
5. Choose and fully specify one locality-failure branch for P0100-A1-05.
6. Add the P0099, P0100, and P0101 dependency and write-ownership ledger for
   P0100-A1-06.
7. Complete the Slice 0 transport inventory for P0100-A1-07 and append the
   bounded review and proportional validation text from P0100-A1-08.

Cycle 2 must verify only accepted findings `P0100-A1-01` through
`P0100-A1-08` and critical contradictions introduced by their remediation. It
must not start a new broad architecture review. If an accepted blocking item
still fails after Cycle 2, split or block that implementation unit rather than
starting another optimization loop.

## Cycle 2 Closed-World Verification

Date: 2026-08-09
Review mode: `closed_world`
Review cycle: 2 of 2
Target SHA-256: `cffe5cf17248eaa799ed71e36e523cd767a74a3a8edced69ebc7ad2783a2538a`
Target hash verified: yes
Plan version: 2
Reviewer runtime: `/root/audit_plan_status_projection`

This verification was limited to accepted findings `P0100-A1-01` through
`P0100-A1-08` and contradictions introduced by their remediation. It did not
reopen architecture discovery.

### P0100-A1-01 — Pass

Version 2 now freezes a normative field-role and unavailable-behavior ledger,
makes Browser Session Authority an explicit typed preprojection authority
input, preserves its present `non_viable` gate, and prevents missing process
evidence from creating a negative verdict
(`docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md:169-264`).
It also freezes required legacy `manualBrowsers: []` plus typed unknown
provenance and distinguishes raw process observations from authority verdicts.

Residual disposition: no blocking or nonblocking residual.

### P0100-A1-02 — Pass

The plan now specifies one async `ServiceStatusProjector::project` operation,
one async substitutable observation source, the complete typed authority input,
three-stage execution, adapter-owned blocking work, cancellation cleanup, and
fatal versus partial error mapping (`:266-334`). Entry adapters retain envelope
ownership while invalid authority fails before projection (`:789-821`).

Residual disposition: no blocking or nonblocking residual.

### P0100-A1-03 — Pass

The revised keyed state machine removes the mutex-across-request design. It
freezes a 32-key bound, complete backend identity and path key, independently
owned request, shared waiter result, success-only five-second retention,
failure sharing without caching, cancellation and panic cleanup, late-result
request-id protection, and uncached overflow (`:595-636`). It explicitly
preserves ten seconds for each connect, write, and read phase rather than
claiming an end-to-end deadline, with matching Slice 4 and test requirements
(`:638-640`, `:852-885`, `:975-983`).

Residual disposition: no blocking or nonblocking residual.

### P0100-A1-04 — Pass

Version 2 supplies an exact additive v1 example, required and nullable field
rules, identity representation, closed enums, deterministic ordering,
source-host privacy format, observation and validity timestamps, stale-on-receipt
semantics, cache-age interaction, and a four-way server and client
compatibility matrix (`:337-479`). All legacy v1 fields and mirrors remain for
the v1 lifetime; later removal requires a separately versioned contract plan.

Residual disposition: no blocking or nonblocking residual.

### P0100-A1-05 — Pass

The locality fork is now closed. P100 implements only in-process local or typed
unavailable adapters, with a runtime capability check. Cross-host transport,
ports, endpoints, flags, and environment variables are explicitly deferred.
Static wiring proof and optional read-only runtime capability proof are
separated, and missing runtime evidence selects unavailable rather than a new
gateway repair (`:493-532`, `:1098-1109`).

Residual disposition: no blocking or nonblocking residual.

### P0100-A1-06 — Pass

The plan freezes sequential order P0099, P0100, then P0101 and gives each plan
owned decisions, permitted handoff, forbidden overlap, and exact `actions.rs`
responsibility (`:642-659`). P0100 is limited to P0099's public status-input
adapter and observation tests and cannot recreate workspace presentation
decisions (`:887-919`). P0101 owns the final monolith budget.

Residual disposition: no blocking or nonblocking residual.

### P0100-A1-07 — Pass

The complete transport ledger fixes the parity boundary at the typed data
object and names CLI, native action, control plane, direct HTTP, dashboard
backend, CLI fallback, generated client, and all four dashboard consumers. It
also enumerates the current MCP resources and templates, records that no MCP
surface produces full status, and requires exhaustive rejection and
nonproducer assertions (`:661-725`). Slice 0 and contract tests bind these
claims to no-launch evidence (`:737-761`, `:986-1007`). Current source confirms
that `browser_command` uses `SERVICE_REQUEST_ACTIONS` and already rejects
`service_status`; the named no-launch scripts exist in `package.json`.

Residual disposition: no blocking or nonblocking residual.

### P0100-A1-08 — Pass

The plan now records the two-cycle review protocol with no Cycle 3 and carries
the stable finding ledger (`:1072-1096`). Validation includes the focused
projection and gateway tests, exact no-launch status, contracts and MCP smokes,
client and dashboard gates, formatting, strict Clippy, the full Rust suite, and
a selector bound to verified base
`ae36b272327982e3227f4dc7c5d6dc5b4b16350c` (`:1009-1038`). The current local
`HEAD` and `origin/main` both resolve to that frozen base.

Residual disposition: no blocking or nonblocking residual.

## Cycle 2 Final Verdict

Implementation-ready: **Yes**.

Residual blocking findings: none.

Residual nonblocking findings: none.

Critical contradictions introduced by remediation: none found within the
closed-world scope.

Effects: this Cycle 2 audit changed only this audit note. It did not edit Plan
0100, source, contracts, generated files, runtime state, installed services, or
commits. Other orchestrated candidate work was already present in the shared
worktree and was not modified or evaluated by this closed-world review.
