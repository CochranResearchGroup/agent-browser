# Plan 0131 Slice E Production Controlled Fixture Acceptance

Date: 2026-08-26

Result: PASS

Accepted state: `production_controlled_fixture_accepted`

Authority after this record: NONE. This record does not authorize a formal
release, Slice F, or a real authentication or credential workflow.

## Outcome

The production workstation now runs the exact reconciled Plan 0131 candidate,
and one repository-owned controlled X11 fixture completed successfully on the
third static route. The fixture did not close, take over, park, or displace an
existing workload. The route remains installed as spare capacity after exact
fixture cleanup.

In plain English, the accepted candidate found the fixture's target, moved the
pointer, clicked it, typed the fixed harmless fixture text, and independently
confirmed the fixture changed to its expected final state. Submitting the same
operation again returned the saved result instead of performing the input a
second time.

## Accepted Source And Installed Identity

- Branch: `feature/plan0131-production-candidate-reconciled`
- Source head: `cdb883f533ee26f368e3b427511c7818494d7e06`
- Candidate version: `0.28.0`
- Candidate SHA-256:
  `32e8c9318bebfd6de2ed4027cbeb0f4d0416766b7264c63ba103ee5d759782c2`
- Candidate generation: `0.28.0-32e8c9318beb-b2bd0fba532f`
- Runtime manifest SHA-256:
  `48ffad0bc36009833c0c99faafb752479db0227dec1c92658d4a8fa52af71c60`
- Accepted workstation transaction:
  `upgrade-337eda48-7200-4fca-8f08-76e6792db568`

The release build completed in 12 minutes 15 seconds. Cargo ran through the
repository wrapper with its normal job and cgroup protections. The only
authorized override was `AGENT_BROWSER_CARGO_MINIMUM_SWAP_FREE_KIB=0`.

The installed command SHA-256 matches the candidate. Workstation status reports
the transaction as `accepted`, all seven readiness axes as ready, and the
managed dashboard backend and presentation receipt bound to the same candidate
generation. Install doctor reports `steady_current` with one runtime host, one
dashboard process, one executable generation, and zero legacy daemons.

## Source Repair Discovered During Live Validation

A harmless service reconciliation originally acquired controller mutation
fences for every controlled route even when controller authority was unchanged.
That could cancel an interaction already in progress. Source head `cdb883f5`
changes reconciliation to compare the old and new primary controller lease and
epoch first, then acquire the route fence only for routes whose controller
authority actually changes.

The new regression
`reconcile_fences_only_primary_controller_authority_changes`, the existing
controller cancellation regression, Rust formatting, and strict Clippy all
passed before rebuilding and installing the accepted candidate.

## Controlled Fixture Receipt

The accepted fixture used logical browser
`session:plan0131-production-fixture-v2`, route `guacamole:3`, display
allocation `remote-view-display:16`, and controller epoch 4. Operator-visible
state was ready before the successful interaction.

Operation request `plan0131-d82c213c61534735bc577348fab89381` produced
transaction `desktop-interaction-3f3b29b28dec7e2ad91c5938` with:

- `effectState=verified_success`;
- `verificationState=passed`;
- `cleanupState=released`;
- 34 planned, attempted, and acknowledged effect keys with equal digests;
- six pointer events and 13 characters of fixed benign text;
- present after-context, frame, and observation evidence;
- `retention=ephemeral` and `persistedPixels=false`;
- no stop reason.

The first execution completed in 161 milliseconds. Repeating the same canonical
operation returned `replayed_terminal`, the same transaction identity, and the
same 34-effect receipt. It did not execute the input again.

## Retained Fail-Closed Evidence

Live acceptance retains the failed attempts instead of rewriting history:

- Early workstation attempts could not prove candidate presentation because
  the watcher was absent, expected the wrong manifest field, or used a plain
  page request that did not perform the authenticated handoff resolution. Each
  attempt failed closed and preserved the old selected generation. One later
  attempt also encountered a transient CDP `Runtime.enable` timeout; the next
  bounded direct probe succeeded before the accepted transaction.
- Requests with a mismatched desktop identity, provider agent name, or viewer
  lease failed before input.
- One request stopped before a desktop transaction because another exact
  profile lease owned the lane.
- Operation request `plan0131-7d2557f2058642458b5e37b796c7c84e` produced
  transaction `desktop-interaction-94ce9bf9049dd43c8cbc25b3`. It acknowledged
  nine pointer-movement effects, then stopped as
  `desktop_interaction_authority_changed` before any click or keyboard input.
  It remains `effect_uncertain`, has no after-state verification, retained no
  pixels, and was never retried.

The uncertain receipt is expected safety behavior: once authority changed, the
system refused to guess whether continuing was safe. The accepted operation
used a new operation identity only after the source defect and runtime state
were reconciled.

## Cleanup And Current State

The exact fixture controller lease and the earlier incorrect fixture lease are
both disconnected. The route has no controller and no viewer leases. The
fixture browser record is absent, its Chrome process is absent, and the
repository fixture process is absent.

Route `guacamole:3` is `released`; display allocation
`remote-view-display:16` is `released` with no route IDs. Pool entry
`guacamole-rdp-3` is `available` with no current allocation. Its static
display `:16` and route user remain installed as the approved spare capacity.

Service resources report zero cleanup candidates and no Plan 0131 fixture
resource. Workstation GC dry-run reports zero generation deletion candidates.
The previous healthy generation remains intentionally retained for rollback.

The only install-doctor issue is the pre-existing
`service_duplicate_profile_pressure` warning for `last30days-facebook`. It is
outside Plan 0131 and did not affect the fixture route or accepted transaction.

## Post-Integration Validation

Remote main commit `4f79b89d` already contained the earlier Plan 0131 candidate
merge and its post-integration gate note. It was merged into this branch after
the acceptance receipt without conflict. The four later production hotfixes
and this receipt remain the branch's unique integration delta.

The validation selector chose diff hygiene, Rust formatting, strict Clippy,
six workstation and Guacamole fixture contracts, the workstation installer
Rust module, the service-health Rust module, and a selector self-check. Every
selected gate passed on the merged branch.

The first installer-module run used Cargo's default parallel test execution and
reported one PID-identity test failure while another Cargo lane was active. The
same exact test passed alone, and the complete 92-test installer module then
passed with `--test-threads=1`. The 78-test service-health selection also passed
serially, including the new controller-fence regression. This is classified as
parallel test interference, not a product failure. The failed result remains
part of the validation history.

## Boundary After Acceptance

Slice E is complete. This acceptance proves the controlled production input
boundary and makes Plan 0110 closure eligible, but it does not itself perform
Slice F. The next recommended step is review and integration of this reconciled
feature branch, followed by separately authorized Slice F planning updates.
Formal release and any real credential workflow remain separate maintainer
decisions.

## Merge Review Remediation

A later review against the production hotfixes found three source-level merge
blockers. They did not invalidate the retained live acceptance receipt, but the
branch was not ready to merge until they were corrected:

- reconciliation could restore stale controller authority if another
  controller took over while the asynchronous health probe was running;
- transitional runtime-host acceptance compared only PID and generation, so it
  did not reject every form of PID reuse or binary and socket substitution;
- the blank route-pool override regression inspected source text instead of
  exercising the environment-loading behavior.

The reconciled source now preserves a newer controller lease, controller epoch,
and viewer set while still applying fresh health observations. Transitional
runtime hosts must match the transaction's PID, process start token, binary
hash, generation, and socket identity. The route-pool test now loads an actual
fixture environment and proves that an intentionally blank override remains
blank.

In plain English, a background health check can no longer undo a controller
handoff that happened while it was checking the browser. An upgrade can no
longer mistake a reused process number or a different binary or socket for the
runtime host it approved. The environment regression now tests what the code
does instead of checking how the code happens to be written.

The remediation passed strict Clippy, Rust formatting, all 92 workstation
installer tests, all 78 service-health tests, focused runtime-multiplicity and
host-identity regressions, the route-inventory behavior test, six workstation
and Guacamole fixture suites, the remote-view documentation contract, and the
production docs build.

This is a source merge-readiness update only. The installed production
candidate remains the accepted `cdb883f5` artifact identified above. No binary
was rebuilt or installed, no workstation transaction was started, and no live
runtime or provider state was changed during merge remediation.
