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
 */

/**
 * @param {ServiceObservabilityHttpOptions} options
 * @returns {Promise<ServiceStatusResponse>}
 */
export function getServiceStatus(options) {
  return serviceGet(options, '/api/service/status');
}

/**
 * @param {ServiceQueryOptions} options
 * @returns {Promise<ServiceProfilesResponse>}
 */
export function getServiceProfiles(options) {
  return serviceGet(options, '/api/service/profiles');
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
 * @returns {Promise<TResult>}
 */
async function servicePost({ baseUrl, fetch = globalThis.fetch, signal }, pathname) {
  if (typeof fetch !== 'function') {
    throw new TypeError('service observability helpers require a fetch implementation');
  }
  if (typeof baseUrl !== 'string' || baseUrl.length === 0) {
    throw new TypeError('service observability helpers require a baseUrl string');
  }

  const response = await fetch(new URL(pathname, baseUrl), {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
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
