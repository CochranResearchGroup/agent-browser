#!/usr/bin/env node
// @ts-check

import { createServiceRequest, postServiceRequest, requestServiceTab } from '@agent-browser/client/service-request';
import {
  acquireServiceLoginProfile,
  cancelServiceJob,
  getServiceTrace,
} from '@agent-browser/client/service-observability';

const DEFAULT_URL = 'https://example.com';
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
 *   siteId?: string;
 *   loginId?: string;
 *   registerProfileId?: string;
 *   profileUserDataDir?: string;
 *   registerReadinessMonitor?: boolean;
 *   readinessMonitorId?: string;
 *   readinessMonitorIntervalMs?: number;
 *   cancelJobId?: string;
 *   fetch?: typeof globalThis.fetch;
 *   dryRun?: boolean;
 * }} WorkflowOptions
 */

/**
 * @param {Omit<WorkflowOptions, 'baseUrl' | 'dryRun' | 'cancelJobId'>} options
 */
export function buildServiceTabRequest({
  url = DEFAULT_URL,
  serviceName = 'JournalDownloader',
  agentName = 'article-probe-agent',
  taskName = 'probeACSwebsite',
  siteId = 'example',
  loginId = siteId,
} = {}) {
  return createServiceRequest({
    serviceName,
    agentName,
    taskName,
    siteId,
    loginId,
    action: 'tab_new',
    params: { url },
    jobTimeoutMs: 30000,
  });
}

/**
 * @param {WorkflowOptions} options
 */
export async function runServiceWorkflow({
  baseUrl,
  url = DEFAULT_URL,
  serviceName = 'JournalDownloader',
  agentName = 'article-probe-agent',
  taskName = 'probeACSwebsite',
  siteId = 'example',
  loginId = siteId,
  registerProfileId,
  profileUserDataDir,
  registerReadinessMonitor = false,
  readinessMonitorId,
  readinessMonitorIntervalMs,
  cancelJobId,
  fetch = globalThis.fetch,
  dryRun = false,
} = {}) {
  const request = buildServiceTabRequest({ url, serviceName, agentName, taskName, siteId, loginId });

  if (dryRun) {
    return {
      dryRun: true,
      request,
      traceQuery: { serviceName, agentName, taskName, limit: 50 },
      accessPlanQuery: { serviceName, agentName, taskName, siteId, loginId },
      profileSelection: {
        requestedIdentity: loginId || siteId,
        preferredProfileFields: ['authenticatedServiceIds', 'targetServiceIds', 'sharedServiceIds'],
        helper: 'acquireServiceLoginProfile',
        registrationPolicy: 'only register fallback profile after the access plan reports no selected profile',
      },
      profileRegistration: registerProfileId
        ? buildLoginProfileRegistration({
            id: registerProfileId,
            serviceName,
            loginId,
            siteId,
            userDataDir: profileUserDataDir,
          })
        : null,
      profileReadinessMonitor:
        registerProfileId && registerReadinessMonitor
          ? buildProfileReadinessMonitor({
              id: readinessMonitorId,
              serviceName,
              loginId,
              siteId,
              intervalMs: readinessMonitorIntervalMs,
            })
          : null,
      cancelRequest: cancelJobId ? { jobId: cancelJobId, remedy: 'cancelServiceJob' } : null,
    };
  }

  if (!baseUrl) {
    throw new Error('Missing baseUrl. Pass --base-url http://127.0.0.1:<stream-port>.');
  }

  const profileAcquisition = await acquireServiceLoginProfile({
    baseUrl,
    fetch,
    serviceName,
    agentName,
    taskName,
    siteId,
    loginId,
    registerProfileId,
    profileName: registerProfileId ? `${serviceName} ${loginId || siteId} profile` : undefined,
    profileUserDataDir,
    registerReadinessMonitor,
    readinessMonitorId,
    readinessMonitorIntervalMs,
  });
  const { initialAccessPlan, profileRegistration, profileReadinessMonitor, accessPlan } = profileAcquisition;
  const commandResult = await requestServiceTab({
    baseUrl,
    fetch,
    accessPlan,
    url,
    jobTimeoutMs: 30000,
  });
  const titleResult = await postServiceRequest({
    baseUrl,
    fetch,
    request: createServiceRequest({
      serviceName,
      agentName,
      taskName,
      siteId,
      loginId,
      action: 'title',
      jobTimeoutMs: 30000,
    }),
  });
  const waitResult = await postServiceRequest({
    baseUrl,
    fetch,
    request: createServiceRequest({
      serviceName,
      agentName,
      taskName,
      siteId,
      loginId,
      action: 'wait',
      params: { timeoutMs: 1 },
      jobTimeoutMs: 30000,
    }),
  });
  const viewportResult = await postServiceRequest({
    baseUrl,
    fetch,
    request: createServiceRequest({
      serviceName,
      agentName,
      taskName,
      siteId,
      loginId,
      action: 'viewport',
      params: { width: 1024, height: 768 },
      jobTimeoutMs: 30000,
    }),
  });
  const consoleResult = await postServiceRequest({
    baseUrl,
    fetch,
    request: createServiceRequest({
      serviceName,
      agentName,
      taskName,
      siteId,
      loginId,
      action: 'console',
      jobTimeoutMs: 30000,
    }),
  });
  const cancelResult = cancelJobId ? await cancelServiceJob({ baseUrl, fetch, jobId: cancelJobId }) : null;
  const trace = await getServiceTrace({
    baseUrl,
    fetch,
    query: { serviceName, agentName, taskName, limit: 50 },
  });
  const consoleMessageCount = countConsoleMessages(consoleResult.data);

  return {
    dryRun: false,
    initialAccessPlan,
    profileAcquisition,
    profileRegistration,
    profileReadinessMonitor,
    accessPlan,
    commandResult,
    commandResultData: {
      tabIndex: commandResult.data?.index,
      tabUrl: commandResult.data?.url,
      pageTitle: titleResult.data?.title,
      waitKind: waitResult.data?.waited,
      viewportWidth: viewportResult.data?.width,
      viewportHeight: viewportResult.data?.height,
      consoleMessages: consoleMessageCount,
    },
    titleResult,
    waitResult,
    viewportResult,
    consoleResult,
    cancelResult,
    traceSummary: {
      events: trace.counts.events,
      jobs: trace.counts.jobs,
      incidents: trace.counts.incidents,
      activity: trace.counts.activity,
    },
    latestJobs: trace.jobs.slice(-5).map((/** @type {import('@agent-browser/client/service-observability').ServiceJobRecord} */ job) => ({
      id: job.id,
      action: job.action,
      state: job.state,
      serviceName: job.serviceName,
      agentName: job.agentName,
      taskName: job.taskName,
    })),
  };
}

/**
 * @param {import('@agent-browser/client/service-request').ServiceRequestDataForAction<'console'> | undefined} data
 */
function countConsoleMessages(data) {
  if (!data || !('messages' in data) || !Array.isArray(data.messages)) {
    return 0;
  }
  return data.messages.length;
}

/**
 * @param {{ id: string, serviceName: string, loginId?: string, siteId?: string, userDataDir?: string }} options
 */
function buildLoginProfileRegistration({ id, serviceName, loginId, siteId, userDataDir }) {
  const identity = loginId || siteId;
  return {
    id,
    name: `${serviceName} ${identity} profile`,
    serviceName,
    loginId: identity,
    userDataDir,
  };
}

/**
 * @param {{ id?: string, serviceName: string, loginId?: string, siteId?: string, intervalMs?: number }} options
 */
function buildProfileReadinessMonitor({ id, serviceName, loginId, siteId, intervalMs }) {
  const identity = loginId || siteId;
  return {
    id,
    serviceName,
    loginId: identity,
    siteId,
    intervalMs,
  };
}

if (import.meta.url === `file://${nodeProcess.argv[1]}`) {
  runServiceWorkflow(parseArgs(nodeProcess.argv.slice(2)))
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
 * @returns {WorkflowOptions}
 */
function parseArgs(args) {
  /** @type {WorkflowOptions} */
  const parsed = {
    baseUrl: nodeProcess.env.AGENT_BROWSER_SERVICE_BASE_URL,
    url: nodeProcess.env.AGENT_BROWSER_EXAMPLE_URL || DEFAULT_URL,
    serviceName: nodeProcess.env.AGENT_BROWSER_EXAMPLE_SERVICE || 'JournalDownloader',
    agentName: nodeProcess.env.AGENT_BROWSER_EXAMPLE_AGENT || 'article-probe-agent',
    taskName: nodeProcess.env.AGENT_BROWSER_EXAMPLE_TASK || 'probeACSwebsite',
    siteId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_SITE || 'example',
    loginId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_LOGIN || nodeProcess.env.AGENT_BROWSER_EXAMPLE_SITE || 'example',
    registerProfileId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_REGISTER_PROFILE_ID,
    profileUserDataDir: nodeProcess.env.AGENT_BROWSER_EXAMPLE_PROFILE_USER_DATA_DIR,
    registerReadinessMonitor: nodeProcess.env.AGENT_BROWSER_EXAMPLE_REGISTER_READINESS_MONITOR === '1',
    readinessMonitorId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_READINESS_MONITOR_ID,
    readinessMonitorIntervalMs: numberEnv(nodeProcess.env.AGENT_BROWSER_EXAMPLE_READINESS_MONITOR_INTERVAL_MS),
    cancelJobId: nodeProcess.env.AGENT_BROWSER_EXAMPLE_CANCEL_JOB_ID,
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
    } else if (arg === '--site-id') {
      parsed.siteId = requiredValue(args, ++index, arg);
    } else if (arg === '--login-id') {
      parsed.loginId = requiredValue(args, ++index, arg);
    } else if (arg === '--register-profile-id') {
      parsed.registerProfileId = requiredValue(args, ++index, arg);
    } else if (arg === '--profile-user-data-dir') {
      parsed.profileUserDataDir = requiredValue(args, ++index, arg);
    } else if (arg === '--register-readiness-monitor') {
      parsed.registerReadinessMonitor = true;
    } else if (arg === '--readiness-monitor-id') {
      parsed.readinessMonitorId = requiredValue(args, ++index, arg);
    } else if (arg === '--readiness-monitor-interval-ms') {
      parsed.readinessMonitorIntervalMs = Number(requiredValue(args, ++index, arg));
    } else if (arg === '--cancel-job-id') {
      parsed.cancelJobId = requiredValue(args, ++index, arg);
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
