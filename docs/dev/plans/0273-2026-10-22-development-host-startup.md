# P273: development host startup attribution

Status: OPEN
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#356
Branch: fix/development-host-startup-356
Owner: ecochran76

## Outcome

Qualify the development runtime-host startup failure, preserve selected generation9c8edd62a16f, and restore attributable startup without disturbing production browsers. Existing September socket-directory repair8164eac9 is already canonical; do not duplicate it or infer the current failure has that cause.

## Current state and custody

Read-only current unit readback: failed, exit1, PID0; journal has no entries. Development dashboard remains live. Source base141d17d6e in retained dashboard primary; other primaries are foreign MCP routing and resource-bounds. Foreign MCP work is reconciling an unrelated merge and uses P272; preserve its files. Root owns P273 and startup diagnosis. One independent read-only worker owns remaining355consumer-latency diagnosis through Git objects and existing receipts; no checkout, effects or edits. Source checkpoint and candidate5e74e/4748ee from P270 remain retained; no rebuild or overwrite is needed for diagnosis.

## Gates and bound

Start with the actual installed executable in the existing disposable supervisor no-launch fixture. Rank and test the missing-substrate, selected manifest/executable, and startup-environment hypotheses against its result. Preserve failed diagnostics. No browser/private-account/provider operation, production deployment, blanket cleanup or uncertain retries. Shared development mutation requires exact custody and before/after identity; do not substitute a new executable for selected generation. One bounded diagnostic and one source repair/retest cycle before reassessment, checkpoint within30minutes; no further workers. Meaningful focused tests, format and strict Clippy for Rust, patch hygiene and applicable docs gates. Source integration via PR; runtime proof reported separately.

## Qualified cause and bounded repair

The selected9c8edd62 executable passes the existing isolated supervisor reboot test. Actual retained manifest targets the same executable and generation. Read-only startup preflight reproduces a typed port_conflict on development lane4951. Kernel process1331 already owns that listener and matches the selected binary, development home, socket namespace and runtime ingress; its cgroup is the dashboard ingress unit, while the dedicated supervisor unit is failed/PID0. The root startup refusal is qualified; historical timing of the initial boot race is not recovered from the missing journal.

Generated development ingress units omit the existing AGENT_BROWSER_DASHBOARD_INGRESS_SKIP_SERVICE_BACKEND setting. The ingress server otherwise calls ensure_dashboard_service_backend and may spawn a host while the Type=simple supervisor is still verifying manifests and has not published its socket. Backend-only units already skip that bootstrap. Set the existing guard on generated ingress units only; preserve host/backend unit roles and readiness checks. Do not kill, stop, adopt or replace the current dashboard-owned host or its browser children.

An actual selected-binary regression drives generated ingress with a deliberately delayed supervisor and asserts no extra runtime process. The original unit rendering fails with a competing host; exact cleanup qualifies two processes. With the guard, ingress remains a client and the selected executable starts and serves the synthetic supervisor lane; no Chrome launches. Both source/test verdicts and runtime cleanup are distinct from installed unit repair. Initial fixture cleanup waited on an already-signaled child and yielded an unsettled top-level await; remove that unnecessary event wait after kernel-qualified cleanup, preserve the failed harness log, then confirm the genuine red before the repair. The live installation remains unchanged, and356 stays open until supported ownership recovery is qualified without losing browser work.

The independent355worker finds no qualified consumer shortcut that saves the worst37.35ms deficit. Three fresh Native status composites straddle ownership effects, so stale reuse would remove a fence. The next useful probe separates assignment-before, desktop-control RPC, assignment-after and client queue wait with parent IDs; do not rerun the full workload to seek a favorable sample. No consumer files or auxiliary checkouts were created.

Read-only installed recover-host dry-run refuses takeover with other-supervisor and live-owned-browser blockers. No apply, resume, unit restart or runtime state reset is attempted. The existing development lane still responds HTTP200 and the original selected-generation host remains live. This is the exact remaining installed-recovery boundary, not a request to clear or kill browser work.

P270 source branch is integrated through PR369; its catalog custody is reconciled to INTEGRATED while the wider plan remains OPEN. This session reuses the same retained physical primary for P273, preserving the prior executable and remote source refs without overwriting its pinned CI candidate. No primary or auxiliary checkout is added.

Validation: selected executable reboot smoke passes; boot race regression is red before the guard and green afterward, including later same-PID readiness and independently confirmed absence of both fixture generations. Existing development-runtime fixtures, policy wiring and documentation links pass. Cargo format and strict Clippy wait for the unchanged resource-admission reserve while a foreign release build is active; no thresholds or foreign processes are changed.

Final source qualification: resource admission cleared without changing limits or foreign work. Cargo format and strict integrated workspace Clippy pass (24.51seconds); development fixtures and policy/link checks pass. Current main MCP routing changes are integrated, preserving both P272 and P273 catalog/index entries; no source conflict. The exact tested boot renderer and fixture are unchanged. PR371 lands source only; selected installed9c8edd62 generation and live host remain unchanged.
