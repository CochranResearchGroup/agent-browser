# Plan 0130 | Access-plan owner reuse coherence

Date: 2026-08-24

State: CLOSED

Execution state: `installed_runtime_acceptance_complete`

Lane: P130

Branch: `hotfix/access-plan-owner-reuse-coherence`

Target: `main` through `dfc5b03a`

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

### P130-G | Remove the candidate presentation two-client trap

An authenticated durable-handoff resolution served by the staged dashboard
candidate must select that exact candidate when its response is ready and its
persisted receipt passes the existing owner, route, display, target, provider,
and deployment-generation checks. Selected or stale generations, converging
responses, failed responses, and unrelated actions must not select a
candidate.

Keep the explicit `dashboard ingress commit --handoff-id` command as a
recovery surface for a ready receipt that was persisted before automatic
selection completed. Do not accept client-supplied deployment generation or
weaken the independently authenticated candidate journey.

Exit condition: one authenticated candidate request performs resolution and
checked ingress selection without a second client, while every incomplete or
generation-mismatched path remains no-commit.

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
- one transactional candidate dry-run, one failed-safe blind apply, and one
  evidence-coordinated retry only after the old generation is proven selected;
- one harmless installed Last30Days tab acquisition and release proof;
- one stale-route reattachment proof;
- no third live apply in this repair slice; runtime installation resumes only
  after the automatic candidate-commit packet is integrated and separately
  authorized;
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

## 2026-08-24 transfer and client checkpoint

- P130-C is complete. The cooperative-transfer fixture proves the candidate
  inherits the exact browser, tab, exclusive profile lease, `close_tabs`
  cleanup policy, and refreshed tab-handle ownership while stale aliases are
  released.
- P130-D is complete for the maintained Agent Browser client. The existing
  `requestServiceTabFromAccessPlan` path now rejects unavailable plans,
  incomplete reuse hints, and caller overrides that contradict the planned
  browser or session route.
- Generated request-client types permit a null request only when the plan is
  unavailable and expose the acquisition blocker.
- Focused evidence: the transfer fixture and the complete
  `pnpm test:service-client` suite pass.
- The active presentation lane published commits `8964dbf8` and `543da5e7`;
  the primary `main` worktree is clean. P130-E can now reconcile against that
  checkpoint.

## 2026-08-24 presentation and contract checkpoint

- P130-E is complete by reconciliation with the published presentation lane.
  The retained-browser reattach fixture passes with one browser row, recovery
  admission and release, and no controller displacement. All 16 focused
  presentation-capacity tests pass.
- HTTP, MCP, schema, generated-client, dashboard eligibility, CLI help, README,
  skill, and docs-site surfaces now describe the same acquisition outcomes.
- The access-plan client rejects unavailable plans, incomplete reuse hints, and
  contradictory route overrides before posting.
- P130-F validation is active. The workstation-bound no-launch smoke remains
  deferred until the transactional candidate install because the installed
  runtime currently rejects legacy per-session daemon admission.

## 2026-08-24 integrated validation checkpoint

- The later presentation-ownership checkpoint `4587e8f4` was merged into the
  isolated hotfix without conflict. The combined head is `80e6653b`.
- The repository split Rust CI harness passes: 1,521 parallel-safe tests pass
  with 57 ignored, followed by every environment-mutating partition passing
  serially.
- Formatting, clippy with warnings denied, and `git diff --check` pass.
- The combined-head focused suites pass: 45 access-plan tests, 18
  presentation-capacity tests, and 77 service-health tests.
- Route-confusion, host-provision, fresh-workstation, Guacamole asset and
  durability, route-specific user sync, API/MCP parity, service-client,
  remote-view handoff documentation, and dashboard launcher gates pass.
- The docs site and dashboard production builds pass. The dashboard candidate
  therefore contains current static assets rather than the missing-build
  external-ingress state reported before this repair.
- The release-building workstation fixture was intentionally cancelled while
  a concurrent feature build held the repository-wide Cargo lane. It remains
  required after direct integration, together with the deferred installed
  service-access no-launch smoke.
- P130-F is source-integration ready. Remaining authority is the exact
  integrated candidate build, one transactional workstation dry-run and apply,
  install doctor, and the bounded harmless Last30Days runtime proof.

## 2026-08-24 transactional qualification checkpoint

- Candidate SHA-256 `93dc7492503e750585eb8fe713bc5b390daf445d55b7dc5f07b27153188582fb`
  passed the exact-binary source-free workstation fixture and a live dry-run
  with `success=true`, `state=planned`, and `mutated=false`.
- Transaction `upgrade-bf2e171c-5bbf-493d-9ee6-67f44b016dfd` transferred the
  cooperative Last30Days owners, preserved their presentations, and then
  rolled all three lanes back through receipted owner generations when no
  independently authenticated candidate-dashboard journey was committed
  during the five-minute window.
- The transaction terminal state is `failed_preserved_old_generation`, its
  stop reason is `candidate_dashboard_presentation_unproven`, and the selected
  installed binary remains SHA-256
  `c128349c482fc049b70fe5f3dbfeadd3a9336cdd3ad5f81731dc2cb6b3d5cd63`.
- This was an orchestration miss: the apply output is buffered while the
  required candidate revision, port, and handoff commit must be supplied by a
  second client. A blind retry is prohibited. One retry is authorized only
  with concurrent shadow-dashboard handoff resolution and
  `dashboard ingress commit --handoff-id` evidence.
- Main subsequently published `cd2967f9`, which hardens dashboard ingress
  against pressured service snapshots. Because it changes the failed proof
  surface, the final candidate must include it before the coordinated retry.

## 2026-08-24 candidate presentation replan

- Transaction `upgrade-12045b44-16d8-4d94-8994-30ef360e2839` reached the exact
  candidate dashboard but also ended as `failed_preserved_old_generation` with
  stop reason `candidate_dashboard_presentation_unproven`. Ingress returned to
  revision 318 with no staged candidate and the old installed SHA-256
  `c128349c482fc049b70fe5f3dbfeadd3a9336cdd3ad5f81731dc2cb6b3d5cd63`
  selected.
- The failure is a product orchestration defect. The authenticated candidate
  request stamps and persists the exact dashboard deployment generation, but
  the successful response does not select ingress. The installer waits while
  a second client must discover the hidden revision and run the explicit
  commit command.
- P130-G test-drives automatic checked selection from the authenticated ready
  response. Focused handoff evidence currently passes 121 tests. No third live
  apply is authorized in this source repair slice.

## 2026-08-24 candidate presentation source validation

- The ready-only response gate and durable-evidence selection path pass all 121
  handoff-filtered Rust tests. The complete 91-test workstation installer
  module passes, and clippy passes with warnings denied.
- The source-free workstation fixture passes against the sealed payload. The
  host-provision, fresh-workstation, Guacamole asset, route-user sync,
  durable-handoff documentation, docs production build, and dashboard handoff
  contract gates pass.
- PostgreSQL durability validation exposed a pre-existing fixture race: old
  retained dumps received current mtimes and could outrank the newly published
  backup. The fixture now pins historical artifacts to an old timestamp; the
  durability contract passes twice consecutively.
- This checkpoint proves the source packet and packaging behavior only. The
  installed runtime remains the preserved old generation, and P130-F installed
  acceptance remains open.

## 2026-08-24 P130-G integration checkpoint

- Automatic authenticated candidate selection is committed as `809d9f9f`.
  The deterministic PostgreSQL fixture repair is committed separately as
  `1368b85f`.
- Current `main` checkpoint `98141d07` changed only Plan 0124 documentation and
  was reconciled without source overlap. Integrated hotfix head is `9ae96c8a`.
- P130-G source and packaging acceptance is complete. P130-F remains open only
  for a future authorized transactional installation and bounded installed
  Last30Days proof; this slice will not perform a third apply.

## 2026-08-25 publication and runtime checkpoint

- Source integration is published on `origin/main` at `b8233deb`. The installed
  shared Agent Browser skill now exactly matches the repository skill at
  SHA-256 `5d8f826851e64fd199ea8320a1bce45ea25d7e7650cd4854839776bb9ef97574`.
- The selected binary remains the preserved old generation
  `0.28.0-c128349c482f-d9745dc2e128`, SHA-256
  `c128349c482fc049b70fe5f3dbfeadd3a9336cdd3ad5f81731dc2cb6b3d5cd63`.
  Dashboard ingress is ready at revision 318 with no staged candidate.
- Transaction `upgrade-12045b44-16d8-4d94-8994-30ef360e2839` remains
  `failed_preserved_old_generation` with stop reason
  `candidate_dashboard_presentation_unproven`.
- Last30Days remains healthy on browser
  `session:last30days-facebook--last30days-facebook`, PID 95745, session
  `handoff-17959ea3e226ee61`, ready owner generation 18, and durable handoff
  `r895695`. No temporary candidate-proof browser remains.

## 2026-08-25 exact candidate qualification

- Exact release candidate
  `/tmp/agent-browser-plan0130-target-687abdb3/release/agent-browser` has
  SHA-256 `09d98c8b14cbd28d5a1e1cbe289d13996408a6131438e89b3d2bdda952da373d`
  and reports version `0.28.0`.
- Its live-host dry-run passed with `success=true`, `state=planned`, and
  `mutated=false`. No transaction, ingress, browser, or runtime ownership state
  changed.
- The exact release candidate passes the complete source-free workstation
  fixture. The first fixture invocation exposed a release-pipe `SIGPIPE`; the
  fixture now captures up to 64 MiB while separately enforcing that reconcile
  JSON remains at or below 16 MiB. The rerun passed that payload-size contract.
- Candidate preparation is complete. A live apply remains outside this slice's
  bound and requires a separate explicit authorization for the third
  transactional attempt.

## 2026-08-25 provider-free selection proof

- The durable-handoff fixture now persists an isolated Service State, stages a
  candidate with a live runtime-manifest endpoint, executes the same
  authenticated candidate commit repository path used by the dashboard, and
  proves selected generation and presentation receipt advance together.
- The focused transition test passes, all 121 handoff-filtered tests pass, and
  clippy, formatting, and diff hygiene pass.
- This testability refactor changes Rust bytes without changing the public
  contract. The previously qualified candidate SHA-256 `09d98c8b...` is now
  stale and must not be installed. Build and qualify a new exact candidate
  before requesting live-apply authorization.

## 2026-08-25 replacement candidate qualification

- The replacement exact release candidate
  `/tmp/agent-browser-plan0130-target-79a83b89/release/agent-browser` has
  SHA-256 `2970534ac54b7226baec48690ec70b8dab7a0fe25a1cdc3000baaa5f666f5be9`
  and reports version `0.28.0`. It supersedes the stale `09d98c8b...`
  candidate, which remains prohibited from installation.
- Its live-host dry-run passed with `success=true`, `state=planned`, and
  `mutated=false`. No workstation transaction or runtime mutation occurred.
- The exact replacement binary passes the complete source-free workstation
  installation fixture.
- Source and candidate qualification are complete. The installed runtime
  remains unchanged. A third transactional apply remains outside this slice
  and requires separate explicit operator authorization for this exact SHA-256.

## 2026-08-25 accepted installation and retained-handle repair

- The operator explicitly authorized replacement candidate SHA-256
  `2970534ac54b7226baec48690ec70b8dab7a0fe25a1cdc3000baaa5f666f5be9`.
  Transaction `upgrade-51ba76bb-8d12-4c04-9d67-8b8cf7e5d05f`
  completed with terminal result `accepted`, selected generation
  `0.28.0-2970534ac54b-58e468ee69f5`, and no stop reason. The authenticated
  staged-dashboard proof resolved durable handoff `r895695` against the exact
  candidate and ingress selected the candidate generation.
- `agent-browser install doctor`, invoked outside a source checkout, passed
  with zero issues. Runtime convergence is `converged`; the selected dashboard
  and installed binary match the exact SHA-256; the census reports one
  executable generation, one dashboard process, one runtime host, no legacy
  daemons, no upgrade candidates, and no degraded session supervisors.
- The installed access plan correctly returned
  `reuse_existing_browser` for durable browser
  `session:last30days-facebook--last30days-facebook`, session
  `handoff-50e51527230ae122`, one healthy same-profile browser, and a ready
  generation-19 owner. One harmless `about:blank` tab was acquired through the
  authenticated stable dashboard as target
  `E93F0CD7724B06E4303D86643851EF69`; no provider page was opened.
- Release exposed a separate retained-handle identity defect. `tab_new`
  returned daemon alias `session:handoff-50e51527230ae122` and a null profile
  instead of the lifecycle owner's durable browser and retained profile.
  Service validation then rejected the alias because the retained tab belongs
  to the durable browser, while daemon routing rejected the corrected durable
  browser because it expected the alias. Both failures were fail-closed. The
  harmless tab remains open, and no duplicate tab or browser was created.
- Commit `dfc5b03a` repairs this contradiction by deriving retained tab handles
  from the exact runtime-owner binding, preserving the retained profile, and
  authorizing that identity across daemon follow-up validators while retaining
  the legacy session alias fallback when no binding exists. Focused handle,
  access-plan, formatting, clippy, and the canonical split Rust test harness
  pass. The split harness passed 1,585 parallel-safe tests plus every
  environment-mutating partition serially.
- The repaired exact release candidate
  `/tmp/agent-browser-plan0130-target-dfc5b03a/release/agent-browser` reports
  version `0.28.0` and SHA-256
  `80d87ab7be0d2b3a1c8241a6bc5865fe556a00dcf700c2489759ab1a947af97a`.
  Its live-host dry-run passed with `success=true`, `state=planned`, and
  `mutated=false`; the complete source-free workstation fixture also passed.
- The accepted installed runtime remains healthy at SHA-256 `2970534a...`.
  Installing the repaired `80d87ab7...` candidate is a new mutation and
  requires separate explicit operator authorization. After installation,
  acceptance must release the existing harmless target rather than acquire a
  second tab, then prove the browser process, route, owner, and profile lane
  were preserved.

## 2026-08-25 final installed-runtime acceptance

- The operator explicitly authorized exact candidate SHA-256
  `80d87ab7be0d2b3a1c8241a6bc5865fe556a00dcf700c2489759ab1a947af97a`.
  The first apply attempt failed safely at the five-minute candidate-dashboard
  presentation gate because the operator journey was not issued while the
  candidate was staged. Ingress revision 324 selected the prior generation
  with no candidate backend, and the installed binary remained `2970534a...`.
- The coordinated retry staged exact generation
  `0.28.0-80d87ab7be0d-5926db67f48a` at ingress revision 325. An authenticated
  `service_remote_view_handoff_resolve` request for durable handoff `r895695`
  returned success and automatically advanced ingress to ready revision 326
  with a generation-bound presentation receipt. No provider response body was
  read or retained.
- Transaction `upgrade-4828ff47-b400-4b2f-b875-32c2fb5c6009` completed with
  `success=true`, `complete=true`, `state=ready`, and `mutated=true`. Every
  installer phase through workstation reconciliation, dashboard management,
  and supervisor rebinding completed. The installed binary exactly matches
  SHA-256 `80d87ab7...`.
- The original harmless target `E93F0CD7724B06E4303D86643851EF69`
  was lawfully retired during transactional owner-transfer cleanup and was no
  longer live after installation. A replacement bounded proof therefore
  acquired one `about:blank` tab as target
  `5C38C377B1E89D0FD02CAC6CD68C4125` through the retained browser and current
  session `handoff-cf9000d7f4b26642`.
- The repaired runtime returned a valid `close_tabs` handle with durable
  browser ID `session:last30days-facebook--last30days-facebook`, profile ID
  `last30days-facebook`, and the current session route. Releasing that exact
  handle returned `released=true`, `tabReleased=true`,
  `browserProcessPreserved=true`, `sessionRoutePreserved=true`, and
  `closeBrowserOnRelease=false`. The released target is absent from the final
  tab inventory, while browser PID 95745 remains live.
- The first post-install doctor observed one historical runtime-monitor
  lock-collision receipt created while workstation reconciliation was active.
  Runtime multiplicity was already steady. After the bounded backoff expired,
  the configured runtime interlock completed successfully and replaced the
  receipt with `state=healthy`, zero consecutive failures, and no error.
- Final `agent-browser install doctor --json` exits zero with `success=true`
  and no issues. The payload is source-free and ready, runtime convergence is
  `converged`, dashboard state is ready and matches the exact installed SHA,
  and the census reports one executable generation, one dashboard process,
  one runtime host, zero legacy daemons, zero service candidates, zero
  duplicate-profile warnings, and zero degraded supervisors.
- P130-F and Plan 0130 are complete. The access plan, retained-owner command
  route, durable tab handle, exact release, transactional installer, dashboard
  presentation, and final health surfaces now agree in the installed runtime.
