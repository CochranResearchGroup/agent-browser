# Runbook

Current index. [The September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md) preserves checkpoints through Turn 312; [Turn 313](RUNBOOK-history-2026-09-13-turn313.md), [Turns 314 through 320](RUNBOOK-history-2026-09-14-turn314-through-turn320.md), [P169 history through Turn 319](RUNBOOK-history-2026-09-14-p169-through-turn319.md), [superseded P186 checkpoints through Turn 328](RUNBOOK-history-2026-09-14-p186-through-turn328.md), [P194 through P196 checkpoints from Turns 340 through 343](RUNBOOK-history-2026-09-15-p194-through-turn343.md), and [Turns 323 through 368](RUNBOOK-history-2026-09-14-turn323-through-2026-09-16-turn368.md) remain separately preserved.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P210](docs/dev/plans/0210-2026-09-17-visual-artifact-and-provider-invocation-adapter.md)

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
