# Plan 0142 Final Acceptance

Date: 2026-08-29

Plan: 0142

State: CLOSED

## Integrated source and runtime

- Product source commit: `62ffac191d462e68a45f8420ba00af2307c2c272`
- Structured lock-diagnostic checkpoint: `49bc3d61`
- Development generation: `0.28.0-4ad310a1b16c`
- Browser executable: `/opt/google/chrome/chrome`
- Production unchanged: true
- Development doctor: ready with every reported check passing
- Development skill: current

The closeout-only test-isolation and documentation commit follows the product
source commit. It changes only `cfg(test)` control-plane setup and inline or
plan documentation, so the accepted release binary remains the exact product
behavior under review.

## Service State contention acceptance

- The realistic fixture was 3,146,455 bytes with 150 retained jobs spanning
  launch, remote-view open, tab creation, and viewport actions.
- Six concurrent readers and two writers completed in 490 ms with zero lock
  timeouts, zero duplicate effects, and two committed revisions.
- Fifteen recorded lock holds had p95 117 ms and maximum 117 ms. The existing
  one-second lock budget was not raised.
- A separate Rust process held the cross-process file lock; the contender
  timed out before mutation and created no Service State file.
- Process and file timeout tests prove the contender does not remain as a false
  active holder. Success, early return, and unwind tests prove guard cleanup.
- Process-lock poisoning is recovered once, recorded, and cleared because the
  mutex protects admission rather than shared mutable state.
- The bounded process-local status projection retains at most 32 recent
  activities and never persists paths, payloads, URLs, capabilities, or tenant
  content.

## Client and lifecycle outcomes

- Jobs retain the legacy error string and add the versioned structured failure
  object. Trace aggregation includes the same job record.
- CLI, HTTP, MCP, generated TypeScript clients, dashboard selected-job
  rendering, help, README, docs site, repository skill, installed Agent Browser
  skill, and the shared service-client skill use the same recourse semantics.
- A viewport lock failure remains `effect_uncertain` and
  `inspect_before_retry`; it never grants blind retry.
- Lifecycle owner errors refresh the access plan without inventing reuse.
  Reuse requires exact current `browserId` and `sessionName` hints.
- Recoverable terminal owners use one sealed, revision-bound Recovery Plan.
  Foreign, live, ambiguous, or unsupported authority remains a typed hard
  blocker with a duplicate-lane stop.
- Exact-commit suites passed: 8 service-failure tests, 18 profile-recovery
  tests, and 17 runtime-lifecycle tests.

## Persistence and adapter proof

- Stable snapshot reads use the shared file lock without the process mutation
  mutex. Status projection remains read-only.
- Mutation preparation occurs before exclusive commit. File lock precedes the
  process mutex, and the process mutex is released before durable persistence.
- Monotonic `stateRevision` compare-and-swap rejects stale prepared work before
  effect.
- All four-file interruption and rollback boundaries, mixed-version
  preservation, and transaction recovery tests passed.
- Twenty-eight Service Store tests, 15 status-projection tests, the full
  generated service-client suite, API and MCP parity, cross-seam interlocks,
  no-launch status smoke, dashboard checks, strict Clippy, Rust formatting,
  docs build, and selector-recommended workstation fixtures passed.
- A broad parallel Rust sweep passed 2,641 tests and exposed 16 documented
  environment-mutating partition collisions. Every failed test passed in its
  required serial partition. The control-plane test helper was then corrected
  to reset the actual test-only Service State root, and its canonical 40-test
  serial partition passed completely.

## Development acceptance and census

- Three disposable browser launch, URL-read, close, and residue iterations
  passed on generation `0.28.0-4ad310a1b16c`.
- Live status projected schema
  `agent-browser.service-state-lock-diagnostics.v1`, recent capacity 32, no
  active holder, zero process or file timeout, and recent 4 to 19 ms holds.
- The fresh census found the three expected current development services and
  no smoke Chrome residue. Service resources reported zero cleanup candidates,
  zero correlated processes, zero unknown cleanup obligations, and no warnings.
- Seven development processes from generation `0.28.0-06a24ebb6035` predate
  this run and remain protected retained lanes. They explain the advisory
  executable-generation multiplicity in Service Status. They are not new
  residue or cleanup candidates and were left untouched.

## Boundary and successor state

No production service, Service State, browser, profile, provider, route,
process, or tenant was changed. Plan 0137 now records P142 as satisfied, but
its Slice J production gate remains blocked on the separately recorded state
compatibility and presentation prerequisites. Plan 0142 completion proves
source and isolated-development readiness only.
