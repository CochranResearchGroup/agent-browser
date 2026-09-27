# Runbook

## Current P219 status | 2026-09-27

[Plan 0219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
version 6 is `OPEN`. The operator resumed the campaign under its 1,000,000-token
renewal; production, staging, public ingress, merge, release, and worktree
removal remain excluded. Custody stays on
`platform/p211-simple-cold-upgrade`, draft PR #191, work items #181/#183/#195.
The frozen candidate source checkpoint is `d26a25b9`; the named-retention repair
is `d7ec8d2f`. Reconcile the live branch divergence at turn closeout.

Current installed development identity is generation
`0.28.0-9502ef082e45`, executable SHA-256
`9502ef082e45862674821af690af2a5f033c0109f529c534664d1fbc925422b0`.
The optimized build passed in `job-20260927T015043Z-0ed4e4be3483`, development
installation passed, and the three-iteration browser-launch smoke passed.
Production remained unchanged and the development skill is current.

Provider plan, stage, and preflight passed with the exact validated ingress
binding. Apply receipt `apply-1790473069666-74987.json` then quarantined after
runtime-owned warm routes timed out. Fresh readback found only route one live on
`:58`; routes two through four were absent. The installed helper version 9
omitted the primary `xrdp_chansrv_socket_<display>` from exact reclamation,
leaving stale runtime-owned artifacts that saturated allocation.

The omission is repaired at `e79aa713`, the explicit
`reclaimsPrimaryChansrvSocket` contract at `4da14475`, and installer convergence
at `d26a25b9`. The red-then-green helper test and selector-required workstation
fixtures pass. Focused Rust contract tests, formatting, and strict workspace
Clippy pass. The shared helper remains version 9 because its replacement needs
interactive sudo; one attempt using the operator-designated credential file
was rejected and was not retried. The provider remains quarantined and stopped.

Authenticated access through an opaque `/remote-view/<handoff-id>` URL and real
synthetic browser pixels passed. The development admin credential was aligned
with the existing live credential at the operator's direction without recording
the secret; the live auth file stayed byte-identical. Automated pointer,
keyboard, and scroll effects were not established.

The named-session retention defect is repaired and source-qualified at
`d7ec8d2f`. The manager now applies `sessionIdleTimeoutMs` only to
manager-allocated disposable profiles. Exact named-profile sessions and their
opaque handoffs have no default idle expiry, while explicit close remains
terminal. Host restart normalizes and persists legacy finite named-session
deadlines. The installed generation still predates this repair, so G12/G36 are
not accepted at the installed boundary. G42 and G45 still require installed
exact-session refresh and separation of session heartbeat from viewer
heartbeat. A single operator report that the handoff was absent from SQLite
was not reproduced: the exact row existed and authenticated retry succeeded.

Acceptance remains partial:

| Requirement | Source or installed evidence | Current state | Missing proof |
| --- | --- | --- | --- |
| M1B authority closure | Source cuts and focused gates through `d7ec8d2f`; candidate `d26a25b9` installed | complete for this checkpoint | Installed lifecycle acceptance remains in M2B. |
| M2A provider and viewer | Historical authenticated Guacamole, opaque handoff, and real pixels; current apply quarantined | partial | Install helper version 10, recover four routes, and revalidate pixels and input. |
| M2B Alice/Bob | Named profile and durable handoff exercised | incomplete | Correct retention, joined Alice/Bob identities and effects, input, cleanup, restart recovery, and fresh process census on one candidate. |
| G12/G36 retention | Source regressions prove retained named handoffs, bounded disposable expiry, and restart normalization at `d7ec8d2f` | partial | Installed retention and same-handoff replay on a frozen candidate. |
| Goal control row | Thread `01a0dfee-e1ab-79e0-b34b-b9fa80a58d0c` is `active` | current | Its Plan 211 wording routes through active successor Plan 0219; checkpoint usage was `tokensUsed=471417`, `timeUsedSeconds=2712`. |

The bounded source repair is complete. Its red regression and green focused
receipts, the complete 271-test Service Model lane, 32 host/navigation tests,
formatting, and strict workspace Clippy are recorded in Plan 0219. The
selector-required workstation and Guacamole fixtures, docs build,
remote-view documentation contract, architecture report, and coverage-ledger
validator also pass. Four unrelated `browser_session_authority` failures remain
in a broader name-filtered Rust run and do not invalidate the independently
green changed surfaces. The development skill is synchronized with the
candidate; the shared production skill and production runtime remain unchanged.

Next gate: install the version 10 privileged helper through interactive sudo,
verify `reclaimsPrimaryChansrvSocket=true`, and rerun the exact development
provider recovery sequence. Do not retry the rejected credential file or
broaden cleanup. After four-route readiness, replay installed named retention
and the remaining M2B Alice/Bob workflow. Do not start M3, M4, or M5.

Nonblocking future UI direction: adopt the Guacamole interaction approach from
the sibling `../remote-view` project and make warning banners compact and
dismissible. This does not expand the retention repair packet.

Progress classification: frozen development candidate plus exact provider
blocker removal in source. M2A and M2B remain incomplete; no G-row is promoted
to installed pass by this checkpoint.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md) and [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)

## History

[The P218 archive](RUNBOOK-history-2026-09-26-through-p218.md) preserves the
previous runbook, Turns 418 through 450, P218 source checkpoints, failures,
prior stop instructions, and links to earlier archives. Plan 0219 version 4
preserves the superseded September 26 M2A checkpoint and its receipt.
