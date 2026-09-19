# Runbook

Current index. [The September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md) preserves checkpoints through Turn 312; [Turn 313](RUNBOOK-history-2026-09-13-turn313.md), [Turns 314 through 320](RUNBOOK-history-2026-09-14-turn314-through-turn320.md), [P169 history through Turn 319](RUNBOOK-history-2026-09-14-p169-through-turn319.md), [superseded P186 checkpoints through Turn 328](RUNBOOK-history-2026-09-14-p186-through-turn328.md), [P194 through P196 checkpoints from Turns 340 through 343](RUNBOOK-history-2026-09-15-p194-through-turn343.md), and [Turns 323 through 368](RUNBOOK-history-2026-09-14-turn323-through-2026-09-16-turn368.md) remain separately preserved.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P211](docs/dev/plans/0211-2026-09-17-simple-cold-upgrade.md) and [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)

## Turn 413 | 2026-09-19

P211 checkpoint `18c790bd` removes the legacy hidden-viewer bootstrap from the
runtime and development provider. Browser Session Host desktop placement now
projects only `Ready` route-keeper receipts from the SQLite authority. Legacy
bootstrap environment input cannot alter navigation, and an explicit legacy
marker is rejected. Until keeper-backed handoff resolution is joined, every
remote-required manager open fails before host loading or browser effects with
a typed unavailable or integration-pending result. Development provider
preflight and the real system-effects adapter likewise fail before mutation;
the removed hidden Chrome viewer is not a fallback. The independent review
caught and verified correction of an initial adapter-seam bypass. The provider
fixture, six workstation and Guacamole fixtures, thirteen Browser Session Host
tests, the fail-before-browser-effects daemon test, documentation handoff
contract, production docs build, formatting, strict workspace Clippy, and diff
hygiene pass. User-facing cold-install guidance now matches the forward-only
controller. The authoritative connection catalog, exact XRDP observer and
stop, keeper-backed handoff join, and live readiness remain open. No provider,
browser, installed-runtime, production, ingress, or release effect occurred.

## Turn 412 | 2026-09-19

P211 checkpoint `e9593b5b` adds a browser-independent Guacamole connection
specification and a concrete route-keeper primary factory. The specification
accepts only a nonempty connection ID and the exact credential-free,
query-free, literal-loopback `/guacamole/` provider path. Existing
browser-bound primaries use the same specification and authentication flow.
The configured factory selects one immutable specification by exact keeper
slot, fails an unmapped slot before any provider request, passes the same fresh
SQLite guard through authentication and transport custody, and keeps provider
tokens transient. The 21-test Guacamole-primary lane, eleven route-keeper
connector tests, formatting, strict workspace Clippy, diff hygiene, changed-
surface selection, and independent closed-world review pass. No authoritative
slot-to-connection catalog is yet populated, and the factory is not installed
in RuntimeHostRouter; XRDP observation and exact stop also remain injected.
No provider, browser, installed-runtime, production, ingress, or release effect
occurred.

## Turn 411 | 2026-09-19

P211 checkpoint `1d17f471` adds the provider-free route-keeper supervisor and
durable terminal reconciliation. Startup admits the configured minimum-ready
route before warming one additional action per timed tick, and shutdown can
interrupt an in-flight connector await. Every shutdown and error exit closes
retained primary tasks, drains their exact terminal events, persists the
resulting transition through SQLite compare-and-swap, and only then releases
task custody. A terminal before first readiness returns the slot to `Absent`;
a failed recovery preserves the prior exact Guacamole, XRDP, and display
receipt in `RecoveryFailed`; a ready disconnect becomes degraded; and a
terminal during exact stop quarantines the retained protocol identities and
cleanup obligation. Changed occurrence or fence evidence cannot mutate or
release the current task, and a compare-and-swap failure restores the event
for replay. The full Service Model package, six focused model tests, eleven
adapter tests, ten connector tests, all 21 Guacamole-primary tests, formatting,
strict workspace Clippy, diff hygiene, and independent closed-world review
pass. The concrete provider-authenticated task factory and connection catalog,
XRDP observer and exact stop implementation, RuntimeHostRouter integration,
and live provider readiness remain open. No provider, browser,
installed-runtime, production, ingress, or release effect occurred.

## Turn 410 | 2026-09-19

P211 checkpoint `a481d7e1` adds process-local custody for real route-keeper
`PrimaryTask` instances behind injected provider and XRDP seams. Start retains
the task synchronously before any asynchronous wait, exact replay reuses that
occurrence, and a changed keeper or fence cannot replace it. Readiness requires
both the Guacamole connection UUID and an exact XRDP session and display
receipt; the task status and UUID are rechecked after XRDP observation before
SQLite may publish ready. Stop retains task custody across cancellation or
observer errors, freshly proves the exact durable `Stopping` fence before the
destructive observer, and caches the exact terminal receipt so a final
compare-and-swap conflict cannot close twice or become an ownership
quarantine. Terminal callbacks only enqueue bounded slot, keeper, fence,
occurrence, code, and elapsed-time evidence. Cold-process adoption returns an
explicit unsupported result because a `PrimaryTask` cannot survive process
loss; no continuity is synthesized from stale records. Seven connector tests,
seven durable adapter tests, all 21 Guacamole-primary tests, formatting, strict
workspace Clippy, diff hygiene, and independent closed-world review pass. The
concrete provider-authenticated task factory, XRDP observer and exact stop
implementation, terminal-event host reconciliation, scheduler, and provider
readiness remain open. No provider, browser, installed-runtime, production,
ingress, or release effect occurred.

## Turn 409 | 2026-09-19

P211 checkpoint `ab05e2d3` removes the generic `PrimaryGuard` and fresh
authority-check helper from the legacy browser-bound binding and makes them
transport-owned. A new route-keeper guard reloads the SQLite authority before
each Guacamole write and requires the exact slot, keeper, host and operation
fence in an effect-capable phase. A real in-process `PrimaryTask` over a
duplex websocket reaches protocol readiness on the current fence, then closes
without acknowledging the next frame after that fence is superseded. All 21
existing Guacamole-primary tests, the new exact-fence fixture, two selected
stream target-switch tests, formatting, strict workspace Clippy, diff hygiene,
and closed-world review pass. The selector also recommended the live CDP tab
streaming smoke because a stream module changed; it was not run because this
provider-free checkpoint authorizes no browser or installed-runtime effect.
A connector that creates and retains these tasks, XRDP observation, host-loop
scheduling, and actual provider readiness remain open. No provider, browser,
installed-runtime, production, ingress, or release effect occurred.

## Turn 408 | 2026-09-19

P211 checkpoint `89c05d7e` adds the provider-free runtime adapter between the
durable route-keeper authority and an injected protocol connector. `Starting`,
`Adopting`, and `Stopping` persist before their corresponding effect. A
cancelled start or post-effect compare-and-swap conflict resumes through exact
observation without another start. Connector receipts must match the
dispatched slot and fence; ready and adoption also require the exact keeper,
and adoption requires the prior host generation. Same-slot foreign stop
evidence reaches the model's quarantine transition, while an unrelated slot or
fence is rejected without mutation. Repository operations reopen SQLite for
each load or compare-and-swap, so no connection remains held across an async
connector wait. Seven focused adapter tests, strict workspace Clippy,
formatting, the selected architecture guard, all 118 Lease Authority tests,
diff hygiene, and closed-world review are green. The connector remains
injected: this checkpoint does not start `PrimaryTask`, connect Guacamole,
observe XRDP, supervise a running keeper, or establish provider readiness. No
browser, provider, installed-runtime, production, ingress, or release effect
occurred.

## Turn 407 | 2026-09-19

P211 checkpoint `6390a48a` adds a provider-neutral route-keeper lifecycle
kernel and persists its authority in the runtime SQLite database. The kernel
reconciles the default minimum-ready route before the warm target of four,
bounds inventory at six slots, keeps browser, profile, tab, manager-session,
and handoff concepts outside the contract, and projects only protocol-ready
receipts. Host and operation generations fence stale observations. Adoption
has a distinct phase and must preserve the exact Guacamole connection, XRDP
session, and display across the generation change. Exact stop evidence removes
only the recorded route; mismatched or unproven ownership quarantines it with
a retained cleanup obligation. SQLite publication uses an immediate
compare-and-swap transaction and rejects stale or generation-regressing
writers. The full service-model target, all 16 browser-session-store tests,
formatting, strict workspace Clippy, validation selection, diff hygiene, and
two closed-world review passes are green. This checkpoint does not connect the
kernel to the Guacamole transport or XRDP process adapters and does not prove a
running keeper, provider readiness, or cold-start acceptance. No browser,
provider, installed-runtime, production, ingress, or release effect occurred.

## Turn 406 | 2026-09-19

P211 checkpoint `e1a3edcd` hardens the ambiguous interval after browser process
creation and before the `browser_opened` observation. Every reserved launch now
receives an exact causal process marker. The model can adopt an exactly
observed reserved launch without executing another launch and validates the
session, browser, observed desktop, and persisted healthy route without
recomputing placement. An unproven recovery, probe failure, or rejected
observation advances once to `launch_cleanup_required` with one exact pending
obligation; repeated replay returns the same typed failure without launching
or probing again. The real runtime deliberately reports discovery unproven
until cross-platform marker, profile, PID, process-identity, and CDP ownership
proof is implemented. Focused browser-session tests pass 51 with two
real-browser tests intentionally ignored; all 25 service-model manager tests,
formatting, strict workspace Clippy, validation selection, and diff hygiene
pass. No browser, provider, installed-runtime, production, ingress, or release
effect occurred.

## Turn 405 | 2026-09-19

P211 checkpoint `56a0558e` wires named-profile remote browser open through the
SQLite journal. The operation persists exact session, browser, display-slot,
and handoff intent before effects, checkpoints `launch_started`,
`browser_opened`, `tab_acquired`, and `ready`, then atomically publishes
Browser Session State and the opaque handoff. Exact replay survives multiple
eligible routes without recomputing the slot. Recovery resumes durable
`Prepared` work, reuses a positively observed browser without relaunching, and
fails closed on an ambiguous launch outcome. An exact base-state precondition,
rechecked inside the immediate SQLite publish transaction, prevents an
intervening ordinary session mutation from being overwritten. Explicit-display
fresh opens retain their existing route, while an already journaled operation
continues to replay or recover. Focused browser-session tests pass 49 with two
real-browser tests intentionally ignored; all 22 service-model manager tests,
formatting, strict workspace Clippy, validation selection, and diff hygiene
pass. The branch and remote both resolve to `56a0558e`. No browser, provider,
installed-runtime, production, ingress, or release effect occurred. Automatic
ambiguous-launch reconciliation, protocol keepers, public live configuration,
capacity, shared control, remaining recovery, history, backup, and the
development cold-start matrix remain open.

## Turn 404 | 2026-09-19

P211 checkpoint `759f12e9` adds the SQLite operation-journal kernel. Exact
request replay is idempotent, owner-local generations fence stale effects,
unrelated owners advance independently, committed results replay after reopen,
and changed replay payloads fail closed. All ten store tests, formatting,
strict Clippy, and diff hygiene pass. Browser/display/handoff integration and
pending-operation recovery remain open. No runtime or provider effect occurred.

## Turn 403 | 2026-09-19

P211 checkpoint `c0ff2768` extends the SQLite schema packet with tolerant,
typed migration rejections and revisioned runtime configuration. Missing
legacy files create defaults without synthetic source history. Present inputs
are hashed and archived; invalid session/catalog data and rejected profile
fields become durable typed records while valid sibling profiles import. The
frozen capacity, deadline, cooldown, inactivity, retention, and storage limits
now seed a validated SQLite aggregate with compare-and-swap conflict handling.
The default host reads its timeout and disposable policy from that aggregate
and no longer reads the three ad hoc session/disposable environment variables.
All nine store tests, nine host tests, five cold-install tests, the source-free
workstation fixture, formatting, strict Clippy, and diff hygiene pass. The
public config mutation surface and live refresh remain open, as do provider
cleanup, keepers, capacity use, control, handoff recovery, history, backup, and
development acceptance. No runtime or provider effect occurred.

## Turn 402 | 2026-09-19

P211 checkpoint `726563fb` completes the first version 49 provider-free
implementation slice. The ordinary runtime host now uses one private SQLite
database for Browser Session State and Browser Profile Catalog persistence.
Cold install has an explicit migration phase after shutdown and is
forward-only after that boundary. Migration hashes and archives the legacy
JSON sources, atomically publishes a checkpointed database from a staged path,
and never falls back to JSON when a database is present. Only the cold-upgrade
shutdown adapter can read pre-database JSON. Store, host, shutdown,
cold-install, extracted-crate, workstation fixture, format, and strict Clippy
gates pass, including all 181 affected installer tests. The selector's CDP
architecture script remains unrunnable because it hardcodes the intentionally
disabled `.github/workflows/ci.yml`; its crate tests pass. No browser,
provider, installed-runtime, Service State, production, ingress, or release
effect occurred. Typed migration rejects, remaining SQLite domains, protocol
keepers, capacity, control leasing, handoff recovery, bounded history, backup,
and development acceptance remain open.

## Turn 401 | 2026-09-19

The operator completed a one-question-at-a-time architecture review and froze
P211 version 49. Hidden Chrome viewers, route and inventory environment
authority, the internal bootstrap switch, fragmented runtime JSON, and
rollback to the old installation are rejected. The existing runtime host will
own one SQLite authority, supervised in-process Guacamole tunnel keepers,
generation-fenced crash recovery, live user-scoped capacity and retention
settings, and the shared Desktop Services control lease. Cold upgrade is
forward-only: valid legacy fields import, rejected source is archived, owned
legacy processes and Guacamole state are removed, and the new generation
repairs forward. Development acceptance requires three cold starts plus
restart, failure, overflow, control-transfer, configuration, corruption, quota,
and residue gates. Production and external ingress remain excluded. The next
artifact is the provider-free schema and state-machine test packet; no build or
provider retry precedes it.

## Turn 400 | 2026-09-19

The operator rejected P211 version 47's direct provider-inventory adapter and
required a `just works` cold-start design. Version 48 makes one local
presentation-provider API the sole runtime authority for readiness,
allocation, release, reconciliation, and opaque-handoff resolution. Durable
intent, exact ownership, leases, and cleanup obligations belong in one
transactional service-owned store; XRDP/Xorg displays and Guacamole processes
are ephemeral observations reconstructed after cold start. Browser Session
Manager may read neither route JSON from an environment variable nor a
generated inventory file. Legacy static routes are admitted only through one
provider-boundary migration adapter and must be deleted after production
cutover acceptance. This turn changes planning only and authorizes no live
runtime effect.

## Turn 399 | 2026-09-18

P211 version 46 installed development generation
`0.28.0-30cfdf91ee18`, but its one ordinary retry again failed closed with
`browser_session_handoff_desktop_missing`. Exact close removed the test session
and browser. The remaining defect is now exact: Browser Session Manager's
refresh reads `AGENT_BROWSER_RDP_ROUTE_POOL_JSON` or the legacy two-route
adapter, while the development runtime publishes its healthy `:13` through
`:16` routes through `AGENT_BROWSER_PRESENTATION_PROVIDER_INVENTORY_PATH`.
Version 47 records the required typed adapter and ends this packet without
another build or retry. The provider remains ready, ingress remains deferred,
and production remained unchanged.

## Turn 398 | 2026-09-18

P211 version 45 repaired the bootstrap recursion and its one provider apply
succeeded: warm development displays `:13` through `:16` are ready, ingress is
still deferred, and the apply receipt proves production unchanged. The first
ordinary open then failed closed because the long-lived runtime host retained
the empty route inventory it read while bootstrap was still in progress.
Version 46 authorizes one provider-free dynamic inventory refresh before an
ordinary new browser allocation. The internal viewer bootstrap must continue
with no desktop assignment. One replacement development install and ordinary
ready-handoff retry are allowed; provider reapply and ingress publication are
not.

## Turn 397 | 2026-09-18

The operator reopened P211 for one explicit development-only retry. Current
readback confirms the ordinary handoff model is already route lookup plus exact
tab focus and browser raise/maximize; production routes A, B, and C currently
map to `:10`, `:11`, and `:12`. The remaining defect is isolated-provider
bootstrap: its internal Guacamole browser creates a development XRDP display
before that display can appear in provider inventory, so it must not request a
handoff to itself. Version 45 authorizes one typed development-only bootstrap
marker for exact provider viewer identity. Manager ownership, header
navigation, and exact cleanup remain mandatory; only handoff publication is
deferred until display observation. Ordinary opens remain fail-closed. One
provider-free red-green batch, one candidate and install, and one
deferred-ingress apply are authorized; another quarantine ends the packet.

## Turn 396 | 2026-09-18

P211's third and final bounded development-provider apply used generation
`0.28.0-fae441ed9a8f` from source `975147a3`. Install, plan, stage, and
preflight passed with production unchanged. Request `r97691` successfully
navigated route 1 with the required header and persisted its exact manager
browser, session, tab, target, and Guacamole URL, proving the routing and Fetch
handler repairs. Handoff publication then failed with
`browser_session_handoff_desktop_missing`: the internal warm-route viewer must
open before its display binding exists, while ordinary managed navigation now
requires that binding. Receipt `apply-1789784327708-12563.json` records the
terminal quarantine and production guard. Fresh readback shows no viewer
process, empty active manager browser/session maps, stopped provider
containers, closed provider ports, and the three healthy development runtime
units only. All three provider attempts are consumed; no further live retry is
authorized in this packet.

## Turn 395 | 2026-09-18

P211 installed repaired development generation `0.28.0-c40bd61ec18f`; the
first quarantine's exact Chrome residue disappeared during replacement, all
development units became ready, and production remained unchanged. The
corrected three-pass browser smoke is green through disposable manager
sessions. A second fully green provider preflight was followed by the single
post-fix apply, which quarantined at request `r152796` with
`CDP command timed out: Page.navigate`; receipt
`apply-1789783822885-83753.json` proves production unchanged and exact route-1
browser cleanup. Diagnosis found an unstarted Fetch paused-request handler in
the manager's attached session-command context. The narrow initialization
repair is green in a real-Chrome restart fixture that observes
`Remote-User: operator` at a local HTTP server and closes the exact browser.
Two of three bounded provider attempts are consumed. One final validated
candidate and apply remain; production remains excluded.

## Turn 394 | 2026-09-18

The operator authorized P211 isolated development-runtime acceptance. Candidate
`8682d4748725` installed with all development units ready and production
unchanged. The reviewed provider binding passed plan, stage, and preflight,
but its first deferred-ingress apply quarantined at request `r488783` with
`service_tab_target_unproven`; the receipt remains at
`~/.local/share/agent-browser-dev/presentation-provider/receipts/apply-1789782627538-20403.json`.
Diagnosis proved that `--headers` and the provider's redundant launch argument
forced initial navigation out of Browser Session Manager routing. A red public
routing regression now passes after admitting header-bearing managed
navigation, executing the required `Remote-User` header on the manager-owned
tab, and removing the redundant provider argument. The host-level header
fixture and provider fixture also pass. No blind provider retry occurred;
formatting, strict Clippy, changed-surface selection, a rebuilt candidate, and
replacement development install are the next gates. Production remains
unchanged and out of scope.

## Turn 393 | 2026-09-18

P211 checkpoint `3758f8df` completes repository documentation parity under
the operator-assigned primary custody. CLI help, README, the Agent Browser
skill, installation and remote-view docs, and inline workstation-install
documentation now lead with fixed cold apply, idempotent shutdown, and the
ordinary session-plus-profile remote-view path. Transaction, census, handoff,
and explicit route controls remain documented only as legacy recovery or
advanced compatibility surfaces. Compiled help readback, remote-view docs
contracts, documentation links, the production docs build, workstation
fixture suites, the 181-test focused Rust lane, formatting, and strict
workspace Clippy pass. P207 remains untouched. The installed user-scoped skill
also remains untouched. The goal-scoped planning audit passes; the repo-wide
active-plan audit remains red on pre-existing historical plan wiring and state
findings and reports no P211 finding. No shutdown, installation, browser,
provider, credential, Service State, staging, production, or release effect
occurred.

## Turn 392 | 2026-09-18

The operator assigned P211 primary write custody for the CLI help, README,
Agent Browser skill, installation docs, remote-view docs, RUNBOOK, and P211
catalog entry that previously overlapped P207. Fresh readback proved P211 and
P207 clean and synchronized at `4843c4e2` and `838b771a`. Pull request #191 is
open, draft, and clean; pull request #184 is open, draft, and conflicting.
P207's pending user-facing prose is specific to its unmerged tab-refresh
implementation. P211 will preserve that intent for later reconciliation, will
not publish it ahead of its source, and will not mutate or discard P207's
checkout. This custody transition authorizes documentation work only. It does
not authorize installed shutdown, installation, provider, credential, or
shared-runtime effects.

## Turn 391 | 2026-09-17

P211 checkpoint `83e23eb2` makes the trusted single-user shared-local profile
path recovery-first for retained identity collisions. A valid configured
profile now returns `ExplicitProfile` instead of
`existing_session_profile_identity_inconsistent`, so the wrong retained
browser cannot qualify for reuse and the requested profile can continue through
a fresh or requalified connection. Contradictory records remain intact for
diagnosis; stricter registered-capability and non-shared-local paths remain
unchanged. The exact SoyLei-shaped regression failed before the change and now
passes. Six existing-session tests, nine shared-local tests, formatting, strict
workspace Clippy, and diff hygiene pass. No installed runtime or tenant effect
occurred. Typed collision telemetry and joined launch and remote-view proof
remain open.

## Turn 390 | 2026-09-17

P211 checkpoint `839f8cf8` extends the black-box public shutdown boundary with
a populated retained-profile case. `agent-browser shutdown --json` changes an
exclusive retained session to `released`, preserves the named profile record
and a physical profile-data marker, reports zero runtime-owner and active-lease
residue, and succeeds without changes on replay. Both disposable process
fixtures, formatting, and diff hygiene pass. No installed shutdown, service,
container, browser, Service State, provider, credential, production, or
release effect occurred. Protected-claim, interrupted, and stale-sidecar
fixtures remain with cold install, restart, remote view, and documentation.

## Turn 389 | 2026-09-17

P211 checkpoint `34c6a0e3` adds a passing black-box fixture for the public
`agent-browser shutdown --json` route. The real candidate binary runs twice
against one disposable workstation whose `PATH` contains only fake `docker`
and `systemctl` commands. Both runs return the fixed six-phase receipt with
zero owned residue and no upgrade transaction fields. The fake-command log
contains only the three fixed Guacamole container names, and no uninstalled
user unit is inspected. Formatting and diff hygiene pass. No installed
shutdown, service, container, browser, Service State, provider, credential,
production, or release effect occurred. Populated and interrupted shutdown
fixtures, cold-install routing, restart, remote-view convergence, and
P207-controlled documentation remain.

## Turn 388 | 2026-09-17

P211 reconciled the integrated Service Model and advanced its source-only
shutdown path at pushed checkpoint `766cde6b`. `agent-browser shutdown` now
enters the fixed six-phase controller without transaction, admission, census,
digest, rollback, or target-selection input. Exact browser and daemon process
identities, the fixed workstation unit and Guacamole container sets, durable
authority release, transient metadata cleanup, and final residue readback are
bounded by phase deadlines. All 12 focused shutdown tests, all 118 Lease
Authority tests, the crate architecture guard, formatting, strict workspace
Clippy, and diff hygiene pass. No shutdown, install, browser, container,
Service State, provider, credential, production, or release effect occurred.
Command-level fixture coverage, cold-install routing, restart, remote-view
convergence, and P207-controlled documentation remain.

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
