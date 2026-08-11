# Facebook authenticated-search Blink crash handoff

Date: 2026-08-11

## Purpose

Hand off the Facebook search failure observed through the retained
`last30days-facebook` browser lane. This note separates the Chromium failure
from two agent-browser control-plane defects and defines the next bounded
repair packet.

This is a diagnosis and handoff, not a live-repair authorization. No retained
profile was cleared, copied, rebuilt, or relaunched while preparing it.

## Current assessment

The evidence does not establish a corrupt profile. It points more strongly to
an upstream Blink line-breaking condition exposed by the authenticated
Facebook posts-search DOM and made fatal by the promoted custom Chromium
build.

Confidence:

- High: the immediate renderer failure is a Blink `LineBreaker` DCHECK, not an
  agent-browser selector, timeout, or navigation error.
- High: the custom stealth patch did not introduce the failing layout code. It
  changes `navigator.webdriver` in `navigator.cc` only.
- High: the promoted build has `DCHECK_ALWAYS_ON=1` and
  `is_official_build=false`, so the invariant aborts even though this is a
  non-debug build.
- Medium: updating or backporting newer upstream Blink behavior will repair
  this failure. Newer official Chromium source explicitly says that a larger
  `EndOffset()` can occur because of reshaping or float-to-`LayoutUnit`
  rounding, restores the prior item, and no longer aborts on the old
  `DCHECK_LE` assertion.
- Low: isolated retained-profile corruption is the root cause. The retained
  profile rendered Facebook home successfully. A clean profile did not crash,
  but it reached an unauthenticated `Not Found` response and therefore did not
  exercise the equivalent search DOM.

Profile or account state may still select the triggering DOM. That makes it a
possible trigger, not evidence that the profile itself is corrupt.

## Exact failure evidence

The retained custom Chromium stderr is:

`/home/ecochran76/.agent-browser/tmp/chrome-launches/chrome-83786-1786451676608.stderr.log`

Two authenticated `/search/posts/` attempts, with and without a result filter,
ended at local times `08:38:32` and `08:42:10` with:

```text
FATAL:third_party/blink/renderer/core/layout/inline/line_breaker.cc:4102
DCHECK failed: item_result->EndOffset() <= item_result_before.EndOffset()
```

The renderer then received signal 6. The crash occurred before a
renderer-facing diagnostic could be returned. The agent-browser tab record
remained `ready` and no target-crash incident was emitted.

The promoted artifact is:

`/home/ecochran76/workspace.local/chromium/artifacts/chromium-stealthcdp/150.0.7835.0+stealthcdp.3676a7503929/manifest.json`

Pinned identity:

- artifact: `150.0.7835.0+stealthcdp.3676a7503929`
- browser: `Chromium 150.0.7835.0`
- source SHA: `24ecda02e97db6fa730a7ccf8747776a4d21e4b9`
- upstream revision: `d421c3af8268e2e6227b7fe4461183e69b64bc61`
- patchset SHA: `3676a7503929fc2ff1ce0227c03c48aeaa4b1bae`
- executable SHA256:
  `aebeac48273efa3a2767763cf0694cfa8f1be52c91b7fbafff0d4698a993ffce`

The failing pinned source is:

`/home/ecochran76/workspace.local/chromium/src/third_party/blink/renderer/core/layout/inline/line_breaker.cc`

At the old failure site, `BreakText()` is immediately followed by:

```cpp
DCHECK_LE(item_result->EndOffset(), item_result_before.EndOffset());
```

The generated toolchain includes both `-DNDEBUG` and
`-DDCHECK_ALWAYS_ON=1`. Generated build metadata also reports
`is_official_build=false` and `dcheck_always_on=true`.

The applied custom patch is:

`/home/ecochran76/workspace.local/chromium/artifacts/chromium-stealthcdp/150.0.7835.0+stealthcdp.3676a7503929/patches/0001-Make-navigator.webdriver-non-advertising.patch`

It changes only
`third_party/blink/renderer/core/frame/navigator.cc`. It does not touch inline
layout, shaping, line breaking, fonts, or build DCHECK policy.

Newer official Chromium source for the same block is available at:

<https://chromium.googlesource.com/chromium/src/third_party/+/96119100e5da49d2272838031187f113b0d231cd/blink/renderer/core/layout/inline/line_breaker.cc>

It documents that a larger end offset is possible, removes the fatal ordering
assertion, and restores `item_result_before` when the shorter break attempt
fails. The exact upstream change list that introduced this behavior has not
yet been identified.

## Controls and what they prove

| Control | Result | Interpretation |
|---|---|---|
| Retained profile, Facebook home | Rendered | The profile and Facebook session are not globally unusable |
| Retained profile, authenticated posts search | Renderer abort | Reproduces the Blink condition |
| Retained profile, filtered authenticated posts search | Renderer abort | Search filter is not required |
| Clean profile, same custom build | Unauthenticated `Not Found`; evaluation succeeded | The browser can run, but this does not test authenticated search DOM |

There is no equivalent authenticated clean-profile control yet. Do not claim
profile exoneration from the clean-profile result alone.

## Separate agent-browser defects

The Chromium repair does not close the agent-browser work.

### Renderer crash propagation

The service did not convert the target crash or detach into a terminal tab
state. The tab stayed `ready`, and no incident connected the renderer abort to
the command. The repair should observe the relevant CDP target-crash and
detach signals, classify the tab as crashed or faulted, retain stderr and
process identity, and make the command fail with a typed diagnostic.

### Profile and session routing

After retained PID `83786` and port `39488` disappeared, a nominal inventory
request generated launch job `r804045` and attached
`session:last30days-facebook` to already-live default-profile PID `65800` on
port `38216`. Missing caller labels prevented safe attribution.

A requested runtime profile must fail closed when the detected browser or
attached session has a different profile identity. Read-oriented inventory
must not silently launch a browser or cross-attach a named session to the
default profile.

See the prior related notes rather than reconstructing their history here:

- `docs/dev/notes/2026-08-09-facebook-search-target-cdp-runtime-stall.md`
- `docs/dev/notes/2026-08-10-last30days-stale-runtime-pid-lock-handoff.md`

The downstream evidence ledger is:

`/home/ecochran76/workspace.local/last30days-skill/docs/dev/plans/0046-2026-08-11-facebook-retained-browser-runtime-recovery.md`

## Recommended next packet

### Chromium lane

1. Identify the upstream Chromium change list that replaced the old
   `DCHECK_LE` behavior.
2. Prefer a newer pinned Chromium revision containing the upstream handling.
   If a revision update is too broad, backport the exact upstream change with
   its tests. Do not simply delete the assertion without preserving the
   restore behavior and adding a regression.
3. Build a disposable candidate that retains DCHECKs. A build with DCHECKs
   disabled is useful as a comparison, but it is not sufficient acceptance
   evidence because it can conceal the invariant instead of proving the newer
   behavior.
4. Run the existing artifact smoke, then a bounded browser comparison. An
   authenticated comparison requires an operator-seeded disposable profile or
   explicit authority to copy private state. Do not copy the retained profile
   by default.

### agent-browser lane

1. Add a focused test that injects a CDP target crash during a command and
   proves terminal lifecycle, typed failure, incident emission, and retained
   browser and target identity.
2. Add a profile mismatch test proving that a named profile session cannot
   attach to a default-profile browser.
3. Add a no-launch inventory test for missing or stale retained ownership.
4. Require caller labels on any request capable of launch, attach, restart, or
   cleanup effects.
5. Install only after focused tests and the repository validation gate pass.

Suggested skills for the next agent:

- `codebase-investigator` or the repository CodeGraph tools for crash and
  routing flow discovery
- `diagnosing-bugs` for the cross-layer failure packet
- `tdd` for the target-crash and profile-mismatch regressions
- `agent-browser` for governed runtime validation
- `graphiti-discovery` for advisory recall before changing established
  lifecycle semantics

## Acceptance criteria

The packet is acceptable only when all of the following are true:

- the DCHECK-enabled candidate handles the authenticated search workload
  without the old `line_breaker.cc` abort, or returns a typed crash diagnostic
  if a different renderer failure occurs;
- a renderer crash cannot leave the tab lifecycle `ready`;
- the emitted incident binds command, caller, session, requested profile,
  detected profile, PID, endpoint, target ID, and stderr path;
- a named profile never attaches to a browser whose detected profile differs;
- inventory cannot launch or cross-attach as a hidden side effect;
- home-page and non-Facebook controls still pass; and
- only after those checks does last30days receive one bounded Facebook tick
  retry and verify the exact cache artifact.

## Hard stops

- Do not clear, delete, copy, migrate, or reconstruct the retained Facebook
  profile without explicit authority.
- Do not restart or close unrelated browsers or use a broad close operation.
- Do not treat a disabled DCHECK as proof that the underlying condition is
  repaired.
- Do not claim the profile is corrupt or clean without an equivalent
  authenticated control.
- Do not claim the downstream Facebook tick repaired until its cache evidence
  is present and verified.

## Handoff state

The old retained browser PID and endpoint are no longer live. No new Facebook
browser was launched for this handoff. The downstream tick remains unaccepted,
and this note authorizes no live retry.
