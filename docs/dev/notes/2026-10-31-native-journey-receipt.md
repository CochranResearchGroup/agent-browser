# Native operator-journey recording

Work item: [#512](https://github.com/CochranResearchGroup/agent-browser/issues/512).
Product lane: PL-BUGFIX. Owner: ecochran76.

## Problem and evidence

Installed source fe2a3038a, binary3844ae679bf2, passed a disposable authenticated
Native viewer workflow, native input, same-link reconnect and continued task
automation. Workstation status still reported `operatorJourneyReady=false` and
no presentation receipt. The reviewed private acceptance artifact retains the
synthetic workflow, exact selected generation and OS cleanup evidence outside
Git. A fresh generation-bound status assertion reproduces the mismatch in
under one second. A fresh selected-backend manifest read matches its recorded
digest, excluding an existing backend identity mismatch.

The Native callback bypassed the legacy dashboard service-request acceptance
call. That legacy recorder additionally requires its old Service State route,
display and durable presentation records. Native Browser Session State and
provider-qualified viewer-use observations have a different authority boundary.

## Repair

After the browser owner confirms Native use, the presentation poll performs a
fresh private provider read and joins the current exact handoff, browser, tab,
physical desktop, lifecycle and viewing generations. It requires owner-confirmed
human use, delivered-frame acknowledgement and current Chrome process identity.
Anonymous navigation, pending use and caller-supplied success cannot qualify.

The executing dashboard must match the selected backend's executable-bound
manifest. A second backend check occurs before the short writer commit.
Provider and network waits hold no ingress writer lock. The private native
companion receipt binds the full selected backend identity and hashed viewer
and process identities. Status accepts it only for that backend and projects
its explicit `agent-browser.native-operator-journey.v1` schema through the
existing `presentationReceipt` field. It does not fabricate legacy evidence.

The legacy selector registry remains byte-compatible. Recording acceptance
never selects a staged backend or changes fallback/rollback custody. Missing,
malformed or mismatched companion receipts fail closed. A repeated accepted
receipt is a no-op. Existing Native authentication and ordinary human-use
retention remain unchanged.

## Validation and limits

The focused status regression first failed because a qualified Native receipt
was ignored. The four new regression tests pass, covering selected-backend
projection, pending/stale/incomplete proof rejection, idempotent recording,
changed-backend fencing and unchanged selector/staged custody. Expanded Rust,
quality and exact-binary fixture results are pending at this checkpoint.
Documentation build, policy wiring, changed links, handoff guidance, and the
selected VM/Guacamole/PostgreSQL/route-user source fixtures pass.

The separate idle books-receipts profile-lease advisory is owner-scoped and is
not repaired by this work. Preserve peer PR508's inventory/access UI changes.
No production replacement or receipt backfill has occurred in this source
checkpoint; the installed reproduction remains red until deployed repair and
fresh authenticated Native acceptance. Hosted CI remains excluded by policy.
