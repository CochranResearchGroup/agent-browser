# Handoff maintenance latency

Product lane: PL-BUGFIX. Work item: [677](https://github.com/CochranResearchGroup/agent-browser/issues/677).
Owner: ecochran76. Branch: `fix/handoff-maintenance-latency`.

## Outcome

Make an existing authenticated handoff responsive while background maintenance
retains every lifecycle, identity, publication and peer-work fence. Source repair
and local validation are separate from production installation and original-link
acceptance. Remote View remains the viewer and authentication owner.

## Evidence and feedback loop

Bounded private runtime evidence showed continuation requests above 60 seconds,
a 98.061-second direct owner callback and a 31.319-second read-only inspection.
A correlated inspection returned 0.755 seconds after an 89.212-second maintenance
pass completed. The shared owner mutex encloses that maintenance pass.
Private invitations, account state, runtime payloads and traces stay outside Git.

The synthetic loop uses the actual retirement adapter and SQLite store with
known and legacy peer groups. Before the repair it failed twice at the intended
assertion: five aggregate reads instead of at most two, in 7.33 and 8.53 seconds.
Compilation and helper setup failures are separate from those RED results.
The helper retains legacy-identity deferral, no native preparation and unchanged
session-state assertions. The repair must then pass this loop and existing
custody tests without a provider or browser.

## Bounds

Investigate repeated state/custody reads and native or filesystem work under
maintenance. Do not weaken validation, cache mutable authority across operations,
drop history, change retention, or move effects to a second owner. Record the
responsible substep before selecting a repair. 676 owns managed launch defaults
in the peer checkout; 673 owns Chrome onboarding. Avoid their implementation
surfaces and reconcile shared catalog prose during integration.

## Validation and completion

Run focused regression and affected custody/model checks, Rust formatting and
strict workspace Clippy. Use local validation only. Preserve the original public
latency evidence and do not claim first-pixel or deployed acceptance from source
tests. Re-run the original public link only against a separately qualified
candidate; unchanged production is expected to retain the defect.

## Source repair and local evidence

The SQLite adapter now reads selected assignments in one transaction and joins
peer assignments with immutable process identities in a second transaction.
Each transaction loads and validates the current owner aggregate and launch
ledger once. Transactions end before provider reads or effects. Legacy identity
and release fences remain required; final admission and publication are unchanged.

Baseline source: `296c0f14c6ff72ff5a1045d2b346ad73e8b8d303`.
Focused `scripts/ci/rust-tests.sh --focused retirement_preflight677` passed both
regressions in 10.92 seconds after compilation. The affected pool selection
completed with 140 passed, two failed and one ignored in 952.32 seconds. The two
failures are `retained_interactive_handoff_recovers_with_original_invitation` and
`retained_interactive_recovery_rejects_unqualified_scope_and_control`.
Restoring both changed implementation files to the baseline reproduced those
same failures in 10.03 seconds with `--focused retained_interactive`. The new
batch APIs remain unused on that baseline call path. This comparison separates
the pre-existing recovery defect, tracked under [599](https://github.com/CochranResearchGroup/agent-browser/issues/599),
from the passing latency regressions; it does not make the pool selection green.

Policy wiring (11 tests), changed documentation links and patch hygiene pass.
All 235 service-model tests, strict workspace Clippy and final formatting pass. Validation runs
locally on WSL with disposable fixture homes, Rust 1.94.1, four build jobs,
private target storage, debug symbols disabled and eight codegen units. No
hosted CI, provider/browser acceptance, production install or first-pixel
qualification is included. Private raw logs and comparison artifacts remain
outside the repository.


Commands used for the final source checks:

```bash
scripts/ci/rust-tests.sh --focused retirement_preflight677
scripts/ci/rust-tests.sh --focused pool_tests
scripts/ci/cargo-safe.sh test -p agent-browser-service-model --lib -- --test-threads=1
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
pnpm test:policy-wiring
pnpm test:documentation-links -- --base origin/main
git diff --check
```

The model/lint lane used a 4 GiB memory cap; CLI test compilation used a 7 GiB
cap. A waiting larger reservation was cancelled before compilation. Prior
resource, helper-compilation and intentionally RED receipts are retained
separately. The final source passed focused tests and lint without a retry for a
product failure. The broader pool result remains failed, with baseline comparison
as described above. Keep this repair in draft until that integration risk is
resolved; source qualification does not satisfy original-link acceptance.

A final fetch observed `origin/main` at
`ea9bcb186b41209b2887a871aaf38d7ef6f3115f`, including peer work 673 and 676.
This draft retains the tested baseline above; integration with the new tip and
its overlapping domain/catalog prose remains a separate gate before merge.


## Authorized review, merge and deployment

The operator requested review, merge and deploy on 2026-11-13. Integrated main
at df24da179. Serial Standards review found no blocking convention or ownership
issue. Spec review retained operation-local snapshot authority and unchanged
admission/publication fences; live timing remains an acceptance gate.

The two baseline recovery failures reproduced after integration. The recovery
proof compared the retired and replacement desktops for equality even though
current recovery claims a fresh desktop. Align only that field for the logical
browser comparison, then require both immutable launch records to confirm their
own exact assignments, the completed retirement and original absence. Existing
positive and ten adverse recovery cases changed from RED to GREEN; add a foreign
assignment case to make that publication guard explicit. This bounded correction
resolves the validation dependency with issue599 without changing recovery owners.
The initial production build was stopped at its exact task-owned Cargo scope
before source correction; its failed receipt and partial cache remain retained.
