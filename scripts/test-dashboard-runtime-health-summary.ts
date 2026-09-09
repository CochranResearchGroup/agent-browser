#!/usr/bin/env node

import assert from "node:assert/strict";
import { summarizeRuntimeAccess } from "../packages/dashboard/src/lib/runtime-health-summary.ts";

assert.deepEqual(summarizeRuntimeAccess(undefined), {
  ready: true,
  text: null,
});

assert.deepEqual(summarizeRuntimeAccess({ state: "allowed", findings: [] }), {
  ready: true,
  text: "Access allowed.",
});

const attention = summarizeRuntimeAccess({
  state: "attention",
  findings: Array.from({ length: 80 }, (_, index) => ({
    code: `legacy-profile-${index}`,
    blocking: index < 3,
    message: `Profile ${index} was migrated because legacy identity evidence was inconclusive.`,
  })),
});
assert.deepEqual(attention, {
  ready: false,
  text: "Access attention: 80 findings, 3 blocking. Review Service diagnostics.",
});
assert.doesNotMatch(attention.text ?? "", /legacy identity evidence|Profile 79/);
assert.equal((attention.text ?? "").length < 120, true);

assert.deepEqual(summarizeRuntimeAccess({
  state: "denied",
  findings: [{ code: "denied", blocking: true, message: "Long private diagnostic detail" }],
}), {
  ready: false,
  text: "Access denied: 1 finding, 1 blocking. Review Service diagnostics.",
});

console.log("Dashboard runtime health summary checks passed");
