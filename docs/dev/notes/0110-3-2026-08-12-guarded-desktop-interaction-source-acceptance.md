# P110 PoC 3 Guarded Desktop Interaction Source Acceptance

Date: 2026-08-12

Status: SOURCE ACCEPTED

Authority: SOURCE-ONLY

Plan:
`docs/dev/plans/0110-3-2026-08-12-p110-poc3-guarded-desktop-interaction-plan.md`

Accepted remediation: `fd9c6a41`

## Outcome

PoC 3 proves one complete synthetic observe, locate, act, and verify
transaction behind the canonical `desktop_interact` action. The transaction
selects only the repository-owned `p110-pointer-keyboard-v1` recipe, requires
current service-owned controller authority, computes a deterministic
fixed-point pointer arc, emits one left click and fixed benign text through an
injected synthetic sink, performs bounded release cleanup, captures fresh
after evidence, and verifies the fixture state.

Configured production dispatch has no input adapter. It returns
`desktop_input_provider_unavailable` before resolving capture, controller
mutation, or input. No source status in this note implies live desktop control.

## Audit And Adjudication

The initial fresh audit worker did not return a report. Its timeout and
interruption supplied no acceptance or rejection evidence and were ignored.
A replacement fresh audit evaluated the frozen candidate once and returned
four blocking findings:

1. the interaction engine's local abstraction did not enter the real
   `DesktopInteractionClaim` event fence;
2. several failures after acknowledged pointer motion returned no receipt and
   aborted idempotency, permitting replay;
3. focus, surface, and geometry were not proven at every provider event, and
   the synthetic sink ignored the expected binding and surface;
4. Rust emitted `redacted_receipt_only` while the frozen schema required
   `ephemeral`.

All four findings were accepted as one bounded remediation packet. The same
packet also rejected an empty lease update timestamp. No second broad audit
was run.

## Remediation Evidence

- `DesktopInteractionClaim::begin_event()` now fences each move, button, and
  key event. Under that guard the engine re-reads current authority, freshness,
  focus, surface identity, browser-process identity, dimensions, scale, and
  geometry before exactly one synthetic provider emission.
- Cleanup uses a separate bounded event guard to attempt one release without
  permitting a new action plan.
- After the first acknowledged event, all failure paths attach an
  `InteractionReceipt` and complete the idempotency record. Replaying the same
  caller and request identity returns that receipt without another emission.
- The synthetic provider validates the supplied service binding and expected
  surface. Adversarial fixtures cover move, button-up, and key-up boundary
  drift.
- Receipt retention is exactly `ephemeral`, plaintext fixed text is excluded,
  full paths are excluded, and `persistedPixels` remains false.

## Primary-Agent Closed-World Validation

The primary agent ran these checks after the remediation source froze:

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`:
  passed;
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`:
  passed;
- focused `desktop_interaction`: 17 passed;
- focused `desktop_control_coordinator`: 3 passed;
- focused `desktop_locator`: 10 passed;
- focused `desktop_capture`: 26 passed;
- focused `viewer_lease`: 2 passed;
- `git diff --check`: passed.

The first focused interaction run passed 16 of 17 and exposed an error-domain
loss at guarded release boundaries. That defect was fixed inside the same
remediation packet: a successful emergency release now preserves the original
focus or geometry error, while `desktop_input_cleanup_failed` is reserved for
failed cleanup. The focused suite and strict Clippy then passed.

Before the audit, the complete PoC 3 candidate also passed the service client,
API/MCP parity, no-launch service contracts, action architecture, WSL Cargo
safety, docs build, capture, locator, screenshot, remote-view handoff, and
controller lifecycle selections. The two-module remediation did not alter the
JavaScript, schema, CLI, documentation, capture, locator, or transport surfaces.

## Boundary

No browser, display, RDP, Guacamole, X11, ImageMagick, Tesseract, external
process, network input transport, OS input provider, credential, real prompt,
authentication flow, or challenge was exercised. This is not installed,
live-fixture, credential-manager, challenge, authentication, or release
acceptance.

The next action is to write Plan 0110-4 for controlled browser-external prompt
perception before any PoC 4 implementation.
