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
import { compileP158ControllerScheduleInput } from './p158-execution-schedule.js';

export const P158_AGGREGATE_ENTRY_PATHS = Object.freeze([
  '.github/workflows/p158-external-vantage.yml',
  '.github/workflows/p158-w8-h03-external.yml',
  '.github/workflows/p158-w9-endurance.yml',
  '.github/workflows/p158-w9-endurance-segment.yml',
  '.github/workflows/p158-w9-endurance-preparation.yml',
  'package.json',
  'pnpm-lock.yaml',
  'docs/dev/contracts/p158-campaign-freeze.v1.schema.json',
  'docs/dev/contracts/p158-case-execution-contract.v1.schema.json',
  'docs/dev/contracts/p158-campaign-manifest.v1.schema.json',
  'docs/dev/contracts/p158-campaign-preparation-fixtures.v1.schema.json',
  'docs/dev/contracts/p158-campaign-preparation-report.v1.schema.json',
  'docs/dev/contracts/p158-campaign-result.v1.schema.json',
  'docs/dev/contracts/p158-dashboard-fixtures.v1.schema.json',
  'docs/dev/contracts/p158-dashboard-oracle-report.v1.schema.json',
  'docs/dev/contracts/p158-external-handoff-fixtures.v1.schema.json',
  'docs/dev/contracts/p158-external-handoff-oracle-report.v1.schema.json',
  'docs/dev/contracts/p158-final-analysis.v1.schema.json',
  'docs/dev/contracts/p158-evidence-collector-config.v1.schema.json',
  'docs/dev/contracts/p158-historical-failure-registry.v1.json',
  'docs/dev/contracts/p158-logging-audit-report.v1.schema.json',
  'docs/dev/contracts/p158-logging-causal-fixtures.v1.schema.json',
  'docs/dev/contracts/p158-live-hook-manifest.v1.schema.json',
  'docs/dev/contracts/service-request.v1.schema.json',
  'docs/dev/fixtures/p158/campaign-preparation.v1.json',
  'docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json',
  'docs/dev/fixtures/p158/external-handoff-sessions.v1.json',
  'docs/dev/fixtures/p158/historical-failure-seeds.v1.json',
  'docs/dev/fixtures/p158/logging-causal-envelopes.v1.json',
  'scripts/lib/p158-campaign-controller.js',
  'scripts/lib/p158-campaign-phase-orchestrator.js',
  'scripts/lib/p158-campaign-preparation.js',
  'scripts/lib/p158-dashboard-oracle.js',
  'scripts/lib/p158-evidence-collector.js',
  'scripts/lib/p158-external-handoff-oracle.js',
  'scripts/lib/p158-final-analyzer.js',
  'scripts/lib/p158-execution-schedule.js',
  'scripts/lib/p158-logging-auditor.js',
  'scripts/lib/p158-calibration-runner.js',
  'scripts/lib/p158-distributed-calibration.js',
  'scripts/lib/p158-w7-development-adapters.js',
  'scripts/lib/p158-w7-agent-orchestration.js',
  'scripts/lib/p158-w7-a01-a03-live.js',
  'scripts/lib/p158-w7-a04-a06-live.js',
  'scripts/lib/p158-w7-live-hook-readiness.js',
  'scripts/lib/p158-w8-hd-adapters.js',
  'scripts/lib/p158-w8-h03-external.js',
  'scripts/lib/p158-w8-dashboard-live.js',
  'scripts/lib/p158-w9-campaign-orchestrator.js',
  'scripts/lib/p158-w9-concrete-drivers.js',
  'scripts/lib/p158-w9-endurance.js',
  'scripts/lib/p158-w9-endurance-preparation.js',
  'scripts/generate-p158-campaign-preparation-fixtures.js',
  'scripts/p158-evidence-collector.js',
  'scripts/p158-synthetic-visual-fixture.js',
  'scripts/run-p158-distributed-calibration-live.js',
  'scripts/run-p158-external-vantage.js',
  'scripts/run-p158-w7-a01-a03-live.js',
  'scripts/run-p158-w7-a04-a06-live.js',
  'scripts/run-p158-w8-h03-external.js',
  'scripts/run-p158-w9-endurance.js',
  'scripts/run-p158-w9-endurance-preparation.js',
  'scripts/test-p158-calibration-runner.js',
  'scripts/test-p158-campaign-controller.js',
  'scripts/test-p158-campaign-phase-orchestrator.js',
  'scripts/test-p158-campaign-preparation.js',
  'scripts/test-p158-dashboard-oracle.js',
  'scripts/test-p158-distributed-calibration-live.js',
  'scripts/test-p158-distributed-calibration.js',
  'scripts/test-p158-evidence-collector.js',
  'scripts/test-p158-execution-schedule.js',
  'scripts/test-p158-external-handoff-oracle.js',
  'scripts/test-p158-external-vantage-runner.js',
  'scripts/test-p158-final-analyzer.js',
  'scripts/test-p158-historical-failure-registry.js',
  'scripts/test-p158-logging-auditor.js',
  'scripts/test-p158-synthetic-visual-fixture.js',
  'scripts/test-p158-w7-development-adapters.js',
  'scripts/test-p158-w7-agent-orchestration.js',
  'scripts/test-p158-w7-a01-a03-live.js',
  'scripts/test-p158-w7-a04-a06-live.js',
  'scripts/test-p158-w7-live-hook-readiness.js',
  'scripts/test-p158-w8-hd-adapters.js',
  'scripts/test-p158-w8-h03-external.js',
  'scripts/test-p158-w8-dashboard-live.js',
  'scripts/test-p158-w9-campaign-orchestrator.js',
  'scripts/test-p158-w9-concrete-drivers.js',
  'scripts/test-p158-w9-endurance.js',
  'scripts/test-p158-w9-endurance-preparation.js',
]);

export const P158_REQUIRED_LIVE_HOOK_IDS = Object.freeze([
  'w7.agent_existing_seam_workflow',
  'w7.a01_a03.service_concurrency',
  'w7.a04_a06.profile_policy',
  'w7.browser', 'w7.cli', 'w7.display', 'w7.evidence', 'w7.logs', 'w7.process',
  'w7.shutdown', 'w7.systemd', 'w8.dashboard_capture', 'w8.dashboard_execute',
  'w8.external_workflow', 'w8.playwright', 'w8.stimulus', 'w9.browser_crash',
  'w9.external_dashboard_action', 'w9.external_handoff_reconnect',
  'w9.service_command', 'w9.supervisor_transition',
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

function digest(value) {
  return sha256Bytes(Buffer.from(canonicalJson(value)));
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

function withoutField(value, field) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== field));
}

export function validateP158LiveHookManifest({
  manifest, aggregate, schedule, candidate, adapters, repoRoot,
}) {
  if (!manifest) fail('live_hook_manifest_missing', 'Live freeze requires a concrete live-hook manifest');
  if (manifest.providerFree !== false || manifest.mode !== 'concrete_live' ||
      manifest.hookBindings?.some((binding) => binding.implementationKind !== 'concrete_live') ||
      manifest.adapterBindings?.some((binding) => binding.providerFree !== false)) {
    fail('provider_free_hooks_prohibited', 'Provider-free hooks cannot authorize a live freeze');
  }
  const ajv = new Ajv2020({ strict: true, allErrors: true });
  addFormats(ajv);
  const schema = parseJson(
    readRegularFile(
      resolve(repoRoot, 'docs/dev/contracts/p158-live-hook-manifest.v1.schema.json'),
      'P158 live-hook manifest schema',
    ),
    'P158 live-hook manifest schema',
  );
  const validate = ajv.compile(schema);
  if (!validate(manifest)) {
    fail('live_hook_manifest_invalid', 'Live-hook manifest violates its schema', { errors: validate.errors });
  }
  if (manifest.manifestSha256 !== digest(withoutField(manifest, 'manifestSha256')) ||
      manifest.aggregateSha256 !== aggregate.sha256 ||
      manifest.scheduleSha256 !== schedule.scheduleSha256 ||
      manifest.candidateSha256 !== candidate.candidateSha256) {
    fail('live_hook_manifest_binding_mismatch', 'Live-hook manifest is not bound to the frozen aggregate, schedule, and candidate');
  }
  const aggregateEntries = new Map(aggregate.manifest.entries.map((entry) => [entry.path, entry]));
  const hookIds = manifest.hookBindings.map((entry) => entry.hookId);
  const allowedHookIds = new Set(P158_REQUIRED_LIVE_HOOK_IDS);
  if (new Set(hookIds).size !== hookIds.length || hookIds.some((hookId) => !allowedHookIds.has(hookId))) {
    fail('live_hook_manifest_invalid', 'Live-hook manifest contains duplicate or unknown hook IDs');
  }
  for (const binding of manifest.hookBindings) {
    if (binding.implementationKind !== 'concrete_live' || binding.sourcePath.includes('/test-') ||
        binding.sourcePath.startsWith('scripts/test-')) {
      fail('provider_free_hooks_prohibited', `${binding.hookId} is not a concrete live hook source`);
    }
    const aggregateEntry = aggregateEntries.get(binding.sourcePath);
    const actual = readRegularFile(resolve(repoRoot, binding.sourcePath), `live hook ${binding.hookId}`);
    if (!aggregateEntry || binding.sourceSha256 !== aggregateEntry.sha256 ||
        sha256Bytes(actual) !== binding.sourceSha256) {
      fail('live_hook_source_unsealed', `${binding.hookId} source is absent from the frozen aggregate`);
    }
  }
  const contracts = new Map(schedule.caseContracts.map((entry) => [entry.caseId, entry]));
  const expectedActionCounts = new Map([...contracts.keys()].map((caseId) => [caseId, 0]));
  for (const attempt of schedule.attempts) {
    const allocated = attempt.cardinalityAllocations.reduce(
      (count, allocation) => count + allocation.actionIds.length,
      0,
    );
    expectedActionCounts.set(
      attempt.caseId,
      expectedActionCounts.get(attempt.caseId) + Math.max(1, allocated),
    );
  }
  const adapterObjects = new Map((adapters ?? []).map((entry) => [entry.caseId, entry]));
  if (manifest.adapterBindings.length !== contracts.size || new Set(
    manifest.adapterBindings.map((entry) => entry.caseId),
  ).size !== contracts.size) {
    fail('live_adapter_manifest_incomplete', 'Live-hook manifest must bind all scheduled case adapters');
  }
  for (const binding of manifest.adapterBindings) {
    const contract = contracts.get(binding.caseId);
    const adapter = adapterObjects.get(binding.caseId);
    const sourceEntry = aggregateEntries.get(binding.sourcePath);
    const expectedActionCount = expectedActionCounts.get(binding.caseId);
    if (!sourceEntry || binding.sourceSha256 !== sourceEntry.sha256 ||
        sha256Bytes(readRegularFile(resolve(repoRoot, binding.sourcePath), `adapter ${binding.caseId}`)) !==
          binding.sourceSha256) {
      fail('live_hook_source_unsealed', `${binding.caseId} adapter source is absent from the frozen aggregate`);
    }
    const adapterBindingSha256 = digest(binding);
    const exactAdapterBlocker = binding.blocker === null ? null : {
      ...binding.blocker,
      sourcePath: binding.sourcePath,
      sourceSha256: binding.sourceSha256,
    };
    if (!contract || binding.adapterId !== contract.adapterId ||
        binding.executionContractSha256 !== contract.executionContractSha256 ||
        binding.hookIds.some((hookId) => !hookIds.includes(hookId)) ||
        !adapter || adapter.caseId !== binding.caseId || adapter.adapterId !== binding.adapterId ||
        adapter.executionContractSha256 !== binding.executionContractSha256 ||
        adapter.executionMode !== binding.mode || adapter.providerFree !== false ||
        adapter.effectsAllowed !== binding.effectsAllowed ||
        adapter.sourcePath !== binding.sourcePath || adapter.sourceSha256 !== binding.sourceSha256 ||
        adapter.liveHookManifestSha256 !== manifest.manifestSha256 ||
        adapter.liveBindingSha256 !== adapterBindingSha256 ||
        digest(adapter.liveHookIds ?? []) !== digest(binding.hookIds) ||
        digest(adapter.blocker ?? null) !== digest(exactAdapterBlocker)) {
      fail('provider_free_hooks_prohibited', `${binding.caseId} adapter is not bound to its classified live-hook manifest entry`);
    }
    if (binding.mode === 'explicit_blocked' && (binding.effectsAllowed !== false ||
        binding.blockedActionCount !== expectedActionCount ||
        binding.implementedActionCount > expectedActionCount)) {
      fail('explicit_blocker_binding_mismatch', `${binding.caseId} explicit blocker is not immutable and zero-effect`);
    }
    if (binding.mode === 'concrete_live' && (binding.effectsAllowed !== true ||
        binding.implementedActionCount !== expectedActionCount || binding.blockedActionCount !== 0)) {
      fail('live_adapter_effect_classification_mismatch', `${binding.caseId} concrete adapter lacks an exact effect classification`);
    }
  }
  return structuredClone(manifest);
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

export function collectP158PreparationEvidence({
  config, repoRoot, baseDir = repoRoot, adapters, liveHookManifest,
}) {
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
  const registry = parseJson(
    readRegularFile(
      resolve(repoRoot, 'docs/dev/contracts/p158-historical-failure-registry.v1.json'),
      'P158 registry',
    ),
    'P158 registry',
  );
  let compiled;
  try {
    compiled = compileP158ControllerScheduleInput({
      registry,
      seed: config.seed,
      adapters,
    });
  } catch (error) {
    if (error?.code === 'adapters_not_ready') {
      fail('adapter_readiness_failed', 'P158 case adapters are incomplete before freeze',
        error.details);
    }
    throw error;
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
  const executionScheduleBytes = Buffer.from(canonicalJson(compiled.executionSchedule), 'utf8');
  const executionScheduleSha256 = createHash('sha256').update(executionScheduleBytes).digest('hex');
  artifacts.push({
    artifactId: 'freeze-artifact-19',
    kind: 'execution_schedule',
    relativePath: 'freeze/execution-schedule.json',
    capturedAt: config.aggregateCapturedAt,
    mediaType: 'application/json',
    contentEncoding: 'base64',
    content: executionScheduleBytes.toString('base64'),
    declaredSha256: executionScheduleSha256,
    declaredByteCount: executionScheduleBytes.byteLength,
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
  const preExecutionBlockers = new Map();
  if (liveHookManifest) {
    validateP158LiveHookManifest({
      manifest: liveHookManifest,
      aggregate,
      schedule: compiled.executionSchedule,
      candidate,
      adapters,
      repoRoot,
    });
    for (const binding of liveHookManifest.adapterBindings) {
      if (binding.mode === 'explicit_blocked') {
        preExecutionBlockers.set(binding.caseId, {
          ...structuredClone(binding.blocker),
          sourcePath: binding.sourcePath,
          sourceSha256: binding.sourceSha256,
        });
      }
    }
    const liveHookBytes = Buffer.from(canonicalJson(liveHookManifest));
    artifacts.push({
      artifactId: 'freeze-artifact-20',
      kind: 'live_hook_manifest',
      relativePath: 'freeze/live-hook-manifest.json',
      capturedAt: liveHookManifest.capturedAt,
      mediaType: 'application/json',
      contentEncoding: 'base64',
      content: liveHookBytes.toString('base64'),
      declaredSha256: sha256Bytes(liveHookBytes),
      declaredByteCount: liveHookBytes.byteLength,
    });
  }
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
      campaignMode: 'live',
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
      executionScheduleSeal: {
        artifactId: 'freeze-artifact-19',
        artifactSha256: executionScheduleSha256,
        scheduleSha256: compiled.executionSchedule.scheduleSha256,
        attemptCount: compiled.executionSchedule.attemptCount,
      },
      schedule: compiled.controllerSchedule.map((attempt) => ({
        ...structuredClone(attempt),
        preExecutionBlocker: structuredClone(preExecutionBlockers.get(attempt.caseId) ?? null),
      })),
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
  adapters,
  liveHookManifest,
}) {
  if (freeze && !liveHookManifest) {
    fail('live_hook_manifest_missing', 'Live freeze requires a concrete live-hook manifest before collection');
  }
  if (!freeze && !clock && !Number.isFinite(Date.parse(config.dryRunFrozenAt))) {
    fail('dry_run_time_missing', 'Deterministic dry run requires dryRunFrozenAt');
  }
  const effectiveClock = clock ?? (!freeze ? {
    wallNow: () => config.dryRunFrozenAt,
    monotonicNow: () => 1,
  } : undefined);
  const collected = collectP158PreparationEvidence({
    config, repoRoot, baseDir, adapters, liveHookManifest,
  });
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
  }, { registry, seed: config.seed, adapters });
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
    liveHookManifestSha256: liveHookManifest?.manifestSha256 ?? null,
    preparationReport: report,
  };
}
