import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { projectFoundationStressReceipt } from "./foundation-stress-receipt.ts";

const fixture = JSON.parse(readFileSync(
  new URL("../../../../docs/dev/fixtures/desktop-foundation-stress/verified-success-replay.json", import.meta.url),
  "utf8",
)) as Record<string, unknown>;
const uncertainFixture = JSON.parse(readFileSync(
  new URL("../../../../docs/dev/fixtures/desktop-foundation-stress/post-effect-uncertain-handoff.json", import.meta.url),
  "utf8",
)) as Record<string, unknown>;

// This is the persisted redacted shape produced by the core redactor for the
// repository-owned verified-success replay fixture. Private values are present
// solely to prove that the projection does not return them.
const goldenPersistedResult = {
  success: true,
  data: {
    ok: true,
    action: "desktop_interact",
    interactionReceipt: {
      operationIdDigest: "a".repeat(64),
      operationRequestSha256: "b".repeat(64),
      recipeId: fixture.recipeId,
      recipeProviderId: fixture.recipeProviderId,
      recipeProviderVersion: fixture.recipeProviderVersion,
      effectState: fixture.expectedEffectState,
      replayState: fixture.expectedReplayState,
      cleanupState: fixture.expectedCleanupState,
      verificationState: fixture.expectedVerificationState,
      entryGate: fixture.expectedEntryGate,
      promptDisposition: {
        state: "actionable_observation",
        reasonCode: "synthetic_prompt_actionable",
        observationSha256: "c".repeat(64),
      },
      humanHandoff: null,
      effectKeyCount: fixture.expectedProviderCallCount,
      recipeSha256: "d".repeat(64),
      effectKeyDigest: "e".repeat(64),
    },
  },
};

const projection = projectFoundationStressReceipt(goldenPersistedResult);
assert.deepEqual(projection, {
  recipeId: "p110-foundation-stress-v1",
  operationIdentity: "recorded",
  effectState: "verified_success",
  replayState: "replayed_terminal",
  cleanupState: "not_needed",
  verificationState: "passed",
  entryGate: "closed_live_evidence_required",
  promptState: "actionable_observation",
  promptReasonCode: "synthetic_prompt_actionable",
});

const rendered = JSON.stringify(projection);
for (const forbidden of ["a".repeat(64), "b".repeat(64), "c".repeat(64), "d".repeat(64), "e".repeat(64)]) {
  assert.equal(rendered.includes(forbidden), false);
}

const handoffProjection = projectFoundationStressReceipt({
  success: true,
  data: {
    action: "desktop_interact",
    interactionReceipt: {
      ...goldenPersistedResult.data.interactionReceipt,
      effectState: uncertainFixture.expectedEffectState,
      replayState: uncertainFixture.expectedReplayState,
      cleanupState: uncertainFixture.expectedCleanupState,
      verificationState: uncertainFixture.expectedVerificationState,
      entryGate: uncertainFixture.expectedEntryGate,
      humanHandoff: {
        state: uncertainFixture.expectedHandoffState,
        reason: "effect_uncertain",
        handoffId: "opaque-handoff-1",
      },
    },
  },
});
assert.equal(handoffProjection?.handoffState, "ready");
assert.equal(handoffProjection?.handoffId, "opaque-handoff-1");

const withReceipt = (receipt: Record<string, unknown>) => ({
  success: true,
  data: { action: "desktop_interact", interactionReceipt: receipt },
});
const receipt = goldenPersistedResult.data.interactionReceipt;

assert.equal(projectFoundationStressReceipt(withReceipt({ ...receipt, recipeId: "p110-pointer-keyboard-v1" })), null);
for (const field of ["effectState", "replayState", "cleanupState", "verificationState"] as const) {
  const malformed: Record<string, unknown> = { ...receipt };
  delete malformed[field];
  assert.equal(projectFoundationStressReceipt(withReceipt(malformed)), null);
}
assert.equal(projectFoundationStressReceipt(withReceipt({ ...receipt, entryGate: "planning_open_implementation_blocked" })), null);
assert.equal(projectFoundationStressReceipt(withReceipt({ ...receipt, operationIdDigest: "not-a-digest" })), null);
assert.equal(projectFoundationStressReceipt(withReceipt({ ...receipt, humanHandoff: { state: "ready", handoffId: "handoff-1", handoffUrl: "https://provider.invalid/raw" } })), null);
assert.equal(projectFoundationStressReceipt({ data: { action: "desktop_capture" } }), null);
assert.equal(projectFoundationStressReceipt({ data: { action: "desktop_interact" } }), null);

const servicePanel = readFileSync(new URL("../components/service-panel.tsx", import.meta.url), "utf8");
assert.match(servicePanel, /projectFoundationStressReceipt\(job\.result\)/);
assert.match(servicePanel, /Foundation stress receipt/);
assert.doesNotMatch(servicePanel, /label: "Handoff URL"|label: "Operation ID"|operationIdDigest/);

console.log("foundation stress receipt projection tests passed");
