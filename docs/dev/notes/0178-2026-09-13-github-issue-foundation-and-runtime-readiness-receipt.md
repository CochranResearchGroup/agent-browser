# GitHub Issue Foundation And Runtime Readiness Receipt

Date: 2026-09-13

Product lane: PL-PLATFORM

Governing plan: Plan 0177

## GitHub Foundation

At `2026-09-13T15:05:17-05:00`, authenticated actor `ecochran76` read back
`ADMIN` capability on the owned fork `CochranResearchGroup/agent-browser`.
Issues changed from disabled to enabled. The repository remained unarchived and
identified as a fork. Neither a repository security policy nor private
vulnerability reporting was available, so the registry records no security
route and normal issues remain prohibited for vulnerability detail.

The coordinator created 13 missing labels for the five product lanes, four
active states, governance and operational-gate types, and live-effect posture.
Existing `bug`, `enhancement`, and `documentation` labels were reused. Every
label was read back by exact name, color, and description.

Each issue below was duplicate-searched by its stable idempotency marker before
creation and read back from the exact repository afterward:

| Issue | Lane and state | Outcome |
| --- | --- | --- |
| [#65](https://github.com/CochranResearchGroup/agent-browser/issues/65) | PL-PLATFORM, IN_PROGRESS | Plan 0177 governance and repository readiness |
| [#66](https://github.com/CochranResearchGroup/agent-browser/issues/66) | PL-CHALLENGE, BLOCKED | Plan 0169 Turnstile challenge qualification |
| [#67](https://github.com/CochranResearchGroup/agent-browser/issues/67) | PL-BUGFIX, READY | Unauthenticated access-plan profile selection |
| [#68](https://github.com/CochranResearchGroup/agent-browser/issues/68) | PL-PLATFORM, READY | Plan 0012 inspector and App Intelligence completion |
| [#69](https://github.com/CochranResearchGroup/agent-browser/issues/69) | PL-PLATFORM, READY | Plan 0111 shared-browser authority |
| [#70](https://github.com/CochranResearchGroup/agent-browser/issues/70) | PL-PLATFORM, READY | Plan 0116 upgrade and runtime convergence |
| [#71](https://github.com/CochranResearchGroup/agent-browser/issues/71) | PL-PLATFORM, READY | Plan 0144 lease-authority completion |
| [#72](https://github.com/CochranResearchGroup/agent-browser/issues/72) | PL-PLATFORM, BLOCKED | Plan 0158 protected external-vantage gate |
| [#73](https://github.com/CochranResearchGroup/agent-browser/issues/73) | PL-PLATFORM, READY | Plan 0162 desktop-slot allocation |
| [#74](https://github.com/CochranResearchGroup/agent-browser/issues/74) | PL-PLATFORM, READY | Plan 0163 profile backup, reset, and restore |
| [#75](https://github.com/CochranResearchGroup/agent-browser/issues/75) | PL-AUTH, BLOCKED | Plan 0165 BILL acceptance |
| [#76](https://github.com/CochranResearchGroup/agent-browser/issues/76) | PL-BUGFIX, BLOCKED | Production Service State monitor lock timeout |
| [#77](https://github.com/CochranResearchGroup/agent-browser/issues/77) | PL-BUGFIX, TRIAGE | Stale browser process ownership and cleanup eligibility |
| [#78](https://github.com/CochranResearchGroup/agent-browser/issues/78) | PL-BUGFIX, READY | Development status PID-as-port defect |
| [#79](https://github.com/CochranResearchGroup/agent-browser/issues/79) | PL-PLATFORM, READY | Development generation provenance and skill parity |
| [#80](https://github.com/CochranResearchGroup/agent-browser/issues/80) | PL-PLATFORM, BLOCKED | Development presentation-provider readiness |
| [#81](https://github.com/CochranResearchGroup/agent-browser/issues/81) | PL-PLATFORM, TRIAGE | Stock Chrome CDP-free capability and routing |
| [#82](https://github.com/CochranResearchGroup/agent-browser/issues/82) | PL-PLATFORM, TRIAGE | Residual route-bound extraction decision |
| [#84](https://github.com/CochranResearchGroup/agent-browser/issues/84) | PL-BUGFIX, IN_PROGRESS | Browserless runtime-lane quiescence in PR #83 |
| [#85](https://github.com/CochranResearchGroup/agent-browser/issues/85) | PL-PLATFORM, BLOCKED | Distinct XRDP route-display allocation |

## Preserved Field Note

The formerly untracked profile-selection note is now preserved unchanged at
`docs/dev/notes/0176-2026-09-13-unauthenticated-access-plan-profile-selection-mismatch.md`
with SHA-256
`565b664caaf7e36ff40d4d46d80e323aa69c15a45424513c51e9ccf43de8b5aa`.
Issue #67 owns the profile-selection defect, issue #81 owns the stock-Chrome
capability follow-up, and issue #66 owns the bounded challenge lane. The note
does not become a second backlog.

## Read-Only Runtime Readiness

Production is singular and not mid-install: one runtime host, one dashboard,
one selected generation, zero legacy daemons, no active lifecycle transaction,
and the failed candidate preserved the prior generation. It is not maintenance
ready. The runtime interlock monitor failed twice on `service_state_lock_timeout`
at 1002 ms and 1001 ms against 6,918,163 bytes of Service State, leaving
lifecycle readiness degraded. Issue #76 quarantines production maintenance,
installation, protected-profile actions, and provider acceptance.

Fresh OS evidence also found one old production-shaped managed-ephemeral
browser root and four old test-runtime Chrome roots absent from current Service
State. Issue #77 requires exact ownership before any cleanup.

The development core namespace is isolated and singular on ports 4948, 4949,
and 4951. It remains limited to provider-free diagnostics because its selected
generation points to a removed source worktree, its published skill has drifted,
and its optional presentation provider is not ready. Issues #78 through #80 own
those separate defects and gates.

No installation, restart, repair, retry, process cleanup, browser launch,
profile mutation, provider action, credential action, or tenant effect occurred
during the runtime review.
