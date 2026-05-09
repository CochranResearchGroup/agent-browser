#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  createServiceRequest,
  createServiceRequestMcpToolCall,
  createServiceTabRequest,
  postServiceRequest,
  requestServiceTab,
} from '@agent-browser/client/service-request';
import {
  cancelServiceJob,
  createServiceProfileReadinessMonitor,
  findServiceProfileForIdentity,
  getServiceAccessPlan,
  getServiceProfileForIdentity,
  getServiceProfileReadiness,
  getServiceProfiles,
  getServiceStatus,
  getServiceTrace,
  lookupServiceProfile,
  registerServiceLoginProfile,
  summarizeServiceProfileReadiness,
  updateServiceProfileFreshness,
  upsertServiceProfileReadinessMonitor,
} from '@agent-browser/client/service-observability';

assert.equal(typeof createServiceRequest, 'function');
assert.equal(typeof createServiceRequestMcpToolCall, 'function');
assert.equal(typeof createServiceTabRequest, 'function');
assert.equal(typeof postServiceRequest, 'function');
assert.equal(typeof requestServiceTab, 'function');
assert.equal(typeof cancelServiceJob, 'function');
assert.equal(typeof createServiceProfileReadinessMonitor, 'function');
assert.equal(typeof findServiceProfileForIdentity, 'function');
assert.equal(typeof getServiceAccessPlan, 'function');
assert.equal(typeof getServiceProfileForIdentity, 'function');
assert.equal(typeof getServiceProfileReadiness, 'function');
assert.equal(typeof getServiceProfiles, 'function');
assert.equal(typeof getServiceStatus, 'function');
assert.equal(typeof getServiceTrace, 'function');
assert.equal(typeof lookupServiceProfile, 'function');
assert.equal(typeof registerServiceLoginProfile, 'function');
assert.equal(typeof summarizeServiceProfileReadiness, 'function');
assert.equal(typeof updateServiceProfileFreshness, 'function');
assert.equal(typeof upsertServiceProfileReadinessMonitor, 'function');

console.log('Service client package exports resolved');
