# AuraCall Service CDP Upgrade Plan

Date: 2026-06-13
State: OPEN
Lane: P14
Depends On:
- `docs/dev/notes/2026-05-09-access-plan-service-request-handoff.md`
- `docs/dev/notes/2026-06-13-auracall-cdp-feature-requests.md`
- `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`

## Purpose

Upgrade agent-browser from a browser lifecycle broker with high-level service
requests into a service-owned CDP-capable control plane that software clients
can use without bypassing profile leases, tab ownership, service trace, cleanup
policy, or duplicate-browser prevention.

The motivating downstream user is AuraCall, which wants to migrate away from
its internal browser lifecycle service while keeping provider-specific ChatGPT,
Gemini, and Grok scraping semantics outside agent-browser. The product value is
broader than AuraCall: any software client should be able to request a managed
profile/tab, attach or evaluate only through policy-gated service contracts, and
collect compact diagnostics when browser work fails.

## Current State

- The prior access-plan to service-request handoff exists at
  `docs/dev/notes/2026-05-09-access-plan-service-request-handoff.md`.
- P13 minimal-profile reuse has established the prevention invariant: reuse
  compatible live browser/profile lanes, wait for profile leases, and launch
  only when isolation or policy requires a new lane.
- The AuraCall handoff note records the migration gap and source context at
  `docs/dev/notes/2026-06-13-auracall-cdp-feature-requests.md`.
- Existing service request actions cover many high-level browser operations,
  but software clients do not yet have a lease-backed service tab handle,
  controlled CDP attach contract, bounded evaluate service action, or
  profile-origin/BYOP contract.
- No implementation has started in this lane. Slice A is the recommended first
  implementation target.

## Operating Invariant

```text
agent-browser owns lifecycle, leases, CDP access, traceability, and cleanup;
clients own domain semantics and only operate through service-owned handles.
```

This lane must not create a second caller-owned browser path. Each slice should
extend the existing access-plan to service-request handoff, keep HTTP, MCP,
Rust contracts, and `@agent-browser/client` aligned, and preserve the minimal
runtime-profile reuse invariant from P13.

## Goal Shape For Subagents

Use this plan as a parent goal. Assign one subagent per slice, with each
subagent responsible for a bounded contract surface and its validation evidence.
Subagents should report back with:

- changed files and public contract deltas;
- pass/fail validation commands;
- unresolved policy or migration questions;
- whether the slice is no-launch, live-smoke, or both;
- whether the slice is safe to stack onto the next slice.

Subagents should not implement provider-specific AuraCall selectors, copy
private browser profiles into this repo, or touch live authenticated browser
state unless a slice explicitly requires a live smoke.

## Slice A: Profile Origin And BYOP Registration

Goal: make profile ownership explicit before agent-browser accepts external
profiles as service-owned lanes.

Deliverables:

- Add profile-origin vocabulary to service profile records:
  `agent_browser_owned`, `external_byop`, and `external_observed`.
- Add an explicit BYOP registration/readback path for caller-supplied
  user-data directories with service/account/target-service identity metadata.
- Record browser family/build compatibility evidence for registered BYOP
  profiles.
- Teach access-plan output to distinguish owned, BYOP, and observed profiles.
- Keep cleanup conservative for BYOP profiles and disallow destructive cleanup
  for observed-only external processes.

Acceptance:

- Access-plan never silently falls back to an unrelated default profile when
  account or target identity hints require an account-bound lane.
- BYOP registration is auditable through service profile readback and trace or
  event evidence.
- No-launch contract tests prove profile-origin serialization, access-plan
  selection, and cleanup-policy behavior.

Suggested subagent prompt:

```text
Implement Slice A of P14. Add profile-origin and BYOP registration/readback
contracts without launching Chrome. Keep service model, schema, HTTP/MCP, and
generated client types aligned. Validate with focused Rust contract tests,
pnpm service-client/parity tests, cargo fmt, cargo clippy, and git diff check.
```

## Slice B: Lease-Backed Service Tab Handle

Goal: make `requestServiceTab()` return a stable service-owned handle that
software clients can use for follow-on operations without reconstructing CDP
identity from ports, process lists, or target scans.

Deliverables:

- Extend `tab_new` response data and service tab records with a handle
  containing `browserId`, `sessionName`, `tabId`, optional CDP target id,
  current URL/title, profile id, profile origin, cleanup policy, lease metadata,
  and trace/job id.
- Add stale/invalid handle semantics.
- Surface the handle consistently in `service tabs`, `service browsers`, and
  `service trace`.
- Add client helper ergonomics for requesting a tab from access-plan output.

Acceptance:

- A no-launch fixture proves a client can request a service tab and bind
  follow-on commands to the returned handle.
- Tab reuse and stale-handle rejection do not launch duplicate browser/profile
  lanes.
- Existing service request contract tests continue to pass.

Suggested subagent prompt:

```text
Implement Slice B of P14. Extend service tab response/readback with a
lease-backed handle. Keep schemas, generated client files, Rust metadata,
HTTP/MCP output, and CLI formatting aligned. Validate no-launch first; add a
live smoke only if existing live requestServiceTab coverage needs handle
readback.
```

## Slice C: Controlled CDP Attach

Goal: provide a service-owned way for software clients to attach to a leased
CDP target when site policy allows it.

Deliverables:

- Add a controlled attach descriptor or client-helper-managed attach path for
  valid leased service tab handles.
- Gate attach on site policy, access-plan `cdpAttachmentAllowed`, profile
  readiness, and handle freshness.
- Record attach/detach events in service trace and job/session metadata.
- Ensure detach/close semantics do not kill the browser by default.
- Fail closed for unverified, stale, wrong-account, CDP-free, or policy-blocked
  profiles.

Acceptance:

- A no-launch policy test proves attach is denied without a valid leased handle
  and allowed only when policy permits CDP.
- A focused live smoke proves a software client can request a tab, attach,
  perform a bounded read, detach, and find trace evidence.

Suggested subagent prompt:

```text
Implement Slice C of P14. Add controlled CDP attach for leased service tabs,
with policy gates and trace evidence. Preserve service ownership and avoid raw
process ownership leakage. Validate no-launch policy denial/allowance first,
then one live smoke for attach-read-detach.
```

## Slice D: Bounded Evaluate Service Request

Goal: give clients a safe escape hatch between high-level selector commands and
fully raw CDP by adding a bounded evaluate action tied to a service-owned tab.

Deliverables:

- Add `evaluate` as a service request action or equivalent service-owned job.
- Require a leased tab handle or explicit browser/tab binding.
- Enforce `timeoutMs`, `maxReturnBytes`, and failure evidence caps.
- Return compact result, exception, URL/title, truncation metadata, screenshot
  path when requested, console summary, and trace/job id.
- Keep service-client, HTTP, MCP, schemas, and Rust service request metadata
  aligned.

Acceptance:

- Evaluation cannot run without a service-owned tab binding.
- Large returns and slow scripts are capped deterministically.
- Failure diagnostics are compact and linked to trace/job evidence.

Suggested subagent prompt:

```text
Implement Slice D of P14. Add a bounded evaluate service request against a
leased tab handle. Enforce timeout and result-size caps, return compact failure
evidence, and update all contract/client surfaces. Validate with no-launch
contract tests plus focused Rust tests; live smoke only after no-launch passes.
```

## Slice E: Diagnostics And Readiness Evidence

Goal: reduce repeated client-side forensic code by making agent-browser collect
standard browser failure evidence and readiness records.

Deliverables:

- Add a diagnostic bundle helper or action by tab handle or recent job id.
- Include profile id/origin, browser/session/tab ids, URL/title, snapshot
  summary, screenshot path, console errors, recent request summary, route/view
  metadata, browser health, caller context, and trace/job ids.
- Add readiness/freshness probe lifecycle improvements so access-plan can
  block or warn on stale, unverified, wrong-account, or manual-action-required
  states.
- Allow client-supplied provider evidence without moving provider semantics into
  agent-browser.

Acceptance:

- Diagnostic bundle responses are compact by default and capped for large
  fields.
- Readiness records link to service profile, target service/account identity,
  timestamp, and evidence.
- Access-plan gates authenticated work on wrong-account or expired freshness
  evidence where configured.

Suggested subagent prompt:

```text
Implement Slice E of P14. Add compact diagnostics and readiness evidence
contracts without embedding provider selectors. Make access-plan use freshness
evidence for warnings/blocks. Validate contract parity, generated clients, and
focused Rust tests; add live smoke only for evidence capture.
```

## Slice F: Client Ergonomics And Migration Harness

Goal: make the correct broker-first path easy for software projects and prove
the AuraCall migration bridge without making AuraCall a product boundary.

Deliverables:

- Add helpers such as `requestServiceTabFromAccessPlan`,
  `attachServiceTabCdp`, `evaluateServiceTab`, `getServiceTabDiagnostics`, and
  `registerExternalProfile`.
- Add example or smoke fixture that exercises the bridge with generic service
  names and no private profile data.
- Document the migration path in README, docs site, CLI help, and the installed
  agent-browser skill only when user-facing behavior changes.

Acceptance:

- Clients no longer need to destructure access-plan internals for normal
  service-tab work.
- Helper failures include actionable policy reasons.
- Generated types expose the stable contracts without local casts.

Suggested subagent prompt:

```text
Implement Slice F of P14 after Slices A-E are stable. Add client helpers and a
generic migration harness that demonstrates broker-first profile/tab/CDP/evaluate
work without private AuraCall data. Update all user-facing docs required by
AGENTS.md and validate the client, docs, Rust, and smoke surfaces selected by
the changed files.
```

## Coordination Rules

- Sequence matters: A and B define ownership and handle contracts before C and
  D expose CDP/evaluate behavior.
- A subagent may split a slice into smaller commits, but must not merge two
  slices unless the contracts are inseparable.
- Contract changes must update schemas, Rust metadata, HTTP/MCP surfaces, and
  generated client files in the same slice.
- User-facing behavior changes must update `cli/src/output.rs`, `README.md`,
  `skills/agent-browser/SKILL.md`, and docs-site MDX pages per `AGENTS.md`.
- Live smokes must use isolated or explicitly registered profiles and must not
  mutate private AuraCall profiles unless the maintainer explicitly asks.
- Preserve P13 minimal-profile behavior: prefer reuse, wait on leases, and
  launch only when isolation or policy requires a new lane.

## Validation Matrix

Baseline for every slice:

```bash
git diff --check
cargo fmt --manifest-path cli/Cargo.toml -- --check
pnpm validation:select -- --base HEAD
```

Contract or client slices:

```bash
pnpm test:service-client
pnpm test:service-api-mcp-parity
cargo test --manifest-path cli/Cargo.toml service_request -- --test-threads=1
```

Rust service behavior slices:

```bash
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_access -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_profile -- --test-threads=1
```

Live smoke candidates, only after no-launch tests pass:

```bash
pnpm test:service-request-live
pnpm test:service-cdp-tab-streaming-live
```

## Open Questions

- Should controlled attach return a raw websocket descriptor, a temporary
  broker URL, or only a managed client helper?
- Should BYOP profiles ever be eligible for automatic browser close, or only
  tab close and lease release?
- Which readiness evidence fields are mandatory for authenticated account work
  versus optional advisory state?
- Should bounded evaluate be implemented directly as a service request action,
  as a CDP attach helper method, or both?
- What is the minimum live smoke that proves AuraCall migration compatibility
  without depending on private provider state?
