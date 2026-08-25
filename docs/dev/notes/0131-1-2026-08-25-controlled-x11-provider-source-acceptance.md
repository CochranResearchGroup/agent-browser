# Plan 0131 Slices A And B Source Acceptance

Date: 2026-08-25

State: SOURCE ACCEPTED

Branch: `feature/plan0131-controlled-x11-desktop-provider`

Baseline: `a54b0f976fb20e801d8e09e844708753c80ac79d`

## Accepted Source Boundary

Slices A and B implement one development-only provider behind the existing
`desktop_interact` action. The provider is admitted only when the running
binary matches an immutable development-generation manifest that enables
`p131-controlled-x11-v1`. Production and unmanifested binaries fail before
capture, service-state resolution, effect-journal preparation, or input.

The accepted source includes:

- a closed native XTEST event adapter that rejects root execution;
- a private cross-process route and display fence shared by input effects and
  controller mutations;
- a private atomic prepared, acknowledged, or uncertain effect journal;
- an authority recheck after acquiring the external fence and before emission;
- an exact installed-generation hash and provider-capability admission gate;
- deterministic desktop target and after-state detection;
- native active-fixture focus and pointer observation;
- a repository-owned, network-free X11 fixture with bounded failure modes;
- synchronized CLI, HTTP, MCP, generated-client, README, skill, contract, and
  docs-site surfaces.

No installed development binary, live fixture, XTEST effect, production
provider, authentication flow, credential, or release is accepted by this
note.

## Source Evidence

The following gates passed from the Plan 0131 worktree:

- strict Rust formatting and Clippy;
- focused desktop interaction, controller coordinator, desktop capture,
  service health, and service contract suites;
- generated service client, client types, examples, and API to MCP parity;
- no-launch service contracts and service collections;
- dashboard inspector actions and core actions structural checks;
- route-confusion, WSL Cargo safety, workstation installer, host provision,
  fresh-VM harness, Guacamole assets, PostgreSQL durability, and route-user
  synchronization fixtures;
- remote-view handoff documentation checks and the Next.js docs production
  build;
- development runtime fixture, controlled target detector, effect replay,
  abandoned prepared state, same-route contention, unrelated-route progress,
  forbidden routing, production-unavailable dispatch, and XTEST constructor
  tests;
- `git diff --check`.

`pnpm test:actions-remediation-architecture` remains red on its independent
P0101 and P0108 inventory ledger. The exact failure set was reproduced from
clean `main`; the Plan 0131-specific core actions structural check passed. This
baseline debt is not treated as installed or live acceptance evidence and is
not expanded into this lane.

## Next Gate

Slice C may begin only with a fresh development-runtime preflight that captures
the exact source commit, candidate hash, selected development generation,
production identity, provider readiness, route and display inventory, browser
and controller state, resource census, rollback selector, and retained cleanup
obligations. Production effects remain forbidden.
