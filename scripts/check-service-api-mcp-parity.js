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
  mcp: read('cli/src/mcp.rs'),
  http: read('cli/src/native/stream/http.rs'),
  readme: read('README.md'),
  skill: read('skills/agent-browser/SKILL.md'),
  docs: read('docs/src/app/commands/page.mdx'),
};

const failures = [];
const serviceSurface = [['service_request', 'POST', '/api/service/request']].map(
  ([tool, method, route]) => ({ tool, method, route }),
);

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
  expectIncludes(
    files.http,
    `path == "${entry.route}"`,
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

if (failures.length > 0) {
  console.error('Service API/MCP parity check failed:');
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(
  `Service API/MCP parity check passed for ${browserSurface.length} browser controls and ${serviceSurface.length} service controls`,
);

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

function expectIncludes(source, needle, message) {
  if (!source.includes(needle)) {
    failures.push(message);
  }
}
