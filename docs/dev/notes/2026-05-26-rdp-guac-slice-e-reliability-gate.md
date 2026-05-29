# RDP Guac Slice E Reliability Gate

Date: 2026-05-26
State: VALIDATED
Plan: `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## Scope

This note records the final Slice E reliability gate for the current RDP and
Guacamole full-control path. It combines the same-day Slice D readiness matrix
with fresh Slice B and Slice C live smokes run in one validation session.

## Environment

- Local time: 2026-05-26 11:06 CDT.
- Branch: `main`.
- Validation base: `HEAD`.
- Dirty state: broad active remote-view lane with Rust, dashboard, docs,
  script, client, plan, and note changes. The Slice E note and plan update are
  part of that lane.
- `AGENT_BROWSER_REMOTE_VIEW_PROVIDER`: `rdp_gateway`.
- `AGENT_BROWSER_REMOTE_VIEW_URL`: redacted public Guacamole client route.
- `AGENT_BROWSER_REMOTE_HEADED_DISPLAY`: `:10`.
- Dashboard client A: `/usr/bin/google-chrome`.
- Dashboard client B: `/usr/bin/brave-browser`.
- Dashboard auth: initialized in each isolated harness `AGENT_BROWSER_HOME`.
- Public ingress: ready by readiness smoke and dashboard live harnesses.

## Live Evidence

Passed:

- `pnpm test:service-dashboard-remote-control-ui-live`
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- `pnpm test:rdp-guac-viewer-transfer-live`
- `pnpm test:rdp-guac-browser-switch-live`

Remote-control UI smoke:

- Browser id: `session:dashboard-remote-control-51957`.
- Tab id: `target:2EF14F0157BAAE3BE00DBDDD1D265328`.
- Browser and tab dialogs loaded the redacted public Guacamole client route.
- Focus jobs:
  `http-service-request-view_focus-96ef4308-4894-4606-b28d-f0842ffea61a`
  and
  `http-service-request-view_focus-069040ea-c374-44c3-91a4-55c61f258b5a`.

Readiness smoke:

- `readiness.status`: `ready`.
- `guacd`: ready, `/usr/sbin/guacd`, TCP `127.0.0.1:4822` reachable.
- `xrdp`: ready, `/usr/sbin/xrdp`, TCP `127.0.0.1:3389` reachable.
- `xrdp_sesman`: ready, `/usr/sbin/xrdp-sesman`.
- Guacamole web app: ready, HTTP 302 from the configured route.

Viewer-transfer live smoke:

- Artifact directory:
  `/tmp/agent-browser-rdp-guac-hardening-2026-05-26T16-04-06-283Z`.
- Browser id: `session:rdp-guac-transfer-54916`.
- Service tab id: `target:82827F6F31DA529DF79ECB75651B3732`.
- Display isolation: `shared_display`.
- Outcome: `simultaneous_view`.
- External-open takeover job:
  `http-service-request-view_takeover-ac41ca4c-32c5-42f6-9c5a-459244a6d512`.
- Screenshots captured: client 1 connected, client 1 after client 2 open,
  client 1 after takeover, client 1 mobile viewport, client 1 after refresh,
  client 2 connected, client 2 after client 1 takeover, and client 2 after
  refresh.
- Service-state samples captured before client 1, after client 2 opens, after
  client 1 takeover, after client 1 refresh, and after client 2 refresh.

Browser-switch live smoke:

- Artifact directory:
  `/tmp/agent-browser-rdp-guac-browser-switch-2026-05-26T16-04-43-083Z`.
- Browser A id: `session:rdp-guac-switch-a-59102`.
- Browser A tab id: `target:FEA0834FB3212FF444EA6688EBF48E39`.
- Browser B id: `session:rdp-guac-switch-b-59102`.
- Browser B tab id: `target:D5022340B6A550367BE995B8BBB05AB0`.
- Display isolation: `shared_display`.
- Cross-browser viewer outcome: `simultaneous_view`.
- External-open takeover job:
  `http-service-request-view_takeover-8236162f-dd14-4bb2-90d9-9aac2fcd9a1c`.
- Screenshots captured: client 1 browser A connected, client 1 switched to
  browser B, client 1 browser B after refresh, client 1 after client 2 opens
  browser A, client 2 browser A connected, four A/B alternation screenshots,
  and client 1 after external open.
- Service-state samples captured after browser launches, after client 1
  switches to B, after client 1 refreshes B, after client 2 opens A, and final
  retained state.

## Viewport Path Matrix

<table>
  <thead>
    <tr>
      <th>Path</th>
      <th>Evidence</th>
      <th>Result</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Desktop dashboard route</td>
      <td><code>test:service-dashboard-remote-control-ui-live</code>, viewer-transfer, browser-switch</td>
      <td><code>connected</code> RDP-backed viewport rendered</td>
    </tr>
    <tr>
      <td>Mobile-width dashboard route</td>
      <td><code>client-1-mobile-viewport.png</code></td>
      <td>same essential controls remained visible</td>
    </tr>
    <tr>
      <td>Iframe route</td>
      <td>dashboard state artifacts with <code>hasFrame=true</code></td>
      <td>Guacamole iframe loaded from the redacted public route</td>
    </tr>
    <tr>
      <td>Popout or external-open route</td>
      <td><code>view_takeover</code> jobs in both live smokes</td>
      <td>service-owned external open path remained deterministic</td>
    </tr>
    <tr>
      <td>Refresh recovery</td>
      <td>client 1 and client 2 refresh screenshots, plus browser B refresh screenshot</td>
      <td>workspace route recovered selected browser and live tab</td>
    </tr>
    <tr>
      <td>Fullscreen control path</td>
      <td>dashboard state artifacts with <code>hasFullscreenButton=true</code></td>
      <td>fullscreen or window control remained exposed on live RDP viewports</td>
    </tr>
  </tbody>
</table>

## Same-Day Slice D Readiness Evidence

Slice E cites the same-day Slice D live note:
`docs/dev/notes/2026-05-26-rdp-guac-slice-d-live-validation.md`.

Slice D artifact directory:

- `/tmp/agent-browser-rdp-guac-readiness-2026-05-26T15-55-20-209Z`

Slice D proved a live healthy RDP and Guacamole viewport, plus rendered
dashboard evidence for auth failure, missing Guacamole connection, ingress
failure, viewer ownership change, browser unavailable, missing stream, and
stale retained focus job recovery. Destructive shared-provider outages remained
fixture-backed by design.

## Final Validation

Passed in this slice or same active plan session:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:service-dashboard-remote-control-ui-live`
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- `pnpm test:rdp-guac-viewer-transfer-live`
- `pnpm test:rdp-guac-browser-switch-live`
- `pnpm validation:select -- --base HEAD`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- focused Rust service-model, remote-headed stream, retained remote-headed,
  persisted service-browser-record, service-access-plan, service-health, and
  service-contract tests
- focused client, dashboard, docs, parity, and readiness script checks
- `git diff --check`

Docs and skill sync:

- `README.md`
- `docs/src/app/commands/page.mdx`
- `docs/src/app/service-mode/page.mdx`
- `docs/src/app/dashboard/page.mdx`
- `skills/agent-browser/SKILL.md`
- installed skill copy at
  `/home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

## Result

Slice E is validated for the current RDP and Guacamole deployment. The live
gate proves backend readiness, public ingress reachability, dashboard
remote-control dialogs, two-client viewer transfer, managed browser A/B
switching, refresh recovery, mobile-width rendering, iframe embedding, external
open, and exposed fullscreen controls.

## Residual Risk And Fallback

- The current Guacamole deployment presents simultaneous-view behavior, so
  natural single-active-viewer disconnect copy remains fixture-backed by Slice
  D rather than observed as a provider-enforced disconnect.
- The run used the shared XRDP display `:10`; private display allocation is
  still a separate hardening item.
- Destructive provider outages were not induced against shared `xrdp`,
  `xrdp-sesman`, `guacd`, Guacamole, dashboard auth, or public ingress.
- If RDP and Guacamole regress under load or private-display requirements, the
  fallback campaign remains CDP streaming first, then VNC/noVNC as the next
  full-control backend family.
