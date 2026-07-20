# Clipboard Read And Target Recovery Performance

Date: 2026-07-19

## Summary

A retained remote-headed LinkedIn smoke exposed three related agent-browser
problems:

- `clipboard read` can wait for the full CDP timeout when
  `navigator.clipboard.readText()` never resolves;
- later `Runtime.evaluate` calls against the selected target also failed to
  complete after the unresolved evaluation;
- multi-command page interactions incur enough repeated setup latency that a
  small menu workflow becomes slow and difficult to recover.

The target site successfully produced a canonical post URL when its
`Clipboard.prototype.writeText` call was intercepted. In that historical run,
this distinguished the site's successful write call from the later clipboard
read failure. It does not by itself prove the cause of the target's subsequent
evaluation failures.

This note does not contain the copied URL, browser cookies, authentication
state, private page text, or other browser-profile data.

## Runtime Context

The smoke used one retained service browser with:

- a named runtime profile and session;
- a remote-headed Chromium build;
- an RDP/Guacamole operator view;
- one Facebook tab and one selected LinkedIn tab.

The browser and operator route were healthy before the clipboard test. The
LinkedIn search page was loaded, authenticated, and responsive to ordinary
DOM evaluation and click commands.

## Reproduction

The site workflow was:

1. Open a post control menu.
2. Select `Copy link to post`.
3. Run `agent-browser clipboard read` against the same retained session.

Observed result:

```text
CDP command timed out: Runtime.evaluate
```

The command took approximately the generic 30-second CDP timeout before
returning the error.

The current implementation in `cli/src/native/actions.rs` handles clipboard
reads by directly awaiting:

```javascript
navigator.clipboard.readText()
```

There is no clipboard-specific promise timeout, permission preflight, or
target-health recovery around that evaluation.

## Historical Runtime Observations

The observations below came from one retained authenticated runtime. They are
useful incident evidence, but the original run did not produce a durable,
privacy-safe command artifact. Treat timings and causal explanations as
historical narrative until a deterministic fixture or a new redacted live
artifact reproduces them. Plan 0076 defines the required evidence shape and
regression coverage.

### 1. Copy succeeded but read did not

The menu item was visible and its click handler ran. Temporarily intercepting
`Clipboard.prototype.writeText` captured one canonical LinkedIn post URL.
The URL was reduced to a redacted shape for this note:

```text
https://www.linkedin.com/posts/<post-slug>-<activity-id>/
```

This distinguishes a successful site copy action from a failed clipboard read.

### 2. Permission grant did not make read resolve

A CDP `Browser.grantPermissions` call for LinkedIn completed with
`clipboardReadWrite` and `clipboardSanitizedWrite`. A following
`navigator.clipboard.readText()` evaluation still did not resolve.

The failure should therefore not be reported only as a missing-permission
problem. Clipboard backend behavior and unresolved promise handling also need
coverage.

### 3. X11 had no readable clipboard owner

An X11 clipboard read on the retained browser display did not return data and
timed out. The visible display included a Chromium clipboard window, but the
X11 selection path was not a usable fallback for this browser copy action.

### 4. The selected target was evaluation-unhealthy after the timeout

After the unresolved clipboard read, later `Runtime.evaluate` calls against
the same target did not return useful responses. Navigating the existing tab
to the same URL did not restore evaluation health. The run established this
ordering, but did not isolate whether the unresolved promise, renderer state,
CDP session state, or another target condition caused the later failures.

Closing only the affected LinkedIn tab and opening a replacement tab in the
same retained browser restored normal evaluation. Authentication remained in
the managed profile, and the browser returned to one Facebook tab plus one
selected LinkedIn tab.

### 5. Semantic menu lookup was inconsistent

DOM inspection showed three visible elements with `role="menuitem"`, including
`Copy link to post`. A role-and-name lookup reported that the element was not
found, while a direct selector/DOM path could invoke the same menu action. The
original run did not preserve the privacy-safe DOM and accessibility-tree shape
needed to distinguish `aria-labelledby`, hidden descendant text, iframe,
shadow-root, or other accessible-name differences.

This is separate from clipboard retrieval and should have its own locator
regression test.

### 6. Repeated command setup dominated the workflow

During the smoke, ordinary page commands generally took about seven seconds
of wall-clock time each. A service-status read was materially faster. The
smoke did not isolate whether the page-command cost came from queue wait,
target resolution, CDP attachment, action execution, or response assembly.

The service status payload also retained several invalid closed tab handles
with `staleReason="tab_closed"` after the visible tab set had been compacted to
two live tabs. Those rows are useful history, but unbounded inclusion in the
ordinary status payload increases output size and target-selection noise.

## Product Follow-Up

### P0: Bound clipboard promises and recover target health

- Give clipboard reads an operation-specific timeout shorter than the generic
  CDP command timeout. Implement the deadline inside the CDP command lifecycle
  so timeout and cancellation always remove the pending response entry; do not
  wrap `BrowserManager::evaluate` in an outer timeout that can bypass cleanup.
- Return a typed clipboard error that distinguishes permission denial,
  unresolved promise, unavailable backend, recovery failure, and CDP failure.
  A read that resolves to an empty string is successful clipboard output.
- On timeout, use a recovery primitive that a regression test proves restores
  a normal evaluation on the same retained browser. Do not assume that
  detaching and reattaching a CDP page session cancels renderer work or repairs
  the execution context.
- If context repair cannot be proven, return an explicit recommendation to
  replace the affected tab instead of silently continuing on it.

### P1: Capture clipboard writes as an action result

Add a bounded action mode such as `click` with clipboard capture that:

1. temporarily wraps `Clipboard.prototype.writeText` in the selected target;
2. performs the requested click;
3. returns the written text as structured action output;
4. restores the original clipboard method in a `finally` path;
5. caps captured text length and preserves existing redaction policy.

This avoids a system clipboard round trip for sites whose explicit Copy action
already supplies the desired value to the browser Clipboard API.

### P1: Expose page-command latency components

Record bounded timing fields for:

- service queue wait;
- browser and target resolution;
- CDP session acquisition or reuse;
- browser action execution;
- response serialization.

The timings should be available in JSON output and service traces without
requiring raw debug logs.

### P1: Preserve one target across dependent batch steps

For a batch that opens a menu, waits, selects an item, and captures a result:

- resolve the browser and target once while steps preserve target identity;
- retain one queue lease and CDP page session for the dependent commands;
- refresh element references only when the DOM changes;
- revalidate after navigation, tab creation, close, switch, or another
  target-changing step;
- stop on the first target-health failure when batch bail behavior is enabled.

This should remove repeated target-resolution work without weakening service
serialization or site-specific pacing.

### P2: Repair role-based menu lookup

- Reconstruct an accessible-name fixture that differs from raw `aria-label`
  and `textContent`, using the observed failure shape when a privacy-safe
  reproduction becomes available.
- Cover `aria-labelledby`, hidden descendant text, dynamically mounted menus,
  iframe context, and supported shadow-root behavior. Keep an ordinary portal
  menu as baseline coverage, not as the incident regression by itself.
- Ensure failed and successful `find` actions always emit one structured JSON
  response.

### P2: Compact stale tab history in ordinary status output

- Keep live tab handles prominent and bounded.
- Move older closed handles to a capped history or an explicit verbose view.
- Preserve lifecycle evidence required for diagnostics and audit trails.
- Prove that compaction cannot make a stale handle appear live or change tab
  routing authority.

## Suggested Regression Tests

1. A clipboard-read promise that never resolves returns a typed error within
   the clipboard-specific timeout and leaves no pending CDP response entry.
2. A following CDP command succeeds without waiting for the generic timeout.
3. A normal evaluation succeeds immediately after that clipboard timeout on
   the same retained browser, or the action returns a typed replacement-tab
   requirement when same-target repair is not supported.
4. A clipboard read that resolves to `""` succeeds with empty text.
5. A copy-action capture returns a synthetic canonical URL and restores the
   original Clipboard API method.
6. A role locator resolves an element whose accessible name differs from its
   raw `aria-label` and `textContent`; an ordinary portal menu remains a
   separate baseline case.
7. A dependent batch resolves one target and one CDP page session while
   preserving command order and bail behavior.
8. A target-changing batch step forces target revalidation before the next
   dependent step.
9. Ordinary service status caps closed tab-handle history while a diagnostic
   or verbose surface can still retrieve the retained lifecycle evidence.

## Privacy-Safe Validation Artifact Template

Future deterministic or live validation should record the following without
private site or profile data:

```text
artifactId: <timestamp-or-stable-fixture-id>
browserMode: <headless-or-remote-headed>
profileClass: <temporary-fixture-or-retained-redacted>
commandShape: clipboard read
clipboardDeadlineMs: <bounded-value>
observedDurationMs: <measured-value>
clipboardOutcome: <success-empty|success-text-redacted|permission-denied|unresolved-promise|backend-unavailable|cdp-failure>
pendingCommandCleanup: <proved|not-proved>
targetHealthProbe: <normal-evaluation-success|normal-evaluation-failure>
recoveryAction: <none|execution-context-repair|replace-tab-required|replacement-tab-opened>
finalTopology: <same-browser-and-tab|same-browser-replacement-tab|browser-replaced>
privateDataRetained: false
```

For timing claims, preserve the command start and finish monotonic durations or
the structured timing output. For target recovery, preserve the redacted target
identity transition and the result of one normal post-timeout evaluation.

## Relationship To Existing Notes

`docs/dev/notes/2026-07-06-last30days-profile-routing-failure.md` covers
runtime-profile authority and wrong-profile routing. That issue has remediation
progress under Plan 0069. This note covers a different failure class:
clipboard promise handling, target health after CDP timeout, locator behavior,
and per-command performance.

## Recommended Next Step

Execute
`docs/dev/plans/0076-2026-07-19-clipboard-target-recovery-and-interaction-performance-remediation-plan.md`.
Start with the cancellation-safe CDP deadline tracer bullet, then complete the
remaining separately reviewable slices with focused tests and rollback.
