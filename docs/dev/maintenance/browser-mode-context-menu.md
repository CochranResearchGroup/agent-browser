# Browser mode context menu

Product lane: PL-BUGFIX
Scope: expose existing mode conversion from browser and tab right-click menus.
Source baseline: 0e138050fa2ea26050a717833a9f958c177ad5fc.

The browser heading context menu and managed tab context/actions menus offer
Relaunch with CDP and Relaunch without CDP. Both reuse BrowserModeDialog and
its owner journal inspection, affected-task review and explicit relaunch.
Only an exact managed task with its owner session enables conversion. Opening
menus has no browser effects. Observed windows do not gain managed custody.

This bounded patch is prepared in the existing checkout without a branch or
worktree transition. Existing profile-copy lifecycle and workspace/site-guide
edits belong to other lanes and are excluded. CLI flags and command behavior
are unchanged; the existing CLI mode-convert help remains applicable.

Validation: mounted browser-heading and tab context-menu cases exercise both
requested modes, exact task/owner routing, and inspection-only selection. The
existing compact rail workflow, dashboard TypeScript/build and handoff docs
checks cover the surrounding interface. Results are recorded at closeout.
Production installation and real-account authentication are separate gates.

## Local results

Passed: `pnpm test:compact-native-workspace` (including mounted heading/tab
right-click choices in `test-browser-mode-inspection.cjs`),
`pnpm --dir packages/dashboard exec tsc --noEmit`, `pnpm build:dashboard`,
`pnpm test:remote-view-handoff-docs`, `pnpm test:service-api-mcp-parity`,
`pnpm --dir docs build`, and `git diff --check`. The skill initially exceeded its 10 KB cap; concise
wording brought it under the cap and the parity gate passed. Dashboard static
export retains the existing warning that Next rewrites are not exported.
Checks use local Linux Chrome and synthetic owner replies; no hosted CI,
installed dashboard replacement or account login was attempted.
