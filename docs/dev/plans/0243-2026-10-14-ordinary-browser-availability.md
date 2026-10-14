# P243: ordinary browser availability

Product lane: PL-BUGFIX
Work items: #284, #286
State: COMPLETED
Branch: fix/ordinary-browser-availability

## Outcome

Restore the existing Alice/Bob G05/G23/G42 contract: valid ordinary browser work
uses current browser observations, never historical ownership as permission.
Downloads deliver the correct task browser's file without owner-binding gates.
Interrupted launch publication reconciles the live observed browser and resumes
its logical task without a replacement launch or lost profile state.

## Scope and overlap

Primary writer for this slice: P243. Own download handlers, focused regressions,
and bounded observed-launch reconciliation. P237 owns its existing dirty native
UI/lifecycle work in a separate checkout; preserve it. Integration must reconcile
shared launch-store/remote-view adapters by behavior, retaining P237 changes.
P242 branch/PR285 and P240 checkout/candidate remain preserved. No production,
provider, real-account, authentication or broad cleanup effects are authorized.
No historical plan execution is resumed.

## Validation

Use the installed #284 public MCP reproduction and turn it into an asserting
regression. Reproduce #286's interrupted-publication state in a deterministic
fixture before fixing it. Cover live continuation, repeated/concurrent publication,
wrong process identity, dead observed process and unrelated stale claims. Keep
exact file/process identity checks where needed to deliver the right artifact.
Run focused suites, Rust format and strict workspace Clippy, required changed
surface checks and isolated browser red/green with fresh owned-process census.

## Custody

Baseline origin/main cb765a7be. Admission inventoried three existing checkouts:
clean P242 primary, dirty P237 dashboard, and clean retained P240 enrollment.
P232 catalog custody describes a preserved candidate, not a fourth checkout.
This user-directed repair adds one primary checkout; no foreign bytes were edited.
Retain this checkout until validated PR integration or explicit handoff.

## Reproduction checkpoint

Installed generation 0.28.0-53f9c81eee8b-93bb61bc8815 refuses existing-profile
download with current owner binding unavailable. The asserting public fixture
fails on that exact error and confirms zero owned residue. The ordinary native
host/SQLite interrupted-observed-publication regression fails on baseline with
browser_session_publication_release_fenced and passes in the first repaired
58-test consumer run. The expanded 60-test run also passes dead observed launch
and mismatched-history cases. A second public MCP red injects historical custody
only into its disposable fixture database; concurrent continuation fails with
the same publication refusal while its real browser remains live. Its final
fresh owned-process census is zero. Original failures remain in local logs.

The docs build and changed-link check pass. The old baseline policy-wiring test
fails on its retired policy alias; the incoming canonical P242 merge contains
that independent repair. Integrate current canonical source before final checks.


## Completed bounded outcome

Integrated-source selected gates and all three public synthetic workflows pass.
The final extended absence regression, six store tests, eleven runtime tests
(six ignored), 39 output tests and all 330 Service Model tests pass. Format,
strict workspace Clippy, docs, links, policy wiring and handoff guidance pass.
The Service Model linker EAGAIN and successful four-job infrastructure retry
remain recorded. Public download, concurrent same-process continuation with
stale custody and Alice/Bob/borrowed-profile checks end with zero owned residue.

[The curated receipt](../notes/0243-2026-10-14-ordinary-browser-availability.md)
binds source, executable digest, red/green evidence and review limits. PR287
owns canonical integration. Source qualification is complete; no production
installation or real-account/native-provider acceptance is claimed. The
original manual service-recovery build/presentation symptoms remain unverified.
