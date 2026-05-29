# RDP Guac Slice D Live Validation

Date: 2026-05-26
State: VALIDATED
Plan: `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## Scope

This note records the Slice D live readiness and failure-state checkpoint for
the RDP and Guacamole path. The checkpoint is intentionally non-destructive: it
uses a live healthy RDP/Guacamole viewport and isolated or fixture-backed
failure payloads rather than stopping shared `xrdp`, `xrdp-sesman`, `guacd`,
Guacamole, dashboard auth, or public ingress services.

## Environment

- Local time: 2026-05-26 10:55 CDT.
- Branch: `main`.
- Validation base: `HEAD`.
- Dirty state: broad active remote-view lane with existing Rust, dashboard,
  docs, script, client, and untracked plan or note files. The Slice D live
  changes are part of that lane.
- `AGENT_BROWSER_REMOTE_VIEW_PROVIDER`: `rdp_gateway`.
- `AGENT_BROWSER_REMOTE_VIEW_URL`: redacted public Guacamole route.
- `AGENT_BROWSER_REMOTE_HEADED_DISPLAY`: `:10`, discovered from the active
  XRDP-backed Xorg display.
- Dashboard client: `/usr/bin/google-chrome`.
- Dashboard auth: initialized in the isolated harness `AGENT_BROWSER_HOME`.
- Artifact directory:
  `/tmp/agent-browser-rdp-guac-readiness-2026-05-26T15-55-20-209Z`.

## Source Fixes Required By The Live Run

- `ViewStream` now preserves optional `readiness` and `remoteReadiness`
  payloads so `/api/service/status` can carry compact stream readiness to the
  dashboard.
- Stale retained `focus_job`, `takeover_job`, `view_focus`, or
  `view_takeover` readiness no longer blocks a stream that has a URL and has
  passed preflight. A recovered stale selected target can now render as
  `stale_target_recovered` with readiness `ready`.
- The guarded Slice D harness now prefers an XRDP `Xorg` display over unrelated
  Xvfb displays when `AGENT_BROWSER_REMOTE_HEADED_DISPLAY` is not set.

## Live Evidence

Passed:

- `pnpm test:rdp-guac-readiness-failures-live`

Healthy baseline inside the harness:

- `node scripts/smoke-rdp-gateway-readiness.js --require-html5-client`
- `readiness.status`: `ready`.
- `guacd`: ready, `/usr/sbin/guacd`, TCP `127.0.0.1:4822` reachable.
- `xrdp`: ready, `/usr/sbin/xrdp`, TCP `127.0.0.1:3389` reachable.
- `xrdp_sesman`: ready, `/usr/sbin/xrdp-sesman`.
- Guacamole web app: ready, HTTP 302 from the configured route.
- Public ingress: ready. Public URL is redacted in this note.

Live browser evidence:

- Browser id: `session:rdp-guac-readiness-97666`.
- Tab id: `target:7C2D32EC01EBB1F7513C703103C28FD5`.
- Display: `:10`.
- Healthy viewport state: `uxState=connected`,
  `readinessStatus=ready`, `readinessAction=none`.
- Screenshot:
  `/tmp/agent-browser-rdp-guac-readiness-2026-05-26T15-55-20-209Z/live-healthy-rdp-guac.png`.

## Evidence Matrix

<table>
  <thead>
    <tr>
      <th>Check</th>
      <th>Evidence class</th>
      <th>Rendered state</th>
      <th>Action</th>
      <th>Artifact</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Healthy RDP and Guacamole viewport</td>
      <td><code>live</code></td>
      <td><code>connected</code>, readiness <code>ready</code></td>
      <td><code>none</code></td>
      <td><code>live-healthy-rdp-guac.png</code></td>
    </tr>
    <tr>
      <td>Dashboard or Guacamole auth failure</td>
      <td><code>fixture-backed</code></td>
      <td>readiness <code>action_required</code></td>
      <td><code>sign_in_again</code></td>
      <td><code>fixture-auth_failure.png</code></td>
    </tr>
    <tr>
      <td>Missing or invalid Guacamole connection</td>
      <td><code>isolated-live</code></td>
      <td>readiness <code>blocked</code></td>
      <td><code>inspect_readiness</code></td>
      <td><code>fixture-guacamole_connection_missing.png</code></td>
    </tr>
    <tr>
      <td>Provider or ingress route refused</td>
      <td><code>fixture-backed</code></td>
      <td>readiness <code>blocked</code></td>
      <td><code>open_externally</code></td>
      <td><code>fixture-provider_ingress_refused.png</code></td>
    </tr>
    <tr>
      <td>Viewer ownership changed</td>
      <td><code>fixture-backed</code></td>
      <td>readiness <code>action_required</code></td>
      <td><code>take_over</code></td>
      <td><code>fixture-viewer_ownership_changed.png</code></td>
    </tr>
    <tr>
      <td>Browser process or CDP unavailable</td>
      <td><code>fixture-backed</code></td>
      <td><code>browser_unavailable</code>, readiness <code>blocked</code></td>
      <td><code>relaunch_browser</code></td>
      <td><code>fixture-browser_unavailable.png</code></td>
    </tr>
    <tr>
      <td>Missing selected stream</td>
      <td><code>fixture-backed</code></td>
      <td><code>provider_unavailable</code>, readiness <code>blocked</code></td>
      <td><code>inspect_readiness</code></td>
      <td><code>fixture-missing_stream.png</code></td>
    </tr>
    <tr>
      <td>Stale retained focus job with later healthy stream</td>
      <td><code>fixture-backed</code></td>
      <td><code>stale_target_recovered</code>, readiness <code>ready</code></td>
      <td><code>none</code></td>
      <td><code>fixture-stale_focus_job_recovered.png</code></td>
    </tr>
  </tbody>
</table>

## Result

Slice D is validated for the current RDP and Guacamole deployment. The live
portion proves backend readiness, public ingress reachability, isolated
dashboard auth initialization, and a rendered healthy RDP-backed workspace
viewport. The fixture-backed portion proves rendered dashboard recovery copy
and actions for failure classes that should not be induced by mutating shared
provider services.

## Residual Risk

- Destructive provider outages were not run. Provider-down, ingress-refused,
  auth-expired, and single-active-viewer disconnect paths that would require
  shared-service mutation remain fixture-backed by design.
- The current deployment still reports simultaneous-view behavior in Slice B
  and Slice C. Single-active-viewer takeover copy is covered by rendered
  fixture evidence rather than a natural provider disconnect from this
  deployment.
- Slice E must repeat or cite same-day Slice D readiness evidence together with
  the Slice B and Slice C live gates before any release handoff calls RDP/Guac
  the supportable first production full-control path.
