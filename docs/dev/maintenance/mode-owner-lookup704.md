# External browser mode owner routing

Product lane: PL-BUGFIX
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/704
Branch: fix/mode-owner-lookup704
Base: d0785fb8634c81309f16092990c04038836d2636
State: OPEN

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
