# Plan 0030: Terminated Browser History Boundary

## Goal

Terminated browser records must not appear as active work in the dashboard. A browser that has already terminated belongs in retained records, events, prune output, or logs, not in the left rail or the default actionable browser view.

## Evidence

- User direction on 2026-06-07: once a browser is terminated there is no reason to retain it anywhere other than logs.
- `packages/dashboard/src/lib/service-workspaces.ts` builds left-rail workspace nodes from every service browser before deciding whether the record is operational.
- `packages/dashboard/src/components/service-panel.tsx` treats `process_exited` and `unreachable` browser records as high-priority default rows, even though those states are post-termination history.

## Scope

1. Add an explicit dashboard predicate for post-termination browser history.
2. Prevent historical browser records from producing left-rail browser nodes.
3. Prevent sessions whose only browser evidence is historical from being promoted into standalone left-rail session nodes.
4. Keep historical browser records available in the browser table under retained or all-record views.
5. Keep live or recoverable records, such as degraded or CDP-disconnected rows with process evidence, eligible for operational attention.

## Validation

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-browser-table`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD`
- `git diff --check`

## Closeout

Commit the implementation and tests once the dashboard selectors and validations pass.

## Result

Implemented on 2026-06-07. Post-termination browser records now stay out of left-rail workspace nodes, do not promote their linked sessions into standalone workspace nodes, and are hidden from the default actionable browser-table view. Retained and all-record table views still expose the historical records for review.

Validation passed:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-browser-table`
- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `pnpm publish:local-dashboard -- --expect-marker "All records" --browser-profile /tmp/agent-browser-dashboard-smoke-profile-0030`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `git diff --check`
