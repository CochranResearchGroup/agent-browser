# Plan 0100: Service Status Projection Authority Deepening

Date: 2026-08-09
State: REVISED FOR CYCLE 2 CLOSED-WORLD AUDIT
Plan version: 2
Review cycle: Cycle 1 remediation complete; Cycle 2 is the final plan audit
Lane: P100
Depends On:

- `docs/dev/notes/2026-04-22-agent-browser-service-roadmap.md`
- `docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md`
- `docs/dev/plans/0070-2026-07-09-browser-session-authority-plan.md`
- `docs/dev/plans/0074-2026-07-09-dashboard-gateway-plan.md`
- `docs/dev/plans/0075-2026-07-09-cross-seam-interlock-tests-plan.md`
- `docs/dev/plans/0096-2026-08-07-durable-remote-view-handoff-plan.md`

## Goal

Create one deep Service Status Projection module that turns reconciled Service
State, typed preprojection authority, and explicitly scoped runtime
observations into the status contract used by CLI, HTTP, generated clients, and
the dashboard. MCP collection resources continue to expose their narrower
Service State projections; the current MCP tool surface intentionally has no
full Service Status producer. Remove the dashboard gateway's ability to
reinterpret browser, stream, route, or display authority by mutating arbitrary
JSON after the canonical response has been produced.

The resulting interface must remain locally substitutable. Production uses a
bounded host-local observation adapter. Tests use an in-memory adapter. A
deployment without local display access uses an explicit unavailable adapter
whose typed result cannot be mistaken for negative browser proof.

## Planning Delegation Receipt

- Disposition: `spawned`
- Bounded lane: Candidate 3 deep analysis and implementation-ready planning for
  Service Status Projection authority
- Runtime handle: `/root/plan_status_projection`
- Parent orchestrator: `/root`
- Write scope: this plan only
- Source edits, commits, runtime mutation, and live-system effects: none
- Terminal status: completed for plan authoring
- Evidence returned: CodeGraph flow and blast-radius reads, repo policy reads,
  Graphiti advisory recall, direct reads of the oversized `actions.rs` source,
  prior roadmap and plan decisions, contract and consumer inspection
- Reconciliation: the parent orchestrator retains integration and finding
  disposition authority
- Cycle 1 reviewer: `/root/audit_plan_status_projection`
- Cycle 1 audit artifact:
  `docs/dev/notes/0100-2026-08-09-service-status-projection-authority-plan-audit.md`
- Cycle 1 disposition: findings `P0100-A1-01` through `P0100-A1-08`
  accepted for one consolidated remediation; no source execution authorized

## Source Audit

### Indexed structure

CodeGraph was healthy with 419 indexed files, 14,341 nodes, and 43,350 edges.
It traced the canonical status path as:

```text
daemon::handle_connection
  -> ControlPlaneHandle::service_status_response
  -> service_status_projection::project_service_status
```

It also traced the dashboard-only mutation path as:

```text
dashboard::handle_service_api_request
  -> proxy_dashboard_service_api_request
  -> repair_dashboard_service_status_response
  -> repair_dashboard_service_status_value
  -> remote_view::route_display_content
  -> remote_view::route_display_content_with_bound_display
```

`cli/src/native/actions.rs` is intentionally skipped by CodeGraph because it is
larger than the one MiB index limit. The relevant `handle_service_status`,
`refresh_remote_view_stream_urls`, Guacamole URL selection, and status tests
were therefore read directly, as required by the oversized-file policy.

### Advisory memory

Graphiti group `agent_browser_main` was healthy and returned ten facts. The
useful sourced direction was that agent-browser owns browser lifecycle and that
the roadmap targets authoritative service-owned state. It did not return a
specific prior decision for this projection seam, so current repo source and
plans control this plan.

### Observed implementation facts

- `cli/src/native/service_status_projection.rs` currently owns only closed-tab
  response compaction and manual runtime browser projection.
- `cli/src/native/actions.rs::handle_service_status` separately refreshes
  remote-view URLs, CDP streams, attachability, profile readiness, allocations,
  retained-display summaries, Browser Session Authority, launch configuration,
  closed-tab projection, and browser process stats.
- `cli/src/native/control_plane.rs::service_status_response` independently
  reconciles and persists Service State, then assembles a similar but not
  identical response. It does not run the same URL refresh or process-stat
  injection path.
- `cli/src/native/stream/dashboard.rs` repairs only proxied
  `GET /api/service/status` responses. It parses arbitrary JSON, inserts
  `frameUrl`, `externalUrl`, and `displayContent`, rebuilds HTTP bytes, and then
  sends the result to dashboard consumers.
- Direct CLI, HTTP, and generated-client consumers therefore do not all
  observe the same response construction path. MCP exposes narrower Service
  State resources rather than a full Service Status response.
- The dashboard gateway preserves two important performance protections: a
  ten-second status-specific backend timeout and a five-second completed
  response cache that coalesces concurrent status polls.
- `remote_view::route_display_content` performs a host-local `xwininfo` probe
  under a two-second command timeout, keeps a five-second per-display cache,
  and only probes configured or explicitly bound displays.
- `packages/client/src/service-observability.generated.d.ts` models the status
  contract, while multiple dashboard surfaces redeclare partial local
  `ServiceStatusData` shapes.
- `docs/dev/contracts/service-status-response.v1.schema.json` is additive at
  the top level and currently treats `service_state` as an open object.

## Architectural Friction

The current seam is split by deployment path rather than by authority:

```text
                     +→ actions.rs response assembly → CLI and action dispatch
Persisted state ─────+
                     +→ control_plane.rs assembly ────→ backend HTTP status
                                                        |
                                                        v
                                      dashboard.rs arbitrary JSON repair
                                                        |
                                                        v
                                                   dashboard only
```

This produces shallow modules and poor locality:

- callers must know which refreshes and derived fields they need before calling
  `project_service_status`;
- the projector's interface exposes less leverage than its name implies;
- host-local observation, wire compatibility, audience URL selection, and
  browser authority are conflated in the gateway repair;
- tests can cover individual helpers while missing divergence between their
  call sites;
- a dashboard-only repair can make a stream look more complete without changing
  the service authority seen by other consumers.

## Deletion Test

Deleting the current `service_status_projection.rs` would move a small amount of
closed-tab and manual-browser logic back into two callers. It would not remove
the larger response-construction complexity. The current module is therefore
shallow relative to the concept it claims to own.

Deleting `repair_dashboard_service_status_value` would remove the dashboard's
only current host-local repair, but the missing behavior would immediately
reappear in dashboard TypeScript, `actions.rs`, or another transport caller.
The observed complexity is real, but its current seam is wrong.

Deleting the proposed deep module would require reimplementing response
construction in the action and control-plane paths, observation policy in each
deployment adapter, and compatibility rules in every consumer. That
concentration is the desired deletion-test result.

## Architecture Decision

### Authority and observation are different

Persisted and reconciled Service State remains the authority for browser,
session, tab, route, lease, proof, inventory, and actionability. A status read
must not promote a host-local observation into persisted truth.

Browser Session Authority is the existing, explicit exception to a purely
persisted-state input. Its typed snapshot is built before projection by the
reconciliation lane and is a preprojection authority input. The dashboard may
preserve its current `non_viable` actionability gate. Raw process inventory,
RSS, cleanup-candidate, and correlation facts remain observations. They cannot
directly gate a browser or manufacture a `non_viable` verdict.

When process evidence is unavailable, Browser Session Authority emits an
`unknown` availability with no per-browser verdict for the affected browser.
It never emits a negative or `non_viable` verdict from missing evidence. An
absent verdict leaves the existing Service State and P99 authority-ledger
decision in force.

Host-local enrichment belongs behind a typed owned adapter that feeds the deep
Service Status Projection module. It does not belong in the dashboard gateway
as ad hoc JSON rewriting, and it does not belong inside the canonical Service
State model merely because the current deployment is usually single-host.

The deep module owns both facts:

1. how authoritative state becomes the stable status response; and
2. how optional runtime observations are attached with source, scope,
   freshness, and availability metadata without changing authority.

### Field-role and unavailable-behavior ledger

This ledger is normative for v1. A later implementation must not reclassify a
field based on convenience at a caller.

| Existing field or behavior | P100 role | Exact unavailable behavior | Legacy v1 behavior |
| --- | --- | --- | --- |
| `service_state` | Reconciled authority snapshot, response-compacted without mutating persistence | Repository, deserialization, or reconciliation failure fails the entire read | Required and retained |
| `control_plane` | Typed preprojection authority supplied by the owning worker | Missing required snapshot fails the entire read | Existing shape retained |
| `profileAllocations` | Authority-derived response from reconciled profiles, sessions, leases, browsers, and jobs | Derived from authority or the read fails | Required and retained |
| `retainedDisplayAllocations` | Authority-derived diagnostic projection | Derived from authority or the read fails | Existing optionality and shape retained |
| `closedTabProjection` | Pure response-only authority projection | Derived from authority or the read fails | Required and retained |
| `browserSessionAuthority` | Typed preprojection reconciled-authority input; its present verdict may gate P99 actionability | Emit availability `unknown`, resource pressure `unknown`, and no affected browser verdict; never emit negative proof | Field and current gate retained for v1 |
| Browser process stats and raw resource facts | Host observation | Omit newly observed stats, report typed unavailable or partial observation, and do not gate | Existing mirrored stats remain optional and are never removed from an already supplied record |
| `manualBrowsers` | Host observation joined to authority route fields by stable identity | Required legacy value is `[]`; typed projection says unavailable, so absence is unknown rather than proof that no manual browser exists | Required array retained for v1 |
| Profile readiness refresh | Preprojection authority derivation from Service State and owned readiness evidence | Required derivation failure fails the read; optional external evidence yields the existing typed readiness unknown state | Existing profile fields retained |
| RDP or Guacamole URL refresh | Compatibility mirror from explicit local route presentation | Preserve stored and explicit route-descriptor URLs; fill nothing; typed observation reports unavailable | Existing `url`, `frameUrl`, and `externalUrl` fields retained for v1 |
| Remote-view attachability | Typed preprojection authority derivation; it may recommend recovery but cannot upgrade readiness | Preserve current authority, emit `not_checked` or typed unknown, and enable no new action | Existing attachability fields retained |
| `displayContent` | Host observation only unless it already arrived through owned remote-view proof | Preserve an existing authoritative proof field; otherwise leave the legacy field absent and report typed unavailable | Existing field remains supported and is never deleted in v1 |
| `launchConfig` | Typed local configuration input, not a display or process observation | Required configuration read failure fails the read; optional manifest evidence uses its existing null or warning states | Existing shape retained |

An unavailable observation is always unknown. It cannot mean no manual
browser, no process, terminal-only display, route failure, non-viable browser,
or disabled action.

Browser Session Authority adds required `availability` with enum `available`,
`partial`, or `unknown`. When the process/resource input for a modeled browser
is unavailable, the authority builder must not synthesize a viable,
attention, or non-viable verdict for that browser. The summary counts only
emitted verdicts, so it does not pretend that unknown browsers were classified.
The exact unavailable form is:

```json
{
  "browserSessionAuthority": {
    "schemaVersion": 1,
    "availability": "unknown",
    "summary": {
      "modeledBrowserCount": 1,
      "viableBrowserCount": 0,
      "attentionBrowserCount": 0,
      "nonViableBrowserCount": 0,
      "unknownBrowserCount": 1
    },
    "resourcePressure": {
      "state": "unknown",
      "totalProcessCount": 0,
      "correlatedProcessCount": 0,
      "candidateCount": 0,
      "protectedCount": 0,
      "observedCount": 0,
      "observedUnownedAgentBrowserProcessCount": 0,
      "candidateRssBytes": 0,
      "totalRssBytes": 0,
      "reasons": ["process_inventory_unavailable"]
    },
    "browserVerdicts": []
  }
}
```

`modeledBrowserCount` remains the number of reconciled modeled browsers;
`unknownBrowserCount` is required and makes the unclassified portion explicit.
For `partial`, verdicts are emitted only for browsers whose required evidence
was available, and the two counts sum to `modeledBrowserCount`. Existing v1
clients ignore the additive fields and see no false negative verdict.

### Local-substitutable seam

Use one async external module interface with an internal typed observation
source:

```rust
pub(crate) struct ServiceStatusProjector {
    observations: Arc<dyn StatusObservationSource>,
    clock: Arc<dyn ProjectionClock>,
}

impl ServiceStatusProjector {
    pub(crate) async fn project(
        &self,
        input: StatusAuthorityInput,
    ) -> Result<ServiceStatusResponse, ServiceStatusProjectionError>;
}

#[async_trait]
pub(crate) trait StatusObservationSource: Send + Sync {
    async fn snapshot(
        &self,
        request: StatusObservationRequest,
    ) -> StatusObservationSnapshot;
}
```

The interface includes the input invariants and observation error semantics,
not only the Rust method shape. `StatusAuthorityInput` owns a validated,
reconciled Service State snapshot, the worker snapshot, Browser Session
Authority snapshot, launch configuration, and full-history mode. Callers
receive a complete typed response. They do not select individual refresh
helpers or mutate the output.

The projector performs exactly three stages:

1. validate the typed authority input and construct an immutable observation
   request;
2. await one observation snapshot from the owned source;
3. run pure authority projection, compatibility mirroring, and serialization.

The local adapter owns every `spawn_blocking` call, per-probe timeout, child or
process-group cleanup, in-flight probe cleanup, and conversion of probe errors
to a snapshot. Request cancellation drops only that waiter's future. A started
blocking probe remains adapter-owned, its OS child remains protected by the
existing bounded `timeout` kill behavior or an equivalent kill guard, and its
completion removes the adapter's in-flight registration even when the result
is discarded.

Invalid authoritative input, repository read failure, reconciliation failure,
required configuration failure, and serialization failure return
`ServiceStatusProjectionError` and fail the status read. Display timeout,
unsupported host, missing X11 capability, runtime-profile observation failure,
and process-inventory observation failure return a successful status response
with typed partial or unavailable observations. Entry adapters retain their
current envelopes: action and CLI use their existing `success`, `data`, and
`error` shape; control-plane responses retain `id`; HTTP retains its current
status mapping and JSON envelope.

The seam is real because deployments vary:

- `LocalStatusObservationAdapter` reads runtime profiles, process facts,
  configured route presentation, and authorized route-display observations;
- `UnavailableStatusObservationAdapter` records that the current process does
  not own the required host locality;
- `InMemoryStatusObservationAdapter` supplies deterministic observations in
  tests.

The production and unavailable adapters represent actual deployment variation.
The in-memory adapter makes the same interface the test surface.

### Exact additive v1 observation contract

Every server after P100 includes `statusProjection`. Older v1 servers may
omit it. All fields shown below are required when `statusProjection` is
present. Nullable fields are present with JSON `null`, not omitted. Stream
identity is an array of explicit `browserId` and `streamId` fields, so there is
no compound-key encoding contract.

```json
{
  "statusProjection": {
    "schemaVersion": 1,
    "authority": {
      "source": "reconciled_service_state",
      "projectedAt": "2026-08-09T21:00:05Z"
    },
    "observations": {
      "state": "partial",
      "source": "local_status_observation_adapter",
      "sourceHostId": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "observedAt": "2026-08-09T21:00:04Z",
      "validUntil": "2026-08-09T21:00:09Z",
      "maxAgeMs": 5000,
      "manualBrowsersState": "observed",
      "browserProcessState": "unavailable",
      "errors": [
        {
          "code": "process_inventory_unavailable",
          "subject": "host",
          "message": "process inventory was unavailable"
        }
      ],
      "viewStreams": [
        {
          "browserId": "session:work",
          "streamId": "remote-headed-view",
          "state": "observed",
          "observedAt": "2026-08-09T21:00:04Z",
          "validUntil": "2026-08-09T21:00:09Z",
          "maxAgeMs": 5000,
          "routePresentation": {
            "frameUrl": "/guacamole/#/client/opaque",
            "externalUrl": "/guacamole/#/client/opaque",
            "source": "configured_client_url"
          },
          "displayContent": {
            "state": "browser_window_visible",
            "displayName": ":10",
            "windows": []
          }
        }
      ]
    }
  }
}
```

Enums are frozen as follows:

- observations `state`: `complete`, `partial`, `unavailable`;
- observations `source`: `local_status_observation_adapter`,
  `unavailable_status_observation_adapter`,
  `in_memory_status_observation_adapter`;
- manual and process state: `observed`, `partial`, `unavailable`;
- stream `state`: `observed`, `timed_out`, `unsupported`, `unavailable`,
  `failed`;
- route presentation `source`: `route_descriptor`, `retained_stream`,
  `configured_client_url`;
- error `code`: `display_probe_timeout`, `display_probe_unsupported`,
  `display_probe_unavailable`, `display_probe_failed`,
  `runtime_profile_unavailable`, `process_inventory_unavailable`,
  `configured_route_unavailable`.

The required and null rules are exact:

- `statusProjection`, `schemaVersion`, `authority`, and `observations` are
  required on a P100 response. `authority.source` and
  `authority.projectedAt` are required and non-null.
- Every observations object requires `state`, `source`, `sourceHostId`,
  `observedAt`, `validUntil`, `maxAgeMs`, `manualBrowsersState`,
  `browserProcessState`, `errors`, and `viewStreams`. Arrays are present even
  when empty. `maxAgeMs` is a non-negative integer and remains present even
  when timestamps are null.
- Every view-stream observation requires `browserId`, `streamId`, `state`,
  `observedAt`, `validUntil`, `maxAgeMs`, `routePresentation`, and
  `displayContent`. `observedAt`, `validUntil`, `routePresentation`, and
  `displayContent` are explicit nulls when that result is unavailable;
  `maxAgeMs` remains the configured non-negative freshness interval.
- A non-null `routePresentation` requires non-null `frameUrl`, `externalUrl`,
  and `source`. A non-null `displayContent` retains the existing typed
  `RouteDisplayContent` shape. Every error requires non-null `code`, `subject`,
  and `message`.

Top-level `complete` means every requested and supported observation completed;
`partial` means at least one completed and at least one did not; `unavailable`
means none completed or the producer lacked locality. Empty successful manual
browser discovery is `manualBrowsersState: "observed"` with legacy
`manualBrowsers: []`. View streams sort by `(browserId, streamId)` and errors
sort by `(subject, code, message)` so byte serialization is deterministic.

`sourceHostId` is required and nullable. A non-null value matches
`^sha256:[0-9a-f]{64}$`. The local adapter derives this non-secret opaque
identifier from host identity; it never exposes the raw machine id or
hostname. It is `null` when host identity or locality is unavailable.
`observedAt` is the UTC completion time of the observation snapshot.
`validUntil` is `observedAt + maxAgeMs`. Both are `null` when no observation
succeeded. The top-level `validUntil` is the earliest child expiry. Each
stream carries its own timestamps because probes may complete at different
times.

Wire variants are also deterministic:

| Case | Exact representation |
| --- | --- |
| Complete and current | observations `state: "complete"`; all applicable child states are `observed`; errors is empty; timestamps are non-null |
| Partial | observations `state: "partial"`; successful children keep values and timestamps; failed children use their typed state and required error |
| Timed out | stream `state: "timed_out"`, null observation values, and one `display_probe_timeout` error for that stream |
| Unsupported host | stream `state: "unsupported"`, null observation values, and one `display_probe_unsupported` error for that stream |
| Unavailable | observations `state: "unavailable"`, source `"unavailable_status_observation_adapter"`, nullable source and time fields set to null, required arrays present, and one or more typed unavailable errors; legacy `manualBrowsers` is `[]` |
| Stale on receipt | Serialized child remains `state: "observed"`; its non-null `validUntil` is before the consumer's current time, so staleness is derived rather than serialized as a second state |

Consumers derive freshness as `currentTime <= validUntil`. They do not trust a
serialized `fresh` boolean. A display observation can be collected near the
end of its five-second probe-cache life and then be served near the end of the
five-second gateway response-cache life. It can therefore be nearly ten
seconds old when received and may already be past `validUntil`; that is a
valid cached response, and the consumer must present it as stale.

All legacy v1 fields are retained for the full v1 lifetime. There is no
installed-dashboard-based deletion criterion and no request-time client
version inference. Only the projection module may mirror a client URL or
display observation into legacy fields, and the mirror must never alter
ownership, actionability, lease state, operator-visible proof, or canonical
inventory class.

Compatibility matrix:

| Server | Client | Required behavior |
| --- | --- | --- |
| Old v1 | Old v1 | Existing legacy behavior unchanged |
| Old v1 | Current | `statusProjection` absent; P99 compatibility input uses legacy authority fields and treats missing observation metadata as unknown |
| Current | Old v1 | All legacy v1 fields remain populated under their existing rules; additive `statusProjection` is ignored safely |
| Current | Current | Typed projection is primary for observation provenance and freshness; legacy mirrors are not double-counted or used to upgrade authority |

### Relationship to P45

This plan advances P45 but must not absorb its `remote_view::proof` or
`remote_view::inventory` responsibilities. Service Status Projection serializes
their typed answers. It does not decide whether a browser is operator-visible
or which dashboard class is live.

If a fresh host observation should change proof or actionability, that change
must enter through the owned remote-view proof and reconciliation path before
projection. A read-only status adapter may show contradictory or stale
evidence, but it cannot upgrade or downgrade authoritative proof by itself.

## Deployment Constraint Gate

P100 implements only an in-process local adapter plus a runtime capability
check. When the producing process lacks environment, process-namespace, X11,
runtime-profile, or display-host locality, it uses the typed unavailable
adapter behavior. P100 does not add a gateway-to-host read, network port,
authenticated observation endpoint, remote adapter, or multi-host transport.
Cross-host observation is deferred to a separate plan with its own authority
and security review.

Before moving the host-local probe, record a static wiring matrix for:

- the dashboard gateway process;
- the dedicated service backend daemon selected by the dashboard;
- ordinary session daemons;
- direct CLI status;
- direct HTTP status;
- MCP resource and tool reads;
- future remote or multi-host backends.

For each process, record whether it owns:

- the current `.env` route presentation values;
- the X11 socket and `xwininfo` capability for the selected display;
- the process namespace used for browser statistics;
- the runtime-profile registry;
- the service-state repository and reconciliation lane.

Static wiring evidence comes from unit definitions, environment-file wiring,
spawn inheritance, and source-level backend selection. It proves intended
deployment, not current capability. Optional read-only runtime proof may check
the installed process environment, X11 socket, authorized display probe,
process namespace, and runtime-profile registry. Runtime proof is not required
for no-launch implementation, and a skipped or failed capability proof yields
typed unavailable observations rather than selecting another architecture.

The local adapter runs in the daemon that produces the canonical response. The
dashboard gateway becomes a pure transport adapter even when that daemon
reports unavailable observations. Missing locality never re-enables gateway
JSON mutation.

## Target Module Layout

Keep `cli/src/native/service_status_projection.rs` as the Rust module root
during migration and add implementation files under
`cli/src/native/service_status_projection/`. Rust must never contain both that
root file and `service_status_projection/mod.rs` at the same time. After the
last compatibility caller is removed, a separate mechanical move may replace
the root file with `service_status_projection/mod.rs` if the directory layout
is clearer.

- `service_status_projection.rs`
  - exports the small projector interface and typed response
  - owns assembly order and compatibility versioning
- `authority.rs`
  - accepts reconciled Service State and control-plane snapshot
  - accepts Browser Session Authority as a typed preprojection authority input
  - derives profile allocations, retained display summaries, closed-tab
    compaction, and response-only authority metadata
- `observation.rs`
  - defines typed host observations and adapter error vocabulary
  - never mutates Service State
- `local_observation.rs`
  - production adapter for manual runtime browsers, raw process stats,
    configured route presentation, and display probes
  - moves blocking display inspection to a bounded blocking task
- `compatibility.rs`
  - owns legacy v1 field mirroring and schema-version gates
  - forbids removal within v1; any later removal requires a separately approved
    versioned-contract plan

Keep remote-view display parsing and proof vocabulary in their existing owned
modules or the P45 target modules. The status adapter consumes their typed
results; it does not duplicate their implementation.

## Interface Invariants

- One call returns the complete status response used by all transport adapters.
- The input Service State is never mutated by projection or compaction.
- Reconciliation and any permitted persistence happen before projection.
- An unavailable observation is distinct from an observation that proves
  absence.
- Every host observation identifies source scope and freshness.
- A raw observation cannot create or change a browser, route, lease, proof,
  inventory class, Browser Session Authority verdict, or enabled action.
- A present typed Browser Session Authority verdict remains preprojection
  authority and may preserve its current P99 actionability gate.
- Missing Browser Session Authority evidence is unknown and produces no
  negative per-browser verdict.
- A configured Guacamole root URL is never fabricated into a client URL.
- Explicit route descriptors and provider URLs outrank compatibility
  environment values.
- Existing explicit `frameUrl` and `externalUrl` values are never overwritten
  by a lower-authority compatibility source.
- Only `remote_headed` browsers with `rdp_gateway` streams are eligible for the
  legacy Guacamole presentation mirror.
- The dashboard gateway preserves typed gateway errors, the ten-second status
  timeout, and the five-second completed-response coalescing cache.
- The host display adapter preserves authorization checks, a bounded command
  timeout, and a five-second per-display observation cache.
- No global async cache lock is held while a blocking display command runs.

## Dashboard Status Single-Flight State Machine

Replace the current mutex-across-request cache with a bounded keyed state
machine. The key is the selected backend identity plus the complete status
path, including query. Current backend identity is the dedicated session name
and resolved port. A changed port creates a different key.

Each key is in exactly one state:

```text
vacant
  -> in_flight(request_id, shared_result)
  -> ready(completed_at, successful_2xx_bytes)
  -> vacant after five seconds
```

Rules:

1. The mutex guards map lookup, expired-entry pruning, in-flight registration,
   ready insertion, and removal only. It is never held while connecting,
   writing, reading, projecting, probing, or awaiting a shared result.
2. The first miss registers `in_flight`, starts one independently owned Tokio
   task, releases the mutex, and then waits on the shared result.
3. Every current waiter subscribes to that same result. Cancelling or dropping
   one waiter does not cancel the backend request.
4. Transport errors, phase timeouts, non-2xx responses, and invalid HTTP are
   delivered once to all current waiters and then removed. Failures are never
   cached. The pure gateway does not parse or validate successful JSON bytes.
5. Successful 2xx response bytes are cached for five seconds measured from
   backend completion, not request start or observation time.
6. The map holds at most 32 keys. Registration first removes expired ready
   entries, then evicts the oldest ready entry. If all 32 entries are in flight,
   the excess request runs uncached and cannot cancel or replace an existing
   flight.
7. A task panic or cancellation publishes the existing typed backend failure
   to current waiters and removes its own request id. A late completion may not
   overwrite a newer in-flight registration for the same key.

The source-established ten-second status timeout remains a separate
ten-second bound for each connect, write, and read I/O phase. It is explicitly
not a ten-second end-to-end deadline. Projection time occurs while awaiting the
backend read and remains subject to that read-phase bound. Other dashboard
gateway paths keep their existing timeout constants.

## Inter-Plan Execution and Write-Ownership Ledger

Execution is sequential:

1. P0099 lands first and owns the Workspace View Projection seam.
2. P0100 lands next and owns Service Status wire projection and observation.
3. P0101 lands last and owns the final `actions.rs` extraction budget and
   dependency fences.

| Plan | Owned decisions and files | Permitted handoff to the next plan | Forbidden overlap |
| --- | --- | --- | --- |
| P0099 | Workspace authority ledger, view projection, stream choice, readiness, selected context, viewport, tiles, and Service inspector | Exposes one public raw-status input adapter and observation input consumed by P0100 tests | P0100 must not recreate workspace selection, stream ranking, readiness precedence, or viewport behavior |
| P0100 | Rust Service Status Projector, status schema, generated observability client, dashboard status transport/cache, and P0099's public status-input adapter plus observation-consumption tests | Exposes the landed projector interface and leaves only a call from `actions.rs` | P0100 must not conduct P0099 presentation migration or P0101's general monolith extraction |
| P0101 | Route-bound open and residual domain extraction from `actions.rs`; final monolith size and dependency gate | Adopts the landed P0100 interface and removes any residual status implementation | P0101 must not fork or replace the projector, schema, observation source, or status cache semantics |

P0100 may edit `actions.rs` only enough to parse its existing entry input, call
the projector, and preserve the existing envelope. That edit does not count as
the final `actions.rs` budget. P0101 owns the final budget and dependency
assertions. The three plans must not execute concurrently in one dirty
worktree.

## Complete Transport, Producer, and Parity Ledger

The canonical parity boundary is the typed `ServiceStatusResponse` data object.
Transport-specific request ids, JSON-RPC ids, `success`, `error`, HTTP status,
content headers, and client unwrapping remain outside the projector.

| Surface | Current producer or consumer | Envelope owner | P100 parity assertion |
| --- | --- | --- | --- |
| CLI `agent-browser service status` | CLI parser creates `service_status`; `main.rs` sends it and renders the command response | CLI command executor and output formatter own `success`, `data`, `error`, exit status, watch formatting, and `--full-tab-history` | With fixed authority, clock, and observations, `data` deep-equals projector output; envelope and exit behavior retain dedicated tests |
| Native action path | `actions.rs::execute_command` dispatches to `handle_service_status` | Action executor owns command id and success or error envelope | Handler performs no response assembly and returns the projector result or typed fatal error |
| Control-plane daemon path | `daemon.rs::handle_connection` special-cases `service_status` and calls `ControlPlaneHandle::service_status_response` | Control-plane entry retains `id`, `success`, `data`, and `error` | Its `data` deep-equals action data for the same typed authority input; invalid authority is `success:false`, never default state |
| Direct stream HTTP | `stream/http.rs` matches `GET /api/service/status`, creates `service_status_command`, relays it, and calls `write_json_result` | HTTP adapter owns status code, CORS, headers, and `{success,data,error}` | JSON `data` validates the schema and deep-equals the relayed projector output; `full-tab-history` query affects only its typed input |
| Dashboard gateway backend | `dashboard.rs::handle_service_api_request` selects the backend and calls the single-flight proxy | Gateway owns backend-selection and typed gateway-error HTTP envelope | Successful body is byte-for-byte the backend response; no JSON repair or authority mutation occurs |
| Dashboard CLI fallback | `dashboard.rs::service_api_cli_fallback` executes CLI `service status` when backend selection or proxying cannot supply a response | Gateway converts CLI JSON bytes to the existing HTTP response | Fallback `data` validates the same schema; transport fallback metadata cannot change projection fields |
| Generated client | `getServiceStatus()` calls `GET /api/service/status`; `serviceGet` unwraps the HTTP `data` object | Client helper owns fetch, query, error conversion, and data unwrapping | Returned value validates as `ServiceStatusResponse` and retains observation nulls, enums, and timestamps |
| Dashboard workspace navigator | Fetches `${SERVICE_API_BASE}/status` | Dashboard gateway supplies HTTP envelope; P0099 public adapter owns dashboard normalization | Reads P0099's public adapter only; no local ownership or selection reconstruction |
| Dashboard remote viewport | Fetches `${SERVICE_API_BASE}/status` | Same | Reads the same P0099 adapter and may current-derive observation freshness only |
| Dashboard Service panel | Fetches `${SERVICE_API_BASE}/status` | Same | Displays canonical fields and labels observations separately from authority |
| Selected-workspace context hook | Fetches `${SERVICE_API_BASE}/status` | Same | Uses the same P0099 adapter; no parallel partial status type |

MCP inventory is explicit:

- `browser_command` rejects `action=service_status` before daemon dispatch.
- `service_request` does not expose `service_status` in its action contract.
- There is no current MCP resource or tool that returns the full
  `ServiceStatusResponse`. P100 adds none.
- The complete static resource list is `agent-browser://contracts`,
  `agent-browser://access-plan`,
  `agent-browser://browser-capability-registry`,
  `agent-browser://incidents`, `agent-browser://profiles`,
  `agent-browser://sessions`, `agent-browser://browsers`,
  `agent-browser://display-allocations`,
  `agent-browser://remote-view-routes`, `agent-browser://route-pool`,
  `agent-browser://viewer-leases`, `agent-browser://tabs`,
  `agent-browser://monitors`, `agent-browser://site-policies`,
  `agent-browser://providers`, `agent-browser://challenges`,
  `agent-browser://jobs`, and `agent-browser://events`.
- The complete template list is
  `agent-browser://access-plan{?...}`,
  `agent-browser://incidents/{incident_id}/activity`,
  `agent-browser://profiles/lookup{?...}`,
  `agent-browser://profiles/{profile_id}/readiness`,
  `agent-browser://profiles/{profile_id}/allocation`, and
  `agent-browser://profiles/{profile_id}/seeding-handoff{?...}`. The `{?...}`
  selector sets remain owned by their existing constants and must deep-equal
  `resources/templates/list`; P100 does not edit or abbreviate them on wire.
- The tools that can route a generic action are exactly `service_request` and
  `browser_command`; both retain explicit full-status rejection. The remaining
  `service_*` read, preflight, trace, incident, lifecycle, monitor, registry,
  policy, provider, profile, session, job, and remedy tools, plus every
  `browser_*` automation tool, are narrower operations and never return
  `ServiceStatusResponse`. A `tools/list` contract assertion checks the entire
  advertised set rather than relying on a hand-maintained allowlist: no tool
  named or schematized as a full-status read may appear.
- State-backed resources load and refresh their own Service State snapshot
  through `read_service_mcp_resource_from_state`; contracts metadata is static,
  while access-plan and templates return their separately owned projections.
  None exposes `statusProjection`, dashboard observations, or the full status
  envelope. P100 parity is limited to deep equality of shared authoritative
  records such as browsers, sessions, tabs, profiles, routes, and allocations.
  `pnpm test:mcp-read-no-launch` proves the resource surface remains readable
  and the exhaustive nonproducer assertion holds; it is not evidence of a
  nonexistent full-status MCP producer.

## Implementation Slices

### Slice 0: Freeze Contracts and Prove Deployment Locality

Files:

- `cli/src/native/actions.rs`
- `cli/src/native/control_plane.rs`
- `cli/src/native/stream/dashboard.rs`
- `cli/src/native/remote_view.rs`
- `cli/src/connection.rs`
- workstation and dashboard user-unit fixtures

Work:

1. Add a no-launch fixture that calls direct action status, control-plane
   status, direct HTTP status, dashboard-proxied status, dashboard CLI fallback,
   and generated-client status with the same input.
2. Record the current differences in URL fields, display observations, process
   stats, profile readiness, and derived summaries.
3. Add static wiring fixtures for environment inheritance, capability checks,
   typed unavailable fallback, and dedicated backend selection. Keep optional
   read-only runtime capability proof separate.
4. Freeze the ten-second status timeout, five-second response cache, two-second
   display command timeout, and five-second display observation cache.
5. Add a contract fixture proving that a Guacamole root remains a root and that
   a configured client URL fills only missing compatibility fields.
6. Assert explicitly that MCP `browser_command` rejects `service_status`,
   `service_request` omits it, and all listed MCP resources and templates remain
   outside the full-status parity boundary.

Acceptance:

- The fixture fails on the current divergent assembly paths for the expected
  reasons.
- The implementation packet has a written locality verdict before moving the
  adapter, with static wiring and optional runtime evidence labeled separately.
- No live browser, Guacamole route, or display is required.

### Slice 1: Introduce the Deep Typed Module

Files:

- `cli/src/native/service_status_projection.rs` module root plus new
  `cli/src/native/service_status_projection/` implementation files
- `cli/src/native/mod.rs`

Work:

1. Define the exact async projector and observation-source interface, typed
   authority input, typed observation output, typed projection metadata, fatal
   error, and typed status response.
2. Move closed-tab compaction and manual-browser joining behind the projector.
3. Inject a clock and an in-memory observation adapter, then test cancellation
   and fatal versus partial error mapping through the public interface.
4. Preserve existing serialization names and optional-field compatibility.
5. Make projector tests exercise only the public interface.

Acceptance:

- The module has one async public projection method.
- Tests do not inspect private helpers or perform host I/O.
- Deleting old shallow helper tests does not reduce observable contract
  coverage because replacement tests cross the new interface.

### Slice 2: Converge Action and Control-Plane Status

Files:

- `cli/src/native/actions.rs`
- `cli/src/native/control_plane.rs`
- `cli/src/native/daemon.rs`
- service model contract assertions

Work:

1. Move response assembly, derived summaries, closed-tab metadata, and process
   observation attachment into the projector.
2. Make both status entry points reconcile first and then call the same
   projector.
3. Build Browser Session Authority before projection as typed reconciled
   authority. Preserve its current gate when a verdict exists, and emit no
   negative verdict when raw process evidence is unavailable.
4. Remove caller-selected sequences of refresh helpers.
5. Preserve the action and control-plane envelopes while making their `data`
   contract identical for identical authority input and observation scope.
6. Fail invalid authoritative state and repository or reconciliation errors
   through each entry adapter's existing failure envelope.
7. Keep persistence out of projection.

Acceptance:

- Direct action, control-plane, CLI, and HTTP fixtures deep-compare the same
  canonical fields.
- `actions.rs` and `control_plane.rs` do not assemble status fields manually.
- Invalid serialized Service State fails through the frozen entry adapter's
  existing failure envelope before the typed projector is called. P100 does
  not permit invalid action or control-plane state to default silently.

### Slice 3: Move Host-Local Enrichment Behind the Adapter

Files:

- new local observation adapter
- `cli/src/native/remote_view.rs`
- `cli/src/native/browser_session_authority.rs`
- runtime-profile and process-stat helpers as needed

Work:

1. Move manual runtime browser discovery, browser process facts, configured
   route presentation, and authorized display observation into typed adapter
   results.
2. Run blocking display inspection through adapter-owned `spawn_blocking`,
   per-probe timeout, child cleanup, and in-flight cleanup.
3. Preserve display authorization and cache semantics.
4. Emit explicit unavailable, stale, timed-out, unsupported-host, and observed
   states.
5. Do not persist adapter output during a status read.

Acceptance:

- The local, unavailable, and in-memory adapters satisfy the same interface.
- An unavailable adapter cannot produce `terminal_only`,
  `browser_window_visible`, or any other observation claim.
- Adapter raw process output cannot directly create a Browser Session Authority
  verdict or change authoritative proof or dashboard actionability.

### Slice 4: Make the Dashboard Gateway a Pure Transport Adapter

Files:

- `cli/src/native/stream/dashboard.rs`
- focused dashboard gateway Rust tests

Work:

1. Implement the bounded 32-key single-flight state machine and delete
   `repair_dashboard_service_status_response`,
   `repair_dashboard_service_status_value`, and the duplicate dashboard
   Guacamole URL selector after canonical parity passes.
2. Keep backend error normalization from P74 unchanged.
3. Keep the status-specific timeout and completed-response coalescing cache.
4. Cache only successful 2xx bytes by backend identity and complete path,
   including query parameters such as `full-tab-history=true`, for five seconds
   from completion.
5. Share concurrent failures without caching them, preserve an independently
   owned request when one waiter cancels, and clean late or panicked flights by
   request id.
6. Preserve ten seconds separately for connect, write, and read rather than
   introducing an end-to-end deadline.
7. Ensure no backend I/O, observation, projection, or waiter await runs while
   the state-map mutex is held.

Acceptance:

- `dashboard.rs` no longer navigates or mutates
  `/data/service_state/browsers`.
- Dashboard-proxied and direct status responses have the same canonical data.
- Concurrent dashboard polls coalesce into one backend status request.
- Connect, write, and read each retain their ten-second phase bound without
  changing other gateway timeouts.

### Slice 5: Migrate Contracts and Consumers

Files:

- `docs/dev/contracts/service-status-response.v1.schema.json`
- `docs/dev/contracts/README.md`
- `scripts/generate-service-observability-client.js`
- generated `packages/client/src/service-observability.generated.d.ts`
- `packages/client/src/service-observability.js`
- dashboard status type and projection consumers
- cross-seam and generated-client tests

Work:

1. Add the projection metadata and typed observation contract additively.
2. Generate client types and add helper coverage.
3. Update only P0099's landed public raw-status input adapter and its
   observation-consumption tests. Do not recreate workspace selection,
   readiness, viewport, tile, or Service inspector projection.
4. Remove repeated partial dashboard status declarations only where P0099's
   public adapter makes them obsolete.
5. Keep old-server compatibility isolated in that adapter. Unknown newer
   schema versions fail closed for observation-dependent affordances.
6. Retain every legacy v1 field and compatibility mirror for the v1 lifetime.

Acceptance:

- Generated client parity passes.
- Removing projection metadata or changing its source and freshness fields
  breaks a focused contract test.
- Dashboard code cannot treat a host observation as a service-owned inventory
  or proof decision.
- Older payloads remain readable without synthesizing route-bound authority.

### Slice 6: Documentation and Operational Proof

Files:

- `cli/src/output.rs`
- `README.md`
- `skills/agent-browser/SKILL.md`
- relevant `docs/src/app/` service and dashboard MDX pages
- inline Rust documentation
- P45 or a companion execution note with completion evidence

Work:

1. Document the distinction between Service State authority, response
   projection, and host observations.
2. Document observation availability and freshness for software clients and
   agents.
3. Document that legacy v1 mirrors remain for the v1 lifetime and may be
   removed only in a separately versioned contract plan.
4. Run one isolated live readback that compares the full status data from CLI,
   direct HTTP, and dashboard-proxied HTTP, then separately compares shared
   authoritative records from one narrower MCP resource. Do not characterize
   the MCP read as full-status parity or navigate to a private target site.
5. Record binary path, version, installed unit identity, response schema
   version, observation source host, and exact readback hashes.

Acceptance:

- User-facing documentation, generated types, source, installed runtime, and
  readbacks agree.
- No status consumer invents a browser, route, URL, display proof, inventory
  class, or actionability decision.
- P45 remains the owner of remote-view proof and canonical inventory.

## Test Plan

### Rust interface tests

- bounded and full closed-tab projections preserve input Service State;
- action and control-plane inputs produce identical canonical output;
- explicit stream URLs outrank configured compatibility URLs;
- a configured client URL fills only missing eligible fields;
- a Guacamole root is preserved without a fabricated client fragment;
- non-RDP and non-remote-headed streams are unchanged;
- display observations carry source, scope, observed time, and freshness;
- unavailable is distinct from negative observation;
- a present Browser Session Authority `non_viable` verdict remains a typed
  actionability gate, while unavailable raw process evidence produces no
  negative verdict;
- unavailable manual-browser observation returns legacy `manualBrowsers: []`
  plus typed unknown provenance;
- a timed-out display probe yields a typed observation error;
- concurrent probes for one display coalesce without a lock around the blocking
  command;
- status cache key includes full path and query;
- connect, write, and read each have an independent ten-second phase timeout;
- one independently owned backend request serves all current waiters;
- waiter cancellation does not cancel the backend request;
- failures are shared and never cached, while successful 2xx bytes cache five
  seconds from completion;
- late flight completion cannot overwrite a newer request id;
- the 32-key cap evicts only ready entries and uses uncached overflow when all
  entries are in flight;
- projection never persists or mutates its input.

### Contract and consumer tests

- service-status JSON Schema accepts an old server with omitted
  `statusProjection`; a current server fixture requires the complete typed
  projection and every enum, required null, timestamp, and source-host field;
- generated service-observability types contain the new projection contract;
- the client helper preserves observation metadata;
- the P0099 authority adapter ignores raw observations for ownership and
  actionability but preserves a present Browser Session Authority verdict;
- viewport presentation may use a fresh URL observation without treating it as
  route proof;
- consumers current-derive staleness from `validUntil`, including a nearly
  ten-second-old cached fixture;
- selected-context evidence labels host observations separately from service
  authority;
- unknown projection versions fail closed only for observation-dependent
  affordances;
- transport fixtures cover CLI, action, control-plane, direct HTTP, dashboard
  backend, dashboard CLI fallback, generated client, all four dashboard status
  consumers, MCP full-status rejection, and narrower MCP resources;
- cross-seam interlocks preserve one authority answer across HTTP, client, and
  P0099 dashboard consumers.

### Suggested validation commands

```bash
cargo test --manifest-path cli/Cargo.toml service_status_projection
cargo test --manifest-path cli/Cargo.toml service_status_response_combines_worker_and_service_state
cargo test --manifest-path cli/Cargo.toml dashboard_service_status
pnpm test:service-status-no-launch
pnpm test:service-contracts-no-launch
pnpm test:mcp-read-no-launch
pnpm test:service-client
pnpm test:cross-seam-interlocks
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-view-streams
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml
pnpm validation:select -- --base ae36b272327982e3227f4dc7c5d6dc5b4b16350c
```

The frozen validation base for P100 is
`ae36b272327982e3227f4dc7c5d6dc5b4b16350c`, the `origin/main` read at Cycle
1 remediation time. If integration rebases or merges that base before P100
execution, record the new integration commit as an explicit replacement in
this plan before running the selector; do not silently use a moving ref.

Execution integration base replacement on 2026-08-10:

`0528e5db48dc26f630b1d1994bfc2fbf9f0f76bb`, the accepted and committed
Candidate 2 workspace-view projection checkpoint. Candidate 3 validation and
content identity use this fixed base so Candidate 1 and Candidate 2 changes are
not misattributed.

Run the isolated installed-runtime readback only after no-launch and canonical
Rust gates pass. Do not turn ordinary implementation closeout into CI
babysitting.

## Risks and Controls

- **Blocking host I/O:** the local adapter owns bounded blocking tasks, child
  cleanup, and probe cleanup; the gateway state mutex never encloses backend or
  projection work.
- **Host-local evidence masquerading as authority:** require typed source,
  scope, availability, and freshness; raw observations cannot drive authority,
  while Browser Session Authority remains an explicit typed preprojection
  input.
- **Multi-host drift:** use typed unavailable when local capability is missing.
  Cross-host observation transport is deferred to a separate plan.
- **Compatibility leakage:** keep all legacy mirroring in one module and retain
  every v1 field for the v1 lifetime.
- **Cache contamination:** key canonical status cache by backend and complete
  path, and keep host observation cache keyed by source host plus display.
- **P45 scope collision:** serialize proof and inventory outputs, but do not
  recreate their decisions inside projection.
- **Installed runtime lag:** bind live proof to executable path, version,
  service unit, schema version, and response hash before deleting the duplicate
  gateway repair.

## Evidence Still Requiring Execution Proof

- Source and unit fixtures show that the dashboard user unit and its spawned
  backend normally inherit the same environment file, but current live process
  locality, X11 access, and environment freshness were not mutated or proven
  during this plan-only analysis.
- Source proves that the action and control-plane assembly paths differ. It
  does not by itself prove which historical incident first motivated the
  dashboard repair, so implementation must preserve the frozen behavior rather
  than infer that rationale.

## Bounded Review Protocol

- Cycle 1 was one broad `drift_discovery` review and produced stable findings
  `P0100-A1-01` through `P0100-A1-08`.
- This version is the one consolidated remediation pass.
- Cycle 2 is `closed_world`. It may inspect only those eight accepted findings
  and critical contradictions introduced by their remediation.
- There is no Cycle 3. A remaining blocking failure after Cycle 2 splits or
  blocks the implementation unit. Nonblocking concerns are recorded once in
  backlog and do not reopen architecture discovery.
- The implementation audit follows the same bounded rule with stable finding
  identifiers, one remediation pass, and one closed-world verification.

## Cycle 1 Adjudication

| Finding | Disposition | Version 2 remediation |
| --- | --- | --- |
| `P0100-A1-01` | accepted, remediated | Browser Session Authority is typed preprojection authority; field-role ledger freezes unavailable behavior |
| `P0100-A1-02` | accepted, remediated | One async projector operation, typed observation source, cancellation ownership, and fatal versus partial error mapping are frozen |
| `P0100-A1-03` | accepted, remediated | Bounded 32-key single-flight state machine and per-phase timeout semantics are frozen |
| `P0100-A1-04` | accepted, remediated | Exact additive JSON, enums, nulls, timestamps, source host, freshness, permanent v1 legacy fields, and compatibility matrix are frozen |
| `P0100-A1-05` | accepted, remediated | P100 selects in-process local or unavailable only; cross-host transport is deferred |
| `P0100-A1-06` | accepted, remediated | P0099 then P0100 then P0101 order and write ownership are frozen |
| `P0100-A1-07` | accepted, remediated | Full transport ledger names every producer, consumer, fallback, envelope, MCP nonproducer, and parity assertion |
| `P0100-A1-08` | accepted, remediated | Two-cycle protocol, full Rust suite, frozen-base selector, and exact no-launch status, contract, and MCP smokes are required |

## Non-Goals

- Do not redesign Guacamole, RDP, or route allocation.
- Do not implement P45's route-bound open coordinator, proof state machine, or
  canonical inventory in this plan.
- Do not persist host-local observations during a status read.
- Do not add a new dashboard-only source of browser truth.
- Do not add a cross-host observation transport, new observation port, new CLI
  flag, or new environment variable in P100.
- Do not add a full-status MCP resource or tool. Existing MCP collection
  resources retain their narrower Service State contracts.
- Do not broaden live validation to private target sites or tenant data.

## Completion Criteria

P100 is complete only when all of the following are proven from current state:

1. Action and control-plane status use one deep typed projection module.
2. CLI, action, control-plane, HTTP, dashboard fallback, generated-client, and
   P0099 dashboard consumers receive one canonical authority contract; MCP
   full-status rejection and narrower resource contracts remain explicit.
3. Host-local enrichment crosses a typed local-substitutable adapter seam and
   carries provenance and freshness.
4. The dashboard gateway no longer parses and repairs arbitrary status JSON.
5. Existing timeout and cache protections remain covered by focused tests.
6. Projection cannot mutate persisted Service State or remote-view authority.
7. All legacy v1 fields remain supported for the v1 lifetime; only duplicate
   gateway repair logic is deleted.
8. Contract schema, generated types, dashboard types, docs, source, and
   installed-runtime readbacks agree.
9. Focused Rust, exact no-launch status, contract and MCP smokes, client,
   cross-seam, dashboard, format, strict Clippy, full Rust, and frozen-base
   validation-selector gates pass.
10. Cycle 2 closed-world plan audit and the bounded independent implementation
    audit find no blocking violation, with residual nonblocking findings
    recorded once rather than reopening review discovery.
