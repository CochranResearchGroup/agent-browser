import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const manifestPath = new URL(
  "../../docs/dev/contracts/p218-grilling-contract-coverage.v1.json",
  import.meta.url,
);

const acceptedTitles = [
  "Existing runtime host owns presentation authority",
  "No operator-maintained route inventory",
  "Legacy policy and runtime JSON are migration-only",
  "One user-private SQLite authority",
  "Stale history cannot veto work",
  "Presentation warms before browser launch",
  "Ordinary open waits for real capacity",
  "No hidden infrastructure browser",
  "Displays and browsers have deterministic placement",
  "Capacity has bounded queuing",
  "Desktop Services owns control",
  "Handoff identity is durable",
  "Handoff resolution performs bounded recovery",
  "Recovery is eager only when useful",
  "Recovery is bounded and singular",
  "History is useful and bounded",
  "Exact and summarized history coexist",
  "SQLite is recoverable",
  "Operations are crash-consistent",
  "Privilege is installed once and used only as needed",
  "Disposable retention is bounded",
  "Configuration is typed and inspectable",
  "Valid user instructions are availability-first",
  "Development acceptance precedes production",
  "Provider credentials follow the SQLite authority",
  "Runtime generations fence every effect",
  "Browser placement is stable",
  "Extra capacity scales in safely",
  "Limit changes converge without eviction",
  "Status and doctor expose reconciliation",
  "Repeated opens are idempotent",
  "Recovery URL means committed top-level navigation",
  "Launch obeys live resource pressure",
  "Provider route state is derived",
  "Live viewer authority is observational",
  "Named and disposable handoffs differ",
  "Disposable quotas fail cleanly",
  "Disposable promotion is deferred",
  "Doctor is read-only",
  "Waiting work does not surprise-launch",
  "Legacy lease denial is physically quarantined",
  "Session management is heartbeat-based",
  "Same-profile sessions share without aliasing",
  "Cleanup follows active-session references",
  "Session and viewer heartbeats stay distinct",
];

const ids = acceptedTitles.map((_, index) => `G${String(index + 1).padStart(2, "0")}`);
const statuses = new Set(["pass", "partial", "fail", "missing"]);
const prohibitionIds = new Set(
  Array.from({ length: 19 }, (_, index) => `P${String(index + 1).padStart(2, "0")}`),
);
const stringArray = (value) =>
  Array.isArray(value) && value.every((entry) => typeof entry === "string");

function validate(manifest) {
  assert.equal(manifest?.schema, "agent-browser.p218-grilling-contract-coverage.v1");
  assert.equal(
    manifest?.plan,
    "0218-2026-09-23-grilling-contract-remote-view-conformance",
  );
  assert.ok(Array.isArray(manifest.rows), "rows must be an array");
  assert.equal(manifest.rows.length, 45, "must contain exactly 45 rows");

  const seen = new Set();
  manifest.rows.forEach((row, index) => {
    assert.ok(!seen.has(row.id), `duplicate requirement ${row.id}`);
    seen.add(row.id);
    assert.equal(row.id, ids[index], `row ${index + 1} must be ${ids[index]}`);
    assert.equal(row.accepted_decision, acceptedTitles[index], `${row.id} title drift`);
    assert.ok(
      typeof row.requirement_summary === "string" && row.requirement_summary.trim(),
      `${row.id} requirement summary must be nonempty`,
    );
    assert.ok(statuses.has(row.status), `${row.id} has invalid status ${row.status}`);
    for (const field of [
      "current_source_symbols",
      "current_source_files",
      "state_stores",
      "tests_evidence",
      "violated_prohibition_ids",
    ]) {
      assert.ok(stringArray(row[field]), `${row.id}.${field} must be a string array`);
    }
    assert.ok(row.tests_evidence.length > 0, `${row.id} needs evidence or an explicit evidence gap`);
    for (const prohibitionId of row.violated_prohibition_ids) {
      assert.ok(prohibitionIds.has(prohibitionId), `${row.id} has invalid prohibition ID ${prohibitionId}`);
    }
    assert.ok(
      typeof row.remaining_gap === "string" && row.remaining_gap.trim(),
      `${row.id} remaining_gap must be nonempty`,
    );
  });
  assert.equal(seen.size, 45, "requirement IDs must be unique");
}

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
validate(manifest);

const omitted = structuredClone(manifest);
omitted.rows.splice(7, 1);
assert.throws(() => validate(omitted), /exactly 45 rows/);

const duplicated = structuredClone(manifest);
duplicated.rows[7].id = duplicated.rows[6].id;
assert.throws(() => validate(duplicated), /duplicate requirement G07/);

const invalidStatus = structuredClone(manifest);
invalidStatus.rows[0].status = "unknown";
assert.throws(() => validate(invalidStatus), /invalid status/);

const invalidProhibition = structuredClone(manifest);
invalidProhibition.rows[0].violated_prohibition_ids = ["P20"];
assert.throws(() => validate(invalidProhibition), /invalid prohibition ID/);

const counts = Object.fromEntries([...statuses].map((status) => [status, 0]));
for (const row of manifest.rows) counts[row.status] += 1;
assert.equal(manifest.rows[0].id, "G01");
assert.equal(manifest.rows.at(-1).id, "G45");
console.log(
  `P218 coverage manifest valid: 45 ordered unique rows; status counts ${JSON.stringify(counts)}; negative fixtures passed (omission, duplicate, bad status, bad prohibition ID).`,
);
