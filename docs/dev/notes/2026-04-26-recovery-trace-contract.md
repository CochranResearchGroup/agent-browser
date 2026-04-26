# Recovery Trace Contract

Date: 2026-04-26

## Context

The `service-model-phase-0` branch now emits and validates a canonical browser
recovery trace for both HTTP and MCP clients. This note records what that trace
means, what live smoke coverage proves today, and what remains before browser
crash detection is mature enough for always-on service use.

The current implementation is deliberately narrow. It documents observable
service behavior after a queued command discovers that the active browser is no
longer usable. It is not yet a complete service supervisor or a full crash
policy engine.

## Current Contract

When a queued browser command detects a stale active browser and the service
successfully relaunches it, clients should be able to read the following
ordered sequence from the service trace surface:

1. A `browser_health_changed` event for the affected browser with
   `currentHealth` set to a stale value such as `process_exited` or
   `cdp_disconnected` and `details.currentReasonKind` set to the same
   structured recovery vocabulary.
2. A `browser_recovery_started` event for the same browser with
   `details.reasonKind`, `details.reason`, `details.attempt`,
   `details.retryBudget`, and `details.nextRetryDelayMs` populated.
3. A later `browser_health_changed` event for the same browser with
   `currentHealth: "ready"` after relaunch.

If the next recovery attempt would exceed the default retained-event retry
budget, the service marks the browser `faulted`, records the faulted health
transition as the incident signal, and fails the command instead of relaunching
the browser again.

HTTP clients read this sequence from `/api/service/trace`. MCP clients read
the same persisted service state through the `service_trace` tool. The shared
smoke assertion in `scripts/smoke-utils.js` now enforces the same ordering and
reason and retry-budget contract for both clients.

## What The Live Smokes Prove

The HTTP live recovery smoke proves that the stream server API can launch a
browser, observe a simulated crash or disconnect through the service state
path, recover on the next queued command, and return the canonical sequence
from `/api/service/trace`.

The MCP live recovery smoke proves the same behavior through the MCP adapter:
the browser command path records recovery through the service queue, and
`service_trace` exposes the same ordered events and crash reason that HTTP sees.

Together, these smokes prove API and MCP parity for the retained recovery trace
contract. They also guard against future regressions where one client surface
sees stale health or ready health but not the recovery-started transition.

## Known Gaps

- Crash classification is still coarse. `process_exited` and
  `cdp_disconnected` are useful operator signals, but the service should later
  distinguish clean close, crash, killed process, port loss, hung DevTools,
  degraded target discovery, and browser shutdown requested by policy.
- Recovery policy has a default retained-event retry budget, but it is not yet
  configurable per service, site, profile, or task. The service still needs
  explicit policy configuration and operator override behavior.
- Reconciliation and command-time detection are not yet unified into one
  supervisor model. Background reconciliation can mark browser health, while
  queued commands can trigger relaunch. The service should eventually centralize
  ownership so all clients see one authoritative lifecycle.
- Dependent streams are not yet fully modeled. Screencast, tab-sharing,
  dashboard viewing, and future remote headed-browser streams need clear
  behavior when the underlying browser exits or CDP disconnects.
- Recovery events are retained, but incident handling policy is still basic.
  The service should eventually support severity, acknowledgement expectations,
  escalation, and service or task attribution rules for repeated failures.

## Next Engineering Slice

The best next backend slice is to harden crash classification and recovery
policy before adding more client controls. A useful definition of done:

- Promote the default retry budget and backoff values into explicit service,
  site, profile, or task policy.
- Add operator override behavior for intentionally retrying a faulted browser.
- Preserve the current public trace fields while adding policy source metadata
  once configurable policy exists.
- Extend live HTTP and MCP recovery smokes to cover the blocked crash-loop path.
- Keep the dashboard as a consumer, not the authority. It should display the
  trace and incident state produced by the service rather than inventing its
  own crash interpretation.

This keeps the roadmap disciplined: make the service more authoritative first,
then build richer dashboard and automation behavior on top of that authority.
