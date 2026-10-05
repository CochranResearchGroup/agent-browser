# Plan 0110-2 | P110 PoC 2 Deterministic Fixture Location

Date: 2026-08-12

State: SOURCE ACCEPTED | NO LIVE OR INPUT PROOF

Authority: SOURCE-ONLY | PROVIDER-FREE FIXTURES | NO INPUT

Lane: P110 PoC 2

Parent: `docs/dev/plans/0110-2026-08-12-desktop-perception-interaction-foundation-plan.md`

Predecessor: PoC 1 source accepted at `c4d2ff5d`

## Objective

Locate a known synthetic desktop control using deterministic geometry,
template, and OCR-token evidence bound to one freshly captured PoC 1 frame.
Return a typed `Observation` with ordered candidates, an explicit selection
state, detector receipts, and an optional bounded response-only visualization
through one canonical queued action and all advertised ingresses.

PoC 2 observes only. It emits no pointer, keyboard, focus, takeover, lease,
browser, route, display, or filesystem effect. It does not use a real
challenge, credential prompt, account, secret, browser, display, RDP route,
Guacamole connection, or ambient OCR engine for source acceptance.

## Reconciled Existing Architecture

- PoC 1 `DesktopContext`, `FrameReceipt`, and `DesktopCaptureResult` already
  bind browser, session, display, route, stream, frame hash, dimensions, scale,
  coordinate space, geometry epoch, freshness, and ephemeral retention.
- `desktop_capture` already performs the exact service-state resolution,
  visible-display proof, bounded capture, and post-capture drift check.
- the Rust crate already uses `image 0.25`; `native/diff.rs` demonstrates
  deterministic RGBA iteration and PNG visualization without OpenCV.
- existing Tesseract use is live shell-script tooling that writes crops and
  OCR text to disk. It is not an ephemeral provider-neutral product seam.
- service request, CLI, HTTP, MCP, generated client, contract metadata,
  privacy projection, help, skill, and docs parity are already proven by
  `desktop_capture` and should be extended through the same seams.

## Frozen Public Contract

### Canonical Action

`desktop_locate` is one atomic queued observation action. It captures a fresh
frame using the PoC 1 authority path and locates against that exact in-memory
frame before returning. It does not accept a caller-supplied frame or create a
durable frame-artifact registry.

Request shape:

```json
{
  "action": "desktop_locate",
  "browserId": "browser-1",
  "sessionName": "optional",
  "locator": {
    "locatorId": "p110-control-v1",
    "maxCandidates": 8
  },
  "includeVisualization": false,
  "serviceName": "DesktopObserver",
  "agentName": "fixture-locator",
  "taskName": "locate-control"
}
```

Only named, versioned, repository-owned locator profiles are accepted. A
profile freezes its content hashes, detector order, theme, supported scale
pyramid, thresholds, ambiguity margin, OCR-token requirements, and target
class. Callers cannot tune scores or submit templates.

The request rejects `imageBase64`, `frameId`, `contextId`, raw coordinates,
template or OCR bytes, asset or output paths, display names, provider URLs,
crop rectangles, thresholds, arbitrary detector parameters, and all input
options.

CLI spelling:

```text
agent-browser desktop locate --browser-id <id> --locator-id <id> \
  [--max-candidates <n>] [--include-visualization]
```

### Observation

The response contains the PoC 1 `context` and `frameReceipt`, plus:

- `observationId` and `schemaVersion`;
- `contextId`, `frameId`, frame SHA-256, and `geometryEpoch`;
- coordinate space fixed to `desktop_physical_pixels`;
- locator ID, profile version, profile SHA-256, and target class;
- ordered detector receipts with detector ID, version, asset or evidence hash,
  normalization version, and integer parameters;
- `status`: `matched`, `not_found`, or `ambiguous`;
- nullable `selectedCandidateId`;
- stable ordered candidates;
- optional `visualizationReceipt` and response-only `visualizationBase64`.

Each candidate contains a stable ID, target class, rank, physical-pixel
bounds and center, integer score, supporting detector evidence, and explicit
decoy or ambiguity evidence. Candidate ordering is score descending, then
top-to-bottom, left-to-right, then candidate ID. Equal or insufficiently
separated leading candidates are ambiguous and never selected.

### Typed Failures

At minimum preserve stable codes for:

- `desktop_locator_not_found` for an unknown profile;
- `desktop_locator_unsupported` for unsupported theme or scale;
- `desktop_locator_frame_mismatch` for context, receipt, hash, dimensions, or
  geometry disagreement;
- `desktop_locator_invalid_image` for malformed or resource-unbounded PNG;
- `desktop_locator_detector_unavailable` for a required detector provider;
- `desktop_locator_detector_failed` for malformed, timed-out, or oversized
  detector output;
- `desktop_locator_visualization_failed` for bounded overlay failure.

`not_found` and `ambiguous` are successful observations, not transport errors.
They contain no selected target and will become hard stops for PoC 3 input.

## Deep Module Boundary

Add a sibling `native::desktop_locator` module with a pure core:

```text
BoundFrame { DesktopContext, FrameReceipt, bytes }
       + LocatorProfile
       + injected OcrEvidenceProvider
                   |
                   v
Observation { binding, receipts, ordered candidates, optional overlay }
```

The service handler composes PoC 1 capture and pure location in one
`spawn_blocking` transaction. The pure core recomputes the frame hash and
validates every context and receipt binding before detector invocation.

Detector mechanisms remain separate:

1. geometry detector supplies reviewed search regions and exact coordinate
   evidence;
2. RGBA template detector scans a finite profile-owned scale pyramid with
   integer normalized-difference scoring and explicit row-major iteration;
3. OCR fusion consumes normalized token boxes from an injected provider and
   requires profile-owned tokens to corroborate class and bounds.

Provider-free tests use pinned token-and-box evidence. A bounded Tesseract
adapter may be implemented only behind the OCR provider seam with direct
missing, timeout, malformed, oversized, version, and stderr-redaction tests.
Ambient Tesseract output is not an acceptance oracle.

## Pinned Fixture Corpus

Store synthetic fixture manifests under
`docs/dev/fixtures/desktop-locator/`. A manifest pins scene dimensions, theme,
scale, element geometry, template pixels or asset hashes, OCR token boxes,
expected candidates, selection state, and visualization hash. Tests render
the manifests deterministically into in-memory PNGs. No private or third-party
screen pixels enter the repository.

Required cases:

| Fixture | Theme | Scale | Position | Expected result |
| --- | --- | --- | --- | --- |
| `single-light-100` | light | 1.00 | upper-left | one selected target; similar decoy rejected |
| `single-dark-100` | dark | 1.00 | lower-right | one selected theme-specific target |
| `single-light-125` | light | 1.25 | center | one selected scaled target |
| `single-dark-150` | dark | 1.50 | offset | one selected scaled target |
| `ambiguous-equal` | light | 1.00 | separated | two equal candidates; no selection |
| `decoy-only` | light | 1.00 | arbitrary | no candidates |
| `geometry-edge` | dark | 1.25 | frame edge | exact in-bounds result or typed rejection |
| `stale-binding` | either | 1.00 | arbitrary | binding rejection before detection |

The exact same manifest, frame, profile, and injected OCR evidence must produce
byte-identical serialized observations. Expected geometry uses exact bounds;
template acceptance requires the frozen integer threshold and at least 0.90
IoU with the fixture truth, with center error no greater than one physical
pixel. These are fixture metrics, not claims about arbitrary UI.

## Privacy And Retention

Source pixels, OCR text, templates, and visualization bytes are response-only
and ephemeral. Persisted job results retain ordinary success metadata only.
Long-lived stream broadcasts remove both source and visualization payloads.
Errors and logs contain detector/profile IDs and typed states, never provider
stderr, raw OCR text, image bytes, or private screen content.

The returned observation may retain normalized token identifiers and bounds
when the profile marks them non-sensitive. It does not return arbitrary OCR
text in PoC 2.

## Execution Slices

### Slice A | Fixtures And Pure Contract

- commit synthetic manifests and a deterministic renderer;
- add red tests for exact frame binding, scale/theme/position, decoy,
  ambiguity, stale binding, and deterministic serialization;
- define profile, detector receipt, candidate, observation, visualization,
  and typed error records.

Commit: `test: freeze deterministic desktop locator contract`

### Slice B | Deterministic Detectors

- implement bounded PNG decode and binding validation;
- implement geometry, integer RGBA template, OCR-token fusion, stable ordering,
  selection, and visualization;
- add injected and fake-subprocess OCR provider tests.

Commit: `feat: locate deterministic desktop fixture targets`

### Slice C | Atomic Capture And Ingress Parity

- expose a crate-owned PoC 1 capture seam without weakening its validation;
- compose fresh capture and location in `desktop_locate`;
- add service action, schema, CLI, HTTP, generic and dedicated MCP, generated
  client, metadata, redaction, and no-launch parity;
- ensure only `sessionName`, never `browserId`, narrows daemon routing.

Commit: `feat: expose deterministic desktop location`

### Slice D | Documentation

- update help, README, repo skill, commands docs, service-mode docs, schemas,
  and inline source docs;
- describe matched, not-found, ambiguous, response-only visualization, and
  source-only OCR posture without promising general challenge recognition.

Commit: `docs: document deterministic desktop location`

### Slice E | Audit And Source Acceptance

- freeze the complete diff and selected evidence;
- run one fresh independent audit across binding, determinism, ambiguity,
  privacy, ingress parity, no-input behavior, docs, and tests;
- adjudicate once and perform at most one bounded remediation packet;
- run closed-world verification and record source acceptance or the exact
  blocker.

Commit: `test: close deterministic desktop location proof`

## Required Tests

1. context, receipt, frame hash, dimensions, scale, coordinate space, and
   geometry epoch agree before any detector runs;
2. unknown profile, caller assets, paths, pixels, coordinates, thresholds, or
   detector parameters are rejected;
3. light and dark fixtures at 1.00, 1.25, and 1.50 scales locate exact expected
   bounds with stable candidate IDs and order;
4. a similar decoy stays below the profile threshold;
5. equal or insufficiently separated leaders return `ambiguous` with no
   selected candidate;
6. absent target returns `not_found`, not an invented target or retry;
7. geometry, template, and OCR-token evidence receipts are versioned and bound
   to the observation;
8. repeated runs serialize byte-identically and visualization hash is stable;
9. invalid or oversized PNG, unsupported scale/theme, out-of-bounds candidate,
   transform overflow, and stale binding fail closed;
10. missing, timed-out, malformed, oversized, or version-incompatible OCR
    provider output is typed and stderr-redacted;
11. `desktop_locate` performs one fresh PoC 1 capture and no launch, CDP,
    navigation, display grant, route mutation, takeover, input, or disk write;
12. source and visualization pixels and raw OCR text do not enter streams,
    jobs, logs, incidents, or durable service state;
13. CLI, HTTP, generic MCP, dedicated MCP, generated client, schemas, contract
    metadata, help, skill, and docs normalize to one action and response;
14. PoC 1 capture, page screenshot, and remote-view handoff behavior remain
    intact.

## Validation

At minimum:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_locator -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_capture -- --test-threads=1
pnpm test:service-client
pnpm test:service-api-mcp-parity
pnpm test:service-contracts-no-launch
node scripts/check-actions-architecture.js --check
pnpm test:wsl-cargo-safety
pnpm --dir docs build
pnpm validation:select -- --base c4d2ff5d
git diff --check
```

Do not run ignored E2E, real Tesseract, real frame capture, browser, display,
RDP, Guacamole, workstation, or challenge smokes under this plan.

## Hard Stops

- Stop if location can consume an untrusted caller frame or select an arbitrary
  filesystem asset, display, provider, crop, coordinate, or score.
- Stop if observation is not cryptographically and geometrically bound to the
  fresh PoC 1 frame.
- Stop if ambiguity or detector disagreement can produce a selected target.
- Stop if the core depends on ambient OCR output for deterministic acceptance.
- Stop if pixels, OCR text, or visualization become durable by default.
- Stop if any pointer, keyboard, focus, controller, takeover, route, display,
  browser, profile, or navigation effect enters this proof.
- Stop if a real challenge or credential prompt becomes a fixture.

## Delegation Receipt | Reconnaissance

| Handle | Task | Status | Evidence | Primary reconciliation |
| --- | --- | --- | --- | --- |
| `p110_poc2_detector_arch` | detector architecture and dependency seam | complete after timebox interruption | PoC 1 bound-frame types, `image` and diff mechanisms, OCR script limitations, deterministic integer ordering | accepted sibling deep module, fixed-point scoring, injected OCR provider, and strict frame binding |
| `p110_poc2_ingress_contract` | canonical ingress and response contract | complete after timebox interruption | response-only PoC 1 pixels, service-request parity surfaces, session relay rules | accepted atomic `desktop_locate`; rejected caller image submission and durable artifact registry |
| `p110_poc2_fixture_tests` | fixture and adversarial test matrix | complete after timebox interruption | synthetic matrix, `.gitignore` raster caveat, OCR provider posture, visualization and no-launch gates | accepted manifest-rendered synthetic fixtures, ambiguity and decoy cases, injected OCR evidence, and stable overlays |

The workers were interrupted once when they exceeded the reconnaissance
timebox, then returned verified read-only evidence. No worker edited files,
touched a live system, or spawned nested agents. Graphiti supplied only an old
OCR-backed remote-view fixture fact; current repo source and the PoC 1
acceptance record control this plan.

## Acceptance

PoC 2 is source accepted when the fourteen requirements pass, every supported
fixture produces the frozen deterministic observation, one fresh audit has no
unresolved blocking finding after the single remediation packet, and status is
recorded without claiming ambient OCR, installed, live, or input proof.

The sole next recommendation is to write Plan 0110-3 for guarded pointer and
keyboard transactions. No PoC 3 implementation begins in PoC 2 closeout.

## Source Acceptance | 2026-08-12

PoC 2 is source accepted at `4281196a`. Implementation commits are
`eee02341` for contracts/client/MCP, `6f11ea34` for CLI/docs, `da7f91e2` for
the pure locator and synthetic fixtures, and `b0792c85` for native dispatch,
HTTP routing, and service control-plane integration.

One independent broad audit produced four blocking candidates and one evidence
candidate. The single remediation commit `4281196a`:

- compares capture-provider identity across context and receipt before OCR;
- gives generic service requests the same strict top-level allowlist as the
  generated client;
- replaces nonexistent challenge-like example profiles with the sole
  synthetic `p110-control-v1` profile and documents its narrow meaning;
- expands the corpus to both themes at 1.00, 1.25, and 1.50 scale and pins full
  serialized observation and visualization hashes;
- asserts complete selected bounds, centers, scores, ranks, and evidence;
- adds typed timeout, oversize, and version-incompatible OCR-provider tests.

Closed-world validation passed guarded formatting, strict Clippy, 10 locator
core tests, 12 ingress/CLI tests, 26 PoC 1 capture tests, 46 remote-view
handoff tests, 30 page-screenshot tests with two ignored E2E cases, full
service client, 98-action API/MCP parity, no-launch service-contract smoke,
action inventory, WSL Cargo safety, docs build, and patch checks.

No live browser, display, RDP route, Guacamole connection, ImageMagick,
Tesseract, pointer, keyboard, controller, credential, or challenge was used.
The timeout case proves the provider contract propagates a typed timeout; PoC
2 does not introduce an ambient OCR subprocess adapter. This is source
acceptance of the synthetic deterministic locator only.
