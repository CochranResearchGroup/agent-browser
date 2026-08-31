# Plan 0144 | Lease Authority Coordination And Revocation

Date: 2026-08-31

State: OPEN

Execution state: `slice_b_kernel_and_access_in_progress`

Lane: P144

Source baseline: `c21118a30b01eaf23acabdec80e81f5d79a130b3`

Branch: `plan/lease-authority-coordination`

Target: `main`

Integration model: cohesive validated checkpoints on a short-lived topic
branch, followed by a merge to `main`.

Authority: PLAN, SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, ISOLATED
DEVELOPMENT-RUNTIME VALIDATION, EXACT PRODUCTION-CANDIDATE INSTALLATION, AND
POST-INSTALL PROVIDER-FREE LIVE SMOKE ARE IN SCOPE. Tenant navigation,
provider mutation, broad process cleanup, manual database editing, and
unreviewed lease revocation are out of scope.

Depends on:

- Plan 0134 principal, profile, crash-epoch, and lifecycle coherence;
- Plan 0137 acquisition and recovery contracts;
- Plan 0142 Service State concurrency and client recourse reliability; and
- Plan 0143 workstation upgrade self-admission repair.

## Executive Decision

Agent Browser will replace inferred lease authority with one canonical lease
authority kernel used by every planner, admission gate, runtime owner,
session, tab, presentation route, installer, recovery action, doctor, and
client projection.

The kernel separates two different things that the current model conflates:

1. an active claim, which is current fenced mutation authority; and
2. a lease event or terminal record, which is append-only operational history.

History can explain what happened. It cannot reserve a profile, block a
client, create an owner, extend a deadline, or prove that a process exists.
Only a current active claim returned by an atomic acquisition can do that.

Ordinary ephemeral clients receive short, renewable claims whose authority
expires without a clean shutdown. Lease-aware software may request strict
claims, but must provide a recovery policy and use first-class renew, release,
revoke, and recovery operations. No strict claim may require state-file
editing or a product hotfix to recover.

## Goal

Make the reported and enforced lease state truthful under ephemeral agents,
sleeping workers, process crashes, daemon replacement, runtime transfer,
mixed-version installation, and retained history.

The design must make these defect classes unrepresentable:

- a historical record becoming an operational blocker;
- a dead or absent session being treated as a current holder;
- an access plan being executable while admission rejects the same evidence;
- two Agent Browser components independently deciding who owns a profile;
- a stale holder continuing to mutate after expiry, transfer, or revocation;
- an unbounded transition such as `closing`, `prepared`, or `transferring`;
- an abandoned strict claim requiring manual state editing;
- a read path creating or reattributing durable state;
- a successful effect being retried because its receipt was not durably
  observed; and
- a logical lease being used as proof of a physical browser process, route,
  display, or daemon collision.

The system cannot make process crashes, storage failures, or operator mistakes
impossible. It must make them converge to bounded, inspectable, revocable
states without inventing authority or indefinitely blocking ordinary work.

## Audited Failure Inventory

The hotfix history and Plans 0128, 0134, 0137, 0142, and 0143 show recurring
failures in six families:

1. **History promoted to authority.** Retained sessions and legacy principal
   evidence produce lease warnings or blockers after work is terminal.
2. **Identity-axis collapse.** Service principal, client session, daemon
   route, durable browser, profile, process, owner generation, and
   presentation route are treated as interchangeable strings.
3. **Split planning and enforcement.** The access planner reports an
   executable launch while the daemon later rejects it with different or
   newer inferred identity evidence.
4. **Unbounded lifecycle transitions.** `closing`, `prepared`, and
   `transferring` survive the process that could finish or compensate them.
5. **Projection drift.** Read paths and dashboard projections create
   ready-looking browsers, owners, or warnings from stale and processless
   evidence.
6. **Effect and installation ambiguity.** A mutation may succeed before its
   caller observes a receipt, or a candidate runtime may inherit only part of
   the authority model and block its own transaction.

No individual hotfix closes these families. This plan closes them through one
shared authority model and public recovery plane.

## Ubiquitous Language

| Term | Meaning |
| --- | --- |
| active claim | the only record that can authorize or block a mutation |
| lease event | append-only evidence of acquisition, renewal, transfer, expiry, release, revocation, or recovery |
| fencing token | monotonically increasing token required by every effect under a claim |
| ephemeral claim | short renewable authority designed for clients that may disappear without cleanup |
| strict claim | explicitly requested authority with declared recovery ownership and bounded revocation policy |
| transition deadline | mandatory deadline for a nonterminal lifecycle transition |
| authority snapshot | revision-bound result consumed by planning, acquisition, admission, and effects |
| physical collision | current process, socket, lock, display, or route evidence independent of logical lease authority |
| recovery controller | the one component authorized to finish, compensate, or revoke a strict claim |

## Canonical State Model

The lease authority kernel owns these durable collections:

```text
activeClaims[resourceKey] -> ActiveClaim
claimRevisions[resourceKey] -> monotonic revision
leaseEvents[] -> append-only LeaseEvent
effectReceipts[idempotencyKey] -> EffectReceipt
cleanupObligations[id] -> CleanupObligation
```

`activeClaims` is the sole operational index. Terminal rows never remain in
that index. A historical query joins `leaseEvents`, terminal receipts, and
cleanup obligations without changing current authority.

Every active claim contains at least:

- typed resource key and optional parent claim id;
- stable principal id and capability provenance;
- claim mode, state, revision, and fencing token;
- acquisition, heartbeat, expiry, and transition deadline timestamps;
- boot epoch and current process evidence when process ownership matters;
- runtime owner generation when lifecycle ownership matters;
- strict recovery controller and revocation policy when mode is strict; and
- subordinate claim ids or a derivable child index.

## Frozen Invariants

### Truth and safety

1. Only `activeClaims` may authorize or block an operation.
2. Lease events, terminal sessions, terminal owners, old generations, and
   doctor findings are never consulted as operational authority.
3. Every effect validates the resource key, principal, claim revision,
   fencing token, expiry, parent authority, and relevant owner generation in
   one authority snapshot.
4. Expiry, release, revocation, transfer commit, and recovery monotonically
   advance the fencing token before later effects can commit.
5. A stale holder cannot mutate even if its process is still running.
6. A read path cannot acquire, renew, transfer, revoke, release, recover,
   synthesize, or reattribute authority.

### Liveness

7. Ephemeral claims have finite expiry and require no clean release.
8. Every nonterminal transition has a deadline and one named reconciler.
9. A strict claim declares its recovery controller and revocation policy at
   acquisition. Missing recovery metadata makes strict acquisition invalid.
10. Every abandoned strict claim has a supported inspect, revoke-plan,
    revoke-apply, and recovery path that does not edit raw state.
11. A terminal transition with an unsatisfied physical cleanup obligation may
    block only the exact unsafe physical resource, not unrelated profile work.

### Consistency

12. Planning and execution consume the same authority snapshot and evaluator.
13. A plan is advisory. Authority is granted only by atomic compare-and-swap
    acquisition against the current claim revision.
14. Exactly one module owns active-claim mutation. Other components call it
    and never project their own operational lease state.
15. Hierarchical child claims cannot outlive or exceed their parent claim.
16. Runtime-owner transfer is a receipted saga coordinated by the kernel, not
    an independent lease implementation.
17. Logical authority and physical collision evidence are separate axes and
    are reported separately.

### Recoverability and compatibility

18. Every mutation is idempotent by operation key and persists its effect
    receipt before reporting success.
19. Repeating an operation returns the same terminal receipt without duplicate
    effects.
20. Mixed-version runtimes fail before effect when they cannot understand the
    current claim schema or fencing token.
21. Migration preserves history while building a new active index only from
    evidence that is current, typed, nonterminal, and positively proven.
22. An ambiguous legacy record migrates to history or quarantine, never to an
    active blocker.
23. Administrative revocation is exact, revision-bound, audited, and narrower
    than generic force unlock.
24. Doctor health is based on current safety and recoverability. Historical
    warnings remain queryable but do not make the current system unhealthy.
25. Lease time is authority-owned. Callers request a policy class, not an
    arbitrary expiry, and wall-clock rollback cannot lengthen a claim.
26. Resource keys are canonicalized before admission. Aliases, profile
    shorthands, paths, and route projections cannot create two claims for one
    physical resource.
27. Revision and fencing counters fail before mutation on exhaustion. They
    never saturate, wrap, or reuse an earlier value.
28. Idempotency survives terminalization and history archival. Replaying an
    acquire, renew, release, recover, revoke, or effect operation returns its
    original receipt rather than creating new authority.
29. Multi-resource work is acquired as one ordered bundle or as a bounded,
    receipted saga. A partial profile, route, display, browser, or installer
    acquisition cannot become an indefinite blocker.
30. Parent authority is revalidated recursively at every child effect, not
    only when the child is created. Parent expiry or fencing invalidates the
    child immediately even if a stale child row remains.
31. History retention and compaction preserve fencing high-water marks and
    idempotency receipts. Unbounded history growth cannot exhaust the active
    authority store or change admission.

## Claim Modes

### Ephemeral

- default for ordinary Agent Browser clients;
- finite server-selected TTL with bounded renewals;
- automatic expiry and fencing without requiring client shutdown;
- no persistent blocker after the client, worker, session, or daemon exits;
- optional best-effort release for fast reuse.

### Strict

- opt-in for lease-aware software only;
- finite heartbeat or recovery-grace policy, never an undocumented permanent
  hold;
- named recovery controller and supported crash-recovery workflow;
- explicit subordinate work and cleanup obligations;
- first-class administrative revoke plan and apply surfaces;
- revocation advances fencing before cleanup and reports any remaining exact
  physical obligation separately.

## Public Control Plane

The public operations are:

- `lease list|inspect|history|doctor` for read-only truth;
- `lease acquire|renew|release` for ordinary lifecycle control;
- `lease revoke plan|apply` for exact administrative recovery;
- `lease recover plan|apply` for strict owner recovery;
- `lease explain` for dominant blocker, active claim, physical evidence, and
  safe recourse; and
- `lease watch` for revision-bound changes without polling-derived authority.

The same operations and schemas must remain aligned across CLI help, HTTP,
MCP, generated client, dashboard, README, docs site, shared skill, and inline
documentation.

Revocation plan and apply are separate. Apply requires the plan id, exact
claim revision, fencing token, principal or administrative authority,
observed subordinate work, and expiry. A stale plan has zero effect and
returns the refreshed current claim.

## Coordination Boundaries

The kernel is authoritative for:

- profile and runtime-lane claims;
- service-session and tab child claims;
- viewer and controller child claims;
- lifecycle-owner transfer authority;
- installer transaction claims;
- transition deadlines and reconciliation eligibility;
- revocation and recovery plans;
- effect fencing and receipts; and
- lease health diagnostics.

Browser process discovery, daemon census, sockets, profile locks, displays,
routes, and provider state remain owned by their current subsystems. They
submit typed current evidence to the kernel. They do not convert historical
lease records into physical truth, and the kernel does not infer a process
from a logical claim.

## Execution Slices

### Slice A | Separate history from operational authority

- Add a regression proving that retained released or expired legacy sessions
  remain visible without blocking axes or an unhealthy doctor result.
- Prove that adding arbitrary terminal history cannot change an access or
  admission decision.
- Classify live legacy evidence separately from historical legacy evidence.
- Keep history inspectable without deleting or rewriting it.

Exit condition: historical lease records are observational only and cannot
change current admission.

### Slice B | Introduce the canonical active-claim kernel

- Add the active claim, event, fencing, expiry, and transition-deadline model.
- Centralize current-authority evaluation behind one interface.
- Route profile acquisition and authenticated work authority through it.
- Preserve current public responses through an explicit compatibility
  projection.

Exit condition: the current lease decision has one mutation owner and one
evaluator.

### Slice C | Make acquisition atomic and effects fenced

- Replace executable-plan assumptions with atomic acquisition.
- Require claim revision and fencing token at every profile, session, tab,
  owner, and route mutation seam.
- Add idempotent effect receipts and uncertain-effect recourse.

Exit condition: two contenders cannot both receive authority, and a stale
holder cannot commit.

### Slice D | Add ephemeral expiry and transition reconciliation

- Apply finite TTLs and heartbeat rules to ordinary clients.
- Add deadlines and named reconcilers for closing, prepared, transferring,
  recovery, and revocation transitions.
- Separate exact physical cleanup obligations from logical claim expiry.

Exit condition: client or daemon disappearance converges without manual
release or an indefinite gate.

### Slice E | Add strict claims, recovery, and revocation

- Validate strict recovery metadata at acquisition.
- Implement revision-bound recover plan and apply.
- Implement administrative revoke plan and apply.
- Fence before cleanup and retain exact cleanup obligations on failure.

Exit condition: an abandoned strict claim is recoverable through supported
surfaces without raw state editing.

### Slice F | Coordinate runtime-owner transfer and hierarchy

- Make owner transfer consume parent claim authority.
- Make sessions, tabs, viewers, controllers, and routes bounded children.
- Compensate or quarantine incomplete transfer sagas at their deadlines.

Exit condition: no Agent Browser subsystem maintains an independent competing
definition of current ownership.

### Slice G | Align public surfaces and migration

- Add CLI, HTTP, MCP, generated client, dashboard, documentation, and skill
  parity.
- Migrate only positively proven live authority into `activeClaims`.
- Retain all other legacy material as history or explicit quarantine.
- Add mixed-version read, write, rollback, and installer compatibility.

Exit condition: old and new generations cannot silently disagree about
operational authority.

### Slice H | Validate, integrate, install, and accept

- Run focused, presubmit, comprehensive, and isolated development checks.
- Install one exact development-approved production candidate.
- Prove provider-free acquisition, expiry, revocation, transfer recovery,
  restart convergence, and zero process residue.
- Audit current production history to prove it is nonblocking without deleting
  it.

Exit condition: source, installed identity, live authority, and current
runtime receipts all satisfy the acceptance matrix.

## Acceptance Matrix

| Case | Required outcome |
| --- | --- |
| released or expired legacy session retained for years | visible in history, absent from operational blockers |
| arbitrary terminal history added | identical access and admission result |
| ephemeral holder process crashes | claim expires, fencing advances, next acquisition succeeds |
| stale process continues after expiry | every later effect rejected by fencing token |
| two simultaneous acquisitions | exactly one claim, one conflict outcome, no duplicate browser |
| planner snapshot becomes stale | acquisition fails before effect with refreshed claim |
| active foreign principal holds exact profile | bounded conflict with current claim proof |
| same principal starts a new client session | rejoin or child claim, never self-conflict by session name |
| strict holder disappears | recover or revoke available through public surfaces |
| stale revoke or recovery plan | zero effect and refreshed current revision |
| strict revoke cleanup fails | logical authority fenced, exact cleanup obligation retained |
| transition controller crashes | deadline reconciliation finishes, compensates, or quarantines |
| owner transfer interrupted | one authoritative owner generation after replay |
| parent claim expires or is revoked | all child effects fenced immediately |
| processless ready-looking browser record | historical or quarantined projection, never live authority |
| logical claim exists with physical profile lock collision | separate exact physical blocker and recourse |
| effect succeeds but response is lost | retry returns same durable receipt without duplicate effect |
| candidate runtime cannot read claim schema | installation fails before runtime-owner mutation |
| rollback to compatible old generation | authority and history remain readable without split ownership |
| doctor sees historical inconsistencies only | current health remains healthy with history count reported |
| caller requests an excessive or backdated TTL | server selects a bounded expiry from current authority time |
| wall clock moves backward after acquisition | remaining authority never increases |
| profile shorthand and canonical path name the same profile | one canonical resource key and one possible winner |
| revision or fencing counter reaches its numeric limit | mutation fails atomically before authority changes |
| completed acquisition operation is replayed after expiry | original terminal receipt, no new claim |
| route bundle fails after profile admission | bounded compensation or exact cleanup obligation, no stranded claim |
| child row remains after parent expiry | every child effect is rejected immediately |
| historical events are archived | fencing and idempotency high-water marks remain unchanged |

## Design Completeness Audit | 2026-08-31

The six hotfix families are covered by the architecture, but the first draft
did not state five cross-cutting failure modes strongly enough. They are now
part of the frozen invariants and acceptance matrix:

1. authority-owned time and bounded TTL policy, including wall-clock rollback;
2. canonical resource identity across shorthand, path, route, and owner aliases;
3. non-saturating revision and fencing counters;
4. terminal idempotency retained independently of active claims and event
   history; and
5. ordered bundle or bounded-saga semantics for operations that need several
   resources.

The audit also makes recursive parent fencing and history compaction explicit.
Without these controls, a single kernel could still admit duplicate aliases,
revive a completed request, strand a partial acquisition, or allow a child to
outlive its parent. These are required before the redesign can be described as
structurally recurrence-resistant.

## Validation Contract

Each slice names the protected invariant and demonstrates the defect before
the fix when practical. Tests use the cheapest stable public or contract seam.

Touched Rust code requires:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
```

Every checkpoint runs focused tests for its authority seam. Before push and
installation, run:

```bash
pnpm validation:select -- --base c21118a30b01eaf23acabdec80e81f5d79a130b3
```

Service action or schema changes require parity across Rust contract metadata,
JSON schema, CLI, HTTP, MCP, generated client, dashboard, help, README, docs
site, skill, and inline comments. Isolated development publication and browser
smoke precede any production candidate installation.

## Migration And Rollback

Migration first writes a sealed candidate active index and a source-state
digest. It imports only claims with current principal capability, nonterminal
state, unexpired time, current owner generation when required, and current
process evidence when process ownership is asserted. Everything else remains
history or quarantine.

The candidate validates cardinality, hierarchy, fencing monotonicity, expiry,
transition deadlines, and reverse-reader compatibility before the selected
state pointer changes. A failure leaves the old state untouched. Rollback
restores the prior compatible state pointer while retaining new append-only
events and receipts in a reader-compatible terminal ledger.

## Bounds

- Maximum two implementation attempts per slice before a local reframe.
- Maximum one broad drift-discovery review for the full goal.
- Maximum one closed-world remediation cycle for accepted blocking findings.
- Durable checkpoint every three slices, ninety minutes, or any material
  authority or installation transition, whichever comes first.
- No automatic retry after uncertain external effect.
- Historical warnings never consume an implementation retry or block an
  unrelated ready slice.

Checkpoint fields are state transition, acceptance state, progress
classification, evidence, material blockers, and next action or stop reason.

## Hard Stops

- Do not delete retained history to make doctor green.
- Do not add a generic force-unlock or wildcard revoke.
- Do not infer principal authority from a client session name.
- Do not infer physical liveness from a lease, owner row, route, or status
  label.
- Do not allow a planner response to reserve authority.
- Do not install a candidate that has not passed exact development identity,
  compatibility, and rollback checks.
- Do not use tenant profiles or provider navigation for source acceptance.
- Do not terminate a process without exact current process and ownership
  evidence.

## First Execution Packet

1. Add the retained-history regression at the profile-lease projection and
   doctor seam.
2. Demonstrate the current false warning before the fix.
3. Classify a legacy profile with no active session or owner as historical,
   with no blocking axes and no mutation actions.
4. Preserve current blocking behavior for positively active unproven legacy
   authority.
5. Run focused Rust tests, formatting, and changed-surface selection.
6. Record the red and green evidence here and commit the slice.

This packet changes no live runtime state and performs no installation.

## Slice A Checkpoint | 2026-08-31

State transition: `slice_a_history_separation_in_progress` to
`slice_a_complete_slice_b_ready`.

Acceptance state: retained released and expired legacy sessions remain visible
as `historical`, expose read actions only, carry no blocking identity axes, and
leave profile-lease doctor healthy. Current unproven legacy work remains an
observation-only identity-reconciliation blocker. Adding terminal session
history does not change the access-plan decision.

Progress classification: `outcome_progress`.

Evidence:

- Red: the focused historical regression failed because the projected state
  was `identity_reconciliation_required` instead of `historical`.
- Green: the focused historical regression passed after current and historical
  evidence were separated.
- Three legacy-profile tests passed, including the current-authority negative
  control.
- The terminal-history access-plan invariance regression passed.
- The 45-test `service_access_plan` family passed serially.
- Rust formatting and strict Clippy passed.
- Documentation build, API/MCP parity, generated client checks, JavaScript
  type checks, remote-view documentation checks, and every validation-selector
  workstation fixture passed.

Material blockers: none for Slice B. The unrelated P110 worktree retains one
uncommitted note and does not overlap this lane.

Next action: introduce the canonical active-claim kernel behind a compatibility
projection, beginning with the smallest profile authority seam.

## Slice B Kernel And Access Checkpoint | 2026-08-31

State transition: `slice_a_complete_slice_b_ready` to
`slice_b_kernel_and_access_in_progress`.

Acceptance state: Service State now has a backward-readable canonical lease
authority envelope containing a resource-keyed active-claim map, durable
fencing counters, authority revision, and append-only events. Atomic
acquisition validates expiry, expected revision, parent authority, strict
recovery metadata, and idempotent replay. Access planning consults the current
profile claim even when no session projection exists. A matching principal may
continue, an unauthenticated caller must authenticate, and a foreign principal
must wait.

Progress classification: `outcome_progress`.

Evidence:

- Red: the first kernel regression failed with `Unsupported` while retained
  terminal history was present.
- Green: five kernel tests pass for history independence, revision
  compare-and-swap, strict recovery requirements, Service State round-trip,
  and repository-level two-contender atomicity.
- Red: a fencing high-water mark at the numeric limit had no typed failure and
  would have saturated, reusing the prior token.
- Green: counter exhaustion now fails before any authority mutation; the six
  kernel tests include an exact state-equality regression for this boundary.
- Red: an access plan with a canonical claim but no session incorrectly
  returned `launch_new_browser`.
- Green: the access plan now returns `authenticate_for_profile_reuse`, exposes
  the claim id, revision, fencing token, and principal, and reports one active
  lease. Matching and foreign principal controls also pass.

Material blockers: profile-lease doctor and effect admission still use the
legacy compatibility projection. The public acquire, renew, release, recovery,
and revocation operations do not yet issue or consume canonical claim tokens.
This checkpoint must not be installed as the completed lease redesign.

Next action: project canonical claims through profile-lease doctor, then make
profile acquisition and daemon effects consume the same atomic claim before
adding renew, release, recovery, and revocation.
