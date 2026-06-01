# Codex App Server Supervisor Plan

Date: 2026-05-31
State: COMPLETE
Lane: P12-E2
Parent Roadmap: `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`
Depends On: `docs/dev/plans/0014-2026-05-31-contextual-chat-codex-app-server-plan.md`
Prepares: P12-F proposed actions and audit trail

## Purpose

Harden the contextual Chat App Intelligence bridge by replacing the current
host-generated read-only observation with a supervised `codex app-server`
inspection turn.

Plan 0014 created the selected-workspace packet, Codex-only UI surface, HTTP
contract, run ledger, and structured observation renderer. That proved the
operator workflow and provider boundary. The next slice should make the
provider boundary honest at the adapter level: the dashboard should still expose
only Codex app server, but the service-side adapter should launch or reuse a
supervised Codex app-server process, send the redacted selected-workspace packet,
capture app-server events, validate the structured observation, and persist a
replayable ledger.

This is a prerequisite to proposed actions. Do not add action execution yet.

## Current Baseline

- `packages/dashboard/src/lib/selected-workspace-chat-packet.ts` builds
  `selected-workspace-chat.v1` packets with provider `codex-app-server` and
  redaction flags.
- `packages/dashboard/src/components/chat-panel.tsx` renders the Codex app
  server read-only provider surface and submits selected-workspace packets to
  `POST /api/app-intelligence/inspect-workspace`.
- `cli/src/native/stream/app_intelligence.rs` validates the packet, rejects
  non-Codex providers and mutating fields, computes a context packet hash,
  writes a minimal ledger, and returns a deterministic host-generated
  `codex-workspace-observation.v1`.
- The installed dashboard runtime was published and externally smoked against
  `https://agent-browser.ecochran.dyndns.org/`, including the Chat tab and CDP
  canvas workspace path.
- Local verification showed the installed `codex` binary supports
  `codex app-server`, schema generation, TypeScript generation, stdio, Unix
  sockets, and WebSocket transport. This plan must continue with stdio only.

## Non-Goals

- Do not expose OpenAI, AI Gateway, model selection, Codex exec, OpenClaw,
  AuraCall, or any other provider in contextual Chat.
- Do not add mutating proposed actions or service request execution.
- Do not allow Codex output to execute browser commands, service requests,
  shell commands, file edits, cleanup, deploys, storage clears, or profile
  pruning.
- Do not expose cookies, storage values, headers, screenshots, credentials,
  private page content, or raw auth artifacts to Codex.
- Do not introduce a remote WebSocket app-server control surface.
- Do not commit generated Codex protocol artifacts unless a later plan
  explicitly snapshots a protocol version.
- Do not make the dashboard depend on app-server availability to show the
  selected workspace facts.

## Product Contract

The Chat tab remains a selected-workspace copilot with one visible provider:

```ts
type ContextualChatProvider = {
  id: "codex-app-server";
  label: "Codex app server";
  transport: "stdio-jsonl";
  mode: "read-only-inspection";
};
```

When the operator runs an inspection:

- the current selected-workspace packet is sent to the service-side supervisor
- the supervisor starts or resumes a Codex app-server thread for this inspection
- Codex receives only redacted evidence and a strict structured-output schema
- the host validates the observation before returning it to the dashboard
- the dashboard renders only validated observations
- failures are structured and operator-readable
- the run ledger can be used to replay what evidence was supplied and what
  Codex returned

## Supervisor Contract

Add a host-owned supervisor around `codex app-server`.

Minimum run state:

```json
{
  "runId": "codex-inspect-...",
  "provider": "codex-app-server",
  "mode": "read-only-inspection",
  "createdAt": "2026-05-31T00:00:00Z",
  "workspaceId": "browser:session:default",
  "contextPacketHash": "sha256...",
  "codex": {
    "threadId": "optional",
    "turnId": "optional",
    "transport": "stdio-jsonl",
    "cliVersion": "codex-cli 0.135.0"
  },
  "policy": {
    "readOnly": true,
    "allowedActions": ["inspect_context", "summarize", "recommend_next_inspection"],
    "forbiddenActions": ["browser_mutation", "service_mutation", "file_write", "shell_command"]
  },
  "artifacts": {
    "requestPath": "request.json",
    "eventsPath": "codex-events.jsonl",
    "normalizedEventsPath": "events.jsonl",
    "observationPath": "observation.json"
  }
}
```

Run root:

```text
~/.agent-browser/app-intelligence/runs/<run-id>/
  run.json
  request.json
  codex-events.jsonl
  events.jsonl
  observation.json
```

The existing `AGENT_BROWSER_APP_INTELLIGENCE_RUN_ROOT` override should continue
to work for tests.

## Observation Schema

Continue using `codex-workspace-observation.v1` from Plan 0014. Add host-side
schema validation before returning the response.

Required validation:

- `version` is `codex-workspace-observation.v1`
- `provider` is `codex-app-server`
- `runId` matches the host run id
- `workspaceId` matches the supplied packet when present
- `blockers`, `risks`, `suggestedNextInspections`, and `unsupportedActions`
  are arrays
- every evidence reference points to an evidence id present in the packet or to
  an allowed unavailable placeholder
- `unsupportedActions` includes a read-only policy reminder when Codex suggests
  anything action-like
- no observation field contains forbidden sensitive markers

If Codex returns invalid JSON, invalid schema, unsupported action text, or
sensitive markers, return a structured failure and persist the rejected output
under the run directory for debugging.

## App Server Protocol Tasks

Use generated protocol schemas/types only as implementation guidance.

Required local verification:

```bash
codex --version
codex app-server --help
codex app-server generate-json-schema --out /tmp/agent-browser-codex-app-server-schema
codex app-server generate-ts --out /tmp/agent-browser-codex-app-server-ts
```

Implementation should start with stdio JSONL:

- spawn `codex app-server` as a child process
- write JSON-RPC requests to stdin
- read JSON-RPC responses and event messages from stdout
- capture stderr separately in the run ledger
- set bounded timeouts for startup, thread start, turn start, and turn
  completion
- terminate the child process after a bounded inspection unless a later slice
  deliberately adds pooling

Start with a simple per-inspection child process. Process reuse is a later
optimization and should not block correctness.

## Prompt And Policy Packet

The host should construct a deterministic prompt package that includes:

- the redacted selected-workspace packet
- the observation schema
- explicit read-only policy
- the current task prompt from the operator
- evidence source availability and unavailable placeholders
- instruction to return only the structured observation payload

The prompt must not ask Codex to use tools, run commands, edit files, mutate
browser state, or inspect private page data outside the provided packet.

## HTTP Contract

Keep the Plan 0014 endpoint:

```text
POST /api/app-intelligence/inspect-workspace
GET  /api/app-intelligence/status
```

Enhance responses:

- include `data.ledger.threadId` when Codex returns one
- include `data.ledger.turnId` when available
- include `data.ledger.eventLogPath`
- include `data.ledger.normalizedEventLogPath`
- include `data.ledger.observationPath`
- include `data.failure` for unavailable, timeout, invalid observation, policy
  violation, and empty evidence states

Do not change the frontend provider selector contract.

## Failure Modes

Failures should be explicit, structured, and testable:

- `codex_unavailable`: `codex app-server` cannot start or report readiness
- `protocol_error`: malformed JSON-RPC response or unexpected protocol shape
- `timeout`: startup, thread, turn, or completion timeout
- `invalid_observation`: Codex output failed schema validation
- `policy_violation`: output proposed mutation or included forbidden data
- `empty_evidence`: selected-workspace packet has no included evidence
- `ledger_write_failed`: inspection ran but artifact persistence failed

The dashboard should render these as read-only inspection failures, not generic
chat errors.

## Implementation Slices

### Slice 1 | Extract Adapter Boundary

Goal: separate the deterministic Plan 0014 observation builder from the HTTP
handler.

Tasks:

- Split request validation, packet validation, observation validation, ledger
  writing, and provider readiness into focused helpers.
- Keep the current deterministic observation path as an internal fallback only
  for tests or explicit development mode.
- Add unit tests for each helper.

Exit criteria:

- HTTP handler becomes thin route glue.
- Provider and mutation rejection tests still pass.

### Slice 2 | Run Ledger Directory

Goal: persist replayable inspection artifacts.

Tasks:

- Change the ledger from a single JSON file to a run directory.
- Write `run.json`, `request.json`, `events.jsonl`, and `observation.json`.
- Include workspace id, packet hash, provider id, CLI version, timestamps, and
  status transitions.
- Use atomic writes where practical.

Exit criteria:

- Tests prove successful and failed inspections leave a readable run directory.
- Redacted request artifacts contain no forbidden markers.

### Slice 3 | Stdio App Server Client

Goal: add a minimal supervised Codex app-server client.

Tasks:

- Start `codex app-server` over stdio.
- Implement JSON-RPC request ids and response matching.
- Support thread start and one bounded turn.
- Capture stdout event stream and stderr into run artifacts.
- Add timeout and child-process cleanup.

Exit criteria:

- Fixture or mocked process tests cover success, malformed response, timeout,
  and process exit.
- A local smoke can start the installed Codex app server without committing
  generated protocol files.

### Slice 4 | Structured Read-Only Turn

Goal: request a real structured observation from Codex.

Tasks:

- Build the prompt and schema package from the selected-workspace packet.
- Pass explicit read-only policy and output schema.
- Validate the returned observation.
- Fall back to structured failure on invalid or missing output.

Exit criteria:

- A fixture selected workspace produces a Codex-authored validated
  `codex-workspace-observation.v1`.
- Invalid or action-like responses are rejected.

### Slice 5 | Dashboard Failure Rendering

Goal: make app-server failures useful to the operator.

Tasks:

- Render app-server unavailable, timeout, invalid observation, policy
  violation, and empty evidence failures as structured Chat artifacts.
- Keep the selected-workspace packet summary visible even when Codex is
  unavailable.
- Keep provider display Codex-only.

Exit criteria:

- Dashboard tests prove failure artifacts render and model/provider selection
  stays hidden.

### Slice 6 | Publish And External Smoke

Goal: prove the real app-server path is visible in the installed runtime.

Tasks:

- Build and publish the local dashboard runtime.
- Run external smoke against the stable `default` CDP workspace.
- Trigger a Chat inspection and verify a structured observation includes
  app-server ledger metadata, not only host-generated placeholder text.

Exit criteria:

- Live chunks include the supervisor marker.
- External DOM smoke proves Chat provider is Codex app server only.
- External DOM smoke proves a structured observation renders.
- Run directory contains replayable app-server artifacts.

## File-Level Plan

Expected new or heavily edited files:

- `cli/src/native/stream/app_intelligence.rs`
- `cli/src/native/stream/app_intelligence_supervisor.rs`
- `cli/src/native/stream/app_intelligence_schema.rs`
- `packages/dashboard/src/components/chat-panel.tsx`
- `packages/dashboard/src/lib/selected-workspace-chat-packet.ts`
- `scripts/test-dashboard-contextual-chat.js`
- `scripts/smoke-local-dashboard-runtime.js`
- `package.json`
- `scripts/dev/select-validation.js`

Docs may need updates only if the user-facing workflow changes beyond the
existing Plan 0014 Chat surface.

## Validation Matrix

Required:

```bash
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-contextual-chat
cargo test --manifest-path cli/Cargo.toml native::stream::app_intelligence -- --nocapture
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm build:dashboard
git diff --check
```

Add focused Rust tests for any new supervisor module.

Required app-server verification:

```bash
codex --version
codex app-server --help
codex app-server generate-json-schema --out /tmp/agent-browser-codex-app-server-schema
codex app-server generate-ts --out /tmp/agent-browser-codex-app-server-ts
```

Required local runtime publish:

```bash
pnpm publish:local-dashboard -- --expect-marker data-codex-app-server-contextual-chat --expect-marker /api/app-intelligence/inspect-workspace --json
```

Required external smoke:

```bash
node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session default \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker /api/app-intelligence/inspect-workspace \
  --json
```

The final smoke should also assert that the rendered observation exposes app
server ledger metadata, including an event log path and thread or turn metadata
when the protocol provides it.

## Risks And Mitigations

- Risk: Codex app-server protocol drift.
  Mitigation: generate schemas/types from the installed `codex` binary during
  implementation and isolate protocol assumptions in one module.
- Risk: app-server turns are slow.
  Mitigation: enforce startup and turn timeouts and render structured timeout
  artifacts.
- Risk: Codex output proposes mutation despite read-only instructions.
  Mitigation: validate output, classify action-like text as policy violation,
  and do not render it as an accepted observation.
- Risk: context packet leakage.
  Mitigation: reuse packet redaction tests and add ledger artifact negative
  tests for sensitive markers.
- Risk: child process leaks.
  Mitigation: always terminate the per-inspection child process on completion,
  timeout, or route cancellation.
- Risk: live smoke closes the `default` workspace.
  Mitigation: launch `default` with `--leave-open` before external smoke when
  needed and keep the smoke aware of CDP canvas workspaces.

## Completion Criteria

Plan 0015 is complete when:

- Codex app-server stdio supervision is implemented behind the existing
  contextual Chat endpoint.
- Successful inspections produce Codex-authored, host-validated structured
  observations.
- Failed inspections produce structured failure artifacts.
- Run directories contain request, run state, event log, and observation
  artifacts.
- Non-Codex providers and mutating requests remain rejected.
- Contextual Chat still exposes only Codex app server.
- Source validation passes.
- Installed local runtime is republished.
- External smoke proves the selected workspace, CDP canvas, Codex-only Chat
  provider, structured observation rendering, and replayable app-server ledger.

## Recommended Next Step

Start with Slice 1 and Slice 2 together. That keeps the current operator-visible
behavior stable while creating the adapter boundary and replayable ledger shape
needed for the real stdio client.

## Completion Evidence

Completed on 2026-05-31.

- Implemented supervised per-inspection `codex app-server --listen stdio://`
  execution behind `POST /api/app-intelligence/inspect-workspace`.
- Added replayable run directories under
  `~/.agent-browser/app-intelligence/runs/<run-id>/` with `run.json`,
  `request.json`, `codex-events.jsonl`, `events.jsonl`, and
  `observation.json`.
- Validated Codex-authored `codex-workspace-observation.v1` responses before
  returning them to the dashboard, with deterministic mode kept for tests and
  explicit development use only.
- Rendered structured app-server failures and ledger metadata in the Chat tab
  while keeping contextual Chat Codex-only.
- Republished the user-scoped local dashboard runtime and restarted
  `agent-browser-dashboard.service`.

Validation:

```bash
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-contextual-chat
cargo test --manifest-path cli/Cargo.toml native::stream::app_intelligence -- --nocapture
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm build:dashboard
git diff --check
pnpm publish:local-dashboard -- --expect-marker data-codex-app-server-contextual-chat --expect-marker /api/app-intelligence/inspect-workspace --json
node scripts/smoke-local-dashboard-runtime.js --dashboard-url https://agent-browser.ecochran.dyndns.org/ --workspace-session plan0015-live-smoke-5 --expect-marker data-codex-app-server-contextual-chat --expect-marker /api/app-intelligence/inspect-workspace --json
```

External smoke proved:

- hosted chunks contain the Codex contextual Chat marker and inspection API
- selected workspace route renders a CDP screencast canvas
- Chat exposes only Codex app server and no model/provider selector
- a live Codex-authored structured observation renders
- event-log and thread/turn ledger metadata render in the Chat pane
