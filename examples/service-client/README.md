# Service Client Example

This example shows the software-client workflow for agent-browser service mode:

- register or refresh a login profile with `registerServiceLoginProfile`
- request one intent-based service tab with `requestServiceTab`
- read the matching service trace with `getServiceTrace`
- optionally cancel a queued job with `cancelServiceJob`
- keep `serviceName`, `agentName`, and `taskName` attached to both calls

## Dry Run

```bash
pnpm --filter agent-browser-service-client-example dry-run
```

The dry run validates imports and prints the request and trace query without
contacting a running agent-browser service.

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
  --register-profile-id journal-example
```

You can also set `AGENT_BROWSER_SERVICE_BASE_URL` instead of passing
`--base-url`.

The script prints the command result, typed tab, title, wait, viewport, and
console result fields, trace counts, and the latest retained jobs so software
projects can confirm that the request and trace metadata are connected.

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

## Managed Profile Broker Recipe

Use `managed-profile-flow.mjs` when a software client needs the CanvaCLI-style
profile-broker pattern:

1. Inspect managed profiles with `getServiceProfiles()`.
2. Inspect a candidate profile with `getServiceProfileReadiness({ id })`.
3. Request the target identity with `requestServiceTab()`.
4. Register a managed profile only when agent-browser has no suitable profile.
5. Ask the operator to seed the profile when readiness reports `needs_manual_seeding`.

Dry-run the recipe without contacting a service:

```bash
pnpm --filter agent-browser-service-client-example managed-profile-dry-run
```

Validate the existing-profile path without launching Chrome or a daemon:

```bash
pnpm test:service-client-managed-profile-flow
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
  --url https://www.canva.com/
```

Only add `--register-profile-id canva-default` when the service has no suitable
managed profile or the operator intentionally wants a new account lane. The
script registers with `authenticated: false` by default so readiness can drive
manual seeding instead of pretending a new profile is already signed in.

Pass `--cancel-job-id <job-id>` when your software already knows a queued job
that should be cancelled. The script calls `cancelServiceJob` and prints the
cancellation result alongside the tab request and trace output. Use
`pnpm test:service-job-naming-live` for the repo-local live smoke that creates
and cancels a queued job end to end.
