# Runbook

Current index. [Turns 369 through 417](RUNBOOK-history-2026-09-16-turn369-through-2026-09-19-turn417.md) preserve the prior active runbook. [The September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md) preserves checkpoints through Turn 312; [Turn 313](RUNBOOK-history-2026-09-13-turn313.md), [Turns 314 through 320](RUNBOOK-history-2026-09-14-turn314-through-turn320.md), [P169 history through Turn 319](RUNBOOK-history-2026-09-14-p169-through-turn319.md), [superseded P186 checkpoints through Turn 328](RUNBOOK-history-2026-09-14-p186-through-turn328.md), [P194 through P196 checkpoints from Turns 340 through 343](RUNBOOK-history-2026-09-15-p194-through-turn343.md), and [Turns 323 through 368](RUNBOOK-history-2026-09-14-turn323-through-2026-09-16-turn368.md) remain separately preserved.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P211](docs/dev/plans/0211-2026-09-17-simple-cold-upgrade.md) and [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)

## Turn 419 | 2026-09-19

P211 source checkpoint `b0a82574` adds one proof-bound, provider-free
cold-process recovery seam for a retained route. Persistence cannot construct
the private predecessor-exit proof. The proof binds the complete prior ready
receipt and a strictly newer successor host generation. Recovery first persists
the exact disconnect, then prepares or replays one adoption. Compare-and-swap
conflicts, route rebound, and a foreign operation ID fail before connector
adoption. An interrupted connector call leaves replayable `Adopting` state, and
a missing exact receipt returns `Pending` instead of claiming readiness.

The first unsafe tracer was rejected because it manufactured disconnect proof,
could strand partial multi-route recovery, and blurred whole-authority startup.
The replacement passed four focused red-green cycles, all 42 route-keeper
tests, formatting, strict workspace Clippy, and two closed-world reviews. This
checkpoint does not construct predecessor process proof, rebase absent slots,
clear quarantine, implement configured connector adoption, or remove the
configured startup refusal. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred. Next:
bind exact predecessor process-exit evidence to this private proof without
enabling live provider recovery.

## Turn 418 | 2026-09-19

P211 implementation checkpoint `da9438a7` resolves Browser Session Manager
handoffs only through the current SQLite route-keeper authority. The keeper
catalog digest now binds the reviewed public operator origin with each exact
connection binding. Journaled opens validate their persisted slot, display,
handoff identity, and opaque URL before browser effects, then atomically commit
manager state and the handoff. Navigation validates the complete keeper-backed
route vector, and resolution reloads both the SQLite handoff registry and
current keeper authority before focus. Public responses retain only the durable
`/remote-view/<handoff-id>` URL and provider-neutral presentation semantics.

Qualification checkpoint `3922f138` adds the missing daemon-boundary regression
for SQLite lookup, current keeper reload, and failure before focus. The focused
test, formatting, strict workspace Clippy, the development-provider fixture,
the prior focused keeper, host, store, and model gates, and the complete
provider-free Rust runner pass. The comprehensive run completed in 818 seconds
with both lanes at zero. Source and remote match at `3922f138`. This remains
provider-free source qualification. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred.

The documentation closeout reconciles CLI help, README, the repository skill,
the dashboard and remote-view guides, inline comments, planning projections,
and the active-lane catalog. The remote-view documentation contract, link
checker, production docs build, six selected workstation and Guacamole
fixtures, planning audit, and goal audit pass. The shared installed skill was
not mutated; experimental guidance remains repository-scoped pending its
governed development-runtime publication step.

Plan 0211 remains `OPEN`. Cold-process adoption or recovery for durable
non-`Absent` keeper state is the next bounded implementation packet. Live
supervisor health and readiness, public capacity use, remaining SQLite domains,
shared Desktop Services control, the frozen development candidate, and the
isolated cold-start matrix remain open. Production and external ingress remain
excluded.
