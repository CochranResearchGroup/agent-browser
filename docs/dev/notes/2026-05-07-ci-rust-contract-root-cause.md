# CI Rust Contract Root Cause

Date: 2026-05-07

## Finding

Fast CI failed in the Rust unit-test job only. Dashboard, Version Sync Check, Service Client, Rust Quality, and the Release workflow were not the failing surfaces for the inspected commits.

The first recurring failure began when the service monitor MCP resource was added:

```text
mcp::tests::mcp_resources_lists_read_only_service_resources
left: String("agent-browser://monitors")
right: "agent-browser://site-policies"
```

`agent-browser://monitors` was inserted before `agent-browser://site-policies`, but one MCP unit test still asserted fixed resource indices from the pre-monitor resource list. The JSON-RPC resource-list test had already been updated, but the CLI-facing resource-list test was missed.

The latest failure added a second stale-contract issue after monitor incident fields became required:

```text
native::service_model::tests::service_operator_mutation_response_contracts_match_wire_shape
native::service_model::tests::service_trace_aggregate_contracts_match_wire_shape
missing required incident field monitorId
```

The incident contract requires `monitorId`, `monitorTarget`, and `monitorResult` so monitor-originated incidents have a stable wire shape. Two hand-built JSON fixtures in service-model contract tests did not include those fields as explicit nulls.

## Root Cause

The commits changed cross-cutting service contracts, but local validation stayed too narrow. The focused service-client and metadata checks were useful, but they did not execute the Rust unit tests that encode MCP resource ordering and required service-model wire fields.

The failures were not caused by runtime browser behavior, Chrome launches, release packaging, dashboard code, or pnpm service-client generation. They were stale Rust contract tests.

## Prevention

When a change adds an MCP resource, changes resource order, or changes required service-model fields, run the affected Rust filters before pushing:

```bash
cargo test --manifest-path cli/Cargo.toml mcp_resources_lists_read_only_service_resources -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml resources_list_returns_jsonrpc_resources -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_operator_mutation_response_contracts_match_wire_shape -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_trace_aggregate_contracts_match_wire_shape -- --test-threads=1
```

For broader contract changes, prefer `scripts/ci/rust-tests.sh` before pushing because it matches the CI Rust job more closely than service-client checks alone.
