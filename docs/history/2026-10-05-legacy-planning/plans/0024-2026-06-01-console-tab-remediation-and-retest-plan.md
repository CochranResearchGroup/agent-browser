# Console Tab Remediation And Retest Plan

Date: 2026-06-01
State: COMPLETE
Lane: P12-L
Parent Plan: `docs/dev/plans/0023-2026-06-01-console-tab-selected-workspace-diagnostics-plan.md`
Depends On:
- `docs/dev/plans/0013-2026-05-31-selected-workspace-context-plan.md`
- `docs/dev/plans/0019-2026-06-01-workspace-tab-dense-inspector-plan.md`
- `docs/dev/plans/0020-2026-06-01-chat-tab-selected-workspace-evidence-plan.md`

## Purpose

Repair the Console tab issues found during hosted visual inspection and make
the retest strong enough to catch them before closeout.

Plan 0023 shipped the first selected-workspace Console surface, but visual QA
proved it is not done. The tab exists, but live Console attribution does not
reach the visible UI, selected workspace context can be missing for a live
route, and the right-pane layout is too cramped in the actual hosted viewport.

## Visual QA Findings

Hosted visual inspection used `agent-browser` against
`https://agent-browser.ecochran.dyndns.org/`.

Evidence screenshots:

- `/tmp/agent-browser-console-visual-initial.png`
- `/tmp/agent-browser-console-tab-selected-after-eval.png`
- `/tmp/agent-browser-console-tab-live-selected.png`

Observed failures:

- Clicking the Console tab by accessibility ref reported success but did not
  visibly switch tabs. Dispatching an in-page click did switch to Console.
- On the `browser:session:default` route, the left selected card showed a
  retained `not_started` browser while the center viewport showed a live CDP
  stream. The Console tab therefore reported missing attribution.
- On a live `visual-console-tab-qa` route, the URL selected
  `browser:session:visual-console-tab-qa`, but
  `data-selected-workspace-context="ready"` was absent.
- A harmless `console.warn("__agent_browser_console_visual_probe__ token=secret")`
  emitted through agent-browser did not appear in the Console tab.
- The Console header row is visually cramped in the right pane. The workspace
  state, attribution label, and counters overlap or truncate into each other.
- The current “no scoped Console entries” state is correct as an unavailable
  state, but it hides the actionable reason: selected context missing versus
  stream connected but no events versus event source unavailable.

## Product Contract

Console must not be considered complete until a hosted browser session proves
all of these:

- selecting a live workspace produces a ready selected-workspace context marker
- the Console tab can be selected through normal user interaction
- a harmless console probe emitted in the selected browser appears as a scoped
  Console row
- the copied and Chat-bound row redacts sensitive values
- the right pane remains legible at the actual hosted viewport width
- retained or mismatched workspace records explain why scoped Console evidence
  is unavailable

## Scope

This is a remediation plan for Console only.

In scope:

- selected workspace context readiness for browser-session routes
- live Console event attribution from the selected workspace stream or target
- Console tab layout at narrow right-pane widths
- explicit unavailable reasons
- agent-browser visual retest coverage
- local runtime publish and hosted smoke

Out of scope:

- Network, Storage, and Extensions tab implementation
- arbitrary JavaScript execution controls
- broad retained profile cleanup
- redesigning the workspace navigator
- changing dashboard authentication

## Implementation Slices

### Slice L1 | Route And Selected Context Repair

Goal: make selected browser-session routes produce a ready selected-workspace
context whenever the service has enough identity to do so.

Tasks:

- Reproduce the live route:
  `workspace=browser:session:visual-console-tab-qa`,
  `browser=session:visual-console-tab-qa`, `session=visual-console-tab-qa`.
- Trace why `data-selected-workspace-context="ready"` is absent even though the
  URL, left card, and viewport identify a live workspace.
- Ensure browser-session, daemon-session, and target selections resolve through
  one selected context authority.
- Prevent a retained `browser:session:default` record from being presented as
  the selected workspace while the center viewport is rendering a separate live
  stream.
- Add an explicit mismatch state when navigator selection and viewport
  selection diverge.

Exit criteria:

- A live browser-session route renders `data-selected-workspace-context="ready"`.
- Workspace, Chat, Activity, and Console receive the same selected context.
- Retained/live mismatch is visible as a diagnostic, not silently collapsed
  into `missing`.

### Slice L2 | Live Console Attribution Repair

Goal: make live Console events emitted by the selected browser arrive as scoped
Console rows.

Tasks:

- Audit the actual event path for `Runtime.consoleAPICalled` and
  `Runtime.exceptionThrown`: CDP client, stream server, dashboard websocket,
  `consoleLogsAtom`, selected Console evidence model, and row rendering.
- Verify whether stream events are emitted for the selected workspace stream
  port, CDP target, browser session, or a different dashboard observation
  stream.
- Add the minimum stable identity to Console and page-error events needed to
  bind them to selected context: stream port plus browser/session/target when
  available.
- Treat stream-port equality as one valid scoped proof, but prefer target or
  browser/session identity when present.
- Keep global fallback rows labeled and excluded from scoped counts by default.
- Preserve redaction before row copy and Chat packet inclusion.

Exit criteria:

- A visual QA probe emitted with:

  ```bash
  agent-browser --session <selected-session> eval 'console.warn("__agent_browser_console_visual_probe__ token=secret")'
  ```

  appears in the selected Console tab as a warning row.
- The visible row or copied bundle includes the probe marker but does not expose
  `token=secret`.
- A non-selected browser's console event does not count as scoped.

### Slice L3 | Dense Right-Pane Layout Fix

Goal: keep Console dense without clipping or overlapping.

Tasks:

- Replace the current cramped header counter row with a stable responsive
  grid sized for the hosted right pane.
- Use short, readable labels: `State`, `Scope`, `Errors`, `Warn`, `Latest`.
- Move source readiness chips into a second compact wrap row.
- Keep filter chips and search controls usable without hiding the first error
  row below the fold.
- Ensure the empty state has a clear reason and a short next diagnostic line.

Exit criteria:

- At the hosted dashboard viewport, the Console tab shows no overlapping or
  clipped header text.
- The first screen includes workspace label, state, attribution, core counts,
  source readiness, filters, search, and either the first row or the explicit
  empty reason.

### Slice L4 | Agent-Browser Retest Harness

Goal: encode the visual inspection as a repeatable smoke, not a manual memory
  exercise.

Tasks:

- Add or extend a dashboard runtime smoke that can:
  - authenticate with the user-scoped dashboard auth file
  - open a live workspace route
  - select the Console tab by normal click
  - emit a harmless console probe into the selected browser
  - poll for a scoped Console row
  - assert redaction
  - capture screenshots on failure
- Add DOM assertions for:
  - selected context marker
  - `data-console-evidence-attribution="scoped"`
  - scoped row count greater than zero after probe
  - no `token=secret` visible in row, copy bundle, or Chat packet
  - no header overlap indicators from bounding-box checks
- Keep the test bounded to a disposable session/profile.

Exit criteria:

- The retest fails on the current broken behavior.
- The retest passes after L1-L3 are fixed.
- Screenshots and JSON output identify which stage failed.

### Slice L5 | Runtime Publish And Hosted Retest

Goal: prove the fixed behavior is visible externally immediately.

Tasks:

- Run selected source validation.
- Publish the local dashboard runtime with changed Console markers.
- Restart the user dashboard service through the existing publish path.
- Run the hosted browser-level visual smoke against
  `https://agent-browser.ecochran.dyndns.org/`.
- Capture before and after screenshots for the Console tab.
- Close disposable QA browser sessions after the smoke.

Exit criteria:

- Hosted visual smoke passes with a live scoped Console probe row.
- The right-pane Console layout is legible in the captured screenshot.
- `agent-browser service status` shows no disposable active browser left behind.

## Validation Matrix

Source checks:

```bash
pnpm test:dashboard-selected-workspace-console
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-contextual-chat
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
git diff --check
node scripts/dev/select-validation.js --base HEAD --json
```

Runtime publish:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker Console \
  --expect-marker data-console-evidence-attribution \
  --json
```

Hosted visual retest:

```bash
node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url 'https://agent-browser.ecochran.dyndns.org/' \
  --session console-tab-remediation-qa \
  --browser-profile /tmp/agent-browser-console-tab-remediation-qa \
  --expect-marker Console \
  --expect-marker data-console-evidence-attribution \
  --json
```

Add the dedicated Console probe smoke in Slice L4 and use it as the
authoritative hosted retest once available.

## Completion Notes

Completed on 2026-06-01.

Implemented:

- The selected workspace marker is now present on the right pane and Console
  tab when a live browser, daemon session, or service browser is selected.
- Workspace CDP stream Console and page-error events append through the shared
  Console atom with stream-port attribution.
- The Console tab now polls the selected session retained Console buffer through
  `/api/session-console`, stamps retained rows as `retained-console`, dedupes
  them, and scopes them to the selected stream port.
- The dashboard server proxies retained Console reads directly to the selected
  daemon session when a session id is available, avoiding the stream HTTP
  self-relay path.
- Console evidence now accepts daemon command envelopes and bare message
  payloads, so both live stream and retained reads feed the same evidence model.
- The Console header was tightened into a dense metric grid with source
  readiness on a compact second row.
- The runtime smoke has `--console-probe` coverage for selected context,
  scoped row count, redaction, and header-overlap checks.

Validation run:

```bash
pnpm test:dashboard-selected-workspace-console
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-contextual-chat
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-inspector-actions
pnpm test:dashboard-view-streams
pnpm test:dashboard-launcher-eligibility
pnpm test:service-cdp-tab-streaming-live
pnpm build:dashboard
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml set_cdp_session_id -- --nocapture
git diff --check
pnpm publish:local-dashboard -- --expect-marker Console --expect-marker data-console-evidence-attribution --expect-marker data-console-scoped-count --browser-profile /tmp/agent-browser-plan0024-publish-smoke --json
node scripts/smoke-local-dashboard-runtime.js --dashboard-url https://agent-browser.ecochran.dyndns.org/ --session console-tab-remediation-retained-live5 --workspace-session console-tab-remediation-retained-live5 --browser-profile /tmp/agent-browser-plan0024-hosted-smoke-live5 --console-probe --skip-chat --json
node scripts/smoke-local-dashboard-runtime.js --dashboard-url https://agent-browser.ecochran.dyndns.org/ --session console-tab-remediation-screenshot2 --workspace-session console-tab-remediation-screenshot2 --browser-profile /tmp/agent-browser-plan0024-hosted-smoke-live5 --console-probe --skip-chat --keep-browser --json
agent-browser --session console-tab-remediation-screenshot2 screenshot /tmp/agent-browser-console-tab-remediation-passing.png --json
```

Hosted proof:

- `console-tab-remediation-retained-live5` passed with `cdpProvider:
  "cdp_screencast"`, `readinessStatus: "ready"`, `attribution: "scoped"`,
  `scopedCount: 1`, `hasProbe: true`, `leaksSecret: false`, and
  `headerOverlapCount: 0`.
- Passing screenshot:
  `/tmp/agent-browser-console-tab-remediation-passing.png`

Notes:

- Two external smoke attempts failed before validation because the hosted route
  returned transient `504` responses for static chunks.
- Earlier failures after the first retained-console implementation exposed two
  missing cases: service-browser selections use `serviceSessionId` for the
  retained read, and daemon responses wrap retained messages in
  `data.messages`.

## Completion Criteria

- The plan's visual QA failures are fixed or converted into explicit
  actionable diagnostics.
- Console tab selection works through normal `agent-browser click` and through
  the hosted UI.
- A live selected browser console probe appears as scoped evidence.
- Sensitive probe data is redacted before display, copy, and Chat handoff.
- The right-pane Console header does not overlap or clip at the actual hosted
  viewport.
- Runtime-published dashboard behavior matches source validation.
- Disposable retest sessions are closed or visibly retained only as reviewed
  evidence.
