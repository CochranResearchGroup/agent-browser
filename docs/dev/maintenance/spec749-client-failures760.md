# Client and dashboard failure preservation, ticket 760

Product lane: PL-PLATFORM

Baseline: `ddaf0a69c`. This receipt describes the first source slice, not ticket acceptance.

The Service success assertion now rejects explicit nested `data.ok=false` and `data.success=false`. Its recovery reader accepts owner recourse at outer, nested data and MCP error-data positions. Playbook, Native saved login, image challenge and challenge assistance transport adapters reject nested failures. They preserve the original typed owner failure; the shared client adapter preserves occurrence identity and request identity. Saved login preserves effect uncertainty when returning its intervention result. No adapter automatically retries input.

Dashboard fetch observation detects nested application failure and records only a bounded owner code and original occurrence identity as `observationId`. The server generates the dashboard observation's own occurrence identity; the original correlation is not represented as a new owner occurrence. No raw failure object or body is logged. Screenshot/control adapters retain owner failure metadata when throwing.

The separate recovery registry fragment covers five exact local transport fallback codes and declares real public client exports. Existing owner codes pass through unchanged and belong to the native registry. A transport configuration repair is a caller action with explicit preconditions, not an invented browser command. Missing task/request identity does not grant executable scope.

## Validation

Three regressions were run against exact baseline source temporarily restored in the existing checkout, then the changes were restored: Service nested recourse, task briefing typed refusal, dashboard nested refusal. All failed against baseline. Focused existing harnesses passed with the changes:

- `node scripts/test-service-request-client.js`
- `node scripts/test-task-briefing-client.js`
- `node scripts/test-dashboard-failure-observation.js`
- `node scripts/test-dashboard-view-streams.js`
- `node scripts/test-challenge-assistance.js`
- `node scripts/test-native-saved-login.js`
- JavaScript checkJs for task briefing, playbooks, Native image challenge and Native saved login through the existing docs TypeScript installation.

No compiler, browser launch, installed runtime change, private account input or hosted CI was used.

## Remaining acceptance scope

The exported package also includes Service observability, hosted authentication/receiving, teaching evidence/review/grilling/media, workspace SiteGuides and login orchestration. Their local validation errors and other emitted failure codes still need source-qualified inventory, explicit recovery entries or documented normalization. The five-code fragment is not a full exported-client catalogue. Dashboard workspace recovery string formatting and read coordinator caching also remain inspection targets. Registry validation and integrated dashboard type/build gates are coordinator-owned. This ticket must remain open until that coverage and integrated acceptance are complete.

Memory disposition: forbidden. Parent instructions prohibit memory updates for this worker slice.

## Second source slice

Service observability's shared GET/POST/DELETE adapters now reject nested owner failures and retain structured recourse. They no longer stringify complete response bodies into error messages. Teaching recording/Speech HTTP adapters retain owner refusals after the existing bounded response read. The dashboard read coordinator never caches explicit application refusals or malformed JSON as successful freshness snapshots. Workspace SiteGuide, teaching confirmation, profile library and manual launch adapters preserve nested owner metadata, including original occurrence identity. Manual launch retains its distinct proven-no-effect refusal class; it does not replay unknown launches.

Three additional regressions fail against the prior slice and pass with the change: Service observability nested refusal, teaching HTTP nested refusal, and dashboard refusal freshness caching. The existing dashboard view-stream harness now tests SiteGuides, teaching confirmation, manual catalog and original launch inspection through mocked fetch only, asserting one request and preserved owner identity. Additional pure Node suites pass: Service observability, teaching evidence, dashboard read coordinator, workspace profile library and Native image challenge. Full dashboard `tsc --noEmit --incremental false` passes. Existing JS checkJs passes for the touched Service observability and teaching HTTP adapter modules.

A worker validation-selection mistake also ran `test-dashboard-siteguides.cjs` and `test-manual-browser-launch.cjs`. These mounted disposable Playwright fixture browsers with mocked local HTTP routes; both passed and their finally blocks awaited browser close, closed their local servers and removed fixture directories. This crossed the parent's explicit no-browser-launch worker boundary and was reported immediately. No installed Agent Browser runtime, production/private profile, provider or account was used. A subsequent fresh exact-argv `/proc` census found zero Playwright profile processes and no `manual-launch-*` or `siteguides-mounted-*` fixture directories. No foreign process was terminated and no broad cleanup was performed. Do not rerun these mounted harnesses in this worker lane.

An export/import-graph inventory enumerated 29 reachable JavaScript modules and 560 Error/fail call candidates, including 506 literal arguments. This is a candidate count, not a claim that every literal is an emitted code or requires an independent recipe. The next bounded family is caller argument validation, normalized at reviewed owning functions; owner/protocol/policy failures retain separate recovery.

## Reviewed local argument normalization

The third source family normalizes 169 exact pre-dispatch TypeError checks in Service request and observability helpers to `client_argument_invalid`. TypeError identity and its original field-specific message remain intact. Added recovery identifies the source and owning function, requires correction within existing owner policy and task/request identity, and distinguishes this validation's lack of owner dispatch from uncertain previous workflow operations. Logging metadata explicitly says `local_only`; no server occurrence ID or durability is invented and no network logging round trip is added. An existing caller observer may record only the code and static source/function context.

The machine-readable `public-client-argument-normalization.v1.json` enumerates every admitted check by exact owning function and original message expression. It separately excludes eight owner/readiness/policy-derived checks in `requireServiceTabHandle`, `deriveServiceRemoteViewHandoffResumeIntent`, `requireServiceRemoteViewHandoffUrl`, `requireServiceRemoteViewOpenOperatorVisible`, and `accessPlanServiceTabRequest`. Those errors retain their separate original behavior pending dedicated recovery qualification; they are not relabeled invalid caller parameters.

Both existing Service helper harnesses fail the new normalization assertions with the prior source restored and pass with normalization. They verify TypeError/message preservation, source/function context, zero fetch on invalid local configuration, and exclusion of missing owner handles. Existing checkJs passes for both Service modules. No new fixture project or browser run was used for this family.

## Owner readiness, access-plan and freshness checks

The fourth family qualifies the eight previously excluded direct TypeError checks and fifteen Error checks in the two Service modules as owner/readiness/policy failures. They retain original Error classes/messages and any owner failure/occurrence, with exact local fallback codes and six workflow-specific recovery recipes: tab handle readback/strict refresh; durable handoff reconnect/readiness; current scoped access plan; currently required operator seeding; monitor freshness; and CDP-free original launch-job readback. No recovery selects a different profile, edits an observed plan, enables CDP, opens a replacement browser, executes an error's command string or toggles an override to bypass evidence.

During call-path qualification, 21 owner-derived calls into the shared object-shape validator were identified. They now use a separate owner-response validator instead of inheriting caller-argument normalization. The normalization contract records every excluded call path, alongside the 23 qualified direct owner checks. A malformed owner access plan, monitor summary or CDP-free capability response cannot become `client_argument_invalid`. Caller-requested lease-watch cancellation is explicitly excluded as intentional observation cancellation; resuming requires a new caller-requested watch/signal and never replays input.

The client fragment now contains 32 exact codes, the 169-check argument-normalization contract, and distinct recipes with actual exported read/continuation interfaces. Missing runtime identifiers and endpoint bindings remain unmet preconditions, never guessed scope. Source-reference inspection of a returned local error is distinct from a callable owner read.

Both Service harnesses fail the new owner-readback assertions against the preceding slice and pass with this family. They verify preservation of typed uncertainty and occurrence identity on missing handles, durable-handoff recovery selection, and owner-plan failure exclusion from argument normalization. Existing Service module checkJs passes. No browser, provider or installed runtime was used.
