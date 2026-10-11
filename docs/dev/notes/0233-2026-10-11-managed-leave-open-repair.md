# Managed leave-open close repair

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P233 / #247
Related lanes: none
Baseline: 4d857509d9abfc3db23909994888df8874a0847d

Three isolated public CLI runs reproduced final managed-browser shutdown despite the leave-open flag, environment setting or configured launch preference. The managed close path ended the logical session without consulting the preference. The repair preserves the exact logical task and its browser, retaining profile occupancy and protecting it from logical expiry and physical idle cleanup. Explicit false removes the preference and ends the task; peer sessions retain their browser. Preservation requires current publication custody and a live owned browser. It does not release an externally occupied profile.

A persisted session retention set restores the preference when the managed host loads state. The CLI now forwards both true and explicit false on managed commands, which bypass prestart launch. Full-shutdown replacement explicitly overrides retention. Human output distinguishes browser and task preservation from task closure with a remaining peer browser.

The first host regression failed before the repair. An initial green run exposed an incorrect test response path; that failed attempt remains retained. Public acceptance then caught missing CLI forwarding even though all 50 consumer tests passed. A separate routing regression failed with null instead of true and passed after command forwarding. Compilation also caught an unqualified JSON Value name in the human formatter, corrected before the passed quality gates.

The unchanged original public loop now preserves the exact browser PID, CDP endpoint and profile lock through leave-open close and standalone-host termination. A separate live-host continuation loop reads the original URL from the same process, prints the preservation result, and verifies explicit false closes that browser. Both finish with zero task-owned process residue. These are disposable about:blank workflows with an unsealed debug CLI, SHA256 cf9f778234eb7160f88ffffe5b9181c74b2d4ec4a2bacdd47d543b4327785b4b. No installed runtime was published.

Automatic continuation after direct standalone-host termination remains failed: the surviving browser cannot be reached by the next CLI command because endpoint metadata is incomplete. Exact cleanup left zero residue. This is separate from leave-open close and remains current evidence for issue #183; model state reload coverage does not prove that CLI restart path.

The broader CLI core run passed 1,113 tests, with one ignored, and failed its existing transaction source-order fixture. Canonical baseline already contains the mismatch: workstation preparation moved into prepare_workstation_host, but the fixture still expects a direct legacy privilege-installer call. The focused fixture reproduces the failure. Its anchor now checks the current preparation call in the original effect sequence rather than removing the ordering assertion. The corrected CLI core suite passes all 1,114 tests, with one ignored.

Other passed checks: 50 managed consumers, 329 Service Model tests, six runtime replacement tests, focused CLI routing regression, docs build, handoff docs, service client contracts, format and strict workspace Clippy. Final batch format, strict workspace Clippy and changed-document links also pass. Private receipts preserve resource-admission deferrals, the bounded task-scope adjustments, failed attempts, public red and green results, and exact cleanup under the serial-repair checkpoint. The host reserve remained unchanged. Foreign runtimes, credentials and provider state were untouched; reboot remains deferred.
