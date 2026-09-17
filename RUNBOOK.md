# Runbook

Current index. [The September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md) preserves checkpoints through Turn 312; [Turn 313](RUNBOOK-history-2026-09-13-turn313.md), [Turns 314 through 320](RUNBOOK-history-2026-09-14-turn314-through-turn320.md), [P169 history through Turn 319](RUNBOOK-history-2026-09-14-p169-through-turn319.md), [superseded P186 checkpoints through Turn 328](RUNBOOK-history-2026-09-14-p186-through-turn328.md), [P194 through P196 checkpoints from Turns 340 through 343](RUNBOOK-history-2026-09-15-p194-through-turn343.md), and [Turns 323 through 368](RUNBOOK-history-2026-09-14-turn323-through-2026-09-16-turn368.md) remain separately preserved.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P206](docs/dev/plans/0206-2026-09-16-visual-multi-round-challenge-contract.md)
- [P209](docs/dev/plans/0209-2026-09-16-visual-provider-protocol.md)

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
