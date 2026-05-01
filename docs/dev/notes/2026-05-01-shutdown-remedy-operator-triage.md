# Shutdown Remedy Operator Triage

Date: 2026-05-01

## Context

This note updates the browser-health closeout after adding live coverage for
the force-kill failure branch. The service roadmap depends on agent-browser
being able to distinguish a recoverable browser problem from a possible host
problem while keeping that distinction visible through service state and
incidents.

## Remedy Ladder

Owned browser shutdown has two remedies:

- First, attempt polite browser shutdown through the browser control path.
- If polite shutdown fails, force kill the owned browser process tree.

The persisted health mapping is:

- Clean polite shutdown records `not_started`.
- Polite shutdown failure with successful force kill records `degraded`.
- Force-kill failure records `faulted`.

The operator rule is intentionally simple: if polite kill fails, the browser is
degraded. If force kill fails, the OS may be degraded.

## Incident Triage

Use `agent-browser service incidents` or the HTTP and MCP incident surfaces to
separate browser-level remediation from host-level escalation.

For `browser_degraded` incidents:

- Expected severity is `warning`.
- Expected browser health is `degraded`.
- Treat the browser process as cleaned up but unreliable.
- Inspect recent browser health events and recovery traces.
- Retry or relaunch the affected browser before escalating to host inspection.

For `os_degraded_possible` incidents:

- Expected severity is `critical`.
- Expected browser health is `faulted`.
- Treat the browser shutdown as unresolved even if the original CLI command
  returned.
- Inspect the host OS process table for leftover Chrome or helper processes.
- Check whether process permissions, stuck process groups, container runtime
  behavior, or OS resource exhaustion prevented force kill.
- Prefer host remediation before reusing the same browser profile for further
  automation.

## Validation

The degraded branch is validated by:

```bash
pnpm test:service-shutdown-health-live
```

That smoke forces polite close failure, verifies force kill succeeds, and
asserts the persisted browser record is `degraded` with incident escalation
`browser_degraded`.

The faulted branch is validated by:

```bash
pnpm test:service-shutdown-faulted-live
```

That smoke forces polite close failure plus force-kill failure reporting,
verifies the persisted browser record is `faulted`, and asserts the incident
escalates as `os_degraded_possible`.

Focused Rust health mapping coverage is:

```bash
cargo test --manifest-path cli/Cargo.toml close_health -- --test-threads=1
```

Use the live smokes when changing shutdown behavior or incident projection.
Use the Rust filter when changing only classification helpers.

## Best Next Step

The best next step is to add a compact operator command or dashboard affordance
that groups `browser_degraded` and `os_degraded_possible` incidents with their
recommended next action. That would let service clients and human operators
consume the remedy ladder without parsing raw health events.
