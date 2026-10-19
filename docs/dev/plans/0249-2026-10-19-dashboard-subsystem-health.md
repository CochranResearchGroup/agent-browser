# P249: Dashboard subsystem health

Status: OPEN
Lane: P249
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#317
Branch: fix/dashboard-subsystem-health
Owner: ecochran76

## Outcome and scope

Operators can inspect subsystem health from a compact SVG toolbar control and an accessible side drawer. Inventory, runtime diagnostic readiness, Native slot readiness and selected viewer page failure stay separate. Unknown or stale evidence never appears healthy. No recurring diagnostic requests, automatic repair, authentication changes or Docker lifecycle effects.

## Delivery and ownership

Reuse the clean dashboard primary checkout after fresh origin/main reconciliation. P248/UX301 are integrated; retained candidate artifacts stay pinned. Foreign planning and enrollment checkouts remain untouched. Primary writer owns the new health projection and drawer, compact toolbar integration, and feature guidance. No shared source writer is active for these surfaces.

## Acceptance and validation

Pure projection tests cover mixed readiness, stale evidence, missing schema and unreachable checks. Mounted browser checks cover accessible opening/closing and focus, health states, bounded calls and no background diagnostic polling. Run dashboard type check/build and existing dashboard action guards. Source qualification does not imply installed production UI. Record exact validation and remaining gates before closeout.

## Source qualification

The real compact toolbar mounts the SVG health indicator and right-side Radix dialog. Browser inventory and Native census errors update locally. Selected managed and observed Native viewers contribute their actual connection state, while an iframe alone remains unverified. Explicit runtime and Native slot reads have an eight-second shared deadline, no redirect following, and no automatic refresh. Dated evidence expires after thirty seconds. Runtime findings expose only bounded code identifiers, excluding raw private response messages.

Passed: projection regressions; real compact-workspace mounted fixture using the production stylesheet, including Native failure/recovery, selected-view failure, Escape/focus restoration, desktop/mobile placement, timeout clearing of prior healthy evidence and zero cold-open/background health calls; dashboard type check and production build; docs production build; inspector-action, browser-table, workspace-navigator and remote-view guidance checks; Rust format and strict workspace Clippy for the help-only Rust edit; selected P249 planning audit and changed-document link check.

Initial mounted checks exposed an actual centering conflict from the shared Dialog styles; explicit drawer insets/translation repair it. Earlier fixture role/focus timing assertions were corrected to handle Radix modal accessibility and deferred focus restoration. The older `test-compact-native-workspace.cjs` fails its historical explicit `view_focus` assertion at line 458 both here and in an isolated untouched baseline archive at `39547e76e9b436a260e40d2e82531c3bca658b06`. Existing direct handoff behavior is preserved; that baseline failure is not claimed as passing. The new fixture mounts the actual compact workspace and qualifies the changed toolbar and drawer.

Standards review: no new service action, viewer, access ceremony, auth policy, runtime repair, recurring diagnostic poll or retained task mutation. Spec review: subsystem states remain separately evidenced; stale, timeout, malformed and partial observations stay non-green; current Native viewer failure overrides successful runtime checks. Production runtime is unchanged by source qualification.
