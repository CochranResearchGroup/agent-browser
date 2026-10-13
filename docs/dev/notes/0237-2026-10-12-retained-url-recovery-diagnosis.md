# Retained native handoff URL recovery diagnosis

Product lane: PL-PLATFORM
Owning plan: [P237](../plans/0237-2026-10-11-native-dashboard-handoffs.md)
Work item: [261](https://github.com/CochranResearchGroup/agent-browser/issues/261)
State: peer-preserving and last-browser synthetic recovery qualified; physical and wider acceptance open

## Observed workflow and control

The synthetic HTTPS workflow opens the actual opaque URL, follows the production redirect, renders the native viewer and records private successful use. After the owner expires the addressed tab under its configured tab policy, the link remains valid. Opening that same URL returns 410. This fails both when the last browser ends and when a live peer keeps the browser and native assignment available. The peer case minimises the failure without browser-pool churn.

A diagnostic control explicitly invokes the existing owner recovery command and then prepares native observation using the exact tab and target identities returned by recovery. The same URL then returns a second 303 and renders native frames. The peer remains present. The composed control passes in 26.34 seconds, with zero Dashboard-login requests. This confirms that the public workflow lacks a continuation to owner recovery; it does not qualify a product repair or second-use persistence.

The pre-auth public locator is deliberately read-only. Remote View authenticates ingress before admitting native viewer operations, but its current consumer bridge addresses Remote View's control owner, not an Agent Browser task-resume callback. The authenticated Dashboard API already reaches browser-owner recovery. Requiring that separate Dashboard authentication would not satisfy the native handoff contract.

## Attribution corrections

Earlier owner controls omitted mandatory `expectedTabId` and `expectedTargetId` in native-use preparation. The handler rejected those requests before provider effects. Their `remote_view_human_use_target_changed` result does not establish a stale native assignment. The corrected control uses identities from the actual recovery response without weakening the product fence.

An earlier last-browser control also encountered a fixed synthetic window PID mismatch after a second synthetic browser launch. That is a fixture qualification failure. The independent ordinary-path 410 and preserved-peer 410 remain product RED evidence.

## Reproducibility and proof limits

The opt-in `tests/native_handoff_public_tls_driver.py` in the owned Remote View source accepts `--recovery`; the temporary `--owner-recovery-probe` flag selects the diagnostic control. It drives the actual public handler, SQLite-backed owner and native gateway through a task-owned HTTPS relay and disposable Chrome process. Physical browser/window effects, display frames and authentication are synthetic. No installed or production effect is qualified.

The owner-probe flag has since been removed from current source. The diagnostic pins and patches retain that earlier control; current acceptance uses the ordinary `--recovery` mode and the real native continuation.

Retained evidence:

- Original last-browser RED: `/tmp/p237-public-recovery-first.log`.
- Preserved-peer RED: `/tmp/p237-public-recovery-peer-red.log`.
- Corrected owner control: `/tmp/p237-public-recovery-qualified-probe.log` and its sibling artifact directory.
- Agent Browser executable: `/tmp/p237-public-recovery-qualified-agent-browser-tests`, SHA256 `081032df662bcbf7d3bf8423f0dd2afd702dd33abc1eeccbc6b39df254ad0d32`.
- Remote View executable: `/tmp/p237-public-recovery-probe-remote-view-tests`, SHA256 `ac6549aff421618fd5b6049f865ed9df0a3681c8afa2d2678ecaab138b0ed2e3`.

These candidates include uncommitted fixture changes against Agent Browser `3cda91a5` and Remote View `9b26c82c`. They are not embedded final-commit or installed identities. The final control is terminal; its cleanup receipt reports no remaining non-zombie processes in the process groups created by that driver. This is task-scoped cleanup evidence, not a whole-host census.

## Delivery gate at diagnosis

Specify a continuation after existing native authentication that reaches the existing browser owner and preserves the opaque URL. Agent Browser retains browser recovery, exact-target focus and observation preparation; Remote View retains authentication and viewer ownership. Keep public pre-auth reads free of launch, recovery, focus and successful-use acknowledgement. Update the owning plan before adding any endpoint or protocol. Remove or move diagnostic owner-probe code before final repair qualification, retain the ordinary failing regression, and rerun both preserved-peer and original last-browser workflows after repair. Exact physical acceptance and the replacement dashboard remain separate open requirements.

## Native continuation source checkpoint

[ADR 0002](../../adr/0002-native-authenticated-handoff-recovery.md) defines the implemented boundary. The public locator validates retained intent without recovery and redirects a missing target to Remote View's native continuation. Fresh native authentication and document-navigation admission precede the capability-authenticated private owner POST. Existing recovery and exact-target preparation return the current native destination. Invalid private capabilities, browser cookies and missing operator context leave SQLite unchanged and emit no daemon command. Native authentication failures and prefetches emit no callback; foreign owner destinations are rejected.

The actual peer-preserving opaque HTTPS workflow passes WebSocket in 26.58 seconds and HTTP fallback in 26.58 seconds. Both reopen the same URL after actual configured target expiry, preserve the peer, obtain a new private native observation and matching frame 12, and request no Dashboard login. Task-created process groups have no remaining non-zombie members after cleanup. This is synthetic source qualification, not private operator authentication, physical browser/window proof, last-browser recovery or installation.

The first repaired HTTP run reached native recovery and loaded the new viewer, then received 503 at tunnel connection and hit the 25-second script limit. The connector returned the same upstream tunnel token for every rendered connection. Giving each rendered synthetic connection a distinct identity makes the original workflow pass without changing the gateway's occupied-token rejection. Preserve `/tmp/p237-native-resume-http-first.log` and its artifact directory. A probe compile failure used an unavailable UUID dependency; the fixture now uses its local deterministic sequence without a dependency change.

Qualification artifacts are `/tmp/p237-native-resume-ws-first.log`, `/tmp/p237-native-resume-http-distinct.log` and their sibling artifact directories. Agent Browser pin `/tmp/p237-native-resume-agent-browser-tests` has SHA256 `3dc1a4d6d25cfbd2a93cc167ba9cd2abf36a86cac265d4900561ba4900f78831`. WebSocket used Remote View pin `/tmp/p237-native-resume-remote-view-tests`, SHA256 `9b07cb9232e5ce884545456a85db7eedbc365fbf26d0dcf66194c84c64b53aa0`; the corrected HTTP fixture used `/tmp/p237-native-resume-distinct-http-remote-view-tests`, SHA256 `6713ccc4744f9ccd739d2168485b1e16567fd33392fdae9f39e58b4600e4bb34`. The latter changes only rendered synthetic tunnel identities. These are dirty-source candidates against the source baselines above, not installed or embedded final-commit identities.

Agent Browser passes 79 consumer tests with one explicit ignored composed workflow, 64 Dashboard-handler tests, five presentation tests and 39 output tests. Strict workspace Clippy, formatting check, documentation links, handoff docs, docs build, patch hygiene and both repositories' active planning audits pass. Remote View's final full offline suite is terminal with 414 passed, zero failed and 12 explicit ignores across 33 result blocks. Final all-target strict Clippy and formatting check pass. These source gates do not establish installation or full P237 acceptance.

The ordinary diagnostic shortcut is removed. Last-browser recovery requires a fixture that models pool/assignment evolution rather than the preserved-peer bridge snapshot. That original case and physical acceptance remain open, alongside the chosen workspace layout, native selection, real table/Properties/menus, old UI retirement and canonical integration. No whole-P237 completion is claimed.

## Stronger owner-state qualification

The final fixture also waits for the actual foreground owner observation to clear the recreated tab's pending-human hold. It checks the persisted first-use timestamp against the original value, exact peer identity, recreated target presence and one total browser launch. The new immutable Agent Browser pin `/tmp/p237-native-resume-owner-state-agent-browser-tests` has SHA256 `10de835360718387a5b4316ce2193a99f9208256efba0515d786f9aafb561f91`; executable inputs include the stronger fixture assertions, against baseline `3cda91a5`. Remote View pin remains the final `6713ccc4` digest recorded above.

Both ordinary same-URL workflows pass: WebSocket in 26.47 seconds and HTTP fallback in 26.57 seconds. Logs are `/tmp/p237-native-resume-owner-state-ws.log` and `/tmp/p237-native-resume-owner-state-http.log`. Their sibling artifact directories retain the two opaque 303 responses, zero Dashboard-login requests and exact task-process cleanup receipts with empty remaining lists. The owner readback reports `pendingHuman=false`, `recoveredTargetPresent=true` and `human_use_already_confirmed`. The private native provider separately proves the new correlation's frame 12; this does not infer a new first-use timestamp or require rewriting an already-confirmed observation.

Final formatting check and strict workspace Clippy pass after the fixture change. Earlier consumer/handler/output gates retain their scope because no product executable behavior changed after those gates. Remote View source commit `7ed47c1c628096c0dd90fcd099d5b1b96da9a112` is pushed on `feat/native-handoff-use-proof`, verified by exact remote-ref readback. Source custody is separate from installed activation and canonical integration. The original last-browser case remains open.


## Last-browser continuation replay

An explicit `--last-browser` fixture mode implies recovery and removes the preserved peer. At expiry the actual owner state contains no tabs or browsers while the original link remains valid. With the native continuation repair, the opaque URL returns 303 and the authenticated continuation returns 503. This is a different boundary from the original pre-repair 410. Preserve `/tmp/p237-last-browser-mode-ws.log` and `/tmp/p237-last-browser-mode-ws-artifacts`; the workflow failed in 26.41 seconds and its cleanup receipt has no remaining task-process members.

The frozen last-browser-mode Agent Browser pin has SHA256 `0d29b3e8a499bc9046abf3bc7f2886444a1729783315ccefdb57eb5e96edec39`, baseline `1ec8ccb6`; Remote View pin has SHA256 `989055a297d67da812267d7a239b0e2591150eacbc7fbc11d52c806ae3c8d475`, baseline `7ed47c1`. Both change fixture mode only; the provider snapshot and fixed window PID remain unchanged.

Ranked hypotheses before the next probe: fixed window PID after a new browser launch, fixed provider assignment state after lifecycle change, then incorrect first-use/hold restoration despite successful recovery. The first probe makes the composed synthetic window follow the fake process launch's recorded PID and prints only the owner result's success/error for diagnosis. No product focus fence is weakened. This probe is not yet qualified, and its tagged instrumentation must be removed before final acceptance.

The planning audit's successful return is structural hygiene only: P237 is excluded as unclassified and no plans are audited. It is not substantive acceptance of this plan or its remaining requirements.


The PID probe passes the original last-browser workflow in 26.62 seconds. Existing native assignment state remains valid without forwarding additional owner operations or changing assignment policy. Both recovery commands report success; a new native correlation records frame 12 and the actual owner clears pending human use while preserving the original first-use timestamp. The task-process cleanup list is empty. Probe log `/tmp/p237-last-browser-pid-probe-ws.log` and pin `/tmp/p237-last-browser-pid-probe-agent-browser-tests`, SHA256 `db36be4e623774221fdaff1a14c1fdd18dffa3f0e1262e2330548c21686cb955`, retain the result. This attributes the repaired-path 503 to the fixture's fixed window PID. It does not qualify a lifecycle-generation or assignment replacement: this last-browser case reuses the retained native assignment.

Tagged diagnostics are removed from current source. The final ordinary last-browser candidate must still pass both transports after removal before source qualification is updated. The original pre-repair last-browser 410 remains product RED evidence; physical focus and installed/private-authentication acceptance remain separate.


## Final last-browser source qualification

With diagnostic logging removed, ordinary last-browser opaque HTTPS recovery passes WebSocket in 26.68 seconds and HTTP fallback in 26.46 seconds. Both prove no live tabs or browsers at configured expiry, retain the original valid link, recreate its target through fresh native authentication, publish a distinct private native frame-12 observation, preserve the original first-use timestamp and clear the actual pending-human hold. Exactly two synthetic launches cover the original browser and its replacement. No Dashboard login is requested. The retained native assignment is reused; this does not prove assignment replacement, lifecycle-generation changes or physical focus.

The final Agent Browser pin `/tmp/p237-last-browser-final-agent-browser-tests` has SHA256 `55c149cf018da799efc6b7e20ba45ab4fcdb775bb7133446360a971e4766581c`, against source baseline `1ec8ccb6` with fixture-only changes. Remote View's last-browser-mode pin remains `989055a2` as fully recorded above. Logs are `/tmp/p237-last-browser-final-ws.log` and `/tmp/p237-last-browser-final-http.log`; their sibling artifact directories retain exact task-process cleanup receipts with empty remaining lists. A fresh process-group readback independently checks those lists. Both repositories pass final strict Clippy, formatting check and patch hygiene. Earlier product consumer/handler/output and Remote View full-suite evidence retains its stated scope: this batch changes the opt-in fixture mode, the composed fake process/window PID join and qualification prose, with no product behavior change. The peer-preserving case continues to use PID43 and the existing code path; its strongest two-transport evidence remains applicable. No full-suite rerun or installed identity is claimed.

The source-level same-URL requirement is qualified for peer-preserving and last-browser expiry on a retained assignment. Physical selection, provider/foreign and post-effect races, reviewed workspace/table/Properties/menus, old UI retirement and canonical integration remain open. Full P237 is not complete.
