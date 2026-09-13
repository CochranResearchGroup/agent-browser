# CAPTCHA Guard Contract v1

Status: frozen for Plan 0180 provider-free validation

Owner: PL-CHALLENGE

Authority owner: Agent Browser service control plane

## Purpose

This contract defines the small internal boundary between challenge policy and
the shared desktop transaction. It is not a public raw-input API and does not
claim that a browser, pointer pattern, model, or solver can deterministically
pass an external anti-abuse decision.

The contract has three records:

- `captcha-guard-request.v1` carries policy-owned mode and effect bounds plus
  opaque service-issued target and authority references.
- `captcha-guard-capability.v1` reports the guarantees an exact adapter can
  provide without projecting unsupported platform behavior.
- `captcha-guard-receipt.v1` separates input delivery, challenge completion,
  downstream admission, replay, uncertainty, and human intervention.

The schemas describe the future pure guard boundary. Existing public CLI, HTTP,
MCP, generated-client, and dashboard contracts do not change in this packet.

## Trust Boundaries

### Agent Browser adapter

The Agent Browser adapter is trusted to resolve Service State and issue the
opaque browser, route, display, window, process, frame, candidate, and
controller-authority references. It also binds caller attribution, policy,
expiry, and idempotency. An external caller cannot mint or replace those
values.

### CAPTCHA guard

The guard validates the request, capability, state transition, remaining
budget, replay disposition, and requested action intent. It does not capture
pixels, run OCR, inspect processes, select windows, inject input, load
credentials, or mutate Service State.

### Solver

A solver is untrusted policy input. It may propose a bounded action intent from
the observation supplied by the guard. It never receives a raw desktop command
surface and never emits input directly. Model output, prompt text, detector
thresholds, and runtime-loaded assets are data whose exact digest is part of the
policy identity.

### Desktop services

Shared desktop services are trusted to revalidate the exact target, frame,
geometry, controller authority, focus, and effect budget. They journal before
input, report acknowledgement or uncertainty, suppress replay, and produce a
fresh after-state observation.

### Platform adapter

The X11, Windows, macOS, Wayland, or remote adapter may provide only the
capabilities it advertises. A successful operating-system call proves delivery
acknowledgement, not challenge completion or downstream admission.

### External verifier

The challenge provider and protected application are outside Agent Browser's
trust boundary. Challenge completion and downstream admission are separate
observations. Widget disappearance, callback delivery, or input acknowledgement
cannot substitute for either observation.

## Protected Assets

- exclusive operator and controller authority over the selected window;
- browser, profile, route, display, window, and process identity;
- fresh frame and geometry binding;
- finite attempt, step, event, and deadline budgets;
- effect-journal and replay integrity;
- credentials, tokens, pixels, OCR, page contents, and local system metadata;
- truthful challenge-completion and downstream-admission classification;
- the existing opaque remote-view handoff.

## Threats And Required Controls

| Threat | Required control | Terminal result |
| --- | --- | --- |
| Confused deputy or caller-selected target | Service-issued opaque target and authority references | reject before effect |
| Stale frame or geometry drift | Revalidate frame freshness and geometry epoch before each effect | `inconclusive` or intervention |
| Window or process replacement | Revalidate opaque window and process identity before each effect | `authority_lost` |
| Human and machine input race | Require current controller authority and serialize effects | `authority_lost` |
| Replay or duplicate delivery | Prepare the effect journal before input and replay the original receipt with zero new effects | replayed receipt |
| Partial or uncertain effect | Preserve the uncertain journal state and forbid automatic retry | intervention |
| Solver or prompt injection | Treat solver output as bounded action intent and validate every transition | reject or intervention |
| Unbounded multi-round loop | One attempt, finite step and event budgets, and an absolute deadline | `attempt_exhausted` |
| False success from callback or widget absence | Observe challenge completion and downstream admission separately | `inconclusive` or denied |
| Sensitive evidence retention | Keep pixels and OCR response-only and retain opaque evidence references | reject redaction failure |
| Platform guarantee inflation | Capability projection reports explicit booleans per adapter | capability failure |
| Arbitrary command injection | No shell, executable, raw coordinate, or caller-defined input fields | schema rejection |

## Request Semantics

`observe_only` has zero attempt, pointer-event, and key-event allowance.
`fixture_solve` and `operator_authorized_automated_solve` have exactly one
attempt and finite step, event, and deadline bounds. One attempt may include
multiple policy-defined steps, but exhausted or uncertain execution never
creates another attempt.

The request is an internal trusted-adapter record. It intentionally contains no
raw coordinates, process IDs, pixels, OCR transcripts, executable paths,
commands, credentials, tokens, network identity, proxy selection, or retry
count. `maxAttempts` is a policy-owned ceiling, not a caller retry knob.

## Receipt Semantics

`delivery` reports only what happened at the desktop effect boundary:
`no_effect`, `acknowledged`, `partial`, `uncertain`, or `rejected`.

`verification.challengeCompletion` reports whether the named challenge was
externally observed as passed, failed, or inconclusive.
`verification.downstreamAdmission` separately reports whether the protected
action was admitted. A `passed` guard state requires challenge completion but
does not imply downstream admission.

A replayed receipt names the original receipt, reports zero new effects, and
sets `emittedNewEffects` to false. Partial and uncertain effects require human
intervention or another terminal no-retry disposition.

Durable receipts contain opaque references and digests. They do not contain
pixels, OCR transcripts, page text, credentials, tokens, IP addresses, raw
process IDs, local paths, command lines, or provider stderr.

## State Vocabulary

- `not_present`: no eligible challenge exists in the bound observation.
- `checkbox_present`: one eligible bounded target is visible.
- `solving`: the one authorized attempt is active.
- `verification_in_progress`: effects ended and fresh verification is pending.
- `passed`: challenge completion is externally verified.
- `failed`: the challenge provider explicitly rejected the attempt.
- `expired`: the challenge or authority envelope expired.
- `challenge_open`: a supported initial control opened an unsupported or
  separately governed challenge.
- `inconclusive`: evidence cannot classify the result safely.
- `human_intervention_required`: automation stopped with a typed reason and an
  opaque handoff identifier.

## Dependency Direction

The frozen dependency direction is:

```text
agent-browser-captcha-guard
              |
              v
agent-browser-desktop-services
              ^
              |
 agent-browser CLI adapter
```

Neither future library crate may depend on the `agent-browser` CLI package. The
guard crate may depend only on the provider-neutral desktop-services interface
plus reviewed data-model dependencies. It may not own operating-system I/O,
network clients, OCR engines, image decoders, browser protocols, runtime
supervision, persistence, or CLI parsing.

The machine-readable dependency policy is
`captcha-guard-dependency-policy.v1.json`. A later extraction packet must add a
Cargo metadata gate that enforces it against the created manifests.

## Versioning

Any new required field, widened effect ceiling, changed state meaning, weakened
redaction, or expanded trust boundary requires a new schema version. Adding a
new challenge profile under unchanged semantics changes policy identity, not
the schema version.
