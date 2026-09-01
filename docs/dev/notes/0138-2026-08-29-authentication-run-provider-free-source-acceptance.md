# Plan 0138 Slice A | Authentication Run Provider-Free Source Acceptance

Date: 2026-08-29

State: SOURCE ACCEPTED

Branch: `plan/authentication-run-foundation`

Source checkpoint: `d0786a5a`

Rebased integration baseline: `1c96ac6782c3e8f5519c4e6005b9f58db084b578`

Effect posture: provider-free and runtime-neutral

## Accepted Outcome

Agent Browser now has an internal provider-free `AuthenticationRun`
foundation for unattended authentication. The run binds all stable authority
and target identities, records only redacted evidence, rejects effect replay,
limits transitions, and requires exact-target verification before reporting
authentication success.

The sensitive-material boundary is structural: the run calls a
`ResponseOnlyAuthenticationAction` that performs the whole native-credential,
OTP, or device-link effect internally and returns only a redacted
`AuthenticationActionReceipt`. The core does not accept a password, OTP, email
body, or verification URL parameter.

## Accepted Contract

- Complete stable binding is required at construction.
- A positive transition budget is mandatory.
- Delivery observation must be ready before challenge delivery is triggered.
- Exactly one post-fence candidate is required before an adapter is called.
- Successful challenge receipts retain the delivery-fence id, candidate count,
  response-only custody proof, and consumed-once posture.
- Native primary authentication proves stock-browser credential-store use and
  forbids credential replay.
- Device verification additionally requires a distinct new tab in the exact
  bound profile, browser, and service session.
- Operation ids and completed challenge ids cannot be reused.
- Authentication succeeds only when the verifier proves the exact target
  service, account, profile, browser, and session.
- Verifier ambiguity or mismatch becomes operator intervention, not success.

## Synthetic Canaries

The tests use one synthetic OTP and one synthetic bearer-like verification
URL inside fake response-only adapters. Assertions scan serialized run state,
receipts, errors, and debug projections and confirm neither canary crosses the
adapter boundary.

The email path includes both an accepted same-profile new-tab receipt and a
deliberate wrong-profile receipt. The wrong-profile proof is rejected and the
run becomes blocked.

## Validation

Focused test command:

```text
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml authentication_run -- --test-threads=1
```

Result after rebasing onto `1c96ac67`:

```text
9 passed; 0 failed; 0 ignored; 2690 filtered out
```

Additional checks:

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`:
  passed;
- `git diff --check`: passed;
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`:
  passed; and
- the repository validation selector passed and selected these exact Rust
  quality gates. A broader optional `--tests` lint run reached pre-existing
  test-only warnings outside the Authentication Run packet.

## Effect Audit

No browser was launched. No tenant profile was acquired. No SMS, email,
credential store, im-receipts, mail-receipts, BILL, or other provider was
queried or mutated. No candidate was installed and no production Service State
was read or changed.

## Remaining Boundary

This packet intentionally has no public action and no Service State
persistence. The next provider-free packet should add durable run storage and
one coherent create, inspect, resume, cancel, and result contract across CLI,
HTTP, MCP, generated client, schema, help, skill, and documentation surfaces.

Live im-receipts SMS and mail-receipts verification-link adapters remain later
packets. The mail path still needs an internal first-class verification-link
claimant so bearer-like URLs never cross public tool or command request
surfaces.
