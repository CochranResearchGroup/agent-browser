#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import {
  assert,
  closeSession,
  createSmokeContext,
  createMcpStdioClient,
  parseJsonOutput,
  readResourceContents,
  runCli,
} from './smoke-utils.js';
import { parseMcpJsonResource } from './smoke-schema-utils.js';

const context = createSmokeContext({
  prefix: 'ab-mcp-read-no-launch-',
  sessionPrefix: 'mcp-read-no-launch',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { agentHome, session } = context;
const profileId = `mcp-read-google-${process.pid}`;
const targetServiceId = 'google';
let mcp;

const MCP_TOOL_ALLOWLIST = [
  'service_access_plan', 'service_request', 'desktop_capture', 'service_job_cancel', 'service_browser_retry',
  'service_incidents', 'service_remedies_apply', 'service_profile_upsert',
  'service_profile_delete', 'service_profile_freshness_update',
  'service_profile_seeding_handoff_update', 'service_session_upsert',
  'service_session_delete', 'service_site_policy_upsert', 'service_site_policy_delete',
  'service_monitor_upsert', 'service_monitor_delete', 'service_monitors_run_due',
  'service_monitor_pause', 'service_monitor_resume', 'service_monitor_reset_failures',
  'service_monitor_triage', 'service_provider_upsert', 'service_provider_delete',
  'service_browser_capability_registry_upsert', 'service_browser_capability_preflight',
  'service_remote_view_route_preflight', 'browser_snapshot', 'browser_get_url',
  'browser_get_title', 'browser_tabs', 'browser_screenshot', 'browser_click', 'browser_fill',
  'browser_wait', 'browser_type', 'browser_press', 'browser_hover', 'browser_select',
  'browser_get_text', 'browser_get_value', 'browser_get_attribute', 'browser_get_html',
  'browser_get_styles', 'browser_count', 'browser_get_box', 'browser_is_visible',
  'browser_is_enabled', 'browser_check', 'browser_is_checked', 'browser_uncheck',
  'browser_scroll', 'browser_scroll_into_view', 'browser_focus', 'browser_clear',
  'browser_navigate', 'browser_requests', 'browser_request_detail', 'browser_headers',
  'browser_offline', 'browser_cookies_get', 'browser_cookies_set',
  'browser_cookies_clear', 'browser_storage_get', 'browser_storage_set',
  'browser_storage_clear', 'browser_user_agent', 'browser_viewport',
  'browser_geolocation', 'browser_permissions', 'browser_timezone', 'browser_locale',
  'browser_media', 'browser_dialog', 'browser_upload', 'browser_download',
  'browser_wait_for_download', 'browser_har_start', 'browser_har_stop', 'browser_route',
  'browser_unroute', 'browser_console', 'browser_errors', 'browser_pdf',
  'browser_response_body', 'browser_clipboard', 'browser_back', 'browser_forward',
  'browser_reload', 'browser_tab_new', 'browser_tab_switch', 'browser_tab_close',
  'browser_set_content', 'browser_command', 'service_trace',
];

const MCP_RESOURCE_ALLOWLIST = [
  'agent-browser://contracts', 'agent-browser://access-plan',
  'agent-browser://browser-capability-registry', 'agent-browser://incidents',
  'agent-browser://profiles', 'agent-browser://sessions', 'agent-browser://browsers',
  'agent-browser://display-allocations', 'agent-browser://remote-view-routes',
  'agent-browser://route-pool', 'agent-browser://viewer-leases', 'agent-browser://tabs',
  'agent-browser://monitors', 'agent-browser://site-policies', 'agent-browser://providers',
  'agent-browser://challenges', 'agent-browser://jobs', 'agent-browser://events',
];

const MCP_TEMPLATE_ALLOWLIST = [
  'agent-browser://access-plan{?serviceName,agentName,taskName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,accountId,accountIds,url,sitePolicyId,challengeId,readinessProfileId,runtimeProfile,browserBuild,browserHost,viewStreamProvider,controlInputProvider,displayIsolation}',
  'agent-browser://incidents/{incident_id}/activity',
  'agent-browser://profiles/lookup{?query,hostname,profileId,profileName,serviceName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,accountId,accountIds,authenticationState,freshnessState,tag,url,readinessProfileId,browserBuild}',
  'agent-browser://profiles/{profile_id}/readiness',
  'agent-browser://profiles/{profile_id}/allocation',
  'agent-browser://profiles/{profile_id}/seeding-handoff{?targetServiceId,siteId,loginId}',
];

function mcpToolResultClassification(name) {
  if (name === 'desktop_capture') return 'bounded_ephemeral_desktop_observation';
  if (name === 'browser_command') return 'explicit_full_status_rejection';
  if (name.startsWith('browser_')) return 'narrow_browser_result';
  if (['service_access_plan', 'service_incidents', 'service_trace',
    'service_browser_capability_preflight', 'service_remote_view_route_preflight'].includes(name)) {
    return 'narrow_service_read_result';
  }
  if (name.startsWith('service_')) return 'narrow_service_mutation_or_job_result';
  return null;
}

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
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
            name: 'MCP read Google profile',
            userDataDir: join(context.tempHome, 'google-profile-user-data'),
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [],
            sharedServiceIds: ['McpReadSmoke'],
            targetReadiness: [
              {
                targetServiceId,
                loginId: targetServiceId,
                state: 'needs_manual_seeding',
                manualSeedingRequired: true,
                evidence: 'manual_seed_required_without_authenticated_hint',
                recommendedAction:
                  'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
                seedingMode: 'detached_headed_no_cdp',
                cdpAttachmentAllowedDuringSeeding: false,
                preferredKeyring: 'basic_password_store',
                setupScopes: ['signin', 'chrome_sync', 'passkeys', 'browser_plugins'],
              },
            ],
            persistent: true,
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

function assertNoLaunchSideEffects(statePath) {
  if (!existsSync(statePath)) return;
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  assert(
    Object.keys(state.jobs ?? {}).length === 0,
    `mcp read persisted jobs: ${JSON.stringify(state.jobs)}`,
  );
  assert(
    Object.keys(state.browsers ?? {}).length === 0,
    `mcp read persisted browsers: ${JSON.stringify(state.browsers)}`,
  );
}

function assertMcpDoesNotProduceFullStatus(value, label) {
  const serialized = JSON.stringify(value);
  assert(!serialized.includes('statusProjection'), `${label} exposed statusProjection`);
  assert(
    !(serialized.includes('control_plane') && serialized.includes('service_state')),
    `${label} exposed a full status envelope`,
  );
}

async function assertStatusToolRejected(name, argumentsValue) {
  let result;
  try {
    result = await mcp.send('tools/call', { name, arguments: argumentsValue });
  } catch (error) {
    assertMcpDoesNotProduceFullStatus({ message: error.message }, `${name} rejection`);
    return;
  }
  assert(
    result?.isError === true,
    `${name} unexpectedly accepted full status production: ${JSON.stringify(result)}`,
  );
  assertMcpDoesNotProduceFullStatus(result, `${name} rejection payload`);
}

try {
  seedServiceState();

  const sessionsResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'mcp',
    'read',
    'agent-browser://sessions',
  ]);
  const sessions = readResourceContents(
    parseJsonOutput(sessionsResult.stdout, 'mcp sessions resource'),
    'sessions',
  );

  assert(
    Array.isArray(sessions.sessions),
    `invalid sessions resource: ${sessionsResult.stdout}`,
  );
  assert(sessions.count === 0, `mcp read returned unexpected sessions: ${sessionsResult.stdout}`);

  const statePath = join(agentHome, 'service', 'state.json');
  assertNoLaunchSideEffects(statePath);

  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: (message, stderr) => {
      console.error(message);
      if (stderr.trim()) {
        console.error(stderr.trim());
      }
    },
  });
  try {
    const initialize = await mcp.send('initialize', {
      protocolVersion: '2025-06-18',
      capabilities: {},
      clientInfo: { name: 'agent-browser-mcp-read-no-launch-smoke', version: '0' },
    });
    assert(initialize.capabilities?.resources, 'MCP resources capability missing');
    mcp.notify('notifications/initialized');

    const inventory = {
      resources: await mcp.send('resources/list'),
      templates: await mcp.send('resources/templates/list'),
      tools: await mcp.send('tools/list'),
    };
    assert(
      isDeepStrictEqual(inventory.tools.tools?.map((tool) => tool.name), MCP_TOOL_ALLOWLIST),
      'MCP tool inventory drifted from the frozen allowlist',
    );
    assert(
      isDeepStrictEqual(
        inventory.resources.resources?.map((resource) => resource.uri),
        MCP_RESOURCE_ALLOWLIST,
      ),
      'MCP resource inventory drifted from the frozen allowlist',
    );
    assert(
      isDeepStrictEqual(
        inventory.templates.resourceTemplates?.map((template) => template.uriTemplate),
        MCP_TEMPLATE_ALLOWLIST,
      ),
      'MCP resource template inventory drifted from the frozen allowlist',
    );
    assertMcpDoesNotProduceFullStatus(inventory, 'MCP inventory');
    assert(
      !JSON.stringify(inventory).includes('service_status'),
      'MCP inventory advertised service_status',
    );
    for (const tool of inventory.tools.tools ?? []) {
      const classification = mcpToolResultClassification(tool.name);
      assert(classification, `MCP tool ${tool.name} has no frozen narrower-result classification`);
      assertMcpDoesNotProduceFullStatus(tool, `MCP tool schema ${tool.name}`);
      assert(
        !JSON.stringify(tool).includes('statusProjection'),
        `MCP tool ${tool.name} schema advertised full status`,
      );
    }
    for (const resource of inventory.resources.resources ?? []) {
      const result = await mcp.send('resources/read', { uri: resource.uri });
      assertMcpDoesNotProduceFullStatus(result, `MCP resource ${resource.uri}`);
    }
    for (const template of inventory.templates.resourceTemplates ?? []) {
      const uri = template.uriTemplate.replace(/\{[^}]+\}/g, 'missing-status-inventory-id');
      try {
        const result = await mcp.send('resources/read', { uri });
        assertMcpDoesNotProduceFullStatus(result, `MCP resource template ${uri}`);
      } catch (error) {
        assertMcpDoesNotProduceFullStatus(
          { message: error.message },
          `MCP resource template rejection ${uri}`,
        );
      }
    }
    await assertStatusToolRejected('browser_command', {
      action: 'service_status',
      params: {},
    });
    await assertStatusToolRejected('service_request', {
      action: 'service_status',
      serviceName: 'McpReadSmoke',
      agentName: 'codex',
      taskName: 'verifyMcpStatusNonproducer',
      params: {},
    });

    const readinessUri = `agent-browser://profiles/${profileId}/readiness`;
    const readiness = parseMcpJsonResource(
      await mcp.send('resources/read', { uri: readinessUri }),
      readinessUri,
      'MCP profile readiness resource',
    );

    assert(readiness.profileId === profileId, `readiness profile mismatch: ${JSON.stringify(readiness)}`);
    assert(readiness.count === 1, `readiness count mismatch: ${JSON.stringify(readiness)}`);
    assert(
      readiness.targetReadiness?.[0]?.targetServiceId === targetServiceId,
      `readiness target mismatch: ${JSON.stringify(readiness)}`,
    );
    assert(
      readiness.targetReadiness?.[0]?.state === 'needs_manual_seeding',
      `readiness state mismatch: ${JSON.stringify(readiness)}`,
    );

    const allocationUri = `agent-browser://profiles/${profileId}/allocation`;
    const allocation = parseMcpJsonResource(
      await mcp.send('resources/read', { uri: allocationUri }),
      allocationUri,
      'MCP profile allocation resource',
    );

    assert(allocation.profileId === profileId, `allocation profile mismatch: ${JSON.stringify(allocation)}`);
    assert(
      allocation.profileAllocation?.profileId === profileId,
      `allocation row profile mismatch: ${JSON.stringify(allocation)}`,
    );
    assert(
      allocation.profileAllocation?.targetReadiness?.[0]?.state === 'needs_manual_seeding',
      `allocation readiness mismatch: ${JSON.stringify(allocation)}`,
    );

    const lookupUri = `agent-browser://profiles/lookup?serviceName=McpReadSmoke&loginId=${targetServiceId}`;
    const lookup = parseMcpJsonResource(
      await mcp.send('resources/read', { uri: lookupUri }),
      lookupUri,
      'MCP profile lookup resource',
    );

    assert(
      lookup.selectedProfile?.id === profileId,
      `lookup selected profile mismatch: ${JSON.stringify(lookup)}`,
    );
    assert(
      lookup.selectedProfileMatch?.reason === 'target_match',
      `lookup match reason mismatch: ${JSON.stringify(lookup)}`,
    );
    assert(
      lookup.readiness?.profileId === profileId,
      `lookup readiness profile mismatch: ${JSON.stringify(lookup)}`,
    );

    const uri = `agent-browser://profiles/${profileId}/seeding-handoff?targetServiceId=${targetServiceId}`;
    const handoff = parseMcpJsonResource(
      await mcp.send('resources/read', { uri }),
      uri,
      'MCP profile seeding handoff resource',
    );

    assert(handoff.profileId === profileId, `handoff profile mismatch: ${JSON.stringify(handoff)}`);
    assert(
      handoff.targetServiceId === targetServiceId,
      `handoff target mismatch: ${JSON.stringify(handoff)}`,
    );
    assert(
      handoff.command === `agent-browser --runtime-profile ${profileId} runtime login https://accounts.google.com`,
      `handoff command mismatch: ${JSON.stringify(handoff)}`,
    );
    assert(
      handoff.lifecycle?.state === 'needs_manual_seeding',
      `handoff lifecycle mismatch: ${JSON.stringify(handoff)}`,
    );
    assert(
      handoff.operatorIntervention?.defaultChannels?.includes('mcp'),
      `handoff intervention missing MCP channel: ${JSON.stringify(handoff)}`,
    );
    assert(
      handoff.operatorIntervention?.blocksProfileLease === true,
      `handoff intervention should block profile lease: ${JSON.stringify(handoff)}`,
    );
  } finally {
    mcp.close();
    mcp = null;
  }

  assertNoLaunchSideEffects(statePath);

  await cleanup();
  console.log('MCP read no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
