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

## Validation

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

Add a focused live smoke that launches a real owned Chrome, forces the polite
shutdown path to fail, verifies `degraded`, then simulates a force-kill failure
boundary in a controlled helper test. That will make the remedy ladder visible
in live service-state artifacts, not only unit-level shutdown classification.
