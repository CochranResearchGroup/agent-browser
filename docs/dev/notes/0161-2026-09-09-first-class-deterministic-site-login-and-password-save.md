# Field Note 0161 | First-Class Deterministic Site Login And Password Save

Date: 2026-09-09

Status: OPEN PRODUCT GAP

Related work: P110 desktop perception and interaction, Plan 0138 authentication
run foundation, and the Books Receipts unattended-authentication handoff

Authority: documentation only. This note does not authorize implementation,
installed-runtime changes, provider effects, or another live login attempt.

## Decision

Entering a site account identifier and handling the browser's password-manager
Save, Never, and Update choices must be first-class Agent Browser facilities.
They belong in a versioned site-login recipe orchestrated by
`AuthenticationRun`, with the same deterministic, redacted, and replay-safe
contract as credential submission and second-factor handling.

Ordinary website login is not browser-profile or Google-account seeding. A
retained Chrome profile may already have the required browser identity while a
particular website session is expired or its credential is absent. Agent
Browser must model those states separately and must not classify routine site
authentication as manual profile seeding.

## Field Observation

Authorized accounting fieldwork exposed the missing contract without retaining
private credential values in this repository:

1. The exact managed profile, browser, session, display, and remote-view route
   were bound successfully.
2. The target site first required an account identifier. Only after that step
   did it reveal the password form.
3. The acquired profile did not contain the site's saved credential, although
   another already authorized managed profile did.
4. Generic desktop input could advance the site form, but it could not provide
   a deterministic or adequately redacted credential workflow.
5. Chrome displayed a native `Save password?` prompt after authentication. The
   prompt was outside the page DOM and partially clipped by the remote-view
   geometry, so page selectors and coordinate clicks could not safely select
   Save.
6. Exact authenticated-page readback succeeded and no second-factor challenge
   appeared. That success did not prove that Chrome persisted the credential.
7. A later profile-freshness recording attempt was rejected because the
   profile's shared-service allocation contained two naming variants. The
   failure demonstrates that login completion and profile metadata repair need
   separate typed outcomes.

The reusable lesson is that successful page authentication is not equivalent
to successful browser credential persistence. Both effects require their own
receipts and verification.

## Required First-Class Workflow

A site-login recipe must be able to perform this ordered workflow without raw
secrets in public actions, logs, transcripts, screenshots, or durable state:

1. acquire and attest the exact profile, browser, session, tab, display, and
   target origin;
2. recognize the expected account-identifier form;
3. resolve an opaque account reference through an approved sealed provider and
   submit the identifier;
4. recognize the resulting password form and submit the credential through
   native autofill or another policy-approved response-only provider;
5. observe and classify any site or browser-level account-choice, password
   manager, unlock, Save, Never, or Update prompt;
6. apply the tenant policy for credential persistence;
7. arm second-factor providers before triggering delivery, then handle an SMS
   code through IM Receipts or a verification link through Mail Receipts when
   required;
8. handle an approved remember-device choice;
9. verify the exact authenticated service and account; and
10. verify, independently and without reading the secret, whether the intended
    password-manager persistence effect occurred.

The recipe must stop with a typed intervention when the account, prompt,
origin, browser window, persistence policy, or post-effect state is ambiguous.

## Contract Additions

The successor authentication lane should add sealed internal capabilities for:

- account-identifier submission before password-form discovery;
- native saved-credential selection and submission;
- authorized credential transfer or import between explicitly governed
  managed profiles without exposing plaintext to tool arguments;
- Chrome password-manager prompt observation through browser-chrome or desktop
  accessibility semantics;
- policy-bound Save, Never, and Update selection;
- persistence verification that confirms presence and account binding without
  returning a password; and
- profile authentication-freshness recording after the login result is known.

These are internal capability descriptions, not final public action names.
The public surface should continue to accept only an opaque account reference,
site-recipe reference, policy reference, exact profile reference, deadline, and
idempotency key.

## Determinism Requirements

The implementation must:

- bind every effect to the exact process, profile, browser, session, display,
  window, tab when applicable, origin, account reference, and observed form or
  browser-chrome state;
- locate browser-chrome prompts by stable accessibility role, name, state, and
  owning window rather than viewport-relative coordinates;
- remain correct when the page viewport clips or excludes browser chrome;
- distinguish no prompt, Save prompt, Update prompt, unlock prompt, ambiguous
  prompt, and already-persisted outcomes;
- record prepared, acknowledged, uncertain, verified, declined, and
  intervention outcomes with effect idempotency;
- prove that retries cannot double-submit a login, consume a second factor
  twice, or reverse a previous password-manager choice;
- prevent generic `fill`, `type`, `evaluate`, clipboard, keyboard automation,
  or desktop text APIs from becoming secret transports; and
- canonicalize or explicitly reject shared-service naming aliases before a
  metadata update, without changing the already verified login outcome.

## Acceptance Evidence

Before live provider use, a provider-free headed fixture must prove:

- identifier page to password page to authenticated page sequencing;
- native autofill and an approved response-only credential-provider path;
- Save, Never, Update, unlock, missing, clipped, and ambiguous native prompts;
- password persistence verification without secret disclosure;
- watch-before-trigger SMS and email-link challenges with exact candidate
  fencing;
- exact-account authenticated readback and wrong-account rejection;
- crash, resume, replay, cancellation, and effect-uncertain handling;
- secret canaries absent from CLI, HTTP, MCP, generated clients, dashboard,
  service jobs, logs, screenshots, and durable receipts;
- CLI, HTTP, MCP, generated-client, dashboard, help, README,
  documentation-site, and skill parity for any public action; and
- profile freshness recording that either succeeds with canonical service
  identity or returns a typed pre-effect conflict.

A live acceptance packet must additionally prove the same behavior on one
explicitly authorized test account and profile, including a native
password-manager prompt visible outside the DOM. Page-login success alone is
not sufficient acceptance.

## Planning Relationship

Plan 0138 remains closed as the provider-free authentication-run foundation.
This note is an input to its successor implementation lane and extends the
Books Receipts unattended-authentication handoff. The successor plan should
freeze the identifier and password-manager state machines before adding a
public action or repeating live accounting fieldwork.
