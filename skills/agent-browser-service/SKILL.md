---
name: agent-browser-service
description: Operate shared, authenticated, service-owned Agent Browser work when profile selection, retained-browser reuse, native Remote View presentation, route capacity, doctor findings, or runtime cleanup could affect the request. Use for persistent browser work and troubleshooting; use the general agent-browser skill for isolated throwaway page automation.
---

# Agent Browser Service

Let Agent Browser own browser lifecycle, profile leases, retained-browser reuse,
route selection, handoffs, and cleanup. Your job is to declare the browser
intent, follow the returned plan, and report the exact blocked axis.

## Current native Remote View workflow

When the installed runtime has `AGENT_BROWSER_REMOTE_VIEW_ORIGIN` and
`AGENT_BROWSER_REMOTE_VIEW_POOL` configured, Remote View owns desktop allocation,
transport and authentication. Agent Browser owns the browser and its profile.
Use the configured integration; omit RDP stream, route-pool and display overrides.

For a human handoff, supply both session selectors explicitly:

```bash
agent-browser --json --session operator-review --session-name operator-review \
  remote-view open https://example.com/secure
```

An omitted profile selector uses a disposable profile. Add
`--runtime-profile <existing-profile-id>` when saved authentication or persistent
profile continuity is needed. Keep the same session and profile for follow-up
commands. Do not replace a requested persistent profile with a disposable one.
Require `operatorVisible.state=ready`, share only the returned `handoffUrl`, and
reopen that same durable URL for viewer reconnection. The operator uses existing
Remote View authentication. Continue ordinary automation in the same browser.
Return, store, and reopen only `handoffUrl`, shaped as `/remote-view/<handoff-id>`.
Never share `providerExternalUrl`, a raw Guacamole URL or a route-binding URL
as the operator handoff.


### Complete external handoff URL

A returned `handoffUrl` may be an absolute HTTPS URL or a relative
`/remote-view/<handoff-id>` path. Keep an absolute URL unchanged. Resolve a
relative path against the configured **public Agent Browser origin**, which
serves the handoff resolver. Do not prepend `AGENT_BROWSER_REMOTE_VIEW_ORIGIN`
(the provider control API), `AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN` (the native
viewer origin), a raw provider URL, or a loopback address.

For example, with public Agent Browser origin `https://browser.example.com`,
`/remote-view/<handoff-id>` becomes
`https://browser.example.com/remote-view/<handoff-id>`. Return a clickable complete
HTTPS link to the person. If the public Agent Browser origin is not known,
inspect its installed ingress configuration rather than guessing from the
Remote View provider hostname. Resolving the URL does not create a new handoff.

### Diagnose the requested operation

`browser_session_field_missing:sessionName` is a missing request field, not a
desktop outage. Inspect the failed request for effects, then correct the missing
`--session-name` before another open. `--session` alone does not supply it.
Software clients must pass `sessionName` at the top level of the request.

`production_presentation_inventory_scope_mismatch`, a zero admitted maximum in
the legacy `presentationCapacity` projection, missing legacy route X11 sockets,
or ownership warnings about unrelated browsers do not establish that a native
open is blocked. Use the exact request's typed failure and current native
provider evidence. Preserve genuine ownership failures for the selected profile;
do not bypass them or clean up another task's browser. Distinguish browser
acquisition, native presentation and runtime maintenance.

## Shared-profile tasks and retention

Choose one named session and tab per task. Deliberately select the same durable
profile when both tasks need its login, cookies and persistent site storage;
Agent Browser reuses one browser. Address each task through its own session.
Background tab automation does not require either tab to be physically visible.
Serialize foreground desktop input and human control. Coordinate logout, account
switching and conflicting application changes because profile data is shared.

Close only the completed task session; preserve its peers and the durable profile.
Use a disposable profile for work that does not need retained identity. Durable
logical handoffs default to no time expiry; disposable handoffs and inactivity
have finite configurable retention. Explicit close remains terminal. Handoff
retention does not promise a continuously running browser or viewer connection.

Native capacity prefers a ready vacant desktop and replenishes one spare in the
background where possible. At the configured limit, placement wraps around
eligible desktops. Automatic shrink belongs to Remote View and must preserve
active viewers, current work and one ready spare. Unrelated legacy projections
are not proof that the requested native launch is unavailable.

## Start with intent

For shared or authenticated work, provide these fields whenever known:

- `serviceName`
- `agentName`
- `taskName`
- one or more of `targetServiceId`, `siteId`, `loginId`, `accountId`, or `url`

Read `agent-browser://operating-guide`, then call `service_access_plan`. Use the
returned profile, reuse hints, readiness action, and browser posture in
`service_request`. Do not inspect route occupancy or install doctor first and
invent a different acquisition policy.

```json
{
  "serviceName": "BooksReceipts",
  "agentName": "receipt-agent",
  "taskName": "review-bill",
  "targetServiceId": "bill",
  "accountId": "soylei"
}
```

If MCP is unavailable, use the equivalent CLI read:

```bash
agent-browser --json service access-plan \
  --service-name BooksReceipts \
  --agent-name receipt-agent \
  --task-name review-bill \
  --target-service-id bill \
  --account-id soylei
```

## Keep three axes separate

1. **Browser acquisition** selects or reuses the profile, browser, session, and
   tab. Follow `decision.profileReuse`, `decision.recommendedAction`, and the
   service request result.
2. **Operator presentation** attaches a healthy browser to a view route. Route
   occupancy blocks presentation only when the requested workflow requires a
   native desktop view and the exact request returns a presentation blocker.
3. **Runtime maintenance** reports installation, multiplicity, cleanup, and
   retention health. A doctor warning is not a request blocker unless scoped
   readiness or the request result classifies it as one.

Never replace one axis with evidence from another. A checked-out route does not
prove that no browser can launch. A nonzero install doctor does not prove that
an unrelated profile or tab request must stop.

## Follow Agent Browser ownership

- Reuse a compatible retained browser and open a service-owned tab when the
  access plan provides `browserId` and `sessionName` route hints.
- Wait for an exclusive profile lease when the plan selects
  `profileLeasePolicy: wait`. Do not create another profile to avoid a holder.
- Request `remote_view_open` only when a person needs a visible desktop or the
  selected site policy requires that posture.
- Share only the durable `/remote-view/<handoff-id>` URL and require
  `operatorVisible.state=ready`.
- Use route preflight to inspect presentation readiness without launching.
- Use route switch only for an explicit presentation reassignment. Agent
  Browser decides whether an occupied route is parkable and protects active
  controller leases.
- Never close another browser, release another viewer, run GC, delete a
  profile, or kill a process merely to make the current request succeed.

## Report a blocked request

Name the exact axis, the typed code or field, and the safe next action. For
example:

```text
Browser acquisition: ready to launch profile bill-soylei.
Operator presentation: blocked by route_pool_exhausted.
Runtime maintenance: advisory only; zero readiness-impacting GC candidates.
Next action: wait for a route or request an explicit route switch. No workload was closed.
```

Do not report “Agent Browser is unhealthy” when only presentation is occupied
or a global advisory is present.

## Read the focused guide

- For profile selection, reuse, seeding, and release, read
  [references/profiles.md](references/profiles.md).
- For Guacamole, RDP, durable handoffs, route occupancy, and route switching,
  read [references/guacamole-rdp.md](references/guacamole-rdp.md).
- For doctor, resource pressure, cleanup, and failure classification, read
  [references/troubleshooting.md](references/troubleshooting.md).
