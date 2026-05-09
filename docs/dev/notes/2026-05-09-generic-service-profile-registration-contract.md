# Generic service profile registration contract

Date: 2026-05-09

## Current decision

The default software-client integration path is service-owned and identity-first:

1. The client names `serviceName`, `agentName`, `taskName`, and a target identity such as `loginId`, `siteId`, or `targetServiceId`.
2. The client asks agent-browser for `GET /api/service/access-plan` or `getServiceAccessPlan()`.
3. If no suitable managed profile exists, the client registers one through `registerServiceLoginProfile()` or HTTP `POST /api/service/profiles/<id>`.
4. Recurring managed profiles should also get a retained `profile_readiness` monitor through `upsertServiceProfileReadinessMonitor()`, MCP `service_monitor_upsert`, or HTTP `POST /api/service/monitors/<id>`.
5. The client submits the planned tab request from `decision.serviceRequest` through `requestServiceTab()`, HTTP `POST /api/service/request`, or MCP `service_request`.

Direct `profile`, `runtimeProfile`, or custom profile paths remain override workflows. The normal path should let agent-browser coordinate profile selection, readiness, leases, browser reuse, and queued control requests.

New profiles that may need Google sign-on, Chrome sync setup, passkeys, or browser plugin setup need a detached headed seeding phase before CDP is attached. Do not launch the first Google sign-in with `--attachable`, a remote debugging port, or any other DevTools/CDP attachment. The intended flow is:

1. Register the managed profile with `authenticated: false` and `keyring: "basic_password_store"` unless the operator explicitly chooses a different keyring policy.
2. Launch manual seeding with `agent-browser --runtime-profile <name> runtime login https://accounts.google.com` without `--attachable`.
3. Let the human complete Google sign-in, sync, passkey, and extension/plugin setup, then close Chrome.
4. Relaunch or request future tabs through the service-owned path so agent-browser can attach only after the seeded profile is ready.

`basic_password_store` is the preferred managed-profile keyring posture because it avoids blocking GNOME Keyring, KWallet, or OS keychain modals during unattended browser workflows. Other keyring policies should be explicit service or operator decisions.

The structured readiness contract now exposes this directly on `targetReadiness` rows. Google-style manual seeding rows should report `seedingMode: "detached_headed_no_cdp"`, `cdpAttachmentAllowedDuringSeeding: false`, `preferredKeyring: "basic_password_store"`, and `setupScopes` covering `signin`, `chrome_sync`, `passkeys`, and `browser_plugins`.

Use the seeding handoff surface when an operator needs exact instructions: `agent-browser service profiles <profile-id> seeding-handoff google`, HTTP `GET /api/service/profiles/<profile-id>/seeding-handoff?targetServiceId=google`, or `getServiceProfileSeedingHandoff({ id, targetServiceId: "google" })`.

## What changed

- `55ebfb8` added `createServiceProfileReadinessMonitor()` and `upsertServiceProfileReadinessMonitor()` to the service observability client and taught the managed-profile example how to create the retained freshness monitor.
- `751cea8` updated the generic `examples/service-client/service-request-trace.mjs` workflow so non-Canva software clients can pass `--register-profile-id` plus `--register-readiness-monitor`.
- `7678cde` added no-launch HTTP/MCP coverage for the generic contract in `scripts/smoke-service-config.js`: HTTP profile upsert, MCP `profile_readiness` monitor upsert, HTTP access-plan read, and MCP access-plan resource read all agree on the planned `service_request` recipe.

## Validation

Current validation coverage:

- `pnpm test:service-client-example` validates the generic dry-run workflow.
- `pnpm test:service-client-example-live` validates the generic JavaScript client path against an isolated live daemon and browser session.
- `pnpm test:service-config-live` validates the no-launch HTTP/MCP profile registration, monitor registration, and access-plan contract.
- `pnpm test:service-client` validates generated client exports, types, service request helpers, service observability helpers, and managed-profile flow mocks.

## Guidance for future agents

When a downstream software project proposes creating its own runtime profile directly, first steer it to the generic service-owned path above. A bring-your-own profile is still allowed, but it should be an explicit override, not the default integration pattern.

When the target is Google, Gmail, Microsoft SSO through Google, browser sync, passkeys, or plugin setup, also steer the operator to detached seeding without CDP before the profile is marked ready for automation. Treat a CDP-attached first sign-in as a likely cause of Google sign-on failure, not as a normal retry path.
