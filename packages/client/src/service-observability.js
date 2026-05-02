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
 * @typedef {import('./service-observability.generated.js').ServiceQueryOptions} ServiceQueryOptions
 * @typedef {import('./service-observability.generated.js').ServiceTraceResponse} ServiceTraceResponse
 */

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
