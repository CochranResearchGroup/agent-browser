# Failed-upgrade browser preservation qualification

Product lane: PL-BUGFIX
Work item: #183
Branch: fix/failed-upgrade-shutdown-183
State: COMPLETED
Version: 1
Lane: P239

## Bounded outcome

Deliver an executable isolated acceptance check for the live-browser preservation
boundary already repaired by P233/P234. Under an active failed-upgrade admission
drain, graceful termination of the exact runtime host must finish within five
seconds while the same browser process, CDP endpoint and profile control survive.
After fixture-only drain removal, the same logical task must continue in that
browser. Explicit task close and a fresh ownership census must leave no residue.

This packet verifies one owner exit. It does not implement or qualify the wider
workstation-wide shutdown operation requested by #183.

## Ownership and authority

The operator requested the next ticket after P238 deployment. P237 already owns
the dashboard and handoff-lifetime work under #260/#261. This lane owns a new
acceptance script, its package command and its evidence documents only. It edits
no shared Rust lifecycle, browser manager, service model or dashboard source.

The clean enrollment checkout is reused from fresh origin/main at
`9bd54cfbc711c338dadaec7ad92f4fdd7dfc85b5`. The prior P238 branch and P232 sealed
candidate bytes remain retained. No production shutdown, provider effect,
private authentication, profile reset, privileged helper or CI dispatch belongs
to this packet. Fixture signals require positive namespace, executable and
process-instance identity. A stale PID alone never permits a signal.

## Acceptance

The public workflow MCP opens one synthetic managed persistent task. Admission
is then drained and a second opening must fail with `runtime_admission_draining`.
This proves the drain is active, rather than merely writing unused fixture data.
SIGTERM targets only the exact owned host. The fixture compares browser process
start time, PID, argv, CDP websocket and singleton-lock target before/after exit.
It also verifies endpoint metadata removal and unchanged drain bytes.

The `--break-preservation` negative control terminates only the fixture browser
after owner exit and must fail the precise browser-preservation assertion. Both
positive and negative runs require zero exact fixture-owned process residue.
Its failure is an oracle control, not a reproduced product regression.

## Result and remaining ticket scope

The positive installed-source check passes. Exact host exit takes 29ms in the
first qualified run; the same browser and task continue after restart. The
negative control fails exactly at browser PID preservation and cleans up.
The [evidence note](../notes/0239-2026-10-12-failed-upgrade-preservation.md) retains
fixture corrections, cleanup and executable provenance.

#183 remains open. No workstation-wide plan/apply command with independent
preserve/close/ambiguous dispositions has been qualified. Failed transaction
ledger reconciliation, multiple generations, service/timer/container/listener
shutdown, exact escalation, dead-lease reconciliation and partial retry remain
unproven. An admission-drain fixture is not a failed installer transaction.
The next delivery must define and exercise that terminal operation using current
native ownership contracts, without extending P237's shared lifecycle work or
introducing another authority ceremony.
