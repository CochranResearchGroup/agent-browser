# Plan 0099 Cycle 1 Audit | Workspace View Projection Deepening

Review mode: `drift_discovery`

Review cycle: 1 of 2

Reviewer role: independent plan auditor

Branch: `architecture-deepening-20260809`

Repository base and HEAD:
`ae36b272327982e3227f4dc7c5d6dc5b4b16350c`

Frozen target:
`docs/dev/plans/0099-2026-08-09-workspace-view-projection-deepening-plan.md`

Target content SHA-256:
`8dd82c5e6422ef3da90541acb0a2bd8d1ef4d25ac8aeb2d737041829d6aa20a0`

Audit date: 2026-08-09

## Review Packet

The frozen acceptance requires one deep, pure workspace view projection
interface; preserved canonical service authority; explicit daemon fallback;
stable operator stream preference; foreign CDP read-only lifecycle and Borrow
semantics; source-preserving route, provider fallback, and readiness behavior;
thin rendering callers; deletion of duplicate scoring and readiness
derivations; replace-not-layer tests; no public wire expansion unless proven
necessary; and a two-cycle review bound.

The audit used the `codebase-design` deep-module and deepening criteria. The
candidate has the correct in-process dependency classification, one external
seam, and an appropriate replace-not-layer test strategy. CodeGraph was
healthy at 419 indexed files, 14,341 nodes, and 43,350 edges. Focused CodeGraph
reads covered workspace node derivation, selected context, viewport source
merge, stream preference, tile projection, readiness, route URL restoration,
and Service inspector selection. Direct reads were limited to the plan and
policy authorities, literal preference and route fields, and exact source
lines already identified through CodeGraph. Graphiti MCP was healthy and the
focused `agent_browser_main` query returned ten facts, but none directly
described P99 or its proposed projection contract, so no Graphiti claim was
used as source authority.

## Findings

| ID | Criterion | Exact evidence | Consequence | Reproducer or check | Confidence | Suggested disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `P0099-A1-01` | Canonical inventory, proof, and lifecycle authority must enter the projection as explicit upstream facts. The pure module may select a presentation stream, but it must not recreate service ownership from stream capability or row shape. | P45 assigns the dashboard record to Rust inventory and forbids dashboard reconstruction of lease, proof, or ownership (`docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md:56-74,142-145,176-181,346-367`). P99 repeats that rule (`plan:159-175,220-235`) and says projection output contains canonical inventory class and authority source (`plan:307-319`). Its shown source interface, however, names browsers, tabs, routes, daemon sessions, and selected context without a defined canonical inventory or authority envelope (`plan:268-290`); the structural source types are deferred to implementation (`plan:345-355`). The current caller cannot simply supply a preexisting class: `createBrowserWorkspaceNode` selects and proof-gates a stream first (`packages/dashboard/src/lib/service-workspaces.ts:1039-1112`), derives workspace state from the resulting view capability (`service-workspaces.ts:1113-1124,1605-1623`), and only then computes inventory class (`service-workspaces.ts:1167-1197,1632-1652`). P99 simultaneously asks this caller to project once up front, pass upstream authority into the projection, and prevent projection output from reclassification (`plan:359-370`). | The implementer must invent an ordering and authority contract. Projecting before node derivation lacks the promised canonical class; deriving the class from projected capability makes the new dashboard module an ownership engine; projecting after node derivation leaves stream and readiness derivation layered in the old caller. Any path can violate canonical service authority or the deletion requirement while still satisfying the loose type sketch. | Add an explicit source-authority ledger and a fixture whose stream URL and control fields look ready while canonical inventory or route-bound ownership is diagnostic. The projected result must retain the diagnostic class and disabled lifecycle actions. Trace the intended call order through `deriveWorkspaceNodes -> createBrowserWorkspaceNode -> workspaceState -> browserInventoryClass`; no projection output may be used to manufacture an upstream authority fact. | High | `blocking` |
| `P0099-A1-02` | Selected context, viewport, workspace nodes, and inspector must consume one source snapshot, including route descriptors needed for manual-runtime and public-ingress URL projection. Direct selected-context route restoration must actually become deletable. | P12 requires selected context to derive from service and daemon stream state (`docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md:204-218`) and to be shared across the viewport and right-pane tabs (`P12:270-290`). P99 includes `remoteViewRoutes` in the deep interface (`plan:275-281`), requires compatibility fields to come from projection output (`plan:372-381`), and requires direct selected-context route restoration to disappear (`plan:590-600`). The actual selected-context fetch shape omits `remoteViewRoutes` (`packages/dashboard/src/hooks/use-selected-workspace-context.ts:31-42`) and its projection input passes browsers, sessions, tabs, manual browsers, and authority but no route map (`use-selected-workspace-context.ts:136-150`). The viewport separately reads the route by selected context route ID (`packages/dashboard/src/components/workspace-remote-viewport.tsx:1599-1605`). P99's caller migration does not name or modify `use-selected-workspace-context.ts`, even though that hook owns the missing source plumbing. | Following the listed migration leaves two unsatisfactory outcomes: selected context loses route-descriptor, public URL, or manual-runtime evidence that only the route map supplies, or the viewport keeps the separate route-restoration path that acceptance criterion 5 requires deleting. The shared P12 context and viewport can continue to report different URLs or readiness from nominally the same service status. | Extend a selected-context fixture with a node stream containing only a route ID and a `remoteViewRoutes[routeId]` record containing `localEmbedUrl`, public or dashboard URL, and structured readiness. Build the context through `useSelectedWorkspaceContext` and project the viewport from the same status snapshot. Both must expose the same effective route evidence without a viewport-local lookup. Add the hook and its status shape to Slice 2's explicit migration files and tests. | High | `blocking` |
| `P0099-A1-03` | Explicit operator preference must retain its current scope while tile ordering remains stable. A scalar route preference must not silently become a broadcast preference over every projected candidate. | Today `view-provider` is resolved only for the selected browser and passed to `primaryViewStream` (`packages/dashboard/src/components/workspace-remote-viewport.tsx:1613-1625`). Tile projection separately selects each browser from its browser-specific persisted key and does not receive `view-provider` (`workspace-remote-viewport.tsx:723-750,1626`). P99 replaces both paths with one output containing `selected`, `candidates`, and `tiles`, but models `preferredProvider` as one scalar intent field (`plan:268-304`). Its fixture list tests explicit CDP preference and tile ordering separately, not their interaction (`plan:429-450`). | An implementation can reasonably apply `preferredProvider` to every candidate in tile mode, changing stream choice, route-sharing counts, and tile order, or ignore it too broadly and break selected-browser intent. Either result could pass the listed independent fixtures while changing current preference semantics. | Add a combined fixture with two browsers, different stream sets, `mode: "tile"`, a selected-browser `view-provider`, and persisted keys for both browsers. Freeze whether route preference is selected-only, ignored in tile mode as today, or deliberately widened. Model that scope explicitly, for example as a selected candidate preference rather than an unqualified global scalar. | Medium-high | `needs_evidence` |

## Findings Summary

- Blocking candidates: `P0099-A1-01`, `P0099-A1-02`.
- Needs-evidence candidate: `P0099-A1-03`.
- Nonblocking backlog candidates: none.
- Rejected candidates: none at discovery time. Final disposition belongs to the
  orchestrator under the frozen review contract.

## Cycle 1 Terminal Recommendation

Plan 0099 is not implementation-ready in its current form. The deep-module
seam, pure dependency classification, deletion strategy, readiness
source-preservation goal, foreign CDP guardrails, and bounded review workflow
are sound. The unresolved parts are at the authority and caller-plumbing edges
of that seam.

Use the one allowed remediation pass to define an explicit canonical authority
input and call order that cannot derive ownership from projected capability;
add `use-selected-workspace-context.ts` and its `remoteViewRoutes` status
plumbing to the selected-context migration; and adjudicate the scope of
`view-provider` when the same projection also emits tiles. Add the corresponding
crossed fixtures before implementation.

After accepted blockers are remediated, Cycle 2 should be a closed-world
verification of `P0099-A1-01` and `P0099-A1-02`, plus `P0099-A1-03` only if the
orchestrator accepts it as blocking after choosing the intended preference
scope. Do not reopen broad discovery.

## Audit Effects

- Source changes: none.
- Plan changes: none.
- Runtime, browser, tenant, provider, scheduler, installed-service, release,
  commit, and network effects: none.
- Artifact written: this audit note only.

## Cycle 2 Closed-World Verification

Review mode: `closed_world`

Review cycle: 2 of 2

Revised target content SHA-256:
`20ee1a82365ba9a89ce3140c05e0ace9915d5b5be032ae7f0e308a066a6b68c7`

Verification scope was limited to `P0099-A1-01`, `P0099-A1-02`,
`P0099-A1-03`, and critical contradictions introduced by their remediation.
No broad discovery was reopened.

| Finding | Result | Exact verification evidence | Residual disposition |
| --- | --- | --- | --- |
| `P0099-A1-01` | `PASS` | Plan version 2 defines a required `WorkspaceViewAuthorityLedger` with inventory, lifecycle, route ownership, proof, action ceilings, and diagnostics (`plan:179-220`); forbids authority inference from stream shape and fails closed on missing authority (`plan:222-227`); fixes the raw snapshot to ledger to projection to node call order (`plan:248-262`); requires deletion of stream-dependent state and class branches (`plan:264-269,488-509,657-680`); and adds crossed ready-looking stream versus diagnostic-authority fixtures (`plan:602-610`) plus exact-preservation acceptance (`plan:802-808`). The import direction is made acyclic through seam-owned or lower type-only definitions (`plan:470-484`). | Resolved. No blocking or nonblocking residual for this ID. |
| `P0099-A1-02` | `PASS` | The hook status shape now explicitly gains `remoteViewRoutes`, and the hook must build ledger and projection from the same immutable parsed status snapshot (`plan:511-527`). Selected context carries a completed projected view and source snapshot identity, while the old route-restoration helper is deleted (`plan:529-542`). The viewport is forbidden to replace selected route evidence from its separately refreshed status and must remove its local route lookup (`plan:544-557`). The route-ID-only crossed fixture, Slice 2 exit gate, acceptance criterion, and guard searches cover the same-snapshot behavior (`plan:621-624,666-680,833-836,857-868`). | Resolved for route evidence and direct route-restoration deletion. No blocking or nonblocking residual for this ID. |
| `P0099-A1-03` | `FAIL` | The revised interface correctly separates selected-subject preference from per-browser tile keys and forbids broadcasting selected preference to tiles (`plan:271-293,385-409`). The combined tile fixture covers tile choices, sharing counts, and order (`plan:629-633`). A critical integration contradiction remains: the hook is told to build and return a complete selected projection from the status snapshot (`plan:519-527,531-540`), and the viewport is told to render that complete projection (`plan:550-555`), but the viewport remains the owner of local preference persistence and is separately told to supply `view-provider` and `byBrowserId` (`plan:558-566`). The plan gives the hook no preference input or preference-change trigger. In current source, `DashboardWorkspaceUrlSelection` omits `view-provider` (`packages/dashboard/src/lib/workspace-url-selection.ts:3-21,36-46`), the hook tracks only that selection and service status (`packages/dashboard/src/hooks/use-selected-workspace-context.ts:61-70,99-108,136-151`), while preference storage and direct `view-provider` parsing live only in the viewport (`packages/dashboard/src/components/workspace-remote-viewport.tsx:146-164,1563-1574,1613-1625`). Therefore the already-complete context projection cannot be guaranteed to reflect the explicit or persisted preference that acceptance criterion 10 promises. | Residual `blocking`. Freeze one preference-aware construction point: either the hook receives the selected and persisted preference scope and rebuild triggers, or selected context carries same-snapshot route-completed projection sources so the viewport can make its one preference-aware projection without a fresh route lookup. Add an integration fixture proving a selected alternate preference changes the viewport effective stream while context and viewport retain identical route evidence, and the existing tile fixture still proves no broadcast. |

### Cycle 2 Disposition

- Resolved findings: `P0099-A1-01`, `P0099-A1-02`.
- Residual blocking finding: `P0099-A1-03`.
- Residual nonblocking findings: none.
- Critical contradiction introduced by remediation: preference ownership versus
  the precomputed complete selected-context projection, recorded under
  `P0099-A1-03` rather than as a new discovery finding.
- Final implementation-ready: **no**.

Under the two-cycle bound, this result must not trigger a third broad plan
audit. The residual preference-construction contradiction should be reported as
a blocker and handled by the orchestrator through a bounded plan correction or
split before implementation.

### Cycle 2 Effects

- Plan changes: none.
- Source changes: none.
- Runtime, browser, tenant, provider, scheduler, installed-service, release,
  commit, and network effects: none.
- Artifact effect: appended this Cycle 2 result to the existing audit note.
