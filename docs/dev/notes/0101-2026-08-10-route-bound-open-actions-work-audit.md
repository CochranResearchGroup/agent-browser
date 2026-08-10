# Plan 0101 Route-Bound Open And Actions Work Audit

## Audit identity

- Role: independent Candidate 4 implementation work auditor
- Runtime handle: `/root/audit_work_actions_deepening`
- Cycle: `1`, fresh-context `drift_discovery`
- Frozen implementation range: `4f052ba3..6449557a`
- Audited head: `6449557a4a470fd1da722a8ff5b19c3f09021bb4`
- Audit date: 2026-08-10
- Source write authority: none
- Audit write authority: this note only
- External effects: none
- Browser, ignored end-to-end, install, doctor, live-runtime, and tenant effects: not run

## Disposition

**Cycle 1 result: NOT ACCEPTED.**

The deterministic inventory, predecessor reconciliation, final `actions.rs`
size and definition budgets, explicit six-definition allowlist, recorded wrapper
count, and production reverse-import check pass. Those checks do not establish
the frozen deep-architecture or route-bound transaction contracts. Seven
blocking implementation findings remain:

| Finding | Disposition | Summary |
| --- | --- | --- |
| `P0101-W1-01` | `blocking` | Synchronous repository work and the post-timeout await are outside the total deadline. |
| `P0101-W1-02` | `blocking` | Lease finalization and durable-handoff persistence are separate mutations, and the JSON store also commits two files separately. |
| `P0101-W1-03` | `blocking` | The route seam retains raw JSON, string-derived outcomes, and the transitional adapter; the required outcome, fallback, and authorization matrix is absent. |
| `P0101-W1-04` | `blocking` | The extraction produced a broad common prelude, many shallow wrapper-shaped modules, a test-only re-export facade, and an 11,737-line test monolith. |
| `P0101-W1-05` | `blocking` | The 81 packet commit and rollback boundaries were not executed, while the receipt marks grouped packets and all A through G slices complete. |
| `P0101-W1-06` | `blocking` | The architecture checker is materially false-green for target depth, interface-local tests, permanent runtime ownership, and transaction semantics. |
| `P0101-W1-07` | `blocking` | Prescribed WSL validation still emits or directly launches uncapped Cargo commands. |

Per the two-cycle contract, no Cycle 2 was performed. One consolidated
remediation is prescribed below. Cycle 2 must remain closed-world over these
seven accepted finding IDs and critical regressions introduced by their repair.

## Authorities and evidence reviewed

The audit read the complete Plan 0101, Plan 0102 overlay, both cycles of the
independent plan audit, the Preflight P0 and final execution sections of the
execution receipt, `CONTEXT.md`, the service roadmap, applicable repository
policies, the current architecture inventory and fixtures, the complete commit
range, and current source.

Graphiti discovery in `agent_browser_main` was advisory and returned only older
route-bound leads, so no current acceptance claim relies on it. CodeGraph was
healthy at audit time with 502 files, 16,629 nodes, and 54,162 edges. It was used
for indexed call and ownership structure. Direct reads were used for exact
current source and large generated or test files.

The worktree started with only this pre-existing untracked build cache:

```text
?? scripts/architecture/actions-inventory/target/
```

It contained 1,507 files and occupied 431 MiB during this audit. The execution
receipt recorded 225 MiB. The cache was excluded from source identity, was not
modified or deleted by the auditor, and remains untracked. The size drift is
operational evidence only and is not a source finding.

## Verified acceptance evidence

### Predecessor and inventory identity

- P0098 commit `7f89ea49`, P0099 commit `0528e5db`, and P0100 commit
  `71e3d691` are all ancestors of campaign base `4f052ba3`.
- The frozen pre-P0100 baseline contains exactly 615 production definitions.
- The current inventory contains exactly 615 stable records: 609 have
  `movementStatus=moved`, and the exact six reviewed dispatcher definitions
  remain at baseline with `finalDisposition=retain`.
- Four predecessor-moved definitions are included in the 609 moved records.
- The inventory has six allowlist entries, 57 target-depth entries, no duplicate
  stable IDs, and no recorded `wrapperOwner`.
- Current inventory SHA-256 is
  `d7c17b86b456f6a2bfbae6a37b32916fee2b647b2a8b0efc87bb360514f0825f`.
- The P0-time inventory digest recorded in the receipt is a historical P0
  checkpoint, not a claim that the final inventory kept that digest.

### Final dispatcher shell

`cli/src/native/actions.rs` is 861 lines, contains six production definitions,
contains no test definitions, and defines no struct, enum, or union. The six
definitions exactly match the reviewed allowlist:

1. `action_skips_browser_launch`
2. `active_target_binding`
3. `handle_dependent_batch`
4. `execute_command`
5. `success_response`
6. `error_response`

No extracted production owner imports `actions`. The only current importers are
the control plane, HTTP test code, parity tests, end-to-end tests, and the
central action-runtime test file. This satisfies the literal line, definition,
allowlist, recorded-wrapper, and production reverse-import gates.

### Nominal A through G coverage

The source range contains files assigned to every A through G family, and the
inventory accounts for all 615 baseline definitions. The route-open children,
daemon runtime split, workflow owners, browser owners, service owners, and final
dispatcher reduction all exist. This is path and inventory coverage only.
Findings `P0101-W1-01` through `P0101-W1-06` show why it is not semantic
acceptance of those slices.

### Context and public-contract posture

`CONTEXT.md` accurately defines the route-bound terms introduced by the plan,
including total deadline, compensation reserve, coordinator-owned completion,
rollback quarantine, concrete owner module, and transitional facade. No public
CLI option or deadline meaning was intentionally changed. The service roadmap,
however, contains no Plan 0101 closeout or ownership reconciliation. That
documentation drift is included in `P0101-W1-05` rather than duplicated as a
separate finding.

## Reproduced validation

Every compiling Cargo command was serialized through
`scripts/ci/cargo-safe.sh`. No raw Cargo command was run. Available memory was
about 37 GiB before the compiling checks and 36 to 38 GiB afterward. Swap use
remained about 1.1 GiB of 32 GiB, and no Cargo or Rust compiler process remained
afterward.

| Gate | Result | Exact count or qualification |
| --- | --- | --- |
| `pnpm test:actions-architecture` | pass | 6 definitions, 0 tests, 861 lines, 0 recorded wrappers, `final=true` |
| Architecture checker self-test | pass | classified fixture accepted; unclassified fixture rejected |
| Guarded `remote_view::open::tests` | pass | 4 passed, 0 failed, 0 ignored, 1,852 filtered |
| Guarded `coordinated_` tests | pass | 2 passed, 0 failed, 1,854 filtered |
| `pnpm test:route-handoff-audit` | pass | static no-launch fixture |
| `pnpm test:service-client` | pass | generated client, type, request, observability, fixed-input, managed-profile, and dry-run example checks |
| `git diff --check 4f052ba3..6449557a` | pass | no patch whitespace failure |
| `pnpm validation:select -- --base 4f052ba3` | completed | selection was polluted by the 1,507 untracked cache files and emitted raw Cargo commands |
| `pnpm test:route-confusion-gates` | not run | script directly invokes four compiling Cargo tests on WSL; see `P0101-W1-07` |
| Ignored end-to-end, browser, install, doctor, and live gates | prohibited | outside this role's effect boundary |

The four route-open tests exercise the reserve formula, an already-cancelled
token before nine isolated runtime calls, one supervisor timeout and
compensation path, and a scripted event list. They do not invoke
`RouteBoundOpenCoordinator::open`. The two coordination tests use trivial
futures. These counts explain why their green result does not contradict the
findings below.

## Findings

### `P0101-W1-01` | Total deadline and queue release are not bounded

- **Criterion:** Plan 0102 requires every forward and compensation operation,
  including the last repository write, to finish within the existing total
  `jobTimeoutMs`; at the total deadline no task or rollback work remains and the
  serialized queue is released. It explicitly requires fake-clock tests for
  response and queue release by the total deadline.
- **Evidence:** `RouteBoundOpenSupervisor::forward` and `compensate` bound only
  async runtime futures in `cli/src/native/remote_view/open/deadline.rs:127` and
  `:170`. The coordinator performs synchronous repository load and mutation
  before or after only a point-in-time `ensure_forward` check at
  `cli/src/native/remote_view/open/coordinator.rs:257-265`, `:320-331`, and
  `:633-659`. Compensation performs synchronous begin and completion mutations
  outside `compensate` at
  `cli/src/native/remote_view/open/compensation.rs:62-96`.
  `LockedServiceStateRepository` takes process and file locks and performs
  synchronous load and save at `cli/src/native/service_store.rs:172-205`.
  After the public timeout fires, `await_coordinated_execution` cancels and then
  awaits the same future without an outer bound at
  `cli/src/native/control_plane.rs:527-565`.
- **Consequence:** A file lock, filesystem, serialization, or rename stall can
  exceed `jobTimeoutMs` and withhold the serialized queue indefinitely. The
  coordinator then cannot guarantee a terminal `rolled_back` or
  `rollback_incomplete` result at the deadline. This is the exact failure class
  made material by the observed WSL filesystem stalls.
- **Reproducer:** Add a repository test double whose load, reserve, finalize,
  begin-recovery, or complete-recovery call blocks past T. Invoke
  `await_coordinated_execution` around the real coordinator with a fake clock.
  The current future remains awaited after T because only runtime trait futures
  are supervised. Existing command:
  `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml coordinated_ -- --test-threads=1`
  passes because its two futures do not exercise repository blocking.
- **Confidence:** high
- **Disposition:** `blocking`; accepted remediation must encompass all
  repository phases and bound the control-plane completion wait.

### `P0101-W1-02` | Finalization and durable handoff are not one repository transaction

- **Criterion:** Plan 0101 marks `finalized_and_persisted` as one repository
  transaction and requires a hard stop if lease finalization and durable
  handoff cannot agree atomically.
- **Evidence:** `complete_route_bound_handoff_open` first calls
  `complete_route_bound_handoff_plan_acquisition`, then separately calls
  `persist_remote_view_handoff` at
  `cli/src/native/remote_view_handoff.rs:1523-1553`. The first call can commit
  lease and route finalization before the second call fails. Even one repository
  `mutate` is not an atomic two-file commit: `JsonServiceStateStore::save`
  writes and renames `remote-view-handoffs.json` first, then writes and renames
  `state.json` at `cli/src/native/service_store.rs:118-161` and `:279-317`.
- **Consequence:** A write or rename failure can leave finalized route ownership
  without the durable handoff, or the handoff registry ahead of the main state.
  The returned failure is then neither an atomic success nor an exact rollback.
- **Reproducer:** Use a fault-injecting `ServiceStateStore` or filesystem fixture
  that succeeds finalization and fails handoff persistence, then inspect the
  stored lease, route, and handoff. A second fixture should fail the main state
  rename after the handoff-registry rename. No current route-open interface test
  covers either boundary.
- **Confidence:** high
- **Disposition:** `blocking`; do not claim Slice A or durable-handoff
  acceptance until one atomic commit boundary and fault-injection tests exist.

### `P0101-W1-03` | The route-open seam is not the frozen typed interface

- **Criterion:** The frozen interface has one typed normalized invocation, an
  exact 13-operation runtime seam with typed request and result types, typed
  outcomes including `Planned`, no coordinator error-string parsing, a
  permanent Slice C daemon/browser adapter, nine closed fallback predicates,
  and ingress parity proving authorization before invocation construction.
- **Evidence:** The trait has exactly 13 methods, but most results are
  `RouteBoundOpenFuture<Value>`, and several request types contain a raw
  `command: Value` in `cli/src/native/remote_view/open/runtime.rs:23-100`.
  `DirectOpen` itself carries `request: Value`, while outcome payloads remain
  raw `Value` in `cli/src/native/remote_view/open/coordinator.rs:34-84`.
  `rolled_back_outcome` parses `"; cleanup="`, searches the cleanup string for
  `rollback_incomplete`, and derives a blocker code from message text at
  `:161-188`. The type named `ActionsRouteBoundOpenRuntime`, documented in
  source as transitional, still owns `&mut DaemonState` and implements the full
  trait at `cli/src/native/remote_view/open/runtime.rs:102-310`; both route
  handlers still instantiate it at
  `cli/src/native/remote_view/open/coordinator.rs:190-236`.

  Durable resolution converts JSON `status` strings back into outcomes and
  defaults a derived dry run to `Opened` or `Reopened`, not `Planned`, at
  `coordinator.rs:131-156`. Fallback uses the pre-attempt `ServiceState` rather
  than a typed immutable snapshot and has no freshness identity. It checks a
  useful subset of the ledger, but does not prove unchanged retained resources
  or no ownership mutation. The older string-based fallback helper also
  survives at `coordinator.rs:801-812`.

  Only the common action handlers construct `RouteBoundOpenInvocation`; the
  dashboard, HTTP, MCP, CLI, and daemon ingress adapters do not construct typed
  parity facts independently. No test references the typed fallback function,
  `RouteBoundOpenCoordinator::open`, or any of the six outcome variants. The
  four local route-open tests call the supervisor and scripted runtime directly.
- **Consequence:** The coordinator remains coupled to transport-shaped JSON and
  compatibility strings, typed fallback can drift from the older helper, a
  durable dry run can be misclassified, and ingress authorization parity is not
  established. Slice C's adapter deletion checkpoint is false.
- **Reproducer:** Run
  `rg -n 'RouteBoundOpenCoordinator::open|typed_remote_view_handoff_provider_fallback|RouteBoundOpenOutcome::' cli/src/native/remote_view/open/tests.rs`.
  It returns no test use. Inspect the cited runtime and coordinator ranges, then
  add table-driven interface fixtures for all invocations, outcomes, typed
  blockers, nine fallback predicates, and authorized and unauthorized ingress
  parity. The current guarded four-test filter remains green without them.
- **Confidence:** high
- **Disposition:** `blocking`; replace the raw and transitional seam and prove
  the full closed outcome, fallback, and authorization matrix.

### `P0101-W1-04` | Mechanical scattering replaced the monolith with facades and shallow modules

- **Criterion:** Every new target must hide a multi-step invariant used by at
  least two operations or production callers. Otherwise the packet must deepen
  the nearest cohesive owner. Domain tests must live beside and cross the
  extracted interface. A small dispatcher delegating to shallow pass-through
  modules, a renamed monolith, or one shallow module per action fails.
- **Evidence:** `cli/src/native/action_runtime/common.rs` is a 164-line wildcard
  prelude re-exporting browser, CDP, Service State, repository, process,
  filesystem, remote-view, network, tracing, and utility authority. Extracted
  files repeatedly import `common::*` plus the same broad runtime list, wrap
  handlers inside `action_commands` or `service_commands`, then re-export the
  nested module. Concrete examples are `page_pdf.rs` at 56 lines and one
  handler, `browser_evaluation.rs` at 52 lines and one handler,
  `stream_screencast.rs` at 80 lines and two handlers, and
  `service_configuration_inventory.rs` at 68 lines and three simple collection
  reads.

  `cli/src/native/action_runtime.rs:13-162` recreates a test-only umbrella by
  re-exporting nearly every extracted owner. All 262 moved action-runtime tests
  remain in `cli/src/native/action_runtime/tests.rs`, an 11,737-line file that
  imports `super::*`, `common::*`, `actions::*`, and many private owner modules.
  The four new route-open tests are beside the route module, but they do not
  cross its coordinator interface. This is not the required test migration.
- **Consequence:** Dependencies remain implicit and cross-domain, deletion of
  many targets merely moves one handler, private implementation details remain
  the test surface, and future changes still require monolithic context. The
  line budget improved without the intended architectural locality.
- **Reproducer:** Run
  `wc -l cli/src/native/action_runtime.rs cli/src/native/action_runtime/common.rs cli/src/native/action_runtime/tests.rs cli/src/native/page_pdf.rs cli/src/native/browser_evaluation.rs` and inspect their imports. Delete or inline one cited shallow target in a scratch branch: no multi-caller invariant is lost. The architecture checker still passes.
- **Confidence:** high
- **Disposition:** `blocking`; recombine shallow files into cohesive owners,
  remove the god prelude and test facade, and move tests to the interfaces they
  specify.

### `P0101-W1-05` | Atomic packet history and completion receipt are not truthful

- **Criterion:** Plan 0102 preserves 81 atomic packet and commit checkpoints.
  Each D, E, and F packet is an independently reviewable and revertible boundary
  with its own test movement, validation, rollback, receipt, and commit. Slice G
  and final closeout remain open while a deep-module, typed-domain, wrapper, or
  documentation gap exists.
- **Evidence:** The implementation range has 15 commits total for P0, all A
  through G work, harness changes, and documentation. Commit `88fba9ff` changed
  33 files with 36,637 insertions and 38,204 deletions. Commit `4854b5af`
  changed 37 files with 7,628 insertions and 6,509 deletions and represents all
  `P0101-E01-01` through `P0101-E21` work in the receipt. Commit `6fbceca5`
  changed 20 files with 3,154 insertions and 2,733 deletions and represents all
  `P0101-F01` through `P0101-F26` work. The receipt similarly assigns all five D
  packets to one commit. One revert cannot isolate a planned packet.

  The execution receipt at lines 146 through 155 marks every A through G slice
  and every P0102 repair complete, including claims of total-deadline
  compensation, complete typed outcomes, cohesive deep targets, and deleted
  transitional facades. Findings `P0101-W1-01` through `P0101-W1-04` directly
  contradict those claims. Its statement that there is no source blocker at
  lines 400 through 407 is therefore false. `CONTEXT.md` was updated, but the
  durable service roadmap has no matching Plan 0101 ownership or remaining-work
  closeout, contrary to completion criterion 12.
- **Consequence:** The planned rollback unit does not exist, packet-level review
  evidence cannot be reconstructed from the receipt, and future operators can
  incorrectly treat unsafe transaction and shallow architecture work as
  complete.
- **Reproducer:** Run
  `git log --format='%h %s' --reverse 4f052ba3..6449557a` and
  `git show --shortstat 88fba9ff 4854b5af 6fbceca5`, then compare the results
  with the packet ledger in the execution receipt and the 81-checkpoint
  requirement in Plan 0102 lines 152 through 178.
- **Confidence:** high
- **Disposition:** `blocking`; because the branch is not accepted, restore
  bounded review and rollback history before integration or explicitly obtain
  superseding authority for a different rollback contract. Correct the receipt
  and roadmap to actual state.

### `P0101-W1-06` | The architecture checker is false-green for the plan's primary risks

- **Criterion:** The checker must fail on one-handler wrappers, dispatcher
  coordination of private steps, cohesive-invariant scattering, domain test
  displacement, expired transitional adapters, typed-domain leaks, and final
  architecture drift, not merely on dispatcher size.
- **Evidence:** The generator synthesizes every target-depth statement from
  inventory strings, assigns `native::action_dispatch` as the sole production
  caller, and emits boilerplate deletion text at
  `scripts/architecture/actions-inventory/src/main.rs:1066-1093`. The checker
  never validates that ledger. Its final boundary check only rejects three
  exact legacy facade paths and literal imports of `actions` outside an
  allowlist at `:884-930`. Its final checks at `:1213-1258` cover dispatcher
  lines, definitions, tests, recorded wrapper owners, allowlist IDs, and a short
  forbidden substring list.

  It does not inspect module size or operation count, common-prelude breadth,
  interface call graphs, test locality, private implementation imports,
  `ActionsRouteBoundOpenRuntime`, raw `Value` at the route seam, string-derived
  outcomes, deadline coverage, or repository transaction atomicity. It therefore
  reports `final=true` against the exact violations in `P0101-W1-01` through
  `P0101-W1-04`.
- **Consequence:** The main acceptance signal certifies only a reduced
  dispatcher shell and inventory bookkeeping. It permits both false completion
  and regression back to the risks the plan explicitly froze.
- **Reproducer:** Run `pnpm test:actions-architecture`; it passes. Then inspect
  `page_pdf.rs`, `action_runtime/common.rs`, `action_runtime/tests.rs`, and the
  surviving transitional route runtime. Each violates a frozen criterion that
  the same green gate does not query.
- **Confidence:** high
- **Disposition:** `blocking`; add failing architecture fixtures and structural
  checks for all accepted findings before repairing production source so the
  remediation cannot return the same false green.

### `P0101-W1-07` | WSL Cargo safety does not cover prescribed validation

- **Criterion:** Following the Rust compiler OOM, every WSL Cargo command that
  can compile code must serialize repository builds and run under the explicit
  memory and swap cap. Prescribed validation must be safely runnable rather than
  requiring the auditor or executor to skip an aggregate gate.
- **Evidence:** `scripts/ci/cargo-safe.sh` correctly uses a repository lock,
  `CARGO_BUILD_JOBS=1`, and a user-systemd scope with `MemoryHigh=20G`,
  `MemoryMax=24G`, and `MemorySwapMax=4G`, failing closed when the user manager
  is unavailable. The architecture wrapper and canonical Rust suite use it.

  However, `scripts/test-route-confusion-gates.js:5-53` directly invokes Cargo
  for four tests. `scripts/dev/select-validation.js` emits raw `cargo fmt`,
  `cargo clippy`, and multiple raw `cargo test` commands, including at lines 80
  and 138. `package.json` still defines `build:native` and `build:macos` with raw
  Cargo, and the macOS command starts two Cargo builds concurrently. The
  execution receipt confirms the prescribed aggregate route-confusion gate was
  skipped for this reason.
- **Consequence:** Following repo-selected validation or common build scripts in
  WSL can recreate uncapped or concurrent Rust compilation. The intended
  prevention is therefore advisory rather than closed over the normal repo
  entry points.
- **Reproducer:** Run the non-executing inspection
  `rg -n "command: 'cargo'|cargo (test|build|clippy|fmt)" scripts/test-route-confusion-gates.js scripts/dev/select-validation.js package.json`.
  Do not execute the resulting raw Cargo commands on WSL.
- **Confidence:** high
- **Disposition:** `blocking`; make every WSL-capable selected and aggregate
  entry point route through the guarded wrapper, add a static fail-closed gate,
  and then run the full prescribed no-launch validation under the cap.

## Consolidated remediation prescription

Authorize one bounded remediation packet for all seven accepted blockers, in
this order:

1. Extend the architecture and safety harness first. Add red fixtures for a
   one-handler module, broad common prelude, central private-state test facade,
   surviving transitional adapter, raw route request or result, string-derived
   rollback, split finalize and handoff persistence, unbounded repository phase,
   and raw WSL Cargo entry point.
2. Make route-bound persistence and completion deadline-aware. Every repository
   phase must be bounded by the same total deadline, and the control plane must
   have a final bounded terminalization path that releases the queue at T with
   no remaining task. A missed cleanup confirmation must atomically remove
   active checkout and persist typed `rollback_incomplete` quarantine.
3. Combine final acquisition ownership and optional durable handoff into one
   repository mutation with fault-injection coverage. Resolve the store's
   two-file commit gap without weakening the frozen persistence contract or
   silently changing stored data authority.
4. Replace raw JSON route requests and results with normalized domain types,
   preserve the exact 13 effect operations with typed results, install the
   permanent daemon/browser runtime adapter, delete
   `ActionsRouteBoundOpenRuntime`, and delete outcome string parsing. Prove
   `Planned`, every terminal outcome, all nine fallback predicates, immutable
   snapshot freshness, unchanged retained resources, and ingress authorization
   parity through `RouteBoundOpenCoordinator::open`.
5. Remove `action_runtime/common.rs` as a god prelude, remove the test-only
   re-export facade, deepen cohesive existing owners, recombine shallow targets,
   and move the 262 action tests beside the interfaces they specify. Preserve
   complete 615-definition coverage and the exact six-definition dispatcher
   allowlist.
6. Restore truthful packet rollback boundaries on the unintegrated branch, or
   stop for explicit authority if doing so requires a superseding history
   contract. Rewrite the execution receipt and roadmap to distinguish inventory
   movement, semantic acceptance, validation, installed proof, and remaining
   blockers.
7. Route WSL-capable aggregate, selector, build, format, Clippy, and test entry
   points through `scripts/ci/cargo-safe.sh`; statically reject new raw WSL Cargo
   paths. Rerun selected no-launch gates serially with memory snapshots before
   and after compilation.

Stop after that single remediation. Do not begin installed or live Slice H
proof, and do not begin a broad second discovery pass. The orchestrator must
first adjudicate this ledger. If it accepts the blockers and authorizes Cycle 2,
Cycle 2 may verify only `P0101-W1-01` through `P0101-W1-07` plus critical
regressions introduced by their repairs.

## Audit integrity

The auditor did not edit implementation, plans, tests, commits, runtime state,
or build caches. The only new tracked-path change from this role is this audit
note. The pre-existing untracked inventory build cache remains excluded from
source identity.
