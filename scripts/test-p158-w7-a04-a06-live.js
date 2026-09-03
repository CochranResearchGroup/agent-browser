#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtemp, writeFile } from 'node:fs/promises';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import {
  createP158W7LiveDevelopmentAdapterBundle,
  enumerateP158W7ActionPlans,
} from './lib/p158-w7-development-adapters.js';
import {
  assessP158W7A04A06ActionReadiness,
  createP158W7A04A06LiveBundle,
  createP158W7A04A06OwnershipManifest,
  createP158W7A05DevelopmentService,
  enumerateP158W7A05LoggingRequests,
  p158W7A04A06SourceBinding,
  P158W7A04A06Error,
} from './lib/p158-w7-a04-a06-live.js';

const registry = JSON.parse(fs.readFileSync(new URL(
  '../docs/dev/contracts/p158-historical-failure-registry.v1.json', import.meta.url,
), 'utf8'));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w7-a04-a06-live' });

function json(response, status, value) {
  const body = JSON.stringify(value);
  response.writeHead(status, { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) });
  response.end(body);
}

async function body(request) {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}

function targetPolicy(command) {
  return command.params?.targetPolicy;
}

function startService({ wrongOutcomeAction = null, retainReleasedTabAction = null,
  sameConnectionAction = null, bothConflictSucceedAction = null,
  wrongDenialReasonAction = null } = {}) {
  const profiles = new Map();
  const tabs = new Map();
  const jobs = [];
  const socketIds = new WeakMap();
  let socketOrdinal = 0;
  let tabOrdinal = 0;
  let requests = 0;
  const fixtureSetupRequestIds = [];
  const server = createServer(async (request, response) => {
    requests += 1;
    if (!socketIds.has(request.socket)) socketIds.set(request.socket, `connection-${++socketOrdinal}`);
    const url = new URL(request.url, 'http://127.0.0.1');
    if (request.method === 'GET' && url.pathname === '/api/service/status') {
      return json(response, 200, { success: true, data: {
        runtimeLifecycle: { environment: 'development', runId: 'p158-a04-a06-run' },
        service_state: { candidateSha256: 'a'.repeat(64) },
      } });
    }
    if (request.method === 'GET' && url.pathname === '/api/service/tabs') {
      return json(response, 200, { success: true, data: { tabs: [...tabs.values()] } });
    }
    if (request.method === 'GET' && url.pathname === '/api/service/trace') {
      return json(response, 200, { success: true, data: { jobs } });
    }
    if (request.method === 'GET' && url.pathname === '/api/service/access-plan') {
      const profile = profiles.get(url.searchParams.get('runtimeProfile'));
      const wrongReason = wrongDenialReasonAction && url.searchParams.get('taskName')?.includes(wrongDenialReasonAction);
      return json(response, 200, { success: true, data: { decision: {
        profileAccess: {
          policy: profile?.accessPolicy,
          decision: { allowed: false, operation: 'tab_create',
            nextAction: { action: wrongReason ? 'inspect_profile_access_policy' : 'inspect_profile_occupancy',
              executable: true, request: null } },
        },
        serviceRequest: { acquisitionBlocker: 'profile_access_denied', available: false },
      } } });
    }
    if (request.method === 'POST' && url.pathname.startsWith('/api/service/profiles/')) {
      fixtureSetupRequestIds.push(request.headers['x-agent-browser-request-id'] ?? null);
      const profile = await body(request);
      const id = decodeURIComponent(url.pathname.split('/').at(-1));
      profiles.set(id, structuredClone(profile));
      return json(response, 200, { success: true, data: { id, profile, upserted: true } });
    }
    if (request.method !== 'POST' || url.pathname !== '/api/service/request') {
      return json(response, 404, { success: false, error: { code: 'not_found' } });
    }
    const command = await body(request);
    const requestId = command.requestId;
    const forcedConnection = sameConnectionAction && requestId.includes(sameConnectionAction)
      ? 'connection-forced-same' : socketIds.get(request.socket);
    jobs.push({ id: `job-${jobs.length + 1}`, state: 'completed', provenance: {
      requestId, traceId: command.traceId, action: command.action,
      clientSubjectId: command.clientSubjectId, connectionInstanceId: forcedConnection,
      runtimeEnvironmentId: command.runtimeEnvironmentId,
    } });
    const profile = profiles.get(command.profileId);
    if (!profile) return json(response, 200, { id: requestId, success: false,
      error: { code: 'profile_missing' } });
    if (command.action === 'tab_new') {
      const policy = profile.accessPolicy;
      if (policy.state === 'draining') return json(response, 200, { id: requestId, success: false,
        error: { code: 'profile_access_denied' } });
      const tabId = `tab-${++tabOrdinal}`;
      const handle = {
        browserId: `browser-${command.profileId}`, sessionName: command.sessionName,
        tabId, targetId: `target-${tabOrdinal}`, url: command.params?.url ?? null,
        title: command.taskName, profileId: command.profileId, profileOrigin: 'agent_browser_owned',
        leaseId: `lease-${tabOrdinal}`, leaseState: 'shared', cleanupPolicy: 'close_tabs',
        leaseHeartbeatExpected: false, ownerSessionId: command.sessionName,
        profileAccess: { schemaVersion: 'agent-browser.profile-child-access.v1',
          parentPolicyRevision: policy.revision, accessDecisionId: `decision-${tabOrdinal}`,
          subjectId: command.clientSubjectId, identityAssurance: 'registered-capability',
          connectionInstanceId: forcedConnection, connectionState: 'active',
          permissions: [...policy.defaultPermissions] },
        jobId: jobs.at(-1).id, traceFilter: {}, valid: true, staleReason: null,
      };
      tabs.set(tabId, { id: tabId, browserId: handle.browserId, targetId: handle.targetId,
        sessionId: command.sessionName, profileId: command.profileId, lifecycle: 'ready',
        serviceTabHandle: handle });
      return json(response, 200, { id: requestId, success: true, data: { serviceTabHandle: handle } });
    }
    if (command.action === 'tab_handle_release') {
      const tab = tabs.get(command.serviceTabHandle?.tabId);
      if (!tab) return json(response, 200, { id: requestId, success: false,
        error: { code: 'profile_access_denied' } });
      if (!retainReleasedTabAction || !requestId.includes(retainReleasedTabAction)) tab.lifecycle = 'closed';
      return json(response, 200, { id: requestId, success: true,
        data: { serviceTabHandle: { ...tab.serviceTabHandle, valid: false, leaseState: 'released' } } });
    }
    if (command.action === 'service_profile_policy_mutate') {
      const current = profile.accessPolicy;
      const conflictCase = bothConflictSucceedAction && requestId.includes(bothConflictSucceedAction);
      if (command.params.expectedRevision !== current.revision && !conflictCase) {
        return json(response, 200, { id: requestId, success: false,
          error: { code: 'policy_revision_conflict', currentRevision: current.revision } });
      }
      const target = targetPolicy(command);
      const occupants = [...tabs.values()].filter((tab) =>
        tab.profileId === command.profileId && tab.lifecycle !== 'closed');
      let outcome;
      if (current.mode === 'restricted' && target.mode === 'shared-local') {
        outcome = 'widened';
      } else if (occupants.length > 0) {
        outcome = 'drain_started';
      } else {
        outcome = 'restricted';
      }
      if (wrongOutcomeAction && requestId.includes(wrongOutcomeAction)) outcome = 'unchanged';
      if (outcome === 'drain_started') {
        current.state = 'draining';
        current.drain = { targetMode: target.mode, expectedRevision: current.revision,
          incompatibleOccupancy: occupants.map((tab) => tab.id), forceAuthorized: false };
      } else {
        profile.accessPolicy = { ...current, ...target, revision: current.revision + 1,
          state: 'active', drain: null, updatedAt: '2026-09-03T00:00:00Z' };
      }
      return json(response, 200, { id: requestId, success: true, data: {
        profileId: command.profileId, outcome, policy: profile.accessPolicy,
        blockingOccupancy: occupants.map((tab) => tab.id), evictionPlan: null,
        evictionReceipt: null, receipt: { receiptId: `receipt-${requestId}` },
      } });
    }
    return json(response, 200, { id: requestId, success: false,
      error: { code: 'unexpected_action' } });
  });
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve({
    origin: `http://127.0.0.1:${server.address().port}`,
    requests: () => requests,
    serviceRequestIds: () => jobs.map((entry) => entry.provenance.requestId),
    fixtureSetupRequestIds: () => [...fixtureSetupRequestIds],
    close: () => new Promise((done, reject) => server.close((error) => error ? reject(error) : done())),
  })));
}

function fixture(attempt, capabilities) {
  const adminSubjectId = `principal:${attempt.attemptId.toLowerCase()}:admin`;
  const participantSubjectId = `principal:${attempt.attemptId.toLowerCase()}:participant`;
  return {
    profileId: `p158-${attempt.attemptId.toLowerCase()}-profile`,
    sessionName: `p158-${attempt.attemptId.toLowerCase()}-session`,
    url: `http://127.0.0.1:43158/${attempt.attemptId.toLowerCase()}`,
    adminSubjectId,
    participantSubjectId,
    adminCapability: { ...capabilities.admin, principalId: adminSubjectId },
    participantCapability: { ...capabilities.participant, principalId: participantSubjectId },
  };
}

function manifest(origin, capabilities) {
  const attempts = schedule.attempts.filter((attempt) => attempt.caseId === 'A05');
  return createP158W7A04A06OwnershipManifest({
    schemaVersion: 'agent-browser.p158-w7-a04-a06-ownership.v1',
    campaignRunId: 'p158-a04-a06-run', candidateSha256: 'a'.repeat(64),
    liveHookManifestSha256: 'b'.repeat(64),
    environmentSealSha256s: { E0: 'c'.repeat(64), E1: 'd'.repeat(64) },
    environments: Object.fromEntries(['E0', 'E1'].map((environmentId) => [environmentId, {
      serviceOrigin: origin, runtimeLane: 'development', production: false,
      runtimeEnvironmentId: environmentId,
      ownershipStatus: { runtimeLifecycle: { environment: 'development', runId: 'p158-a04-a06-run' },
        service_state: { candidateSha256: 'a'.repeat(64) } },
    }])),
    fixtures: { A05: Object.fromEntries(attempts.map((attempt) => [attempt.attemptId,
      fixture(attempt, capabilities)])) },
  });
}

const readiness = assessP158W7A04A06ActionReadiness({ schedule });
assert.deepEqual(assessP158W7A04A06ActionReadiness({ schedule }), readiness);
assert.deepEqual(readiness.counts, {
  A04: { scheduled: 216, executable: 0, blocked: 216 },
  A05: { scheduled: 12, executable: 12, blocked: 0 },
  A06: { scheduled: 8, executable: 0, blocked: 8 },
});
assert.equal(readiness.actions.filter((action) => action.caseId === 'A06' &&
  action.blocker.code === 'queued_command_barrier_seam_missing').length, 4);
assert.equal(readiness.effectsAttempted, false);

const enumeratedLoggingRequests = enumerateP158W7A05LoggingRequests({
  schedule, campaignRunId: 'p158-a04-a06-run',
});
assert.equal(enumeratedLoggingRequests.length, 40);
assert.equal(new Set(enumeratedLoggingRequests.map((entry) => entry.expectationId)).size, 40);
assert.equal(new Set(enumeratedLoggingRequests.map((entry) => entry.requestId)).size, 40);
assert.deepEqual(enumerateP158W7A05LoggingRequests({
  schedule, campaignRunId: 'p158-a04-a06-run',
}), enumeratedLoggingRequests);
assert.ok(enumeratedLoggingRequests.every((entry) => entry.expectationId === entry.requestId &&
  entry.caseId === 'A05' && entry.phaseId === 'W7' && ['E0', 'E1'].includes(entry.environmentId) &&
  ['accepted_request', 'rejected_request'].includes(entry.requestKind) &&
  entry.requestId.includes(entry.actionId)));
assert.equal(enumeratedLoggingRequests.filter((entry) => entry.requestKind === 'rejected_request').length, 2);
assert.ok(enumeratedLoggingRequests.filter((entry) => entry.operationKind === 'fixture-setup')
  .every((entry) => entry.expectedSurfaceRoles === undefined),
'fixture setup must retain the full accepted-request causal surface default');
assert.deepEqual([...new Set(enumeratedLoggingRequests.map((entry) => entry.operationKind))].sort(), [
  'admission-probe', 'conflict-a', 'conflict-b', 'drain-complete', 'fixture-setup',
  'occupant-open', 'own-release', 'policy-mutate',
]);

const temporary = await mkdtemp(path.join(os.tmpdir(), 'p158-a04-a06-'));
const adminCapabilityPath = path.join(temporary, 'admin-profile-capability');
const participantCapabilityPath = path.join(temporary, 'participant-profile-capability');
const rawCapability = 'p158-test-admin-capability-with-more-than-thirty-two-characters';
const rawParticipantCapability = 'p158-test-participant-capability-with-more-than-thirty-two-characters';
await writeFile(adminCapabilityPath, `${rawCapability}\n`, { mode: 0o600 });
await writeFile(participantCapabilityPath, `${rawParticipantCapability}\n`, { mode: 0o600 });
const capabilities = {
  admin: { absolutePath: adminCapabilityPath, sha256: sha256(rawCapability) },
  participant: { absolutePath: participantCapabilityPath, sha256: sha256(rawParticipantCapability) },
};

async function runCampaign(serverOptions = {}, select = () => true) {
  const server = await startService(serverOptions);
  const receipts = [];
  const service = createP158W7A05DevelopmentService();
  const frozenManifest = manifest(server.origin, capabilities);
  const manifestBefore = JSON.stringify(frozenManifest);
  const bundle = createP158W7A04A06LiveBundle({
    schedule, ownershipManifest: frozenManifest, service,
    receiptStore: { append: async (receipt) => receipts.push(structuredClone(receipt)) },
    clock: () => '2026-09-03T01:00:00.000Z',
  });
  try {
    const adapter = bundle.adapters[0];
    const results = [];
    for (const attempt of schedule.attempts.filter((entry) => entry.caseId === 'A05' && select(entry))) {
      results.push(await adapter.execute({ attempt }));
    }
    for (const receipt of receipts) {
      assert.deepEqual(receipt.requestIds, bundle.loggingRequestExpectations
        .filter((entry) => entry.attemptId === receipt.attemptId)
        .map((entry) => entry.requestId));
    }
    assert.equal(JSON.stringify(frozenManifest), manifestBefore, 'live driver mutated its frozen ownership input');
    return { server, service, bundle, receipts, results };
  } catch (error) {
    await server.close();
    service.close?.();
    throw error;
  }
}

const clean = await runCampaign();
assert.equal(clean.bundle.freezeEligible, true);
assert.equal(clean.results.length, 12);
assert.ok(clean.results.every((result) => result.resultState === 'passed' && result.actionCount === 1));
assert.equal(clean.receipts.length, 12);
assert.deepEqual(clean.bundle.loggingRequestExpectations, enumeratedLoggingRequests);
assert.deepEqual(clean.server.fixtureSetupRequestIds(), enumeratedLoggingRequests
  .filter((entry) => entry.operationKind === 'fixture-setup').map((entry) => entry.requestId));
assert.deepEqual(clean.server.serviceRequestIds().sort(), enumeratedLoggingRequests
  .filter((entry) => entry.operationKind !== 'fixture-setup').map((entry) => entry.requestId).sort());
assert.equal(new Set(clean.receipts.map((receipt) => receipt.actionId)).size, 12);
assert.ok(clean.receipts.every((receipt) => receipt.attempt === 1 &&
  receipt.retryAttempted === false && receipt.repairAttempted === false &&
  receipt.garbageCollectionAttempted === false && receipt.receiptSha256 === sha256(
    Object.fromEntries(Object.entries(receipt).filter(([key]) => key !== 'receiptSha256')),
  )));
clean.service.close?.();
await clean.server.close();

for (const [serverOptions, transition, expectedState, failureCode] of [
  [{ wrongOutcomeAction: 'A05-E0-r001' }, 'widen', 'reproduced_historical_failure', 'policy_transition_oracle_failed'],
  [{ retainReleasedTabAction: 'A05-E0-r004' }, 'own_tab_release', 'reproduced_historical_failure', 'own_tab_release_oracle_failed'],
  [{ sameConnectionAction: 'A05-E0-r005' }, 'revision_conflict', 'reproduced_historical_failure', 'distinct_connection_evidence_failed'],
  [{ bothConflictSucceedAction: 'A05-E0-r005' }, 'revision_conflict', 'reproduced_historical_failure', 'revision_conflict_oracle_failed'],
  [{ wrongDenialReasonAction: 'A05-E0-r003' }, 'admission', 'reproduced_historical_failure', 'draining_admission_reason_oracle_failed'],
]) {
  const run = await runCampaign(serverOptions, (attempt) =>
    attempt.executionUnit.dimensionAssignment?.value === transition && attempt.environmentId === 'E0');
  assert.equal(run.results.length, 1);
  assert.equal(run.results[0].resultState, expectedState);
  assert.equal(run.receipts[0].failure.code, failureCode);
  run.service.close?.();
  await run.server.close();
}

const source = p158W7A04A06SourceBinding();
assert.equal(source.sourceSha256, sha256(fs.readFileSync(source.sourcePath)));

const injected = createP158W7A05DevelopmentService({
  transportFor: () => async () => { throw new Error('not called'); },
  capabilityFor: () => rawCapability,
});
const unproven = createP158W7A04A06LiveBundle({ schedule,
  ownershipManifest: manifest('http://127.0.0.1:43158', capabilities),
  service: injected, receiptStore: { append: async () => {} } });
assert.equal(unproven.freezeEligible, false);

assert.throws(() => createP158W7A04A06LiveBundle({ schedule,
  ownershipManifest: { ...manifest('http://127.0.0.1:43158', capabilities), manifestSha256: '0'.repeat(64) },
  service: injected, receiptStore: { append: async () => {} } }),
(error) => error instanceof P158W7A04A06Error && error.code === 'frozen_ownership_manifest_invalid');

const actionPlans = enumerateP158W7ActionPlans({ schedule });
const browserBindingsByActionId = Object.fromEntries(actionPlans.filter((action) => action.caseId === 'A07')
  .map((action, index) => [action.actionId, { pid: 7000 + index, browserId: `a07-browser-${index}`,
    profilePath: `/tmp/p158-a04-a06-run/a07-${index}` }]));
const desktopFixtureBindingsByActionId = Object.fromEntries(actionPlans.filter((action) => action.caseId === 'X06')
  .map((action, index) => [action.actionId, { browserId: `x06-browser-${index}`,
    profilePath: `/tmp/p158-a04-a06-run/x06-${index}`, displayName: `:${300 + index}`,
    locatorId: 'p110-control-v1',
    windowState: action.dimensionAssignments.find((entry) => entry.dimensionId === 'window_state').value }]));
const full = createP158W7LiveDevelopmentAdapterBundle({
  schedule,
  target: { runtimeLane: 'development', isolationState: 'isolated', ownership: 'p158_campaign',
    production: false, foreign: false, tenantDataPresent: false, targetId: 'p158-target',
    campaignRunId: 'p158-a04-a06-run', candidateSha256: 'a'.repeat(64),
    daemonUnit: 'agent-browser-development.service', supervisorUnit: 'agent-browser-development-supervisor.service',
    evidenceSince: '2026-09-03T00:00:00Z', redactedComparisonOnly: true,
    disposableRoot: '/tmp/p158-a04-a06-run', browserBindingsByActionId,
    desktopFixtureBindingsByActionId, developmentBinary: '/opt/agent-browser-dev',
    agentBrowserId: 'agent-browser-main', agentProfilePath: '/tmp/p158-a04-a06-run/main',
    runtimeProfile: 'p158-a04-a06', sessionName: 'p158-a04-a06',
    localFixtureOrigin: 'http://127.0.0.1:43158',
    allowedExecutables: ['/opt/agent-browser-dev', '/usr/bin/journalctl', '/usr/bin/systemctl', '/usr/bin/ps'],
    allowedSystemdUnits: ['agent-browser-development.service', 'agent-browser-development-supervisor.service'],
    allowedProcessIds: Object.values(browserBindingsByActionId).map((entry) => entry.pid),
    allowedBrowserIds: ['agent-browser-main', ...Object.values(browserBindingsByActionId).map((entry) => entry.browserId),
      ...Object.values(desktopFixtureBindingsByActionId).map((entry) => entry.browserId)],
    allowedProfilePaths: ['/tmp/p158-a04-a06-run/main',
      ...Object.values(browserBindingsByActionId).map((entry) => entry.profilePath),
      ...Object.values(desktopFixtureBindingsByActionId).map((entry) => entry.profilePath)],
    allowedDisplayNames: Object.values(desktopFixtureBindingsByActionId).map((entry) => entry.displayName) },
  primitives: { captureEvidence: async () => ({}), captureLogs: async () => ({}),
    executeCli: async () => ({}), executeBrowser: async () => ({}), executeDisplay: async () => ({}),
    executeShutdown: async () => ({}), executeSystemd: async () => ({}), executeProcess: async () => ({}) },
  a04A06LiveBundle: clean.bundle,
  liveHookManifestSha256: 'b'.repeat(64),
});
assert.equal(full.adapterBindings.find((binding) => binding.caseId === 'A05').mode, 'concrete_live');
assert.equal(full.adapterBindings.find((binding) => binding.caseId === 'A04').mode, 'explicit_blocked');
assert.equal(full.adapterBindings.find((binding) => binding.caseId === 'A06').mode, 'explicit_blocked');

process.stdout.write('P158 W7 A04-A06 live boundary passed: A05=12 concrete, A04=216 blocked, A06=8 blocked\n');
