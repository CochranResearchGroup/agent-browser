#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import {
  createCampaignController,
  createMemoryArtifactStore,
  sha256,
} from './lib/p158-campaign-controller.js';
import {
  P158_PREPARATION_FINDING_CODES,
  canonicalCalibrationDigest,
  canonicalCandidateDigest,
  canonicalEnvironmentDigest,
  canonicalEnvironmentSealDigest,
  prepareAndFreezeCampaign,
} from './lib/p158-campaign-preparation.js';

const root = new URL('..', import.meta.url).pathname;

function readJson(relativePath) {
  return JSON.parse(readFileSync(join(root, relativePath), 'utf8'));
}

const registry = readJson('docs/dev/contracts/p158-historical-failure-registry.v1.json');
const manifestSchema = readJson('docs/dev/contracts/p158-campaign-manifest.v1.schema.json');
const freezeSchema = readJson('docs/dev/contracts/p158-campaign-freeze.v1.schema.json');
const reportSchema = readJson('docs/dev/contracts/p158-campaign-preparation-report.v1.schema.json');
const fixtureSchema = readJson('docs/dev/contracts/p158-campaign-preparation-fixtures.v1.schema.json');
const fixtureSet = readJson('docs/dev/fixtures/p158/campaign-preparation.v1.json');
const w4ReportSchema = readJson('docs/dev/contracts/p158-external-handoff-oracle-report.v1.schema.json');

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
const validateManifest = ajv.compile(manifestSchema);
const validateFreeze = ajv.compile(freezeSchema);
const validateReport = ajv.compile(reportSchema);
ajv.addSchema(fixtureSchema);
const validateFixtureSet = ajv.getSchema(fixtureSchema.$id);
const validateFixtureInput = ajv.compile({ $ref: `${fixtureSchema.$id}#/$defs/input` });
const validateW4Report = ajv.compile(w4ReportSchema);

const REQUIRED_ARTIFACT_KINDS = fixtureSchema.$defs.artifactKind.enum;
const REQUIRED_INGRESS_CLASSES = fixtureSchema.$defs.ingressObservations.required;
const EXPECTED_FREEZE_AT = '2026-09-02T20:00:20.000Z';
const EXPECTED_MONOTONIC_TIME = 1_580_000;

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

async function runTest(name, body) {
  try {
    await body();
    process.stdout.write(`PASS ${name}\n`);
  } catch (error) {
    error.message = `${name}: ${error.message}`;
    throw error;
  }
}

function makeClock() {
  return {
    wallNow: () => EXPECTED_FREEZE_AT,
    monotonicNow: () => EXPECTED_MONOTONIC_TIME,
  };
}

function fixtureById(fixtureId) {
  const fixture = fixtureSet.fixtures.find((entry) => entry.fixtureId === fixtureId);
  assert.ok(fixture, `preparation corpus omitted ${fixtureId}`);
  return fixture;
}

function makeContext(fixture) {
  const fixtureInput = clone(fixture.input);
  const store = createMemoryArtifactStore();
  const clock = makeClock();
  const pristineController = createCampaignController({
    registry,
    runId: fixtureInput.candidate.runId,
    seed: 'p158-w6-fixed-seed',
    store,
    clock,
  });
  const controller = fixture.controllerSetup === 'pristine'
    ? pristineController
    : {
        snapshot: () => ({
          state: 'prepared',
          prepared: true,
          counts: { terminal: 0 },
          results: [],
        }),
      };
  const input = {
    ...fixtureInput,
    clock,
    controller,
  };
  return { input, fixtureInput, controller: pristineController, store };
}

function bytesForArtifact(artifact) {
  return artifact.contentEncoding === 'base64'
    ? Buffer.from(artifact.content, 'base64')
    : Buffer.from(artifact.content, 'utf8');
}

function withoutStorageFields(freezeReceipt) {
  const { sha256: _sha256, byteCount: _byteCount, ...persisted } = freezeReceipt;
  return persisted;
}

runTest('accepts the strict closed-world preparation corpus', async () => {
  assertValid(validateFixtureSet, fixtureSet, 'campaign-preparation.v1.json');
  assert.deepEqual(
    sorted(P158_PREPARATION_FINDING_CODES),
    sorted(reportSchema.$defs.findingCode.enum),
  );
  assert.deepEqual(
    sorted(new Set(fixtureSet.fixtures.flatMap((fixture) => fixture.expectedFindingCodes))),
    sorted(P158_PREPARATION_FINDING_CODES.filter(
      (code) => code !== 'execution_schedule_mismatch',
    )),
  );
  assert.deepEqual(sorted(REQUIRED_INGRESS_CLASSES), [
    'cookie', 'dns', 'form_action', 'iframe', 'reconnect', 'redirect', 'tls', 'websocket',
  ]);
});

for (const fixture of fixtureSet.fixtures) {
  await runTest(`classifies ${fixture.fixtureId} without preflight effects`, async () => {
    assertValid(validateFixtureInput, fixture.input, fixture.fixtureId);
    const first = makeContext(fixture);
    const report = await prepareAndFreezeCampaign(first.input);
    assertValid(validateReport, report, `${fixture.fixtureId} report`);
    assert.deepEqual(
      sorted(new Set(report.findings.map((finding) => finding.code))),
      sorted(fixture.expectedFindingCodes),
    );
    assert.equal(report.passed, fixture.expectedFindingCodes.length === 0);
    assert.equal(report.effectsAttempted, false);
    assert.equal(report.repairAttempted, false);
    assert.deepEqual(
      Object.fromEntries(Object.entries(first.input).filter(([key]) => !['controller', 'clock'].includes(key))),
      first.fixtureInput,
      `${fixture.fixtureId} input was mutated`,
    );
    if (!report.passed) {
      assert.deepEqual(first.store.paths(), [], `${fixture.fixtureId} wrote evidence before passing preflight`);
      assert.equal(first.controller.snapshot().prepared, false);
    }
    const second = makeContext(fixture);
    assert.deepEqual(await prepareAndFreezeCampaign(second.input), report, `${fixture.fixtureId} is nondeterministic`);
  });
}

await runTest('persists schema-valid canonical seals and actual artifact bytes before freeze', async () => {
  const fixture = fixtureById('clean-freeze-ready');
  const context = makeContext(fixture);
  const report = await prepareAndFreezeCampaign(context.input);
  assertValid(validateReport, report, 'clean preparation report');
  assert.equal(report.passed, true);
  assert.deepEqual(report.findings, []);

  const snapshot = context.controller.snapshot();
  assert.equal(snapshot.state, 'frozen');
  assert.equal(snapshot.counts.terminal, 0);
  assert.deepEqual(snapshot.results, []);
  assert.equal(report.zeroStartedCaseCount, 0);
  assert.equal(report.zeroStartedAttemptCount, 0);

  const manifest = JSON.parse(await context.store.read('campaign-manifest.json'));
  const persistedFreezeBytes = await context.store.read('campaign-freeze.json');
  const persistedFreeze = JSON.parse(persistedFreezeBytes);
  assertValid(validateManifest, manifest, 'persisted campaign manifest');
  assertValid(validateFreeze, persistedFreeze, 'persisted campaign freeze receipt');
  assert.deepEqual(persistedFreeze, withoutStorageFields(report.freezeReceipt));
  assert.equal(report.freezeReceipt.sha256, sha256(persistedFreezeBytes));
  assert.equal(report.freezeReceipt.byteCount, persistedFreezeBytes.byteLength);
  assert.equal(report.freezeReceipt.frozenAt, EXPECTED_FREEZE_AT);
  assert.equal(report.freezeReceipt.monotonicTimeNanoseconds, EXPECTED_MONOTONIC_TIME);
  assert.ok(Date.parse(context.input.calibration.completedAt) < Date.parse(report.freezeReceipt.frozenAt));
  assert.ok(context.input.environments.every(
    (environment) => Date.parse(environment.sealedAt) < Date.parse(report.freezeReceipt.frozenAt),
  ));

  assert.deepEqual(manifest.artifactBindings, report.artifactBindings);
  assert.equal(report.artifactBindings.length, context.input.artifacts.length);
  for (const binding of report.artifactBindings) {
    const inputArtifact = context.input.artifacts.find((artifact) => artifact.artifactId === binding.artifactId);
    const storageReceipt = snapshot.evidence.artifacts.find(
      (artifact) => artifact.artifactId === binding.artifactId,
    );
    assert.ok(storageReceipt, `${binding.artifactId} has no storage receipt`);
    const expectedBytes = bytesForArtifact(inputArtifact);
    const persistedBytes = await context.store.read(storageReceipt.relativePath);
    assert.deepEqual(persistedBytes, expectedBytes, `${binding.artifactId} content changed`);
    assert.equal(binding.sha256, sha256(expectedBytes), `${binding.artifactId} hash disagrees`);
    assert.equal(binding.byteCount, expectedBytes.byteLength, `${binding.artifactId} byte count disagrees`);
  }

  assert.equal(report.candidateSha256, canonicalCandidateDigest(context.input.candidate));
  assert.equal(report.candidateSha256, context.input.candidate.candidateSha256);
  assert.equal(report.calibrationSha256, canonicalCalibrationDigest(context.input.calibration));
  for (const seal of report.environmentSeals) {
    const environment = context.input.environments.find((entry) => entry.environmentId === seal.environmentId);
    assert.equal(seal.identitySha256, canonicalEnvironmentDigest(environment));
    assert.equal(seal.sealSha256, canonicalEnvironmentSealDigest(seal));
  }
  assert.deepEqual(sorted(report.environmentSeals.map((seal) => seal.environmentId)), ['E1', 'E2']);
  const e2Seal = report.environmentSeals.find((seal) => seal.environmentId === 'E2');
  const externalClientIds = context.input.externalVantage.clients.map((client) => client.clientId);
  assert.deepEqual(e2Seal.externalClientIds, sorted(externalClientIds));
  assert.equal(new Set(externalClientIds).size, 2);
  assert.ok(context.input.externalVantage.clients.every((client) =>
    client.outsideServiceHost && client.outsideServiceNetworkNamespace && client.publicEgressObserved));
  assert.ok(context.input.externalVantage.clients.every((client) =>
    REQUIRED_INGRESS_CLASSES.every((evidenceClass) =>
      client.ingressObservations[evidenceClass].state === 'passed')));
  assertValid(validateW4Report, context.input.w4Report, 'sealed W4 report');
  assert.equal(context.input.w4Report.passed, true);
  assert.deepEqual(context.input.w4Report.findings, []);

  const recordTypes = snapshot.evidence.events.map((event) => event.recordType);
  assert.ok(!recordTypes.includes('attempt_started'));
  assert.ok(!recordTypes.includes('attempt_terminal'));
  assert.ok(!JSON.stringify(report).includes('synthetic installation_receipt'));
});

await runTest('rejects calibration that completes after the actual freeze instant', async () => {
  const fixture = clone(fixtureById('clean-freeze-ready'));
  fixture.fixtureId = 'calibration-after-freeze';
  fixture.input.calibration.startedAt = '2026-09-02T19:50:00.000Z';
  fixture.input.calibration.completedAt = '2026-09-02T20:10:00.000Z';
  fixture.input.calibration.declaredSha256 = canonicalCalibrationDigest(fixture.input.calibration);
  const context = makeContext(fixture);
  const report = await prepareAndFreezeCampaign(context.input);
  assert.equal(report.passed, false);
  assert.ok(report.findings.some((finding) => finding.code === 'invalid_calibration'));
  assert.deepEqual(context.store.paths(), []);
});

await runTest('accepts actual Uint8Array artifact bytes without weakening hash agreement', async () => {
  const fixture = clone(fixtureById('clean-freeze-ready'));
  fixture.fixtureId = 'clean-byte-artifact';
  const target = fixture.input.artifacts.find((artifact) => artifact.contentEncoding === 'base64');
  const bytes = Buffer.from(target.content, 'base64');
  target.content = new Uint8Array(bytes);
  delete target.contentEncoding;
  const context = makeContext(fixture);
  const report = await prepareAndFreezeCampaign(context.input);
  assert.equal(report.passed, true);
  const binding = report.artifactBindings.find((entry) => entry.artifactId === target.artifactId);
  const receipt = context.controller.snapshot().evidence.artifacts.find(
    (entry) => entry.artifactId === target.artifactId,
  );
  assert.deepEqual(await context.store.read(receipt.relativePath), bytes);
  assert.equal(binding.sha256, sha256(bytes));
});

process.stdout.write('P158 campaign preparation adversarial self-test passed\n');
