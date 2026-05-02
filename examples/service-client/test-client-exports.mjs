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
  getServiceStatus,
  getServiceTrace,
  registerServiceLoginProfile,
} from '@agent-browser/client/service-observability';

assert.equal(typeof createServiceRequest, 'function');
assert.equal(typeof createServiceRequestMcpToolCall, 'function');
assert.equal(typeof createServiceTabRequest, 'function');
assert.equal(typeof postServiceRequest, 'function');
assert.equal(typeof requestServiceTab, 'function');
assert.equal(typeof cancelServiceJob, 'function');
assert.equal(typeof getServiceStatus, 'function');
assert.equal(typeof getServiceTrace, 'function');
assert.equal(typeof registerServiceLoginProfile, 'function');

console.log('Service client package exports resolved');
