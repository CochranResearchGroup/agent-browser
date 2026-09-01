# Plan 0138 | Authentication Run Provider-Free Foundation

Date: 2026-08-29

State: CLOSED

Acceptance: SOURCE ACCEPTED

Execution state: `slice_a_integrated_successor_development_open`

Lane: P110 Authentication Run Slice A

Source baseline: `abbfd3c995a57e35cd2b832acdcd83ca29b4cff7`

Rebased integration baseline: `1c96ac6782c3e8f5519c4e6005b9f58db084b578`

Source checkpoint: `d0786a5a`

Integration receipt: `18fe5995e061c16fe97d83119040ea0a37c721c2`

Branch: plan/authentication-run-foundation

Target: main

Integration: merge

Integration model: isolated worktree and reviewable branch. Promotion to
`main` is a separate integration decision after source acceptance.

Authority: PLAN, SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, AND LOCAL
VALIDATION ARE IN SCOPE. THIS PLAN DOES NOT AUTHORIZE A LIVE BROWSER LAUNCH,
TENANT PROFILE USE, MESSAGE OR MAILBOX ACCESS, CREDENTIAL OR CHALLENGE
MATERIAL RETRIEVAL, PROVIDER MUTATION, INSTALLATION, OR PRODUCTION RUNTIME
CHANGE.

Depends on:

- Plan 0110 and its accepted desktop perception and interaction proofs;
- `docs/dev/notes/0110-f1-2026-08-23-passkey-and-two-factor-authentication-fieldwork.md`;
- `docs/dev/notes/2026-08-29-books-receipts-unattended-authentication-handoff.md`;
- Books Receipts `browser-authentication-recipe.v2`; and
- the current profile lifecycle and same-profile tab guarantees on `main`.

## Executive Decision

Agent Browser will model unattended authentication as a durable
`AuthenticationRun`, not as a loose sequence of browser and provider calls.
The run binds one authenticated service principal, task, target account,
managed profile, browser, service session, login tab, and site recipe. It owns
challenge delivery fences, response-only provider actions, verification,
replay protection, bounded transitions, and safe receipts.

The first packet freezes and proves this contract with internal Rust types and
synthetic providers. It deliberately exposes no public action and performs no
live effect. This establishes the security boundary before public API parity
or provider integration can make it expensive to change.

## Goal

Implement a focused provider-free Authentication Run foundation that proves:

- exact profile, browser, session, tab, account, principal, task, and recipe
  binding;
- channel-specific actions for native credential submission, SMS OTP
  submission, same-profile device-verification link opening, and remember-
  device confirmation;
- provider material remains inside a response-only adapter action and never
  enters the run, receipt, error, serialized state, or debug output;
- an email verification link can succeed only with proof that a new tab was
  opened in the run's exact profile, browser, and service session;
- delivery observation is ready before the action that triggers a challenge;
- exactly one post-fence candidate is required before challenge material can
  be consumed;
- operation and challenge replay are rejected before another effect;
- a bounded transition budget prevents open-ended retry loops; and
- authentication succeeds only after an exact-target verifier returns a
  positive result.

## Non-Goals

- No BILL, Google Messages, im-receipts, Gmail, or mail-receipts traffic.
- No real username, password, OTP, email body, or verification URL.
- No generic credential broker. Stock Chrome native credential storage remains
  the default BILL primary-authentication path.
- No public CLI, MCP, HTTP, TypeScript, schema, dashboard, or skill surface.
- No Service State migration or installed runtime change in this packet.
- No CAPTCHA, passkey, or human-escalation implementation.
- No inference that a visible page or successful navigation proves the target
  account is authenticated.

## Frozen Domain Boundary

`AuthenticationRun` owns orchestration and durable redacted evidence.
`ResponseOnlyChallengeAction` owns sensitive challenge-material custody and
returns only a redacted effect receipt. `AuthenticationVerifier` observes the
result and returns an exact-target verdict without returning private page
content.

The core run engine never receives the OTP or verification URL. Synthetic
adapter canaries retain those values only inside their adapter instance and
prove that all outward projections remain free of them.

## Acceptance Contract

Slice A is source accepted only when all of the following are true:

1. A focused module defines the run binding, state, challenge, action,
   response-only adapter, verifier, and durable receipt types.
2. Construction rejects missing stable identities and a zero transition
   budget.
3. Challenge handling rejects a delivery trigger until its watch is ready.
4. Consumption rejects zero or multiple candidates without calling the
   response-only effect.
5. A successful SMS action records delivery-fence, unique-candidate,
   response-only, and consumption proof without recording the OTP.
6. A successful email-link action additionally proves a new tab in the exact
   bound profile, browser, and service session without recording the URL.
7. Reusing an operation id or already-consumed challenge fails before effect.
8. Transition-budget exhaustion fails closed.
9. The terminal authenticated state requires an exact-target positive
   verifier receipt.
10. Synthetic OTP and verification-URL sentinels are absent from serialized
    runs, receipts, errors, and debug projections.
11. Focused tests pass through `scripts/ci/cargo-safe.sh` and formatting is
    clean.

## Work Units

### Slice A1: Contract

- add one focused internal module;
- define validation and safe receipt shapes; and
- register the module without changing the public service contract.

### Slice A2: Synthetic acceptance

- implement fake response-only SMS and email-link actions in tests;
- implement an exact-target fake verifier; and
- exercise ordering, uniqueness, same-profile, replay, budget, verification,
  and canary-redaction invariants.

### Slice A3: Closeout

- run focused tests, formatting, and the repository validation selector;
- write a source-acceptance receipt;
- update ROADMAP and RUNBOOK with the exact completed and remaining boundary;
  and
- commit and push the isolated branch.

## Hard Stops

Stop before:

- invoking any installed browser, tenant profile, im-receipts helper, mailbox,
  or external provider;
- adding a public action before CLI, MCP, HTTP, TypeScript, schema,
  documentation, and generated-client parity are planned together;
- persisting secret-bearing request data or hashing a low-entropy OTP;
- opening a bearer-like verification URL through a public command request;
- weakening exact profile, browser, session, tab, principal, or target-account
  binding; or
- promoting the branch to `main` without a fresh integration decision.

## Next Packet After Slice A

The next bounded packet may persist `AuthenticationRun` in Service State and
publish its safe create, inspect, resume, cancel, and result contract across
all public surfaces. Live im-receipts and mail-receipts adapters remain later
packets and require their own authority and acceptance gates.

## Slice A Acceptance

Slice A is source accepted at checkpoint `d0786a5a`. Nine focused synthetic
tests pass after rebasing onto `1c96ac67`. They prove complete binding,
watch-before-trigger ordering, unique-candidate enforcement, response-only SMS
and email-link custody, same-profile new-tab proof, operation and challenge
replay rejection, transition-budget exhaustion, exact-target verification,
and absence of the OTP and URL canaries from durable and debug projections.

The repository selector's strict production-target Clippy gate, safe formatting
gate, and serialized focused tests pass. A broader optional test-target Clippy
run surfaced pre-existing test-only warnings outside this branch; they do not
affect the selected gate. No live browser, profile, provider, message, mailbox,
installation, or runtime effect occurred.

Detailed evidence is in
`docs/dev/notes/0138-2026-08-29-authentication-run-provider-free-source-acceptance.md`.
