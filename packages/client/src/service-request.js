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
 * @typedef {import('./service-request.generated.js').ServiceExternalByopAdoptRequestHttpOptions} ServiceExternalByopAdoptRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceExternalByopAdoptRequestOptions} ServiceExternalByopAdoptRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpAttachRequestHttpOptions} ServiceCdpAttachRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpAttachRequestOptions} ServiceCdpAttachRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpDetachRequestHttpOptions} ServiceCdpDetachRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpDetachRequestOptions} ServiceCdpDetachRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceEvaluateRequestHttpOptions} ServiceEvaluateRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceEvaluateRequestOptions} ServiceEvaluateRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceDiagnosticsRequestHttpOptions} ServiceDiagnosticsRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceDiagnosticsRequestOptions} ServiceDiagnosticsRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceProbeRequestHttpOptions} ServiceProbeRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceProbeRequestOptions} ServiceProbeRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceUiActionRequestHttpOptions} ServiceUiActionRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceUiActionRequestOptions} ServiceUiActionRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceNetworkCaptureRequestHttpOptions} ServiceNetworkCaptureRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceNetworkCaptureRequestOptions} ServiceNetworkCaptureRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceFileTransferRequestHttpOptions} ServiceFileTransferRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceFileTransferRequestOptions} ServiceFileTransferRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceTabHandleRefreshHttpOptions} ServiceTabHandleRefreshHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceTabHandleRefreshOptions} ServiceTabHandleRefreshOptions
 * @typedef {import('./service-request.generated.js').ServiceTabHandleReleaseHttpOptions} ServiceTabHandleReleaseHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceTabHandleReleaseOptions} ServiceTabHandleReleaseOptions
 * @typedef {import('./service-request.generated.js').ServiceTabAccessPlan} ServiceTabAccessPlan
 * @typedef {import('./service-request.generated.js').ServiceCdpFreeLaunchRequestHttpOptions} ServiceCdpFreeLaunchRequestHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceCdpFreeLaunchRequestOptions} ServiceCdpFreeLaunchRequestOptions
 * @typedef {import('./service-request.generated.js').ServiceControllerLeaseTakeoverHttpOptions} ServiceControllerLeaseTakeoverHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceControllerLeaseTakeoverOptions} ServiceControllerLeaseTakeoverOptions
 * @typedef {import('./service-request.generated.js').ServiceRemoteViewRouteCheckoutHttpOptions} ServiceRemoteViewRouteCheckoutHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceRemoteViewRouteCheckoutOptions} ServiceRemoteViewRouteCheckoutOptions
 * @typedef {import('./service-request.generated.js').ServiceRemoteViewBrowserReattachHttpOptions} ServiceRemoteViewBrowserReattachHttpOptions
 * @typedef {import('./service-request.generated.js').ServiceRemoteViewBrowserReattachOptions} ServiceRemoteViewBrowserReattachOptions
 * @typedef {import('./service-request.generated.js').ServiceRemoteViewOpenProofSummary} ServiceRemoteViewOpenProofSummary
 * @typedef {import('./service-request.generated.js').ServiceSharedProfileAcquisitionSummary} ServiceSharedProfileAcquisitionSummary
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
    const tabRecord = recordFromUnknown(dataRecord.tab);
    if (isServiceTabHandle(tabRecord?.serviceTabHandle)) {
      return /** @type {ServiceTabHandle} */ (tabRecord.serviceTabHandle);
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
 * Extract a service-owned tab handle for refresh. Unlike requireServiceTabHandle,
 * this accepts stale handles so the daemon can classify or repair them.
 *
 * @param {unknown} response
 * @returns {ServiceTabHandle}
 */
function requireRefreshableServiceTabHandle(response) {
  const handle = getServiceTabHandle(response);
  if (!handle) {
    throw new TypeError('service tab handle refresh request requires serviceTabHandle');
  }
  if (typeof handle.tabId !== 'string' || handle.tabId.length === 0) {
    throw new TypeError('service tab handle refresh request requires serviceTabHandle.tabId');
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
 * Builds an explicit adoption request for a registered external_byop profile
 * and caller-supplied Chrome DevTools endpoint.
 *
 * @param {ServiceExternalByopAdoptRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceExternalByopAdoptRequest(input) {
  assertPlainObject(input, 'external BYOP adopt request');
  const { profileId, runtimeProfile, cdpUrl, cdpPort, url, params, ...request } = input;
  const effectiveProfileId = runtimeProfile ?? profileId;
  if (typeof effectiveProfileId !== 'string' || effectiveProfileId.trim().length === 0) {
    throw new TypeError('external BYOP adopt request requires runtimeProfile or profileId');
  }
  if (cdpUrl !== undefined && typeof cdpUrl !== 'string') {
    throw new TypeError('external BYOP adopt request cdpUrl must be a string');
  }
  if (cdpPort !== undefined && (!Number.isInteger(cdpPort) || cdpPort < 1)) {
    throw new TypeError('external BYOP adopt request cdpPort must be a positive integer');
  }
  const hasCdpUrl = typeof cdpUrl === 'string' && cdpUrl.trim().length > 0;
  const hasCdpPort = cdpPort !== undefined;
  if (hasCdpUrl === hasCdpPort) {
    throw new TypeError('external BYOP adopt request requires exactly one of cdpUrl or cdpPort');
  }
  if (url !== undefined && typeof url !== 'string') {
    throw new TypeError('external BYOP adopt request url must be a string');
  }
  if (params !== undefined) {
    assertPlainObject(params, 'external BYOP adopt request params');
  }
  return createServiceRequest({
    ...request,
    action: 'external_byop_adopt',
    runtimeProfile: effectiveProfileId,
    ...(profileId !== undefined ? { profileId } : {}),
    ...(hasCdpUrl ? { cdpUrl } : {}),
    ...(hasCdpPort ? { cdpPort } : {}),
    ...(url !== undefined ? { url } : {}),
    ...(params !== undefined ? { params } : {}),
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
 * Builds a compact diagnostic bundle request for a leased service tab.
 *
 * @param {ServiceDiagnosticsRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceDiagnosticsRequest(input) {
  assertPlainObject(input, 'service diagnostics request');
  const { serviceTabHandle, params, ...request } = input;
  const handle = requireServiceTabHandle({ serviceTabHandle });
  if (params !== undefined) {
    assertPlainObject(params, 'service diagnostics request params');
  }
  for (const field of ['maxConsoleEntries', 'maxErrorEntries', 'maxRequestEntries']) {
    const value = /** @type {Record<string, unknown>} */ (request)[field];
    if (value !== undefined && (!Number.isInteger(value) || Number(value) < 1)) {
      throw new TypeError(`service diagnostics request ${field} must be a positive integer`);
    }
  }
  if (request.includeScreenshot !== undefined && typeof request.includeScreenshot !== 'boolean') {
    throw new TypeError('service diagnostics request includeScreenshot must be a boolean');
  }
  if (request.screenshotDir !== undefined && typeof request.screenshotDir !== 'string') {
    throw new TypeError('service diagnostics request screenshotDir must be a string');
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'diagnostics',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    ...(targetId !== undefined && targetId !== null ? { targetId } : {}),
    serviceTabHandle: handle,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * Builds a provider-neutral probe request against a leased service tab.
 *
 * @param {ServiceProbeRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceProbeRequest(input) {
  assertPlainObject(input, 'service probe request');
  const { serviceTabHandle, probe, params, ...request } = input;
  const handle = requireServiceTabHandle({ serviceTabHandle });
  assertPlainObject(probe, 'service probe request probe');
  if (params !== undefined) {
    assertPlainObject(params, 'service probe request params');
  }
  const probeRecord = /** @type {Record<string, unknown>} */ (probe);
  const detectors = probeRecord.detectors;
  if (!Array.isArray(detectors) || detectors.length === 0) {
    throw new TypeError('service probe request requires probe.detectors array');
  }
  const timeoutMs = request.timeoutMs ?? probeRecord.timeoutMs;
  const maxReturnBytes = request.maxReturnBytes ?? probeRecord.maxReturnBytes;
  if (!Number.isInteger(timeoutMs) || Number(timeoutMs) < 1) {
    throw new TypeError('service probe request requires positive timeoutMs');
  }
  if (!Number.isInteger(maxReturnBytes) || Number(maxReturnBytes) < 1) {
    throw new TypeError('service probe request requires positive maxReturnBytes');
  }
  if (!handle.targetId) {
    throw new TypeError('service probe request requires serviceTabHandle.targetId');
  }
  const recordFreshness = /** @type {Record<string, unknown> | undefined} */ (probeRecord.recordFreshness);
  if (recordFreshness !== undefined) {
    assertPlainObject(recordFreshness, 'service probe request probe.recordFreshness');
    if (typeof recordFreshness.targetServiceId !== 'string' || recordFreshness.targetServiceId.length === 0) {
      throw new TypeError('service probe request probe.recordFreshness requires targetServiceId');
    }
    if (typeof recordFreshness.accountId !== 'string' || recordFreshness.accountId.length === 0) {
      throw new TypeError('service probe request probe.recordFreshness requires accountId');
    }
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'probe',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    targetId,
    timeoutMs: Number(timeoutMs),
    maxReturnBytes: Number(maxReturnBytes),
    serviceTabHandle: handle,
    probe,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * Builds a provider-neutral UI action recipe request against a leased service tab.
 *
 * @param {ServiceUiActionRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceUiActionRequest(input) {
  assertPlainObject(input, 'service UI action request');
  const { serviceTabHandle, uiAction, params, ...request } = input;
  const handle = requireServiceTabHandle({ serviceTabHandle });
  assertPlainObject(uiAction, 'service UI action request uiAction');
  if (params !== undefined) {
    assertPlainObject(params, 'service UI action request params');
  }
  const uiActionRecord = /** @type {Record<string, unknown>} */ (uiAction);
  const steps = uiActionRecord.steps;
  if (!Array.isArray(steps) || steps.length === 0) {
    throw new TypeError('service UI action request requires uiAction.steps array');
  }
  const timeoutMs = request.timeoutMs ?? uiActionRecord.timeoutMs;
  if (!Number.isInteger(timeoutMs) || Number(timeoutMs) < 1) {
    throw new TypeError('service UI action request requires positive timeoutMs');
  }
  const maxTextBytes = request.maxTextBytes ?? uiActionRecord.maxTextBytes;
  if (maxTextBytes !== undefined && (!Number.isInteger(maxTextBytes) || Number(maxTextBytes) < 1)) {
    throw new TypeError('service UI action request maxTextBytes must be positive when supplied');
  }
  if (!handle.targetId) {
    throw new TypeError('service UI action request requires serviceTabHandle.targetId');
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'ui_action',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    targetId,
    timeoutMs: Number(timeoutMs),
    ...(maxTextBytes !== undefined ? { maxTextBytes: Number(maxTextBytes) } : {}),
    serviceTabHandle: handle,
    uiAction,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * Builds a provider-neutral network evidence capture request against a leased service tab.
 *
 * @param {ServiceNetworkCaptureRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceNetworkCaptureRequest(input) {
  assertPlainObject(input, 'service network capture request');
  const { serviceTabHandle, networkCapture, params, ...request } = input;
  const handle = requireServiceTabHandle({ serviceTabHandle });
  assertPlainObject(networkCapture, 'service network capture request networkCapture');
  if (params !== undefined) {
    assertPlainObject(params, 'service network capture request params');
  }
  const networkCaptureRecord = /** @type {Record<string, unknown>} */ (networkCapture);
  const timeoutMs = request.timeoutMs ?? networkCaptureRecord.timeoutMs ?? networkCaptureRecord.maxDurationMs;
  if (!Number.isInteger(timeoutMs) || Number(timeoutMs) < 1) {
    throw new TypeError('service network capture request requires positive timeoutMs');
  }
  const maxEvents = networkCaptureRecord.maxEvents;
  if (!Number.isInteger(maxEvents) || Number(maxEvents) < 1) {
    throw new TypeError('service network capture request requires positive networkCapture.maxEvents');
  }
  const captureBodies = networkCaptureRecord.captureBodies === true;
  const maxBodyBytes = request.maxBodyBytes ?? networkCaptureRecord.maxBodyBytes;
  if (captureBodies && (!Number.isInteger(maxBodyBytes) || Number(maxBodyBytes) < 1)) {
    throw new TypeError('service network capture request captureBodies requires positive maxBodyBytes');
  }
  if (maxBodyBytes !== undefined && (!Number.isInteger(maxBodyBytes) || Number(maxBodyBytes) < 1)) {
    throw new TypeError('service network capture request maxBodyBytes must be positive when supplied');
  }
  if (!handle.targetId) {
    throw new TypeError('service network capture request requires serviceTabHandle.targetId');
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'network_capture',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    targetId,
    timeoutMs: Number(timeoutMs),
    ...(maxBodyBytes !== undefined ? { maxBodyBytes: Number(maxBodyBytes) } : {}),
    serviceTabHandle: handle,
    networkCapture,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * Builds a provider-neutral file input and download capture request against a leased service tab.
 *
 * @param {ServiceFileTransferRequestOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceFileTransferRequest(input) {
  assertPlainObject(input, 'service file transfer request');
  const { serviceTabHandle, fileTransfer, params, ...request } = input;
  const handle = requireServiceTabHandle({ serviceTabHandle });
  assertPlainObject(fileTransfer, 'service file transfer request fileTransfer');
  if (params !== undefined) {
    assertPlainObject(params, 'service file transfer request params');
  }
  const fileTransferRecord = /** @type {Record<string, unknown>} */ (fileTransfer);
  const timeoutMs = request.timeoutMs ?? fileTransferRecord.timeoutMs;
  if (!Number.isInteger(timeoutMs) || Number(timeoutMs) < 1) {
    throw new TypeError('service file transfer request requires positive timeoutMs');
  }
  if (fileTransferRecord.upload === undefined && fileTransferRecord.download === undefined) {
    throw new TypeError('service file transfer request requires upload or download recipe');
  }
  if (fileTransferRecord.upload !== undefined) {
    assertFileTransferUploadRecipe(fileTransferRecord.upload);
  }
  if (fileTransferRecord.download !== undefined) {
    assertFileTransferDownloadRecipe(fileTransferRecord.download);
  }
  if (!handle.targetId) {
    throw new TypeError('service file transfer request requires serviceTabHandle.targetId');
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'file_transfer',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    targetId,
    timeoutMs: Number(timeoutMs),
    serviceTabHandle: handle,
    fileTransfer,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * @param {unknown} upload
 */
function assertFileTransferUploadRecipe(upload) {
  assertPlainObject(upload, 'service file transfer upload');
  const record = /** @type {Record<string, unknown>} */ (upload);
  if (typeof record.selector !== 'string' && typeof record.labelText !== 'string' && typeof record.label !== 'string') {
    throw new TypeError('service file transfer upload requires selector or labelText');
  }
  const files = record.files;
  if (!Array.isArray(files) || files.length === 0 || files.some((file) => typeof file !== 'string' || file.length === 0)) {
    throw new TypeError('service file transfer upload requires files array');
  }
  const maxFiles = record.maxFiles;
  if (!Number.isInteger(maxFiles) || Number(maxFiles) < 1) {
    throw new TypeError('service file transfer upload requires positive maxFiles');
  }
  if (files.length > Number(maxFiles)) {
    throw new TypeError('service file transfer upload file count exceeds maxFiles');
  }
  assertNonemptyStringArray(record.allowedPaths, 'service file transfer upload allowedPaths');
}

/**
 * @param {unknown} download
 */
function assertFileTransferDownloadRecipe(download) {
  assertPlainObject(download, 'service file transfer download');
  const record = /** @type {Record<string, unknown>} */ (download);
  if (typeof record.selector !== 'string' || record.selector.length === 0) {
    throw new TypeError('service file transfer download requires selector');
  }
  if (typeof record.directory !== 'string' || record.directory.length === 0) {
    throw new TypeError('service file transfer download requires directory');
  }
  assertNonemptyStringArray(record.allowedDirectories, 'service file transfer download allowedDirectories');
  if (record.maxBytes !== undefined && (!Number.isInteger(record.maxBytes) || Number(record.maxBytes) < 1)) {
    throw new TypeError('service file transfer download maxBytes must be positive when supplied');
  }
}

/**
 * @param {unknown} value
 * @param {string} label
 */
function assertNonemptyStringArray(value, label) {
  if (!Array.isArray(value) || value.length === 0 || value.some((item) => typeof item !== 'string' || item.length === 0)) {
    throw new TypeError(`${label} must be a nonempty string array`);
  }
}

/**
 * Builds a generic service-tab-handle refresh request.
 *
 * @param {ServiceTabHandleRefreshOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceTabHandleRefreshRequest(input) {
  assertPlainObject(input, 'service tab handle refresh request');
  const { serviceTabHandle, params, ...request } = input;
  const handle = requireRefreshableServiceTabHandle({ serviceTabHandle });
  if (params !== undefined) {
    assertPlainObject(params, 'service tab handle refresh request params');
  }
  const repairPolicy = request.repairPolicy ?? 'reject_only';
  if (!['reject_only', 'reuse_compatible', 'open_if_missing', 'replace_duplicates'].includes(repairPolicy)) {
    throw new TypeError(
      'service tab handle refresh request repairPolicy must be reject_only, reuse_compatible, open_if_missing, or replace_duplicates',
    );
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'tab_handle_refresh',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    ...(targetId !== undefined && targetId !== null ? { targetId } : {}),
    repairPolicy,
    serviceTabHandle: handle,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * Builds a generic service-tab-handle release request.
 *
 * @param {ServiceTabHandleReleaseOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceTabHandleReleaseRequest(input) {
  assertPlainObject(input, 'service tab handle release request');
  const { serviceTabHandle, params, ...request } = input;
  const handle = requireRefreshableServiceTabHandle({ serviceTabHandle });
  if (params !== undefined) {
    assertPlainObject(params, 'service tab handle release request params');
  }
  const sessionName = request.sessionName ?? handle.sessionName ?? handle.ownerSessionId;
  const targetId = request.targetId ?? handle.targetId;
  return createServiceRequest({
    ...request,
    action: 'tab_handle_release',
    browserId: request.browserId ?? handle.browserId,
    ...(sessionName !== undefined && sessionName !== null ? { sessionName } : {}),
    ...(targetId !== undefined && targetId !== null ? { targetId } : {}),
    serviceTabHandle: handle,
    ...(params !== undefined ? { params } : {}),
  });
}

/**
 * @param {ServiceRemoteViewRouteCheckoutOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceRemoteViewRoutePreflightRequest(input) {
  assertPlainObject(input, 'remote-view route preflight request');
  const { params, ...request } = input;
  const preflightParams = mergeParams(params, request, [
    'displayAllocationId',
    'routeId',
    'remoteViewRouteId',
    'routePoolEntryId',
    'routePoolEntry',
    'routePool',
    'browserId',
    'sessionName',
    'streamId',
    'viewStreamProvider',
    'provider',
    'providerMode',
    'frameUrl',
    'externalUrl',
    'connectionId',
    'connectionName',
  ]);
  return createServiceRequest({
    ...request,
    action: 'service_remote_view_route_preflight',
    params: preflightParams,
  });
}

/**
 * @param {ServiceRemoteViewRouteCheckoutOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceRemoteViewOpenRequest(input) {
  assertPlainObject(input, 'remote-view open request');
  const { params, allowInfrastructureOnlyReadiness: _allowInfrastructureOnlyReadiness, ...request } = input;
  const openParams = mergeParams(params, request, [
    'displayAllocationId',
    'routeId',
    'remoteViewRouteId',
    'routePoolEntryId',
    'routePoolEntry',
    'routePool',
    'browserId',
    'sessionName',
    'streamId',
    'viewStreamProvider',
    'provider',
    'providerMode',
    'frameUrl',
    'externalUrl',
    'connectionId',
    'connectionName',
    'routeDescriptor',
    'remoteHeadedDisplay',
    'display',
    'displayName',
    'url',
    'dryRun',
  ]);
  return createServiceRequest({
    ...request,
    action: 'remote_view_open',
    params: openParams,
  });
}

const REMOTE_VIEW_REATTACH_PARAM_FIELDS = [
  'browserId',
  'profileId',
  'sessionName',
  'displayAllocationId',
  'routeId',
  'remoteViewRouteId',
  'routePoolEntryId',
  'routePoolEntry',
  'routePool',
  'streamId',
  'viewStreamProvider',
  'provider',
  'providerMode',
  'frameUrl',
  'externalUrl',
  'connectionId',
  'connectionName',
  'routeDescriptor',
  'openMode',
  'viewerId',
  'viewerName',
  'viewerRole',
  'controllerTakeover',
];

/**
 * @param {ServiceRemoteViewBrowserReattachOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceRemoteViewBrowserReattachRequest(input) {
  assertPlainObject(input, 'remote-view browser reattach request');
  const { params, ...request } = input;
  return createServiceRequest({
    ...request,
    action: 'service_remote_view_browser_reattach',
    params: mergeParams(params, request, REMOTE_VIEW_REATTACH_PARAM_FIELDS),
  });
}

/**
 * @param {ServiceRemoteViewBrowserReattachOptions} input
 * @returns {ServiceRequest}
 */
export function createServiceRemoteViewRouteSwitchRequest(input) {
  assertPlainObject(input, 'remote-view route switch request');
  const { params, ...request } = input;
  return createServiceRequest({
    ...request,
    action: 'service_remote_view_route_switch',
    params: mergeParams(params, request, REMOTE_VIEW_REATTACH_PARAM_FIELDS),
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
 * Request a service-owned tab from an access-plan response.
 *
 * @param {ServiceTabRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceTabFromAccessPlan(options) {
  return requestServiceTab(options);
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
 * @param {ServiceExternalByopAdoptRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceExternalByopAdopt({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceExternalByopAdoptRequest(request),
  });
}

export const adoptExternalByopBrowser = requestServiceExternalByopAdopt;

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
 * Attach to a service-owned tab through the policy-gated CDP descriptor path.
 *
 * @param {ServiceCdpAttachRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function attachServiceTabCdp(options) {
  return requestServiceCdpAttach(options);
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
 * Run bounded JavaScript against a leased service tab handle.
 *
 * @param {ServiceEvaluateRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function evaluateServiceTab(options) {
  return requestServiceEvaluate(options);
}

/**
 * @param {ServiceDiagnosticsRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceDiagnostics({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceDiagnosticsRequest(request),
  });
}

/**
 * Collect compact diagnostics for a leased service tab handle.
 *
 * @param {ServiceDiagnosticsRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function getServiceTabDiagnostics(options) {
  return requestServiceDiagnostics(options);
}

/**
 * @param {ServiceProbeRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceProbe({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceProbeRequest(request),
  });
}

/**
 * Run a provider-neutral probe against a leased service tab handle.
 *
 * @param {ServiceProbeRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function probeServiceTab(options) {
  return requestServiceProbe(options);
}

/**
 * @param {ServiceUiActionRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceUiAction({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceUiActionRequest(request),
  });
}

/**
 * Run a provider-neutral UI action recipe against a leased service tab handle.
 *
 * @param {ServiceUiActionRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function runServiceUiAction(options) {
  return requestServiceUiAction(options);
}

/**
 * @param {ServiceNetworkCaptureRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceNetworkCapture({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceNetworkCaptureRequest(request),
  });
}

/**
 * Capture capped provider-neutral network evidence for a leased service tab handle.
 *
 * @param {ServiceNetworkCaptureRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function captureServiceNetwork(options) {
  return requestServiceNetworkCapture(options);
}

/**
 * @param {ServiceFileTransferRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceFileTransfer({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceFileTransferRequest(request),
  });
}

/**
 * Run a provider-neutral file input or download capture recipe against a leased service tab handle.
 *
 * @param {ServiceFileTransferRequestHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function transferServiceFiles(options) {
  return requestServiceFileTransfer(options);
}

/**
 * @param {ServiceTabHandleRefreshHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceTabHandleRefresh({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceTabHandleRefreshRequest(request),
  });
}

/**
 * Refresh, repair, or reject a leased service tab handle through agent-browser.
 *
 * @param {ServiceTabHandleRefreshHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function refreshServiceTabHandle(options) {
  return requestServiceTabHandleRefresh(options);
}

/**
 * @param {ServiceTabHandleReleaseHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function requestServiceTabHandleRelease({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceTabHandleReleaseRequest(request),
  });
}

/**
 * Release a leased service tab handle while preserving the retained browser.
 *
 * @param {ServiceTabHandleReleaseHttpOptions} options
 * @returns {Promise<ServiceRequestResponse>}
 */
export async function releaseServiceTabHandle(options) {
  return requestServiceTabHandleRelease(options);
}

/**
 * @param {ServiceRemoteViewRouteCheckoutHttpOptions} options
 */
export async function requestServiceRemoteViewRoutePreflight({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceRemoteViewRoutePreflightRequest(request),
  });
}

/**
 * @param {ServiceRemoteViewRouteCheckoutHttpOptions} options
 */
export async function requestServiceRemoteViewOpen({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  const { allowInfrastructureOnlyReadiness, ...requestFields } = request;
  const response = await postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceRemoteViewOpenRequest(requestFields),
  });
  requireServiceRemoteViewOpenOperatorVisible(response, {
    allowInfrastructureOnlyReadiness,
  });
  return response;
}

/**
 * @param {ServiceRemoteViewBrowserReattachHttpOptions} options
 */
export async function requestServiceRemoteViewBrowserReattach({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceRemoteViewBrowserReattachRequest(request),
  });
}

/**
 * @param {ServiceRemoteViewBrowserReattachHttpOptions} options
 */
export async function requestServiceRemoteViewRouteSwitch({ baseUrl, fetch = globalThis.fetch, signal, ...request }) {
  return postServiceRequest({
    baseUrl,
    fetch,
    signal,
    request: createServiceRemoteViewRouteSwitchRequest(request),
  });
}

/**
 * @param {unknown} response
 * @returns {Record<string, unknown> | null}
 */
function serviceRemoteViewOpenData(response) {
  if (!response || typeof response !== 'object') return null;
  const record = /** @type {Record<string, unknown>} */ (response);
  const data = record.data;
  if (data && typeof data === 'object') return /** @type {Record<string, unknown>} */ (data);
  return /** @type {Record<string, unknown>} */ (record);
}

/**
 * @param {unknown} response
 * @returns {Record<string, unknown> | null}
 */
export function getServiceRemoteViewOpenOperatorVisible(response) {
  const data = serviceRemoteViewOpenData(response);
  const operatorVisible = data?.operatorVisible;
  return operatorVisible && typeof operatorVisible === 'object'
    ? /** @type {Record<string, unknown>} */ (operatorVisible)
    : null;
}

/**
 * @param {unknown} response
 * @returns {boolean}
 */
export function isServiceRemoteViewOpenOperatorVisibleReady(response) {
  const operatorVisible = getServiceRemoteViewOpenOperatorVisible(response);
  return operatorVisible?.state === 'ready';
}

/**
 * Build a one-line route, tab, profile, and visual-proof summary for
 * route-bound operator handoff logs.
 *
 * @param {unknown} response
 * @returns {ServiceRemoteViewOpenProofSummary}
 */
export function summarizeServiceRemoteViewOpenProof(response) {
  const data = serviceRemoteViewOpenData(response);
  const operatorVisible = getServiceRemoteViewOpenOperatorVisible(response);
  const tab = recordFromUnknown(data?.tab) ?? recordFromUnknown(data?.tabNew);
  const serviceTabHandle = recordFromUnknown(data?.serviceTabHandle) ?? recordFromUnknown(tab?.serviceTabHandle);
  const proof = recordFromUnknown(operatorVisible?.proof);
  const target = recordFromUnknown(operatorVisible?.target);
  const components = recordFromUnknown(operatorVisible?.components);
  const routeComponent = recordFromUnknown(components?.route);
  const tabComponent = recordFromUnknown(components?.tab);
  const browserComponent = recordFromUnknown(components?.browser);
  const guacamoleComponent = recordFromUnknown(components?.guacamole);
  const browserBuildProof = recordFromUnknown(data?.browserBuildProof);
  const displayContent = recordFromUnknown(proof?.displayContent);
  const state = stringOrNull(operatorVisible?.state);
  const routeId = stringOrNull(operatorVisible?.routeId ?? data?.routeId ?? data?.remoteViewRouteId);
  const displayAllocationId = stringOrNull(operatorVisible?.displayAllocationId ?? data?.displayAllocationId);
  const displayName = stringOrNull(operatorVisible?.displayName ?? data?.displayName);
  const browserId = stringOrNull(operatorVisible?.browserId ?? data?.browserId ?? tab?.browserId ?? serviceTabHandle?.browserId);
  const sessionName = stringOrNull(operatorVisible?.sessionName ?? data?.sessionName ?? data?.sessionId ?? tab?.sessionId ?? serviceTabHandle?.sessionName);
  const tabId = stringOrNull(
    data?.tabId ??
      tab?.tabId ??
      tab?.id ??
      target?.targetId ??
      tabComponent?.targetId ??
      tab?.targetId ??
      serviceTabHandle?.tabId ??
      serviceTabHandle?.targetId,
  );
  const profileId = stringOrNull(
    data?.profileId ??
      data?.runtimeProfile ??
      target?.profileId ??
      browserComponent?.profileId ??
      tab?.profileId ??
      tab?.runtimeProfile ??
      serviceTabHandle?.profileId,
  );
  const visualProof = stringOrNull(displayContent?.state ?? proof?.state ?? state);
  const routeState = stringOrNull(routeComponent?.state);
  const tabState = stringOrNull(tabComponent?.state ?? target?.state);
  const guacamoleState = stringOrNull(guacamoleComponent?.state);
  const browserBuildState = stringOrNull(browserBuildProof?.state);
  const requestedBrowserBuild = stringOrNull(browserBuildProof?.requestedBrowserBuild);
  const selectedBrowserBuild = stringOrNull(browserBuildProof?.selectedBrowserBuild);
  const actualExecutablePath = stringOrNull(browserBuildProof?.actualExecutablePath);
  const browserBuildMismatchReason = stringOrNull(browserBuildProof?.mismatchReason);
  const failureReason = stringOrNull(
    operatorVisible?.reason ??
      proof?.reason ??
      (routeState && routeState !== 'ready' && routeState !== 'not_checked' ? routeState : null) ??
      (tabState && tabState !== 'ready' && tabState !== 'not_checked' ? tabState : null) ??
      (guacamoleState && guacamoleState !== 'ready' && guacamoleState !== 'not_checked'
        ? guacamoleState
        : null) ??
      (browserBuildState && browserBuildState === 'mismatch' ? browserBuildMismatchReason ?? browserBuildState : null) ??
      data?.error,
  );
  const ready = state === 'ready';
  const parts = [
    'remote_view_open',
    `operatorVisible=${state ?? 'missing'}`,
    `route=${routeId ?? 'missing'}`,
    `display=${displayAllocationId ?? displayName ?? 'missing'}`,
    `browser=${browserId ?? 'missing'}`,
    `session=${sessionName ?? 'missing'}`,
    `tab=${tabId ?? 'missing'}`,
    `profile=${profileId ?? 'missing'}`,
    `proof=${visualProof ?? 'missing'}`,
    requestedBrowserBuild ? `requestedBuild=${requestedBrowserBuild}` : null,
    selectedBrowserBuild ? `selectedBuild=${selectedBrowserBuild}` : null,
    actualExecutablePath ? `executable=${actualExecutablePath}` : null,
    browserBuildState ? `buildProof=${browserBuildState}` : null,
    failureReason ? `reason=${failureReason}` : null,
  ].filter(Boolean);
  return {
    ready,
    state,
    routeId,
    displayAllocationId,
    displayName,
    browserId,
    sessionName,
    tabId,
    profileId,
    visualProof,
    browserBuildState,
    requestedBrowserBuild,
    selectedBrowserBuild,
    actualExecutablePath,
    browserBuildMismatchReason,
    failureReason,
    summary: parts.join(' '),
  };
}

/**
 * Summarize access-plan or tab-response shared-profile acquisition facts so
 * clients do not need to parse nested profileReuse and tab data.
 *
 * @param {unknown} input
 * @returns {ServiceSharedProfileAcquisitionSummary}
 */
export function summarizeServiceSharedProfileAcquisition(input) {
  const record = recordFromUnknown(input);
  const data = recordFromUnknown(record?.data);
  const decision = recordFromUnknown(record?.decision) ?? recordFromUnknown(data?.decision);
  const serviceRequest = recordFromUnknown(decision?.serviceRequest);
  const plannedRequest = recordFromUnknown(serviceRequest?.request);
  const profileReuse = recordFromUnknown(decision?.profileReuse);
  const sharedAcquisition = recordFromUnknown(profileReuse?.sharedAcquisition) ?? recordFromUnknown(data?.sharedAcquisition);
  const intent = recordFromUnknown(data?.intent);
  const tab = recordFromUnknown(data?.tab);
  const serviceTabHandle = getServiceTabHandle(input) ?? getServiceTabHandle({ data }) ?? getServiceTabHandle(record);
  const runtimeProfile = stringOrNull(
    data?.runtimeProfile ??
      data?.profile ??
      intent?.runtime_profile ??
      intent?.runtimeProfile ??
      intent?.profile ??
      tab?.runtimeProfile ??
      plannedRequest?.runtimeProfile ??
      plannedRequest?.profile ??
      serviceTabHandle?.profileId,
  );
  const profileId = stringOrNull(
    data?.profileId ?? tab?.profileId ?? sharedAcquisition?.profileId ?? plannedRequest?.profileId ?? runtimeProfile ?? serviceTabHandle?.profileId,
  );
  const requestedProfile = stringOrNull(
    sharedAcquisition?.requestedProfile ??
      plannedRequest?.runtimeProfile ??
      plannedRequest?.profileId ??
      plannedRequest?.profile ??
      intent?.runtime_profile ??
      intent?.runtimeProfile ??
      intent?.profile ??
      runtimeProfile ??
      profileId,
  );
  const plannedProfile = stringOrNull(
    profileReuse?.selectedProfileId ??
      profileReuse?.profileId ??
      sharedAcquisition?.plannedProfile ??
      sharedAcquisition?.profileId ??
      plannedRequest?.profileId ??
      plannedRequest?.runtimeProfile ??
      data?.profileId ??
      data?.runtimeProfile ??
      tab?.profileId,
  );
  const browserId = stringOrNull(sharedAcquisition?.browserId ?? plannedRequest?.browserId ?? data?.browserId ?? serviceTabHandle?.browserId);
  const sessionName = stringOrNull(
    sharedAcquisition?.sessionName ??
      plannedRequest?.sessionName ??
      data?.sessionName ??
      data?.sessionId ??
      serviceTabHandle?.sessionName,
  );
  const tabId = stringOrNull(data?.tabId ?? data?.targetId ?? tab?.tabId ?? tab?.targetId ?? serviceTabHandle?.tabId);
  const targetId = stringOrNull(data?.targetId ?? tab?.targetId ?? serviceTabHandle?.targetId);
  const acquisitionMode = stringOrNull(sharedAcquisition?.mode ?? profileReuse?.defaultAcquisition);
  const recommendedAction = stringOrNull(
    profileReuse?.recommendedAction ?? sharedAcquisition?.recommendedAction ?? (sharedAcquisition ? 'reuse_existing_browser' : null),
  );
  const browserReused =
    typeof sharedAcquisition?.browserReused === 'boolean'
      ? sharedAcquisition.browserReused
      : sharedAcquisition
        ? true
        : null;
  const tabOpened = typeof sharedAcquisition?.tabOpened === 'boolean' ? sharedAcquisition.tabOpened : data ? true : null;
  const duplicateProcessPolicy = stringOrNull(
    sharedAcquisition?.duplicateProcessPolicy ?? profileReuse?.duplicateProcessPolicy ?? profileReuse?.profileProcessPolicy,
  );
  const requiresRouteHints = sharedAcquisition?.requiresRouteHints === true || Boolean(browserId || sessionName);
  const routeHintFields = stringArray(sharedAcquisition?.routeHintFields);
  const available = Boolean(sharedAcquisition || browserId || sessionName || serviceTabHandle);
  const parts = [
    'shared_profile_acquisition',
    `available=${available ? 'true' : 'false'}`,
    `mode=${acquisitionMode ?? 'missing'}`,
    `requestedProfile=${requestedProfile ?? 'missing'}`,
    `plannedProfile=${plannedProfile ?? 'missing'}`,
    `browser=${browserId ?? 'missing'}`,
    `session=${sessionName ?? 'missing'}`,
    `tab=${tabId ?? 'missing'}`,
    duplicateProcessPolicy ? `duplicateProcessPolicy=${duplicateProcessPolicy}` : null,
    requiresRouteHints ? 'routeHints=true' : null,
  ].filter(Boolean);

  return {
    available,
    recommendedAction,
    acquisitionMode,
    requestedProfile,
    plannedProfile,
    runtimeProfile,
    profileId,
    browserId,
    sessionName,
    tabId,
    targetId,
    browserReused,
    tabOpened,
    profileProcessPolicy: stringOrNull(profileReuse?.profileProcessPolicy),
    clientSharingPolicy: stringOrNull(profileReuse?.clientSharingPolicy ?? sharedAcquisition?.policy),
    duplicateProcessPolicy,
    requiresRouteHints,
    routeHintFields,
    serviceTabHandle,
    summary: parts.join(' '),
  };
}

/**
 * Require `operatorVisible.state=ready` for a real route-bound handoff.
 * Dry-runs and explicit infrastructure-only readiness checks do not claim
 * operator visibility, so they are allowed to return without throwing.
 *
 * @param {unknown} response
 * @param {{ allowInfrastructureOnlyReadiness?: boolean }} [options]
 * @returns {Record<string, unknown> | null}
 */
export function requireServiceRemoteViewOpenOperatorVisible(response, options = {}) {
  const data = serviceRemoteViewOpenData(response);
  if (data?.dryRun === true || options.allowInfrastructureOnlyReadiness === true) {
    return data;
  }
  if (isServiceRemoteViewOpenOperatorVisibleReady(response)) {
    return data;
  }
  const summary = summarizeServiceRemoteViewOpenProof(response);
  throw new TypeError(`remote-view open response is not operator-visible: ${summary.summary}`);
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
  const sharedAcquisition = accessPlanSharedTabAcquisition(decisionRecord);
  if (sharedAcquisition) {
    if (tabRequest.browserId === undefined) {
      tabRequest.browserId = sharedAcquisition.browserId;
    }
    if (tabRequest.sessionName === undefined) {
      tabRequest.sessionName = sharedAcquisition.sessionName;
    }
  }
  return tabRequest;
}

/**
 * @param {Record<string, unknown>} decision
 * @returns {{ browserId: string, sessionName: string } | null}
 */
function accessPlanSharedTabAcquisition(decision) {
  const profileReuse = decision.profileReuse;
  if (!profileReuse || typeof profileReuse !== 'object' || Array.isArray(profileReuse)) {
    return null;
  }
  const sharedAcquisition = /** @type {Record<string, unknown>} */ (profileReuse).sharedAcquisition;
  if (
    !sharedAcquisition ||
    typeof sharedAcquisition !== 'object' ||
    Array.isArray(sharedAcquisition)
  ) {
    return null;
  }
  const acquisitionRecord = /** @type {Record<string, unknown>} */ (sharedAcquisition);
  if (acquisitionRecord.mode !== 'tab_new') {
    return null;
  }
  const browserId = acquisitionRecord.browserId;
  const sessionName = acquisitionRecord.sessionName;
  if (typeof browserId !== 'string' || typeof sessionName !== 'string') {
    return null;
  }
  return { browserId, sessionName };
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
 * @returns {Record<string, unknown> | null}
 */
function recordFromUnknown(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null;
  }
  return /** @type {Record<string, unknown>} */ (value);
}

/**
 * @param {unknown} value
 * @returns {string | null}
 */
function stringOrNull(value) {
  return typeof value === 'string' && value.length > 0 ? value : null;
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
