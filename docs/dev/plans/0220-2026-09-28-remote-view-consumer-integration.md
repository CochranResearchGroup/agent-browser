# Plan 0220 | Remote View Consumer Integration

Date: 2026-09-28

Plan version: 13

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P220

Predecessor: Agent Browser Issue #195 and the linked P219 supersession audit, cancelled as superseded while incomplete

Work item: [CochranResearchGroup/agent-browser#202](https://github.com/CochranResearchGroup/agent-browser/issues/202)

Provider evidence: closed [CochranResearchGroup/remote-view#70](https://github.com/CochranResearchGroup/remote-view/issues/70), J2/J3, and corrective Plan 0028

Target: `main`

Baseline: `origin/main` at `a3848e16`

Branch: `platform/p220-remote-view-consumer`

Owner: primary P220 implementation owner

Integration: merge through the protected pull-request workflow after J2/J3 consumer replay, retained-domain reconciliation, and changed-surface validation

Authority: the operator directed creation and execution preparation on 2026-09-28. This plan authorizes repository planning, provider-free fixtures, and ordinary source implementation after branch admission. Installed, privileged, public-ingress, production, merge, release, destructive cleanup, and Remote View runtime effects remain separately gated.

## Objective

Make Agent Browser a consumer of Remote View desktop and presentation
contracts in the trusted single-user WSL proof of concept. An ordinary request
selects or acquires a fixed Remote View desktop, launches or recovers the
addressed browser there, and returns only the durable opaque operator handoff.

Agent Browser continues to own browser profiles, browser processes, CDP,
logical sessions, tabs, targets, browser-to-desktop affinity, activity,
recovery, and final-session cleanup. Remote View owns desktop identity,
lifecycle, health, viewing routes, viewer admission, presentation, OS-level
input, diagnostics, and exact desktop cleanup.

## Current State

Remote View Plan 0017 is closed from corrected canonical evidence. J2
integrated at `7298ae7f76626c61ba5086eb4f934699251009ce`; J3 integrated at
`018d3d752f9d99805242f99c00034f0caabc53d1`. Its final provider-free Agent
Browser fixture proves two simultaneous desktop assignments, one external
browser association per desktop, independent desktop and mobile viewer
sessions, private-state exclusion, truthful reacquisition, and exact joined
cleanup. RV-014 and Remote View Issue #70 are closed.

The initial Plan 0017 closure was later reopened because its complete operation
matrix was reachable only through in-memory adapter-shaped fixtures and its
closure evidence lacked release and documentation validation plus a fresh
resource census. Corrective Plan 0028 added real production CLI,
authenticated HTTP, and MCP reachability through one `ControlRuntime`, repaired
the legacy capacity-routing regression found during canonical validation, and
reclosed Plan 0017 from source `b7caa1f209f2d64cd400b1766c50389ab4dc5cb3`
with governance evidence at
`eefddd2c862ce97bcf4387b2cc43dbbb2d9274eb`. Remote View `origin/main` at
`30f3e37` reconciles the closure narrative. This is authoritative provider
source and provider-free validation evidence, not Agent Browser installed
acceptance or authority for a live effect.

P220 source checkpoint `9a34e292` establishes the Agent Browser-owned F0
consumer boundary and distinct two-desktop browser/profile association
invariant. The current follow-up consumes the complete published foundation
observation, including command, resource states, and effect discipline. Remote
View J1 source checkpoint `f674518e34fea346002c72c4adc3966b628d0b78`
publishes lifecycle observations, durable operation status, exact generation
fencing, and release semantics. The now-published J2 and J3 contracts add
application registration, bounded pools, exact assignment and placement,
opaque viewing-route identity, viewer admission and layered live status,
generation-safe reacquisition, placement stop, and joined release that retires
the exact route and viewer sessions.

Retained-domain extraction is active through `ca3709e1`. The extracted
presentation-neutral spine owns named browser sessions and tabs, independent
durable SQLite persistence, one-time archived JSON migration, verified
rotating backups, restart reattachment, idle reaping, focus, and addressed
ordinary-command routing. It now also bounds disposable profile count and
regular-file bytes with oldest-first protected-session-aware eviction, rejects
pin or promotion bypass fields, and compacts exact URL history into
restart-safe daily identity summaries under a fixed byte ceiling. Desktop
selection consumes Remote View UUID, route
label, generation, and readiness observations instead of the legacy Agent
Browser route inventory. Real Remote View placement still fails closed at
`remote_view_application_placement_contract_unavailable`. Current Remote View
production entrypoints reach one `ControlRuntime`, but that runtime still uses
`ProviderFreeHost`, `ProviderFreeApplicationEffect`, and `ProviderFreeControl`.
The public record contract is complete; the installed application effect,
browser launch, and CDP adoption contract required by Agent Browser is not.
P220 therefore continues independent retained-domain work without pretending
that transport reachability supplies a usable runtime adapter.

Remote View's opaque `routeId` is a provider route identity, not by itself the
Agent Browser operator-facing `/remote-view/<handoff-id>` URL. P220 will keep
those identities distinct and add one Agent Browser handoff adapter over the
public route contract. It must not expose a route ID or provider URL as though
it were the durable operator handoff.

P220 checkpoint `552c8162` consumes the strict public J3 registration,
assignment, placement, viewing-route, viewer-session status, and joined-release
records in the provider-free service-model boundary. The source-bound fixture
proves two exact desktops, one ready placement and opaque route per desktop,
independent desktop/mobile viewer sessions, and release that retires the exact
route and both sessions. Unknown browser-private fields, partial retirement,
identity mismatch, and generation mismatch fail closed. This completes the
J3 wire-consumption tracer, not the CLI placement adapter or installed
acceptance.

Checkpoint `d7c43915` exposes the P220-owned SQLite backup boundary through
`service runtime-backup status|create` and the generic HTTP and MCP
`service_request` transport. Status verifies without mutation; create performs
one online SQLite backup, validates integrity and digest, and rotates at most
one previous copy. Neither surface restores data or includes the separate
Service State store. The generated client publishes typed status and manifest
responses. Disposable smoke cleanup now accommodates the intentionally
read-only migration archives.

Checkpoint `b47f7a0c` maps the validated Remote View public registration, pool,
assignment, placement, desktop generation, opaque route, and viewer-session
identities into an Agent Browser-owned retention record keyed by the exact
browser. The binding is idempotent across viewer-list ordering, survives a
SQLite restart, and rejects duplicate public identity across active browsers.
Release changes state only when the released assignment, desktop generation,
route, and complete viewer-session set match exactly. The retained record has
no provider URL, display number, credential, or provider-private state.
Follow-up `fd4264ec` makes bind and release single `BEGIN IMMEDIATE`
transactions over the complete Browser Session aggregate, preventing a future
runtime adapter from losing concurrent session or presentation updates.

Agent Browser Plan 0219 is cancelled as superseded while incomplete after its
authorized final cold-install attempt failed. Its branch is 36 commits ahead of
the published topic ref at audit start and contains both reusable Agent Browser
domain work and retired presentation-specific work. Two later governance
commits bring the preserved branch to 38 commits ahead. The successor branch was admitted from
`origin/main` at `a3848e16`; P219 custody remains preserved and is not merged
wholesale.

## Product boundary

Agent Browser owns:

- profile, process, CDP, logical-session, tab, and target state;
- selection of the browser-to-desktop association;
- navigation, browser recovery, and addressed-session cleanup; and
- browser readiness and CDP diagnostics.

Remote View owns:

- fixed or bounded desktop selection and allocation;
- desktop generation, readiness, presentation, and OS-level input;
- stable opaque viewing routes and viewer connectivity;
- generic allowlisted application-launch execution when used; and
- exact desktop release, cleanup, and presentation diagnostics.

Neither product imports the other product's private state authority. The
initial operating model is one trusted privileged user in one WSL instance.

## Consolidated batch

The first delivery batch contains four related outcomes:

1. Classify every unpublished P219 commit and changed surface as retain, adapt,
   retire, or evidence-only.
2. Freeze a provider-neutral Agent Browser presentation-client boundary against
   the current Remote View F0 identities and observations.
3. Prove two fixed desktops can carry distinct Agent Browser-owned browser
   associations without moving process, profile, CDP, or session authority.
4. Produce an integration candidate that removes dependency on the retired
   Agent Browser-owned XRDP/Guacamole installation path without activating a
   live Remote View runtime.

Dynamic capacity, iframe embedding, generalized IAM, cross-principal policy,
multi-tenant isolation, and a formal release are deferred.

## Delivery sequence and budget

### S0 — Custody and semantic audit

Inventory the 36 unpublished commits, changed files, schemas, tests, docs, and
runtime assumptions. Record exact retain, adapt, retire, and evidence-only
groups. Select an integration base without rewriting or deleting P219 custody.

Exit: every changed surface has one disposition and conflicting mixed commits
have an explicit extraction strategy.

### S1 — Provider-free consumer contract

Add an Agent Browser-owned Remote View client boundary and fixtures for public
identity, observation, desktop selection, opaque handoff, status, and release
shapes. Use no Remote View internal modules and perform no runtime effects.

Exit: the F0-shaped fixture proves two fixed desktop selections and distinct
browser associations while Agent Browser retains its domain authority.

Status: F0 and J1 complete at source checkpoint `8f753da0`; strict J3 public
wire consumption is complete at `552c8162`. Public identity,
foundation observation, lifecycle observation, durable operation status,
allocated fixed-desktop selection, distinct browser association, and exact
UUID-plus-generation release targeting are covered. J2 and J3 are now
canonical. Registration, assignment, ready placement, viewing-route,
viewer-session status, and joined-release records are consumed through the
existing boundary. The next packet must bind those records to the CLI runtime,
including placement-stop and truthful reacquisition behavior. Operator handoff
materialization remains Agent Browser-owned and must preserve the route-ID
versus handoff-URL distinction.

### S2 — Retained-domain integration

Integrate only the P219 browser/session/runtime changes selected by S0. Adapt
presentation calls behind the new client boundary. Exclude or remove the
retired XRDP user, Guacamole provider-rebuild, route-pool, and
presentation-helper path from the successor candidate.

Exit: changed-surface tests pass and architecture checks reject reintroduction
of the retired presentation ownership.

Status: partial. The initial session, durable store, and runtime spine plus
named-tab, reaping, focus, addressed-command, disposable-retention, and exact
URL compaction behaviors are extracted and locally qualified. The quota model
accepts protected session identities without importing the retired
viewer/controller tables. The J2/J3 join must now map exact public assignment,
placement, route, and viewer-session observations into Agent Browser's own
retention policy and fail closed on ambiguity. Checkpoint `ca3709e1` replaces
the supported JSON persistence path with SQLite WAL authority for the session
and profile aggregates. Migration stages the database atomically, archives
legacy inputs read-only, makes an existing database authoritative even when it
is corrupt, and provides integrity-checked current-plus-previous online backup
rotation. Checkpoint `64f75821` adds pure recovery admission and transition
rules plus a restart-safe SQLite recovery registry. Replacement requires exact
client or authenticated-viewer demand and proof that the old browser is
unusable; retry timing is bounded, and observed-live, failure, and success
transitions reject stale generations. Runtime launch and observation wiring,
operator-handoff integration, final-session cleanup, remaining P219 admission
and observability work, and protected integration remain. Checkpoint
`a2080250` adds the architecture guard
for this boundary. It rejects provider-private types in the pure model, local
display inference from Remote View identity, retired route-keeper authority in
the new CLI store, persistence dependencies in the service-model crate, J3
checkpoint drift, and loss of strict private-field or cleanup validation.
Checkpoint `d7c43915` completes operator backup status and creation across CLI,
HTTP, MCP, schema, and generated-client surfaces. Focused Rust backup and
contract tests, strict Clippy, formatting, client contract and type checks,
API/MCP parity, the no-launch contract smoke, P220 architecture guard,
documentation links, and the production documentation build pass. The broad
`test:service-client` umbrella remains blocked by a pre-existing stale P157
source-literal oracle that expects `ServiceRequestProvenance::capture` while
both the batch baseline and current source use
`capture_service_request_provenance`.
Checkpoint `b47f7a0c` completes the provider-neutral retention mapping for one
validated public presentation binding and its exact joined release. Runtime
ingress still needs to invoke that pure mutation after real Remote View
placement; it cannot do so until the installed application effect and
browser/CDP adoption contract exists.
The store adapter is transaction-ready at `fd4264ec`; transport ingress remains
intentionally absent rather than accepting caller-forged public identities.

### S3 — Checkpoint replay and final acceptance preparation

Replay the consumer fixture against Remote View J1, J2, and J3 contracts as
they become available. Extend it for generation-safe viewing, diagnostics,
many-to-many observation, and exact cleanup without importing Remote View
internals.

Exit: provider-free final acceptance is source-bound and the separately gated
installed acceptance packet has exact targets, stop rules, and rollback or
forward-recovery boundaries.

No installed acceptance begins merely because S0 through S3 source work passes.

Status: J1 replay complete against Remote View source checkpoint
`f674518e34fea346002c72c4adc3966b628d0b78`. The fixture rejects unversioned
shape drift and tampered operation payload evidence. The J3 Agent Browser wire
fixture now passes at `552c8162`, including strict private-state exclusion and
exact joined cleanup. Inspection of Remote View `origin/main@30f3e37` confirms
that the corrected CLI, HTTP, and MCP entrypoints still terminate in
provider-free host, application-effect, and control implementations. The
remaining runtime replay therefore depends on a real installed application
effect plus a browser/CDP adoption contract, not merely another Agent Browser
transport adapter. Remote View's provider-free fixture is compatibility
evidence, not a substitute for proving Agent Browser's browser launch,
recovery, durable handoff, retention mapping, or cleanup adapter.

## Worker assignments

One primary P220 owner controls the integration base, shared client contract,
and final reconciliation. Read-only commit classification and provider-free
fixture design are parallelizable after branch admission because they write
separate audit and test surfaces. Client-contract implementation, extraction
of mixed P219 commits, schema generation, and shared documentation remain on
the serialized critical path.

Remote View retains authority over its repository. Issue #70 and RV-014 are
closed from Remote View's J3 fixture; Agent Browser consumes the published J2,
J3, and corrective production-adapter checkpoints and does not edit Remote
View source as part of this plan.

## Evidence and exit

| Axis | Required evidence | Invalidation |
| --- | --- | --- |
| Custody | Exact P219 commit and file disposition with preserved branch and artifacts | Missing or silently discarded unpublished work |
| Boundary | Tests prove Agent Browser and Remote View retain the ownership split above | Either product becomes authoritative for the other's private state |
| Contract | Provider-free fixtures consume only published Remote View contracts | Importing Remote View internals or inventing a second control plane |
| Browser behavior | Two desktop associations retain distinct browser/process/profile/CDP identities and addressed sessions | Duplicate association, profile substitution, or Remote View browser ownership |
| Presentation | Only opaque handoffs leave the boundary; readiness and failures remain layered | Raw provider URLs, credentials, or fabricated viewer readiness |
| Cleanup | Release targets exact associations and ambiguity fails visibly | Cleanup by slot, PID, window title, or guessed ownership |
| Resources | Fresh process/resource census after separately authorized installed acceptance | Unexplained browser, desktop, daemon, listener, unit, or container residue |

## Acceptance criteria

- The P219 audit assigns every unpublished commit and changed file one durable
  disposition.
- Agent Browser uses a versioned provider-neutral consumer boundary for Remote
  View operations and observations.
- Two fixed desktops support distinct Agent Browser-owned browsers with no
  transfer of profile, process, CDP, session, or recovery authority.
- The same opaque handoff reconnects to the current desktop generation without
  exposing provider credentials or raw Guacamole routes.
- Diagnostics independently report desktop, browser/CDP, transport, viewer,
  and application readiness.
- Provider-free probe replays pass at applicable Remote View F0/J1/J2/J3
  checkpoints; each result is compatibility evidence at that checkpoint only.
- Exact release and cleanup leave no unexplained owned resources, while
  ambiguous ownership fails without broad deletion.
- Required Rust, generated-client, schema, documentation, architecture,
  planning, and lane gates pass for every touched surface.

## Non-goals

- Repairing or accepting the P219 Agent Browser-owned XRDP/Guacamole cold
  installation.
- Moving browser lifecycle or browser-private state into Remote View.
- Implementing Remote View dynamic capacity or embedding from Agent Browser.
- Adding generalized multi-user security to the present single-user proof of
  concept.
- Treating a provider-free test as installed, public, production, or release
  authority.

## Hard stops

- Keep the admitted successor worktree bound to the audited `origin/main`
  baseline and preserve the separate P219 custody branch.
- Do not merge the 36-commit P219 branch wholesale.
- Do not delete, rewrite, or clean the P219 branch, failed overlays, serial
  logs, or receipts without an explicit custody disposition.
- Do not repair the retired XRDP/Guacamole presentation path under this plan.
- Stop on ownership ambiguity, raw provider credential exposure, contract drift
  without an explicit checkpoint decision, or any required live effect without
  exact authority.

## Definition of done

Plan 0220 is complete when the selected presentation-neutral P219 work is
integrated through protected review; Agent Browser consumes Remote View public
contracts through one provider-neutral boundary; provider-free F0/J1/J2/J3
evidence and final installed acceptance are source-bound and truthful; the
Alice/Bob browser/session behavior survives the presentation replacement; the
retired XRDP/Guacamole ownership does not remain on the supported path; exact
cleanup and a fresh resource census pass; and roadmap, runbook, plan, work item,
lane, Git, validation, and installed identities agree.
