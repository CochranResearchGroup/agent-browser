#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  profileAllocationFromLookupPayload,
  serviceProfileAllocationLookupUrl,
} from '../packages/dashboard/src/lib/service-profile-allocation.ts';

assert.equal(
  serviceProfileAllocationLookupUrl('http://localhost:9223/api/service', 'journal-downloader'),
  'http://localhost:9223/api/service/profiles/journal-downloader/allocation',
);
assert.equal(
  serviceProfileAllocationLookupUrl('http://localhost:9223/api/service/', 'profile with space'),
  'http://localhost:9223/api/service/profiles/profile%20with%20space/allocation',
);
assert.throws(
  () => serviceProfileAllocationLookupUrl(' ', 'journal-downloader'),
  /service base URL/,
);
assert.throws(
  () => serviceProfileAllocationLookupUrl('http://localhost:9223/api/service', ' '),
  /profile ID/,
);

const fallback = {
  profileId: 'journal-downloader',
  leaseState: 'shared',
};
const fresh = {
  profileId: 'journal-downloader',
  leaseState: 'exclusive',
  recommendedAction: 'inspect_conflicts',
};

assert.deepEqual(
  profileAllocationFromLookupPayload(
    {
      success: true,
      data: {
        profileAllocation: fresh,
      },
    },
    fallback,
  ),
  fresh,
);
assert.deepEqual(
  profileAllocationFromLookupPayload(
    {
      success: true,
      data: {},
    },
    fallback,
  ),
  fallback,
);
assert.throws(
  () =>
    profileAllocationFromLookupPayload(
      {
        success: false,
        error: 'Profile allocation not found: journal-downloader',
      },
      fallback,
    ),
  /Profile allocation not found/,
);

console.log('Dashboard profile allocation lookup smoke passed');
