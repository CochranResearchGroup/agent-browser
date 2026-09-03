import { sha256 } from './p158-campaign-controller.js';

export const P158_EXECUTION_PHASES = Object.freeze([
  Object.freeze({ phaseId: 'W7', casePrefixes: Object.freeze(['A', 'X']) }),
  Object.freeze({ phaseId: 'W8', casePrefixes: Object.freeze(['H', 'D']) }),
  Object.freeze({ phaseId: 'W9', casePrefixes: Object.freeze(['C']) }),
]);

export const P158_ADAPTER_READINESS_CODES = Object.freeze([
  'adapter_case_mismatch',
  'adapter_effect_contract_mismatch',
  'adapter_evidence_profile_mismatch',
  'adapter_execution_contract_mismatch',
  'adapter_execute_missing',
  'adapter_repair_capability_forbidden',
  'adapter_retry_capability_forbidden',
  'adapter_undeclared_effect_capability_forbidden',
  'duplicate_case_adapter',
  'missing_case_adapter',
  'unexpected_case_adapter',
]);

const RESULT_STATES = new Set([
  'passed',
  'reproduced_historical_failure',
  'new_product_failure',
  'harness_failure',
  'inconclusive',
  'skipped_blocked',
  'safety_stopped',
]);

export class P158ExecutionScheduleError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158ExecutionScheduleError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158ExecutionScheduleError(code, message, details);
}

function adapterId(caseId) {
  return `p158.case.${caseId}.v1`;
}

function effectId(caseId) {
  return `p158.effect.${caseId}.declared`;
}

function phaseId(caseId) {
  const phase = P158_EXECUTION_PHASES.find((candidate) =>
    candidate.casePrefixes.includes(caseId[0]));
  if (!phase) fail('case_phase_missing', `P158 case ${caseId} has no W7 to W9 phase`);
  return phase.phaseId;
}

function phaseSequence(caseId) {
  return P158_EXECUTION_PHASES.findIndex((phase) => phase.casePrefixes.includes(caseId[0]));
}

function topologicalCases(cases) {
  const byId = new Map();
  for (const testCase of cases) {
    if (byId.has(testCase.id)) fail('duplicate_case_id', `Duplicate P158 case ${testCase.id}`);
    byId.set(testCase.id, testCase);
  }
  for (const testCase of cases) {
    for (const dependency of testCase.dependsOn ?? []) {
      if (!byId.has(dependency)) {
        fail('unknown_case_dependency', `${testCase.id} depends on unknown case ${dependency}`);
      }
    }
  }
  const pending = new Set(byId.keys());
  const ordered = [];
  while (pending.size > 0) {
    const ready = [...pending]
      .filter((id) => (byId.get(id).dependsOn ?? []).every((dependency) => !pending.has(dependency)))
      .sort((left, right) => phaseSequence(left) - phaseSequence(right) || left.localeCompare(right));
    if (ready.length === 0) {
      fail('cyclic_case_dependency', 'P158 registry contains a dependency cycle', {
        pending: [...pending].sort(),
      });
    }
    for (const id of ready) {
      pending.delete(id);
      ordered.push(byId.get(id));
    }
  }
  return ordered;
}

function externalIngressRequired(testCase, environmentIds) {
  const caseId = testCase.caseId ?? testCase.id;
  return environmentIds.includes('E2') && (
    ['external', 'dashboard', 'combined'].includes(testCase.evidenceProfile) ||
    ['A15', 'X06', 'X10'].includes(caseId)
  );
}

function validateExecutionContract(testCase) {
  const contract = testCase.executionContract;
  if (contract?.schemaVersion !== 'agent-browser.p158-case-execution-contract.v1') {
    fail('execution_contract_missing', `${testCase.id} has no machine-readable execution contract`);
  }
  if (!['separate', 'combined'].includes(contract.environmentMode)) {
    fail('execution_contract_environment_mode_invalid',
      `${testCase.id} has an invalid execution environment mode`);
  }
  const expansion = contract.expansion;
  if (!['aggregate', 'repeat', 'dimension'].includes(expansion?.strategy) ||
      !Number.isInteger(expansion?.count) || expansion.count < 1 ||
      typeof expansion?.unit !== 'string' || expansion.unit.length === 0) {
    fail('execution_contract_expansion_invalid', `${testCase.id} has an invalid attempt expansion`);
  }
  if (expansion.strategy === 'aggregate' && expansion.count !== 1) {
    fail('execution_contract_expansion_invalid', `${testCase.id} aggregate expansion must have count 1`);
  }
  if (!Array.isArray(contract.dimensions) || !Array.isArray(contract.cardinalities)) {
    fail('execution_contract_shape_invalid', `${testCase.id} execution dimensions are incomplete`);
  }
  const dimensionIds = new Set();
  for (const dimension of contract.dimensions) {
    if (typeof dimension.id !== 'string' || dimensionIds.has(dimension.id) ||
        !['each', 'cartesian'].includes(dimension.coverage) ||
        !Array.isArray(dimension.values) || dimension.values.length === 0 ||
        new Set(dimension.values).size !== dimension.values.length) {
      fail('execution_contract_dimension_invalid', `${testCase.id} has an invalid dimension`);
    }
    dimensionIds.add(dimension.id);
  }
  if (expansion.strategy === 'dimension') {
    const expandedDimension = contract.dimensions.find(
      (dimension) => dimension.id === expansion.dimensionId,
    );
    if (!expandedDimension || expandedDimension.values.length !== expansion.count) {
      fail('execution_contract_expansion_invalid',
        `${testCase.id} dimension expansion does not match its declared values`);
    }
  }
  const cardinalityIds = new Set();
  for (const cardinality of contract.cardinalities) {
    if (typeof cardinality.id !== 'string' || cardinalityIds.has(cardinality.id) ||
        !['exact', 'minimum', 'capacity'].includes(cardinality.mode) ||
        !['aggregate', 'per_attempt', 'shared'].includes(cardinality.scope) ||
        !Number.isInteger(cardinality.value) || cardinality.value < 1) {
      fail('execution_contract_cardinality_invalid', `${testCase.id} has an invalid cardinality`);
    }
    cardinalityIds.add(cardinality.id);
  }
  if (contract.duration !== undefined && (
    !['exact', 'minimum'].includes(contract.duration?.mode) ||
    !Number.isInteger(contract.duration?.seconds) || contract.duration.seconds < 1
  )) {
    fail('execution_contract_duration_invalid', `${testCase.id} has an invalid duration`);
  }
  if (contract.dimensions.length === 0 && contract.cardinalities.length === 0 &&
      contract.duration === undefined) {
    fail('execution_contract_empty', `${testCase.id} has an empty execution contract`);
  }
  return structuredClone(contract);
}

function cardinalityAllocations(contract, ordinal, attemptId) {
  return contract.cardinalities.map((cardinality) => {
    let assignedValue;
    let firstActionOrdinal = null;
    if (cardinality.scope === 'aggregate') {
      const quotient = Math.floor(cardinality.value / contract.expansion.count);
      const remainder = cardinality.value % contract.expansion.count;
      assignedValue = quotient + (ordinal <= remainder ? 1 : 0);
      firstActionOrdinal = quotient * (ordinal - 1) + Math.min(ordinal - 1, remainder) + 1;
    } else {
      assignedValue = cardinality.value;
      if (cardinality.scope === 'per_attempt') firstActionOrdinal = 1;
    }
    const actionIds = firstActionOrdinal === null
      ? []
      : Array.from({ length: assignedValue }, (_, index) =>
        `${attemptId}:${cardinality.id}:${String(firstActionOrdinal + index).padStart(5, '0')}`);
    return { ...cardinality, assignedValue, actionIds };
  });
}

function normalizeAdapters(adapters) {
  if (adapters === undefined) return [];
  if (!Array.isArray(adapters)) fail('invalid_adapters', 'P158 adapters must be an array');
  return adapters;
}

function finding(code, caseId, field, expected, observed) {
  return { code, caseId, field, expected, observed };
}

export function assessP158AdapterReadiness({ schedule, adapters }) {
  const findings = [];
  const expectedCases = new Map(schedule.caseContracts.map((contract) => [contract.caseId, contract]));
  const byCase = new Map();
  for (const candidate of normalizeAdapters(adapters)) {
    if (!expectedCases.has(candidate.caseId)) {
      findings.push(finding(
        'unexpected_case_adapter', candidate.caseId ?? null, 'caseId',
        [...expectedCases.keys()].sort(), candidate.caseId ?? null,
      ));
      continue;
    }
    if (byCase.has(candidate.caseId)) {
      findings.push(finding(
        'duplicate_case_adapter', candidate.caseId, 'caseId', 'exactly one adapter', candidate.caseId,
      ));
      continue;
    }
    byCase.set(candidate.caseId, candidate);
  }
  for (const [caseId, contract] of expectedCases) {
    const candidate = byCase.get(caseId);
    if (!candidate) {
      findings.push(finding(
        'missing_case_adapter', caseId, 'adapterId', contract.adapterId, null,
      ));
      continue;
    }
    if (candidate.adapterId !== contract.adapterId || candidate.caseId !== contract.caseId) {
      findings.push(finding(
        'adapter_case_mismatch', caseId, 'adapterId', contract.adapterId,
        candidate.adapterId ?? null,
      ));
    }
    if (candidate.evidenceProfile !== contract.evidenceProfile) {
      findings.push(finding(
        'adapter_evidence_profile_mismatch', caseId, 'evidenceProfile',
        contract.evidenceProfile, candidate.evidenceProfile ?? null,
      ));
    }
    if (candidate.executionContractSha256 !== contract.executionContractSha256) {
      findings.push(finding(
        'adapter_execution_contract_mismatch', caseId, 'executionContractSha256',
        contract.executionContractSha256, candidate.executionContractSha256 ?? null,
      ));
    }
    const actualEffectIds = Array.isArray(candidate.declaredEffectIds)
      ? [...candidate.declaredEffectIds].sort()
      : [];
    if (JSON.stringify(actualEffectIds) !== JSON.stringify(contract.declaredEffectIds)) {
      findings.push(finding(
        'adapter_effect_contract_mismatch', caseId, 'declaredEffectIds',
        contract.declaredEffectIds, actualEffectIds,
      ));
    }
    if (candidate.reactionaryRepairAllowed !== false) {
      findings.push(finding(
        'adapter_repair_capability_forbidden', caseId, 'reactionaryRepairAllowed', false,
        candidate.reactionaryRepairAllowed ?? null,
      ));
    }
    if (candidate.undeclaredEffectsAllowed !== false) {
      findings.push(finding(
        'adapter_undeclared_effect_capability_forbidden', caseId, 'undeclaredEffectsAllowed', false,
        candidate.undeclaredEffectsAllowed ?? null,
      ));
    }
    if (candidate.opportunisticRetryAllowed !== false) {
      findings.push(finding(
        'adapter_retry_capability_forbidden', caseId, 'opportunisticRetryAllowed', false,
        candidate.opportunisticRetryAllowed ?? null,
      ));
    }
    if (typeof candidate.execute !== 'function') {
      findings.push(finding(
        'adapter_execute_missing', caseId, 'execute', 'function', typeof candidate.execute,
      ));
    }
  }
  findings.sort((left, right) =>
    left.caseId?.localeCompare(right.caseId ?? '') ||
    left.code.localeCompare(right.code) ||
    left.field.localeCompare(right.field));
  return {
    schemaVersion: 'agent-browser.p158-adapter-readiness.v1',
    planId: 'P158',
    scheduleSha256: schedule.scheduleSha256,
    ready: findings.length === 0,
    expectedCaseCount: expectedCases.size,
    readyCaseCount: expectedCases.size - new Set(
      findings
        .filter((entry) => expectedCases.has(entry.caseId))
        .map((entry) => entry.caseId),
    ).size,
    findingCount: findings.length,
    findings,
  };
}

export function compileP158ExecutionSchedule({ registry, seed, adapters }) {
  if (registry?.registryState !== 'frozen') {
    fail('registry_not_frozen', 'P158 execution schedule requires the frozen registry');
  }
  if (!Array.isArray(registry.cases) || registry.cases.length !== 54) {
    fail('registry_case_count_mismatch', 'P158 execution schedule requires exactly 54 cases', {
      observed: registry?.cases?.length ?? null,
    });
  }
  if (seed === undefined || seed === null || String(seed).length === 0) {
    fail('seed_missing', 'P158 execution schedule requires a deterministic seed');
  }
  const evidenceProfiles = new Set(Object.keys(registry.evidenceProfiles ?? {}));
  const environmentIds = new Set(Object.keys(registry.environments ?? {}));
  const orderedCases = topologicalCases(registry.cases);
  const caseContracts = orderedCases.map((testCase) => {
    if (!evidenceProfiles.has(testCase.evidenceProfile)) {
      fail('evidence_profile_missing', `${testCase.id} has no named evidence profile`, {
        evidenceProfile: testCase.evidenceProfile ?? null,
      });
    }
    if (typeof testCase.executionBound !== 'string' || testCase.executionBound.length === 0) {
      fail('execution_bound_missing', `${testCase.id} has no declared execution bound`);
    }
    const executionContract = validateExecutionContract(testCase);
    if (!Array.isArray(testCase.environmentIds) || testCase.environmentIds.length === 0) {
      fail('case_environment_missing', `${testCase.id} has no declared environment`);
    }
    if (new Set(testCase.environmentIds).size !== testCase.environmentIds.length) {
      fail('duplicate_case_environment', `${testCase.id} declares an environment more than once`);
    }
    for (const environmentId of testCase.environmentIds) {
      if (!environmentIds.has(environmentId)) {
        fail('unknown_case_environment', `${testCase.id} declares unknown environment ${environmentId}`);
      }
    }
    return {
      caseId: testCase.id,
      phaseId: phaseId(testCase.id),
      evidenceProfile: testCase.evidenceProfile,
      executionBound: testCase.executionBound,
      executionBoundSha256: sha256(testCase.executionBound),
      executionContract,
      executionContractSha256: sha256(executionContract),
      environmentIds: [...new Set(testCase.environmentIds)].sort(),
      dependsOnCaseIds: [...new Set(testCase.dependsOn ?? [])].sort(),
      adapterId: adapterId(testCase.id),
      declaredEffectIds: [effectId(testCase.id)],
      reactionaryRepairAllowed: false,
      opportunisticRetryAllowed: false,
      undeclaredEffectsAllowed: false,
    };
  });
  const caseOrder = new Map(caseContracts.map((contract, index) => [contract.caseId, index]));
  const attempts = caseContracts.flatMap((contract) => {
    const workloads = contract.executionContract.environmentMode === 'combined'
      ? [contract.environmentIds]
      : contract.environmentIds.map((environmentId) => [environmentId]);
    return workloads.flatMap((workloadEnvironmentIds) => Array.from(
      { length: contract.executionContract.expansion.count },
      (_, index) => {
        const repetition = index + 1;
        const suffix = `r${String(repetition).padStart(3, '0')}`;
        const environmentKey = workloadEnvironmentIds.join('_');
        const attemptId = `${contract.caseId}-${environmentKey}-${suffix}`;
        const expandedDimension = contract.executionContract.expansion.strategy === 'dimension'
          ? contract.executionContract.dimensions.find(
            (dimension) => dimension.id === contract.executionContract.expansion.dimensionId,
          )
          : null;
        return {
          caseId: contract.caseId,
          attemptId,
          phaseId: contract.phaseId,
          environmentId: workloadEnvironmentIds[0],
          environmentIds: [...workloadEnvironmentIds],
          repetition,
          seed: Number.parseInt(
            sha256(`${seed}\0${contract.caseId}\0${environmentKey}\0${suffix}`).slice(0, 13),
            16,
          ),
          evidenceProfile: contract.evidenceProfile,
          executionBound: contract.executionBound,
          executionBoundSha256: contract.executionBoundSha256,
          executionContractSha256: contract.executionContractSha256,
          executionUnit: {
            strategy: contract.executionContract.expansion.strategy,
            unit: contract.executionContract.expansion.unit,
            ordinal: repetition,
            count: contract.executionContract.expansion.count,
            plannedOffsetSeconds: contract.executionContract.duration
              ? Math.floor(
                (repetition - 1) * contract.executionContract.duration.seconds /
                Math.max(contract.executionContract.expansion.count - 1, 1),
              )
              : null,
            dimensionAssignment: expandedDimension
              ? { dimensionId: expandedDimension.id, value: expandedDimension.values[index] }
              : null,
          },
          cardinalityAllocations: cardinalityAllocations(
            contract.executionContract,
            repetition,
            attemptId,
          ),
          adapterId: contract.adapterId,
          declaredEffectIds: contract.declaredEffectIds,
          externalIngressRequired: externalIngressRequired(contract, workloadEnvironmentIds),
          dependsOnCaseIds: contract.dependsOnCaseIds,
        };
      },
    ));
  });
  const attemptsByCase = new Map(caseContracts.map((contract) => [
    contract.caseId,
    attempts.filter((attempt) => attempt.caseId === contract.caseId).map((attempt) => attempt.attemptId),
  ]));
  attempts.sort((left, right) =>
    caseOrder.get(left.caseId) - caseOrder.get(right.caseId) ||
    left.environmentIds.join(',').localeCompare(right.environmentIds.join(',')) ||
    left.repetition - right.repetition);
  for (const [sequence, attempt] of attempts.entries()) {
    attempt.scheduleSequence = sequence;
    attempt.scheduleId = `${attempt.phaseId}:${attempt.attemptId}`;
    attempt.dependsOnAttemptIds = attempt.dependsOnCaseIds
      .flatMap((dependency) => attemptsByCase.get(dependency))
      .sort();
    delete attempt.dependsOnCaseIds;
  }
  const phases = P158_EXECUTION_PHASES.map((phase, sequence) => {
    const phaseAttempts = attempts.filter((attempt) => attempt.phaseId === phase.phaseId);
    return {
      phaseId: phase.phaseId,
      phaseSequence: sequence,
      caseIds: [...new Set(phaseAttempts.map((attempt) => attempt.caseId))],
      attemptIds: phaseAttempts.map((attempt) => attempt.attemptId),
      attemptCount: phaseAttempts.length,
    };
  });
  const environments = [...environmentIds].sort().map((environmentId) => {
    const environmentAttempts = attempts.filter((attempt) =>
      attempt.environmentIds.includes(environmentId));
    return {
      environmentId,
      description: registry.environments[environmentId],
      caseIds: [...new Set(environmentAttempts.map((attempt) => attempt.caseId))],
      attemptIds: environmentAttempts.map((attempt) => attempt.attemptId),
      externalIngressAttemptIds: environmentAttempts
        .filter((attempt) => attempt.externalIngressRequired)
        .map((attempt) => attempt.attemptId),
      attemptCount: environmentAttempts.length,
    };
  });
  const scheduleBody = {
    schemaVersion: 'agent-browser.p158-execution-schedule.v1',
    planId: 'P158',
    registrySha256: sha256(registry),
    masterSeed: String(seed),
    caseCount: caseContracts.length,
    attemptCount: attempts.length,
    phases,
    environments,
    caseContracts,
    attempts,
    repairAllowed: false,
    opportunisticRetryAllowed: false,
    undeclaredEffectsAllowed: false,
  };
  const schedule = { ...scheduleBody, scheduleSha256: sha256(scheduleBody) };
  return {
    ...schedule,
    adapterReadiness: assessP158AdapterReadiness({ schedule, adapters }),
  };
}

export function compileP158ControllerScheduleInput({ registry, seed, adapters }) {
  const executionSchedule = compileP158ExecutionSchedule({ registry, seed, adapters });
  if (!executionSchedule.adapterReadiness.ready) {
    fail('adapters_not_ready', 'P158 live freeze requires all case adapters before preparation', {
      findings: executionSchedule.adapterReadiness.findings,
    });
  }
  return {
    executionSchedule,
    controllerSchedule: executionSchedule.attempts.map((attempt) => ({
      caseId: attempt.caseId,
      attemptId: attempt.attemptId,
      environmentId: attempt.environmentId,
      environmentIds: [...attempt.environmentIds],
      seed: attempt.seed,
      dependsOn: [...attempt.dependsOnAttemptIds],
    })),
  };
}

export function createP158CaseAdapter({ caseId, evidenceProfile, executionContract, execute }) {
  return {
    adapterId: adapterId(caseId),
    caseId,
    evidenceProfile,
    executionContractSha256: executionContract === undefined ? null : sha256(executionContract),
    declaredEffectIds: [effectId(caseId)],
    reactionaryRepairAllowed: false,
    opportunisticRetryAllowed: false,
    undeclaredEffectsAllowed: false,
    execute,
  };
}

export function createP158AdapterExecutor({ schedule, adapters, effects = {} }) {
  const readiness = assessP158AdapterReadiness({ schedule, adapters });
  if (!readiness.ready) {
    fail('adapters_not_ready', 'P158 execution adapters are incomplete before freeze', {
      findings: readiness.findings,
    });
  }
  const attempts = new Map(schedule.attempts.map((attempt) => [attempt.attemptId, attempt]));
  const adapterByCase = new Map(adapters.map((adapter) => [adapter.caseId, adapter]));
  const outcomes = new Map();
  const started = new Set();
  return {
    readiness,
    outcomes,
    async executeAttempt(attemptId) {
      const attempt = attempts.get(attemptId);
      if (!attempt) fail('unscheduled_attempt', `Attempt ${attemptId} is not scheduled`);
      if (started.has(attemptId)) {
        fail('opportunistic_retry_prohibited', `Attempt ${attemptId} cannot execute twice`);
      }
      const incompleteDependencies = attempt.dependsOnAttemptIds
        .filter((dependency) => !outcomes.has(dependency));
      if (incompleteDependencies.length > 0) {
        fail('attempt_dependency_incomplete', `${attemptId} has incomplete dependencies`, {
          incompleteDependencies,
        });
      }
      started.add(attemptId);
      const adapter = adapterByCase.get(attempt.caseId);
      const requestedEffects = [];
      const requestEffect = async (requestedEffectId, payload = {}) => {
        if (!attempt.declaredEffectIds.includes(requestedEffectId)) {
          fail('undeclared_effect_prohibited', `${requestedEffectId} is not declared for ${attemptId}`, {
            declaredEffectIds: attempt.declaredEffectIds,
          });
        }
        const driver = effects[requestedEffectId];
        if (typeof driver !== 'function') {
          fail('effect_driver_missing', `No provider-free driver was supplied for ${requestedEffectId}`);
        }
        const result = await driver(structuredClone(payload), structuredClone(attempt));
        requestedEffects.push({ effectId: requestedEffectId, payloadSha256: sha256(payload) });
        return result;
      };
      let result;
      try {
        result = await adapter.execute({ attempt: structuredClone(attempt), requestEffect });
      } catch (error) {
        outcomes.set(attemptId, {
          resultState: 'harness_failure',
          errorName: error?.name ?? 'Error',
          errorMessage: error?.message ?? String(error),
          requestedEffects,
        });
        throw error;
      }
      if (!RESULT_STATES.has(result?.resultState)) {
        fail('adapter_result_invalid', `${attempt.adapterId} returned an invalid terminal result`, {
          resultState: result?.resultState ?? null,
        });
      }
      const outcome = { ...structuredClone(result), requestedEffects };
      outcomes.set(attemptId, outcome);
      return structuredClone(outcome);
    },
  };
}
