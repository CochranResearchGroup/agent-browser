# App routing, borrowed browser custody and cooperative control

Status: Accepted
Date: 2026-10-22
Work items: #347, #348, #349, #350

## Decision

Keep browser task/profile/automation integration in Agent Browser and native
viewer/desktop ownership in Remote View. Reviewed application configuration
selects app ingress and provider identity; callers never supply arbitrary viewer
redirects. Opaque app handoffs retain exact participant/session/task/native target.
Standalone handoff remains independently available.

Attachment observes exact process/start/executable, canonical profile, CDP browser
PID, page and Native assignment/window before publishing. Catalog visibility and
borrowed custody share one transaction. The original complete page census remains
borrowed; later explicit sharing creates independent task attribution without
transferring termination or relocation permission. Missing app browsers are not
implicitly replaced. App-directed relocation and loss/rebind are separate work.

One durable cooperative control journal fences supported mutation dispatch across
MCP, CLI and Service. Viewing does not take over. Explicit takeover changes the
current epoch before admitting new work; optional read pause preserves status and
return. An in-flight operation retains its outcome/uncertainty. Explicit return
freshly observes the original target without recovery or effect replay. Scoped
operator override uses existing authority plus attribution, with no new viewer
ceremony. Arbitrary OS input and unmanaged CDP interception are not promised.

## Consequences

The app can retain placement and process lifecycle while using higher-level
Agent Browser controls. Compact metadata separates custody/control from browser,
viewer and account readiness. Source qualification and isolated Native evidence
do not imply installed configuration, deployment or completion of the wider #347
program. A second viewer, blanket app-process adoption, silent profile duplication
and timeout-driven return would violate this boundary.
