// @ts-check

export {
  SERVICE_BROWSER_HEALTH_STATES,
  SERVICE_EVENT_KINDS,
  SERVICE_INCIDENT_ESCALATIONS,
  SERVICE_INCIDENT_HANDLING_STATES,
  SERVICE_INCIDENT_SEVERITIES,
  SERVICE_INCIDENT_STATES,
  SERVICE_JOB_PRIORITIES,
  SERVICE_JOB_STATES,
  SERVICE_NAMING_WARNINGS,
} from './service-observability.generated.js';

/**
 * @typedef {import('./service-observability.generated.js').ServiceEventsResponse} ServiceEventsResponse
 * @typedef {import('./service-observability.generated.js').ServiceIdOptions} ServiceIdOptions
 * @typedef {import('./service-observability.generated.js').ServiceIncidentActivityOptions} ServiceIncidentActivityOptions
 * @typedef {import('./service-observability.generated.js').ServiceIncidentActivityResponse} ServiceIncidentActivityResponse
 * @typedef {import('./service-observability.generated.js').ServiceIncidentsResponse} ServiceIncidentsResponse
 * @typedef {import('./service-observability.generated.js').ServiceJobsResponse} ServiceJobsResponse
 * @typedef {import('./service-observability.generated.js').ServiceBrowsersResponse} ServiceBrowsersResponse
 * @typedef {import('./service-observability.generated.js').ServiceChallengesResponse} ServiceChallengesResponse
 * @typedef {import('./service-observability.generated.js').ServiceObservabilityHttpOptions} ServiceObservabilityHttpOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfilesResponse} ServiceProfilesResponse
 * @typedef {import('./service-observability.generated.js').ServiceProvidersResponse} ServiceProvidersResponse
 * @typedef {import('./service-observability.generated.js').ServiceQueryOptions} ServiceQueryOptions
 * @typedef {import('./service-observability.generated.js').ServiceReconcileResponse} ServiceReconcileResponse
 * @typedef {import('./service-observability.generated.js').ServiceSessionsResponse} ServiceSessionsResponse
 * @typedef {import('./service-observability.generated.js').ServiceSitePoliciesResponse} ServiceSitePoliciesResponse
 * @typedef {import('./service-observability.generated.js').ServiceStatusResponse} ServiceStatusResponse
 * @typedef {import('./service-observability.generated.js').ServiceTabsResponse} ServiceTabsResponse
 * @typedef {import('./service-observability.generated.js').ServiceTraceResponse} ServiceTraceResponse
 * @typedef {import('./service-observability.generated.js').ServiceLoginProfileRegistrationOptions} ServiceLoginProfileRegistrationOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileDeleteResponse} ServiceProfileDeleteResponse
 * @typedef {import('./service-observability.generated.js').ServiceProfileAllocationResponse} ServiceProfileAllocationResponse
 * @typedef {import('./service-observability.generated.js').ServiceProfileReadinessResponse} ServiceProfileReadinessResponse
 * @typedef {import('./service-observability.generated.js').ServiceProfileMutationOptions} ServiceProfileMutationOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileUpsertResponse} ServiceProfileUpsertResponse
 * @typedef {import('./service-observability.generated.js').ServiceProviderDeleteResponse} ServiceProviderDeleteResponse
 * @typedef {import('./service-observability.generated.js').ServiceProviderMutationOptions} ServiceProviderMutationOptions
 * @typedef {import('./service-observability.generated.js').ServiceProviderUpsertResponse} ServiceProviderUpsertResponse
 * @typedef {import('./service-observability.generated.js').ServiceContractsResponse} ServiceContractsResponse
 * @typedef {import('./service-observability.generated.js').ServiceBrowserRetryOptions} ServiceBrowserRetryOptions
 * @typedef {import('./service-observability.generated.js').ServiceBrowserRetryResponse} ServiceBrowserRetryResponse
 * @typedef {import('./service-observability.generated.js').ServiceIncidentAcknowledgeResponse} ServiceIncidentAcknowledgeResponse
 * @typedef {import('./service-observability.generated.js').ServiceIncidentMutationOptions} ServiceIncidentMutationOptions
 * @typedef {import('./service-observability.generated.js').ServiceIncidentResolveResponse} ServiceIncidentResolveResponse
 * @typedef {import('./service-observability.generated.js').ServiceJobCancelOptions} ServiceJobCancelOptions
 * @typedef {import('./service-observability.generated.js').ServiceJobCancelResponse} ServiceJobCancelResponse
 * @typedef {import('./service-observability.generated.js').ServiceSessionDeleteResponse} ServiceSessionDeleteResponse
 * @typedef {import('./service-observability.generated.js').ServiceSessionMutationOptions} ServiceSessionMutationOptions
 * @typedef {import('./service-observability.generated.js').ServiceSessionUpsertResponse} ServiceSessionUpsertResponse
 * @typedef {import('./service-observability.generated.js').ServiceSitePolicyDeleteResponse} ServiceSitePolicyDeleteResponse
 * @typedef {import('./service-observability.generated.js').ServiceSitePolicyMutationOptions} ServiceSitePolicyMutationOptions
 * @typedef {import('./service-observability.generated.js').ServiceSitePolicyUpsertResponse} ServiceSitePolicyUpsertResponse
 * @typedef {import('./service-observability.generated.js').ServiceProfileRecord} ServiceProfileRecord
 * @typedef {import('./service-observability.generated.js').ServiceProfileIdentityMatchOptions} ServiceProfileIdentityMatchOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileIdentityMatchResult} ServiceProfileIdentityMatchResult
 * @typedef {import('./service-observability.generated.js').ServiceProfileIdentityLookupOptions} ServiceProfileIdentityLookupOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileLookupResponse} ServiceProfileLookupResponse
 * @typedef {import('./service-observability.generated.js').ServiceAccessPlanOptions} ServiceAccessPlanOptions
 * @typedef {import('./service-observability.generated.js').ServiceAccessPlanResponse} ServiceAccessPlanResponse
 * @typedef {import('./service-observability.generated.js').ServiceProfileFreshnessUpdateOptions} ServiceProfileFreshnessUpdateOptions
 */

/**
 * @param {ServiceObservabilityHttpOptions} options
 * @returns {Promise<ServiceStatusResponse>}
 */
export function getServiceStatus(options) {
  return serviceGet(options, '/api/service/status');
}

/**
 * @param {ServiceObservabilityHttpOptions} options
 * @returns {Promise<ServiceContractsResponse>}
 */
export function getServiceContracts(options) {
  return serviceGet(options, '/api/service/contracts');
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceProfilesResponse>}
 */
export function getServiceProfiles(options) {
  return serviceGet(options, '/api/service/profiles');
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceProfileAllocationResponse>}
 */
export function getServiceProfileAllocation({ id, ...options }) {
  assertServiceId(id, 'getServiceProfileAllocation');
  return serviceGet(options, `/api/service/profiles/${encodeURIComponent(id)}/allocation`);
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceProfileReadinessResponse>}
 */
export function getServiceProfileReadiness({ id, ...options }) {
  assertServiceId(id, 'getServiceProfileReadiness');
  return serviceGet(options, `/api/service/profiles/${encodeURIComponent(id)}/readiness`);
}

/**
 * @param {ServiceProfileReadinessResponse | null | undefined} readiness
 * @returns {import('./service-observability.generated.js').ServiceProfileReadinessSummary}
 */
export function summarizeServiceProfileReadiness(readiness) {
  const rows = Array.isArray(readiness?.targetReadiness) ? readiness.targetReadiness : [];
  const manualRows = rows.filter(
    (row) => row?.state === 'needs_manual_seeding' || row?.manualSeedingRequired === true,
  );
  return {
    needsManualSeeding: manualRows.some((row) => row?.state === 'needs_manual_seeding'),
    manualSeedingRequired: manualRows.length > 0,
    targetServiceIds: manualRows.map((row) => row.targetServiceId),
    recommendedActions: [...new Set(manualRows.map((row) => row.recommendedAction).filter(Boolean))],
  };
}

/**
 * @param {ServiceProfileRecord[] | undefined | null} profiles
 * @param {ServiceProfileIdentityMatchOptions} options
 * @returns {ServiceProfileIdentityMatchResult}
 */
export function findServiceProfileForIdentity(profiles, options) {
  const candidates = Array.isArray(profiles) ? profiles : [];
  const identities = uniqueStrings([
    options?.loginId,
    options?.siteId,
    options?.targetServiceId,
    ...(options?.loginIds ?? []),
    ...(options?.siteIds ?? []),
    ...(options?.targetServiceIds ?? []),
  ]);
  const identitySet = new Set(identities);
  const serviceName = options?.serviceName;

  const authenticatedProfile = candidates.find((profile) =>
    profileMatchesAny(profile, identitySet, 'authenticatedServiceIds'),
  );
  if (authenticatedProfile) {
    return {
      profile: authenticatedProfile,
      reason: 'authenticated_target',
      matchedField: 'authenticatedServiceIds',
      matchedIdentity: firstMatchingProfileValue(authenticatedProfile, identitySet, 'authenticatedServiceIds'),
    };
  }

  const targetProfile = candidates.find((profile) => profileMatchesAny(profile, identitySet, 'targetServiceIds'));
  if (targetProfile) {
    return {
      profile: targetProfile,
      reason: 'target_match',
      matchedField: 'targetServiceIds',
      matchedIdentity: firstMatchingProfileValue(targetProfile, identitySet, 'targetServiceIds'),
    };
  }

  const serviceProfile =
    typeof serviceName === 'string' && serviceName.length > 0
      ? candidates.find((profile) => profileMatchesAny(profile, new Set([serviceName]), 'sharedServiceIds'))
      : null;
  if (serviceProfile) {
    return {
      profile: serviceProfile,
      reason: 'service_allow_list',
      matchedField: 'sharedServiceIds',
      matchedIdentity: serviceName,
    };
  }

  return {
    profile: null,
    reason: null,
    matchedField: null,
    matchedIdentity: null,
  };
}

/**
 * @param {ServiceProfileIdentityLookupOptions} options
 * @returns {Promise<ServiceProfileLookupResponse>}
 */
export async function getServiceProfileForIdentity({ readinessProfileId, ...options }) {
  return serviceGet(
    {
      ...options,
      query: {
        ...options.query,
        serviceName: options.serviceName,
        loginId: options.loginId,
        siteId: options.siteId,
        targetServiceId: options.targetServiceId,
        loginIds: options.loginIds?.join(','),
        siteIds: options.siteIds?.join(','),
        targetServiceIds: options.targetServiceIds?.join(','),
        readinessProfileId,
      },
    },
    '/api/service/profiles/lookup',
  );
}

/**
 * @param {ServiceProfileIdentityLookupOptions} options
 * @returns {Promise<ServiceProfileLookupResponse>}
 */
export function lookupServiceProfile(options) {
  return getServiceProfileForIdentity(options);
}

/**
 * Ask agent-browser for a no-launch profile, policy, provider, challenge, and
 * readiness recommendation before requesting browser control.
 *
 * @param {ServiceAccessPlanOptions} options
 * @returns {Promise<ServiceAccessPlanResponse>}
 */
export async function getServiceAccessPlan({ readinessProfileId, sitePolicyId, challengeId, ...options }) {
  return serviceGet(
    {
      ...options,
      query: {
        ...options.query,
        serviceName: options.serviceName,
        agentName: options.agentName,
        taskName: options.taskName,
        loginId: options.loginId,
        siteId: options.siteId,
        targetServiceId: options.targetServiceId,
        loginIds: options.loginIds?.join(','),
        siteIds: options.siteIds?.join(','),
        targetServiceIds: options.targetServiceIds?.join(','),
        readinessProfileId,
        sitePolicyId,
        challengeId,
      },
    },
    '/api/service/access-plan',
  );
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceBrowsersResponse>}
 */
export function getServiceBrowsers(options) {
  return serviceGet(options, '/api/service/browsers');
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceSessionsResponse>}
 */
export function getServiceSessions(options) {
  return serviceGet(options, '/api/service/sessions');
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceTabsResponse>}
 */
export function getServiceTabs(options) {
  return serviceGet(options, '/api/service/tabs');
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceSitePoliciesResponse>}
 */
export function getServiceSitePolicies(options) {
  return serviceGet(options, '/api/service/site-policies');
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceProvidersResponse>}
 */
export function getServiceProviders(options) {
  return serviceGet(options, '/api/service/providers');
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceChallengesResponse>}
 */
export function getServiceChallenges(options) {
  return serviceGet(options, '/api/service/challenges');
}

/**
 * @param {ServiceObservabilityHttpOptions} options
 * @returns {Promise<ServiceReconcileResponse>}
 */
export function postServiceReconcile(options) {
  return servicePost(options, '/api/service/reconcile');
}

/**
 * @param {ServiceProfileMutationOptions} options
 * @returns {Promise<ServiceProfileUpsertResponse>}
 */
export function upsertServiceProfile({ id, profile, ...options }) {
  return servicePost(options, `/api/service/profiles/${encodeURIComponent(id)}`, profile);
}

/**
 * @param {ServiceLoginProfileRegistrationOptions} options
 * @returns {Promise<ServiceProfileUpsertResponse>}
 */
export function registerServiceLoginProfile({
  id,
  serviceName,
  loginId,
  siteId,
  targetServiceId,
  targetServiceIds = [],
  authenticatedServiceIds = [],
  sharedServiceIds = [],
  targetReadiness = [],
  readinessState,
  readinessEvidence,
  readinessRecommendedAction,
  lastVerifiedAt,
  freshnessExpiresAt,
  name,
  allocation = 'per_service',
  keyring = 'basic_password_store',
  persistent = true,
  authenticated = true,
  userDataDir,
  profile = {},
  ...options
}) {
  if (typeof id !== 'string' || id.length === 0) {
    throw new TypeError('registerServiceLoginProfile requires an id string');
  }
  const targetId = loginId ?? siteId ?? targetServiceId;
  if (!targetId && targetServiceIds.length === 0) {
    throw new TypeError('registerServiceLoginProfile requires loginId, siteId, targetServiceId, or targetServiceIds');
  }
  if (typeof serviceName !== 'string' || serviceName.length === 0) {
    throw new TypeError('registerServiceLoginProfile requires a serviceName string');
  }

  const targets = uniqueStrings([...targetServiceIds, targetId]);
  const authenticatedTargets = authenticated
    ? uniqueStrings([...authenticatedServiceIds, ...targets])
    : uniqueStrings(authenticatedServiceIds);
  const sharedServices = uniqueStrings([...sharedServiceIds, serviceName]);
  const readinessRows = serviceLoginProfileTargetReadiness({
    targets,
    loginId: targetId,
    authenticated,
    targetReadiness,
    readinessState,
    readinessEvidence,
    readinessRecommendedAction,
    lastVerifiedAt,
    freshnessExpiresAt,
  });
  const userDataDirRecord = userDataDir === undefined ? {} : { userDataDir };
  const targetReadinessRecord = readinessRows.length === 0 ? {} : { targetReadiness: readinessRows };

  return upsertServiceProfile({
    ...options,
    id,
    profile: {
      name: name ?? id,
      allocation,
      keyring,
      persistent,
      targetServiceIds: targets,
      authenticatedServiceIds: authenticatedTargets,
      ...targetReadinessRecord,
      sharedServiceIds: sharedServices,
      ...userDataDirRecord,
      ...profile,
    },
  });
}

/**
 * Merge bounded-probe freshness evidence into an existing managed profile.
 *
 * @param {ServiceProfileFreshnessUpdateOptions} options
 * @returns {Promise<ServiceProfileUpsertResponse>}
 */
export async function updateServiceProfileFreshness({
  id,
  profile,
  loginId,
  siteId,
  targetServiceId,
  targetServiceIds = [],
  readinessState = 'fresh',
  readinessEvidence,
  readinessRecommendedAction,
  lastVerifiedAt,
  freshnessExpiresAt,
  updateAuthenticatedServiceIds = true,
  ...options
}) {
  assertServiceId(id, 'updateServiceProfileFreshness');
  const profileRecord = profile ?? (await fetchServiceProfileRecord({ id, ...options }));
  const targetId = loginId ?? siteId ?? targetServiceId;
  const targets = uniqueStrings([...targetServiceIds, targetId]);
  if (targets.length === 0) {
    throw new TypeError(
      'updateServiceProfileFreshness requires loginId, siteId, targetServiceId, or targetServiceIds',
    );
  }

  const readinessRows = serviceLoginProfileTargetReadiness({
    targets,
    loginId: targetId,
    authenticated: readinessState === 'fresh',
    targetReadiness: [],
    readinessState,
    readinessEvidence,
    readinessRecommendedAction,
    lastVerifiedAt: lastVerifiedAt ?? new Date().toISOString(),
    freshnessExpiresAt,
  });
  const existingReadiness = Array.isArray(profileRecord.targetReadiness) ? profileRecord.targetReadiness : [];
  const targetReadiness = mergeServiceProfileTargetReadiness(existingReadiness, readinessRows);
  /** @type {Record<string, unknown>} */
  const profilePatch = {
    ...profileRecord,
    targetServiceIds: uniqueStrings([...stringArray(profileRecord.targetServiceIds), ...targets]),
    targetReadiness,
  };

  if (updateAuthenticatedServiceIds) {
    const authenticatedTargets = new Set(uniqueStrings(stringArray(profileRecord.authenticatedServiceIds)));
    for (const target of targets) {
      if (readinessState === 'fresh' || readinessState === 'seeded_unknown_freshness') {
        authenticatedTargets.add(target);
      } else {
        authenticatedTargets.delete(target);
      }
    }
    profilePatch.authenticatedServiceIds = [...authenticatedTargets];
  }

  return upsertServiceProfile({
    ...options,
    id,
    profile: profilePatch,
  });
}

function serviceLoginProfileTargetReadiness({
  targets,
  loginId,
  authenticated,
  targetReadiness,
  readinessState,
  readinessEvidence,
  readinessRecommendedAction,
  lastVerifiedAt,
  freshnessExpiresAt,
}) {
  const explicitRows = Array.isArray(targetReadiness) ? targetReadiness : [];
  const shouldGenerate =
    readinessState !== undefined ||
    readinessEvidence !== undefined ||
    readinessRecommendedAction !== undefined ||
    lastVerifiedAt !== undefined ||
    freshnessExpiresAt !== undefined;
  if (!shouldGenerate) {
    return explicitRows;
  }

  const state = readinessState ?? (authenticated ? 'fresh' : 'stale');
  const generatedRows = targets.map((targetServiceId) => ({
    targetServiceId,
    loginId: loginId ?? null,
    state,
    manualSeedingRequired: state === 'needs_manual_seeding',
    evidence: readinessEvidence ?? (state === 'fresh' ? 'client_reported_authenticated' : 'client_reported_stale'),
    recommendedAction: readinessRecommendedAction ?? serviceLoginProfileReadinessAction(state),
    lastVerifiedAt: lastVerifiedAt ?? null,
    freshnessExpiresAt: freshnessExpiresAt ?? null,
  }));
  const rowsByTarget = new Map();
  for (const row of generatedRows) {
    rowsByTarget.set(row.targetServiceId, row);
  }
  for (const row of explicitRows) {
    if (typeof row?.targetServiceId === 'string' && row.targetServiceId.length > 0) {
      rowsByTarget.set(row.targetServiceId, row);
    }
  }
  return [...rowsByTarget.values()];
}

function mergeServiceProfileTargetReadiness(existingRows, updateRows) {
  const rowsByTarget = new Map();
  for (const row of existingRows) {
    if (typeof row?.targetServiceId === 'string' && row.targetServiceId.length > 0) {
      rowsByTarget.set(row.targetServiceId, row);
    }
  }
  for (const row of updateRows) {
    if (typeof row?.targetServiceId === 'string' && row.targetServiceId.length > 0) {
      rowsByTarget.set(row.targetServiceId, row);
    }
  }
  return [...rowsByTarget.values()];
}

function stringArray(value) {
  return Array.isArray(value) ? value.filter((item) => typeof item === 'string') : [];
}

function serviceLoginProfileReadinessAction(state) {
  switch (state) {
    case 'fresh':
      return 'use_profile';
    case 'stale':
      return 'probe_target_auth_or_reseed_if_needed';
    case 'needs_manual_seeding':
      return 'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable';
    case 'seeded_unknown_freshness':
      return 'probe_target_auth_or_reuse_if_acceptable';
    case 'blocked_by_attached_devtools':
      return 'close_attached_devtools_then_verify_profile';
    default:
      return 'verify_or_seed_profile_before_authenticated_work';
  }
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceProfileRecord>}
 */
async function fetchServiceProfileRecord({ id, ...options }) {
  const profiles = await getServiceProfiles(options);
  const profile = Array.isArray(profiles?.profiles)
    ? profiles.profiles.find((candidate) => candidate?.id === id)
    : null;
  if (!profile) {
    throw new Error(`updateServiceProfileFreshness could not find profile ${id}`);
  }
  return profile;
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceProfileDeleteResponse>}
 */
export function deleteServiceProfile({ id, ...options }) {
  return serviceDelete(options, `/api/service/profiles/${encodeURIComponent(id)}`);
}

/**
 * @param {ServiceSessionMutationOptions} options
 * @returns {Promise<ServiceSessionUpsertResponse>}
 */
export function upsertServiceSession({ id, session, ...options }) {
  return servicePost(options, `/api/service/sessions/${encodeURIComponent(id)}`, session);
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceSessionDeleteResponse>}
 */
export function deleteServiceSession({ id, ...options }) {
  return serviceDelete(options, `/api/service/sessions/${encodeURIComponent(id)}`);
}

/**
 * @param {ServiceSitePolicyMutationOptions} options
 * @returns {Promise<ServiceSitePolicyUpsertResponse>}
 */
export function upsertServiceSitePolicy({ id, sitePolicy, ...options }) {
  return servicePost(options, `/api/service/site-policies/${encodeURIComponent(id)}`, sitePolicy);
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceSitePolicyDeleteResponse>}
 */
export function deleteServiceSitePolicy({ id, ...options }) {
  return serviceDelete(options, `/api/service/site-policies/${encodeURIComponent(id)}`);
}

/**
 * @param {ServiceProviderMutationOptions} options
 * @returns {Promise<ServiceProviderUpsertResponse>}
 */
export function upsertServiceProvider({ id, provider, ...options }) {
  return servicePost(options, `/api/service/providers/${encodeURIComponent(id)}`, provider);
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceProviderDeleteResponse>}
 */
export function deleteServiceProvider({ id, ...options }) {
  return serviceDelete(options, `/api/service/providers/${encodeURIComponent(id)}`);
}

/**
 * @param {ServiceJobCancelOptions} options
 * @returns {Promise<ServiceJobCancelResponse>}
 */
export function cancelServiceJob({ jobId, ...options }) {
  return servicePost(options, `/api/service/jobs/${encodeURIComponent(jobId)}/cancel`);
}

/**
 * @param {ServiceBrowserRetryOptions} options
 * @returns {Promise<ServiceBrowserRetryResponse>}
 */
export function retryServiceBrowser({ browserId, by, note, serviceName, agentName, taskName, ...options }) {
  return servicePost(options, `/api/service/browsers/${browserId}/retry`, undefined, {
    by,
    note,
    'service-name': serviceName,
    'agent-name': agentName,
    'task-name': taskName,
  });
}

/**
 * @param {ServiceIncidentMutationOptions} options
 * @returns {Promise<ServiceIncidentAcknowledgeResponse>}
 */
export function acknowledgeServiceIncident({ incidentId, by, note, ...options }) {
  return servicePost(
    options,
    `/api/service/incidents/${encodeURIComponent(incidentId)}/acknowledge`,
    undefined,
    { by, note },
  );
}

/**
 * @param {ServiceIncidentMutationOptions} options
 * @returns {Promise<ServiceIncidentResolveResponse>}
 */
export function resolveServiceIncident({ incidentId, by, note, ...options }) {
  return servicePost(
    options,
    `/api/service/incidents/${encodeURIComponent(incidentId)}/resolve`,
    undefined,
    { by, note },
  );
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceJobsResponse>}
 */
export function getServiceJobs(options) {
  return serviceGet(options, '/api/service/jobs');
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceJobsResponse>}
 */
export function getServiceJob(options) {
  return serviceGet(options, `/api/service/jobs/${encodeURIComponent(options.id)}`);
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceEventsResponse>}
 */
export function getServiceEvents(options) {
  return serviceGet(options, '/api/service/events');
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceIncidentsResponse>}
 */
export function getServiceIncidents(options) {
  return serviceGet(options, '/api/service/incidents');
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceIncidentsResponse>}
 */
export function getServiceIncident(options) {
  return serviceGet(options, `/api/service/incidents/${encodeURIComponent(options.id)}`);
}

/**
 * @param {ServiceIncidentActivityOptions} options
 * @returns {Promise<ServiceIncidentActivityResponse>}
 */
export function getServiceIncidentActivity(options) {
  return serviceGet(
    options,
    `/api/service/incidents/${encodeURIComponent(options.incidentId)}/activity`,
  );
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceTraceResponse>}
 */
export function getServiceTrace(options) {
  return serviceGet(options, '/api/service/trace');
}

/**
 * @template TResult
 * @param {{ baseUrl: string, fetch?: typeof globalThis.fetch, signal?: AbortSignal, query?: Record<string, string | number | boolean | null | undefined> }} options
 * @param {string} pathname
 * @returns {Promise<TResult>}
 */
async function serviceGet({ baseUrl, fetch = globalThis.fetch, signal, query }, pathname) {
  if (typeof fetch !== 'function') {
    throw new TypeError('service observability helpers require a fetch implementation');
  }
  if (typeof baseUrl !== 'string' || baseUrl.length === 0) {
    throw new TypeError('service observability helpers require a baseUrl string');
  }

  const url = new URL(pathname, baseUrl);
  appendQuery(url, query);

  const response = await fetch(url, { method: 'GET', signal });
  if (!response.ok) {
    throw new Error(`agent-browser service read failed: ${response.status}`);
  }

  const payload = await response.json();
  if (!payload?.success) {
    throw new Error(`agent-browser service read failed: ${JSON.stringify(payload)}`);
  }

  return payload.data;
}

/**
 * @template TResult
 * @param {{ baseUrl: string, fetch?: typeof globalThis.fetch, signal?: AbortSignal }} options
 * @param {string} pathname
 * @param {unknown} [body]
 * @param {Record<string, string | number | boolean | null | undefined>} [query]
 * @returns {Promise<TResult>}
 */
async function servicePost({ baseUrl, fetch = globalThis.fetch, signal }, pathname, body = undefined, query = undefined) {
  if (typeof fetch !== 'function') {
    throw new TypeError('service observability helpers require a fetch implementation');
  }
  if (typeof baseUrl !== 'string' || baseUrl.length === 0) {
    throw new TypeError('service observability helpers require a baseUrl string');
  }

  const url = new URL(pathname, baseUrl);
  appendQuery(url, query);

  const response = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal,
  });
  if (!response.ok) {
    throw new Error(`agent-browser service write failed: ${response.status}`);
  }

  const payload = await response.json();
  if (!payload?.success) {
    throw new Error(`agent-browser service write failed: ${JSON.stringify(payload)}`);
  }

  return payload.data;
}

/**
 * @template TResult
 * @param {{ baseUrl: string, fetch?: typeof globalThis.fetch, signal?: AbortSignal }} options
 * @param {string} pathname
 * @returns {Promise<TResult>}
 */
async function serviceDelete({ baseUrl, fetch = globalThis.fetch, signal }, pathname) {
  if (typeof fetch !== 'function') {
    throw new TypeError('service observability helpers require a fetch implementation');
  }
  if (typeof baseUrl !== 'string' || baseUrl.length === 0) {
    throw new TypeError('service observability helpers require a baseUrl string');
  }

  const response = await fetch(new URL(pathname, baseUrl), { method: 'DELETE', signal });
  if (!response.ok) {
    throw new Error(`agent-browser service delete failed: ${response.status}`);
  }

  const payload = await response.json();
  if (!payload?.success) {
    throw new Error(`agent-browser service delete failed: ${JSON.stringify(payload)}`);
  }

  return payload.data;
}

/**
 * @param {URL} url
 * @param {Record<string, string | number | boolean | null | undefined> | undefined} query
 */
function appendQuery(url, query) {
  if (!query) return;

  for (const [key, value] of Object.entries(query)) {
    if (value === null || value === undefined) continue;
    url.searchParams.set(key, String(value));
  }
}

/**
 * @param {unknown} id
 * @param {string} helperName
 * @returns {asserts id is string}
 */
function assertServiceId(id, helperName) {
  if (typeof id !== 'string' || id.length === 0) {
    throw new TypeError(`${helperName} requires an id string`);
  }
}

/**
 * @param {unknown} profile
 * @param {Set<string>} identities
 * @param {string} field
 */
function profileMatchesAny(profile, identities, field) {
  return firstMatchingProfileValue(profile, identities, field) !== null;
}

/**
 * @param {unknown} profile
 * @param {Set<string>} identities
 * @param {string} field
 * @returns {string | null}
 */
function firstMatchingProfileValue(profile, identities, field) {
  if (!profile || typeof profile !== 'object') {
    return null;
  }
  const value = /** @type {Record<string, unknown>} */ (profile)[field];
  if (!Array.isArray(value)) {
    return null;
  }
  const match = value.find((item) => typeof item === 'string' && identities.has(item));
  return typeof match === 'string' ? match : null;
}

/**
 * @param {unknown[]} values
 * @returns {string[]}
 */
function uniqueStrings(values) {
  return [...new Set(values.flatMap((value) => (typeof value === 'string' && value.length > 0 ? [value] : [])))];
}
