# Plan 0100 Cycle 1 Work Audit: Service Status Projection Authority

Date: 2026-08-10
Review mode: `drift_discovery`
Review cycle: 1 of at most 2
Fixed integration base: `0528e5db48dc26f630b1d1994bfc2fbf9f0f76bb`
Reviewed implementation paths: 29
Reviewed implementation content identity: `afb651b7339ad5c950d63e2ca28d4be1b1cfacbe4fda0e076469eab5947723e6`
Reviewer runtime: `/root/audit_work_status_projection`
Implementation, plan, test, runtime, commit, and push mutation: none

## Verdict

Accepted for final testing: **No**.

The implementation establishes the intended deep seam: action and control-plane
status call one async `ServiceStatusProjector`, host reads cross an observation
adapter, legacy mirroring is concentrated, P99 consumes observation metadata
without changing its authority ledger, and the old shallow projector and
dashboard status-repair functions are deleted. Focused tests, format, strict
Clippy, dashboard and docs builds, and no-launch smokes pass.

Nine blocking findings remain against the frozen Plan 0100 contract. They are
not style complaints. They cover fatal input behavior, persistence failure
propagation, unknown and partial classification, observation freshness and
error variants, pure dashboard transport, single-flight cancellation cleanup,
old-server client compatibility, and the required executable producer and MCP
nonproducer ledger. One consolidated bounded remediation is prescribed below.

## Scope and Evidence

The 29-path manifest is the sorted current P100 implementation set relative to
the fixed base. It includes the 23 modified tracked implementation and
documentation paths, five new Rust projection files, and the P100 execution
note. It excludes Plans 0100 through 0102 and their plan-audit notes. The
content identity is the SHA-256 of the sorted per-file SHA-256 stream.

CodeGraph was current with 426 files, 14,603 nodes, and 44,468 edges.
`actions.rs` remained intentionally skipped at 1,463,053 bytes and was read
directly. Graphiti group `agent_browser_main` was healthy but returned no
source-backed decision for this new seam, so the plan, current source, and
tests controlled the audit.

Deletion proof passed for:

- `project_service_status`
- `refresh_remote_view_stream_urls`
- `repair_dashboard_service_status_response`
- `repair_dashboard_service_status_value`
- `dashboard_guacamole_client_url`

The retained `manual_runtime_browser_projection` has a current Chrome
diagnostic caller and is not a duplicate canonical status producer.

## Finding Ledger

### P0100-W1-01 — `blocking` — The projector input and observation vocabulary are only partially typed and required inputs are not validated

Criterion: the deep module has one typed async interface; invalid authoritative
input and required configuration fail the status read before observation; the
wire enum vocabulary cannot drift through arbitrary strings.

Evidence:

- `StatusAuthorityInput` keeps `control_plane` and `launch_config` as arbitrary
  `serde_json::Value` (`cli/src/native/service_status_projection.rs:47-54`).
- `validate_authority` checks only browser map keys and IDs
  (`cli/src/native/service_status_projection/authority.rs:6-16`).
- `StatusObservationSnapshot`, errors, sources, aggregate states, child states,
  and route-presentation sources are represented as `String`, not typed enums
  (`cli/src/native/service_status_projection/observation.rs:23-69`).
- The projector accepts those values and serializes a successful response
  without validating the control-plane snapshot, launch configuration, enum
  combinations, timestamps, or null invariants
  (`cli/src/native/service_status_projection.rs:169-217`).

Consequence: malformed required configuration or control-plane authority can
cross the canonical interface as a successful current-server response, and an
adapter can compile while emitting a state or source rejected by the v1 schema.
The interface therefore does not concentrate the invariant knowledge promised
by the plan.

Reproducer: construct `StatusAuthorityInput` with a valid empty `ServiceState`
but `control_plane: null` or `launch_config: "invalid"`, or return an in-memory
snapshot with `state: "fresh"`. The projector reaches serialization instead of
returning a typed fatal error.

Confidence: high.

Disposition: blocking. Replace arbitrary authority values and wire-state
strings with validated typed records and enums, or add an explicit validation
stage that enforces the complete current-server contract before observation
and serialization.

### P0100-W1-02 — `blocking` — Control-plane status still hides repository persistence failures

Criterion: repository and permitted preprojection persistence failures use the
entry adapter's existing failure envelope; action and control-plane status do
not diverge on fatal authority preparation.

Evidence:

- The action path opens the repository and propagates reconciliation
  persistence errors with `?` (`cli/src/native/actions.rs:17586-17589`).
- The control-plane helper silently ignores both repository construction and
  mutation errors (`cli/src/native/control_plane.rs:431-436`).
- `service_status_response` calls that helper and continues to the projector and
  a success envelope (`cli/src/native/control_plane.rs:192-225`).

Consequence: identical authoritative state can fail through the action path
while succeeding through the control-plane, direct HTTP, or dashboard backend
path after reconciliation was not persisted. That violates fatal-error parity
and makes persistence ownership depend on ingress.

Reproducer: make the default Service State repository path unwritable and call
action status and `ControlPlaneHandle::service_status_response` with equivalent
state. The action returns an error; the control-plane path discards the same
repository failure and can return `success: true`.

Confidence: high.

Disposition: blocking. Make the control-plane persistence helper return
`Result`, propagate it through the existing `success: false` envelope, and add
the paired failure test.

### P0100-W1-03 — `blocking` — Partial process evidence can still classify unknown browsers and observations as complete

Criterion: unavailable is unknown; partial Browser Session Authority emits
verdicts only for browsers with required evidence; modeled, verdict, and unknown
counts reconcile; partial raw process observation is not reported as observed.

Evidence:

- Browser Session Authority switches to all-unknown only when the resource list
  is empty and collection warnings are nonempty
  (`cli/src/native/browser_session_authority.rs:66-75`).
- With any resource plus a warning, it maps every modeled browser to a verdict,
  hardcodes `unknown_browser_count: 0`, and labels availability `partial`
  (`cli/src/native/browser_session_authority.rs:76-144`).
- The local observation adapter reports `browserProcessState: "observed"` when
  any modeled browser produced stats, even when other modeled browsers did not
  (`cli/src/native/service_status_projection/local_observation.rs:107-124`).

Consequence: missing per-browser process evidence can produce a `viable`,
`attention`, or `non_viable` authority verdict and can present a partial host
snapshot as fully observed. P99 consumes present nonviable verdicts as an
actionability gate, so this is an authority error rather than a diagnostic-only
label.

Reproducer: create two modeled browsers and a `ResourceAuthoritySnapshot` with
one correlated resource plus one collection warning. The result is
`availability: "partial"`, two verdicts, and `unknownBrowserCount: 0`. Likewise,
make one of two PIDs readable in the local adapter; the aggregate process state
is `observed`, not `partial`.

Confidence: high.

Disposition: blocking. Track evidence availability per modeled browser, emit
only supported verdicts, reconcile the summary equation, and implement the
declared partial process-observation state with focused crossed tests.

### P0100-W1-04 — `blocking` — Display-cache hits are restamped as new observations

Criterion: `observedAt` is the actual observation completion time;
`validUntil` is that time plus `maxAgeMs`; a cached observation can arrive stale
and consumers derive that staleness without the producer extending it.

Evidence:

- `local_snapshot` records one timestamp before manual, process, and display
  work and assigns that value to every successful stream
  (`cli/src/native/service_status_projection/local_observation.rs:83-92`,
  `:181-190`, `:217-230`).
- The display cache stores an internal `Instant` but returns only the content;
  callers cannot recover the original observation time
  (`cli/src/native/remote_view.rs:1340-1351`, `:1392-1407`).
- The dashboard correctly treats `validUntil` as authoritative freshness
  metadata (`packages/dashboard/src/lib/workspace-view-projection.ts:154-188`).

Consequence: a display result collected near the start of the five-second probe
cache can be served again near cache expiry and receive a new five-second
validity window. The gateway can then cache that response for another five
seconds. Consumers may treat evidence as current well after the frozen source
observation expired.

Reproducer: populate the display cache, wait until just before its five-second
TTL, then project status again for the same source host and display. The second
response contains the same cached content but an `observedAt` and `validUntil`
based on the second request.

Confidence: high.

Disposition: blocking. Return typed display observation metadata from the
cache, preserve the original completion time and expiry, and derive the
top-level earliest expiry from actual child expiries.

### P0100-W1-05 — `blocking` — Display timeout, unsupported, and failure variants collapse to unavailable and violate the null rules

Criterion: timed out, unsupported, unavailable, failed, partial, and observed
wire cases are distinct; unavailable child values are null; each declared enum
and error code is executable through the production adapter.

Evidence:

- A missing `xwininfo`, timeout exit, other nonzero exit, and probe panic all
  become `display_probe_unavailable` content
  (`cli/src/native/remote_view.rs:1377-1388`, `:1525-1562`).
- The local adapter recognizes only that one content state and emits only
  `display_probe_unavailable`; its stream state is only `observed` or
  `unavailable` (`cli/src/native/service_status_projection/local_observation.rs:145-190`).
- When route presentation succeeds but display probing fails, the stream is
  marked `observed` and the unavailable error object remains non-null in
  `displayContent` (`:158-189`).
- The schema and generated client advertise `timed_out`, `unsupported`,
  `failed`, and their corresponding error codes
  (`docs/dev/contracts/service-status-response.v1.schema.json:369-420`,
  `packages/client/src/service-observability.generated.d.ts:646-667`).

Consequence: software clients cannot distinguish timeout, unsupported host, and
other failure even though the v1 interface promises that distinction. Mixed
route success can also mirror an unavailable display object into legacy
`displayContent` while reporting the stream observed.

Reproducer: run the production adapter with `xwininfo` absent, with a probe that
exits through the two-second timeout, and with a non-timeout command failure.
Every case serializes the same unavailable state and code; none reaches the
other advertised variants.

Confidence: high.

Disposition: blocking. Introduce a typed probe result that preserves exit
classification and timestamps, map every frozen variant, and enforce null
observation values for failed children.

### P0100-W1-06 — `blocking` — The status gateway still parses successful JSON and its byte-forwarding test stops below the public handler

Criterion: the dashboard status gateway is a pure transport adapter; successful
2xx bytes are forwarded unchanged and are not parsed or validated by the
gateway.

Evidence:

- `handle_service_api_request` sends every proxied response through
  `require_json_backend_response` before writing it
  (`cli/src/native/stream/dashboard.rs:522-548`).
- That helper locates the body and parses it as `serde_json::Value`, replacing a
  successful backend response with a gateway error when parsing fails
  (`cli/src/native/stream/dashboard.rs:1014-1047`).
- The byte-for-byte test calls `proxy_dashboard_service_api_request` directly
  and never crosses the public handler or JSON validator
  (`cli/src/native/stream/dashboard.rs:2273-2305`).

Consequence: dashboard-proxied status still has a response interpretation step
that direct HTTP does not. A successful backend body that is not accepted by
the gateway becomes a synthesized 502 rather than the original response, so
the tested function is not the full transport path.

Reproducer: return a syntactically valid HTTP 200 response with a non-JSON body
from the selected status backend. The single-flight byte test passes at the
proxy seam, while `handle_service_api_request` rejects the same bytes.

Confidence: high.

Disposition: blocking. Make status bypass response JSON interpretation and add
a handler-level byte-identity fixture. Other gateway paths may retain their
existing validation where separately owned.

### P0100-W1-07 — `blocking` — A cancelled owned status flight leaves a dead in-flight cache entry

Criterion: task panic or cancellation publishes the typed failure and removes
only its own request ID; a late completion cannot overwrite a newer flight; the
32-key overflow and independent phase bounds have executable coverage.

Evidence:

- The spawned task handle is discarded, and cache removal occurs only after the
  backend future returns or its panic is caught
  (`cli/src/native/stream/dashboard.rs:874-925`).
- If that task is cancelled, dropping the watch sender wakes current waiters,
  but no guard removes the `InFlight` entry. The cache itself retains a receiver
  for that closed channel (`:54-63`, `:833-845`).
- The five focused tests cover cacheability, success coalescing, waiter
  cancellation, shared non-2xx failure, and ready-entry eviction. They do not
  cancel the owned task, fill all 32 slots with in-flight entries, exercise
  uncached overflow, replace a request ID, or prove the connect, write, and read
  timeout phases (`:2253-2450`).

Consequence: after owned-task cancellation, future callers for that key keep
joining a dead flight and immediately receive backend-unavailable; the entry
also permanently consumes one of 32 slots. The late-result protection and
all-in-flight bound are not proven through reachable state transitions.

Reproducer: expose or inject the owned backend task, abort it after registration,
then issue a second request for the same backend and path. The map still holds
the closed `InFlight` receiver rather than returning to vacant state.

Confidence: high.

Disposition: blocking. Add request-ID-scoped drop cleanup around the owned task
and complete the exact state-machine test matrix, including task cancellation,
all-in-flight overflow, late completion, success TTL from completion, and the
three established timeout phases.

### P0100-W1-08 — `blocking` — Generated current-client types reject an older valid v1 Browser Session Authority payload

Criterion: old v1 server to current client remains supported; new current-server
fields are additive without making fields absent on old servers compile-time
required.

Evidence:

- The JSON Schema does not require `browserSessionAuthority.availability` or
  `summary.unknownBrowserCount`, which permits the older v1 shape
  (`docs/dev/contracts/service-status-response.v1.schema.json:146-201`).
- The generated TypeScript makes both new fields required
  (`packages/client/src/service-observability.generated.d.ts:604-644`).
- The client test exercises only a current fixture containing both fields
  (`scripts/test-service-observability-client.js:104-178`).

Consequence: runtime compatibility is permissive while the published current
client interface says an older valid payload is impossible. TypeScript callers
cannot honestly model the old-server case required by the compatibility matrix.

Reproducer: type an old v1 status fixture whose Browser Session Authority has
the pre-P100 summary and no `availability`. Runtime `getServiceStatus` returns
it, but assignment to the generated `ServiceStatusResponse` fails type checking.

Confidence: high.

Disposition: blocking. Model new Browser Session Authority fields as optional
for v1 compatibility, or introduce a version-discriminated union that accepts
the old shape, and add old-server compile and runtime fixtures.

### P0100-W1-09 — `blocking` — The frozen producer and MCP nonproducer ledger is not executable coverage

Criterion: action, control plane, direct HTTP, dashboard backend, dashboard CLI
fallback, generated client, and all four P99 consumers are bound to the same
canonical data contract; MCP lists are exhaustively proven not to advertise or
return full Service Status.

Evidence:

- The no-launch status smoke calls only CLI status and checks no-launch state;
  it does not compare any other producer or require `statusProjection`
  (`scripts/smoke-service-status-no-launch.js:30-58`).
- The MCP no-launch smoke reads sessions and four profile resources. It never
  calls `resources/list`, `resources/templates/list`, or `tools/list`
  (`scripts/smoke-mcp-read-no-launch.js:89-225`).
- Existing MCP Rust tests check the static resource list, six templates, one
  broad tools list, and explicit `browser_command` rejection, but do not assert
  that the entire advertised tool schemas and resource payloads are full-status
  nonproducers (`cli/src/mcp.rs:10103-10148`, `:10232-10270`,
  `:11217-11227`).
- The cross-seam test uses a hand-authored status fixture rather than any real
  producer (`scripts/test-cross-seam-interlocks.js:58-165`).
- The Rust response contract assertion does not inspect `statusProjection`
  (`cli/src/native/service_model.rs:2032-2112`).

Consequence: the named required gates can all remain green if one real ingress
omits projection metadata, one fallback diverges, or a newly advertised MCP
surface becomes a full-status producer. The plan's complete ledger exists only
as prose, not as a replace-not-layer regression gate.

Reproducer: remove `statusProjection` from one entry adapter, add a status-like
MCP tool schema, or change dashboard fallback data while leaving the synthetic
fixture unchanged. The reproduced no-launch status, MCP read, and cross-seam
commands do not exercise that divergence.

Confidence: high.

Disposition: blocking. Add one no-launch transport fixture with fixed authority,
clock, and observations that deep-compares every real status producer and
fallback at the data seam. Extend the MCP stdio smoke to assert the complete
resource, template, and tool inventories, generic-action rejection, and absence
of `statusProjection` or full-status envelopes from every narrower resource.

## Consolidated Bounded Remediation Before Cycle 2

Perform one remediation pass only:

1. Make the projector's authority and observation vocabulary typed and validate
   every required current-server invariant before serialization.
2. Propagate control-plane repository and persistence failures through its
   existing failure envelope and add paired action and control-plane failure
   coverage.
3. Implement per-browser unknown and partial process semantics for Browser
   Session Authority and local observations.
4. Replace unstructured display content collection with a typed cached result
   carrying the original completion time, expiry, and timeout, unsupported,
   unavailable, or failure classification.
5. Remove status JSON interpretation from the dashboard gateway, add
   request-ID-scoped cancellation cleanup, and finish the exact 32-key
   single-flight state-machine tests.
6. Restore old-server compatibility in generated types and add the old and
   current server-client matrix fixtures.
7. Add the real producer-parity and exhaustive MCP nonproducer fixture, then
   make current-server response contract tests require the complete projection.

Cycle 2 must be closed-world over `P0100-W1-01` through `P0100-W1-09` plus
critical regressions introduced by their remediation. It must not reopen broad
architecture discovery. There is no Cycle 3.

## Reproduced Validation

Passed:

- `cargo test --manifest-path cli/Cargo.toml service_status_projection`:
  6 passed
- `cargo test --manifest-path cli/Cargo.toml service_status_response_combines_worker_and_service_state`:
  1 passed
- `cargo test --manifest-path cli/Cargo.toml dashboard_service_status`:
  5 passed
- `pnpm test:service-status-no-launch`
- `pnpm test:service-contracts-no-launch`
- `pnpm test:mcp-read-no-launch`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`: 66 browser controls, 26 service tools,
  19 resources, 62 native service actions, and 96 request actions
- `pnpm test:cross-seam-interlocks`
- `pnpm test:dashboard-workspace-view-projection`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:route-confusion-gates`
- `pnpm build:dashboard`
- `pnpm --dir docs build`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base 0528e5db48dc26f630b1d1994bfc2fbf9f0f76bb`

The selector also recommended live, workstation, installed-dashboard publish,
and broader dashboard gates because `remote_view.rs` and dashboard files are in
the slice. Those were not run: this audit's authority explicitly forbids live,
install, browser, and runtime mutation. The execution note accurately records
installed-runtime convergence as unclaimed.

The full Rust suite was not rerun by this auditor. The executor reported 1,763
passes, 57 ignored, and one unchanged DISPLAY-sensitive failure, but that report
was accepted only as delegated evidence and was not needed to establish the
blocking findings above.

## Audit Effects

This audit wrote only this note. It did not edit implementation, plans, tests,
contracts, generated files, installed binaries, user services, browsers,
routes, displays, tenant state, commits, or remotes.

## Cycle 2 Closed-World Verification

Review mode: `closed_world`
Review cycle: 2 of 2
Supplied Candidate 3 identity: `d9cb5fc68416a325717bd77f36f43b3d5f6ff332bc23b49bea6267f4ea4ad460`
Reviewed Candidate 3 identity: `9a876b886a264086032f7972fc509575bf409bdcc3794b1ba6723ddfcbc6f65d`
Reviewed implementation paths: 33

The reviewed identity is the independently recomputed SHA-256 of the sorted
per-file SHA-256 stream for the 33-path manifest in the execution note. The
supplied identity did not match current bytes because that identity was
computed before the included execution note was updated. No concurrent
implementation writer was expected. This is a nonblocking receipt and
provenance correction; Cycle 2 is bound to the independently reproduced
current identity above.

### Cycle 2 verdict

Ready for distinct final testing: **No**.

Six findings pass closed-world verification. Three remain blocking: the launch
configuration still does not enforce its required typed invariant, the old-v1
compatibility fix lacks the required generated-type compile fixture, and the
producer plus MCP nonproducer ledger remains non-exhaustive. No unrelated drift
or new critical contradiction was found.

| Finding | Cycle 2 disposition | Evidence |
| --- | --- | --- |
| `P0100-W1-01` | **FAIL, blocking** | Typed control-plane and observation vocabulary landed, but launch configuration remains an unchecked arbitrary object |
| `P0100-W1-02` | PASS | Action and control-plane repository failures now converge and the paired failure test passes |
| `P0100-W1-03` | PASS | Partial resource and process evidence now preserves per-browser unknown and partial states with reconciled counts |
| `P0100-W1-04` | PASS | Display cache entries retain the original typed observation and its completion-derived timestamps |
| `P0100-W1-05` | PASS | Timeout, unsupported, unavailable, and failed display outcomes remain distinct and non-observed values are null |
| `P0100-W1-06` | PASS | Status bypasses gateway JSON interpretation and the handler-level non-JSON byte identity test passes |
| `P0100-W1-07` | PASS | Request-ID cleanup, owner cancellation, 32-key overflow, late completion, completion TTL, and independent timeout phases are implemented and pass |
| `P0100-W1-08` | **FAIL, blocking coverage** | Generated fields are optional and the runtime old-v1 fixture passes, but there is no old-v1 TypeScript assignment fixture in the compiled type gate |
| `P0100-W1-09` | **FAIL, blocking coverage** | The fixed projector/helper parity test and expanded MCP resource scan pass, but real producer/fallback coverage and the MCP tool nonproducer inventory are not exhaustive |

### `P0100-W1-01` Cycle 2 FAIL — launch configuration is still not a typed required-input contract

Criterion: the projector interface rejects invalid authoritative input and all
required launch configuration before observation or serialization.

Evidence:

- `StatusLaunchConfiguration` is a transparent `Map<String, Value>` and its
  conversion checks only that the value is an object
  (`cli/src/native/service_status_projection.rs:140-154`).
- The v1 schema requires nine launch fields with field-specific types
  (`docs/dev/contracts/service-status-response.v1.schema.json:203-216`).
- The projector test input explicitly constructs the launch configuration from
  `{}` and every projector and parity test succeeds with it
  (`cli/src/native/service_status_projection/tests.rs:69-97`).

Consequence: callers can cross the deep module's interface with an empty or
wrongly typed launch record and receive a successful current-server response
that violates the published v1 schema. The interface still leaves required
configuration knowledge outside the projection module.

Reproducer: `cargo test --manifest-path cli/Cargo.toml
service_status_projection` passes 12 tests while the shared input at line 83
uses `StatusLaunchConfiguration::try_from(json!({})).unwrap()`. The same object
is invalid under the schema's required list.

Confidence: high.

Disposition: blocking. The remediation did not close `P0100-W1-01`; with no
Cycle 3, this implementation unit must be blocked or split.

### `P0100-W1-08` Cycle 2 FAIL — old-v1 runtime compatibility is not protected by a generated-type compile fixture

Criterion: an old valid v1 response must be accepted by the current generated
TypeScript interface, and a compiled fixture must prevent the additive fields
from becoming required again.

Evidence:

- The generated interface correctly makes `availability` and
  `unknownBrowserCount` optional
  (`packages/client/src/service-observability.generated.d.ts:604-644`).
- The old-v1 fixture is JavaScript runtime data only
  (`scripts/test-service-observability-client.js:180-218`).
- The compiled `test:service-client-types` inputs do not include that test or
  another old-v1 typed assignment fixture (`package.json:67-72`).

Consequence: the current product type is compatible, but the required
replace-not-layer regression gate is absent. Making both fields required in the
generator and regenerated declaration would keep generator parity green, and
the runtime fixture would still pass because it is not compiled against
`ServiceStatusResponse`.

Reproducer: inspect the `test:service-client-types` file list and the old-v1
fixture, then run `pnpm test:service-client`. The gate passes without compiling
an old-v1 object against the generated status type.

Confidence: high.

Disposition: blocking coverage. The runtime incompatibility is repaired, but
the accepted Cycle 1 disposition required both compile and runtime fixtures.

### `P0100-W1-09` Cycle 2 FAIL — producer and MCP nonproducer coverage remains non-exhaustive

Criterion: fixed-input coverage deep-compares every real status producer and
fallback, and the complete MCP inventory proves that no advertised surface can
produce full Service Status.

Evidence:

- The new parity test calls the projector directly and then exercises test-only
  envelope, HTTP formatting, handler, and fallback-formatting helpers. It does
  not invoke the action entry, control-plane status method, direct HTTP route,
  or actual CLI fallback (`cli/src/native/service_status_projection/tests.rs:357-383`).
- The CLI no-launch smoke still checks success, browser health, and persisted
  no-launch state but never requires or schema-validates `statusProjection`
  (`scripts/smoke-service-status-no-launch.js:30-56`).
- The MCP smoke lists all tools but checks only for the literal
  `service_status`; it does not compare the tool names to a frozen allowlist or
  exercise their result classification
  (`scripts/smoke-mcp-read-no-launch.js:157-193`).
- The Rust tools-list test inspects selected tool schemas but does not assert
  the complete tool-name inventory (`cli/src/mcp.rs:10233-10270`).

Consequence: a real ingress can drift outside the fixed helper composition, or
a differently named MCP tool can become a full-status producer, while all
reproduced gates remain green. The nonproducer ledger is broader than Cycle 1
but is still not executable as an exhaustive replace-not-layer invariant.

Reproducer: add an advertised MCP tool named `status_snapshot` whose input
schema does not spell `statusProjection` and whose result returns full status.
The smoke neither matches the forbidden literal nor calls or rejects that tool.
Similarly, removing projection assertions from a real ingress is not caught by
the test-only helper composition.

Confidence: high.

Disposition: blocking coverage. The remediation does not close
`P0100-W1-09`; with no Cycle 3, split or block this implementation unit.

### Cycle 2 reproduced validation

Passed:

- projector and observation tests: 12 passed
- Browser Session Authority tests: 4 passed
- paired action and control-plane persistence-failure test: 1 passed
- typed display-cache freshness and terminal-state test: 1 passed
- dashboard status-cache matrix: 9 passed
- handler byte-identity, late-completion, and independent timeout-phase tests:
  1 passed each
- Service Status control-plane response test: 1 passed
- service model tests, serial: 32 passed
- `pnpm test:service-client`
- `pnpm test:cross-seam-interlocks`
- `pnpm test:mcp-read-no-launch`
- `pnpm test:service-status-no-launch`
- `pnpm test:service-contracts-no-launch`
- `pnpm test:service-api-mcp-parity`: 66 browser controls, 26 service tools,
  19 service resources, 62 native service actions, and 96 request actions
- dashboard workspace projection, view-stream, workspace-node, and selected
  workspace-context gates
- `pnpm test:route-confusion-gates`
- Rust format, strict Clippy, `git diff --check`, and the fixed-base validation
  selector

Live, install, browser, workstation, embedded-runtime publication, and broad
unrelated gates were not run under the explicit Cycle 2 authority.

### Final bounded disposition

Cycle 2 is complete and there is no Cycle 3. The current 33-path Candidate 3
unit is not ready for distinct final testing. Under the bounded review policy,
the remaining failures require the unit to be split, reframed, or blocked;
they do not authorize another remediation and evaluator loop in this audit.
