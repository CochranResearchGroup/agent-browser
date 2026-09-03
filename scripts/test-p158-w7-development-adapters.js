#!/usr/bin/env node

import assert from 'node:assert/strict';
import fs from 'node:fs';

import {
  assessP158AdapterReadiness,
  compileP158ExecutionSchedule,
  createP158AdapterExecutor,
  createP158CaseAdapter,
} from './lib/p158-execution-schedule.js';
import {
  createP158DevelopmentCommandPrimitives,
  createP158W7DevelopmentAdapterBundle,
  createP158W7LiveDevelopmentAdapterBundle,
  assessP158W7ReviewedLiveDispatcher,
  validateP158W7LiveBindingManifest,
  enumerateP158W7ActionPlans,
  P158_W7_CASE_IDS,
  P158_W7_REQUIRED_SEAMS,
  P158W7AdapterError,
} from './lib/p158-w7-development-adapters.js';

const registry = JSON.parse(fs.readFileSync(new URL(
  '../docs/dev/contracts/p158-historical-failure-registry.v1.json',
  import.meta.url,
), 'utf8'));
const preliminary = compileP158ExecutionSchedule({ registry, seed: 'p158-w7-adapter-seed' });
const preliminaryW7Plans = enumerateP158W7ActionPlans({ schedule: preliminary });
const a07BrowserBindingsByActionId = Object.fromEntries(preliminaryW7Plans
  .filter((action) => action.caseId === 'A07')
  .map((action, index) => [action.actionId, {
    pid: 5000 + index,
    browserId: `p158-a07-browser-${index + 1}`,
    profilePath: `/tmp/p158-development-run-01/profiles/a07-${index + 1}`,
  }]));
const x06DesktopFixturesByActionId = Object.fromEntries(preliminaryW7Plans
  .filter((action) => action.caseId === 'X06')
  .map((action, index) => {
    const windowState = action.dimensionAssignments.find((entry) =>
      entry.dimensionId === 'window_state').value;
    return [action.actionId, {
      browserId: `p158-x06-browser-${index + 1}`,
      profilePath: `/tmp/p158-development-run-01/profiles/x06-${index + 1}`,
      displayName: `:${100 + index}`,
      locatorId: 'p110-control-v1',
      windowState,
    }];
  }));
const target = Object.freeze({
  targetId: 'p158-development-target-01',
  campaignRunId: 'p158-development-run-01',
  runtimeLane: 'development',
  isolationState: 'isolated',
  ownership: 'p158_campaign',
  production: false,
  foreign: false,
  tenantDataPresent: false,
  redactedComparisonOnly: true,
  disposableRoot: '/tmp/p158-development-run-01',
  browserBindingsByActionId: a07BrowserBindingsByActionId,
  daemonUnit: 'agent-browser-development.service',
  supervisorUnit: 'agent-browser-development-supervisor.service',
  evidenceSince: '2026-09-02T00:00:00Z',
  developmentBinary: '/opt/agent-browser-dev',
  agentBrowserId: 'p158-agent-browser-01',
  agentProfilePath: '/tmp/p158-development-run-01/profiles/agent-main',
  runtimeProfile: 'p158-development-run-01',
  sessionName: 'p158-development-run-01',
  localFixtureOrigin: 'http://127.0.0.1:43158',
  desktopFixtureBindingsByActionId: x06DesktopFixturesByActionId,
  allowedExecutables: [
    '/usr/bin/printf',
    '/usr/bin/journalctl',
    '/opt/agent-browser-dev',
    '/usr/bin/systemctl',
    '/usr/bin/ps',
    '/opt/p158/bin/p158-w7-driver',
    '/opt/p158/bin/p158-evidence',
  ],
  allowedSystemdUnits: [
    'agent-browser-development.service',
    'agent-browser-development-supervisor.service',
  ],
  allowedProcessIds: [
    4242,
    ...Object.values(a07BrowserBindingsByActionId).map((binding) => binding.pid),
  ],
  allowedBrowserIds: Object.values(a07BrowserBindingsByActionId)
    .map((binding) => binding.browserId)
    .concat(
      'p158-agent-browser-01',
      ...Object.values(x06DesktopFixturesByActionId).map((binding) => binding.browserId),
    ),
  allowedProfilePaths: Object.values(a07BrowserBindingsByActionId)
    .map((binding) => binding.profilePath)
    .concat(
      '/tmp/p158-development-run-01/profiles/agent-main',
      ...Object.values(x06DesktopFixturesByActionId).map((binding) => binding.profilePath),
    ),
  allowedDisplayNames: Object.values(x06DesktopFixturesByActionId)
    .map((binding) => binding.displayName),
});

function recordingPrimitives(overrides = {}) {
  const calls = [];
  const operation = async (action) => {
    calls.push({ kind: action.hook, action: structuredClone(action) });
    return { resultState: 'passed', artifactIds: [`effect:${action.actionId}`] };
  };
  return {
    calls,
    captureEvidence: async (action) => {
      calls.push({ kind: `capture:${action.stage}`, action: structuredClone(action) });
      return { artifactId: `${action.stage}:${action.actionId}` };
    },
    captureLogs: async (action) => {
      calls.push({ kind: 'capture:logs', action: structuredClone(action) });
      return { artifactId: `logs:${action.actionId}` };
    },
    executeCli: operation,
    executeBrowser: operation,
    executeDisplay: operation,
    executeShutdown: operation,
    executeSystemd: operation,
    executeProcess: operation,
    ...overrides,
  };
}

function planAction({ caseId, action, caseSpec }) {
  const stimulusByCase = {
    A04: 'policy_mutation', A05: 'policy_transition', A06: 'revocation',
    A07: 'browser_crash', A08: 'identity_fixture', A09: 'target_pathology',
    A10: 'inventory_churn', A11: 'scheduler_fault', A12: 'lock_or_timeout',
    A13: 'supervisor_transition', A14: 'full_shutdown', A15: 'history_marker',
    X01: 'xvfb_orphan', X02: 'allocator_race', X03: 'display_evidence_fault',
    X04: 'display_exhaustion', X05: 'x11_authority_fixture',
    X06: 'desktop_locator_state', X07: 'supervisor_transition',
    X08: 'preserve_transition', X09: 'generation_mismatch', X10: 'service_restart',
  };
  return {
    hook: caseSpec.hook,
    stimulusKind: stimulusByCase[caseId] ?? null,
    targetId: target.targetId,
    campaignRunId: target.campaignRunId,
    operationId: `${caseId}:${action.source}`,
    repair: false,
    retry: false,
    garbageCollect: false,
  };
}

const primitives = recordingPrimitives();
const otherAdapters = preliminary.caseContracts
  .filter((contract) => !P158_W7_CASE_IDS.includes(contract.caseId))
  .map((contract) => createP158CaseAdapter({
    caseId: contract.caseId,
    evidenceProfile: contract.evidenceProfile,
    executionContract: contract.executionContract,
    execute: async () => ({ resultState: 'passed' }),
  }));
const bundle = createP158W7DevelopmentAdapterBundle({
  schedule: preliminary,
  target,
  primitives,
  planAction,
  additionalAdapters: otherAdapters,
});
assert.equal(bundle.w7Adapters.length, 25);
assert.deepEqual(bundle.w7Adapters.map((adapter) => adapter.caseId), P158_W7_CASE_IDS);
assert.equal(bundle.adapters.length, 54);
assert.equal(assessP158AdapterReadiness({
  schedule: preliminary,
  adapters: bundle.adapters,
}).ready, true, 'W7 factory output must compose into evidence-collector readiness');

const schedule = compileP158ExecutionSchedule({
  registry,
  seed: 'p158-w7-adapter-seed',
  adapters: bundle.adapters,
});
const liveActionPlans = enumerateP158W7ActionPlans({ schedule });
assert.equal(liveActionPlans.filter((action) =>
  action.caseId === 'A14' && action.requiredStimulus === 'full_shutdown').length, 2);
assert.equal(liveActionPlans.filter((action) =>
  action.caseId === 'A14' && action.requiredStimulus === null).length, 8);
assert.equal(liveActionPlans.filter((action) =>
  action.caseId === 'X08' && action.requiredStimulus === 'full_shutdown').length, 2);
assert.equal(liveActionPlans.filter((action) =>
  action.caseId === 'X10' && action.requiredStimulus === 'host_restart').length, 1);
const reviewedLive = assessP158W7ReviewedLiveDispatcher({ schedule, target });
assert.equal(reviewedLive.ready, false);
assert.deepEqual(reviewedLive.implementedCaseIds, ['A07', 'A13', 'X06']);
assert.deepEqual(reviewedLive.partiallyImplementedCaseIds, ['A09', 'A15']);
assert.equal(reviewedLive.blockerCount, 22);
assert.equal(reviewedLive.effectsExecuted, false);
assert.deepEqual(Object.entries(P158_W7_REQUIRED_SEAMS)
  .filter(([, seam]) => seam.kind === 'product_source')
  .map(([caseId]) => caseId), ['A11', 'A12', 'A14']);
for (const caseId of ['A01', 'A02', 'A03', 'A04', 'A05', 'A06', 'A08', 'A09', 'A10', 'A15']) {
  assert.equal(P158_W7_REQUIRED_SEAMS[caseId].kind, 'campaign_harness');
}
for (const caseId of Object.keys(P158_W7_REQUIRED_SEAMS)) {
  const blocker = reviewedLive.blockers.find((entry) => entry.caseId === caseId);
  assert.deepEqual(blocker.requiredSeam, P158_W7_REQUIRED_SEAMS[caseId]);
}
assert.equal(reviewedLive.implementedActionCount,
  liveActionPlans.filter((action) =>
    ['A07', 'A13', 'X06'].includes(action.caseId) ||
    (action.caseId === 'A09' &&
      action.dimensionAssignments.some((entry) =>
        entry.dimensionId === 'target_pathology' && entry.value === 'blank')) ||
    (action.caseId === 'A15' &&
      action.dimensionAssignments.some((entry) =>
        entry.dimensionId === 'control_transport' && entry.value === 'cli'))).length);
assert(reviewedLive.bindings.every((binding) =>
  binding.targetId === target.targetId && binding.campaignRunId === target.campaignRunId));
assert(reviewedLive.bindings.filter((binding) => binding.caseId === 'A07')
  .every((binding) =>
    binding.process.pid === target.browserBindingsByActionId[binding.actionId].pid &&
    binding.browserId === target.browserBindingsByActionId[binding.actionId].browserId &&
    binding.profilePath === target.browserBindingsByActionId[binding.actionId].profilePath &&
    binding.process.signal === 'SIGKILL'));
assert.equal(new Set(reviewedLive.bindings.filter((binding) => binding.caseId === 'A07')
  .map((binding) => binding.process.pid)).size,
liveActionPlans.filter((action) => action.caseId === 'A07').length);
assert(reviewedLive.bindings.filter((binding) => binding.caseId === 'A13')
  .some((binding) => binding.systemd.unit === target.daemonUnit));
assert(reviewedLive.bindings.filter((binding) => binding.caseId === 'A13')
  .some((binding) => binding.systemd.unit === target.supervisorUnit));
assert.equal(reviewedLive.bindings.filter((binding) => binding.caseId === 'X06').length, 21);
assert(reviewedLive.bindings.filter((binding) => binding.caseId === 'X06')
  .every((binding) =>
    binding.command.args.includes('desktop') &&
    binding.command.args.includes('locate') &&
    binding.displayName === target.desktopFixtureBindingsByActionId[binding.actionId].displayName));
assert.equal(reviewedLive.bindings.filter((binding) => binding.caseId === 'A09').length, 2);
assert(reviewedLive.bindings.filter((binding) => binding.caseId === 'A09')
  .every((binding) => binding.command.args.at(-1) === 'about:blank'));
assert.equal(reviewedLive.bindings.filter((binding) => binding.caseId === 'A15').length, 1);
assert(reviewedLive.bindings.find((binding) => binding.caseId === 'A15')
  .command.args.at(-1).includes(encodeURIComponent(
    reviewedLive.bindings.find((binding) => binding.caseId === 'A15').actionId,
  )));
const liveBindingManifest = {
  schemaVersion: 'agent-browser.p158-w7-live-bindings.v1',
  actions: liveActionPlans.map((action) => ({
    actionId: action.actionId,
    caseId: action.caseId,
    attemptId: action.attemptId,
    targetId: target.targetId,
    campaignRunId: target.campaignRunId,
    hook: action.hook,
    stimulusKind: action.requiredStimulus,
    evidenceCommand: {
      executable: '/opt/p158/bin/p158-evidence',
      args: ['pre-post', action.actionId],
    },
    logCommand: {
      executable: '/opt/p158/bin/p158-evidence',
      args: ['logs', action.actionId],
    },
    ...(['cli', 'browser', 'display', 'shutdown'].includes(action.hook) ? {
      command: {
        executable: '/opt/p158/bin/p158-w7-driver',
        args: [action.caseId, action.attemptId, action.actionId],
      },
    } : {}),
    ...(action.hook === 'systemd' ? {
      systemd: { unit: 'agent-browser-development.service', verb: 'restart' },
    } : {}),
    ...(action.hook === 'process' ? {
      process: { pid: 4242, signal: 'SIGTERM' },
    } : {}),
  })),
};
const liveValidation = validateP158W7LiveBindingManifest({
  schedule,
  target,
  manifest: liveBindingManifest,
});
assert.equal(liveValidation.manifest.actionCount, liveActionPlans.length);
assert.equal(liveValidation.manifest.targetSha256.length, 64);
assert.equal(liveValidation.manifest.manifestSha256.length, 64);
assert.equal(liveValidation.liveReady, false);
assert.equal(liveValidation.blockerCode, 'live_w7_dispatcher_implementation_unproven');
const reviewedPrimitives = recordingPrimitives();
const reviewedBundle = createP158W7LiveDevelopmentAdapterBundle({
  schedule,
  target,
  primitives: reviewedPrimitives,
  additionalAdapters: otherAdapters,
});
assert.equal(reviewedBundle.ready, true);
assert.equal(reviewedPrimitives.calls.length, 0);
assert.equal(reviewedBundle.adapterBindings.length, 25);
assert.equal(reviewedBundle.adapterBindings.filter((binding) =>
  binding.mode === 'concrete_live').length, 3);
assert.equal(reviewedBundle.adapterBindings.filter((binding) =>
  binding.mode === 'explicit_blocked').length, 22);
assert(reviewedBundle.adapterBindings.every((binding) =>
  binding.providerFree === false && /^[a-f0-9]{64}$/.test(binding.sourceSha256)));
assert.equal(assessP158AdapterReadiness({
  schedule,
  adapters: reviewedBundle.adapters,
}).ready, true);
const blockedA01 = reviewedBundle.w7Adapters.find((adapter) => adapter.caseId === 'A01');
let blockedEffectCalls = 0;
const blockedOutcome = await blockedA01.execute({
  attempt: schedule.attempts.find((attempt) => attempt.caseId === 'A01'),
  requestEffect: async () => {
    blockedEffectCalls += 1;
    throw new Error('blocked adapter must not request an effect');
  },
});
assert.equal(blockedOutcome.resultState, 'skipped_blocked');
assert.equal(blockedOutcome.effectState, 'not_started');
assert.equal(blockedOutcome.blocker.code, 'live_case_hook_missing');
assert.equal(blockedEffectCalls, 0);
assert.throws(
  () => validateP158W7LiveBindingManifest({
    schedule,
    target,
    manifest: {
      ...liveBindingManifest,
      actions: liveBindingManifest.actions.slice(1),
    },
  }),
  (error) => error instanceof P158W7AdapterError &&
    error.code === 'live_binding_action_missing',
);
assert.throws(
  () => validateP158W7LiveBindingManifest({
    schedule,
    target,
    manifest: {
      ...liveBindingManifest,
      actions: liveBindingManifest.actions.map((action, index) => index === 0
        ? { ...action, command: undefined }
        : action),
    },
  }),
  (error) => error instanceof P158W7AdapterError &&
    error.code === 'live_command_binding_invalid',
);
const attemptByCase = new Map(P158_W7_CASE_IDS.map((caseId) => [
  caseId,
  schedule.attempts.find((attempt) => attempt.caseId === caseId),
]));
const allActionIds = [];
const outcomesByAttempt = new Map();
const adapterByCase = new Map(bundle.w7Adapters.map((adapter) => [adapter.caseId, adapter]));
for (const attempt of schedule.attempts.filter((entry) => P158_W7_CASE_IDS.includes(entry.caseId))) {
  const adapter = adapterByCase.get(attempt.caseId);
  const outcome = await adapter.execute({
    attempt: structuredClone(attempt),
    requestEffect: (effectId, payload) => bundle.effects[effectId](payload, attempt),
  });
  outcomesByAttempt.set(attempt.attemptId, outcome);
  assert.equal(outcome.resultState, 'passed', adapter.caseId);
  assert.equal(outcome.repairAttempted, false);
  assert.equal(outcome.retryAttempted, false);
  assert.equal(outcome.garbageCollectionAttempted, false);
  assert.equal(outcome.actionCount, outcome.actionIds.length);
  assert.equal(new Set(outcome.actionIds).size, outcome.actionIds.length);
  for (const receipt of outcome.receipts) {
    assert.equal(receipt.evidence.length, 3);
    assert.equal(receipt.artifactIds.length, 3);
    assert.equal(receipt.repairAttempted, false);
    assert.equal(receipt.retryAttempted, false);
    assert.equal(receipt.garbageCollectionAttempted, false);
  }
  allActionIds.push(...outcome.actionIds);
}
assert.equal(new Set(allActionIds).size, allActionIds.length);
assert.equal(bundle.executedActionIds.size, allActionIds.length);

const a01Outcome = outcomesByAttempt.get('A01-E1-r001');
assert.equal(a01Outcome.actionCount, 130);
const a01Ids = new Set(a01Outcome.actionIds);
assert(a01Outcome.receipts.every((receipt) => a01Ids.has(receipt.actionId)));

const exactExecutor = createP158AdapterExecutor({
  schedule,
  adapters: bundle.adapters,
  effects: bundle.effects,
});
const freshA01 = schedule.attempts.find((attempt) =>
  attempt.caseId === 'A01' && attempt.environmentId === 'E0');
await assert.rejects(
  () => exactExecutor.executeAttempt(freshA01.attemptId),
  (error) => error instanceof P158W7AdapterError && error.code === 'action_already_executed',
);
await assert.rejects(
  () => exactExecutor.executeAttempt(freshA01.attemptId),
  (error) => error.code === 'opportunistic_retry_prohibited',
);

for (const defectiveTarget of [
  { ...target, runtimeLane: 'production', production: true },
  { ...target, foreign: true },
  { ...target, tenantDataPresent: true },
  { ...target, ownership: 'unknown' },
]) {
  assert.throws(
    () => createP158W7DevelopmentAdapterBundle({
      schedule,
      target: defectiveTarget,
      primitives,
      planAction,
    }),
    (error) => error instanceof P158W7AdapterError &&
      error.code === 'development_target_unproven',
  );
}

const badPlanBundle = createP158W7DevelopmentAdapterBundle({
  schedule,
  target,
  primitives: recordingPrimitives(),
  planAction: ({ caseSpec }) => ({
    hook: caseSpec.hook,
    stimulusKind: 'reactionary_repair',
    targetId: target.targetId,
    campaignRunId: target.campaignRunId,
  }),
});
const badA01 = badPlanBundle.w7Adapters.find((adapter) => adapter.caseId === 'A01');
await assert.rejects(
  () => badA01.execute({
    attempt: freshA01,
    requestEffect: (effectId, payload) => badPlanBundle.effects[effectId](payload, freshA01),
  }),
  (error) => error instanceof P158W7AdapterError &&
    error.code === 'undeclared_stimulus_prohibited',
);

const commands = [];
const kills = [];
const realHooks = createP158DevelopmentCommandPrimitives({
  target,
  execFile: async (executable, args, options) => {
    commands.push({ executable, args, options });
    return { stdout: 'synthetic stdout', stderr: 'synthetic stderr' };
  },
  kill: (pid, signal) => kills.push({ pid, signal }),
  clock: () => '2026-09-02T23:30:00.000Z',
});
const evidenceReceipt = await realHooks.captureEvidence({
  actionId: 'hook-evidence',
  evidenceCommand: { executable: '/usr/bin/printf', args: ['pre'] },
});
await realHooks.captureLogs({
  actionId: 'hook-logs',
  logCommand: { executable: '/usr/bin/journalctl', args: ['--user', '-n', '1'] },
});
await realHooks.executeCli({
  actionId: 'hook-cli',
  command: { executable: '/opt/agent-browser-dev', args: ['status'] },
});
await realHooks.executeSystemd({
  actionId: 'hook-systemd',
  systemd: { unit: 'agent-browser-development.service', verb: 'restart' },
});
await realHooks.executeProcess({
  actionId: 'hook-process',
  process: { pid: 4242, signal: 'SIGTERM' },
});
assert.deepEqual(commands.map((entry) => [entry.executable, entry.args]), [
  ['/usr/bin/printf', ['pre']],
  ['/usr/bin/journalctl', ['--user', '-n', '1']],
  ['/opt/agent-browser-dev', ['status']],
  ['/usr/bin/systemctl', ['--user', 'restart', 'agent-browser-development.service']],
]);
assert.deepEqual(kills, [{ pid: 4242, signal: 'SIGTERM' }]);
assert.equal('stdout' in evidenceReceipt, false, 'real hooks must return digests, not raw output');
assert.match(evidenceReceipt.stdoutSha256, /^[a-f0-9]{64}$/);
await assert.rejects(
  () => realHooks.executeCli({
    actionId: 'foreign-executable',
    command: { executable: '/usr/bin/bash', args: ['-lc', 'true'] },
  }),
  (error) => error instanceof P158W7AdapterError && error.code === 'executable_not_owned',
);
assert.throws(
  () => createP158DevelopmentCommandPrimitives(),
  (error) => error instanceof P158W7AdapterError &&
    error.code === 'development_target_unproven',
);

console.log(JSON.stringify({
  ok: true,
  adapterCount: bundle.w7Adapters.length,
  exercisedCaseCount: attemptByCase.size,
  executedActionCount: bundle.executedActionIds.size,
  primitiveCallCount: primitives.calls.length,
}, null, 2));
