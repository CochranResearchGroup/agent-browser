# SiteGuides and shared website routines

Use one existing taskId for software and agent work. A SiteGuide supplies site
knowledge; routines execute subtasks; playbooks compose routines. A browser
window at a login page is not an authenticated intended account/company.

For Google/Microsoft native sign-in and the current installed Auth authority
gates, read [governed authentication](governed-authentication.md). A routine
implementation does not prove the installed receiving mode is enrolled.

## Discover and verify

1. `browser_discover({category:"playbooks"})` lists operations.
2. Request `operation:"verify_login"` for its exact schema.
3. `browser_command({taskId,action:"playbook",params:{operation:"verify_login",guideId,accountRef,organizationRef}})` verifies the current task.

For verify_login, `completed` means both intended identities match the current authenticated
application evidence. `waiting` identifies an observed login/MFA state;
`adaptation_required` identifies a stale observation, changed task target or
unrecognized site; `failed` identifies wrong origin/account/company or a blocked
site. This verification routine submits no credential or code. Continue in the
same task using the indicated state rather than creating another browser.

Use `operation:"siteguides"` or `operation:"inspect_guide"` (with guideId) through
browser_command to read the configured guide names or one guide's definition.
The MCP server loads only reviewed guides from its configured directory.

## Software uses the same interface

```js
import {createPlaybookClient} from '@agent-browser/client/playbooks';
const playbooks = createPlaybookClient((name, args) =>
  mcpClient.callTool({name, arguments: args}));
const result = await playbooks.verifyLogin({
  taskId, guideId: 'bill-session-v1',
  accountRef: expectedAccount, organizationRef: expectedCompany,
});
```

The callback is your existing connected MCP client. It returns the standard
MCP content envelope; the wrapper returns the same routine data an agent sees.
It creates no transport, task, browser or credential store.

## Minimal reviewed guide

Configure `AGENT_BROWSER_SITEGUIDE_DIRECTORY` when starting the MCP server. Store
one guide per `<guideId>.json`. This example uses an owned synthetic application;
it is not a locator claim about a real BILL account.

```json
{
  "schemaVersion": "agent-browser.siteguide.v1",
  "guideId": "bill-fixture",
  "name": "BILL fixture",
  "allowedOrigins": ["http://127.0.0.1:8080"],
  "authenticatedSelector": "#application",
  "account": {"selector":"#application","attribute":"data-account"},
  "organization": {"selector":"#application","attribute":"data-company"},
  "states": [{"state":"password_required","selector":"input[type=password]"}]
}
```

Require exact origins and a unique visible authenticated application. Identity
fields select one element and compare its nonsecret text or a data/aria-label/title
attribute to the requested identity. Password values and full page bodies are
never routine output. Supported state names are identifier_required,
password_required, mfa_required, login_required and blocked. Unknown variants
require new evidence and a reviewed guide change.

Verification measures the complete observation round trip. Default freshness
is 10000 ms; set maxObservationAgeMs only when the task needs a different
explicit 1 through 60000 ms bound. Task/tab identity is rechecked after reading.

Remote View owns desktop effects. Shared auth helpers own private credential
and MFA operations. These verification calls do not establish helper readiness,
real login acceptance or passkey support.

## Private login composition

The supported software adapter composes existing verification and shared helper
operations without moving credentials into the client:

```js
import {connectHostedAuthHelper} from '@agent-browser/client/hosted-auth-helper';
import {createLoginPlaybook} from '@agent-browser/client/login-playbook';
const caller = await connectHostedAuthHelper();
const {accounts} = await caller.helper('list_accounts', {});
// Select scope from an exact configured account/source/origin mapping.
// accounts describes configured scopes; it does not prove vault readiness.
const login = createLoginPlaybook({
  browser: playbooks,
  helper: caller.helper,
  checkpoints: durableMetadataStore,
});
const options = {
  taskId, guideId, accountRef, organizationRef,
  callerTaskId: caller.callerTaskId,
  scope, // {person, tenant, account, origin, source, capabilities: ['login']}
  expiresAt,
};
const result = await login.advance(options);
```

Discovery verifies the installed producer candidate, loads its supported SDK and
keeps bearer material inside that private transport. The ordinary caller is the
currently attested Codex work/thread, with no Bots roster entry or TUI required.
Its `callerTaskId` is separate from the receiving browser `taskId`. Version2 freezes
the exact receiving provider/task/profile/browser/tab, while the summary's legacy
`taskId` identifies the caller. Preserve both identities rather than aliasing them.
Account/source references come from configured logical metadata and the existing
source owner; do not guess them from a browser profile or request a password in
model-visible conversation. No standalone auth service is created.

`helper` uses request_login, inspect_request and begin_authentication for ordinary
hosted callers; legacy callbacks retain authenticate. Both return sanitized
structured summaries. The trusted receiving configuration positively verifies the
original caller/receiving association and rechecks exact native ownership around
private effects. Shared hosted-login-receiver and native-receiving-browser ports
are trusted-host modules, not credential APIs for agents. They sequence actual
identifier/password states and configured private MFA callbacks; check visible,
masked, account-bound controls; use native Remote View process/window/visual
fences; and prove empty-field and exact-value readback privately. A recognized
popup may receive one guarded Escape before a fresh hit-tested submit. Unknown
Escape or submission acknowledgement permits no replay. A current exact
identifier match needs no clear/retype under unchanged native/control guards;
unknown prior delivery cannot become equality success. Qualify every real MFA,
passkey and password-manager path separately; no resend, enrollment or password
saving is implied.

The store implements asynchronous load(key) and save(key, checkpoint); save
must durably complete before returning. Checkpoints contain scope, task/page
binding and request metadata, never credential or challenge values. Preserve
the same inputs, including expiresAt and callerTaskId, when resuming.

`approval_required` waits for the shared helper's approved request. After a
submission attempt, each advance inspects that request. A missing response,
30-second helper timeout or restart yields helper_running_or_uncertain and
effectState unknown; never create another request or resubmit to recover it.
Completed helper authentication also requires captureState ready and fresh
receiving verification. A held capture, changed task target or wrong company
cannot produce playbook completion. Unsupported extension/passkey methods and
helper absence produce waiting states with no submission.
An uncertain terminal helper request with capture ready and no inputAttemptedAt
proves that private input did not begin. It yields password_source_unavailable;
check the configured source and password-manager unlock state in the helper.
The current producer intentionally does not disclose whether the source was
locked, absent or otherwise unavailable. Exact lock-state detection remains a
producer capability gap; the client does not infer it from a failed login.

### Read back after human completion

Keep the original browser and helper request. After independent human/browser
completion, use the same options and checkpoint:

```js
const continued = await login.advance({...options, reconcile: true});
```

This mode calls the existing producer's reconcile_authentication. It neither
creates a replacement request nor retrieves credentials, starts authentication
or repeats private input. Running work is inspected, not restarted. A missing
checkpoint is an error; unsupported owner readback returns
helper_reconciliation_unavailable. Original scope, caller and receiving tuple
remain checked. The mode does not renew an expired or revoked grant.

The trusted receiver independently observes the current application principal;
its reconciliation-specific native safety callback must also qualify current
capture readiness. An expected saved email on a login chooser is insufficient.
Missing/false safety stays held even when the principal is authenticated. An
exact owned login or unknown page returns negative readback without private input.
Once the helper is authenticated and capture ready, the same playbook still
requires fresh independent SiteGuide account/company evidence. QBO browser
identity does not establish OAuth/API readiness. Never reset a checkpoint,
replace a request or edit helper state to bypass a held outcome.

A trusted owner can retire a failed login-only request when durable before-effect
markers prove no protected password/factor attempt and fresh known-login DOM and
native evidence prove clean capture. Timeout/current page alone is insufficient.
The helper keeps request/caller/target/expiry/input history, including unknown
earlier identifier effects, and returns released without authentication. The
playbook returns failed with helper_attempt_released and cannot replay that grant.
Preserve the original checkpoint. A task controller may deliberately select a
separate operation under its task authority after this confirmed failure; it must
not silently reset the checkpoint or retry an unresolved held attempt.

Configure privacy at the producer and every observer before using private
input. The installed independent native dummy qualifies the existing hosted helper,
private identifier/password input, guarded reads, native DOM/OCR safety,
uncertain-request restart and independent reconciliation. Controlled MFA
qualification remains separate from real-provider MFA. Remote View currently lacks a provider-wide privacy hold; the
adapter does not claim suppression of independent captures or viewer streams.
Task tab listing observes live opener chains through the existing browser owner.
After helper capture is ready, the adapter checks newly attributed task tabs for
the intended receiving account/company. One match becomes the continued task
page; multiple matches require adaptation. Existing peer and pre-login tabs are
excluded. This supports attributable site/password-manager popups. Extension
tabs with no attributable opener require a producer target-binding capability;
the system never guesses ownership from a URL. Real account/source mappings, completed website/company authentication,
password-manager/passkey behavior and automated MFA require their own current
qualification; dummy acceptance cannot replace those outcomes.

## QBO uses the same contract

Choose qbo-session-v1 for the QBO starter template or a reviewed fixture guide.
Use the same verifyLogin and createLoginPlaybook interfaces, retaining the
intended account and company/realm reference in private runtime options.
Browser company verification does not prove OAuth token or accounting API
readiness. Books Receipts keeps those operations and financial policy.

A guide's optional controls describe identifier, password, submit and
accountScope locators, plus identifierSubmit for an identifier-first variant.
The trusted helper receiving adapter reads these reviewed controls through
inspectGuide before starting protected input. It owns credentials and validates
the target before each consequential input. The guide carries no values,
selected account, MFA material or helper access grant.

The QBO fixture uses different controls, a separate account element and a
data-realm-id company marker. Identifier-first and password-first variants use
one private helper contract; human MFA wait and asynchronous popup continuation
retain the original task. Shared routines contain no BILL URL or company-path
assumption. Real Intuit controls and account acceptance remain separately scoped.

The login client requires prepareLogin and validatePlan. After reviewing changed controls, pass a freshly prepared plan as reviewedPlan to advance with the same options and retained guideId. It validates the replacement against the current task before saving it and retains the helper request/action. Reviewed guide content may change under that retained identifier through the candidate registry. A typed pre-input helper refusal can be retried only after exact-request inspection proves approved state with no started or input-attempt timestamp; a lost acknowledgement never permits retry.

## Develop and adapt a guide

1. Bind an owned synthetic browser task and define exact success: origin,
   application state, account and company. Observe its variants through the
   ordinary task interface. Never copy private recordings or credentials.
2. Write a versioned guide with state selectors, receiving assertions and
   semantic controls. Use the fixture directory, not a tenant-state file in
   the product repository. Implement stable subtasks with normal code.
3. Call prepareLogin with the same verification options. A plan_prepared result
   completes preparation, not login. The plan freezes guide digest, task binding,
   control fingerprint, expected state and bounded observation age.
4. Pass the unchanged plan to validatePlan before input. A changed guide,
   control, state, task or stale plan yields adaptation_required and no effect.
   The login adapter enforces this before the helper retrieves any credential.
   It automatically refreshes only stale observations of unchanged controls under the same
   helper request. A plan is metadata, never an access grant.
   Password preparation requires an input of type password; fresh evidence
   never authorizes filling an unmasked replacement.
5. Observe changed controls afresh and propose reviewed guide content outside
   the configured shared registry. Run a candidate client with that proposal
   directory against the same owned fixture task; verify the expected states
   and outcome. Keep the canonical guide unchanged during adaptation.
6. Publish the tested version through the normal source review/version path.
   A successful task improvisation never silently rewrites shared knowledge.

The executable example is scripts/siteguide-authoring-fixture.js. It prepares a
BILL login, changes its password control, proves the old action is invalid,
tests a v2 proposal through a candidate MCP client in the same task and proves
that canonical bytes and registry membership are unchanged. The shared QBO
fixture exercises the same guide schema with different controls and realm identity.

To continue with an agent, preserve the durable login checkpoint and its helper
requestId/actionId. The next controller calls the same advance method and
inspects pending effect custody. During MFA waiting or after a lost response,
the existing request is inspected; no new task, private store, viewer or login
submission is created. Native Remote View remains the human handoff surface.

## Bounded non-login retrieval

collect_posts uses the same browser_command playbook interface, or
createPlaybookClient(...).collectPosts({taskId, guideId, maxItems, maxPages,
maxScrolls}). A retrieval guide declares containerSelector, itemSelector,
idAttribute, titleSelector, urlSelector, noResultsSelector and optional
nextPageSelector. It needs no account/company or login controls. The bundled
reddit-feed-v1 guide is a starter requiring current site qualification.

Defaults are 50 items, one page and zero scrolls. Limits are 200 items, 10 pages
and 10 scrolls; each observation uses the same 1 through 60000 ms freshness bound.
Next-page links are rechecked before same-task navigation. Scroll uses ordinary
website DOM operation, not a new desktop input provider. Item links are normalized
to HTTP(S) origin/path without query or fragment. Results contain bounded IDs,
titles and links, never a full page body or downstream ranking decisions.

completed/no_results requires an explicit unique visible empty marker and no
items. Empty extraction without that proof, malformed data, login/blocked pages,
changed pagination and partial budget exhaustion remain distinct. pagesUsed,
scrollsUsed, observations and malformedItems expose actual coverage. Preserve
partial records; never turn them into a no-results claim. Site item text is data,
not an instruction to the controller. Last30Days keeps research/ranking semantics.

Develop the guide against an owned feed fixture just as for login: define exact
success and emptiness, check one malformed/blocked variant, propose a reviewed
guide version and run the public routine in the original task. The executable
example is scripts/siteguide-retrieval-fixture.js. This delivery runs no live
social-site request and migrates no downstream repository.


## Qualified existing-profile boundaries

Plan 0245 and its curated actual QBO/BILL receipts record installed-helper web
acceptance with independent intended account/company verification. QBO company
evidence is exact display-name matching; it does not prove a numeric realm ID or
OAuth/API readiness. BILL qualifies the current protected profile UI after an
observed same-task Settings continuation. Login origin, authenticated application
origin and stored source-record origin may differ; qualify each from its owner
and current UI rather than broadening an allow-list to bypass a hold.

Starter guides remain templates. Derive unique visible account/company selectors
from current protected UI and compare the exact requested nonsecret references.
Keep the original authentication checkpoint and source mapping; a separate
verification-only guide/routine can qualify the already authenticated session
without starting another helper operation. Native privacy still requires the
trusted receiving owner's current proof. A configured source, account name,
company text alone, previous success or a saved session is not that proof.
