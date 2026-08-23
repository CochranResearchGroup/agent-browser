# Plan 0122 Source Acceptance

Date: 2026-08-23

Source baseline: `8c81de89e8103f9d990af7fbb7bb752d6473d1e9`

Implementation commit: `990e6b31`

## Finding

The Service access plan admitted a profile-compatibility row when any one of
profile ID, host ID, or executable ID matched. Executable preflight required
all three fields to match. This let an access plan report compatibility for a
selected profile using a row that actually belonged to another tenant profile.

## Repair

`browser_profile_compatibility_matches` now owns the exact
profile-host-executable predicate. Access-plan evidence selection and runtime
preflight both call it. The regression fixture selects `bill-soylei` by BILL
target and SoyLei account while the only registry row names
`bill-other-tenant` on the same host and executable.

## Red And Green Evidence

Before the repair, the new fixture failed with `left: Number(1), right: 0`.
After the repair it passed and the access plan reported
`profileCompatibility.status=not_declared`.

Passing checks:

- `git diff --check`
- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml service_model -- --test-threads=1` (34 passed)
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1` (41 passed)
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml service_browser_capability_preflight -- --test-threads=1` (4 passed)
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`

## Boundary

No browser, provider, tenant, profile, authentication, lease, or installed
runtime effect occurred. This receipt proves source behavior only.
