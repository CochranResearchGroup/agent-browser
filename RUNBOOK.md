# Runbook

This file records dated execution turns for repo governance, planning, release,
and operational handoff work. Detailed command output belongs in validation
notes or artifacts, not in this log.

## Turn 211 | 2026-09-02

Scope: complete P157 W9 single-owner workstation convergence without
production Profile, browser, runtime, ACL, eviction, or shutdown effects.

Actions:

- added one Rust owner for desired workstation state, normalized observations,
  a sealed convergence plan, one executable action, and the final receipt;
- split dashboard runtime, convergence, access, and acquisition health so
  Profile ambiguity cannot create an installation warning;
- recognized the selected single runtime-host listener as authoritative
  without requiring the retired default socket;
- bound the privileged adapter to a Rust-sealed action list and verified its
  helper, lease-authority, and dependency postconditions; and
- changed the dashboard warning to consume only typed runtime and convergence
  blockers.

Validation:

- the full provider-free Rust gate passes with 1,915 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- source-free workstation and host-provision fixtures, dashboard build,
  API/MCP parity, generated-client, type, and dashboard contracts pass; and
- the P157 oracle passes with all five implementation cases green.

Result:

- W9 is complete and pushed at `d806c74c` without production effects; and
- W10 public contract, migration, client, doctor, help, skill, and docs
  alignment is next.

## Turn 210 | 2026-09-02

Scope: complete P157 W8 cohesive lease authority and exact lifecycle proof
without production Profile, browser, runtime, ACL, eviction, or shutdown
effects.

Actions:

- centralized protected lease-authority exchange and typed response validation
  in one client while preserving the kernel policy evaluator;
- kept human takeover separate from Profile access and proved it advances only
  controller authority while fencing the former controller;
- persisted exact lifecycle authorizations from forced eviction plans and
  joined them to current policy, daemon, browser, tab, and physical CDP target
  evidence before any close;
- settled only the proven tab, matching queued work, empty exact session, and
  matching viewer authority behind a minimal idempotency receipt; and
- required Operator assurance, both lifecycle permissions, the reviewed plan
  digest, and exact managed targets for full-runtime shutdown.

Validation:

- the full provider-free Rust gate passes with 1,909 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- all validation-selector workstation, route, streaming, API/MCP parity, and
  generated-client gates pass; and
- the P157 oracle remains at four green cases with only the W9 convergence
  case intentionally red.

Result:

- W8 is complete and pushed at `c46c6d43` without production effects; and
- W9 single-owner Rust install convergence and its privileged effect adapter
  are next.

## Turn 209 | 2026-09-02

Scope: complete P157 W7 revision-fenced Profile drain-and-restrict without
production profile, browser, runtime, ACL, or eviction effects.

Actions:

- added expected-revision compare-and-swap with current-revision and redacted
  structural conflict evidence;
- made widening immediate and made occupied narrowing persist a drain at the
  current revision until incompatible occupancy reaches zero;
- fenced new admission and later child control during draining while retaining
  exact own-tab release for graceful departure;
- computed blockers from attributed persisted tabs instead of caller input;
  and
- required separate explicit eviction permission, mode, exact targets, and
  minimal outcome receipts.

Validation:

- the full provider-free Rust gate passes with 1,901 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- focused policy, repository, revision-conflict, drain, and eviction-receipt
  tests pass; and
- the P157 oracle remains at four green cases with only the W9 convergence
  case intentionally red.

Result:

- W7 is complete and pushed at `d3c12100` without production effects; and
- W8 cohesive lease authority, human takeover, lifecycle proof, and
  full-shutdown authorization is next.

## Turn 208 | 2026-09-02

Scope: complete P157 W6 attributable tab participation without production
profile, browser, runtime, or ACL effects.

Actions:

- bound each admitted tab child to one service-generated transport connection
  and stable subject while preserving shared-browser reuse;
- persisted inherited child permissions on tab records and handles, then
  intersected every operation with the current parent policy;
- allowed the same stable subject to reconnect a disconnected child for
  one-shot HTTP and MCP work while preventing a live connection from being
  stolen by repeated labels;
- authorized observation, control, refresh, and exact own-tab release through
  the child policy; and
- protected service-owned connection and child-policy fields from caller
  injection and exposed the additive child contract through generated clients.

Validation:

- the full provider-free Rust gate passes with 1,896 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- complete service-client, generated-client, API/MCP parity, route-confusion,
  no-launch collection, and close-scope gates pass;
- focused connection, reconnect, subject isolation, and own-tab close tests
  pass; and
- the P157 oracle remains at four green cases with only the W9 convergence
  case intentionally red.

Result:

- W6 is complete and pushed at `83319369` without production effects; and
- W7 revision-fenced drain-and-restrict, graceful release, and explicit
  receipted eviction is next.

## Turn 207 | 2026-09-02

Scope: complete P157 W5 revisioned Profile access policy without production
profile, browser, runtime, or ACL effects.

Actions:

- added one deterministic access-policy evaluator to the Profile acquisition
  owner for `shared-local`, `restricted`, and `exclusive` modes;
- made `shared-local` the missing-policy default and limited strict runtime
  identity checks to explicit strict modes;
- prevented caller-supplied metadata from promoting self-declared identity to
  trusted ingress, registered capability, or operator assurance;
- propagated the admitted subject, assurance, policy revision, and access
  decision into executable requests and immutable provenance; and
- replaced circular identity-error recourse with typed permission context and
  one executable service-owned recovery action.

Validation:

- the full provider-free Rust gate passes with 1,891 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- Service Access, Service Request, MCP, policy, API/MCP parity,
  route-confusion, no-launch collection, and complete service-client gates
  pass; and
- the P157 oracle passes with four green cases and only the W9 convergence case
  intentionally red.

Result:

- W5 is complete and pushed at `f7166030` without production effects; and
- W6 attributable tab participation, reconnect, and inherited child policy is
  next.

## Turn 206 | 2026-09-02

Scope: complete P157 W4 unified terminal outcomes without changing profile
access policy or production runtime state.

Actions:

- added one typed terminal outcome model and one control-plane finalizer for
  success, failure, cancellation, timeout, and rejection;
- preserved the same structured failure and immutable provenance across the
  response, ServiceJob, terminal ServiceEvent, and trace projection;
- converted enqueue, scheduler, execution, cancellation, timeout, and response
  delivery exits to the common terminal path; and
- updated frozen schemas and generated client projections, then turned only
  the scheduler-rejection regression case green.

Validation:

- focused scheduler, timeout, cancellation, failure-classification, event,
  job, and terminal-outcome tests pass;
- the full provider-free Rust gate passes with 1,886 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass; and
- the P157 oracle, generated-client drift, service-client contract, client
  type, and patch checks pass.

Result:

- W4 is complete and pushed at `2ab08e87` without production effects; and
- W5 revisioned Profile access-policy evaluation is next.

## Turn 205 | 2026-09-02

Scope: complete P157 W3 immutable request provenance without changing access
policy or production runtime state.

Actions:

- added one deny-unknown-fields provenance model containing only the frozen
  causal identity allowlist;
- generated one service-owned connection-instance identity per daemon
  transport and retained the exact per-lane control-plane identity;
- preserved the same provenance through queued, waiting, running, succeeded,
  failed, timed-out, cancelled, and failed-to-enqueue ServiceJob records; and
- added the public ServiceJob contract and generated client type, then turned
  only the runtime-lane regression case green.

Validation:

- four focused provenance and persistence tests pass, including stable
  connection identity across two requests and private-field exclusion;
- the full provider-free Rust gate passes with 1,883 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass; and
- the P157 oracle, generated-client drift check, complete service-client gate,
  and client type check pass.

Result:

- W3 is complete and pushed at `79bb71f4` without production effects; and
- W4 unified terminal outcomes are next.

## Turn 204 | 2026-09-02

Scope: freeze P157 W2 schemas and source-grounded red provider-free
regressions without changing runtime behavior.

Actions:

- added v1 contracts for profile policy, access decisions, request provenance,
  terminal outcomes, migration, and dashboard health axes;
- froze `shared-local` as the exact default and kept access, coordination,
  lifecycle proof, and convergence independent;
- added five source-backed red cases for the Research.gov defects; and
- included the P157 oracle in the normal service-client test gate.

Validation:

- the P157 oracle passes with six schemas and five bounded red cases;
- the complete service-client gate, API/MCP parity, generated-client contract
  checks, and type checks pass; and
- the release verifier fixture, validation selector, and patch checks pass.

Result:

- W2 is complete and pushed at `cd0bdb1d` without production effects; and
- W3 immutable ingress provenance is next.

## Turn 203 | 2026-09-02

Scope: complete P157 W1 attempt 2 by placing current Profile acquisition
behavior behind one typed owner without changing policy or public contracts.

Actions:

- moved reuse evaluation, lifecycle replacement, dominant-blocker selection,
  route derivation, and executable request projection into Profile acquisition;
- changed Service Access to consume the typed acquisition plan and removed the
  temporary JSON-to-decision parser;
- preserved recovery as an internal child and action runtime as a typed result
  consumer; and
- applied one projection-consistency oracle to every current Service Access
  fixture.

Validation:

- 51 Service Access, 87 route-host, and 23 acquisition-recovery tests pass;
- the full provider-free Rust gate passes, including 1,881 parallel-safe tests
  and all serial environment-mutating partitions;
- formatting and workspace clippy with warnings denied pass; and
- service API/MCP parity, generated-client drift, and client type checks pass.

Result:

- W1 is source complete and pushed at `5166dabf` with no production or
  installed-runtime effects;
- the deep acquisition seam and initial semantic oracle are established; and
- W2 schema freezing and red provider-free regressions are next.

## Turn 202 | 2026-09-02

Scope: execute P157 W1 attempt 1 by establishing behavior-preserving Profile
acquisition ownership without changing permission or public-contract semantics.

Actions:

- introduced a typed acquisition artifact shared by access planning and
  action-runtime route application;
- removed downstream JSON reconstruction of acquisition blockers, lifecycle
  replacement, browser reuse, and authenticated launch-route receipts;
- placed the existing recovery coordinator inside the Profile acquisition
  module and redirected all production callers; and
- moved exact lifecycle-replacement evaluation under that owner and added
  reuse, launch, and denial oracle assertions.

Validation:

- 51 Service Access, 87 route-host, and 23 acquisition-recovery tests pass;
- the full provider-free Rust gate passes, including 1,881 parallel-safe tests
  and all serial environment-mutating partitions;
- formatting and workspace clippy with warnings denied pass; and
- service API/MCP parity, generated-client drift, and client type checks pass.

Result:

- W1 attempt 1 is source accepted at `2566592f` with no public behavior or
  runtime mutation;
- W1 remains open because profile reuse, dominant-blocker selection, and
  executable request projection still originate in `service_access.rs`; and
- attempt 2 must move those computations behind the typed interface and remove
  the temporary projection parser before permission semantics change.

## Turn 201 | 2026-09-02

Scope: incorporate the deep-module architecture review into P157 and restore
the current production runtime supervisor without deleting managed profiles.

Actions:

- revised the accepted ADR, glossary, roadmap, and P157 work graph around one
  deep in-process Profile acquisition owner, one cohesive lease-authority
  client, one Rust install-convergence owner, and one semantic contract oracle;
- separated dashboard convergence health from nonblocking profile-access
  ambiguity and request-scoped acquisition denial;
- diagnosed a healthy tmux-owned runtime host alongside an inactive matching
  systemd supervisor, then gracefully closed the four exact service-owned
  browser routes blocking controlled takeover; and
- applied the fresh digest-bound supervisor recovery plan, replacing source
  PID 1718 with supervised PID 43905 under transaction
  `runtime-host-takeover-d36051d4-bf3c-4069-89e7-e0202583b5ac`.

Validation:

- the recovery plan reported `ready_for_takeover` with no blockers and the
  applied outcome reported `accepted` at transaction revision 9;
- `agent-browser-runtime-host.service` is active and running on PID 43905;
- the dashboard backend supervisor is ready, its stream is reachable, and its
  executable matches installed generation `0.28.0-4a92c42517e1`; and
- install doctor reports one current runtime host, zero legacy daemons,
  `steady_current` multiplicity, and no supervisor issues.

Result:

- the current supervisor defect is repaired without removing any profile or
  stored credential state;
- the four service-owned browsers are closed and their tabs are no longer
  live, while unrelated externally owned browsers were preserved; and
- P157 W1 is now the behavior-preserving acquisition-owner extraction and
  semantic-oracle checkpoint, ahead of ACL or public-contract changes.

## Turn 200 | 2026-09-02

Scope: productize the profile-permission and logging defects uncovered during
Research.gov fieldwork without changing production runtime state.

Actions:

- pressure-tested and accepted a filesystem-like profile permission model with
  `shared-local` as the trusted local default and strict identity as opt-in;
- separated access policy, coordination leases, human controller posture, and
  exact runtime lifecycle proof in the domain model and accepted ADR;
- traced scheduler rejection and confirmed it returns before structured failure
  decoration while the runtime lane is removed before `ControlRequest` retains
  it; and
- opened P157 with request provenance and unified terminal outcomes ahead of
  access-policy enforcement, migration, eviction, and full-shutdown
  authorization.

Validation:

- CodeGraph and current source agree on the control-plane terminal-path split;
- Graphiti was healthy and recalled shared-profile routing but no prior complete
  permission or request-provenance decision;
- the active-lane and goal-governance audits pass before registration; and
- the worktree was clean before the P157 branch and documentation changes.

Result:

- the architecture, glossary, roadmap lane, and bounded implementation plan are
  registered;
- no source behavior, installed runtime, browser, profile, permission, lease,
  credential, or provider state changed; and
- W1 red contract fixtures are the next bounded slice.

## Turn 199 | 2026-09-02

Scope: qualify and close P156 in the isolated installed development runtime.

Actions:

- built source checkpoint `e46d9f75` with the optimized development profile
  and installed its byte-identical candidate as generation
  `0.28.0-fc8bf7e8bb33`;
- repaired development publication to persist the exact selected runtime-host
  ingress registry and added that identity to development doctor;
- reran installed doctor and the three-cycle disposable browser smoke; and
- exercised the installed full-shutdown dry-run without mutation.

Validation:

- built and installed candidate SHA-256 values both equal
  `fc8bf7e8bb332aeca599ed099d17104609484b22c54f3e067b2030d54f93536f`;
- every development doctor check passes, including runtime-host ingress;
- disposable open, URL-read, close, and residue checks pass for all three
  iterations; and
- the installed dry-run rejects one ambiguous historical development owner
  while leaving it untouched.

Result:

- P156 W1 through W6 are source complete and integration ready at
  `3bfb1c49`;
- production remained unchanged; and
- production full shutdown still requires a new reviewed plan and explicit
  authority.

## Turn 198 | 2026-09-02

Scope: implement and source-qualify P156 full-runtime-shutdown replacement
without changing production browser state.

Actions:

- added deterministic digest-bound replacement plans with exact browser and
  source-host process identities;
- added durable replay receipts, bounded cooperative close, exact verified
  browser termination escalation, two browserless census gates, and exact
  source-host retirement;
- integrated full shutdown with workstation prepare, resume, inspection,
  forward-only recovery, post-shutdown migration baselines, and candidate
  activation;
- exposed redacted replacement state and safe actions in transaction inspect;
  and
- updated CLI help, README, the agent skill, and both docs-site surfaces.

Validation:

- focused full-shutdown, migration, replay, digest, and forward-only tests pass;
- strict workspace Clippy, format, and patch hygiene pass;
- the source-free workstation fixture, host-provision fixture, fresh-VM harness,
  docs build, and remote-view docs guard pass;
- the canonical Rust runner recorded 1,879 passing parallel-safe tests; its
  source-anchor regression was repaired and its unrelated timing failure plus
  the repaired anchor both pass independently; and
- a current production dry-run was read-only and returned a ready plan with
  exact process identity for every managed close target.

Result:

- W2 through W5 are source qualified;
- W6 isolated installed qualification remains; and
- no production process, browser, profile, route, handoff, credential, or
  provider state changed.

## Turn 197 | 2026-09-02

Scope: open the provider-neutral full-runtime-shutdown installer escape hatch
without changing the retained Research.gov runtime.

Actions:

- diagnosed the missing composition between cooperative live-browser transfer
  and browserless supervisor takeover;
- opened P156 with an explicit destructive replacement contract that preserves
  profiles and credential-bearing browser data while ending live runtime state;
- added the installer replacement-policy parser boundary with `preserve` as the
  default and `full-shutdown` as an explicit reviewed-plan choice; and
- made full-shutdown apply require a 64-character plan SHA-256 and reject the
  older browserless override.

Validation:

- the focused parser regression was red before the new fields existed and is
  green after implementation;
- the missing-digest rejection passes; and
- Rust formatting and patch hygiene pass.

Result:

- W1 is complete on `feature/full-runtime-shutdown`;
- the dry-run plan, durable receipt, and exact shutdown executor remain to be
  implemented, so this source must not be installed yet; and
- no production process, browser, profile, route, handoff, or credential state
  changed.

## Turn 196 | 2026-09-02

Scope: reconcile first-time Research.gov authentication with the installed
desktop observation and interaction boundary.

Actions:

- reran the canonical Research.gov no-launch access plan and confirmed the
  retained `research-gov-nsf` browser is reusable while target readiness
  remains unknown and authenticated target evidence is empty;
- proved the selected posture is headed remote control with manual attached
  desktop input and an attachable login, so a separate detached seeding browser
  is not required;
- ran scoped remote-view doctor and resolved durable handoff `r580584` to ready
  presentation generation 6 without navigation or relaunch; and
- reconciled installed contract discovery with current product guidance:
  password-manager and passkey surfaces are named policy inputs, but real
  LastPass/passkey recognition and manipulation are not product-accepted.

Result:

- the operator can perform the first sensitive login through the ready durable
  handoff;
- LastPass credential selection, master-password entry, passkey approval, PIN,
  biometric, secure-desktop, and consent steps remain human-controlled; and
- the next product requirement is a bounded real browser-external credential
  workflow with paired page evidence, exact scene and controller binding,
  secret-free receipts, human-continuation gates, and no blind retry.

## Turn 195 | 2026-09-02

Scope: productize deterministic Research.gov durable-handoff resume and effect
authority classification without changing live browser state.

Actions:

- resolved handoff `r580584` without navigation and preserved the exact
  Research.gov service, agent, task, browser, session, profile, target, and URL
  tuple from its valid service tab handle;
- added `deriveServiceRemoteViewHandoffResumeIntent()` so clients no longer
  reconstruct retained route or caller identity from provider URLs or process
  inspection;
- added `classifyServiceControlPlaneAuthority()` so a current handle with
  incomplete attestation remains explicitly observation-only and only
  `complete=true` becomes effect-capable; and
- updated generated declarations, CLI help, README, agent skill guidance, and
  the remote-view guide with the same resume sequence.

Validation:

- the full service-client gate, package exports, JavaScript type checking,
  remote-view documentation guard, route-confusion gates, and API/MCP parity
  pass;
- the docs production build passes;
- workspace formatting, strict workspace Clippy, and 45 focused output tests
  pass through the WSL Cargo safety wrapper; and
- patch hygiene passes.

Result:

- source checkpoint `804519f0` is integration-ready;
- the live Research.gov browser, profile, route, lease, and freshness state
  were not mutated; and
- live navigation and input remain withheld because the current diagnostics
  lack canonical profile-lease proof.

## Turn 194 | 2026-09-02

Scope: stop the exact P152/P153 worktree-owned Chrome trees and complete Git
and active-lane custody reconciliation.

Actions:

- proved P152 process group `45686` belonged to its disposable test profile
  and P153 process group `16792` belonged to the managed default profile using
  exact start tokens, executables, profile paths, and worktree working
  directories;
- sent `TERM` to only those two process groups; both Chrome trees and their
  crashpad handlers exited immediately without forced termination;
- ran service reconciliation after exit, preserving the viable Research.gov
  browser and repairing retained route/display projections without releasing
  an active route;
- closed P147 from its merged and production-proven P148 receipt, and closed
  P152 at its installed truthful-planning boundary; and
- reduced the active-lane catalog to the genuinely open P144 paused branch.

Validation:

- neither removed Chrome process group nor its crashpad handlers remain;
- neither P152 nor P153 worktree has a live working-directory holder;
- every cleanup candidate is clean, equal to its remote, and contained in
  `origin/main`; and
- the active-lane auditor passes for the retained P144 catalog entry.

Result:

- P152 and P153 runtime cleanup obligations are satisfied without affecting
  the live Research.gov browser;
- P147, P148, P152, and P153 worktree custody is closed; and
- P144 remains open as a published paused ref rather than being mistaken for
  completed cleanup.

## Turn 193 | 2026-09-01

Scope: install the merged P150 race hardening, synchronize operator guidance,
and close only Git custody proven integrated and inactive.

Actions:

- reconciled clean `main` with `origin/main` at
  `b73dafcccb2366805de4350d0af8c09a01431ef4` and qualified optimized candidate
  SHA-256 `4a92c42517e1441f5e30b6fcf52857123efa7eb8273a8b126fc504de966333f7`;
- allowed the first transactional apply to fail closed when the shadow
  candidate received no authenticated handoff resolution within five minutes;
- used the existing private dashboard bootstrap credential to authenticate the
  staged candidate and resolve opaque handoff `r580584`, without logging or
  copying credential material;
- synchronized the ignored workspace candidate and user-scoped Agent Browser
  skill to the accepted source;
- removed four clean accepted or closed P153/P154 worktrees, deleted their
  integrated local branches plus the merged P154 remote branch, deleted the
  merged P150 remote branch, pruned worktree metadata, and ran automatic Git
  maintenance. The P153 topic worktree was retained because live Chrome
  processes still hold it as their working directory; active and ambiguous
  lanes were left untouched.

Validation:

- strict Clippy and formatting pass for the merged workspace;
- 91 service-health, 19 quarantine, and 10 desktop-locator tests pass;
- the source-free workstation fixture and durable-handoff documentation check
  pass;
- transaction `upgrade-a65e0348-7d32-4f62-9889-b4908c8cbe91` is accepted at
  revision 13 with candidate and dashboard presentation receipts; and
- production doctor succeeds with one current runtime host, one executable
  generation, zero legacy daemons, and no runtime-multiplicity issue.

Result:

- generation `0.28.0-4a92c42517e1-6121fd69672b` is selected and installed;
- installed command, workspace candidate, and source candidate have the exact
  SHA-256 `4a92c42517e1441f5e30b6fcf52857123efa7eb8273a8b126fc504de966333f7`;
- Research.gov remains healthy in browser PID 43472 and handoff `r580584` is
  ready at presentation generation 3 and owner generation 9; and
- P150 reaches installed acceptance. Legacy profile-lease provenance and the
  inactive optional supervisor remain nonblocking warnings owned elsewhere.

## Turn 192 | 2026-09-01

Scope: remove the lost-capability catch-22 without rotating production
authority.

Actions:

- confirmed the production Last30days lease is observation-only and idle with
  one active registered capability, no active claim, and no capability file;
- proved ordinary registration rejects another active grant while every
  reconciliation path requires the missing secret;
- opened P153 in an isolated worktree because the canonical checkout contains
  unrelated P150 edits;
- added public capability status and operator-local exact compare-and-swap
  rotation with active-work fences, staged-file cleanup, old-grant revocation,
  and exact owner-binding replacement; and
- left production capability, Last30days private configuration, credentials,
  browsers, and provider work untouched.

Validation:

- registry rotation, CLI parser, active-work blocker, and lost-file end-to-end
  fixtures pass individually;
- the lost-file fixture proves the old capability is revoked, the replacement
  authenticates, raw secret material is absent from Service State, and only the
  matching owner binding advances;
- the complete serialized Rust driver, strict Clippy, formatting,
  documentation, workstation, route-confusion, CDP architecture, CDP stream,
  and live tab-streaming gates pass;
- development generation `0.28.0-b48230cb56b0` passes doctor, and two
  independent smoke invocations each pass three disposable launch, URL, close,
  and residue iterations; and
- Graphiti discovery returned no prior rotation contract, so current source,
  live lease reads, and P153 remain authoritative.

Result:

- P153 topic commit `c09f3e19` was merged by `c664c25b` and pushed to
  `origin/main`;
- transaction `upgrade-c65674ff-8d5f-437c-a5e8-d46a7efed92c` installed exact
  generation `0.28.0-e2244cd2447c-c25a91eb0d2b` with binary SHA-256
  `e2244cd2447ce0de6239d41b7fbec7e77aad9145e57ca86cd2ad2de7bf3c7d94`;
- the authenticated presentation receipt resolved successfully without page
  navigation or credential access, production doctor succeeds with one current
  host and zero legacy daemons, and installed capability status reports
  `rotationAllowed=true` with no blockers; and
- P153 is accepted as installed recovery capability. Production capability
  rotation, owner-private Last30days wiring, and another provider tick remain
  unexecuted and require separate operator authority.

## Turn 191 | 2026-09-01

Scope: install the P152 companion repairs, execute the single authorized
Last30days acceptance tick, and reconcile its remaining browser-acquisition
failure against the public planner and executor seams.

Actions:

- installed Agent Browser generation `0.28.0-390ee922ae7b-e2f4b5e5d874`
  and Last30days service 0.3.93 with schema 17;
- executed the one authorized X and LinkedIn acceptance tick; both providers
  reached all three configured attempts and released every resource lease;
- proved Last30days copied the planner's fresh `terminal-profile-*` session but
  supplied no profile capability to either access planning or execution; and
- made unauthenticated terminal replacement planning fail closed with
  `profile_capability_required` before daemon relay.

Validation:

- the installed Agent Browser source and binary digests match, doctor succeeds,
  and runtime state has one current host with zero legacy daemons;
- Last30days reports service 0.3.93, schema 17, compatible MCP 4.0.4, database
  quick-check success, and zero active ticks, attempts, or leases;
- the acceptance tick terminalized `complete_degraded`; X and LinkedIn each
  persisted retry ordinals zero, one, and two; and
- all 61 Agent Browser service-access tests, focused Plan 0137 regressions,
  formatting, and production-target Clippy pass for the truthful-planning fix.

Result:

- the retry-persistence repair is production-proven;
- the original planner/executor disagreement is merged and installed as
  generation `0.28.0-3a090663b346-7118cc148917`: an
  unauthenticated caller is no longer given an executor-inadmissible request;
- live Last30days planning now returns `available=false`, a null request, and
  `profile_capability_required` while doctor remains successful with one
  current runtime host and zero legacy daemons;
- successful Last30days acquisition still requires an operator-authorized,
  privately stored capability registration and client wiring; and
- the single authorized acceptance tick is consumed and must not be retried
  without new authority.

## Turn 191 | 2026-09-01

Scope: make terminal route quarantine recovery self-converging during live
Research.gov fieldwork without weakening active acquisition ownership.

Actions:

- installed production generation `0.28.0-2d1c34415875-1aa980055f30` after a
  browserless-upgrade census and confirmed the exact August 31 Research.gov
  lease reached `failed/rollback_complete` with inactive cleanup proof;
- identified and reconciled restored prior owners, stale ready handoffs,
  detached historical pending leases, completed historical receipts, and the
  missing compare-and-swap persistence of reconciled acquisition rows;
- observed a fresh-acquisition race where background reconciliation rolled back
  a seconds-old pending lease before the foreground `DisplayReady` transition;
- recovered the surviving ready browser through a no-launch reattach and
  created durable handoff `r580584`; and
- added a 15-minute minimum age plus ownership-identity matching before
  detached pending acquisition convergence.

Validation:

- 66 service-health tests and 19 quarantine tests pass;
- strict Clippy and formatting pass;
- development generation `0.28.0-4d2c8ce7ecba` passed doctor, isolated provider
  checks, and three disposable browser launches with the age fence installed;
- production candidate
  `672d311dd3542e9f58551ce8f97a4f3fa10de0a92dfff48a60b8d4f95e2fdd97`
  was built from the same source, while its handoff-safe activation correctly
  preserved the old generation when candidate presentation was not proven;
- a separate coordinated upgrade accepted generation
  `0.28.0-e2244cd2447c-c25a91eb0d2b` after an authenticated candidate
  presentation receipt, finalized the Research.gov lane at owner generation 6,
  and restored a fully green install doctor;
- installed hash `2d1c3441587521bc75a5fc2115bdae3f05e50dc770bc0761cbd5f636cfd60d13`
  matched the accepted production candidate; and
- the live Research.gov browser, route, display, target URL, and operator
  presentation all read ready after reattach.

Result:

- the operator can use the durable Research.gov handoff without another browser
  launch;
- the age-fence source amendment is development-qualified but is not part of
  the accepted `e2244cd2447c` production generation; its later installation
  must use the same handoff-safe path and must not disrupt the ready handoff.

## Turn 190 | 2026-09-01

Scope: reconcile P152 with P150/P151 and qualify the exact integrated terminal
replacement repair before production installation.

Actions:

- merged current `origin/main` into the P152 topic branch while preserving both
  plan histories and the current P150/P151 lane population;
- kept P150 automatic quarantine convergence and P152 terminal session
  replacement as distinct, composed safety contracts;
- qualified the schema-17 Last30days companion source and compatibility
  contract; and
- built and installed only an isolated Agent Browser development generation.

Validation:

- focused P152 regressions and strict production-binary Clippy pass;
- the complete serial Rust workspace passes 2,854 tests with zero failures and
  57 intentional ignores;
- development generation `0.28.0-4a9882a9f4d7` passes doctor; and
- three disposable development browser launch, URL-read, close, and residue
  cycles pass.

Result:

- P152 is development-qualified at `slice_f_production_installation`;
- production remained unchanged; and
- the exact integrated workstation dry-run and apply are the next gate.

## Turn 189 | 2026-09-01

Scope: open P152 to repair terminal session replacement planner and executor
parity plus the downstream Last30days retry persistence contradiction.

Actions:

- confirmed the production profile is available with zero holders and zero
  live browsers;
- proved access planning recommends a new browser while copying terminal route
  `last30days-force-20260901-c35` into the request;
- proved the terminal owner is generation 71 while its retained authenticated
  principal binding is generation 66;
- reconciled conceptual overlap with active P150 while preserving its separate
  remote-view quarantine implementation files; and
- opened isolated branch `fix/terminal-session-replacement-parity` without
  touching the dirty P150 worktree.

Validation:

- Last30days service 0.3.92 is ready and MCP-compatible;
- Agent Browser reports the selected profile available and the terminal lane's
  exact process exit plus profile-lock release;
- CodeGraph identifies the terminal relaunch and authenticated continuity
  predicates used by the executor; and
- Graphiti returned ten older routing facts but no current incident evidence,
  so live state and repository source remain authoritative.

Result:

- P152 is OPEN at `slice_a_red_fixture`;
- progress classification is `blocker_reduction` under inherited operator
  authority; and
- no browser, profile, provider, database, or installed-runtime mutation has
  occurred.

Next action: add one provider-free public-seam regression that reproduces the
planner-to-executor failure before implementation.

## Turn 188 | 2026-09-01

Scope: make a provably inactive route-bound acquisition quarantine converge
automatically without weakening live-owner safety.

Actions:

- opened P150 after proving the older P114 repair required route and display
  records to be released even though normal reconciliation leaves dead resources
  orphaned;
- added automatic dependency-ordered convergence to normal service reconcile;
- added one matching convergence attempt inside route-bound acquisition before
  its historical quarantine scan;
- preserved the pre-pass ownership snapshot so reconcile cannot erase active
  viewer, controller, handoff, presentation-slot, or route-checkout evidence and
  then treat the same pass as safe;
- made explicit repair return `repaired=false` and typed next-step guidance when
  no candidate was actually changed; and
- updated CLI help, README, the Agent Browser skill, and service-mode docs.

Validation:

- the orphaned-state and matching-acquisition fixtures failed before the repair
  and pass afterward;
- seventeen quarantine tests and eighty-six service-health tests pass;
- strict Clippy, Rust formatting, documentation build, remote-view handoff docs,
  and the selected source-free workstation checks pass; and
- no development or production candidate has been installed yet, and no browser,
  profile, provider, authentication, or retained production state was mutated.

Result:

- P150 is development-qualified through Slice D;
- generation `0.28.0-52fde82a55d7` passed development doctor and three
  disposable browser-launch iterations while production remained unchanged;
- the next gate is integration followed by an exact production workstation
 dry run.

## Turn 187 | 2026-08-29

Scope: implement and source-accept the provider-free Authentication Run
foundation for unattended BILL authentication without accessing a browser,
tenant profile, message service, or mailbox.

Actions:

- opened Plan 0138 on isolated branch `plan/authentication-run-foundation`;
- added an internal `AuthenticationRun` model bound to one principal, task,
  target account, profile, browser, session, login tab, site recipe, and policy;
- separated sensitive challenge custody behind a response-only action trait
  whose outward contract contains only redacted receipts;
- retained delivery-fence, unique-candidate, native-credential-store,
  same-profile new-tab, replay, transition-budget, and exact-target verifier
  evidence;
- rebased the source checkpoint onto concurrent main commit `1c96ac67` without
  touching that lifecycle repair; and
- kept public API, Service State persistence, installed runtime, and live
  provider integration out of this packet.

Validation:

- nine focused Rust tests passed after the rebase;
- synthetic SMS OTP and email verification URL canaries are absent from
  serialized runs, receipts, errors, and debug projections;
- formatting and diff hygiene passed;
- the selector's strict production-target Clippy gate passed; a broader
  optional test-target lint run reached only pre-existing warnings outside the
  branch; and
- no browser, profile, message, mailbox, provider, installation, or tenant
  effect occurred.

Result:

- P138 Slice A is `source_accepted_public_contract_not_started`;
- source checkpoint is `d0786a5a`; and
- the next bounded packet is durable Service State persistence plus complete
  public contract parity, still provider-free.

## Turn 186 | 2026-08-24

Scope: open and execute P129 for request delivery, lifecycle projection, and
reviewed runtime cleanup, followed by one transactional workstation upgrade.

Actions:

- re-anchored current main, installed generation, doctor, lifecycle, resource,
  supervisor, QBO, and BILL-profile evidence;
- confirmed every existing secondary worktree branch is already integrated
  into `origin/main` and left those worktrees plus the isolated development
  runtime unchanged;
- reproduced that non-retry-safe dashboard ingress requests can return a
  retryable 503 after their backend jobs complete;
- recorded duplicate cold-start targets, cleanup-policy disagreement, hidden
  terminal replacement evidence, nonterminal lifecycle debt, resource
  misclassification, and split QBO identity as P129 acceptance criteria.

Validation:

- Graphiti runtime and MCP are healthy, but current P128/P129 recall is not
  useful; repository and live runtime evidence remain authoritative;
- policy selection classifies the repo as an operations platform and the
  goal-execution audit passes;
- production remains on the accepted P128 generation and no new install,
  provider, profile, or cleanup effect has occurred in this turn.

Result:

- P129 is `source_repair_in_progress`;
- the delayed-backend at-most-once regression is the first implementation
  slice.

## Turn 185 | 2026-08-23

Scope: reconcile and source-accept the P128 runtime lifecycle hotfix collection
without changing production or the active development feature lane.

Actions:

- merged the accepted absent-closing lifecycle branch into an isolated hotfix
  worktree and retained its prior acceptance artifacts as P128 historical
  packets;
- routed auto-launch and explicit owned launch through one persistence cleanup
  obligation, using lifecycle-authorized close when registration already
  created an owner binding;
- retained exact orphan recovery through the existing durable handoff,
  process, profile, CDP, target, owner-generation, and runtime-state seam;
- separated quiescent historical supervisor warnings from blocking live,
  starting, drifted, conflicted, and unavailable supervisor failures;
- aligned the install-doctor fixture with the current exact route-session
  termination helper contract.

Validation:

- 9 close/launch, 12 runtime lifecycle, 12 session supervisor, 20 serial install
  doctor, 4 workstation payload, and the exact missing-projection recovery
  tests passed;
- Rust formatting, strict Clippy, diff hygiene, workstation install and host
  fixtures, fresh-VM harness, Guacamole assets and PostgreSQL durability,
  route-user sync, remote-view handoff docs, and the docs production build
  passed;
- the installed workstation dry-run returned `mutated=false`; no production
  browser, provider, profile, service, or accounting state changed.

Result:

- P128 is `source_accepted_install_pending`;
- protected-main integration, candidate build, transactional census, install,
  and one harmless local `bill-soylei` acquisition proof remain.

## Turn 181 | 2026-08-23

Scope: install P124 in the isolated development environment and execute every
currently runnable Slice G gate without touching production.

Actions:

- built and installed development generation `0.28.0-76c3dddafb22` through the
  P125 publisher, which reported production unchanged;
- validated development doctor, local and external ingress, authenticated
  dashboard, policy projection, three browser launch/close cycles, live CDP tab
  streaming, zero retained dev browser/session records, service GC dry-run,
  and generation cleanup;
- removed four unreferenced development generations while retaining the
  selected generation and one rollback generation;
- refused to borrow production Guacamole/RDP routes or fabricate presentation
  slots when the development environment reported no presentation capacity;
- recorded the exact partial acceptance and remaining provider gate in
  `docs/dev/notes/0124-7-2026-08-23-controlled-development-install-partial-acceptance.md`.

Validation:

- development generation, all three units, ports 4948, 4949, and 4951, auth,
  manifests, executable selection, browser executable, backend, and local
  ingress passed their environment-owned doctor;
- external authenticated dashboard, three browser launch cycles, and live CDP
  tab streaming passed;
- development service GC found zero candidates and Service State retained zero
  browsers and sessions after cleanup;
- production non-interference passed in the development publisher.

Result:

- P124 is `slice_g_development_installed_partial_provider_namespace_blocked`;
- source Slices A through F and the non-provider installed-development surface
  are accepted;
- four-to-six provider-backed presentation acceptance requires a new isolated
  development Guacamole/XRDP/display namespace.

## Turn 180 | 2026-08-23

Scope: implement and accept P124 Slice F agent and operator product surfaces.

Actions:

- added the redacted `desktopEvidencePolicy` to Service Status, CLI output,
  install doctor, generated client types, and the dashboard;
- documented the CDP-first evidence decision, browser-external desktop cases,
  capture-ready proof, generic-failure prohibition, sensitive human
  continuation, and independent P110 input gate in every required user-facing
  surface;
- retained logical browsers, presentation capacity, and evidence transport as
  separate dashboard concepts;
- recorded source acceptance in
  `docs/dev/notes/0124-6-2026-08-23-desktop-evidence-product-surface-slice-f-source-acceptance.md`.

Validation:

- 15 desktop-evidence, 18 status-projection, 2 status-formatter, and 18 install
  doctor focused Rust tests passed;
- the generated service client suite, dashboard inspector contract, dashboard
  production build, documentation production build, Rust formatting, strict
  Clippy, and diff hygiene passed;
- service API and MCP parity, remote-view handoff docs, and the selected
  source-free workstation, host, VM, Guacamole, PostgreSQL, route-user, and
  payload-provenance fixtures passed;
- no live or installed effect occurred.

Result:

- P124 is `slice_f_source_accepted`;
- Slice G controlled installed acceptance in the isolated development runtime
  is next.

## Turn 179 | 2026-08-23

Scope: implement and accept P124 Slice E elastic presentation lifecycle and
exact garbage-collection authority.

Actions:

- promoted presentation lifecycle from test-only fixtures to a provider-free
  authority over the durable presentation-capacity inventory;
- added pressure and hard-limit admission, one-slot scale-out, exact lifecycle
  and owned-resource identity, cooldown, reference-aware scale-in, rollback
  quarantine, and cleanup obligations;
- removed the prior duplicate private slot inventory;
- recorded source acceptance in
  `docs/dev/notes/0124-5-2026-08-23-elastic-presentation-lifecycle-slice-e-source-acceptance.md`.

Validation:

- 7 focused lifecycle tests passed, including three repeated elastic cycles
  converging to the warm minimum with no retained elastic identities;
- pressure rejection and retained references produced no provider or garbage
  collector call;
- Rust formatting, strict Clippy, and diff hygiene passed;
- no live or installed effect occurred.

Result:

- P124 is `slice_e_source_accepted`;
- Slice F agent and operator product surfaces are next.

## Turn 178 | 2026-08-23

Scope: confirm isolated development prerequisites and accept P124 Slice D at
the provider-free source boundary.

Actions:

- verified the accepted P125 development runtime through its environment-owned
  doctor and direct unit, port, selector, and executable readback;
- promoted the desktop-evidence decision model into an injected transaction
  coordinator spanning CDP, capacity, staging, window semantics, trigger,
  frame capture, authorized input, verification, restoration, handoff, release,
  and cleanup seams;
- enforced CDP-only page evidence, paired absence evidence for browser-external
  prompts, pre-trigger staging, human precedence, two-sided capture-ready proof,
  terminal drift receipts, and the independent P110 input gate;
- recorded source acceptance in
  `docs/dev/notes/0124-4-2026-08-23-desktop-evidence-episode-slice-d-source-acceptance.md`.

Validation:

- the development doctor passed with one selected generation and all three
  isolated services on their expected ports;
- 14 focused desktop-evidence tests, Rust formatting, strict Clippy, and diff
  hygiene passed;
- Build Admission reported active capacity two and ran each admitted Rust
  command with four Cargo jobs;
- production remained read-only and no provider, browser, dashboard, install,
  or ingress effect occurred.

Result:

- P124 is `slice_d_source_accepted`;
- Slice E elastic lifecycle and garbage collection is the next bounded
  source-only packet.
## Turn 184 | 2026-08-23

Scope: qualify and transactionally install the absent-closing lifecycle
hotfix while preserving concurrent feature work.

Actions:

- repaired candidate-dashboard startup timing, exact reversed stale handoff
  retries, old-daemon retry cleanup, and stale in-memory lifecycle bindings;
- validated focused regressions, formatting, and strict Clippy, then built
  candidate SHA-256
  `a89625b870c3cda3cde9b41f27271ebe36d60683b1235c4196c4be337bb39ea6`;
- accepted workstation transaction
  `upgrade-7d9a2776-2c7e-458c-8e4c-eb2bbe989c46` using an authenticated
  provider-free presentation receipt;
- proved exact same-profile route-bound replacement at owner generation 3 and
  closed the fixture through its current owner.

Result:

- production selects generation `0.28.0-a89625b870c3-1e2c09b12ebc` with all
  workstation readiness axes true and rollback ready;
- the replacement closed to `terminal/satisfied` with exact process-exit and
  profile-lock evidence, leaving no browser or session record, Route B
  available, and zero cleanup candidates;
- the isolated development generation and unrelated feature-work checkout
  remained unchanged;
- the predecessor lifecycle packet is `CLOSED`. The next action is
  protected-main review and integration,
  not another workstation upgrade.

## Turn 183 | 2026-08-23

Scope: continue the unaffected lifecycle source and provider-free development-runtime
work after the workstation upgrade gate closed.

Actions:

- repaired shutdown aggregation so a graceful browser exit followed by
  successful auxiliary cleanup cannot be classified as a force-kill failure;
- merged then-current `origin/main` and reconciled the plan-number collision
  that existed in that branch history;
- isolated unit tests from the operator's installed runtime-ingress registry
  unless a test explicitly opts in, and aligned a stale dashboard test with
  the deliberate removal of unknown-session fallback;
- published the candidate only to the isolated development runtime as
  generation `0.28.0-b1a74a64a0dc` with SHA-256
  `b1a74a64a0dc0a80bb145a7334b741b7376c04b06829f77c72aa2ca955d9f22f`;
- ran three disposable provider-free `about:blank` open, URL-read, and close
  cycles without navigating to X, LinkedIn, or another provider.

Validation:

- canonical Rust CI passed: 1,412 parallel-safe tests, 57 ignored tests, all
  isolated environment-mutating modules, and integration partitions passed;
- strict Clippy, Rust formatting, diff hygiene, route-confusion gates, all
  selected workstation and Guacamole fixtures, docs build, remote-view handoff
  docs, and installed skill parity passed;
- development doctor passed with all three units using the selected
  development generation;
- post-smoke development readback found zero sessions, zero active incidents,
  and zero force-kill failure classifications;
- the development publisher and smoke both reported the production generation
  remained `0.28.0-4b975a51aa89-d0782705d5ff` with installed SHA-256
  `4b975a51aa892241ea73cc6e8acef42bb67d781c8b9be43edbc1086f4d7956f8`.

Result:

- provider-free normal close and shutdown classification are accepted in the
  isolated development runtime;
- deterministic lifecycle coverage accepts exact next-generation
  same-profile replacement;
- production and authenticated provider profiles were not changed or opened;
- the lifecycle packet remains `OPEN` only for harmless route-bound same-profile replacement
  acceptance, which does not block unrelated Agent Browser work.

## Turn 182 | 2026-08-23

Scope: reconcile the preserved named-profile lifecycle and diagnose one bounded
X and LinkedIn feed tick after the Agent Browser upgrade.

Actions:

- verified installed version `0.28.0`, selected generation
  `0.28.0-4b975a51aa89-d0782705d5ff`, one dashboard, one runtime host, zero
  legacy daemons, and no live browser for the selected profile;
- implemented exact absent-process and absent-lock completion for matching
  `closing/owned` lifecycle records, then repaired repository merge so the
  completed transition persists;
- reconciled the selected profile to `terminal/satisfied` without replacing,
  reseeding, reauthenticating, launching, or killing its browser;
- ran the one authorized X and LinkedIn tick and traced both pre-navigation
  failures to terminal replacement rejecting the service's new logical browser
  ID;
- repaired terminal replacement so a collision-free next-generation owner may
  move the lifecycle record to a new logical ID, while pending transfers,
  duplicate profile records, and key collisions remain fail-closed;
- recomputed package launch identity after generation advance and added
  cross-logical-ID and collision regression tests.

Validation:

- all 12 `native::runtime_lifecycle::tests` pass;
- all 50 `native::service_health::tests` pass;
- strict Clippy, formatting, and diff hygiene pass;
- the bounded tick `tick-7224876f30d729e41ff5435b387be4df`
  launched and politely closed both browser processes, released both profile
  locks, and rolled back route and display leases;
- X job `r923698` and LinkedIn job `r841495` each observed zero posts because
  `remote_view_open` failed before provider navigation.

Result:

- the authenticated profile is preserved and the stale lifecycle is
  `terminal/satisfied`;
- the newly exposed replacement defect is repaired and validated in source;
- the lifecycle packet remains `OPEN` because provider-free normal-close and replacement
  acceptance is incomplete;
- the corrected production binary was not installed and no
  provider retrieval has yet occurred;
- no second tick was run, so this turn yields no X or LinkedIn authentication,
  retrieval, acceptance, or filtering conclusion.
## Turn 177 | 2026-08-23

Scope: diagnose, repair, and accept P126 before new feature development.

Actions:

- reproduced development Chrome exit code zero before DevTools and proved the
  selected Windows-mounted browser was incompatible with the Linux-only
  development profile root;
- pinned a validated Linux browser executable through the development
  descriptor, stable launcher, units, status, doctor, and generation manifest;
- required positive development Runtime Environment ownership before service
  GC can admit any global process as a candidate;
- added deterministic publisher and service-resource regressions plus a
  three-cycle installed browser launch and residue smoke;
- reinstalled only the development generation and preserved production
  identities through every installed effect;
- terminated exact stale test process group `72225` after repeated PID, start
  token, executable, profile, process-group, and no-service-owner checks, then
  moved its test home and two closed diagnostic profiles to trash;
- synchronized the repository and installed Agent Browser skill guidance.

Validation:

- the original stable development launch succeeds without a caller override;
- three consecutive open, URL-read, close, and no-residue cycles pass with
  production identity unchanged;
- development doctor passes with selected generation
  `0.28.0-b3dc87dcc29a` and browser `/opt/google/chrome/chrome`;
- fresh production and development GC dry-runs both report zero candidates;
- focused Rust tests, development publisher fixtures, Rust format, strict
  Clippy, docs build, remote-view docs, release fixtures, skill sync, and diff
  hygiene pass;
- production remains on generation
  `0.28.0-4b975a51aa89-d0782705d5ff`, runtime host PID 75310, and dashboard PIDs
  87827 and 87877 with three retained browsers and 63 sessions.

Result:

- P126 is `ACCEPTED` at source and installed-development boundaries;
- production remained read-only and active;
- P124 Slice A is the next bounded source-only packet.

## Turn 176 | 2026-08-23

Scope: implement and accept P125 without interrupting active production use.

Actions:

- replaced the full-lifetime Cargo mutex with resource-aware Build Admission,
  capped at two bounded invocations and reduced by current memory, swap, CPU,
  disk, live-claim, and user-systemd evidence;
- implemented the isolated immutable `agent-browser-dev` publisher, launcher,
  pseudo-home, state, socket, auth, runtime lane, three user units, doctor,
  rollback, and generation cleanup;
- added runtime-environment identity to the manifest and a visible Development
  dashboard badge;
- published separate local and external Cooper inventory routes with no
  development `/guacamole` binding;
- replaced the development generation, stopped and restarted only development
  units, and moved disposable development smoke profiles to trash after proving
  no matching process remained;
- recorded exact acceptance in
  `docs/dev/notes/0125-2026-08-23-development-runtime-isolation-acceptance.md`.

Validation:

- two real Cargo commands were admitted concurrently when live capacity
  allowed; observed workstation pressure reduced admission and produced a
  typed wait;
- the installed development doctor passes unit, executable, process, port,
  lane, auth, manifest, and local-ingress checks;
- local and external authenticated dashboard smokes return the development
  manifest and exact selected generation;
- production before/after receipts preserve its selected generation, three
  process identities, binary and dashboard digests, durable handoff digest,
  three browser identities, and 63 prior session identities;
- source, Rust, dashboard, documentation, contract, client, and diff-hygiene
  gates pass;
- a final development service GC dry-run reported three reviewed candidates
  totaling 83.5 MiB and made no change.

Result:

- P125 is `ACCEPTED` at source, installed development, and ingress boundaries;
- production remained read-only and active;
- later installed P124 work now has a dedicated experimental runtime;
- fresh Chrome launch remains a recorded host-level diagnostic follow-up after
  an earlier isolated development-profile launch proved process and profile
  separation.

## Turn 175 | 2026-08-23

Scope: freeze and begin P125 so experimental installation and compilation can
proceed without interrupting production Agent Browser use.

Actions:

- inspected the selected production binary, generation store, runtime host,
  dashboard pair, sockets, ports, authentication path, Cooper inventory, and
  Cargo wrapper;
- confirmed production is active on dashboard ports 4848 and 4849 with its
  Guacamole path on 8092, while no `agent-browser-dev` Cooper inventory exists;
- measured 47 GiB available memory, 12 GiB free swap, 20 logical CPU cores,
  and 706 GiB free disk during active workstation use;
- defined Runtime Environment, Development Runtime, Build Admission, and
  Environment Receipt in the repository glossary;
- wrote P125 with exact development identities, initial two-build admission
  policy, development-only installation and ingress authority, and mandatory
  production non-interference acceptance;
- made P124 depend on P125 before installed experimental desktop work.

Validation:

- Graphiti was healthy; its broad query returned older source facts but no
  authoritative existing development-runtime design;
- CodeGraph exposed the production path and runtime-host blast radius and the
  current distributed HOME, socket, auth, installer, and unit decisions;
- the repository policy selector recommended the existing operations-platform
  profile and the active planning audit returned zero findings;
- no production process, unit, browser, provider, or ingress effect occurred.

Result:

- P125 is `OPEN` with execution authorized for source and the isolated
  development environment;
- production is explicitly read-only;
- implementation begins with Build Admission and the development publisher.

## Turn 174 | 2026-08-23

Scope: plan scalable desktop-evidence admission and presentation capacity
before adding more desktop-mode features, without changing source behavior or
live runtime state.

Actions:

- reconciled P110 desktop perception, P60 capacity exhaustion, P67 retained
  browser reattachment, P117 lifecycle ownership, current route selection,
  desktop capture readiness, window focus, viewer/controller leases, and
  garbage-collection authority;
- confirmed the current Guacamole readiness implementation recognizes route A
  and B and truncates selected candidates to two;
- defined Desktop Evidence Episode, Presentation Slot, Warm Presentation Pool,
  and Capture-Ready Proof in the repository glossary;
- wrote Plan 0124 to make presentation capacity arbitrary-N, accept four warm
  slots with controlled elastic scale-out to six, keep CDP work slot-free,
  prioritize humans and recovery, stage and restore exact scenes, and reclaim
  only unreferenced presentation resources through exact lifecycle authority;
- preserved the unrelated runtime-profile repair and P110 fieldwork changes
  already present in the shared worktree.

Validation:

- planning and architecture policies were re-read;
- Graphiti group `agent_browser_main` returned 12 facts across five episodes;
  relevant route-capacity and viewer-contention leads were verified against
  P60, P67, and current source;
- CodeGraph confirmed the route-selection, desktop-context, and fixed
  readiness-script impact surfaces;
- no browser, route, display, provider, installed runtime, or tenant effect
  occurred.

Result:

- P124 is `PLANNED` with `architecture_frozen_source_not_started`;
- two presentation slots are now a legacy installed configuration, not the
  product architecture or target capacity;
- the first bounded packet is provider-free Slice A fixtures and architecture
  guards only.

## Turn 173 | 2026-08-23

Scope: preserve redacted productization lessons from the operator-authorized
LastPass passkey and email-verification rehearsal without changing source or
live runtime state.

Actions:

- reconciled the private fieldwork receipt with P110, the service
  authentication roadmap, control-plane attestation, runtime boundaries, and
  fieldwork-productization policy;
- recorded the observed authentication state machine, LastPass adapter
  boundary, second-factor orchestration phases, deterministic image pipeline,
  XTEST input-provider requirements, stop taxonomy, privacy rules, and staged
  productization sequence;
- retained only the private artifact identifier and completion-receipt digest,
  excluding private pixels, account identity, message content, one-time codes,
  provider URLs, and runtime paths;
- linked the fieldwork note from P110 without changing its state.

Validation:

- `git diff --check` passed;
- every repository authority linked by the fieldwork note exists;
- a changed-note privacy scan found none of the excluded private identifiers;
- `pnpm validation:select -- --base HEAD` selected the expected documentation
  hygiene check;
- the documentation production build completed successfully with 35 static
  pages generated.

Result:

- the fieldwork is classified `REFACTOR BEFORE KEEP`;
- P110 remains `ALL FIVE POCS SOURCE ACCEPTED | LIVE FOUNDATION ACCEPTANCE
  BLOCKED` because the click did not use a production input provider,
  controller lease, cross-process fence, or canonical `desktop_interact`
  receipt;
- the next bounded step is a separately authorized controlled-provider plan
  beginning with synthetic LastPass and second-factor fixtures.

## Turn 172 | 2026-08-23

Scope: qualify the exact Plan 0122 source for publication and installed-runtime
admission without changing the installed generation or any browser workload.

Actions:

- verified local source, remote baseline, installed binary, selected
  generation, workstation status, and installed doctor;
- built the exact optimized candidate from commit `5fd4be88`;
- ran the candidate's non-mutating workstation dry-run;
- corrected the expanded Plan 0122 baseline SHA to the authoritative commit;
- opened Plan 0123 for separately authorized transactional installation and
  installed no-launch qualification.

Validation:

- candidate SHA-256 is
  `ae49edfd9d71161543c8378c06688876984f891b46cedca5272de1e77ca2f811`;
- installed doctor has zero issues and reports one dashboard, one runtime host,
  one executable generation, zero legacy daemons, and converged state;
- candidate dry-run returns success, planned state, and `mutated=false` on a
  supported host with effective groups, no missing commands, and sufficient
  disk;
- no upgrade transaction, runtime census receipt, admission drain, payload
  staging, installation, browser, provider, or tenant effect occurred.

Result:

- Plan 0123 read-only admission is accepted;
- commits `990e6b31`, `5fd4be88`, and `f697969d` are published on `origin/main`
  with exact remote readback and zero divergence;
- service resource inventory and both GC dry-runs report zero candidates and
  zero warnings, while all retained and observed browser rows remain preserved;
- live transactional apply still requires explicit authority and a fresh
  pre-apply readback.

## Turn 171 | 2026-08-23

Scope: reconcile access-plan profile compatibility with executable no-launch
preflight without launching a browser or changing installed or tenant state.

Actions:

- reproduced an account-selected `bill-soylei` plan borrowing a compatible row
  from another profile on the same host and executable;
- added one exact profile-host-executable predicate in the Service model;
- used the shared predicate in access-plan evidence selection and executable
  capability preflight;
- added a deterministic provider-free red-to-green regression fixture.

Validation:

- formatting, strict Clippy, 34 service-model tests, 41 access-plan tests, and
  4 capability-preflight tests pass;
- API/MCP parity, generated-client checks, client type checks, and diff hygiene
  pass;
- no browser, provider, tenant, profile, authentication, lease, or installed
  runtime effect occurred.

Result:

- Plan 0122 is complete at the source and provider-free validation boundary;
- access plans no longer overstate selected-profile compatibility;
- installed qualification remains a separate governed consuming-workflow step.

## Turn 170 | 2026-08-22

Scope: complete Plan 0117 installed convergence, preserve the authenticated
browser and durable handoff through hot upgrade, reclaim reviewed obsolete
runtime material, and close Plan 0121.

Actions:

- repaired cross-host handoff descriptor staging, canonical lifecycle identity,
  supervisor rebinding, rollback retention, and exact operator recovery;
- made shared-host idle lanes defer process retirement until generation cutover;
- made accepted cutover finalize transferred browser lanes before terminating
  the old host once through exact generation, socket, PID, and start identity;
- added prelaunch managed-owner admission and postlaunch persistence rollback;
- restored one finalized source lane by attaching its existing managed browser,
  without launching another browser or creating another durable handoff;
- accepted the exact optimized candidate, finalized the reviewed transaction,
  and removed only the obsolete rollback generation.

Validation:

- Rust formatting, strict Clippy, 85 workstation tests, 12 lifecycle tests, 8
  retention tests, and 11 supervisor tests pass;
- source-free install, host provision, fresh-VM, Guacamole asset, PostgreSQL
  durability, route-user synchronization, remote-view docs, and production docs
  gates pass;
- candidate SHA-256 is
  `aa21c5fe8a6dd75f1422bd84147756f984ea8662fc5d9a1ea3afac1c37eed452`;
- final install doctor reports one dashboard, one runtime host, one executable
  generation, zero legacy daemons, converged status, and no issues;
- durable handoff `r520477` resolves ready and returns HTTP 200 after cutover.

Result:

- Plan 0117 and Plan 0121 are complete;
- transaction `upgrade-52684512-bfc2-4c30-971b-ab166eaa5364` is accepted and
  finalized;
- service GC has zero candidates and generation GC removed only
  `0.28.0-bcfab70c2be9-7ad9e5b748d3`;
- the installed runtime is converged on
  `0.28.0-aa21c5fe8a6d-25828e3b8aed` with the authenticated browser preserved.

## Turn 169 | 2026-08-20

Scope: reconcile concurrent profile-owner maintenance into the Plan 0117
candidate and complete Slice I read-only admission without changing installed
or live state.

Actions:

- imported six compatible runtime handoff and workstation-recovery commits;
- retained Slice H's versioned lifecycle sidecar instead of importing forward
  fields into the legacy owner registry;
- made canonical browser profile identity repair a stale single-browser
  handoff lease while rejecting ambiguous multi-browser repair;
- replaced a race-prone GNU `yes` fake-browser fixture with deterministic exact
  process evidence;
- rebuilt the optimized candidate and repeated installed-state, resource,
  retention, generation, profile, and dry-run admission readback.

Validation:

- the official Rust cadence passes 1,358 parallel-safe tests, 57 intentional
  ignores, all integration scopes, and every serialized environment-sensitive
  partition;
- the Chrome module, strict Clippy, formatting, real isolated multi-lane hot
  handoff, and no-launch runtime-host convergence pass;
- candidate SHA-256 is
  `2431fdc51d44403bac5b9b26024ad3a6c405366ea3e26f8fa3c553dfab7dc523`;
- workstation dry-run reports a planned, non-mutating, supported admission with
  ready host and disk gates.

Result:

- the reconciled source candidate is admitted for Slice I;
- the installed runtime remains intentionally unchanged and is not healthy by
  Plan 0117 terminal criteria: three listener daemons, false duplicate profile
  leases, 26 generations, and 219 profile directories remain;
- process GC has zero candidates and retained-state pruning has only the inert
  `display-orphan` candidate;
- Slice I remains pending explicit live authorization.

## Turn 168 | 2026-08-20

Scope: accept Plan 0117 Slice H at the source, contract, deterministic-fixture,
and no-launch runtime boundary without performing Slice I installed
convergence.

Actions:

- moved lifecycle evidence into a versioned sidecar committed under the shared
  Service State lock and four-file recovery transaction;
- preserved the exact legacy `state.json` and `runtime-owner-registry.json`
  shapes for installed deny-unknown readers;
- aligned CLI runtime health, HTTP Service Status, typed MCP, generated client,
  schema, dashboard, help, README, repository skill, and docs site on additive
  `runtimeLifecycle` readback;
- derived multiplicity and owner counts from one reconciled snapshot and used
  monitor receipt time for deterministic cross-ingress observations;
- made lifecycle-store failures public-safe and made missing monitor or
  lifecycle authority degrade the overall status;
- isolated the confirmed-close test from process-wide durable Service State;
- kept the narrow privileged-helper and timer workflow distinct from P117
  installed-runtime acceptance.

Validation:

- the repository Rust cadence passes 1,352 parallel-safe tests, 57 intentional
  ignores, every integration scope, and all serialized environment-sensitive
  partitions;
- strict Clippy, Rust formatting, patch whitespace, lifecycle-store rollback,
  status projection, typed MCP, generated client, fixed-input parity, service
  parity, and cross-seam gates pass;
- isolated Service Status CLI and MCP, workstation source-free install, runtime
  host, dashboard, docs, Guacamole asset, PostgreSQL durability, route-user,
  and remote-view documentation fixtures pass without launching Chrome in the
  status smoke.

Result:

- Slice H is accepted at the source and isolated-runtime boundary;
- the timer is enabled and active after the separate helper workflow, and the
  authenticated browser remains preserved;
- no Slice I installed convergence or cleanup was performed by this slice;
- Slice I remains pending explicit live authorization.

## Turn 167 | 2026-08-16

Scope: close the remaining Plan 0116 deterministic legacy-daemon adoption gap
at the source and isolated-fixture boundary without changing installed or live
state.

Actions:

- classified only typed or exact textual unknown handoff commands as legacy
  protocol absence, while retaining fail-closed behavior for every other
  transaction failure;
- bound the recorded daemon process identity to the selected old-generation
  executable before revoking that exact process through the verified process
  handle;
- advanced only the exact registry owner ID and generation from `ready` to
  `orphaned` after daemon loss, preserving compare-and-swap protection against
  concurrent owner replacement;
- preserved the browser process and required the candidate to enter the
  existing process, profile, endpoint, target, and logical-identity orphan
  adoption proof seam without launch or navigation;
- skipped source finalization for orphan adoption and made rollback after
  irreversible legacy-daemon revocation or completed cooperative source
  finalization enter explicit operator recovery;
- aligned help, README, repository skill, docs site, plan, roadmap, and this
  runbook with the corrected legacy boundary.

Validation:

- all thirty-four workstation tests pass, including exact-daemon revocation,
  retained browser survival, protocol classification, orphan finalization, and
  recovery-state semantics;
- all fourteen owner-transfer tests pass, including exact owner-ID and
  generation fencing for the `ready` to `orphaned` transition;
- the repository Rust cadence passes 1,294 parallel-safe tests and every
  serialized environment-mutating partition;
- strict Clippy, Rust formatting, diff hygiene, the production docs build,
  remote-view and route-confusion guards, the optimized source-free workstation
  matrix, host and fresh-VM harnesses, Guacamole assets, PostgreSQL durability,
  and route-user synchronization pass.

Result:

- the deterministic legacy-daemon source gap is accepted at the source and
  isolated-fixture boundary;
- no installed payload, unit, daemon, browser, profile, route, display,
  dashboard, supervisor, finalization, generation-GC apply, or external state
  changed;
- Slice I remains unexecuted and requires separate explicit live
  authorization.

## Turn 166 | 2026-08-16

Scope: reopen and remediate the Plan 0116 Slices G and H source acceptance
after a closed-world audit, without changing installed or live state.

Actions:

- corrected postcommit validation so install doctor recognizes only its exact
  in-flight transaction while still rejecting every other stranded upgrade;
- replaced synthesized dashboard and operator summaries with an independently
  authenticated candidate `PresentationReceipt` gathered while a sealed
  generation-specific shadow backend runs behind stable ingress;
- preserved independent operational fallback and rollback dashboard authority,
  including the authenticated old-generation receipt, and made rollback stop
  and reap the shadow process explicitly;
- made payload status resolve the atomic `current` selector and aligned status
  plus doctor on seven readiness axes;
- made controlled shutdown prove exact child exit and wait read-only for the
  profile lock to disappear before relaunch;
- retained accepted rollback material until the explicit reviewed
  `install workstation finalize` transition, with GC still a separate dry-run
  and apply action;
- added direct deterministic coverage for postcommit presentation, dashboard
  receipt restoration, candidate commit concurrency, shutdown barriers, and
  every required generation-reference class;
- made the malformed-backend proxy fixture use a dedicated OS thread so its
  result tests HTTP normalization rather than Tokio scheduling latency;
- aligned help, README, repository skill, docs site, inline comments, plan,
  roadmap, and this runbook with the corrected lifecycle.

Validation:

- the repository Rust cadence passes 1,290 parallel-safe tests and every
  serialized environment-mutating partition;
- strict Clippy, Rust formatting, diff hygiene, thirty-one workstation tests,
  eight dashboard-ingress tests, runtime adoption and owner-transfer tests,
  focused shutdown and doctor guards, route-confusion gates, and remote-view
  documentation guards pass;
- the production docs build and the optimized source-free workstation matrix
  pass;
- the first broad raw Rust sweep exposed two parallel-only failures that
  passed serially. The prescribed partitioned cadence later exposed the
  dashboard fixture starvation repaired in this turn and then passed fully.

Result:

- Turn 165's source-acceptance evidence is superseded by this correction;
- Slices G and H are accepted at the remediated source and isolated-fixture
  boundary;
- Slice E remains `IMPLEMENTED_NOT_ACCEPTED` because its earlier user-scoped
  boundary incident is not adjudicated;
- no installed payload, unit, daemon, browser, profile, route, display,
  dashboard, supervisor, finalization, generation-GC apply, or external state
  changed;
- Slice I remains unexecuted and requires separate explicit live
  authorization.

## Turn 165 | 2026-08-16

Scope: complete Plan 0116 Slices G and H at the source and isolated-fixture
boundary without installed-payload, browser, or generation-GC mutation.

Actions:

- made fresh install and upgrade enter one durable, revision-fenced transaction
  before candidate staging;
- separated immutable candidate staging from selector commit and required two
  stable census rounds, host gates, admission drain, exact runtime-owner
  transfer receipts, and `CandidateReady` before commit;
- added deterministic precommit and postcommit rollback, explicit operator
  recovery when reversal cannot be proved, and typed blocked transactions for
  failed apply preconditions;
- added reviewed generation GC with separate dry-run and apply modes that retain
  selected, live-process, supervisor, failed, unclosed, and rollback-referenced
  generations;
- reduced the development publisher to a builder and canonical installer
  client;
- projected one redacted workstation-upgrade status through the CLI, runtime
  health, dashboard, and install doctor;
- aligned CLI help, README, repository skill, docs site, inline comments,
  fixtures, architectural guards, plan, roadmap, and runbook.

Validation:

- strict Clippy and Rust formatting pass;
- all twelve runtime-adoption tests and twenty-six workstation installer tests
  pass serially;
- dashboard and docs production builds, publisher guards, host and VM harness
  fixtures, Guacamole durability and route-user fixtures, route-confusion
  gates, and remote-view documentation guards pass;
- an exact-revision optimized build and source-free workstation fixture pass.
  The fixture covers transaction cardinality, selector preservation across
  injected failures, redacted status, stale admission drain, lock contention,
  and safe GC preview retention.

Result:

- Slices G and H are accepted at the source and isolated-fixture boundary;
- no installed payload, unit, daemon, browser, profile, route, display,
  dashboard, supervisor, or external state changed, and GC apply did not run;
- Slice E remains `IMPLEMENTED_NOT_ACCEPTED` because its earlier user-scoped
  boundary incident is not adjudicated;
- Slice I remains unexecuted and requires separate explicit live authorization.

## Turn 164 | 2026-08-16

Scope: implement Plan 0116 Slice F durable-handoff self-healing at the source
and isolated-fixture boundary without installed-payload or browser mutation.

Actions:

- changed ordinary durable resolution to remove stored navigation and
  ephemeral route selectors, require the exact retained target, and prohibit
  replacement target creation;
- routed missing-daemon recovery through the existing two-phase retained
  browser adoption seam without launch or navigation;
- removed the resolver and dashboard raw-provider fallback and made requested
  provider mismatch fail closed;
- added a monotonic durable presentation receipt bound to dashboard
  deployment, logical browser, daemon owner generation, process identity,
  target, route, display, and requested and observed provider;
- made the resolver and dashboard remain on the opaque durable URL with a
  retryable `converging` state until the exact receipt is ready;
- kept explicit reopen separate as the only path that restores the stored URL
  and may create or navigate a target;
- aligned CLI help, README, repository skill, docs site, inline comments, and
  source contract guards.

Validation:

- the focused retained-browser fixture proves the same opaque handoff across
  daemon loss, owner and process generation replacement, route and display
  replacement, and dashboard generation change with no launch, target-open,
  or navigation event;
- all fifteen coordinator tests and forty-five remote-view handoff tests pass
  serially, and thirty-four service-model contract tests pass;
- the broad parallel Rust sweep passed 2,058 tests with one unrelated
  control-plane monitor interval failure; that test passed immediately when
  rerun alone with one test thread;
- strict Clippy, Rust formatting, route-confusion gates, dashboard and docs
  production builds, service API and MCP parity, generated client checks, and
  handoff documentation guards pass.

Result:

- Slice F is accepted at the source and isolated-fixture boundary;
- one early positive fixture attempted the default user-scoped service-state
  process lock through the legacy tab-persistence path. The lock timed out and
  no state write succeeded. Reacquire-only target handling now bypasses that
  global persistence path, and the accepted rerun uses only its fixture
  repository;
- this source acceptance is not installed or live acceptance and does not
  adjudicate Slice E's earlier user-scoped file mutations;
- Slice G source work is next. Slice I still requires separate explicit live
  authorization.

## Turn 163 | 2026-08-16

Scope: implement Plan 0116 Slice E at the source and isolated-fixture boundary
without installed-payload or browser mutation.

Actions:

- separated stable dashboard ingress from generation-specific backends and
  added revision-fenced selected, candidate, fallback, and presentation state;
- bound stage and commit to live candidate runtime-manifest probes, retained
  the prior accepted backend after commit, and retried only safe requests;
- made split workstation and development-publisher flows preserve stable
  ingress while restarting or quiescing only generation backends;
- projected ingress and operator-journey readiness into runtime health and
  install doctor, with live missing journey proof reported as non-ready;
- added isolated dashboard auth-directory, backend-only, and relay-skip seams
  after a diagnostic boundary incident exposed inherited user-scoped paths;
- aligned CLI help, README, repository skill, docs site, inline comments,
  fixture expectations, plan, roadmap, and runbook.

Validation:

- seven ingress tests cover manifest binding, revision compare-and-swap,
  receipt derivation, failed candidate preservation, fallback continuity, and
  mutation no-replay;
- twenty-five workstation tests and the source-free workstation fixture pass;
- an isolated CLI transaction selected a candidate, terminated it, and served
  the same public manifest from retained fallback;
- strict Clippy, Rust formatting, publisher guards, help readback, and docs
  production build pass.

Result:

- Slice E is `IMPLEMENTED_NOT_ACCEPTED`;
- early diagnostic processes rewrote user-scoped `dashboard-auth.json` through
  normal startup, and shared service-state mtime advanced while lock contention
  was observed. No semantic before-and-after proof exists, so this turn does
  not claim a clean no-live-state boundary;
- exact leaked temporary dashboard processes were terminated. No installed
  binary, unit, browser, profile, route, display, or public handoff was
  intentionally changed;
- Slice F source work remains authorized. Slice I still requires separate
  explicit live authorization.

## Turn 162 | 2026-08-15

Scope: complete Plan 0116 Slice D at the source and isolated-fixture boundary
without installed or live runtime mutation.

Actions:

- replaced relinquish-first cooperative handoff with old-owner prepare,
  observation-only candidate resume, exact owner-generation commit, and stale
  old-owner finalize;
- added exact pre-commit abort plus receipt-bearing post-commit rollback, with
  reverse generation refresh restricted to a matching reverse receipt;
- added verified ownerless and legacy-descriptor orphan adoption through the
  same process, profile, endpoint, target-set, selected-target, and logical
  browser evidence seam without launch or navigation;
- preserved the existing logical browser record while rebinding its current
  daemon attachment, and fenced supervisor-restored old sessions as
  observation-only after candidate commit;
- removed implicit stale-daemon replacement from connection startup and made
  executable drift require the explicit two-phase protocol;
- updated the development publisher and runtime interlock for candidate
  sessions, abort, reverse, finalize, and active named-supervisor discovery;
- aligned CLI help, README, repository skill, docs site, plan, roadmap, and
  runbook with the two-phase command contract.

Validation:

- thirteen owner-transfer tests pass for exact commit, abort, replay, orphan,
  reverse, supervisor restart, generation refresh, identity preservation, and
  effect fencing;
- all ten runtime-adoption tests and the focused command, no-launch action,
  publisher, and interlock contract tests pass;
- strict Clippy and Rust formatting pass;
- no installed payload, daemon, browser, profile, route, display, dashboard,
  supervisor, or external state was changed.

Result:

- Slice D is accepted at the source and isolated-fixture boundary;
- the descriptor-bound orphan red seam is closed. Payload commit before full
  runtime preservation remains intentionally red for later installer work;
- the next packet is Slice E stable dashboard ingress and presentation commit.

## Turn 161 | 2026-08-15

Scope: begin Plan 0116 Slice D with a provider-neutral two-phase owner-transfer
foundation and no installed or live runtime mutation.

Actions:

- added the single locked service-state runtime owner registry using the P111
  profile identity, owner state, owner generation, process identity, browser,
  and daemon-route vocabulary;
- implemented observation-only candidate attachment followed by exact
  owner-generation compare-and-swap, with old-owner authority preserved until
  commit;
- added same-nonce idempotent replay, verified ownerless-orphan adoption
  through a generation-zero observation, and receipt-bearing reverse transfer
  at a new generation to prevent ABA;
- rejected manual preservation at the transfer boundary and retained existing
  manual and external no-automation classifications;
- added a transfer-bound daemon effect fence before stream broadcast, browser
  recovery, auto-launch, or action dispatch;
- changed the census profile-owner source to prefer canonical registry owners
  and use legacy session references only when no authoritative owner exists;
- froze seven cooperative, orphan, rollback, manual, external, and mismatch
  authority timelines in a provider-free JSON corpus.

Validation:

- ten owner-transfer tests pass for commit boundaries, replay, orphan adoption,
  reverse transfer, mismatch preservation, repository persistence, effect
  fencing, manual preservation, source ordering, and the frozen corpus;
- the ten runtime-adoption tests pass after owner-source integration;
- strict Clippy and Rust formatting pass;
- no installed payload, daemon, browser, profile, route, display, dashboard,
  supervisor, or external state was changed.

Result:

- Slice D has a coherent source foundation but is not accepted;
- current handoff prepare and resume remain descriptor-bound and
  relinquish-first, so their red source guards remain valid;
- the next packet must bind active supervisors plus old and candidate daemons
  to the registry protocol and turn those guards green.

## Turn 160 | 2026-08-15

Scope: complete Plan 0116 Slice C at the source and isolated-fixture boundary
without changing installed or live runtime state.

Actions:

- added observation-only adapters for all ten frozen census sources and joined
  duplicate browser, profile, session, PID, CDP, display, route, stream, and
  handoff aliases into one exact runtime candidate;
- reused P108 process ownership assessment and added a shared P111-compatible
  canonical profile identity digest that converges existing symlinks and
  not-yet-created leaves without persisting private paths;
- added bounded `/json/version` and `/json/list` probes that retain only browser
  identity and target-set digests;
- required one owner generation before classifying a cooperative live daemon,
  so current source remains fail-closed until P111 lands its owner registry;
- inserted the stable two-round census before user-unit quiescence and payload
  materialization, with a private atomic transaction receipt for both stable
  and blocked outcomes;
- exposed `runtimeCensusTransaction` in successful workstation JSON and updated
  CLI help, README, the repository skill, and installation docs.

Validation:

- ten focused runtime-adoption tests pass, including all-source join,
  duplicate-PID ambiguity, two-round drift, exact-once classification, and
  source ordering;
- canonical profile identity and bounded CDP digest tests pass;
- the pre-payload private transaction test passes and proves no generation,
  selector, or binary path exists when the census receipt is committed;
- focused workstation tests, strict Clippy, Rust formatting, source-free
  workstation install, host-provision, fresh-VM harness, Guacamole asset,
  PostgreSQL durability, route-user sync, remote-view handoff docs, and docs
  production build pass;
- no installed payload, live daemon, browser, profile, route, display,
  dashboard, supervisor, or external state was changed.

Result:

- Slice C is accepted at the source and isolated-fixture boundary;
- current live cooperative owners remain blocked until P111 supplies canonical
  owner generations, which is a safe `insufficient_evidence` outcome;
- the next packet is Slice D provider-free two-phase transfer and orphan
  adoption.

## Turn 159 | 2026-08-15

Scope: begin Plan 0116 Slice C with a provider-free stable-census engine and no
live runtime observation or mutation.

Actions:

- introduced typed source snapshots, normalized runtime candidates, census
  rounds, persisted records, and stable census reports;
- required the exact ten-source ledger, rejected duplicate or missing source
  attribution, and required the candidate set to equal the union observed by
  every source;
- compared two rounds and registry revisions, classified every runtime exactly
  once, and converted any round or revision drift to insufficient evidence;
- generated a canonical SHA-256 census digest and persisted migration records,
  transaction revision, checkpoint, state, and typed stop reason;
- rejected non-digest profile identity input so transactions cannot persist a
  raw profile path.

Validation:

- eight focused `runtime_adoption::tests` pass;
- Rust formatting and strict Clippy pass;
- Graphiti returned only advisory prior profile and foreign-CDP context; the
  current plan, fixtures, and source remained authoritative;
- no installed payload or live runtime state was read or changed.

Result:

- Slice C has a tested provider-free census and transaction foundation;
- Slice C is not accepted because the ten source read adapters and the
  pre-payload installer block are not implemented;
- the next packet is read-only source adaptation followed by two-round
  installer ordering proof.

## Turn 158 | 2026-08-15

Scope: execute Plan 0116 Slice B without changing the installed payload or live
runtime state.

Actions:

- replaced mutable workstation payload commits with sealed generation
  directories containing the binary, support assets, payload and generation
  manifests, and rendered unit templates;
- added stable command and unit links through an atomically replaced `current`
  generation selector, including rollback after an injected post-commit
  selector failure;
- made a no-op apply reuse the byte-identical generation, retained prior and
  failed candidate generations as complete immutable rollback assets, and
  rejected incomplete, writable, or externally targeted current selectors;
- kept payload materialization separate from runtime reconciliation and proved
  the standalone reconcile command does not alter the generation store or
  selector;
- replaced shallow inode helper tests with public installer-interface coverage
  across all seven staging and selector failure boundaries;
- reconciled one validation-only network race so the download diagnostic test
  accepts both refused and reset local connections while still requiring the
  underlying connection detail.

Validation:

- the expanded source-free workstation fixture passes first install, no-op
  reinstall, seven injected failures, selector rollback, changed-generation
  selection, old-generation retention, and reconcile non-mutation;
- the workstation host-provision, fresh-VM harness, Guacamole asset,
  PostgreSQL durability, and route-user projection fixtures pass;
- 23 focused workstation installer tests, three workstation payload-status
  tests, Rust formatting, strict Clippy, and `git diff --check` pass;
- the repaired connection diagnostic test passes ten consecutive isolated
  runs;
- one complete CI-partitioned Rust run passed every parallel-safe and serialized
  partition; repeated stress runs surfaced unrelated timing-sensitive baseline
  failures in download diagnostics, service inventory, remote-view fallback,
  and MCP invalid-input tests, while each failed case or module passed on an
  immediate isolated serial rerun;
- CodeGraph is current at 554 files, 19,801 nodes, and 68,606 edges.

Result:

- P116 Slice B is accepted at the source and isolated-fixture boundary;
- repeated full-suite baseline timing remains a nonblocking validation risk for
  later CI evaluation, not evidence of a Slice B generation failure;
- no installed payload, daemon, browser, profile, route, display, dashboard,
  supervisor, or external state was changed;
- the next authorized packet is Slice C closed-world runtime census and
  adoption decisions.

## Turn 157 | 2026-08-15

Scope: reconcile the Plan 0116 baseline failures and decide Slice A
acceptance without changing installer behavior or live runtime state.

Actions:

- classified `confirm` and `deny` as control-plane actions that skip browser
  launch, so confirmation decisions execute without starting Chrome;
- added denial coverage proving the pending confirmation is cleared with no
  browser present;
- repaired two legacy service-status fixtures with explicit ready-stream
  evidence so current reconciliation retains the browser rows whose URL
  compatibility behavior the tests exercise;
- preserved the current runtime-evidence rule that prunes unsupported legacy
  `ready` browser placeholders.

Validation:

- focused confirmation tests pass, including the new denial case;
- both previously failing service-status compatibility tests pass;
- Rust formatting, `git diff --check`, and strict Clippy pass;
- `scripts/ci/rust-tests.sh` passes the 1,194-test parallel-safe partition,
  all integration tests in that phase, and every environment-sensitive serial
  partition with no failures.

Result:

- P116 Slice A is accepted;
- installer, daemon, browser, profile, route, display, dashboard, installed
  payload, supervisor, and external state remain unchanged;
- the next authorized packet is Slice B immutable generation staging.

## Turn 156 | 2026-08-15

Scope: execute Plan 0116 Slice A without changing installer behavior or live
runtime state.

Actions:

- added the internal provider-free runtime-adoption authority model with the
  frozen `RuntimeGeneration`, `UpgradeTransaction`, `BrowserAdoptionReceipt`,
  and `PresentationReceipt` schemas;
- froze a closed ten-source runtime census ledger and thirteen deterministic
  fixtures spanning all eight runtime classifications, including PID reuse,
  identity mismatches, conflicting owners, external preservation, and a census
  that changes during classification;
- added two intentional red fixtures for payload commit before runtime
  preservation and for verified orphan adoption remaining blocked on an
  old-daemon handoff descriptor;
- bound both red fixtures to current source ordering so later production repair
  must deliberately change the expected seam;
- completed the one allowed broad architecture-drift review with no blocking
  finding: the fixture model remains compatible with P108 process identity and
  P111 profile digest and owner generation, creates no competing registry, and
  exposes no public surface.

Validation:

- five focused `runtime_adoption::tests` pass;
- JSON fixture parsing, `git diff --check`, Rust formatting, and strict Clippy
  pass;
- the broad Rust run reached 2,025 passing tests, 57 ignored tests, and six
  failures; isolated serial reruns cleared three race-sensitive failures;
- three tests remain reproducibly failing when the Slice A module registration
  is removed: `test_confirm_executes_once_and_restores_confirmation_gate`,
  `test_service_status_leaves_guacamole_root_without_route`, and
  `test_service_status_repairs_stale_guacamole_view_url`.

Result:

- P116 Slice A is source-implemented but not yet accepted because the broad
  baseline gate is not green;
- installer, daemon, browser, profile, route, display, dashboard, installed
  payload, supervisor, and external state remain unchanged;
- the next bounded packet is baseline-test reconciliation followed by Slice A
  acceptance, then Slice B immutable generation staging.

## Turn 155 | 2026-08-15

Scope: convert the wrong-daemon, wrong-executable, recurring interlock outage,
fresh-install corruption, orphan-browser, and durable-handoff convergence
discussion into one bounded runtime-adoption and transactional-upgrade plan.

Actions:

- traced workstation apply from payload staging through unit quiescence,
  reconciliation, service restoration, and final doctor;
- confirmed payload replacement currently precedes live runtime census or
  browser-owner transfer, while failure restoration covers unit active states
  rather than prior payload and ownership;
- reconciled the cooperative development publisher with its socket-only
  discovery, relinquish-first transfer, dashboard outage, and incomplete
  post-handoff rollback boundaries;
- connected the August 15 durable-handoff failures to effectful open replay,
  soft provider fallback, and the absence of one end-to-end presentation
  generation;
- opened Plan 0116 for immutable runtime generations, closed-world census,
  verified orphan adoption, two-phase owner transfer, continuous dashboard
  ingress, read-mostly handoff recovery, generation-bound presentation proof,
  and transactional rollback;
- limited the first implementation slice to provider-free red fixtures and
  contract vocabulary.

Validation:

- CodeGraph was healthy at 553 files, 19,654 nodes, and 68,296 edges;
- Graphiti `agent_browser_main` returned source-linked control-plane and
  durable-install direction, which was checked against current source and repo
  plans;
- plan links, roadmap linkage, markdown formatting, and scoped diff checks
  passed;
- no browser, profile, daemon, route, display, dashboard, installed payload,
  supervisor, or external state was mutated.

Result:

- P116 is open and indexed in the roadmap;
- the current installer and handoff behavior remain unchanged;
- the next bounded packet is P116 Slice A red fixtures for unsafe payload
  ordering and missing verified orphan adoption.

## Turn 154 | 2026-08-14

Scope: repair one terminal route-bound acquisition quarantine without weakening
the fail-closed boundary for live or ambiguous browser effects.

Actions:

- confirmed the Google Messages native supervisor and loopback endpoint are
  healthy while one August 13 focus timeout blocks every new acquisition;
- proved the quarantined browser, process identity, and session are absent and
  the matching route, display, and route-pool entry are terminal;
- opened Plan 0114 and extended `service_route_pool_repair` with exact
  `acquisitionLeaseId` scoping;
- added dry-run candidate reporting and apply-time promotion from
  `rollback_incomplete` to `rollback_complete` only when retained-state
  inactivity is fully demonstrated;
- added typed skipped reasons for live browser, process, session, route,
  display, and pool evidence, plus generated service-client support.

Validation:

- the red focused Rust fixture failed on the missing terminal recovery helper;
- five route-pool state tests, three repair-action tests, and the full service
  client suite now pass;
- broader validation, documentation gates, checkpoint install, and live exact
  recovery remain pending.

Result:

- P114 is source-implemented but not yet installed or applied;
- the existing Google Messages profile, browser family, route, authentication,
  and keyring posture remain unchanged.

## Turn 153 | 2026-08-14

Scope: implement, install, and live-validate Plan 0113 so the workspace
viewport connects a healthy managed browser without exposing provider and
lease mechanics as ordinary controls.

Actions:

- added a pure semantic source resolver and bounded automatic connection
  coordinator for ready-source selection, approved route recovery, and
  observer-lease acquisition;
- replaced duplicate provider buttons with a compact `View` menu and one
  connection state, retained one `Retry connection` fallback, and moved
  low-level operations into text-labelled `Advanced connection controls`;
- kept safely recoverable live browsers in active inventory and excluded
  recovered informational incidents from attention classification;
- updated help, README, the repository and installed Agent Browser skill, and
  remote-view documentation;
- published the release-mode checkpoint, synchronized workstation payload and
  the inactive supervisor manifest, and preserved the existing QBO browser
  process, profile, display, CDP endpoint, and RDP route.

Validation:

- focused dashboard source, projection, navigator, inspector, viewport,
  route-confusion, and handoff-documentation gates passed;
- dashboard and docs builds, Rust formatting, strict Clippy, focused CLI-help
  test, validation selection, and diff checks passed;
- installed runtime smoke passed with runtime health ready, route readiness
  ready, a live Guacamole frame, workspace state `controllable`, and QBO PID
  `51579` preserved;
- installed, release, and packaged Linux binaries share SHA-256
  `2ec94b993431da5c91db921250c2a7fadb363f2a9cb4bfa9a52e1b2712141452`;
  installed dashboard SHA-256 is
  `017b73508b33138b6b61bd552e26630db871daefb391dc760d41a3843d0e6d3e`;
- install doctor reports no binary, dashboard, workstation, or supervisor
  drift. The remaining duplicate-profile-pressure finding is the preserved
  P111 forensic issue.

Result:

- P113 is complete, installed, and live-smoked;
- the normal viewport no longer requires a provider-selection or viewer-lease
  click to expose the healthy QBO desktop;
- P111 remains the sole owner of duplicate-profile authority work.

## Turn 152 | 2026-08-14

Scope: simplify the workspace viewport so a healthy managed browser displays
through its best usable source without exposing provider and lease mechanics as
normal operator controls.

Actions:

- audited the focused and tiled viewport source controls, refresh and lease
  actions, automatic iframe retry, and workspace inventory classification;
- confirmed the QBO browser is healthy while its RDP stream can independently
  require safe route reattachment;
- found that provider-only source labels can be indistinguishable and that two
  adjacent circular-arrow buttons perform unrelated reload and lease effects;
- opened Plan 0113 for an automatic connection coordinator, compact semantic
  source menu, progressive disclosure, and honest browser versus stream state.

Validation:

- implementation has not begun;
- read-only service status and current source are the planning authority.

Result:

- P113 is in progress;
- the next action is the first red-to-green automatic source-selection slice.

## Turn 151 | 2026-08-13

Scope: repair the dashboard remote-view reconnect failure, validate all four
incident defects, and synchronize the committed source with the installed and
active runtime.

Actions:

- reproduced the retired `38215` port and mapped the current dashboard backend
  to `38017`;
- measured current service status at about 3.5 to 4.0 seconds against a generic
  two-second dashboard proxy allowance;
- found the persisted viewer reconnect job failed against pseudo-route
  `daemon:qbo-soylei` while the browser's authoritative RDP stream reports
  `guacamole:2`;
- confirmed no viewer lease was created by the failed request;
- confirmed current remote-view doctor evidence is ready across route pool,
  displays, Guacamole ingress, and runtime convergence;
- opened Plan 0112 for authoritative route selection, bounded remote-view
  request timeouts, typed error rendering, readiness wording, and synchronized
  installation;
- implemented and pushed the four repairs in `cadccee3`, with the prerequisite
  large-state stack hardening isolated in `885db6db`;
- published and installed the release-mode native binary and dashboard assets,
  synchronized workstation payload and session-supervisor provenance, and
  confirmed zero stale runtime executables;
- reattached the existing QBO route through the installed dashboard proxy and
  persisted an observer lease on `guacamole:2` without replacing browser PID
  `51579`, its profile, or its display;
- passed the installed dashboard workspace smoke with the RDP gateway viewport
  and ready runtime health.

Validation:

- focused TDD slices, dashboard tests and build, docs build, Cargo format,
  strict Clippy, the canonical Rust partition, selector-chosen gates, install
  readback, route readiness, and live viewer reconnect passed;
- install doctor reports synchronized binary, dashboard, workstation,
  supervisor, and runtime provenance. Its only remaining warning is the
  pre-existing P111 duplicate-profile-pressure evidence, for which retained
  cleanup reports no safe candidate.

Result:

- P112 is closed and installed;
- P111 remains the bounded owner of duplicate-profile authority and stale-row
  classification work.

## Turn 150 | 2026-08-13

Scope: convert the maintainer discussion about duplicate-profile pressure and
multi-agent browser sharing into a bounded development plan without changing
browser, profile, route, service, or installed runtime state.

Actions:

- reconciled the current profile reuse, exclusive lease, retained-browser tab
  acquisition, duplicate-pressure warning, process identity, and service-state
  locking seams;
- confirmed that P69 already defines one profile-owning browser with shared
  tabs, but leaves browser-owner reservation non-atomic and user-facing
  `profile lease` language broader than the contested resource;
- wrote Plan 0111 around one canonical profile owner, many accountable agent
  participants, per-tab mutation queues, browser-global coordination, and
  display-controller authority;
- froze atomic reserve, launch, finalize, and rollback semantics with owner
  generations and process-instance proof;
- required duplicate diagnostics to count root browser owners rather than
  Chromium child processes or evidence-poor retained rows;
- limited the first implementation packet to red fixtures and contract
  vocabulary, with controlled live validation deferred to a new noncritical
  temporary profile.

Validation:

- CodeGraph was healthy at 551 files, 19,498 nodes, and 67,830 edges;
- Graphiti `agent_browser_main` returned three facts, including the implemented
  P69 shared-profile routing episode, which was verified against the current
  plan and source;
- current read-only resource evidence showed one duplicate-profile warning for
  two evidence-poor attached-existing `default` rows and zero GC candidates;
- planning patch formatting and link checks passed;
- pre-existing uncommitted `daemon.rs` and `service_store.rs` implementation
  changes were not modified.

Result:

- P111 is open and indexed in the roadmap;
- no implementation, browser launch, profile mutation, process termination,
  cleanup, install, or live validation was performed;
- the next bounded packet is P111 Slice A red fixtures for profile identity,
  root ownership, reservation races, stale adoption evidence, and exact route
  validation.

## Turn 149 | 2026-08-12

Scope: open P110 and freeze the sequential desktop perception and interaction
foundation plus the detailed no-input PoC 1 contract before implementation.

Actions:

- reconciled the product vision with current browser, display-allocation,
  view-stream, route, attachability, and durable handoff authority;
- wrote umbrella Plan 0110 with five sequential proofs and required a new
  detail plan before each proof begins;
- wrote Plan 0110-1 for one canonical `desktop_capture` action returning a
  typed `DesktopContext`, `FrameReceipt`, and bounded ephemeral PNG payload;
- froze exact service-state selection, X11 provider boundaries, post-capture
  drift checks, typed failures, no-live authority, privacy posture, ingress
  parity, validation, and hard stops;
- used three shallow read-only reconnaissance agents for capture architecture,
  ingress parity, and provider-free fixture strategy; no agent edited files,
  touched live state, or spawned nested agents.

Validation:

- CodeGraph was healthy at 544 files, 18,687 nodes, and 64,526 edges;
- Graphiti discovery returned no P110-specific current contract, so current
  repo source and plans remained authority;
- the three reconnaissance packets were reconciled, validation selection was
  reviewed against the dirty worktree, and P110 planning patch checks passed;
- unrelated concurrent dashboard source and smoke changes remained outside
  the planning slice and were not modified or staged.

Result:

- P110 is open and PoC 1 is detailed before implementation;
- implementation remains source-only and blocked from live capture, display
  access grants, browser effects, image recognition, and desktop input;
- the next bounded packet is Slice A red provider-free fixtures and the
  internal capture provider boundary.

## Turn 148 | 2026-08-11

Scope: make the RDP remote-view handoff workflow explicit for agents and
operators without changing installed runtime, browser, profile, route, or live
service state.

Actions:

- added one canonical RDP remote-view guide centered on the authenticated,
  opaque `/remote-view/<handoff-id>` URL;
- added early hard rules to `AGENTS.md`, the README, and the agent-browser skill
  so agents require operator-visible readiness and share only `handoffUrl`;
- corrected guidance that described transient provider route URLs as operator
  handoff links;
- linked the guide from Quick Start, Commands, Dashboard, Service Mode, and the
  docs navigation;
- added a static regression gate and validation-selector mapping for the
  durable-link, readiness, reconnect, and raw-provider prohibitions;
- synchronized the repo skill with the installed shared skill copy.

Validation:

- the remote-view handoff documentation regression, service API and MCP parity,
  docs production build with all 35 pages, skill synchronization, selector
  self-check, and patch checks passed;
- the existing dashboard durable-handoff source regression remained red at
  `ab30c9f9` because its provider-preference assertion no longer matched the
  concurrently changed viewport source; this docs turn did not alter that
  source or test.

Result:

- agents now have one findable, task-shaped procedure for opening, handing off,
  and reconnecting an RDP browser without leaking or bookmarking a transient
  provider URL;
- no browser, daemon, doctor, install, route, profile, or dashboard runtime
  command ran.

## Turn 147 | 2026-08-11

Scope: execute the source-authorized Plan 0109 remediation derived from the
Google Messages and Facebook handoffs, without installing or touching live
sessions, browsers, profiles, routes, or downstream workflows.

Actions:

- made explicit-session global close a typed zero-effect rejection and bound
  ordinary daemon termination to recorded process identity;
- added validated Linux named-session supervision with a fixed stream port,
  no-browser startup, bounded restart policy, and typed health projection;
- separated requested-subject remote-view doctor readiness from preserved
  global host advisories;
- projected `Inspector.targetCrashed` into typed command failure, crashed tab
  lifecycle, event, and deduplicated incident evidence;
- required accountable attribution before effect-capable service requests and
  added cross-ingress no-launch collection coverage;
- completed a fresh-context structure review, corrected dispatcher ownership
  and process-consumer inventory, and isolated one ambient-HOME test fixture.

Validation:

- canonical guarded Rust exited zero: 1,071 parallel-safe tests passed, 57
  ignored, all three integration suites passed, and every serial partition
  passed;
- strict Rust formatting and Clippy, actions architecture and remediation,
  WSL Cargo safety, service client, API/MCP parity, no-launch collection smoke,
  route-confusion fixtures, dashboard tests and build, docs build, validation
  selection, and patch checks passed;
- every compiling Cargo command used the capped wrapper with four build jobs,
  20 GiB high memory, 24 GiB maximum memory, and 4 GiB swap maximum.

Result:

- Plan 0109 is source accepted at `c00c9655`;
- the recurring runtime interlock remains disabled;
- installed canary, unit activation, browser operation, protected-profile use,
  Chromium work, and downstream Google Messages, Facebook, or Last30Days retry
  remain separately authorized effects.

## Turn 146 | 2026-08-11

Scope: review the new Google Messages RDP handoff and the tracked Facebook
Blink-crash handoff, reconcile both with current source and completed plans,
and draft a source-only dependability successor.

Actions:

- classified the helper compatibility and opaque handoff recommendations as
  already implemented contracts requiring regressions, not redesign;
- confirmed current profile mismatch rejection, process-identity ownership,
  and read-only service collection handlers;
- confirmed four remaining gaps: ambiguous global close, missing named
  fixed-port daemon supervision, global doctor coupling, and absent
  `Inspector.targetCrashed` lifecycle propagation;
- identified a related close-path safety defect: an unreachable daemon can be
  force-killed from PID-file liveness without bound process-start identity;
- wrote Review 0109 and Plan 0109 with exact non-goals, public contracts,
  vertical slices, rollback, validation, and a separate installed canary gate;
- bound implementation authority to
  `docs/dev/plans/0109-2026-08-11-runtime-dependability-handoff-remediation-plan.md`;
- kept the recurring runtime interlock disabled and left Chromium repair in
  the separately governed Chromium repository.

Validation:

- CodeGraph was healthy with 535 files, 18,425 nodes, and 63,691 edges;
- Graphiti `agent_browser_main` discovery returned no current source-backed
  answer for the two handoffs, so current plans and source remained authority;
- no Cargo, browser, daemon, doctor, install, route, profile, or downstream
  retry command ran.

Result:

- Plan 0109 is planned with source-only authority;
- the first recommended implementation packet is the red zero-effect close
  scope regression and minimal parser rejection;
- installed and live acceptance remain unauthorized.

## Turn 145 | 2026-08-09

Scope: repair the upstream renderer and command-delivery boundaries exposed by
three exhausted Last30Days Facebook candidates, without consuming a fourth
provider attempt or changing the retained Facebook browser.

Actions:

- used raw page and flattened browser CDP sessions to distinguish a
  Facebook-target Runtime stall from browser inventory and transport health;
- added Chromium renderer deadlines, browser-level navigation metadata,
  cached evaluation metadata, and response-before-health worker ordering;
- removed the pre-action health probe from the caller-bounded path;
- added a Linux same-inode daemon identity fast path while retaining SHA-256 as
  the rebuild, upgrade, and unavailable-procfs fallback;
- published the debug candidate through browser-preserving executable handoff,
  refreshed the source-free workstation payload, and closed the exact
  disposable investigation sessions plus stale session metadata advertised by
  doctor remedies.

Validation:

- focused red and green renderer-deadline, target-metadata, worker-response,
  and executable-identity regressions;
- 34 browser tests, 27 control-plane tests, 35 connection tests, 260 action
  tests, and three CDP stream tests;
- service CDP tab streaming live smoke and route-confusion no-launch gates;
- Rust formatting, required production Clippy, dashboard production build,
  selected validation, patch checks, installed binary/reference parity,
  install doctor, and remote-view doctor;
- the canonical partition passed 1,220 parallel-safe tests and the touched
  serial partitions before the unrelated untouched Chrome test
  `test_headed_display_fallback_not_used_when_display_set` rejected the current
  production behavior of returning `DISPLAY=:9`;
- broader all-target Clippy also surfaced twelve pre-existing test-only lint
  rejections. Required production Clippy passed.

Result:

- installed executable and reference SHA-256 are
  `17f393c716f63de5008a25045f1ead0a4377efb7936300c8e1bcce2247d5995b`;
- installed baseline evaluation returned `2` in 455 ms, the infinite loop
  returned a typed CDP failure in 4.546 seconds under a 6-second outer guard,
  and immediate recovery returned `2` in 1.482 seconds;
- install doctor is clean, runtime convergence is `converged`, remote-view and
  remote-control status are ready, and the dashboard payload is current;
- retained Facebook PID 63205 remained ready and was not restarted or closed;
  no Last30Days provider attempt was submitted.

Graphiti Write Status:

- compact source-backed closeout job
  `64a8beb5-806d-457a-84c6-2c7e4c51449d` was queued in
  `agent_browser_main`, then timed out after 120 seconds with no episode UUID;
  the durable memory write is therefore unconfirmed.

## Turn 144 | 2026-08-08

Scope: close the Last30Days sequential-social handoff with downstream installed
acceptance, without claiming an agent-browser source repair.

Actions:

- reconciled note 0098 with the retained agent-browser job IDs from installed
  Last30Days 0.3.27 and the accepted 0.3.28 proof;
- confirmed both prior tab-switch jobs succeeded after their 8-second callers
  exited, while the fresh eval independently reached its 15-second worker
  timeout;
- recorded the exact client contract proven live: 3-second retained worker jobs
  with 15-second callers, and 30/45 seconds only for a fresh auth target;
- bound the downstream result to pushed Last30Days commit
  `24474f62e5e11f1c51d5ab5adf0f0933764dce91`.

Validation:

- manual tick `tick-f273eb12d642b31d49a7f12959b93b87` accepted Facebook;
  attempt `provider-attempt-5e5205b623e52dfd122dbbf2e4e668af` observed 19,
  accepted two, rejected 17, and every browser operation succeeded;
- PID 63205 remains ready on canonical `session:last30days-facebook` with 17
  tabs; no duplicate browser launch or retained-tab closure occurred;
- no agent-browser source, generated client, dashboard, docs command surface,
  or installed executable changed in this closeout.

Result:

- the cross-repo blocker is closed as a downstream deadline-contract repair;
  queue timing observability may remain future nonblocking backlog.

Graphiti Write Status:

- pending the shared closeout memory after both repository commits are pushed.

## Turn 143 | 2026-08-08

Scope: investigate the Last30Days sequential X-to-Facebook evaluation timeout
without mutating retained PID 96078.

Actions:

- confirmed Plan 0097's timeout cleanup and queue-release repair is already in
  `origin/main`;
- separated the caller deadline from the dispatched worker deadline and found
  that Last30Days allowed only 5 seconds of grace despite observing successful
  commands taking 8.2 to 8.4 seconds in the same attempt;
- found later ownership drift: PID 96078 remains ready as
  `session:plan0058`, while `last30days-facebook` is an alias to that browser;
- wrote the privacy-safe cross-repo investigation note and downstream alias
  routing acceptance contract.

Validation:

- source inspection confirms `jobTimeoutMs` starts after worker dequeue;
- installed service state confirms PID 96078 is ready with 17 retained tabs;
- a diagnostic bare `tab list` accidentally auto-launched PID 47946 on the
  drifted daemon; exact attribution was established and only PID 47946 was
  closed. PID 96078 remained ready and unchanged.

Result:

- no agent-browser source repair is justified by the retained evidence yet;
  the immediate proven repair belongs in Last30Days deadline layering and
  exact alias-owner routing;
- note 0098 is the durable agent-browser investigation handoff.

Graphiti Write Status:

- no write; note 0098, current source, installed jobs, and service readbacks are
  sufficient durable evidence.

## Turn 142 | 2026-08-08

Scope: verify installed timeout/queue behavior during the explicit Last30Days
manual governed tick before pushing the completed remote-browser repair.

Actions:

- observed the installed Last30Days Facebook lane skip two frozen retained tabs,
  switch to a responsive tab, and complete bounded auth evaluation;
- observed query navigation job `r198316` fail at the page-operation timeout;
- verified the serialized queue released and later tab, eval, navigation, and
  service reconciliation jobs completed successfully;
- preserved retained Chrome PID 96078 and all eight tabs. No browser or tab was
  opened or closed.

Validation:

- manual tick `tick-848f61b8a22d7e603c7e473c16ba5fdf` terminalized
  `complete_degraded` with seven items, zero cost/model use, zero incidents,
  and zero notifications;
- failed job `r198316` reports the ordinary page-operation timeout rather than
  a queue stall; subsequent jobs `r998619`, `r498581`, `r461931`, `r922601`,
  `r422781`, `r500611`, and `r617918` all succeeded;
- current executable SHA-256 remains
  `e899753a27005a79fe820f9128420eb0ea80ed8ea59a8719c64d9bc14c278d5f`.

Result:

- Plan 0097 remains closed: cancellation and queue release are proven in the
  ordinary governed path. Last30Days now owns the separate post-auth navigation
  recovery in Plan 0027.
- Changes are ready for scoped commit and push; unrelated `--full-page` and
  declaration-campaign evaluation artifacts remain excluded.

Graphiti Write Status:

- no new write; agent-browser jobs plus both repo runbooks are the durable
  source-backed evidence.

## Turn 141 | 2026-08-08

Scope: complete durable authenticated remote-view handoffs, then repair ordinary
CLI timeout layering, retained-target handoff recovery, and interrupted-command
browser ownership for the Last30Days Facebook lane.

Actions:

- Added opaque authenticated `/remote-view/<handoff-id>` URLs, sidecar-backed
  durable handoff persistence, authenticated dashboard resolution, bounded
  route reacquisition, and explicit-close fail-closed behavior.
- Added global `--job-timeout-ms` parsing and top-level command JSON carriage
  while preserving action-specific timeout values and invalid-input rejection.
- Made runtime handoff persist an optional preferred target and probe retained
  targets under bounded Page, Runtime, and Network domain initialization,
  retaining schema-v1 backward compatibility.
- Changed control-plane cancellation cleanup so only an observed process exit
  discards browser ownership; a timeout or cancelled future preserves a live or
  reconnectable BrowserManager.
- Updated CLI help, README, docs site, source skill, and installed shared skill.
- Published and converged the release-mode executable without replacing the
  retained Last30Days browser.

Validation:

- durable URL, dashboard auth, handoff sidecar, legacy-writer, resolver,
  explicit-close, generated-client, and isolated live reacquisition gates
- focused red/green parser, command JSON, timeout queue-release, legacy
  handoff, target ordering, same-profile guard, and interruption cleanup tests
- canonical Rust suite: 1,789 passed, 57 ignored, zero failed
- Rust formatting, strict Clippy, dashboard/docs builds, patch checks,
  installed runtime and skill convergence, install doctor, and remote-view
  doctor
- live one-second never-resolving eval followed by a 466 ms tab-list command;
  retained PID 96078, seven tabs, and active index 3 were unchanged during the
  agent-browser proof

Result:

- Plans 0096/P96 and 0097/P97 are closed. Installed executable SHA-256 is
  `e899753a27005a79fe820f9128420eb0ea80ed8ea59a8719c64d9bc14c278d5f`;
  runtime convergence and `remoteControl.status=ready` are current.
- The first timeout proof exposed the prior unconditional cleanup branch and
  launched default-profile Chrome PID 97130. Exact daemon ownership and start
  time were proved; only that repair-created browser was closed. Retained PID
  96078 was never closed or restarted, and no default-profile Chrome remains.
- A later Last30Days proof opened one additional Facebook target, so current
  browser readback is eight tabs at active index 5; the final Last30Days repair
  preserves that complete set.
- Changes remain local and uncommitted. Unrelated untracked files, including
  `--full-page`, were untouched.

Graphiti Write Status:

- one compact closeout write was attempted as job
  `7e1b6b06-e449-4668-b469-99118eb1f14b`; it timed out after its explicit
  120-second bound before creating an episode UUID. No retry was submitted;
  this runbook, Plan 0097/C04, and installed readbacks remain authoritative.

## Turn 140 | 2026-08-07

Scope: repair the Last30days single-route remote-control readiness false block
without hiding duplicate-profile pressure or weakening target-profile launch
guards.

Actions:

- Added a fail-closed classifier that distinguishes raw embedded install-doctor
  success from effective single-route remote-control install readiness.
- Accepted `service_duplicate_profile_pressure` only when it is the complete
  structured issue set and readiness-impacting resource candidates equal zero.
- Applied the shared classification to nested remote-control status, top-level
  next-action recommendation, and remote-view issue projection.
- Added `installDoctorReady`, effective `installReady`, and
  `nonBlockingInstallIssueCodes` to the JSON contract while preserving the
  complete embedded install report.
- Updated CLI help, README, docs-site guidance, repo skill guidance, and the
  installed shared skill.
- Published and converged the installed `0.28.0` checkpoint without opening or
  closing a browser.

Validation:

- observed-red then green cross-seam regression matching the original two
  warning and zero readiness-candidate payload
- fail-closed mixed, malformed, timeout, and positive-candidate cases
- all 44 remote-view doctor tests and the same-profile retained-browser guard
- canonical partitioned Rust suite, Rust formatting, strict Clippy, docs
  production build, patch hygiene, installed help readback, and skill sync
- installed workstation payload, dashboard manifest, runtime convergence,
  remote-control readiness, daemon SHA-256, browser PID, URL, and service-state
  readbacks

Result:

- Plan 0095 is closed. Current remote control reports all single-route
  prerequisites ready and recommends `run_remote_view_open_live_gate`.
- The installed executable SHA-256 is
  `8582bf0900b4d974994846c4ff3985746dcbbf5ee2136699f68e56ea5e73726b`.
- Publish reported a `p0065` resume error after the replacement daemon had
  already attached. Direct readback proved the installed daemon still owns
  original browser PID `19675` and its existing URL. The LitScout lane also
  retained its existing ChatGPT target, and all four pre-publish ready browser
  records remain ready.
- Current resource warnings are zero because inactive daemon listeners were
  retired during publish. The exact original duplicate-pressure branch remains
  covered by its regression fixture.
- No browser was opened or closed, no profile lease was released, and no
  resource cleanup was applied. At this checkpoint the untracked `--full-page`,
  declaration-campaign evaluation artifacts, and Last30Days handoff note
  remained uncommitted and untouched; the handoff note is included only in the
  later scoped P95-P97 integration recorded by Turns 141-142.

## Turn 139 | 2026-08-06

Scope: open the exact guided launcher from actionable profile rows and close
any live service-owned browser from the dashboard without extending lifecycle
ownership to detected external browsers.

Actions:

- Routed profile-row Open browser through the existing browser/profile launcher,
  preserving exact profile selection and automatic no-launch access planning.
- Made workspace browser Close contract-aware and wired its confirmation to
  `service_browser_close` for browser records that do not carry daemon ports.
- Enabled Close in the Service browser table for every live service-owned
  browser instead of only the currently selected daemon-session browser.
- Added browser-specific confirmation copy that explains polite shutdown and
  retained lifecycle history.
- Preserved disabled lifecycle controls for detected non-owned browsers and
  explicit contract-unavailable reasons for service-owned rows.
- Updated CLI help, README, dashboard and service-mode docs, source skill, and
  installed shared skill, then published the embedded dashboard.

Validation:

- focused red then green workspace-node, workspace-navigator, and Service
  browser-table contracts
- dashboard view-stream, rendered browser-row action, selected-context,
  selected-chat, selected-console, launcher, and inspector tests
- route-confusion gates, 34 focused Rust output tests, Rust formatting, strict
  Clippy, dashboard and docs production builds, patch hygiene, and installed
  runtime smoke
- live dashboard inspection of all 485 stored profiles across 1,952 launcher
  combinations, exact disposable-profile preflight, disposable service-owned
  browser Close confirmation, session removal, and local/public HTTP 200

Result:

- Plan 0094 is closed. Actionable profile rows can wake the exact guided
  launcher, and live service-owned browser tiles can be politely closed from
  both lifecycle surfaces.
- Profiles without compatible browser evidence remain visible but blocked by
  the existing safety gate. Adding reviewed evidence from the UX is the next
  recommended slice.
- The installed dashboard SHA-256 is
  `b2d74b07f2d649f34858c67e3830fc41427818cbbbb4a6fb4b75b0c56fabbb16`;
  the executable SHA-256 is
  `32af83cf90e0940183f83e4e7f02ecd4f1b3b6ffaada96d6863f866c6485e3be`.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 138 | 2026-08-05

Scope: repair stored-profile launcher identity handling and open two bounded
stored-profile browser workspaces without disturbing retained browser owners.

Actions:

- Repaired nested browser-capability preflight parsing so explicit
  browser-build, runtime-profile, custom-profile, and headed flags survive
  global flag cleaning.
- Added exact compatibility rows for the Last30days stealth Chromium profile
  and the AuraCall stock Chrome profile, then proved both installed no-launch
  preflights selected passed validation evidence.
- Changed dashboard launcher session arguments to pass both the service
  runtime-profile identity and exact custom profile path when access-plan
  provides both.
- Launched `stored-last30days-social` on its exact profile as PID `90765`, CDP
  port `37077`, and display `:90`; direct CDP and screenshot readbacks showed
  the authenticated Facebook feed.
- Launched `stored-auracall-chatgpt` on its exact registered AuraCall path as
  PID `95241`, CDP port `38441`, and display `:91`; direct CDP and screenshot
  readbacks showed ChatGPT loaded but logged out.
- Released both Guacamole route checkouts created by rejected shared-display
  attempts. Route A and Route B returned to available. Preserved the existing
  `litscout-e3-auracall` CDP endpoint.
- Published the final embedded dashboard and executable. Both new browsers
  remain running.

Validation:

- focused red then green Rust preflight parser regression
- focused red then green dashboard launcher argument regression
- Rust formatting, strict Clippy, focused Rust tests, dashboard and docs
  production builds, patch hygiene, and installed preflight readbacks
- direct process command-line, runtime-state, CDP target, HTTP screenshot,
  service profile allocation, stream, dashboard local/public HTTP 200, route
  release, and retained-browser readbacks

Result:

- Plan 0093 is closed. Stored-profile launches now retain both service identity
  and exact browser-state paths, and the two requested browsers are available
  as controllable CDP stream workspaces.
- The installed executable SHA-256 is
  `2c07c043a2af5a7063a161159f856c1e9c3974e31ceaf95300f2a46383fae32b`.
- AuraCall's retained fresh ChatGPT authentication evidence is stale relative
  to the current logged-out page. Login repair remains a separate authorization
  boundary.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 137 | 2026-08-05

Scope: make detected non-owned CDP browser tiles useful for capture, responsive
watching, and explicit time-bounded interaction without taking lifecycle
ownership.

Actions:

- Added functional Screenshot and Watch live actions for foreign CDP page
  targets. Watch refreshes a bounded PNG frame every 750 milliseconds.
- Added authenticated Borrow status, grant, fixed-input, and Release endpoints.
  Grants bind one superuser, CDP port, and live page target for five minutes by
  default and no more than fifteen minutes.
- Restricted borrowed input to server-built pointer, keyboard, and wheel CDP
  commands. Arbitrary CDP, navigation, evaluation, Close, Kill, profile
  release, and adoption remain unavailable.
- Added viewport pointer, drag, wheel, and keyboard forwarding plus visible
  grant expiry and Release controls. Foreign rows no longer expose Close or
  Kill affordances.
- Updated help, README, dashboard docs, the agent-browser skill, runtime feature
  markers, tests, Plan 0041, and the roadmap.
- Published the embedded dashboard and executable while preserving retained
  browsers and daemon handoffs.

Validation:

- dashboard workspace navigator, view-stream, workspace-node, inspector,
  selected-context, chat-packet, console, service parity, service-client
  contract, and JavaScript type suites
- dashboard and docs production builds
- focused foreign-CDP Rust tests, formatting, strict Clippy, and the canonical
  partitioned Rust suite
- installed runtime marker smoke for `Borrow control`, `Capture PNG`, and
  `workspace.foreignCdpBorrow`
- disposable foreign Chrome proof of capture, Borrow, mouse and keyboard input,
  wheel input, Release, post-release HTTP 403, continued process health, and
  unchanged `foreign_cdp` ownership

Result:

- The installed dashboard at `http://127.0.0.1:4848/` can capture, watch, and
  temporarily interact with reachable non-owned browser targets without
  claiming their lifecycle.
- The installed dashboard SHA-256 is
  `d215c9b5fe7fc731abff307db240e383c31811060254d2a58514e3b4059d8cb4`;
  the executable SHA-256 is
  `f5d0c1ef6220415671f6e756e56bf9f18c6c6b5ade884ffbfacb0a4b264510fd`.
- Install doctor reports runtime convergence and the dashboard ready, but
  remains nonzero for the separate
  `workstation_payload_partial_or_drifted` source-free workstation payload
  issue. This slice did not broaden into workstation repair.
- Plan 0041 remains active for a native CDP screencast and durable Service or
  Activity audit history. The current Watch feed is a responsive screenshot
  stream.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 136 | 2026-08-03

Scope: make recurring workstation privilege actions passwordless through the
narrow installed helper, enable Guacamole text input by default, and restore
the named Route A browser after live drift.

Actions:

- Replaced byte-only helper readiness with root-owned capability checks while
  retaining exact helper provenance as advisory evidence.
- Removed direct sudo fallbacks from route-user and display-access maintenance.
- Made the managed Chrome AppArmor policy conditional on a kernel where
  AppArmor and restricted unprivileged user namespaces are both active.
- Added and packaged a Guacamole JavaScript extension that performs a
  versioned one-time browser-origin migration to text input while preserving
  later operator overrides.
- Published a prechange PostgreSQL backup, mounted the extension, and recreated
  only the Guacamole web container. PostgreSQL and guacd were not recreated.
- Found `wsl-chrome-3` with a dead retained DevTools port, then performed the
  requested Route A recovery on its canonical `:10` allocation and reused its
  restored ChatGPT target.

Validation:

- privilege clean and host-provision regressions, including compatible helper
  drift, compatible AppArmor annotation drift, and AppArmor-disabled WSL
- Guacamole asset, workstation install, fresh VM, PostgreSQL durability, and
  route-user fixture suites
- Rust workstation-install tests, formatting, strict Clippy, docs build, and
  selected validation
- live no-prompt privilege apply returned no privileged changes needed
- Guacamole logged the defaults extension loaded; served application code
  contains the migration; a fresh origin read back `inputMethod: text` and
  migration version `1`
- PostgreSQL retained 2 connections, 22 parameters, and 6 permissions; public
  ingress returned the expected authentication redirect and HTTP 200 login
- `wsl-chrome-3` operator-visible proof returned browser, display `:10`, Route
  A, stream, selected ChatGPT target, and operator access ready

Result:

- Guacamole text input is the live default for each browser origin on its first
  load after this deployment. A later user-selected input method is retained.
- Recurring route maintenance uses only the fixed passwordless helper. Missing
  or incompatible root state still stops at the one-time bootstrap boundary.
- The installed 0.28.0 candidate remains unchanged to avoid invalidating active
  daemon owners. Plan 0091 retains the coordinated candidate-install and
  remaining-daemon handoff gate; four stale daemon owners remained at final
  readback, and its recurring timer stays disabled.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 135 | 2026-08-03

Scope: restore the public agent-browser and Guacamole routes, repair the
systemd interlock self-quiesce defect, and attempt one bounded installed proof.

Actions:

- Restored missing Guacamole READ grants for the three governed users on
  connection ids `1` and `2` from a prechange PostgreSQL backup.
- Diagnosed the external HTTP 502 as the dashboard service being stopped by a
  timer-triggered reconcile that included its own running interlock service in
  the systemd stop request.
- Added a red regression for the quiesce set, removed only the running service
  from that set, and retained the dashboard, interlock timer, and PostgreSQL
  backup timer.
- Built and materialized the corrected `0.28.0` payload. The apply then stopped
  at the existing interactive sudo gate for the root-owned privilege helper.
- Preserved every active daemon and browser session. Restored the dashboard and
  backup timer directly, and kept the recurring interlock disabled fail-closed.
- After the operator reported `wsl-chrome-3` unreachable, confirmed its Chrome
  process was absent, relaunched the durable profile once on Route A display
  `:10`, and reattached the restored ChatGPT target through the browser's
  canonical display allocation.

Validation:

- focused red then green Rust regression
- fifteen workstation install Rust tests
- workstation install fixture, host provision, fresh VM harness, Guacamole
  assets, PostgreSQL durability, and route-user sync tests
- Rust formatting, strict Clippy, selected validation, and release build
- installed executable and payload manifest SHA-256
  `23e71f0ffd8e75355719896a71d09849f57bf6c7e5c417eaf366e8489405d684`
- dashboard local and public HTTP 200; Guacamole readiness `ready`
- connections `1` and `2` each at three of three READ grants; route displays
  remain `:10` and `:11`
- dashboard active, backup timer enabled and active, interlock timer disabled
- `wsl-chrome-3` browser healthy, visible on `:10`, Route A ready, and the
  Guacamole operator URL HTTP 200

Result:

- The external dashboard and exact Guacamole Route A URL are reachable again.
- The source defect is repaired and the corrected payload is installed, but
  Plan 0091 is blocked at the installed runtime gate. Install doctor reports
  nineteen stale active daemons after `wsl-chrome-3` moved to the corrected
  runtime.
- The remaining gate requires interactive sudo and an owner-coordinated handoff
  of all active daemon sessions. Do not re-enable the recurring interlock until
  one installed pass exits successfully.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 134 | 2026-08-02

Scope: reconcile the installed P90 workstation payload and prove the repaired
route-bound display path before returning authority to last30days.

Actions:

- Accepted the operator-completed interactive workstation reconciliation and
  re-read installed provenance rather than assuming the source push implied
  runtime readiness.
- Ran both installed-runtime doctors and confirmed executable, daemon,
  dashboard, route-pool, route-display, and external-ingress convergence.
- Ran the repository live fixture. Its route-bound visible-window proof passed,
  but its in-process HTTP fixture could not serve while the same Node process
  blocked on the synchronous CLI child, so navigation remained `about:blank`.
  Cleanup released every disposable resource.
- Ran the installed binary directly against the already-running dashboard on
  disposable Route B. It loaded the target, returned a direct external
  Guacamole URL, and aligned route, display, browser window, operator-visible,
  and attachability proof before clean close.

Validation:

- install doctor: success, zero issues, executable SHA-256
  `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`,
  one converged daemon, dashboard ready
- remote-view doctor: ready, runtime converged, both `:10` and `:11`
  accessible, route pool and external Guacamole ingress ready
- direct gate: job `r63183`, `remote_view_open` succeeded on
  `guacamole-rdp-b` / `guacamole:2` / `:11`; target dashboard loaded in one
  readback with `browser_window_visible`, operator-visible ready, and
  attachability ready
- post-close remote-view doctor remained ready and both route allocations were
  free

Result:

- P90 is closed both in source and in the installed runtime. The original C63
  route-bound proof defect is repaired and live-proven.
- The fixture self-server/synchronous-child interaction is a separate harness
  defect; it consumed no last30days source attempt and left no retained
  resource.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

Next bounded action:

- Return to last30days Plan 0018 and review one fresh X successor identity.
  Preserve the durable `last30days-facebook` binding and require any genuine
  human handoff to use the direct external Guacamole URL.

## Turn 133 | 2026-08-02

Scope: finish the reviewed P90 runtime installation and publish the repo fix
after the litscout owner paused its workflow.

Actions:

- Confirmed the blocking `litscout-plan0311` daemon had exited.
- Re-ran the no-browser local publisher. It installed executable SHA-256
  `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`,
  restarted the dashboard, passed dashboard smoke, and reattached ten retained
  targets in `litscout-0312`.
- Ran install doctor, which failed closed on workstation-payload provenance:
  the source-free manifest still records the previous executable and the
  installed root-owned privilege helper differs from the bundled helper.
- Attempted the doctor-prescribed workstation reconciliation. It stopped at
  the interactive sudo gate without changing the root-owned helper.

Validation:

- installed, built, and checkout-reference binaries share SHA-256 `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`
- dashboard runtime manifest reports the same executable SHA-256
- dashboard service is active and its no-browser smoke passed
- install doctor remains non-ready until interactive workstation reconciliation

Result:

- The reviewed P90 executable is installed, but the full workstation payload
  is not provenance-converged. Do not consume a last30days source proof until
  interactive reconciliation and install doctor pass.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 132 | 2026-08-02

Scope: diagnose and repair the last30days route-bound visible-window proof
timeout without launching another browser or consuming a source attempt.

Actions:

- Read the two retained failed acquisition leases. Both selected the expected
  `last30days-facebook` profile, route `guacamole:2`, and display `:11`, then
  failed after twenty-one visible-window probes reported
  `display_probe_unavailable`.
- Confirmed both Chrome processes launched on the route-B environment and that
  current `xwininfo` probes can read displays `:10` and `:11`.
- Removed redundant ambient route discovery from the already normalized,
  exact-display proof path while retaining that guard for unbound probes.
- Preserved a sanitized, bounded underlying display-probe reason through the
  visible-window proof and AI-friendly error renderer.
- Independent review found the initial probe-error test did not lock down
  sanitization and truncation. Reworked it with multiline, quoted, over-limit
  input and exact assertions for single-line output and the 240-character cap.

Validation:

- forty-seven focused display-proof and error-rendering tests
- thirty remote-view unit tests after the independent-review rework
- all twenty-nine `remote_view_open` tests
- Rust formatting and strict Clippy with warnings denied
- complete Rust suite: 1,755 passed, 57 ignored, two state-sensitive failures;
  the authenticated-target case passed alone, while the unknown-command case
  remains intercepted by installed browser-recovery retry state before its
  expected assertion
- independent re-review passed exact commit `116ee810`
- no-browser publisher built candidate SHA-256 `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`
  but rolled back when active daemon `litscout-plan0311` did not exit; the
  resumed prior runtime then passed install doctor with all active daemons
  matching installed SHA-256 `cc22abe43a069e55e2dd46598b3eaa4954ffd4b8859388f646d7761c6c05da60`

Result:

- P90 is closed with independent PASS. The repo repair is implemented and
  validated, while installation is deferred until the owning litscout workflow
  can quiesce its active daemon.
- No browser or last30days source attempt was launched.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 131 | 2026-07-30

Scope: finish exact-head release gating, publish `v0.28.0`, and prove the
public artifact.

Actions:

- Bound the release decision to candidate `412684f6`, exact-head fast CI
  `30552821524`, full CI `30553477964`, and an independent fresh evaluator.
- Merged PR 7 as `80f64885`.
- Let dry run `30575205599` fail closed when its inventory included the
  repository JavaScript shim beside the seven release binaries.
- Used the single authorized remediation to isolate workflow asset staging.
  Commit `4132e782` passed exact-head full CI `30576313066`, corrected dry run
  `30578774481`, and publication run `30579564702`.
- Downloaded the public Linux x64 binary and checksum manifest, then performed
  a source-free idempotent reinstall on the accepted disposable Ubuntu VM.

Validation:

- Public tag `v0.28.0` resolves to exact commit `4132e782`.
- The public and installed Linux x64 binaries share SHA-256
  `4af2aba4e3670b2ffcd9601ab0134ad24cd13ec9e8131212f42a5645cb9baa22`
  and report version `0.28.0`.
- Install doctor, remote-view doctor, and the no-launch Route A dry run passed.
  The dry run requested no browser launch, route checkout, or tab open.

Result:

- P82 is closed and `v0.28.0` is the supported public workstation baseline.
- Detailed evidence is in
  `docs/dev/notes/2026-07-30-v0-28-0-release-validation.md`.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 130 | 2026-07-30

Scope: repair the next exact-head Windows and native E2E release defects.

Actions:

- Confirmed exact-head fast CI run `30549724644` fully green at commit
  `98316d14`.
- Dispatched full CI run `30550334355`. Windows found three native-path
  assertions, two inventory classifications hidden by a non-Unix process
  liveness stub, and one repository test whose `HOME` isolation is
  Unix-specific.
- Changed path fixtures to compare native paths, constrained the
  Unix-specific repository isolation test, and implemented Windows process
  liveness through the existing `windows-sys` dependency.
- The native E2E lane passed 55 of 56 browser tests and exposed a lifecycle
  regression after an intentional Chrome crash. Reconciliation preserved the
  terminal health event and correctly removed operational browser state, but
  relaunch could no longer reconstruct the recovery tombstone. Recovery now
  rehydrates that bounded state from event history, preserves trace context,
  emits the recovery sequence, and resolves through a fresh ready browser.

Validation:

- five focused cross-platform regressions
- recovery-history unit regression
- real-browser crash and automatic relaunch E2E
- Rust formatting
- strict Clippy with warnings denied
- complete serialized Rust CI harness

Result:

- All focused regressions, the real-browser E2E, formatting, strict Clippy,
  and the complete serialized Rust CI harness pass locally.
- Full CI run `30550334355` remains valid Windows and E2E failure evidence for
  commit `98316d14`. Matrix fail-fast cancelled both macOS lanes after those
  failures.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 129 | 2026-07-30

Scope: repair the next exact-head full-CI defect.

Actions:

- Confirmed exact-head fast CI run `30547407293` fully green at commit
  `60c784e3`.
- Dispatched full CI run `30548163584`. Its ordinary Linux Rust suite found
  both sequential private-display launches selecting `:90`.
- Traced the collision to `start_remote_headed_virtual_display`, which returned
  after a fixed 150 ms delay without checking that the spawned Xvfb owned a
  ready display.
- Serialized selection inside the daemon and replaced the fixed delay with
  ownership-backed readiness polling. Early child exit, inspection failure,
  and timeout now fail closed and reap the child.
- The completed matrix also found an Apple Silicon daemon-socket fixture
  inheriting a temporary path longer than macOS `SUN_LEN`. Moved that Unix
  fixture to a short, unique path under `/tmp`.

Validation:

- twenty consecutive distinct-live-display regressions
- Rust formatting
- strict Clippy with warnings denied
- complete serialized Rust CI harness

Result:

- The repeated focused regressions, formatting, strict Clippy, and complete
  serialized Rust CI harness pass locally.
- Full CI run `30548163584` remains valid collision evidence for commit
  `60c784e3`. Its Apple Silicon lane independently found the overlong socket
  fixture; fail-fast cancelled the Windows and macOS x64 lanes.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 128 | 2026-07-30

Scope: continue exact-head cross-platform release gating.

Actions:

- Confirmed exact-head fast CI run `30545123372` fully green at commit
  `a5423d6e`.
- Dispatched full CI run `30545744595`. The Windows target compiled and ran
  tests for nearly ten minutes before
  `test_manifest_resolves_stealthcdp_executable_path` exited abnormally.
- Traced the exit to a test fixture that interpolated a native Windows path
  directly into JSON. Backslashes in the path became invalid JSON escape
  sequences, and the explicit-config failure path correctly terminated.
- Replaced string interpolation with structured JSON serialization so the
  fixture remains valid on Unix and Windows.

Validation:

- focused manifest-resolution regression
- Rust formatting
- strict Clippy with warnings denied
- complete serialized Rust CI harness

Result:

- The focused regression, formatting, strict Clippy, and complete serialized
  Rust CI harness pass locally.
- Full CI run `30545744595` is valid Windows failure evidence for commit
  `a5423d6e`; its macOS jobs were cancelled by matrix fail-fast.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 127 | 2026-07-30

Scope: run exact-head release CI and repair the first cross-platform defect.

Actions:

- Confirmed exact-head fast CI run `30541737279` fully green at commit
  `0cbd1729`.
- Dispatched full CI run `30542411936` against that exact commit. Its
  Apple Silicon macOS Rust job failed compilation because
  `statvfs.f_bavail` is 32-bit on that target while the byte calculation
  assumed the Linux 64-bit field shape. Matrix fail-fast then cancelled the
  Windows job.
- Added a typed conversion helper that accepts both platform widths and
  saturates the byte multiplication. Added regression coverage for mixed
  32-bit and 64-bit inputs plus overflow.
- Target-gated the `Path` import used only by Linux `/proc` process sampling,
  removing the adjacent macOS warning.
- Exact-head fast CI run `30543600554` passed at repair commit `2db64424`.
- Full CI run `30544211166` moved Apple Silicon beyond the repaired compile
  site, then the Windows test build found a Linux-only WSL helper referenced
  through runtime `cfg!` and a Unix `libc::kill` call compiled inside
  workstation lock recovery. Matrix fail-fast cancelled both macOS jobs.
- Converted the WSL test to compile-time Linux gating and split stale-lock
  probing into Unix and fail-closed non-Unix implementations. Target-gated the
  adjacent Unix-only resource-monitor imports.

Validation:

- Rust formatting
- strict Clippy with warnings denied
- complete serialized Rust CI harness
- focused mixed-width and saturation regression

Result:

- The local portability repair is green.
- Full CI runs `30542411936` and `30544211166` remain valid macOS and Windows
  failure evidence for their exact commits. The current repair requires a new
  exact-head fast run and a new manually dispatched full run after commit and
  push.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 126 | 2026-07-30

Scope: prove the rebuilt doctor-discovery candidate on the clean Ubuntu host
and enter release gating.

Actions:

- Built commit `ce26f0f6` as release-mode SHA-256
  `06e3b85ebc734c914ad8937afe0f169107cd6e646f5c129ebe1d7afe29aacca2`
  and staged it on the clean rebooted Ubuntu 24.04 host.
- The first idempotent convergence stopped fail-closed because Route A and B
  viewer daemons still ran the preceding candidate. The diagnostic identified
  both exact sessions and supplied bounded close commands. After closing only
  those sessions, the retry passed with candidate and installed hashes equal,
  dashboard active, and interlock timer active.
- Ran standalone doctors from a new login shell. Install doctor and
  remote-view doctor both returned success with no issues. Remote-view doctor
  resolved
  `/home/agent/.local/lib/agent-browser/0.28.0/scripts` and reported remote
  control, many-to-many prerequisites, and the path-scoped managed Chrome
  sandbox policy ready.
- Opened `about:blank` through Route A using the exact installed binary. The
  response selected `guacamole:1`, connection `1`, display `:10`, and returned
  `operatorVisible.state=ready`.
- Closed the bounded browser session. Retained Route A returned to
  `available` with `currentRouteAllocationId=null`.
- Ran whole-slice local validation. The first full Rust pass found that the
  installer order test still searched for
  `install_remote_view_privileges()` after the helper gained explicit
  arguments. Production order remained privilege setup before dependency
  installation. Updated the assertion to the current signature and reran the
  serialized Rust CI harness successfully.
- Fast CI run `30540857427` passed version sync, Rust Quality, the full Rust
  suite, dashboard, service-client, and workstation fixture jobs. Its final
  no-launch packet found stale profile-lookup MCP template and selection-order
  assertions, followed by a fixture assumption that local service status
  starts a daemon and writes `state.json`.
- Consolidated the lookup expectations around the current generated contract.
  Updated shared no-launch setup to accept the offline empty control-plane
  snapshot and create a minimal service state only when no daemon-created file
  exists. The full ten-command no-launch CI packet passes locally.

Validation:

- exact release artifact and installed-binary SHA-256 parity
- fail-closed stale-runtime diagnosis and emitted remediation
- zero-prompt idempotent convergence retry
- standalone install doctor from a fresh login shell
- standalone remote-view doctor from versioned installed assets
- live Route A operator-visible open and cleanup
- Rust formatting, strict Clippy, and the complete serialized Rust suite
- source-free installer, host, VM harness, Guacamole asset, durability, route
  user, and release-verifier fixtures
- service API/MCP and generated-client contracts
- route-confusion gates and live CDP tab streaming
- dashboard contract packet and production build
- docs production build, version sync, planning audit, and shared-skill parity
- complete CI no-launch service smoke packet

Result:

- The clean-install, reboot, durability, conflict, sandbox, installed-helper
  discovery, and final live Route A acceptance evidence is complete.
- Release remains no-go until exact-head fast and full CI, release dry run,
  merge, publication, and public-asset reinstall pass.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 125 | 2026-07-30

Scope: finish clean-install durability and route evidence, then repair a
managed Chrome sandbox defect found by the live gate.

Actions:

- Proved the exact candidate's mutation-free dry run, one-sudo first apply,
  exit-75 reboot boundary, zero-prompt continuation, canonical two-route
  substrate, and final doctors on a clean Ubuntu 24.04 overlay.
- Proved idempotency by preserving binary, manifest, units, secret, named
  volume, PostgreSQL system identity, Guacamole row IDs, routes, displays,
  profiles, and active units across a same-artifact rerun.
- Created a checksummed PostgreSQL custom-format backup and passed its isolated
  temporary-database restore drill with the expected tables, connections, and
  permissions.
- Proved the no-launch Route A plan selects `guacamole:1`, connection `1`, and
  display `:10` while requesting no launch, checkout, or tab.
- The first live Route A open failed before DevTools because Ubuntu 24.04
  AppArmor denied the managed Chrome sandbox user namespace. Lease rollback
  restored the route.
- Added a path-scoped AppArmor `userns` profile to the one-sudo host installer.
  The repair keeps the host restriction and Chromium sandbox enabled.
  Remote-view doctor now reports this policy and blocks live-gate readiness
  when it is missing, inactive, or mismatched.
- Loaded the exact profile on the disposable VM and repeated the live open.
  Route A reached `operatorVisible=ready`.
- Submitted a conflicting `guacamole:999/:99` authoritative definition while
  Route A was checked out. Reconciliation reported
  `skippedActiveConflictEntryIds=["guacamole-rdp-a"]`; the retained entry's
  pre/post SHA-256 remained
  `207ff06af5a214ee29a6cce2f2a8385f39db1e63048e2802310f627fcaef164f`.
  Cleanup returned Route A to `available`.
- Rebuilt commit `a05e7ed0` as SHA-256
  `b929200b25a4104995e41ee64510ad3b650b81dc663e00f8d1bbfb459c4e072d`
  and installed it on a new immutable-base overlay. The embedded installer
  produced one prompt, exit 75, a distinct reboot ID, loaded AppArmor policy
  before and after reboot, and a zero-prompt ready continuation.
- A standalone doctor then fell back to `/home/agent/scripts` even though the
  payload correctly installed its helpers under the versioned support root.
  Doctor discovery now checks
  `~/.local/lib/agent-browser/<version>/scripts` without requiring a checkout
  or ambient override.

Validation:

- workstation host-provision fixture
- managed Chrome sandbox-policy doctor unit
- remote-control viewer-prerequisite doctor unit
- live AppArmor parser and idempotent installer pass on Ubuntu 24.04
- live Route A open with `operatorVisible=ready`
- live active-conflict preservation and cleanup
- clean embedded-policy install and post-reboot loaded-profile proof
- versioned installed support-root discovery regression

Result:

- The installer and doctor now cover the live Chrome sandbox prerequisite that
  the earlier no-launch gates missed.
- Release remains no-go pending the rebuilt doctor-discovery candidate,
  standalone live doctor and Route A proof, full CI, release dry run, merge,
  and public-asset reinstall.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 124 | 2026-07-30

Scope: build the `0.28.0` release candidate, execute the disposable VM lane,
and repair independent Packet F findings.

Actions:

- Built a release-mode `0.28.0` binary with 80 embedded dashboard assets and
  pushed release preparation commit `4a374bea` to PR 7.
- Rebooted the iterative Ubuntu VM to a new boot ID and ran the release
  artifact source-free. The first run stopped fail-closed at route opening; a
  redacted direct retry selected canonical connections 1 and 2 and opened
  distinct displays `:10` and `:11`.
- Completed three independent audits covering installer safety, evidence
  sufficiency, release workflow, and documentation parity.
- Repaired secret-bearing subprocess arguments, ambient route-pool override,
  install-wide locking, payload hash provenance, changelog/version binding,
  Cargo.lock version validation, and missing installer fixtures in selector
  and fast CI.
- The first authoritative clean overlay exposed a 3.5 GiB cloud-image capacity
  defect during apt unpack. Added a 24 GiB VM disk default and a real-host
  6 GiB free-space gate that fails before sudo, payload staging, or package
  mutation.
- The resized clean overlay reached the zero-prompt post-reboot continuation,
  then Guacamole's first JVM process crashed while concurrent header-auth
  requests raced automatic account creation. One account transaction
  succeeded and the other returned a duplicate-key 500. Reconciliation now
  waits for full application readiness, makes one creation request, and
  accepts only the exact database user postcondition.
- The next clean continuation passed header creation and route opening, then
  failed while resetting a newly written interlock service that systemd had
  not loaded yet. A resumed run proved file-derived `LoadState=loaded` is not
  manager-load evidence for a static unit. Activation now observes the
  state-bearing `is-failed` output independently of its exit status, resets
  only an exact `failed` result, and verifies the postcondition after a reset
  race.
- The resumed candidate reached executable handoff, where the retiring daemon
  removed the replacement daemon's rebound Unix socket and session metadata.
  Shutdown now compares the socket device and inode before cleaning any shared
  session artifact.
- A new exact-candidate clean overlay passed the mutation-free dry run,
  one-sudo host preparation, exit-75 relogin, reboot, zero-prompt route
  convergence, unit activation, and install doctor. Final remote-view doctor
  exposed missing `xdpyinfo` plus a legacy host-guacd readiness assumption.
  Host packages now include display and visual-proof tools; readiness accepts
  the pinned Guacd container and managed Chrome outside `PATH`.

Validation:

- focused workstation and payload-integrity Rust tests
- source-free install fixture with concurrent-lock rejection
- route-specific user sync fixture
- release asset and version-bound changelog fixture
- exact selector recommendations for the new installer gates
- disk-capacity boundary unit test and resized VM harness contract
- Guacamole header-user postcondition unit test
- systemd unit-load/reset boundary unit test
- retiring-daemon socket-ownership unit test
- live runtime executable-handoff smoke
- clean-overlay dry run, one-sudo, reboot, and zero-prompt continuation
- container-backed Guacd and viewer-prerequisite readiness fixture

Result:

- Focused repair gates are green.
- Release remains no-go until the rebuilt candidate passes a clean Ubuntu
  install, reboot, idempotent rerun, restore and conflict drills, full CI,
  release workflows, merge, and public-asset reinstall.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 123 | 2026-07-29

Scope: implement and locally validate the source-free workstation installer
before disposable-host testing.

Actions:

- Hardened the release workflow around exact assets, embedded versions,
  execution, checksums, and published-download verification.
- Embedded the pinned Guacamole Compose stack, normalized schema, controller
  helpers, binary, manifest, and systemd user units.
- Added Ubuntu 24.04 amd64 host preflight, one initial sudo authorization,
  noninteractive dependency and privilege work, service verification, and the
  resumable group-refresh boundary.
- Added installed reconciliation for Chrome, Guacamole, PostgreSQL continuity,
  route users, canonical rows, readiness-selected XRDP displays,
  readiness-authoritative service projection, unit activation, final doctors,
  and a private receipt.
- Added fail-closed validation for active legacy route conflicts and private,
  idempotent generated secrets.
- Updated the public install help, README, docs site, skill, roadmap, and Plan
  0082 checkpoint to match the two-stage fresh-login contract.

Validation:

- 8 focused workstation Rust tests
- focused installed-script-root doctor test
- Rust format and strict Clippy
- source-free payload fixture
- embedded Guacamole asset fixture
- workstation host-provision fixture
- clean privilege fixture

Result:

- The source-free payload and local mocked host path are green.
- The release remains no-go until a disposable Ubuntu host proves install,
  reboot, canonical routes, backup restore, idempotency, and the remaining
  release gates.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 122 | 2026-07-29

Scope: open the fresh-install productization and formal `v0.28.0` release lane.

Actions:

- Verified that current local runtime health comes from source-checkout
  convergence and is not reproducible by the public release installer.
- Compared public `v0.27.0` at `17a284f` with plan-intake `main` at `ffda60dd`;
  current `main` is 107 commits newer while still reporting version `0.27.0`.
- Confirmed that P81's canonical route-state projection is absent from the
  public release.
- Confirmed the working dashboard interlock references this repository and
  invokes pnpm, while the complete Guacamole compose and schema substrate is
  not distributable from the release binary.
- Assigned three independent read-only audits covering installer architecture,
  clean-host test design, and release/version eligibility.
- Selected `v0.28.0` as the feature-release target and opened branch
  `prepare-v0.28.0`.
- Added Plan 0082 with explicit clean-install, one-sudo, reboot, idempotency,
  P81 regression, CI, pull-request, dry-run, publication, and public-asset
  gates.

Validation:

- current repository, installed binary, doctor, systemd, GitHub release, CI,
  and workflow readbacks
- Graphiti advisory recall verified against current source and runtime evidence
- three independent read-only audit receipts
- `git diff --check`

Result:

- P82 is open and the release is currently no-go.
- Implementation begins with the existing Rust Quality repair and red
  source-free installer tests.
- The operator-owned untracked `--full-page` file remains excluded and
  untouched.

## Turn 121 | 2026-07-28

Scope: diagnose and repair post-reboot Guacamole route-selection state drift
without launching a browser or consuming a Plan 0012 attempt.

Actions:

- Proved route readiness was current at `guacamole:1/:11` and
  `guacamole:2/:12`, while retained stable entries still pointed at legacy
  routes `guacamole:4/:10` and `guacamole:5/:11`.
- Traced `remote-view open` to retained route-pool selection and proved local
  convergence discarded the readiness JSON before health reconciliation.
- Added red-green parser and reconciliation tests, including fail-closed
  active-conflict and concurrent-checkout cases.
- Added guarded authoritative route-pool refresh, structured response
  evidence, schema and client alignment, and interlock projection.
- Published the local `0.27.0` candidate through guarded daemon handoff after a
  mode-0600 retained-state backup.
- Ran one normal convergence apply and observed the next scheduled interlock
  pass complete successfully.
- Ran a stable-entry `remote-view open` dry run for
  `last30days-facebook`; it selected `guacamole:1/:11` and requested no browser
  launch, route checkout, or tab open.
- Updated CLI help, README, docs site, agent skill, plan, roadmap, and a dated
  validation note.

Validation:

- focused Rust parser and reconciliation tests
- `pnpm test:local-runtime-convergence`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:route-confusion-gates`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- applied convergence plus scheduled interlock receipt
- install doctor, remote-view doctor, retained-state, and no-launch selector
  readbacks

Result:

- P81 is complete. Retained route A is `guacamole:1/:11`; retained route B is
  `guacamole:2/:12`; both are available with no allocation.
- Installed and recurring runtime paths are healthy.
- No browser, source authentication probe, source canary, or Plan 0012 request
  ran. The next Plan 0012 attempt remains explicitly unauthorized.

## Turn 120 | 2026-07-28

Scope: investigate and remediate repeated Guacamole PostgreSQL reinitialization.

Actions:

- Proved the running container retained a stale Docker Desktop WSL bind
  attachment as `tmpfs`, while the declared host path remained on ext4 with a
  different cluster identifier.
- Added a red-capable durability contract, then implemented continuity status,
  atomic checksummed custom-format backup, identity recording, retention, and
  isolated restore drill.
- Made schema assurance fail closed on stale mount, identity discontinuity,
  partial schema, and absent schema for a recorded identity.
- Captured and restore-drilled a current backup before migration.
- Stopped Guacamole web access, migrated PostgreSQL to a named volume, restored
  the verified dump, and required exact route and permission invariants before
  restarting Guacamole.
- Installed and ran the daily backup service, restored the recurring runtime
  interlock, and retained the old bind directory.
- Recorded subagent receipt `spawned`: independent read-only P80 review,
  handle `/root/p80_independent_review`, with no runtime or file mutation. Its
  first verdict found six consolidated durability gaps; the bounded
  remediation pass fixed them and its re-review returned `PASS` with no
  residual blocking findings.

Validation:

- `pnpm test:guacamole-postgres-durability`
- `pnpm test:rdp-guac-postgres-hardening`
- `pnpm ensure:rdp-guac-postgres -- --dry-run`
- pre-migration and post-migration isolated restore drills
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- named-volume mount, identity, route, parameter, and permission readbacks
- installed backup service and timer readbacks
- recurring interlock receipt
- route-pool readiness, remote-view doctor, and install doctor

Result:

- P80 is complete. PostgreSQL uses the named volume on ext4, continuity is
  ready, the database retains two routes, 22 parameters, and four permissions,
  and a verified recovery artifact exists.
- Backup and interlock services succeeded; both timers are enabled and active.
- Remote control and many-to-many readiness remain ready.
- No browser, source-authentication, or old-bind deletion occurred.

## Turn 119 | 2026-07-28

Scope: execute and close P79 route-specific isolation repair.

Actions:

- Added red-green convergence coverage for missing fixtures and typed
  same-user display collapse.
- Added a guarded transaction that migrates exact supported legacy rows in
  place, configures distinct route-specific users, removes `color-depth`,
  preserves permissions, and fails closed on ambiguous topology.
- Made the existing-user sync spelling a compatibility alias to the safe
  route-specific migration.
- Made live Xorg inference authoritative over persisted display hints.
- Updated CLI help, README, docs site, agent skill, roadmap, plan, and
  validation note; synchronized the installed shared agent skill.
- Committed and pushed the reviewed implementation as `2dcac761`.
- Captured live pre-state, closed only the two failed Guacamole viewer
  sessions, and ran one guarded convergence apply.
- Preserved Guacamole connection ids `1` and `2`, migrated to canonical route
  names and users `agent-browser-rdp-a/b`, and opened displays `:11` and `:12`.
- Diagnosed the final aggregate's residual false failure as a cwd-relative
  inspector path, added a failing regression, fixed module-relative
  resolution, and pushed `641f45ae`. No second migration or display
  restoration was run.
- Proved read-only convergence success, installed doctor success, one
  successful recurring interlock pass with no route mutation steps, and an
  enabled active waiting timer.

Validation:

- `pnpm test:local-runtime-convergence`
- `pnpm test:rdp-guac-route-specific-user-sync`
- `pnpm test:rdp-route-display-selection`
- `pnpm test:rdp-guac-postgres-hardening`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml output -- --test-threads=1`
- `pnpm --dir docs build`
- `agent-browser doctor remote-view --json`
- `agent-browser install doctor --json`
- read-only `pnpm converge:local-runtime -- --skip-publish --json`
- `systemctl --user start agent-browser-runtime-interlock.service`
- exact Guacamole row, parameter, permission, Xorg, and X11 socket readbacks

Result:

- P79 is complete. Remote control and many-to-many prerequisites are ready,
  route displays are isolated by user, and the recurring interlock is healthy.
- Source and installed runtime are reported separately: `origin/main` includes
  `641f45ae`; the binary was not replaced and install doctor reports seven
  converged runtimes with zero issues.
- No application browser or source-authentication state was touched.
- Guacamole PostgreSQL reset attribution and backup/restore durability remain a
  separate unresolved packet.

## Turn 118 | 2026-07-28

Scope: diagnose the failed post-reboot agent-browser route substrate and open
the authorized bounded repair.

Actions:

- Reproduced route readiness and confirmed that the two Guacamole RDP records
  and read permissions exist, while route B's expected display socket does
  not.
- Correlated Guacamole, guacd, and XRDP evidence. Both routes authenticated,
  but XRDP reconnected route B to the same user's display `:10`.
- Confirmed XRDP 0.9.24 with sesman `Policy=Default`; the configured 24/32
  color-depth distinction did not yield separate allocation keys.
- Confirmed the existing route-specific users and secret keys are ready and
  the route-specific setup isolation gate passes read-only.
- Identified two additional repair requirements: migrate the existing legacy
  rows in place to avoid four managed routes, and prefer live inferred display
  allocation over stale configured display hints.
- Opened P79 with one implementation/review cycle, one authorized live attempt,
  explicit rollback/stops, and no application-browser or authentication scope.
- Recorded subagent receipt `not_spawned`: this repair crosses one shared live
  Guacamole/XRDP route substrate and needs one critical-path owner.

Validation:

- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
- `node scripts/inspect-rdp-route-displays.js --windows`
- `pnpm --silent setup:rdp-guac-route-pool -- --dry-run`
- Guacamole PostgreSQL route and permission readbacks
- XRDP service configuration and journal readbacks

Result:

- Root cause is the same-user XRDP session collapse, not repository drift,
  Guacamole authentication, application-browser behavior, or source-account
  authentication.
- P79 is active. Live state remains unchanged while failing behavior tests and
  the guarded migration are implemented.

## Turn 117 | 2026-07-27

Scope: run the authorized replacement P78 Packet C attempt and diagnose the
first residual failure.

Actions:

- Confirmed the recurring interlock timer remained enabled but inactive and
  the pre-state still contained zero Guacamole routes.
- Ran exactly one corrected convergence apply with publication skipped.
- Provisioned two Guacamole RDP connections, their required read grants, and
  distinct configured target identities.
- Stopped after display restoration returned status 1. No second sync or
  display-open attempt ran.
- Diagnosed current route readiness, live XRDP processes, Guacamole and XRDP
  logs, installed XRDP policy, and the retained convergence receipt.

Validation:

- Report-only route readiness finds two connections and complete permissions.
- Route A has a live `:10` X11 socket.
- Route B has no `:11` X11 socket.
- XRDP logs show route A created display `:10`, then route B reconnected the
  same user to display `:10`.
- XRDP 0.9.24 is configured with `Policy=Default`, which allocates by user and
  negotiated bit depth. The configured Guacamole 24 and 32 color-depth values
  did not produce distinct live allocation keys.
- The retained convergence receipt records successful fixture provisioning,
  failed `restore_rdp_route_displays`, and final next action
  `repair_rdp_route_display_session`.

Result:

- Packet C remains blocked after its authorized replacement attempt. Packet D
  did not run.
- The database route-fixture defect is repaired, but the existing-user
  two-display isolation assumption is disproved on this runtime.
- The timer remains paused and no application browser or authentication
  surface was touched.

Next recommendation:

- open a new bounded isolation plan to choose between route-specific users and
  a reviewed XRDP session-policy change before any further live display
  mutation.

## Turn 116 | 2026-07-27

Scope: execute P78 through its single authorized live recovery attempt.

Actions:

- Added a behavior-level fixture harness for empty route fixtures, dry-run
  immutability, unrelated doctor actions, and doctor refresh ordering.
- Added the exact typed fixture-provisioning remedy to local-runtime
  convergence and updated README, CLI help, agent skill, docs site, and
  post-reboot operator guidance.
- Paused the enabled interlock timer to prevent a race with the controlled live
  attempt.
- Captured the empty pre-state and ran one authorized convergence apply with
  installed doctors and binaries.
- Stopped at the first typed failure. The existing-user sync rejected the
  plan's unsupported `--apply` argument before changing Guacamole rows.
- Corrected the controller, regression fixture, and plan example through the
  packet's one review/rework cycle. Did not run a second live sync.
- Audited backup truth. No usable PostgreSQL dump, archive, snapshot timer, or
  documented restore workflow was found for the bind mount.

Validation:

- `pnpm test:local-runtime-convergence` failed before the controller remedy,
  passed after it, failed again when the fixture enforced the real
  apply-by-default sync contract, and passed after the rework.
- `node --check scripts/converge-local-runtime.js` passed.
- `node --check scripts/test-local-runtime-convergence-fixture.js` passed.
- `pnpm test:rdp-guac-postgres-hardening` passed.
- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.
- `cargo test --manifest-path cli/Cargo.toml output -- --test-threads=1`
  passed 34 focused tests.
- `pnpm --dir docs build` passed.
- The repo and installed agent-browser skill copies match.
- `git diff --check` passed after closeout documentation.
- The retained convergence receipt records
  `provision_rdp_guac_route_fixtures` status 2 and contains no later display or
  access steps.
- Post-attempt report-only readiness still reports zero RDP connections.

Result:

- Packets A and B are complete.
- Packet C is blocked after its single authorized attempt; Packet D remains
  blocked.
- The corrected live command has not run. A replacement Packet C attempt
  requires new explicit authorization.
- No application browser, authentication surface, source canary, or
  last30days acquisition was touched.

Next recommendation:

- explicitly authorize one replacement Plan 0078 Packet C live attempt, then
  complete installed timer proof and open a separate database durability
  packet.

## Turn 115 | 2026-07-27

Scope: bind P78 planning to commit, push, validation, and durable-memory
receipts.

Actions:

- Preserved `6d5cc908` as the coherent P78 roadmap, plan, and diagnostic
  runbook commit.
- Pushed the commit to `origin/main` and verified local, tracking, and remote
  refs agree.
- Queued one source-backed Graphiti closeout in `agent_browser_main` after the
  provider readiness probe passed.
- Did not provision route fixtures, open route displays, invoke convergence
  apply, retry remote-view acquisition, or launch a browser.

Validation:

- `node scripts/test-local-runtime-convergence.js` passed.
- `node scripts/test-rdp-guac-postgres-hardening.js` passed.
- `git diff --check` passed.
- local `HEAD`, `origin/main`, and the remote main ref agreed at
  `6d5cc908f1849970085d0ad059fbebfcdf8b9652`.

Result:

- P78 remains `PLANNED` and awaits explicit Plan 0078 Packet A authorization.
- Both agent-browser and the coordinating last30days Plan 0012 authority are
  durably pushed and clean.

Graphiti:

- job `6022b120-eb35-4bd9-8a50-0079a40b3782` completed in one attempt;
- episode `2e3a6d86-3d53-40b0-b4c9-0db6398d9264` is visible in
  `agent_browser_main` and passed the read-after-write check;
- the source description binds the episode to Plan 0078 commit `6d5cc908`.

Next recommendation:

- explicitly authorize Plan 0078 Packet A before any source or live route
  repair.

## Turn 114 | 2026-07-27

Scope: diagnose the post-reboot Guacamole route-preflight failure and create a
bounded implementation plan without mutating the live route substrate.

Actions:

- Reproduced the blocker with the report-only route-pool readiness and
  route-display inspection commands.
- Proved the current Guacamole database has zero connection and permission
  rows, while the retained route service state still refers to historical
  routes.
- Bound the data loss to PostgreSQL `initdb` at 2026-07-27 11:46:23 UTC; the
  reason the bind-mounted data directory was empty remains unresolved.
- Traced the remote-view acquisition preflight and confirmed it correctly
  fails before browser creation when no display allocation or available
  route-pool entry exists.
- Traced the convergence controller and found that it ensures the schema but
  has no remedy branch for
  `provision_second_guacamole_rdp_connection`; its display-recovery predicate
  recognizes only three later display-session actions.
- Opened planned P78/Plan 0078 for an idempotent existing-user route-fixture
  remedy, deterministic coverage, documentation, one separately authorized
  live recovery attempt, and installed interlock proof.
- Did not provision routes, open displays, retry remote-view acquisition,
  launch a browser, inspect authentication, or run a source canary.

Validation:

- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
  deterministically reported zero RDP connections and the provisioning next
  action.
- `pnpm --silent inspect:rdp-route-displays` reported no candidate route
  displays.
- Direct read-only Guacamole database counts reported zero connections and
  zero connection permissions.
- Current convergence receipt is unsuccessful, retains the provisioning next
  action, and contains no selected remedy.
- CodeGraph source tracing bound the missing action coverage to
  `routeDisplayRecoveryRequired()` and the apply sequence in
  `scripts/converge-local-runtime.js`.

Result:

- P78 is `PLANNED` and awaits explicit execution authorization.
- The failure class is route-fixture recovery, not application-browser or
  source authentication.
- The next safe action is Plan 0078 Packet A; no live repair is authorized by
  this planning turn.

Graphiti:

- Discovery in `agent_browser_main` supplied advisory history for prior
  post-reboot display reconciliation and earlier two-route live proof.
- Current repo source, runtime doctors, database counts, and retained
  convergence evidence remain authoritative.
- A closeout write should occur only after this planning slice has a durable
  commit.

## Turn 113 | 2026-07-25

Scope: reconnect a registered client session to its healthy retained service
browser instead of auto-launching an unrelated profile.

Actions:

- Reproduced the `last30days` X adapter failure below the content worker.
- Proved access planning and workspace acquisition selected
  `session:last30days-facebook`.
- Proved the next registered-session `tab_list` command attempted to launch
  the default profile and collided with `auracall-corel`.
- Added a same-session retained-browser resolver before ordinary-command
  auto-launch.
- Kept acquisition actions on the existing shared-profile fresh-tab path and
  made reconnected client teardown detach rather than close the browser.
- Rebuilt and installed the runtime through the executable handoff publisher.
- Ran one bounded no-navigation X authentication readback through the repaired
  installed session.

Validation:

- focused retained-session and existing shared-profile Rust tests
- Rust format and clippy
- CDP stream derivation tests
- `pnpm test:route-confusion-gates`
- `pnpm test:service-cdp-tab-streaming-live`
- installed SHA convergence and `agent-browser install doctor --json`
- installed registered-session X auth evaluation without navigation

Result:

- The installed `last30days-facebook` session reconnects to
  `session:last30days-facebook`.
- The existing X tab reports authenticated with no login form, checkpoint, or
  restriction.
- Browser PID `1669680`, the retained CDP endpoint, RDP provider, shared
  display, and Guacamole route `guacamole:4` remained intact.
- The runtime publisher reported a separate missing-handoff-file failure for
  `auracall-corel`; subsequent doctor passed, but its duplicate listener
  inventory remains out-of-scope follow-up.

Graphiti:

- Discovery found prior shared-profile attach/reuse context in
  `agent_browser_main`.
- Closeout memory should use this note after the source commit is durable.

## Turn 112 | 2026-07-25

Scope: repair the X workspace tile so its retained remote-headed browser opens
through Guacamole/RDP instead of an unrelated CDP snapshot.

Actions:

- Reproduced the failure with the exact dashboard selection
  `daemon-session:last30days-facebook`.
- Traced the selection path to `WorkspaceRemoteViewport`: only an explicit
  browser ID could resolve a retained service browser, so a daemon-session
  selection synthesized a CDP browser and let that snapshot win.
- Added one deterministic workspace-selection resolver that maps explicit
  browser IDs first and selected daemon sessions second.
- Made a linked retained service browser authoritative over a selected CDP
  snapshot while preserving CDP as the fallback for sessions with no linked
  service browser.
- Tightened the dashboard runtime smoke with an explicit expected-provider
  assertion so a Guacamole route cannot pass by rendering a CDP canvas.
- Recovered the retained X browser on the existing
  `last30days-facebook` profile and Route A Guacamole allocation after stale
  runtime evidence was reconciled.
- Published the dashboard bundle and converged the installed user-scoped
  runtime.

Validation:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:route-confusion-gates`
- `pnpm build:dashboard`
- `node --check scripts/smoke-local-dashboard-runtime.js`
- installed dashboard smoke with `--workspace-session
  last30days-facebook --expect-workspace-provider rdp_gateway`
- `agent-browser install doctor --json`

Result:

- The X tile resolves to `browser:session:last30days-facebook`.
- The rendered viewport is the Guacamole iframe for `guacamole:4`; no CDP
  canvas is present.
- The retained browser reports `rdp_gateway`,
  `manual_attached_desktop`, `remote_headed`, and ready operator visibility.
- The live and installed dashboard hashes match, runtime convergence reports
  zero stale runtimes, and install doctor reports no issues.
- This proves provider and route correctness, not X authentication state.
- The Graphiti closeout write remains pending because its bounded provider
  readiness probe timed out; no write was queued while the provider was
  degraded.

## Turn 111 | 2026-07-25

Scope: open, implement, validate, and close P77 profile discovery and manual
browser launch UX.

Actions:

- Kept the source note unchanged and converted its requirements into Plan
  0077.
- Audited current launcher, profile selector, HTTP, MCP, generated client,
  runtime state, resource discovery, and workspace inventory surfaces.
- Confirmed that the existing dashboard launcher and profile lookup are
  implementation inputs rather than satisfactory product behavior.
- Reproduced a no-launch selector defect where an X query chose
  `stealthcdp-default` by browser-build fallback instead of the exact
  authenticated `last30days-facebook` profile.
- Implemented one deterministic selector/catalog contract, one authoritative
  manual-browser runtime projection, a dedicated workspace class, and a
  server-backed dashboard workflow.
- Added CLI, HTTP, MCP, generated-client, and dashboard discovery parity with
  explicit ranked evidence and structured `not_found`.
- Added executable handoff so local runtime publication replaces daemons while
  preserving active browser PIDs, ports, targets, and streams.
- Tightened stale session lease expiration and merge persistence, repaired
  orphaned Route B pool checkouts, and made convergence run the real service
  reconcile action.
- Published the installed runtime and proved manual no-CDP visibility through
  both CLI status and the authenticated dashboard API.
- Recorded privacy boundaries, live acceptance criteria, and the explicit
  non-delegation reason.

Validation:

- Rust format, clippy, focused selector, status, lifecycle, route, model, and
  contract tests.
- Service client generation and API/MCP parity.
- Dashboard workspace, navigator, selected-context, launcher, docs, and
  production build checks.
- Live exact X and unmatched free-text lookup, detached no-CDP inventory and
  cleanup, executable handoff, Route B open, install doctor, remote-view
  doctor, and local runtime convergence.

Result:

- P77 is closed.
- Exact X lookup selected `last30days-facebook` by authenticated-target
  evidence; a free-text miss returned structured `not_found`.
- Five active sessions survived final daemon replacement with their browser
  PIDs and CDP endpoints unchanged.
- Final convergence reported zero stale runtimes, current authoritative
  listeners, ready remote control, and no install issues.

## Turn 110 | 2026-07-19

Scope: complete P76 source, contract, installed-runtime, and closeout gates.

Actions:

- Completed bounded clipboard-write capture, daemon-owned dependent batches,
  per-command timing fields, browser accessibility-tree role lookup, and
  bounded closed-tab status projection with full diagnostic retrieval.
- Updated every required CLI help, README, skill, docs-site, schema, contract,
  HTTP, MCP, generated-client, and inline documentation surface.
- Added and passed a real Chrome accessibility fixture for dynamically mounted
  `aria-labelledby` content and supported shadow-root lookup.
- Corrected two stale close-action tests to match intentional removal of
  `NotStarted` browser placeholders and empty released sessions.
- Used the installed smoke to find and repair missing
  `closedTabProjection` metadata on the CLI-local no-launch status path.
- Published the local dashboard runtime, retired stale daemon sessions, and
  verified a converged installed runtime with `agent-browser install doctor`.
- Queued one compact Graphiti closeout episode in `agent_browser_main` from the
  completed plan and redacted incident note after provider readiness passed.

Validation:

- `scripts/ci/rust-tests.sh`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- focused clipboard, CDP lifecycle, dependent batch, service projection, and
  real Chrome accessibility tests
- `pnpm --config.verify-deps-before-run=false test:service-client`
- `pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity`
- repeated successful docs and dashboard production builds
- installed live capture, timeout recovery, dependent batch, status projection,
  and doctor readbacks with a temporary profile

Result:

- P76 is closed. All six slices are implemented and documented.
- The unresolved clipboard promise returned within the bounded deadline, a
  following evaluation succeeded on the same target, and opt-in write capture
  restored the patched method.
- Installed ordinary and full status modes returned their respective
  projection metadata. Final install doctor reported no issues and zero stale
  runtimes.
- The privacy-safe closeout evidence is recorded in Plan 0076 and the incident
  note. The temporary validation profile was removed.

## Turn 109 | 2026-07-19

Scope: open and execute P76 clipboard target recovery and interaction
performance remediation.

Actions:

- Reviewed the retained clipboard incident against the current clipboard,
  CDP timeout, evaluation, locator, batch, and service-status implementations.
- Opened Plan 0076 with six bounded slices covering evidence correction,
  cancellation-safe clipboard timeout and recovery, clipboard-write capture,
  timing and dependent batching, accessible locator repair, and closed-tab
  status projection.
- Made the CDP command lifecycle the deep module for deadline enforcement,
  pending-command cleanup, late responses, and timeout classification.
- Recorded review mediation so empty clipboard text remains successful, target
  recovery must be proved, locator coverage reproduces accessible-name
  behavior, and service-status compaction remains a projection rather than a
  mutation of persisted lifecycle authority.
- Completed Slice A by correcting causal language in the incident note,
  labeling historical observations, replacing the insufficient portal-only
  locator regression, and adding a privacy-safe validation artifact template.
- Completed Slice B source work with a cancellation-safe per-command CDP
  deadline, Chrome renderer timeout, execution termination fallback, normal
  evaluation health probe, successful empty-text output, stable failure codes,
  and explicit replacement-tab guidance.
- Updated CLI help, README, docs command and streaming pages, MCP tool
  description, and repo plus installed skill guidance for the bounded read
  contract.

Validation:

- `git diff --check`
- `cargo test --manifest-path cli/Cargo.toml clipboard -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml native::cdp::client::tests -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity`
- `pnpm --config.verify-deps-before-run=false --dir docs build`
- `pnpm validation:select -- --base HEAD` was blocked before selection by
  pnpm 11 ignored-build enforcement. The underlying selector script completed
  directly without approving dependency build scripts.

Result:

- P76 is open and in progress. Slices A and B source work are complete. Slice C
  clipboard-write capture is the current execution boundary; Slice B installed
  retained-browser proof remains a final closeout gate.

## Turn 108 | 2026-07-06

Scope: close P69 Slice F live proof and fix live-discovered shared-profile and
route repeat-open failures.

Actions:

- Reproduced the in-use profile refusal through
  `scripts/open-rdp-guac-route-displays.js`: plain `open` against
  `/home/ecochran76/.agent-browser/guacamole-route-viewers/a` failed even
  though service state showed `session:rdp-guac-route-a-viewer` already owned
  the profile and exposed a CDP endpoint.
- Updated shared-profile auto-launch target selection so `open` participates in
  retained-browser attach/reuse and so the current session's live service
  browser can be selected when daemon metadata drift leaves `state.browser`
  empty.
- Reproduced the P69 route repeat bug in the full fixture smoke: first
  `remote_view open` checked out `guacamole-rdp-a`, while repeat open failed
  with `route_pool_entry_unavailable`.
- Updated remote-view acquisition to treat same-owner `checked_out` and
  reconciliation-stale `orphaned` route records as reusable when browser id,
  session id, route id, and display allocation still agree.
- Overrode stale route-display env for the live proof with inspected route
  displays `:10` and `:11`.

Validation:

- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile_attach_target -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reuses_same_owner -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `pnpm test:service-client`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:route-confusion-gates`
- `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser pnpm test:rdp-guac-route-pool-readiness`
- `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live`
- `git diff --check`

Result:

- Full fixture live proof passed with artifact
  `/tmp/agent-browser-remote-view-open-live-2026-07-06T22-14-26-356Z`.
  It proved route `guacamole:4`, display allocation `remote-view-display:10`,
  display `:10`, `route_bound_ready`, `browser_window_visible`, one active
  intended target, and OCR text containing `REMOTE VIEW OPEN FIXTURE 55948`.
  P69 validation is complete.

## Turn 107 | 2026-07-06

Scope: audit P69 Slice C residual `remote_view_open` orchestration and remove
stale dispatcher rollback wrappers before live proof.

Actions:

- Audited the remaining `remote_view_open` route-bound sequence after the
  handoff recovery extraction.
- Confirmed the remaining action-local responsibilities are command dispatch,
  live browser side effects, timestamp supply, and repository/service plumbing.
- Removed stale `remote_view_open_rollback_acquisition_lease`.
- Removed stale `remote_view_open_update_acquisition_lease_cleanup`.
- Updated the acquisition-rollback test to call
  `remote_view_handoff::rollback_route_bound_handoff_acquisition` directly with
  an explicit observed timestamp.
- Updated P69 to mark Slice C ready for live proof rather than continuing
  unbounded micro-extractions.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Stale dispatcher rollback vocabulary is gone. P69 remains open for Slice F
  installed-runtime/live proof.

## Turn 106 | 2026-07-06

Scope: continue P69 Slice C by moving failure recovery cleanup-task selection
into the handoff recovery result.

Actions:

- Extended `remote_view_handoff::RouteBoundHandoffFailureRecovery` with
  `cleanup_task`.
- Updated `remote_view_handoff::begin_route_bound_handoff_failure_recovery` so
  it returns the selected cleanup task alongside rollback and cleanup-plan
  evidence.
- Rewired `remote_view_open_rollback_failure_after_cleanup` so `actions.rs`
  executes the handoff-selected cleanup task directly instead of interpreting
  `cleanup_plan` and `skipped_cleanup`.
- Updated the action cleanup test to exercise the task form.
- Extended handoff-module recovery coverage to assert the selected skipped
  cleanup task.
- Updated P69 to record this recovery-result extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Failure recovery now returns the cleanup task selected by the handoff module.
  P69 remains open for final sequencing assessment and Slice F live proof.

## Turn 105 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound failure cleanup task
vocabulary into the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffFailureCleanupTask`.
- Added `remote_view_handoff::route_bound_handoff_failure_cleanup_task`.
- Added
  `remote_view_handoff::route_bound_handoff_failure_cleanup_task_result`.
- Rewired `remote_view_open_cleanup_after_failure` so `actions.rs` still
  dispatches async tab-close or browser-close side effects, but no longer owns
  the close-tab command payload, close-browser task marker, skipped-cleanup
  payload, or cleanup result mapping.
- Added handoff-module coverage for close-tab, close-browser, and skipped
  cleanup task shapes.
- Updated P69 to record this cleanup task extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Failure cleanup task construction and cleanup result mapping are now owned by
  `remote_view_handoff`. P69 remains open for broader sequencing consolidation
  and Slice F live proof.

## Turn 104 | 2026-07-06

Scope: continue P69 Slice C by moving operator-visible proof record assembly
into the handoff module.

Actions:

- Added `remote_view_handoff::route_bound_handoff_operator_visible`.
- Moved route, display, browser, tab, stream, Guacamole, and URL-readiness
  response vocabulary out of `actions.rs`.
- Rewired `remote_view_open` dry-run, pre-checkout proof, final proof, and
  related route-bound tests to use the handoff-owned proof builder.
- Added handoff-module coverage for the operator-visible proof record shape.
- Updated P69 to record the proof assembly extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Operator-visible route-bound proof vocabulary is now owned by
  `remote_view_handoff`. P69 remains open for broader sequencing consolidation
  and Slice F live proof.

## Turn 103 | 2026-07-06

Scope: continue P69 Slice C by moving the remaining simple rollback failure
descriptors into the handoff module.

Actions:

- Added `remote_view_handoff::route_bound_handoff_tab_open_failure`.
- Added `remote_view_handoff::route_bound_handoff_focus_failure`.
- Added `remote_view_handoff::route_bound_handoff_visible_window_proof_failure`.
- Rewired the `remote_view_open` tab, focus, and visible-window proof failure
  branches so `actions.rs` still executes async browser commands and rollback
  cleanup, but no longer owns those failure phase strings or rollback cleanup
  payloads.
- Added handoff-module coverage for the simple rollback failure descriptor
  shapes.
- Updated P69 to record this descriptor consolidation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Tab-open, focus, visible-window proof, and checkout failure descriptors now
  share the handoff module shape. P69 remains open for broader sequencing
  consolidation and Slice F live proof.

## Turn 102 | 2026-07-06

Scope: continue P69 Slice C by moving checkout failure diagnostic preparation
into the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffRollbackFailure`.
- Added `remote_view_handoff::route_bound_handoff_checkout_failure`.
- Rewired the `remote_view_open` checkout failure branch so `actions.rs` still
  executes the async checkout command and rollback cleanup, but no longer owns
  the checkout failure phase string or rollback cleanup payload.
- Added handoff-module coverage for the checkout failure phase and cleanup
  payload.
- Updated P69 to record this checkout failure diagnostic extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Checkout failure phase and rollback cleanup payload construction now live in
  the handoff module. P69 remains open for broader sequencing consolidation and
  Slice F live proof.

## Turn 101 | 2026-07-06

Scope: continue P69 Slice C by moving post-checkout proof sequencing into the
handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffPostCheckoutProof`.
- Added `remote_view_handoff::RouteBoundHandoffPostCheckoutProofInput`.
- Added `remote_view_handoff::route_bound_handoff_post_checkout_proof`.
- Rewired `remote_view_open` so `actions.rs` supplies the final
  operator-visible proof calculation and executes rollback when needed, while
  the handoff module derives the final route binding, invokes the proof
  calculation, and applies the final proof readiness gate.
- Added handoff-module tests for ready and not-ready post-checkout proof
  results.
- Updated P69 to record this post-checkout proof sequencing extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Final route-binding derivation, final proof calculation invocation, and final
  proof readiness gating now run as one handoff step after checkout. P69 remains
  open for broader sequencing consolidation and Slice F live proof.

## Turn 100 | 2026-07-06

Scope: continue P69 Slice C by moving opened-response final route-binding
derivation into the handoff completion path.

Actions:

- Changed `remote_view_handoff::CompleteRouteBoundHandoffOpenInput` so callers
  no longer pass a precomputed final route binding.
- Updated `remote_view_handoff::complete_route_bound_handoff_open` to derive
  the final route binding from checkout readback before completing the lease and
  assembling the opened response.
- Rewired `remote_view_open` to pass only the planned route binding and checkout
  readback into the completion helper.
- Strengthened handoff-module coverage so the completion helper proves it uses
  checkout readback by returning `route-final` in the opened response.
- Updated P69 to record this final route-binding ownership move.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Opened-response final route-binding derivation now lives in the handoff
  completion path. P69 remains open for broader sequencing consolidation and
  Slice F live proof.

## Turn 99 | 2026-07-06

Scope: continue P69 Slice C by moving operator-visible readiness gating into
the handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_operator_visible_failure_if_not_ready`.
- Added
  `remote_view_handoff::route_bound_handoff_final_operator_visible_failure_if_not_ready`.
- Rewired `remote_view_open` so `actions.rs` still computes operator-visible
  proof, but no longer interprets pre-checkout or final proof `state` values to
  decide whether rollback diagnostics are required.
- Added handoff-module tests for ready and not-ready pre-checkout proof gates
  and final proof context preservation.
- Updated P69 to record this readiness-gating extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Operator-visible readiness decisions now live in the handoff module. P69
  remains open for broader sequencing consolidation and Slice F live proof.

## Turn 98 | 2026-07-06

Scope: continue P69 Slice C by moving successful route-bound open finalization
into the handoff module.

Actions:

- Added `remote_view_handoff::CompleteRouteBoundHandoffOpenInput`.
- Added `remote_view_handoff::complete_route_bound_handoff_open`.
- Rewired `remote_view_open` so `actions.rs` still performs command dispatch
  and final operator-visible proof, but no longer completes the route-bound
  lease, derives browser-build proof, serializes the lease, or assembles the
  opened response locally.
- Added handoff-module coverage proving the helper finalizes the lease and
  returns the opened `routeBoundHandoff` response surface.
- Updated P69 to record this successful-open finalization extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Successful route-bound open finalization now lives in the handoff module.
  P69 remains open for broader sequencing consolidation and Slice F live proof.

## Turn 97 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound failure recovery staging into
the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffFailureRecoveryInput` and
  `RouteBoundHandoffFailureRecovery`.
- Added
  `remote_view_handoff::begin_route_bound_handoff_failure_recovery` to perform
  rollback-before-cleanup sequencing and return the cleanup plan plus any
  skipped-cleanup payload.
- Added `remote_view_handoff::RouteBoundHandoffImmediateFailureInput` and
  `route_bound_handoff_immediate_failure` for pre-browser display/launch
  failures that only need rollback plus summary formatting.
- Rewired `remote_view_open` so `actions.rs` still executes async tab/browser
  close commands, but no longer derives cleanup plans from launch/tab evidence
  or builds immediate failure rollback summaries locally.
- Added handoff-module tests for failure recovery staging and immediate
  failures.
- Updated P69 to record this recovery-staging extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Rollback-before-cleanup ordering, cleanup-plan selection, skipped-cleanup
  detection, and immediate display/launch failure summaries now live in the
  handoff module. P69 remains open for broader sequencing consolidation and
  Slice F live proof.

## Turn 96 | 2026-07-06

Scope: continue P69 Slice C by moving operator-visible failure diagnostics into
the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffProofFailure`.
- Added
  `remote_view_handoff::route_bound_handoff_operator_visible_failure`.
- Added
  `remote_view_handoff::route_bound_handoff_final_operator_visible_failure`.
- Rewired `remote_view_open` operator-visible and final operator-visible
  failure branches to use those helpers for paired error text and rollback
  cleanup payloads.
- Added handoff-module tests proving the diagnostics preserve
  `routeBoundHandoff` and `preCheckoutOperatorVisible` labels.
- Updated P69 to record this failure-diagnostic extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Operator-visible proof failure error text and cleanup payload construction now
  lives in the handoff module. P69 remains open for full sequencing extraction
  and Slice F live proof.

## Turn 95 | 2026-07-06

Scope: continue P69 Slice C by moving pre-launch and launch-failure cleanup
payloads into the handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_pre_launch_failure_cleanup`.
- Added
  `remote_view_handoff::route_bound_handoff_launch_failure_cleanup`.
- Rewired the `remote_view_open` display-access failure branch and browser
  launch failure branch to use those handoff helpers instead of hand-built JSON.
- Added handoff-module coverage for the skipped-before-launch and
  skipped-after-launch cleanup payload shapes.
- Updated P69 to record this cleanup-payload extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Pre-launch and launch-failure cleanup JSON shapes now live in the handoff
  module. P69 remains open for deeper end-to-end orchestration and Slice F live
  proof.

## Turn 94 | 2026-07-06

Scope: continue P69 Slice C by moving reused-browser launch evidence into the
handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_reused_browser_launch_result`.
- Rewired `remote_view_open` to use that helper when the selected route is
  already checked out to the current browser/session.
- Removed the inline reused-launch JSON shape from `actions.rs`.
- Added handoff-module coverage for browser, session, route, display, and
  reason evidence in the reused-launch result.
- Updated P69 to record this launch-result vocabulary extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Reused route-bound browser launch evidence now lives in the handoff module.
  P69 remains open for deeper end-to-end orchestration and Slice F live proof.

## Turn 93 | 2026-07-06

Scope: continue P69 Slice C by moving visible-window checkout command
finalization into the handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_checkout_command_with_visible_window_proof`.
- Rewired `remote_view_open` to finalize the route-checkout command through
  the handoff helper after visible-window proof.
- Removed the action-local checkout command mutation that attached readiness
  and display-content proof.
- Added handoff-module tests for checkout command finalization with and
  without display content.
- Updated P69 to record this checkout finalization extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Visible-window proof enrichment of checkout commands now lives in the
  handoff module. P69 remains open for deeper end-to-end orchestration and
  Slice F live proof.

## Turn 92 | 2026-07-06

Scope: continue P69 Slice C by moving failure rollback cleanup payload
vocabulary into the handoff module.

Actions:

- Added handoff helpers for generic pending rollback cleanup, operator-visible
  failure cleanup, and final operator-visible failure cleanup payloads.
- Rewired `remote_view_open` tab, focus, visible-window proof, checkout,
  operator-visible, and final operator-visible failure branches to use the
  handoff cleanup payload helpers.
- Kept rollback execution and async browser cleanup in `actions.rs`.
- Added handoff-module tests for simple rollback cleanup and the two
  operator-visible proof cleanup surfaces.
- Updated P69 to record the failure-cleanup vocabulary extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Failure cleanup JSON shapes for the route-bound handoff path now live in the
  handoff module. P69 remains open for deeper end-to-end orchestration and
  Slice F live proof.

## Turn 91 | 2026-07-06

Scope: continue P69 Slice C by moving plan-path acquisition begin/complete
adapters into the handoff module.

Actions:

- Added `begin_route_bound_handoff_plan_acquisition` to own the plan-path
  begin-acquisition call plus default control-input selection.
- Added `complete_route_bound_handoff_plan_acquisition` to restore a missing
  lease when needed and complete the acquisition in one handoff API.
- Rewired `remote_view_open` to call those handoff helpers while keeping
  timestamp generation in `actions.rs`.
- Removed the action-local begin, complete, and restore acquisition wrappers.
- Updated the lease rollback test to exercise the handoff begin helper.
- Updated P69 to record the acquisition helper consolidation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- The route-bound plan path now enters and completes acquisition through named
  handoff-module APIs. P69 remains open for deeper end-to-end orchestration and
  Slice F live proof.

## Turn 90 | 2026-07-06

Scope: continue P69 Slice C by grouping route-bound plan artifacts behind the
handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffPlan` and
  `route_bound_handoff_plan` to group the normalized route binding with launch,
  tab, and route-checkout command artifacts.
- Rewired `remote_view_open` to consume one handoff plan after acquisition-plan
  selection instead of normalizing the route binding and constructing command
  values locally.
- Removed the action-local route-binding normalization helper.
- Updated stale acquisition-pending readiness coverage to exercise the handoff
  plan path and added a handoff-module test for grouped plan artifacts.
- Updated P69 to record this plan-artifact consolidation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Route-bound planning now has a named handoff-module API that returns the
  normalized route binding and command artifacts together. P69 remains open for
  deeper acquisition/finalization sequencing and Slice F live proof.

## Turn 89 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound command artifact
construction into the handoff module.

Actions:

- Added handoff-owned builders for route-bound launch, tab, focus, and
  route-checkout commands.
- Rewired `remote_view_open` to use the handoff command builders while keeping
  browser/service command execution in `actions.rs`.
- Removed the action-local route-bound command builders for launch, tab,
  focus, and checkout.
- Moved focus-command coverage into the handoff module and added coverage for
  launch/checkout route fields and tab default URL behavior.
- Updated P69 to record this command-artifact extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Route-bound command artifacts now have named handoff-module APIs. P69 remains
  open for deeper plan/acquire/finalize orchestration and Slice F live proof.

## Turn 88 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound browser-build proof
finalization into the handoff module.

Actions:

- Added `remote_view_handoff::route_bound_handoff_browser_build_proof` to own
  selected browser build, executable path, applied capability, and mismatch
  evidence for route-bound opened responses.
- Rewired `remote_view_open` to call the handoff helper before opened-response
  assembly.
- Removed the action-local browser-build proof helper and moved its mismatch
  regression coverage into the handoff module.
- Updated P69 to record this proof-finalization extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml browser_build_proof -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Browser-build proof finalization now has a named handoff-module API. P69
  remains open for deeper plan/acquire orchestration and Slice F live proof.

## Turn 87 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound final route-binding
derivation into the handoff module.

Actions:

- Added `remote_view_handoff::final_route_bound_handoff_route_binding` to own
  the merge of planned route binding, route checkout readback, and route-pool
  checkout readback.
- Rewired `remote_view_open` to use that handoff helper before final
  operator-visible proof and opened-response assembly.
- Removed the action-local final binding merge helper.
- Added handoff-module coverage for route and route-pool checkout readback
  overriding the planned binding, and kept the action-level stale-route proof
  coverage green.
- Updated P69 to record the finalization extraction.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_final_route_binding -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/actions.rs cli/src/native/remote_view_handoff.rs docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- Final route binding derivation now has a named handoff-module API. P69
  remains open for deeper plan/acquire orchestration and Slice F live proof.

## Turn 86 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound failure rollback sequencing
behind the handoff module.

Actions:

- Added `remote_view_handoff::rollback_route_bound_handoff_failure` to restore
  a missing acquisition lease and roll route, display, route-pool, and browser
  display-allocation state back through one handoff API.
- Added `remote_view_handoff::complete_route_bound_handoff_failure_cleanup` to
  attach browser cleanup evidence to the rollback and produce the cleanup
  summary string.
- Rewired `remote_view_open` tab, focus, proof, checkout, and final-proof
  failure branches through one cleanup adapter. `actions.rs` still performs the
  async browser cleanup command, but no longer open-codes lease restoration,
  rollback mutation, cleanup attachment, and summary formatting in every branch.
- Added a repository-backed handoff-module test for restoring a missing lease,
  rolling pending state back to previous values, recording browser cleanup, and
  returning a parseable cleanup summary.
- Updated P69 to record this Slice C sequencing progress.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_cleanup -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/actions.rs cli/src/native/remote_view_handoff.rs docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- Route-bound failure rollback and cleanup summary sequencing now has a named
  handoff-module interface. P69 remains open for deeper plan/acquire/finalize
  orchestration and Slice F live proof.

## Turn 85 | 2026-07-06

Scope: continue P69 Slice C by routing plain remote-headed `open` through the
shared acquisition-result surface.

Actions:

- Added a short-lived daemon response slot for launch-time shared-profile
  acquisition evidence.
- Reused `remote_view_handoff::shared_profile_acquisition_result` for plain
  `open`/`navigate` auto-launch when it attaches to a compatible retained
  same-profile browser and opens a tab there.
- Taught the subsequent navigation response to include `sharedAcquisition`
  with the selected retained owner browser/session, requested/planned profile,
  duplicate-process policy, and `routeHintSource: shared_profile_auto_launch`.
- Added focused Rust coverage for the plain-open owner evidence shape.
- Updated P69 to remove the plain remote-headed `open` acquisition-result gap.

Validation:

- `~/.local/bin/graphiti-runtime doctor`
- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml shared_profile_auto_launch_acquisition -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `node --check packages/client/src/service-request.js`
- `node --check scripts/test-service-request-client.js`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/actions.rs docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- Plain remote-headed `open` now participates in the same named
  `sharedAcquisition` response vocabulary as `remote_view_open` and HTTP/MCP
  `service_request` tab acquisition. P69 remains open for the full
  plan/acquire/finalize/rollback sequencing move and Slice F live proof.

## Turn 67 | 2026-07-06

Scope: execute P69 Slice A and the ordinary-open part of Slice B.

Actions:

- Added global `--browser-build` parsing and `clean_args` handling.
- Preserved explicit global launch-routing flags on plain `open`, `goto`, and
  `navigate` command payloads.
- Added a shared-profile auto-launch acquisition path that attaches to a
  compatible retained same-profile browser with a CDP endpoint, creates a fresh
  tab, and then lets the existing navigation handler load the requested URL.
- Updated the P69 plan, the `last30days` routing-failure note, CLI help,
  README, docs site commands page, and `skills/agent-browser/SKILL.md`.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml test_parse_global_browser_build_flag -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_navigate_preserves_explicit_global_launch_routing_flags -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/flags.rs cli/src/commands.rs cli/src/native/actions.rs cli/src/output.rs README.md docs/src/app/commands/page.mdx skills/agent-browser/SKILL.md`

Result:

- Slice A is implemented for plain navigation commands.
- Slice B is partially implemented for ordinary `open`, `goto`, and `navigate`.
  HTTP/MCP `service_request` parity, public acquisition response fields,
  dashboard/client actionability, and live two-tab proof remain open P69 work.

## Turn 66 | 2026-07-06

Scope: write P69 for shared-profile routing and handoff deepening.

Actions:

- Reviewed the architecture review report, the `last30days` profile-routing
  failure note, the runtime-profile sharing plan, and P67/P68 profile identity
  follow-ups.
- Added
  `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
  to make plain `open` preserve explicit runtime identity, route compatible
  in-use profiles through retained-browser tab acquisition, deepen the
  route-bound handoff module, and align workspace inventory plus generated
  client contracts.
- Added the P69 roadmap entry so the new lane is discoverable from
  `ROADMAP.md`.

Validation run:

- Read-only policy, Graphiti, CodeGraph, roadmap, runbook, and source-note
  inspection only.

Result:

- P69 is open and ready for Slice A implementation.

## Turn 65 | 2026-06-27

Scope: implement P46 S10 harness and stop at the S10 retry lock.

Actions:

- Added S10 scenario metadata for a service-owned route-bound browser beside a
  zero-lease foreign CDP browser.
- Added a live S10 runner path that launches a foreign Chromium profile outside
  `~/.agent-browser`, captures authenticated dashboard inventory, and evaluates
  selected workspace action and route/display isolation.
- Ran two live S10 attempts from the installed binary lane. Both failed before
  S10 evaluation on dashboard inventory endpoint/auth issues, and both reset
  cleanly with zero active incidents.
- Repaired the harness to read `/api/sessions` and
  `/api/session-tabs?port=...` through the authenticated dashboard
  viewer-client session.
- Added P63 and locked P46 at S10 pending validation-backed retry clearance.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-dashboard-workspace-nodes.js`

Result:

- No-live checks pass after the authenticated inventory fix.
- Failed live artifacts:
  `/tmp/agent-browser-p46-s10-2026-06-27T22-17-57-154Z` and
  `/tmp/agent-browser-p46-s10-2026-06-27T22-20-21-552Z`.
- P46 is locked at S10 pending P63. Do not run another S10 retry until P63's
  green preflight authorizes exactly one retry.

## Turn 64 | 2026-06-27

Scope: complete P62 and clear P46 S9.

Actions:

- Repaired dashboard selected-target recovery so an explicitly selected live
  blank tab is preserved as the selected target, while missing or dead stale
  selections still recover to a live tab.
- Updated S9 viewer-client and evaluator checks to accept exact blank-target
  preservation or typed stale-target recovery before requiring final blank
  navigation.
- Rebuilt and installed the dashboard runtime with
  `pnpm publish:local-dashboard -- --skip-smoke --json`.
- Verified installed runtime convergence, then reran S9 from the installed
  binary authority.
- Marked P62 complete and advanced P46 to S10.

Validation run:

- `node --check scripts/lib/p47-viewer-client.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-dashboard-view-streams.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- packages/dashboard/src/components/workspace-remote-viewport.tsx scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-dashboard-view-streams.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js`
- `agent-browser --json install doctor`
- `node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json`
- `node scripts/run-p46-stress-scenario.js --scenario s9 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- Installed runtime doctor passed with zero issues.
- S9 passed with artifact
  `/tmp/agent-browser-p46-s9-2026-06-27T22-03-14-950Z`.
- The pass proved exact initial blank-target selection, blank navigation to
  IANA, duplicate same-origin tab isolation, browser-window-visible route
  display, route-bound finalization, one default-profile browser row, and zero
  active incidents after reset-after.
- P46 is now in progress at S10.

## Turn 63 | 2026-06-27

Scope: implement P46 S9 stale target and duplicate tab stress, then record the
S9 lock.

Actions:

- Added S9 scenario metadata, live capture, evaluator checks, and no-live
  harness assertions.
- Added a narrow viewer-client stale selected-tab recovery option for the S9
  operator C blank-tab proof.
- Ran S9 live attempts from the explicit rebuilt-binary lane with reset-before
  and reset-after.
- Added P62 for the selected-target recovery follow-up.
- Updated P46 and the P46 execution note with the S9 lock.

Validation run:

- `node --check scripts/lib/p47-viewer-client.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js`

Result:

- S9 did not pass. Corrected failure artifact:
  `/tmp/agent-browser-p46-s9-2026-06-27T21-42-54-990Z`.
- The run proved stale blank-tab recovery notice and CLI navigation of the blank
  target, but the dashboard rewrote operator C back to duplicate target A when
  the harness re-requested the blank-tab dashboard URL.
- Reset-after reported zero sessions, zero browsers, zero tabs, and zero active
  incidents.
- P46 is locked at S9 pending P62. Do not run another S9 retry until P62
  records validation-backed retry authorization.

## Turn 62 | 2026-06-27

Scope: implement and clear P46 S8 display-access recovery.

Actions:

- Added P61 for the S8 display-access denial and recovery proof.
- Added S8 metadata, live capture, evaluator checks, and no-live harness
  assertions.
- Used a temporary `timeout` shim in `PATH` to safely simulate display-access
  denial without mutating host X11 permissions.
- Reran the same route-bound open with normal display access as the recovery
  proof.
- Updated P46 and the P46 execution note with S8 clearance.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js`
- `node scripts/run-p46-stress-scenario.js --scenario s8 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S8 passed with artifact
  `/tmp/agent-browser-p46-s8-2026-06-27T21-07-22-844Z`.
- The pass proved typed `display_access_grant_failed` denial before browser
  launch, cleanup rollback of display allocation, remote-view route, and
  route-pool entry, no retained denied-profile browser row, terminal-free route
  displays after denial, successful recovery open with
  `displayAccessGrant.state: already_ready`, and zero active incidents after
  reset-after.
- P46 is now in progress at S9.

## Turn 61 | 2026-06-27

Scope: implement and clear P46 S7 route-pool exhaustion.

Actions:

- Added P60 for the S7 route-capacity diagnostic repair.
- Added S7 metadata, live capture, evaluator checks, and no-live harness
  assertions for third-demand route-pool exhaustion and retry after release.
- Tightened `plan_remote_view_acquisition` so unpinned route-bound demand that
  lands on a checked-out pool display owned by another session reports
  `route_pool_exhausted`.
- Rebuilt `./cli/target/debug/agent-browser`, restarted the stale default
  daemon, and ran the rebuilt-binary S7 verifier.
- Updated P46 and the P46 execution note with the S7 clearance.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reports_route_pool_exhausted -- --nocapture`
- `node scripts/test-p47-scenario-harness.js`
- `cargo build --manifest-path cli/Cargo.toml`
- `node scripts/run-p46-stress-scenario.js --scenario s7 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S7 passed with artifact
  `/tmp/agent-browser-p46-s7-2026-06-27T20-58-30-721Z`.
- The pass proved both route-pool entries occupied, third demand failing with
  `route_pool_exhausted`, no retained profile C browser row after the failed
  demand, no terminal fallback on occupied displays, successful profile C retry
  after releasing profile A, and zero active incidents after reset-after.
- P46 is now in progress at S8.

## Turn 60 | 2026-06-27

Scope: clear P46 S6 and advance to S7.

Actions:

- Ran the P55-authorized S6 retry from the explicit rebuilt-binary lane.
- Manually closed the two retained S6 profile sessions after reset-after missed
  them.
- Added P56 to reconnect the external viewer-client CDP websocket after swapped
  dashboard navigation.
- Added a 32 MiB `spawnSync` output buffer to the P46 runner so large
  `service status` payloads remain parseable during reset.
- Added no-live coverage for swapped reconnect artifacts and reset buffer
  hardening.
- Ran one P56-authorized S6 retry after green preflight.
- Added P57 to require DevTools target-discovery evidence before another S6
  retry.
- Added P58 to wait for the swapped DevTools page URL before reconnecting.
- Added P59 to use same-origin `history.pushState` plus `popstate` for
  dashboard workspace swaps.
- Ran the P59-authorized S6 retry after green preflight.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0055-2026-06-27-s6-dashboard-swap-navigation-plan.md docs/dev/plans/0056-2026-06-27-s6-dashboard-reconnect-and-reset-buffer-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md`

Result:

- P55 retry failed with artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T20-06-51-508Z`.
- The failure moved to post-swap state polling:
  `CDP command Runtime.evaluate timed out after 30000ms`.
- P56 retry failed with artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T20-15-52-909Z`.
- The failure moved to reconnect command enablement:
  `CDP command Page.enable timed out after 30000ms`.
- P56 reset-after closed both retained S6 profile sessions, and final readback
  showed zero sessions, zero browsers, zero tabs, zero active incidents, and
  both route-pool entries available.
- P57 retry showed the selected DevTools page URL still pointed at profile A
  after requesting profile B.
- P58 retry showed `location.assign()` did not change the selected DevTools
  page URL for the same-origin dashboard workspace swap.
- P59 retry passed with artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T20-32-54-709Z`.
- S6 pass proved swapped selected-browser readback for both operators, working
  swapped refresh controls, swapped screenshots, distinct route-bound profile
  checkouts, profile B readiness after profile A closed, and clean reset-after.
- P46 is now in progress at S7.

## Turn 59 | 2026-06-27

Scope: repair P46 S5 viewer-client port allocation, pass S5, and start S6.

Actions:

- Added P54 for the S5 viewer-client DevTools port collision that locked P46
  after S5 attempt 2.
- Changed the external dashboard viewer-client launch path to use Chromium
  dynamic DevTools allocation with `--remote-debugging-port=0` by default.
- Added `DevToolsActivePort` readback before viewer-client `/json/version` and
  `/json` calls.
- Kept explicit viewer-client DevTools port overrides only for diagnostics.
- Extended the P47 viewer-client no-live test to cover dynamic launch metadata,
  `DevToolsActivePort` parsing, override validation, and absence of the old
  random fixed-port selector.
- Updated P46 and the P46 execution note with the P54 repair and S5 pass.
- Added S6 metadata, runner support, and no-live coverage for two-profile
  cross-observation with swapped dashboard selection.
- Added a CDP command timeout to the viewer-client adapter after S6 attempt 1
  hung before writing swapped selection artifacts.
- Updated reset handling to close retained browser rows from `activeSessionIds`
  and `session:<name>` browser IDs when session rows are missing.

Validation run:

- `node --check scripts/lib/p47-viewer-client.js`
- `node --check scripts/test-p47-viewer-client-separation.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`
- `pnpm test:p47-viewer-client-separation`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/test-p47-viewer-client-separation.js scripts/run-p46-stress-scenario.js scripts/lib/p46-scenario-harness.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0054-2026-06-27-s5-viewer-client-port-allocation-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`
- `node scripts/run-p46-stress-scenario.js --scenario s5 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`
- `node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S5 passed with artifact
  `/tmp/agent-browser-p46-s5-2026-06-27T19-41-29-598Z`.
- The pass proved profile A on route `guacamole:3` and display `:13`, profile
  B on route `guacamole:4` and display `:14`, finalized route-bound checkouts
  for both profiles, working refresh controls for both external dashboard
  viewer clients, browser-visible route displays for both routes, and profile B
  staying ready after profile A closed.
- Reset-after and final readback showed zero sessions, zero browsers, zero
  tabs, and zero active incidents.
- S6 attempt 1 artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T19-49-19-793Z` proved both profile
  browsers and both initial dashboard viewers became ready, but the run hung
  before swapped dashboard selection artifacts were written.
- S6 attempt 2 artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T19-56-33-105Z` failed in bounded form
  with `CDP command Page.navigate timed out after 30000ms` during the swapped
  dashboard selection step.
- Manual cleanup closed
  `p46-s6-profile-a-2026-06-27T19-56-29-450Z` and
  `p46-s6-profile-b-2026-06-27T19-56-29-450Z`; final readback showed zero
  sessions, zero browsers, zero tabs, zero active incidents, route-pool entries
  available, and idle displays.
- P54 is complete. P46 is locked at S6 by the two-consecutive-failure rule.

## Turn 57 | 2026-06-27

Scope: diagnose the P46 S4 lock and create the S4 topology follow-up plan.

Actions:

- Re-read P46, the P46 execution note, repo validation and memory policies, and
  the S4 attempt 2 artifact.
- Confirmed Graphiti was healthy, but the focused read did not add S4-specific
  authority beyond repo files and artifacts.
- Classified S4 attempt 2 as a same-profile topology and typed-blocker gap:
  window A reached `operatorVisible.state=ready` on `p46-s4-profile`, route A,
  and display `:13`; window B then tried the same runtime profile on route B,
  timed out, and left route-bound finalization cleanup evidence.
- Added P53 to decide and implement the supported S4 topology before any live
  S4 retry.
- Implemented the P53 Goal 1 no-live S4 topology guard. The S4 runner now
  writes `s4-topology-preflight.json` and stops with
  `same_profile_multi_process_unsupported` before launching window B for the
  current one-profile, two-session, two-route-pool-entry shape.
- Selected the P53 Goal 2 topology: one retained remote-headed browser process,
  one route lease, one runtime profile, and two top-level same-profile windows.
- Added `agent-browser window new [url] --same-profile` and rewired S4 window B
  to use that same-session window target instead of a second route-bound
  browser process.
- Switched S4 to a unique `p46-s4-window-<timestamp>` daemon session per run
  after the first P53-shaped retry reused a stale named session and exercised
  the older window handler.
- Updated P46 and the P46 execution note to keep the lock in place pending P53.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0053-2026-06-27-s4-single-profile-window-topology-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md`
- Read-only service status: zero sessions, zero browsers, zero tabs, and zero
  active incidents.
- Read-only install doctor: success, zero issues, one matching default socket
  listener, and zero deleted default-socket listeners.

Result:

- P46 remains locked at S4 by its two-failure rule.
- No live S4 retry was run.

## Turn 56 | 2026-06-23

Scope: complete the P44 Slice H dashboard inventory class inspector and local
publish smoke.

Actions:

- Added `WorkspaceInventoryClass` to the shared dashboard workspace node model.
- Classified service-owned controllable browsers, service-owned view-only
  browsers, service-owned diagnostic browsers, detected non-owned browsers,
  viewer clients, retained history, service-owned sessions, and profile action
  rows.
- Exposed the inventory class through selected-workspace context, diagnostic
  bundles, and evidence rows so inspector, chat, console, and automation
  consumers do not infer ownership from URL shape.
- Added the selected Workspace inspector Class row backed by the canonical
  `WorkspaceInventoryClass` value.
- Published the dashboard runtime locally and ran the full local dashboard
  smoke against `/home/ecochran76/.local/bin/agent-browser`.
- Updated README, dashboard docs, Plan 0044, `ROADMAP.md`, and repo plus
  installed skill guidance.

Validation run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm publish:local-dashboard -- --expect-marker service-owned-controllable-browser --skip-browser --json`
- `pnpm smoke:local-dashboard-runtime -- --expect-marker service-owned-controllable-browser --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --json`
- `agent-browser install doctor --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`

Result:

- Focused dashboard workspace model, selected workspace, chat-packet, console,
  view-stream, navigator, inspector action, docs, dashboard build, local
  publish, runtime smoke, skill-sync, and hygiene checks passed.
- Local publish restarted `agent-browser-dashboard.service` and installed
  executable SHA
  `6c7c9b879c1b564130fb74e4d2abec7502252033be14e66586c20477e7762649`
  with dashboard bundle SHA
  `10177dc55ce0a76f29fbcce7ede2acf8e7b5cbb896d83987ddff2e2aaa193967`.
- Runtime smoke loaded `http://127.0.0.1:4848/`, found
  `service-owned-controllable-browser`, and confirmed the workspace pane in
  browser session `local-dashboard-runtime-smoke-1606766`.
- Closing stale daemon session `default` brought install doctor runtime
  convergence back to `converged` with stale daemon count `0`.
- Slice H dashboard inventory refactor is complete. P44 remains open for the
  installed privileged helper refresh and the later Slice I and Slice J work.
  Doctor still reports `remote_view_route_desktop_helper_stale`, which needs
  `agent-browser install --with-remote-view-privileges` from an interactive
  sudo shell, plus readiness-impacting stale resource candidates.

## Turn 55 | 2026-06-23

Scope: start P44 Slice H dashboard inventory actionability.

Actions:

- Moved non-ready RDP gateway operator-visible proof rows out of the active
  workspace control group and into `needs-attention`.
- Kept View and Control disabled with route-proof reasons while enabling Repair
  for actionable route-proof failures.
- Extended dashboard workspace fixture coverage for terminal-only, unbound,
  missing-proof, wrong-tab, unavailable-route, missing-CDP-target, and
  stale-route rows, while preserving active controllable service-owned rows and
  detected non-owned CDP rows.
- Updated README, dashboard docs, commands docs, Plan 0044, `ROADMAP.md`, and
  repo plus installed skill guidance.

Validation run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-view-streams`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-workspace-navigator`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`

Result:

- The focused dashboard workspace, adjacent view-stream, selected workspace,
  docs, dashboard build, skill-sync, and hygiene checks passed.
- P44 remains open. The installed helper refresh still needs interactive sudo,
  and Slice H still needs inspector and manual publish smoke coverage before it
  can be called complete.

## Turn 54 | 2026-06-23

Scope: start P44 Slice G fast route preflight.

Actions:

- Added `fastPreflight` to the existing
  `service_remote_view_route_preflight` no-launch action.
- The response now reports `ready`, `partial`, `stale`, or `blocked` from
  component evidence for acquisition planning, Guacamole route URL shape,
  retained Guacamole web/login/permission and RDP TCP readiness, display access,
  and route desktop state.
- Added HTTP `GET /api/service/remote-view/route-preflight`, MCP
  `service_remote_view_route_preflight`, and
  `getServiceRemoteViewRoutePreflight()` as first-class no-launch convenience
  surfaces over the same fast preflight response.
- Bounded the shared display-access probe with `timeout --kill-after=1 2` so
  fake or unreachable route displays cannot hang preflight or route-open display
  access checks.
- Updated README, CLI help, service-mode docs, service-request schema
  description, Plan 0044, `ROADMAP.md`, and repo plus installed skill guidance.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_route_and_lease_actions_mutate_service_state -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_route -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm test:remote-view-route-preflight-timing`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`

Result:

- Focused route preflight and route-action Rust tests, clippy, docs,
  service-client, API/MCP parity, skill-sync, and hygiene checks passed.
- P44 remains open. Slice G now has HTTP/MCP/client convenience surfaces and a
  bounded timing smoke; remaining live boundaries are still the installed helper
  refresh and guarded route-bound repeat-open smoke.

## Turn 53 | 2026-06-23

Scope: continue P44 Slice F route-bound repeat-open target convergence.

Actions:

- Routed `remote_view_open` tab acquisition through same-origin live target reuse
  before opening a new tab.
- Added `tabAcquisitionDecision` and `duplicateTargetCleanup` evidence to
  successful route-bound tab acquisition results.
- Extended the remote-view-open live smoke to assert that CLI first, CLI repeat,
  and HTTP helper opens converge to one active intended target in service state.
- Updated README, CLI help, docs site, Plan 0044, `ROADMAP.md`, and repo plus
  installed skill guidance for the repeat-open convergence contract.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_reusable_live_target -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `node --check scripts/smoke-remote-view-open-live.js`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `agent-browser install doctor --json`

Result:

- Static, docs, and focused Rust checks passed.
- The live route-bound repeat-open smoke is implemented but not run in this
  turn because install doctor still reports
  `remote_view_route_desktop_helper_stale`; refreshing the installed helper
  requires an interactive sudo boundary.
- P44 remains open. Slice D still needs the interactive helper refresh and cold
  route desktop proof; Slice F still needs the guarded live smoke run after that
  refresh.

## Turn 52 | 2026-06-23

Scope: continue P44 Slice F dashboard stale-target URL recovery.

Actions:

- Updated the workspace remote viewport to treat missing, closed, blank, or
  target-shaped stale `tab=target:*` URL selections as recoverable stale target
  identity for the selected browser.
- Replaced stale workspace tab URL selections with the current live service tab
  before control mode queues `view_focus`.
- Preserved the existing `stale_target_recovered` UX and readiness vocabulary
  while adding a focused recovery message that names the stale selection and
  current live tab.
- Added dashboard view-stream fixture assertions for stale URL replacement and
  target-shaped stale tab recovery.
- Updated README, docs site, Plan 0044, `ROADMAP.md`, and repo skill guidance.

Validation run:

- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm validation:select -- --base HEAD`
- `git diff --check`

Result:

- All listed checks passed.
- P44 remains open. Slice F still needs a route-bound repeat-open live proof
  that verifies one intended active target.
- Slice D remains open on the interactive sudo boundary for refreshing the
  installed privileged helper and proving a cold browser-control-ready route
  desktop.

## Turn 51 | 2026-06-23

Scope: start P44 Slice F tab acquisition cleanup with a duplicate-replacement
refresh policy.

Actions:

- Added `replace_duplicates` to `tab_handle_refresh` repair-policy validation in
  the daemon, HTTP ingress, MCP ingress, service schema, generated client
  template, and service-client helper.
- Implemented best-effort compatible duplicate cleanup for `replace_duplicates`.
  The refresh path reuses or opens one compatible target, preserves that selected
  target, closes other compatible live targets when possible, and returns
  `duplicateTargetCleanup` evidence.
- Added Rust coverage for compatible duplicate target selection and client
  coverage proving the new policy is accepted and forwarded.
- Updated README, CLI help, docs site, Plan 0044, `ROADMAP.md`, and repo plus
  installed skill guidance.

Validation run:

- `pnpm generate:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml tab_handle_refresh -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-request-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm test:route-confusion-gates`
- `pnpm --dir docs build`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- All listed checks passed.
- P44 remains open. Slice F still needs dashboard stale-target URL recovery and a
  route-bound repeat-open live proof that verifies one intended active target.
- Slice D also remains open on the interactive sudo boundary for refreshing the
  installed privileged helper and proving a cold browser-control-ready route
  desktop.

## Turn 21 | 2026-06-23

Scope: start P44 Slice E by returning structured route-bound operator-visible
proof components.

Actions:

- Extended successful `remote_view_open` `operatorVisible` output with selected
  target evidence plus route, display, browser, tab, stream, and Guacamole
  component states while preserving the existing `proof` field.
- Updated `summarizeServiceRemoteViewOpenProof()` to prefer
  `operatorVisible.target` and `operatorVisible.components` before falling back
  to the tab result.
- Updated CLI help, README, docs site, and repo plus installed skill guidance
  for the richer `operatorVisible` proof shape.
- Added selected-target URL readiness so a visible browser with the wrong
  selected tab reports `operatorVisible.state=wrong_tab`, with
  `components.display.state=ready` and `components.tab.state=wrong_tab`.
- Added Guacamole route availability to the same proof vocabulary so ready
  display and tab evidence with a missing or non-ready operator route reports
  `operatorVisible.state=guacamole_route_unavailable`.
- Added CDP target availability to the selected-tab proof so URL-bearing tab
  results without a CDP `targetId` report
  `operatorVisible.state=cdp_target_unavailable`.
- Added retained route metadata to the route proof so stale or mismatched
  route-pool allocation records report
  `operatorVisible.state=stale_route_record`.
- Added dashboard readiness fixture coverage so workspace rows preserve
  `wrong_tab`, `guacamole_route_unavailable`, `cdp_target_unavailable`, and
  `stale_route_record` from structured stream readiness while keeping View and
  Control disabled with state-specific reasons.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_operator_visible_reports_ready_proof -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_operator_visible -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-request-client`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:route-confusion-gates`
- `node --no-warnings --experimental-strip-types scripts/test-dashboard-workspace-nodes.js`
- `pnpm test:dashboard-view-streams`
- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`

Result:

- Focused Rust proof coverage, the full `remote_view_open` filter, service
  model and CDP stream tests, clippy, service-client helpers, API/MCP parity,
  route-confusion gates, docs build, skill sync, diff hygiene, and the live CDP
  tab streaming smoke passed. Slice E remains open for failure-case proof
  vocabulary and dashboard readiness fixture coverage.

## Turn 20 | 2026-06-23

Scope: continue P44 Slice D by making stale installed route desktop helpers
visible in fast doctor surfaces.

Actions:

- Added `helperDesktopSession` inspection to `agent-browser install doctor` and
  `agent-browser doctor remote-view`; both parse the installed privileged
  helper's `.xsession` heredoc and classify terminal-first, missing, unreadable,
  incomplete, or browser-control-ready templates.
- Added `remote_view_route_desktop_helper_stale` issue reporting to both doctor
  surfaces when a root-owned helper exists but still writes a terminal-first
  route desktop.
- Added text output for route desktop helper state and focused Rust coverage for
  terminal-first rejection, idle Openbox acceptance, and stale-helper issue
  generation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_helper_desktop_session -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml reports_stale_remote_view_helper_desktop_template -- --nocapture`
- `pnpm test:route-confusion-gates`
- `cargo build --manifest-path cli/Cargo.toml`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cli/target/debug/agent-browser --json install doctor`
- `cli/target/debug/agent-browser --json doctor remote-view`

Result:

- Both focused Rust test groups passed, route-confusion gates passed, debug build
  passed, and clippy passed.
- The rebuilt debug `install doctor` and `doctor remote-view` readbacks both
  reported `helperDesktopSession.state=terminal_first_template`,
  `terminalStartupDetected=true`, and issue code
  `remote_view_route_desktop_helper_stale` for the currently installed helper.
- The live route proof remains blocked on refreshing the root-owned helper from
  an interactive sudo shell and then starting a cold route session.

## Turn 19 | 2026-06-21

Scope: repair the Plan 0039 audit findings after closeout review.

Actions:

- Made `agent-browser remote-view open` accept the documented
  `--browser-build stealthcdp_chromium` and `--provider rdp_gateway` flags.
- Added post-launch failure cleanup to `remote_view_open`: tab open, focus, visible-window proof, or checkout failures now clean up before returning the typed error. New
  browser launches close the browser; reused retained browsers preserve the
  browser process and close only the opened tab when possible.
- Updated CLI help, README, docs command page, repo skill guidance, Plan 0039,
  and P16 roadmap text for the accepted flags and cleanup boundary.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_builds_route_bound_service_action -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_config -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- The focused Plan 0039 parser and cleanup tests passed, the non-live Rust,
  client, docs, and dashboard gates above passed, and the installed skill copy
  matches the repo skill.
- The direct documented dry-run command
  `agent-browser remote-view open --runtime-profile stealthcdp-default
  --browser-build stealthcdp_chromium --provider rdp_gateway --url
  https://www.linkedin.com/ --dry-run` returned `success=true` and
  `status=planned`.
- The repo-wide planning audit still reports older unrelated drift, but the
  Plan 0039 row remains clean: `state=CLOSED`, `current_state_ok=true`,
  `wired_in_roadmap=true`, and `wired_in_runbook=true`.

## Turn 18 | 2026-06-21

Scope: close Plan 0039 by making the route-specific `remote_view_open` lane the
documented default and proving it on the installed binary.

Actions:

- Added prelaunch route-display access repair to `remote_view_open`: it probes
  the selected route display, invokes the installed privileged helper when
  access is missing, and fails with typed display-access errors if access still
  cannot be proven.
- Fixed route binding selection so checked-out retained routes reuse their
  existing display allocation when no inline route material overrides them.
- Updated README, CLI help, docs site, service-request contract description,
  repo skill, installed skill, Plan 0039, ROADMAP, and downstream handoff note
  `docs/dev/notes/2026-06-21-remote-view-open-route-specific-handoff.md`.
- Rebuilt and installed binary SHA
  `54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad` into
  `~/.local/bin/agent-browser`, `bin/agent-browser-linux-x64`, and the pnpm
  package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `pnpm test:service-client`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `pnpm test:remote-view-open-fixture-live`
- `pnpm test:rdp-guac-many-to-many-live`
- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- `agent-browser install doctor --json` passed with no issues and aligned SHA
  `54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad`.
- `agent-browser doctor remote-view --json` reported `status=ready`,
  `remoteControl.status=ready`, `remoteControl.routeId=guacamole:3`,
  `remoteControl.displayName=:11`, and `manyToMany.status=ready`.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-24-32-095Z`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-24-32-207Z`.
- `git diff --check` passed.
- The repo-wide planning audit still reports older unrelated planning-contract
  drift, but the Plan 0039 row is clean: `state=CLOSED`,
  `current_state_ok=true`, `wired_in_roadmap=true`, and
  `wired_in_runbook=true`.
- Plan 0039 and P16 are closed.

## Turn 17 | 2026-06-20

Scope: continue Plan 0039 remote-control ready command hardening after the
route-specific Guacamole/RDP lane exposed stale retained route state.

Actions:

- Repaired the retained service route pool from the current route-pool
  readiness report after backing up
  `~/.agent-browser/service/state.json.pre-route-pool-refresh-2026-06-21T00-56-42-211Z`.
- Changed `remote_view_open` route binding to prefer supplied/current
  route-pool identity over stale retained route id and display allocation
  state.
- Made requested route-pool entry id authoritative for allocation lookup and
  allowed top-level `readiness.state=ready` route-pool entries to be used even
  when informational nested components are not ready.
- Updated the remote-view open live smoke to use the selected route entry's
  display name and display isolation for CLI, HTTP, state, and X11 checks.
- Rebuilt and installed the local binary into `~/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the pnpm global package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_dry_run_prefers_inline_route_pool_identity_over_stale_state -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/smoke-rdp-guac-route-pool-readiness.js`
- `node --check scripts/open-rdp-guac-route-displays.js`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-remote-view-open-live.js`
- `pnpm test:remote-view-open-fixture-live`
- `pnpm test:rdp-guac-many-to-many-live`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `git diff --check`

Result:

- Route-specific `remote-view open` dry-run resolves `guacamole-rdp-a` to
  `guacamole:3`, display `:11`, and display allocation
  `remote-view-display:11`.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-05-37-262Z`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-05-55-809Z`.
- `agent-browser doctor remote-view --json` reports `status=ready`,
  `remoteControl.status=ready`, and `manyToMany.status=ready`.
- Plan 0039 remains open only for Slice F documentation and downstream
  handoff closeout.

## Turn 1 | 2026-05-26

Scope: repair the planning contract after adopting Graphiti and CodeGraph
policy modules.

Actions:

- Added top-level `ROADMAP.md` as the planning index.
- Added top-level `RUNBOOK.md` as the dated execution log.
- Wired `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`
  into both planning authorities.
- Changed plan 0001's deterministic plan state to `CLOSED` while preserving
  its `VALIDATED` outcome.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed for the planning-contract repair.

## Turn 2 | 2026-05-27

Scope: create the Guacamole remote-view routing hardening lane after roadmap
alignment review.

Actions:

- Added `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`.
- Added P02 to `ROADMAP.md`.
- Kept P01 closed and made the hardcoded Guacamole route, metadata-only
  `view_takeover`, and external-open race the explicit P02 scope.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed for the P02 planning turn.

## Turn 3 | 2026-05-27

Scope: implement the first Guacamole route hardening slices.

Actions:

- Added `docs/dev/notes/2026-05-27-guac-route-authority-audit.md`.
- Added service-owned `ViewStream` route metadata: `frameUrl`,
  `externalUrl`, `routeId`, `connectionId`, `connectionName`, and
  `routeSource`.
- Removed production Guacamole client-hash repair from Rust service status
  handling and the dashboard workspace viewport.
- Changed dashboard external open to await `view_takeover` acceptance before
  opening `externalUrl`.
- Changed `view_takeover` to return typed acceptance metadata and persist a
  `viewer_takeover_requested` service event with `viewerLeaseId` and route
  details.
- Updated README, CLI help, docs site pages, service contracts, generated
  observability client, harness artifacts, and the repo plus installed
  `agent-browser` skill.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_headed_view_stream -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml guacamole -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml view_takeover -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_events -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml apply_remote_headed_launch_env_hints -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml apply_daemon_env_forwards_keychain_settings -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-inspector-actions`
- `node --check scripts/smoke-remote-headed-utils.js`
- `node --check scripts/test-rdp-guac-browser-switch-live.js`
- `node --check scripts/test-rdp-guac-viewer-transfer-live.js`
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:0 pnpm test:rdp-guac-viewer-transfer-live`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:0 pnpm test:rdp-guac-browser-switch-live`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Local source and contract validation passed.
- Live readiness, viewer-transfer, and browser-switch validation passed for
  the configured shared Guacamole route.
- Viewer-transfer artifacts:
  `/tmp/agent-browser-rdp-guac-hardening-2026-05-27T19-40-36-319Z`
- Browser-switch artifacts:
  `/tmp/agent-browser-rdp-guac-browser-switch-2026-05-27T19-41-29-855Z`

## Turn 4 | 2026-05-29

Scope: refactor the P05 handoff after maintainer clarification that the
Guacamole/RDP campaign is not ready for a formal release.

Actions:

- Reframed P05 as a validated installed-runtime checkpoint instead of a release
  preparation lane.
- Replaced the P05 plan with
  `docs/dev/plans/0005-2026-05-29-runtime-checkpoint-and-no-release-handoff-plan.md`.
- Added P06 in
  `docs/dev/plans/0006-2026-05-29-guac-rdp-productization-hardening-plan.md`.
- Removed the public docs changelog `v0.27.0` entry and kept current work under
  `## Unreleased` in `CHANGELOG.md`.
- Kept `CHANGELOG.md` release markers around the latest published `0.26.1`
  release entry.
- Changed `.github/workflows/release.yml` to manual dispatch only so ordinary
  pushes to `main` cannot publish checkpoint work as a GitHub release.
- Updated `AGENTS.md` and `ROADMAP.md` with the formal release boundary:
  release only after the hardened many-to-many Guacamole/RDP operational
  milestone, including one-time-sudo install and fully diagnostic doctors.

Validation run:

- `git diff --check`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `pnpm version:sync`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `agent-browser --version`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Checks passed. The installed runtime reports `agent-browser 0.27.0`, install
  doctor is successful with matching installed, workspace, and pnpm package
  binary checksum
  `e99093bb46891983afe71c2bf992a5f5c1ded16ecbbd29504a3e9e55a16be33f`, and
  remote-view doctor reports route pool, route displays, display access,
  privileged helper, and simultaneous viewing readiness with
  `requiresInteractiveSudo=false`.

## Turn 5 | 2026-05-29

Scope: execute and refactor the first P06 slice after auditing the installed
checkpoint against the productization issues from P05.

Actions:

- Added install-doctor remote-view privilege readiness fields for helper,
  sudoers, group, membership, helper check, nested issues, and
  `requiresInteractiveSudo`.
- Added remote-view doctor top-level issue codes, remediations, viewer browser
  and OCR prerequisites, install drift propagation, sudoers readiness, and
  many-to-many prerequisite status.
- Changed the many-to-many live harness to prefer installed `agent-browser`,
  hydrate route-pool and route-display environment from remote-view doctor
  output, auto-discover common viewer browsers, and fail public Guacamole route
  URLs with `non_embeddable_guacamole_url`.
- Updated README, CLI help, docs site pages, the repo skill guidance, the P06
  plan, the roadmap, and the P06 validation note.
- Rebuilt and installed the checkpoint binary to the local command, workspace
  binary, and pnpm package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-utils.js`
- `pnpm --dir docs build`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
- `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`

Result:

- Installed doctor and remote-view doctor passed with no issues. The installed
  runtime checksum is
  `1b67077ccdb5e80d8667d3bcc8327e9c2a1a8521417c25280f71d059bc3b1694`.
- The public Guacamole URL invocation failed fast with the intended
  `non_embeddable_guacamole_url` precondition diagnostic.
- The local embeddable Guacamole many-to-many gate passed from the installed
  command with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-06-07-291Z`.
- P06 remains open for clean-machine first-install sudo proof and the
  install-doctor service-readiness ownership decision.

## Turn 6 | 2026-05-29

Scope: continue P06 by resolving the remaining install-doctor service
ownership decision and strengthening the already-provisioned privilege
installer re-run contract.

Actions:

- Added `data.service` to `agent-browser install doctor --json` using an
  isolated no-launch service-status probe.
- Made install doctor fail with `service_status_not_ready` when the no-launch
  service probe does not report ready.
- Changed `scripts/install-agent-browser-privileges.sh --apply` to exit before
  privileged changes when the helper source matches the installed helper, the
  sudoers file exists, the operator is in the `agent-browser` group, and
  `sudo -n <helper> check` succeeds.
- Updated CLI help, README, docs site installation/service-mode pages, skill
  guidance, the P06 plan, roadmap, and validation note.

Validation run:

- `cargo run --quiet --manifest-path cli/Cargo.toml -- install doctor --json`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `bash -n scripts/install-agent-browser-privileges.sh`
- `AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE=scripts/libexec/agent-browser-privileged-helper bash scripts/install-agent-browser-privileges.sh --dry-run`
- `AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE=scripts/libexec/agent-browser-privileged-helper bash scripts/install-agent-browser-privileges.sh --apply`
- `pnpm build:native`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- The source-build install doctor showed the new service probe as ready and
  no-launch, while still correctly reporting source/install binary drift.
- The already-provisioned helper installer re-run exited with "already ready"
  and made no privileged changes.
- The rebuilt installed runtime checksum is
  `1ec7a0528944fad76fc4b3c2539b57b15944a503126038e47fb9d8727bdfa53a`.
- Installed doctor and remote-view doctor passed with no issues, and install
  doctor reports `data.service.ready=true` plus `data.service.noLaunch=true`.
- P06 remains open for clean-host or equivalent reset-fixture proof that first
  install uses one clear sudo authorization boundary.

## Turn 7 | 2026-05-29

Scope: finish P06 by proving the first-install sudo boundary with an equivalent
clean reset fixture, validating route-pool restart durability, and running the
final installed gates.

Actions:

- Added `pnpm test:install-privileges-clean-fixture`, which runs the privilege
  installer against fake `sudo`, `getent`, `id`, `groupadd`, `usermod`, and
  `visudo` under a temp install root.
- Reordered the Linux install path so
  `agent-browser install --with-deps --with-remote-view-privileges` runs
  remote-view privilege setup before dependency installation.
- Added a Rust guard that keeps remote-view privilege setup before Linux
  dependency installation.
- Updated README, docs site installation guidance, skill guidance, P06 plan,
  roadmap, and P06 validation note.
- Rebuilt and installed the checkpoint binary to the local command, workspace
  binary, and pnpm package binary.

Validation run:

- `pnpm test:install-privileges-clean-fixture`
- `cargo test --manifest-path cli/Cargo.toml install_orders_remote_view_privileges_before_linux_deps -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `pnpm build:native`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`
- `docker restart agent-browser-guacamole agent-browser-guacd && node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`
- `pnpm sync:rdp-guac-existing-user-route-pool`
- `pnpm grant:rdp-route-display-access -- --apply`
- `agent-browser --json get title`
- `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
- `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`

Result:

- The clean-fixture smoke proved first apply uses exactly one explicit
  `sudo -v` boundary and second apply does not add another prompt boundary or
  repeat privileged install commands.
- Installed doctor and remote-view doctor passed with no issues. The final
  P06 installed runtime checksum is
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.
- Route-pool readiness survived Guacamole web and guacd restarts.
- Route sync and route-display access grant reruns passed without interactive
  sudo.
- Default command attach passed.
- The local embeddable Guacamole many-to-many gate passed with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-39-55-085Z`.
- The public Guacamole URL invocation failed fast with the intended
  `non_embeddable_guacamole_url` diagnostic and artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-40-34-292Z`.
- P06 is closed. Formal release work remains a separate lane.

## Turn 8 | 2026-05-29

Scope: open the formal release lane now that P06 closed the Guacamole/RDP
productization blocker.

Actions:

- Created
  `docs/dev/plans/0007-2026-05-29-v0-27-0-formal-release-plan.md`.
- Moved `CHANGELOG.md` release extraction markers from `0.26.1` to `0.27.0`.
- Added the public docs changelog entry for `v0.27.0` dated May 29, 2026.
- Added P07 to `ROADMAP.md`.
- Added release-preparation validation note
  `docs/dev/notes/2026-05-29-p07-v0-27-0-release-prep-validation.md`.

Validation run:

- `git log v0.26.1..HEAD --format='%an <%ae>' | sort -u`
- `pnpm version:sync`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm --dir docs build`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Local release-preparation validation passed. The installed runtime checksum
  remains
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.
- P07 remains open for release PR merge, release workflow dry run, real
  release workflow run, and GitHub release asset verification.

## Turn 9 | 2026-05-29

Scope: respond to the first manual `Release` workflow dry-run failure.

Actions:

- Ran the `Release` workflow with `dry_run=true` on `main`.
- Confirmed release-state precheck passed.
- Diagnosed the platform build failures as a Rust cfg leak in
  `cli/src/native/cdp/chrome.rs`.
- Kept the private remote-headed virtual-display fallback inside the Linux cfg
  block so non-Linux targets do not reference Linux-only helpers.
- Added
  `docs/dev/notes/2026-05-29-p07-release-dry-run-cross-target-fix.md`.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml private_remote_display -- --test-threads=1`
- `cargo check --manifest-path cli/Cargo.toml --target x86_64-pc-windows-gnu`

Result:

- Format, clippy, and the focused private remote-display unit test passed.
- The local Windows cross-target check advanced past the previous missing
  symbols, then stopped because this workstation lacks
  `x86_64-w64-mingw32-gcc` for the `ring` build script.
- The release workflow dry run must be retried after this fix lands on `main`.

## Turn 10 | 2026-05-29

Scope: respond to the second manual `Release` workflow dry-run failure.

Actions:

- Reran the `Release` workflow with `dry_run=true` on `main`.
- Confirmed Windows x64, macOS x64, and macOS ARM64 passed after the cfg fix.
- Diagnosed Linux target failures as release-time `-lX11` linking from the
  browser-focus helper.
- Changed the Linux X11 focus helper to load `libX11` dynamically with
  `dlopen` and `dlsym` at runtime instead of statically linking X11.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml browser -- --test-threads=1`
- `git diff --check`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `rg -n "#\\[link\\(name = \\\"X11\\\"\\)|-lX11" cli/src`

Result:

- Local validation passed.
- No static X11 link remains in `cli/src`.
- The local machine does not have `cargo-zigbuild`, so the release workflow
  dry run must be retried after this fix lands on `main`.

## Turn 11 | 2026-05-29

Scope: publish and verify the formal `v0.27.0` GitHub release.

Actions:

- Reran the manual `Release` workflow with `dry_run=true`.
- Ran the manual `Release` workflow with `dry_run=false` after the dry run
  passed.
- Verified the public GitHub release and asset list.
- Closed P07 in the roadmap and plan surfaces.

Validation run:

- `gh run view 26648621169 --json conclusion,url,headSha`
- `gh run view 26649196974 --json conclusion,url,headSha`
- `gh release view v0.27.0 --json tagName,name,url,isDraft,isPrerelease,assets,targetCommitish`
- `git fetch --tags origin`
- `git rev-list -n1 v0.27.0`
- `git rev-parse origin/main`

Result:

- Dry run succeeded:
  `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26648621169`
- Real release run succeeded:
  `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26649196974`
- Release URL:
  `https://github.com/CochranResearchGroup/agent-browser/releases/tag/v0.27.0`
- Release commit and `origin/main` both resolve to
  `17a284f8624e6108473970e2ec2b380debf9f7ac`.
- The release is not a draft, is not a prerelease, and has seven assets:
  `agent-browser-darwin-arm64`, `agent-browser-darwin-x64`,
  `agent-browser-linux-arm64`, `agent-browser-linux-musl-arm64`,
  `agent-browser-linux-musl-x64`, `agent-browser-linux-x64`, and
  `agent-browser-win32-x64.exe`.

## Turn 12 | 2026-05-29

Scope: repair stale planning-audit residue after the `v0.27.0` release.

Actions:

- Normalized historical runbook headings to the deterministic
  `## Turn N | YYYY-MM-DD` format.
- Changed P02 plan state from `VALIDATED` to deterministic `CLOSED` while
  preserving `Outcome: VALIDATED`.
- Changed P03 plan state from `COMPLETE` to deterministic `CLOSED` while
  preserving `Outcome: COMPLETE`.
- Wired the existing P03 and P04 plan filenames into this runbook:
  `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md` and
  `docs/dev/plans/0004-2026-05-29-release-candidate-install-validation-plan.md`.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed. The planning audit now reports `ok: true`, no problems,
  no open roadmap lanes, deterministic state for every plan, and runbook plus
  roadmap wiring for every plan.

## Turn 13 | 2026-05-30

Scope: open the CDP tab streaming lane for non-remote browsers.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for prior CDP streaming
  context.
- Inspected the existing CDP stream server, stream WebSocket, service
  view-stream model, action-derived view streams, dashboard view-stream
  rendering, roadmap, and runbook surfaces.
- Added
  `docs/dev/plans/0008-2026-05-30-cdp-tab-streaming-for-non-remote-browsers-plan.md`.
- Added P08 to `ROADMAP.md`.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`

Result:

- Planning audit passed with `ok: true`, no problems, and P08 wired through the
  roadmap, runbook, and open plan file.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` selected only `git diff --check` for
  the documentation-only planning slice.

## Turn 14 | 2026-06-04

Scope: open a resource monitor and garbage collector lane after live
agent-browser resource pressure cleanup.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for prior resource
  cleanup and service lifecycle context.
- Confirmed the related retained orphan profile cleanup plan exists, but it
  covers service-state/profile metadata rather than live OS process pressure.
- Added
  `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`.
- Added P13 to `ROADMAP.md` with the dry-run-first resource monitor and GC
  recommendation.

Validation run:

- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- P13 is open for Slice A and Slice B: read-only resource inventory plus
  conservative stale classification before any apply-mode garbage collection.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` included the pre-existing dirty
  dashboard files in its recommendation set, so it selected dashboard checks in
  addition to the documentation-only change.
- The planning audit still fails due to pre-existing roadmap/runbook drift for
  older plans, but the new P13 plan is wired in both `ROADMAP.md` and
  `RUNBOOK.md`.

## Turn 15 | 2026-06-05

Scope: open and start the minimal runtime-profile reuse lane after Plan 0026
closed the resource-monitor and GC cleanup surface.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for profile reuse,
  service queue, lease, and access-plan context.
- Added
  `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`.
- Updated P13 in `ROADMAP.md` so Plan 0026 is the closed cleanup surface and
  Plan 0027 is the prevention surface.

Current target:

- Plan 0027 Slice A: add a read-only access-plan `profileReuse` advisory that
  recommends `reuse_existing_browser`, `wait_for_profile_lease`, or
  `launch_new_browser` before any launch mutates runtime state.

## Turn 16 | 2026-06-13

Scope: write an implementation handoff note for AuraCall-driven browser
service feature requests.

Actions:

- Ran Graphiti discovery against `agent_browser_main` and verified the local
  Graphiti runtime was healthy.
- Reviewed the existing access-plan service-request handoff note and the
  service request/client contract surfaces.
- Added
  `docs/dev/notes/2026-06-13-auracall-cdp-feature-requests.md`.
- Patched the note so AuraCall source paths are explicitly relative to the
  sibling `../auracall` repository.

Validation run:

- `git diff --check`
- Verified the listed agent-browser source surfaces exist in this repository.
- Verified the listed AuraCall source surfaces exist under the sibling
  `../auracall` repository.
- Ran Graphiti discovery against `agent_browser_main` for AuraCall CDP
  migration, BYOP, controlled CDP attach, bounded evaluate, and service tab
  handle context.

Result:

- The handoff note requests profile-origin and BYOP registration, a
  lease-backed service tab handle, controlled CDP attach, bounded evaluate
  jobs, readiness and identity probe recipes, tab reuse repair, diagnostic
  evidence bundles, and service-client ergonomics.
- The note keeps provider-specific ChatGPT, Gemini, Grok, and AuraCall
  semantics out of agent-browser and frames the work as service-owned browser
  primitives for a future implementation agent.

## Turn 17 | 2026-06-13

Scope: open a high-level upgrade plan suitable for subagents and goal-driven
execution.

Actions:

- Added
  `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`.
- Added P14 to `ROADMAP.md`.
- Structured the plan as a parent goal with slice-level subagent prompts,
  acceptance criteria, coordination rules, validation matrix, and open
  questions.

Validation run:

- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `pnpm validation:select -- --base HEAD`

Result:

- P14 is open for profile origin/BYOP, lease-backed service tab handles,
  controlled CDP attach, bounded evaluate, diagnostics/readiness evidence, and
  client ergonomics.
- The first recommended implementation slice is P14 Slice A: profile-origin
  schema plus explicit BYOP registration/readback.

## Turn 18 | 2026-06-13

Scope: implement P14 Slice A profile-origin and BYOP registration/readback.

Actions:

- Added durable service profile origin values:
  `agent_browser_owned`, `external_byop`, and `external_observed`.
- Added external profile registration metadata and browser compatibility
  evidence to service profile records.
- Added `registerExternalProfile()` to
  `@agent-browser/client/service-observability` for explicit BYOP or observed
  external profile registration.
- Exposed `profileOrigin` through service profile allocation readback and
  access-plan selected profiles.
- Hardened retained-state orphan profile pruning so `external_byop` and
  `external_observed` profiles are never pruned as owned profile data.
- Preserved profile origin and external metadata through the dashboard profile
  config save path.
- Updated service schemas, generated client types, README, docs site, and the
  installed agent-browser skill.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_profiles -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml test_prune_retained_service_state_removes_orphaned_custom_profiles -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-profile-allocation`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Slice A is implemented as a no-launch contract slice.
- Access-plan and profile readback can distinguish owned, BYOP, and observed
  external profile lanes.
- Explicit external profile registration records caller identity, target
  identities, account ids, user-data directory, and browser compatibility
  evidence.
- The next recommended P14 slice is Slice B: lease-backed service tab handles.

## Turn 19 | 2026-06-13

Scope: implement P14 Slice B lease-backed service tab handles.

Actions:

- Added `ServiceTabHandle` and `ServiceTabHandleTraceFilter` to the service
  model.
- Derived tab handles from service state for `service tabs`, grouped browser
  `tabHandles`, and tab lifecycle trace event details.
- Extended direct `tab_new` responses with CDP target/session IDs and a
  conservative immediate `serviceTabHandle`.
- Added `getServiceTabHandle()` and `requireServiceTabHandle()` to
  `@agent-browser/client/service-request`.
- Updated service tab/browser schemas, generated client declarations, README,
  docs site, and the installed agent-browser skill.
- Added no-launch Rust and service-client fixtures for valid handles, binding
  fields, and stale-handle rejection.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Slice B is implemented as a no-launch contract slice.
- Software clients can use the returned service tab handle instead of
  rediscovering browser, session, profile, tab, target, lease, or trace
  identity.
- Stale handles fail closed through the client helper and expose explicit
  stale reasons in service readbacks.
- The selector recommended `pnpm test:service-cdp-tab-streaming-live` because
  browser/tab surfaces changed; that live smoke was deferred to Slice C unless
  live proof is requested before controlled CDP attach work starts.
- The next recommended P14 slice is Slice C: controlled CDP attach for leased
  service tab handles.

## Turn 20 | 2026-06-13

Scope: implement P14 Slice C controlled CDP attach for leased service tab
handles.

Actions:

- Added `cdp_attach` and `cdp_detach` to the service request action metadata,
  HTTP relay, MCP service request surface, Rust daemon dispatcher, JSON schema,
  generated client types, and `@agent-browser/client/service-request` helpers.
- Gated attach on a valid `serviceTabHandle`, `cdpAttachmentAllowed: true`,
  non-CDP-free posture, matching service session, handle freshness, and target
  identity.
- Returned a service-owned attach descriptor with browser, session, tab,
  target, profile, lease, cleanup, trace, websocket, and detach metadata.
- Made detach preserve the browser process by default and return explicit
  detach metadata.
- Updated README, docs site, repo skill, and installed agent-browser skill for
  the new attach/detach helper path.
- Updated P14 plan and ROADMAP so Slice D bounded evaluate is the next
  implementation target.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice C is implemented with no-launch policy and stale-handle coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-98925`,
  stream `37669`.
- A dedicated attach-read-detach live smoke remains as the validation gap before
  treating controlled attach as AuraCall migration proof.
- The next recommended P14 slice is Slice D: bounded evaluate against leased
  service tab handles.

## Turn 21 | 2026-06-13

Scope: implement P14 Slice D bounded evaluate against leased service tab
handles.

Actions:

- Added `evaluate` to the service request action metadata, HTTP relay, MCP
  service request surface, JSON schema, generated client types, and
  `@agent-browser/client/service-request` helpers.
- Required `serviceTabHandle`, `script` or `expression`, positive `timeoutMs`,
  and positive `maxReturnBytes` for service-owned evaluate requests.
- Made service-bound evaluate skip browser auto-launch, switch to the handle's
  CDP target, execute with a daemon-side timeout, cap serialized return data,
  and return URL/title plus truncation metadata.
- Added no-launch HTTP, MCP, and service-client coverage for missing handles,
  missing caps, stale handles, and helper request shape.
- Updated README, docs site, repo skill, installed agent-browser skill, P14
  plan, and ROADMAP for the new bounded evaluate helper path.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice D is implemented with no-launch contract coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-73918`,
  stream `37595`.
- A dedicated live bounded-evaluate smoke remains as the validation gap before
  treating bounded evaluate as AuraCall migration proof.
- Screenshot-on-failure capture is deferred to Slice E diagnostic bundles so
  screenshot storage, caps, and trace links are implemented in one evidence
  surface.
- The next recommended P14 slice is Slice E: diagnostics and readiness
  evidence.

## Turn 22 | 2026-06-13

Scope: implement the P14 Slice E diagnostic bundle sub-slice for leased service
tab handles.

Actions:

- Added `diagnostics` to service request action metadata, HTTP relay, MCP
  service request validation, Rust daemon dispatch, JSON schema, generated
  client types, and `@agent-browser/client/service-request` helpers.
- Required a valid `serviceTabHandle` and reused the service-owned queue and
  handle validation path rather than adding a caller-owned browser path.
- Returned a compact evidence bundle with URL/title, browser/session/tab
  identity, profile readiness, route/view metadata, browser health, console
  entries, page errors, recent request summaries, snapshot summary, caller
  context, trace filter, and optional screenshot path.
- Added no-launch client helper coverage for request shape, stale handles, and
  evidence count caps.
- Updated README, docs site, repo skill, P14 plan, and ROADMAP for the new
  diagnostic helper path.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm generate:service-client`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice E diagnostic bundles are implemented with no-launch contract coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-95746`,
  stream `36831`.
- Slice E remains open for readiness/freshness lifecycle gating and any focused
  live diagnostics smoke requested before AuraCall migration proof.

## Turn 23 | 2026-06-20

Scope: open the corrective planning lane for recurring Guacamole/RDP
false-ready states after the live LinkedIn manual-auth route repair.

Actions:

- Added
  `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`.
- Added P16 to `ROADMAP.md`.
- Made the combined readiness invariant explicit: a remote-control browser is
  ready only when the selected browser window is loaded, visible, and
  controllable through the selected external Guacamole/RDP route.
- Captured the two recurring failure classes as plan gates:
  - Guacamole unhappy document or internal error caused by schema, route, URL,
    or permission drift.
  - Terminal-only remote desktop caused by browser/display mismatch.
- Scoped the next fix as a generic one-command/API path,
  `agent-browser remote-view open` and service action `remote_view_open`,
  rather than a LinkedIn-specific or AuraCall-specific repair.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Focused Plan 0039 validation passed. The broad planning audit remains red
  from pre-existing historical plan drift, but it reports no Plan 0039
  problems.
- Implementation remains open under Plan 0039. Slice A and Slice B are the
  recommended parallel starting points.

## Turn 24 | 2026-06-22

Scope: open the runtime convergence lane after remote-view and dashboard
binary harmonization exposed remaining runtime identity confusion.

Actions:

- Added `docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md`.
- Added P42 to `ROADMAP.md`.
- Captured the missing invariant: the dashboard runtime manifest proves only
  the dashboard service identity, not every active daemon session, stream
  backend, route helper, retained browser row, or foreign CDP browser.
- Scoped executable slices for active runtime inventory, daemon executable
  SHA-256 convergence, actionable doctor remedies, idempotent remote-view
  bootstrap, live rail boundaries, and one-command local convergence.
- Kept P41 foreign CDP discovery as a separate dependency so non-owned browser
  addressability is not confused with lifecycle ownership.

Validation run:

- `git diff --check`

Result:

- P42 is active and not complete. Slice D is already in progress through the
  Guacamole Postgres/schema bootstrap guard. The next implementation slice is
  daemon executable SHA convergence and active runtime inventory.

## Turn 25 | 2026-06-22

Scope: execute P42 Slice B daemon executable SHA convergence.

Actions:

- Added daemon executable SHA metadata next to the existing daemon version
  metadata.
- Made daemon reuse compare the invoking executable SHA-256 against the daemon
  SHA metadata when the invoking executable can be hashed.
- Treated missing daemon SHA metadata as stale by default, with
  `AGENT_BROWSER_ALLOW_LEGACY_DAEMON_SHA_REUSE=1` as an explicit reviewed
  compatibility escape hatch.
- Extended stale daemon cleanup to remove `<session>.sha256`.
- Updated P42 Slice B completion notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml daemon_executable_sha -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml cleanup_stale_files_removes_version_and_executable_sha -- --nocapture`

Result:

- Focused daemon SHA convergence tests passed. P42 remains open for active
  runtime inventory, doctor remedies, live rail convergence boundaries, and
  one-command local convergence.

## Turn 26 | 2026-06-22

Scope: execute P42 Slice A active runtime inventory in doctor output.

Actions:

- Added `runtimeInventory` to `agent-browser install doctor --json`.
- The inventory scans the daemon socket metadata directory without launching
  Chrome and reports daemon session PID, PID liveness, package version match,
  executable SHA-256 match, stream port, and metadata presence.
- Added `active_runtime_stale_executable` install doctor issues for active
  daemon sessions whose metadata is stale or incomplete.
- Lifted the install doctor's runtime inventory into
  `agent-browser doctor remote-view --json` as top-level `runtimeInventory`.
- Updated P42 Slice A completion notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml runtime_inventory_from_install -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml daemon_executable_sha -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `./cli/target/debug/agent-browser install doctor --json`
- `./cli/target/debug/agent-browser doctor remote-view --json`

Result:

- Focused tests and clippy passed.
- The rebuilt debug-binary install doctor reported
  `runtimeInventory.status=stale`, `runtimeCount=4`, and `staleCount=4`.
- The rebuilt debug-binary remote-view doctor lifted the same inventory and
  reported `runtimeInventory.status=stale`. This intentionally made the
  debug-binary readback not remote-control ready against the installed runtime,
  proving stale active runtimes are no longer omitted from readiness.

## Turn 27 | 2026-06-22

Scope: execute the first P42 Slice C convergence doctor remedy.

Actions:

- Added session-scoped remedy metadata to `active_runtime_stale_executable`
  install doctor issues.
- Each stale daemon issue now carries `session`,
  `nextAction=restart_stale_daemon_session`, and an argv-safe remedy for
  `agent-browser close --session <session>`.
- Made remote-view doctor prefer
  `restart_stale_daemon_sessions_then_rerun_doctor` when install readiness is
  blocked by stale active daemon sessions.
- Updated P42 Slice C progress notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `./cli/target/debug/agent-browser install doctor --json`
- `./cli/target/debug/agent-browser doctor remote-view --json`

Result:

- Focused tests, clippy, and debug CLI build passed.
- The rebuilt debug-binary install doctor reported four
  `active_runtime_stale_executable` issues; the first issue carried
  `session=default`, `nextAction=restart_stale_daemon_session`, and
  `remedy.argv=["agent-browser","close","--session","default"]`.
- The rebuilt debug-binary remote-view doctor reported
  `nextAction=restart_stale_daemon_sessions_then_rerun_doctor` and a
  next-command explanation that points operators back to each issue's
  session-scoped `remedy.argv`.

## Turn 28 | 2026-06-22

Scope: execute P42 local binary/runtime convergence after publishing the
structured commits.

Actions:

- Extended `pnpm publish:local-dashboard` so it synchronizes the user-scoped
  install binary, ignored workspace package binary, and user pnpm package
  binary to the same freshly built executable by default.
- Added `--skip-reference-sync` for operator cases that intentionally do not
  want reference binaries changed.
- Published the current debug build to the local dashboard runtime and restarted
  `agent-browser-dashboard.service`.
- Applied the stale daemon restart path by invoking the three
  session-scoped remedies reported by install doctor. Those commands returned
  nonzero because `close --session` still routes through daemon restart, but
  the restart path did replace the stale daemon metadata and all active daemon
  rows converged.
- Reran publish after adding reference-binary sync so install doctor no longer
  failed on pnpm/workspace binary drift.

Validation run:

- `pnpm publish:local-dashboard -- --skip-browser --json`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- The publish report synchronized
  `/home/ecochran76/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the user pnpm global package binary to
  `94d1d022b4f1315b2f3eb9ff08fdc3faa816d77960500c6b6854cab98161cfa8`.
- Installed `agent-browser install doctor --json` reported `success=true`,
  `runtimeInventory.status=converged`, `staleCount=0`, no issue codes, and
  matching PATH, pnpm, and workspace binary SHA-256 values.
- Installed `agent-browser doctor remote-view --json` reported `success=true`,
  `status=ready`, `remoteControl.ready=true`,
  `runtimeInventory.status=converged`, and
  `nextAction=run_many_to_many_live_gate`.
- Follow-up required: make stale daemon close remedies return success without
  depending on a daemon restart side effect.

## Turn 29 | 2026-06-22

Scope: finish P42 close/remedy and install-doctor probe convergence discovered
during local execution.

Actions:

- Added a `close --session` prestart path that targets an existing daemon
  before daemon convergence startup.
- Added explicit-session stale metadata cleanup for unauthorized or non-ready
  daemon close attempts, returning success with a warning instead of trying to
  start a replacement daemon.
- Classified running PID metadata without an addressable socket, stream, or
  port as `diagnostic` instead of stale active runtime inventory.
- Changed `service status` to execute locally before daemon startup.
- Changed install doctor service-status probing to use a unique owned session,
  terminate the owned probe daemon after reading status, and treat the isolated
  no-state probe as no-launch ready.
- Ran service GC apply for the orphaned Xvfb candidate that was blocking local
  install doctor readiness.
- Published the final local runtime and synchronized reference binaries.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml force_close_session_from_metadata -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml close_targets_existing_daemon_before_prestart -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_status_locally_before_daemon -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm publish:local-dashboard -- --skip-browser --json`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Final local publish succeeded and restarted
  `agent-browser-dashboard.service`.
- Final installed executable SHA-256:
  `19ba0d616388e1eb84241eea5ddcffa56a1803831c5085acc25abb01277b78e6`.
- Reference binaries in `~/.local/bin`, ignored workspace `bin/`, and user
  pnpm global package path matched the installed executable SHA-256.
- Final installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeInventory.status=none`,
  `runtimeCount=0`, and `staleCount=0`.
- Final installed `agent-browser doctor remote-view --json` reported
  `success=true`, `status=ready`, `remoteControl.ready=true`,
  `runtimeInventory.status=none`, `staleCount=0`, and
  `nextAction=run_many_to_many_live_gate`.

## Turn 30 | 2026-06-22

Scope: finish P42 live-rail and one-command local runtime convergence.

Actions:

- Added `pnpm converge:local-runtime` as a dry-run by default local operator
  convergence command.
- In apply mode, the command runs local dashboard publication, applies only
  doctor-reported `agent-browser close --session <name>` stale-daemon
  remedies, runs the Guacamole Postgres schema ensure, runs route-pool
  readiness, applies route display-access grants only when remote-view doctor
  asks for them, and reruns final doctors.
- Added `pnpm test:local-runtime-convergence` to lock the command contract,
  foreign-process refusal boundary, display-grant sequencing, and retained
  evidence behavior.
- Marked P42 Slice E done from the dashboard live-rail contract tests and
  Slice F done from command validation.

Validation run:

- `node --check scripts/converge-local-runtime.js`
- `node --check scripts/test-local-runtime-convergence.js`
- `pnpm test:local-runtime-convergence`
- `pnpm --silent converge:local-runtime -- --json`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-evidence.json`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`

Result:

- Dry-run convergence returned `success=true`, final install doctor ready,
  final remote-view ready, zero safe stale remedies, and zero skipped remedies.
- Apply convergence returned `success=true`, wrote
  `/tmp/agent-browser-converge-local-runtime-evidence.json`, final install
  doctor ready, final remote-view ready, and zero skipped remedies.
- Dashboard workspace tests passed, proving the live rail keeps retained and
  no-action attention rows out of the live control surface and groups
  reachable non-owned CDP browsers separately.
- P42 remains open for Slice C stale dashboard/stream classifications and
  Slice D bootstrap hardening.

## Turn 31 | 2026-06-22

Scope: continue P42 Slice C by classifying stale or unreadable live dashboard
runtime manifests.

Actions:

- Added an install-doctor live dashboard manifest probe for the local
  `/api/runtime/manifest` endpoint.
- Kept dashboard-not-running as non-drift, but classified a running dashboard
  that serves no readable manifest or a mismatched executable SHA-256 as
  `dashboard_runtime_stale_or_unreadable`.
- Added a bounded remedy pointing to
  `pnpm converge:local-runtime -- --apply --json`.
- Updated remote-view doctor so that dashboard runtime drift recommends
  `converge_local_runtime_then_rerun_doctor` before generic install drift.
- Updated `pnpm converge:local-runtime -- --apply --json` so initial nonzero
  doctor JSON is treated as repairable input in apply mode instead of aborting
  before local publish.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `./cli/target/debug/agent-browser install doctor --json`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn31-final.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, clippy, and debug CLI build passed.
- The rebuilt debug install doctor reported
  `dashboard_runtime_stale_or_unreadable` with `state=stale_executable` when
  the running dashboard manifest executable SHA-256 did not match the debug
  executable.
- Convergence apply started with initial install issue
  `dashboard_runtime_stale_or_unreadable`, published the new local runtime, and
  ended with final install doctor ready, final remote-view ready, zero skipped
  remedies, and retained evidence at
  `/tmp/agent-browser-converge-local-runtime-turn31-final.json`.
- Direct installed `agent-browser install doctor --json` then reported
  `success=true`, no issue codes, `liveDashboardRuntime.ready=true`,
  `liveDashboardRuntime.state=ready`, and `runtimeInventory.status=none`.
- P42 Slice C still has remaining stale stream-backend classification work.

## Turn 32 | 2026-06-22

Scope: continue P42 Slice C by adding explicit runtime convergence summary
states.

Actions:

- Added install-doctor `runtimeConvergence` with schema
  `agent-browser.runtime-convergence.v1`.
- Derived summary status from runtime inventory plus live dashboard manifest
  state, using `converged`, `partial`, `stale`, and
  `manual_review_required`.
- Lifted the summary into remote-view doctor and printed it in text output
  separately from raw runtime inventory status.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml runtime_convergence -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn32.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, and clippy passed.
- Unit coverage now locks the `converged`, `partial`, `stale`, and
  `manual_review_required` summary statuses plus remote-view summary lifting.
- Convergence apply published the summary-state build and ended with final
  install doctor ready, final remote-view ready, zero skipped remedies, and
  retained evidence at `/tmp/agent-browser-converge-local-runtime-turn32.json`.
- Direct installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeConvergence.status=converged`,
  `liveDashboardRuntime.state=ready`, and `runtimeInventory.status=none`.
- P42 Slice C still has remaining stale stream-backend and diagnostic
  retained-row classification work.

## Turn 33 | 2026-06-22

Scope: finish P42 Slice C stale stream-backend classification.

Actions:

- Extended runtime inventory to probe advertised daemon stream ports.
- Added runtime row `streamReachable` and `driftReasons` evidence.
- Classified live daemon sessions with unreachable or invalid stream metadata
  as stale instead of converged.
- Added install-doctor issue code `active_runtime_stale_stream_backend` with
  the bounded `agent-browser close --session <session>` remedy.
- Updated remote-view doctor to treat stale stream backends as a
  session-scoped daemon restart prerequisite before generic install drift.
- Marked P42 Slice C done.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml stream_backend -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn33.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, and clippy passed.
- Unit coverage now proves unreachable stream metadata produces a stale runtime
  inventory row, install doctor emits
  `active_runtime_stale_stream_backend`, and remote-view doctor recommends the
  same session-scoped restart prerequisite.
- Convergence apply published the stream-backend build and ended with final
  install doctor ready, final remote-view ready, zero skipped remedies, and
  retained evidence at `/tmp/agent-browser-converge-local-runtime-turn33.json`.
- Direct installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeConvergence.status=converged`,
  `staleRuntimeCount=0`, and `runtimeInventory.status=none`.

## Turn 34 | 2026-06-22

Scope: close P42 by auditing and validating Slice D idempotent remote-view
bootstrap.

Actions:

- Verified `pnpm ensure:rdp-guac-postgres -- --apply` exists and is invoked by
  local convergence.
- Verified route-pool setup, existing-user route sync, and legacy autologin
  setup call the shared Guacamole Postgres schema guard before writing records.
- Verified the schema guard refuses partial `guacamole_*` relation state,
  imports only absent schema state, waits for Postgres readiness, and
  checkpoints after ready/imported states.
- Verified the live Guacamole compose file keeps explicit Postgres durability
  settings for WSL hard-stop resilience.
- Marked P42 `State: CLOSED`.

Validation run:

- `bash scripts/ensure-rdp-guac-postgres.sh --dry-run`
- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
- `agent-browser doctor remote-view --json`

Result:

- Schema guard dry-run reported `Guacamole Postgres schema is ready.`
- Route-pool readiness reported `success=true`; Postgres, schema, Guacamole
  web/login, guacd, RDP connections, connection permissions, distinct targets,
  and both RDP backend TCP checks were ready.
- Direct installed remote-view doctor reported `success=true`, `status=ready`,
  `remoteControl.ready=true`, `runtimeConvergence.status=converged`,
  `runtimeInventory.status=none`, and
  `nextAction=run_many_to_many_live_gate`.

## Turn 35 | 2026-06-22

Scope: investigate the `last30days` Facebook remote-view friction and open the
next route-handoff audit lane.

Actions:

- Read the incident note at
  `docs/dev/notes/2026-06-22-facebook-remote-view-open-friction.md`.
- Used Graphiti discovery for advisory prior context and CodeGraph for the
  route-binding and dashboard stream helper joins.
- Captured live readbacks from `agent-browser doctor remote-view --json`,
  `agent-browser service browsers --json`, and
  `agent-browser service tabs --json`.
- Added P43 in
  `docs/dev/plans/0043-2026-06-22-route-handoff-confusion-audit-plan.md`.
- Updated `ROADMAP.md` with the open P43 lane.

Findings:

- P42 binary/runtime convergence remains green. The failure sits above that
  layer.
- `session:default` owns the `last30days-facebook` browser on display `:11`
  with Facebook tabs and a generic Guacamole stream.
- `session:litscout-ai-smoke-clean` is a separate browser on display `:93`
  with several `127.0.0.1` tabs and its own generic Guacamole stream.
- The dashboard has stream metadata that can embed Guacamole, but it does not
  yet require row-bound proof that the stream is showing the intended browser
  instead of a terminal.

Validation run:

- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` selected only `git diff --check` for
  the docs-only change set.
- The planning-contract audit still fails on pre-existing older plan wiring and
  deterministic-state debt. The new P43 plan itself is reported with
  `filename_ok=true`, `lane_ok=true`, `state_ok=true`,
  `wired_in_roadmap=true`, and `wired_in_runbook=true`.

## Turn 36 | 2026-06-22

Scope: execute P43 Slice A with a read-only route-handoff audit surface.

Actions:

- Added `scripts/audit-route-handoff.js`.
- Added package command `pnpm audit:route-handoff`.
- Added no-launch fixture coverage in `scripts/test-route-handoff-audit.js`.
- Added package command `pnpm test:route-handoff-audit`.
- Documented the audit command in `README.md`.
- Marked P43 Slice A done and updated `ROADMAP.md` next recommendation.

Validation run:

- `node --check scripts/audit-route-handoff.js`
- `node --check scripts/test-route-handoff-audit.js`
- `pnpm test:route-handoff-audit`
- `pnpm --silent audit:route-handoff -- --json --skip-doctor`
- `pnpm --silent audit:route-handoff -- --json`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:route-handoff-audit && pnpm --silent audit:route-handoff -- --json --skip-doctor | jq -e '.success == true and .data.summary.route_bound_ready == 2 and .data.summary.direct_remote_headed == 11'`

Result:

- Syntax checks passed.
- The fixture test passed and covers `route_bound_ready`,
  `route_bound_proof_missing`, `route_bound_terminal_only`,
  `direct_remote_headed`, `foreign_cdp`, and `stale_or_retained`
  classifications.
- The live read-only audit with `--skip-doctor` returned `success=true`,
  `collections.browsers=2`, `collections.tabs=13`, and summary
  `route_bound_ready=2`, `direct_remote_headed=11`.
- The full live audit also returned `success=true`, no collection errors,
  `runtime.convergenceStatus=converged`,
  `runtime.inventoryStatus=converged`, `runtime.runtimeCount=1`, and
  `runtime.remoteControlStatus=ready`.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` recommended `git diff --check` and
  `node scripts/dev/select-validation.js --base HEAD --json`; both passed.
- The combined fixture plus live summary assertion passed.

## Turn 37 | 2026-06-22

Scope: execute P43 Slice B one-line CLI contract and help.

Actions:

- Added command-specific `remote-view` help covering
  `agent-browser remote-view open`.
- Added the Facebook-style one-liner and flag placement guidance to CLI help.
- Changed `parse_remote_view_open` to copy global `--session-name` into the
  `remote_view_open` request.
- Added parser tests for post-subcommand `--runtime-profile`, `--session`,
  `--session-name`, `--browser-build`, and `--provider` placement.
- Updated `README.md`, `docs/src/app/commands/page.mdx`, and
  `skills/agent-browser/SKILL.md`.
- Marked P43 Slice B done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture`
- `cargo run --quiet --manifest-path cli/Cargo.toml -- remote-view open --help | rg -n "Facebook|Global placement|--session selects|last30days-facebook"`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Rust format passed after applying `cargo fmt`.
- Focused Rust tests passed: 10 passed, 0 failed.
- Help output includes the global placement section, Facebook examples, and
  the `--session` versus `--session-name` distinction.
- Docs build passed.
- Clippy passed with `-D warnings`.
- Validation selector required the Rust format, focused Rust test, clippy,
  docs build, diff hygiene, and repo-installed skill sync checks.
- The repo and installed `agent-browser` skill copies now match.

## Turn 38 | 2026-06-22

Scope: execute P43 Slice C route allocation diagnostics.

Actions:

- Added compact route-pool diagnostic JSON for `route_pool_unavailable`.
- Added the same diagnostic context to stale explicit pool-entry failures:
  `route_pool_entry_missing` and `route_pool_entry_unavailable`.
- Included requested route, route-pool entry, display allocation, display name,
  display isolation, owner browser, owner session, profile, provider, matching
  pool entries, available pool entries, ready display allocation IDs, existing
  remote-view routes, and recommended commands.
- Kept the existing string error-code contract intact so callers that check
  `route_pool_unavailable` continue to work.
- Tightened route-pool tests to parse the diagnostic suffix and assert the
  actionable identity fields.
- Marked P43 Slice C done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml route_pool -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:service-cdp-tab-streaming-live`
- Direct temp-session probe:
  `HOME=<temp> AGENT_BROWSER_HOME=<temp>/.agent-browser AGENT_BROWSER_SOCKET_DIR=<temp>/s cargo run --quiet --manifest-path cli/Cargo.toml -- --json --session daemon-probe stream status`

Result:

- Focused route-pool Rust tests passed: 12 passed, 0 failed.
- Focused CDP stream Rust tests passed: 3 passed, 0 failed.
- Rust format, clippy, and diff hygiene passed.
- The new route-pool unavailable test verifies stable error code retention and
  machine-readable diagnostic fields for requested display identity, checked
  out matching pool entry, ready display allocations, and recommended repair
  command.
- The selector-recommended live CDP smoke did not reach the CDP path. It failed
  while starting a temporary daemon with
  `Daemon failed to start (socket: <temp>/s/<session>.sock)`. A direct
  temp-session `stream status` probe reproduced the same daemon-start failure
  and left only pid/token/version files. This appears independent of the
  route-pool diagnostic change and should be handled as a separate daemon
  startup validation issue.

## Turn 39 | 2026-06-22

Scope: execute P43 Slice D profile-lock ownership diagnostics.

Actions:

- Added profile-lock diagnostic JSON to Chrome profile lock failures while
  preserving the existing hard stop against launching a second Chrome process
  on the same user-data-dir.
- The diagnostic includes lock PID, user-data-dir, matching runtime profile
  state, matching service browser rows, primary owner, and safe remedies.
- Known service-owned locks now identify browser ID, active session, profile,
  host, health, PID, CDP endpoint, display, display allocation, and view stream
  IDs when persisted service state has them.
- Remedies include exact session-scoped service-status reuse and close commands
  for known owners, runtime-profile inspection and attach commands for matching
  runtime state, service-status inspection for unknown owners, and explicit
  separate-profile guidance for intentionally separate identities.
- Updated README, CLI runtime help, docs command page, and the
  `agent-browser` skill.
- Marked P43 Slice D done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml locked_profile -- --test-threads=1 --nocapture`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Focused profile-lock tests passed for known service/runtime owner diagnostics
  and unknown-owner diagnostics.
- Docs build passed with the known Next.js multiple-lockfile root warning.
- Clippy, diff hygiene, validation selector, and installed-skill sync passed.

## Turn 40 | 2026-06-22

Scope: execute P43 Slice E operator-visible success contract.

Actions:

- Added top-level `operatorVisible` to `remote-view open` dry-run and opened
  responses.
- Dry-runs report `operatorVisible.state=not_checked` with route, browser,
  session, display, provider, and display allocation identity.
- Successful opened responses report `operatorVisible.state=ready` and include
  the visible-window proof that already gates success.
- Added dry-run assertions and a pure ready-proof unit test for the
  `operatorVisible` contract.
- Updated README, CLI remote-view help, docs command page, and the
  `agent-browser` skill to tell clients to require
  `operatorVisible.state=ready`.
- Marked P43 Slice E done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Focused remote-view-open tests passed, including the new
  `operatorVisible` dry-run and ready-proof coverage.
- Docs build passed with the known Next.js multiple-lockfile root warning.
- Clippy, format check, diff hygiene, validation selector, installed-skill
  sync, and the no-launch CDP stream test passed.
- The selector still recommends `pnpm test:service-cdp-tab-streaming-live`
  because `actions.rs` changed; the same live smoke was already attempted in
  Turn 38 and failed before CDP validation while starting a temporary daemon.

## Turn 41 | 2026-06-22

Scope: execute P43 Slice F dashboard row binding and route-proof UX.

Actions:

- Added `operatorVisibleState` and `operatorVisibleReason` to dashboard
  workspace view-stream rows.
- Required current browser-window proof before RDP gateway View, Control, or
  external open actions are enabled.
- Kept terminal-only, idle-display, and missing-proof route rows in the live
  owned group as disabled diagnostics rather than moving them into a no-action
  attention category.
- Preserved detected non-owned browser grouping and retained-record filtering in
  the live workspace navigator.
- Updated README, docs dashboard/service/commands pages, the `agent-browser`
  skill, P43, and `ROADMAP.md`.

Validation run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm publish:local-dashboard -- --expect-marker "operator-visible proof missing" --skip-browser --json`

Result:

- Dashboard workspace node, navigator, selected-context, chat-packet, and
  console smokes passed.
- Docs and dashboard builds passed with the known Next.js multiple-lockfile and
  static-export rewrite warnings.
- Diff hygiene, validation selector, selector JSON, and installed-skill sync
  passed.
- The local dashboard runtime was rebuilt into
  `/home/ecochran76/.local/bin/agent-browser`, `agent-browser-dashboard.service`
  restarted, `/api/runtime/manifest` matched the installed executable SHA
  `f626320b5d084f824917560bdad60c8111678896cf81299a602c2d3a35c9d0a6`, and the
  served chunk contained `operator-visible proof missing`.
- The full publish browser smoke was attempted first and failed at the known
  temp-daemon startup boundary:
  `Daemon failed to start (socket: /run/user/1000/agent-browser/local-dashboard-runtime-smoke-2846318.sock)`.
  The final publish used `--skip-browser`, so live browser launch remains
  covered by the separate temp-daemon startup blocker rather than this Slice F
  dashboard contract.

## Turn 42 | 2026-06-22

Scope: execute P43 Slice G downstream client contract and last30days handoff
guidance.

Actions:

- Made `requestServiceRemoteViewOpen` require `operatorVisible.state=ready`
  before returning non-dry-run handoff success.
- Added service-client helpers for reading operator-visible state, checking
  readiness, throwing on invalid handoff proof, and logging one compact route,
  tab, profile, and visual-proof summary line.
- Kept dry-run remote-view open responses allowed as `not_checked` and made
  infrastructure-only readiness an explicit client opt-in that is not posted to
  the service API.
- Updated README, docs commands page, service-client examples, generated client
  types, and the installed `agent-browser` skill.
- Updated `last30days` so Facebook uses the route-bound
  `agent-browser remote-view open` one-liner with the `last30days-facebook`
  runtime profile and rejects missing-proof, CDP-only, or terminal-only
  Guacamole/RDP handoff success.
- Marked P43 Slice G done and moved `ROADMAP.md` to Slice H live gates.

Validation run:

- `git diff --check`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `uv run pytest tests/test_facebook.py`
- `python3 -m py_compile skills/last30days/scripts/lib/facebook.py skills/last30days/scripts/lib/env.py`

Result:

- Service API/MCP parity, service-client contract/type/export/request/helper
  smokes, docs build, diff hygiene, and installed-skill sync passed.
- Focused last30days Facebook tests passed with 9 tests, including the
  terminal-only rejection case.
- P43 remains open for Slice H. The next gate needs no-launch route-confusion
  fixtures and an OCR-backed live route proof that fails on terminal-only
  route displays.

## Turn 43 | 2026-06-22

Scope: execute P43 Slice H live gates and close the route-handoff confusion
audit lane.

Actions:

- Added `pnpm test:route-confusion-gates` as the focused no-launch gate for
  route-handoff confusion regressions.
- Covered wrong flag placement, named-session route-pool mismatch,
  same-owner route-pool repeat checkout, known-owner profile-lock messaging,
  direct remote-headed audit classification, and dashboard missing-proof plus
  terminal-only row classification.
- Updated validation selection so route, dashboard stream, service-client, and
  remote-view command changes recommend the route-confusion gate.
- Strengthened the live `remote-view open` fixture smoke with isolated daemon
  session and runtime profile defaults, bounded daemon-start retry, available
  route-pool selection, repeat handoff through the first route/display
  identity, route-handoff audit assertion, and OCR of the route display.
- Fixed the route-pool checkout resolver so an already checked-out route is
  reusable only for the same ready route, browser, session, and display
  allocation. Other owners still receive `route_pool_unavailable`.
- Marked P43 complete in the plan and roadmap.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:route-confusion-gates`
- `AGENT_BROWSER_COMMAND=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Diff hygiene, Rust formatting, validation selector JSON, clippy, no-launch
  CDP stream regressions, and the route-confusion gate passed.
- The OCR-backed live route gate passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-22T16-23-29-784Z`,
  route `guacamole:5`, display allocation `remote-view-display:12`,
  route-handoff classification `route_bound_ready`, visual state
  `browser_window_visible`, and fixture text
  `REMOTE VIEW OPEN FIXTURE 3815575`.
- `pnpm test:service-cdp-tab-streaming-live` was retried twice and failed
  before CDP validation at the known temporary-daemon startup boundary:
  `Daemon failed to start`.

## Turn 44 | 2026-06-22

Scope: diagnose and repair `pnpm test:service-cdp-tab-streaming-live`.

## Turn 45 | 2026-06-22

Scope: execute P44 Slice A intent normalization and remote-view provider
harmonization.

Actions:

- Added `RemoteViewOpenIntent` normalization for `remote_view_open` before
  route binding or launch.
- Made `viewStreamProvider` the canonical remote-view stream field and kept
  `provider=rdp_gateway` as a compatibility alias.
- Rejected provider/view-stream conflicts before acquisition.
- Updated CLI help, README, docs site, repo skill guidance, Plan 0044, and
  `ROADMAP.md`.
- Added focused CLI parser tests and remote-view intent normalization tests.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml normalize_remote_view_open_intent -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:route-confusion-gates`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:browser-capability-registry-draft`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `node scripts/dev/select-validation.js --base HEAD --json`

Result:

- All listed no-launch checks passed.
- `pnpm test:service-cdp-tab-streaming-live` was selected by validation but
  not rerun in this slice because the prior P43 closeout recorded the existing
  temporary-daemon startup boundary before CDP validation.

Actions:

- Reproduced the original failure with an isolated temp home and debug daemon
  logs. The client timed out before the daemon bound its socket, but the daemon
  stayed alive and became usable seconds later.
- Added daemon startup milestones under `--debug`.
- Moved Unix control-socket bind ahead of stream-server startup.
- Moved executable SHA calculation out of the daemon startup critical path by
  writing a short-lived `pending` marker and filling the real SHA in a
  background task. The client tolerates `pending` only during startup grace.
- Avoided hashing the current executable on fresh daemon startup unless an
  already-running daemon must be compared.
- Added a bounded smoke retry around first `stream status` daemon startup.
- Fixed service-owned `navigate` so it persists the active tab record and
  service tab handle, matching the existing `tab_new` retained-tab contract.
- Hardened the CDP tab streaming smoke diagnostics and allowed data-URL marker
  matching when Chrome has not populated a tab title yet.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml test_daemon_executable_sha_pending_is_startup_grace_only -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:route-confusion-gates`
- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm validation:select -- --base HEAD --json`

Result:

- Isolated first-command startup dropped from roughly 10 to 13 seconds to
  about 145 ms on the debug binary.
- The original live smoke passed end to end:
  `Service CDP tab streaming live smoke passed`.

## Turn 46 | 2026-06-22

Scope: execute P44 Slice B no-mutation acquisition planner.

Actions:

- Added `RemoteViewAcquisitionPlan` for route-bound remote-view acquisition.
- Routed `remote_view_open`, route preflight, and route checkout through the
  planner before state mutation.
- Moved route/display fallback selection into named planner decisions and
  surfaced `acquisitionPlan` in dry-run, opened, preflight, and checkout
  responses.
- Added blockers and diagnostics for unavailable route-pool entries and
  named-session display-allocation mismatches.
- Added planner fixtures for stale browser fallback ordering, checked-out
  same-owner reuse, checked-out other-owner rejection, named-session mismatch
  diagnostics, and dry-run acquisition-plan output.
- Updated Plan 0044 and `ROADMAP.md` so Slice C is the next P44 boundary.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture`
- `pnpm test:route-handoff-audit`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:route-confusion-gates`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm --dir docs build`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed no-launch checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-2019837` and stream `38157`.

## Turn 47 | 2026-06-22

Scope: execute the P44 Slice C no-launch acquisition lease and rollback
foundation.

Actions:

- Added persisted `RemoteViewAcquisitionLease` state to `ServiceState`.
- Wrapped `remote_view_open` in an acquisition lease that marks selected
  route-pool entry, display allocation, and remote-view route records as
  pending before display access, browser launch, tab acquisition, proof, and
  checkout complete.
- Added rollback for display-access, launch, tab-open, focus, proof, and
  checkout failures.
- Changed failure cleanup summaries to typed JSON with cleanup and lease
  rollback evidence.
- Added a no-launch fixture proving a failed pending acquisition restores the
  available route-pool entry and removes pending display/route rows.
- Updated Plan 0044 and `ROADMAP.md` to record Slice C progress and keep the
  forced-proof live smoke as remaining Slice C work.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:route-handoff-audit`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:route-confusion-gates`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-2342690` and stream `38445`.
- Slice C is not complete yet. The focused forced-proof failing live smoke and
  remaining service-contract metadata coverage still need to run before moving
  to Slice D.

## Turn 48 | 2026-06-22

Scope: close the P44 Slice C no-launch service-contract metadata gap.

Actions:

- Added a service-state wire-contract assertion for
  `remoteViewAcquisitionLeases`.
- Extended the nested service-state round-trip fixture with an acquisition
  lease and previous route-pool, display-allocation, and remote-view-route
  snapshots.
- Strengthened route checkout assertions for acquisition-plan metadata,
  checked-out route-pool entry state, and route provider event metadata.
- Strengthened route release assertions for release status, released
  viewer-lease metadata shape, and route release provider event metadata.
- Updated Plan 0044 and `ROADMAP.md` so the remaining Slice C gap is the
  focused forced-proof failing live smoke.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml service_state_round_trips_nested_entities -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_route_and_lease_actions_mutate_service_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:route-confusion-gates`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-2420462` and stream `37215`.
- Slice C still needs the focused forced-proof failing live smoke before it can
  be marked complete.

## Turn 49 | 2026-06-23

Scope: close the P44 Slice C forced-proof live-smoke gap.

Actions:

- Added a forced visible-window-proof failure hook behind
  `AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE`.
- Added `--force-proof-failure` support to
  `scripts/smoke-remote-view-open-live.js`, including assertions for typed
  cleanup JSON, route-pool rollback, display/route rollback, and failed
  acquisition-lease state.
- Fixed post-launch failure ordering so rollback happens before browser/tab
  cleanup can prune pending lease state, then records actual cleanup back onto
  the failed lease when the lease is still retained.
- Restored missing acquisition-lease snapshots before rollback and completion
  when service mutations overwrite pending lease state.
- Allowed released or orphaned display allocations from previous sessions to be
  reclaimed by a new acquisition.
- Allowed same-owner pending route reservations to be reused during checkout and
  repeat open.
- Updated Plan 0044 and `ROADMAP.md` to mark Slice C complete and point the next
  P44 slice at browser-only route desktop work.

Validation run:

- `node --check scripts/smoke-remote-view-open-live.js`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_acquisition_lease_rollback -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reclaims_released_display_allocation_from_previous_session -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reuses_same_owner_pending_route_reservation -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `AGENT_BROWSER_COMMAND=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live -- --force-proof-failure`
- `AGENT_BROWSER_COMMAND=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live`

Result:

- All listed checks passed.
- The forced-proof live smoke passed with route `guacamole:4`, display
  allocation `remote-view-display:16`, cleanup state `closed_new_browser`,
  rollback state `rolled_back`, and artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T03-16-04-025Z`.
- The normal fixture smoke passed afterward with repeat open, HTTP helper, CDP
  readback, X11 PID proof, route-handoff classification `route_bound_ready`,
  visual state `browser_window_visible`, OCR proof, and artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T03-17-24-564Z`.
- P44 Slice C is complete. P44 remains open for Slice D and later closeout
  criteria.

## Turn 50 | 2026-06-23

Scope: start P44 Slice D browser-only route desktop work.

Actions:

- Removed foreground terminal startup from new route-pool XRDP user sessions.
- Updated the installed-helper source and the route-pool setup fallback so
  generated `.xsession` files start Openbox when available, keep XRDP alive with
  an idle sleep loop, and do not launch terminal UI.
- Added `scripts/test-rdp-route-xsession.js` to guard maintained route
  `.xsession` writers against terminal startup.
- Wired the xsession guard into `pnpm test:route-confusion-gates`.
- Added `terminal_topmost` route display classification and
  `terminal_topmost_route` proof failure coverage so visible browser proof
  cannot pass when a terminal is the top application window over the browser.
- Updated README, docs install page, Plan 0044, and `ROADMAP.md`.

Validation run:

- `bash -n scripts/libexec/agent-browser-privileged-helper scripts/setup-rdp-guac-route-pool.sh`
- `pnpm test:rdp-route-xsession`
- `cargo test --manifest-path cli/Cargo.toml display_content_rejects_terminal_topmost_over_browser -- --nocapture`
- `pnpm test:route-confusion-gates`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-3021946` and stream `37741`.
- Slice D is not complete yet. The installed privileged helper still needs to be
  refreshed on the host, then a cold route session and route display inspection
  should prove the desktop is browser-control-ready instead of terminal-first.
- Installed helper readback showed
  `/usr/local/libexec/agent-browser/agent-browser-privileged-helper` still
  writes the old `xterm` `.xsession`; `sudo -n true` failed with password
  required, so this session could not refresh the helper or run the cold-route
  proof.

## Turn 58 | 2026-06-27

Scope: close P53 and unlock P46 S4.

Actions:

- Added `agent-browser window new [url] --same-profile` and wired S4 to create
  the second top-level window inside the same retained browser process instead
  of launching a second same-profile Chrome process.
- Rebuilt and converged the local runtime with
  `pnpm converge:local-runtime -- --apply --json`.
- Updated the S4 harness to accept explicit-command runs with no pre-existing
  daemon listener, while still rejecting duplicated or mismatched daemon
  authority.
- Updated S4 evaluation to require one retained same-profile browser row for
  the P53 topology.
- Marked P53 complete and moved P46 to S5.

Validation run:

- `node scripts/test-p47-scenario-harness.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `cargo test --manifest-path cli/Cargo.toml test_window_new_same_profile_with_url`
- `node scripts/run-p46-stress-scenario.js --scenario s4 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S4 passed with artifact
  `/tmp/agent-browser-p46-s4-2026-06-27T19-12-55-449Z`.
- The pass proved one retained browser process
  `session:p46-s4-window-2026-06-27T19-12-53-709Z`, one runtime profile
  `p46-s4-profile`, one route `guacamole:3`, one display `:13`, two
  same-profile top-level windows, working refresh controls for both dashboard
  operators, and window B staying ready after closing window A.
- Reset-before and reset-after both ended with zero active incidents.

## Turn 66 | 2026-06-27

Scope: close P63 and unlock P46 S11 after the S10 foreign CDP inventory lock.

Actions:

- Added dashboard `/api/session-tabs?port=<foreign-cdp-port>` fallback from
  agent-browser `/api/tabs` to raw Chrome CDP `/json/list`.
- Changed the local dashboard proxy to stop reading once declared
  `Content-Length` is satisfied, with bounded per-read timeout and response
  size cap.
- Updated S10 to read dashboard inventory through the authenticated
  viewer-client session.
- Made foreign CDP browser cleanup best-effort so profile removal cannot mask
  the scenario failure.
- Updated selected workspace probing to accept viewport-route context when the
  optional detail panel is not mounted.
- Scoped S10 foreign route-borrow detection to selected-workspace evidence
  instead of global workspace-list text.
- Marked P63 complete and advanced P46 to S11.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml dashboard -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-dashboard-workspace-nodes.js`
- `git diff --check -- cli/src/native/stream/dashboard.rs scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md`
- `pnpm publish:local-dashboard -- --skip-smoke --json`
- `/home/ecochran76/.local/bin/agent-browser --json install doctor`
- `node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`
- `node scripts/run-p46-stress-scenario.js --scenario s10 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S10 passed with artifact
  `/tmp/agent-browser-p46-s10-2026-06-27T22-52-43-936Z`.
- The pass proved authenticated foreign CDP inventory, normalized foreign tab
  inventory, no service route/display borrowing, stable foreign and
  service-owned selected workspace context, service-owned control readiness,
  and complete route-bound finalization.
- Reset-before and reset-after ended with zero active incidents.
- Installed executable SHA:
  `502f05830dfb756cda44eae7d6bb8c71999dd4ce39ee109eb51ff36136de155a`.

## Turn 67 | 2026-06-27

Scope: implement P46 S11, clear P64, and advance P46 to S12.

Actions:

- Added S11 scenario metadata for one route-bound service-owned browser and one
  zero-lease dashboard viewer-client.
- Added S11 capture for dashboard reload, stale workspace URL navigation,
  viewer-client reconnect, viewport refresh, direct Guacamole frame URL
  readback, route display inspection, service status, incidents, and
  route-bound finalization evidence.
- Added S11 evaluator checks for stale target recovery, reconnect proof,
  refresh control function, direct Guacamole reachability, route display state,
  stream binding, finalization, and incident cleanliness.
- Ran two live S11 attempts from the installed binary authority. Both reset
  cleanly with zero active incidents but failed in the harness before S11
  evaluation.
- Added P64 to repair the stale URL live-target recovery acceptance boundary.
- Added `allowRecoveredLiveTab` for S11 so immediate rewrite from a stale target
  to a current live target can satisfy the stale URL recovery criterion without
  weakening default tab matching.
- Ran the P64-authorized S11 retry and marked P64 complete.
- Advanced P46 to S12.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node --check scripts/lib/p47-viewer-client.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js scripts/test-p47-viewer-client-separation.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`
- `node scripts/run-p46-stress-scenario.js --scenario s11 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command-match --require-agent-browser-daemon-command-match`

Result:

- First failed S11 artifact:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-02-14-303Z`.
- Second failed S11 artifact:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-05-10-207Z`.
- Both failures proved the dashboard rejected the stale target and recovered to
  a live target, but the harness expected either exact stale URL persistence or
  explicit stale-recovery notice text.
- S11 passed with artifact
  `/tmp/agent-browser-p46-s11-2026-06-27T23-09-57-372Z`.
- The pass proved dashboard reload restoration, stale URL recovery to a live
  target, viewer-client reconnect, viewport refresh, direct Guacamole HTTP 200,
  route display `browser_window_visible`, route-bound finalization, and zero
  active incidents after reset-after.
- Command metadata caveat: the pass used an explicit installed binary command
  and daemon realpath matching passed, but the explicit-command guard flag was
  misspelled, so the artifact reports `requireExplicit: false` while also
  reporting `explicit: true`.
- P46 is now in progress at S12.

## Turn 68 | 2026-06-27

Scope: implement P46 S12 soak harness and stop at the S12 lock.

Actions:

- Added S12 scenario metadata for repeated normal-use drift and reset soak.
- Added S12 runner support for ten cycles of route-bound open, dashboard
  reload, viewer-client reconnect, viewport refresh, navigate, tab creation,
  tab switch, direct Guacamole readback, route-bound finalization, close,
  reset, and cycle-boundary doctor and incident probes.
- Added active-pressure and route-pool-baseline evaluation for each cycle.
- Corrected the pressure classifier after the first S12 run showed completed
  acquisition-lease history was being counted as active pressure.
- Ran a second S12 attempt and stopped after it exposed real route-pool reset
  drift.
- Repaired the failed live state through authenticated `service_route_pool_repair`
  dry-run and apply, followed by service reconcile and incident resolution.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node --check scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`

Result:

- First S12 artifact:
  `/tmp/agent-browser-p46-s12-2026-06-27T23-20-14-868Z`.
- The first run completed ten cycles with zero active incidents, zero retained
  sessions/browsers/tabs, route-pool baseline true, and direct Guacamole HTTP
  200 in every cycle, but failed due to the harness counting completed
  acquisition-lease history as active pressure.
- Second S12 artifact:
  `/tmp/agent-browser-p46-s12-2026-06-27T23-39-14-415Z`.
- The second run exposed real drift: cycle 3 left `guacamole:3` orphaned on
  `remote-view-display:13`, with `guacamole-rdp-a` still checked out; cycle 4
  then failed with `route_pool_entry_unavailable`.
- `service_route_pool_repair` dry-run found one stale checkout, one stale
  route, and one stale display allocation; apply repaired all three.
- Post-repair reconcile showed both route-pool entries available,
  `guacamole:3` released, and `remote-view-display:13` released.
- Final incident summary has no active incidents; the transient cycle-browser
  incident is recovered with an explicit resolution note.
- Historical state, superseded by the later repair and S12 pass: P46 was
  locked at S12 until a follow-up plan addressed orphaned route-bound display
  cleanup after normal close.

## Turn 69 | 2026-06-27

Scope: repair P46 S12 route cleanup and classify the remaining selector
failure.

Actions:

- Updated service-health reconciliation to preserve newer remote-view release
  mutations for display allocations, routes, and route-pool entries.
- Updated normal close cleanup to release session-owned display allocations and
  routes when process-exit reconcile removed the browser row before close
  persistence.
- Added regression coverage for the absent-browser close race.
- Rebuilt and converged the installed runtime after closing stale daemon
  listeners.
- Ran S12 against installed SHA
  `43d85bebf6c2e68fb7b86a5e9a1628f6e20698d7140533bb033bf932dd26c113`.
- Classified the remaining S12 failure as a harness selector defect and patched
  S12 to prefer the current cycle's `tab new` result by service tab ID or exact
  returned index and URL.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml close_releases_session_owned_route_after_process_exit_removed_browser -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml native::service_health::tests:: -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `pnpm converge:local-runtime -- --apply --json`
- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`

Result:

- Third S12 artifact:
  `/tmp/agent-browser-p46-s12-2026-06-28T00-43-57-985Z`.
- The run completed ten cycles with zero active incidents at all boundaries,
  route-pool baseline true after every reset, no post-reset pressure increase,
  and direct Guacamole HTTP 200 in every cycle.
- The only failures were switched-tab URL assertions caused by stale positional
  tab selection under repeated-cycle tab accumulation.
- S12 is unlocked for one selector-repaired retry.

## Turn 70 | 2026-06-27

Scope: clear P46 S12 after selector repair.

Actions:

- Reran S12 with the selector-repaired harness against
  `/home/ecochran76/.local/bin/agent-browser`.
- Captured final install doctor, remote-view doctor, and incident-summary
  evidence.
- Updated the P46 plan and execution note to mark S12 cleared.

Validation run:

- `node scripts/run-p46-stress-scenario.js --scenario s12 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`
- `/home/ecochran76/.local/bin/agent-browser --json install doctor`
- `/home/ecochran76/.local/bin/agent-browser --json doctor remote-view`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`

Result:

- S12 pass artifact:
  `/tmp/agent-browser-p46-s12-2026-06-28T01-05-24-861Z`.
- The pass reports `requireExplicit: true`, `explicit: true`, and daemon
  realpath matching passed.
- All ten cycles completed with route-pool baseline true after every reset.
- Active incidents stayed zero at every boundary and reset point.
- Post-reset pressure did not increase; checked-out route-pool, active
  remote-view routes, sessions, browsers, and tabs were zero after every reset.
- Direct Guacamole returned HTTP 200 in every cycle.
- Final install doctor succeeded with no issues and installed SHA
  `43d85bebf6c2e68fb7b86a5e9a1628f6e20698d7140533bb033bf932dd26c113`.
- Final remote-view doctor status was `ready`.
- Final incident summary count was 0.
- P46 S12 is cleared.

## Turn 71 | 2026-06-27

Scope: close P46 after auditing completion criteria.

Actions:

- Re-read the P46 plan closeout criteria and audited current evidence against
  each required proof point.
- Updated the P46 plan state to `COMPLETE`.
- Added the missing S9 through S12 entries to the current execution ledger.
- Added the campaign summary, residual risks, and next hardening target to the
  P46 plan and execution note.
- Captured fresh final service status after the S12 pass.

Validation run:

- `~/.local/bin/graphiti-runtime doctor`
- `~/.local/bin/graphiti-runtime discover --group-id agent_browser_main --max-facts 8 --max-nodes 5 --max-episodes 5 "agent-browser P46 plan 0046 S12 route cleanup selector repair final closeout residual risk next hardening target"`
- `/home/ecochran76/.local/bin/agent-browser --json service status`
- `/home/ecochran76/.local/bin/agent-browser --json install doctor`
- `/home/ecochran76/.local/bin/agent-browser --json doctor remote-view`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`
- `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`

Result:

- P46 is complete through S12.
- Final service status artifact:
  `/tmp/agent-browser-p46-final-service-status.json`.
- Final install doctor artifact:
  `/tmp/agent-browser-p46-final-install-doctor.json`.
- Final remote-view doctor artifact:
  `/tmp/agent-browser-p46-final-remote-view-doctor.json`.
- Final incident summary artifact:
  `/tmp/agent-browser-p46-final-incidents-summary.json`.
- Final route-pool readiness artifact:
  `/tmp/agent-browser-p46-final-route-pool-readiness.json`.
- Final service status reported zero service browsers, zero service sessions,
  zero tabs, and zero active incidents.
- Final install doctor succeeded with no issues and runtime status `converged`.
- Final remote-view doctor status was `ready`.
- Final incident summary count was 0.
- Final route-pool readiness succeeded.
- Route-pool entries `guacamole-rdp-a` and `guacamole-rdp-b` were available
  with no current route allocation.
- Residual risk: historical orphaned display-allocation records remain visible
  in service status, but they are not live control rows and do not hold
  route-pool capacity.
- Next hardening target: retained-state compaction and doctor-surface cleanup
  for historical orphaned display allocations and stale metadata visibility.

## Turn 72 | 2026-06-28

Scope: plan the retained display-state compaction follow-up after P46.

Actions:

- Created `docs/dev/plans/0065-2026-06-28-retained-display-state-compaction-plan.md`.
- Scoped P65 to classify, explain, and safely compact retained historical
  display-allocation metadata without weakening P46 route-pool or live-control
  guarantees.
- Reused prior retained-state cleanup conventions: dry-run before apply,
  service-owned actions only, no manual service-state edits, and doctor/readback
  proof after live cleanup.

Result:

- P65 is `PLANNED`.
- First implementation slice should add the retained display-state classifier
  and focused tests before adding apply behavior.

## Turn 73 | 2026-06-28

Scope: execute P65 retained display-state compaction.

Actions:

- Added retained display-allocation classification to service state model.
- Extended `service prune-retained` with `--display-allocations` dry-run/apply.
- Added `retainedDisplayAllocations` to service status JSON and text output.
- Updated service request/status contracts, generated client types, README,
  docs site pages, and `skills/agent-browser/SKILL.md`.
- Rebuilt and installed the local debug binary, ran
  `pnpm publish:local-dashboard -- --skip-browser --json`, and removed two
  stale deleted-executable default daemon listeners reported by install doctor.

Validation:

- `cargo test --manifest-path cli/Cargo.toml service_prune_retained -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_prune_retained_service_state_classifies_display_allocations -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_service_status_via_actions_does_not_launch_browser -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_format_service_status_text_includes_profile_and_session_summaries -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_status_and_collection_response_contracts_match_wire_shape -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`

Live proof:

- Artifact directory:
  `/tmp/agent-browser-p65-retained-display-20260628T174225Z`.
- `display-prune-dry-run.json` reported zero apply-safe display allocation
  candidates; apply was skipped.
- Final status retained 22 display allocations: 16 `diagnostic-retained`, 6
  `live`, 0 apply-safe.
- `final2-incidents-summary.json` reported incident count 0.
- `final2-install-doctor.json` succeeded with no issues.
- `final2-remote-view-doctor.json` succeeded with status `ready`.
- `final2-route-pool-readiness.json` succeeded with status `ready`.

Result:

- P65 is complete.
- No retained display allocation compaction is needed until a future dry-run
  reports apply-safe candidates.

## Turn 84 | 2026-07-06

Scope: continue P69 Slice C by sharing one acquisition-result builder.

Actions:

- Added `remote_view_handoff::shared_profile_acquisition_result` as the common
  JSON constructor for shared-profile acquisition evidence.
- Rewired route-bound `remote_view_open` shared acquisition records and
  `tab_new_shared_acquisition_evidence` in `cli/src/native/actions.rs` through
  that one builder.
- Extended the existing tab evidence tests to assert common acquisition-result
  fields such as `duplicateProcessPolicy`, `plannedProfile`, and
  `routeHintFields`.
- Updated P69 to narrow the remaining shared acquisition-result gap to plain
  remote-headed `open`.

Validation:

- `node --check packages/client/src/service-request.js`
- `node --check scripts/test-service-request-client.js`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml tab_new_shared_acquisition -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- HTTP/MCP-routed `tab_new` responses and route-bound `remote_view_open`
  responses now share the same acquisition-result constructor. P69 remains open
  for plain remote-headed `open` convergence and Slice F live proof.

## Turn 83 | 2026-07-06

Scope: continue P69 Slice C shared acquisition-result convergence.

Actions:

- Added route-bound `sharedAcquisition` records to `remote_view_open` planned
  and opened responses through
  `remote_view_handoff::route_bound_handoff_shared_acquisition`.
- Kept `routeBoundHandoff` as the detailed route/display/operator-visible proof
  surface while exposing the same top-level acquisition-result name already
  used by access-plan and service-request tab responses.
- Extended `summarizeServiceSharedProfileAcquisition()` in
  `packages/client/src/service-request.js` to summarize route-bound
  `remote_view_open` responses from `data.intent`, `data.sharedAcquisition`,
  nested `data.tab`, and nested `serviceTabHandle`.
- Added focused Rust and service-client coverage for route-bound shared
  acquisition summaries.

Validation:

- `node --check packages/client/src/service-request.js`
- `node --check scripts/test-service-request-client.js`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/remote_view_handoff.rs packages/client/src/service-request.js scripts/test-service-request-client.js docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- P69 Slice C now has a common named acquisition-result record on
  `remote_view_open` responses and on the existing access-plan/tab response
  path. Remaining Slice C work is to route plain remote-headed `open`, HTTP
  `service_request`, and MCP `service_request` through that same acquisition
  result as a real shared planning artifact, not just a response field.
- P69 Slice F live proof remains open.

## Turn 82 | 2026-07-06

Scope: continue P69 Slice C retained-browser failure cleanup deepening.

Actions:

- Added handoff-owned cleanup decision helpers in
  `cli/src/native/remote_view_handoff.rs` for route-bound failure recovery:
  close only the opened tab for a reused retained browser, close a newly
  launched browser, or skip cleanup when no opened-tab index is available.
- Rewired `remote_view_open_cleanup_after_failure` in
  `cli/src/native/actions.rs` so the dispatcher executes the selected async
  browser command while the handoff module owns cleanup decision and result
  vocabulary.
- Updated P69 with the current Slice C progress and remaining work.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns failure-cleanup recovery decisions for retained versus newly launched
  browsers, but the full shared acquisition-result routing across
  `remote-view open`, plain remote-headed `open`, HTTP `service_request`, and
  MCP `service_request` remains open.
- P69 Slice F live proof remains open.

## Turn 81 | 2026-07-06

Scope: continue P69 Slice C begin-acquisition lease reservation deepening.

Actions:

- Moved route-bound begin-acquisition lease reservation into
  `cli/src/native/remote_view_handoff.rs` behind
  `begin_route_bound_handoff_acquisition`.
- Rewired the `remote_view_open` begin-acquisition adapter in
  `cli/src/native/actions.rs` to supply the observation timestamp and
  provider-derived default control-input adapter, while the handoff module now
  owns pending route-pool, display-allocation, route, and lease repository
  mutations.
- Updated P69 with the current Slice C progress and remaining work.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_acquisition_lease_rollback_restores_route_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns begin-acquisition reservation, planned/opened response assembly,
  cleanup/rollback summary reporting, acquisition completion, lease
  restoration, rollback mutation, and cleanup update mutation. Retained-browser
  recovery sequencing and shared acquisition-result routing remain open.
- P69 Slice F live proof remains open.

## Turn 80 | 2026-07-06

Scope: continue P69 Slice C acquisition lease lifecycle deepening.

Actions:

- Moved route-bound acquisition completion, lease restoration, rollback
  mutation, and post-cleanup rollback update into
  `cli/src/native/remote_view_handoff.rs`.
- Rewired `remote_view_open` helper adapters in `cli/src/native/actions.rs` to
  delegate those service-state mutations to the handoff module while retaining
  timestamp generation and browser/repository orchestration in the dispatcher.
- Kept begin-acquisition lease reservation in `actions.rs` for now because it
  still constructs a pending `RemoteViewRoute` with the local
  `default_control_input_provider` helper. That is the next obvious Slice C
  sequencing move.
- Updated P69 with the current Slice C progress and remaining work.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_acquisition_lease_rollback_restores_route_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns planned/opened response assembly, cleanup/rollback summary reporting,
  acquisition completion, lease restoration, rollback mutation, and cleanup
  update mutation. Begin-acquisition reservation and retained-browser recovery
  sequencing still remain to move behind the handoff module interface.
- P69 Slice F live proof remains open.

## Turn 79 | 2026-07-06

Scope: continue P69 Slice C handoff cleanup reporting.

Actions:

- Added `route_bound_handoff_cleanup_summary()` to
  `cli/src/native/remote_view_handoff.rs`.
- Rewired `remote_view_open` failure paths in `cli/src/native/actions.rs` to
  use the handoff module's cleanup summary for rollback and cleanup reporting.
- Removed the duplicate local cleanup-summary formatter from `actions.rs`.
- Added handoff-module coverage for cleanup summary shape.
- Updated P69 with the new Slice C progress. Repository rollback mutation still
  remains in `actions.rs` for the next deeper sequencing pass.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_acquisition_lease_rollback_restores_route_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns planned/opened response assembly and cleanup/rollback summary reporting.
  Full acquisition, finalization, rollback mutation, and retained-browser
  recovery sequencing still remain to move behind the handoff module interface.
- P69 Slice F live proof remains open.

## Turn 78 | 2026-07-06

Scope: continue P69 Slice C route-bound handoff deepening.

Actions:

- Added `planned_route_bound_handoff_response()` and
  `opened_route_bound_handoff_response()` to `cli/src/native/remote_view_handoff.rs`.
- Rewired `remote_view_open` dry-run and opened success paths in
  `cli/src/native/actions.rs` to call the handoff response builders instead of
  assembling the authoritative profile, browser, session, route, display, tab,
  operator-visible proof, and verification fields in the command dispatcher.
- Preserved existing response shape for dry-run and opened handoffs while
  concentrating that shape behind the handoff module interface.
- Fixed `parse_remote_view_open` to preserve global `--browser-build` into
  the `remote_view_open` command payload. The broader `remote_view_open_`
  filter caught this as a P69 flag-preservation regression.
- Updated P69 and the routing-failure note.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns planned/opened response assembly. Full acquisition, finalization,
  rollback, and retained-browser recovery sequencing still remains to move
  behind the handoff module interface.
- P69 Slice F live proof remains open.

## Turn 77 | 2026-07-06

Scope: continue P69 Slice E shared-profile client ergonomics.

Actions:

- Added generated TypeScript declaration coverage for
  `ServiceSharedProfileAcquisitionSummary`.
- Added `summarizeServiceSharedProfileAcquisition()` to
  `@agent-browser/client/service-request`.
- The helper accepts either an access-plan response or a tab response and
  returns compact requested profile, planned profile, runtime profile, profile
  id, retained browser/session route hints, tab/target ids, service tab handle,
  acquisition mode, route-hint requirement, and duplicate-process policy.
- Added service-request client tests for summaries from both access-plan
  `decision.profileReuse.sharedAcquisition` and tab response
  `data.sharedAcquisition`.
- Updated P69, the routing-failure note, README, docs site service-mode
  guidance, and the agent-browser skill so software clients use the helper
  instead of parsing raw profile reuse state.

Validation:

- `node scripts/generate-service-request-client.js`
- `pnpm test:service-request-client`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm test:service-client`

Result:

- P69 Slice E shared-profile client ergonomics is implemented. P69 remains
  open for Slice C's full handoff-module sequencing and Slice F live proof.

## Turn 76 | 2026-07-06

Scope: continue P69 Slice D workspace inventory actionability.

Actions:

- Added `WorkspaceProfileActionability` to the dashboard workspace inventory
  projection.
- Marked compatible live service-owned retained browser rows with
  `openSharedProfileTab` and enabled their `add-tab` action as the recommended
  shared-profile operation.
- Marked profile-only lock rows with `waitForProfileHolder` or
  `rejectDuplicateProcess` so the dashboard distinguishes agent-browser-owned
  retained profile sharing from unknown or incompatible profile holders.
- Surfaced profile actionability in workspace navigator search and selected
  row detail.
- Wired the service-owned browser row `add-tab` action to HTTP
  `service_request` `tab_new` using the retained owner route hints, then
  refreshed service status and selected the returned browser/tab identity.
- Added viewer-controller lease and route-switch actionability to the same
  workspace inventory interface. Those rows now recommend `takeOverViewer` or
  `routeSwitch`, carry the lease or attachability reason, and keep `add-tab`
  disabled when opening another shared-profile tab is not the correct
  operation.
- Updated P69, the routing-failure note, README, dashboard docs, and the
  agent-browser skill.

Validation:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `git diff --check -- packages/dashboard/src/lib/service-workspaces.ts packages/dashboard/src/components/workspace-navigator.tsx scripts/test-dashboard-workspace-nodes.js scripts/test-dashboard-workspace-navigator.js docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md docs/dev/notes/2026-07-06-last30days-profile-routing-failure.md RUNBOOK.md README.md docs/src/app/dashboard/page.mdx skills/agent-browser/SKILL.md`

Result:

- P69 Slice D no-launch workspace inventory actionability is implemented. The
  inventory projection now carries the retained-owner versus duplicate-process
  distinction, viewer takeover, and route-switch recommendations, and the
  executable service-owned browser `add-tab` dashboard flow uses the
  service-request tab creation path.
- P69 remains open for Slice C's full handoff-module sequencing and Slice E/F
  contract, client, and live proof work.

## Turn 75 | 2026-07-06

Scope: continue P69 Slice C route-bound handoff deepening.

Actions:

- Added `cli/src/native/remote_view_handoff.rs` as the named module for the
  route-bound handoff proof record.
- Wired `remote_view_open` dry-run and success responses to publish
  `routeBoundHandoff` with one authoritative profile, browser, session, tab,
  route, display, and operator-visible proof surface.
- Reworked operator-visible proof failure diagnostics to include a focused
  `routeBoundHandoff` failure record for the failing route binding. Final
  post-checkout proof failures keep pre-checkout evidence separately labeled
  instead of blending it into the final proof record.
- Updated P69 and the profile-routing failure note to record Slice C progress
  and keep the remaining full handoff-module sequencing work open.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_request -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is partially implemented. The response proof vocabulary is now a
  named module, while the larger plan/acquire/finalize/rollback sequencing
  still needs to move behind the handoff module interface.

## Turn 74 | 2026-07-06

Scope: continue P69 shared-profile routing and service-request parity.

Actions:

- Added an access-plan-owned helper that applies shared-profile route hints to
  `tab_new` service requests when a compatible retained same-profile browser
  already owns the requested runtime profile.
- Wired HTTP `POST /api/service/request` through that helper with the live
  service state and taught non-focus relay to honor synthesized top-level
  command hints while continuing to ignore `params.sessionName`.
- Wired MCP `service_request` through the same persisted-plus-configured service
  state used by `agent-browser://access-plan` and routed hinted requests to the
  owner daemon session.
- Extended MCP `service_request` command/schema handling for
  `runtimeProfile`, `profileId`, `profile`, `browserHost`,
  `viewStreamProvider`, `controlInputProvider`, and `displayIsolation`.
- Repaired service-request contract drift for access-plan planned tab requests
  by adding `profileClass` to the JSON schema, generated client, HTTP adapter,
  and MCP adapter.

Validation:

- `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser pnpm test:service-request-live`
- `pnpm test:service-client-contract`
- `pnpm test:service-request-client`
- `cargo test --manifest-path cli/Cargo.toml service_request -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice B now covers plain navigation plus HTTP/MCP `service_request`
  `tab_new` route-hint parity in no-launch tests and live service-request
  smoke proof.
- The live smoke opened two same-profile service tabs through one retained
  browser, released one physical target, preserved the browser/session route,
  and successfully evaluated the surviving tab handle.
- Remaining P69 work starts at Slice C route-bound handoff deepening.
