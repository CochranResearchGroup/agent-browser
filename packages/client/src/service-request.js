// @ts-check

import {
  SERVICE_REQUEST_ACTIONS,
  SERVICE_REQUEST_BOOLEAN_FIELDS,
  SERVICE_REQUEST_INTEGER_FIELDS,
  SERVICE_REQUEST_MCP_TOOL_NAME,
  SERVICE_REQUEST_OBJECT_FIELDS,
  SERVICE_REQUEST_STRING_ARRAY_FIELDS,
  SERVICE_REQUEST_STRING_FIELDS,
} from './service-request.generated.js';

const actionSet = new Set(SERVICE_REQUEST_ACTIONS);

/**
 * @typedef {import('./service-request.generated.js').ServiceRequest} ServiceRequest
 * @typedef {import('./service-request.generated.js').ServiceRequestHttpOptions} ServiceRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceRequestResponse} ServiceRequestResponse
 * @typedef {import('./service-request.generated.js').ServiceTabAccessPlan} ServiceTabAccessPlan
 * @typedef {import('./service-request.generated.js').ServiceCdpFreeLaunchRequestHttpOptions} ServiceCdpFreeLaunchRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpFreeLaunchRequestOptions} ServiceCdpFreeLaunchRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceTabRequestHttpOptions} ServiceTabRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceTabRequestOptions} ServiceTabRequestOptions
 */

export {
  SERVICE_REQUEST_ACTIONS,
  SERVICE_REQUEST_BOOLEAN_FIELDS,
  SERVICE_REQUEST_INTEGER_FIELDS,
  SERVICE_REQUEST_MCP_TOOL_NAME,
  SERVICE_REQUEST_OBJECT_FIELDS,
  SERVICE_REQUEST_STRING_ARRAY_FIELDS,
  SERVICE_REQUEST_STRING_FIELDS,
} from './service-request.generated.js';

/**
 * @param {ServiceRequest} input
 * @returns {ServiceRequest}
 */
export function createServiceRequest(input) {
  assertPlainObject(input, 'service request');
  const record = /** @type {Record<string, unknown>} */ (/** @type {unknown} */ (input));
  if (!actionSet.has(input.action)) {
    throw new TypeError(`Unsupported service request action: ${String(input.action)}`);
  }
  if (Object.hasOwn(input, 'params')) {
    assertPlainObject(input.params, 'service request params');
  }
  for (const field of SERVICE_REQUEST_INTEGER_FIELDS) {
    if (
      Object.hasOwn(input, field) &&
      (!Number.isInteger(record[field]) || Number(record[field]) < 1)
    ) {
      throw new TypeError(`service request ${field} must be a positive integer`);
    }
  }
  for (const field of SERVICE_REQUEST_BOOLEAN_FIELDS) {
    if (Object.hasOwn(input, field) && typeof record[field] !== 'boolean') {
      throw new TypeError(`service request ${field} must be a boolean`);
    }
  }
  for (const field of SERVICE_REQUEST_OBJECT_FIELDS) {
    if (Object.hasOwn(input, field)) {
      assertPlainObject(record[field], `service request ${field}`);
    }
  }
  for (const field of SERVICE_REQUEST_STRING_FIELDS) {
    if (Object.hasOwn(input, field) && typeof record[field] !== 'string') {
      throw new TypeError(`service request ${field} must be a string`);
    }
  }
  if (
    Object.hasOwn(input, 'profileLeasePolicy') &&
    record.profileLeasePolicy !== 'reject' &&
    record.profileLeasePolicy !== 'wait'
  ) {
    throw new TypeError('service request profileLeasePolicy must be reject or wait');
  }
  for (const field of SERVICE_REQUEST_STRING_ARRAY_FIELDS) {
    if (Object.hasOwn(input, field)) {
      const value = record[field];
      if (!Array.isArray(value) || value.some((item) => typeof item !== 'string')) {
        throw new TypeError(`service request ${field} must be an array of strings`);
      }
    }
  }

  return { ...input };
}

/**
 * @param {ServiceRequest} input
 */
export function createServiceRequestMcpToolCall(input) {
  return {
    name: SERVICE_REQUEST_MCP_TOOL_NAME,
    arguments: createServiceRequest(input),
  };
}

/**
 * Builds a tab request and refuses access plans that still require manual
 * profile seeding unless allowManualAction is explicitly true.
 *
 * @param {ServiceTabRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceTabRequest(input) {
  assertPlainObject(input, 'service tab request');
  const { accessPlan, allowManualAction, allowMonitorFreshnessRisk, monitorRunDueSummary, url, params, ...request } =
    input;
  if (url !== undefined && typeof url !== 'string') {
    throw new TypeError('service tab request url must be a string');
  }
  if (allowManualAction !== undefined && typeof allowManualAction !== 'boolean') {
    throw new TypeError('service tab request allowManualAction must be a boolean');
  }
  if (allowMonitorFreshnessRisk !== undefined && typeof allowMonitorFreshnessRisk !== 'boolean') {
    throw new TypeError('service tab request allowMonitorFreshnessRisk must be a boolean');
  }
  if (params !== undefined) {
    assertPlainObject(params, 'service tab request params');
  }
  assertMonitorRunDueSummarySafe(monitorRunDueSummary, {
    allowMonitorFreshnessRisk,
    actionLabel: 'tab request',
  });
  const plannedRequest =
    accessPlan !== undefined ? accessPlanServiceTabRequest(accessPlan, { allowManualAction }) : {};

  const tabParams = { ...(params ?? {}) };
  if (url !== undefined) {
    tabParams.url = url;
  }

  return createServiceRequest({
    ...plannedRequest,
    ...request,
    action: 'tab_new',
    ...(accessPlan !== undefined && allowManualAction === true ? { allowManualAction: true } : {}),
    ...(Object.keys(tabParams).length > 0 ? { params: tabParams } : {}),
  });
}

/**
 * @param {ServiceTabAccessPlan} accessPlan
 * @param {Omit<ServiceTabRequestOptions, 'accessPlan'>} [input]
 * @returns {ServiceRequest}
 */
export function createServiceTabRequestFromAccessPlan(accessPlan, input = {}) {
  assertPlainObject(input, 'service tab request override');
  return createServiceTabRequest({ ...input, accessPlan });
}

/**
 * Builds a headed no-DevTools launch request for CDP-sensitive services.
 *
 * @param {ServiceCdpFreeLaunchRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceCdpFreeLaunchRequest(input) {
  assertPlainObject(input, 'CDP-free launch request');
  const { accessPlan, allowManualAction, allowMonitorFreshnessRisk, monitorRunDueSummary, url, params, ...request } =
    input;
  if (url !== undefined && typeof url !== 'string') {
    throw new TypeError('CDP-free launch request url must be a string');
  }
  if (allowManualAction !== undefined && typeof allowManualAction !== 'boolean') {
    throw new TypeError('CDP-free launch request allowManualAction must be a boolean');
  }
  if (allowMonitorFreshnessRisk !== undefined && typeof allowMonitorFreshnessRisk !== 'boolean') {
    throw new TypeError('CDP-free launch request allowMonitorFreshnessRisk must be a boolean');
  }
  if (params !== undefined) {
    assertPlainObject(params, 'CDP-free launch request params');
  }
  assertMonitorRunDueSummarySafe(monitorRunDueSummary, {
    allowMonitorFreshnessRisk,
    actionLabel: 'CDP-free launch',
  });
  const plannedRequest =
    accessPlan !== undefined ? accessPlanServiceCdpFreeLaunchRequest(accessPlan, { allowManualAction }) : {};

  const launchParams = { ...(params ?? {}) };
  if (url !== undefined) {
    launchParams.url = url;
  }

  return createServiceRequest({
    ...plannedRequest,
    ...request,
    action: 'cdp_free_launch',
    requiresCdpFree: true,
    cdpAttachmentAllowed: false,
    ...(accessPlan !== undefined && allowManualAction === true ? { allowManualAction: true } : {}),
    ...(url !== undefined ? { url } : {}),
    ...(Object.keys(launchParams).length > 0 ? { params: launchParams } : {}),
  });
}

/**
 * @param {ServiceRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function postServiceRequest({ baseUrl, request, fetch = globalThis.fetch, signal }) {
  if (typeof fetch !== 'function') {
    throw new TypeError('postServiceRequest requires a fetch implementation');
  }
  if (typeof baseUrl !== 'string' || baseUrl.length === 0) {
    throw new TypeError('postServiceRequest requires a baseUrl string');
  }

  const response = await fetch(new URL('/api/service/request', baseUrl), {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(createServiceRequest(request)),
    signal,
  });

  if (!response.ok) {
    throw new Error(`agent-browser service request failed: ${response.status}`);
  }

  return response.json();
}

/**
 * @param {ServiceTabRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceTab({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceTabRequest(request),
  });
}

/**
 * @param {ServiceCdpFreeLaunchRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceCdpFreeLaunch({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceCdpFreeLaunchRequest(request),
  });
}

/**
 * @param {Record<string, unknown>} accessPlan
 * @param {{ allowManualAction?: boolean }} options
 * @returns {Record<string, unknown>}
 */
function accessPlanServiceTabRequest(accessPlan, options = {}) {
  assertPlainObject(accessPlan, 'service access plan');
  const accessPlanRecord = /** @type {Record<string, unknown>} */ (accessPlan);
  const decision = accessPlanRecord.decision;
  assertPlainObject(decision, 'service access plan decision');
  const decisionRecord = /** @type {Record<string, unknown>} */ (decision);
  if (!options.allowManualAction && accessPlanRequiresManualSeeding(accessPlanRecord, decisionRecord)) {
    const handoff = accessPlanRecord.seedingHandoff;
    const command =
      handoff && typeof handoff === 'object' && !Array.isArray(handoff)
        ? /** @type {Record<string, unknown>} */ (handoff).command
        : undefined;
    throw new Error(
      `service access plan requires manual profile seeding before tab request${typeof command === 'string' ? `: ${command}` : ''}`,
    );
  }
  if (accessPlanRequiresCdpFree(decisionRecord)) {
    throw new Error(
      'service access plan requires CDP-free browser operation; use createServiceCdpFreeLaunchRequest for lifecycle-only launch tracking',
    );
  }
  const serviceRequest = decisionRecord.serviceRequest;
  assertPlainObject(serviceRequest, 'service access plan serviceRequest');
  const serviceRequestRecord = /** @type {Record<string, unknown>} */ (serviceRequest);
  const request = serviceRequestRecord.request;
  assertPlainObject(request, 'service access plan serviceRequest.request');
  const requestRecord = /** @type {Record<string, unknown>} */ (request);
  if (requestRecord.action !== 'tab_new') {
    throw new TypeError('service access plan serviceRequest.request action must be tab_new');
  }

  const { action: _action, ...tabRequest } = requestRecord;
  return tabRequest;
}

/**
 * @param {Record<string, unknown>} accessPlan
 * @param {{ allowManualAction?: boolean }} options
 * @returns {Record<string, unknown>}
 */
function accessPlanServiceCdpFreeLaunchRequest(accessPlan, options = {}) {
  assertPlainObject(accessPlan, 'service access plan');
  const accessPlanRecord = /** @type {Record<string, unknown>} */ (accessPlan);
  const decision = accessPlanRecord.decision;
  assertPlainObject(decision, 'service access plan decision');
  const decisionRecord = /** @type {Record<string, unknown>} */ (decision);
  if (!options.allowManualAction && accessPlanRequiresManualSeeding(accessPlanRecord, decisionRecord)) {
    const handoff = accessPlanRecord.seedingHandoff;
    const command =
      handoff && typeof handoff === 'object' && !Array.isArray(handoff)
        ? /** @type {Record<string, unknown>} */ (handoff).command
        : undefined;
    throw new Error(
      `service access plan requires manual profile seeding before CDP-free launch${typeof command === 'string' ? `: ${command}` : ''}`,
    );
  }
  if (!accessPlanRequiresCdpFree(decisionRecord)) {
    throw new Error('service access plan does not require CDP-free browser operation');
  }
  const serviceRequest = decisionRecord.serviceRequest;
  assertPlainObject(serviceRequest, 'service access plan serviceRequest');
  const serviceRequestRecord = /** @type {Record<string, unknown>} */ (serviceRequest);
  const request = serviceRequestRecord.request;
  assertPlainObject(request, 'service access plan serviceRequest.request');
  const requestRecord = /** @type {Record<string, unknown>} */ (request);
  const { action: _action, params: requestParams, ...launchRequest } = requestRecord;
  if (requestParams && typeof requestParams === 'object' && !Array.isArray(requestParams)) {
    launchRequest.params = { .../** @type {Record<string, unknown>} */ (requestParams) };
  }
  return launchRequest;
}

/**
 * @param {Record<string, unknown>} accessPlan
 * @param {Record<string, unknown>} decision
 */
function accessPlanRequiresManualSeeding(accessPlan, decision) {
  const readinessSummary = accessPlan.readinessSummary;
  const summaryRequiresSeeding =
    readinessSummary &&
    typeof readinessSummary === 'object' &&
    !Array.isArray(readinessSummary) &&
    /** @type {Record<string, unknown>} */ (readinessSummary).manualSeedingRequired === true;
  return (
    decision.manualSeedingRequired === true ||
    (summaryRequiresSeeding && accessPlan.seedingHandoff !== null && accessPlan.seedingHandoff !== undefined)
  );
}

/**
 * @param {Record<string, unknown>} decision
 */
function accessPlanRequiresCdpFree(decision) {
  const launchPosture = decision.launchPosture;
  const postureRequiresCdpFree =
    launchPosture &&
    typeof launchPosture === 'object' &&
    !Array.isArray(launchPosture) &&
    /** @type {Record<string, unknown>} */ (launchPosture).requiresCdpFree === true &&
    /** @type {Record<string, unknown>} */ (launchPosture).cdpAttachmentAllowed !== true;
  const serviceRequest = decision.serviceRequest;
  const serviceRequestRequiresCdpFree =
    serviceRequest &&
    typeof serviceRequest === 'object' &&
    !Array.isArray(serviceRequest) &&
    /** @type {Record<string, unknown>} */ (serviceRequest).requiresCdpFree === true &&
    /** @type {Record<string, unknown>} */ (serviceRequest).cdpAttachmentAllowed !== true;
  return postureRequiresCdpFree || serviceRequestRequiresCdpFree;
}

/**
 * @param {unknown} summary
 * @param {{ allowMonitorFreshnessRisk?: boolean, actionLabel: string }} options
 */
function assertMonitorRunDueSummarySafe(summary, options) {
  if (summary === undefined || summary === null || options.allowMonitorFreshnessRisk === true) {
    return;
  }
  assertPlainObject(summary, 'monitor run-due summary');
  const record = /** @type {Record<string, unknown>} */ (summary);
  const expiredTargetServiceIds = stringArray(record.expiredTargetServiceIds);
  const unverifiedTargetServiceIds = stringArray(record.unverifiedTargetServiceIds);
  const recommendedAction =
    typeof record.recommendedAction === 'string' ? record.recommendedAction : 'inspect_monitor_results';
  const matched = typeof record.matched === 'number' ? record.matched : 0;
  const failed = record.failed === true;
  if (expiredTargetServiceIds.length > 0) {
    throw new Error(
      `service monitor run-due found expired profile freshness before ${options.actionLabel}: ${expiredTargetServiceIds.join(',')}`,
    );
  }
  if (unverifiedTargetServiceIds.length > 0) {
    throw new Error(
      `service monitor run-due could not verify profile freshness before ${options.actionLabel}: ${unverifiedTargetServiceIds.join(',')}`,
    );
  }
  if (matched === 0 || (failed && recommendedAction !== 'use_selected_profile')) {
    throw new Error(
      `service monitor run-due requires inspection before ${options.actionLabel}: ${recommendedAction}`,
    );
  }
}

/**
 * @param {unknown} value
 */
function stringArray(value) {
  return Array.isArray(value) ? value.filter((item) => typeof item === 'string') : [];
}

/**
 * @param {unknown} value
 * @param {string} label
 */
function assertPlainObject(value, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
}
