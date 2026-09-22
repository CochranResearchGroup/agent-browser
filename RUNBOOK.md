# Runbook

Current index. [Turns 369 through 417](RUNBOOK-history-2026-09-16-turn369-through-2026-09-19-turn417.md) preserve the prior active runbook. [The September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md) preserves checkpoints through Turn 312; [Turn 313](RUNBOOK-history-2026-09-13-turn313.md), [Turns 314 through 320](RUNBOOK-history-2026-09-14-turn314-through-turn320.md), [P169 history through Turn 319](RUNBOOK-history-2026-09-14-p169-through-turn319.md), [superseded P186 checkpoints through Turn 328](RUNBOOK-history-2026-09-14-p186-through-turn328.md), [P194 through P196 checkpoints from Turns 340 through 343](RUNBOOK-history-2026-09-15-p194-through-turn343.md), and [Turns 323 through 368](RUNBOOK-history-2026-09-14-turn323-through-2026-09-16-turn368.md) remain separately preserved.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P211](docs/dev/plans/0211-2026-09-17-simple-cold-upgrade.md) and [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)

## Turn 426 | 2026-09-21

P211 version 54 source `a9595ee2` qualifies bounded capacity admission. New
browsers use an unused healthy display before sharing at the configured display
maximum and density. Existing browser reuse consumes no new allocation; lowered
limits preserve retained browsers and report over_target. Additional route
demand persists in SQLite and the keeper reconciles it. Demand-only concurrent
updates merge without ignoring phase or fence conflicts. Status and mandatory
docs distinguish retained assignment counts from a live process census.

Primary qualification: 299 Service Model tests, 770 native-other (57 ignored),
253 stream, 16 host, 25 store, two output tests, formatting, strict Clippy,
client suite and final types, API/MCP parity, final docs build, links, handoff
docs and repo-local planning audit pass. The
[plan amendment](docs/dev/plans/0211-2026-09-17-simple-cold-upgrade.md#september-21-capacity-admission-amendment)
records the source, receipts, worker integration and limits of the evidence.

This is outcome progress; Plan 0211 remains OPEN. About 116 elapsed minutes are
recorded since 02:46 UTC; older effort remains unknown. No installed or provider
effect occurred. Next: durable bounded queueing and public configuration, then
scale-in and remaining recovery, Desktop Services, persistence and isolated
cold-start acceptance. Production, ingress and release remain excluded. This
checkpoint is not full-suite or installed acceptance.

## Turn 425 | 2026-09-21

P211 checkpoint `45124f99` completes the provider-free recovery proof and
adoption-terminal packet. Exact predecessor-exit proof now reconstructs only
from retained `Ready`, `Degraded`, or deterministic interrupted `Adopting`
state. A current adoption terminal event persists `RecoveryFailed` while
retaining predecessor evidence; stale fences and predecessor occurrences do
not mutate the successor.

The combined 50-test route-keeper lane, formatting, strict workspace Clippy,
and diff check pass. Configured startup recovery and multi-route orchestration
remain the next bounded packet. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred.

## Turn 424 | 2026-09-21

P211 checkpoint `8f60434f` implements configured Guacamole adoption while
leaving runtime startup fail-closed. The adapter validates the exact durable
catalog and `Adopting` fence before provider access, retains one fresh tunnel
task for same-process replay, reobserves XRDP under the configured route user,
and emits the schema-v4 predecessor/current occurrence receipt.

The 18-test focused Guacamole keeper lane, formatting, strict workspace Clippy,
and diff check pass. The next bounded packet must reconstruct exact
predecessor-exit proof for retained `Ready`, `Degraded`, and interrupted
`Adopting` records and persist a failed-adoption terminal transition before
configured startup recovery is enabled. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred.

## Turn 423 | 2026-09-21

P211 checkpoint `11f606a4` completes the version 50 provider-free route
identity packet. Authority schema v4 separates the durable configured route
and exact XRDP witness from predecessor and successor Guacamole tunnel
occurrences. Historical v3 adoption receipts migrate deterministically from
their former same-UUID invariant. New adoption requires the exact catalog
digest, route user, complete XRDP witness, predecessor occurrence, and successor
fence. Stale predecessor disconnect evidence cannot degrade the successor.

The service-model unit and integration suites pass, as do the 44-test focused
CLI route-keeper lane, the SQLite v1-to-v4 migration test, formatting, and
strict workspace Clippy. Configured connector adoption joined to the exact
predecessor-exit capability is the next bounded packet. No provider, browser,
Service State, installed-runtime, production, ingress, or release effect
occurred.

## Turn 422 | 2026-09-21

P211 version 50 corrects a configured-adoption identity contradiction before
provider code is written. The configured Guacamole path can create only a new
websocket tunnel, whose protocol UUID is newly generated. The version 49 model
required a cold successor to preserve the predecessor UUID, so no truthful
configured adapter could satisfy it.

The corrected contract keeps the digest-fenced catalog connection and complete
XRDP ownership witness as durable route identity. A Guacamole UUID identifies
one transport occurrence under one keeper fence. A successor occurrence may be
published only after exact predecessor process exit, fresh catalog validation,
and unchanged XRDP witness readback. Provider-free model and stale-event tests
come first; configured connector and runtime-host recovery remain later gates.
No provider, browser, Service State, installed-runtime, production, ingress, or
release effect occurred.

## Turn 421 | 2026-09-21

P211 checkpoint `536d58a9` clears the broad provider-free support-lane gate.
The failure reproduced in six browser handoff and host fixtures whose v3
route-keeper authorities omitted the exact host-process claim for their active
generation. Production validation correctly failed closed. The repaired
fixtures register the complete process identity before starting a keeper.

The isolated workstation compartment passes all 215 tests, the browser
compartment passes 140 active tests with two browser-launch tests ignored,
strict workspace Clippy passes, and the complete provider-free runner passes
both lanes in 720 seconds. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred. Next:
implement the provider-free configured Guacamole connector adoption join while
preserving the exact predecessor-exit proof and retained route identity.

## Turn 420 | 2026-09-20

P211 source checkpoint `1ebfa757` advances the route-keeper authority to v3.
Every host generation now carries an append-only exact host-process claim: boot
epoch plus PID, start token, and executable identity. An active predecessor can
remain bound to its original claim while an absent slot is rebased for a newer
successor. A predecessor-exit proof is constructed only from those persisted
claims: a different boot proves exit, while a same-boot observation must prove
the exact predecessor missing or reused by an unrelated process. Ambiguous,
failed, or exact-live observations do not construct proof. Configured startup
registers its own exact claim, but continues to refuse cold recovery because the
configured Guacamole connector cannot adopt a retained primary task.

Diff hygiene, formatting, strict workspace Clippy, and all 44 focused
route-keeper tests pass. The broad provider-free runner ended nonzero after 815
seconds in its support lane; its slow workstation diagnostic rerun was stopped
without a failure diagnosis, so that broader receipt remains an explicit gate.
This is a source checkpoint only. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred. Next: make
the broad support-lane failure reproducible or clear it, then design the
separate configured connector-adoption and runtime-evidence joins.

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
