# Left Pane Workspace Navigator Slice 3

Date: 2026-05-23

## Scope

Implemented URL-persisted workspace selection for the dashboard left pane.

Selection now writes and restores:

- `workspace`
- `browser`
- `session`
- `tab`
- `profile`
- internal `job` for job related-record jumps

Top-level dashboard paths preserve those query parameters, so `/service`, `/browsers`, `/activity`, and `/` no longer drop workspace context during route changes.

## Implementation Notes

- `WorkspaceNavigator` reads URL selection on mount and on `popstate`.
- `WorkspaceNavigator` also listens for service-panel related-record selection events.
- Row selection writes browser history through `pushState` without native dialogs.
- URL restoration picks the best matching workspace by workspace, browser, session, tab, and profile identity.
- URL restoration fills a missing derived `workspace` id without expanding a profile, session, tab, or job jump back into stale browser and session query params.
- Restored selections scroll into view with centered placement so refresh persistence is visible.
- Service state reads now use the configured dashboard daemon origin instead of the selected browser port. Selecting or restoring a workspace no longer blanks the service records table in local dashboard development.
- The selected row affordance is stronger in dense lists through a primary inset, gradient background, and primary row icon/title treatment.
- Service panel workspace tabs now use `view=service:<tab>` instead of overloading `workspace`.
- Browser, profile, session, tab, and job row clicks update the shared URL selection helper before opening the inspector.
- Related-record actions in the inspector write the selected record identity before pushing the Service view. This avoids Next router replaying a stale intermediate URL.
- Service view changes dispatch the workspace-selection event immediately and on the next tick so the navigator sync runs after Next has settled the URL.

## Rendered QA

Verified with `agent-browser` against `http://127.0.0.1:3104`. During the final verification pass, a long DOM evaluation left the agent-browser page command path unresponsive, so the last URL assertions and screenshots used direct CDP against the same agent-browser-launched Chrome page.

- Desktop reload with URL selection: `/tmp/agent-browser-dashboard-workspace-navigator-slice-3/desktop-final-centered-selection-stable.png`
- Mobile Workspaces tab with URL selection: `/tmp/agent-browser-dashboard-workspace-navigator-slice-3/mobile-final-centered-selection-ref.png`
- Desktop service rows visible in first viewport: `/tmp/agent-browser-dashboard-workspace-navigator-slice-3-sync/desktop-service-initial.png`
- Desktop browser row selected with right inspector open: `/tmp/agent-browser-dashboard-workspace-navigator-slice-3-sync/desktop-browser-selected.png`
- Desktop profile related-record jump with Profiles center view and right inspector open: `/tmp/agent-browser-dashboard-workspace-navigator-slice-3-sync/desktop-profile-related-record.png`
- Mobile Service tab after profile related-record jump: `/tmp/agent-browser-dashboard-workspace-navigator-slice-3-sync/mobile-profile-service-view.png`

Behavior verified:

- Selecting `workspace-navigator-qa` wrote `workspace`, `browser`, `session`, and `tab` query parameters.
- Reload restored the selected workspace.
- Back and forward navigation preserved and restored the selected workspace URL.
- Desktop service records remained populated after workspace selection.
- Mobile Workspaces tab centered the restored selected workspace.
- Selecting the Service browser row for `session:auracall-ui-review` wrote `browser`, `session`, `profile`, and derived `workspace`, selected the matching left-pane row, and opened the browser inspector.
- Clicking `Show profile` from that inspector moved the center Service view to Profiles, kept the right inspector on the profile, preserved the left-pane workspace selection, and left the URL with `profile`, `view=service:profiles`, and `workspace` only. Stale `browser` and `session` params were not retained.
- Direct reload of `/service?profile=custom%3A15138782548871342749&view=service%3Aprofiles&workspace=browser%3Asession%3Aauracall-ui-review` restored the selected workspace row, Profiles center view, and profile inspector.
- Mobile-width inspection showed the Service records panel and profile result within the first viewport after the compact mobile top tab row. The existing issue toast still overlaps the lower-left edge of the viewport.

## Validation

- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `git diff --check`

`pnpm build:dashboard` rewrote `packages/dashboard/next-env.d.ts` to production route types during the build. The file was restored to the existing dev route reference after validation.

Selector recommendations still include Rust, service-client, docs-site, and skill-sync checks because this dirty worktree contains broader pre-existing service, docs, package, and skill changes outside the Slice 3 dashboard URL synchronization surface.

## Follow-up

Slice 3 now covers center Service view and right inspector synchronization. The next campaign slice should start Launch Eligibility And Access Plan Preview, then continue toward guided browser/profile launch and Guacamole-backed viewport control.
