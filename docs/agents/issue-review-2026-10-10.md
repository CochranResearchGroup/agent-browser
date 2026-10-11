# Post-overhaul issue review

Product lane: PL-PLATFORM

Work item: #240. Reviewed canonical source: `2d266b6234998b3ad8af299993abe09813270dff`. Operator approved the setup and default labels. All 27 original open issue bodies and comments were reviewed; 12 are closed as completed or superseded, and 15 remain with explicit next steps. Historical reports remain preserved. This review does not resume archived plans or authorize runtime effects.

## Attention before new features

Prioritize #79 (confirmed development drift) and #176 (reproduced profile allocation inconsistency). Qualify #84, #105, #181 and #183 with bounded current-runtime evidence before selecting repairs. #164 requires a human merge-policy decision because CI is intentionally disabled. #75 and #96 need current consumer-owner information. Remaining enhancements are deferred product choices.

## Every original issue

| Issue | Category | Triage role | Disposition |
| --- | --- | --- | --- |
| [#66](https://github.com/CochranResearchGroup/agent-browser/issues/66) | enhancement | needs-triage | Future challenge work, not a proven pre-feature blocker |
| [#68](https://github.com/CochranResearchGroup/agent-browser/issues/68) | enhancement | needs-triage | Deferred dashboard feature |
| [#69](https://github.com/CochranResearchGroup/agent-browser/issues/69) | enhancement | wontfix | Closed: not planned |
| [#70](https://github.com/CochranResearchGroup/agent-browser/issues/70) | enhancement | wontfix | Closed: not planned |
| [#72](https://github.com/CochranResearchGroup/agent-browser/issues/72) | enhancement | wontfix | Closed as superseded |
| [#73](https://github.com/CochranResearchGroup/agent-browser/issues/73) | enhancement | wontfix | Closed: completed |
| [#74](https://github.com/CochranResearchGroup/agent-browser/issues/74) | enhancement | needs-triage | Deferred backup/restore feature |
| [#75](https://github.com/CochranResearchGroup/agent-browser/issues/75) | enhancement | needs-info | Consumer-specific acceptance, not product implementation |
| [#77](https://github.com/CochranResearchGroup/agent-browser/issues/77) | bug | wontfix | Closed: not planned |
| [#79](https://github.com/CochranResearchGroup/agent-browser/issues/79) | bug | ready-for-agent | Confirmed development drift; attend before new features |
| [#80](https://github.com/CochranResearchGroup/agent-browser/issues/80) | bug | wontfix | Closed: not planned |
| [#81](https://github.com/CochranResearchGroup/agent-browser/issues/81) | enhancement | needs-triage | Deferred capability-design question |
| [#82](https://github.com/CochranResearchGroup/agent-browser/issues/82) | enhancement | wontfix | Closed as superseded |
| [#84](https://github.com/CochranResearchGroup/agent-browser/issues/84) | bug | needs-triage | Unverified browserless competing-lane acceptance |
| [#85](https://github.com/CochranResearchGroup/agent-browser/issues/85) | bug | wontfix | Closed: not planned |
| [#96](https://github.com/CochranResearchGroup/agent-browser/issues/96) | bug | needs-info | Historical authentication-run acceptance |
| [#105](https://github.com/CochranResearchGroup/agent-browser/issues/105) | bug | needs-triage | Current resource-governance concern; attend before features |
| [#164](https://github.com/CochranResearchGroup/agent-browser/issues/164) | bug | ready-for-human | Human merge-policy decision |
| [#176](https://github.com/CochranResearchGroup/agent-browser/issues/176) | bug | ready-for-agent | Confirmed legacy profile consistency defect; attend before features |
| [#181](https://github.com/CochranResearchGroup/agent-browser/issues/181) | bug | needs-triage | Current progress verification; attend before features |
| [#183](https://github.com/CochranResearchGroup/agent-browser/issues/183) | bug | needs-triage | Live browser preservation verification; attend before features |
| [#189](https://github.com/CochranResearchGroup/agent-browser/issues/189) | bug | wontfix | Closed: not planned |
| [#190](https://github.com/CochranResearchGroup/agent-browser/issues/190) | bug | wontfix | Closed: not planned |
| [#199](https://github.com/CochranResearchGroup/agent-browser/issues/199) | enhancement | needs-triage | Deferred desktop executor feature |
| [#203](https://github.com/CochranResearchGroup/agent-browser/issues/203) | enhancement | needs-triage | Deferred artifact-capture feature |
| [#204](https://github.com/CochranResearchGroup/agent-browser/issues/204) | bug | wontfix | Closed: completed |
| [#218](https://github.com/CochranResearchGroup/agent-browser/issues/218) | bug | wontfix | Closed: completed |

## Evidence boundaries

#176 was reproduced in a disposable isolated home: a legacy per_service allocation with multiple aliases is selectable but its freshness update rejects that policy. A shared_service control succeeds. No real profile was changed. #79 has a nonzero development install doctor with executable/dashboard identity drift. Neither result is explained by retired RDP prerequisites.

Completed-source dispositions do not claim new exhaustive installed qualification. Native lifecycle and capacity evidence remains bounded by the existing acceptance receipts. Application authentication continuity is unverified. No browser credentials, tenant state or private page data are included here.

## Setup and validation

The six docs/agents files configure the engineering skills without creating a competing domain glossary. CONTEXT.md remains the domain authority. Default triage roles replace obsolete reviewed state/triage and state/ready labels; stale coordination labels are retired on these reviewed issues. Scoped rejected requests #72 and #82 are recorded under .out-of-scope. Validation covers local documentation links, reviewed manifest source hashes and diff hygiene. No Rust or browser behavior changes.
