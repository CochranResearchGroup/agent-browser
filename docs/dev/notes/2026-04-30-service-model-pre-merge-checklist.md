# Service Model Pre-Merge Checklist

Date: 2026-04-30

## Scope

This checklist applies to `service-model-phase-0` before merging service-model,
browser-health, service-control, HTTP API, MCP, or docs changes into the main
line or preparing a release branch.

Normal project work lands in this project's `origin` repository. Do not prepare
release or feature pull requests against the original upstream repository unless
the maintainer explicitly asks for an upstream contribution.

## Required Gates

Run these gates from the repo root unless noted otherwise:

```bash
cargo test --manifest-path cli/Cargo.toml service_ -- --test-threads=1
pnpm test:service-api-mcp-parity
cd docs && pnpm build
pnpm test:service-health-live
git diff --check
```

Use the broader native serial regression before a release cut or when a service
slice touches shared native behavior outside the `service_` selector:

```bash
cd cli && cargo test -- --test-threads=1
```

The service-health aggregate includes the static API/MCP parity guard first,
then the live Chrome service-health smokes. If it fails, run the individual
expanded commands listed in `docs/dev/notes/2026-04-27-browser-health-closeout.md`
to isolate the failing surface.

## What These Gates Prove

- `service_` Rust tests cover the service model, command mapping, HTTP mapping,
  MCP mapping, contract assertions, job lifecycle, recovery policy, incidents,
  collections, and trace read models without launching Chrome.
- `pnpm test:service-api-mcp-parity` keeps named browser-control HTTP endpoints,
  typed MCP tools, README, skill guidance, and docs-site references aligned.
- `cd docs && pnpm build` proves the Next.js docs site still compiles after
  feature or contract documentation changes.
- `pnpm test:service-health-live` proves the minimum always-on service behavior
  with live Chrome sessions: shutdown remedies, recovery traces, retry override,
  HTTP/MCP incident parity, and job naming warnings.
- `git diff --check` catches whitespace and patch hygiene issues before commit
  or handoff.

## Release Preparation Additions

For an actual release, also follow the release process in `AGENTS.md`:

- create a release-preparation branch
- bump `package.json`
- run `pnpm version:sync`
- update `CHANGELOG.md` with exactly one active release marker block
- update `docs/src/app/changelog/page.mdx`
- include contributors from the git log since the previous tag
- validate the release workflow with the GitHub Actions `Release` workflow in
  `dry_run` mode before publishing-sensitive changes are merged

## Handoff Rule

Closeout notes should report each gate as passed, failed, or intentionally not
run. If a live gate is skipped, state why and name the remaining risk directly.
