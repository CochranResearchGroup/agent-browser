#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtemp, readFile, readdir, writeFile, mkdir } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import { canonicalJson, sha256 } from './lib/p158-campaign-controller.js';
import {
  P158_FINAL_ANALYSIS_PATH,
  P158_FINAL_REVIEW_PATH,
  runP158FinalAnalysis,
} from './lib/p158-final-analysis-runner.js';

const registry = JSON.parse(await readFile(
  'docs/dev/contracts/p158-historical-failure-registry.v1.json', 'utf8'));
const loggingCorpus = JSON.parse(await readFile(
  'docs/dev/fixtures/p158/logging-causal-envelopes.v1.json', 'utf8'));
const dashboardCorpus = JSON.parse(await readFile(
  'docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json', 'utf8'));
const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats(ajv);
const validateAnalysis = ajv.compile(JSON.parse(await readFile(
  'docs/dev/contracts/p158-final-analysis.v1.schema.json', 'utf8')));
const runnerSource = await readFile('scripts/lib/p158-final-analysis-runner.js', 'utf8');
assert(!runnerSource.includes('node:child_process'));
assert(!runnerSource.includes('globalThis.fetch'));

function projection(attempt) {
  return {
    scheduleSequence: attempt.scheduleSequence, scheduleId: attempt.scheduleId,
    caseId: attempt.caseId, attemptId: attempt.attemptId, repetition: attempt.repetition,
    seed: attempt.seed, environmentIds: attempt.environmentIds,
    dependsOnAttemptIds: attempt.dependsOnAttemptIds, preconditionIds: attempt.preconditionIds,
    stimuli: attempt.stimuli, evidenceProfile: attempt.evidenceProfile,
    externalIngressRequired: attempt.externalIngressRequired,
    preExecutionBlocker: attempt.preExecutionBlocker,
  };
}

function record({ runId, manifestSha256, sequence, previousRecordSha256, recordType,
  controllerState, payload, artifacts = [] }) {
  return {
    schemaVersion: 'agent-browser.p158-campaign-result.v1', planId: 'P158', runId,
    manifestSha256, recordId: `${runId}:record:${String(sequence).padStart(8, '0')}`,
    sequence, previousRecordSha256, recordType, controllerState,
    wallTime: new Date(Date.parse('2026-09-03T01:00:00.000Z') + sequence * 1000).toISOString(),
    monotonicTimeNanoseconds: sequence + 1, clockOffsetMilliseconds: 0, payload, artifacts,
  };
}

async function writeBound(runRoot, relativePath, value, extra = {}) {
  const bytes = Buffer.isBuffer(value) ? value : Buffer.from(canonicalJson(value));
  const target = path.join(runRoot, relativePath);
  await mkdir(path.dirname(target), { recursive: true });
  await writeFile(target, bytes);
  return { relativePath, sha256: sha256(bytes), byteCount: bytes.byteLength, ...extra };
}

async function fixture({ sealed = true, forbidden = false, binary = false } = {}) {
  const runRoot = await mkdtemp(path.join(os.tmpdir(), 'p158-w10-runner-'));
  const runId = `p158-w10-${path.basename(runRoot)}`;
  const registrySha256 = sha256(registry);
  const attempt = {
    scheduleSequence: 0, scheduleId: 'W7:A01-E0-r001', caseId: 'A01',
    attemptId: 'A01-E0-r001', repetition: 1, seed: 101, environmentIds: ['E0'],
    dependsOnAttemptIds: [], preconditionIds: [], stimuli: [], evidenceProfile: 'profile',
    externalIngressRequired: false, preExecutionBlocker: null,
  };
  const scheduleBody = {
    schemaVersion: 'agent-browser.p158-execution-schedule.v1', planId: 'P158', runId,
    registrySha256, attempts: [attempt], caseCount: 1, attemptCount: 1,
  };
  const schedule = { ...scheduleBody, scheduleSha256: sha256(scheduleBody) };
  const candidate = { candidateSha256: '11'.repeat(32) };
  const manifest = {
    schemaVersion: 'agent-browser.p158-campaign-manifest.v1', planId: 'P158', runId,
    registrySha256, controllerState: 'prepared', candidate,
    artifactBindings: [], environmentSeals: [], calibration: null, fixtureSeal: null,
    schedule: [projection(attempt)], freezePolicy: {}, safetyPolicy: {},
    evidencePolicy: { appendOnly: true, atomicWrites: true, digestAlgorithm: 'sha256',
      forbiddenCapturedFields: registry.forbiddenCapturedFields },
  };
  const manifestBinding = await writeBound(runRoot, 'campaign-manifest.json', manifest);
  const artifactBindings = [
    await writeBound(runRoot, 'artifacts/logging/w3-corpus.json', loggingCorpus, {
      artifactId: `${runId}:logging`, mediaType: 'application/json', analysisRole: 'logging_evidence',
      captureState: 'complete', captureGap: null, redactions: [], parentArtifactSha256s: [],
    }),
  ];
  artifactBindings.push(await writeBound(runRoot, 'artifacts/dashboard/w5-fixture.json',
    dashboardCorpus.baseline, { artifactId: `${runId}:dashboard`, mediaType: 'application/json',
      analysisRole: 'dashboard_fixture', captureState: 'complete', captureGap: null, redactions: [],
      parentArtifactSha256s: [artifactBindings.at(-1).sha256] }));
  const operationGaps = [{ descriptorId: `${runId}:A13-001:handoff-prepare`,
    operationCorrelationId: `${runId}:A13-001:handoff-prepare`, productRequestId: null,
    correlationState: 'product_request_id_unavailable', operationKind: 'handoff-prepare',
    actionId: 'A13-001', attemptId: 'A13-E1-r001', caseId: 'A13', phaseId: 'W7',
    environmentId: 'E1', loggingGap: { code: 'product_request_id_not_preserved',
      detail: 'Product request identity is unavailable.' } }];
  artifactBindings.push(await writeBound(runRoot, 'artifacts/logging/operation-gaps.json', {
    schemaVersion: 'agent-browser.p158-logging-operation-gaps.v1', planId: 'P158', runId,
    operationGapCount: operationGaps.length, loggingOperationGapsSha256: sha256(operationGaps),
    operations: operationGaps, effectsAttempted: false, repairAttempted: false,
  }, { artifactId: `${runId}:operation-gaps`, mediaType: 'application/json',
    analysisRole: 'logging_operation_gaps', captureState: 'complete', captureGap: null,
    redactions: [], parentArtifactSha256s: [artifactBindings.at(-1).sha256] }));
  if (forbidden) {
    artifactBindings.push(await writeBound(runRoot, 'artifacts/logging/forbidden.json', {
      schemaVersion: 'fixture.v1', credentialCharacters: 'must-not-survive',
    }, { artifactId: `${runId}:forbidden`, mediaType: 'application/json',
      analysisRole: 'sensitive_capture', captureState: 'complete', captureGap: null,
      redactions: [], parentArtifactSha256s: [artifactBindings.at(-1).sha256] }));
  }
  if (binary) {
    artifactBindings.push(await writeBound(runRoot, 'artifacts/pixels/frame.png',
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x00, 0xff]), {
        artifactId: `${runId}:binary-frame`, mediaType: 'image/png',
        analysisRole: 'pixel_evidence', captureState: 'complete', captureGap: null,
        redactions: [], parentArtifactSha256s: artifactBindings.length > 0
          ? [artifactBindings.at(-1).sha256] : [],
      }));
  }
  const evidenceManifest = {
    schemaVersion: 'agent-browser.p158-evidence-manifest.v1', runId,
    candidateSha256: sha256(candidate), registrySha256, scheduleSha256: sha256([projection(attempt)]),
    resultsSha256: '22'.repeat(32), events: [], artifacts: artifactBindings.map((entry) => ({
      artifactId: entry.artifactId, relativePath: entry.relativePath, mediaType: entry.mediaType,
      sha256: entry.sha256, byteCount: entry.byteCount, captureState: entry.captureState,
      captureGap: entry.captureGap, redactions: entry.redactions,
      parentArtifactSha256s: entry.parentArtifactSha256s,
    })), eventHeadSha256: null,
  };
  const evidenceBinding = await writeBound(runRoot, 'artifacts/manifest/sealed-evidence-manifest.json',
    evidenceManifest, { artifactId: `${runId}:sealed-manifest`, mediaType: 'application/json',
      analysisRole: 'evidence_manifest', captureState: 'complete', captureGap: null,
      redactions: [], parentArtifactSha256s: artifactBindings.length > 0
        ? [artifactBindings.at(-1).sha256] : [] });
  let previous = null;
  const records = [];
  const append = (recordType, controllerState, payload, artifacts = []) => {
    const value = record({ runId, manifestSha256: manifestBinding.sha256,
      sequence: records.length, previousRecordSha256: previous, recordType, controllerState,
      payload, artifacts });
    previous = sha256(canonicalJson(value));
    records.push(value);
  };
  append('controller_transition', 'prepared', { kind: 'controller_transition', from: null,
    to: 'prepared', terminal: false });
  const preparedLedgerHeadSha256 = previous;
  append('controller_transition', 'frozen', { kind: 'controller_transition', from: 'prepared',
    to: 'frozen', terminal: false });
  append('attempt_terminal', 'executing', { kind: 'attempt_terminal', attempt: {
    scheduleId: attempt.scheduleId, caseId: attempt.caseId, attemptId: attempt.attemptId,
    repetition: 1, seed: 101, environmentIds: ['E0'] }, resultState: 'passed',
    effectState: 'verified_effect', retryDisposition: 'prohibited_opportunistic_retry',
    terminal: true, firstFailureSignature: null, blocker: null, safetyStop: null, causalIds: {} });
  append('scheduled_teardown_terminal', 'executing', { kind: 'scheduled_teardown_terminal',
    scheduleId: 'TEARDOWN-E0-r001', resultState: 'passed', effectState: 'verified_effect',
    retryDisposition: 'prohibited_opportunistic_retry', terminal: true });
  if (sealed) append('evidence_seal', 'evidence_sealed', { kind: 'evidence_seal',
    manifestSha256: evidenceBinding.sha256, ledgerHeadSha256: previous,
    artifactCount: artifactBindings.length + 1,
    artifactBytes: [...artifactBindings, evidenceBinding].reduce((sum, entry) => sum + entry.byteCount, 0),
    allScheduledAttemptsTerminal: true, teardownTerminal: true,
    sealedAt: '2026-09-03T01:00:04.000Z', terminal: true,
  }, [{ artifactId: evidenceBinding.artifactId, relativePath: evidenceBinding.relativePath,
    mediaType: evidenceBinding.mediaType, sha256: evidenceBinding.sha256,
    byteCount: evidenceBinding.byteCount, captureState: 'complete', captureGap: null,
    redactions: [], parentArtifactSha256s: evidenceBinding.parentArtifactSha256s }]);
  const freeze = {
    schemaVersion: 'agent-browser.p158-campaign-freeze.v1', planId: 'P158', runId,
    freezeId: `${runId}:freeze`, controllerState: 'frozen', manifestSha256: manifestBinding.sha256,
    candidateSha256: candidate.candidateSha256, artifactBindingsSha256: sha256([]),
    environmentSealsSha256: sha256([]), calibrationSha256: sha256(null),
    fixtureSealSha256: sha256(null), preparedLedgerHeadSha256,
    frozenAt: '2026-09-03T01:00:01.000Z', monotonicTimeNanoseconds: 2,
    startedCaseCount: 0, startedAttemptCount: 0,
  };
  const [freezeBinding, scheduleBinding, registryBinding] = await Promise.all([
    writeBound(runRoot, 'campaign-freeze.json', freeze),
    writeBound(runRoot, 'schedule.json', schedule),
    writeBound(runRoot, 'registry.json', registry),
  ]);
  const ledgerBindings = [];
  for (const value of records) ledgerBindings.push(await writeBound(runRoot,
    `ledger/${String(value.sequence).padStart(8, '0')}-${value.recordType}.json`, value));
  const descriptor = {
    schemaVersion: 'agent-browser.p158-final-analysis-runner.v1', planId: 'P158', runRoot,
    sourceBindings: [
      { hookId: 'p158.final_analysis_descriptor',
        sourcePath: 'scripts/lib/p158-final-analysis-descriptor.js',
        sourceSha256: sha256(await readFile('scripts/lib/p158-final-analysis-descriptor.js')) },
      { hookId: 'p158.final_analysis_runner',
        sourcePath: 'scripts/lib/p158-final-analysis-runner.js',
        sourceSha256: sha256(await readFile('scripts/lib/p158-final-analysis-runner.js')) },
    ],
    files: { manifest: manifestBinding, freeze: freezeBinding, schedule: scheduleBinding,
      registry: registryBinding, evidenceManifest: evidenceBinding,
      ledger: ledgerBindings, artifacts: [...artifactBindings, evidenceBinding] },
    loggingExpectations: [], architectureCriteria: [], p157Criteria: [],
    output: { analysis: P158_FINAL_ANALYSIS_PATH, reviewCandidate: P158_FINAL_REVIEW_PATH },
  };
  const descriptorBinding = await writeBound(runRoot, 'p158-final-analysis-descriptor.json', descriptor);
  return { runRoot, descriptorPath: path.join(runRoot, descriptorBinding.relativePath),
    descriptorSha256: descriptorBinding.sha256, descriptor, artifactBindings };
}

async function run(input) {
  return runP158FinalAnalysis({ ...input, clock: { wallNow: () => '2026-09-03T02:00:00.000Z' } });
}

const clean = await fixture();
const beforeTopLevel = await readdir(clean.runRoot);
const originalFetch = globalThis.fetch;
globalThis.fetch = () => { throw new Error('network effect prohibited'); };
const result = await run(clean);
globalThis.fetch = originalFetch;
assert.equal(result.report.effectsAttempted, false);
assert.equal(validateAnalysis(result.report), true, ajv.errorsText(validateAnalysis.errors));
assert.equal(result.report.repairAttempted, false);
assert.equal(result.effectsAttempted, false);
assert.equal(result.reviewCandidate.redaction.rawArtifactsIncluded, false);
assert.equal(result.reviewCandidate.redaction.rawCausalRecordsIncluded, false);
assert.equal(result.reviewCandidate.redaction.automaticallyCommitted, false);
assert.equal(result.reviewCandidate.sourceReportSha256, result.report.reportSha256);
assert.equal(result.controllerState, 'analyzed');
assert.match(result.analysisTerminalSha256, /^[a-f0-9]{64}$/u);
assert.deepEqual((await readdir(clean.runRoot)).filter((entry) => !beforeTopLevel.includes(entry)), ['analysis']);
assert.deepEqual((await readdir(path.join(clean.runRoot, 'analysis'))).sort(),
  [path.basename(P158_FINAL_ANALYSIS_PATH), path.basename(P158_FINAL_REVIEW_PATH)].sort());

const analysisBefore = await readFile(path.join(clean.runRoot, P158_FINAL_ANALYSIS_PATH));
const reviewBefore = await readFile(path.join(clean.runRoot, P158_FINAL_REVIEW_PATH));
const resumed = await run(clean);
assert.equal(resumed.resumed, true);
assert.deepEqual(await readFile(path.join(clean.runRoot, P158_FINAL_ANALYSIS_PATH)), analysisBefore);
assert.deepEqual(await readFile(path.join(clean.runRoot, P158_FINAL_REVIEW_PATH)), reviewBefore);

const duplicateTerminal = await fixture();
await writeFile(path.join(duplicateTerminal.runRoot, 'ledger/99999999-analysis_terminal.json'), '{}\n');
await assert.rejects(run(duplicateTerminal), (error) => error.code === 'analysis_terminal_duplicate');

const tampered = await fixture();
await writeFile(path.join(tampered.runRoot, 'artifacts/manifest/sealed-evidence-manifest.json'), '{}\n');
await assert.rejects(run(tampered), (error) => error.code === 'sealed_artifact_binding_mismatch');

for (const selectBinding of [
  (value) => value.descriptor.files.manifest,
  (value) => value.descriptor.files.freeze,
  (value) => value.descriptor.files.schedule,
  (value) => value.descriptor.files.registry,
  (value) => value.descriptor.files.ledger[2],
]) {
  const authorityTamper = await fixture();
  await writeFile(path.join(authorityTamper.runRoot,
    selectBinding(authorityTamper).relativePath), '{}\n');
  await assert.rejects(run(authorityTamper),
    (error) => error.code === 'sealed_artifact_binding_mismatch');
}

for (const role of ['logging_evidence', 'dashboard_fixture']) {
  const roleTamper = await fixture();
  const binding = roleTamper.artifactBindings.find((entry) => entry.analysisRole === role);
  await writeFile(path.join(roleTamper.runRoot, binding.relativePath), '{}\n');
  await assert.rejects(run(roleTamper), (error) => error.code === 'sealed_artifact_binding_mismatch');
}

const incomplete = await fixture({ sealed: false });
await assert.rejects(run(incomplete), (error) => error.code === 'campaign_not_evidence_sealed');

const forbidden = await fixture({ forbidden: true });
const forbiddenResult = await run(forbidden);
assert.equal(forbiddenResult.report.integrity.passed, false);
assert(forbiddenResult.report.findings.some((finding) => finding.code === 'forbidden_capture_present'));
assert(!canonicalJson(forbiddenResult.reviewCandidate).includes('must-not-survive'));

const binary = await fixture({ binary: true });
const binaryResult = await run(binary);
assert.equal(binaryResult.controllerState, 'analyzed');

const changedDescriptor = await fixture();
await writeFile(changedDescriptor.descriptorPath, '{}\n');
await assert.rejects(run(changedDescriptor), (error) => error.code === 'analysis_descriptor_changed');

const forgedSource = await fixture();
forgedSource.descriptor.sourceBindings[0].sourceSha256 = 'ff'.repeat(32);
const forgedBytes = Buffer.from(canonicalJson(forgedSource.descriptor));
await writeFile(forgedSource.descriptorPath, forgedBytes);
await assert.rejects(run({ ...forgedSource, descriptorSha256: sha256(forgedBytes) }),
  (error) => error.code === 'analysis_source_binding_invalid');

process.stdout.write('P158 W10 final analysis runner passed sealed/tamper/incomplete/redaction/resume/no-effect checks\n');
