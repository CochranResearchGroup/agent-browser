# Plan 0190 | Advisory Candidate Build And Promotion Orchestrator

Date: 2026-09-14

Plan version: 4

State: OPEN

Lane: P190

Product lane: PL-PLATFORM

Planning branch: `platform/p190-advisory-candidate-orchestrator-plan`

Amendment branch: `platform/p190-dev-build-test-promotion-amendment`

Implementation branch: `platform/p190-advisory-candidate-orchestrator`

Target: `main`

Integration: merge

Work item: [issue #136](https://github.com/CochranResearchGroup/agent-browser/issues/136)

Source baseline: `d101eb1516a26a8ae40821c7da69bba77e43f78d`

Version 2 baseline: `4e047fb4b51f605094b3557eb0cb0de1a2643a1e`

Implementation baseline: `552f692502ec63032b0a936029993510f3299fb7`

Consolidation: required

## Objective

Build one deterministic advisory surface that explains current candidate,
build, install, and recovery state; recommends a next action; presents every
safe supported alternative; and executes the operator's selected transition
through the existing workstation transaction. Prevent concurrent corrupting
commits without turning the tool into a permission authority or forcing a
second equivalent production build after merge.

## Current State

The repository already has the durable mechanisms the orchestrator must use:
sealed workstation generations, binary and support-manifest digest validation,
resumable `UpgradeTransaction` states, runtime-replacement planning, final
doctor verification, development candidate publication, and Cargo admission.
The missing layer is a common candidate identity and advisory state machine
that lets independent sessions discover, join, queue, cancel, discard,
supersede, install, recover, or roll back without racing or rebuilding by
default.

The existing development publisher accepts an explicit binary, copies those
exact bytes into an immutable development generation, and supplies the
development environment through its launcher and units rather than a different
compiled executable. The production workstation installer stages its own
current executable and binds that binary digest into the upgrade transaction.
Therefore a release-profile binary can be tested in development and later
installed unchanged. The existing `ci` binary is not that artifact because its
Cargo profile differs from production.

The Plan 0186 installer collision showed that a process lock can serialize two
commands while still leaving their intent and candidate ownership ambiguous.
Policies 0051 and 0052 now treat coordination records as integrity mechanisms,
not agent-role permissions. Issue #136 tracks implementation. Implementation
is admitted from canonical `main` checkpoint `552f6925` on
`platform/p190-advisory-candidate-orchestrator`. The first packet inventories
the executable-input closure and develops the pure candidate manifest and
advice interface without a candidate build or any install, supervisor,
browser, profile, provider, or tenant effect. The incoming bugfix was integrated
through `origin/main` at `7b5a88dc`, then merged into this implementation branch
before the first shared read-only adapter. Costly build qualification and all
effect-capable adapters remain deferred.

## Implementation Progress

The first source-only checkpoint is `e10869f4`. It adds the pure
`agent-browser-candidate` crate, a durable executable-input inventory, stable
input and manifest digests, artifact-reuse equivalence independent of merge
provenance, typed advisory results, and revision and fencing-aware idempotent
transitions. Production-shaped manifests fail closed when dashboard output or
embedded support assets are absent.

The focused candidate tests, candidate architecture guard, existing extracted
crate architecture guards and tests, workspace format, and strict workspace
Clippy pass at this checkpoint. The ordinary Rust runner now exposes the
`candidate` compartment. No CLI adapter, candidate build, installed runtime,
browser, profile, provider, Service State, or production effect was introduced.
The next packet remains behind the announced bugfix integration readback
because it begins the shared build and workstation adapter surfaces.

The second pure-kernel checkpoint is `6e37707b`. Equivalent build identities
now join an active operation or reuse an exactly verified sealed artifact;
different identities receive deterministic disjoint output directories; failed
operations are not reused; and a changed sealed digest fails closed. Promotion
advice verifies the production artifact class, full release profile, clean and
integrated source provenance, current executable-input equivalence, binary and
support-manifest digests, exact scoped receipts, development doctor, and task
residue. It returns explicit rebuild reasons separately from integrity
precondition failures. Focused tests and strict workspace Clippy pass. This
checkpoint still performs no build, repository collection, runtime action, or
shared CLI adaptation.

The third pure-kernel checkpoint is `32f4f212`. It deterministically allocates
stable lane namespaces with disjoint install, home, runtime, socket, profile,
browser-state, output, provider, and port identities. Test-run identity binds
the candidate, binary, suite revision and selection, fixture, target, runtime
capability, environment, and resource class. Exact active runs join; only exact
hermetic successful receipts with proven terminal cleanup are reusable;
shared-resource conflicts wait; isolated provider-free work remains eligible
to overlap. Focused tests, the candidate architecture guard, and strict
workspace Clippy pass without creating any namespace or test process.

The fourth pure-kernel checkpoint is `d2f7f032`. The coordination ledger now
records idempotent request receipts, joins same-candidate requests, queues
competing candidates in FIFO order, and supports exact cancel, discard,
activate, supersede, and completion transitions. Active and queued artifacts
remain pinned until disposition. Ownership-changing transitions advance a
fencing generation; stale writers and exhausted counters fail without partial
mutation. Liveness renewal requires new durable phase or process evidence and
reports bounded recovery choices after a deadline. Lock traces enforce the
coordination, install, runtime, Service State acquisition order, reverse
release order, and a caller-selected maximum physical hold duration. Candidate
tests and strict workspace Clippy pass; the checkpoint remains pure and causes
no build or runtime effect.

Checkpoint `95c92430` freezes checked-in versioned fixtures for the candidate
manifest and competing-candidate advisory result. Deserialized input closures
must be canonical, and candidate manifests now validate their source,
candidate ID, content and artifact digests, build profile, features, embedded
assets, timestamp, and receipt locators before adapter use. The promotion
decision rejects internally inconsistent manifests as integrity failures while
preserving executable-input drift as a distinct rebuild reason. Fixture tests,
all focused candidate tests, the architecture guard, and strict workspace
Clippy pass without performing a candidate build.

Checkpoint `1aa11f1b` adds the first shared CLI adapter. `candidate status`
projects the existing read-only workstation transaction status into the common
advisory contract, while `candidate inspect` validates an explicitly supplied
manifest against its executable-input closure. Both commands dispatch before
daemon-backed commands and expose build provenance, alternatives, consequences,
integrity preconditions, reuse eligibility, rebuild reasons, and receipt
locators without building, installing, recovering, launching a browser, or
connecting to a daemon. Five focused CLI tests, all 29 candidate-kernel tests,
the architecture and documentation contract, version sync, workspace format,
strict workspace Clippy, the docs production build, and patch hygiene pass.

Checkpoint `ef8fbbe3` adds the durable coordination adapter without exposing an
effect command. Deserialized ledgers now validate their schema, environment,
revision and receipt sequence, request linkage, operation identity, and active
fencing generation before use. The CLI adapter holds a coordination-only lock
for read, compare, and commit; writes a private same-directory temporary file;
syncs it; atomically replaces the ledger on Unix and Windows; and releases the
lock before any future installer work. Exact request replay performs no rewrite.
Read-only `candidate status` projects an absent ledger without creating state.
Three atomic-store tests, five advisory CLI tests, all 30 candidate-kernel tests,
a broader 87-test provider-free candidate-name selection, workspace format,
strict workspace Clippy, the architecture and documentation contract, the docs
production build, and patch hygiene pass. The production coordination path was
confirmed absent after validation; no runtime or installer effect occurred.

Checkpoint `07aaf9e9` adds the provider-free executable-input collector. It
combines Cargo dep-info from the CLI and linked local crates with explicit
manifest, lockfile, build-script, package-version, toolchain, dashboard, and
embedded-asset roots; resolves every file to a repository-contained canonical
path; hashes content; and emits the pure crate's canonical closure schema. A
production-shaped collection rejects a missing or placeholder dashboard and a
closure with no embedded assets. Reviewed environment inputs accept only
SHA-256 digests, never raw values. Focused collector tests, the candidate
architecture guard, patch hygiene, the conservative release-asset fixture, and
the validation-selector self-check pass. The collector cannot spawn Cargo or
another process, and this checkpoint performed no candidate build or runtime
effect.

Checkpoint `641c2f17` separates immutable build support identity from the
workstation's install-specific deployment manifest. The collector now derives
the executable-input digest, embedded-dashboard digest, embedded-asset map,
reviewed-environment digest, resolved-profile digest, encoded build-support
manifest, and candidate manifest deterministically. Its fixture reproduces the
Rust kernel's known closure digest and the complete checked-in Rust candidate
manifest, preventing a parallel JavaScript identity contract. The focused
collector test, architecture guard, and patch hygiene pass. Canonical main was
also refreshed through `9a46d73b`, retaining P190 while accepting the completed
P196 lane removal and its source fixes. No build or runtime effect occurred.

Checkpoint `495fc8dc` adds the provider-free candidate build executor seam.
It derives deterministic fast-iteration and production-shaped build plans,
uses disjoint Cargo target directories, keeps dry runs strictly zero-effect,
rejects production builds from non-clean source, detects plan tampering before
adapter observation, reuses an exact sealed artifact, joins an active build,
and records acquired-build failure before propagating it. The complete effect
adapter is validated before claim acquisition, so missing seal or completion
support cannot leave a newly claimed partial build. Fast-iteration support
manifests may omit production-only dashboard and embedded assets, while
production-shaped manifests continue to fail closed without both. The focused
executor and collector tests, candidate architecture guard, release-asset
fixture, validation selector, and patch hygiene pass. This checkpoint did not
invoke Cargo, perform a candidate build, or mutate an installed runtime.

Checkpoint `008944c6` deepens the existing workstation installer instead of
adding a candidate-specific installer. Transaction preparation now accepts one
payload-source seam used by both the current executable and later sealed
candidate bytes. A reviewed source binds an expected binary digest before the
upgrade transaction is created, the staged copy is hashed again, and any
source change during staging fails before generation commit. The existing
`UpgradeTransaction`, install-specific support manifest, immutable generation,
runtime census, migration, and activation flow remain the sole production
path. Strict workspace Clippy, all 166 workstation-install Rust tests, the six
selected workstation fixture suites, the candidate architecture guard, format,
and patch hygiene pass. Only disposable fixture workstations were exercised;
neither installed runtime was mutated.

Checkpoint `962a927b` binds an exact sealed production candidate into that
existing transaction path. Before a transaction record is created, the pure
candidate kernel verifies the artifact seal, exact candidate-manifest byte
digest, executable-input closure, build identity, binary digest, and immutable
build-support-manifest digest. The workstation adapter additionally requires a
production-shaped artifact and persists the candidate ID, source identity,
build operation and seal, executable-input and manifest digests, and validation
receipt locators in additive `UpgradeTransaction` metadata. The immutable build
support digest remains explicitly distinct from the install-specific support
manifest generated by the workstation adapter. Strict workspace Clippy, all
163 selected workstation-install tests, all 30 candidate-kernel tests, the
candidate architecture guard, format, and patch hygiene pass. The seam is not
reachable from a public command, and no build or installed-runtime effect was
performed.

Checkpoint `3fbb7e13` exposes the first public promotion preflight without
opening an effect path. `candidate install --dry-run` accepts an explicit
binary, candidate manifest, executable-input closure, and sealed-artifact
document; it reuses the same workstation review seam, verifies exact binary
bytes, and returns the common advisory schema without creating a coordination
ledger, workstation transaction, or runtime state. `--apply` fails explicitly
until the effect transition adapter is complete. All seven candidate CLI tests,
strict workspace Clippy, format, the candidate architecture contract, rendered
candidate help, docs production build, remote-view documentation contract, and
patch hygiene pass. README, CLI help, repository skill, docs site, and inline
documentation describe the same temporary read-only contract. The shared
user-scoped skill was inspected but intentionally not overwritten from an
experimental branch.

Checkpoint `7166b082` adds exact install custody and the first fenced
workstation mutation boundary. Candidate and sealed-artifact identity are
compared, started or joined, and receipted from one coordination-locked
snapshot. Harmless joins may advance the ledger revision without changing the
operation or fencing generation. A bounded mutation runs only while the same
operation, candidate, artifact, environment, and fence remain active, and the
coordination lock prevents cancellation or supersession during that mutation.
The existing workstation transaction records this exact custody alongside the
sealed artifact binding, refreshes its observed revision, and now fences the
generation selector and Service State commit before either can change. A
superseded writer fails before commit and leaves the selector unchanged. Five
coordination-adapter tests, the focused workstation transaction test, strict
workspace Clippy, format, the candidate architecture guard, and patch hygiene
pass. Candidate `--apply` remains unavailable: activation, runtime transfer,
dashboard promotion, acceptance, coordination completion, and the public
cancel, discard, supersede, recovery, and rollback transitions still require
their bounded adapters. No installed runtime, browser, profile, provider, or
Service State was touched; all mutation evidence came from disposable fixture
roots.

Checkpoint `c223a1ef` extends that same custody fence through the bounded
publication of candidate readiness. After runtime transfer, the workstation
adapter now revalidates the exact operation, candidate, sealed artifact,
environment, and fencing generation while holding the coordination lock across
the `PresentationsRebinding` and `CandidateReady` transaction writes and the
matching admission-drain update. A superseded writer is rejected before any of
those files change, including on durable resume. The existing generation and
Service State commit uses the same reusable bounded-mutation guard. Five
coordination-adapter tests, both focused workstation custody regressions,
strict workspace Clippy, format, the candidate architecture guard, and patch
hygiene pass. The preceding exact head also passed every required pull-request
gate, including Rust and Workstation Fixtures. Candidate `--apply` remains
unavailable: admission-drain and runtime-transfer mutations, dashboard
promotion, acceptance, coordination completion, and the public transition and
recovery adapters still require bounded custody integration. No installed
runtime, browser, profile, provider, or Service State was touched; all mutation
evidence came from disposable fixture roots.

Checkpoint `81560f59` fences the bounded activation publication that precedes
runtime transfer. Optional legacy-runtime quiescence remains outside the
coordination lock because it may inspect and close processes; an exact custody
preflight runs before that effect. The subsequent `AdmissionDraining` and
`RuntimesTransferring` transitions and admission-drain writes now commit under
one exact custody lock. Post-transfer drain evidence moved into the already
fenced candidate-readiness publication. A superseded writer cannot publish the
transfer start, transaction revisions, or drain claim, while an active writer
publishes transaction and drain revisions from the same guarded mutation. Both
new activation regressions, the three existing pre-drain quiescence cases, the
candidate-readiness fence, durable resume to isolated acceptance, strict
workspace Clippy, format, the candidate architecture guard, and patch hygiene
pass. Runtime transfer itself still contains process operations and waits, so
it is intentionally not wrapped in a long-held coordination lock and remains a
later decomposition packet. No installed runtime, browser, profile, provider,
or Service State was touched; all mutation evidence came from disposable
fixture roots.

Checkpoint `5e56fe99` fences durable resume into runtime transfer and the
cooperative handoff evidence commit. An `AdmissionDraining` transaction now
reuses the same custody-locked transition as first activation. Before a
cooperative handoff begins, the adapter revalidates exact install custody; the
effect then accumulates migration and handoff results outside the durable
transaction. Candidate-host identity, owner-transfer receipts, ingress staging,
and the transaction rewrite commit together only after custody is revalidated
under the short coordination lock. A superseded writer leaves the persisted
transaction unchanged. The branch also merges current `origin/main` at
`c2ade1d1`, preserving P198's retained-owner repair and the P197 overlap while
retaining P190's newer checkpoint. The new resume and supersession regressions,
existing cooperative forward-only and durable-resume cases, all five
coordination-adapter tests, strict workspace Clippy, format, the candidate
architecture guard, and patch hygiene pass. Full-shutdown replacement still
persists checkpoints and performs process effects through its older unfenced
adapter, so it remains the next runtime-transfer packet. No installed runtime,
browser, profile, provider, or Service State was touched.

Checkpoint `f01ef5c3` adds candidate fencing inside the reviewed full-shutdown
replacement engine. The engine now asks its caller-supplied fence for an exact
custody preflight immediately before each cooperative close, forced browser
termination, and source-runtime retirement. Every effect-receipt checkpoint is
also delegated to that fence, so candidate custody is revalidated while the
short coordination lock covers only the transaction write. No coordination
lock remains held during process exit or census convergence waits. The legacy
unfenced live entry point was removed; the workstation adapter supplies the
candidate-aware fence, while transactions without candidate custody retain
their existing behavior. A lost-custody fixture proves no full-shutdown receipt
can be persisted, and the effect-order fixture proves loss before the first
process mutation stops the engine. All four runtime-replacement tests, the
full-shutdown forward-only and migration-refresh cases, strict workspace
Clippy, format, the candidate architecture guard, and patch hygiene pass.
Post-shutdown Service State restaging and candidate-host evidence publication
still need their own bounded custody seams. No live process or runtime was
mutated by these provider-free fixtures.

Checkpoint `c2f5a13c` closes the remaining post-shutdown transfer writes.
Service State restaging is now a prepare-then-commit operation: authoritative
state parsing and migration serialization happen without a coordination lock,
then exact custody is revalidated while the prepared snapshot, staged bytes,
transaction metadata, and drain revision commit. Candidate-host startup gets a
fresh preflight without retaining the lock across process startup or identity
capture. The candidate host identity, retired-migration dispositions, ingress
candidate, and transaction evidence then commit together under the bounded
fence. The shared supersession fixture proves that neither cooperative nor
full-shutdown host evidence can be attached by a stale writer; a separate
fixture proves post-shutdown restaging cannot begin after custody loss. The
existing post-shutdown migration-baseline case, strict workspace Clippy,
format, the candidate architecture guard, and patch hygiene pass. Runtime
transfer now has fenced activation, effect preflights, checkpoint persistence,
and final evidence commits without a physical lock spanning a process wait.
Dashboard promotion, acceptance, terminal coordination, and the public effect
and recovery commands remain. No installed runtime, browser, profile, provider,
or Service State was touched.

Checkpoint `1bd3994d` fences the dashboard shadow and managed-ingress
transitions and tightens the post-shutdown restaging lock duration. Candidate
custody is checked before the shadow dashboard process starts, and its observed
sealed backend is staged into ingress only under the short coordination lock.
Managed-backend readiness polling remains lock-free; the exact ingress
selection commits under custody, then the shadow process receives a fresh
preflight and stops outside the lock. A superseded candidate cannot start a
shadow backend or alter the ingress registry. Post-shutdown Service State
restaging now performs state reads, migration planning, and serialization
outside the lock, then revalidates custody for the prepared snapshot, staged
bytes, transaction, and drain commit. The two stale-dashboard regressions, the
existing exact already-selected promotion and forward-restage cases, the
post-shutdown refresh regressions, strict workspace Clippy, format, the
candidate architecture guard, and patch hygiene pass. Acceptance, terminal
coordination, and the public effect and recovery commands remain. No installed
runtime, browser, profile, provider, or Service State was touched.

Checkpoint `1b3a538e` fences the post-commit validation and acceptance
publications. The `GenerationCommitted` to `PostCommitValidating` transition
and its admission-drain refresh now commit under exact candidate custody.
Runtime handoff finalization and supervisor transition remain outside the
coordination lock because they may wait on processes; custody is checked
immediately before those effects and revalidated before acceptance evidence,
the `Accepted` transition, or drain removal is persisted. Supervisor failure
recovery transitions use the same bounded fence. Two stale-writer regressions
prove that a superseded candidate can neither begin post-commit validation nor
accept the transaction, and that the existing transaction and drain bytes stay
unchanged. The existing accepted-candidate validation, strict workspace
Clippy, format, candidate architecture guard, source-free workstation installer
fixture, and patch hygiene pass. Terminal coordination, public effect and
recovery commands, and production-shaped qualification remain. No installed
runtime, browser, profile, provider, or Service State was touched.

Checkpoint `c93acb5e` terminates candidate coordination with accepted
workstation state. The adapter holds exact install custody across the bounded
`Accepted` transaction write, commits `CompleteActive` and advances the fence
before releasing the coordination lock, then removes the admission drain.
Runtime handoff and supervisor process waits remain outside the lock. If a
crash lands after workstation acceptance but before coordination completion,
the accepted-transaction recovery path can finish the exact operation and
idempotently reuse its durable completion receipt before clearing the drain.
A competing supersede cannot enter during the terminal commit, and an already
completed operation is not rewritten on recovery replay. The seven focused
coordination adapter tests, terminal acceptance fixture, stale-writer
acceptance regression, existing accepted-candidate validation, source-free
workstation installer fixture, strict workspace Clippy, format, candidate
architecture guard, and patch hygiene pass. Public effect and recovery
commands and production-shaped qualification remain. No installed runtime,
browser, profile, provider, or Service State was touched.

Checkpoint `886cf660` exposes the sealed candidate effect and recovery seams
without introducing another installer. `candidate install` now requires
exactly one of `--dry-run` or `--apply`; apply revalidates the production-shaped
manifest, executable-input closure, artifact seal, and binary bytes, then
binds them and newly started coordination custody into the existing workstation
transaction. An identical concurrent request joins the active operation and
stops before creating a second transaction. `candidate recover
<resume|rollback|close>` forwards the exact transaction ID, revision,
generation, and census digest to the existing guarded transaction actions and
never selects the latest record implicitly. An isolated end-to-end fixture
proves the reviewed bytes reach `Accepted`, become the selected generation,
complete coordination, and leave no active operation. The candidate CLI tests,
sealed-binding and join regression, source-free workstation installer fixture,
strict workspace Clippy, format, debug CLI help readback, candidate architecture
guard, remote-view documentation contract, docs production build, and patch
hygiene pass. Public queue, cancel, discard, and supersede choice adapters and
production-shaped qualification remain. Neither installed runtime was
mutated.

Checkpoint `8ea89766` exposes the remaining operator-selected coordination
choices through one exact compare-and-swap command. `candidate coordinate`
supports queue, cancel-active, discard-queued, supersede, and activate-queued;
every request binds a unique request ID, candidate, artifact, expected ledger
revision, and expected fencing generation, while all actions except queue also
bind the exact operation ID. The response returns the durable receipt and
resulting ledger and explicitly reports that no runtime effect occurred.
Cancellation and supersession still advance the fence immediately, so a stale
installer cannot publish after the operator's choice. Ten focused candidate
CLI tests, a real disposable-root CLI queue receipt, strict workspace Clippy,
format, debug CLI help readback, candidate architecture guard, docs production
build, and patch hygiene pass. The disposable coordination root was moved to
trash after readback. Public build and test execution adapters and
production-shaped qualification remain. Neither installed runtime was
mutated.

Checkpoint `4043af74` exposes the isolated build execution adapter without
touching an installed runtime. `candidate build` requires an explicit source
checkout, artifact class, and exactly one of dry-run or apply. Dry-run returns
the deterministic command and output plan without creating a claim or invoking
Cargo. Apply uses an exclusive request claim below `cli/target`, disjoint target
directories, the existing Cargo safety wrapper, compiler dep-info, and the
frozen executable-input collector to publish an immutable binary, closure,
build-support manifest, candidate manifest, and Rust-compatible artifact seal.
An exact concurrent request joins rather than compiling twice; an exact sealed
result is reused only after every artifact byte and path is revalidated.
Source drift, cache tampering, and failed claims fail closed with durable typed
evidence. The adapter fsyncs claims and artifact publications and never reads
or writes production or development runtime state. Focused JavaScript command,
planner, collector, concurrency, reuse, tamper, and failure tests pass, as do
the complete candidate crate, the JavaScript-to-Rust seal compatibility test,
the CLI bridge tests, strict workspace Clippy, format, architecture guard,
rendered help, docs production build, remote-view documentation contract, and
patch hygiene. A real rebuilt CLI dry-run returned the current dirty source
identity and performed no build. Public test execution, explicit failed or
abandoned build recovery, and production-shaped qualification remain.

Checkpoint `45b93fee` makes failed build recovery an explicit exact-operation
transition. `--retry-failed-operation <id> --apply` accepts only the currently
failed claim for the same build request, takes a short exclusive recovery
guard, archives the failed claim and any partial sealed directory, then
publishes a replacement claim atomically. Active, different, or concurrently
recovering operations fail closed. The failed receipt and partial evidence are
preserved rather than deleted, and an interrupted recovery leaves its guard for
inspection instead of admitting an ambiguous writer. Focused retry, mismatch,
failure, and reuse fixtures, the candidate architecture guard, docs production
build, remote-view documentation contract, and patch hygiene pass. The branch
also integrated canonical `origin/main` through `3863106e`, including the
completed compatible-access-profile repair, before this checkpoint. Public
test execution and production-shaped qualification remain; neither runtime was
mutated.

Checkpoint `41db948c` exposes exact provider-free test coordination and
execution. `candidate test` verifies the candidate manifest, executable-input
closure, artifact seal, binary bytes, and exact source-checkout revision before
deriving a canonical test identity. Named `candidate-kernel`,
`candidate-build-adapter`, and `candidate-cli` selections bind deterministic
command, fixture, platform-capability, and environment digests. Dry-run creates
no state and reports join, reuse, or start advice. Apply first executes the
exact candidate binary's help surface, then runs only the named provider-free
suites with private HOME, XDG, temporary, Cargo, log, output, and receipt roots
below `cli/target`; Cargo admission retains its required host control socket.
An exact active run joins, a failed receipt is preserved without blocking a new
attempt, and only an exact successful hermetic receipt with terminal cleanup
proof is reused. Deserialized identities and records must remain canonical.
Four CLI adapter fixtures, all five development-coordination kernel tests, the
complete candidate crate, strict workspace Clippy, format, candidate
architecture guard, rebuilt CLI help, docs production build, remote-view
documentation contract, and patch hygiene pass. No browser, provider, profile,
or installed runtime was touched. Abandoned active build or test recovery and
production-shaped qualification remain.

Checkpoint `21bd6afd` corrects the candidate CLI suite commands to enter
through `scripts/ci/rust-tests.sh --focused`. The runner preserves the real
user-systemd runtime directory needed by the WSL Cargo admission wrapper while
continuing to give every CLI test process its own disposable HOME and XDG
trees. The four focused adapter fixtures, strict workspace Clippy, format,
candidate architecture guard, and patch hygiene pass after this correction.

Checkpoint `b7778835` closes abandoned build and test liveness recovery.
Build claims now record their owner PID. Exact `--recover-active-operation`
apply verifies that PID is no longer live, rechecks the unchanged claim under a
short exclusive recovery guard, archives the claim and partial artifacts, and
only then publishes a replacement operation. Candidate test active claims now
wrap the canonical run record with the owner PID; exact
`--recover-active-run` apply likewise refuses a live owner, archives the
abandoned claim, and admits a replacement run. Mismatched IDs, changed claims,
and concurrent recovery fail closed. Focused live-owner refusal, dead-owner
recovery, archive preservation, failure-retry, join, and reuse tests pass with
strict workspace Clippy, format, the candidate architecture guard, WSL Cargo
safety and capacity tests, docs production build, remote-view documentation
contract, and patch hygiene. The branch also integrated canonical main through
the P200 Cargo-scope descendant-lifetime repair. No installed runtime was
mutated. Production-shaped qualification is now the remaining planned gate.

Checkpoint `5f1bdc05` completes source qualification and the production-shaped
artifact gate without installing into either runtime. Real release-profile
builds exposed and preserved five bounded failures: linked-worktree Git inputs
outside the checkout, directory entries in Cargo dep-info, JavaScript locale
ordering that disagreed with Rust canonical ordering, an interactive Corepack
download caused by isolated `HOME`, and an sccache Unix-socket path longer than
the platform limit. The collector now hashes linked-worktree Git inputs under
stable `.source-control/` paths, recursively expands recorded directories,
uses cross-language lexical ordering, runs the five build-adapter fixtures
directly with Node, and binds an explicit no-sccache policy into isolated Rust
test identity. Each changed-input or verification failure retained its typed
claim or receipt before the next build.

The final clean source `5f1bdc059c80c6df346a8aa822fde9d54d07fad8`
produced production-shaped candidate
`candidate-1b69db93f039bddb-657fb949b6cbbf29`, executable-input digest
`1b69db93f039bddbef41fefecc4eed1f7b1f8cd74cf8cedd0d14f7d051e81c42`,
and binary SHA-256
`657fb949b6cbbf292ed14e5cc36d28362e6167f610119b7b90a3b5a87166d4ab`.
Exact self-inspection and install dry-run passed with no effect. The named
`candidate-kernel`, `candidate-build-adapter`, and `candidate-cli` suites then
passed as isolated provider-free run
`candidate-test-a28d8ea2-f12f-4202-81fc-9c709431caca`; its reusable hermetic
receipt is
`cli/target/candidate-test-state/completed/fae3a8375a580caa9962fc9a92bcaeb325662f7c4928aa3f3726583d2f09b25a.json`.
No browser, profile, provider, production runtime, or development runtime was
read or mutated. Development publication and acceptance remain a separately
admitted operational gate, followed by protected integration and post-merge
executable-input equivalence readback.

## Frozen Decisions

- The user retains authority to start, cancel, discard, install, supersede,
  recover, or roll back. An issue, plan, branch, lease, agent role, or tool
  recommendation does not grant or revoke that authority.
- The tool is advisory at the workflow boundary and mandatory only at the
  integrity boundary. It may fence a stale writer or reject an internally
  inconsistent commit, but it must explain the named invariant and supported
  recovery choices.
- There is no permanent coordinator agent. Any session acting within current
  user authority may inspect state and request a supported transition. The
  active operation owns temporary execution custody.
- Extend the existing workstation install and runtime-replacement transaction.
  Do not create a second installer, candidate store, or runtime state machine.
- Build once and reuse the sealed artifact after integration when its complete
  executable-input closure is unchanged. A merge commit identifier or docs-only
  change is not by itself a rebuild reason.
- Keep ordinary iteration on `pnpm build:development-candidate`. Use a full
  production candidate build only at qualification or when the input digest
  proves a rebuild necessary.
- Do not promote the current `ci`-profile development binary as a production
  build. It inherits release optimization but uses thin LTO and 16 codegen
  units, while the production profile uses full LTO and one codegen unit.
- Allow one production-shaped release artifact to be installed and tested in an
  isolated development namespace, then promote those exact sealed bytes after
  integration when every qualification condition remains true.

## Consolidated Batch

### Candidate identity kernel

Add a focused `agent-browser-candidate` Rust crate that owns pure manifest,
input-closure digest, advisory decision, and state-transition logic. Its
manifest records schema version, candidate ID, source commit and tree,
executable-input digest, target, toolchain, Cargo profile, features, reviewed
environment-input digest, embedded dashboard and asset digests, binary digest,
support-manifest digest, creation time, artifact class, resolved build-profile
configuration digest, and validation receipt locators. It stores no
credentials, browser data, tenant payloads, or raw environment secrets.

The first implementation packet must inventory the actual build dependency
closure before freezing the digest contract. At minimum it evaluates Rust
workspace sources and manifests, `Cargo.lock`, build scripts, embedded assets,
package-version inputs, target, toolchain, profile, features, and an explicit
allowlist of build-affecting environment values. Documentation and merge
metadata stay outside the closure unless the build demonstrably embeds them.
The existing `cli/build.rs` embeds the exact source revision and clean or dirty
tree state. Preserve those values as provenance while comparing the separately
defined executable-input closure. Verify the embedded dashboard is complete and
bind its digest; a placeholder or stale dashboard makes a candidate
non-promotable.

Deduplicate an active build by that input digest plus target, toolchain,
profile, features, and reviewed environment digest. An equivalent request joins
or observes the same build and receives its sealed artifact. Distinct admitted
builds use isolated outputs and the existing Cargo resource-admission wrapper;
they do not compete for one mutable target or hold the runtime install record.

### Development build and test coordination

Support two explicit artifact classes:

- `fast_iteration`: the existing `ci` profile for economical compile and
  provider-free development loops. Repeat it whenever the executable-input
  digest changes; equivalent requests join the active build or reuse its
  artifact. This class is never directly production-promotable.
- `production_shaped`: the full production `release` profile with complete
  embedded dashboard and assets, fixed target, toolchain, features, reviewed
  environment inputs, and a clean source commit. Build this class once near the
  qualification boundary and publish the exact binary into an isolated
  development runtime for acceptance.

Each lane receives a stable development namespace with disjoint install root,
pseudo-home, runtime directory, sockets, ports, profiles, browser state, and
optional provider resources. Namespace allocation is deterministic and
observable. Tests must not borrow production identity or another lane's
resources.

Identify each test run by candidate digest, test-suite revision and exact
selection, fixture digest, target platform, runtime capability manifest, and
relevant environment-input digest. Equivalent requests join an active run.
They may reuse a completed receipt only when the test declares itself hermetic,
the complete identity still matches, and the receipt proves terminal cleanup.
A changed identity starts a new run. Never reuse failed, partial, cancelled,
quarantined, or provider-backed evidence outside its exact scope.

Provider-free tests may overlap when output directories, homes, runtime trees,
ports, profiles, and process groups are disjoint. Serialize only tests that
genuinely share Chrome, a desktop, a provider, a production-like supervisor, or
another workstation resource. Cancellation and timeout preserve the receipt
and trigger exact task-owned residue inspection; client exit is not cleanup
proof.

### Development-to-production promotion

Promotion is an evidence-backed reclassification of an immutable
`production_shaped` artifact, not recompilation and not copying a development
runtime into production. The same binary digest becomes eligible for the
existing workstation transaction only when all of these are true:

- the manifest says `release`, full LTO, one codegen unit, expected target,
  toolchain, features, and reviewed environment inputs;
- the embedded dashboard and every required support asset are complete and
  digest-bound;
- the build source tree was clean and the exact source commit is now an
  ancestor of current canonical `origin/main` through the merged pull request;
- current `origin/main` has an equivalent executable-input closure, with any
  difference limited to excluded documentation or merge provenance;
- all required provider-free, development-runtime, source-free workstation,
  and selected acceptance receipts bind the exact binary digest and remain in
  scope;
- development-runtime doctor and exact task-owned residue checks pass; and
- production preflight validates the sealed binary and support-manifest digests
  before any runtime mutation.

If any condition is false or unprovable, report `rebuild_required` with the
exact differing input or missing receipt. The ordinary `fast_iteration`
artifact can inform qualification but cannot be relabeled or installed as the
production candidate.

### Existing transaction adapter

Keep operating-system and runtime effects in the current CLI and native
workstation modules. The adapter reuses `StagedWorkstationGeneration`,
`validate_install_transaction_candidate()`, `UpgradeTransaction`,
`resume_install_transaction()`, `plan_runtime_replacement()`, and
`verify_final_doctors()`. The pure crate proposes transitions; the adapter
performs them with the current install, supervisor, Service State, and doctor
contracts.

Every mutating phase submits the exact operation ID, revision, and fencing
generation. At most one install or recovery transaction may commit for a
target environment. A same-candidate request joins, observes, or resumes the
existing transaction. A competing candidate reports the active operation and
offers queue, wait, cancel, discard, or transactional supersede when supported.

### Advisory command and result surface

Plan a public command family equivalent to:

- `agent-browser candidate inspect|list|status`
- `agent-browser candidate build`
- `agent-browser candidate cancel|discard`
- `agent-browser candidate install|supersede|rollback`

Effectful transitions require an explicit `--apply`; read-only inspection is
the default. Final command names must follow the repository's documentation
parity contract before implementation merges.

Machine-readable output includes `observedState`, `recommendation`,
`alternatives`, `consequences`, `integrityPreconditions`, active operation and
candidate identities, artifact reuse eligibility, rebuild reasons, and receipt
locators. Use typed results including `already_applied`, `joined_existing`,
`queued`, `rebuild_required`, `superseded`, `recovery_required`, and
`integrity_precondition_failed`. Reserve `authorization_required` for genuine
absence of user authority. Do not emit generic `permission_denied` for normal
contention, drift, or a non-recommended operator choice.

### Jam resistance and recovery

- Separate long-lived logical transactions from short physical locks. Never
  hold repository, installer, supervisor, or Service State locks during Cargo,
  CI, upload, network waits, browser convergence, or large serialization.
- Publish and test one lock order: coordination transaction, install
  transaction, supervisor or runtime mutation, then Service State commit.
- Queued requests hold no committing lease or file lock. Revalidate their
  candidate, authority context, dependencies, and runtime state when selected.
- Require durable progress evidence for renewal. Bound stalled operations and
  expose exact last phase, timestamp, process evidence, and recovery choices.
- Fence stale writers after cancellation, supersede, or recovery. A former
  process cannot commit against a later generation even if it resumes.
- Pin active and queued artifacts against garbage collection. Release the pin
  only after install, discard, supersede, rollback disposition, or terminal
  failure is durably recorded.
- Make request and transition identifiers idempotent. Duplicate requests return
  the current or terminal receipt instead of starting another operation.
- Append request, recommendation, selected action, transition, and final
  readback receipts without secrets or tenant data.

## Non-Goals

- No permanent coordinator service, agent permission registry, generalized
  policy enforcement engine, or replacement for current user and goal
  authority.
- No second workstation installer, supervisor, Service State owner, artifact
  store, or independent recovery graph.
- No automatic production install, release, browser cleanup, profile reset,
  tenant action, provider mutation, or retry of an unrelated failed workflow.
- No absorption of the P181 lease-authority kernel or its branch. Reuse shared
  concepts only through an explicit dependency review after current custody is
  reconciled.
- No broad CI replay during early packets and no rebuild merely to obtain a
  different commit identifier.

## Delivery Sequence And Budget

1. Inventory executable inputs and current candidate, transaction, supervisor,
   and doctor seams. Freeze manifest and advisory-result fixtures without a
   build.
2. Implement the pure candidate crate with digest, equivalence, idempotency,
   advice, and state-transition tests.
3. Add explicit fast-iteration and production-shaped artifact manifests,
   same-input build joining, isolated outputs, and tamper detection.
4. Add development namespace and test-run identities, receipt reuse, exact
   cleanup, and shared-resource serialization.
5. Add read-only CLI inspection and status adapters, then align output, README,
   repository skill, docs site, and inline documentation.
6. Add explicit effect transitions over the existing workstation transaction,
   with compare-and-swap, fencing, join, queue, cancellation, supersede,
   recovery, and rollback receipts.
7. Build one production-shaped artifact, install it into the isolated
   development runtime, run selected acceptance, and prove post-merge
   executable-input equivalence without rebuilding.
8. Perform any production install only as a separately user-directed operation
   after source integration and final state readback.

Budget the implementation as seven source packets plus the separate operational
gate. Permit fast-iteration builds whenever their executable-input digest
changes; equivalent requests must join or reuse instead of compiling again.
Permit one production-shaped build. A second production-shaped build requires a
recorded changed-input, missing-artifact, or verification-failure reason. Use
focused provider-free tests per packet, reuse exact hermetic receipts, run one
final selected validation set, and allow at most one bounded repair cycle
before revising the plan. Do not start broad CI merely to observe progress.

## Worker Assignments

This planning turn assigns no implementation worker and creates no worktree.
When admitted, one PL-PLATFORM lane owner holds the implementation branch and
integrates the complete batch. Bounded review or test specialists may work in
disposable directories or the lane checkout; they do not receive additional
durable primary worktrees. Shared CLI, workstation, policy, roadmap, and
generated-client surfaces have one active writer per transition.

## Test Plan

Provider-free and isolated-development fixtures must cover:

- identical and changed executable-input closures across a merge;
- docs-only and merge-metadata changes that preserve artifact reuse;
- missing, malformed, stale, or tampered candidate manifests and artifacts;
- duplicate same-candidate requests joining one operation;
- duplicate same-input build requests producing one sealed artifact;
- distinct admitted builds using isolated outputs and Cargo resource claims;
- fast-iteration artifacts failing production-promotion eligibility;
- a production-shaped artifact retaining one digest through isolated
  development publication and production preflight;
- clean source-commit ancestry plus equivalent post-merge executable inputs;
- changed build profile, dashboard, asset, toolchain, feature, target, or
  reviewed environment input requiring a rebuild;
- duplicate test requests joining one run or reusing one exact hermetic receipt;
- changed suite, fixture, capability, target, or environment identity starting
  a new run;
- parallel isolated provider-free runs and serialized shared-resource runs;
- cancelled and timed-out tests preserving receipts and proving exact residue;
- different-candidate wait, queue, cancel, discard, and supersede choices;
- crash or timeout before effect, between commits, after runtime replacement,
  and before final doctor acceptance;
- a stale writer attempting to commit after a fencing-generation change;
- queue revalidation, bounded progress, fairness, and idempotent replay;
- active and queued artifact retention plus terminal garbage collection;
- lock-order and maximum physical-lock-hold assertions;
- exact transaction resume, rollback, supervisor, listener, generation, and
  final doctor readback through existing workstation fixtures; and
- help, README, repository skill, docs site, service schema, and generated
  client parity for every public surface actually introduced.

Use `pnpm validation:select -- --base <batch-baseline>` after each coherent
batch. Any Rust source change requires repository format and strict workspace
Clippy at batch completion. Run focused crate and workstation fixtures during
implementation. Run the comprehensive provider-free Rust lane once at the
final source gate only if selected by policy or explicitly required for merge.
Provider-backed or production acceptance remains a separate authorization and
evidence boundary.

## Evidence And Exit

Plan 0190 is implementation-complete only when:

- the pure candidate identity and advice crate has complete focused tests;
- the CLI uses the existing workstation transaction and does not introduce a
  competing installer, state store, or supervisor owner;
- two simultaneous same-candidate requests converge on one operation;
- competing candidates produce safe supported choices and cannot both commit;
- cancellation, supersede, timeout, crash, resume, recovery, and rollback each
  preserve fencing and return durable typed receipts;
- a sealed artifact is reused after merge when executable inputs are equivalent
  and rebuilt only with an exact recorded reason when they are not;
- ordinary `ci` artifacts cannot pass production promotion, while one
  production-shaped artifact can retain the same digest through isolated
  development acceptance and workstation preflight;
- equivalent development builds and hermetic tests join or reuse prior work,
  while changed inputs produce isolated new work;
- concurrent development lanes retain disjoint runtime, port, profile, browser,
  provider, process, and output identities with exact residue readback;
- active and queued artifacts survive generation cleanup until disposition;
- all introduced user-facing contracts are documented in every required
  surface and selected provider-free gates pass; and
- source integration is recorded separately from any installed-runtime
  acceptance. A successful plan does not itself authorize production effect.

Plan 0190 may move from `PLANNED` to active only after the current worktree
population is reconciled, issue #136 is assigned to the implementation lane,
the branch and checkout are admitted under policy 0052, and the baseline is
refreshed from canonical `origin/main`. Closeout records the integrated commit,
artifact manifest and digest, selected tests, reuse or rebuild decision, and
remaining operational gate. A production install, if requested, records its
own transaction, fencing generation, doctor evidence, and runtime receipt.
