# Plan 0138 F1 | BILL Saved Credential Fieldwork Delta

Date: 2026-08-29

Status: FIELDWORK EVIDENCE READY | SUCCESSOR DEVELOPMENT OPEN

Lane: P110 Authentication Run

## Purpose

Record the Agent Browser development needs exposed when Books Receipts used a
tenant-owned stock Chrome profile to authenticate to BILL. This is a narrow
fieldwork delta to
`2026-08-29-books-receipts-unattended-authentication-handoff.md`; it does not
replace that consumer contract or Plan 0138's provider-free foundation.

The authoritative consumer receipt is Books Receipts commit
`f6e4d8969f0c6896b5946ffb7295e732e7c7f16d`, file
`docs/dev/validation/0231-r12-live-bill-authentication-fieldwork.md`.

## Proven Behavior

- The exact retained stock Chrome profile authenticated to the expected BILL
  company.
- Chrome's saved-credential row could be selected through native input. The
  password remained inside Chrome and was observed only as masked, nonempty
  page-field state.
- Page-level CDP input could not control Chrome's browser-chrome credential
  popup.
- The provider accepted login after an expired BILL login transaction was
  restarted from remembered identity state.
- One service-state lock timeout happened before native input delivery. A
  bounded retry was safe only because the fieldwork could establish that the
  first request had no effect.

The run did not exercise an SMS, device-email, account-selection, CAPTCHA, or
remember-device branch. It provides no acceptance evidence for those paths.

## Development Gaps

### Semantic native saved-credential action

`AuthenticationRun` needs a sealed browser-native action that can select a
saved credential in Chrome-owned UI without reading or accepting the password.
Generic page input, caller-supplied coordinates, clipboard input, and generic
text actions are not acceptable transports.

The action must bind:

- the exact principal, task, profile, browser, service session, and page;
- a fresh expected login origin and page state;
- a private, versioned site recipe;
- one unambiguous Chrome credential row or an opaque prebound selector; and
- a post-action observation that the target field is masked and nonempty,
  without reading its value.

An unlock prompt, multiple eligible credential rows, a password-manager
confirmation, or any ambiguity must return a typed intervention before input.

### Geometry and observation freshness

The fieldwork's first popup click missed after a BILL error banner changed page
geometry. The popup closed and the next provider submission contained no
password. A production action therefore needs semantic browser-chrome
perception, not a persisted coordinate recipe.

Every layout-affecting event must invalidate the observation used for native
input. The action must observe again after navigation, banner insertion,
viewport change, focus change, popup dismissal, or browser-chrome transition.
The effect receipt should bind the accepted observation generation to the
input operation.

### Input effect certainty

Authentication retry policy needs three explicit delivery states:

- `not_delivered`: failure happened before provider input and the same
  idempotent action may be retried within budget;
- `delivered`: native input acknowledgement is durably journaled and the run
  must observe provider state before deciding what follows; and
- `outcome_unknown`: the input may have been delivered and automatic replay is
  prohibited.

A service-state lock timeout must identify whether it occurred before effect
dispatch. The existing operation ledger and effect journal should carry this
proof; a generic timeout error is insufficient for a credential action.

### Provider transaction freshness

BILL can retain an expired login transaction while preserving remembered
identity state. The site recipe must recognize the expired-transaction page,
restart at a known login state, and establish a fresh page observation before
invoking the saved-credential action. A stale provider transaction is not a
credential failure and must not consume a credential retry or lockout budget.

## Recommended Successor Packet

Keep Plan 0138 Slice A as the accepted provider-free domain foundation. Open a
bounded successor slice that adds a synthetic stock-Chrome native credential
adapter behind the existing response-only action boundary. Do not expose a
public BILL action in that packet.

The packet should:

1. define a private semantic credential-row observation and freshness token;
2. bind native input to the exact authentication run and observation
   generation;
3. record `not_delivered`, `delivered`, or `outcome_unknown` through the effect
   journal;
4. verify only a masked, nonempty target field or a fresh provider transition;
5. add site-recipe recovery for an expired login transaction; and
6. keep all credential material out of requests, durable state, receipts,
   errors, traces, and debug projections.

## Provider-Free Acceptance

Use a disposable synthetic stock Chrome profile and fixture UI. Acceptance
must prove:

- one uniquely eligible saved credential can be selected without exposing its
  value;
- multiple rows, an unlock prompt, or an unknown popup fail before input;
- a layout shift invalidates old geometry and requires a new observation;
- a lock failure before dispatch returns `not_delivered` and permits exactly
  one bounded replay;
- a failure after dispatch returns `outcome_unknown` unless durable delivery
  acknowledgement exists;
- a repeated operation ID cannot emit a second native effect;
- an expired provider transaction restarts before credential selection;
- successful selection verifies masked, nonempty field state without reading
  the field; and
- synthetic password canaries are absent from every durable and debug surface.

Live BILL validation remains a later, separately authorized acceptance step.
It must not be used to manufacture the unobserved SMS or device-email branches.

## Hard Stops

- Do not read, export, copy, hash, log, or persist the saved password.
- Do not turn Guacamole coordinates into the public contract.
- Do not retry an input after `outcome_unknown`.
- Do not interpret a visible BILL home page as exact-account verification
  without the existing target verifier.
- Do not modify the retained tenant profile while building provider-free
  fixtures.
- Do not initiate an Agent Browser workstation upgrade for this work.

## Suggested Skills

- `agent-browser`
- `codegraph-workspace`
- `diagnosing-bugs`
- `tdd`

## Restart Point

Start from Plan 0138's source-accepted internal `AuthenticationRun` contract.
Inspect the native desktop interaction effect journal and operation ledger,
then design the smallest private adapter that can prove the acceptance cases
above. Preserve P110 lane ownership and do not widen into live provider work.
