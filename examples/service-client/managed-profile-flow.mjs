#!/usr/bin/env node
// @ts-check

import { requestServiceTab } from '@agent-browser/client/service-request';
import {
  getServiceProfileForIdentity,
  registerServiceLoginProfile,
} from '@agent-browser/client/service-observability';

const DEFAULT_URL = 'https://www.canva.com/';
const nodeProcess = /** @type {{ argv: string[], env: Record<string, string | undefined>, exit(code?: number): never }} */ (
  /** @type {any} */ (globalThis).process
);

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
} = {}) {
  return {
    serviceName,
    agentName,
    taskName,
    requestedIdentity: loginId || targetServiceId,
    targetServiceId,
    url,
    decisionOrder: [
      'inspect managed service profiles',
      'inspect target readiness when a candidate profile is known',
      'request a tab by login or target identity',
      'register a managed profile only when agent-browser has no suitable one',
      'seed the profile manually when readiness reports needs_manual_seeding',
    ],
    profileInspection: {
      helper: 'getServiceProfiles',
      matchFields: ['authenticatedServiceIds', 'targetServiceIds', 'sharedServiceIds'],
    },
    readinessInspection: readinessProfileId
      ? {
          helper: 'getServiceProfileReadiness',
          id: readinessProfileId,
        }
      : null,
    tabRequest: {
      helper: 'requestServiceTab',
      serviceName,
      agentName,
      taskName,
      loginId,
      targetServiceId,
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

  const profileLookup = await getServiceProfileForIdentity({
    baseUrl,
    fetch,
    serviceName,
    loginId,
    targetServiceId,
    readinessProfileId,
  });
  const selectedProfile = profileLookup.selectedProfile;

  const profileRegistration =
    !selectedProfile && registerProfileId
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

  const tab = await requestServiceTab({
    baseUrl,
    fetch,
    url,
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    jobTimeoutMs: 30000,
  });

  return {
    dryRun: false,
    plan,
    selectedProfile: selectedProfile ? summarizeProfile(selectedProfile) : null,
    selectedProfileMatch: profileLookup.selectedProfileMatch,
    readiness: profileLookup.readiness,
    readinessSummary: profileLookup.readinessSummary,
    profileRegistration,
    tab,
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
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return parsed;
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
