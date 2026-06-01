# Contextual Chat Codex App Server Plan

Date: 2026-05-31
State: COMPLETE
Lane: P12-E
Parent Roadmap: `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`
Depends On: `docs/dev/plans/0013-2026-05-31-selected-workspace-context-plan.md`

## Purpose

Implement the next workspace-inspection slice: make Chat understand the
selected workspace and support a read-only App Intelligence inspection path.

The only Chat/App Intelligence provider exposed in this slice is the Codex app
server. Do not expose OpenAI, AI Gateway, model-provider selection, Codex exec,
generic SDK calls, OpenClaw, AuraCall, or any other provider as a dashboard
choice. The implementation may keep existing lower-level chat plumbing intact
while migrating this workspace-aware path, but the operator-facing provider
surface for contextual Chat must be Codex app server only.

## Source Findings

- Plan 0013 added `SelectedWorkspaceContext` and passed it to
  `ChatPanel`, the Workspace pane, the viewport, and the other right-pane tabs.
- `packages/dashboard/src/components/chat-panel.tsx` currently uses
  `DefaultChatTransport` against `/api/chat` and sends only `session` and
  `model` from the frontend.
- `packages/dashboard/src/store/chat.ts` currently fetches `/api/chat/status`
  and `/api/models`, and `model-selector.tsx` exposes model selection.
- `cli/src/native/stream/chat.rs` currently handles `/api/chat` by sending
  OpenAI-compatible chat-completions requests to `AI_GATEWAY_URL` with
  `AI_GATEWAY_API_KEY`.
- The `app-intelligence-automation` guidance recommends `codex app-server` for
  long-lived integrations needing session state, streamed events, steering,
  approvals, structured outputs, and replayable event logs.
- Local verification on 2026-05-31 showed `codex-cli 0.135.0` supports
  `codex app-server`, stdio transport, Unix and WebSocket transports, and
  protocol schema/type generation. This plan should start with stdio JSONL and
  avoid exposing a remote app-server transport.
- Graphiti discovery for `agent_browser_main` was healthy and reinforced that
  dashboard expansion should consume authoritative service-owned state and keep
  the installed user-scoped runtime isolated from the active repo workspace.

## Non-Goals

- Do not expose any provider selector besides Codex app server for contextual
  Chat.
- Do not implement mutating App Intelligence actions in this slice.
- Do not let Codex output directly execute service actions, browser actions,
  file writes, deploys, or storage/browser cleanup.
- Do not add Network, Storage, Console, Activity, or Extensions backend
  evidence providers beyond the safe selected-context packets already available
  from Plan 0013.
- Do not expose cookies, storage values, headers, screenshots, credentials,
  browser auth artifacts, or raw private page data in the context packet by
  default.
- Do not use WebSocket app-server transport for this first slice.
- Do not commit generated Codex app-server protocol artifacts unless a later
  plan explicitly decides to snapshot a protocol version.
- Do not replace the existing Service selected-record inspector.
- Do not publish a formal release. Local runtime publication is only for
  operator-visible validation.

## Product Contract

Chat becomes a contextual workspace copilot backed by Codex app server.

The user-facing Chat tab should:

- show the selected workspace identity and freshness
- show Codex app server as the active provider
- let the operator include or exclude safe context groups
- send selected-workspace context with Chat turns
- run a read-only App Intelligence inspection
- render structured observations as first-class artifacts
- cite which evidence groups contributed to each observation
- keep unsupported or future providers hidden

The first App Intelligence response should be able to answer:

- what browser or workspace is selected
- whether it is live, retained, blocked, stale, viewable, or controllable
- what page is selected by title and URL
- which service-owned jobs, incidents, runtime facts, and stream facts matter
- what should be inspected next
- what risks or blockers are visible from the supplied evidence

## Provider Contract

Only one provider is exposed:

```ts
type ChatProvider = {
  id: "codex-app-server";
  label: "Codex app server";
  transport: "stdio";
  mode: "read-only-inspection";
};
```

Rules:

- The dashboard must not display model-provider options for this contextual
  Chat lane.
- If the older AI Gateway chat remains available internally or for legacy CLI
  use, it must not appear as a selectable contextual Chat provider.
- The Codex app-server supervisor owns the thread, turn, evidence packet, and
  event log.
- Codex receives bounded selected-workspace evidence and returns structured
  observations.
- The host validates every structured response before rendering it or recording
  it as activity.

## Context Packet Contract

Add a versioned selected-workspace Chat packet derived from
`SelectedWorkspaceContext`.

Suggested shape:

```ts
export type SelectedWorkspaceChatPacket = {
  version: "selected-workspace-chat.v1";
  createdAt: string;
  provider: "codex-app-server";
  selection: {
    workspaceId: string | null;
    browserId: string | null;
    sessionId: string | null;
    tabId: string | null;
    profileId: string | null;
    jobId: string | null;
  };
  workspace: {
    id: string | null;
    label: string;
    source: string;
    state: string;
    health: string | null;
    live: boolean;
    retained: boolean;
    viewable: boolean;
    controllable: boolean;
    missingReason: string | null;
  };
  runtime: {
    pid: number | null;
    running: boolean | null;
    rssBytes: number | null;
    cpuSeconds: number | null;
    cdpPort: number | null;
    streamPort: number | null;
    lastFrameAt: number | null;
  };
  page: {
    title: string | null;
    url: string | null;
    targetId: string | null;
    lifecycle: string | null;
    active: boolean | null;
  };
  stream: {
    provider: string | null;
    routeSummary: string | null;
    controlInput: string | null;
    embeddable: boolean;
    controllable: boolean;
  };
  ownership: {
    serviceName: string | null;
    agentName: string | null;
    taskName: string | null;
  };
  evidence: Array<{
    id: string;
    source: "workspace" | "activity" | "console" | "network" | "storage" | "extensions";
    summary: string;
    facts: Record<string, unknown>;
    freshness: string;
    included: boolean;
  }>;
  redaction: {
    secretsOmitted: true;
    screenshotsIncluded: false;
    rawStorageIncluded: false;
    rawHeadersIncluded: false;
  };
};
```

The initial packet should include the Workspace evidence from Plan 0013. Other
evidence sources may contribute placeholder summaries only when they are
clearly marked as fallback or unavailable.

## Observation Contract

Codex app server returns validated structured observations.

Suggested shape:

```ts
export type CodexWorkspaceObservation = {
  version: "codex-workspace-observation.v1";
  provider: "codex-app-server";
  runId: string;
  threadId: string | null;
  createdAt: string;
  workspaceId: string | null;
  summary: string;
  detectedState: string;
  blockers: Array<{
    severity: "info" | "warning" | "blocked";
    summary: string;
    evidenceIds: string[];
  }>;
  risks: Array<{
    summary: string;
    evidenceIds: string[];
  }>;
  suggestedNextInspections: Array<{
    label: string;
    reason: string;
    evidenceIds: string[];
  }>;
  unsupportedActions: Array<{
    label: string;
    reason: string;
  }>;
  confidence: "low" | "medium" | "high";
};
```

Rules:

- Observations are read-only.
- Suggested next inspections are not executable actions.
- Any future mutating action belongs to Slice F and must map to service
  contracts with explicit operator confirmation.

## Implementation Slices

### Slice E1 | Context Packet Builder

Goal: build a Chat-safe packet from selected workspace context.

Tasks:

- Add a dashboard library module, for example
  `packages/dashboard/src/lib/selected-workspace-chat-packet.ts`.
- Reuse `SelectedWorkspaceContext`.
- Add redaction rules and evidence IDs.
- Keep raw IDs inspectable only where already visible in Workspace evidence.
- Mark non-implemented evidence providers as unavailable instead of inventing
  data.

Exit criteria:

- Fixture tests prove live, retained, stale, and missing selections produce
  distinct packets.
- Packet output contains no raw cookies, storage values, headers, screenshots,
  credentials, or auth artifacts.

### Slice E2 | Codex App Server Adapter

Goal: add a host-owned read-only Codex app-server adapter.

Tasks:

- Add a service-side adapter module under `cli/src/native/stream/` or a nearby
  service-control namespace.
- Start with stdio `codex app-server` or proxy to the local daemon only when
  explicitly configured.
- Generate protocol schemas/types into a temporary build location during
  implementation if needed, but do not commit generated protocol output.
- Persist a minimal run ledger for each inspection:
  - run id
  - workspace id
  - context packet hash
  - Codex thread id when available
  - request timestamp
  - provider id `codex-app-server`
  - raw app-server event log path or bounded event summary
  - validated observation result
- Use structured output schema for observation turns.
- Enforce read-only turn policy with no file edits, no service mutations, no
  browser mutations, and no destructive commands.

Exit criteria:

- Adapter can run a bounded read-only inspection from a fixture packet.
- Failure modes are structured: unavailable app server, timeout, invalid
  observation, policy violation, and empty evidence.

### Slice E3 | HTTP And Dashboard Contract

Goal: expose the adapter to the dashboard without adding a provider picker.

Tasks:

- Add a focused endpoint such as
  `POST /api/app-intelligence/inspect-workspace`.
- Request body accepts the selected-workspace packet and optional include flags.
- Response body returns the validated observation contract.
- Add a status endpoint only if needed for readiness, and report only
  `codex-app-server`.
- Do not reuse `/api/models` or model-selector UI for this lane.
- Do not expose AI Gateway provider labels in contextual Chat.

Exit criteria:

- Dashboard can request a read-only inspection for the selected workspace.
- Endpoint rejects non-Codex providers.
- Endpoint rejects mutating action requests.

### Slice E4 | Chat UI Integration

Goal: make the Chat tab visibly contextual.

Tasks:

- Replace generic provider/model affordances in contextual Chat with a compact
  `Codex app server` provider badge.
- Add context include controls for:
  - Workspace evidence: enabled
  - Page summary: enabled when available
  - Activity: disabled or fallback until Slice B
  - Console: disabled or fallback until Slice B
  - Network: disabled or fallback until Slice C
  - Storage: disabled or fallback until Slice C
  - Extensions: disabled or fallback until Slice D
- Add `Inspect with Codex` or equivalent read-only command.
- Render `CodexWorkspaceObservation` as a structured Chat artifact with
  summary, blockers, risks, next inspections, confidence, and evidence refs.
- Keep existing free-form chat history if practical, but workspace inspection
  results must be structured and tied to the selected context packet.

Exit criteria:

- Chat clearly shows it is scoped to the selected workspace.
- Chat does not look like a generic global assistant.
- The only visible provider is Codex app server.
- Operator can trigger a read-only inspection and see structured observations.

### Slice E5 | Tests, Publish, And External Smoke

Goal: prove the lane works in source and in the installed dashboard.

Tasks:

- Add packet builder tests.
- Add adapter contract tests with fixture responses.
- Add HTTP rejection tests for non-Codex providers or mutating requests.
- Add dashboard structure tests proving the provider picker is hidden and the
  Codex app-server badge is visible.
- Run source validation.
- Publish to the local runtime.
- Smoke the external dashboard against the stable selected workspace.

Exit criteria:

- Source tests pass.
- Live dashboard chunks include a Codex app-server contextual Chat marker.
- External DOM smoke proves selected workspace context is visible in Chat and
  the provider surface is Codex app server only.

## File-Level Plan

Expected new files:

- `docs/dev/plans/0014-2026-05-31-contextual-chat-codex-app-server-plan.md`
- `packages/dashboard/src/lib/selected-workspace-chat-packet.ts`
- `scripts/test-dashboard-selected-workspace-chat-packet.js`
- `scripts/test-dashboard-contextual-chat.js`
- service-side Codex app-server adapter and tests, exact path to be chosen
  after implementation inspection

Expected edited files:

- `packages/dashboard/src/components/chat-panel.tsx`
- `packages/dashboard/src/store/chat.ts`
- `packages/dashboard/src/components/model-selector.tsx` if the selector needs
  to be hidden or bypassed for contextual Chat
- `packages/dashboard/src/lib/dashboard-api.ts`
- `cli/src/native/stream/chat.rs` or a new adjacent HTTP handler module
- `cli/src/native/stream/http.rs`
- `cli/src/native/stream/dashboard.rs`
- `package.json`
- `scripts/dev/select-validation.js`
- docs only if visible workflow language is added beyond this plan

## Data And Action Rules

- Prefer `SelectedWorkspaceContext` and service-owned state over daemon-only
  session state.
- Every context packet must be redacted before it reaches Codex.
- Do not include screenshots by default.
- Do not include storage values, cookies, auth headers, bearer tokens,
  passwords, local private page data, or raw request/response bodies.
- Keep page URLs and titles because they are already visible operational
  evidence.
- Use source evidence IDs so observations can cite their inputs.
- The adapter must treat Codex output as advisory until it passes schema
  validation.
- No mutating action is executable from an observation in this slice.

## Validation Matrix

Required source validation:

- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-contextual-chat`
- App Intelligence/Codex adapter fixture tests
- HTTP contract tests for provider and read-only policy
- `pnpm build:dashboard`
- `git diff --check`

Likely existing checks to keep green when touched:

- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-navigator`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- focused Rust tests for new stream or app-intelligence handler code

Required local app-server verification:

```bash
codex --version
codex app-server --help
codex app-server generate-json-schema --out /tmp/agent-browser-codex-app-server-schema
codex app-server generate-ts --out /tmp/agent-browser-codex-app-server-ts
```

Required live validation before operator closeout:

```bash
pnpm publish:local-dashboard -- --dashboard-url https://agent-browser.ecochran.dyndns.org/ --expect-marker codex-app-server --json
```

Then run a live DOM smoke against:

```text
https://agent-browser.ecochran.dyndns.org/?workspace=browser%3Asession%3Adefault&session=default&view=workspace%3Acontrol&browser=session%3Adefault&profile=default
```

The smoke should prove:

- Chat shows selected workspace context.
- Chat provider surface shows Codex app server only.
- No model/provider picker exposes AI Gateway, OpenAI, or other providers.
- Read-only inspection can be requested.
- Structured observation artifact renders with evidence references.
- The workspace viewport still renders the live CDP canvas.

## Risks And Mitigations

- Risk: Chat path accidentally exposes the existing AI Gateway/model selector.
  Mitigation: add dashboard static tests and live DOM smoke for provider
  exclusivity.
- Risk: Codex app-server protocol drift breaks the adapter.
  Mitigation: generate schemas/types from the installed `codex` binary during
  implementation and keep adapter contract tests fixture-backed.
- Risk: context packets leak sensitive browser data.
  Mitigation: packet builder owns redaction and has explicit negative tests for
  cookies, storage values, headers, screenshots, and auth artifacts.
- Risk: observations blur into executable actions.
  Mitigation: schema separates suggested next inspections from actions, and
  Slice F owns proposed action execution.
- Risk: app-server turns are slow or unavailable.
  Mitigation: make unavailable, timeout, and invalid-output states explicit in
  the UI and keep Chat usable as a selected-context display.
- Risk: dashboard runtime publication restarts the service and makes the
  stable `default` browser retained.
  Mitigation: relaunch `default` before final external smoke if needed, then
  prove the CDP canvas is live.

## Completion Criteria

Slice E is complete when:

- `SelectedWorkspaceChatPacket` exists and is tested.
- Contextual Chat receives selected-workspace packets.
- The only user-visible provider for contextual Chat is Codex app server.
- A read-only Codex app-server inspection path exists and validates structured
  observations.
- Chat renders structured observation artifacts with evidence references.
- Non-Codex providers and mutating requests are rejected or hidden.
- Source validation passes.
- The installed dashboard service has been updated through
  `pnpm publish:local-dashboard`.
- External smoke proves selected workspace Chat context, Codex-only provider
  visibility, structured observations, and no regression to the live viewport.

## Execution Notes

Completed on 2026-05-31.

Implemented:

- `SelectedWorkspaceChatPacket` with Codex-only provider id, selected-workspace
  evidence, unavailable placeholders for later inspector evidence groups, and
  redaction tests.
- Read-only App Intelligence HTTP contract at
  `POST /api/app-intelligence/inspect-workspace` plus
  `GET /api/app-intelligence/status`.
- Service-side `codex-app-server` inspection response validation, provider
  rejection, mutating-request rejection, context packet hashing, and a minimal
  run ledger under the user runtime.
- Chat pane Codex-only contextual UI with provider badge, selected workspace
  packet submission, structured observation rendering, and no model selector.
- Dashboard validation for the contextual Chat surface and local runtime smoke
  coverage for CDP canvas workspaces.

Validation evidence:

- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-contextual-chat`
- `cargo test --manifest-path cli/Cargo.toml native::stream::app_intelligence -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm build:dashboard`
- `pnpm publish:local-dashboard -- --expect-marker data-codex-app-server-contextual-chat --expect-marker /api/app-intelligence/inspect-workspace --skip-browser --json`
- `node scripts/smoke-local-dashboard-runtime.js --dashboard-url https://agent-browser.ecochran.dyndns.org/ --workspace-session default --expect-marker data-codex-app-server-contextual-chat --expect-marker /api/app-intelligence/inspect-workspace --json`

The final external smoke authenticated, loaded the `default` workspace, proved
the CDP screencast canvas was present, switched to Chat, verified Codex app
server was the only provider surface, verified no model/provider selector text
was present, requested `Inspect viewport readiness`, and saw a structured
Codex inspection observation.

## Recommended Implementation Order

1. Add packet builder and redaction tests.
2. Add Codex app-server adapter contract and fixture tests.
3. Add the read-only HTTP endpoint and provider rejection tests.
4. Replace contextual Chat provider/model UI with Codex app-server-only
   controls.
5. Render structured observation artifacts.
6. Run source validation.
7. Publish and run live external smoke.
