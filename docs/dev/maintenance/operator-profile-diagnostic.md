# Advisory profile lock diagnosis

Product lane: PL-BUGFIX
Parent: P300, ticket 530
Branch: fix/operator-profile-diagnostic
Base: db8da1e3f1571b9750eb205dfeb0eba68b23f903
State: ACTIVE

## Bounded outcome

The legacy ServiceState profile diagnosis must not reject an operator-authorized
workflow solely because a live Chrome singleton lock has no legacy browser join.
Preserve the observation and lock; leave actual process binding, foreign custody
and uncertain-effect checks in the execution path.

This source lane owns service_profile_diagnosis.rs and its documentation. The
existing P300 recovery lane retains host publication, desktop allocation, shared
runtime deployment and donor custody. The additional primary checkout was admitted
after a clean inventory and remote refresh under the repository admission lock.

## Acceptance and stop condition

A real current-process singleton-lock fixture must first fail the nonblocking
assertion, then pass with truthful processState=not_recorded, Ready state and
continue_authorized_workflow recourse. Diagnosis must preserve state and the lock.
All diagnosis regressions and applicable local formatting checks must pass.

The installed live reproduction showed a blocked legacy diagnosis while ordinary
commands on the exact existing browser subsequently succeeded. Native install
doctor independently reported convergence and operatorJourneyReady. A later
browser_session_publication_conflict recurred; this advisory fix does not claim
to resolve that separate failure. No effect replay or shared runtime restart is
in scope. Installation acceptance is separate from source completion.

## Validation

RED: focused occupied_lock_diagnostic_does_not_veto_authorized_workflow failed
at the nonblocking-finding assertion, with zero compiler errors (0.01s test).
Command: scripts/ci/rust-tests.sh --focused occupied_lock_diagnostic_does_not_veto_authorized_workflow
through Cargo Signal and cargo-safe, using the existing shared target cache.
GREEN: all six service_profile_acquisition::diagnosis::tests pass (133.98s
build and run, zero compiler warnings/errors). The suite retains foreign-profile
join rejection and stale-lock repair coverage.
Private build evidence remains outside Git. Formatting, strict workspace Clippy
and docs-site compilation are being finalized before the source checkpoint.

The identified cause is the legacy diagnosis treating occupied_unproven or
observation_failed lock evidence as blocking without a canonical runtime join.
The repair retains the evidence as a warning and recommends continuation through
actual exact-resource checks. It grants no authority to clear locks or replay
an uncertain effect. No debug instrumentation was introduced.
Hosted CI is excluded; no provider writes or production installation occurred.
