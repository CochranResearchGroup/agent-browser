# Plan 0118: Agent Operating Knowledge Closure

Date: 2026-08-21

State: ACCEPTED

Lane: P118

Source baseline: `6e179c8dcfb3d6726223325788a1652d2cf7b573`

Depends on:

- `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`
- `docs/dev/plans/0066-2026-06-28-rdp-browser-reattachment-plan.md`
- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
- `docs/dev/plans/0077-2026-07-25-profile-discovery-and-manual-browser-launch-ux-plan.md`
- `docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md`
- `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

## Goal

Give external agents one intent-first operating interface for shared browser
work. The interface must explain what the caller owns, what Agent Browser
owns, how profile and presentation decisions differ, and which evidence can
actually block a request. Publish the same decision model through packaged
skills, public documentation, MCP discovery, embedded dashboard-agent context,
CLI help, and the installed Codex skill surface.

## Revealed Gap

An agent declined to launch a BILL browser because both Guacamole routes were
checked out and install doctor reported shared-runtime cleanup issues. The
agent correctly preserved other workloads, but combined three independent
axes:

1. browser and profile acquisition;
2. optional operator presentation through RDP or Guacamole; and
3. runtime maintenance and garbage-collection health.

The current runtime demonstrated why that inference is unsafe. Both route-pool
entries were checked out, while BILL profile discovery still recommended a
new browser launch. Install doctor reported cleanup advisories with zero
readiness-impacting GC candidates. The service access plan assigned profile
freshness verification to the service and did not classify global cleanup
advisories as a BILL launch blocker.

The implementation already contains the correct ownership mechanisms, but the
agent interface is shallow. A caller must reconstruct the operating contract
from a large general skill, route records, profile recommendations, doctor
fields, and separate route-switch behavior.

## Architecture Decision

Deepen one **Agent operating guide module** at the external-agent seam.

Its interface is one short sequence:

1. declare service, agent, task, target, and account intent;
2. read the access plan;
3. request browser work through the service control plane;
4. request presentation only when a person needs it; and
5. troubleshoot the failed axis without taking over lifecycle management.

The implementation behind that interface includes profile discovery,
exclusive-profile leasing, retained-browser reuse, Guacamole and RDP route
preflight, durable handoffs, scoped doctor evidence, runtime advisories, and
reviewed cleanup. This concentrates locality in Agent Browser and gives every
caller leverage from the same ownership rules.

The new focused skill complements the general browser-command skill. It does
not duplicate the complete command catalog. Conditional Guacamole/RDP,
profile, and troubleshooting details live in focused references.

## Required Deliverables

### Packaged and installed skills

- Add `skills/agent-browser-service/SKILL.md` with a discriminating trigger for
  shared, authenticated, service-owned, remote-view, profile, route-capacity,
  and runtime-troubleshooting work.
- Add focused references for Guacamole/RDP, profile selection and management,
  and troubleshooting.
- Update the general `skills/agent-browser/SKILL.md` with the ownership and
  three-axis decision model plus a link to the focused skill when installed.
- Include the focused skill in the embedded dashboard-agent prompt inventory.
- Install both current source skills into the user-scoped Codex shared-skill
  directory and verify source-to-installed equality.

### MCP agent discovery

- Add a read-only `agent-browser://operating-guide` resource with a compact,
  structured, versioned decision model.
- Make MCP initialization direct agents to the operating guide and access plan.
- Clarify `service_access_plan`, `service_request`,
  `service_remote_view_route_preflight`, and `service_status` descriptions so
  route occupancy, scoped blockers, and maintenance advisories are not
  conflated.
- Add source tests proving resource listing, resource readback, and the critical
  ownership language.

### Public and CLI documentation

- Add an intent-first agent workflow and ownership matrix to Service Mode.
- Add route-occupancy, parking, waiting, and non-disruption guidance to the
  remote-view guide.
- Add the focused skill to the Skills page.
- Add a concise external-agent entrypoint to README and CLI help.
- Keep examples runnable and preserve durable handoff URL rules.

### Verification

- Validate both skills with the Codex skill validator.
- Run focused MCP resource and schema tests.
- Run service API/MCP parity and relevant documentation guards.
- Build the docs site.
- Run Rust formatting and strict Clippy because MCP and embedded-agent Rust
  surfaces change.
- Run `pnpm validation:select -- --base 6e179c8d` and satisfy every applicable
  selected gate.
- Verify installed skill equality and inspect the installed MCP resource from
  the selected runtime.

## Acceptance Criteria

1. An agent can determine from the focused skill that route occupancy does not
   by itself block browser acquisition.
2. An agent can determine that global doctor advisories block a request only
   when the requested action or scoped readiness evidence says they do.
3. An agent can select or reuse a profile through access-plan evidence without
   inventing a new profile to escape an active workload.
4. An agent can distinguish automation, presentation, and maintenance failures
   and report the exact blocked axis.
5. An agent can request a durable Guacamole/RDP handoff, wait for presentation
   capacity, or request an explicit route switch without killing another
   browser or sharing a provider URL.
6. MCP clients discover the same rules without reading repository files.
7. The packaged skill, embedded dashboard-agent skill inventory, public docs,
   CLI help, and installed Codex skills agree.
8. Focused tests, parity checks, docs build, Rust format, and strict Clippy pass.

## Non-goals

- Add a third Guacamole route without measured concurrent presentation demand.
- Automatically take over an active controller lease.
- Turn install doctor into the per-request access planner.
- Allow caller-directed process cleanup, profile deletion, route release, or
  browser termination.
- Change authentication state or open the BILL browser during this documentation
  and discovery slice.

## Bounds

- Implementation attempts: `1/2`.
- Review and rework cycles: `1/1`.
- Installed skill sync passes: `1/2`.
- Live browser launches: zero.
- Browser, route, profile, viewer, and runtime cleanup effects: zero.
- Existing retained workloads remain untouched.

## Current Evidence

- Source skill and installed skill differ; the installed copy lacks current
  runtime-lifecycle guidance.
- The source general skill is 2,250 lines and mixes ordinary automation,
  runtime installation, remote presentation, profile management, service
  contracts, and troubleshooting in one entrypoint.
- MCP exposes access-plan, profile, route, status, and request surfaces but no
  single operating-guide resource.
- Current route-pool state has both canonical entries checked out.
- Current install doctor reports `service_duplicate_profile_pressure` and
  `runtime_cleanup_obligations_missing`, while service resources report zero
  candidates and zero readiness-impacting candidates.
- BILL profile lookup selects `bill-soylei`, reports the profile lease as
  available, and recommends launch while separately reporting no available
  presentation route.

## Implementation Outcome

The source implementation and user-scoped Codex skill installation are
complete. The focused skill, its three references, the refreshed general
skill, the embedded dashboard-agent skill inventory, CLI help, README, public
docs, and the versioned MCP operating-guide resource now express the same
ownership model.

The source-to-installed general skill file is byte-identical. The complete
`agent-browser-service` source and installed directories are byte-identical,
and the installed focused skill passes the shared skill validator.

The release candidate binary exposes `agent-browser://operating-guide` with
schema `agent-browser.operating-guide.v1` and all three readiness axes. The
transactional live install stopped before candidate selection with transaction
`upgrade-4daef554-626f-4009-b5b5-980d175be2d6`. Its state is
`blocked_ambiguous_runtime`; admission is no longer draining, dashboard
ingress remains ready, and selected generation
`0.28.0-fb5a8ef317c2-9cf9b4f6919d` remains unchanged.

The exact rejected census rows are:

- `session:last30days-owner-repair-20260821-c07` with
  `profile_identity_mismatch`;
- `session:p0204-a06` with `required_identity_evidence_missing`; and
- `session:plan0233-qbo` with `required_identity_evidence_missing`.

No retry, forced cleanup, browser close, profile mutation, route reassignment,
or weaker census rule was used. The installed MCP resource remains absent
until those runtime identities are reconciled by a separate reviewed repair
and the transactional installer succeeds.

Subsequent repair work resolved the original three census ambiguities and
proved the source operating guide in isolated release candidates. It also
closed terminal-browser projection, historical owner aliasing, shared-host PID
joining, and stale owner to reused-profile joining defects. The latest
candidate digest is
`266024cca43fbe243cc9139c9af36a9a2fb3382e85aac1f082327273a774eb34`.

Live transaction `upgrade-7ab1af56-f2d4-43d4-9981-61bcbc30a240`
still failed before generation selection. The remaining seam is exact:
`p0204-a06` and `plan0233-qbo-owned` are ready named lanes behind the selected
single runtime host, but the census route probe resolves the legacy socket
directory instead of the selected runtime-host ingress directory. They are
therefore misclassified as fenced orphans. The fallback then looks for a
per-session daemon identity that a shared host intentionally does not expose,
and rollback reports `runtime_lifecycle_relinquish_record_missing`.

Recovery verified the exact retained candidate identity, stopped only that
candidate, restored admission, removed candidate ingress, and sealed the
transaction as `failed_preserved_old_generation`. The selected generation is
still `0.28.0-fb5a8ef317c2-9cf9b4f6919d`. No browser, profile, route, viewer,
or authenticated workload was closed. The next repair must make the named-lane
readiness probe resolve through selected runtime-host ingress. Adding route
capacity or weakening owner evidence is not an acceptable workaround.

The ingress-aware readiness repair and the remaining transactional adoption
seams are now closed. Release candidate
`9062cd9bf8ec2ecd6dbe0a3d1f23f0a600677be1ee1c743a3e7b7f5fdbfde568`
was accepted by transaction
`upgrade-fc81e16c-654c-4aae-b748-10dfa20af107` as installed generation
`0.28.0-9062cd9bf8ec-b5f50b43bc88`. Authenticated dashboard acceptance reused
durable handoff `r539344`, produced a ready presentation receipt at generation
33, and committed dashboard ingress revision 186 without launching or closing
a browser.

Post-install runtime census reports exactly one dashboard process, one runtime
host, one executable generation, and zero legacy daemons. The runtime monitor
is ready after one bounded reconciliation. All eight profiles are preserved.
The three retained ready browsers are preserved with their original logical
browser IDs, PIDs, and profile IDs: PID 16807 for `last30days-facebook`, PID
60208 for `default`, and PID 46155 for `qbo-soylei`. Their active session aliases
were transactionally rebound to the accepted owner generation.

Install doctor remains nonzero for two non-blocking historical bookkeeping
findings. One inactive, dead Odollo session-supervisor manifest names the prior
executable generation, and one managed process identity lacks a lifecycle
cleanup obligation. Service process GC reports zero candidates, zero
terminations, and no reclaimed pressure. Neither finding changes the accepted
runtime multiplicity or the scoped browser-acquisition decision. The inactive
named lane was neither removed nor restarted because that would require
separate workload authority.

## Validation Evidence

- Focused MCP Rust tests: pass.
- Strict Rust formatting and Clippy: pass.
- MCP no-launch inventory and resource smoke: pass.
- Service API and MCP parity: pass.
- Service client contract, type, and no-launch collection checks: pass.
- Remote-view documentation guard: pass.
- Next.js documentation build: pass.
- Workstation installer, host, VM harness, Guacamole asset, PostgreSQL
  durability, and route-user fixtures: pass.
- Release build: pass.
- Candidate operating-guide readback: pass.
- Live transactional install: blocked safely before selection as recorded
  above.
- Runtime-adoption focused tests after the repair: 45 passed.
- Strict Clippy after the repair: pass.
- Latest live transaction recovery: pass; old generation preserved and
  admission reopened.
- Final runtime-adoption suite: 46 passed.
- Final runtime-lifecycle suite: 9 passed.
- Selected-ingress and unreachable identity-fencing regressions: pass.
- Final Rust formatting, strict Clippy, and release build: pass.
- Installed binary digest and selected generation: exact match.
- Installed `agent-browser://operating-guide` listing and readback: pass.
- Installed focused skill source equality and validator: pass.
- Accepted live transaction and authenticated dashboard receipt: pass.
- Post-install profile and retained-browser preservation: pass.
