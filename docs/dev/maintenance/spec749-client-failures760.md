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

## Exported constructor validation and guest-clock owner veto

Sixteen additional reviewed constructor/configuration and caller-identity TypeError checks now use the same local correction contract, for 185 admitted checks across the reachable client modules. Original messages remain unchanged; static `checkedFields` identifies the validated options without logging their values. The owner Auth SDK is not loaded and no credential operation is attempted to diagnose a constructor error.

In coordination with ticket 761, Native saved login no longer subtracts guest `Date.now()` from `binding.observedAtMs`, treats future diagnostic timestamps as stale evidence, or expires a retained episode from guest wall-clock movement. Retained created/deadline wall values remain diagnostic and immutable identity metadata. Owner input admission validates the original observation reference, physical context and qualified service-time stamp; presence of a stamp alone is not client proof of freshness. Legacy/unqualified captures remain readable and the real owner's typed recapture refusal is preserved.

Each resolve invocation is bounded by one ordinary OS timer/shared abort signal, independent of trusted evidence expiry. `timeoutMs` remains runtime configuration; timeout reports `native_saved_login_invocation_budget_exhausted`, with an executable same-episode continuation when no unsettled input exists. Unknown input returns the exact original-operation readback interface and request ID, and never replays. Actual owner time refusals retain their occurrence ID and whole recovery object. Existing ACK/context checks remain effective after guest time moves; no new explicit-resume veto is added merely to replace the removed guest expiry.

The prior source fails the updated wall-jump/no-false-expiry regression. The existing Native saved-login harness passes nine focused tests, including forward/backward two-hour wall jumps, owner diagnostic timestamps far in the future, a real stalled-read OS wait budget, original uncertainty, owner time refusal, and pure constructor-correction checks. Existing provider-free hosted receiver, Native receiving adapter, login playbook, hosted playbook and login verifier suites pass, as does checkJs for all touched helper modules. The three standalone owner Auth fixtures require reviewed `AUTH_RECEIPTS_SOURCE`, `RECEIPTS_SOURCE`, and SDK roots; attempts stopped at their explicit missing-source guard. Their fixture execution is not claimed. Pure constructor assertions and checkJs cover the changed validation boundary without importing a private installation. No browser/runtime/provider effect was used in this family.

## Native image and checkbox client time qualification

The image and checkbox clients now also leave capture freshness and cross-capture time qualification to the original owner's service-time admission. They no longer infer authority from guest wall age, diagnostic `evidenceAgeMs`, guest boot clocks or client timestamp ordering. Image/control/puzzle hashes, original operation identity, physical context, site/provider/task policy, exact target selection and independent clearance/access verification remain checked. The client does not treat a returned time stamp's presence as proof that input is fresh.

Native image accepts runtime `timeoutMs` (default 60000 ms); checkbox resolution accepts it with default 10000 ms. These are ordinary per-invocation wait budgets shared across owner reads, selection/progress, and verification. Image selectors retain a 45000 ms per-call wait cap and share the aggregate abort signal, so a second round does not renew the invocation allowance. Diagnostic selector `deadlineAtMs` is for display only; the abort signal and relative caller wait limit bound work. The legacy `steadyNow` option remains accepted for compatibility and supplies no authority. Passive/verification observation-count bounds remain finite.

Budget expiry returns the exact original-operation readback path and never establishes owner expiry or no-effect. The separate client registry now includes the three invocation-wait codes and caller-configurable continuation recipes; missing endpoint/identity bindings remain preconditions rather than guessed executable scope.

The existing provider-free Native image suite passes clock correction, legacy clock changes/malformed diagnostics, a real aggregate OS timeout during the second selection, owner/policy/image failures and lost-ack no replay. The checkbox suite passes its registered families and policy/context/uncertainty negatives; its stale-evidence case now models the actual owner's typed recapture refusal, checks preserved occurrence identity and zero dispatch, instead of letting a guest-derived age number assert authority. A real stalled-read timeout verifies its original-ID recovery. Saved login and all touched-module checkJs also pass. No browser/runtime/provider effect was used.

## Owner-settled no-effect SDK continuation, ticket 762 integration

Saved login, image and checkbox original-operation readback recognize `owner_refused_no_effect` as additive `failed_no_effect` SDK status. This is obtained only from the authenticated exact original task/request/action record with complete logical context; it is never inferred from a caller error string, a thrown no-effect assertion or absence of an operation. Saved-login proof is carried through its internal catch path using a private Symbol, so an external callback cannot settle history by attaching a similarly named field. No checkpoint is promoted to acknowledged and the failed original is never replayed or converted to success.

Results preserve the exact original operation and provide original-ID readback plus a required explicit new episode/request ID. The caller may choose that fresh intent under existing authorization after fresh owner task/capture observation; this local recovery adds no human approval ceremony. Missing identity, current owner denial and uncertain/pending originals remain distinct. No adapter generates a new ID, dispatches a fresh input, or modifies the owner's retained failed outcome to achieve recovery.

The three existing provider-free SDK suites fail the new settled-outcome checks against the preceding source and pass with this change. Saved login also verifies that forged caller-thrown no-effect fields do not produce a terminal settlement. Image and checkbox prove one original read with no additional capture, handoff or input. Existing JS/declaration checkJs passes. Task-operation declaration includes the owner terminal state. The client registry now contains 38 exact codes and the 185-site local argument contract.

This is client integration with ticket 762's server-owned settlement contract, not proof that the native signed no-effect reconciliation workflow has passed acceptance. Native owner proof, archive eligibility and end-to-end continuation qualification remain with that ticket and the coordinator. No browser/runtime/provider effect was used here.
