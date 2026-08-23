# Plan 0127 | Development Presentation Provider Isolation

Date: 2026-08-23

State: IN PROGRESS

Execution state: `slice_a_source_and_development_skill_accepted_provider_apply_pending`

Lane: P127

Authority: SOURCE AND DEVELOPMENT SKILL EFFECTS | NO PROVIDER EFFECTS | PRODUCTION READ-ONLY

Depends on:

- `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`
- `docs/dev/plans/0125-2026-08-23-development-runtime-isolation-and-build-capacity-plan.md`
- `docs/dev/policies/0031-runtime-vs-product-boundary.md`
- `docs/dev/policies/0032-runtime-state-governance.md`
- `docs/dev/policies/0036-architecture-guardrails.md`

## Goal

Add a development-owned presentation-provider authority that can later host
Plan 0124 provider-backed acceptance without borrowing production Guacamole,
XRDP, display, route, database, process, secret, cleanup, or ingress state.
Give development agents a skill copy in the development pseudo-home so new
guidance can be exercised without changing the shared production skill.

## Frozen Decisions

### One Descriptor Owns The Namespace

`DevelopmentPresentationProvider` owns the provider root, secrets, durable
state, receipts, route inventory, compose project, service identities,
database identity, ports, users, connections, displays, slot lifecycle, and
development skill target. Callers consume the descriptor as a unit.

The initial exact namespace is:

- root: `~/.local/share/agent-browser-dev/presentation-provider`;
- manifest: `provider.json` inside that root;
- Guacamole port 8093, guacd port 4823, and PostgreSQL port 55433;
- compose project `agent-browser-dev-presentation`;
- route users `agent-browser-dev-route-1` through
  `agent-browser-dev-route-6`;
- displays `:120` through `:125` and RDP ports 3490 through 3495;
- four warm slots and a pressure-admitted hard maximum of six;
- development skill target
  `~/.local/share/agent-browser-dev/home/.codex/skills/agent-browser`.

Environment overrides remain one reviewed descriptor input. Isolation
validation rejects known production ports, overlapping production paths,
production identities, duplicate route fields, and incomplete route
inventory. There is no legacy route A or route B adapter in this authority.

### Unconfigured Is Visible But Initially Nonblocking

The accepted development executable and dashboard continue to work while the
provider is absent. Status reports `unconfigured`, `ready: false`, and
`blocking: false`. Setting
`AGENT_BROWSER_DEV_PRESENTATION_PROVIDER_REQUIRED=1` makes absence fail doctor.
Once `provider.json` exists, exact descriptor drift is always blocking. The
doctor never reports a configured provider from ports or processes alone.

### Development Skill Is A Separate Publication

`development-runtime:skill-sync` copies the repository Agent Browser skill
atomically into the development pseudo-home. It never writes
`~/.codex/skills/agent-browser` or another shared user-scoped skill. Status
compares a deterministic tree digest so missing and stale development guidance
are distinguishable.

### Provider Apply Is A Later Effect Boundary

This source packet does not create users, displays, XRDP listeners, containers,
database records, secrets, systemd units, Cooper routes, or provider resources.
A later reviewed apply packet must:

1. capture exact production and development before-state;
2. provision only identities in the development descriptor;
3. publish a development-only `/guacamole` ingress after readiness;
4. write the exact provider manifest and an effect receipt;
5. prove production identity unchanged;
6. support exact rollback and quarantine on ambiguous cleanup.

Privileged operations must cross one intentional operator boundary. No helper
may silently reuse production credentials or provider state.

## Execution Slices

### Slice A | Source Authority

- add the descriptor, isolation validator, manifest projection, status, and
  doctor;
- integrate provider status into the development runtime doctor without
  breaking the accepted dashboard-only environment;
- add arbitrary-N and collision fixtures;
- add the development-only skill publisher and digest status.

Status: ACCEPTED. See
`docs/dev/notes/0127-1-2026-08-23-development-provider-source-and-skill-acceptance.md`.

### Slice B | Provider Deployment Adapter

- render development-owned Guacamole, guacd, PostgreSQL, XRDP, display, and
  route-user resources from the descriptor;
- add secrets bootstrap with private permissions and no production fallback;
- add exact readiness, reconciliation, rollback, and cleanup receipts;
- extend Cooper inventory with development `/guacamole` only after provider
  readiness.

### Slice C | Plan 0124 Live Acceptance

- converge to four warm development slots;
- exercise provider-backed evidence staging and restoration;
- scale from four to six under measured pressure and return to four;
- prove three lifecycle repetitions leave no route, display, process,
  container, database, lease, browser, or handoff residue;
- prove production before and after identity unchanged.

## Validation

The source packet runs:

```bash
pnpm test:development-presentation-provider
pnpm test:development-runtime
pnpm validation:select -- --base origin/main
git diff --check
```

Provider deployment and live acceptance require their own focused fixtures,
development doctor receipt, provider readiness receipt, production
non-interference receipt, and fresh OS process and pressure readback.

## Acceptance Boundary

Slice A acceptance proves only that the source owns an isolated namespace and
can detect configuration drift. It does not prove Guacamole, XRDP, display,
remote-view, desktop capture, scale-out, or cleanup readiness. Plan 0124 stays
in progress until P127 Slices B and C complete the remaining provider-backed
Slice G evidence.

## Hard Stops

- Never map development `/guacamole` to production port 8092.
- Never reuse production route users, connections, displays, database, secrets,
  compose project, containers, or cleanup inventory.
- Never publish provider ingress before exact readiness.
- Never update the shared production skill as part of development rollout.
- Never claim live provider acceptance from a descriptor or fixture manifest.
- Never provision beyond current resource admission or the configured maximum.
