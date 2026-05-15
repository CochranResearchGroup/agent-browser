// @ts-check

import { postServiceRequest, requestServiceTab } from './service-request.js';

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
 * @typedef {import('./service-observability.generated.js').ServiceMonitorsResponse} ServiceMonitorsResponse
 * @typedef {import('./service-observability.generated.js').ServiceMonitorDeleteResponse} ServiceMonitorDeleteResponse
 * @typedef {import('./service-observability.generated.js').ServiceMonitorMutationOptions} ServiceMonitorMutationOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileReadinessMonitorOptions} ServiceProfileReadinessMonitorOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileReadinessMonitorRecipeOptions} ServiceProfileReadinessMonitorRecipeOptions
 * @typedef {import('./service-observability.generated.js').ServiceMonitorRunDueResponse} ServiceMonitorRunDueResponse
 * @typedef {import('./service-observability.generated.js').ServiceMonitorStateResponse} ServiceMonitorStateResponse
 * @typedef {import('./service-observability.generated.js').ServiceMonitorTriageOptions} ServiceMonitorTriageOptions
 * @typedef {import('./service-observability.generated.js').ServiceMonitorTriageResponse} ServiceMonitorTriageResponse
 * @typedef {import('./service-observability.generated.js').ServiceMonitorUpsertResponse} ServiceMonitorUpsertResponse
 * @typedef {import('./service-observability.generated.js').ServiceObservabilityHttpOptions} ServiceObservabilityHttpOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfilesResponse} ServiceProfilesResponse
 * @typedef {import('./service-observability.generated.js').ServiceProvidersResponse} ServiceProvidersResponse
 * @typedef {import('./service-observability.generated.js').ServiceBrowserCapabilityRegistryResponse} ServiceBrowserCapabilityRegistryResponse
 * @typedef {import('./service-observability.generated.js').ServiceBrowserCapabilityPreflightOptions} ServiceBrowserCapabilityPreflightOptions
 * @typedef {import('./service-observability.generated.js').ServiceBrowserCapabilityPreflightResponse} ServiceBrowserCapabilityPreflightResponse
 * @typedef {import('./service-observability.generated.js').ServiceBrowserCapabilityRegistryUpsertOptions} ServiceBrowserCapabilityRegistryUpsertOptions
 * @typedef {import('./service-observability.generated.js').ServiceBrowserCapabilityRegistryUpsertResponse} ServiceBrowserCapabilityRegistryUpsertResponse
 * @typedef {import('./service-observability.generated.js').ServiceQueryOptions} ServiceQueryOptions
 * @typedef {import('./service-observability.generated.js').ServiceReconcileResponse} ServiceReconcileResponse
 * @typedef {import('./service-observability.generated.js').ServiceSessionsResponse} ServiceSessionsResponse
 * @typedef {import('./service-observability.generated.js').ServiceSitePoliciesResponse} ServiceSitePoliciesResponse
 * @typedef {import('./service-observability.generated.js').ServiceStatusResponse} ServiceStatusResponse
 * @typedef {import('./service-observability.generated.js').ServiceTabsResponse} ServiceTabsResponse
 * @typedef {import('./service-observability.generated.js').ServiceTraceResponse} ServiceTraceResponse
 * @typedef {import('./service-observability.generated.js').ServiceTraceAttentionSummary} ServiceTraceAttentionSummary
 * @typedef {import('./service-observability.generated.js').ServiceLoginProfileRegistrationOptions} ServiceLoginProfileRegistrationOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileDeleteResponse} ServiceProfileDeleteResponse
 * @typedef {import('./service-observability.generated.js').ServiceProfileAllocationResponse} ServiceProfileAllocationResponse
 * @typedef {import('./service-observability.generated.js').ServiceProfileReadinessResponse} ServiceProfileReadinessResponse
 * @typedef {import('./service-observability.generated.js').ServiceProfileSeedingHandoffOptions} ServiceProfileSeedingHandoffOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileSeedingHandoffResponse} ServiceProfileSeedingHandoffResponse
 * @typedef {import('./service-observability.generated.js').ServiceProfileSeedingHandoffUpdateOptions} ServiceProfileSeedingHandoffUpdateOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileSeedingHandoffUpdateResponse} ServiceProfileSeedingHandoffUpdateResponse
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
 * @typedef {import('./service-observability.generated.js').ServiceAccessPlanBrowserCapabilityPreflightRunOptions} ServiceAccessPlanBrowserCapabilityPreflightRunOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileAcquisitionOptions} ServiceProfileAcquisitionOptions
 * @typedef {import('./service-observability.generated.js').ServiceProfileAcquisitionResult} ServiceProfileAcquisitionResult
 * @typedef {import('./service-observability.generated.js').ServiceProfileFreshnessUpdateOptions} ServiceProfileFreshnessUpdateOptions
 * @typedef {import('./service-observability.generated.js').ServiceAccessPlanPostSeedingProbeRunOptions} ServiceAccessPlanPostSeedingProbeRunOptions
 * @typedef {import('./service-observability.generated.js').ServiceAccessPlanPostSeedingProbeRunResult} ServiceAccessPlanPostSeedingProbeRunResult
 * @typedef {import('./service-observability.generated.js').ServiceAccessPlanMonitorRunDueOptions} ServiceAccessPlanMonitorRunDueOptions
 * @typedef {import('./service-observability.generated.js').ServiceAccessPlanMonitorRunDueSummary} ServiceAccessPlanMonitorRunDueSummary
 * @typedef {import('./service-observability.generated.js').ServiceRemediesApplyOptions} ServiceRemediesApplyOptions
 * @typedef {import('./service-observability.generated.js').ServiceRemediesApplyResponse} ServiceRemediesApplyResponse
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
 * @param {ServiceObservabilityHttpOptions} options
 * @returns {Promise<ServiceBrowserCapabilityRegistryResponse>}
 */
export function getServiceBrowserCapabilityRegistry(options) {
  return serviceGet(options, '/api/service/browser-capability-registry');
}

/**
 * Evaluate browser capability launch gates without starting Chrome.
 *
 * @param {ServiceBrowserCapabilityPreflightOptions} options
 * @returns {Promise<ServiceBrowserCapabilityPreflightResponse>}
 */
export function getServiceBrowserCapabilityPreflight(options) {
  return serviceGet(
    {
      ...options,
      query: {
        ...options.query,
        browserBuild: options.browserBuild,
        serviceName: options.serviceName,
        agentName: options.agentName,
        taskName: options.taskName,
        loginId: options.loginId,
        siteId: options.siteId,
        targetServiceId: options.targetServiceId,
        accountId: options.accountId,
        url: options.url,
        loginIds: options.loginIds?.join(','),
        siteIds: options.siteIds?.join(','),
        targetServiceIds: options.targetServiceIds?.join(','),
        accountIds: options.accountIds?.join(','),
        runtimeProfile: options.runtimeProfile,
        profile: options.profile,
        headless: options.headless,
        headed: options.headed,
        cdpFree: options.cdpFree,
        requiresCdpFree: options.requiresCdpFree,
        cdpAttachmentAllowed: options.cdpAttachmentAllowed,
      },
    },
    '/api/service/browser-capability/preflight',
  );
}

/**
 * Run the browser-capability preflight recipe advertised by an access plan.
 *
 * @param {ServiceAccessPlanBrowserCapabilityPreflightRunOptions} options
 * @returns {Promise<ServiceBrowserCapabilityPreflightResponse>}
 */
export function runServiceAccessPlanBrowserCapabilityPreflight({ accessPlan, ...options }) {
  const recipe = accessPlanBrowserCapabilityPreflight(accessPlan);
  if (recipe.available !== true) {
    throw new Error('access plan browserCapabilityPreflight is not available');
  }
  assertPlainObject(recipe.request, 'service access plan decision.browserCapabilityPreflight.request');
  return getServiceBrowserCapabilityPreflight({
    ...options,
    ...recipe.request,
  });
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
 * @param {ServiceProfileSeedingHandoffOptions} options
 * @returns {Promise<ServiceProfileSeedingHandoffResponse>}
 */
export function getServiceProfileSeedingHandoff({ id, targetServiceId, ...options }) {
  assertServiceId(id, 'getServiceProfileSeedingHandoff');
  return serviceGet(
    {
      ...options,
      query: {
        targetServiceId,
      },
    },
    `/api/service/profiles/${encodeURIComponent(id)}/seeding-handoff`,
  );
}

/**
 * Persist lifecycle state for a CDP-free profile seeding handoff.
 *
 * @param {ServiceProfileSeedingHandoffUpdateOptions} options
 * @returns {Promise<ServiceProfileSeedingHandoffUpdateResponse>}
 */
export function updateServiceProfileSeedingHandoff({
  id,
  targetServiceId,
  state,
  pid,
  startedAt,
  expiresAt,
  lastPromptedAt,
  declaredCompleteAt,
  closedAt,
  actor,
  note,
  ...options
}) {
  assertServiceId(id, 'updateServiceProfileSeedingHandoff');
  return servicePost(options, `/api/service/profiles/${encodeURIComponent(id)}/seeding-handoff`, {
    targetServiceId,
    state,
    pid,
    startedAt,
    expiresAt,
    lastPromptedAt,
    declaredCompleteAt,
    closedAt,
    actor,
    note,
  });
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
    options?.accountId,
    ...(options?.loginIds ?? []),
    ...(options?.siteIds ?? []),
    ...(options?.targetServiceIds ?? []),
    ...(options?.accountIds ?? []),
  ]);
  const identitySet = new Set(identities);
  const serviceName = options?.serviceName;
  const browserBuild = options?.browserBuild;

  const rankedProfile = candidates
    .map((profile, index) => ({
      profile,
      index,
      rank: profileIdentityRank(profile, {
        identitySet,
        serviceName,
        browserBuild,
      }),
    }))
    .filter((candidate) => candidate.rank.reason !== null)
    .sort((left, right) => compareProfileIdentityRank(right.rank, left.rank) || left.index - right.index)[0];

  if (rankedProfile) {
    return /** @type {ServiceProfileIdentityMatchResult} */ ({
      profile: rankedProfile.profile,
      reason: rankedProfile.rank.reason,
      matchedField: rankedProfile.rank.matchedField,
      matchedIdentity: rankedProfile.rank.matchedIdentity,
    });
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
        accountId: options.accountId,
        browserBuild: options.browserBuild,
        url: options.url,
        loginIds: options.loginIds?.join(','),
        siteIds: options.siteIds?.join(','),
        targetServiceIds: options.targetServiceIds?.join(','),
        accountIds: options.accountIds?.join(','),
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
        accountId: options.accountId,
        browserBuild: options.browserBuild,
        url: options.url,
        loginIds: options.loginIds?.join(','),
        siteIds: options.siteIds?.join(','),
        targetServiceIds: options.targetServiceIds?.join(','),
        accountIds: options.accountIds?.join(','),
        readinessProfileId,
        sitePolicyId,
        challengeId,
      },
    },
    '/api/service/access-plan',
  );
}

/**
 * Acquire a managed login profile recommendation through the service broker.
 *
 * The helper asks for an access plan first, registers the fallback profile only
 * when agent-browser has no selected profile, optionally installs the standard
 * profile-readiness monitor, optionally runs due monitors recommended by the
 * plan, optionally runs the browser-capability preflight advertised by the
 * final plan, then returns the broker-owned recommendation callers can pass to
 * requestServiceTab().
 *
 * @param {ServiceProfileAcquisitionOptions} options
 * @returns {Promise<ServiceProfileAcquisitionResult>}
 */
export async function acquireServiceLoginProfile({
  registerProfileId,
  profileUserDataDir,
  registerAuthenticated,
  registerReadinessMonitor = false,
  runDueReadinessMonitor = false,
  runBrowserCapabilityPreflight = false,
  readinessMonitorId,
  readinessMonitorIntervalMs,
  profileName,
  profile,
  ...accessPlanOptions
}) {
  const initialAccessPlan = await getServiceAccessPlan(accessPlanOptions);
  const profileRegistration =
    !initialAccessPlan.selectedProfile && registerProfileId
      ? await registerServiceLoginProfile({
          ...accessPlanOptions,
          id: registerProfileId,
          serviceName: accessPlanOptions.serviceName,
          loginId: accessPlanOptions.loginId,
          siteId: accessPlanOptions.siteId,
          targetServiceId: accessPlanOptions.targetServiceId,
          targetServiceIds: accessPlanOptions.targetServiceIds,
          userDataDir: profileUserDataDir,
          ...(registerAuthenticated === undefined ? {} : { authenticated: registerAuthenticated }),
          ...(profileName === undefined ? {} : { name: profileName }),
          ...(profile === undefined ? {} : { profile }),
        })
      : null;
  const profileReadinessMonitor =
    profileRegistration && registerReadinessMonitor
      ? await upsertServiceProfileReadinessMonitor({
          ...accessPlanOptions,
          id: readinessMonitorId,
          serviceName: accessPlanOptions.serviceName,
          loginId: accessPlanOptions.loginId,
          siteId: accessPlanOptions.siteId,
          targetServiceId: accessPlanOptions.targetServiceId,
          targetServiceIds: accessPlanOptions.targetServiceIds,
          intervalMs: readinessMonitorIntervalMs,
        })
      : null;
  let accessPlan = profileRegistration ? await getServiceAccessPlan(accessPlanOptions) : initialAccessPlan;
  const monitorRunDue =
    runDueReadinessMonitor && accessPlan?.decision?.monitorRunDue?.recommendedBeforeUse === true
      ? await runServiceAccessPlanMonitorRunDue({
          ...accessPlanOptions,
          accessPlan,
        })
      : null;
  if (monitorRunDue) {
    accessPlan = await getServiceAccessPlan(accessPlanOptions);
  }
  const monitorRunDueSummary = monitorRunDue
    ? summarizeServiceAccessPlanMonitorRunDue({
        accessPlan,
        monitorRunDue,
      })
    : null;
  const browserCapabilityPreflight =
    runBrowserCapabilityPreflight && accessPlan?.decision?.browserCapabilityPreflight?.available === true
      ? await runServiceAccessPlanBrowserCapabilityPreflight({
          baseUrl: accessPlanOptions.baseUrl,
          fetch: accessPlanOptions.fetch,
          signal: accessPlanOptions.signal,
          accessPlan,
        })
      : null;

  return {
    initialAccessPlan,
    accessPlan,
    selectedProfile: accessPlan.selectedProfile ?? null,
    profileRegistration,
    profileReadinessMonitor,
    monitorRunDue,
    monitorRunDueSummary,
    browserCapabilityPreflight,
    registered: profileRegistration !== null,
    monitorRegistered: profileReadinessMonitor !== null,
    monitorRunDueRan: monitorRunDue !== null,
    browserCapabilityPreflightRan: browserCapabilityPreflight !== null,
  };
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
 * @param {import('./service-observability.generated.js').ServiceMonitorQueryOptions} options
 * @returns {Promise<ServiceMonitorsResponse>}
 */
export function getServiceMonitors(options) {
  return serviceGet(
    {
      ...options,
      query: {
        ...options.query,
        state: options.state,
        failed: options.failedOnly,
        summary: options.summary,
      },
    },
    '/api/service/monitors',
  );
}

/**
 * Ask the service to run due active monitor checks immediately.
 *
 * @param {ServiceObservabilityHttpOptions} options
 * @returns {Promise<ServiceMonitorRunDueResponse>}
 */
export function runDueServiceMonitors(options) {
  return servicePost(options, '/api/service/monitors/run-due');
}

/**
 * Summarize monitor run results for the target identities advertised by an access plan.
 *
 * @param {{ accessPlan: ServiceAccessPlanResponse, monitorRunDue: ServiceMonitorRunDueResponse }} options
 * @returns {ServiceAccessPlanMonitorRunDueSummary}
 */
export function summarizeServiceAccessPlanMonitorRunDue({ accessPlan, monitorRunDue }) {
  const recipe = accessPlanMonitorRunDue(accessPlan);
  const targetServiceIds = uniqueStrings([
    ...(Array.isArray(recipe.targetServiceIds) ? recipe.targetServiceIds : []),
    ...(Array.isArray(accessPlan?.query?.targetServiceIds) ? accessPlan.query.targetServiceIds : []),
  ]);
  const targetSet = new Set(targetServiceIds);
  const recipeMonitorIds = new Set(Array.isArray(recipe.monitorIds) ? recipe.monitorIds : []);
  const results = Array.isArray(monitorRunDue?.results) ? monitorRunDue.results : [];
  const matchingResults = results.filter((result) => {
    const targetServiceId = profileReadinessTargetServiceId(result?.target);
    return (
      (targetServiceId !== null && (targetSet.size === 0 || targetSet.has(targetServiceId))) ||
      recipeMonitorIds.has(String(result?.monitorId ?? ''))
    );
  });
  const resultStates = uniqueStrings(matchingResults.map((result) => result?.result));
  const staleProfileIds = uniqueStrings(
    matchingResults.flatMap((result) => (Array.isArray(result?.staleProfileIds) ? result.staleProfileIds : [])),
  );
  const freshTargetServiceIds = targetServiceIdsWithResult(matchingResults, 'profile_readiness_fresh');
  const expiredTargetServiceIds = targetServiceIdsWithResult(matchingResults, 'profile_readiness_expired');
  const unverifiedTargetServiceIds = targetServiceIdsWithAnyResult(matchingResults, [
    'profile_readiness_not_fresh',
    'profile_readiness_missing',
  ]);
  const failed = matchingResults.some((result) => result?.success === false);
  const succeeded = matchingResults.length > 0 && matchingResults.every((result) => result?.success === true);
  const recommendedAction = monitorRunDueRecommendedAction({
    matchingResults,
    failed,
    succeeded,
    expiredTargetServiceIds,
    unverifiedTargetServiceIds,
  });

  return {
    targetServiceIds,
    matched: matchingResults.length,
    monitorIds: uniqueStrings(matchingResults.map((result) => result?.monitorId)),
    resultStates,
    staleProfileIds,
    freshTargetServiceIds,
    expiredTargetServiceIds,
    unverifiedTargetServiceIds,
    succeeded,
    failed,
    recommendedAction,
    matchingResults,
  };
}

/**
 * Summarize UI-neutral trace-context attention rows for software clients.
 *
 * @param {ServiceTraceResponse | null | undefined} trace
 * @returns {ServiceTraceAttentionSummary}
 */
export function summarizeServiceTraceAttention(trace) {
  const contexts = Array.isArray(trace?.summary?.contexts) ? trace.summary.contexts : [];
  const attentionContexts = contexts
    .map((context) => {
      const attention = context?.attention;
      if (!attention || typeof attention !== 'object' || Array.isArray(attention)) {
        return null;
      }
      return {
        serviceName: context.serviceName ?? null,
        agentName: context.agentName ?? null,
        taskName: context.taskName ?? null,
        browserId: context.browserId ?? null,
        profileId: context.profileId ?? null,
        sessionId: context.sessionId ?? null,
        eventCount: context.eventCount ?? 0,
        jobCount: context.jobCount ?? 0,
        incidentCount: context.incidentCount ?? 0,
        activityCount: context.activityCount ?? 0,
        attention,
      };
    })
    .filter((context) => context?.attention?.required === true);
  const ownerCounts = attentionContexts.reduce(
    (counts, context) => {
      if (context.attention.owner === 'operator') {
        counts.operator += 1;
      } else if (context.attention.owner === 'service') {
        counts.service += 1;
      }
      return counts;
    },
    { operator: 0, service: 0 },
  );

  return {
    required: attentionContexts.length > 0,
    requiredCount: attentionContexts.length,
    operatorRequiredCount: ownerCounts.operator,
    serviceRequiredCount: ownerCounts.service,
    requiresOperator: ownerCounts.operator > 0,
    requiresService: ownerCounts.service > 0,
    highestSeverity: highestTraceAttentionSeverity(attentionContexts),
    reasons: uniqueStrings(attentionContexts.map((context) => context.attention.reason)),
    suggestedActions: uniqueStrings(
      attentionContexts.flatMap((context) =>
        Array.isArray(context.attention.suggestedActions) ? context.attention.suggestedActions : [],
      ),
    ),
    messages: uniqueStrings(attentionContexts.map((context) => context.attention.message)),
    contexts: attentionContexts,
  };
}

/**
 * Run the due-monitor recipe advertised by an access plan.
 *
 * @param {ServiceAccessPlanMonitorRunDueOptions} options
 * @returns {Promise<ServiceMonitorRunDueResponse>}
 */
export function runServiceAccessPlanMonitorRunDue({ accessPlan, ...options }) {
  const recipe = accessPlanMonitorRunDue(accessPlan);
  if (recipe.available !== true) {
    throw new Error('access plan monitorRunDue is not available');
  }
  return runDueServiceMonitors(options).then((monitorRunDue) => ({
    ...monitorRunDue,
    accessPlanSummary: summarizeServiceAccessPlanMonitorRunDue({ accessPlan, monitorRunDue }),
  }));
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
  browserBuild,
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
      ...(browserBuild === undefined ? {} : { browserBuild }),
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
 * Build the standard no-launch profile-readiness monitor record for one target identity.
 *
 * @param {ServiceProfileReadinessMonitorRecipeOptions} options
 * @returns {{ id: string, monitor: Record<string, unknown> }}
 */
export function createServiceProfileReadinessMonitor({
  id,
  serviceName,
  loginId,
  siteId,
  targetServiceId,
  targetServiceIds = [],
  name,
  intervalMs = 3600000,
  state = 'active',
  monitor = {},
}) {
  const targetId = loginId ?? siteId ?? targetServiceId ?? targetServiceIds[0];
  if (typeof targetId !== 'string' || targetId.length === 0) {
    throw new TypeError(
      'createServiceProfileReadinessMonitor requires loginId, siteId, targetServiceId, or targetServiceIds',
    );
  }
  const monitorId = id ?? serviceProfileReadinessMonitorId(serviceName, targetId);
  return {
    id: monitorId,
    monitor: {
      name: name ?? serviceProfileReadinessMonitorName(serviceName, targetId),
      target: { profile_readiness: targetId },
      intervalMs,
      state,
      ...monitor,
    },
  };
}

/**
 * Upsert the standard no-launch profile-readiness monitor for one target identity.
 *
 * @param {ServiceProfileReadinessMonitorOptions} options
 * @returns {Promise<ServiceMonitorUpsertResponse>}
 */
export function upsertServiceProfileReadinessMonitor(options) {
  const { id, monitor } = createServiceProfileReadinessMonitor(options);
  return upsertServiceMonitor({ ...options, id, monitor });
}

/**
 * Ask the service to merge bounded-probe freshness evidence into a profile.
 *
 * @param {ServiceProfileFreshnessUpdateOptions} options
 * @returns {Promise<ServiceProfileUpsertResponse>}
 */
export async function updateServiceProfileFreshness({
  id,
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
  const targetId = loginId ?? siteId ?? targetServiceId;
  const targets = uniqueStrings([...targetServiceIds, targetId]);
  if (targets.length === 0) {
    throw new TypeError(
      'updateServiceProfileFreshness requires loginId, siteId, targetServiceId, or targetServiceIds',
    );
  }
  return servicePost(options, `/api/service/profiles/${encodeURIComponent(id)}/freshness`, {
    loginId,
    siteId,
    targetServiceId,
    targetServiceIds: targets,
    readinessState,
    readinessEvidence,
    readinessRecommendedAction,
    lastVerifiedAt,
    freshnessExpiresAt,
    updateAuthenticatedServiceIds,
  });
}

/**
 * Record the result of a bounded post-close seeding auth probe.
 *
 * @param {ServiceProfileFreshnessUpdateOptions} options
 * @returns {Promise<ServiceProfileUpsertResponse>}
 */
export function verifyServiceProfileSeeding({
  readinessState = 'fresh',
  readinessEvidence,
  ...options
}) {
  return updateServiceProfileFreshness({
    ...options,
    readinessState,
    readinessEvidence: readinessEvidence ?? `post_seeding_auth_probe_${readinessState}`,
  });
}

/**
 * Run the bounded post-close seeding verification recipe advertised by an
 * access plan.
 *
 * @param {ServiceAccessPlanPostSeedingProbeRunOptions} options
 * @returns {Promise<ServiceAccessPlanPostSeedingProbeRunResult>}
 */
export async function runServiceAccessPlanPostSeedingProbe({
  accessPlan,
  baseUrl,
  fetch = globalThis.fetch,
  signal,
  url,
  expectedUrlIncludes,
  expectedTitleIncludes,
  freshnessExpiresAt,
  readinessEvidence,
  jobTimeoutMs = 30000,
}) {
  const recipe = accessPlanPostSeedingProbe(accessPlan);
  if (recipe.available !== true) {
    throw new Error('access plan postSeedingProbe is not available');
  }
  const profileId = recipe.profileId;
  const targetServiceId = recipe.targetServiceId ?? recipe.targetServiceIds?.[0];
  if (typeof profileId !== 'string' || profileId.length === 0) {
    throw new Error('access plan postSeedingProbe is missing profileId');
  }
  if (typeof targetServiceId !== 'string' || targetServiceId.length === 0) {
    throw new Error('access plan postSeedingProbe is missing targetServiceId');
  }

  const serviceName = stringOrUndefined(
    nestedString(accessPlan, ['query', 'serviceName']) ?? nestedString(recipe, ['query', 'serviceName']),
  );
  const agentName = stringOrUndefined(
    nestedString(accessPlan, ['query', 'agentName']) ?? nestedString(recipe, ['query', 'agentName']),
  );
  const taskName = stringOrUndefined(
    nestedString(accessPlan, ['query', 'taskName']) ?? nestedString(recipe, ['query', 'taskName']),
  );
  const probeUrl = url ?? accessPlanProbeUrl(accessPlan);
  if (!probeUrl) {
    throw new Error('runServiceAccessPlanPostSeedingProbe requires url or an access-plan site policy URL');
  }
  const requestContext = optionalRequestContext({
    serviceName,
    agentName,
    taskName,
  });

  const lookup = await lookupServiceProfile({
    baseUrl,
    fetch,
    signal,
    serviceName,
    targetServiceId,
    readinessProfileId: profileId,
  });
  const selectedProfileId = lookup?.selectedProfile?.id;
  if (selectedProfileId !== profileId) {
    throw new Error(
      `Post-seeding probe refused to verify ${profileId}: broker selected ${selectedProfileId || 'no profile'}.`,
    );
  }

  const tab = await requestServiceTab({
    baseUrl,
    fetch,
    signal,
    ...requestContext,
    targetServiceId,
    loginId: targetServiceId,
    url: probeUrl,
    jobTimeoutMs,
  });
  const urlResult = await postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: {
      ...requestContext,
      targetServiceId,
      loginId: targetServiceId,
      action: 'url',
      jobTimeoutMs,
    },
  });
  const titleResult = await postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: {
      ...requestContext,
      targetServiceId,
      loginId: targetServiceId,
      action: 'title',
      jobTimeoutMs,
    },
  });

  const observedUrl = stringData(urlResult.data, 'url');
  const observedTitle = stringData(titleResult.data, 'title');
  const checks = evaluatePostSeedingProbeChecks({
    observedUrl,
    observedTitle,
    expectedUrlIncludes,
    expectedTitleIncludes,
  });
  const readinessState = checks.fresh ? 'fresh' : 'stale';
  const evidence =
    readinessEvidence ??
    (checks.fresh
      ? `post_seeding_auth_probe_passed:${checks.passed.join(',') || 'service_tab_opened'}`
      : `post_seeding_auth_probe_failed:${checks.failed.join(',') || 'bounded_probe_failed'}`);
  const freshness = await verifyServiceProfileSeeding({
    baseUrl,
    fetch,
    signal,
    id: profileId,
    targetServiceId,
    loginId: targetServiceId,
    readinessState,
    readinessEvidence: evidence,
    lastVerifiedAt: new Date().toISOString(),
    freshnessExpiresAt,
  });

  return {
    recipe,
    lookup,
    tab,
    observed: {
      url: observedUrl,
      title: observedTitle,
    },
    checks,
    freshness,
  };
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
    seedingMode: state === 'needs_manual_seeding' ? 'detached_headed_no_cdp' : 'not_required',
    cdpAttachmentAllowedDuringSeeding: false,
    preferredKeyring: state === 'needs_manual_seeding' ? 'basic_password_store' : null,
    setupScopes: state === 'needs_manual_seeding' ? serviceLoginProfileSeedingSetupScopes(targetServiceId) : [],
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

function serviceLoginProfileSeedingSetupScopes(targetServiceId) {
  const normalized = typeof targetServiceId === 'string' ? targetServiceId.toLowerCase() : '';
  if (['google', 'gmail', 'google-login', 'google_signin', 'google-signin'].includes(normalized)) {
    return ['signin', 'chrome_sync', 'passkeys', 'browser_plugins'];
  }
  return ['signin'];
}

function accessPlanPostSeedingProbe(accessPlan) {
  assertPlainObject(accessPlan, 'service access plan');
  const decision = /** @type {Record<string, unknown>} */ (accessPlan).decision;
  assertPlainObject(decision, 'service access plan decision');
  const recipe = /** @type {Record<string, unknown>} */ (decision).postSeedingProbe;
  assertPlainObject(recipe, 'service access plan decision.postSeedingProbe');
  return /** @type {import('./service-observability.generated.js').ServiceAccessPlanPostSeedingProbe} */ (recipe);
}

function accessPlanMonitorRunDue(accessPlan) {
  assertPlainObject(accessPlan, 'service access plan');
  const decision = /** @type {Record<string, unknown>} */ (accessPlan).decision;
  assertPlainObject(decision, 'service access plan decision');
  const recipe = /** @type {Record<string, unknown>} */ (decision).monitorRunDue;
  assertPlainObject(recipe, 'service access plan decision.monitorRunDue');
  return /** @type {import('./service-observability.generated.js').ServiceAccessPlanMonitorRunDue} */ (recipe);
}

function accessPlanBrowserCapabilityPreflight(accessPlan) {
  assertPlainObject(accessPlan, 'service access plan');
  const decision = /** @type {Record<string, unknown>} */ (accessPlan).decision;
  assertPlainObject(decision, 'service access plan decision');
  const recipe = /** @type {Record<string, unknown>} */ (decision).browserCapabilityPreflight;
  assertPlainObject(recipe, 'service access plan decision.browserCapabilityPreflight');
  return /** @type {import('./service-observability.generated.js').ServiceAccessPlanBrowserCapabilityPreflight} */ (recipe);
}

function profileReadinessTargetServiceId(target) {
  if (!target || typeof target !== 'object') {
    return null;
  }
  const value = /** @type {Record<string, unknown>} */ (target).profile_readiness;
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function targetServiceIdsWithResult(results, resultState) {
  return targetServiceIdsWithAnyResult(results, [resultState]);
}

function targetServiceIdsWithAnyResult(results, resultStates) {
  const resultStateSet = new Set(resultStates);
  return uniqueStrings(
    results
      .filter((result) => resultStateSet.has(String(result?.result ?? '')))
      .map((result) => profileReadinessTargetServiceId(result?.target)),
  );
}

function monitorRunDueRecommendedAction({
  matchingResults,
  failed,
  succeeded,
  expiredTargetServiceIds,
  unverifiedTargetServiceIds,
}) {
  if (matchingResults.length === 0) {
    return 'inspect_monitor_results';
  }
  if (expiredTargetServiceIds.length > 0) {
    return 'probe_target_auth_or_reseed_if_needed';
  }
  if (unverifiedTargetServiceIds.length > 0) {
    return 'verify_or_seed_profile_before_authenticated_work';
  }
  if (succeeded) {
    return 'use_selected_profile';
  }
  if (failed) {
    return 'inspect_monitor_results';
  }
  return 'use_selected_profile';
}

function accessPlanProbeUrl(accessPlan) {
  const explicitUrl = nestedString(accessPlan, ['decision', 'postSeedingProbe', 'url']);
  if (isHttpUrl(explicitUrl)) {
    return explicitUrl;
  }
  const sitePolicyUrl = nestedString(accessPlan, ['sitePolicy', 'originPattern']);
  if (isHttpUrl(sitePolicyUrl)) {
    return sitePolicyUrl;
  }
  const handoffUrl = nestedString(accessPlan, ['seedingHandoff', 'url']);
  if (isHttpUrl(handoffUrl)) {
    return handoffUrl;
  }
  return undefined;
}

function isHttpUrl(value) {
  return typeof value === 'string' && (value.startsWith('http://') || value.startsWith('https://'));
}

function nestedString(value, path) {
  let current = value;
  for (const segment of path) {
    if (!current || typeof current !== 'object' || Array.isArray(current)) {
      return undefined;
    }
    current = /** @type {Record<string, unknown>} */ (current)[segment];
  }
  return typeof current === 'string' ? current : undefined;
}

function stringOrUndefined(value) {
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

function optionalRequestContext({ serviceName, agentName, taskName }) {
  return {
    ...(serviceName ? { serviceName } : {}),
    ...(agentName ? { agentName } : {}),
    ...(taskName ? { taskName } : {}),
  };
}

function evaluatePostSeedingProbeChecks({ observedUrl, observedTitle, expectedUrlIncludes, expectedTitleIncludes }) {
  const passed = [];
  const failed = [];
  if (observedUrl) {
    passed.push('url_read');
  } else {
    failed.push('url_missing');
  }
  if (observedTitle) {
    passed.push('title_read');
  } else {
    failed.push('title_missing');
  }
  if (expectedUrlIncludes) {
    if (observedUrl.includes(expectedUrlIncludes)) {
      passed.push('expected_url');
    } else {
      failed.push('expected_url');
    }
  }
  if (expectedTitleIncludes) {
    if (observedTitle.includes(expectedTitleIncludes)) {
      passed.push('expected_title');
    } else {
      failed.push('expected_title');
    }
  }
  return {
    fresh: failed.length === 0,
    passed,
    failed,
  };
}

function stringData(data, field) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    return '';
  }
  const value = /** @type {Record<string, unknown>} */ (data)[field];
  return typeof value === 'string' ? value : '';
}

function serviceProfileReadinessMonitorId(serviceName, targetId) {
  const prefix = typeof serviceName === 'string' && serviceName.length > 0 ? serviceName : 'service';
  return `${serviceIdSlug(prefix)}-${serviceIdSlug(targetId)}-profile-readiness`;
}

function serviceProfileReadinessMonitorName(serviceName, targetId) {
  const prefix = typeof serviceName === 'string' && serviceName.length > 0 ? serviceName : 'Service';
  return `${prefix} ${targetId} profile readiness`;
}

function serviceIdSlug(value) {
  return String(value)
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'target';
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
 * @param {ServiceMonitorMutationOptions} options
 * @returns {Promise<ServiceMonitorUpsertResponse>}
 */
export function upsertServiceMonitor({ id, monitor, ...options }) {
  return servicePost(options, `/api/service/monitors/${encodeURIComponent(id)}`, monitor);
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceMonitorDeleteResponse>}
 */
export function deleteServiceMonitor({ id, ...options }) {
  return serviceDelete(options, `/api/service/monitors/${encodeURIComponent(id)}`);
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceMonitorStateResponse>}
 */
export function pauseServiceMonitor({ id, ...options }) {
  assertServiceId(id, 'pauseServiceMonitor');
  return servicePost(options, `/api/service/monitors/${encodeURIComponent(id)}/pause`);
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceMonitorStateResponse>}
 */
export function resumeServiceMonitor({ id, ...options }) {
  assertServiceId(id, 'resumeServiceMonitor');
  return servicePost(options, `/api/service/monitors/${encodeURIComponent(id)}/resume`);
}

/**
 * @param {ServiceIdOptions} options
 * @returns {Promise<ServiceMonitorStateResponse>}
 */
export function resetServiceMonitorFailures({ id, ...options }) {
  assertServiceId(id, 'resetServiceMonitorFailures');
  return servicePost(options, `/api/service/monitors/${encodeURIComponent(id)}/reset-failures`);
}

/**
 * @param {ServiceMonitorTriageOptions} options
 * @returns {Promise<ServiceMonitorTriageResponse>}
 */
export function triageServiceMonitor({ id, by, note, serviceName, agentName, taskName, ...options }) {
  assertServiceId(id, 'triageServiceMonitor');
  return servicePost(options, `/api/service/monitors/${encodeURIComponent(id)}/triage`, undefined, {
    by,
    note,
    'service-name': serviceName,
    'agent-name': agentName,
    'task-name': taskName,
  });
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
 * @param {ServiceBrowserCapabilityRegistryUpsertOptions} options
 * @returns {Promise<ServiceBrowserCapabilityRegistryUpsertResponse>}
 */
export function upsertServiceBrowserCapabilityRegistryRecord({ collection, id, record, ...options }) {
  return servicePost(
    options,
    `/api/service/browser-capability-registry/${encodeURIComponent(collection)}/${encodeURIComponent(id)}`,
    record,
  );
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
 * @param {ServiceRemediesApplyOptions} options
 * @returns {Promise<ServiceRemediesApplyResponse>}
 */
export function applyServiceRemedies({ escalation = 'monitor_attention', by, note, serviceName, agentName, taskName, ...options }) {
  return servicePost(options, '/api/service/remedies/apply', undefined, {
    escalation,
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

function assertPlainObject(value, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
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
 * @param {{ identitySet: Set<string>, serviceName?: string, browserBuild?: string }} request
 */
function profileIdentityRank(profile, request) {
  const authenticatedMatch = firstMatchingProfileValue(profile, request.identitySet, 'authenticatedServiceIds');
  const accountMatch = firstMatchingProfileValue(profile, request.identitySet, 'accountIds');
  const targetMatch = firstMatchingProfileValue(profile, request.identitySet, 'targetServiceIds');
  const serviceMatch =
    typeof request.serviceName === 'string' && request.serviceName.length > 0
      ? firstMatchingProfileValue(profile, new Set([request.serviceName]), 'sharedServiceIds')
      : null;
  const browserBuildMatch =
    typeof request.browserBuild === 'string' &&
    request.browserBuild.length > 0 &&
    profile !== null &&
    typeof profile === 'object' &&
    /** @type {Record<string, unknown>} */ (profile).browserBuild === request.browserBuild;
  const browserBuildDefault =
    browserBuildMatch &&
    profileArrayFieldLength(profile, 'targetServiceIds') === 0 &&
    profileArrayFieldLength(profile, 'authenticatedServiceIds') === 0 &&
    profileArrayFieldLength(profile, 'accountIds') === 0 &&
    profileArrayFieldLength(profile, 'sitePolicyIds') === 0;
  const persistent =
    profile !== null && typeof profile === 'object' && /** @type {Record<string, unknown>} */ (profile).persistent === true;

  return {
    authenticated: authenticatedMatch === null ? 0 : 1,
    account: accountMatch === null ? 0 : 1,
    target: targetMatch === null ? 0 : 1,
    service: serviceMatch === null ? 0 : 1,
    browserBuild: browserBuildMatch ? 1 : 0,
    browserBuildDefault: browserBuildDefault ? 1 : 0,
    persistent: persistent ? 1 : 0,
    reason:
      authenticatedMatch !== null
        ? 'authenticated_target'
        : accountMatch !== null
          ? 'account_match'
          : targetMatch !== null
            ? 'target_match'
            : serviceMatch !== null
              ? 'service_allow_list'
              : browserBuildDefault
                ? 'browser_build_default'
                : null,
    matchedField:
      authenticatedMatch !== null
        ? 'authenticatedServiceIds'
        : accountMatch !== null
          ? 'accountIds'
          : targetMatch !== null
            ? 'targetServiceIds'
            : serviceMatch !== null
              ? 'sharedServiceIds'
              : browserBuildDefault
                ? 'browserBuild'
                : null,
    matchedIdentity: authenticatedMatch ?? accountMatch ?? targetMatch ?? serviceMatch ?? request.browserBuild ?? null,
  };
}

function compareProfileIdentityRank(left, right) {
  for (const field of ['authenticated', 'account', 'target', 'service', 'browserBuild', 'browserBuildDefault', 'persistent']) {
    const diff = left[field] - right[field];
    if (diff !== 0) {
      return diff;
    }
  }
  return 0;
}

function profileArrayFieldLength(profile, field) {
  if (!profile || typeof profile !== 'object') {
    return 0;
  }
  const value = /** @type {Record<string, unknown>} */ (profile)[field];
  return Array.isArray(value) ? value.length : 0;
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

/**
 * @param {Array<{ attention: { severity?: unknown } }>} contexts
 * @returns {'none' | 'info' | 'warning' | string}
 */
function highestTraceAttentionSeverity(contexts) {
  const severities = contexts.map((context) => context.attention.severity);
  if (severities.includes('warning')) {
    return 'warning';
  }
  if (severities.includes('info')) {
    return 'info';
  }
  return 'none';
}
