# Plan 0267: Hosted human takeover

Status: OPEN
Product lane: PL-PLATFORM
Work item: CochranResearchGroup/agent-browser#350
Branch: platform/hosted-takeover-350
Owner: ecochran76
Parent specification: #347

## Objective

A person can explicitly take control of a hosted browser and return it without automation racing their input. Viewing alone does not pause automation. All supported Agent Browser dispatch routes share the same durable control state; unmanaged OS/CDP controllers remain cooperative integrations.

## Scope

Add one durable browser-scoped control journal and narrow dispatch hooks, plus a public MCP/CLI control adapter. Keep browser/profile/tab custody in the existing session store and Native viewing in Remote View. Preserve pending outcomes across takeover and restart; never replay them automatically. Return requires a fresh observation of the same browser without recovery or launch. Optional read/capture pause keeps status and return available. Explicit override records the actor and reason against an expected current epoch.

## Coordination

P266 owns attachment and shared main feature documentation. P267 owns dedicated control modules and minimal MCP, host, action-dispatch and snapshot hooks. Integrate after P266; reconcile overlapping hooks by intent. No production effects or private-account acceptance.

## Validation

Provider-free state/race/persistence tests and dispatch-boundary regressions, focused public MCP synthetic acceptance where feasible, Rust formatting and workspace clippy through cargo-safe. Source tests do not establish installed, actual-host or unmanaged-controller interception evidence.
