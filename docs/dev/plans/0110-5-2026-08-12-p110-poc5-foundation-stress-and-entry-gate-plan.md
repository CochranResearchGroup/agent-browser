# Plan 0110-5 | P110 PoC 5 Foundation Stress And Entry Gate

Date: 2026-08-12

State: DETAILED | IMPLEMENTATION READY

Authority: SOURCE-ONLY | SYNTHETIC PROVIDERS | NO LIVE FOUNDATION PROOF

Lane: P110 PoC 5

Parent: `docs/dev/plans/0110-2026-08-12-desktop-perception-interaction-foundation-plan.md`

Predecessor: PoC 4 source accepted at `987923c7`

## Objective

Stress the source foundation through one provider-neutral named interaction
recipe across every advertised ingress, safe dashboard and durable
projections, authority conflicts, partial effects, cleanup, verification,
replay after restart, prompt intervention, and opaque human handoff. Produce a
typed entry-gate decision for later discrete use-case planning.

This proof adds no real capture, detector, prompt, input, browser, or remote
desktop provider.

## Frozen Decisions

1. Reuse canonical `desktop_interact`; add named recipe
   `p110-foundation-stress-v1` beside `p110-pointer-keyboard-v1`.
2. Add required `operationId` to `desktop_interact`. It is a caller-generated
   opaque idempotency key, not authority and not a daemon selector.
3. Replay scope is accountable principal plus operation ID plus canonical
   recipe/request hash. Same scope and hash returns the durable receipt;
   mismatched hash returns `desktop_interaction_operation_conflict`.
4. Transport `requestId` remains trace identity. CLI, HTTP, generic MCP,
   dedicated MCP, and generated client may use different transport IDs while
   sharing one operation ID.
5. The stress recipe is provider-neutral: the same engine runs deterministic
   fixture and fault-injection providers without changing recipe semantics.
6. Public configured dispatch remains `desktop_input_provider_unavailable`
   before action-dispatch effects.
7. Source proof uses process-owned event fencing plus a durable operation
   ledger. It does not claim cross-process OS input fencing. A real provider
   requires a separately designed external fence before live use.
8. The durable handoff result contains only existing opaque `handoffId` and
   authenticated `handoffUrl`, readiness, and reason. It never creates a
   handoff, runs another remote-view open, or exposes provider/Guacamole URLs.
9. Dashboard/job/stream projections are immutable receipt summaries. They do
   not depend on mutable selected-record state.
10. PoC 5 source acceptance opens planning-only use-case entries. Use-case
    implementation remains blocked until a separately authorized controlled
    RDP/Guacamole recipe passes with an installed binary and real provider.

## Request And Response

```json
{
  "action": "desktop_interact",
  "browserId": "browser-1",
  "sessionName": "optional",
  "controllerLeaseId": "lease-1",
  "operationId": "operation-1",
  "recipe": { "recipeId": "p110-foundation-stress-v1" },
  "serviceName": "FoundationStress",
  "agentName": "fixture-agent",
  "taskName": "stress-fixture"
}
```

Existing response remains `{ok, action, interactionReceipt}`. The stress
receipt adds `operationId`, `operationRequestSha256`, `recipeProviderId`,
`recipeProviderVersion`, optional `promptDisposition`, optional
`humanHandoff`, and `entryGate`.

`entryGate` is one of:

- `planning_open_implementation_blocked` after all source scenarios pass;
- `closed_source_failure` when a source scenario fails;
- `closed_live_evidence_required` for any implementation or live claim.

## Durable Operation Ledger

Add an injected `InteractionOperationLedger` with begin, complete, and lookup.
The production adapter uses service-owned persisted state or a dedicated
versioned state record and atomic save semantics. Tests reload the serialized
ledger into a fresh engine instance.

States are `in_progress | complete | uncertain`. Complete and uncertain
receipts are replay terminal. An abandoned in-progress record fails typed; it
never emits automatically. Store only the redacted receipt and request hash.

## Stress Scenarios

Pin deterministic manifests for:

- verified success and identical replay;
- locator ambiguity and not found;
- stale frame and geometry;
- focus loss before and after acknowledgement;
- route or display replacement;
- controller conflict and takeover cancellation;
- provider unavailable;
- move, down, up, key, and emergency-cleanup failure;
- verification failure and unavailable after evidence;
- prompt operator intervention with zero input;
- opaque durable handoff required after uncertain effect;
- replay after ledger reload;
- same operation ID with a different canonical hash;
- unrelated routes and operations remaining independent.

Every manifest pins provider version, canonical request hash, expected provider
call count, event order, authority epoch, effect state, cleanup state,
verification state, replay state, projection hash, and handoff state.

## Ingress And Dashboard

- CLI requires `--operation-id` for `desktop interact` and accepts either
  registered recipe.
- HTTP and MCP route by `sessionName` only.
- generated client helpers validate operation ID and both recipe IDs.
- dashboard projections show recipe, typed outcome, cleanup, verification,
  replay, intervention, and opaque handoff readiness from the selected receipt
  record, without pixels, text, paths, raw routes, or provider URLs.
- durable jobs and streams use one strict redactor.

## Privacy

Durable state may contain opaque IDs, non-secret hashes, typed states, safe
bounds, authority epochs, acknowledgement IDs, provider IDs/versions, and
opaque handoff identity. It excludes pixels, DOM/OCR/prompt text, titles,
plaintext input, full paths, stderr, display names, filesystem paths,
credentials, and raw provider or Guacamole URLs.

## Implementation Slices

### Slice A | Durable Stress Engine

- extend recipe registry, receipt, operation ledger, provider abstraction,
  prompt/handoff summaries, deterministic scenarios, and restart replay tests.

### Slice B | Ingress And Contracts

- add operation ID to canonical schemas, CLI/HTTP/MCP/client normalization,
  both recipe IDs, safe persistence, metadata, and cross-ingress parity.

### Slice C | Dashboard And Documentation

- add safe receipt projection assertions and documentation;
- keep raw remote-view routing and sensitive evidence excluded.

### Slice D | Audit And Source Acceptance

- run one fresh audit, adjudicate once, perform at most one remediation packet,
  run closed-world verification, and record source status and entry-gate state.

## Required Tests

1. all stress manifests and canonical receipts are pinned and deterministic;
2. fixture and fault providers run one unchanged recipe;
3. same operation scope/hash replays after ledger reload with zero events;
4. same operation scope with another hash fails and never emits;
5. in-progress restart state fails closed;
6. every pre-effect failure emits zero input and every post-effect failure
   stores an uncertain receipt with exactly one bounded cleanup attempt;
7. route/display/controller/focus/geometry drift stops before the next event;
8. prompt intervention emits zero input and yields a safe operator outcome;
9. uncertain effects project only an opaque existing handoff identity;
10. CLI, HTTP, generic MCP, dedicated MCP, and client normalize identical
    operation identity and sessionName-only routing;
11. stream, job, event, incident, dashboard, error, and ledger projections
    exclude every forbidden sentinel;
12. configured provider absence precedes public dispatch effects;
13. PoC 1 through 4, controller, handoff, screenshot, service, and dashboard
    regressions pass;
14. source acceptance yields only planning-open, implementation-blocked.

## Validation

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml foundation_stress -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_interaction -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_prompt -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_locator -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_capture -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml remote_view_handoff -- --test-threads=1
pnpm test:service-client
pnpm test:service-api-mcp-parity
pnpm test:service-contracts-no-launch
pnpm test:dashboard-inspector-actions
pnpm test:dashboard-workspace-view-projection
pnpm build:dashboard
pnpm test:actions-architecture
pnpm test:wsl-cargo-safety
pnpm --dir docs build
git diff --check
```

No ignored E2E, browser, display, RDP, Guacamole, CDP, prompt, credential,
challenge, external process, network provider, or OS input smoke is authorized.

## Hard Stops

- Stop if replay can emit twice or request mismatch can reuse a receipt.
- Stop if process-local fencing is called cross-process provider fencing.
- Stop if route replacement directs input or cleanup to the replacement route.
- Stop if ambiguity, staleness, focus loss, prompt intervention, or authority
  drift permits the next input.
- Stop if partial effects lose receipts or become automatically retryable.
- Stop if acknowledgement is treated as verification.
- Stop if raw provider routing or sensitive evidence enters durable state.
- Stop if a real provider or live dependency enters source acceptance.
- Stop if P110 source acceptance is called live foundation acceptance.

## Reconnaissance Receipt

Three read-only lanes inspected deep architecture, ingress/dashboard/handoff,
and stress/entry-gate evidence. Graphiti returned older remote-view stress
context only; current source and PoC acceptance records control. The primary
accepted reuse of `desktop_interact`, explicit operation identity, durable
replay, provider-neutral scenarios, opaque handoff projection, and a
planning-only use-case gate. Cross-process OS fencing remains a future live
provider prerequisite, not a PoC 5 source claim.

## Acceptance

PoC 5 is source accepted only when all fourteen requirements pass, one fresh
audit has no unresolved blocker after one remediation packet, and the entry
gate is exactly `planning_open_implementation_blocked`.
