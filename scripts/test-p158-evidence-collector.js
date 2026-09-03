#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

import {
  buildP158AggregateFixtureManifest,
  collectP158PreparationEvidence,
  P158EvidenceCollectorError,
  P158_SUPPLIED_ARTIFACT_KINDS,
  runP158EvidenceCollector,
} from './lib/p158-evidence-collector.js';

const repoRoot = new URL('..', import.meta.url).pathname;
const fixedClock = {
  wallNow: () => '2026-09-02T20:01:00.000Z',
  monotonicNow: () => 100,
};

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function makeContext(label) {
  const root = mkdtempSync(join(tmpdir(), `agent-browser-p158-collector-${label}-`));
  const evidenceRoot = join(root, 'evidence');
  mkdirSync(evidenceRoot);
  const cleanFixture = JSON.parse(
    readFileSync(join(repoRoot, 'docs/dev/fixtures/p158/campaign-preparation.v1.json'), 'utf8'),
  ).fixtures.find((fixture) => fixture.fixtureId === 'clean-freeze-ready').input;
  const artifactFiles = {};
  for (const [index, kind] of P158_SUPPLIED_ARTIFACT_KINDS.entries()) {
    const path = join(evidenceRoot, `${kind}.json`);
    let bytes = Buffer.from(`${JSON.stringify({ kind, ordinal: index + 1 })}\n`);
    if (kind === 'external_vantage') bytes = Buffer.from(`${JSON.stringify(cleanFixture.externalVantage)}\n`);
    if (kind === 'external_handoff_oracle_report') bytes = Buffer.from(`${JSON.stringify(cleanFixture.w4Report)}\n`);
    writeFileSync(path, bytes, { mode: 0o600 });
    artifactFiles[kind] = {
      path,
      ...(kind === 'external_vantage' ? { artifactId: 'artifact-13' } : {}),
      expectedSha256: sha256(bytes),
      expectedByteCount: bytes.length,
      capturedAt: '2026-09-02T20:00:05.000Z',
    };
  }
  const aggregate = buildP158AggregateFixtureManifest({ repoRoot });
  const config = {
    schemaVersion: 'agent-browser.p158-evidence-collector-config.v1',
    runId: `p158-collector-${label}`,
    seed: 'p158-collector-fixed-seed',
    expectedAggregateSha256: aggregate.sha256,
    aggregateCapturedAt: '2026-09-02T20:00:06.000Z',
    dryRunFrozenAt: fixedClock.wallNow(),
    candidate: {
      sourceCommit: 'e26a6b05c315cfed06a833a5c4d7406803bcc0fb',
      binarySha256: '11'.repeat(32),
      dashboardSha256: '22'.repeat(32),
      installedGenerationId: 'development-generation-p158',
      browserExecutableSha256: '33'.repeat(32),
      runtimeManifestRevision: 'runtime-manifest-p158-v1',
      providerConfigurationRevision: 'provider-configuration-p158-v1',
      externalIngressDeploymentRevision: 'external-ingress-p158-v1',
      preparedAt: '2026-09-02T19:30:00.000Z',
    },
    artifactFiles,
    environments: [
      {
        environmentId: 'E1',
        identityId: 'development-runtime-e1',
        identity: { environment: 'development', runtimeLane: 'development-default' },
        sealedAt: '2026-09-02T20:00:10.000Z',
      },
      {
        environmentId: 'E2',
        identityId: 'external-presentation-e2',
        identity: { environment: 'development', provider: 'isolated-guacamole', ingressScheme: 'https' },
        sealedAt: '2026-09-02T20:00:15.000Z',
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
      environmentRelativeBudgets: { agentCommandP95Ms: 750, handoffPixelsP95Ms: 8000 },
    },
    freezeId: `${label}:freeze`,
    schedule: [{ caseId: 'A01', attemptId: `${label}:A01`, environmentId: 'E1', dependsOn: [] }],
    scheduledTeardown: { caseId: 'TEARDOWN', environmentId: 'E1', dependsOn: ['A01'] },
  };
  return {
    root,
    config,
    freezeRoot: join(root, 'campaign'),
    configPath: join(root, 'collector-config.json'),
    cleanup: () => rmSync(root, { recursive: true, force: true }),
  };
}

async function runTest(name, body) {
  await body();
  process.stdout.write(`PASS ${name}\n`);
}

await runTest('builds a deterministic aggregate over the frozen P158 source set', async () => {
  const first = buildP158AggregateFixtureManifest({ repoRoot });
  const second = buildP158AggregateFixtureManifest({ repoRoot });
  assert.deepEqual(second, first);
  assert.equal(first.manifest.entryCount, first.manifest.entries.length);
  assert.ok(first.manifest.entries.length >= 20);
  assert.equal(first.sha256, sha256(first.bytes));
});

await runTest('assembles exactly eighteen byte-bound preparation artifacts', async () => {
  const context = makeContext('assemble');
  try {
    const collected = collectP158PreparationEvidence({
      config: context.config,
      repoRoot,
      baseDir: context.root,
    });
    assert.equal(collected.input.artifacts.length, 18);
    assert.equal(new Set(collected.input.artifacts.map((artifact) => artifact.kind)).size, 18);
    for (const artifact of collected.input.artifacts) {
      const bytes = Buffer.from(artifact.content, 'base64');
      assert.equal(artifact.declaredSha256, sha256(bytes));
      assert.equal(artifact.declaredByteCount, bytes.length);
    }
    assert.equal(collected.input.candidate.aggregateFixtureManifestSha256, collected.aggregate.sha256);
  } finally {
    context.cleanup();
  }
});

await runTest('defaults to an in-memory dry run and never starts execution', async () => {
  const context = makeContext('dry-run');
  try {
    const report = await runP158EvidenceCollector({
      config: context.config,
      repoRoot,
      baseDir: context.root,
      clock: fixedClock,
    });
    assert.equal(report.mode, 'dry_run');
    assert.equal(report.externalEffectsAttempted, false);
    assert.equal(report.executionStarted, false);
    assert.equal(report.preparationReport.controllerState, 'frozen');
    assert.equal(report.preparationReport.zeroStartedAttemptCount, 0);
    assert.equal(existsSync(context.freezeRoot), false);
  } finally {
    context.cleanup();
  }
});

await runTest('fails closed on missing evidence and digest drift', async () => {
  const context = makeContext('fail-closed');
  try {
    const missing = context.config.artifactFiles.runtime_doctor_receipt.path;
    rmSync(missing);
    assert.throws(
      () => collectP158PreparationEvidence({ config: context.config, repoRoot, baseDir: context.root }),
      (error) => error instanceof P158EvidenceCollectorError && error.code === 'evidence_file_missing',
    );
    writeFileSync(missing, '{}\n');
    assert.throws(
      () => collectP158PreparationEvidence({ config: context.config, repoRoot, baseDir: context.root }),
      (error) => error instanceof P158EvidenceCollectorError && error.code === 'evidence_hash_drift',
    );
    context.config.artifactFiles.runtime_doctor_receipt.expectedSha256 = sha256(Buffer.from('{}\n'));
    context.config.artifactFiles.runtime_doctor_receipt.expectedByteCount = 3;
    context.config.expectedAggregateSha256 = 'ff'.repeat(32);
    assert.throws(
      () => collectP158PreparationEvidence({ config: context.config, repoRoot, baseDir: context.root }),
      (error) => error instanceof P158EvidenceCollectorError && error.code === 'aggregate_hash_drift',
    );
  } finally {
    context.cleanup();
  }
});

await runTest('persists only prepared and frozen state behind the explicit freeze flag', async () => {
  const context = makeContext('freeze');
  try {
    const report = await runP158EvidenceCollector({
      config: context.config,
      repoRoot,
      baseDir: context.root,
      freeze: true,
      runRoot: context.freezeRoot,
      clock: fixedClock,
    });
    assert.equal(report.mode, 'freeze');
    assert.equal(report.executionStarted, false);
    assert.ok(existsSync(join(context.freezeRoot, 'campaign-manifest.json')));
    assert.ok(existsSync(join(context.freezeRoot, 'campaign-freeze.json')));
    const records = readdirSync(join(context.freezeRoot, 'ledger'))
      .map((path) => JSON.parse(readFileSync(join(context.freezeRoot, 'ledger', path), 'utf8')));
    const states = records.map((record) => record.controllerState);
    assert.ok(states.slice(0, -1).every((state) => state === 'prepared'));
    assert.equal(states.at(-1), 'frozen');
    assert.equal(states.includes('executing'), false);
  } finally {
    context.cleanup();
  }
});

await runTest('CLI remains dry-run unless the freeze flag is explicit', async () => {
  const context = makeContext('cli');
  try {
    writeFileSync(context.configPath, `${JSON.stringify(context.config, null, 2)}\n`);
    const result = spawnSync(process.execPath, [
      join(repoRoot, 'scripts/p158-evidence-collector.js'), '--config', context.configPath,
    ], { cwd: repoRoot, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    const report = JSON.parse(result.stdout);
    assert.equal(report.mode, 'dry_run');
    assert.equal(report.executionStarted, false);
    assert.equal(existsSync(context.freezeRoot), false);
  } finally {
    context.cleanup();
  }
});

process.stdout.write('P158 evidence collector provider-free self-test passed\n');
