import { sha256 as controllerSha256 } from './p158-campaign-controller.js';
import { compileP158ControllerScheduleInput } from './p158-execution-schedule.js';

export const P158_PREPARATION_FINDING_CODES = Object.freeze([
  'artifact_byte_count_mismatch',
  'artifact_hash_mismatch',
  'candidate_digest_mismatch',
  'controller_not_pristine',
  'duplicate_artifact_id',
  'environment_digest_mismatch',
  'execution_schedule_mismatch',
  'external_client_identity_invalid',
  'external_ingress_observation_missing',
  'fixture_digest_mismatch',
  'invalid_calibration',
  'missing_artifact_kind',
  'referenced_artifact_missing',
  'w4_oracle_not_clean',
]);

const REQUIRED_ARTIFACT_KINDS = Object.freeze([
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
]);

const REQUIRED_INGRESS_OBSERVATIONS = Object.freeze([
  'dns',
  'tls',
  'redirect',
  'cookie',
  'websocket',
  'iframe',
  'form_action',
  'reconnect',
]);

const CALIBRATION_WORKLOAD = Object.freeze({
  durationMinutes: 20,
  agentClients: 25,
  externalViewers: 2,
  controllers: 1,
  serviceCommands: 500,
  dashboardActions: 50,
  handoffReconnects: 10,
});

function sha256Bytes(value) {
  return controllerSha256(value);
}

function digest(value) {
  return controllerSha256(value);
}

function clone(value) {
  return structuredClone(value);
}

function artifactBytes(artifact) {
  const { content, contentEncoding } = artifact;
  if (typeof content === 'string' && contentEncoding === 'utf8') return Buffer.from(content, 'utf8');
  if (typeof content === 'string' && contentEncoding === 'base64') return Buffer.from(content, 'base64');
  if (content instanceof Uint8Array && (contentEncoding === 'bytes' || contentEncoding === undefined)) {
    return Buffer.from(content);
  }
  throw new TypeError('P158 preparation artifacts require string or Uint8Array content');
}

function without(object, fields) {
  return Object.fromEntries(
    Object.entries(object ?? {}).filter(([field]) => !fields.includes(field)),
  );
}

export function canonicalCandidateDigest(candidate) {
  return digest(without(candidate, ['candidateSha256']));
}

export function canonicalEnvironmentDigest(environment) {
  return digest(environment?.identity ?? environment);
}

export function canonicalEnvironmentSealDigest(environmentSeal) {
  return digest(without(environmentSeal, ['sealSha256']));
}

export function canonicalCalibrationDigest(calibration) {
  return digest(without(calibration, ['calibrationSha256', 'declaredSha256']));
}

function addFinding(findings, code, field, expected, observed) {
  const finding = {
    code,
    field,
    expected: clone(expected ?? null),
    observed: clone(observed ?? null),
  };
  const identity = digest(finding);
  if (!findings.some((entry) => entry.identity === identity)) {
    findings.push({ identity, ...finding });
  }
}

function artifactProjection(artifact, bytes, actualSha256) {
  return {
    artifactId: artifact.artifactId,
    kind: artifact.kind,
    relativePath: artifact.relativePath,
    sha256: actualSha256,
    byteCount: bytes.byteLength,
    capturedAt: artifact.capturedAt,
  };
}

function inspectArtifacts(artifacts, findings, campaignMode) {
  const entries = [];
  const byId = new Map();
  for (const artifact of artifacts ?? []) {
    if (byId.has(artifact.artifactId)) {
      addFinding(findings, 'duplicate_artifact_id', 'artifacts.artifactId', 'unique', artifact.artifactId);
      continue;
    }
    const bytes = artifactBytes(artifact);
    const actualSha256 = sha256Bytes(bytes);
    if (bytes.byteLength === 0) {
      addFinding(findings, 'artifact_byte_count_mismatch', `${artifact.artifactId}.content`, 'at least 1 byte', 0);
    }
    if (artifact.declaredSha256 !== actualSha256) {
      addFinding(findings, 'artifact_hash_mismatch', `${artifact.artifactId}.declaredSha256`, actualSha256, artifact.declaredSha256);
    }
    if (artifact.declaredByteCount !== bytes.byteLength) {
      addFinding(findings, 'artifact_byte_count_mismatch', `${artifact.artifactId}.declaredByteCount`, bytes.byteLength, artifact.declaredByteCount);
    }
    const entry = { source: artifact, bytes, binding: artifactProjection(artifact, bytes, actualSha256) };
    byId.set(artifact.artifactId, entry);
    entries.push(entry);
  }
  const requiredKinds = campaignMode === 'live'
    ? [...REQUIRED_ARTIFACT_KINDS, 'execution_schedule']
    : REQUIRED_ARTIFACT_KINDS;
  for (const kind of requiredKinds) {
    if (!entries.some((entry) => entry.binding.kind === kind)) {
      addFinding(findings, 'missing_artifact_kind', 'artifacts.kind', kind, null);
    }
  }
  return { entries, byId };
}

function inspectExecutionSchedule(input, artifacts, findings, executionContext, controllerSnapshot) {
  if (input.campaignMode !== 'live') return;
  const artifact = artifacts.byId.get(input.executionScheduleSeal?.artifactId);
  let persisted = null;
  try {
    persisted = artifact ? JSON.parse(artifact.bytes.toString('utf8')) : null;
  } catch {
    persisted = null;
  }
  const seal = input.executionScheduleSeal;
  const expectedAttempts = persisted?.attempts?.map((attempt) => ({
    caseId: attempt.caseId,
    attemptId: attempt.attemptId,
    environmentId: attempt.environmentId,
    environmentIds: attempt.environmentIds,
    seed: attempt.seed,
    dependsOn: attempt.dependsOnAttemptIds,
  })) ?? null;
  let independentlyCompiled = null;
  try {
    independentlyCompiled = compileP158ControllerScheduleInput({
      registry: executionContext?.registry,
      seed: executionContext?.seed,
      adapters: executionContext?.adapters,
    });
  } catch {
    independentlyCompiled = null;
  }
  const valid =
    artifact?.binding.kind === 'execution_schedule' &&
    artifact.binding.sha256 === seal?.artifactSha256 &&
    persisted?.schemaVersion === 'agent-browser.p158-execution-schedule.v1' &&
    persisted?.scheduleSha256 === seal?.scheduleSha256 &&
    persisted?.caseCount === 54 &&
    persisted?.attemptCount === 1592 &&
    seal?.attemptCount === 1592 &&
    persisted?.adapterReadiness?.ready === true &&
    persisted?.adapterReadiness?.readyCaseCount === 54 &&
    independentlyCompiled !== null &&
    independentlyCompiled.executionSchedule.registrySha256 === controllerSnapshot.registrySha256 &&
    digest(persisted) === digest(independentlyCompiled.executionSchedule) &&
    Array.isArray(input.schedule) &&
    input.schedule.length === 1592 &&
    digest(input.schedule) === digest(expectedAttempts) &&
    digest(input.schedule) === digest(independentlyCompiled.controllerSchedule);
  if (!valid) {
    addFinding(findings, 'execution_schedule_mismatch', 'executionScheduleSeal', {
      caseCount: 54,
      attemptCount: 1592,
      adaptersReady: true,
      exactControllerSchedule: true,
    }, {
      artifactKind: artifact?.binding.kind ?? null,
      artifactSha256: artifact?.binding.sha256 ?? null,
      seal: seal ?? null,
      caseCount: persisted?.caseCount ?? null,
      attemptCount: persisted?.attemptCount ?? null,
      adaptersReady: persisted?.adapterReadiness?.ready ?? null,
      independentlyCompiled: independentlyCompiled !== null,
      controllerAttemptCount: input.schedule?.length ?? null,
    });
  }
}

function requireArtifactIds(ids, byId, findings, field) {
  for (const artifactId of ids ?? []) {
    if (!byId.has(artifactId)) {
      addFinding(findings, 'referenced_artifact_missing', field, 'known artifactId', artifactId);
    }
  }
}

function buildEnvironmentSeals(environments, byId, findings, notAfter) {
  const seals = [];
  const environmentIds = (environments ?? []).map((entry) => entry.environmentId);
  if (environmentIds.length !== 2 || new Set(environmentIds).size !== 2) {
    addFinding(findings, 'environment_digest_mismatch', 'environments', 'exactly one E1 and one E2 environment', environmentIds);
  }
  for (const environmentId of ['E1', 'E2']) {
    const environment = (environments ?? []).find((entry) => entry.environmentId === environmentId);
    if (!environment) {
      addFinding(findings, 'environment_digest_mismatch', 'environments.environmentId', environmentId, null);
      continue;
    }
    if (!Number.isFinite(Date.parse(environment.sealedAt)) || Date.parse(environment.sealedAt) > Date.parse(notAfter)) {
      addFinding(findings, 'environment_digest_mismatch', `${environmentId}.sealedAt`, `at or before ${notAfter}`, environment.sealedAt ?? null);
    }
    requireArtifactIds(environment.receiptArtifactIds, byId, findings, `${environmentId}.receiptArtifactIds`);
    const identitySha256 = canonicalEnvironmentDigest(environment);
    if (environment.identitySha256 !== identitySha256) {
      addFinding(findings, 'environment_digest_mismatch', `${environmentId}.identitySha256`, identitySha256, environment.identitySha256);
    }
    const sealBody = {
      environmentId,
      identityId: environment.identityId,
      identitySha256,
      sealedAt: environment.sealedAt,
      receiptArtifactIds: [...(environment.receiptArtifactIds ?? [])].sort(),
      ...(environmentId === 'E2' ? {
        externalClientIds: [...(environment.externalClientIds ?? [])].sort(),
        externalVantageArtifactId: environment.externalVantageArtifactId,
        externalHandoffOracleArtifactId: environment.externalHandoffOracleArtifactId,
      } : {}),
    };
    const sealSha256 = canonicalEnvironmentSealDigest(sealBody);
    if (environment.sealSha256 !== sealSha256) {
      addFinding(findings, 'environment_digest_mismatch', `${environmentId}.sealSha256`, sealSha256, environment.sealSha256);
    }
    seals.push({ ...sealBody, sealSha256 });
  }
  return seals;
}

function inspectCalibration(calibration, byId, findings, notAfter) {
  const workloadMatches = Object.entries(CALIBRATION_WORKLOAD).every(
    ([field, value]) => calibration?.workload?.[field] === value,
  );
  const start = Date.parse(calibration?.startedAt);
  const end = Date.parse(calibration?.completedAt);
  if (
    calibration?.clean !== true ||
    !workloadMatches ||
    !Number.isFinite(start) ||
    !Number.isFinite(end) ||
    end < start ||
    end - start < 20 * 60_000 ||
    end > Date.parse(notAfter) ||
    !calibration?.environmentRelativeBudgets ||
    Object.keys(calibration.environmentRelativeBudgets).length === 0
  ) {
    addFinding(findings, 'invalid_calibration', 'calibration', 'clean exact C01 workload completed over at least 20 minutes', calibration ?? null);
  }
  const references = [
    ['rawArtifactId', 'rawArtifactSha256', 'calibration_raw'],
    ['summaryArtifactId', 'summaryArtifactSha256', 'calibration_summary'],
    ['budgetArtifactId', 'budgetSha256', 'calibration_budget'],
  ];
  for (const [idField, shaField, expectedKind] of references) {
    const artifact = byId.get(calibration?.[idField]);
    if (!artifact) {
      addFinding(findings, 'referenced_artifact_missing', `calibration.${idField}`, 'known artifactId', calibration?.[idField] ?? null);
    } else if (calibration?.[shaField] !== artifact.binding.sha256) {
      addFinding(findings, 'invalid_calibration', `calibration.${shaField}`, artifact.binding.sha256, calibration?.[shaField] ?? null);
    } else if (artifact.binding.kind !== expectedKind) {
      addFinding(findings, 'invalid_calibration', `calibration.${idField}`, expectedKind, artifact.binding.kind);
    }
  }
  if (calibration?.declaredSha256 && calibration.declaredSha256 !== canonicalCalibrationDigest(calibration)) {
    addFinding(findings, 'invalid_calibration', 'calibration.declaredSha256', canonicalCalibrationDigest(calibration), calibration.declaredSha256);
  }
}

function inspectExternalVantage(externalVantage, byId, findings) {
  const clients = externalVantage?.clients ?? [];
  const ids = clients.map((client) => client.clientId);
  if (clients.length < 2 || new Set(ids).size !== clients.length) {
    addFinding(findings, 'external_client_identity_invalid', 'externalVantage.clients', 'at least two unique client IDs', ids);
  }
  for (const client of clients) {
    const outside =
      client.outsideServiceHost === true &&
      client.outsideServiceNetworkNamespace === true &&
      client.publicEgressObserved === true &&
      client.hostId &&
      client.hostId !== externalVantage.serviceHostId &&
      client.networkNamespaceId &&
      client.networkNamespaceId !== externalVantage.serviceNetworkNamespaceId;
    if (!outside) {
      addFinding(findings, 'external_client_identity_invalid', `externalVantage.clients.${client.clientId}`, 'off-host and off-network-namespace proof', client);
    }
    for (const evidenceClass of REQUIRED_INGRESS_OBSERVATIONS) {
      const observation = client.ingressObservations?.[evidenceClass];
      if (observation?.state !== 'passed' || !observation.artifactId) {
        addFinding(findings, 'external_ingress_observation_missing', `${client.clientId}.ingressObservations.${evidenceClass}`, 'passed observation with artifactId', observation ?? null);
      } else {
        requireArtifactIds([observation.artifactId], byId, findings, `${client.clientId}.${evidenceClass}.artifactId`);
      }
    }
  }
}

function inspectE2Bindings(environments, externalVantage, byId, findings) {
  const environment = (environments ?? []).find((entry) => entry.environmentId === 'E2');
  if (!environment) return;
  const expectedClientIds = [...new Set((externalVantage?.clients ?? []).map((client) => client.clientId))].sort();
  const observedClientIds = [...(environment.externalClientIds ?? [])].sort();
  if (JSON.stringify(expectedClientIds) !== JSON.stringify(observedClientIds)) {
    addFinding(findings, 'external_client_identity_invalid', 'E2.externalClientIds', expectedClientIds, observedClientIds);
  }
  for (const [field, kind] of [
    ['externalVantageArtifactId', 'external_vantage'],
    ['externalHandoffOracleArtifactId', 'external_handoff_oracle_report'],
  ]) {
    const artifact = byId.get(environment[field]);
    if (artifact && artifact.binding.kind !== kind) {
      addFinding(findings, 'referenced_artifact_missing', `E2.${field}`, kind, artifact.binding.kind);
    }
  }
}

function inspectCandidateBinding(candidate, environments, findings) {
  const observed = (environments ?? []).find((entry) => entry.environmentId === 'E1')?.identity?.candidate;
  const fields = [
    'sourceCommit',
    'binarySha256',
    'dashboardSha256',
    'installedGenerationId',
    'browserExecutableSha256',
    'runtimeManifestRevision',
    'providerConfigurationRevision',
    'externalIngressDeploymentRevision',
    'aggregateFixtureManifestSha256',
  ];
  if (!observed) {
    addFinding(findings, 'candidate_digest_mismatch', 'E1.identity.candidate', 'observed installed candidate identity', null);
    return;
  }
  for (const field of fields) {
    if (candidate?.[field] !== observed[field]) {
      addFinding(findings, 'candidate_digest_mismatch', `candidate.${field}`, observed[field] ?? null, candidate?.[field] ?? null);
    }
  }
}

function inspectAggregateArtifact(kind, value, artifacts, findings, code, field) {
  const entry = artifacts.entries.find((candidate) => candidate.binding.kind === kind);
  const expected = value === undefined ? null : digest(value);
  let observed = null;
  try {
    observed = entry ? digest(JSON.parse(entry.bytes.toString('utf8'))) : null;
  } catch {
    observed = null;
  }
  if (!entry || observed !== expected) {
    addFinding(findings, code, field, expected, observed);
  }
}

function reportFor(input, findings, additions = {}) {
  const finalized = findings
    .sort((left, right) => left.code.localeCompare(right.code) || left.identity.localeCompare(right.identity))
    .map(({ identity, ...finding }, index) => ({
      findingId: `preparation-finding-${String(index + 1).padStart(4, '0')}-${identity.slice(0, 12)}`,
      ...finding,
      repairAttempted: false,
    }));
  return {
    schemaVersion: 'agent-browser.p158-campaign-preparation-report.v1',
    planId: 'P158',
    runId: input.candidate?.runId ?? null,
    inputSha256: digest(without(input, ['controller', 'clock'])),
    passed: finalized.length === 0,
    effectsAttempted: false,
    repairAttempted: false,
    findings: finalized,
    ...additions,
  };
}

export async function prepareAndFreezeCampaign(input, executionContext = undefined) {
  const original = clone(without(input, ['controller', 'clock']));
  const findings = [];
  const preparationObservedAt = input.clock?.wallNow?.() ?? new Date().toISOString();
  const snapshot = input.controller.snapshot();
  if (snapshot.state !== 'prepared' || snapshot.prepared !== false || snapshot.counts.terminal !== 0 || snapshot.results.length !== 0) {
    addFinding(findings, 'controller_not_pristine', 'controller.snapshot', 'unprepared controller with zero terminal results', snapshot);
  }

  if (!['fixture', 'live'].includes(input.campaignMode)) {
    addFinding(findings, 'execution_schedule_mismatch', 'campaignMode', 'fixture or live',
      input.campaignMode ?? null);
  }
  const artifacts = inspectArtifacts(input.artifacts, findings, input.campaignMode);
  inspectExecutionSchedule(input, artifacts, findings, executionContext, snapshot);
  const environmentSeals = buildEnvironmentSeals(input.environments, artifacts.byId, findings, preparationObservedAt);
  inspectCandidateBinding(input.candidate, input.environments, findings);
  if (!Number.isFinite(Date.parse(input.candidate?.preparedAt)) || Date.parse(input.candidate.preparedAt) > Date.parse(preparationObservedAt)) {
    addFinding(findings, 'candidate_digest_mismatch', 'candidate.preparedAt', `at or before ${preparationObservedAt}`, input.candidate?.preparedAt ?? null);
  }
  inspectCalibration(input.calibration, artifacts.byId, findings, preparationObservedAt);
  inspectExternalVantage(input.externalVantage, artifacts.byId, findings);
  inspectE2Bindings(input.environments, input.externalVantage, artifacts.byId, findings);
  inspectAggregateArtifact(
    'external_vantage', input.externalVantage, artifacts, findings,
    'external_ingress_observation_missing', 'externalVantage.artifact',
  );
  inspectAggregateArtifact(
    'external_handoff_oracle_report', input.w4Report, artifacts, findings,
    'w4_oracle_not_clean', 'w4Report.artifact',
  );
  requireArtifactIds(
    [input.environments?.find((entry) => entry.environmentId === 'E2')?.externalVantageArtifactId,
      input.environments?.find((entry) => entry.environmentId === 'E2')?.externalHandoffOracleArtifactId].filter(Boolean),
    artifacts.byId,
    findings,
    'E2.externalArtifacts',
  );

  if (input.w4Report?.passed !== true || (input.w4Report.findings ?? []).length !== 0) {
    addFinding(findings, 'w4_oracle_not_clean', 'w4Report', 'clean passing W4 report', input.w4Report ?? null);
  }
  const fixtureArtifact = artifacts.entries.find((entry) => entry.binding.kind === 'fixture_aggregate_manifest');
  if (!fixtureArtifact || input.fixtureSeal?.aggregateArtifactId !== fixtureArtifact.binding.artifactId ||
      input.fixtureSeal?.aggregateSha256 !== fixtureArtifact.binding.sha256 ||
      input.candidate?.aggregateFixtureManifestSha256 !== fixtureArtifact?.binding.sha256) {
    addFinding(findings, 'fixture_digest_mismatch', 'fixtureSeal.aggregateSha256', fixtureArtifact?.binding.sha256 ?? null, {
      fixtureSeal: input.fixtureSeal?.aggregateSha256 ?? null,
      candidate: input.candidate?.aggregateFixtureManifestSha256 ?? null,
    });
  }
  if (input.candidate?.candidateSha256 && input.candidate.candidateSha256 !== canonicalCandidateDigest(input.candidate)) {
    addFinding(findings, 'candidate_digest_mismatch', 'candidate.candidateSha256', canonicalCandidateDigest(input.candidate), input.candidate.candidateSha256);
  }
  if (JSON.stringify(without(input, ['controller', 'clock'])) !== JSON.stringify(original)) {
    throw new Error('P158 preparation inspection mutated its input');
  }
  if (findings.length > 0) return reportFor(input, findings);

  const artifactBindings = artifacts.entries.map((entry) => entry.binding);
  const prepared = await input.controller.prepare({
    candidate: input.candidate,
    schedule: input.schedule,
    scheduledTeardown: input.scheduledTeardown,
    artifactBindings,
    environmentSeals,
    calibration: without(input.calibration, ['declaredSha256']),
    fixtureSeal: input.fixtureSeal,
    freezeContract: input.freezeContract,
  });
  if (prepared.counts.terminal !== 0 || prepared.results.length !== 0 || prepared.state !== 'prepared') {
    throw new Error('P158 preparation started or terminalized a campaign attempt before freeze');
  }
  if (input.campaignMode === 'live' && prepared.counts.total !== input.executionScheduleSeal.attemptCount) {
    throw new Error('P158 controller was not initialized from the sealed full execution schedule');
  }
  for (const entry of artifacts.entries) {
    await input.controller.writeArtifact({
      artifactId: entry.binding.artifactId,
      relativePath: entry.binding.relativePath,
      content: entry.bytes,
      metadata: {
        mediaType: entry.source.mediaType ?? 'application/octet-stream',
        capturePurpose: entry.binding.kind,
        captureState: 'complete',
      },
    });
  }
  const integrity = await input.controller.verifyIntegrity();
  if (!integrity.valid) throw new Error('P158 preparation artifact integrity failed before freeze');
  const frozen = await input.controller.freeze();
  if (frozen.state !== 'frozen' || frozen.counts.terminal !== 0 || frozen.results.length !== 0 ||
      frozen.freezeReceipt?.startedCaseCount !== 0 || frozen.freezeReceipt?.startedAttemptCount !== 0) {
    throw new Error('P158 freeze did not preserve the zero-start execution gate');
  }
  return reportFor(input, [], {
    artifactBindings,
    environmentSeals,
    calibrationSha256: canonicalCalibrationDigest(input.calibration),
    candidateSha256: canonicalCandidateDigest(input.candidate),
    fixtureSealSha256: digest(input.fixtureSeal),
    externalVantageSha256: digest(input.externalVantage),
    w4ReportSha256: digest(input.w4Report),
    ...(input.campaignMode === 'live'
      ? { executionScheduleSha256: input.executionScheduleSeal.scheduleSha256 }
      : {}),
    freezeReceipt: clone(frozen.freezeReceipt),
    controllerState: frozen.state,
    zeroStartedCaseCount: frozen.freezeReceipt.startedCaseCount,
    zeroStartedAttemptCount: frozen.freezeReceipt.startedAttemptCount,
  });
}
