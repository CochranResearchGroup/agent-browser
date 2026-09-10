# Field Note 0162 | Site Login State Machine Source Slice

Date: 2026-09-10

Status: SOURCE SLICE COMPLETE | PUBLIC SEALED TRANSPORT OPEN

Consumer: Books Receipts Plan 0240

Related work: Plan 0138 authentication-run foundation, Field Note 0161
first-class deterministic site login and password save

## Outcome

`AuthenticationRun` now has the provider-free states and receipt contracts
needed to model an identifier-first login and Chrome password persistence
without treating either as a generic page-input recipe.

The v2 run contract adds:

- distinct identifier-form, password-form, and password-manager-decision
  states;
- response-only identifier submission bound to one observed form instance;
- browser-chrome semantic Save, Update, and Never actions bound to one observed
  prompt instance and persistence policy;
- exact profile, browser, session, tab, service, and account validation on each
  site observation;
- replay fences for operation ids and completed form or prompt instances; and
- typed operator intervention for unlock, ambiguous, and unsupported states.

The durable projections retain provider and effect identifiers, semantic state,
policy outcome, and verification booleans. They do not retain an account
identifier value, password, one-time code, verification URL, or generic input
payload.

## Validation

The focused Rust test command is:

```text
cargo test --manifest-path cli/Cargo.toml authentication_run -- --nocapture
```

It passes provider-free cases for the identifier-to-password sequence, exact
target verification, SMS and email challenge fencing, Save/Update/Never prompt
handling, ambiguous prompt refusal, policy mismatch refusal, operation and
challenge replay, wrong-tab observation, transition budgets, and canary
absence from durable projections.

## Remaining Boundary

This slice deliberately does not expose a public service action and does not
touch a live profile. The next slice must persist the v2 run in Service State
and add closed start/status/resume/cancel actions whose internal adapters own
site observation, native credential use, browser-chrome accessibility, and
response-only challenge delivery. Generic `fill`, `type`, `evaluate`,
clipboard, keyboard, and `ui_action` remain forbidden secret transports.

No installed runtime or provider effect is qualified by this source result.
