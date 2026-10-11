# Graceful host restart repair

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P234 / #249
Related lanes: none
Baseline: 5a24570781fa81ac6085698709dab0a3134b4c21

P233's public preservation loop retained the exact browser PID, profile lock and CDP endpoint through host termination, but its first continuation command failed on incomplete endpoint metadata. A minimized loop removed leave-open and the surviving browser: ordinary close, exact host SIGTERM, then open still failed. Only runtime-host.token remained. The CLI's duplicate-host guard correctly refuses partial endpoint evidence; weakening it would hide owner ambiguity.

A one-variable probe removed only the exact task-owned token after host-exit proof. The next open succeeded, and cleanup left zero process residue. Source inspection confirmed that graceful owner cleanup removed PID, identity, version and executable records but omitted authentication metadata. Owner cleanup now also removes its matching token, guarded by the existing endpoint ownership proof. Changed tokens, unreadable files, empty expected tokens and a replacement socket preserve metadata. The public red loop predates the fix; targeted tests additionally cover the ownership fences.

Twelve daemon tests, 46 connection tests and 39 output tests pass. The unchanged minimized public loop now finds no endpoint metadata and successfully reopens, with zero residue. The initial strict Clippy and format checks pass.

That correction exposed a second boundary in the original loop: the new host starts, but the first getter returns browser_session_runtime_browser_missing. One explicit open attaches the same original PID/CDP, after which getter, human preservation output and explicit false close succeed. A cache-empty named-task regression failed with the same runtime error before changing command routing. Named-session commands now use the existing fenced liveness/attachment path before operating on their recorded tab, including headless lanes that do not use automatic native admission. No new recovery owner, provider request, fresh profile allocation or task identity is introduced.

The browser-session suite passes 87 tests, with one ignored, including the new first-command attachment case. Final format and strict workspace Clippy pass. The unchanged original continuation loop now preserves the original PID/CDP through graceful host exit, continues directly with a getter, reports browser/task preservation, and closes the exact browser on explicit false. It finishes with zero task-owned process residue. Tested unsealed debug CLI SHA256: f4d75614f21257677f8b4a47797fa5f644cb5c1b1844b069ac5ee59b506f4466. The corrected docs site builds; handoff documentation and changed-document links pass. No installed runtime was published, no private account was used, and no foreign browser, service or provider was stopped. Reboot remains deferred. The wider failed-upgrade shutdown dispositions and multigeneration qualification in #183 remain separate.

Private source-fixture receipts retain original and minimized failures, the task-token probe, the attachment-unit red, the first successful empty-profile restart, the explicit-open control, all validation and exact process cleanup. An attempted soft-limit adjustment found its task scope already exited before any effect; no property changed. The later builds use reviewed bounded task scopes while preserving the 16 GiB host reserve.
