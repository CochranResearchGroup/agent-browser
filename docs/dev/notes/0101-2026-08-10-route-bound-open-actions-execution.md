# Plan 0101 Route-Bound Open And Actions Deepening Execution

Date: 2026-08-10

Role: distinct implementation executor

Plan authority:

- `docs/dev/plans/0101-2026-08-09-route-bound-open-actions-deepening-plan.md`
- `docs/dev/plans/0102-2026-08-09-route-bound-open-cycle2-residual-repair-plan.md`
- `docs/dev/plans/0106-2026-08-10-actions-work-cycle1-remediation-plan.md`

Runtime handle: `/root/resume_actions_deepening` (replacement for the
OOM-terminated `/root/execute_actions_deepening` executor)

Effects authorized: local source, tests, documentation, deterministic generated
architecture artifacts, and green local checkpoint commits. No push, install,
release, browser launch, live runtime, route, display, tenant, or external
effect is authorized.

Current disposition: **PLAN 0106 SOURCE REMEDIATION COMPLETE; BOUNDED
CLOSED-WORLD VERIFICATION PENDING**

The original closeout below is retained as execution history, but its
acceptance claims were rejected by
`docs/dev/notes/0101-2026-08-10-route-bound-open-actions-work-audit.md`.
The final section, “Plan 0106 superseding remediation receipt,” is the current
authority for implementation state, validation, and rollback.

## Preflight P0 Receipt

Disposition: **PASS**

Checkpoint:

```text
24265340 build: freeze actions architecture inventory
```

The checkout was clean at exact campaign base `4f052ba3` before P0 wrote any
path. `git merge-base --is-ancestor` passed for all three predecessor commits.
The P0 checkpoint stages only the ten P0101 harness, fixture, glossary, and
package-script paths listed in its commit.

### Predecessor and write-ownership matrix

| Plan | Landed implementation commit | Validation receipt | Shared write ownership and reconciliation | Current state |
| --- | --- | --- | --- | --- |
| P0098 | `7f89ea49` | `docs/dev/notes/0098-2026-08-09-service-request-normalization-test-receipt.md` | Owns request normalization, HTTP and MCP request adapters, request schemas, and the generated request client. P0101 adopts these paths without duplicate normalization. | Clean and in ancestry |
| P0099 | `0528e5db` | `docs/dev/notes/0099-2026-08-10-workspace-view-projection-test-receipt.md` | Owns dashboard workspace view projection, selected context, and preference control. P0101 adopts the projection paths without dashboard ownership reconstruction. | Clean and in ancestry |
| P0100 | `71e3d691` | `docs/dev/notes/0100-2026-08-10-service-status-projection-test-receipt.md`, including its appended superseding closure of `P0100-T1-01` | Owns status projection, browser authority observation, compatibility, and generated observability fields. Its `actions.rs` edit is reconciled explicitly in the 615-record baseline ledger. | Clean and in ancestry |

### Mandatory shared-path reconciliation

| Shared path or family | Predecessor evidence | P0101 action |
| --- | --- | --- |
| `cli/src/native/actions.rs` | P0100 moved four baseline status helpers and added one typed status command adapter | Manual semantic reconciliation recorded under `predecessorReconciliation`; no lost or unclassified definition |
| native module roots | P0098 added the request module; P0100 added the status projection children | Fast-forward adoption; P0101 adds only reviewed domain owners |
| `package.json` | P0099 and P0100 added dashboard and status gates | Manual nonconflicting script adoption; P0101 adds `test:actions-architecture` and its self-test |
| service contracts and generated clients | P0098 owns request shape; P0100 owns observability shape | No overlap and no public contract change in P0 |
| dashboard projection files | P0099 owns projection; P0100 supplies typed observation input | No overlap; P0101 does not move authority into the dashboard |
| ROADMAP and shared planning docs | predecessor commits contain their own bounded plan and receipt updates | No P0 edit |

The last known validated predecessor head is `71e3d691`. The current plan-only
campaign base is `4f052ba3`. `pnpm validation:select -- --base 71e3d691`
selected `git diff --check` for the plan-only delta; the selector will be rerun
against `71e3d691` for the complete implementation slice.

### CodeGraph and Graphiti discovery

CodeGraph was current at P0 with 428 indexed files, 14,784 nodes, 45,014
edges, and one intentional oversized-file skip:
`cli/src/native/actions.rs`. Indexed surrounding remote-view and status
modules were used before bounded direct parsing of `actions.rs`.

Graphiti runtime doctor was healthy. One focused read of
`agent_browser_main` returned five facts, five nodes, and five episode
previews. It supplied prior route-bound and retained-browser leads but no
current P0101 execution authority. Current plans, commits, source, and tests
remain authoritative.

### Frozen baseline and predecessor reconciliation

The accepted 615-definition baseline is the exact P0099 head immediately
before P0100 changed `actions.rs`:

| Measurement | Value |
| --- | ---: |
| Baseline commit | `0528e5db` |
| Source SHA-256 | `a868a2e9fa81e6debd7e4e676f51752b5a98ddaaf0e2db0236a3f687dc38d111` |
| Bytes | 1,466,172 |
| Lines | 37,746 |
| Production definitions | 615 |
| In-file tests | 260 |

The deterministic `syn` inventory reconciles that baseline to the clean P0101
base:

| Measurement | Value |
| --- | ---: |
| Current base commit | `4f052ba3` |
| Current source SHA-256 | `b8a35f60f18defc9f07101e16ac8d03661623c45f543f65bb50a3b0ce7272228` |
| Current lines | 37,719 |
| Current production definitions | 612 |
| Current in-file tests | 263 |
| P0100-moved baseline definitions | 4 |
| P0100-added current definitions | 1 |

All 615 baseline records have a stable ID, full digest, owner, normalized
signature, packet, responsibility, target owner, movement state, wrapper
state, deletion packet, and final disposition. The inventory has 84 distinct
packet labels, a reviewed eight-definition final dispatcher allowlist, and 57
target-depth entries with an owned invariant, interface operations,
production caller, and deletion-test statement.

Tracked inventory identity:

```text
ed7a7873ec8707e580ed01d4182b0e0f1957fdc1019055e477f6d9f08054c3ad
```

Tracked route-bound fixture identity:

```text
37d64dfc4e5b676e4f6b9b167a275187b60c4cb3c8e90e89ed35bbd8f80f52f4
```

The fixture freezes the typed outcome set, all transaction phases, P0102 total
deadline formula, cancellation and compensation cases, cleanup ownership,
transport authorization seam, and all nine provider-fallback predicates.

### Architecture harness evidence

```text
generated definitions=615 tests=260
actions architecture check passed definitions=612 tests=263 lines=37719 wrappers=0 final=false
self-test passed classified_fixture=accepted unclassified_fixture=unclassified_definition:ari:function:native::actions:intentionally_unclassified_action:6b6f671d2e198e91
```

The expected first final-budget report is red only for the planned monolith
reduction:

```text
actions_line_budget_exceeded:37719
```

No parser, identity, collision, predecessor, classification, allowlist,
wrapper, or self-test blocker remains. The P0 harness passed its own Cargo
format and strict Clippy gates, both pnpm architecture gates, and patch checks
before checkpoint `24265340`.

## Packet Ledger

Historical note: this is the original pre-audit packet ledger. Its grouped
commits did not provide the promised 81 independent rollback boundaries, and
its completion state was not accepted. Plan 0106 supersedes only that commit
granularity with cohesive green checkpoints; it does not waive packet
classification, the 615 responsibility records, source rollback clarity, or
truthful receipt requirements. The current reconciliation follows at the end
of this receipt.

| Packet | State | Commit | Evidence |
| --- | --- | --- | --- |
| `P0101-P00` | complete | `24265340` | 615 of 615 classified; current reconciliation clean; checker and self-test green |
| `P0101-A` | complete | `88fba9ff`, `331d9497`, `96805dde`, `ebd5bfdc` | Route-bound coordinator, exact 13-operation typed runtime seam, total-deadline supervisor, terminal compensation and quarantine, cohesive private owners, and deletion of the superseded state-coupled target path |
| `P0101-B` | complete | `96805dde`, `ebd5bfdc` | Handoff, proof, route-pool, route-lifecycle, target, and compensation ownership are behind the route-open interface |
| `P0101-C` | complete | `940d2575` | Daemon runtime, launch, recovery, profile-lease, navigation, capability, and CDP-free owners replace the runtime bucket |
| `P0101-D01` through `P0101-D05` | complete | `ef15a932` | Probe, UI action, network capture, file transfer, and diagnostics workflows have concrete owners |
| `P0101-E01-01` through `P0101-E21` | complete | `4854b5af`, `5409dabd` | Browser operations and viewer lease behavior are distributed to cohesive browser, network, interaction, stream, and remote-view owners |
| `P0101-F01` through `P0101-F26` | complete | `6fbceca5`, `5409dabd` | Service State, inventory, access, health, retained-state, trace, incident, lifecycle, route repair, and viewer lease operations have concrete owners |
| `P0101-G` | complete | `3a428271` | Both transitional facades are deleted; direct owner imports, final inventory state, reverse-import enforcement, and the final architecture gate are green |
| `P0101-H` | prohibited in this role | none | Installed and live proof requires a separate effect-boundary authorization |
| `P0102-R01` through `P0102-R03` | complete | `88fba9ff`, `331d9497`, `96805dde`, `ebd5bfdc`, `4854b5af` | Total-deadline compensation, complete typed outcomes and transport-neutral authorization boundary, and cohesive deep-module targets are implemented |

## Delegation Receipt

- Disposition: `not_spawned`
- Reason: this role is itself the user-authorized distinct Candidate 4
  executor. The next roles are intentionally independent work audit and final
  test, so implementation is kept within this executor rather than delegated
  into overlapping shared source.
- Runtime handle: `/root/resume_actions_deepening`
- Status: source execution complete at `3a428271`; independent audit and test
  roles remain separate

## Slice A implementation checkpoint evidence

The recovered Slice A source no longer uses the compile-pathological cyclic
wildcard module graph. `actions.rs` now owns only the reviewed dispatcher and
shared coordination allowlist: 815 lines, eight production definitions, and no
in-file tests. The other 608 current responsibility records are marked moved,
with zero compatibility wrappers. Route-bound open is owned by
`remote_view::open`. The route-bound coordinator depends on an exact
13-operation typed runtime interface; the production adapter converts launch
failures into typed runtime issues at the adapter boundary.

The deadline supervisor uses the existing `jobTimeoutMs` total deadline. It
reserves `min(15000, max(250, total / 5))` milliseconds for compensation and
does not extend the public timeout. Forward effects observe cancellation and
the forward deadline. Compensation ignores cancellation and remains bounded by
the original total deadline.

Focused deterministic evidence:

```text
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml remote_view::open::tests -- --test-threads=1
4 passed; 0 failed; 1847 filtered out
scope: run-u771.scope
memory peak: 5.3G
memory swap peak: 0B
```

The scripted runtime implements all 13 operations and covers the deadline
formula, cancellation before each mutating effect, forward-reserve exhaustion,
bounded compensation, and completed-effect recording. It launches no browser
and performs no live runtime effect.

The same focused tests were repeated after restoring the eight frozen
dispatcher stable IDs to `actions.rs`:

```text
4 passed; 0 failed; 1847 filtered out
scope: run-u1278.scope
memory peak: 2.4G
memory swap peak: 0B
```

Architecture reconciliation evidence:

```text
actions architecture check passed definitions=8 tests=0 lines=815 wrappers=0 final=false
self-test passed classified_fixture=accepted
movement: 608 moved; 8 retained
```

Strict Rust lint evidence after the focused tests:

```text
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
finished successfully in 55.76s
scope: run-u894.scope
memory peak: 3.3G
memory swap peak: 0B
```

This is an implementation checkpoint, not Slice A acceptance. Remaining Slice
A work includes terminal failure/quarantine persistence, removal of superseded
state-coupled recovery helpers, and decomposition of the current route-open
recovery file into cohesive private owners.

### Control-plane completion handoff

The worker now materializes its configured default timeout into
`jobTimeoutMs`, so the route supervisor and the worker share one total budget.
For direct route-bound open and durable handoff resolution, outer timeout or
cancellation signals the shared token and retains the coordinator future until
bounded compensation returns. The worker then emits the existing timeout or
cancellation envelope. Other commands preserve their prior outer deadline
behavior.

```text
coordinated_cancellation_awaits_terminal_compensation ... ok
coordinated_timeout_signals_then_awaits_compensation ... ok
default_job_timeout_is_materialized_for_the_route_supervisor ... ok
strict clippy ... ok
```

The control-plane tests use only deterministic futures and cooperative tokens.
They perform no browser or live runtime effect.

### Deep route-open ownership and terminal quarantine

The 4,655-line recovery file is now a 30-line facade over eleven explicit
owners. No sibling module uses a wildcard import and no child imports the root
facade. The production module sizes after formatting are:

```text
shared 14        proof 78          compensation 116
deadline 237     operator_route 265 route_pool 305
runtime 310      planner 407        preflight 478
route_lifecycle 687                 target 818
coordinator 853
```

`rollback_incomplete` is now a typed terminal lease lifecycle state. A failed
cleanup confirmation records the unconfirmed external effects, removes the
active route checkout, quarantines the affected route-pool identity, and blocks
a new acquisition matching the quarantined browser, session, route, or display.
Established route, display, browser, and reused-target state remains restored
or preserved.

Guarded validation:

```text
cargo check ... green (17.01s after path repair)
rollback_failure_restores_lease_and_summarizes_cleanup ... ok
rollback_incomplete_is_terminal_and_never_publishable ... ok
remote_view::open::tests ... 4 passed
strict clippy ... green (55.10s)
largest scope: run-u1667.scope, 5.2G memory peak, 0B swap peak
actions architecture: definitions=8 tests=0 lines=815 wrappers=0
```

The untracked `scripts/architecture/actions-inventory/target/` directory is a
225 MiB build-only cache. It remains preserved locally and excluded from
source checkpoints.

Structural caller analysis found no caller for the superseded
`remote_view_open_acquire_tab` state-coupled path; its only wait-helper caller
was inside that same dead path. Both functions were removed with deterministic
source-range verification. The generic runtime target coordinator remains at
366 lines, strict Clippy is green, and all four scripted runtime tests still
pass.

## Historical pre-audit campaign source closeout

Historical executor disposition: **REJECTED BY THE PLAN 0101 WORK AUDIT AND
SUPERSEDED BY PLAN 0106**

Installed and operator-visible acceptance remains outside this executor's
effect boundary. No browser, installed runtime, route, display, tenant, push,
release, or external effect was performed.

### Checkpoint manifest

| Commit | Durable result |
| --- | --- |
| `24265340` | Freeze the stable actions responsibility inventory and architecture harness |
| `8ec4ab43` | Record the accepted P0 preflight receipt |
| `e92edaed` | Serialize WSL Cargo and cap compilation memory and swap |
| `88fba9ff` | Establish the guarded action-runtime seam and route-bound coordinator |
| `331d9497` | Retain the coordinator through bounded compensation on timeout and cancellation |
| `96805dde` | Split route-bound open into cohesive private owners |
| `ebd5bfdc` | Delete the superseded state-coupled route-target path |
| `940d2575` | Split daemon and browser runtime ownership |
| `ef15a932` | Extract the five service workflow owners |
| `4854b5af` | Distribute browser operation ownership |
| `de5ec433` | Format the architecture split harness |
| `6fbceca5` | Distribute service command ownership |
| `5409dabd` | Assign remote viewer lease ownership |
| `3a428271` | Delete both migration facades and enable the final architecture gate |

### Final responsibility and architecture evidence

The frozen inventory still contains exactly 615 stable baseline responsibility
records. Its final state is 609 moved and six retained, with the four
predecessor-moved dispositions included in the moved population. No record is
unclassified and no compatibility wrapper remains.

```text
actions architecture check passed definitions=6 tests=0 lines=861 wrappers=0 final=true
self-test passed classified_fixture=accepted
```

The six retained production definitions are the reviewed dispatch and shared
coordination allowlist. `actions.rs` defines no domain struct, enum, or union;
contains no repository mutation, raw CDP, process-command, route-pool,
acquisition-lease, durable-handoff, or Service State projection logic; and has
no compatibility branch. The checker now also fails when either migration
facade exists, when a typed domain definition returns to dispatch, or when an
extracted production module reverse-imports `actions` outside the explicit
test and ingress consumers.

Both temporary re-export files are deleted:

```text
cli/src/native/action_runtime/browser_operations.rs
cli/src/native/action_runtime/service_commands.rs
```

Production consumers import concrete owners directly. The largest production
file in the private `action_runtime` ownership tree is `runtime/launch.rs` at
1,265 lines. The intentionally test-only `action_runtime/tests.rs` is 11,737
lines and does not count toward the `actions.rs` production budget. The final
`actions.rs` result is below both plan budgets: 861 of 2,500 lines and six of
35 production definitions.

### Validation receipt

| Gate | Result |
| --- | --- |
| Final actions architecture checker | 6 definitions, 0 tests, 861 lines, 0 wrappers, `final=true` |
| Architecture checker self-test | Classified fixture accepted; unclassified fixture rejected |
| Focused action runtime tests | 262 passed, 0 failed, 1,594 filtered |
| Prescribed partitioned Rust suite | 1,799 passed, 0 failed, 57 ignored; all 1,856 discovered tests accounted |
| Focused CDP partition | 74 passed, 0 failed |
| Strict CLI Clippy | Green with `-D warnings`; final guarded run completed in 30.15 seconds |
| Harness strict Clippy | Green with `-D warnings` |
| Rust formatting and patch checks | Green |
| Service client no-launch suite | Green, including generated files, type coverage, request and observability helpers, managed-profile flow, and example dry runs |
| Dashboard workspace and inspector contracts | Green |
| Route handoff, dashboard workspace, RDP route, and Guacamole hardening static fixtures | Four passed |

An initial unpartitioned diagnostic Rust run found four environment-coupled
failures after 1,795 passes: incident resolve metadata, ambient display,
profile lease waiting, and service monitor interval. The repo-prescribed
partitioned harness was missing the newly relocated
`native::action_runtime::tests` partition, and the ambient display assertion
still expected the preexisting fallback behavior. The harness partition and
assertion were corrected. The complete prescribed rerun then passed all 1,799
executed tests with zero failures and 57 intentional ignores.

The aggregate `test:route-confusion-gates` script was not invoked because it
launches Cargo directly, which is prohibited on WSL. Its four Rust fixtures ran
inside the green guarded canonical suite, and its four no-Cargo fixtures were
run separately and passed. The live CDP streaming gate, ignored E2E suite,
install doctors, and neutral live remote-view smokes were not run because they
would cross this role's explicit no-browser and no-runtime effect boundary.
No installed-runtime proof is claimed.

Every compiling command used `scripts/ci/cargo-safe.sh`, no two Cargo processes
ran concurrently, and no cgroup or OOM failure occurred. At the final compile
boundary the host had 37 GiB available before Clippy and 40 GiB afterward;
swap use remained below 1 GiB of 32 GiB. The 225 MiB untracked
`scripts/architecture/actions-inventory/target/` build cache remains preserved
and excluded from every source checkpoint.

### Remaining gates and authority

The executor has no source blocker. Candidate 4 still requires the planned
independent work audit and independent tester disposition. Installed and
operator-visible acceptance additionally requires a separately authorized
Slice H with the ignored E2E suite, release-candidate installation, doctors,
and neutral live remote-view proof. Those gates cannot be inferred from the
green source and no-launch evidence above.

## Plan 0106 superseding remediation receipt

Disposition: **SOURCE REMEDIATION GREEN; CLOSED-WORLD AUDIT AND EFFECT-BOUNDARY
ACCEPTANCE REMAIN OPEN**

Plan 0106 accepted findings `P0101-W1-01` through `P0101-W1-04`,
`P0101-W1-06`, and `P0101-W1-07` in full. It accepted the receipt, roadmap,
and rollback-evidence portions of `P0101-W1-05`. This was one bounded
remediation pass. It did not run a browser, ignored end-to-end test, install,
doctor, route, display, tenant, release, push, or other external effect.

### Commit-granularity adjudication

The original executor did not create the promised 81 independently green
packet commits. The grouped commits in the historical ledger cannot isolate
each packet with one revert, and this receipt does not claim otherwise.
Rewriting already-green history into retroactive commits would destroy rather
than improve execution evidence. Plan 0106 therefore supersedes only the
81-commit granularity requirement with the cohesive green checkpoints below.
The 84 packet labels, all 615 stable responsibility records, six-definition
dispatcher allowlist, movement dispositions, and source rollback mapping
remain authoritative.

### Cohesive checkpoint and rollback manifest

| Commit | Finding or authority | Durable rollback unit |
| --- | --- | --- |
| `1bd92f51` | Plan 0106 | Records the bounded remediation authority and the explicit commit-granularity adjudication |
| `1a53332a` | W1-01 through W1-07 fixtures | Freezes the accepted red source and WSL-safety regressions before remediation |
| `1c3346f9` | W1-07 | Routes WSL-capable Cargo entrypoints through the fail-closed wrapper and adds the static gate |
| `581130c6` | WSL build guard | Preserves the repository lock and 24 GiB memory plus 4 GiB swap cap while allowing four bounded jobs |
| `338978dd` | W1-02 | Makes Service State and durable handoff persistence one recoverable two-file store transaction with four injected failure boundaries |
| `4f0dda8c` | W1-01 | Puts repository phases and control-plane terminalization inside the total deadline and drops unfinished work at return |
| `240e6a94` | W1-03 | Installs typed invocation, result, blocker, compensation, and outcome seams plus the permanent daemon adapter |
| `5cb66f4b` | W1-04 | Recombines the four rejected shallow action owners into cohesive browser, capture, service inventory, and stream owners |
| `4f6c25cb` | W1-04 | Deletes the common god prelude and replaces it with direct owner imports and the narrow cancellation owner |
| `31dfb3ee` | W1-04 and W1-05 evidence | Deletes the central test facade and colocates all 261 tests in 27 owner or dispatcher modules |
| `fb50e5e6` | W1-06 | Binds the primary architecture gate to the repaired structures and their exact test layout |

These commits are cohesive rollback units, not retroactive packet substitutes.
Rollback proceeds in reverse dependency order with `git revert`, starting at
the newest affected checkpoint. Reverting a lower checkpoint also requires
reverting later checkpoints that import its types or assert its architecture.
The responsibility inventory supplies the packet, stable ID, target owner, and
deletion packet for source selection; the table above supplies the actual
reviewable commit boundary. No history rewrite is required or authorized.

### Finding reconciliation

| Finding | Current source result | Evidence |
| --- | --- | --- |
| `P0101-W1-01` | Repository snapshot, finalization, compensation, and terminalization share the existing total `jobTimeoutMs`; no coordinator task remains after return | Deadline supervisor fixtures include repository-future drop and control-plane unfinished-execution drop |
| `P0101-W1-02` | Acquisition finalization and an optional durable handoff mutate one state snapshot and commit through one recoverable store operation | Fault injection covers handoff write, state write, handoff rename, and state rename; neither durable file can remain ahead after recovery |
| `P0101-W1-03` | The coordinator accepts typed direct or durable invocations and returns the seven exhaustive typed outcomes through `DaemonRouteBoundOpenRuntime` | All seven outcomes and all nine fallback predicates are checked by the architecture gate and exercised through coordinator tests; the transitional adapter and string fallback are deleted |
| `P0101-W1-04` | The broad common prelude, central test facade, and four rejected shallow owners are deleted | 261 tests are distributed across 27 owner or dispatcher modules; exact reverse imports remain limited to the four reviewed ingress and parity consumers |
| `P0101-W1-05` | The original granularity deviation is explicit; current rollback units and roadmap state are recorded truthfully | This manifest and the service-roadmap checkpoint replace the rejected completion narrative without rewriting history |
| `P0101-W1-06` | The primary checker inspects repaired production and test structures instead of accepting generated prose alone | It checks typed outcomes, fallback fields, store boundaries, deadline fixtures, test locality, old adapters, raw seams, split mutations, facades, reverse imports, dispatcher budgets, and responsibility counts |
| `P0101-W1-07` | Aggregate, selector, format, Clippy, test, and architecture Cargo paths use the serialized cgroup wrapper on WSL | Static fail-closed entrypoint gate is green; all executor Cargo boundaries used the wrapper |

### Current responsibility and architecture state

The deterministic inventory retains exactly 615 baseline responsibility
records. Its current disposition is 609 moved and six retained, with zero
compatibility wrappers and the exact reviewed six-definition dispatcher
allowlist. The final checker reports:

```text
actions architecture check passed definitions=6 tests=0 lines=897 wrappers=0 final=true
```

`actions.rs` contains no domain struct, enum, or union and no repository,
process, raw CDP, route-pool, acquisition-lease, durable-handoff, or Service
State mutation authority. The only native production consumers importing the
dispatcher are the reviewed control-plane and stream HTTP ingress modules;
the other two allowed imports are parity and end-to-end test consumers.

The migrated test distribution is exact:

```text
27 physical owner or dispatcher test modules
261 migrated tests
193 dispatcher and execute-command integration tests
68 public-owner domain tests
0 tests in the deleted central facade
```

### Remediation validation

| Gate | Current result |
| --- | --- |
| Primary actions architecture gate | Green: 6 definitions, 0 in-file tests, 897 lines, 0 wrappers, `final=true` |
| Structural remediation gate | Green: seven outcomes, nine predicates, four store boundaries, deadline-drop fixtures, exact 261-test layout |
| WSL Cargo entrypoint gate | Green |
| Guarded Rust check with tests | Green |
| Guarded strict CLI Clippy | Green with `-D warnings`; final W1-04 run completed in 30.55 seconds |
| Migrated dispatcher partition | 193 passed, 0 failed |
| Migrated public-owner partitions | 68 passed, 0 failed |
| Canonical partitioned suite before final path-only ownership reconciliation | 1,810 executed, 0 failed, 57 ignored; every serial partition green |
| Formatting and patch checks | Green |

The final ownership move changed module paths and imports, not test bodies or
production behavior. After that move, all 261 migrated tests were executed
again and passed, guarded check with tests and strict Clippy passed, and the
structural checker rejected any reverse import outside the unchanged four-site
allowlist. Host memory remained between 38 and 39 GiB available at the final
compile boundaries, swap remained 2.0 GiB of 32 GiB used, and no cgroup or OOM
failure occurred.

### Remaining acceptance boundary

Source execution is green for the single authorized Plan 0106 remediation
pass. The next permissible review is closed-world over findings
`P0101-W1-01` through `P0101-W1-07` and critical regressions introduced by
their fixes; no third broad work-audit cycle is authorized. Installed and
operator-visible acceptance remains separate and requires explicit authority
for ignored end-to-end tests, release-candidate installation, doctors, and a
neutral live remote-view proof. None of those effects is claimed here.
