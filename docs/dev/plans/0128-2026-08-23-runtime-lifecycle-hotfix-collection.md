# Plan 0128 | Runtime Lifecycle Hotfix Collection

Date: 2026-08-23

State: OPEN

Execution state: `source_followup_accepted_install_pending`

Lane: P128

Branch: `hotfix/runtime-lifecycle-collection`

Follow-up branch: `hotfix/runtime-source-session-selection`

Target: `origin/main` at `88418a99b7eb76cb995421f89c5ece93dc8ccd19`

Authority: SOURCE, PROVIDER-FREE DEVELOPMENT, AND TRANSACTIONAL WORKSTATION HOTFIX

Depends on:

- `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`
- `docs/dev/plans/0127-2026-08-23-development-presentation-provider-isolation-plan.md`
- accepted branch `fix/reconcile-absent-closing-lifecycle` at `ac0864cb`
- Books Receipts validation receipt
  `docs/dev/validation/0232-i6-scope-inspection-agent-browser-lifecycle-blocked.md`

## Goal

Integrate the accepted absent-closing lifecycle hotfix with current main,
prevent failed lifecycle registration from leaving a live service-unowned
browser, restore one supported recovery path for an exact live named-profile
lane, and prevent a historical stopped supervisor advisory from being
misreported as a global installation failure.

Preserve current development-presentation feature work, every unrelated live
browser and profile, the accepted workstation rollback generation, and all
provider and accounting effect boundaries.

## Current Incidents

### BILL named-profile acquisition

The `bill-soylei` lifecycle records old process group `86911` as
`terminal/satisfied`. A later service launch created process group `83894`
under the selected runtime host, then failed
`runtime_lifecycle_terminal_replacement_rejected` before persisting a service
browser or session. The live process is protected by the durable profile but
cannot be reused by the broker.

### Historical supervisor projection

The selected workstation is converged on one runtime host and one executable
generation. A retained `last30days-home-feed` supervisor manifest still
projects an inactive shared systemd unit and contributes a warning to install
doctor. The warning is useful maintenance evidence, but warning severity alone
must not make an otherwise accepted workstation report global failure.

### Branch reconciliation

The accepted absent-closing branch contains source repairs not yet integrated
into current main. Current main independently contains active development
presentation work and commit `88418a99`, which prevents terminal history from
rehydrating an effect-capable owner binding. Integration must preserve both
lines and renumber the lifecycle hotfix documentation to P128 because current
main already owns P127.

## Acceptance Criteria

1. Current main and the accepted absent-closing branch are reconciled without
   replacing or weakening current development-presentation behavior.
2. Exact `closing/owned` absent-process reconciliation, next-generation
   terminal replacement under a collision-free logical browser ID, terminal
   binding suppression, and correct shutdown aggregation remain covered by
   deterministic regressions.
3. If a newly launched owned browser cannot register lifecycle and service
   ownership, the launch fails closed and exact cleanup leaves no browser
   process, profile lock, service browser, service session, or cleanup
   obligation residue.
4. An exact live named-profile browser without service browser/session
   projection can be recovered only through existing process, profile, CDP,
   target, owner-generation, and runtime-host evidence. Ambiguous, foreign,
   locked, or identity-mismatched processes remain protected and unchanged.
5. After provider-free recovery, the access plan returns
   `reuse_existing_browser` with valid route hints, or the exact live lane is
   lawfully closed and one next-generation replacement succeeds. One harmless
   local tab can be acquired and released with no duplicate profile process.
6. Install doctor retains the stopped supervisor warning in structured health
   evidence but does not return global failure when every non-advisory
   installation, convergence, ingress, and rollback axis is ready.
7. A genuinely required or active stopped supervisor, executable mismatch,
   non-warning doctor issue, runtime multiplicity drift, or workstation
   readiness failure still fails doctor.
8. The historical `plan0233-qbo` browser remains separately classified from
   current runtime readiness. Any retained-state cleanup uses the supported
   dry-run and reviewed apply surface and preserves incident evidence.
9. Focused Rust tests, canonical formatting, strict Clippy, selected contract
   and fixture tests, docs validation for changed user-facing behavior, and
   diff hygiene pass.
10. A transactional workstation install selects the exact reviewed candidate,
    preserves unrelated browsers and the isolated development runtime, and
    passes a fresh harmless BILL-profile acquisition proof without opening
    BILL or QBO.

## Execution Units

1. Reconcile the accepted absent-closing branch onto current main and resolve
   the P127 documentation collision as P128.
2. Add one red provider-free regression for terminal replacement failure after
   process launch and make launch registration atomic with exact cleanup.
3. Add one red provider-free regression for exact live named-profile recovery,
   then deepen the existing lifecycle/adoption seam instead of adding a second
   owner registry or consumer-side process bypass.
4. Separate warning-level supervisor health from blocking install-doctor
   failure while retaining the full structured observation.
5. Validate source and development-runtime behavior, checkpoint, and publish
   the branch.
6. Build and transactionally install the reviewed candidate. Use the installer
   dry-run and exact census before apply; stop if it classifies any live lane
   ambiguously.
7. Recover the exact `bill-soylei` lane through Agent Browser ownership, then
   acquire and release one harmless local tab. Do not navigate to BILL or QBO.
8. Re-read supervisor, historical QBO, multiplicity, resources, incidents,
   profile allocation, lifecycle, and rollback state before closeout.

## Bounds

- two implementation attempts per failing behavioral seam before local
  replan;
- one source reconciliation and one closed-world remediation pass;
- one transactional candidate dry-run and one apply after exact admission;
- one harmless `bill-soylei` recovery and tab acquisition proof;
- one reviewed retained-state cleanup pass only if dry-run proves the target is
  historical and unreferenced;
- no provider navigation, authentication inspection, profile reseeding,
  accounting effect, raw process kill, manual state edit, route theft, or
  formal release publication.

## Hard Stops

- Do not mutate the current main worktree or the P127 development-provider
  worktree.
- Do not hand-edit Service State, runtime-owner state, profile locks, or
  supervisor manifests.
- Do not adopt a browser from profile path alone.
- Do not close process group `83894` unless Agent Browser first proves exact
  lifecycle authority and uses its owned close path.
- Do not hide or delete the supervisor warning merely to make doctor green.
- Do not install a candidate while census, rollback, runtime multiplicity, or
  operator-journey evidence is ambiguous.
- Do not open BILL, QBO, or another authenticated provider during repair
  qualification.

## Initial Evidence

- source target: `88418a99b7eb76cb995421f89c5ece93dc8ccd19`;
- accepted lifecycle branch: `ac0864cbefd169aa90c2ceffcf33539bf5fc40d1`;
- installed generation: `0.28.0-a89625b870c3-1e2c09b12ebc`;
- installed SHA-256:
  `a89625b870c3cda3cde9b41f27271ebe36d60683b1235c4196c4be337bb39ea6`;
- runtime multiplicity: one dashboard, one runtime host, one executable
  generation, zero legacy daemons, `steady_current`;
- BILL old lifecycle: process group `86911`, `terminal/satisfied`;
- BILL current protected process: process group `83894`, no service browser or
  session ownership;
- failed acquisition job:
  `mcp-service-request-tab_new-2a40e988-8d11-4ad3-99ee-1fdc306fb804`;
- supervisor observation: `last30days-home-feed`, unit inactive/dead, warning
  `supervisor_stopped`.

## Transactional Install Follow-up

The first reviewed candidate, SHA-256
`7d19b21c7801bbed90ca398967662b4e3fbf121c851bcbaef27e93d376cc583d`,
failed closed before activation in transaction
`upgrade-56f5c32a-d939-4610-a299-fee113b5571e`. The selected generation
remained unchanged, and supported reconciliation removed the failed candidate
without terminating a live browser.

The exact live owner route for the `last30days-facebook` browser prepared a
cooperative handoff. A historical alias on the same shared runtime host then
returned the legacy diagnostic `browser PID is unavailable`. The installer
treated that browserless alternate as a blocking command failure even though
the exact browser-bearing primary was already selected. The bounded follow-up
classifies that diagnostic only for `handoff prepare`, retires it only when it
comes from a non-primary alternate after valid owner selection, and keeps the
same result blocking for a primary route.
