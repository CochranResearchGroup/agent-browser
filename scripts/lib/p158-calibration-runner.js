import { isAbsolute } from 'node:path';

import { canonicalJson, sha256 } from './p158-campaign-controller.js';
import { canonicalCalibrationDigest } from './p158-campaign-preparation.js';

export const C01_WORKLOAD = Object.freeze({
  durationMinutes: 20,
  agentClients: 25,
  externalViewers: 2,
  controllers: 1,
  serviceCommands: 500,
  dashboardActions: 50,
  handoffReconnects: 10,
});

const ACTION_COUNTS = Object.freeze([
  ['service_command', C01_WORKLOAD.serviceCommands],
  ['dashboard_action', C01_WORKLOAD.dashboardActions],
  ['handoff_reconnect', C01_WORKLOAD.handoffReconnects],
]);
const TOTAL_ACTIONS = ACTION_COUNTS.reduce((total, [, count]) => total + count, 0);
const REQUIRED_ENVIRONMENTS = Object.freeze(['E1', 'E2']);

export class CalibrationError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'CalibrationError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new CalibrationError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function assertUrl(value, field, protocols) {
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    fail('invalid_development_target', `${field} must be an explicit absolute URL`, { field, value });
  }
  if (!protocols.includes(parsed.protocol) || !parsed.hostname) {
    fail('invalid_development_target', `${field} uses an unsupported URL`, { field, value });
  }
}

function validateTargets(targets) {
  if (!Array.isArray(targets) || targets.length !== 2) {
    fail('invalid_development_target', 'C01 requires exact E1 and E2 development targets');
  }
  const ids = targets.map((target) => target?.environmentId);
  if (JSON.stringify([...ids].sort()) !== JSON.stringify(REQUIRED_ENVIRONMENTS)) {
    fail('invalid_development_target', 'C01 development targets must be exact E1 and E2', { ids });
  }
  for (const target of targets) {
    if (target.scope !== 'development') {
      fail('invalid_development_target', 'Calibration refuses non-development target scope', target);
    }
    assertUrl(target.serviceUrl, `${target.environmentId}.serviceUrl`, ['http:', 'https:']);
    assertUrl(target.dashboardUrl, `${target.environmentId}.dashboardUrl`, ['http:', 'https:']);
    assertUrl(target.handoffUrl, `${target.environmentId}.handoffUrl`, ['https:']);
    if (typeof target.profileRoot !== 'string' || !isAbsolute(target.profileRoot)) {
      fail('invalid_development_target', 'Development profileRoot must be an explicit absolute path', target);
    }
  }
}

function validateClientIds(ids) {
  if (!Array.isArray(ids) || ids.length !== C01_WORKLOAD.agentClients || new Set(ids).size !== ids.length ||
      ids.some((id) => typeof id !== 'string' || id.length === 0)) {
    fail('invalid_agent_clients', 'C01 requires 25 distinct explicit agent client IDs');
  }
}

function validateViewerReceipts(receipts, targets, notAfter) {
  const e2Handoff = targets.find((target) => target.environmentId === 'E2').handoffUrl;
  if (!Array.isArray(receipts) || receipts.length !== C01_WORKLOAD.externalViewers ||
      new Set(receipts.map((receipt) => receipt?.viewerId)).size !== receipts.length ||
      new Set(receipts.map((receipt) => receipt?.receiptId)).size !== receipts.length) {
    fail('invalid_external_viewer_receipts', 'C01 requires two distinct externally supplied viewer receipts');
  }
  for (const receipt of receipts) {
    if (!receipt?.viewerId || !receipt?.receiptId || !Number.isFinite(Date.parse(receipt.capturedAt)) ||
        Date.parse(receipt.capturedAt) > notAfter ||
        receipt.handoffUrl !== e2Handoff || receipt.external !== true ||
        receipt.outsideServiceHost !== true || receipt.outsideServiceNetworkNamespace !== true ||
        receipt.publicEgressObserved !== true) {
      fail('invalid_external_viewer_receipts', 'External viewer receipt does not prove off-host public ingress', receipt);
    }
  }
}

function validateClientSeparation(agentClientIds, viewerReceipts) {
  const agentIds = new Set(agentClientIds);
  const overlaps = viewerReceipts
    .map((receipt) => receipt.viewerId)
    .filter((viewerId) => agentIds.has(viewerId));
  if (overlaps.length > 0) {
    fail('invalid_external_viewer_receipts', 'External viewer and agent client identities must be distinct', overlaps);
  }
}

function validateDependencies(input) {
  const requiredEffects = [
    'executeServiceCommand',
    'executeDashboardAction',
    'executeHandoffReconnect',
  ];
  for (const method of requiredEffects) {
    if (typeof input.effects?.[method] !== 'function') {
      fail('invalid_effects', `effects.${method} is required`);
    }
  }
  if (typeof input.clock?.wallNow !== 'function' || typeof input.clock?.monotonicNow !== 'function') {
    fail('invalid_clock', 'An injected wall and monotonic clock is required');
  }
  if (input.scheduler !== undefined && typeof input.scheduler?.waitUntil !== 'function') {
    fail('invalid_scheduler', 'scheduler.waitUntil must be a function');
  }
  if (input.schedulePlanner !== undefined && typeof input.schedulePlanner !== 'function') {
    fail('invalid_scheduler', 'schedulePlanner must be a function');
  }
  if (input.safetyStop !== undefined && typeof input.safetyStop !== 'function') {
    fail('invalid_safety_stop', 'safetyStop must be a function');
  }
}

function actionPlans(targets, clientIds, viewerReceipts, startedMs, schedulePlanner) {
  const plans = [];
  let ordinal = 0;
  for (const [kind, count] of ACTION_COUNTS) {
    for (let actionOrdinal = 1; actionOrdinal <= count; actionOrdinal += 1) {
      ordinal += 1;
      const target = kind === 'service_command'
        ? targets[(actionOrdinal - 1) % targets.length]
        : targets.find((entry) => entry.environmentId === 'E2');
      const viewer = viewerReceipts[(actionOrdinal - 1) % viewerReceipts.length];
      const defaultPlannedAt = new Date(
        startedMs + Math.floor(((ordinal - 1) * C01_WORKLOAD.durationMinutes * 60_000) / TOTAL_ACTIONS),
      ).toISOString();
      const plannedAt = schedulePlanner
        ? schedulePlanner({ kind, ordinal, actionOrdinal, defaultPlannedAt })
        : defaultPlannedAt;
      if (!Number.isFinite(Date.parse(plannedAt))) {
        fail('invalid_scheduler', 'schedulePlanner returned an invalid wall time', {
          kind,
          ordinal,
          actionOrdinal,
          plannedAt,
        });
      }
      plans.push({
        ordinal,
        actionOrdinal,
        kind,
        plannedAt,
        target: clone(target),
        clientId: kind === 'service_command'
          ? clientIds[(actionOrdinal - 1) % clientIds.length]
          : viewer.viewerId,
        externalViewerReceiptId: kind === 'service_command' ? undefined : viewer.receiptId,
      });
    }
  }
  return plans;
}

function normalizeFailure(error) {
  return {
    code: typeof error?.code === 'string' && error.code.length > 0 ? error.code : 'effect_failed',
    name: typeof error?.name === 'string' && error.name.length > 0 ? error.name : 'Error',
    message: typeof error?.message === 'string' && error.message.length > 0
      ? error.message
      : String(error),
  };
}

function percentile(values, fraction) {
  if (values.length === 0) return 0;
  const ordered = [...values].sort((left, right) => left - right);
  return ordered[Math.max(0, Math.ceil(ordered.length * fraction) - 1)];
}

function metricFor(observations, kind) {
  const samples = observations
    .filter((entry) => entry.kind === kind && entry.state === 'passed')
    .map((entry) => entry.result?.latencyMs)
    .filter((value) => Number.isFinite(value) && value >= 0);
  return {
    sampleCount: samples.length,
    p50Ms: percentile(samples, 0.5),
    p95Ms: percentile(samples, 0.95),
    p99Ms: percentile(samples, 0.99),
    maximumMs: samples.length === 0 ? 0 : Math.max(...samples),
  };
}

function makeArtifact(calibrationId, kind, capturedAt, body) {
  const content = canonicalJson(body);
  return {
    artifactId: `${calibrationId}-${kind.replaceAll('_', '-')}`,
    kind,
    relativePath: `calibration/${calibrationId}/${kind}.json`,
    capturedAt,
    mediaType: 'application/json',
    contentEncoding: 'utf8',
    content,
    declaredSha256: sha256(content),
    declaredByteCount: Buffer.byteLength(content),
  };
}

export function canonicalCalibrationArtifact(value) {
  return canonicalJson(value);
}

export async function runC01Calibration(input) {
  validateTargets(input?.developmentTargets);
  validateClientIds(input?.agentClientIds);
  validateDependencies(input);
  if (typeof input.calibrationId !== 'string' || input.calibrationId.length === 0) {
    fail('invalid_calibration_id', 'calibrationId is required');
  }

  const targets = clone(input.developmentTargets);
  const clientIds = clone(input.agentClientIds);
  const viewerReceipts = clone(input.externalViewerReceipts);
  const startedAt = input.clock.wallNow();
  const startedMs = Date.parse(startedAt);
  const startedMonotonicTimeNanoseconds = input.clock.monotonicNow();
  if (!Number.isFinite(startedMs) || !Number.isFinite(startedMonotonicTimeNanoseconds)) {
    fail('invalid_clock', 'Injected clock returned an invalid start time');
  }
  const receiptNotAfter = input.externalReceiptNotAfter === undefined
    ? startedMs
    : Date.parse(input.externalReceiptNotAfter);
  if (!Number.isFinite(receiptNotAfter) || receiptNotAfter < startedMs) {
    fail('invalid_external_viewer_receipts', 'External receipt custody horizon is invalid');
  }
  validateViewerReceipts(viewerReceipts, targets, receiptNotAfter);
  validateClientSeparation(clientIds, viewerReceipts);
  const plans = actionPlans(targets, clientIds, viewerReceipts, startedMs, input.schedulePlanner);
  const observations = [];
  let activeSafetyStop = null;

  for (const plan of plans) {
    if (!activeSafetyStop && input.safetyStop) {
      const signal = await input.safetyStop({
        completedActionCount: observations.length,
        nextAction: clone(plan),
        observations: clone(observations),
      });
      if (signal) activeSafetyStop = clone(signal);
    }
    if (activeSafetyStop) {
      observations.push({
        ...plan,
        attempt: 0,
        state: 'safety_stopped',
        safetyStop: clone(activeSafetyStop),
      });
      continue;
    }
    if (input.scheduler) await input.scheduler.waitUntil({ wallTime: plan.plannedAt, action: clone(plan) });
    const request = { ...clone(plan), attempt: 1 };
    const observedAt = input.clock.wallNow();
    try {
      const method = plan.kind === 'service_command'
        ? input.effects.executeServiceCommand
        : plan.kind === 'dashboard_action'
          ? input.effects.executeDashboardAction
          : input.effects.executeHandoffReconnect;
      const result = await method(request);
      observations.push({ ...plan, attempt: 1, state: 'passed', observedAt, result: clone(result ?? {}) });
    } catch (error) {
      observations.push({ ...plan, attempt: 1, state: 'failed', observedAt, failure: normalizeFailure(error) });
    }
  }

  const requiredCompletedAt = startedMs + C01_WORKLOAD.durationMinutes * 60_000;
  if (input.scheduler) {
    await input.scheduler.waitUntil({
      wallTime: new Date(requiredCompletedAt).toISOString(),
      action: null,
    });
  }
  const completedAt = input.clock.wallNow();
  const completedMs = Date.parse(completedAt);
  const completedMonotonicTimeNanoseconds = input.clock.monotonicNow();
  if (!Number.isFinite(completedMs) || !Number.isFinite(completedMonotonicTimeNanoseconds) ||
      completedMs - startedMs < C01_WORKLOAD.durationMinutes * 60_000) {
    fail('calibration_duration_short', 'C01 calibration did not span the required 20 wall-clock minutes', {
      startedAt,
      completedAt,
      observations: clone(observations),
    });
  }

  const metrics = {
    serviceCommand: metricFor(observations, 'service_command'),
    dashboardAction: metricFor(observations, 'dashboard_action'),
    handoffReconnect: metricFor(observations, 'handoff_reconnect'),
  };
  const environmentRelativeBudgets = {
    agentCommandP95Ms: metrics.serviceCommand.p95Ms,
    dashboardActionP95Ms: metrics.dashboardAction.p95Ms,
    handoffReconnectP95Ms: metrics.handoffReconnect.p95Ms,
  };
  const failures = observations.filter((entry) => entry.state === 'failed');
  const safetyStopped = observations.filter((entry) => entry.state === 'safety_stopped');
  const rawArtifact = makeArtifact(input.calibrationId, 'calibration_raw', completedAt, {
    schemaVersion: 'agent-browser.p158-calibration-raw.v1',
    calibrationId: input.calibrationId,
    startedAt,
    completedAt,
    startedMonotonicTimeNanoseconds,
    completedMonotonicTimeNanoseconds,
    developmentTargets: targets,
    agentClientIds: clientIds,
    externalViewerReceipts: viewerReceipts,
    observations,
  });
  const summaryArtifact = makeArtifact(input.calibrationId, 'calibration_summary', completedAt, {
    schemaVersion: 'agent-browser.p158-calibration-summary.v1',
    calibrationId: input.calibrationId,
    workload: C01_WORKLOAD,
    plannedActionCount: TOTAL_ACTIONS,
    passedActionCount: observations.filter((entry) => entry.state === 'passed').length,
    failedActionCount: failures.length,
    safetyStoppedActionCount: safetyStopped.length,
    firstFailure: failures[0] ?? null,
    safetyStop: activeSafetyStop,
    metrics,
    retryAttempted: false,
    repairAttempted: false,
  });
  const budgetArtifact = makeArtifact(input.calibrationId, 'calibration_budget', completedAt, {
    schemaVersion: 'agent-browser.p158-calibration-budget.v1',
    calibrationId: input.calibrationId,
    frozen: true,
    frozenAt: completedAt,
    sourceSummarySha256: summaryArtifact.declaredSha256,
    environmentRelativeBudgets,
  });
  const calibration = {
    calibrationId: input.calibrationId,
    environmentIds: REQUIRED_ENVIRONMENTS,
    startedAt,
    completedAt,
    clean: failures.length === 0 && safetyStopped.length === 0,
    workload: C01_WORKLOAD,
    rawArtifactId: rawArtifact.artifactId,
    rawArtifactSha256: rawArtifact.declaredSha256,
    summaryArtifactId: summaryArtifact.artifactId,
    summaryArtifactSha256: summaryArtifact.declaredSha256,
    budgetArtifactId: budgetArtifact.artifactId,
    budgetSha256: budgetArtifact.declaredSha256,
    environmentRelativeBudgets,
  };
  calibration.declaredSha256 = canonicalCalibrationDigest(calibration);
  return {
    calibration,
    artifacts: [rawArtifact, summaryArtifact, budgetArtifact],
    observations,
    effectsAttempted: observations.some((entry) => entry.attempt === 1),
    retryAttempted: false,
    repairAttempted: false,
  };
}
