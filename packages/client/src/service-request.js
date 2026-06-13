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
const displayIsolationSet = new Set([
  'private_virtual_display',
  'shared_display',
  'ambient_display',
]);

/**
 * @typedef {import('./service-request.generated.js').ServiceRequest} ServiceRequest
 * @typedef {import('./service-request.generated.js').ServiceRequestAction} ServiceRequestAction
 * @typedef {import('./service-request.generated.js').ServiceRequestHttpOptions} ServiceRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceRequestResponse} ServiceRequestResponse
 * @typedef {import('./service-request.generated.js').ServiceCdpFreeLaunchAvailability} ServiceCdpFreeLaunchAvailability
 * @typedef {import('./service-request.generated.js').ServiceCdpFreeLaunchData} ServiceCdpFreeLaunchData
 * @typedef {import('./service-request.generated.js').ServiceCdpAttachRequestHttpOptions} ServiceCdpAttachRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpAttachRequestOptions} ServiceCdpAttachRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpDetachRequestHttpOptions} ServiceCdpDetachRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpDetachRequestOptions} ServiceCdpDetachRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceEvaluateRequestHttpOptions} ServiceEvaluateRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceEvaluateRequestOptions} ServiceEvaluateRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceTabAccessPlan} ServiceTabAccessPlan
 * @typedef {import('./service-request.generated.js').ServiceCdpFreeLaunchRequestHttpOptions} ServiceCdpFreeLaunchRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpFreeLaunchRequestOptions} ServiceCdpFreeLaunchRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceControllerLeaseTakeoverHttpOptions} ServiceControllerLeaseTakeoverHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceControllerLeaseTakeoverOptions} ServiceControllerLeaseTakeoverOptions
 * @typedef {import('./service-request.generated.js').ServiceRemoteViewRouteCheckoutHttpOptions} ServiceRemoteViewRouteCheckoutHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceRemoteViewRouteCheckoutOptions} ServiceRemoteViewRouteCheckoutOptions
 * @typedef {import('./service-request.generated.js').ServiceRemoteViewRouteReleaseHttpOptions} ServiceRemoteViewRouteReleaseHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceRemoteViewRouteReleaseOptions} ServiceRemoteViewRouteReleaseOptions
 * @typedef {import('./service-request.generated.js').ServiceRoutePoolRepairHttpOptions} ServiceRoutePoolRepairHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceRoutePoolRepairOptions} ServiceRoutePoolRepairOptions
 * @typedef {import('./service-request.generated.js').ServiceTabHandle} ServiceTabHandle
 * @typedef {import('./service-request.generated.js').ServiceTabRequestHttpOptions} ServiceTabRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceTabRequestOptions} ServiceTabRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceViewerLeaseHeartbeatHttpOptions} ServiceViewerLeaseHeartbeatHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceViewerLeaseHeartbeatOptions} ServiceViewerLeaseHeartbeatOptions
 * @typedef {import('./service-request.generated.js').ServiceViewerLeaseReleaseHttpOptions} ServiceViewerLeaseReleaseHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceViewerLeaseReleaseOptions} ServiceViewerLeaseReleaseOptions
 * @typedef {import('./service-request.generated.js').ServiceViewerLeaseRequestHttpOptions} ServiceViewerLeaseRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceViewerLeaseRequestOptions} ServiceViewerLeaseRequestOptions
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
  if (Object.hasOwn(input, 'displayIsolation') && !displayIsolationSet.has(String(record.displayIsolation))) {
    throw new TypeError(
      'service request displayIsolation must be private_virtual_display, shared_display, or ambient_display',
    );
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
  const { params: plannedParams, ...plannedRequestFields } = plannedRequest;

  const tabParams = { ...plainRecordOrEmpty(plannedParams), ...(params ?? {}) };
  if (url !== undefined) {
    tabParams.url = url;
  }

  return createServiceRequest({
    ...plannedRequestFields,
    ...request,
    action: 'tab_new',
    ...(accessPlan !== undefined && allowManualAction === true ? { allowManualAction: true } : {}),
    ...(monitorRunDueSummary !== undefined && monitorRunDueSummary !== null ? { monitorRunDueSummary } : {}),
    ...(allowMonitorFreshnessRisk === true ? { allowMonitorFreshnessRisk: true } : {}),
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
 * Extract the service-owned tab handle from a tab response or data object.
 *
 * @param {unknown} response
 * @returns {ServiceTabHandle | null}
 */
export function getServiceTabHandle(response) {
  if (!response || typeof response !== 'object') {
    return null;
  }
  const record = /** @type {Record<string, unknown>} */ (response);
  if (isServiceTabHandle(record.serviceTabHandle)) {
    return /** @type {ServiceTabHandle} */ (record.serviceTabHandle);
  }
  const data = record.data;
  if (data && typeof data === 'object') {
    const dataRecord = /** @type {Record<string, unknown>} */ (data);
    if (isServiceTabHandle(dataRecord.serviceTabHandle)) {
      return /** @type {ServiceTabHandle} */ (dataRecord.serviceTabHandle);
    }
  }
  return null;
}

/**
 * Extract a usable service-owned tab handle or throw with the stale reason.
 *
 * @param {unknown} response
 * @returns {ServiceTabHandle}
 */
export function requireServiceTabHandle(response) {
  const handle = getServiceTabHandle(response);
  if (!handle) {
    throw new TypeError('service tab response did not include serviceTabHandle');
  }
  if (handle.valid === false) {
    const reason = typeof handle.staleReason === 'string' ? `: ${handle.staleReason}` : '';
    throw new TypeError(`service tab handle is stale${reason}`);
  }
  return handle;
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
  const { params: plannedParams, ...plannedRequestFields } = plannedRequest;

  const launchParams = { ...plainRecordOrEmpty(plannedParams), ...(params ?? {}) };
  if (url !== undefined) {
    launchParams.url = url;
  }

  return createServiceRequest({
    ...plannedRequestFields,
    ...request,
    action: 'cdp_free_launch',
    requiresCdpFree: true,
    cdpAttachmentAllowed: false,
    ...(accessPlan !== undefined && allowManualAction === true ? { allowManualAction: true } : {}),
    ...(monitorRunDueSummary !== undefined && monitorRunDueSummary !== null ? { monitorRunDueSummary } : {}),
    ...(allowMonitorFreshnessRisk === true ? { allowMonitorFreshnessRisk: true } : {}),
    ...(url !== undefined ? { url } : {}),
    ...(Object.keys(launchParams).length > 0 ? { params: launchParams } : {}),
  });
}

/**
 * Builds a policy-gated CDP attach descriptor request for a leased service tab.
 *
 * @param {ServiceCdpAttachRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceCdpAttachRequest(input) {
  assertPlainObject(input, 'service CDP attach request');
  const { serviceTabHandle, params, ...request } = input;
  const handle = requireServiceTabHandle({ serviceTabHandle });
  if (params !== undefined) {
    assertPlainObject(params, 'service CDP attach request params');
  }
  if (request.cdpAttachmentAllowed !== true) {
    throw new TypeError('service CDP attach request requires cdpAttachmentAllowed=true');
  }
  if (request.requiresCdpFree === true) {
    throw new TypeError('service CDP attach request cannot run when requiresCdpFree=true');
  }
  if (!handle.targetId) {
    throw new TypeError('service CDP attach request requires serviceTabHandle.targetId');
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'cdp_attach',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    ...(targetId !== undefined && targetId !== null ? { targetId } : {}),
    cdpAttachmentAllowed: true,
    serviceTabHandle: handle,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * Builds a CDP detach marker request without closing the browser.
 *
 * @param {ServiceCdpDetachRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceCdpDetachRequest(input) {
  assertPlainObject(input, 'service CDP detach request');
  const { serviceTabHandle, params, ...request } = input;
  const handle = requireServiceTabHandle({ serviceTabHandle });
  if (params !== undefined) {
    assertPlainObject(params, 'service CDP detach request params');
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'cdp_detach',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    ...(targetId !== undefined && targetId !== null ? { targetId } : {}),
    serviceTabHandle: handle,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * Builds a bounded evaluate request against a leased service tab.
 *
 * @param {ServiceEvaluateRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceEvaluateRequest(input) {
  assertPlainObject(input, 'service evaluate request');
  const { serviceTabHandle, script, expression, params, ...request } = input;
  const handle = requireServiceTabHandle({ serviceTabHandle });
  if (params !== undefined) {
    assertPlainObject(params, 'service evaluate request params');
  }
  const source = script ?? expression;
  if (typeof source !== 'string' || source.length === 0) {
    throw new TypeError('service evaluate request requires script or expression');
  }
  if (request.returnByValue === false) {
    throw new TypeError('service evaluate request requires returnByValue=true');
  }
  if (!Number.isInteger(request.timeoutMs) || Number(request.timeoutMs) < 1) {
    throw new TypeError('service evaluate request requires positive timeoutMs');
  }
  if (!Number.isInteger(request.maxReturnBytes) || Number(request.maxReturnBytes) < 1) {
    throw new TypeError('service evaluate request requires positive maxReturnBytes');
  }
  if (!handle.targetId) {
    throw new TypeError('service evaluate request requires serviceTabHandle.targetId');
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'evaluate',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    targetId,
    script: source,
    returnByValue: true,
    timeoutMs: request.timeoutMs,
    maxReturnBytes: request.maxReturnBytes,
    serviceTabHandle: handle,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * @param {ServiceRemoteViewRouteCheckoutOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceRemoteViewRouteCheckoutRequest(input) {
  assertPlainObject(input, 'remote-view route checkout request');
  const { params, ...request } = input;
  const checkoutParams = mergeParams(params, request, [
    'displayAllocationId',
    'routeId',
    'remoteViewRouteId',
    'routePoolEntryId',
    'browserId',
    'sessionName',
    'streamId',
    'provider',
    'providerMode',
    'frameUrl',
    'externalUrl',
    'connectionId',
    'connectionName',
  ]);
  return createServiceRequest({
    ...request,
    action: 'service_remote_view_route_checkout',
    params: checkoutParams,
  });
}

/**
 * @param {ServiceRemoteViewRouteReleaseOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceRemoteViewRouteReleaseRequest(input) {
  assertPlainObject(input, 'remote-view route release request');
  const { params, ...request } = input;
  return createServiceRequest({
    ...request,
    action: 'service_remote_view_route_release',
    params: mergeParams(params, request, ['routeId']),
  });
}

/**
 * @param {ServiceRoutePoolRepairOptions} [input]
 * @returns {ServiceRequest}
 */
export function createServiceRoutePoolRepairRequest(input = {}) {
  assertPlainObject(input, 'route-pool repair request');
  const { params, ...request } = input;
  return createServiceRequest({
    ...request,
    action: 'service_route_pool_repair',
    params: mergeParams(params, request, ['apply', 'staleCheckouts', 'serviceState']),
  });
}

/**
 * @param {ServiceViewerLeaseRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceViewerLeaseRequest(input) {
  assertPlainObject(input, 'viewer lease request');
  const { params, ...request } = input;
  return createServiceRequest({
    ...request,
    action: 'service_viewer_lease_request',
    params: mergeParams(params, request, [
      'routeId',
      'viewerId',
      'viewerName',
      'viewerRole',
      'openMode',
      'browserId',
      'expiresAt',
    ]),
  });
}

/**
 * @param {ServiceViewerLeaseHeartbeatOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceViewerLeaseHeartbeatRequest(input) {
  assertPlainObject(input, 'viewer lease heartbeat request');
  const { params, ...request } = input;
  return createServiceRequest({
    ...request,
    action: 'service_viewer_lease_heartbeat',
    params: mergeParams(params, request, ['viewerLeaseId', 'expiresAt']),
  });
}

/**
 * @param {ServiceViewerLeaseReleaseOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceViewerLeaseReleaseRequest(input) {
  assertPlainObject(input, 'viewer lease release request');
  const { params, ...request } = input;
  return createServiceRequest({
    ...request,
    action: 'service_viewer_lease_release',
    params: mergeParams(params, request, ['viewerLeaseId']),
  });
}

/**
 * @param {ServiceControllerLeaseTakeoverOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceControllerLeaseTakeoverRequest(input) {
  assertPlainObject(input, 'controller lease takeover request');
  const { params, ...request } = input;
  return createServiceRequest({
    ...request,
    action: 'service_controller_lease_takeover',
    params: mergeParams(params, request, [
      'routeId',
      'viewerLeaseId',
      'viewerId',
      'viewerName',
      'openMode',
      'browserId',
      'expiresAt',
    ]),
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
 * @param {ServiceCdpAttachRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceCdpAttach({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceCdpAttachRequest(request),
  });
}

/**
 * @param {ServiceCdpDetachRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceCdpDetach({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceCdpDetachRequest(request),
  });
}

/**
 * @param {ServiceEvaluateRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceEvaluate({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceEvaluateRequest(request),
  });
}

/**
 * @param {ServiceRemoteViewRouteCheckoutHttpOptions} options
 */
export async function requestServiceRemoteViewRouteCheckout({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceRemoteViewRouteCheckoutRequest(request),
  });
}

/**
 * @param {ServiceRemoteViewRouteReleaseHttpOptions} options
 */
export async function requestServiceRemoteViewRouteRelease({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceRemoteViewRouteReleaseRequest(request),
  });
}

/**
 * @param {ServiceRoutePoolRepairHttpOptions} options
 */
export async function requestServiceRoutePoolRepair({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceRoutePoolRepairRequest(request),
  });
}

/**
 * @param {ServiceViewerLeaseRequestHttpOptions} options
 */
export async function requestServiceViewerLease({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceViewerLeaseRequest(request),
  });
}

/**
 * @param {ServiceViewerLeaseHeartbeatHttpOptions} options
 */
export async function heartbeatServiceViewerLease({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceViewerLeaseHeartbeatRequest(request),
  });
}

/**
 * @param {ServiceViewerLeaseReleaseHttpOptions} options
 */
export async function releaseServiceViewerLease({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceViewerLeaseReleaseRequest(request),
  });
}

/**
 * @param {ServiceControllerLeaseTakeoverHttpOptions} options
 */
export async function takeoverServiceControllerLease({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceControllerLeaseTakeoverRequest(request),
  });
}

/**
 * Converts a CDP-free launch response into a serializable command availability
 * summary for API, MCP, and dashboard clients.
 *
 * @param {ServiceCdpFreeLaunchData} data
 * @returns {ServiceCdpFreeLaunchAvailability}
 */
export function summarizeServiceCdpFreeLaunchAvailability(data) {
  assertPlainObject(data, 'CDP-free launch data');
  const record = /** @type {Record<string, unknown>} */ (/** @type {unknown} */ (data));
  const hasUnsupportedCommandList = Array.isArray(record.unsupportedCommands);
  const unsupportedCommands = serviceRequestActionArray(record.unsupportedCommands);
  const unsupportedCommandSet = new Set(unsupportedCommands);
  const availableCommands = hasUnsupportedCommandList
    ? SERVICE_REQUEST_ACTIONS.filter((action) => !unsupportedCommandSet.has(action))
    : /** @type {ServiceRequestAction[]} */ (['cdp_free_launch']);

  return {
    controlPlaneMode: 'cdp_free',
    lifecycleOnly: true,
    cdpAttachmentAllowed: record.cdpAttachmentAllowed === true,
    supportedOperations: stringArray(record.supportedOperations),
    unsupportedOperations: stringArray(record.unsupportedOperations),
    unsupportedCommands,
    availableCommands,
    hasUnsupportedCommandList,
  };
}

/**
 * @param {ServiceCdpFreeLaunchData} data
 * @param {ServiceRequestAction} action
 * @returns {boolean}
 */
export function isServiceCdpFreeActionAvailable(data, action) {
  if (!actionSet.has(action)) {
    return false;
  }
  return summarizeServiceCdpFreeLaunchAvailability(data).availableCommands.includes(action);
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
 * @returns {ServiceRequestAction[]}
 */
function serviceRequestActionArray(value) {
  return stringArray(value)
    .filter((action) => actionSet.has(/** @type {ServiceRequestAction} */ (action)))
    .map((action) => /** @type {ServiceRequestAction} */ (action));
}

/**
 * @param {unknown} value
 */
function isServiceTabHandle(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return false;
  }
  const record = /** @type {Record<string, unknown>} */ (value);
  return (
    typeof record.browserId === 'string' &&
    typeof record.tabId === 'string' &&
    typeof record.valid === 'boolean'
  );
}

/**
 * @param {Record<string, unknown> | undefined} params
 * @param {Record<string, unknown>} request
 * @param {string[]} keys
 * @returns {Record<string, unknown>}
 */
function mergeParams(params, request, keys) {
  if (params !== undefined) {
    assertPlainObject(params, 'service request params');
  }
  const merged = { ...(params ?? {}) };
  for (const key of keys) {
    if (Object.hasOwn(request, key) && request[key] !== undefined) {
      merged[key] = request[key];
      delete request[key];
    }
  }
  return merged;
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

/**
 * @param {unknown} value
 * @returns {Record<string, unknown>}
 */
function plainRecordOrEmpty(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {};
  }
  return /** @type {Record<string, unknown>} */ (value);
}
