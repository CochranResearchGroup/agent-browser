# Plan 0115: CDP Automation Etiquette

Date: 2026-08-14

State: VALIDATED

Lane: P115

Source baseline: `040976499965132c4a1046c9b22b3b712913df6e`

## Goal

Reduce unnecessary browser instrumentation during ordinary automation without
pretending that CDP attachment is invisible or attempting to bypass a website
challenge. Preserve the current eager bootstrap as the default while adding one
bounded, explicitly selected posture that enables only the CDP capability needed
for initial page navigation.

## Evidence

- A fresh local browser currently attaches to every eligible page target and
  immediately sends `Page.enable`, `Runtime.enable`, `Network.enable`, and
  `Target.setAutoAttach` before the requested navigation runs.
- Controlled Google trials did not identify a deterministic executable or empty
  WebSocket boundary. Direct process-start navigation had the lowest observed
  challenge rate, while attach-then-navigate had a materially higher observed
  challenge rate in the small sample.
- Identical stock-Chrome trials produced both challenge and non-challenge
  outcomes. The external result is therefore an aggregate signal, not a
  deterministic correctness oracle.
- The installed Chromium candidate has two narrow patches. Only the
  `navigator.webdriver` change concerns automation disclosure; the LineBreaker
  change is a correctness repair.

## Frozen First Slice

Add an experimental local-Chrome bootstrap mode selected by
`AGENT_BROWSER_CDP_BOOTSTRAP_MODE=navigation_minimal`.

- The default remains `eager` and preserves the existing command sequence.
- `navigation_minimal` attaches to the selected target and enables `Page` only.
- It does not initially enable `Runtime` events, `Network` events, iframe
  auto-attachment, or debugger-resume behavior.
- A command that requires omitted capabilities must explicitly promote the
  target or enable the required domain before performing that command. The
  first slice promotes per CDP session for network-idle navigation; existing
  HAR and bounded network-capture commands enable Network explicitly.
- External CDP, provider, runtime-handoff, Lightpanda, CDP-free, and manual-login
  paths retain their existing behavior in this slice.
- This slice does not claim lower CAPTCHA incidence. It creates a deterministic
  experimental boundary that can be compared without changing browser source.

## Test Strategy

Use vertical TDD through the browser-manager interface:

1. a local launch in `navigation_minimal` sends `Page.enable` but does not send
   `Runtime.enable`, `Network.enable`, or `Target.setAutoAttach`;
2. the default eager posture retains the existing command sequence;
3. a network-idle navigation promotes the target before relying on network
   lifecycle events;
4. invalid bootstrap values fail closed before launching Chrome;
5. external and provider attachment remain eager;
6. existing browser, capture, streaming, remote-view, and service request tests
   remain green.

## Documentation And Runtime Boundary

Document the experimental environment variable in CLI help, README, the
repository Agent Browser skill, and the docs site. State that it is a diagnostic
posture, not a concealment guarantee, and that capability-rich commands opt into
their required domains.

After source validation, synchronize the installed application through the
normal local publisher and run install doctor. Do not close, copy, or replace
the six retained experiment browsers or their profiles merely to install the
new binary.

## Bounds

- Implementation attempts: `1/2`.
- Review and rework cycles: `1/1`.
- Installed checkpoint publishes: `1/1`.
- Live controlled comparison rounds: `1/1`.
- Retained browser/profile closure or replacement: zero.
- CAPTCHA interaction, challenge solving, account authentication, and external
  state mutation: zero.

## Acceptance Criteria

- Focused Rust tests prove the exact minimal and eager bootstrap command sets.
- Network-idle promotion is deterministic and tested.
- Invalid configuration fails before browser launch.
- Rust format, focused tests, strict Clippy, selected validation, and docs build
  pass.
- The installed binary, embedded dashboard, package/runtime manifest, and
  checkout binary are synchronized and `agent-browser install doctor` reports
  no new drift introduced by this slice.
- Any live comparison is reported as randomized aggregate evidence and leaves
  challenges untouched for operator review.

## Outcome

- Local Chrome accepts `eager` or `navigation_minimal`; invalid values fail in
  configuration selection before browser launch. The default and every
  attached-existing, provider, handoff, and Lightpanda constructor remain
  eager.
- Minimal bootstrap sends only `Page.enable`. Network-idle navigation promotes
  exactly the active CDP session, and repeated promotion is idempotent without
  marking sibling target sessions eager.
- Focused bootstrap tests passed 9/9, the complete browser-manager module passed
  42/42, strict Clippy and formatting passed, and the docs build plus remote-view
  documentation guard passed.
- The installed binary, checkout binary, dashboard runtime, workstation payload,
  and installed Agent Browser skill were synchronized. Runtime convergence was
  8/8 with zero stale runtimes. Doctor remained nonzero only for the deliberately
  retained duplicate-profile experiment pressure and an unrelated inactive Mail
  Receipts supervisor manifest that still names the previous executable hash.
- One bounded Google comparison used fresh isolated profiles and the same
  installed stealth Chromium. Eager bootstrap reached search results in 2/2
  trials. Minimal bootstrap reached results in 1/2 and received an untouched
  `/sorry/` challenge in 1/2. This tiny, non-randomly ordered sample does not
  satisfy the plan's randomized-comparison evidence standard or establish
  efficacy; it specifically rejects treating minimal bootstrap as a
  deterministic stealth improvement and is retained as observational evidence
  only.
