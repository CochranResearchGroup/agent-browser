import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');

const browserSurface = [
  ['browser_get_url', 'GET', '/api/browser/url'],
  ['browser_get_title', 'GET', '/api/browser/title'],
  ['browser_tabs', 'GET', '/api/browser/tabs'],
  ['browser_navigate', 'POST', '/api/browser/navigate'],
  ['browser_back', 'POST', '/api/browser/back'],
  ['browser_forward', 'POST', '/api/browser/forward'],
  ['browser_reload', 'POST', '/api/browser/reload'],
  ['browser_tab_new', 'POST', '/api/browser/new-tab'],
  ['browser_tab_switch', 'POST', '/api/browser/switch-tab'],
  ['browser_tab_close', 'POST', '/api/browser/close-tab'],
  ['browser_viewport', 'POST', '/api/browser/viewport'],
  ['browser_user_agent', 'POST', '/api/browser/user-agent'],
  ['browser_media', 'POST', '/api/browser/media'],
  ['browser_timezone', 'POST', '/api/browser/timezone'],
  ['browser_locale', 'POST', '/api/browser/locale'],
  ['browser_geolocation', 'POST', '/api/browser/geolocation'],
  ['browser_permissions', 'POST', '/api/browser/permissions'],
  ['browser_cookies_get', 'POST', '/api/browser/cookies/get'],
  ['browser_cookies_set', 'POST', '/api/browser/cookies/set'],
  ['browser_cookies_clear', 'POST', '/api/browser/cookies/clear'],
  ['browser_storage_get', 'POST', '/api/browser/storage/get'],
  ['browser_storage_set', 'POST', '/api/browser/storage/set'],
  ['browser_storage_clear', 'POST', '/api/browser/storage/clear'],
  ['browser_console', 'POST', '/api/browser/console'],
  ['browser_errors', 'POST', '/api/browser/errors'],
  ['browser_set_content', 'POST', '/api/browser/set-content'],
  ['browser_headers', 'POST', '/api/browser/headers'],
  ['browser_offline', 'POST', '/api/browser/offline'],
  ['browser_dialog', 'POST', '/api/browser/dialog'],
  ['browser_clipboard', 'POST', '/api/browser/clipboard'],
  ['browser_upload', 'POST', '/api/browser/upload'],
  ['browser_download', 'POST', '/api/browser/download'],
  ['browser_wait_for_download', 'POST', '/api/browser/wait-for-download'],
  ['browser_pdf', 'POST', '/api/browser/pdf'],
  ['browser_response_body', 'POST', '/api/browser/response-body'],
  ['browser_har_start', 'POST', '/api/browser/har/start'],
  ['browser_har_stop', 'POST', '/api/browser/har/stop'],
  ['browser_route', 'POST', '/api/browser/route'],
  ['browser_unroute', 'POST', '/api/browser/unroute'],
  ['browser_requests', 'POST', '/api/browser/requests'],
  ['browser_request_detail', 'POST', '/api/browser/request-detail'],
  ['browser_snapshot', 'POST', '/api/browser/snapshot'],
  ['browser_screenshot', 'POST', '/api/browser/screenshot'],
  ['browser_click', 'POST', '/api/browser/click'],
  ['browser_fill', 'POST', '/api/browser/fill'],
  ['browser_wait', 'POST', '/api/browser/wait'],
  ['browser_type', 'POST', '/api/browser/type'],
  ['browser_press', 'POST', '/api/browser/press'],
  ['browser_hover', 'POST', '/api/browser/hover'],
  ['browser_select', 'POST', '/api/browser/select'],
  ['browser_get_text', 'POST', '/api/browser/get-text'],
  ['browser_get_value', 'POST', '/api/browser/get-value'],
  ['browser_is_visible', 'POST', '/api/browser/is-visible'],
  ['browser_get_attribute', 'POST', '/api/browser/get-attribute'],
  ['browser_get_html', 'POST', '/api/browser/get-html'],
  ['browser_get_styles', 'POST', '/api/browser/get-styles'],
  ['browser_count', 'POST', '/api/browser/count'],
  ['browser_get_box', 'POST', '/api/browser/get-box'],
  ['browser_is_enabled', 'POST', '/api/browser/is-enabled'],
  ['browser_is_checked', 'POST', '/api/browser/is-checked'],
  ['browser_check', 'POST', '/api/browser/check'],
  ['browser_uncheck', 'POST', '/api/browser/uncheck'],
  ['browser_scroll', 'POST', '/api/browser/scroll'],
  ['browser_scroll_into_view', 'POST', '/api/browser/scroll-into-view'],
  ['browser_focus', 'POST', '/api/browser/focus'],
  ['browser_clear', 'POST', '/api/browser/clear'],
].map(([tool, method, route]) => ({ tool, method, route }));

const files = {
  actions: read('cli/src/native/actions.rs'),
  serviceContracts: read('cli/src/native/service_contracts.rs'),
  serviceRequestSchema: read('docs/dev/contracts/service-request.v1.schema.json'),
  mcp: `${read('cli/src/mcp.rs')}\n${read('cli/src/native/service_contracts.rs')}`,
  http: `${read('cli/src/native/stream/http.rs')}\n${read('cli/src/native/service_contracts.rs')}`,
  readme: read('README.md'),
  skill: read('skills/agent-browser/SKILL.md'),
  docs: read('docs/src/app/commands/page.mdx'),
  serviceModeDocs: read('docs/src/app/service-mode/page.mdx'),
};

const failures = [];
const nativeServiceActions = extractNativeServiceActions(files.actions);
const noLaunchServiceActions = extractNoLaunchServiceActions(files.actions);
const rustServiceRequestActions = extractRustStringArray(
  files.serviceContracts,
  'pub const SERVICE_REQUEST_ACTIONS',
);
const schemaServiceRequestActions = extractServiceRequestSchemaActions(files.serviceRequestSchema);
const serviceSurface = [
  {
    tool: 'service_request',
    method: 'POST',
    route: '/api/service/request',
    httpNeedles: ['path == SERVICE_REQUEST_HTTP_ROUTE'],
  },
  {
    tool: 'service_job_cancel',
    method: 'POST',
    route: '/api/service/jobs/<id>/cancel',
    docsNeedles: ['/api/service/jobs/<job-id>/cancel', '/api/service/jobs/<id>/cancel'],
    httpNeedles: ['service_job_cancel_id(path)', '"/api/service/jobs/"', '"/cancel"'],
  },
  {
    tool: 'service_browser_retry',
    method: 'POST',
    route: '/api/service/browsers/<id>/retry',
    docsNeedles: ['/api/service/browsers/<browser-id>/retry', '/api/service/browsers/<id>/retry'],
    httpNeedles: ['service_browser_retry_id(path)', '"/api/service/browsers/"', '"/retry"'],
  },
  {
    tool: 'service_incidents',
    method: 'GET',
    route: '/api/service/incidents',
    httpNeedles: ['path == "/api/service/incidents"', 'service_incidents_command(query)'],
  },
  {
    tool: 'service_trace',
    method: 'GET',
    route: '/api/service/trace',
    httpNeedles: ['path == "/api/service/trace"', 'service_trace_command(query)'],
  },
  {
    tool: 'service_remedies_apply',
    method: 'POST',
    route: '/api/service/remedies/apply',
    docsNeedles: ['/api/service/remedies/apply'],
    httpNeedles: ['path == "/api/service/remedies/apply"', 'service_remedies_apply_command(query)'],
    clientNeedles: ['applyServiceRemedies', '/api/service/remedies/apply'],
  },
  {
    tool: 'service_profile_upsert',
    method: 'POST',
    route: '/api/service/profiles/<id>',
    httpNeedles: ['service_profile_id(path)', 'service_profile_upsert_command(profile_id, body_str)'],
  },
  {
    tool: 'service_profile_freshness_update',
    method: 'POST',
    route: '/api/service/profiles/<id>/freshness',
    httpNeedles: [
      'service_profile_freshness_id(path)',
      'service_profile_freshness_command(profile_id, body_str)',
    ],
    clientNeedles: [
      'updateServiceProfileFreshness',
      '/api/service/profiles/${encodeURIComponent(id)}/freshness',
    ],
  },
  {
    tool: 'service_profile_seeding_handoff_update',
    method: 'POST',
    route: '/api/service/profiles/<id>/seeding-handoff',
    docsNeedles: [
      '/api/service/profiles/<profile-id>/seeding-handoff',
      '/api/service/profiles/<id>/seeding-handoff',
      '/api/service/profiles/&lt;id&gt;/seeding-handoff',
    ],
    httpNeedles: [
      'service_profile_seeding_handoff_id(path)',
      'service_profile_seeding_handoff_update_command(profile_id, body_str)',
    ],
    clientNeedles: [
      'updateServiceProfileSeedingHandoff',
      '/api/service/profiles/${encodeURIComponent(id)}/seeding-handoff',
    ],
    contractNeedles: [
      'SERVICE_PROFILE_SEEDING_HANDOFF_UPDATE_MCP_TOOL_NAME',
      'service-profile-seeding-handoff-response.v1.schema.json',
    ],
  },
  {
    tool: 'service_profile_delete',
    method: 'DELETE',
    route: '/api/service/profiles/<id>',
    httpNeedles: ['service_profile_id(path)', 'service_profile_delete_command(profile_id)'],
  },
  {
    tool: 'service_session_upsert',
    method: 'POST',
    route: '/api/service/sessions/<id>',
    httpNeedles: [
      'service_session_id(path)',
      'service_session_upsert_command(service_session_id, body_str)',
    ],
  },
  {
    tool: 'service_session_delete',
    method: 'DELETE',
    route: '/api/service/sessions/<id>',
    httpNeedles: ['service_session_id(path)', 'service_session_delete_command(service_session_id)'],
  },
  {
    tool: 'service_site_policy_upsert',
    method: 'POST',
    route: '/api/service/site-policies/<id>',
    httpNeedles: [
      'service_site_policy_id(path)',
      'service_site_policy_upsert_command(site_policy_id, body_str)',
    ],
  },
  {
    tool: 'service_site_policy_delete',
    method: 'DELETE',
    route: '/api/service/site-policies/<id>',
    httpNeedles: [
      'service_site_policy_id(path)',
      'service_site_policy_delete_command(site_policy_id)',
    ],
  },
  {
    tool: 'service_monitor_upsert',
    method: 'POST',
    route: '/api/service/monitors/<id>',
    httpNeedles: ['service_monitor_id(path)', 'service_monitor_upsert_command(monitor_id, body_str)'],
    clientNeedles: ['upsertServiceMonitor', '/api/service/monitors/${encodeURIComponent(id)}'],
  },
  {
    tool: 'service_monitor_delete',
    method: 'DELETE',
    route: '/api/service/monitors/<id>',
    httpNeedles: ['service_monitor_id(path)', 'service_monitor_delete_command(monitor_id)'],
    clientNeedles: ['deleteServiceMonitor', '/api/service/monitors/${encodeURIComponent(id)}'],
  },
  {
    tool: 'service_monitors_run_due',
    method: 'POST',
    route: '/api/service/monitors/run-due',
    httpNeedles: ['"/api/service/monitors/run-due"', 'service_monitors_run_due_command()'],
    clientNeedles: ['runDueServiceMonitors', '/api/service/monitors/run-due'],
  },
  {
    tool: 'service_monitor_pause',
    method: 'POST',
    route: '/api/service/monitors/<id>/pause',
    docsNeedles: ['/api/service/monitors/<id>/pause', '/api/service/monitors/&lt;id&gt;/pause'],
    httpNeedles: ['service_monitor_action_id(path, "/pause")', 'service_monitor_state_command(monitor_id, "service_monitor_pause")'],
    clientNeedles: ['pauseServiceMonitor', '/api/service/monitors/${encodeURIComponent(id)}/pause'],
  },
  {
    tool: 'service_monitor_resume',
    method: 'POST',
    route: '/api/service/monitors/<id>/resume',
    docsNeedles: ['/api/service/monitors/<id>/resume', '/api/service/monitors/&lt;id&gt;/resume'],
    httpNeedles: ['service_monitor_action_id(path, "/resume")', 'service_monitor_state_command(monitor_id, "service_monitor_resume")'],
    clientNeedles: ['resumeServiceMonitor', '/api/service/monitors/${encodeURIComponent(id)}/resume'],
  },
  {
    tool: 'service_monitor_reset_failures',
    method: 'POST',
    route: '/api/service/monitors/<id>/reset-failures',
    docsNeedles: ['/api/service/monitors/<id>/reset-failures', '/api/service/monitors/&lt;id&gt;/reset-failures'],
    httpNeedles: ['service_monitor_action_id(path, "/reset-failures")', 'service_monitor_state_command(monitor_id, "service_monitor_reset_failures")'],
    clientNeedles: ['resetServiceMonitorFailures', '/api/service/monitors/${encodeURIComponent(id)}/reset-failures'],
  },
  {
    tool: 'service_monitor_triage',
    method: 'POST',
    route: '/api/service/monitors/<id>/triage',
    docsNeedles: ['/api/service/monitors/<id>/triage', '/api/service/monitors/&lt;id&gt;/triage'],
    httpNeedles: ['service_monitor_action_id(path, "/triage")', 'service_monitor_triage_command(monitor_id, query)'],
    clientNeedles: ['triageServiceMonitor', '/api/service/monitors/${encodeURIComponent(id)}/triage'],
  },
  {
    tool: 'service_provider_upsert',
    method: 'POST',
    route: '/api/service/providers/<id>',
    httpNeedles: ['service_provider_id(path)', 'service_provider_upsert_command(provider_id, body_str)'],
  },
  {
    tool: 'service_provider_delete',
    method: 'DELETE',
    route: '/api/service/providers/<id>',
    httpNeedles: ['service_provider_id(path)', 'service_provider_delete_command(provider_id)'],
  },
];

for (const action of nativeServiceActions) {
  expectIncludes(
    noLaunchServiceActions,
    action,
    `native service action ${action} must skip browser launch`,
  );
}

for (const action of noLaunchServiceActions) {
  expectIncludes(
    nativeServiceActions,
    action,
    `no-launch service action ${action} must be handled by execute_command`,
  );
}

expectIncludes(
  files.actions,
  'action.starts_with("service_")',
  'profile lease gate must exempt service control actions by prefix',
);

expectSameItems(
  rustServiceRequestActions,
  schemaServiceRequestActions,
  'Rust SERVICE_REQUEST_ACTIONS',
  'service-request schema action enum',
);

const serviceResourceSurface = [
  { resource: 'agent-browser://contracts', route: '/api/service/contracts' },
  {
    resource: 'agent-browser://access-plan',
    route: '/api/service/access-plan',
    docsNeedles: [
      '/api/service/access-plan',
      'agent-browser://access-plan{?serviceName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,sitePolicyId,challengeId,readinessProfileId}',
    ],
    httpNeedles: [
      'path == "/api/service/access-plan"',
      'service_access_plan_response(query)',
      'SERVICE_ACCESS_PLAN_HTTP_ROUTE',
      'SERVICE_ACCESS_PLAN_RESPONSE_SCHEMA_ID',
      '"serviceAccessPlanResponse"',
    ],
    clientNeedles: [
      'getServiceAccessPlan',
      '/api/service/access-plan',
    ],
    contractNeedles: [
      'service-access-plan-response.v1.schema.json',
      'GET /api/service/access-plan',
      'agent-browser://access-plan',
    ],
  },
  { resource: 'agent-browser://profiles', route: '/api/service/profiles' },
  { resource: 'agent-browser://sessions', route: '/api/service/sessions' },
  { resource: 'agent-browser://browsers', route: '/api/service/browsers' },
  { resource: 'agent-browser://tabs', route: '/api/service/tabs' },
  {
    resource: 'agent-browser://monitors',
    route: '/api/service/monitors',
    clientNeedles: ['getServiceMonitors', '/api/service/monitors'],
    contractNeedles: [
      'service-monitor-record.v1.schema.json',
      'service-monitors-response.v1.schema.json',
      'GET /api/service/monitors',
      'agent-browser://monitors',
    ],
  },
  { resource: 'agent-browser://site-policies', route: '/api/service/site-policies' },
  { resource: 'agent-browser://providers', route: '/api/service/providers' },
  { resource: 'agent-browser://challenges', route: '/api/service/challenges' },
  { resource: 'agent-browser://jobs', route: '/api/service/jobs' },
  { resource: 'agent-browser://events', route: '/api/service/events' },
  { resource: 'agent-browser://incidents', route: '/api/service/incidents' },
  {
    resource: 'agent-browser://incidents/{incident_id}/activity',
    route: '/api/service/incidents/<id>/activity',
    docsNeedles: [
      '/api/service/incidents/<incident-id>/activity',
      '/api/service/incidents/<id>/activity',
    ],
    httpNeedles: ['service_incident_action_id(path, "/activity")'],
  },
  {
    resource: 'agent-browser://profiles/{profile_id}/seeding-handoff{?targetServiceId,siteId,loginId}',
    route: '/api/service/profiles/<id>/seeding-handoff',
    docsNeedles: [
      'agent-browser://profiles/{profile_id}/seeding-handoff{?targetServiceId,siteId,loginId}',
      '/api/service/profiles/<profile-id>/seeding-handoff',
      '/api/service/profiles/<id>/seeding-handoff',
      '/api/service/profiles/&lt;id&gt;/seeding-handoff',
    ],
    httpNeedles: [
      'service_profile_seeding_handoff_id(path)',
      '"operatorSteps"',
      'SERVICE_PROFILE_SEEDING_HANDOFF_HTTP_ROUTE',
      'SERVICE_PROFILE_SEEDING_HANDOFF_RESPONSE_SCHEMA_ID',
      '"serviceProfileSeedingHandoffResponse"',
    ],
    clientNeedles: [
      'getServiceProfileSeedingHandoff',
      '/api/service/profiles/${encodeURIComponent(id)}/seeding-handoff',
    ],
    contractNeedles: [
      'service-profile-seeding-handoff-response.v1.schema.json',
      'GET /api/service/profiles/<id>/seeding-handoff',
      'agent-browser://profiles/{profile_id}/seeding-handoff{?targetServiceId,siteId,loginId}',
    ],
  },
  {
    resource: 'agent-browser://profiles/{profile_id}/readiness',
    route: '/api/service/profiles/<id>/readiness',
    docsNeedles: [
      'agent-browser://profiles/{profile_id}/readiness',
      '/api/service/profiles/<profile-id>/readiness',
      '/api/service/profiles/<id>/readiness',
      '/api/service/profiles/&lt;id&gt;/readiness',
    ],
    httpNeedles: [
      'service_profile_readiness_id(path)',
      '"targetReadiness"',
      'SERVICE_PROFILE_READINESS_HTTP_ROUTE',
      'SERVICE_PROFILE_READINESS_RESPONSE_SCHEMA_ID',
      '"serviceProfileReadinessResponse"',
    ],
    clientNeedles: [
      'getServiceProfileReadiness',
      '/api/service/profiles/${encodeURIComponent(id)}/readiness',
    ],
    contractNeedles: [
      'service-profile-readiness-response.v1.schema.json',
      'GET /api/service/profiles/<id>/readiness',
      'agent-browser://profiles/{profile_id}/readiness',
    ],
  },
  {
    resource: 'agent-browser://profiles/{profile_id}/allocation',
    route: '/api/service/profiles/<id>/allocation',
    docsNeedles: [
      'agent-browser://profiles/{profile_id}/allocation',
      '/api/service/profiles/<profile-id>/allocation',
      '/api/service/profiles/<id>/allocation',
      '/api/service/profiles/&lt;id&gt;/allocation',
      '/api/service/profiles/journal-downloader/allocation',
    ],
    httpNeedles: [
      'service_profile_allocation_id(path)',
      '"profileAllocation"',
      'SERVICE_PROFILE_ALLOCATION_HTTP_ROUTE',
      'SERVICE_PROFILE_ALLOCATION_RESPONSE_SCHEMA_ID',
      '"serviceProfileAllocationResponse"',
    ],
    clientNeedles: [
      'getServiceProfileAllocation',
      '/api/service/profiles/${encodeURIComponent(id)}/allocation',
    ],
    contractNeedles: [
      'service-profile-allocation-response.v1.schema.json',
      'GET /api/service/profiles/<id>/allocation',
      'agent-browser://profiles/{profile_id}/allocation',
    ],
  },
  {
    resource: 'agent-browser://profiles/lookup{?serviceName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,readinessProfileId}',
    route: '/api/service/profiles/lookup',
    docsNeedles: [
      'agent-browser://profiles/lookup{?serviceName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,readinessProfileId}',
      '/api/service/profiles/lookup',
      'getServiceProfileForIdentity',
    ],
    httpNeedles: [
      'path == "/api/service/profiles/lookup"',
      'service_profile_lookup_response(query)',
      'SERVICE_PROFILE_LOOKUP_HTTP_ROUTE',
      'SERVICE_PROFILE_LOOKUP_RESPONSE_SCHEMA_ID',
      '"serviceProfileLookupResponse"',
    ],
    clientNeedles: [
      'getServiceProfileForIdentity',
      '/api/service/profiles/lookup',
    ],
    contractNeedles: [
      'service-profile-lookup-response.v1.schema.json',
      'GET /api/service/profiles/lookup',
      'agent-browser://profiles/lookup{?serviceName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,readinessProfileId}',
    ],
  },
];

const serviceHttpOnlySurface = [];

for (const entry of browserSurface) {
  expectIncludes(files.mcp, entry.tool, `MCP source exposes ${entry.tool}`);
  expectIncludes(
    files.http,
    `("${entry.method}", "${entry.route}")`,
    `HTTP source exposes ${entry.method} ${entry.route}`,
  );

  for (const [label, source] of [
    ['README.md', files.readme],
    ['skills/agent-browser/SKILL.md', files.skill],
    ['docs/src/app/commands/page.mdx', files.docs],
  ]) {
    expectIncludes(source, entry.tool, `${label} mentions ${entry.tool}`);
    expectIncludes(source, entry.route, `${label} mentions ${entry.route}`);
  }
}

for (const entry of serviceSurface) {
  expectIncludes(files.mcp, entry.tool, `MCP source exposes ${entry.tool}`);
  for (const needle of entry.httpNeedles) {
    expectIncludes(files.http, needle, `HTTP source exposes ${entry.method} ${entry.route}`);
  }

  for (const [label, source] of [
    ['README.md', files.readme],
    ['skills/agent-browser/SKILL.md', files.skill],
    ['docs/src/app/commands/page.mdx', files.docs],
    ['docs/src/app/service-mode/page.mdx', files.serviceModeDocs],
  ]) {
    expectIncludes(source, entry.tool, `${label} mentions ${entry.tool}`);
    expectAnyIncludes(source, entry.docsNeedles ?? [entry.route], `${label} mentions ${entry.route}`);
  }
  for (const needle of entry.clientNeedles ?? []) {
    expectIncludes(read('packages/client/src/service-observability.js'), needle, `service client exposes ${entry.route}`);
  }
}

for (const entry of serviceResourceSurface) {
  expectIncludes(files.mcp, entry.resource, `MCP source exposes resource ${entry.resource}`);
  for (const needle of entry.httpNeedles ?? [entry.route.replace('/<id>', '')]) {
    expectIncludes(files.http, needle, `HTTP source exposes ${entry.route}`);
  }

  for (const [label, source] of [
    ['README.md', files.readme],
    ['skills/agent-browser/SKILL.md', files.skill],
    ['docs/src/app/commands/page.mdx', files.docs],
    ['docs/src/app/service-mode/page.mdx', files.serviceModeDocs],
  ]) {
    expectIncludes(source, entry.resource, `${label} mentions ${entry.resource}`);
    expectAnyIncludes(source, entry.docsNeedles ?? [entry.route], `${label} mentions ${entry.route}`);
  }
  for (const needle of entry.clientNeedles ?? []) {
    expectIncludes(read('packages/client/src/service-observability.js'), needle, `service client exposes ${entry.route}`);
  }
  for (const needle of entry.contractNeedles ?? []) {
    expectIncludes(read('docs/dev/contracts/README.md'), needle, `contract docs mention ${entry.route}`);
  }
}

for (const entry of serviceHttpOnlySurface) {
  for (const needle of entry.httpNeedles) {
    expectIncludes(files.http, needle, `HTTP source exposes ${entry.method} ${entry.route}`);
  }
  for (const needle of entry.clientNeedles) {
    expectIncludes(read('packages/client/src/service-observability.js'), needle, `service client exposes ${entry.route}`);
  }
  for (const needle of entry.contractNeedles) {
    expectIncludes(read('docs/dev/contracts/README.md'), needle, `contract docs mention ${entry.route}`);
  }

  for (const [label, source] of [
    ['README.md', files.readme],
    ['skills/agent-browser/SKILL.md', files.skill],
    ['docs/src/app/commands/page.mdx', files.docs],
    ['docs/src/app/service-mode/page.mdx', files.serviceModeDocs],
  ]) {
    expectAnyIncludes(source, entry.docsNeedles ?? [entry.route], `${label} mentions ${entry.route}`);
  }
}

if (failures.length > 0) {
  console.error('Service API/MCP parity check failed:');
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(
  `Service API/MCP parity check passed for ${browserSurface.length} browser controls, ${serviceSurface.length} service tools, ${serviceResourceSurface.length} service resources, ${serviceHttpOnlySurface.length} HTTP-only service routes, ${nativeServiceActions.length} native service actions, and ${rustServiceRequestActions.length} service-request actions`,
);

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

function expectIncludes(source, needle, message) {
  if (!source.includes(needle)) {
    failures.push(message);
  }
}

function expectAnyIncludes(source, needles, message) {
  if (!needles.some((needle) => source.includes(needle))) {
    failures.push(message);
  }
}

function extractNativeServiceActions(source) {
  const body = extractRustFunctionBody(source, 'pub async fn execute_command');
  return sortedUnique(
    [...body.matchAll(/"(?<action>service_[a-z0-9_]+)"\s*=>/g)].map(
      (match) => match.groups.action,
    ),
  );
}

function extractNoLaunchServiceActions(source) {
  const body = extractRustFunctionBody(source, 'pub(crate) fn action_skips_browser_launch');
  return sortedUnique(
    [...body.matchAll(/"(?<action>service_[a-z0-9_]+)"/g)].map(
      (match) => match.groups.action,
    ),
  );
}

function extractRustStringArray(source, signature) {
  const start = source.indexOf(signature);
  if (start < 0) {
    failures.push(`Rust source missing ${signature}`);
    return [];
  }
  const open = source.indexOf('[', start);
  if (open < 0) {
    failures.push(`Rust source missing array for ${signature}`);
    return [];
  }
  const close = source.indexOf('];', open);
  if (close < 0) {
    failures.push(`Rust source has unterminated array for ${signature}`);
    return [];
  }
  return sortedUnique(
    [...source.slice(open, close).matchAll(/"(?<value>[a-z0-9_]+)"/g)].map(
      (match) => match.groups.value,
    ),
  );
}

function extractServiceRequestSchemaActions(source) {
  const schema = JSON.parse(source);
  const actions = schema?.properties?.action?.enum;
  if (!Array.isArray(actions)) {
    failures.push('service-request schema missing properties.action.enum');
    return [];
  }
  return sortedUnique(actions);
}

function extractRustFunctionBody(source, signature) {
  const start = source.indexOf(signature);
  if (start < 0) {
    failures.push(`Rust source missing ${signature}`);
    return '';
  }
  const open = source.indexOf('{', start);
  if (open < 0) {
    failures.push(`Rust source missing body for ${signature}`);
    return '';
  }

  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(open + 1, index);
      }
    }
  }

  failures.push(`Rust source has unterminated body for ${signature}`);
  return '';
}

function sortedUnique(values) {
  return [...new Set(values)].sort();
}

function expectSameItems(left, right, leftLabel, rightLabel) {
  const missingFromRight = left.filter((item) => !right.includes(item));
  const missingFromLeft = right.filter((item) => !left.includes(item));
  for (const item of missingFromRight) {
    failures.push(`${rightLabel} missing ${item} from ${leftLabel}`);
  }
  for (const item of missingFromLeft) {
    failures.push(`${leftLabel} missing ${item} from ${rightLabel}`);
  }
}
