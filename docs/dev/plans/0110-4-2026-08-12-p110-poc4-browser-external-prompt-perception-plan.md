# Plan 0110-4 | P110 PoC 4 Browser-External Prompt Perception

Date: 2026-08-12

State: SOURCE ACCEPTED

Authority: SOURCE-ONLY | SYNTHETIC FIXTURE | NO LIVE PROMPT

Lane: P110 PoC 4

Parent: `docs/dev/plans/0110-2026-08-12-desktop-perception-interaction-foundation-plan.md`

Predecessor: PoC 3 source accepted at `b74dbdd6`

## Objective

Prove that agent-browser can classify one repository-owned synthetic prompt as
browser-external because it is present in an exactly bound desktop composite
and absent from the corresponding independently rendered page image and
normalized DOM manifest. Return deterministic observation evidence and, when
policy requires it, a typed no-effect operator intervention.

This is a source proof of the repository fixture model. It is not proof that a
real browser, extension popup, native dialog, credential manager, passkey
prompt, CAPTCHA, or challenge is detectable.

## Frozen Decisions

1. The canonical queued read-only action is `desktop_prompt_observe`.
2. The only profile is `p110-external-prompt-v1` and the only positive target
   is `synthetic_browser_external_confirmation`.
3. Callers supply browser identity, optional session identity, the named
   profile, visualization preference, and attribution only. They never supply
   pixels, DOM, page images, prompt text, templates, thresholds, coordinates,
   providers, routes, displays, URLs, paths, challenge kinds, or input.
4. Public configured dispatch resolves provider availability first and returns
   `desktop_prompt_provider_unavailable` before capture, CDP, launch, route or
   display mutation, controller mutation, process, network, filesystem, or
   input effects.
5. The complete proof runs only through an injected repository fixture
   provider.
6. Desktop, page, and DOM evidence are independently produced and then bound.
   The page image may not be derived by cropping the completed desktop image.
7. Browser-external classification requires a matched desktop candidate, zero
   matching page candidates, zero matching DOM tokens, and verified geometric
   correspondence between the page layer and desktop viewport.
8. Detection and handling remain separate. A matched neutral fixture is an
   actionable observation without input authority. Ambiguity or a manual-only
   profile returns an operator intervention and no selected action.
9. Existing `Challenge`, CDP `PendingDialog`, profile-seeding handoff, PoC 2
   `Observation`, and PoC 3 interaction semantics remain separate.
10. One fresh audit and one bounded remediation packet are permitted.

## Public Contract

Request:

```json
{
  "action": "desktop_prompt_observe",
  "browserId": "browser-1",
  "sessionName": "optional",
  "promptProfileId": "p110-external-prompt-v1",
  "includeVisualization": false,
  "serviceName": "DesktopPerception",
  "agentName": "fixture-agent",
  "taskName": "observe-fixture"
}
```

`sessionName` is the only daemon-lane selector. `browserId` never selects a
daemon. Nested `params` and unknown fields are rejected for this action.

Response:

```json
{
  "ok": true,
  "action": "desktop_prompt_observe",
  "context": {},
  "frameReceipt": {},
  "promptObservation": {},
  "visualizationBase64": "optional"
}
```

CLI:

```text
agent-browser desktop prompt observe --browser-id <id> \
  --prompt-profile-id p110-external-prompt-v1
```

HTTP uses `POST /api/service/request`; MCP exposes generic `service_request`
and dedicated `desktop_prompt_observe`; the generated client exposes
`createServiceDesktopPromptObserveRequest`,
`requestServiceDesktopPromptObserve`, and
`observeServiceDesktopPrompt`.

## Deep Module

Add `native::desktop_prompt_perception` with a pure provider-free engine:

```text
PromptEvidenceBundle
  = BoundFrame
  + BrowserSurfaceEvidence
  + SyntheticPageReference
  + observedAt

observe_desktop_prompt(bundle, profile, includeVisualization, clock)
  -> PromptObservation
```

`BrowserSurfaceEvidence` binds browser, session, profile, display allocation,
stream, route, geometry epoch, browser bounds, viewport bounds, opaque surface
identity digest, browser-process identity digest, provider ID, and version.

`SyntheticPageReference` carries `proofClass=repository_fixture`, renderer ID
and version, independently rendered page PNG bytes and hash, normalized DOM
token IDs and manifest hash, viewport geometry, and observation time. It does
not pretend to be a real CDP target, tab, or screenshot receipt.

The engine validates all context, receipt, image, surface, process, geometry,
scale, coordinate-space, provider, freshness, and page-correspondence identity
before invoking any detector. Fixed maximum age is 750 ms.

## Fixture Corpus

Create `docs/dev/fixtures/desktop-prompt-perception/` with a corpus index and
JSON manifests for:

- light 100 percent matched overlay;
- dark 125 percent matched overlay;
- 150 percent extension-like toolbar panel;
- synthetic native-modal manual-only disposition;
- external prompt plus page-resident lookalike decoy;
- page decoy only;
- two equally eligible external surfaces;
- not found;
- occlusion inside and beyond the fixed budget;
- unsupported version;
- stale or mismatched binding.

The renderer uses integer RGBA primitives and pinned synthetic glyphs only. It
does not use system fonts, browser engines, OCR binaries, native theme APIs,
timestamps, randomness, floating-point score inputs, third-party artwork, or
real authentication semantics.

Each scene has three layers: normalized `pageDom`, independent `pageSurface`,
and `desktopScene` containing browser chrome, the exact page surface, and
separate prompt surfaces. The exact page-surface bytes are embedded at the
declared viewport bounds before external surfaces are composited.

Pin corpus, manifest, profile, renderer, page image, DOM manifest, desktop
frame, viewport-layer, detector, observation, visualization, and paired-receipt
hashes with domain-separated SHA-256 and canonical integer JSON.

## Observation And Blindness Receipt

`PromptObservation` binds context/frame/profile/surface/process IDs and hashes,
detector receipts, ordered candidates, and:

- `detectionStatus`: `matched | not_found | ambiguous`;
- `pageVisibility`: `absent | present`;
- `classification`: `browser_external | page_surface | unclassified`;
- `handlingOutcome`: `actionable_observation |
  operator_intervention_required | none`;
- selected candidate ID only for one matched, page-absent external candidate;
- optional static `operatorIntervention` with no URL, command, prompt text, or
  input instruction.

`BlindnessReceipt` includes `proofClass=repository_fixture`, desktop, page, DOM,
prompt-signature, and binding hashes; exact desktop/page/DOM match counts; and
`correspondenceState=verified`. The claim is named
`absent_from_fixture_page_inputs`, never live CDP blindness.

## Privacy And Persistence

Desktop/page/visualization bytes, DOM content, normalized prompt text, titles,
labels, provider stderr, URLs, and paths remain response-only or absent.
Stream, job, incident, event, log, and error projections retain only an
allowlisted receipt of opaque IDs, hashes, typed axes, proof class, safe bounds,
and intervention state. `retention=ephemeral`; pixels are never persisted.

Do not hash raw secret-bearing text as a redaction substitute. Only repository
token IDs and typed absence evidence may be hashed.

## Typed Outcomes

Successful observation states are matched, not found, ambiguous, and
page-visible. Invalid or unbound page evidence is a typed failure. Typed
failures are:

- `desktop_prompt_profile_not_found`;
- `desktop_prompt_provider_unavailable`;
- `desktop_prompt_binding_mismatch`;
- `desktop_prompt_stale_evidence`;
- `desktop_prompt_invalid_image`;
- `desktop_prompt_page_evidence_invalid`;
- `desktop_prompt_detector_failed`;
- `desktop_prompt_visualization_failed`.

## Implementation Slices

### Slice A | Corpus And Deep Engine

- add deterministic renderer, evidence types, exact validation, detector,
  observation, paired receipt, visualization, redaction, and fixture tests;
- add a configured handler that fails provider-unavailable before effects.

### Slice B | Contracts And Ingress

- add the canonical action, strict normalizer, no-launch dispatch, HTTP/MCP
  adapters, response schemas, contract metadata, generated client helpers, and
  parity tests.

### Slice C | CLI And Documentation

- add CLI parsing and safe text output;
- update `cli/src/output.rs`, README, repository agent-browser skill, docs site,
  and inline documentation without claiming real prompt detection.

### Slice D | Audit And Source Acceptance

- freeze the complete diff and run one fresh audit;
- adjudicate once, perform at most one remediation packet, run closed-world
  verification, and record exact source status.

## Required Tests

1. corpus inventory, schemas, versions, and every pinned hash are exact;
2. rendering and serialized observations are byte-identical across runs;
3. exact context/frame/surface/process/geometry/freshness/page correspondence
   is required before detector invocation;
4. positive scenes match only in desktop evidence while the independently
   rendered page image and DOM manifest contain zero prompt evidence;
5. page-resident lookalikes are decoys and cannot classify external;
6. ambiguity remains visible before candidate truncation and selects nothing;
7. not found and over-occluded cases invent no candidate;
8. fixed theme, scale, occlusion, score, candidate ordering, and budgets are
   pinned; malformed, oversized, overflowed, unsupported, and stale evidence
   fails typed;
9. handling outcome and operator intervention are deterministic, redacted,
   no-effect, and separate from challenge state;
10. public provider absence fails before capture, CDP, launch, route/display,
    controller, input, process, network, filesystem, or OCR resolution;
11. every ingress rejects caller evidence, tuning, assets, providers, paths,
    URLs, challenge, takeover, and input fields identically;
12. response-only bytes and forbidden sentinels do not enter durable
    projections;
13. CLI, HTTP, generic and dedicated MCP, client, schemas, metadata, help,
    skill, and docs are coherent with sessionName-only routing;
14. PoC 1 capture, PoC 2 locator, PoC 3 interaction, page screenshot, JS
    dialog, challenge/access-plan, and remote-view behavior regressions pass.

## Validation

At minimum:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_prompt -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_locator -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_capture -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_interaction -- --test-threads=1
pnpm test:service-client
pnpm test:service-api-mcp-parity
pnpm test:service-contracts-no-launch
node scripts/check-actions-architecture.js --check
pnpm test:wsl-cargo-safety
pnpm --dir docs build
git diff --check
```

Do not run ignored E2E, browser, display, RDP, Guacamole, X11, ImageMagick,
Tesseract, CDP, credential, authentication, prompt, challenge, or input smoke.

## Hard Stops

- Stop if page evidence is cropped from the completed desktop frame.
- Stop if browser-external classification lacks cryptographic and geometric
  correspondence among the bound desktop frame, page image, DOM evidence, and
  selected browser surface.
- Stop if a page decoy can become an external candidate or ambiguity selects a
  candidate.
- Stop if callers control evidence, detector assets, tuning, provider identity,
  coordinates, prompt text, challenge state, or effects.
- Stop if any real browser, prompt, credential, challenge, provider, external
  process, network, input, or display-access dependency enters source proof.
- Stop if sensitive evidence becomes durable.
- Stop if source acceptance is described as installed, live, real-prompt,
  credential-manager, challenge, authentication, or release proof.

## Reconnaissance Receipt

| Handle | Lane | Result | Primary reconciliation |
| --- | --- | --- | --- |
| `p110_poc4_perception_arch` | deep architecture and semantic boundaries | complete | accepted separate deep module, paired evidence, source-only claim, provider-unavailable posture; rejected reuse of Challenge and CDP dialog models |
| `p110_poc4_ingress_contract` | ingress and contract options | complete | accepted strict request, no-launch parity, existing attribution/routing patterns; rejected reusing `desktop_locate` because paired page/DOM evidence and intervention semantics require a distinct response contract |
| `p110_poc4_fixture_proof` | corpus, privacy, hashes, and tests | complete | accepted three-layer independent rendering, exact hashes, decoy/ambiguity matrix, privacy and hard stops |

All reconnaissance was read-only and touched no live system. Graphiti returned
only older remote-view fixture context; current plans and source control.

## Acceptance

PoC 4 is source accepted only when all fourteen requirements pass, configured
production remains provider-unavailable, one fresh audit has no unresolved
blocking finding after the single remediation packet, and the status explicitly
limits the blindness claim to repository fixture inputs.

The sole next recommendation after acceptance is to write Plan 0110-5 for
provider-neutral foundation stress and use-case entry.

## Source Acceptance | 2026-08-12

PoC 4 is source accepted at remediation commit `7391409b`; its initial
implementation is `54cbc9e3`. The one fresh audit found six blocking defects:
MCP browser-ID routing, provider gating after dispatch effects, unattested
viewport correspondence, unpinned corpus outputs and missing failure cases, a
shared parity count, and an advertised but unreachable successful
`indeterminate` state. All six were accepted into one remediation packet.

The remediation makes `sessionName` the sole MCP lane selector, gates provider
absence before public dispatch effects, derives the viewport layer from the
decoded desktop frame, binds its geometry and hash, pins the corpus and
canonical evidence receipts, covers malformed and adversarial inputs, derives
the parity count from the field ledger, and restricts page visibility to the
reachable `absent | present` states.

Primary closed-world verification passed: golden corpus 1/1, focused prompt
27/27, strict Clippy, formatting, PoC 1 through 3 regressions, full service
client, 100-action API/MCP parity, no-launch service contracts, docs build, and
diff checks. The durable evidence is
`docs/dev/notes/0110-4-2026-08-12-browser-external-prompt-perception-source-acceptance.md`.

No live browser, CDP screenshot, display, prompt, credential manager,
authentication flow, challenge, or input provider was exercised. The
blindness claim applies only to independently rendered repository fixture
inputs.
