# Plan 0098 | Service Request Normalization Deepening

State: IMPLEMENTATION READY | CYCLE 2 RESIDUAL RESOLVED
Roadmap: Service control-plane contract hardening
Plan version: 2
Date: 2026-08-09
Review state: closed-world Cycle 2 complete; bounded orchestrator correction recorded

## Objective

Replace the duplicated HTTP and MCP service-request preparation paths with one
deep in-process normalization module. The module must validate a request, merge
action parameters, preserve caller and routing hints, apply service-owned route
selection, and return one normalized daemon command through a small interface.
HTTP and MCP remain transport adapters at the seam.

The change must preserve the current public service-request contract, command
shape, action set, request-id prefixes, relay selection, queue behavior, and
transport-specific error envelopes. It must also repair already-observed drift
where a canonical field is not accepted or forwarded consistently, or where an
active routing field exposed by both adapters is missing from the canonical
contract.

## Authority And Dependencies

Repository authority, in descending order for this packet:

1. `AGENTS.md` and relevant policy under `docs/dev/policies/`;
2. `docs/dev/contracts/service-request.v1.schema.json` for the public request
   shape;
3. `cli/src/native/service_contracts.rs` for the canonical action set and
   stable HTTP/MCP contract identifiers;
4. current HTTP and MCP behavior in `cli/src/native/stream/http.rs` and
   `cli/src/mcp.rs`;
5. generated client declarations and parity gates;
6. earlier plans and notes as historical rationale.

Relevant prior authorities:

- `docs/dev/notes/2026-04-22-agent-browser-service-roadmap.md` establishes the
  always-available, service-owned browser control plane;
- `docs/dev/notes/2026-05-05-service-roadmap-alignment-checkpoint.md` requires
  the same browser intent to mean the same thing across Rust, schema, HTTP,
  MCP, and generated clients;
- `docs/dev/notes/2026-05-09-access-plan-service-request-handoff.md` makes the
  queued request the handoff from service-owned access planning;
- `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md` and
  `docs/dev/plans/0034-2026-06-14-generic-browser-service-routines-plan.md`
  introduced the action-specific validation now duplicated at both ingress
  adapters;
- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
  makes shared-profile route selection authoritative for HTTP and MCP
  `tab_new` requests.

`CONTEXT.md`, `docs/adr/`, `docs/agents/codex-code-discovery.md`, and
`docs/agents/codex-stack.md` are not present in the current tree. This plan
therefore uses the existing domain terms from the service contract and roadmap:
service request, access plan, service tab handle, retained browser, daemon
session, route hint, and queued daemon command.

## Current-State Evidence

### Structural evidence

CodeGraph is healthy with 419 indexed files, 14,341 nodes, and 43,350 edges.
The index intentionally skips the oversized `cli/src/native/actions.rs`; that
file is not needed to prove the ingress duplication in this packet.

CodeGraph impact analysis reports:

- MCP `service_request_command_with_state` affects 37 symbols, including 32
  request-construction and validation tests plus the queued MCP call path;
- HTTP `service_request_command_with_state` affects 42 symbols, including the
  HTTP handler, relay selection, and 38 request-construction and validation
  tests;
- `apply_shared_profile_route_hints_for_service_request` affects 82 symbols
  through the two duplicated ingress paths and their tests.

### Duplicated implementation

Both adapters currently implement the same ordered work independently:

1. read and validate `action` against `SERVICE_REQUEST_ACTIONS`;
2. reject manual-seeding, monitor-freshness, CDP-free, CDP-attach, bounded
   evaluate, diagnostics, probe, tab-handle refresh, UI action, network
   capture, and file-transfer requests;
3. merge `params` while protecting `id` and `action`;
4. copy a large top-level field allowlist over the merged parameters;
5. construct caller trace fields;
6. apply retained remote-view handoff route hints;
7. apply shared-profile route hints from the access planner;
8. return a daemon command for queued relay.

The HTTP implementation spans `service_request_command_with_state` and eleven
local validation helpers in `cli/src/native/stream/http.rs`. The MCP
implementation repeats the same work in `cli/src/mcp.rs`, split between
`service_request_command_with_state`, eleven validation helpers,
`ServiceToolContext`, `browser_command_command`, and target-profile copying.

### Observed drift

The duplication has already produced material contract drift:

- HTTP validates `external_byop_adopt` and forwards `runtimeProfile` or
  `profileId` plus exactly one of `cdpUrl` and `cdpPort`. MCP has no matching
  validation, and its `service_request` input schema does not advertise
  `cdpUrl` or `cdpPort`, even though both fields are canonical schema fields.
- MCP advertises `accountId` and `accountIds` in its `service_request` tool
  schema, but `ServiceToolContext::from_arguments` does not read them, so the
  command and trace silently drop them. HTTP forwards both fields.
- `manualLoginLaunch` is canonical and both builders can forward it, but the
  MCP tool schema does not advertise it.
- `browserHost`, `viewStreamProvider`, and `controlInputProvider` are exposed,
  parsed, forwarded, traced, and consumed by both active ingress paths and
  daemon routing, but are absent from the canonical service-request schema.
- HTTP preserves the existing top-level `args` behavior and tests it, while
  `args` is absent from the canonical schema and MCP surface. MCP callers can
  carry it only inside the permissive `params` object.
- Equivalent invalid probe requests can return different text, for example
  `probe requires probe recipe object` in MCP versus `probe requires probe
  object` in HTTP. The validation rule is the same, but the duplicated
  formatting makes error compatibility harder to reason about.
- The existing parity script compares the action enum but does not compare
  role-specific structural, consumed-only, trace, routing, command, and
  transport-legacy field sets.

### Dependency classification

Request validation, merge precedence, trace construction, and command
normalization are in-process dependencies. The only state dependency is a
read-only `ServiceState` snapshot supplied by the adapter for route-hint
selection. No I/O is required behind the new interface, and no injected port
or mock adapter is justified.

HTTP and MCP are already two real transport adapters at the external seam.
Their parsing, request-id prefix, error envelope, relay-session selection, and
queue transport legitimately vary. Everything between parsed JSON and the
normalized command should have one implementation.

## Architecture Diagnosis

The current HTTP and MCP request builders are shallow modules. Their interfaces
force both callers and tests to understand the action allowlist, every
action-specific rule, reserved-field precedence, caller metadata, profile and
target hints, route-hint ordering, and daemon command shape. The implementation
is nearly the same knowledge written twice.

Deletion test:

- deleting either current builder mostly moves its logic into its transport
  file because the other builder cannot be reused without transport-specific
  error and context assumptions;
- deleting the proposed normalization module would force both adapters to
  recreate the full validation, merge, trace, and route-hint sequence.

The proposed module therefore earns depth. Its small interface gives both
adapters leverage, while the validation and command-shape knowledge gains
locality. The interface also becomes the test surface for request semantics.

## Target Module And Seam

Add `cli/src/native/service_request.rs` and register it in
`cli/src/native/mod.rs`.

The module owns:

- canonical action membership validation using `SERVICE_REQUEST_ACTIONS`;
- action-specific request validation;
- `params` object validation and reserved `id`/`action` handling;
- canonical top-level type, enum, minimum, array-item, and unknown-property
  validation according to the property ledger below;
- role-specific consumption, trace projection, routing, and command
  forwarding;
- top-level-over-`params` precedence;
- caller trace construction;
- retained handoff route-hint application followed by shared-profile
  route-hint application;
- stable structured issue classification for adapter error mapping.

The module does not own:

- parsing HTTP bytes into JSON;
- loading or overlaying persisted/configured state;
- generating the transport-specific command id;
- selecting an HTTP relay daemon session or reviving it;
- writing an HTTP response or JSON-RPC response;
- sending a command to the daemon queue;
- executing daemon actions.
- interpreting or canonicalizing the HTTP-only top-level `args` legacy input.

### Intended interface

Keep one external operation and two result types. Exact Rust names may change
during implementation if existing conventions require it, but the interface
must not grow transport methods.

```rust
pub(crate) struct ServiceRequestNormalization<'a> {
    pub request: &'a Value,
    pub service_state: Option<&'a ServiceState>,
}

pub(crate) struct NormalizedServiceRequest {
    pub command: Value,
    pub trace: Value,
}

pub(crate) fn normalize_service_request(
    input: ServiceRequestNormalization<'_>,
) -> Result<NormalizedServiceRequest, ServiceRequestIssue>;
```

`command` leaves `id` unset. Each adapter adds its existing id immediately
after successful normalization:

- HTTP: `http-service-request-<action>-<uuid>`;
- MCP: `mcp-service-request-<action>-<uuid>`.

This keeps transport identity out of the module and preserves existing request
ids. The normalizer must reject or ignore `params.id` exactly as today, so the
adapter-added id remains authoritative.

`ServiceRequestIssue` must carry a stable internal kind plus message data, not
an HTTP status or JSON-RPC code. Expected kinds include missing action,
unsupported action, invalid field type, blocked manual action, stale monitor
evidence, forbidden CDP execution, invalid service tab handle, invalid bounded
recipe, and route-hint failure. HTTP maps an issue to its current 400 response;
MCP maps it to JSON-RPC invalid params (`-32602`). Internal state-loading errors
remain MCP internal errors and are not normalization issues.

Keep one private `ServiceRequestFieldSpec` table inside the module. Each entry
names one canonical property, its expected top-level kind/constraint, and its
`validation`, `trace`, `routing`, `command`, or `structural` roles. Validation,
forwarding, trace projection, and Rust parity tests derive from that one table;
do not create separate hand-maintained allowlists that can drift again. The
HTTP `args` exception is not an entry in this table.

### Normalization order

The implementation must make this order explicit and test it:

1. require a JSON object and a nonempty string `action`;
2. verify the action is in `SERVICE_REQUEST_ACTIONS`;
3. reject unknown canonical top-level properties and enforce the ledger's
   schema type, enum, minimum, and string-array item rules;
4. run common safety gates in the current order;
5. validate `params` as an object when present;
6. copy `params` except reserved `id` and `action`;
7. consume validation-only fields without copying them, then copy only fields
   carrying the `command` role over parameter values;
8. build trace output only from fields carrying the `trace` role;
9. apply retained handoff route hints;
10. apply shared-profile route hints;
11. return the command and trace without performing I/O.

The retained-handoff helper must run before the shared-profile helper, matching
both current implementations. Explicit route hints and
`allowDuplicateProfileLane=true` continue to short-circuit shared-profile
selection according to `service_access.rs`.

HTTP removes the recognized top-level legacy `args` member before the canonical
normalizer call and reapplies its raw JSON value to the normalized command
afterward, preserving current top-level-over-`params.args` precedence. This is
an adapter compatibility operation, not part of the module's interface. MCP
does not accept top-level `args`; MCP callers retain `params.args` only.

## Canonical Property Ledger

Roles are additive. `validation` means the normalizer consumes the field in a
general or action-specific rule. `trace` means MCP trace output includes the
field. `routing` means access-plan or relay selection reads it. `command` means
the normalized daemon command carries it. `structural` means it shapes the
request rather than being an ordinary forwarded property. `transport-legacy`
is outside the canonical schema and normalizer.

After remediation, the canonical schema has 62 properties: the current 59 plus
`browserHost`, `viewStreamProvider`, and `controlInputProvider`. Every
canonical property appears in the ledger below. `args` is the only adopted
transport-legacy exception.

| Properties | Canonical type or constraint | Current HTTP | Current MCP | Adopted roles | Adopted behavior |
| --- | --- | --- | --- | --- | --- |
| `action` | nonempty string in `SERVICE_REQUEST_ACTIONS` | validates and commands | validates and commands | structural, validation, command | Require, validate once, and place in the command. |
| `params` | object with arbitrary nested properties | flattens except `id` and `action` | flattens except `id` and `action` | structural, validation | Validate as object, flatten its nested values first, and protect reserved fields. `params` itself is not a carried command property. |
| `jobTimeoutMs`, `profileLeaseWaitTimeoutMs` | integer, minimum 1 | raw-copy | typed, traced, copied | validation, trace, command | Enforce positive integers, trace, and command. |
| `profileLeasePolicy` | string enum `reject` or `wait` | raw-copy | typed, traced, copied | validation, trace, command | Enforce enum, trace, and command. |
| `blockedByManualAction`, `manualSeedingRequired`, `allowManualAction` | boolean | gate-only | gate-only | validation-only | Consume for the manual-seeding gate and omit from command and trace. |
| `monitorRunDueSummary` | object | gate-only | gates, traces, copies | validation-only | Consume for the freshness gate and omit from command and trace. This intentionally removes the nonsemantic MCP-only forwarding drift. |
| `allowMonitorFreshnessRisk` | boolean | gate-only | gates, traces, copies | validation-only | Consume for the freshness override and omit from command and trace. |
| `requiresCdpFree`, `cdpAttachmentAllowed` | boolean | validates and copies | validates, traces, copies | validation, trace, command | Enforce boolean, apply CDP safety gates, trace, and command. |
| `serviceTabHandle` | object via canonical `$ref` | action-validates and copies | action-validates, traces, copies | validation, trace, command | Require an object generally and preserve existing action-specific handle invariants. Do not introduce a second deep schema engine in this packet. |
| `targetId` | string | copies | typed, traces, copies | validation, trace, routing, command | Enforce string and preserve target binding. |
| `script`, `expression` | string | action-validates and copies | action-validates, traces, copies | validation, trace, command | Preserve bounded-evaluate alias and command shape. |
| `returnByValue` | boolean | action-validates and copies | action-validates, traces, copies | validation, trace, command | Preserve bounded-evaluate rule, trace, and command. |
| `timeoutMs`, `maxReturnBytes`, `maxTextBytes`, `maxBodyBytes` | integer, minimum 1 | action-validates and copies | typed/action-validates, traces, copies | validation, trace, command | Enforce positive integers, preserve recipe caps, trace, and command. |
| `includeScreenshot`, `captureEvidenceOnFailure` | boolean | copies | typed, traces, copies | validation, trace, command | Enforce boolean, trace, and command. |
| `screenshotDir` | string | copies | typed, traces, copies | validation, trace, command | Enforce string, trace, and command. |
| `maxConsoleEntries`, `maxErrorEntries`, `maxRequestEntries` | integer, minimum 1 | copies | typed, traces, copies | validation, trace, command | Enforce positive integers, trace, and command. |
| `probe`, `uiAction`, `networkCapture`, `fileTransfer` | object with permissive nested properties | action-validates and copies | action-validates, traces, copies | validation, trace, command | Enforce object type plus existing bounded recipe rules, then trace and command. |
| `repairPolicy` | string enum `reject_only`, `reuse_compatible`, `open_if_missing`, or `replace_duplicates` | action-validates and copies | action-validates, traces, copies | validation, trace, command | Enforce enum, trace, and command. |
| `browserBuild` | string enum `stock_chrome`, `stealthcdp_chromium`, or `cdp_free_headed` | raw-copy and routing input | typed, traces, copies | validation, trace, routing, command | Enforce enum and preserve routing evidence in trace and command. |
| `displayIsolation` | string enum `private_virtual_display`, `shared_display`, or `ambient_display` | raw-copy and routing input | typed, traces, copies | validation, trace, routing, command | Enforce enum and preserve routing evidence in trace and command. |
| `browserHost` | add string enum `local_headless`, `local_headed`, `docker_headed`, `remote_headed`, `cloud_provider`, or `attached_existing` | raw-copy and consumed by routing/daemon | MCP-public, typed, traced, copied | validation, trace, routing, command | Add to canonical schema, generated clients, docs, and role gates; preserve both active paths. |
| `viewStreamProvider` | add string enum `cdp_screencast`, `chrome_tab_webrtc`, `virtual_display_webrtc`, `novnc`, `rdp_gateway`, or `external_url` | raw-copy and consumed by routing/daemon | MCP-public, typed, traced, copied | validation, trace, routing, command | Add to canonical schema, generated clients, docs, and role gates; preserve both active paths. |
| `controlInputProvider` | add string enum `cdp_input`, `webrtc_input`, `vnc_input`, or `manual_attached_desktop` | raw-copy and consumed by routing/daemon | MCP-public, typed, traced, copied | validation, trace, routing, command | Add to canonical schema, generated clients, docs, and role gates; preserve both active paths. |
| `serviceName`, `agentName`, `taskName` | string | copies | typed, base-traces, copies | validation, trace, command | Enforce string and preserve caller identity in trace and command. |
| `targetServiceId`, `targetService`, `siteId`, `loginId`, `accountId` | string | copies and routes | typed except dropped `accountId`; traces except `accountId`; copies except `accountId` | validation, trace, routing, command | Enforce string and preserve every identity hint through both adapters, trace, access planning, and command. |
| `targetServiceIds`, `targetServices`, `siteIds`, `loginIds`, `accountIds` | array of strings | copies and routes | typed except dropped `accountIds`; traces except `accountIds`; copies except `accountIds` | validation, trace, routing, command | Enforce string items and preserve every identity hint. Empty arrays remain allowed unless an action rule says otherwise. |
| `url`, `desiredUrl` | string | copies and routes | typed, traces, copies | validation, trace, routing, command | Enforce string and preserve site-policy/tab-acquisition input. |
| `profile`, `profileId`, `runtimeProfile` | string | copies and routes | typed, traces, copies | validation, trace, routing, command | Enforce string and preserve explicit profile authority. |
| `profileClass` | string enum `default`, `managed_one_time`, `durable_named`, or `operator_supplied` | raw-copy | typed, traces, copies | validation, trace, routing, command | Enforce enum and preserve profile-lane semantics. |
| `cdpUrl` | string | `external_byop_adopt` validates and copies | absent from tool schema and builder | validation, command | Add to MCP schema and normalizer, enforce nonempty action rule, and do not trace endpoint credentials. |
| `cdpPort` | integer, minimum 1 | `external_byop_adopt` validates and copies | absent from tool schema and builder | validation, command | Add to MCP schema and normalizer, enforce positive integer and endpoint exclusivity, and do not trace. |
| `browserId`, `sessionName` | string | copies and influences relay | typed, traces, copies | validation, trace, routing, command | Enforce string and preserve explicit/derived retained-lane routing. |
| `allowDuplicateProfileLane` | boolean | copies and short-circuits routing | typed, traces, copies | validation, trace, routing, command | Enforce boolean and preserve explicit duplicate-lane authority. |
| `manualLoginLaunch` | boolean | copies | builder copies but MCP schema omits | validation, command | Add to MCP schema, enforce boolean, and preserve command behavior without adding it to trace. |
| top-level `args` | not canonical | raw-copies any JSON value and overrides `params.args` | rejected as unknown; `params.args` remains possible | transport-legacy, HTTP command only | Preserve exactly in the HTTP adapter. Exclude from canonical schema, generated clients, MCP top-level input, normalizer, and canonical parity. MCP retains `params.args`. |

There are no adopted fields whose sole role is `trace-only` or
`routing-only`. Trace and routing inputs that must survive for diagnostics or
daemon behavior also carry `command`. The two empty exclusive sets are guarded
explicitly so future additions cannot appear accidentally.

### Canonical type policy

- The normalizer enforces the public schema's top-level types, enum values,
  positive-integer minima, and string-array item types for both adapters.
- Explicit `null` is invalid for every non-null canonical property. Omission is
  allowed except for required `action`.
- Empty strings and empty string arrays remain allowed where the schema allows
  them. Existing action-specific rules may require nonempty values.
- `params`, recipe objects, and `monitorRunDueSummary` remain permissive inside
  the object except for existing action-specific safety checks.
- Unknown top-level properties are rejected according to
  `additionalProperties: false`, except that HTTP recognizes and removes the
  single legacy `args` member before normalization.
- This intentionally tightens HTTP acceptance of schema-invalid wrong types
  and unknown properties. Such inputs were never part of the public contract.
  It preserves MCP's typed posture and does not loosen either transport.
- Characterization fixtures must cover wrong-type string, enum, integer,
  boolean, string-array, object, explicit-null, and unknown-property inputs.
  Each fixture records the current divergence and asserts the adopted common
  `ServiceRequestIssue` kind and message.
- The canonical wrong-type issue text follows the existing MCP field-specific
  phrasing where available. HTTP returns that message inside its existing 400
  JSON envelope; MCP returns it inside the existing JSON-RPC `-32602` invalid
  params envelope. The envelope remains transport-specific and exact tests bind
  status/code/message placement.

### Role-specific parity invariants

Replace a naive property-equals-forwarded assertion with these set checks.
Rust module tests own role-to-command/trace/routing/validation invariants;
`scripts/check-service-api-mcp-parity.js` owns schema, MCP, action, generated,
and transport-legacy surface invariants:

1. canonical schema properties equal MCP top-level input properties and the
   normalizer's recognized canonical property set;
2. schema action enum equals `SERVICE_REQUEST_ACTIONS` and the MCP action enum;
3. fields tagged `command` equal the normalizer's canonical top-level command
   forwarding set; structural `params` is excluded because its nested values,
   not the container property, are flattened into the command;
4. fields tagged `trace` equal its trace projection set;
5. fields tagged `routing` contain every canonical top-level field read by
   retained-handoff, shared-profile, and HTTP relay routing; arbitrary nested
   `params` relay aliases are an adapter compatibility surface and are covered
   separately by exact HTTP relay fixtures;
6. validation-only fields are read by their gates and are absent from command
   and trace sets;
7. structural fields are handled by structure-specific code and are not
   duplicated in ordinary forwarding tables;
8. the HTTP transport-legacy set is exactly `{args}` and is excluded from all
   canonical, MCP top-level, generated-client, and cross-adapter equality
   checks;
9. generated request field/type sets equal the canonical schema after adding
   the three routing properties, with no generated top-level `args`;
10. a fixture verifies MCP `params.args` and HTTP top-level `args` both reach
    the command through their distinct authorized paths.

## Compatibility Contract

### Inputs and output

- Every schema-valid request accepted today remains accepted.
- Every canonical field follows the adopted ledger. Command-forwarded fields
  retain JSON type and top-level precedence; the two monitor fields cease their
  MCP-only command/trace leakage and join the three manual markers as
  validation-only.
- Schema-invalid HTTP inputs may be rejected under the canonical type policy.
- HTTP top-level `args` remains a raw transport-local overlay with current
  precedence and is not a canonical field.
- `params.id` and `params.action` remain ignored.
- Top-level fields continue to override same-named fields in `params`.
- Existing request-id prefixes remain unchanged.
- Existing HTTP relay-session behavior remains in
  `cli/src/native/stream/http.rs` and must continue to inspect the original body
  plus normalized command.
- MCP state loading, configured-entity overlay, readiness refresh, and queued
  tool response stay in `cli/src/mcp.rs`.
- Action execution and daemon-side defense-in-depth validation stay unchanged.

### Contract repairs required by the current schema

The migration must not freeze known drift as the new design:

1. MCP must advertise, validate, and forward `cdpUrl`, `cdpPort`, and
   `manualLoginLaunch` because they are already canonical schema fields.
2. MCP must preserve `accountId` and `accountIds` in the normalized command and
   trace instead of accepting then dropping them.
3. `external_byop_adopt` must apply the same acceptance rule through HTTP and
   MCP: a nonempty `runtimeProfile` or `profileId`, plus exactly one CDP
   endpoint selector.
4. Add `browserHost`, `viewStreamProvider`, and `controlInputProvider` to the
   canonical schema with the existing MCP enum values because both adapters
   already expose, route, trace, forward, and execute them.
5. Normalize the five safety markers according to the ledger: manual-action
   and monitor-freshness fields are consumed by ingress validation and do not
   enter the daemon command or trace.
6. Preserve HTTP top-level `args` exactly as a transport-local legacy input.
   Do not add it to the canonical schema, MCP top-level input, normalizer,
   generated client, or public cross-transport contract. MCP retains
   `params.args`.
7. Enforce canonical top-level types and unknown-property rejection through
   both adapters. HTTP acceptance of schema-invalid inputs is intentionally
   tightened; transport-specific error envelopes remain stable.

These are additive or corrective repairs. They must not alter action execution
semantics.

### Error compatibility

Before moving logic, freeze a table of current HTTP and MCP outcomes for every
action-specific rejection and canonical type class. Adopt one exact shared
issue kind and message per rejection condition. Use existing MCP typed-field
wording where available and adjudicate current action-message differences in
the characterization table. Adapter tests must assert the exact shared message
inside the existing transport-specific envelope.

Do not expose Rust enum variant names as a new public error contract. Existing
HTTP status and JSON-RPC code are the stable public envelope.

## File Plan

### Add

- `cli/src/native/service_request.rs`
  - deep normalization module;
  - structured issue kinds;
  - module-interface tests and parity fixtures.

### Modify

- `cli/src/native/mod.rs`
  - register the module.
- `cli/src/mcp.rs`
  - reduce `service_request_command_with_state` to an MCP adapter;
  - remove duplicated service-request validation and construction helpers after
    module tests cover them;
  - preserve generic browser-tool helpers still used by other MCP tools;
  - source the MCP `service_request` input schema from the canonical contract
    helper or prove exact property parity with it.
- `cli/src/native/stream/http.rs`
  - reduce `service_request_command_with_state` to JSON parsing, normalizer
    invocation, HTTP id assignment, and error mapping;
  - preserve top-level `args` extraction and post-normalization overlay as the
    single named legacy compatibility path;
  - delete duplicated validators;
  - keep relay-session selection, daemon revival, and response writing local.
- `cli/src/native/service_contracts.rs`
  - keep `SERVICE_REQUEST_ACTIONS` authoritative;
  - add a canonical MCP input-schema helper if needed so the hand-written MCP
    property list can be deleted;
  - keep stable schema IDs and contract metadata unchanged.
- `docs/dev/contracts/service-request.v1.schema.json`
  - add `browserHost`, `viewStreamProvider`, and `controlInputProvider` with
    their current MCP enum values and descriptions;
  - do not add `args`.
- `packages/client/src/service-request.generated.js`
- `packages/client/src/service-request.generated.d.ts`
  - regenerate, never hand-edit.
- `scripts/generate-service-request-client.js`
  - change only if existing generation cannot represent the three added enum
    string fields correctly; otherwise regenerate without generator edits.
- `scripts/check-service-api-mcp-parity.js`
  - implement the role-specific set invariants above rather than equating all
    canonical properties with command forwarding.
- `scripts/test-service-request-client.js`
  - assert generated and MCP tool-call preservation for `browserHost`,
    `viewStreamProvider`, and `controlInputProvider`;
  - assert top-level `args` is not generated or canonical.
- `cli/src/output.rs`
  - align the service-request help text for the three routing fields where the
    existing help surface describes raw service requests; if no such field list
    exists, add an explicit no-change receipt to this plan rather than inventing
    unrelated CLI help.
- `README.md`, `skills/agent-browser/SKILL.md`, and
  `docs/src/app/service-mode/page.mdx`
  - align existing raw service-request descriptions for the three canonical
    routing fields;
  - do not advertise top-level `args` for MCP or generated clients.
- inline doc comments in `service_request.rs`, `mcp.rs`, `http.rs`, and
  `service_contracts.rs`
  - document the module seam, role ledger ownership, type enforcement, and
    HTTP legacy exception without duplicating the full ledger in source.
- this plan
  - record checkpoints, validation evidence, accepted audit findings, and
    closeout state during execution.

### Must not modify in this packet

- `cli/src/native/actions.rs`;
- daemon action implementations;
- remote-view acquisition/finalization modules;
- dashboard behavior;
- installed runtime or retained browser state;
- release metadata or changelog.

The `actions.rs` monolith is a separate architectural campaign. This plan
reduces ingress duplication without mixing that larger execution-module
refactor into the same write set.

## Bounded Execution Sequence

### Slice 0 | Characterization and frozen parity matrix

1. Add module-level fixture inputs covering every special action and all
   canonical top-level field classes.
2. Record current accepted valid-command shapes for HTTP and MCP, normalizing
   only request ids in comparisons.
3. Record each invalid request's accept/reject result, HTTP status, JSON-RPC
   code, and message text.
4. Add explicit red tests for the observed drift:
   - MCP `external_byop_adopt` with `cdpUrl`;
   - MCP `external_byop_adopt` with `cdpPort`;
   - MCP `accountId` and `accountIds` preservation;
   - MCP `manualLoginLaunch` schema exposure;
   - canonical schema and generated-client coverage for `browserHost`,
     `viewStreamProvider`, and `controlInputProvider`;
   - HTTP-only top-level `args` preservation and MCP `params.args` preservation;
   - rejection of top-level MCP `args` and exclusion from the canonical schema;
   - adopted wrong-type and unknown-property rejection through both adapters.

Stop this slice if a current behavior conflicts with a documented safety rule.
Adjudicate it in the plan before implementation rather than guessing.

### Slice 1 | Introduce the deep module behind existing adapters

1. Add `service_request.rs` with the single normalization interface.
2. Implement common validation, merge, trace, and route-hint behavior.
3. Add table-driven tests at the module interface.
4. Keep both old adapter implementations temporarily and compare their output
   against the module for frozen fixtures.

No public adapter switches in this slice.

### Slice 2 | Migrate HTTP adapter

1. Parse the body in HTTP as today.
2. Call the normalizer with the current service-state snapshot.
3. Add the unchanged HTTP request id.
4. Map issues to the existing 400 response shape.
5. Keep relay-session selection and recovery unchanged.
6. Delete HTTP-only validators and builder logic once interface tests and
   adapter tests pass.

### Slice 3 | Migrate MCP adapter and repair canonical drift

1. Keep state loading, configured overlay, and readiness refresh in MCP.
2. Call the normalizer with parsed MCP arguments.
3. Add the unchanged MCP request id.
4. Map normalization issues to JSON-RPC invalid params.
5. Replace the hand-written MCP `service_request` property list with a
   canonical schema projection, or add exact role-specific set gates if a
   projection would make external `$ref` handling unsafe.
6. Implement the seven contract repairs listed above.
7. Delete MCP-only service-request validators and the service-request-specific
   `ServiceToolContext` construction path. Preserve `ServiceToolContext` and
   generic command helpers used by other MCP tools.

### Slice 4 | Replace tests and strengthen parity gates

1. Delete duplicate HTTP/MCP tests whose only purpose was to retest the same
   validation implementation.
2. Retain thin adapter tests for parsing, request-id prefix, error envelope,
   relay-session selection, state loading, and queue handoff.
3. Keep all semantics tests at the normalizer interface.
4. Implement the ten role-specific invariants across Rust module tests and the
   JavaScript schema/MCP/generated parity gate as assigned above.
5. Regenerate and verify client artifacts.

The replace-don't-layer rule applies: once the deep module's interface proves
a rule, do not retain two transport-local copies of the same rule.

### Slice 5 | Documentation and closeout

1. Update the existing public surfaces for the three canonical routing fields
   and leave top-level `args` unadvertised outside its HTTP compatibility test.
2. Run selected and required validation.
3. Run the single closed-world Cycle 2 verification against the adjudication
   table. Do not reopen broad discovery or add another remediation cycle.
4. Record exact pass/fail evidence and close the plan only when every
   acceptance criterion is proved.

## Test Replacement Strategy

The interface is the test surface.

### Normalizer tests

- accepts every `SERVICE_REQUEST_ACTIONS` value with the minimum valid fixture;
- rejects unknown and empty actions;
- recognizes every canonical scalar, array, object, and recipe field and
  applies its ledger roles;
- rejects wrong-type, invalid-enum, nonpositive-integer, invalid string-array,
  explicit-null, and unknown canonical inputs with stable issues;
- protects `id` and `action` inside `params`;
- proves top-level-over-`params` precedence;
- proves the exact safety-gate order when multiple inputs are invalid;
- covers valid/stale service tab handles;
- covers all bounded recipe requirements;
- covers manual-seeding and monitor-freshness overrides;
- proves the five validation-only safety fields are absent from command and
  trace output;
- covers `external_byop_adopt` endpoint exclusivity;
- covers retained-handoff hinting before shared-profile hinting;
- covers explicit route-hint and duplicate-lane short circuits;
- returns a trace and command from the same normalized caller context;
- performs no file, network, queue, browser, or runtime mutation.

### HTTP adapter tests

- empty, invalid, and non-object JSON parsing;
- HTTP request-id prefix;
- issue-to-400 mapping;
- exact preservation of raw top-level `args` and precedence over `params.args`;
- original relay-session rules for focus/takeover and non-focus actions;
- retained handoff daemon revival remains outside normalization.

### MCP adapter tests

- MCP request-id prefix;
- issue-to-`-32602` mapping;
- state overlay and readiness refresh before normalization;
- trace and queued tool handoff;
- canonical input-schema property parity;
- top-level `args` rejection and `params.args` preservation.

### Cross-adapter fixture test

For each canonical valid fixture, compare normalized HTTP and MCP commands after
removing only the transport-generated `id`. Keep HTTP top-level `args` out of
this equality matrix and test it as a transport-legacy fixture. For each invalid
fixture, compare the stable issue kind and rejection condition while separately
asserting exact HTTP and MCP public error envelopes.

## Validation

Run focused checks after each migration slice:

```bash
cargo test --manifest-path cli/Cargo.toml service_request -- --test-threads=1
pnpm test:service-api-mcp-parity
pnpm test:service-client
node scripts/generate-service-request-client.js --check
git diff --check
```

Run required Rust quality gates because `cli/src/` changes:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
```

Use the repository selector against the pre-slice base and run every returned
gate:

```bash
pnpm validation:select -- --base <last-known-green-ref>
```

Expected wider gates include the canonical Rust suite and docs build because
the normalizer is cross-cutting and the schema is user-facing:

```bash
cargo test --manifest-path cli/Cargo.toml
pnpm --dir docs build
```

No live browser smoke is required for a behavior-preserving in-process
refactor. If a focused no-launch test exposes a change in relay selection or
daemon command shape, stop and repair it before considering a live smoke.

## Acceptance Criteria

1. `cli/src/native/service_request.rs` exposes one normalization operation that
   owns request validation, merge precedence, trace construction, and route
   hint ordering.
2. HTTP and MCP are thin adapters and contain no duplicated action-specific
   service-request validators.
3. Deleting the normalizer would force both adapters to recreate the action
   rules, merge semantics, trace fields, and route-hint sequence, satisfying
   the deletion test.
4. All existing schema-valid requests retain their command shape and behavior,
   apart from transport-generated id values.
5. HTTP request ids, MCP request ids, HTTP statuses, JSON-RPC codes, relay
   selection, queue handoff, and state-loading behavior remain unchanged.
6. `external_byop_adopt`, `cdpUrl`, `cdpPort`, `manualLoginLaunch`,
   `accountId`, and `accountIds` have one acceptance and forwarding rule across
   HTTP and MCP.
7. `browserHost`, `viewStreamProvider`, and `controlInputProvider` are
   canonical schema fields with generated-client, MCP, docs, and test coverage
   matching their existing routing and daemon use.
8. Existing HTTP top-level `args` behavior is preserved only in the HTTP
   adapter, excluded from canonical and generated surfaces, and MCP retains
   `params.args` without accepting top-level `args`.
9. The normalizer enforces canonical property types, enum values, positive
   minima, string-array item types, explicit-null rejection, and unknown-field
   rejection for both adapters. HTTP may intentionally reject schema-invalid
   inputs it previously raw-copied.
10. The five validation-only safety markers are consumed by ingress gates and
    omitted consistently from command and trace output.
11. The parity gate proves all role-specific property, action, command, trace,
    routing, validation-only, structural, transport-legacy, and generated
    invariants without assuming every canonical input is forwarded.
12. Duplicate transport-local semantics tests are replaced by normalizer
   interface tests; remaining adapter tests cover only actual transport
   variation.
13. Focused Rust, parity, client, generation, formatting, strict Clippy,
    selected validation, canonical Rust, docs build, and patch checks pass.
14. No browser is launched, closed, restarted, or reprofiled, and no installed
    runtime or tenant state is changed.
15. The independent work audit finds no unresolved blocking regression against
    this frozen plan after the bounded review cycle.

## Non-Goals

- no `actions.rs` decomposition or daemon execution refactor;
- no action addition, removal, or semantic redesign;
- no queue, worker, cancellation, timeout, lease, or repository rewrite;
- no change to access-plan selection policy;
- no change to HTTP relay-session rules or remote-view daemon revival;
- no new public error envelope or transport status code;
- no canonicalization, generated-client exposure, or MCP top-level exposure of
  HTTP legacy `args`;
- no dashboard changes;
- no live browser, installed-runtime, tenant, provider, or scheduler effects;
- no release, version bump, tag, push, or pull request in this packet.

## Risks And Controls

### Risk: tightening undocumented invalid inputs

Control: characterize both adapters first, then enforce the frozen canonical
type policy. Preserve every schema-valid public input and current HTTP `args`.
Schema-invalid wrong types, explicit nulls, and unknown fields may begin failing
through HTTP by design. Exact issue and transport-envelope fixtures make that
tightening explicit rather than accidental.

### Risk: changing error text accidentally

Control: use stable issue kinds internally and fixture existing public
envelopes. Transport-specific wording can remain in adapter mapping until a
separate contract change is authorized.

### Risk: moving HTTP relay knowledge into the module

Control: keep original-body inspection, relay-session selection, handoff daemon
revival, and HTTP response writing in the HTTP adapter.

### Risk: making the new module shallow

Control: expose one operation. Keep field parsing, validation helpers, merge
rules, trace projection, and route-hint sequencing private. Do not export an
interface per action or per validator.

### Risk: schema projection breaks MCP `$ref` consumers

Control: project the canonical request schema into the MCP input schema with an
inline permissive `serviceTabHandle` property, matching current behavior. If
that cannot be done without client regressions, retain schema construction in
`service_contracts.rs` and enforce role-specific property/action parity
mechanically.

### Risk: legacy `args` leaks into the canonical seam

Control: keep the compatibility extraction and overlay visibly in the HTTP
adapter, name `{args}` as the exact transport-legacy set, and add negative
schema, generated-client, and MCP top-level tests. Future canonicalization or
deprecation is nonblocking backlog outside this campaign.

### Risk: concurrent architecture packets touch shared files

Control: this plan owns only its named plan during planning. Execution must
recheck worktree status and coordinate overlapping edits before changing
`native/mod.rs`, `service_contracts.rs`, docs, or validation scripts.

## Cycle 1 Adjudication

Audit source:
`docs/dev/notes/0098-2026-08-09-service-request-normalization-plan-audit.md`

| Finding | Orchestrator disposition | Plan remediation | Cycle 2 proof target |
| --- | --- | --- | --- |
| `P0098-A1-01` | `blocking`, accepted | Added `browserHost`, `viewStreamProvider`, and `controlInputProvider` to the canonical contract plan, generated/docs/test implications, property ledger, and role-specific parity invariants. | Verify all three fields are canonical, typed, routed, traced, commanded, generated, documented, and tested without removing current behavior. |
| `P0098-A1-02` | `blocking`, accepted | Added the complete property ledger. Adopted the five safety markers as validation-only and replaced property-equals-forwarded parity with role-specific sets. | Verify every canonical and legacy property appears exactly once in the ledger and validation-only fields cannot leak into command or trace. |
| `P0098-A1-03` | `nonblocking_backlog`, resolved conservatively | Rejected canonical top-level `args` for this campaign. Preserved raw top-level `args` only in HTTP, kept MCP access through `params.args`, excluded it from canonical parity, and added negative contract tests. | Verify the exact transport-legacy set is `{args}`, canonical/generated/MCP top-level sets exclude it, and both authorized compatibility paths remain tested. Future canonicalization or deprecation stays backlog. |
| `P0098-A1-04` | `blocking`, accepted | Froze canonical type enforcement for both adapters, explicitly authorizing HTTP tightening for schema-invalid inputs while preserving exact status/code envelopes. Added wrong-type/null/unknown characterization and issue tests. | Verify type policy covers every schema class and exact HTTP/MCP issue plus envelope assertions are required before migration. |

No further broad discovery is authorized for this plan. Cycle 2 is limited to
the proof targets in this table and critical regressions introduced by this
remediation.

## Cycle 2 Residual And Bounded Orchestrator Resolution

Cycle 2 passed `P0098-A1-01`, `P0098-A1-03`, and `P0098-A1-04`. It retained one
blocking contradiction under `P0098-A1-02`: `params` was both structural and
tagged as a carried command property, while canonical routing parity also
claimed arbitrary nested relay aliases.

The orchestrator resolved that closed finding without a third audit:

1. `params` is `structural, validation` only. The normalizer validates the
   object and flattens its nested values, but the `params` container is not a
   canonical top-level command property.
2. Command parity compares only ledger entries tagged `command` with the
   canonical top-level forwarding set.
3. Routing parity compares only canonical top-level routing fields. Existing
   open-`params` relay aliases such as `daemonSession`, `targetSession`,
   `targetSessionName`, and `sessionId` remain HTTP adapter compatibility
   behavior and receive exact fixtures outside canonical ledger equality.

This is the bounded repair for the terminal Cycle 2 finding, not another
review cycle. The finding and its consequence remain recorded in the audit
note. Root verification confirms the role definitions, ledger row, parity
invariants, and adapter fixtures are now mutually satisfiable. Plan 0098 may
proceed to implementation with no further plan audit.

## Review Bounds

- Cycle 1 `drift_discovery` is complete and adjudicated above.
- This Plan version 2 is the one allowed remediation pass.
- Cycle 2 is one `closed_world` verification limited to
  `P0098-A1-01`, `P0098-A1-02`, `P0098-A1-03`, `P0098-A1-04`, and critical
  regressions introduced by this remediation.
- Review ends after Cycle 2. Remaining nonblocking concerns are logged in this
  plan and do not reopen broad discovery.
- The terminal `P0098-A1-02` contradiction is closed only by the bounded
  orchestrator resolution above. No Cycle 3 exists.

## Delegation Receipt | Planning Role

- task_name: `/root/plan_request_normalization`
- role: deep analysis and implementation-ready plan author
- status: completed
- write_scope:
  `docs/dev/plans/0098-2026-08-09-service-request-normalization-deepening-plan.md`
- source_changes: none
- runtime_effects: none
- completion_evidence:
  - Graphiti runtime was healthy and `agent_browser_main` was queried twice;
  - CodeGraph status, exploration, and impact analysis covered both ingress
    builders, route-hint authority, contract metadata, generated client, and
    parity gates;
  - canonical schema, current roadmap, access-plan handoff note, Plans 0033,
    0034, 0069, and relevant policy were read;
  - observed HTTP/MCP field and validation drift is enumerated above;
  - Cycle 1 audit findings were adjudicated into Plan version 2 with a complete
    property ledger, canonical type policy, role-specific invariants, and
    conservative `args` compatibility decision;
  - only this plan artifact was created.
- reconciliation_owner: root orchestrator
- next_role: implementation executor using the Cycle 2 bounded orchestrator
  resolution as part of the frozen plan

## Done Definition

- every acceptance criterion has current source or validation evidence;
- the HTTP and MCP request semantics cross one deep normalization seam;
- no duplicate action-specific ingress validators remain;
- role-specific schema/property/action parity is mechanically guarded;
- review is closed within the two-cycle bound;
- residual nonblocking concerns are recorded rather than silently expanding
  this packet;
- the plan records final validation evidence and transitions to `CLOSED`.
