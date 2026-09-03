#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import {
  DASHBOARD_FINDING_CODES,
  auditDashboardFixture,
  auditDashboardProjection,
  calculateResourceSlopes,
  calculateTimingDistribution,
  generateDenseDashboardFixture,
  materializeDashboardFixture,
} from './lib/p158-dashboard-oracle.js';

const root = new URL('..', import.meta.url).pathname;

function readJson(relativePath) {
  return JSON.parse(readFileSync(join(root, relativePath), 'utf8'));
}

const fixtureSet = readJson('docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json');
const fixtureSchema = readJson('docs/dev/contracts/p158-dashboard-fixtures.v1.schema.json');
const reportSchema = readJson('docs/dev/contracts/p158-dashboard-oracle-report.v1.schema.json');
const findingCodes = fixtureSchema.$defs.findingCode.enum;
const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
ajv.addSchema(fixtureSchema);
const validateFixtureSet = ajv.getSchema(fixtureSchema.$id);
const validateMaterializedFixture = ajv.compile({ $ref: `${fixtureSchema.$id}#/$defs/fixture` });
const validateReport = ajv.compile(reportSchema);

function clone(value) {
  return structuredClone(value);
}

function sorted(values) {
  return [...values].sort();
}

function assertValid(validate, value, label) {
  assert.equal(
    validate(value),
    true,
    `${label} violates its JSON Schema: ${ajv.errorsText(validate.errors, { separator: '; ' })}`,
  );
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

function caseSpec(fixtureId) {
  const value = fixtureSet.fixtures.find((fixture) => fixture.fixtureId === fixtureId);
  assert.ok(value, `fixture corpus omitted ${fixtureId}`);
  return value;
}

function materialized(fixtureId) {
  return materializeDashboardFixture({ baseline: fixtureSet.baseline, caseSpec: caseSpec(fixtureId) });
}

runTest('accepts the closed-world dashboard fixture corpus', () => {
  assertValid(validateFixtureSet, fixtureSet, 'dashboard-oracle-fixtures.v1.json');
  assert.equal(fixtureSet.syntheticOnly, true);
  assert.deepEqual(sorted(DASHBOARD_FINDING_CODES), sorted(findingCodes));
  assert.deepEqual(
    sorted(new Set(fixtureSet.fixtures.flatMap((fixture) => fixture.expectedFindingCodes))),
    sorted(findingCodes),
    'corpus does not isolate every dashboard defect code',
  );
  assert.deepEqual(
    sorted(new Set(fixtureSet.fixtures.map((fixture) => fixture.density))),
    ['dense', 'empty', 'normal', 'sparse'],
  );
});

const originalFixtureSet = clone(fixtureSet);
const reportSet = auditDashboardProjection({
  fixtureSet,
  options: { auditedAt: '2026-09-03T00:00:00.000Z' },
});

runTest('emits deterministic schema-valid reports without mutation or repair', () => {
  assert.equal(reportSet.reports.length, fixtureSet.fixtures.length);
  for (const report of reportSet.reports) {
    assertValid(validateReport, report, report.fixtureId);
    assert.equal(report.repairAttempted, false);
    assert.ok(report.findings.every((finding) => finding.repairAttempted === false));
    assert.deepEqual(
      report.findings.map((finding) => finding.findingId),
      sorted(report.findings.map((finding) => finding.findingId)),
      `${report.fixtureId} finding order drifted`,
    );
  }
  assert.deepEqual(fixtureSet, originalFixtureSet, 'dashboard oracle mutated its fixture corpus');
  assert.deepEqual(
    auditDashboardProjection({
      fixtureSet: clone(fixtureSet),
      options: { auditedAt: '2026-09-03T00:00:00.000Z' },
    }),
    reportSet,
    'dashboard oracle is not deterministic',
  );
});

for (const spec of fixtureSet.fixtures) {
  runTest(`classifies ${spec.fixtureId} exactly`, () => {
    const report = reportSet.reports.find((entry) => entry.fixtureId === spec.fixtureId);
    assert.ok(report, `report set omitted ${spec.fixtureId}`);
    const codes = sorted(new Set(report.findings.map((finding) => finding.code)));
    assert.deepEqual(codes, sorted(spec.expectedFindingCodes));
    assert.equal(report.passed, spec.expectedFindingCodes.length === 0);
    assert.equal(report.summary.findingCount, report.findings.length);
    for (const code of findingCodes) {
      assert.equal(
        report.summary.findingCounts[code] ?? 0,
        report.findings.filter((finding) => finding.code === code).length,
        `${spec.fixtureId} ${code} count drifted`,
      );
    }
  });
}

runTest('proves an exact left-rail bijection with stable same-label identities', () => {
  const baseline = fixtureSet.baseline;
  assertValid(validateMaterializedFixture, baseline, 'dashboard baseline');
  const expected = baseline.truth.resources.filter((resource) => resource.rowExpected);
  assert.equal(baseline.railRows.length, expected.length);
  assert.equal(new Set(baseline.railRows.map((row) => row.rowId)).size, baseline.railRows.length);
  assert.equal(new Set(baseline.railRows.map((row) => row.resourceId)).size, baseline.railRows.length);
  assert.deepEqual(
    baseline.railRows.map((row) => row.resourceId),
    [...expected].sort((left, right) => left.orderKey - right.orderKey).map((resource) => resource.resourceId),
    'left-rail order is not the authoritative resource order',
  );
  for (const resource of expected) {
    const rows = baseline.railRows.filter((row) => row.resourceId === resource.resourceId);
    assert.equal(rows.length, 1, resource.resourceId);
    const [row] = rows;
    assert.deepEqual(
      {
        resourceType: row.resourceType,
        label: row.label,
        state: row.state,
        orderKey: row.orderKey,
        badge: row.badge,
        count: row.count,
        snapshotRevision: row.snapshotRevision,
      },
      {
        resourceType: resource.resourceType,
        label: resource.label,
        state: resource.state,
        orderKey: resource.orderKey,
        badge: resource.badge,
        count: resource.count,
        snapshotRevision: baseline.truth.snapshotRevision,
      },
    );
  }
  const sameLabel = materialized('defect-unstable-same-label-identity');
  const repeatedLabels = sameLabel.truth.resources.filter(
    (resource, index, all) => all.some(
      (other, otherIndex) => otherIndex !== index && other.label === resource.label,
    ),
  );
  assert.ok(repeatedLabels.length >= 2, 'same-label fixture does not contain distinct resources');
  assert.equal(new Set(repeatedLabels.map((resource) => resource.resourceId)).size, repeatedLabels.length);
});

runTest('keeps selection, inspector, actions, and multi-client state keyed by IDs', () => {
  const baseline = fixtureSet.baseline;
  assert.equal(baseline.selection.selectedResourceId, baseline.selection.inspectorResourceId);
  assert.ok(baseline.truth.resources.some(
    (resource) => resource.resourceId === baseline.selection.selectedResourceId,
  ));
  assert.ok(baseline.actions.every((action) =>
    action.invokedTargetResourceId === null ||
      action.invokedTargetResourceId === action.declaredTargetResourceId));
  assert.ok(baseline.actions.every((action) => action.displayedEligible === action.eligible));

  const leakageSpec = fixtureSet.fixtures.find(
    (fixture) => fixture.expectedFindingCodes.includes('multi_client_selection_leakage'),
  );
  assert.ok(leakageSpec, 'corpus omits typed multi-client selection leakage');
  const leakage = materializeDashboardFixture({ baseline, caseSpec: leakageSpec });
  assert.ok(leakage.clientSelections.length >= 2);
  assert.equal(
    new Set(leakage.clientSelections.map((client) => client.clientId)).size,
    leakage.clientSelections.length,
  );
  const parameters = leakageSpec.mutation.parameters;
  const clientA = leakage.clientSelections.find((client) => client.clientId === parameters.clientAId);
  const clientB = leakage.clientSelections.find((client) => client.clientId === parameters.clientBId);
  assert.ok(clientA && clientB, 'the typed leakage seed omitted a named client');
  assert.notEqual(clientA.clientId, clientB.clientId);
  assert.equal(clientA.expectedSelectedResourceId, parameters.expectedClientAResourceId);
  assert.equal(clientA.observedSelectedResourceId, parameters.expectedClientAResourceId);
  assert.equal(clientB.expectedSelectedResourceId, parameters.expectedClientBResourceId);
  assert.equal(clientB.observedSelectedResourceId, parameters.expectedClientAResourceId);
  assert.ok(leakage.clientSelections.every((client) => !Object.hasOwn(client, 'label')));
  assert.ok(leakage.clientSelections.some(
    (client) => client.expectedSelectedResourceId !== client.observedSelectedResourceId ||
      client.expectedInspectorResourceId !== client.observedInspectorResourceId,
  ));
  assert.ok(auditDashboardFixture({ fixture: leakage }).findings.some(
    (finding) => finding.code === 'multi_client_selection_leakage',
  ));
});

runTest('keeps warning axes independent and exposes exactly one convergence action', () => {
  assert.deepEqual(fixtureSet.baseline.warnings.displayedAxes, []);
  assert.deepEqual(fixtureSet.baseline.warnings.convergenceActionIds, []);
  const convergence = materialized('warning-typed-convergence-clean');
  assert.equal(convergence.warnings.runtimeHealth, 'healthy');
  assert.equal(convergence.warnings.convergenceHealth, 'failed');
  assert.equal(convergence.warnings.accessHealth, 'healthy');
  assert.equal(convergence.warnings.acquisitionHealth, 'healthy');
  assert.deepEqual(convergence.warnings.displayedAxes, ['convergence']);
  assert.equal(convergence.warnings.convergenceActionIds.length, 1);
  const [actionId] = convergence.warnings.convergenceActionIds;
  assert.ok(convergence.actions.some(
    (action) => action.actionId === actionId && action.rendered && action.eligible,
  ));
  assert.equal(auditDashboardFixture({ fixture: convergence }).passed, true);
});

runTest('inherits W4 durable-handoff URL hygiene and rejects stale readiness', () => {
  const cleanUrls = fixtureSet.baseline.handoffUrls;
  assert.ok(cleanUrls.length > 0);
  assert.ok(cleanUrls.every((value) => {
    const url = new URL(value);
    return url.protocol === 'https:' && /^\/remote-view\/[^/]+$/.test(url.pathname);
  }));
  assert.equal(auditDashboardFixture({ fixture: fixtureSet.baseline }).handoffUrlHygienePassed, true);
  assert.equal(
    reportSet.reports.find((report) => report.fixtureId === 'defect-internal-handoff-url')
      .handoffUrlHygienePassed,
    false,
  );
  assert.ok(reportSet.reports.find((report) => report.fixtureId === 'defect-stale-stream-ready')
    .findings.some((finding) => finding.code === 'stale_stream_ready'));
});

runTest('covers responsive, accessibility, keyboard, focus, modal, overflow, and motion evidence', () => {
  const baselineChecks = fixtureSet.baseline.uiChecks;
  assert.deepEqual(
    sorted(new Set(baselineChecks.map((check) => check.kind))),
    ['accessibility', 'focus', 'keyboard', 'modal', 'overflow', 'reduced_motion', 'viewport'],
  );
  assert.ok(baselineChecks.every((check) => check.state === 'passed'));
  const widths = baselineChecks.map((check) => check.viewportWidth);
  assert.ok(Math.min(...widths) <= 390, 'small viewport is untested');
  assert.ok(widths.some((width) => width >= 768 && width < 1440), 'typical viewport is untested');
  assert.ok(Math.max(...widths) >= 1440, 'wide viewport is untested');
});

runTest('calculates exact timing percentiles and preserves threshold misses', () => {
  assert.deepEqual(calculateTimingDistribution([50, 10, 40, 20, 30], 49), {
    p50Ms: 30,
    p95Ms: 50,
    p99Ms: 50,
    worstMs: 50,
    p95BudgetMs: 49,
    budgetMiss: true,
  });
  const latency = materialized('defect-latency-budget-exceeded');
  const report = auditDashboardFixture({ fixture: latency });
  assert.ok(report.timingDistributions.some((distribution) => distribution.budgetMiss));
  assert.ok(report.findings.some((finding) => finding.code === 'latency_budget_exceeded'));
});

runTest('calculates all frozen resource slopes and detects every upward trend class', () => {
  const slopes = calculateResourceSlopes(fixtureSet.baseline.resourceSamples);
  assert.deepEqual(sorted(Object.keys(slopes)), sorted(reportSchema.$defs.slopes.required));
  assert.ok(Object.values(slopes).every(Number.isFinite));
  const slopeCodes = findingCodes.filter((code) => code.endsWith('_growth_exceeded'));
  for (const code of slopeCodes) {
    const report = reportSet.reports.find((entry) =>
      caseSpec(entry.fixtureId).expectedFindingCodes.includes(code));
    assert.ok(report?.findings.some((finding) => finding.code === code), code);
  }
});

runTest('materializes the exact deterministic dense inventory and audits it clean', () => {
  const options = {
    seed: 158,
    profiles: 100,
    browsers: 500,
    tabs: 2_000,
    jobs: 10_000,
    events: 10_000,
    idNamespace: 'p158-dense-proof',
    labelCardinality: 17,
  };
  const dense = generateDenseDashboardFixture(options);
  assert.deepEqual(generateDenseDashboardFixture(options), dense);
  assertValid(validateMaterializedFixture, dense, 'generated dense dashboard fixture');
  assert.deepEqual(dense.truth.counts, {
    profiles: 100,
    browsers: 500,
    tabs: 2_000,
    jobs: 10_000,
    events: 10_000,
  });
  assert.equal(dense.truth.resources.length, 22_600);
  assert.equal(new Set(dense.truth.resources.map((resource) => resource.resourceId)).size, 22_600);
  assert.equal(dense.railRows.length, 600);
  assert.equal(new Set(dense.railRows.map((row) => row.rowId)).size, 600);
  assert.ok(new Set(dense.truth.resources.map((resource) => resource.label)).size < 22_600);
  assert.equal(auditDashboardFixture({ fixture: dense }).passed, true);
});

process.stdout.write('P158 dashboard oracle adversarial self-test passed\n');
