# Workspace Inventory Audit

Date: 2026-07-09

## Context

The left rail currently populates slowly, caps visible rows unpredictably, drops owned browser rows, and can hide viable non-owned browsers. Needs Attention is not consistently a bottom area, and non-existent browsers can still be represented as attention rows. This is a projection failure at the workspace inventory layer.

This audit covers Candidate 3 from the architecture review: Workspace Inventory.

## Current Shape

The dashboard workspace inventory is assembled in `packages/dashboard/src/lib/service-workspaces.ts` and rendered by `packages/dashboard/src/components/workspace-navigator.tsx`.

The current `WorkspaceNode` shape mixes several decisions:

- source and role
- active, detected, retained, and needs-attention grouping
- inventory class
- live versus retained
- route-bound ownership
- profile actionability
- view stream projection
- diagnostics
- actions
- related IDs and counts

The navigator groups nodes by `node.group`, then renders scopes for all, active, needs-attention, and detected. The viewport also builds its own top stream tiles from service-state browsers, separate from the left-rail grouping.

## Failure Mechanism

The inventory projection is doing too much local inference without an upstream authority verdict:

- A browser row can be retained, degraded, route-blocked, profile-actionable, or stream-blocked for different reasons.
- Non-owned browsers can be viable as read-only CDP snapshots, but still lose ranking or visibility when the owned rows are degraded.
- Needs Attention is a group, not a final sorted bottom lane, so attention rows can compete with initial inventory rather than being a later fallback.
- Non-existent or no-longer-live rows can survive through retained/history paths and look like action targets.
- The viewport can choose different stream candidates than the navigator, so an item can look usable in one surface and fail in the other.

This explains why the rail can first show owned and unowned rows, then drop owned rows, while non-owned viable rows do not reliably stay visible.

## Deletion Test

Deleting `workspace-navigator.tsx` would remove the visual symptom but leave the projection ambiguity in `service-workspaces.ts`.

Deleting `service-workspaces.ts` would remove the projection but not replace it with a deeper inventory interface. Consumers would still need to infer from service status, sessions, tabs, profiles, streams, resources, and incidents.

Deleting viewport tile selection would not stabilize the rail, because rail grouping and row viability are decided separately.

## Recommended Deep Module

Create a Workspace Inventory Authority in the dashboard data layer, fed by daemon authority verdicts. Its interface should produce ordered lanes rather than raw grouped nodes:

- primary viable browsers
- viable read-only detected browsers
- actionable profile launch rows
- retained history
- needs attention

Rows that do not represent an existing viable or actionable entity should be excluded from initial inventory and should not appear in Needs Attention unless they point to a concrete recovery action.

## Required Interlocks

- The inventory layer must distinguish `hidden`, `primary`, `detected`, `retained`, and `attention` placement from the node's health label.
- Needs Attention must be a final lane after viable and retained rows, not a peer that can take initial focus.
- A row must have one of: live browser authority, viable detected CDP target, retained history with explicit retained state, or actionable profile launch path.
- Viewport tile selection should consume the same ordered inventory candidates instead of rebuilding a separate ranking from service-state browsers.
- Non-existent rows must be excluded unless they have a concrete recovery action.

## Risks

- A purely visual sort change would mask stale inventory but not prevent invalid selections.
- Removing retained rows outright would lose useful recovery history.
- Treating non-owned browsers as lower quality by default would regress the shared-profile and external-CDP workflows.

## Acceptance For Candidate 3

- A plan exists to split inventory placement from node state.
- The first implementation slice should add a pure inventory ordering function with focused tests.
- Needs Attention rows should sort after viable rows.
- Non-viable, non-actionable rows should not enter initial inventory.
- Viewport and navigator should be able to share the same candidate ordering in a later slice.
