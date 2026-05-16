# API Contracts

This directory holds machine-readable schemas for service API records that
software clients and MCP agents are expected to consume directly.

## Service Job Record v1

`service-job-record.v1.schema.json` describes service job records returned by:

- HTTP `GET /api/service/jobs`
- HTTP `GET /api/service/jobs/<id>`
- MCP `agent-browser://jobs`
- service trace job arrays

The schema is guarded by Rust tests for the model, HTTP, and MCP surfaces. Keep
new contractual job fields in this schema before relying on them from external
clients.

Retained job records preserve the caller's target identity hints. Singular
`targetServiceId`, `siteId`, and `loginId` fields keep the exact singular hints
when present, while `targetServiceIds` contains the normalized target-service,
site, and login identity set used by profile selection. This lets HTTP, MCP, and
trace clients explain why a request targeted a particular profile without
replaying the original command payload.

`service-jobs-response.v1.schema.json` describes the response envelope returned
by:

- CLI `agent-browser service jobs`
- HTTP `GET /api/service/jobs`
- HTTP `GET /api/service/jobs/<id>`

The schema covers list and detail responses, including the returned job array,
count, matched and total counters, and the detail-only `job` field.

## Service Request v1

`service-request.v1.schema.json` describes the service request intent object
accepted by:

- HTTP `POST /api/service/request`
- MCP `service_request` `arguments`

The schema requires only `action` for compatibility, but callers should include
`serviceName`, `agentName`, and `taskName` when known so retained jobs remain
traceable. Target hints such as `siteId`, `loginId`, and `targetServiceId`
drive profile selection for the requested site or login scope. `profileLeasePolicy`
can be `reject` or `wait`; `wait` uses `profileLeaseWaitTimeoutMs` to bound how
long the service request waits for another exclusive profile lease to release.

`service-request-mcp-tool-call.v1.schema.json` describes the MCP `tools/call`
wrapper for invoking `service_request` with the same intent object.

HTTP `GET /api/service/contracts` and MCP `agent-browser://contracts` expose
runtime compatibility metadata for these contract IDs, their shared `v1`
version, route and tool names, and the supported service request action list.
The HTTP contracts metadata also advertises `serviceProfileAllocationResponse`
for `GET /api/service/profiles/<id>/allocation` and MCP
`agent-browser://profiles/{profile_id}/allocation`,
`serviceProfileReadinessResponse` for `GET /api/service/profiles/<id>/readiness`
and MCP `agent-browser://profiles/{profile_id}/readiness`, and
`serviceProfileLookupResponse` for `GET /api/service/profiles/lookup` and MCP
`agent-browser://profiles/lookup{?serviceName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,accountId,accountIds,url,readinessProfileId,browserBuild}`,
plus
`serviceAccessPlanResponse` for `GET /api/service/access-plan`, MCP
`service_access_plan`, and MCP `agent-browser://access-plan`, and
`serviceBrowserCapabilityPreflightResponse` for HTTP
`GET /api/service/browser-capability/preflight`, MCP
`service_browser_capability_preflight`, and
`getServiceBrowserCapabilityPreflight()`.
Readiness, lookup, and access-plan metadata also names the
`@agent-browser/client/service-observability` helpers that consume those
routes. Software clients should prefer `lookupServiceProfile()` when they want
agent-browser to select by `serviceName` plus `loginId`, `siteId`,
`targetServiceId`, `accountId`, or `url`; the selector advertises the same
preference order used by service launches: authenticated target state, account
identity, target scope, then shared caller
service. Software clients should prefer `getServiceAccessPlan()` when they need
the broader no-launch recommendation that combines the selected profile,
readiness summary, profile-readiness monitor findings, matching site policy,
enabled providers, retained challenges, advisory browser capability registry
evidence, and the service-owned decision before requesting browser control.
Access-plan `browserCapabilityEvidence` is filtered to the planned browser
build, selected profile, target identities, account identities, and caller
labels. It remains advisory, but preference bindings can set the access-plan
browser build recommendation when no explicit request, site policy, or profile
browser build has already won. In that case it reports `routingApplied: true`
with `routingScope: "access_plan_recommendation"`. The queued launch path may
apply the matching local executable only after host ownership, executable
existence, profile compatibility, and validation evidence gates pass.
Access-plan
`monitorFindings` reports active
`profile_readiness` monitor incidents for the requested target identities, and
`decision.monitorAttentionRequired` mirrors whether those findings need
operator or probe attention before trusting the profile. It also reports
matching active profile-readiness monitors that are due or never checked through
`profileReadinessProbeDue`, `profileReadinessDueMonitorIds`,
`profileReadinessNeverCheckedMonitorIds`, and `dueTargetServiceIds`; those
findings set `decision.monitorProbeDue` and recommend
`run_due_profile_readiness_monitor` before the caller relies on retained
freshness. `decision.monitorRunDue` is the copyable recipe for that action,
covering HTTP `POST /api/service/monitors/run-due`, MCP
`service_monitors_run_due`, CLI `agent-browser service monitors run-due`, and
`runServiceAccessPlanMonitorRunDue()`. Lookup and access-plan
responses also include `seedingHandoff` when readiness requires manual profile
seeding, so clients can show the detached runtime-login command without making
a second profile-specific call. The seeding handoff response includes
`operatorIntervention`, the canonical user-feedback contract for dashboards,
agents, optional desktop popups, webhooks, and software clients. That block
describes severity, notification channels, lease blocking, completion signals,
and safe or dangerous actions; notification providers should render it instead
of creating their own profile-seeding state machine.

## Service Read Surface Parity

The guarded service read surface has MCP resource parity. Software clients can
use HTTP helpers, while agents can use the matching MCP resources without
launching Chrome or bypassing the service-owned state model. Prefer the
access-plan resource for agent-facing profile selection because it combines the
selected profile, readiness, policy, providers, retained challenges, monitor
findings, and the service-owned decision before the caller requests a tab.

## Browser Capability Registry Draft

`service-browser-capability-registry.v1.schema.json` describes the advisory
browser capability registry that service config may carry as
`service.browserCapabilityRegistry`. The merged service state exposes configured
records under `service_state.browserCapabilityRegistry` in CLI
`agent-browser service status --json`, HTTP `GET /api/service/status`, and
state-bearing MCP/HTTP payloads that serialize the service state. This is a
no-launch inventory and evidence surface for browser hosts, executables,
capabilities, profile compatibility, preference bindings, and validation
evidence. `POST /api/service/browser-capability-registry/<collection>/<id>`,
MCP `service_browser_capability_registry_upsert`, and
`upsertServiceBrowserCapabilityRegistryRecord()` upsert one advisory registry
record through the service worker queue. The registry remains advisory and is
not yet authoritative routing policy.

`service-browser-capability-preflight-response.v1.schema.json` describes the
no-launch preflight response for evaluating whether a requested browser build
would pass the same executable, host, profile compatibility, and validation
evidence gates that queued launches use. It is exposed through HTTP
`GET /api/service/browser-capability/preflight`, MCP
`service_browser_capability_preflight`, and
`getServiceBrowserCapabilityPreflight()`. The preflight path relays through the
daemon queue but reports `wouldLaunch: false`, so clients can inspect launch
posture without starting Chrome.
Access-plan responses also include `decision.browserCapabilityPreflight` with a
copyable HTTP, MCP, CLI, and client-helper recipe. Use
`runServiceAccessPlanBrowserCapabilityPreflight()` when a software client wants
to run the recipe returned by `getServiceAccessPlan()` directly.

<table>
  <thead>
    <tr>
      <th>HTTP route</th>
      <th>MCP resource</th>
      <th>Primary use</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>GET /api/service/contracts</code></td>
      <td><code>agent-browser://contracts</code></td>
      <td>Compatibility metadata and service request action support</td>
    </tr>
    <tr>
      <td><code>GET /api/service/access-plan</code></td>
      <td><code>service_access_plan</code> or <code>agent-browser://access-plan{?serviceName,agentName,taskName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,accountId,accountIds,url,sitePolicyId,challengeId,readinessProfileId,browserBuild}</code></td>
      <td>Preferred no-launch selector and recommendation payload</td>
    </tr>
    <tr>
      <td><code>GET /api/service/browser-capability-registry</code></td>
      <td><code>agent-browser://browser-capability-registry</code></td>
      <td>Advisory no-launch browser host, executable, capability, profile compatibility, preference binding, and validation evidence registry</td>
    </tr>
    <tr>
      <td><code>GET /api/service/browser-capability/preflight</code></td>
      <td><code>service_browser_capability_preflight</code></td>
      <td>No-launch browser build routing and executable gate preflight</td>
    </tr>
    <tr>
      <td><code>GET /api/service/profiles/lookup</code></td>
      <td><code>agent-browser://profiles/lookup{?serviceName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,accountId,accountIds,url,readinessProfileId,browserBuild}</code></td>
      <td>Narrow profile selector when the caller does not need the full access plan</td>
    </tr>
    <tr>
      <td><code>GET /api/service/profiles/&lt;id&gt;/readiness</code></td>
      <td><code>agent-browser://profiles/{profile_id}/readiness</code></td>
      <td>One profile's no-launch target readiness</td>
    </tr>
    <tr>
      <td><code>GET /api/service/profiles/&lt;id&gt;/allocation</code></td>
      <td><code>agent-browser://profiles/{profile_id}/allocation</code></td>
      <td>One profile's lease, holder, conflict, and readiness state</td>
    </tr>
    <tr>
      <td><code>GET /api/service/profiles/&lt;id&gt;/seeding-handoff</code></td>
      <td><code>agent-browser://profiles/{profile_id}/seeding-handoff{?targetServiceId,siteId,loginId}</code></td>
      <td>Operator-ready detached seeding command and lifecycle state</td>
    </tr>
    <tr>
      <td><code>GET /api/service/profiles</code>, <code>sessions</code>, <code>browsers</code>, <code>tabs</code>, <code>monitors</code>, <code>site-policies</code>, <code>providers</code>, <code>challenges</code>, <code>jobs</code>, <code>events</code>, and <code>incidents</code></td>
      <td><code>agent-browser://profiles</code>, <code>sessions</code>, <code>browsers</code>, <code>tabs</code>, <code>monitors</code>, <code>site-policies</code>, <code>providers</code>, <code>challenges</code>, <code>jobs</code>, <code>events</code>, and <code>incidents</code></td>
      <td>Service-owned collections for agents and software clients</td>
    </tr>
    <tr>
      <td><code>GET /api/service/incidents/&lt;id&gt;/activity</code></td>
      <td><code>agent-browser://incidents/{incident_id}/activity</code></td>
      <td>Incident timeline detail</td>
    </tr>
  </tbody>
</table>

`packages/client/src/service-request.generated.d.ts` and
`packages/client/src/service-request.generated.js` are generated from these
schemas. Run `pnpm generate:service-client` after changing the schemas and
`pnpm test:service-client-contract` to verify the generated client surface is
current. Run `pnpm test:service-client-types` to type-check the runtime helper
against those declarations.

`packages/client/src/service-observability.generated.d.ts` and
`packages/client/src/service-observability.generated.js` are generated from the
service browser capability registry, job, event, incident, incident activity, and trace schemas. The
`@agent-browser/client/service-observability` helper reads those HTTP endpoints
and returns the generated response types, including `getServiceContracts` for
the runtime compatibility metadata endpoint,
`getServiceBrowserCapabilityRegistry` for the advisory browser registry, and
`getServiceMonitors` for the retained monitor collection.

## Draft Browser Capability Registry v1

`service-browser-capability-registry.v1.schema.json` is a draft advisory
runtime contract for browser inventory and future routing. It is exposed by
service state, HTTP `GET /api/service/browser-capability-registry`, MCP
`agent-browser://browser-capability-registry`, and the generated
`getServiceBrowserCapabilityRegistry()` software-client helper, but it is not
yet authoritative routing policy.

The draft separates current access-plan browser posture labels from future
executable placement. Current posture labels such as `stock_chrome`,
`stealthcdp_chromium`, and `cdp_free_headed` continue to describe what a site
or account needs. Future registry records will describe which host,
executable, capability, profile compatibility rule, and preference binding can
satisfy that posture.

The draft record groups are:

- `browserHosts`: local, Windows-hosted, Docker, cloud, or attached browser
  owners
- `browserExecutables`: system installs, configured paths, promoted artifacts,
  or attached browser identities
- `browserCapabilities`: CDP, CDP-free launch, extension, passkey, headed,
  headless, streaming, profile-lock, and keyring behavior
- `profileCompatibility`: rules that prevent accidental profile reuse across
  incompatible browser families, vendors, operating systems, keyrings, or
  extension postures
- `browserPreferenceBindings`: future primary-browser routing for a site,
  account identity, service, or task
- `validationEvidence`: retained smoke evidence for launch, CDP attach,
  CDP-free launch, extension availability, profile reuse, streaming, and
  site-specific reliability

Do not use the registry as unrestricted launch policy. Access-plan
recommendation code may consume `browserPreferenceBindings` for browser build
selection when no explicit request, site policy, or profile browser build has
already won. The launch path may apply a matching local executable from that
planned request only when the host is local, reachable, and agent-browser owned,
the executable exists, the selected profile has a positive compatibility row
when a profile is selected, and matching validation evidence has passed.
The launch path records `browserCapabilityLaunch` on the persisted session and
the `browser_launch_recorded` event details, including `applied`, `reason`,
`browserBuild`, and selected binding or executable IDs when available.

`examples/browser-capability-registry.sample.json` shows the intended shape for
a local Linux `stealthcdp_chromium` default plus a future Windows
`stock_chrome` primary binding for one site/account identity. The sample is
illustrative and should not be read as live host inventory.

Run `pnpm test:browser-capability-registry-draft` to parse the draft schema and
sample fixture, check required collections, enforce unique IDs, and verify that
host, executable, and capability cross-references resolve. This is intentionally
not a full JSON Schema validator.

## Service Incident Record v1

`service-incident-record.v1.schema.json` describes grouped service incident
records returned by:

- HTTP `GET /api/service/incidents`
- HTTP `GET /api/service/incidents/<id>`
- MCP `agent-browser://incidents`
- MCP `service_incidents`
- service trace incident arrays

The schema is guarded by Rust tests for the model, HTTP, MCP, and service trace
surfaces. Keep new contractual incident fields in this schema before relying on
them from external clients.

`service-incidents-response.v1.schema.json` describes the response envelope
returned by:

- CLI `agent-browser service incidents`
- HTTP `GET /api/service/incidents`
- HTTP `GET /api/service/incidents/<id>`
- MCP `service_incidents`

The schema covers list and detail responses, including the returned incident
array, count, matched and total counters, list filters, and detail-only related
events and jobs.

When changing incident summary grouping or filters, run both no-launch guards:
`pnpm test:service-incident-summary-http` and
`pnpm test:service-incident-summary-mcp`. Together they verify that HTTP
`summary=true` and MCP `service_incidents` with `summary: true` preserve the
same grouped remedy contract across state, severity, escalation,
handling-state, browser, profile, session, service, agent, task, and since
filters.
Run `pnpm test:service-remedies-cli-no-launch` when changing CLI remedy text
rendering. It seeds persisted incidents, runs `agent-browser service remedies`,
and verifies the operator ladder includes affected browser IDs and batch apply
commands without launching Chrome.
Run `pnpm test:service-remedies-json-no-launch` when changing JSON remedy
output. It verifies `agent-browser --json service remedies` preserves grouped
browser IDs, incident IDs, and remedy apply commands without launching Chrome.
Run `pnpm test:service-remedies-apply-json-no-launch` when changing remedy
apply output or browser retry behavior. It verifies
`agent-browser --json service remedies apply --escalation monitor_attention`,
`--escalation browser_degraded`, and `--escalation os_degraded_possible` return
batch apply responses and update persisted monitor or browser state without
launching Chrome.

Failed service monitors derive incidents with `monitor_attention` escalation.
Monitor targets include `url`, `tab`, `site_policy`, and `profile_readiness`.
The `profile_readiness` target checks retained no-launch target readiness, marks
expired fresh rows stale, removes the expired target from
`authenticatedServiceIds`, and records `staleProfileIds` in the monitor event
without launching Chrome. Incident records include `monitorId`, `monitorTarget`,
and `monitorResult`, and summary groups include `browserIds`, `monitorIds`,
`monitorResetCommands`, and `remedyApplyCommand` so operator and dashboard
surfaces can show affected browsers, failed probes, and batch apply commands
without expanding raw events.
`service-monitor-triage-response.v1.schema.json` describes the serialized
monitor triage response returned by CLI `service monitors triage <id>`, HTTP
`POST /api/service/monitors/<id>/triage`, MCP `service_monitor_triage`, and
`triageServiceMonitor()`. It acknowledges the related monitor incident and
resets reviewed failure counts in one service-owned operation.
`service-remedies-apply-response.v1.schema.json` describes the grouped remedy
apply response returned by CLI `service remedies apply --escalation
monitor_attention`, HTTP `POST /api/service/remedies/apply`, MCP
`service_remedies_apply`, and `applyServiceRemedies()`. It supports
`monitor_attention` by applying the same monitor triage operation to each active
failed monitor through the service worker. It supports `browser_degraded` after
operator review by batching retry enablement for active degraded-browser
incidents. It also supports `os_degraded_possible` after host inspection by
batching the existing faulted-browser retry remedy for active
OS-degraded-possible incidents.

## Service Event Record v1

`service-event-record.v1.schema.json` describes retained service event records
returned by:

- HTTP `GET /api/service/events`
- MCP `agent-browser://events`
- service trace event arrays

The schema is guarded by Rust tests for the model, HTTP, MCP, and service trace
surfaces. Keep new contractual event fields in this schema before relying on
them from external clients.

`service-events-response.v1.schema.json` describes the response envelope
returned by:

- CLI `agent-browser service events`
- HTTP `GET /api/service/events`

The schema covers the returned event array plus count, matched, and total
counters for filtered event list consumers.

## Service Collection Records v1

`service-status-response.v1.schema.json` describes the full service status
response returned by `agent-browser service status` and HTTP
`GET /api/service/status`. It includes the derived `profileAllocations` view so
software clients can consume profile holder, waiting job, conflict, lease
state, browser ownership, browser health, and recommended-action data without
reconstructing it from raw state.
Its `launchConfig.profileSmoke` field reports whether the WSL Windows
`chromium-stealthcdp` profile-write smoke is currently applicable and provides
the repo command plus machine-readable reason.

The service collection record schemas describe compact records returned by HTTP
collection APIs and the matching MCP resources:

- `service-profile-record.v1.schema.json`
- `service-browser-record.v1.schema.json`
- `service-session-record.v1.schema.json`
- `service-tab-record.v1.schema.json`
- `service-monitor-record.v1.schema.json`
- `service-site-policy-record.v1.schema.json`
- `service-provider-record.v1.schema.json`
- `service-challenge-record.v1.schema.json`

These schemas cover `profiles`, `browsers`, `sessions`, `tabs`, `monitors`,
`sitePolicies`, `providers`, and `challenges` records. Monitor records are
currently service-owned state for recurring heartbeat, site-policy, tab, and
login freshness probes; scheduling and mutation workflows are separate service
roadmap slices. Profile records include derived `targetReadiness` rows for
no-launch target-service readiness. Google targets without authenticated
evidence report `needs_manual_seeding`, `seedingMode:
"detached_headed_no_cdp"`, `cdpAttachmentAllowedDuringSeeding: false`,
`preferredKeyring: "basic_password_store"`, and setup scopes for sign-in,
Chrome sync, passkeys, and browser plugins. Once a managed profile lists the
target in `authenticatedServiceIds`, readiness changes to
`seeded_unknown_freshness` and access-plan no longer treats first-login seeding
as a required manual action. Explicit `fresh`, `stale`, and
`blocked_by_attached_devtools` rows, plus rows with `lastVerifiedAt` or
`freshnessExpiresAt`, are preserved through derived readiness refreshes so
software clients can record bounded-probe freshness without losing it on the
next profile update. They are guarded by Rust model tests and MCP
resource tests so software clients can consume the same camelCase record fields
from HTTP and MCP without inferring Rust internals.

The matching collection response schemas cover the compact collection envelopes
returned by CLI, HTTP, and MCP resources:

- `service-profiles-response.v1.schema.json`
- `service-browsers-response.v1.schema.json`
- `service-sessions-response.v1.schema.json`
- `service-tabs-response.v1.schema.json`
- `service-monitors-response.v1.schema.json`
- `service-site-policies-response.v1.schema.json`
- `service-providers-response.v1.schema.json`
- `service-challenges-response.v1.schema.json`

These schemas guard the collection array field and `count` field. The profiles
response also includes `profileSources` so clients can distinguish config,
runtime-observed, and persisted profile provenance. It also includes the same
derived `profileAllocations` view returned by service status and the MCP
profiles resource. Profile allocation rows include the same `targetReadiness`
rows as the profile record so detail clients do not have to join back to the
full profile collection.

Monitor collection consumers can read the same retained monitor records through
CLI `agent-browser service monitors`, HTTP `GET /api/service/monitors`, MCP
`agent-browser://monitors`, or the `getServiceMonitors()` client helper. Active
monitors are checked by the daemon scheduler when due. Monitor records retain
`lastCheckedAt`, `lastSucceededAt`, `lastFailedAt`, `lastResult`, and
`consecutiveFailures` so operators and clients can distinguish a recovered
heartbeat from a repeated failure without scanning the event log. CLI and HTTP
monitor collections support `state`, failed-only, and summary filters; MCP
`agent-browser://monitors` remains the full unfiltered retained resource.
`service-monitor-run-due-response.v1.schema.json` describes the immediate
run summary returned by CLI `agent-browser service monitors run-due`, HTTP
`POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`, and
`runDueServiceMonitors()`. The response includes aggregate counts and a
per-monitor `results` list with target, outcome, result string, check
timestamp, and stale profile IDs affected by profile-readiness expiry.
`service-monitor-state-response.v1.schema.json` describes the state update
response returned by CLI `service monitors pause <id>` and
`service monitors resume <id>`, plus the reviewed-failure reset returned by
CLI `service monitors reset <id>`. The same contract covers HTTP monitor
pause/resume/reset routes, MCP `service_monitor_pause`,
`service_monitor_resume`, `service_monitor_reset_failures`, and the matching
client helpers.

`service-profile-allocation-response.v1.schema.json` describes the response
envelope returned by HTTP `GET /api/service/profiles/<id>/allocation` and MCP
`agent-browser://profiles/{profile_id}/allocation` when a caller needs one
derived profile allocation row without fetching the full profile collection.

`service-profile-readiness-response.v1.schema.json` describes the response
envelope returned by HTTP `GET /api/service/profiles/<id>/readiness` and MCP
`agent-browser://profiles/{profile_id}/readiness` when a software client or
agent needs one profile's no-launch target-readiness rows without fetching
allocation details or the full profile collection.

`service-profile-seeding-handoff-response.v1.schema.json` describes the
operator-ready handoff returned by HTTP
`GET /api/service/profiles/<id>/seeding-handoff` and MCP
`agent-browser://profiles/{profile_id}/seeding-handoff{?targetServiceId,siteId,loginId}`.
The handoff is derived from `targetReadiness` and includes the exact detached
`runtime login` command, setup URL, seeding mode, keyring preference, setup
scopes, operator steps, and warnings. It also includes the persisted lifecycle
record that drives `operatorIntervention.state` after the detached browser is
launched, waiting for close, declared complete, closed but unverified, verified
fresh, failed, or abandoned. Use `targetServiceId`, `siteId`, or `loginId`
query parameters when a profile has multiple target identities.

`POST /api/service/profiles/<id>/freshness` and MCP
`service_profile_freshness_update` reuse the profile upsert response envelope
after merging bounded-probe freshness evidence into one persisted profile.
When a matching seeding handoff is already closed but unverified, the same
freshness update records the post-close probe result by moving the handoff to
`verification_pending` or `fresh`. The CLI `service profiles <id>
verify-seeding <target>` command and service client
`verifyServiceProfileSeeding()` helper are thin wrappers over this same
serialized freshness mutation.
`POST /api/service/profiles/<id>/seeding-handoff` and
`updateServiceProfileSeedingHandoff()` persist CDP-free seeding lifecycle
updates through the same serialized service-state mutator. Non-attachable
`runtime login` records the seeding browser PID when the runtime profile maps
to a known manual-seeding target, and later runtime or service reads mark the
handoff closed but unverified once that PID exits.

`service-profile-lookup-response.v1.schema.json` describes the response
envelope returned by HTTP `GET /api/service/profiles/lookup` and MCP
`agent-browser://profiles/lookup{?serviceName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,accountId,accountIds,url,readinessProfileId,browserBuild}`
when a caller wants agent-browser to apply the authoritative service profile
selector for a service name plus site, login, account, or URL identity without fetching the
full profile collection. `selectedProfileMatch` includes `matchedField` and
`matchedIdentity` so clients can explain whether the match came from
`authenticatedServiceIds`, `accountIds`, `targetServiceIds`,
`sharedServiceIds`, or `browserBuild`. When
`readinessSummary.manualSeedingRequired` is true, `seedingHandoff` contains the
same operator-ready command and warnings as the explicit profile seeding
handoff endpoint.

`service-access-plan-response.v1.schema.json` describes the response envelope
returned by HTTP `GET /api/service/access-plan`, MCP `service_access_plan`, and
MCP `agent-browser://access-plan{?serviceName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,accountId,accountIds,url,sitePolicyId,challengeId,readinessProfileId,browserBuild}`.
It is a read-only, no-launch planning surface. The response includes the same
profile selector metadata and readiness summary as profile lookup, then adds the
selected site policy, enabled providers, retained challenges, optional
`seedingHandoff`, advisory `browserCapabilityEvidence`, and a `decision` object with `recommendedAction`,
manual-action flags, selected profile ID, provider IDs, challenge IDs, stable
reason strings, and `freshnessUpdate` instructions that identify the serialized
profile freshness write path for bounded auth probes. The same decision
includes `monitorRunDue`, a copyable due-monitor recipe for HTTP
`POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`,
CLI `agent-browser service monitors run-due`, and
`runServiceAccessPlanMonitorRunDue()`. It includes
`browserCapabilityPreflight`, a copyable no-launch browser routing gate recipe
for HTTP `GET /api/service/browser-capability/preflight`, MCP
`service_browser_capability_preflight`, CLI
`agent-browser service browser-capability preflight`, and
`runServiceAccessPlanBrowserCapabilityPreflight()`. It also includes
`serviceRequest`, a copyable queued tab-request recipe for HTTP `POST
/api/service/request`, MCP `service_request`, and the `requestServiceTab()`
client helper. When CDP-free posture blocks that tab recipe, `serviceRequest`
also includes `cdpFreeAvailability`, a no-launch command-availability summary
that names `cdp_free_launch` as the lifecycle-only alternative and lists
service-request actions that still require CDP. `acquireServiceLoginProfile()` is the higher-level
software-client helper that can run both the due-monitor recipe and the
browser-capability preflight recipe before returning the final access plan.
`summarizeServiceProfileAcquisition()` turns that result into compact
selected-profile, registration, due-monitor, browser-preflight, and
access-plan attention fields for software logs and operator output. It reports
whether the request can be sent immediately or should be reused after manual
seeding, challenge approval, or provider work completes.

`service-browser-capability-registry-upsert-response.v1.schema.json` describes
the response envelope returned by HTTP `POST
/api/service/browser-capability-registry/<collection>/<id>` and MCP
`service_browser_capability_registry_upsert`. The path collection and ID are
authoritative; the response returns the upserted record, updated advisory
registry, collection counts, and `routingApplied: false`.

The service config mutation schemas describe write response envelopes returned
by HTTP service APIs and matching MCP tools:

- `service-profile-upsert-response.v1.schema.json`
- `service-profile-delete-response.v1.schema.json`
- `service-session-upsert-response.v1.schema.json`
- `service-session-delete-response.v1.schema.json`
- `service-site-policy-upsert-response.v1.schema.json`
- `service-site-policy-delete-response.v1.schema.json`
- `service-monitor-upsert-response.v1.schema.json`
- `service-monitor-delete-response.v1.schema.json`
- `service-monitor-run-due-response.v1.schema.json`
- `service-monitor-state-response.v1.schema.json`
- `service-provider-upsert-response.v1.schema.json`
- `service-provider-delete-response.v1.schema.json`
- `service-browser-capability-registry-upsert-response.v1.schema.json`

These schemas cover the authoritative path ID, mutation flag, and returned or
removed record payload for persisted profile, session, site policy, monitor,
and provider writes. Monitor mutation paths are HTTP
`POST /api/service/monitors/<id>` and `DELETE /api/service/monitors/<id>` plus
MCP `service_monitor_upsert` and `service_monitor_delete`; they persist monitor
definitions for the scheduler.
`service-monitor-run-due-response.v1.schema.json` covers the immediate due
monitor run summary returned by CLI, HTTP, MCP, and the service client helper,
including per-monitor results and stale profile IDs from profile-readiness
expiry.
`service-monitor-state-response.v1.schema.json` covers pause/resume state
updates that preserve retained health history.

Profile mutation inputs are policy checked before persistence. The
`caller_supplied` allocation requires `userDataDir`, and `per_service` profiles
may list at most one `sharedServiceIds` entry. Session mutation inputs infer
`owner` from `agentName`, then `serviceName`, when omitted, require `profileId`
to reference a persisted profile, and enforce profile `sharedServiceIds`
allow-lists.

Profile records separate caller ownership from target login scope.
`sharedServiceIds` names caller services allowed to use the profile,
`targetServiceIds` names target sites or identity providers whose credentials
or login state should live in the profile, and `authenticatedServiceIds` names
targets currently believed to have usable authenticated state. `targetReadiness`
is the no-launch readiness view derived from those hints and site policy. Its
seeding metadata lets clients distinguish Google-style detached headed seeding
from ordinary attachable login flows without parsing prose recommendations.
Explicit freshness rows are preserved when they report `fresh`, `stale`,
`blocked_by_attached_devtools`, `lastVerifiedAt`, or `freshnessExpiresAt`.
Session records include `profileSelectionReason` so clients can distinguish
`authenticated_target`, `target_match`, `service_allow_list`, and
`explicit_profile` profile choices without reconstructing selector behavior
from events.
They also include `profileLeaseDisposition` and
`profileLeaseConflictSessionIds` so clients can see whether the selected
profile started a new browser, reused a retained session browser, or hit
another exclusive session. Service-scoped launches reject or wait on active
exclusive conflicts according to `profileLeasePolicy`.

The operator remedy mutation schemas describe write response envelopes returned
by HTTP service APIs and matching MCP tools:

- `service-job-cancel-response.v1.schema.json`
- `service-browser-retry-response.v1.schema.json`
- `service-incident-acknowledge-response.v1.schema.json`
- `service-incident-resolve-response.v1.schema.json`

These schemas cover queued job cancellation, manual browser recovery retry
overrides, and durable incident acknowledgement or resolution metadata.

`service-reconcile-response.v1.schema.json` describes the response envelope
returned by `agent-browser service reconcile` and HTTP
`POST /api/service/reconcile`. It covers the reconciliation flag, browser
counts, changed browser count, and returned service state snapshot.

## Service Trace Aggregate Records v1

`service-trace-response.v1.schema.json` describes the full `service_trace`
response returned by `agent-browser service trace`, HTTP `GET
/api/service/trace`, and MCP `service_trace`.

`service-trace-summary-record.v1.schema.json` describes the `summary` object
returned by `agent-browser service trace`, HTTP `GET /api/service/trace`, and
MCP `service_trace`. Its `profileLeaseWaits` object provides a per-job rollup
of profile lease waits, including outcome, timing, conflict sessions, and trace
labels, so clients do not need to reconstruct waits from raw event pairs. Its
`browserCapabilityLaunches` object provides compact launch binding decision rows
from retained launch events or matching service sessions, including applied or
skipped state, reason, browser build, binding ID, host ID, executable ID, and
trace context labels.
Summary context rows also include `targetIdentityCount` and `targetServiceIds`
so dashboards and software clients can show the target profile-selection
identity for each service, agent, task, browser, profile, or session context
without scanning every retained job.

`service-trace-activity-record.v1.schema.json` describes normalized `activity`
items returned by:

- CLI `agent-browser service activity <incident-id>`
- CLI `agent-browser service trace`
- HTTP `GET /api/service/incidents/<id>/activity`
- HTTP `GET /api/service/trace`
- MCP `agent-browser://incidents/{incident_id}/activity`
- MCP `service_trace`

The schemas are guarded by Rust model/action tests and live HTTP/MCP trace
smokes so dashboards, API clients, and agents can use the aggregate trace
payload without rejoining raw event, job, and incident records themselves.

`service-incident-activity-response.v1.schema.json` describes the standalone
incident activity response returned by `agent-browser service activity
<incident-id>`, HTTP `GET /api/service/incidents/<id>/activity`, and MCP
`agent-browser://incidents/{incident_id}/activity`.
