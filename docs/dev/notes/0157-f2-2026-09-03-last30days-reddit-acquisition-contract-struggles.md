# Last30Days Reddit acquisition contract struggles

Date: 2026-09-03

Status: FIELD OBSERVATION RECORDED | PRODUCT REPAIR NOT AUTHORIZED

Related plans: P155 durable handoff resume intent; P157 profile permissions and
request provenance

Scope: authenticated Service acquisition, route-bound cold launch, retained
display allocation identity, and service tab-handle acquisition

Authority: REDACTED FIELD EVIDENCE AND PROVIDER-FREE PRODUCT FOLLOW-UP ONLY

## Bottom line

The five-attempt Last30Days Reddit campaign exposed a mix of consumer mistakes,
Agent Browser documentation and API gaps, and genuine residual Agent Browser
defects. The campaign did not fail because the browser profile lacked Reddit
authentication. Its fifth attempt opened the existing authenticated profile on
a ready Guacamole/RDP route, and a later exact-handle probe confirmed the page
was authenticated. The campaign still could not start scraping because the
client had no service tab handle for its first evaluation.

The classification is:

- **Last30Days client defects:** the first request placed route material at the
  wrong schema level, the third request used a direct CLI path that could not
  carry the required protected Service authority, and the MCP adapter hid the
  server's exact JSON-RPC validation error.
- **Agent Browser documentation or API gaps:** the available guidance did not
  provide one end-to-end recipe for route-bound, protected-profile acquisition;
  direct CLI and authenticated `service_request` capabilities differ; and the
  successful `remote_view_open` plus discovery-only `tab_list` sequence did not
  yield a service tab handle.
- **Genuine Agent Browser residual defects:** generic cold `tab_new` accepted a
  route-pool hint but ignored it during launch, and a display-number-derived
  allocation identifier collided with a released historical allocation whose
  stored display name differed. Route readiness did not expose that collision
  before the live request.

This note follows
[the earlier durable-handoff field note](0157-f1-2026-09-02-last30days-reddit-handoff-link-errors.md).
It contains no credentials, cookies, page bodies, raw provider URLs, profile
paths, capability bearer material, private handoff identifiers, or scraped
posts. It does not authorize profile replacement, browser termination, route
takeover, installation, provider interaction, or a production repair.

## Operator goal and attempt boundary

The operator authorized five additional attempts to collect 80 unique Reddit
home-feed posts from the existing authenticated social profile. Each retry
required a distinct evidence-backed reason. Reddit remained disabled in every
recurring schedule, and each live specification remained disabled outside its
one manual invocation.

The consumer used all five attempts. None accepted or stored a Reddit post. The
campaign did progressively prove request transport, route-bound launch,
protected-profile authority, the authenticated page, and the missing tab-handle
boundary.

## Attempt ledger

| Attempt | Last30Days run | Agent Browser evidence | Result | Primary classification |
|---|---|---|---|---|
| 1 | `collection-run-8a0feed7763c128fa80c96c47e134109` | No Agent Browser job was created | MCP schema rejected top-level route material after 58 ms | Last30Days client defect; documentation discoverability gap |
| 2 | `collection-run-259f395f5128215dd2a04e6165a2f817` | `mcp-service-request-tab_new-33ba6452-3ef6-44f0-8302-30bc1b4f24f1` | `tab_new` retained Route B but cold launch attempted private Xvfb display `:90` three times | Agent Browser behavioral defect or misleading action contract |
| 3 | `collection-run-e3e4a93eca1eb48fce3ecf6179de9df7` | `r646051` | Direct `remote_view_open` reached Route B, then failed `existing_session_profile_identity_unproven` | Last30Days path-selection defect; Agent Browser CLI and Service API asymmetry |
| 4 | `collection-run-4216a1d89238daec3eacf3b531b15aca` | `mcp-service-request-remote_view_open-44a0fa3e-4af4-49d0-b7ae-326e8ba2f9f0` | Failed `route_pool_target_mismatch` against derived allocation `remote-view-display:10` | Genuine Agent Browser retained-state identity defect |
| 5 | `collection-run-a15de07b9397bee8683a459f61229fa7` | `mcp-service-request-remote_view_open-348fa7de-d78b-4f0d-8378-417758b2c444` | Route open succeeded; first evaluation lacked `serviceTabHandle` | Agent Browser API and documentation gap plus Last30Days error-loss defect |

## Detailed findings

### 1. Route material has two request locations

The first retry sent `routePoolEntryId` at the top level of the MCP request.
The Agent Browser MCP schema admits action material under `params`, not at that
top level. The request therefore failed before Agent Browser created a Service
job.

Agent Browser rejected the malformed request correctly. This was a Last30Days
client defect. The difficulty was discoverability: caller attribution, browser
and session routing, protected authority, and action parameters occupy different
levels of the same request. The Service guide states that route-selection
material belongs under `params`, but no narrow route-bound scraper example
showed the complete request shape.

Last30Days commit `dec7672` moved the field into `params` and proved that the
request then crossed the MCP boundary.

### 2. Generic cold tab acquisition accepted but did not honor the route

The second retry created an Agent Browser `tab_new` job. Its retained job record
contained `routePoolEntryId=guacamole-rdp-b`, so the service accepted the route
intent. The cold auto-launch path nevertheless attempted private virtual Xvfb
display `:90` three times. It never used Route B's ready shared display.

Current source keeps generic browser auto-launch separate from the dedicated
remote-view route and display allocator. This is not merely a Last30Days
misunderstanding. Agent Browser accepted and retained a route hint on an action
whose cold-launch implementation did not consume it. A route-bearing `tab_new`
request should either delegate cold launch to route-aware acquisition or fail
validation before it enqueues a job.

Last30Days commit `880b82d` worked around the split by selecting
`remote_view_open` for cold route-bound acquisition.

### 3. The direct remote-view CLI could not express the required authority

The third retry used the direct `remote-view open` CLI. It reached Route B but
failed `existing_session_profile_identity_unproven`. The workflow required the
protected Service principal and a reviewed duplicate-profile-lane override.
The authenticated `service_request` surface can carry that authority. The
direct remote-view CLI used by the consumer could not express the same
combination.

This was primarily a Last30Days path-selection defect, repaired in commit
`a6e7178` by routing `remote_view_open` through authenticated
`service_request`. It also exposes an Agent Browser API and documentation gap:
the direct CLI and Service action have materially different authority, but the
route-opening guidance does not present a capability matrix or warn a software
client when the direct path cannot satisfy protected-profile acquisition.

### 4. A historical allocation key governed the wrong display

The fourth retry used authenticated `service_request` and reached route
planning. It failed:

```text
route_pool_target_mismatch: route pool entry 'guacamole-rdp-b' does not target
display allocation 'remote-view-display:10'
```

The route was not wrong. Route B targeted live display `:10`. The retained
allocation named `remote-view-display:10` stored display name `:11`, belonged
to an unrelated historical session, and was already released. Current source
derives a default route allocation identifier from the route target when the
caller does not supply an explicit allocation identifier. Reuse of the display
number therefore selected the conflicting historical row, and the strict
route-to-display guard rejected the current healthy route.

This is a genuine Agent Browser retained-state identity defect. A released
historical allocation should not govern a new route checkout merely because
its key resembles the current display number. Route B simultaneously projected
`available` and `ready`, so the readiness surface also failed to warn that the
default derived allocation would be rejected during the effect.

A Service prune dry run did not offer an applicable safe repair for the exact
historical key because the record remained linked to diagnostic history. The
field investigation found no supported exact rekey or quarantine action for
this collision.

Last30Days commit `d69467d` used a fresh session-scoped allocation identifier
while retaining verified display `:10`. A live no-effect route preflight then
returned the requested Route B plan with zero blockers. That workaround does
not repair the underlying Agent Browser identity rule.

### 5. Remote view succeeded without an automation handle

The fifth retry successfully opened Route B. The retained job reported success,
the browser displayed Reddit, and the browser contained an active home-feed
target. Direct evaluation through the exact retained service handle later
reported authenticated state with no login form, checkpoint, or network block.

The consumer's first evaluation still failed because `evaluate` requires
`serviceTabHandle`. The successful `remote_view_open` response did not provide
one. The follow-up `tab_list` operation was discovery-only. Current
`BrowserManager::tab_list` returns index, title, URL, type, and active state; in
verbose mode it adds target and page-session identifiers. It does not mint or
return a service tab handle. By contrast, `tab_new` has an explicit optional
`serviceTabHandle` result.

Agent Browser's handle requirement is deliberate and correct. The gap is that
the documented route-open workflow establishes operator presentation but does
not establish an immediately usable automation capability, and the discovery
surface cannot adopt the selected retained target. The only clear consumer
workaround was to create another tab with `tab_new`, adopt its handle, and
clean up the redundant tab later.

Agent Browser returned the actionable JSON-RPC validation text
`evaluate requires serviceTabHandle`. The Last30Days MCP adapter collapsed that
response into a generic no-result error. That error loss belongs to the
consumer, not Agent Browser. Last30Days commit `d1ff629` now creates an exact
route-owner tab after `remote_view_open`, validates its returned handle, and
uses that handle for evaluation. The five-attempt ceiling prevented a live
test of that final consumer repair.

## Classification matrix

| Finding | Agent Browser defect | Agent Browser docs or API gap | Last30Days defect |
|---|---:|---:|---:|
| Route material sent at the wrong request level | No | Yes, narrow example missing | Yes |
| Route hint retained by cold `tab_new` but ignored by auto-launch | Yes | Yes, action contract is misleading | No |
| Direct CLI lacked protected Service authority used by `service_request` | No confirmed implementation defect | Yes, capability asymmetry is unclear | Yes, wrong path selected |
| Released `remote-view-display:10` row described display `:11` and blocked Route B `:10` | Yes | Yes, no exact repair workflow found | No |
| Ready route projection omitted the derived-allocation conflict | Yes | Yes, preflight expectations are unclear | No |
| Successful `remote_view_open` plus `tab_list` yielded no service tab handle | API ergonomics gap | Yes, no end-to-end recipe or adopt operation | Workaround required |
| Exact JSON-RPC error became a generic no-result consumer error | No | No | Yes |
| Failed job projections retained only string errors and sparse result identity | Observability gap | Yes | No |

## Why the diagnosis took so long

The struggle came from crossing several individually strict but weakly joined
surfaces:

1. MCP schema validation happened before job creation, so attempt 1 had no
   Agent Browser job to inspect.
2. A job could retain a route hint without proving that its launch path would
   use the route.
3. Direct CLI and authenticated Service requests exposed different authority.
4. Route readiness described provider capacity but did not include compatibility
   with the allocation identifier the effect would derive.
5. A successful presentation action did not return an automation handle.
6. Job records commonly retained only `result.success`, sparse top-level
   routing identity, and an unstructured error string. Diagnosis required
   joining Service Status, route state, display allocations, consumer receipts,
   and source structure.
7. Historical retained state contained many released or orphaned sessions and
   allocations. The maintenance surface could classify them but did not offer a
   narrow repair for the exact conflicting allocation key.

The browser itself was healthy by attempt 5. Most of the remaining work was
contract archaeology across the control plane, not provider scraping.

## Product changes recommended

### Make route intent executable or reject it early

For cold `tab_new`, either consume route-pool intent through the same allocator
as `remote_view_open` or reject route-bearing requests before a job starts.
Never retain a route-pool identifier on a job that launches an unrelated
private display.

### Make display allocation identity collision-resistant

Do not derive a globally reusable allocation key from the display number alone.
Use immutable route, checkout, or session identity. Ignore or quarantine a
terminal historical row when its stored display name, route owner, boot epoch,
or profile does not match the new request.

Add the same compatibility check to access planning, route preflight, and
doctor. A route should not project ready for a request whose default allocation
will deterministically fail at effect time.

### Add an exact retained-allocation repair

Provide reviewed plan and apply operations that can quarantine or rekey one
terminal conflicting display allocation without deleting profiles, cookies,
browsers, unrelated route history, or diagnostic evidence.

### Return or acquire an automation handle after route open

Have `remote_view_open` return the selected browser, session, target, and a
valid service tab handle when its selected target is automation-eligible.
Alternatively, add an explicit action that adopts one exact retained target and
mints a service tab handle under Service policy. Do not require clients to infer
handle identity from `tab_list` or raw target identifiers.

### Publish one protected route-bound acquisition recipe

Document this complete software-client sequence:

1. request `service_access_plan` with caller and target attribution;
2. use the returned protected capability and replacement or reuse identity;
3. request authenticated `remote_view_open` with route material in `params`;
4. acquire or receive one exact service tab handle;
5. run handle-scoped evaluation and extraction; and
6. release the handle while preserving the shared browser and profile.

The guide should state which fields are top-level routing authority, which are
action parameters, and which capabilities exist only on authenticated Service
requests rather than the direct CLI.

### Preserve actionable job evidence

Retain the structured failure object, retry disposition, browser ID, session
name, selected target, requested and selected display allocation, and relevant
service tab handle on each job. Preserve the exact MCP validation error when a
request fails before job creation through a correlated request trace.

## Provider-free acceptance matrix

1. Submit cold `tab_new` with a route-pool identifier. It must either launch on
   that route's display or fail validation before enqueue.
2. Seed a released allocation `remote-view-display:10` whose stored display is
   `:11`, then request a ready route targeting `:10`. The historical row must
   not block the new checkout.
3. Run access plan and preflight against that collision. Both must expose the
   conflict or select a collision-resistant allocation before any effect.
4. Acquire a protected profile through authenticated `remote_view_open` with a
   reviewed duplicate-lane override. The direct CLI must either support the
   same authority or report that the Service surface is required.
5. Complete `remote_view_open` on a browser with multiple retained tabs, then
   immediately perform handle-scoped evaluation without guessing from tab
   order, URL, target ID, or page-session ID.
6. Verify `tab_list` remains discovery-only while an explicit adopt action can
   mint a handle for one exact eligible target.
7. Trigger a missing-handle evaluation and verify that the caller receives
   `evaluate requires serviceTabHandle` plus structured recourse.
8. Plan and apply exact quarantine of the conflicting historical allocation.
   Verify that unrelated profiles, browser records, route history, and
   diagnostics remain intact.

## Evidence index

Agent Browser retained the following relevant jobs:

- retry 2 `tab_new` failure:
  `mcp-service-request-tab_new-33ba6452-3ef6-44f0-8302-30bc1b4f24f1`;
- retry 3 direct remote-view failure: `r646051`;
- retry 4 route/display mismatch:
  `mcp-service-request-remote_view_open-44a0fa3e-4af4-49d0-b7ae-326e8ba2f9f0`;
  and
- retry 5 successful route open:
  `mcp-service-request-remote_view_open-348fa7de-d78b-4f0d-8378-417758b2c444`.

Last30Days recorded the complete attempt receipts in Plan 0063 checkpoint C11
and Runbook Turn 403. The consumer workarounds landed as commits `dec7672`,
`880b82d`, `a6e7178`, `d69467d`, and `d1ff629`. Installed Last30Days service
0.3.106 contains the final workaround and passed its complete repository suite,
but no sixth live Reddit attempt was run.

At the final Agent Browser readback for this note, Route B was available and
ready on display `:10`; the successful attempt-5 browser had been intentionally
released; and the contradictory released allocation
`remote-view-display:10` still stored display `:11`. This readback confirms the
historical collision remains present without claiming it currently blocks all
browser work.

## Recommended next Agent Browser slice

Start with the collision fixture and the route-bearing cold `tab_new` fixture.
They isolate the two confirmed behavioral defects without using a live social
profile. Then define the route-open-to-service-handle contract and add the
protected acquisition recipe. Validate the product changes in the isolated
development runtime before any production installation or another provider
attempt.
