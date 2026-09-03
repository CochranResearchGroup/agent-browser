#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir, tmpdir } from 'node:os';
import { join } from 'node:path';

import { canonicalJson, sha256 } from './lib/p158-campaign-controller.js';
import {
  buildP158CampaignPhasePreparation,
  runP158CampaignPhases,
} from './lib/p158-campaign-phase-orchestrator.js';
import {
  P158LiveCampaignEntrypointError,
  runP158LiveCampaignEntrypoint,
} from './lib/p158-live-campaign-entrypoint.js';

const SOURCE_PATH = 'scripts/lib/p158-live-campaign-entrypoint.js';
const SOURCE_SHA256 = sha256(await readFile(new URL('./lib/p158-live-campaign-entrypoint.js', import.meta.url)));
const COMMIT = '1'.repeat(40);
const NOW = '2026-09-03T12:00:00.000Z';

function sealed(value, field) {
  const body = structuredClone(value);
  return { ...body, [field]: sha256(body) };
}

function scheduleFixture() {
  const attempts = [
    { scheduleSequence: 0, scheduleId: 'schedule:A01', caseId: 'A01', attemptId: 'A01-E1-r001',
      repetition: 1, seed: 'seed-a', phaseId: 'W7', environmentId: 'E1', environmentIds: ['E1'],
      dependsOnAttemptIds: [], preconditionIds: [], stimuli: [], evidenceProfile: 'service',
      externalIngressRequired: false, declaredEffectIds: ['effect:a'], preExecutionBlocker: null },
    { scheduleSequence: 1, scheduleId: 'schedule:D01', caseId: 'D01', attemptId: 'D01-E2-r001',
      repetition: 1, seed: 'seed-d', phaseId: 'W8', environmentId: 'E2', environmentIds: ['E2'],
      dependsOnAttemptIds: [], preconditionIds: [], stimuli: [], evidenceProfile: 'dashboard',
      externalIngressRequired: true, declaredEffectIds: ['effect:d'], preExecutionBlocker: null },
  ];
  const body = {
    schemaVersion: 'agent-browser.p158-execution-schedule.v1', planId: 'P158', registrySha256: '2'.repeat(64),
    caseCount: 2, attemptCount: 2,
    caseContracts: [
      { caseId: 'A01', phaseId: 'W7', adapterId: 'adapter-a', executionContractSha256: '3'.repeat(64), declaredEffectIds: ['effect:a'] },
      { caseId: 'D01', phaseId: 'W8', adapterId: 'adapter-d', executionContractSha256: '4'.repeat(64), declaredEffectIds: ['effect:d'] },
    ],
    attempts,
  };
  return { ...body, scheduleSha256: sha256(body) };
}

function bindingsFor(schedule) {
  return schedule.caseContracts.map((contract) => ({
    caseId: contract.caseId, adapterId: contract.adapterId,
    executionContractSha256: contract.executionContractSha256,
    mode: 'explicit_blocked', providerFree: false, sourcePath: SOURCE_PATH, sourceSha256: SOURCE_SHA256,
    hookIds: [], implementedActionCount: 0, blockedActionCount: 1, effectsAllowed: false,
    blocker: { code: 'fixture_blocked', detail: `${contract.caseId} intentionally blocked` },
  }));
}

function bundleFor(phaseId, schedule, liveHookManifestSha256, invocations) {
  const bindings = bindingsFor(schedule).filter((entry) => entry.caseId.startsWith(phaseId === 'W7' ? 'A' : 'D'));
  const adapters = bindings.map((binding) => ({
    caseId: binding.caseId, executionMode: binding.mode, providerFree: false, effectsAllowed: false,
    sourcePath: binding.sourcePath, sourceSha256: binding.sourceSha256, liveHookManifestSha256,
    liveBindingSha256: sha256(binding), liveHookIds: [], blocker: { ...binding.blocker,
      sourcePath: binding.sourcePath, sourceSha256: binding.sourceSha256 },
    execute: async () => { invocations.push(binding.caseId); throw new Error('blocked adapter was invoked'); },
  }));
  return { [phaseId === 'W7' ? 'w7Adapters' : 'w8Adapters']: adapters, adapterBindings: bindings, effects: {} };
}

async function createFixture(label) {
  const runRoot = await mkdtemp(join(tmpdir(), `p158-entrypoint-${label}-`));
  const paths = {
    manifest: join(runRoot, 'campaign-manifest.json'), freeze: join(runRoot, 'campaign-freeze.json'),
    schedule: join(runRoot, 'freeze', 'execution-schedule.json'), phasePreparation: join(runRoot, 'freeze', 'phase-preparation.json'),
    liveHookManifest: join(runRoot, 'freeze', 'live-hooks.json'), runtimeIdentity: join(runRoot, 'freeze', 'runtime-identity.json'),
    assemblyConfig: join(runRoot, 'freeze', 'bundle-assembly.json'), descriptor: join(runRoot, 'live-entrypoint.json'),
    candidate: join(runRoot, 'candidate', 'agent-browser-dev'),
  };
  await mkdir(join(runRoot, 'freeze'), { recursive: true });
  await mkdir(join(runRoot, 'candidate'), { recursive: true });
  const candidateBytes = Buffer.from('exact development candidate');
  await writeFile(paths.candidate, candidateBytes);
  const schedule = scheduleFixture();
  const candidateBody = {
    runId: `run-${label}`, sourceCommit: COMMIT, binarySha256: sha256(candidateBytes), dashboardSha256: '5'.repeat(64),
    installedGenerationId: 'development-generation', browserExecutableSha256: '6'.repeat(64),
    runtimeManifestRevision: 'development-runtime', providerConfigurationRevision: 'development-provider',
    externalIngressDeploymentRevision: 'reviewed-ingress', aggregateFixtureManifestSha256: '7'.repeat(64), preparedAt: NOW,
  };
  const candidate = { ...candidateBody, candidateSha256: sha256(candidateBody) };
  const identities = [
    { environmentId: 'E1', identity: { runtimeLane: 'development', environmentId: 'E1', generation: 'one' } },
    { environmentId: 'E2', identity: { runtimeLane: 'development', environmentId: 'E2', generation: 'two' } },
  ].map((entry) => ({ ...entry, identitySha256: sha256(entry.identity) }));
  const environmentSeals = identities.map((entry) => ({
    environmentId: entry.environmentId, identityId: `identity-${entry.environmentId}`,
    identitySha256: entry.identitySha256, sealSha256: '8'.repeat(64), sealedAt: NOW, receiptArtifactIds: ['receipt'],
  }));
  const hookBody = {
    schemaVersion: 'agent-browser.p158-live-hook-manifest.v1', planId: 'P158', manifestId: `hooks-${label}`,
    capturedAt: NOW, mode: 'concrete_live', providerFree: false, aggregateSha256: '9'.repeat(64),
    scheduleSha256: schedule.scheduleSha256, candidateSha256: candidate.candidateSha256,
    hookBindings: [{ hookId: 'entrypoint.fixture', implementationKind: 'concrete_live', sourcePath: SOURCE_PATH, sourceSha256: SOURCE_SHA256 }],
    adapterBindings: bindingsFor(schedule), repairAllowed: false, retryAllowed: false, garbageCollectionAllowed: false,
  };
  const liveHooks = sealed(hookBody, 'manifestSha256');
  const invocations = [];
  const w7Bundle = bundleFor('W7', schedule, liveHooks.manifestSha256, invocations);
  const w8Bundle = bundleFor('W8', schedule, liveHooks.manifestSha256, invocations);
  const phasePreparation = buildP158CampaignPhasePreparation({
    schedule, w7Bundle, w8Bundle, liveHookManifestSha256: liveHooks.manifestSha256, runId: candidate.runId,
  });
  const runtimeIdentity = {
    schemaVersion: 'agent-browser.p158-current-runtime-identity.v1', runtimeLane: 'development', production: false,
    runId: candidate.runId, candidateSha256: candidate.candidateSha256, environments: identities,
    repairAllowed: false, retryAllowed: false, garbageCollectionAllowed: false,
  };
  const assemblyConfig = { schemaVersion: 'agent-browser.p158-test-bundle-assembly.v1', runId: candidate.runId };
  await writeFile(paths.schedule, canonicalJson(schedule));
  await writeFile(paths.phasePreparation, canonicalJson(phasePreparation));
  await writeFile(paths.liveHookManifest, canonicalJson(liveHooks));
  await writeFile(paths.runtimeIdentity, canonicalJson(runtimeIdentity));
  await writeFile(paths.assemblyConfig, canonicalJson(assemblyConfig));
  const binding = async (path) => ({ path, sha256: sha256(await readFile(path)) });
  const artifactBindings = [
    { artifactId: 'execution-schedule', kind: 'execution_schedule', relativePath: 'freeze/execution-schedule.json',
      sha256: sha256(await readFile(paths.schedule)), byteCount: (await readFile(paths.schedule)).byteLength, capturedAt: NOW },
    { artifactId: 'live-hooks', kind: 'live_hook_manifest', relativePath: 'freeze/live-hooks.json',
      sha256: sha256(await readFile(paths.liveHookManifest)), byteCount: (await readFile(paths.liveHookManifest)).byteLength, capturedAt: NOW },
  ];
  const manifest = {
    schemaVersion: 'agent-browser.p158-campaign-manifest.v1', planId: 'P158', runId: candidate.runId,
    registrySha256: schedule.registrySha256, controllerState: 'prepared', candidate, artifactBindings, environmentSeals,
    calibration: { clean: true }, fixtureSeal: { fixtureId: 'synthetic' }, freezeContract: { freezeId: `freeze-${label}` },
    schedule: schedule.attempts.map(({ phaseId: _phaseId, environmentId: _environmentId, declaredEffectIds: _effects, ...entry }) => entry),
    freezePolicy: {}, safetyPolicy: {}, evidencePolicy: {},
  };
  await writeFile(paths.manifest, canonicalJson(manifest));
  const freeze = {
    schemaVersion: 'agent-browser.p158-campaign-freeze.v1', planId: 'P158', runId: candidate.runId,
    freezeId: `freeze-${label}`, controllerState: 'frozen', manifestSha256: sha256(await readFile(paths.manifest)),
    candidateSha256: candidate.candidateSha256, artifactBindingsSha256: sha256(artifactBindings),
    environmentSealsSha256: sha256(environmentSeals), calibrationSha256: sha256(manifest.calibration),
    fixtureSealSha256: sha256(manifest.fixtureSeal), preparedLedgerHeadSha256: 'a'.repeat(64), frozenAt: NOW,
    monotonicTimeNanoseconds: 1, startedCaseCount: 0, startedAttemptCount: 0,
  };
  await writeFile(paths.freeze, canonicalJson(freeze));
  const descriptor = {
    schemaVersion: 'agent-browser.p158-live-campaign-entrypoint.v1', planId: 'P158', runId: candidate.runId,
    runtimeLane: 'development', production: false, runRoot, candidateExecutablePath: paths.candidate,
    isolation: { home: join(runRoot, 'home'), xdgConfigHome: join(runRoot, 'xdg-config'),
      xdgRuntimeDir: join(runRoot, 'xdg-runtime'), xdgStateHome: join(runRoot, 'xdg-state') },
    manifest: await binding(paths.manifest), freeze: await binding(paths.freeze), schedule: await binding(paths.schedule),
    phasePreparation: await binding(paths.phasePreparation), liveHookManifest: await binding(paths.liveHookManifest),
    runtimeIdentity: await binding(paths.runtimeIdentity),
    bundleAssembly: { sourcePath: SOURCE_PATH, sourceSha256: SOURCE_SHA256,
      exportName: 'constructP158LiveCampaignBundles', runtimeIdentityExport: 'readP158LiveCampaignRuntimeIdentity',
      configuration: await binding(paths.assemblyConfig) },
    scheduledTeardown: { caseId: 'TEARDOWN', attemptId: 'TEARDOWN-E0', environmentId: 'E0' },
    repairAllowed: false, retryAllowed: false, garbageCollectionAllowed: false,
  };
  await writeFile(paths.descriptor, canonicalJson(descriptor));
  return { runRoot, paths, descriptor, descriptorSha256: sha256(await readFile(paths.descriptor)), schedule,
    phasePreparation, liveHooks, w7Bundle, w8Bundle, invocations,
    cleanup: () => rm(runRoot, { recursive: true, force: true }) };
}

async function expectCode(code, action) {
  await assert.rejects(action, (error) => {
    assert(error instanceof P158LiveCampaignEntrypointError || error?.code === code, String(error));
    assert.equal(error.code, code);
    return true;
  });
}

function options(fixture, additions = {}) {
  return {
    descriptorPath: fixture.paths.descriptor, descriptorSha256: fixture.descriptorSha256,
    clock: { wallNow: () => NOW }, testing: true, sourceCommitReadback: async () => COMMIT,
    bundleAssemblyLoader: async () => ({
      readP158LiveCampaignRuntimeIdentity: async () => JSON.parse(await readFile(fixture.paths.runtimeIdentity, 'utf8')),
      constructP158LiveCampaignBundles: async () => ({
        w7Bundle: fixture.w7Bundle, w8Bundle: fixture.w8Bundle,
        w9: { target: { runId: fixture.descriptor.runId } },
        repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false,
      }),
    }),
    ...additions,
  };
}

const exact = await createFixture('exact');
try {
  let w9Calls = 0;
  const terminal = await runP158LiveCampaignEntrypoint(options(exact, {
    runCampaignPhases: (input) => runP158CampaignPhases({ ...input, runW9: async ({ controller }) => {
      w9Calls += 1;
      if (!controller.snapshot().scheduledTeardown.resultState) {
        await controller.recordScheduledTeardown({ resultState: 'passed', effectState: 'verified_effect' });
      }
      await controller.finishExecution();
      await controller.sealEvidence();
      return { state: 'evidence_sealed' };
    } }),
  }));
  assert.equal(terminal.outcome, 'completed');
  assert.equal(w9Calls, 1);
  assert.deepEqual(exact.invocations, [], 'explicit blockers must not invoke their adapters');
  const resumed = await runP158LiveCampaignEntrypoint(options(exact, {
    bundleAssemblyLoader: async () => assert.fail('a terminal campaign must not reconstruct bundles'),
  }));
  assert.equal(resumed.checkpointSha256, terminal.checkpointSha256);
} finally { await exact.cleanup(); }

const changedPreparation = await createFixture('changed-preparation');
try {
  const value = JSON.parse(await readFile(changedPreparation.paths.phasePreparation, 'utf8'));
  value.loggingExpectations[0].operatorVisible = !value.loggingExpectations[0].operatorVisible;
  await writeFile(changedPreparation.paths.phasePreparation, canonicalJson(value));
  let assemblyLoaded = false;
  await expectCode('authority_digest_mismatch', () => runP158LiveCampaignEntrypoint(options(changedPreparation, {
    bundleAssemblyLoader: async () => { assemblyLoaded = true; return {}; },
  })));
  assert.equal(assemblyLoaded, false);
} finally { await changedPreparation.cleanup(); }

const candidateDrift = await createFixture('candidate-drift');
try {
  await writeFile(candidateDrift.paths.candidate, 'changed candidate bytes');
  await expectCode('candidate_binary_drift', () => runP158LiveCampaignEntrypoint(options(candidateDrift)));
} finally { await candidateDrift.cleanup(); }

const sourceDrift = await createFixture('source-drift');
try {
  await expectCode('source_commit_drift', () => runP158LiveCampaignEntrypoint(options(sourceDrift, {
    sourceCommitReadback: async () => 'f'.repeat(40),
  })));
} finally { await sourceDrift.cleanup(); }

const configDrift = await createFixture('config-drift');
try {
  await writeFile(configDrift.paths.assemblyConfig, '{}\n');
  await expectCode('authority_digest_mismatch', () => runP158LiveCampaignEntrypoint(options(configDrift)));
} finally { await configDrift.cleanup(); }

const runtimeDrift = await createFixture('runtime-drift');
try {
  const runtime = JSON.parse(await readFile(runtimeDrift.paths.runtimeIdentity, 'utf8'));
  runtime.environments[0].identity.generation = 'changed';
  await writeFile(runtimeDrift.paths.runtimeIdentity, canonicalJson(runtime));
  runtimeDrift.descriptor.runtimeIdentity.sha256 = sha256(await readFile(runtimeDrift.paths.runtimeIdentity));
  await writeFile(runtimeDrift.paths.descriptor, canonicalJson(runtimeDrift.descriptor));
  runtimeDrift.descriptorSha256 = sha256(await readFile(runtimeDrift.paths.descriptor));
  await expectCode('runtime_identity_drift', () => runP158LiveCampaignEntrypoint(options(runtimeDrift)));
} finally { await runtimeDrift.cleanup(); }

const liveRuntimeDrift = await createFixture('live-runtime-drift');
try {
  const current = JSON.parse(await readFile(liveRuntimeDrift.paths.runtimeIdentity, 'utf8'));
  current.environments[1].identity.generation = 'currently-changed';
  current.environments[1].identitySha256 = sha256(current.environments[1].identity);
  let constructed = false;
  await expectCode('runtime_identity_drift', () => runP158LiveCampaignEntrypoint(options(liveRuntimeDrift, {
    bundleAssemblyLoader: async () => ({
      readP158LiveCampaignRuntimeIdentity: async () => current,
      constructP158LiveCampaignBundles: async () => { constructed = true; return {}; },
    }),
  })));
  assert.equal(constructed, false, 'runtime identity drift must fail before bundle construction');
} finally { await liveRuntimeDrift.cleanup(); }

const failedTerminal = await createFixture('failed-terminal');
try {
  let assemblyCalls = 0;
  const failingOptions = options(failedTerminal, {
    bundleAssemblyLoader: async () => {
      assemblyCalls += 1;
      return {
        readP158LiveCampaignRuntimeIdentity: async () => JSON.parse(await readFile(failedTerminal.paths.runtimeIdentity, 'utf8')),
        constructP158LiveCampaignBundles: async () => {
          throw Object.assign(new Error('sealed assembly failed'), { code: 'sealed_assembly_failed' });
        },
      };
    },
  });
  await assert.rejects(() => runP158LiveCampaignEntrypoint(failingOptions), (error) => {
    assert.equal(error.terminalReceipt.outcome, 'failed');
    assert.equal(error.terminalReceipt.failure.code, 'sealed_assembly_failed');
    return true;
  });
  await expectCode('prior_terminal_failure', () => runP158LiveCampaignEntrypoint(failingOptions));
  assert.equal(assemblyCalls, 1, 'an append-only failed terminal result must not be replayed');
} finally { await failedTerminal.cleanup(); }

const changedAcrossResume = await createFixture('changed-across-resume');
try {
  const stateRoot = join(changedAcrossResume.runRoot, 'live-campaign-entrypoint');
  await mkdir(stateRoot, { recursive: true });
  await writeFile(join(stateRoot, 'entrypoint-started.json'), canonicalJson(sealed({
    state: 'started', descriptorSha256: changedAcrossResume.descriptorSha256,
    sourceDigest: 'f'.repeat(64), observedAt: NOW, repairAttempted: false,
    retryAttempted: false, garbageCollectionAttempted: false,
  }, 'checkpointSha256')));
  await expectCode('entrypoint_source_drift', () => runP158LiveCampaignEntrypoint(options(changedAcrossResume)));
} finally { await changedAcrossResume.cleanup(); }

const interrupted = await createFixture('started-without-terminal');
try {
  let effectCalls = 0;
  const crashingW7 = {
    ...interrupted.w7Bundle,
    adapterBindings: interrupted.w7Bundle.adapterBindings.map((entry) => structuredClone(entry)),
    w7Adapters: [...interrupted.w7Bundle.w7Adapters],
  };
  crashingW7.adapterBindings[0].mode = 'concrete_live';
  crashingW7.adapterBindings[0].effectsAllowed = true;
  crashingW7.adapterBindings[0].hookIds = ['w7.test'];
  crashingW7.adapterBindings[0].blocker = null;
  crashingW7.w7Adapters[0] = { ...crashingW7.w7Adapters[0], executionMode: 'concrete_live', effectsAllowed: true,
    liveHookIds: ['w7.test'], blocker: null, liveBindingSha256: sha256(crashingW7.adapterBindings[0]),
    execute: async ({ attempt, requestEffect }) => {
      await requestEffect('effect:a', { attemptId: attempt.attemptId });
      return { resultState: 'passed', effectState: 'verified_effect' };
    } };
  crashingW7.effects = { 'effect:a': async () => { effectCalls += 1; throw Object.assign(new Error('lost'), { code: 'simulated_loss' }); } };
  const alteredPreparation = buildP158CampaignPhasePreparation({ schedule: interrupted.schedule,
    w7Bundle: crashingW7, w8Bundle: interrupted.w8Bundle,
    liveHookManifestSha256: interrupted.liveHooks.manifestSha256, runId: interrupted.descriptor.runId });
  await writeFile(interrupted.paths.phasePreparation, canonicalJson(alteredPreparation));
  interrupted.descriptor.phasePreparation.sha256 = sha256(await readFile(interrupted.paths.phasePreparation));
  await writeFile(interrupted.paths.descriptor, canonicalJson(interrupted.descriptor));
  interrupted.descriptorSha256 = sha256(await readFile(interrupted.paths.descriptor));
  const sourceDigest = sha256({
    descriptorSha256: interrupted.descriptorSha256,
    manifestSha256: interrupted.descriptor.manifest.sha256,
    scheduleSha256: interrupted.schedule.scheduleSha256,
    phasePreparationSha256: alteredPreparation.preparationSha256,
    liveHookManifestSha256: interrupted.liveHooks.manifestSha256,
    runtimeIdentitySha256: interrupted.descriptor.runtimeIdentity.sha256,
    bundleAssemblySourceSha256: interrupted.descriptor.bundleAssembly.sourceSha256,
    bundleAssemblyConfigurationSha256: interrupted.descriptor.bundleAssembly.configuration.sha256,
  });
  const phaseSourceDigest = sha256({
    scheduleSha256: interrupted.schedule.scheduleSha256,
    liveHookManifestSha256: interrupted.liveHooks.manifestSha256,
    bindings: [...crashingW7.adapterBindings, ...interrupted.w8Bundle.adapterBindings],
  });
  const stateRoot = join(interrupted.runRoot, 'live-campaign-entrypoint');
  const phaseStartRoot = join(interrupted.runRoot, 'campaign-phases', 'W7', 'attempts-started');
  await mkdir(stateRoot, { recursive: true });
  await mkdir(phaseStartRoot, { recursive: true });
  await writeFile(join(stateRoot, 'entrypoint-started.json'), canonicalJson(sealed({
    state: 'started', descriptorSha256: interrupted.descriptorSha256, sourceDigest, observedAt: NOW,
    repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false,
  }, 'checkpointSha256')));
  const attempt = interrupted.schedule.attempts[0];
  await writeFile(join(phaseStartRoot, `${attempt.attemptId}.json`), canonicalJson(sealed({
    state: 'started', sourceDigest: phaseSourceDigest,
    correlationIds: { requestId: `p158:${interrupted.descriptor.runId}:${attempt.attemptId}:request`,
      eventId: `p158:${interrupted.descriptor.runId}:W7:${attempt.attemptId}:terminal:0`,
      traceId: `p158:${interrupted.descriptor.runId}:W7` }, observedAt: NOW,
  }, 'checkpointSha256')));
  const resumedOptions = options(interrupted, {
    bundleAssemblyLoader: async () => ({
      readP158LiveCampaignRuntimeIdentity: async () => JSON.parse(await readFile(interrupted.paths.runtimeIdentity, 'utf8')),
      constructP158LiveCampaignBundles: async () => ({
        w7Bundle: crashingW7, w8Bundle: interrupted.w8Bundle, w9: { target: { runId: interrupted.descriptor.runId } },
      }),
    }),
    runCampaignPhases: (input) => runP158CampaignPhases({ ...input, runW9: async ({ controller }) => {
      await controller.recordScheduledTeardown({ resultState: 'passed' });
      await controller.finishExecution();
      await controller.sealEvidence();
      return { state: 'evidence_sealed' };
    } }),
  });
  const resumed = await runP158LiveCampaignEntrypoint(resumedOptions);
  assert.equal(resumed.outcome, 'completed');
  assert.equal(effectCalls, 0, 'a started effect must not be replayed after process loss');
  const uncertain = JSON.parse(await readFile(join(
    interrupted.runRoot, 'campaign-phases', 'W7', 'attempts-terminal', `${attempt.attemptId}.json`), 'utf8'));
  assert.equal(uncertain.resultState, 'harness_failure');
  assert.equal(uncertain.effectState, 'effect_uncertain');
} finally { await interrupted.cleanup(); }

const postSeal = await createFixture('post-seal');
try {
  await mkdir(join(postSeal.runRoot, 'manifest'), { recursive: true });
  await writeFile(join(postSeal.runRoot, 'manifest', 'sealed-evidence-manifest.json'), '{}\n');
  await expectCode('post_seal_refused', () => runP158LiveCampaignEntrypoint(options(postSeal)));
} finally { await postSeal.cleanup(); }

const productionDescriptor = {
  schemaVersion: 'agent-browser.p158-live-campaign-entrypoint.v1', planId: 'P158', runId: 'production-attempt',
  runtimeLane: 'development', production: false, runRoot: join(homedir(), '.agent-browser', 'campaign'),
  repairAllowed: false, retryAllowed: false, garbageCollectionAllowed: false,
};
const productionPath = join(tmpdir(), `p158-production-descriptor-${process.pid}.json`);
try {
  await writeFile(productionPath, canonicalJson(productionDescriptor));
  const productionDescriptorSha256 = sha256(await readFile(productionPath));
  await expectCode('production_root_prohibited', () => runP158LiveCampaignEntrypoint({
    descriptorPath: productionPath, descriptorSha256: productionDescriptorSha256, testing: true,
  }));
} finally { await rm(productionPath, { force: true }); }

process.stdout.write('P158 resume-safe live campaign entrypoint tests passed\n');
