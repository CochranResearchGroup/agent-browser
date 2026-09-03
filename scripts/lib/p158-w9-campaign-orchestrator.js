import { dirname, isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, createFileArtifactStore, sha256 } from './p158-campaign-controller.js';
import { classifyOperatorUrl } from './p158-external-handoff-oracle.js';
import {
  finalizeLiveDistributedCalibration,
  prepareLiveDistributedCalibration,
  startLiveDistributedCalibration,
} from '../run-p158-distributed-calibration-live.js';

export const P158_W9_CASE_IDS = Object.freeze(['C01', 'C02', 'C03', 'C04', 'C05']);
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const P158_W9_LIVE_HOOK_CONTRACT = Object.freeze({
  executeDistributedC01: 'agent-browser.p158-distributed-c01-result.v1',
  executeServiceCommand: 'agent-browser.p158-w9-action-receipt.v1',
  executeExternalDashboardAction: 'agent-browser.p158-w9-action-receipt.v1',
  executeExternalHandoffReconnect: 'agent-browser.p158-w9-action-receipt.v1',
  executeDeclaredBrowserCrash: 'agent-browser.p158-w9-action-receipt.v1',
  executeDeclaredSupervisorTransition: 'agent-browser.p158-w9-action-receipt.v1',
  executeScheduledTeardown: 'agent-browser.p158-w9-teardown-receipt.v1',
  verifyEvidenceArtifact: 'agent-browser.p158-evidence-artifact-verification.v1',
});

const ACTION_KINDS = Object.freeze({
  service_commands: 'service_command',
  dashboard_actions: 'dashboard_action',
  reconnects: 'handoff_reconnect',
  browser_crashes: 'declared_browser_crash',
  supervisor_transitions: 'declared_supervisor_transition',
});

const DRIVER_METHODS = Object.freeze({
  service_command: 'executeServiceCommand',
  dashboard_action: 'executeExternalDashboardAction',
  handoff_reconnect: 'executeExternalHandoffReconnect',
  declared_browser_crash: 'executeDeclaredBrowserCrash',
  declared_supervisor_transition: 'executeDeclaredSupervisorTransition',
});

const RESULT_STATES = new Set([
  'passed', 'reproduced_historical_failure', 'new_product_failure', 'harness_failure',
  'inconclusive', 'skipped_blocked', 'safety_stopped',
]);

export class P158W9OrchestrationError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W9OrchestrationError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W9OrchestrationError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

export function canonicalP158W9TargetBindingDigest(target) {
  return sha256({
    candidateSha256: target?.candidateSha256,
    serviceOrigins: target?.serviceOrigins,
    serviceResolvedAddresses: target?.serviceResolvedAddresses,
    reviewedLocalDevelopmentOrigin: target?.reviewedLocalDevelopmentOrigin,
    allowedExternalServiceOrigins: target?.allowedExternalServiceOrigins,
    syntheticTarget: target?.syntheticTarget,
    productionHostnames: target?.productionHostnames,
  });
}

export function assertP158W9DevelopmentTarget(target) {
  let e1;
  let e2;
  try {
    e1 = new URL(target?.serviceOrigins?.E1);
    e2 = new URL(target?.serviceOrigins?.E2);
  } catch {
    fail('development_target_unproven', 'W9 requires parseable exact E1/E2 origins');
  }
  const e1Local = ['localhost', '127.0.0.1', '[::1]'].includes(e1.hostname) &&
    ['http:', 'https:'].includes(e1.protocol) && e1.port.length > 0 && e1.origin === target.reviewedLocalDevelopmentOrigin;
  const resolved = target?.serviceResolvedAddresses?.E2;
  const e2Classification = classifyOperatorUrl(e2.origin, {
    role: 'location_header', resolvedAddresses: Array.isArray(resolved) ? resolved : [],
  });
  const productionHostnames = target?.productionHostnames;
  const externalAllowlist = target?.allowedExternalServiceOrigins;
  if (target?.runtimeLane !== 'development' || target.production !== false ||
      target.repairAllowed !== false || target.retryAllowed !== false ||
      target.garbageCollectionAllowed !== false ||
      typeof target.runId !== 'string' || !target.runId ||
      !/^[a-f0-9]{64}$/u.test(target.candidateSha256 ?? '') ||
      !/^\d+$/u.test(target.workflowRunId ?? '') ||
      !Number.isInteger(target.workflowRunAttempt) || target.workflowRunAttempt < 1 ||
      !/^[a-f0-9]{64}$/u.test(target.handoffUrlSha256 ?? '') ||
      !/^[a-f0-9]{64}$/u.test(target.retainedIdentitySha256 ?? '') ||
      !/^[a-f0-9]{64}$/u.test(target.externalVantageAggregateSha256 ?? '') ||
      !/^[a-f0-9]{64}$/u.test(target.externalHandoffOracleSha256 ?? '') ||
      !Array.isArray(target.environmentIds) ||
      target.environmentIds.length !== 2 || target.environmentIds.join(',') !== 'E1,E2' ||
      !e1Local || e2.protocol !== 'https:' || e2.origin !== target.serviceOrigins.E2 ||
      !Array.isArray(resolved) || resolved.length === 0 || e2Classification.findingCodes.length > 0 ||
      !Array.isArray(externalAllowlist) || !externalAllowlist.includes(e2.origin) ||
      externalAllowlist.some((origin) => { try { return new URL(origin).protocol !== 'https:'; } catch { return true; } }) ||
      target.syntheticTarget !== true || !Array.isArray(productionHostnames) || productionHostnames.length === 0 ||
      productionHostnames.map((value) => value.toLowerCase()).includes(e2.hostname.toLowerCase()) ||
      target.reviewedOriginBindingSha256 !== canonicalP158W9TargetBindingDigest(target)) {
    fail('development_target_unproven', 'W9 requires an explicit development-only target with repair, retry, and GC disabled');
  }
  return target;
}

function assertDriverBindings(drivers) {
  for (const method of Object.keys(P158_W9_LIVE_HOOK_CONTRACT)) {
    const binding = drivers?.hookBindings?.[method];
    if (binding?.implementationKind === 'explicit_blocked') {
      fail('live_hook_blocked', `${method} is explicitly blocked and cannot masquerade as live`, binding);
    }
    if (typeof drivers?.[method] !== 'function' || binding?.implementationKind !== 'concrete_live' ||
        typeof binding.sourcePath !== 'string' || binding.sourcePath.startsWith('/') ||
        binding.sourcePath.split('/').includes('..') || !/^[a-f0-9]{64}$/u.test(binding.sourceSha256 ?? '')) {
      fail('live_hook_binding_unproven', `${method} lacks an exact source-hashed concrete live binding`);
    }
  }
}

export function canonicalW9ReceiptDigest(receipt) {
  return sha256(Object.fromEntries(Object.entries(receipt ?? {})
    .filter(([key]) => key !== 'receiptSha256')));
}

function assertSchedule(schedule) {
  if (schedule?.schemaVersion !== 'agent-browser.p158-execution-schedule.v1' ||
      schedule.scheduleSha256 !== sha256(Object.fromEntries(Object.entries(schedule)
        .filter(([key]) => !['scheduleSha256', 'adapterReadiness'].includes(key))))) {
    fail('schedule_seal_mismatch', 'W9 requires the unchanged sealed execution schedule');
  }
  const contracts = schedule.caseContracts.filter((entry) => P158_W9_CASE_IDS.includes(entry.caseId));
  if (contracts.map((entry) => entry.caseId).join(',') !== P158_W9_CASE_IDS.join(',')) {
    fail('w9_contract_missing', 'The sealed schedule does not contain exact C01 through C05 contracts');
  }
}

function allocation(attempt, id) {
  return attempt.cardinalityAllocations.find((entry) => entry.id === id)?.assignedValue ?? 0;
}

function actionEnvironment(attempt, kind, ordinal) {
  if (kind === 'dashboard_action' || kind === 'handoff_reconnect' || attempt.caseId === 'C05') return 'E2';
  return attempt.environmentIds[(attempt.repetition + ordinal) % attempt.environmentIds.length];
}

export function buildP158W9ActionPlan(schedule) {
  assertSchedule(schedule);
  return schedule.attempts.filter((attempt) => P158_W9_CASE_IDS.includes(attempt.caseId)).map((attempt) => {
    const actions = [];
    for (const [cardinalityId, kind] of Object.entries(ACTION_KINDS)) {
      const declared = attempt.cardinalityAllocations.find((entry) => entry.id === cardinalityId);
      for (let index = 0; index < (declared?.actionIds.length ?? 0); index += 1) {
        const actionId = declared.actionIds[index];
        const actionOrdinal = Number.parseInt(actionId.slice(actionId.lastIndexOf(':') + 1), 10);
        const environmentId = actionEnvironment(attempt, kind, actionOrdinal);
        actions.push({
          actionId, attemptId: attempt.attemptId, caseId: attempt.caseId,
          kind, cardinalityId, actionOrdinal, environmentId,
          transport: ['dashboard_action', 'handoff_reconnect'].includes(kind) ? 'external_ingress' : 'development_direct',
          declaredFault: ['declared_browser_crash', 'declared_supervisor_transition'].includes(kind),
          mixedLoad: attempt.caseId === 'C03'
            ? ['retained_browser_commands', 'dashboard_use', 'durable_handoff_reopen'][(attempt.repetition - 1) % 3]
            : null,
          enduranceEvent: attempt.caseId === 'C05'
            ? ['viewer_expiry', 'controller_expiry', 'client_restart', 'scheduled_network_profile'][(attempt.repetition - 1) % 4]
            : null,
        });
      }
    }
    const expected = Object.entries(ACTION_KINDS).reduce((sum, [id]) => sum + allocation(attempt, id), 0);
    if (actions.length !== expected || new Set(actions.map((entry) => entry.actionId)).size !== actions.length) {
      fail('action_allocation_mismatch', `${attempt.attemptId} does not have an exact action allocation`);
    }
    return { attempt: clone(attempt), actions };
  });
}

function checkpointPath(kind, id) {
  return `w9/checkpoints/${kind}/${id}.json`;
}

async function readOptional(store, path) {
  try {
    const content = await store.read(path);
    return content === undefined ? null : JSON.parse(content.toString('utf8'));
  } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
}

async function writeCheckpoint(store, path, value) {
  const body = { ...clone(value), checkpointSha256: sha256(value) };
  await store.writeOnce(path, canonicalJson(body));
  return body;
}

function verifyCheckpoint(value) {
  if (!value || value.checkpointSha256 !== sha256(Object.fromEntries(Object.entries(value)
    .filter(([key]) => key !== 'checkpointSha256')))) {
    fail('checkpoint_integrity_mismatch', 'Append-only W9 checkpoint is missing or changed');
  }
  return value;
}

function validateReceipt(action, receipt, target, schedule) {
  if (receipt?.schemaVersion !== 'agent-browser.p158-w9-action-receipt.v1' ||
      receipt.receiptSha256 !== canonicalW9ReceiptDigest(receipt) ||
      receipt.runId !== target.runId || receipt.candidateSha256 !== target.candidateSha256 ||
      receipt.scheduleSha256 !== schedule.scheduleSha256 ||
      receipt.workflowRunId !== target.workflowRunId ||
      receipt.workflowRunAttempt !== target.workflowRunAttempt ||
      receipt.caseId !== action.caseId || receipt.attemptId !== action.attemptId ||
      receipt.environmentId !== action.environmentId || receipt.kind !== action.kind ||
      receipt?.actionId !== action.actionId || receipt.attempt !== 1 ||
      !['passed', 'failed'].includes(receipt.state) || receipt.retryAttempted !== false ||
      receipt.repairAttempted !== false || receipt.garbageCollectionAttempted !== false ||
      !Array.isArray(receipt.evidenceArtifactIds) || receipt.evidenceArtifactIds.length === 0 ||
      !Number.isFinite(Date.parse(receipt.observedAt)) ||
      (receipt.state === 'failed' && typeof receipt.failure?.code !== 'string')) {
    fail('action_receipt_invalid', `${action.actionId} did not return one exact terminal receipt`);
  }
  if (action.kind === 'service_command' && receipt.effectClass !== 'read_only') {
    fail('action_receipt_invalid', `${action.actionId} is not proven read-only`);
  }
  if (action.declaredFault && (
    receipt.effectClass !== 'declared_fault' ||
    receipt.declaredTransition?.declarationId !== action.actionId ||
    receipt.declaredTransition?.kind !== action.kind
  )) fail('declared_transition_unproven', `${action.actionId} lacks its declared transition receipt`);
  if (action.transport === 'external_ingress') {
    const external = receipt.externalEvidence;
    if (receipt.effectClass !== 'external_ingress' || external?.offHost !== true ||
        external.outsideServiceNetworkNamespace !== true || external.operatorVisibleState !== 'ready' ||
        external.readyBeforePixels !== true || external.pixelsObserved !== true ||
        external.externalVantageAggregateSha256 !== target.externalVantageAggregateSha256 ||
        external.externalHandoffOracleSha256 !== target.externalHandoffOracleSha256 ||
        external.handoffUrlSha256 !== target.handoffUrlSha256 ||
        external.retainedIdentitySha256 !== target.retainedIdentitySha256) {
      fail('external_ingress_receipt_unproven', `${action.actionId} lacks exact external ingress evidence`);
    }
  }
  return clone(receipt);
}

function validateTeardownReceipt(receipt, target, schedule) {
  if (receipt?.schemaVersion !== 'agent-browser.p158-w9-teardown-receipt.v1' ||
      receipt.receiptSha256 !== canonicalW9ReceiptDigest(receipt) ||
      receipt.runId !== target.runId || receipt.candidateSha256 !== target.candidateSha256 ||
      receipt.scheduleSha256 !== schedule.scheduleSha256 || receipt.attempt !== 1 ||
      !['passed', 'failed'].includes(receipt.state) || receipt.retryAttempted !== false ||
      receipt.repairAttempted !== false || receipt.garbageCollectionAttempted !== false ||
      receipt.effectClass !== 'scheduled_teardown' ||
      receipt.declaredTeardownId !== `${target.runId}:scheduled-teardown` ||
      !Array.isArray(receipt.evidenceArtifactIds) || receipt.evidenceArtifactIds.length === 0) {
    fail('teardown_receipt_invalid', 'Scheduled teardown lacks its exact hash-bound terminal receipt');
  }
  return clone(receipt);
}

function resultState(receipts) {
  if (receipts.some((entry) => entry.state === 'safety_stopped')) return 'safety_stopped';
  if (receipts.some((entry) => entry.state === 'effect_uncertain')) return 'harness_failure';
  if (receipts.some((entry) => entry.receipt?.state === 'failed')) return 'new_product_failure';
  return 'passed';
}

function plannedTime(caseWindows, entry) {
  const window = caseWindows[entry.attempt.caseId];
  if (!window) fail('case_window_missing', `${entry.attempt.caseId} has no frozen future barrier`);
  const start = Date.parse(window.startAt);
  const end = Date.parse(window.endAt);
  if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) fail('case_window_invalid', entry.attempt.caseId);
  const required = entry.attempt.caseId === 'C01' ? 1_200_000
    : entry.attempt.caseId === 'C04' ? 28_800_000
      : entry.attempt.caseId === 'C05' ? 86_400_000 : 0;
  if (end - start < required) fail('case_window_invalid', `${entry.attempt.caseId} duration is below contract`);
  const offset = entry.attempt.executionUnit.plannedOffsetSeconds ?? 0;
  return { wallTime: new Date(start + offset * 1000).toISOString(), endAt: window.endAt };
}

function validateWindowOrder(caseWindows) {
  let priorEnd = -Infinity;
  for (const caseId of P158_W9_CASE_IDS) {
    const window = caseWindows?.[caseId];
    const start = Date.parse(window?.startAt);
    const end = Date.parse(window?.endAt);
    if (!Number.isFinite(start) || !Number.isFinite(end) || start < priorEnd || end < start) {
      fail('case_window_invalid', 'W9 case windows must be complete, ordered, and non-overlapping');
    }
    priorEnd = end;
  }
}

export function createDistributedC01LiveHook({ config, runRoot, fetch, clock, scheduler, loadExternalEvidence }) {
  return async ({ actions }) => {
    const preparation = await prepareLiveDistributedCalibration({ config, runRoot, fetch, clock });
    const local = await startLiveDistributedCalibration({ runRoot, fetch, clock, scheduler });
    const external = await loadExternalEvidence();
    const result = await finalizeLiveDistributedCalibration({
      runRoot, externalAggregate: external.aggregate, externalReceipts: external.receipts, clock,
    });
    return {
      preparationSha256: preparation.envelopeSha256,
      localSha256: local.localEnvelopeSha256,
      resultSha256: sha256(result),
      result,
    };
  };
}

function c01ActionReceipts(aggregate, actions, target, schedule) {
  const result = aggregate?.result;
  if (!/^[a-f0-9]{64}$/u.test(aggregate?.preparationSha256 ?? '') ||
      !/^[a-f0-9]{64}$/u.test(aggregate?.localSha256 ?? '') ||
      aggregate?.resultSha256 !== sha256(result) || result?.calibration?.clean !== true ||
      result?.distributedEvidence?.serviceCommandCount !== 500 ||
      result.distributedEvidence.dashboardActionCount !== 50 ||
      result.distributedEvidence.handoffReconnectCount !== 10 ||
      result.distributedEvidence.externalReplayEffectCount !== 0 ||
      !Array.isArray(result.observations) || result.observations.length !== 560 ||
      !Array.isArray(result.artifacts) || result.artifacts.length === 0 ||
      result.artifacts.some((artifact) => artifact.declaredSha256 !== sha256(artifact.content))) {
    fail('c01_distributed_receipt_invalid', 'C01 did not return the existing clean distributed live-driver result');
  }
  const artifactIds = result.artifacts.map((artifact) => artifact.artifactId);
  const queues = new Map();
  for (const action of actions) {
    if (!queues.has(action.kind)) queues.set(action.kind, []);
    queues.get(action.kind).push(action);
  }
  const cursors = new Map();
  const receipts = result.observations.map((observation) => {
    const list = queues.get(observation.kind) ?? [];
    const index = cursors.get(observation.kind) ?? 0;
    const action = list[index];
    cursors.set(observation.kind, index + 1);
    if (!action) fail('c01_receipt_allocation_mismatch', `Unexpected C01 ${observation.kind} observation`);
    const body = {
      schemaVersion: 'agent-browser.p158-w9-action-receipt.v1',
      runId: target.runId, candidateSha256: target.candidateSha256,
      scheduleSha256: schedule.scheduleSha256,
      workflowRunId: target.workflowRunId, workflowRunAttempt: target.workflowRunAttempt,
      caseId: action.caseId, attemptId: action.attemptId, actionId: action.actionId,
      environmentId: action.environmentId, kind: action.kind, attempt: 1,
      state: observation.state === 'passed' ? 'passed' : 'failed', observedAt: observation.observedAt,
      effectClass: action.transport === 'external_ingress' ? 'external_ingress' : 'read_only',
      evidenceArtifactIds: artifactIds,
      retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
      ...(action.transport === 'external_ingress' ? { externalEvidence: {
        offHost: true, outsideServiceNetworkNamespace: true, operatorVisibleState: 'ready',
        readyBeforePixels: true, pixelsObserved: true,
        externalVantageAggregateSha256: target.externalVantageAggregateSha256,
        externalHandoffOracleSha256: target.externalHandoffOracleSha256,
        handoffUrlSha256: target.handoffUrlSha256,
        retainedIdentitySha256: target.retainedIdentitySha256,
      } } : {}),
      ...(observation.state === 'passed' ? {} : { failure: observation.failure ?? { code: 'c01_observation_failed' } }),
    };
    return { ...body, receiptSha256: canonicalW9ReceiptDigest(body) };
  });
  const byActionId = new Map(receipts.map((receipt) => [receipt.actionId, receipt]));
  return actions.map((action) => byActionId.get(action.actionId));
}

export async function runP158W9Phase({
  schedule, target, caseWindows, drivers, controller, runRoot, artifactStore, clock, scheduler, safetyStop,
}) {
  assertSchedule(schedule);
  assertP158W9DevelopmentTarget(target);
  assertDriverBindings(drivers);
  validateWindowOrder(caseWindows);
  if (!isAbsolute(runRoot ?? '')) fail('run_root_invalid', 'W9 requires an absolute runtime root');
  const fromRepo = relative(REPO_ROOT, resolve(runRoot));
  if (fromRepo === '' || (!fromRepo.startsWith('..') && !isAbsolute(fromRepo))) {
    fail('run_root_inside_repository', 'W9 runtime evidence must remain outside the product repository');
  }
  const sourceDigest = sha256({ schedule, target, caseWindows, hookBindings: drivers.hookBindings });
  const sourceSnapshot = clone({ schedule, target, caseWindows, hookBindings: drivers.hookBindings });
  const store = artifactStore ?? createFileArtifactStore(runRoot);
  const plan = buildP158W9ActionPlan(schedule);
  const phaseStartPath = checkpointPath('phase', 'started');
  let phaseStart = await readOptional(store, phaseStartPath);
  if (!phaseStart) {
    if (Date.parse(clock.wallNow()) > Date.parse(caseWindows.C01.startAt)) {
      fail('late_phase_start', 'A new W9 execution must begin before its frozen C01 barrier');
    }
    phaseStart = await writeCheckpoint(store, phaseStartPath, {
      schemaVersion: 'agent-browser.p158-w9-phase-checkpoint.v1', state: 'started', sourceDigest,
      observedAt: clock.wallNow(), retryAttempted: false, repairAttempted: false,
    });
  } else {
    verifyCheckpoint(phaseStart);
    if (phaseStart.sourceDigest !== sourceDigest) fail('source_config_mutated', 'W9 inputs changed across process restart');
  }
  if (controller.snapshot().state === 'frozen') await controller.startExecution();

  const c01Actions = plan.filter((entry) => entry.attempt.caseId === 'C01').flatMap((entry) => entry.actions);
  let stopped = safetyStop && c01Actions.length > 0
    ? clone(await safetyStop({ action: clone(c01Actions[0]), stage: 'before_distributed_c01' })) ?? null
    : null;
  let c01Aggregate = null;
  if (!stopped && typeof drivers?.executeDistributedC01 === 'function') {
    const c01StartedPath = checkpointPath('aggregate-started', 'C01');
    const c01TerminalPath = checkpointPath('aggregate-terminal', 'C01');
    c01Aggregate = await readOptional(store, c01TerminalPath);
    if (!c01Aggregate) {
      const priorStart = await readOptional(store, c01StartedPath);
      if (priorStart) {
        verifyCheckpoint(priorStart);
        c01Aggregate = { state: 'effect_uncertain', failure: { code: 'interrupted_after_c01_start' } };
      } else {
        await writeCheckpoint(store, c01StartedPath, {
          caseId: 'C01', state: 'started', sourceDigest, observedAt: clock.wallNow(),
        });
        const aggregate = await drivers.executeDistributedC01({
          actions: clone(c01Actions), target: clone(target), safetyStop,
        });
        const actionReceipts = c01ActionReceipts(aggregate, c01Actions, target, schedule);
        if (actionReceipts.length !== c01Actions.length ||
            actionReceipts.some((receipt, index) => receipt.actionId !== c01Actions[index].actionId)) {
          fail('c01_receipt_allocation_mismatch', 'Distributed C01 did not cover the exact frozen action allocation');
        }
        for (let index = 0; index < c01Actions.length; index += 1) {
          validateReceipt(c01Actions[index], actionReceipts[index], target, schedule);
        }
        c01Aggregate = await writeCheckpoint(store, c01TerminalPath, {
          caseId: 'C01', state: 'terminal', aggregateSha256: aggregate.resultSha256,
          preparationSha256: aggregate.preparationSha256, localSha256: aggregate.localSha256,
          actionReceipts, observedAt: clock.wallNow(),
        });
      }
    } else verifyCheckpoint(c01Aggregate);
  }

  for (const entry of plan) {
    const barrier = plannedTime(caseWindows, entry);
    await scheduler.waitUntil({ wallTime: barrier.wallTime, attemptId: entry.attempt.attemptId });
    const terminalReceipts = [];
    for (const action of entry.actions) {
      const terminalPath = checkpointPath('actions-terminal', action.actionId);
      const existingTerminal = await readOptional(store, terminalPath);
      if (existingTerminal) {
        terminalReceipts.push(verifyCheckpoint(existingTerminal));
        continue;
      }
      const startedPath = checkpointPath('actions-started', action.actionId);
      const existingStarted = await readOptional(store, startedPath);
      if (existingStarted) {
        verifyCheckpoint(existingStarted);
        terminalReceipts.push(await writeCheckpoint(store, terminalPath, {
          actionId: action.actionId, state: 'effect_uncertain', observedAt: clock.wallNow(),
          failure: { code: 'interrupted_after_action_start' }, retryAttempted: false, repairAttempted: false,
        }));
        continue;
      }
      await writeCheckpoint(store, startedPath, {
        actionId: action.actionId, state: 'started', observedAt: clock.wallNow(), sourceDigest,
      });
      if (!stopped && safetyStop) stopped = clone(await safetyStop({ action: clone(action) })) ?? null;
      if (stopped) {
        terminalReceipts.push(await writeCheckpoint(store, terminalPath, {
          actionId: action.actionId, state: 'safety_stopped', observedAt: clock.wallNow(), safetyStop: stopped,
          retryAttempted: false, repairAttempted: false,
        }));
        continue;
      }
      if (action.caseId === 'C01' && c01Aggregate) {
        if (c01Aggregate.state === 'effect_uncertain') {
          terminalReceipts.push(await writeCheckpoint(store, terminalPath, {
            actionId: action.actionId, state: 'effect_uncertain', observedAt: clock.wallNow(),
            failure: clone(c01Aggregate.failure), retryAttempted: false, repairAttempted: false,
          }));
        } else {
          const receipt = c01Aggregate.actionReceipts.find((item) => item.actionId === action.actionId);
          terminalReceipts.push(await writeCheckpoint(store, terminalPath, {
            actionId: action.actionId, state: 'terminal', receipt: validateReceipt(action, receipt, target, schedule),
          }));
        }
      } else {
        const method = DRIVER_METHODS[action.kind];
        if (typeof drivers?.[method] !== 'function') fail('driver_missing', `${method} is required`);
        const receipt = validateReceipt(action, await drivers[method](clone({ ...action, target })), target, schedule);
        terminalReceipts.push(await writeCheckpoint(store, terminalPath, {
          actionId: action.actionId, state: 'terminal', receipt,
        }));
      }
    }
    const attemptPath = checkpointPath('attempts', entry.attempt.attemptId);
    let terminal = await readOptional(store, attemptPath);
    if (!terminal) {
      terminal = await writeCheckpoint(store, attemptPath, {
        caseId: entry.attempt.caseId, attemptId: entry.attempt.attemptId,
        resultState: resultState(terminalReceipts), actionCount: terminalReceipts.length,
        actionReceiptSha256: sha256(terminalReceipts), completedAt: clock.wallNow(),
        blocksDependents: terminalReceipts.some((receipt) => receipt.state === 'effect_uncertain'),
        retryDisposition: 'prohibited_opportunistic_retry',
      });
    } else verifyCheckpoint(terminal);
    const recorded = controller.snapshot().results.some((item) => item.attemptId === entry.attempt.attemptId);
    if (!recorded) await controller.recordAttempt(terminal);
  }

  for (const caseId of ['C01', 'C04', 'C05']) {
    await scheduler.waitUntil({ wallTime: caseWindows[caseId].endAt, caseId });
  }
  if (sha256({ schedule, target, caseWindows, hookBindings: drivers.hookBindings }) !== sourceDigest ||
      sha256(sourceSnapshot) !== sourceDigest) {
    fail('source_config_mutated', 'W9 source or configuration mutated during execution');
  }
  const teardownPath = checkpointPath('phase', 'scheduled-teardown');
  let teardown = await readOptional(store, teardownPath);
  if (!teardown) {
    const receipt = validateTeardownReceipt(
      await drivers.executeScheduledTeardown({ target: clone(target), scheduleSha256: schedule.scheduleSha256, attempt: 1 }),
      target,
      schedule,
    );
    teardown = await writeCheckpoint(store, teardownPath, { state: 'terminal', receipt });
  } else verifyCheckpoint(teardown);
  if (!controller.snapshot().scheduledTeardown?.resultState) {
    await controller.recordScheduledTeardown({
      resultState: teardown.receipt.state === 'passed' ? 'passed' : 'new_product_failure',
      effectState: teardown.receipt.state === 'passed' ? 'verified_effect' : 'effect_uncertain',
      evidence: { checkpointSha256: teardown.checkpointSha256 },
    });
  }
  if (typeof drivers.verifyEvidenceArtifact !== 'function') {
    fail('evidence_verifier_missing', 'W9 sealing requires an exact evidence artifact verifier');
  }
  const artifactIds = new Set(teardown.receipt.evidenceArtifactIds);
  for (const entry of plan) {
    for (const action of entry.actions) {
      verifyCheckpoint(await readOptional(store, checkpointPath('actions-started', action.actionId)));
      const terminal = verifyCheckpoint(await readOptional(store, checkpointPath('actions-terminal', action.actionId)));
      for (const artifactId of terminal.receipt?.evidenceArtifactIds ?? []) artifactIds.add(artifactId);
    }
    verifyCheckpoint(await readOptional(store, checkpointPath('attempts', entry.attempt.attemptId)));
  }
  for (const artifactId of [...artifactIds].sort()) {
    if (await drivers.verifyEvidenceArtifact(artifactId) !== true) {
      fail('evidence_artifact_unproven', `Evidence artifact ${artifactId} is missing or changed`);
    }
  }
  const auditPath = checkpointPath('phase', 'evidence-audit');
  let audit = await readOptional(store, auditPath);
  if (!audit) {
    audit = await writeCheckpoint(store, auditPath, {
      state: 'complete', sourceDigest, scheduleSha256: schedule.scheduleSha256,
      expectedActionCount: plan.reduce((sum, entry) => sum + entry.actions.length, 0),
      terminalActionCount: plan.reduce((sum, entry) => sum + entry.actions.length, 0),
      expectedAttemptCount: plan.length, terminalAttemptCount: plan.length,
      evidenceArtifactIds: [...artifactIds].sort(), teardownCheckpointSha256: teardown.checkpointSha256,
      auditedAt: clock.wallNow(), retryAttempted: false, repairAttempted: false,
    });
  } else verifyCheckpoint(audit);
  if (controller.snapshot().state === 'executing') await controller.finishExecution();
  if (controller.snapshot().state === 'execution_terminal') await controller.sealEvidence();
  return {
    state: controller.snapshot().state, sourceDigest, safetyStop: stopped,
    planAttemptCount: plan.length, evidenceAuditSha256: audit.checkpointSha256,
  };
}
