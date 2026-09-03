#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import { auditCausalEnvelopes } from './lib/p158-logging-auditor.js';

const root = new URL('..', import.meta.url).pathname;

function readJson(relativePath) {
  return JSON.parse(readFileSync(join(root, relativePath), 'utf8'));
}

const fixtureSet = readJson('docs/dev/fixtures/p158/logging-causal-envelopes.v1.json');
const fixtureSchema = readJson('docs/dev/contracts/p158-logging-causal-fixtures.v1.schema.json');
const reportSchema = readJson('docs/dev/contracts/p158-logging-audit-report.v1.schema.json');
const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
const validateFixtureSet = ajv.compile(fixtureSchema);
const validateReport = ajv.compile(reportSchema);

const findingCodes = [
  'missing_record',
  'duplicate_terminal',
  'conflicting_projection',
  'timestamp_inversion',
  'null_failure',
  'null_provenance',
  'one_transport_only',
  'broken_parent',
  'effect_retry_conflict',
  'capture_gap',
  'sensitive_value_leak',
];

const summaryFields = {
  missing_record: 'missingRecordCount',
  duplicate_terminal: 'duplicateTerminalCount',
  conflicting_projection: 'conflictingProjectionCount',
  timestamp_inversion: 'timestampInversionCount',
  null_failure: 'nullFailureCount',
  null_provenance: 'nullProvenanceCount',
  one_transport_only: 'oneTransportOnlyCount',
  broken_parent: 'brokenParentCount',
  effect_retry_conflict: 'effectRetryConflictCount',
  capture_gap: 'captureGapCount',
  sensitive_value_leak: 'sensitiveValueLeakCount',
};

function assertValid(validate, value, label) {
  assert.equal(
    validate(value),
    true,
    `${label} violates its JSON Schema: ${ajv.errorsText(validate.errors, { separator: '; ' })}`,
  );
}

function sorted(values) {
  return [...values].sort();
}

function clone(value) {
  return structuredClone(value);
}

function audit(input = fixtureSet) {
  return auditCausalEnvelopes({
    fixtureSet: input,
    options: {
      runId: 'p158-logging-auditor-self-test',
      auditId: 'p158-logging-auditor-self-test:audit',
      auditedAt: '2026-09-02T22:00:00.000Z',
    },
  });
}

function fixtureReport(report, fixtureId) {
  const fixture = fixtureSet.fixtures.find((entry) => entry.fixtureId === fixtureId);
  assert.ok(fixture, `fixture corpus omitted ${fixtureId}`);
  const requestId = fixture.records.find((record) => record.surfaceRole === 'ingress_request')?.requestId;
  const envelope = report.envelopes.find((entry) => entry.requestId === requestId);
  assert.ok(envelope, `audit report omitted fixture ${fixtureId}`);
  return {
    envelope,
    findings: report.findings.filter((finding) => finding.envelopeId === fixtureId),
  };
}

function runTest(name, body) {
  try {
    body();
    process.stdout.write(`PASS ${name}\n`);
  } catch (error) {
    error.message = `${name}: ${error.message}`;
    throw error;
  }
}

runTest('accepts the frozen synthetic fixture corpus', () => {
  assertValid(validateFixtureSet, fixtureSet, 'logging-causal-envelopes.v1.json');
  assert.equal(fixtureSet.syntheticOnly, true);
  assert.equal(fixtureSet.fixtures.length, 13);
  assert.deepEqual(
    sorted(fixtureSet.fixtures.flatMap((fixture) => fixture.expectedFindingCodes)),
    sorted(findingCodes),
    'fixture corpus does not isolate every required W3 defect exactly once',
  );
});

const originalInput = clone(fixtureSet);
const report = audit();

runTest('emits a deterministic schema-valid report without mutating evidence', () => {
  assertValid(validateReport, report, 'logging audit report');
  assert.deepEqual(fixtureSet, originalInput, 'auditor mutated its input records or expectations');
  assert.deepEqual(audit(clone(fixtureSet)), report, 'same fixture corpus produced a different report');
  assert.equal(report.repairAttempted, false);
  assert.ok(report.findings.every((finding) => finding.repairAttempted === false));
  assert.deepEqual(
    report.findings.map((finding) => finding.findingId),
    sorted(report.findings.map((finding) => finding.findingId)),
    'finding order is not deterministic',
  );
});

for (const fixture of fixtureSet.fixtures) {
  const expectedCodes = sorted(fixture.expectedFindingCodes);
  runTest(`classifies ${fixture.fixtureId} exactly`, () => {
    const { envelope, findings } = fixtureReport(report, fixture.fixtureId);
    assert.deepEqual(sorted(findings.map((finding) => finding.code)), expectedCodes);
    assert.deepEqual(sorted(envelope.findingIds), sorted(findings.map((finding) => finding.findingId)));
    assert.equal(envelope.requestId, fixture.records[0].requestId);
    assert.equal(envelope.sourceRecordCount, fixture.records.length);
    assert.deepEqual(envelope.expectedSurfaceRoles, fixture.expectedSurfaceRoles);
    assert.deepEqual(
      sorted(envelope.observedSurfaceRoles),
      sorted(new Set(fixture.records.map((record) => record.surfaceRole))),
    );
    assert.equal(envelope.state, expectedCodes.length === 0 ? 'complete' : (
      expectedCodes.includes('sensitive_value_leak') ? 'leaking' :
        expectedCodes.some((code) => [
          'conflicting_projection', 'effect_retry_conflict',
        ].includes(code)) ? 'conflicting' : 'incomplete'
    ));
  });
}

runTest('reports exact closed-world counts for every defect class', () => {
  const expectedRecords = fixtureSet.fixtures.reduce(
    (count, fixture) => count + fixture.expectedSurfaceRoles.length,
    0,
  );
  const observedRecords = fixtureSet.fixtures.reduce(
    (count, fixture) => count + fixture.records.length,
    0,
  );
  assert.equal(report.summary.envelopeCount, fixtureSet.fixtures.length);
  const cleanFixtureCount = fixtureSet.fixtures.filter(
    (fixture) => fixture.expectedFindingCodes.length === 0,
  ).length;
  assert.equal(report.summary.completeEnvelopeCount, cleanFixtureCount);
  assert.equal(report.summary.incompleteEnvelopeCount, fixtureSet.fixtures.length - cleanFixtureCount);
  assert.equal(report.summary.expectedRecordCount, expectedRecords);
  assert.equal(report.summary.observedRecordCount, observedRecords);
  for (const code of findingCodes) {
    const findings = report.findings.filter((finding) => finding.code === code);
    assert.equal(findings.length, 1, `${code} did not produce exactly one isolated finding`);
    assert.equal(report.summary[summaryFields[code]], 1, `${summaryFields[code]} drifted`);
  }
  assert.equal(report.findings.length, findingCodes.length);
});

runTest('detects historical null envelopes on terminal durable jobs', () => {
  const nullFailure = fixtureReport(report, 'logging-null-failure').findings[0];
  const nullProvenance = fixtureReport(report, 'logging-null-provenance').findings[0];
  assert.equal(nullFailure.code, 'null_failure');
  assert.deepEqual(nullFailure.surfaceRoles, ['durable_job']);
  assert.ok(nullFailure.observed.recordIds.includes('nullfail-job'));
  assert.equal(nullProvenance.code, 'null_provenance');
  assert.deepEqual(nullProvenance.surfaceRoles, ['durable_job']);
  assert.ok(nullProvenance.observed.recordIds.includes('nullprov-job'));
});

runTest('keeps complete and reordered causal envelopes clean across every expected surface', () => {
  const cleanFixtures = fixtureSet.fixtures.filter(
    (fixture) => fixture.expectedFindingCodes.length === 0,
  );
  assert.deepEqual(
    sorted(cleanFixtures.map((fixture) => fixture.fixtureId)),
    ['logging-good-complete', 'logging-reordered-input'],
  );
  for (const clean of cleanFixtures) {
    const { envelope, findings } = fixtureReport(report, clean.fixtureId);
    assert.deepEqual(findings, []);
    assert.equal(envelope.state, 'complete');
    assert.deepEqual(sorted(envelope.observedSurfaceRoles), sorted(clean.expectedSurfaceRoles));
  }
});

runTest('audits an honest pre-execution blocker on controller-only evidence surfaces', () => {
  const base = clone(fixtureSet.fixtures.find((fixture) => fixture.fixtureId === 'logging-good-complete'));
  const template = base.records[0];
  const requestId = 'blocked-request-001';
  const blockerFailure = {
    schemaVersion: 'agent-browser.service-failure-recourse.v1', code: 'live_case_hook_missing',
    axis: 'unknown', phase: 'finalize', effectState: 'no_effect',
    retryDisposition: 'do_not_retry', recommendedAction: 'retain_explicit_blocker',
  };
  const fixture = {
    ...base,
    fixtureId: 'logging-clean-explicit-blocker',
    description: 'A frozen explicit blocker has controller transition, blocker, and terminal evidence only.',
    operatorVisible: false,
    incidentExpected: false,
    expectedSurfaceRoles: ['controller_transition', 'pre_execution_blocker', 'terminal_event'],
    expectedFindingCodes: [],
    records: [
      { ...template, surfaceRole: 'controller_transition', transport: 'service', recordId: 'blocked-transition',
        requestId, timestamp: '2026-09-02T22:01:00.000Z', parentId: null, terminal: false,
        state: 'accepted', phase: 'scheduler_admission', effectState: 'no_effect' },
      { ...template, surfaceRole: 'pre_execution_blocker', transport: 'service', recordId: 'blocked-declaration',
        requestId, timestamp: '2026-09-02T22:01:00.010Z', parentId: 'blocked-transition', terminal: true,
        state: 'rejected', phase: 'scheduler_admission', effectState: 'no_effect',
        failure: blockerFailure },
      { ...template, surfaceRole: 'terminal_event', transport: 'service', recordId: 'blocked-terminal',
        requestId, timestamp: '2026-09-02T22:01:00.020Z', parentId: 'blocked-declaration', terminal: true,
        state: 'rejected', phase: 'finalize', effectState: 'no_effect',
        failure: blockerFailure },
    ],
  };
  assertValid(validateFixtureSet, { ...clone(fixtureSet), fixtures: [...clone(fixtureSet.fixtures), fixture] },
    'blocked logging fixture');
  const blockedFixtureSet = { ...clone(fixtureSet), fixtures: [fixture] };
  const blockedReport = audit(blockedFixtureSet);
  assertValid(validateReport, blockedReport, 'blocked logging report');
  assert.equal(blockedReport.findings.length, 0, JSON.stringify(blockedReport.findings));
  assert.equal(blockedReport.summary.expectedRecordCount, 3);
  assert.equal(blockedReport.summary.observedRecordCount, 3);
  assert.deepEqual(blockedReport.envelopes[0].expectedSurfaceRoles,
    ['controller_transition', 'pre_execution_blocker', 'terminal_event']);
});

process.stdout.write('P158 logging auditor adversarial self-test passed\n');
