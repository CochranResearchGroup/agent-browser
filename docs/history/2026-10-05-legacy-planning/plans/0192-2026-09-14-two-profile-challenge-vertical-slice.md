# Plan 0192 | Two-Profile Challenge Vertical Slice

Date: 2026-09-14

State: COMPLETE

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P169

Parent plan: Plan 0187 W4

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 remains the
Turnstile and hCaptcha leaf

Branch: `challenge/p169-control-plane`

Target: `main`

## Objective

Route the existing Turnstile and hCaptcha checkbox profiles through the pure
challenge lifecycle and expose one no-launch Service request that returns a
composite provider-free receipt. Preserve the profiles as distinct detection
and interaction identities while keeping lifecycle evaluation generic.

## Current Baseline

The reconciled branch starts this packet at
`e1a43ed645a8b41cb62192a0e13292e35c3d60b6`. It already contains the extracted
desktop-services and challenge-control crates, the phase-bound freshness fix,
and distinct Turnstile and hCaptcha locator fixtures. The worktree is clean and
current `origin/main` is already merged.

## Consolidated Batch

This packet contains one vertical outcome:

1. Register immutable Turnstile and hCaptcha challenge profiles that reference
   their existing locator, interaction recipe, version, and threshold.
2. Evaluate `not_present`, `eligible`, `passed`, `denied`, and
   `intervention_required` scenarios through the same lifecycle function.
3. Add `challenge_control_evaluate` to the HTTP and MCP Service request
   contract as an early no-launch action.
4. Return a composite receipt that separates scenario evidence, delivery,
   verification, terminal outcome, and effect emission.

## Scope and Safety Boundary

The request accepts only a registered `challengeProfileId` and one enumerated
`scenarioOutcome`. It does not accept pixels, coordinates, selectors, detector
thresholds, provider choices, credentials, timing controls, or retry counts.
Every receipt is marked `provider_free_scenario` and `emittedEffects=false`.

This plan authorizes repository source, contract, documentation, and
provider-free validation only. It does not authorize browser launch, desktop
input, CAPTCHA interaction, provider mutation, challenge retry, installation,
shared-runtime mutation, production effect, or release.

## Delivery Sequence and Budget

- Attempt 1: implement the profile registry, common evaluator, no-launch
  Service adapter, contract projection, generated client types, and focused
  tests.
- Validation budget: one focused crate test pass, one focused native Service
  request and dispatch pass, generated-client checks, architecture checks,
  formatting, and strict workspace Clippy.
- Rework budget: one bounded correction pass for failures caused by this
  packet. A second semantic defect or any need for live evidence stops the
  packet for replanning.

## Worker Assignments

The primary session owns planning, implementation, validation, and closeout.
No subagent, benchmark worker, fixture worktree, browser worker, or runtime
operator is assigned.

## Expected Write Surface

- `crates/agent-browser-challenge-control/`
- `cli/Cargo.toml` and the CLI challenge-control adapter and routing
- `cli/src/native/service_request.rs` and `service_contracts.rs`
- `docs/dev/contracts/service-request.v1.schema.json`
- generated `@agent-browser/client` request types and their generator
- required CLI, README, skill, and docs-site user guidance
- focused provider-free tests and this plan

Shared roadmap, runbook, and active-lane catalog files remain outside this
packet until the source checkpoint is ready for normal integration custody.

## Evidence and Exit

Exit requires:

- both profile identities retain different locator IDs, profile versions,
  detector digests, and fixture-backed thresholds;
- all five scenarios produce deterministic composite receipts for both
  profiles through one generic evaluator;
- the Service request normalizes and dispatches without a browser manager and
  always reports that it emitted no effects;
- schema, Rust action registry, MCP and HTTP parity, and generated client types
  agree;
- challenge-control and desktop-services architecture guards pass;
- selected focused tests, formatting, and strict workspace Clippy pass.

Stop before any live CAPTCHA, provider, browser, installation, shared-runtime,
production, or release action.

## Completion

The packet is source-complete. `agent-browser-challenge-control` now owns two
immutable profiles whose recipe IDs come from `agent-browser-desktop-services`.
The CLI detector consumes those shared locator IDs, profile versions, and
thresholds, while a focused binding test verifies the exact detector digests.
Both profiles traverse one generic evaluator for all five required outcomes.

`challenge_control_evaluate` is registered across Rust, HTTP, MCP, the
canonical field-role ledger, and generated client types. Its early daemon
dispatch passes with no browser manager and returns a receipt marked
`provider_free_scenario` and `emittedEffects=false`.

Passing evidence:

- all 4 challenge-control crate tests;
- all 4 challenge-control CLI adapter, normalization, and no-launch dispatch
  tests;
- the exact locator-to-control-profile binding test;
- the complete supported-action normalization fixture and canonical 97-field
  schema and role-ledger test;
- focused Service contract metadata tests;
- Service API and MCP parity at 114 actions and 97 canonical fields;
- generated Service client contract, type, export, request, observability,
  managed-profile, and no-launch example checks;
- challenge-control and desktop-services architecture checks;
- docs-site production build and remote-view documentation check;
- workspace formatting and strict workspace Clippy.

No browser, CAPTCHA, desktop input, provider, install, shared-runtime,
production, or release effect ran.
