# S10 Foreign CDP Inventory Plan

Date: 2026-06-27
State: COMPLETE
Lane: P63
Parent: `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`

## Problem

P46 S10 is locked by the two-consecutive-failure rule. The scenario is intended
to prove that a process-scanned, non-owned CDP browser can appear beside a
service-owned route-bound RDP browser without being promoted into service-owned
route, display, lifecycle, or mutation control.

The first two S10 live attempts failed before the scenario evaluator ran:

- `/tmp/agent-browser-p46-s10-2026-06-27T22-17-57-154Z` queried `/sessions`
  and received dashboard HTML instead of JSON.
- `/tmp/agent-browser-p46-s10-2026-06-27T22-20-21-552Z` queried
  `/api/sessions` without dashboard authentication and received HTTP 401.

Both attempts reset cleanly and reported zero active incidents after
reset-after.

## Repair

The S10 harness now reads dashboard inventory through the authenticated
viewer-client session after dashboard login:

- `/api/sessions` for the process-scanned foreign CDP row;
- `/api/session-tabs?port=<foreign-cdp-port>` for the foreign CDP tab list.

The harness keeps the foreign browser profile outside `~/.agent-browser`, so
stream discovery should classify it as `ownership: foreign_cdp` rather than as
agent-browser-owned runtime state.

## Authorized Retry Failure

P63's first authorized retry ran after the green no-live preflight:

```text
/tmp/agent-browser-p46-s10-2026-06-27T22-27-54-257Z
```

The retry proved the authenticated `/api/sessions` read and detected the
launched foreign CDP browser as:

- `provider: detected-cdp`;
- `detected: true`;
- `ownership: foreign_cdp`;
- `addressability: cdp_reachable`;
- `profilePath` under the temporary S10 artifact directory.

The retry then failed before S10 evaluation because `/api/session-tabs` proxied
the foreign raw Chrome CDP port as if it were an agent-browser stream server
with `/api/tabs`. Raw Chrome CDP exposes `/json/list` instead, so the dashboard
CDP evaluation timed out waiting for the tab inventory fetch to complete.

Reset-after closed `default` and reported zero active incidents.

## Product Repair

`cli/src/native/stream/dashboard.rs` now bounds local dashboard backend proxy
reads with a timeout and falls back from `/api/tabs` to raw Chrome CDP
`/json/list` for `/api/session-tabs?port=<foreign-cdp-port>`. The fallback
normalizes page targets into the dashboard `TabInfo` shape with `index`,
`active`, `title`, `url`, `type`, and `targetId`.

Validation for this repair:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml dashboard -- --nocapture
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
node scripts/test-dashboard-workspace-nodes.js
git diff --check -- cli/src/native/stream/dashboard.rs scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md
pnpm publish:local-dashboard -- --skip-smoke --json
/home/ecochran76/.local/bin/agent-browser --json install doctor
node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json
/home/ecochran76/.local/bin/agent-browser --json service incidents --summary
```

The installed executable SHA after publish is:

```text
c3288e17066c5275f149bef52ddace793ce08eba8f356ef1be6039520c8d8e8d
```

Authorize exactly one post-repair S10 retry only after the above validation is
green and incidents remain zero.

## Validation Gate

Before another live S10 retry, run:

```bash
node --check scripts/run-p46-stress-scenario.js
node --check scripts/lib/p46-scenario-harness.js
node scripts/test-p47-scenario-harness.js
node scripts/test-dashboard-workspace-nodes.js
git diff --check -- scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js scripts/test-dashboard-workspace-nodes.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md
/home/ecochran76/.local/bin/agent-browser --json service incidents --summary
```

Authorize exactly one retry only if the no-live gate passes and active
incidents are zero:

```bash
node scripts/run-p46-stress-scenario.js --scenario s10 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

## Pass Conditions

The retry must prove all of the following:

- the foreign browser appears in authenticated dashboard inventory as
  `provider: detected-cdp`, `detected: true`, `ownership: foreign_cdp`, and
  `addressability: cdp_reachable`;
- the foreign selected workspace is a daemon-session detected non-owned browser
  and exposes no runnable focus, view, control, add-tab, repair, close, kill,
  or borrow-control mutation action;
- read-only foreign CDP actions remain listed or explicitly frontend-disabled,
  and mutation requires explicit borrow or adoption;
- the foreign row has no service-owned route ID, display allocation ID,
  route-pool entry, remote viewport iframe, or borrowed stream state;
- the service-owned row remains selected as a service-browser with view/control
  ready and a remote viewport iframe;
- switching from service-owned to foreign and back preserves selected workspace
  context;
- route-bound finalization remains complete for the service-owned browser;
- reset-after leaves zero active incidents.

## Current State

The authenticated inventory fix was insufficient by itself; the first
authorized retry exposed the raw Chrome CDP tab inventory fallback gap. The
product repair is installed locally and runtime doctor is green. One
post-repair retry is authorized if incidents remain zero immediately before the
run.

## Post-Repair Cleanup-Mask Failure

The first post-repair retry artifact is:

```text
/tmp/agent-browser-p46-s10-2026-06-27T22-35-11-209Z
```

The run did not fail on the previous `/api/session-tabs` CDP evaluation
timeout. Instead, `foreignBrowser.close()` threw `ENOTEMPTY` while removing the
temporary foreign Chromium profile, masking the real scenario stage before the
runner could write the later S10 artifacts or evaluator result. Reset-after
again closed `default` and reported zero active incidents.

Harness cleanup now treats foreign profile removal as best-effort with bounded
retries and records `cleanupError` in the launch artifact instead of throwing
from `finally`.

Additional validation:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
git diff --check -- cli/src/native/stream/dashboard.rs scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md
```

One cleanup-fix retry is authorized if incidents remain zero immediately before
the run. Its purpose is to reveal the actual S10 evaluator state after the raw
CDP tab fallback repair.

## Raw CDP Keep-Alive Proxy Failure

The cleanup-fix retry artifact is:

```text
/tmp/agent-browser-p46-s10-2026-06-27T22-38-40-614Z
```

The run reached the authenticated foreign tab inventory request and failed
with:

```text
Session tabs proxy failed: timed out reading from 127.0.0.1:<port>/json/list
```

This proved the `/api/session-tabs` fallback selected raw Chrome CDP
`/json/list`, but the dashboard proxy still used `read_to_end` and waited for
Chrome to close the connection. Raw Chrome can keep the connection open even
when the response body is complete.

`cli/src/native/stream/dashboard.rs` now reads proxied local HTTP responses
until the declared `Content-Length` body is present, with a response-size cap
and the existing per-read timeout. This preserves bounded behavior without
requiring backend connection close.

Additional validation:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml dashboard -- --nocapture
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
git diff --check -- cli/src/native/stream/dashboard.rs scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md
pnpm publish:local-dashboard -- --skip-smoke --json
/home/ecochran76/.local/bin/agent-browser --json install doctor
node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json
/home/ecochran76/.local/bin/agent-browser --json service incidents --summary
```

The installed executable SHA after this publish is:

```text
502f05830dfb756cda44eae7d6bb8c71999dd4ce39ee109eb51ff36136de155a
```

One content-length proxy retry is authorized if incidents remain zero
immediately before the run.

## Viewport-Route Context Probe Failure

The content-length proxy retry artifact is:

```text
/tmp/agent-browser-p46-s10-2026-06-27T22-46-02-732Z
```

This run passed the foreign CDP inventory and tab-list gates. The dashboard
returned the launched raw CDP browser from `/api/sessions`, and
`/api/session-tabs?port=<foreign-port>` returned the normalized
`https://example.org/?p46=s10-foreign-cdp` page.

The run then failed while waiting for the service-owned selected workspace
panel. The workspace viewport was mounted with
`data-selected-workspace-id="browser:session:default"`, a Guacamole iframe,
and the expected service-owned control text, but the optional
`.workspace-selection-panel` detail surface was not mounted on the route used
by the live harness.

The harness now distinguishes detail-panel evidence from selected workspace
context evidence. S10 accepts mounted viewport-route context when the optional
detail panel is absent, falls back to the viewport refresh control when the
detail-panel refresh control is unavailable, and keeps the foreign CDP
read-only and mutation gates backed by the dashboard session capability model.

Additional validation:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
git diff --check -- scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md
```

One viewport-context retry is authorized if incidents remain zero immediately
before the run.

## Global Workspace Text Borrow False Positive

The viewport-context retry artifact is:

```text
/tmp/agent-browser-p46-s10-2026-06-27T22-50-53-053Z
```

This run completed capture and reset-after. It failed only in evaluation with:

```text
foreign selected workspace borrowed service-owned route, stream, or display state
```

The selected foreign workspace evidence showed the expected non-owned CDP
stream route: `daemon-session:<foreign>`, no iframe, no Guacamole frame source,
`cdp screencast / cdp input`, and `ws://127.0.0.1:<foreign-port>/`. The false
positive came from scanning the full dashboard body text, which also includes
the service-owned row in the global workspace list.

The harness now scopes the borrow detector to selected-workspace facts and the
selected viewport text. Global workspace-list text is no longer treated as
evidence that the selected foreign row borrowed the service-owned route.

Additional validation:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
git diff --check -- scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md
```

One borrow-detector retry is authorized if incidents remain zero immediately
before the run.

## Completion

The borrow-detector retry passed:

```text
/tmp/agent-browser-p46-s10-2026-06-27T22-52-43-936Z
```

Evidence:

- foreign browser detected as `ownership: foreign_cdp`, `provider:
  detected-cdp`, and `addressability: cdp_reachable`;
- foreign tab inventory returned through authenticated
  `/api/session-tabs?port=<foreign-port>`;
- selected foreign workspace stayed on
  `daemon-session:detected-s10-foreign-cdp-profile-nnyhwn-38405`;
- `foreignRouteBorrowed: false`;
- `foreignContextStable: true`;
- `serviceControlReady: true`;
- `serviceContextStable: true`;
- `serviceBrowserHasRoute: true`;
- route `guacamole:3`, route-pool entry `guacamole-rdp-a`, and display `:13`
  remained bound to the service-owned browser;
- reset-before and reset-after both ended with zero active incidents.

Warnings are accepted for this pass: the optional selected-workspace detail
panel was not mounted, so S10 used viewport-route context and dashboard session
capabilities as evidence. Mutation and adoption remain intentionally disabled
in the compact dashboard action surface.

P63 is complete. P46 may continue at S11.
