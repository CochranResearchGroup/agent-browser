# Managed upgrade admission repair

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P235 / #251
Related lanes: none
Baseline: 207fdb5ef5144f192d66bfd1eb4530d230790d63

A disposable public open attempted three launches through a /bin/false sentinel despite a valid persisted upgrade admission drain. The minimized reproduction needs neither the installer nor a legacy provider identity. The native action adapter enforced admission, but managed requests executed earlier and bypassed it. Action classification already defaults unknown actions to effects; changing that classification would not repair the dispatch gap.

Four provider-free broker composition regressions failed before the production change: explicit launch, implicit launch from a URL read, physical recovery, and tab acquisition without an owned tab. The initial test draft had catalog-access and ordinary-admission fixture errors; corrected tests demonstrated successful forbidden effects before repair. Those attempts remain private evidence, not accepted red regressions.

Managed command boundaries now use the existing CLI runtime admission authority before profile or browser effects. Implicit launch, physical recovery and tab acquisition receive effect checks even when the requested action is observational. Live observations still attach the recorded browser through the existing publication and operation fence. Managed status remains readable with a malformed drain, while browser effects reject it. Existing exact installer claim rules remain unchanged; an ordinary profile does not gain admission merely by copying a transaction claim.

The broker suite passes 91 tests with one intentionally ignored. The four final regressions additionally verify blocked navigation, close and tab creation on a live task without state, launch or provider changes. The exact installer-claim regression and 39 output tests pass. Format, strict workspace Clippy, documentation build, handoff guidance and documentation links pass. All five user guidance surfaces are updated. No service schema, client protocol or new authority is introduced.

The unchanged minimized public open now returns runtime_admission_draining without launching. Its no-drain control still reaches three sentinel attempts. A real disposable about:blank Chrome task preserves its original PID/CDP through exact host termination, reattaches directly for a URL read during drain, and rejects navigation. Removing only that fixture's marker allows the ordinary human preservation output and explicit false close to finish. All three public loops leave zero task-owned process residue. Tested unsealed debug binary SHA256: 18dbd6584a6fb73f2a6c79ae59378ef469d1573d4e6304d311720e04d6c12e5a.

This is source and isolated fixture qualification. No installed runtime was published, private authentication performed, foreign process stopped, provider mutated or reboot performed. The installer acceptance limitation below was discovered after primary integration; its failed receipts are preserved. Wider upgrade-progress and failed-upgrade qualification remain separate issues.

## Claimed-wire acceptance follow-up

PR252 qualified ordinary drain enforcement, but its pure exact-claim test did not qualify daemon dispatch. The original source-free installer fixture and a minimized CLI wire request rejected an exact canonical route-viewer claim. The claim survived command rewriting; native caller attribution ran after managed dispatch. The issue was reopened before the follow-up.

The existing native attribution now runs before managed dispatch, including browser_session actions that the fallback launch classification skips. Caller assurance, exact transaction/revision matching and ordinary-profile exclusion remain unchanged. The first attribution move still failed because it omitted those managed action names; that failed probe remains retained. The final minimized claimed request reaches three sentinel launches as expected. Ordinary open and cold URL-read requests still return the typed drain error before implicit launch, while a live retained task reads its URL during drain and blocks navigation with the same browser PID/CDP.

The installer fixture now supplies its own empty synthetic ordinary profile and removes inherited Agent Browser settings. Its finalizer retires only hosts with the exact fixture HOME, nonce, executable SHA, start identity and inode, using Linux pidfds before deleting the fixture home. This test utility requires Linux and Python 3.9 or newer with pidfd support; it has no privileged service role. Negative checks preserve wrong-home, wrong-nonce and unproven executable processes. An intentionally failed fixture assertion cleans both owned hosts without outer-wrapper rescue and preserves the expected failure result.

The complete source-free installer fixture passes, including exact claimed admission, ordinary rejection, repeat installation, repair and GC assertions. Its own finalizer retires two hosts with zero residue; the outer wrapper retires none. An earlier ten-minute wrapper stopped only its launcher and lost the fixture child's exit result; that run is excluded. The accepted run streams output from an owned process group with a twenty-minute bound and returns status zero. No acceptance assertion was weakened.

Twelve daemon tests, the admission contract test and strict workspace Clippy pass. Primary broker, output and documentation gates remain applicable because their inputs are unchanged. The follow-up public wire, ordinary drain, cold getter and real live continuation loops all leave zero owned residue. Accepted unsealed debug binary SHA256: e9492d48e92b4b044d6229dd0507a4d637c91593f0d22d3fe31c274cc9d7e9f6. Compatibility profile labels in the synthetic installer test do not imply legacy provider provisioning. No production or development installation was published by this follow-up, and reboot remains deferred.

Follow-up source checkpoint: c5200fad027065f02e7ed72bbe5583b7305d4ab0. Final format and documentation links also pass.

Follow-up integration: [PR253](https://github.com/CochranResearchGroup/agent-browser/pull/253), merge 28aa65a0927d39c5687c319920e3f4b1c45e77f8. Issue251 is closed.
