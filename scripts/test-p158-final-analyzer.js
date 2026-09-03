#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import { canonicalJson } from './lib/p158-campaign-controller.js';
import {
  analyzeP158SealedCampaign,
  stableP158AnalysisHash,
} from './lib/p158-final-analyzer.js';

const repoRoot = new URL('..', import.meta.url).pathname;
const registry = JSON.parse(readFileSync(join(
  repoRoot, 'docs/dev/contracts/p158-historical-failure-registry.v1.json',
), 'utf8'));
const loggingCorpus = JSON.parse(readFileSync(join(
  repoRoot, 'docs/dev/fixtures/p158/logging-causal-envelopes.v1.json',
), 'utf8'));
const handoffCorpus = JSON.parse(readFileSync(join(
  repoRoot, 'docs/dev/fixtures/p158/external-handoff-sessions.v1.json',
), 'utf8'));
const dashboardCorpus = JSON.parse(readFileSync(join(
  repoRoot, 'docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json',
), 'utf8'));
const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats(ajv);
const validateReport = ajv.compile(JSON.parse(readFileSync(join(
  repoRoot, 'docs/dev/contracts/p158-final-analysis.v1.schema.json',
), 'utf8')));

function clone(value) {
  return structuredClone(value);
}

function ledgerRecord({ runId, manifestSha256, sequence, previousRecordSha256, recordType, payload }) {
  const record = {
    schemaVersion: 'agent-browser.p158-campaign-result.v1',
    planId: 'P158',
    runId,
    manifestSha256,
    recordId: `${runId}:record:${String(sequence).padStart(8, '0')}`,
    sequence,
    previousRecordSha256,
    recordType,
    controllerState: recordType === 'evidence_seal' ? 'evidence_sealed' : 'executing',
    wallTime: new Date(Date.parse('2026-09-03T01:00:00.000Z') + sequence * 1000).toISOString(),
    monotonicTimeNanoseconds: sequence + 1,
    clockOffsetMilliseconds: 0,
    payload,
    artifacts: [],
  };
  record.sha256 = stableP158AnalysisHash(record);
  return record;
}

function cleanEvidence({
  resultState = 'passed',
  firstFailureSignature = null,
  includeRawAudits = true,
} = {}) {
  const runId = 'p158-w10-synthetic';
  const manifest = {
    schemaVersion: 'agent-browser.p158-campaign-manifest.v1',
    planId: 'P158',
    runId,
    candidate: { candidateSha256: '11'.repeat(32) },
    schedule: [{
      caseId: 'A01',
      attemptId: 'A01-E0-r001',
      environmentId: 'E0',
      environmentIds: ['E0'],
      seed: 101,
    }],
  };
  const manifestSha256 = stableP158AnalysisHash(manifest);
  const terminal = ledgerRecord({
    runId,
    manifestSha256,
    sequence: 0,
    previousRecordSha256: null,
    recordType: 'attempt_terminal',
    payload: {
      kind: 'attempt_terminal',
      attempt: {
        scheduleId: 'W7:A01-E0-r001',
        caseId: 'A01',
        attemptId: 'A01-E0-r001',
        repetition: 1,
        seed: 101,
        environmentIds: ['E0'],
      },
      resultState,
      effectState: resultState === 'passed' ? 'verified_effect' : 'effect_uncertain',
      retryDisposition: 'prohibited_opportunistic_retry',
      completedAt: '2026-09-03T01:00:00.000Z',
      terminal: true,
      firstFailureSignature,
      blocker: null,
      safetyStop: null,
      causalIds: { requestId: 'request-good' },
    },
  });
  const teardown = ledgerRecord({
    runId,
    manifestSha256,
    sequence: 1,
    previousRecordSha256: terminal.sha256,
    recordType: 'scheduled_teardown_terminal',
    payload: {
      kind: 'scheduled_teardown_terminal', scheduleId: 'TEARDOWN-E0-r001', resultState: 'passed',
      effectState: 'verified_effect', retryDisposition: 'prohibited_opportunistic_retry',
      completedAt: '2026-09-03T01:00:01.000Z', terminal: true,
    },
  });
  const evidenceManifestBytes = Buffer.from(canonicalJson({
    schemaVersion: 'agent-browser.p158-evidence-manifest.v1', runId, terminal: true,
  }));
  const artifact = {
    artifactId: `${runId}:sealed-manifest`,
    relativePath: 'artifacts/manifest/sealed-evidence-manifest.json',
    mediaType: 'application/json',
    sha256: stableP158AnalysisHash(JSON.parse(evidenceManifestBytes.toString('utf8'))),
    byteCount: evidenceManifestBytes.byteLength,
    captureState: 'complete',
    captureGap: null,
    redactions: [],
    parentArtifactSha256s: [],
    bytes: evidenceManifestBytes,
  };
  assert.equal(artifact.sha256, stableP158AnalysisHash(JSON.parse(evidenceManifestBytes)));
  const seal = ledgerRecord({
    runId,
    manifestSha256,
    sequence: 2,
    previousRecordSha256: teardown.sha256,
    recordType: 'evidence_seal',
    payload: {
      kind: 'evidence_seal', manifestSha256: artifact.sha256,
      ledgerHeadSha256: teardown.sha256, artifactCount: 1,
      artifactBytes: artifact.byteCount, allScheduledAttemptsTerminal: true,
      teardownTerminal: true, sealedAt: '2026-09-03T01:00:02.000Z', terminal: true,
    },
  });
  seal.artifacts = [clone(artifact)];
  delete seal.artifacts[0].bytes;
  seal.sha256 = stableP158AnalysisHash(Object.fromEntries(
    Object.entries(seal).filter(([key]) => !['sha256', 'byteCount', 'type'].includes(key)),
  ));
  const input = {
    runId,
    manifest,
    manifestSha256,
    ledgerRecords: [terminal, teardown, seal],
    artifacts: [artifact],
    registry,
    analyzedAt: '2026-09-03T02:00:00.000Z',
  };
  if (includeRawAudits) {
    input.loggingExpectations = [{
      attemptId: 'A01-E0-r001',
      requestId: 'request-good',
      incidentExpected: false,
      operatorVisible: false,
      expectedSurfaceRoles: [
        'ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome',
      ],
    }];
    input.loggingEvidence = [{
      ...clone(loggingCorpus),
      fixtures: loggingCorpus.fixtures.filter((fixture) => fixture.fixtureId === 'logging-good-complete'),
    }];
    input.externalHandoffSessions = [clone(handoffCorpus.sessions.find(
      (session) => session.fixtureId === 'handoff-clean-public-https',
    ))];
    input.dashboardFixtures = [clone(dashboardCorpus.baseline)];
  }
  return input;
}

function analyze(input) {
  return analyzeP158SealedCampaign({
    sealedCampaign: input,
    architectureCriteria: [{
      criterionId: 'profile-acquisition-owner',
      boundary: 'Profile acquisition has one owner.',
      caseIds: ['A01'],
    }],
    p157Criteria: [{
      criterionId: 'p157-profile-sharing',
      statement: 'Profile sharing remains coherent.',
      caseIds: ['A01'],
    }],
    clock: { wallNow: () => '2026-09-03T02:00:00.000Z' },
  });
}

function runTest(name, body) {
  body();
  process.stdout.write(`PASS ${name}\n`);
}

runTest('independently verifies a sealed clean campaign without effects', () => {
  const input = cleanEvidence();
  const beforeSha256 = stableP158AnalysisHash(input);
  const report = analyze(input);
  assert.equal(report.integrity.passed, true);
  assert.equal(validateReport(report), true, ajv.errorsText(validateReport.errors));
  assert.equal(report.effectsAttempted, false);
  assert.equal(report.repairAttempted, false);
  assert.equal(report.findings.length, 0);
  assert.equal(report.resultSet.resultCounts.passed, 1);
  assert.equal(report.p157Acceptance[0].status, 'proven');
  assert.equal(report.architectureAssessments[0].assessment, 'insufficient_evidence');
  assert.match(report.resultSetSha256, /^[a-f0-9]{64}$/);
  assert.match(report.remediationGraphSha256, /^[a-f0-9]{64}$/);
  assert.match(report.reportSha256, /^[a-f0-9]{64}$/);
  assert.equal(stableP158AnalysisHash(input), beforeSha256);
  assert.deepEqual(analyze(clone(input)), report);
});

runTest('retains historical reproduction in clusters timelines and criterion judgment', () => {
  const report = analyze(cleanEvidence({
    resultState: 'reproduced_historical_failure',
    firstFailureSignature: 'existing_session_profile_identity_unproven',
  }));
  assert.equal(report.resultSet.clusters.length, 1);
  assert.equal(report.resultSet.clusters[0].signature, 'existing_session_profile_identity_unproven');
  assert.equal(report.resultSet.timelines[0].attemptId, 'A01-E0-r001');
  assert.equal(report.resultSet.historicalReproduction.find(
    (family) => family.familyId === 'F01',
  ).reproducedAttemptCount, 1);
  assert.equal(report.p157Acceptance[0].status, 'disproven');
  assert(report.findings.some((finding) => finding.disposition === 'blocking'));
});

runTest('detects artifact tampering and incomplete raw recomputation without repair', () => {
  const input = cleanEvidence({ includeRawAudits: false });
  input.artifacts[0].bytes = Buffer.from('tampered\n');
  const report = analyze(input);
  assert.equal(report.integrity.passed, false);
  assert(report.findings.some((finding) => finding.code === 'artifact_hash_mismatch'));
  assert(report.findings.some((finding) => finding.code === 'logging_raw_evidence_missing'));
  assert(report.findings.some((finding) => finding.code === 'handoff_raw_evidence_missing'));
  assert(report.findings.some((finding) => finding.code === 'dashboard_raw_evidence_missing'));
  assert.equal(report.remediationGraph.nodes.length, report.findings.length);
});

runTest('detects broken ledger ancestry and refuses to call it intact', () => {
  const input = cleanEvidence();
  input.ledgerRecords[1].previousRecordSha256 = 'ff'.repeat(32);
  const report = analyze(input);
  assert.equal(report.integrity.passed, false);
  assert(report.findings.some((finding) => finding.code === 'ledger_chain_invalid'));
});

runTest('does not let an unrelated clean logging envelope satisfy a terminal attempt', () => {
  const input = cleanEvidence();
  input.loggingExpectations[0].requestId = 'request-for-this-attempt';
  input.ledgerRecords[0].payload.causalIds = { requestId: 'request-for-this-attempt' };
  input.ledgerRecords[0].sha256 = stableP158AnalysisHash(input.ledgerRecords[0]);
  input.ledgerRecords[1].previousRecordSha256 = input.ledgerRecords[0].sha256;
  input.ledgerRecords[1].sha256 = stableP158AnalysisHash(input.ledgerRecords[1]);
  input.ledgerRecords[2].previousRecordSha256 = input.ledgerRecords[1].sha256;
  input.ledgerRecords[2].payload.ledgerHeadSha256 = input.ledgerRecords[1].sha256;
  input.ledgerRecords[2].sha256 = stableP158AnalysisHash(input.ledgerRecords[2]);
  const report = analyze(input);
  assert(report.findings.some((finding) => finding.code === 'logging_attempt_envelope_missing'));
});

runTest('requires every attempt to declare its expected causal logging measurements', () => {
  const input = cleanEvidence();
  delete input.loggingExpectations;
  const report = analyze(input);
  assert(report.findings.some((finding) => finding.code === 'logging_expectation_missing'));
});

runTest('retains explicitly missing response job event trace incident and dashboard measurements', () => {
  const input = cleanEvidence();
  const fixture = input.loggingEvidence[0].fixtures[0];
  fixture.operatorVisible = true;
  fixture.incidentExpected = true;
  fixture.expectedSurfaceRoles.push('incident', 'dashboard_projection');
  fixture.records = fixture.records.filter((record) =>
    !['immediate_response', 'durable_job', 'terminal_event', 'trace_outcome'].includes(record.surfaceRole));
  input.loggingExpectations[0] = {
    ...input.loggingExpectations[0],
    incidentExpected: true,
    operatorVisible: true,
    expectedSurfaceRoles: [
      'ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome',
      'incident', 'dashboard_projection',
    ],
  };
  const report = analyze(input);
  assert(report.findings.some((finding) => finding.code === 'logging:missing_record'));
  assert.equal(report.independentAudits.logging[0].missingRecordCount, 6);
  assert(!report.findings.some((finding) => finding.code === 'logging_attempt_envelope_missing'));
});

process.stdout.write('P158 W10 final analyzer provider-free self-test passed\n');
