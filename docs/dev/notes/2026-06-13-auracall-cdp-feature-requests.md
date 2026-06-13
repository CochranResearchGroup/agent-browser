# AuraCall CDP Migration Feature Requests

Date: 2026-06-13

## Purpose

This handoff is for the repo-specific agent-browser implementation agent.
AuraCall is evaluating a migration from its internal browser lifecycle service
to agent-browser as the shared browser control plane. The first read-only
AuraCall audit found that agent-browser already handles the right lifecycle
domain: access plans, profile selection, service requests, profile leases,
remote-headed posture, resource attribution, retained browser/session/tab
state, and cleanup policy.

The remaining migration gap is not ordinary browser actions. AuraCall services
still depend on a raw CDP-style contract for provider scraping and diagnostics.
Some of those CDP patterns are broadly useful and should become higher-level
agent-browser service features rather than AuraCall-only adapter code.

## Source Context

Relevant AuraCall source surfaces. These paths are relative to the sibling
`../auracall` repository from this repo root:

- `../auracall/docs/dev/plans/0141-2026-06-12-agent-browser-migration.md`
- `../auracall/packages/browser-service/src/service/types.ts`
- `../auracall/src/browser/service/browserService.ts`
- `../auracall/src/browser/llmService/llmService.ts`
- `../auracall/src/browser/providers/chatgptAdapter.ts`
- `../auracall/src/browser/providers/grokAdapter.ts`
- `../auracall/src/accountMirror/chatgptMetadataCollector.ts`

Relevant agent-browser source surfaces:

- `docs/dev/notes/2026-05-09-access-plan-service-request-handoff.md`
- `docs/dev/contracts/service-request.v1.schema.json`
- `docs/dev/contracts/service-access-plan-response.v1.schema.json`
- `docs/dev/contracts/service-tab-record.v1.schema.json`
- `docs/dev/contracts/service-browser-record.v1.schema.json`
- `docs/dev/contracts/service-trace-response.v1.schema.json`
- `packages/client/src/service-request.js`
- `packages/client/src/service-observability.js`

The 2026-05-09 access-plan checkpoint is the right integration foundation.
The new work should extend that service-owned handoff, not create a separate
caller-owned browser path.

## Current AuraCall Contract Shape

AuraCall currently expects a browser-service handle that can provide:

- `resolveDevToolsTarget()` with `host`, `port`, selected service tab, and
  launch or reuse behavior.
- `connectDevTools()` returning a Chrome/CDP client.
- selected `tabTargetId` and `tabUrl` for provider adapters.
- direct CDP domains such as `Runtime`, `Page`, `Network`, `DOM`, `Input`,
  `Browser`, and `Target`.
- arbitrary bounded and unbounded `Runtime.evaluate(...)` calls for provider
  DOM reads, identity checks, prompt/upload flows, and diagnostics.
- screenshot, URL, title, console, network, request-body, and DOM evidence
  when provider work fails.
- mutation audit and browser-operation queue evidence.

Agent-browser already exposes service request actions for navigation, tabs,
snapshots, screenshots, input, selectors, cookies, storage, upload/download,
network request tracking, HAR, route/unroute, and cleanup. That covers many
ordinary actions. It does not yet expose an AuraCall-compatible lease-backed
CDP attachment or bounded evaluate contract that lets provider scrapers keep
using targeted CDP safely under agent-browser ownership.

## Design Direction

The implementation should preserve this split:

- agent-browser owns browser lifecycle, profile leasing, tab routing, resource
  attribution, remote view, cleanup, and generic browser primitives.
- AuraCall owns ChatGPT, Gemini, and Grok provider semantics, including DOM
  interpretation, account-mirror cursors, artifact materialization policy, and
  identity mismatch business rules.

Do not move AuraCall provider scraping logic into agent-browser. Instead, add
service-owned primitives that let any software client do controlled CDP-backed
work without bypassing profile leases, tab reuse, traceability, and cleanup.

## Feature Request 1: Profile Origin And BYOP Registration

Priority: P0

Add first-class profile-origin semantics to service profile records and
access-plan/profile lookup output.

Required origins:

- `agent_browser_owned`: normal fresh-install and newly seeded account lane.
- `external_byop`: caller-supplied user-data directory registered for service
  ownership and lease policy.
- `external_observed`: process/resource evidence only, not trusted for
  authenticated work or cleanup.

AuraCall should be able to register an existing managed browser profile as
BYOP for continuity, for example:

- service name: `AuraCall`
- agent name: `auracall-api`
- target service id: `chatgpt`
- account id: `consult@polymerconsultinggroup.com`
- profile origin: `external_byop`
- user-data dir:
  `~/.auracall/browser-profiles/wsl-chrome-2/chatgpt`
- browser build/family compatibility: explicit and validated

Fresh installs should not need AuraCall profile directories. They should use
`agent_browser_owned` profiles created, seeded, leased, and cleaned by
agent-browser.

Acceptance criteria:

- `getServiceAccessPlan()` can distinguish `agent_browser_owned`,
  `external_byop`, and `external_observed`.
- Access plans never select an unrelated default profile when account or target
  identity hints require an account-bound lane.
- BYOP registration is explicit, auditable, and records browser-family/build
  compatibility evidence.
- Cleanup policy treats `external_byop` more conservatively than
  `agent_browser_owned`, and never treats `external_observed` as owned.

## Feature Request 2: Lease-Backed Service Tab Handle

Priority: P0

Extend the access-plan to service-request handoff so a successful tab request
returns a stable, lease-backed handle for follow-on software-client work.

The handle should include:

- `browserId`
- `sessionName`
- `tabId` or equivalent service tab record id
- CDP target id when CDP attachment is allowed
- current URL and title when available
- selected service profile id
- profile origin
- lease id or lease metadata
- cleanup policy and lease heartbeat expectation
- trace or job id for evidence lookup

This should be available through HTTP, MCP, and `@agent-browser/client`.
Callers should not need to reconstruct browser identity from process lists,
DevTools ports, or ad hoc target scans after `requestServiceTab()`.

Acceptance criteria:

- `requestServiceTab()` returns enough metadata for a software client to bind
  subsequent commands to the same service-owned tab.
- The handle survives ordinary tab reuse and avoids duplicate browser/profile
  lanes.
- The handle is reflected in `service tabs`, `service browsers`, and
  `service trace`.
- The handle has a clear invalid/stale state when the tab or browser is gone.

## Feature Request 3: Controlled CDP Attach For Software Clients

Priority: P0

Add a supported service-owned way for software clients to attach to a leased
CDP target when site policy allows CDP attachment.

This does not need to expose raw process ownership. It can be a controlled
attachment descriptor or client helper, but it must let a caller run existing
provider code that requires CDP domains while preserving service ownership.

Required behavior:

- only allow CDP attach for a valid leased service tab handle;
- enforce site policy and access-plan `cdpAttachmentAllowed`;
- record the attach in service trace and job/session metadata;
- expose enough endpoint data for the official client helper to connect;
- support detach/close semantics without killing the browser by default;
- prevent accidental duplicate browser launches when the profile is locked.

Acceptance criteria:

- A software client can request a tab, attach through the supported service
  path, run bounded CDP reads, detach, and see trace evidence.
- The attach path fails closed when the profile is unverified, stale, wrong
  account, CDP-free, or policy-blocked.
- Browser lifecycle remains owned by agent-browser, not by the client.

## Feature Request 4: Bounded Evaluate Jobs

Priority: P1

Add a service request action for bounded JavaScript evaluation against a
service-owned tab.

Suggested action name: `evaluate`

Suggested request fields:

- leased tab handle or `browserId` plus service tab id;
- expression or named recipe id;
- `returnByValue`;
- `timeoutMs`;
- `maxReturnBytes`;
- `captureEvidenceOnFailure`;
- optional console and request capture windows;
- optional screenshot on failure;
- caller context: `serviceName`, `agentName`, `taskName`.

Suggested response fields:

- `ok`;
- result value or truncation metadata;
- exception details;
- captured URL/title;
- screenshot path when requested;
- console error summary;
- trace/job id.

This is useful beyond AuraCall because many browser agents need a safe escape
hatch between high-level selector commands and fully raw CDP.

Acceptance criteria:

- Evaluation is always timeout-bound and return-size-bound.
- Evaluation is tied to a service-owned browser/tab handle.
- Failures produce a compact diagnostic bundle rather than only an exception.
- The service-client, HTTP, MCP, and contract schemas stay aligned.

## Feature Request 5: Provider Readiness And Identity Probe Recipes

Priority: P1

Agent-browser should own the readiness/freshness record, while clients can
provide provider-specific probe logic or evidence.

Needed capability:

- run a bounded service-owned readiness probe after seeding or before
  authenticated work;
- record that the selected profile was fresh for a target service/account at a
  timestamp;
- support client-supplied probe evidence when provider semantics live outside
  agent-browser;
- expose stale, unverified, wrong-account, and manual-action-required states
  in access-plan output.

AuraCall can supply the ChatGPT/Gemini/Grok account identity logic. The generic
agent-browser feature is the probe lifecycle, evidence record, and gate in the
access-plan and service-request path.

Acceptance criteria:

- Access plans can block or warn on expired/unverified freshness.
- A successful post-seeding probe can mark one target service/account fresh.
- Wrong-account evidence remains a hard block for authenticated work.
- The readiness record links back to the service profile and probe evidence.

## Feature Request 6: Tab Reuse, Blank-Tab Control, And Stale-Target Repair

Priority: P1

Promote the useful parts of AuraCall's tab-resolution behavior into
agent-browser service routing.

Needed behavior:

- prefer compatible existing service tabs for the same profile/service/account;
- avoid accumulating blank tabs;
- explain why a retained tab was reused, ignored, or closed;
- repair stale tab handles by rescanning service-owned tab state;
- keep duplicate profile lane prevention visible in access-plan reuse advice.

Acceptance criteria:

- Service request trace explains tab reuse and discarded candidates.
- Blank tabs have explicit policy and cleanup behavior.
- A stale tab handle can be repaired or rejected without launching an
  unrelated browser lane.

## Feature Request 7: Diagnostic Evidence Bundle

Priority: P1

Add a single diagnostic bundle action or helper that collects the evidence
software clients repeatedly need after browser automation fails.

Suggested contents:

- profile id and origin;
- browser id, session name, tab id;
- URL and title;
- snapshot summary;
- screenshot path;
- console errors;
- recent request summary and selected request details;
- active route/view metadata;
- browser health and recovery state;
- caller context and trace/job ids.

Acceptance criteria:

- A client can request the bundle by tab handle or recent job id.
- The response is compact by default and has explicit caps for large fields.
- The bundle is linked from service trace or incident records.

## Feature Request 8: Service-Client Ergonomics

Priority: P2

The service-client should make the broker-first path hard to misuse.

Follow-on helpers to consider:

- `requestServiceTabFromAccessPlan(accessPlan, overrides)`;
- `attachServiceTabCdp(handle, options)`;
- `evaluateServiceTab(handle, options)`;
- `getServiceTabDiagnostics(handleOrJobId, options)`;
- `registerExternalProfile(...)` or a clearer BYOP registration helper.

Acceptance criteria:

- Clients do not manually destructure access-plan internals for normal use.
- Helper failures include actionable policy reasons.
- Generated types expose the new contracts without custom local casting.

## Non-Goals

- Do not implement ChatGPT, Gemini, Grok, or AuraCall-specific selectors in
  agent-browser.
- Do not bypass site policy when a service requires CDP-free operation.
- Do not auto-click captcha, anti-bot, or human-verification flows.
- Do not make external observed processes eligible for destructive cleanup.
- Do not require fresh installs to import AuraCall profile directories.
- Do not remove existing high-level selector actions.

## Suggested Implementation Order

1. Add profile-origin schema and BYOP registration/readback.
2. Extend `tab_new` response and service tab records with a lease-backed handle.
3. Add controlled CDP attach for leased service tabs.
4. Add bounded `evaluate` service request action.
5. Add diagnostic bundle helper.
6. Add readiness/freshness probe lifecycle improvements.
7. Harden tab reuse, blank-tab policy, and stale-handle repair.
8. Polish service-client helpers after the contracts stabilize.

This order lets AuraCall test the smallest useful bridge first: profile mapping
plus a lease-backed tab/CDP handle. The higher-level evaluate and diagnostics
features can then reduce how much raw CDP AuraCall needs to keep in its adapter.

## Validation Guidance

No implementation is expected in this handoff note. When implementation starts,
use the repo's normal contract discipline:

- update `docs/dev/contracts/*.schema.json`;
- regenerate `packages/client/src/*generated*`;
- keep HTTP, MCP, Rust service contracts, and generated client helpers aligned;
- add no-launch service-client tests for schema/helper behavior;
- add focused Rust tests for service request handling and trace records;
- add a live smoke only after no-launch contract tests pass.

Likely validation commands for the first implementation slice:

```bash
pnpm test:service-client
pnpm test:service-api-mcp-parity
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml service_request -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
git diff --check
```

If user-facing CLI, README, docs-site, or skill behavior changes, follow
`AGENTS.md` and update every required documentation surface in the same slice.

Validation performed for this handoff note:

- `git diff --check`
- Verified the listed agent-browser source surfaces exist in this repository.
- Verified the listed AuraCall source surfaces exist under the sibling
  `../auracall` repository.
- Ran Graphiti discovery against `agent_browser_main`; the relevant prior
  checkpoint is
  `docs/dev/notes/2026-05-09-access-plan-service-request-handoff.md`.

## Open Questions For The Implementation Agent

- Should controlled CDP attach return a temporary endpoint descriptor, a
  websocket URL, or only a client-helper-managed connection?
- Should bounded evaluate be a generic service request action, a CDP attach
  helper method, or both?
- How should profile-origin names align with existing runtime-profile and
  service-profile vocabulary?
- What is the minimum profile freshness evidence required before an access
  plan should return `use_selected_profile` for authenticated work?
- Should BYOP profiles ever be eligible for automatic browser close, or only
  tab close and lease release?
