# Foreign CDP Browser Discovery And Control Plan

Date: 2026-06-22
State: PLANNED
Lane: P41
Depends On:
- `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`
- `docs/dev/plans/0035-2026-06-15-external-byop-browser-adoption-plan.md`
- `docs/dev/plans/0040-2026-06-21-dashboard-binary-harmonization-plan.md`

## Purpose

Make externally owned Chrome instances with reachable CDP endpoints discoverable
and addressable without pretending agent-browser owns their lifecycle.

The motivating examples are AuraCall and im-receipts. Both can run visible
Chrome windows with reachable DevTools endpoints, but the dashboard live rail
does not show them as addressable non-owned browsers. The desired model is:
agent-browser can inspect, stream, and optionally borrow control of a foreign
browser target while leaving process lifetime, profile ownership, and owning
service semantics with the process owner.

## Current Evidence

Live inspection on 2026-06-22 found:

- AuraCall Chrome launch-owner processes with fixed CDP ports:
  - PID `684111`, profile
    `/home/ecochran76/.auracall/browser-profiles/default/chatgpt`, port
    `45011`, `/json/version` reachable.
  - PID `278036`, profile
    `/home/ecochran76/.auracall/browser-profiles/wsl-chrome-2/chatgpt`, port
    `45013`, `/json/version` reachable.
- im-receipts Google Messages Chrome:
  - PID `7060`, profile
    `/home/ecochran76/.im-receipts/tenants/default/accounts/google_messages/google-messages-main/adapter-state/profile-mirror`,
    launched with `--remote-debugging-port=0`.
  - Its `DevToolsActivePort` resolved to port `37107`, and that port was
    reachable.
- Authenticated `GET /api/sessions` from the local dashboard service returned
  only `install-doctor-service-probe` and `default`; no `detected: true` rows
  reached the dashboard.
- `/proc/<pid>/cmdline` for the foreign Chrome launch-owner processes exposed
  the full Chrome command as one space-separated string rather than normal
  NUL-separated argv entries. Current discovery checks each argv item with
  `starts_with("--user-data-dir")`, so these processes are skipped even though
  the flags are textually present.

Graphiti discovery in group `agent_browser_main` surfaced the existing
service-owned-state direction and the implemented external BYOP adoption lane.
Those are advisory context only. This plan keeps the foreign-CDP lane separate
from explicit BYOP adoption.

## Vocabulary

- **Owned browser**: agent-browser launched or adopted the browser into service
  state and owns lifecycle actions according to the service cleanup contract.
- **Foreign CDP browser**: a Chrome/Chromium process not owned by agent-browser
  but exposing a reachable DevTools endpoint.
- **Observed foreign browser**: a process or profile hint without a reachable
  CDP endpoint.
- **Borrowed control**: an explicit, scoped, time-bounded operator or service
  action that permits mutating CDP operations against a foreign browser target.
- **Lifecycle action**: close browser, kill process, delete profile, release
  profile lease, or otherwise mutate the owning service's process ownership.

## Operating Invariants

- Discovering a foreign CDP browser does not make it agent-browser owned.
- Foreign CDP rows must be clearly labeled as non-owned.
- Read-only inspection and streaming may be available without lifecycle
  ownership.
- Mutating actions require an explicit borrow-control posture.
- Lifecycle actions remain disabled for foreign browsers unless the browser has
  been explicitly adopted through a separate ownership contract.
- Foreign browser discovery must not silently register or reuse profile
  identity lanes for access-plan decisions.
- Dead, stale, and historical foreign browser evidence belongs in Service,
  trace, event, incident, job, and log viewers, not the live left rail.

## Non-Goals

- Do not auto-adopt arbitrary local Chrome processes as `external_byop`.
- Do not close or kill AuraCall, im-receipts, or other foreign browser
  processes.
- Do not treat a reachable CDP endpoint as proof of permission to mutate the
  owning application's workflow.
- Do not store private page contents, cookies, auth state, screenshots, or raw
  browser artifacts in Graphiti or durable plan notes.
- Do not make provider-specific AuraCall or im-receipts selectors part of
  agent-browser core.

## Target UX

The dashboard left rail should show a `Detected non-owned browsers` group for
live foreign CDP browsers. Rows should include:

- process PID, profile path, CDP port, and source confidence;
- page title and URL from `/json/list` when available;
- ownership label such as `Non-owned, CDP reachable`;
- owning-process hints inferred from profile path, process parent, or optional
  registry metadata;
- enabled read-only actions: `Inspect`, `Stream`, `Screenshot`, `Open targets`;
- disabled lifecycle actions with clear reasons: `Close`, `Kill`, and profile
  release are not available for non-owned browsers;
- an explicit `Borrow control` action when mutating operations are supported.

If a foreign browser has no reachable CDP endpoint, it should not appear as a
live addressable browser. It may appear only in a service/log/debug surface as
observed process evidence.

## Target API Shape

Extend the sessions or workspace discovery response with explicit capability
metadata rather than overloading daemon session fields:

```json
{
  "session": "foreign-cdp-chatgpt-45011",
  "detected": true,
  "ownership": "foreign_cdp",
  "addressability": "cdp_reachable",
  "pid": 684111,
  "profilePath": "/home/.../.auracall/browser-profiles/default/chatgpt",
  "cdpPort": 45011,
  "cdpUrl": "http://127.0.0.1:45011",
  "capabilities": {
    "inspect": true,
    "screenshot": true,
    "stream": true,
    "mutateRequiresBorrow": true,
    "lifecycle": false
  },
  "borrow": {
    "state": "not_borrowed",
    "expiresAt": null,
    "owner": null
  }
}
```

The exact final shape can use existing `SessionInfo` fields for compatibility,
but it must preserve the explicit distinction between addressability and
ownership.

## Implementation Slices

### Slice A: Robust Foreign CDP Discovery

Goal: reliably discover reachable non-owned Chrome CDP endpoints.

Deliverables:

- Normalize `/proc/<pid>/cmdline` for both NUL-separated argv and single-string
  command lines.
- Parse `--user-data-dir`, `--remote-debugging-port`, and related flags from
  both forms without ad hoc path truncation.
- Resolve `--remote-debugging-port=0` through
  `<user-data-dir>/DevToolsActivePort`.
- Require a reachable `127.0.0.1:<port>` endpoint and a successful
  `/json/version` or `/json/list` probe before returning a live foreign CDP
  row.
- Deduplicate by CDP browser websocket id or port/profile pair.
- Add source metadata that explains whether the port came from an explicit
  flag, `DevToolsActivePort`, or a fallback parser.

Acceptance:

- Unit tests cover normal argv, single-string cmdline, explicit fixed port,
  `--remote-debugging-port=0`, missing `DevToolsActivePort`, unreachable port,
  renderer/helper process exclusion, and duplicate rows.
- A no-launch fixture proves AuraCall-style and im-receipts-style command lines
  produce detected foreign CDP candidates.

### Slice B: Read-Only Addressability Contract

Goal: make foreign CDP browsers inspectable and streamable without ownership.

Deliverables:

- Add a foreign CDP target model that records `ownership=foreign_cdp`,
  `addressability=cdp_reachable`, process metadata, profile path, and CDP
  endpoint metadata.
- Add read-only endpoints or service request actions for:
  - target list;
  - page title and URL readback;
  - screenshot;
  - DOM or accessibility snapshot;
  - CDP screencast frame stream.
- Route these operations through a bounded foreign-CDP client that never calls
  close, kill, profile release, or service lifecycle operations.
- Redact or cap returned evidence using the same size and privacy discipline as
  service diagnostics.

Acceptance:

- Foreign CDP rows can produce screenshot and target-list evidence in a live
  smoke.
- No lifecycle mutation is available from the foreign CDP path.
- Service-owned browser paths and foreign CDP paths remain distinguishable in
  JSON and UI labels.

### Slice C: Dashboard Live Rail And Detail UX

Goal: make foreign CDP browsers visible as non-owned but addressable live
targets.

Deliverables:

- Render foreign CDP rows under `Detected non-owned browsers`.
- Show capability badges for `CDP reachable`, `Read-only`, and `Borrow
  required for mutation`.
- Enable `Inspect`, `Stream`, and `Screenshot` when corresponding capabilities
  are true.
- Keep `Close`, `Kill`, `Release`, and ownership-changing actions disabled with
  explicit reasons.
- Open the workspace detail panel and viewport for foreign read-only streams.
- Keep stale or no-CDP process evidence out of the live left rail.

Acceptance:

- Dashboard structural tests assert foreign CDP rows are not grouped with
  agent-browser owned browsers.
- A fixture with AuraCall and im-receipts-style rows renders under
  `Detected non-owned browsers`.
- The live rail does not reintroduce a generic `Needs attention` group.

### Slice D: Borrowed Control

Goal: support explicit, scoped mutation without taking lifecycle ownership.

Deliverables:

- Add a `foreign_cdp_borrow_control` or equivalent action with:
  - caller identity;
  - selected target id;
  - allowed operations;
  - TTL;
  - optional reason;
  - clear audit event.
- Mutating operations such as click, type, navigate, and evaluate must check an
  active borrow grant.
- Borrow grants must expire automatically and be visible in Service or Activity
  logs.
- Borrowing must not grant close, kill, profile release, or adoption.

Acceptance:

- Mutating requests are rejected without a borrow grant.
- Mutating requests succeed with a valid borrow grant and are recorded with the
  foreign target id and caller labels.
- Expired borrow grants fail closed.

### Slice E: Optional Explicit Adoption Bridge

Goal: connect foreign CDP discovery to the existing BYOP adoption lane only
when a caller explicitly asks for ownership integration.

Deliverables:

- Offer an `Adopt as BYOP` affordance only when:
  - the operator or caller selects a registered `external_byop` profile;
  - the profile path and CDP endpoint match the foreign browser evidence;
  - the caller supplies required service, agent, task, and target identity.
- Reuse the existing `external_byop_adopt` contract from P35.
- Make adoption visibly change ownership state from `foreign_cdp` to
  `attached_existing` service-owned retained state.

Acceptance:

- Discovery alone never changes access-plan reuse decisions.
- Explicit adoption reuses the P35 validation path and updates access-plan
  reuse only after successful adoption.

### Slice F: Diagnostics, Docs, And Closeout

Goal: make the operating model durable for future agents and operators.

Deliverables:

- Update README, dashboard docs, `skills/agent-browser/SKILL.md`, and CLI help
  if user-facing commands or fields are added.
- Add a troubleshooting section explaining why a visible browser may not appear:
  no CDP, unreachable port, unreadable profile, stale `DevToolsActivePort`,
  renderer-only evidence, or policy-disabled foreign mutation.
- Add live diagnostic output that lists skipped foreign CDP candidates with
  reason codes, without exposing private page content.
- Update dashboard/binary harmonization expectations if new UI feature flags
  are needed, for example `workspace.foreignCdpBrowsers` and
  `workspace.foreignCdpBorrowControl`.

Acceptance:

- Future agents can distinguish owned, foreign addressable, observed-only, and
  adopted BYOP browser state from docs and JSON readback.
- Skipped-candidate diagnostics explain why AuraCall or im-receipts browser
  windows are not shown.

## Validation Matrix

Discovery and Rust contract:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml foreign_cdp -- --nocapture
cargo test --manifest-path cli/Cargo.toml discovery -- --nocapture
```

Dashboard and client:

```bash
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-view-streams
pnpm build:dashboard
```

Live proof:

```bash
pnpm test:foreign-cdp-discovery-live
pnpm smoke:local-dashboard-runtime -- --expect-marker "Detected non-owned browsers"
```

Publication when dashboard-visible behavior changes:

```bash
pnpm publish:local-dashboard -- --skip-browser --expect-marker "Detected non-owned browsers" --json
curl -fsS http://127.0.0.1:4848/api/runtime/manifest | jq .
```

Closeout hygiene:

```bash
git diff --check
pnpm validation:select -- --base HEAD
```

## Done Definition

- AuraCall-style fixed-port Chrome and im-receipts-style
  `DevToolsActivePort` Chrome are discovered as foreign CDP browsers when live.
- The dashboard shows them under `Detected non-owned browsers` with read-only
  addressability.
- Read-only inspect, screenshot, and stream work without lifecycle ownership.
- Mutating operations require an explicit borrow-control grant.
- Close, kill, profile release, and ownership-changing actions remain
  unavailable unless explicit BYOP adoption is requested and succeeds.
- Skipped candidates are diagnosable without private browser artifacts.
- Docs, skill guidance, tests, live smoke, and local dashboard runtime
  publication agree.
