# Plan 0210 | Visual Artifact And Provider Invocation Adapter

Date: 2026-09-17

Plan version: 2

State: PLANNED | NOT ADMITTED

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P210

Parent plan: Plan 0187 W7-C

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 retains every
browser, provider-backed, Turnstile or hCaptcha acceptance effect

Proposed branch: `challenge/p210-visual-artifact-adapter`

Target: `main`

Dependency: P209 must enter canonical `main` before P210 source admission

## Objective

Add the provider-free adapter immediately above P209's visual-provider
protocol. The adapter will bind one already-prepared synthetic visual payload
to the exact P209 request, enforce redaction and ephemeral-retention evidence,
invoke one injected fake provider exactly once, and pass its typed response to
P209 adjudication.

This packet proves artifact custody and invocation behavior without selecting
a real model, opening a network connection, loading credentials, capturing a
browser frame, persisting pixels, translating candidates to coordinates,
emitting desktop input, exposing a public Service action, or attempting a
challenge.

## Dependency And Admission Gate

P209 owns the provider-neutral request and response protocol. P210 is a
dependent consumer and must not begin implementation from a local-only P209
ref. Admission requires:

1. P206 and P209 are integrated into canonical `main` in dependency order.
2. The P209 integration receipt and exact source checkpoint are recorded.
3. No active writer owns the proposed adapter crate or its workspace manifest
   entries.
4. The current worktree inventory permits one primary P210 checkout, or the
   existing challenge checkout is explicitly and cleanly reassigned.
5. The changed-surface selector is run from the admitted baseline before the
   first source edit.

Until those gates pass, this document is planning evidence only. It grants no
branch, worktree, provider, browser, credential, runtime, CI or effect
authority.

## Fixture Relationship

The repository CAPTCHA lab remains the correct later acceptance surface for
issue #66, but it is not P210's provider-free oracle. Its default hCaptcha test
key does not open an image challenge, while its visual mode depends on an
external non-production Always Challenge configuration and provider-served
content. That behavior is intentionally outside a deterministic source test.

P210 may reuse the lab's safety patterns: secrets remain server-side, request
bodies have explicit byte ceilings, responses are not cached, and provider
verification has a fixed timeout. P210 must not read the lab credentials,
start the lab, open its widget, capture its pixels, use a retained browser or
handoff, or treat a provider-selected live image as a source fixture. Its
artifact matrix uses small repository-owned synthetic bytes with no private or
third-party content. A later issue #66 packet may join the accepted adapter to
the lab under a fresh provider and effect budget.

## Proposed Boundary

Create a narrow `agent-browser-challenge-visual-adapter` crate that depends on
`agent-browser-challenge-control` and ordinary serialization and digest
libraries. It must not depend on the CLI, Service State, desktop services, CDP,
browser launch, async runtimes, HTTP clients, model SDKs, filesystem storage,
image decoders, OCR, platform input or credential providers.

The challenge-control crate remains the policy owner. The adapter crate owns
only process-local artifact custody and one-shot provider invocation. A later
CLI or Service adapter may supply real capture and transport implementations,
but none are part of P210.

The proposed contract contains:

- `VisualInvocationPolicy`: exact maximum payload, serialized request and
  serialized response bytes, allowed media types and dimension bounds;
- `VisualArtifactEnvelope`: artifact identity, byte digest and length, media
  type, dimensions, source evidence, frame, context and geometry digests,
  redaction policy and receipt digests, provider capability, preparation time,
  expiry and an explicit ephemeral-retention posture;
- `PreparedVisualPayload`: the envelope plus process-local bytes whose digest
  and length must match the envelope;
- `VisualProviderInvocation`: the exact P209 request plus the matching prepared
  payload, with one canonical invocation digest;
- `VisualProviderTransport`: one injected call that accepts the invocation and
  returns serialized response bytes or a typed transport failure; and
- one orchestration function that validates the payload, prepares the P209
  request, calls the injected transport at most once, strictly parses the
  response and delegates semantic adjudication back to P209.

The adapter returns only a P209 selection or typed intervention and transport
failure evidence. It never returns coordinates, selectors, executable events,
retry instructions, raw provider text or a second attempt token.

## Invariants

1. Artifact source identity matches the exact P206 evidence envelope before a
   provider call.
2. Payload bytes match the declared digest and length; empty or oversized
   payloads fail before a provider call.
3. Serialized request and response bodies are measured against explicit
   policy ceilings before transport or deserialization; an oversized body is
   a terminal typed failure, never a truncation or retry signal.
4. Media type and dimensions come from a small policy-owned allowlist and are
   included in the artifact and invocation digests.
5. A nonempty redaction policy digest and redaction receipt digest are required
   even for repository-owned synthetic fixtures.
6. Retention is exactly `ephemeral_process_local`; the adapter exposes no save,
   log, cache or replay-of-bytes operation.
7. Artifact, request, evidence, capability and expiry identities agree before
   transport invocation.
8. One orchestration call invokes the transport zero or one time. Invalid
   input, malformed output, timeout, abstention or rejection never triggers a
   retry.
9. Provider output remains limited to P209's candidate identities or typed
   abstention. The adapter cannot broaden that response schema.
10. Raw payload bytes and raw provider output do not appear in errors, receipts,
   debug formatting or durable test snapshots.
11. Adjudication remains deterministic and effect-free after transport output
    is obtained.

## Consolidated Batch

1. Freeze the artifact, payload, invocation, transport and result contracts
   with strict serialization where a serialized boundary exists.
2. Add an architecture guard for the adapter crate's forbidden dependencies.
3. Implement payload and invocation digest validation plus one-shot orchestration.
4. Add a recording fake transport and repository-owned synthetic byte fixtures.
5. Prove one valid selected response reaches P209 adjudication after exactly
   one fake call.
6. Prove invalid artifact identity, digest, length, media type, dimensions,
   redaction, retention, capability, freshness or request binding makes zero
   transport calls.
7. Prove malformed, oversized, stale, mismatched and effect-smuggling provider
   responses make one call, produce no selection and never retry.
8. Prove ambiguous, unsupported and inconclusive responses retain their P209
   intervention types without retaining payload bytes.
9. Run changed-surface validation and record one clean source checkpoint.

## Provider-Free Fixture Matrix

The minimum matrix includes:

1. one valid synthetic payload, one fake-provider call and one bound candidate
   selection;
2. deterministic artifact and invocation digests;
3. source evidence, frame, context or geometry mismatch;
4. byte digest or byte-length mismatch;
5. empty and policy-oversized payloads;
6. serialized request or response above its exact policy ceiling;
7. unsupported media type, zero dimensions and dimensions above policy bounds;
8. missing or malformed redaction policy and receipt digests;
9. non-ephemeral retention posture;
10. stale artifact, stale request and capability mismatch;
11. malformed JSON, unknown fields and coordinate, event, retry or instruction
    smuggling in provider output;
12. typed ambiguous, unsupported and inconclusive responses; and
13. transport failure with exactly one call, no retry, no selection and no raw
    payload in the returned error.

## Delivery Budget

- Attempt 1: contracts, recording fake and one valid tracer.
- Attempt 2: consolidated fail-closed matrix and at most one semantic repair.
- Attempt 3: architecture guard, validation and closeout.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome
  progress.
- Overall source effort ceiling: 180 active minutes.

## Worker Assignments And Write Surface

- Primary: one challenge-lane owner implements the adapter contracts, recording
  fake, provider-free fixtures, architecture guard and validation receipt.
- No subagent, model-provider worker, browser worker, runtime operator,
  benchmark checkout or live-acceptance worker is assigned.
- P209 remains the sole owner of the provider protocol. P210 consumes its
  integrated contract and does not rewrite `visual_provider.rs` unless a
  separately recorded blocking contract defect requires dependency repair.
- PL-PLATFORM remains the owner of desktop capture, desktop services, provider
  admission and runtime surfaces. P210 neither edits nor shadows them.

Expected source writes are limited to:

- root `Cargo.toml` and the resulting shared `Cargo.lock` workspace entry;
- `crates/agent-browser-challenge-visual-adapter/` contracts and tests;
- one `scripts/test-challenge-visual-adapter-architecture.js` dependency guard;
- the corresponding package script in `package.json`; and
- this plan plus the bounded challenge-lane roadmap, runbook and active-lane
  projections at admission and checkpoint time.

The shared lockfile makes the initial changed-surface selection potentially
comprehensive. Run the selector at admission and reuse unaffected extracted
crate evidence only when its dependency closure is unchanged and the current
validation policy permits reuse.

## Deferred Successors

- A real model SDK, HTTP transport, provider credential or provider call.
- Browser or desktop frame capture, image encoding, OCR or redaction execution.
- Durable artifact storage, caching, telemetry containing pixels or replay of
  raw payload bytes.
- Candidate-to-coordinate translation and desktop-services execution.
- Public CLI, HTTP, MCP, generated-client, dashboard or Service State exposure.
- Installed fixture, development-runtime or live challenge acceptance.

The next effect-bearing packet after P210 should first bind a P209
`VisualRoundIntent` to fresh desktop-services candidate geometry and controller
authority using provider-free fixtures. A real provider and live challenge
acceptance remain separate, later gates.

## Evidence And Exit

Source acceptance requires the complete adapter fixture matrix, the new crate
architecture guard, challenge-control and adapter tests, workspace formatting,
strict workspace Clippy, changed-surface selection and diff hygiene at one
clean commit.

Publication requires P209's canonical integration receipt, a branch based on
that exact `main`, durable remote custody and the protected validation path in
effect at admission time.

## Stop Condition

Stop and replan if the adapter requires a network dependency, model-specific
schema, secret, browser frame, private pixel fixture, filesystem artifact,
desktop-services change, public Service schema or effect-capable call. Stop
before any browser launch, image capture, provider call, credential read,
CAPTCHA attempt, desktop input, installed runtime mutation, shared Service
State mutation, production effect or release.
