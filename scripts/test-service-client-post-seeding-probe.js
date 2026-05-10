#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  buildPostSeedingProbePlan,
  runPostSeedingProbe,
} from '../examples/service-client/post-seeding-probe.mjs';

await testDryRunPlan();
await testFreshProbeUpdatesFreshness();
await testFailedExpectationMarksStale();
await testBrokerMismatchRefusesFreshness();

console.log('Post-seeding probe no-launch smoke passed');

async function testDryRunPlan() {
  const result = await runPostSeedingProbe({
    dryRun: true,
    profileId: 'google-work',
    targetServiceId: 'google',
    expectedTitleIncludes: 'Google Account',
  });

  assert.equal(result.dryRun, true);
  assert.equal(result.plan.profileId, 'google-work');
  assert.deepEqual(
    result.plan.sequence,
    buildPostSeedingProbePlan().sequence,
  );
  assert.equal(result.plan.boundedChecks.expectedTitleIncludes, 'Google Account');
}

async function testFreshProbeUpdatesFreshness() {
  const calls = [];
  const fetch = createMockFetch({
    calls,
    url: 'https://myaccount.google.com/',
    title: 'Google Account',
  });

  const result = await runPostSeedingProbe({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    profileId: 'google-work',
    url: 'https://myaccount.google.com/',
    loginId: 'google',
    targetServiceId: 'google',
    expectedUrlIncludes: 'myaccount.google.com',
    expectedTitleIncludes: 'Google Account',
  });

  assert.equal(result.checks.fresh, true);
  assert.deepEqual(result.checks.failed, []);
  assert.equal(result.observed.url, 'https://myaccount.google.com/');
  assert.equal(result.observed.title, 'Google Account');

  const freshnessCall = calls.find(
    (call) => call.method === 'POST' && call.path === '/api/service/profiles/google-work/freshness',
  );
  assert(freshnessCall, 'post-seeding probe did not record freshness');
  const body = JSON.parse(String(freshnessCall.body));
  assert.equal(body.loginId, 'google');
  assert.equal(body.targetServiceId, 'google');
  assert.equal(body.readinessState, 'fresh');
  assert.match(body.readinessEvidence, /^post_seeding_auth_probe_passed:/);

  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    [
      'GET /api/service/profiles/lookup',
      'POST /api/service/request',
      'POST /api/service/request',
      'POST /api/service/request',
      'POST /api/service/profiles/google-work/freshness',
    ],
  );
}

async function testFailedExpectationMarksStale() {
  const calls = [];
  const fetch = createMockFetch({
    calls,
    url: 'https://accounts.google.com/signin',
    title: 'Sign in',
  });

  const result = await runPostSeedingProbe({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    profileId: 'google-work',
    targetServiceId: 'google',
    expectedTitleIncludes: 'Google Account',
  });

  assert.equal(result.checks.fresh, false);
  assert.deepEqual(result.checks.failed, ['expected_title']);

  const freshnessCall = calls.find(
    (call) => call.method === 'POST' && call.path === '/api/service/profiles/google-work/freshness',
  );
  assert(freshnessCall, 'failed post-seeding probe did not record freshness');
  const body = JSON.parse(String(freshnessCall.body));
  assert.equal(body.readinessState, 'stale');
  assert.match(body.readinessEvidence, /^post_seeding_auth_probe_failed:/);
}

async function testBrokerMismatchRefusesFreshness() {
  const calls = [];
  const fetch = createMockFetch({
    calls,
    url: 'https://myaccount.google.com/',
    title: 'Google Account',
    selectedProfileId: 'personal-google',
  });

  await assert.rejects(
    () =>
      runPostSeedingProbe({
        baseUrl: 'http://127.0.0.1:4849',
        fetch,
        profileId: 'google-work',
        targetServiceId: 'google',
      }),
    /broker selected personal-google/,
  );
  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    ['GET /api/service/profiles/lookup'],
  );
}

function createMockFetch({ calls, url, title, selectedProfileId = 'google-work' }) {
  return async (input, init = {}) => {
    const parsed = new URL(String(input));
    const body = init.body ? JSON.parse(String(init.body)) : null;
    calls.push({
      method: init.method || 'GET',
      path: parsed.pathname,
      body: init.body,
    });

    if (parsed.pathname === '/api/service/request') {
      if (body?.action === 'tab_new') {
        return jsonResponse({
          success: true,
          data: {
            index: 0,
            url,
          },
        });
      }
      if (body?.action === 'url') {
        return jsonResponse({
          success: true,
          data: {
            url,
          },
        });
      }
      if (body?.action === 'title') {
        return jsonResponse({
          success: true,
          data: {
            title,
          },
        });
      }
    }

    if (parsed.pathname === '/api/service/profiles/lookup') {
      return jsonResponse({
        success: true,
        data: {
          query: Object.fromEntries(parsed.searchParams.entries()),
          selectedProfile: {
            id: selectedProfileId,
            targetServiceIds: ['google'],
            authenticatedServiceIds: ['google'],
          },
          selectedProfileMatch: {
            reason: 'authenticated_target',
            matchedField: 'authenticatedServiceIds',
            matchedIdentity: 'google',
          },
        },
      });
    }

    if (parsed.pathname.endsWith('/freshness')) {
      return jsonResponse({
        success: true,
        data: {
          id: parsed.pathname.split('/').at(-2),
          upserted: true,
          profile: body,
        },
      });
    }

    throw new Error(`Unexpected request: ${init.method || 'GET'} ${parsed.pathname}`);
  };
}

function jsonResponse(payload) {
  return {
    ok: true,
    json: async () => payload,
  };
}
