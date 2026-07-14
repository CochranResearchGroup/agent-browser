# P44 Fresh Session Handoff

Date: 2026-06-23
Branch: `main`
Base HEAD at handoff: `13e90376f2d378ba1ec74f1d5230ae61e63148ed`
Goal: continue `execute plan 0044`

## Current State

P44 is still open. The route-bound Facebook one-liner now succeeds through the
installed binary and Guacamole route, but the plan is not complete because the
root-owned privileged helper is stale and cold route desktops still need a
terminal-free proof after an interactive privileged refresh.

The important distinction for the next session:

- Fixed and proven: selected-target readiness on reused tabs, request-scoped
  route-pool persistence, route B convergence from stale `:14` to live `:12`,
  and successful Facebook route-bound open through `rdp_gateway`.
- Still open: root helper install drift and cold desktop proof. The route
  desktop still has old XTerm session evidence until the helper is refreshed
  and route desktops are cold-started.

## Files To Read First

Read these before changing code:

- `AGENTS.md`
- `docs/dev/plans/0044-2026-06-22-rdp-browser-deterministic-refactor-plan.md`
- `docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md`
- `docs/dev/notes/2026-06-22-rdp-browser-determinism-audit.md`
- `docs/dev/notes/2026-06-22-facebook-remote-view-open-friction.md`

Relevant repo policy for the first turn:

- `docs/dev/policies/0009-turn-closeout.md`
- `docs/dev/policies/0010-validation-and-handoff.md`
- `docs/dev/policies/0011-graph-backed-memory-usage.md`
- `docs/dev/policies/0012-codegraph-usage.md`

Use Graphiti discovery at the start of the resumed non-trivial turn:

```bash
~/.local/bin/graphiti-runtime doctor
~/.local/bin/graphiti-runtime discover --group-id agent_browser_main "P44 remote view route-bound RDP browser deterministic refactor helper stale cold desktop proof"
```

Use CodeGraph first for structural source exploration.

## Current Dirty Worktree

There is a large dirty P44 worktree. Do not revert unrelated files. As of this
handoff, notable active files include:

- `cli/src/native/actions.rs`
- `cli/src/native/remote_view.rs`
- `cli/src/native/service_model.rs`
- `cli/src/native/stream/discovery.rs`
- `packages/dashboard/src/components/workspace-navigator.tsx`
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`
- `packages/dashboard/src/lib/service-workspaces.ts`
- `packages/dashboard/src/lib/workspace-viewport-state.ts`
- `scripts/smoke-rdp-guac-route-pool-readiness.js`
- `scripts/smoke-remote-view-open-live.js`
- `scripts/test-route-confusion-gates.js`
- `skills/agent-browser/SKILL.md`

Untracked P44 and P45 artifacts exist and should be preserved:

- `docs/dev/plans/0044-2026-06-22-rdp-browser-deterministic-refactor-plan.md`
- `docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md`
- `docs/dev/contracts/service-remote-view-route-preflight-response.v1.schema.json`
- `scripts/smoke-remote-view-route-preflight-timing.js`
- `scripts/test-rdp-guac-cold-restart-readback-live.js`
- `scripts/test-rdp-guac-postgres-hardening.js`
- `scripts/test-rdp-route-xsession.js`

Run this early:

```bash
git status --short
```

## What Was Fixed In The Latest Slice

The repeated `wrong_tab` failure was caused by reused targets. The planner
selected a target from cached same-origin metadata, but live CDP switching
proved the target was still `about:blank`. Only the open-new path waited for
selected-target URL readiness.

`cli/src/native/actions.rs` now:

- runs selected-target readiness for reused targets too;
- drains CDP target events during the readiness wait;
- switches to the selected target before proving URL readiness;
- navigates the selected target when live readback is still blank;
- scans for a same-origin non-blank target and reselects it when needed;
- persists the updated `serviceTabHandle` after target readback;
- includes target acquisition diagnostics in `operatorVisible.components.tab`;
- persists all request-scoped route-pool entries from fresh `routePool`
  evidence on non-dry-run opens.

This fixed two concrete failures:

- Facebook route-bound open no longer succeeds against a stale `about:blank`
  tab.
- Retained route B no longer keeps stale display `:14` after fresh request
  route-pool evidence says route B is `:12`.

## Latest Validation Evidence

These passed after the selected-target and route-pool persistence patch:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture
cargo test --manifest-path cli/Cargo.toml route_pool -- --nocapture
cargo test --manifest-path cli/Cargo.toml tab_handle -- --nocapture
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm publish:local-dashboard -- --skip-browser --json
git diff --check
```

Published local executable SHA from the final publish:

```text
fc5da74cb9813b4b3eb5453d7ee17465c9ced8ebb781b4cc1b8a28531e7d6b1d
```

Dashboard SHA from the final publish:

```text
42026ca79c74926a43827e9baa6290474597de50dbd48dd65e140852d9e274eb
```

## Latest Live Proof Artifact

The final successful Facebook proof is stored at:

```text
/tmp/agent-browser-p44-facebook-open-latest.json
```

Important fields are under `.data`:

```bash
jq '{
  success: .success,
  status: .data.status,
  routeId: .data.routeId,
  routePoolEntryId: .data.routePoolEntryId,
  operatorVisibleState: .data.operatorVisible.state,
  targetId: .data.tab.targetId,
  title: .data.tab.title,
  url: .data.tab.url,
  publicOperatorUrl: .data.routeDescriptor.publicOperatorUrl
}' /tmp/agent-browser-p44-facebook-open-latest.json
```

Latest readback:

```json
{
  "success": true,
  "status": "opened",
  "routeId": "guacamole:3",
  "routePoolEntryId": "guacamole-rdp-a",
  "operatorVisibleState": "ready",
  "targetId": "67704DA80474C9FE718CB3FBE7D378AF",
  "title": "Facebook",
  "url": "https://www.facebook.com/",
  "publicOperatorUrl": "https://agent-browser.ecochran.dyndns.org/guacamole/#/client/MwBjAHBvc3RncmVzcWw="
}
```

Service-status proof is stored at:

```text
/tmp/agent-browser-p44-service-status-after-final-facebook.json
```

It showed one ready `session:default` browser on profile
`last30days-facebook`, one Facebook tab, route A checked out on `:11`, and
route B available on `:12`.

## Known Remaining Blocker

The root-owned helper was not refreshed because `pnpm install:privileges -- --apply`
requires interactive sudo in this environment. Noninteractive sudo failed with:

```text
sudo: a terminal is required to read the password
```

Until a human runs the privileged install from an interactive shell, doctor is
expected to continue reporting helper drift similar to:

- `remote_view_route_desktop_helper_stale`
- `remote_view_privileged_helper_status_stale`
- helper lacks `status-json`
- helper desktop state `terminal_first_template`

Do not mark P44 complete until helper refresh plus cold desktop proof passes.

## 2026-06-24 Resumption Audit

The current checkout and installed dashboard runtime were rechecked during the
resumed P44 audit. `pnpm publish:local-dashboard -- --skip-browser --json`
rebuilt and restarted the local dashboard runtime successfully. The installed
executable SHA remains
`fc5da74cb9813b4b3eb5453d7ee17465c9ced8ebb781b4cc1b8a28531e7d6b1d`, and
the dashboard bundle SHA is now
`2caf3aca7718add187c9835e2b5020a07ff69b8abe3cf3fb0c8c591785757677`.

Fresh route readiness is green. `scripts/smoke-rdp-guac-route-pool-readiness.js
--report-only` selected route A `guacamole:3` on display `:11` for
`agent-browser-rdp-a` and route B `guacamole:4` on display `:12` for
`agent-browser-rdp-b`, both with ready Guacamole, RDP, and abstract X11 socket
evidence. `scripts/inspect-rdp-route-displays.js` also reported both route
displays present.

The installed helper is still stale and still blocks closeout. Source helper
SHA is `8331be13d7f02bae15c026bfddef24a3cc2e9ba1245720b7851b1c1a3a9385f7`,
while the installed root-owned helper SHA is
`e5bab71e89028c718581c8afb044219658a766dffadcc33a1c8bd28b96b6a336`.
`agent-browser install doctor --json` reports
`remote_view_route_desktop_helper_stale` and
`remote_view_privileged_helper_status_stale`; the helper lacks `status-json`,
and `helperDesktopSession.state` is still `terminal_first_template`.

`pnpm install:privileges -- --apply` still cannot cross the noninteractive
sudo boundary. It reports:

```text
helper: installed helper differs from bundled helper and must be refreshed
sudo: a terminal is required to read the password
sudo: a password is required
```

The cold restart/readback smoke passed, but that smoke only proves service
state and Guacamole route agreement. It is not a terminal-free desktop proof.
`scripts/inspect-rdp-route-displays.js --display-content` currently shows
route A has both Facebook Chromium and XTerm windows, while route B is
`terminal_only` with an XTerm window. `ps` also shows `xterm -title
agent-browser route-pool RDP session` running for both route-specific users.

Current next step is still the interactive privileged refresh, then a cold
route desktop restart and display-content proof that no XTerm or terminal-only
state remains.

## 2026-06-24 Continuation Check

The helper boundary was rechecked after the resumption audit. The installed
root-owned helper is still
`e5bab71e89028c718581c8afb044219658a766dffadcc33a1c8bd28b96b6a336`, while the
source helper remains
`8331be13d7f02bae15c026bfddef24a3cc2e9ba1245720b7851b1c1a3a9385f7`.
`sudo -n true` still fails with `sudo: a password is required`.

The existing sudoers rule permits the installed helper enough for
`sudo -n /usr/local/libexec/agent-browser/agent-browser-privileged-helper
check` to succeed, but the installed helper still rejects `status-json` with
`Unknown command: status-json`. The source helper has the required
`status-json` and terminal-free `.xsession` writer, but copying it into
`/usr/local/libexec/agent-browser/` requires the interactive privileged install
step.

The route users' home directories are not writable or readable by the operator
account: `/home/agent-browser-rdp-a` and `/home/agent-browser-rdp-b` are owned
by their route users with mode `750`, and `.xsession` is not stat-readable from
this session. There is therefore no valid non-root route-template repair path
from this shell.

Current display-content proof still contradicts P44 closeout. Route A on `:11`
shows Facebook Chromium plus an XTerm window. Route B on `:12` is
`terminal_only` with an XTerm window. The next required action remains:

```bash
cd /home/ecochran76/workspace.local/agent-browser
pnpm install:privileges -- --apply
```

After that, cold-start the route desktops and rerun install doctor,
remote-view doctor, display-content inspection, and the final Facebook
dashboard/direct-Guacamole proof.

## Next Recommended Slice

1. From an interactive shell, refresh the privileged helper:

```bash
cd /home/ecochran76/workspace.local/agent-browser
pnpm install:privileges -- --apply
```

2. Confirm install and remote-view doctor convergence:

```bash
agent-browser install doctor --json > /tmp/agent-browser-p44-install-doctor-after-helper.json
agent-browser doctor remote-view --json > /tmp/agent-browser-p44-remote-view-doctor-after-helper.json
jq '{status, remoteControl: .remoteControl, install: .install, routePool: .routePool}' /tmp/agent-browser-p44-remote-view-doctor-after-helper.json
```

3. Cold-start route desktops and prove no terminal-first route state. Use the
existing live smoke or the P44 cold-start readback script, then inspect the
desktop/window proof carefully:

```bash
pnpm test:rdp-guac-cold-restart-readback-live
```

If that script is not wired in `package.json`, inspect and run:

```bash
node scripts/test-rdp-guac-cold-restart-readback-live.js
```

4. Re-run route-pool readiness and the one-liner:

```bash
ROUTE_POOL_JSON="$(node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only | jq -c '.routePoolJson')"
AGENT_BROWSER_RDP_ROUTE_POOL_JSON="$ROUTE_POOL_JSON" \
  agent-browser --json remote-view open https://www.facebook.com/ \
  --runtime-profile last30days-facebook \
  --browser-build stealthcdp_chromium \
  --view-stream-provider rdp_gateway \
  > /tmp/agent-browser-p44-facebook-open-after-helper.json
```

5. Prove the dashboard and direct Guacamole route show the same browser, not a
terminal-only or stale CDP viewport. The previous public operator URL shape was:

```text
https://agent-browser.ecochran.dyndns.org/guacamole/#/client/MwBjAHBvc3RncmVzcWw=
```

6. Only close P44 when every closeout criterion in the plan is proven,
especially:

- `operatorVisible.state=ready`;
- route, display, browser, profile, and selected target agree;
- no terminal-only or terminal-topmost route state;
- dashboard workspace URL without a stale `tab` param renders the RDP browser;
- direct Guacamole URL renders the same browser;
- no active incident, stale retained row, or useless `needs attention` row
  appears in the live left rail.

## Useful Commands

Route-pool readiness:

```bash
node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only | jq
```

Focused Rust gates:

```bash
cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture
cargo test --manifest-path cli/Cargo.toml route_pool -- --nocapture
cargo test --manifest-path cli/Cargo.toml tab_handle -- --nocapture
```

Quality gates after Rust changes:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
git diff --check
```

Publish local dashboard and binary:

```bash
pnpm publish:local-dashboard -- --skip-browser --json
```

Service status compact readback:

```bash
agent-browser --json service status > /tmp/agent-browser-p44-service-status-current.json
jq '.data.service_state | {
  browsers: (.browsers | length),
  routePool: .routePool,
  viewStreams: .viewStreams
}' /tmp/agent-browser-p44-service-status-current.json
```

## Completion Warning

Do not call P44 complete just because the Facebook one-liner returns success.
The final completion proof must include a fresh helper, cold route desktop
readback, dashboard proof, direct Guacamole proof, and left-rail cleanup proof.
The current state is meaningful progress, not full closeout.

## 2026-06-24 Final Closeout

P44 is now complete. The stale privileged-helper blocker was removed, the route
desktops were cold-started onto clean route-specific displays, the final
Facebook one-liner opens through route A, and the live doctor and incident
surfaces are green.

Final helper proof:

- bundled and installed helper SHA match:
  `8331be13d7f02bae15c026bfddef24a3cc2e9ba1245720b7851b1c1a3a9385f7`;
- helper `status-json` reports
  `helperVersion=2026-06-23.p44-route-desktop-v2`,
  `routeDesktopSession.state=browser_control_ready_template`,
  `terminalStartupDetected=false`, and abstract X11 socket support;
- route A is pinned to `:13` and route B is pinned to `:14` in
  `/home/ecochran76/.agent-browser/.env`.

Final live artifacts:

- `/tmp/agent-browser-p44-route-pool-after-publish-close.json`:
  route A `guacamole:3` on `:13` and route B `guacamole:4` on `:14`, both
  ready;
- `/tmp/agent-browser-p44-facebook-open-after-spool-publish.json`:
  `success=true`, `status=opened`, route-pool entry `guacamole-rdp-a`, route
  `guacamole:3`, display allocation `remote-view-display:13`,
  `operatorVisible.state=ready`, URL `https://www.facebook.com/`, and visible
  window proof containing `Facebook - Chromium`;
- `/tmp/agent-browser-p44-display-content-final.json`:
  route A `browser_window_visible`, route B `non_browser_windows`, and no XTerm
  on either route display;
- `/tmp/agent-browser-p44-install-doctor-final.json`:
  `success=true`, no issues, `remoteViewPrivileges.ready=true`,
  `service.ready=true`, `service.timedOut=false`, and runtime convergence
  ready;
- `/tmp/agent-browser-p44-remote-view-doctor-final.json`:
  `success=true`, `status=ready`, no issues, and
  `remoteControl.ready=true`;
- `/tmp/agent-browser-p44-service-status-final-after-resolve.json`:
  route A checked out on `:13`, route B available on `:14`,
  `session:default` healthy on profile `last30days-facebook`, and zero active
  incidents;
- `/tmp/agent-browser-p44-service-incidents-summary-after-resolve.json`:
  no active incidents after resolving stale route-viewer incidents caused by
  the intentional installed-binary refresh close;
- `/tmp/agent-browser-p44-guacamole-route-a-final.html`:
  public Guacamole route fetch returned HTTP 200 through the dashboard login
  entry path.

Final local publish installed executable SHA:

```text
9628c8111540494abc632370259fe5039748a9742ca8af4441473b65885ae3b8
```

Final dashboard SHA:

```text
88e513b00a196f9cb8d53dc658ef23b21d2fb7e53ee6f7c3462146ecb263e4ea
```

Validation passed:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:rdp-route-xsession
pnpm test:dashboard-inspector-actions
pnpm publish:local-dashboard -- --skip-browser --json
git diff --check
```
