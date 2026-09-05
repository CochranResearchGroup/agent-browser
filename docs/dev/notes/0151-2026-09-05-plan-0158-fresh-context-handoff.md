# Plan 0158 fresh-context handoff

Date: 2026-09-05

## Shared-view capability repair checkpoint

The forward-auth 500 remains unresolved, but the next bounded diagnostic
reproduced a separate clean-visual gate failure: an anonymous PostgreSQL shared
viewer requests its unavailable tunnel re-sharing profiles and receives 404.
The extension now declares that capability empty before sending the request,
only for the named embedded sharing viewer and current shared anonymous
identity. Full users, unknown identities and other tunnel operations keep
their original calls and failures. This is not an authentication bypass or a
retry policy change.

The focused regression failed with the recorded error before the fix and
passed afterward. The real Guacamole client also passed in a disposable
response-substitution prototype, with exact extension hash and active guard
readback. That prototype is not installed acceptance. The optimized candidate
build and exact-candidate workstation fixture passed, as did asset, provider,
connection-sharing, host-provision, VM-harness, database-durability and route
sync checks. An earlier fixture accidentally selected the old debug binary;
its disposable child was stopped, and that run is excluded from validation.

The temporary ingress trace required one controlled restart of only the
isolated ingress. Its tracer is detached, its override removed, and production
and default-development guards passed. Runtime host, backend and retained
browser were not restarted by that trace. Next is guarded publication of the
candidate and extension to `p158`, followed by installed same-handoff proof.
The earlier 500, failed visuals and distinct 404 remain preserved.

## Earlier repair investigation: forward-auth timeout, not yet repaired

The user requested repair. The original 17:27:58 HTTP 500 is now localized:
Cooper's local Traefik timed out calling the isolated dashboard's
`/api/dashboard-auth/verify` while handling Guacamole active-connection
discovery. Its access log records an empty 500 response after 27,649 ms with
no selected Guacamole upstream. Guacamole itself had authenticated the prior
token request successfully. Do not repair sharing-profile grants or replace
the retained browser based on this failure.

The failure remains intermittent. Two instrumented same-handoff diagnostics
both reached a frame. The first then observed a distinct shared-tunnel
sharing-profile 404; the second had no failed HTTP responses in its bounded
observation. These are diagnostic results, not a repaired original attempt or
visual acceptance. Separate read-only probes passed 64 sequential path checks
and 32 concurrent forward-auth checks. No code fix, timeout increase, retry
policy, runtime restart, screenshot, or external workflow was applied.

The private cleanroom evidence root now also contains
`forward-auth-timeout-extract.log`, `sharing-diagnostic-a/`,
`sharing-inventory-prelude-b/`, `auth-path-a.json`, `auth-concurrent-b.json`,
and durable copies of `sharing-diagnostic.mjs` and `auth-path-probe.mjs`.
`baselines-repair-f.json` confirms production and default development unchanged.
The missing repair prerequisite is a trace locating the delayed verification
request inside or before the isolated ingress/backend. The diagnosing-bugs
skill's red-capable reproduction gate prevents a speculative source patch.
Do not repeat the completed probes merely to accumulate green counts. Reframe
the next diagnostic around that transport boundary with private, bounded
request-stage instrumentation. No new authority is needed for ordinary
in-scope diagnosis; the gap is causal evidence, not provisioning approval.

## Elevated-authority cleanroom checkpoint

The user authorized the separately namespaced synthetic-only runtime and
reviewed ingress. That provisioning approval wait is resolved. Source commits
`02b3307d`, `415fb421`, `93042cde` and `ce38228e` are pushed: complete namespace
isolation, exact ordered host activation, exact provider database probing, and
unready first-install inventory bootstrap. Preserve the three earlier failure
receipts rather than retrying their unchanged conditions.

Namespace `p158` is installed as generation `0.28.0-7c7150b01efd`, with executable
`agent-browser-dev-p158`, separate home, sockets, units, provider containers,
database, route users, four warm displays and a six-route maximum. All seven
ports are pinned: 5148, 5149, 5150, 5151, 8193, 4923 and 56433.
Provider-required doctor and three disposable browser smokes passed. Existing
production and default-development identities and state passed repeated
non-interference readbacks. Never run an unscoped default-development command
to operate the cleanroom.

The new dashboard and capability ingress are published from Cooper inventory
revisions `1592a78` and `b58221b`. Local and public authentication checks passed;
the shared proxies and XRDP were not restarted. Authenticated Service Status
projects four presentation slots and hard maximum six without binding warnings.
The service broker selected the new synthetic durable profile and returned a
ready route-bound handoff. Scene custody accounts for eight owned profiles
(including three closed smoke profiles), five browsers, six synthetic/provider/
blank tabs and no manual browsers.

Private source receipts and exact configuration are under
`/home/ecochran76/.local/state/agent-browser/campaigns/p158/cleanroom-20260905`.
Use `environment.json`, `doctor-required-a.json`, `baselines-d.json`,
`scene-open-a.json`, `scene-inventory-b.json` and `scene-custody-a.json`.
The temporary bounded operator scripts are `/tmp/p158-cleanroom-control.mjs`
and `/tmp/p158-cleanroom-scene.mjs`. Their receipts are exclusive-create;
never rerun a mutation against an existing receipt or blindly choose a new label.

The latest local visual attempt failed before any screenshot: its durable
handoff resolver job succeeded, but the view received HTTP 500 and the journal
recorded `guacamole_connection_sharing_failed` at 17:27:58 UTC. Evidence is in
`cleanroom-20260905/visual-a/`, with job readback in `scene-jobs-a.json`.
An earlier successful DOM frame is not acceptance for this failed attempt.
Next: diagnose the exact connection-sharing load while preserving the retained
browser and using the same durable handoff. Do not open a replacement or blindly
repeat capture. The attempted Graphiti checkpoint write timed out; do not assume
the graph contains this continuation state.

The synthetic scene still needs fresh visual/marker verification and attestation,
then protected external-workflow secret rebinding, exact-source readiness and
shared-schedule calibration. Do not use the old default-development handoff or
visual attestation. W6 freeze and W7 through W10 live acceptance remain incomplete.
The Plan's final checkpoints are the current causal authority.

## Earlier resumption checkpoint

This section supersedes the repository and runtime snapshot below, which is
preserved as history. The Plan's final checkpoints contain the current causal
record. Repair commits `f953fc45` and `0829d9a0` are pushed. Development
generation `0.28.0-f472f3bc9cbe` is installed; provider-required doctor and all
three disposable launch smokes passed with production unchanged. Discovery
readback now contains three owned socket rows and zero foreign rows, compared
with seven foreign rows before repair. All three attached service processes
prove external discovery disabled.

Readiness C, workflow `33971756471`, failed both external clients. Its retained
artifacts exposed unsafe keyboard focus, blocked Guacamole textarea autofocus,
and a full-frame visual-boundary failure. Download and anchor-shutdown defects
were also repaired. Full campaign harness, focused regressions, disposable DOM
tests, Rust format/clippy, and docs build passed. W6 calibration and freeze are
still incomplete; no W7 through W10 live completion is claimed.

Do not dispatch another visual workflow using the existing attestation. The
current development inventory retains non-campaign profiles and an older
local tab. Disabling foreign discovery does not make that state synthetic-only.
No such records were deleted, masked, or silently adopted into the campaign.
A clean campaign-only runtime/presentation and reviewed ingress binding is the
proposed next isolation boundary; changing that environment requires resolving
the new binding and resource ownership before capture. Preserve the existing
development state and prior failed evidence.

Current private receipts live under
`/home/ecochran76/.local/state/agent-browser/campaigns/p158/resume-20260905`.
Use `doctor-discovery-isolated.json`, `status-discovery-isolated.json`, and
`inventory-after-isolation.json` for the installed checkpoint. External
artifacts were recovered under `readiness-c-recovered-artifacts`; their visual
boundary is failed evidence, not a valid synthetic-only receipt.

## Original handoff

Status: Goal active; campaign acceptance incomplete. This note records a
handoff and drift review, not acceptance or authorization for production work.

## Start here

Continue the user objective: execute Plan 158, using specialist agents where
useful, with the primary agent remaining the orchestrator. The latest user
steering was to review goal progress and potential drift, then write this
handoff. Do not restart research.gov fieldwork or substitute a smaller goal.

Canonical authority:
`docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md`.
Read its Objective, Frozen-Candidate Contract, work-unit table, Acceptance
Criteria, and final checkpoints. Older status tables are historical snapshots;
do not treat an old missing-driver list as current without checking the files.

The user wants actual historical failure reproduction, external-ingress human
simulation, agent-only tests, handoff correctness, dashboard performance and
left-rail accuracy, and excellent failure records for postmortem analysis.
Defer nonblocking repairs. Repair sequence-blocking defects only after sealing
the failed evidence, then resume under a distinct candidate epoch. Continue
independent cases. Passive production observation windows cannot delay repairs
or installation. Filesystem stop threshold is 90 percent.

## Verified repository state at handoff

- Workspace: `/home/ecochran76/workspace.local/agent-browser`.
- Branch: `plan/profile-permissions-and-request-provenance`.
- HEAD: `ca85f96705a55d6964c57f14b2278bdef84fb182`,
  `fix(remote-view): recover rejected Guacamole share keys`.
- Earlier execution reports this product commit pushed. Remote equality was
  not refreshed during this handoff.
- Worktree is dirty with campaign infrastructure and evidence-contract changes.
  Preserve these changes; do not reset or assume they are disposable.
- Team listing at handoff contains only `/root`. No specialist is running.
  A requested fresh review failed due to the prior model's usage limit; there
  is no completed review result from that request to rely upon.
- No new installation, test run, or external dispatch occurred during this
  handoff or the preceding drift review. Installed identity and live processes
  must be inspected before continuing runtime work.

Tracked modifications include the Plan, `package.json`,
`.github/workflows/p158-external-vantage.yml`, distributed calibration and W6
evidence enforcement, external runner retry detection, evidence source sealing,
and their tests. Use `git diff` for exact contents.

Untracked files are the retained-anchor implementation and tests:

- `scripts/lib/p158-retained-authenticated-anchor.js`
- `scripts/lib/p158-retained-authenticated-anchor-playwright.js`
- `scripts/run-p158-retained-authenticated-anchor.js`
- `scripts/test-p158-retained-authenticated-anchor.js`
- `scripts/lib/p158-retained-anchor-coordinator.js`
- `scripts/test-p158-retained-anchor-coordinator.js`
- `scripts/lib/p158-retained-anchor-live-adapter.js`
- `scripts/lib/p158-retained-anchor-github-provider.js`
- `scripts/run-p158-retained-anchor-live.js`
- `scripts/test-p158-retained-anchor-live-adapter.js`

## Progress and evidence limits

The Plan records W1 through W5 infrastructure complete, multiple development
installations, and successful five-surface journal calibration. W6 is still
cycling through external readiness and calibration repairs. Live completion of
W7 through W9 and W10 final analysis is not established by this handoff.

Read the Plan section `Clean Readiness And First Full C01 Findings`: workflow
33933739849 accompanied a full 20-minute attempt with 500 local observations.
It exposed real gateway timeouts, oversized status polling, Guacamole sharing
failures, clock-accounting mistakes, and missing runtime logging. Subsequent
sections document capacity/logging repairs and installed journal calibration.
These were substantive discoveries, not merely synthetic test improvements.

The newest product repair addresses restricted Guacamole keys invalidated when
their source viewer disappears. The installed extension sends an allowlisted
cross-origin signal; the parent rejects the stale iframe and performs bounded
fresh election. The claim lifetime increased to 30 seconds. Product recovery
must remain visible as a retry in campaign results; recovery cannot erase the
initial failure. Refer to commit ca85f967 and the Plan's final sections.

Earlier execution recorded passing dashboard/docs builds, focused sharing and
Guacamole asset tests, Rust fmt/clippy and focused claim tests. It also recorded
two complete `pnpm test:p158-harness` passes before the newest live adapter was
finished. These are prior results, not a new verification of the current tree.
The phrase "54 cases and 894 scheduled attempts" describes provider-free
harness coverage and must not be presented as 894 executed live attempts.

The specialist last reported that core anchor/coordinator tests passed and the
new live-adapter test stopped at a missing `readFileSync` import. That import
is now present in the file, verified during handoff. A successful final rerun
and full harness result for the latest adapter have not been recovered.

## Drift assessment that must steer continuation

The prior assistant understated remaining work by saying only an import was
left. That described one test, not Plan 158 completion.

The central drift is repeated pursuit of clean calibration turning a diagnostic
campaign into an expanding repair project. Additional review, receipt, and
orchestration layers must be justified by an actual acceptance requirement or
an observed sequence blocker. Test volume and implementation volume do not
demonstrate advancement of live acceptance.

The retained anchor deserves explicit adjudication before the next dispatch.
It holds one authenticated viewer through both external clients' reconnects.
That can provide a declared stable-primary fixture, but it also changes the
conditions that exposed the ordinary reconnect failure. An anchored pass is
not evidence of anchor-free reliability. Preserve anchor-free failures and
coverage; do not silently make an extra persistent viewer a user requirement.
This is an unresolved methodological concern, not a verified new code defect.

The corrective direction is a short execution replan within the existing
objective: identify remaining live cases, their actual prerequisites, and the
evidence needed for terminal results; continue independent cases; repair only
defects that demonstrably prevent the remaining sequence. No new broad
architecture or perpetual review cycle is warranted by current evidence.

## Immediate next actions

1. Reconcile the Plan, dirty files, installed candidate, and retained artifacts.
   Build a compact current ledger of live case states and missing acceptance
   evidence. Do not derive completion percentages from document length or tests.
2. Decide the retained anchor's declared test role and scope any admission gate
   accordingly. Keep the user-facing reconnect failure visible in the results.
3. Finish verification of the existing adapter changes. Start with
   `pnpm test:p158-retained-anchor-live-adapter`,
   `pnpm test:p158-retained-authenticated-anchor`,
   `pnpm test:p158-retained-anchor-coordinator`,
   `pnpm test:p158-external-vantage-runner`, and
   `pnpm test:p158-evidence-collector`. After actual fixes, run the integrated
   `pnpm test:p158-harness` once for the final tree, plus selected validation
   and `git diff --check`. Do not launch another open-ended review loop.
4. Inspect specific outstanding adapter concerns before relying on it: existing
   output directories must not supply stale ready/final receipts; child startup
   or exit failures must be observed; workflow identity and failure diagnostics
   must survive timeout or partial artifact download; commit matching must bind
   the actual source being executed. These are review leads, not established
   failures. Scope fixes to reproducible acceptance risks.
5. Integrate the coherent campaign slice, then build/install and verify the
   development candidate if needed for the next live cases. Continue the
   original campaign toward terminal results and W10, retaining failures and
   exact blocked dependencies instead of waiting for universal green behavior.

The current adapter command is `pnpm p158:retained-anchor-live`. Its documented
inputs and lifecycle are in the final Plan section. It starts an anchor,
dispatches the manual workflow, identifies a run by campaign ID/commit/branch,
observes terminal status, downloads receipts, closes the anchor, and emits an
aggregate. It is not yet proven against the live workflow in its current form.

## Runtime and validation boundaries

Use pnpm and `scripts/ci/cargo-safe.sh` for compiling Cargo commands on WSL.
The default is eight build jobs; two admitted Cargo invocations is a separate
limit. Do not reduce build jobs to two merely because of invocation concurrency.

Use `pnpm build:development-candidate` and development-runtime publication
commands. Plan 158 excludes production runtime replacement and tenant effects.
Before provider mutation, read current runtime policies and perform provider
plan, stage, and preflight. The public HTTPS origin and immutable Cooper ingress
revision must be explicitly supplied together for staging/mutation. Provider
apply uses the documented deferred-ingress flow. Verify that Guacamole actually
loads the revised extension, then provider-required doctor and three disposable
launch smokes. Reopen the same durable handoff through the supported mechanism
if installation closed its target; do not dispatch against a closed target.

External evidence must use the manual `p158-external-vantage.yml` workflow and
its protected environment secrets. Calibration starts at least 30 minutes in
the future and binds both external clients to one 20-minute schedule. Preserve
actual failed workflow artifacts. Never print handoff URLs, credentials,
tokens, browser content, or raw private runtime state in tracked notes.

## Suggested skills and context recovery

- `graphiti-discovery` for narrowly scoped prior decisions. The last query of
  `agent_browser_main` was healthy but returned older history, not useful new
  Plan 158 evidence. Current repo artifacts outrank that recall.
- `agent-browser-service` for service/runtime operations and
  `cooper-service-ingress` if ingress work is actually needed.
- `diagnosing-bugs` for a reproduced sequence blocker; relevant repo policy
  selection before implementation or publication.

Use CodeGraph for structural source discovery and native reads for known files,
literal logs, and policy documents. Delegate bounded execution or validation
tasks under the user's standing authorization; the primary owns drift
adjudication, integration, and completion claims. Keep the goal active until
the original acceptance criteria and final analysis have evidence.
