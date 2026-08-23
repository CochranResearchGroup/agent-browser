# P110 Fieldwork 1 | Passkey And Two-Factor Authentication Orchestration

Date: 2026-08-23

Fieldwork date: 2026-08-22

Classification: OPERATOR-AUTHORIZED LIVE FIELDWORK

Product status: EVIDENCE RECORDED | ABSTRACTION NOT IMPLEMENTED

Roadmap: P110 Desktop Perception And Interaction Foundation

## Outcome

An operator-authorized accounting-service authentication run successfully
selected one uniquely matched LastPass passkey from a browser-external popup
and reached the authenticated application home.

The run combined:

- one retained service-owned browser and runtime profile;
- one ready Guacamole and RDP presentation bound to an X11 display;
- fresh service-owned desktop frames;
- Tesseract OCR and deterministic crop geometry;
- X11 window and geometry readback;
- one `xdotool` and XTEST pointer click with no retry;
- a fresh after-frame plus retained service-tab readback.

This result proves that the current service-owned display, capture, and route
seams can support a real browser-external authentication workflow on this
workstation. It does not prove the current P110 production-provider contract.
The click did not pass through the canonical `desktop_interact` action, no
Agent Browser input provider owned the effect, and the experiment did not use
the required controller lease or cross-process input fence.

P110 live Foundation Acceptance therefore remains blocked. This note records
fieldwork evidence for the separately planned productization pass.

## Source And Privacy Boundary

The authoritative fieldwork receipt remains in tenant-private runtime state.
Its stable artifact identifier is:

`qbo-passkey-xdotool-feasibility-20260822T124919Z`

The private receipt digest at closeout was:

`62b1fb9a721229a3adbc04ea5dd99b5c8190fbfcd66a0eb93e3e759419fc6e1a`

This repository note intentionally omits:

- the tenant profile and organization names;
- the account email and LastPass account label;
- private screenshot pixels and OCR transcripts;
- the runtime-home path;
- raw Guacamole or provider URLs;
- browser-auth state, cookies, passkey material, and one-time codes;
- mailbox message identifiers and message bodies.

The note retains only the minimum redacted facts needed to design reusable
product contracts. Deleting the private runtime home would leave this
repository coherent, while removing the private evidence needed to audit the
specific live account run.

## Systems Touched During Fieldwork

The live run touched these bounded systems:

1. Agent Browser service state for one already running retained browser.
2. The browser's existing RDP and Guacamole route on one X11 display.
3. The public login and authenticated application surfaces.
4. The LastPass Chromium extension popup.
5. A read-only Mail Receipts Gmail source through bounded auth-code watches.
6. Ubuntu package state to install `xdotool` and `libxdo3`.
7. Tenant-private runtime artifacts and tenant-local workflow memory.

The run did not change Agent Browser source, install a candidate Agent Browser
generation, replace the retained browser, release the route, expose a raw
provider URL, or persist a passkey or one-time code.

## Installed Fieldwork Tools

The workstation installed these Ubuntu Noble packages for the experiment:

- `xdotool` version `3.20160805.1`, package revision
  `1:3.20160805.1-5build1`;
- `libxdo3` package revision `1:3.20160805.1-5build1`.

Tesseract `5.3.4` and the existing ImageMagick desktop-capture path supplied
the image-processing tools.

`xdotool` was an effective experiment driver. It is not the recommended
public abstraction. A production provider must own display selection, window
ownership, focus, controller authority, operation replay, event fencing,
cleanup, and receipt generation. A raw subprocess with `DISPLAY` and screen
coordinates cannot establish those contracts by itself.

## Observed Authentication State Machine

The live site did not expose one stable login sequence. The same account and
profile moved through several distinct branches. A future orchestrator must
classify the current state before choosing an action.

### State 1 | Initial Identifier Form

The first fresh frame showed an identifier field with the expected account
already filled and one sign-in button. The LastPass passkey chooser was no
longer open, so coordinates from an earlier frame were rejected as stale.

Selecting the sign-in button moved to email verification rather than directly
to a passkey choice.

Lesson: account identity and browser profile do not uniquely determine the
next challenge. The workflow must observe the resulting state after each
transition.

### State 2 | Email Verification Required

The site displayed a six-digit verification-code field and a control labeled
as an email-delivery recovery action.

The first Mail Receipts auth-code watch started after the email had already
arrived. Its starting cursor therefore fenced the existing message out, and
the watch expired without a candidate.

Lesson: a one-time-code provider must enter a ready watching state before the
browser triggers code delivery. Provider readiness and delivery trigger are
separate barriers.

### State 3 | Verification Method Selector

Selecting the email-delivery recovery control did not send another email. It
opened a method selector with an email option and an alternate identity path.

A second auth-code watch correctly observed a later mailbox cursor but still
expired without a code because no resend occurred.

Lesson: visible labels such as “didn't get an email” are intent hints, not
effect proof. The orchestrator must verify that the next browser state or
provider cursor confirms delivery.

### State 4 | Account Recovery, Not Passkey Authentication

The alternate identity path opened an account-recovery application that
requested identity-document workflow choices. The run classified this as the
wrong branch and backed out without entering data.

Lesson: “verify another way,” “recover account,” and “use passkey” are not
interchangeable. A challenge classifier must preserve their different risk,
authority, and completion semantics.

### State 5 | Remembered-Account Selector

Returning through browser history exposed a remembered-account card. Selecting
that card produced the expected verification-options surface.

Lesson: remembered-account selection is a distinct transition from submitting
an identifier form. Site adapters may need both states even when they refer to
the same account.

### State 6 | Passkey Choice And Browser-External Popup

The page showed passkey and password options. At the same time, LastPass
opened a browser-external “Log in with a passkey” chooser above the page.

The page DOM and page screenshot could not establish the full chooser row.
The service-owned desktop frame did. The fieldwork locator required all of the
following before input:

- the expected chooser title appeared exactly once;
- the relying-party domain appeared exactly once in the credential-row crop;
- the configured account identity appeared exactly once in that crop;
- the desktop dimensions and scale matched the fresh service context;
- the geometry epoch had not changed;
- the active X11 window title remained the expected login window;
- the active X11 window remained at the expected desktop geometry;
- the chosen pointer coordinate remained inside the matched row crop.

The run dispatched one left click to the matched row. It did not retry.

### State 7 | Authenticated Application

A fresh desktop capture showed the authenticated application home. The
retained service tab independently reported its authenticated home title and
application origin. Browser health and route attachability remained ready.

Lesson: input dispatch is not authentication proof. A verifier must observe a
postcondition from fresh desktop evidence and, when available, an independent
service-tab or provider readback.

## What The Experiment Proved

The fieldwork supports these bounded claims:

1. A service-bound X11 root frame can expose a LastPass popup that page-level
   CDP evidence does not fully represent.
2. OCR plus deterministic geometry can identify one credential row when the
   relevant relying-party and account predicates are unique.
3. Cropping and upscaling can materially improve small browser-extension text
   recognition compared with full-frame OCR.
4. XTEST input can select the matched row on the current X11-backed RDP lane.
5. A single-dispatch, no-retry posture can complete the passkey selection and
   preserve a clear effect boundary.
6. Fresh after-state capture plus retained tab readback can verify the
   authenticated result without recording a secret.
7. Mailbox cursor fencing matters: starting an auth-code watch after delivery
   misses the earlier message by design.
8. A visible browser control does not prove that an email, SMS, push, or other
   second-factor delivery occurred.

## What The Experiment Did Not Prove

The run does not establish any of the following:

- a production-ready X11 input provider;
- Agent Browser controller-lease enforcement for the native click;
- cross-process event fencing or durable effect replay through the service;
- a general LastPass detector across themes, versions, layouts, scrolling,
  multiple credentials, or other browsers;
- passkey completion when a biometric, PIN, master-password, secure-desktop,
  consent, or operating-system prompt follows selection;
- safe automation of every accounting-service login state;
- general email one-time-code extraction;
- provider-independent authentication completion;
- deterministic acceptance by an external identity provider;
- P110 live Foundation Acceptance or release readiness.

The experiment also retained screenshots under explicit operator authority.
That exception must not weaken P110's product default: sensitive desktop
frames remain response-only and ephemeral unless a separate retention policy
explicitly authorizes a bounded evidence artifact.

## Productization Classification

Disposition: REFACTOR BEFORE KEEP

The live procedure is repeatable enough to inform product design, but its
current shell harness is not a product interface. Productization should keep
the service-owned capture and verification concepts, replace raw coordinates
and subprocess effects with registered providers and recipes, and move
site-specific or credential-manager-specific rules to narrow adapters.

The durable home should be split across:

- core Agent Browser contracts for challenge state, orchestration, authority,
  perception, input, verification, and receipts;
- credential-provider adapters for LastPass and later password managers;
- second-factor adapters for email, SMS, TOTP, push, and manual approval;
- site recipes for page-state recognition and safe transitions;
- tenant runtime configuration for account selectors, allowed providers,
  retention policy, and operator approval requirements;
- repository-owned synthetic fixtures and redacted acceptance corpora.

## Proposed Service-Agnostic Architecture

The following names describe candidate responsibilities. They are not shipped
commands, schemas, or endpoints.

### 1. Authentication Challenge Observation

`AuthenticationChallengeObservation` should classify the current challenge
without granting effect authority.

Candidate fields:

```json
{
  "challengeId": "opaque",
  "runId": "opaque",
  "kind": "passkey | password | email_otp | sms_otp | totp | push | recovery | consent | unknown",
  "surface": "page | browser_extension | browser_chrome | native_dialog | secure_desktop | external_device",
  "state": "matched | not_found | ambiguous | operator_intervention_required",
  "targetServiceRef": "opaque",
  "accountRef": "opaque",
  "sourceContextId": "opaque",
  "sourceFrameId": "opaque",
  "geometryEpoch": "opaque",
  "candidateIds": ["opaque"],
  "selectedCandidateId": null,
  "allowedTransitions": ["opaque-policy-action"],
  "sensitivity": "private_authentication"
}
```

The challenge kind describes what the site requests. The surface describes
where the evidence lives. These axes must remain separate. A page can request
a passkey while the selectable credential lives in an extension popup or
native dialog.

### 2. Authentication Run State Machine

`AuthenticationRun` should own the ordered state transitions and stop reasons
for one login attempt.

It should bind:

- service, agent, task, and accountable principal;
- target service and opaque account selector;
- service-owned browser, session, profile, and tab;
- current desktop context and controller authority when desktop input is used;
- challenge observations and transition receipts;
- provider jobs and delivery cursor fences;
- retry budget and non-retryable partial effects;
- operator intervention and durable handoff state;
- authenticated-state verifier and terminal result.

A run should never infer success from “click returned zero.” Terminal success
requires an explicit authenticated-state verifier. Terminal failure should
retain a typed reason and safe next action without retaining secrets.

### 3. Credential Provider Adapter

A `CredentialProviderAdapter` should expose capabilities rather than raw vault
contents.

Candidate operations:

- inspect whether the provider can handle the observed challenge;
- return redacted candidate metadata;
- select one candidate by relying party and opaque account selector;
- request operator intervention for master-password, biometric, consent, or
  ambiguous selection;
- report provider completion or a typed stop reason.

The core should not contain LastPass title text, icons, row geometry, or
account-label rules. A LastPass adapter should own those details through a
versioned visual and semantic profile. Other adapters can target browser-native
passkeys, KeePassXC, another extension, or a manual operator workflow.

For passkeys, the relying-party identifier is a stronger match key than page
branding alone. The adapter should require an exact relying-party match and an
opaque configured account selector. A visible account string may be used
inside the adapter for matching, but it must not enter service logs or durable
receipts.

### 4. Second-Factor Provider Adapter

A `SecondFactorProviderAdapter` should separate readiness, trigger, delivery,
extraction, consumption, and verification.

Candidate phases:

1. `prepare_watch`: establish provider readiness and a starting cursor.
2. `watch_ready`: return a receipt proving the provider can observe only later
   deliveries.
3. `trigger_delivery`: let the site adapter request one delivery action.
4. `await_delivery`: require provider evidence beyond the starting cursor.
5. `extract_candidate`: apply sender, subject, body, code-shape, code-length,
   age, and uniqueness rules.
6. `consume_response_only`: send the code directly to the browser action
   without durable plaintext persistence.
7. `verify_result`: confirm that the site accepted the factor or returned a
   typed rejection.
8. `close_watch`: release urgency leases and record a redacted terminal state.

The live Mail Receipts work exposed two requirements:

- start the watch before requesting delivery;
- verify that the browser action caused delivery rather than merely opening a
  method selector.

Earlier rehearsal evidence also showed that raw HTML and CSS tokens can look
like short alphanumeric codes. An email adapter therefore needs MIME-aware or
rendered-body extraction plus explicit code shape and length. For a numeric
six-digit challenge, alphabetic CSS tokens must not become peer candidates.

### 5. Desktop Perception Pipeline

The P110 perception pipeline should keep these stages independently testable:

1. resolve one `DesktopContext` from service state;
2. capture one fresh frame and `FrameReceipt`;
3. identify the active window and browser-external prompt region;
4. classify prompt type and provider;
5. derive candidate crops from detected geometry;
6. apply deterministic preprocessing;
7. run one or more registered semantic or visual detectors;
8. normalize detector outputs into candidate evidence;
9. require a unique policy-eligible candidate;
10. map candidate-local bounds to physical desktop coordinates;
11. recheck context, geometry, focus, and controller authority;
12. dispatch one registered input recipe;
13. recapture and verify the resulting state.

No stage should smuggle effect authority into observation. No caller should
supply a raw display name, raw provider URL, arbitrary pixels, or arbitrary
screen coordinates to bypass service resolution.

### 6. Input Provider Adapter

P110 already anticipates replaceable input sinks. This fieldwork adds concrete
evidence for an X11 implementation.

Candidate providers include:

- X11 XTEST for an owned X11 route display;
- Guacamole input for a route-bound remote desktop;
- Windows SendInput under an owned desktop session;
- Wayland RemoteDesktop portal input with explicit portal authority;
- macOS accessibility or event input under an approved session;
- manual attached desktop, which exposes operator continuation without machine
  effects.

An X11 provider should call XTEST through a native or tightly controlled
library boundary. It should not expose `xdotool` command construction as the
public contract. The provider must attest:

- exact display and display-allocation identity;
- browser and X11 window ownership;
- active window and focus state;
- current controller lease and epoch;
- frame and geometry epoch used for location;
- server-derived input coordinates;
- operation ID and provider effect key;
- attempted and acknowledged event counts;
- release cleanup after partial failure;
- after-state verification.

### 7. Authentication Verifier Adapter

Authentication verification should be independent of the provider that emits
input. Candidate evidence may include:

- URL and title classification through a valid service tab handle;
- a bounded page-state probe;
- a fresh desktop-state classifier;
- profile freshness update after a verified authenticated read;
- a provider completion receipt;
- a site-specific account or organization label checked inside a private
  verifier and reduced to a redacted boolean result.

The verifier should return `verified`, `rejected`, `inconclusive`, or
`operator_intervention_required`. It must not treat a disappeared popup as
sufficient proof by itself.

## Deterministic Image-Processing Requirements

The fieldwork exposed several concrete requirements for a production locator.

### Freshness And Binding

- Bind every candidate to one frame ID, context ID, geometry epoch, scale, and
  coordinate space.
- Reject candidates after any window, route, display, scale, crop, or geometry
  change.
- Re-observe immediately before input. Never reuse coordinates from an older
  popup frame.
- Verify that the active window belongs to the service-owned browser before
  mapping prompt coordinates to desktop input.

### Prompt Region Detection

- Detect the popup region from window or visual evidence instead of fixing one
  global screen crop.
- Retain the source region, crop transform, and desktop transform in the
  observation receipt.
- Distinguish browser-extension popup, page modal, browser chrome, native
  dialog, and secure-desktop surfaces.
- Treat scrolling or clipped provider rows as explicit states.

### OCR And Semantic Matching

- Run OCR on a bounded candidate region rather than depending only on
  full-frame OCR.
- Support deterministic preprocessing profiles such as grayscale conversion,
  contrast stretch, integer upscaling, and fixed segmentation mode.
- Version the OCR engine, language data, preprocessing profile, page
  segmentation mode, and normalization rules.
- Normalize case and whitespace without weakening exact relying-party or
  account matching.
- Require exact expected-token counts after normalization.
- Keep OCR confidence as evidence, not as authority to break a tie.
- Return `ambiguous` when two eligible rows or two plausible identities remain.

In the live frame, full-frame OCR read the small account text incorrectly.
Cropping the credential row, upscaling it three times, converting it to
grayscale, and applying a fixed contrast stretch produced exact domain and
account text under multiple Tesseract segmentation modes. That improvement is
a detector-profile lesson, not a reason to embed the live crop or its screen
coordinates in product code.

### Visual And Structural Fusion

OCR alone should not own selection. A production LastPass profile should fuse:

- popup title and provider chrome;
- relying-party text;
- account-label text;
- row bounds and containment;
- icon or template evidence where stable;
- scrollbar and visible-row state;
- active-window or accessibility metadata where available;
- negative evidence for alternate-passkey and generic-options rows.

The selected point should come from the candidate row's safe interior, not
from OCR text coordinates alone. The receipt should retain candidate bounds
and a server-derived point while omitting private recognized text.

### Fixture Corpus

A repository-owned corpus should cover:

- one eligible LastPass row;
- two eligible rows for an ambiguity stop;
- correct domain with wrong account;
- correct account with wrong relying party;
- alternate-passkey and generic-options decoys;
- popup absent while the page offers passkey;
- page modal that visually resembles the extension popup;
- crop clipping and popup scrolling;
- scale changes, window movement, and geometry-epoch changes;
- light and dark themes;
- OCR corruption and low-confidence text;
- provider version or layout drift;
- popup disappearance before input;
- focus loss, route replacement, and controller-lease conflict;
- successful input followed by failed authentication verification.

All fixtures must be synthetic or reviewed and redacted. Do not derive a
committed fixture by copying a private fieldwork screenshot.

## LastPass Adapter Requirements

A future LastPass adapter should own these responsibilities:

- identify supported LastPass popup versions and browser families;
- classify the popup independently from the underlying page;
- expose only redacted credential candidates;
- match exact relying-party ID and opaque account selector;
- reject generic options and alternate-passkey rows;
- detect multiple matching credentials;
- expose scrolling as a bounded adapter operation, not an arbitrary wheel
  command;
- stop for master-password, biometric, PIN, consent, or vault-unlock prompts;
- verify popup disappearance but delegate site authentication verification to
  the site verifier;
- retain provider version, detector version, candidate ID, bounds digest, and
  typed outcome without recognized private text.

The adapter should not know how to fetch an email code or how to classify one
specific site's page. Those responsibilities belong to the second-factor and
site adapters.

## Site Adapter Requirements

A site adapter should model page-level states and transitions without owning
desktop input implementation.

For the fieldwork sequence, the site adapter would distinguish:

- identifier form;
- remembered-account selector;
- verification method selector;
- email-code entry;
- explicit passkey or password choice;
- account recovery;
- authenticated application;
- unknown or changed state.

Each transition should specify:

- required observation evidence;
- allowed browser action;
- expected next states;
- whether it triggers an external provider effect;
- whether the effect is retryable;
- whether operator approval is required;
- the verifier that closes the transition.

Site recipes may include selectors and public UI labels. Core Agent Browser
contracts must remain service agnostic and must not contain site-specific
selectors or coordinates.

## Failure And Stop Taxonomy

A first-class authentication orchestrator needs typed stops that preserve the
next safe action. Candidate reasons include:

- `authentication_state_not_recognized`;
- `authentication_transition_changed`;
- `passkey_not_offered`;
- `credential_provider_popup_not_found`;
- `credential_provider_popup_ambiguous`;
- `credential_provider_version_unsupported`;
- `relying_party_mismatch`;
- `account_selector_mismatch`;
- `multiple_credential_candidates`;
- `desktop_context_stale`;
- `geometry_epoch_changed`;
- `active_window_mismatch`;
- `controller_lease_missing`;
- `controller_epoch_changed`;
- `input_provider_unavailable`;
- `input_partial_effect_non_retryable`;
- `operator_secret_required`;
- `biometric_or_pin_required`;
- `secure_desktop_required`;
- `operator_consent_required`;
- `second_factor_watch_not_ready`;
- `second_factor_delivery_not_observed`;
- `second_factor_candidate_not_found`;
- `second_factor_candidate_ambiguous`;
- `second_factor_expired`;
- `second_factor_rejected`;
- `authentication_after_state_unverified`;
- `authentication_rejected`;
- `operator_intervention_required`.

These reasons should not automatically become retries. Each site and provider
policy should declare whether the reason is terminal, operator-actionable, or
eligible for a bounded fresh observation.

## Receipt And Privacy Requirements

A durable authentication receipt should include only redacted operational
metadata:

- opaque run, challenge, operation, provider, candidate, and account refs;
- service, agent, task, principal, browser, session, profile, tab, route,
  display-allocation, and controller refs where policy permits;
- before and after frame receipts without pixel bytes;
- detector and preprocessing profile versions;
- candidate-count and uniqueness decisions;
- geometry, focus, controller, and provider attestations;
- attempted and acknowledged input counts;
- provider delivery cursor fences and redacted candidate fingerprints;
- transition and verifier outcomes;
- cleanup, handoff, and non-retryable partial-effect state;
- explicit retention and redaction policy.

Do not retain:

- passwords, passkey material, private keys, recovery codes, or OTP plaintext;
- recognized account labels or mailbox message bodies;
- private screenshot pixels by default;
- clipboard contents, extension vault payloads, or browser cookies;
- raw input trajectories when a digest and bounded event summary suffice;
- raw provider URLs or local display names as caller-controlled authority.

## Candidate First-Class Product Surface

The product should expose task-shaped authentication orchestration rather than
making callers assemble raw capture, OCR, and pointer commands.

One candidate service shape is:

1. observe or resume one authentication run;
2. return the current challenge, allowed transitions, and any operator
   intervention;
3. execute one registered transition recipe under current authority;
4. await one configured credential or second-factor provider when needed;
5. verify the resulting authentication state;
6. return a redacted run receipt.

Simple CLI, HTTP, MCP, generated-client, dashboard, and agent-facing helpers
should lower into the same service action and state machine. Provider adapters
should not add parallel ingress-specific execution paths.

Capability discovery should report at least:

- observable challenge surfaces;
- configured credential providers;
- configured second-factor providers;
- available frame, semantic, locator, input, and verifier providers;
- controller and operator-handoff posture;
- supported challenge kinds;
- secret-disclosure and artifact-retention policy;
- whether a requested run can proceed automatically, needs an operator, or is
  unavailable.

The dashboard should show typed authentication state and operator action. It
should not expose raw OCR text, OTP plaintext, vault contents, provider URLs,
or private pixels in ordinary service history.

## Recommended Productization Sequence

### Slice A | Freeze Redacted Contracts And Fixtures

- Write a separately authorized P110 controlled-provider plan.
- Define challenge, run, provider, transition, and verifier contracts.
- Build synthetic site, LastPass, email-code, ambiguity, and failure fixtures.
- Keep configured production capture and input unavailable.

### Slice B | Read-Only Authentication Observation

- Compose page-state, desktop-prompt, and provider classification.
- Return one typed challenge observation or operator intervention.
- Add no effect and no secret disclosure.
- Validate CLI, HTTP, MCP, generated client, dashboard, help, skill, and docs
  parity.

### Slice C | Controlled X11 Input Provider

- Implement an XTEST provider behind `desktop_interact`.
- Require installed-binary identity, control-plane attestation, current
  controller lease, cross-process effect fencing, and a repository-owned RDP
  fixture.
- Prove success, replay, partial effect, cleanup, focus loss, geometry change,
  route replacement, and failed verification.
- Do not use a real credential or external identity provider.

### Slice D | Synthetic LastPass Adapter

- Implement the versioned LastPass detector and candidate policy against
  synthetic fixtures.
- Keep account refs opaque and omit recognized text from durable receipts.
- Exercise one row, ambiguity, mismatch, scrolling, popup loss, and operator
  secret stops.

### Slice E | Second-Factor Orchestration Adapter

- Define provider readiness, cursor fence, delivery trigger, candidate
  extraction, response-only consumption, verification, and close contracts.
- Validate with a fake mailbox provider before integrating Mail Receipts.
- Add code shape, length, sender, subject, age, MIME-body, and uniqueness
  constraints.

### Slice F | Separately Authorized Live Acceptance

- Install the exact candidate generation through the transactional workstation
  workflow.
- Use one controlled account and explicit operator authority.
- Keep private evidence in governed runtime state.
- Prove service-owned input, controller serialization, cross-process replay,
  provider redaction, operator handoff, and authenticated after-state.
- Update P110 only if the controlled provider satisfies its existing live
  Foundation Acceptance criteria.

## Open Design Questions

1. Should authentication orchestration be one long-running service action or a
   durable run resource with one queued transition action at a time?
2. Which component owns retries when a site transition is idempotent but the
   provider effect is not?
3. How should a provider declare that a biometric, PIN, master-password, or
   consent step is manual-only?
4. Can the current controller lease fully fence XTEST input, or does the X11
   provider need an additional display-local interprocess lock?
5. Which accessibility or extension metadata can supplement pixels without
   requiring DevTools access to extension internals?
6. How should candidate account matching remain exact while durable receipts
   retain only opaque account refs?
7. Should one-time-code plaintext cross the service process boundary, or
   should a provider fill the browser field internally and return only a
   redacted completion receipt?
8. How should the orchestrator prove that a delivery trigger produced a new
   message when the provider cursor advanced for an unrelated message?
9. Which fixture transformations are required before a LastPass adapter can
   claim layout-version compatibility?
10. Which authenticated-state verifier is authoritative when page, desktop,
    and provider evidence disagree?

## Decisions Preserved From P110

This fieldwork does not change the following frozen P110 decisions:

- desktop work remains part of Agent Browser;
- callers do not choose raw displays or provider URLs;
- observation remains separate from input authority;
- machine input requires current control authority and human serialization;
- ambiguous targets stop without input;
- sensitive frames remain ephemeral by default;
- deterministic evidence does not imply deterministic external acceptance;
- source, installed, live, and release proof remain separate;
- use-case-specific heuristics stay outside the core engine;
- durable operator continuation uses the opaque remote-view handoff.

## Next Recommendation

Use this note as input to a separately authorized P110 controlled-provider
plan. Start with synthetic LastPass and second-factor fixtures plus an XTEST
provider contract. Do not normalize the fieldwork shell harness into product
code, and do not use another real credential workflow to discover the missing
contracts.

## Related Repository Authorities

- `ROADMAP.md` P110
- `VISION.md`
- `docs/dev/plans/0110-2026-08-12-desktop-perception-interaction-foundation-plan.md`
- `docs/dev/plans/0110-1-2026-08-12-p110-poc1-display-bound-frame-capture-plan.md`
- `docs/dev/plans/0110-2-2026-08-12-p110-poc2-deterministic-fixture-location-plan.md`
- `docs/dev/plans/0110-3-2026-08-12-p110-poc3-guarded-desktop-interaction-plan.md`
- `docs/dev/plans/0110-4-2026-08-12-p110-poc4-browser-external-prompt-perception-plan.md`
- `docs/dev/plans/0110-5-2026-08-12-p110-poc5-foundation-stress-and-entry-gate-plan.md`
- `docs/dev/notes/0110-5-2026-08-12-foundation-stress-source-acceptance.md`
- `docs/dev/notes/2026-04-22-agent-browser-service-roadmap.md`
- `docs/dev/notes/2026-05-09-access-plan-service-request-handoff.md`
- `docs/dev/notes/2026-08-21-service-control-plane-attestation-source-acceptance.md`
