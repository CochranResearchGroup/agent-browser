# Service Client Example

This example shows the software-client workflow for agent-browser service mode:

- acquire a broker-selected profile with `acquireServiceLoginProfile`
- register or refresh a login profile with `registerServiceLoginProfile`
- add a retained profile-readiness monitor with `upsertServiceProfileReadinessMonitor`
- ask agent-browser for an access plan with `getServiceAccessPlan`
- request one intent-based service tab with `requestServiceTab`
- read the matching service trace with `getServiceTrace`
- optionally cancel a queued job with `cancelServiceJob`
- keep `serviceName`, `agentName`, and `taskName` attached to both calls

## Dry Run

```bash
pnpm --filter agent-browser-service-client-example dry-run
```

The dry run validates imports and prints the request, access-plan query, and
trace query without contacting a running agent-browser service.

The repo-level live smoke validates the same example against an isolated
daemon and browser session:

```bash
pnpm test:service-client-example-live
```

## Live Run

Start or identify an agent-browser stream port, then pass it as the base URL:

```bash
pnpm --filter agent-browser-service-client-example exec node service-request-trace.mjs \
  --base-url http://127.0.0.1:<stream-port> \
  --url https://example.com \
  --service-name JournalDownloader \
  --agent-name article-probe-agent \
  --task-name probeACSwebsite \
  --site-id example \
  --login-id example \
  --register-profile-id journal-example \
  --register-readiness-monitor
```

You can also set `AGENT_BROWSER_SERVICE_BASE_URL` instead of passing
`--base-url`.

The script prints the access plan, command result, typed tab, title, wait,
viewport, and console result fields, trace counts, and the latest retained jobs
so software projects can confirm that the planning, request, and trace metadata
are connected.

For a recurring service-owned profile, pass `--register-profile-id` with
`--register-readiness-monitor`. The script first asks for the no-launch access
plan. It registers the managed profile and adds a `profile_readiness` monitor
only when agent-browser reports no selected profile, then asks for a refreshed
access plan before submitting the planned tab request. That is the default
integration pattern for non-Canva clients: request by identity, let
agent-browser own profile coordination, and let monitor findings warn when
retained freshness has expired.

## Profile Selection By Login Identity

The normal software-client contract is to name the desired site or login
identity and let agent-browser choose the browser profile. Register profiles
with the target identities they can serve, then request tabs with `loginId`,
`siteId`, or `targetServiceId`.

```js
import { registerServiceLoginProfile } from '@agent-browser/client/service-observability';

await registerServiceLoginProfile({
  baseUrl: 'http://127.0.0.1:4849',
  id: 'journal-acs',
  serviceName: 'JournalDownloader',
  loginId: 'acs',
});
```

When a bounded auth probe has just confirmed usable login state, pass
`readinessState: 'fresh'`, `readinessEvidence`, `lastVerifiedAt`, and
`freshnessExpiresAt` to `registerServiceLoginProfile()`. The helper writes
matching `targetReadiness` rows, and explicit `targetReadiness` rows override
generated rows for the same target identity.
For an existing profile, prefer `updateServiceProfileFreshness()` so the helper
posts to the service-side freshness mutation endpoint. The service merges the
new readiness row under the serialized service-state mutator, preserves
unrelated fields, and updates `authenticatedServiceIds` for fresh, stale, or
blocked targets.

```js
await requestServiceTab({
  baseUrl: 'http://127.0.0.1:4849',
  serviceName: 'JournalDownloader',
  agentName: 'article-probe-agent',
  taskName: 'probeACSwebsite',
  loginId: 'acs',
  url: 'https://example.com',
});
```

The selector prefers profiles whose `authenticatedServiceIds` match the
requested identity, then profiles whose `targetServiceIds` match, then profiles
shared with the calling service. Pass `profile` or `runtimeProfile` only when
you intentionally want to override service-owned selection.
Use `findServiceProfileForIdentity()` from
`@agent-browser/client/service-observability` when a client needs to inspect
the profile collection itself; it returns the selected profile plus the matched
field, identity, and selection reason.
Use `lookupServiceProfile()` when a client wants the common broker read path in
one call: it calls HTTP `GET /api/service/profiles/lookup`, lets agent-browser
apply the authoritative selector, and returns the selected profile, reason,
readiness, and readiness summary. `getServiceProfileForIdentity()` remains as
the older descriptive alias for the same route.
Prefer `acquireServiceLoginProfile()` when the client is deciding whether to
register or seed a managed profile. It calls `getServiceAccessPlan()` first,
registers the fallback profile only when no profile is selected, optionally
adds the standard retained profile-readiness monitor, optionally runs due
readiness monitors when `runDueReadinessMonitor` is true, then returns the
refreshed access plan for the tab request. Set
`runBrowserCapabilityPreflight: true` or pass
`--run-browser-capability-preflight` when the client should also run the final
access plan's no-launch browser-capability gate before browser work. The
generic example output includes `profileAcquisitionSummary` with
`monitorRunDueRan`, `browserCapabilityPreflightRan`, initial recommendation,
refreshed recommendation fields, and latest trace job `controlPlaneMode` plus
`lifecycleOnly` values for operator inspection.

```js
import { requestServiceTab } from '@agent-browser/client/service-request';
import {
  acquireServiceLoginProfile,
} from '@agent-browser/client/service-observability';

const { accessPlan } = await acquireServiceLoginProfile({
  baseUrl: 'http://127.0.0.1:4849',
  serviceName: 'CanvaCLI',
  agentName: 'canva-cli-agent',
  taskName: 'openCanvaWorkspace',
  loginId: 'canva',
  targetServiceId: 'canva',
  registerProfileId: 'canva-default',
  registerAuthenticated: false,
  registerReadinessMonitor: true,
  runDueReadinessMonitor: true,
  runBrowserCapabilityPreflight: true,
});

await requestServiceTab({
  baseUrl: 'http://127.0.0.1:4849',
  accessPlan,
  loginId: 'canva',
  targetServiceId: 'canva',
  url: 'https://www.canva.com/',
});
```

## Managed Profile Broker Recipe

Use `managed-profile-flow.mjs` when a software client needs the CanvaCLI-style
profile-broker pattern:

1. Ask agent-browser for a no-launch access plan with `getServiceAccessPlan()`.
2. Inspect `decision.attention` to decide whether the client should log,
   prompt, invoke a provider, or show an operator-facing affordance.
3. Register a managed profile only when agent-browser has no suitable profile.
4. Add a `profile_readiness` monitor when registering a new recurring profile.
5. Optionally run due profile-readiness monitors when access-plan recommends it.
6. Refresh the access plan before passing it to `requestServiceTab({ accessPlan })`.
7. Optionally run the browser-capability preflight before browser work.
8. Ask the operator to seed the profile when readiness reports `needs_manual_seeding`.

The workflow output includes `accessAttention` plus
`profileAcquisitionSummary.initialAttention` and `refreshedAttention`. These
are compact summaries of the service-owned intervention decision; presentation
remains the caller's responsibility. The examples build
`profileAcquisitionSummary` with `summarizeServiceProfileAcquisition()` so
software clients can reuse the same selected-profile, registration,
due-monitor, browser-preflight, and attention fields without copying example
logic.

Dry-run the recipe without contacting a service:

```bash
pnpm --filter agent-browser-service-client-example managed-profile-dry-run
```

Validate the existing-profile path without launching Chrome or a daemon:

```bash
pnpm test:service-client-managed-profile-flow
```

Validate the due-monitor path against an isolated daemon and browser session:

```bash
pnpm test:service-client-managed-profile-flow-live
```

Run it against a live service when you have a stream port:

```bash
pnpm --filter agent-browser-service-client-example exec node managed-profile-flow.mjs \
  --base-url http://127.0.0.1:<stream-port> \
  --service-name CanvaCLI \
  --agent-name canva-cli-agent \
  --task-name openCanvaWorkspace \
  --login-id canva \
  --target-service-id canva \
  --readiness-profile-id canva-default \
  --run-due-readiness-monitor \
  --run-browser-capability-preflight \
  --url https://www.canva.com/
```

Only add `--register-profile-id canva-default` when the service has no suitable
managed profile or the operator intentionally wants a new account lane. The
script registers with `authenticated: false` by default so readiness can drive
manual seeding instead of pretending a new profile is already signed in.
For Google sign-on, Chrome sync, passkeys, or browser plugin setup, seed the
new profile in headed Chrome before CDP is attached. Do not use `--attachable`
or a remote debugging port for the first Google sign-in. Prefer the default
`basic_password_store` keyring policy so OS keyring prompts do not block later
unattended work.
Add `--register-readiness-monitor` for recurring service-owned profiles. The
script then calls `upsertServiceProfileReadinessMonitor()` after registration
so the service can mark expired freshness stale and expose the result through
access-plan `monitorFindings`.
Add `--run-due-readiness-monitor` when the script should execute an
access-plan-recommended due profile-readiness monitor through
`runServiceAccessPlanMonitorRunDue()` before it requests the tab. The script
then refreshes the access plan and prints `profileAcquisitionSummary` with the
selected profile ID, whether registration happened, whether a due monitor ran,
the initial recommendation, and the refreshed recommendation.
Add `--run-browser-capability-preflight` when the script should also run the
access-plan-recommended no-launch browser-capability gate through
`runServiceAccessPlanBrowserCapabilityPreflight()` before requesting browser
work. The script prints `browserCapabilityPreflightRan`, whether the launch
binding would apply, and the preflight reason in `profileAcquisitionSummary`.
When a readiness row reports `needs_manual_seeding`, the script output includes
`readinessSummary.needsManualSeeding: true` plus the target service IDs and
recommended actions so the client can show operator instructions directly.
The underlying `targetReadiness` row also includes `seedingMode`,
`cdpAttachmentAllowedDuringSeeding`, `preferredKeyring`, and `setupScopes` so
clients can render Google-style detached no-CDP setup requirements without
parsing the recommendation string.
Use the inline `seedingHandoff` from the access plan when a client needs the
exact detached `runtime login` command, setup URL, operator steps, warnings,
and persisted lifecycle record for handoff instead of rebuilding those
instructions itself.
`seedingHandoff.operatorIntervention` is the machine-readable feedback contract
for dashboards, agents, desktop notifications, webhooks, and software clients:
render its severity, channels, completion signals, and actions rather than
inventing separate seeding rules. The managed profile flow skips
`requestServiceTab()` while manual seeding is required and switches to
`requestServiceCdpFreeLaunch()` when the access plan requires CDP-free
operation. The lower-level tab helper also throws before posting
`/api/service/request` unless the caller explicitly passes
`allowManualAction: true`.
Use `updateServiceProfileSeedingHandoff()` when the operator, dashboard, or
supervising agent needs to record detached launch, waiting for close, declared
complete, closed but unverified, verified fresh, failed, or abandoned states.
The summary comes from `summarizeServiceProfileReadiness()` in
`@agent-browser/client/service-observability`, so clients can reuse the same
logic without copying this example.
For trace review, use `summarizeServiceTraceAttention()` on the
`getServiceTrace()` response. It rolls up trace-context attention into required
operator follow-up, service labeling follow-up, reasons, suggested actions,
messages, and affected contexts without copying dashboard logic.

When a client has just completed a bounded auth probe for an existing managed
profile, pass the probe result back to agent-browser instead of editing profile
JSON locally:

```bash
pnpm --filter agent-browser-service-client-example exec node managed-profile-flow.mjs \
  --base-url http://127.0.0.1:<stream-port> \
  --login-id canva \
  --target-service-id canva \
  --freshness-profile-id canva-default \
  --freshness-state fresh \
  --freshness-evidence auth_probe_cookie_present
```

The script calls `updateServiceProfileFreshness()`, which posts to
`POST /api/service/profiles/<id>/freshness` so the service serializes the merge
and updates `authenticatedServiceIds` consistently.

## Post-Seeding Probe Recipe

Use `post-seeding-probe.mjs` after an operator has completed detached CDP-free
profile seeding and closed the seeding browser. The recipe opens a
service-owned tab for the seeded identity, reads the current URL and title with
bounded service requests, evaluates optional expectations, then calls
`verifyServiceProfileSeeding()` so the matching closed handoff moves to
`fresh` or `verification_pending`. Before launching the tab, it performs a
no-launch profile lookup and refuses to verify the profile if the broker would
select a different profile for the requested identity.
Software clients that already called `getServiceAccessPlan()` can use
`runServiceAccessPlanPostSeedingProbe()` from
`@agent-browser/client/service-observability` to run the discovered
`decision.postSeedingProbe` recipe directly, rather than manually copying the
profile ID, target identity, tab request, and freshness fields.

Dry-run the probe plan without launching Chrome:

```bash
pnpm --filter agent-browser-service-client-example post-seeding-probe-dry-run
```

Validate the no-launch mock path:

```bash
pnpm test:service-client-example
```

Validate the same recipe against an isolated live daemon and temporary profile:

```bash
pnpm test:service-client-post-seeding-probe-live
```

Run it against a live service when a stream port is available:

```bash
pnpm --filter agent-browser-service-client-example exec node post-seeding-probe.mjs \
  --base-url http://127.0.0.1:<stream-port> \
  --profile-id google-work \
  --login-id google \
  --target-service-id google \
  --url https://myaccount.google.com/ \
  --expected-url-includes myaccount.google.com \
  --expected-title-includes "Google Account"
```

Pass `--cancel-job-id <job-id>` when your software already knows a queued job
that should be cancelled. The script calls `cancelServiceJob` and prints the
cancellation result alongside the tab request and trace output. Use
`pnpm test:service-job-naming-live` for the repo-local live smoke that creates
and cancels a queued job end to end.
