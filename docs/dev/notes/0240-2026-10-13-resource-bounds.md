# Resource budgets and durable terminal-history qualification

Product lane: PL-PLATFORM
Disposition: accepted-evidence
Owning plan or work item: P240 / #105
Related lanes: P237, native selection and handoff policy preserved
Baseline: 65368991beee90839083b7330f1de09b69b05451

The operator accepted preserving active work under memory pressure and keeping
older completed history durable while bounding its active-memory projection.
The implemented defaults are an adaptive reserve of 5% RAM with a 1 GiB minimum,
a five-second new-process wait, a supervisor soft threshold of 25% RAM and 1000
recent unreferenced completed records per kind. Explicit settings are captured
into production and isolated-development units. There is no default hard cap.

Completed session/tab records move into an append-only table in the existing
SQLite runtime database, atomically with active-state publication. The archive
has no automatic expiry. Bounded read-only CLI pages use an immutable sequence
cursor. Exact handoff references pin their terminal evidence beyond the recent
record budget; this preserves P237's legacy target-restoration prerequisite.
Native host memory adopts the exact published projection before another CAS.

## Qualification

Reserve and terminal-projection tests failed before their implementations and
passed afterward. The soft-unit test likewise failed before adding its budget.
The wait test first failed to compile because its injectable boundary did not
exist; its green controlled-clock run proves recovery and the deadline without
sleeping. SQLite tests cover archival failure rollback, stale publication,
restart, session/tab records and handoff-reference preservation.

At the frozen source, browser-session tests pass 99 with six ignored; admission
tests pass four; supervisor tests pass 15; output tests pass 39; MCP tests pass
133; launch-custody tests pass six. Format and strict workspace Clippy pass.
Development-unit fixtures prove default and overridden budgets and rejection of
an invalid wait. Generated client, types, exports, helper, flow and example
checks pass, as do the docs build, link and planning checks and validation
selection suite. Two pre-existing documentation assertions incorrectly read the
compact MCP skill for SDK/development instructions; they now read its linked
advanced reference. No large default catalog was restored.

The optimized CI executable from source `8780673b3` has SHA256
`47995a367a0ab28202d646a9056798eac2aa50aa47efb68a9359fa649a7e4305`.
Later changes before qualification affect only the test fixture's names.
It was published solely into isolated generation `0.28.0-47995a367a0a` in p240.
The active unit showed an explicitly configured 8 GiB soft threshold and
`MemoryMax=infinity`; its reserve, wait and history overrides were captured.
Production and default development identities remained unchanged.

The tracked `scripts/smoke-resource-budget-history.js`, with absolute candidate
and browser paths in `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD` and
`AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE`, passes against those installed bytes.
Five completed tasks with a three-record projection produce readable archive
pages. Two live peers share a browser; edits and continuation survive archival,
and peer close preserves the remaining task. With a deliberately impossible
reserve, a new process defers while an externally owned existing browser stays
usable and survives scoped task close. Both fixtures report zero owned process
residue. The first private probe used an unsupported runCli result field; that
fixture assertion was corrected and excluded from product evidence.

Three standard development launch/read/close cycles pass. The composite doctor
passes executable, browser, supervisor and namespace checks, with only local
web ingress unpublished. The standalone install doctor was run after intentional
unit retirement and reports missing live-dashboard steady state; this is not a
full installation-readiness claim. Its exact inspection host was positively
identified and retired. All three p240 units are stopped and disabled; candidate
bytes and receipts remain retained. No production process or provider was stopped.

## Standards

The review accepted a blocking finding that a combined configuration/capacity
failure could trigger pressure cleanup. The shared memory-only predicate now
permits cleanup/wait only for the single recoverable memory reason. Focused
admission tests and strict Clippy pass after that correction. Numeric settings
remain small platform adapters with parity fixtures; no new authority layer or
speculative framework was added. No accepted Standards blocker remains.

## Spec

Archive and working history are distinct; failed archival cannot publish a
truncated document. Handoff-referenced evidence is explicitly protected. The
public borrowed-browser pressure check proves that a reserve cannot veto reuse.
The scoped source/behavior outcome passes; no accepted blocker remains for it.

#105 remains open for longer representative installed workload/PSS acceptance,
cleanup convergence under revision churn and qualification of other lifecycle
record categories. These checks do not prove a leak was fixed, production was
upgraded, or native viewer/private-account workloads were exercised. #183 stays
deferred until after the next reboot. CI remains disabled.
