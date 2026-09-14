# Plan 0187 | Challenge Countermeasure Control Plane Blueprint

Date: 2026-09-14

State: OPEN

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P169

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 remains the Turnstile and hCaptcha leaf

Branch: `challenge/p169-control-plane`

Target: `main`

Integration: merge through the protected `main` workflow after custody normalization

## Objective

Recover the product trunk behind the current hCaptcha test and fix loop. Define
a provider-neutral challenge countermeasure control plane, the shared desktop
transaction capability it consumes, the challenge-family and consumer branches
that plug into it, and the workfront needed to deliver those pieces in small
vertical slices.

The operator activated this plan on 2026-09-14. W0 through W4 authorize
repository reconciliation, provider-free source implementation, documentation,
and validation. They do not authorize a browser launch, desktop input, provider
mutation, a challenge attempt, a retry, production mutation, or release.

## Current State

W0 is active on `challenge/p169-control-plane` through parent issue #127.
Current-main merge checkpoint `99c85c683e40331d4d3f592c76f7ca3725d73ea7`
is published and includes the extracted lease-authority crate. The original
feature branch remains preserved at
`2ae7a68332b7a505c74bbd1e987d8a82fb52409d`. Challenge packets are normalized
as Plan 0188 and Plan 0189. Reconciliation validation and exact active-lane
readback remain before W0 exit and W1 implementation.

## Consolidation

The current branch contains three related layers that have become conflated:

1. Plan 0169 proves a Turnstile-specific desktop interaction leaf.
2. The branch-local Plan 0188 freezes a provider-neutral CAPTCHA guard contract.
3. The branch-local Plan 0189 adds an hCaptcha fixture leaf and exposes an
   interaction-freshness defect during the guarded pointer trajectory.

The first two challenge families are useful probes, but neither is the product
trunk. The trunk is the lifecycle and policy system that decides whether a
challenge should be avoided, attempted, verified, cooled down, or handed to a
human. It must remain useful when no CAPTCHA is present and when a future
challenge family does not use a checkbox.

At the planning-start snapshot, `origin/main` was 36 commits ahead of this
branch and this branch was 20 commits ahead of `origin/main`, with 64 changed
files. The challenge packets were then numbered Plan 0180 and Plan 0181, which
collided with different plans on current main. W0 renumbers them as Plan 0188
and Plan 0189 before establishing the integration baseline.

## Product Outcomes

Every challenge-aware task reaches one terminal control-plane outcome:

- `not_present`: no eligible challenge was observed.
- `avoided`: posture, profile reuse, pacing, or cooldown avoided the challenge.
- `passed`: an authorized resolution was delivered and independently verified.
- `denied`: the site or provider produced a terminal rejection.
- `intervention_required`: policy, capability, ambiguity, exhaustion, or a
  multi-round challenge requires a human.

Delivery success is not challenge completion, and challenge completion is not
downstream task admission. The receipt must preserve all three boundaries.

## Architecture

```text
consumer intent
    |
    v
challenge control plane
    |-- posture and avoidance
    |-- lifecycle and attempt budget
    |-- strategy and provider selection
    |-- completion verification
    |-- cooldown and intervention
    `-- receipt and replay
          |
          v
resolution profile adapter
    |-- Turnstile checkbox
    |-- hCaptcha checkbox
    |-- manual intervention
    `-- future visual or accessibility strategy
          |
          v
shared desktop transaction service
    |-- capture and semantic evidence
    |-- coordinate mapping and freshness
    |-- controller and process authority
    |-- bounded input transaction
    `-- after-state evidence and effect journal
          |
          v
Agent Browser authority adapter
    `-- Service State, browser, profile, route, display, controller, ledger
```

### Module: challenge control plane

This is the product trunk. It is a pure, provider-neutral policy and state
module provisionally named `agent-browser-challenge-control`. Its lean
interface is:

```rust
fn decide(envelope: ChallengeEnvelope) -> ChallengeDecision
```

`ChallengeEnvelope` carries typed observations, effective site policy,
capabilities, lifecycle state, attempt history, and downstream intent. It does
not carry caller-selected pixels, coordinates, executables, process IDs,
credentials, or arbitrary event sequences. `ChallengeDecision` can request an
observation, posture change, one registered resolution intent, verification,
cooldown, or human intervention. The pure module emits no effects.

### Module: desktop transaction service

This is a shared capability, not challenge policy. The existing desktop seams
should be extracted into a deep workspace crate provisionally named
`agent-browser-desktop-services`. Its interface accepts an opaque authority
binding plus a registered interaction intent and returns an attributable
receipt. It owns capture, evidence correlation, coordinate transforms,
freshness checks, authority fencing, event delivery, after-state observation,
and effect classification behind one transaction boundary.

Freshness is phase-bound. Pointer movement consumes time and changes the
interaction phase. Before `LeftDown`, the service must obtain or validate a
fresh observation and prove continuity of target, surface, geometry,
controller, display, route, browser process, and provider generation. Extending
the original observation timeout is not an acceptable repair.

### Adapter: Agent Browser authority

Agent Browser remains the initial authority owner and in-process client. It
resolves browser, profile, route, display, process, controller, and ledger
bindings, then presents only opaque capabilities to the two modules. Neither
extracted module creates a second Service State, browser manager, controller
lease, route registry, or operation ledger.

### Branch: detection profiles

Detection combines page, network, and desktop evidence through registered
profiles. A profile identifies a challenge family and confidence contract. It
does not decide policy or emit input. Turnstile and hCaptcha remain separate
profiles with separate fixtures and receipt identities.

### Branch: posture and avoidance

Avoidance is the preferred first strategy: persistent profile reuse, reasonable
pacing, headed mode, cooldown, and site-policy selection. It must not include
identity rotation, fingerprint deception, challenge farming, or retry loops.

### Branch: resolution providers

Resolution providers translate one control-plane decision into one registered
intent. The initial providers are bounded checkbox interaction and manual
intervention. Future visual reasoning or accessibility providers require their
own capability, policy, evidence, and effect budgets. They cannot inherit
checkbox authority implicitly.

### Branch: verification and admission

Verification observes a fresh after-state and classifies the challenge
lifecycle. A separate consumer adapter decides whether the original task may
continue. Authentication, recipes, navigation, and public-site workflows are
consumers of the challenge result, not owners of challenge policy.

## State Model

The durable lifecycle is:

```text
unobserved
  -> not_present
  -> detected
       -> avoided
       -> eligible
            -> resolving
                 -> passed
                 -> denied
                 -> cooldown
                 -> intervention_required
            -> intervention_required
       -> denied
```

Every transition records the policy revision, capability identity, attempt
number, evidence references, effect classification, and downstream admission
decision. Replay returns the prior receipt and emits no new effect.

## Consolidated Batch

The first implementation batch is W0 through W4 below. It establishes clean
custody, repairs the shared transaction invariant, extracts the two deep
modules, and proves them with Turnstile and hCaptcha fixtures. It excludes live
retry, visual challenge solving, consumer-wide integration, cross-platform
backends, a sister repository, and release work.

## W0 Reconciliation Decision

Current-main reconciliation is required before W1 or W2 begins, but rewriting
the published `feature/turnstile-desktop-challenge` history is not justified.
The 2026-09-14 decision is to preserve its exact decision tip
`2ae7a68332b7a505c74bbd1e987d8a82fb52409d`, create a new branch with the
`challenge/` product-lane prefix from that tip, and merge the freshly fetched
`origin/main` checkpoint into the new branch.

At decision time, `origin/main` was
`5d07b94f8db301378a73f103bd6e8d2b6d195c68`, 84 commits ahead of the challenge
branch, while the challenge branch had 22 unique commits including four prior
main-join merge commits. The published branch was unprotected and had no pull
request, but it remained durable shared custody. Rebasing it would replay 18
unique non-merge patches and require a history rewrite without improving build
performance.

A read-only synthetic merge initially reported three textual conflicts. By the
execution fetch, current main had advanced to
`994ed7b5f5c1deda5a968fa94a6ae3a82f04e2fc` and added one conflict in
`cli/src/native/action_runtime/runtime/launch.rs`. The other conflicts were
`RUNBOOK.md`, `scripts/open-rdp-guac-route-displays.js`, and
`scripts/test-development-presentation-provider.js`. The newly integrated
`agent-browser-lease-authority` crate does not directly edit
`desktop_interaction.rs`, `controlled_x11_provider.rs`, or
`desktop_locator.rs`, but it changes the authority kernel and CLI adapter that
the future desktop transaction module must consume. Building W2 against the
pre-extraction authority shape would create immediate rework.

The merge resolution preserves both sides' semantics and renames the challenge
packets to Plan 0188 and Plan 0189. Planning, architecture, changed-surface,
Rust formatting, and strict Clippy gates must pass before the new branch becomes
an implementation baseline. The old published branch remains a recovery and
evidence ref. No force push is part of this reconciliation.

## Workfront

### W0 | Custody and integration normalization

- Reconcile the branch with current main without erasing retained receipts.
- Renumber branch-local plans that collide with current main.
- Create a parent forge work item for the control-plane trunk and retain issue
  #66 as the current leaf.
- Assign one active implementation plan and update the active-lane catalog.
- Create the `challenge/` integration branch from the preserved feature tip and
  merge current `origin/main` without rewriting the old published branch.

Exit: current main, plan IDs, work item, branch, active lane, and baseline
commit agree. No source implementation starts before this checkpoint.

W0 integration checkpoint: merge commit
`99c85c683e40331d4d3f592c76f7ca3725d73ea7` joins current-main checkpoint
`994ed7b5f5c1deda5a968fa94a6ae3a82f04e2fc` without rewriting preserved feature
checkpoint `2ae7a68332b7a505c74bbd1e987d8a82fb52409d`. Parent issue #127 is open and
in progress. Plan 0188 and Plan 0189 are the normalized challenge packet
identities. The integration branch is published; validation remains the final
W0 exit gate.

W0 source and custody acceptance: activation checkpoint
`342bb8fa94eaf714f70f34328894e5d2867fbfe7` is published with the branch-local
active-lane proposal. Version synchronization, lease-authority architecture,
development presentation-provider fixtures, Rust formatting, strict workspace
Clippy, all 108 lease-authority tests, all 7 hCaptcha-focused tests, all 32
desktop-interaction-focused tests, route-confusion gates, the source-free
workstation fixture, and the planning audit pass. The canonical active-lane
projection remains pending through the protected-main pull request; issue #127
and the draft pull request provide shared discovery in the interim. W1 may
begin only on this same registered branch and remains provider-free.

### W1 | Desktop transaction correctness

- Freeze the phase-bound freshness contract with provider-free tests.
- Refresh or revalidate observation evidence between motion and button-down.
- Prove continuity of target, surface, geometry, controller, display, route,
  process, and provider generation.
- Preserve exact no-effect, effect-applied, and effect-uncertain outcomes.

Exit: deterministic tests cover long trajectories, geometry drift, controller
change, stale refresh, and zero-effect rejection. The live hCaptcha attempt is
still not retried in this workfront.

W1 source checkpoint: the desktop transaction now requires a distinct fresh
observation after pointer motion and before button-down. The refreshed target
must retain the same recipe target class, physical bounds, center, browser,
session, profile, display allocation, stream, route, coordinate mapping, and
geometry epoch as the planning observation. The existing guarded-event fence
then revalidates controller authority, surface and process identity, and input
provider generation before the button event. Capture freshness is timestamped
immediately after capture, before locator work, so recognition latency consumes
the 750 ms budget. Provider-free tests cover a successful long trajectory,
stale refresh, target movement, geometry change, and controller change while
preserving `effect_uncertain` for acknowledged motion with perception failure
and `cancelled_after_effect` for authority revocation. Formatting, strict
workspace Clippy, all 37 desktop-interaction-focused tests, all 7
hCaptcha-focused tests, and both controlled-X11-provider-focused tests pass.
No live hCaptcha retry or browser effect was run.

### W2 | Desktop services extraction

- Extract the transaction kernel into `agent-browser-desktop-services`.
- Keep platform input, capture, OCR, and window-system details behind adapters.
- Retain the CLI implementation as the first adapter during adoption.
- Add dependency rules that prevent the shared crate from importing Service
  State or challenge policy.

Exit: existing Turnstile and hCaptcha provider-free interaction tests pass
through the extracted interface with no public behavior change.

#### W2 architecture brief

Context and constraints: `desktop_interaction.rs` currently combines the pure
transaction state machine with CLI dispatch, Service State handoff lookup,
durable filesystem ledger I/O, and platform provider wiring. The extraction
must preserve serialized receipts and error codes byte-for-byte, retain the CLI
as the first adapter, and introduce no runtime or live effect.

Repository shape: `crates/agent-browser-desktop-services/` owns interaction
contracts, deterministic planning, phase validation, event sequencing, cleanup,
verification, and the provider-free in-memory ledger. `cli/src/native/` retains
command parsing, stream redaction, Service State projection, durable ledger
files, provider admission, capture, OCR, X11 input, and global runtime wiring.

Module contracts: the shared crate accepts only injected provider, authority,
clock, claim/coordinator, operation-ledger, and handoff traits. Platform and
Service adapters translate their local evidence into those contracts. The
crate has no upward CLI/native import and no CDP, async runtime, HTTP, image,
dashboard, generated-client, Service State, locator, or X11 dependency.

Testing and rollout: first freeze the dependency rule in
`test:desktop-services-crate-architecture`, then move one provider-free
transaction vertical at a time while keeping the existing CLI-focused tests
green. Add an independent crate test lane before deleting the old kernel.
Rollback is a normal commit revert because this phase changes source ownership
only and performs no installed-runtime or provider mutation.

Open risk: route claim serialization currently shares a module with
Service-State and external-fence adapters. Extract only its process-local pure
coordination primitive; keep external fence acquisition and service route
resolution in the CLI adapter.

W2 source checkpoint: `agent-browser-desktop-services` now owns the provider,
authority, clock, ledger, handoff, request, observation, event, receipt, and
error contracts; deterministic motion planning; phase and identity validation;
event sequencing and cleanup; verification; effect classification; replay; and
the process-local route coordinator. The CLI retains command parsing, provider
admission and wiring, Service State handoff projection, the durable filesystem
ledger adapter, stream redaction, capture, OCR, X11 input, and external route
fences. The architecture guard rejects upward native imports and CDP, async,
HTTP, image, Service State, locator, or platform-provider dependencies.

The architecture contract, strict workspace Clippy, formatting, 3 independent
desktop-services tests, all 37 CLI desktop-interaction tests, all 7
hCaptcha-focused tests, all 108 lease-authority tests, all 3 CDP tests, and the
remote-view documentation check pass. The prior CLI transaction kernel is
absent; existing Turnstile and hCaptcha tests call the extracted entrypoint
through the CLI adapters with no receipt, error-code, or effect behavior change.
No runtime, browser, provider, production, or release effect ran.

### W3 | Challenge control extraction

- Promote the frozen guard request, capability, receipt, and threat model into
  `agent-browser-challenge-control`.
- Add lifecycle, posture, attempt-budget, cooldown, provider-selection,
  verification, replay, and intervention decisions.
- Keep the decision engine pure and deterministic.
- Add compile-time dependency checks so it depends only on contract types.

Exit: a table-driven replay corpus proves every state transition and proves
that no decision can directly emit desktop or browser effects.

### W4 | Two-profile vertical slice

- Adapt Turnstile and hCaptcha detection and checkbox resolution to the common
  control plane.
- Preserve distinct profile IDs, fixtures, thresholds, and terminal states.
- Prove `not_present`, `eligible`, `passed`, `denied`, and
  `intervention_required` through provider-free scenarios.
- Add one no-launch Service contract path that returns the composite receipt.

Exit: both challenge families traverse one lifecycle without shared detector
heuristics or provider-specific branches in the control-plane core.

### W5 | Task-shaped Service orchestration

- Add a challenge-aware task request that owns observe, decide, resolve,
  verify, cooldown, and downstream admission.
- Keep low-level desktop primitives internal capability surfaces.
- Project task state and intervention through Service Status and resources.

Exit: one task receipt explains challenge state, delivery, verification, and
downstream admission without reconstructing multiple jobs.

### W6 | Consumer integration

- Integrate authentication, recipe, and navigation consumers one at a time.
- Give each consumer an explicit site-policy and downstream-admission adapter.
- Preserve consumer failure separately from challenge failure.

Exit: at least two consumers use the same trunk without importing provider
logic or desktop details.

### W7 | Visual reasoning and multi-round challenges

- Define a new evidence and capability contract before any implementation.
- Require synthetic, provider-controlled fixtures and human-intervention
  boundaries.
- Add explicit per-round and cumulative effect budgets.

Exit: a separate approved plan demonstrates a safe need beyond checkbox
interaction. This workfront is optional and is not implied by W0 through W6.

### W8 | Portability and optional external host

- Add a second platform adapter only after the shared interface is stable.
- Prove a versioned transport contract with an in-repo adapter.
- Consider a sister process or repository only after a second real client,
  separate release ownership, and justified lifecycle isolation exist.

Exit: portability evidence supports the boundary. Extraction is not a goal by
itself.

## Delivery Sequence And Budget

Execute W0 through W4 in order. W5 and W6 follow only after that trunk is
accepted. W7 and W8 are independent successor investments.

Estimated focused engineering effort:

| Workfront | Estimate |
| --- | --- |
| W0 custody normalization | 1 to 2 days |
| W1 transaction correctness | 1 to 2 days |
| W2 desktop services extraction | 3 to 5 days |
| W3 challenge control extraction | 2 to 4 days |
| W4 two-profile vertical slice | 2 to 4 days |
| W5 task orchestration | 3 to 5 days |
| W6 consumer integration | 2 to 4 days |
| W7 visual and multi-round work | 4 to 8 days |
| W8 portability or external host | 3 to 8 days |

The first trunk batch, W0 through W4, is approximately 9 to 17 focused
engineering days. W5 and W6 add approximately 5 to 9 days. Estimates are
planning ranges, not deadlines. Each workfront gets its own bounded plan,
attempt ceiling, validation selection, and stop rule.

## Test And Acceptance Ladder

1. Structured replay fixtures for challenge lifecycle and receipts.
2. Pure crate tests for policy, budgets, cooldown, replay, and redaction.
3. Desktop transaction tests with fake capture, authority, input, and clock.
4. Adapter tests for Service State binding and effect projection.
5. No-launch CLI, HTTP, MCP, and generated-client contract tests.
6. Repository CAPTCHA lab acceptance with synthetic provider keys.
7. One explicitly authorized installed interaction only after provider-free
   evidence, candidate identity, resource census, and a fresh effect budget.

No live third-party challenge is required to accept the trunk. Fixture success
does not authorize production or public-site use.

## Worker Assignments

Until W0 establishes a parent work item and active implementation plan, this
document has one coordinator and no implementation workers.

- Coordinator: owns ROADMAP, RUNBOOK, plan identity, dependency boundaries,
  integration sequence, and evidence reconciliation.
- Desktop transaction worker: W1 and W2 after assignment.
- Challenge control worker: W3 after the desktop interface is frozen.
- Vertical-slice worker: W4 after W2 and W3 expose reviewed interfaces.
- Consumer worker: W5 and W6 after W4 acceptance.

Only one worker may edit a shared contract surface at a time. The provider
contract lands before dependent adapters. Parallel work requires disjoint file
ownership recorded in the active-lane catalog.

## Rollout And Rollback

Adopt the new modules behind the existing Service action and preserve the
current implementation as the comparison path until two-profile acceptance.
Rollout order is provider-free tests, no-launch contracts, development
candidate, fixture acceptance, then consumer adoption. A failed stage rolls
back the adapter selection, not the retained evidence or receipt history.

The hCaptcha stale-observation receipt remains a regression fixture. It must
not be erased by replaying the live attempt.

## Risks And Decisions

- Risk: optimizing the hCaptcha leaf hardens provider quirks into the trunk.
  Decision: only typed profile adapters may contain provider heuristics.
- Risk: extracting crates creates shallow wrappers around the CLI monolith.
  Decision: extract complete transaction and decision invariants behind lean
  interfaces, then migrate callers incrementally.
- Risk: desktop freshness is treated as one timestamp.
  Decision: bind freshness and authority to interaction phases.
- Risk: a challenge pass is treated as task success.
  Decision: preserve separate delivery, completion, and admission fields.
- Risk: the architecture becomes a general anti-detection product.
  Decision: prohibit identity rotation, fingerprint deception, farming, and
  unbounded retry in the contract and threat model.
- Risk: branch drift and plan collisions corrupt integration history.
  Decision: W0 is a hard gate before source work.

## Evidence And Exit

This blueprint is accepted when ROADMAP and RUNBOOK point to it, planning
policy audit passes, the architecture note identifies it as the parent design,
and the working tree contains no unintended changes. Activation requires a
stable parent work item, normalized plan IDs, a current-main integration
baseline, and an active implementation plan for W0 or W1.

The immediate next packet is W0. The current hCaptcha browser and retained
receipts remain evidence only. No retry or runtime effect is authorized by this
plan.
