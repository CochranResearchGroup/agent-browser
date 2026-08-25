# Plan 0132 | Terminal-owner supersession route coherence

Date: 2026-08-25

State: CLOSED

Execution state: `installed_runtime_accepted`

Lane: P132

Branch: `hotfix/terminal-owner-supersede-executable`

Target: `main` through `481d319c`

Integration method: direct rebase and push after active-lane reconciliation

Authority: SOURCE, PROVIDER-FREE VALIDATION, CANDIDATE QUALIFICATION, AND
TRANSACTIONAL WORKSTATION HOTFIX AFTER EXACT-CANDIDATE AUTHORIZATION

## Goal

Make an access-plan-advertised `supersede_terminal_owner` launch executable.
The copied service request must carry the exact collision-free replacement
session route authorized by the terminal owner, and an explicit caller route
must be accepted only when it names that same terminal replacement route.

## Incident

Books Receipts Plan 0232 version 87 observed Agent Browser generation
`0.28.0-80d87ab7be0d-5926db67f48a` advertise terminal replacement for profile
`bill-soylei`. The copied `tab_new` request omitted `sessionName` and failed
twice with `runtime_lifecycle_terminal_replacement_rejected`. Supplying the
terminal owner's route explicitly produced `explicit_session_route_invalid`.
Both launches cleaned up fully, and no BILL browser, tab, holder, lease, or
provider effect remained.

Current source and runtime evidence show one terminal, cleanup-satisfied BILL
owner. Launch admission correctly rejects a replacement logical browser ID
that collides with another lifecycle record. The access plan neither projects
the terminal owner's daemon session route into its copied request nor treats
that exact route as lawful fresh-launch identity.

## Acceptance criteria

1. A provider-free fixture reproduces a terminal owner whose copied access-plan
   request omits `sessionName` and would select a colliding default identity.
2. An eligible terminal replacement exposes its exact replacement session and
   logical browser identities from the same owner and lifecycle record.
3. The copied `tab_new` request includes that exact session route and remains
   `available=true`.
4. An explicit session equal to the terminal owner's route is accepted as a
   fresh replacement launch even when no live service session exists.
5. An unrelated, ambiguous, live-incompatible, or owner-inconsistent explicit
   route remains fail-closed as `explicit_session_route_invalid`.
6. Launch admission continues rejecting logical-browser collisions with other
   profiles and same-profile lifecycle conflicts that are not the exact
   superseded terminal record.
7. HTTP, MCP, CLI, generated-client, schema, skill, README, help, and docs-site
   surfaces remain aligned if the public access-plan shape changes.
8. Focused lifecycle and access-plan tests, formatting, clippy, canonical Rust
   tests, and source-free workstation packaging pass.
9. Any runtime installation uses a separately authorized exact candidate SHA
   through the transactional workstation installer and preserves all browsers.
10. Installed acceptance proves one harmless BILL-root acquisition and release
    through the broker before Books Receipts retries private inspection.

## Work units

### P132-A | Freeze the contradiction

Add access-plan tests for omitted and exact explicit terminal replacement
routes, plus a lifecycle test proving the resulting logical identity activates
the next owner generation without weakening collision rejection.

### P132-B | Unify planning and launch identity

Derive replacement route hints from the exact current owner and matching
terminal lifecycle record. Copy the session route into launch requests and
recognize the same route as lawful explicit replacement authority.

### P132-C | Align public surfaces

Update every required user and agent documentation surface if new public
fields are added. Keep generated clients and schemas consistent with the
access-plan contract.

### P132-D | Validate, integrate, and qualify

Run focused and broad source gates in this isolated worktree, reconcile current
`origin/main`, push coherent commits directly to `main`, build an isolated
release candidate, and stop at its exact transactional installation gate.

## Bounds and hard stops

- Two implementation attempts per behavioral seam before local replan.
- One broad validation pass and one closed-world remediation pass.
- Checkpoint at each completed work unit or 90 minutes.
- Do not open BILL, QBO, or any authenticated provider during source work.
- Do not retry Books Receipts Plan 0232 from this source worktree.
- Do not edit runtime owner, lifecycle, session, browser, or profile state.
- Do not launch a duplicate profile process, kill a process, run GC, or bypass
  the service broker.
- Do not build or edit in the primary worktree or Plan 0131 worktree.
- Do not install a changed binary without explicit authorization for its exact
  SHA-256.

## Initial control record

- State transition: `planned` to `provider_free_regression_active`.
- Acceptance state: all criteria open.
- Progress classification: `blocker_reduction`.
- Source baseline: `481d319c985492cd1429a3ff684a92f85a0e0cf5`.
- Worktree custody: isolated
  `hotfix/terminal-owner-supersede-executable` worktree.
- Current evidence: Books Receipts commit `9b5c4e5`, current no-launch access
  plan, exact terminal BILL owner generation 4, and source trace from
  `lifecycle_replacement_decision` through `service_request_decision` to
  `register_managed_lane` and `ActivateTerminalReplacement`.
- Material blocker: the copied request and explicit-session validator do not
  share the terminal owner's replacement route identity.
- Next action: add the provider-free red regression before implementation.

## 2026-08-25 source implementation checkpoint

- The public-interface regression now covers an omitted terminal replacement
  session, the exact explicit terminal route, copied-request normalization,
  and the existing invalid explicit-session cases.
- The access plan derives replacement browser and session identities only from
  one exact owner and matching cleanup-satisfied terminal lifecycle record.
  Inconsistent owner, generation, browser, or daemon-route evidence makes
  replacement ineligible.
- Copied fresh-launch requests now carry the exact terminal owner session.
  Request normalization accepts a session-only route only when it equals the
  plan's eligible terminal replacement session; all other incomplete route
  hints remain fail-closed.
- CLI help, README, the Agent Browser skill, and the commands and service-mode
  docs pages describe the same terminal replacement contract.
- Provider-free completed gates: API/MCP parity, generated-client drift, client
  type coverage, managed-profile flow, service-client examples, remote-view
  handoff documentation, docs production build, planning audit, Rust source
  formatting through `rustfmt`, and diff hygiene.
- Rust compilation and tests have not run. The required Cargo wrapper reports
  `swap_pressure` with zero active claims because WSL has about 1.7 GiB free
  swap against the repository's 2 GiB minimum. The waiting test process was
  cancelled without starting Cargo. The guard was not bypassed or weakened.
- State transition: `provider_free_regression_active` to
  `source_implemented_validation_blocked_wsl_swap_pressure`.
- Acceptance state: source and documentation implemented; Rust validation,
  integration, candidate qualification, and installed acceptance remain open.
- Progress classification: `blocker_reduction`.
- Material blocker: WSL Cargo admission requires swap recovery.
- Next action: reboot WSL or otherwise restore at least 2 GiB free swap, then
  run the focused access-plan test, formatting, clippy, and canonical Rust
  suite before committing or publishing the hotfix.

## 2026-08-25 Rust validation checkpoint

- The maintainer explicitly authorized bypassing the WSL free-swap admission
  threshold for this hotfix, running Rust validation, integrating the change,
  qualifying the exact resulting binary, and installing it. Cargo continued
  to use the repository lock, four-job limit, and user-systemd memory cgroup.
- Focused access-plan coverage passed: 45 tests, including the terminal-owner
  replacement regression. Focused runtime-lifecycle coverage passed: 14
  tests, including exact replacement and collision rejection.
- `cargo fmt --check`, clippy with warnings denied, diff hygiene, API/MCP
  parity, the complete service-client suite, remote-view handoff docs, and the
  docs production build passed.
- The first canonical Rust run exposed one pre-existing stale structural test
  anchor. The gate had moved to `admit_default_action_effect`, while the test
  still searched for the removed inline `runtime_owner_binding.is_none()`
  expression. The assertion now follows the installed gate and again proves
  that it precedes desktop effects, stream broadcast, and browser recovery.
- The complete canonical Rust validation then passed: 1,528 parallel-safe
  unit tests, the integration-test partitions, and every serial
  environment-mutating partition completed with zero failures.
- State transition: `source_implemented_validation_blocked_wsl_swap_pressure`
  to `source_validated_integration_ready`.
- Acceptance state: criteria 1 through 8 complete. Integration, exact binary
  qualification, transactional installation, and installed acceptance remain.
- Progress classification: `acceptance_advance`.
- Next action: reconcile current `origin/main`, integrate the coherent hotfix,
  then build and qualify one exact candidate SHA before transactional install.

## 2026-08-25 integration and installed-runtime acceptance

- Source commit `93b355baee7dcf4f4fc7f18979bbdb95bbfd09cb` was pushed
  directly to `origin/main` from the isolated Plan 0132 worktree after a fresh
  remote readback proved no intervening mainline commits.
- The dashboard production payload built successfully. The isolated release
  candidate reports version `0.28.0` and SHA-256
  `05d9da26035e0e86b55d6b2beaed25ae6dfe45ee6eeb0aa14362ce4ec08b0d10`.
  Its source-free workstation fixture and live-host transactional dry run both
  passed against that exact file.
- The first transactional apply failed safely at candidate presentation. The
  existing durable handoff was ready, but its Guacamole route was orphaned and
  resolution reported `runtime_handoff_orphan_owner_present`. Transaction
  `upgrade-9aa8a12c-39a1-484a-b3ba-5fc7da1b65fb` preserved the old generation,
  and exact installed-binary and ingress readbacks confirmed no partial swap.
- A normal service reconciliation refreshed the route to the current owner
  session. Resolving the same opaque handoff then returned `resolved=true` and
  `status=ready`, without browser replacement, provider navigation, or page
  inspection.
- The coordinated retry staged generation
  `0.28.0-05d9da26035e-7fa3fbcb7248`. Authenticated candidate resolution of
  the same durable handoff committed presentation generation 7. Transaction
  `upgrade-3a9d3ace-cd02-48aa-851d-f1452c0832f5` completed with
  `success=true`, `complete=true`, `state=ready`, and `mutated=true`.
- The installed binary and qualified candidate both hash to exact SHA-256
  `05d9da26035e0e86b55d6b2beaed25ae6dfe45ee6eeb0aa14362ce4ec08b0d10`.
  Final doctor exits zero with no issues, converged runtime state, one selected
  executable generation, one dashboard process, one runtime host, zero legacy
  daemons, and a healthy runtime monitor with zero consecutive failures.
- The installed no-launch BILL-profile access plan exposes
  `replacementSessionName=bill-soylei`, copies `sessionName=bill-soylei`, and
  authorizes `supersede_terminal_owner`. One provider-free `about:blank`
  request succeeded as job
  `http-service-request-tab_new-720331d0-d416-4283-a6aa-60bac0f5d9b8`, advanced
  the terminal owner from generation 4 to ready generation 5, and acquired
  exact target `515E783FDA7998B5AAA42DBF243DAECF` on
  `session:bill-soylei`.
- Chrome restored retained profile tabs during the cold profile launch,
  including old BILL login targets. No page content was inspected and no UI,
  login, transaction, or provider mutation was attempted. Releasing the exact
  requested `about:blank` handle returned `released=true`,
  `tabReleased=true`, `browserProcessPreserved=true`, and
  `sessionRoutePreserved=true`; the released target is absent afterward.
- The final access plan now returns `reuse_existing_browser` with exact
  `browserId=session:bill-soylei` and `sessionName=bill-soylei`, and
  `blockedByLifecycleOwner=false`. Books Receipts may retry through that copied
  broker route without an Agent Browser upgrade.
- Acceptance state: criteria 1 through 9 complete. Criterion 10 is complete in
  its provider-free form: exact BILL-profile acquisition, release, and
  post-release reuse were proven without intentionally opening BILL.
- Progress classification: `objective_complete`.
