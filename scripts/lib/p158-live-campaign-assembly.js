import { createHash } from 'node:crypto';
import { execFile as execFileCallback } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { lstat, readFile, readdir, readlink } from 'node:fs/promises';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { canonicalJson, sha256 } from './p158-campaign-controller.js';
import { buildP158CampaignPhasePreparation } from './p158-campaign-phase-orchestrator.js';
import {
  createP158LoggingHarvestHook,
  p158LoggingEvidenceSourceBinding,
} from './p158-logging-evidence-harvester.js';
import {
  createP158DevelopmentCommandPrimitives,
  createP158W7LiveDevelopmentAdapterBundle,
} from './p158-w7-development-adapters.js';
import {
  createP158W7A01A03LiveBundle,
} from './p158-w7-a01-a03-live.js';
import { createP158W7A04A06LiveBundle } from './p158-w7-a04-a06-live.js';
import { createP158W7A07A13LiveBundle } from './p158-w7-a07-a13-live.js';
import { createP158W7A08LiveBundle } from './p158-w7-a08-live.js';
import { auditP158W7LiveHookReadiness } from './p158-w7-live-hook-readiness.js';
import { buildP158W8ActionPlan, createP158W8ReviewedLiveAdapterBundle } from './p158-w8-hd-adapters.js';
import {
  createP158W9ConcreteDriverBundle,
  createP158W9FreezeAdapterEntries,
} from './p158-w9-concrete-drivers.js';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const execFile = promisify(execFileCallback);
const SHA256 = /^[a-f0-9]{64}$/u;
export const P158_LIVE_ASSEMBLY_SOURCE_PATH = 'scripts/lib/p158-live-campaign-assembly.js';
export const P158_RUNTIME_IDENTITY_PROJECTION_AXES = Object.freeze([
  'candidate_binary_identity',
  'runtime_generation',
  'runtime_host_pid',
  'runtime_host_start_token',
  'service_socket_identity',
  'stream_identity',
  'runtime_profile_identity',
  'config_path',
  'config_digest',
  'state_path',
  'state_digest',
  'environment_ownership',
  'run_ownership',
  'browser_profile_identity',
]);
export const P158_RUNTIME_CALIBRATION_STATUS_PATHS = Object.freeze({
  runtime_generation: Object.freeze(['data', 'runtimeLifecycle', 'lifecycle', 'registryRevision']),
  runtime_hosts: Object.freeze(['data', 'runtimeLifecycle', 'multiplicity', 'runtimeHosts']),
});

export const P158_RUNTIME_IDENTITY_PROBE_KINDS = Object.freeze({
  candidate_binary_identity: 'candidate_file_sha256',
  runtime_generation: 'service_state_runtime_owner_revision',
  runtime_host_pid: 'calibrated_runtime_host_proc',
  runtime_host_start_token: 'proc_start_token',
  service_socket_identity: 'socket_census_sha256',
  stream_identity: 'service_state_stream_projection_sha256',
  runtime_profile_identity: 'profile_binding',
  config_path: 'confined_file_path',
  config_digest: 'confined_file_sha256',
  state_path: 'confined_file_path',
  state_digest: 'confined_file_sha256',
  environment_ownership: 'runtime_process_environment_marker',
  run_ownership: 'runtime_process_environment_marker',
  browser_profile_identity: 'service_state_profile_sha256',
});

export class P158LiveCampaignAssemblyError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158LiveCampaignAssemblyError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158LiveCampaignAssemblyError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function without(value, field) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== field));
}

function inside(parent, child) {
  const path = relative(resolve(parent), resolve(child));
  return path === '' || (!path.startsWith(`..${sep}`) && path !== '..' && !isAbsolute(path));
}

function assertDigest(value, field) {
  if (!SHA256.test(value ?? '')) fail('assembly_digest_invalid', `${field} requires a SHA-256 digest`);
}

function assertProbeEnvironment(environment, runRoot) {
  const allowedEnvironment = new Set([
    'HOME', 'XDG_CONFIG_HOME', 'XDG_RUNTIME_DIR', 'XDG_STATE_HOME',
    'AGENT_BROWSER_PROFILE', 'AGENT_BROWSER_RUNTIME_PROFILE',
    'P158_CAMPAIGN_RUN_ID', 'P158_CAMPAIGN_ENVIRONMENT_ID',
  ]);
  const pathKeys = ['HOME', 'XDG_CONFIG_HOME', 'XDG_RUNTIME_DIR', 'XDG_STATE_HOME'];
  if (Object.keys(environment ?? {}).some((key) => !allowedEnvironment.has(key)) ||
      pathKeys.some((key) => environment?.[key] !== undefined &&
        (!isAbsolute(environment[key]) || !inside(runRoot, environment[key]))) ||
      ['AGENT_BROWSER_PROFILE', 'AGENT_BROWSER_RUNTIME_PROFILE'].some((key) =>
        environment?.[key] !== undefined && !/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/u.test(environment[key])) ||
      ['P158_CAMPAIGN_RUN_ID', 'P158_CAMPAIGN_ENVIRONMENT_ID'].some((key) =>
        typeof environment?.[key] !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,255}$/u.test(environment[key]))) {
    fail('runtime_identity_probe_invalid', 'Runtime identity probe environment escaped the isolated campaign root');
  }
}

function assertClosedWorldProjection(identityProjection) {
  const axes = identityProjection?.map((field) => field.axis);
  if (!Array.isArray(identityProjection) ||
      JSON.stringify(axes) !== JSON.stringify(P158_RUNTIME_IDENTITY_PROJECTION_AXES) ||
      identityProjection.some((field) => field.probeKind !== P158_RUNTIME_IDENTITY_PROBE_KINDS[field.axis] ||
        field.expectedValue === null || field.expectedValue === undefined ||
        field.valueSha256 !== sha256(field.expectedValue) || !validProjectionValue(field.axis, field.expectedValue))) {
    fail('runtime_identity_probe_invalid', 'Runtime identity probe must seal every mandatory identity axis exactly once');
  }
}

function assertProbeSpecification(specification, runRoot, environmentId) {
  const expectedKeys = ['configPath', 'profileName', 'profilePath', 'socketDir', 'statePath'];
  if (!specification || JSON.stringify(Object.keys(specification).sort()) !== JSON.stringify(expectedKeys) ||
      expectedKeys.filter((key) => key !== 'profileName').some((key) =>
        !isAbsolute(specification[key] ?? '') || !inside(runRoot, specification[key])) ||
      typeof specification.profileName !== 'string' || specification.profileName.length === 0) {
    fail('runtime_identity_probe_invalid', 'Runtime identity probe paths must be the closed, campaign-confined source set');
  }
}


function validProjectionValue(axis, value) {
  if (['candidate_binary_identity', 'service_socket_identity', 'stream_identity', 'config_digest',
    'state_digest', 'browser_profile_identity'].includes(axis)) return SHA256.test(value ?? '');
  if (axis === 'runtime_generation') return Number.isInteger(value) && value >= 0;
  if (axis === 'runtime_host_pid') return Number.isInteger(value) && value >= 2;
  if (['config_path', 'state_path'].includes(axis)) return typeof value === 'string' && isAbsolute(value);
  return typeof value === 'string' && value.length > 0;
}

async function executeStatusProbe(candidateExecutablePath, commandArgs, environment) {
  try {
    const result = await execFile(candidateExecutablePath, commandArgs, {
      env: { PATH: process.env.PATH, ...environment }, maxBuffer: 8 * 1024 * 1024,
    });
    return JSON.parse(result.stdout);
  } catch (error) {
    fail('runtime_identity_probe_failed', 'Live Service Status readback failed', {
      cause: error?.code ?? error?.message,
    });
  }
}

const defaultProbePrimitives = Object.freeze({
  readFile,
  readdir: (path) => readdir(path, { withFileTypes: true }),
  lstat,
  readlink,
});

async function fileDigest(primitives, path, label) {
  try { return sha256(await primitives.readFile(path)); } catch (error) {
    fail('runtime_identity_probe_failed', `${label} could not be read`, { cause: error?.code });
  }
}

function procStartTicks(stat) {
  const end = stat.lastIndexOf(')');
  const fields = stat.slice(end + 2).trim().split(/\s+/u);
  const token = fields[19];
  if (!token || !/^\d+$/u.test(token)) fail('runtime_identity_probe_failed', 'Runtime host /proc start token is invalid');
  return token;
}

async function collectRuntimeIdentityAxes({ candidateExecutablePath, runRoot, environmentId, environment,
  probeSpecification, runtimeHostBinding, primitives = defaultProbePrimitives }) {
  assertProbeSpecification(probeSpecification, runRoot, environmentId);
  const candidateSha256 = await fileDigest(primitives, candidateExecutablePath, 'Candidate binary');
  const host = runtimeHostBinding;
  if (host?.binarySha256 !== candidateSha256 || resolve(host.executablePath ?? '') !== resolve(candidateExecutablePath) ||
      !Number.isInteger(host.pid) || host.pid < 2 || typeof host.processStartToken !== 'string' ||
      typeof host.socketIdentity !== 'string' || host.socketIdentity.length === 0) {
    fail('runtime_identity_probe_failed', 'Calibrated runtime host binding is incomplete or belongs to another candidate');
  }
  const procRoot = `/proc/${host.pid}`;
  let executableLink;
  let statText;
  let environBytes;
  let bootId;
  try {
    [executableLink, statText, environBytes, bootId] = await Promise.all([
      primitives.readlink(`${procRoot}/exe`), primitives.readFile(`${procRoot}/stat`, 'utf8'),
      primitives.readFile(`${procRoot}/environ`),
      primitives.readFile('/proc/sys/kernel/random/boot_id', 'utf8'),
    ]);
  } catch (error) {
    fail('runtime_identity_probe_failed', 'Runtime host process identity is unavailable', { cause: error?.code });
  }
  const observedStartToken = `linux:${String(bootId).trim()}:${procStartTicks(String(statText))}`;
  if (resolve(executableLink) !== resolve(candidateExecutablePath) || observedStartToken !== host.processStartToken) {
    fail('runtime_identity_probe_failed', 'Service Status runtime host does not match the live process');
  }
  const processEnvironment = new Map(Buffer.from(environBytes).toString('utf8').split('\0')
    .filter(Boolean).map((entry) => [entry.slice(0, entry.indexOf('=')), entry.slice(entry.indexOf('=') + 1)]));
  if (processEnvironment.get('P158_CAMPAIGN_RUN_ID') !== environment.P158_CAMPAIGN_RUN_ID ||
      processEnvironment.get('P158_CAMPAIGN_ENVIRONMENT_ID') !== environmentId ||
      processEnvironment.get('AGENT_BROWSER_RUNTIME_PROFILE') !== probeSpecification.profileName ||
      environment.P158_CAMPAIGN_ENVIRONMENT_ID !== environmentId ||
      typeof environment.P158_CAMPAIGN_RUN_ID !== 'string' || environment.P158_CAMPAIGN_RUN_ID.length === 0) {
    fail('runtime_identity_probe_failed', 'Live runtime process lacks the frozen campaign ownership markers');
  }
  let entries;
  try { entries = await primitives.readdir(probeSpecification.socketDir); } catch (error) {
    fail('runtime_identity_probe_failed', 'Runtime socket directory is unavailable', { cause: error?.code });
  }
  const socketCensus = [];
  for (const entry of entries) {
    const entryPath = resolve(probeSpecification.socketDir, entry.name);
    if (!inside(probeSpecification.socketDir, entryPath)) fail('runtime_identity_probe_failed', 'Socket census escaped its directory');
    const metadata = await primitives.lstat(entryPath);
    socketCensus.push({ name: entry.name, socket: metadata.isSocket(), symbolicLink: metadata.isSymbolicLink(),
      device: String(metadata.dev), inode: String(metadata.ino), mode: String(metadata.mode) });
  }
  if (!socketCensus.some((entry) => entry.socket)) fail('runtime_identity_probe_failed', 'Runtime socket census contains no socket');
  let state;
  try { state = JSON.parse(String(await primitives.readFile(probeSpecification.statePath, 'utf8'))); } catch (error) {
    fail('runtime_identity_probe_failed', 'Development Service State is not valid JSON', { cause: error?.code });
  }
  const processIdentities = state.browserProcessIdentities;
  const profileRecords = Object.entries(processIdentities ?? {}).filter(([, record]) =>
    record?.runtimeProfile === probeSpecification.profileName &&
    resolve(record.userDataDir ?? '') === resolve(probeSpecification.profilePath));
  if (profileRecords.length === 0) fail('runtime_identity_probe_failed', 'Development Service State omitted the exact browser/profile identity');
  const streamProjection = {
    remoteViewHandoffs: state.remoteViewHandoffs ?? {}, remoteViewRoutes: state.remoteViewRoutes ?? {},
    viewerLeases: state.viewerLeases ?? {}, remoteViewAcquisitionLeases: state.remoteViewAcquisitionLeases ?? {},
  };
  if (Object.values(streamProjection).every((records) => Object.keys(records).length === 0)) {
    fail('runtime_identity_probe_failed', 'Development Service State omitted all durable stream identities');
  }
  const values = {
    candidate_binary_identity: candidateSha256,
    runtime_generation: state.runtimeOwnerRegistry?.revision,
    runtime_host_pid: host.pid,
    runtime_host_start_token: host.processStartToken,
    service_socket_identity: sha256({ reportedSocketIdentity: host.socketIdentity,
      census: socketCensus.sort((a, b) => a.name.localeCompare(b.name)) }),
    stream_identity: sha256(streamProjection),
    runtime_profile_identity: `${probeSpecification.profileName}:${sha256(resolve(probeSpecification.profilePath))}`,
    config_path: resolve(probeSpecification.configPath),
    config_digest: await fileDigest(primitives, probeSpecification.configPath, 'Development config'),
    state_path: resolve(probeSpecification.statePath),
    state_digest: await fileDigest(primitives, probeSpecification.statePath, 'Development Service State'),
    environment_ownership: processEnvironment.get('P158_CAMPAIGN_ENVIRONMENT_ID'),
    run_ownership: processEnvironment.get('P158_CAMPAIGN_RUN_ID'),
    browser_profile_identity: sha256(profileRecords.sort(([left], [right]) => left.localeCompare(right))),
  };
  if (JSON.stringify(Object.keys(values)) !== JSON.stringify(P158_RUNTIME_IDENTITY_PROJECTION_AXES) ||
      !P158_RUNTIME_IDENTITY_PROJECTION_AXES.every((axis) => validProjectionValue(axis, values[axis]))) {
    fail('runtime_identity_probe_failed', 'A mandatory live runtime identity axis is unavailable');
  }
  return values;
}

async function readBoundJson(ref, runRoot, field) {
  if (!isAbsolute(ref?.path ?? '') || !inside(runRoot, ref.path)) {
    fail('assembly_path_invalid', `${field}.path must be an absolute child of the campaign run root`);
  }
  assertDigest(ref.sha256, `${field}.sha256`);
  let bytes;
  try { bytes = await readFile(ref.path); } catch (error) {
    fail('assembly_artifact_missing', `${field} is absent`, { cause: error?.code });
  }
  const actual = sha256(bytes);
  if (actual !== ref.sha256) fail('assembly_artifact_changed', `${field} changed after preparation`, { expected: ref.sha256, actual });
  try { return JSON.parse(bytes.toString('utf8')); } catch {
    fail('assembly_artifact_invalid', `${field} is not JSON`);
  }
}

function assertConfiguration(configuration, descriptor, manifest, schedule, liveHookManifest) {
  const body = without(configuration, 'configurationSha256');
  if (configuration?.schemaVersion !== 'agent-browser.p158-live-bundle-assembly-config.v1' ||
      configuration.configurationSha256 !== sha256(body) || configuration.runId !== descriptor.runId ||
      configuration.candidateSha256 !== manifest.candidate.candidateSha256 ||
      configuration.scheduleSha256 !== schedule.scheduleSha256 ||
      configuration.liveHookManifestSha256 !== liveHookManifest.manifestSha256 ||
      configuration.runtimeLane !== 'development' || configuration.production !== false ||
      configuration.repairAllowed !== false || configuration.retryAllowed !== false ||
      configuration.garbageCollectionAllowed !== false) {
    fail('assembly_configuration_drift', 'Bundle assembly configuration is not the frozen development campaign identity');
  }
}

async function appendReceipt(artifactStore, runId, receipt) {
  const digest = sha256(receipt);
  const path = `live-bundle-assembly/w7-receipts/${runId}/${digest}.json`;
  try {
    const prior = await artifactStore.read(path);
    if (prior === undefined) {
      await artifactStore.writeOnce(path, canonicalJson(receipt));
      return;
    }
    if (sha256(prior) !== sha256(canonicalJson(receipt))) fail('receipt_collision', `${path} already contains different bytes`);
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
    await artifactStore.writeOnce(path, canonicalJson(receipt));
  }
}

function createA13ReceiptStore(artifactStore, runId) {
  const pathFor = (attemptId) => `live-bundle-assembly/w7-a13-receipts/${runId}/${attemptId}.json`;
  return {
    async read(attemptId) {
      try {
        const bytes = await artifactStore.read(pathFor(attemptId));
        return bytes === undefined ? null : JSON.parse(Buffer.from(bytes).toString('utf8'));
      } catch (error) {
        if (error?.code === 'ENOENT') return null;
        throw error;
      }
    },
    async append(receipt) {
      await writeExact(artifactStore, pathFor(receipt.attemptId), receipt);
    },
  };
}

function createA08ReceiptStore(artifactStore, runId) {
  const pathFor = (kind, cellId) => `live-bundle-assembly/w7-a08-${kind}/${runId}/${cellId}.json`;
  const read = async (kind, cellId) => {
    try {
      const bytes = await artifactStore.read(pathFor(kind, cellId));
      return bytes === undefined ? null : JSON.parse(Buffer.from(bytes).toString('utf8'));
    } catch (error) {
      if (error?.code === 'ENOENT') return null;
      throw error;
    }
  };
  return {
    readClaim: (cellId) => read('claims', cellId),
    appendClaim: (receipt) => writeExact(artifactStore, pathFor('claims', receipt.cellId), receipt),
    readTerminal: (cellId) => read('terminals', cellId),
    appendTerminal: (receipt) => writeExact(artifactStore, pathFor('terminals', receipt.cellId), receipt),
  };
}

async function writeExact(artifactStore, path, value) {
  const content = canonicalJson(value);
  try {
    const prior = await artifactStore.read(path);
    if (prior === undefined) {
      await artifactStore.writeOnce(path, content);
      return;
    }
    if (sha256(prior) !== sha256(content)) fail('assembly_checkpoint_changed', `${path} changed across resume`);
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
    await artifactStore.writeOnce(path, content);
  }
}

const liveScheduler = Object.freeze({
  waitUntil: async ({ wallTime }) => {
    const delay = Date.parse(wallTime) - Date.now();
    if (!Number.isFinite(delay)) fail('case_window_invalid', 'Scheduled wall time is not RFC 3339');
    if (delay > 0) await new Promise((resolveWait) => setTimeout(resolveWait, delay));
  },
});

function verifyAdapterManifest(liveHookManifest, bundles) {
  const actual = [
    ...bundles.w7Bundle.adapterBindings,
    ...bundles.w8Bundle.adapterBindings,
    ...bundles.w9.adapterBindings,
  ];
  if (actual.length !== 54 || sha256(actual) !== sha256(liveHookManifest.adapterBindings)) {
    fail('assembly_adapter_manifest_mismatch', 'Constructed adapters differ from the frozen 54-case live-hook manifest');
  }
}

/**
 * Reads fresh development runtime identities from source-hashed probe receipts.
 * The probe files must be separate from the frozen expected-identity artifact.
 */
function valueAtPath(value, path) {
  return path.reduce((current, key) => current?.[key], value);
}

export async function readP158LiveCampaignRuntimeIdentity({ descriptor, manifest, expectedRuntimeIdentity, isolation,
  probePrimitives }) {
  const probes = descriptor?.runtimeIdentityProbes;
  if (!Array.isArray(probes) || probes.length !== 2 ||
      probes.map((probe) => probe.environmentId).sort().join(',') !== 'E1,E2') {
    fail('runtime_identity_probe_missing', 'Exactly E1 and E2 require fresh live runtime identity probes');
  }
  const expected = new Map(expectedRuntimeIdentity.environments.map((entry) => [entry.environmentId, entry]));
  if (!manifest.environmentSeals.some((entry) => entry.environmentId === 'E0') ||
      !manifest.environmentSeals.some((entry) => entry.environmentId === 'E3')) {
    fail('runtime_identity_probe_missing', 'Frozen E0 fixture and E3 redacted comparison seals are required');
  }
  const observed = new Map(expectedRuntimeIdentity.environments.map((entry) => [entry.environmentId, clone(entry)]));
  for (const probe of probes) {
    const probeBody = without(probe, 'probeSha256');
    if (probe.probeSha256 !== sha256(probeBody) ||
        JSON.stringify(probe.commandArgs) !== JSON.stringify(['service', 'status', '--json']) ||
        !Array.isArray(probe.identityProjection)) {
      fail('runtime_identity_probe_invalid', 'Runtime identity probe command and projection must be frozen');
    }
    assertClosedWorldProjection(probe.identityProjection);
    assertProbeEnvironment(probe.environment, descriptor.runRoot);
    const axes = await collectRuntimeIdentityAxes({
      candidateExecutablePath: descriptor.candidateExecutablePath, runRoot: descriptor.runRoot,
      environmentId: probe.environmentId, environment: probe.environment,
      probeSpecification: probe.probeSpecification, runtimeHostBinding: probe.runtimeHostBinding,
      primitives: typeof probePrimitives === 'function'
        ? probePrimitives(probe.environmentId) : probePrimitives,
    });
    if (probe.identityProjection.some((field) => sha256(axes[field.axis]) !== field.valueSha256)) {
      fail('runtime_identity_drift', `${probe.environmentId} live runtime identity differs from its frozen projection`);
    }
    const entry = probe.expectedEnvironmentIdentity;
    const wanted = expected.get(probe.environmentId);
    if (!wanted || entry.environmentId !== probe.environmentId || entry.identitySha256 !== sha256(entry.identity) ||
        sha256(entry) !== sha256(wanted) || entry.identity?.runtimeLane !== 'development' ||
        entry.identity?.production === true || entry.identity?.campaignRunId !== descriptor.runId ||
        entry.identity?.candidateSha256 !== manifest.candidate.candidateSha256 ||
        entry.identity?.ownership !== 'p158_campaign' || entry.identity?.foreign === true ||
        entry.identity?.tenantDataPresent === true || entry.identity?.isolationState !== 'isolated') {
      fail('runtime_identity_drift', `${probe.environmentId} current ownership differs from its frozen identity`);
    }
    observed.set(probe.environmentId, entry);
  }
  for (const path of Object.values(isolation)) {
    if (!inside(descriptor.runRoot, path)) fail('runtime_identity_drift', 'Runtime isolation escaped the frozen run root');
  }
  return { ...clone(expectedRuntimeIdentity), environments: [...expectedRuntimeIdentity.environments]
    .map((entry) => observed.get(entry.environmentId)) };
}

/** Constructs only source-bound live adapters. It performs no browser or provider effect. */
export async function constructP158LiveCampaignBundles({
  descriptor, manifest, schedule, phasePreparation, liveHookManifest, runtimeIdentity,
  configuration, artifactStore, clock,
}) {
  assertConfiguration(configuration, descriptor, manifest, schedule, liveHookManifest);
  for (const [index, ref] of (configuration.requiredArtifacts ?? []).entries()) {
    await readBoundJson(ref, descriptor.runRoot, `requiredArtifacts.${index}`);
  }
  const loggingSource = p158LoggingEvidenceSourceBinding();
  const frozenLoggingSource = liveHookManifest.hookBindings?.find((entry) =>
    entry.hookId === 'p158.logging_evidence_harvest');
  if (frozenLoggingSource?.sourcePath !== loggingSource.sourcePath ||
      frozenLoggingSource?.sourceSha256 !== loggingSource.sourceSha256 ||
      frozenLoggingSource?.implementationKind !== 'concrete_live') {
    fail('assembly_logging_harvester_unsealed', 'The concrete logging harvester source is absent from the live-hook manifest');
  }
  const registry = await readBoundJson(configuration.registry, descriptor.runRoot, 'registry');
  if (sha256(registry) !== schedule.registrySha256) fail('registry_identity_drift', 'Assembly registry differs from the frozen schedule');
  const [a01Ownership, a04Ownership, a13Ownership, a08ReplayManifest,
    externalWorkflowPlan, declaredTransitionPlan] = await Promise.all([
    readBoundJson(configuration.w7.a01A03Ownership, descriptor.runRoot, 'w7.a01A03Ownership'),
    readBoundJson(configuration.w7.a04A06Ownership, descriptor.runRoot, 'w7.a04A06Ownership'),
    readBoundJson(configuration.w7.a07A13Ownership, descriptor.runRoot, 'w7.a07A13Ownership'),
    readBoundJson(configuration.w7.a08ReplayManifest, descriptor.runRoot, 'w7.a08ReplayManifest'),
    readBoundJson(configuration.w9.externalWorkflowPlan, descriptor.runRoot, 'w9.externalWorkflowPlan'),
    readBoundJson(configuration.w9.declaredTransitionPlan, descriptor.runRoot, 'w9.declaredTransitionPlan'),
  ]);
  const receiptStore = { append: (receipt) => appendReceipt(artifactStore, descriptor.runId, receipt) };
  const a01A03LiveBundle = createP158W7A01A03LiveBundle({
    schedule, ownershipManifest: a01Ownership, receiptStore, clock: clock.wallNow,
  });
  const a04A06LiveBundle = createP158W7A04A06LiveBundle({
    schedule, ownershipManifest: a04Ownership, receiptStore, clock: clock.wallNow,
  });
  const a07A13LiveBundle = createP158W7A07A13LiveBundle({
    schedule, ownershipManifest: a13Ownership,
    receiptStore: createA13ReceiptStore(artifactStore, descriptor.runId), clock: clock.wallNow,
  });
  const a08LiveBundle = createP158W7A08LiveBundle({ schedule, replayManifest: a08ReplayManifest,
    receiptStore: createA08ReceiptStore(artifactStore, descriptor.runId), clock: clock.wallNow });
  const environmentSealSha256s = Object.fromEntries(manifest.environmentSeals
    .map((entry) => [entry.environmentId, entry.sealSha256]));
  const w7Readiness = auditP158W7LiveHookReadiness({
    candidateSha256: manifest.candidate.candidateSha256,
    environmentSealSha256s,
    a01A03LiveBundle,
    a04A06LiveBundle,
    a07A13LiveBundle,
    a08LiveBundle,
  });
  await writeExact(artifactStore, 'live-bundle-assembly/w7-live-hook-readiness.json', w7Readiness);
  const w7Bundle = createP158W7LiveDevelopmentAdapterBundle({
    schedule, target: clone(configuration.w7.target),
    primitives: createP158DevelopmentCommandPrimitives({ target: clone(configuration.w7.target) }),
    a01A03LiveBundle, a04A06LiveBundle, a07A13LiveBundle, a08LiveBundle,
    liveHookManifestSha256: liveHookManifest.manifestSha256,
  });
  const w8Bundle = createP158W8ReviewedLiveAdapterBundle({
    registry, schedule, seals: clone(configuration.w8.seals),
    operatorAssisted: clone(configuration.w8.operatorAssisted ?? { enabled: false }),
    externalActionExecution: clone(configuration.w8.externalActionExecution ?? null),
    h03ExternalExecution: clone(configuration.w8.h03ExternalExecution ?? null),
    dashboardCampaignExecution: clone(configuration.w8.dashboardCampaignExecution ?? null),
    liveHookManifestSha256: liveHookManifest.manifestSha256,
  });
  const w9Bundle = createP158W9ConcreteDriverBundle({
    schedule, target: clone(configuration.w9.target), artifactStore,
    externalWorkflowPlan, declaredTransitionPlan,
    caseWindows: clone(configuration.w9.caseWindows), c01: configuration.w9.c01
      ? { ...clone(configuration.w9.c01), clock, scheduler: liveScheduler }
      : null,
  });
  const w9Entries = createP158W9FreezeAdapterEntries({
    schedule, bundle: w9Bundle, liveHookManifestSha256: liveHookManifest.manifestSha256,
  });
  const w8Cases = new Map(registry.cases.map((entry) => [entry.id, entry]));
  const w8Logging = schedule.attempts.filter((attempt) =>
    w8Bundle.adapterBindings.find((entry) => entry.caseId === attempt.caseId)?.mode === 'concrete_live')
    .flatMap((attempt) => buildP158W8ActionPlan({ testCase: w8Cases.get(attempt.caseId), attempt }).actions
      .map((action) => ({
        expectationId: action.actionId, operationCorrelationId: action.actionId,
        productRequestId: null, productRequestIdState: 'assigned_at_runtime',
        requestKind: 'dashboard_action',
        correlationState: 'product_request_id_unavailable',
        actionId: action.actionId, attemptId: attempt.attemptId, caseId: attempt.caseId,
        phaseId: 'W8', environmentId: action.environmentId, operatorVisible: true,
        expectedSurfaceRoles: ['ingress_request', 'immediate_response', 'terminal_event', 'dashboard_projection'],
      })));
  const exactLoggingRequestExpectations = [
    ...a01A03LiveBundle.loggingRequestExpectations,
    ...a04A06LiveBundle.loggingRequestExpectations,
    ...w8Logging,
    ...w9Bundle.loggingRequestExpectations,
  ];
  const exactLoggingOperationGaps = clone([
    ...a07A13LiveBundle.loggingOperationDescriptors,
    ...a08LiveBundle.loggingOperationDescriptors,
  ]);
  if (sha256(configuration.loggingRequestExpectations) !== sha256(exactLoggingRequestExpectations)) {
    fail('logging_request_expectations_incomplete',
      'Frozen logging requests differ from concrete driver-emitted request and action identities');
  }
  const w9 = {
    target: clone(configuration.w9.target), caseWindows: clone(configuration.w9.caseWindows),
    drivers: w9Bundle.drivers, adapterBindings: w9Entries.adapterBindings,
    runRoot: descriptor.runRoot, artifactStore, clock,
    scheduler: liveScheduler,
    loggingRequestExpectations: clone(exactLoggingRequestExpectations),
    loggingOperationGaps: exactLoggingOperationGaps,
    loggingHarvest: createP158LoggingHarvestHook({
      configuration: { ...clone(configuration.loggingHarvest),
        loggingExpectations: clone(exactLoggingRequestExpectations),
        loggingExpectationsSha256: sha256(exactLoggingRequestExpectations),
        loggingOperationGaps: exactLoggingOperationGaps,
        loggingOperationGapsSha256: sha256(exactLoggingOperationGaps) },
      artifactStore, runRoot: descriptor.runRoot, clock,
    }),
  };
  verifyAdapterManifest(liveHookManifest, { w7Bundle, w8Bundle, w9 });
  const expectedPreparation = buildP158CampaignPhasePreparation({
    schedule, w7Bundle, w8Bundle, w9AdapterBindings: w9.adapterBindings,
    loggingRequestExpectations: w9.loggingRequestExpectations,
    loggingOperationGaps: w9.loggingOperationGaps,
    liveHookManifestSha256: liveHookManifest.manifestSha256, runId: descriptor.runId,
  });
  if (sha256(expectedPreparation) !== sha256(phasePreparation)) {
    fail('phase_preparation_drift', 'Constructed adapter classifications differ from the frozen phase preparation');
  }
  if (runtimeIdentity.runId !== descriptor.runId) fail('runtime_identity_drift', 'Assembly received another runtime identity');
  return {
    w7Bundle, w8Bundle, w9, w7Readiness,
    repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false,
  };
}

export function createP158LiveCampaignDescriptor({
  runRoot, runId, candidateExecutablePath, isolation, authorities, runtimeIdentityProbes,
  assemblyConfiguration, scheduledTeardown,
}) {
  if (!isAbsolute(runRoot ?? '') || !isAbsolute(candidateExecutablePath ?? '') ||
      !inside(runRoot, assemblyConfiguration?.path ?? '') ||
      !Array.isArray(runtimeIdentityProbes)) {
    fail('descriptor_input_invalid', 'Descriptor paths must be absolute and campaign-root confined');
  }
  for (const [field, ref] of Object.entries({ ...authorities, assemblyConfiguration })) {
    if (!isAbsolute(ref?.path ?? '') || !inside(runRoot, ref.path) || !SHA256.test(ref.sha256 ?? '')) {
      fail('descriptor_input_invalid', `${field} is not an exact campaign-root authority binding`);
    }
  }
  const sourceSha256 = createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
  const descriptor = {
    schemaVersion: 'agent-browser.p158-live-campaign-entrypoint.v1', planId: 'P158', runId,
    runtimeLane: 'development', production: false, runRoot, candidateExecutablePath,
    isolation: clone(isolation), manifest: clone(authorities.manifest), freeze: clone(authorities.freeze),
    schedule: clone(authorities.schedule), phasePreparation: clone(authorities.phasePreparation),
    liveHookManifest: clone(authorities.liveHookManifest), runtimeIdentity: clone(authorities.runtimeIdentity),
    runtimeIdentityProbes: clone(runtimeIdentityProbes),
    bundleAssembly: {
      sourcePath: P158_LIVE_ASSEMBLY_SOURCE_PATH, sourceSha256,
      exportName: 'constructP158LiveCampaignBundles',
      runtimeIdentityExport: 'readP158LiveCampaignRuntimeIdentity',
      configuration: clone(assemblyConfiguration),
    },
    scheduledTeardown: clone(scheduledTeardown), repairAllowed: false, retryAllowed: false,
    garbageCollectionAllowed: false,
  };
  const descriptorText = canonicalJson(descriptor);
  if (/(?:https?|wss?):\/\//iu.test(descriptorText) ||
      /"(?:authorization|cookie|password|secret|token|handoffUrl)"\s*:/iu.test(descriptorText)) {
    fail('descriptor_sensitive_value_prohibited', 'The live descriptor must contain only hashes and paths, never URLs or secret-bearing fields');
  }
  return Object.freeze({ descriptor: Object.freeze(descriptor), descriptorSha256: sha256(descriptorText) });
}

export function sealP158LiveBundleAssemblyConfiguration(input) {
  const body = clone(input);
  if (body?.schemaVersion !== 'agent-browser.p158-live-bundle-assembly-config.v1') {
    fail('assembly_configuration_invalid', 'Assembly configuration schemaVersion is required');
  }
  return Object.freeze({ ...body, configurationSha256: sha256(body) });
}

export function sealP158RuntimeIdentityProbe(input) {
  const body = clone(input);
  assertClosedWorldProjection(body?.identityProjection);
  if (!body?.probeSpecification) fail('runtime_identity_probe_invalid', 'Runtime identity probe specification is required');
  return Object.freeze({ ...body, probeSha256: sha256(body) });
}

export async function prepareP158RuntimeIdentityProbe({
  candidateExecutablePath, runRoot, environmentId, environment,
  expectedEnvironmentIdentity, probeSpecification, executeStatus, probePrimitives,
}) {
  if (!isAbsolute(candidateExecutablePath ?? '') || !isAbsolute(runRoot ?? '') ||
      typeof environmentId !== 'string') {
    fail('runtime_identity_probe_invalid', 'Runtime identity preparation requires absolute candidate and run-root paths');
  }
  assertProbeEnvironment(environment, runRoot);
  assertProbeSpecification(probeSpecification, runRoot, environmentId);
  const commandArgs = ['service', 'status', '--json'];
  const candidateSha256 = await fileDigest(probePrimitives ?? defaultProbePrimitives,
    candidateExecutablePath, 'Candidate binary');
  const status = await (executeStatus ?? executeStatusProbe)(candidateExecutablePath, commandArgs, environment);
  const hosts = valueAtPath(status, P158_RUNTIME_CALIBRATION_STATUS_PATHS.runtime_hosts);
  const matches = Array.isArray(hosts) ? hosts.filter((host) =>
    host?.binarySha256 === candidateSha256 && resolve(host.executablePath ?? '') === resolve(candidateExecutablePath)) : [];
  if (matches.length !== 1) {
    fail('runtime_identity_probe_failed', 'Service Status calibration did not expose one exact candidate runtime host');
  }
  const runtimeHostBinding = clone(matches[0]);
  const axes = await collectRuntimeIdentityAxes({
    candidateExecutablePath, runRoot, environmentId, environment, probeSpecification,
    runtimeHostBinding, primitives: probePrimitives,
  });
  if (valueAtPath(status, P158_RUNTIME_CALIBRATION_STATUS_PATHS.runtime_generation) !== axes.runtime_generation) {
    fail('runtime_identity_probe_failed', 'Service Status calibration and persisted runtime generation disagree');
  }
  return sealP158RuntimeIdentityProbe({
    environmentId, commandArgs, environment: clone(environment),
    probeSpecification: clone(probeSpecification),
    runtimeHostBinding,
    expectedEnvironmentIdentity: clone(expectedEnvironmentIdentity),
    identityProjection: P158_RUNTIME_IDENTITY_PROJECTION_AXES.map((axis) => ({
      axis, probeKind: P158_RUNTIME_IDENTITY_PROBE_KINDS[axis],
      expectedValue: clone(axes[axis]), valueSha256: sha256(axes[axis]),
    })),
  });
}

export function p158LiveCampaignAssemblySourceBinding() {
  return Object.freeze({
    hookId: 'p158.live_bundle_assembly', implementationKind: 'concrete_live',
    sourcePath: P158_LIVE_ASSEMBLY_SOURCE_PATH,
    sourceSha256: createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex'),
  });
}

export function p158LiveCampaignAssemblyHookBindings() {
  return Object.freeze([
    p158LiveCampaignAssemblySourceBinding(),
    Object.freeze({ hookId: 'p158.logging_evidence_harvest', implementationKind: 'concrete_live',
      ...p158LoggingEvidenceSourceBinding() }),
  ]);
}
