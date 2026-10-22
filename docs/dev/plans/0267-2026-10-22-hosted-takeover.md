# Plan 0267: Hosted human takeover

Status: CLOSED
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

P268 owns attachment and shared main feature documentation. P267 owns dedicated control modules and minimal MCP, host, action-dispatch and snapshot hooks. Integrate after P268; reconcile overlapping hooks by intent. No production effects or private-account acceptance.

## Validation

Provider-free state/race/persistence tests and dispatch-boundary regressions, focused public MCP synthetic acceptance where feasible, Rust formatting and workspace clippy through cargo-safe. Source tests do not establish installed, actual-host or unmanaged-controller interception evidence.

## Source closeout

Integrated with the paired attachment/control outcome through provider PR361 and
consumer PR175, in provider-first order. The [source acceptance note](../notes/0268-2026-10-22-hosted-attachment-control.md)
records the model/session/control/MCP/action, strict Rust and client/docs gates,
actual Native run 04 with healthy chat, 63 marker samples and 26 WebSocket frames,
and positive absence of 14 observed app process generations before release.
Native run 03 retains its invalid cleanup history and separate correction evidence.
Native run 05 also passes synthetic cookie continuity through both task detachments,
healthy chat, real Native SDK control, 63 markers/47 WebSocket frames and all 14
observed process generations absent before exact release, with natural harness exit.
Accepted source only; no production deployment or private-account acceptance.
The wider #347 specification and unrelated P266/P269 custody remain separate.
