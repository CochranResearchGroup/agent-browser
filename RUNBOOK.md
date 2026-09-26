# Runbook

## Current P219 status | 2026-09-26

[Plan 0219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is `OPEN`. P218 version 45 is `CANCELLED` as superseded while incomplete.
The operator directed execution of Plan 0219 on September 26 and renewed the
blocked campaign with 1,000,000 additional tokens after the first P219 window
stopped at 545,502 tokens.
The full G01–G45 specification and P01–P19 prohibitions remain unchanged.

Custody remains on `platform/p211-simple-cold-upgrade`, draft PR #191, work
items #181/#183/#195. Successor planning began at
`6b3a41e265d1fd3da27431788e4373b66177225d`; execution incorporated the remote
policy capsule at `8bb9518eb78b37c22aaec0183348ee6c77adf0da`. The inherited worktree is retained.
No candidate, runtime, provider, or installed acceptance changed in this slice.

Acceptance: the inherited [coverage ledger](docs/dev/contracts/p218-grilling-contract-coverage.v1.json)
has zero pass, 28 partial, seven fail, and ten missing rows. M1A will
reconcile stale evidence descriptions without promoting tests to installed
acceptance. `541d7346` already removed the JSON ordinary-open coordinator and
fallback. SQLite manual-seeding dispatch already uses its dedicated launcher,
and both product Cargo manifests lack a direct Lease Authority dependency.
The remaining complete dependency closure still needs qualification.

M1A checkpoint: [the current authority closure](docs/dev/architecture/p219-m1a-current-authority-closure.v1.json)
reconciles six bounded blocker groups against current source. A new red detector
identified an unreachable compiled JSON manager-handoff adapter; that adapter
and its JSON-only tests are removed while six focused handoff tests pass in
`job-20260926T174302Z-3972e141d58c`. Formatting passes in
`job-20260926T174504Z-c8d8e5240533`. No coverage row is promoted. The next
causal batch removes the compiled keeper-bypassing `ManagerHandoffAuthority::Legacy`
variant and qualifies the affected host recovery fixtures.

That keeper-only batch now passes all eight journaled-open tests in
`job-20260926T175133Z-8e7342c18660`. One first run exposed two fixtures whose
route IDs still named the removed static inventory; the consolidated repair
bound them to the current keeper route and the exact rerun passed. M1A's
inventory is complete. M1B continues with stale-history availability, ordinary
open coalescing, restart behavior, and remaining command heartbeat coverage.
Strict workspace Clippy passes in `job-20260926T175308Z-6dc3a4cd5bae`.
G05 advances from missing to partial: malformed owner, cleanup and ambiguous
historical fields do not change ordinary manager translation
(`job-20260926T175454Z-a004f9e83c83`), and contradictory migration history
does not block valid profile opens (`job-20260926T175616Z-5f35b7c10690`).
Provider-free B04 qualification also passes: concurrent exact ordinary opens
share one execution permit and replay one operation and handoff
(`job-20260926T175808Z-5fb37dfa5612`), while waiting-only restart performs no
browser operation before exact client resume
(`job-20260926T175854Z-10e864191671`). G19, G31 and G40 remain partial until
their browser-host and installed-daemon boundaries are proven.
The B05 inventory confirms that ordinary page-effect commands centralize
success-only heartbeat publication in `execute_managed_command`. Failure,
execution-error, success and exact Alice/Bob isolation pass in
`job-20260926T180401Z-b2578ab7c2d2`; shared-browser command and cleanup
isolation passes in `job-20260926T180405Z-5f6a121c50fc`. B05 remains open for
generation-plus-operation fencing and browser-backed qualification.

Source evidence carried forward: `44961424` exact browser selection,
`680eb2df` synthetic navigation proof repair, and `1884c407` journaled
success-only heartbeat. The recorded 270 model tests, 31 host tests, format,
strict Clippy, docs, and contract checks retain their original scope. They were
not rerun as Rust or installed acceptance during successor writing.

Architecture readback: P09 is violated; P02/P03/P05/P12/P15/P16/P19 have
detector gaps; eleven prohibitions are unverified. The Service-model cut
guard passes. The old M0 dependency graph is historical, not current reachability.

Authority and effort: the latest P218 600,000-token window is exhausted. The
September 26 handoff recorded goal thread
`01a0da16-eee9-7511-99e6-ffc04d7b3cff` as `blocked` with
`tokensUsed=3,808,483`. This is a historical service readback, not a new
measurement or a sum of all windows. P218's earlier budget/attempt history
remains preserved. The renewed P219 allowance is 1,000,000 additional tokens
with 200,000 held for reconciliation, validation, evidence and custody. At
700,000 renewed tokens, implementation continues only if direct evidence shows
M2B remains reachable before the 800,000 implementation stop. The alternative
outcome stop is complete M1B and M2A plus one frozen isolated-development
candidate passing the installed M2B Alice/Bob checkpoint. M3 through M5 remain
open after that stop. The verified policy entrypoint is [the P219 M1B–M2B
capsule](docs/dev/policy-capsules/p219-m1b-m2b.md); broad policy rereads occur
only on one of its explicit triggers.

Current action: finish M1B's B02, B05 and B06 source blockers, join the minimum
M2A provider and live-viewer path, then freeze and qualify one M2B Alice/Bob
candidate. The early candidate cannot supply final proof for a different final
candidate.

Retained stops: no production/staging mutation, ingress publication, merge,
release, worktree removal, private-site acceptance, or automatic external
workflow retries. GitHub CI remains operator-disabled. Recheck development
identity, ingress binding, provider preflight, process ownership, and external
observer inputs before effects. September 26 stopped-container and blocked
provider observations are historical leads, not current runtime proof.

Validation: documentation links, policy wiring, active planning audit, goal
policy audit, coverage validator, and diff hygiene pass. Direct checks prove
all 45 requirements are mapped exactly once, the normative specification and
coverage ledger are unchanged, all 33 local plan/runbook links resolve, and
the archive preserves the previous runbook entries. Selection classifies this
change as docs; no compiler or runtime checks are required for this slice.

Progress classification: blocker reduction (current closure frozen and one
compiled JSON handoff fallback removed); no G-row acceptance advanced yet.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md) and [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)

## History

[The P218 archive](RUNBOOK-history-2026-09-26-through-p218.md) preserves the
previous runbook, Turns 418 through 450, P218 source checkpoints, failures,
prior stop instructions, and links to earlier archives. Read only the entry
needed for an active decision.
