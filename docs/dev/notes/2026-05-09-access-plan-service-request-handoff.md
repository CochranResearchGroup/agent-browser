# Access Plan Service Request Handoff Checkpoint

Date: 2026-05-09

## Purpose

This note records the service-roadmap slice that connected no-launch access
planning to the queued service request path for software clients.

The goal was to keep agent-browser as the service-owned profile and browser
coordinator. Clients should not decide which profile to use, bypass queueing,
or reconstruct browser-control handoff rules after asking for an access plan.

## What Changed

Commit `de9b3a1` added `decision.serviceRequest` to access-plan responses.
The field is returned by HTTP `GET /api/service/access-plan`, MCP
`agent-browser://access-plan`, and `getServiceAccessPlan()`.

The `serviceRequest` block describes the queued tab-request handoff:

- whether the planned request can be queued immediately
- whether the same request should be reused after manual seeding, challenge
  approval, or provider work completes
- the selected profile ID
- the recommended `profileLeasePolicy: "wait"` queue behavior
- the copyable `tab_new` service request payload
- the HTTP route, MCP tool, and service-client helper to use

Commit `eb075f9` extended `pnpm test:service-request-live` so it now calls
`getServiceAccessPlan()`, consumes `decision.serviceRequest.request`, passes it
into `requestServiceTab()`, and verifies the resulting live queued tab job
preserves the expected service, task, and target identity state.

## Validation

The implementation and follow-up live smoke were validated with:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-access-plan-no-launch`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm test:service-request-live`
- `pnpm --dir docs build`
- `git diff --check`
- `node scripts/dev/select-validation.js --base HEAD --json`
- installed skill sync with `/home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

The live smoke used an isolated daemon and real browser session. It proved that
the access-plan handoff can drive the browser-launching service request path
without caller-side profile-selection logic.

## Current Contract

Software clients should normally:

1. Call `getServiceAccessPlan()` with `serviceName`, `agentName`, `taskName`,
   and a target identity such as `loginId` or `targetServiceId`.
2. If `decision.serviceRequest.available` is true, pass the planned request
   fields into `requestServiceTab()` and add only action-specific fields such
   as `url`, `params`, or `jobTimeoutMs`.
3. If `recommendedAfterManualAction` is true, complete the required seeding,
   challenge approval, or provider work, then reuse the same service-owned
   request shape.

Direct `runtimeProfile`, `profile`, or caller-created runtime profiles remain
overrides. The normal path is for agent-browser to choose the managed profile,
coordinate lease waiting, and serialize browser control through the service
queue.

## Residual Gaps

- `requestServiceTab()` does not yet accept a whole access-plan response
  directly. Callers currently destructure `decision.serviceRequest.request`.
- Access-plan still describes the policy decision. Provider execution for
  2FA, passkeys, captcha, and human approval remains future work.
- Identity freshness probes are represented in readiness and freshness-update
  contracts, but recurring service-owned freshness probes are not complete.

## Recommended Next Slice

Make the service-client ergonomics match the contract by letting
`requestServiceTab()` or a small companion helper accept an access-plan response
directly. That would remove the remaining caller-side destructuring and keep
software clients on the service-owned path by default.
