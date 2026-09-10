# Public AuthenticationRun Custody Slice

Date: 2026-09-10
Branch: `feature/p0240-sealed-auth`

## Outcome

The generic service-request boundary now admits four closed authentication-run
actions: start, status, resume, and cancel. Start and status establish durable,
caller-bound custody; cancel is idempotent; resume fails closed with
`authentication_run_sealed_provider_unavailable` until the sealed credential,
site-observation, and response-only challenge provider is connected.

The request contract accepts only opaque account and recipe references. It
rejects credential values, one-time codes, selectors, arbitrary UI actions,
scripts, expressions, and generic params. A start is bound to the current
valid service tab handle across profile, browser, session, tab, target, lease,
owner, caller, and trace identity. Idempotency-key reuse with a changed request
is rejected even when the changed request would derive another run id.

Durable Service State contains the internal state machine and redacted
receipts. Public projections hash the account reference and omit the raw
idempotency key. Generated TypeScript request and response contracts include
the closed actions.

## Validation

- `cargo test --manifest-path cli/Cargo.toml service_authentication_run -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_request -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_state_round_trips -- --nocapture`
- `pnpm generate:service-client --check`
- `git diff --check`

The focused service-request suite passed 62 tests. Authentication custody
passed four focused tests, and the generated-client and persistence gates were
green.

## Remaining Gate

This slice does not claim deterministic login. The resume action is deliberately
inert until the sealed provider can observe one repository-owned site recipe,
resolve browser-owned credential references, obtain a bound response-only IM
Receipts code, execute one budgeted action per observed form/challenge instance,
and append a redaction-safe receipt. No installed daemon or live profile was
changed by this slice.
