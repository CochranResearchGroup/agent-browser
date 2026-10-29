# Standalone Auth integration contract readiness

Date: 2026-10-25 America/Chicago
Product lane: PL-AUTH
Disposition: accepted-evidence
Owning plan or work item: Agent Browser #449; existing P279 source is an input
Related lanes: PL-PLATFORM, PL-RECIPES

## Question and finding

Are the developing components far enough along to write the contracts for
replacing Agent Browser's Bots authentication dependency with standalone Auth
Receipts? Scope assumes Agent Browser, Auth Receipts and parent Receipts; Remote
View is assessed only through Auth's documented receiving prerequisites.

**Yes for a bounded core integration contract.** Existing source and public-seam
tests establish ownership, credential request/use, owner review, delivery and
original-attempt recovery. Write against those interfaces now, with explicit
provider prerequisites and acceptance requirements. Do not freeze installation
layout, actual sign-in-control execution or newer authentication methods as
already implemented guarantees. These need targeted provider/consumer agreement.
The assessment does not establish installed end-to-end readiness.

This is an integration migration assessment. No product implementation, new
credential authority, runtime adoption, provider login or live request retry is
part of this note. Source, specification, tests and first-party tracker evidence
were the allowed source classes. Investigation stops at identifying contractable
interfaces, moving dependencies and a bounded next specification scope.

## Source identities and active work

Freshly fetched origin/main identities:

| Component | Published main | Relevant developing source |
| --- | --- | --- |
| Agent Browser | c5063a3f84712b53b00448e182c1ced4b1db32e3 | auth/receipts-journeys at cc981b2beb037bd2cd077c49997b8e9f4aea3f62, with existing uncommitted P279 adapter/guidance |
| Auth Receipts | e000f82563b445062411592947f3607cfe7c9228 | chore/acceptance-reconciliation-20261025 at that base, with uncommitted owner-entry, account-binding and JavaScript receiving changes; separate implement/spec110-auth-methods work |
| Parent Receipts | 22a955efaa0b80301462b2b872df305c7585a4b0 | receipts-issue106 at d6e07227e18b4ad9c866c6fe24e64bdcecdde294 has byte-identical apps/receipts-interoperation content to fetched main |

The primary Receipts checkout is an older unrelated branch at b7210a3. Its roadmap
alone would understate integration readiness. Its newer published interoperation
source contains both the request hub and notification relay. Auth's older retained
issue81 checkout also must not be mistaken for the latest reconciliation source.
No checkout was switched, created, removed or assigned during this research.

Source pins for inspected uncommitted inputs, SHA256:

```text
Agent packages/client/src/hosted-auth-helper.js
25ff2c6585fc1e1693deb8673f941e3b54951ce13920e2e38967e35a150620ad
Agent packages/client/src/login-playbook.js
764f9897c44415dce34c28f276032bae9724022cc48edbf25138bac1be303dcd
Auth reconciliation src/credential-service.mjs
e82593ba069296c404f5b03c818342ad4316e60e7f6f9cff107118950a22490e
Auth reconciliation src/generic-browser-receiver.mjs
62513abe7ebd1e8cc1b6b89e25a0b05f5b8d65052c6970e69ba9a954ea69177d
```

## Contract readiness by boundary

| Boundary | Assessment | Contract implication |
| --- | --- | --- |
| Ownership | Ready | Auth owns credential policy, decisions, requests and private-use attempts. Parent owns channel routing/delivery. Agent Browser owns browser workflow and target evidence. Bots is another client. |
| Standalone SDK/bootstrap | Needs agreement | Auth exports its client, but Agent defaults to the Bots installation and imports that installation's verifier. Auth's client also retains Bots-named discovery defaults/transport. Specify producer-owned verified installation discovery, version/capability negotiation and actionable failure behavior; do not invent a replacement path. |
| Service/account intent and owner review | Ready to specify; provider publication pending | The Auth reconciliation source permits exact service/account discovery and pending owner review without a prior site rule. Preserve original owner choices, explicit deny/revoke and zero protected reads before authorization/delivery. Do not specify published main's earlier denial as desired behavior. |
| Request/use/recovery | Ready | Existing request_credential, inspect_credential_request, receive_credential_approval, use_credential and reconcile_credential_request supply the authority lifecycle. Preserve idempotency and original attempt IDs across interruption. Unknown use is readback-only. |
| Browser workflow composition | Consumer migration required | P279 requestLogin/advanceLogin/followLogin are usable source foundations, but createLoginPlaybook still calls legacy request_login/inspect_request/begin_authentication operations. Define one owner-based composition and explicit retained legacy-checkpoint handling. |
| Private fill and actual submission | Needs evidence contract and implementation | The guarded JavaScript-form repair prevents native GET leakage, but still uses requestSubmit. That does not establish execution of a site's separate Sign in click handler. Bind reviewed controls to exact task/profile/tab/browser generation and distinguish fill, dispatch, acknowledgement and authenticated identity. |
| Parent approval and notifications | Ready | Published application-bound send/receive and relay accept/deliver/inspect interfaces preserve actor/correlation and independent requested/released receipts. Parent delivery is not a credential grant. Capability-gate challenge image/cancellation/reconciliation extensions. |
| TOTP and number matching | Ready to specify with separate acceptance | Auth exposes observe_authentication/continue_authentication for TOTP and refresh/follow for existing number-matching attempts. followCredentialLogin alone does not invoke the TOTP continuation operations. Compose returned next actions explicitly and retain linked factor-request custody. |
| Email/SMS/link/passkey methods | Provisional extension | Spec110 is actively changing source. Keep method negotiation extensible and unsupported methods explicit; do not make these unfinished implementations prerequisites for migrating the core password/TOTP interface. |

Evidence: Agent [adapter](../../../packages/client/src/hosted-auth-helper.js),
[legacy playbook](../../../packages/client/src/login-playbook.js), Auth
[published SDK](https://github.com/CochranResearchGroup/auth-receipts/blob/e000f82563b445062411592947f3607cfe7c9228/src/auth-client.mjs),
[TOTP contract](https://github.com/CochranResearchGroup/auth-receipts/blob/e000f82563b445062411592947f3607cfe7c9228/docs/agents/totp-continuation.md),
parent [request hub contract](https://github.com/CochranResearchGroup/receipts/blob/22a955efaa0b80301462b2b872df305c7585a4b0/apps/receipts-interoperation/README.md)
and [notification contract](https://github.com/CochranResearchGroup/receipts/blob/22a955efaa0b80301462b2b872df305c7585a4b0/apps/receipts-interoperation/notifications.md).

Developing Auth evidence is in the current local reconciliation checkout:
[ADR0011](../../../../auth-receipts-reconcile-20261025/docs/adr/0011-owner-driven-request-entry.md),
[owner-entry tests](../../../../auth-receipts-reconcile-20261025/test/owner-request-entry.test.mjs)
and notes0073–0080. In particular,
note0077 qualifies synthetic JavaScript POST/GET protection; note0078 retains
the [real Stripe private release](../../../../auth-receipts-reconcile-20261025/docs/dev/notes/0078-stripe-owner-grant-private-input.md) as unknown, without verified submission/login;
notes0079/0080 qualify the later standing-account-binding repair as uncommitted
and uninstalled. Their current source hashes above prevent substituting main or
an older checkout for those findings. Native AT-SPI registration remains open;
the explicitly qualified browser-client path does not close that finding.
[Issue449](https://github.com/CochranResearchGroup/agent-browser/issues/449)
records these required browser-side changes and acceptance boundaries.

Auth's separate [spec110 execution ledger](../../../../auth-receipts-spec110/docs/dev/notes/0074-spec110-execution.md)
records locally validated email/SMS/link slices,
developing passkey work and pending enrollment/selection/installed acceptance.
This is uncommitted source evidence, not published interface stability.
[Auth110](https://github.com/CochranResearchGroup/auth-receipts/issues/110)
owns that extension. [Auth81](https://github.com/CochranResearchGroup/auth-receipts/issues/81)
and [Auth91](https://github.com/CochranResearchGroup/auth-receipts/issues/91)
remain open for installed ordinary-path and TOTP acceptance. Parent
[issue20](https://github.com/CochranResearchGroup/receipts/issues/20) remains open;
its publication and consumer acceptance are separate boundaries.

## Fresh validation

Research reran existing synthetic checks, without adding tests or changing code:

| Command / source | Observed result | Limit |
| --- | --- | --- |
| pnpm exec node scripts/test-hosted-credential-follow.js, Agent | Pass | Synthetic SDK boundary; no standalone default-install proof |
| pnpm exec node scripts/test-login-playbook.js and test-login-playbook-hosted.js, Agent | Pass | Existing legacy playbook contracts, not owner-path migration |
| Auth reconciliation test/agent-browser-owner.acceptance.mjs, pinned actual Agent adapter | 1 passed | Real SDK/IPC/authority; synthetic installation verifier and external ports |
| Auth reconciliation test/shared-owner-parent.acceptance.mjs, parent source receipts-issue106 | 1 passed | Actual hub/relay/owner source; synthetic child/browser/source effects |
| Parent apps/receipts-interoperation/test/*.test.mjs | 36 passed, zero failed/skipped | Public local source contracts, not real Slack delivery |

Auth acceptance commands used Node's existing isolated-time import. The Agent
composition set AUTH_RECEIPTS_BROWSER_HELPER_MODULE to the inspected adapter;
the parent composition set AUTH_RECEIPTS_HUB_SOURCE to receipts-issue106.
Their source hash assertions passed unchanged. No full suite, real Chrome fixture,
installed preflight, private state inspection or provider test was rerun. Historical
native and Stripe findings above are attributed source reports, not fresh runtime
readbacks. Graphiti discovery supplied advisory historical leads; current source
and named notes determined the assessment.

## Recommended specification scope

Write the core consumer/provider contract under #449 now. Cover verified
standalone bootstrap, service/account discovery, owner review, exact receiving
binding, private input plus actual sign-in action, typed evidence, linked MFA and
original-attempt recovery. Declare minimum capability requirements and explicit
unsupported states. Retain compatibility storage/transport names until a deliberate
migration; removing the Bots dependency does not require moving private state.

Resolve two concrete contract details with the owning source before freezing the
implementation plan: Auth's supported verified SDK installation locator/version
surface, and browser-action evidence for the exact Sign in control. Reconcile
Auth81's uncommitted owner-entry/account-binding changes before dependent adoption.
Keep spec110 methods as separately negotiated extensions. Contract writing can
proceed while those implementations develop; installed acceptance waits for their
required published/adopted checkpoints and separately bounded workflow evidence.
