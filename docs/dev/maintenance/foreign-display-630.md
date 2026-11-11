# Foreign display cleanup classification

Product lane: PL-BUGFIX. Work item: #630. Branch: `fix/foreign-display-630`.

Restore the promised ownership boundary: an Xvfb process absent from Agent
Browser state is foreign or unknown, rather than a proven cleanup candidate.
The concrete workflow is an ordinary preserve-mode upgrade while another
application owns a display. It must not require interrupting that application.

Bound this packet to the resource classifier, its real inventory and GC test
seams, existing cleanup guidance, and one source review. Use at most three repair
attempts within 45 minutes; retain failures and replan on a material new failure.
No production install, process cleanup, foreign-service interruption or broader
installer redesign belongs to this source packet.

The main checkout is owned by the standalone handoff lane. A separate primary
checkout binds this user-directed repair; compact596 remains in open PR626,
worker-status retains its documented process cwd references, and five known
missing registrations remain untouched. This lane writes only its catalog
entry, this receipt and the resource-classifier surfaces. 630 owns only resource-cleanup paragraphs in README, CLI help, skill and
service-mode docs. The handoff lane owns its separate handoff paragraphs and
Remote View page; preserve those hunks.

The read-only installed reproduction is deterministic: positively verify the
foreign display owner’s systemd cgroup, then assert its PID has no cleanup action in actual
service resources. Two runs failed in 0.850 and 0.797 seconds, reporting
candidate, terminate_process and orphaned_remote_display_process. No effects
occur in that loop. Private provenance stays outside Git.

The minimized regression uses the actual production inventory projection with
an empty Service State and one Xvfb sample. It asserts zero candidates and no
cleanup identity or action. Removing that sample removes the failure; cgroup,
age and systemd are unnecessary to trigger the ownership error.

## Diagnosis and repair

The minimized test is RED against the unmodified classifier: candidateCount is
one instead of zero. Running its frozen test executable reproduces the failure
in 1.23 seconds; the development equivalent passes in 0.17 seconds and the old
production test explicitly expects unsafe candidate classification. The ranked
predictions were absence-as-orphan inference, environment asymmetry, and age.
Direct production/development comparison supports the first two. The production
remote-display branch has no age condition. No temporary instrumentation is
needed to distinguish those paths.

Protect uncorrelated displays in both environments, with the diagnostic reason
remote_display_ownership_unproven. Do not introduce a service-name allowlist:
systemd identifies the observed example's foreign owner but is unnecessary to
establish that Agent Browser has no positive ownership evidence. Known retained
display and lifecycle-owned browser rules stay intact. A GC regression covers
unknown, fresh and old ages with a terminator that panics if invoked, including
explicit force-without-review and unattended entrypoints.

## Validation checkpoint

The full resource-module group passes34 tests in1.68 seconds, including the
new inventory and GC effect-boundary regressions. Cargo-safe Rustfmt passes.
Policy wiring passes11 cases; changed links, handoff guidance and validation
selection contracts pass. Documentation builds successfully with Next Webpack.
The initial Turbopack attempt refused a dependency symlink outside this checkout;
its failure is retained as setup evidence, not described as a product pass.

The changed package script only updates the renamed production safety-test
filter. The selector's broad package-file recommendation over-approximates this
change: no installer, VM provisioning, Guacamole, PostgreSQL, RDP, release asset
or dependency behavior changed, so unrelated live/provider and release suites
are excluded. Focused Rust, strict workspace Clippy, formatting and the applicable
source/documentation gates qualify this packet, rather than comprehensive CI.
The installed shared skill is not overwritten by a source-only repair.

The same public loop in a disposable home reproduces candidate plus
terminate_process against the installed pre-repair binary while the foreign
service stays active. Its cleanup leaves zero task-owned processes, and the
foreign process identity is unchanged. Exact candidate replay passes: disposition protected, no cleanup action, no
foreign PID in GC dry-run, unchanged foreign process identity and zero fixture
residue. The frozen debug binary SHA-256 is
`a7d0294c2255e70d8693bf6592fb672e9e4cf27e326a9ed39a4f7cfdc52bd488`.
This is a source-validation executable, not an installable sealed release.

Strict workspace Clippy passes without warnings. The single self-review verified
that retained display and owned closing-browser predicates are unchanged, both
production and development protect the unknown display, force-without-review
cannot turn it into a candidate, and the original public inventory path clears
the symptom. No temporary debug instrumentation was added. Throwaway harnesses
and first failure/setup/cancellation receipts stay in private debug artifacts.

Source validation commands:

```bash
CARGO_PROFILE_TEST_DEBUG=0 scripts/ci/rust-tests.sh --focused service_resources::tests
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
CARGO_PROFILE_DEV_DEBUG=0 scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
pnpm test:policy-wiring
pnpm test:documentation-links -- --base origin/main
pnpm test:remote-view-handoff-docs
pnpm run test:validation-selection
pnpm --dir docs exec next build --webpack
```

Compiling stages used four Cargo jobs with10GiB for RED/GREEN tests and6GiB
for final Clippy/debug compilation. Formatting used2GiB; the host reserve
remained16GiB. Capacity waits, the original unmodified RED, cancelled admission
waiters and the Turbopack setup refusal remain retained. Hosted CI was skipped
under operator policy. No production installation, allocator change, foreign
service interruption, profile reset or cleanup effect occurred in this packet.
Current state: qualified source and public candidate proof; PR integration and
any later deployment remain separate.
