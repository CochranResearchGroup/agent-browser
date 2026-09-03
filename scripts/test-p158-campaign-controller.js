#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import {
  CampaignControllerError,
  createCampaignController,
  sha256 as campaignSha256,
} from './lib/p158-campaign-controller.js';

const root = new URL('..', import.meta.url).pathname;
const registryPath = join(root, 'docs/dev/contracts/p158-historical-failure-registry.v1.json');
const registryBytes = readFileSync(registryPath);
const registry = JSON.parse(registryBytes);
const manifestSchema = JSON.parse(
  readFileSync(join(root, 'docs/dev/contracts/p158-campaign-manifest.v1.schema.json'), 'utf8'),
);
const resultSchema = JSON.parse(
  readFileSync(join(root, 'docs/dev/contracts/p158-campaign-result.v1.schema.json'), 'utf8'),
);
const freezeSchema = JSON.parse(
  readFileSync(join(root, 'docs/dev/contracts/p158-campaign-freeze.v1.schema.json'), 'utf8'),
);
const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
const validateManifest = ajv.compile(manifestSchema);
const validateResult = ajv.compile(resultSchema);
const validateFreeze = ajv.compile(freezeSchema);

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function candidate(runId) {
  const value = {
    runId,
    sourceCommit: 'e26a6b05c315cfed06a833a5c4d7406803bcc0fb',
    binarySha256: '11'.repeat(32),
    dashboardSha256: '22'.repeat(32),
    installedGenerationId: 'development-generation-p158',
    browserExecutableSha256: '33'.repeat(32),
    runtimeManifestRevision: 'runtime-manifest-p158-v1',
    providerConfigurationRevision: 'provider-configuration-p158-v1',
    externalIngressDeploymentRevision: 'external-ingress-p158-v1',
    aggregateFixtureManifestSha256: '44'.repeat(32),
    preparedAt: '2026-09-02T20:00:00.000Z',
  };
  value.candidateSha256 = campaignSha256(value);
  return value;
}

const ARTIFACT_KINDS = [
  'installation_receipt',
  'runtime_status_receipt',
  'runtime_doctor_receipt',
  'runtime_manifest',
  'provider_plan_receipt',
  'provider_stage_receipt',
  'provider_preflight_receipt',
  'provider_apply_receipt',
  'provider_status_receipt',
  'provider_doctor_receipt',
  'provider_configuration',
  'external_ingress_deployment',
  'external_vantage',
  'external_handoff_oracle_report',
  'calibration_raw',
  'calibration_summary',
  'calibration_budget',
  'fixture_aggregate_manifest',
];

function freezePrerequisites(runId) {
  const artifactBindings = ARTIFACT_KINDS.map((kind, index) => ({
    artifactId: `artifact-${String(index + 1).padStart(2, '0')}`,
    kind,
    relativePath: `freeze/${kind}.json`,
    sha256: String((index % 9) + 1).repeat(64),
    byteCount: index + 1,
    capturedAt: '2026-09-02T20:00:00.000Z',
  }));
  const artifactId = (kind) => artifactBindings.find((artifact) => artifact.kind === kind).artifactId;
  return {
    artifactBindings,
    environmentSeals: [
      {
        environmentId: 'E1',
        identityId: 'development-runtime-e1',
        identitySha256: 'aa'.repeat(32),
        sealSha256: 'ab'.repeat(32),
        sealedAt: '2026-09-02T20:00:20.000Z',
        receiptArtifactIds: [
          artifactId('installation_receipt'),
          artifactId('runtime_status_receipt'),
          artifactId('runtime_doctor_receipt'),
          artifactId('runtime_manifest'),
        ],
      },
      {
        environmentId: 'E2',
        identityId: 'external-presentation-e2',
        identitySha256: 'ba'.repeat(32),
        sealSha256: 'bb'.repeat(32),
        sealedAt: '2026-09-02T20:00:30.000Z',
        receiptArtifactIds: [
          artifactId('provider_configuration'),
          artifactId('external_ingress_deployment'),
          artifactId('external_vantage'),
          artifactId('external_handoff_oracle_report'),
        ],
        externalClientIds: ['external-runner-a', 'external-runner-b'],
        externalVantageArtifactId: artifactId('external_vantage'),
        externalHandoffOracleArtifactId: artifactId('external_handoff_oracle_report'),
      },
    ],
    calibration: {
      calibrationId: 'p158-calibration-c01',
      environmentIds: ['E1', 'E2'],
      startedAt: '2026-09-02T19:40:00.000Z',
      completedAt: '2026-09-02T20:00:00.000Z',
      clean: true,
      workload: {
        durationMinutes: 20,
        agentClients: 25,
        externalViewers: 2,
        controllers: 1,
        serviceCommands: 500,
        dashboardActions: 50,
        handoffReconnects: 10,
      },
      rawArtifactId: artifactId('calibration_raw'),
      rawArtifactSha256: artifactBindings.find((entry) => entry.kind === 'calibration_raw').sha256,
      summaryArtifactId: artifactId('calibration_summary'),
      summaryArtifactSha256: artifactBindings.find((entry) => entry.kind === 'calibration_summary').sha256,
      budgetArtifactId: artifactId('calibration_budget'),
      budgetSha256: artifactBindings.find((entry) => entry.kind === 'calibration_budget').sha256,
      environmentRelativeBudgets: { agentCommandP95Ms: 750, handoffPixelsP95Ms: 8000 },
    },
    fixtureSeal: {
      aggregateArtifactId: artifactId('fixture_aggregate_manifest'),
      aggregateSha256: '44'.repeat(32),
      entryCount: 9,
      includedPaths: [
        'docs/dev/contracts/p158-historical-failure-registry.v1.json',
        'docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json',
      ],
    },
    freezeContract: {
      freezeId: `${runId}:freeze`,
      requiredStartedCaseCount: 0,
      requiredStartedAttemptCount: 0,
    },
  };
}

function basicSchedule() {
  return [
    { caseId: 'A01', attemptId: 'A01-001', environmentId: 'E0', dependsOn: [] },
    { caseId: 'A02', attemptId: 'A02-001', environmentId: 'E0', dependsOn: ['A01'] },
    { caseId: 'D01', attemptId: 'D01-001', environmentId: 'E0', dependsOn: [] },
  ];
}

function createFixture(
  label,
  { seed = 'p158-fixed-seed', schedule = basicSchedule(), runId = `p158-${label}` } = {},
) {
  const fixtureRoot = mkdtempSync(join(tmpdir(), `agent-browser-p158-controller-${label}-`));
  const runRoot = join(fixtureRoot, 'run');
  const controller = createCampaignController({ runRoot, registry, runId, seed });
  return {
    controller,
    fixtureRoot,
    runRoot,
    runId,
    prepare: () => controller.prepare({
      candidate: candidate(runId),
      schedule,
      scheduledTeardown: { caseId: 'TEARDOWN', environmentId: 'E0' },
      ...freezePrerequisites(runId),
    }),
    cleanup: () => rmSync(fixtureRoot, { recursive: true, force: true }),
  };
}

async function expectControllerError(action, code) {
  await assert.rejects(action, (error) => {
    assert.ok(error instanceof CampaignControllerError, `expected CampaignControllerError, received ${error}`);
    assert.equal(error.code, code);
    return true;
  });
}

function stableSchedule(snapshot) {
  return snapshot.schedule.map(({ caseId, attemptId, environmentId, dependsOn, seed }) => ({
    caseId,
    attemptId,
    environmentId,
    dependsOn,
    seed,
  }));
}

function assertValid(validate, value, label) {
  assert.equal(
    validate(value),
    true,
    `${label} violates its JSON Schema: ${ajv.errorsText(validate.errors, { separator: '; ' })}`,
  );
}

function validatePersistedCampaign(runRoot) {
  const manifest = JSON.parse(readFileSync(join(runRoot, 'campaign-manifest.json'), 'utf8'));
  assertValid(validateManifest, manifest, 'campaign-manifest.json');

  const freezePath = join(runRoot, 'campaign-freeze.json');
  const freeze = existsSync(freezePath) ? JSON.parse(readFileSync(freezePath, 'utf8')) : null;
  if (freeze) assertValid(validateFreeze, freeze, 'campaign-freeze.json');

  const ledgerRoot = join(runRoot, 'ledger');
  const ledgerFiles = readdirSync(ledgerRoot).sort();
  const ledger = [];
  assert.ok(ledgerFiles.length > 0, 'campaign ledger is empty');
  let previousBytes = null;
  for (const [index, filename] of ledgerFiles.entries()) {
    const bytes = readFileSync(join(ledgerRoot, filename));
    const record = JSON.parse(bytes);
    assertValid(validateResult, record, filename);
    assert.equal(record.sequence, index, `${filename} sequence is not contiguous`);
    assert.equal(
      record.previousRecordSha256,
      previousBytes === null ? null : sha256(previousBytes),
      `${filename} does not hash-link to its predecessor`,
    );
    ledger.push({ filename, record });
    previousBytes = bytes;
  }
  return { manifest, freeze, ledger, ledgerFiles };
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

await runTest('rejects overwrite of an existing campaign root', async () => {
  const fixture = createFixture('overwrite');
  try {
    await fixture.prepare();
    const before = fixture.controller.snapshot();
    const second = createCampaignController({
      runRoot: fixture.runRoot,
      registry,
      runId: fixture.runId,
      seed: 'different-seed',
    });
    await expectControllerError(
      () => second.prepare({
        candidate: candidate(fixture.runId),
        schedule: basicSchedule(),
        scheduledTeardown: { caseId: 'TEARDOWN', environmentId: 'E0' },
        ...freezePrerequisites(fixture.runId),
      }),
      'campaign_root_exists',
    );
    assert.deepEqual(fixture.controller.snapshot(), before, 'overwrite rejection changed the first run');
  } finally {
    fixture.cleanup();
  }
});

await runTest('derives reproducible seeds and ordering', async () => {
  const deterministicOptions = { seed: 'repeatable-master-seed', runId: 'p158-deterministic-run' };
  const first = createFixture('determinism-a', deterministicOptions);
  const second = createFixture('determinism-b', deterministicOptions);
  try {
    await first.prepare();
    await second.prepare();
    const firstSchedule = stableSchedule(first.controller.snapshot());
    const secondSchedule = stableSchedule(second.controller.snapshot());
    assert.deepEqual(firstSchedule, secondSchedule);
    assert.equal(new Set(firstSchedule.map((attempt) => attempt.seed)).size, firstSchedule.length);
  } finally {
    first.cleanup();
    second.cleanup();
  }
});

await runTest('requires external ingress for every E2 operator-visible and combined attempt', async () => {
  const fixture = createFixture('external-ingress-classification', {
    schedule: [
      { caseId: 'A15', attemptId: 'A15-E2', environmentId: 'E2', dependsOn: [] },
      { caseId: 'H01', attemptId: 'H01-E2', environmentId: 'E2', dependsOn: [] },
      { caseId: 'X06', attemptId: 'X06-E2', environmentId: 'E2', dependsOn: [] },
      { caseId: 'X10', attemptId: 'X10-E2', environmentId: 'E2', dependsOn: [] },
      { caseId: 'D01', attemptId: 'D01-E2', environmentId: 'E2', dependsOn: [] },
      { caseId: 'C01', attemptId: 'C01-E2', environmentId: 'E2', dependsOn: [] },
    ],
  });
  try {
    await fixture.prepare();
    const byCase = Object.fromEntries(
      fixture.controller.snapshot().manifest.schedule.map((attempt) => [attempt.caseId, attempt]),
    );
    for (const caseId of ['A15', 'H01', 'X06', 'X10', 'D01', 'C01']) {
      assert.equal(byCase[caseId].externalIngressRequired, true, `${caseId} omitted its E2 ingress gate`);
    }
  } finally {
    fixture.cleanup();
  }
});

await runTest('writes an immutable actual freeze receipt with zero started work', async () => {
  const fixture = createFixture('freeze-receipt');
  try {
    await fixture.prepare();
    const prepared = fixture.controller.snapshot();
    assert.equal(existsSync(join(fixture.runRoot, 'campaign-freeze.json')), false);
    await fixture.controller.freeze();
    const snapshot = fixture.controller.snapshot();
    const persisted = validatePersistedCampaign(fixture.runRoot);
    assert.ok(persisted.freeze, 'freeze receipt was not persisted');
    assert.equal(persisted.freeze.freezeId, prepared.manifest.freezeContract.freezeId);
    assert.equal(persisted.freeze.manifestSha256, prepared.manifestSha256);
    assert.equal(persisted.freeze.candidateSha256, prepared.candidate.candidateSha256);
    assert.equal(
      persisted.freeze.environmentSealsSha256,
      campaignSha256(prepared.manifest.environmentSeals),
    );
    assert.equal(persisted.freeze.startedCaseCount, 0);
    assert.equal(persisted.freeze.startedAttemptCount, 0);
    const frozenTransition = persisted.ledger
      .map((entry) => entry.record)
      .find((record) => record.controllerState === 'frozen');
    assert.equal(frozenTransition.wallTime, persisted.freeze.frozenAt);
    assert.equal(snapshot.freezeReceipt.sha256, sha256(readFileSync(join(fixture.runRoot, 'campaign-freeze.json'))));
  } finally {
    fixture.cleanup();
  }
});

await runTest('preserves a first failure without an opportunistic retry', async () => {
  const fixture = createFixture('no-retry', {
    schedule: [{ caseId: 'A01', attemptId: 'A01-001', environmentId: 'E0', dependsOn: [] }],
  });
  try {
    await fixture.prepare();
    await fixture.controller.freeze();
    await fixture.controller.startExecution();
    await fixture.controller.recordAttempt({
      caseId: 'A01',
      attemptId: 'A01-001',
      resultState: 'reproduced_historical_failure',
      evidence: { signature: 'existing_session_profile_identity_unproven' },
    });
    await expectControllerError(
      () => fixture.controller.recordAttempt({
        caseId: 'A01',
        attemptId: 'A01-001',
        resultState: 'passed',
      }),
      'attempt_already_terminal',
    );
    await expectControllerError(
      () => fixture.controller.recordAttempt({
        caseId: 'A01',
        attemptId: 'A01-002',
        resultState: 'passed',
      }),
      'unscheduled_attempt',
    );
    const snapshot = fixture.controller.snapshot();
    assert.equal(snapshot.schedule.length, 1, 'the controller invented an undeclared attempt');
    assert.equal(snapshot.results[0].resultState, 'reproduced_historical_failure');
  } finally {
    fixture.cleanup();
  }
});

await runTest('blocks dependents while independent cases continue', async () => {
  const fixture = createFixture('blocked-propagation');
  try {
    await fixture.prepare();
    await fixture.controller.freeze();
    await fixture.controller.startExecution();
    await fixture.controller.recordAttempt({
      caseId: 'A01',
      attemptId: 'A01-001',
      resultState: 'harness_failure',
      blocksDependents: true,
      stateObservation: { fixtureReady: false },
      evidence: { signature: 'fixture_precondition_lost' },
    });
    const snapshot = fixture.controller.snapshot();
    const dependent = snapshot.results.find((attempt) => attempt.attemptId === 'A02-001');
    const independent = snapshot.results.find((attempt) => attempt.attemptId === 'D01-001');
    assert.equal(dependent.resultState, 'skipped_blocked');
    assert.deepEqual(dependent.blockedBy, { caseId: 'A01', attemptId: 'A01-001' });
    assert.equal(independent, undefined);
  } finally {
    fixture.cleanup();
  }
});

await runTest('does not block a usable prerequisite after an observed failure', async () => {
  const fixture = createFixture('nonblocking-failure');
  try {
    await fixture.prepare();
    await fixture.controller.freeze();
    await fixture.controller.startExecution();
    await fixture.controller.recordAttempt({
      caseId: 'A01',
      attemptId: 'A01-001',
      resultState: 'reproduced_historical_failure',
      evidence: { signature: 'historical_failure_with_prerequisite_still_available' },
    });
    await fixture.controller.recordAttempt({
      caseId: 'A02',
      attemptId: 'A02-001',
      resultState: 'passed',
    });
    assert.equal(
      fixture.controller.snapshot().results.find((result) => result.attemptId === 'A02-001').resultState,
      'passed',
    );
  } finally {
    fixture.cleanup();
  }
});

await runTest('rejects non-monotonic controller transitions', async () => {
  const fixture = createFixture('monotonic');
  try {
    await fixture.prepare();
    await fixture.controller.freeze();
    await fixture.controller.startExecution();
    await expectControllerError(() => fixture.controller.freeze(), 'invalid_state_transition');
    await expectControllerError(() => fixture.prepare(), 'invalid_state_transition');
    assert.equal(fixture.controller.snapshot().state, 'executing');
  } finally {
    fixture.cleanup();
  }
});

await runTest('rejects candidate and schedule mutation after freeze', async () => {
  const schedule = basicSchedule();
  const fixture = createFixture('freeze-mutation', { schedule });
  try {
    await fixture.prepare();
    await fixture.controller.freeze();
    schedule.push({ caseId: 'D02', attemptId: 'D02-001', environmentId: 'E0', dependsOn: ['D01'] });
    await expectControllerError(() => fixture.controller.startExecution(), 'frozen_input_mutated');
    assert.equal(fixture.controller.snapshot().state, 'frozen');
  } finally {
    fixture.cleanup();
  }
});

await runTest('writes artifacts atomically and seals a hash-verifiable manifest', async () => {
  const fixture = createFixture('artifact-integrity');
  try {
    await fixture.prepare();
    await fixture.controller.freeze();
    await fixture.controller.startExecution();
    const content = Buffer.from('synthetic p158 artifact\n');
    const artifact = await fixture.controller.writeArtifact({
      artifactId: 'artifact-001',
      relativePath: 'A01/attempt-001.json',
      content,
    });
    assert.equal(artifact.sha256, sha256(content));
    assert.equal(artifact.byteCount, content.byteLength);
    const artifactPath = join(fixture.runRoot, artifact.relativePath);
    assert.equal(existsSync(artifactPath), true);
    assert.equal(sha256(readFileSync(artifactPath)), artifact.sha256);
    const temporaryFiles = readdirSync(join(fixture.runRoot, 'artifacts', 'A01'))
      .filter((name) => name.includes('.tmp'));
    assert.deepEqual(temporaryFiles, [], 'atomic write residue remained visible');
    await expectControllerError(
      () => fixture.controller.writeArtifact({
        artifactId: 'artifact-002',
        relativePath: 'A01/attempt-001.json',
        content: 'overwrite attempt\n',
      }),
      'artifact_already_exists',
    );
    await expectControllerError(
      () => fixture.controller.recordAttempt({
        caseId: 'A01',
        attemptId: 'A01-001',
        resultState: 'passed',
        evidence: { artifactIds: ['artifact-does-not-exist'] },
      }),
      'unknown_evidence_artifact',
    );

    for (const attempt of fixture.controller.snapshot().schedule) {
      if (fixture.controller.snapshot().results.some((result) => result.attemptId === attempt.attemptId)) continue;
      await fixture.controller.recordAttempt({
        caseId: attempt.caseId,
        attemptId: attempt.attemptId,
        resultState: 'passed',
      });
    }
    await fixture.controller.recordScheduledTeardown({ resultState: 'passed' });
    await fixture.controller.finishExecution();
    await fixture.controller.sealEvidence();
    const sealed = fixture.controller.snapshot().seal;
    const sealedPath = join(fixture.runRoot, sealed.relativePath);
    assert.equal(sha256(readFileSync(sealedPath)), sealed.sha256);
    const manifest = JSON.parse(readFileSync(sealedPath, 'utf8'));
    assert.equal(manifest.runId, fixture.runId);
    assert.ok(manifest.artifacts.some((entry) => entry.artifactId === 'artifact-001'));
    const persisted = validatePersistedCampaign(fixture.runRoot);
    assert.equal(persisted.manifest.schedule.length, fixture.controller.snapshot().schedule.length);
    assert.ok(persisted.ledgerFiles.length >= 8, 'the sealed run omitted expected causal records');
  } finally {
    fixture.cleanup();
  }
});

await runTest('terminalizes an affected environment after consecutive safety violations', async () => {
  const fixture = createFixture('safety-stop', {
    schedule: [
      { caseId: 'A01', attemptId: 'A01-001', environmentId: 'E0', dependsOn: [] },
      { caseId: 'D01', attemptId: 'D01-001', environmentId: 'E2', dependsOn: [] },
    ],
  });
  try {
    await fixture.prepare();
    await fixture.controller.freeze();
    await fixture.controller.startExecution();
    const violation = {
      environmentId: 'E0',
      availableMemoryBytes: registry.resourceCeilings.minimumAvailableMemoryBytes - 1,
      availableMemoryPlusFreeSwapBytes:
        registry.resourceCeilings.minimumAvailableMemoryPlusFreeSwapBytes - 1,
      filesystemUsedPercent: 1,
      campaignProcessCount: 1,
      chromeProcessCount: 0,
      xvfbProcessCount: 0,
      allocatedDisplayCount: 0,
      allocatedRouteCount: 0,
      externalConnectionCount: 0,
      unresolvedJobCount: 0,
    };
    await fixture.controller.observeSafety(violation);
    assert.equal(fixture.controller.snapshot().results.length, 0);
    await fixture.controller.observeSafety(violation);
    const snapshot = fixture.controller.snapshot();
    assert.equal(
      snapshot.results.find((attempt) => attempt.attemptId === 'A01-001').resultState,
      'safety_stopped',
    );
    assert.equal(
      snapshot.results.find((attempt) => attempt.attemptId === 'D01-001'),
      undefined,
      'a disjoint environment was stopped',
    );
    assert.equal(snapshot.safety.E0.stopped, true);
  } finally {
    fixture.cleanup();
  }
});

await runTest('preserves scheduled teardown failure as evidence', async () => {
  const fixture = createFixture('teardown', {
    schedule: [{ caseId: 'A01', attemptId: 'A01-001', environmentId: 'E0', dependsOn: [] }],
  });
  try {
    await fixture.prepare();
    await fixture.controller.freeze();
    await fixture.controller.startExecution();
    await fixture.controller.recordAttempt({
      caseId: 'A01', attemptId: 'A01-001', resultState: 'passed',
    });
    await expectControllerError(() => fixture.controller.finishExecution(), 'execution_not_terminal');
    await fixture.controller.recordScheduledTeardown({
      resultState: 'harness_failure',
      evidence: { signature: 'disposable_teardown_failed' },
    });
    await fixture.controller.finishExecution();
    const snapshot = fixture.controller.snapshot();
    assert.equal(snapshot.state, 'execution_terminal');
    assert.equal(
      snapshot.results.find((result) => result.scheduledTeardown).resultState,
      'harness_failure',
    );
    await expectControllerError(
      () => fixture.controller.recordScheduledTeardown({ resultState: 'passed' }),
      'invalid_state_transition',
    );
  } finally {
    fixture.cleanup();
  }
});

await runTest('requires exact terminal-count closure before execution completes', async () => {
  const fixture = createFixture('terminal-closure', {
    schedule: [
      { caseId: 'A01', attemptId: 'A01-001', environmentId: 'E0', dependsOn: [] },
      { caseId: 'D01', attemptId: 'D01-001', environmentId: 'E0', dependsOn: [] },
    ],
  });
  try {
    await fixture.prepare();
    await fixture.controller.freeze();
    await fixture.controller.startExecution();
    await fixture.controller.recordAttempt({
      caseId: 'A01', attemptId: 'A01-001', resultState: 'passed',
    });
    await expectControllerError(() => fixture.controller.finishExecution(), 'execution_not_terminal');
    await expectControllerError(
      () => fixture.controller.recordScheduledTeardown({ resultState: 'passed' }),
      'teardown_not_scheduled_yet',
    );
    await fixture.controller.recordAttempt({
      caseId: 'D01', attemptId: 'D01-001', resultState: 'new_product_failure',
    });
    await fixture.controller.recordScheduledTeardown({ resultState: 'passed' });
    await fixture.controller.finishExecution();
    const snapshot = fixture.controller.snapshot();
    assert.equal(snapshot.state, 'execution_terminal');
    const caseResults = snapshot.results.filter((result) => !result.scheduledTeardown);
    assert.equal(snapshot.schedule.length, 2);
    assert.equal(caseResults.length, 2);
    assert.equal(
      new Set(caseResults.map((result) => result.attemptId)).size,
      2,
    );
    const transition = snapshot.evidence.events.at(-1);
    assert.equal(transition.type, 'controller-state-transition');
    assert.equal(
      Object.values(transition.payload.resultCounts).reduce((sum, count) => sum + count, 0),
      3,
      'terminal counts must close over cases plus the scheduled teardown',
    );
    const persisted = validatePersistedCampaign(fixture.runRoot);
    const persistedTransition = persisted.ledger
      .map((entry) => entry.record)
      .find((record) => record.recordType === 'controller_transition' && record.controllerState === 'execution_terminal');
    assert.deepEqual(persistedTransition.payload.resultCounts, transition.payload.resultCounts);
  } finally {
    fixture.cleanup();
  }
});

process.stdout.write('P158 campaign controller adversarial self-test passed\n');
