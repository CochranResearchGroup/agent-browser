# AuraCall Gemini Development Runtime Remote-View Handoff

Date: 2026-08-30

Status: BLOCKED ON DEVELOPMENT ROUTE-POOL PROJECTION

Repository: `/home/ecochran76/workspace.local/agent-browser`

Branch and source checkpoint: `main` at
`d9697146e5c34eb781199356411e819ebe083ba9`, equal to `origin/main` when this
note was written.

## Purpose

Continue the bounded Agent Browser investigation that began when production
`remote_view_open` returned `service_state_lock_timeout: process mutation
lock` while opening AuraCall's existing Gemini browser profile.

The immediate question is no longer whether Chromium or CDP works. Both the
production and development runtimes launched the browser successfully. The
remaining development blocker is an inconsistent route-pool projection that
selects one display during planning and rejects the same route during live
checkout.

This is an Agent Browser runtime and presentation investigation. It is not a
Gemini authentication, CAPTCHA, or AuraCall provider-behavior investigation.
No current CAPTCHA was observed.

## Authority Order

1. Fresh OS process, Service Status, exact job, and trace readback.
2. Current source at the checkpoint above, while preserving the pre-existing
   dirty worktree listed below.
3. `docs/dev/plans/0142-2026-08-29-service-state-concurrency-and-client-recourse-reliability-plan.md`
   and its acceptance notes for lock-timeout recourse.
4. `docs/dev/notes/2026-07-28-guacamole-route-pool-state-reconciliation.md`
   for the prior authoritative route-readiness projection repair.
5. `docs/dev/notes/0125-2026-08-23-development-runtime-isolation-acceptance.md`
   for development-runtime isolation boundaries.
6. Graphiti group `agent_browser_main` as advisory discovery only. Verify any
   retrieved claim against current source or live receipts.

## Production Incident And Closeout

The selected production generation was
`0.28.0-2851117fd877-04e7cf4c8b54`.

Two production presentation requests returned the legacy unstructured lock
timeout:

- job `r763680`, submitted `2026-08-30T12:43:13.919285657Z`; and
- job `r751572`, submitted `2026-08-30T12:45:14.924134591Z`.

The first request nevertheless launched Chromium with browser id
`session:auracall-auracall-gemini-pro-gemini`, session
`auracall-auracall-gemini-pro-gemini`, and PID `4999`. This is direct evidence
that a lock timeout can leave the runtime effect uncertain.

The exact owner-aware close command was:

```bash
agent-browser --json --session auracall-auracall-gemini-pro-gemini close
```

It also returned `service_state_lock_timeout: process mutation lock`, but the
effect completed. Fresh reconciliation proved:

- PID `4999` and every process using the AuraCall Gemini user-data directory
  were absent;
- the production profile allocation had zero browser ids and was available;
- trace event `event-e2cf76a2-89b1-4d48-87f0-499576156c3e` recorded a polite
  operator-requested shutdown at `2026-08-30T13:04:56.862107941Z`;
- `politeCloseSucceeded=true`; and
- no force kill was attempted.

The direct close did not return a service job id. Do not invent one.

## Development Runtime Identity And Readiness

The development command and selected generation were:

```text
/home/ecochran76/.local/bin/agent-browser-dev
0.28.0-dee0c6c0138d
```

The repository skill was synchronized only into the development pseudo-home:

```bash
pnpm development-runtime:skill-sync
```

`pnpm development-runtime:doctor` then passed every runtime, unit, executable,
port, manifest, ingress, skill, and isolated presentation-provider check. The
provider reported six configured routes. Development Service Status reported:

- worker `Ready`;
- queue depth `0`;
- non-null `presentationCapacity`;
- four `warm_idle` slots;
- no active Service State lock holder;
- `processTimeouts=0`; and
- `processPoisonRecoveries=0`.

The development profile catalog intentionally did not contain AuraCall's
production registration. The experiment therefore used the exact
operator-supplied profile directory:

```text
/home/ecochran76/.auracall/browser-profiles/gemini-stealthcdp/gemini
```

The no-launch browser-capability preflight succeeded as job `r685121`. It
selected the development-pinned `/opt/google/chrome/chrome` executable because
an explicit profile path was supplied.

## Development Attempts

Both attempts used browser id
`session:auracall-gemini-dev-20260830`, session
`auracall-gemini-dev-20260830`, the exact AuraCall profile directory, Gemini's
application URL, RDP gateway presentation, manual attached desktop input, and
duplicate-process rejection.

### Route 1

The dry-run succeeded as job `r604732` and planned:

- route-pool entry and route `development-route-1`;
- display allocation `remote-view-display:12`; and
- launch display `:12`.

The live request failed as job `r270572`:

```text
route_pool_entry_unavailable: route pool entry 'development-route-1' is not available for checkout
```

The live diagnostic showed two conflicting projections for the same logical
route:

- base entry `development-route-1` selected display `:12` and changed to
  `pending`; while
- warm-slot entry `development-slot-1` also referenced route id
  `development-route-1`, but referenced `development-display-1` on `:13`.

The failure rolled back the lease and returned
`cleanup.state=closed_new_browser`.

### Route 2

After confirming no process, browser record, lock holder, or queued work, the
typed diagnostic's route-2 dry-run was executed. It succeeded as job `r515941`
and planned:

- route-pool entry and route `development-route-2`;
- display allocation `remote-view-display:13`; and
- launch display `:13`.

One live request then failed as job `r604926` with the same typed error. Its
diagnostic again showed conflicting projections:

- base entry `development-route-2` selected display `:13` and changed to
  `pending`; while
- warm-slot entry `development-slot-2` also referenced route id
  `development-route-2`, but referenced `development-display-2` on `:14`.

The failure again rolled back the lease and returned
`cleanup.state=closed_new_browser`.

Job `r604926` retains structured recourse:

```text
effectState=effect_uncertain
retryDisposition=inspect_before_retry
hardStops=[blind_retry]
recommendedAction=inspect_failure
reuseAllowed=false
```

## Final Current-State Readback

At `2026-08-30T08:08:06-05:00`:

- no OS process used the AuraCall Gemini profile directory;
- development Service Status contained no matching browser for
  `auracall-gemini-dev-20260830`;
- development worker state was `Ready` with queue depth `0`;
- no Service State lock was active;
- development lock counters still showed zero process timeouts; and
- four warm presentation slots remained idle.

No durable `/remote-view/<handoff-id>` was produced. Raw Guacamole route URLs
are intentionally omitted because they are not operator handoffs.

## Assessment

The development runtime did not reproduce the production process-mutation
lock timeout. It completed job persistence, returned structured failures, and
closed each newly launched browser during compensation.

The experiment did expose a separate route-authority defect or stale
projection boundary. Planning consumes the base `development-route-*` rows,
while the authoritative warm capacity projection also exposes
`development-slot-*` rows that reuse each logical route id with a different
display allocation and display name. Live checkout then rejects the route it
just moved to `pending` because the persisted and authoritative projections no
longer agree.

The typed failure is emitted in `cli/src/native/remote_view.rs` near the
`route_pool_entry_unavailable` branch. The diagnostic assembly for
`matchingRoutePoolEntries` and `availableRoutePoolEntries` is in the same file.
Use CodeGraph before changing the merge, reconciliation, planning, or checkout
path; do not infer the source of the duplicate projection from this live
receipt alone.

## Hard Stops

- Do not retry jobs `r270572` or `r604926`.
- Do not cycle through routes 3 and 4. Two matching failures establish the
  route-projection pattern.
- Do not launch a second browser lane on the AuraCall Gemini profile.
- Before any future launch, re-read the exact process table, profile lock,
  development Service Status, and access plan.
- On `service_state_lock_timeout`, stop and inspect the exact job, trace, and
  current browser and route state. Retry only when a current structured result
  explicitly permits it.
- Do not kill, prune, garbage-collect, scale, or repair provider state merely
  to make the smoke pass.
- Do not run development provider mutation without the required
  `development-runtime:provider-plan`, `development-runtime:provider-stage`,
  and `development-runtime:provider-preflight` sequence and explicit authority
  for the apply.
- Do not expose raw Guacamole, route-binding, local embed, dashboard embed, or
  health URLs. Require `operatorVisible.state=ready` before sharing a durable
  handoff.
- Do not automate CAPTCHA or human-verification controls. None is currently
  evidenced in this incident.

## One Bounded Next Packet

1. Re-anchor `HEAD`, `origin/main`, the worktree, selected development
   generation, development doctor, and current Service Status. Preserve the
   dirty files listed below.
2. Use CodeGraph context and exploration for the acquisition planner, route
   pool refresh or merge, development provider capacity projection, and route
   checkout path. Start from `cli/src/native/remote_view.rs` and the stable-id
   refresh precedent in the P81 note.
3. Add a provider-free fixture that reproduces two rows sharing one logical
   route id while disagreeing on stable entry id, display allocation, and
   display name. Prove the intended authority order before editing live state.
4. Repair the narrowest source boundary so planning and checkout consume one
   coherent route identity and display binding. Do not weaken the live-owner,
   controller-lease, or display-agreement interlocks.
5. Run focused Rust tests through `scripts/ci/cargo-safe.sh`, formatting,
   strict Clippy, the relevant development-runtime fixtures, and documentation
   validation if operator semantics change.
6. Publish and doctor a new isolated development generation only after source
   validation passes. Confirm non-null presentation capacity and zero
   production identity drift.
7. Request fresh live authority before one new Gemini remote-view smoke. A
   successful smoke must return `operatorVisible.state=ready`, a durable
   handoff URL, exact route/display/browser agreement, and a clean owner-aware
   close receipt with no residual profile process.

## Pre-Existing Dirty Worktree

The following modifications existed before this note and belong to another
active slice. Preserve and reconcile them rather than overwriting or folding
them into this handoff:

```text
README.md
cli/src/output.rs
cli/src/workstation_install.rs
docs/dev/notes/0148-2026-08-29-plan-0137-slice-j-lock-bootstrap-checkpoint.md
docs/dev/plans/0137-2026-08-28-profile-acquisition-recovery-and-lifecycle-reliability-plan.md
docs/src/app/installation/page.mdx
skills/agent-browser/SKILL.md
```

Only this `0149` note was added by the handoff-writing slice. No source,
runtime profile, provider configuration, or Plan 0137 artifact was changed by
the note itself.

## Suggested Skills

- `agent-browser-service` for access-plan, lifecycle, presentation, and typed
  failure handling.
- `graphiti-discovery` for the required advisory lookup in
  `agent_browser_main`.
- `codegraph-workspace` for the structural route-planning and checkout flow.
- `diagnosing-bugs` for a fixture-first root-cause packet.
