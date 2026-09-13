# Plan 0180 | CAPTCHA Guard Contract And Threat Model

Date: 2026-09-13

State: OPEN

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P169

Work item: `CochranResearchGroup/agent-browser#66`

Branch: `feature/turnstile-desktop-challenge`

Target: `main`

Integration: merge through the protected `main` workflow

## Objective

Freeze the provider-neutral request, capability, receipt, threat-model, and
dependency-direction contracts for guarded CAPTCHA automation before extracting
desktop services or adding another challenge solver. Prove the contracts with
provider-free deterministic fixtures and leave Plan 0169's live acceptance gate
unchanged.

## Current State

Plan 0169 already contains a Turnstile-specific locator and bounded X11
interaction on this branch. The existing CLI implementation owns desktop
capture, observations, controller authority, process and display binding,
effect journaling, replay suppression, input acknowledgement, and after-state
verification. Those types remain coupled to `cli/src/native/` and do not yet
define the smaller interface needed by the staged guard architecture.

The branch was reconciled with `origin/main` and published at `fc16ac08`. Issue
`#66` remains open, blocked, and live-gated. This packet is a provider-free
contract slice only.

## Authority And Effect Boundary

The operator authorized resuming anti-bot feature work. This packet may add or
change repository documentation, JSON schemas, deterministic fixtures, and
provider-free validation scripts. It may not install or launch a browser,
modify a profile, use credentials, emit desktop input, attempt a live CAPTCHA,
change a site identity, publish a release, or retry any previous live outcome.

## Consolidated Batch

- Define one internal guard request whose target and authority are opaque,
  service-issued references rather than caller-selected coordinates or process
  values.
- Define one capability projection with exact supported modes, challenge
  families, effect ceilings, and backend guarantees.
- Define one redacted receipt that separates delivery, challenge completion,
  downstream admission, replay, uncertainty, and human intervention.
- Record the threat model and trust boundaries for the guard, solver, Agent
  Browser adapter, desktop-services transaction, platform adapter, and external
  verifier.
- Freeze the dependency direction for the future
  `agent-browser-captcha-guard` and `agent-browser-desktop-services` crates.
- Add deterministic allow, deny, replay, stale-binding, ambiguous-target,
  authority-loss, partial-effect, and inconclusive-verification fixtures.

## Expected Write Surface

- `docs/dev/contracts/captcha-guard-*.v1.schema.json`
- `docs/dev/contracts/captcha-guard.v1.md`
- `docs/dev/contracts/examples/captcha-guard-contract-fixtures.v1.json`
- `docs/dev/contracts/README.md`
- `scripts/test-captcha-guard-contract.js`
- `package.json`
- this plan and the Plan 0169 branch-local continuation pointer

No CLI, service model, generated client, dashboard, runtime, installer, browser,
or platform adapter source belongs in this packet.

## Delivery Sequence And Budget

1. Map the current desktop transaction types and authority seams.
2. Freeze schemas and the threat model against the existing invariants.
3. Add valid and adversarial fixtures plus one deterministic AJV contract test.
4. Run the focused contract test, planning audit, documentation checks selected
   by changed files, and patch hygiene.
5. Commit and publish one coherent provider-free checkpoint.

Maximum implementation attempts: 2. Maximum review and repair cycles: 1.
Maximum provider, browser, desktop-input, installation, or live effects: 0.
The first decisive evidence deadline is a passing adversarial contract test in
this packet.

## Worker Assignments

The primary agent owns the plan, contract decisions, writes, validation, and
acceptance. No subagents are assigned. No other lane receives a shared-source
write from this packet.

## Acceptance Criteria

1. Request fixtures cannot supply raw coordinates, pixels, OCR transcripts,
   executable paths, shell commands, network identity, credentials, or retry
   counts outside the policy-owned effect budget.
2. Every effect-capable request binds caller, operation, challenge profile,
   exact opaque target, fresh frame, geometry epoch, authority, expiry,
   idempotency key, policy digest, and finite attempt and step budgets.
3. Observation-only requests have zero effect allowance. Live-capable requests
   allow exactly one attempt and a finite challenge-defined step budget.
4. Capability fixtures state actual capture, semantic, pointer, keyboard,
   focus, process-identity, journal, replay, verification, and handoff
   guarantees without inferring unsupported platform behavior.
5. Receipts distinguish no effect, acknowledged effect, uncertain or partial
   effect, challenge completion, downstream admission, and human intervention.
6. Replay fixtures emit zero new effects and uncertainty never authorizes a
   retry.
7. Durable fixtures contain no pixels, OCR text, page text, credentials, tokens,
   IP addresses, raw process IDs, local paths, command lines, or provider
   stderr.
8. The dependency policy forbids either future library crate from depending on
   the CLI package and forbids the guard crate from owning operating-system I/O.
9. The focused deterministic contract test rejects all frozen adversarial
   fixtures and accepts all frozen positive fixtures.
10. Plan 0169 remains blocked on its separate installed live-interaction gate.

## Evidence And Exit

| Requirement | Evidence | State |
| --- | --- | --- |
| Current transaction mapped | CodeGraph exploration of desktop interaction, capture, authority, and journal seams | complete |
| Request contract | Schema plus positive and adversarial fixtures | pending |
| Capability contract | Schema plus cross-platform guarantee fixtures | pending |
| Receipt contract | Schema plus replay, uncertainty, and verification fixtures | pending |
| Threat model and dependency direction | Contract document and machine-readable dependency policy | pending |
| Provider-free qualification | Focused test and selected documentation checks | pending |
| Installed or live acceptance | Separate Plan 0169 gate | not applicable |

Close this packet only when every provider-free row is complete and the exact
source checkpoint is published. A clean contract packet does not unblock or
complete Plan 0169's live acceptance.

## Non-Goals

- Extracting either planned Rust workspace crate
- Changing public CLI, HTTP, MCP, generated-client, or dashboard behavior
- Adding hCaptcha, visual-puzzle, audio, or accessibility solver behavior
- Browser fingerprint spoofing, proxy rotation, or identity rotation
- Installing a candidate or exercising a browser or desktop provider
- Executing or retrying a production challenge
- Opening or merging a pull request in this packet
