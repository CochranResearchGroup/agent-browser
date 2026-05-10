#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { getServiceAccessPlan } from '../packages/client/src/service-observability.js';

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  readResourceContents,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-access-plan-',
  sessionPrefix: 'access-plan',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
context.env.AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS = '0';

const { agentHome, session, tempHome } = context;
const serviceName = 'AccessPlanSmoke';
const agentName = 'codex';
const taskName = 'planGoogleAccess';
const targetServiceId = 'google';
const profileId = `access-plan-google-${process.pid}`;
const sitePolicyId = 'google';
const providerId = 'manual';
const challengeId = `access-plan-challenge-${process.pid}`;
const monitoredTargetServiceId = 'acs';
const monitoredProfileId = `access-plan-acs-${process.pid}`;
const monitoredMonitorId = `access-plan-acs-freshness-${process.pid}`;

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    if (process.env.AGENT_BROWSER_SMOKE_KEEP_HOME === '1') {
      console.error(`Keeping smoke home: ${tempHome}`);
    } else {
      context.cleanupTempHome();
    }
  }
}

function seedServiceState() {
  const serviceDir = join(agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    join(serviceDir, 'state.json'),
    `${JSON.stringify(
      {
        profiles: {
          [profileId]: {
            id: profileId,
            name: 'Access-plan Google profile',
            userDataDir: join(tempHome, 'google-profile-user-data'),
            sitePolicyIds: [sitePolicyId],
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [],
            sharedServiceIds: [serviceName],
            credentialProviderIds: [providerId],
            persistent: true,
          },
          [monitoredProfileId]: {
            id: monitoredProfileId,
            name: 'Access-plan ACS profile',
            userDataDir: join(tempHome, 'acs-profile-user-data'),
            targetServiceIds: [monitoredTargetServiceId],
            authenticatedServiceIds: [monitoredTargetServiceId],
            sharedServiceIds: [serviceName],
            targetReadiness: [
              {
                targetServiceId: monitoredTargetServiceId,
                loginId: monitoredTargetServiceId,
                state: 'fresh',
                manualSeedingRequired: false,
                evidence: 'auth_probe_cookie_present',
                recommendedAction: 'use_profile',
                lastVerifiedAt: '2026-05-01T00:00:00Z',
                freshnessExpiresAt: '2026-05-01T00:00:01Z',
              },
            ],
            persistent: true,
          },
        },
        monitors: {
          [monitoredMonitorId]: {
            id: monitoredMonitorId,
            name: 'ACS profile readiness',
            target: { profile_readiness: monitoredTargetServiceId },
            intervalMs: 60000,
            state: 'active',
            lastCheckedAt: null,
            lastSucceededAt: null,
            lastFailedAt: null,
            lastResult: null,
            consecutiveFailures: 0,
          },
        },
        sitePolicies: {
          [sitePolicyId]: {
            id: sitePolicyId,
            originPattern: 'https://accounts.google.com',
            browserHost: 'local_headed',
            interactionMode: 'human_like_input',
            manualLoginPreferred: true,
            profileRequired: true,
            authProviders: [providerId],
            challengePolicy: 'manual_only',
            allowedChallengeProviders: [providerId],
          },
        },
        providers: {
          [providerId]: {
            id: providerId,
            kind: 'manual_approval',
            displayName: 'Manual approval',
            enabled: true,
            capabilities: ['human_approval'],
          },
        },
        challenges: {
          [challengeId]: {
            id: challengeId,
            kind: 'two_factor',
            state: 'waiting_for_human',
            providerId,
            humanApproved: false,
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

async function runDueMonitors() {
  const result = await runCli(context, [
    '--json',
    '--session',
    session,
    'service',
    'monitors',
    'run-due',
  ]);
  const payload = parseJsonOutput(result.stdout, 'service monitors run-due');
  assert(payload.success === true, `service monitors run-due failed: ${result.stdout}${result.stderr}`);
  assert(payload.data?.checked === 1, `service monitors run-due checked mismatch: ${JSON.stringify(payload)}`);
  assert(payload.data?.failed === 1, `service monitors run-due failed count mismatch: ${JSON.stringify(payload)}`);
  assert(
    payload.data?.monitorIds?.includes(monitoredMonitorId),
    `service monitors run-due missing monitor ID: ${JSON.stringify(payload)}`,
  );
}

async function enableStream() {
  const streamStatusResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'stream',
    'status',
  ]);
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );

  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, [
      '--json',
      '--session',
      session,
      'stream',
      'enable',
    ]);
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`);
  }

  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream enable did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

function assertAccessPlan(data, label) {
  assert(data?.query?.serviceName === serviceName, `${label} serviceName mismatch: ${JSON.stringify(data)}`);
  assert(data?.query?.agentName === agentName, `${label} agentName mismatch: ${JSON.stringify(data)}`);
  assert(data?.query?.taskName === taskName, `${label} taskName mismatch: ${JSON.stringify(data)}`);
  assert(
    Array.isArray(data?.query?.namingWarnings) && data.query.namingWarnings.length === 0,
    `${label} naming warnings mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.query?.hasNamingWarning === false, `${label} naming warning flag mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.query?.targetServiceIds?.includes(targetServiceId),
    `${label} targetServiceIds mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.query?.sitePolicyId === sitePolicyId, `${label} sitePolicyId mismatch: ${JSON.stringify(data)}`);
  assert(data?.query?.challengeId === challengeId, `${label} challengeId mismatch: ${JSON.stringify(data)}`);
  assert(data?.selectedProfile?.id === profileId, `${label} selected profile mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.selectedProfileMatch?.reason === 'target_match',
    `${label} selected profile reason mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.selectedProfileMatch?.matchedField === 'targetServiceIds',
    `${label} matched field mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readiness?.profileId === profileId,
    `${label} readiness profile mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readiness?.targetReadiness?.[0]?.state === 'needs_manual_seeding',
    `${label} readiness state mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readiness?.targetReadiness?.[0]?.seedingMode === 'detached_headed_no_cdp',
    `${label} readiness seeding mode mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readiness?.targetReadiness?.[0]?.cdpAttachmentAllowedDuringSeeding === false,
    `${label} readiness CDP attachment policy mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readiness?.targetReadiness?.[0]?.preferredKeyring === 'basic_password_store',
    `${label} readiness keyring preference mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readiness?.targetReadiness?.[0]?.setupScopes?.includes('signin'),
    `${label} readiness setup scopes missing signin: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readinessSummary?.manualSeedingRequired === true,
    `${label} readiness summary mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.seedingHandoff?.profileId === profileId, `${label} seeding handoff profile mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.seedingHandoff?.targetServiceId === targetServiceId,
    `${label} seeding handoff target mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.seedingMode === 'detached_headed_no_cdp',
    `${label} seeding handoff mode mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.cdpAttachmentAllowedDuringSeeding === false,
    `${label} seeding handoff CDP policy mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.preferredKeyring === 'basic_password_store',
    `${label} seeding handoff keyring mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.setupScopes?.includes('chrome_sync'),
    `${label} seeding handoff setup scopes mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.command ===
      `agent-browser --runtime-profile ${profileId} runtime login https://accounts.google.com`,
    `${label} seeding handoff command mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.warnings?.some((warning) => warning.includes('CDP')),
    `${label} seeding handoff warnings mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.operatorIntervention?.severity === 'action_required',
    `${label} seeding handoff operator intervention severity mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.operatorIntervention?.desktopPopupPolicy === 'optional_policy_controlled',
    `${label} seeding handoff desktop popup policy mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.operatorIntervention?.blocksProfileLease === true,
    `${label} seeding handoff lease block mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.seedingHandoff?.operatorIntervention?.actions?.some(
      (action) => action.id === 'force_close_seeded_browser' && action.safety === 'danger',
    ),
    `${label} seeding handoff force-close action mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.monitorFindings?.profileReadinessAttentionRequired === false,
    `${label} monitor findings mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    Array.isArray(data?.monitorFindings?.profileReadinessMonitorIds) &&
      data.monitorFindings.profileReadinessMonitorIds.length === 0,
    `${label} monitor IDs mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.sitePolicy?.id === sitePolicyId, `${label} site policy mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.sitePolicySource?.source === 'persisted_state',
    `${label} site policy source mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.sitePolicySource?.matchedBy === 'explicit_site_policy_id',
    `${label} site policy match source mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.providers?.[0]?.id === providerId, `${label} providers mismatch: ${JSON.stringify(data)}`);
  assert(data?.challenges?.[0]?.id === challengeId, `${label} challenges mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.decision?.recommendedAction ===
      'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
    `${label} recommended action mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.manualActionRequired === true,
    `${label} manual action flag mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.manualSeedingRequired === true,
    `${label} manual seeding flag mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.monitorAttentionRequired === false,
    `${label} monitor attention flag mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.providerIds?.includes(providerId),
    `${label} provider IDs mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.challengeIds?.includes(challengeId),
    `${label} challenge IDs mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.freshnessUpdate?.available === true,
    `${label} freshness update availability mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.freshnessUpdate?.recommendedAfterProbe === true,
    `${label} freshness update recommendation mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.freshnessUpdate?.profileId === profileId,
    `${label} freshness update profile mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.freshnessUpdate?.targetServiceIds?.includes(targetServiceId),
    `${label} freshness update target mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.freshnessUpdate?.http?.route === `/api/service/profiles/${profileId}/freshness`,
    `${label} freshness update HTTP route mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.freshnessUpdate?.http?.routeTemplate === '/api/service/profiles/<id>/freshness',
    `${label} freshness update HTTP route template mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.freshnessUpdate?.mcp?.tool === 'service_profile_freshness_update',
    `${label} freshness update MCP tool mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.freshnessUpdate?.client?.helper === 'updateServiceProfileFreshness',
    `${label} freshness update client helper mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.postSeedingProbe?.available === true,
    `${label} post-seeding probe availability mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.postSeedingProbe?.recommendedAfterClose === true,
    `${label} post-seeding probe recommendation mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.postSeedingProbe?.profileId === profileId,
    `${label} post-seeding probe profile mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.postSeedingProbe?.targetServiceId === targetServiceId,
    `${label} post-seeding probe target mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.postSeedingProbe?.client?.helper === 'verifyServiceProfileSeeding',
    `${label} post-seeding probe client helper mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.postSeedingProbe?.serviceClientExample?.script ===
      'examples/service-client/post-seeding-probe.mjs',
    `${label} post-seeding probe example mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.postSeedingProbe?.cli?.command ===
      `agent-browser service profiles ${profileId} verify-seeding ${targetServiceId} --state fresh --evidence <probe-evidence>`,
    `${label} post-seeding probe CLI mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.available === false,
    `${label} service request availability mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.recommendedAfterManualAction === true,
    `${label} service request manual-action recommendation mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.blockedByManualAction === true,
    `${label} service request manual block mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.action === 'tab_new',
    `${label} service request action mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.selectedProfileId === profileId,
    `${label} service request selected profile mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.profileLeasePolicy === 'wait',
    `${label} service request lease policy mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.request?.serviceName === serviceName &&
      data.decision.serviceRequest.request.agentName === agentName &&
      data.decision.serviceRequest.request.taskName === taskName,
    `${label} service request caller labels mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.request?.targetServiceIds?.includes(targetServiceId),
    `${label} service request target IDs mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.request?.profileLeasePolicy === 'wait',
    `${label} service request body lease policy mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.request?.blockedByManualAction === true,
    `${label} service request body manual block marker mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.request?.manualSeedingRequired === true,
    `${label} service request body manual seeding marker mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.http?.route === '/api/service/request',
    `${label} service request HTTP route mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.mcp?.tool === 'service_request',
    `${label} service request MCP tool mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.client?.helper === 'requestServiceTab',
    `${label} service request client helper mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    Array.isArray(data?.decision?.namingWarnings) && data.decision.namingWarnings.length === 0,
    `${label} decision naming warnings mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.hasNamingWarning === false,
    `${label} decision naming warning flag mismatch: ${JSON.stringify(data)}`,
  );
}

function assertMonitoredAccessPlan(data, label) {
  assert(data?.query?.serviceName === serviceName, `${label} serviceName mismatch: ${JSON.stringify(data)}`);
  assert(data?.query?.agentName === agentName, `${label} agentName mismatch: ${JSON.stringify(data)}`);
  assert(data?.query?.taskName === taskName, `${label} taskName mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.query?.targetServiceIds?.includes(monitoredTargetServiceId),
    `${label} targetServiceIds mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.selectedProfile?.id === monitoredProfileId, `${label} selected profile mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.selectedProfileMatch?.reason === 'target_match',
    `${label} selected profile reason mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readiness?.profileId === monitoredProfileId,
    `${label} readiness profile mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readiness?.targetReadiness?.[0]?.state === 'stale',
    `${label} readiness state mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.readiness?.targetReadiness?.[0]?.evidence === `freshness_expired_by_monitor:${monitoredMonitorId}`,
    `${label} readiness evidence mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.seedingHandoff === null, `${label} seeding handoff should be null: ${JSON.stringify(data)}`);
  assert(
    data?.monitorFindings?.profileReadinessAttentionRequired === true,
    `${label} monitor findings mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.monitorFindings?.profileReadinessIncidentIds?.includes(`monitor:${monitoredMonitorId}`),
    `${label} monitor incident IDs mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.monitorFindings?.profileReadinessMonitorIds?.includes(monitoredMonitorId),
    `${label} monitor IDs mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.monitorFindings?.profileReadinessResults?.includes('profile_readiness_expired'),
    `${label} monitor results mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.monitorFindings?.targetServiceIds?.includes(monitoredTargetServiceId),
    `${label} monitor target IDs mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.monitorAttentionRequired === true,
    `${label} monitor attention flag mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.recommendedAction === 'probe_target_auth_or_reseed_if_needed',
    `${label} recommended action mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.reasons?.includes('profile_readiness_monitor_attention'),
    `${label} monitor attention reason mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.freshnessUpdate?.recommendedAfterProbe === true,
    `${label} freshness update recommendation mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.postSeedingProbe?.recommendedAfterClose === true,
    `${label} post-seeding probe recommendation mismatch: ${JSON.stringify(data)}`,
  );
}

function assertAnonymousAccessPlan(data, label) {
  assert(data?.query?.serviceName === null, `${label} anonymous serviceName mismatch: ${JSON.stringify(data)}`);
  assert(data?.query?.agentName === null, `${label} anonymous agentName mismatch: ${JSON.stringify(data)}`);
  assert(data?.query?.taskName === null, `${label} anonymous taskName mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.query?.namingWarnings?.includes('missing_service_name') &&
      data.query.namingWarnings.includes('missing_agent_name') &&
      data.query.namingWarnings.includes('missing_task_name'),
    `${label} anonymous naming warnings mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.query?.hasNamingWarning === true, `${label} anonymous naming warning flag mismatch: ${JSON.stringify(data)}`);
  assert(
    JSON.stringify(data?.decision?.namingWarnings) === JSON.stringify(data?.query?.namingWarnings),
    `${label} anonymous decision naming warnings mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.decision?.hasNamingWarning === true, `${label} anonymous decision naming flag mismatch: ${JSON.stringify(data)}`);
}

function assertNoBrowserLaunchState() {
  const statePath = join(agentHome, 'service', 'state.json');
  if (!existsSync(statePath)) {
    return;
  }

  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  const jobs = Object.values(state.jobs ?? {});
  assert(
    jobs.every((job) => !['launch', 'navigate', 'tab_new'].includes(job.action)),
    `access plan recorded browser-launching jobs: ${JSON.stringify(state.jobs)}`,
  );
  assert(
    Object.keys(state.browsers ?? {}).length === 0,
    `access plan persisted browsers: ${JSON.stringify(state.browsers)}`,
  );
}

try {
  seedServiceState();
  const port = await enableStream();
  await runDueMonitors();
  const query =
    `service-name=${encodeURIComponent(serviceName)}` +
    `&agent-name=${encodeURIComponent(agentName)}` +
    `&task-name=${encodeURIComponent(taskName)}` +
    `&login-id=${encodeURIComponent(targetServiceId)}` +
    `&site-policy-id=${encodeURIComponent(sitePolicyId)}` +
    `&challenge-id=${encodeURIComponent(challengeId)}`;

  const httpPlan = await httpJson(port, 'GET', `/api/service/access-plan?${query}`);
  assert(httpPlan.success === true, `HTTP access plan failed: ${JSON.stringify(httpPlan)}`);
  assertAccessPlan(httpPlan.data, 'HTTP access plan');

  const mcpResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'mcp',
    'read',
    `agent-browser://access-plan?${query}`,
  ]);
  const mcpPlan = readResourceContents(
    parseJsonOutput(mcpResult.stdout, 'mcp access plan resource'),
    'access plan',
  );
  assertAccessPlan(mcpPlan, 'MCP access plan');

  const clientPlan = await getServiceAccessPlan({
    baseUrl: `http://127.0.0.1:${port}`,
    serviceName,
    agentName,
    taskName,
    loginId: targetServiceId,
    sitePolicyId,
    challengeId,
  });
  assertAccessPlan(clientPlan, 'client access plan');

  const monitoredQuery =
    `service-name=${encodeURIComponent(serviceName)}` +
    `&agent-name=${encodeURIComponent(agentName)}` +
    `&task-name=${encodeURIComponent(taskName)}` +
    `&login-id=${encodeURIComponent(monitoredTargetServiceId)}` +
    '&challenge-id=none';

  const monitoredHttpPlan = await httpJson(port, 'GET', `/api/service/access-plan?${monitoredQuery}`);
  assert(
    monitoredHttpPlan.success === true,
    `HTTP monitored access plan failed: ${JSON.stringify(monitoredHttpPlan)}`,
  );
  assertMonitoredAccessPlan(monitoredHttpPlan.data, 'HTTP monitored access plan');

  const monitoredMcpResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'mcp',
    'read',
    `agent-browser://access-plan?${monitoredQuery}`,
  ]);
  const monitoredMcpPlan = readResourceContents(
    parseJsonOutput(monitoredMcpResult.stdout, 'mcp monitored access plan resource'),
    'monitored access plan',
  );
  assertMonitoredAccessPlan(monitoredMcpPlan, 'MCP monitored access plan');

  const monitoredClientPlan = await getServiceAccessPlan({
    baseUrl: `http://127.0.0.1:${port}`,
    serviceName,
    agentName,
    taskName,
    loginId: monitoredTargetServiceId,
    challengeId: 'none',
  });
  assertMonitoredAccessPlan(monitoredClientPlan, 'client monitored access plan');

  const anonymousPlan = await httpJson(port, 'GET', `/api/service/access-plan?login-id=${targetServiceId}`);
  assert(anonymousPlan.success === true, `anonymous HTTP access plan failed: ${JSON.stringify(anonymousPlan)}`);
  assertAnonymousAccessPlan(anonymousPlan.data, 'anonymous HTTP access plan');

  assertNoBrowserLaunchState();

  await cleanup();
  console.log('Service access plan no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
