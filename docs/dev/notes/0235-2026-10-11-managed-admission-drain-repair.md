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

This is source and isolated fixture qualification. No installed runtime was published, private authentication performed, foreign process stopped, provider mutated or reboot performed. The separate installer fixture's unseeded ordinary profile and orphan-host cleanup still need repair; its previously failed receipts are preserved. Wider upgrade-progress and failed-upgrade qualification remain separate issues.
