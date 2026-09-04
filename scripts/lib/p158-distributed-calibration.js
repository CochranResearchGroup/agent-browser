import { createHash } from 'node:crypto';
import { isAbsolute } from 'node:path';

import { canonicalJson, sha256 } from './p158-campaign-controller.js';
import { finalizeC01CalibrationEvidence } from './p158-calibration-runner.js';

const SERVICE_COMMAND_COUNT = 500;
const EXTERNAL_ACTION_COUNTS = Object.freeze({ dashboard_action: 50, handoff_reconnect: 10 });
const SERVICE_ACTIONS = Object.freeze([
  'service_status', 'resource_inventory', 'incident_summary', 'profile_source', 'site_policy',
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

function without(value, fields) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([field]) => !fields.includes(field)));
}

function runnerCanonicalize(value) {
  if (Array.isArray(value)) return value.map(runnerCanonicalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, runnerCanonicalize(value[key])]));
  }
  return value;
}

function externalRunnerHash(value) {
  return createHash('sha256').update(JSON.stringify(runnerCanonicalize(value))).digest('hex');
}

function expectedExternalSchedule(durationMs) {
  const events = [];
  for (let ordinal = 1; ordinal <= 25; ordinal += 1) {
    const offsetMs = Math.floor((ordinal * durationMs) / 26);
    events.push({ kind: 'dashboard_action', ordinal, offsetMs });
    if (ordinal % 5 === 0) {
      events.push({ kind: 'handoff_reconnect', ordinal: ordinal / 5, offsetMs });
    }
  }
  return events;
}

export function canonicalExternalDispatchDigest(descriptor) {
  return externalRunnerHash(without(descriptor, ['descriptorSha256']));
}

export function canonicalExternalRunnerReceiptDigest(receipt) {
  return sha256(without(receipt, ['receiptSha256']));
}

function preparedDigest(prepared) {
  return sha256(without(prepared, ['preparedSha256']));
}

function localRunDigest(localRun) {
  return sha256(without(localRun, ['localRunSha256']));
}

function validateTargets(targets) {
  if (!Array.isArray(targets) || targets.length !== 2 ||
      JSON.stringify(targets.map((target) => target?.environmentId).sort()) !== JSON.stringify(['E1', 'E2'])) {
    fail('invalid_development_target', 'Exact E1 and E2 development targets are required');
  }
  for (const target of targets) {
    if (target.scope !== 'development' || !isAbsolute(target.profileRoot ?? '') ||
        !/^[a-f0-9]{64}$/u.test(target.handoffUrlSha256 ?? '')) {
      fail('invalid_development_target', 'Targets require development scope, profile root, and handoff digest');
    }
    for (const field of ['serviceUrl', 'dashboardUrl']) {
      try {
        const url = new URL(target[field]);
        if (!['http:', 'https:'].includes(url.protocol) || !url.hostname) throw new Error('unsupported');
      } catch {
        fail('invalid_development_target', `${target.environmentId}.${field} is not an absolute development URL`);
      }
    }
    if ('handoffUrl' in target) fail('raw_handoff_forbidden', 'Distributed targets must contain only handoffUrlSha256');
  }
}

function validateAgentIds(ids) {
  if (!Array.isArray(ids) || ids.length !== 25 || new Set(ids).size !== 25 ||
      ids.some((id) => typeof id !== 'string' || id.length === 0)) {
    fail('invalid_agent_clients', 'Exactly 25 distinct explicit agent client IDs are required');
  }
}

function validateBinding(input) {
  if (!input.runId || !/^\d+$/u.test(input.workflowRunId ?? '') ||
      !Number.isInteger(input.workflowRunAttempt) || input.workflowRunAttempt < 1 ||
      !/^[a-f0-9]{40}$/u.test(input.sourceCommit ?? '')) {
    fail('invalid_workflow_binding', 'Run, GitHub workflow, attempt, and full commit bindings are required');
  }
}

function validateDispatch(descriptor, input) {
  if (!descriptor || descriptor.descriptorSha256 !== canonicalExternalDispatchDigest(descriptor)) {
    fail('external_dispatch_hash_mismatch', 'External dispatch descriptor self-hash does not agree');
  }
  if (descriptor.schemaVersion !== 'agent-browser.p158-external-calibration-dispatch.v1' ||
      descriptor.planId !== 'P158' || descriptor.runId !== input.runId ||
      descriptor.candidateCommit !== input.sourceCommit || descriptor.durationMs < 20 * 60_000 ||
      descriptor.actionCountPerClient !== 25 || descriptor.reconnectCountPerClient !== 5 ||
      descriptor.scheduleSha256 !== externalRunnerHash(expectedExternalSchedule(descriptor.durationMs))) {
    fail('external_dispatch_binding_mismatch', 'External dispatch does not bind exact C01 inputs');
  }
  const startMs = Date.parse(descriptor.calibrationStartAt);
  const endMs = Date.parse(descriptor.calibrationEndAt);
  if (!Number.isFinite(startMs) || !Number.isFinite(endMs) ||
      endMs - startMs !== descriptor.durationMs || endMs - startMs < 20 * 60_000) {
    fail('external_dispatch_window_mismatch', 'External dispatch window is invalid');
  }
  const clients = input.externalClients;
  if (!Array.isArray(clients) || clients.length !== 2 ||
      new Set(clients.map((item) => item.clientId)).size !== 2 ||
      new Set(clients.map((item) => item.viewerId)).size !== 2 ||
      clients.map((item) => item.paceProfile).sort().join(',') !== 'human_controller,slow_concurrency') {
    fail('external_dispatch_identity_mismatch', 'Two distinct frozen external client identities are required');
  }
  const agentIds = new Set(input.agentClientIds);
  if (clients.some((client) => agentIds.has(client.clientId) || agentIds.has(client.viewerId))) {
    fail('external_dispatch_identity_mismatch', 'External client identities must be isolated from agent clients');
  }
  return { startMs, endMs };
}

function verifyPrepared(prepared) {
  if (!prepared || prepared.state !== 'prepared' || prepared.preparedSha256 !== preparedDigest(prepared)) {
    fail('prepared_descriptor_mismatch', 'Prepared distributed calibration descriptor is missing or changed');
  }
}

export function prepareDistributedC01Calibration(input) {
  validateTargets(input.developmentTargets);
  validateAgentIds(input.agentClientIds);
  validateBinding(input);
  if (typeof input.calibrationId !== 'string' || input.calibrationId.length === 0) {
    fail('invalid_calibration_id', 'calibrationId is required');
  }
  const descriptor = clone(input.externalDispatchDescriptor);
  const window = validateDispatch(descriptor, input);
  const preparedAt = input.clock?.wallNow?.();
  if (!Number.isFinite(Date.parse(preparedAt)) || Date.parse(preparedAt) > window.startMs) {
    fail('late_preparation', 'Distributed calibration must be prepared before the shared start');
  }
  const prepared = {
    state: 'prepared', calibrationId: input.calibrationId, runId: input.runId,
    sourceCommit: input.sourceCommit, workflowRunId: input.workflowRunId,
    workflowRunAttempt: input.workflowRunAttempt,
    developmentTargets: clone(input.developmentTargets), agentClientIds: clone(input.agentClientIds),
    externalClients: clone(input.externalClients), externalDispatchDescriptor: descriptor, preparedAt,
  };
  prepared.preparedSha256 = preparedDigest(prepared);
  return prepared;
}

function normalizeFailure(error) {
  return {
    code: typeof error?.code === 'string' ? error.code : 'service_command_failed',
    name: typeof error?.name === 'string' ? error.name : 'Error',
    message: typeof error?.message === 'string' ? error.message : String(error),
  };
}

export async function startDistributedC01Calibration(input) {
  verifyPrepared(input.prepared);
  if (typeof input.serviceTransport?.executeReadOnlyCommand !== 'function' ||
      typeof input.scheduler?.waitUntil !== 'function' || typeof input.clock?.wallNow !== 'function' ||
      typeof input.clock?.monotonicNow !== 'function' || typeof input.artifactStore?.writeOnce !== 'function') {
    fail('invalid_start_dependencies', 'Service transport, scheduler, clock, and artifact store are required');
  }
  const prepared = clone(input.prepared);
  const descriptor = prepared.externalDispatchDescriptor;
  const startMs = Date.parse(descriptor.calibrationStartAt);
  const endMs = Date.parse(descriptor.calibrationEndAt);
  const observedBeforeStart = Date.parse(input.clock.wallNow());
  if (!Number.isFinite(observedBeforeStart) || observedBeforeStart > startMs) {
    fail('late_local_start', 'Local Service calibration cannot start after the shared window begins');
  }
  await input.scheduler.waitUntil({ wallTime: descriptor.calibrationStartAt, action: null });
  const startedAt = input.clock.wallNow();
  const startedMonotonicTimeNanoseconds = input.clock.monotonicNow();
  const observations = [];
  let activeSafetyStop = null;
  for (let ordinal = 1; ordinal <= SERVICE_COMMAND_COUNT; ordinal += 1) {
    const target = prepared.developmentTargets[(ordinal - 1) % 2];
    const plannedAt = new Date(startMs + Math.floor(((ordinal - 1) * (endMs - startMs)) / 500)).toISOString();
    const plan = {
      ordinal, actionOrdinal: ordinal, kind: 'service_command', plannedAt, target: clone(target),
      clientId: prepared.agentClientIds[(ordinal - 1) % 25],
    };
    if (!activeSafetyStop && input.safetyStop) {
      activeSafetyStop = clone(await input.safetyStop({
        completedActionCount: observations.length, nextAction: clone(plan), observations: clone(observations),
      })) ?? null;
    }
    if (activeSafetyStop) {
      observations.push({ ...plan, attempt: 0, state: 'safety_stopped', safetyStop: clone(activeSafetyStop) });
      continue;
    }
    await input.scheduler.waitUntil({ wallTime: plannedAt, action: clone(plan) });
    const request = {
      ...clone(plan), attempt: 1, action: SERVICE_ACTIONS[(ordinal - 1) % SERVICE_ACTIONS.length],
      effectClass: 'read_only',
    };
    try {
      const response = await input.serviceTransport.executeReadOnlyCommand(request);
      if (!response || !['read_only', 'harmless'].includes(response.effectClass) ||
          response.attempt !== 1 || response.retryAttempted !== false || response.repairAttempted !== false ||
          !['passed', 'failed'].includes(response.state) || !Number.isFinite(response.latencyMs) ||
          !Number.isFinite(Date.parse(response.observedAt)) || Date.parse(response.observedAt) < Date.parse(plannedAt) ||
          Date.parse(response.observedAt) > endMs) {
        const error = new Error('Service response did not prove one harmless attempt');
        error.code = 'service_effect_contract_mismatch';
        throw error;
      }
      if (response.state === 'failed') {
        const error = new Error(response.failure?.message ?? 'Read-only Service command failed');
        error.code = response.failure?.code ?? 'service_command_failed';
        throw error;
      }
      observations.push({ ...plan, attempt: 1, state: 'passed', observedAt: response.observedAt, result: clone(response) });
    } catch (error) {
      observations.push({ ...plan, attempt: 1, state: 'failed', observedAt: input.clock.wallNow(), failure: normalizeFailure(error) });
    }
  }
  await input.scheduler.waitUntil({ wallTime: descriptor.calibrationEndAt, action: null });
  const completedAt = input.clock.wallNow();
  const localBody = {
    schemaVersion: 'agent-browser.p158-local-calibration-observations.v1',
    preparedSha256: prepared.preparedSha256, startedAt, completedAt,
    startedMonotonicTimeNanoseconds,
    completedMonotonicTimeNanoseconds: input.clock.monotonicNow(), observations,
    safetyStop: activeSafetyStop, retryAttempted: false, repairAttempted: false,
  };
  const content = canonicalJson(localBody);
  const localObservationArtifact = {
    artifactId: `${prepared.calibrationId}-local-service-observations`, kind: 'calibration_service_raw',
    relativePath: `calibration/${prepared.calibrationId}/local-service-observations.json`, capturedAt: completedAt,
    mediaType: 'application/json', contentEncoding: 'utf8', content,
    declaredSha256: sha256(content), declaredByteCount: Buffer.byteLength(content),
  };
  const writeReceipt = await input.artifactStore.writeOnce(localObservationArtifact.relativePath, content);
  const localRun = {
    state: 'local_complete', preparedSha256: prepared.preparedSha256, localObservationArtifact, writeReceipt,
    windowValid: Date.parse(startedAt) >= startMs && Date.parse(completedAt) <= endMs,
  };
  localRun.localRunSha256 = localRunDigest(localRun);
  return localRun;
}

function validateReceipts(prepared, receipts) {
  if (!Array.isArray(receipts) || receipts.length !== 2) {
    fail('external_receipt_count_mismatch', 'Exactly two completed external receipts are required');
  }
  const descriptor = prepared.externalDispatchDescriptor;
  const clients = new Map(prepared.externalClients.map((client) => [client.clientId, client]));
  const runnerIds = new Set();
  const actions = new Map();
  for (const receipt of receipts) {
    if (receipt.schemaVersion !== 'agent-browser.p158-external-calibration-receipt.v1' ||
        receipt.receiptSha256 !== canonicalExternalRunnerReceiptDigest(receipt)) {
      fail('external_receipt_hash_mismatch', 'External receipt schema or self-hash does not agree');
    }
    const client = clients.get(receipt.clientId);
    if (!client || receipt.viewerId !== client.viewerId || receipt.paceProfile !== client.paceProfile ||
        receipt.runId !== prepared.runId || receipt.sourceCommit !== prepared.sourceCommit ||
        receipt.workflowRunId !== prepared.workflowRunId || receipt.workflowRunAttempt !== prepared.workflowRunAttempt ||
        receipt.calibration?.dispatchDescriptor?.descriptorSha256 !== descriptor.descriptorSha256 ||
        receipt.handoff?.urlSha256 !== prepared.developmentTargets.find((item) => item.environmentId === 'E2').handoffUrlSha256 ||
        receipt.startedAt !== descriptor.calibrationStartAt || receipt.completedAt !== descriptor.calibrationEndAt ||
        receipt.runnerIdentity?.provider !== 'github_actions' || receipt.outsideServiceHost !== true ||
        receipt.outsideServiceNetworkNamespace !== true || receipt.publicEgressObserved !== true ||
        receipt.success !== true || receipt.retryCount !== 0 || receipt.repairAttempted !== false) {
      fail('external_receipt_binding_mismatch', 'External receipt does not match the prepared schedule and identity');
    }
    runnerIds.add(receipt.runnerIdentity.runnerId);
    for (const action of receipt.actions ?? []) {
      const maximum = EXTERNAL_ACTION_COUNTS[action.kind];
      const key = `${action.kind}:${action.ordinal}`;
      const observedMs = Date.parse(action.observedAt);
      if (!maximum || !Number.isInteger(action.ordinal) || action.ordinal < 1 || action.ordinal > maximum ||
          actions.has(key) || action.viewerId !== receipt.viewerId || action.attempt !== 1 ||
          action.retryAttempted !== false || action.repairAttempted !== false ||
          !['passed', 'failed'].includes(action.state) || !Number.isFinite(observedMs) ||
          observedMs < Date.parse(descriptor.calibrationStartAt) || observedMs > Date.parse(descriptor.calibrationEndAt) ||
          !Number.isFinite(action.latencyMs) || action.latencyMs < 0 ||
          (action.state === 'failed' && !action.failure?.code)) {
        fail('external_action_contract_mismatch', 'External action violates terminal timing or attempt contract', action);
      }
      actions.set(key, { receipt, action });
    }
  }
  if (runnerIds.size !== 2) fail('external_receipt_identity_mismatch', 'External runner identities are not distinct');
  for (const [kind, count] of Object.entries(EXTERNAL_ACTION_COUNTS)) {
    if (Array.from({ length: count }, (_, index) => `${kind}:${index + 1}`).some((key) => !actions.has(key))) {
      fail('external_action_count_mismatch', `External receipts lack exact ${kind} ordinals`);
    }
  }
  return actions;
}

function externalObservation({ receipt, action }, globalOrdinal, target) {
  const common = {
    ordinal: globalOrdinal, actionOrdinal: action.ordinal, kind: action.kind, plannedAt: action.observedAt,
    target: clone(target), clientId: action.viewerId, externalViewerReceiptId: receipt.receiptId,
    attempt: 1, observedAt: action.observedAt,
  };
  const evidence = {
    state: action.state, outcome: action.state, latencyMs: action.latencyMs, observedAt: action.observedAt,
    source: 'external_runner_receipt', receiptId: receipt.receiptId,
    runnerId: receipt.runnerIdentity.runnerId, performedLocally: false, attempt: 1,
    retryAttempted: false, repairAttempted: false,
  };
  return action.state === 'passed'
    ? { ...common, state: 'passed', result: evidence }
    : { ...common, state: 'failed', failure: clone(action.failure), externalEvidence: evidence };
}

function safeReceiptProjection(receipt) {
  return {
    schemaVersion: receipt.schemaVersion,
    receiptId: receipt.receiptId,
    receiptSha256: receipt.receiptSha256,
    runId: receipt.runId,
    clientId: receipt.clientId,
    viewerId: receipt.viewerId,
    paceProfile: receipt.paceProfile,
    sourceCommit: receipt.sourceCommit,
    workflowRunId: receipt.workflowRunId,
    workflowRunAttempt: receipt.workflowRunAttempt,
    runnerIdentity: clone(receipt.runnerIdentity),
    outsideServiceHost: receipt.outsideServiceHost,
    outsideServiceNetworkNamespace: receipt.outsideServiceNetworkNamespace,
    publicEgressObserved: receipt.publicEgressObserved,
    handoff: { urlSha256: receipt.handoff.urlSha256 },
    startedAt: receipt.startedAt,
    completedAt: receipt.completedAt,
    calibration: { dispatchDescriptor: clone(receipt.calibration.dispatchDescriptor) },
    actions: clone(receipt.actions),
    retryCount: receipt.retryCount,
    repairAttempted: receipt.repairAttempted,
  };
}

export function finalizeDistributedC01Calibration(input) {
  verifyPrepared(input.prepared);
  const prepared = clone(input.prepared);
  const finalizedAt = input.clock?.wallNow?.();
  const endMs = Date.parse(prepared.externalDispatchDescriptor.calibrationEndAt);
  if (!Number.isFinite(Date.parse(finalizedAt)) || Date.parse(finalizedAt) < endMs) {
    fail('early_finalization', 'Finalization must wait for completed external jobs; late receipts are valid');
  }
  const localRun = clone(input.localRun);
  if (!localRun || localRun.state !== 'local_complete' || localRun.preparedSha256 !== prepared.preparedSha256 ||
      localRun.localRunSha256 !== localRunDigest(localRun) ||
      localRun.localObservationArtifact.declaredSha256 !== sha256(localRun.localObservationArtifact.content)) {
    fail('local_observation_integrity_mismatch', 'Persisted local observations are missing or changed');
  }
  if (localRun.windowValid !== true) {
    fail('local_window_mismatch', 'Local Service calibration did not remain inside the prepared window');
  }
  const localBody = JSON.parse(localRun.localObservationArtifact.content);
  if (localBody.observations.length !== 500 ||
      localBody.observations.some((entry, index) => entry.ordinal !== index + 1)) {
    fail('local_observation_count_mismatch', 'Local run must retain all 500 Service ordinals');
  }
  const receipts = clone(input.externalRunnerReceipts);
  const actions = validateReceipts(prepared, receipts);
  const e2Target = prepared.developmentTargets.find((target) => target.environmentId === 'E2');
  const external = [];
  for (const [kind, count, offset] of [['dashboard_action', 50, 500], ['handoff_reconnect', 10, 550]]) {
    for (let ordinal = 1; ordinal <= count; ordinal += 1) {
      external.push(externalObservation(actions.get(`${kind}:${ordinal}`), offset + ordinal, e2Target));
    }
  }
  const result = finalizeC01CalibrationEvidence({
    calibrationId: prepared.calibrationId, developmentTargets: prepared.developmentTargets,
    agentClientIds: prepared.agentClientIds,
    externalViewerReceipts: receipts.map(safeReceiptProjection),
    startedAt: prepared.externalDispatchDescriptor.calibrationStartAt,
    completedAt: prepared.externalDispatchDescriptor.calibrationEndAt,
    startedMonotonicTimeNanoseconds: localBody.startedMonotonicTimeNanoseconds,
    completedMonotonicTimeNanoseconds: localBody.completedMonotonicTimeNanoseconds,
    observations: [...localBody.observations, ...external], safetyStop: localBody.safetyStop,
  });
  if (JSON.stringify(result).includes('/remote-view/')) {
    fail('raw_handoff_forbidden', 'Final calibration artifacts contain a raw durable handoff');
  }
  return {
    ...result, localObservationArtifact: localRun.localObservationArtifact,
    distributedEvidence: {
      preparedSha256: prepared.preparedSha256, localRunSha256: localRun.localRunSha256,
      dispatchDescriptorSha256: prepared.externalDispatchDescriptor.descriptorSha256,
      externalReceiptIds: receipts.map((receipt) => receipt.receiptId).sort(),
      externalRunnerIds: receipts.map((receipt) => receipt.runnerIdentity.runnerId).sort(),
      sharedWindowStartedAt: prepared.externalDispatchDescriptor.calibrationStartAt,
      sharedWindowCompletedAt: prepared.externalDispatchDescriptor.calibrationEndAt,
      sharedWindowDurationMs: endMs - Date.parse(prepared.externalDispatchDescriptor.calibrationStartAt),
      serviceCommandCount: localBody.observations.filter((entry) => entry.attempt === 1).length,
      dashboardActionCount: 50, handoffReconnectCount: 10, externalReplayEffectCount: 0,
      finalizedAt, retryAttempted: false, repairAttempted: false,
    },
  };
}
