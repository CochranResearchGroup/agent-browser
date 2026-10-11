# Development runtime repair

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P229 / #79
Related lanes: PL-PLATFORM

The default isolated development runtime pointed to a deleted checkout and mixed an old dashboard executable with current guidance. Publication from an extant, sealed source restored coherent launchers and services. Generation metadata now retains the reviewed source commit and manifest digest independently of its original binary path. Unsealed publication makes no source claim; mismatched seals reject before writes.

The installed workflow exposed two remaining defects: headless development doctor demanded an unused legacy privilege helper, and an authoritative older SQLite database lacked a newer migration-summary key. Doctor now selects headless, native or explicitly required legacy readiness. SQLite receipt reading falls back to the original migration-source table without replacing state, importing JSON again or writing synthetic migration evidence. The unused legacy development provider manifest was archived privately; provider resources were unchanged.

Regression tests failed before fixes. Nine store tests, five development-doctor tests, development publication fixtures, format and strict workspace Clippy passed. An installed sealed candidate built from commit `79844519fcff487d7c40d191be52e8c8493ea67f` has binary SHA256 `6a87f4e3bc2fd4eecea8cf82783438723a844480e152ab26dccf1a5a4e99b189`. Both installed doctors passed. Three disposable browsers opened, read `about:blank`, closed and left no test browser processes. Skill digests matched. Publication and smoke both verified unchanged production identity.

Preserve failed attempts: the first installed smoke rejected the missing SQLite receipt key; the first sealed publication rejected a source-tree format mismatch. The corrected publication regression uses the actual candidate builder's SHA256 tree format. Its green run and final installed workflow supplement those failures rather than relabeling them.

Repository reconciliation preserved dirty tracked/untracked work in a durable backup ref and verified recovery bundle, and preserved three local policy commits on an archive branch before aligning the sole checkout with canonical main. No surplus registered worktrees existed. Historical active-worktree claims were reconciled to integrated custody.

This qualifies headless isolated development and source provenance only. It does not qualify native handoff, real account authentication, production promotion or live-browser upgrade preservation. Reboot remains deferred. Issue #176 is the next serial repair.
