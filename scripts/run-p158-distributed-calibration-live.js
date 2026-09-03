#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import { dirname, isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { canonicalJson, createFileArtifactStore, sha256 } from './lib/p158-campaign-controller.js';
import {
  finalizeDistributedC01Calibration,
  prepareDistributedC01Calibration,
  startDistributedC01Calibration,
} from './lib/p158-distributed-calibration.js';
import { canonicalHash } from './run-p158-external-vantage.js';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const PREPARATION_PATH = 'distributed-c01/preparation.json';
const LOCAL_RUN_PATH = 'distributed-c01/local-run.json';
const FINAL_RESULT_PATH = 'distributed-c01/final-result.json';

export const C01_READ_ONLY_ROTATION = Object.freeze([
  Object.freeze({ action: 'service_status', path: '/api/service/status' }),
  Object.freeze({ action: 'resource_inventory', path: '/api/service/resources' }),
  Object.freeze({ action: 'incident_summary', path: '/api/service/incidents?summary=true&limit=20' }),
  Object.freeze({ action: 'profile_source', path: '/api/service/profiles' }),
  Object.freeze({ action: 'site_policy', path: '/api/service/site-policies' }),
]);

export class LiveCalibrationError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'LiveCalibrationError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new LiveCalibrationError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function without(value, fields) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([field]) => !fields.includes(field)));
}

export function canonicalRuntimeCandidateDigest(candidate) {
  return sha256(without(candidate, ['candidateSha256']));
}

function envelopeDigest(envelope) {
  return sha256(without(envelope, ['envelopeSha256']));
}

function localEnvelopeDigest(envelope) {
  return sha256(without(envelope, ['localEnvelopeSha256']));
}

function assertRunRoot(runRoot) {
  if (!isAbsolute(runRoot ?? '')) {
    fail('invalid_run_root', 'The calibration run root must be an explicit absolute path');
  }
  const normalized = resolve(runRoot);
  const fromRepo = relative(REPO_ROOT, normalized);
  if (fromRepo === '' || (!fromRepo.startsWith('..') && !isAbsolute(fromRepo))) {
    fail('run_root_inside_repository', 'The calibration run root must be outside the product repository');
  }
  return normalized;
}

function exactOrigin(value, label) {
  let url;
  try {
    url = new URL(value);
  } catch {
    fail('invalid_development_origin', `${label} must be an absolute HTTP origin`);
  }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password ||
      url.pathname !== '/' || url.search || url.hash) {
    fail('invalid_development_origin', `${label} must contain only an HTTP scheme, host, and optional port`);
  }
  return url.origin;
}

function validateCandidate(candidate) {
  if (!candidate || candidate.runtimeEnvironment !== 'development' ||
      !/^[a-f0-9]{64}$/u.test(candidate.executableSha256 ?? '') ||
      !/^[a-f0-9]{64}$/u.test(candidate.dashboardSha256 ?? '') ||
      typeof candidate.packageVersion !== 'string' || !candidate.packageVersion ||
      typeof candidate.serviceContractVersion !== 'string' || !candidate.serviceContractVersion ||
      typeof candidate.installedGenerationId !== 'string' || !candidate.installedGenerationId ||
      typeof candidate.runtimeManifestRevision !== 'string' || !candidate.runtimeManifestRevision ||
      candidate.candidateSha256 !== canonicalRuntimeCandidateDigest(candidate)) {
    fail('candidate_identity_mismatch', 'Candidate identity must be complete, development-only, and self-hashed');
  }
}

function validateConfig(config) {
  validateCandidate(config.candidate);
  if (!Array.isArray(config.developmentTargets) || config.developmentTargets.length !== 2 ||
      config.developmentTargets.map((target) => target.environmentId).sort().join(',') !== 'E1,E2') {
    fail('development_environment_mismatch', 'Exactly E1 and E2 development targets are required');
  }
  if (!Array.isArray(config.agentClientIds) || config.agentClientIds.length !== 25 ||
      new Set(config.agentClientIds).size !== 25) {
    fail('client_identity_mismatch', 'Exactly 25 explicit agent client IDs are required');
  }
  const targets = config.developmentTargets.map((target) => {
    if (target.scope !== 'development' || !isAbsolute(target.profileRoot ?? '')) {
      fail('development_environment_mismatch', `${target.environmentId} is not explicitly development-scoped`);
    }
    const profileRelative = relative(REPO_ROOT, resolve(target.profileRoot));
    if (profileRelative === '' || (!profileRelative.startsWith('..') && !isAbsolute(profileRelative))) {
      fail('profile_root_inside_repository', 'Development profile roots must remain outside the product repository');
    }
    return {
      ...clone(target),
      serviceUrl: exactOrigin(target.serviceUrl, `${target.environmentId}.serviceUrl`),
      dashboardUrl: exactOrigin(target.dashboardUrl, `${target.environmentId}.dashboardUrl`),
    };
  });
  if (new Set(targets.map((target) => target.serviceUrl)).size !== 2 ||
      new Set(targets.map((target) => target.dashboardUrl)).size !== 2) {
    fail('development_environment_mismatch', 'E1 and E2 require distinct frozen Service and dashboard origins');
  }
  return { ...clone(config), developmentTargets: targets };
}

async function fetchJsonOnce(fetchImpl, url, init, failureCode) {
  let response;
  try {
    response = await fetchImpl(url, init);
  } catch (error) {
    fail(failureCode, `Development endpoint request failed: ${error?.message ?? String(error)}`);
  }
  if (response.redirected || new URL(response.url).href !== new URL(url).href) {
    fail('development_endpoint_redirected', 'Development endpoint redirected away from its frozen URL');
  }
  if (!response.ok) fail(failureCode, `Development endpoint returned HTTP ${response.status}`);
  try {
    return { response, body: await response.json() };
  } catch {
    fail(failureCode, 'Development endpoint did not return JSON');
  }
}

async function observeRuntimeBindings(config, fetchImpl) {
  const bindings = [];
  for (const target of config.developmentTargets) {
    const common = { method: 'GET', redirect: 'error', cache: 'no-store', headers: { accept: 'application/json' } };
    const statusUrl = new URL('/api/service/status', target.serviceUrl).href;
    const manifestUrl = new URL('/api/runtime/manifest', target.dashboardUrl).href;
    const status = await fetchJsonOnce(fetchImpl, statusUrl, common, 'development_status_unavailable');
    const manifestResult = await fetchJsonOnce(fetchImpl, manifestUrl, common, 'development_manifest_unavailable');
    const manifest = manifestResult.body;
    if (manifest?.schemaVersion !== 'agent-browser.runtime-manifest.v1' ||
        manifest.runtimeEnvironment !== 'development' ||
        manifest.executable?.sha256 !== config.candidate.executableSha256 ||
        manifest.dashboard?.sha256 !== config.candidate.dashboardSha256 ||
        manifest.packageVersion !== config.candidate.packageVersion ||
        manifest.serviceContractVersion !== config.candidate.serviceContractVersion) {
      fail('candidate_runtime_identity_mismatch', `${target.environmentId} does not expose the frozen development candidate`);
    }
    bindings.push({
      environmentId: target.environmentId,
      serviceOrigin: target.serviceUrl,
      dashboardOrigin: target.dashboardUrl,
      profileRoot: target.profileRoot,
      statusSha256: sha256(status.body),
      runtimeManifestSha256: sha256(manifest),
      candidateSha256: config.candidate.candidateSha256,
      executableSha256: manifest.executable.sha256,
      dashboardSha256: manifest.dashboard.sha256,
    });
  }
  return bindings;
}

function verifyEnvelope(envelope) {
  if (!envelope || envelope.schemaVersion !== 'agent-browser.p158-distributed-c01-live-preparation.v1' ||
      envelope.envelopeSha256 !== envelopeDigest(envelope) ||
      envelope.prepared?.preparedSha256 !== envelope.preparedSha256) {
    fail('preparation_integrity_mismatch', 'Persisted live calibration preparation is missing or changed');
  }
  validateCandidate(envelope.candidate);
  return envelope;
}

async function readJson(store, relativePath) {
  try {
    return JSON.parse((await store.read(relativePath)).toString('utf8'));
  } catch (error) {
    if (error instanceof SyntaxError) fail('artifact_json_invalid', `${relativePath} is not valid JSON`);
    throw error;
  }
}

export function createDevelopmentC01ServiceTransport({ preparation, fetch: fetchImpl, clock }) {
  const envelope = verifyEnvelope(clone(preparation));
  if (typeof fetchImpl !== 'function' || typeof clock?.wallNow !== 'function' ||
      typeof clock?.monotonicNow !== 'function') {
    fail('invalid_transport_dependencies', 'Injected fetch and wall/monotonic clocks are required');
  }
  const targets = new Map(envelope.developmentTargets.map((target) => [target.environmentId, target]));
  const clients = new Set(envelope.agentClientIds);
  const actions = new Map(C01_READ_ONLY_ROTATION.map((entry) => [entry.action, entry.path]));
  const observations = [];
  return Object.freeze({
    observations: () => clone(observations),
    async executeReadOnlyCommand(request) {
      const target = targets.get(request?.target?.environmentId);
      const path = actions.get(request?.action);
      const ordinal = request?.ordinal;
      const expectedRotation = Number.isInteger(ordinal) && ordinal >= 1 && ordinal <= 500
        ? C01_READ_ONLY_ROTATION[(ordinal - 1) % C01_READ_ONLY_ROTATION.length]
        : null;
      const expectedClientId = expectedRotation ? envelope.agentClientIds[(ordinal - 1) % 25] : null;
      const expectedTarget = expectedRotation ? envelope.developmentTargets[(ordinal - 1) % 2] : null;
      if (!target || request.target.serviceUrl !== target.serviceUrl || !clients.has(request.clientId) || !path ||
          request.attempt !== 1 || request.effectClass !== 'read_only' ||
          request.action !== expectedRotation?.action || request.clientId !== expectedClientId ||
          target.environmentId !== expectedTarget?.environmentId) {
        fail('read_only_command_outside_freeze', 'Service command is outside the frozen target, client, or action rotation');
      }
      const url = new URL(path, target.serviceUrl).href;
      const before = clock.monotonicNow();
      let response;
      let body;
      try {
        response = await fetchImpl(url, {
          method: 'GET', redirect: 'error', cache: 'no-store',
          headers: { accept: 'application/json', 'x-agent-browser-client-id': request.clientId },
        });
        if (response.redirected || new URL(response.url).href !== new URL(url).href) {
          throw Object.assign(new Error('Service command redirected'), { code: 'service_command_redirected' });
        }
        body = await response.json();
      } catch (error) {
        const result = {
          state: 'failed', effectClass: 'read_only', attempt: 1,
          retryAttempted: false, repairAttempted: false,
          httpStatus: response?.status ?? null,
          latencyMs: Math.max(0, Number(clock.monotonicNow() - before) / 1_000_000),
          observedAt: clock.wallNow(),
          failure: { code: error?.code ?? 'service_transport_failed', name: error?.name ?? 'Error', message: error?.message ?? String(error) },
        };
        observations.push({
          ordinal: request.ordinal, action: request.action, clientId: request.clientId,
          environmentId: target.environmentId, url, ...clone(result),
        });
        return result;
      }
      const failure = !response.ok
        ? { code: 'service_http_status', name: 'ServiceHttpError', message: `HTTP ${response.status}` }
        : body?.success === false
          ? { code: body.failure?.code ?? body.error?.code ?? 'service_response_failed', name: 'ServiceResponseError', message: body.failure?.message ?? body.error?.message ?? 'Service response failed' }
          : null;
      const result = {
        state: failure ? 'failed' : 'passed', effectClass: 'read_only', attempt: 1,
        retryAttempted: false, repairAttempted: false, httpStatus: response.status,
        latencyMs: Math.max(0, Number(clock.monotonicNow() - before) / 1_000_000),
        observedAt: clock.wallNow(), ...(failure ? { failure } : { responseSha256: sha256(body) }),
      };
      observations.push({
        ordinal: request.ordinal, action: request.action, clientId: request.clientId,
        environmentId: target.environmentId, url, ...clone(result),
      });
      return result;
    },
  });
}

export async function prepareLiveDistributedCalibration({ config, runRoot, fetch: fetchImpl, clock }) {
  const root = assertRunRoot(runRoot);
  const normalized = validateConfig(config);
  const runtimeBindings = await observeRuntimeBindings(normalized, fetchImpl);
  const prepared = prepareDistributedC01Calibration({ ...clone(normalized), clock });
  const envelope = {
    schemaVersion: 'agent-browser.p158-distributed-c01-live-preparation.v1',
    candidate: clone(normalized.candidate),
    candidateSha256: normalized.candidate.candidateSha256,
    developmentTargets: clone(normalized.developmentTargets),
    agentClientIds: clone(normalized.agentClientIds),
    runtimeBindings,
    preparedSha256: prepared.preparedSha256,
    prepared,
    effectsAttempted: false,
  };
  envelope.envelopeSha256 = envelopeDigest(envelope);
  const store = createFileArtifactStore(root);
  await store.writeOnce(PREPARATION_PATH, canonicalJson(envelope));
  return clone(envelope);
}

export async function startLiveDistributedCalibration({ runRoot, fetch: fetchImpl, clock, scheduler, safetyStop }) {
  const root = assertRunRoot(runRoot);
  const store = createFileArtifactStore(root);
  const envelope = verifyEnvelope(await readJson(store, PREPARATION_PATH));
  const serviceTransport = createDevelopmentC01ServiceTransport({ preparation: envelope, fetch: fetchImpl, clock });
  const localRun = await startDistributedC01Calibration({
    prepared: envelope.prepared, serviceTransport, scheduler, clock, artifactStore: store, safetyStop,
  });
  const transportObservations = serviceTransport.observations();
  if (transportObservations.length !== 500 ||
      transportObservations.some((entry, index) => entry.ordinal !== index + 1)) {
    fail('transport_observation_count_mismatch', 'Live transport did not retain all 500 one-shot command observations');
  }
  const localEnvelope = {
    schemaVersion: 'agent-browser.p158-distributed-c01-live-local-run.v1',
    preparationEnvelopeSha256: envelope.envelopeSha256,
    preparedSha256: envelope.preparedSha256,
    localRun,
    transportObservations,
    retryAttempted: false,
    repairAttempted: false,
  };
  localEnvelope.localEnvelopeSha256 = localEnvelopeDigest(localEnvelope);
  await store.writeOnce(LOCAL_RUN_PATH, canonicalJson(localEnvelope));
  return clone(localEnvelope);
}

function verifyExternalAggregate(aggregate, receipts, prepared) {
  if (!aggregate || aggregate.schemaVersion !== 'agent-browser.p158-external-vantage-aggregate.v1' ||
      aggregate.success !== true || aggregate.mode !== 'calibration' || aggregate.runId !== prepared.runId ||
      aggregate.repairAttempted !== false || aggregate.retryCount !== 0 ||
      aggregate.aggregateSha256 !== canonicalHash(without(aggregate, ['aggregateSha256']))) {
    fail('external_aggregate_integrity_mismatch', 'External aggregate is missing, failed, or changed');
  }
  const hashes = receipts.map((receipt) => canonicalHash(receipt)).sort();
  if (JSON.stringify(hashes) !== JSON.stringify([...(aggregate.receiptSha256s ?? [])].sort()) ||
      aggregate.handoffUrlSha256 !== prepared.externalDispatchDescriptor.handoffUrlSha256) {
    fail('external_aggregate_receipt_mismatch', 'Downloaded receipts do not match the external aggregate');
  }
}

export async function finalizeLiveDistributedCalibration({ runRoot, externalAggregate, externalReceipts, clock }) {
  const root = assertRunRoot(runRoot);
  const store = createFileArtifactStore(root);
  const envelope = verifyEnvelope(await readJson(store, PREPARATION_PATH));
  const localEnvelope = await readJson(store, LOCAL_RUN_PATH);
  if (localEnvelope?.schemaVersion !== 'agent-browser.p158-distributed-c01-live-local-run.v1' ||
      localEnvelope.preparationEnvelopeSha256 !== envelope.envelopeSha256 ||
      localEnvelope.preparedSha256 !== envelope.preparedSha256 ||
      localEnvelope.localEnvelopeSha256 !== localEnvelopeDigest(localEnvelope) ||
      localEnvelope.transportObservations?.length !== 500) {
    fail('local_live_evidence_integrity_mismatch', 'Persisted live transport evidence is missing or changed');
  }
  verifyExternalAggregate(clone(externalAggregate), clone(externalReceipts), envelope.prepared);
  const result = finalizeDistributedC01Calibration({
    prepared: envelope.prepared, localRun: localEnvelope.localRun,
    externalRunnerReceipts: clone(externalReceipts), clock,
  });
  for (const artifact of result.artifacts) {
    await store.writeOnce(artifact.relativePath, artifact.content);
  }
  await store.writeOnce(FINAL_RESULT_PATH, canonicalJson(result));
  return clone(result);
}

function takeOption(args, name, { multiple = false } = {}) {
  const values = [];
  for (let index = 0; index < args.length;) {
    if (args[index] !== name) { index += 1; continue; }
    if (index + 1 >= args.length) fail('missing_cli_option', `${name} requires a value`);
    values.push(args[index + 1]);
    args.splice(index, 2);
  }
  if (multiple) return values;
  if (values.length !== 1) fail('missing_cli_option', `Exactly one ${name} is required`);
  return values[0];
}

function realClock() {
  return { wallNow: () => new Date().toISOString(), monotonicNow: () => Number(process.hrtime.bigint()) };
}

function realScheduler() {
  return {
    async waitUntil({ wallTime }) {
      let remaining = Date.parse(wallTime) - Date.now();
      while (remaining > 0) {
        await new Promise((resolveWait) => setTimeout(resolveWait, Math.min(remaining, 30_000)));
        remaining = Date.parse(wallTime) - Date.now();
      }
    },
  };
}

async function readJsonFile(path, label) {
  if (!isAbsolute(path)) fail('invalid_cli_path', `${label} must be an absolute path`);
  return JSON.parse(await readFile(path, 'utf8'));
}

export async function runCli(argv, dependencies = {}) {
  const args = [...argv];
  const command = args.shift();
  const runRoot = takeOption(args, '--run-root');
  const clock = dependencies.clock ?? realClock();
  const fetchImpl = dependencies.fetch ?? globalThis.fetch;
  let result;
  if (command === 'prepare') {
    const config = await readJsonFile(takeOption(args, '--config'), 'config');
    if (args.length) fail('unknown_cli_option', `Unknown arguments: ${args.join(' ')}`);
    result = await prepareLiveDistributedCalibration({ config, runRoot, fetch: fetchImpl, clock });
  } else if (command === 'start') {
    if (args.length) fail('unknown_cli_option', `Unknown arguments: ${args.join(' ')}`);
    result = await startLiveDistributedCalibration({
      runRoot, fetch: fetchImpl, clock, scheduler: dependencies.scheduler ?? realScheduler(),
    });
  } else if (command === 'finalize') {
    const aggregate = await readJsonFile(takeOption(args, '--external-aggregate'), 'external aggregate');
    const receiptPaths = takeOption(args, '--external-receipt', { multiple: true });
    if (receiptPaths.length !== 2 || args.length) {
      fail('external_receipt_count_mismatch', 'Finalize requires exactly two --external-receipt paths');
    }
    const receipts = await Promise.all(receiptPaths.map((path) => readJsonFile(path, 'external receipt')));
    result = await finalizeLiveDistributedCalibration({ runRoot, externalAggregate: aggregate, externalReceipts: receipts, clock });
  } else {
    fail('invalid_cli_command', 'Expected prepare, start, or finalize');
  }
  const summary = {
    command,
    runRoot: assertRunRoot(runRoot),
    state: result.state ?? (result.passed === true ? 'passed' : 'complete'),
    sha256: sha256(result),
  };
  (dependencies.stdout ?? process.stdout).write(`${JSON.stringify(summary)}\n`);
  return result;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  runCli(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${JSON.stringify({ error: error.code ?? 'distributed_calibration_failed', message: error.message })}\n`);
    process.exitCode = 1;
  });
}
