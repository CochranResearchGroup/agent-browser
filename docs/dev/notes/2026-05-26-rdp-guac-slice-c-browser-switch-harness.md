# RDP Guac Slice C Browser Switch Harness

Date: 2026-05-26
State: VALIDATED
Plan: `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## Scope

Slice C now has an opt-in live harness:

- `scripts/test-rdp-guac-browser-switch-live.js`
- `pnpm test:rdp-guac-browser-switch-live`

The harness validates managed browser A/B switching for the RDP and Guacamole
path. It uses two `remote_headed` daemon sessions with distinct runtime
profiles in the same isolated `AGENT_BROWSER_HOME`, serves one dashboard route
from browser A's stream server, and relies on workspace `view_focus` routing to
target browser B when the URL selection changes.

## Required Inputs

- `AGENT_BROWSER_REMOTE_VIEW_URL`
- `AGENT_BROWSER_REMOTE_HEADED_DISPLAY`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE`
- `AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE`

Optional inputs:

- `AGENT_BROWSER_RDP_TEST_BROWSER_A`
- `AGENT_BROWSER_RDP_TEST_BROWSER_B`
- `AGENT_BROWSER_RDP_TEST_PROFILE_A`
- `AGENT_BROWSER_RDP_TEST_PROFILE_B`
- `AGENT_BROWSER_RDP_TEST_PUBLIC_URL`
- `AGENT_BROWSER_RDP_TEST_DISPLAY_ISOLATION`

## Evidence Captured By A Successful Run

The harness creates
`/tmp/agent-browser-rdp-guac-browser-switch-<timestamp>/` and writes:

- browser A and browser B launch responses
- fixture metadata for both browser daemon sessions, both browser runtime
  profiles, both dashboard clients, both workspace URLs, and the RDP gateway
  URL
- service status after launch, after switching to B, after refreshing B, after
  client 2 opens A, and at the end
- dashboard state for browser A, browser B, refresh recovery, and A/B
  alternation
- screenshots for browser A, browser B, browser B after refresh, client 2 on
  browser A, each A/B alternation, and external open
- the external-open `view_takeover` job
- a summary naming the observed cross-browser viewer outcome

The run passes only when both browser records remain distinct and `ready`, the
same dashboard client can switch from A to B by workspace URL, browser B
survives refresh, a second dashboard client can open A while the first remains
routed to B, alternation observes service-owned `view_focus` jobs, and external
open queues a succeeded `view_takeover`.

## Current Validation Status

The harness passed against the live RDP and Guacamole deployment. Slice C live
evidence is recorded in
`docs/dev/notes/2026-05-26-rdp-guac-slice-c-live-validation.md`.
