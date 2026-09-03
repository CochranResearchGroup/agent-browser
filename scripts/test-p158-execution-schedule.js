#!/usr/bin/env node

import assert from 'node:assert/strict';
import fs from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';
import {
  P158_ADAPTER_READINESS_CODES,
  P158ExecutionScheduleError,
  assessP158AdapterReadiness,
  compileP158ControllerScheduleInput,
  compileP158ExecutionSchedule,
  createP158AdapterExecutor,
  createP158CaseAdapter,
} from './lib/p158-execution-schedule.js';

const root = new URL('..', import.meta.url);
const readJson = (path) => JSON.parse(fs.readFileSync(new URL(path, root), 'utf8'));
const registry = readJson('docs/dev/contracts/p158-historical-failure-registry.v1.json');
const executionContractSchema = readJson(
  'docs/dev/contracts/p158-case-execution-contract.v1.schema.json',
);
const serviceRequestSchema = readJson('docs/dev/contracts/service-request.v1.schema.json');
const ajv = new Ajv2020({ allErrors: true, strict: true });
const validateExecutionContract = ajv.compile(executionContractSchema);

function expectError(code, action) {
  assert.throws(action, (error) => {
    assert(error instanceof P158ExecutionScheduleError);
    assert.equal(error.code, code);
    return true;
  });
}

async function expectRejection(code, action) {
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
    executionContract: contract.executionContract,
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

function attemptsFor(schedule, caseId, environmentId) {
  return schedule.attempts.filter((attempt) =>
    attempt.caseId === caseId && attempt.environmentIds.includes(environmentId));
}

function cardinality(contract, id) {
  const value = contract.cardinalities.find((entry) => entry.id === id);
  assert(value, `${id} cardinality is missing`);
  return value;
}

function assignedTotal(attempts, id) {
  return attempts.reduce((total, attempt) => total +
    attempt.cardinalityAllocations.find((entry) => entry.id === id).assignedValue, 0);
}

const originalRegistry = structuredClone(registry);
for (const testCase of registry.cases) {
  assert.equal(
    validateExecutionContract(testCase.executionContract),
    true,
    `${testCase.id}: ${ajv.errorsText(validateExecutionContract.errors)}`,
  );
}

const first = compileP158ExecutionSchedule({ registry, seed: 'frozen-seed' });
const second = compileP158ExecutionSchedule({ registry, seed: 'frozen-seed' });
assert.deepEqual(first, second);
assert.deepEqual(registry, originalRegistry, 'compiler mutated the frozen registry');
assert.equal(first.caseCount, 54);
assert.equal(first.attemptCount, 1592);
assert.equal(new Set(first.attempts.map((entry) => entry.attemptId)).size, 1592);
assert.equal(new Set(first.attempts.map((entry) => entry.seed)).size, 1592);
assert.deepEqual([...new Set(first.attempts.map((entry) => entry.phaseId))], ['W7', 'W8', 'W9']);
assert.deepEqual(first.environments.map((entry) => entry.environmentId), ['E0', 'E1', 'E2', 'E3']);

const registryCases = new Map(registry.cases.map((entry) => [entry.id, entry]));
const scheduleSequence = new Map(first.attempts.map((entry) => [
  entry.attemptId,
  entry.scheduleSequence,
]));
const everyActionId = [];
for (const contract of first.caseContracts) {
  const source = registryCases.get(contract.caseId);
  assert.deepEqual(contract.executionContract, source.executionContract);
  assert.equal(typeof contract.executionContractSha256, 'string');
  const workloads = contract.executionContract.environmentMode === 'combined'
    ? [contract.environmentIds]
    : contract.environmentIds.map((environmentId) => [environmentId]);
  for (const workloadEnvironmentIds of workloads) {
    const attempts = first.attempts.filter((attempt) =>
      attempt.caseId === contract.caseId &&
      JSON.stringify(attempt.environmentIds) === JSON.stringify(workloadEnvironmentIds));
    const environmentKey = workloadEnvironmentIds.join('_');
    assert.equal(attempts.length, contract.executionContract.expansion.count, contract.caseId);
    assert(attempts.every((attempt) =>
      JSON.stringify(attempt.environmentIds) === JSON.stringify(workloadEnvironmentIds)));
    assert.deepEqual(attempts.map((entry) => entry.repetition),
      Array.from({ length: attempts.length }, (_, index) => index + 1));
    assert.deepEqual(attempts.map((entry) => entry.attemptId),
      Array.from({ length: attempts.length }, (_, index) =>
        `${contract.caseId}-${environmentKey}-r${String(index + 1).padStart(3, '0')}`));
    for (const declared of contract.executionContract.cardinalities) {
      const allocations = attempts.map((attempt) =>
        attempt.cardinalityAllocations.find((entry) => entry.id === declared.id));
      assert(allocations.every(Boolean), `${contract.caseId} omitted ${declared.id} allocation`);
      if (declared.scope === 'aggregate') {
        assert.equal(assignedTotal(attempts, declared.id), declared.value,
          `${contract.caseId}/${environmentKey}/${declared.id} multiplied its aggregate`);
      } else {
        assert(allocations.every((entry) => entry.assignedValue === declared.value));
      }
      if (declared.scope === 'shared') {
        assert(allocations.every((entry) => entry.actionIds.length === 0));
      }
      everyActionId.push(...allocations.flatMap((entry) => entry.actionIds));
    }
    if (contract.executionContract.expansion.strategy === 'dimension') {
      const dimension = contract.executionContract.dimensions.find((entry) =>
        entry.id === contract.executionContract.expansion.dimensionId);
      assert.deepEqual(attempts.map((entry) => entry.executionUnit.dimensionAssignment.value),
        dimension.values);
    }
  }
}
assert.equal(new Set(everyActionId).size, everyActionId.length,
  'every predetermined action must have a globally distinct action ID');

for (const attempt of first.attempts) {
  const source = registryCases.get(attempt.caseId);
  const expectedDependencies = source.dependsOn.flatMap((caseId) =>
    first.attempts.filter((entry) => entry.caseId === caseId).map((entry) => entry.attemptId)).sort();
  assert.deepEqual(attempt.dependsOnAttemptIds, expectedDependencies);
  assert(attempt.dependsOnAttemptIds.every((id) =>
    scheduleSequence.get(id) < attempt.scheduleSequence));
  const expectedIngress = attempt.environmentIds.includes('E2') && (
    ['external', 'dashboard', 'combined'].includes(source.evidenceProfile) ||
    ['A15', 'X06', 'X10'].includes(source.id)
  );
  assert.equal(attempt.externalIngressRequired, expectedIngress, attempt.attemptId);
}

const expandedCases = {
  A02: 20, A05: 6, A13: 25, A14: 5,
  H03: 4, H05: 3, H09: 7, H10: 3, H12: 500,
  X07: 25, X08: 6, X10: 6, D02: 8,
  C01: 10, C02: 100, C03: 25, C04: 200, C05: 500,
};
for (const testCase of registry.cases) {
  assert.equal(testCase.executionContract.expansion.count, expandedCases[testCase.id] ?? 1,
    `${testCase.id} expansion drifted`);
}
for (const [caseId, environmentId, repetitions] of [
  ['A02', 'E0', 20], ['A02', 'E1', 20], ['A13', 'E1', 25],
  ['H12', 'E2', 500], ['C05', 'E2', 500],
]) {
  assert.equal(attemptsFor(first, caseId, environmentId).length, repetitions);
}
assert.equal(assignedTotal(attemptsFor(first, 'H12', 'E2'), 'reconnects'), 500);
assert.equal(assignedTotal(attemptsFor(first, 'C05', 'E2'), 'reconnects'), 500);
assert.deepEqual(registryCases.get('H12').executionContract.duration,
  { mode: 'minimum', seconds: 86400 });
assert.deepEqual(registryCases.get('C05').executionContract.duration,
  { mode: 'minimum', seconds: 86400 });
for (const [caseId, environmentId, lastOffset] of [
  ['H12', 'E2', 86400],
  ['C01', 'E1', 1200],
  ['C04', 'E2', 28800],
  ['C05', 'E2', 86400],
]) {
  const attempts = attemptsFor(first, caseId, environmentId);
  assert.equal(attempts[0].executionUnit.plannedOffsetSeconds, 0);
  assert.equal(attempts.at(-1).executionUnit.plannedOffsetSeconds, lastOffset);
  assert(attempts.every((attempt, index) => index === 0 ||
    attempt.executionUnit.plannedOffsetSeconds >=
      attempts[index - 1].executionUnit.plannedOffsetSeconds));
}

const c01 = first.attempts.filter((attempt) => attempt.caseId === 'C01');
assert.equal(c01.length, 10);
assert(c01.every((attempt) =>
  JSON.stringify(attempt.environmentIds) === JSON.stringify(['E1', 'E2'])));
assert.equal(assignedTotal(c01, 'service_commands'), 500);
assert.equal(assignedTotal(c01, 'dashboard_actions'), 50);
assert.equal(assignedTotal(c01, 'reconnects'), 10);
for (const [caseId, repetitions, totals] of [
  ['C02', 100, { service_commands: 2000, dashboard_actions: 500, reconnects: 100, browser_crashes: 20 }],
  ['C04', 200, { service_commands: 10000, dashboard_actions: 2000, reconnects: 200, browser_crashes: 50 }],
]) {
  const combinedAttempts = first.attempts.filter((attempt) => attempt.caseId === caseId);
  assert.equal(combinedAttempts.length, repetitions);
  assert(combinedAttempts.every((attempt) =>
    JSON.stringify(attempt.environmentIds) === JSON.stringify(['E1', 'E2'])));
  for (const [cardinalityId, total] of Object.entries(totals)) {
    assert.equal(assignedTotal(combinedAttempts, cardinalityId), total);
  }
}
const a15 = first.attempts.filter((attempt) => attempt.caseId === 'A15');
assert.equal(a15.length, 1);
assert.deepEqual(a15[0].environmentIds, ['E0', 'E1', 'E2', 'E3']);

const d09 = registryCases.get('D09').executionContract;
assert.equal(cardinality(d09, 'profiles').value, 100);
assert.equal(cardinality(d09, 'browsers_or_historical_rows').value, 500);
assert.equal(cardinality(d09, 'tabs').value, 2000);
assert.equal(cardinality(d09, 'jobs').value, 10000);
assert.equal(cardinality(d09, 'events').value, 10000);

const matrixCells = Object.fromEntries(['A04', 'A08', 'H02', 'X05', 'D06', 'D07', 'D10', 'D12']
  .map((caseId) => [caseId, registryCases.get(caseId).executionContract.dimensions
    .filter((dimension) => dimension.coverage === 'cartesian')
    .reduce((count, dimension) => count * dimension.values.length, 1)]));
assert.deepEqual(matrixCells, {
  A04: 108, A08: 8, H02: 130, X05: 72, D06: 8, D07: 8, D10: 28, D12: 21,
});
assert.equal(cardinality(registryCases.get('A07').executionContract,
  'supported_service_request_actions').value, serviceRequestSchema.properties.action.enum.length);

assert.equal(first.adapterReadiness.ready, false);
assert.equal(first.adapterReadiness.findingCount, 54);
assert(first.adapterReadiness.findings.every((entry) => entry.code === 'missing_case_adapter'));
const adapters = adaptersFor(first);
assert.equal(assessP158AdapterReadiness({ schedule: first, adapters }).ready, true);
const contractMismatch = assessP158AdapterReadiness({
  schedule: first,
  adapters: adapters.map((entry) => entry.caseId === 'A01'
    ? { ...entry, executionContractSha256: '0'.repeat(64) }
    : entry),
});
assert.deepEqual(contractMismatch.findings.map((entry) => entry.code),
  ['adapter_execution_contract_mismatch']);
const missingTwo = assessP158AdapterReadiness({
  schedule: first,
  adapters: adapters.filter((entry) => !['A01', 'A02'].includes(entry.caseId)),
});
assert.deepEqual(missingTwo.findings.map((entry) => [entry.code, entry.caseId]), [
  ['missing_case_adapter', 'A01'], ['missing_case_adapter', 'A02'],
]);
assert(P158_ADAPTER_READINESS_CODES.includes('adapter_execution_contract_mismatch'));
expectError('adapters_not_ready', () => compileP158ControllerScheduleInput({
  registry,
  seed: 'bridge-seed',
  adapters: [],
}));
const bridged = compileP158ControllerScheduleInput({
  registry,
  seed: 'bridge-seed',
  adapters,
});
assert.equal(bridged.executionSchedule.attemptCount, 1592);
assert.equal(bridged.controllerSchedule.length, 1592);
assert.deepEqual(bridged.controllerSchedule[0], {
  caseId: bridged.executionSchedule.attempts[0].caseId,
  attemptId: bridged.executionSchedule.attempts[0].attemptId,
  environmentId: bridged.executionSchedule.attempts[0].environmentId,
  environmentIds: bridged.executionSchedule.attempts[0].environmentIds,
  seed: bridged.executionSchedule.attempts[0].seed,
  dependsOn: bridged.executionSchedule.attempts[0].dependsOnAttemptIds,
});

for (const [code, mutate] of [
  ['execution_contract_missing', (draft) => { delete draft.cases[0].executionContract; }],
  ['execution_contract_expansion_invalid', (draft) => {
    draft.cases[0].executionContract.expansion.count = 2;
  }],
  ['execution_contract_dimension_invalid', (draft) => {
    draft.cases[0].executionContract.dimensions[0].values.push('sequential');
  }],
  ['execution_contract_cardinality_invalid', (draft) => {
    draft.cases[0].executionContract.cardinalities[0].scope = 'unknown';
  }],
  ['execution_contract_environment_mode_invalid', (draft) => {
    draft.cases[0].executionContract.environmentMode = 'unknown';
  }],
]) {
  const draft = structuredClone(registry);
  mutate(draft);
  expectError(code, () => compileP158ExecutionSchedule({ registry: draft, seed: 'invalid' }));
}

const execution = compileReady('executor-seed', {
  H01: async ({ requestEffect }) => {
    await requestEffect('p158.effect.H01.declared', { operation: 'synthetic' });
    return { resultState: 'passed' };
  },
});
const executor = createP158AdapterExecutor({
  ...execution,
  effects: { 'p158.effect.H01.declared': async () => ({ observed: true }) },
});
await expectRejection('attempt_dependency_incomplete', () =>
  executor.executeAttempt('H02-E0-r001'));
await executor.executeAttempt('H01-E2-r001');
await executor.executeAttempt('H02-E0-r001');
await expectRejection('opportunistic_retry_prohibited', () =>
  executor.executeAttempt('H01-E2-r001'));

const undeclared = compileReady('undeclared-seed', {
  A01: async ({ requestEffect }) => {
    await requestEffect('p158.effect.repair.undeclared');
    return { resultState: 'passed' };
  },
});
const undeclaredExecutor = createP158AdapterExecutor(undeclared);
await expectRejection('undeclared_effect_prohibited', () =>
  undeclaredExecutor.executeAttempt('A01-E0-r001'));
assert.equal(undeclaredExecutor.outcomes.get('A01-E0-r001').resultState, 'harness_failure');
await expectRejection('opportunistic_retry_prohibited', () =>
  undeclaredExecutor.executeAttempt('A01-E0-r001'));

console.log(JSON.stringify({
  ok: true,
  caseCount: first.caseCount,
  attemptCount: first.attemptCount,
  actionIdCount: everyActionId.length,
  matrixCells,
}, null, 2));
