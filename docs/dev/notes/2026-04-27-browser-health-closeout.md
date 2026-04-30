# Browser Health Closeout

Date: 2026-04-27

## Context

This note closes the current browser-health slice after the recovery trace,
retry override, and blocked crash-loop coverage work. The immediate goal was to
make the shutdown remedy ladder explicit enough for operators to distinguish a
browser problem from a possible host problem.

## Findings

Browser shutdown now reports an explicit outcome for owned browser processes.
The daemon first attempts a polite browser shutdown through the browser control
path. If that fails, it falls back to force-killing the owned process tree.

The persisted browser health mapping is intentionally conservative:

- polite shutdown failure with successful force kill records `degraded`
- force-kill failure records `faulted`
- force-kill failure also records a last-error warning that the OS may be
  degraded
- clean polite shutdown records `not_started`

This matches the operating rule for service mode: if polite kill fails, the
browser is degraded; if force kill fails, the OS may be degraded.

Grouped service incidents now carry `severity`, `escalation`, and
`recommendedAction` fields so clients do not have to infer operator priority
from browser health strings. The shutdown ladder maps polite close failure to
`warning` plus `browser_degraded`, and force-kill failure to `critical` plus
`os_degraded_possible`.

## Validation

The live shutdown-health smoke now validates the polite-shutdown failure remedy:

```bash
pnpm test:service-shutdown-health-live
```

It launches an owned Chrome in an isolated `AGENT_BROWSER_HOME`, forces the
polite close path to fail through an internal smoke-only hook, lets the daemon
force-kill the owned browser process, and verifies the persisted service
browser record is `degraded` with shutdown-remedy details in `lastError`.
It also verifies the derived incident exposes `severity: "warning"`,
`escalation: "browser_degraded"`, and a browser-health recommended action.

The last full native validation passed when run serially:

```bash
cd cli && cargo test -- --test-threads=1
```

Evidence from the 2026-04-26 run: 897 passed, 0 failed, 55 ignored.

The plain parallel native test command is still not the best closeout signal
for this branch. It can trip shared-environment contamination in auth and
Chrome-profile tests, while the individual failing selectors pass and the full
serial suite passes. Until those tests are isolated, use the serial command as
the release-quality native regression check.

## Service Health Acceptance Gate

Before merging service-health, browser recovery, incident, or service-control
changes, run the aggregate live service-health gate. It runs the underlying
smokes serially so Chrome and service state do not contend across tests:

```bash
pnpm test:service-health-live
```

For targeted troubleshooting, the gate expands to:

```bash
pnpm test:service-api-mcp-parity
pnpm test:service-shutdown-health-live
pnpm test:service-recovery-http-live
pnpm test:service-recovery-mcp-live
pnpm test:service-recovery-override-http-live
pnpm test:service-recovery-override-mcp-live
pnpm test:service-incident-parity-live
```

This gate proves the minimum always-on service behavior expected for the
current roadmap slice:

- shutdown remedies classify polite close failure as browser degradation
- HTTP exposes recovery traces, retry override behavior, and retained incidents
- MCP exposes the same recovery traces, retry override behavior, and retained
  incidents for agent clients
- incident severity, escalation, and recommended action stay consistent across
  HTTP and MCP filters

The gate passed on 2026-04-27 with all then-current scripts run serially from
the repo checkout. The current pre-merge checklist is maintained in
`docs/dev/notes/2026-04-30-service-model-pre-merge-checklist.md`.

## Best Next Step

The best next step is to move from closeout classification into service-owned
supervision: add an explicit browser incident severity model so `degraded`
browser remedies and `faulted` possible-OS remedies can drive operator
acknowledgement, retries, and escalation without each client inventing its own
interpretation.
