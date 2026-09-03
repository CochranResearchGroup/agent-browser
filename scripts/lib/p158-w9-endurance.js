import { sha256 } from './p158-campaign-controller.js';

export const P158_W9_ENDURANCE_SCHEMA = 'agent-browser.p158-w9-endurance-dispatch.v1';
export const P158_W9_ENDURANCE_TEMPLATE_SCHEMA = 'agent-browser.p158-w9-endurance-dispatch-template.v1';
export const P158_W9_ENDURANCE_SHARD_SCHEMA = 'agent-browser.p158-w9-endurance-shard-receipt.v1';
export const P158_W9_ENDURANCE_FINAL_SCHEMA = 'agent-browser.p158-w9-endurance-final-receipt.v1';

export const P158_W9_ENDURANCE_CASES = Object.freeze({
  C04: Object.freeze({
    durationMs: 8 * 60 * 60 * 1000,
    segmentCount: 2,
    dashboardActionCount: 2000,
    reconnectCount: 200,
    scheduledEventKinds: Object.freeze([]),
  }),
  C05: Object.freeze({
    durationMs: 24 * 60 * 60 * 1000,
    segmentCount: 6,
    dashboardActionCount: 0,
    reconnectCount: 500,
    scheduledEventKinds: Object.freeze([
      'viewer_expiry', 'controller_expiry', 'client_restart', 'scheduled_network_profile',
    ]),
  }),
});

const SHA256 = /^[a-f0-9]{64}$/u;
const COMMIT = /^[a-f0-9]{40}$/u;
const PRODUCER_PATHS = Object.freeze({
  workflowPath: '.github/workflows/p158-w9-endurance.yml',
  segmentWorkflowPath: '.github/workflows/p158-w9-endurance-segment.yml',
  runnerPath: 'scripts/run-p158-w9-endurance.js',
  libraryPath: 'scripts/lib/p158-w9-endurance.js',
});

export class P158W9EnduranceError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W9EnduranceError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W9EnduranceError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function bodyWithoutDigest(value, digestField) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([field]) => field !== digestField));
}

function requireDigest(value, field) {
  if (!SHA256.test(value ?? '')) fail('endurance_binding_invalid', `${field} must be a lowercase SHA-256 digest`);
}

function parsedTime(value, field) {
  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed)) fail('endurance_time_invalid', `${field} must be RFC3339 time`);
  return parsed;
}

function assertNoRawUrls(value, path = 'value', seen = new Set()) {
  if (typeof value === 'string') {
    if (/^(?:https?|wss?|file):\/\//iu.test(value) || /(?:localhost|127\.0\.0\.1|\[::1\]|\/remote-view\/)/iu.test(value)) {
      fail('endurance_raw_url_prohibited', `${path} contains URL material`);
    }
    return;
  }
  if (!value || typeof value !== 'object' || seen.has(value)) return;
  seen.add(value);
  for (const [field, entry] of Object.entries(value)) assertNoRawUrls(entry, `${path}.${field}`, seen);
  seen.delete(value);
}

function normalizedActions(caseId, actions) {
  if (!Array.isArray(actions)) fail('endurance_action_set_invalid', `${caseId} actions must be an array`);
  const relevant = actions.filter((action) =>
    action?.caseId === caseId && ['dashboard_action', 'handoff_reconnect'].includes(action.kind));
  if (relevant.length !== actions.length || new Set(relevant.map((action) => action.actionId)).size !== relevant.length) {
    fail('endurance_action_set_invalid', `${caseId} has foreign or duplicate actions`);
  }
  for (const action of relevant) {
    if (typeof action.actionId !== 'string' || typeof action.attemptId !== 'string' ||
        action.environmentId !== 'E2' || action.transport !== 'external_ingress') {
      fail('endurance_action_set_invalid', `${caseId} action identity is incomplete`, { actionId: action?.actionId });
    }
    if (action.kind === 'dashboard_action') {
      const proof = action.postcondition;
      const region = proof?.region;
      if (proof?.kind !== 'pixel_region_transition' || !SHA256.test(proof.beforeSha256 ?? '') ||
          !SHA256.test(proof.afterSha256 ?? '') || proof.beforeSha256 === proof.afterSha256 ||
          !region || !['x', 'y', 'width', 'height'].every((field) => Number.isInteger(region[field]) && region[field] >= 0) ||
          region.width < 1 || region.height < 1) {
        fail('endurance_dashboard_postcondition_unbound', `${action.actionId} lacks an exact frozen visual transition`);
      }
    }
  }
  return relevant.map(clone).sort((left, right) => left.actionId.localeCompare(right.actionId));
}

function distributeKind(actions, kind, segmentCount, startMs, segmentDurationMs) {
  const matching = actions.filter((action) => action.kind === kind);
  return matching.map((action, index) => {
    const globalOrdinal = index + 1;
    const segmentIndex = Math.min(segmentCount, Math.floor(index * segmentCount / matching.length) + 1);
    const segmentRows = matching.filter((_, rowIndex) =>
      Math.min(segmentCount, Math.floor(rowIndex * segmentCount / matching.length) + 1) === segmentIndex);
    const firstIndex = matching.indexOf(segmentRows[0]);
    const ordinalInSegment = index - firstIndex + 1;
    const segmentStart = startMs + (segmentIndex - 1) * segmentDurationMs;
    const offsetMs = Math.floor(ordinalInSegment * segmentDurationMs / (segmentRows.length + 1));
    return {
      ...action,
      globalOrdinal,
      segmentIndex,
      ordinalInSegment,
      plannedAt: new Date(segmentStart + offsetMs).toISOString(),
    };
  });
}

function scheduledEvents(caseId, contract, startMs, segmentDurationMs, eventPostconditions) {
  if (caseId !== 'C05') return [];
  const rows = [];
  for (let segmentIndex = 1; segmentIndex <= contract.segmentCount; segmentIndex += 1) {
    for (let index = 0; index < contract.scheduledEventKinds.length; index += 1) {
      const kind = contract.scheduledEventKinds[index];
      const postcondition = eventPostconditions?.[kind];
      const valid = kind === 'scheduled_network_profile'
        ? postcondition?.kind === 'offline_failure_then_unchanged_handoff_recovery'
        : kind === 'client_restart'
          ? postcondition?.kind === 'retained_identity_reopen' && SHA256.test(postcondition.retainedIdentitySha256 ?? '')
          : postcondition?.kind === 'authoritative_lease_expiry' && SHA256.test(postcondition.leaseIdSha256 ?? '') &&
            ['viewer', 'controller'].includes(postcondition.viewerRole) && postcondition.fromState === 'active' &&
            postcondition.toState === 'expired' && Number.isInteger(postcondition.timeoutMs) && postcondition.timeoutMs > 0;
      if (!valid) fail('endurance_event_postcondition_unbound', `${kind} lacks a frozen observable postcondition`);
      rows.push({
        eventId: `${caseId}:endurance:${kind}:${String(segmentIndex).padStart(2, '0')}`,
        kind,
        globalOrdinal: (segmentIndex - 1) * contract.scheduledEventKinds.length + index + 1,
        segmentIndex,
        ordinalInSegment: index + 1,
        plannedAt: new Date(startMs + (segmentIndex - 1) * segmentDurationMs +
          Math.floor((index + 1) * segmentDurationMs / (contract.scheduledEventKinds.length + 1))).toISOString(),
        postcondition: clone(postcondition),
      });
    }
  }
  return rows;
}

function verifyProducer(producer) {
  for (const [field, expected] of Object.entries(PRODUCER_PATHS)) {
    if (producer?.[field] !== expected) fail('endurance_producer_unsealed', `${field} is not the reviewed producer`);
    requireDigest(producer?.[field.replace('Path', 'Sha256')], field.replace('Path', 'Sha256'));
  }
}

export function buildP158W9EnduranceDispatch({
  caseId, runId, sourceCommit, workflowRunId, workflowRunAttempt, candidateSha256,
  scheduleSha256, handoffUrlSha256, retainedIdentitySha256,
  externalVantageAggregateSha256, externalHandoffOracleSha256, postconditionPreparationSha256,
  startAt, actions, eventPostconditions = {}, producer, receiptRoot,
}) {
  const contract = P158_W9_ENDURANCE_CASES[caseId];
  if (!contract) fail('endurance_case_invalid', 'Only C04 and C05 have endurance dispatches');
  if (typeof runId !== 'string' || !runId || !COMMIT.test(sourceCommit ?? '') ||
      !/^\d+$/u.test(workflowRunId ?? '') || !Number.isInteger(workflowRunAttempt) || workflowRunAttempt < 1 ||
      typeof receiptRoot !== 'string' || !receiptRoot.startsWith('/') || receiptRoot.includes('..')) {
    fail('endurance_binding_invalid', `${caseId} run, workflow, commit, or receipt-root binding is invalid`);
  }
  for (const [field, value] of Object.entries({ candidateSha256, scheduleSha256, handoffUrlSha256,
    retainedIdentitySha256, externalVantageAggregateSha256, externalHandoffOracleSha256,
    postconditionPreparationSha256 })) requireDigest(value, field);
  verifyProducer(producer);
  const startMs = parsedTime(startAt, 'startAt');
  const normalized = normalizedActions(caseId, actions);
  const dashboard = normalized.filter((action) => action.kind === 'dashboard_action');
  const reconnects = normalized.filter((action) => action.kind === 'handoff_reconnect');
  if (dashboard.length !== contract.dashboardActionCount || reconnects.length !== contract.reconnectCount) {
    fail('endurance_action_count_mismatch', `${caseId} external action cardinality differs from Plan 0158`, {
      dashboardActionCount: dashboard.length,
      reconnectCount: reconnects.length,
    });
  }
  const segmentDurationMs = contract.durationMs / contract.segmentCount;
  const scheduledActions = [
    ...distributeKind(normalized, 'dashboard_action', contract.segmentCount, startMs, segmentDurationMs),
    ...distributeKind(normalized, 'handoff_reconnect', contract.segmentCount, startMs, segmentDurationMs),
  ].sort((left, right) => Date.parse(left.plannedAt) - Date.parse(right.plannedAt) ||
    left.actionId.localeCompare(right.actionId));
  const events = scheduledEvents(caseId, contract, startMs, segmentDurationMs, eventPostconditions);
  const segments = Array.from({ length: contract.segmentCount }, (_, index) => {
    const segmentIndex = index + 1;
    return {
      segmentIndex,
      startAt: new Date(startMs + index * segmentDurationMs).toISOString(),
      endAt: new Date(startMs + segmentIndex * segmentDurationMs).toISOString(),
      actionIds: scheduledActions.filter((action) => action.segmentIndex === segmentIndex).map((action) => action.actionId),
      eventIds: events.filter((event) => event.segmentIndex === segmentIndex).map((event) => event.eventId),
    };
  });
  const body = {
    schemaVersion: P158_W9_ENDURANCE_SCHEMA,
    planId: 'P158', caseId, runId, sourceCommit, workflowRunId, workflowRunAttempt,
    candidateSha256, scheduleSha256, handoffUrlSha256, retainedIdentitySha256,
    externalVantageAggregateSha256, externalHandoffOracleSha256, postconditionPreparationSha256,
    startAt: new Date(startMs).toISOString(),
    endAt: new Date(startMs + contract.durationMs).toISOString(),
    durationMs: contract.durationMs, segmentCount: contract.segmentCount, segmentDurationMs,
    dashboardActionCount: dashboard.length, reconnectCount: reconnects.length,
    scheduledActions, scheduledEvents: events, eventPostconditions: clone(eventPostconditions), segments,
    producer: clone(producer), receiptRoot,
    repairAllowed: false, retryAllowed: false, garbageCollectionAllowed: false,
  };
  assertNoRawUrls(body);
  return Object.freeze({ ...body, dispatchSha256: sha256(body) });
}

export function buildP158W9EnduranceDispatchTemplate(input) {
  const body = {
    schemaVersion: P158_W9_ENDURANCE_TEMPLATE_SCHEMA,
    planId: 'P158',
    ...clone(input),
  };
  for (const field of ['sourceCommit', 'workflowRunId', 'workflowRunAttempt', 'dispatchSha256', 'templateSha256']) {
    delete body[field];
  }
  buildP158W9EnduranceDispatch({
    ...clone(body), sourceCommit: '0'.repeat(40), workflowRunId: '0', workflowRunAttempt: 1,
  });
  assertNoRawUrls(body);
  return Object.freeze({ ...body, templateSha256: sha256(body) });
}

export function bindP158W9EnduranceDispatchTemplate({
  template, sourceCommit, workflowRunId, workflowRunAttempt,
}) {
  if (template?.schemaVersion !== P158_W9_ENDURANCE_TEMPLATE_SCHEMA ||
      template.templateSha256 !== sha256(bodyWithoutDigest(template, 'templateSha256'))) {
    fail('endurance_template_integrity_mismatch', 'Endurance dispatch template is missing or changed');
  }
  const input = bodyWithoutDigest(template, 'templateSha256');
  delete input.schemaVersion;
  delete input.planId;
  return buildP158W9EnduranceDispatch({
    ...input, sourceCommit, workflowRunId, workflowRunAttempt,
  });
}

export function validateP158W9EnduranceDispatch(dispatch) {
  const digest = dispatch?.dispatchSha256;
  if (dispatch?.schemaVersion !== P158_W9_ENDURANCE_SCHEMA || digest !== sha256(bodyWithoutDigest(dispatch, 'dispatchSha256'))) {
    fail('endurance_dispatch_integrity_mismatch', 'Endurance dispatch is missing or changed');
  }
  const rebuilt = buildP158W9EnduranceDispatch({
    ...clone(dispatch), actions: dispatch.scheduledActions.map(({ globalOrdinal: _globalOrdinal,
      segmentIndex: _segmentIndex, ordinalInSegment: _ordinalInSegment, plannedAt: _plannedAt, ...action }) => action),
  });
  if (rebuilt.dispatchSha256 !== digest) fail('endurance_dispatch_projection_mismatch', 'Endurance schedule is not canonical');
  return dispatch;
}

function verifyArtifactReceipts(artifacts) {
  if (!Array.isArray(artifacts)) fail('endurance_artifact_invalid', 'Artifact receipts must be an array');
  for (const artifact of artifacts) {
    if (typeof artifact?.artifactId !== 'string' || typeof artifact.relativePath !== 'string' ||
        !SHA256.test(artifact.sha256 ?? '') || !Number.isInteger(artifact.byteCount) || artifact.byteCount < 0) {
      fail('endurance_artifact_invalid', 'Artifact receipt is incomplete');
    }
  }
}

export async function runP158W9EnduranceShard({
  dispatch, segmentIndex, predecessorReceipt = null, driver, scheduler,
  clock = { now: () => Date.now(), wallNow: () => new Date().toISOString() },
  lateToleranceMs = 60_000, recordProgress = async () => {},
}) {
  validateP158W9EnduranceDispatch(dispatch);
  const segment = dispatch.segments.find((entry) => entry.segmentIndex === segmentIndex);
  if (!segment || !driver || typeof driver.observeAction !== 'function' ||
      typeof driver.observeContinuity !== 'function' || typeof scheduler?.waitUntil !== 'function') {
    fail('endurance_shard_invalid', 'Shard index, driver, or scheduler is invalid');
  }
  if (segmentIndex === 1 ? predecessorReceipt !== null : predecessorReceipt === null) {
    fail('endurance_predecessor_invalid', 'Shard predecessor presence is invalid');
  }
  if (predecessorReceipt) {
    validateP158W9EnduranceShardReceipt(predecessorReceipt, dispatch);
    if (predecessorReceipt.segmentIndex !== segmentIndex - 1 || predecessorReceipt.success !== true) {
      fail('endurance_predecessor_invalid', 'Shard predecessor is not the immediately successful segment');
    }
  }
  await scheduler.waitUntil(segment.startAt);
  const startedAt = clock.wallNow();
  const queueDelayMs = Math.max(0, parsedTime(startedAt, 'startedAt') - parsedTime(segment.startAt, 'segment.startAt'));
  if (queueDelayMs > lateToleranceMs) fail('endurance_shard_late', `Segment ${segmentIndex} started after its frozen tolerance`);
  const continuity = [];
  const actionReceipts = [];
  const eventReceipts = [];
  const artifacts = [];
  continuity.push(await driver.observeContinuity({ dispatch: clone(dispatch), segment: clone(segment), boundary: 'start' }));
  await recordProgress({ type: 'continuity', value: clone(continuity[0]) });
  const entries = [
    ...dispatch.scheduledActions.filter((action) => action.segmentIndex === segmentIndex).map((value) => ({ type: 'action', value })),
    ...dispatch.scheduledEvents.filter((event) => event.segmentIndex === segmentIndex).map((value) => ({ type: 'event', value })),
  ].sort((left, right) => Date.parse(left.value.plannedAt) - Date.parse(right.value.plannedAt) ||
    (left.value.actionId ?? left.value.eventId).localeCompare(right.value.actionId ?? right.value.eventId));
  for (const entry of entries) {
    await scheduler.waitUntil(entry.value.plannedAt);
    const receipt = entry.type === 'action'
      ? await driver.observeAction(clone(entry.value))
      : await driver.executeScheduledEvent?.(clone(entry.value));
    if (!receipt || receipt.state !== 'passed' || receipt.attempt !== 1 || receipt.retryAttempted !== false ||
        receipt.repairAttempted !== false || receipt.garbageCollectionAttempted !== false) {
      fail('endurance_observation_failed', `${entry.value.actionId ?? entry.value.eventId} did not produce one clean terminal receipt`);
    }
    if (entry.type === 'action' && (receipt.actionId !== entry.value.actionId ||
        receipt.caseId !== entry.value.caseId || receipt.attemptId !== entry.value.attemptId ||
        receipt.kind !== entry.value.kind)) {
      fail('endurance_observation_failed', `${entry.value.actionId} receipt identity differs from the dispatch`);
    }
    if (entry.type === 'event' && (receipt.eventId !== entry.value.eventId || receipt.kind !== entry.value.kind)) {
      fail('endurance_observation_failed', `${entry.value.eventId} receipt identity differs from the dispatch`);
    }
    verifyArtifactReceipts(receipt.artifacts ?? []);
    if (entry.type === 'action' && entry.value.kind === 'dashboard_action' &&
        (receipt.postconditionSatisfied !== true || receipt.postconditionSha256 !== sha256(entry.value.postcondition) ||
          receipt.artifacts.length < 2)) {
      fail('endurance_observation_failed', `${entry.value.actionId} lacks its exact visual postcondition evidence`);
    }
    if (entry.type === 'action' && entry.value.kind === 'handoff_reconnect' && receipt.artifacts.length < 1) {
      fail('endurance_observation_failed', `${entry.value.actionId} lacks reconnect pixel evidence`);
    }
    if (entry.type === 'event' && (!SHA256.test(receipt.observationSha256 ?? '') || receipt.artifacts.length < 1)) {
      fail('endurance_observation_failed', `${entry.value.eventId} lacks observed transition evidence`);
    }
    artifacts.push(...clone(receipt.artifacts ?? []));
    (entry.type === 'action' ? actionReceipts : eventReceipts).push(receipt);
    await recordProgress({ type: entry.type, value: clone(receipt) });
  }
  await scheduler.waitUntil(segment.endAt);
  continuity.push(await driver.observeContinuity({ dispatch: clone(dispatch), segment: clone(segment), boundary: 'end' }));
  await recordProgress({ type: 'continuity', value: clone(continuity.at(-1)) });
  for (const observation of continuity) {
    if (observation?.state !== 'passed' || observation.handoffUrlSha256 !== dispatch.handoffUrlSha256 ||
        observation.retainedIdentitySha256 !== dispatch.retainedIdentitySha256 || observation.operatorVisibleState !== 'ready') {
      fail('endurance_continuity_failed', `Segment ${segmentIndex} continuity differs from the frozen handoff identity`);
    }
    verifyArtifactReceipts(observation.artifacts ?? []);
    artifacts.push(...clone(observation.artifacts ?? []));
  }
  const body = {
    schemaVersion: P158_W9_ENDURANCE_SHARD_SCHEMA, planId: 'P158', runId: dispatch.runId,
    caseId: dispatch.caseId, dispatchSha256: dispatch.dispatchSha256, segmentIndex,
    segmentCount: dispatch.segmentCount, predecessorReceiptSha256: predecessorReceipt?.receiptSha256 ?? null,
    workflowRunId: dispatch.workflowRunId, workflowRunAttempt: dispatch.workflowRunAttempt,
    sourceCommit: dispatch.sourceCommit, producer: clone(dispatch.producer),
    candidateSha256: dispatch.candidateSha256, scheduleSha256: dispatch.scheduleSha256,
    handoffUrlSha256: dispatch.handoffUrlSha256, retainedIdentitySha256: dispatch.retainedIdentitySha256,
    plannedStartAt: segment.startAt, plannedEndAt: segment.endAt, startedAt,
    completedAt: clock.wallNow(), queueDelayMs, actionReceipts, eventReceipts, continuity,
    artifacts, success: true, retryAttempted: false, repairAttempted: false,
    garbageCollectionAttempted: false,
  };
  assertNoRawUrls(body);
  return Object.freeze({ ...body, receiptSha256: sha256(body) });
}

export function validateP158W9EnduranceShardReceipt(receipt, dispatch) {
  if (receipt?.schemaVersion !== P158_W9_ENDURANCE_SHARD_SCHEMA ||
      receipt.receiptSha256 !== sha256(bodyWithoutDigest(receipt, 'receiptSha256')) ||
      receipt.dispatchSha256 !== dispatch.dispatchSha256 || receipt.runId !== dispatch.runId ||
      receipt.caseId !== dispatch.caseId || receipt.retryAttempted !== false ||
      receipt.workflowRunId !== dispatch.workflowRunId || receipt.workflowRunAttempt !== dispatch.workflowRunAttempt ||
      receipt.sourceCommit !== dispatch.sourceCommit || sha256(receipt.producer) !== sha256(dispatch.producer) ||
      receipt.candidateSha256 !== dispatch.candidateSha256 || receipt.scheduleSha256 !== dispatch.scheduleSha256 ||
      receipt.handoffUrlSha256 !== dispatch.handoffUrlSha256 || receipt.retainedIdentitySha256 !== dispatch.retainedIdentitySha256 ||
      receipt.repairAttempted !== false || receipt.garbageCollectionAttempted !== false) {
    fail('endurance_shard_receipt_invalid', 'Endurance shard receipt is missing, changed, or unbound');
  }
  assertNoRawUrls(receipt);
  return receipt;
}

export function finalizeP158W9Endurance({ dispatch, shardReceipts }) {
  validateP158W9EnduranceDispatch(dispatch);
  if (!Array.isArray(shardReceipts) || shardReceipts.length !== dispatch.segmentCount) {
    fail('endurance_segment_set_incomplete', 'Every frozen endurance segment is required');
  }
  const ordered = shardReceipts.map((receipt) => validateP158W9EnduranceShardReceipt(receipt, dispatch))
    .sort((left, right) => left.segmentIndex - right.segmentIndex);
  for (let index = 0; index < ordered.length; index += 1) {
    const receipt = ordered[index];
    if (receipt.segmentIndex !== index + 1 || receipt.success !== true ||
        receipt.predecessorReceiptSha256 !== (index === 0 ? null : ordered[index - 1].receiptSha256) ||
        receipt.plannedStartAt !== dispatch.segments[index].startAt ||
        receipt.plannedEndAt !== dispatch.segments[index].endAt) {
      fail('endurance_segment_chain_invalid', 'Endurance segment chain or frozen window changed');
    }
  }
  const observedActions = ordered.flatMap((receipt) => receipt.actionReceipts);
  const expectedIds = dispatch.scheduledActions.map((action) => action.actionId).sort();
  const observedIds = observedActions.map((receipt) => receipt.actionId).sort();
  if (sha256(expectedIds) !== sha256(observedIds) || new Set(observedIds).size !== observedIds.length) {
    fail('endurance_action_receipts_incomplete', 'Endurance action receipts do not exactly cover the dispatch');
  }
  const eventReceipts = ordered.flatMap((receipt) => receipt.eventReceipts);
  if (sha256(dispatch.scheduledEvents.map((event) => event.eventId).sort()) !==
      sha256(eventReceipts.map((receipt) => receipt.eventId).sort())) {
    fail('endurance_event_receipts_incomplete', 'Scheduled endurance events are incomplete');
  }
  const body = {
    schemaVersion: P158_W9_ENDURANCE_FINAL_SCHEMA, planId: 'P158', runId: dispatch.runId,
    caseId: dispatch.caseId, dispatchSha256: dispatch.dispatchSha256,
    workflowRunId: dispatch.workflowRunId, workflowRunAttempt: dispatch.workflowRunAttempt,
    sourceCommit: dispatch.sourceCommit, producer: clone(dispatch.producer),
    candidateSha256: dispatch.candidateSha256, scheduleSha256: dispatch.scheduleSha256,
    handoffUrlSha256: dispatch.handoffUrlSha256, retainedIdentitySha256: dispatch.retainedIdentitySha256,
    startedAt: ordered[0].startedAt, completedAt: ordered.at(-1).completedAt,
    durationMs: parsedTime(dispatch.endAt, 'endAt') - parsedTime(dispatch.startAt, 'startAt'),
    segmentReceiptSha256s: ordered.map((receipt) => receipt.receiptSha256),
    actionCount: observedActions.length, reconnectCount: observedActions.filter((entry) => entry.kind === 'handoff_reconnect').length,
    dashboardActionCount: observedActions.filter((entry) => entry.kind === 'dashboard_action').length,
    eventCount: eventReceipts.length, actionObservations: observedActions, eventReceipts,
    artifactReceipts: ordered.flatMap((receipt) => receipt.artifacts),
    success: true, retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
  };
  assertNoRawUrls(body);
  return Object.freeze({ ...body, finalReceiptSha256: sha256(body) });
}

export function projectP158W9EnduranceActionReceipts({ dispatch, finalReceipt, workflowPlan }) {
  validateP158W9EnduranceDispatch(dispatch);
  const workflowPlanSha256 = workflowPlan?.planSha256;
  if (workflowPlan?.schemaVersion !== 'agent-browser.p158-w9-external-workflow-plan.v1' ||
      workflowPlanSha256 !== sha256(bodyWithoutDigest(workflowPlan, 'planSha256')) ||
      workflowPlan.runId !== dispatch.runId || workflowPlan.candidateSha256 !== dispatch.candidateSha256 ||
      workflowPlan.scheduleSha256 !== dispatch.scheduleSha256 ||
      workflowPlan.enduranceDispatches?.[dispatch.caseId]?.dispatchSha256 !== dispatch.dispatchSha256) {
    fail('endurance_workflow_plan_mismatch', 'Projection requires the exact self-hashed workflow plan containing this dispatch');
  }
  if (finalReceipt?.schemaVersion !== P158_W9_ENDURANCE_FINAL_SCHEMA ||
      finalReceipt.finalReceiptSha256 !== sha256(bodyWithoutDigest(finalReceipt, 'finalReceiptSha256')) ||
      finalReceipt.dispatchSha256 !== dispatch.dispatchSha256 || finalReceipt.success !== true) {
    fail('endurance_final_receipt_invalid', 'Final endurance receipt is missing, changed, or unbound');
  }
  const observed = new Map(finalReceipt.actionObservations.map((entry) => [entry.actionId, entry]));
  if (observed.size !== dispatch.scheduledActions.length) {
    fail('endurance_final_receipt_invalid', 'Final endurance receipt does not cover every action');
  }
  return dispatch.scheduledActions.map((action) => {
    const observation = observed.get(action.actionId);
    if (!observation) fail('endurance_final_receipt_invalid', `${action.actionId} observation is absent`);
    const body = {
      actionId: action.actionId, caseId: action.caseId, attemptId: action.attemptId,
      kind: action.kind, environmentId: 'E2', state: 'passed', attempt: 1,
      observedAt: observation.observedAt, retryAttempted: false, repairAttempted: false,
      garbageCollectionAttempted: false, offHost: true, outsideServiceNetworkNamespace: true,
      operatorVisibleState: 'ready', readyBeforePixels: true, pixelsObserved: true,
      handoffUrlSha256: dispatch.handoffUrlSha256,
      retainedIdentitySha256: dispatch.retainedIdentitySha256,
      externalVantageAggregateSha256: dispatch.externalVantageAggregateSha256,
      externalHandoffOracleSha256: dispatch.externalHandoffOracleSha256,
      workflowPlanSha256,
    };
    return { ...body, receiptSha256: sha256(body) };
  });
}

export function validateP158W9EndurancePlanBinding({ externalWorkflowPlan, caseId, actions, target, caseWindow }) {
  try {
    const dispatch = externalWorkflowPlan?.enduranceDispatches?.[caseId];
    validateP158W9EnduranceDispatch(dispatch);
    const externalActions = actions.filter((action) =>
      ['dashboard_action', 'handoff_reconnect'].includes(action.kind));
    if (externalActions.some((action) => action.caseId !== caseId || action.environmentId !== 'E2' ||
        action.transport !== 'external_ingress' || typeof action.actionId !== 'string') ||
        new Set(externalActions.map((action) => action.actionId)).size !== externalActions.length) {
      fail('endurance_action_set_invalid', `${caseId} live actions are foreign or duplicated`);
    }
    const exactIds = externalActions.map((action) => action.actionId).sort();
    const serviceCommandCount = actions.filter((action) => action.kind === 'service_command').length;
    const browserCrashCount = actions.filter((action) => action.kind === 'declared_browser_crash').length;
    if (dispatch.runId !== target.runId || dispatch.candidateSha256 !== target.candidateSha256 ||
        dispatch.scheduleSha256 !== externalWorkflowPlan.scheduleSha256 ||
        dispatch.workflowRunId !== target.workflowRunId || dispatch.workflowRunAttempt !== target.workflowRunAttempt ||
        dispatch.handoffUrlSha256 !== target.handoffUrlSha256 ||
        dispatch.retainedIdentitySha256 !== target.retainedIdentitySha256 ||
        dispatch.startAt !== caseWindow?.startAt || dispatch.endAt !== caseWindow?.endAt ||
        (caseId === 'C04' && (serviceCommandCount !== 10000 || browserCrashCount !== 50)) ||
        sha256(dispatch.scheduledActions.map((action) => action.actionId).sort()) !== sha256(exactIds) ||
        externalWorkflowPlan.enduranceProducer?.workflowSourceSha256 !== dispatch.producer.workflowSha256 ||
        externalWorkflowPlan.enduranceProducer?.segmentWorkflowSourceSha256 !== dispatch.producer.segmentWorkflowSha256 ||
        externalWorkflowPlan.enduranceProducer?.runnerSourceSha256 !== dispatch.producer.runnerSha256 ||
        externalWorkflowPlan.enduranceProducer?.librarySourceSha256 !== dispatch.producer.librarySha256 ||
        externalWorkflowPlan.enduranceProducer?.postconditionPreparationSha256 !== dispatch.postconditionPreparationSha256) {
      fail('endurance_plan_binding_invalid', `${caseId} endurance producer or dispatch is not frozen into the workflow plan`);
    }
    return { valid: true, dispatchSha256: dispatch.dispatchSha256 };
  } catch (error) {
    if (error instanceof P158W9EnduranceError) return { valid: false, code: error.code };
    throw error;
  }
}
