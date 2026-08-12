# Plan 0110-1 | P110 PoC 1 Display-Bound Frame Capture

Date: 2026-08-12

State: PLANNED

Authority: SOURCE-ONLY | NO LIVE CAPTURE

Lane: P110 PoC 1

Parent: `docs/dev/plans/0110-2026-08-12-desktop-perception-interaction-foundation-plan.md`

## Objective

Capture one fresh PNG frame from an exactly bound service-owned RDP workspace
and return a typed `DesktopContext`, `FrameReceipt`, and bounded ephemeral
image payload through one canonical queued service action. Expose that action
coherently through CLI, HTTP, MCP, generated client, contract metadata, help,
README, repo skill, and docs.

PoC 1 proves observation only. It adds no locator, image recognition, pointer,
keyboard, recipe engine, external challenge handling, or live acceptance.

## Reconciled Existing Architecture

- `BrowserProcess` owns browser, profile, display name, display allocation,
  view streams, sessions, process health, and tab handles.
- `DisplayAllocation` owns display name and isolation, browser and session
  owners, profile, host, state, route IDs, and readiness.
- `ViewStream` and `RemoteViewRoute` bind stream, route, display allocation,
  Guacamole connection, viewer and controller leases, attachability, and
  readiness.
- `derive_stream_attachability` is the existing identity and readiness join.
- remote-view visual proof uses bounded X11 window metadata, not pixels.
- current root-display pixel capture exists only in controlled smoke tooling
  through bounded ImageMagick `import` commands.
- CDP page screenshot remains a distinct page-only provider.

The production seam is a new provider-neutral desktop-capture module. It must
not be embedded inside remote-view open or conflated with
`ViewStreamProvider`.

## Frozen Public Contract

### Canonical Action

`desktop_capture` is the single service action. HTTP uses
`POST /api/service/request`. Generic MCP `service_request`, the dedicated MCP
tool, CLI, and generated client all lower into that same queued action.

The request names one service-owned browser by `browserId`. An optional
`sessionName` may narrow routing. It accepts bounded capture parameters only:

- `format`: fixed to `png` in PoC 1;
- `maxBytes`: positive integer with a conservative default and hard maximum.

The request never accepts `displayName`, route URL, Guacamole URL, provider
URL, Xauthority path, output path, crop coordinates, or input options.

CLI spelling:

```text
agent-browser desktop capture --browser-id <id> [--max-bytes <bytes>]
```

### DesktopContext

The response context contains:

- `contextId` and `schemaVersion`;
- `browserId`, `sessionName`, and optional `profileId`;
- `displayAllocationId`, `streamId`, and `routeId`;
- capture provider and view-stream provider;
- display isolation;
- coordinate space fixed to `desktop_physical_pixels` for PoC 1;
- width, height, scale factor, and `geometryEpoch`;
- `resolvedAt` and readiness evidence.

Raw display names and provider URLs remain internal implementation details and
are not returned in the public context.

### FrameReceipt

The response receipt contains:

- `frameId`, `schemaVersion`, and `contextId`;
- capture provider and provider version;
- monotonically unique sequence within the request process;
- `capturedAt`, width, height, scale factor, and geometry epoch;
- MIME type, byte length, and SHA-256 content hash;
- freshness posture;
- `retention: ephemeral` and `persisted: false`.

The image payload is a bounded response-only base64 value. It is not written to
service state, events, incidents, job summaries, command logs, or disk.

### Typed Failures

At minimum, preserve stable codes for:

- `desktop_workspace_not_found`;
- `desktop_workspace_ambiguous`;
- `desktop_display_not_ready`;
- `desktop_route_not_ready`;
- `desktop_identity_mismatch`;
- `desktop_geometry_unavailable`;
- `desktop_geometry_changed`;
- `desktop_capture_provider_unavailable`;
- `desktop_frame_too_large`;
- `desktop_capture_failed`.

Failures contain IDs and non-sensitive readiness evidence, never raw frame
bytes, raw provider URLs, or credential-bearing output.

## Resolution And Capture Algorithm

1. Normalize and attribute the request before queueing.
2. Resolve exactly one nonterminal `BrowserProcess` by `browserId` and optional
   session narrowing.
3. Resolve exactly one service-owned stream and route whose browser, session,
   display allocation, and route IDs agree.
4. Resolve the matching nonterminal `DisplayAllocation` and require current
   attached-ready identity and operator-visible proof.
5. Confirm the exact internally resolved display is within existing
   route-display authorization. Do not grant access.
6. Read geometry from the selected X display with a bounded provider command.
7. Capture one PNG root frame with a bounded subprocess and in-memory output.
8. Decode dimensions, enforce byte cap, hash bytes, and build context and
   receipt.
9. Re-resolve identity and geometry. Reject the frame if route, display,
   browser ownership, or geometry changed during capture.
10. Return the bounded image only in the immediate result and persist only the
    ordinary redacted job outcome.

## Provider Interface

Create a small internal frame-source boundary with a request, provider result,
and typed error. The first implementation is `X11RootFrameProvider` using
bounded process execution against the exact resolved display. Provider-neutral
types own context and receipt construction. ImageMagick availability is a
capability decision, not evidence that a workspace is ready.

The provider must bound timeout, stdout size, stderr retention, image
dimensions, and total bytes. Only safe static command arguments are used. No
shell is invoked.

## Execution Slices

### Slice A | Contract Fixtures And Provider Boundary

- add red provider-free tests for identity resolution, ambiguity, route and
  display mismatch, readiness, geometry drift, payload caps, and redaction;
- define `DesktopContext`, `FrameReceipt`, typed capture errors, and internal
  provider trait;
- add a fake provider returning pinned PNG bytes and geometry;
- record the cross-ingress action and field ledger.

Commit: `test: freeze display-bound frame capture contract`

### Slice B | Native Resolution And X11 Provider

- implement the service-state resolution function;
- reuse current attachability and route-display authorization decisions;
- implement bounded X11 geometry and root-frame capture without shell use;
- re-resolve context after capture and reject drift;
- add pure and subprocess-fixture tests without touching a real display.

Commit: `feat: capture service-bound desktop frames`

### Slice C | Service Request And Ingress Parity

- register `desktop_capture` in canonical service request actions and schema;
- classify it as no-auto-launch observation while retaining accountable
  attribution because pixels may be sensitive;
- dispatch it without BrowserManager or CDP;
- add CLI `desktop capture`, dedicated thin MCP tool, generic MCP and HTTP
  parity, contract metadata, response schema, and generated client helpers;
- ensure all adapters preserve one response envelope and typed failures.

Commit: `feat: expose desktop frame capture across ingresses`

### Slice D | Documentation And Capability Discovery

- update CLI help, README, repo skill, service-mode docs, and inline docs;
- advertise global contract support separately from workspace readiness;
- add or extend a no-launch workspace capability read only if callers cannot
  otherwise discover provider readiness truthfully;
- update ROADMAP and RUNBOOK with source status.

Commit: `docs: document desktop frame capture`

### Slice E | Audit, Remediation, And Source Acceptance

- freeze the full diff and validation evidence;
- run one fresh independent audit across identity, privacy, ingress parity,
  no-launch behavior, docs, and tests;
- primary agent adjudicates findings and performs at most one bounded
  remediation packet;
- run closed-world verification for accepted findings and touched regressions;
- record source acceptance or the exact remaining blocker.

Commit: `test: close desktop frame capture proof`

## Delegation Receipt | Reconnaissance

| Handle | Task | Status | Evidence | Primary reconciliation |
| --- | --- | --- | --- | --- |
| `p110_capture_arch` | Capture and route authority seam | complete | service model, route binding, attachability, X11 proof, controlled ImageMagick capture | accepted provider-neutral module outside remote-view open; capture binds to exact service state and never grants display access |
| `p110_ingress_parity` | CLI, HTTP, MCP, client, schema, and docs parity | complete | `probe` and service-request analogues, parity scripts, generation and docs surfaces | accepted `desktop_capture` as one canonical queued action; rejected CDP tab-handle coupling |
| `p110_fixture_tests` | Provider-free fixture and test strategy | complete | pinned PNG and fake-provider matrix, no-launch route fixture, validation and live-boundary inventory | accepted injected clock and sequence, two distinguishable committed frames, no reuse of live `--fixture` capture, and a separately authorized later live smoke |

No reconnaissance agent edited files, touched live services, or spawned nested
agents. The existing queued worker returns the full response through its
in-memory one-shot channel while `persist_service_job_finished` records only a
success boolean and redacted error. PoC 1 therefore keeps image bytes in the
immediate response without adding a second response channel or a durable
artifact registry.

## Required Tests

1. exact browser, stream, route, allocation, and session identity resolves;
2. missing, duplicate, mismatched, terminal, or replaced identity fails before
   provider invocation;
3. URL presence without attached-ready proof fails;
4. arbitrary caller display or provider URL fields are rejected;
5. capture never invokes display-access grant, browser launch, CDP attach,
   navigation, takeover, or input;
6. fixed PNG fixture returns deterministic dimensions, hash, byte count,
   context ID, geometry epoch, and ephemeral retention;
7. provider timeout, missing binary, invalid PNG, and oversized frame return
   typed redacted failures;
8. route or geometry drift after capture discards pixels and fails;
9. raw bytes and provider stderr do not enter persisted service state or logs;
10. CLI, HTTP, generic MCP, dedicated MCP, and generated client requests
    normalize to the same action and response;
11. service contracts and workspace capability discovery distinguish global
    support from current readiness;
12. existing page screenshot and remote-view handoff behavior remain intact.

## Validation

At minimum:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_capture -- --test-threads=1
pnpm test:service-client
pnpm test:service-api-mcp-parity
pnpm test:service-contracts-no-launch
pnpm test:actions-architecture
pnpm test:wsl-cargo-safety
pnpm --dir docs build
pnpm validation:select -- --base <last-known-green-ref>
git diff --check
```

Run dashboard tests only if dashboard or workspace capability projection is
touched. Do not run ignored E2E, a real display capture, or a live RDP or
Guacamole smoke under this plan's current authority.

## Hard Stops

- Stop if selection can escape service-owned browser and display identity.
- Stop if capture needs a caller-supplied display name or raw provider URL.
- Stop if capture mutates display access, route, lease, browser, profile, or
  operator state.
- Stop if implementation requires a real credential-bearing frame.
- Stop if image bytes cannot remain response-only and bounded.
- Stop if a dedicated ingress bypasses the canonical service queue.
- Stop if PoC 2 locator or PoC 3 input behavior enters this slice.

## Acceptance

PoC 1 is source accepted when the twelve requirements pass, the cross-ingress
contract and documentation are coherent, one fresh audit has no unresolved
blocking finding after the allowed remediation, and source status is recorded
without claiming installed or live proof.

The sole next recommendation is to write the detailed PoC 2 plan. No PoC 2
implementation begins in the PoC 1 closeout slice.
