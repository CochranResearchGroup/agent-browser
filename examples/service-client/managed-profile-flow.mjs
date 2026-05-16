#!/usr/bin/env node
// @ts-check

import {
  requestServiceCdpFreeLaunch,
  requestServiceTab,
} from '@agent-browser/client/service-request';
import {
  getServiceAccessPlan,
  registerServiceLoginProfile,
  runServiceAccessPlanBrowserCapabilityPreflight,
  runServiceAccessPlanMonitorRunDue,
  summarizeServiceProfileAcquisition,
  updateServiceProfileFreshness,
  upsertServiceProfileReadinessMonitor,
} from '@agent-browser/client/service-observability';

const DEFAULT_URL = 'https://www.canva.com/';
const nodeProcess = /** @type {{ argv: string[], env: Record<string, string | undefined>, exit(code?: number): never }} */ (
  /** @type {any} */ (globalThis).process
);

/**
 * @typedef {import('@agent-browser/client/service-observability').ServiceProfileTargetReadiness['state']} ServiceProfileReadinessState
 */

/**
 * @typedef {{
 *   baseUrl?: string;
 *   url?: string;
 *   serviceName?: string;
 *   agentName?: string;
 *   taskName?: string;
 *   loginId?: string;
 *   targetServiceId?: string;
 *   readinessProfileId?: string;
 *   registerProfileId?: string;
 *   profileUserDataDir?: string;
 *   registerAuthenticated?: boolean;
 *   registerReadinessMonitor?: boolean;
 *   runDueReadinessMonitor?: boolean;
 *   runBrowserCapabilityPreflight?: boolean;
 *   readinessMonitorId?: string;
 *   readinessMonitorIntervalMs?: number;
 *   freshnessProfileId?: string;
 *   freshnessReadinessState?: ServiceProfileReadinessState;
 *   freshnessEvidence?: string;
 *   freshnessLastVerifiedAt?: string;
 *   freshnessExpiresAt?: string;
 *   freshnessUpdateAuthenticatedServiceIds?: boolean;
 *   fetch?: typeof globalThis.fetch;
 *   dryRun?: boolean;
 * }} ManagedProfileOptions
 */

/**
 * @param {ManagedProfileOptions} options
 */
export function buildManagedProfilePlan({
  url = DEFAULT_URL,
  serviceName = 'CanvaCLI',
  agentName = 'canva-cli-agent',
  taskName = 'openCanvaWorkspace',
  loginId = 'canva',
  targetServiceId = loginId,
  readinessProfileId,
  registerProfileId,
  profileUserDataDir,
  registerAuthenticated = false,
  registerReadinessMonitor = false,
  runDueReadinessMonitor = false,
  runBrowserCapabilityPreflight = false,
  readinessMonitorId,
  readinessMonitorIntervalMs,
  freshnessProfileId,
  freshnessReadinessState = 'fresh',
  freshnessEvidence,
  freshnessLastVerifiedAt,
  freshnessExpiresAt,
  freshnessUpdateAuthenticatedServiceIds = true,
} = {}) {
  return {
    serviceName,
    agentName,
    taskName,
    requestedIdentity: loginId || targetServiceId,
    targetServiceId,
    url,
    decisionOrder: [
      'ask agent-browser for the no-launch access plan',
      'inspect the service-owned profile, readiness, policy, provider, challenge, and decision fields',
      'inspect decision.attention before choosing a client prompt, log, or popup',
      'register a managed profile only when agent-browser has no suitable one',
      'optionally run due profile-readiness monitors when access-plan recommends it',
      'refresh the access plan before requesting a tab or CDP-free launch',
      'optionally run the browser-capability preflight before browser work',
      'inspect decision.serviceRequest.cdpFreeAvailability before CDP-free launch work',
      'seed the profile manually when readiness reports needs_manual_seeding',
    ],
    profileInspection: {
      helper: 'getServiceAccessPlan',
      includes: [
        'selectedProfile',
        'readinessSummary',
        'sitePolicy',
        'providers',
        'challenges',
        'decision',
        'decision.attention',
      ],
    },
    readinessInspection: readinessProfileId
      ? {
          helper: 'getServiceProfileReadiness',
          id: readinessProfileId,
        }
      : null,
    tabRequest: {
      helper: 'requestServiceTab or requestServiceCdpFreeLaunch',
      accessPlan: 'getServiceAccessPlan response',
      overrides: ['url', 'jobTimeoutMs'],
      url,
    },
    optionalRegistration: registerProfileId
      ? buildLoginProfileRegistration({
          id: registerProfileId,
          serviceName,
          loginId,
          targetServiceId,
          userDataDir: profileUserDataDir,
          authenticated: registerAuthenticated,
        })
      : null,
    optionalReadinessMonitor:
      registerProfileId && registerReadinessMonitor
        ? buildProfileReadinessMonitor({
            id: readinessMonitorId,
            serviceName,
            loginId,
            targetServiceId,
            intervalMs: readinessMonitorIntervalMs,
          })
        : null,
    optionalMonitorRunDue: runDueReadinessMonitor
      ? {
          helper: 'runServiceAccessPlanMonitorRunDue',
          when: 'decision.monitorRunDue.recommendedBeforeUse',
          refreshesAccessPlan: true,
        }
      : null,
    optionalBrowserCapabilityPreflight: runBrowserCapabilityPreflight
      ? {
          helper: 'runServiceAccessPlanBrowserCapabilityPreflight',
          when: 'decision.browserCapabilityPreflight.available',
          launchesBrowser: false,
        }
      : null,
    optionalFreshnessUpdate: freshnessProfileId
      ? buildProfileFreshnessUpdate({
          id: freshnessProfileId,
          loginId,
          targetServiceId,
          readinessState: freshnessReadinessState,
          readinessEvidence: freshnessEvidence,
          lastVerifiedAt: freshnessLastVerifiedAt,
          freshnessExpiresAt,
          updateAuthenticatedServiceIds: freshnessUpdateAuthenticatedServiceIds,
        })
      : null,
  };
}

/**
 * @param {ManagedProfileOptions} options
 */
export async function runManagedProfileWorkflow({
  baseUrl,
  url = DEFAULT_URL,
  serviceName = 'CanvaCLI',
  agentName = 'canva-cli-agent',
  taskName = 'openCanvaWorkspace',
  loginId = 'canva',
  targetServiceId = loginId,
  readinessProfileId,
  registerProfileId,
  profileUserDataDir,
  registerAuthenticated = false,
  registerReadinessMonitor = false,
  runDueReadinessMonitor = false,
  runBrowserCapabilityPreflight = false,
  readinessMonitorId,
  readinessMonitorIntervalMs,
  freshnessProfileId,
  freshnessReadinessState = 'fresh',
  freshnessEvidence,
  freshnessLastVerifiedAt,
  freshnessExpiresAt,
  freshnessUpdateAuthenticatedServiceIds = true,
  fetch = globalThis.fetch,
  dryRun = false,
} = {}) {
  const plan = buildManagedProfilePlan({
    url,
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    readinessProfileId,
    registerProfileId,
    profileUserDataDir,
    registerAuthenticated,
    registerReadinessMonitor,
    runDueReadinessMonitor,
    runBrowserCapabilityPreflight,
    readinessMonitorId,
    readinessMonitorIntervalMs,
    freshnessProfileId,
    freshnessReadinessState,
    freshnessEvidence,
    freshnessLastVerifiedAt,
    freshnessExpiresAt,
    freshnessUpdateAuthenticatedServiceIds,
  });

  if (dryRun) {
    return {
      dryRun: true,
      plan,
      note: 'Dry run does not contact agent-browser or create a runtime profile.',
    };
  }

  if (!baseUrl) {
    throw new Error('Missing baseUrl. Pass --base-url http://127.0.0.1:<stream-port>.');
  }

  const initialAccessPlan = await getServiceAccessPlan({
    baseUrl,
    fetch,
    serviceName,
    loginId,
    targetServiceId,
    readinessProfileId,
  });
  let accessPlan = initialAccessPlan;

  const profileRegistration =
    !initialAccessPlan.selectedProfile && registerProfileId
      ? await registerServiceLoginProfile({
          baseUrl,
          fetch,
          ...buildLoginProfileRegistration({
            id: registerProfileId,
            serviceName,
            loginId,
            targetServiceId,
            userDataDir: profileUserDataDir,
            authenticated: registerAuthenticated,
          }),
        })
      : null;

  const profileReadinessMonitor =
    profileRegistration && registerReadinessMonitor
      ? await upsertServiceProfileReadinessMonitor({
          baseUrl,
          fetch,
          ...buildProfileReadinessMonitor({
            id: readinessMonitorId,
            serviceName,
            loginId,
            targetServiceId,
            intervalMs: readinessMonitorIntervalMs,
          }),
        })
      : null;

  const profileFreshnessUpdate = freshnessProfileId
    ? await updateServiceProfileFreshness({
        baseUrl,
        fetch,
        ...buildProfileFreshnessUpdate({
          id: freshnessProfileId,
          loginId,
          targetServiceId,
          readinessState: freshnessReadinessState,
          readinessEvidence: freshnessEvidence,
          lastVerifiedAt: freshnessLastVerifiedAt,
          freshnessExpiresAt,
          updateAuthenticatedServiceIds: freshnessUpdateAuthenticatedServiceIds,
        }),
      })
    : null;

  if (profileRegistration || profileReadinessMonitor || profileFreshnessUpdate) {
    accessPlan = await getServiceAccessPlan({
      baseUrl,
      fetch,
      serviceName,
      loginId,
      targetServiceId,
      readinessProfileId,
    });
  }

  const monitorRunDue =
    runDueReadinessMonitor && accessPlan.decision?.monitorRunDue?.recommendedBeforeUse === true
      ? await runServiceAccessPlanMonitorRunDue({
          baseUrl,
          fetch,
          accessPlan,
        })
      : null;

  if (monitorRunDue) {
    accessPlan = await getServiceAccessPlan({
      baseUrl,
      fetch,
      serviceName,
      loginId,
      targetServiceId,
      readinessProfileId,
    });
  }

  const browserCapabilityPreflight =
    runBrowserCapabilityPreflight && accessPlan.decision?.browserCapabilityPreflight?.available === true
      ? await runServiceAccessPlanBrowserCapabilityPreflight({
          baseUrl,
          fetch,
          accessPlan,
        })
      : null;
  const selectedProfile = accessPlan.selectedProfile;

  const manualSeedingRequired =
    accessPlan.readinessSummary?.manualSeedingRequired === true ||
    accessPlan.decision?.manualSeedingRequired === true;
  const cdpFreeRequired = accessPlanRequiresCdpFree(accessPlan);
  const cdpFreeAvailability = summarizeCdpFreeAvailability(accessPlan);
  const tab = manualSeedingRequired
    ? {
        success: false,
        skipped: true,
        reason: 'manual_seeding_required',
        seedingHandoff: accessPlan.seedingHandoff ?? null,
      }
    : cdpFreeRequired
      ? {
          ...(await requestServiceCdpFreeLaunch({
            baseUrl,
            fetch,
            accessPlan,
            monitorRunDueSummary: monitorRunDue?.accessPlanSummary,
            url,
            jobTimeoutMs: 30000,
          })),
          mode: 'cdp_free_launch',
        }
    : await requestServiceTab({
        baseUrl,
        fetch,
        accessPlan,
        monitorRunDueSummary: monitorRunDue?.accessPlanSummary,
        url,
        jobTimeoutMs: 30000,
      });

  const profileAcquisitionSummary = summarizeServiceProfileAcquisition({
    initialAccessPlan,
    accessPlan,
    selectedProfile,
    profileRegistration,
    profileReadinessMonitor,
    monitorRunDue,
    monitorRunDueSummary: monitorRunDue?.accessPlanSummary ?? null,
    browserCapabilityPreflight,
    registered: Boolean(profileRegistration),
    monitorRegistered: Boolean(profileReadinessMonitor),
    monitorRunDueRan: Boolean(monitorRunDue),
    browserCapabilityPreflightRan: Boolean(browserCapabilityPreflight),
  });

  return {
    dryRun: false,
    plan,
    profileAcquisitionSummary,
    initialAccessPlan,
    accessPlan,
    selectedProfile: selectedProfile ? summarizeProfile(selectedProfile) : null,
    selectedProfileMatch: accessPlan.selectedProfileMatch,
    readiness: accessPlan.readiness,
    readinessSummary: accessPlan.readinessSummary,
    accessDecision: accessPlan.decision,
    accessAttention: profileAcquisitionSummary.refreshedAttention,
    cdpFreeAvailability,
    sitePolicy: accessPlan.sitePolicy,
    providers: accessPlan.providers,
    challenges: accessPlan.challenges,
    profileRegistration,
    profileReadinessMonitor,
    monitorRunDue,
    browserCapabilityPreflight,
    profileFreshnessUpdate,
    tab,
  };
}

/**
 * @param {Record<string, any>} accessPlan
 */
function accessPlanRequiresCdpFree(accessPlan) {
  const launchPosture = accessPlan.decision?.launchPosture;
  const postureRequiresCdpFree =
    launchPosture &&
    typeof launchPosture === 'object' &&
    launchPosture.requiresCdpFree === true &&
    launchPosture.cdpAttachmentAllowed !== true;
  const serviceRequest = accessPlan.decision?.serviceRequest;
  const serviceRequestRequiresCdpFree =
    serviceRequest &&
    typeof serviceRequest === 'object' &&
    serviceRequest.requiresCdpFree === true &&
    serviceRequest.cdpAttachmentAllowed !== true;
  return Boolean(postureRequiresCdpFree || serviceRequestRequiresCdpFree);
}

/**
 * @param {Record<string, any>} accessPlan
 */
function summarizeCdpFreeAvailability(accessPlan) {
  const availability = accessPlan.decision?.serviceRequest?.cdpFreeAvailability;
  if (!availability || typeof availability !== 'object') {
    return null;
  }
  return {
    applies: availability.applies === true,
    availableCommands: Array.isArray(availability.availableCommands)
      ? availability.availableCommands
      : [],
    unsupportedCommands: Array.isArray(availability.unsupportedCommands)
      ? availability.unsupportedCommands
      : [],
    supportedOperations: Array.isArray(availability.supportedOperations)
      ? availability.supportedOperations
      : [],
    unsupportedOperations: Array.isArray(availability.unsupportedOperations)
      ? availability.unsupportedOperations
      : [],
    summaryHelper: availability.client?.summaryHelper ?? null,
    predicateHelper: availability.client?.predicateHelper ?? null,
  };
}

/**
 * @param {unknown} profile
 */
function summarizeProfile(profile) {
  const record = /** @type {Record<string, unknown>} */ (profile);
  return {
    id: record.id,
    name: record.name,
    authenticatedServiceIds: record.authenticatedServiceIds,
    targetServiceIds: record.targetServiceIds,
    sharedServiceIds: record.sharedServiceIds,
  };
}

/**
 * @param {{
 *   id: string,
 *   serviceName: string,
 *   loginId?: string,
 *   targetServiceId?: string,
 *   userDataDir?: string,
 *   authenticated?: boolean,
 * }} options
 */
function buildLoginProfileRegistration({
  id,
  serviceName,
  loginId,
  targetServiceId,
  userDataDir,
  authenticated,
}) {
  const identity = loginId || targetServiceId;
  return {
    id,
    name: `${serviceName} ${identity} managed profile`,
    serviceName,
    loginId: identity,
    targetServiceId,
    userDataDir,
    authenticated,
  };
}

/**
 * @param {{
 *   id: string,
 *   loginId?: string,
 *   targetServiceId?: string,
 *   readinessState?: ServiceProfileReadinessState,
 *   readinessEvidence?: string,
 *   lastVerifiedAt?: string,
 *   freshnessExpiresAt?: string,
 *   updateAuthenticatedServiceIds?: boolean,
 * }} options
 */
function buildProfileFreshnessUpdate({
  id,
  loginId,
  targetServiceId,
  readinessState,
  readinessEvidence,
  lastVerifiedAt,
  freshnessExpiresAt,
  updateAuthenticatedServiceIds,
}) {
  const identity = loginId || targetServiceId;
  return {
    id,
    loginId: identity,
    targetServiceId,
    readinessState,
    readinessEvidence,
    lastVerifiedAt,
    freshnessExpiresAt,
    updateAuthenticatedServiceIds,
  };
}

/**
 * @param {{
 *   id?: string,
 *   serviceName: string,
 *   loginId?: string,
 *   targetServiceId?: string,
 *   intervalMs?: number,
 * }} options
 */
function buildProfileReadinessMonitor({
  id,
  serviceName,
  loginId,
  targetServiceId,
  intervalMs,
}) {
  const identity = loginId || targetServiceId;
  return {
    id,
    serviceName,
    loginId: identity,
    targetServiceId,
    intervalMs,
  };
}

if (import.meta.url === `file://${nodeProcess.argv[1]}`) {
  runManagedProfileWorkflow(parseArgs(nodeProcess.argv.slice(2)))
    .then((result) => {
      console.log(JSON.stringify(result, null, 2));
    })
    .catch((err) => {
      console.error(err?.stack || err?.message || String(err));
      nodeProcess.exit(1);
    });
}

/**
 * @param {string[]} args
 * @returns {ManagedProfileOptions}
 */
function parseArgs(args) {
  /** @type {ManagedProfileOptions} */
  const parsed = {
    baseUrl: nodeProcess.env.AGENT_BROWSER_SERVICE_BASE_URL,
    url: nodeProcess.env.AGENT_BROWSER_EXAMPLE_URL || DEFAULT_URL,
    serviceName: nodeProcess.env.AGENT_BROWSER_EXAMPLE_SERVICE || 'CanvaCLI',
    agentName: nodeProcess.env.AGENT_BROWSER_EXAMPLE_AGENT || 'canva-cli-agent',
    taskName: nodeProcess.env.AGENT_BROWSER_EXAMPLE_TASK || 'openCanvaWorkspace',
    loginId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_LOGIN || 'canva',
    targetServiceId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_TARGET_SERVICE || 'canva',
    readinessProfileId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_READINESS_PROFILE_ID,
    registerProfileId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_REGISTER_PROFILE_ID,
    profileUserDataDir: nodeProcess.env.AGENT_BROWSER_EXAMPLE_PROFILE_USER_DATA_DIR,
    registerAuthenticated: nodeProcess.env.AGENT_BROWSER_EXAMPLE_REGISTER_AUTHENTICATED === '1',
    registerReadinessMonitor: nodeProcess.env.AGENT_BROWSER_EXAMPLE_REGISTER_READINESS_MONITOR === '1',
    runDueReadinessMonitor: nodeProcess.env.AGENT_BROWSER_EXAMPLE_RUN_DUE_READINESS_MONITOR === '1',
    runBrowserCapabilityPreflight: nodeProcess.env.AGENT_BROWSER_EXAMPLE_RUN_BROWSER_CAPABILITY_PREFLIGHT === '1',
    readinessMonitorId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_READINESS_MONITOR_ID,
    readinessMonitorIntervalMs: numberEnv(nodeProcess.env.AGENT_BROWSER_EXAMPLE_READINESS_MONITOR_INTERVAL_MS),
    freshnessProfileId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_FRESHNESS_PROFILE_ID,
    freshnessReadinessState: nodeProcess.env.AGENT_BROWSER_EXAMPLE_FRESHNESS_STATE || 'fresh',
    freshnessEvidence: nodeProcess.env.AGENT_BROWSER_EXAMPLE_FRESHNESS_EVIDENCE,
    freshnessLastVerifiedAt: nodeProcess.env.AGENT_BROWSER_EXAMPLE_FRESHNESS_LAST_VERIFIED_AT,
    freshnessExpiresAt: nodeProcess.env.AGENT_BROWSER_EXAMPLE_FRESHNESS_EXPIRES_AT,
    freshnessUpdateAuthenticatedServiceIds:
      nodeProcess.env.AGENT_BROWSER_EXAMPLE_FRESHNESS_UPDATE_AUTH_IDS !== '0',
    dryRun: false,
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === '--dry-run') {
      parsed.dryRun = true;
    } else if (arg === '--base-url') {
      parsed.baseUrl = requiredValue(args, ++index, arg);
    } else if (arg === '--url') {
      parsed.url = requiredValue(args, ++index, arg);
    } else if (arg === '--service-name') {
      parsed.serviceName = requiredValue(args, ++index, arg);
    } else if (arg === '--agent-name') {
      parsed.agentName = requiredValue(args, ++index, arg);
    } else if (arg === '--task-name') {
      parsed.taskName = requiredValue(args, ++index, arg);
    } else if (arg === '--login-id') {
      parsed.loginId = requiredValue(args, ++index, arg);
    } else if (arg === '--target-service-id') {
      parsed.targetServiceId = requiredValue(args, ++index, arg);
    } else if (arg === '--readiness-profile-id') {
      parsed.readinessProfileId = requiredValue(args, ++index, arg);
    } else if (arg === '--register-profile-id') {
      parsed.registerProfileId = requiredValue(args, ++index, arg);
    } else if (arg === '--profile-user-data-dir') {
      parsed.profileUserDataDir = requiredValue(args, ++index, arg);
    } else if (arg === '--register-authenticated') {
      parsed.registerAuthenticated = true;
    } else if (arg === '--register-readiness-monitor') {
      parsed.registerReadinessMonitor = true;
    } else if (arg === '--run-due-readiness-monitor') {
      parsed.runDueReadinessMonitor = true;
    } else if (arg === '--run-browser-capability-preflight') {
      parsed.runBrowserCapabilityPreflight = true;
    } else if (arg === '--readiness-monitor-id') {
      parsed.readinessMonitorId = requiredValue(args, ++index, arg);
    } else if (arg === '--readiness-monitor-interval-ms') {
      parsed.readinessMonitorIntervalMs = Number(requiredValue(args, ++index, arg));
    } else if (arg === '--freshness-profile-id') {
      parsed.freshnessProfileId = requiredValue(args, ++index, arg);
    } else if (arg === '--freshness-state') {
      parsed.freshnessReadinessState = /** @type {ServiceProfileReadinessState} */ (
        requiredValue(args, ++index, arg)
      );
    } else if (arg === '--freshness-evidence') {
      parsed.freshnessEvidence = requiredValue(args, ++index, arg);
    } else if (arg === '--freshness-last-verified-at') {
      parsed.freshnessLastVerifiedAt = requiredValue(args, ++index, arg);
    } else if (arg === '--freshness-expires-at') {
      parsed.freshnessExpiresAt = requiredValue(args, ++index, arg);
    } else if (arg === '--skip-authenticated-service-id-update') {
      parsed.freshnessUpdateAuthenticatedServiceIds = false;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return parsed;
}

/**
 * @param {string | undefined} value
 */
function numberEnv(value) {
  if (value === undefined || value === '') {
    return undefined;
  }
  return Number(value);
}

/**
 * @param {string[]} args
 * @param {number} index
 * @param {string} flag
 */
function requiredValue(args, index, flag) {
  const value = args[index];
  if (!value) {
    throw new Error(`Missing value for ${flag}`);
  }
  return value;
}
