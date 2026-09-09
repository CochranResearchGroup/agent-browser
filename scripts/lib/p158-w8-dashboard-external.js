import { sha256 } from './p158-campaign-controller.js';
import {
  assertP158DashboardScenarioPlan,
  auditP158DashboardScenarioReceipt,
} from './p158-w8-dashboard-scenarios.js';

const SHA256 = /^[a-f0-9]{64}$/u;
const COMMIT = /^[a-f0-9]{40}$/u;

export class P158W8DashboardExternalError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'P158W8DashboardExternalError';
    this.code = code;
  }
}

function fail(code, message) {
  throw new P158W8DashboardExternalError(code, message);
}

function without(value, field) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== field));
}

function validGithubRunnerAttestation(runnerAttestation) {
  const { attestationSha256, ...body } = runnerAttestation ?? {};
  return runnerAttestation?.schemaVersion === 'agent-browser.p158-dashboard-github-runner-attestation.v1' &&
    runnerAttestation.provider === 'github_actions' && runnerAttestation.runnerEnvironment === 'github-hosted' &&
    runnerAttestation.runnerOs === 'Linux' && ['X64', 'ARM64'].includes(runnerAttestation.runnerArch) &&
    SHA256.test(runnerAttestation.runIdSha256 ?? '') && Number.isInteger(runnerAttestation.runAttempt) &&
    runnerAttestation.runAttempt > 0 && runnerAttestation.offHost === true &&
    runnerAttestation.outsideServiceHost === true &&
    runnerAttestation.outsideServiceNetworkNamespace === true && attestationSha256 === sha256(body);
}

function validateProjectionExternalProof({ projection, manifest, runnerAttestation }) {
  const { proofSha256, ...proofBody } = projection?.externalProof ?? {};
  if (projection?.externalProof?.schemaVersion !== 'agent-browser.p158-dashboard-external-proof.v1' ||
      projection.externalProof.source !== 'validated_external_runner' ||
      projection.externalProof.runnerAttestationSchemaVersion !== runnerAttestation.schemaVersion ||
      projection.externalProof.runnerAttestationSha256 !== runnerAttestation.attestationSha256 ||
      projection.externalProof.publicUrlSha256 !== manifest.publicUrlSha256 ||
      projection.externalProof.offHost !== true || projection.externalProof.outsideServiceHost !== true ||
      projection.externalProof.outsideServiceNetworkNamespace !== true ||
      projection.externalProof.publicHttps !== true ||
      projection.externalProof.operatorVisibleState !== 'ready' ||
      proofSha256 !== sha256(proofBody)) {
    fail('external_result_invalid', 'External projection proof is not bound to its runner and dashboard URL');
  }
}

/** Seal one synthetic-only dashboard action for manual off-host execution. */
export function buildP158DashboardExternalManifest({
  expectedCommit,
  campaignPlanSha256,
  candidateSha256,
  scenarioPlan,
  expectedState,
  materializationReceipt,
  publicUrlSha256,
  publicPath,
  selectionReceiptSha256,
}) {
  assertP158DashboardScenarioPlan(scenarioPlan);
  if (!COMMIT.test(expectedCommit ?? '') || !SHA256.test(campaignPlanSha256 ?? '') ||
      !SHA256.test(candidateSha256 ?? '') || !SHA256.test(publicUrlSha256 ?? '') ||
      !SHA256.test(selectionReceiptSha256 ?? '') || typeof publicPath !== 'string' ||
      !publicPath.startsWith('/p158/') || !['D03', 'D04', 'D05'].includes(scenarioPlan.caseId) ||
      (scenarioPlan.caseId === 'D05' && scenarioPlan.scenarioTruth.executable !== true) ||
      materializationReceipt?.stateSha256 !== sha256(expectedState) ||
      materializationReceipt?.receiptSha256 !== sha256(without(materializationReceipt, 'receiptSha256')) ||
      scenarioPlan.stateSha256 !== materializationReceipt.stateSha256) {
    fail('external_manifest_invalid', 'External dashboard manifest is incomplete, changed, blocked, or unsafe');
  }
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-external-manifest.v1',
    planId: 'P158',
    expectedCommit,
    campaignPlanSha256,
    candidateSha256,
    actionId: scenarioPlan.actionId,
    caseId: scenarioPlan.caseId,
    scenarioPlan: structuredClone(scenarioPlan),
    expectedState: structuredClone(expectedState),
    materializationReceipt: structuredClone(materializationReceipt),
    publicUrlSha256,
    publicPath,
    selectionReceiptSha256,
    syntheticOnly: true,
    offHostRequired: true,
    outsideServiceNetworkNamespaceRequired: true,
    repairAllowed: false,
    retryAllowed: false,
    garbageCollectionAllowed: false,
  };
  return { ...body, manifestSha256: sha256(body) };
}

export function validateP158DashboardExternalManifest(manifest) {
  const { manifestSha256, ...body } = manifest ?? {};
  if (manifest?.schemaVersion !== 'agent-browser.p158-dashboard-external-manifest.v1' ||
      manifestSha256 !== sha256(body)) {
    fail('external_manifest_invalid', 'External dashboard manifest is missing or changed');
  }
  const rebuilt = buildP158DashboardExternalManifest(body);
  if (rebuilt.manifestSha256 !== manifestSha256) {
    fail('external_manifest_invalid', 'External dashboard manifest does not match its canonical contract');
  }
  return manifest;
}

/** Validate the secret action URL without persisting it in result evidence. */
export function validateP158DashboardExternalActionUrl({ manifest, publicUrl }) {
  validateP158DashboardExternalManifest(manifest);
  let parsed;
  try {
    parsed = new URL(publicUrl);
  } catch {
    fail('external_url_invalid', 'External dashboard action URL is invalid');
  }
  const hostname = parsed.hostname.toLowerCase();
  if (parsed.protocol !== 'https:' || sha256(publicUrl) !== manifest.publicUrlSha256 ||
      (parsed.pathname !== manifest.publicPath && !parsed.pathname.startsWith(`${manifest.publicPath}/`)) ||
      hostname === 'localhost' ||
      hostname.endsWith('.localhost') || hostname.endsWith('.local') ||
      /^(?:127\.|10\.|192\.168\.|169\.254\.|0\.)/u.test(hostname) ||
      /^172\.(?:1[6-9]|2\d|3[01])\./u.test(hostname) || hostname === '::1') {
    fail('external_url_invalid', 'External dashboard action URL is not the exact reviewed public HTTPS route');
  }
  return parsed.href;
}

export function buildP158DashboardGithubRunnerAttestation(environment) {
  if (environment.GITHUB_ACTIONS !== 'true' || environment.RUNNER_ENVIRONMENT !== 'github-hosted' ||
      environment.RUNNER_OS !== 'Linux' || !['X64', 'ARM64'].includes(environment.RUNNER_ARCH) ||
      !/^\d+$/u.test(environment.GITHUB_RUN_ID ?? '') || !/^\d+$/u.test(environment.GITHUB_RUN_ATTEMPT ?? '')) {
    fail('external_runner_invalid', 'External capture requires an exact GitHub-hosted Linux runner identity');
  }
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-github-runner-attestation.v1',
    provider: 'github_actions',
    runnerEnvironment: 'github-hosted',
    runnerOs: environment.RUNNER_OS,
    runnerArch: environment.RUNNER_ARCH,
    runIdSha256: sha256(environment.GITHUB_RUN_ID),
    runAttempt: Number(environment.GITHUB_RUN_ATTEMPT),
    offHost: true,
    outsideServiceHost: true,
    outsideServiceNetworkNamespace: true,
  };
  return { ...body, attestationSha256: sha256(body) };
}

export function sealP158DashboardExternalResult({
  manifest,
  scenarioReceipt = null,
  projection = null,
  dashboardFixture = null,
  oracleBinding = null,
  runnerAttestation = null,
  failure = null,
}) {
  validateP158DashboardExternalManifest(manifest);
  if (failure && (typeof failure.code !== 'string' || !failure.code ||
      typeof failure.message !== 'string' || /(?:https?|wss?|file|data|javascript):/iu.test(failure.message))) {
    fail('external_result_invalid', 'External failure classification is missing or exposes URL material');
  }
  let scenarioOracle = null;
  if (failure === null) {
    if (!validGithubRunnerAttestation(runnerAttestation)) {
      fail('external_runner_invalid', 'Successful external capture lacks GitHub-hosted runner proof');
    }
    validateProjectionExternalProof({ projection, manifest, runnerAttestation });
    scenarioOracle = auditP158DashboardScenarioReceipt({
      plan: manifest.scenarioPlan,
      receipt: scenarioReceipt,
    });
  }
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-external-result.v1',
    planId: 'P158',
    manifestSha256: manifest.manifestSha256,
    campaignPlanSha256: manifest.campaignPlanSha256,
    candidateSha256: manifest.candidateSha256,
    actionId: manifest.actionId,
    caseId: manifest.caseId,
    scenarioReceipt,
    scenarioOracle,
    projection,
    dashboardFixture,
    oracleBinding,
    runnerAttestation,
    failure,
    terminalState: 'completed',
    resultState: failure === null ? 'passed' : 'harness_failure',
    success: failure === null,
    productionStateTouched: false,
    repairAttempted: false,
    retryAttempted: false,
    garbageCollectionAttempted: false,
  };
  return { ...body, resultSha256: sha256(body) };
}

export function validateP158DashboardExternalResult({ result, manifest }) {
  validateP158DashboardExternalManifest(manifest);
  const { resultSha256, ...body } = result ?? {};
  if (result?.schemaVersion !== 'agent-browser.p158-dashboard-external-result.v1' ||
      result.manifestSha256 !== manifest.manifestSha256 || result.actionId !== manifest.actionId ||
      result.candidateSha256 !== manifest.candidateSha256 || result.terminalState !== 'completed' ||
      resultSha256 !== sha256(body)) {
    fail('external_result_invalid', 'External dashboard result is missing, changed, or foreign');
  }
  if (result.success) {
    auditP158DashboardScenarioReceipt({ plan: manifest.scenarioPlan, receipt: result.scenarioReceipt });
    if (result.resultState !== 'passed' || result.failure !== null || result.scenarioOracle?.passed !== true ||
        !validGithubRunnerAttestation(result.runnerAttestation)) {
      fail('external_result_invalid', 'Successful external result lacks its exact scenario oracle');
    }
    validateProjectionExternalProof({
      projection: result.projection,
      manifest,
      runnerAttestation: result.runnerAttestation,
    });
  } else if (result.resultState !== 'harness_failure' || !result.failure?.code) {
    fail('external_result_invalid', 'Failed external result lacks an append-only failure classification');
  }
  return result;
}

export function externalResultBody(result) {
  return without(result, 'resultSha256');
}
