import { sha256 } from './p158-campaign-controller.js';
import { runC01Calibration } from './p158-calibration-runner.js';

const EXTERNAL_ACTION_COUNTS = Object.freeze({
  dashboard_action: 50,
  handoff_reconnect: 10,
});
const SERVICE_ACTIONS = Object.freeze([
  'service_status',
  'resource_inventory',
  'incident_summary',
  'profile_source',
  'site_policy',
]);

export class DistributedCalibrationError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'DistributedCalibrationError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new DistributedCalibrationError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function withoutReceiptDigest(receipt) {
  const { receiptSha256: _receiptSha256, ...body } = receipt ?? {};
  return body;
}

export function canonicalExternalRunnerReceiptDigest(receipt) {
  return sha256(withoutReceiptDigest(receipt));
}

function validateBinding(input, receipts) {
  if (typeof input.runId !== 'string' || input.runId.length === 0 ||
      typeof input.workflowRunId !== 'string' || !/^\d+$/u.test(input.workflowRunId) ||
      !Number.isInteger(input.workflowRunAttempt) || input.workflowRunAttempt < 1 ||
      typeof input.sourceCommit !== 'string' || !/^[a-f0-9]{40}$/u.test(input.sourceCommit)) {
    fail('invalid_workflow_binding', 'runId, workflowRunId, and a full source commit are required');
  }
  for (const receipt of receipts) {
    if (receipt.runId !== input.runId || receipt.workflowRunId !== input.workflowRunId ||
        receipt.sourceCommit !== input.sourceCommit ||
        receipt.workflowRunAttempt !== input.workflowRunAttempt) {
      fail('external_receipt_binding_mismatch', 'External receipt does not bind the requested workflow run and commit', {
        receiptId: receipt.receiptId,
      });
    }
  }
}

function validateReceiptIdentities(receipts, e2Target) {
  for (const field of ['receiptId', 'viewerId']) {
    const values = receipts.map((receipt) => receipt?.[field]);
    if (values.some((value) => typeof value !== 'string' || value.length === 0) ||
        new Set(values).size !== receipts.length) {
      fail('external_receipt_identity_mismatch', `External receipts require distinct ${field} values`, values);
    }
  }
  const runnerIds = receipts.map((receipt) => receipt?.runnerIdentity?.runnerId);
  if (runnerIds.some((value) => typeof value !== 'string' || value.length === 0) ||
      new Set(runnerIds).size !== receipts.length) {
    fail('external_receipt_identity_mismatch', 'External receipts require two distinct GitHub runner identities', runnerIds);
  }
  for (const receipt of receipts) {
    if (receipt.schemaVersion !== 'agent-browser.p158-external-calibration-receipt.v1' ||
        receipt.runnerIdentity.provider !== 'github_actions' ||
        !receipt.runnerIdentity.runnerName || !receipt.runnerIdentity.runnerOs ||
        !receipt.runnerIdentity.runnerArch || receipt.outsideServiceHost !== true ||
        receipt.outsideServiceNetworkNamespace !== true || receipt.publicEgressObserved !== true ||
        receipt.handoffUrl !== e2Target?.handoffUrl) {
      fail('external_receipt_identity_mismatch', 'External runner identity or durable handoff proof is invalid', {
        receiptId: receipt.receiptId,
      });
    }
  }
}

function validateReceiptHashes(receipts) {
  for (const receipt of receipts) {
    const actual = canonicalExternalRunnerReceiptDigest(receipt);
    if (receipt.receiptSha256 !== actual) {
      fail('external_receipt_hash_mismatch', 'External runner receipt self-hash does not agree', {
        receiptId: receipt.receiptId,
        declared: receipt.receiptSha256,
        actual,
      });
    }
  }
}

function validateSharedWindow(receipts) {
  const starts = receipts.map((receipt) => receipt.startedAt);
  const ends = receipts.map((receipt) => receipt.completedAt);
  const startMs = Date.parse(starts[0]);
  const endMs = Date.parse(ends[0]);
  if (new Set(starts).size !== 1 || new Set(ends).size !== 1 || !Number.isFinite(startMs) ||
      !Number.isFinite(endMs) || endMs - startMs < 20 * 60_000) {
    fail('external_receipt_window_mismatch', 'External receipts must share one window spanning at least 20 minutes', {
      starts,
      ends,
    });
  }
  return { startedAt: starts[0], completedAt: ends[0], startMs, endMs };
}

function indexExternalActions(receipts, window) {
  const index = new Map();
  const counts = { dashboard_action: 0, handoff_reconnect: 0 };
  for (const receipt of receipts) {
    if (!Array.isArray(receipt.actions)) {
      fail('external_action_count_mismatch', 'External receipt actions must be an array', receipt.receiptId);
    }
    for (const action of receipt.actions) {
      const expectedCount = EXTERNAL_ACTION_COUNTS[action?.kind];
      const observedMs = Date.parse(action?.observedAt);
      if (!expectedCount || !Number.isInteger(action.ordinal) || action.ordinal < 1 ||
          action.ordinal > expectedCount || action.viewerId !== receipt.viewerId ||
          action.attempt !== 1 || action.retryAttempted !== false || action.repairAttempted !== false ||
          !['passed', 'failed'].includes(action.state) || !Number.isFinite(observedMs) ||
          observedMs < window.startMs || observedMs > window.endMs ||
          !Number.isFinite(action.latencyMs) || action.latencyMs < 0 ||
          (action.state === 'failed' && (!action.failure?.code || !action.failure?.message))) {
        fail('external_action_contract_mismatch', 'External action violates timing, attempt, or terminal-result contract', {
          receiptId: receipt.receiptId,
          action,
        });
      }
      const key = `${action.kind}:${action.ordinal}`;
      if (index.has(key)) {
        fail('external_action_count_mismatch', 'External action kind ordinals must be globally unique', { key });
      }
      index.set(key, { receipt, action });
      counts[action.kind] += 1;
    }
  }
  for (const [kind, expected] of Object.entries(EXTERNAL_ACTION_COUNTS)) {
    if (counts[kind] !== expected ||
        Array.from({ length: expected }, (_, offset) => `${kind}:${offset + 1}`).some((key) => !index.has(key))) {
      fail('external_action_count_mismatch', `External receipts must prove exactly ${expected} ${kind} actions`, counts);
    }
  }
  return index;
}

function preflight(input) {
  if (!Array.isArray(input.externalRunnerReceipts) || input.externalRunnerReceipts.length !== 2) {
    fail('external_receipt_count_mismatch', 'Exactly two external runner receipts are required');
  }
  if (typeof input.serviceTransport?.executeReadOnlyCommand !== 'function') {
    fail('invalid_service_transport', 'serviceTransport.executeReadOnlyCommand is required');
  }
  const receipts = clone(input.externalRunnerReceipts);
  const e2Target = input.developmentTargets?.find((target) => target.environmentId === 'E2');
  validateReceiptHashes(receipts);
  validateBinding(input, receipts);
  validateReceiptIdentities(receipts, e2Target);
  const window = validateSharedWindow(receipts);
  const actions = indexExternalActions(receipts, window);
  return { receipts, window, actions };
}

function externalReplay(actions, kind, request) {
  const evidence = actions.get(`${kind}:${request.actionOrdinal}`);
  if (!evidence || evidence.action.viewerId !== request.clientId) {
    fail('external_action_identity_mismatch', 'External replay request does not match the frozen receipt identity', {
      kind,
      actionOrdinal: request.actionOrdinal,
      clientId: request.clientId,
    });
  }
  const projection = {
    state: evidence.action.state,
    outcome: evidence.action.state,
    latencyMs: evidence.action.latencyMs,
    observedAt: evidence.action.observedAt,
    source: 'external_runner_receipt',
    receiptId: evidence.receipt.receiptId,
    runnerId: evidence.receipt.runnerIdentity.runnerId,
    performedLocally: false,
    attempt: 1,
    retryAttempted: false,
    repairAttempted: false,
  };
  if (evidence.action.state === 'failed') {
    const error = new Error(evidence.action.failure.message);
    error.code = evidence.action.failure.code;
    error.externalEvidence = projection;
    throw error;
  }
  return projection;
}

export async function runDistributedC01Calibration(input) {
  const { receipts, window, actions } = preflight(input);
  const observedBeforeStart = Date.parse(input.clock?.wallNow?.());
  if (!Number.isFinite(observedBeforeStart) || observedBeforeStart > window.startMs) {
    fail('shared_window_timing_mismatch', 'Service calibration cannot start after the external shared window begins');
  }
  if (typeof input.scheduler?.waitUntil !== 'function') {
    fail('invalid_scheduler', 'Distributed calibration requires an injected scheduler');
  }
  await input.scheduler.waitUntil({ wallTime: window.startedAt, action: null });

  const serviceEffects = {
    executeServiceCommand: async (request) => {
      const action = SERVICE_ACTIONS[(request.actionOrdinal - 1) % SERVICE_ACTIONS.length];
      const response = await input.serviceTransport.executeReadOnlyCommand({
        ...clone(request),
        action,
        effectClass: 'read_only',
      });
      if (!response || !['read_only', 'harmless'].includes(response.effectClass) ||
          response.attempt !== 1 || response.retryAttempted !== false ||
          response.repairAttempted !== false || !['passed', 'failed'].includes(response.state) ||
          !Number.isFinite(response.latencyMs) || response.latencyMs < 0 ||
          !Number.isFinite(Date.parse(response.observedAt)) ||
          Date.parse(response.observedAt) < Date.parse(request.plannedAt) ||
          Date.parse(response.observedAt) > window.endMs) {
        const error = new Error('Service response did not prove a once-only read-only or harmless effect');
        error.code = 'service_effect_contract_mismatch';
        throw error;
      }
      if (response.state === 'failed') {
        const error = new Error(response.failure?.message ?? 'Read-only Service command failed');
        error.code = response.failure?.code ?? 'service_command_failed';
        throw error;
      }
      return clone(response);
    },
    executeDashboardAction: (request) => externalReplay(actions, 'dashboard_action', request),
    executeHandoffReconnect: (request) => externalReplay(actions, 'handoff_reconnect', request),
  };
  const externalViewerReceipts = receipts.map((receipt) => ({
    ...receipt,
    capturedAt: receipt.completedAt,
    external: true,
  }));
  const schedulePlanner = ({ kind, actionOrdinal, defaultPlannedAt }) => {
    if (kind === 'service_command') {
      return new Date(
        window.startMs + Math.floor(((actionOrdinal - 1) * (window.endMs - window.startMs)) / 500),
      ).toISOString();
    }
    return actions.get(`${kind}:${actionOrdinal}`)?.action.observedAt ?? defaultPlannedAt;
  };
  const result = await runC01Calibration({
    calibrationId: input.calibrationId,
    developmentTargets: clone(input.developmentTargets),
    agentClientIds: clone(input.agentClientIds),
    externalViewerReceipts,
    externalReceiptNotAfter: window.completedAt,
    effects: serviceEffects,
    scheduler: input.scheduler,
    schedulePlanner,
    safetyStop: input.safetyStop,
    clock: input.clock,
  });
  const resultStart = Date.parse(result.calibration.startedAt);
  const resultEnd = Date.parse(result.calibration.completedAt);
  if (resultStart < window.startMs || resultEnd > window.endMs) {
    fail('shared_window_timing_mismatch', 'Service and external calibration evidence do not share the frozen window', {
      service: { startedAt: result.calibration.startedAt, completedAt: result.calibration.completedAt },
      external: { startedAt: window.startedAt, completedAt: window.completedAt },
    });
  }
  return {
    ...result,
    distributedEvidence: {
      runId: input.runId,
      sourceCommit: input.sourceCommit,
      workflowRunId: input.workflowRunId,
      workflowRunAttempt: input.workflowRunAttempt,
      externalReceiptIds: receipts.map((receipt) => receipt.receiptId).sort(),
      externalRunnerIds: receipts.map((receipt) => receipt.runnerIdentity.runnerId).sort(),
      sharedWindowStartedAt: window.startedAt,
      sharedWindowCompletedAt: window.completedAt,
      sharedWindowDurationMs: window.endMs - window.startMs,
      serviceCommandCount: result.observations.filter(
        (entry) => entry.kind === 'service_command' && entry.attempt === 1,
      ).length,
      dashboardActionCount: EXTERNAL_ACTION_COUNTS.dashboard_action,
      handoffReconnectCount: EXTERNAL_ACTION_COUNTS.handoff_reconnect,
      externalReplayEffectCount: 0,
      retryAttempted: false,
      repairAttempted: false,
    },
  };
}
