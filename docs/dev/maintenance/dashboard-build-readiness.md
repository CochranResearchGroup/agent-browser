# Dashboard build readiness

Product lane: PL-BUGFIX
Work item: [#587](https://github.com/CochranResearchGroup/agent-browser/issues/587)
Status: SOURCE_QUALIFIED

## Confirmed cause

The original build record identifies source `8db2baf3` and the maintained
`pnpm build:development-candidate` command. That command performed version sync
and Cargo compilation without building the frontend. Rust's source-development
fallback became the installed dashboard: one asset totaling 118 bytes. The build
passed in 561.744 seconds without compiler warnings; the successful build embedded incomplete frontend output.

The candidate backend check verified the runtime-manifest schema and matching
digest without checking dashboard content. Doctor likewise accepted the
installation. Authentication is unnecessary to reproduce the failure: an
isolated invocation of the retained incident binary serves the fallback over
HTTP, and its isolated public install dry-run succeeds.

## Repair

The maintained candidate command builds the frontend before Cargo and stops on
frontend failure. The manifest publishes `dashboard.built` from the actual index
and script inventory; a fallback beside stale scripts remains incomplete.
Candidate backend qualification and doctor reject incomplete assets. Ordinary
installation rejects them before creating a payload or installation transaction.
Complete older manifests retain their existing asset-metadata compatibility.
Rust-only source builds may still compile with the fallback.

## Validation

- Fresh-checkout public entrypoint regression: RED on the fallback, GREEN after
  repair; a frontend failure prevents Cargo even when earlier output exists.
- Real HTTP manifest-probe regression: RED despite matching incident identity,
  GREEN after repair. Final `scripts/ci/rust-tests.sh --focused dashboard_`:
  156 tests pass, plus the runner isolation sentinel.
- `scripts/ci/rust-tests.sh --focused install_doctor_`: 20 tests pass.
- Required workspace Rustfmt and strict Clippy with `-D warnings`: pass. The
  first Clippy attempt found two boolean-style errors; both were corrected.
- Exact-source CLI build and `AGENT_BROWSER_FIXTURE_BIN`-bound
  `pnpm test:workstation-install-fixture`: pass. Both fixture-owned hosts were
  positively absent afterward.
- Public CLI probe with a controlled fallback beside stale assets: installation
  rejects it without payload or transaction state, and doctor reports
  `dashboard_build_incomplete`. Normal error startup creates a service lock;
  this is not a claim of zero filesystem writes. Restoring the index reproduces
  the healthy HTTP shell, with identical before/after frontend bytes.
- Development runtime, repository tooling, validation selection, policy wiring,
  documentation links, handoff documentation and the 44-page docs build: pass.

The optional broad `install::tests::` run was stopped during repeated copying
and hashing of its 857 MB unoptimized harness, without a failed assertion. Its
partial result is not a passing suite. The interrupted synthetic transaction
was preserved privately and its exact owned fixture removed. Earlier failed
manifest fixtures were corrected to publish real asset metadata; their original
failed receipts remain retained. No test was removed or ignored to pass a gate.

Local validation ran on WSL/Linux, using an isolated Cargo target with compiler
cache disabled. Final compilation used four jobs, MemoryHigh 6 GiB and MemoryMax 8 GiB
with an 8 GiB claim, leaving the configured host reserve and aggregate limits
unchanged. The public fixture used the same debug CLI with debug symbols removed.
These are provider-free source and local-artifact checks, not target-platform
CI or a production deployment. Hosted GitHub CI remains disabled.

## Custody and remaining scope

The source base is canonical `a779f3fe9`; this PR binds the qualified changes.
The existing recovered-invitation checkout is reused without increasing the
eight-checkout population. Its previous branch and sealed production candidate
remain retained, with review due 2026-11-09. PR 588 independently owns idle-handoff
source and its guide paragraphs; 587 owns dashboard readiness and the build
paragraphs. Shared documentation must preserve both at integration.

The earlier dashboard restoration remains separate. This packet performs no
production installation, browser input, authentication mapping change or repair
of the earlier rail/access/footer findings. Broader lifecycle tests and actual
installed acceptance of the new guard are not claimed.
