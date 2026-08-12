# Vision

Date: 2026-08-12
Status: PRODUCT DIRECTION

## Product Vision

Agent-browser should be the dependable browser control plane for agents,
software projects, and human operators. It should own browser identity,
processes, profiles, sessions, tabs, queues, policies, viewing, interaction,
health, intervention, and evidence while exposing one coherent product through
the CLI, HTTP API, MCP, generated client, dashboard, and documentation.

The product should make the common path simple without hiding authority or
uncertainty. A caller should be able to express an intent, discover whether the
required capabilities are available, perform bounded work, and receive a
result that explains what agent-browser observed, decided, changed, verified,
and left for a human.

## The Complete Browser Workspace

Web pages are only one part of a real browser workflow. Authentication and
site access can involve browser chrome, extensions, password managers,
passkeys, native dialogs, remote desktops, and windows that are intentionally
outside the page DOM and CDP screenshot surface.

Agent-browser should gain a first-class desktop perception and interaction
layer for those surfaces. The layer should observe and control the complete
service-owned browser workspace while preserving the same browser, profile,
session, display, route, lease, policy, and operator-handoff authority already
used by the service.

This is not a separate desktop robot attached beside agent-browser. It is the
non-DOM half of the same interaction system:

- page interaction uses DOM, accessibility, CDP, and browser-level input when
  those are the best tools;
- desktop interaction uses display frames, desktop semantics, coordinate
  mapping, and operating-system or remote-desktop input when the target is
  outside the page or CDP must be absent;
- policy can combine both without making callers reconstruct browser state or
  choose unsafe backends themselves;
- manual control remains a supported outcome rather than an automation
  failure.

## Product Promise

The interaction layer should make five promises:

1. **Bound to the right workspace.** Every observation and action names the
   exact service-owned browser, session, profile, display allocation, view
   stream, geometry, and control authority that made it valid.
2. **Evidence before input.** Agent-browser observes current state, identifies
   a target, and checks focus, freshness, geometry, policy, and lease authority
   before emitting pointer or keyboard input.
3. **Verification after input.** Emitted input is not treated as success. The
   layer observes the resulting state and returns a typed, inspectable receipt.
4. **One product surface.** CLI, API, MCP, generated client, dashboard, help,
   skills, and docs describe the same capabilities, requests, results, and
   limitations.
5. **Natural human continuation.** When consent, credentials, ambiguity, or
   unsupported secure UI requires a person, agent-browser presents the durable
   remote-view handoff and a clear completion signal without losing workflow
   identity.

## Observe, Locate, Act, Verify

The reusable interaction model is a bounded transaction:

1. **Observe** a fresh display frame and available semantic desktop evidence.
2. **Locate** candidate targets through deterministic or approved
   probabilistic detectors.
3. **Act** through the selected pointer or keyboard backend under an exclusive
   control authority.
4. **Verify** the resulting state against explicit postconditions.

Each stage produces evidence that the next stage consumes. Frames and targets
are opaque references, not unscoped coordinates copied between clients. If the
display, crop, scale, focus, route, frame, or controller changes, the action is
invalidated and must re-observe rather than guess.

Simple tools should lower into this model. A one-command click on a desktop
target and a multi-step visual workflow should use the same context, detector,
input, receipt, and stop-condition contracts.

## Deterministic Where It Matters

Determinism means that agent-browser can explain and reproduce its own work:

- a pinned frame and pinned detector produce the same candidate set;
- a replayable motion profile records the seed and generated trajectory;
- coordinate mapping is derived from retained geometry rather than inferred by
  the caller;
- request validation, leases, timeouts, retry budgets, and stop conditions are
  explicit;
- before and after evidence is bound to one interaction receipt.

Determinism does not mean that an external site must accept an interaction, a
challenge must pass, or an authentication ceremony must complete. Those are
separate observed outcomes. Learned and vision-language detectors may be
useful, but their uncertainty must remain visible and policy must decide when
their candidates are actionable, require corroboration, or require a human.

Smooth pointer paths, realistic focus transitions, key cadence, pauses, and
scrolling are general interaction profiles. They should improve compatibility
and user-visible behavior without being described as a guarantee of human
classification or anti-bot evasion.

## Reusable Mechanisms, Use-Case Policy

The foundation should supply replaceable mechanisms for:

- frame capture;
- desktop accessibility and window semantics;
- template, OCR, geometric, local-model, and approved remote-model location;
- coordinate mapping;
- pointer, keyboard, and text input;
- motion profiles;
- before and after verification;
- receipts, redaction, retention, and human handoff.

Use cases should supply:

- fixtures and target classes;
- detector assets or model selections;
- policy for required evidence and allowable providers;
- approval and secret-handling boundaries;
- retry budgets and terminal states;
- use-case-specific verification.

This prevents Turnstile, LastPass, passkeys, or any future workflow from
becoming a one-off input path. A use case may reveal missing abstractions, but
the remedy should improve the shared foundation whenever the need generalizes.

## Coherent And Discoverable By Default

Users should not need to know the internal capture provider, remote-view route,
or operating-system API to begin. The intended experience is:

- `agent-browser desktop` makes the capability family visible in CLI help;
- capability inspection explains what the selected workspace can observe and
  control before an action is attempted;
- concise commands cover observation and common bounded actions;
- recipes cover advanced multi-step work without a second execution engine;
- JSON output returns stable typed references and receipts;
- MCP exposes task-shaped tools and readable capability resources that are
  easy for agents to discover;
- the HTTP API and generated client expose the same contract for software
  integrations;
- the dashboard shows the selected workspace, proposed target, controller,
  verification result, and operator-takeover path.

All ingress adapters should submit work to the same service queue and consume
the same service-owned state. New functionality is incomplete when only one
client surface knows how to invoke or interpret it.

## Human Authority And Sensitive UI

Desktop perception reaches surfaces that may contain credentials, account
identities, one-time codes, private messages, and consent prompts. The layer
must therefore be more explicit about authority and retention than ordinary
page automation.

The default posture is:

- capture only the selected workspace and minimum region needed;
- keep sensitive frames ephemeral unless retention is explicitly authorized;
- redact logs and receipts while retaining enough structure to diagnose the
  interaction;
- never expose raw credentials merely because pixels or accessibility metadata
  made them observable;
- serialize agent input with human takeover;
- stop at biometric, secure-desktop, PIN, master-password, or user-consent
  boundaries unless an explicit approved workflow defines otherwise;
- use the opaque durable remote-view handoff when a human must continue.

Passkeys remain authentication capabilities with manual fallback. Challenge
handling remains policy-governed authorized access, not a generic bypass
promise. Controlled fixtures and provider-supplied testing modes are the
preferred development surfaces.

## Develop The Foundation Through Real Use Cases

The first work should prove display binding, frame receipts, deterministic
location, guarded input, verification, and cross-ingress parity on controlled
fixtures. Once that foundation is usable, discrete use cases should be added
one at a time.

Each use case has two responsibilities:

1. deliver the specific detector, policy, interaction, and verification needed
   for that workflow;
2. test whether the shared foundation makes the work natural, safe, and
   reusable.

If a use case needs raw coordinates in a client, duplicates logic across CLI
and MCP, cannot explain its selected frame, bypasses service leases, hides
uncertainty, or invents a new receipt shape, the foundation is working against
the product. The correct response is to revisit the shared abstraction rather
than institutionalize the workaround.

This feedback loop is intentional. The foundation should become more general
because real workflows exercise it, while remaining small enough that one use
case cannot turn its assumptions into global policy.

## Success

The vision is realized when an agent or software client can naturally discover
that a browser workspace supports desktop interaction, observe browser-external
UI, act through a policy-selected backend, verify the result, and hand control
to a human when needed, all without leaving the agent-browser authority model
or learning provider-specific plumbing.

The first target substrate is RDP/Guacamole because its display allocation,
view stream, control route, and durable operator handoff already exist. The
architecture succeeds only if later X11, Windows, Wayland, macOS, and other
remote-desktop providers can join without changing the use-case recipe model.
