import { createHash } from 'node:crypto';
import { lstatSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import {
  canonicalJson,
  sha256,
} from './p158-campaign-controller.js';
import { createP158CaseAdapter } from './p158-execution-schedule.js';
import {
  parseEnvText,
} from './p47-viewer-client.js';
import {
  EXTERNAL_HANDOFF_FINDING_CODES,
  REQUIRED_INGRESS_CHECKS,
  stableHandoffHash,
} from './p158-external-handoff-oracle.js';

const SHA256 = /^[a-f0-9]{64}$/u;
const ASSEMBLER_SOURCE_PATH = 'scripts/lib/p158-w6-evidence-assembler.js';

const HOOK_SOURCE_PATHS = Object.freeze({
  'p158.live_bundle_assembly': 'scripts/lib/p158-live-campaign-assembly.js',
  'p158.logging_evidence_harvest': 'scripts/lib/p158-logging-evidence-harvester.js',
  'w7.agent_existing_seam_workflow': 'scripts/lib/p158-w7-agent-orchestration.js',
  'w7.a01_a03.service_concurrency': 'scripts/lib/p158-w7-a01-a03-live.js',
  'w7.a04_a06.profile_policy': 'scripts/lib/p158-w7-a04-a06-live.js',
  'w7.a07_a13.retained_generation': 'scripts/lib/p158-w7-a07-a13-live.js',
  'w7.browser': 'scripts/lib/p158-w7-development-adapters.js',
  'w7.cli': 'scripts/lib/p158-w7-development-adapters.js',
  'w7.display': 'scripts/lib/p158-w7-development-adapters.js',
  'w7.evidence': 'scripts/lib/p158-w7-development-adapters.js',
  'w7.logs': 'scripts/lib/p158-w7-development-adapters.js',
  'w7.process': 'scripts/lib/p158-w7-development-adapters.js',
  'w7.shutdown': 'scripts/lib/p158-w7-development-adapters.js',
  'w7.systemd': 'scripts/lib/p158-w7-development-adapters.js',
  'w8.dashboard_capture': 'scripts/lib/p158-w8-hd-adapters.js',
  'w8.dashboard_execute': 'scripts/lib/p158-w8-hd-adapters.js',
  'w8.external_workflow': 'scripts/run-p158-external-vantage.js',
  'w8.playwright': 'scripts/run-p158-external-vantage.js',
  'w8.stimulus': 'scripts/lib/p158-w8-hd-adapters.js',
  'w9.browser_crash': 'scripts/lib/p158-w9-concrete-drivers.js',
  'w9.external_dashboard_action': 'scripts/lib/p158-w9-concrete-drivers.js',
  'w9.external_handoff_reconnect': 'scripts/lib/p158-w9-concrete-drivers.js',
  'w9.service_command': 'scripts/lib/p158-w9-concrete-drivers.js',
  'w9.supervisor_transition': 'scripts/lib/p158-w9-concrete-drivers.js',
});

export class P158W6EvidenceAssemblerError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W6EvidenceAssemblerError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W6EvidenceAssemblerError(code, message, details);
}

function fileSha256(repoRoot, sourcePath) {
  const entry = resolve(repoRoot, sourcePath);
  return createHash('sha256').update(readFileSync(entry)).digest('hex');
}

function sourcePathForCase(caseId) {
  if (caseId === 'A05') return 'scripts/lib/p158-w7-a04-a06-live.js';
  if (caseId === 'A13') return 'scripts/lib/p158-w7-a07-a13-live.js';
  if (caseId.startsWith('A') || caseId.startsWith('X')) {
    return 'scripts/lib/p158-w7-development-adapters.js';
  }
  if (caseId.startsWith('H') || caseId.startsWith('D')) {
    return 'scripts/lib/p158-w8-hd-adapters.js';
  }
  return 'scripts/lib/p158-w9-concrete-drivers.js';
}

function hookIdsForCase(caseId) {
  if (['A01', 'A02', 'A03'].includes(caseId)) return ['w7.a01_a03.service_concurrency'];
  if (caseId === 'A05') return ['w7.a04_a06.profile_policy'];
  if (caseId === 'A13') return ['w7.a07_a13.retained_generation'];
  if (caseId.startsWith('A') || caseId.startsWith('X')) return ['w7.cli'];
  if (caseId.startsWith('H')) return ['w8.external_workflow', 'w8.playwright', 'w8.stimulus'];
  if (caseId.startsWith('D')) {
    return ['w8.external_workflow', 'w8.dashboard_execute', 'w8.dashboard_capture', 'w8.stimulus'];
  }
  return ['w9.service_command'];
}

function actionCount(schedule, caseId) {
  return schedule.attempts.filter((attempt) => attempt.caseId === caseId).reduce((total, attempt) => {
    const allocated = attempt.cardinalityAllocations.reduce(
      (count, allocation) => count + allocation.actionIds.length,
      0,
    );
    return total + Math.max(1, allocated);
  }, 0);
}

function manifestDigest(value) {
  const body = Object.fromEntries(Object.entries(value).filter(([key]) => key !== 'manifestSha256'));
  return sha256(body);
}

/**
 * Assemble every Plan 0158 case and hook binding from frozen source metadata.
 * Cases remain explicit zero-effect blockers until their phase runner supplies
 * its separately frozen live execution bundle.
 */
export function assembleP158W6LiveBindings({ schedule, candidate, aggregate, runId, capturedAt }) {
  if (schedule?.caseCount !== 54 || schedule.caseContracts?.length !== 54 ||
      !SHA256.test(schedule.scheduleSha256 ?? '') || candidate?.candidateSha256 === undefined ||
      aggregate?.sha256 === undefined || !Number.isFinite(Date.parse(capturedAt))) {
    fail('w6_assembly_input_invalid', 'W6 assembly requires the exact schedule, candidate, aggregate, and capture time');
  }
  const repoRoot = resolve(new URL('../..', import.meta.url).pathname);
  const aggregateEntries = new Map(aggregate.manifest.entries.map((entry) => [entry.path, entry]));
  const sealedSource = (sourcePath) => {
    const entry = aggregateEntries.get(sourcePath);
    const actualSha256 = fileSha256(repoRoot, sourcePath);
    if (!entry || entry.sha256 !== actualSha256) {
      fail('w6_source_unsealed', `${sourcePath} is absent from the aggregate or changed`);
    }
    return actualSha256;
  };
  const hookBindings = Object.entries(HOOK_SOURCE_PATHS).sort(([left], [right]) => left.localeCompare(right))
    .map(([hookId, sourcePath]) => ({
      hookId, implementationKind: 'concrete_live', sourcePath, sourceSha256: sealedSource(sourcePath),
    }));
  const adapterBindings = schedule.caseContracts.map((contract) => {
    const sourcePath = sourcePathForCase(contract.caseId);
    return {
      caseId: contract.caseId,
      adapterId: contract.adapterId,
      executionContractSha256: contract.executionContractSha256,
      mode: 'explicit_blocked',
      sourcePath,
      sourceSha256: sealedSource(sourcePath),
      providerFree: false,
      hookIds: hookIdsForCase(contract.caseId),
      implementedActionCount: 0,
      blockedActionCount: actionCount(schedule, contract.caseId),
      effectsAllowed: false,
      blocker: {
        code: 'phase_live_bundle_not_frozen',
        detail: `${contract.caseId} remains zero-effect until its source-owned phase live bundle is frozen`,
      },
    };
  });
  const liveHookManifest = {
    schemaVersion: 'agent-browser.p158-live-hook-manifest.v1',
    planId: 'P158',
    manifestId: `${runId}:live-hooks`,
    capturedAt,
    mode: 'concrete_live',
    providerFree: false,
    aggregateSha256: aggregate.sha256,
    scheduleSha256: schedule.scheduleSha256,
    candidateSha256: candidate.candidateSha256,
    hookBindings,
    adapterBindings,
    repairAllowed: false,
    retryAllowed: false,
    garbageCollectionAllowed: false,
  };
  liveHookManifest.manifestSha256 = manifestDigest(liveHookManifest);
  const contracts = new Map(schedule.caseContracts.map((entry) => [entry.caseId, entry]));
  const adapters = adapterBindings.map((binding) => {
    const contract = contracts.get(binding.caseId);
    const blocker = Object.freeze({
      ...structuredClone(binding.blocker), sourcePath: binding.sourcePath, sourceSha256: binding.sourceSha256,
    });
    const base = createP158CaseAdapter({
      caseId: binding.caseId,
      evidenceProfile: contract.evidenceProfile,
      executionContract: contract.executionContract,
      execute: async () => ({
        resultState: 'skipped_blocked', blocker: structuredClone(blocker), effectState: 'not_started',
        requestedEffects: [], retryAttempted: false, repairAttempted: false,
        garbageCollectionAttempted: false,
      }),
    });
    return Object.freeze({
      ...base,
      executionMode: binding.mode,
      providerFree: false,
      effectsAllowed: false,
      sourcePath: binding.sourcePath,
      sourceSha256: binding.sourceSha256,
      liveHookIds: Object.freeze([...binding.hookIds]),
      blocker,
      liveBindingSha256: sha256(binding),
      liveHookManifestSha256: liveHookManifest.manifestSha256,
    });
  });
  return Object.freeze({ liveHookManifest: Object.freeze(liveHookManifest), adapters: Object.freeze(adapters) });
}

function assertOracleReport(report) {
  if (report?.schemaVersion !== 'agent-browser.p158-external-handoff-oracle-report.v1' ||
      report.planId !== 'P158' || report.repairAttempted !== false || report.passed !== true ||
      !Array.isArray(report.urlClassifications) || !Array.isArray(report.findings)) {
    fail('external_oracle_report_invalid', 'Each external client requires its complete passing oracle report');
  }
}

/** Project downloaded external receipts into the exact W6 preparation shapes. */
export function projectP158W6ExternalEvidence({
  externalAggregate, externalReceipts, oracleReports, serviceHostId,
  serviceNetworkNamespaceId, artifactId,
}) {
  if (externalAggregate?.schemaVersion !== 'agent-browser.p158-external-vantage-aggregate.v1' ||
      externalAggregate.success !== true || !Array.isArray(externalReceipts) ||
      externalReceipts.length !== 2 || !Array.isArray(oracleReports) || oracleReports.length !== 2) {
    fail('external_evidence_incomplete', 'W6 projection requires one passing aggregate, two receipts, and two complete oracle reports');
  }
  for (const report of oracleReports) assertOracleReport(report);
  const receipts = [...externalReceipts].sort((left, right) => left.clientId.localeCompare(right.clientId));
  const clientIds = receipts.map((receipt) => receipt.clientId);
  if (new Set(clientIds).size !== 2 || clientIds.join(',') !== [...externalAggregate.clientIds].sort().join(',')) {
    fail('external_client_identity_mismatch', 'External aggregate and receipt client identities differ');
  }
  const clients = receipts.map((receipt) => {
    if (receipt.runId !== externalAggregate.runId || receipt.success !== true ||
        receipt.outsideServiceHost !== true || receipt.outsideServiceNetworkNamespace !== true ||
        receipt.publicEgressObserved !== true) {
      fail('external_vantage_unproven', `${receipt.clientId} does not prove an external vantage`);
    }
    const runnerSha256 = stableHandoffHash(receipt.runnerIdentity ?? {});
    const checks = new Map((receipt.ingressChecks ?? []).map((check) => [check.kind, check]));
    const ingressObservations = Object.fromEntries(REQUIRED_INGRESS_CHECKS.map((kind) => {
      const check = checks.get(kind);
      return [kind, { state: check?.state === 'passed' ? 'passed' : check ? 'failed' : 'missing', artifactId }];
    }));
    return {
      clientId: receipt.clientId,
      hostId: `external-host-${runnerSha256.slice(0, 24)}`,
      networkNamespaceId: `external-network-${runnerSha256.slice(24, 48)}`,
      outsideServiceHost: true,
      outsideServiceNetworkNamespace: true,
      publicEgressObserved: true,
      ingressObservations,
    };
  });
  const findingCounts = Object.fromEntries(EXTERNAL_HANDOFF_FINDING_CODES.map((code) => [
    code, oracleReports.reduce((count, report) => count + (report.summary.findingCounts[code] ?? 0), 0),
  ]));
  const externalHandoffOracleReport = {
    schemaVersion: 'agent-browser.p158-external-handoff-oracle-report.v1',
    planId: 'P158',
    auditId: `${externalAggregate.runId}:combined-external-oracle`,
    fixtureId: `${externalAggregate.runId}:external-calibration`,
    inputSha256: stableHandoffHash({
      aggregate: externalAggregate.aggregateSha256 ?? stableHandoffHash(externalAggregate),
      reports: oracleReports.map((report) => report.inputSha256).sort(),
    }),
    auditedAt: oracleReports.map((report) => report.auditedAt).sort().at(-1),
    repairAttempted: false,
    passed: true,
    summary: {
      urlObservationCount: oracleReports.reduce((count, report) => count + report.summary.urlObservationCount, 0),
      ingressCheckCount: oracleReports.reduce((count, report) => count + report.summary.ingressCheckCount, 0),
      reconnectCount: oracleReports.reduce((count, report) => count + report.summary.reconnectCount, 0),
      findingCount: oracleReports.reduce((count, report) => count + report.summary.findingCount, 0),
      findingCounts,
    },
    urlClassifications: oracleReports.flatMap((report) => structuredClone(report.urlClassifications)),
    findings: oracleReports.flatMap((report) => structuredClone(report.findings)),
  };
  return {
    externalVantage: { serviceHostId, serviceNetworkNamespaceId, clients },
    externalHandoffOracleReport,
  };
}

function loadCredentials(authEnvPath, env) {
  let values = env;
  let source = 'environment';
  if (authEnvPath) {
    const path = resolve(authEnvPath);
    const stat = lstatSync(path);
    if (!stat.isFile() || stat.isSymbolicLink() || (stat.mode & 0o077) !== 0) {
      fail('e2_auth_file_unsafe', 'E2 auth file must be a private regular non-symlink file');
    }
    values = parseEnvText(readFileSync(path, 'utf8'));
    source = 'private_env_file';
  }
  const username = values?.P158_DEV_DASHBOARD_USERNAME ??
    values?.AGENT_BROWSER_DASHBOARD_CODEX_USERNAME ?? values?.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME;
  const password = values?.P158_DEV_DASHBOARD_PASSWORD ??
    values?.AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD ?? values?.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD;
  if (!username || !password) fail('e2_auth_missing', 'E2 dashboard credentials are unavailable');
  return { username, password, source };
}

function mergeHeaders(current, additions) {
  const headers = Object.fromEntries(new Headers(current ?? {}).entries());
  return { ...headers, ...additions };
}

/**
 * Return a cookie-bearing fetch for E2 only. Credentials are loaded lazily,
 * retained only in the closure during login, and absent from describe().
 */
export function createP158E2AuthenticatedFetch({
  fetch: baseFetch = globalThis.fetch, authEnvPath = null, env = process.env,
  dashboardOrigin, protectedOrigins,
}) {
  const dashboard = new URL(dashboardOrigin).origin;
  const protectedSet = new Set(protectedOrigins.map((origin) => new URL(origin).origin));
  let cookie = null;
  let authSource = null;
  async function authenticate() {
    const credentials = loadCredentials(authEnvPath, env);
    authSource = credentials.source;
    const url = new URL('/api/dashboard-auth/login', dashboard).href;
    const response = await baseFetch(url, {
      method: 'POST', redirect: 'error', cache: 'no-store',
      headers: { accept: 'application/json', 'content-type': 'application/json' },
      body: JSON.stringify({ username: credentials.username, password: credentials.password }),
    });
    const payload = await response.json();
    const setCookie = response.headers?.get?.('set-cookie');
    if (!response.ok || payload?.authenticated !== true || typeof setCookie !== 'string') {
      fail('e2_authentication_failed', `E2 dashboard authentication failed with HTTP ${response.status}`);
    }
    cookie = setCookie.split(';', 1)[0];
  }
  const authenticatedFetch = async (url, init = {}) => {
    const origin = new URL(url).origin;
    if (!protectedSet.has(origin)) return baseFetch(url, init);
    if (cookie === null) await authenticate();
    return baseFetch(url, { ...init, headers: mergeHeaders(init.headers, { cookie }) });
  };
  Object.defineProperty(authenticatedFetch, 'describe', { value: () => Object.freeze({
    schemaVersion: 'agent-browser.p158-e2-authenticated-fetch.v1',
    authentication: 'dashboard_session_cookie',
    credentialSource: authSource ?? (authEnvPath ? 'private_env_file' : 'environment'),
    serializedSecrets: false,
    dashboardOrigin: dashboard,
    protectedOrigins: [...protectedSet].sort(),
  }) });
  return authenticatedFetch;
}

export function serializeP158W6ProjectedArtifact(value) {
  return Buffer.from(canonicalJson(value));
}
