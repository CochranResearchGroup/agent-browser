# P260 exact selected-tab evidence acceptance

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: docs/dev/plans/0260-2026-10-21-selected-tab-evidence.md / CochranResearchGroup/agent-browser#339
Related lanes: #68, P237, P248

## Defect and repair

Selecting a secondary tab in a browser left selected-workspace context pointing
at the browser node's default page. Its diagnostics and contextual chat could
therefore describe the wrong task. A pure two-tab fixture reproduced actual
`tab-live` versus expected `tab-second` before the repair.

The context now projects only the explicitly selected owned tab's page fields.
Logical and target IDs accept raw, `tab:` and `target:` handles consistently.
Target-only selection joins secondary tab inventory to its owning browser. An
explicit missing or foreign tab produces no replacement page; an absent explicit
tab retains the original browser default. Browser-wide jobs, incidents and
resources keep their owner scope. No native selection, polling, action authority
or browser effect changes. Page projection does not copy task metadata or other
whole-tab properties into the existing diagnostic field.

## Qualification

The regression first failed on the unchanged source. Final context checks pass
for four explicit handle forms, secondary target-only lookup, missing and foreign
targets, diagnostics/chat agreement, retained owner jobs and restricted page
fields. Existing selected-workspace chat packet, view projection, contextual
chat, inspector action and workspace inspector checks pass. Typecheck passes.
The final dashboard and docs builds, Cargo-safe format and strict workspace
Clippy, policy wiring, service/API/MCP parity, links and P260 planning audit are
passed at closeout.

Standards review checks exact owner scoping and the unchanged small page shape.
Specification review keeps this one proven defect separate from the broad #68
legacy inspector/App Intelligence request. No new inspector tabs or AI actions
are introduced and #68 remains open.

## Boundaries

Source only. No installed publication, browser launch, runtime restart, provider
operation, private-account inspection, shared skill publication or CI dispatch.
The prior public/native dashboard receipts are not a new installed test of this
projection repair. Sealed candidates and foreign root/resource checkouts remain
preserved.
