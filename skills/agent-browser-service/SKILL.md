---
name: agent-browser-service
description: Operate shared, authenticated, service-owned Agent Browser work when profile selection, retained-browser reuse, Guacamole or RDP presentation, route capacity, doctor findings, or runtime cleanup could affect the request. Use for persistent browser work and troubleshooting; use the general agent-browser skill for isolated throwaway page automation.
---

# Agent Browser Service

Let Agent Browser own browser lifecycle, profile leases, retained-browser reuse,
route selection, handoffs, and cleanup. Your job is to declare the browser
intent, follow the returned plan, and report the exact blocked axis.

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
   Guacamole or RDP view and Agent Browser returns a route blocker.
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
