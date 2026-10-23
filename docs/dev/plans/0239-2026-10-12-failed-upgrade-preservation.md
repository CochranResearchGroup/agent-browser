# Failed-upgrade browser preservation qualification

Product lane: PL-BUGFIX
Work item: #183
Branch: fix/shutdown-retry-fixture-183
State: COMPLETED
Version: 2
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
ledger reconciliation, multiple generations, owned service/timer/listener
shutdown, exact escalation, dead-lease reconciliation and partial retry remain
unproven. An admission-drain fixture is not a failed installer transaction.
The next delivery must define and exercise that terminal operation using current
native ownership contracts, without extending P237's shared lifecycle work or
introducing another authority ceremony.

The issue's current review supersedes its historical helper/container checklist.
Native Remote View and foreign lanes are preserved. Legacy privileged helper,
Guacamole-container and global route shutdown are not acceptance requirements.


## Bounded continuation, 2026-10-23

Root takes183 after next-ticket instruction in the retained clean dashboard primary from ade8ecd. This packet repairs current public fixture attribution and extends the existing isolated preservation oracle to two sequential host exit/continuation cycles under a proven active drain. Each cycle pins its newly created host and unchanged browser process/CDP/profile, enforces the existing five-second exit bound, verifies drain bytes and endpoint retirement, then explicitly removes only its own drain for same-task continuation. This is repeated graceful-host preservation, not a workstation-wide shutdown command, multi-binary generation retirement or transaction reconciliation. Keep183 open for those wider gates.

The existing negative control must still fail exactly at browser preservation and finish exact owned cleanup. Use the sealed fast_iteration f16e5df/33fdd29 candidate and reviewed Chrome in disposable namespaces only. No production, provider, account, shared cleanup, legacy helper or privileged effects. Root is sole writer of this script and P239 metadata. A separately admitted clean auxiliary /tmp/agent-browser-focus-68 belongs only to the parallel274 Selection Focus repair; no shared source writer or runtime effect overlaps. Current stock capability81 and resource105 audits find no new implementation defect; preserve their explicit wider gates.


Continuation result: current public fixture first fails before browser launch because required attribution is absent. After the two-call attribution repair and repeated-cycle extension, sealed33fdd29 candidate passes both exact host exits in30ms and29ms, independently proves two distinct host PID/start instances, same original browser/CDP/profile lock and same-task continuation. The deliberate negative control fails exactly at the browser-preservation assertion and completes zero-owned-residue cleanup. Its injected failure occurs in the first cycle; it is not an independent second-cycle-only negative proof. Source review finds no new blocker. Syntax and patch hygiene pass.

Fresh independent proc/stat readback proves both recorded host generations absent and zero readable private fixture markers. Denied foreign proc reads bound census coverage in the user-scoped receipt. Do not equate unavailable executable links with process absence or infer global visibility. Wider183 shutdown dispositions, failed-ledger reconciliation, multi-binary generations and terminal retry idempotence remain open; this packet qualifies repeated graceful-host preservation only. Original red, green and negative logs remain /tmp/p239-current-{red,green,negative}.log. Production and Native services remain unchanged.
