# CDP-Free Access-Plan Posture

## Context

The service roadmap calls out sites that are sensitive to Chrome DevTools Protocol attachment. Canva is the current concrete example: some sessions can fail before the page loads when a remote debugging port is present.

## Decision

Site policies now include `requiresCdpFree`. Access-plan responses expose that policy through `decision.launchPosture.requiresCdpFree` and `decision.launchPosture.cdpAttachmentAllowed` so agents, MCP clients, and software clients can decide before launching Chrome or requesting a tab.

The built-in Canva site policy defaults to:

- `browserHost: "local_headed"`
- `requiresCdpFree: true`
- `interactionMode: "human_like_input"`
- `manualLoginPreferred: true`
- `profileRequired: true`
- `challengePolicy: "manual_only"`

Local configured or persisted policies with the same ID still override built-in defaults.

## Current Boundary

The access-plan and request surfaces now separate CDP-backed tab control from CDP-free process ownership. When `requiresCdpFree` is true, clients should avoid DevTools attachment and treat CDP-backed commands as unavailable unless a later service capability explicitly says otherwise.

The follow-up enforcement slice copies `requiresCdpFree` and `cdpAttachmentAllowed` into `decision.serviceRequest.request`. HTTP `POST /api/service/request`, MCP `service_request`, and the generated service-request client reject requests with `requiresCdpFree: true` and `cdpAttachmentAllowed: false`, so a normal queued tab request cannot accidentally open a DevTools-attached browser for a CDP-sensitive site.

The explicit `cdp_free_launch` service request action is the narrow exception. It launches headed Chrome without a DevTools port, records service-owned browser PID, profile, session, lifecycle, and lease metadata, and returns typed launch metadata. It does not provide snapshot, screenshot, DOM, or input control; those need future non-CDP observation and control primitives.

## Validation

Run the focused no-launch checks for access-plan and site-policy source parity, plus Rust contract tests, after changing this surface:

```bash
pnpm test:service-access-plan-no-launch
pnpm test:service-site-policy-sources-no-launch
cargo test --manifest-path cli/Cargo.toml service_access -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
```
