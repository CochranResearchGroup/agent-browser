#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('..', import.meta.url).pathname;
const outputPath = join(root, 'docs/dev/fixtures/p158/campaign-preparation.v1.json');
const artifactKinds = [
  'installation_receipt', 'runtime_status_receipt', 'runtime_doctor_receipt', 'runtime_manifest',
  'provider_plan_receipt', 'provider_stage_receipt', 'provider_preflight_receipt',
  'provider_apply_receipt', 'provider_status_receipt', 'provider_doctor_receipt',
  'provider_configuration', 'external_ingress_deployment', 'external_vantage',
  'external_handoff_oracle_report', 'calibration_raw', 'calibration_summary',
  'calibration_budget', 'fixture_aggregate_manifest',
];
const ingressClasses = [
  'dns', 'tls', 'redirect', 'cookie', 'websocket', 'iframe', 'form_action', 'reconnect',
];

function canonicalize(value) {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.map(canonicalize);
  return Object.fromEntries(
    Object.keys(value).sort().filter((key) => value[key] !== undefined)
      .map((key) => [key, canonicalize(value[key])]),
  );
}

function canonicalJson(value) {
  return `${JSON.stringify(canonicalize(value))}\n`;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function digest(value) {
  return sha256(canonicalJson(value));
}

function clone(value) {
  return structuredClone(value);
}

function artifact(kind, index) {
  const raw = `synthetic ${kind} receipt ${index + 1}\n`;
  const contentEncoding = kind === 'runtime_manifest' ? 'base64' : 'utf8';
  const content = contentEncoding === 'base64' ? Buffer.from(raw).toString('base64') : raw;
  const bytes = Buffer.from(content, contentEncoding);
  return {
    artifactId: `artifact-${String(index + 1).padStart(2, '0')}`,
    kind,
    relativePath: `freeze/${kind}.json`,
    capturedAt: '2026-09-02T20:00:05.000Z',
    mediaType: 'application/json',
    contentEncoding,
    content,
    declaredSha256: sha256(bytes),
    declaredByteCount: bytes.length,
  };
}

function replaceAggregateArtifact(input, kind, value) {
  const target = input.artifacts.find((entry) => entry.kind === kind);
  target.contentEncoding = 'utf8';
  target.content = canonicalJson(value);
  target.declaredSha256 = sha256(target.content);
  target.declaredByteCount = Buffer.byteLength(target.content);
}

function makeBaseline() {
  const artifacts = artifactKinds.map(artifact);
  const byKind = (kind) => artifacts.find((entry) => entry.kind === kind);
  const w4Report = {
    schemaVersion: 'agent-browser.p158-external-handoff-oracle-report.v1',
    planId: 'P158',
    auditId: 'p158-w6-clean-w4',
    fixtureId: 'clean-public-https',
    inputSha256: '91'.repeat(32),
    auditedAt: '2026-09-02T20:00:00.000Z',
    repairAttempted: false,
    passed: true,
    summary: { findingCount: 0 },
    urlClassifications: [],
    findings: [],
  };
  const ingressObservations = Object.fromEntries(
    ingressClasses.map((kind) => [kind, {
      state: 'passed', artifactId: byKind('external_vantage').artifactId,
    }]),
  );
  const externalVantage = {
    serviceHostId: 'service-host-p158',
    serviceNetworkNamespaceId: 'service-namespace-p158',
    clients: [1, 2].map((ordinal) => ({
      clientId: `external-runner-${ordinal}`,
      hostId: `external-host-${ordinal}`,
      networkNamespaceId: `external-namespace-${ordinal}`,
      outsideServiceHost: true,
      outsideServiceNetworkNamespace: true,
      publicEgressObserved: true,
      ingressObservations: clone(ingressObservations),
    })),
  };
  const fixtureAggregate = byKind('fixture_aggregate_manifest');
  fixtureAggregate.content = canonicalJson({
    schemaVersion: 'agent-browser.p158-fixture-aggregate.v1',
    entries: ['registry', 'logging', 'external', 'dashboard'],
  });
  fixtureAggregate.declaredSha256 = sha256(fixtureAggregate.content);
  fixtureAggregate.declaredByteCount = Buffer.byteLength(fixtureAggregate.content);
  const candidate = {
    runId: 'p158-w6-synthetic',
    sourceCommit: 'e26a6b05c315cfed06a833a5c4d7406803bcc0fb',
    binarySha256: '11'.repeat(32),
    dashboardSha256: '22'.repeat(32),
    installedGenerationId: 'development-generation-p158',
    browserExecutableSha256: '33'.repeat(32),
    runtimeManifestRevision: 'runtime-manifest-p158-v1',
    providerConfigurationRevision: 'provider-configuration-p158-v1',
    externalIngressDeploymentRevision: 'external-ingress-p158-v1',
    aggregateFixtureManifestSha256: fixtureAggregate.declaredSha256,
    preparedAt: '2026-09-02T19:30:00.000Z',
  };
  candidate.candidateSha256 = digest(candidate);
  const calibration = {
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
    rawArtifactId: byKind('calibration_raw').artifactId,
    rawArtifactSha256: byKind('calibration_raw').declaredSha256,
    summaryArtifactId: byKind('calibration_summary').artifactId,
    summaryArtifactSha256: byKind('calibration_summary').declaredSha256,
    budgetArtifactId: byKind('calibration_budget').artifactId,
    budgetSha256: byKind('calibration_budget').declaredSha256,
    environmentRelativeBudgets: { agentCommandP95Ms: 750, handoffPixelsP95Ms: 8000 },
  };
  calibration.declaredSha256 = digest(calibration);
  const e1 = {
    environmentId: 'E1',
    identityId: 'development-runtime-e1',
    identity: { environment: 'development', runtimeLane: 'development-default', candidate: clone(candidate) },
    sealedAt: '2026-09-02T20:00:10.000Z',
    receiptArtifactIds: [
      'installation_receipt', 'runtime_status_receipt', 'runtime_doctor_receipt', 'runtime_manifest',
    ].map((kind) => byKind(kind).artifactId),
  };
  e1.identitySha256 = digest(e1.identity);
  e1.sealSha256 = digest({
    environmentId: e1.environmentId,
    identityId: e1.identityId,
    identitySha256: e1.identitySha256,
    sealedAt: e1.sealedAt,
    receiptArtifactIds: [...e1.receiptArtifactIds].sort(),
  });
  const e2 = {
    environmentId: 'E2',
    identityId: 'external-presentation-e2',
    identity: {
      environment: 'development',
      provider: 'isolated-guacamole',
      ingressScheme: 'https',
      ingressRevision: candidate.externalIngressDeploymentRevision,
    },
    sealedAt: '2026-09-02T20:00:15.000Z',
    receiptArtifactIds: [
      'provider_plan_receipt', 'provider_stage_receipt', 'provider_preflight_receipt',
      'provider_apply_receipt', 'provider_status_receipt', 'provider_doctor_receipt',
      'provider_configuration', 'external_ingress_deployment', 'external_vantage',
      'external_handoff_oracle_report',
    ].map((kind) => byKind(kind).artifactId),
    externalClientIds: ['external-runner-1', 'external-runner-2'],
    externalVantageArtifactId: byKind('external_vantage').artifactId,
    externalHandoffOracleArtifactId: byKind('external_handoff_oracle_report').artifactId,
  };
  e2.identitySha256 = digest(e2.identity);
  e2.sealSha256 = digest({
    environmentId: e2.environmentId,
    identityId: e2.identityId,
    identitySha256: e2.identitySha256,
    sealedAt: e2.sealedAt,
    receiptArtifactIds: [...e2.receiptArtifactIds].sort(),
    externalClientIds: [...e2.externalClientIds].sort(),
    externalVantageArtifactId: e2.externalVantageArtifactId,
    externalHandoffOracleArtifactId: e2.externalHandoffOracleArtifactId,
  });
  const input = {
    candidate,
    artifacts,
    environments: [e1, e2],
    calibration,
    externalVantage,
    w4Report,
    fixtureSeal: {
      aggregateArtifactId: fixtureAggregate.artifactId,
      aggregateSha256: fixtureAggregate.declaredSha256,
      entryCount: 4,
      includedPaths: [
        'docs/dev/contracts/p158-historical-failure-registry.v1.json',
        'docs/dev/fixtures/p158/logging-causal-envelopes.v1.json',
        'docs/dev/fixtures/p158/external-handoff-sessions.v1.json',
        'docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json',
      ],
    },
    freezeContract: {
      freezeId: 'p158-w6-synthetic:freeze',
      requiredStartedCaseCount: 0,
      requiredStartedAttemptCount: 0,
    },
    schedule: [{ caseId: 'A01', attemptId: 'A01-E1-001', environmentId: 'E1', dependsOn: [] }],
    scheduledTeardown: {
      caseId: 'TEARDOWN', attemptId: 'TEARDOWN-E1', environmentId: 'E1', dependsOn: [],
    },
  };
  replaceAggregateArtifact(input, 'external_vantage', externalVantage);
  replaceAggregateArtifact(input, 'external_handoff_oracle_report', w4Report);
  return input;
}

const baseline = makeBaseline();
const fixtures = [];
function add(fixtureId, expectedFindingCodes, mutate, controllerSetup = 'pristine') {
  const input = clone(baseline);
  if (mutate) mutate(input);
  fixtures.push({ fixtureId, controllerSetup, input, expectedFindingCodes });
}

add('clean-freeze-ready', [], null);
add('artifact-byte-count-mismatch', ['artifact_byte_count_mismatch'], (input) => {
  input.artifacts[0].declaredByteCount += 1;
});
add('artifact-hash-mismatch', ['artifact_hash_mismatch'], (input) => {
  input.artifacts[0].declaredSha256 = 'ff'.repeat(32);
});
add('candidate-digest-mismatch', ['candidate_digest_mismatch'], (input) => {
  input.candidate.candidateSha256 = 'ff'.repeat(32);
});
add('candidate-install-binding-mismatch', ['candidate_digest_mismatch'], (input) => {
  input.candidate.sourceCommit = 'f'.repeat(40);
  input.candidate.candidateSha256 = digest(input.candidate);
});
add('controller-not-pristine', ['controller_not_pristine'], null, 'already_prepared');
add('duplicate-artifact-id', ['duplicate_artifact_id'], (input) => {
  input.artifacts.push(clone(input.artifacts[0]));
});
add('environment-digest-mismatch', ['environment_digest_mismatch'], (input) => {
  input.environments[0].identitySha256 = 'ff'.repeat(32);
});

const externalClientMutations = [
  ['duplicate-external-client', (input) => {
    input.externalVantage.clients[1].clientId = input.externalVantage.clients[0].clientId;
  }],
  ['external-client-on-service-host', (input) => {
    input.externalVantage.clients[0].hostId = input.externalVantage.serviceHostId;
    input.externalVantage.clients[0].outsideServiceHost = false;
  }],
  ['external-client-in-service-namespace', (input) => {
    input.externalVantage.clients[0].networkNamespaceId = input.externalVantage.serviceNetworkNamespaceId;
    input.externalVantage.clients[0].outsideServiceNetworkNamespace = false;
  }],
  ['external-client-public-egress-unproven', (input) => {
    input.externalVantage.clients[0].publicEgressObserved = false;
  }],
];
for (const [fixtureId, mutate] of externalClientMutations) {
  add(fixtureId, ['external_client_identity_invalid'], (input) => {
    mutate(input);
    replaceAggregateArtifact(input, 'external_vantage', input.externalVantage);
  });
}
for (const evidenceClass of ingressClasses) {
  add(`${evidenceClass}-proof-missing`, ['external_ingress_observation_missing'], (input) => {
    input.externalVantage.clients[0].ingressObservations[evidenceClass].state = 'missing';
    replaceAggregateArtifact(input, 'external_vantage', input.externalVantage);
  });
}
add('fixture-digest-mismatch', ['fixture_digest_mismatch'], (input) => {
  input.fixtureSeal.aggregateSha256 = 'ff'.repeat(32);
});
add('invalid-calibration-order', ['invalid_calibration'], (input) => {
  input.calibration.completedAt = '2026-09-02T19:39:59.000Z';
  input.calibration.declaredSha256 = digest(input.calibration);
});
add('invalid-calibration-digest', ['invalid_calibration'], (input) => {
  input.calibration.declaredSha256 = 'ff'.repeat(32);
});
add('missing-artifact-kind', ['missing_artifact_kind'], (input) => {
  input.artifacts.find((entry) => entry.kind === 'provider_doctor_receipt').kind = 'provider_status_receipt';
});
add('referenced-artifact-missing', ['referenced_artifact_missing'], (input) => {
  const environment = input.environments[0];
  environment.receiptArtifactIds[0] = 'absent-install-receipt';
  environment.sealSha256 = digest({
    environmentId: environment.environmentId,
    identityId: environment.identityId,
    identitySha256: environment.identitySha256,
    sealedAt: environment.sealedAt,
    receiptArtifactIds: [...environment.receiptArtifactIds].sort(),
  });
});
add('w4-oracle-not-clean', ['w4_oracle_not_clean'], (input) => {
  input.w4Report.passed = false;
  input.w4Report.findings = [{ code: 'capture_gap' }];
});

writeFileSync(outputPath, `${JSON.stringify({
  schemaVersion: 'agent-browser.p158-campaign-preparation-fixtures.v1',
  planId: 'P158',
  fixtures,
}, null, 2)}\n`);
process.stdout.write(`Wrote ${fixtures.length} P158 campaign preparation fixtures to ${outputPath}\n`);
