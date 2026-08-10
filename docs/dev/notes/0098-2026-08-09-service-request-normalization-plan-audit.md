# Plan 0098 Cycle 1 Audit | Service Request Normalization Deepening

Review mode: `drift_discovery`

Review cycle: 1 of 2

Reviewer role: independent plan auditor

Branch: `architecture-deepening-20260809`

Repository base: `ae36b272327982e3227f4dc7c5d6dc5b4b16350c`

Frozen target:
`docs/dev/plans/0098-2026-08-09-service-request-normalization-deepening-plan.md`

Target content SHA-256:
`96a65e540c6c552f12b87561976a2e17b87792de80c95280e264beba7710acd5`

Audit date: 2026-08-09

## Review Packet

The frozen acceptance requires one deep in-process normalizer, thin HTTP and
MCP transport adapters, preservation of the action set, command shapes,
request ids, public error envelopes, relay selection, and queue behavior, and
repair of proven canonical drift only. Runtime, live-service, tenant, release,
and installation effects are excluded. Review is bounded to one broad discovery
cycle, one remediation pass for accepted blockers, and one closed-world
verification.

The audit used the `codebase-design` deep-module and deepening criteria. The
candidate has the correct dependency classification and seam: normalization is
in-process, while HTTP and MCP are the two real adapters. CodeGraph was healthy
at 419 indexed files, 14,341 nodes, and 43,350 edges. Focused CodeGraph reads
covered both `service_request_command_with_state` implementations,
`ServiceToolContext`, route-hint ordering, command construction, and queue
handoff. Direct reads were limited to the canonical JSON schema, generated
artifacts, literal property lists, tests, and policy. Graphiti MCP was readable,
but the focused `agent_browser_main` query returned no source-specific Plan
0098 recall, so no Graphiti claim was used as authority.

## Findings

| ID | Criterion | Exact evidence | Consequence | Reproducer or check | Confidence | Suggested disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `P0098-A1-01` | The canonical property set, MCP input schema, normalizer forwarding contract, and compatibility file plan must be mutually satisfiable without dropping active routing inputs or silently widening scope. | The canonical schema's properties end at `manualLoginLaunch` and do not contain `browserHost`, `viewStreamProvider`, or `controlInputProvider` (`docs/dev/contracts/service-request.v1.schema.json:257-265,380-385`). MCP publicly advertises all three (`cli/src/mcp.rs:970-988`), parses them in `ServiceToolContext::from_arguments` (`cli/src/mcp.rs:10100-10104`), and forwards them (`cli/src/mcp.rs:10198-10205`). HTTP forwards all three (`cli/src/native/stream/http.rs:1734-1736`). Daemon-side code consumes them, including `cli/src/native/actions.rs:651-664`, and the live service-request smoke supplies them (`scripts/smoke-service-request.js:140-142`). The plan nevertheless says the canonical schema will add `args` only (`plan:319-321`) while requiring exact property-set parity or canonical projection (`plan:400-402,534-535`). | An implementation following the file plan cannot make the exact parity gate pass. It must either remove three current public and execution-relevant routing inputs, expand the canonical schema beyond the authorized file plan, or weaken the promised exact parity. Any of those is a material divergence from the frozen packet. | Compare property sets: `jq -r '.properties \| keys[]' docs/dev/contracts/service-request.v1.schema.json`, then compare with top-level keys in `cli/src/mcp.rs:801-1067`. The current schema-to-MCP difference is missing `cdpUrl`, `cdpPort`, and `manualLoginLaunch`; the MCP-to-schema difference is `browserHost`, `viewStreamProvider`, and `controlInputProvider`. | High | `blocking` |
| `P0098-A1-02` | The normalizer must preserve daemon command shapes while the parity gate distinguishes public input properties from fields that are validation-only, trace-only, routing-only, or command-forwarded. | The schema includes `blockedByManualAction`, `manualSeedingRequired`, `allowManualAction`, `monitorRunDueSummary`, and `allowMonitorFreshnessRisk` (`docs/dev/contracts/service-request.v1.schema.json:136-155`). HTTP uses all five as ingress gates (`cli/src/native/stream/http.rs:1645-1649,1708-1711`) but forwards none in its command allowlist (`cli/src/native/stream/http.rs:1726-1783`). MCP does not forward the three manual-action markers, but does forward the monitor summary and override through `ServiceToolContext` (`cli/src/mcp.rs:10067-10068,10133-10137,10285-10290`). The plan says to validate and copy canonical top-level fields (`plan:159-165,223-234`), preserve all canonical field classes (`plan:438-451`), and compare the complete canonical property set with the normalization forwarding surface (`plan:534-535`) without defining field roles or the intended disposition of these five fields. | Copying every canonical property changes HTTP command shape and adds three fields to both transports. Preserving current shapes leaves transport drift for the monitor fields and makes a one-to-one property-to-forwarding parity gate false. The implementer is forced to invent a field taxonomy and semantic choice after the plan is frozen. | Derive the current sets from the schema, HTTP allowlist, and MCP `command[...]` assignments. Excluding structural `action` and `params`, canonical fields not forwarded by HTTP are the five safety markers; canonical fields not forwarded by MCP include the three manual-action markers. Add fixtures asserting both normalized commands for these five inputs before deciding whether each field is consumed-only or forwarded. | High | `blocking` |
| `P0098-A1-03` | Repairs must be supported by canonical authority and must not publish a new cross-transport contract merely because one adapter has a legacy behavior. | HTTP forwards and tests top-level `args` (`cli/src/native/stream/http.rs:1772,5265-5292`), and daemon launch code consumes it as browser arguments (`cli/src/native/actions.rs:535-549,6011-6020`). The canonical schema omits it and closes the object with `additionalProperties: false` (`docs/dev/contracts/service-request.v1.schema.json:380-385`). MCP also advertises a closed input object with no top-level `args` (`cli/src/mcp.rs:795-1069`). The plan proposes adding `args` to the canonical schema, MCP, generated clients, parity checks, and public docs (`plan:269-273,319-336`) even though its own authority order places the canonical schema above current adapter behavior (`plan:24-34`) and its objective limits repairs to fields already declared by that contract (`plan:16-20`). MCP can already carry action-specific `args` through the existing permissive `params` object, so this is primarily a new top-level advertised and typed contract, not recovery of otherwise unreachable daemon capability. | Proceeding makes an HTTP-only extension a canonical cross-transport promise and broadens the advertised caller-controlled browser-flag surface. Removing it breaks tested HTTP compatibility. The frozen authorities do not decide between those outcomes. | Confirm absence with `jq -e '.properties.args' docs/dev/contracts/service-request.v1.schema.json` and inspect `cli/src/mcp.rs:795-1069`; confirm current HTTP and daemon behavior at the cited source and test lines. Obtain an explicit contract-authority decision before changing schema and public docs. | High | `needs_evidence` |
| `P0098-A1-04` | One normalizer must have a frozen type-validation policy that does not accidentally tighten HTTP, loosen MCP, or alter transport error behavior outside accepted drift repairs. | HTTP copies every listed top-level value as raw JSON without general type validation (`cli/src/native/stream/http.rs:1726-1786`). MCP parses the same fields through typed helpers such as `optional_positive_u64_argument`, `optional_string_argument`, array validators, and boolean validators (`cli/src/mcp.rs:10074-10142`) and returns JSON-RPC invalid params on mismatch. The target introduces `invalid field type` as a shared issue (`plan:213-219`) and requires canonical top-level validation (`plan:223-234`), while the compatibility control says rejection behavior should change only for canonical drift (`plan:560-565`). Slice 0 records the divergent outcomes but does not state which outcome becomes authoritative or list that decision as a stop-and-adjudicate condition unless a documented safety rule is involved (`plan:356-372`). | For a request such as `{"action":"navigate","jobTimeoutMs":"1000"}`, current HTTP forwards a string but current MCP rejects it. A single normalizer must choose one result. Canonical enforcement tightens HTTP; raw copying loosens MCP. Either can change accepted inputs and error behavior without an accepted finding or explicit plan rule. | Add paired characterization fixtures for wrong-type scalar, array, object, enum, and positive-integer fields. Record current HTTP command or error and MCP code/message, then state the authoritative post-migration rule for schema-invalid inputs before implementing the normalizer. | High | `blocking` |

## Findings Summary

- Blocking candidates: `P0098-A1-01`, `P0098-A1-02`, `P0098-A1-04`.
- Needs-evidence candidate: `P0098-A1-03`.
- Nonblocking backlog candidates: none.
- Rejected candidates: none at discovery time. Final disposition belongs to the
  orchestrator under the frozen review contract.

## Cycle 1 Terminal Recommendation

Plan 0098 is not implementation-ready in its current form. The deep-module seam
is sound, the state and effect boundaries are appropriate, and the planned
relay, queue, id, and error-envelope ownership is well placed. The property
contract is not yet closed enough to implement safely.

Use the one allowed remediation pass to add an explicit canonical property
ledger that classifies every field as validation-only, trace-only, routing-only,
command-forwarded, or structural; reconcile the three active noncanonical
routing fields; freeze the cross-transport rule for schema-invalid types; and
obtain or reject explicit authority for canonical top-level `args`. The parity
gate and fixtures should then check the ledger's role-specific sets instead of
assuming that every canonical input property must be daemon-forwarded.

After those accepted blockers are remediated, Cycle 2 should be a closed-world
verification of `P0098-A1-01`, `P0098-A1-02`, and `P0098-A1-04`, plus
`P0098-A1-03` only if the orchestrator accepts it as blocking after obtaining
the missing contract authority. Do not reopen broad discovery.

## Audit Effects

- Source changes: none.
- Plan changes: none.
- Runtime, browser, tenant, provider, scheduler, installed-service, release,
  commit, and network effects: none.
- Artifact written: this audit note only.

## Cycle 2 Closed-World Verification

Review mode: `closed_world`

Review cycle: 2 of 2, terminal

Revised target content SHA-256:
`6363bd2ce850f7736473a9e14c356c23d98a59b2d192c0cfe201736c49971a4b`

Scope was limited to `P0098-A1-01` through `P0098-A1-04` and critical
contradictions introduced by their remediation. No broad discovery was
reopened.

The revised property ledger was checked mechanically against the current 59
schema properties plus the three proposed canonical routing properties. It has
62 unique canonical entries, with no missing, extra, or duplicate canonical
property. Top-level `args` remains separately classified as the only
transport-legacy entry.

| Finding | Cycle 2 result | Exact verification evidence | Terminal disposition |
| --- | --- | --- | --- |
| `P0098-A1-01` | PASS | Plan version 2 adds `browserHost`, `viewStreamProvider`, and `controlInputProvider` to the 62-property canonical ledger with their existing MCP enums and `validation`, `trace`, `routing`, and `command` roles (`plan:276-305`). Contract repairs, file scope, tests, generated clients, docs, and acceptance criteria all require preserving these active fields (`plan:410-412,471-501,534-535,708-710`). Role-specific parity requires schema, MCP, normalizer, and generated sets to converge without deleting current behavior (`plan:350-375`). | Accepted blocker resolved. No residual blocking or nonblocking item. |
| `P0098-A1-02` | FAIL | The five safety markers are now explicitly validation-only and prohibited from command and trace output (`plan:287-289,365-366,413-415,619-620`), and the mechanical ledger coverage check passes. A residual contradiction remains: the ledger defines `command` to mean that the normalized command carries the property (`plan:269-274`), but tags structural `params` with `command` while specifying that `params` is flattened rather than carried (`plan:284`). Parity invariant 3 requires every `command`-tagged field to equal the canonical top-level forwarding set, while invariant 7 requires structural fields to stay out of ordinary forwarding tables (`plan:360-368`). The routing invariant also says ledger routing tags contain every field read by HTTP relay routing (`plan:363-364`), but current relay selection reads arbitrary nested `params` keys including `daemonSession`, `targetSession`, `targetSessionName`, and `sessionId` (`cli/src/native/stream/http.rs:1831-1847`), which cannot be canonical ledger entries because `params` is intentionally open. | Residual `blocking`. Remove the `command` role from `params` or redefine the role and parity set coherently. Scope routing parity to canonical top-level routing fields and separately preserve the existing open-`params` relay aliases with adapter fixtures. No further review cycle is authorized. |
| `P0098-A1-03` | PASS | The orchestrator resolved the finding conservatively as `nonblocking_backlog`. Plan version 2 keeps top-level `args` out of the canonical schema, MCP top-level input, normalizer, generated clients, and public cross-transport contract; HTTP alone strips and reapplies the raw value after normalization, while MCP retains `params.args` (`plan:261-265,317,369-375,416-419,462-487,632-649`). Negative parity and adapter tests are required, and the exact transport-legacy set is `{args}`. | Original evidence need resolved. Future canonicalization or deprecation remains explicit `nonblocking_backlog` outside Plan 0098; no active blocker. |
| `P0098-A1-04` | PASS | Plan version 2 freezes canonical top-level type, enum, positive-minimum, string-array item, explicit-null, and unknown-property enforcement for both adapters (`plan:324-348`). It explicitly authorizes rejection of previously raw-copied schema-invalid HTTP inputs while preserving the HTTP 400 and MCP `-32602` envelopes, and requires paired characterization plus exact issue/message placement tests before migration (`plan:340-348,420-434,529-538,611-612,714-717`). The sole unknown-property exception is the separately extracted HTTP legacy `args`. | Accepted blocker resolved. No residual blocking or nonblocking item. |

### Critical Remediation Contradictions

No critical contradiction was found outside the residual `P0098-A1-02` role and
parity inconsistency described above. The deep in-process module, two-adapter
seam, effect exclusions, id ownership, route-hint order, state loading, relay
ownership, queue handoff, and transport error-envelope ownership remain
coherent.

### Cycle 2 Terminal Recommendation

Final implementation-ready: **NO**.

Plan 0098 closes `P0098-A1-01`, `P0098-A1-03`, and `P0098-A1-04`, but the
role-specific parity contract is not implementable exactly while `params` is
both structural and tagged as a carried command property, and while canonical
routing parity claims coverage of arbitrary open-`params` relay aliases. Under
the two-cycle cap, this is a terminal residual blocker rather than grounds for
another discovery or remediation review loop. The execution packet should
remain blocked or be reframed with the two precise ledger corrections recorded
under `P0098-A1-02`.

### Cycle 2 Effects

- Source changes: none.
- Plan changes: none.
- Runtime, browser, tenant, provider, scheduler, installed-service, release,
  commit, and network effects: none.
- Artifact change: appended this Cycle 2 section to the existing audit note
  only.
