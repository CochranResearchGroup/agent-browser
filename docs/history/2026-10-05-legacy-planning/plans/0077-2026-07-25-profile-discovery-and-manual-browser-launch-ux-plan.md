# Plan 0077: Profile Discovery And Manual Browser Launch UX

Date: 2026-07-25
Status: Complete
Lane: P77

## Goal

Make registered browser identities directly discoverable, explainable, and
launchable through one authoritative service contract and one operator-facing
dashboard workflow. Manual no-CDP browsers must remain visible and accurately
owned throughout their lifecycle without weakening the one-process-per-profile
invariant.

The source requirement is:

- `docs/dev/notes/2026-07-25-profile-discovery-and-manual-browser-launch-ux.md`

The note remains unchanged. Existing launcher and profile-lookup surfaces are
implementation inputs, not completion evidence. They may be reworked or
replaced where their behavior does not satisfy the note.

## Current Contradictions

The plan starts from current behavior rather than assuming the existing
surfaces are correct:

1. `agent-browser://profiles/lookup?targetServiceId=x` currently selects
   `stealthcdp-default` with `browser_build_default` even though
   `last30days-facebook` has an exact authenticated X target. This violates the
   required precedence contract.
2. The dashboard has a launcher panel, but its combinations, search semantics,
   access-plan step, and generic `Launch` action do not provide the direct
   profile and website workflow required by the note.
3. Detached `runtime login` writes runtime PID and profile state but is absent
   from service browser/session/tab inventory and therefore from the left rail.
4. Resource discovery can observe Chromium processes but does not have a
   durable launch identity that safely correlates a manual browser, profile,
   target, display, route, and lifecycle.
5. Existing profile lookup returns one selected profile. It does not return
   ranked alternatives, complete match evidence, holder actionability, or the
   same search semantics across CLI, HTTP, MCP, client, and dashboard.

## Deep-Module Design

### Profile catalog and selector

One Rust module owns profile and site catalog normalization, safe searchable
metadata, deterministic candidate ranking, match evidence, not-found behavior,
holder actionability, and recommended next action.

Exact explicit profile, authenticated target plus account, authenticated
target, account, target, alias, tag, and safe free-text matches are ordered
before browser-build compatibility. Browser build is a compatibility
constraint or tie-breaker, never an identity-bearing fallback when the request
contains an unmatched identity.

Search is read-only. Launch remains a separate serialized service request.

### Manual browser lifecycle

`runtime login` writes an authoritative manual-launch record before returning.
The record binds:

- stable launch ID;
- runtime profile and normalized user-data directory;
- browser PID plus process start identity;
- browser family and executable;
- launch mode and target URL;
- display, route, and remote-view metadata when known;
- start, last-observed, and exit timestamps;
- lifecycle state and supported operations;
- source provenance.

Process discovery reconciles a launch record but does not invent ownership from
an uncorrelated process. PID reuse and Chromium child processes cannot inherit
the record. Clean exit marks the record closed; stale records become explicit
diagnostic history rather than live browser rows.

### Workspace projection

The service inventory projects live manual-launch records as a dedicated
manual-browser class. The row states which operations are supported and never
advertises CDP automation without an attachable endpoint. Placement remains a
pure projection; lifecycle authority stays in the service/runtime record.

### Dashboard profile workspace

The dashboard uses the authoritative search response rather than locally
ranking raw profiles. It presents profiles and sites as human-recognizable
records with direct actions:

- Open browser;
- Open website;
- Add tab;
- Seed login;
- View or control;
- Inspect holder.

Each mutating action is created from an access plan and submitted through the
service queue. The UI does not synthesize profile ownership, compatibility, or
route policy.

## Privacy And Authorization

- Searchable account labels are explicitly safe display labels, not secrets.
- Raw cookie values, authentication tokens, browser storage, private page
  content, and full process command lines are never indexed or returned.
- Match evidence returns the safe normalized field and value responsible for
  the rank.
- Dashboard URLs contain stable catalog IDs and action IDs, not raw account
  identifiers or evidence text.
- Profile paths remain available to local superuser/operator surfaces but are
  not included in generic search text or unauthenticated URLs.

## Slice A | Contract And Selector Repair

Status: Complete

- Add failing fixtures for exact X selection, unmatched identity `not_found`,
  ranked alternatives, aliases, tags, profile names, safe account labels,
  holder actionability, and provenance.
- Replace browser-build fallback for identity-bearing misses.
- Define one versioned profile-discovery response shared by CLI, HTTP, MCP,
  generated client, and dashboard.
- Return selected profile, ranked alternatives, match field/value/type,
  readiness, holder, route availability, recommended action, provenance, and
  copyable request/dashboard references.
- Add a first-class CLI lookup/search command rather than requiring raw MCP
  resource reads.

Exit criteria:

- Exact X lookup selects `last30days-facebook` for authenticated target reason.
- An unknown identity returns structured `not_found`; it never selects the
  default profile.
- All transports serialize the same deterministic ranking and actionability.

## Slice B | Searchable Profile And Site Catalog

Status: Complete

- Extend profile records with description, aliases, safe account labels, site
  origins, browser posture, and per-target observation provenance.
- Add site/login catalog records for canonical name, aliases, origins, login
  IDs, safe account labels, recommended profiles, browser posture, challenge
  constraints, manual-login requirements, freshness probes, bounded guidance,
  and using adapters/services.
- Layer config, persisted state, built-ins, and runtime observation with
  explicit source precedence.
- Keep catalog mutations serialized through existing service-state governance.

Exit criteria:

- `X`, `x.com`, `Twitter`, `Facebook`, `LinkedIn`, and `last30days` produce
  deterministic results using safe catalog metadata.
- Provenance and observation time are present for every returned readiness or
  runtime claim.

## Slice C | Authoritative Manual No-CDP Browser Lifecycle

Status: Complete

- Add a persisted manual-launch record and state transitions.
- Write the record from `runtime login` and CDP-free service launch.
- Reconcile exact parent browser PID, process start identity, profile, display,
  route, and liveness without attaching CDP.
- Mark exit and remove the live projection without losing bounded history.
- Expose manual launch records through service status, focused HTTP and MCP
  resources, generated client helpers, and CLI text/JSON.

Exit criteria:

- A live detached no-CDP login browser has one authoritative manual record.
- PID reuse and Chromium child processes do not create false owners.
- Browser exit removes the live row and records a closed lifecycle outcome.

## Slice D | Workspace Inventory Projection

Status: Complete

- Add a `manual-observed-browser` inventory class and placement.
- Project profile, PID, browser family, display/route, target site, remote
  control availability, unsupported automation, lifecycle, and next action.
- Preserve detected foreign-CDP and service-owned inventory semantics.
- Ensure manual rows never expose snapshot, click, navigation, tab, or other
  CDP actions without an attachable endpoint.

Exit criteria:

- The left rail shows a live manual browser with accurate ownership.
- The row remains operator-actionable without claiming automation.
- Closed or stale records cannot remain in active inventory.

## Slice E | Dashboard Profile Discovery And Direct Actions

Status: Complete

- Replace the launcher-local filter/rank model with server-backed profile and
  site discovery.
- Add direct profile/site results and target-specific readiness.
- Render holder and route state before action selection.
- Implement Open browser, Open website, Add tab, Seed login, View or control,
  and Inspect holder from authoritative recommendations.
- Persist selected profile, site, and action in safe dashboard URL fields.
- Refresh workspace inventory after accepted launch jobs.

Exit criteria:

- Searching `X` or `last30days` finds the named shared profile and shows
  Facebook, LinkedIn, and X readiness.
- One direct action produces a visible browser or a typed service-owned
  blocker.
- The resulting manual, detected, or service-owned browser appears in the left
  rail with accurate ownership.

## Slice F | Validation, Installed Runtime, And Closeout

Status: Complete

Required no-launch validation:

- deterministic Rust selector tests;
- CLI parser and output tests;
- HTTP and MCP parity;
- schema and generated-client parity;
- dashboard search, action, and inventory interaction tests;
- manual lifecycle and PID-reuse fixtures;
- policy and privacy redaction fixtures.

Required live validation:

1. Use a disposable registered profile and harmless target.
2. Search by profile name, site alias, origin, target, login, tag, and safe
   account label.
3. Prove unknown identity returns `not_found`.
4. Launch a detached no-CDP browser on a disposable display.
5. Prove the manual browser appears in service inventory and the dashboard left
   rail with the same profile, PID, display, route, and target.
6. Prove unsupported automation actions are absent.
7. Close the browser and prove the live row disappears while bounded lifecycle
   history remains.
8. Launch or reuse an attachable browser and prove Add tab versus duplicate
   process behavior.
9. Publish the exact source state and verify installed runtime and dashboard
   readbacks.

Closeout requires:

- all required user-facing documentation surfaces;
- a durable privacy-safe validation receipt;
- ROADMAP and RUNBOOK updates;
- Graphiti closeout memory sourced from this plan and its validation receipt;
- clean worktree, committed and pushed `main`;
- a true requirement-by-requirement audit against the source note.

## Validation Receipt

Completed: 2026-07-25

Requirement audit:

- Profile and login discovery now uses one versioned deterministic contract
  across CLI, HTTP, MCP, generated client, and dashboard. Exact X lookup
  selected `last30days-facebook` with reason `authenticated_target`.
- Identity-bearing misses fail closed. A live unmatched free-text query
  returned `status=not_found`, zero ranked profiles, and
  `profile_not_found`; it did not fall back to the configured browser build.
- Profile and site records now expose safe names, descriptions, aliases,
  origins, login IDs, account labels, tags, readiness, holder, route, source,
  and observation metadata used by the shared selector.
- The dashboard launcher consumes the server-ranked response and exposes the
  authoritative launch, add-tab, seed, view, and holder-inspection actions
  through access-plan and service-request contracts.
- A disposable detached no-CDP browser remained live as PID `132218` and
  appeared through installed CLI status and the authenticated dashboard API as
  `manual-runtime:p77-manual-observed:132218`. Both surfaces reported the same
  runtime profile, profile path, target URL, manual launch mode, absent
  DevTools port, `automationAvailable=false`, and safe next action.
- The dashboard workspace projection produced a dedicated
  `manual-runtime-browser` row whose controls do not advertise CDP-only
  automation. After the disposable browser exited, installed status reported
  no remaining row and the disposable runtime profile was moved to trash.
- Stale lease reconciliation expires missing-browser and overdue leases,
  preserves a concurrent renewal, and removes expired session IDs from browser
  authority. The installed convergence interlock finished with zero stale
  runtimes and authoritative listeners on the current executable.
- Route reconciliation released an orphaned Route B checkout without touching
  an independent browser window. A disposable real Route B open then reached
  display `:11`, `guacamole:5`, pool `guacamole-rdp-b`, and ready operator
  control before normal cleanup.
- Runtime publication replaced active daemons while retaining five active
  browser processes and their CDP endpoints. The final installed executable
  SHA was `c4aefab002b27dbccf7ebe5dd690815f17976703b5b58b0b8c7c928898d36a04`.

Validation:

- Rust format and clippy gates.
- Focused profile discovery, service status, stale lease, route-pool, runtime
  profile, service model, and service contract tests.
- Service generated-client and API/MCP parity gates.
- Dashboard workspace node, navigator, selected-context, launcher, production
  build, and docs production build gates.
- Local runtime convergence contract tests and repeated installed convergence.
- Live executable handoff, manual no-CDP inventory, exact and missing profile
  lookup, Route B open, install doctor, and remote-view doctor readbacks.

The source note remained unchanged.

## Delegation Receipt

Status: not_spawned

Reason: the active root instruction disallows subagents unless the user or an
applicable instruction explicitly requests them. The initial selector,
persisted-state, transport, generated-client, and dashboard work also share
tightly coupled schemas, so the primary agent owns integration.
