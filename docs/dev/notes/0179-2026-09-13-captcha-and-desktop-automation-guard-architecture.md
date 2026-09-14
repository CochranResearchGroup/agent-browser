# CAPTCHA And Desktop Automation Guard Architecture

Date: 2026-09-12

Product lane: PL-CHALLENGE

Disposition: design baseline adopted and refined by Plan 0187

Governing plan: Plan 0187

Related leaf plans: Plan 0169, branch-local Plan 0180, and branch-local Plan 0181

Source branch: `feature/turnstile-desktop-challenge`

## 2026-09-14 Product-Trunk Refinement

[Plan 0187](../plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md)
places this extraction design inside a broader provider-neutral challenge
control plane. The control plane owns posture and avoidance, lifecycle,
attempt budgets, strategy selection, completion verification, cooldown,
intervention, receipt, replay, and downstream admission. The CAPTCHA guard
defined here becomes the first resolution-profile contract rather than the
whole product trunk.

The shared desktop transaction module remains a sibling capability consumed by
the control plane. Agent Browser remains the authority adapter for browser,
profile, route, display, process, controller, and operation-ledger state.

## Objective

Create a staged architecture for safe CAPTCHA handling and reusable desktop
automation. Start with a small, independently testable Rust guard crate inside
Agent Browser that composes with the evolving native desktop-services
infrastructure. It must not create a parallel capture, semantic, coordinate,
input, authority, journal, verification, or handoff stack. Preserve an
extraction path to a sister desktop-control project with a versioned stdio
protocol only after the interface and authority model are proven by more than
one real client.

The product goal is policy-authorized automated CAPTCHA solving through guarded
desktop interaction, not a preemptive human handoff. An eligible challenge
should proceed through one bounded automated solve attempt without operator
intervention. A failed, unsupported, ambiguous, or exhausted attempt becomes a
typed human-intervention state. The system must not farm challenges, disguise
automation, rotate identity to escape a rejection, or blindly repeat a failed
attempt.

## Current Evidence

[Plan 0169](0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md)
owns the current Turnstile-specific proof. It already establishes fresh desktop
capture, deterministic location, controller fencing, X11 input, effect
journaling, and after-state verification. The controlled CAPTCHA lab proves
pointer delivery and callback state with vendor test keys. Production fieldwork
also proves that a delivered click and an external anti-abuse decision are
different outcomes.

The existing code has two useful seams:

1. `DesktopInteractionProvider` owns observation, probing, event execution, and
   after-state observation behind one provider interface.
2. Desktop evidence uses narrow frame, input, verification, and human-handoff
   adapters while the service control plane retains browser, route, display,
   controller, and operation-ledger authority.

These seams are deep enough to support an internal extraction. They do not yet
prove that a separate repository, process lifecycle, installation surface, or
cross-platform broker is warranted.

Graphiti discovery on 2026-09-12 returned general service ownership history but
no current CAPTCHA architecture decision. Current source, Plan 0169, the
controlled field note, and P110 are therefore the governing evidence.

## Architecture Decision

Adopt a staged hybrid:

1. Extract the provider-neutral desktop-services transaction kernel from the
   CLI package into a deep workspace crate provisionally named
   `agent-browser-desktop-services`.
2. Extract a pure, higher-churn policy crate provisionally named
   `agent-browser-captcha-guard` that depends on the lean default interface of
   `agent-browser-desktop-services`, never on the CLI package.
3. Reuse the shared desktop-services interfaces for capture, semantic evidence,
   coordinate mapping, controller authority, input, journaling, verification,
   capabilities, and human handoff.
4. Keep Agent Browser as the initial authority owner and in-process client.
5. Place X11 actuation behind the shared desktop effect seam, with direct
   `libxdo` evaluated as an adapter rather than exposed as the public interface.
6. Prove a versioned transport-neutral contract by placing an in-repo stdio
   adapter in front of the same desktop-services implementation.
7. Create a sister project only when a second client, stable protocol, separate
   release ownership, and a justified multi-platform backend plan all exist.

The intended dependency direction is:

```text
Agent Browser client         future stdio client
          │                         │
          └────────────┬────────────┘
                       ▼
       CAPTCHA guard policy and state module
                       │
                       ▼
          shared native desktop services
                       │
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
   X11 adapter    Windows adapter   macOS or remote adapter
```

The interface is the architectural seam. `libxdo`, XTEST, Windows input,
macOS accessibility, and remote display mechanisms are replaceable adapters.
No client receives a raw primitive that means “run an arbitrary desktop
command.” Native CAPTCHA handling consumes the same services and receipts as
every other desktop workflow. It does not fork or wrap those services with a
CAPTCHA-specific infrastructure layer.

## Option Assessment

### Option A | Agent Browser Module Only

This option keeps all policy and adapters under `cli/src/native/`.

Benefits:

- minimal packaging and runtime change;
- direct access to current service-owned browser and controller evidence;
- no new inter-process protocol or local daemon authority.

Costs:

- slower compile and test cycles as the CLI module grows;
- reusable policy remains coupled to Agent Browser types;
- later extraction becomes harder if OS backends and clients multiply.

Disposition: do not continue growing the feature as one large CLI module.

### Option B | Compartmentalized Workspace Crate

Create a crate provisionally named `agent-browser-captcha-guard` under
`crates/`. It owns pure request validation, policy decisions, state-machine
transitions, solver orchestration, redacted receipts, replay rules, and
verification classification. It performs no capture, OCR, process lookup,
filesystem mutation, network I/O, or input injection. Its production adapter
is the shared native desktop-services module, and its tests use in-memory
adapters at those same seams.

Benefits:

- focused builds and tests avoid rebuilding unrelated CLI surfaces;
- fixture-heavy policy development remains deterministic and provider-free;
- dependency direction becomes explicit;
- improvements to shared desktop capture, evidence, authority, effects, and
  verification automatically benefit guarded challenge workflows;
- the same public types can later back in-process and stdio adapters.

Costs:

- requires careful type ownership and migration from CLI-private models;
- premature generalization could weaken existing service invariants.

Disposition: recommended first implementation boundary.

### Option C | Sister Desktop-Control Project Now

Create a separate project that exposes a Codex-compatible stdio server and can
observe or control an explicitly selected operating-system window. Agent
Browser would become one client. An X11 backend could use `libxdo` directly.

Benefits:

- reusable by Codex and future non-browser clients;
- independent build, test, install, and release cycles;
- OS control can remain outside the browser CLI process.

Costs:

- the server becomes a high-authority local broker over arbitrary windows;
- process supervision, protocol compatibility, installation, logging,
  upgrades, crash recovery, and client authentication become product work;
- `libxdo` is X11-specific and does not supply a cross-platform architecture;
- split ownership can duplicate or weaken Agent Browser's route, display,
  controller, replay, and after-state proofs;
- one Agent Browser client does not yet demonstrate a real reusable boundary.

Disposition: defer until the extraction gate is met.

### Option D | Staged Hybrid

Build Option B first, then prove Option C's protocol shape inside the repository
by adapting the shared desktop-services implementation, without creating a
second product or a second desktop stack prematurely.

Disposition: selected.

## Proposed Module Boundaries

### Pure Guard Crate

The guard crate owns:

- `GuardRequest`, including caller, operation, target class, mode, and bounded
  effect budget;
- opaque window, frame, candidate, geometry, and authority references;
- challenge state such as `not_present`, `checkbox_present`,
  `solving`, `verification_in_progress`, `passed`, `failed`, `expired`,
  `challenge_open`, `inconclusive`, and `human_intervention_required`;
- policy modes `observe_only`, `fixture_solve`, and
  `operator_authorized_automated_solve`;
- deterministic admission and denial reasons;
- replay-safe state transitions;
- after-state classification and redacted receipts.

It does not own browser profiles, Service State, process discovery, window
selection, pixels, OCR engines, vendor tokens, credentials, raw coordinates,
input devices, or runtime supervision. These remain responsibilities of the
shared desktop-services implementation or its platform adapters.

### Shared Native Desktop Services

Move the provider-neutral transaction kernel into a workspace crate
provisionally named `agent-browser-desktop-services`. The CLI remains the owner
of Service State and maps current service authority into the crate's small
interface. The existing and evolving desktop-services infrastructure owns:

- display-bound capture and frame receipts;
- semantic window, process, focus, and accessibility evidence;
- coordinate-space and geometry mapping;
- controller leases and cross-process effect fencing;
- platform input adapters and partial-effect reporting;
- operation journaling, replay suppression, and after-state verification;
- capability projection and durable human handoff.

Challenge-specific development may extend these shared services when a missing
general capability is proven. It must not add a second CAPTCHA-only version of
the same mechanism. The shared service interface remains use-case-neutral;
Turnstile, hCaptcha, authentication prompts, and ordinary desktop fixtures
contribute policy, detectors, and verification rules above it.

The default crate feature set must remain provider-neutral and avoid Tokio,
HTTP, TLS, WebSocket, CLI parsing, embedded dashboard assets, and release
packaging. Platform code is compiled only when its named adapter feature is
selected. If the X11 adapter itself becomes expensive enough to disrupt the
shared-crate loop, it may move to its own deep crate after measurement. It must
not be split merely to create another package.

### Challenge Solver Module

A named solver consumes service-bound observations and produces bounded action
intents. It may use deterministic rules, a pinned local model, or a separately
approved remote reasoning adapter. It never injects input directly. The shared
desktop services validate and execute each intent against the current window,
frame, geometry, controller authority, effect journal, and remaining attempt
budget.

One automated attempt may contain several challenge-defined observe, select,
submit, and verify steps, including a bounded multi-round visual challenge.
Every step must re-observe current state. A failed submission, unsupported
challenge, ambiguous target, authority loss, or exhausted step budget ends the
automated attempt and requests human intervention. It does not start another
attempt.

### Agent Browser Adapter

Agent Browser owns:

- mapping the current browser, session, route, display, process generation,
  geometry epoch, and controller lease into one guard authority envelope;
- challenge-specific detector registration and site policy;
- issuing opaque candidate IDs bound to one fresh frame;
- translating a successful guard decision into the existing
  `DesktopInteractionProvider` transaction;
- durable operation-ledger and service-event projection;
- existing remote-view human handoff.

### Desktop Effect Adapter

The effect adapter owns:

- exact window and process revalidation before every event;
- frame capture and coordinate translation;
- pointer and keyboard event delivery;
- immediate acknowledgement and fresh after-state observation;
- typed partial-effect and uncertain-result reporting.

The initial backend remains the controlled X11 path. A direct `libxdo` adapter
is acceptable if it removes shell-process overhead while retaining fixed event
types, display ownership, process identity, controller fencing, and the effect
journal. It must not accept caller-provided shell arguments or arbitrary
commands.

### Future Stdio Host

The future host exposes a small versioned protocol over stdin and stdout. A
single high-level request should lower through the same guard transaction used
in-process and then through the same native desktop-services implementation.
The stdio host is a transport adapter, not a second desktop engine. Candidate
selection uses opaque IDs issued from a service-owned frame, not unbound
coordinates.

Read-only window discovery may enumerate bounded metadata. Control requires an
explicit target binding and capability that names the exact desktop session,
window identity, process start identity, geometry epoch, caller, operation ID,
expiry, and effect budget. Discovery alone never grants control.

Protocol output is structured data only. Diagnostics go to stderr and must not
contain pixels, OCR transcripts, credentials, tokens, private titles, or raw
window contents. Unknown protocol versions, capabilities, targets, or authority
fields fail closed.

## Cross-Platform Posture

`libxdo` is one X11 backend, not the abstraction. The architecture must permit:

- X11 through direct `libxdo` or the existing controlled XTEST path;
- Wayland only through a compositor-approved portal or another explicit
  provider, never by pretending X11 guarantees apply;
- Windows through reviewed window identity plus UI Automation or bounded input;
- macOS through reviewed Accessibility and event APIs;
- RDP or Guacamole through the existing service-owned route and handoff model.

Each backend advertises its actual capture, semantic, pointer, keyboard,
focus, and verification capabilities. Unsupported guarantees are typed
capability failures.

## CAPTCHA And Automation Guard Policy

The normal eligible path is one automated solve attempt without human handoff.
Automated effects require a named challenge policy, an exact step budget, and
an exact attempt budget. Observation-only mode remains available for unknown or
disallowed challenge classes.

Allowed initial modes:

1. repository-owned synthetic fixtures;
2. vendor-published integration test keys;
3. deterministic and approved model-assisted checkbox or visual-puzzle solving;
4. one policy-authorized automated attempt on an exactly bound live window;
5. multi-step and multi-round action inside that one attempt when the challenge
   profile defines a finite step budget;
6. read-only state classification and durable redacted receipts;
7. handoff to the existing opaque remote-view URL only after the automated path
   fails, is unsupported, becomes ambiguous, or exhausts its budget.

Excluded modes:

- challenge farms, token brokerage, or outsourced solver APIs;
- fingerprint spoofing or concealment of automation;
- blind retry after a failed attempt, browser rotation, profile rotation, or
  proxy rotation;
- generic click loops or caller-supplied unbound coordinates;
- interaction with secure desktops, consent prompts, biometrics, payments, or
  destructive controls.

The term `solve` is reserved for an externally verified challenge completion.
Input acknowledgement, widget disappearance, or a vendor callback alone is not
proof that the protected downstream action was admitted.

## Rapid Iteration Architecture

The full `agent-browser` binary is an integration artifact, not the inner
development loop. Desktop and challenge work uses progressively wider tiers.
A wider tier runs only when the narrower tier passes and the changed surface
requires it.

### Compilation Compartments

1. `agent-browser-desktop-services` contains the provider-neutral desktop
   transaction, authority value types, evidence contracts, effect budget,
   replay state, verification result, and in-memory fixture adapter.
2. `agent-browser-captcha-guard` contains challenge detectors, solver policy,
   multi-step attempt orchestration, and challenge-specific verification. It
   depends on the default provider-neutral desktop-services interface.
3. Platform adapters remain named optional features of desktop services until
   measurement justifies a deeper crate. The initial X11 implementation is the
   first platform adapter.
4. `agent-browser` depends on both crates and supplies the Service State,
   ingress, persistence, dashboard, install, and supervisor adapters. Changes
   confined to either library do not require linking this binary during the
   inner loop.

The intended package dependency direction is:

```text
agent-browser-captcha-guard
              │
              ▼
agent-browser-desktop-services
              ▲
              │
 agent-browser CLI adapter
```

Neither library depends on `agent-browser`. A deterministic dependency check
must reject reverse imports and accidental additions of heavyweight CLI-only
dependencies.

### Feedback Tiers

Tier 0 is structured replay. Most solver changes run against serialized
observations, candidate geometry, challenge states, and expected action traces.
No image decoder, OCR process, browser, X server, network, or CLI binary is
started.

Tier 1 is guard-crate validation. Run the focused
`agent-browser-captcha-guard` library test through `scripts/ci/cargo-safe.sh`.
This is the normal red-green solver loop.

Tier 2 is shared desktop-services validation. Run the focused
`agent-browser-desktop-services` library test when transaction, authority,
receipt, replay, or verification semantics change.

Tier 3 is adapter validation. Enable only the affected platform feature and
run the controlled local fixture. X11 work uses an isolated Xvfb or owned RDP
fixture and never requires a production browser or profile.

Tier 4 is Agent Browser integration. Run selected CLI and contract tests once
the library packet is coherent. Use `pnpm validation:select` from the batch
baseline to identify every affected integration surface.

Tier 5 is candidate qualification. Run `pnpm build:development-candidate`,
development installation, and governed acceptance once for the frozen batch.
Do not run an optimized candidate build after each detector, prompt, fixture,
or solver change. Reserve the full release build for an explicit release gate.

### Initial Wall-Clock Budgets

Phase 0 records cold, warm p50, and warm p95 measurements on the current WSL
host. The initial targets are:

- Tier 0 structured replay: warm p95 at or below 2 seconds;
- Tier 1 focused guard test: warm p95 at or below 8 seconds;
- Tier 2 focused desktop-services test: warm p95 at or below 20 seconds;
- Tier 3 one controlled adapter fixture: warm p95 at or below 60 seconds;
- Tier 4 selected CLI integration: at or below 3 minutes;
- Tier 5 optimized development candidate: at or below 15 minutes and no more
  than once per frozen batch.

These are provisional budgets, not claims about current performance. A tier
that misses its budget is profiled before its timeout is raised. Record compile
time, link time, test time, cache status, peak memory, and which package or
feature invalidated the build.

### Runtime-Loaded Solver Assets

Prompts, detector thresholds, solver manifests, and fixture expectations should
be versioned data where changing them does not require changing Rust types or
safety invariants. The development replay runner may load them at runtime and
records their content digest in every result. A frozen candidate pins exact
digests before installed or live acceptance.

Pixels and OCR remain adapter inputs rather than guard-core dependencies. Most
cases use small structured evidence fixtures. A smaller image corpus exercises
the detector adapter, and the local CAPTCHA lab exercises rendered widgets.
Only the final governed tier uses a live external challenge.

### Cache And Link Discipline

- use the existing `cargo-safe.sh` wrapper so incremental artifacts, `sccache`,
  `mold`, admission, and cgroup limits remain coherent;
- reuse the shared target directory for ordinary iteration;
- avoid `version:sync`, dashboard builds, binary embedding, and optimized
  linking in Tiers 0 through 3;
- add package scripts for the focused guard, desktop-services, and affected
  adapter tiers so agents do not improvise broad Cargo commands;
- teach `scripts/dev/select-validation.js` the new crate-to-gate mapping and
  retain full workspace formatting and Clippy at the completed batch boundary.

### Iteration Acceptance

Rapid iteration is proven only when:

1. a solver-policy change can reach a failing then passing Tier 1 test without
   compiling or linking `agent-browser`;
2. a desktop transaction change can reach Tier 2 without compiling HTTP, TLS,
   WebSocket, dashboard embedding, or release packaging dependencies;
3. an X11 adapter change can run its controlled fixture without building or
   installing a candidate;
4. the selection helper widens library changes to the correct CLI and contract
   gates before merge readiness;
5. the measured tier budgets and cache identities are recorded in the packet
   evidence.

## Consolidated Batch

The roadmap is divided into seven bounded packets:

1. freeze the guard vocabulary, threat model, request schema, receipt schema,
   capability model, and compile-time dependency rule;
2. extract the provider-neutral desktop-services crate, focused commands, and
   validation selection without behavior change;
3. extract the pure guard crate, replay runner, and fixture tiers while
   retaining the shared desktop-services implementation;
4. adapt Plan 0169 challenge profiles through the crate and the current shared
   X11 provider while preserving all existing service authority;
5. prove one non-CAPTCHA window-control fixture so the seam is not shaped only
   around challenge widgets;
6. prove one bounded automated visual-challenge solver through the shared
   desktop-services transaction with human intervention only on failure;
7. add an in-repo stdio host, prove the second client, and evaluate
   sister-project extraction plus cross-platform investment against the evidence
   gates below.

Each packet receives its own implementation plan before source work begins.
Plan 0169 remains the current Turnstile-specific lane and does not acquire the
authority of the broader packets by reference.

## Delivery Sequence And Budget

### Phase 0 | Contract And Threat Model

- enumerate protected assets, callers, windows, display sessions, controller
  conflicts, stale frames, coordinate drift, replay, partial effects, protocol
  injection, and sensitive evidence;
- freeze the smallest useful request and receipt interface;
- record a baseline for focused compile and test latency.

Exit: contract fixtures cover allow, deny, replay, stale binding, ambiguous
target, authority loss, partial effect, and inconclusive verification.

### Phase 1 | Shared Desktop-Services Crate

- extract the provider-neutral transaction kernel from `cli/src/native/` into
  `agent-browser-desktop-services` without changing behavior;
- keep Service State loading, ingress, projections, installation, and
  supervision in the Agent Browser adapter;
- add focused package commands, a dependency allowlist, and validation-selector
  mappings;
- measure cold and warm package validation against the Phase 0 baseline.

Exit: focused desktop-services fixtures pass without linking the CLI binary,
the CLI adapter preserves existing behavior, and the Tier 2 budget is met or a
measured follow-up is recorded.

### Phase 2 | CAPTCHA Guard Crate

- create the workspace crate with minimal dependencies and no operating-system
  I/O;
- move policy and state transitions behind the new interface without changing
  CLI, HTTP, MCP, generated client, or dashboard behavior;
- consume capture, semantic evidence, coordinate mapping, authority, effects,
  journaling, verification, capabilities, and handoff only through the shared
  desktop-services seams;
- reject any extraction that duplicates one of those native mechanisms;
- enforce that the guard crate never depends on `cli`.

Exit: provider-free focused tests pass, existing focused desktop tests remain
green, and editing only the pure crate does not require compiling the CLI
binary.

### Phase 3 | X11 Adapter Proof

- route the controlled X11 provider through the guard interface;
- compare direct `libxdo` with the existing XTEST implementation for process
  overhead, error fidelity, focus proof, event semantics, and packaging;
- retain the safer implementation unless measurements demonstrate a material
  benefit without loss of invariants.

Exit: synthetic desktop and CAPTCHA-lab fixtures pass with exact event budgets,
replay suppression, stale-window rejection, and after-state receipts.

### Phase 4 | General Desktop Fixture

- add one repository-owned non-CAPTCHA application window fixture;
- prove read-only discovery, explicit target binding, one bounded action, and
  human takeover;
- verify that CAPTCHA profiles remain policy adapters, not core concepts.

Exit: the same guard transaction works for browser and non-browser fixtures
without importing Agent Browser service models into the pure crate.

### Phase 5 | Automated Solver Proof

- define one named visual-challenge solver that produces action intents rather
  than direct input;
- exercise success, multi-round success, failed submission, ambiguous target,
  unsupported challenge, stale observation, and exhausted-budget fixtures;
- prove that successful eligible fixtures complete without human handoff;
- prove that every terminal failure produces one typed human-intervention state
  and no automatic second attempt.

Exit: a bounded automated fixture solve passes end to end through shared native
desktop services, while every failed path stops before another attempt and
returns the existing opaque remote-view handoff.

### Phase 6 | Experimental Stdio Host

- specify protocol version negotiation and capability discovery;
- implement one child-process transport adapter over the native desktop
  services with structured stdin and stdout;
- use one Agent Browser adapter and one direct fixture client;
- add supervisor, cancellation, EOF, crash, duplicate request, and partial
  write fixtures without installing a user service.

Exit: in-process and stdio adapters produce semantically equivalent receipts
for the same fixtures, and host loss cannot cause an unjournaled repeat effect.

### Phase 7 | Sister-Project Decision

Approve extraction only if all are true:

1. two independent real clients need the interface;
2. in-process and stdio adapters have remained stable through at least two
   implementation packets;
3. the host has a named maintainer, versioning policy, compatibility matrix,
   security model, installer, supervisor, doctor, and rollback plan;
4. Agent Browser can prove its browser and controller authority across the
   process boundary without sharing mutable Service State;
5. at least one non-X11 backend has a credible identity and effect model;
6. repository split costs are lower than workspace-crate costs.

If any gate fails, retain the crate and experimental host in Agent Browser.

Maximum architecture retries per packet: 2. Maximum live CAPTCHA effects under
this roadmap: 0. Any live automated solve requires its own bounded plan, named
challenge policy, explicit operator authority, and finite step and attempt
budgets.

## Worker Assignments

The primary agent owns roadmap reconciliation, interface decisions, source
custody, and acceptance. Future implementation plans may allocate independent
workers only to non-overlapping provider-free fixtures, protocol review, or
platform research. No worker receives live desktop, browser, profile,
credential, CAPTCHA, installation, or supervisor authority by implication.

## Acceptance Criteria

1. One pure crate owns guard policy and transitions with no CLI or OS dependency.
2. The primary guard interface is smaller than its implementation and does not
   expose arbitrary commands, raw display selection, or unbound coordinates.
3. Native execution reuses the shared desktop-services capture, evidence,
   coordinate, controller, effect, journal, verification, capability, and
   handoff mechanisms without a parallel CAPTCHA stack.
4. Agent Browser retains browser, profile, route, display, controller, and
   operation-ledger authority.
5. Every effect binds caller, operation, exact window and process identity,
   frame, geometry epoch, candidate, authority, expiry, and effect budget.
6. Replay emits no new input; uncertainty never becomes permission to retry.
7. The X11 adapter passes synthetic success, stale focus, replaced process,
   changed geometry, lost controller, partial event, and failed verification
   fixtures.
8. An eligible policy-authorized challenge completes one bounded automated
   attempt without human handoff, including bounded visual-puzzle steps when its
   solver profile permits them.
9. A failed, unsupported, ambiguous, authority-lost, or exhausted attempt emits
   one typed human-intervention result and no automatic second attempt.
10. Focused pure-crate validation has a materially shorter warm development
    cycle than the equivalent CLI package validation, measured and recorded at
    Phase 0 and Phase 2.
11. The stdio protocol, if built, is versioned, capability-negotiated,
   structured, replay-safe, crash-safe, and free of private durable evidence.
12. Tier 1 and Tier 2 meet their measured rapid-feedback budgets without
    linking the Agent Browser binary.
13. Sister-project extraction occurs only after every Phase 7 gate passes.

## Evidence And Exit

Each implementation packet records its baseline, dependency graph, focused
validation, elapsed compile and test times, fixture matrix, protocol or schema
identity, redaction checks, and exact remaining gate. Architecture acceptance
requires provider-free evidence from both CAPTCHA and non-CAPTCHA fixtures.

Close this roadmap only when the internal guard crate is accepted and the
sister-project decision is recorded as either approved with its own repository
plan or deliberately retained in-tree. Do not equate a completed Plan 0169
interaction, stdio prototype, or successful vendor callback with roadmap
completion.

## Non-Goals

- implementing source code in this planning packet;
- installing, repairing, restarting, or supervising a runtime;
- executing a production CAPTCHA solve or retrying any prior challenge in this
  planning packet;
- creating a general-purpose unauthenticated desktop-control daemon;
- promising that one input pattern will pass external anti-abuse systems;
- replacing Agent Browser's service authority with Codex process identity;
- selecting final sister-project naming, repository ownership, or release date.
