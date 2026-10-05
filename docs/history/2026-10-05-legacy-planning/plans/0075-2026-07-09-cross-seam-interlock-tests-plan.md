# Plan 0075: Cross-Seam Interlock Tests

Date: 2026-07-09
Status: Completed

## Goal

Add fast no-launch tests that prove daemon authority, generated client contracts, dashboard inventory, stream readiness, and selected viewport context consume the same interlock data.

## Source Audit

Primary audit: `docs/dev/notes/2026-07-09-cross-seam-interlock-tests-audit.md`

Relevant files:

- `docs/dev/contracts/service-status-response.v1.schema.json`
- `packages/client/src/service-observability.generated.d.ts`
- `packages/dashboard/src/lib/service-workspaces.ts`
- `packages/dashboard/src/lib/selected-workspace-context.ts`
- `packages/dashboard/src/lib/service-view-streams.ts`
- `scripts/test-dashboard-workspace-nodes.js`
- `scripts/test-dashboard-view-streams.js`
- `scripts/test-service-observability-client.js`

## Design

Create a fixture-driven script:

```text
scripts/test-cross-seam-interlocks.js
```

The fixture should contain:

- one viable service-owned browser
- one non-viable authority browser
- one authority attention browser with live evidence
- one viable detected non-owned CDP browser
- one stream with ready readiness
- one stream with known-bad readiness

The test should call real helpers rather than asserting text snapshots.

## Implementation Steps

1. Add the cross-seam fixture script.
   - Validate service-status schema if the existing schema helper can be imported cheaply.
   - Derive workspace nodes and live workspace nodes.
   - Build selected-workspace context for viable, non-viable, and attention rows.
   - Run stream helper gating for ready and bad readiness states.

2. Wire it into package scripts.
   - Add a focused `pnpm test:cross-seam-interlocks` command.
   - Include it in the validation selector if this repo already maps script names by path.

3. Add assertions.
   - Non-viable authority rows are absent from live rail.
   - Attention rows are present and grouped as Needs Attention.
   - Selected context does not revive non-viable rows as viewable/controllable.
   - Generated service-client type output contains `browserSessionAuthority`.
   - Known-bad readiness blocks view/control helper opening.

4. Keep it no-launch.
   - Do not start Chrome, Guacamole, RDP, or the service daemon.
   - Do not depend on live ports or screenshots.

## Non-Goals

- Do not replace live RDP smoke tests.
- Do not validate browser rendering pixels.
- Do not mutate service state.
- Do not introduce a broad end-to-end harness in this slice.

## Acceptance Criteria

- `pnpm test:cross-seam-interlocks` passes.
- The test fails if `browserSessionAuthority` is removed from generated client types.
- The test fails if dashboard projection ignores non-viable authority verdicts.
- The test fails if known-bad stream readiness states remain openable.

## Validation Commands

```bash
pnpm test:cross-seam-interlocks
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-view-streams
pnpm test:service-client-contract
```

## Closeout Evidence

Implemented `scripts/test-cross-seam-interlocks.js` and wired
`pnpm test:cross-seam-interlocks` in `package.json`.

Validation completed on 2026-07-10:

```bash
pnpm test:cross-seam-interlocks
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-view-streams
pnpm test:service-client
```

Independent outcome audit: passed with no blocking findings; residual risk is that this is fixture-backed no-launch coverage rather than live daemon, browser, or RDP behavior.
