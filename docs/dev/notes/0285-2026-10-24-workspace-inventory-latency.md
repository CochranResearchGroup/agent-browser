# P285 workspace inventory latency

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P285 / CochranResearchGroup/agent-browser#393
Related lanes: P283, P284
Work item: CochranResearchGroup/agent-browser#393

## Reproduction and cause

The current production-shaped bf263f69342b candidate reproduces the issue in an isolated i393 namespace with zero browsers/tabs. A direct generation-backend GET with `projection=workspace` takes2.73s in the minimised single-read case. Five repeated empty reads during host compilation pressure have median5.156s/max6.880s. The response is14,632 bytes and includes full diagnostic sections. The CLI fallback takes3.50s and returns the same broad shape.

The stream listener already honours the narrow projection. The generation-specific dashboard handler instead enters transport discovery and may fall back to `service status`, dropping the workspace projection. This disproves the earlier assumption that the measured backend request necessarily reached the narrow producer. The failing regression goes through the real authenticated Service handler boundary with a diagnostic backend that must remain untouched; it returns the broad diagnostic envelope before the repair.

## Repair and qualified development candidate

Both listeners now share the existing read-only owner workspace producer. The generation handler serves it after authentication and before diagnostic discovery/cache/fallback. Ordinary full diagnostics retain their transport and cache. Owner snapshot, presence joins, observation clocks/expiry, polling cadence and unknown/absent/live semantics are unchanged. Owner-read errors remain failures rather than inventing an empty workspace.

Optimized development binary SHA-256 `cb4848c136f20924a7ee439d3b30138080c10e641ead7949240a205e1f2fa28a` is qualified against the recorded patch over9e0a01b8a. Five empty reads have median3.358ms/max6.393ms, narrow payload334 bytes and five distinct observation timestamps. With one synthetic about:blank browser and tab, five reads have median12.867ms/max19.958ms and truthful1/1 counts. The higher-level disposable open/continue/close workflow preserves the same browser/tab and passes.

Five fresh development UI contexts populate the rail on their first owner response, with no page errors. Median rail1.591s/max3.324s; median shell1.577s. The first context spends3.157s before shell readiness, so this is not evidence that all cold-shell latency is fixed. Inventory appears within257.180ms of shell readiness in every sample. Native first pixels and private-account workflows are not qualified.

All253 stream tests pass with two existing ignored cases; the regression was captured failing first. Workspace strict Clippy, format, API/MCP parity, handoff guidance and docs build pass. All five required user guidance surfaces are updated; the agent entry remains9,998 bytes. The selector's unrelated workstation/provider harness and shared-skill synchronization suggestions are not claimed: no installer/provider code changes in this slice and experimental guidance stays isolated.

The synthetic browser's original PID is independently absent, as are all16 observed UI-probe process generations. Development publication preserves production identity. Private timing/process receipts remain user-scoped. The temporary namespace is stopped/disabled and an independent scan finds zero remaining namespace processes. Source integration remains a closeout obligation. Source qualification does not claim installed production acceptance.

## Integration boundary

P284/PR406 merged as442c3387d during qualification. Reconcile its installer and policy changes without changing the P285 routing hunk; preserve P283's active profile/store/observation ownership and retained candidates. The standing dashboard implementation/deployment instruction covers the next qualified preserve-mode publication. Recheck current policy and custody, build one production-shaped combined artifact after integration, and verify exact installed/public behavior. No formal release, private-account operation, raw binary replacement or Native provider/ingress reconfiguration is part of publication.

P284 integration retains both catalog intents. Rust routing hunks are unchanged; only the entry-point wording is shortened to keep the merged guidance at9,994 bytes. Combined validation reuses the unchanged253-case stream packet and P284's222-case workstation packet, with integrated strict Rust and focused workspace checks plus the exact combined installer fixture before publication.

Integrated checks after P284 reconciliation pass: strict workspace Clippy, format, both actual workspace route tests, docs build and guidance. The two workspace tests finish in0.04s and prove neither listener enters diagnostic transport. The merged entry guidance is9,994 bytes.

## Installed checkpoint

PR410 merged as61aa1c3ea666bae228b6d4372bf8ea08ed87a248. The production-shaped candidate `candidate-19f4d83e701f9cf0-4c8131146eb95ca5` has binary SHA-256 `4c8131146eb95ca5701e45e1803efa94caf0e508eb30893986d87ac9dbc96807`. All562 executable inputs match freshly fetched canonical ae6adde7fdf05752f40623eb37af35c799d998f9 (481 tracked,81 generated). The exact source-free combined installer fixture passes and its two owned hosts leave no residue. Supported preserve-mode install transaction `upgrade-ad756107-d48c-4025-b06e-e69f2249908d` exits0, complete/ready. Public manifest selects the candidate digest and80 assets. All11 pre-install browser PID/start generations are preserved through installation.

Five paired direct-backend workspace reads improve from predecessor median487.841ms/max533.133ms to installed median32.755ms/max41.949ms with the same22-browser/47-tab inventory. All14 startup JS/CSS assets pass gzip negotiation, identity decoding, cache and content-length checks; wire reduction is62.77%. Install doctor exits0 while retaining operator-journey and principal-binding advisories; these are not full browser-workflow acceptance.

The first five public fresh contexts have median rail821.788ms, but one takes4981.246ms and needs the second poll despite a prompt first owner response. Five additional instrumented fresh contexts populate in826.103–1031.732ms with no page or serialization errors, and the first owner response already reports live tabs. This follow-up does not reproduce or explain the earlier tail. Keep #393 open for that observation rather than declaring all cold opens fixed. Native census redirects to authentication in these fresh app-authenticated contexts, so Native first pixels remain unqualified.

The production disposable about:blank canary fails during open with `remote_view_pool_acquisition_readback_required` before returning a task/browser ID. One supported resume with the exact same task key fails identically. Read-only inventory works; one pending acquisition remains without a creation timestamp or proven origin. Do not infer no Native effect from the missing browser ID. Preserve its idempotency key and history; no replacement request or provider reconfiguration was attempted. #412 tracks exact recovery and actionable diagnostics separately. Development open/continue/close remains passed, but production end-to-end browser acceptance is incomplete.

Private raw timing, process, acquisition and authentication receipts remain user-scoped. Both public probe browser trees are closed; the later probe's16 observed process generations are independently absent. The i393 namespace remains stopped. Retain this primary checkout and its sealed candidates; no worktree removal is requested. The next bounded packet is to distinguish the first-response presence/render boundary for the intermittent rail tail, without changing clocks or masking unknown state.
