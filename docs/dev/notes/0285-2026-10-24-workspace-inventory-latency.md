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

The synthetic browser's original PID is independently absent, as are all16 observed UI-probe process generations. Development publication preserves production identity. Private timing/process receipts remain user-scoped. Development namespace shutdown and source integration remain closeout obligations. Source qualification does not claim installed production acceptance.

## Integration boundary

P284/PR406 merged as442c3387d during qualification. Reconcile its installer and policy changes without changing the P285 routing hunk; preserve P283's active profile/store/observation ownership and retained candidates. The standing dashboard implementation/deployment instruction covers the next qualified preserve-mode publication. Recheck current policy and custody, build one production-shaped combined artifact after integration, and verify exact installed/public behavior. No formal release, private-account operation, raw binary replacement or Native provider/ingress reconfiguration is part of publication.
