# Plan 0137 | Profile Acquisition Recovery And Lifecycle Reliability

Date: 2026-08-28

State: OPEN

Execution state: `slice_h_complete_slice_i_not_started`

Lane: P137

Source baseline: `e636a2501165ad33c8b4223b84005abfba0387d7`

Branch: `main`

Target: `main`

Integration model: direct to `main` through cohesive, validated checkpoints.
No pull request is required for this one-maintainer repository. Candidate
runtime work remains isolated until its guarded installation gate.

Authority: PLAN, SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, AND ISOLATED
DEVELOPMENT-RUNTIME VALIDATION ARE IN SCOPE. THIS PLAN DOES NOT BY ITSELF
AUTHORIZE PRODUCTION STATE REPAIR, PROFILE OR PROVIDER USE, ROUTE CREATION,
PROCESS TERMINATION, OR CANDIDATE INSTALLATION. EACH LIVE EFFECT REQUIRES ITS
EXACT EXECUTION GATE.

Depends on:

- Plan 0130 owner reuse coherence;
- Plan 0132 terminal-owner supersession route coherence;
- closed Plan 0134 principal, lease, crash, migration, and installation work;
- `docs/dev/notes/0135-2026-08-28-cdp-free-google-dashboard-seeding-gap-handoff.md`;
- `docs/dev/notes/0136-2026-08-28-last30days-terminal-replacement-launch-handoff.md`;
- Plan 0124 presentation capacity and desktop evidence;
- Plan 0131 controlled X11 provider behavior; and
- Plan 0133 process-bound operator-visible proof.

## Executive Decision

Agent Browser will expose profile acquisition as the stable client contract and
make lease, owner, session, process, route, and presentation records internal
evidence used to satisfy that contract.

A client asking for its own profile must receive exactly one of three outcomes:

1. an acquired or safely reused browser lane;
2. one exact Recovery Plan that the client or operator can inspect and apply;
   or
3. a hard blocker backed by current proof of a genuinely live competing
   authority or an unavailable external dependency.

An expanding list of internal reason codes is not an acceptable fourth
outcome. Safe lifecycle defects become productized Mitigation Actions. Clients
must not wait for a source hotfix and workstation upgrade each time a new
recoverable state shape appears.

The campaign therefore ships the recovery plane early, uses the current
Last30days terminal-owner defect as its first vertical recovery case, repairs
the state-authority defect that created fictitious dashboard browsers, adds
route-bound CDP-free manual profile seeding, and only then performs the next
production candidate installation.

## Goal

Deliver a first-class, transactional profile acquisition and recovery system
that lets authenticated services safely use their own profiles despite stale,
terminal, migrated, processless, or internally inconsistent lifecycle state.

The system must:

- distinguish durable principal, profile, browser, daemon route, service
  session, process, presentation route, and lifecycle-owner identities;
- explain one Dominant Blocker without hiding secondary evidence;
- produce a sealed Recovery Plan whenever current evidence supports bounded
  repair;
- apply safe plans with compare-and-swap guards, idempotency, compensation, and
  a durable Recovery Receipt;
- automatically retry the original Profile Acquisition Intent after a
  successful repair;
- refuse generic force-unlock behavior and preserve any current foreign or
  ambiguous live authority;
- make status and projection reads incapable of creating durable entities;
- make inert and provenance-free legacy records diagnosable and exactly
  retireable;
- support a CDP-free, route-bound, dashboard-visible manual-seeding workflow;
  and
- survive source-free installation, migration, mixed-version rollback, and
  future additions to the Mitigation Action registry without another schema
  emergency.

## Non-Goals

- This plan does not weaken principal authentication or profile capability
  checks.
- It does not permit copying, renaming, clearing, or deleting authenticated
  profile directories as a recovery shortcut.
- It does not infer authentication from a running process, visible desktop, or
  ready route.
- It does not turn raw Guacamole addresses into operator handoffs.
- It does not automate real Google, FedEx, X, LinkedIn, or other provider
  interactions during source or provider-free acceptance.
- It does not consolidate every Service subsystem behind one new monolith.
- It does not treat every diagnostic-retained display allocation as garbage.
- It does not reopen the accepted Plan 0134 installation transaction.

## Current Control Record

At planning time:

- `HEAD` and `origin/main` are both
  `e636a2501165ad33c8b4223b84005abfba0387d7`;
- production runs `agent-browser 0.28.0`, generation
  `0.28.0-2851117fd877-04e7cf4c8b54`, binary SHA-256
  `2851117fd8778d18ef05cadfb999a2bed82ed16e7d56206188f5bd753467f9c9`;
- runtime multiplicity is steady with one dashboard, one runtime host, one
  executable generation, and zero legacy daemons;
- the installed workstation payload is source-free and ready;
- the installed doctor still reports legacy principal, owner-generation, and
  owner-binding lease warnings, including `last30days-facebook` and SoyLei
  profiles;
- the two untracked notes 0135 and 0136 are preserved as other-agent handoffs
  and become source authorities for this plan; and
- no live state was mutated while preparing this plan.

## Incident Inventory

### Last30days cannot replace a terminal owner

The exact profile has no active lease, live compatible browser, profile
process, or unsatisfied cleanup obligation. Its generation-55 lifecycle owner
is terminal, yet acquisition returns
`terminal_replacement_route_inconsistent`.

The retained owner preserves durable browser id
`session:last30days-facebook--last30days-facebook` while its current daemon
route is `handoff-a79ef2887412addf`. Current replacement logic assumes the
browser id must equal `session:{daemonSessionRoute}`. Cooperative transfer
intentionally changes the daemon route without changing durable browser
identity, so that equality is not an identity invariant.

### SoyLei and other clients cannot use their own profiles

Observed blockers include:

- `existing_session_profile_identity_inconsistent`;
- `existing_session_profile_identity_unproven`;
- `legacy_principal_unproven`;
- `runtime_owner_principal_binding_missing`; and
- `owner_generation_or_binding_mismatch`.

Existing lease commands can inspect, explain, rejoin, renew, release, and
reconcile some records. They do not coordinate the whole acquisition graph or
resume the client request. A client can therefore own the profile yet have no
effect-capable recourse.

### Odollo fulfillment cannot use its own carrier profile

Odollo fulfillment is blocked before its own FedEx tracking-number lookup
service can acquire its profile. Provider behavior is not the current failure
axis. The acceptance case must prove profile acquisition independently from
FedEx navigation or data retrieval.

The Odollo contractor-portal test profile also reproduces
`existing_session_profile_identity_unproven`: retained session evidence exists,
but the current runtime-owner binding cannot prove the exact profile identity.
This remains a distinct principal/profile reconciliation case and must not be
treated as terminal-owner supersession.

### Dashboard contains fictitious browsers with no valid action

The installed Service State contains processless records `browser-cdp` and
`session:odollo-carrier-ups`. They report ready-like health and reattachability
while browser-session authority correctly reports
`live_browser_missing_pid`. Dashboard heuristics classify them as live and
offer Close, but `service_browser_close` can only close the active daemon
browser. Existing prune and GC dry runs find no candidate.

The records match a historical test fixture. A status path accepts
caller-supplied Service State, reconciles it, and can persist missing browser
records. This means a nominal read can create durable production entities from
synthetic input. It is a state-authority defect, not merely a dashboard-label
bug.

### CDP-free manual seeding is not route-bound

Initial Google authentication correctly requires headed stock Chrome without
DevTools. Current CDP-free launch can create the process, but it does not
reserve a presentation route, produce process-bound visibility proof, or
return a durable `/remote-view/<handoff-id>`. A browser placed on a display
that happens to belong to a route is still not route-owned or dashboard
visible.

## Ubiquitous Language And Identity Boundaries

The glossary in `CONTEXT.md` is authoritative for Profile Acquisition Intent,
Dominant Blocker, Recovery Plan, Recovery Receipt, and Mitigation Action.

| Identity | Stable meaning | May change independently from |
| --- | --- | --- |
| service principal | authenticated service authority | task, tab, session, daemon route |
| managed profile | persistent browser user-data identity | principal lease state, browser process |
| durable browser | logical retained browser lane | process instance, daemon route, presentation route |
| runtime lane | serialized daemon command scope | durable browser id, service session id |
| service session | client work and handle grouping | daemon route, process, presentation |
| process instance | boot-scoped executable identity | durable browser and profile identity |
| lifecycle owner | generation-bound ownership record | current process and daemon route after transfer |
| presentation route | operator-visible route, display, and viewer binding | browser lifecycle and CDP attachment |
| acquisition lease | exclusive in-flight claim | long-lived profile ownership |

No implementation may infer equality between two identities merely because
their current strings share a prefix or were historically derived from each
other. Joins require explicit typed fields plus generation and provenance.

## Frozen Invariants

1. One authenticated principal may acquire its own exact profile unless a
   current foreign authority or unsafe ambiguity is proven.
2. A durable browser id and daemon session route are separate identities.
3. A terminal lifecycle with satisfied cleanup and absent process evidence is
   not live competition.
4. A status, explain, plan, doctor, dashboard refresh, or other read must not
   create, adopt, delete, or reattribute durable state.
5. Caller-supplied Service State is projection input only. It has no durable
   effect authority.
6. A browser is live or reattachable only with current process or managed
   runtime proof. A health label or view-stream record is insufficient.
7. Recovery applies only a sealed plan whose state revision, profile digest,
   principal authority, generations, and observed process evidence still
   match.
8. A stale Recovery Plan fails before effect and returns a fresh plan or one
   new Dominant Blocker.
9. Successful recovery emits a durable receipt before acquisition retry is
   reported successful.
10. Failed compensation creates a retained cleanup obligation and blocks an
    equivalent retry. It never becomes permission for broad cleanup.
11. Route cleanup is independent from browser cleanup. Releasing one does not
    imply authority over the other.
12. A CDP-free browser may be operator-visible and controllable without being
    automation-attachable.
13. Manual-seeding completion requires the exact browser process to close and
    a later authenticated-state probe. Visibility is not authentication.
14. All operator handoffs use only durable opaque remote-view URLs.
15. Migration preserves unclassified legacy evidence until an exact reviewed
    action proves retirement safe.
16. No migration silently performs mass deletion or rewrites another tenant's
    profile, principal, browser, route, or lease.

## Public Acquisition Outcome

The public response shape is conceptually:

```json
{
  "state": "acquired | recovery_available | blocked",
  "dominantBlocker": null,
  "automatic": false,
  "browser": null,
  "recoveryId": null,
  "nextAction": null,
  "evidence": []
}
```

`evidence` retains every relevant internal reason, but clients branch only on
the stable top-level state and Dominant Blocker. New internal diagnostics must
not require clients to learn a new workflow.

Initial public surfaces, subject to contract review in Slice A, are:

- `service profiles <profile-id> acquire`;
- `service recovery plan`;
- `service recovery apply`;
- `service recovery status <recovery-id>`;
- HTTP and MCP parity;
- generated TypeScript client helpers; and
- dashboard Acquire, Review Recovery, Apply Recovery, Retry, and Retire Record
  actions appropriate to the selected record's lifecycle class.

## Recovery Plan Contract

Every plan binds:

- schema version, plan id, recovery id, creation time, expiry, and idempotency
  key;
- authenticated principal and exact profile identity digest;
- Service State revision and every affected record id;
- lifecycle-owner generation and acquisition-lease revision;
- durable browser, daemon route, service session, process, lock, display,
  presentation-route, and handoff evidence when applicable;
- the Dominant Blocker and supporting evidence;
- ordered Mitigation Actions and their effect authority;
- expected postconditions;
- compensation actions and retained cleanup obligations;
- original Profile Acquisition Intent; and
- an integrity seal over all effect-bearing fields.

The apply response and durable receipt record:

- precondition comparison;
- each attempted effect and stable operation id;
- each receipt-persistence boundary;
- compensation and quarantine outcome;
- final state revision and affected identities;
- terminal result; and
- automatic acquisition retry result.

Plan generation is always zero-effect. Apply is compare-and-swap guarded,
idempotent, and resumable after process interruption or receipt-persistence
failure.

## Mitigation Action Registry

The first registry must support these bounded actions:

| Defect shape | Safe action when evidence is conclusive | Reviewed action when evidence is incomplete |
| --- | --- | --- |
| terminal owner, cleanup satisfied, process absent | supersede exact terminal owner | inspect missing identity axis |
| durable browser and daemon route differ after valid transfer | rebind replacement command route without rewriting browser id | inspect lineage or generation conflict |
| same principal has legacy unproven binding | reconcile exact principal capability | preserve observation-only state |
| expired or ownerless lease | release exact lease | wait for lease or prove owner absence |
| live process and profile agree but state is detached | adopt exact browser | preserve foreign or ambiguous process |
| PID-less unreferenced browser record | retire exact inert record | quarantine provenance-free legacy record |
| synthetic fixture-shaped durable record | retire by exact provenance migration | require operator review if provenance is absent |
| stale route independent of live browser | repair or release exact route | preserve route with current viewer or handoff |
| browser, session, and profile binding mismatch | repair the exact subordinate binding | block if two live authorities remain possible |
| installation transaction is terminal | clear only terminal admission bookkeeping | preserve active transfer or rollback obligations |

The registry is extensible data and typed behavior, not a sequence of hardcoded
client special cases. Adding a new safe action may require a software update;
encountering an already modeled state must not.

## Execution Slices

### Slice A | Freeze acquisition and recovery contracts

Objective: establish the shared model before more local lifecycle predicates
accumulate.

Deliverables:

- redacted provider-free fixtures for every incident and blocker in this plan;
- explicit typed identity joins for principal, profile, browser, daemon route,
  session, process, lifecycle owner, and presentation route;
- versioned acquisition outcome, Recovery Plan, Recovery Receipt, and
  Mitigation Action schemas;
- a source authority map identifying which module may propose, apply, persist,
  project, and render each state transition; and
- no-launch tests proving plan generation has zero effects.

Acceptance:

- the Last30days generation-55 fixture reproduces before repair;
- SoyLei inconsistent and unproven identity fixtures remain distinct;
- Odollo profile acquisition is represented without provider data;
- the two fictitious browser records are represented with redacted fixture
  provenance; and
- each fixture has one expected Dominant Blocker and one recovery class.

### Slice B | Ship the recovery core and first vertical repair

Objective: make the first useful mitigation available before the full campaign
finishes.

Deliverables:

- zero-effect recovery planning;
- sealed compare-and-swap apply;
- idempotent effects, compensation, quarantine, and durable receipts;
- recovery status and interrupted-operation resume;
- terminal-owner supersession as the first action; and
- a corrected Last30days predicate that preserves separate durable-browser and
  daemon-route identities across cooperative transfer.

Acceptance:

- terminal, cleanup-satisfied, process-absent generation 55 yields a recovery
  plan rather than `terminal_replacement_route_inconsistent`;
- changed generation, state revision, process, profile lock, or foreign lease
  makes the plan stale before effect;
- apply supersedes only the exact owner, retries acquisition, and returns the
  new browser and daemon route without rewriting durable history; and
- current live-owner controls remain blocked.

### Slice C | Add the high-level acquisition coordinator

Objective: remove lifecycle choreography from service clients.

Deliverables:

- one Profile Acquisition Intent entry point;
- normalized acquired, recovery-available, and blocked outcomes;
- one Dominant Blocker with secondary evidence;
- automatic planning and safe auto-apply policy for conclusive zero-ambiguity
  actions;
- reviewed apply for bounded ambiguity; and
- automatic retry after recovery success.

Acceptance:

- clients do not choose a daemon route or replacement session to get their own
  profile;
- a modeled recoverable defect never ends as an unexplained action-less block;
- automatic recovery is limited to actions whose evidence is complete; and
- a live foreign principal remains a hard blocker with wait or coordination
  recourse.

### Slice D | Repair Service State provenance and fictitious records

Objective: stop reads from creating durable entities and make legacy inert
records safely manageable.

Deliverables:

- strict separation between durable repository input and caller-supplied
  projection input;
- read-only status reconciliation for supplied Service State;
- provenance, creation time, last-observed time, and authority source for
  browser records;
- live and reattachable classification that requires current process or
  managed runtime evidence;
- exact retirement and quarantine actions for PID-less, unreferenced, and
  fixture-shaped records;
- a one-time installed contamination detector and migration plan; and
- an audit test proving a live runtime home is never written by source tests,
  regardless of build-time or process `HOME`.

Acceptance:

- repeated status reads cannot change durable state;
- synthetic caller records remain projection-only;
- `browser-cdp` and `session:odollo-carrier-ups` classify as inert legacy
  evidence, not live or reattachable browsers;
- dashboard offers Review or Retire instead of Close for inert records;
- `service_browser_close` remains scoped to a proven active service browser;
  and
- retirement requires exact ids, revision, evidence digest, and a receipt.

The 36 diagnostic-retained display allocations receive an audit and
classification report. They are not deleted merely because browser fixtures
were discovered.

### Slice E | Expand the mitigation registry

Objective: cover recurring client-blocking lifecycle states without local
one-off commands.

Deliverables:

- exact actions for legacy principal reconciliation, missing owner-principal
  binding, owner-generation mismatch, expired or ownerless leases, exact live
  browser adoption, subordinate browser-session-profile mismatch, independent
  route repair, and terminal installation bookkeeping;
- action-specific preconditions and compensation;
- dominance rules when several blockers coexist; and
- action discovery so clients can render new server capabilities without a
  client release.

Acceptance:

- every blocker named in this plan is acquired, recovery-available, or hard
  blocked by concrete live evidence;
- no action widens from one exact profile graph to broad garbage collection;
  and
- recovery of one tenant or profile cannot alter another.

### Slice F | Deliver route-bound CDP-free manual seeding

Objective: make safe first authentication visible and operable without
DevTools.

Deliverables:

- transactional managed-profile selection or registration;
- route capacity reservation before browser launch;
- headed stock Chrome launch without CDP on the reserved display;
- process-bound desktop visibility proof;
- one durable opaque remote-view handoff;
- explicit `manual_seeding` lifecycle state that denies CDP actions;
- exact close, route release, viewer/controller/display release, profile-lease
  transition, and compensation; and
- post-close attachable relaunch plus separate authenticated-state probe.

Acceptance:

- route exhaustion fails before Chrome launch;
- wrong process, absent window, stale route, missing display socket, or
  unavailable Guacamole returns typed not-visible evidence;
- `operatorVisible.state=ready` is required before visibility is claimed;
- closing the exact manual browser releases only its owned resources;
- an interrupted launch or close resumes without duplicate Chrome or route
  checkout; and
- no test or product response equates visibility with Google authentication.

### Slice G | Complete first-class surfaces and documentation

Objective: make recovery usable without source knowledge.

Deliverables:

- CLI help, README, shared skill, inline documentation, and docs-site updates;
- service contract metadata, JSON schemas, HTTP, MCP, generated client, and
  JavaScript type parity;
- dashboard incident and selected-record actions that remain distinct;
- confirmation dialogs through shadcn/ui, never native browser dialogs;
- redacted plan preview, affected identities, risk, compensation, expiry, and
  receipt presentation; and
- concise client guidance that leads with the next action rather than internal
  reason-code history.

Acceptance:

- every public surface expresses the same action names, authority, and result
  states;
- private capabilities never persist in Service State, logs, dashboard
  storage, plan files, or receipts; and
- older clients receive a stable blocked or recovery-available response rather
  than silently applying an unknown action.

### Slice H | Build migration and installation compatibility

Objective: ensure the repair can be installed without amplifying damaged
state.

Deliverables:

- source-free migration discovery and dry run;
- exact before-and-after summary by record class and affected id;
- backup locator, restore procedure, and migration receipt;
- forward, backward, and mixed-version readers for plans, receipts, browser
  provenance, and lifecycle identity fields;
- preservation of unknown records and action types;
- candidate-led contamination report before any apply;
- transactional runtime-owner transfer, ingress, rollback, and selected
  generation commit through the existing workstation installer; and
- installed skill and dashboard synchronization.

Acceptance:

- dry run classifies current production state without mutation;
- no default migration deletes browsers, profiles, leases, displays, routes,
  or handoffs;
- old generation rollback can read terminal bookkeeping and preserve unknown
  successor fields;
- a candidate failure restores the selected old generation and leaves one
  exact recovery obligation or none;
- rerun after interruption converges without duplicate effects; and
- no upgrade is required merely to apply a recovery action already advertised
  by the installed action registry.

### Slice I | Provider-free and isolated development acceptance

Objective: prove the complete contract before production mutation.

Required proof:

- focused Rust unit and contract tests at the cheapest stable seams;
- service request action, schema, HTTP, MCP, generated-client, and dashboard
  parity tests;
- source-free workstation packaging and migration fixtures;
- strict formatting and Clippy through `scripts/ci/cargo-safe.sh`;
- validation selection from the last known green baseline;
- three disposable development browser launch and residue iterations;
- one isolated route-bound CDP-free manual-seeding acceptance with no real
  credential;
- separate process, profile, acquisition, presentation, route-cleanup, and
  authenticated-readiness proof axes; and
- a fresh OS process and resource census after development acceptance.

### Slice J | Guarded production candidate and consumer acceptance

Objective: install one candidate and prove real clients can acquire their own
profiles.

Preconditions:

- Slices A through I are closed with durable receipts;
- the exact candidate commit, binary SHA-256, generation, dashboard assets,
  support payload, and migration digest are frozen;
- candidate dry run classifies current production state and proposed changes;
- backup and rollback evidence are current;
- any forced shutdown or route mutation has separate exact authority;
- no active foreign profile, browser, viewer, controller, route, or install
  transaction would be displaced; and
- one fresh presentation route and durable handoff are ready for candidate
  validation.

Installation sequence:

1. capture current selected generation, Service State revision, process census,
   lease doctor, route inventory, durable handoff proof, and backup receipt;
2. run candidate migration and install dry run;
3. review exact effects and hard stops;
4. apply through the transactional workstation installer;
5. validate candidate dashboard, runtime host, ingress, durable handoff, and
   source-free payload before commit;
6. commit the selected generation only after every required axis is ready;
7. reconcile and prove idempotent reapply; and
8. run final doctor, lease doctor, recovery inventory, dashboard review, and OS
   process/resource census.

Consumer acceptance is bounded and separates browser acquisition from provider
effects:

- Last30days: acquire or recover `last30days-facebook` and reach a harmless
  `about:blank` tab without X or LinkedIn navigation;
- Odollo fulfillment: acquire its own carrier profile and prove the tracking
  lookup lane can be created without opening FedEx or submitting a tracking
  number;
- SoyLei: recover one profile that previously returned an existing-session or
  owner-binding inconsistency and prove exact reuse;
- fictitious records: preview and apply exact retirement only after operator
  review of the candidate plan;
- manual seeding: use one disposable profile and nonproduction target, return
  a durable handoff, prove CDP absent, close exactly, and separately probe
  readiness; and
- foreign-principal control: prove another principal cannot acquire or repair
  a live owned profile.

Real provider navigation, credentials, consent, extraction, tracking lookup,
or downstream scheduling remain separate consumer authority.

## Acceptance Matrix

| Case | Required terminal outcome |
| --- | --- |
| `terminal_replacement_route_inconsistent` | recovery plan supersedes exact terminal owner and retries acquisition |
| `existing_session_profile_identity_inconsistent` | exact binding repair or one concrete live-authority block |
| `existing_session_profile_identity_unproven` | principal reconciliation plan without duplicate launch |
| `legacy_principal_unproven` | capability-bound reviewed reconciliation |
| missing owner-principal binding | exact rejoin or recovery plan |
| owner-generation mismatch | compare-and-swap repair or stale-plan no-effect |
| `live_browser_missing_pid` | inert classification and exact retirement recourse |
| fixture-shaped browser record | provenance report and reviewed retirement receipt |
| expired ownerless lease | exact release and automatic acquisition retry |
| same-profile live foreign authority | hard block with current process and principal proof |
| CDP-free manual seeding | durable handoff, ready visibility, no CDP, exact cleanup |
| route failure during seeding | compensation or quarantine with cleanup obligation |
| terminal installation transaction | admission resumes without broad state rewrite |
| stale recovery plan | zero effect and refreshed plan or blocker |
| repeated recovery apply | same terminal receipt and no duplicate effects |
| migration rollback | old generation restored with readable terminal ledger |

## Validation Contract

Each slice names the invariant before adding tests and uses the cheapest seam
that proves it. The full campaign records whether each run is focused,
presubmit, comprehensive, or live. Retries preserve the first failure.

At minimum, touched Rust surfaces require:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
```

Focused tests are selected by the changed authority. Before any push or
installation checkpoint, run:

```bash
pnpm validation:select -- --base <last-known-green-ref>
```

Any service action or schema change must keep Rust contract metadata, JSON
schema, MCP, HTTP, generated client, JavaScript types, dashboard, help, README,
shared skill, docs site, and inline comments aligned.

## Rollback And Recovery

- Source checkpoints are cohesive and independently revertible where their
  schema compatibility permits.
- Runtime migrations are forward-only only after both generations can read
  their terminal bookkeeping. Unknown fields are preserved.
- Production apply captures a recoverable backup and exact selected-generation
  locator before effects.
- Failed precommit acceptance restores the old selected generation, ingress,
  runtime owner, and compatible state projection.
- Any external effect whose compensation cannot be proven becomes one exact
  quarantined cleanup obligation.
- Rollback never deletes a profile directory or fabricates old process, route,
  browser, or lease evidence.
- A rollback receipt states what changed, what was restored, what remains, and
  the only safe next action.

## Hard Stops

- Do not bypass principal or profile capability checks.
- Do not add a generic force unlock, force close, or delete-all action.
- Do not edit Service State or lease files by hand.
- Do not copy, rename, clear, or delete an authenticated profile to escape a
  lifecycle blocker.
- Do not treat absent Service State entries as authority to kill a process.
- Do not let status or dashboard refresh persist caller-supplied entities.
- Do not treat health labels, streams, or URLs as live process proof.
- Do not publish or persist raw Guacamole or provider route URLs.
- Do not attach CDP during initial Google manual seeding.
- Do not infer authentication from visibility or process existence.
- Do not navigate real provider pages during provider-free acceptance.
- Do not mutate production, install a candidate, create a route, or terminate a
  process without the exact execution gate.
- Do not retry an uncertain external effect until its prior state is
  reconciled.
- Do not classify all retained displays or legacy records as garbage from one
  fixture match.

## Execution Bounds And Replanning

- One critical source lane is active at a time. Parallel agents are not part of
  this plan unless separately authorized with disjoint write scopes.
- Checkpoint each completed slice, or at least every 90 minutes during a long
  slice, with objective, evidence, changed identities, remaining gate, and
  rollback state.
- Use at most two implementation attempts at one failing seam before recording
  the evidence and locally reframing the slice.
- Use at most one broad drift-discovery review for the whole campaign when
  current evidence justifies it. Any later review is closed-world against
  accepted blocking findings and critical regressions.
- Use one consolidated source-review candidate set and one bounded remediation
  pass. Nonblocking findings enter backlog without expanding this campaign.
- Use one isolated development provider acceptance for CDP-free visibility and
  cleanup.
- Use one guarded production installation attempt. A single retry is allowed
  only after the first transaction is terminal, all effects are reconciled,
  and the exact failed gate has been repaired.
- When a safe in-scope action remains, continue by default. Stop only at a hard
  gate, exhausted bound, unresolved authority conflict, or unsafe ambiguity.

## Material Checkpoint Fields

Every checkpoint records:

- slice and objective;
- source commit and worktree status;
- changed contract and affected identities;
- validation tier, exact commands, result, retries, and exclusions;
- current Service State revision or fixture revision;
- runtime environment and candidate identity when applicable;
- effects, compensation, and cleanup obligations;
- independent-review disposition when used;
- next bounded action; and
- exact remaining approval or authority gate.

## First Execution Packet

Execute only Slices A and the smallest vertical portion of Slice B:

1. freeze the redacted incident fixtures and identity joins;
2. define versioned acquisition outcome, Recovery Plan, Recovery Receipt, and
   Mitigation Action schemas;
3. implement zero-effect plan generation for the Last30days generation-55
   terminal-owner fixture;
4. implement exact terminal-owner supersession with stale-plan and live-owner
   controls;
5. automatically retry provider-free acquisition after a successful apply;
6. expose the minimal CLI, service contract, HTTP, MCP, and generated-client
   parity needed for that vertical path;
7. update all required user-facing documentation; and
8. stop before development publication, production state repair, candidate
   installation, route creation, provider navigation, or process termination.

The first checkpoint is accepted when the real failure shape is protected at a
stable seam, terminal replacement succeeds only under exact evidence, and all
live or stale controls fail before effect. Slice C begins only after that
checkpoint is durable and the plan's identity model remains coherent.

## First Packet Checkpoint | 2026-08-28

- Objective: freeze Slice A contracts and incidents, then ship only the exact
  terminal-owner Slice B vertical.
- Source commits: `5eaca482`, `e14c9bbc`, `1752bfd1`, `3cabd593`, and
  `7c1331c2` on `main`, pushed to `origin/main`.
- Contract and identities: versioned acquisition outcome, Recovery Plan,
  Recovery Receipt, and Mitigation Action contracts bind principal, profile,
  profile digest, lifecycle owner generation, durable browser, daemon route,
  service session, process digest, and optional presentation route. Durable
  browser id and daemon route remain separate identities.
- Fixture revision: Plan 0137 provider-free fixture set at `7c1331c2`, covering
  Last30days generation 55, Odollo contractor portal identity uncertainty,
  SoyLei identity and principal blockers, both fictitious PID-less records,
  and the CDP-free route-proof gap.
- Effects: source and provider-free fixture changes only. No development or
  production publication, profile use, provider navigation, route creation,
  process termination, candidate installation, or live Service State repair.
- Validation: Rust formatting and Clippy passed; focused Plan 0137 recovery,
  command, contract, and fixture tests passed; all fixture JSON parsed; service
  API/MCP parity, generated-client contract, JavaScript types and exports,
  route-confusion gates, and the production docs build passed.
- Retry record: one compile correction changed fixture test keys from borrowed
  strings to owned strings. The next run passed. No effect retry occurred.
- Rollback: each source checkpoint is a cohesive Git commit. No runtime state
  or cleanup obligation exists.
- Independent review: not used for this single critical source lane.
- Remaining gate: Slice C is not started. Full interrupted-operation recovery,
  additional mitigation actions, state-provenance repair, dashboard actions,
  migration, installation compatibility, isolated development acceptance, and
  every production gate remain open under their later slices.

## Slice C Checkpoint | 2026-08-28

- Objective: make authenticated profile acquisition the stable client entry
  point and remove route and replacement-session choreography from callers.
- Source commit: `69d75aa1` on `main`, pushed to `origin/main`.
- Contract: `service recovery acquire`, HTTP
  `POST /api/service/profiles/acquire`, MCP `service_profile_acquire`, and
  generated helper `acquireServiceProfile()` return exactly `acquired`,
  `recovery_available`, or `blocked`. The acquisition request rejects a
  client-supplied daemon route.
- Decision behavior: an exact process-backed lane with a current matching
  principal binding is reused; current foreign-principal authority is a hard
  blocker with wait or coordination recourse; missing or inconsistent binding
  produces a reviewed `reconcile_exact_principal_profile_identity` plan; and
  only conclusive terminal-owner recovery auto-applies before one retry.
- Validation: Rust formatting and Clippy passed; 11 focused recovery and parser
  tests passed; service contract metadata, API and MCP parity, route-confusion
  gates, service collection no-launch parity, the complete generated-client
  suite, and the docs production build passed.
- Exclusions: provider-backed and CDP streaming live tests were not run because
  Slice C requires no provider or browser effect. The repository skill changed
  as required, while the shared user-scoped installed skill was intentionally
  not overwritten during source validation.
- Effects and rollback: source changes only. No development or production
  runtime, profile, route, process, provider, or installation effect occurred.
  Reverting `69d75aa1` restores the prior public surface.
- Remaining gate: Slice D provenance repair, exact inert-record retirement,
  the remaining mitigation registry, CDP-free manual seeding, dashboard and
  migration completion, isolated development acceptance, and every production
  gate remain open.

## Slice D Checkpoint | 2026-08-28

- Objective: make status projection read-only, distinguish provenance from
  liveness authority, and give inert legacy browser rows exact retirement
  recourse.
- Source commit: `7fd03776` on `main`, pushed to `origin/main`.
- Authority split: caller-supplied `serviceState` is projection-only. A status
  request without supplied state loads the durable repository snapshot. Neither
  path persists reconciliation results during the read.
- Browser projection: every returned browser row identifies source, authority
  source, creation and observation evidence when known, lifecycle
  classification, recommended action, record revision, and evidence digest.
  Only process-identity or managed-runtime evidence can classify a row as live
  or reattachable. PID-less unreferenced rows classify as `inert_legacy` with
  `retire`; ambiguous rows remain `review_required` with `review`.
- Retirement and contamination: exact browser retirement binds browser id,
  record revision, evidence digest, expiry, and a terminal receipt. It rejects
  any new process, managed lifecycle, session, display, or view-stream
  authority. The no-effect contamination detector identifies `browser-cdp`
  and `session:odollo-carrier-ups` while reporting, and preserving, all 36
  diagnostic display allocations.
- Test isolation: test builds always redirect the default Service State path to
  a process-scoped temporary home unless the explicit test escape hatch is set,
  regardless of build-time or process `HOME`.
- Validation: formatting and Clippy passed; focused read-only projection,
  contamination, exact retirement, replay, live-authority rejection, and test
  home interlock tests passed; generated client contract and JavaScript type
  checks passed.
- Effects and rollback: source and provider-free fixture behavior only. No live
  Service State, browser, display, route, profile, or process was changed.
  Reverting `7fd03776` restores the prior status and browser schema behavior.
- Remaining gate: Slice E must connect the remaining lifecycle blocker classes
  to exact mitigation actions. Public retirement surfaces and dashboard actions
  remain assigned to Slice G, and migration execution remains assigned to
  Slice H.

## Slice E Checkpoint | 2026-08-28

- Objective: make recurring lifecycle blockers discoverable as server-owned,
  exact-profile mitigation actions instead of client-local repair recipes.
- Source commit: `e5fc37fd` on `main`, pushed to `origin/main`.
- Registry: mitigation metadata now covers exact principal reconciliation,
  missing owner-principal binding, owner-generation repair, expired ownerless
  lease release, exact live-browser adoption, subordinate binding repair,
  route-bound manual-seeding acquisition, terminal installation bookkeeping,
  terminal-owner supersession, and exact inert-record retirement.
- Action contract: every descriptor names its server-owned executor, automatic
  or reviewed apply posture, exact-profile-graph authority, blocker codes,
  preconditions, and compensation. Clients discover the registry and blocker
  dominance order from Service contract metadata.
- Dominance: current live foreign-principal authority remains the highest hard
  blocker. Identity inconsistency precedes identity uncertainty, followed by
  principal, owner, subordinate, lease, route, and terminal installation
  defects.
- Validation: Rust formatting and strict Clippy passed; all nine frozen Plan
  0137 fixture recovery classes resolve to a registry action; every recoverable
  dominant blocker has a nonempty executor, preconditions, compensation, and
  exact-profile authority; Service contract metadata tests passed.
- Effects and rollback: source metadata and provider-free tests only. No live
  profile, lease, browser, route, process, provider, or installation state was
  changed. Reverting `e5fc37fd` restores the earlier registry.
- Remaining gate: Slice F must implement the route-bound CDP-free manual
  seeding executor named by the registry. Public parity and dashboard actions
  remain assigned to Slice G, migration and install compatibility to Slice H,
  and all development and production effects remain gated by Slices I and J.

## Slice F Checkpoint | 2026-08-28

- Objective: acquire a visible, route-bound first-authentication browser
  without enabling DevTools and retain one opaque operator handoff.
- Source commit: `242e5225` on `main`, pushed to `origin/main`.
- Transaction order: the existing supervised route coordinator selects the
  exact registered profile, plans and reserves presentation capacity, proves
  display access, and only then invokes headed `cdp_free_launch`. Route
  exhaustion and unregistered profiles fail before Chrome launch.
- Visibility: manual seeding substitutes process-bound visible-window evidence
  for a CDP target. Wrong process, absent browser window, missing display
  socket, stale route, and unavailable Guacamole states return typed
  `notVisible` evidence. Only `operatorVisible.state=ready` permits a visible
  response.
- Lifecycle: the persisted seeding handoff records the exact PID and blocks
  profile lease and CDP actions until close. An interrupted acquire reuses the
  existing process and opaque handoff instead of launching another Chrome or
  checking out a second route.
- Close and continuation: exact profile, target, handoff, route, and PID joins
  gate process termination. Close releases the owned route, viewer,
  controller, display, and capacity records through the existing exact route
  release contract, advances the seeding lifecycle to
  `seeding_closed_unverified`, and advertises attachable profile acquisition
  separately from the authenticated-state probe. Replay converges when the
  process or route was already released.
- Authentication boundary: every manual-seeding response reports
  authentication as `not_probed`; visibility is never accepted as login
  evidence.
- Validation: Rust formatting and strict Clippy passed; all 47 route-open tests
  passed serially, including seven focused manual-seeding tests for ordering,
  CDP denial, visibility failure evidence, durable handoff persistence, exact
  close mismatch rejection, and interrupted-close replay.
- Effects and rollback: source and provider-free scripted runtime changes only.
  No development or production browser, profile, route, provider, process, or
  installation effect occurred. Reverting `242e5225` restores the pre-seeding
  coordinator.
- Remaining gate: Slice G must publish acquire and close through every public
  surface and add dashboard review and confirmation UX before isolated
  development execution is eligible.

## Slice G Checkpoint | 2026-08-28

- Objective: make manual-seeding recovery and exact inert-record retirement
  usable from public contracts and dashboard surfaces without source knowledge.
- Source commit: `93a9cda9` on `main`.
- Contract parity: HTTP `/api/service/request`, MCP `service_request`, Rust
  contract metadata, the canonical JSON schema and field-role ledger, generated
  JavaScript and TypeScript clients, and action-specific helpers expose
  `service_profile_manual_seeding_acquire`,
  `service_profile_manual_seeding_close`,
  `service_browser_contamination_report`,
  `service_browser_retirement_plan`, and
  `service_browser_retirement_apply` with the same top-level fields and typed
  result shapes.
- Client guidance: README, CLI help, repository skill guidance, inline Rust
  documentation, and the docs site lead unproven identity callers to exact
  principal/profile/process/route reconciliation, then distinguish manual
  seeding close, attachable relaunch, authentication probe, contamination
  reporting, and exact record retirement.
- Dashboard: inert PID-less records expose a selected-browser retirement
  preview only when both retirement actions are advertised. The preview shows
  affected identity, plan, risk reasons, compensation, and expiry. Apply uses a
  shadcn/ui `AlertDialog`, and the terminal receipt remains visible. Browser
  retirement handlers remain distinct from incident acknowledge and resolve
  handlers.
- Compatibility and privacy: generic service request transport remains the
  compatibility path for older clients. Unknown actions remain rejected by the
  client or server instead of being applied implicitly. No capability field was
  added to plan or receipt data, and ephemeral profile capabilities remain
  outside queued Service State.
- Validation: Rust formatting and strict Clippy passed; focused Service contract
  tests, compiled no-launch contract smoke, API/MCP parity, the full generated
  service-client suite, route-confusion gates, dashboard view-stream, row-action,
  browser-table and inspector tests, dashboard production build, docs production
  build, and diff hygiene passed.
- Effects and exclusions: source and provider-free no-launch validation only.
  No development or production browser, profile, route, provider, process,
  dashboard publication, installed skill, or candidate installation was
  changed. Broad workstation and live-route selector recommendations were not
  run because they cross Slice G's no-live-effect boundary.
- Remaining gate: Slice H must prove source-free migration, mixed-version
  compatibility, rollback, and installation contract behavior before isolated
  development acceptance begins.

## Slice H Checkpoint | 2026-08-28

- Objective: make candidate migration inspectable and reversible without
  amplifying damaged Service State or dropping successor bookkeeping.
- Source commit: `216b3a4d` on `main`.
- Source-free discovery: every workstation dry run now emits
  `serviceStateMigrationPreview` with no write, backup, receipt, host-command,
  or artifact-directory effect. The preview reports exact added, removed,
  changed, and preserved IDs across 17 record classes plus the candidate-led
  browser contamination report.
- Migration safety: staging preserves unknown top-level and surviving nested
  successor fields. Protected record removals are explicit and empty for the
  accepted fixtures. Profiles, browsers, sessions, displays, routes, leases,
  handoffs, principals, capabilities, runtime owners, and lifecycle identities
  are not default deletion targets.
- Mixed-version compatibility: upgrade transactions, migration records,
  browser provenance, and runtime-owner lifecycle records tolerate additive
  successor fields. Current, legacy, and future recovery artifacts receive a
  read-only compatibility classification; unknown action types remain
  preserve-only and cannot acquire effect authority. Older terminal projections
  retain unknown top-level bookkeeping while the private detail artifact keeps
  candidate-era migration fields.
- Transactional apply and rollback: candidate preparation retains exact before
  and candidate bytes, backup locator, before and after digests, restore
  procedure, summary, and contamination report. Commit and rollback write one
  private terminal migration receipt. Existing selector, runtime-owner transfer,
  ingress, rollback, failure injection, and interrupted-resume paths remain in
  the workstation transaction engine.
- Public synchronization: CLI help, README, packaged repository skill,
  installation docs, inline Rust documentation, and the source-free workstation
  fixture describe and enforce the same preview, backup, receipt, preservation,
  and no-upgrade-for-advertised-recovery boundaries. The shared installed skill
  was deliberately not overwritten during source validation; development skill
  publication belongs to Slice I.
- Validation: diff hygiene and Rust formatting passed; strict Clippy passed; all
  20 Service State migration-filtered tests and all 116 workstation installer
  tests passed serially. The source-free workstation fixture, isolated host
  provision fixture, fresh-workstation VM harness contract, Guacamole asset and
  PostgreSQL durability contracts, route-specific user sync, remote-view docs
  gate, and docs production build passed.
- Effects and exclusions: source changes and disposable provider-free fixtures
  only. No production or development browser, profile, route, provider, process,
  selected generation, shared installed skill, or workstation installation was
  changed. Current production-state dry run and installation remain Slice J
  gates, not evidence claimed by this checkpoint.
- Remaining gate: Slice I must publish the isolated development candidate and
  prove disposable launch residue, CDP-free manual seeding, separated readiness
  axes, and a fresh post-acceptance OS resource census before any production
  candidate is eligible.
