# P300 SiteGuides composition source checkpoint

Product lane: PL-PLATFORM

Issue #528 implements the SiteGuides manager and teaching composition under the frozen #523 contract. This checkpoint is source implementation, not actual narration, source-state, performance or human acceptance. Those remain #529 and deferred #478.

The public owner is `packages/client/src/workspace-siteguides.js`. It composes existing teaching evidence, Speech, candidate directory, review, grilling and fresh-repeat/promotion owners. Guide files remain the existing immutable candidate lineage. No guide, approval or authentication registry is added. Intended identity and authentication freshness are independent and unchanged by teaching.

The dashboard exports `SiteGuidesLibrary` and `TeachSite`. Their `context` is the exact selected `{taskId,profileId,sitePolicyId,account,organization}`; no browser disables teaching. `onNativeView` opens the existing native owner. `onInterpret` must be a real ordinary-agent integration: absent response remains awaiting, and no draft or repeat is inferred. Root composes these components into the workspace and forwards `/api/dashboard/siteguides` through dashboard.rs. ProfileLibrary guide navigation now supplies optional profileId alongside sitePolicyId.

`stream/http.rs` owns authenticated GET/POST `/api/dashboard/siteguides`. Operation classification keeps metadata reads separate from mutations, narration import and transcription. Mutations require the current operator role. Rust constructs the authenticated dashboard subject and derives allowed task/profile visibility/use from the existing profile owner and native session inventory. Caller actor, assurance, private configuration, source binding and runtime paths are discarded.

The Node host is embedded alongside its exact transitive owner modules in existing `CONTROLLER_ASSETS`; the existing workstation materialization, manifest hashes and doctor therefore cover these support files. It resolves only relative to the canonical installed executable generation; a source checkout/cwd fallback is deliberately absent. Node.js 18 or newer is required. Private `AGENT_BROWSER_SITEGUIDES_CONFIG` identifies existing candidate directory, scoped Remote View read grants, Speech and existing authenticated MCP transport. Missing support, configuration, Node, agent or receiving verifier remains unavailable. No daemon or npm runtime dependency is introduced.

Teaching review and answer receipts read existing App Intelligence operator-confirmation records only with dedicated teaching purpose, exact task/root/raw/content/question digest and the actual authenticated actor. An unrelated browser action confirmation is insufficient. Fresh-repeat input is a reference to receiving-verifier evidence, never caller `passed` assertions. Original-operation lookup does not infer retry authority.

## Local source evidence

- RED: `node scripts/test-workspace-siteguides.mjs`, missing composition module; `/tmp/p300-528-owner-red1.log`.
- GREEN: same public owner fixture; `/tmp/p300-528-owner-green1.log`.
- `node scripts/test-workspace-siteguides-installed.mjs`: real packaged transitive module graph executes from a temporary generation and unrelated cwd; missing config fails closed.
- `node scripts/test-dashboard-siteguides.cjs`: mounted purpose search, selected context, native owner callback, finalized evidence, awaiting agent and original-publication lookup without replay; `/tmp/p300-528-mounted-green1.log`.
- `AGENT_BROWSER_CARGO_CACHE=off CARGO_TARGET_DIR=/tmp/agent-browser-p300-526-target scripts/ci/rust-tests.sh --focused siteguides_operation_policy`: PASS, one focused CLI test; `/tmp/p300-528-rust1.log`.

## Remaining checkpoint gates

The real agent/media packet loading and durable original agent submission orchestration, full HTTP mutation/read/policy fixture and expanded owner qualification remain pending. This checkpoint does not qualify #528 end to end. Root owns exact integrated dashboard forwarding/composition, shared docs in all five feature surfaces, final format/strict Clippy, installed package doctor and source-free candidate acceptance. Hosted CI stays disabled. No runtime publication, external model, provider, account or media access occurred.

Graphiti disposition: unavailable. The scoped discovery service was busy and root directed continuation from authoritative source; this checkpoint makes no memory write or persistence claim.

## Focused followup

- Existing teaching-draft owner now owns immutable original agent intent, media digest and resolved-response provenance sidecars. Exactly one concurrent begin is admitted. Pending/unknown never invokes another model call; a response must be read and staged without model replay. These records are not guide heads or acceptance authority.
- Read-only `reconcileNarration` delegates only the original Speech Saved request readback. RED missing method and GREEN existing evidence suite are retained at `/tmp/p300-528-reconcile-{red,green}.log`.
- `scripts/test-teaching-agent-submission.mjs` has RED missing owner/method and GREEN for atomic intent/media/result, concurrent admission and fresh-process readback. `scripts/test-workspace-siteguides-agent.mjs` exercises exact recording protocol, bounded video, existing owner publication, no replay and source/policy refusal with mocked external transports.
- Existing authenticated confirmation ledger directory now comes directly from the App Intelligence owner, not configurable receipt storage. `scripts/test-workspace-siteguides-receipts.mjs` verifies actor, purpose, exact content and question binding, and rejects unrelated browser confirmations.
- Real Socket HTTP fixture `siteguides_real_http_auth_classification_and_no_launch` passes through the existing authenticated route. Initial RED expected403 for a stale role cookie; existing authentication correctly returned401. The fixture now checks stale-cookie401, authenticated-observer403 writes and read classification separately. Exact receipt: `/tmp/p300-528-http3.log`.
- Public owner JS type check and dashboard TypeScript pass. Mounted consumer passes with explicit source selection, unavailable agent and original-operation readback. Browser client retains descriptive original operation IDs across reload; those references supply no acceptance or replay authority.

The real default model/media bridge is still pending integration with root-owned bounded teaching-media and private chat adapters. External ordinary agents can already inspect exact authorized video bytes and original transcript operations, stage an inert guide and use the existing review/grilling APIs. No synthetic guide is presented as a real agent response.
