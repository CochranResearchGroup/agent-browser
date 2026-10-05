# Plan 0081: Guacamole Route-Pool State Reconciliation

Date: 2026-07-28
Status: Complete
Lane: P81
Predecessors: P79 and P80
Source incident: Plan 0012 post-reboot route-open selected a legacy route

## Goal

Make normal local-runtime convergence project the readiness-verified
Guacamole route pool into retained service state, then prove without launching
a browser that stable entry `guacamole-rdp-a` resolves to current route
`guacamole:1` on display `:11`.

## Causal Diagnosis

- Route readiness and remote-view doctor report current canonical routes
  `guacamole:1` on `:11` and `guacamole:2` on `:12`.
- Retained service state still maps stable entries `guacamole-rdp-a/b` to
  legacy routes `guacamole:4/5`.
- `remote-view open` correctly selects from retained route-pool state, so the
  stale definition caused Plan 0012 to choose `guacamole:4` on `:10`.
- Local convergence ran route readiness but discarded its structured output.
  The subsequent `service reconcile` health pass did not replace route
  definitions.
- The failed acquisition rolled back its lease before browser launch. No
  authentication probe, source canary, or request ID was consumed.

The defect is missing authoritative-state projection between route readiness
and retained service reconciliation. It is not an authentication or browser
launch failure.

## Scope

- let CLI `service reconcile` accept a readiness-verified authoritative route
  pool;
- replace stale inactive route definitions by stable entry ID;
- preserve active same-route allocation state and fail closed on an active
  conflicting route;
- return structured `routePoolRefresh` evidence;
- make the local-runtime interlock pass successful route-readiness output into
  reconciliation;
- align response schema, generated client declarations, CLI help, README,
  docs site, agent skill, and inline documentation;
- install the repaired candidate, reconcile current retained state, and run a
  no-launch route selection dry run;
- record and push repository-native closeout evidence.

## Non-Goals

- opening a browser or source site;
- probing authentication, cookies, credentials, checkpoints, or CAPTCHA;
- running social-source canaries;
- consuming any Plan 0012 request ID;
- authorizing or executing another Plan 0012 attempt;
- changing Guacamole database rows, XRDP sessions, or route displays;
- formal release preparation.

## Execution Packets

### Packet A | Red reproduction and causal trace

Status: completed

Add focused parser and state-reconciliation tests. Prove they fail because
`service reconcile` cannot accept or persist an authoritative route pool.
Trace acquisition from stable route-pool selection through retained state.

### Packet B | Bounded source repair

Status: completed

Implement guarded route-definition refresh, wire convergence to route
readiness, align public contracts, and run focused deterministic tests.

### Packet C | Installed no-launch proof

Status: completed

Publish the local candidate, back up retained service state, run the normal
convergence interlock once, and verify:

- retained `guacamole-rdp-a` is `guacamole:1` on `:11`;
- retained `guacamole-rdp-b` is `guacamole:2` on `:12`;
- both entries are available with no allocation;
- `remote-view open ... --route-pool-entry-id guacamole-rdp-a --dry-run`
  selects `guacamole:1` and `:11`;
- no browser or authentication action ran.

### Packet D | Validation and closeout

Status: completed

Run Rust format, lint, focused tests, service contract/client gates, local
convergence fixtures, docs build, and selected validation. Update the roadmap,
runbook, source incident checkpoint, and push clean synchronized repositories.

## Hard Stops

- never replace a retained route whose conflicting allocation is still active;
- stop if readiness is not successful or does not contain a route-pool array;
- stop if live canonical route IDs or displays differ from `1/:11` and
  `2/:12`;
- stop before any browser launch, authentication probe, source canary, or
  request-ID consumption;
- leave the next Plan 0012 attempt behind a fresh explicit authorization gate.

## Acceptance Criteria

- focused tests prove stale inactive definitions refresh and active conflicts
  remain unchanged;
- repository merge tests prove a concurrent checkout or release survives an
  older reconciliation snapshot;
- the interlock consumes only successful readiness JSON;
- the service response schema and generated client expose
  `routePoolRefresh`;
- installed retained state contains both canonical routes and displays;
- a no-launch dry run selects route A as `guacamole:1` on `:11`;
- install doctor, remote-view doctor, route readiness, Git state, and timer
  state are reported separately;
- Plan 0012 remains unexecuted and explicitly unauthorized.

## Completion Evidence

- Readiness reports route A as `guacamole:1` on `:11` and route B as
  `guacamole:2` on `:12`.
- The installed convergence pass refreshed both stable retained entries to
  those definitions. Both remain `available` with no allocation.
- The next scheduled interlock pass completed successfully with no failed
  step, proving the installed recurring path consumes the new contract.
- A normal stable-entry dry run selected route A as `guacamole:1`, connection
  `1`, and display `:11`, with browser launch, route checkout, and tab opening
  all false.
- Install doctor reports six converged runtimes, remote-view doctor is ready,
  and the installed binary remains version `0.27.0`.
- The validation note
  `docs/dev/notes/2026-07-28-guacamole-route-pool-state-reconciliation.md`
  records the exact backup, installed hash, gates, and no-launch evidence.
- No browser, source authentication probe, source canary, or Plan 0012 request
  ran. Fresh authorization is still required for the next Plan 0012 attempt.
