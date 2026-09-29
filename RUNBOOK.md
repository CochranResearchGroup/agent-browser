# Runbook

Current index. [The September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md) preserves checkpoints through Turn 312; [Turn 313](RUNBOOK-history-2026-09-13-turn313.md), [Turns 314 through 320](RUNBOOK-history-2026-09-14-turn314-through-turn320.md), [P169 history through Turn 319](RUNBOOK-history-2026-09-14-p169-through-turn319.md), [superseded P186 checkpoints through Turn 328](RUNBOOK-history-2026-09-14-p186-through-turn328.md), [P194 through P196 checkpoints from Turns 340 through 343](RUNBOOK-history-2026-09-15-p194-through-turn343.md), and [Turns 323 through 368](RUNBOOK-history-2026-09-14-turn323-through-2026-09-16-turn368.md) remain separately preserved.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)
- [P220](docs/dev/plans/0220-2026-09-28-remote-view-consumer-integration.md)

## Turn 402 | 2026-09-29

P220 Plan version 19 records the exact external custody seam on Remote View
issue #62. Remote View Milestone 5 currently launches applications under an
owned service/cgroup, while Plan 0015 leaves browser process, profile, CDP, and
recovery ownership with the consumer. Agent Browser requested a provider-neutral
contract for exact generation-bound allowlisted launch, process-start evidence
for CDP adoption, layered readiness, inspectable unknown outcomes, and release
fencing while the consumer browser remains live. It did not request embedding,
dynamic capacity, live effects, caller-supplied shell execution, or ownership
of Remote View internals. No browser, Remote View, installed, provider,
privilege, production, or release effect occurred.

## Turn 401 | 2026-09-29

P220 Plan version 18 prepares the separately gated installed-acceptance packet
without authorizing or performing effects. It fixes the target to two reviewed
Remote View desktops and two disposable development profiles; orders baseline,
single-association, many-to-many, recovery, exact-cleanup, and fresh-census
axes; binds required identities; and records hard stops plus forward-only
unknown-outcome recovery. The evidence table keeps source qualification,
installed proof, protected integration, and operator authorization distinct.
Remote View's installed application effect and browser/CDP adoption contract
remain the next external dependency. No browser, Remote View, installed,
provider, privilege, production, or release effect occurred.

## Turn 400 | 2026-09-29

P220 checkpoint `c1f5fe27` proves the existing explicit final-session browser
cleanup path directly. Closing the only logical session invokes one exact
browser close, issues no redundant tab-close effect, clears active browser,
session, and tab records, and preserves terminal histories. The focused
service-model test and workspace formatting pass. This resolves the stale plan
item for Agent Browser-owned final-session cleanup; it does not fabricate a
Remote View release. Joined presentation cleanup still requires authenticated
assignment, generation, route, and complete viewer-session evidence from the
missing runtime ingress. No browser, Remote View, installed, provider,
privilege, production, or release effect occurred.

## Turn 399 | 2026-09-29

P220 checkpoint `e36f7d42` adds additive, read-only Browser Runtime health to
Service status, the generated client type, and install doctor. It reports
redacted SQLite integrity and size, migration archive state, verified-backup
state, and current browser-launch admission. Missing or invalid storage
returns a stable failure code and never its path; status does not migrate the
database or create a backup. Focused available and unavailable Rust tests,
Service-status no-persistence coverage, generated-client checks, TypeScript,
observability helpers, cross-seam schema tests, API/MCP parity, install-doctor
provenance tests, P220 architecture and custody guards, documentation links,
remote-view docs contracts, formatting, strict workspace Clippy, and the
production docs build pass. A browser-launching collection smoke timed out
before status assertions and is not acceptance evidence. Its exact
`sc-24089` process tree and empty disposable directory were removed; no
matching residue remained. No production, installed, provider, privilege, or
release effect occurred.

## Turn 398 | 2026-09-29

P220 checkpoint `496c25f8` restores the provider-neutral browser-launch
resource admission selected from P219. The Browser Session worker now checks
available memory, profile-filesystem space, host PID capacity, and Agent
Browser root-process count immediately before `BrowserManager::launch`.
Resource pressure returns a typed error without a browser effect. Unsupported
Remote View desktop placement retains its exact earlier fail-closed error, so
the packet does not mask or invent the missing installed application-effect
and browser/CDP adoption contract. Both focused admission tests, the runtime
no-launch test, version sync, P220 architecture guard, P219 custody ledger,
formatting, strict workspace Clippy, and diff hygiene pass. No live Remote
View, browser, provider, install, privilege, production, or release effect
occurred.

## Turn 397 | 2026-09-29

P220 checkpoint `d3e31c6f` adds the provider-neutral operator-handoff
projection. An active retained public binding and a bounded path-safe Agent
Browser handoff ID produce exactly `handoffId` and
`/remote-view/<handoff-id>`. Released bindings and slash-bearing IDs fail
closed. A self-testing architecture rule rejects any route, desktop, provider,
display, or credential reference in the projection body. Focused retention
tests, the P220 architecture guard, formatting, and strict workspace Clippy
pass. Link construction does not claim viewer readiness or perform a runtime
effect.

## Turn 396 | 2026-09-29

P220 follow-up `fd4264ec` wraps public Remote View presentation bind and joined
release in immediate SQLite transactions over the complete Browser Session
aggregate. This closes the lost-update gap before runtime ingress exists. The
focused close/reopen test proves both active bind and released state persist;
the architecture guard and strict workspace Clippy pass. No caller-facing
mutation endpoint was added, so untrusted callers cannot forge provider
identity while the real Remote View runtime adapter remains unavailable.

## Turn 395 | 2026-09-29

P220 checkpoint `b47f7a0c` retains one validated Remote View public
registration, pool, assignment, placement, desktop generation, opaque route,
and viewer-session set against the exact Agent Browser browser, profile,
session, tab, and target. Binding is idempotent across viewer-list ordering and
rejects active identity reuse. Joined release mutates only when assignment,
desktop generation, route, and the complete viewer-session set match exactly.
The record survives a SQLite restart and contains no provider URL, display
number, credential, or provider-private state. The full 228-test service-model
unit suite and all integration tests, the focused CLI restart test, P220
architecture guard, formatting, and strict workspace Clippy pass. Runtime
invocation remains blocked on Remote View's missing installed application
effect and browser/CDP adoption contract. No live runtime effect occurred.

## Turn 394 | 2026-09-28

P220 checkpoint `d7c43915` exposes verified Browser Runtime SQLite backup
status and creation through `service runtime-backup status|create` and the
generic HTTP and MCP `service_request` transport. The generated client carries
typed status and manifest responses. Status is read-only; creation performs an
online integrity-checked backup and rotates at most one previous copy. Neither
surface restores data or includes the separate Service State store. Focused
Rust backup and contract tests, formatting, strict Clippy, generated-client
contract and type checks, request-client tests, API/MCP parity, the no-launch
contract smoke, P220 architecture guard, documentation links, and the
production documentation build pass. The umbrella service-client gate stops
at a pre-existing stale P157 source-literal oracle that disagrees with both
the checkpoint baseline and current provenance helper name. No browser,
provider, install, privilege, production, or release effect occurred.

## Turn 393 | 2026-09-28

P220 checkpoint `a2080250` adds a deterministic, self-testing architecture
guard for the successor path. It rejects Remote View provider-private types in
the service model, retired Guacamole, XRDP, route-keeper, or presentation-queue
authority in the supported P220 modules, local-display inference from public
desktop identity, persistence dependencies in the pure model crate, canonical
J3 checkpoint drift, and loss of strict private-field or joined-cleanup
validation. The guard, its four negative fixtures, the 63-file P219 custody
ledger, validation-selection suite, release-asset fixture, documentation links,
and diff hygiene pass. No runtime or provider effect occurred.

## Turn 392 | 2026-09-28

P220 checkpoint `64f75821` extracts the retained browser-recovery model without
presentation ownership. Exact-client resume and authenticated active-viewer
demand may admit a replacement only after the old browser is proven unusable;
dormant demand waits, unknown health fails closed, retry backoff and deadlines
are bounded, and every observed-live, failure, and success transition is fenced
by generation. The recovery registry commits transactionally in SQLite and
replays the admitted generation after restart. The complete service-model
package, focused browser-session tests, formatting, and strict workspace Clippy
pass. Runtime observation and replacement-launch wiring remain pending, and no
browser or provider effect occurred.

## Turn 391 | 2026-09-28

P220 checkpoint `ca3709e1` replaces the supported browser-session JSON path
with a bounded SQLite authority for session state and the profile catalog. A
one-time migration stages and validates the database before atomic publication,
archives all legacy inputs read-only, and treats any existing database as
authoritative rather than falling back on stale JSON. Online backup creation
verifies SQLite integrity and schema before publication, binds a manifest to
the exact digest and byte count, and retains one previous copy. Focused store
and session tests, both anomalous broad-suite tests under the supported runner,
formatting, and strict workspace Clippy pass. Operator-facing backup status and
creation remain to be adapted from P219.

Remote View `origin/main@30f3e37` has transport-reachable CLI, HTTP, and MCP
entrypoints, but its `ControlRuntime` still instantiates `ProviderFreeHost`,
`ProviderFreeApplicationEffect`, and `ProviderFreeControl`. This clears the
public contract wait but does not provide the installed application effect,
browser launch, or CDP adoption endpoint needed to replace Agent Browser's
placement-unavailable stop. Independent P220 extraction continues; no legacy
display inference or Agent Browser-owned presentation provider is restored.
No live Remote View, browser, provider, install, privilege, production, or
release effect occurred.

## Turn 390 | 2026-09-28

P220 source checkpoint `552c8162` consumes Remote View's strict public J3
registration, assignment, ready-placement, opaque-route, viewer-session status,
and joined-release records in `agent-browser-service-model`. Its source-bound
fixture proves two desktops, distinct placement and route identity, independent
desktop/mobile sessions, exact route-and-session retirement, and rejection of
browser-private state or partial cleanup. The complete service-model package
passes with 228 unit tests plus all integration tests; workspace formatting and
strict Clippy pass. This is provider-free wire-consumption evidence only. The
next packet must replace the CLI's placement-unavailable stop with an injected
Remote View adapter while preserving browser/profile/CDP authority and the
opaque route-ID versus durable operator-handoff distinction. No live Remote
View, browser, provider, install, privilege, production, or release effect
occurred.

## Turn 389 | 2026-09-28

Remote View Plan 0017 is closed from corrected canonical evidence. J2
`7298ae7f76626c61ba5086eb4f934699251009ce` and J3
`018d3d752f9d99805242f99c00034f0caabc53d1` publish registration, bounded
pools, exact assignment and placement, opaque route identity, layered viewer
status, truthful reacquisition, placement stop, and joined cleanup. Corrective
source `b7caa1f209f2d64cd400b1766c50389ab4dc5cb3` makes the operation matrix
reachable through real CLI, authenticated HTTP, and MCP adapters; governance
evidence is canonical at `eefddd2c862ce97bcf4387b2cc43dbbb2d9274eb`, and
Remote View `origin/main@30f3e37` reconciles the closure narrative. P220's
external J2/J3 wait is therefore cleared. Its next packet is Agent
Browser-owned provider-free consumption and replay, including replacement of
the temporary placement-unavailable boundary, retention mapping, exact cleanup,
and a handoff adapter that does not confuse Remote View's opaque `routeId` with
the operator-facing `/remote-view/<handoff-id>` URL. SQLite backup and recovery
authority, remaining admission and observability extraction, the architecture
guard, protected integration, and separately authorized installed acceptance
remain. Remote View's provider-free and production-entrypoint evidence does not
authorize an Agent Browser live effect.

## Turn 388 | 2026-09-28

P220 is active on `platform/p220-remote-view-consumer` from canonical
`origin/main@a3848e16`. Checkpoint `9a34e292` adds the first provider-free
Remote View F0 consumer boundary and proves two distinct Agent Browser-owned
browser/profile associations over distinct fixed desktop identities. The
source through `d6f390be` also extracts the first presentation-neutral
browser/session spine with independent SQLite state, restart reattachment,
idle reaping, focus, named-tab lifecycle, and addressed ordinary commands.
Desktop choice consumes Remote View UUID, route-label, generation, and
readiness observations; the legacy route inventory is not selection authority
and real placement fails closed until the public J2 contract exists. J1
lifecycle and durable operation status are source-bound to Remote View
`f674518e34fea346002c72c4adc3966b628d0b78`; exact release preserves desktop
UUID and generation, while tampered operation payload evidence fails closed.
Disposable browser retention now has count and regular-file byte ceilings,
oldest-first eviction, explicit protected-session fencing, and no pin or
promotion bypass. Exact URL history compacts deterministically into
restart-safe daily identity summaries before host persistence. The retired
Agent Browser viewer/controller tables are not reintroduced; Remote View J2
must later supply any presentation-derived protected-session evidence.
The preserved P219 branch is not merged wholesale;
its Agent Browser-owned XRDP/Guacamole presentation path is retired. Remote
View J2 and J3 contracts, SQLite backup and recovery extraction, protected
integration, and separately authorized installed acceptance remain. No live
Remote View, browser, provider, install, privilege, production, or release
effect occurred.

## Turn 387 | 2026-09-17

P216 source candidate `6ff7bc0d` and candidate-freeze receipt `3fdfc147`
entered the single protected integration path through PR #200. The canonical
provider-free Service State model now lives in `agent-browser-service-model`;
the CLI retains every filesystem, process, browser, runtime-owner, HTTP, MCP,
and platform effect. Complete local changed-surface qualification passes. The
build claim is limited to the measured cold pure-model loop, from 171.01
seconds to 4.08 seconds, not general CLI or workspace acceleration. Issue #178
and Plan 0216 close with the protected merge. P211 remains separate and must
reconcile the integrated model boundary before continuing overlapping urgent
bug-fix work. GitHub Actions remained disabled, and no browser, provider,
credential, install, runtime, staging, production, or release effect occurred.

## Turn 386 | 2026-09-17

P213 exact head `c8012bd0` merged through PR #196 as `7e56d9c7`; issue #194
closed and no GitHub Actions branch or merge-head run started. P214 is admitted
from that canonical baseline in the clean reassigned P213 worktree; no checkout
was created or removed. It owns only an effect-free desktop-services planner
that accounts every raw pointer move, down and up against the P212 permit
budget. P205 retains the root Cargo manifest and lockfile, P211 source remains
disjoint, and shared planning projections are an explicit reconciliation
overlap. Source checkpoint `f90ef7a7` implements canonical permit validation,
exact-budget interpolation, monotonic checked scheduling and deterministic
plan digests. All 18 desktop-services tests, all 58 challenge-control tests,
the strengthened architecture guard, workspace formatting, strict workspace
Clippy, documentation links, planning audit, selection and diff hygiene pass
locally. GitHub CI remains disabled and was not restored or run. Publication
and protected integration remain. No browser, capture, provider, credential,
CAPTCHA, route claim, desktop input, runtime or production effect occurred.

## Turn 385 | 2026-09-17

P212 exact head `dc20155e` merged through PR #193 as `ddae1897`. No GitHub
Actions branch or merge-head run started. P213 is admitted from that canonical
baseline in the clean reassigned P212 worktree; no checkout was created or
removed. It owns only a pure challenge-control adapter that proves a visual
intent is the current state-machine-authorized intent before mapping it to the
P212 desktop permit. P205 retains the root Cargo manifest and lockfile; P213
avoids both. P211's source remains disjoint and shared planning projections are
an explicit reconciliation overlap. Source checkpoint `1bad68e3` implements
the exact join and bounds permit expiry by the earlier evidence or visual
policy deadline. All 58 challenge-control tests, all 12 desktop-services tests,
the strengthened architecture guard, workspace formatting, strict workspace
Clippy, documentation links, planning audit, selection and diff hygiene pass
locally. GitHub CI remains disabled and was not restored or run. Publication
and protected integration remain. No browser, capture, provider, credential,
CAPTCHA, route-claim, desktop-input, runtime or production effect occurred.

## Turn 384 | 2026-09-17

P210 exact head `343a61b9` merged through PR #192 as `06972a5e`. P212 is
admitted from that canonical baseline in the clean reassigned worktree; no new
worktree was created. It owns only an effect-free desktop-services candidate
geometry and controller-authority contract plus provider-free fixtures. P211's
active cold-upgrade branch touches CLI shutdown and workstation routing, not
the P212 source surface; shared planning files are an explicit reconciliation
overlap. Source checkpoint `00f41715` now binds exact observation and ordered
candidate geometry to current controller authority and checked effect budgets.
All 12 desktop-services tests, the strengthened architecture guard, workspace
formatting, strict workspace Clippy, documentation links and diff hygiene pass
locally. GitHub CI remains disabled and was not restored or run. Publication
and protected integration remain. No browser, capture, provider, CAPTCHA,
desktop-input, runtime or production effect occurred.

## Turn 383 | 2026-09-17

P210 source checkpoint `0729b63d` adds the pure visual artifact and one-shot
injected transport adapter. Ten adapter fixtures prove exact payload custody,
deterministic digests, zero-call invalid input, explicit byte ceilings,
one-call transport and malformed-output failure, strict effect-smuggling
rejection, typed abstention, delayed response receipt time and raw-byte
redaction. Review repaired exact artifact/request expiry binding and separated
request time from transport receipt time. All 52 challenge-control tests, 10
adapter tests, both architecture guards, workspace formatting, strict
workspace Clippy and diff hygiene pass. GitHub CI remains disabled and was not
restored or run. Publication and protected integration remain. No browser,
image capture, real provider, credential, CAPTCHA, desktop-input, runtime or
production effect occurred.

## Turn 382 | 2026-09-17

P209 exact head `27cd5342` merged through PR #188 as `fb616aee`. P210 is
admitted from that exact canonical baseline on
`challenge/p210-visual-artifact-adapter` in the clean reassigned challenge
worktree. No new worktree was created. The baseline selector reports no
changed files and only diff hygiene. P210 owns a new pure visual-adapter crate,
repository-owned synthetic bytes and one injected fake transport; it owns no
real provider, network, browser, capture, credential, CAPTCHA, desktop-input,
runtime, production or CI effect.

## Turn 381 | 2026-09-17

P206 exact head `82e25624` merged through PR #180 as `d3f923a1`. P209 joined
that canonical result at `3ef2ad9e`; the only conflict was the active-lane
catalog, resolved by preserving the canonical CI-shutdown record and P209's
bounded entry. No P209 Rust source or Cargo dependency changed. The reconciled
challenge-control package passes all 52 tests, including 25 visual-round and
11 provider-protocol cases. GitHub CI remains disabled and was not restored or
run. P209 is ready for publication and protected integration. No provider,
image, browser, credential, CAPTCHA, desktop-input, runtime or production
effect occurred.

## Turn 380 | 2026-09-17

P209 merge `9df85b9b` joins corrected published P206 head `72eeea03`. The
combined dependency head passes all 52 challenge-control tests, including 25
visual-round and 11 provider-protocol cases, the crate architecture guard,
strict workspace Clippy, formatting and diff hygiene. P209 remains local and
unpushed until P206 PR #180 enters canonical `main`. No provider, browser,
CAPTCHA, credential, runtime, production or CI-dispatch effect occurred.

## Turn 380 | 2026-09-17

P206 joined operator-disabled-CI `main@f6d49f89` at `bb961c96`, P208's source
integration at `db987e4e`, and canonical `main@692f77c6` at `1c9ee159` after
P208 closed. The only conflicts were shared runbook projections, resolved by
retaining both plans' records. None of these main slices changes
challenge-control source or Cargo metadata, so the repaired
41-test source evidence at published head `72eeea03` remains reusable.
Conflict-affected policy wiring, documentation links, validation-selection,
P208 closeout fixtures, active planning audit and diff hygiene pass locally.
GitHub CI remains disabled by operator direction; no workflow was restored,
dispatched, retried or run. Reconciled publication and protected integration
remain. No browser, provider, CAPTCHA, credential, runtime or production
effect occurred.

## Turn 379 | 2026-09-17

P206 pre-merge review reproduced two budget-boundary defects: cumulative
selection arithmetic could saturate and admit an actual total above 255, and a
256-candidate selection returned a generic transition error rather than typed
round-budget intervention. Repair checkpoint `4813d385` replaces saturation
with widened and checked arithmetic. All 41 challenge-control tests, including
25 visual-round cases, the crate architecture guard, strict workspace Clippy,
formatting and diff hygiene pass. Corrected published head `72eeea03` requires
protected exact-head evaluation. No provider, browser, CAPTCHA, credential,
runtime or production effect occurred.

## Turn 378 | 2026-09-17

P209 checkpoint `f9987721` proves the response digest binds request, evidence,
candidate-set, capability, selected-candidate order, production-time and expiry
fields. All 50 challenge-control tests, strict workspace Clippy, formatting and
diff hygiene pass. No provider, browser, CAPTCHA, credential, runtime,
production or CI-dispatch effect occurred.

## Turn 377 | 2026-09-17

P209 checkpoint `89edafd5` proves request preparation rejects invalid policy,
mutated evidence, malformed artifact identity or digest, pre-observation and
expired preparation times, and over-budget execution plans before a provider
request exists. The complete 49-test challenge-control crate, strict workspace
Clippy, formatting and diff hygiene pass. No provider, browser, CAPTCHA,
credential, runtime, production or CI-dispatch effect occurred.

## Turn 376 | 2026-09-17

P209 checkpoint `e7250217` completes the provider-response temporal fixture:
pre-request, future-produced, produced-at-expiry, expired-at-adjudication,
beyond-request-expiry and request-expiry cases all fail closed as stale. The
complete 49-test challenge-control crate, strict workspace Clippy, formatting
and diff hygiene pass. No provider, browser, CAPTCHA, credential, runtime,
production or CI-dispatch effect occurred.

## Turn 375 | 2026-09-17

P209 review-hardening checkpoint `3aed9a4b` explicitly proves strict request
deserialization rejects coordinate, event-sequence, retry and instruction
smuggling plus nested artifact bytes and execution-plan repeat authority. All
49 challenge-control tests, strict workspace Clippy, formatting and diff
hygiene pass. The branch remains local and unpushed behind P206 PR #180; no
provider, image, browser, credential, CAPTCHA, desktop-input, runtime,
production or CI-dispatch effect occurred.

[Plan 0210](docs/dev/plans/0210-2026-09-17-visual-artifact-and-provider-invocation-adapter.md)
records the proposed W7-C artifact-custody and one-shot fake-provider adapter.
It is `PLANNED | NOT ADMITTED`; no branch, worktree or implementation has
started, and P209 canonical integration is its hard source-admission gate.

## Turn 374 | 2026-09-17

P209 local checkpoint `8ada33d5` is accepted. The pure protocol binds
prepared visual artifacts and P206 evidence into deterministic provider
requests, admits only candidate identities or typed abstention, and binds a
caller-owned, policy-checked execution budget before provider adjudication. It
rejects serialized coordinate, event, retry and instruction smuggling. All 48
challenge-control tests, strict workspace Clippy, formatting,
four architecture guards and 114 selector-expanded extracted-crate tests pass;
the exact P206 dependency reconciliation also passes the 48-test
challenge-control compartment, formatting and strict workspace Clippy.
The branch has locally joined published P206 head `99793061` and remains
unpushed until PR #180 enters `main` and the canonical checkpoint is reconciled.
No browser, provider, CAPTCHA, credential, runtime or production effect
occurred.

## Turn 373 | 2026-09-17

[Plan 0206](docs/dev/plans/0206-2026-09-16-visual-multi-round-challenge-contract.md)
is source-complete and acceptance-complete at `ac9f50a7` on
`challenge/p206-visual-round-contract`,
based on exact published P197 head `cd22a39f`. The pure challenge-control
contract keeps visual rounds inside one attempt, binds each selection and
effect receipt to fresh evidence and exact candidate identities, preserves
after-state continuity, and enforces per-round plus cumulative budgets. All 39
crate tests, including the complete 23-case visual-round matrix, the crate
architecture guard, formatting, strict workspace Clippy and diff hygiene pass.
P197 head `cd22a39f` passed all ordinary required checks and merged through PR
#157 as `c855fc33`. P206 joined that canonical checkpoint at tree-preserving
merge `5d6e3d57`, then joined merged P204 and current `main@f5e3f31b` at
`89bdfdbf`. Reconciled local validation passes the 39-test challenge-control
compartment, strict workspace Clippy and formatting, four architecture guards,
and 114 selector-expanded extracted-crate tests. Exact-head forge evaluation
and protected P206 integration remain. No browser, model provider, CAPTCHA,
desktop input, credential, installed runtime or production effect occurred.

## Turn 376 | 2026-09-17

[Plan 0208](docs/dev/plans/0208-2026-09-16-worktree-closeout-candidate-custody.md)
is closed. PR #182 merged validated source `c2ce2b35` into `main` as
`59928044`; issue #171 closed automatically. The durable advisory closeout
transaction, candidate archive locator, two-process serialization, explicit
retain/archive/discard dispositions, interrupted-effect recovery, policy, and
dormant Repository Tooling definition are integrated. GitHub CI remains
disabled. No real worktree, candidate, browser, provider, installed-runtime,
Service State, production, or release effect occurred.

## Turn 375 | 2026-09-17

[Plan 0208](docs/dev/plans/0208-2026-09-16-worktree-closeout-candidate-custody.md)
exact-head `8a010264` passed GitHub run `35227459177`, including Repository
Tooling and the stable Presubmit aggregate. Before integration, operator-directed
PRs #185 and #186 disabled GitHub CI and advanced `main` to `f6d49f89`. P208 is
rebased onto that tip without restoring an active workflow or trigger. The
dormant workflow retains Repository Tooling, the obsolete comprehensive-job
fixture expectation is removed, and the conflict-affected repository-tooling,
selector, aggregate, dormant-workflow, policy, planning, documentation-link,
and docs-build checks pass locally at `8c513789`. PR #182 integration remains.
No real worktree, candidate, browser, provider, installed-runtime, Service
State, production, or release effect occurred.

## Turn 373 | 2026-09-17

[Plan 0208](docs/dev/plans/0208-2026-09-16-worktree-closeout-candidate-custody.md)
is rebased onto `origin/main@f5e3f31b` after P204 merged through PR #179. The
provider-free closeout transaction, candidate archive locator, two-process
serialization, retain/archive/discard matrix, and interrupted-effect recovery
remain source-qualified. Checkpoint `9d34365d` adds package registration and a
dedicated `Repository Tooling` selector and CI lane; repository-tooling,
validation-control-plane, workflow syntax, and release-verifier fixtures pass
locally. P208 now owns the shared integration transition while preserving
P204's still-open issue #164 records. Exact-head PR #182 evaluation and
protected integration remain. No real worktree, candidate, browser, provider,
installed-runtime, Service State, production, or release effect occurred.

## Turn 374 | 2026-09-17

The operator clarified that CI itself should be disabled for now, not merely
the full-suite routes. Run `35228725370` was cancelled. The active
`.github/workflows/ci.yml` is removed and the reviewed path-selected workflow
is retained as `.github/workflows/ci.yml.disabled` at candidate `ca077d9e`,
which GitHub does not load.
There are no automatic or manual CI triggers. Re-enablement requires new
maintainer direction. The separate Lease Authority CI matrix is also retained
as `.github/workflows/lease-authority.yml.disabled` in candidate `ea254ecd`; no
active workflow has a push or pull-request trigger. Manual release and governed P158 operational
workflows remain separate and were not dispatched.

## Turn 373 | 2026-09-17

P204 initially interpreted operator direction as removing full CI while keeping
focused PR CI. The
bounded correction removes `main` push, scheduled, manual CI dispatch, and
commit-message qualification routes together with the comprehensive Rust and
slow platform jobs. Pull requests retain path-selected jobs, broad ordinary
fail-safe coverage, superseded-head cancellation, and the stable `Presubmit`
aggregate. Candidate `d9fede9d` passes the selector and workflow contract suite
and `actionlint`. The local comprehensive Rust command remains available outside
GitHub CI. Issue #164 remains useful for enforcing `Presubmit`, but it is no
longer a dependency for removing duplicate post-merge CI.

## Turn 372 | 2026-09-16

[Plan 0204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
has published source checkpoint `b481ab01`. CI run `35172965793` passed every
selected ordinary gate and the stable `Presubmit` aggregate; comprehensive and
platform qualification remained excluded. Superseded run `35170641311`
cancelled as designed. Failed run `35171646162` exposed same-target CLI test
binary replacement, and the corrected two-lane runner then passed. P204 is
reconciled with `main@c855fc33`; protected PR evaluation of the merge result
remains. The plan stays open for post-merge docs-only and narrow-Rust evidence,
an explicitly authorized comprehensive dispatch, and issue #164 branch-rule
enforcement before the temporary `main` fallback can be removed.

## Turn 371 | 2026-09-16

[Plan 0204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
has implementation checkpoint `7f6c7e2e`. The versioned classifier now drives
exact-head conditional jobs and the stable fail-closed `Presubmit` aggregate;
unknown and control-plane changes fail safe, while explicit comprehensive
qualification avoids duplicate focused Rust. Selector, aggregate, economics,
documentation-link, workflow, docs-build, policy, planning, and Challenge
Control compartment validation is green locally. One independent review and
bounded rework corrected every blocking finding. Organic PR receipts and an
explicitly authorized comprehensive dispatch remain pending. The `main`
fallback remains because issue #164 has not proved live required-check
enforcement. No workflow dispatch, branch-rule, browser, provider, credential,
installed-runtime, Service State, production, or release effect occurred.

## Turn 370 | 2026-09-16

### P204 admission

[Plan 0204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
is admitted from `main@2632e31c` for issue #174. The current selector is
advisory and has no versioned tier, fixed job outputs, unknown-impact fallback,
or direct fixtures; CI does not consume it and has no PR cancellation or stable
aggregate check. A docs-only probe is red on the missing contract. P204 owns the
selector, CI workflow, aggregate verifier, and provider-free fixtures. The
`main` fallback remains until issue #164 proves live `Presubmit` enforcement.
No workflow dispatch, branch-rule mutation, browser, provider, credential,
installed-runtime, Service State, production, or release effect is authorized.

### P197 integration

[Plan 0197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md)
joins canonical `main@2632e31c` after P202 protected integration and closeout.
The source merge is clean outside this runbook projection, retains the complete
P197 consumer-admission implementation and combined provider-free validation,
and removes P202 as an active dependency. Final plan and lane reconciliation,
branch publication at `4321961c` is complete; exact-head forge evaluation and
protected P197 integration remain. P197 now has durable remote custody and its
clean primary checkout can be released. W7-A is selected as the next
provider-free challenge packet but is not yet admitted. No browser, CAPTCHA,
provider, credential, installed-runtime, Service State, production, release, or
CI-policy effect occurred.

## Turn 369 | 2026-09-16

[Plan 0202](docs/dev/plans/0202-2026-09-16-abandoned-service-browser-retirement.md)
is closed. [PR #168](https://github.com/CochranResearchGroup/agent-browser/pull/168)
merged source head `6f099292` into `main` as `528f2ef0`; issue #103 closed.
Provider-free qualification and the isolated disposable real-browser acceptance
passed. CI run `35163527521` passed every ordinary gate at reviewed code head
`d7ceca98`; the final head added only integrated P203 closeout documentation,
and its in-flight Rust rerun was cancelled after the PR merged. P202 is removed
from the active-lane catalog and releases its shared surfaces to P197. No
browser, provider, credential, profile, installed-runtime, Service State,
production, or release effect occurred during integration or closeout.
