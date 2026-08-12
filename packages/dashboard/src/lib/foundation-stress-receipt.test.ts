import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { projectFoundationStressReceipt } from "./foundation-stress-receipt.ts";

const projection = projectFoundationStressReceipt({
  success: true,
  data: {
    action: "desktop_interact",
    interactionReceipt: {
      operationIdDigest: "operation-digest-7",
      operationRequestSha256: "request-hash-private",
      recipeId: "p110-foundation-stress-v1",
      effectState: "uncertain",
      replayState: "replayed_complete",
      cleanupState: "attempted_once",
      verificationState: "failed",
      entryGate: "planning_open_implementation_blocked",
      promptDisposition: { state: "operator_intervention", reasonCode: "external_prompt" },
      humanHandoff: {
        state: "ready",
        reason: "uncertain_effect",
        handoffId: "handoff-opaque-7",
        handoffUrl: "https://provider.invalid/raw-route",
      },
      plaintext: "must-not-project",
      fullPath: [[1, 2]],
    },
  },
});

assert.deepEqual(projection, {
  recipeId: "p110-foundation-stress-v1",
  operationIdentity: "recorded",
  effectState: "uncertain",
  replayState: "replayed_complete",
  cleanupState: "attempted_once",
  verificationState: "failed",
  entryGate: "planning_open_implementation_blocked",
  promptState: "operator_intervention",
  promptReasonCode: "external_prompt",
  handoffState: "ready",
  handoffId: "handoff-opaque-7",
});
assert.equal(JSON.stringify(projection).includes("operation-digest-7"), false);
assert.equal(JSON.stringify(projection).includes("provider.invalid"), false);
assert.equal(JSON.stringify(projection).includes("must-not-project"), false);

assert.equal(projectFoundationStressReceipt({ data: { action: "desktop_capture" } }), null);
assert.equal(projectFoundationStressReceipt({ data: { action: "desktop_interact" } }), null);

const servicePanel = readFileSync(
  new URL("../components/service-panel.tsx", import.meta.url),
  "utf8",
);
assert.match(servicePanel, /projectFoundationStressReceipt\(job\.result\)/);
assert.match(servicePanel, /Foundation stress receipt/);
assert.doesNotMatch(servicePanel, /label: "Handoff URL"|label: "Operation ID"/);

console.log("foundation stress receipt projection tests passed");
