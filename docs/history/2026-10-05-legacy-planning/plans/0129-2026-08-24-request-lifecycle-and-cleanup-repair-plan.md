# Plan 0129 | Request Delivery, Lifecycle Projection, And Cleanup Repair

Date: 2026-08-24

State: CLOSED

Execution state: `accepted_installed_and_cleaned`

Lane: P129

Branch: `main`

Target: `origin/main` at `b283b2a6f1347429356ebbc833adf452ccdb785b`

Authority: SOURCE, PROVIDER-FREE DEVELOPMENT, TRANSACTIONAL WORKSTATION
UPGRADE, AND REVIEWED RUNTIME CLEANUP

Depends on:

- accepted Plan 0128 runtime lifecycle hotfix collection;
- installed generation `0.28.0-6b461233692c-7e71e8fd473b`;
- current Service State, runtime lifecycle, resource, supervisor, and install
  doctor readback from 2026-08-24.

## Goal

Make dashboard service mutations at-most-once across ingress timeouts, make a
cold `tab_new` acquisition produce one authoritative requested target, expose
terminal replacement and cleanup authority through public service surfaces,
converge exact lifecycle evidence, classify runtime processes without false
unowned pressure, and reconcile historical terminal browser projections.

Build and transactionally install the accepted source while preserving active
production and development work. Then perform only reviewed, identity-proven
runtime cleanup through Agent Browser ownership.

## Current Defects

1. Dashboard ingress waits two seconds for ordinary first response bytes. A
   non-retry-safe `tab_new` and `service_browser_close` each returned HTTP 503
   with retry guidance after the selected backend had accepted and completed
   the operation.
2. The retried BILL proof showed a cold service launch can create an initial
   provider target and then create the requested `tab_new` target. Durable
   profile restoration also surfaced older targets that were not represented
   in the acquisition contract.
3. Access planning and resource projections can report no conflicting lane
   while a terminal lifecycle owner exists under the generic
   `session:dashboard-service-backend` identity.
4. One response reported `serviceTabHandle.cleanupPolicy=detach` while its
   shared acquisition reported `cleanupPolicy=client_tab`.
5. Current lifecycle projection has four `closing/owned` lanes and one
   `retained/owned` lane. `session:p0086-cookie-schema` includes terminal exit
   and profile-lock evidence while remaining `closing/owned`.
6. Resource pressure counts selected production, isolated development, and
   short-lived diagnostic Agent Browser processes as unowned by Service State.
7. Historical QBO state is split between degraded retained browser
   `session:plan0233-qbo` and terminal lifecycle
   `session:plan0233-qbo-owned`.

## Acceptance Criteria

1. A delayed selected backend cannot make a non-retry-safe service mutation
   return retryable failure after backend acceptance. One operation identity
   produces one job and one effect across response delay and client replay.
2. A cold `tab_new` request returns exactly one authoritative requested target.
   Initial and restored targets are either reused deliberately or modeled and
   excluded from duplicate requested effects.
3. Access plan, profile allocation, resources, and trace expose terminal
   replacement eligibility, logical owner identity, owner generation, cleanup
   state, and a typed blocking reason when replacement is not admissible.
4. Service tab handle and acquisition cleanup policy agree. The public release
   action returns a receipt that identifies the released handle, tab, lease,
   and retained-browser outcome.
5. Exact terminal evidence converges to `terminal/satisfied`. Every
   nonterminal cleanup obligation exposes a typed preservation or blocking
   reason rather than an unexplained degraded aggregate.
6. Resource classification recognizes the selected dashboard, runtime host,
   isolated development runtime, named supervisors, and transient diagnostics.
   Only evidence-proven unknown processes contribute to unowned pressure.
7. Historical retained browser identities correlate with their owned lifecycle
   identity. Reviewed pruning can remove the inert QBO projection without
   provider access or loss of its shutdown-failure evidence.
8. Focused Rust and client regressions, formatting, strict Clippy, selected
   contract fixtures, documentation validation, and diff hygiene pass.
9. A transactional workstation dry-run and apply select the exact reviewed
   candidate while preserving unrelated browsers, the development runtime,
   rollback generation, and authenticated operator journey.
10. Post-install provider-free acceptance proves delayed request delivery,
    exact-one tab acquisition and release, terminal replacement visibility,
    and corrected resource classification. Reviewed runtime cleanup leaves no
    accepted stale lane without a typed preservation reason.

## Execution Units

1. Add a delayed-backend ingress regression, make it red on false retryable
   failure, and repair non-retry-safe delivery and replay semantics.
2. Add a cold-acquisition regression, make it red on duplicate requested
   targets, and repair launch-target reuse plus restored-target modeling.
3. Align tab-handle release semantics and add a public release receipt
   regression.
4. Extend lifecycle and access-plan projections with exact replacement and
   cleanup evidence.
5. Repair evidence convergence, resource classification, and historical
   retained-browser correlation one vertical regression at a time.
6. Run selected validation, update required user-facing documentation, commit
   coherent checkpoints directly to `main`, and push.
7. Build the exact candidate and run one transactional installer dry-run and
   one admitted apply.
8. Run installed provider-free acceptance, then perform a reviewed cleanup
   dry-run and exact supported apply for eligible historical state.

## Bounds

- two implementation attempts per behavioral seam before local replan;
- one red-green cycle at a time rather than one broad speculative rewrite;
- one source reconciliation and one closed-world remediation pass;
- one transactional candidate dry-run and one apply after exact admission;
- one provider-free installed acceptance packet;
- one reviewed runtime cleanup pass after active-lane and ownership readback.

## Hard Stops

- Do not navigate to BILL, QBO, or another authenticated provider for source or
  installed acceptance.
- Do not retry a non-retry-safe request whose backend outcome is unknown.
- Do not kill processes, edit Service State, remove profile locks, or delete
  runtime state manually.
- Do not classify production, development, supervisor, or diagnostic runtime
  processes as cleanup targets merely because Service State lacks a browser
  row.
- Do not remove the historical supervisor manifest or QBO evidence without a
  reviewed supported-operation receipt.
- Do not upgrade while runtime census, rollback, operator journey, active
  browser ownership, or development-runtime identity is ambiguous.

## Initial Evidence

- source and remote main: `b1e8b31ea558cbbace1e2a6d480a64925ccda340`;
- installed binary SHA-256:
  `6b461233692c0b51f67e690b50c9bf5bbf1e180c1b784b2d67fb90fd1277fdd1`;
- runtime convergence: one dashboard, one runtime host, one executable
  generation, zero legacy daemons, operator journey ready;
- ingress failures completed as jobs
  `http-service-request-tab_new-4990bab4-34b4-4f42-8456-7127c7780777`
  and
  `http-service-request-service_browser_close-45db28c2-47d5-4a38-8042-d53528f8eda2`;
- BILL replacement generation 3 closed to `terminal/satisfied` with exact
  process-exit and profile-lock evidence;
- `install-acceptance-0232-c` process group `49753` currently contains 16
  processes and is protected by retained profile and display evidence;
- Graphiti group `agent_browser_main` was healthy but had no useful current
  P128/P129 recall, so repository and runtime evidence are authoritative.

## Closeout Evidence

- Source repair commits landed directly on `main`:
  `abcaf266`, `a0743268`, `071004be`, `23454cd8`, and `b283b2a6`.
- Fresh access plans now retain an explicit lane through the real CLI flag
  cleaning path. Installed release-mode readback returned
  `query.sessionName=bill-soylei` and
  `decision.serviceRequest.request.sessionName=bill-soylei` while preserving
  `launch_new_browser` as the no-conflict acquisition decision.
- Candidate binary SHA-256
  `c128349c482fc049b70fe5f3dbfeadd3a9336cdd3ad5f81731dc2cb6b3d5cd63`
  was built from `b283b2a6`, passed a no-mutation workstation dry run, and was
  selected as generation `0.28.0-c128349c482f-d9745dc2e128`.
- Accepted transaction
  `upgrade-db8bdb81-cf53-4df3-8264-31f95ab15a85` completed stable census,
  runtime transfer, presentation rebound, authenticated candidate proof,
  payload commit, workstation reconciliation, dashboard cutover, and
  supervisor rebound. Admission draining is false and all seven readiness
  axes are true.
- Candidate presentation used only `https://example.com/` through disposable
  handoff `r386851`. Its receipt bound the candidate deployment generation,
  exact browser, target, owner generation, display, route, and RDP provider.
  No BILL or QBO content was opened.
- Installed doctor passes with no issues. Runtime multiplicity is steady with
  one dashboard process, one runtime host, one executable generation, and zero
  legacy daemons. The runtime monitor is fresh and healthy.
- The runtime monitor safely pruned the processless degraded
  `session:plan0233-qbo` and historical Odollo browser projections while
  retaining their lifecycle evidence. The exact inactive
  `last30days-home-feed` supervisor was removed through
  `session supervisor remove`; unrelated units, profiles, browser storage, and
  Service State were preserved.
- The provider-neutral acceptance browser was closed through its transferred
  owner lane. Its expected close incident was resolved with an operator note.
  The pre-existing `remote-view-route:guacamole:2` incident remains active
  because it references historical BILL route evidence. Its linked display
  records remain diagnostic-retained and were not force-pruned.
- Validation passed: focused request-delivery, cold-tab, lifecycle, access-plan,
  handoff-race, CLI, MCP, generated-client, and release regressions; all 91
  workstation installer tests; all 42 service access-plan tests; complete
  service-client checks; route-confusion, API/MCP parity, client type,
  workstation host, source-free workstation install, and remote-view docs
  fixtures; Rust format and strict Clippy; and the full docs production build.
- Primary `main`, `origin/main`, and the reviewed source tree were reconciled
  to `b283b2a6` with a clean worktree. Active feature development was not
  overwritten or staged by this repair.
