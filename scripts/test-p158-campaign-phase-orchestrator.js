#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { createMemoryArtifactStore, sha256 } from './lib/p158-campaign-controller.js';
import {
  buildP158CampaignPhasePreparation,
  applyP158PhasePreparationToControllerSchedule,
  runP158CampaignPhases,
} from './lib/p158-campaign-phase-orchestrator.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';

const registry = JSON.parse(await readFile(
  new URL('../docs/dev/contracts/p158-historical-failure-registry.v1.json', import.meta.url), 'utf8',
));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-integrated-phases' });
const LIVE_MANIFEST = '91'.repeat(32);

function bundle(phaseId, { blockedCaseId, concreteCaseIds = null, interruptAttemptId = null, calls = [] } = {}) {
  const contracts = schedule.caseContracts.filter((entry) => entry.phaseId === phaseId);
  const bindings = contracts.map((contract) => {
    const blocked = concreteCaseIds ? !concreteCaseIds.includes(contract.caseId) : contract.caseId === blockedCaseId;
    return {
      caseId: contract.caseId,
      adapterId: contract.adapterId,
      executionContractSha256: contract.executionContractSha256,
      mode: blocked ? 'explicit_blocked' : 'concrete_live',
      providerFree: false,
      sourcePath: `scripts/live-hooks/${phaseId.toLowerCase()}.js`,
      sourceSha256: (phaseId === 'W7' ? '71' : '81').repeat(32),
      hookIds: blocked ? [] : [`${phaseId.toLowerCase()}.test`],
      implementedActionCount: blocked ? 0 : 1,
      blockedActionCount: blocked ? 1 : 0,
      effectsAllowed: !blocked,
      blocker: blocked ? { code: 'test_explicit_blocker', detail: `${contract.caseId} intentionally blocked` } : null,
    };
  });
  const effects = {};
  const adapters = contracts.map((contract, index) => {
    const binding = bindings[index];
    const effectId = contract.declaredEffectIds[0];
    effects[effectId] = async ({ attemptId }) => {
      calls.push({ phaseId, caseId: contract.caseId, attemptId, effectId });
      if (attemptId === interruptAttemptId) throw Object.assign(new Error('simulated process loss'), { code: 'simulated_loss' });
      return { resultState: 'passed', artifactIds: [`artifact:${attemptId}`] };
    };
    return {
      caseId: contract.caseId,
      executionMode: binding.mode,
      providerFree: false,
      effectsAllowed: binding.effectsAllowed,
      sourcePath: binding.sourcePath,
      sourceSha256: binding.sourceSha256,
      liveHookManifestSha256: LIVE_MANIFEST,
      liveBindingSha256: sha256(binding),
      liveHookIds: [...binding.hookIds],
      blocker: binding.blocker,
      execute: async ({ attempt, requestEffect }) => {
        if (binding.mode === 'explicit_blocked') {
          await requestEffect(effectId, { attemptId: attempt.attemptId });
          assert.fail('an explicit_blocked adapter must never be invoked');
        }
        await requestEffect(effectId, { attemptId: attempt.attemptId });
        return { resultState: 'passed', effectState: 'verified_effect', retryDisposition: 'prohibited',
          repairAttempted: false, retryAttempted: false };
      },
    };
  });
  return {
    [phaseId === 'W7' ? 'w7Adapters' : 'w8Adapters']: adapters,
    adapterBindings: bindings,
    effects,
  };
}

function preparationInputs(w7, w8) {
  const w9AdapterBindings = schedule.caseContracts.filter((entry) => entry.phaseId === 'W9').map((contract) => ({
    caseId: contract.caseId, adapterId: contract.adapterId,
    executionContractSha256: contract.executionContractSha256, mode: 'explicit_blocked', providerFree: false,
    sourcePath: 'scripts/live-hooks/w9.js', sourceSha256: '92'.repeat(32), hookIds: [],
    implementedActionCount: 0, blockedActionCount: 1, effectsAllowed: false,
    blocker: { code: 'live_case_hook_missing', detail: 'fixture W9 blocked' },
  }));
  const loggingRequestExpectations = schedule.attempts.flatMap((attempt) => {
    const blocked = [...w7.adapterBindings, ...w8.adapterBindings, ...w9AdapterBindings]
      .find((entry) => entry.caseId === attempt.caseId).mode === 'explicit_blocked';
    if (blocked) return [];
    const actionIds = attempt.cardinalityAllocations.flatMap((entry) => entry.actionIds);
    return (actionIds.length > 0 ? actionIds : [null]).map((actionId, index) => {
      const expectationId = `${attempt.attemptId}:${actionId ?? `request-${index + 1}`}`;
      const environmentId = attempt.environmentId ?? attempt.environmentIds[0];
      const operationCorrelationId = `p158:p158-integrated-run:${attempt.attemptId}:${environmentId}:${expectationId}:request`;
      return { expectationId, operationCorrelationId,
        productRequestId: null, productRequestIdState: 'assigned_at_runtime',
        requestKind: 'accepted_request',
        actionId, attemptId: attempt.attemptId, caseId: attempt.caseId, phaseId: attempt.phaseId,
        environmentId };
    });
  });
  return {
    w9AdapterBindings,
    loggingRequestExpectations,
  };
}

function controllerHarness(preparedSchedule = schedule.attempts.map((attempt) => ({
  attemptId: attempt.attemptId, preExecutionBlocker: null,
}))) {
  let state = 'frozen';
  const results = [];
  const artifacts = [];
  let scheduledTeardown = {};
  return {
    snapshot: () => ({ state, runId: 'p158-integrated-run', results: structuredClone(results),
      evidence: { artifacts: structuredClone(artifacts) },
      scheduledTeardown: structuredClone(scheduledTeardown) }),
    startExecution: async () => { assert.equal(state, 'frozen'); state = 'executing'; },
    recordAttempt: async (result) => {
      assert.equal(results.some((entry) => entry.attemptId === result.attemptId), false);
      const phaseId = schedule.attempts.find((entry) => entry.attemptId === result.attemptId)?.phaseId;
      if (['W7', 'W8'].includes(phaseId)) {
        assert.ok(['not_started', 'no_effect', 'effect_uncertain', 'verified_effect'].includes(result.effectState));
        assert.ok(['not_applicable', 'prohibited_opportunistic_retry', 'predetermined_distinct_attempt']
          .includes(result.retryDisposition));
        assert.deepEqual(Object.keys(result.causalIds).sort(), ['eventId', 'traceId']);
        assert.ok(result.evidence.artifactIds.every((artifactId) => artifacts.some((entry) => entry.artifactId === artifactId)));
        const frozen = preparedSchedule.find((entry) => entry.attemptId === result.attemptId)?.preExecutionBlocker;
        if (result.resultState === 'skipped_blocked') {
          assert.deepEqual(result.blocker, frozen);
          assert.deepEqual(result.requestedEffects, []);
          assert.equal(result.effectState, 'not_started');
        }
      }
      results.push(structuredClone(result));
    },
    writeArtifact: async ({ artifactId, relativePath, content }) => {
      assert.equal(artifacts.some((entry) => entry.artifactId === artifactId), false);
      artifacts.push({ artifactId, relativePath, sha256: sha256(content) });
    },
    recordScheduledTeardown: async (result) => { scheduledTeardown = structuredClone(result); },
    finishExecution: async () => {
      const missing = schedule.attempts.filter((attempt) => !results.some((entry) => entry.attemptId === attempt.attemptId));
      assert.equal(missing.length, 0, 'finish is forbidden before every phase attempt is terminal');
      assert.ok(scheduledTeardown.resultState);
      state = 'execution_terminal';
    },
    sealEvidence: async () => { assert.equal(state, 'execution_terminal'); state = 'evidence_sealed'; },
  };
}

function w9Harness(controller, observations) {
  return async ({ schedule: inputSchedule, controller: inputController }) => {
    assert.equal(inputSchedule.scheduleSha256, schedule.scheduleSha256);
    assert.equal(inputController, controller);
    const pre = schedule.attempts.filter((attempt) => ['W7', 'W8'].includes(attempt.phaseId));
    assert.ok(pre.every((attempt) => controller.snapshot().results.some((entry) => entry.attemptId === attempt.attemptId)));
    observations.push(controller.snapshot().results.length);
    for (const attempt of schedule.attempts.filter((entry) => entry.phaseId === 'W9')) {
      if (!controller.snapshot().results.some((entry) => entry.attemptId === attempt.attemptId)) {
        await controller.recordAttempt({ attemptId: attempt.attemptId, caseId: attempt.caseId, resultState: 'passed' });
      }
    }
    if (!controller.snapshot().scheduledTeardown.resultState) {
      await controller.recordScheduledTeardown({ resultState: 'passed' });
    }
    await controller.finishExecution();
    await controller.sealEvidence();
    return { state: 'evidence_sealed' };
  };
}

async function withRoot(body) {
  const root = await mkdtemp(join(tmpdir(), 'p158-phases-'));
  try { return await body(root); } finally { await rm(root, { recursive: true, force: true }); }
}

await withRoot(async (runRoot) => {
  const calls = [];
  const w7 = bundle('W7', { concreteCaseIds: ['A02'], calls });
  const w8 = bundle('W8', { concreteCaseIds: ['D02'], calls });
  const prepared = preparationInputs(w7, w8);
  const store = createMemoryArtifactStore();
  const w9Starts = [];
  const phasePreparation = buildP158CampaignPhasePreparation({
    schedule, w7Bundle: w7, w8Bundle: w8, liveHookManifestSha256: LIVE_MANIFEST,
    runId: 'p158-integrated-run', ...prepared,
  });
  const controllerSchedule = applyP158PhasePreparationToControllerSchedule({
    controllerSchedule: schedule.attempts.map((attempt) => ({
      attemptId: attempt.attemptId, preExecutionBlocker: null,
    })),
    phasePreparation,
  });
  const controller = controllerHarness(controllerSchedule);
  const result = await runP158CampaignPhases({
    schedule, controller, w7Bundle: w7, w8Bundle: w8,
    w9: { target: { runId: 'p158-integrated-run' }, adapterBindings: prepared.w9AdapterBindings,
      loggingRequestExpectations: prepared.loggingRequestExpectations }, runRoot, artifactStore: store,
    liveHookManifestSha256: LIVE_MANIFEST, clock: { wallNow: () => '2026-09-03T12:00:00.000Z' },
    phasePreparation,
    runW9: w9Harness(controller, w9Starts),
  });
  const preAttempts = schedule.attempts.filter((entry) => ['W7', 'W8'].includes(entry.phaseId));
  assert.equal(result.state, 'evidence_sealed');
  assert.equal(result.terminalPreAttemptCount, preAttempts.length);
  assert.deepEqual(result.preExecutionBlockers.map(({ phaseId, caseId, attemptId }) => `${phaseId}:${caseId}:${attemptId}`).sort(),
    schedule.attempts.filter((attempt) => !['A02', 'D02'].includes(attempt.caseId))
      .map((attempt) => `${attempt.phaseId}:${attempt.caseId}:${attempt.attemptId}`).sort());
  assert.deepEqual(w9Starts, [preAttempts.length]);
  assert.equal(calls.length, preAttempts.filter((attempt) => ['A02', 'D02'].includes(attempt.caseId)).length);
  assert.equal(result.loggingExpectations.length, prepared.loggingRequestExpectations.length +
    schedule.attempts.filter((attempt) => !['A02', 'D02'].includes(attempt.caseId))
      .reduce((count, attempt) => count + attempt.environmentIds.length, 0));
  assert.ok(result.loggingExpectations.every((entry) => entry.operationCorrelationId.includes(entry.attemptId)));
  assert.ok(result.loggingExpectations.every((entry) => entry.operationCorrelationId.includes(`:${entry.environmentId}:`) &&
    entry.caseId && entry.phaseId && entry.executionMode));
  assert.ok(result.loggingExpectations.every((entry) => entry.causalIds.requestId === null &&
    entry.causalIds.jobId === null && entry.causalIds.eventId === null && entry.causalIds.traceId === null));
  assert.ok(result.loggingExpectations.every((entry) =>
    entry.operationCorrelationId.startsWith('p158:p158-integrated-run:')));
  const blockedIds = new Set(result.preExecutionBlockers.map((entry) => entry.attemptId));
  assert.ok(result.loggingExpectations.filter((entry) => blockedIds.has(entry.attemptId)).every((entry) =>
    entry.operatorVisible === false && entry.incidentExpected === false &&
    JSON.stringify(entry.expectedSurfaceRoles) === JSON.stringify([
      'controller_transition', 'pre_execution_blocker', 'terminal_event',
    ])));
  assert.ok(result.loggingExpectations.filter((entry) => !blockedIds.has(entry.attemptId)).every((entry) =>
    ['ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome']
      .every((role) => entry.expectedSurfaceRoles.includes(role))));
  assert.ok(controller.snapshot().results.find((entry) => entry.caseId === 'A01').evidence.artifactIds[0]
    .includes('p158-integrated-run'));
  assert.ok(store.paths().includes('campaign-phases/pre-execution-blockers.json'));
});

await withRoot(async (runRoot) => {
  const calls = [];
  const interrupted = schedule.attempts.find((entry) => entry.phaseId === 'W7' && entry.caseId !== 'A01').attemptId;
  const interruptedCase = schedule.attempts.find((entry) => entry.attemptId === interrupted).caseId;
  const w7 = bundle('W7', { concreteCaseIds: [interruptedCase], interruptAttemptId: interrupted, calls });
  const w8 = bundle('W8', { concreteCaseIds: [], calls });
  const prepared = preparationInputs(w7, w8);
  const store = createMemoryArtifactStore();
  const phasePreparation = buildP158CampaignPhasePreparation({
    schedule, w7Bundle: w7, w8Bundle: w8, liveHookManifestSha256: LIVE_MANIFEST,
    runId: 'p158-integrated-run', ...prepared,
  });
  const controller = controllerHarness(applyP158PhasePreparationToControllerSchedule({
    controllerSchedule: schedule.attempts.map((attempt) => ({
      attemptId: attempt.attemptId, preExecutionBlocker: null,
    })), phasePreparation,
  }));
  const args = {
    schedule, controller, w7Bundle: w7, w8Bundle: w8,
    w9: { target: { runId: 'p158-integrated-run' }, adapterBindings: prepared.w9AdapterBindings,
      loggingRequestExpectations: prepared.loggingRequestExpectations }, runRoot, artifactStore: store,
    liveHookManifestSha256: LIVE_MANIFEST, clock: { wallNow: () => '2026-09-03T12:00:00.000Z' },
    phasePreparation,
    runW9: w9Harness(controller, []),
  };
  await assert.rejects(() => runP158CampaignPhases(args), /simulated process loss/u);
  assert.equal(calls.filter((entry) => entry.attemptId === interrupted).length, 1);
  const resumed = bundle('W7', { concreteCaseIds: [interruptedCase], calls });
  const result = await runP158CampaignPhases({ ...args, w7Bundle: resumed });
  assert.equal(result.state, 'evidence_sealed');
  assert.equal(calls.filter((entry) => entry.attemptId === interrupted).length, 1, 'resume must not replay an uncertain effect');
  const terminal = controller.snapshot().results.find((entry) => entry.attemptId === interrupted);
  assert.equal(terminal.resultState, 'harness_failure');
  assert.equal(terminal.effectState, 'effect_uncertain');
});

await withRoot(async (runRoot) => {
  const controller = controllerHarness();
  const w7 = bundle('W7', { blockedCaseId: 'A01' });
  w7.w7Adapters[0].liveBindingSha256 = 'ff'.repeat(32);
  await assert.rejects(() => runP158CampaignPhases({
    schedule, controller, w7Bundle: w7, w8Bundle: bundle('W8', { blockedCaseId: 'D01' }),
    w9: { target: { runId: 'p158-integrated-run' } }, runRoot, artifactStore: createMemoryArtifactStore(),
    liveHookManifestSha256: LIVE_MANIFEST, runW9: async () => assert.fail('W9 must not start'),
  }), (error) => error.code === 'phase_adapter_binding_unproven');
  assert.equal(controller.snapshot().state, 'frozen');
});

{
  const w7 = bundle('W7', { concreteCaseIds: [] });
  const w8 = bundle('W8', { concreteCaseIds: [] });
  const prepared = preparationInputs(w7, w8);
  const blockedAttempt = schedule.attempts.find((attempt) => attempt.phaseId === 'W7');
  const extra = {
    expectationId: `p158:p158-integrated-run:${blockedAttempt.attemptId}:forged`,
    operationCorrelationId: `p158:p158-integrated-run:${blockedAttempt.attemptId}:forged`,
    productRequestId: null, productRequestIdState: 'assigned_at_runtime',
    requestKind: 'accepted_request', actionId: null, attemptId: blockedAttempt.attemptId,
    caseId: blockedAttempt.caseId, phaseId: blockedAttempt.phaseId,
    environmentId: blockedAttempt.environmentIds[0],
  };
  assert.throws(() => buildP158CampaignPhasePreparation({
    schedule, w7Bundle: w7, w8Bundle: w8, w9AdapterBindings: prepared.w9AdapterBindings,
    loggingRequestExpectations: [...prepared.loggingRequestExpectations, extra],
    liveHookManifestSha256: LIVE_MANIFEST, runId: 'p158-integrated-run',
  }), (error) => error.code === 'logging_request_expectations_incomplete');
  assert.throws(() => buildP158CampaignPhasePreparation({
    schedule, w7Bundle: w7, w8Bundle: w8, w9AdapterBindings: prepared.w9AdapterBindings,
    loggingRequestExpectations: prepared.loggingRequestExpectations,
    loggingOperationGaps: [{
      descriptorId: `${blockedAttempt.attemptId}:gap`, operationCorrelationId: `${blockedAttempt.attemptId}:gap`,
      productRequestId: null, correlationState: 'product_request_id_unavailable',
      operationKind: 'blocked-operation', actionId: null, attemptId: blockedAttempt.attemptId,
      caseId: blockedAttempt.caseId, phaseId: blockedAttempt.phaseId,
      environmentId: blockedAttempt.environmentIds[0],
      loggingGap: { code: 'product_request_id_not_preserved', detail: 'must not bind a blocked case' },
    }],
    liveHookManifestSha256: LIVE_MANIFEST, runId: 'p158-integrated-run',
  }), (error) => error.code === 'logging_request_expectations_incomplete');
}

process.stdout.write('P158 integrated W7/W8/W9 phase orchestration test passed\n');
