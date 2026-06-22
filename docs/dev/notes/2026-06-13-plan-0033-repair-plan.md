# Plan 0033 Repair Plan

Date: 2026-06-13
State: IMPLEMENTED
Lane: P14
Parent Plan: `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`

## Trigger

The Plan 0033 audit found two contract gaps in the implemented Slice E
diagnostics and readiness checkpoint:

- MCP `service_request` accepted `diagnostics`, but its public input schema and
  command builder did not expose or forward `includeScreenshot`,
  `screenshotDir`, `maxConsoleEntries`, `maxErrorEntries`, or
  `maxRequestEntries`.
- The shared service-request JSON contract says raw HTTP and MCP requests reject
  stale, unverified, or missing `monitorRunDueSummary` freshness evidence unless
  `allowMonitorFreshnessRisk` is true. HTTP enforced that gate, but MCP did not
  expose, forward, or reject those fields.

## Scope

Repair only the service-request contract parity gaps. Do not add provider
selectors, private AuraCall profile state, or a new browser lifecycle path.

## Implementation Steps

1. Extend the MCP `service_request` input schema with the diagnostics evidence
   options and monitor freshness fields already documented by
   `docs/dev/contracts/service-request.v1.schema.json`.
2. Parse and forward those fields through the MCP `ServiceToolContext` into the
   queued daemon command.
3. Add MCP-side monitor freshness rejection that matches the HTTP behavior for
   expired, unverified, missing, and explicitly overridden summaries.
4. Add focused Rust tests so MCP schema exposure, command forwarding, and
   rejection behavior cannot drift behind HTTP again.
5. Re-run the focused service-request, parity, and client checks, plus
   whitespace and validation selection.

## Completion Evidence

The repair is complete when:

- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
  covers the MCP monitor freshness and diagnostics forwarding paths.
- `pnpm test:service-api-mcp-parity` still passes.
- `pnpm test:service-client` still passes.
- `git diff --check` passes.
- The parent plan records the repair checkpoint and the remaining Plan 0033
  work accurately.
