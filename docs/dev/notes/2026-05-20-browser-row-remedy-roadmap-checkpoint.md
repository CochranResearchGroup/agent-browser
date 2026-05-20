# Browser Row Remedy Roadmap Checkpoint

Date: 2026-05-20

## Purpose

This note pauses the browser row remedy lane and checks it against the service
roadmap before more feature work continues.

The lane started as a dashboard row-action affordance but stayed aligned with
the backend-first rule: the dashboard now exposes actions only after the service
owns the contract, request action, lifecycle behavior, and validation evidence.

## Roadmap Alignment

This work supports the always-available service roadmap because browser
lifecycle remedies now route through the service request path that agents and
software clients already use.

Completed alignment points:

- `service_browser_close` is a service-request action for the active
  service-owned browser only.
- `service_browser_repair` is a service-request action for one degraded or
  faulted retained browser record after operator review.
- HTTP, MCP, generated clients, schema metadata, docs, and the agent-browser
  skill now describe the same row remedy semantics.
- The dashboard treats `GET /api/service/contracts` as optional capability
  discovery. Older installed services still render safely with row remedies
  disabled.
- Dashboard row actions are gated by both advertised backend capability and
  row eligibility.
- The rendered disabled copy distinguishes missing backend support from row
  ineligibility.

This is not a new roadmap pillar. It is a browser-lifecycle control-plane slice
that closes an operator affordance gap in the existing Service workbench.

## Validation Coverage

The lane now has coverage at the important boundaries:

- Rust focused tests cover service-browser repair, service-browser close
  rejection for non-active browsers, service contract metadata, and service
  request action acceptance.
- No-launch HTTP and MCP smoke coverage verifies row-scoped repair and
  non-active close rejection without launching Chrome.
- Live isolated smoke coverage verifies successful `service_browser_close`
  through HTTP service request against one disposable browser session.
- Dashboard source-contract smoke coverage verifies row action wiring,
  capability gating, active-browser close eligibility, degraded or faulted
  repair eligibility, optional contracts discovery, and title-helper wiring.
- Rendered-title smoke coverage verifies the actual Close and Repair button
  title strings for unsupported, ineligible, and available states.
- The validation selector now recommends the rendered-title smoke when the
  Service panel, row-action title helper, or rendered-title smoke changes.

Recent commits in this lane:

- `0272c9f9` Add service browser row action group
- `0ad1c7f4` Add service browser row remedies
- `f75bdbfe` Add no-launch browser row remedy smoke
- `baf632d4` Add live browser row close smoke
- `577f1b75` Harden browser row action safety contract
- `469deb26` Guard optional service contracts discovery
- `8f6a32f5` Clarify browser row remedy disabled reasons
- `84c9aad6` Add rendered browser row action title smoke
- `5cdd3b63` Recommend browser row action render smoke

## What Is Complete Enough To Pause

Treat this lane as complete enough to pause unless a regression appears:

- service-owned browser close and repair request actions
- dashboard row action enablement and disabled explanations
- optional contract compatibility with older installed services
- local validation coverage for backend contracts, no-launch paths, live close,
  dashboard wiring, rendered copy, and selector recommendations

Do not keep polishing this lane only for aesthetics. More work here should be
triggered by a specific defect, a release-gating failure, or a concrete operator
UX problem observed in the live Service tab.

## Remaining Risks

Known residual risks:

- Successful close is live-smoked only through HTTP service request, not MCP
  service request. MCP close should be added only if MCP-specific close drift
  appears, because the no-launch MCP path already verifies dispatch and
  rejection semantics.
- The dashboard rendered-title smoke renders the row-action button title
  helper through static React markup, not the full Service panel with a mocked
  API. This is intentional to avoid adding a heavy frontend test harness before
  the repo needs one.
- Installed user-scoped services will not advertise the new actions until the
  new build is installed. The dashboard compatibility guard keeps those
  services usable with row remedies disabled.

## Recommended Next Slice

Return to the backend-first roadmap. The best next slice is service-owned
profile readiness and access-policy authority, not more browser row remedy
polish.

Recommended focus:

1. Revisit access-plan and profile-readiness notes.
2. Pick one small backend-owned decision field that helps agents choose the
   right profile, browser build, or manual action.
3. Expose it through HTTP, MCP, generated clients, and dashboard only after the
   service owns the state.
4. Validate no-launch first, then add one bounded live smoke only if launch
   selection changes.
