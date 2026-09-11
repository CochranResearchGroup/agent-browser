# Incident: release skipped physical cleanup; follow-up closed a different tab

Date: 2026-09-06
Status: Reported; root cause and fix unverified
Reporter: SoyLei Website Plan0137 agent
Scope: Investigation handoff only. No browser actions or product changes performed while writing this report.

## Problem and impact

An owned tab release returned success and a closed lifecycle while explicitly
reporting that physical closure was skipped because there was `no_live_browser`.
Direct CDP target readback still showed the owned tab. A follow-up `tab_close`
request, intended to target that same tab, returned success with
`{"activeIndex":0,"closed":0}`. Readback showed the owned tab remained and a
different existing page had disappeared.

The other page was a Google Rich Results Test for the public SoyLei
`/ca/apex-1132/` route. No website content mutation or payment submission resulted
from these browser cleanup actions. The agent stopped all browser mutations;
neither the remaining owned tab nor the closed other page was subsequently repaired
by that agent. This also stopped the surrounding website test campaign under its
unexpected-effects rule. This is an observation at incident time, not a claim
about the browser's current state.

## Incident identities

These are correlation identifiers for retained private receipts, not reusable
authority to operate a present-day browser.

- Profile: `Default`.
- Browser: `session:shared-profile-96c7a9506522e4d194865554`.
- Session: `shared-profile-96c7a9506522e4d194865554`.
- Intended target: `E5266CD1BE9129AC2F7AC83B34205E9F`.
- Intended tab: `target:E5266CD1BE9129AC2F7AC83B34205E9F`.
- Subject: `service:SoyLeiWebsite/agent:codex-p0137/task:hosted-invoice-qualification`.
- Different target later identified as closed: `81553D9C9FDE2646FB9136DD3D619EF4`.
- Other surviving target: `D2A82EB2F2FBBD549892B78C912BFC27`.
- Terminal provenance classified the agent-browser runtime as `production`;
  the website being tested was a development surface. These are distinct scopes.

## Observed sequence

1. The caller used a service-approved CDP attachment for the exact owned handle.
   Before cleanup it removed its fixture authentication cookie, disconnected its
   Playwright client and called `requestServiceCdpDetach`. The detach succeeded.
   It did not call Playwright `browser.close()` or kill Chrome.
2. `requestServiceTabHandleRelease` completed at
   `2026-09-06T20:11:07.205415628Z`, request
   `http-service-request-tab_handle_release-5926a816-1ada-4f1f-b7ac-267e702d3fa7`.
   Its response reported `success:true`, `released:true`,
   `beforeLifecycle:ready`, `afterLifecycle:closed`, `cleanupPolicy:close_tabs`,
   `closeBrowserOnRelease:false`, and `browserProcessPreserved:true`.
   But `physicalTabCloseAttempted:false`, `physicalTabClosed:false`, and
   `physicalTabCloseSkippedReason:no_live_browser` were also present.
   Terminal outcome was `succeeded`, `effectState:verified_effect`, and
   `retryDisposition:do_not_retry`.
3. The caller recorded direct CDP readback: three pages remained, including the
   intended target. The incident summary retains the count and intended-target
   presence, not a complete pre-close target inventory.
4. The caller then used `postServiceRequest` with action `tab_close`. The caller's
   execution account says the intended target was supplied as top-level
   `targetId`, together with the same browser/session/subject. The exact outbound
   request body is not present among the dedicated incident receipt files.
   Confirm its serialized shape from request logs or the caller transcript before
   concluding that the server ignored a supported selector.
5. The follow-up completed at `2026-09-06T20:14:29.672360904Z`, request
   `http-service-request-tab_close-84bd73c4-42cb-439b-8ad3-0f806fcdbe1c`.
   Response: `success:true`, data `{"activeIndex":0,"closed":0}`, terminal
   `succeeded`, `verified_effect`, `do_not_retry`. Its provenance had `tabId:null`.
   The meaning of `closed:0` needs confirmation; it is not proof of the intended
   target being closed.
6. Fresh direct CDP readback showed two pages: the intended target and the other
   surviving target above. The intended target had not closed.
7. Read-only service status identified the different target above as closed,
   titled `Rich Results Test - Google Search Console`. The caller recorded that
   identity separately. This identification came after the action, rather than
   from a complete before/after direct-CDP inventory.
8. The caller stopped browser mutations and preserved the receipts.

The release's embedded `serviceTabHandle.cleanupPolicy` was `close_browser`,
while its outer effective `cleanupPolicy` was `close_tabs`. Its embedded handle
became invalid with `staleReason:tab_closed` despite the physical skip. These
discrepancies are additional investigation leads, not diagnosed causes.

## Expected behavior and investigation targets

- Distinguish logical handle release from verified physical tab closure. A
  consumer must be able to determine that requested physical cleanup is pending.
- Trace why detach/release found `no_live_browser` while direct CDP could still
  enumerate the process's targets. Earlier backend/endpoint changes were reported
  during this session; verify their relevance from service records.
- Establish the actual `tab_close` request schema and serialized request. Check
  whether a top-level `targetId` is supported, dropped by the client, ignored by
  dispatch, or replaced by active/index selection. Caller misuse remains possible.
- If an explicit selector is unsupported, reject it without closing another tab.
  An exact-target cleanup must never silently fall back to active tab or index0.
- Verify ownership enforcement for close operations after logical handle release,
  including why the terminal provenance has `tabId:null`.
- Check whether `verified_effect` proves only some close occurred or verifies
  the requested target's removal. Preserve unrelated targets in either case.
- Reproduce in an isolated disposable browser with at least three targets and
  the intended target away from index0. Cover attach, detach, release, skipped
  physical close, unsupported selectors, stale handles and exact-target close.
  Check target sets before and after, plus process preservation.

Do not replay these identifiers or close tabs in the shared profile to investigate.
This report requests diagnosis by the repository's agent, not a live cleanup retry.
No installed binary version or binary hash was captured in the dedicated incident
receipts; current workspace HEAD or current installed version cannot prove which
build handled these historical requests. Recover provenance from retained logs.

## Evidence location

Original artifacts remain outside the product repo under the reporter's
user-scoped runtime directory:

`~/.soylei-website/operation-receipts/p0137-browser-readiness-r1/`

- `cdp-attach.json`, `cdp-detach.json`: service attachment lifecycle.
- `fixture-cookie-cleanup.json`: fixture-cookie cleanup receipt.
- `tab-release.json`: exact release response and terminal outcome.
- `physical-tab-close.json`: exact follow-up close response and terminal outcome.
- `physical-close-readback.json`: surviving direct-CDP target identities.
- `cleanup-incident.json`: initial caller summary and limitations.
- `cleanup-incident-identity.json`: subsequent different-page identification.
- `status-after-incident.json`: retained service status supporting identification.

Read those artifacts locally as needed. They can include private page URLs,
profile/access metadata and unrelated tabs; do not copy raw bodies, cookies,
authentication material or unrelated browser state into this repository.

Website-side checkpoint: Plan0137 C74, commit `18ba709b` in
`CochranResearchGroup/soylei-website`; later C75 records a separate payment-harness
correction and does not resolve this browser incident.

Report validation: directly reread the release, close, readback and incident
identity receipts. Graphiti discovery was healthy but returned no specific
matching incident; no historical memory claim is used as incident evidence.

## Repository investigation: isolated reproduction, 2026-09-06

Both failure patterns reproduce with the installed candidate binary, SHA256
`d870205e3bed66e882a40b4d17a54d153e183eb8fcb2e2dd1f45c90f87e597e3`,
source `f2786e1a`, in a disposable development HOME and runtime host. This proves
current candidate defects; it does not establish the historical incident build
or recover its missing outbound request.

The fixture created three requested blank tabs plus the browser's initial page,
selected index zero, then sent an explicit top-level `targetId` for another
fixture tab through `postServiceRequest`. The exact request was retained before
dispatch. CDP target inventories showed four pages before and three after: the
specified target survived and a different target disappeared. The response
reported success. All targets belonged to the disposable fixture.

Next the fixture attached and detached the surviving intended handle, interrupted
only its own host, restarted that host and requested handle release. Release
returned success, `verified_effect` and `no_live_browser`. Independent CDP
readback showed that the intended target still existed. The controlled host
interruption is a proven reproducer of the missing manager condition, not a
claim that the historical reporter caused such an interruption.

Source findings:

- `service-request.js::createServiceRequest` preserves the input fields.
  `service_request.rs::normalize_service_request` projects the schema-recognized
  top-level `targetId` into the command. The selector reaches execution.
- `browser_lifecycle.rs::handle_tab_close` reads only `index`; an absent index
  becomes the manager's active index. It ignores the explicit target selector.
- `browser.rs::tab_close` removes the local page entry before issuing
  `Target.closeTarget`, discards that command's error and returns the selected
  index as `closed`. Therefore `closed:0` means index zero, not zero effects.
- Release is permitted without a daemon-local browser manager. Its physical
  helper returns `no_live_browser`; unlike service CDP attach, it does not call
  the existing identity-fenced retained-target recovery helper.
- `release_service_tab_handle_record` unconditionally marks the tab closed.
  The existing unit test explicitly expects closed lifecycle after a skipped
  physical close. That expectation needs correction, not preservation as a
  compatibility requirement.
- The release-specific CDP close also removes its local entry before command
  acknowledgement and does not verify target disappearance. Recovery attaches
  only one authorized target, so the attached-page count cannot establish that
  it is the browser's last physical page.

Private evidence is under campaigns/p160/tab-cleanup-red-eTgjm8 in the existing
user-scoped campaign root. `ledger.jsonl` SHA256 is
`c89fd9e352728324837fbf82aa9a25f5b17448f281a16e228bada46951f53aa3`;
`probe-source.mjs` SHA256 is
`53c08cda6a0432a8e7050d04db00c34e3c95979ad1011c1f4bc6d8341686006a`.
An evidence manifest binds the saved state as well. Two preceding fixture setup
attempts stopped before tab close: an unattributed CLI switch hit
`existing_session_profile_identity_unproven`, and a service switch without the
named profile hit `explicit_profile_conflicts_with_current_owner`. The third
attempt supplied the named profile and completed both cases. These setup
failures remain retained; the profile inference and weak error classification
are additional A1/AX cases, not successful control evidence.

After the interrupted-host fixture, ten positively identified disposable
processes were terminated through PID descriptors with start/executable checks.
No matching fixture HOME processes remained across all three attempts. The
first two normal host shutdowns had left no owned residue. No production browser
or incident tab was touched. Product repair and green verification remain open.


## SoyLei r2 consumer gate, 2026-09-07

A fresh Plan0137 r2 readiness check is blocked at service ownership evidence.
This is a separate observation from the historical wrong-tab incident above;
it neither diagnoses that incident nor establishes that its repair is installed.

The service successfully opened the consumer's own target
`296BABDFC11C3C9F147FAD7A1124BD55`. Diagnostics identify owner generation24 and
browser PID66046, but control-plane attestation lists missing proofs
`profile_lease` and `handoff_receipt`. Owner readiness alone therefore does not
establish permission for the consumer's browser work.

The consumer registered its own capability successfully, but registration returned
`boundToCurrentOwner:false`. The terminal-replacement recovery-plan response remained
`blocked`, with dominant blocker `terminal_owner_evidence_incomplete`,
`recoverable:false`, and next action `inspect_lifecycle_owner`. Its stated reason
is missing exact terminal cleanup and process-absence proof for the retained
lifecycle owner. Registration is not evidence that owner binding was repaired.

Repository source review clarified that this planner requires a terminal owner
and process-absence evidence. The observed owner is live and Ready at generation24
with PID66046. Its refusal is therefore an expected unsatisfied terminal-replacement
contract, not contradictory health or a diagnosed service defect. The ordinary
live-lease reconciliation path was evaluated separately, as recorded below;
terminal-replacement refusal alone does not establish that no safe consumer
recovery path exists.

No CDP attachment, fixture authentication, payment or page mutation was performed
in this fresh resumed check. Browser effects were limited to opening the owned
tab and registering the consumer capability. No cleanup retry or foreign-tab
repair is claimed. The consumer has not proceeded with browser work or applied a reconciliation
transition; these correlation identifiers are not reusable authority.

Private evidence remains in
`~/.soylei-website/operation-receipts/p0137-browser-readiness-r2/`:
`access-plan.json`, `tab-response.json`, `diagnostics.json`,
`capability-register.json`, and `recovery-plan.json`. The writer directly read
the tab, diagnostic, registration and recovery outcome fields. Capability file
contents, access credentials and unrelated page URLs are intentionally absent
from this note. No browser actions occurred while appending this section.


The subsequent own-lease plan, retained as `own-lease-reconcile-plan.json`,
reports `no_safe_reconciliation_transition`, `effectCapable:false`, and
`proposedTransitions:[]`. The separate legacy live-lease plan in
`live-lease-reconcile-plan.json` rejects `profile_lease_authority_mismatch`;
that denial is expected for its null principal and is not permission to adopt it.
These exact evaluated plans admit no consumer transition. No apply or payment
followed them.

A repository-agent source review identifies a prospective identity-resolution
lead: the newly registered own lease has profile digest prefix `ac1e2fb`, while
the actual Ready owner's profile digest prefix is `71def0b`. Registration appears
to canonicalize raw `userDataDir: Default` relative to the caller's working
directory, whereas terminal recovery resolves the named profile. This is a
reported investigation lead, not a confirmed cause or repair. The agent-browser
repository agent should check that registration and live-lease reconciliation
resolve the named profile to the same physical identity as the current owner,
and reproduce the mismatch before changing behavior. Do not work around the
mismatch by applying a foreign lease or bypassing owner proof.

## Consumer source-repair coordination, 2026-09-07

The SoyLei operator asked to force progress after the blocked consumer check.
The consumer is preparing a narrow named-profile identity correction on branch
`fix/p0137-named-profile-identity-20260906`, isolated from the ongoing Plan0160
attestation and tab-cleanup work. Scope: consistent named-profile resolution for
capability registration, rotation and lease projection, with regression tests.
This is a source contribution for the Plan0160 owner to reconcile; it does not
authorize bypassing ownership checks or publish a competing installed runtime.
The consumer will record the frozen commit and validation here when available.
No browser mutation or payment resulted from this coordination note.

Coordination update: the consumer observed Plan0160's new
`named_profile_registration_and_lease_projection_use_launch_identity` regression
in the primary worktree and stopped its overlapping implementation lane. Its
isolated branch already carries a candidate `resolved_profile_identity_digest`
helper shared by registration, rotation, unbound lease projection and recovery,
plus a test that also rejects a foreign principal binding. The consumer will
preserve that patch for review rather than merging over the primary owner's
active repair. The primary Plan0160 owner retains integration and publication.

Consumer contribution is now frozen at local commit
`3cb446bd2f3906c7cf58bcbec15caf23d3bd3813` on the isolated branch above, based on
`406f47ba`. The focused frozen test passed (one test, exit0), including reuse of
an already-issued capability through guarded rejoin and rejection of a conflicting
foreign principal. No red-before-fix claim is made. Root reviewed the three-file
diff and the retained test log. Formatting, Clippy and broader lease coverage
remain for the integrating owner; the contribution is not installed or published.
No builds remain. Prefer one reconciled Plan0160 candidate over duplicate fixes.
For the existing consumer capability, the prospective post-fix route is a fresh
lease projection followed by its permitted rejoin using the current lease ID and
revision; no capability rotation, new principal or synthetic owner state is needed.
Actual runtime readback remains required and could still reveal a separate gate.

## SoyLei original-client recovery retry, 2026-09-08

The original Plan0137 client retried diagnostics against its retained target
`296BABDFC11C3C9F147FAD7A1124BD55` using the existing `codex-p0137`
registered capability and the original saved `serviceTabHandle`. The installed
production MCP route accepted and correlated the request, then refused it at
child admission before browser effects.

Exact response classification:

```json
{
  "success": false,
  "id": "mcp-service-request-diagnostics-064ce074-f55d-4182-9a49-60161fb5db47",
  "failure": {
    "axis": "profile_access",
    "code": "profile_child_subject_mismatch",
    "phase": "child_admission",
    "effectState": "no_effect",
    "retryDisposition": "do_not_retry",
    "recommendedAction": "use_own_service_tab_handle",
    "reuseAllowed": false,
    "hardStops": [
      "blind_retry",
      "impersonate_child_owner"
    ]
  },
  "terminalOutcome": {
    "state": "failed",
    "phase": "execution",
    "effectState": "no_effect",
    "retryDisposition": "do_not_retry"
  }
}
```

The response and retained job use the same request and job ID shown above.
The denial evidence reports `ownerAssurance:self-declared`,
`callerAssurance:registered-capability`, `connectionState:disconnected`,
`permission:tab_observe`, and `reconnectRequested:true`. Both the child and
current policy contain the required permission at revision1, so this is an
identity continuity refusal rather than a missing read permission.

The client obeyed the typed hard stop. It did not retry, change attribution,
refresh or replace the handle, register or rotate a capability, adopt a lease,
create a profile or tab, close a tab, or perform a payment or other page
action. Consequently no harmless page read was attempted. Read-only status
after refusal still showed the exact target `ready`, its retained handle
`valid:true`, and its child connection `disconnected`; the refusal reports
`effectState:no_effect`.

Private evidence is retained outside the product repo under
`~/.soylei-website/operation-receipts/p0137-browser-readiness-r3/`, including
the exact MCP response, correlated job, trace, post-refusal Service status and
checksums. An earlier direct attempt used the stream port advertised by a
status projection, but the listener was already absent and returned only local
`ECONNREFUSED`; it created no Agent Browser request or job ID and did not touch
the browser. Credential contents, private URLs, connection tokens and unrelated
tab details remain outside this note.

## SoyLei corrected self-declared recovery retry, 2026-09-08

After maintainer correction at Git `692f3fc9`, the original Plan0137 client
read a fresh access plan using its original `SoyLeiWebsite`, `codex-p0137` and
`final-qualification-r2` labels without the optional profile capability. Access
plan response `r129188` selected `Default`, retained `shared-local` policy
revision1, identified the exact original self-declared subject, and returned
`allowed:true`, no missing permission, `nextAction:proceed`, and the existing
browser/session reuse route.

The client then submitted one capability-free diagnostics request with the
original saved `serviceTabHandle`. Exact response classification:

```json
{
  "success": true,
  "id": "mcp-service-request-diagnostics-04088dda-6dfe-4f53-b932-30c763f63879",
  "error": null,
  "data": {
    "ok": true,
    "action": "diagnostics",
    "controlPlaneAttestation": {
      "complete": true,
      "missingProofs": []
    }
  },
  "terminalOutcome": {
    "state": "succeeded",
    "phase": "execution",
    "effectState": "verified_effect",
    "failure": null,
    "retryDisposition": "do_not_retry"
  }
}
```

The response and retained job use the same request and job ID shown above.
Terminal provenance records the full original service/agent/task subject with
`identityAssurance:self-declared`. Diagnostics recovered the exact retained
target, verified browser owner generation24, matching process identity, active
profile lease with matching handle and requested profile, and managed-launch
owner custody. The attestation is complete with zero missing proofs. Its bounded
URL and title read matched the intended fixed-development contractor-login page.

This closes the original-client reconnect/readiness check as successful for the
retained handle. It does not authorize payment or another consequential action.
The client did not use or rotate the registered credential, adopt or mutate a
lease, replace or refresh the handle, create or close a tab, or mutate the page.
Read-only status after success still showed the exact target ready and handle
valid; the durable child record remains disconnected because the recovered MCP
connection is request-scoped.

Private evidence and checksums are retained outside the product repo under
`~/.soylei-website/operation-receipts/p0137-browser-readiness-r4/`. The directory
contains the full access plan, exact MCP response, correlated job and trace, and
post-success Service status. Credential contents, private connection tokens and
unrelated tab details remain outside this note.
