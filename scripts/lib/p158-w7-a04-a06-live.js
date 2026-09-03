import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import {
  createServiceProfilePolicyMutationRequest,
  postServiceRequest,
  releaseServiceTabHandle,
  requestServiceTab,
} from '../../packages/client/src/service-request.js';
import {
  getServiceAccessPlan,
  getServiceStatus,
  getServiceTabs,
  getServiceTrace,
  upsertServiceProfile,
} from '../../packages/client/src/service-observability.js';
import { sha256 } from './p158-campaign-controller.js';
import { createP158CaseAdapter } from './p158-execution-schedule.js';
import { enumerateP158W7ActionPlans } from './p158-w7-development-adapters.js';
import { createP158W7PinnedDevelopmentTransports } from './p158-w7-a01-a03-live.js';

export const P158_W7_A04_A06_SOURCE_PATH = 'scripts/lib/p158-w7-a04-a06-live.js';
export const P158_W7_A04_A06_HOOK_ID = 'w7.a04_a06.profile_policy';
export const P158_W7_A04_A06_REVIEWED_CASE_IDS = Object.freeze(['A04', 'A05', 'A06']);
export const P158_W7_A04_A06_CONCRETE_CASE_IDS = Object.freeze(['A05']);

const BUILTIN_SERVICE = Symbol('p158-w7-a04-a06-builtin-service');
const BUILTIN_CAPABILITY = Symbol('p158-w7-a04-a06-file-capability');
const PERMISSIONS = Object.freeze([
  'profile_use', 'policy_read', 'policy_write', 'tab_create', 'tab_observe',
  'tab_control_own', 'tab_close_own', 'tab_control_any', 'tab_close_any',
  'view_open', 'view_control', 'drain', 'evict', 'lifecycle_manage', 'full_shutdown',
]);
const PARTICIPANT_PERMISSIONS = Object.freeze([
  'profile_use', 'policy_read', 'policy_write', 'tab_create', 'tab_observe',
  'tab_control_own', 'tab_close_own', 'view_open', 'view_control', 'drain',
]);

export class P158W7A04A06Error extends Error {
  constructor(code, message, details) {
    super(message);
    this.name = 'P158W7A04A06Error';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W7A04A06Error(code, message, details);
}

function freeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value)) freeze(child);
  }
  return value;
}

function sourceSha256() {
  return createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
}

function value(action, dimensionId) {
  return action.dimensionAssignments?.find((entry) => entry.dimensionId === dimensionId)?.value ?? null;
}

function actionInventory(schedule) {
  return enumerateP158W7ActionPlans({ schedule })
    .filter((action) => P158_W7_A04_A06_REVIEWED_CASE_IDS.includes(action.caseId));
}

function a05RequestSuffixes(action) {
  const transition = value(action, 'transition');
  if (transition === 'revision_conflict') return ['fixture-setup', 'conflict-a', 'conflict-b'];
  const suffixes = ['fixture-setup'];
  if (['admission', 'own_tab_release', 'drain_completion'].includes(transition)) {
    suffixes.push('occupant-open');
  }
  suffixes.push('policy-mutate');
  if (transition === 'admission') suffixes.push('admission-probe');
  if (['own_tab_release', 'drain_completion'].includes(transition)) suffixes.push('own-release');
  if (transition === 'drain_completion') suffixes.push('drain-complete');
  return suffixes;
}

function a05LoggingRequestDescriptors(action, campaignRunId, environmentId) {
  return a05RequestSuffixes(action).map((suffix) => {
    const requestId = `${campaignRunId}:${action.actionId}:${suffix}`;
    return {
      expectationId: requestId,
      requestId,
      // Both CAS contenders are admitted as durable Service jobs; exactly one
      // later terminates with policy_revision_conflict, without pre-assigning
      // the nondeterministic winner to a request identity.
      requestKind: suffix === 'admission-probe' ? 'rejected_request' : 'accepted_request',
      operationKind: suffix,
      actionId: action.actionId,
      attemptId: action.attemptId,
      caseId: 'A05',
      phaseId: 'W7',
      environmentId,
    };
  });
}

export function enumerateP158W7A05LoggingRequests({ schedule, campaignRunId }) {
  if (typeof campaignRunId !== 'string' || campaignRunId.length === 0) {
    fail('campaign_run_id_missing', 'A05 logging request enumeration requires a campaign run ID');
  }
  const attemptsById = new Map(schedule.attempts.map((attempt) => [attempt.attemptId, attempt]));
  return freeze(actionInventory(schedule).filter((action) => action.caseId === 'A05')
    .flatMap((action) => a05LoggingRequestDescriptors(action, campaignRunId,
      attemptsById.get(action.attemptId)?.environmentIds?.[0])));
}

/**
 * Report the exact current-product boundary. A04 cannot be promoted because
 * service_access_plan evaluates only tab_create. A06 atomic cells have the
 * necessary exact lifecycle APIs, but queued cells have no declared hold and
 * release seam, so the aggregate case remains blocked rather than partially
 * executing a frozen matrix.
 */
export function assessP158W7A04A06ActionReadiness({ schedule }) {
  const actions = actionInventory(schedule).map((action) => {
    let executable = action.caseId === 'A05';
    let blocker = null;
    if (action.caseId === 'A04') {
      blocker = {
        code: 'arbitrary_profile_operation_decision_oracle_missing',
        sourceSymbol: 'cli/src/native/service_profile_acquisition.rs::profile_access_decision',
        detail: 'The public service_access_plan path hardcodes permission TabCreate and operation tab_create.',
      };
    } else if (action.caseId === 'A06' && value(action, 'command_state') === 'queued') {
      blocker = {
        code: 'queued_command_barrier_seam_missing',
        sourceSymbol: 'cli/src/native/control_plane.rs::run_worker',
        detail: 'No declared development API can hold one queued command across revocation and release it exactly once.',
      };
    } else if (action.caseId === 'A06') {
      blocker = {
        code: 'aggregate_case_partial_execution_prohibited',
        sourceSymbol: 'docs/dev/contracts/p158-historical-failure-registry.v1.json#A06',
        detail: 'Atomic lifecycle APIs exist, but the frozen aggregate case cannot run until queued cells are executable.',
      };
    }
    return { ...structuredClone(action), executable, blocker };
  });
  const counts = Object.fromEntries(P158_W7_A04_A06_REVIEWED_CASE_IDS.map((caseId) => {
    const rows = actions.filter((action) => action.caseId === caseId);
    return [caseId, {
      scheduled: rows.length,
      executable: rows.filter((action) => action.executable).length,
      blocked: rows.filter((action) => !action.executable).length,
    }];
  }));
  return freeze({ actions, counts, effectsAttempted: false });
}

function exactSubset(actual, expected, path = 'status') {
  if (!expected || typeof expected !== 'object' || Array.isArray(expected)) {
    if (!Object.is(actual, expected)) fail('ownership_status_mismatch', `${path} did not match`);
    return;
  }
  if (!actual || typeof actual !== 'object' || Array.isArray(actual)) {
    fail('ownership_status_mismatch', `${path} was absent`);
  }
  for (const [key, expectedValue] of Object.entries(expected)) {
    exactSubset(actual[key], expectedValue, `${path}.${key}`);
  }
}

function validateManifest(manifest, schedule) {
  const body = manifest && typeof manifest === 'object'
    ? Object.fromEntries(Object.entries(manifest).filter(([key]) => key !== 'manifestSha256'))
    : null;
  if (manifest?.schemaVersion !== 'agent-browser.p158-w7-a04-a06-ownership.v1' ||
      manifest?.manifestSha256 !== sha256(body) ||
      !manifest?.environmentSealSha256s ||
      ['E0', 'E1'].some((environmentId) =>
        !/^[a-f0-9]{64}$/u.test(manifest.environmentSealSha256s[environmentId] ?? '')) ||
      !/^[a-f0-9]{64}$/u.test(manifest?.candidateSha256 ?? '') ||
      !/^[a-f0-9]{64}$/u.test(manifest?.liveHookManifestSha256 ?? '') ||
      typeof manifest?.campaignRunId !== 'string' || manifest.campaignRunId.length === 0) {
    fail('frozen_ownership_manifest_invalid', 'A04-A06 require a self-hashed ownership manifest');
  }
  const attempts = schedule.attempts.filter((attempt) => attempt.caseId === 'A05');
  for (const attempt of attempts) {
    const environmentId = attempt.environmentIds[0];
    const environment = manifest.environments?.[environmentId];
    const fixture = manifest.fixtures?.A05?.[attempt.attemptId];
    let origin;
    try { origin = new URL(environment?.serviceOrigin); } catch {
      fail('development_service_origin_invalid', environmentId);
    }
    if (origin.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]'].includes(origin.hostname) ||
        !origin.port || environment.runtimeLane !== 'development' || environment.production !== false ||
        environment.runtimeEnvironmentId !== environmentId ||
        typeof environment.ownershipStatus !== 'object') {
      fail('development_service_origin_invalid', environmentId);
    }
    for (const field of ['profileId', 'sessionName', 'url', 'adminSubjectId', 'participantSubjectId']) {
      if (typeof fixture?.[field] !== 'string' || fixture[field].length === 0) {
        fail('frozen_fixture_missing', `${attempt.attemptId}.${field}`);
      }
    }
    for (const role of ['adminCapability', 'participantCapability']) {
      const capability = fixture[role];
      if (typeof capability?.absolutePath !== 'string' || !capability.absolutePath.startsWith('/') ||
          !/^[a-f0-9]{64}$/u.test(capability?.sha256 ?? '') ||
          capability?.principalId !== fixture[role === 'adminCapability' ? 'adminSubjectId' : 'participantSubjectId']) {
        fail('frozen_capability_binding_invalid', `${attempt.attemptId}.${role}`);
      }
    }
  }
  return freeze(structuredClone(manifest));
}

function policy(profileId, mode, revision, defaultPermissions, adminSubjectId) {
  return {
    schemaVersion: 'agent-browser.profile-access-policy.v1',
    profileId,
    mode,
    revision,
    state: 'active',
    defaultPermissions: [...defaultPermissions],
    grants: [{
      subjectId: adminSubjectId,
      minimumAssurance: 'registered-capability',
      permissions: [...PERMISSIONS],
    }],
    drain: null,
    updatedAt: '1970-01-01T00:00:00Z',
  };
}

function profileFixture(fixture, accessPolicy) {
  return {
    id: fixture.profileId,
    name: `P158 disposable ${fixture.profileId}`,
    description: 'Plan 0158 disposable development fixture',
    aliases: [], origins: [], loginIds: [], accountLabels: [],
    profileOrigin: 'agent_browser_owned', profileClass: 'managed_one_time',
    accessPolicy, userDataDir: null, sitePolicyIds: [], targetServiceIds: [],
    authenticatedServiceIds: [], accountIds: [], defaultBrowserHost: 'local_headless',
    browserBuild: 'stock_chrome', allocation: 'shared_service',
    keyring: 'basic_password_store', sharedServiceIds: [], credentialProviderIds: [],
    manualLoginPreferred: false, targetReadiness: [], registration: null,
    browserCompatibilityEvidence: [], persistent: false,
    tags: ['p158', 'disposable', 'development-only'],
  };
}

function requestBase(context, subjectId, requestId) {
  return {
    serviceName: 'p158-a05', agentName: 'p158-w7-live-runner',
    taskName: context.action.actionId, clientSubjectId: subjectId,
    identityAssurance: 'self-declared', runtimeEnvironmentId: context.environmentId,
    requestId, traceId: `${requestId}:trace`, runtimeProfile: context.fixture.profileId,
    profileId: context.fixture.profileId, sessionName: context.fixture.sessionName,
  };
}

function responseData(response) {
  return response?.data ?? response;
}

async function assertTrace(service, context, requestId, subjectId, fetch) {
  const trace = await service.trace({ context, requestId, fetch });
  const jobs = Array.isArray(trace) ? trace : (trace?.jobs ?? trace?.data?.jobs ?? []);
  const job = jobs.find((candidate) => candidate.provenance?.requestId === requestId);
  if (!job || job.provenance?.clientSubjectId !== subjectId ||
      typeof job.provenance?.connectionInstanceId !== 'string' ||
      job.provenance.connectionInstanceId.length === 0) {
    fail('causal_connection_evidence_missing', requestId);
  }
  return job.provenance.connectionInstanceId;
}

export function createP158W7FileCapabilityProvider() {
  const provider = (binding) => {
    const raw = readFileSync(binding.absolutePath, 'utf8').trim();
    if (sha256(raw) !== binding.sha256) fail('capability_digest_mismatch', binding.absolutePath);
    return raw;
  };
  Object.defineProperty(provider, BUILTIN_CAPABILITY, { value: true });
  return provider;
}

export function createP158W7A05DevelopmentService(options = {}) {
  const builtInTransport = !Object.hasOwn(options, 'transportFor');
  const builtInCapability = !Object.hasOwn(options, 'capabilityFor');
  const transportFor = options.transportFor ?? createP158W7PinnedDevelopmentTransports();
  const capabilityFor = options.capabilityFor ?? createP158W7FileCapabilityProvider();
  const service = {
    transportFor,
    capabilityFor,
    async revalidate(context, fetch) {
      const status = await getServiceStatus({ baseUrl: context.environment.serviceOrigin, fetch });
      exactSubset(status, context.environment.ownershipStatus);
      return status;
    },
    upsertProfile: ({ context, profile, fetch, requestId }) => upsertServiceProfile({
      baseUrl: context.environment.serviceOrigin, fetch, id: context.fixture.profileId, profile,
      headers: { 'x-agent-browser-request-id': requestId },
    }),
    async mutate({ context, subjectId, capability, expectedRevision, targetPolicy, fetch, suffix }) {
      const requestId = `${context.manifest.campaignRunId}:${context.action.actionId}:${suffix}`;
      const response = await postServiceRequest({
        baseUrl: context.environment.serviceOrigin, fetch, profileCapability: capability,
        request: createServiceProfilePolicyMutationRequest({
          ...requestBase(context, subjectId, requestId), expectedRevision, targetPolicy,
        }),
      });
      return { response, requestId };
    },
    async open({ context, subjectId, capability, fetch, suffix }) {
      const requestId = `${context.manifest.campaignRunId}:${context.action.actionId}:${suffix}`;
      const response = await requestServiceTab({
        baseUrl: context.environment.serviceOrigin, fetch, profileCapability: capability,
        ...requestBase(context, subjectId, requestId), url: context.fixture.url,
      });
      return { response, requestId };
    },
    async release({ context, subjectId, capability, handle, fetch, suffix }) {
      const requestId = `${context.manifest.campaignRunId}:${context.action.actionId}:${suffix}`;
      const response = await releaseServiceTabHandle({
        baseUrl: context.environment.serviceOrigin, fetch, profileCapability: capability,
        ...requestBase(context, subjectId, requestId), serviceTabHandle: handle,
      });
      return { response, requestId };
    },
    accessPlan: ({ context, subjectId, capability, fetch }) => getServiceAccessPlan({
      baseUrl: context.environment.serviceOrigin, fetch, profileCapability: capability,
      serviceName: 'p158-a05', agentName: 'p158-w7-live-runner', taskName: context.action.actionId,
      clientSubjectId: subjectId, runtimeProfile: context.fixture.profileId,
      sessionName: context.fixture.sessionName, url: context.fixture.url,
    }),
    tabs: ({ context, fetch }) => getServiceTabs({ baseUrl: context.environment.serviceOrigin, fetch }),
    trace: ({ context, requestId, fetch }) => getServiceTrace({
      baseUrl: context.environment.serviceOrigin, fetch, query: { requestId, limit: 100 },
    }),
  };
  if (transportFor && transportFor.close) service.close = () => transportFor.close();
  if (builtInTransport && builtInCapability && capabilityFor[BUILTIN_CAPABILITY] === true) {
    Object.defineProperty(service, BUILTIN_SERVICE, { value: true });
  }
  return service;
}

function requireSuccess(response, code, expected = true) {
  if (response?.success !== expected) fail(code, `Expected success=${expected}`, response);
  return responseData(response);
}

async function setupProfile(context, service, accessPolicy, fetch, requestId) {
  await service.revalidate(context, fetch);
  const response = await service.upsertProfile({
    context, fetch, requestId, profile: profileFixture(context.fixture, accessPolicy),
  });
  if (response?.success === false) fail('fixture_setup_failed', context.action.actionId, response);
}

async function openOccupant(context, service, participantFetch, participantCapability) {
  const opened = await service.open({
    context, fetch: participantFetch, subjectId: context.fixture.participantSubjectId,
    capability: participantCapability, suffix: 'occupant-open',
  });
  const data = requireSuccess(opened.response, 'occupant_open_failed');
  const handle = data?.serviceTabHandle;
  if (!handle?.tabId || handle.profileId !== context.fixture.profileId) {
    fail('occupant_handle_invalid', opened.requestId, data);
  }
  const connectionInstanceId = await assertTrace(
    service, context, opened.requestId, context.fixture.participantSubjectId, participantFetch,
  );
  return { ...opened, handle, connectionInstanceId };
}

async function runA05Transition({ manifest, attempt, service, receiptStore, clock }) {
  // enumerateP158W7ActionPlans requires the complete schedule, so the bundle
  // attaches the corresponding frozen action before execution.
  const frozenAction = attempt.p158Action;
  if (!frozenAction || frozenAction.caseId !== 'A05') fail('frozen_action_missing', attempt.attemptId);
  const environmentId = attempt.environmentIds[0];
  const context = {
    manifest, attempt, action: frozenAction, environmentId,
    environment: manifest.environments[environmentId], fixture: manifest.fixtures.A05[attempt.attemptId],
  };
  const transition = value(frozenAction, 'transition');
  const adminFetch = service.transportFor({ action: { actionId: `${frozenAction.actionId}:admin` } });
  const participantFetch = service.transportFor({ action: { actionId: `${frozenAction.actionId}:participant` } });
  const adminCapability = service.capabilityFor(context.fixture.adminCapability);
  const participantCapability = service.capabilityFor(context.fixture.participantCapability);
  const shared = policy(context.fixture.profileId, 'shared-local', 1, PARTICIPANT_PERMISSIONS,
    context.fixture.adminSubjectId);
  const restricted = policy(context.fixture.profileId, 'restricted', 1, [], context.fixture.adminSubjectId);
  const targetShared = { mode: 'shared-local', defaultPermissions: [...PARTICIPANT_PERMISSIONS], grants: shared.grants };
  const targetRestricted = { mode: 'restricted', defaultPermissions: [], grants: shared.grants };
  const loggingRequestExpectations = a05LoggingRequestDescriptors(
    frozenAction, manifest.campaignRunId, environmentId,
  );
  const requestIds = loggingRequestExpectations.map((entry) => entry.requestId);
  const requestIdFor = (suffix) => `${manifest.campaignRunId}:${frozenAction.actionId}:${suffix}`;
  const connectionInstanceIds = [];
  let effectObserved = false;
  try {
    await setupProfile(context, service, transition === 'widen' ? restricted : shared, adminFetch,
      requestIdFor('fixture-setup'));
    let occupant = null;
    if (['admission', 'own_tab_release', 'drain_completion'].includes(transition)) {
      occupant = await openOccupant(context, service, participantFetch, participantCapability);
      connectionInstanceIds.push(occupant.connectionInstanceId);
      effectObserved = true;
    }
    await service.revalidate(context, adminFetch);
    if (transition === 'revision_conflict') {
      const first = service.mutate({ context, fetch: adminFetch, subjectId: context.fixture.adminSubjectId,
        capability: adminCapability, expectedRevision: 1, targetPolicy: targetRestricted, suffix: 'conflict-a' });
      const second = service.mutate({ context, fetch: participantFetch, subjectId: context.fixture.adminSubjectId,
        capability: adminCapability, expectedRevision: 1, targetPolicy: targetRestricted, suffix: 'conflict-b' });
      const results = await Promise.all([first, second]);
      const passed = results.filter((entry) => entry.response?.success === true);
      const conflicted = results.filter((entry) => entry.response?.success === false &&
        JSON.stringify(entry.response).includes('policy_revision_conflict'));
      if (passed.length !== 1 || conflicted.length !== 1) fail('revision_conflict_oracle_failed', frozenAction.actionId, results);
      for (const result of results) connectionInstanceIds.push(await assertTrace(
        service, context, result.requestId, context.fixture.adminSubjectId,
        result === results[0] ? adminFetch : participantFetch,
      ));
      effectObserved = true;
    } else {
      const targetPolicy = transition === 'widen' ? targetShared : targetRestricted;
      const mutated = await service.mutate({
        context, fetch: adminFetch, subjectId: context.fixture.adminSubjectId,
        capability: adminCapability, expectedRevision: 1, targetPolicy, suffix: 'policy-mutate',
      });
      connectionInstanceIds.push(await assertTrace(
        service, context, mutated.requestId, context.fixture.adminSubjectId, adminFetch,
      ));
      const mutation = requireSuccess(mutated.response, 'policy_transition_failed');
      const expectedOutcome = transition === 'widen' ? 'widened'
        : (['admission', 'own_tab_release', 'drain_completion'].includes(transition) ? 'drain_started' : 'restricted');
      if (mutation?.outcome !== expectedOutcome ||
          (expectedOutcome === 'widened' || expectedOutcome === 'restricted') && mutation.policy?.revision !== 2 ||
          expectedOutcome === 'drain_started' && mutation.policy?.state !== 'draining') {
        fail('policy_transition_oracle_failed', frozenAction.actionId, mutation);
      }
      effectObserved = true;
      if (transition === 'admission') {
        const plan = await service.accessPlan({ context, fetch: participantFetch,
          subjectId: context.fixture.participantSubjectId, capability: participantCapability });
        const decision = plan?.decision?.profileAccess?.decision;
        const policyState = plan?.decision?.profileAccess?.policy?.state;
        if (decision?.allowed !== false || decision?.operation !== 'tab_create' ||
            decision?.nextAction?.action !== 'inspect_profile_occupancy' ||
            policyState !== 'draining' ||
            plan?.decision?.serviceRequest?.acquisitionBlocker !== 'profile_access_denied') {
          fail('draining_admission_reason_oracle_failed', frozenAction.actionId, plan);
        }
        const denied = await service.open({ context, fetch: participantFetch,
          subjectId: context.fixture.participantSubjectId, capability: participantCapability,
          suffix: 'admission-probe' });
        if (denied.response?.success !== false || !JSON.stringify(denied.response).includes('profile_access_denied')) {
          fail('draining_admission_oracle_failed', frozenAction.actionId, denied.response);
        }
      }
      if (transition === 'own_tab_release' || transition === 'drain_completion') {
        const released = await service.release({ context, fetch: participantFetch,
          subjectId: context.fixture.participantSubjectId, capability: participantCapability,
          handle: occupant.handle, suffix: 'own-release' });
        requireSuccess(released.response, 'own_tab_release_oracle_failed');
        const tabs = await service.tabs({ context, fetch: participantFetch });
        const rows = tabs?.tabs ?? tabs?.data?.tabs ?? [];
        if (!rows.some((tab) => tab.id === occupant.handle.tabId && tab.lifecycle === 'closed')) {
          fail('own_tab_release_oracle_failed', frozenAction.actionId, tabs);
        }
      }
      if (transition === 'drain_completion') {
        await service.revalidate(context, adminFetch);
        const completed = await service.mutate({ context, fetch: adminFetch,
          subjectId: context.fixture.adminSubjectId, capability: adminCapability,
          expectedRevision: 1, targetPolicy: targetRestricted, suffix: 'drain-complete' });
        const completion = requireSuccess(completed.response, 'drain_completion_failed');
        if (completion?.outcome !== 'restricted' || completion.policy?.revision !== 2 ||
            completion.policy?.state !== 'active') {
          fail('drain_completion_oracle_failed', frozenAction.actionId, completion);
        }
      }
    }
    if (connectionInstanceIds.length >= 2 && new Set(connectionInstanceIds).size < 2) {
      fail('distinct_connection_evidence_failed', frozenAction.actionId, connectionInstanceIds);
    }
    const receipt = {
      schemaVersion: 'agent-browser.p158-w7-action-receipt.v1',
      campaignRunId: manifest.campaignRunId, caseId: 'A05', attemptId: attempt.attemptId,
      actionId: frozenAction.actionId, environmentId, transition,
      clientSubjectIds: [context.fixture.adminSubjectId, context.fixture.participantSubjectId],
      connectionInstanceIds, requestIds, state: 'passed', attempt: 1,
      resultState: 'passed', effectState: 'verified_effect', observedAt: clock(),
      retryDisposition: 'prohibited_opportunistic_retry', repairAttempted: false,
      retryAttempted: false, garbageCollectionAttempted: false,
    };
    receipt.receiptSha256 = sha256(receipt);
    await receiptStore.append(freeze(structuredClone(receipt)));
    return freeze({ resultState: 'passed', actionCount: 1, actionIds: [frozenAction.actionId],
      receipts: [receipt], artifactIds: [`p158-w7-action:${receipt.receiptSha256}`],
      effectState: 'verified_effect', retryDisposition: 'prohibited_opportunistic_retry',
      repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false });
  } catch (error) {
    const productCodes = new Set([
      'policy_transition_oracle_failed', 'revision_conflict_oracle_failed',
      'draining_admission_oracle_failed', 'own_tab_release_oracle_failed',
      'draining_admission_reason_oracle_failed', 'drain_completion_oracle_failed',
      'distinct_connection_evidence_failed',
    ]);
    const inconclusiveCodes = new Set(['ownership_status_mismatch', 'causal_connection_evidence_missing']);
    const resultState = productCodes.has(error?.code) ? 'reproduced_historical_failure'
      : (inconclusiveCodes.has(error?.code) ? 'inconclusive' : 'harness_failure');
    const receipt = {
      schemaVersion: 'agent-browser.p158-w7-action-receipt.v1', campaignRunId: manifest.campaignRunId,
      caseId: 'A05', attemptId: attempt.attemptId, actionId: frozenAction.actionId,
      environmentId, transition, clientSubjectIds: [context.fixture.adminSubjectId, context.fixture.participantSubjectId],
      connectionInstanceIds, requestIds, state: 'failed', attempt: 1, resultState,
      effectState: effectObserved ? 'effect_uncertain' : 'no_effect', observedAt: clock(),
      failure: { code: error?.code ?? 'service_transport_failed', message: error?.message ?? String(error) },
      retryDisposition: 'prohibited_opportunistic_retry', repairAttempted: false,
      retryAttempted: false, garbageCollectionAttempted: false,
    };
    receipt.receiptSha256 = sha256(receipt);
    await receiptStore.append(freeze(structuredClone(receipt)));
    return freeze({ resultState, actionCount: 1, actionIds: [frozenAction.actionId], receipts: [receipt],
      artifactIds: [`p158-w7-action:${receipt.receiptSha256}`], effectState: receipt.effectState,
      retryDisposition: 'prohibited_opportunistic_retry', repairAttempted: false,
      retryAttempted: false, garbageCollectionAttempted: false });
  }
}

export function createP158W7A04A06LiveBundle({
  schedule, ownershipManifest, receiptStore,
  service = createP158W7A05DevelopmentService(), clock = () => new Date().toISOString(),
}) {
  const manifest = validateManifest(ownershipManifest, schedule);
  if (typeof receiptStore?.append !== 'function' || typeof clock !== 'function') {
    fail('live_dependency_missing', 'A05 requires an append-only receipt store and clock');
  }
  const contract = schedule.caseContracts.find((entry) => entry.caseId === 'A05');
  if (!contract) fail('case_contract_missing', 'A05');
  const actionsByAttempt = new Map(actionInventory(schedule)
    .filter((action) => action.caseId === 'A05').map((action) => [action.attemptId, action]));
  const adapter = createP158CaseAdapter({
    caseId: 'A05', evidenceProfile: contract.evidenceProfile,
    executionContract: contract.executionContract,
    execute: ({ attempt }) => runA05Transition({
      manifest, attempt: { ...attempt, p158Action: actionsByAttempt.get(attempt.attemptId) },
      service, receiptStore, clock,
    }),
  });
  const source = freeze({ sourcePath: P158_W7_A04_A06_SOURCE_PATH, sourceSha256: sourceSha256() });
  const readiness = assessP158W7A04A06ActionReadiness({ schedule });
  const loggingRequestExpectations = enumerateP158W7A05LoggingRequests({
    schedule, campaignRunId: manifest.campaignRunId,
  });
  return freeze({
    schemaVersion: 'agent-browser.p158-w7-a04-a06-live-bundle.v1',
    freezeEligible: service[BUILTIN_SERVICE] === true,
    providerFree: false,
    concreteCaseIds: ['A05'], adapters: [adapter], readiness,
    ownershipManifestSha256: manifest.manifestSha256,
    campaignRunId: manifest.campaignRunId, candidateSha256: manifest.candidateSha256,
    liveHookManifestSha256: manifest.liveHookManifestSha256,
    environmentSealSha256s: structuredClone(manifest.environmentSealSha256s),
    liveHookIds: [P158_W7_A04_A06_HOOK_ID], driverSource: source,
    loggingRequestExpectations,
    adapterBindingSha256: sha256({ caseIds: ['A05'], ownershipManifestSha256: manifest.manifestSha256,
      campaignRunId: manifest.campaignRunId, candidateSha256: manifest.candidateSha256,
      liveHookManifestSha256: manifest.liveHookManifestSha256,
      environmentSealSha256s: manifest.environmentSealSha256s, source,
      liveHookIds: [P158_W7_A04_A06_HOOK_ID] }),
  });
}

export function createP158W7A04A06OwnershipManifest(input) {
  const body = structuredClone(input);
  return freeze({ ...body, manifestSha256: sha256(body) });
}

export function p158W7A04A06SourceBinding() {
  return freeze({ hookId: P158_W7_A04_A06_HOOK_ID,
    sourcePath: P158_W7_A04_A06_SOURCE_PATH, sourceSha256: sourceSha256() });
}
