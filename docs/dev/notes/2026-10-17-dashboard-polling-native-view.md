# Dashboard polling and native embedding repair

Product lane: PL-BUGFIX. Execution lane: UX301. Work item: #301.
Disposition: active-input
Owning plan or work item: CochranResearchGroup/agent-browser#301
Related lanes: Remote View plan0132 / #456; Cooper native workspace ingress

## Outcome and scope

Restore the existing workflow: discover a live browser promptly, select its tab,
and keep the native Remote View desktop embedded. Live inventory must not wait
for full service reconciliation and workstation diagnostics. The native viewer
continues to own desktop operation and access controls.

The production Chrome reproducer reaches focus and handoff success, receives
the opaque locator redirect, then loses the iframe and reports failure. The
installed public origin points at a separate Remote View host, but workspace
framing requires a shared origin. Production ingress has only a legacy
Guacamole path; the development native ingress configuration is not installed
for production. Earlier substituted viewer documents did not qualify this seam.

Three consecutive live status responses also report no verified live browser.
Forced summary reads take about four seconds and the completion-driven poll
waits another three seconds. The owner state read takes about ten milliseconds.

## Bounded repair and acceptance

1. Give the existing status endpoint an explicit workspace projection which
   reads the redacted owner inventory and bounded native/CDP observations without
   service reconciliation or workstation diagnostics. Use it in the native UI.
2. Resolve iframe navigation onto native workspace routes at the dashboard's
   shared origin. Standalone opaque links retain native continuation.
3. Install the corresponding production native ingress paths using the existing
   provider and authentication. Preserve unrelated operational checkout changes.
4. Compare native browser windows with owner records; distinguish unowned
   observed windows from managed automation tasks. Empty desktops stay excluded.

Acceptance requires actual HTTP latency and Chrome embedding evidence, including
the real native framing policy. Synthetic viewer replacement cannot establish
installed iframe success. Preserve live browsers, tabs, profiles and handoffs;
do not renew TTLs, recreate tasks or change authentication policy for this repair.

Private runtime evidence stays outside the repository. Source checks and installed
proof remain separate. This note records active work, not completion.

## Source checkpoint

Authenticated production Chrome confirms native sign-in and both task requests
succeed, then CSP blocks the cross-origin iframe. The native browser inventory
endpoint is absent (404). Existing polling completes roughly every six to seven
seconds. This is a real native document boundary, without a substituted viewer.

The dedicated HTTP workspace regression first fails when it enters the diagnostic
pipeline, then passes in 0.01 seconds after bypassing launch-admission, integrity,
backup and reconciliation work. Operational-status redaction checks still pass.
Managed handoff continuation, compact UI, routing, type checks and native-window
identity tests pass. The actual native-window components/hooks preserve one
same-desktop connection across Alpha/Beta selection, issue three exact explicit
window effects, refuse changed generations and invoke no managed task API.
That UI fixture has a synthetic provider boundary and is not installed proof.

Remote View #456 owns native origin aliases, same-host reauthorization and a
read-only browser-window endpoint using its existing discovery and native probes.
Only current browser-class non-dialog windows enter the rail. A failed peer does
not hide a verified browser. Legacy daemon history stays in diagnostics. Native
selection calls Remote View's existing connected helper, preserving its input
lock and generation checks; no managed session or tab is invented.
