import { createHash } from 'node:crypto';
import { execFile as nodeExecFile } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import {
  getServiceBrowsers,
  getServiceIncidents,
  getServiceProfiles,
  getServiceSitePolicies,
  getServiceStatus,
} from '../../packages/client/src/service-observability.js';
import { createP158CaseAdapter } from './p158-execution-schedule.js';
import { canonicalJson, sha256 } from './p158-campaign-controller.js';
import {
  buildP158W9ActionPlan,
  assertP158W9DevelopmentTarget,
  canonicalW9ReceiptDigest,
  createDistributedC01LiveHook,
  P158_W9_CASE_IDS,
} from './p158-w9-campaign-orchestrator.js';

const SOURCE_PATH = 'scripts/lib/p158-w9-concrete-drivers.js';
const SOURCE_SHA256 = createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
const C01_SOURCE_PATH = 'scripts/run-p158-distributed-calibration-live.js';
const C01_SOURCE_SHA256 = createHash('sha256').update(readFileSync(
  fileURLToPath(new URL('../run-p158-distributed-calibration-live.js', import.meta.url)),
)).digest('hex');

export const P158_W9_MANIFEST_HOOK_IDS = Object.freeze([
  'w9.browser_crash', 'w9.external_dashboard_action', 'w9.external_handoff_reconnect',
  'w9.service_command', 'w9.supervisor_transition',
]);

const SERVICE_CLIENT_CALLS = Object.freeze([
  (options) => getServiceStatus(options),
  (options) => getServiceBrowsers(options),
  (options) => getServiceIncidents({ ...options, query: { summary: true, limit: 20 } }),
  (options) => getServiceProfiles(options),
  (options) => getServiceSitePolicies(options),
]);

export class P158W9ConcreteDriverError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W9ConcreteDriverError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W9ConcreteDriverError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function without(value, key) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([field]) => field !== key));
}

export function canonicalW9PlanDigest(plan) {
  const body = without(plan, 'planSha256');
  return sha256({
    ...body,
    actions: (body.actions ?? []).map((action) => without(action, 'receipt')),
  });
}

function verifyPlan(plan, schemaVersion, label) {
  if (plan?.schemaVersion !== schemaVersion || plan.planSha256 !== canonicalW9PlanDigest(plan) ||
      !Array.isArray(plan.actions) || new Set(plan.actions.map((entry) => entry.actionId)).size !== plan.actions.length) {
    fail('live_plan_invalid', `${label} is missing, changed, or duplicates an action`);
  }
  return new Map(plan.actions.map((entry) => [entry.actionId, clone(entry)]));
}

function assertPlanBinding(plan, schedule, target, label) {
  if (plan.scheduleSha256 !== schedule.scheduleSha256 || plan.candidateSha256 !== target.candidateSha256 ||
      plan.runId !== target.runId || plan.workflowRunId !== target.workflowRunId ||
      plan.workflowRunAttempt !== target.workflowRunAttempt ||
      plan.repairAllowed !== false || plan.retryAllowed !== false || plan.garbageCollectionAllowed !== false) {
    fail('live_plan_binding_mismatch', `${label} does not bind the frozen run, candidate, schedule, and effect policy`);
  }
}

function actionCounts(plan) {
  const counts = new Map(P158_W9_CASE_IDS.map((caseId) => [caseId, 0]));
  for (const entry of plan) counts.set(entry.attempt.caseId, counts.get(entry.attempt.caseId) + entry.actions.length);
  return counts;
}

function requiredKinds(caseId) {
  return {
    C01: ['service_command', 'dashboard_action', 'handoff_reconnect'],
    C02: ['service_command', 'dashboard_action', 'handoff_reconnect', 'declared_browser_crash'],
    C03: ['declared_supervisor_transition'],
    C04: ['service_command', 'dashboard_action', 'handoff_reconnect', 'declared_browser_crash'],
    C05: ['handoff_reconnect'],
  }[caseId];
}

function hookIds(caseId) {
  const ids = {
    service_command: 'w9.service_command',
    dashboard_action: 'w9.external_dashboard_action',
    handoff_reconnect: 'w9.external_handoff_reconnect',
    declared_browser_crash: 'w9.browser_crash',
    declared_supervisor_transition: 'w9.supervisor_transition',
  };
  return [...new Set(requiredKinds(caseId).map((kind) => ids[kind]))].sort();
}

function receiptBody(action, target, fields) {
  return {
    schemaVersion: 'agent-browser.p158-w9-action-receipt.v1',
    runId: target.runId, candidateSha256: target.candidateSha256,
    scheduleSha256: action.scheduleSha256,
    workflowRunId: target.workflowRunId, workflowRunAttempt: target.workflowRunAttempt,
    caseId: action.caseId, attemptId: action.attemptId, actionId: action.actionId,
    environmentId: action.environmentId, kind: action.kind, attempt: 1,
    retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
    ...clone(fields),
  };
}

async function writeEvidence(artifactStore, action, evidence) {
  const content = canonicalJson(evidence);
  const relativePath = `w9-driver-evidence/${action.actionId}.json`;
  await artifactStore.writeOnce(relativePath, content);
  return { artifactId: `w9:${action.actionId}`, relativePath, sha256: sha256(content) };
}

function exactExternalEvidence(receipt, action, target, workflowPlanSha256) {
  if (receipt?.receiptSha256 !== sha256(without(receipt, 'receiptSha256')) ||
      receipt.actionId !== action.actionId || receipt.caseId !== action.caseId ||
      receipt.attemptId !== action.attemptId || receipt.kind !== action.kind ||
      receipt.environmentId !== 'E2' || receipt.state !== 'passed' || receipt.attempt !== 1 ||
      receipt.retryAttempted !== false || receipt.repairAttempted !== false ||
      receipt.offHost !== true || receipt.outsideServiceNetworkNamespace !== true ||
      receipt.operatorVisibleState !== 'ready' || receipt.readyBeforePixels !== true ||
      receipt.pixelsObserved !== true || receipt.handoffUrlSha256 !== target.handoffUrlSha256 ||
      receipt.retainedIdentitySha256 !== target.retainedIdentitySha256 ||
      receipt.externalVantageAggregateSha256 !== target.externalVantageAggregateSha256 ||
      receipt.externalHandoffOracleSha256 !== target.externalHandoffOracleSha256 ||
      receipt.workflowPlanSha256 !== workflowPlanSha256) {
    fail('external_workflow_receipt_invalid', `${action.actionId} lacks exact downloaded external evidence`);
  }
  return receipt;
}

function concreteTransitionPrimitives({ execFile = promisify(nodeExecFile), kill = process.kill.bind(process) } = {}) {
  return {
    async executeProcess({ process: binding }) {
      if (!Number.isInteger(binding?.pid) || binding.pid < 2 || typeof binding.signal !== 'string') {
        fail('declared_process_binding_invalid', 'Declared browser crash requires an exact PID and signal');
      }
      kill(binding.pid, binding.signal);
      return { resultState: 'passed', observedAt: new Date().toISOString() };
    },
    async executeSystemd({ systemd }) {
      if (typeof systemd?.unit !== 'string' || typeof systemd.verb !== 'string') {
        fail('declared_supervisor_binding_invalid', 'Declared supervisor transition requires an exact unit and verb');
      }
      await execFile(systemd.executable ?? 'systemctl', ['--user', systemd.verb, systemd.unit]);
      return { resultState: 'passed', observedAt: new Date().toISOString() };
    },
  };
}

export function createP158W9ConcreteDriverBundle({
  schedule, target, artifactStore, externalWorkflowPlan, declaredTransitionPlan,
  c01, fetch: suppliedFetch, clock = { wallNow: () => new Date().toISOString(), monotonicNow: () => Number(process.hrtime.bigint()) },
  transitionPrimitives: suppliedTransitionPrimitives, testing = false,
}) {
  assertP158W9DevelopmentTarget(target);
  const fetch = suppliedFetch ?? globalThis.fetch;
  const transitionPrimitives = suppliedTransitionPrimitives ?? concreteTransitionPrimitives();
  const plan = buildP158W9ActionPlan(schedule);
  const actions = plan.flatMap((entry) => entry.actions).map((action) => ({ ...action, scheduleSha256: schedule.scheduleSha256 }));
  const byId = new Map(actions.map((action) => [action.actionId, action]));
  const external = verifyPlan(
    externalWorkflowPlan,
    'agent-browser.p158-w9-external-workflow-plan.v1',
    'External workflow plan',
  );
  assertPlanBinding(externalWorkflowPlan, schedule, target, 'External workflow plan');
  const externalPlanSha256 = externalWorkflowPlan.planSha256;
  const transitions = verifyPlan(
    declaredTransitionPlan,
    'agent-browser.p158-w9-declared-transition-plan.v1',
    'Declared transition plan',
  );
  assertPlanBinding(declaredTransitionPlan, schedule, target, 'Declared transition plan');
  const c01Complete = c01?.driverId === 'p158.distributed-c01-live.v1' &&
    typeof c01.runRoot === 'string' && c01.runRoot.startsWith('/') &&
    c01.config?.candidate?.candidateSha256 === target.candidateSha256 &&
    c01.config?.runId === target.runId &&
    typeof c01.externalAggregatePath === 'string' && c01.externalAggregatePath.startsWith('/') &&
    Array.isArray(c01.externalReceiptPaths) && c01.externalReceiptPaths.length === 2 &&
    c01.externalReceiptPaths.every((path) => typeof path === 'string' && path.startsWith('/')) &&
    typeof c01.clock?.wallNow === 'function' && typeof c01.scheduler?.waitUntil === 'function';
  const c01Hook = c01Complete ? createDistributedC01LiveHook({
    config: c01.config, runRoot: c01.runRoot, fetch,
    clock: c01.clock, scheduler: c01.scheduler,
    loadExternalEvidence: async () => ({
      aggregate: JSON.parse(await readFile(c01.externalAggregatePath, 'utf8')),
      receipts: await Promise.all(c01.externalReceiptPaths.map(async (path) =>
        JSON.parse(await readFile(path, 'utf8')))),
    }),
  }) : null;
  const actionCountByCase = actionCounts(plan);
  const classification = new Map();
  for (const caseId of P158_W9_CASE_IDS) {
    const caseActions = actions.filter((action) => action.caseId === caseId);
    const missing = [];
    if (caseId === 'C01' && !c01Hook) missing.push('distributed_c01_live_driver');
    for (const action of caseActions) {
      if (caseId !== 'C01' && ['dashboard_action', 'handoff_reconnect'].includes(action.kind) && !external.has(action.actionId)) {
        missing.push(`external_workflow:${action.actionId}`);
      }
      if (action.declaredFault && !transitions.has(action.actionId)) {
        missing.push(`declared_transition:${action.actionId}`);
      }
    }
    classification.set(caseId, {
      mode: missing.length === 0 ? 'concrete_live' : 'explicit_blocked',
      blocker: missing.length === 0 ? null : {
        code: 'live_case_hook_missing', detail: missing.sort().join(','),
      },
    });
  }

  async function loadExternalReceipt(action, descriptor) {
    let receipt;
    if (testing && descriptor.receipt) receipt = clone(descriptor.receipt);
    else {
      if (typeof descriptor.receiptPath !== 'string' || !descriptor.receiptPath.startsWith('/')) {
        fail('external_receipt_path_invalid', `${action.actionId} requires an absolute downloaded receipt path`);
      }
      receipt = JSON.parse(await readFile(descriptor.receiptPath, 'utf8'));
    }
    return exactExternalEvidence(receipt, action, target, externalPlanSha256);
  }

  async function executeServiceCommand(input) {
    const action = byId.get(input.actionId);
    const origin = target.serviceOrigins?.[action.environmentId];
    if (!action || action.kind !== 'service_command' || typeof origin !== 'string') {
      fail('service_action_binding_invalid', input.actionId);
    }
    const before = clock.monotonicNow();
    let body;
    try {
      const call = SERVICE_CLIENT_CALLS[(action.actionOrdinal - 1) % SERVICE_CLIENT_CALLS.length];
      body = await call({ baseUrl: origin, fetch });
    } catch (error) {
      body = { failure: { code: error?.code ?? 'service_transport_failed', message: error?.message ?? String(error) } };
    }
    const observedAt = clock.wallNow();
    const evidence = await writeEvidence(artifactStore, action, {
      serviceOrigin: origin, responseSha256: sha256(body), observedAt,
      latencyMs: Math.max(0, Number(clock.monotonicNow() - before) / 1_000_000),
    });
    const failed = body?.failure !== undefined || body?.success === false;
    const receipt = receiptBody(action, target, {
      state: failed ? 'failed' : 'passed', observedAt, effectClass: 'read_only',
      evidenceArtifactIds: [evidence.artifactId],
      ...(failed ? { failure: { code: body?.failure?.code ?? 'service_http_failure' } } : {}),
    });
    return { ...receipt, receiptSha256: canonicalW9ReceiptDigest(receipt) };
  }

  async function executeExternal(actionInput) {
    const action = byId.get(actionInput.actionId);
    const descriptor = external.get(action?.actionId);
    if (!action || !descriptor || !['dashboard_action', 'handoff_reconnect'].includes(action.kind)) {
      fail('external_action_binding_invalid', actionInput.actionId);
    }
    const downloaded = await loadExternalReceipt(action, descriptor);
    const evidence = await writeEvidence(artifactStore, action, downloaded);
    const receipt = receiptBody(action, target, {
      state: 'passed', observedAt: downloaded.observedAt, effectClass: 'external_ingress',
      evidenceArtifactIds: [evidence.artifactId],
      externalEvidence: {
        offHost: true, outsideServiceNetworkNamespace: true, operatorVisibleState: 'ready',
        readyBeforePixels: true, pixelsObserved: true,
        externalVantageAggregateSha256: target.externalVantageAggregateSha256,
        externalHandoffOracleSha256: target.externalHandoffOracleSha256,
        handoffUrlSha256: target.handoffUrlSha256,
        retainedIdentitySha256: target.retainedIdentitySha256,
      },
    });
    return { ...receipt, receiptSha256: canonicalW9ReceiptDigest(receipt) };
  }

  async function executeTransition(actionInput) {
    const action = byId.get(actionInput.actionId);
    const binding = transitions.get(action?.actionId);
    if (!action?.declaredFault || !binding || binding.transitionKind !== action.kind) {
      fail('declared_transition_binding_invalid', actionInput.actionId);
    }
    const operation = action.kind === 'declared_browser_crash'
      ? await transitionPrimitives.executeProcess({ ...action, process: clone(binding.process) })
      : await transitionPrimitives.executeSystemd({ ...action, systemd: clone(binding.systemd) });
    const evidence = await writeEvidence(artifactStore, action, operation);
    const receipt = receiptBody(action, target, {
      state: operation.resultState === 'passed' ? 'passed' : 'failed',
      observedAt: operation.observedAt ?? clock.wallNow(), effectClass: 'declared_fault',
      evidenceArtifactIds: [evidence.artifactId],
      declaredTransition: {
        declarationId: action.actionId, kind: action.kind,
        beforeState: binding.beforeState, afterState: binding.afterState,
      },
      ...(operation.resultState === 'passed' ? {} : { failure: { code: operation.errorCode ?? 'declared_transition_failed' } }),
    });
    return { ...receipt, receiptSha256: canonicalW9ReceiptDigest(receipt) };
  }

  async function executeScheduledTeardown() {
    const binding = declaredTransitionPlan.teardown;
    if (!binding?.systemd) fail('scheduled_teardown_binding_missing', 'W9 scheduled teardown is not declared');
    const operation = await transitionPrimitives.executeSystemd({
      actionId: `${target.runId}:scheduled-teardown`, systemd: clone(binding.systemd),
    });
    const pseudoAction = { actionId: `${target.runId}:scheduled-teardown` };
    const evidence = await writeEvidence(artifactStore, pseudoAction, operation);
    const body = {
      schemaVersion: 'agent-browser.p158-w9-teardown-receipt.v1',
      runId: target.runId, candidateSha256: target.candidateSha256,
      scheduleSha256: schedule.scheduleSha256, attempt: 1,
      state: operation.resultState === 'passed' ? 'passed' : 'failed',
      effectClass: 'scheduled_teardown', declaredTeardownId: pseudoAction.actionId,
      evidenceArtifactIds: [evidence.artifactId], retryAttempted: false,
      repairAttempted: false, garbageCollectionAttempted: false,
    };
    return { ...body, receiptSha256: canonicalW9ReceiptDigest(body) };
  }

  const drivers = {
    executeDistributedC01: c01Hook,
    executeServiceCommand,
    executeExternalDashboardAction: executeExternal,
    executeExternalHandoffReconnect: executeExternal,
    executeDeclaredBrowserCrash: executeTransition,
    executeDeclaredSupervisorTransition: executeTransition,
    executeScheduledTeardown,
    async verifyEvidenceArtifact(artifactId) {
      if (!artifactId.startsWith('w9:')) return false;
      const actionId = artifactId.slice(3);
      const content = await artifactStore.read(`w9-driver-evidence/${actionId}.json`);
      return content !== undefined;
    },
    hookBindings: Object.fromEntries([
      ['executeDistributedC01', C01_SOURCE_PATH],
      ['executeServiceCommand', SOURCE_PATH],
      ['executeExternalDashboardAction', SOURCE_PATH],
      ['executeExternalHandoffReconnect', SOURCE_PATH],
      ['executeDeclaredBrowserCrash', SOURCE_PATH],
      ['executeDeclaredSupervisorTransition', SOURCE_PATH],
      ['executeScheduledTeardown', SOURCE_PATH],
      ['verifyEvidenceArtifact', SOURCE_PATH],
    ].map(([method, sourcePath]) => [method, {
      implementationKind: c01Hook || method !== 'executeDistributedC01' ? 'concrete_live' : 'explicit_blocked',
      sourcePath,
      sourceSha256: sourcePath === SOURCE_PATH ? SOURCE_SHA256 : C01_SOURCE_SHA256,
    }])),
  };
  return {
    drivers, classification, actions, actionCountByCase,
    freezeEligible: testing === false && suppliedFetch === undefined && suppliedTransitionPrimitives === undefined,
    c01FetchSource: suppliedFetch === undefined ? 'global' : 'supplied',
    sourcePath: SOURCE_PATH, sourceSha256: SOURCE_SHA256,
  };
}

export function createP158W9FreezeAdapterEntries({ schedule, bundle, liveHookManifestSha256 }) {
  if (!/^[a-f0-9]{64}$/u.test(liveHookManifestSha256 ?? '')) fail('live_manifest_digest_invalid', 'Manifest digest is required');
  const contracts = new Map(schedule.caseContracts.map((entry) => [entry.caseId, entry]));
  const adapterBindings = [];
  const adapters = [];
  const effects = {};
  let c01ResultPromise = null;
  for (const caseId of P158_W9_CASE_IDS) {
    const contract = contracts.get(caseId);
    const observed = bundle.classification.get(caseId);
    const classified = bundle.freezeEligible ? observed : {
      mode: 'explicit_blocked',
      blocker: { code: 'provider_free_test_driver', detail: 'Injected test drivers cannot authorize live execution' },
    };
    const count = bundle.actionCountByCase.get(caseId);
    const effectId = contract.declaredEffectIds[0];
    effects[effectId] = async (payload) => {
      const actionIds = payload?.actionIds ?? [];
      const actions = actionIds.map((actionId) => bundle.actions.find((action) => action.actionId === actionId));
      if (actions.some((action) => !action || action.caseId !== caseId)) {
        fail('adapter_action_binding_invalid', `${caseId} requested an action outside its frozen allocation`);
      }
      if (caseId === 'C01') {
        c01ResultPromise ??= bundle.drivers.executeDistributedC01({
          actions: clone(bundle.actions.filter((action) => action.caseId === 'C01')),
        });
        const aggregate = await c01ResultPromise;
        return {
          resultState: aggregate?.result?.calibration?.clean === true ? 'passed' : 'new_product_failure',
          distributedResultSha256: aggregate?.resultSha256 ?? null,
        };
      }
      const receipts = [];
      for (const action of actions) {
        const method = {
          service_command: 'executeServiceCommand', dashboard_action: 'executeExternalDashboardAction',
          handoff_reconnect: 'executeExternalHandoffReconnect', declared_browser_crash: 'executeDeclaredBrowserCrash',
          declared_supervisor_transition: 'executeDeclaredSupervisorTransition',
        }[action.kind];
        receipts.push(await bundle.drivers[method](action));
      }
      return {
        resultState: receipts.some((receipt) => receipt.state === 'failed') ? 'new_product_failure' : 'passed',
        receipts,
      };
    };
    const base = createP158CaseAdapter({
      caseId, evidenceProfile: contract.evidenceProfile, executionContract: contract.executionContract,
      execute: async ({ attempt, requestEffect }) => {
        if (classified.mode === 'explicit_blocked') return {
          resultState: 'skipped_blocked', blocker: clone(classified.blocker), effectState: 'not_started',
          retryDisposition: 'prohibited', repairAttempted: false, retryAttempted: false,
          garbageCollectionAttempted: false,
        };
        const actionIds = bundle.actions.filter((action) => action.attemptId === attempt.attemptId).map((action) => action.actionId);
        const receipt = await requestEffect(effectId, { actionIds });
        return { ...clone(receipt), actionIds, retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false };
      },
    });
    const adapter = {
      ...base, executionMode: classified.mode, providerFree: false,
      liveHookManifestSha256, effectsAllowed: classified.mode === 'concrete_live',
      blocker: clone(classified.blocker),
    };
    adapters.push(adapter);
    adapterBindings.push({
      caseId, adapterId: contract.adapterId, executionContractSha256: contract.executionContractSha256,
      mode: classified.mode, sourcePath: bundle.sourcePath, sourceSha256: bundle.sourceSha256,
      hookIds: classified.mode === 'concrete_live' ? hookIds(caseId) : [],
      implementedActionCount: classified.mode === 'concrete_live' ? count : 0,
      blockedActionCount: classified.mode === 'concrete_live' ? 0 : count,
      effectsAllowed: classified.mode === 'concrete_live', blocker: clone(classified.blocker),
    });
  }
  return { adapters, adapterBindings, effects };
}

export function p158W9HookManifestEntries() {
  return P158_W9_MANIFEST_HOOK_IDS.map((hookId) => ({
    hookId, implementationKind: 'concrete_live', sourcePath: SOURCE_PATH, sourceSha256: SOURCE_SHA256,
  }));
}
