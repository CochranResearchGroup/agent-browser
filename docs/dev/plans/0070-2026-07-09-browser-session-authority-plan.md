# Plan 0070: Browser Session Authority

Date: 2026-07-09
Status: Completed

## Goal

Add a deep Browser Session Authority interface that reconciles daemon service-state rows with process/resource truth and exposes a read-only lifecycle verdict to clients. This addresses the stale-process and out-of-sync inventory failure mode without adding automatic cleanup in the first slice.

## Source Audit

Primary audit: `docs/dev/notes/2026-07-09-browser-session-authority-audit.md`

Related notes and plans:

- `docs/dev/notes/2026-07-06-last30days-profile-routing-failure.md`
- `docs/dev/plans/0068-2026-07-06-operator-handoff-and-one-time-profile-hardening-plan.md`
- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`

## Design

Introduce a daemon-side authority snapshot with a shallow public interface and deep internals:

```text
BrowserSessionAuthoritySnapshot
  summary
  resourcePressure
  browserVerdicts[]
```

The snapshot should answer three questions:

1. Is the runtime under lifecycle pressure from stale, unowned, or cleanup-candidate processes?
2. Which service-state browser rows are viable enough for primary inventory?
3. Which rows should be held for the attention area because required authority inputs are missing or degraded?

## Implementation Steps

1. Create a Rust module for Browser Session Authority.
   - Use service-state browser rows as the modeled inventory.
   - Reuse process/resource classification from `service_resources.rs`.
   - Produce a serializable snapshot with aggregate resource pressure and per-browser verdicts.

2. Expose the snapshot through service status.
   - Add `browserSessionAuthority` to the action service-status response.
   - Add the same field to the control-plane status response so both status paths agree.
   - Keep the shape additive for compatibility.

3. Make resource facts reusable without weakening cleanup safeguards.
   - Keep termination and review-token logic inside the existing cleanup path.
   - Export only read-only summary data needed by the authority module.
   - Preserve protected/candidate/observed classification semantics.

4. Teach the dashboard projection to consume the authority snapshot.
   - Read `browserSessionAuthority` from service status.
   - Use per-browser verdicts to avoid initial primary-rail rendering for rows the authority marks non-viable.
   - Keep actionable degraded rows available for Needs Attention, sorted after viable rows.

5. Add focused validation.
   - Rust tests for authority summary with normal, candidate, and observed process samples.
   - Dashboard workspace-node tests for viable versus non-viable authority verdicts.
   - Contract fixture/schema update if the status schema is enforced.

## Non-Goals

- Do not automatically kill processes.
- Do not redesign the Guacamole route pool.
- Do not rewrite `workspace-remote-viewport.tsx`.
- Do not remove existing `service gc` review-token protection.

## Acceptance Criteria

- `service status` returns `browserSessionAuthority`.
- The authority snapshot includes process totals, candidate count, observed unowned `agent-browser` count, protected count, and a resource-pressure verdict.
- Each modeled browser receives a stable authority verdict keyed by browser id or equivalent stable id.
- The dashboard can consume the authority snapshot while preserving older status payload compatibility.
- Focused Rust and dashboard tests pass.

## Validation Commands

```bash
cargo test --manifest-path cli/Cargo.toml browser_session_authority
pnpm test:dashboard-workspace-nodes
```

If schema or generated client files change:

```bash
pnpm test:service-client
```

If Rust source outside the new module changes materially:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
```

## Completion Evidence

Completed on 2026-07-09.

Validation passed:

- `cargo test --manifest-path cli/Cargo.toml browser_session_authority`
- `cargo test --manifest-path cli/Cargo.toml service_status_response_combines_worker_and_service_state`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir packages/dashboard exec tsc --noEmit --pretty false`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:service-client`

The implementation exposes `browserSessionAuthority` through service status, models it in the contract and generated client, consumes it in dashboard workspace projection, suppresses non-viable authority rows from live inventory, keeps explicit attention rows visible, and preserves review-token-gated service GC behavior.

Independent outcome audit: initial audit found dashboard/client consumption gaps; remediation was re-audited with no remaining blocker.
