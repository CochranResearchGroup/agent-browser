#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { sha256 } from './lib/p158-campaign-controller.js';

import {
  buildP158W9EnduranceDispatch,
  buildP158W9EnduranceDispatchTemplate,
  bindP158W9EnduranceDispatchTemplate,
  finalizeP158W9Endurance,
  P158W9EnduranceError,
  runP158W9EnduranceShard,
  projectP158W9EnduranceActionReceipts,
  validateP158W9EnduranceDispatch,
} from './lib/p158-w9-endurance.js';

const producer = {
  workflowPath: '.github/workflows/p158-w9-endurance.yml', workflowSha256: '11'.repeat(32),
  segmentWorkflowPath: '.github/workflows/p158-w9-endurance-segment.yml', segmentWorkflowSha256: 'aa'.repeat(32),
  runnerPath: 'scripts/run-p158-w9-endurance.js', runnerSha256: '22'.repeat(32),
  libraryPath: 'scripts/lib/p158-w9-endurance.js', librarySha256: '33'.repeat(32),
  preparationWorkflowPath: '.github/workflows/p158-w9-endurance-preparation.yml', preparationWorkflowSha256: '34'.repeat(32),
  preparationRunnerPath: 'scripts/run-p158-w9-endurance-preparation.js', preparationRunnerSha256: '35'.repeat(32),
  preparationLibraryPath: 'scripts/lib/p158-w9-endurance-preparation.js', preparationLibrarySha256: '36'.repeat(32),
};

const producerFiles = {
  workflowPath: '.github/workflows/p158-w9-endurance.yml',
  segmentWorkflowPath: '.github/workflows/p158-w9-endurance-segment.yml',
  runnerPath: 'scripts/run-p158-w9-endurance.js',
  libraryPath: 'scripts/lib/p158-w9-endurance.js',
  preparationWorkflowPath: '.github/workflows/p158-w9-endurance-preparation.yml',
  preparationRunnerPath: 'scripts/run-p158-w9-endurance-preparation.js',
  preparationLibraryPath: 'scripts/lib/p158-w9-endurance-preparation.js',
};

async function actualProducer() {
  const result = {};
  for (const [field, path] of Object.entries(producerFiles)) {
    result[field] = path;
    result[field.replace('Path', 'Sha256')] = createHash('sha256').update(await readFile(path)).digest('hex');
  }
  return result;
}

function actions(caseId) {
  const counts = caseId === 'C04'
    ? { dashboard_action: 2000, handoff_reconnect: 200 }
    : { handoff_reconnect: 500 };
  return Object.entries(counts).flatMap(([kind, count]) => Array.from({ length: count }, (_, index) => ({
    actionId: `${caseId}:${kind}:${String(index + 1).padStart(6, '0')}`,
    attemptId: `${caseId}-E2-r${String((index % (caseId === 'C04' ? 200 : 500)) + 1).padStart(3, '0')}`,
    caseId, kind, environmentId: 'E2', transport: 'external_ingress',
    ...(kind === 'dashboard_action' ? { postcondition: {
      kind: 'pixel_region_transition', region: { x: 10, y: 10, width: 20, height: 20 },
      beforeSha256: 'ab'.repeat(32), afterSha256: 'cd'.repeat(32),
    } } : {}),
  })));
}

const eventPostconditions = {
  viewer_expiry: { kind: 'authoritative_lease_expiry', leaseIdSha256: '10'.repeat(32), viewerRole: 'viewer',
    fromState: 'active', toState: 'expired', baselineGeneration: 1, timeoutMs: 60_000 },
  controller_expiry: { kind: 'authoritative_lease_expiry', leaseIdSha256: '20'.repeat(32), viewerRole: 'controller',
    fromState: 'active', toState: 'expired', baselineGeneration: 1, timeoutMs: 60_000 },
  client_restart: { kind: 'retained_identity_reopen', retainedIdentitySha256: '77'.repeat(32) },
  scheduled_network_profile: { kind: 'offline_failure_then_unchanged_handoff_recovery' },
};

function preparation(caseId, preparedActions, events, bindings = {}) {
  const artifactCount = preparedActions.filter((action) => action.kind === 'dashboard_action').length * 2 +
    (caseId === 'C05' ? 2 : 0);
  const body = {
    schemaVersion: 'agent-browser.p158-w9-endurance-postcondition-preparation.v1', planId: 'P158', caseId,
    runId: bindings.runId ?? 'p158-endurance-test', sourceCommit: bindings.sourceCommit ?? 'a'.repeat(40),
    candidateSha256: '44'.repeat(32), scheduleSha256: '55'.repeat(32),
    handoffUrlSha256: '66'.repeat(32), retainedIdentitySha256: '77'.repeat(32),
    syntheticFixtureAttestationSha256: '41'.repeat(32), externalRunnerIdentitySha256: '43'.repeat(32),
    workflowRunId: '111111', workflowRunAttempt: 1, workflowJob: 'prepare-postconditions',
    preparedAt: '2026-09-03T00:00:00.000Z', externalIngress: true, providerFree: false, syntheticOnly: true,
    dashboardActionCount: preparedActions.filter((action) => action.kind === 'dashboard_action').length,
    actionPostconditionsSha256: sha256(
      preparedActions.filter((action) => action.kind === 'dashboard_action')
        .map((action) => ({ actionId: action.actionId, postcondition: action.postcondition }))),
    leaseBaselines: [], eventPostconditionsSha256: sha256(events),
    artifactReceipts: Array.from({ length: artifactCount }, (_, index) => ({
      artifactId: `preparation-${caseId}-${index}`, relativePath: `preparation-${index}.png`,
      sha256: 'ef'.repeat(32), byteCount: 1,
    })),
    retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
  };
  return { ...body, postconditionPreparationSha256: sha256(body) };
}

function dispatch(caseId) {
  const input = {
    caseId, runId: 'p158-endurance-test', sourceCommit: 'a'.repeat(40),
    workflowRunId: '123456789', workflowRunAttempt: 1,
    candidateSha256: '44'.repeat(32), scheduleSha256: '55'.repeat(32),
    handoffUrlSha256: '66'.repeat(32), retainedIdentitySha256: '77'.repeat(32),
    externalVantageAggregateSha256: '88'.repeat(32), externalHandoffOracleSha256: '99'.repeat(32),
    startAt: caseId === 'C04' ? '2026-09-04T00:00:00.000Z' : '2026-09-05T00:00:00.000Z',
    actions: actions(caseId), eventPostconditions: caseId === 'C05' ? eventPostconditions : {},
    producer, receiptRoot: `/tmp/p158-endurance-test/${caseId}`,
  };
  input.postconditionPreparation = preparation(caseId, input.actions, input.eventPostconditions);
  const template = buildP158W9EnduranceDispatchTemplate(input);
  return bindP158W9EnduranceDispatchTemplate({
    template, sourceCommit: 'a'.repeat(40), workflowRunId: '123456789', workflowRunAttempt: 1,
  });
}

function harness(startAt) {
  let now = Date.parse(startAt) - 60_000;
  const progress = [];
  const scheduler = { waitUntil: async (value) => { now = Math.max(now, Date.parse(value)); } };
  const evidenceArtifact = (id) => ({ artifactId: id, relativePath: `${id}.json`, sha256: 'ef'.repeat(32), byteCount: 1 });
  const terminal = (identity) => ({
    ...identity, state: 'passed', attempt: 1, observedAt: new Date(now).toISOString(),
    retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
    artifacts: [evidenceArtifact(identity.actionId ?? identity.eventId)],
  });
  return {
    scheduler,
    progress,
    recordProgress: async (entry) => { progress.push(structuredClone(entry)); },
    clock: { now: () => now, wallNow: () => new Date(now).toISOString() },
    driver: {
      observeAction: async (action) => ({ ...terminal({
        actionId: action.actionId, caseId: action.caseId, attemptId: action.attemptId, kind: action.kind,
      }), ...(action.kind === 'dashboard_action' ? {
        postconditionSatisfied: true,
        postconditionSha256: sha256(action.postcondition),
        artifacts: [evidenceArtifact(`${action.actionId}-before`), evidenceArtifact(`${action.actionId}-after`)],
      } : {}) }),
      executeScheduledEvent: async (event) => ({ ...terminal({ eventId: event.eventId, kind: event.kind }),
        observationSha256: '12'.repeat(32), observation: { synthetic: true } }),
      observeContinuity: async ({ dispatch: frozen, segment, boundary }) => ({
        state: 'passed', segmentIndex: segment.segmentIndex, boundary,
        operatorVisibleState: 'ready', handoffUrlSha256: frozen.handoffUrlSha256,
        retainedIdentitySha256: frozen.retainedIdentitySha256, artifacts: [],
      }),
    },
  };
}

async function runCase(caseId) {
  const frozen = dispatch(caseId);
  const before = structuredClone(frozen);
  assert.equal(validateP158W9EnduranceDispatch(frozen), frozen);
  const runtime = harness(frozen.startAt);
  const receipts = [];
  for (let segmentIndex = 1; segmentIndex <= frozen.segmentCount; segmentIndex += 1) {
    receipts.push(await runP158W9EnduranceShard({
      dispatch: frozen, segmentIndex, predecessorReceipt: receipts.at(-1) ?? null, ...runtime,
    }));
  }
  assert.deepEqual(frozen, before, 'endurance execution mutated the frozen dispatch');
  const final = finalizeP158W9Endurance({ dispatch: frozen, shardReceipts: receipts });
  const workflowPlanBody = {
    schemaVersion: 'agent-browser.p158-w9-external-workflow-plan.v1', runId: frozen.runId,
    candidateSha256: frozen.candidateSha256, scheduleSha256: frozen.scheduleSha256,
    enduranceDispatches: { [caseId]: frozen },
  };
  const workflowPlan = { ...workflowPlanBody, planSha256: sha256(workflowPlanBody) };
  const projected = projectP158W9EnduranceActionReceipts({
    dispatch: frozen, finalReceipt: final, workflowPlan,
  });
  assert.equal(final.success, true);
  assert.equal(final.segmentReceiptSha256s.length, frozen.segmentCount);
  assert.equal(final.dashboardActionCount, caseId === 'C04' ? 2000 : 0);
  assert.equal(final.reconnectCount, caseId === 'C04' ? 200 : 500);
  assert.equal(new Set(projected.map((entry) => entry.actionId)).size, final.actionCount);
  assert(projected.every((entry) => entry.attempt === 1 &&
    entry.retryAttempted === false && entry.repairAttempted === false));
  assert.equal(runtime.progress.length, frozen.scheduledActions.length + frozen.scheduledEvents.length +
    frozen.segmentCount * 2);
  return { frozen, receipts, final, projected, workflowPlan };
}

const c04 = await runCase('C04');
assert.deepEqual(c04.frozen.segments.map((segment) => segment.actionIds.length), [1100, 1100]);
assert.equal(c04.final.durationMs, 8 * 60 * 60 * 1000);
process.stdout.write('PASS seals and finalizes two exact four-hour C04 shards\n');

const wrongPlan = structuredClone(c04.workflowPlan);
wrongPlan.candidateSha256 = 'cd'.repeat(32);
assert.throws(
  () => projectP158W9EnduranceActionReceipts({
    dispatch: c04.frozen, finalReceipt: c04.final, workflowPlan: wrongPlan,
  }),
  (error) => error instanceof P158W9EnduranceError && error.code === 'endurance_workflow_plan_mismatch',
);
process.stdout.write('PASS rejects a mismatched workflow plan during effect-free projection\n');

const c05 = await runCase('C05');
assert.deepEqual(c05.frozen.segments.map((segment) => segment.actionIds.length), [84, 83, 83, 84, 83, 83]);
assert.equal(c05.final.durationMs, 24 * 60 * 60 * 1000);
assert.deepEqual([...new Set(c05.final.eventReceipts.map((entry) => entry.kind))].sort(), [
  'client_restart', 'controller_expiry', 'scheduled_network_profile', 'viewer_expiry',
]);
process.stdout.write('PASS seals and finalizes six exact four-hour C05 shards with scheduled endurance events\n');

const changed = structuredClone(c04.receipts);
changed[1].predecessorReceiptSha256 = 'ff'.repeat(32);
changed[1].receiptSha256 = 'ee'.repeat(32);
assert.throws(
  () => finalizeP158W9Endurance({ dispatch: c04.frozen, shardReceipts: changed }),
  (error) => error instanceof P158W9EnduranceError && error.code === 'endurance_shard_receipt_invalid',
);
process.stdout.write('PASS rejects changed or disconnected shard evidence without repair\n');

assert.throws(
  () => buildP158W9EnduranceDispatch({
    ...structuredClone(c05.frozen), actions: actions('C05'), handoffUrlSha256: 'https://localhost/remote-view/raw',
  }),
  (error) => error instanceof P158W9EnduranceError,
);
process.stdout.write('PASS rejects raw handoff custody and invalid frozen bindings\n');

assert.throws(
  () => buildP158W9EnduranceDispatch({
    ...structuredClone(c04.frozen),
    actions: actions('C04').map(({ postcondition: _postcondition, ...action }) => action),
  }),
  (error) => error instanceof P158W9EnduranceError &&
    error.code === 'endurance_dashboard_postcondition_unbound',
);
assert.throws(
  () => buildP158W9EnduranceDispatch({
    ...structuredClone(c05.frozen), actions: actions('C05'), eventPostconditions: {},
  }),
  (error) => error instanceof P158W9EnduranceError &&
    error.code === 'endurance_event_postcondition_unbound',
);
process.stdout.write('PASS refuses descriptor-only dashboard and expiry execution\n');

const workflow = await readFile('.github/workflows/p158-w9-endurance.yml', 'utf8');
const segmentWorkflow = await readFile('.github/workflows/p158-w9-endurance-segment.yml', 'utf8');
assert.match(workflow, /schedule-passive-observation:/);
assert.match(workflow, /performs no elapsed-time wait/);
assert.match(segmentWorkflow, /validate-passive-segment:/);
assert.match(segmentWorkflow, /No browser action, reconnect, repair, installation block, or synchronous wait/);
assert(!workflow.includes('c04-segment-1:'));
assert(!workflow.includes('c05-segment-1:'));
assert(!segmentWorkflow.includes('timeout-minutes: 250'));
assert(!workflow.includes('continue-on-error'));
assert(!segmentWorkflow.includes('continue-on-error'));
const runnerSource = await readFile('scripts/run-p158-w9-endurance.js', 'utf8');
assert(!/^import .*playwright/m.test(runnerSource));
assert.match(runnerSource, /import\('playwright'\)/);
const sourceBoundTemplate = buildP158W9EnduranceDispatchTemplate({
  caseId: 'C05', runId: 'p158-endurance-source-bound', sourceCommit: 'b'.repeat(40), candidateSha256: '44'.repeat(32),
  scheduleSha256: '55'.repeat(32), handoffUrlSha256: '66'.repeat(32),
  retainedIdentitySha256: '77'.repeat(32), externalVantageAggregateSha256: '88'.repeat(32),
  externalHandoffOracleSha256: '99'.repeat(32), startAt: '2026-09-05T00:00:00.000Z',
  actions: actions('C05'), producer: await actualProducer(), receiptRoot: '/tmp/p158-endurance-source-bound/C05',
  eventPostconditions,
  postconditionPreparation: preparation('C05', actions('C05'), eventPostconditions, {
    runId: 'p158-endurance-source-bound', sourceCommit: 'b'.repeat(40),
  }),
});
const sourceBound = bindP158W9EnduranceDispatchTemplate({
  template: sourceBoundTemplate, sourceCommit: 'b'.repeat(40), workflowRunId: '987654321', workflowRunAttempt: 2,
});
assert.equal(sourceBound.workflowRunId, '987654321');
assert.equal(sourceBound.workflowRunAttempt, 2);
process.stdout.write('PASS retires active endurance workflows in favor of passive nonblocking observation\n');
