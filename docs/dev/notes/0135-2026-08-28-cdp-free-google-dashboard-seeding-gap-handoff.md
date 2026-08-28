# CDP-Free Google Dashboard Seeding Gap Handoff

Date: 2026-08-28

Status: OPEN FIELD DEFECT

Lane: unassigned; `0135` is reserved only as this note's deterministic serial

Scope: detached manual login, managed-profile discovery, remote-view route
checkout, and dashboard inventory

Authority: SOURCE ANALYSIS AND DEVELOPMENT-RUNTIME REPAIR ONLY. PRODUCTION
BROWSER, PROFILE, PROVIDER, AND PRESENTATION MUTATION REQUIRE SEPARATE EXACT
AUTHORIZATION.

## Purpose

Continue one bounded Agent Browser repair for initial Google authentication.
The safe no-DevTools browser can launch successfully, including on an existing
RDP display, but the dashboard cannot present it because the route remains
released and the manual browser has no remote-view binding.

This note is provider-neutral and intentionally omits the consuming tenant,
operator identity, credentials, portal URL, spreadsheet identity, and page
content. The field consumer was a contractor-portal Google Sheets OAuth setup.

## Executive Summary

Google's first sign-in must use headed stock Chrome without DevTools. The
existing two-phase workflow remains correct:

1. launch the persistent profile without CDP;
2. let the operator authenticate;
3. close Chrome;
4. relaunch the same profile attachably; and
5. run a bounded authenticated-state probe.

The current installed runtime can perform step 1, but it cannot make that
browser dashboard-visible through the governed RDP route.

Two live attempts reproduced the gap:

- a detached browser on ambient display `:0` appeared in Service Status under
  `manualBrowsers`, but `remoteViewRouteId` and `remoteViewUrl` were null;
- a detached browser on display `:12`, which matched the only available
  Guacamole route-pool entry, still had null remote-view fields after Service
  reconciliation. Route `guacamole:3` remained `released` and its pool entry
  remained `available`.

The dashboard therefore had no governed handoff it could display. Both
browsers were closed by exact PID after the operator reported that no browser
was visible. Final runtime status reported `browserAlive: false`. No Google
navigation, credential entry, consent, authorization-code exchange, token
storage, Sheets API request, or scheduling effect occurred.

## Installed Evidence

Installed generation:

```text
0.28.0-92d2015dd76c-d017d3f4db8a
```

The runtime had already recovered from an earlier workstation transition and
reported one dashboard process, one runtime host, one executable generation,
and no legacy daemon. That recovery is not the defect described here.

The dedicated Chrome-family runtime profile was created successfully:

```text
runtime create <redacted-fulfillment-google-profile> --browser-family chrome
created: true
userDataDir: ~/.agent-browser/runtime-profiles/<redacted-fulfillment-google-profile>/user-data
```

The broker-first access plan still returned:

```text
selectedProfile: null
recommendedAction: register_or_seed_managed_profile
attention.reason: register_or_seed_managed_profile
sitePolicy.profileRequired: true
sitePolicy.manualLoginPreferred: true
browserBuild: stock_chrome
```

This shows a second seam: creating a local runtime profile does not register a
managed Service profile or make it selectable by the access-plan broker.

An older registered operator profile was discoverable, but it had no
`browserBuild` and no `browserCompatibilityEvidence`. Stock-Chrome preflight
therefore failed with:

```text
profile_compatibility_missing_or_blocked
```

The field workflow did not override that gate and did not reuse an unrelated
profile.

## Reproducer

Use a disposable Chrome-family profile and a nonproduction portal or Google
test target. Do not use a real credential in automated tests.

Create the runtime profile:

```bash
agent-browser runtime create <profile-id> --browser-family chrome
```

Launch the required initial manual browser without `--attachable` and pin the
stock Chrome executable so the profile family cannot fall back to Chromium:

```bash
agent-browser \
  --runtime-profile <profile-id> \
  --executable-path /opt/google/chrome/chrome \
  runtime login <test-url>
```

Readback shows a live manual browser with no DevTools and no remote-view route:

```text
browserAlive: true
devtoolsPort: null
devtoolsReachable: false
headed: true
launchMode: manual
display: :0
remoteViewRouteId: null
```

Repeat on a ready RDP display that belongs to an available route-pool entry:

```bash
DISPLAY=:12 agent-browser \
  --runtime-profile <profile-id> \
  --executable-path /opt/google/chrome/chrome \
  runtime login <test-url>

agent-browser service reconcile
agent-browser service status
```

The observed post-reconcile state was:

```text
manualBrowsers[profile].display: :12
manualBrowsers[profile].remoteViewRouteId: null
manualBrowsers[profile].remoteViewUrl: null
manualBrowsers[profile].nextSafeAction: finish_login_then_close_or_relaunch_attachable
remoteViewRoutes[guacamole:3].state: released
routePool[guacamole-rdp-3].state: available
routePool[guacamole-rdp-3].target.displayName: :12
```

The browser process is present on the route's display, but no service-owned
checkout or durable handoff exists. The dashboard cannot claim or prove
operator visibility.

## Contract Conflict

The current safe and visible paths do not meet:

- `runtime login` correctly launches without CDP, but does not acquire a
  remote-view route or publish a durable dashboard handoff;
- `remote-view open` selects, launches, checks, and checks out a route, but its
  operator-visible proof requires a CDP target. Its `--manual-login-launch`
  posture still keeps managed CDP enabled;
- Google can reject first sign-in when DevTools is attached, so using the
  existing route-bound CDP path is not an acceptable workaround;
- merely launching detached Chrome on the display named by a route-pool entry
  does not cause reconciliation to bind or check out that route.

The missing product action is a CDP-free, route-bound manual-seeding handoff.
It must distinguish remote-desktop visibility and input from CDP automation.

## Required Repair Contract

The next implementation packet should satisfy all of these conditions:

1. Access planning selects or registers the exact managed profile before the
   initial login. Local runtime-profile creation alone is not treated as
   Service registration.
2. A manual-seeding action reserves one authorized remote-view route before
   launch and launches stock Chrome without DevTools on that route's bound
   display.
3. The browser is represented as a manual, no-CDP inventory row with exact
   profile, process, display, route, handoff, and launch-record identity.
4. Operator visibility is proven from process-bound desktop evidence and route
   health. It does not require a CDP target in this explicit manual-seeding
   mode.
5. The dashboard exposes the opaque durable `/remote-view/<handoff-id>` route
   and remote-desktop input only when the proof is ready. It does not expose
   CDP actions, DOM inspection, or automation while the browser is detached.
6. A launch record, visible window, route checkout, or browser close never
   marks the target authenticated. Only the bounded post-close probe may
   advance target readiness.
7. Browser close releases the exact route, viewer/controller leases, display
   allocation, profile lease, and cleanup obligation without touching another
   workload.
8. The same profile can then relaunch attachably and complete the existing
   `verify-seeding` workflow.
9. Failure after route reservation is transactional. It closes only the newly
   launched browser when owned, releases only the exact reservation, and
   retains an inspectable receipt.
10. Raw Guacamole URLs, credentials, tokens, account identifiers, screenshots,
    and private page content never enter Service State, logs, fixtures, or
    handoff notes.

## Suggested Implementation Packet

1. Add a provider-free service-request action or extend the existing CDP-free
   launch action with explicit remote-view route reservation and manual-seeding
   semantics.
2. Reuse the access-plan selected profile, browser-build policy, route-pool
   authority, display allocation, lifecycle owner, cleanup obligation, and
   durable-handoff machinery. Do not create a parallel route model.
3. Add a desktop-evidence predicate for a process-bound manual Chrome window on
   the reserved display. Reuse the controlled X11 evidence model where its
   authority matches.
4. Teach Service Status and the dashboard workspace inventory to join the
   manual launch record to the reserved route without inventing a CDP target.
5. Keep the row's control posture explicit: RDP or Guacamole input may be
   available; CDP input and automation are unavailable until post-seeding
   relaunch.
6. Close the seeding handoff when the exact process exits, release its route
   transactionally, and expose the bounded post-close verification command.
7. Update CLI help, README, the shared Agent Browser skill, docs site, generated
   client, HTTP and MCP contracts, and inline documentation if the user-facing
   action or contract changes.

## Regression Scenarios

At minimum, cover these cases:

- no-CDP Google seeding checks out a route before browser launch;
- the launched manual browser appears in dashboard inventory with a durable
  handoff and no CDP action affordances;
- operator-visible proof succeeds from the exact process and desktop scene
  without a CDP target;
- wrong display, wrong process, absent window, stale route, missing X11 socket,
  or unavailable Guacamole returns a typed not-visible result;
- route capacity exhaustion fails before browser launch;
- profile registration, selection, family, and browser compatibility disagree
  before launch and fail closed;
- closing the manual browser releases only its exact route and leases;
- a launch or close race resumes idempotently without duplicate Chrome or a
  duplicate route checkout;
- no readiness transition occurs until post-close authenticated probing;
- attachable relaunch after successful manual seeding uses the same profile and
  preserves the existing Google two-phase workflow;
- unrelated active routes, profiles, browsers, viewers, and controller leases
  remain unchanged.

## Validation Expectations

Start with provider-free fixtures and the isolated development runtime. Do not
use production credentials or production provider mutation for the first
repair packet.

Required validation should include:

- focused Rust tests for service request admission, route reservation,
  lifecycle cleanup, manual-browser projection, and seeding-handoff state;
- API, MCP, schema, and generated-client parity for any changed action;
- focused dashboard workspace and action-surface tests;
- Rust formatting and strict Clippy through `scripts/ci/cargo-safe.sh`;
- `pnpm validation:select -- --base <known-green-ref>` for the complete slice;
- one isolated-development provider acceptance proving
  `operatorVisible.state=ready` without CDP and proving exact cleanup after the
  manual browser exits; and
- a fresh OS process and resource census after the live development smoke.

Record browser acquisition, profile identity, presentation, authenticated
readiness, route cleanup, and installed-runtime state as separate proof axes.

## Hard Stops

- Do not attach DevTools during the initial Google sign-in.
- Do not use `AGENT_BROWSER_ALLOW_PROFILE_BROWSER_MISMATCH=true` to bypass
  profile-family safety.
- Do not add compatibility evidence merely to make a preflight pass. Require a
  source-backed browser-family and executable observation.
- Do not reuse an unrelated authenticated profile.
- Do not claim dashboard visibility unless the returned
  `operatorVisible.state` is `ready`.
- Do not claim authentication from browser visibility, route checkout, or
  manual process existence.
- Do not publish a raw Guacamole URL. Return only the opaque durable handoff.
- Do not alter P134's active principal, lease, crash, or installation slice in
  order to absorb this defect without an explicit roadmap decision.
- Do not modify the installed production runtime or presentation provider
  without separate exact authority.

## Existing Authorities

- `docs/dev/notes/google-runtime-profile-login.md` proves and defines the
  two-phase no-DevTools then attachable Google workflow.
- `docs/dev/notes/2026-04-16-google-runtime-profile-live-test-report.md`
  records its live validation.
- `docs/dev/notes/2026-07-25-profile-discovery-and-manual-browser-launch-ux.md`
  already requires detached manual browsers to appear in workspace inventory.
- `docs/dev/notes/0133-2026-08-25-operator-visible-window-focus-gap-handoff.md`
  defines stronger operator-visible proof for route-bound browsers.
- `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`
  governs presentation capacity and desktop evidence.
- `docs/dev/plans/0131-2026-08-25-controlled-x11-desktop-provider-foundation-acceptance-plan.md`
  governs controlled X11 provider behavior.
- `docs/dev/plans/0134-2026-08-26-crash-epoch-and-profile-lifecycle-coherence-plan.md`
  owns current principal, lease, crash, and installation work and must remain a
  separate active lane.

## Suggested Skills

- `agent-browser-service` for access-plan, profile, route, durable-handoff, and
  cleanup boundaries.
- `codegraph-workspace` for tracing `runtime login`, CDP-free service launch,
  `remote-view open`, manual-browser projection, route checkout, and dashboard
  inventory without re-reading the repo through grep loops.
- `diagnosing-bugs` for the provider-free failing fixture and root-cause
  separation.
- `tdd` for freezing the no-CDP operator-visible contract before repair.
- `define-architecture` if the action boundary requires a new service contract
  rather than an extension of the existing CDP-free launch path.

## Best Next Action

Open one short-lived development worktree from a reviewed clean baseline,
leaving the current dirty `main` P134 work untouched. First write a
provider-free failing fixture that reserves an available RDP route, launches a
no-CDP manual browser on its display, and demonstrates that the current
dashboard projection still lacks a durable operator-visible handoff. Then
repair the smallest shared contract that makes that fixture pass without
weakening Google sign-in safety or route ownership.
