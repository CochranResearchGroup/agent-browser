# Generic Browser Service Routines Plan

Date: 2026-06-14
State: COMPLETE
Lane: P15
Depends On:
- `docs/dev/notes/2026-06-13-auracall-cdp-feature-requests.md`
- `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`

## Purpose

Plan 0033 made agent-browser a safe broker-first bridge for software clients:
profile ownership, service tab handles, controlled CDP attach, bounded
evaluate, diagnostics, and client helper ergonomics are now available through
service-owned contracts.

This plan takes the next step. It reduces the need for downstream software to
keep broad raw-CDP code by promoting repeated, generic browser-service patterns
into higher-level agent-browser routines. AuraCall is the motivating migration
consumer, but every feature in this plan must be generic enough for other
software clients. Do not add ChatGPT, Gemini, Grok, AuraCall-specific selectors,
or private profile assumptions to agent-browser.

The target outcome is not to remove all CDP. The target is to make raw CDP the
rare escape hatch. Common account detection, page-state probing, tab repair,
UI action, evidence capture, network capture, download capture, and file-input
work should be service-owned, traceable, capped, and live testable.

## Current State

- Plan 0033 is closed. Software clients can request an access plan, acquire a
  service-owned tab, attach CDP through policy gates, run bounded evaluate,
  request diagnostics, and use generated client helpers.
- AuraCall still has provider adapters that use direct CDP domains such as
  `Runtime`, `Page`, `Network`, `DOM`, `Input`, `Browser`, and `Target` for
  browser-service work. Some of this is provider-specific and should stay in
  AuraCall. Some is generic browser-control behavior and should move into
  agent-browser service routines.
- Agent-browser already has many lower-level primitives: navigation, tabs,
  snapshots, screenshots, selectors, input, cookies, storage, uploads,
  download and network tracking, HAR, diagnostics, trace, and service request
  queueing. This plan composes and hardens those primitives into safer
  service-owned routines.

## Operating Invariant

```text
agent-browser owns lifecycle, handles, leases, generic browser routines,
traceability, evidence caps, and live validation; clients own website-specific
recipes, semantic interpretation, and business decisions.
```

## Non-Goals

- Do not mutate the AuraCall repository while implementing this plan.
- Do not migrate AuraCall code in this repo.
- Do not add AuraCall-specific, ChatGPT-specific, Gemini-specific, or
  Grok-specific selectors, labels, URLs, or business rules to agent-browser.
- Do not auto-click captcha, anti-bot, human-verification, payment, or
  destructive confirmation flows.
- Do not bypass CDP-free site policy.
- Do not create a second caller-owned browser lifecycle path.
- Do not make private AuraCall profiles part of tests or fixtures.

When this plan is complete, write a separate handoff note in the AuraCall repo
describing the new generic agent-browser features and suggested migration
mapping. The AuraCall agent will decide whether and how to migrate its code.

## Parent Goal Definition

Goal: make agent-browser the generic browser-service layer that downstream
software can use for common CDP-backed tasks without writing raw CDP for every
provider adapter.

Done means:

- clients can run service-owned page-state and account-detection probes using
  generic routines plus caller-provided recipes or instructions;
- stale service tab handles can be refreshed, repaired, or rejected with trace
  evidence without launching unrelated browser lanes;
- common UI interactions are available as high-level service routines with
  visibility checks, evidence, and safety gates;
- network and download evidence can be captured through allowlisted, capped,
  trace-linked service routines;
- file-input and upload flows have a generic service-owned path with fallback
  diagnostics;
- every public routine is available through HTTP, MCP, generated client types,
  docs, and skill guidance where user-facing;
- each slice has no-launch contract tests and at least one provider-neutral
  live smoke that proves the feature against an isolated browser session;
- no AuraCall repo files are modified during implementation;
- a final AuraCall handoff note is written after the generic features land.

## Generic Recipe Model

Several routines in this plan need website-specific knowledge without
embedding provider semantics in agent-browser. Use a generic recipe model:

- A recipe can be inline request data, a named local service recipe, or a
  caller-provided instruction bundle.
- Recipes describe what to observe or extract, not what business decision to
  make.
- Recipes must be bounded: timeouts, max returned bytes, max matched elements,
  max captured response bodies, max screenshots, and explicit action limits.
- Recipes must be auditable: request id, caller context, target handle, recipe
  fingerprint, observed URL/title, and trace/job ids.
- Recipes must be portable: examples and tests use generic pages, not private
  provider UIs.

Identity/account detection is a good use case for this model. Agent-browser
can provide a general routine that loads a page, evaluates bounded detectors,
normalizes candidate identity evidence, and records freshness. The client may
provide the specific detector recipe for a website or service. Agent-browser
must not decide what a ChatGPT, Gemini, Grok, bank, CRM, or email account means
beyond the generic target/account fields supplied by the caller.

## Subagent Work Allocation

Use one subagent per slice. Each subagent owns a narrow, generic contract and
must prove it with no-launch tests before any live smoke. Subagents should
report:

```text
Slice:
Goal:
Generic contract:
Files changed:
Public contract delta:
No-launch validation:
Live smoke:
Residual risks:
Next slice readiness:
AuraCall handoff note impact:
```

Recommended sequence:

1. Slice A: generic probe and identity evidence contract.
2. Slice B: tab handle refresh and stale-target repair.
3. Slice C: UI action routines.
4. Slice D: network evidence capture.
5. Slice E: download and file-input routines.
6. Slice F: composed workflow harness and client ergonomics.
7. Slice G: final AuraCall handoff note.

Slices C, D, and E can start discovery after Slices A and B define shared
recipe, evidence, and handle-refresh vocabulary. Slice F waits for A-E. Slice G
waits for the generic features and validation evidence.

## Slice A: Generic Probe And Identity Evidence

State: DONE for the generic `probe` action. Remaining slices are still open.

Goal: add a service-owned probe routine that can collect bounded page-state and
identity/account evidence from a service tab handle using generic caller
recipes.

Deliverables:

- Add a service request action such as `probe` or `run_probe`.
- Accept a valid `serviceTabHandle`, caller context, timeout, max return bytes,
  and one or more bounded detector recipes.
- Support generic detector types:
  - evaluate expression returning JSON-serializable evidence;
  - selector/text extraction with visibility filters;
  - URL/title expectation;
  - cookie or storage presence checks where existing policy allows;
  - client-supplied evidence payload for probes run outside agent-browser.
- Normalize identity/account evidence into generic fields such as
  `detectedIdentity`, `detectedAccountId`, `confidence`, `source`,
  `observedAt`, `targetServiceId`, and `accountId`.
- Update service profile readiness/freshness records only when the caller
  explicitly asks for a freshness update and supplies target/account context.
- Fail closed on stale handles, missing target identity for freshness updates,
  unbounded recipes, or CDP-free policy conflicts.

Acceptance:

- A no-launch test proves invalid recipes, stale handles, missing caps, and
  wrong freshness-update inputs are rejected.
- A live smoke opens a generic page with a visible synthetic account label,
  runs a probe recipe, records freshness for a generic target/account, refreshes
  access-plan output, and verifies the evidence appears in diagnostics or trace.
- The routine works without provider names or private profile data.

Implementation:

- Added service request action `probe` to the canonical request contract,
  daemon action list, HTTP `/api/service/request`, MCP `service_request`, and
  generated service client declarations.
- Added provider-neutral detector support for `url_title`, `selector_text`,
  `evaluate`, and `client_evidence`.
- Added generic identity normalization fields:
  `detectedIdentity`, `detectedAccountId`, `expectedIdentity`, `confidence`,
  and `source`.
- Added explicit `probe.recordFreshness` handling. The daemon records target
  readiness only when the caller supplies target/account/profile context, and
  profile freshness updates now merge account IDs without replacing unrelated
  profile metadata.
- Added client helpers `createServiceProbeRequest()`, `requestServiceProbe()`,
  and `probeServiceTab()`.
- Added live smoke `scripts/smoke-service-probe-live.js` and package script
  `pnpm test:service-probe-live`.
- Updated `README.md`, CLI help, docs site command reference, and
  `skills/agent-browser/SKILL.md`.

Validation evidence:

- `pnpm test:service-client` passed.
- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed.
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --nocapture`
  passed, including MCP and HTTP probe rejection/forwarding tests.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.
- `pnpm test:service-api-mcp-parity` passed.
- `pnpm test:service-probe-live` passed. The smoke creates a generic service
  profile, opens an isolated synthetic account page, runs `url_title`,
  `selector_text`, and `evaluate` detectors through `action=probe`, records
  target/account freshness, verifies the profile collection has a fresh
  readiness row, and refreshes access-plan output to confirm the broker sees
  the generic profile as fresh.

Suggested subagent prompt:

```text
Implement Slice A of P15. Add a generic bounded probe service request that can
extract page-state and identity/account evidence from a service tab handle using
caller-provided recipes. Keep it provider-neutral. Update HTTP, MCP, schemas,
generated client types, docs, and skill guidance as required. Validate no-launch
contract failures first, then a live synthetic-account probe smoke.
```

## Slice B: Tab Handle Refresh And Stale-Target Repair

State: DONE for the generic `tab_handle_refresh` action. Remaining slices are
still open.

Goal: let clients recover from stale or incomplete tab handles through
service-owned state reconciliation instead of rescanning DevTools targets or
launching another browser lane.

Deliverables:

- Add a service request action such as `tab_handle_refresh`.
- Accept a `serviceTabHandle`, desired URL/site/account hints, and a repair
  policy: `reject_only`, `reuse_compatible`, or `open_if_missing`.
- Reconcile service-owned browser/tab state and classify candidates:
  exact handle still valid, matching target found, compatible blank tab,
  compatible same-origin tab, incompatible profile, closed tab, dead browser,
  CDP-free route, or no candidate.
- Return a refreshed handle or a structured rejection with discarded candidate
  reasons.
- Record repair decisions in service trace.
- Preserve minimal profile reuse and avoid duplicate profile lanes.

Acceptance:

- No-launch tests prove candidate classification and repair-policy decisions.
- A live smoke opens a generic page, obtains a handle, closes or invalidates the
  tab in a controlled way, runs refresh with `reject_only` and
  `open_if_missing`, and proves no unrelated profile lane is launched.
- Trace output explains why each candidate was reused, ignored, or replaced.

Implementation:

- Added service request action `tab_handle_refresh` to the canonical request
  contract, daemon action list, HTTP `/api/service/request`, MCP
  `service_request`, and generated service client declarations.
- Added refresh inputs for `serviceTabHandle`, optional `desiredUrl`, and
  `repairPolicy` values `reject_only`, `reuse_compatible`, and
  `open_if_missing`.
- Added retained-state and live-browser candidate classification for exact
  handles, matching targets, compatible blank tabs, compatible same-origin
  tabs, closed tabs, dead browsers, and incompatible tabs.
- Added daemon behavior that refuses to launch an unrelated browser lane for
  refresh. It requires the routed service session to be running, can return a
  structured stale rejection, can reuse a compatible live tab, and can open a
  replacement tab only under `open_if_missing`.
- Added service trace events for refresh decisions with candidate counts and
  candidate evidence.
- Added client helpers `createServiceTabHandleRefreshRequest()`,
  `requestServiceTabHandleRefresh()`, and `refreshServiceTabHandle()`. The
  refresh helper intentionally accepts stale handles while ordinary follow-on
  helpers continue to reject them.
- Added live smoke `scripts/smoke-service-tab-handle-refresh-live.js` and
  package script `pnpm test:service-tab-handle-refresh-live`.
- Updated `README.md`, CLI help, docs site command reference, and
  `skills/agent-browser/SKILL.md`.

Validation evidence:

- `pnpm test:service-client` passed, including generated contract checks,
  TypeScript checkJs coverage, export checks, service request helper tests,
  and the no-launch client examples.
- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed.
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --nocapture`
  passed, including MCP and HTTP `tab_handle_refresh` validation and
  forwarding tests.
- `cargo test --manifest-path cli/Cargo.toml tab_handle_refresh -- --nocapture`
  passed, including no-launch retained candidate classification, live-page
  classification, and refreshed handle shape tests.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.
- `pnpm test:service-api-mcp-parity` passed.
- `pnpm test:service-tab-handle-refresh-live` passed. The smoke opens an
  isolated generic page, obtains a service tab handle, proves exact refresh,
  closes the original tab in a controlled live session, proves `reject_only`
  returns structured stale evidence, proves `open_if_missing` repairs to a
  valid handle by opening or reusing a compatible blank tab, and verifies trace
  jobs plus refresh events expose repair decisions and candidate counts.

Suggested subagent prompt:

```text
Implement Slice B of P15. Add generic service tab handle refresh and stale
target repair. Do not expose raw DevTools target scanning to clients. Preserve
profile reuse and record candidate decisions in trace. Validate with no-launch
classification tests and one isolated live stale-handle smoke.
```

## Slice C: Generic UI Action Routines

State: DONE for the generic `ui_action` action. Remaining slices are still
open.

Goal: replace repeated direct `Runtime.evaluate` plus `Input.dispatch*` patterns
with higher-level service-owned UI actions.

Deliverables:

- Add generic actions or one parameterized `ui_action` service request for:
  - find visible element or control;
  - click visible element;
  - set text in input, textarea, or contenteditable target;
  - open menu and choose item by generic selector/text recipe;
  - wait for selector/text/URL/title predicate;
  - dismiss generic blocking surfaces when caller supplies allowed labels.
- Use existing selector, input, snapshot, and diagnostics primitives where
  possible.
- Require explicit caps: timeout, max candidates, max text bytes, max action
  attempts, and evidence capture level.
- Return structured action evidence: matched candidate, visibility summary,
  coordinates when used, post-action URL/title, and diagnostics-on-failure.
- Keep destructive actions opt-in. Do not special-case provider labels.

Acceptance:

- No-launch tests cover request shape, safety gating, and client helper
  generation.
- A live smoke serves a generic HTML page with a menu, dialog, editable field,
  and delayed status text, then proves find/click/type/menu/wait routines.
- The smoke verifies trace jobs and a compact diagnostics bundle on one
  intentionally failed UI action.

Implementation:

- Added service request action `ui_action` to the canonical request contract,
  daemon action list, HTTP `/api/service/request`, MCP `service_request`, and
  generated service client declarations.
- Added a bounded `uiAction.steps` recipe model that requires a valid
  `serviceTabHandle`, positive timeout, and nonempty step list. `maxActions`
  defaults conservatively and is capped; `maxTextBytes` caps extracted text.
- Added generic step support for `find`, `focus`, `fill`, `type`, `select`,
  `menu_select`, `click`, `wait`, `clear`, and guarded `dialog`.
- Reused existing browser primitives for click/fill/type/select/wait/focus and
  clear, added a generic visible-candidate `find` evaluator, and composed
  `menu_select` as menu click plus option click.
- Added per-step evidence with step id, type, selector, result, timestamps,
  and post-step URL/title. Failed steps return `ok: false`,
  `failedStepIndex`, before/after page evidence, completed step evidence, and
  optional compact diagnostics when requested.
- Added guarded dialog validation with allowed label matching and bounded
  timeout behavior so modal handling cannot hang a service job indefinitely.
- Added client helpers `createServiceUiActionRequest()`,
  `requestServiceUiAction()`, and `runServiceUiAction()`.
- Added live smoke `scripts/smoke-service-ui-action-live.js` and package script
  `pnpm test:service-ui-action-live`.
- Updated `README.md`, CLI help, docs site command reference, and
  `skills/agent-browser/SKILL.md`.

Validation evidence:

- `pnpm generate:service-client` passed.
- `pnpm test:service-client` passed, including generated contract checks,
  TypeScript checkJs coverage, export checks, service request helper tests,
  and no-launch client examples.
- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed.
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --nocapture`
  passed, including MCP and HTTP `ui_action` rejection and forwarding tests.
- `cargo test --manifest-path cli/Cargo.toml ui_action -- --nocapture`
  passed, covering the focused no-launch `ui_action` HTTP and MCP contract
  tests.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.
- `pnpm test:service-api-mcp-parity` passed.
- `pnpm test:service-ui-action-live` passed. The smoke opens an isolated
  generic page, runs a service-owned `ui_action` recipe with find, focus, fill,
  select, click, wait, and menu selection, verifies per-step evidence, runs one
  intentionally failed UI action with diagnostics-on-failure, and verifies
  trace jobs include the `ui_action` work.

Suggested subagent prompt:

```text
Implement Slice C of P15. Add generic UI action service routines for visible
element discovery, verified click, text entry, menu selection, and bounded waits.
Use generic selectors and labels only. Validate no-launch contracts, then a live
synthetic UI page smoke with trace and failure diagnostics.
```

## Slice D: Network Evidence Capture

State: DONE for the generic `network_capture` action. Remaining slices are
still open.

Goal: make selected network-response evidence a capped service routine instead
of asking clients to subscribe to raw `Network` CDP events and call
`getResponseBody`.

Deliverables:

- Add a service request action such as `network_capture` or an arm/wait pair.
- Allow callers to specify URL patterns, resource types, method filters,
  status ranges, max events, max body bytes, max duration, and whether response
  bodies are captured.
- Default to metadata-only. Body capture must be explicit and capped.
- Redact or omit headers and bodies by default unless the caller explicitly
  asks for allowed fields.
- Link captured events to service trace and diagnostics.
- Provide generated client helpers for common arm/wait/read workflows.

Acceptance:

- No-launch tests prove caps, invalid filters, and redaction defaults.
- A live smoke loads a local generic test page that fetches small JSON and a
  larger text payload, captures only allowlisted responses, verifies truncation,
  and reads trace evidence.
- The routine does not expose an unbounded network listener to clients.

Implementation:

- Added service request action `network_capture` to the canonical request
  contract, daemon action list, HTTP `/api/service/request`, MCP
  `service_request`, and generated service client declarations.
- Added bounded `networkCapture` recipes with filters for URL patterns,
  methods, resource types, and status. Recipes require positive timeout and
  `maxEvents`; metadata is the default capture mode.
- Added explicit body capture with `captureBodies: true` plus positive
  `maxBodyBytes`. The daemon caps returned bodies, reports original byte
  counts, and marks truncation.
- Added header redaction by default. Request or response headers are only
  returned when requested and filtered through `allowedHeaderNames`.
- Added optional `trigger: { type: "reload" }` so callers can arm capture
  before triggering provider-neutral page activity.
- Added client helpers `createServiceNetworkCaptureRequest()`,
  `requestServiceNetworkCapture()`, and `captureServiceNetwork()`.
- Added live smoke `scripts/smoke-service-network-capture-live.js` and package
  script `pnpm test:service-network-capture-live`.
- Updated `README.md`, CLI help, docs site command reference, and
  `skills/agent-browser/SKILL.md`.

Validation evidence:

- `pnpm generate:service-client` passed.
- `pnpm test:service-client` passed, including generated contract checks,
  TypeScript checkJs coverage, export checks, service request helper tests,
  and no-launch client examples.
- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed.
- `cargo test --manifest-path cli/Cargo.toml network_capture -- --nocapture`
  passed, covering focused HTTP and MCP rejection and forwarding tests.
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --nocapture`
  passed, including the full HTTP and MCP service request action loop.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.
- `pnpm test:service-api-mcp-parity` passed.
- `pnpm test:service-network-capture-live` passed. The smoke serves a local
  generic page with small JSON and larger text fetches, opens it through a
  service-owned tab, captures metadata-only response evidence with allowlisted
  response headers and redaction, captures explicit capped bodies, verifies the
  large body is truncated, and verifies trace jobs include
  `network_capture`.

Suggested subagent prompt:

```text
Implement Slice D of P15. Add capped generic network evidence capture for
service-owned tabs. Metadata-only by default; response bodies require explicit
caps. Validate filters and redaction with no-launch tests, then a local live
fetch smoke.
```

## Slice E: Download Capture And File Input

State: DONE for the generic `file_transfer` action. Remaining slices are still
open.

Goal: replace direct `Browser.setDownloadBehavior`, `Page.setDownloadBehavior`,
`DOM.querySelector`, and `DOM.setFileInputFiles` usage with generic
service-owned download and file-input routines.

Deliverables:

- Add a download capture routine that arms a download directory, performs or
  waits for an action, returns downloaded file metadata, and records trace
  evidence.
- Add a file-input routine that finds an input by selector or visible label
  recipe, sets allowed files, and verifies selected file names where possible.
- Use existing upload/download primitives where available and align them with
  service request metadata.
- Require explicit allowed paths, max files, max wait time, max bytes when
  available, and caller context.
- Return compact metadata: local path, file name, size, MIME when known, source
  URL when known, and diagnostics on timeout or selector failure.

Acceptance:

- No-launch tests prove path safety, cap enforcement, and helper types.
- A live smoke serves a generic upload/download page, sets a temp file through
  the file-input routine, triggers a generated download, waits for capture, and
  verifies trace and diagnostics.
- No private file paths or provider pages are used in tests.

Implementation notes:

- Added service request action `file_transfer` across Rust service contracts,
  HTTP `/api/service/request`, MCP `service_request`, JSON schema, generated
  client declarations, client helpers, README, docs site, CLI help, and
  `skills/agent-browser/SKILL.md`.
- `fileTransfer.upload` accepts a selector or visible `labelText`, explicit
  `files`, `allowedPaths`, `maxFiles`, and optional selected-file-name
  verification.
- `fileTransfer.download` accepts a selector, destination directory,
  `allowedDirectories`, optional expected file name, optional `maxBytes`, and
  optional `captureMode`. The default capture path fetches the link from the
  browser page context with credentials, writes the capped body into the
  allowlisted directory, and returns compact file metadata. `captureMode:
  "browser"` keeps the browser-event fallback available for true browser
  download flows.
- Failure responses can include compact diagnostics when
  `captureEvidenceOnFailure` or `includeDiagnosticsOnFailure` is true.

Validation evidence:

- `pnpm generate:service-client`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-file-transfer-live`
- `git diff --check`

Suggested subagent prompt:

```text
Implement Slice E of P15. Add generic download capture and file-input service
routines with path safety, caps, trace evidence, and generated client helpers.
Validate with no-launch contract tests and a local synthetic upload/download
live smoke.
```

## Slice F: Composed Workflow Harness And Client Ergonomics

State: DONE for the generic composed service-client workflow. Slice G remains
open.

Goal: prove the new routines compose into a provider-neutral browser-service
workflow that downstream projects can adopt without raw CDP for common tasks.

Deliverables:

- Extend or add an example under `examples/service-client/` that performs:
  access plan, service tab request, probe, UI action, network capture, file
  input, download capture, diagnostics, and detach.
- Add generated helper aliases that make the safe path obvious.
- Add a no-launch test that uses fake fetch calls to assert request sequence,
  caps, and failure handling.
- Add a live smoke that uses a local generic HTML test page and isolated temp
  files.
- Update README, docs site, CLI help, and `skills/agent-browser/SKILL.md` for
  any user-facing routines.

Acceptance:

- The example demonstrates a full broker-first workflow without provider
  selectors, private data, or direct CDP.
- The live smoke proves the composed workflow through the HTTP service route.
- The generated TypeScript declarations expose stable action data types.

Implementation notes:

- Added `examples/service-client/composed-workflow.mjs`, a copyable
  broker-first client workflow that performs access-plan read, service tab
  request, policy-gated CDP attach, generic identity/account probe, generic UI
  action recipe, capped network capture, allowlisted upload and download file
  transfer, compact diagnostics, and detach.
- The composed example keeps website-specific selectors, labels, expected text,
  account extraction, and file paths in caller-owned recipe data. It does not
  embed provider selectors or AuraCall-specific logic.
- Added generic tab launch `tabParams` support to the example so callers can
  pass ordinary launch options such as `headless` and `waitUntil` without
  forking the workflow.
- Added `scripts/test-service-client-composed-workflow.js`, a no-launch fake
  fetch smoke that asserts request order, recipe caps, helper payload shape, and
  failure cleanup. It verifies `cdp_detach` runs after a mid-workflow
  `network_capture` failure.
- Added `scripts/smoke-service-composed-workflow-live.js` and package script
  `pnpm test:service-composed-workflow-live`. The smoke serves a local generic
  page with synthetic account evidence, an input, a button, an API response, an
  upload input, and a generated download, then runs the composed workflow
  through the HTTP service route against an isolated daemon.
- Added `transferServiceFiles()` and `requestServiceFileTransfer()` to the
  generated TypeScript declaration surface so file-transfer helpers are stable
  for service-client consumers.
- Updated `README.md`, `docs/src/app/commands/page.mdx`,
  `docs/src/app/service-mode/page.mdx`, `skills/agent-browser/SKILL.md`,
  `examples/service-client/package.json`, and `package.json`.

Validation evidence:

- `pnpm generate:service-client` passed.
- `pnpm test:service-client` passed.
- `pnpm test:service-api-mcp-parity` passed.
- `pnpm test:service-client-contract` passed.
- `pnpm test:service-composed-workflow-live` passed. The smoke proved access
  plan, tab handle, policy-gated attach, identity/account probe, UI action,
  network capture, upload, download capture, diagnostics, detach, and service
  trace readback against an isolated local fixture.
- `git diff --check` passed.

Suggested subagent prompt:

```text
Implement Slice F of P15 after Slices A-E land. Add a generic composed workflow
example and smoke that proves access-plan, tab, probe, UI action, network
capture, file input, download capture, diagnostics, and detach. Keep all content
provider-neutral and update user-facing docs.
```

## Slice G: AuraCall Handoff Note

State: DONE. Handoff note written in the AuraCall repo.

Goal: after the generic agent-browser features are implemented and validated,
write a handoff note in the AuraCall repo for its agent to decide migration
work.

Deliverables:

- Add a dated handoff note under the AuraCall repo, likely
  `../auracall/docs/dev/notes/`, after all generic feature slices are complete.
- Summarize the new agent-browser feature set, public contracts, client helper
  names, validation evidence, and migration opportunities.
- Map direct CDP categories to generic routines:
  - `Runtime.evaluate` page-state and identity reads to probe routines;
  - `Target` rescans to tab handle refresh;
  - `Input` mouse/key flows to UI action routines;
  - `Network` listeners and body reads to network evidence capture;
  - `Browser/Page.setDownloadBehavior` to download capture;
  - `DOM.setFileInputFiles` to file-input routines;
  - post-failure screenshots and URL/title reads to diagnostics.
- State explicitly that AuraCall owns provider-specific selectors, DOM
  interpretation, identity mismatch rules, account-mirror cursors,
  materialization policy, and migration sequencing.

Acceptance:

- The handoff note is written only after the agent-browser generic routines are
  implemented and validated.
- The handoff note does not mutate AuraCall runtime state or provider code.
- The handoff gives AuraCall's agent enough detail to plan migration without
  requiring chat context.

Implementation notes:

- Added
  `/home/ecochran76/workspace.local/auracall/docs/dev/notes/2026-06-14-agent-browser-generic-service-routines-handoff.md`.
- The handoff summarizes the new generic service routines, helper names,
  validation evidence, and suggested direct-CDP migration mapping.
- The note explicitly leaves provider-specific selectors, DOM interpretation,
  identity mismatch rules, account-mirror cursors, materialization policy, and
  migration sequencing to AuraCall.
- No AuraCall source code, provider adapters, runtime state, roadmap, runbook,
  or account data were changed.

Validation evidence:

- `git diff --check` passed in `/home/ecochran76/workspace.local/auracall`.

Suggested subagent prompt:

```text
Implement Slice G of P15 only after the generic agent-browser slices are
complete. Write a concise AuraCall repo handoff note describing the new generic
agent-browser routines, validation evidence, and possible migration mapping.
Do not change AuraCall source code or runtime state.
```

## Validation Matrix

Baseline for every slice:

```bash
git diff --check
cargo fmt --manifest-path cli/Cargo.toml -- --check
pnpm validation:select -- --base HEAD
```

Contract or generated-client slices:

```bash
pnpm test:service-client
pnpm test:service-api-mcp-parity
cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1
```

Rust service behavior slices:

```bash
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_access -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture
```

Docs and skill changes:

```bash
pnpm --dir docs build
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
```

Live smoke requirements:

- Each feature slice must add or update one provider-neutral live smoke.
- Live smokes must use isolated temp homes, synthetic pages, local test servers
  or data URLs, and temporary files.
- Live smokes must not depend on private provider state or authenticated
  accounts.
- Live smokes must prove service trace evidence, not only API success.
- Live smokes should be named in `package.json` under `test:*live` or a clearly
  scoped no-launch test name.

## Follow-Up Questions For Future Work

These are not blockers for Plan 0034 completion. They are follow-up design
questions for later feature hardening:

- Whether recipes should become durable named service recipes in addition to
  inline request data.
- Whether probe freshness writes should eventually split into a separate
  explicit action from probe execution.
- Whether generic network capture needs a richer response-body redaction
  vocabulary.
- Which routines need dashboard affordances beyond API, MCP, client helper, and
  skill guidance.
