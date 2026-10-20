# P254 compact workspace fixture acceptance

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: [P254](../plans/0254-2026-10-20-compact-workspace-fixture.md), CochranResearchGroup/agent-browser#330
Related lanes: P251, P248, P253, UX301
Pull request: https://github.com/CochranResearchGroup/agent-browser/pull/332

## Result

The existing full compact workspace fixture now matches integrated Properties, combined selection and Native failure categories. Missing provenance is explained once without inventing a peer owner. Known inventory values remain visible, while absent Desktop and Created facts remain Unknown. Selection is one `remote_view_open` request that retains maximize, exact browser/session/desktop binding and no navigation; every scenario rejects standalone `view_focus` requests.

Typed denials at HTTP400 and HTTP200 and unknown/legacy refusal cases now apply at that combined request. Denial does not create an iframe, leak private diagnostics or automatically retry. Four held success/denial cases cover the same and independent desktops: Alice remains selected after the late reply, and the independent Alice viewer document is preserved. Share waits only for selection on the same desktop and does not issue another task selection. Existing clipboard, close confirmation, peer preservation, handoff policy/deletion, absent/unverified inventory, mobile recovery, rail sizing and 200-tab checks remain exercised.

## Evidence and limits

The unchanged command first failed with a 30-second locator timeout at Bob's absent Service row. Aligning Properties reached an obsolete generic Native error message. Updating request mocks and expectations allowed the full mounted fixture to reach its final exact-binding check; that newly added expectation initially named the wrong synthetic desktop. It was corrected from desktop-c to the fixture's actual desktop-b. These are retained fixture failures, not a flaky pass or production repair.

Final qualification at code checkpoint b889407efc1f590d817607018f6c38387009c77d passes the actual `pnpm test:compact-native-workspace`, native-task-selection (13 groups), mounted profile Properties, inspector action contracts, policy wiring (11 tests), fixture syntax and validation selection. The [bounded receipt](../receipts/p254/mounted-acceptance.json) records script/bundle hashes, all four late-response outcomes and fresh kernel identity readback: all seven captured fixture processes are gone. Changed documentation links and patch hygiene are checked after this qualification record is committed. Raw synthetic browser screenshots and logs are retained outside Git.

This repairs test maintenance only. Production React/client/Rust behavior and installed binaries are unchanged. No Native provider, real account, profile, shared runtime or pinned candidate was changed. P253 source publication remains a separate installation task. The dashboard primary remains available for the continuing dashboard objective; no auxiliary checkout was created.
