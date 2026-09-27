# Runbook

## Current P219 status | 2026-09-27 M3-P2A source-qualified

[Plan 0219 version 17](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md#fresh-context-continuation-after-m2b)
is OPEN. The operator resumed the amended continuation with a cumulative
1,000,000-token ceiling before stop and checkpoint, including a 200,000-token
closeout reserve and an 800,000-token implementation stop. Goal-service thread
`01a0e3f7-adbb-7312-b748-ce5a462ccd90` reported 141,869 tokens used at startup
but no remaining-token field. The read-only audit preserved successful M2B subproofs and found a
desktop error receipt reported as outer success, remaining ordinary desktop
JSON dependencies, and policy/budget, ledger, shared-helper, candidate and retry
provenance gaps. The linked continuation owns findings A01–A06 and their exits.
A01 is source-qualified: failed desktop receipts now produce truthful outer,
job, event, and terminal failures with uncertain-effect recourse and redacted
diagnostic preservation. A02 is also source-qualified: managed capture,
ready-handoff lookup, and interaction replay now use Browser Runtime SQLite,
with one-time fail-closed import and archival of the former JSON ledger. A03–A06
are reconciled with the current budget, scoped helper/binary identities, ledger
rows, and retained first-failure disposition. M3-P1 is now the active bounded
provider-free packet: pure eager/lazy recovery decisions plus SQLite-serialized
singular admission, exact old-browser unusability proof, and bounded retry.
It has no browser, provider, route, display, publication, or production effect.
M3-P1 is source-qualified: 276 service-model and 37 browser-session-store tests
pass, including a four-connection race with one SQLite admission winner.
Source checkpoint `6000b9fd` is clean. The goal service reports 712,314 tokens
used. M3-P2 is planned but unstarted: wire the SQLite admission fence into the
provider-free host recovery effect, add a fenced success/reset transition, and
inject values from the existing recovery configuration. No push, publication,
install, merge, or runtime effect occurred.
M3-P2A is source-qualified at `53c66ce9`: exact success is generation-fenced,
restart-durable, resets attempts only after success, and advances generation for
the next episode. The affected complete suites pass 277 and 38 tests; strict
Clippy and formatting pass. Goal usage is 753,564 and implementation is stopped.
Configuration translation and actual host fence consumption remain unstarted.

A01 validation passed 40 desktop-interaction tests, 273 service-model tests,
18 Desktop Services tests, the focused dispatch/redaction/terminal regressions,
formatting, strict workspace Clippy, architecture ownership, route-confusion
gates, and diff hygiene. The selected CDP live smoke remains unqualified: two
attempts failed before browser launch and a preserved debug receipt reports a
missing temporary-home runtime SQLite database. The Rust command selected zero
tests and is not evidence. No candidate was published and no provider effect
was attempted.

A02 validation passes 43 desktop-interaction tests, 21 desktop-capture tests,
50 browser-session-store tests, 18 Desktop Services tests, architecture detector
self-tests, formatting, and strict workspace Clippy. The reconciled coverage
ledger is nine pass, 28 partial, three fail, and five missing. G14 is partial for
provider-free eager/lazy scheduling, and G15 adds exact old-browser proof,
bounded retry/backoff, restart persistence, and one concurrent admission winner.
Actual host effect consumption remains open. Whole-product G04/P03/P05 closure
remains open outside the desktop cut.
M3-P1 can improve source evidence for G14/G15 but cannot accept installed or
joined recovery by itself.

Audit baseline: clean `00935477`, 55 commits ahead of the unchanged remote
`8bb9518e`; development binary digest `ac3c1daed8ab` matches its recorded full
identity. The prior audit coverage readback was 9/26/3/7; architecture reports four pass, four
detector gaps, eleven unverified. No production repair, runtime publication,
push, merge, or release is part of this documentation continuation.
Documentation validation: policy wiring, all three changed Markdown files'
local links, selector, and diff hygiene pass. Planning audit reports 287
findings on other plan files, none on P219; no repository-wide clean claim.

## Prior P219 status | 2026-09-27 M2B accepted

[Plan 0219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
version 9 is `OPEN`: M2B is installed and accepted, while M3 through M5 remain
unstarted. Custody remains on `platform/p211-simple-cold-upgrade`, draft PR
#191, and work items #181/#183/#195. Production, merge, release, and worktree
removal remain excluded.

The selected isolated development generation is `0.28.0-ac3c1daed8ab`, built
from the working tree based on `1d940440eeb3e0aae93faba739ed6c89e0af5f96`.
Every development doctor check passes, helper version 11 is installed, four
warm provider displays are ready, and production remains
`0.28.0-b589b318c530-c0c0977896a8`.

M2B passed shared-profile Alice/Bob identity isolation, exact addressed-command
activity, failed-command non-refresh, authenticated viewer lifecycle and
control transfer, stale-controller rejection, guarded desktop input, Alice-first
cleanup, Bob-final process termination, and a separate disposable expiry case.
Two runtime-host replacement cycles preserved the same logical browser,
sessions, route slot, and opaque handoff IDs while launching only one replacement
browser each time. Both original public handoffs resolved to their fresh exact
targets with `operatorVisible.state=ready`, `uxState=connected`, and visible
Guacamole pixels. Pre-restart page markers were absent after recovery, and the
navigation ledger advanced only for four explicit reopens. Final cleanup left no
profile browser process.

The 45-row grilling ledger now totals nine pass, 26 partial, three fail, and
seven missing. G11, G24, G31, G35, G43, G44, and G45 are newly accepted; G36
is partial pending delayed disposable-profile deletion and quota cleanup.

M2B closeout validation is complete. Rust format, strict Clippy, every retained
green comprehensive compartment, the repaired 40-test actions compartment, the
118-test Lease Authority rerun, dashboard and viewer contracts, docs builds,
service parity and types, route-confusion gates, workstation/provider fixtures,
coverage validation, policy wiring, documentation links, diff hygiene, final
doctor, skill sync, and fresh development process census pass. M3 recovery and
retention is the next bounded milestone and is not started. The goal service still shows a stale blocked row
from the former helper gate; the operator explicitly resumed this successor
plan and set the active continuation ceiling to 2,000,000 tokens.

Nonblocking UI direction remains unchanged: adopt the Guacamole interaction
approach from `../remote-view` and replace the large persistent yellow banners
with compact dismissible notices.

## Prior P219 checkpoint | 2026-09-27

[Plan 0219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
version 8 is `OPEN`. The operator expanded the active continuation ceiling to
2,000,000 tokens; production, staging, public ingress, merge, release, and worktree
removal remain excluded. Custody stays on
`platform/p211-simple-cold-upgrade`, draft PR #191, work items #181/#183/#195.
The current source checkpoint is `375cdfbe`; the named-retention repair is
`d7ec8d2f`. The branch is 52 commits ahead and zero behind its remote.

Current installed development identity is generation
`0.28.0-57c51d356866`, executable SHA-256
`57c51d3568663fe9f50bd14bab69bc183c71b37b819decd22d64ee62c8f35cf2`.
The optimized build, development installation, and the three-iteration
browser-launch smoke passed.
Production remained unchanged and the development skill is current.

The operator installed helper version 10, including exact primary channel
socket reclamation. Four subsequent applies failed closed and retained
production identity: `apply-1790478847005-74899.json` exposed the historical
display backlog; `apply-1790479449908-49709.json` exposed a quarantined `dev-2`
stop that had to resume before sweeping; `apply-1790479918735-97420.json`
exposed the short X-server teardown race; and
`apply-1790480067753-19562.json` advanced through displays 10 through 19 before
`.X20-lock` named a live numeric PID.

The `:20` PID is not the old route X server. Kernel readback proves PID 88087
is now a Chrome thread under UID 1000, while the exact lock inode is owned by
route UID 1004. Commits `662ab5d2`, `d4e75b70`, and `a11be934` add the bounded
sweep, retained-stop recovery, and exact PID teardown wait. Commit `375cdfbe`
adds helper version 11, which accepts numeric PID reuse only when the kernel
proves a different UID from the route user and rechecks that fact immediately
before exact inode deletion. Same-route UID, unreadable identity, a live XRDP
session, a live display socket, or an active channel socket still fails closed.

The v11 helper and provider regressions, focused Rust contracts, formatting,
strict workspace Clippy, source-free installer, host provisioning, fresh-VM,
Guacamole asset, PostgreSQL durability, and route-user synchronization fixtures
pass. The new user-scoped candidate is installed. The root-owned helper remains
version 10, so provider preflight now fails the explicit
`privileged-helper-foreign-pid-reuse` capability gate until one interactive
sudo installation. The provider is quarantined and stopped; all six keeper
records are absent and no development XRDP route process remains.

Authenticated access through an opaque `/remote-view/<handoff-id>` URL and real
synthetic browser pixels passed. The development admin credential was aligned
with the existing live credential at the operator's direction without recording
the secret; the live auth file stayed byte-identical. Automated pointer,
keyboard, and scroll effects were not established.

The named-session retention defect is repaired and source-qualified at
`d7ec8d2f`. The manager now applies `sessionIdleTimeoutMs` only to
manager-allocated disposable profiles. Exact named-profile sessions and their
opaque handoffs have no default idle expiry, while explicit close remains
terminal. Host restart normalizes and persists legacy finite named-session
deadlines. The installed generation contains this repair, but G12/G36 are not
accepted at the installed boundary because provider quarantine prevents the
same-handoff retention replay. G42 still requires installed exact-session
refresh. G45 is source-qualified as partial: the current authenticated
Guacamole primary is re-observed on connect and heartbeat, authority expires
after 15 seconds with an inclusive boundary, and heartbeat or disconnect does
not refresh the logical session. A single operator report that the handoff was
absent from SQLite was not reproduced: the exact row existed and authenticated
retry succeeded.

Acceptance remains partial:

| Requirement | Source or installed evidence | Current state | Missing proof |
| --- | --- | --- | --- |
| M1B authority closure | Source cuts and focused gates through `bdf695db`; candidate `0.28.0-57c51d356866` installed | complete for this checkpoint | Installed lifecycle acceptance remains in M2B. |
| M2A provider and viewer | Historical authenticated Guacamole, opaque handoff, and real pixels; four exact recovery receipts retained | partial | Install helper version 11, recover four routes, and revalidate pixels and input. |
| M2B Alice/Bob | Named profile and durable handoff exercised | incomplete | Correct retention, joined Alice/Bob identities and effects, input, cleanup, restart recovery, and fresh process census on one candidate. |
| G12/G36 retention | Source regressions prove retained named handoffs, bounded disposable expiry, and restart normalization at `d7ec8d2f` | partial | Installed retention and same-handoff replay on a frozen candidate. |
| G35/G45 viewer authority | Exact-primary observation, bounded viewer TTL, inclusive expiry, disconnect, current-control revalidation, and session/viewer separation pass at `bdf695db` | partial | Installed viewer lifecycle, control transfer, eager recovery, and session retention after provider recovery. |
| Goal control row | Thread `01a0dfee-e1ab-79e0-b34b-b9fa80a58d0c` is `active` | current | Its Plan 211 wording routes through active successor Plan 0219; the operator set a 2,000,000-token continuation cap. |

The bounded retention repair is complete. Its red regression and green focused
receipts, the complete 271-test Service Model lane, 32 host/navigation tests,
formatting, and strict workspace Clippy are recorded in Plan 0219. The
selector-required workstation and Guacamole fixtures, docs build,
remote-view documentation contract, architecture report, and coverage-ledger
validator also pass. Four unrelated `browser_session_authority` failures remain
in a broader name-filtered Rust run and do not invalidate the independently
green changed surfaces. The development skill is synchronized with the
candidate; the shared production skill and production runtime remain unchanged.

The live-viewer boundary repair is committed at `bdf695db`. A heartbeat at the
exact expiry instant is rejected as `live_viewer_lease_inactive`, and viewer
heartbeat and disconnect leave Browser Session Manager state unchanged. Four
focused live-viewer tests passed in receipt
`20260927T021202Z-a0456af8a0fa`; all 11 desktop-control tests passed in
`20260927T021442Z-dbae322f207f`; formatting passed in
`20260927T021442Z-42874926b5c2`; and strict workspace Clippy passed without
warnings in `20260927T021455Z-db18b3da9800`. P19 now passes its deterministic
architecture gate. The ledger totals two pass, 31 partial, three fail, and nine
missing.

The ignored same-profile browser fixture is stale diagnostic evidence. It
first failed on a missing disposable SQLite database in receipt
`20260927T020349Z-39f194809aba`; a temporary migration advanced it to the
correct `presentation_keeper_unavailable` boundary in
`20260927T020645Z-3f7e82c03caa`. The temporary fixture change was reverted. Do
not restore a JSON route fallback.

Next gate: install the version 11 privileged helper through interactive sudo,
verify `acceptsProvablyForeignPidReuse=true`, and rerun the exact development
provider plan, stage, preflight, and one deferred-ingress recovery apply. Do not
retry the rejected credential file or broaden cleanup. After four-route readiness,
replay installed named retention
and the remaining M2B Alice/Bob workflow. Do not start M3, M4, or M5.

Nonblocking future UI direction: adopt the Guacamole interaction approach from
the sibling `../remote-view` project and make warning banners compact and
dismissible. This does not expand the retention repair packet.

Progress classification: refreshed frozen development candidate, exact provider
blocker removal in source, and source-qualified live-viewer separation. M2A and
M2B remain incomplete; no G-row is promoted to installed pass by this
checkpoint.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md) and [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)

## History

[The P218 archive](RUNBOOK-history-2026-09-26-through-p218.md) preserves the
previous runbook, Turns 418 through 450, P218 source checkpoints, failures,
prior stop instructions, and links to earlier archives. Plan 0219 version 4
preserves the superseded September 26 M2A checkpoint and its receipt.
