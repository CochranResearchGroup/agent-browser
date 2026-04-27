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

## Best Next Step

The best next step is to move from closeout classification into service-owned
supervision: add an explicit browser incident severity model so `degraded`
browser remedies and `faulted` possible-OS remedies can drive operator
acknowledgement, retries, and escalation without each client inventing its own
interpretation.
