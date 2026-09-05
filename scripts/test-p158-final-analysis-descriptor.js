#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

import { canonicalJson, createFileArtifactStore, sha256 } from './lib/p158-campaign-controller.js';
import {
  createP158FinalAnalysisDescriptorHook,
  P158_FINAL_ANALYSIS_DESCRIPTOR_PATH,
} from './lib/p158-final-analysis-descriptor.js';
import { runP158FinalAnalysis } from './lib/p158-final-analysis-runner.js';

const registry = JSON.parse(await readFile(
  'docs/dev/contracts/p158-historical-failure-registry.v1.json', 'utf8'));
const loggingCorpus = JSON.parse(await readFile(
  'docs/dev/fixtures/p158/logging-causal-envelopes.v1.json', 'utf8'));
const dashboardCorpus = JSON.parse(await readFile(
  'docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json', 'utf8'));

function without(value, fields) {
  const excluded = new Set(fields);
  return Object.fromEntries(Object.entries(value).filter(([field]) => !excluded.has(field)));
}

async function createHarness() {
  const runRoot = await mkdtemp(path.join(os.tmpdir(), 'p158-w10-descriptor-'));
  const store = createFileArtifactStore(runRoot);
  const runId = `p158-descriptor-${path.basename(runRoot)}`;
  const attempt = {
    scheduleSequence: 0, scheduleId: 'W7:A01-E0-r001', caseId: 'A01',
    attemptId: 'A01-E0-r001', repetition: 1, seed: 101, environmentIds: ['E0'],
    dependsOnAttemptIds: [], preconditionIds: [], stimuli: [], evidenceProfile: 'profile',
    externalIngressRequired: false, preExecutionBlocker: null,
  };
  const scheduleBody = { schemaVersion: 'agent-browser.p158-execution-schedule.v1', planId: 'P158',
    runId, registrySha256: sha256(registry), attempts: [attempt], caseCount: 1, attemptCount: 1 };
  const schedule = { ...scheduleBody, scheduleSha256: sha256(scheduleBody) };
  const manifest = { schemaVersion: 'agent-browser.p158-campaign-manifest.v1', planId: 'P158', runId,
    registrySha256: sha256(registry), controllerState: 'prepared',
    candidate: { candidateSha256: '11'.repeat(32) }, artifactBindings: [], environmentSeals: [],
    calibration: null, fixtureSeal: null, schedule: [without(attempt, [])], freezePolicy: {},
    safetyPolicy: {}, evidencePolicy: { forbiddenCapturedFields: registry.forbiddenCapturedFields } };
  const writeAuthority = async (relativePath, value) => {
    const bytes = Buffer.from(canonicalJson(value));
    await store.writeOnce(relativePath, bytes);
    return { relativePath, sha256: sha256(bytes), byteCount: bytes.byteLength };
  };
  const manifestBinding = await writeAuthority('campaign-manifest.json', manifest);
  let preparedLedgerHeadSha256 = null;
  const freeze = { schemaVersion: 'agent-browser.p158-campaign-freeze.v1', planId: 'P158', runId,
    freezeId: `${runId}:freeze`, controllerState: 'frozen', manifestSha256: manifestBinding.sha256,
    candidateSha256: manifest.candidate.candidateSha256, artifactBindingsSha256: sha256([]),
    environmentSealsSha256: sha256([]), calibrationSha256: sha256(null), fixtureSealSha256: sha256(null),
    preparedLedgerHeadSha256: null, frozenAt: '2026-09-03T01:00:01.000Z',
    monotonicTimeNanoseconds: 2, startedCaseCount: 0, startedAttemptCount: 0 };
  const events = [];
  const artifacts = [];
  let parent = null;
  async function appendEvent(recordType, controllerState, payload, eventArtifacts = []) {
    const sequence = events.length;
    const value = { schemaVersion: 'agent-browser.p158-campaign-result.v1', planId: 'P158', runId,
      manifestSha256: manifestBinding.sha256,
      recordId: `${runId}:record:${String(sequence).padStart(8, '0')}`, sequence,
      previousRecordSha256: parent, recordType, controllerState,
      wallTime: new Date(Date.parse('2026-09-03T01:00:00.000Z') + sequence * 1000).toISOString(),
      monotonicTimeNanoseconds: sequence + 1, clockOffsetMilliseconds: 0,
      payload, artifacts: eventArtifacts };
    const bytes = Buffer.from(canonicalJson(value));
    await store.writeOnce(`ledger/${String(sequence).padStart(8, '0')}-${recordType}.json`, bytes);
    parent = sha256(bytes);
    events.push({ ...value, sha256: parent, byteCount: bytes.byteLength });
  }
  async function rawArtifact(artifactId, relativePath, value, mediaType = 'application/json', register = true) {
    const storagePath = relativePath.startsWith('artifacts/') ? relativePath : `artifacts/${relativePath}`;
    const bytes = Buffer.isBuffer(value) ? value : Buffer.from(canonicalJson(value));
    await store.writeOnce(storagePath, bytes);
    const receipt = { artifactId, relativePath: storagePath, mediaType, sha256: sha256(bytes),
      byteCount: bytes.byteLength, captureState: 'complete', captureGap: null, redactions: [],
      parentArtifactSha256s: artifacts.length ? [artifacts.at(-1).sha256] : [] };
    if (register) artifacts.push(receipt);
    return receipt;
  }
  await appendEvent('controller_transition', 'prepared', { kind: 'controller_transition',
    from: null, to: 'prepared', terminal: false });
  preparedLedgerHeadSha256 = parent;
  freeze.preparedLedgerHeadSha256 = preparedLedgerHeadSha256;
  await appendEvent('controller_transition', 'frozen', { kind: 'controller_transition',
    from: 'prepared', to: 'frozen', terminal: false });
  const logging = await rawArtifact(`${runId}:logging`, 'logging/w3.json', loggingCorpus,
    'application/json', false);
  const dashboard = await rawArtifact(`${runId}:dashboard`, 'dashboard/w5.json', dashboardCorpus.baseline,
    'application/json', false);
  await appendEvent('attempt_terminal', 'executing', { kind: 'attempt_terminal', attempt: {
    scheduleId: attempt.scheduleId, caseId: attempt.caseId, attemptId: attempt.attemptId,
    repetition: 1, seed: 101, environmentIds: ['E0'] }, resultState: 'passed',
    effectState: 'verified_effect', retryDisposition: 'prohibited_opportunistic_retry',
    terminal: true, firstFailureSignature: null, blocker: null, safetyStop: null, causalIds: {} });
  await appendEvent('scheduled_teardown_terminal', 'executing', { kind: 'scheduled_teardown_terminal',
    scheduleId: 'TEARDOWN-E0-r001', resultState: 'passed', effectState: 'verified_effect',
    retryDisposition: 'prohibited_opportunistic_retry', terminal: true });
  await appendEvent('controller_transition', 'execution_terminal', { kind: 'controller_transition',
    from: 'executing', to: 'execution_terminal', terminal: false });
  let state = 'execution_terminal';
  let seal = null;
  const controller = {
    snapshot() {
      return structuredClone({ schemaVersion: 'agent-browser.p158-live-resumed-controller.v1',
        state, runId, manifest, manifestSha256: manifestBinding.sha256,
        schedule: [{ ...attempt, resultState: 'passed' }],
        scheduledTeardown: { attemptId: 'TEARDOWN-E0-r001', resultState: 'passed' },
        evidence: { events, artifacts }, seal });
    },
    async writeArtifact({ artifactId, relativePath, content, metadata }) {
      if (state !== 'execution_terminal') throw new Error('wrong state');
      const storagePath = relativePath.startsWith('artifacts/') ? relativePath : `artifacts/${relativePath}`;
      const existing = await store.read(storagePath).catch((error) => {
        if (error?.code === 'ENOENT') return null;
        throw error;
      });
      let receipt;
      if (metadata.adoptExisting === true && existing && sha256(existing) === sha256(content)) {
        receipt = { artifactId, relativePath: storagePath, mediaType: metadata.mediaType,
          sha256: sha256(existing), byteCount: existing.byteLength, captureState: metadata.captureState,
          captureGap: metadata.captureGap ?? null, redactions: metadata.redactions ?? [],
          parentArtifactSha256s: artifacts.length ? [artifacts.at(-1).sha256] : [] };
        artifacts.push(receipt);
      } else {
        receipt = await rawArtifact(artifactId, relativePath, Buffer.from(content), metadata.mediaType);
      }
      await appendEvent('artifact_recorded', 'execution_terminal', { kind: 'artifact_recorded',
        artifactId, terminal: false }, [receipt]);
      return structuredClone(receipt);
    },
    async sealEvidence() {
      const evidenceManifest = { schemaVersion: 'agent-browser.p158-evidence-manifest.v1', runId,
        candidateSha256: sha256(manifest.candidate), registrySha256: sha256(registry),
        scheduleSha256: sha256([attempt]), resultsSha256: '22'.repeat(32),
        events: structuredClone(events), artifacts: structuredClone(artifacts), eventHeadSha256: parent };
      seal = await rawArtifact(`${runId}:sealed-manifest`, 'manifest/sealed-evidence-manifest.json',
        evidenceManifest, 'application/json', false);
      await appendEvent('evidence_seal', 'evidence_sealed', { kind: 'evidence_seal',
        manifestSha256: seal.sha256, ledgerHeadSha256: parent, artifactCount: artifacts.length,
        artifactBytes: artifacts.reduce((sum, item) => sum + item.byteCount, 0),
        allScheduledAttemptsTerminal: true, teardownTerminal: true, terminal: true }, [seal]);
      state = 'evidence_sealed';
    },
  };
  const authorities = {
    manifest: { path: path.join(runRoot, manifestBinding.relativePath),
      sha256: manifestBinding.sha256, byteCount: manifestBinding.byteCount },
    freeze: await writeAuthority('campaign-freeze.json', freeze),
    schedule: await writeAuthority('schedule.json', schedule),
    registry: await writeAuthority('registry.json', registry),
  };
  return { runRoot, runId, store, controller, authorities,
    rawArtifactInventory: [
      { ...logging, analysisRole: 'logging_evidence' },
      { ...dashboard, analysisRole: 'dashboard_fixture' },
    ] };
}

function operationGaps(runId) {
  return ['handoff-prepare', 'handoff-resume', 'handoff-finalize'].map((operationKind) => ({
    descriptorId: `${runId}:A13-001:${operationKind}`,
    operationCorrelationId: `${runId}:A13-001:${operationKind}`, productRequestId: null,
    correlationState: 'product_request_id_unavailable', operationKind,
    actionId: 'A13-001', attemptId: 'A13-E1-r001', caseId: 'A13', phaseId: 'W7',
    environmentId: 'E1', loggingGap: { code: 'product_request_id_not_preserved',
      detail: 'Product surface does not preserve the harness operation ID.' },
  }));
}

async function addOperationGapArtifact(harness, gaps) {
  const body = { schemaVersion: 'agent-browser.p158-logging-operation-gaps.v1', planId: 'P158',
    runId: harness.runId, operationGapCount: gaps.length, loggingOperationGapsSha256: sha256(gaps),
    operations: structuredClone(gaps), effectsAttempted: false, repairAttempted: false };
  const bytes = Buffer.from(canonicalJson(body));
  const entry = { artifactId: `${harness.runId}:a13-operation-gaps`,
    relativePath: 'artifacts/logging/operation-gaps.json', mediaType: 'application/json',
    sha256: sha256(bytes), byteCount: bytes.byteLength, captureState: 'complete', captureGap: null,
    redactions: [], parentArtifactSha256s: [], analysisRole: 'logging_operation_gaps' };
  await harness.store.writeOnce(entry.relativePath, bytes);
  harness.rawArtifactInventory.push(entry);
}

const harness = await createHarness();
const gaps = operationGaps(harness.runId);
await addOperationGapArtifact(harness, gaps);
const hook = createP158FinalAnalysisDescriptorHook({ runRoot: harness.runRoot,
  controller: harness.controller, artifactStore: harness.store, authorities: harness.authorities,
  loggingOperationGapsSha256: sha256(gaps), loggingOperationGapCount: gaps.length });
const preparation = await hook.prepareBeforeSeal({
  rawArtifactInventory: harness.rawArtifactInventory, loggingOperationGaps: gaps });
assert.equal(harness.controller.snapshot().state, 'execution_terminal');
assert.equal(harness.controller.snapshot().evidence.artifacts.filter((artifact) =>
  ['analysis-artifact-roles', 'a13-operation-gaps'].some((suffix) =>
    artifact.artifactId.endsWith(suffix))).length, 2);
await harness.controller.sealEvidence();
const finalized = await hook.finalizeAfterSeal({ preparation });
assert.equal(finalized.effectsAttempted, false);
assert.equal(finalized.repairAttempted, false);
assert.equal(finalized.descriptor.sourceBindings.length, 2);
assert.equal(finalized.descriptor.files.artifacts.filter((entry) =>
  entry.analysisRole === 'logging_operation_gaps').length, 1);
assert.equal(finalized.descriptor.files.artifacts.filter((entry) =>
  entry.analysisRole === 'dashboard_fixture').length, 1);
assert.equal(finalized.descriptor.files.ledger.length,
  harness.controller.snapshot().evidence.events.length);
assert.equal(sha256(await readFile(path.join(harness.runRoot, P158_FINAL_ANALYSIS_DESCRIPTOR_PATH))),
  finalized.descriptorSha256);
const analysis = await runP158FinalAnalysis({ descriptorPath: finalized.descriptorPath,
  descriptorSha256: finalized.descriptorSha256,
  clock: { wallNow: () => '2026-09-03T03:00:00.000Z' } });

const emptyHarness = await createHarness();
const emptyGaps = [];
await addOperationGapArtifact(emptyHarness, emptyGaps);
const emptyHook = createP158FinalAnalysisDescriptorHook({ runRoot: emptyHarness.runRoot,
  controller: emptyHarness.controller, artifactStore: emptyHarness.store, authorities: emptyHarness.authorities,
  loggingOperationGapsSha256: sha256(emptyGaps), loggingOperationGapCount: 0 });
const emptyPreparation = await emptyHook.prepareBeforeSeal({
  rawArtifactInventory: emptyHarness.rawArtifactInventory, loggingOperationGaps: emptyGaps });
await emptyHarness.controller.sealEvidence();
const emptyFinalized = await emptyHook.finalizeAfterSeal({ preparation: emptyPreparation });
assert.equal(emptyFinalized.descriptor.files.artifacts.filter((entry) =>
  entry.analysisRole === 'logging_operation_gaps').length, 1);
assert.equal(analysis.controllerState, 'analyzed');

const resumed = await hook.finalizeAfterSeal({ preparation });
assert.equal(resumed.resumed, true);
assert.equal(resumed.descriptorSha256, finalized.descriptorSha256);

const badRoles = await createHarness();
const badGaps = operationGaps(badRoles.runId);
await addOperationGapArtifact(badRoles, badGaps);
const badHook = createP158FinalAnalysisDescriptorHook({ runRoot: badRoles.runRoot,
  controller: badRoles.controller, artifactStore: badRoles.store, authorities: badRoles.authorities,
  loggingOperationGapsSha256: sha256(badGaps), loggingOperationGapCount: badGaps.length });
await assert.rejects(badHook.prepareBeforeSeal({
  rawArtifactInventory: badRoles.rawArtifactInventory.slice(0, 1), loggingOperationGaps: badGaps,
}), (error) => error.code === 'analysis_unclassified_raw_artifact');

const unsealed = await createHarness();
const unsealedGaps = operationGaps(unsealed.runId);
await addOperationGapArtifact(unsealed, unsealedGaps);
const unsealedHook = createP158FinalAnalysisDescriptorHook({ runRoot: unsealed.runRoot,
  controller: unsealed.controller, artifactStore: unsealed.store, authorities: unsealed.authorities,
  loggingOperationGapsSha256: sha256(unsealedGaps), loggingOperationGapCount: unsealedGaps.length });
const unsealedPreparation = await unsealedHook.prepareBeforeSeal({
  rawArtifactInventory: unsealed.rawArtifactInventory, loggingOperationGaps: unsealedGaps });
await assert.rejects(unsealedHook.finalizeAfterSeal({ preparation: unsealedPreparation }),
  (error) => error.code === 'analysis_finalize_order_invalid');

const unclassified = await createHarness();
await unclassified.store.writeOnce('unexpected-raw-output.json', canonicalJson({ unclassified: true }));
const unclassifiedGaps = operationGaps(unclassified.runId);
await addOperationGapArtifact(unclassified, unclassifiedGaps);
const unclassifiedHook = createP158FinalAnalysisDescriptorHook({ runRoot: unclassified.runRoot,
  controller: unclassified.controller, artifactStore: unclassified.store, authorities: unclassified.authorities,
  loggingOperationGapsSha256: sha256(unclassifiedGaps),
  loggingOperationGapCount: unclassifiedGaps.length });
await assert.rejects(unclassifiedHook.prepareBeforeSeal({
  rawArtifactInventory: unclassified.rawArtifactInventory, loggingOperationGaps: unclassifiedGaps,
}), (error) => error.code === 'analysis_unclassified_raw_artifact');

process.stdout.write('P158 W10 two-stage final analysis descriptor integration passed\n');
