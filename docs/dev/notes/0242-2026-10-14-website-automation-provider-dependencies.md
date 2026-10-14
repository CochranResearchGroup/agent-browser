# Website automation provider qualification dependencies

Product lane: PL-RECIPES
Disposition: active-input
Owning plan or work item: P242, spec271
Related lanes: PL-AUTH, PL-PLATFORM

P242 adopts this source-backed dependency record. It preserves the distinction
between qualified owned synthetic workflows and future provider/real-account
acceptance. It authorizes no foreign repository edit, issue publication, private
data access, production deployment or new viewer/authority ceremony.

## Remote View private-operation integration

Read-only source inspection at remote-view c67d1f60ae33d8aebf8fe0948ddf777d5e17e121
found no supported desktop-wide private-operation/capture hold. The advertised
input_hold operation is held keys/buttons with a short TTL. capture,
capture_region, OCR and desktop watchers operate independently. Viewing route
revocation removes sessions and does not gate capture.

The provider capability proposal is a generation-bound private-operation hold
that drains and suppresses relevant machine observers, retains exact task/desktop
custody through interruption, and releases only after a sanitized safe-capture
readback. Integrate Agent Browser's machine reads with that same boundary.
Preserve normal authenticated native human assistance, including MFA, through
the existing Remote View handoff; do not introduce another viewer/access layer.
Provider-wide observer/stream qualification is required before claiming general
private-input readiness. P242's constrained fixture has no independent viewers,
recorders or watches and wires every participating client read to authPrivacy.

Source anchors: remote-view src/control_runtime_agent.rs capture/input catalog
and capture implementation; src/control_runtime.rs viewer schemas; src/viewing.rs
revoke_route. These are locators in the provider repository, not copied code.

## Shared auth-helper capabilities

Initial grok-bots draft source qualification used
b1fa8e8ef2f9f0f4f65b0373a2c0fdb6227091b0 and
/tmp/p242-helper-source-identity.json. The active foreign lane advanced during
final checks to 23877a5c0c200b11517a0369cf35ced65e31ca99. Final qualification
uses a read-only four-module source snapshot at /tmp/p242-auth-helper-final,
with exact module hashes in /tmp/p242-helper-source-final-identity.json. No
foreign source edit or installation is performed.
Source request_login, inspect_request and authenticate are qualified through
an owned dummy receiver and actual private IPC. Production installation and
real LastPass/extension/passkey/MFA acceptance remain distinct.

The producer intentionally collapses locked, absent and failed password sources
into an unavailable outcome. Exact presence/locked-state disclosure requires a
sanitized producer capability. Extension use, existing passkey use and passkey
provisioning must remain separate; unsupported method is not an entry lookup.
Automated MFA requires current challenge/account correlation and exact authorized
second-source/provider binding; a credential-source grant does not imply access
to arbitrary email/SMS. The supported source injection can await human MFA.

The current IPC timeout is 30 seconds and an apply operation may continue after
disconnection. Inspect the original request; never resubmit an uncertain effect.
The final pinned producer publishes a running boolean and start/input
timestamps. P242 qualifies inspection and receiving reconciliation through its
dummy lane; structured challenge projection and general private-operation
recovery remain producer qualification dependencies. The current origin assertion is singular; real redirected login
and receiving origins require explicit adapter/provider acceptance, not inferred
credential authority for every SiteGuide origin.

Native task attribution supports live opener chains and preserves peer claims.
An extension target with no attributable opener needs a trusted producer target
binding; its URL is not ownership proof. Source anchors are grok-bots
codex-bots/src/auth-service.mjs, auth-broker.mjs, auth-browser.mjs and
auth-privacy.mjs plus issue23/draft PR34. Preserve that foreign lane's custody.

## Acceptance boundary

The stronger P242 fixture gives the synthetic website an independent opaque
credential oracle. Wrong private input does not complete login. Correct input,
human confirmation, actual helper receiving readback and the Agent Browser
account/company assertion must agree. An injected lost acknowledgement resumes
the same request/action without another credential retrieval or submission.
Those receipts prove the declared fixture deployment, not real providers or a
general privacy barrier. The next provider tickets can use this record without
reopening implementation of the lower desktop layer in Agent Browser.
