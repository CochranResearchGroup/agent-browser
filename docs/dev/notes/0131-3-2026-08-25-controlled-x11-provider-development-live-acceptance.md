# Plan 0131 Controlled X11 Provider Development Live Acceptance

Date: 2026-08-25

Result: PASS

Accepted state: `development_live_accepted`

Authority after this record: NONE. Slice E requires fresh explicit authority.

## Accepted Boundary

Plan 0131 Slices A through D are accepted for the isolated development runtime.
This record does not accept production input, close Plan 0110's live Foundation
Acceptance boundary, authorize a formal release, or authorize a real
authentication or credential workflow.

The accepted source head is `116f47f8`. The installed development generation
is `0.28.0-86026cf08fe7`, built from candidate SHA-256
`86026cf08fe76352b92a52312a6b8992c9c0bc8fba455e8cf58d8e461382429b`.
The production generation remained
`0.28.0-05d9da26035e-7fa3fbcb7248` with command SHA-256
`05d9da26035e0e86b55d6b2beaed25ae6dfe45ee6eeb0aa14362ce4ec08b0d10`.

## Controlled Live Receipt

One repository-owned fixture ran on browser `session:default`, managed profile
`p131-controlled-x11`, route `development-route-1`, display allocation
`development-display-1`, display `:12`, stream `remote-headed-view`, and
controller epoch `1`. Operator-visible state was `ready` before input.

Operation scope
`1d1786219a9b117f2258e5f92a19ebc8039e421f292fc5a262226a04d703ba1b`
completed as `verified_success`. Its receipt records 28 attempted effect keys,
28 acknowledged effect keys, a passed independent after-state verification,
released cleanup state, ephemeral retention, and `persistedPixels=false`.
The before and after frame SHA-256 values differ, and the after observation is
bound by SHA-256 without retaining pixels or plaintext input.

The same canonical operation was submitted once more. Both service jobs
completed with `effectState=verified_success`, while the private effect journal
remained at 88 records and the operation ledger remained at three records.
No effect record was appended by replay. The stored first-execution receipt is
unchanged; the replay response is projected as terminal replay at request time.

## Retained Fail-Closed Evidence

The first live scene was rejected as ambiguous before input because text split
the exact target color. A later operation acknowledged 31 of 32 attempted
events and retained one uncertain key-down after the X11 hyphen keysym defect.
That operation was never retried. A remediated operation acknowledged all 28
events but failed independent after-state verification because label text split
the exact verification marker. That operation was also never retried.

One bounded remediation packet made the target and verification regions solid
and mapped the admitted hyphen to the registered X11 `minus` keysym. The final
success used a new operation ID. The private journal intentionally retains 87
acknowledged records and one uncertain record across all attempts.

## Fresh Safety Audit

The final source audit found no new blocking Plan 0131 issue. The focused
provider suite passed 10 tests, including same-route exclusion, unrelated-route
progress, acknowledged replay with zero new X11 emission, abandoned prepared
reopen as uncertain, bounded partial-effect handling, exact development
generation admission, and production rejection. The controller suite passed
four tests, including mutation waiting on the external route fence, epoch
advance, cancellation, and unrelated-route independence. The interaction suite
passed 28 tests, including ledger reload replay, abandoned in-progress reload,
focus and geometry drift, route authority drift, bounded release, verification
failure, privacy redaction, and production unavailability.

This gives the restart boundary a durable ledger-reload proof and gives
cross-process exclusion, takeover, prepared uncertainty, focus drift, and
geometry drift hermetic proofs. The installed live lane supplied the exact
provider, route, display, process, controller, input acknowledgement, replay,
after-state, and cleanup proof. No live runtime-host restart was used because it
would destroy the exact browser and controller binding needed to distinguish a
replay from a different canonical request.

The structural actions check passed. The aggregate
`pnpm test:actions-architecture` command remains red only on the independently
reproduced P0101 and P0108 architecture baselines, including interface-test
count drift, missing coordinator outcomes, atomic-store fault coverage, and
unclassified process-identity consumers. No Plan 0131 finding was added to
that baseline. Strict formatting, Clippy, focused Rust modules, service
contract and generated-client parity, dashboard receipt projection, route and
installer tests, WSL Cargo safety, documentation build, Python fixture
compilation, development doctor, three-cycle browser launch smoke, and diff
hygiene passed during the source and installed validation tiers.

The original candidate build took 17 minutes 5 seconds. The remediated
candidate build took 15 minutes 17 seconds. Cargo ran through the repository
WSL admission wrapper with four jobs and the configured cgroup limits. Live
operations were serialized on one primary route. No uncertain operation was
retried, and no broad process cleanup was performed.

## Cleanup And Remaining Gate

The exact viewer lease was released, the fixture process exited, and the exact
service-owned browser closed. Reconciliation returned
`development-route-1` to `warm_idle`; all four presentation slots are again
warm idle. A fresh OS process readback found no fixture process or
`p131-controlled-x11` Chrome tree. Service resources reported zero cleanup
candidates and retained only the pre-existing historical lifecycle obligation
recorded in the preflight.

Production was not installed, restarted, selected, or given provider input
authority. Plan 0110 remains open. The only next executable boundary is Plan
0131 Slice E after fresh explicit production authority and a new transactional
preflight.
