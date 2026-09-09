#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import { sha256 } from './lib/p158-campaign-controller.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import {
  createP158W7A01A03LiveBundle,
  createP158W7A01A03OwnershipManifest,
  P158_W7_A01_A03_CASE_IDS,
} from './lib/p158-w7-a01-a03-live.js';
import {
  createP158W7A04A06LiveBundle,
  createP158W7A04A06OwnershipManifest,
} from './lib/p158-w7-a04-a06-live.js';
import {
  auditP158W7LiveHookReadiness,
  p158W7LiveHookReadinessSourceBinding,
  P158_W7_LIVE_HOOK_AUDIT_CASE_IDS,
  P158_W7_PRODUCT_BLOCKED_CASE_IDS,
} from './lib/p158-w7-live-hook-readiness.js';

const registry = JSON.parse(await readFile(
  'docs/dev/contracts/p158-historical-failure-registry.v1.json', 'utf8'));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w7-live-hook-readiness' });

const input = {
  candidateSha256: '11'.repeat(32),
  environmentSealSha256s: { E0: '20'.repeat(32), E1: '21'.repeat(32), E2: '22'.repeat(32), E3: '23'.repeat(32) },
};
const original = structuredClone(input);
const report = auditP158W7LiveHookReadiness(input);
assert.deepEqual(input, original);
assert.equal(report.reviewedCaseCount, 24);
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

function a01A03Manifest() {
  const fixture = (caseId, environmentId) => ({
    url: `http://127.0.0.1:43158/${caseId.toLowerCase()}/${environmentId.toLowerCase()}`,
    profileId: `p158-${caseId.toLowerCase()}-${environmentId.toLowerCase()}-profile`,
    sessionName: `p158-${caseId.toLowerCase()}-${environmentId.toLowerCase()}-session`,
    ...(caseId === 'A01' ? {} : { browserId: `p158-${caseId.toLowerCase()}-${environmentId.toLowerCase()}-browser` }),
    ...(caseId === 'A03' ? { sharedLabel: 'same-label' } : {}),
  });
  const fixtures = Object.fromEntries(P158_W7_A01_A03_CASE_IDS.map((caseId) => [caseId,
    Object.fromEntries(['E0', 'E1'].map((environmentId) => [environmentId, fixture(caseId, environmentId)]))]));
  const environment = (environmentId) => ({
    serviceOrigin: 'http://127.0.0.1:43158', runtimeLane: 'development', production: false,
    runtimeEnvironmentId: environmentId, targetId: `p158-${environmentId.toLowerCase()}-target`,
    ownershipStatus: {
      runtimeLifecycle: { environment: 'development', runId: 'p158-readiness-run' },
      service_state: {
        candidateSha256: input.candidateSha256,
        browsers: Object.fromEntries(['A02', 'A03'].map((caseId) => {
          const browserId = fixtures[caseId][environmentId].browserId;
          return [browserId, { id: browserId, lifecycle: 'ready', retained: true }];
        })),
      },
    },
  });
  return createP158W7A01A03OwnershipManifest({
    schemaVersion: 'agent-browser.p158-w7-a01-a03-ownership.v1',
    campaignRunId: 'p158-readiness-run', candidateSha256: input.candidateSha256,
    liveHookManifestSha256: '30'.repeat(32),
    environmentSealSha256s: { E0: input.environmentSealSha256s.E0, E1: input.environmentSealSha256s.E1 },
    environments: { E0: environment('E0'), E1: environment('E1') }, fixtures,
  });
}

function a04A06Manifest() {
  const attempts = schedule.attempts.filter((attempt) => attempt.caseId === 'A05');
  const fixture = (attempt) => {
    const adminSubjectId = `principal:${attempt.attemptId}:admin`;
    const participantSubjectId = `principal:${attempt.attemptId}:participant`;
    return {
      profileId: `profile-${attempt.attemptId}`, sessionName: `session-${attempt.attemptId}`,
      url: `http://127.0.0.1:43158/${attempt.attemptId}`, adminSubjectId, participantSubjectId,
      adminCapability: { absolutePath: `/tmp/${attempt.attemptId}-admin`, sha256: '41'.repeat(32),
        principalId: adminSubjectId },
      participantCapability: { absolutePath: `/tmp/${attempt.attemptId}-participant`, sha256: '42'.repeat(32),
        principalId: participantSubjectId },
    };
  };
  return createP158W7A04A06OwnershipManifest({
    schemaVersion: 'agent-browser.p158-w7-a04-a06-ownership.v1',
    campaignRunId: 'p158-readiness-run', candidateSha256: input.candidateSha256,
    liveHookManifestSha256: '30'.repeat(32),
    environmentSealSha256s: { E0: input.environmentSealSha256s.E0, E1: input.environmentSealSha256s.E1 },
    environments: Object.fromEntries(['E0', 'E1'].map((environmentId) => [environmentId, {
      serviceOrigin: 'http://127.0.0.1:43158', runtimeLane: 'development', production: false,
      runtimeEnvironmentId: environmentId,
      ownershipStatus: { runtimeLifecycle: { environment: 'development', runId: 'p158-readiness-run' },
        service_state: { candidateSha256: input.candidateSha256 } },
    }])),
    fixtures: { A05: Object.fromEntries(attempts.map((attempt) => [attempt.attemptId, fixture(attempt)])) },
  });
}

const receiptStore = { async append() {} };
const a01A03LiveBundle = createP158W7A01A03LiveBundle({
  schedule, ownershipManifest: a01A03Manifest(), receiptStore,
});
const a04A06LiveBundle = createP158W7A04A06LiveBundle({
  schedule, ownershipManifest: a04A06Manifest(), receiptStore,
});
const promoted = auditP158W7LiveHookReadiness({
  ...input,
  a01A03LiveBundle,
  a04A06LiveBundle,
});
assert.deepEqual(promoted.concreteCaseIds, ['A01', 'A02', 'A03', 'A05']);
assert.deepEqual(Object.fromEntries(promoted.cases.filter((entry) => entry.implementationKind === 'concrete_live')
  .map((entry) => [entry.caseId, entry.implementedActionCount])), { A01: 250, A02: 400, A03: 20, A05: 12 });
assert.ok(['A04', 'A06', 'A07', 'A08', 'A09', 'A10', 'A13', 'A15', 'X01', 'X02', 'X03', 'X04', 'X05',
  'X07', 'X08', 'X09', 'X10', 'A11', 'A12', 'A14'].every((caseId) =>
  promoted.explicitBlockedCaseIds.includes(caseId)));

for (const forged of [
  { a01A03LiveBundle: { ...a01A03LiveBundle, candidateSha256: '99'.repeat(32) }, a04A06LiveBundle },
  { a01A03LiveBundle: { ...a01A03LiveBundle,
    driverSource: { ...a01A03LiveBundle.driverSource, sourceSha256: '99'.repeat(32) } }, a04A06LiveBundle },
  { a01A03LiveBundle: { ...a01A03LiveBundle, campaignRunId: 'forged-run' }, a04A06LiveBundle },
  { a01A03LiveBundle, a04A06LiveBundle: { ...a04A06LiveBundle,
    environmentSealSha256s: { ...a04A06LiveBundle.environmentSealSha256s, E1: '99'.repeat(32) } } },
  { a01A03LiveBundle, a04A06LiveBundle: { ...a04A06LiveBundle, adapterBindingSha256: '99'.repeat(32) } },
]) {
  const forgedReport = auditP158W7LiveHookReadiness({ ...input, ...forged });
  const rejectedCaseIds = forged.a01A03LiveBundle === a01A03LiveBundle ? ['A05'] : ['A01', 'A02', 'A03'];
  assert.ok(rejectedCaseIds.every((caseId) => forgedReport.explicitBlockedCaseIds.includes(caseId)));
}

const mismatchedHookBundle = {
  ...a04A06LiveBundle,
  liveHookManifestSha256: '31'.repeat(32),
};
mismatchedHookBundle.adapterBindingSha256 = sha256({
  caseIds: ['A05'], ownershipManifestSha256: mismatchedHookBundle.ownershipManifestSha256,
  campaignRunId: mismatchedHookBundle.campaignRunId,
  candidateSha256: mismatchedHookBundle.candidateSha256,
  liveHookManifestSha256: mismatchedHookBundle.liveHookManifestSha256,
  environmentSealSha256s: mismatchedHookBundle.environmentSealSha256s,
  source: mismatchedHookBundle.driverSource,
  liveHookIds: mismatchedHookBundle.liveHookIds,
});
const mismatchedCohort = auditP158W7LiveHookReadiness({
  ...input, a01A03LiveBundle, a04A06LiveBundle: mismatchedHookBundle,
});
assert.ok(['A01', 'A02', 'A03', 'A05'].every((caseId) =>
  mismatchedCohort.explicitBlockedCaseIds.includes(caseId)));

for (const invalid of [
  { ...input, candidateSha256: 'bad' },
  { ...input, environmentSealSha256s: { E1: 'bad' } },
]) {
  assert.throws(() => auditP158W7LiveHookReadiness(invalid), (error) => error.code === 'w7_readiness_seal_missing');
}

process.stdout.write('P158 W7 readiness audit passed with four exact concrete cases and fail-closed bundle validation\n');
