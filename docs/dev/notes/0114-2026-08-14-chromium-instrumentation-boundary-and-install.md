# Chromium instrumentation boundary and install

Date: 2026-08-14

## Bottom line

The installed Chromium candidate does not contain hidden agent-browser input,
navigation, or interaction machinery. Its downstream source delta consists of
two narrow changes: `Navigator::webdriver()` returns `false`, and Blink clears
a stale shape result while rewinding line overflow. The second change is a
layout correctness repair with a regression test, not an automation-concealment
feature.

No further Chromium source changes are presently justified for a normal
headed browsing experience. Browser launch posture, durable profiles, display
and GPU behavior, desktop input, pacing, and account state belong in
agent-browser. Additional engine patches should require a reproduced,
page-observable Chromium inconsistency that cannot be repaired coherently at
that layer.

This is not a claim that an instrumented browser is universally
indistinguishable from an uninstrumented browser. A website may infer
automation from behavior, environment, network reputation, inconsistent
fingerprints, or browser/protocol side effects. The current build proves only
the narrower facts recorded below.

## What the candidate contains

The candidate was built from Chromium source
`778b60e96df92414e75d0fb241b108ba1da452db` on upstream revision
`b84d5f5c55a43d2f7778cfbc1996d3b835296d1c`.

Its patch queue contains:

- `Make navigator.webdriver non-advertising`, which changes
  `Navigator::webdriver()` to return `false` instead of exposing Chromium's
  automation-controlled and DevTools override state.
- `Fix stale shape result during line overflow`, which clears an invalid
  `ShapeResult` when an `InlineItemResult` is expanded after overflow and adds
  a focused DCHECK-backed regression test.

The candidate does not add hidden mouse, keyboard, scrolling, navigation,
timing, profile, canvas, WebGL, audio, font, locale, timezone, device, or
permission behavior. Agent-browser still performs CDP input and target
management when a CDP-attached access posture is selected.

## Detection boundary

For a CDP-free, headed browser controlled through the ordinary desktop input
stack, websites do not receive an attached agent-browser protocol connection.
That posture is the clearest boundary when a site must not receive browser
instrumentation. Agent-browser's current CDP-free service action is still a
lifecycle foundation rather than a complete general-purpose execution path,
so this statement describes the architecture boundary, not a claim that every
current workflow already uses it.

For CDP-attached sessions, `navigator.webdriver === false` removes one explicit
automation disclosure. It does not prove that all DevTools attachment side
effects are absent. Before changing Chromium again, compare the exact target
workload under stock headed Chromium, the patched headed build without CDP,
and the patched headed build with CDP. Patch only a reproduced difference that
is internally coherent and browser-owned.

Do not add speculative broad fingerprint spoofing. Independent overrides for
canvas, WebGL, fonts, plugins, hardware, timezone, locale, media devices, or
viewport are likely to create contradictions and increase maintenance risk.

## Installed candidate

Agent-browser's user configuration now binds
`service.browserBuildManifests.stealthcdp_chromium.manifestPath` to:

```text
/home/ecochran76/workspace.local/chromium/artifacts/chromium-stealthcdp-candidates/linebreaker-778b60e96d/current/manifest.json
```

Installed identity:

- artifact: `153.0.8003.0+stealthcdp.02d1c35532eb`
- browser: `Chromium 153.0.8003.0`
- Chromium source: `778b60e96df92414e75d0fb241b108ba1da452db`
- patchset source: `9f8084c151aab38c62b72d0830ebec0a27562796`
- patch queue SHA256:
  `02d1c35532eba8d4eaf18a043b8db1a8a8f47b0374c830378444263bda37fd84`
- executable SHA256:
  `75aad493dc70e9f2529985e842bedf5b34a325fbf455fb6670c1b8abecbaf3de`

The repository freshness checker reported `fresh=true`. The artifact smoke
reported success, Chromium `153.0.8003.0`, a reachable DevTools endpoint, and
`navigator.webdriver=false`. Agent-browser v0.28.0 then resolved the candidate
as its default `stealthcdp_chromium` build with a valid manifest, an existing
executable, matching SHA256, successful smoke evidence, and no launch-config
warnings.

The candidate was compiled with `dcheck_always_on=true`. It is therefore the
right diagnostic build for the repaired Blink invariant, but it is not yet a
production-parity performance build. A later production candidate should be
rebuilt with production DCHECK policy and pass the same correctness and
workload controls rather than merely disabling the assertion.

Six agent-browser daemon sessions were live during installation. They were not
restarted, and no retained browser or profile was closed, copied, or migrated.
The new binding applies to subsequent service-owned launches; already-running
browser processes retain their original executable image until their normal
lifecycle ends.

The full install doctor remained non-green only because it detected pre-existing
duplicate retained-profile pressure. Runtime convergence was otherwise green,
and the default service's no-launch status readback selected this exact
candidate. The profile-pressure warning is a separate ownership issue and was
not bypassed during this installation.

## Decision for future work

Treat the current candidate as the accepted diagnostic engine for new
agent-browser launches. Do not represent it as proof of universal automation
obliviousness, and do not enlarge the Chromium patch surface without an
evidence-backed browser-internal defect.

For workflows that require the strongest separation from browser
instrumentation, finish and validate the CDP-free headed desktop-control path
in agent-browser. That work should preserve visible operator control, retained
profile ownership, explicit site policy, and auditable lifecycle receipts.
