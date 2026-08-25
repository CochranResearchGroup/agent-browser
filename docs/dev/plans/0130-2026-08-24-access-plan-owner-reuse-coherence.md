# Plan 0130 | Access-plan owner reuse coherence

Date: 2026-08-24

State: OPEN

Execution state: `client_implementation_active`

Lane: P130

Branch: `hotfix/access-plan-owner-reuse-coherence`

Target: `main` at `0535131b61627a1bbff34773bb116e462334aafe`

Integration method: direct local merge after active-lane reconciliation

Authority: SOURCE, PROVIDER-FREE DEVELOPMENT, TRANSACTIONAL WORKSTATION
HOTFIX, AND BOUNDED HARMLESS RUNTIME ACCEPTANCE

Depends on:

- Plan 0069 shared-profile routing and handoff deepening;
- Plan 0117 runtime lifecycle authority and convergence;
- accepted Plan 0128 runtime lifecycle hotfix collection;
- closed Plan 0129 request delivery, lifecycle projection, and cleanup repair;
- the current `last30days-facebook` lifecycle and service-state readback;
- the current active presentation-feature edits on `main`.

## Goal

Make retained-browser acquisition work without caller-side lifecycle knowledge.
When one exact-profile browser is healthy and has a ready lifecycle owner, the
access plan must either reuse that browser with complete route hints or return
one accurate typed blocker. It must never advertise readiness while generating
a request that attempts a forbidden replacement launch.

Preserve authenticated profiles, retained tabs, lifecycle generations, active
feature work, and presentation-route custody throughout the repair.

## Incident

The `last30days-facebook` browser is healthy and retained under logical browser
`session:last30days-facebook--last30days-facebook`. Runtime owner generation 14
is ready and routes to session `handoff-17959ea3e226ee61`.

The service projection records that browser as `attached_existing` on a shared
display. A fresh profile plan requests `remote_headed` on a private virtual
display. Exact launch-posture comparison therefore excludes the existing
browser from reusable candidates.

The same plan reports one `compatibleLiveBrowserCount`, no reusable browser or
session hints, `defaultAcquisition=launch_new_browser`, and
`attention.required=false`. The generated `tab_new` request enters launch
admission and fails with
`runtime_lifecycle_existing_owner_requires_explicit_transition`.

Labeled Last30Days presentation jobs separately fail on the exclusive profile
lease because they also omit the retained browser and session route hints. The
stale Guacamole route is a presentation-axis incident and is not evidence that
the browser or authentication is unhealthy.

## Just-works invariant

For a selected durable profile with one healthy same-profile browser and a
ready lifecycle owner, an access plan must produce exactly one outcome:

1. `reuse_existing_browser` with nonempty, mutually consistent `browserId` and
   `sessionName` hints; or
2. a typed blocker with `attention.required=true`,
   `serviceRequest.available=false`, and no launch-capable request.

The plan must not return `launch_new_browser` while lifecycle replacement is
ineligible. Supplying an explicit session that maps uniquely to a retained
browser must populate both route hints. Missing or ambiguous mappings must fail
closed before queueing.

## Acceptance criteria

1. A provider-free fixture reproduces the transferred-owner posture mismatch
   and fails on the pre-repair source.
2. `compatibleLiveBrowserCount` counts browsers that can accept the planned
   operation. A separate `sameProfileLiveBrowserCount` reports healthy
   same-profile observations when that diagnostic remains useful.
3. Tab acquisition compatibility is independent from presentation posture.
   An exact cooperatively transferred owner can accept a tab when its profile,
   process, CDP endpoint, lifecycle owner, and target evidence agree.
4. Presentation requirements remain strict for operations that require a
   viewer. Relaxing tab reuse must not bypass route, display, stream, or
   controller-lease checks.
5. Top-level `recommendedAction`, `attention`, `profileReuse`,
   `lifecycleReplacement`, and `serviceRequest` cannot contradict one another.
6. A ready lifecycle owner that cannot be reused blocks replacement before a
   request is queued and names the exact safe next action.
7. An explicit session with one retained browser automatically produces both
   route hints. A missing or ambiguous session-to-browser mapping returns a
   typed no-launch error.
8. Cooperative handoff preserves or reconciles the service browser,
   candidate session, tab handles, cleanup policy, profile lease semantics,
   and runtime-owner route as one ownership projection.
9. Browser-process sharing remains separate from viewer-controller
   exclusivity. A viewer lease cannot convert a shared browser process into a
   duplicate-process blocker.
10. Software clients can consume a complete access-plan response without
    reconstructing profile, browser, session, or lifecycle routing.
11. HTTP, MCP, CLI, dashboard, external-ingress, generated-client, schema, and
    documentation surfaces agree on the repaired contract.
12. Provider-free acceptance proves one process across cooperative transfer,
    one tab acquisition, exact tab release, and no duplicate profile lane.
13. Transactional workstation acceptance preserves unrelated browsers and
    the active development runtime.
14. A bounded Last30Days proof obtains and releases one harmless local tab
    through the retained owner without navigating to X, LinkedIn, Facebook, or
    another authenticated provider.
15. Presentation acceptance reattaches the stale route to the same browser
    without launching a replacement or displacing an active controller.

## Work units

### P130-A | Freeze the regression

Add one public-interface access-plan test that models the exact transferred
owner. Assert the just-works invariant rather than private helper calls.

Exit condition: the focused test fails for the current contradiction and
passes only when the plan either routes reuse or returns the typed blocker.

### P130-B | Repair acquisition planning

Deepen the access-plan module so one acquisition decision owns compatibility,
route hints, lifecycle replacement admission, generated request availability,
and attention projection.

Separate these observations:

- same-profile live browser;
- operation-compatible reusable browser;
- presentation-compatible route;
- lifecycle replacement eligibility;
- active profile or viewer lease.

Exit condition: all access-plan fields derive from one acquisition outcome and
the regression passes without weakening lifecycle admission.

### P130-C | Preserve handoff ownership projection

Add a cooperative-transfer fixture that verifies the candidate daemon route,
service browser, service session, tab handles, cleanup policy, and profile
sharing semantics after commit and finalization.

If current transferred state needs repair, extend the supported exact-evidence
reconciliation path. Do not add another owner registry or consumer-side state
rewrite.

Exit condition: one provider-free transfer produces a reusable owner with no
stale browser linkage, duplicate active lease, or orphaned cleanup obligation.

### P130-D | Make clients consume the plan

Add a generated-client helper that accepts the complete access-plan response
and queues the planned tab request. The helper must reject unavailable plans,
missing required route hints, and contradictory acquisition state.

Use the same helper in maintained Agent Browser consumers. Record any
downstream Last30Days source change as a separate repository checkpoint after
the Agent Browser contract is installed.

Exit condition: callers no longer destructure and reconstruct lifecycle route
hints on the normal path.

### P130-E | Reconcile presentation behavior

Wait for the active presentation lane to publish a recoverable checkpoint
before editing its files. Reconcile its route-lifecycle behavior against the
repaired acquisition contract.

Exit condition: a reattachable stale route reuses the retained browser, while
route occupancy and controller protection remain fail-closed.

### P130-F | Validate and install

Run focused source gates, cross-surface contract gates, provider-free runtime
acceptance, transactional workstation admission, and bounded installed-runtime
proof.

Exit condition: every acceptance criterion has current evidence bound to the
integrated commit and installed binary identity.

## Write surfaces

Primary source ownership:

- `cli/src/native/service_access.rs`;
- focused service-access and lifecycle fixtures;
- service access-plan contract schema;
- generated service-request and observability client surfaces;
- focused no-launch and client contract scripts;
- this plan and its validation note.

Conditional source ownership after active-lane reconciliation:

- `cli/src/native/remote_view/open/route_lifecycle.rs`;
- `cli/src/native/presentation_capacity.rs`;
- remote-view route tests;
- `README.md`, `cli/src/output.rs`, `skills/agent-browser/SKILL.md`, and
  `docs/src/app/` user-facing pages.

The conditional surfaces are currently dirty in the primary `main` worktree.
P130 must not overwrite or silently reproduce those edits.

## Validation

Use the repository Cargo safety wrapper for every compiling Cargo command.

Focused source gates:

```bash
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml runtime_lifecycle -- --test-threads=1
pnpm test:service-access-plan-no-launch
pnpm test:service-api-mcp-parity
pnpm test:service-client
pnpm test:dashboard-launcher-eligibility
```

Required Rust gates when `cli/src/` changes:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml
```

Documentation and selection gates:

```bash
pnpm validation:select -- --base <last-green-ref>
pnpm --dir docs build
git diff --check
```

Provider-free and installed acceptance must record the exact command, commit,
binary hash, runtime generation, browser ID, session route, owner generation,
process count, tab acquisition, tab release, and final resource census.

## Bounds

- two implementation attempts per behavioral seam before local replan;
- one source integration reconciliation with the active presentation lane;
- one closed-world remediation pass after focused review;
- one provider-free cooperative-transfer proof;
- one transactional candidate dry-run and one apply;
- one harmless installed Last30Days tab acquisition and release proof;
- one stale-route reattachment proof;
- checkpoint after every completed work unit or 90 minutes, whichever occurs
  first;
- active-agent concurrency is one for the critical source path.

## Hard stops

- Do not edit, stage, commit, stash, or reset the dirty primary `main`
  worktree.
- Do not hand-edit Service State, runtime-owner state, profile locks, route
  registries, or supervisor manifests.
- Do not kill a browser process or close another workload to satisfy a test.
- Do not launch a second Chrome process on a durable profile.
- Do not weaken owner-generation, canonical-profile, process, CDP, or target
  identity checks.
- Do not treat route availability as browser acquisition authority.
- Do not navigate to X, LinkedIn, Facebook, BILL, QBO, or another authenticated
  provider during qualification.
- Do not displace an active viewer or controller lease.
- Do not merge conditional presentation or documentation edits until the
  current active lane has a published checkpoint and its intent is reconciled.
- Do not claim installed acceptance from source tests or a development runtime.

## Initial control record

- State transition: `planned` to `source_implementation_active`.
- Acceptance state: all criteria open.
- Progress classification: `outcome_progress`.
- Source baseline: `0535131b61627a1bbff34773bb116e462334aafe`.
- Worktree custody: isolated hotfix worktree on
  `hotfix/access-plan-owner-reuse-coherence`.
- Current evidence: deterministic three-run no-launch contradiction, healthy
  browser, ready generation-14 owner, missing route hints, exclusive session,
  and separate stale-route presentation incident.
- Material blocker: conditional presentation and user-facing documentation
  files are dirty on `main`; P130-A through P130-D remain ready on disjoint
  surfaces.
- Next action: add the P130-A red regression through the public access-plan
  interface.

## 2026-08-24 planner checkpoint

- P130-A is complete. The transferred-owner fixture failed with
  `wait_for_profile_lease` before the repair and now returns exact reuse hints.
- P130-B is complete for the access-plan and command-delivery surfaces.
  Existing-tab compatibility now uses explicit acquisition constraints instead
  of inherited replacement posture. Replacement-ineligible owners and invalid
  explicit sessions fail closed before request queueing.
- `compatibleLiveBrowserCount` now reports operation-compatible browsers;
  `sameProfileLiveBrowserCount` retains the broader health observation.
- Focused evidence: 45 `service_access_plan` tests pass, including HTTP, MCP,
  CLI output, transferred-owner reuse, explicit-session expansion, lifecycle
  replacement blocking, and partial route-hint completion.
- Next action: validate cooperative-transfer projection and implement the
  generated-client plan consumer.
