#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  attachServiceTabCdp,
  createServiceControllerLeaseTakeoverRequest,
  createServiceCdpAttachRequest,
  createServiceCdpDetachRequest,
  createServiceCdpFreeLaunchRequest,
  createServiceDiagnosticsRequest,
  createServiceEvaluateRequest,
  createServiceFileTransferRequest,
  createServiceNetworkCaptureRequest,
  createServiceProbeRequest,
  createServiceUiActionRequest,
  createServiceTabHandleRefreshRequest,
  createServiceTabHandleReleaseRequest,
  createServiceRemoteViewRouteCheckoutRequest,
  createServiceRemoteViewOpenRequest,
  createServiceRemoteViewRouteReleaseRequest,
  createServiceRoutePoolRepairRequest,
  createServiceRequest,
  createServiceRequestMcpToolCall,
  createServiceTabRequest,
  createServiceTabRequestFromAccessPlan,
  createServiceViewerLeaseHeartbeatRequest,
  createServiceViewerLeaseReleaseRequest,
  createServiceViewerLeaseRequest,
  evaluateServiceTab,
  getServiceTabHandle,
  getServiceTabDiagnostics,
  heartbeatServiceViewerLease,
  isServiceCdpFreeActionAvailable,
  postServiceRequest,
  probeServiceTab,
  requestServiceFileTransfer,
  captureServiceNetwork,
  requestServiceUiAction,
  refreshServiceTabHandle,
  runServiceUiAction,
  releaseServiceTabHandle,
  releaseServiceViewerLease,
  requireServiceTabHandle,
  requestServiceCdpAttach,
  requestServiceCdpDetach,
  requestServiceCdpFreeLaunch,
  requestServiceDiagnostics,
  requestServiceEvaluate,
  requestServiceNetworkCapture,
  requestServiceProbe,
  requestServiceTabHandleRefresh,
  requestServiceTabHandleRelease,
  requestServiceRemoteViewRouteCheckout,
  requestServiceRemoteViewOpen,
  requestServiceRoutePoolRepair,
  requestServiceTab,
  requestServiceTabFromAccessPlan,
  requestServiceViewerLease,
  transferServiceFiles,
  SERVICE_REQUEST_ACTIONS,
  summarizeServiceCdpFreeLaunchAvailability,
  takeoverServiceControllerLease,
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

function createCdpFreeAccessPlanToServiceRequestFetchRecorder() {
  const calls = [];
  const accessPlan = {
    selectedProfile: {
      id: 'canva-default',
    },
    decision: {
      launchPosture: {
        browserBuild: 'cdp_free_headed',
        requiresCdpFree: true,
        cdpAttachmentAllowed: false,
      },
      serviceRequest: {
        available: false,
        blockedByCdpFree: true,
        requiresCdpFree: true,
        cdpAttachmentAllowed: false,
        cdpFreeAvailability: {
          applies: true,
          availableCommands: ['cdp_free_launch'],
          unsupportedCommands: ['snapshot', 'click'],
          supportedOperations: ['process_lifecycle', 'service_state'],
          unsupportedOperations: ['cdp_commands', 'dom_interaction'],
          client: {
            summaryHelper: 'summarizeServiceCdpFreeLaunchAvailability',
            predicateHelper: 'isServiceCdpFreeActionAvailable',
          },
        },
        request: {
          serviceName: 'CanvaCLI',
          agentName: 'canva-cli-agent',
          taskName: 'openCanvaWorkspace',
          loginId: 'canva',
          targetServiceId: 'canva',
          profileLeasePolicy: 'wait',
          action: 'tab_new',
          requiresCdpFree: true,
          cdpAttachmentAllowed: false,
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
        json: async () => ({ success: true, data: { jobId: 'job-access-plan-cdp-free' } }),
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
        action: 'tab_new',
        displayIsolation: 'exclusive',
      }),
    /displayIsolation must be private_virtual_display, shared_display, or ambient_display/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        loginIds: ['acs', 42],
      }),
    /service request loginIds must be an array of strings/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'tab_new',
        allowManualAction: 'true',
      }),
    /service request allowManualAction must be a boolean/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'tab_new',
        monitorRunDueSummary: 'stale',
      }),
    /service request monitorRunDueSummary must be an object/,
  );

  const request = createServiceRequest({
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    loginIds: ['acs', 'google'],
    profile: 'journal-acs',
    action: 'navigate',
    displayIsolation: 'private_virtual_display',
    params: {
      url: 'https://example.com',
      waitUntil: 'load',
    },
    jobTimeoutMs: 30_000,
    profileLeasePolicy: 'wait',
    profileLeaseWaitTimeoutMs: 5_000,
    monitorRunDueSummary: {
      targetServiceIds: ['acs'],
      matched: 1,
      expiredTargetServiceIds: [],
      unverifiedTargetServiceIds: [],
      failed: false,
      recommendedAction: 'use_selected_profile',
    },
  });
  assert.deepEqual(request, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    loginIds: ['acs', 'google'],
    profile: 'journal-acs',
    action: 'navigate',
    displayIsolation: 'private_virtual_display',
    params: {
      url: 'https://example.com',
      waitUntil: 'load',
    },
    jobTimeoutMs: 30_000,
    profileLeasePolicy: 'wait',
    profileLeaseWaitTimeoutMs: 5_000,
    monitorRunDueSummary: {
      targetServiceIds: ['acs'],
      matched: 1,
      expiredTargetServiceIds: [],
      unverifiedTargetServiceIds: [],
      failed: false,
      recommendedAction: 'use_selected_profile',
    },
  });

  const cdpFreeLaunchRequest = createServiceRequest({
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    action: 'cdp_free_launch',
    requiresCdpFree: true,
    cdpAttachmentAllowed: false,
    params: {
      url: 'https://www.canva.com/',
    },
    jobTimeoutMs: 30_000,
  });
  assert.deepEqual(cdpFreeLaunchRequest, {
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    action: 'cdp_free_launch',
    requiresCdpFree: true,
    cdpAttachmentAllowed: false,
    params: {
      url: 'https://www.canva.com/',
    },
    jobTimeoutMs: 30_000,
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
          params: {
            browserHost: 'remote_headed',
            displayIsolation: 'private_virtual_display',
          },
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
          blockedByManualAction: true,
          manualSeedingRequired: true,
        },
      },
    },
  };
  const cdpFreeAccessPlan = {
    decision: {
      launchPosture: {
        requiresCdpFree: true,
        cdpAttachmentAllowed: false,
      },
      serviceRequest: {
        available: false,
        blockedByCdpFree: true,
        requiresCdpFree: true,
        cdpAttachmentAllowed: false,
        request: {
          serviceName: 'CanvaCLI',
          agentName: 'article-probe-agent',
          taskName: 'openCanva',
          targetServiceIds: ['canva'],
          profileLeasePolicy: 'wait',
          action: 'tab_new',
          requiresCdpFree: true,
          cdpAttachmentAllowed: false,
        },
      },
    },
  };
  const sharedProfileAccessPlan = {
    decision: {
      profileReuse: {
        sharedAcquisition: {
          policy: 'shared_browser_tabs',
          mode: 'tab_new',
          browserId: 'browser-shared',
          sessionName: 'shared-session',
          requiresRouteHints: true,
        },
      },
      serviceRequest: {
        request: {
          serviceName: 'AuraCall',
          agentName: 'auracall-agent',
          taskName: 'openSharedTab',
          targetServiceIds: ['chatgpt'],
          profileLeasePolicy: 'wait',
          action: 'tab_new',
        },
      },
    },
  };
  assert.deepEqual(
    createServiceTabRequestFromAccessPlan(accessPlan, {
      monitorRunDueSummary: {
        targetServiceIds: ['acs'],
        matched: 1,
        expiredTargetServiceIds: [],
        unverifiedTargetServiceIds: [],
        failed: false,
        recommendedAction: 'use_selected_profile',
      },
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
      monitorRunDueSummary: {
        targetServiceIds: ['acs'],
        matched: 1,
        expiredTargetServiceIds: [],
        unverifiedTargetServiceIds: [],
        failed: false,
        recommendedAction: 'use_selected_profile',
      },
      params: {
        browserHost: 'remote_headed',
        displayIsolation: 'private_virtual_display',
        url: 'https://example.com/planned',
      },
      jobTimeoutMs: 60_000,
    },
  );
  assert.deepEqual(
    createServiceTabRequestFromAccessPlan(sharedProfileAccessPlan, {
      url: 'https://chatgpt.com/',
    }),
    {
      serviceName: 'AuraCall',
      agentName: 'auracall-agent',
      taskName: 'openSharedTab',
      targetServiceIds: ['chatgpt'],
      profileLeasePolicy: 'wait',
      browserId: 'browser-shared',
      sessionName: 'shared-session',
      action: 'tab_new',
      params: {
        url: 'https://chatgpt.com/',
      },
    },
  );
  assert.deepEqual(
    createServiceTabRequestFromAccessPlan(sharedProfileAccessPlan, {
      browserId: 'browser-override',
      sessionName: 'override-session',
      url: 'https://chatgpt.com/',
    }),
    {
      serviceName: 'AuraCall',
      agentName: 'auracall-agent',
      taskName: 'openSharedTab',
      targetServiceIds: ['chatgpt'],
      profileLeasePolicy: 'wait',
      browserId: 'browser-override',
      sessionName: 'override-session',
      action: 'tab_new',
      params: {
        url: 'https://chatgpt.com/',
      },
    },
  );
  assert.throws(
    () =>
      createServiceTabRequestFromAccessPlan(manualSeedingAccessPlan, {
        url: 'https://accounts.google.com',
      }),
    /requires manual profile seeding.*agent-browser --runtime-profile google-work runtime login https:\/\/accounts.google.com/,
  );
  assert.throws(
    () =>
      createServiceTabRequestFromAccessPlan(cdpFreeAccessPlan, {
        url: 'https://www.canva.com/',
      }),
    /requires CDP-free browser operation.*createServiceCdpFreeLaunchRequest/,
  );
  assert.throws(
    () =>
      createServiceTabRequestFromAccessPlan(accessPlan, {
        monitorRunDueSummary: {
          targetServiceIds: ['acs'],
          matched: 1,
          expiredTargetServiceIds: ['acs'],
          unverifiedTargetServiceIds: [],
          failed: true,
          recommendedAction: 'probe_target_auth_or_reseed_if_needed',
        },
        url: 'https://example.com/planned',
      }),
    /expired profile freshness before tab request: acs/,
  );
  assert.throws(
    () =>
      createServiceTabRequestFromAccessPlan(accessPlan, {
        monitorRunDueSummary: {
          targetServiceIds: ['acs'],
          matched: 1,
          expiredTargetServiceIds: [],
          unverifiedTargetServiceIds: ['acs'],
          failed: true,
          recommendedAction: 'verify_or_seed_profile_before_authenticated_work',
        },
        url: 'https://example.com/planned',
      }),
    /could not verify profile freshness before tab request: acs/,
  );
  assert.throws(
    () =>
      createServiceTabRequestFromAccessPlan(accessPlan, {
        monitorRunDueSummary: {
          targetServiceIds: ['acs'],
          matched: 0,
          expiredTargetServiceIds: [],
          unverifiedTargetServiceIds: [],
          failed: false,
          recommendedAction: 'inspect_monitor_results',
        },
        url: 'https://example.com/planned',
      }),
    /requires inspection before tab request: inspect_monitor_results/,
  );
  assert.deepEqual(
    createServiceTabRequestFromAccessPlan(accessPlan, {
      allowMonitorFreshnessRisk: true,
      monitorRunDueSummary: {
        targetServiceIds: ['acs'],
        matched: 1,
        expiredTargetServiceIds: ['acs'],
        unverifiedTargetServiceIds: [],
        failed: true,
        recommendedAction: 'probe_target_auth_or_reseed_if_needed',
      },
      url: 'https://example.com/planned',
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'plannedProbeACSwebsite',
      targetServiceIds: ['acs'],
      profileLeasePolicy: 'wait',
      action: 'tab_new',
      allowMonitorFreshnessRisk: true,
      monitorRunDueSummary: {
        targetServiceIds: ['acs'],
        matched: 1,
        expiredTargetServiceIds: ['acs'],
        unverifiedTargetServiceIds: [],
        failed: true,
        recommendedAction: 'probe_target_auth_or_reseed_if_needed',
      },
      params: {
        browserHost: 'remote_headed',
        displayIsolation: 'private_virtual_display',
        url: 'https://example.com/planned',
      },
    },
  );
  assert.deepEqual(
    createServiceCdpFreeLaunchRequest({
      accessPlan: cdpFreeAccessPlan,
      url: 'https://www.canva.com/',
      jobTimeoutMs: 60_000,
    }),
    {
      serviceName: 'CanvaCLI',
      agentName: 'article-probe-agent',
      taskName: 'openCanva',
      targetServiceIds: ['canva'],
      profileLeasePolicy: 'wait',
      requiresCdpFree: true,
      cdpAttachmentAllowed: false,
      action: 'cdp_free_launch',
      url: 'https://www.canva.com/',
      params: {
        url: 'https://www.canva.com/',
      },
      jobTimeoutMs: 60_000,
    },
  );
  assert.throws(
    () =>
      createServiceCdpFreeLaunchRequest({
        accessPlan,
        url: 'https://example.com/',
      }),
    /does not require CDP-free browser operation/,
  );
  assert.deepEqual(
    createServiceCdpFreeLaunchRequest({
      accessPlan: cdpFreeAccessPlan,
      allowMonitorFreshnessRisk: true,
      monitorRunDueSummary: {
        targetServiceIds: ['canva'],
        matched: 1,
        expiredTargetServiceIds: [],
        unverifiedTargetServiceIds: ['canva'],
        failed: true,
        recommendedAction: 'verify_or_seed_profile_before_authenticated_work',
      },
      url: 'https://www.canva.com/',
    }).monitorRunDueSummary,
    {
      targetServiceIds: ['canva'],
      matched: 1,
      expiredTargetServiceIds: [],
      unverifiedTargetServiceIds: ['canva'],
      failed: true,
      recommendedAction: 'verify_or_seed_profile_before_authenticated_work',
    },
  );
  assert.throws(
    () =>
      createServiceCdpFreeLaunchRequest({
        accessPlan: cdpFreeAccessPlan,
        monitorRunDueSummary: {
          targetServiceIds: ['canva'],
          matched: 1,
          expiredTargetServiceIds: [],
          unverifiedTargetServiceIds: ['canva'],
          failed: true,
          recommendedAction: 'verify_or_seed_profile_before_authenticated_work',
        },
        url: 'https://www.canva.com/',
      }),
    /could not verify profile freshness before CDP-free launch: canva/,
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
      blockedByManualAction: true,
      manualSeedingRequired: true,
      allowManualAction: true,
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
        browserHost: 'remote_headed',
        displayIsolation: 'private_virtual_display',
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

  const tabHandle = {
    browserId: 'session:acs',
    sessionName: 'acs',
    tabId: 'target:target-1',
    targetId: 'target-1',
    url: 'https://example.com/new',
    title: 'Example',
    profileId: 'acs-work',
    profileOrigin: 'agent_browser_owned',
    leaseId: 'acs',
    leaseState: 'shared',
    cleanupPolicy: 'close_tabs',
    leaseHeartbeatExpected: true,
    ownerSessionId: 'acs',
    jobId: 'job-tab',
    traceFilter: {
      browserId: 'session:acs',
      profileId: 'acs-work',
      sessionId: 'acs',
    },
    valid: true,
    staleReason: null,
  };
  const tabRecorder = createFetchRecorder({
    success: true,
    data: { jobId: 'job-tab', serviceTabHandle: tabHandle },
  });
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
  assert.deepEqual(tabResponse, { success: true, data: { jobId: 'job-tab', serviceTabHandle: tabHandle } });
  assert.deepEqual(getServiceTabHandle(tabResponse), tabHandle);
  assert.deepEqual(requireServiceTabHandle(tabResponse), tabHandle);
  assert.deepEqual(getServiceTabHandle(tabResponse.data), tabHandle);
  assert.deepEqual(
    {
      browserId: requireServiceTabHandle(tabResponse).browserId,
      tabId: requireServiceTabHandle(tabResponse).tabId,
      targetId: requireServiceTabHandle(tabResponse).targetId,
      profileId: requireServiceTabHandle(tabResponse).profileId,
    },
    {
      browserId: 'session:acs',
      tabId: 'target:target-1',
      targetId: 'target-1',
      profileId: 'acs-work',
    },
  );
  assert.throws(
    () =>
      requireServiceTabHandle({
        success: true,
        data: {
          serviceTabHandle: {
            ...tabHandle,
            valid: false,
            staleReason: 'tab_closed',
          },
        },
      }),
    /service tab handle is stale: tab_closed/,
  );
  assert.deepEqual(
    createServiceCdpAttachRequest({
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      serviceTabHandle: tabHandle,
      cdpAttachmentAllowed: true,
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      action: 'cdp_attach',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      cdpAttachmentAllowed: true,
      serviceTabHandle: tabHandle,
    },
  );
  assert.throws(
    () =>
      createServiceCdpAttachRequest({
        serviceTabHandle: tabHandle,
        cdpAttachmentAllowed: false,
      }),
    /requires cdpAttachmentAllowed=true/,
  );
  assert.throws(
    () =>
      createServiceCdpAttachRequest({
        serviceTabHandle: {
          ...tabHandle,
          valid: false,
          staleReason: 'tab_closed',
        },
        cdpAttachmentAllowed: true,
      }),
    /service tab handle is stale: tab_closed/,
  );
  assert.deepEqual(
    createServiceCdpDetachRequest({
      serviceName: 'JournalDownloader',
      serviceTabHandle: tabHandle,
    }),
    {
      serviceName: 'JournalDownloader',
      action: 'cdp_detach',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      serviceTabHandle: tabHandle,
    },
  );
  const cdpAttachRecorder = createFetchRecorder({
    success: true,
    data: {
      attached: true,
      browserId: 'session:acs',
      tabId: 'target:target-1',
      targetId: 'target-1',
      browserWebSocketUrl: 'ws://127.0.0.1:9222/devtools/browser/test',
      detachAction: 'cdp_detach',
      browserProcessPreserved: true,
    },
  });
  await requestServiceCdpAttach({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: cdpAttachRecorder.fetch,
    serviceName: 'JournalDownloader',
    serviceTabHandle: tabHandle,
    cdpAttachmentAllowed: true,
  });
  assert.equal(cdpAttachRecorder.calls[0].body.action, 'cdp_attach');
  assert.deepEqual(cdpAttachRecorder.calls[0].body.serviceTabHandle, tabHandle);
  const cdpAttachAliasRecorder = createFetchRecorder({ success: true, data: { attached: true } });
  await attachServiceTabCdp({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: cdpAttachAliasRecorder.fetch,
    serviceTabHandle: tabHandle,
    cdpAttachmentAllowed: true,
  });
  assert.equal(cdpAttachAliasRecorder.calls[0].body.action, 'cdp_attach');
  const cdpDetachRecorder = createFetchRecorder({
    success: true,
    data: {
      detached: true,
      browserProcessPreserved: true,
      closeBrowserOnDetach: false,
    },
  });
  await requestServiceCdpDetach({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: cdpDetachRecorder.fetch,
    serviceTabHandle: tabHandle,
  });
  assert.equal(cdpDetachRecorder.calls[0].body.action, 'cdp_detach');
  assert.equal(cdpDetachRecorder.calls[0].body.browserId, 'session:acs');
  assert.deepEqual(
    createServiceEvaluateRequest({
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      serviceTabHandle: tabHandle,
      script: 'document.title',
      timeoutMs: 1000,
      maxReturnBytes: 128,
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      action: 'evaluate',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      script: 'document.title',
      returnByValue: true,
      timeoutMs: 1000,
      maxReturnBytes: 128,
      serviceTabHandle: tabHandle,
    },
  );
  assert.throws(
    () =>
      createServiceEvaluateRequest({
        serviceTabHandle: tabHandle,
        timeoutMs: 1000,
        maxReturnBytes: 128,
      }),
    /requires script or expression/,
  );
  assert.throws(
    () =>
      createServiceEvaluateRequest({
        serviceTabHandle: tabHandle,
        script: 'document.title',
        timeoutMs: 0,
        maxReturnBytes: 128,
      }),
    /positive timeoutMs/,
  );
  assert.throws(
    () =>
      createServiceEvaluateRequest({
        serviceTabHandle: {
          ...tabHandle,
          valid: false,
          staleReason: 'tab_closed',
        },
        script: 'document.title',
        timeoutMs: 1000,
        maxReturnBytes: 128,
      }),
    /service tab handle is stale: tab_closed/,
  );
  const evaluateRecorder = createFetchRecorder({
    success: true,
    data: {
      ok: true,
      action: 'evaluate',
      result: 'Example',
      resultTruncated: false,
      resultBytes: 9,
      maxReturnBytes: 128,
    },
  });
  await requestServiceEvaluate({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: evaluateRecorder.fetch,
    serviceTabHandle: tabHandle,
    expression: 'document.title',
    timeoutMs: 1000,
    maxReturnBytes: 128,
  });
  assert.equal(evaluateRecorder.calls[0].body.action, 'evaluate');
  assert.equal(evaluateRecorder.calls[0].body.script, 'document.title');
  assert.equal(evaluateRecorder.calls[0].body.returnByValue, true);
  assert.deepEqual(evaluateRecorder.calls[0].body.serviceTabHandle, tabHandle);
  const evaluateAliasRecorder = createFetchRecorder({ success: true, data: { ok: true } });
  await evaluateServiceTab({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: evaluateAliasRecorder.fetch,
    serviceTabHandle: tabHandle,
    script: 'document.title',
    timeoutMs: 1000,
    maxReturnBytes: 128,
  });
  assert.equal(evaluateAliasRecorder.calls[0].body.action, 'evaluate');
  assert.deepEqual(
    createServiceDiagnosticsRequest({
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      serviceTabHandle: tabHandle,
      includeScreenshot: true,
      screenshotDir: '/tmp/agent-browser-diagnostics',
      maxConsoleEntries: 5,
      maxErrorEntries: 3,
      maxRequestEntries: 4,
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      action: 'diagnostics',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      serviceTabHandle: tabHandle,
      includeScreenshot: true,
      screenshotDir: '/tmp/agent-browser-diagnostics',
      maxConsoleEntries: 5,
      maxErrorEntries: 3,
      maxRequestEntries: 4,
    },
  );
  assert.throws(
    () =>
      createServiceDiagnosticsRequest({
        serviceTabHandle: {
          ...tabHandle,
          valid: false,
          staleReason: 'tab_closed',
        },
      }),
    /service tab handle is stale: tab_closed/,
  );
  const diagnosticsRecorder = createFetchRecorder({
    success: true,
    data: {
      ok: true,
      action: 'diagnostics',
      compact: true,
      browserId: 'session:acs',
      tabId: 'target:target-1',
      serviceTabHandle: tabHandle,
      console: { count: 0, returned: 0, messages: [] },
      errors: { count: 0, returned: 0, errors: [] },
      requests: { count: 0, returned: 0, items: [] },
    },
  });
  await requestServiceDiagnostics({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: diagnosticsRecorder.fetch,
    serviceTabHandle: tabHandle,
    maxConsoleEntries: 5,
  });
  assert.equal(diagnosticsRecorder.calls[0].body.action, 'diagnostics');
  assert.equal(diagnosticsRecorder.calls[0].body.maxConsoleEntries, 5);
  assert.deepEqual(diagnosticsRecorder.calls[0].body.serviceTabHandle, tabHandle);
  const diagnosticsAliasRecorder = createFetchRecorder({ success: true, data: { ok: true } });
  await getServiceTabDiagnostics({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: diagnosticsAliasRecorder.fetch,
    serviceTabHandle: tabHandle,
  });
  assert.equal(diagnosticsAliasRecorder.calls[0].body.action, 'diagnostics');
  const probeRecipe = {
    detectors: [
      { id: 'title', type: 'url_title' },
      { id: 'identity', type: 'selector_text', selector: '[data-account]' },
    ],
    expectedIdentity: 'acct@example.test',
    recordFreshness: {
      targetServiceId: 'acs',
      accountId: 'acct@example.test',
      profileId: 'journal-acs',
    },
  };
  assert.deepEqual(
    createServiceProbeRequest({
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      serviceTabHandle: tabHandle,
      probe: probeRecipe,
      timeoutMs: 1000,
      maxReturnBytes: 256,
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      action: 'probe',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      timeoutMs: 1000,
      maxReturnBytes: 256,
      serviceTabHandle: tabHandle,
      probe: probeRecipe,
    },
  );
  assert.throws(
    () =>
      createServiceProbeRequest({
        serviceTabHandle: tabHandle,
        probe: {},
        timeoutMs: 1000,
        maxReturnBytes: 128,
      }),
    /probe.detectors array/,
  );
  assert.throws(
    () =>
      createServiceProbeRequest({
        serviceTabHandle: {
          ...tabHandle,
          valid: false,
          staleReason: 'tab_closed',
        },
        probe: { detectors: [{ id: 'title', type: 'url_title' }] },
        timeoutMs: 1000,
        maxReturnBytes: 128,
      }),
    /service tab handle is stale: tab_closed/,
  );
  const probeRecorder = createFetchRecorder({
    success: true,
    data: {
      ok: true,
      action: 'probe',
      observedAt: '2026-06-14T00:00:00Z',
      identity: { confidence: 'high', detectedIdentity: 'acct@example.test' },
      detectors: [{ id: 'title', type: 'url_title', ok: true }],
      serviceTabHandle: tabHandle,
    },
  });
  await requestServiceProbe({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: probeRecorder.fetch,
    serviceTabHandle: tabHandle,
    probe: probeRecipe,
    timeoutMs: 1000,
    maxReturnBytes: 256,
  });
  assert.equal(probeRecorder.calls[0].body.action, 'probe');
  assert.deepEqual(probeRecorder.calls[0].body.probe, probeRecipe);
  assert.equal(probeRecorder.calls[0].body.timeoutMs, 1000);
  assert.equal(probeRecorder.calls[0].body.maxReturnBytes, 256);
  const probeAliasRecorder = createFetchRecorder({ success: true, data: { ok: true } });
  await probeServiceTab({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: probeAliasRecorder.fetch,
    serviceTabHandle: tabHandle,
    probe: { detectors: [{ id: 'title', type: 'url_title' }] },
    timeoutMs: 1000,
    maxReturnBytes: 128,
  });
  assert.equal(probeAliasRecorder.calls[0].body.action, 'probe');
  const uiAction = {
    recipeId: 'generic-ui',
    steps: [
      { id: 'find-main', type: 'find', selector: 'main' },
      { id: 'fill-query', type: 'fill', selector: '#query', value: 'search text' },
    ],
  };
  assert.deepEqual(
    createServiceUiActionRequest({
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      serviceTabHandle: tabHandle,
      uiAction,
      timeoutMs: 1000,
      maxTextBytes: 256,
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      action: 'ui_action',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      timeoutMs: 1000,
      maxTextBytes: 256,
      serviceTabHandle: tabHandle,
      uiAction,
    },
  );
  assert.throws(
    () =>
      createServiceUiActionRequest({
        serviceTabHandle: tabHandle,
        uiAction: { steps: [] },
        timeoutMs: 1000,
      }),
    /uiAction\.steps/,
  );
  assert.throws(
    () =>
      createServiceUiActionRequest({
        serviceTabHandle: { ...tabHandle, valid: false, staleReason: 'tab_closed' },
        uiAction,
        timeoutMs: 1000,
      }),
    /service tab handle is stale: tab_closed/,
  );
  const uiActionRecorder = createFetchRecorder({
    success: true,
    data: { ok: true, action: 'ui_action', steps: [{ id: 'find-main', ok: true }] },
  });
  await requestServiceUiAction({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: uiActionRecorder.fetch,
    serviceTabHandle: tabHandle,
    uiAction,
    timeoutMs: 1000,
  });
  assert.equal(uiActionRecorder.calls[0].body.action, 'ui_action');
  assert.deepEqual(uiActionRecorder.calls[0].body.uiAction, uiAction);
  const uiActionAliasRecorder = createFetchRecorder({ success: true, data: { ok: true } });
  await runServiceUiAction({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: uiActionAliasRecorder.fetch,
    serviceTabHandle: tabHandle,
    uiAction,
    timeoutMs: 1000,
  });
  assert.equal(uiActionAliasRecorder.calls[0].body.action, 'ui_action');
  const networkCapture = {
    recipeId: 'generic-network',
    urlPatterns: ['/api/data'],
    methods: ['GET'],
    resourceTypes: ['Fetch', 'XHR'],
    status: '2xx',
    maxEvents: 2,
  };
  assert.deepEqual(
    createServiceNetworkCaptureRequest({
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      serviceTabHandle: tabHandle,
      networkCapture,
      timeoutMs: 2000,
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      action: 'network_capture',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      timeoutMs: 2000,
      serviceTabHandle: tabHandle,
      networkCapture,
    },
  );
  assert.throws(
    () =>
      createServiceNetworkCaptureRequest({
        serviceTabHandle: tabHandle,
        networkCapture: { maxEvents: 0 },
        timeoutMs: 1000,
      }),
    /networkCapture\.maxEvents/,
  );
  assert.throws(
    () =>
      createServiceNetworkCaptureRequest({
        serviceTabHandle: tabHandle,
        networkCapture: { maxEvents: 1, captureBodies: true },
        timeoutMs: 1000,
      }),
    /maxBodyBytes/,
  );
  assert.throws(
    () =>
      createServiceNetworkCaptureRequest({
        serviceTabHandle: { ...tabHandle, valid: false, staleReason: 'tab_closed' },
        networkCapture,
        timeoutMs: 1000,
      }),
    /service tab handle is stale: tab_closed/,
  );
  const networkCaptureRecorder = createFetchRecorder({
    success: true,
    data: { ok: true, action: 'network_capture', events: [] },
  });
  await requestServiceNetworkCapture({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: networkCaptureRecorder.fetch,
    serviceTabHandle: tabHandle,
    networkCapture: { ...networkCapture, captureBodies: true, maxBodyBytes: 128 },
    timeoutMs: 1000,
  });
  assert.equal(networkCaptureRecorder.calls[0].body.action, 'network_capture');
  assert.equal(networkCaptureRecorder.calls[0].body.maxBodyBytes, 128);
  const networkCaptureAliasRecorder = createFetchRecorder({ success: true, data: { ok: true } });
  await captureServiceNetwork({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: networkCaptureAliasRecorder.fetch,
    serviceTabHandle: tabHandle,
    networkCapture,
    timeoutMs: 1000,
  });
  assert.equal(networkCaptureAliasRecorder.calls[0].body.action, 'network_capture');
  const fileTransfer = {
    recipeId: 'generic-file-transfer',
    upload: {
      labelText: 'Upload report',
      files: ['/tmp/report.txt'],
      allowedPaths: ['/tmp'],
      maxFiles: 1,
    },
    download: {
      selector: '#download',
      directory: '/tmp/downloads',
      allowedDirectories: ['/tmp'],
      expectedFileName: 'report.txt',
      maxBytes: 1024,
    },
  };
  assert.deepEqual(
    createServiceFileTransferRequest({
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      serviceTabHandle: tabHandle,
      fileTransfer,
      timeoutMs: 2000,
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      action: 'file_transfer',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      timeoutMs: 2000,
      serviceTabHandle: tabHandle,
      fileTransfer,
    },
  );
  assert.throws(
    () =>
      createServiceFileTransferRequest({
        serviceTabHandle: tabHandle,
        fileTransfer: { upload: { selector: '#file', files: ['/tmp/a.txt'], maxFiles: 1 } },
        timeoutMs: 1000,
      }),
    /allowedPaths/,
  );
  assert.throws(
    () =>
      createServiceFileTransferRequest({
        serviceTabHandle: tabHandle,
        fileTransfer: {
          upload: {
            selector: '#file',
            files: ['/tmp/a.txt', '/tmp/b.txt'],
            allowedPaths: ['/tmp'],
            maxFiles: 1,
          },
        },
        timeoutMs: 1000,
      }),
    /maxFiles/,
  );
  assert.throws(
    () =>
      createServiceFileTransferRequest({
        serviceTabHandle: tabHandle,
        fileTransfer: { download: { selector: '#download', directory: '/tmp/downloads' } },
        timeoutMs: 1000,
      }),
    /allowedDirectories/,
  );
  assert.throws(
    () =>
      createServiceFileTransferRequest({
        serviceTabHandle: { ...tabHandle, valid: false, staleReason: 'tab_closed' },
        fileTransfer,
        timeoutMs: 1000,
      }),
    /service tab handle is stale: tab_closed/,
  );
  const fileTransferRecorder = createFetchRecorder({
    success: true,
    data: { ok: true, action: 'file_transfer', upload: { uploaded: 1 } },
  });
  await requestServiceFileTransfer({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: fileTransferRecorder.fetch,
    serviceTabHandle: tabHandle,
    fileTransfer,
    timeoutMs: 1000,
  });
  assert.equal(fileTransferRecorder.calls[0].body.action, 'file_transfer');
  assert.deepEqual(fileTransferRecorder.calls[0].body.fileTransfer, fileTransfer);
  const fileTransferAliasRecorder = createFetchRecorder({ success: true, data: { ok: true } });
  await transferServiceFiles({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: fileTransferAliasRecorder.fetch,
    serviceTabHandle: tabHandle,
    fileTransfer,
    timeoutMs: 1000,
  });
  assert.equal(fileTransferAliasRecorder.calls[0].body.action, 'file_transfer');
  assert.deepEqual(
    createServiceTabHandleRefreshRequest({
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      serviceTabHandle: tabHandle,
      repairPolicy: 'open_if_missing',
      url: 'https://example.com/recover',
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      action: 'tab_handle_refresh',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      repairPolicy: 'open_if_missing',
      url: 'https://example.com/recover',
      serviceTabHandle: tabHandle,
    },
  );
  const staleTabHandle = { ...tabHandle, valid: false, staleReason: 'tab_closed' };
  assert.deepEqual(
    createServiceTabHandleRefreshRequest({
      serviceTabHandle: staleTabHandle,
      repairPolicy: 'open_if_missing',
      desiredUrl: 'https://example.com/recover',
    }),
    {
      action: 'tab_handle_refresh',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      repairPolicy: 'open_if_missing',
      desiredUrl: 'https://example.com/recover',
      serviceTabHandle: staleTabHandle,
    },
  );
  assert.throws(
    () =>
      createServiceTabHandleRefreshRequest({
        serviceTabHandle: tabHandle,
        repairPolicy: 'surprise_me',
      }),
    /repairPolicy/,
  );
  const refreshRecorder = createFetchRecorder({
    success: true,
    data: {
      ok: true,
      action: 'tab_handle_refresh',
      refreshed: true,
      decision: 'exact_handle_still_valid',
      serviceTabHandle: tabHandle,
    },
  });
  await requestServiceTabHandleRefresh({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: refreshRecorder.fetch,
    serviceTabHandle: tabHandle,
    repairPolicy: 'reject_only',
  });
  assert.equal(refreshRecorder.calls[0].body.action, 'tab_handle_refresh');
  assert.equal(refreshRecorder.calls[0].body.repairPolicy, 'reject_only');
  const refreshAliasRecorder = createFetchRecorder({ success: true, data: { ok: true } });
  await refreshServiceTabHandle({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: refreshAliasRecorder.fetch,
    serviceTabHandle: tabHandle,
    repairPolicy: 'reuse_compatible',
  });
  assert.equal(refreshAliasRecorder.calls[0].body.action, 'tab_handle_refresh');
  assert.deepEqual(
    createServiceTabHandleReleaseRequest({
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'releaseACSwebsite',
      serviceTabHandle: tabHandle,
    }),
    {
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'releaseACSwebsite',
      action: 'tab_handle_release',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      serviceTabHandle: tabHandle,
    },
  );
  assert.deepEqual(
    createServiceTabHandleReleaseRequest({
      serviceTabHandle: staleTabHandle,
    }),
    {
      action: 'tab_handle_release',
      browserId: 'session:acs',
      sessionName: 'acs',
      targetId: 'target-1',
      serviceTabHandle: staleTabHandle,
    },
  );
  const releaseRecorder = createFetchRecorder({
    success: true,
    data: {
      ok: true,
      action: 'tab_handle_release',
      released: true,
      browserProcessPreserved: true,
      sessionRoutePreserved: true,
    },
  });
  await requestServiceTabHandleRelease({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: releaseRecorder.fetch,
    serviceTabHandle: tabHandle,
  });
  assert.equal(releaseRecorder.calls[0].body.action, 'tab_handle_release');
  const releaseAliasRecorder = createFetchRecorder({ success: true, data: { ok: true } });
  await releaseServiceTabHandle({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: releaseAliasRecorder.fetch,
    serviceTabHandle: tabHandle,
  });
  assert.equal(releaseAliasRecorder.calls[0].body.action, 'tab_handle_release');
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
  const tabAliasRecorder = createFetchRecorder({
    success: true,
    data: {
      serviceTabHandle: tabHandle,
    },
  });
  await requestServiceTabFromAccessPlan({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: tabAliasRecorder.fetch,
    accessPlan,
    url: 'https://example.com/alias',
  });
  assert.equal(tabAliasRecorder.calls[0].body.action, 'tab_new');
  assert.equal(tabAliasRecorder.calls[0].body.params.url, 'https://example.com/alias');
  const cdpFreeLaunchRecorder = createFetchRecorder({
    success: true,
    data: {
      launched: true,
      cdpFree: true,
      cdpAttachmentAllowed: false,
      browserId: 'session:canva',
      browserPid: 4242,
      userDataDir: '/tmp/canva',
      supportedOperations: ['process_lifecycle', 'profile_lease', 'service_state'],
      unsupportedOperations: ['cdp_commands', 'snapshot', 'screenshot', 'dom_interaction'],
      unsupportedCommands: SERVICE_REQUEST_ACTIONS.filter((action) => action !== 'cdp_free_launch'),
    },
  });
  const cdpFreeLaunchResponse = await requestServiceCdpFreeLaunch({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: cdpFreeLaunchRecorder.fetch,
    accessPlan: cdpFreeAccessPlan,
    url: 'https://www.canva.com/',
  });
  assert.equal(cdpFreeLaunchResponse.data.cdpFree, true);
  assert.ok(cdpFreeLaunchResponse.data.unsupportedCommands.includes('snapshot'));
  assert.ok(cdpFreeLaunchResponse.data.unsupportedCommands.includes('click'));
  const cdpFreeAvailability = summarizeServiceCdpFreeLaunchAvailability(cdpFreeLaunchResponse.data);
  assert.deepEqual(cdpFreeAvailability.availableCommands, ['cdp_free_launch']);
  assert.equal(cdpFreeAvailability.unsupportedCommands.includes('snapshot'), true);
  assert.equal(cdpFreeAvailability.unsupportedCommands.includes('click'), true);
  assert.equal(cdpFreeAvailability.controlPlaneMode, 'cdp_free');
  assert.equal(cdpFreeAvailability.lifecycleOnly, true);
  assert.equal(cdpFreeAvailability.hasUnsupportedCommandList, true);
  assert.equal(isServiceCdpFreeActionAvailable(cdpFreeLaunchResponse.data, 'cdp_free_launch'), true);
  assert.equal(isServiceCdpFreeActionAvailable(cdpFreeLaunchResponse.data, 'snapshot'), false);
  assert.deepEqual(cdpFreeLaunchRecorder.calls[0].body, {
    serviceName: 'CanvaCLI',
    agentName: 'article-probe-agent',
    taskName: 'openCanva',
    targetServiceIds: ['canva'],
    profileLeasePolicy: 'wait',
    requiresCdpFree: true,
    cdpAttachmentAllowed: false,
    action: 'cdp_free_launch',
    url: 'https://www.canva.com/',
    params: {
      url: 'https://www.canva.com/',
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
      browserHost: 'remote_headed',
      displayIsolation: 'private_virtual_display',
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

  const overrideTabRecorder = createFetchRecorder({ success: true, data: { jobId: 'job-manual-override' } });
  const overrideTabResponse = await requestServiceTab({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: overrideTabRecorder.fetch,
    accessPlan: manualSeedingAccessPlan,
    allowManualAction: true,
    url: 'https://accounts.google.com',
  });
  assert.deepEqual(overrideTabResponse, { success: true, data: { jobId: 'job-manual-override' } });
  assert.deepEqual(overrideTabRecorder.calls[0].body, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'seedThenProbeGoogle',
    targetServiceIds: ['google'],
    profileLeasePolicy: 'wait',
    action: 'tab_new',
    blockedByManualAction: true,
    manualSeedingRequired: true,
    allowManualAction: true,
    params: {
      url: 'https://accounts.google.com',
    },
  });

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

  const cdpFreeAccessPlanWorkflow = createCdpFreeAccessPlanToServiceRequestFetchRecorder();
  const observedCdpFreeAccessPlan = await getServiceAccessPlan({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: cdpFreeAccessPlanWorkflow.fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
  });
  assert.equal(
    observedCdpFreeAccessPlan.decision.serviceRequest.cdpFreeAvailability.availableCommands.includes(
      'cdp_free_launch',
    ),
    true,
  );
  const accessPlanCdpFreeResponse = await requestServiceCdpFreeLaunch({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: cdpFreeAccessPlanWorkflow.fetch,
    accessPlan: observedCdpFreeAccessPlan,
    url: 'https://www.canva.com/',
    jobTimeoutMs: 120_000,
  });
  assert.deepEqual(accessPlanCdpFreeResponse, {
    success: true,
    data: { jobId: 'job-access-plan-cdp-free' },
  });
  assert.equal(cdpFreeAccessPlanWorkflow.calls.length, 2);
  assert.equal(
    cdpFreeAccessPlanWorkflow.calls[0].url,
    'http://127.0.0.1:4849/api/service/access-plan?serviceName=CanvaCLI&agentName=canva-cli-agent&taskName=openCanvaWorkspace&loginId=canva&targetServiceId=canva',
  );
  assert.deepEqual(cdpFreeAccessPlanWorkflow.calls[1].body, {
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    profileLeasePolicy: 'wait',
    requiresCdpFree: true,
    cdpAttachmentAllowed: false,
    action: 'cdp_free_launch',
    url: 'https://www.canva.com/',
    params: {
      url: 'https://www.canva.com/',
    },
    jobTimeoutMs: 120_000,
  });

  const takeoverRequest = createServiceRequest({
    serviceName: 'agent-browser-dashboard',
    agentName: 'operator',
    taskName: 'workspace-viewport-takeover',
    action: 'view_takeover',
    params: {
      browserId: 'session:rdp-hardening-a',
      sessionName: 'rdp-hardening-a',
      streamId: 'remote-headed-view',
      provider: 'rdp_gateway',
      openMode: 'iframe',
    },
    jobTimeoutMs: 5000,
  });
  assert.equal(takeoverRequest.action, 'view_takeover');
  assert.equal(takeoverRequest.params.browserId, 'session:rdp-hardening-a');
  assert.deepEqual(createServiceRequestMcpToolCall(takeoverRequest), {
    name: 'service_request',
    arguments: takeoverRequest,
  });

  const routeCheckoutRequest = createServiceRemoteViewRouteCheckoutRequest({
    serviceName: 'agent-browser-dashboard',
    agentName: 'operator',
    taskName: 'route-checkout',
    displayAllocationId: 'display-a',
    routePoolEntryId: 'pool-a',
    routeId: 'route-a',
    browserId: 'session:rdp-a',
    sessionName: 'rdp-a',
    streamId: 'remote-headed-view',
    provider: 'rdp_gateway',
    frameUrl: 'https://guac.example/#/client/route-a',
  });
  assert.equal(routeCheckoutRequest.action, 'service_remote_view_route_checkout');
  assert.equal(routeCheckoutRequest.serviceName, 'agent-browser-dashboard');
  assert.deepEqual(routeCheckoutRequest.params, {
    displayAllocationId: 'display-a',
    routeId: 'route-a',
    routePoolEntryId: 'pool-a',
    browserId: 'session:rdp-a',
    sessionName: 'rdp-a',
    streamId: 'remote-headed-view',
    provider: 'rdp_gateway',
    frameUrl: 'https://guac.example/#/client/route-a',
  });

  const routeOpenRequest = createServiceRemoteViewOpenRequest({
    serviceName: 'agent-browser-dashboard',
    agentName: 'operator',
    taskName: 'route-open',
    displayAllocationId: 'display-a',
    routePoolEntryId: 'pool-a',
    routePoolEntry: {
      id: 'pool-a',
      routeId: 'route-a',
      frameUrl: 'https://guac.example/#/client/route-a',
    },
    routePool: [
      {
        id: 'pool-a',
        routeId: 'route-a',
      },
    ],
    routeId: 'route-a',
    browserId: 'session:rdp-a',
    sessionName: 'rdp-a',
    streamId: 'remote-headed-view',
    provider: 'rdp_gateway',
    frameUrl: 'https://guac.example/#/client/route-a',
    url: 'https://www.linkedin.com/',
  });
  assert.equal(routeOpenRequest.action, 'remote_view_open');
  assert.equal(routeOpenRequest.serviceName, 'agent-browser-dashboard');
  assert.deepEqual(routeOpenRequest.params, {
    displayAllocationId: 'display-a',
    routeId: 'route-a',
    routePoolEntryId: 'pool-a',
    routePoolEntry: {
      id: 'pool-a',
      routeId: 'route-a',
      frameUrl: 'https://guac.example/#/client/route-a',
    },
    routePool: [
      {
        id: 'pool-a',
        routeId: 'route-a',
      },
    ],
    browserId: 'session:rdp-a',
    sessionName: 'rdp-a',
    streamId: 'remote-headed-view',
    provider: 'rdp_gateway',
    frameUrl: 'https://guac.example/#/client/route-a',
    url: 'https://www.linkedin.com/',
  });

  const routeReleaseRequest = createServiceRemoteViewRouteReleaseRequest({
    serviceName: 'agent-browser-dashboard',
    routeId: 'route-a',
  });
  assert.equal(routeReleaseRequest.action, 'service_remote_view_route_release');
  assert.deepEqual(routeReleaseRequest.params, { routeId: 'route-a' });

  const routePoolRepairRequest = createServiceRoutePoolRepairRequest({
    serviceName: 'agent-browser-dashboard',
    apply: false,
    staleCheckouts: true,
    serviceState: {
      routePool: {},
    },
  });
  assert.equal(routePoolRepairRequest.action, 'service_route_pool_repair');
  assert.deepEqual(routePoolRepairRequest.params, {
    apply: false,
    staleCheckouts: true,
    serviceState: {
      routePool: {},
    },
  });

  const viewerLeaseRequest = createServiceViewerLeaseRequest({
    serviceName: 'agent-browser-dashboard',
    routeId: 'route-a',
    viewerId: 'viewer-a',
    viewerName: 'Operator A',
    openMode: 'tile',
  });
  assert.equal(viewerLeaseRequest.action, 'service_viewer_lease_request');
  assert.deepEqual(viewerLeaseRequest.params, {
    routeId: 'route-a',
    viewerId: 'viewer-a',
    viewerName: 'Operator A',
    openMode: 'tile',
  });

  const controllerTakeoverRequest = createServiceControllerLeaseTakeoverRequest({
    serviceName: 'agent-browser-dashboard',
    routeId: 'route-a',
    viewerLeaseId: 'viewer-a',
    viewerId: 'operator-a',
  });
  assert.equal(controllerTakeoverRequest.action, 'service_controller_lease_takeover');
  assert.deepEqual(controllerTakeoverRequest.params, {
    routeId: 'route-a',
    viewerLeaseId: 'viewer-a',
    viewerId: 'operator-a',
  });

  const viewerLeaseHeartbeatRequest = createServiceViewerLeaseHeartbeatRequest({
    serviceName: 'agent-browser-dashboard',
    viewerLeaseId: 'viewer-a',
    expiresAt: '2026-05-28T04:00:00Z',
  });
  assert.equal(viewerLeaseHeartbeatRequest.action, 'service_viewer_lease_heartbeat');
  assert.deepEqual(viewerLeaseHeartbeatRequest.params, {
    viewerLeaseId: 'viewer-a',
    expiresAt: '2026-05-28T04:00:00Z',
  });

  const viewerLeaseReleaseRequest = createServiceViewerLeaseReleaseRequest({
    serviceName: 'agent-browser-dashboard',
    viewerLeaseId: 'viewer-a',
  });
  assert.equal(viewerLeaseReleaseRequest.action, 'service_viewer_lease_release');
  assert.deepEqual(viewerLeaseReleaseRequest.params, { viewerLeaseId: 'viewer-a' });

  const remoteViewWorkflow = createFetchRecorder({
    success: true,
    data: {
      status: 'checked_out',
      routeId: 'route-a',
      displayAllocationId: 'display-a',
    },
  });
  await requestServiceRemoteViewRouteCheckout({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: remoteViewWorkflow.fetch,
    displayAllocationId: 'display-a',
    routeId: 'route-a',
  });
  assert.equal(remoteViewWorkflow.calls[0].body.action, 'service_remote_view_route_checkout');
  assert.deepEqual(remoteViewWorkflow.calls[0].body.params, {
    displayAllocationId: 'display-a',
    routeId: 'route-a',
  });

  const remoteViewOpenWorkflow = createFetchRecorder({
    success: true,
    data: {
      status: 'opened',
      routeId: 'route-a',
      displayAllocationId: 'display-a',
    },
  });
  await requestServiceRemoteViewOpen({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: remoteViewOpenWorkflow.fetch,
    displayAllocationId: 'display-a',
    routeId: 'route-a',
    routePoolEntry: {
      id: 'pool-a',
      routeId: 'route-a',
      frameUrl: 'https://guac.example/#/client/route-a',
    },
    routePool: [
      {
        id: 'pool-a',
        routeId: 'route-a',
      },
    ],
    url: 'https://www.linkedin.com/',
  });
  assert.equal(remoteViewOpenWorkflow.calls[0].body.action, 'remote_view_open');
  assert.deepEqual(remoteViewOpenWorkflow.calls[0].body.params, {
    displayAllocationId: 'display-a',
    routeId: 'route-a',
    routePoolEntry: {
      id: 'pool-a',
      routeId: 'route-a',
      frameUrl: 'https://guac.example/#/client/route-a',
    },
    routePool: [
      {
        id: 'pool-a',
        routeId: 'route-a',
      },
    ],
    url: 'https://www.linkedin.com/',
  });

  const routePoolRepairWorkflow = createFetchRecorder({
    success: true,
    data: {
      repaired: false,
      dryRun: true,
      candidateCounts: {
        staleCheckouts: 1,
        total: 1,
      },
      repairedCounts: {
        staleCheckouts: 0,
        total: 0,
      },
    },
  });
  await requestServiceRoutePoolRepair({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: routePoolRepairWorkflow.fetch,
    apply: false,
    staleCheckouts: true,
  });
  assert.equal(routePoolRepairWorkflow.calls[0].body.action, 'service_route_pool_repair');
  assert.deepEqual(routePoolRepairWorkflow.calls[0].body.params, {
    apply: false,
    staleCheckouts: true,
  });

  const viewerWorkflow = createFetchRecorder({
    success: true,
    data: {
      status: 'viewer_connected',
      routeId: 'route-a',
      viewerLeaseId: 'viewer-a',
    },
  });
  await requestServiceViewerLease({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: viewerWorkflow.fetch,
    routeId: 'route-a',
    viewerId: 'viewer-a',
  });
  assert.equal(viewerWorkflow.calls[0].body.action, 'service_viewer_lease_request');

  const heartbeatWorkflow = createFetchRecorder({
    success: true,
    data: {
      status: 'viewer_heartbeat',
      viewerLeaseId: 'viewer-a',
    },
  });
  await heartbeatServiceViewerLease({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: heartbeatWorkflow.fetch,
    viewerLeaseId: 'viewer-a',
  });
  assert.equal(heartbeatWorkflow.calls[0].body.action, 'service_viewer_lease_heartbeat');

  const takeoverWorkflow = createFetchRecorder({
    success: true,
    data: {
      status: 'controller_taken',
      routeId: 'route-a',
      viewerLeaseId: 'viewer-a',
    },
  });
  await takeoverServiceControllerLease({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: takeoverWorkflow.fetch,
    routeId: 'route-a',
    viewerLeaseId: 'viewer-a',
  });
  assert.equal(
    takeoverWorkflow.calls[0].body.action,
    'service_controller_lease_takeover',
  );

  const releaseWorkflow = createFetchRecorder({
    success: true,
    data: {
      status: 'released',
      viewerLeaseId: 'viewer-a',
    },
  });
  await releaseServiceViewerLease({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: releaseWorkflow.fetch,
    viewerLeaseId: 'viewer-a',
  });
  assert.equal(releaseWorkflow.calls[0].body.action, 'service_viewer_lease_release');

  console.log('Service request client helper tests passed');
}

main().catch((err) => {
  console.error(err?.stack || err?.message || String(err));
  process.exit(1);
});
