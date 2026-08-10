# Plan 0101 Route-Bound Open And Actions Deepening Execution

Date: 2026-08-10

Role: distinct implementation executor

Plan authority:

- `docs/dev/plans/0101-2026-08-09-route-bound-open-actions-deepening-plan.md`
- `docs/dev/plans/0102-2026-08-09-route-bound-open-cycle2-residual-repair-plan.md`

Runtime handle: `/root/resume_actions_deepening` (replacement for the
OOM-terminated `/root/execute_actions_deepening` executor)

Effects authorized: local source, tests, documentation, deterministic generated
architecture artifacts, and green local checkpoint commits. No push, install,
release, browser launch, live runtime, route, display, tenant, or external
effect is authorized.

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

| Packet | State | Commit | Evidence |
| --- | --- | --- | --- |
| `P0101-P00` | complete | `24265340` | 615 of 615 classified; current reconciliation clean; checker and self-test green |
| `P0101-A` | implementation checkpoint | this commit | Route-bound coordinator, exact 13-operation typed runtime seam, deadline supervisor, typed outcomes, typed profile-owner collision, deterministic scripted-runtime tests, and stable-ID reconciliation are green; residual Slice A work remains listed below |
| `P0101-B` | pending | none | Handoff surface deepening follows the coordinator |
| `P0101-C` | pending | none | Daemon runtime and browser lifecycle extraction |
| `P0101-D01` through `P0101-D05` | pending | none | Service workflows |
| `P0101-E01-01` through `P0101-E21` | pending | none | Browser operations |
| `P0101-F01` through `P0101-F26` | pending | none | Service State and remote-view commands |
| `P0101-G` | pending | none | Dispatch closeout and final architecture gate |
| `P0101-H` | prohibited in this role | none | Installed and live proof requires a separate effect-boundary authorization |

## Delegation Receipt

- Disposition: `not_spawned`
- Reason: this role is itself the user-authorized distinct Candidate 4
  executor. The next roles are intentionally independent work audit and final
  test, so implementation is kept within this executor rather than delegated
  into overlapping shared source.
- Runtime handle: `/root/resume_actions_deepening`
- Status: active after P0

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
A work includes control-plane cancellation handoff through the coordinator,
default deadline propagation, terminal failure/quarantine persistence, removal
of superseded state-coupled recovery helpers, and decomposition of the current
route-open recovery file into cohesive private owners.
