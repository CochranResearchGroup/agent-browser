# Plan 0166: Repository Custody and Timed-Out Connection Release

Date: 2026-09-11

State: PLANNED

Consolidation: required

Lane: P166 maintenance gate before P162

Branch: `maintenance/plan-0166-connection-release`

Target: `main`

Integration: short-lived branch through the protected `main` workflow

Parent: [Plan 0160](0160-2026-09-06-production-profile-identity-and-operational-readiness.md), A1 and AX

Blocks: [Plan 0162](0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md) implementation start

Current execution status: [RUNBOOK.md](../../../RUNBOOK.md)

## Objective and authority

Restore truthful Git custody before beginning another implementation lane, then
investigate and repair one Agent Browser control-plane failure reported by the
Odollo agent. A multi-command call exceeded its client deadline and the CLI
process exited, but the prior task-owned connection remained recorded as active.
The next process was consequently denied instead of safely rejoining after the
old transport ended.

This is a future-execution maintenance plan. It authorizes repository
reconciliation, provider-free fixtures, source changes, documentation changes,
and isolated development-runtime validation. It does not authorize an Odollo
page retry, a new browser or profile, production installation, provider
mutation, tenant effects, broad process cleanup, release publication, or
deletion of an unmerged branch. The completed RuFresh no-result remains valid
and must not be reopened or repeated as part of this work.

## Current evidence and working hypothesis

The 2026-09-11 Git audit found nine clean worktrees. During planning, Plan 0165
landed on `main` and `origin/main` at `ad612596`; its BILL repair remains a
separate active operation that this plan must not disturb. Several checked-out
branches or detached worktrees are already represented in `main`, while
`feature/p0240-sealed-auth-runtime-compatible` retains unique work and the P157
branch remains the source ref for the open Plan 0158 lane. Those two refs require
preservation unless a fresh bounded ancestry and lane audit proves a different
disposition.

The report does not yet distinguish a client transport deadline from the
control plane's coordinated worker deadline. Current source inspection suggests,
but does not yet prove, this ordering:

1. one CLI connection submits the complete dependent batch;
2. the daemon records that connection as active and awaits the queued response;
3. the client deadline expires and its process exits while the worker request is
   still pending;
4. the daemon does not observe transport closure or drop its disconnect guard
   until the awaited worker response completes; and
5. a replacement process reaches profile child-access policy while the old
   connection is still persisted as active and receives
   `owner_connection_still_active`.

Likely source owners are `cli/src/main.rs` dependent-batch submission,
`cli/src/native/daemon.rs` connection lifetime,
`cli/src/native/control_plane.rs` request coordination,
`cli/src/native/service_profile_access_policy.rs` child admission, and
`cli/src/native/service_model.rs` persisted disconnection. CodeGraph was not
initialized in the current `main` worktree during planning, so these are
source-read hypotheses rather than graph-backed impact claims. The executor
must initialize or synchronize the expected derived index before structural
impact analysis, then prove the actual failure path with a focused fixture.

## Consolidated batch

This batch contains two ordered maintenance outcomes:

1. reconcile worktree, branch, remote-tracking, plan-state, and active-lane
   custody so stale or integrated refs do not obscure the repair baseline; and
2. make connection custody converge when the exact client transport ends after
   a deadline, without allowing another process to steal genuinely live work or
   confusing transport death with permission to repeat an operation.

The repair must keep request execution, connection liveness, profile-child
custody, and operation-effect status distinct. A disconnected client may leave
an independently executing or terminal job that remains observable by request
and job ID. Releasing the connection must not silently cancel, retry, duplicate,
or declare that job effect-free.

## Delivery sequence and budget

### M0: Git custody reconciliation

1. Refresh local, origin, and upstream refs read-only, then inventory every
   worktree and branch with cleanliness, checkout owner, ahead/behind state,
   target ancestry, plan state, active-lane registration, and unique commits.
2. Preserve the active Plan 0165 operation, the open P157 source ref, and the
   unique P0240 runtime-compatible work. Reconcile the latter in its own later
   lane rather than folding it into this bug repair.
3. Remove only clean detached or branch worktrees whose commits are proven
   reachable from `main` or another named retained ref. Delete only proven
   integrated local branches. Prune stale remote-tracking refs only after a
   current remote readback proves their upstream refs absent.
4. Restore one unambiguous canonical `main` worktree and record the retained
   worktrees, paused refs, and any unresolved custody blocker in the active-lane
   catalog and runbook.

No force deletion, reset, rebase of a published branch, or discard of unique
commits is authorized. Any dirty worktree, divergence, missing remote evidence,
or plan/catalog contradiction stops M0 for that target without blocking safe
maintenance of unrelated proven targets.

### M1: Exact incident evidence

Capture the provided Odollo report as a defect locator without interacting with
the New Jersey page. If the exact task-owned connection is still present, use
read-only Service status, request, job, event, connection, profile, process-ID,
and process-start-token evidence to determine whether it is live, terminal,
disconnected, or stale. Wait for its bounded natural release when appropriate.
Do not create another browser, profile, connection, or page action to collect
evidence.

### M2: Provider-free reproducer

Build one deterministic test harness for both deadline boundaries: a client
transport deadline shorter than a multi-command worker delay, and a coordinated
worker deadline shorter than one nested command. Prove the order of timeout
classification, client exit, daemon EOF or write failure, worker completion,
disconnect persistence, and successor admission. Cover each competing
explanation before choosing a repair:

- the original request is still legitimately executing;
- the client is gone but the daemon is blocked awaiting the worker;
- the worker is terminal but disconnection persistence failed or lagged; or
- process-liveness reconciliation can prove the recorded connection host ended.

The red fixture must reproduce the denial using one profile and one logical
task. It must not launch Chrome or depend on Odollo, Guacamole, or provider
state.

### M3: Minimal lifecycle repair

Implement the smallest change that lets the daemon observe exact transport
closure independently of the queued response while retaining one authoritative
execution result. Persist disconnection idempotently and make successor
admission depend on positive evidence that the prior transport or host ended.
Do not weaken denial for a genuinely live connection, a different subject, an
unreadable legacy identity, or ambiguous process evidence.

If the architecture cannot safely separate transport observation from request
execution in one bounded change, stop with a proven design seam and amend this
plan before introducing a second worker, queue, or ownership model.

### M4: Observability and recourse

Return enough stable evidence on deadline and admission refusal to identify the
connection instance, request, job, current execution state, and supported
read-only recourse. Align CLI text and JSON, Service events or incidents, HTTP,
MCP, generated client, help, README, skill, docs site, and inline comments only
for contracts actually changed. Do not expose raw provider routes, tenant data,
or private page content.

### M5: Qualification and integration

Run focused red/green tests during implementation. After source freeze, run
`pnpm validation:select -- --base <baseline>`, Rust format, strict workspace
Clippy, the relevant provider-free Rust compartments, and any selected contract
or documentation checks. Build and publish at most one development candidate if
installed behavior must be verified, then run the development browser-launch
smoke and exact no-residue checks. Production installation and a live Odollo
retry require separate operator authority.

The whole batch is bounded to 180 minutes of active work: 35 minutes for Git
custody, 25 for incident evidence, 45 for the reproducer, 45 for repair and
contracts, and 30 for frozen-source qualification and closeout. It permits one
review/rework cycle and one development candidate build. Exceeding a phase
budget requires a checkpoint with the exact blocker rather than an improvised
scope expansion.

## Worker assignments

The primary executor owns Git custody, failure reproduction, lifecycle design,
source changes, validation, and integration. No parallel worker is assigned by
default because repository custody and connection ownership share the same
critical path. After a frozen patch exists, one optional fresh-context reviewer
may inspect only the no-steal, no-retry, and effect-status boundaries. The
primary remains responsible for reconciling review findings and rerunning the
affected gates.

## Evidence and exit

| Requirement | Required evidence | Exit condition |
| --- | --- | --- |
| Git custody | Fresh worktree, branch, remote, ancestry, and active-lane report | Canonical `main` is unambiguous; only clean integrated targets are removed; every retained unique ref has an owner and disposition |
| Reproducer | Provider-free deadline matrix with timestamped lifecycle transitions | The matching client or worker deadline reliably leaves the old connection active long enough to reproduce the successor denial |
| Prompt disconnect | Focused green daemon and persistence tests | Exact transport or host termination marks only that connection disconnected without waiting for unrelated worker completion |
| Live-owner protection | Concurrent live-connection, different-subject, legacy-identity, and ambiguous-liveness tests | A genuinely live or unprovable prior owner still denies takeover with typed evidence |
| Request truth | Timeout, late-success, late-failure, and cancellation fixtures | Connection release never implies retry, cancellation, success, failure, or `no_effect`; the original request and job remain independently queryable |
| Dependent batch | Multi-command partial-progress fixture | Completed subcommands remain attributable, unfinished work has one terminal state, and no command is duplicated |
| Stable recourse | CLI and changed contract tests | Deadline and denial outputs expose safe identifiers and read-only lookup or wait guidance without suggesting another browser/profile |
| Frozen source | Validation selector, format, strict Clippy, selected Rust and contract checks | Every touched surface since the batch baseline passes once on the frozen tree |
| Isolated installed behavior | Development manifest, doctor, three browser-launch smokes, and process census if a candidate is built | Development identity is healthy, disposable fixture residue is absent, and production state is untouched |
| Scope preservation | Runbook closeout and incident locator | No Odollo retry or tenant effect occurred, and the RuFresh no-result remains accepted evidence |

This plan is complete only when the source repair is integrated into `main`,
the repository has truthful retained custody, and all applicable exit evidence
is recorded. A completed plan packet is not production installation authority
and does not close Plan 0160 or Plan 0162.
