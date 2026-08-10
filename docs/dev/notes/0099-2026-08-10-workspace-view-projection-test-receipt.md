# Candidate 2 Workspace View Projection Test Receipt

Date: 2026-08-10

Role: distinct independent tester

Verdict: **PASS**

Candidate 2 has no scoped test failure at the final target identity. The full
Plan 0099 dashboard packet, both selected-workspace compatibility gates, the
dashboard build, all four literal ownership guards, and every authorized local
check selected by the validation selector passed.

## Scope And Target Identity

The test scope is the 25-path Candidate 2 implementation, planning-direction,
test-contract, and bounded Plan 0104 repair packet. Concurrent Candidate 3 and
Candidate 4 plans and audits are excluded from failure attribution.

Base revision:

```text
7f89ea491b8dac48cb7eb163c05e3a001b455137
```

The path-sorted binary diff stream was built by applying
`git diff --binary --no-ext-diff 7f89ea49 -- <path>` to paths present at the
base and `git diff --no-index --binary /dev/null <path>` to new paths, in this
order:

```text
docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md
docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md
docs/dev/plans/0104-2026-08-10-workspace-view-work-cycle2-residual-repair-plan.md
package.json
packages/dashboard/src/app/page.tsx
packages/dashboard/src/components/service-panel.tsx
packages/dashboard/src/components/workspace-remote-viewport.tsx
packages/dashboard/src/hooks/use-selected-workspace-context.ts
packages/dashboard/src/hooks/use-workspace-view-preferences.ts
packages/dashboard/src/lib/selected-workspace-context.ts
packages/dashboard/src/lib/service-view-streams.ts
packages/dashboard/src/lib/service-workspaces.ts
packages/dashboard/src/lib/workspace-browser-selection.ts
packages/dashboard/src/lib/workspace-view-projection.ts
packages/dashboard/src/lib/workspace-view-stream-selection.ts
scripts/test-cross-seam-interlocks.js
scripts/test-dashboard-browser-table.js
scripts/test-dashboard-inspector-actions.js
scripts/test-dashboard-selected-workspace-chat-packet.js
scripts/test-dashboard-selected-workspace-context.js
scripts/test-dashboard-view-streams.js
scripts/test-dashboard-workspace-inspector-tab.js
scripts/test-dashboard-workspace-navigator.js
scripts/test-dashboard-workspace-nodes.js
scripts/test-dashboard-workspace-view-projection.js
```

Recomputed aggregate SHA-256:

```text
70a5d082275e23385d7bd46ad2c3b9237b53d176c9c0bdaf2a88853dcefdcac5
```

Per-path SHA-256 identity:

```text
32933c00231b8a10236921053918b935c7c29e776b5f0002268e9fb538150e07  docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md
43b1258b013d5bd3005accceef8b1778c93b3e46151b992953dc73dac1cc2f9d  docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md
cd6146b50b90e60472cd4d5f942ed417bbb9016ed486256b4002fa7b0f14b634  docs/dev/plans/0104-2026-08-10-workspace-view-work-cycle2-residual-repair-plan.md
d99b02466208fa2cd5791097bf5e7ddb485df8ea2ae15848c0572f96e62b087f  package.json
43dbe5d42825f527d94fd33a8cabfab525c4df2417d2f7c13fcfc4086ecb2226  packages/dashboard/src/app/page.tsx
cac97fd123e5905812b2807630a1e6140732419fba97749dd1e1a7ec95d1c7ae  packages/dashboard/src/components/service-panel.tsx
d884cae6635360a4fae5c7fb92050aa905b5765391d24b4a6f7997f48bcc0590  packages/dashboard/src/components/workspace-remote-viewport.tsx
52169666fd47cc769ae2ce3abe45ca8da46f71614b294d5eaaba686a252ceecf  packages/dashboard/src/hooks/use-selected-workspace-context.ts
1f03cae96bc15d6b6049feeb495c160a34a1e0bacce78aa65c025dbc4b24ea12  packages/dashboard/src/hooks/use-workspace-view-preferences.ts
2e0a7d14b28b53099b62aab9077863f8e34d8526d67ab8da1d35cbc2b9b134ac  packages/dashboard/src/lib/selected-workspace-context.ts
ac9526799f08617210a406c37fed3ff81604757670fb4a5bde3f42a885d14713  packages/dashboard/src/lib/service-view-streams.ts
bc3002992de07bd77632d1352e75f88d1bc2ed8e34d392a855ee25de4e99185e  packages/dashboard/src/lib/service-workspaces.ts
ef254ddeaca69dc4348eb80fa5b525db8ad0462a70999064e3f27f23f3be1788  packages/dashboard/src/lib/workspace-browser-selection.ts [deleted; base content]
d5a374ac58ded0d3e53c9471e38d68729aa47b609a758218d82561bfe6ab2a8b  packages/dashboard/src/lib/workspace-view-projection.ts
06247f22e8bc73c1d78ee6d784c61e171f0a777edfb22938b14b3ebb24d26ece  packages/dashboard/src/lib/workspace-view-stream-selection.ts [deleted; base content]
9a0198e3241725ef4806a5c721e92cfb68b72d19d7f0108deb890de9b3f7f0d3  scripts/test-cross-seam-interlocks.js
3e793c05f7a365558ec7fe93cfa2042130fea249912db857ce3e2313bdcf0343  scripts/test-dashboard-browser-table.js
62317f1e9b285607c63870e2b163153b8b52cabbd17ebad709a5a988aec3de93  scripts/test-dashboard-inspector-actions.js
7e975463c15edb915c9e22a6ed0ce31e0f33df10f1af7257bf063c38923e2587  scripts/test-dashboard-selected-workspace-chat-packet.js
a481316f89b98877b1b5d207f42215a650650c6ac24460a8b24ecdd9e8e01cfd  scripts/test-dashboard-selected-workspace-context.js
2e474841cf5598538afc3ff0b63ac71cbeec5795a4269f3619e3cb9232a1d29d  scripts/test-dashboard-view-streams.js
80a162bf2f4fa52c9278098280d6b23c0a030b3e17790be7d3a1543cbe16e757  scripts/test-dashboard-workspace-inspector-tab.js
781ac8d58888c74b4ee228e66b079ab623e5ee9e30e29b7a0dd58e9717c232c9  scripts/test-dashboard-workspace-navigator.js
3723e81f9033cce849ce57d6c6846faf97e40c5abd8c5125a92eb176f01eaf0d  scripts/test-dashboard-workspace-nodes.js
550919388c105e7df2f47a38c71e23bff0b842a7bbff4b39a0c93840e5c30205  scripts/test-dashboard-workspace-view-projection.js
```

The frozen Plan 0099 SHA-256 is
`698052922136ce5608f3e54c2bfad3ebc6999ff6c698dbde31ae4cab52fa6514`.
The terminal work-audit SHA-256 is
`7e864b467557ce8bbce0ffd32196727fe68e7ad4c3b7c2dd32b29e8d97a205c6`.

## Targeted Dashboard Results

All 11 Plan 0099 test commands passed at the final target:

- `pnpm test:dashboard-workspace-view-projection`;
- `pnpm test:dashboard-view-streams`;
- `pnpm test:dashboard-workspace-nodes`;
- `pnpm test:dashboard-selected-workspace-context`;
- `pnpm test:dashboard-workspace-navigator`;
- `pnpm test:dashboard-workspace-inspector-tab`;
- `pnpm test:dashboard-inspector-actions`;
- `pnpm test:dashboard-browser-table`;
- `pnpm test:dashboard-browser-row-actions-render`;
- `pnpm test:workspace-viewport-controller`;
- `pnpm test:cross-seam-interlocks`.

Both selected-workspace compatibility commands also passed:

- `pnpm test:dashboard-selected-workspace-chat-packet`;
- `pnpm test:dashboard-selected-workspace-console`.

Final test-command count: 13 passed, 0 failed, 0 skipped. These JavaScript
contract scripts report command-level success rather than an internal assertion
count, so no assertion total is inferred.

The first Chat run found one stale expectation: the fixture contained an
unresolved warning incident but still expected a ready stream to upgrade the
workspace from `needs-attention` to `controllable`. The bounded Plan 0104
correction changed only that assertion to preserve incident authority. The
independent rerun passed. This was Candidate 2 replacement-test drift, not an
unrelated baseline failure and not an implementation change by the tester.

## Build, Guards, And Selected Local Checks

- `pnpm build:dashboard`: passed. Next.js compiled, type-checked, generated 7
  of 7 static pages, and finalized the export.
- Four of four Plan 0099 literal guards passed. The first three negative
  searches returned no old scoring or readiness wrappers, no references to the
  two deleted shallow modules, and no viewport-local selected-route
  reconstruction. The positive route guard found four same-snapshot
  `remoteViewRoutes` declarations or uses across the hook and context builder.
- `pnpm validation:select -- --base 7f89ea49`: passed and reported 32 changed
  paths across Candidate 2 plus concurrent planning artifacts.
- `pnpm test:route-confusion-gates`: passed all 8 reported no-launch fixtures.
- `bash scripts/release/test-verify-release-assets.sh`: passed for 7 of 7
  platform binaries, the 7-entry checksum file, Linux x64 execution, and
  release-note extraction.
- `node scripts/dev/select-validation.js --base HEAD --json`: passed.
- Final `git diff --check`: passed.

## Warnings And Residual Risk

The dashboard build emitted three instances of the existing Next.js warning
that rewrites are not applied automatically to static export output. Compile,
type-check, page generation, and export completion all succeeded.

The selector also recommended `pnpm test:service-cdp-tab-streaming-live` and
`pnpm publish:local-dashboard -- --expect-marker <changed-ui-marker>`. They
were not run because this tester was explicitly prohibited from launching
browsers, publishing or restarting the dashboard, installing artifacts, or
touching live state. Plan 0099 is an in-process dashboard architecture
deepening with no wire-contract or installed-runtime change, so these remain
operator-visible live QA rather than a scoped source-test failure.

No Rust source or cross-language contract changed in Candidate 2, so Rust
format, clippy, and unit-test gates were not implied. No browser, visual,
installed-service, or public-ingress smoke was performed. The source-level
dashboard and cross-seam contracts are fully green; runtime rendering remains
outside this tester's authority.

## Effects

This role edited only this test receipt. It did not edit Candidate 2 source,
tests, plans, or prior audit artifacts. It did not start a browser, contact or
mutate an installed service, alter runtime or tenant state, install software,
commit, push, release, or perform a live-system effect. Next.js created only
ordinary ignored local build artifacts.
