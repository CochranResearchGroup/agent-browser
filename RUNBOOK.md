# Runbook

Current index. [The September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md) preserves checkpoints through Turn 312; [Turn 313](RUNBOOK-history-2026-09-13-turn313.md), [Turns 314 through 320](RUNBOOK-history-2026-09-14-turn314-through-turn320.md), [P169 history through Turn 319](RUNBOOK-history-2026-09-14-p169-through-turn319.md), [superseded P186 checkpoints through Turn 328](RUNBOOK-history-2026-09-14-p186-through-turn328.md), [P194 through P196 checkpoints from Turns 340 through 343](RUNBOOK-history-2026-09-15-p194-through-turn343.md), and [Turns 323 through 368](RUNBOOK-history-2026-09-14-turn323-through-2026-09-16-turn368.md) remain separately preserved.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)
- [P220](docs/dev/plans/0220-2026-09-28-remote-view-consumer-integration.md)

## Turn 414 | 2026-10-01

Configured Remote View now selects the managed-session host before an ordinary
browser request has any session. The owner admits its first request with an
exact catalog profile or the default disposable policy. Repeated requests and
restart preserve browser and current-tab identity. Commands retain their original
navigation headers, wait policy and request ID; responses add `browserSession`
identity beside the native action result. Successful ordinary navigation records
exact URL history through host publication. Failed navigation adds no success
row. Invalid, unknown and conflicting selectors stop before allocation.

Focused provider-free qualification passed 43 CLI session tests, with one
ignored real-Chrome test. After test-only additions for recorded-directory
selection and malformed input, all 12 consumer composition tests passed again.
Production code remained unchanged, so strict workspace Clippy and the earlier
session selection remain applicable. Final formatting, architecture, guidance,
documentation links and the docs production build passed. All five required
source documentation surfaces describe the admission behavior. Installed skill
publication and installed browser/provider acceptance were not performed.

P220 v26 remains open. Stable tab-specific handoffs remain in milestone one;
explicit tab lifecycle routing, foreground ownership, recovery, relocation and
final cleanup retain milestone-two scope. Installed Alice/Bob proof and candidate
qualification remain ahead. This is source qualification, not merge readiness.
The fresh active goal meter read 391,649, below the requested one-million
checkpoint ceiling. Memory disposition: `not_durable`, with routine source
behavior retained in the owning contract and runbook and a machine-readable
receipt at `/tmp/p220-open-memory-disposition-2026-10-01.json`.

## Turn 413 | 2026-10-01

P220 now prepares configured pool capacity only when a new managed browser
needs placement. Runtime construction, status, healthy reuse and restart reuse
allocate nothing. Desired desktop count defaults to one and is bounded by the
Remote View pool policy. SQLite retains a shared acquisition request head and
exact mutation history before provider effects. Unknown acquisition blocks
another capacity operation across restart, even when provider inventory shows
an assignment. Completed request continuity advances with exact active evidence
or the existing ledger's confirmed assignment retirement; disappearance alone
cannot retire a request. Fresh window observations omit unavailable peers and
prefer window-free desktops when browser counts tie.

Focused provider-free qualification passed 41 CLI session tests with one ignored
real-Chrome test, plus all 26 tests in the model's `browser_session_manager`
integration target. The initial model symbol filter selected zero tests and was
corrected to the explicit integration target. Six new SQLite composition cases
cover demand, restart, unknown result, policy bounds, shared request identity,
retirement continuity and healthy peer selection. Strict workspace Clippy,
format, architecture, documentation links, remote-view guidance and the docs
production build passed. Required guidance was updated in all five source
documentation surfaces. No installed provider or browser was contacted.

P220 v26 remains open. Next is the complete ordinary open and durable tab-handoff
journey; everyday focus, recovery, relocation and final cleanup, installed
Alice/Bob acceptance and release-candidate qualification retain their original
scope. This source increment is not installed acceptance or merge readiness.
The fresh active goal meter read 310,061, below the requested one-million
checkpoint ceiling. Memory disposition: `not_durable`, because the owning
contract and runbook retain this routine increment; machine-readable local
receipt: `/tmp/p220-pool-memory-disposition-2026-10-01.json`.

## Turn 412 | 2026-10-01

P220 milestone one now connects native loopback HTTP to ordinary runtime
construction through explicit origin, pool and optional application settings.
Public inventory supplies active assignments and fresh live observations qualify
exact desktop UUID and lifecycle generation. Local construction remains the
unconfigured path. Managed commands use the composed owner after the host
supplies its published baseline again following current-tab acquisition.

Focused validation: `browser_session` passed 35 tests with one ignored real-Chrome
restart test; HTTP transport passed three disposable loopback tests, including
lost-response handling without another POST. Strict workspace Clippy passed.
Formatting, P220 architecture, documentation links, remote-view guidance checks
and the docs production build passed. This is source qualification, not installed
acceptance or merge readiness. No installed browser or Remote View service was
contacted. Repository guidance changed across all five required documentation
surfaces; installed skill publication was not performed.

Empty-pool durable acquisition, capacity changes, stable tab handoffs and the
remaining everyday lifecycle work remain open under P220 v26. The fresh goal
meter was 138,520 at this continuation, well below the one-million checkpoint
ceiling. Memory disposition: `not_durable`; this routine increment is retained
in the owning contract and runbook, with a local machine-readable receipt at
`/tmp/p220-runtime-memory-disposition-2026-10-01.json`.

## Turn 411 | 2026-10-01

P220 milestone one now includes a native loopback HTTP transport for Remote
View's direct JSON application endpoint. Disposable server tests prove the
actual envelope, operation response, invocation inside an existing Tokio
runtime, redirect refusal and bounded malformed-response handling. Both
focused tests passed. Runtime construction, managed dispatch and stable tab
handoffs remain open; installed-provider acceptance has not run.

Implementation compilation caught an empty struct-variant fixture error,
which was corrected before the passing focused run. Format, architecture and
documentation-link checks passed; workspace strict Clippy found the unused
transport connection, which Turn 412 resolves. No installed runtime or provider was contacted. Memory disposition:
`not_durable`, recorded by the local machine-readable receipt at
`/tmp/p220-http-memory-disposition-2026-10-01.json`; the owning contract retains
this routine increment. P220 v26 remains active under the fresh one-million
checkpoint ceiling.

## Turn 410 | 2026-10-01

The operator explicitly resumed P220 v26 with a new goal meter starting at zero
and requested a checkpoint before it reaches one million. Startup verified the
clean published amendment checkpoint `682872b0`. Milestone one now composes the
ordinary host and Session Manager with public transport, durable launch
admission, private process inputs and atomic session publication. The consumer
effect adapter retains exact intent and exposes it only after observation;
unresolved records block restart effects. Process/runtime delegation stays with
Agent Browser, and selected descriptive labels do not become placement identity.

Three provider-free real-SQLite composition tests pass: open/reuse/co-location
and restart reuse; unknown process or competing publication without retry;
and stale admission baseline before provider read or process effect. All 26
tests in the focused Session Manager test binary pass. Strict workspace Clippy,
format, self-testing architecture guard, documentation links and diff checks
pass. Cargo admission briefly waited for memory pressure and then completed;
no failed test or automatic effect retry occurred. No Chrome, provider, install
or production effect was executed.

Milestone one remains incomplete until default live transport/runtime bootstrap,
managed command dispatch and stable tab-specific handoffs are connected. Dead
browser replacement still needs durable old-association reconciliation before
fresh admission in milestone two. The current tests prove synthetic composition,
not installed viewing, focus or recovery. Advisory Graphiti discovery returned
three old July route facts and supplied no useful current recall. Memory closeout
is not_durable: this increment is recorded in the owning contract and runbook;
no memory write was performed.

## Turn 409 | 2026-10-01

The operator requested a planning amendment while the implementation goal remains
paused. P220 v26 makes four remaining outcomes the primary execution sequence:
normal browser requests, everyday behavior, installed Alice/Bob validation and
release-candidate qualification. Earlier delivery stages and evidence remain
historical support. Each outcome has an exit and checkpoint; unrelated
infrastructure expansion and repeated unchanged validation are excluded.
An additional million tokens is a future resumption ceiling, not a delivery
promise or new execution authority. Original acceptance, installed-effect and
formal-release boundaries remain in force. No source or runtime work occurred.

## Turn 408 | 2026-10-01

The operator explicitly resumed P220 implementation. Plan v25 continues the
provider-free revision 3 external-mode adapter packet in the existing branch.
Startup verified local and recorded remote HEAD `6274601e`, with only the four
review-owned documentation amendments. Remote View's recorded canonical ref
remains `da540f22`; its public request enum confirms snake_case operation
arguments, camelCase cleanup acknowledgement, and independent lifecycle and
viewing generations. Events use lifecycle generation alone. No runtime,
provider, installation, ingress, production, merge or release effect is authorized
by this resumption. The request-only checkpoint adds a source-bound fixture for all 12 external
operations and rejects managed lifecycle and obsolete principal fields. Two
focused contract tests, the extended self-testing architecture guard and
documentation links pass. Response validation, environment privacy, injectable
transport and ambiguous-effect reconciliation remain pending. Strict workspace
Clippy, format verification, API/MCP parity and generated client contract checks
pass. This is not adapter acceptance. No live effects occurred. Graphiti recall
was historical route evidence only; no memory write is warranted for this
intermediate request-shape checkpoint.
The next implementation increment adds exact independent assignment joins,
private full launch environment and injectable observe/environment/observe
coordination. Six focused tests pass, including generation change and unknown
outcome behavior. A compile-fail doctest proves launch values are not
serializable. Initial redaction testing failed because its oracle confused a
public generation with a secret environment value; the corrected oracle passes.
Initial strict Clippy rejected a nonminimal boolean expression; that expression
was simplified; the exact changed source passes strict workspace Clippy and
focused tests. Formatting, architecture and documentation link checks pass. Other operation response
models and durable ambiguous-effect reconciliation remain incomplete.

The read-side increment models all published response shapes and injects
inventory, windows, events and retained view resolution. Twelve focused tests
pass. Released assignment history remains valid after pool membership changes;
window IDs remain locator hints rather than browser identity. View grant
issuance and resolution remain distinct from visible/input readiness. Public
inventory/events omit mutation-key correlation, so they are not proof for an
unknown acquisition or action inferred by target/timing. Mutation custody,
exact joined release validation and authoritative reconciliation remain next.
No source-backed memory write is warranted for these intermediate fixtures.

The mutation increment covers acquire, activate, issue-view, revoke-view and
release through injectable transport with custody before send. Exact cleanup
and complete retirement sets are enforced. Browser Runtime SQLite persists
pending/completed public requests in atomic per-operation documents and gives
competing connections one admission winner. Seventeen service-model focused
tests and two real SQLite focused tests pass. Strict workspace Clippy passes.
Clock-at-response expiry and cached-acquisition drift are covered. The first
SQLite test failed in teardown on an intentionally read-only migration archive;
exact synthetic residue was inspected and removed, and corrected teardown plus
both SQLite tests pass. This is a fixture cleanup failure, not discarded
storage evidence. Actual network transport, authoritative unknown-outcome
reconciliation, complete cleanup evidence construction and runtime integration
remain pending. The narrow mutation contract carries storage semantics; no live
Remote View, browser, installation, ingress or production effect occurred.

The current goal meter starts at zero; the earlier handoff's 286,939 tokens are
historical accounting. Checkpoint before the requested 1 million token bound.

The cleanup increment replaces raw acknowledgement admission with a private,
consumed permit bound to the complete release target. Current browser affinities,
retained assignment joins, sessions and tabs are checked independently of
retained-peer count. Recovery, foreground and cleanup inventories distinguish
unknown from complete-empty evidence. Seven focused mutation and cleanup tests
pass, including no-retention live affinity, detached live tab references,
generation conflicts, dangling sessions and terminal history. Architecture,
strict workspace Clippy, format and diff checks pass. Runtime collection,
transactional release fencing, physical absence readback and installed acceptance
remain pending; this source gate grants no runtime effect authority.

The host-publication increment replaces unconditional ordinary host saves with
a SQLite aggregate comparison under `BEGIN IMMEDIATE`. A stale host cannot erase
newer session or retained presentation state. The host advances its persisted
baseline only after commit. The unconditional SQLite helper is test-only.
The compile check and initial stale-state and host restart tests passed; the
strengthened two-connection test and final strict workspace Clippy passed.
Architecture, format, documentation links and diff checks pass.
The cleanup permit also has two passing compile-fail doctests preventing cloning
and deserialization. Pre-launch custody and release-claim fencing remain pending.

The ordinary host now checks its durable aggregate baseline before each Session
Manager operation. A competing writer makes the next operation return a
publication conflict before mutating local session state. The existing restart
fixture now advances a peer persistence owner, checks rejection of a fresh open,
and verifies both local and durable state remain intact. That focused host test,
strict workspace Clippy, architecture and documentation checks pass. This check
detects existing drift; durable intent is still required to close the concurrent
read-to-effect window. It is not final release admission or installed proof.

Session admission now checks expiry and new-session sequence arithmetic before
browser launch. The focused Session Manager binary passes all 26 tests, including
no launch or close for exhausted fields and healthy reuse at exhausted sequence.
This removes deterministic post-launch failures but does not provide durable
launch custody. Strict workspace Clippy, format, architecture, documentation links and diff
checks pass for this source increment.

The launch-custody source model retains exact active assignment and profile
intent, preserves an unknown process outcome, binds a nonzero PID and browser ID,
and confirms exact session-state publication. Two focused provider-free tests
pass for interrupted records, duplicate and conflicting observation, generation
drift, partial records and profile mismatch. Environment and CDP endpoint are
excluded from this record. Architecture and documentation checks pass; final
strict workspace Clippy and format checks pass. SQLite admission and runtime
wiring remain pending.

Launch custody now has an injectable persistence contract and a SQLite adapter.
Admission commits pending intent after exact current-state comparison and excludes
another unresolved intent or current browser for that profile. Exact replay
returns Existing and grants no second launch. Observation binds the same intent;
publication commits session state and custody together. Two disposable SQLite
tests passed across connections and reopen. Initial compilation failed on the
document loader's generic return shape; explicit optional session loading fixed
it. The ledger rejects stored null rather than treating it as absent and rejects
contradictory assignment generations while allowing exact co-location across
profiles. Both final SQLite tests, strict workspace Clippy, architecture, format,
documentation links and diff checks pass. Runtime effect
wiring, release-claim exclusion and physical process reconciliation remain open.

Release admission now reads durable session and launch custody in one SQLite
transaction and persists the exact assignment fence before returning a cleanup
permit. Unknown launch intent blocks release; the retained fence blocks new
launch admission, observation, publication and ordinary aggregate publication.
Three custody tests pass across reopen, exact replay, conflicting release UUID
and generation, both admission orders and fenced browser publication. Initial
compile failure exposed missing target serialization; strict public target
serialization fixed it. Seven related mutation tests, strict workspace Clippy,
format, architecture, documentation links and diff checks pass. No fence completion or deletion is implemented yet. Recovery,
foreground and cleanup owner admission plus actual runtime wiring remain pending.

Retirement completion now retains the exact immutable joined outcome alongside
its historical fence. Fresh assignment identity may reuse the same desktop and
lifecycle generation after exact completion; retired assignment and stale
generation remain excluded. Ordinary publication requires a qualified published
launch record for that fresh assignment. Atomic launch publication validates
the whole aggregate, blocking unrelated fenced desktop resurrection. Four
SQLite custody tests pass, including partial retirement rejection, exact replay,
same-generation reuse, rollback and reopen. An initial test build failed because
a broad edit changed a two-value fixture return signature; its correction passed.
Strict workspace Clippy, format, architecture, documentation links and diff
checks pass. These are source/storage guarantees,
not physical absence, owner coordination or installed consumer acceptance.

The fresh launch coordinator connects durable intent to public assignment and
private environment reads, a final independent-generation check and injectable
process effect. It records exact observation before returning and leaves session
publication separate. Two provider-free tests pass for full environment
consumption, custody-before-read, one launch, viewing drift, unknown process
outcome, wrong returned generation and failed observation commit. Existing
intent always requires readback. Architecture and documentation checks pass;
strict workspace Clippy, format and diff checks pass. At that checkpoint, no process
driver or Session Manager atomic publication path was wired, and no browser or
provider effect was run.

The process ingress increment implements `RemoteViewBrowserProcessEffects` on
BrowserManagerRuntime. It validates fresh private inputs before worker ingress,
forwards the complete environment, uses exact assignment lifecycle identity and
bypasses cached display lookup for this path. Chrome makes one attempt,
suppresses persisted stderr, and BrowserManager redacts errors before journaling.
The worker checks sequence capacity before process effects. Four ordinary
LaunchOptions constructors initially failed compilation after the additive
internal field; all now explicitly preserve their ordinary path with `None`.

Focused isolated checks pass: two private-ingress tests, 58 `remote_view_d`
matching CLI tests and one resource-pressure rejection test. The new process
test uses only a disposable synthetic shell child, verifies complete inputs and
one attempt, and checks the static diagnostic. The other new test rejects
stale viewing generation before worker ingress. Strict workspace Clippy, format,
architecture, documentation links and diff checks pass. No Chrome, provider,
install or production effect was executed. Actual fresh headed success and
CDP attach, ordinary Session Manager coordinator use and atomic publication
remain pending alongside broader P220 acceptance.
The validation selector returns its broad set because main and ordinary launch
constructors changed. Those edits only initialize the internal field to `None`;
installer, workstation, provider assets and service API/client contracts were
not changed. Their broad recommendations are not new qualification claims.

The host publication bridge now receives the durable aggregate baseline before
operation effects. Effect adapters can retain one observed launch intent, and
SQLite publishes that intent with the resulting session aggregate under the
existing custody transaction. The host advances its baseline and acknowledges
only after commit; unsupported persistence fails closed. Ordinary whole-state
publication rejects a browser matching an unpublished intent's profile or
observed browser identity, closing the generic-save bypass.

Provider-free qualification passes: two isolated host tests, four real SQLite
custody tests and all 26 tests in the focused service-model Session Manager
binary. The new host test uses real SQLite and covers wrong process identity,
competing writer, generic-save bypass, failed publication without acknowledgement
and successful atomic publication with terminal custody. Existing ordinary
restart/reuse remains covered. Strict workspace Clippy, format, architecture,
documentation links and diff checks pass. This does not qualify default external
mode: the consumer driver must still admit launches against the supplied
baseline, retain unknown outcomes and expose its intent to this publication
bridge. No installed, provider, browser or production effects were executed.

The operator-requested pre-one-million token checkpoint stops at the clean,
published source checkpoint `1483b504`. Goal readback reported 863,019 used
tokens before preparing the restart locator. P220 remains OPEN. The branch-local
lane projection now points to that source qualification checkpoint; this does
not update the canonical default-branch catalog or grant runtime authority.
Next on explicit resumption, connect consumer launch admission and unresolved
intent retention to the host baseline and atomic publication bridge. Retain all
remaining handoff, focus, recovery, cleanup and installed acceptance criteria.

## Turn 407 | 2026-10-01

P220 v24 reconciles Remote View revision 3 at `da540f22`. Issues #62/#146
are closed. Provider receipts qualify installed external lifecycle, viewing,
input, embedding and cleanup; Agent Browser adoption remains incomplete.
The next packet after explicit continuation is the external-mode wire adapter,
independent generation fences, private launch environment and versioned cleanup
acknowledgement. No source implementation or runtime effect occurred here.
The prior goal remains paused; its handoff recorded 286,939 tokens, while this
session's goal tool returns no goal. Do not invent a current token-meter value.
Prior comprehensive tests preceded final retention refinements; focused tests
and strict Clippy qualified those refinements. Older entries retain historical
status, including the former provider join blocker. Existing runbook length
debt is unchanged by this bounded amendment review; no history is deleted.

## Turn 406 | 2026-09-29

P220 version 23 and checkpoint `cfaf3f12` adapt presentation retention to the
accepted shared-desktop model. Multiple Agent Browser browsers may retain one
exact Remote View registration, pool, assignment, desktop generation, route,
and viewer-session set while preserving distinct browser, profile, session,
tab, and target identity. A durable detach removes one browser reference
without releasing the assignment. Joined provider release fails while any
peer browser remains active. Partial or conflicting shared identity fails both
before and after detach.

The legacy schema-v1 `placementId` remains an empty string for older-reader
compatibility but is no longer validated, retained from input, or used as
authority. Five focused retention tests, the full 228-test service-model unit
suite and all service-model integration tests, the SQLite detach/release
restart test, the self-testing architecture guard, formatting, and strict
workspace Clippy pass. The first comprehensive replay attempt failed before
compilation in the optional cache wrapper; the documented cache-off replay
passed and preserves that infrastructure failure rather than erasing it. No
live Remote View, browser, provider, install, privilege, production, or release
effect occurred.

## Turn 405 | 2026-09-29

P220 version 22 records the bounded Remote View source integration result and
checkpoint `716ef3a5`. Remote View `origin/main` at `0a42bff5` now exposes exact
live launch resources in installed desktop lifecycle observations. Agent
Browser projects the selected desktop UUID and generation to an authoritative
display name, rejects missing, stale, or invalid context before admission or
Chrome launch, and no longer requires managed application placement. The
default host remains fail-closed until the public adapter supplies context.

The spike also demonstrated one narrower Remote View gap: installed dynamic
control and gateway viewing use different desktop UUID projections, while
public window actions remain slot-addressed. Agent Browser cannot infer the
join from route labels, display numbers, capacity order, or local conventions.
Remote View issue #146 requests one authenticated generation-bound join for
viewing, window inventory, and exact window activation. It does not request
browser launch, process/profile/CDP ownership, application placement,
embedding, or one browser per desktop. Focused service-model and CLI tests,
the self-testing P220 architecture guard, formatting, and strict workspace
Clippy pass. No live Remote View, browser, provider, install, privilege,
production, or release effect occurred.

## Turn 404 | 2026-09-29

P220 version 21 corrects the Remote View consumer boundary after the
Alice/Bob design grilling. Remote View managed `application place` and
`application stop` are not Agent Browser dependencies and no longer block the
plan. Agent Browser owns browser launch, process/profile/CDP identity, logical
sessions and tabs, foreground arbitration, recovery, and browser cleanup.
Remote View owns the Agent Browser-reserved desktop pool, generation-bound
desktop environment, application/window observation, viewing, OS-level effects,
events, and exact desktop lifecycle.

The next critical-path packet is a bounded source integration spike over the
existing Remote View pool-assignment, desktop-environment, window-inventory,
event, viewing-route, and window-raise surfaces. One service-level assignment
may host multiple Agent Browser browsers; empty desktops are preferred but not
required. Durable Agent Browser handoffs resolve one logical tab through its
browser's current desktop. Focus combines CDP target activation and Remote View
window raise under one serialized heartbeat/TTL lease epoch. Attributable
operator stop suppresses recovery until explicit reopen; unattributed loss uses
bounded demand-driven recovery. Assignment release, return to the reserved
pool, and clean pool scale-in remain separate transitions.

The prior Remote View issue #62 request for an installed application-placement
effect is withdrawn as a P220 prerequisite. Any new Remote View dependency must
come from a concrete failure in the bounded integration spike. No live Remote
View, browser, provider, install, privilege, production, or release effect was
authorized or performed during this correction.

## Turn 403 | 2026-09-29

P220 Plan version 20 corrects the Remote View ownership boundary. Remote View
does not allow, disallow, expose, adopt, inspect, or validate CDP. Its missing
runtime contribution is only the installed, generation-bound, allowlisted
application-placement effect and its own operation, placement,
application-instance, and effect-status evidence. Agent Browser independently
owns browser launch or adoption, process identity, profile, CDP, targets,
readiness, and recovery after placement. The existing Remote View issue #62
comment was edited in place to preserve its locator while removing the broader
CDP request. No browser, Remote View, installed, provider, privilege,
production, or release effect occurred.

## Turn 402 | 2026-09-29

P220 Plan version 19 records the exact external custody seam on Remote View
issue #62. Remote View Milestone 5 currently launches applications under an
owned service/cgroup, while Plan 0015 leaves browser process, profile, CDP, and
recovery ownership with the consumer. Agent Browser requested an exact,
generation-bound, allowlisted application-placement effect with inspectable
unknown outcomes. Remote View returns only its own placement and effect
evidence; Agent Browser owns CDP independently. It did not request embedding,
dynamic capacity, live effects, caller-supplied shell execution, or ownership
of Remote View internals. No browser, Remote View, installed, provider,
privilege, production, or release effect occurred.

## Turn 401 | 2026-09-29

P220 Plan version 18 prepares the separately gated installed-acceptance packet
without authorizing or performing effects. It fixes the target to two reviewed
Remote View desktops and two disposable development profiles; orders baseline,
single-association, many-to-many, recovery, exact-cleanup, and fresh-census
axes; binds required identities; and records hard stops plus forward-only
unknown-outcome recovery. The evidence table keeps source qualification,
installed proof, protected integration, and operator authorization distinct.
Remote View's installed application-placement effect remains the next external
dependency. No browser, Remote View, installed,
provider, privilege, production, or release effect occurred.

## Turn 400 | 2026-09-29

P220 checkpoint `c1f5fe27` proves the existing explicit final-session browser
cleanup path directly. Closing the only logical session invokes one exact
browser close, issues no redundant tab-close effect, clears active browser,
session, and tab records, and preserves terminal histories. The focused
service-model test and workspace formatting pass. This resolves the stale plan
item for Agent Browser-owned final-session cleanup; it does not fabricate a
Remote View release. Joined presentation cleanup still requires authenticated
assignment, generation, route, and complete viewer-session evidence from the
missing runtime ingress. No browser, Remote View, installed, provider,
privilege, production, or release effect occurred.

## Turn 399 | 2026-09-29

P220 checkpoint `e36f7d42` adds additive, read-only Browser Runtime health to
Service status, the generated client type, and install doctor. It reports
redacted SQLite integrity and size, migration archive state, verified-backup
state, and current browser-launch admission. Missing or invalid storage
returns a stable failure code and never its path; status does not migrate the
database or create a backup. Focused available and unavailable Rust tests,
Service-status no-persistence coverage, generated-client checks, TypeScript,
observability helpers, cross-seam schema tests, API/MCP parity, install-doctor
provenance tests, P220 architecture and custody guards, documentation links,
remote-view docs contracts, formatting, strict workspace Clippy, and the
production docs build pass. A browser-launching collection smoke timed out
before status assertions and is not acceptance evidence. Its exact
`sc-24089` process tree and empty disposable directory were removed; no
matching residue remained. No production, installed, provider, privilege, or
release effect occurred.

## Turn 398 | 2026-09-29

P220 checkpoint `496c25f8` restores the provider-neutral browser-launch
resource admission selected from P219. The Browser Session worker now checks
available memory, profile-filesystem space, host PID capacity, and Agent
Browser root-process count immediately before `BrowserManager::launch`.
Resource pressure returns a typed error without a browser effect. Unsupported
Remote View desktop placement retains its exact earlier fail-closed error, so
the packet does not mask or invent the missing installed application-placement
effect. Both focused admission tests, the runtime
no-launch test, version sync, P220 architecture guard, P219 custody ledger,
formatting, strict workspace Clippy, and diff hygiene pass. No live Remote
View, browser, provider, install, privilege, production, or release effect
occurred.

## Turn 397 | 2026-09-29

P220 checkpoint `d3e31c6f` adds the provider-neutral operator-handoff
projection. An active retained public binding and a bounded path-safe Agent
Browser handoff ID produce exactly `handoffId` and
`/remote-view/<handoff-id>`. Released bindings and slash-bearing IDs fail
closed. A self-testing architecture rule rejects any route, desktop, provider,
display, or credential reference in the projection body. Focused retention
tests, the P220 architecture guard, formatting, and strict workspace Clippy
pass. Link construction does not claim viewer readiness or perform a runtime
effect.

## Turn 396 | 2026-09-29

P220 follow-up `fd4264ec` wraps public Remote View presentation bind and joined
release in immediate SQLite transactions over the complete Browser Session
aggregate. This closes the lost-update gap before runtime ingress exists. The
focused close/reopen test proves both active bind and released state persist;
the architecture guard and strict workspace Clippy pass. No caller-facing
mutation endpoint was added, so untrusted callers cannot forge provider
identity while the real Remote View runtime adapter remains unavailable.

## Turn 395 | 2026-09-29

P220 checkpoint `b47f7a0c` retains one validated Remote View public
registration, pool, assignment, placement, desktop generation, opaque route,
and viewer-session set against the exact Agent Browser browser, profile,
session, tab, and target. Binding is idempotent across viewer-list ordering and
rejects active identity reuse. Joined release mutates only when assignment,
desktop generation, route, and the complete viewer-session set match exactly.
The record survives a SQLite restart and contains no provider URL, display
number, credential, or provider-private state. The full 228-test service-model
unit suite and all integration tests, the focused CLI restart test, P220
architecture guard, formatting, and strict workspace Clippy pass. Runtime
invocation remains blocked on Remote View's missing installed
application-placement effect. No live runtime effect occurred.

## Turn 394 | 2026-09-28

P220 checkpoint `d7c43915` exposes verified Browser Runtime SQLite backup
status and creation through `service runtime-backup status|create` and the
generic HTTP and MCP `service_request` transport. The generated client carries
typed status and manifest responses. Status is read-only; creation performs an
online integrity-checked backup and rotates at most one previous copy. Neither
surface restores data or includes the separate Service State store. Focused
Rust backup and contract tests, formatting, strict Clippy, generated-client
contract and type checks, request-client tests, API/MCP parity, the no-launch
contract smoke, P220 architecture guard, documentation links, and the
production documentation build pass. The umbrella service-client gate stops
at a pre-existing stale P157 source-literal oracle that disagrees with both
the checkpoint baseline and current provenance helper name. No browser,
provider, install, privilege, production, or release effect occurred.

## Turn 393 | 2026-09-28

P220 checkpoint `a2080250` adds a deterministic, self-testing architecture
guard for the successor path. It rejects Remote View provider-private types in
the service model, retired Guacamole, XRDP, route-keeper, or presentation-queue
authority in the supported P220 modules, local-display inference from public
desktop identity, persistence dependencies in the pure model crate, canonical
J3 checkpoint drift, and loss of strict private-field or joined-cleanup
validation. The guard, its four negative fixtures, the 63-file P219 custody
ledger, validation-selection suite, release-asset fixture, documentation links,
and diff hygiene pass. No runtime or provider effect occurred.

## Turn 392 | 2026-09-28

P220 checkpoint `64f75821` extracts the retained browser-recovery model without
presentation ownership. Exact-client resume and authenticated active-viewer
demand may admit a replacement only after the old browser is proven unusable;
dormant demand waits, unknown health fails closed, retry backoff and deadlines
are bounded, and every observed-live, failure, and success transition is fenced
by generation. The recovery registry commits transactionally in SQLite and
replays the admitted generation after restart. The complete service-model
package, focused browser-session tests, formatting, and strict workspace Clippy
pass. Runtime observation and replacement-launch wiring remain pending, and no
browser or provider effect occurred.

## Turn 391 | 2026-09-28

P220 checkpoint `ca3709e1` replaces the supported browser-session JSON path
with a bounded SQLite authority for session state and the profile catalog. A
one-time migration stages and validates the database before atomic publication,
archives all legacy inputs read-only, and treats any existing database as
authoritative rather than falling back on stale JSON. Online backup creation
verifies SQLite integrity and schema before publication, binds a manifest to
the exact digest and byte count, and retains one previous copy. Focused store
and session tests, both anomalous broad-suite tests under the supported runner,
formatting, and strict workspace Clippy pass. Operator-facing backup status and
creation remain to be adapted from P219.

Remote View `origin/main@30f3e37` has transport-reachable CLI, HTTP, and MCP
entrypoints, but its `ControlRuntime` still instantiates `ProviderFreeHost`,
`ProviderFreeApplicationEffect`, and `ProviderFreeControl`. This clears the
public contract wait but does not provide the installed application-placement
effect needed to replace Agent Browser's placement-unavailable stop. Agent
Browser retains browser launch and CDP ownership. Independent P220 extraction
continues; no legacy display inference or Agent Browser-owned presentation
provider is restored.
No live Remote View, browser, provider, install, privilege, production, or
release effect occurred.

## Turn 390 | 2026-09-28

P220 source checkpoint `552c8162` consumes Remote View's strict public J3
registration, assignment, ready-placement, opaque-route, viewer-session status,
and joined-release records in `agent-browser-service-model`. Its source-bound
fixture proves two desktops, distinct placement and route identity, independent
desktop/mobile sessions, exact route-and-session retirement, and rejection of
browser-private state or partial cleanup. The complete service-model package
passes with 228 unit tests plus all integration tests; workspace formatting and
strict Clippy pass. This is provider-free wire-consumption evidence only. The
next packet must replace the CLI's placement-unavailable stop with an injected
Remote View adapter while preserving browser/profile/CDP authority and the
opaque route-ID versus durable operator-handoff distinction. No live Remote
View, browser, provider, install, privilege, production, or release effect
occurred.

## Turn 389 | 2026-09-28

Remote View Plan 0017 is closed from corrected canonical evidence. J2
`7298ae7f76626c61ba5086eb4f934699251009ce` and J3
`018d3d752f9d99805242f99c00034f0caabc53d1` publish registration, bounded
pools, exact assignment and placement, opaque route identity, layered viewer
status, truthful reacquisition, placement stop, and joined cleanup. Corrective
source `b7caa1f209f2d64cd400b1766c50389ab4dc5cb3` makes the operation matrix
reachable through real CLI, authenticated HTTP, and MCP adapters; governance
evidence is canonical at `eefddd2c862ce97bcf4387b2cc43dbbb2d9274eb`, and
Remote View `origin/main@30f3e37` reconciles the closure narrative. P220's
external J2/J3 wait is therefore cleared. Its next packet is Agent
Browser-owned provider-free consumption and replay, including replacement of
the temporary placement-unavailable boundary, retention mapping, exact cleanup,
and a handoff adapter that does not confuse Remote View's opaque `routeId` with
the operator-facing `/remote-view/<handoff-id>` URL. SQLite backup and recovery
authority, remaining admission and observability extraction, the architecture
guard, protected integration, and separately authorized installed acceptance
remain. Remote View's provider-free and production-entrypoint evidence does not
authorize an Agent Browser live effect.

## Turn 388 | 2026-09-28

P220 is active on `platform/p220-remote-view-consumer` from canonical
`origin/main@a3848e16`. Checkpoint `9a34e292` adds the first provider-free
Remote View F0 consumer boundary and proves two distinct Agent Browser-owned
browser/profile associations over distinct fixed desktop identities. The
source through `d6f390be` also extracts the first presentation-neutral
browser/session spine with independent SQLite state, restart reattachment,
idle reaping, focus, named-tab lifecycle, and addressed ordinary commands.
Desktop choice consumes Remote View UUID, route-label, generation, and
readiness observations; the legacy route inventory is not selection authority
and real placement fails closed until the public J2 contract exists. J1
lifecycle and durable operation status are source-bound to Remote View
`f674518e34fea346002c72c4adc3966b628d0b78`; exact release preserves desktop
UUID and generation, while tampered operation payload evidence fails closed.
Disposable browser retention now has count and regular-file byte ceilings,
oldest-first eviction, explicit protected-session fencing, and no pin or
promotion bypass. Exact URL history compacts deterministically into
restart-safe daily identity summaries before host persistence. The retired
Agent Browser viewer/controller tables are not reintroduced; Remote View J2
must later supply any presentation-derived protected-session evidence.
The preserved P219 branch is not merged wholesale;
its Agent Browser-owned XRDP/Guacamole presentation path is retired. Remote
View J2 and J3 contracts, SQLite backup and recovery extraction, protected
integration, and separately authorized installed acceptance remain. No live
Remote View, browser, provider, install, privilege, production, or release
effect occurred.

## Turn 387 | 2026-09-17

P216 source candidate `6ff7bc0d` and candidate-freeze receipt `3fdfc147`
entered the single protected integration path through PR #200. The canonical
provider-free Service State model now lives in `agent-browser-service-model`;
the CLI retains every filesystem, process, browser, runtime-owner, HTTP, MCP,
and platform effect. Complete local changed-surface qualification passes. The
build claim is limited to the measured cold pure-model loop, from 171.01
seconds to 4.08 seconds, not general CLI or workspace acceleration. Issue #178
and Plan 0216 close with the protected merge. P211 remains separate and must
reconcile the integrated model boundary before continuing overlapping urgent
bug-fix work. GitHub Actions remained disabled, and no browser, provider,
credential, install, runtime, staging, production, or release effect occurred.

## Turn 386 | 2026-09-17

P213 exact head `c8012bd0` merged through PR #196 as `7e56d9c7`; issue #194
closed and no GitHub Actions branch or merge-head run started. P214 is admitted
from that canonical baseline in the clean reassigned P213 worktree; no checkout
was created or removed. It owns only an effect-free desktop-services planner
that accounts every raw pointer move, down and up against the P212 permit
budget. P205 retains the root Cargo manifest and lockfile, P211 source remains
disjoint, and shared planning projections are an explicit reconciliation
overlap. Source checkpoint `f90ef7a7` implements canonical permit validation,
exact-budget interpolation, monotonic checked scheduling and deterministic
plan digests. All 18 desktop-services tests, all 58 challenge-control tests,
the strengthened architecture guard, workspace formatting, strict workspace
Clippy, documentation links, planning audit, selection and diff hygiene pass
locally. GitHub CI remains disabled and was not restored or run. Publication
and protected integration remain. No browser, capture, provider, credential,
CAPTCHA, route claim, desktop input, runtime or production effect occurred.

## Turn 385 | 2026-09-17

P212 exact head `dc20155e` merged through PR #193 as `ddae1897`. No GitHub
Actions branch or merge-head run started. P213 is admitted from that canonical
baseline in the clean reassigned P212 worktree; no checkout was created or
removed. It owns only a pure challenge-control adapter that proves a visual
intent is the current state-machine-authorized intent before mapping it to the
P212 desktop permit. P205 retains the root Cargo manifest and lockfile; P213
avoids both. P211's source remains disjoint and shared planning projections are
an explicit reconciliation overlap. Source checkpoint `1bad68e3` implements
the exact join and bounds permit expiry by the earlier evidence or visual
policy deadline. All 58 challenge-control tests, all 12 desktop-services tests,
the strengthened architecture guard, workspace formatting, strict workspace
Clippy, documentation links, planning audit, selection and diff hygiene pass
locally. GitHub CI remains disabled and was not restored or run. Publication
and protected integration remain. No browser, capture, provider, credential,
CAPTCHA, route-claim, desktop-input, runtime or production effect occurred.

## Turn 384 | 2026-09-17

P210 exact head `343a61b9` merged through PR #192 as `06972a5e`. P212 is
admitted from that canonical baseline in the clean reassigned worktree; no new
worktree was created. It owns only an effect-free desktop-services candidate
geometry and controller-authority contract plus provider-free fixtures. P211's
active cold-upgrade branch touches CLI shutdown and workstation routing, not
the P212 source surface; shared planning files are an explicit reconciliation
overlap. Source checkpoint `00f41715` now binds exact observation and ordered
candidate geometry to current controller authority and checked effect budgets.
All 12 desktop-services tests, the strengthened architecture guard, workspace
formatting, strict workspace Clippy, documentation links and diff hygiene pass
locally. GitHub CI remains disabled and was not restored or run. Publication
and protected integration remain. No browser, capture, provider, CAPTCHA,
desktop-input, runtime or production effect occurred.

## Turn 383 | 2026-09-17

P210 source checkpoint `0729b63d` adds the pure visual artifact and one-shot
injected transport adapter. Ten adapter fixtures prove exact payload custody,
deterministic digests, zero-call invalid input, explicit byte ceilings,
one-call transport and malformed-output failure, strict effect-smuggling
rejection, typed abstention, delayed response receipt time and raw-byte
redaction. Review repaired exact artifact/request expiry binding and separated
request time from transport receipt time. All 52 challenge-control tests, 10
adapter tests, both architecture guards, workspace formatting, strict
workspace Clippy and diff hygiene pass. GitHub CI remains disabled and was not
restored or run. Publication and protected integration remain. No browser,
image capture, real provider, credential, CAPTCHA, desktop-input, runtime or
production effect occurred.

## Turn 382 | 2026-09-17

P209 exact head `27cd5342` merged through PR #188 as `fb616aee`. P210 is
admitted from that exact canonical baseline on
`challenge/p210-visual-artifact-adapter` in the clean reassigned challenge
worktree. No new worktree was created. The baseline selector reports no
changed files and only diff hygiene. P210 owns a new pure visual-adapter crate,
repository-owned synthetic bytes and one injected fake transport; it owns no
real provider, network, browser, capture, credential, CAPTCHA, desktop-input,
runtime, production or CI effect.

## Turn 381 | 2026-09-17

P206 exact head `82e25624` merged through PR #180 as `d3f923a1`. P209 joined
that canonical result at `3ef2ad9e`; the only conflict was the active-lane
catalog, resolved by preserving the canonical CI-shutdown record and P209's
bounded entry. No P209 Rust source or Cargo dependency changed. The reconciled
challenge-control package passes all 52 tests, including 25 visual-round and
11 provider-protocol cases. GitHub CI remains disabled and was not restored or
run. P209 is ready for publication and protected integration. No provider,
image, browser, credential, CAPTCHA, desktop-input, runtime or production
effect occurred.

## Turn 380 | 2026-09-17

P209 merge `9df85b9b` joins corrected published P206 head `72eeea03`. The
combined dependency head passes all 52 challenge-control tests, including 25
visual-round and 11 provider-protocol cases, the crate architecture guard,
strict workspace Clippy, formatting and diff hygiene. P209 remains local and
unpushed until P206 PR #180 enters canonical `main`. No provider, browser,
CAPTCHA, credential, runtime, production or CI-dispatch effect occurred.

## Turn 380 | 2026-09-17

P206 joined operator-disabled-CI `main@f6d49f89` at `bb961c96`, P208's source
integration at `db987e4e`, and canonical `main@692f77c6` at `1c9ee159` after
P208 closed. The only conflicts were shared runbook projections, resolved by
retaining both plans' records. None of these main slices changes
challenge-control source or Cargo metadata, so the repaired
41-test source evidence at published head `72eeea03` remains reusable.
Conflict-affected policy wiring, documentation links, validation-selection,
P208 closeout fixtures, active planning audit and diff hygiene pass locally.
GitHub CI remains disabled by operator direction; no workflow was restored,
dispatched, retried or run. Reconciled publication and protected integration
remain. No browser, provider, CAPTCHA, credential, runtime or production
effect occurred.

## Turn 379 | 2026-09-17

P206 pre-merge review reproduced two budget-boundary defects: cumulative
selection arithmetic could saturate and admit an actual total above 255, and a
256-candidate selection returned a generic transition error rather than typed
round-budget intervention. Repair checkpoint `4813d385` replaces saturation
with widened and checked arithmetic. All 41 challenge-control tests, including
25 visual-round cases, the crate architecture guard, strict workspace Clippy,
formatting and diff hygiene pass. Corrected published head `72eeea03` requires
protected exact-head evaluation. No provider, browser, CAPTCHA, credential,
runtime or production effect occurred.

## Turn 378 | 2026-09-17

P209 checkpoint `f9987721` proves the response digest binds request, evidence,
candidate-set, capability, selected-candidate order, production-time and expiry
fields. All 50 challenge-control tests, strict workspace Clippy, formatting and
diff hygiene pass. No provider, browser, CAPTCHA, credential, runtime,
production or CI-dispatch effect occurred.

## Turn 377 | 2026-09-17

P209 checkpoint `89edafd5` proves request preparation rejects invalid policy,
mutated evidence, malformed artifact identity or digest, pre-observation and
expired preparation times, and over-budget execution plans before a provider
request exists. The complete 49-test challenge-control crate, strict workspace
Clippy, formatting and diff hygiene pass. No provider, browser, CAPTCHA,
credential, runtime, production or CI-dispatch effect occurred.

## Turn 376 | 2026-09-17

P209 checkpoint `e7250217` completes the provider-response temporal fixture:
pre-request, future-produced, produced-at-expiry, expired-at-adjudication,
beyond-request-expiry and request-expiry cases all fail closed as stale. The
complete 49-test challenge-control crate, strict workspace Clippy, formatting
and diff hygiene pass. No provider, browser, CAPTCHA, credential, runtime,
production or CI-dispatch effect occurred.

## Turn 375 | 2026-09-17

P209 review-hardening checkpoint `3aed9a4b` explicitly proves strict request
deserialization rejects coordinate, event-sequence, retry and instruction
smuggling plus nested artifact bytes and execution-plan repeat authority. All
49 challenge-control tests, strict workspace Clippy, formatting and diff
hygiene pass. The branch remains local and unpushed behind P206 PR #180; no
provider, image, browser, credential, CAPTCHA, desktop-input, runtime,
production or CI-dispatch effect occurred.

[Plan 0210](docs/dev/plans/0210-2026-09-17-visual-artifact-and-provider-invocation-adapter.md)
records the proposed W7-C artifact-custody and one-shot fake-provider adapter.
It is `PLANNED | NOT ADMITTED`; no branch, worktree or implementation has
started, and P209 canonical integration is its hard source-admission gate.

## Turn 374 | 2026-09-17

P209 local checkpoint `8ada33d5` is accepted. The pure protocol binds
prepared visual artifacts and P206 evidence into deterministic provider
requests, admits only candidate identities or typed abstention, and binds a
caller-owned, policy-checked execution budget before provider adjudication. It
rejects serialized coordinate, event, retry and instruction smuggling. All 48
challenge-control tests, strict workspace Clippy, formatting,
four architecture guards and 114 selector-expanded extracted-crate tests pass;
the exact P206 dependency reconciliation also passes the 48-test
challenge-control compartment, formatting and strict workspace Clippy.
The branch has locally joined published P206 head `99793061` and remains
unpushed until PR #180 enters `main` and the canonical checkpoint is reconciled.
No browser, provider, CAPTCHA, credential, runtime or production effect
occurred.

## Turn 373 | 2026-09-17

[Plan 0206](docs/dev/plans/0206-2026-09-16-visual-multi-round-challenge-contract.md)
is source-complete and acceptance-complete at `ac9f50a7` on
`challenge/p206-visual-round-contract`,
based on exact published P197 head `cd22a39f`. The pure challenge-control
contract keeps visual rounds inside one attempt, binds each selection and
effect receipt to fresh evidence and exact candidate identities, preserves
after-state continuity, and enforces per-round plus cumulative budgets. All 39
crate tests, including the complete 23-case visual-round matrix, the crate
architecture guard, formatting, strict workspace Clippy and diff hygiene pass.
P197 head `cd22a39f` passed all ordinary required checks and merged through PR
#157 as `c855fc33`. P206 joined that canonical checkpoint at tree-preserving
merge `5d6e3d57`, then joined merged P204 and current `main@f5e3f31b` at
`89bdfdbf`. Reconciled local validation passes the 39-test challenge-control
compartment, strict workspace Clippy and formatting, four architecture guards,
and 114 selector-expanded extracted-crate tests. Exact-head forge evaluation
and protected P206 integration remain. No browser, model provider, CAPTCHA,
desktop input, credential, installed runtime or production effect occurred.

## Turn 376 | 2026-09-17

[Plan 0208](docs/dev/plans/0208-2026-09-16-worktree-closeout-candidate-custody.md)
is closed. PR #182 merged validated source `c2ce2b35` into `main` as
`59928044`; issue #171 closed automatically. The durable advisory closeout
transaction, candidate archive locator, two-process serialization, explicit
retain/archive/discard dispositions, interrupted-effect recovery, policy, and
dormant Repository Tooling definition are integrated. GitHub CI remains
disabled. No real worktree, candidate, browser, provider, installed-runtime,
Service State, production, or release effect occurred.

## Turn 375 | 2026-09-17

[Plan 0208](docs/dev/plans/0208-2026-09-16-worktree-closeout-candidate-custody.md)
exact-head `8a010264` passed GitHub run `35227459177`, including Repository
Tooling and the stable Presubmit aggregate. Before integration, operator-directed
PRs #185 and #186 disabled GitHub CI and advanced `main` to `f6d49f89`. P208 is
rebased onto that tip without restoring an active workflow or trigger. The
dormant workflow retains Repository Tooling, the obsolete comprehensive-job
fixture expectation is removed, and the conflict-affected repository-tooling,
selector, aggregate, dormant-workflow, policy, planning, documentation-link,
and docs-build checks pass locally at `8c513789`. PR #182 integration remains.
No real worktree, candidate, browser, provider, installed-runtime, Service
State, production, or release effect occurred.

## Turn 373 | 2026-09-17

[Plan 0208](docs/dev/plans/0208-2026-09-16-worktree-closeout-candidate-custody.md)
is rebased onto `origin/main@f5e3f31b` after P204 merged through PR #179. The
provider-free closeout transaction, candidate archive locator, two-process
serialization, retain/archive/discard matrix, and interrupted-effect recovery
remain source-qualified. Checkpoint `9d34365d` adds package registration and a
dedicated `Repository Tooling` selector and CI lane; repository-tooling,
validation-control-plane, workflow syntax, and release-verifier fixtures pass
locally. P208 now owns the shared integration transition while preserving
P204's still-open issue #164 records. Exact-head PR #182 evaluation and
protected integration remain. No real worktree, candidate, browser, provider,
installed-runtime, Service State, production, or release effect occurred.

## Turn 374 | 2026-09-17

The operator clarified that CI itself should be disabled for now, not merely
the full-suite routes. Run `35228725370` was cancelled. The active
`.github/workflows/ci.yml` is removed and the reviewed path-selected workflow
is retained as `.github/workflows/ci.yml.disabled` at candidate `ca077d9e`,
which GitHub does not load.
There are no automatic or manual CI triggers. Re-enablement requires new
maintainer direction. The separate Lease Authority CI matrix is also retained
as `.github/workflows/lease-authority.yml.disabled` in candidate `ea254ecd`; no
active workflow has a push or pull-request trigger. Manual release and governed P158 operational
workflows remain separate and were not dispatched.

## Turn 373 | 2026-09-17

P204 initially interpreted operator direction as removing full CI while keeping
focused PR CI. The
bounded correction removes `main` push, scheduled, manual CI dispatch, and
commit-message qualification routes together with the comprehensive Rust and
slow platform jobs. Pull requests retain path-selected jobs, broad ordinary
fail-safe coverage, superseded-head cancellation, and the stable `Presubmit`
aggregate. Candidate `d9fede9d` passes the selector and workflow contract suite
and `actionlint`. The local comprehensive Rust command remains available outside
GitHub CI. Issue #164 remains useful for enforcing `Presubmit`, but it is no
longer a dependency for removing duplicate post-merge CI.

## Turn 372 | 2026-09-16

[Plan 0204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
has published source checkpoint `b481ab01`. CI run `35172965793` passed every
selected ordinary gate and the stable `Presubmit` aggregate; comprehensive and
platform qualification remained excluded. Superseded run `35170641311`
cancelled as designed. Failed run `35171646162` exposed same-target CLI test
binary replacement, and the corrected two-lane runner then passed. P204 is
reconciled with `main@c855fc33`; protected PR evaluation of the merge result
remains. The plan stays open for post-merge docs-only and narrow-Rust evidence,
an explicitly authorized comprehensive dispatch, and issue #164 branch-rule
enforcement before the temporary `main` fallback can be removed.

## Turn 371 | 2026-09-16

[Plan 0204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
has implementation checkpoint `7f6c7e2e`. The versioned classifier now drives
exact-head conditional jobs and the stable fail-closed `Presubmit` aggregate;
unknown and control-plane changes fail safe, while explicit comprehensive
qualification avoids duplicate focused Rust. Selector, aggregate, economics,
documentation-link, workflow, docs-build, policy, planning, and Challenge
Control compartment validation is green locally. One independent review and
bounded rework corrected every blocking finding. Organic PR receipts and an
explicitly authorized comprehensive dispatch remain pending. The `main`
fallback remains because issue #164 has not proved live required-check
enforcement. No workflow dispatch, branch-rule, browser, provider, credential,
installed-runtime, Service State, production, or release effect occurred.

## Turn 370 | 2026-09-16

### P204 admission

[Plan 0204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
is admitted from `main@2632e31c` for issue #174. The current selector is
advisory and has no versioned tier, fixed job outputs, unknown-impact fallback,
or direct fixtures; CI does not consume it and has no PR cancellation or stable
aggregate check. A docs-only probe is red on the missing contract. P204 owns the
selector, CI workflow, aggregate verifier, and provider-free fixtures. The
`main` fallback remains until issue #164 proves live `Presubmit` enforcement.
No workflow dispatch, branch-rule mutation, browser, provider, credential,
installed-runtime, Service State, production, or release effect is authorized.

### P197 integration

[Plan 0197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md)
joins canonical `main@2632e31c` after P202 protected integration and closeout.
The source merge is clean outside this runbook projection, retains the complete
P197 consumer-admission implementation and combined provider-free validation,
and removes P202 as an active dependency. Final plan and lane reconciliation,
branch publication at `4321961c` is complete; exact-head forge evaluation and
protected P197 integration remain. P197 now has durable remote custody and its
clean primary checkout can be released. W7-A is selected as the next
provider-free challenge packet but is not yet admitted. No browser, CAPTCHA,
provider, credential, installed-runtime, Service State, production, release, or
CI-policy effect occurred.

## Turn 369 | 2026-09-16

[Plan 0202](docs/dev/plans/0202-2026-09-16-abandoned-service-browser-retirement.md)
is closed. [PR #168](https://github.com/CochranResearchGroup/agent-browser/pull/168)
merged source head `6f099292` into `main` as `528f2ef0`; issue #103 closed.
Provider-free qualification and the isolated disposable real-browser acceptance
passed. CI run `35163527521` passed every ordinary gate at reviewed code head
`d7ceca98`; the final head added only integrated P203 closeout documentation,
and its in-flight Rust rerun was cancelled after the PR merged. P202 is removed
from the active-lane catalog and releases its shared surfaces to P197. No
browser, provider, credential, profile, installed-runtime, Service State,
production, or release effect occurred during integration or closeout.
