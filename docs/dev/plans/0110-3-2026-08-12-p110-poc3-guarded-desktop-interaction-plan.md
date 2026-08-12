# Plan 0110-3 | P110 PoC 3 Guarded Desktop Interaction

Date: 2026-08-12

State: SOURCE ACCEPTED

Authority: SOURCE-ONLY | PROVIDER-FREE SYNTHETIC INPUT | NO LIVE INPUT

Lane: P110 PoC 3

Parent: `docs/dev/plans/0110-2026-08-12-desktop-perception-interaction-foundation-plan.md`

Predecessor: PoC 2 source accepted at `eefcb9bd`

## Objective

Execute one bounded, replayable observe, locate, act, and verify transaction
against the repository-owned synthetic desktop fixture. The transaction must
derive its target from a fresh PoC 2 observation, prove current controller,
focus, surface, coordinate, and geometry authority, emit a deterministic
pointer arc followed by one left click and fixed non-sensitive keyboard input,
guarantee release cleanup, capture fresh after-state evidence, and return a
typed `InteractionReceipt`.

PoC 3 has exactly one effect sink: an injected in-memory synthetic fixture
provider used by provider-free tests. It does not add or invoke X11,
`xdotool`, Guacamole, VNC, WebRTC, CDP Input, platform accessibility, or an
operating-system mouse or keyboard provider. Configured production dispatch
fails closed before capture, controller mutation, or input with
`desktop_input_provider_unavailable`.

## Reconciled Existing Architecture

- PoC 1 resolves exact browser, session, profile, display allocation, view
  stream, route, operator-visible proof, physical-pixel geometry, and frame
  identity in `native::desktop_capture`.
- PoC 2 atomically captures and locates a selected synthetic control in
  `native::desktop_locator`. `not_found` and `ambiguous` observations have no
  selected candidate and are hard stops for input.
- `RemoteViewRoute`, `ViewStream`, and `ViewerLease` retain controller
  identity, but the current lease model is a remote-view viewer contract, not
  yet a sufficient machine-input capability.
- takeover replaces the route and stream primary controller IDs, but the
  former lease can remain `viewerRole=controller` and `state=controlling`.
  Lease state alone therefore never authorizes input.
- current controller IDs can be caller-selected and reused. A route-scoped
  monotonic controller epoch is required to prevent ABA reuse.
- current native mouse and keyboard paths are CDP page input. Dashboard
  Guacamole forwarding is a human viewer path. Neither is a PoC 3 input sink.
- service requests already attach transport-proven `callerId`, `requestId`,
  and principal source metadata after canonical normalization. The request ID
  becomes this action's idempotency identity and cannot be forged through the
  public schema.

## Frozen Public Contract

### Canonical Action

`desktop_interact` is one atomic queued effect action. It accepts service-owned
identity, an existing controller lease, and one named repository recipe. It is
not a general desktop event interpreter.

Request shape:

```json
{
  "action": "desktop_interact",
  "browserId": "browser-1",
  "sessionName": "optional",
  "controllerLeaseId": "viewer:route:fixture-agent:opaque",
  "recipe": {
    "recipeId": "p110-pointer-keyboard-v1"
  },
  "serviceName": "DesktopInteractor",
  "agentName": "fixture-agent",
  "taskName": "verify-synthetic-control",
  "jobTimeoutMs": 30000
}
```

All three attribution labels are required. Only `sessionName`, never
`browserId`, may narrow daemon routing. The transport-proven caller and request
IDs remain internal command metadata.

The sole PoC recipe owns:

- locator profile `p110-control-v1`;
- accepted target class and matched-only selection policy;
- pointer algorithm, duration, step, click, and hold limits;
- one fixed lowercase ASCII test text with a maximum of 32 characters;
- focus and surface requirements;
- before and after detector profiles;
- expected fixture transition;
- one effect attempt and no automatic retry.

The caller cannot supply a frame, context, observation, candidate, coordinate,
path, motion seed, timing, button, key, text, clipboard content, locator,
template, OCR evidence, threshold, retry, verifier, display, route, stream,
provider, provider URL, focus action, takeover action, lease expiry, asset,
file, or output path. `params` is rejected for this action so there is one
canonical authority source.

CLI spelling:

```text
agent-browser desktop interact --browser-id <id> \
  --controller-lease-id <id> --recipe-id p110-pointer-keyboard-v1
```

The optional global session selector lowers to `sessionName`. There are no raw
input or coordinate flags.

### Controller Preconditions

`desktop_interact` never requests, grants, renews, releases, or takes over a
controller lease. A separate existing lease action establishes the current
controller before the interaction request.

The action proves all of the following before claiming the route and again at
every event boundary:

1. PoC 1 browser, session, profile, route, stream, display allocation,
   readiness, visible-content, map-key, record-ID, and geometry predicates
   still hold.
2. Route and stream are writable and declare the same nonempty machine input
   capability. Manual attached-desktop control is not machine capability.
3. Route and stream primary controller IDs both equal the supplied
   `controllerLeaseId`.
4. The lease exists under that exact map key and its record ID matches.
5. Both route and stream viewer-lease lists contain the lease ID.
6. Lease route and browser IDs match the resolved desktop context.
7. Lease role is `controller`, state is `controlling`, and its nonempty expiry
   parses and is later than the transaction clock.
8. Lease viewer identity matches the accountable agent identity required by
   the named machine recipe.
9. Route and stream controller epochs are equal and unchanged.
10. The authority digest binds controller epoch, lease ID and update time,
    browser, route, stream, display allocation, geometry epoch, caller ID, and
    request ID.

Add `controllerEpoch` with a compatibility default to service-owned route and
stream records. Increment it whenever the primary controller is granted,
taken over, released, expired, reconciled away, or cleared during route
lifecycle cleanup. Stream projection copies the authoritative route epoch.
The epoch is never caller-controlled.

### Human Takeover Serialization

Add a process-owned per-route `DesktopControlCoordinator` shared by controller
grant, takeover, release, reconciliation, and interaction paths.

- one interaction claim exists per route;
- each emitted event is protected by a short coordinator event guard;
- takeover waits for an in-flight atomic event, marks the interaction claim
  cancelled, advances the persisted controller epoch, then grants control;
- interaction re-reads persisted authority under the event guard before the
  next event and stops without another emission after cancellation or drift;
- two machine interactions for the same route never interleave;
- unrelated routes remain independent;
- route cleanup cancels and drains the claim before clearing controller state.

This is a source proof for one service process and a synthetic sink. Cross-
process fencing and a real provider remain PoC 5 readiness work. No source
acceptance claim may imply that the coordinator already guards an external OS
provider.

### Response And Receipt

Response shape:

```json
{
  "ok": true,
  "action": "desktop_interact",
  "interactionReceipt": {}
}
```

`InteractionReceipt` v1 contains:

- transaction ID derived from transport request identity;
- schema version, recipe ID, version, content SHA-256, and fixed budgets;
- authority decision, controller epoch and digest, accountable actor digest,
  and route/browser/stream/display IDs;
- before context, frame receipt, observation IDs and hashes, and selected
  candidate evidence without frame bytes;
- focus and surface identity digest, browser process identity digest, pointer
  start, physical-pixel bounds, coordinate mapping receipt, and freshness;
- motion profile/version, control-point digest, event count, duration, and
  emitted-path SHA-256;
- ordered input acknowledgements and cleanup receipt;
- fixed text length and SHA-256, never plaintext;
- after context, frame receipt, observation or verifier evidence IDs and
  hashes without frame bytes;
- verification state, effect state, stop reason, timestamps, and ephemeral
  retention posture.

Before the first acknowledged input event, a failed precondition returns a
typed action error and no receipt needs to imply an effect. Once any input is
acknowledged, every failure returns an `InteractionReceipt` with `ok=false`
and an explicit `effectState`; the service must not discard partial-effect
evidence behind a transport error.

Effect states are:

- `no_effect` when no input was acknowledged;
- `verified_success` only after the after-state verifier passes;
- `effect_uncertain` after acknowledged input when cleanup, after-capture, or
  verification does not establish the intended terminal state;
- `cancelled_after_effect` when takeover or authority change stops a partially
  emitted transaction after safe release.

Repeating the same transport `callerId` and `requestId` returns the existing
redacted receipt or the in-progress state. It never emits a second event plan.

## Deep Module Boundary

Add `native::desktop_interaction` with a pure planner and injected capabilities:

```text
DesktopInteractionRequest
  + DesktopCaptureProvider
  + DesktopLocator
  + ControllerAuthorityRepository
  + DesktopControlCoordinator
  + DesktopFocusProvider
  + DesktopInputProvider
  + DesktopVerifier
  + Clock
               |
               v
InteractionReceipt
```

The public handler resolves the configured input provider first. The PoC 3
production resolver always returns unavailable. Provider-free tests call the
deep engine with `SyntheticFixtureDesktopProvider`, which captures, locates,
reports focus and pointer state, consumes a guarded event plan, mutates only an
in-memory fixture, and supplies deterministic after frames.

The input provider owns two operations:

```text
probe(binding) -> SurfaceSnapshot
execute_event(binding, expected_surface, event) -> EventAcknowledgement
```

Every event execution checks the stable surface/window identity, browser
process identity digest, focus, dimensions, scale, coordinate space, geometry
epoch, bounds, and provider identity. The orchestrator never focuses a window
by clicking and never applies the frame scale factor twice.

## Deterministic Motion And Input

Coordinates remain `desktop_physical_pixels`. PoC 2 selected centers map by
identity to the synthetic input provider. Any other coordinate-space claim is
unsupported in this proof.

Generate one fixed-point cubic Bézier trajectory:

1. `P0` is the probed current pointer and `P3` is the selected candidate
   center.
2. Distance uses checked integer square root over checked squared deltas.
3. Steps are `clamp(ceil(distance / 12), 6, 64)` and duration is
   `clamp(140 + distance / 3, 160, 650)` milliseconds.
4. A SHA-256 over recipe hash, before frame ID, candidate ID, `P0`, and `P3`
   selects bend side and bounded perpendicular magnitude.
5. `P1` and `P2` use one-third and two-thirds progress plus that bend. Bend is
   deterministically reduced until both controls fit display bounds.
6. Each uniform time step uses integer smoothstep `3s^2 - 2s^3` followed by
   checked fixed-point Bézier evaluation and round-half-away-from-zero.
7. Consecutive duplicate points are removed; exact endpoints are retained;
   every emitted point stays in bounds. Distances below four pixels use the
   exact endpoint path.

The event sequence is bounded to 64 pointer moves, one `left_down`, one
`left_up`, and the recipe-owned text of at most 32 characters. Click hold is a
deterministic 45 to 90 milliseconds derived from the same digest. Keyboard
input permits only lowercase ASCII letters, digits, space, and hyphen with
explicit key down and key up and a deterministic 35 to 65 millisecond delay.
Enter, Tab, Escape, modifiers, shortcuts, clipboard, drag, double-click,
right-click, middle-click, wheel, and arbitrary text are unsupported.

Immediately before `left_down`, before each key down, and before after capture,
the engine rechecks controller epoch, cancellation, focus, surface, and
geometry. The before frame must remain no older than a fixed 750 milliseconds
at the last pre-press check.

An acknowledged down event arms a release guard. On any later failure, the
provider attempts the matching up event exactly once. A release failure yields
`desktop_input_cleanup_failed`, `effect_uncertain`, no retry, and a receipt
that distinguishes the failed primary release from the emergency attempt.

## Verification

After all releases and one final authority, focus, and geometry check, capture
one fresh PoC 1 frame from the same context. Browser, session, route, stream,
display allocation, dimensions, scale, coordinate space, and geometry epoch
must still agree.

A repository-owned verifier proves the synthetic control changed to the exact
activated state and contains evidence for the fixed text hash. Verification
does not infer success from input acknowledgements alone.

- passed verification yields `verified_success`;
- unchanged, decoy, not-found, or ambiguous after-state yields
  `desktop_interaction_verification_failed` and `effect_uncertain`;
- after-capture or binding loss yields
  `desktop_interaction_verification_unavailable` and `effect_uncertain`;
- neither failure automatically repeats input.

## Typed Failures

Preserve stable codes for at least:

- `desktop_interaction_unsupported`;
- `desktop_interaction_authority_required`;
- `desktop_interaction_authority_changed`;
- `desktop_interaction_conflict`;
- `desktop_interaction_duplicate`;
- `desktop_interaction_target_unavailable`;
- `desktop_interaction_stale_observation`;
- `desktop_interaction_focus_not_ready`;
- `desktop_interaction_focus_changed`;
- `desktop_interaction_coordinate_mismatch`;
- `desktop_input_provider_unavailable`;
- `desktop_input_failed`;
- `desktop_input_cleanup_failed`;
- `desktop_interaction_verification_failed`;
- `desktop_interaction_verification_unavailable`.

Errors expose typed identity and state only. Provider stderr, frame bytes, raw
text, full motion points, and private surface labels do not enter messages.

## Privacy And Persistence

Source and after frames remain in-memory and ephemeral. The interaction
response, stream projection, job result, incident details, logs, and persisted
idempotency record contain no image bytes, visualization bytes, raw OCR text,
plaintext keyboard content, full motion path, provider stderr, filesystem
paths, or provider URLs.

The response may return safe before/after receipt metadata and event summaries.
Durable state stores only the minimum redacted receipt needed for idempotency,
audit, effect uncertainty, and human handoff. Retention is declared in the
receipt and bounded by existing service job policy.

## Synthetic Fixture Corpus

Store versioned manifests under
`docs/dev/fixtures/desktop-interaction/`. The renderer reuses the PoC 2
synthetic control and adds deterministic focus, pointer, activated, typed-text,
and failure-transition state. No private or third-party pixels enter the repo.

Required cases include:

- centered, near-edge, short-distance, and long-distance pointer starts;
- exact matched target and exact after-state verification;
- ambiguous, not-found, and stale before observations;
- wrong, missing, changed, and out-of-bounds focus or surface evidence;
- controller takeover before start, during motion, before press, during fixed
  typing, and before verification;
- move, button-down, button-up, key-down, key-up, emergency-release, capture,
  and verification failures;
- same request identity replay and concurrent request conflict;
- unchanged, decoy, ambiguous, and binding-drift after states.

## Execution Slices

### Slice A | Authority Epoch And Coordinator

- add compatible route and stream controller epochs;
- centralize all primary-controller changes through epoch advancement;
- add the route-scoped interaction coordinator and cancellation contract;
- add takeover, release, reconcile, ABA, and unrelated-route concurrency tests.

Commit: `feat: fence desktop controller authority`

### Slice B | Synthetic Engine And Motion

- add manifests, deterministic renderer, pure fixed-point trajectory planner,
  input event plan, focus/surface contract, release guard, verifier, receipts,
  typed outcomes, and idempotency seam;
- record red tests before completing the implementation;
- implement only the injected synthetic fixture provider.

Commit: `feat: execute guarded synthetic desktop interaction`

### Slice C | Canonical Ingress And Privacy

- add `desktop_interact` to service action, schema, HTTP, generic MCP,
  dedicated MCP, generated client, CLI, metadata, lifecycle classification,
  and no-launch parity;
- expose `productionProviderConfigured=false` and make public dispatch fail
  before capture, lease mutation, or input;
- redact stream, job, incident, log, and idempotency projections.

Generated client names:

- `createServiceDesktopInteractRequest`;
- `requestServiceDesktopInteract`;
- `runServiceDesktopInteraction`.

Commit: `feat: expose guarded desktop interaction contract`

### Slice D | Documentation

- update CLI help, README, repo skill, commands docs, service-mode docs,
  contract schemas, capability metadata, and inline source documentation;
- state that only the synthetic provider is source-proven and configured
  production input is unavailable;
- do not advertise Turnstile, CAPTCHA, LastPass, passkey, credential, or
  general desktop control.

Commit: `docs: document guarded synthetic desktop interaction`

### Slice E | Audit And Source Acceptance

- freeze the complete PoC 3 diff and selected evidence;
- run one fresh independent audit across controller fencing, event cleanup,
  determinism, focus and geometry, verification, privacy, ingress parity,
  no-live behavior, docs, and tests;
- adjudicate once and perform at most one bounded remediation packet;
- run closed-world verification and record source acceptance or the exact
  blocker.

Commit: `test: close guarded desktop interaction proof`

## Required Tests

1. exact controller lease, primary IDs, route/browser ownership, active state,
   expiry, actor, and controller epoch are required before any provider call;
2. former controllers that remain `controlling`, reused lease IDs, malformed or
   expired leases, observers, wrong routes, and wrong browsers emit zero input;
3. takeover-first rejects interaction; interaction-first permits no mixed-
   controller event sequence; takeover at every event boundary stops before
   the next event; unrelated routes remain concurrent;
4. production provider absence fails before capture, controller mutation, or
   input, and no input backend executable or network transport is invoked;
5. only a matched fresh PoC 2 candidate from the internally captured frame can
   act; ambiguous, not-found, stale, mismatched, or caller-supplied evidence
   emits zero input;
6. identity physical-pixel mapping is exact and does not double-apply scale;
   overflow, bounds, dimension, scale, provider, surface, or geometry mismatch
   fails closed;
7. same bound inputs produce byte-identical trajectory and receipt hashes;
   paths retain exact endpoints, monotonic timing, bounded noncollinear arcs
   when space permits, fixed limits, and no overflow at edges;
8. missing or wrong focus emits zero input; focus or surface change after
   motion permits no button down; later changes stop before the next key or
   verification step;
9. successful fixture input has exact move, down, up, key-down, and key-up
   order with bounded delays and acknowledgements;
10. failures before down, after down, on primary up, during keys, and during
    emergency release prove cleanup, effect state, no stuck input, and no
    automatic retry;
11. fresh after-state verification alone produces `verified_success`;
    unchanged, decoy, ambiguous, unavailable, or binding-drift after states
    return the correct uncertain outcome and never repeat input;
12. duplicate caller/request identity returns the existing receipt or
    in-progress state and never re-emits;
13. caller coordinates, paths, seeds, timing, frames, candidates, arbitrary
    text or keys, clipboard, provider selection, paths, takeover, and nested
    params are rejected identically at CLI, HTTP, MCP, and client boundaries;
14. frames, visualizations, raw OCR, plaintext text, full paths, stderr, and
    provider URLs do not enter stream, job, incident, log, error, or durable
    idempotency projections;
15. the action performs no launch, CDP attach, navigation, display grant,
    route allocation, controller acquisition, takeover, filesystem write,
    external process, or network effect;
16. CLI, HTTP, generic MCP, dedicated MCP, generated client, schemas,
    capability metadata, help, skill, and docs normalize to the same action,
    receipt, provider-unavailable posture, and sessionName-only routing;
17. PoC 1 capture, PoC 2 locator, viewer lease, takeover, release,
    reconciliation, page screenshot, and remote-view handoff regressions pass.

## Validation

At minimum:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_interaction -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_locator -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_capture -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml viewer_lease -- --test-threads=1
pnpm test:service-client
pnpm test:service-api-mcp-parity
pnpm test:service-contracts-no-launch
node scripts/check-actions-architecture.js --check
pnpm test:wsl-cargo-safety
pnpm --dir docs build
pnpm validation:select -- --base eefcb9bd
git diff --check
```

Do not run ignored E2E, a browser, display, RDP, Guacamole, X11, ImageMagick,
Tesseract, external process, OS input, workstation, credential, or challenge
smoke under this plan.

## Hard Stops

- Stop if any production-capable input sink enters PoC 3.
- Stop if callers can submit coordinates, event plans, timing, arbitrary keys
  or text, pixels, candidates, provider identity, or takeover instructions.
- Stop if current primary controller identity and epoch are not revalidated at
  every emission boundary.
- Stop if human takeover can interleave another controller event after its
  cancellation fence.
- Stop if a down event can escape without one bounded release attempt.
- Stop if input acknowledgement is treated as verified outcome.
- Stop if a partial effect is converted into an ordinary retryable error or
  automatically repeated.
- Stop if pixels, plaintext text, full paths, provider stderr, or URLs become
  durable.
- Stop if source acceptance would imply installed, live, OS-input, challenge,
  or authentication proof.

## Delegation Receipt | Reconnaissance

| Handle | Task | Status | Evidence | Primary reconciliation |
| --- | --- | --- | --- | --- |
| `p110_poc3_authority_arch` | controller, takeover, binding, and input-provider architecture | complete | current primary-controller replacement, stale former controller state, PoC 1 and 2 binding seams, absence of native OS input | accepted exact primary authority plus epoch; accepted synthetic sink only; strengthened short transaction locking into per-event cancellation fencing |
| `p110_poc3_ingress` | canonical action, request, response, and parity | complete | effect attribution, session routing, named-recipe boundary, public provider-unavailable posture | accepted `desktop_interact`, strict allowlist, partial-effect receipts, generated helper names; removed caller motion seed |
| `p110_poc3_motion_tests` | motion, focus, cleanup, verification, and adversarial tests | complete | fixed-point Bézier bounds, focus and geometry checks, release guard, after-state proof, privacy matrix | accepted server-derived deterministic arc, exact event limits, fixed safe text, uncertain outcomes, and no retry |

All workers were read-only, touched no live system, and spawned no nested
workers. Graphiti returned only older service-control-plane and stale-target
context. Current CodeGraph and repository source control this plan.

## Acceptance

PoC 3 is source accepted when all seventeen requirements pass, the complete
synthetic transaction is deterministic and verified, public production
dispatch proves provider-unavailable before effects, one fresh audit has no
unresolved blocking finding after the single remediation packet, and status
is recorded without claiming installed or live machine input.

The sole next recommendation after acceptance is to write Plan 0110-4 for a
controlled browser-external prompt perception fixture. No PoC 4 implementation
begins during PoC 3 closeout.

## Source Acceptance | 2026-08-12

PoC 3 is source accepted at remediation commit `fd9c6a41`. The canonical
`desktop_interact` action remains a named, source-only synthetic transaction.
Public production dispatch still fails with
`desktop_input_provider_unavailable` before capture, controller mutation, or
input resolution.

The first audit worker returned no usable report and supplied no acceptance
evidence. A replacement fresh audit found four blocking defects: the pure
engine did not use the real per-event coordinator fence, some post-effect
failures discarded receipts and aborted idempotency, surface evidence was not
rechecked at every event boundary, and Rust receipt retention disagreed with
the frozen schema. All four findings were accepted as one remediation packet.

The remediation now:

- owns one real route-scoped interaction claim and holds a short event guard
  across current authority, focus, surface, geometry, and provider emission;
- records every post-acknowledgement failure as an uncertain or cancelled
  receipt and completes idempotency so replay cannot emit another plan;
- validates the synthetic provider's binding and expected surface for every
  move, button, key, and bounded cleanup event;
- rejects an empty controller lease update timestamp and serializes the
  schema-frozen `ephemeral` retention posture.

Primary-agent closed-world verification passed: format, strict Clippy,
`desktop_interaction` 17/17, `desktop_control_coordinator` 3/3,
`desktop_locator` 10/10, `desktop_capture` 26/26, and `viewer_lease` 2/2.
Earlier complete-candidate gates for service client, API/MCP parity,
no-launch contracts, action architecture, WSL Cargo safety, and docs also
passed and were unaffected by the two-module remediation. The durable evidence
summary is
`docs/dev/notes/0110-3-2026-08-12-guarded-desktop-interaction-source-acceptance.md`.

No browser, display, RDP, Guacamole, external process, OS input provider,
credential, authentication prompt, or challenge was exercised. This status is
not installed-runtime, live-fixture, challenge, authentication, or release
acceptance.
