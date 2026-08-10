# Plan 0098 Work Audit | Service Request Normalization Deepening

Review mode: `drift_discovery`

Review cycle: 1 of 2

Reviewer role: independent work auditor

Branch: `architecture-deepening-20260809`

Repository base: `ae36b272327982e3227f4dc7c5d6dc5b4b16350c`

Frozen plan:
`docs/dev/plans/0098-2026-08-09-service-request-normalization-deepening-plan.md`

Frozen plan SHA-256:
`1579cbba4fcf398f5f8563b298ae9415f69a335e5e83d501b63dcba9dd2ab680`

Executor-reported target diff SHA-256:
`8d615786f01b0477a9e7e4bc03e1a92b43975b1ef3d85e996cfdd511f51dca04`

Audit date: 2026-08-09

## Scope And Packet Binding

This audit covers only the Candidate 1 implementation in these paths:

- `README.md`
- `cli/src/mcp.rs`
- `cli/src/native/mod.rs`
- `cli/src/native/service_request.rs`
- `cli/src/native/stream/http.rs`
- `cli/src/output.rs`
- `docs/dev/contracts/service-request.v1.schema.json`
- `docs/src/app/service-mode/page.mdx`
- `packages/client/src/service-request.generated.d.ts`
- `packages/client/src/service-request.generated.js`
- `scripts/check-service-api-mcp-parity.js`
- `scripts/test-service-request-client.js`
- `skills/agent-browser/SKILL.md`

Concurrent Plan 0099 dashboard changes and every plan or prior-audit artifact
were excluded. No implementation, plan, runtime, commit, or unrelated file was
modified by this review.

The executor did not supply the byte-stream construction command for the
reported aggregate diff hash. A normal tracked-file `git diff` cannot include
the untracked new module and therefore cannot reproduce that aggregate. The
review is instead bound to the current contents of the explicit path set above.
The per-file SHA-256 receipt is:

- `README.md`: `55789153c162ffec0fddd52fa4e715db7dc12b06a8261e630b57324d7bd89768`
- `cli/src/mcp.rs`: `e1e86d8b221c0f06889af17dec4fa58983c2caf04f72624dc9731c94315febb6`
- `cli/src/native/mod.rs`: `4a3bbcc420b07d4041b6e381d6a671048406ebd49a7ff81a0e409eb1ca40a343`
- `cli/src/native/service_request.rs`: `d667e5abea47a0fc97aed492c614422d4d13015619f4c9afa7cb89c3a922ffb3`
- `cli/src/native/stream/http.rs`: `09a094dbc451a753b9c9582f9f942c5b89cd91268afc6b7617d7a862a71d5702`
- `cli/src/output.rs`: `ae730ef5e2158684222bd8495742528d06c623f6a07e608fbf2d34cca5ca5558`
- `docs/dev/contracts/service-request.v1.schema.json`: `bc6031676b781458c2864206c3f0c0a2698f69367c4af60700d339f278e3f01f`
- `docs/src/app/service-mode/page.mdx`: `7408155957f707837881d5b9a264f9ef5b1d93e78eec3fb146ce0d57dc627359`
- `packages/client/src/service-request.generated.d.ts`: `09022afeefe5bdcf0f6e778e9c3ef1a1f78bb2f48f66ae4ad744656954295546`
- `packages/client/src/service-request.generated.js`: `5b1fde4109210f9c8b5b0222b988c4c967fb15630ff752c402e16b705d0d03d4`
- `scripts/check-service-api-mcp-parity.js`: `9e565dc06604918e12fb5c9263ccbea8b925fd355b884dae5e7f7130ffd2f87a`
- `scripts/test-service-request-client.js`: `38097fbc2f09a651b52117a6d4069c20f1ff7e01272a3abf34043d67e87ee299`
- `skills/agent-browser/SKILL.md`: `05d45f8b5189f99195a0b4b4e865e27022320095b3846a9e052f92c4515b856c`

## Review Method

CodeGraph was healthy at 422 indexed files, 14,537 nodes, and 43,764 edges.
It established the current normalizer call graph, route-hint dependencies, and
adapter ownership. Direct reads were used for the current edited source,
generated artifacts, JSON schema, docs, exact diff, and tests. The oversized
`cli/src/native/actions.rs` file was not part of the Candidate 1 write set.

The implementation was checked against all Plan 0098 acceptance criteria, its
62-property ledger, its bounded Cycle 2 correction, and the explicit
replace-don't-layer test requirement.

## Confirmed Implementation Properties

- `cli/src/native/service_request.rs:355-439` exposes one normalization
  operation and performs no file, network, queue, browser, or runtime I/O. It
  receives only the request and an optional read-only `ServiceState` snapshot.
- The normalizer owns canonical recognition, type and enum validation,
  action-specific safety gates, `params` flattening, reserved `id` and `action`
  protection, top-level precedence, command projection, trace projection, and
  route-hint sequencing.
- Route hints run in the frozen order: retained handoff first at
  `cli/src/native/service_request.rs:431-432`, then shared-profile planning at
  `cli/src/native/service_request.rs:433-435`.
- HTTP is a thin parsing and compatibility adapter at
  `cli/src/native/stream/http.rs:1620-1653`. It removes only top-level `args`,
  normalizes, adds the unchanged HTTP request-id prefix, then reapplies raw
  `args` so it overrides `params.args`.
- MCP is a thin JSON-RPC and identity adapter at `cli/src/mcp.rs:5439-5460`.
  It normalizes, adds the unchanged MCP request-id prefix, and preserves
  `params.args` while rejecting top-level `args`.
- HTTP relay-session selection, daemon revival, response writing, and queue
  relay remain in `cli/src/native/stream/http.rs:342-363` and the existing
  relay helpers. MCP state loading, configured-entity overlay, readiness
  refresh, JSON-RPC envelope mapping, and queue handoff remain in
  `cli/src/mcp.rs:5463-5477`.
- The 62 canonical property names match between schema, MCP, and the Rust
  table. The three routing fields are present in schema, MCP, generated client,
  docs, command, and trace. The five adopted validation-only fields are absent
  from top-level command and trace projection. Top-level `args` remains absent
  from canonical, MCP, and generated surfaces.
- The duplicate transport-local validator implementations were deleted. The
  new module has enough private implementation depth that deleting it would
  force both adapters to recreate validation, merging, trace projection, and
  route sequencing.

## Findings

### P0098-W1-01 | Blocking | Semantic tests were layered instead of replaced

Criterion:

Plan 0098 Slice 4, the Test Replacement Strategy, Acceptance Criterion 12, and
the done definition require action semantics to be tested at the normalizer
interface. HTTP and MCP must retain tests only for their real transport
variation. Duplicate adapter semantics tests must be deleted rather than kept
as two copies over the shared implementation.

Exact evidence:

- `cli/src/native/service_request.rs:1138-1294` contains only six normalizer
  tests.
- HTTP still has 34 functions named `service_request_command_*`; MCP still has
  31.
- The two adapter files retain at least 28 identical semantic test names,
  including the manual gate, CDP gates, evaluate, probe, UI action, network
  capture, file transfer, monitor freshness, tab-handle refresh, and
  shared-profile routing cases. These are visible beginning at
  `cli/src/native/stream/http.rs:4768` and `cli/src/mcp.rs:12332`.
- The required normalizer-interface coverage is absent for the complete action
  set, exact gate ordering, every action-specific recipe, stable issue kind and
  message per rejection, retained-handoff versus shared-profile precedence,
  explicit route and duplicate-lane short circuits, and the full valid and
  invalid cross-adapter fixture matrix.
- Existing adapter assertions commonly check `contains(...)` on a builder
  error. They do not bind each shared message inside an actual HTTP 400 response
  and JSON-RPC `-32602` envelope as required by the frozen error compatibility
  section.

Reproducer:

```bash
rg -c '^    fn service_request_command_' cli/src/mcp.rs cli/src/native/stream/http.rs
rg -c '^    #\[test\]' cli/src/native/service_request.rs
comm -12 <(rg -o 'fn service_request[^ (]+' cli/src/mcp.rs | sed 's/^fn //' | sort -u) <(rg -o 'fn service_request[^ (]+' cli/src/native/stream/http.rs | sed 's/^fn //' | sort -u)
```

Consequence:

The runtime source has one semantic authority, but the proof surface still has
two adapter-shaped semantic suites and only a small normalizer suite. A future
semantic change can require synchronized edits in both adapter suites, while
the shared issue contract, route order, and cross-transport equality remain
under-specified. Acceptance Criterion 12 and the frozen replace-don't-layer
control are not satisfied.

Confidence: high.

Suggested disposition: `blocking`.

### P0098-W1-02 | Blocking | The promised role-specific parity gate is incomplete

Criterion:

Plan 0098 requires mechanical proof of all canonical property roles and types:
schema, MCP, normalizer recognition, command projection, trace projection,
routing consumption, validation-only fields, structural fields,
transport-legacy exclusions, and generated-client fields and types.

Exact evidence:

- `scripts/check-service-api-mcp-parity.js:382-393` compares only property
  names among schema, MCP, and the normalizer.
- `scripts/check-service-api-mcp-parity.js:394-400` checks generated presence
  only for the three newly added string fields. It does not compare the full
  generated field and type sets with all 62 schema properties.
- `scripts/check-service-api-mcp-parity.js:401-421` checks top-level `args`
  exclusion and the two HTTP compatibility source needles. It does not compare
  command, trace, routing, validation-only, or structural sets.
- `ServiceRequestFieldSpec.routing` at
  `cli/src/native/service_request.rs:69-76` has no production or parity
  consumer. Its sole read is the weak assertion at
  `cli/src/native/service_request.rs:1169-1172` that every routing field is also
  a command field. That assertion cannot detect a missing routing tag.
- The mismatch is already observable: shared-profile route projection reads
  `serviceName`, `agentName`, and `taskName` at
  `cli/src/native/service_access.rs:1074-1096`, while their ledger entries in
  `cli/src/native/service_request.rs` carry `routing: false`. Current command
  forwarding happens to preserve the values, but the declared routing set does
  not contain every canonical top-level input read by that route projection.
- The parity gate reports success despite this mismatch, demonstrating that it
  does not prove the frozen routing invariant.

Reproducer:

```bash
rg -n '\.routing|routing:' cli/src/native/service_request.rs scripts/check-service-api-mcp-parity.js
sed -n '1070,1101p' cli/src/native/service_access.rs
pnpm test:service-api-mcp-parity
```

Consequence:

The current runtime paths happen to receive all three caller-context values
because they are command-forwarded, but the architectural ledger and its guard
can drift without failing CI. The requested proof of all 62 field roles and
types is absent, and Acceptance Criterion 11 is not satisfied.

Confidence: high.

Suggested disposition: `blocking`.

### P0098-W1-03 | Needs evidence | Aggregate diff identity is not reproducible

Criterion:

The audit should bind its judgment to the executor-reported target diff and
exclude later concurrent edits.

Exact evidence:

The reported SHA-256 names a combined diff containing 12 tracked files plus an
untracked new module, but no construction command or byte ordering was
provided. A standard `git diff` over the named tracked paths produces a
different stream because it necessarily omits the untracked module. No current
Candidate 1 path showed evidence of a concurrent dashboard edit, and the
per-file content hashes above provide an exact replacement receipt for this
review.

Reproducer:

```bash
git status --short
git diff -- README.md cli/src/mcp.rs cli/src/native/mod.rs cli/src/native/stream/http.rs cli/src/output.rs docs/dev/contracts/service-request.v1.schema.json docs/src/app/service-mode/page.mdx packages/client/src/service-request.generated.d.ts packages/client/src/service-request.generated.js scripts/check-service-api-mcp-parity.js scripts/test-service-request-client.js skills/agent-browser/SKILL.md | sha256sum
git diff --no-index /dev/null cli/src/native/service_request.rs | sha256sum
```

Consequence:

This does not identify a source defect, but the aggregate receipt cannot by
itself prove that Cycle 2 reviews the identical byte stream. The explicit path
and per-file hash receipt must remain the review binding unless the executor
supplies the construction command.

Confidence: high.

Suggested disposition: `needs_evidence`, non-code.

## Independent Validation

The auditor ran these checks against the current scoped content:

- `cargo test --manifest-path cli/Cargo.toml service_request -- --test-threads=1`
  passed 89 tests.
- `pnpm test:service-api-mcp-parity` passed and reported 96 service-request
  actions, but the structural gap in `P0098-W1-02` explains why that pass is not
  sufficient acceptance evidence.
- `pnpm test:service-client` passed.
- `node scripts/generate-service-request-client.js --check` passed.
- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.
- `pnpm --dir docs build` passed. Its multiple-lockfile workspace-root warning
  was nonfatal and unrelated to Candidate 1.
- `git diff --check` passed.

The executor's unrelated Rust baseline claim was independently reproduced:

```bash
cargo test --manifest-path cli/Cargo.toml native::cdp::chrome::tests::test_headed_display_fallback_not_used_when_display_set -- --exact --test-threads=1
```

The exact test failed with `left: Some(":9")` and `right: None`.
`cli/src/native/cdp/chrome.rs` has no diff from
`ae36b272327982e3227f4dc7c5d6dc5b4b16350c`, so this display-environment
baseline is not attributed to Candidate 1. A raw parallel `cargo test` run was
not used as acceptance evidence because this repository separates
environment-mutating modules; one observed monitor test failure passed when
reproduced alone with one test thread. The dedicated test role owns final broad
validation after remediation.

No browser, installed runtime, tenant state, provider, scheduler, service,
release, commit, or network effect was used.

## Cycle 1 Verdict

Implementation-ready for terminal acceptance: **NO**.

The production architecture has the intended deep normalizer and thin
adapters, and focused executable checks pass. Two frozen acceptance controls
remain blocking: test semantics were not replaced at the new interface, and
the role-specific 62-field parity proof is not implemented. These are bounded
proof-architecture corrections, not grounds to redesign the normalizer or
reopen broader discovery.

## One Bounded Remediation Packet

1. Move the duplicated action-specific, safety-gate, issue-kind/message, merge,
   trace, and route-order fixtures into `service_request.rs`. Retain in HTTP
   only parsing, raw top-level `args`, request-id, actual 400 envelope, relay,
   and revival tests. Retain in MCP only request-id, actual `-32602` envelope,
   state overlay/readiness, trace-to-queue handoff, and top-level `args`
   rejection tests.
2. Add one table-driven cross-adapter valid/invalid fixture matrix. Normalize
   request IDs for valid comparisons and assert exact transport envelopes for
   invalid comparisons.
3. Expand the parity gate and Rust role tests to compare all 62 property names,
   schema types and constraints, MCP types and constraints, generated fields
   and types, command, trace, routing, validation-only, structural, and the
   exact `{args}` transport-legacy set. Correct the routing taxonomy for
   canonical fields read by route projection, or explicitly define and test a
   narrower selection-only role if that is the intended authority.
4. Re-run the focused Rust, parity, client, generator, format, strict Clippy,
   docs, and patch checks. Cycle 2 must be `closed_world` and limited to
   `P0098-W1-01`, `P0098-W1-02`, the non-code receipt in `P0098-W1-03`, and
   critical regressions introduced by these fixes.

No third review cycle is authorized.

## Audit Effects

- Implementation changes: none.
- Plan or prior-audit changes: none.
- Runtime or external effects: none.
- Artifact written: this work-audit note only.

## Cycle 2 Closed-World Verification

Review mode: `closed_world`

Review cycle: 2 of 2

Verification scope: accepted findings `P0098-W1-01`, `P0098-W1-02`, and
`P0098-W1-03`, plus critical regressions introduced by their bounded
remediation. No broad discovery was reopened.

### Reproducible Target Identity

Cycle 2 is bound to the executor's 16-path, path-sorted binary diff stream
against `ae36b272327982e3227f4dc7c5d6dc5b4b16350c`. The stream includes new
files through `git diff --no-index --binary /dev/null <path>` and tracked files
through `git diff --binary --no-ext-diff <base> -- <path>`.

Independent reproduction returned:

```text
45db30cc7f36d47ccd34f0467e202b9149b36fa1319f74ddf1eb1906b18303d9  -
```

The bound paths are `README.md`, `cli/src/mcp.rs`, `cli/src/native/mod.rs`,
`cli/src/native/service_access.rs`, `cli/src/native/service_request.rs`,
`cli/src/native/stream/http.rs`, `cli/src/native/stream/mod.rs`,
`cli/src/output.rs`,
`docs/dev/contracts/service-request-field-roles.v1.json`,
`docs/dev/contracts/service-request.v1.schema.json`,
`docs/src/app/service-mode/page.mdx`,
`packages/client/src/service-request.generated.d.ts`,
`packages/client/src/service-request.generated.js`,
`scripts/check-service-api-mcp-parity.js`,
`scripts/test-service-request-client.js`, and
`skills/agent-browser/SKILL.md`.

Disposition for the earlier receipt uncertainty: resolved. The Cycle 1 target
hash is historical; this reproducible remediation target is the Cycle 2
acceptance identity.

### P0098-W1-01 | Pass | Test semantics were replaced at the deep module interface

The remediation deletes the duplicated HTTP and MCP action-semantics suites
and concentrates their behavior at the normalizer interface:

- `cli/src/native/service_request.rs` now has 12 interface tests covering all
  96 supported actions, exact issue kind and message fixtures, safety-gate
  order, canonical types and nulls, reserved fields and precedence, the five
  validation-only markers, command and trace projection, route-hint order and
  short circuits, and a table-driven cross-adapter valid and invalid matrix.
- HTTP retains nine service-request tests for parsing and request identity,
  raw top-level `args`, the exact 400 envelope, retained-daemon revival, and
  relay selection.
- MCP retains five service-request tests for full tool-schema parity, request
  identity and trace handoff, top-level versus nested `args`, queue-session
  routing, and retained-handoff queue ownership.
- The only names shared between the adapter files are the production/helper
  names `service_request_adapter_fixture`, `service_request_command`, and
  `service_request_command_with_state`; no duplicated semantic test names
  remain.

Focused evidence:

```text
native::service_request::tests: 12 passed, 0 failed
service_request filter: 35 passed, 0 failed
```

Residual disposition: none for `P0098-W1-01`.

### P0098-W1-02 | Fail | Routing-consumer parity is still incomplete

Most of the accepted finding is corrected:

- the machine-readable role contract declares 62 canonical properties with
  role counts of 2 structural, 56 command, 52 trace, 28 routing, and 5
  validation-only fields; its role union is exactly 62 and its sole
  transport-legacy member is `args`;
- Rust compares the schema property set and type constraints with the
  normalizer table, compares every Rust role set with the machine-readable
  ledger, and compares the access-plan routing-consumer constant with the
  routing role;
- an MCP test compares all 62 actual tool properties with canonical type,
  minimum, enum, object, and array-item constraints;
- the JavaScript parity gate compares all generated field names, types, and
  optionality with the schema;
- `serviceName`, `agentName`, and `taskName` now carry the routing role and are
  present in `SERVICE_REQUEST_ACCESS_PLAN_ROUTING_FIELDS`.

One direct instance of the accepted routing-consumer criterion remains. The
frozen invariant at Plan 0098 lines 364-367 requires the routing role to
contain every canonical top-level field read by HTTP relay routing.
`service_request_relay_session` reads
`/serviceTabHandle/sessionName` and `/serviceTabHandle/browserId` at
`cli/src/native/stream/http.rs:1691-1692`. `serviceTabHandle` is a canonical
top-level property, but its Rust field specification has `routing: false` at
`cli/src/native/service_request.rs:183`, and it is absent from the routing role
in `docs/dev/contracts/service-request-field-roles.v1.json:35-40`.

The current mechanical proof does not cover that consumer. The Rust assertion
at `cli/src/native/service_request.rs:1240-1246` checks only that entries in
`SERVICE_REQUEST_ACCESS_PLAN_ROUTING_FIELDS` appear in the routing ledger. It
does not enumerate HTTP relay's canonical reads. The JavaScript gate compares
the role file with schema and generated surfaces and explicitly checks only
`serviceName`, `agentName`, and `taskName` as routing members. It therefore
passes while the frozen HTTP-relay routing invariant remains false.

Reproducer:

```bash
jq -e '(.roles.routing | index("serviceTabHandle")) == null' \
  docs/dev/contracts/service-request-field-roles.v1.json
sed -n '1680,1705p' cli/src/native/stream/http.rs
sed -n '175,186p' cli/src/native/service_request.rs
pnpm test:service-api-mcp-parity
```

Consequence: the new ledger and parity tests can certify success while omitting
a canonical field that actively selects an HTTP daemon lane. Runtime behavior
is still present, so this is a proof-architecture defect rather than evidence
of current wire drift, but Acceptance Criterion 11 is not satisfied.

Residual disposition: `blocking`. This is not reopened discovery; it is a
remaining case of the accepted `P0098-W1-02` routing-consumer finding.

### P0098-W1-03 | Pass | Aggregate identity is reproducible

The executor supplied the exact ordered construction command, explicit path
list, base revision, and target SHA-256. Independent reproduction matched
`45db30cc7f36d47ccd34f0467e202b9149b36fa1319f74ddf1eb1906b18303d9`.

Residual disposition: none for `P0098-W1-03`.

### Critical Regression Check

No critical runtime, command-shape, error-envelope, request-id, queue, state,
or adapter-ownership regression introduced by the remediation was found in the
closed scope. The focused 35-test service-request slice passed. The retained
`P0098-W1-02` defect concerns the completeness of mechanical routing proof,
not a newly introduced runtime behavior change.

### Cycle 2 Validation

- Target identity reproduction passed with the exact expected SHA-256.
- `cargo test --manifest-path cli/Cargo.toml native::service_request::tests -- --test-threads=1`
  passed 12 tests.
- `cargo test --manifest-path cli/Cargo.toml service_request -- --test-threads=1`
  passed 35 tests.
- `pnpm test:service-api-mcp-parity` passed for 96 service-request actions, but
  the reproducer above demonstrates why that pass is insufficient for
  `P0098-W1-02`.
- `node scripts/generate-service-request-client.js --check` passed.

The broad Rust suite and the unrelated display-environment baseline were not
rerun. Final broad validation remains owned by the distinct tester.

### Final Work Acceptance

Candidate 1 accepted: **NO**.

`P0098-W1-01` and `P0098-W1-03` pass. `P0098-W1-02` remains blocking because
full machine-readable routing-consumer parity is not proved. No third review
cycle is authorized.

The bounded residual correction is:

1. add `serviceTabHandle` to the Rust and machine-readable routing roles;
2. mechanically enumerate the canonical top-level fields consumed by HTTP
   relay routing, including the `serviceTabHandle` lane source, and assert that
   set against the routing ledger alongside the access-plan consumer set;
3. add an exact HTTP relay fixture for a top-level `serviceTabHandle`, then run
   the same focused Rust, parity, generation, and patch checks.

### Cycle 2 Effects

- Implementation, plan, runtime, commit, and external effects: none.
- Unrelated Plan 0099 dashboard work and all other concurrent artifacts:
  untouched and excluded.
- Audit effect: this Cycle 2 section was appended to the existing Candidate 1
  work-audit note.
