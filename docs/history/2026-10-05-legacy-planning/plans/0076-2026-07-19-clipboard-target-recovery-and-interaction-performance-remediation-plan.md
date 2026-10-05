# Plan 0076: Clipboard Target Recovery And Interaction Performance Remediation

Date: 2026-07-19
Status: Complete
Lane: P76

## Goal

Turn the retained LinkedIn clipboard incident into bounded, test-backed
product behavior for clipboard reads, target recovery, semantic locators,
dependent command execution, latency evidence, and retained tab history.

The completed behavior must keep the retained browser and profile authoritative,
avoid leaving unresolved CDP commands or unusable targets behind, and expose
enough structured evidence for callers to distinguish a site action failure
from clipboard, target, locator, queue, or retained-history failures.

## Source Evidence

- Incident note:
  `docs/dev/notes/2026-07-19-clipboard-read-target-recovery-performance.md`
- Current clipboard action:
  `cli/src/native/actions.rs::handle_clipboard`
- Current CDP command lifecycle:
  `cli/src/native/cdp/client.rs::CdpClient::send_command`
- Current evaluation path:
  `cli/src/native/browser.rs::BrowserManager::evaluate`
- Current role locator:
  `cli/src/native/actions.rs::handle_getbyrole`
- Current CLI batch path:
  `cli/src/main.rs::run_batch`
- Current service status serialization:
  `cli/src/native/actions.rs::handle_service_status`

The incident source included private authenticated browser state. Durable repo
evidence must remain redacted and must not retain copied URLs, cookies,
authentication state, private page text, screenshots, or raw profile data.

## Review Mediation Decisions

The following decisions reconcile the incident recommendations with the review
of the current implementation:

1. A shorter clipboard timeout must be implemented inside the CDP command
   lifecycle. An outer timeout around `BrowserManager::evaluate` is not
   sufficient because cancellation could bypass pending-command cleanup.
2. A clipboard read that resolves to an empty string is successful output, not
   a typed error. Typed failures cover permission denial, unresolved promise,
   unavailable clipboard backend, recovery failure, and CDP transport failure.
3. A CDP page-session detach or reattach is not assumed to repair an unresolved
   renderer promise. The recovery primitive must be proved by a regression test
   that demonstrates a normal evaluation after timeout on the same retained
   browser. If same-target repair cannot be proved, replacement-tab guidance is
   the explicit supported outcome.
4. Locator coverage must reproduce accessible-name behavior that the current
   `aria-label` or `textContent` approximation misses. A conventional
   portal-mounted menu item is not a sufficient regression fixture because the
   current implementation already finds ordinary document descendants.
5. Dependent batching may pin a target only while steps preserve target
   identity. Navigation, tab creation, close, switch, or other target-changing
   steps must force revalidation or reject pinned-target execution.
6. Ordinary service status may compact closed tab history only through an
   explicit bounded projection. The persisted lifecycle authority and the
   diagnostic retrieval surface must remain complete enough for routing,
   incidents, and audit.
7. Runtime observations are treated as evidence only when a privacy-safe
   artifact or deterministic regression fixture records the command, timing,
   and outcome.

## Deep-Module Design

### CDP command lifecycle module

The `CdpClient` command interface owns command registration, transport send,
deadline enforcement, pending-entry cleanup, late-response tolerance, and
typed timeout classification. Clipboard actions select a shorter deadline but
do not recreate lifecycle logic.

The interface must keep the existing default-timeout call path compatible while
adding one explicit per-command deadline path. Tests exercise the module through
a local WebSocket adapter and observable command results rather than inspecting
private map operations as the primary assertion.

### Clipboard operation module

Clipboard read and write capture behavior sits behind one action interface that
returns structured outcomes. The implementation owns permission/error
classification, bounded capture, restoration of patched browser methods, and
target-health recovery. Callers do not need to know CDP method sequencing.

### Dependent command execution module

A dependent batch is one daemon request with ordered step results, one queue
lease, and an explicitly scoped target binding. The module owns target
revalidation and bail behavior. The existing CLI-only batch remains compatible
until the daemon-owned path proves parity.

### Service status projection module

Persisted service state remains the authority. Ordinary status uses a bounded
projection for closed tab history and returns compaction metadata. A diagnostic
or verbose interface exposes retained lifecycle evidence without changing tab
routing truth.

## Slice A | Correct The Incident Authority And Add Evidence Shape

Status: Complete

- Amend the incident note with the seven mediation decisions above.
- Add a privacy-safe validation-note template for reproduction command,
  observed duration, timeout classification, target-health probe, recovery
  action, and final retained-browser topology.
- Record that the original runtime observations are historical narrative until
  replaced by deterministic fixtures or a new redacted live artifact.

Exit criteria:

- The incident note no longer classifies an empty clipboard as an error.
- It does not imply that session reattachment is a proven recovery primitive.
- Its locator regression describes the actual accessible-name mismatch.
- Evidence strength and residual uncertainty are explicit.

Completion evidence:

- The incident summary now records temporal correlation rather than unproved
  target-poisoning causality.
- Historical runtime observations are labeled separately from deterministic
  regression evidence.
- Clipboard timeout guidance requires cancellation-safe pending-command
  cleanup inside the CDP lifecycle.
- Empty clipboard text remains a successful result.
- Locator coverage targets an accessible-name mismatch rather than an ordinary
  portal descendant.
- The note includes a privacy-safe validation artifact template for future
  fixture and live proof.

## Slice B | Cancellation-Safe Clipboard Timeout And Target Recovery

Status: Complete

Deliver vertical red-to-green behaviors in this order:

1. A CDP command with a short deadline returns a typed timeout and a following
   command succeeds without waiting for the default 30-second deadline.
2. A transport-send failure and an externally cancelled command do not retain
   an unreachable pending response entry.
3. `clipboard read` uses the operation-specific deadline and preserves `""` as
   successful text.
4. Permission denial, unavailable backend, unresolved promise, CDP failure,
   and recovery failure have stable structured classifications.
5. After an unresolved clipboard promise, a normal evaluation succeeds on the
   same retained browser through a proved recovery path, or the response names
   replacement-tab recovery as required.

Exit criteria:

- No clipboard read waits for the generic CDP timeout.
- Timeout and cancellation cleanup are test-covered through the command
  interface.
- A post-timeout evaluation outcome is explicit and deterministic.
- Existing evaluation callers retain their default-timeout behavior.

Source completion evidence:

- `CdpClient::send_command_with_timeout` returns typed lifecycle failures and
  uses a drop guard that removes pending registrations after response,
  transport failure, deadline expiry, channel closure, or external future
  cancellation.
- The compatibility `send_command` interface retains its 30-second default and
  existing string errors.
- `clipboard::read_text` sends Chrome's `Runtime.evaluate.timeout` with a
  three-second renderer deadline and a short transport grace period.
- Transport stalls issue `Runtime.terminateExecution` before a normal
  evaluation probe. Protocol responses stating that execution was already
  terminated probe directly so termination is not armed against the next
  evaluation.
- Clipboard success includes `text`, `empty`, and `clipboardOutcome`. Empty
  text is successful.
- Failure diagnostic JSON uses stable codes for permission denial, unresolved
  promise, unavailable backend, CDP failure, and recovery failure, plus
  `same_target_ready` or `replace_tab_required` recovery.

Focused validation:

```bash
cargo test --manifest-path cli/Cargo.toml clipboard -- --nocapture
cargo test --manifest-path cli/Cargo.toml native::cdp::client::tests -- --nocapture
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity
pnpm --config.verify-deps-before-run=false --dir docs build
git diff --check
```

The normal `pnpm validation:select -- --base HEAD` wrapper was blocked before
selection because pnpm 11 rejected already-ignored dependency build scripts.
Running `node scripts/dev/select-validation.js --base HEAD` produced the
recommendations without approving or executing those dependency scripts.

## Slice C | Bounded Clipboard-Write Capture

Status: Complete

- Add an opt-in click action mode that captures `Clipboard.writeText` values
  without reading the system clipboard.
- Bound captured text length and report truncation.
- Restore the original method in a `finally` path on success, action failure,
  timeout, and target replacement.
- State explicitly that `Clipboard.write`, `document.execCommand("copy")`, OS
  clipboard ownership, and browser-native copy UI are not captured unless
  separately implemented and tested.
- Preserve existing output redaction contracts and never include captured text
  in traces or debug logs by default.

Exit criteria:

- Synthetic copy success returns structured captured text.
- Action failure and timeout restore the original method.
- Unsupported copy mechanisms return an explicit no-capture result rather than
  a false success claim.

Source completion evidence:

- `click --capture-clipboard-write` and MCP/HTTP
  `captureClipboardWrite: true` explicitly install a page-scoped wrapper for
  `Clipboard.prototype.writeText` around one click.
- The response reports `supported`, `invoked`, bounded `text`, `truncated`,
  `originalLength`, `restored`, and an unsupported `reason` where applicable.
- Captured text is capped at 4096 characters and is returned only in the
  explicit command response. It is not added to service trace or debug fields.
- The clipboard module owns setup, deadline enforcement, and restoration. Unit
  tests prove restoration after success, action failure, and action timeout.
- The click action uses a five-second capture-specific deadline, shortened to
  leave a one-second cleanup reserve when a smaller worker deadline is present.
- Documentation states that `Clipboard.write`, legacy `execCommand`, and native
  browser UI are outside this capture contract.

Focused validation:

```bash
cargo test --manifest-path cli/Cargo.toml write_capture_ -- --nocapture
cargo test --manifest-path cli/Cargo.toml browser_api_command_maps_named_post_routes -- --nocapture
cargo test --manifest-path cli/Cargo.toml browser_click_command_forwards_options_and_trace_fields -- --nocapture
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity
pnpm --config.verify-deps-before-run=false --dir docs build
git diff --check
```

## Slice D | Page-Command Timing And Dependent Batch Execution

Status: Complete

- Add bounded timing fields for queue wait, browser and target resolution, CDP
  session acquisition or reuse, action execution, and response serialization.
- Expose timings in JSON action output and service traces without raw debug
  logs.
- Add a daemon-owned dependent batch interface that preserves one queue lease
  and target/session binding only across target-stable steps.
- Preserve command order, per-step structured results, JSON behavior, and bail
  semantics.
- Revalidate after any target-changing command and stop on target-health
  failure when bail behavior is enabled.

Exit criteria:

- Timing totals and components are internally consistent and bounded.
- A target-stable menu workflow resolves one target/session binding.
- A target-changing step cannot cause later steps to run against stale identity.
- Existing CLI batch behavior remains compatible or has a documented migration.

Source completion evidence:

- `batch --dependent` parses every step before dispatch and submits one
  `dependent_batch` command through the control-plane queue. Existing batch
  behavior is unchanged without the flag.
- Target-stable steps compare active CDP session identity before and after the
  action. Navigation and tab commands explicitly rebind the following step.
- Nested batches and daemon lifecycle actions are rejected instead of silently
  changing outer worker or connection lifecycle semantics.
- `--bail` stops after the first failed, rejected, or target-invalid step.
- Requested timing output reports `queueWaitMs`, `commandPreparationMs`,
  `actionExecutionMs`, `responseSerializationMs`, `daemonTotalMs`, and
  `totalMs`. Each dependent step also retains its daemon timing object.
- Unit tests cover action classification, nested rejection, ordered results,
  bail behavior, and queue plus daemon timing shape.

Focused validation:

```bash
cargo test --manifest-path cli/Cargo.toml dependent -- --nocapture
cargo test --manifest-path cli/Cargo.toml command_timings -- --nocapture
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity
pnpm --config.verify-deps-before-run=false --dir docs build
git diff --check
```

## Slice E | Accessible Role Locator Repair

Status: Complete

- Replace the `aria-label` or `textContent` approximation with accessible-name
  matching consistent with the snapshot/ref-map semantics already used by the
  native browser implementation.
- Cover `aria-labelledby`, hidden descendant text, dynamically mounted menus,
  exact and partial names, iframe context, and any supported shadow-root path.
- Ensure every success or failure produces one structured response and removes
  temporary locator markers.

Exit criteria:

- The reconstructed incident fixture fails before the repair and passes after
  it.
- A conventional portal-mounted menu item remains covered but is not treated as
  the incident regression by itself.
- Exact accessible-name matching does not fall back to raw `textContent` when
  those values differ.

Completion evidence:

- `getbyrole` now queries Chrome's full accessibility tree and matches the
  browser-computed role and accessible name used by snapshot ref recovery.
- Exact and partial matching share one AX-name predicate. Exact matching does
  not inspect `aria-label` or raw `textContent` separately.
- Main-frame and tracked iframe sessions are searched deterministically.
  Backend-node resolution retains supported shadow-tree behavior.
- The located backend node is installed as a temporary frame-aware ref and is
  removed after either subaction success or failure. No DOM marker attribute is
  created.
- The live ignored E2E fixture uses a dynamically mounted button with no own
  text and a name supplied only by a hidden `aria-labelledby` target. It proves
  exact and partial clicks plus a browser-accessibility shadow-root click.

Focused validation:

```bash
cargo test --manifest-path cli/Cargo.toml accessible_role_name_matching -- --nocapture
cargo test --manifest-path cli/Cargo.toml e2e_getbyrole_uses_browser_accessible_name_from_aria_labelledby -- --ignored --test-threads=1 --nocapture
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm --config.verify-deps-before-run=false --dir docs build
git diff --check
```

## Slice F | Bounded Closed-Tab Status Projection

Status: Complete

- Add a deterministic ordinary-status cap for closed tab lifecycle rows.
- Keep all live, opening, loading, closing, crashed, and routing-referenced tab
  records visible regardless of the closed-history cap.
- Return retained count, omitted count, cap, and ordering metadata.
- Add an explicit diagnostic or verbose retrieval path for retained closed-tab
  lifecycle evidence.
- Prove status compaction does not alter persisted service state, service tab
  handles, browser/session routing, incident attribution, or stale-handle
  classification.

Exit criteria:

- Ordinary status remains bounded under repeated tab churn.
- Diagnostic retrieval preserves the required lifecycle history.
- A stale handle cannot become live because its ordinary status row was
  omitted.

Completion evidence:

- Ordinary status clones the reconciled authority and removes only
  unreferenced `closed` rows beyond a deterministic cap of 50.
- Every non-closed lifecycle remains visible. Closed rows also remain visible
  when referenced by a session, owner, service tab handle, challenge, snapshot,
  or screenshot.
- `closedTabProjection` reports mode, cap, total closed count, retained closed
  count, omitted count, ordering, and diagnostic availability.
- `service status --full-tab-history` and HTTP
  `GET /api/service/status?full-tab-history=true` expose the complete cloned
  authority with no omissions.
- Reconciliation and persistence finish against the complete state before the
  response projection is built. Unit tests prove the source state remains
  unchanged and a stale handle retains its stale reason.
- The status response schema and contract documentation include the projection
  metadata, and generated client contract checks remain green.

Focused validation:

```bash
cargo test --manifest-path cli/Cargo.toml service_status_projection -- --nocapture
cargo test --manifest-path cli/Cargo.toml service_status_full_tab_history -- --nocapture
cargo test --manifest-path cli/Cargo.toml service_status_command_maps_full_tab_history_query -- --nocapture
cargo test --manifest-path cli/Cargo.toml service_status_response_combines_worker_and_service_state -- --nocapture
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm --config.verify-deps-before-run=false test:service-client-contract
pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity
pnpm --config.verify-deps-before-run=false --dir docs build
git diff --check
```

## Closeout Evidence

The final installed runtime used a temporary profile and redacted public test
page. No authenticated site data, copied private URL, cookie, screenshot, or
retained profile content entered the validation artifact.

- An intentionally unresolved `Clipboard.prototype.readText` promise returned
  `unresolved_promise` after the clipboard-specific deadline with recovery
  `same_target_ready`. A following `get title` succeeded on the same target.
- `click --capture-clipboard-write --json` captured the synthetic text
  `captured-live`, reported `invoked: true`, and reported `restored: true`.
- `batch --dependent --json` ran two target-stable reads under one target
  binding and returned per-step daemon timing components.
- The accessibility E2E fixture passed against real Chrome for exact and
  partial `aria-labelledby` names plus the supported shadow-root path.
- Installed CLI status reported `closedTabProjection.mode` as `bounded` for
  ordinary status and `full` for `--full-tab-history`. The installed smoke
  exposed and closed a missing projection on the CLI-local no-launch path
  before final publication.
- `agent-browser install doctor` passed after local runtime publication,
  dashboard restart, and stale daemon-session retirement. The final readback
  reported a ready dashboard, converged runtime state, zero stale runtimes,
  and no install issues.
- Graphiti provider readiness passed, and one compact source-backed closeout
  episode was queued in `agent_browser_main` from this plan and the redacted
  incident note.

Final source and contract validation included:

```bash
scripts/ci/rust-tests.sh
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm --config.verify-deps-before-run=false test:service-client
pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity
pnpm --config.verify-deps-before-run=false --dir docs build
node scripts/dev/select-validation.js --base 0065a938
git diff --check
```

The first unconstrained parallel `cargo test` run exposed two stale assertions
that still expected intentionally pruned `NotStarted` browser and empty session
records. Those assertions now match the operational-record pruning contract.
The same run also exposed Xvfb display contention; the repository CI harness
ran that environment-mutating module serially and passed.

All Slice A through Slice F exit criteria are satisfied. The temporary browser
profile was removed after validation, and the previous installed executable
was retained as a named local backup.

## Documentation And Contract Surfaces

Any new CLI flag, action behavior, environment variable, JSON field, service
request, or response contract must update all applicable user-facing surfaces:

- `cli/src/output.rs`
- `README.md`
- `skills/agent-browser/SKILL.md`
- `docs/src/app/`
- inline source documentation
- service schemas and metadata
- generated `@agent-browser/client` files and generators
- HTTP and MCP parity surfaces

The docs-site MDX must use HTML tables and documentation must not use double
hyphens as prose dashes.

## Validation

Run the validation selector against the complete slice before push:

```bash
pnpm validation:select -- --base <last-known-green-ref>
```

Minimum source gates:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml
pnpm test:service-client
```

Add focused commands for each red-to-green behavior as its test seam becomes
concrete. Real-browser coverage must use a temporary profile and run serially.

## Closeout Criteria

P76 is complete only when:

- every Slice A through Slice F exit criterion is proved by current source,
  focused tests, and required contract or documentation validation;
- the incident note points to the final validation evidence;
- `ROADMAP.md` and `RUNBOOK.md` record the outcome and remaining risk;
- the installed runtime is refreshed and a privacy-safe retained-browser smoke
  proves the clipboard timeout and post-timeout recovery behavior;
- `agent-browser install doctor` passes for the installed candidate;
- Graphiti memory is considered from the curated closeout artifacts under the
  repo policy, without storing private browser data.
