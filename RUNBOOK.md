# Runbook

## Current P219 status | 2026-09-27

[Plan 0219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
version 4 is `OPEN`. The operator resumed the campaign under its 1,000,000-token
renewal; production, staging, public ingress, merge, release, and worktree
removal remain excluded. Custody stays on
`platform/p211-simple-cold-upgrade`, draft PR #191, work items #181/#183/#195.
HEAD is `67a17375`; the branch is 38 commits ahead of its remote and was clean
before this documentation reconciliation.

Current installed development identity is generation
`0.28.0-2423cfb064f2`, executable SHA-256
`2423cfb064f2f3ce4a110f197b5202b64535cfc6c674aabf6484027047fbd2e2`.
Commits `5cf1c10f` and `67a17375` added exact stale X lock and XRDP channel
socket reclamation. Privileged helper contract version 9 is installed. Focused
helper, installer, Rust, route, formatting, and strict workspace Clippy checks
for that repair batch passed. Production remained unchanged.

Three development routes are ready on displays `:58`, `:59`, and `:60`.
The fourth configured warm display is unavailable because older artifacts
outside exact repair custody still saturate the remaining allocation range.
Fresh read-only status therefore reports the development runtime ready but the
four-route presentation provider not ready. Do not broaden cleanup to foreign
or uncertain artifacts.

Authenticated access through an opaque `/remote-view/<handoff-id>` URL and real
synthetic browser pixels passed. The development admin credential was aligned
with the existing live credential at the operator's direction without recording
the secret; the live auth file stayed byte-identical. Automated pointer,
keyboard, and scroll effects were not established.

The current blocking product defect is named-session retention. Exact named
profile sessions 94 through 97 ended with `heartbeat_expired` roughly five
minutes after last activity. SQLite records
`sessionIdleTimeoutMs=300000`, and the current manager applies that finite
timeout to exact and disposable profiles alike. This violates G12/G36. G42 and
G45 still require exact-session refresh and separation of session heartbeat
from viewer heartbeat. The temporary keepalive was removed because it hid the
defect. A single operator report that the handoff was absent from SQLite was
not reproduced: the exact row existed and authenticated retry succeeded.

Acceptance remains partial:

| Requirement | Source or installed evidence | Current state | Missing proof |
| --- | --- | --- | --- |
| M1B authority closure | Source cuts and focused gates through `67a17375` | complete for this checkpoint | Final-candidate qualification remains later. |
| M2A provider and viewer | Three ready RDP routes, authenticated Guacamole, opaque handoff, real pixels | partial | Four-route target readiness and retained exact ownership for the unavailable slot. |
| M2B Alice/Bob | Named profile and durable handoff exercised | incomplete | Correct retention, joined Alice/Bob identities and effects, input, cleanup, restart recovery, and fresh process census on one candidate. |
| G12/G36 retention | Repeated SQLite `heartbeat_expired` terminals at about five minutes | fail | Named profiles must have no default time expiry; disposable expiry must remain bounded. |
| Goal control row | Objective is correct | stale | Goal service still says `blocked`, `tokensUsed=595442`, `timeUsedSeconds=1647`; no resume transition exists. |

Next bounded packet: repair named-profile retention at the service-model and
SQLite host/config seam. Prove an exact named session and the same opaque
handoff survive beyond the disposable idle window, while a disposable session
still expires; preserve explicit-close terminality, success-only exact-session
refresh, and restart-load behavior. Run focused model and host tests, formatting,
and strict workspace Clippy. Stop after a source-qualified custody commit and
evidence update. Do not publish another development candidate or replay M2B
without a separate freeze decision against the cumulative allowance. Do not
start M3, M4, or M5.

Nonblocking future UI direction: adopt the Guacamole interaction approach from
the sibling `../remote-view` project and make warning banners compact and
dismissible. This does not expand the retention repair packet.

Progress classification: blocker reduction and evidence reconciliation. M2A
and M2B remain incomplete; no G-row is promoted by this documentation slice.

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
