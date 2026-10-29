# Standalone Auth Receipts integration specification

Version: 1
Product lane: PL-AUTH
Work item: [Agent Browser449](https://github.com/CochranResearchGroup/agent-browser/issues/449)
Status: specification; implementation and installed acceptance remain open
Provider acceptance: Auth Receipts81/91; parent Receipts20

## Problem Statement

An operator asks Agent Browser to sign in and continue a browser task. The browser
helpers still obtain their authentication client from the deprecated Bots
integration, even though standalone Auth Receipts owns the credential service.
An explicitly configured newer client can reach that authority, but ordinary
default discovery, workflow composition and actual sign-in submission are not
complete. A private credential release or dispatched form event can leave the
browser at login with an unknown outcome.

This is a planned integration migration. Existing failures provide acceptance
cases; they do not change the intended ownership or require a new authentication
product. All three components are developing. Contracts must distinguish existing
published interfaces, qualified developing source and installed workflow evidence.

## Solution

Agent Browser consumes the separately installed Auth Receipts service through its
supported verified client. An enrolled caller supplies service/account intent;
Auth resolves bounded metadata and places the exact request before the owner
without a preconfigured site permission rule. Auth applies the owner's decision
and required delivery gates before private use. The enrolled receiving adapter
privately fills and invokes the actual reviewed sign-in action in the exact owned
browser target. The browser workflow composes Auth's supported MFA operations,
independently verifies the intended signed-in identity, and continues the task.

Auth remains the credential authority. Parent Receipts handles configured
approval/notification channels. Agent Browser supplies browser lifecycle,
SiteGuide/routine composition and trustworthy browser-action evidence. Bots is
another consumer. Remote View retains desktop operation and the native human
viewer. Supported existing, managed persistent and disposable profiles retain
their ordinary continuity and ownership rules.

## User Stories

1. As a browser caller, I want authentication to work with standalone Auth Receipts and no Bots installation, so that deprecated software is not a prerequisite.
2. As an operator, I want the ordinary supported installation discovered automatically, so that each task needs no private SDK path workaround.
3. As an operator, I want verified producer identity and compatible capabilities checked before credential work, so that a stale or incompatible installation cannot silently execute it.
4. As a caller, I want actionable installation and compatibility diagnostics, so that missing software is distinguishable from denied permission.
5. As a caller, I want connection rotation handled by Auth's trusted client, so that service recovery does not expose bearer material or repeat effects.
6. As a caller, I want to name a service and intended account, so that I need not know private source/record identifiers before discovery.
7. As an operator, I want ordinary new-site requests to reach owner review without a bespoke allow rule, so that enrollment is one-time technical setup.
8. As an operator, I want genuine account ambiguity held for a non-secret choice, so that the system does not guess the account.
9. As an operator, I want bounded discovery metadata, so that requesting a sign-in does not disclose my vault inventory.
10. As an operator, I want Deny, Allow Once, Always Allow, 5 minutes, 15 minutes, 1 hour, 8 hours and Revoke preserved, so that the migration does not narrow my choices.
11. As an operator, I want authenticated web and configured Slack decisions applied by the same authority, so that channel selection does not change credential policy.
12. As an operator, I want enrollment and pending review to permit no protected reads or input, so that only an actual authorized decision can release credentials.
13. As an operator, I want current deny, revocation, expiry and caller restrictions enforced before effects, so that old permission cannot override changed authority.
14. As an operator, I want owner-created timed and standing permissions preserved, so that permitted reuse does not require another routine prompt.
15. As an operator, I want separate requested and released notifications retained during standing access, so that unattended use remains attributable.
16. As an operator, I want required notification delivery enforced independently of approval, so that a channel outage cannot silently bypass policy.
17. As a caller, I want repeated advancement to retain the original request, so that a timeout does not create another approval or credential use.
18. As a browser caller, I want receiving bound to my exact task, profile, browser, tab and generation, so that a replacement target cannot inherit authority.
19. As an operator, I want process ownership and cooperative privacy checked before protected input, so that a selected receiving path is actually usable and qualified.
20. As a browser caller, I want reviewed SiteGuide controls to describe page interaction, so that reusable site knowledge remains independent of credential authority.
21. As a caller, I want the actual Sign in action invoked after authorized private fill, so that an enabled button does not become routine manual work.
22. As an operator, I want native GET leakage prevented for JavaScript-managed forms, so that protected fields cannot be serialized into a URL.
23. As a caller, I want private fill, action dispatch, acknowledgement and authenticated identity distinguished, so that an event is not mistaken for completed sign-in.
24. As a caller, I want uncertain fill/submission retained across interruption and restart, so that recovery cannot blindly refill or resubmit.
25. As an operator, I want existing unknown attempts kept readback-only, so that the migration does not replay unresolved real effects.
26. As a caller, I want typed, positively established pre-input refusals distinguished from lost acknowledgements, so that safe original-request recovery remains possible.
27. As a caller, I want TOTP need discovered through Auth's public observation/continuation interface, so that the browser does not retrieve codes itself.
28. As a caller, I want TOTP continuation after SSO or session reuse without a fictitious password request, so that supported authentication journeys can start at their actual state.
29. As an operator, I want linked factor requests to preserve the same intended account and browser context, so that MFA cannot drift to another attempt.
30. As an operator, I want number-matching instructions and safe challenge imagery delivered through Receipts, so that external-factor participation uses the existing channel.
31. As an operator, I want device-to-browser factor input restricted to Auth's authenticated owner path, so that an ordinary caller cannot supply a number or approval.
32. As a caller, I want unsupported factors and missing capabilities reported explicitly, so that the workflow does not silently switch methods or authorities.
33. As a caller, I want independently observed account and requested organization verified, so that a saved profile or remembered identifier is not mistaken for successful authentication.
34. As an operator, I want secrets, codes, bearer material and private authentication URLs absent from model output and logs, so that integration preserves private custody.
35. As a caller, I want existing authenticated sessions reused when their identity is freshly verified, so that unnecessary credential effects are avoided.
36. As an operator, I want useful profiles and unrelated browser tasks preserved, so that authentication integration does not reset ongoing work.
37. As an operator, I want genuinely required intervention available through the existing native Remote View handoff, so that no dashboard or extra viewer ceremony is introduced.
38. As a caller, I want legacy checkpoints inspected under their original contract, so that a request identifier is not silently translated into another authority.
39. As an operator, I want newer email/SMS/link/passkey methods negotiated as capabilities, so that active provider development does not break the core integration.
40. As a maintainer, I want synthetic source qualification, installed qualification and real-provider acceptance reported separately, so that completion claims reflect the evidence.

## Implementation Decisions

### Ownership and supported bootstrap

- The Node authentication adapter consumes Auth's supported client exports and
  producer-owned verification. Browser helpers and the login playbook share this
  client/authority; neither implements a second broker, decision engine or vault.
- The default installation/discovery route belongs to Auth, not Bots. Auth must
  expose or publish the supported producer-owned installation locator and
  verification metadata before the consumer freezes a concrete resolver. Existing
  explicit installation selection remains a diagnostic override, not a per-task
  requirement. A source package export alone is not installed-client verification.
- Keep existing service discovery/connection compatibility names and private state
  locators where needed. Bots-named compatibility metadata does not require Bots
  to run. Renaming/migrating private state is outside this integration.
- Check SDK support and authenticated service capabilities independently. The
  existing shared-owner capability contract requires credential operations,
  shared authority and disabled parallel legacy credential effects. Record the
  selected installation/source identity without returning private capabilities.
  Optional factor and receiving capabilities require explicit qualification.
- Missing, unsafe or incompatible installation, unavailable service, unqualified
  receiving and genuine authorization denial remain distinct actionable outcomes.
  No fallback to legacy credential effects or unverified imports is permitted.
- Preserve Auth's current authenticated caller contract. Generalizing caller
  attestation beyond its supported hosts requires an Auth-owned prerequisite;
  this migration does not manufacture enrollment or promise every agent runtime.

### Intent, owner request and private use

- Compose existing Auth discovery, request, approval retrieval, inspection, use and
  reconciliation operations. Service/account intent resolves bounded candidates
  before request creation; callers do not need to invent internal identifiers.
  Candidate selection creates no grant. An absent site rule must not prevent
  owner review; explicit deny and ambiguous or stale metadata still hold.
- Preserve all original owner choices and their exact scope/expiry semantics.
  Owner-created saved permissions are reusable only within their approved scope.
  Caller fields cannot choose an approver, create policy, replace transport,
  select private paths or change a request into raw credential delivery.
- Auth revalidates current caller, policy, approved account, receiver and required
  delivery before protected source access/input, including after asynchronous
  waits. Parent acceptance/delivery and owner decisions remain independent facts.
- Retain distinct credential requests and actual releases with stable identities.
  Parent delivery retries cannot create another login. Existing Auth81 owner-entry
  and account-binding source must be reconciled into the adopted provider before
  ordinary rule-free new-site acceptance can pass.

### Receiving and actual sign-in action

- A reviewed prepared plan binds SiteGuide version, controls and expected state to
  the exact receiving task/profile/browser/tab and runtime generation. Fresh
  validation precedes private effects. Reviewed page knowledge cannot grant
  credential access, authorize an unrelated origin or replace owner enrollment.
- Use one Auth-owned private attempt to coordinate fill and submission. Agent
  Browser's trusted receiving port supplies guarded page action and evidence;
  it must not add an independent agent-visible submit retry after Auth use.
- Invoke the actual reviewed Sign in control or a qualified equivalent for the
  observed page. A generic form submit event is insufficient when the page's
  authentication operation belongs to a distinct button handler. Preserve native
  POST behavior and prevent native GET serialization for guarded JavaScript forms.
- Keep evidence dimensions separate: private input attempted/acknowledged,
  submission dispatched/acknowledged, observed authentication operation where
  available, MFA need, and independently verified identity. A dispatch receipt,
  navigation or absent network history cannot manufacture authentication or
  positively establish no effect. Exact wire additions must be jointly versioned
  in the Auth request/receiving contract and browser consumer.
- Before input, the selected installed receiving path must qualify process,
  profile, origin, target/generation, control ownership and cooperative capture
  privacy. An explicitly qualified browser-client route may be supported while
  native AT-SPI remains open; route selection never implies that finding is fixed.

### Continuation and recovery

- The playbook advances the existing owner lifecycle rather than sending legacy
  authentication operations. Persist its exact original request, target binding,
  reviewed plan, linked factor identities and effect intent before dispatch.
- Lost acknowledgements, timeout, cancellation and restart retain original
  authority/effect custody. Never regenerate credentials, renew a request, select
  another factor, refill or resubmit to discover whether an earlier effect ran.
  Retry eligibility requires a typed pre-input refusal and authoritative original
  readback proving the effect did not begin; transport loss alone is insufficient.
- Existing legacy checkpoints keep their original issuer/contract and may use
  supported inspection/reconciliation only. Do not translate their request IDs,
  clear attempted markers, or create a replacement owner request automatically.
  Missing compatibility becomes a typed hold with retained recovery evidence.
- For TOTP, observe current authentication need, then compose Auth's returned
  continue-authentication action into its ordinary request/approval/private-use
  lifecycle. This can start after SSO/session reuse. Link factor requests to the
  original browser workflow and verify the same account/recipient; unknown factor
  input reconciles that original factor request rather than generating a new code.
- Existing private-attempt following handles supported number-matching observation
  and reconciliation; it is not by itself the TOTP orchestration operation.
  Challenge delivery keeps text/image receipts, supersession, cancellation,
  freshness and authenticated reverse-input checks under Auth/parent ownership.
- Verify intended account and requested organization/receiving site from fresh
  independent browser evidence. Requested identity, login-field text, profile name,
  private fill or a provider's redirect alone cannot establish completion.
- Unsupported factors remain explicit. Spec110 email/SMS/link/passkey work is an
  optional capability extension and does not block the core migration. No silent
  fallback, enrollment, export or messaging-source access is added here.

## Testing Decisions

Test externally observable behavior at existing public seams. Assertions derive
from owner policy, expected website operations and independent identity evidence,
not private method calls, generated event counts or implementation-shaped mocks.
Use one failing behavior and minimal implementation per cycle; retain honest
regression evidence for behavior already working.

### Primary composition seam

Exercise the ordinary Agent Browser login workflow through the real supported
Auth client, broker and owner authority, composed with actual parent hub/relay
source and synthetic source/channel/browser effects. Existing pinned browser-owner
and shared-owner-parent compositions are prior art; connect the full consumer
workflow rather than inventing another authority fixture.

Required observations:

- Bots absent and no per-task SDK override: ordinary default bootstrap succeeds
  against an independently installed verified Auth candidate. Unsafe/missing SDK,
  incompatible capabilities and unavailable/rotated discovery fail appropriately.
- No site rule: bounded service/account discovery reaches one pending owner
  request. All original owner choices retain their semantics; pending approval,
  forged/stale/wrong-actor decisions and required delivery failure cause zero
  protected reads/input. Explicit deny/revoke and changed source accounts hold.
- Approval and delivery allow the intended single private use. Repeated advance,
  concurrent calls, interrupted transport and fresh-process restart preserve the
  original request without another protected use or channel message.
- TOTP and number-matching continuations retain account/target/attempt binding,
  private custody, policy/delivery gates and attributable linked requests. Unknown
  factor input cannot create another code request. Unsupported methods hold.
- Fresh independent identity is required for completion; wrong account/company,
  unqualified receiver, changed generation and capture hold cannot pass. Inspect
  model-facing output/logs for synthetic secret leakage.

This seam proves orchestration, authority and recovery with controlled external
effects. It does not prove actual page handlers, installed receiving or real Slack.

### Actual browser seam

Drive real Chrome against disposable synthetic HTTPS fixtures through the same
consumer/receiving path. Cover a native POST form, a JavaScript submit handler,
and a separate Sign in button handler that requestSubmit alone cannot execute.
The server independently records one intended authentication operation and the
resulting synthetic account. Check zero native GET leakage and zero duplicate
server authentication operations after lost fill/submit acknowledgements and a
fresh-process restart. Include a deliberately unhandled action and account/target
change: these remain held or unknown rather than falsely completed.

Use disposable profiles and exact owned-process cleanup. This seam establishes
actual browser behavior on those fixtures, not compatibility with every provider.

### Installed and provider acceptance

After source qualification and separately authorized adoption, preflight the exact
installed SDK/service and selected receiving path. Then run separately bounded
Stripe acceptance, preserving its existing unknown attempt as readback-only until
safely resolved. Require actual request/release delivery, private use, actual
submission, supported MFA and intended-account proof. The owner's participation
is release decisions and genuinely required external-factor actions; clicking the
ordinary Sign in button is not acceptance. Existing Auth81/91 and parent acceptance
remain explicit. Source tests cannot close those gates.

## Out of Scope

- A replacement credential authority, direct vault/code access from Agent Browser,
  channel callbacks in the browser, or a new viewer/access ceremony.
- Private-state migration, replacing Bots services, altering owner grants,
  resetting profiles, cleaning unrelated processes or retrying retained unknowns.
- Completing spec110 methods, general passkey enrollment, message acquisition or
  solving unsupported factors as part of the core migration.
- Product implementation, commits, runtime installation, service restart, real
  credential use, provider login, Slack sends or release approval by publication
  of this specification. These retain their applicable execution authority.

## Further Notes

The existing issue449 observations and acceptance criteria remain retained evidence.
This specification supplies the integration behavior and testing contract; it does
not erase failed attempts or convert source qualification into installed proof.

Provider checkpoints land before dependent consumer adoption: supported verified
Auth bootstrap and owner-entry/account-binding contract, then receiving action/
evidence parity, then browser playbook and factor composition, then exact installed
and separately scoped provider acceptance. This is an ordering constraint, not a
new ticket graph or permission to resume provider effects.

The readiness assessment records current source pins, developing branches and
fresh synthetic checks. Concrete SDK locator and evidence wire-shape agreement
remain provider-owned prerequisites; they are not invented by this specification.
The primary composition and actual Chrome seams retain issue449's existing
synthetic hosted and actual Chrome/synthetic HTTPS acceptance requirements.
Future changes to public capabilities require matching source/types/documentation
and meaningful public-seam validation in each owning repository.
