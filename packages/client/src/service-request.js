// @ts-check

import {
  SERVICE_REQUEST_ACTIONS,
  SERVICE_REQUEST_BOOLEAN_FIELDS,
  SERVICE_REQUEST_INTEGER_FIELDS,
  SERVICE_REQUEST_MCP_TOOL_NAME,
  SERVICE_REQUEST_STRING_ARRAY_FIELDS,
  SERVICE_REQUEST_STRING_FIELDS,
} from './service-request.generated.js';

const actionSet = new Set(SERVICE_REQUEST_ACTIONS);

/**
 * @typedef {import('./service-request.generated.js').ServiceRequest} ServiceRequest
 * @typedef {import('./service-request.generated.js').ServiceRequestHttpOptions} ServiceRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceRequestResponse} ServiceRequestResponse
 * @typedef {import('./service-request.generated.js').ServiceTabAccessPlan} ServiceTabAccessPlan
 * @typedef {import('./service-request.generated.js').ServiceTabRequestHttpOptions} ServiceTabRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceTabRequestOptions} ServiceTabRequestOptions
 */

export {
  SERVICE_REQUEST_ACTIONS,
  SERVICE_REQUEST_BOOLEAN_FIELDS,
  SERVICE_REQUEST_INTEGER_FIELDS,
  SERVICE_REQUEST_MCP_TOOL_NAME,
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
  const { accessPlan, allowManualAction, url, params, ...request } = input;
  if (url !== undefined && typeof url !== 'string') {
    throw new TypeError('service tab request url must be a string');
  }
  if (allowManualAction !== undefined && typeof allowManualAction !== 'boolean') {
    throw new TypeError('service tab request allowManualAction must be a boolean');
  }
  if (params !== undefined) {
    assertPlainObject(params, 'service tab request params');
  }
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
 * @param {unknown} value
 * @param {string} label
 */
function assertPlainObject(value, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
}
