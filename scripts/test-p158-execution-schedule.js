#!/usr/bin/env node

import assert from 'node:assert/strict';
import fs from 'node:fs';
import {
  P158_ADAPTER_READINESS_CODES,
  P158_EXECUTION_PHASES,
  P158ExecutionScheduleError,
  assessP158AdapterReadiness,
  compileP158ExecutionSchedule,
  createP158AdapterExecutor,
  createP158CaseAdapter,
} from './lib/p158-execution-schedule.js';

const registryPath = new URL(
  '../docs/dev/contracts/p158-historical-failure-registry.v1.json',
  import.meta.url,
);
const registry = JSON.parse(fs.readFileSync(registryPath, 'utf8'));

function clone(value) {
  return structuredClone(value);
}

function expectScheduleError(code, action) {
  assert.throws(action, (error) => {
    assert(error instanceof P158ExecutionScheduleError);
    assert.equal(error.code, code);
    return true;
  });
}

async function expectScheduleRejection(code, action) {
  await assert.rejects(action, (error) => {
    assert(error instanceof P158ExecutionScheduleError);
    assert.equal(error.code, code);
    return true;
  });
}

function adaptersFor(schedule, executeByCase = {}) {
  return schedule.caseContracts.map((contract) => createP158CaseAdapter({
    caseId: contract.caseId,
    evidenceProfile: contract.evidenceProfile,
    execute: executeByCase[contract.caseId] ?? (async () => ({ resultState: 'passed' })),
  }));
}

function compileReady(seed = 'p158-exhaustive-seed', executeByCase = {}) {
  const preliminary = compileP158ExecutionSchedule({ registry, seed });
  const adapters = adaptersFor(preliminary, executeByCase);
  return {
    adapters,
    schedule: compileP158ExecutionSchedule({ registry, seed, adapters }),
  };
}

function findingCodes(report) {
  return report.findings.map((entry) => entry.code);
}

const originalRegistry = clone(registry);
const first = compileP158ExecutionSchedule({ registry, seed: 'frozen-seed' });
const second = compileP158ExecutionSchedule({ registry, seed: 'frozen-seed' });
assert.deepEqual(first, second, 'same registry and seed must compile byte-equivalent data');
assert.deepEqual(registry, originalRegistry, 'compilation must not mutate the frozen registry');
assert.equal(first.caseCount, 54);
assert.equal(first.attemptCount, 104);
assert.equal(first.caseContracts.length, 54);
assert.equal(first.attempts.length, 104);
assert.equal(new Set(first.caseContracts.map((entry) => entry.caseId)).size, 54);
assert.equal(new Set(first.attempts.map((entry) => entry.attemptId)).size, 104);
assert.equal(new Set(first.attempts.map((entry) => entry.seed)).size, 104);
assert.deepEqual(first.attempts.map((entry) => entry.scheduleSequence),
  Array.from({ length: 104 }, (_, index) => index));
assert.deepEqual([...new Set(first.attempts.map((entry) => entry.phaseId))], ['W7', 'W8', 'W9']);

const registryCases = new Map(registry.cases.map((testCase) => [testCase.id, testCase]));
const attemptSequence = new Map(first.attempts.map((attempt) => [
  attempt.attemptId,
  attempt.scheduleSequence,
]));
for (const contract of first.caseContracts) {
  const testCase = registryCases.get(contract.caseId);
  assert(testCase, `unexpected compiled case ${contract.caseId}`);
  assert.equal(contract.executionBound, testCase.executionBound);
  assert.equal(contract.evidenceProfile, testCase.evidenceProfile);
  assert.deepEqual(contract.environmentIds, [...testCase.environmentIds].sort());
  assert.deepEqual(contract.dependsOnCaseIds, [...testCase.dependsOn].sort());
  assert.equal(contract.adapterId, `p158.case.${contract.caseId}.v1`);
  assert.deepEqual(contract.declaredEffectIds, [`p158.effect.${contract.caseId}.declared`]);
  assert.equal(contract.reactionaryRepairAllowed, false);
  assert.equal(contract.opportunisticRetryAllowed, false);
  assert.equal(contract.undeclaredEffectsAllowed, false);
}

for (const attempt of first.attempts) {
  const testCase = registryCases.get(attempt.caseId);
  const expectedDependencies = testCase.dependsOn
    .flatMap((dependency) => first.attempts
      .filter((candidate) => candidate.caseId === dependency)
      .map((candidate) => candidate.attemptId))
    .sort();
  assert.deepEqual(attempt.dependsOnAttemptIds, expectedDependencies);
  for (const dependency of attempt.dependsOnAttemptIds) {
    assert(attemptSequence.get(dependency) < attempt.scheduleSequence,
      `${dependency} must precede ${attempt.attemptId}`);
  }
  const expectedIngress = attempt.environmentId === 'E2' && (
    ['external', 'dashboard', 'combined'].includes(testCase.evidenceProfile) ||
    ['X06', 'X10'].includes(testCase.id)
  );
  assert.equal(attempt.externalIngressRequired, expectedIngress, attempt.attemptId);
  assert.equal(attempt.scheduleId, `${attempt.phaseId}:${attempt.attemptId}`);
  assert.equal(attempt.repetition, 1);
}
assert.equal(first.attempts.find((entry) => entry.attemptId === 'A15-E2-r001').externalIngressRequired,
  false);
assert.equal(first.attempts.find((entry) => entry.attemptId === 'X06-E2-r001').externalIngressRequired,
  true);
assert.equal(first.attempts.find((entry) => entry.attemptId === 'X10-E2-r001').externalIngressRequired,
  true);
assert(first.attempts.filter((entry) => entry.caseId.startsWith('C') && entry.environmentId === 'E2')
  .every((entry) => entry.externalIngressRequired));

assert.deepEqual(first.phases.map((phase) => phase.phaseId), ['W7', 'W8', 'W9']);
assert.deepEqual(P158_EXECUTION_PHASES.map((phase) => phase.phaseId), ['W7', 'W8', 'W9']);
for (const phase of first.phases) {
  assert(phase.attemptIds.every((attemptId) =>
    first.attempts.find((attempt) => attempt.attemptId === attemptId).phaseId === phase.phaseId));
}
assert.deepEqual(first.environments.map((entry) => entry.environmentId), ['E0', 'E1', 'E2', 'E3']);
assert.equal(first.environments.reduce((total, entry) => total + entry.attemptCount, 0), 104);
assert(first.environments.find((entry) => entry.environmentId !== 'E2')
  .externalIngressAttemptIds.length === 0);
assert(first.environments.filter((entry) => entry.environmentId !== 'E2')
  .every((entry) => entry.externalIngressAttemptIds.length === 0));

assert.equal(first.adapterReadiness.ready, false);
assert.equal(first.adapterReadiness.expectedCaseCount, 54);
assert.equal(first.adapterReadiness.readyCaseCount, 0);
assert.equal(first.adapterReadiness.findingCount, 54);
assert.deepEqual(new Set(findingCodes(first.adapterReadiness)), new Set(['missing_case_adapter']));
assert.deepEqual(first.adapterReadiness.findings.map((entry) => entry.caseId),
  [...registryCases.keys()].sort());

const adapterTemplate = adaptersFor(first);
const adapterTemplateSnapshot = adapterTemplate.map(({ execute: _execute, ...entry }) => clone(entry));
const ready = assessP158AdapterReadiness({ schedule: first, adapters: adapterTemplate });
assert.equal(ready.ready, true);
assert.equal(ready.readyCaseCount, 54);
assert.equal(ready.findingCount, 0);
assert.deepEqual(adapterTemplate.map(({ execute: _execute, ...entry }) => entry), adapterTemplateSnapshot,
  'readiness checks must not mutate adapters');

const missingTwo = assessP158AdapterReadiness({
  schedule: first,
  adapters: adapterTemplate.filter((entry) => !['A01', 'A02'].includes(entry.caseId)),
});
assert.deepEqual(missingTwo.findings.map((entry) => [entry.code, entry.caseId]), [
  ['missing_case_adapter', 'A01'],
  ['missing_case_adapter', 'A02'],
]);
assert.equal(missingTwo.readyCaseCount, 52);

const baseA01 = adapterTemplate.find((entry) => entry.caseId === 'A01');
const adapterDefects = [
  ['adapter_case_mismatch', { ...baseA01, adapterId: 'p158.case.wrong.v1' }],
  ['adapter_effect_contract_mismatch', { ...baseA01, declaredEffectIds: ['undeclared'] }],
  ['adapter_evidence_profile_mismatch', { ...baseA01, evidenceProfile: 'logging' }],
  ['adapter_execute_missing', { ...baseA01, execute: null }],
  ['adapter_repair_capability_forbidden', { ...baseA01, reactionaryRepairAllowed: true }],
  ['adapter_undeclared_effect_capability_forbidden', { ...baseA01, undeclaredEffectsAllowed: true }],
  ['adapter_retry_capability_forbidden', { ...baseA01, opportunisticRetryAllowed: true }],
];
for (const [expectedCode, defective] of adapterDefects) {
  const report = assessP158AdapterReadiness({
    schedule: first,
    adapters: adapterTemplate.map((entry) => entry.caseId === 'A01' ? defective : entry),
  });
  assert.deepEqual(findingCodes(report), [expectedCode]);
}
const duplicate = assessP158AdapterReadiness({
  schedule: first,
  adapters: [...adapterTemplate, baseA01],
});
assert.deepEqual(findingCodes(duplicate), ['duplicate_case_adapter']);
const unexpected = assessP158AdapterReadiness({
  schedule: first,
  adapters: [...adapterTemplate, createP158CaseAdapter({
    caseId: 'Z99',
    evidenceProfile: 'agent',
    execute: async () => ({ resultState: 'passed' }),
  })],
});
assert.deepEqual(findingCodes(unexpected), ['unexpected_case_adapter']);
assert.equal(unexpected.readyCaseCount, 54);
assert.deepEqual(P158_ADAPTER_READINESS_CODES, [
  'adapter_case_mismatch',
  'adapter_effect_contract_mismatch',
  'adapter_evidence_profile_mismatch',
  'adapter_execute_missing',
  'adapter_repair_capability_forbidden',
  'adapter_retry_capability_forbidden',
  'adapter_undeclared_effect_capability_forbidden',
  'duplicate_case_adapter',
  'missing_case_adapter',
  'unexpected_case_adapter',
]);

const alternate = compileP158ExecutionSchedule({ registry, seed: 'different-seed' });
assert.notEqual(alternate.scheduleSha256, first.scheduleSha256);
assert.notDeepEqual(alternate.attempts.map((entry) => entry.seed),
  first.attempts.map((entry) => entry.seed));

for (const [code, mutate] of [
  ['registry_not_frozen', (draft) => { draft.registryState = 'draft'; }],
  ['registry_case_count_mismatch', (draft) => { draft.cases.pop(); }],
  ['unknown_case_dependency', (draft) => { draft.cases[0].dependsOn = ['Z99']; }],
  ['cyclic_case_dependency', (draft) => {
    draft.cases.find((entry) => entry.id === 'A01').dependsOn = ['A02'];
  }],
  ['evidence_profile_missing', (draft) => { draft.cases[0].evidenceProfile = 'absent'; }],
  ['execution_bound_missing', (draft) => { draft.cases[0].executionBound = ''; }],
  ['case_environment_missing', (draft) => { draft.cases[0].environmentIds = []; }],
  ['duplicate_case_environment', (draft) => { draft.cases[0].environmentIds.push('E0'); }],
  ['unknown_case_environment', (draft) => { draft.cases[0].environmentIds = ['E9']; }],
]) {
  const draft = clone(registry);
  mutate(draft);
  expectScheduleError(code, () => compileP158ExecutionSchedule({ registry: draft, seed: 'x' }));
}
expectScheduleError('seed_missing', () => compileP158ExecutionSchedule({ registry, seed: '' }));

const { schedule: executableSchedule, adapters: executableAdapters } = compileReady(
  'executor-seed',
  {
    H01: async ({ attempt, requestEffect }) => {
      const receipt = await requestEffect('p158.effect.H01.declared', {
        caseId: attempt.caseId,
        operation: 'synthetic-observation',
      });
      return { resultState: 'passed', receipt };
    },
  },
);
assert.equal(executableSchedule.adapterReadiness.ready, true);
const effectCalls = [];
const executor = createP158AdapterExecutor({
  schedule: executableSchedule,
  adapters: executableAdapters,
  effects: {
    'p158.effect.H01.declared': async (payload, attempt) => {
      effectCalls.push({ payload, attempt });
      return { observed: true };
    },
  },
});
await expectScheduleRejection('attempt_dependency_incomplete', () =>
  executor.executeAttempt('H02-E0-r001'));
const h01 = await executor.executeAttempt('H01-E2-r001');
assert.equal(h01.resultState, 'passed');
assert.equal(h01.requestedEffects.length, 1);
assert.equal(h01.requestedEffects[0].effectId, 'p158.effect.H01.declared');
assert.equal(effectCalls.length, 1);
assert.equal(effectCalls[0].attempt.attemptId, 'H01-E2-r001');
await executor.executeAttempt('H02-E0-r001');
await expectScheduleRejection('opportunistic_retry_prohibited', () =>
  executor.executeAttempt('H01-E2-r001'));
await expectScheduleRejection('unscheduled_attempt', () =>
  executor.executeAttempt('not-scheduled'));

expectScheduleError('adapters_not_ready', () => createP158AdapterExecutor({
  schedule: first,
  adapters: [],
}));

const undeclared = compileReady('undeclared-seed', {
  A01: async ({ requestEffect }) => {
    await requestEffect('p158.effect.repair.undeclared');
    return { resultState: 'passed' };
  },
});
const undeclaredExecutor = createP158AdapterExecutor(undeclared);
await expectScheduleRejection('undeclared_effect_prohibited', () =>
  undeclaredExecutor.executeAttempt('A01-E0-r001'));
assert.equal(undeclaredExecutor.outcomes.get('A01-E0-r001').resultState, 'harness_failure');
await expectScheduleRejection('opportunistic_retry_prohibited', () =>
  undeclaredExecutor.executeAttempt('A01-E0-r001'));

const missingDriver = compileReady('missing-driver-seed', {
  A01: async ({ requestEffect }) => {
    await requestEffect('p158.effect.A01.declared');
    return { resultState: 'passed' };
  },
});
await expectScheduleRejection('effect_driver_missing', () =>
  createP158AdapterExecutor(missingDriver).executeAttempt('A01-E0-r001'));

const invalidResult = compileReady('invalid-result-seed', {
  A01: async () => ({ resultState: 'retrying' }),
});
await expectScheduleRejection('adapter_result_invalid', () =>
  createP158AdapterExecutor(invalidResult).executeAttempt('A01-E0-r001'));

console.log(JSON.stringify({
  ok: true,
  caseCount: first.caseCount,
  attemptCount: first.attemptCount,
  phaseCount: first.phases.length,
  adapterReadinessCodeCount: P158_ADAPTER_READINESS_CODES.length,
}, null, 2));
