# Plan 0127 Slice A Source And Development Skill Acceptance

Date: 2026-08-23

Status: ACCEPTED | PROVIDER EFFECTS NOT STARTED

Plan: `docs/dev/plans/0127-2026-08-23-development-presentation-provider-isolation-plan.md`

Authority: SOURCE AND DEVELOPMENT SKILL EFFECTS | PRODUCTION READ-ONLY

## Accepted Evidence

- the development provider descriptor owns a separate root, secrets, state,
  receipts, inventory, compose project, services, database, ports, route users,
  connections, display reservations, the shared immutable XRDP target, and
  slot lifecycle;
- the default inventory contains four warm and two elastic slots and supports a
  configurable arbitrary-N maximum without route A or route B assumptions;
- isolation fixtures reject production ports, overlapping production paths,
  duplicate provider and route ports, duplicate route fields, and incomplete
  route inventory;
- an absent provider reports `unconfigured`, `ready: false`, and
  `blocking: false`; required absence and configured manifest drift fail doctor;
- the repository skill was atomically copied to
  `~/.local/share/agent-browser-dev/home/.codex/skills/agent-browser`;
- source and development skill tree SHA-256 both read
  `0d18c36c362b450a8cca2da962184b6d89dd343d5b7dc30357ef1fc7121e0f6d`;
- the shared user-scoped production skill was not changed;
- production selected generation remained
  `0.28.0-a89625b870c3-1e2c09b12ebc` and exact production browser and session
  identity projections were unchanged across development skill publication;
- the installed development doctor remained green with the provider explicitly
  unconfigured and the development skill current.

## Validation

```text
pnpm test:development-presentation-provider  PASS
pnpm test:development-runtime                PASS
pnpm --dir docs build                        PASS
pnpm test:remote-view-handoff-docs           PASS
bash scripts/release/test-verify-release-assets.sh  PASS
git diff --check                             PASS
```

The repository validation selector was run against `origin/main`. Its shared
skill comparison is intentionally superseded by the P127 development-only
target. Updating the shared skill would violate this packet's production
read-only boundary.

## Remaining Gate

No Guacamole, guacd, PostgreSQL, XRDP, display, route-user, secret, systemd,
container, Cooper ingress, or provider-manifest effect occurred. P127 Slice B
must implement and separately review that deployment adapter. P124 Slice G
then requires the provider-backed four-to-six acceptance and exact return to
four before either plan can complete.
