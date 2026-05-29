# Left Pane Workspace Navigator Slice 5

Date: 2026-05-23

## Scope

Implemented the guided browser/profile launch submission surface for the
workspace navigator campaign.

The New workspace dialog now lets an operator select a visible browser/profile
candidate, fetch its no-launch access plan, choose a target URL, choose display
isolation, choose a view stream preference, choose a control input preference,
and submit only through the service request queue when the service-owned gates
mark the combination launchable.

The existing local session creation path remains available, but it is visually
secondary to the service-owned launcher controls.

## Implementation Notes

- Added access-plan request builders to
  `packages/dashboard/src/lib/launcher-eligibility.ts`.
- The request builder starts from
  `decision.serviceRequest.request`, preserves `profileLeasePolicy`, caller
  labels, target identity, browser build, and launch posture, then applies
  operator-selected target URL, display, view, and control overrides.
- The dashboard posts the built request to `/api/service/request`.
- Local dashboard development now uses the relative `/api/service` proxy when
  both the dashboard and configured daemon URL are loopback hosts, avoiding a
  stuck cross-origin plan request.
- Access-plan and launch requests use a bounded timeout so the dialog fails
  visibly instead of leaving the operator in an indefinite loading state.
- After a successful launch submission, the dashboard pushes `/service` with
  `view=service:jobs` and the returned `job` query when available.

## Service Request Shape

The no-launch request-shape test covers the dashboard helper with this shape:

```json
{
  "action": "tab_new",
  "serviceName": "JournalDownloader",
  "agentName": "codex",
  "taskName": "downloadArticle",
  "targetServiceIds": ["acs"],
  "accountIds": ["research@example.test"],
  "browserBuild": "stealthcdp_chromium",
  "profileLeasePolicy": "wait",
  "jobTimeoutMs": 60000,
  "url": "https://example.test/start",
  "params": {
    "browserHost": "remote_headed",
    "viewStreamProvider": "novnc",
    "controlInputProvider": "vnc_input",
    "displayIsolation": "shared_display",
    "url": "https://example.test/start"
  }
}
```

For CDP-free access plans, the same helper changes the action to
`cdp_free_launch` and sets `requiresCdpFree: true` plus
`cdpAttachmentAllowed: false`.

## Visual Inspection

Rendered inspection was run with `agent-browser` against the local dashboard at
`http://127.0.0.1:3400/service?view=service:jobs`.

Screenshots:

- `/tmp/agent-browser-dashboard-workspace-navigator-slice-5/desktop-service-jobs-before-dialog.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-5/desktop-launch-dialog-final.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-5/desktop-launch-dialog-planned-ready.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-5/mobile-launch-dialog-final-open.png`

Findings:

- Desktop launch controls are visible on first open without hiding behind the
  candidate list.
- Mobile launch controls expose Plan and Launch immediately after the target
  URL, before the lower posture selectors.
- Planned rows show service-sourced posture, including lease policy, display,
  view stream, and control input.
- The visible live row remained `needs action` because service state reported
  no viewport evidence, so Launch stayed disabled with the access-plan reason.

## Validation

Passed:

- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm build:dashboard`
- `pnpm --dir docs build`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request -- --test-threads=1`
- `git diff --check`

`pnpm build:dashboard` emitted the existing Next.js export warning about
rewrites, but the build completed successfully.

No live mutating launch smoke was run. The currently visible service-owned
candidate had zero eligible rows after access-plan and viewport checks, and
forcing a launch would have bypassed the disabled state and risked mutating a
non-isolated default profile. The live rendered check did exercise the no-launch
plan path against the running service.

## Remaining Gaps

- The launcher still depends on per-row access-plan fetches. A backend batched
  eligibility endpoint would reduce operator waiting and simplify the UI.
- The current live service state has no eligible visible launch row with
  controllable viewport evidence. Slice 6 should connect eligible launch
  results to view streams and Guacamole or `rdp_gateway` focus behavior.
- The service panel remains visually dense but dim in the current theme. That
  is outside this slice, but it should stay on the campaign QA list because it
  affects first-viewport scanability.

## Next Recommended Slice

Proceed to Slice 6: focus the launched or selected workspace and open the
embedded remote viewport when service state reports an embeddable controllable
stream.
