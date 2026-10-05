# Plan 0146 | Service State Stale-Revision Operability Repair

Date: 2026-08-31

State: CLOSED

Lane: P146

Branch: `fix/service-state-stale-revision-recovery-20260831`

Target: `main`

Source baseline: `9fd50ec9ebbf3c15200fc2815d9f8a362d60d4b7`

Implementation checkpoint: `c22a208703bf8efdc22d87b51784b25e5866f707`

Integration checkpoint: `f1029320ff93641388325d38c77af8c18d997e3f`

Authority: SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, ISOLATED DEVELOPMENT
RUNTIME PUBLICATION, AND EXACT PRODUCTION CANDIDATE INSTALLATION ARE IN SCOPE.
PRODUCTION BROWSER, PROFILE, ROUTE, TENANT, OR PROVIDER EFFECTS ARE OUT OF SCOPE.

## Incident

SoyLei browser evidence failed at jobs `r591783` and `r813532` with
`service_state_stale_revision`, first at revision 13 to 14 and then at revision
20 to 21. At those timestamps, the selected production runtime host and the
retained fallback runtime host were both live with the same production home
and different generation sockets.

The exact writer of revisions 14 and 21 cannot be recovered because the
Service State revision did not retain writer provenance. The causal product
defect is independently reproducible: Plan 0142 moved mutation preparation
outside the cross-process file lock and its realistic fixture manually retried
the resulting stale revision in each test writer. Ordinary commands did not
share that retry wrapper.

## Goal

Make selected and fallback runtime hosts coexist without invalidating prepared
Service State mutations. Give an operator one bounded command flag that can
extend the contention wait without overwriting state or weakening another
authority boundary.

## Work Units

1. Remove caller-local stale-revision retry from the realistic fixture and
   preserve the red failure.
2. Hold the cross-process file lock from authoritative load through prepared
   durable commit while releasing the process mutex before serialization.
3. Add a deterministic competing-writer fixture and retain the realistic
   2.9 MiB mixed-reader and writer fixture.
4. Add `--service-state-lock-timeout-ms <ms>` as a per-command, five-minute
   capped operator override. It changes only the serialized-lane wait budget.
5. Update CLI help, README, repository skill, docs site, and inline source
   documentation.
6. Repair the pre-existing launch-session precedence regression exposed by the
   repository split test runner. Launch requests inherit the effective daemon
   session, while non-launch service actions retain explicit browser-session
   targeting on a shared runtime host.
7. Run focused Rust checks, formatting, strict Clippy, selected validation,
   development publication and doctor, and a disposable browser smoke using
   the override.
8. Install only the exact development-approved candidate into production,
   run install doctor, and verify selected generation, runtime-host
   multiplicity, Service State health, and a disposable no-provider command.

## Acceptance

- Two competing prepared writers both commit exactly once with revisions 1
  and 2 and no lost state.
- The realistic fixture passes without a caller retry loop, lock timeout, or
  duplicate logical effect.
- `set viewport 1440 1000 --service-state-lock-timeout-ms 30000` reaches the
  daemon with the requested bounded wait.
- Launch session inheritance and explicit non-launch browser-session routing
  both pass their complete serial route-host fixture partition.
- Development doctor and three-cycle disposable browser smoke pass.
- Production install doctor passes for the exact candidate hash installed
  after development acceptance.
- No private profile, remote-view provider, or tenant site is used for
  acceptance.

## Stop Rules

- Do not overwrite Service State, delete a lock, or kill a browser to make a
  fixture pass.
- Do not treat the selected and fallback host overlap itself as a defect; the
  defect is unsynchronized shared-state mutation during that supported window.
- Stop production installation if source validation, development doctor, or
  disposable development browser smoke fails.
- Preserve unrelated active worktrees and the P144 and P145 write surfaces.

## Completion

The source repair, bounded lock-wait override, audited browserless installation
override, development qualification, and exact production installation are
complete. Production transaction
`upgrade-c9a883aa-7685-4bc0-be01-197ec21ac0a9` accepted generation
`0.28.0-fa99bc026aa4-a04fbee7185d` with binary SHA-256
`fa99bc026aa47db43141888876e38afa90cc8976c58286d301d38962be3d895d`.

Current production doctor reports one selected runtime host, one dashboard
process, one executable generation, converged runtime and dashboard state, and
zero blocking issues. The exact development and production viewport acceptance
both returned 1440 by 1000. See
`docs/dev/notes/0150-2026-08-31-plan-0146-service-state-stale-revision-repair-acceptance.md`
for the root-cause boundary, validation, installation, reconciliation, and
residue evidence.

The completed branch was integrated into `main` by fast-forward through
`f1029320ff93641388325d38c77af8c18d997e3f`. The declared overlap with
`plan/lease-authority-coordination` is reconciled because that coordination
branch contains the Plan 0146 checkpoint and continues from it without
rewriting the repair.
