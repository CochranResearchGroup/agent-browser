import { execFile as nodeExecFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { promisify } from 'node:util';

import { createP158CaseAdapter } from './p158-execution-schedule.js';

export const P158_W7_CASE_IDS = Object.freeze([
  ...Array.from({ length: 15 }, (_, index) => `A${String(index + 1).padStart(2, '0')}`),
  ...Array.from({ length: 10 }, (_, index) => `X${String(index + 1).padStart(2, '0')}`),
]);

const CASE_SPECS = Object.freeze({
  A01: { hook: 'cli', stimuli: [] },
  A02: { hook: 'browser', stimuli: [] },
  A03: { hook: 'browser', stimuli: [] },
  A04: { hook: 'cli', stimuli: ['policy_mutation'] },
  A05: { hook: 'cli', stimuli: ['policy_transition'] },
  A06: { hook: 'browser', stimuli: ['revocation', 'eviction'] },
  A07: { hook: 'process', stimuli: ['browser_crash'] },
  A08: { hook: 'browser', stimuli: ['identity_fixture'] },
  A09: { hook: 'browser', stimuli: ['target_pathology'] },
  A10: { hook: 'browser', stimuli: ['inventory_churn'] },
  A11: { hook: 'cli', stimuli: ['scheduler_fault'] },
  A12: { hook: 'cli', stimuli: ['lock_or_timeout'] },
  A13: { hook: 'systemd', stimuli: ['daemon_transition', 'supervisor_transition'] },
  A14: { hook: 'shutdown', stimuli: ['full_shutdown'] },
  A15: { hook: 'browser', stimuli: ['history_marker'] },
  X01: { hook: 'display', stimuli: ['xvfb_orphan'] },
  X02: { hook: 'display', stimuli: ['allocator_race'] },
  X03: { hook: 'display', stimuli: ['display_evidence_fault'] },
  X04: { hook: 'display', stimuli: ['display_exhaustion'] },
  X05: { hook: 'display', stimuli: ['x11_authority_fixture'] },
  X06: { hook: 'display', stimuli: ['desktop_locator_state'] },
  X07: { hook: 'systemd', stimuli: ['supervisor_transition'] },
  X08: { hook: 'shutdown', stimuli: ['preserve_transition', 'full_shutdown'] },
  X09: { hook: 'cli', stimuli: ['generation_mismatch'] },
  X10: { hook: 'systemd', stimuli: ['service_restart', 'host_restart'] },
});

const RESULT_STATES = new Set([
  'passed',
  'reproduced_historical_failure',
  'new_product_failure',
  'harness_failure',
  'inconclusive',
  'skipped_blocked',
  'safety_stopped',
]);

const RESULT_PRIORITY = Object.freeze([
  'safety_stopped',
  'harness_failure',
  'new_product_failure',
  'reproduced_historical_failure',
  'inconclusive',
  'skipped_blocked',
  'passed',
]);

export class P158W7AdapterError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W7AdapterError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W7AdapterError(code, message, details);
}

function assertDevelopmentTarget(target) {
  const valid =
    target?.runtimeLane === 'development' &&
    target?.isolationState === 'isolated' &&
    target?.ownership === 'p158_campaign' &&
    target?.production === false &&
    target?.foreign === false &&
    target?.tenantDataPresent === false &&
    typeof target?.targetId === 'string' && target.targetId.length > 0 &&
    typeof target?.campaignRunId === 'string' && target.campaignRunId.length > 0;
  if (!valid) {
    fail('development_target_unproven', 'W7 adapters require an isolated development target', {
      targetId: target?.targetId ?? null,
      runtimeLane: target?.runtimeLane ?? null,
    });
  }
}

function cartesian(rows) {
  return rows.reduce((products, row) => products.flatMap((product) =>
    row.values.map((value) => [...product, { dimensionId: row.id, value }])), [[]]);
}

function plannedActions(contract, attempt, planAction) {
  const actions = [];
  for (const allocation of attempt.cardinalityAllocations ?? []) {
    for (const actionId of allocation.actionIds) {
      actions.push({
        actionId,
        source: 'cardinality',
        cardinalityId: allocation.id,
        ordinal: Number.parseInt(actionId.slice(actionId.lastIndexOf(':') + 1), 10),
      });
    }
  }
  const cartesianDimensions = contract.executionContract.dimensions.filter(
    (dimension) => dimension.coverage === 'cartesian',
  );
  if (cartesianDimensions.length > 0 && attempt.executionUnit.ordinal === 1) {
    for (const [index, assignment] of cartesian(cartesianDimensions).entries()) {
      actions.push({
        actionId: `${attempt.attemptId}:matrix:${String(index + 1).padStart(5, '0')}`,
        source: 'matrix',
        dimensionAssignments: assignment,
      });
    }
  }
  const eachDimensions = contract.executionContract.dimensions.filter(
    (dimension) => dimension.coverage === 'each' &&
      dimension.id !== contract.executionContract.expansion.dimensionId,
  );
  for (const dimension of eachDimensions) {
    const values = contract.executionContract.expansion.count === 1
      ? dimension.values
      : [dimension.values[(attempt.executionUnit.ordinal - 1) % dimension.values.length]];
    for (const value of values) {
      actions.push({
        actionId: `${attempt.attemptId}:dimension:${dimension.id}:${value}`,
        source: 'dimension',
        dimensionAssignments: [{ dimensionId: dimension.id, value }],
      });
    }
  }
  if (attempt.executionUnit.dimensionAssignment) {
    const { dimensionId, value } = attempt.executionUnit.dimensionAssignment;
    actions.push({
      actionId: `${attempt.attemptId}:dimension:${dimensionId}:${value}`,
      source: 'expanded_dimension',
      dimensionAssignments: [{ dimensionId, value }],
    });
  }
  if (actions.length === 0) {
    actions.push({ actionId: `${attempt.attemptId}:unit:00001`, source: 'execution_unit' });
  }
  const unique = new Map(actions.map((action) => [action.actionId, action]));
  if (unique.size !== actions.length) fail('duplicate_planned_action_id', attempt.attemptId);
  return [...unique.values()].map((action) => {
    const planned = planAction({
      caseId: contract.caseId,
      attempt: structuredClone(attempt),
      action: structuredClone(action),
      caseSpec: structuredClone(CASE_SPECS[contract.caseId]),
    });
    if (planned === null || typeof planned !== 'object' || Array.isArray(planned)) {
      fail('action_plan_invalid', `${contract.caseId} planner did not return an action object`);
    }
    for (const field of ['actionId', 'source', 'cardinalityId', 'ordinal', 'dimensionAssignments']) {
      if (Object.hasOwn(planned, field) &&
          JSON.stringify(planned[field]) !== JSON.stringify(action[field])) {
        fail('immutable_action_identity_changed',
          `${contract.caseId} planner changed immutable action field ${field}`);
      }
    }
    return { ...action, ...planned };
  });
}

function assertResourceBindings(action, target) {
  for (const bindingName of ['command', 'evidenceCommand', 'logCommand']) {
    const executable = action[bindingName]?.executable;
    if (executable !== undefined &&
        (!Array.isArray(target.allowedExecutables) ||
          !target.allowedExecutables.includes(executable))) {
      fail('executable_not_owned', `${action.actionId} executable is not development-owned`, {
        bindingName,
        executable,
      });
    }
  }
  if (action.systemd?.unit !== undefined &&
      (!Array.isArray(target.allowedSystemdUnits) ||
        !target.allowedSystemdUnits.includes(action.systemd.unit))) {
    fail('systemd_unit_not_owned', `${action.actionId} systemd unit is not development-owned`);
  }
  if (action.process?.pid !== undefined &&
      (!Array.isArray(target.allowedProcessIds) ||
        !target.allowedProcessIds.includes(action.process.pid))) {
    fail('process_not_owned', `${action.actionId} process is not development-owned`);
  }
}

function assertActionPlan(caseId, action, target) {
  const spec = CASE_SPECS[caseId];
  if (action.hook !== spec.hook) {
    fail('undeclared_hook_prohibited', `${caseId} cannot invoke ${action.hook}`, {
      expected: spec.hook,
    });
  }
  if (action.stimulusKind && !spec.stimuli.includes(action.stimulusKind)) {
    fail('undeclared_stimulus_prohibited', `${caseId} cannot invoke ${action.stimulusKind}`);
  }
  if (action.targetId !== target.targetId || action.campaignRunId !== target.campaignRunId) {
    fail('foreign_target_prohibited', `${action.actionId} targets a foreign campaign resource`);
  }
  if (action.repair === true || action.retry === true || action.garbageCollect === true) {
    fail('reactionary_action_prohibited', `${action.actionId} requests repair, retry, or GC`);
  }
  assertResourceBindings(action, target);
}

function terminalState(receipts) {
  for (const state of RESULT_PRIORITY) {
    if (receipts.some((receipt) => receipt.resultState === state)) return state;
  }
  return 'harness_failure';
}

export function createP158DevelopmentCommandPrimitives({
  target,
  execFile = promisify(nodeExecFile),
  kill = process.kill.bind(process),
  clock = () => new Date().toISOString(),
} = {}) {
  assertDevelopmentTarget(target);
  const targetBinding = structuredClone(target);
  async function executeCommand(action, bindingName = 'command') {
    assertResourceBindings(action, targetBinding);
    const binding = action[bindingName];
    const executable = binding?.executable;
    const args = binding?.args ?? [];
    if (typeof executable !== 'string' || executable.length === 0 || !Array.isArray(args)) {
      fail('command_binding_missing', `${action.actionId} has no reviewed command binding`);
    }
    const result = await execFile(executable, args, {
      cwd: binding.cwd,
      env: binding.env,
      timeout: binding.timeoutMilliseconds,
      maxBuffer: binding.maxBufferBytes ?? 4 * 1024 * 1024,
    });
    const stdout = Buffer.from(result.stdout ?? '');
    const stderr = Buffer.from(result.stderr ?? '');
    return {
      resultState: 'passed',
      observedAt: clock(),
      stdoutSha256: createHash('sha256').update(stdout).digest('hex'),
      stdoutByteCount: stdout.byteLength,
      stderrSha256: createHash('sha256').update(stderr).digest('hex'),
      stderrByteCount: stderr.byteLength,
    };
  }
  return {
    captureEvidence: (action) => executeCommand(action, 'evidenceCommand'),
    captureLogs: (action) => executeCommand(action, 'logCommand'),
    executeCli: executeCommand,
    executeBrowser: executeCommand,
    executeDisplay: executeCommand,
    executeShutdown: executeCommand,
    async executeSystemd(action) {
      assertResourceBindings(action, targetBinding);
      if (!action.systemd?.unit || !action.systemd?.verb) {
        fail('systemd_binding_missing', `${action.actionId} has no exact systemd binding`);
      }
      return executeCommand({
        ...action,
        command: {
          executable: action.systemd.executable ?? '/usr/bin/systemctl',
          args: ['--user', action.systemd.verb, action.systemd.unit],
          ...action.systemd.commandOptions,
        },
      });
    },
    async executeProcess(action) {
      assertResourceBindings(action, targetBinding);
      if (!Number.isInteger(action.process?.pid) || action.process.pid < 2 ||
          typeof action.process.signal !== 'string') {
        fail('process_binding_missing', `${action.actionId} has no exact process binding`);
      }
      kill(action.process.pid, action.process.signal);
      return { resultState: 'passed', observedAt: clock(), pid: action.process.pid };
    },
  };
}

export function createP158W7DevelopmentAdapterBundle({
  schedule,
  target,
  primitives,
  planAction,
  additionalAdapters = [],
}) {
  assertDevelopmentTarget(target);
  const targetBinding = structuredClone(target);
  if (typeof planAction !== 'function') fail('action_planner_missing', 'W7 requires an action planner');
  for (const method of ['captureEvidence', 'captureLogs']) {
    if (typeof primitives?.[method] !== 'function') {
      fail('primitive_missing', `W7 requires primitive ${method}`);
    }
  }
  const contracts = new Map(schedule.caseContracts.map((contract) => [contract.caseId, contract]));
  for (const caseId of P158_W7_CASE_IDS) {
    if (!contracts.has(caseId)) fail('case_contract_missing', `Schedule omitted W7 case ${caseId}`);
  }
  const executedActionIds = new Set();
  const effects = {};
  const adapters = P158_W7_CASE_IDS.map((caseId) => {
    const contract = contracts.get(caseId);
    const effectId = contract.declaredEffectIds[0];
    effects[effectId] = async (payload, attempt) => {
      assertDevelopmentTarget(targetBinding);
      const action = payload.action;
      assertActionPlan(caseId, action, targetBinding);
      if (executedActionIds.has(action.actionId)) {
        fail('action_already_executed', `${action.actionId} cannot execute twice`);
      }
      executedActionIds.add(action.actionId);
      const evidence = [];
      let operation;
      let stage = 'pre';
      try {
        evidence.push(await primitives.captureEvidence({ ...action, stage: 'pre', attempt }));
        stage = 'effect';
        const method = `execute${action.hook[0].toUpperCase()}${action.hook.slice(1)}`;
        if (typeof primitives[method] !== 'function') fail('primitive_missing', method);
        operation = await primitives[method]({ ...action, stage: 'effect', attempt });
      } catch (error) {
        operation = {
          resultState: stage === 'effect' && !(error instanceof P158W7AdapterError)
            ? 'new_product_failure'
            : 'harness_failure',
          failureStage: stage,
          errorCode: error?.code ?? error?.name ?? 'effect_failed',
          errorMessage: error?.message ?? String(error),
        };
      }
      try {
        evidence.push(await primitives.captureLogs({ ...action, stage: 'logs', attempt }));
        evidence.push(await primitives.captureEvidence({ ...action, stage: 'post', attempt }));
      } catch (error) {
        operation = {
          ...operation,
          resultState: 'harness_failure',
          evidenceError: error?.message ?? String(error),
        };
      }
      const resultState = RESULT_STATES.has(operation?.resultState)
        ? operation.resultState
        : 'harness_failure';
      return {
        actionId: action.actionId,
        resultState,
        operation,
        evidence,
        artifactIds: evidence.flatMap((entry) =>
          entry.artifactIds ?? (entry.artifactId ? [entry.artifactId] : [])),
        repairAttempted: false,
        retryAttempted: false,
        garbageCollectionAttempted: false,
      };
    };
    return createP158CaseAdapter({
      caseId,
      evidenceProfile: contract.evidenceProfile,
      executionContract: contract.executionContract,
      execute: async ({ attempt, requestEffect }) => {
        if (!['E0', 'E1', 'E2', 'E3'].includes(attempt.environmentId)) {
          fail('environment_prohibited', attempt.environmentId);
        }
        if (attempt.environmentIds.includes('E3') && targetBinding.redactedComparisonOnly !== true) {
          fail('production_comparison_prohibited', 'E3 requires a pre-frozen redacted comparison');
        }
        const actions = plannedActions(contract, attempt, planAction);
        const receipts = [];
        for (const action of actions) {
          receipts.push(await requestEffect(effectId, { action }));
        }
        return {
          resultState: terminalState(receipts),
          actionCount: actions.length,
          actionIds: actions.map((action) => action.actionId),
          receipts,
          repairAttempted: false,
          retryAttempted: false,
          garbageCollectionAttempted: false,
        };
      },
    });
  });
  return {
    adapters: [...adapters, ...additionalAdapters],
    w7Adapters: adapters,
    effects,
    executedActionIds,
  };
}
