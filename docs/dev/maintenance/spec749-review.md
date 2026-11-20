# Spec 749 single final review and remediation

Product lane: PL-PLATFORM

Review pass: 1. Agent Browser source `e6de8b5b312dc529d6b8d527e4df6c70e80a8365`, fixed baseline `ac0643a29b31b91a908ec0b629e0e02dd67347f8`. Remote View source `655231f9425868b3da92bdf551956ed85819be09`, baseline `79a2a4d100f783df18050530b5c0d202b6103b8a`. Standards and Spec were reviewed independently. The coordinator verified the candidate source paths and adjudicated the findings below. No second broad review is planned.

## Standards

| Finding | Evidence and rule | Disposition |
| --- | --- | --- |
| R1: profile recovery/reset expiry | `service_profile_recovery.rs` supplies `service_now_timestamp`, implemented with guest UTC, to authority-bearing expiry checks. Violates the user StableTime authority rule. | blocking, #765 |
| R2: Auth/challenge relative deadlines | `service_authentication_run.rs` and `service_challenge_task.rs` mint/check relative deadlines using guest UTC. Reliable current UTC alone cannot qualify a legacy start. Same rule. | blocking, #766 |
| R3: finite handoff origin/check | `browser_session_host.rs`, `stream/remote_view_presentation.rs` and `stream/native_browser_mode.rs` mint/check finite retention using guest milliseconds. Violates StableTime authority and policy 0054 continuity. | blocking, #767 |
| R4: provider timing failure attribution | Remote View `human_activity.rs` maps transport, authorization, body-read and HTTP failures to service unavailable. A client failure is not evidence that StableTime is down. Violates the user failure-attribution rule. | blocking, #769 |
| R6: startup ownership from guest age | Coordinator-verified `connection.rs` removes startup markers using `modified().elapsed() > 30s`. This is ownership eligibility, not an incidental wait. Violates StableTime authority and can steal a live admission. | blocking, #768 |

R1, R2, R3 and R6 are retained defects in touched workflows, not claims that this batch introduced their original clocks. The approved audit and continuation scope makes their current blocking behavior material. No additional smell-only candidate warrants expansion; tooling-enforced style remains a local gate.

## Spec

| Finding | Requirement and evidence | Disposition |
| --- | --- | --- |
| R1 | Distinguish unknown/stale/unavailable from positively denied. Guest UTC can falsely expire recovery permission. | blocking, #765 |
| R3 | Exercise clock uncertainty/outage and preserve same-link continuation. Finite link checks remain guest-authoritative. | blocking, #767 |
| R5: quiet-period presentation | Explain other-viewer activity and show meaningful remaining quiet time. Provider returns `remainingMs` and a qualification cause, but the new toolbar displays neither. | blocking, #769 |
| R7: explicit improvement budget | The spec requires an evidence-backed budget after the minimal paired baseline. That baseline now exists; the budget must be recorded and verified before further measurement. | blocking, #768 |

The latency budget is functional: a healthy exact warm continuation must complete while unrelated startup admission remains held, without adding a default StableTime dependency to that warm path. The measured brackets are telemetry, not a numeric SLA or precise speedup ratio. Preserve the existing smallest controlled pair and verify the actual kernel-guard seam after repair.

No scope-creep finding was accepted. Two owner Auth compatibility tests fail because both available historical Receipts inputs reject current Auth's null pending-approval expiry. The frozen baseline already uses the same receive operation. This is an external input-protocol limitation, not evidence for weakening Agent Browser authorization. Number-matching compatibility passed. Keep the original request inspect/cancel/history paths and distinguish compatibility evidence from current Receipts integration.

## One bounded TDD pass

Pass: 1, active. Four initial behavioral RED cases are preserved at1046932d0; first profile planning GREEN atbd33 and five Auth/challenge/handoff/startup cases pass at3899f4b26. The second profile apply slice is behaviorally RED at3899f4b26; qualified apply source awaits GREEN. The five remediation tickets are the complete accepted finding set. Root owns #765 shared time/profile expiry and integration; one bounded provider worker delivered #769 diagnostics/presentation, with capacity performing source qualification after #766. Capacity owns #766, coordination owns #767, latency owns #768. Existing auxiliaries are reused; there is no new architecture lane or test project.

Existing public seams: profile recovery/reset commands; Auth/challenge start/status/resume/cancel and original effect readback; retained handoff/store/native mode routes; connection warm reuse and paired measurement; provider Native activity/control and toolbar. Time is injected only at the existing external service boundary. Preserve exact RED and GREEN commands and source identities. Tests that selected zero cases or failed setup/cleanup do not count as passing acceptance.

Root remains the primary writer for shared time adapter, schemas, catalog, generated exports and shared help/docs. Workers first deliver a behaviorally failing test at an existing seam, then implement only after its RED receipt. Relative-origin persistence compatibility and shared types must be agreed before overlapping edits. No service I/O inside a global writer, guest extrapolation, implicit permission extension, new viewer grant, production install, provider deployment or hosted CI.

Review summary: Standards accepted 5 blocking findings, highest severity authority expiry/ownership; Spec accepted 4 blocking findings, highest severity same-link and permission continuity. Seven unique findings map to five bounded remediation tickets.
