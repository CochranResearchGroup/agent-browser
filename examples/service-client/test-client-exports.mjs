#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  attachServiceTabCdp,
  createServiceCdpFreeLaunchRequest,
  createServiceFileTransferRequest,
  createServiceNetworkCaptureRequest,
  createServiceProbeRequest,
  createServiceUiActionRequest,
  createServiceRequest,
  createServiceRequestMcpToolCall,
  createServiceRemoteViewOpenRequest,
  createServiceRemoteViewRoutePreflightRequest,
  createServiceTabRequest,
  createServiceTabHandleRefreshRequest,
  createServiceTabHandleReleaseRequest,
  evaluateServiceTab,
  getServiceTabDiagnostics,
  postServiceRequest,
  probeServiceTab,
  requestServiceFileTransfer,
  captureServiceNetwork,
  requestServiceNetworkCapture,
  requestServiceUiAction,
  refreshServiceTabHandle,
  releaseServiceTabHandle,
  runServiceUiAction,
  requestServiceCdpFreeLaunch,
  requestServiceProbe,
  requestServiceRemoteViewOpen,
  requestServiceRemoteViewRoutePreflight,
  requestServiceTab,
  requestServiceTabHandleRefresh,
  requestServiceTabHandleRelease,
  requestServiceTabFromAccessPlan,
  transferServiceFiles,
} from '@agent-browser/client/service-request';
import {
  cancelServiceJob,
  createServiceProfileReadinessMonitor,
  findServiceProfileForIdentity,
  getServiceAccessPlan,
  getServiceProfileAllocationForAccessPlan,
  getServiceProfileForIdentity,
  getServiceProfileReadiness,
  getServiceProfileSeedingHandoff,
  getServiceProfiles,
  getServiceStatus,
  getServiceTrace,
  lookupServiceProfile,
  registerServiceLoginProfile,
  runServiceAccessPlanPostSeedingProbe,
  summarizeServiceProfileAllocationBrowserHealth,
  summarizeServiceProfileReadiness,
  summarizeServiceTraceAttention,
  summarizeServiceTraceDisplayAllocations,
  updateServiceProfileFreshness,
  upsertServiceProfileReadinessMonitor,
} from '@agent-browser/client/service-observability';

assert.equal(typeof createServiceRequest, 'function');
assert.equal(typeof createServiceCdpFreeLaunchRequest, 'function');
assert.equal(typeof createServiceFileTransferRequest, 'function');
assert.equal(typeof createServiceNetworkCaptureRequest, 'function');
assert.equal(typeof createServiceProbeRequest, 'function');
assert.equal(typeof createServiceUiActionRequest, 'function');
assert.equal(typeof createServiceRequestMcpToolCall, 'function');
assert.equal(typeof createServiceRemoteViewOpenRequest, 'function');
assert.equal(typeof createServiceRemoteViewRoutePreflightRequest, 'function');
assert.equal(typeof createServiceTabRequest, 'function');
assert.equal(typeof createServiceTabHandleRefreshRequest, 'function');
assert.equal(typeof createServiceTabHandleReleaseRequest, 'function');
assert.equal(typeof attachServiceTabCdp, 'function');
assert.equal(typeof evaluateServiceTab, 'function');
assert.equal(typeof getServiceTabDiagnostics, 'function');
assert.equal(typeof postServiceRequest, 'function');
assert.equal(typeof probeServiceTab, 'function');
assert.equal(typeof requestServiceFileTransfer, 'function');
assert.equal(typeof captureServiceNetwork, 'function');
assert.equal(typeof requestServiceNetworkCapture, 'function');
assert.equal(typeof requestServiceUiAction, 'function');
assert.equal(typeof runServiceUiAction, 'function');
assert.equal(typeof refreshServiceTabHandle, 'function');
assert.equal(typeof releaseServiceTabHandle, 'function');
assert.equal(typeof requestServiceCdpFreeLaunch, 'function');
assert.equal(typeof requestServiceProbe, 'function');
assert.equal(typeof requestServiceRemoteViewOpen, 'function');
assert.equal(typeof requestServiceRemoteViewRoutePreflight, 'function');
assert.equal(typeof requestServiceTab, 'function');
assert.equal(typeof requestServiceTabHandleRefresh, 'function');
assert.equal(typeof requestServiceTabHandleRelease, 'function');
assert.equal(typeof requestServiceTabFromAccessPlan, 'function');
assert.equal(typeof transferServiceFiles, 'function');
assert.equal(typeof cancelServiceJob, 'function');
assert.equal(typeof createServiceProfileReadinessMonitor, 'function');
assert.equal(typeof findServiceProfileForIdentity, 'function');
assert.equal(typeof getServiceAccessPlan, 'function');
assert.equal(typeof getServiceProfileAllocationForAccessPlan, 'function');
assert.equal(typeof getServiceProfileForIdentity, 'function');
assert.equal(typeof getServiceProfileReadiness, 'function');
assert.equal(typeof getServiceProfileSeedingHandoff, 'function');
assert.equal(typeof getServiceProfiles, 'function');
assert.equal(typeof getServiceStatus, 'function');
assert.equal(typeof getServiceTrace, 'function');
assert.equal(typeof lookupServiceProfile, 'function');
assert.equal(typeof registerServiceLoginProfile, 'function');
assert.equal(typeof runServiceAccessPlanPostSeedingProbe, 'function');
assert.equal(typeof summarizeServiceProfileAllocationBrowserHealth, 'function');
assert.equal(typeof summarizeServiceProfileReadiness, 'function');
assert.equal(typeof summarizeServiceTraceAttention, 'function');
assert.equal(typeof summarizeServiceTraceDisplayAllocations, 'function');
assert.equal(typeof updateServiceProfileFreshness, 'function');
assert.equal(typeof upsertServiceProfileReadinessMonitor, 'function');

console.log('Service client package exports resolved');
