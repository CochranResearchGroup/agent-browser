# SoyLei native confirmation blocks service observation and dismissal

State: OPEN / investigation requested. Observed 2026-09-08 about13:58 UTC.

During authorized fixed-dev SoyLei Plan0143 staff UX testing, native click on
“Preview center change” succeeded. The app then calls window.confirm with a
validated optimistic center-change preview. No acceptance of that confirmation
was requested. Subsequent service snapshot timed out after30000ms. After exact
job/trace inspection, a second snapshot also timed out. Both action=dialog with
params.accept=false and action=dialog with params.response=status timed out.
The native handler supports both forms (interaction.rs handle_dialog), so the
first dismissal was not an unsupported-command diagnosis.

Expected: a service-owned tab's pending confirmation can be observed and
explicitly dismissed without waiting on JavaScript blocked by that dialog.
Observed: snapshot and dialog control do not finish through the service path.
Hypothesis to test, not established cause: exact-tab admission/activation or an
in-flight native command holds a lock or waits on JavaScript before dialog dispatch.

Private operation receipts are in the operator p0143-fixed-dev receipt directory:
c46-center-us-7.json (successful preview click), c46-center-preview.json,
c46-timeout-job.json, c46-timeout-trace.json,
c46-center-preview-inspected-retry.json, c46-center-dialog-dismiss.json,
c46-center-after-dialog.json, c46-dialog-status.json.
Initial snapshot job: http-service-request-snapshot-5fb055ec-885d-43c5-baa3-59ec34529acd.
Typed terminal outcome: service_job_timed_out / effect_uncertain /
inspect_before_retry. Full service-tab handles and credentials remain private.

Service subject: SoyLei / p0143-root / fixed-dev-split-fulfillment-qa.
The operator's current browser handle is in c45-new-tab.json. Do not impersonate
another tab owner, close peers, reset profiles, or accept the pending confirmation
as a recovery shortcut. Requested recovery is read/explicit dismissal of this
owned confirmation, preserving the browser and other tabs.

SQL readback confirms the proposed QA center was not created. The fixed-dev
mutation lock was released after investigation. The test staff cookie remains
in the owned browser; its server token is short-lived and should be cleaned up
once control returns. HTTP/mail containment remains active. No payment, warehouse
export, or production website effect occurred.

Separate earlier incident: a13:47 submit was refused with
profile_child_access_record_missing/no_effect after retained-browser health
changes. Fresh access-plan-authorized new-tab acquisition recovered that incident;
the old handle was not retried. Do not conflate it with this confirmation timeout.

## Recovery readback

Later C47 fresh service status retained exact task ownership and tab_close_own
permission. Service action tab_close on that owned handle succeeded:
physicalTabClosed=true, targetRemovalVerified=true, unrelatedTargetsPreserved=true.
No confirmation acceptance or browser restart was used. Receipt:
c47-owned-tab-close.json. The snapshot/dialog timeout defect still needs
investigation; closing the tab is recovery, not a fix.
