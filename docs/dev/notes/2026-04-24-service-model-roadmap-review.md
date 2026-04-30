# Service Model Roadmap Review

Date: 2026-04-24

## Context

This note reviews the current `service-model-phase-0` branch against the
service roadmap after the recent MCP browser-tool expansion.

The roadmap remains to make agent-browser an always-available browser control
plane. The service should own browser processes, CDP connections, profiles,
session leases, tabs, queued jobs, site policy, health, incidents, and event
history. MCP should be the primary agent interface, while HTTP and WebSocket
APIs should serve projects, dashboards, and other software integrations.

## Current Position

The recent MCP slice materially improved the agent-facing control surface.
Agents can now submit named service, agent, and task context and use queued
browser tools for common inspection and interaction work, including snapshots,
URL and title reads, tabs, screenshots, clicks, form entry, keyboard and pointer
actions, dropdowns, targeted DOM reads, readiness checks, checkbox state,
scrolling, focus, clearing, and waits.

That is enough MCP surface for the current phase. Continuing to expose every
native command through MCP before the service state model is more authoritative
would repeat the dashboard sequencing problem noted on 2026-04-22: client
surfaces would keep growing faster than the backend control-plane authority.

## Roadmap Alignment

The branch has made progress on several roadmap pillars:

- control requests run through the queued service path rather than direct
  ad hoc mutation
- retained jobs capture caller context for multi-service and multi-agent
  tracing
- service resources expose profiles, sessions, browsers, tabs, jobs, events,
  incidents, policies, providers, and challenges
- reconciliation discovers live CDP targets for known browser endpoints and
  records tab lifecycle changes
- browser health and incident surfaces exist for crash, disconnect, timeout,
  and cancellation visibility

The remaining gap is not another client command. The gap is authority. The
service must become the durable source of truth for which browsers, profiles,
sessions, tabs, leases, and jobs are active, stale, faulted, or reusable.

## Next Architecture Slice

The next slice should be service-owned active state reconciliation.

Definition of done:

- Persisted browser records are reconciled against process state, DevTools
  endpoint reachability, and `/json/list` target discovery.
- `ServiceState.browsers` records clear health transitions for ready,
  degraded, unreachable, process-exited, CDP-disconnected, and recovered
  cases where the existing model can represent them.
- `ServiceState.tabs` is treated as service-owned state, with stale targets
  marked closed or otherwise made unambiguous.
- Session and browser tab relationships are refreshed from the reconciled
  target set where ownership is known.
- Reconciliation emits bounded events and derived incidents for meaningful
  browser health and tab lifecycle changes.
- MCP resources, CLI status, HTTP APIs, and dashboard panels consume the same
  reconciled state rather than inventing separate client-side truth.
- Targeted tests cover reconciliation edge cases and retained state shape.
- One live Chrome smoke validates that an existing browser launch, reconcile,
  and MCP resource read agree about active browser and tab state.

This should happen before adding more MCP browser commands, more dashboard
panels, or higher-level auth, challenge, monitor, and provider workflows.

## Profile And Session Follow-Up

After active state reconciliation, the next policy-oriented slice should define
profile and session leasing more explicitly:

- profile allocation policy: shared service profile, profile per service,
  profile per site, profile per identity, or caller-supplied custom profile
- keyring posture: basic password store, real OS keychain, managed vault
  provider, or manual login profile
- session lease ownership: service name, agent name, task name, profile,
  browser, tab set, timeout, and cleanup policy
- reuse rules: which services may share one profile or browser process, and
  which must be isolated
- resource tradeoffs: when identity isolation justifies an additional Chrome
  process and when a shared browser with separate tabs is acceptable

This keeps anti-anti-bot hardening, credential providers, passkeys, 2FA,
captcha handling, and headed remote browser management grounded in an
auditable state model instead of scattered per-tool behavior.

## Working Rule

Until the next backend authority slice lands, treat the MCP browser-tool set as
sufficient for Phase 0. Only add MCP tools that are needed to validate service
authority work or close a concrete gap in the live smoke path.

Best next step: implement service-owned browser and tab reconciliation hardening
with crash, disconnect, stale-target, and recovery events, then verify the same
state through CLI, HTTP, MCP resources, and the existing dashboard consumer.
