# External BYOP Browser Adoption Plan

Date: 2026-06-15
State: CLOSED
Lane: P14/P15 follow-up
Depends On:
- `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`
- `docs/dev/plans/0034-2026-06-14-generic-browser-service-routines-plan.md`
- `/home/ecochran76/workspace.local/auracall/docs/dev/plans/0141-2026-06-12-agent-browser-migration.md`

## Purpose

AuraCall proved the no-launch side of its agent-browser migration pilot:
explicit BYOP profile registration, authenticated access-plan selection, and
stock Chrome preflight. The pilot stopped before live mutation because
agent-browser can select the BYOP profile but still sees the already-running
AuraCall Chrome lane as observed-only state. Access-plan therefore recommends
`launch_new_browser` instead of `reuse_existing_browser`.

This plan adds a generic agent-browser adoption path for externally supplied
BYOP browser lanes. The goal is not AuraCall-specific migration code. The goal
is a reusable service-owned contract that can attach to a caller-supplied
existing Chrome/CDP endpoint, record it as retained browser/session/tab state
for the selected `external_byop` profile, and let the next access plan reuse
that lane without duplicate browser/profile pressure.

## Current Gap

- `registerExternalProfile()` records `profileOrigin: "external_byop"` with
  target/account/profile metadata.
- Access-plan can select that BYOP profile.
- `profileReuse` only considers retained `service_state.browsers`.
- A running external Chrome is not retained browser/session/tab state until a
  launch/attach path persists it.
- Existing attach paths can connect to CDP (`cdpUrl`, `cdpPort`,
  `autoConnect`) and persist `host: "attached_existing"` when routed through
  launch, but there is no explicit service request that adopts a BYOP browser
  lane for reuse before the client asks for a normal tab.

## Operating Invariant

```text
agent-browser may adopt an explicitly supplied external BYOP browser into
service state, but it must not silently claim arbitrary observed Chrome
processes as owned profile data.
```

`external_byop` is an opt-in service lease/profile contract. `external_observed`
remains read-only process or resource evidence and must not become a reusable
identity lane.

## Non-Goals

- Do not mutate AuraCall.
- Do not scan all local Chrome processes and auto-adopt a browser.
- Do not treat `external_observed` profiles as reusable identities.
- Do not bypass CDP-free site policy.
- Do not close or kill the adopted external browser by default.
- Do not require provider-specific selectors or AuraCall-specific profile
  paths in agent-browser tests.

## Target Contract

Add a generic service-owned adoption routine, likely as a new service request
action named `external_byop_adopt` or `browser_adopt`.

Required request data:

- caller labels: `serviceName`, `agentName`, `taskName` when known;
- selected profile identity: `runtimeProfile` or access-plan-selected profile
  id;
- one explicit attach target: `cdpUrl`, `cdpPort`, or a reviewed equivalent;
- profile origin gate: selected profile must be `external_byop`;
- safety posture: detach/leave-open by default.

Useful optional request data:

- `url` to open or verify;
- `targetServiceId`, `loginId`, `accountId`;
- `browserBuild`, `browserFamily`, or compatibility evidence;
- `reuseSessionName` or generated session name;
- `recordTabHandle: true` to return the active tab handle.

Response should include:

- adopted browser id;
- session name;
- selected profile id and `profileOrigin: "external_byop"`;
- CDP endpoint or endpoint presence metadata;
- active tab/service tab handle when available;
- detach/cleanup policy;
- trace filters;
- access-plan follow-up hint showing that reuse should now be possible.

## Implementation Slices

### Slice A: No-Launch Contract And Access-Plan Readback

Goal: make the contract visible and prove access-plan behavior with seeded
retained state.

Deliverables:

- Add the new service request action to:
  - `SERVICE_REQUEST_ACTIONS`;
  - `docs/dev/contracts/service-request.v1.schema.json`;
  - HTTP `/api/service/request`;
  - MCP `service_request`;
  - generated `@agent-browser/client` helpers and declarations.
- Add client helper aliases that make the safe path obvious.
- Add service-access tests proving:
  - selected `external_byop` profile without retained browser still recommends
    `launch_new_browser` or an adoption-specific action;
  - selected `external_byop` profile with retained `attached_existing` browser
    recommends `reuse_existing_browser`;
  - selected `external_observed` profile does not become reusable solely from
    observed profile metadata.

### Slice B: Adopt Existing BYOP CDP Lane

Goal: connect to an explicitly supplied existing browser endpoint and persist
it as retained state for the selected BYOP profile.

Deliverables:

- Implement the service request action by reusing the existing attach/launch
  machinery where possible.
- Persist `BrowserProcess.host = "attached_existing"`, ready health, CDP
  endpoint metadata, profile id, caller labels, session lease, and a service tab
  handle.
- Default cleanup to detach/leave-open so agent-browser does not close the
  external browser unexpectedly.
- Reject missing attach target, unknown profile, non-`external_byop` profile,
  CDP-free policy conflicts, and incompatible profile lease conflicts.

### Slice C: Live Generic Adoption Smoke

Goal: prove the end-to-end issue AuraCall hit without using AuraCall state.

Deliverables:

- Start an isolated Chrome or managed external profile with a known CDP port.
- Register a generic `external_byop` profile with target/account freshness.
- Confirm access-plan selects the profile but has no reusable retained browser
  before adoption.
- Run the adoption service request.
- Confirm service status/trace has retained browser/session/tab state with
  `host: "attached_existing"` and `profileOrigin: "external_byop"`.
- Refresh access-plan and assert `decision.profileReuse.recommendedAction` is
  `reuse_existing_browser` with `browserId` and `sessionName` route hints.
- Request a tab from the refreshed access plan and prove it routes to the
  adopted lane instead of launching another browser.

## Validation Matrix

Baseline:

```bash
git diff --check
pnpm validation:select -- --base HEAD
```

Contract and client:

```bash
pnpm generate:service-client
pnpm test:service-client
pnpm test:service-api-mcp-parity
cargo test --manifest-path cli/Cargo.toml service_request_command -- --nocapture
cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1
```

Service behavior:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_access -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_config -- --test-threads=1
```

Live proof:

```bash
pnpm test:service-external-byop-adopt-live
```

Docs and skill:

```bash
pnpm --dir docs build
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
```

## Done Definition

- A generic BYOP adoption action exists through HTTP, MCP, generated client
  helpers, docs, and skill guidance.
- The action only adopts explicit `external_byop` profiles with caller-supplied
  CDP connection details.
- `external_observed` remains non-reusable identity evidence.
- A live smoke proves the post-adoption access plan recommends
  `reuse_existing_browser` and the copied tab request routes to the adopted
  lane.
- AuraCall remains unchanged except for its already-written handoff/plan state.

## Closeout

Implemented 2026-06-15.

Delivered:

- Added `external_byop_adopt` to the service request contract, HTTP relay, MCP
  schema metadata, generated service client, and JS helper surface.
- Added `requestServiceExternalByopAdopt()` and `adoptExternalByopBrowser()` for
  software clients that already have a registered `external_byop` profile and
  exactly one explicit Chrome DevTools endpoint.
- Implemented the daemon action to attach to `cdpUrl` or `cdpPort`, persist the
  browser as `host: "attached_existing"`, retain browser/session/tab state, and
  return a `serviceTabHandle` with `profileOrigin: "external_byop"`.
- Kept `external_observed` non-reusable for identity/profile reuse.
- Relaxed access-plan host matching only for selected `external_byop` profiles
  when the caller did not explicitly request a browser host, allowing adopted
  attached Chrome lanes to satisfy default headed access plans without duplicate
  browser/profile pressure.
- Added `pnpm test:service-external-byop-adopt-live`, which starts an isolated
  source Chrome, adopts it into an isolated service daemon, and proves the next
  access plan recommends `reuse_existing_browser` with route hints.
- Updated README, docs site, CLI help, and the agent-browser skill.

Validation:

```bash
pnpm test:service-api-mcp-parity
pnpm test:service-client
pnpm test:service-external-byop-adopt-live
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_access_plan_reuses_external_byop_attached_browser_without_host_request -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_access_plan_does_not_reuse_external_observed_browser -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_request_command_accepts_contract_actions -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_request_schema_and_command_accept_contract_actions -- --test-threads=1
pnpm --dir docs build
git diff --check
```
