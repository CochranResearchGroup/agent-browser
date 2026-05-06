# Fast CI Rust Root Cause

Date: 2026-05-06

## Finding

Fast CI was failing every push in the Rust unit-test job before the no-launch service smokes could run. The failing test was:

```text
mcp::tests::read_site_policies_resource_returns_policies_sorted_by_id
assertion `left == right` failed
left: Number(3)
right: 2
```

The root cause was a stale MCP test expectation. `read_service_mcp_resource_from_state` refreshes profile readiness before serving MCP resources. That refresh applies built-in site policies. After built-in Google, Gmail, and Microsoft policies were added, the effective `agent-browser://site-policies` resource correctly included built-in `gmail` along with the test's configured `google` and `microsoft` policies. The test still expected only the two configured policies.

## Prevention

When changing built-in service policy behavior, source metadata, or MCP/API resource refresh paths, include the MCP site-policy resource test in local validation. Focused smokes that only check generated metadata can miss stale unit expectations in the Rust job.

The repaired contract is that MCP site-policy resources expose the effective service view, not only persisted fixtures. Tests should assert both the sorted effective collection and source metadata so built-in defaults and local overrides stay intentional.
