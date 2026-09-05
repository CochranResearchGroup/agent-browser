#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import { canonicalCandidateDigest } from './lib/p158-campaign-preparation.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import { auditExternalHandoffSession } from './lib/p158-external-handoff-oracle.js';
import {
  P158W6EvidenceAssemblerError,
  assembleP158W6LiveBindings,
  createP158E2AuthenticatedFetch,
  projectP158W6ExternalEvidence,
} from './lib/p158-w6-evidence-assembler.js';
import {
  buildP158AggregateFixtureManifest,
  P158_REQUIRED_LIVE_HOOK_IDS,
} from './lib/p158-evidence-collector.js';

const repoRoot = new URL('..', import.meta.url).pathname;
const registry = JSON.parse(readFileSync(join(
  repoRoot, 'docs/dev/contracts/p158-historical-failure-registry.v1.json',
), 'utf8'));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w6-assembler-test' });
const aggregate = buildP158AggregateFixtureManifest({ repoRoot });
const candidate = {
  sourceCommit: 'ab'.repeat(20), binarySha256: '11'.repeat(32), dashboardSha256: '22'.repeat(32),
  installedGenerationId: 'p158-development-generation', browserExecutableSha256: '33'.repeat(32),
  runtimeManifestRevision: 'runtime-p158', providerConfigurationRevision: 'provider-p158',
  externalIngressDeploymentRevision: 'ingress-p158', preparedAt: '2026-09-04T20:00:00.000Z',
  runId: 'p158-w6-assembler-test', aggregateFixtureManifestSha256: aggregate.sha256,
};
candidate.candidateSha256 = canonicalCandidateDigest(candidate);

const assembly = assembleP158W6LiveBindings({
  schedule, candidate, aggregate, runId: candidate.runId,
  capturedAt: '2026-09-04T20:01:00.000Z',
});
assert.equal(assembly.adapters.length, 54);
assert.equal(assembly.liveHookManifest.adapterBindings.length, 54);
assert.equal(assembly.liveHookManifest.hookBindings.length, 24);
assert.deepEqual(
  assembly.liveHookManifest.hookBindings.map((entry) => entry.hookId).sort(),
  [...P158_REQUIRED_LIVE_HOOK_IDS].sort(),
);
assert.equal(new Set(assembly.adapters.map((entry) => entry.caseId)).size, 54);
assert(assembly.liveHookManifest.adapterBindings.every((entry) => entry.mode === 'explicit_blocked'));
assert(assembly.adapters.every((entry) => entry.liveHookManifestSha256 ===
  assembly.liveHookManifest.manifestSha256));

const fixtureSet = JSON.parse(readFileSync(join(
  repoRoot, 'docs/dev/fixtures/p158/external-handoff-sessions.v1.json',
), 'utf8'));
const clean = fixtureSet.sessions.find((entry) => entry.fixtureId === 'handoff-clean-public-https');
const reports = [0, 1].map((index) => auditExternalHandoffSession({
  session: { ...structuredClone(clean), fixtureId: `${clean.fixtureId}-${index + 1}` },
  options: { auditId: `p158-w6-client-${index + 1}`, auditedAt: '2026-09-04T20:02:00.000Z' },
}));
const receipt = (clientId, ordinal) => ({
  schemaVersion: 'agent-browser.p158-external-calibration-receipt.v1',
  runId: candidate.runId, clientId, success: true, outsideServiceHost: true,
  repairAttempted: false, retryCount: 0, runnerRetryCount: 0,
  outsideServiceNetworkNamespace: true, publicEgressObserved: true,
  runnerIdentity: { provider: 'github_actions', runnerId: `runner-${ordinal}` },
  ingressChecks: clean.ingressChecks.map((entry) => ({ ...entry, state: 'passed' })),
});
const externalReceipts = [receipt('external-human', 1), receipt('external-slow', 2)];
const externalAggregate = {
  schemaVersion: 'agent-browser.p158-external-vantage-aggregate.v1',
  runId: candidate.runId, success: true, clientIds: externalReceipts.map((entry) => entry.clientId).sort(),
  repairAttempted: false, retryCount: 0, runnerRetryCount: 0,
};
const externalProjectionInput = {
  externalAggregate,
  externalReceipts,
  oracleReports: reports,
  serviceHostId: 'p158-development-service-host',
  serviceNetworkNamespaceId: 'p158-development-service-namespace',
  artifactId: 'artifact-13',
};
const projected = projectP158W6ExternalEvidence({
  ...externalProjectionInput,
});
assert.equal(projected.externalVantage.clients.length, 2);
assert.equal(new Set(projected.externalVantage.clients.map((entry) => entry.hostId)).size, 2);
assert(projected.externalVantage.clients.every((entry) => Object.values(entry.ingressObservations)
  .every((observation) => observation.state === 'passed' && observation.artifactId === 'artifact-13')));
assert.equal(projected.externalHandoffOracleReport.passed, true);
assert.equal(projected.externalHandoffOracleReport.summary.ingressCheckCount,
  reports.reduce((count, report) => count + report.summary.ingressCheckCount, 0));
assert.equal(projected.externalHandoffOracleReport.urlClassifications.length,
  reports.reduce((count, report) => count + report.urlClassifications.length, 0));
for (const field of ['repairAttempted', 'retryCount', 'runnerRetryCount']) {
  for (const mutation of ['missing', 'nonzero']) {
    const input = structuredClone(externalProjectionInput);
    if (mutation === 'missing') delete input.externalAggregate[field];
    else input.externalAggregate[field] = field === 'repairAttempted' ? true : 1;
    assert.throws(
      () => projectP158W6ExternalEvidence(input),
      (error) => error instanceof P158W6EvidenceAssemblerError &&
        error.code === 'external_evidence_retry_or_repair',
      `aggregate ${field} ${mutation} must fail closed`,
    );
  }
  for (const receiptIndex of [0, 1]) {
    for (const mutation of ['missing', 'nonzero']) {
      const input = structuredClone(externalProjectionInput);
      if (mutation === 'missing') delete input.externalReceipts[receiptIndex][field];
      else input.externalReceipts[receiptIndex][field] = field === 'repairAttempted' ? true : 1;
      assert.throws(
        () => projectP158W6ExternalEvidence(input),
        (error) => error instanceof P158W6EvidenceAssemblerError &&
          error.code === 'external_evidence_retry_or_repair',
        `receipt ${receiptIndex + 1} ${field} ${mutation} must fail closed`,
      );
    }
  }
}
const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats(ajv);
const preparationSchema = JSON.parse(readFileSync(join(
  repoRoot, 'docs/dev/contracts/p158-campaign-preparation-fixtures.v1.schema.json',
), 'utf8'));
const oracleSchema = JSON.parse(readFileSync(join(
  repoRoot, 'docs/dev/contracts/p158-external-handoff-oracle-report.v1.schema.json',
), 'utf8'));
ajv.addSchema(preparationSchema);
assert.equal(ajv.compile({ $ref: `${preparationSchema.$id}#/$defs/externalVantage` })(
  projected.externalVantage,
), true);
assert.equal(ajv.compile(oracleSchema)(projected.externalHandoffOracleReport), true);

const root = mkdtempSync(join(tmpdir(), 'p158-w6-auth-'));
try {
  const authPath = join(root, 'dashboard-auth.env');
  writeFileSync(authPath, 'P158_DEV_DASHBOARD_USERNAME=operator\nP158_DEV_DASHBOARD_PASSWORD=top-secret\n',
    { mode: 0o600 });
  const calls = [];
  const baseFetch = async (url, init = {}) => {
    calls.push({ url, init: structuredClone(init) });
    if (new URL(url).pathname === '/api/dashboard-auth/login') {
      return { ok: true, status: 200, redirected: false, url,
        headers: { get: (name) => name.toLowerCase() === 'set-cookie' ? 'session=opaque; Path=/; HttpOnly' : null },
        json: async () => ({ authenticated: true }) };
    }
    return { ok: true, status: 200, redirected: false, url,
      headers: { get: () => null }, json: async () => ({ success: true }) };
  };
  const authenticatedFetch = createP158E2AuthenticatedFetch({
    fetch: baseFetch, authEnvPath: authPath,
    dashboardOrigin: 'https://dashboard.p158.test',
    protectedOrigins: ['https://dashboard.p158.test', 'https://service.p158.test'],
  });
  await authenticatedFetch('https://service.p158.test/api/service/status', { method: 'GET' });
  await authenticatedFetch('http://127.0.0.1:19101/api/service/status', { method: 'GET' });
  assert.equal(calls.filter((entry) => new URL(entry.url).pathname === '/api/dashboard-auth/login').length, 1);
  assert.equal(calls[1].init.headers.cookie, 'session=opaque');
  assert.equal(calls[2].init.headers?.cookie, undefined);
  assert.doesNotMatch(JSON.stringify(authenticatedFetch.describe()), /top-secret|operator/u);
  assert.doesNotMatch(JSON.stringify(assembly), /top-secret|operator/u);

  const environmentCalls = [];
  const environmentFetch = createP158E2AuthenticatedFetch({
    fetch: async (url, init = {}) => {
      environmentCalls.push({ url, init: structuredClone(init) });
      if (new URL(url).pathname === '/api/dashboard-auth/login') {
        return { ok: true, status: 200, headers: { get: () => 'session=environment-opaque; HttpOnly' },
          json: async () => ({ authenticated: true }) };
      }
      return { ok: true, status: 200, headers: { get: () => null }, json: async () => ({ success: true }) };
    },
    env: { P158_DEV_DASHBOARD_USERNAME: 'env-operator', P158_DEV_DASHBOARD_PASSWORD: 'env-secret' },
    dashboardOrigin: 'https://dashboard.p158.test',
    protectedOrigins: ['https://dashboard.p158.test'],
  });
  await environmentFetch('https://dashboard.p158.test/api/service/status');
  assert.equal(environmentCalls[1].init.headers.cookie, 'session=environment-opaque');
  assert.doesNotMatch(JSON.stringify(environmentFetch.describe()), /env-secret|env-operator|environment-opaque/u);
} finally {
  rmSync(root, { recursive: true, force: true });
}

process.stdout.write('P158 W6 evidence assembler provider-free self-test passed\n');
