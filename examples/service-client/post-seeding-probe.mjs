#!/usr/bin/env node
// @ts-check

import { createServiceRequest, postServiceRequest, requestServiceTab } from '@agent-browser/client/service-request';
import { lookupServiceProfile, verifyServiceProfileSeeding } from '@agent-browser/client/service-observability';

const DEFAULT_URL = 'https://accounts.google.com/';
const nodeProcess = /** @type {{ argv: string[], env: Record<string, string | undefined>, exit(code?: number): never }} */ (
  /** @type {any} */ (globalThis).process
);

/**
 * @typedef {{
 *   baseUrl?: string;
 *   profileId?: string;
 *   url?: string;
 *   serviceName?: string;
 *   agentName?: string;
 *   taskName?: string;
 *   loginId?: string;
 *   targetServiceId?: string;
 *   expectedUrlIncludes?: string;
 *   expectedTitleIncludes?: string;
 *   freshnessExpiresAt?: string;
 *   fetch?: typeof globalThis.fetch;
 *   dryRun?: boolean;
 * }} PostSeedingProbeOptions
 */

/**
 * @param {PostSeedingProbeOptions} options
 */
export function buildPostSeedingProbePlan({
  profileId = 'google-work',
  url = DEFAULT_URL,
  serviceName = 'ProfileSeeder',
  agentName = 'post-seeding-probe',
  taskName = 'verifySeededProfile',
  loginId = 'google',
  targetServiceId = loginId,
  expectedUrlIncludes,
  expectedTitleIncludes,
  freshnessExpiresAt,
} = {}) {
  return {
    profileId,
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    url,
    boundedChecks: {
      expectedUrlIncludes: expectedUrlIncludes ?? null,
      expectedTitleIncludes: expectedTitleIncludes ?? null,
    },
    sequence: [
      'confirm the broker-selected profile matches the profile being verified',
      'request an attachable service-owned tab for the seeded profile identity',
      'read the current URL and page title through queued service requests',
      'evaluate bounded URL and title expectations without broad site-specific automation',
      'record freshness with verifyServiceProfileSeeding',
    ],
    verificationUpdate: {
      helper: 'verifyServiceProfileSeeding',
      profileId,
      targetServiceId,
      defaultFreshnessExpiresAt: freshnessExpiresAt ?? null,
    },
  };
}

/**
 * @param {PostSeedingProbeOptions} options
 */
export async function runPostSeedingProbe({
  baseUrl,
  profileId = 'google-work',
  url = DEFAULT_URL,
  serviceName = 'ProfileSeeder',
  agentName = 'post-seeding-probe',
  taskName = 'verifySeededProfile',
  loginId = 'google',
  targetServiceId = loginId,
  expectedUrlIncludes,
  expectedTitleIncludes,
  freshnessExpiresAt,
  fetch = globalThis.fetch,
  dryRun = false,
} = {}) {
  const plan = buildPostSeedingProbePlan({
    profileId,
    url,
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    expectedUrlIncludes,
    expectedTitleIncludes,
    freshnessExpiresAt,
  });

  if (dryRun) {
    return {
      dryRun: true,
      plan,
      note: 'Dry run does not contact agent-browser or launch a browser.',
    };
  }

  if (!baseUrl) {
    throw new Error('Missing baseUrl. Pass --base-url http://127.0.0.1:<stream-port>.');
  }
  if (!profileId) {
    throw new Error('Missing profileId. Pass --profile-id <id>.');
  }

  const lookup = await lookupServiceProfile({
    baseUrl,
    fetch,
    serviceName,
    loginId,
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
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    url,
    jobTimeoutMs: 30000,
  });
  const urlResult = await postServiceRequest({
    baseUrl,
    fetch,
    request: createServiceRequest({
      serviceName,
      agentName,
      taskName,
      loginId,
      targetServiceId,
      action: 'url',
      jobTimeoutMs: 30000,
    }),
  });
  const titleResult = await postServiceRequest({
    baseUrl,
    fetch,
    request: createServiceRequest({
      serviceName,
      agentName,
      taskName,
      loginId,
      targetServiceId,
      action: 'title',
      jobTimeoutMs: 30000,
    }),
  });

  const observedUrl = stringData(urlResult.data, 'url');
  const observedTitle = stringData(titleResult.data, 'title');
  const checks = evaluateChecks({
    observedUrl,
    observedTitle,
    expectedUrlIncludes,
    expectedTitleIncludes,
  });
  const readinessState = checks.fresh ? 'fresh' : 'stale';
  const readinessEvidence = checks.fresh
    ? `post_seeding_auth_probe_passed:${checks.passed.join(',') || 'service_tab_opened'}`
    : `post_seeding_auth_probe_failed:${checks.failed.join(',') || 'bounded_probe_failed'}`;

  const freshness = await verifyServiceProfileSeeding({
    baseUrl,
    fetch,
    id: profileId,
    loginId,
    targetServiceId,
    readinessState,
    readinessEvidence,
    lastVerifiedAt: new Date().toISOString(),
    freshnessExpiresAt,
  });

  return {
    dryRun: false,
    plan,
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

/**
 * @param {{ observedUrl: string, observedTitle: string, expectedUrlIncludes?: string, expectedTitleIncludes?: string }} input
 */
function evaluateChecks({ observedUrl, observedTitle, expectedUrlIncludes, expectedTitleIncludes }) {
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

/**
 * @param {unknown} data
 * @param {string} field
 */
function stringData(data, field) {
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    return '';
  }
  const value = /** @type {Record<string, unknown>} */ (data)[field];
  return typeof value === 'string' ? value : '';
}

if (import.meta.url === `file://${nodeProcess.argv[1]}`) {
  runPostSeedingProbe(parseArgs(nodeProcess.argv.slice(2)))
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
 * @returns {PostSeedingProbeOptions}
 */
function parseArgs(args) {
  /** @type {PostSeedingProbeOptions} */
  const parsed = {
    baseUrl: nodeProcess.env.AGENT_BROWSER_SERVICE_BASE_URL,
    profileId: nodeProcess.env.AGENT_BROWSER_PROBE_PROFILE_ID || 'google-work',
    url: nodeProcess.env.AGENT_BROWSER_PROBE_URL || DEFAULT_URL,
    serviceName: nodeProcess.env.AGENT_BROWSER_PROBE_SERVICE || 'ProfileSeeder',
    agentName: nodeProcess.env.AGENT_BROWSER_PROBE_AGENT || 'post-seeding-probe',
    taskName: nodeProcess.env.AGENT_BROWSER_PROBE_TASK || 'verifySeededProfile',
    loginId: nodeProcess.env.AGENT_BROWSER_PROBE_LOGIN || 'google',
    targetServiceId: nodeProcess.env.AGENT_BROWSER_PROBE_TARGET || nodeProcess.env.AGENT_BROWSER_PROBE_LOGIN || 'google',
    expectedUrlIncludes: nodeProcess.env.AGENT_BROWSER_PROBE_EXPECTED_URL,
    expectedTitleIncludes: nodeProcess.env.AGENT_BROWSER_PROBE_EXPECTED_TITLE,
    freshnessExpiresAt: nodeProcess.env.AGENT_BROWSER_PROBE_FRESHNESS_EXPIRES_AT,
    dryRun: false,
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === '--dry-run') {
      parsed.dryRun = true;
    } else if (arg === '--base-url') {
      parsed.baseUrl = requiredValue(args, ++index, arg);
    } else if (arg === '--profile-id') {
      parsed.profileId = requiredValue(args, ++index, arg);
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
    } else if (arg === '--expected-url-includes') {
      parsed.expectedUrlIncludes = requiredValue(args, ++index, arg);
    } else if (arg === '--expected-title-includes') {
      parsed.expectedTitleIncludes = requiredValue(args, ++index, arg);
    } else if (arg === '--freshness-expires-at') {
      parsed.freshnessExpiresAt = requiredValue(args, ++index, arg);
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
