# Books Receipts Unattended Authentication Handoff

Date: 2026-08-29

Status: CONSUMER CONTRACT READY | AGENT BROWSER PRODUCT CAPABILITY OPEN

Consumer: Books Receipts

Related work: P110 authentication fieldwork and P137 profile acquisition

## Requested Outcome

An accounting automation consumer needs routine BILL and QuickBooks browser
authentication to complete without month-close operator work. The desired
normal path is:

1. acquire the exact tenant-owned stock Chrome profile;
2. accept a still-authenticated retained session;
3. otherwise let Chrome submit the profile's already saved BILL username and
   password without reading either value;
4. if BILL sends an SMS OTP, fence and consume it through im-receipts' existing
   local-private 2FA helper;
5. if BILL sends a device-verification email, claim its URL through mail-
   receipts and open it internally in a new tab on the same acquired profile;
6. confirm a tenant-approved remember-device prompt;
7. verify the exact authenticated service and account target; and
8. return one redacted terminal receipt to the consumer.

Provider-enforced CAPTCHA, biometric, secure-desktop, master-password, legal
consent, push approval, and genuinely ambiguous account choices remain typed
operator interventions. “Hands free” means zero routine authentication work,
not bypassing a provider's required human-presence control.

## Relationship To Existing P110 Fieldwork

`docs/dev/notes/0110-f1-2026-08-23-passkey-and-two-factor-authentication-fieldwork.md`
is the architectural predecessor and remains authoritative for:

- authentication challenge observation;
- a durable authentication run;
- credential-provider adapters;
- second-factor provider phases;
- desktop perception and input authority;
- post-effect authenticated-state verification; and
- response-only one-time-code consumption.

This handoff adds a concrete software-consumer contract and identifies one
current service-surface defect that the P110 note did not freeze: generic UI
input is not a safe credential transport.

The handoff does not replace or widen P137. Profile acquisition and lifecycle
recovery are prerequisites for an authentication run. Authentication is a
separate capability layered on the browser returned by acquisition. P137 may
continue its provider-free lifecycle work without waiting for this successor
lane.

## Evidence From The Accounting Consumer

Historical accounting fieldwork proved two useful behaviors without retaining
private values in this repository:

- a retained exact-profile browser can be shared through one independently
  leased tab, probed, released, and preserved; and
- stock Chrome once supplied saved BILL credentials, while remembered-device
  state later completed login without consuming an available SMS code.

The first behavior is already a public lifecycle pattern. The second is a
feasible fast path, not yet an Agent Browser guarantee. A durable product must
also handle the recovery case where the cookie is expired, Chrome cannot
autofill, or a new challenge is issued.

Books Receipts now implements a fixture-backed provider-neutral state machine
with this action vocabulary:

- `submit_native_autofill`
- `submit_brokered_primary_credential` (disabled by default)
- `select_expected_account`
- `submit_im_receipts_sms_otp`
- `open_mail_receipts_device_verification_link`
- `confirm_remember_device`

The original provider-neutral implementation is bound to Books Receipts commit
`74e9790`; its R9 correction is recipe version
`browser-authentication-recipe.v2`.

Its driver accepts only an operation ID and action name. Every returned action
receipt must prove the expected material custodian and must explicitly report
that neither secret nor challenge material was exposed. Extra fields are
discarded. The consumer never supplies a password, OTP, message body, vault
item, cookie, account label, or secret selector.

## Current Agent Browser Gap

The current service model already carries useful declarative fields:

- `credentialProviderIds` on a profile;
- `keyring` posture;
- target service and account identities;
- authenticated-service projections;
- profile acquisition and manual-seeding contracts; and
- guarded desktop interaction plus redacted interaction receipts.

The missing piece is an executable authentication capability that resolves
those declarations and channel-specific challenge adapters inside the trusted
service boundary.

The generic service UI action cannot serve as that capability:

- `ui_action` accepts ordinary `fill` and `type` steps with caller-supplied
  plaintext;
- `run_service_ui_step` copies the step into the daemon command;
- `handle_fill` consumes the raw value even though its response reports only
  the selector; and
- `handle_type` returns the typed plaintext in its result.

The durable `ServiceJob` model does not appear to retain the original complete
request, which is helpful but insufficient. MCP and HTTP request payloads,
tool transcripts, client instrumentation, process logs, error paths, and the
type result still create disclosure surfaces. Passwords and one-time codes
must not pass through these actions, `evaluate`, clipboard operations, desktop
text input, or generic request parameters.

## Product Decision

Add a first-class durable `AuthenticationRun` resource. Do not add a secret
flag to generic input.

The authentication run owns:

- service principal, task, and accountable caller;
- exact acquired profile, browser, session, and optional tab lease;
- target service, relying party, and opaque account selector;
- site-recipe version and policy digest;
- current challenge observation and freshness evidence;
- transition budget, deadlines, and effect idempotency;
- credential-provider and challenge-provider capability bindings;
- provider cursors established before delivery triggers;
- redacted transition receipts;
- authenticated-state verifier evidence; and
- terminal success, typed intervention, or failure.

The public client asks Agent Browser to start or resume a run. It does not
choose DOM selectors, provider credentials, message queries, field text,
desktop coordinates, or raw challenge values.

## Required Service Actions

Candidate public actions are:

- `service_authentication_run_start`
- `service_authentication_run_status`
- `service_authentication_run_resume`
- `service_authentication_run_cancel`

These names are design candidates, not shipped API. The final actions must
have CLI, HTTP, MCP, generated client, dashboard, contract metadata, schemas,
help, README, documentation-site, and skill parity under the repository's
service-action rules.

A start intent should contain only:

```json
{
  "targetServiceId": "opaque-service-id",
  "profileId": "opaque-profile-id",
  "accountRef": "opaque-account-ref",
  "siteRecipeId": "opaque-versioned-recipe-id",
  "policyRef": "opaque-tenant-policy-ref",
  "deadlineMs": 120000,
  "idempotencyKey": "opaque"
}
```

Agent Browser resolves the exact profile and provider bindings from governed
runtime state. Raw provider secrets never enter the intent or public result.

## Internal Sealed Action Contract

The authentication orchestrator needs internal actions that are not exposed as
generic public input:

### Native profile credential submission

`submit_native_autofill` should focus or submit a page only after a private
site recipe proves:

- exact target origin and expected form state;
- exact acquired profile and current page freshness;
- one eligible saved account, or an opaque uniquely selected account;
- no browser password-manager confirmation, unlock, or ambiguity prompt; and
- a fresh post-effect state transition.

Agent Browser must never read the filled password value. Chrome remains its
custodian. Google documents that Chrome can automatically fill a single saved
credential and can automatically sign in when both the identity service and
site support it.

### Brokered primary credential submission

`submit_brokered_primary_credential` resolves a profile-owned provider binding
inside Agent Browser. The internal provider returns a sealed one-use value to
the internal field writer, not to the orchestrator's public request, job,
event, trace, dashboard, or response object.

The provider interface should support the existing declared classes:

- browser-native password manager;
- real OS keychain;
- managed vault;
- password-manager extension; and
- an intentionally unavailable or manual provider.

The fill implementation must zero or drop plaintext buffers as soon as the
browser dispatch completes and must never interpolate the value into an error
message. Durable state retains only provider ID digest, credential-entry
digest, byte-length class if needed, operation ID, and typed outcome.

This is an optional extension point, not the SoyLei BILL default. The BILL
profile already owns the username/password in stock Chrome. Do not create or
require a second BILL password store merely to satisfy this architecture.

### im-receipts SMS OTP submission

`submit_im_receipts_sms_otp` binds directly to im-receipts. It separates:

1. `prepare_watch`
2. `watch_ready`
3. `trigger_delivery`
4. `await_delivery`
5. `claim_unique_candidate`
6. `consume_response_only`
7. `verify_result`
8. `close_watch`

The watch must be ready before the site triggers delivery. This ordering is
required by prior fieldwork: a watch started after the code arrived correctly
excluded that older message.

Candidate correlation must bind at least:

- tenant and provider binding;
- service and opaque account reference;
- sender identity allowlist;
- delivery channel;
- challenge start cursor and earliest timestamp;
- code shape and exact allowed length;
- maximum age and expiry;
- site challenge ID or nonce when available; and
- uniqueness among post-cursor candidates.

im-receipts already exposes `urgent_auth_code_watch`, which performs a pre-
catchup, bounded post-cursor live window, post-catchup, and restoration while
returning redacted `message.auth_code_candidate` events. It also exposes the
explicit operator-local `subscribers latest-2fa
--include-raw-local-message-text` readback. The Agent Browser adapter should
compose those existing surfaces locally, bind the canonical message identity
to the current challenge, extract exactly one code in memory, fill it, and
discard it. Raw command output is internal challenge material and must never be
copied into the Authentication Run, service request, job, event, trace, log,
dashboard, or response.

An applied receipt must prove watch-before-trigger delivery fencing, exactly
one accepted post-fence candidate, response-only consumption, and fresh BILL
verification. Zero or multiple candidates are typed blocked outcomes, never a
guess.

### mail-receipts device-verification link

`open_mail_receipts_device_verification_link` is a separate challenge class,
not an OTP-fill variant. mail-receipts already owns live Gmail readiness,
incremental synchronization, message provenance, and a response-only auth-code
watch. It does not yet expose a first-class verification-link watch/claim.

Add a narrow local claimant with the same cursor-before-trigger lifecycle. It
must accept exactly one new message matching the tenant-private BILL sender and
subject policy, extract exactly one URL whose origin is allowed by the BILL
site recipe, and transfer that URL only to Agent Browser's internal navigator.
Persist redacted message/link provenance and terminal reason, never the URL.

The authentication orchestrator must create the verification tab using the
already acquired browser and session identity, navigate internally, prove the
new tab shares the exact profile, verify the device effect, and then return to
fresh BILL authentication verification. A verification URL is bearer-like
material: do not pass it as a public `tab_new` or navigation argument. Public
and durable receipts report only `sameProfileNewTabProven=true`, response-only
consumption, candidate cardinality, and typed outcome.

## Site Recipe Boundary

Core Agent Browser owns the state machine and authority checks. Site recipes
own provider-specific recognition and safe transitions.

A versioned recipe should declare:

- allowed origins and relying-party identifiers;
- fresh page-state classifiers;
- expected account-selection semantics;
- submit, resend, challenge-method, remember-device, and verification
  transitions;
- whether a transition is observation-only, idempotent, one-shot, or
  non-retryable;
- supported challenge providers and exact priority;
- authenticated-state and exact-account verifier;
- maximum transitions and deadline;
- human-only challenge classes; and
- synthetic fixtures and compatible layout versions.

BILL and Intuit selectors, text, route details, and account hints belong only
in their narrow recipes or tenant-private bindings. They do not belong in
Books Receipts or the core authentication engine.

## Receipt And Redaction Contract

Public and durable receipts may include:

- run, operation, browser, profile, and recipe IDs;
- target-service ID and opaque account ref;
- challenge and action classes;
- provider class and non-secret provider ID digest;
- observation freshness and verifier result;
- transition status and bounded timing;
- material custody class;
- `secretMaterialExposed=false`;
- `challengeMaterialExposed=false`;
- provider-effect and transaction-effect booleans; and
- one typed next action or intervention reason.

They must not include:

- password, OTP, passkey, private key, recovery code, token, or cookie values;
- message body, subject, sender address, phone number, vault label, account
  label, raw login ID, or DOM field value;
- raw OCR, screenshots, clipboard content, provider stderr, or exception text;
- provider URLs, local paths, Guacamole URLs, display names, or desktop
  coordinates; or
- hashes of small-domain secrets such as six-digit codes.

Add one cross-surface secret canary test that injects synthetic sentinel values
and proves absence from requests after admission, jobs, events, traces, logs,
HTTP and MCP responses, generated client objects, dashboard projections, and
retained state.

## Retry And Lockout Safety

- Default to one native-autofill attempt and one sealed primary-credential
  attempt per authorization generation.
- Never resubmit a primary credential after a site reports invalid credentials
  without a new credential generation.
- Claim at most one challenge response per challenge ID.
- Do not request another SMS or email until the prior delivery effect and
  provider cursor are reconciled.
- Stop on ambiguity, provider disagreement, stale page or geometry evidence,
  target identity mismatch, rate limiting, lockout warning, or an unclassified
  state.
- Preserve one typed intervention with the durable remote-view handoff when
  human presence is required.

## Tenant Configuration

Tenant-private runtime configuration should select:

- exact profile and target service;
- opaque account reference;
- site recipe;
- credential-provider binding order;
- challenge-provider binding order;
- sender, allowed verification-link origin, and challenge-shape policy;
- remember-device permission;
- consent and human-only policy;
- deadline and attempt budget;
- incident and escalation route; and
- sensitive-evidence retention policy.

The repository contains schemas, examples with placeholders, and synthetic
fixtures. It does not contain tenant values, credentials, provider tokens,
message data, account labels, or authenticated browser artifacts.

## Acceptance Sequence

### Slice 1 | Contract and secret-canary fixtures

- Add the Authentication Run schemas and state-machine model.
- Add provider-neutral synthetic site, credential, and challenge adapters.
- Prove bounded transitions, idempotency, interruption recovery, and zero
  secret retention across all public and durable surfaces.
- Do not launch a browser or use a real provider.

### Slice 2 | Chrome-native fast path

- Use one disposable stock-Chrome profile with a synthetic local login site.
- Seed a synthetic saved credential through an approved fixture mechanism.
- Prove Agent Browser can cause Chrome's native fill and submit without reading
  the value.
- Prove ambiguity and password-manager unlock prompts stop without input.

### Slice 3 | Sealed primary credential provider

- Implement one local synthetic provider and the internal one-use field writer.
- Prove plaintext absence with a cross-surface sentinel.
- Add interruption tests before, during, and after dispatch.

### Slice 4 | Response-only challenge broker

- Implement separate fake cursor-based SMS-code and verification-link providers
  first.
- Prove watch-before-trigger ordering, uniqueness, expiry, replay refusal, and
  result verification.
- Bind SMS to im-receipts' existing urgent-watch and local-private exact-message
  surfaces.
- Add the narrow mail-receipts verification-link claimant, then prove internal
  same-profile new-tab navigation without exposing the URL.

### Slice 5 | Provider-specific recipes

- Add synthetic BILL-shaped and Intuit-shaped fixtures without copied private
  page content.
- Bind selectors and state classification only in narrow versioned recipes.
- Prove saved session, native autofill, SMS OTP, email device verification,
  remember-device, target mismatch, lockout, and typed human-intervention
  branches.

### Slice 6 | Separately authorized live acceptance

- Acquire one exact tenant-owned stock Chrome profile through the normal
  profile-acquisition contract.
- Run one bounded authentication attempt with no accounting or transaction
  effect.
- Keep private evidence in tenant runtime state and publish only a redacted
  acceptance receipt.
- Verify the exact target, preserve the browser, release the work lease, and
  confirm no secret reached a public or durable surface.

## Hard Stops

- Do not send a real password or OTP through generic `fill`, `type`,
  `ui_action`, `evaluate`, clipboard, desktop text, MCP, HTTP, CLI arguments,
  or an agent transcript.
- Do not send a device-verification URL through public `tab_new`, navigation,
  clipboard, MCP, HTTP, CLI arguments, or an agent transcript.
- Do not use broad message search as the production OTP provider.
- Do not infer authentication from successful input, a disappeared dialog, a
  ready browser, or a remembered device alone.
- Do not use DevTools virtual WebAuthn authenticators as production passkey
  credentials. Chrome documents them as an emulation and testing surface.
- Do not copy, replace, clear, or delete an authenticated profile to make a
  test deterministic.
- Do not place tenant identity, account selectors, credentials, message data,
  private screenshots, or raw provider URLs in repository fixtures or notes.
- Do not block unrelated P137 lifecycle work on this new capability.

## External Reference Points

- Chrome Password Manager can fill a single saved credential and supports an
  automatic sign-in setting:
  https://support.google.com/chrome/answer/95606
- Chrome Enterprise exposes `PasswordManagerEnabled` and documents that saved
  credentials may be provided on later sign-in:
  https://support.google.com/chrome/a/answer/2657289
- Chrome DevTools virtual WebAuthn authenticators are explicitly an emulation
  and testing facility:
  https://developer.chrome.com/docs/devtools/webauthn

## Best Next Action

Open one successor authentication lane beginning with Slice 1. Reuse P110's
challenge model, P137's acquired-profile outcome, the existing guarded desktop
authority, and Books Receipts recipe version 2. Model the exact BILL paths—
Chrome profile credentials, im-receipts SMS, and mail-receipts same-profile
device verification—from the first fixture. The first acceptance artifact
should be a no-launch synthetic OTP-and-URL canary test, not a live BILL or
Intuit login.
