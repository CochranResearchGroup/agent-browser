# P257 installed retirement diagnostics and dashboard checkpoint

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: [P257](../plans/0257-2026-10-20-retirement-lock.md), CochranResearchGroup/agent-browser#307
Related lanes: P251, P252, P253, P254, P256

## Installed identity

After explicit operator deployment direction, canonical source
`c1ec5b2502c62a03bec736ba221b753ed8c5a6d9` was built once with the release
profile and sealed as `candidate-8537ffba97893a8f-9c8edd62a16f17e7`.

Binary SHA256: `9c8edd62a16f17e79235d0f1bddde3b81be3ac796afca1ac74606e8e266d4d65`.

Embedded dashboard SHA256: `60648363e0a2706a05fca5aea9fa71bff864b0095901ca50db35c6e047dcadaa`.

Executable-input closure SHA256: `8537ffba97893a8f51a1859e41babeb7ef4d292c66b4d2053663d6b3b369b008`.

Production generation: `0.28.0-9c8edd62a16f-3ef0c5e95c2e`.
Transaction: `upgrade-47952d96-97d8-4373-be16-026ef494010c`, accepted, complete
and ready, with admission reopened. This is a local deployment checkpoint,
not a formal release. It includes the previously qualified source changes
since P251, including P253 Native failure guidance and P256 browserless
handoff recovery, alongside P257.

## Qualification

The production-shaped build completed successfully in 17 minutes 1 second.
Those exact sealed bytes were published to isolated development before
production. Development doctor passes, three disposable browser open,
URL-read, close and residue cycles pass, and development publication and
browser testing report production unchanged. The full mounted compact Native
workspace fixture also passes in Chromium with synthetic inventory and viewer
boundaries.

The sealed candidate document check and preserve-mode workstation preview
pass. The preview removes no protected records and retains old-reader and
recovery-artifact compatibility. The supported candidate install transaction
performs the production upgrade; there was no full shutdown, broad cleanup,
Docker/WSL restart, or Native desktop restart requested by this lane.

Production install doctor passes. Both local and public authenticated dashboard
checks verify the exact binary and embedded dashboard identities, render the
browser rail with 16 tab controls, and observe zero JavaScript errors or
status-serialization failure. The supported job-inspection command also
returns successfully. This is not an installed concurrent-retirement stress
test; [source qualification](0257-2026-10-20-retirement-lock.md) owns the
deterministic retirement-contention and handoff preservation evidence.

The complete 13-file source workflow guidance was synchronized to the shared
installed skill after activation. All source file hashes match, additional
installed files remain, and prior guidance bytes were retained privately.

## Preservation and limits

All 14 retained Runtime SQLite browser IDs and 14 profile IDs remain present.
Existing session, tab and handoff IDs are retained. All four runtime-owned
browsers whose pre-upgrade Linux process births were positively observed
retain the exact PID, start token and executable afterward. The separate
Service State browser/session identity projection and durable handoff file
remain unchanged.

Of 273 captured browser/desktop process births immediately before apply, 272
remain exact afterward. One Chrome process created during the development
smoke interval is absent afterward. It was not represented in either retained
runtime browser projection, and its exact origin and exit cause are unproven.
This evidence does not establish full continuity for every process on the
workstation. No process was independently terminated to resolve that finding.
Account authentication and real-site continuation were not exercised.

The pre-existing operator-journey receipt and profile-owner principal-binding
advisories remain explicit. Successful dashboard loading does not prove Native
viewer pixels or human-use acknowledgement. The shared workspace-binary drift
warning concerns the separate uninstalled workspace binary; the selected
production generation and serving manifest match the sealed candidate.

Private build logs, transaction data, snapshots, diagnostic sessions and
validation receipts remain in user-scoped state outside Git. Foreign
worktrees and pinned candidates are preserved. GitHub CI remains disabled.
