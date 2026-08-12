# Plan 0110 | Desktop Perception And Interaction Foundation

Date: 2026-08-12

State: OPEN | POC 1 SOURCE ACCEPTED | POC 2 DETAILED

Authority: SOURCE-ONLY

Lane: P110

Sources:

- `VISION.md`
- `ROADMAP.md` P110
- `docs/dev/plans/0110-1-2026-08-12-p110-poc1-display-bound-frame-capture-plan.md`
- existing service-owned browser, display-allocation, view-stream, route,
  lease, challenge, and durable remote-view handoff contracts

## Current State

The product direction and five proof-of-concept sequence are frozen. Existing
service state can bind a browser to a display allocation, remote-view route,
view stream, and operator-visible proof, but production code has no
provider-neutral desktop frame source, desktop observation contract, locator,
or machine-controlled desktop input path. CDP page screenshots remain a
separate browser-page capability.

Proof of Concept 1 is source accepted at `853c2d90` with no live or installed
proof. The canonical `desktop_capture` action now resolves an exact
service-owned RDP workspace, returns bounded ephemeral PNG bytes plus typed
context and receipt evidence, and is coherent across CLI, HTTP, MCP, generated
client, schemas, help, skill, and docs. No locator or machine input exists.

Proof of Concept 2 is detailed in Plan 0110-2 before implementation.
Later proofs remain undetailed until the immediately preceding proof closes.
This keeps locator, input, prompt, and challenge assumptions out of earlier
contracts.

## Objective

Create a service-owned desktop perception and interaction foundation that can
observe and later control browser-external UI while preserving browser,
profile, session, display, route, geometry, controller, policy, and human
handoff authority. Deliver it through five sequential controlled proofs:

1. display-bound frame capture;
2. deterministic fixture location;
3. guarded pointer and keyboard interaction;
4. browser-external prompt perception;
5. provider-neutral stress and use-case entry.

## Frozen Decisions

1. The foundation is part of agent-browser, not a separate desktop robot.
2. A desktop context is resolved from service-owned identity. Callers never
   supply raw Guacamole URLs or arbitrary display names.
3. Frame transport, semantic evidence, location, coordinate mapping, input,
   verification, policy, and use-case assets remain separate mechanisms.
4. Every later input transaction follows observe, locate, act, and verify.
5. Observation is read-only. Machine input requires explicit current control
   authority and is serialized with human takeover.
6. Sensitive frames are ephemeral and redacted by default.
7. Determinism describes agent-browser's own evidence and replay, not an
   external site's challenge or authentication outcome.
8. Challenge and authentication integrations begin only after the foundation
   passes Proof of Concept 5.
9. Source, installed runtime, live fixture, and release acceptance are separate
   proof boundaries.
10. One fresh audit cycle and one bounded remediation cycle are allowed per
    proof. A third broad review cycle is not.

## Authority And Non-Goals

This umbrella plan authorizes repository source, generated contracts and
clients, controlled fixtures, tests, documentation, and bounded plan records.
It does not authorize:

- installing or replacing an agent-browser runtime;
- opening, navigating, reconnecting, taking over, or closing a live browser;
- granting X display access or invoking the privileged helper;
- interacting with a real Turnstile, CAPTCHA, LastPass, passkey, biometric,
  PIN, master-password, secure-desktop, or consent prompt;
- retaining private desktop pixels or credentials;
- promising anti-bot evasion or deterministic external acceptance;
- publishing a formal release.

Any controlled live RDP or Guacamole proof requires separate explicit
authority after provider-free and no-launch source gates pass.

## Working Contracts

The five proofs refine these types without changing their core identity:

- `DesktopContext` binds browser, session, profile, display allocation, view
  stream, route, frame source, coordinate spaces, geometry epoch, and control
  authority.
- `FrameReceipt` binds a frame to its context, provider, sequence, dimensions,
  scale, capture time, hash, freshness, and retention posture.
- `Observation` binds detector identity and candidate evidence to one frame.
- `InteractionRecipe` describes bounded observe, locate, move, click, key,
  text, wait, and verify steps.
- `InteractionReceipt` records authority, selected evidence, emitted input,
  before and after frames, verification, errors, and human handoff state.

Opaque IDs and typed relationships are authoritative. Unscoped coordinates,
provider URLs, and caller-guessed display identifiers are not accepted.

## Sequential Proofs

### Proof Of Concept 1 | Display-Bound Frame Capture

Deliver one no-input desktop frame from an exactly bound service workspace
through the native service, CLI, HTTP, MCP, generated client, contract
metadata, help, skill, and docs. The detailed authority is Plan 0110-1.

### Proof Of Concept 2 | Deterministic Fixture Location

After PoC 1 closes, write Plan 0110-2. Exercise deterministic template,
geometry, and OCR location on pinned fixtures across scale, theme, position,
and a similar decoy. No input is emitted.

### Proof Of Concept 3 | Guarded Pointer And Keyboard Transaction

After PoC 2 closes, write Plan 0110-3. Add replayable pointer motion, bounded
click and non-sensitive keyboard input, stale-frame abort, focus and geometry
checks, controller authority, before and after evidence, and verification on a
controlled fixture.

### Proof Of Concept 4 | Browser-External Prompt Perception

After PoC 3 closes, write Plan 0110-4. Detect a controlled browser chrome,
extension, or native prompt absent from the page DOM and screenshot. Return an
actionable observation or typed operator intervention without using a real
account, credential, secret, or external challenge.

### Proof Of Concept 5 | Foundation Stress And Use-Case Entry

After PoC 4 closes, write Plan 0110-5. Exercise one provider-neutral recipe
through all advertised ingresses, dashboard projection, authority conflicts,
failure modes, cleanup, and durable human handoff. Only this proof can open the
entry gate for discrete challenge and authentication use cases.

## Orchestration Contract

Each proof uses shallow, bounded delegation:

- reconnaissance agents inspect disjoint architecture, ingress, or test
  surfaces and do not edit;
- implementation agents receive non-overlapping file ownership and a frozen
  contract;
- one fresh auditor reviews the complete frozen diff once;
- one testing agent executes the selected closed-world gates;
- the primary agent adjudicates findings, performs at most one remediation
  packet, and owns the final status.

Every delegation record names the handle, task, status, evidence, and primary
reconciliation. Agents may not touch live systems, spawn nested agents, or
broaden scope unless their packet explicitly grants that authority.

## Proof Acceptance Boundaries

A proof closes only when:

- its detail plan existed before implementation;
- the public contract and hard stops are satisfied;
- required generated artifacts and user-facing docs are aligned;
- selected provider-free and no-launch gates pass;
- the fresh audit has no unresolved blocking finding after the one allowed
  remediation cycle;
- worktree, commit, validation, and remaining authority are recorded;
- source acceptance is not presented as installed or live acceptance.

## Validation Posture

Validation is selected per proof and remains proportional to touched surfaces.
All compiling Rust commands on WSL use `scripts/ci/cargo-safe.sh`. A typical
proof includes strict Rust formatting and Clippy, focused Rust tests, service
contract and client parity, architecture guards, dashboard checks when
touched, docs build, validation selection, and patch checks. Ignored E2E or
live route smokes require separate authority.

## Hard Stops

- Stop if a request can select an arbitrary display, raw provider URL, or
  unrelated browser workspace.
- Stop if observation silently grants display access or launches, adopts,
  navigates, reconnects, takes over, or closes a browser.
- Stop if raw sensitive pixels enter logs or durable state by default.
- Stop if machine input can race a human controller or bypass lease authority.
- Stop if a detector-specific or use-case-specific assumption enters the core
  contract without demonstrating a reusable mechanism.
- Stop if source acceptance would require claiming an unrun installed or live
  proof.

## P110 Acceptance

P110 remains open until all five proofs meet the Foundation Acceptance section
in `ROADMAP.md`. Each accepted proof advances the same lane and records the
next single detail-plan recommendation. No formal release follows from P110
source acceptance without separate maintainer direction.

## Current Recommendation

Execute Plan 0110-2 only. Implement atomic source-bound deterministic fixture
location with pinned profiles, stable evidence and ordering, explicit
ambiguity, and response-only visualization. Do not emit input or use ambient
OCR, a live challenge, credential prompt, browser, RDP route, or display.
