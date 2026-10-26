# Teaching evidence source checkpoint for issue 473

Product lane: PL-RECIPES
Owning work item: [Issue 473](https://github.com/CochranResearchGroup/agent-browser/issues/473)
Disposition: active-input

The public teaching client joins scoped Remote View reads with Speech's owned
upload, immutable Saved copy, timestamped range operations and derived-text
removal. The HTTP adapter uses the actual owner routes and separate backend
credentials. It does not add a recorder, viewer, credential authority or text
store. Import retries reconcile the original Saved request before new copying.

## Seams and red/green

The stable client seam is `createTeachingEvidenceClient` plus concrete owner HTTP
adapters. It catches source/task/track substitution, artifact corruption, range
bounds, original-operation reopen and authority revocation. It does not prove
native recorder delivery, microphone/ASR quality or protected-entry suppression.

- `node scripts/test-teaching-evidence.js`: initial red was missing public module;
  green exercises exact selection, hash validation, explicit five-minute ranges,
  source gaps/bindings, no key-value projection, original-operation reopen,
  revoked/stale/wrong-task refusal and separate HTTP credentials/routes.
- `node scripts/qualify-teaching-speech-owner.mjs /absolute/reviewed/speech/dist/index.mjs`:
  explicit synthetic cross-package lane. Actual local Speech public HTTP,
  upload conversion and Saved/range/transcript APIs are used; the recognition
  worker is deterministic. FFmpeg creates Opus/WebM and the owner actually
  decodes it. A 500–1500 ms range returns its worked absolute 600 ms timestamp;
  known date/number/unit words are preserved. The source recording API is a
  contract fixture, not actual Native recording.
- A second red in that lane exposed HTTP409 on same-ID reimport after upload
  disposal. Existing Saved-request reconciliation fixes the recovery; green
  proves the same Saved ID returns without another copy.
- Sequence red was missing `inspectSequence`; green preserves adjacent exact
  record identities and caller-attributed protected gap with unknown timing.

The cross-package lane creates only disposable local synthetic state. It runs no
installed service, production browser, microphone or inference model. The exact
external artifact digest and owner source revision belong in the coordinator's
execution receipt; a dist artifact is not implicitly current source proof.

## Remaining acceptance boundary

The protected-entry path is owner Stop, confirmed local recorder shutdown and
finalization, credential entry, then explicit new Record. A scoped read grant
cannot perform those effects. Gateway finish alone finalizes storage without
proving local encoder shutdown. Sequence annotations preserve lineage without
claiming suppression. The supported manual Stop sequence is source-backed by Remote View
[8d73717, assistance.js](https://github.com/CochranResearchGroup/remote-view/blob/8d73717d1f32465891dab648200c765b5ff859e8/web/assistance.js#L390):
bounded tail drain, encoder stop/onstop, track stop, reader close, upload-chain
settlement, then authenticated same-origin finish and saved status. Caller gap
annotations remain separate from that owner implementation. Synthetic cases
refuse active sources before constructing the stop/resume lineage. Actual
operator/native stop/resume, physical narration and critical-token ASR quality
are separate qualification. This source checkpoint alone does not close issue
473; the coordinator adjudicates its frozen source criteria after integration.

Memory disposition: forbidden. Parent explicitly excluded Graphiti writes for
this worker. No memory write was attempted.
