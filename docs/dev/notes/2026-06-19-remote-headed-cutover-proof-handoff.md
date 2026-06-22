# Remote Headed Cutover Proof Handoff

Date: 2026-06-19
Source Plan: `docs/dev/plans/0038-2026-06-19-remote-headed-cutover-proof-plan.md`

## Summary

Agent-browser now has a generic proof gate for clients that want to prefer
hidden remote-headed browsers managed through Guacamole/RDP and
`chromium-stealthcdp`. Downstream repos should treat this as an adoption input,
not as an automatic migration. Each downstream agent still owns its own rollout,
schema changes, and live mutation policy.

The preferred generic posture is:

```text
browserBuild=stealthcdp_chromium
browserHost=remote_headed
viewStreamProvider=rdp_gateway
controlInputProvider=manual_attached_desktop
displayIsolation=private_virtual_display
```

## Required Gate

Before a downstream service defaults to this lane, run:

```bash
pnpm test:remote-headed-cutover-proof-live
```

This gate runs:

```bash
pnpm test:rdp-guac-route-pool-readiness
pnpm test:rdp-guac-many-to-many-live
pnpm test:service-request-live
```

Passing evidence from Plan 0038:

- `agent-browser install doctor --json` passed with no issues.
- `agent-browser doctor remote-view --json` passed with `status=ready`.
- The route-pool proof verified both local embed and public operator routes.
- The public operator route was
  `https://agent-browser.ecochran.dyndns.org/guacamole/`.
- The many-to-many Guacamole/RDP proof passed with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-20T02-29-41-100Z`.
- The HTTP/MCP service-request live proof passed.

## No-Launch Access-Plan Proof

Clients can check posture preservation without mutating a downstream site:

```bash
agent-browser --json service access-plan \
  --service-name AuraCall \
  --agent-name codex \
  --task-name remote-headed-cutover-proof \
  --target-service-id chatgpt \
  --login-id chatgpt \
  --account-id consult \
  --browser-build stealthcdp_chromium \
  --browser-host remote_headed \
  --view-stream-provider rdp_gateway \
  --control-input-provider manual_attached_desktop \
  --display-isolation private_virtual_display
```

The response should preserve the requested posture in the query,
`decision.launchPosture`, `decision.profileReuse`, and
`decision.serviceRequest.request.params`. If a policy intentionally overrides a
field, the response should make that override visible rather than silently
falling back to local CDP posture.

## Profile Sharing Boundary

Several clients may share one authenticated profile at runtime only through
one retained browser process group. The expected sharing model is separate tabs
or windows owned by that retained browser, with explicit route hints and tab
release. Independent Chrome process groups must not reuse the same profile
directory by default.

Downstream clients should request retained tab acquisition from access-plan
route hints. They should not ignore those hints and launch a second browser
against the same authenticated profile directory.

Expected retained-sharing fields include:

```text
profileReuse.recommendedAction=reuse_existing_browser
sharedAcquisition.mode=tab_new
browserId=<retained browser id>
sessionName=<retained session name>
```

## Route Descriptors

Clients should consume structured route descriptors instead of treating a
single URL as authoritative for every audience. The descriptors distinguish:

- local embed URL for local dashboard or harness embedding;
- dashboard embed URL when separate from the local route;
- public operator URL for remote operator access;
- health URL for readiness checks;
- backward-compatible external URL.

Local URLs can still appear in route descriptors because local iframes and
health checks need them. The public operator route remains the remote access
surface for human operators.

## Identity And Account Detection

Identity/account detection should remain generic. Agent-browser can provide
general-purpose routines for detection, read-only probes, snapshots, and
extraction scaffolding. Downstream services may provide recipes or their own
instructions for a particular website or service.

Do not hardcode AuraCall selectors, private account IDs, or migration state in
agent-browser. Recipes should be service-owned inputs to a generic detection
surface.

## AuraCall Adoption Shape

AuraCall can use this handoff as the successor input to its Plan 0141 work:

- Prefer Guacamole/RDP plus `chromium-stealthcdp` only after the agent-browser
  proof gate passes in its environment.
- Start with a no-mutation access-plan check that preserves the requested
  remote-headed posture.
- Require either retained-browser reuse through tab acquisition or a safe
  service-owned acquisition path before live-follow mutation.
- Keep AuraCall migrations, profile mapping changes, and live mutation gates in
  the AuraCall repo.

This note does not modify the AuraCall repo and does not authorize downstream
live mutation by itself.
