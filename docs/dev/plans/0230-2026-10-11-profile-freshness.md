# Preserve profile policy during freshness updates

Product lane: PL-BUGFIX
Work item: #176
Branch: fix/profile-freshness-176
State: COMPLETED

An existing legacy per-service profile with two aliases is selectable but a readiness-only mutation rejects its unrelated allocation. Reproduce at the actual dispatcher and serialized repository seam, preserving allocation, aliases, access policy and unrelated readiness. Cover valid per-service and shared-service controls. Correct input validation before effects should return typed no-effect recourse; persistence failures retain uncertainty. No real profile edits, browser launch, credentials or provider effects. Reboot remains deferred.

Accepted evidence: [repair receipt](../notes/0230-2026-10-11-profile-freshness-repair.md). Three public synthetic cases, both regressions and proportional gates passed. No installed runtime publication.
