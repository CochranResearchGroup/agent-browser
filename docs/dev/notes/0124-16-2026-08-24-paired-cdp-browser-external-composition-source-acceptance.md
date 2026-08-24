# Plan 0124 Paired CDP Browser-External Composition Source Acceptance

Date: 2026-08-24

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: SOURCE-ONLY

Status: ACCEPTED

## Accepted Boundary

The configured Desktop Evidence Episode now composes one development-only
`passkey_chooser` evidence request. The request must bind the exact current
service-owned browser and tab through `serviceTabHandle` and must contain
exactly one bounded selector-based page click. Production rejects this branch
before presentation reservation, native scene mutation, or trigger dispatch
and remains read-only pending independent Plan 0110 acceptance.

The provider resolves the current ready browser, tab, target, session, process
identity, and CDP endpoint from Service State. It reads the exact page target
inventory, verifies that the target debugger endpoint remains under the same
endpoint authority, and connects directly to the page target without target
activation or focus mutation.

The transaction order is exact: reserve, snapshot, stage, trigger, paired page
absence evidence, capture-ready proof, desktop capture, verification,
restoration, release, and cleanup. Paired evidence uses bounded DOM and page
screenshot material only in process-local memory, emits a digest receipt, and
discards the raw material. A page-owned passkey modal prevents desktop
classification. Trigger transport, exception, or missing-result uncertainty
returns `desktop_external_trigger_outcome_unknown` with a stable effect digest
that must be reconciled before retry.

Trigger failure and post-trigger paired-evidence failure both restore the
scene, release the exact slot, and complete cleanup. A successful trigger
receipt is preserved in the terminal failure receipt when a later paired-CDP
step fails. Active retained browsers are not parked or terminated.

The CLI, generic and dedicated MCP tools, HTTP service-request schema,
generated JavaScript and TypeScript client, help output, README, agent skill,
and service-mode documentation expose the same bounded request contract.

## Validation

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`
  passed.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_evidence -- --nocapture`
  passed 55 tests.
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
  passed.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
  passed.
- `pnpm test:service-api-mcp-parity` passed.
- `pnpm test:service-client` passed.
- `pnpm --dir docs build` passed.
- Both changed JSON contracts parsed successfully and `git diff --check`
  passed.

## Remaining Boundary

This slice does not install or identify a development runtime and does not
claim observation of a real browser-external chooser. Controlled live proof
must establish installed binary identity, exact service-tab binding, no CDP
target activation, pre-trigger staging, desktop-visible chooser evidence,
paired page absence, exact restoration, retained-browser preservation, human
viewer precedence, and cleanup under the development runtime. Production
desktop input remains unavailable pending Plan 0110.
