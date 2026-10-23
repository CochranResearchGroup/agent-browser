# P273: development host startup attribution

Status: CLOSED
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#356
Branch: fix/development-supervision-recovery-356
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

## Installed same-process supervision recovery, 2026-10-23

The user explicitly approved recovering the existing development host under its proper supervisor while preserving browsers and selected generation. This supersedes the earlier source-only no-adoption boundary for this exact development operation. The PR371 prevention repair remains canonical. Current main and all registered primaries are reconciled; this session reuses the retained dashboard primary, preserves the pinned P270 candidate and foreign work, and creates no auxiliary checkout.

Systemd255 process attachment and MAINPID acknowledgment are qualified first on a disposable process, including the installed NoNewPrivileges/private-mount sandbox and restoration to ordinary service settings without a process restart. The non-delegated test is a retained typed refusal. The first live helper stops before attachment because its sandbox cannot inspect the existing executable. The outside caller retains full UID/PID/start/executable/hash/environment checks; the sandboxed unit only acknowledges the already-confirmed attachment. An unsupported caller Python pidfd API causes a second pre-dispatch failure and notifier timeout. Fresh source-cgroup identity proves no migration in either failed preparation. The corrected system Python caller performs exactly one live attachment, with pidfd liveness and fresh identity readbacks. No uncertain effect is replayed. All failure and synthetic cleanup receipts remain in the user-scoped operation artifact.

The existing development host PID 1331, start token 2213, and exact 9c8edd62a16f executable remain unchanged. Only supervision changes from the dashboard cgroup to agent-browser-dev-runtime-host.service. The destination acknowledges that same MainPID. Its temporary adoption override is removed, and normal Type=simple, Delegate=no, KillMode=process and Restart=on-failure are restored without restarting the host. The retained process keeps its original privilege and mount context; restored unit sandbox settings apply on the next normal start. Adoption does not qualify fresh-start isolation. A persistent development-ingress drop-in applies only PR371's existing skip-bootstrap setting for future startup. The selected generation, application ownership, runtime identity files, original session IDs and all four tracked browser roots remain intact. No host/browser termination or executable publication occurs.

The existing development backend unit starts using the same selected generation. All three development units are active, with host 1331, ingress 816 and backend 968156. Three steady samples preserve the host kernel generation, all four browser roots, original sessions and production unit readbacks. The lane responds HTTP200. Development doctor passes 33 of 34 checks; its sole failure is development-skill source drift, positively present in the pre-recovery status. Guidance publication is outside this recovery and remains unchanged.

The logical operation p273-supervision-adoption is complete, with one live attachment, all synthetic processes absent and no adoption override left loaded. Its curated receipt and raw private evidence live under the user-scoped repairs/p273-development-host-startup/supervision-adoption directory. Browser content and account authentication were not inspected. This qualifies356's recovery and preservation outcome; an actual OS reboot is not part of this acceptance. P270/#355 latency and the wider dashboard objective remain open.
