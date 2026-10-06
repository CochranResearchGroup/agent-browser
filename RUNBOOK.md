# Runbook

Updated: 2026-10-05

## Current execution

P223 version 6 is ACTIVE under the operator's explicit 2026-10-05 direction to execute the fresh-agent handoff and stop with a checkpoint before 1.5 million tokens in the resumed thread. Preserve the prior exhausted allowance and failures. [The current plan](docs/dev/plans/0223-2026-10-04-finish-agent-browser-acceptance.md) owns scope. Implementation remains in `platform/p220-remote-view-consumer`; protected integration and production promotion are separate boundaries.

Prior M1–M3 acceptance remains qualified within its recorded scenarios. M4 and M5 remain incomplete. The historical checkpoint and original failures remain preserved in the archived runbook and private receipts.

### Resumed recovery packet

The prepared absence-reconciliation candidate was published only to isolated p221. Exact binary SHA256: `b1746902c66c85e09334b5f036393598c0d1e4c9ce0a741db1e5c76a343c8e1f`. A current optimized build check reused these bytes without compilation. Publication verified production and default development unchanged; the development runtime doctor passed. The exact unobserved recovery claim remains historical evidence and its occupancy is reconciled.

The retained original handoff expired during the pause. Authenticated presentation now returns HTTP 410 with `remote_view_handoff_expired`; its expiry was not rewritten. Therefore the original-link recovery outcome is not accepted. A separate normal post-expiry open reused the original logical browser, profile, session and tab, with a new physical browser and target. Its new handoff reports operator-visible ready and redirects to native Remote View through existing authentication. An existing acceptance observer opened the native desktop directly with its saved authentication. Mouse focus and keyboard input produced a synthetic marker in the actual Chrome address bar; Escape restored the page, and normal automation read its URL successfully. Authenticated application continuity remains unverified because the application shows its interstitial.

The runtime also demonstrated a distinct recovery defect: a stopped browser PID was reused by an unrelated task-owned observer, and the retained PID alone blocked recovery. Closing that exact observer allowed recovery. The follow-up checks current process identity before a fresh full profile census, preserving unrelated processes and rejecting unknown identity or occupied profiles. Its focused recovery tests (59), format and strict workspace Clippy passed. The optimized candidate was published only to p221 with SHA256 `8799653adcedbdbe4d62957e25c7b34df2939130c80933cee6a9b15becdca43a`; production and default development remained unchanged. Continued automation and three fresh-profile launch/read/close/residue cycles passed. The same new handoff reopened after this update. These observations do not rewrite the original expired-link failure or prove application authentication.

One probe omitted the explicit profile selector and created a disposable session with the same name. Name-based close refused ambiguity. An authenticated exact-session close removed only that disposable browser; fresh OS readback found its PID absent. Preserve those failed harness receipts separately from the recovery result.

Private receipts use `p223-resume` and `p223-pid-reuse` prefixes under the existing evidence root. They bind source inputs, publication, original expiry, claim reconciliation, recovery, doctor, cleanup and follow-up validation. Do not copy credentials or browser content into product sources.

The generic doctor demonstrated an isolated-install contract defect: it inspected the production PATH command, dashboard port and legacy privilege helper when invoked by the isolated candidate. Diagnostics now bind to the selected development binary, exact launcher bytes and configured dashboard port. Native provider diagnostics use read-only inventory admission for the selected pool. Installed p221 SHA256 `658305704703c833171cbfe597705fd75f73a3adc107bfcee21bf968b15ed122` passes both JSON and human-readable install doctor. Production and default development remained unchanged. Three fresh-profile launch/read/close/residue cycles passed, followed serially by same-new-link native keyboard input and continued automation. Provider inventory readiness does not prove application authentication.

Batch validation passed format, strict workspace Clippy, client contracts, dashboard typechecking, architectural seams and documentation checks. The comprehensive Rust run preserved one failed status fixture; its correction passed all 599 service tests. The separately run Service Model suite preserved a transport-order fixture failure; the corrected recovery case and other 326 tests passed. These corrected-surface passes supplement the original run rather than relabel it as a clean aggregate.

The prepared provider dependency entered fresh canonical Remote View main through [PR 310](https://github.com/CochranResearchGroup/remote-view/pull/310), exact merge `8b11f7f84d36e4d5284454be1fad5a845a6aefbc`. Its validated tree publishes existing native routes only for a joined current desktop generation and reclaims exact stopped-slot residue on restart. Full serial tests, format and strict all-target Clippy passed. Provider runtime publication remains separate; the older grant-scoped PR 291 is preserved.

A normal unprofiled `remote-view open` demonstrated another concrete gap: the parser forwarded an implicit `default` catalog selector and failed before allocation. State inspection found no new session, handoff or default-profile launch claim. The repair omits implicit startup defaults for remote-view admission and ordinary session commands while preserving explicit selectors. Thirty remote-view tests and two native routing tests pass; optimized candidate validation is in progress. The first installed replay still failed because the final shared-host boundary reinserted cached lane defaults. That counterexample is retained. The corrected boundary preserves explicit selectors and leaves named disposable sessions unconstrained; its expanded core checks are running. Installed workflow qualification remains pending.

Next: publish the selector repair only to p221, verify a normal disposable open, native controls, continued automation and exact operator-stop cleanup, then prepare the dependent consumer PR from fresh canonical main. Preserve original expiry, application continuity limits and all failed receipts. Reuse unaffected passes.

## Product direction

Complete browser work with less effort and fewer failures. Use a task-appropriate existing, managed persistent or disposable profile. Remote View supplies the native desktop/viewer and existing authentication. Human handoff and return happen in the same browser. Follow [the product contract](docs/dev/policies/0054-browser-product-contract.md).

## Context boundary

Start with [the active plan index](docs/dev/plans/README.md). Historical plans are reference-only. [P223 revision 5](docs/history/2026-10-05-legacy-planning/p223-plan-v5.md) preserves the earlier scope; its removed mechanism and access requirements are not current instructions.
