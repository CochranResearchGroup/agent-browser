#!/usr/bin/env node

import assert from 'node:assert/strict';
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { canonicalJson, createMemoryArtifactStore, sha256 } from './lib/p158-campaign-controller.js';
import {
  constructP158LiveCampaignBundles,
  constructP158W7SpecializedBundles,
  createP158LiveCampaignDescriptor,
  P158_RUNTIME_IDENTITY_PROJECTION_AXES,
  P158LiveCampaignAssemblyError,
  p158LiveCampaignAssemblySourceBinding,
  prepareP158RuntimeIdentityProbe,
  readP158LiveCampaignRuntimeIdentity,
  sealP158RuntimeIdentityProbe,
  sealP158LiveBundleAssemblyConfiguration,
} from './lib/p158-live-campaign-assembly.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import { createP158W7A01A03OwnershipManifest } from './lib/p158-w7-a01-a03-live.js';
import { createP158W7LiveDevelopmentAdapterBundle } from './lib/p158-w7-development-adapters.js';

const RUN_ID = 'p158-assembly-test';
const CANDIDATE = '11'.repeat(32);
function status(candidatePath, candidateSha256, specification, generation = 1) {
  return { data: {
    runtimeLifecycle: { lifecycle: { registryRevision: generation }, multiplicity: { runtimeHosts: [{
      binarySha256: candidateSha256, executablePath: candidatePath, pid: 4242,
      processStartToken: 'linux:test-boot:777', socketIdentity: 'socket-identity',
    }] } },
    statusProjection: { observations: { viewStreams: [{ id: 'view-1', state: 'ready' }] } },
    service_state: { browserProcessIdentities: { 'browser-1': {
      runtimeProfile: specification.profileName, userDataDir: specification.profilePath,
      processIdentity: candidateSha256,
    } } },
  } };
}

function probeSpecification(runRoot, environmentId) {
  return {
    configPath: join(runRoot, 'config', environmentId, 'config.json'),
    profileName: `profile-${environmentId}`,
    profilePath: join(runRoot, 'profiles', environmentId),
    socketDir: join(runRoot, 'sockets', environmentId),
    statePath: join(runRoot, 'state', environmentId, 'state.json'),
  };
}

function fakeProbePrimitives(candidatePath, environmentId) {
  return {
    readFile: async (path, encoding) => {
      if (path === '/proc/4242/stat') return `4242 (agent-browser) S ${Array(18).fill('0').join(' ')} 777 0`;
      if (path === '/proc/sys/kernel/random/boot_id') return 'test-boot\n';
      if (path === '/proc/4242/environ') return Buffer.from(
        `P158_CAMPAIGN_RUN_ID=${RUN_ID}\0P158_CAMPAIGN_ENVIRONMENT_ID=${typeof environmentId === 'function' ? environmentId() : environmentId}\0` +
        `AGENT_BROWSER_RUNTIME_PROFILE=profile-${typeof environmentId === 'function' ? environmentId() : environmentId}\0`,
      );
      return readFile(path, encoding);
    },
    readlink: async () => candidatePath,
    readdir: async () => [{ name: 'service.sock' }],
    lstat: async () => ({ isSocket: () => true, isSymbolicLink: () => false,
      dev: 2049, ino: 158, mode: 49_152 }),
  };
}

async function withRoot(body) {
  const root = await mkdtemp(join(tmpdir(), 'p158-live-assembly-'));
  try { await body(root); } finally { await rm(root, { recursive: true, force: true }); }
}

async function bound(path, value) {
  await mkdir(join(path, '..'), { recursive: true });
  await writeFile(path, canonicalJson(value));
  return { path, sha256: sha256(await readFile(path)) };
}

function identity(environmentId) {
  const identityValue = {
    runtimeLane: 'development', production: false, campaignRunId: RUN_ID,
    candidateSha256: CANDIDATE, ownership: 'p158_campaign', foreign: false,
    tenantDataPresent: false, isolationState: 'isolated', environmentId,
  };
  return { environmentId, identity: identityValue, identitySha256: sha256(identityValue) };
}

async function runTest(name, body) {
  try { await body(); process.stdout.write(`PASS ${name}\n`); }
  catch (error) { error.message = `${name}: ${error.message}`; throw error; }
}

await runTest('reads exact fresh isolated runtime probes without mutating input', () => withRoot(async (runRoot) => {
  const environments = ['E0', 'E1', 'E2', 'E3'].map(identity);
  const candidatePath = join(runRoot, 'agent-browser-dev');
  await writeFile(candidatePath, '#!/bin/sh\nexit 0\n');
  await chmod(candidatePath, 0o700);
  const candidateSha256 = sha256(await readFile(candidatePath));
  let generation = 1;
  const probes = [];
  for (const entry of environments.filter((item) => ['E1', 'E2'].includes(item.environmentId))) {
    const specification = probeSpecification(runRoot, entry.environmentId);
    for (const path of [specification.configPath, specification.statePath,
      join(specification.socketDir, 'service.sock')]) {
      await mkdir(join(path, '..'), { recursive: true });
    }
    await writeFile(specification.configPath, '{}\n');
    await writeFile(specification.statePath, canonicalJson({
      runtimeOwnerRegistry: { revision: generation },
      browserProcessIdentities: { 'browser-1': { runtimeProfile: specification.profileName,
        userDataDir: specification.profilePath, processIdentity: candidateSha256 } },
      remoteViewHandoffs: { 'handoff-1': { state: 'ready' } },
    }));
    probes.push(await prepareP158RuntimeIdentityProbe({
      candidateExecutablePath: candidatePath, runRoot, environmentId: entry.environmentId,
      environment: { HOME: join(runRoot, 'home', entry.environmentId),
        AGENT_BROWSER_RUNTIME_PROFILE: specification.profileName,
        P158_CAMPAIGN_RUN_ID: RUN_ID, P158_CAMPAIGN_ENVIRONMENT_ID: entry.environmentId },
      expectedEnvironmentIdentity: entry, probeSpecification: specification,
      executeStatus: async () => status(candidatePath, candidateSha256, specification, generation),
      probePrimitives: fakeProbePrimitives(candidatePath, entry.environmentId),
    }));
  }
  const expected = {
    schemaVersion: 'agent-browser.p158-current-runtime-identity.v1', runtimeLane: 'development',
    production: false, runId: RUN_ID, candidateSha256: CANDIDATE, environments,
    repairAllowed: false, retryAllowed: false, garbageCollectionAllowed: false,
  };
  const input = {
    descriptor: { runId: RUN_ID, runRoot, candidateExecutablePath: candidatePath,
      runtimeIdentity: { path: join(runRoot, 'freeze', 'identity.json') }, runtimeIdentityProbes: probes },
    manifest: { candidate: { candidateSha256: CANDIDATE }, environmentSeals: environments },
    expectedRuntimeIdentity: expected,
    isolation: { home: join(runRoot, 'home'), xdgStateHome: join(runRoot, 'state') },
  };
  const frozen = structuredClone(input);
  const liveFiles = probes.flatMap((probe) => [probe.probeSpecification.configPath, probe.probeSpecification.statePath]);
  const beforeFileDigests = await Promise.all(liveFiles.map(async (path) => sha256(await readFile(path))));
  assert.deepEqual(await readP158LiveCampaignRuntimeIdentity({ ...input,
    probePrimitives: (environmentId) => fakeProbePrimitives(candidatePath, environmentId) }), expected);
  assert.deepEqual(await Promise.all(liveFiles.map(async (path) => sha256(await readFile(path)))), beforeFileDigests,
    'entry identity readback changed config or Service State');
  assert.deepEqual(input, frozen);
  await assert.rejects(() => readP158LiveCampaignRuntimeIdentity({ ...input,
    descriptor: { ...input.descriptor, runtimeIdentityProbes: [probes[0], { ...probes[1], environmentId: 'E3' }] },
    probePrimitives: (environmentId) => fakeProbePrimitives(candidatePath, environmentId) }),
  (error) => error.code === 'runtime_identity_probe_missing');
  generation = 2;
  for (const entry of environments.filter((item) => ['E1', 'E2'].includes(item.environmentId))) {
    const specification = probeSpecification(runRoot, entry.environmentId);
    const stateValue = JSON.parse(await readFile(specification.statePath, 'utf8'));
    stateValue.runtimeOwnerRegistry.revision = generation;
    await writeFile(specification.statePath, canonicalJson(stateValue));
  }
  await assert.rejects(() => readP158LiveCampaignRuntimeIdentity({ ...input,
    descriptor: { ...input.descriptor, runtimeIdentityProbes: probes },
    probePrimitives: (environmentId) => fakeProbePrimitives(candidatePath, environmentId) }),
  (error) => error.code === 'runtime_identity_drift');
}));

await runTest('seals deterministic configuration and emits a URL-free absolute descriptor', () => withRoot(async (runRoot) => {
  const authorities = {};
  for (const name of ['manifest', 'freeze', 'schedule', 'phasePreparation', 'liveHookManifest', 'runtimeIdentity']) {
    authorities[name] = await bound(join(runRoot, 'freeze', `${name}.json`), { name });
  }
  const configInput = {
    schemaVersion: 'agent-browser.p158-live-bundle-assembly-config.v1', runId: RUN_ID,
    candidateSha256: CANDIDATE, scheduleSha256: '22'.repeat(32), liveHookManifestSha256: '33'.repeat(32),
    runtimeLane: 'development', production: false, repairAllowed: false, retryAllowed: false,
    garbageCollectionAllowed: false, requiredArtifacts: [], w7: {}, w8: {}, w9: {},
  };
  const frozen = structuredClone(configInput);
  const configuration = sealP158LiveBundleAssemblyConfiguration(configInput);
  assert.equal(configuration.configurationSha256, sha256(configInput));
  assert.deepEqual(configInput, frozen);
  const configRef = await bound(join(runRoot, 'freeze', 'assembly.json'), configuration);
  const probe = { probeSha256: sha256('probe'), environmentId: 'E0' };
  const generated = createP158LiveCampaignDescriptor({
    runRoot, runId: RUN_ID, candidateExecutablePath: join(runRoot, 'candidate', 'agent-browser-dev'),
    isolation: { home: join(runRoot, 'home'), xdgConfigHome: join(runRoot, 'config'),
      xdgRuntimeDir: join(runRoot, 'runtime'), xdgStateHome: join(runRoot, 'state') },
    authorities, runtimeIdentityProbes: [probe], assemblyConfiguration: configRef,
    scheduledTeardown: { caseId: 'TEARDOWN', attemptId: 'TEARDOWN-E0', environmentId: 'E0' },
  });
  assert.equal(generated.descriptorSha256, sha256(canonicalJson(generated.descriptor)));
  assert.equal(generated.descriptor.bundleAssembly.sourceSha256,
    p158LiveCampaignAssemblySourceBinding().sourceSha256);
  assert.doesNotMatch(canonicalJson(generated.descriptor), /https?:|remote-view|authorization|cookie/iu);
}));

await runTest('rejects a weak or incomplete runtime identity projection', () => withRoot(async (runRoot) => {
  assert.throws(() => sealP158RuntimeIdentityProbe({
    environmentId: 'E0', commandArgs: ['service', 'status', '--json'],
    environment: { HOME: join(runRoot, 'home') }, expectedEnvironmentIdentity: identity('E0'),
    probeSpecification: probeSpecification(runRoot, 'E0'),
    identityProjection: [{ axis: 'runtime_generation', probeKind: 'service_state_runtime_owner_revision',
      expectedValue: 1, valueSha256: sha256(1) }],
  }), (error) => error.code === 'runtime_identity_probe_invalid');
  const complete = P158_RUNTIME_IDENTITY_PROJECTION_AXES.map((axis) => ({
    axis, probeKind: axis === 'runtime_generation' ? 'service_state_runtime_owner_revision' : 'invalid',
    expectedValue: axis === 'runtime_generation' ? 1 : sha256(axis),
    valueSha256: sha256(axis === 'runtime_generation' ? 1 : sha256(axis)),
  }));
  for (const malformed of [
    complete,
    complete.map((field, index) => index === 1 ? { ...field, axis: 'candidate_binary_identity' } : field),
  ]) {
    assert.throws(() => sealP158RuntimeIdentityProbe({
      environmentId: 'E0', commandArgs: ['service', 'status', '--json'],
      environment: { HOME: join(runRoot, 'home') }, expectedEnvironmentIdentity: identity('E0'),
      probeSpecification: probeSpecification(runRoot, 'E0'),
      identityProjection: malformed,
    }), (error) => error.code === 'runtime_identity_probe_invalid');
  }
}));

await runTest('fails closed before bundle construction when a concrete receipt is absent or changed', () => withRoot(async (runRoot) => {
  const missing = { path: join(runRoot, 'receipts', 'external.json'), sha256: '44'.repeat(32) };
  const configuration = sealP158LiveBundleAssemblyConfiguration({
    schemaVersion: 'agent-browser.p158-live-bundle-assembly-config.v1', runId: RUN_ID,
    candidateSha256: CANDIDATE, scheduleSha256: '22'.repeat(32), liveHookManifestSha256: '33'.repeat(32),
    runtimeLane: 'development', production: false, repairAllowed: false, retryAllowed: false,
    garbageCollectionAllowed: false, requiredArtifacts: [missing], w7: {}, w8: {}, w9: {}, registry: missing,
  });
  await assert.rejects(() => constructP158LiveCampaignBundles({
    descriptor: { runId: RUN_ID, runRoot }, manifest: { candidate: { candidateSha256: CANDIDATE } },
    schedule: { scheduleSha256: '22'.repeat(32) }, phasePreparation: {},
    liveHookManifest: { manifestSha256: '33'.repeat(32) }, runtimeIdentity: { runId: RUN_ID },
    configuration, artifactStore: createMemoryArtifactStore(), clock: { wallNow: () => new Date().toISOString() },
  }), (error) => error instanceof P158LiveCampaignAssemblyError && error.code === 'assembly_artifact_missing');
  await mkdir(join(runRoot, 'receipts'), { recursive: true });
  await writeFile(missing.path, '{}\n');
  await assert.rejects(() => constructP158LiveCampaignBundles({
    descriptor: { runId: RUN_ID, runRoot }, manifest: { candidate: { candidateSha256: CANDIDATE } },
    schedule: { scheduleSha256: '22'.repeat(32) }, phasePreparation: {},
    liveHookManifest: { manifestSha256: '33'.repeat(32) }, runtimeIdentity: { runId: RUN_ID },
    configuration, artifactStore: createMemoryArtifactStore(), clock: { wallNow: () => new Date().toISOString() },
  }), (error) => error.code === 'assembly_artifact_changed');
}));

await runTest('omitted W7 ownership keeps its cases blocked without suppressing supplied live cases', () => withRoot(async (runRoot) => {
  const registry = JSON.parse(await readFile('docs/dev/contracts/p158-historical-failure-registry.v1.json', 'utf8'));
  const schedule = compileP158ExecutionSchedule({ registry, seed: RUN_ID });
  const hookSha256 = '33'.repeat(32);
  const ownership = createP158W7A01A03OwnershipManifest({
    schemaVersion: 'agent-browser.p158-w7-a01-a03-ownership.v1',
    campaignRunId: RUN_ID, candidateSha256: CANDIDATE, liveHookManifestSha256: hookSha256,
    environmentSealSha256s: { E0: '44'.repeat(32), E1: '55'.repeat(32) },
    environments: Object.fromEntries(['E0', 'E1'].map((environmentId) => [environmentId, {
      serviceOrigin: 'http://127.0.0.1:43158', runtimeLane: 'development', production: false,
      runtimeEnvironmentId: environmentId, targetId: `target-${environmentId}`,
      ownershipStatus: { runtimeEnvironment: 'development' },
    }])),
    fixtures: Object.fromEntries(['A01', 'A02', 'A03'].map((caseId) => [caseId,
      Object.fromEntries(['E0', 'E1'].map((environmentId) => [environmentId, {
        profileId: `${caseId}-${environmentId}`, sessionName: `p158-${caseId}-${environmentId}`,
        url: 'http://127.0.0.1:43158/synthetic', browserId: `browser-${caseId}-${environmentId}`,
        sharedLabel: 'same-label',
      }])),
    ])),
  });
  const configuration = { w7: {
    a01A03Ownership: await bound(join(runRoot, 'a01-a03.json'), ownership),
  } };
  const input = { descriptor: { runId: RUN_ID, runRoot }, schedule, configuration,
    artifactStore: createMemoryArtifactStore(), clock: { wallNow: () => '2026-09-05T00:00:00Z' } };
  const before = structuredClone(configuration);
  const specialized = await constructP158W7SpecializedBundles(input);
  assert.deepEqual(configuration, before);
  assert.deepEqual(specialized.a01A03LiveBundle.concreteCaseIds, ['A01', 'A02', 'A03']);
  for (const field of ['a04A06LiveBundle', 'a07A13LiveBundle', 'a08LiveBundle']) {
    assert.equal(specialized[field], null);
  }
  const bundle = createP158W7LiveDevelopmentAdapterBundle({ schedule, ...specialized,
    target: { targetId: 'p158-target', campaignRunId: RUN_ID, candidateSha256: CANDIDATE,
      runtimeLane: 'development', isolationState: 'isolated', ownership: 'p158_campaign',
      production: false, foreign: false, tenantDataPresent: false },
    primitives: Object.fromEntries(['captureEvidence', 'captureLogs', 'runCliCommand', 'runBrowserAction',
      'signalProcess', 'runSystemdAction', 'runDisplayAction', 'runShutdownAction']
      .map((name) => [name, async () => { assert.fail(`Unexpected effect ${name}`); }])),
    liveHookManifestSha256: hookSha256,
  });
  assert.deepEqual(bundle.adapterBindings.filter((entry) => entry.mode === 'concrete_live')
    .map((entry) => entry.caseId), ['A01', 'A02', 'A03']);
  for (const caseId of ['A05', 'A07', 'A08', 'A13', 'X06']) {
    const binding = bundle.adapterBindings.find((entry) => entry.caseId === caseId);
    assert.equal(binding.mode, 'explicit_blocked');
    assert.equal(binding.effectsAllowed, false);
    assert.equal(binding.blocker.code, 'live_case_hook_missing');
    assert.equal(binding.implementedActionCount, 0);
    const result = await bundle.w7Adapters.find((adapter) => adapter.caseId === caseId).execute({});
    assert.equal(result.resultState, 'skipped_blocked');
    assert.equal(result.effectState, 'not_started');
  }
  assert.deepEqual(Object.keys(bundle.effects), []);
  assert.equal(bundle.reviewedLiveDispatcher, null);
  assert.equal(bundle.executedActionIds.size, 0);
  const empty = await constructP158W7SpecializedBundles({ ...input, configuration: { w7: {} } });
  assert.ok(Object.values(empty).every((value) => value === null));
}));

await runTest('supplied optional W7 references retain file, digest and manifest validation', () => withRoot(async (runRoot) => {
  const input = { descriptor: { runId: RUN_ID, runRoot }, schedule: {},
    artifactStore: createMemoryArtifactStore(), clock: { wallNow: () => '2026-09-05T00:00:00Z' } };
  for (const field of ['a01A03Ownership', 'a04A06Ownership', 'a07A13Ownership', 'a08ReplayManifest']) {
    const reference = { path: join(runRoot, `${field}.json`), sha256: '44'.repeat(32) };
    const construct = (ref) => constructP158W7SpecializedBundles({ ...input,
      configuration: { w7: { [field]: ref } } });
    await assert.rejects(() => construct(reference), (error) => error.code === 'assembly_artifact_missing');
    await writeFile(reference.path, 'null\n');
    await assert.rejects(() => construct(reference), (error) => error.code === 'assembly_artifact_changed');
    await assert.rejects(() => construct(null), (error) => error.code === 'assembly_path_invalid');
    const exact = { ...reference, sha256: sha256(await readFile(reference.path)) };
    await assert.rejects(() => construct(exact), 'A supplied null JSON manifest must not become an omission');
    await writeFile(reference.path, '{}\n');
    const malformed = { ...reference, sha256: sha256(await readFile(reference.path)) };
    await assert.rejects(() => construct(malformed),
      'A malformed supplied ownership manifest must not become an omission');
  }
}));

await runTest('assembles the W10 hook with sealed authorities and terminal artifact inventory', async () => {
  const source = await readFile('scripts/lib/p158-live-campaign-assembly.js', 'utf8');
  assert.match(source, /createP158FinalAnalysisDescriptorHook/);
  assert.match(source, /manifest: descriptor\.manifest/);
  assert.match(source, /registry: configuration\.registry/);
  assert.match(source, /byteCount: bytes\.byteLength/);
  assert.match(source, /resolveRawArtifactInventory: \(\{ snapshot \}\)/);
  assert.match(source, /loggingOperationGapCount: exactLoggingOperationGaps\.length/);
});

process.stdout.write('P158 live campaign assembly test passed\n');
