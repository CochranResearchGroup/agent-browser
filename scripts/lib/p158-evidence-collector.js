import { createHash } from 'node:crypto';
import { existsSync, lstatSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { isAbsolute, relative, resolve, sep } from 'node:path';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import {
  canonicalJson,
  createCampaignController,
  createMemoryArtifactStore,
} from './p158-campaign-controller.js';
import {
  canonicalCalibrationDigest,
  canonicalCandidateDigest,
  canonicalEnvironmentDigest,
  canonicalEnvironmentSealDigest,
  prepareAndFreezeCampaign,
} from './p158-campaign-preparation.js';

export const P158_AGGREGATE_ENTRY_PATHS = Object.freeze([
  'docs/dev/contracts/p158-campaign-freeze.v1.schema.json',
  'docs/dev/contracts/p158-campaign-manifest.v1.schema.json',
  'docs/dev/contracts/p158-campaign-preparation-fixtures.v1.schema.json',
  'docs/dev/contracts/p158-campaign-preparation-report.v1.schema.json',
  'docs/dev/contracts/p158-campaign-result.v1.schema.json',
  'docs/dev/contracts/p158-dashboard-fixtures.v1.schema.json',
  'docs/dev/contracts/p158-dashboard-oracle-report.v1.schema.json',
  'docs/dev/contracts/p158-external-handoff-fixtures.v1.schema.json',
  'docs/dev/contracts/p158-external-handoff-oracle-report.v1.schema.json',
  'docs/dev/contracts/p158-evidence-collector-config.v1.schema.json',
  'docs/dev/contracts/p158-historical-failure-registry.v1.json',
  'docs/dev/contracts/p158-logging-audit-report.v1.schema.json',
  'docs/dev/contracts/p158-logging-causal-fixtures.v1.schema.json',
  'docs/dev/fixtures/p158/campaign-preparation.v1.json',
  'docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json',
  'docs/dev/fixtures/p158/external-handoff-sessions.v1.json',
  'docs/dev/fixtures/p158/historical-failure-seeds.v1.json',
  'docs/dev/fixtures/p158/logging-causal-envelopes.v1.json',
  'scripts/lib/p158-campaign-controller.js',
  'scripts/lib/p158-campaign-preparation.js',
  'scripts/lib/p158-dashboard-oracle.js',
  'scripts/lib/p158-external-handoff-oracle.js',
  'scripts/lib/p158-logging-auditor.js',
]);

export const P158_SUPPLIED_ARTIFACT_KINDS = Object.freeze([
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
]);

const E1_RECEIPT_KINDS = Object.freeze([
  'installation_receipt', 'runtime_status_receipt', 'runtime_doctor_receipt', 'runtime_manifest',
]);
const E2_RECEIPT_KINDS = Object.freeze([
  'provider_plan_receipt', 'provider_stage_receipt', 'provider_preflight_receipt',
  'provider_apply_receipt', 'provider_status_receipt', 'provider_doctor_receipt',
  'provider_configuration', 'external_ingress_deployment', 'external_vantage',
  'external_handoff_oracle_report',
]);

export class P158EvidenceCollectorError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158EvidenceCollectorError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158EvidenceCollectorError(code, message, details);
}

function sha256Bytes(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function readRegularFile(path, label) {
  if (!existsSync(path)) fail('evidence_file_missing', `Missing ${label}`, { path });
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink()) {
    fail('evidence_file_not_regular', `${label} must be a regular non-symlink file`, { path });
  }
  return readFileSync(path);
}

function parseJson(bytes, label) {
  try {
    return JSON.parse(bytes.toString('utf8'));
  } catch (error) {
    fail('evidence_json_invalid', `${label} is not valid JSON`, { message: error.message });
  }
}

function resolveExplicitPath(baseDir, path, label) {
  if (typeof path !== 'string' || path.length === 0) {
    fail('evidence_path_missing', `${label} requires an explicit path`);
  }
  return isAbsolute(path) ? path : resolve(baseDir, path);
}

function assertExpectedDigest(descriptor, bytes, label) {
  const actualSha256 = sha256Bytes(bytes);
  if (!/^[a-f0-9]{64}$/.test(descriptor.expectedSha256 ?? '')) {
    fail('expected_digest_missing', `${label} requires expectedSha256`);
  }
  if (descriptor.expectedSha256 !== actualSha256) {
    fail('evidence_hash_drift', `${label} does not match its expected SHA-256`, {
      expected: descriptor.expectedSha256,
      actual: actualSha256,
    });
  }
  if (
    descriptor.expectedByteCount !== undefined &&
    descriptor.expectedByteCount !== bytes.byteLength
  ) {
    fail('evidence_byte_count_drift', `${label} does not match its expected byte count`, {
      expected: descriptor.expectedByteCount,
      actual: bytes.byteLength,
    });
  }
  return actualSha256;
}

export function buildP158AggregateFixtureManifest({ repoRoot }) {
  const normalizedRoot = resolve(repoRoot);
  const entries = P158_AGGREGATE_ENTRY_PATHS.map((path) => {
    const bytes = readRegularFile(resolve(normalizedRoot, path), `aggregate entry ${path}`);
    return { path, sha256: sha256Bytes(bytes), byteCount: bytes.byteLength };
  });
  const manifest = {
    schemaVersion: 'agent-browser.p158-fixture-aggregate.v1',
    planId: 'P158',
    entryCount: entries.length,
    entries,
  };
  const bytes = Buffer.from(canonicalJson(manifest));
  return { manifest, bytes, sha256: sha256Bytes(bytes), byteCount: bytes.byteLength };
}

function suppliedArtifacts({ artifactFiles, baseDir }) {
  if (!artifactFiles || typeof artifactFiles !== 'object') {
    fail('artifact_files_missing', 'artifactFiles is required');
  }
  const unexpectedKinds = Object.keys(artifactFiles)
    .filter((kind) => !P158_SUPPLIED_ARTIFACT_KINDS.includes(kind))
    .sort();
  if (unexpectedKinds.length > 0) {
    fail('artifact_descriptor_unexpected', 'artifactFiles contains unexpected kinds', {
      unexpectedKinds,
    });
  }
  return P158_SUPPLIED_ARTIFACT_KINDS.map((kind, index) => {
    const descriptor = artifactFiles[kind];
    if (!descriptor) fail('artifact_descriptor_missing', `Missing artifact descriptor ${kind}`);
    const path = resolveExplicitPath(baseDir, descriptor.path, `artifact ${kind}`);
    const bytes = readRegularFile(path, `artifact ${kind}`);
    const actualSha256 = assertExpectedDigest(descriptor, bytes, `artifact ${kind}`);
    if (!Number.isFinite(Date.parse(descriptor.capturedAt))) {
      fail('artifact_capture_time_invalid', `artifact ${kind} requires an RFC 3339 capturedAt`);
    }
    return {
      artifactId: descriptor.artifactId ?? `freeze-artifact-${String(index + 1).padStart(2, '0')}`,
      kind,
      relativePath: descriptor.relativePath ?? `freeze/${kind}.json`,
      capturedAt: descriptor.capturedAt,
      mediaType: descriptor.mediaType ?? 'application/json',
      contentEncoding: 'base64',
      content: bytes.toString('base64'),
      declaredSha256: actualSha256,
      declaredByteCount: bytes.byteLength,
    };
  });
}

function environmentSeal(environment, candidate, artifacts, externalClientIds) {
  const byKind = new Map(artifacts.map((artifact) => [artifact.kind, artifact]));
  const environmentId = environment.environmentId;
  const receiptKinds = environmentId === 'E1' ? E1_RECEIPT_KINDS : E2_RECEIPT_KINDS;
  const identity = structuredClone(environment.identity);
  if (environmentId === 'E1') identity.candidate = structuredClone(candidate);
  const result = {
    environmentId,
    identityId: environment.identityId,
    identity,
    identitySha256: canonicalEnvironmentDigest(identity),
    sealedAt: environment.sealedAt,
    receiptArtifactIds: receiptKinds.map((kind) => byKind.get(kind).artifactId),
  };
  if (environmentId === 'E2') {
    result.externalClientIds = [...externalClientIds].sort();
    result.externalVantageArtifactId = byKind.get('external_vantage').artifactId;
    result.externalHandoffOracleArtifactId = byKind.get('external_handoff_oracle_report').artifactId;
  }
  result.sealSha256 = canonicalEnvironmentSealDigest({
    environmentId: result.environmentId,
    identityId: result.identityId,
    identitySha256: result.identitySha256,
    sealedAt: result.sealedAt,
    receiptArtifactIds: [...result.receiptArtifactIds].sort(),
    ...(environmentId === 'E2' ? {
      externalClientIds: result.externalClientIds,
      externalVantageArtifactId: result.externalVantageArtifactId,
      externalHandoffOracleArtifactId: result.externalHandoffOracleArtifactId,
    } : {}),
  });
  return result;
}

export function collectP158PreparationEvidence({ config, repoRoot, baseDir = repoRoot }) {
  const configAjv = new Ajv2020({ strict: true, allErrors: true });
  addFormats(configAjv);
  const configSchema = parseJson(
    readRegularFile(
      resolve(repoRoot, 'docs/dev/contracts/p158-evidence-collector-config.v1.schema.json'),
      'P158 evidence collector config schema',
    ),
    'P158 evidence collector config schema',
  );
  const validateConfig = configAjv.compile(configSchema);
  if (!validateConfig(config)) {
    fail('collector_config_invalid', 'P158 evidence collector config violates its schema', {
      errors: validateConfig.errors,
    });
  }
  const aggregate = buildP158AggregateFixtureManifest({ repoRoot });
  if (!/^[a-f0-9]{64}$/.test(config.expectedAggregateSha256 ?? '')) {
    fail('expected_aggregate_digest_missing', 'expectedAggregateSha256 is required');
  }
  if (config.expectedAggregateSha256 !== aggregate.sha256) {
    fail('aggregate_hash_drift', 'P158 aggregate fixture manifest drifted', {
      expected: config.expectedAggregateSha256,
      actual: aggregate.sha256,
    });
  }
  const artifacts = suppliedArtifacts({ artifactFiles: config.artifactFiles, baseDir });
  artifacts.push({
    artifactId: 'freeze-artifact-18',
    kind: 'fixture_aggregate_manifest',
    relativePath: 'freeze/fixture-aggregate-manifest.json',
    capturedAt: config.aggregateCapturedAt,
    mediaType: 'application/json',
    contentEncoding: 'base64',
    content: aggregate.bytes.toString('base64'),
    declaredSha256: aggregate.sha256,
    declaredByteCount: aggregate.byteCount,
  });
  const byKind = new Map(artifacts.map((artifact) => [artifact.kind, artifact]));
  const externalVantage = parseJson(
    Buffer.from(byKind.get('external_vantage').content, 'base64'),
    'external_vantage',
  );
  const w4Report = parseJson(
    Buffer.from(byKind.get('external_handoff_oracle_report').content, 'base64'),
    'external_handoff_oracle_report',
  );
  const candidate = {
    ...structuredClone(config.candidate),
    runId: config.runId,
    aggregateFixtureManifestSha256: aggregate.sha256,
  };
  delete candidate.candidateSha256;
  candidate.candidateSha256 = canonicalCandidateDigest(candidate);
  const calibration = {
    ...structuredClone(config.calibration),
    rawArtifactId: byKind.get('calibration_raw').artifactId,
    rawArtifactSha256: byKind.get('calibration_raw').declaredSha256,
    summaryArtifactId: byKind.get('calibration_summary').artifactId,
    summaryArtifactSha256: byKind.get('calibration_summary').declaredSha256,
    budgetArtifactId: byKind.get('calibration_budget').artifactId,
    budgetSha256: byKind.get('calibration_budget').declaredSha256,
  };
  delete calibration.declaredSha256;
  calibration.declaredSha256 = canonicalCalibrationDigest(calibration);
  const metadata = config.environments ?? [];
  const e1 = metadata.find((environment) => environment.environmentId === 'E1');
  const e2 = metadata.find((environment) => environment.environmentId === 'E2');
  if (!e1 || !e2 || metadata.length !== 2) {
    fail('environment_metadata_invalid', 'Exactly one E1 and one E2 environment are required');
  }
  const externalClientIds = (externalVantage.clients ?? []).map((client) => client.clientId);
  const environments = [
    environmentSeal(e1, candidate, artifacts, externalClientIds),
    environmentSeal(e2, candidate, artifacts, externalClientIds),
  ];
  const result = {
    aggregate,
    input: {
      candidate,
      artifacts,
      environments,
      calibration,
      externalVantage,
      w4Report,
      fixtureSeal: {
        aggregateArtifactId: 'freeze-artifact-18',
        aggregateSha256: aggregate.sha256,
        entryCount: aggregate.manifest.entryCount,
        includedPaths: aggregate.manifest.entries.map((entry) => entry.path),
      },
      freezeContract: {
        freezeId: config.freezeId,
        requiredStartedCaseCount: 0,
        requiredStartedAttemptCount: 0,
      },
      schedule: structuredClone(config.schedule),
      scheduledTeardown: structuredClone(config.scheduledTeardown),
    },
  };
  const ajv = new Ajv2020({ strict: true, allErrors: true });
  addFormats(ajv);
  const fixtureSchema = parseJson(
    readRegularFile(
      resolve(repoRoot, 'docs/dev/contracts/p158-campaign-preparation-fixtures.v1.schema.json'),
      'P158 preparation fixture schema',
    ),
    'P158 preparation fixture schema',
  );
  const w4Schema = parseJson(
    readRegularFile(
      resolve(repoRoot, 'docs/dev/contracts/p158-external-handoff-oracle-report.v1.schema.json'),
      'P158 W4 report schema',
    ),
    'P158 W4 report schema',
  );
  ajv.addSchema(fixtureSchema);
  const validateInput = ajv.compile({ $ref: `${fixtureSchema.$id}#/$defs/input` });
  const validateW4 = ajv.compile(w4Schema);
  if (!validateInput(result.input)) {
    fail('assembled_input_invalid', 'Assembled P158 preparation input violates its schema', {
      errors: validateInput.errors,
    });
  }
  if (!validateW4(result.input.w4Report)) {
    fail('w4_report_invalid', 'Supplied W4 report violates its schema', {
      errors: validateW4.errors,
    });
  }
  return result;
}

function assertSafeRunRoot(runRoot, repoRoot) {
  if (!isAbsolute(runRoot)) fail('run_root_not_absolute', 'Freeze runRoot must be absolute');
  const normalized = resolve(runRoot);
  const forbidden = new Set(['/', resolve(repoRoot), resolve(homedir())]);
  if (forbidden.has(normalized) || relative(resolve(repoRoot), normalized) === '') {
    fail('run_root_unsafe', 'Freeze runRoot is too broad', { runRoot: normalized });
  }
  const repoRelative = relative(resolve(repoRoot), normalized);
  if (repoRelative && !repoRelative.startsWith(`..${sep}`) && repoRelative !== '..') {
    fail('run_root_in_repo', 'Campaign evidence must remain outside the product repo', { runRoot: normalized });
  }
  return normalized;
}

export async function runP158EvidenceCollector({
  config,
  repoRoot,
  baseDir = repoRoot,
  freeze = false,
  runRoot,
  clock,
}) {
  if (!freeze && !clock && !Number.isFinite(Date.parse(config.dryRunFrozenAt))) {
    fail('dry_run_time_missing', 'Deterministic dry run requires dryRunFrozenAt');
  }
  const effectiveClock = clock ?? (!freeze ? {
    wallNow: () => config.dryRunFrozenAt,
    monotonicNow: () => 1,
  } : undefined);
  const collected = collectP158PreparationEvidence({ config, repoRoot, baseDir });
  const registry = parseJson(
    readRegularFile(resolve(repoRoot, 'docs/dev/contracts/p158-historical-failure-registry.v1.json'), 'P158 registry'),
    'P158 registry',
  );
  const store = freeze
    ? undefined
    : createMemoryArtifactStore();
  const controller = createCampaignController({
    ...(freeze ? { runRoot: assertSafeRunRoot(runRoot, repoRoot) } : { store }),
    registry,
    runId: config.runId,
    seed: config.seed,
    clock: effectiveClock,
  });
  const report = await prepareAndFreezeCampaign({
    ...collected.input,
    controller,
    clock: effectiveClock,
  });
  if (!report.passed) {
    fail('preparation_not_ready', 'P158 preparation evidence did not pass', { findings: report.findings });
  }
  if (report.controllerState !== 'frozen' || report.zeroStartedAttemptCount !== 0) {
    fail('freeze_state_invalid', 'P158 collector crossed or failed the frozen zero-start boundary', {
      controllerState: report.controllerState,
      zeroStartedAttemptCount: report.zeroStartedAttemptCount,
    });
  }
  return {
    schemaVersion: 'agent-browser.p158-evidence-collector-report.v1',
    planId: 'P158',
    mode: freeze ? 'freeze' : 'dry_run',
    externalEffectsAttempted: freeze,
    executionStarted: false,
    aggregateManifest: collected.aggregate.manifest,
    aggregateSha256: collected.aggregate.sha256,
    preparationReport: report,
  };
}
