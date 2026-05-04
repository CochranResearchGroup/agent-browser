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
  mcp: `${read('cli/src/mcp.rs')}\n${read('cli/src/native/service_contracts.rs')}`,
  http: `${read('cli/src/native/stream/http.rs')}\n${read('cli/src/native/service_contracts.rs')}`,
  readme: read('README.md'),
  skill: read('skills/agent-browser/SKILL.md'),
  docs: read('docs/src/app/commands/page.mdx'),
  serviceModeDocs: read('docs/src/app/service-mode/page.mdx'),
};

const failures = [];
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
    tool: 'service_profile_upsert',
    method: 'POST',
    route: '/api/service/profiles/<id>',
    httpNeedles: ['service_profile_id(path)', 'service_profile_upsert_command(profile_id, body_str)'],
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

const serviceResourceSurface = [
  { resource: 'agent-browser://contracts', route: '/api/service/contracts' },
  { resource: 'agent-browser://profiles', route: '/api/service/profiles' },
  { resource: 'agent-browser://sessions', route: '/api/service/sessions' },
  { resource: 'agent-browser://browsers', route: '/api/service/browsers' },
  { resource: 'agent-browser://tabs', route: '/api/service/tabs' },
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
];

const serviceHttpOnlySurface = [
  {
    method: 'GET',
    route: '/api/service/profiles/<id>/allocation',
    docsNeedles: [
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
    ],
  },
  {
    method: 'GET',
    route: '/api/service/profiles/<id>/readiness',
    docsNeedles: [
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
    ],
  },
  {
    method: 'GET',
    route: '/api/service/profiles/lookup',
    docsNeedles: [
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
    ],
  },
];

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
  `Service API/MCP parity check passed for ${browserSurface.length} browser controls, ${serviceSurface.length} service tools, ${serviceResourceSurface.length} service resources, and ${serviceHttpOnlySurface.length} HTTP-only service routes`,
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
