#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  createServiceRequest,
  createServiceRequestMcpToolCall,
  createServiceTabRequest,
  createServiceTabRequestFromAccessPlan,
  postServiceRequest,
  requestServiceTab,
} from '../packages/client/src/service-request.js';
import { getServiceAccessPlan } from '../packages/client/src/service-observability.js';

function assertServiceRequestActionDataCoverage() {
  const schema = JSON.parse(
    readFileSync(
      new URL('../docs/dev/contracts/service-request.v1.schema.json', import.meta.url),
      'utf8',
    ),
  );
  const generatorSource = readFileSync(
    new URL('./generate-service-request-client.js', import.meta.url),
    'utf8',
  );
  const mapBody = generatorSource.match(/export interface ServiceRequestActionDataMap \{([\s\S]*?)\n\}/)?.[1];
  assert.ok(mapBody, 'ServiceRequestActionDataMap must exist in the service request generator');

  const actions = schema.properties.action.enum;
  const mappedActions = [...mapBody.matchAll(/^\s*([a-z0-9_]+):/gm)].map((match) => match[1]);
  assert.deepEqual(actions.filter((action) => !mappedActions.includes(action)), []);
  assert.deepEqual(mappedActions.filter((action) => !actions.includes(action)), []);
}

function createFetchRecorder(payload = { success: true, data: { jobId: 'job-1' } }) {
  const calls = [];
  const fetch = async (url, init = {}) => {
    calls.push({
      url: String(url),
      init,
      body: init.body ? JSON.parse(String(init.body)) : null,
    });
    return {
      ok: true,
      status: 200,
      json: async () => payload,
    };
  };
  return { calls, fetch };
}

function createAccessPlanToServiceRequestFetchRecorder() {
  const calls = [];
  const accessPlan = {
    selectedProfile: {
      id: 'journal-acs',
    },
    decision: {
      serviceRequest: {
        available: true,
        request: {
          serviceName: 'JournalDownloader',
          agentName: 'article-probe-agent',
          taskName: 'probeACSwebsite',
          loginId: 'acs',
          targetServiceId: 'acs',
          profileLeasePolicy: 'wait',
          action: 'tab_new',
        },
      },
    },
  };
  const fetch = async (url, init = {}) => {
    const parsed = new URL(String(url));
    calls.push({
      url: String(url),
      method: init.method || 'GET',
      body: init.body ? JSON.parse(String(init.body)) : null,
    });

    if (parsed.pathname === '/api/service/access-plan') {
      return {
        ok: true,
        status: 200,
        json: async () => ({ success: true, data: accessPlan }),
      };
    }

    if (parsed.pathname === '/api/service/request') {
      return {
        ok: true,
        status: 200,
        json: async () => ({ success: true, data: { jobId: 'job-access-plan-tab' } }),
      };
    }

    throw new Error(`Unexpected request: ${String(url)}`);
  };
  return { calls, fetch };
}

async function main() {
  assertServiceRequestActionDataCoverage();

  assert.throws(
    () =>
      createServiceRequest({
        action: 'unsupported',
      }),
    /Unsupported service request action/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        params: 'not-an-object',
      }),
    /service request params must be an object/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        jobTimeoutMs: 0,
      }),
    /jobTimeoutMs must be a positive integer/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        profileLeaseWaitTimeoutMs: 0,
      }),
    /profileLeaseWaitTimeoutMs must be a positive integer/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        serviceName: 42,
      }),
    /service request serviceName must be a string/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        profileLeasePolicy: 'maybe',
      }),
    /profileLeasePolicy must be reject or wait/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        loginIds: ['acs', 42],
      }),
    /service request loginIds must be an array of strings/,
  );

  const request = createServiceRequest({
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    loginIds: ['acs', 'google'],
    profile: 'journal-acs',
    action: 'navigate',
    params: {
      url: 'https://example.com',
      waitUntil: 'load',
    },
    jobTimeoutMs: 30_000,
    profileLeasePolicy: 'wait',
    profileLeaseWaitTimeoutMs: 5_000,
  });
  assert.deepEqual(request, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    loginIds: ['acs', 'google'],
    profile: 'journal-acs',
    action: 'navigate',
    params: {
      url: 'https://example.com',
      waitUntil: 'load',
    },
    jobTimeoutMs: 30_000,
    profileLeasePolicy: 'wait',
    profileLeaseWaitTimeoutMs: 5_000,
  });

  const mcpToolCall = createServiceRequestMcpToolCall(request);
  assert.deepEqual(mcpToolCall, {
    name: 'service_request',
    arguments: request,
  });

  const tabRequest = createServiceTabRequest({
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    loginId: 'acs',
    targetServiceId: 'acs',
    url: 'https://example.com/articles',
    params: {
      waitUntil: 'domcontentloaded',
    },
    jobTimeoutMs: 45_000,
  });
  assert.deepEqual(tabRequest, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    loginId: 'acs',
    targetServiceId: 'acs',
    action: 'tab_new',
    params: {
      waitUntil: 'domcontentloaded',
      url: 'https://example.com/articles',
    },
    jobTimeoutMs: 45_000,
  });
  const accessPlan = {
    decision: {
      serviceRequest: {
        request: {
          serviceName: 'JournalDownloader',
          agentName: 'article-probe-agent',
          taskName: 'plannedProbeACSwebsite',
          targetServiceIds: ['acs'],
          profileLeasePolicy: 'wait',
          action: 'tab_new',
        },
      },
    },
  };
  const manualSeedingAccessPlan = {
    readinessSummary: {
      manualSeedingRequired: true,
    },
    seedingHandoff: {
      command: 'agent-browser --runtime-profile google-work runtime login https://accounts.google.com',
    },
    decision: {
      manualSeedingRequired: true,
      serviceRequest: {
        available: false,
        blockedByManualAction: true,
        request: {
          serviceName: 'JournalDownloader',
          agentName: 'article-probe-agent',
          taskName: 'seedThenProbeGoogle',
          targetServiceIds: ['google'],
          profileLeasePolicy: 'wait',
          action: 'tab_new',
        },
      },
    },
  };
  assert.deepEqual(
    createServiceTabRequestFromAccessPlan(accessPlan, {
      url: 'https://example.com/planned',
      jobTimeoutMs: 60_000,
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'plannedProbeACSwebsite',
      targetServiceIds: ['acs'],
      profileLeasePolicy: 'wait',
      action: 'tab_new',
      params: {
        url: 'https://example.com/planned',
      },
      jobTimeoutMs: 60_000,
    },
  );
  assert.throws(
    () =>
      createServiceTabRequestFromAccessPlan(manualSeedingAccessPlan, {
        url: 'https://accounts.google.com',
      }),
    /requires manual profile seeding.*agent-browser --runtime-profile google-work runtime login https:\/\/accounts.google.com/,
  );
  assert.deepEqual(
    createServiceTabRequestFromAccessPlan(manualSeedingAccessPlan, {
      allowManualAction: true,
      url: 'https://accounts.google.com',
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'seedThenProbeGoogle',
      targetServiceIds: ['google'],
      profileLeasePolicy: 'wait',
      action: 'tab_new',
      params: {
        url: 'https://accounts.google.com',
      },
    },
  );
  assert.deepEqual(
    createServiceTabRequest({
      accessPlan,
      taskName: 'overrideTask',
      url: 'https://example.com/override',
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'overrideTask',
      targetServiceIds: ['acs'],
      profileLeasePolicy: 'wait',
      action: 'tab_new',
      params: {
        url: 'https://example.com/override',
      },
    },
  );
  assert.throws(
    () =>
      createServiceTabRequest({
        accessPlan: {
          decision: {
            serviceRequest: {
              request: {
                action: 'navigate',
              },
            },
          },
        },
      }),
    /serviceRequest.request action must be tab_new/,
  );

  const postRecorder = createFetchRecorder({ success: true, data: { jobId: 'job-post' } });
  const postResponse = await postServiceRequest({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: postRecorder.fetch,
    request,
  });
  assert.deepEqual(postResponse, { success: true, data: { jobId: 'job-post' } });
  assert.equal(postRecorder.calls.length, 1);
  assert.equal(postRecorder.calls[0].url, 'http://127.0.0.1:4849/api/service/request');
  assert.equal(postRecorder.calls[0].init.method, 'POST');
  assert.deepEqual(postRecorder.calls[0].init.headers, { 'content-type': 'application/json' });
  assert.deepEqual(postRecorder.calls[0].body, request);

  const tabRecorder = createFetchRecorder({ success: true, data: { jobId: 'job-tab' } });
  const tabResponse = await requestServiceTab({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: tabRecorder.fetch,
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    loginId: 'acs',
    targetServiceId: 'acs',
    url: 'https://example.com/new',
  });
  assert.deepEqual(tabResponse, { success: true, data: { jobId: 'job-tab' } });
  assert.deepEqual(tabRecorder.calls[0].body, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    loginId: 'acs',
    targetServiceId: 'acs',
    action: 'tab_new',
    params: {
      url: 'https://example.com/new',
    },
  });
  const plannedTabRecorder = createFetchRecorder({ success: true, data: { jobId: 'job-planned-tab' } });
  const plannedTabResponse = await requestServiceTab({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: plannedTabRecorder.fetch,
    accessPlan,
    url: 'https://example.com/from-plan',
    jobTimeoutMs: 60_000,
  });
  assert.deepEqual(plannedTabResponse, { success: true, data: { jobId: 'job-planned-tab' } });
  assert.deepEqual(plannedTabRecorder.calls[0].body, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'plannedProbeACSwebsite',
    targetServiceIds: ['acs'],
    profileLeasePolicy: 'wait',
    action: 'tab_new',
    params: {
      url: 'https://example.com/from-plan',
    },
    jobTimeoutMs: 60_000,
  });
  const blockedTabRecorder = createFetchRecorder({ success: true, data: { jobId: 'should-not-run' } });
  await assert.rejects(
    () =>
      requestServiceTab({
        baseUrl: 'http://127.0.0.1:4849',
        fetch: blockedTabRecorder.fetch,
        accessPlan: manualSeedingAccessPlan,
        url: 'https://accounts.google.com',
      }),
    /requires manual profile seeding/,
  );
  assert.equal(blockedTabRecorder.calls.length, 0);

  const accessPlanWorkflow = createAccessPlanToServiceRequestFetchRecorder();
  const observedAccessPlan = await getServiceAccessPlan({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: accessPlanWorkflow.fetch,
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    loginId: 'acs',
    targetServiceId: 'acs',
  });
  const accessPlanTabResponse = await requestServiceTab({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: accessPlanWorkflow.fetch,
    accessPlan: observedAccessPlan,
    url: 'https://example.com/access-plan-workflow',
    jobTimeoutMs: 90_000,
  });
  assert.deepEqual(accessPlanTabResponse, {
    success: true,
    data: { jobId: 'job-access-plan-tab' },
  });
  assert.equal(accessPlanWorkflow.calls.length, 2);
  assert.equal(
    accessPlanWorkflow.calls[0].url,
    'http://127.0.0.1:4849/api/service/access-plan?serviceName=JournalDownloader&agentName=article-probe-agent&taskName=probeACSwebsite&loginId=acs&targetServiceId=acs',
  );
  assert.deepEqual(accessPlanWorkflow.calls[1].body, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    loginId: 'acs',
    targetServiceId: 'acs',
    profileLeasePolicy: 'wait',
    action: 'tab_new',
    params: {
      url: 'https://example.com/access-plan-workflow',
    },
    jobTimeoutMs: 90_000,
  });

  console.log('Service request client helper tests passed');
}

main().catch((err) => {
  console.error(err?.stack || err?.message || String(err));
  process.exit(1);
});
