# Governed Google and Microsoft sign-in

Read this before Google/Microsoft sign-in, credential-helper use, number matching,
or resuming a saved authenticated profile. Reuse the current owned browser and
profile. Account bindings and private paths stay in user configuration. Passwords,
OTPs and recovery codes stay in private service custody; number-matching display
values follow the challenge-delivery workflow below.

## Authority first

1. Verify the exact task, profile, browser, selected window/tab and intended
   account. Reuse an authenticated session when current identity evidence matches.
   An open inbox or familiar URL alone does not establish the requested account.
2. Use the enrolled installed Auth caller and receiving path. A valid owner
   standing rule may avoid another approval question; it still requires distinct
   requested and released notifications through parent Receipts. Otherwise wait
   for the correlated owner decision. Parent acceptance, Slack delivery, approval,
   credential release and authentication are different facts.
3. Discover the current routine/host contract before effects. Use the existing
   [login composition](playbooks.md) when its receiving mode is supported. An
   unknown operation, missing native receiving enrollment, or
   `auth_authority_migration_required` means the installed path needs repair.
   Keep the original request and return that missing capability. Do not import a
   vault adapter, create a temporary live authority, become a policy editor, or
   create your own standing rule to complete a sign-in. A conversational login
   instruction does not waive the configured approval workflow.

The Auth evaluation reached intended provider accounts through private helpers,
but those runs bypassed installed approval and notification. They qualify the
provider steps below, not the integrated service. See the
[acceptance correction](https://github.com/CochranResearchGroup/auth-receipts/issues/12#issuecomment-6379594793),
[installed-path repair](https://github.com/CochranResearchGroup/auth-receipts/issues/81)
and [deterministic MFA work](https://github.com/CochranResearchGroup/auth-receipts/issues/79).

## Request and use the owner-authority path

The source hosted adapter exposes `requestLogin` and `advanceLogin` alongside
`followLogin`. Request a private login with the selected source/record, exact
origin, owner-enrolled receiving `targetId`, purpose, expiry and stable idempotency
key. Keep the returned request ID. If request acknowledgement is lost, retain the
same key and complete arguments, including expiry; never invent a replacement.
These fields select existing owner enrollment, not a new policy or profile route.

`advanceLogin({requestId})` inspects that same request, polls its correlated parent
approval when pending, and invokes private use only when approved with no attempt.
It returns pending/held/denied/unknown states as authoritative outcomes. Calling it
again on an unknown attempt does not repeat private use. Then use `followLogin`
for delayed MFA and independent authentication. No method accepts raw release,
policy changes, caller-supplied grants or a replacement client. All three require
the supported SDK and shared-owner capability before proceeding.

The legacy routine playbook retains its historical contract. Use these owner
methods for an enrolled Auth receiver; do not adapt legacy scope labels into
credential references or assume a profile display name is an enrolled target.
Installed owner/native enrollment and real acceptance remain open.

## Follow an existing private attempt

The source `connectHostedAuthHelper()` adapter exposes `followLogin({requestId})`
for an existing owner-authority private login attempt. It delegates observation,
MFA delivery and original-attempt reconciliation to the supported Auth SDK using
its own installed client. It requires the shared-owner credential contract and
returns `auth_authority_migration_required` if the SDK or authority is incompatible.
Authorization and transport errors retain their original codes; distinguish
those holds from missing interface support. Do not pass it to the legacy login
playbook as a replacement authentication call.

The follow-up never requests or repeats private input. Bounded observations,
optional AbortSignal and public `onUpdate` summaries let a client wait for delayed
MFA without agent-authored messages. Timeout/abort stops following; it does not
cancel the authority's request. Use the original request's cancellation operation
when cancellation is intended. Native receiver enrollment and real integrated
acceptance are still open; the installed helper may lack this source capability.

## Native receiving readiness and unknown outcomes

Require a complete HTTPS address in the owned browser chrome before private use.
Stock Chrome can omit its scheme from the address bar. A scheme-less value stays
unverified; never prepend HTTPS from the requested origin. New profile enrollment
can enable Chromium's `omnibox.prevent_url_elisions` full-URL display preference.
Existing profiles need reviewed enrollment changes; do not edit a running profile
or global browser settings during a sign-in attempt.

The enrolled receiver must perform a guarded non-secret click, observe the same
password field focused, then commit its single private typing action. A combined
click/type/Return batch did not deliver text in actual Chrome testing. Native
`delivery: dispatched` acknowledges dispatch; it does not prove form receipt or
authentication. AT-SPI may expose one account label as a paragraph plus its child
text. Only matching parent/child text with contained bounds is one rendered label;
separate matching labels remain ambiguous.

A broker observation timeout leaves the original use unknown. Keep its request ID
and inspect/follow that request; do not start a replacement login or repeat private
input. Stop the bounded follow-up explicitly if it cannot resolve the outcome.
The synthetic Native/Chrome qualification recovered a late acknowledgement this
way and independently observed one matching HTTPS submission. It proves actual
synthetic receiving, not installed approval, Google/Microsoft MFA or Slack delivery.

## Google: headed Chrome and native control

Use the managed persistent profile with stock headed Chrome for the observed
Google sign-in path. Keep DevTools closed during sign-in; use the owned Native
Remote View keyboard/mouse path through the enrolled receiver. Generic DOM login
examples do not qualify this native path. Preserve the profile for future use.

Select the intended account or enter its enrolled identifier, then let the
installed authority handle private password or approved factor use. A saved vault
URL may be a registration or other non-login page: choose the current canonical
provider login surface through the ordinary workflow, then verify actual origin
and account binding before credential use. Do not overwrite the stored URL or
broaden policy merely because discovery needs another page.

For number matching, wait for the bound challenge to produce its parent-routed
number, direction-specific instructions, account context, expiry and safe crop.
The owner performs device approval. Neither a message acknowledgement nor a
clicked phone approval proves provider advancement. Verify the resulting Google
Account identity independently, then verify the consuming application's identity.
Do not promise cold-start persistence until reopening that same profile has been
separately checked. A retained profile and a currently authenticated session are
different evidence.

## Microsoft: password first, then the chosen factor

For the observed work/school Outlook journey, choose the intended work/school
account and password sign-in first. Explicitly focus the password field before
private input. If a hardware-key modal blocks that selected path, dismiss the
modal through supported native control and return to the same account flow.
Do not enroll/remove factors or change account security settings.

The evaluated password-first journey advanced to Authenticator number matching;
starting with passwordless Authenticator had not delivered the expected prompt.
Treat this as observed workflow guidance, not a claim about every Microsoft tenant.
Honor the owner's factor preference when the enrolled route supports it. For SMS,
require the watch's readiness before triggering delivery; an unavailable watch is
an explicit waiting state, not permission to generate repeated messages, reset a
profile or pair another person's device. Owner device approval remains explicit.

After Authenticator approval, verify the provider advanced and inspect exact
Outlook account identity in the same retained profile. An approved phone prompt,
saved browser profile or Outlook page title alone does not prove that identity.

## Handoff, fences and completion

Use the existing task's opaque handoff URL after Native display readiness. Let
the person control the same browser, then continue with the same task. Re-read
current control ownership and generation before input. Missing authority or a
changed target is an appropriate hold. A valid current handle rejected because
an advertised capture operation cannot carry the required post-handoff epoch is
a provider-contract defect; preserve its evidence and profile instead of resetting
fences or silently manufacturing another route. A visible browser alone does not
distinguish these cases.

MFA capture must exclude credential fields, URL/session tokens and unrelated
windows. Missing safe capture or failed delivery stays held. Do not replace the
service's deterministic processing with a discretionary manual screenshot/send
and call the ticket accepted. Changed numbers supersede the original challenge;
retry its delivery without requesting another MFA prompt or replaying a password.

Before claiming completion, retain separate receipts for owner authority,
requested notification, actual private release, release notification, challenge
text/image delivery where applicable, provider advancement and exact account
identity. Keep unknown outcomes and their original request/attempt IDs. Preserve
profiles and peer work; close only the requested owned task. These instructions
add no runtime capability: native enrollment and installed acceptance for Auth
#81/#79 remain open until their actual evidence exists.
