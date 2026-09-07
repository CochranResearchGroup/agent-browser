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
