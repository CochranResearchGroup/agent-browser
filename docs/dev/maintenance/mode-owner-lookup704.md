# External browser mode owner routing

Product lane: PL-BUGFIX
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/704
Branch: fix/mode-owner-lookup704
Base: d0785fb8634c81309f16092990c04038836d2636
State: INSTALLED; owner routing qualified; continuation blocked by707

The external browser mode dialog rejects an existing task with HTTP503
`browser_command_owner_unavailable`. The same exact owner answers read-only
mode inspection through authenticated MCP. Dashboard routing incorrectly
requires a legacy per-session HTTP stream rather than the existing authenticated
owner command socket. No mode conversion has yet been dispatched.

Reuse the clean canonical primary; sole writer owns the dashboard owner routing
and regression. Preserve foreign mode/control lanes, the observer and original
uncertain launch requests. Do not acquire a daemon, create a stream or borrow
another owner to repair this request. Keep exact task, browser and profile scope.

Add a real authenticated dashboard-listener regression with an existing owner
socket and no HTTP stream; observe RED before the repair. Cover authentication,
exact body forwarding and absence of backend fallback. Run focused Rust tests,
Rustfmt and strict workspace Clippy locally. Continue the authorized review,
merge and preserving deployment, then drive both mode directions through the
external UI on supported stable Chrome and Chromium. Exclude Chrome for Testing.
Retain logical identity and synthetic storage, prove actual mode and continued
automation, and close only exact test targets with process/resource readback.
Source checks alone do not qualify the live toggle.

## Execution bound

Continue under the existing verify, review, merge and deploy authority. This
repair uses one source regression cycle and one sealed production candidate
qualification/deployment cycle before the external round-trip milestone.
Reassess after 30 minutes without outcome progress; do not repeat a failed
production deployment to discover fixture setup. The current continuation has
a two-hour ceiling including qualification and live cleanup. Preserve incomplete
acceptance explicitly if an external dependency prevents the round trip.

## Regression evidence

The real authenticated dashboard listener test failed RED in4.05 seconds with
HTTP503 instead of200 for a live authenticated owner socket without a stream.
The real ingress TCP test failed RED in2.21 seconds with
`mutation_outcome_unknown` before a2.1-second owner acknowledgement. Neither
fixture launches a browser or replays the delivered command to a fallback.

The mounted dialog fixture failed RED waiting for a17-second owner response
because its action deadline was15 seconds. With a95-second mode action deadline,
`pnpm test:browser-operation-reload` passes19 cases, including slow owner reply,
reload, lost response, original operation reuse, cancellation and refusal.
`pnpm exec node scripts/test-browser-mode-inspection.cjs` passes, as do
`pnpm build:dashboard`, policy wiring11 cases and documentation links. Static
export emits its existing rewrite advisory. Rust qualification is waiting on
normal host-memory admission and remains pending. The exact command ingress
POST now allows100 seconds around the existing90-second owner relay; other
mutation paths and no-replay semantics stay bounded by their prior contracts.

This is repair of the existing mode workflow, not a new flag or feature. The
remaining live criterion is still an actual supported-browser round trip,
including storage continuity and exact cleanup through the external UI.

## Source qualification

Local focused validation passed:70 dashboard Rust tests,45 ingress Rust tests,
19 mounted operation-recovery browser cases, mounted mode inspection, dashboard
production build, Rustfmt check, strict workspace Clippy, policy wiring11 cases
and documentation links. Cargo used the normal WSL admission wrapper, eight
jobs and disposable test homes; host-memory admission delayed execution.
Hosted GitHub CI was not dispatched. The fixture uses official installed stable
Chrome for rendering and synthetic HTTP owners; it is not live toggle proof.

Serial review applies both axes to the complete repair: Standards checks the
repository ownership, authentication, no-replay and code-style contracts; Spec
checks the existing-owner defect, exact identity forwarding and bounded response
path against issue704 and the user workflow. Neither axis has an accepted
blocking finding. Live external round trips remain the installation acceptance
gate. The source repair does not claim those tests have already passed.

## Installed checkpoint and live acceptance

PR [706](https://github.com/CochranResearchGroup/agent-browser/pull/706) merged as
`e83c07434b237298b27f187d4785813f9615fd7f`. One clean, production-shaped release
build sealed binary
`478f219f6756e770961d77cbac5d60fa59869e2bae327e97febe061dc2749f68`.
The build took 2306.542 seconds; the exact-binary workstation fixture passed in
50.278 seconds. One preserving installation accepted transaction
`upgrade-94e488bb-c2dc-4f06-8c16-af4e35214c72` in 132.457 seconds and selected
`0.28.0-478f219f6756-503614227e66`. The installed command, runtime host, dashboard
backend and stable ingress all match the sealed digest. No formal release or
hosted CI was performed.

The authenticated external workspace catalog returned200 in 9.436 seconds. Its
exact, explicitly selected official stable Google Chrome launch returned200 and
ready in 32.072 seconds. The loaded Chrome157.0.8103.78 digest was
`91e519ce83eac12740d54398364583b3131116055eca863d0a240d6062e84053`,
not the alternate same-version build. External mode inspection returned200 in
1.961 seconds and preparation in 6.154 seconds, proving the owner-lookup repair.

The first managed observer's relaunch and guarded focus requests were refused
with `browser_native_target_changed`. No process change was inferred from those
responses. An independent Playwright browser using the same external HTTPS UI
completed the retained operation's single guarded graceful-close retry. Shared
state changes from the managed observer are a supported hypothesis for the
refusal, not a proven root cause. The owner guard remained intact throughout.

The existing opaque Native handoff connected over WebSocket and its CDP-on
dialog named the exact single affected task. That visible dialog completed the
return operation. The two owner journals reached Completed. Fresh OS and owner
readbacks proved three distinct Chrome process roots, debugging flags and CDP
endpoints on, off, then on, and unchanged logical task, browser, profile,
executable bytes and profile data path. No DOM/CDP actions were performed while
CDP-free. The Native connection observation is distinct from an explicit
`operatorVisible.state=ready` receipt, which this batch did not capture.

| Acceptance | Result |
| --- | --- |
| External operation-owner lookup | PASS on the installed candidate |
| Official stable Chrome physical mode round trip | PASS, including retained-operation recovery |
| Chrome logical/profile identity | PASS |
| Profile storage marker | PASS after explicitly reopening its original origin |
| Automatic page/task navigation continuation | FAIL: returned task is `about:blank` |
| Supported Chromium round trip | NOT RUN |
| Chrome for Testing qualification | EXCLUDED |

The first post-return storage read raised SecurityError because the current
document was `about:blank`. A separate URL read confirmed that value; the task
listed one tab. Explicit navigation to the original public Example Domain URL
then recovered the original synthetic localStorage marker. This manual resume
is recorded separately and does not turn navigation continuity into a pass.
[707](https://github.com/CochranResearchGroup/agent-browser/issues/707) owns the
remaining continuation defect and the supported-browser acceptance follow-up.
This batch does not qualify the complete toggle or profile-cloning workflow.

The initial doctor exited1 for a runtime-monitor backoff. The normal interlock
timer later completed reconciliation and the monitor became healthy, without
manual effects or weakened checks. A second doctor still exited1 because its
no-launch service-status probe timed out. The selected generation and accepted
installation are proven; a fully green doctor is not claimed at this checkpoint.

## Cleanup and retained custody

The failed disposable observer was closed through its owner. Fresh process and
registry checks, followed by scheduled cleanup, proved its root, profile and data
directory absent. The independent browsers were closed through Playwright; their
remaining task-owned TTY controller processes were positively identified and
terminated only after browser closure. The final Chrome test's synthetic marker
was removed, its exact browser was closed through the owning API, and its root,
recorded process family, task and browser records are absent. Its disposable
profile follows the configured five-minute scheduled cleanup; final scheduled cleanup also removed its profile record and directory. There was no broad process cleanup, forced mode
conversion, manual runtime-state edit or replay of original uncertain launches.
The original observer retained its exact process identity. Raw runtime, browser,
authentication and QA evidence stays in user-scoped storage outside Git.

The bounded production cycle is complete. Further source repair or another
production candidate is outside this cycle; the remaining acceptance is
explicitly blocked on707 rather than relabeled successful.

## Final closeout readbacks

The final quiet `agent-browser install doctor --json` subprocess exited0. The
runtime monitor was healthy and the only remaining issue was the advisory
workspace-candidate difference; the installed selected bytes remain authoritative.
The final Chrome root and recorded process family, task, browser, disposable
profile and data directory are absent. A fresh exact-profile process census is
empty. The independent observer controller PIDs are absent, and the original
observer's process/start identity is unchanged. The sealed candidate and
canonical checkout are retained; foreign worktrees and original uncertain
requests are preserved.

The earlier managed observer's self-inspection script returned a request-timeout
PRECONDITION after deployment, not GREEN. A self-targeted HTTP owner request can
queue behind the same lane's evaluating command; that script is not a valid
independent post-repair acceptance client. The real authenticated listener
regression and the independent external UI's exact-target owner inspection,
preparation and dispatch provide the qualified GREEN evidence. Future live
self-tests should use a separate client and retain the original failing requests.

No additional source repair or production cycle was started after the remaining
continuation defect was demonstrated. The complete Chrome/Chromium continuation
matrix remains unaccepted under707.
