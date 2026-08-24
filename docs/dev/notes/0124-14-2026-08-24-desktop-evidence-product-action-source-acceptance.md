# Plan 0124 Desktop Evidence Product Action Source Acceptance

Date: 2026-08-24

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: SOURCE-ONLY

Status: ACCEPTED

## Accepted Boundary

The task-shaped `desktop_evidence_observe` service action now invokes the
configured observation-only Desktop Evidence Episode for the closed
`stacking_or_occlusion` evidence surface. Callers provide a service-owned
browser identity and accountable task labels. They cannot provide raw display,
route, window, coordinate, CDP, or provider plumbing.

The action is coherent across native dispatch, CLI, HTTP, MCP, generated
client, service contracts, help, README, the repository agent skill, and the
documentation site. It does not launch, navigate, attach to, take over, or
close a browser. Durable job, stream, dashboard, and incident projections
remove frame pixels and provider readiness details.

The presentation-capacity adapter now reserves the exact route and display
slot already bound to the requested browser. Releasing an observation lease
preserves an active retained browser's active state and browser binding.
Human-controller, passive-viewer, protected-reserve, browser-exclusion, and
pressure-admission conflicts remain fail-closed.

Successful desktop outcomes include context and frame receipts. Typed
admission, adapter-unavailable, or human-continuation outcomes do not fabricate
capture data, and the response schema and generated TypeScript reflect that
conditional boundary. Frame bytes are response-only and appear only when the
caller explicitly requests them.

## Validation

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`
  passed.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_evidence -- --nocapture`
  passed 43 tests.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml service_request -- --nocapture`
  passed 49 tests.
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
  passed.
- `pnpm test:service-api-mcp-parity` passed for 101 service-request actions.
- `pnpm test:service-client` passed the generated contract, type, export,
  helper, observability, fixed-input, managed-profile, and example gates.
- JSON schema parsing and `git diff --check` passed.

The broader `pnpm test:service-contracts-no-launch` command reached its smoke
binary but was rejected by the independently introduced single-runtime-host
admission gate before exercising this action. The focused native no-launch
test proves this action uses the existing daemon queue and does not launch a
browser. The unrelated smoke compatibility failure remains visible and is not
claimed as a passing gate here.

## Remaining Boundary

Paired CDP evidence for browser-external UI and exact native desktop staging,
snapshot, restoration, and trigger composition remain unfinished. No
development runtime was installed and no live browser state changed in this
slice. Configured production input remains unavailable pending independent
Plan 0110 live acceptance.
