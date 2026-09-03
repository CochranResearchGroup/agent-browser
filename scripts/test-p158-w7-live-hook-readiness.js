#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  auditP158W7LiveHookReadiness,
  p158W7LiveHookReadinessSourceBinding,
  P158_W7_LIVE_HOOK_AUDIT_CASE_IDS,
  P158_W7_PRODUCT_BLOCKED_CASE_IDS,
} from './lib/p158-w7-live-hook-readiness.js';

const input = {
  candidateSha256: '11'.repeat(32),
  environmentSealSha256s: { E0: '20'.repeat(32), E1: '21'.repeat(32), E2: '22'.repeat(32), E3: '23'.repeat(32) },
};
const original = structuredClone(input);
const report = auditP158W7LiveHookReadiness(input);
assert.deepEqual(input, original);
assert.equal(report.reviewedCaseCount, 22);
assert.deepEqual(report.concreteCaseIds, []);
assert.deepEqual(report.explicitBlockedCaseIds,
  [...new Set([...P158_W7_LIVE_HOOK_AUDIT_CASE_IDS, ...P158_W7_PRODUCT_BLOCKED_CASE_IDS])]);
assert.ok(report.cases.every((entry) => entry.implementationKind === 'explicit_blocked' &&
  entry.effectsAllowed === false && entry.implementedActionCount === 0 && entry.findingCodes.length > 0));
assert.ok(report.cases.filter((entry) => P158_W7_PRODUCT_BLOCKED_CASE_IDS.includes(entry.caseId))
  .every((entry) => entry.blockerKind === 'product_source'));
assert.ok(report.cases.filter((entry) => P158_W7_LIVE_HOOK_AUDIT_CASE_IDS.includes(entry.caseId))
  .filter((entry) => !P158_W7_PRODUCT_BLOCKED_CASE_IDS.includes(entry.caseId))
  .every((entry) => entry.blockerKind === 'campaign_harness'));
assert.equal(report.effectsAttempted, false);
assert.equal(report.repairAttempted, false);
assert.equal(report.retryAttempted, false);
const { auditSha256, ...body } = report;
assert.equal(auditSha256, sha256(body));
for (const source of report.sourceEvidence) {
  assert.equal(source.sourceSha256, sha256(await readFile(source.sourcePath)), source.sourcePath);
}
const binding = p158W7LiveHookReadinessSourceBinding();
assert.equal(binding.sourceSha256, sha256(await readFile(binding.sourcePath)));

const promoted = auditP158W7LiveHookReadiness({
  ...input,
  a04A06LiveBundle: {
    freezeEligible: true,
    providerFree: false,
    candidateSha256: input.candidateSha256,
    concreteCaseIds: ['A05'],
    readiness: { counts: { A05: { executable: 12 } } },
    driverSource: {
      sourcePath: 'scripts/lib/p158-w7-a04-a06-live.js',
      sourceSha256: sha256(await readFile('scripts/lib/p158-w7-a04-a06-live.js')),
    },
  },
});
assert.deepEqual(promoted.concreteCaseIds, ['A05']);
assert.equal(promoted.explicitBlockedCaseIds.includes('A05'), false);
assert.deepEqual(promoted.cases.find((entry) => entry.caseId === 'A05'), {
  caseId: 'A05', requestedMode: 'concrete_live', implementationKind: 'concrete_live',
  blockerKind: null, findingCodes: [], effectsAllowed: true, implementedActionCount: 12,
  ownershipReceiptState: 'frozen_and_effect_time_revalidated',
});

for (const invalid of [
  { ...input, candidateSha256: 'bad' },
  { ...input, environmentSealSha256s: { E1: 'bad' } },
]) {
  assert.throws(() => auditP158W7LiveHookReadiness(invalid), (error) => error.code === 'w7_readiness_seal_missing');
}

process.stdout.write('P158 W7 live hook source readiness audit passed with 22 exact blockers\n');
