# Receipt 0109 | Runtime Dependability Source Acceptance

Date: 2026-08-11

Authority: source only

Base: `1bb6ef2fc7bcf05f24a50ec7d8d078ab9a761d8d`

Accepted source head: `c00c96556d4c15d6106ada74e80349a95ea16a05`

## Result

Plan 0109 Slices B through G are source complete. Slice H remains explicitly
unauthorized. The source now rejects ambiguous global close before effects,
supervises named Linux daemon sessions without launching browsers, separates
requested remote-view readiness from global drift, projects renderer crashes
into typed lifecycle evidence, and rejects unaccountable effectful requests.

## Commit Ledger

- `0b0aa14b` `fix: reject ambiguous global session close`
- `9c538699` `docs: clarify global close safety`
- `9ab3e6b6` `fix: project renderer crashes into service lifecycle`
- `bcf73c7b` `feat: supervise named daemon sessions`
- `beb8d18b` `feat: scope remote view diagnostics to requested subjects`
- `7d82faf4` `fix: bind browser effects to accountable requests`
- `7989ba6a` `refactor: keep renderer crash envelope owned`
- `c00c9655` `test: isolate service status compatibility state`

## Validation

- `scripts/ci/rust-tests.sh`: exit zero; parallel-safe lane 1,071 passed, zero
  failed, 57 ignored; close scope 4/4, requested doctor scope 1/1,
  named-session supervisor 1/1, and all serial partitions passed.
- Rust formatting and strict Clippy passed through `scripts/ci/cargo-safe.sh`.
- `pnpm test:service-client`, `pnpm test:service-api-mcp-parity`,
  `pnpm test:service-collections-no-launch`, `pnpm test:mcp-read-no-launch`,
  `pnpm test:actions-architecture`, the remediation architecture gate, and
  `pnpm test:wsl-cargo-safety` passed.
- Dashboard workspace and inspector tests, dashboard build, docs build,
  route-confusion, workstation and host-provision fixtures, release asset
  verification, validation selection, and `git diff --check` passed.

The no-launch collection smoke exercised profiles, browsers, sessions, tabs,
monitors, site policies, providers, and challenges through CLI, HTTP, MCP, and
generated-client surfaces. Its browser sentinel remained unused and no
browser, session, or tab ownership was created.

## Review Closure

Fresh-context Cycle 1 found two structural issues introduced during the work:
the renderer crash response helper was in the action dispatcher and the
process-identity consumer inventory omitted new consumers. `7989ba6a` fixed
both. The first canonical driver then exposed one test reading ambient service
state; `c00c9655` isolated it. Closed-world Cycle 2 found no remaining source
blocker.

## Effects Withheld

No install, unit enablement, service restart, browser launch, browser adoption,
doctor repair, protected-profile access, route mutation, Chromium edit, or
downstream retry occurred. The operator-authored Google Messages handoff note
was reviewed but not modified or committed. A future installed canary must use
a disposable session and profile.
