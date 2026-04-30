#!/usr/bin/env node

import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'agent-browser-mcp-live-',
  session: `mcp-live-${process.pid}-${Date.now()}`,
  socketDir: ({ agentHome }) => join(agentHome, 'sockets'),
});
const { session, tempHome } = context;
const profileDir = join(tempHome, 'chrome-profile');
const screenshotDir = join(tempHome, 'screenshots');
const uploadPath = join(tempHome, 'mcp-upload.txt');
const harPath = join(tempHome, 'mcp-capture.har');
const pdfPath = join(tempHome, 'mcp-page.pdf');
const serviceName = 'McpLiveSmoke';
const agentName = 'smoke-agent';
const taskName = 'browserSnapshotSmoke';

mkdirSync(profileDir, { recursive: true });
mkdirSync(screenshotDir, { recursive: true });
writeFileSync(uploadPath, 'mcp upload smoke\n');

let mcp;
let networkServer;
const timeout = setTimeout(() => {
  fail('Timed out waiting for live MCP smoke to complete');
}, 90000);

function startMcpServer() {
  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: fail,
  });
}

function send(method, params) {
  return mcp.send(method, params);
}

function notify(method, params) {
  mcp.notify(method, params);
}

function parseToolPayload(result) {
  const text = result.content?.[0]?.text;
  assert(typeof text === 'string', 'MCP tool response missing text content');
  return JSON.parse(text);
}

async function cleanup() {
  clearTimeout(timeout);
  if (mcp) mcp.close();
  if (networkServer) {
    networkServer.closeAllConnections?.();
    await new Promise((resolve) => networkServer.close(resolve));
    networkServer = undefined;
  }
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

async function fail(message) {
  if (mcp) mcp.rejectPending(message);
  await cleanup();
  console.error(message);
  const stderr = mcp?.stderr() ?? '';
  if (stderr.trim()) console.error(stderr.trim());
  process.exit(1);
}

async function startNetworkServer() {
  const server = createServer((req, res) => {
    if (req.url?.startsWith('/asset.png')) {
      const png = Buffer.from(
        'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/l1EG6QAAAABJRU5ErkJggg==',
        'base64',
      );
      res.writeHead(200, {
        'content-type': 'image/png',
        'content-length': png.length,
      });
      res.end(png);
      return;
    }

    const body = [
      '<!doctype html>',
      '<html>',
      '<head><title>MCP Browser Command Smoke</title></head>',
      '<body>',
      '<main id="browser-command-main">',
      '<h1>MCP Browser Command Smoke</h1>',
      '<img id="network-probe" src="/asset.png?probe=1" alt="network probe">',
      '<label for="file">Upload</label>',
      '<input id="file" type="file">',
      '</main>',
      '</body>',
      '</html>',
    ].join('');
    res.writeHead(200, {
      'content-type': 'text/html; charset=utf-8',
      'content-length': Buffer.byteLength(body),
    });
    res.end(body);
  });

  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      server.off('error', reject);
      resolve();
    });
  });

  const { port } = server.address();
  return { server, url: `http://127.0.0.1:${port}/browser-command?probe=1` };
}

try {
  const networkFixture = await startNetworkServer();
  networkServer = networkFixture.server;
  const pageHtml = [
    '<!doctype html>',
    '<html>',
    '<head><title>MCP Live Smoke</title></head>',
    '<body>',
    '<main id="main">',
    '<h1>MCP Live Smoke</h1>',
    '<button id="ready" onclick="document.getElementById(\'status\').textContent = \'Clicked\'">Ready</button>',
    '<button id="hidden-control" style="display: none">Hidden</button>',
    '<p id="status">Not clicked</p>',
    '<label for="name">Name</label>',
    '<input id="name" name="name" value="stale" onfocus="document.getElementById(\'focus-output\').textContent = \'Focused name\'" oninput="if (this.value === \'\') { document.getElementById(\'clear-output\').textContent = \'Name cleared\'; }" onkeydown="if (event.key === \'Enter\') { document.getElementById(\'press-output\').textContent = \'Pressed Enter\'; }">',
    '<input id="disabled-name" value="locked" disabled>',
    '<p id="focus-output"></p>',
    '<p id="clear-output"></p>',
    '<p id="press-output"></p>',
    '<button id="hover-target" onmouseover="document.getElementById(\'hover-output\').textContent = \'Hovered menu\'">Hover menu</button>',
    '<p id="hover-output"></p>',
    '<label for="org">Organization</label>',
    '<select id="org" onchange="document.getElementById(\'org-output\').textContent = this.value"><option value="">Choose</option><option value="org-a">Org A</option><option value="org-b">Org B</option></select>',
    '<p id="org-output"></p>',
    '<label for="remember"><input id="remember" type="checkbox" onchange="document.getElementById(\'remember-output\').textContent = this.checked ? \'Remember checked\' : \'Remember unchecked\'">Remember me</label>',
    '<p id="remember-output"></p>',
    '<ul><li class="item">One</li><li class="item">Two</li><li class="item">Three</li></ul>',
    '<div id="box" style="width: 200px; height: 100px; padding: 0; margin: 0;">Box target</div>',
    '<div style="height: 1600px" aria-label="scroll spacer"></div>',
    '<button id="scroll-target" onclick="document.getElementById(\'scroll-into-output\').textContent = \'Scrolled target clicked\'">Scroll target</button>',
    '<p id="scroll-into-output"></p>',
    '<button id="copy-name" onclick="document.getElementById(\'name-output\').textContent = document.getElementById(\'name\').value; setTimeout(() => { document.getElementById(\'wait-status\').textContent = \'Name copied\'; }, 100)">Copy name</button>',
    '<p id="name-output"></p>',
    '<p id="wait-status"></p>',
    '<a href="https://example.com/">Example</a>',
    '</main>',
    '</body>',
    '</html>',
  ].join('');
  const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(pageHtml)}`;
  const browserCommandUrl = networkFixture.url;

  const openResult = await runCli(context, [
    '--json',
    '--session',
    session,
    '--profile',
    profileDir,
    '--args',
    '--no-sandbox',
    'open',
    pageUrl,
  ]);
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  startMcpServer();
  const initialize = await send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-mcp-live-smoke', version: '0' },
  });
  assert(initialize.capabilities?.tools, 'MCP tools capability missing');
  notify('notifications/initialized');

  const commandTrackRequestsResult = await send('tools/call', {
    name: 'browser_requests',
    arguments: {
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandTrackRequestsPayload = parseToolPayload(commandTrackRequestsResult);
  assert(
    commandTrackRequestsPayload.success === true,
    `browser_requests tracking failed: ${JSON.stringify(commandTrackRequestsPayload)}`,
  );
  assert(
    Array.isArray(commandTrackRequestsPayload.data?.requests),
    'browser_requests did not return a request array',
  );

  const commandHeadersResult = await send('tools/call', {
    name: 'browser_headers',
    arguments: {
      headers: {
        'X-Agent-Browser-Smoke': 'typed-headers',
      },
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandHeadersPayload = parseToolPayload(commandHeadersResult);
  assert(
    commandHeadersPayload.success === true,
    `browser_headers failed: ${JSON.stringify(commandHeadersPayload)}`,
  );
  assert(commandHeadersPayload.data?.set === true, 'browser_headers did not report set state');
  assert(
    commandHeadersPayload.trace?.serviceName === serviceName,
    'browser_headers trace missing serviceName',
  );
  assert(commandHeadersPayload.trace?.agentName === agentName, 'browser_headers trace missing agentName');
  assert(commandHeadersPayload.trace?.taskName === taskName, 'browser_headers trace missing taskName');

  const commandOfflineResult = await send('tools/call', {
    name: 'browser_offline',
    arguments: {
      offline: true,
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandOfflinePayload = parseToolPayload(commandOfflineResult);
  assert(
    commandOfflinePayload.success === true,
    `browser_offline enable failed: ${JSON.stringify(commandOfflinePayload)}`,
  );
  assert(commandOfflinePayload.data?.offline === true, 'browser_offline did not enable offline mode');
  assert(
    commandOfflinePayload.trace?.serviceName === serviceName,
    'browser_offline enable trace missing serviceName',
  );

  const commandOnlineResult = await send('tools/call', {
    name: 'browser_offline',
    arguments: {
      offline: false,
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandOnlinePayload = parseToolPayload(commandOnlineResult);
  assert(
    commandOnlinePayload.success === true,
    `browser_offline disable failed: ${JSON.stringify(commandOnlinePayload)}`,
  );
  assert(commandOnlinePayload.data?.offline === false, 'browser_offline did not restore online mode');
  assert(
    commandOnlinePayload.trace?.serviceName === serviceName,
    'browser_offline disable trace missing serviceName',
  );

  const commandNavigateResult = await send('tools/call', {
    name: 'browser_navigate',
    arguments: {
      url: browserCommandUrl,
      waitUntil: 'load',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandNavigatePayload = parseToolPayload(commandNavigateResult);
  assert(
    commandNavigatePayload.success === true,
    `browser_navigate failed: ${JSON.stringify(commandNavigatePayload)}`,
  );
  assert(
    commandNavigatePayload.data?.url === browserCommandUrl,
    'browser_navigate did not report the requested URL',
  );
  assert(
    commandNavigatePayload.trace?.serviceName === serviceName,
    'browser_navigate trace missing serviceName',
  );
  assert(
    commandNavigatePayload.trace?.agentName === agentName,
    'browser_navigate trace missing agentName',
  );
  assert(
    commandNavigatePayload.trace?.taskName === taskName,
    'browser_navigate trace missing taskName',
  );

  const commandReloadResult = await send('tools/call', {
    name: 'browser_reload',
    arguments: {
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandReloadPayload = parseToolPayload(commandReloadResult);
  assert(
    commandReloadPayload.success === true,
    `browser_reload failed: ${JSON.stringify(commandReloadPayload)}`,
  );
  assert(
    commandReloadPayload.data?.url === browserCommandUrl,
    'browser_reload did not report the active URL',
  );
  assert(
    commandReloadPayload.trace?.serviceName === serviceName,
    'browser_reload trace missing serviceName',
  );

  const commandBackResult = await send('tools/call', {
    name: 'browser_back',
    arguments: {
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandBackPayload = parseToolPayload(commandBackResult);
  assert(commandBackPayload.success === true, `browser_back failed: ${JSON.stringify(commandBackPayload)}`);
  assert(
    commandBackPayload.data?.url === pageUrl,
    'browser_back did not return to the original fixture page',
  );
  assert(commandBackPayload.trace?.serviceName === serviceName, 'browser_back trace missing serviceName');

  const commandForwardResult = await send('tools/call', {
    name: 'browser_forward',
    arguments: {
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandForwardPayload = parseToolPayload(commandForwardResult);
  assert(
    commandForwardPayload.success === true,
    `browser_forward failed: ${JSON.stringify(commandForwardPayload)}`,
  );
  assert(
    commandForwardPayload.data?.url === browserCommandUrl,
    'browser_forward did not return to the network fixture page',
  );
  assert(
    commandForwardPayload.trace?.serviceName === serviceName,
    'browser_forward trace missing serviceName',
  );

  const commandUserAgentResult = await send('tools/call', {
    name: 'browser_user_agent',
    arguments: {
      userAgent: 'AgentBrowserMcpLiveSmoke/1.0',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandUserAgentPayload = parseToolPayload(commandUserAgentResult);
  assert(
    commandUserAgentPayload.success === true,
    `browser_user_agent failed: ${JSON.stringify(commandUserAgentPayload)}`,
  );
  assert(
    commandUserAgentPayload.data?.userAgent === 'AgentBrowserMcpLiveSmoke/1.0',
    'browser_user_agent did not report the requested user agent',
  );

  const commandViewportResult = await send('tools/call', {
    name: 'browser_viewport',
    arguments: {
      width: 900,
      height: 650,
      deviceScaleFactor: 1,
      mobile: false,
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandViewportPayload = parseToolPayload(commandViewportResult);
  assert(
    commandViewportPayload.success === true,
    `browser_viewport failed: ${JSON.stringify(commandViewportPayload)}`,
  );
  assert(commandViewportPayload.data?.width === 900, 'browser_viewport did not report width');
  assert(commandViewportPayload.data?.height === 650, 'browser_viewport did not report height');

  const commandGeolocationResult = await send('tools/call', {
    name: 'browser_geolocation',
    arguments: {
      latitude: 41.8781,
      longitude: -87.6298,
      accuracy: 10,
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandGeolocationPayload = parseToolPayload(commandGeolocationResult);
  assert(
    commandGeolocationPayload.success === true,
    `browser_geolocation failed: ${JSON.stringify(commandGeolocationPayload)}`,
  );
  assert(
    commandGeolocationPayload.data?.latitude === 41.8781,
    'browser_geolocation did not report latitude',
  );
  assert(
    commandGeolocationPayload.data?.longitude === -87.6298,
    'browser_geolocation did not report longitude',
  );

  const commandPermissionsResult = await send('tools/call', {
    name: 'browser_permissions',
    arguments: {
      permissions: ['geolocation'],
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandPermissionsPayload = parseToolPayload(commandPermissionsResult);
  assert(
    commandPermissionsPayload.success === true,
    `browser_permissions failed: ${JSON.stringify(commandPermissionsPayload)}`,
  );
  assert(
    commandPermissionsPayload.data?.granted?.[0] === 'geolocation',
    'browser_permissions did not report the granted permission',
  );

  const commandTimezoneResult = await send('tools/call', {
    name: 'browser_timezone',
    arguments: {
      timezoneId: 'America/Chicago',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandTimezonePayload = parseToolPayload(commandTimezoneResult);
  assert(
    commandTimezonePayload.success === true,
    `browser_timezone failed: ${JSON.stringify(commandTimezonePayload)}`,
  );
  assert(
    commandTimezonePayload.data?.timezoneId === 'America/Chicago',
    'browser_timezone did not report the timezone id',
  );

  const commandLocaleResult = await send('tools/call', {
    name: 'browser_locale',
    arguments: {
      locale: 'en-US',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandLocalePayload = parseToolPayload(commandLocaleResult);
  assert(
    commandLocalePayload.success === true,
    `browser_locale failed: ${JSON.stringify(commandLocalePayload)}`,
  );
  assert(commandLocalePayload.data?.locale === 'en-US', 'browser_locale did not report locale');

  const commandMediaResult = await send('tools/call', {
    name: 'browser_media',
    arguments: {
      media: 'screen',
      colorScheme: 'light',
      reducedMotion: 'no-preference',
      features: {
        'prefers-contrast': 'no-preference',
      },
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandMediaPayload = parseToolPayload(commandMediaResult);
  assert(
    commandMediaPayload.success === true,
    `browser_media failed: ${JSON.stringify(commandMediaPayload)}`,
  );
  assert(commandMediaPayload.data?.set === true, 'browser_media did not report set state');

  const commandDialogResult = await send('tools/call', {
    name: 'browser_dialog',
    arguments: {
      response: 'status',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandDialogPayload = parseToolPayload(commandDialogResult);
  assert(
    commandDialogPayload.success === true,
    `browser_dialog status failed: ${JSON.stringify(commandDialogPayload)}`,
  );
  assert(commandDialogPayload.data?.hasDialog === false, 'browser_dialog reported an unexpected dialog');

  const commandUploadResult = await send('tools/call', {
    name: 'browser_upload',
    arguments: {
      selector: '#file',
      files: [uploadPath],
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandUploadPayload = parseToolPayload(commandUploadResult);
  assert(
    commandUploadPayload.success === true,
    `browser_upload failed: ${JSON.stringify(commandUploadPayload)}`,
  );
  assert(commandUploadPayload.data?.uploaded === 1, 'browser_upload did not report one uploaded file');

  const commandHarStartResult = await send('tools/call', {
    name: 'browser_har_start',
    arguments: {
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandHarStartPayload = parseToolPayload(commandHarStartResult);
  assert(
    commandHarStartPayload.success === true,
    `browser_har_start failed: ${JSON.stringify(commandHarStartPayload)}`,
  );
  assert(commandHarStartPayload.data?.started === true, 'browser_har_start did not report started state');

  const commandHarNavigateResult = await send('tools/call', {
    name: 'browser_navigate',
    arguments: {
      url: `${browserCommandUrl}&har=1`,
      waitUntil: 'load',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandHarNavigatePayload = parseToolPayload(commandHarNavigateResult);
  assert(
    commandHarNavigatePayload.success === true,
    `browser_navigate during HAR failed: ${JSON.stringify(commandHarNavigatePayload)}`,
  );

  const commandHarStopResult = await send('tools/call', {
    name: 'browser_har_stop',
    arguments: {
      path: harPath,
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandHarStopPayload = parseToolPayload(commandHarStopResult);
  assert(
    commandHarStopPayload.success === true,
    `browser_har_stop failed: ${JSON.stringify(commandHarStopPayload)}`,
  );
  assert(commandHarStopPayload.data?.path === harPath, 'browser_har_stop did not report HAR path');
  assert(existsSync(harPath), 'browser_har_stop did not write HAR file');

  const commandRouteResult = await send('tools/call', {
    name: 'browser_route',
    arguments: {
      url: '**/mocked-api',
      response: {
        status: 200,
        body: '{"mocked":true}',
        contentType: 'application/json',
      },
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandRoutePayload = parseToolPayload(commandRouteResult);
  assert(
    commandRoutePayload.success === true,
    `browser_route failed: ${JSON.stringify(commandRoutePayload)}`,
  );
  assert(commandRoutePayload.data?.routed === '**/mocked-api', 'browser_route did not report route');

  const commandUnrouteResult = await send('tools/call', {
    name: 'browser_unroute',
    arguments: {
      url: '**/mocked-api',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandUnroutePayload = parseToolPayload(commandUnrouteResult);
  assert(
    commandUnroutePayload.success === true,
    `browser_unroute failed: ${JSON.stringify(commandUnroutePayload)}`,
  );
  assert(commandUnroutePayload.data?.unrouted === '**/mocked-api', 'browser_unroute did not report route');

  const commandConsoleResult = await send('tools/call', {
    name: 'browser_console',
    arguments: {
      clear: true,
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandConsolePayload = parseToolPayload(commandConsoleResult);
  assert(
    commandConsolePayload.success === true,
    `browser_console failed: ${JSON.stringify(commandConsolePayload)}`,
  );
  assert(commandConsolePayload.data?.cleared === true, 'browser_console did not report clear state');

  const commandErrorsResult = await send('tools/call', {
    name: 'browser_errors',
    arguments: {
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandErrorsPayload = parseToolPayload(commandErrorsResult);
  assert(
    commandErrorsPayload.success === true,
    `browser_errors failed: ${JSON.stringify(commandErrorsPayload)}`,
  );
  assert(commandErrorsPayload.data && typeof commandErrorsPayload.data === 'object', 'browser_errors returned no data');

  const commandPdfResult = await send('tools/call', {
    name: 'browser_pdf',
    arguments: {
      path: pdfPath,
      printBackground: true,
      landscape: false,
      preferCSSPageSize: false,
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandPdfPayload = parseToolPayload(commandPdfResult);
  assert(
    commandPdfPayload.success === true,
    `browser_pdf failed: ${JSON.stringify(commandPdfPayload)}`,
  );
  assert(commandPdfPayload.data?.path === pdfPath, 'browser_pdf did not report PDF path');
  assert(existsSync(pdfPath), 'browser_pdf did not write PDF file');

  const commandCookiesSetResult = await send('tools/call', {
    name: 'browser_cookies_set',
    arguments: {
      name: 'mcp_typed_cookie',
      value: 'cookie-value',
      url: browserCommandUrl,
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandCookiesSetPayload = parseToolPayload(commandCookiesSetResult);
  assert(
    commandCookiesSetPayload.success === true,
    `browser_cookies_set failed: ${JSON.stringify(commandCookiesSetPayload)}`,
  );
  assert(commandCookiesSetPayload.data?.set === true, 'browser_cookies_set did not report set state');
  assert(
    commandCookiesSetPayload.trace?.serviceName === serviceName,
    'browser_cookies_set trace missing serviceName',
  );

  const commandCookiesGetResult = await send('tools/call', {
    name: 'browser_cookies_get',
    arguments: {
      urls: [browserCommandUrl],
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandCookiesGetPayload = parseToolPayload(commandCookiesGetResult);
  assert(
    commandCookiesGetPayload.success === true,
    `browser_cookies_get failed: ${JSON.stringify(commandCookiesGetPayload)}`,
  );
  assert(
    commandCookiesGetPayload.data?.cookies?.some(
      (cookie) => cookie.name === 'mcp_typed_cookie' && cookie.value === 'cookie-value',
    ),
    `browser_cookies_get did not return the typed cookie: ${JSON.stringify(commandCookiesGetPayload)}`,
  );

  const commandStorageSetResult = await send('tools/call', {
    name: 'browser_storage_set',
    arguments: {
      type: 'local',
      key: 'mcpTypedStorage',
      value: 'storage-value',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandStorageSetPayload = parseToolPayload(commandStorageSetResult);
  assert(
    commandStorageSetPayload.success === true,
    `browser_storage_set failed: ${JSON.stringify(commandStorageSetPayload)}`,
  );
  assert(commandStorageSetPayload.data?.set === true, 'browser_storage_set did not report set state');

  const commandStorageGetResult = await send('tools/call', {
    name: 'browser_storage_get',
    arguments: {
      type: 'local',
      key: 'mcpTypedStorage',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandStorageGetPayload = parseToolPayload(commandStorageGetResult);
  assert(
    commandStorageGetPayload.success === true,
    `browser_storage_get failed: ${JSON.stringify(commandStorageGetPayload)}`,
  );
  assert(
    commandStorageGetPayload.data?.value === 'storage-value',
    `browser_storage_get did not return the typed storage value: ${JSON.stringify(commandStorageGetPayload)}`,
  );

  const commandStorageClearResult = await send('tools/call', {
    name: 'browser_storage_clear',
    arguments: {
      type: 'local',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandStorageClearPayload = parseToolPayload(commandStorageClearResult);
  assert(
    commandStorageClearPayload.success === true,
    `browser_storage_clear failed: ${JSON.stringify(commandStorageClearPayload)}`,
  );
  assert(
    commandStorageClearPayload.data?.cleared === true,
    'browser_storage_clear did not report cleared state',
  );

  const commandCookiesClearResult = await send('tools/call', {
    name: 'browser_cookies_clear',
    arguments: {
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandCookiesClearPayload = parseToolPayload(commandCookiesClearResult);
  assert(
    commandCookiesClearPayload.success === true,
    `browser_cookies_clear failed: ${JSON.stringify(commandCookiesClearPayload)}`,
  );
  assert(
    commandCookiesClearPayload.data?.cleared === true,
    'browser_cookies_clear did not report cleared state',
  );

  const commandRequestsResult = await send('tools/call', {
    name: 'browser_requests',
    arguments: {
      filter: '/asset.png',
      method: 'GET',
      status: '2xx',
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandRequestsPayload = parseToolPayload(commandRequestsResult);
  assert(
    commandRequestsPayload.success === true,
    `browser_requests failed: ${JSON.stringify(commandRequestsPayload)}`,
  );
  const commandRequests = commandRequestsPayload.data?.requests;
  assert(Array.isArray(commandRequests), 'browser_requests did not return requests array');
  assert(
    commandRequests.some(
      (request) =>
        typeof request.url === 'string' &&
        request.url.includes('/asset.png?probe=1') &&
        request.method === 'GET' &&
        request.status === 200,
    ),
    `browser_requests did not capture the local asset request: ${JSON.stringify(commandRequests)}`,
  );
  const assetRequest = commandRequests.findLast(
    (request) =>
      typeof request.url === 'string' &&
      request.url.includes('/asset.png?probe=1') &&
      request.method === 'GET' &&
      request.status === 200,
  );
  assert(
    typeof assetRequest?.requestId === 'string' && assetRequest.requestId.length > 0,
    `browser_requests did not include a requestId for the local asset request: ${JSON.stringify(assetRequest)}`,
  );
  assert(
    commandRequestsPayload.trace?.serviceName === serviceName,
    'browser_requests trace missing serviceName',
  );
  assert(
    commandRequestsPayload.trace?.agentName === agentName,
    'browser_requests trace missing agentName',
  );
  assert(
    commandRequestsPayload.trace?.taskName === taskName,
    'browser_requests trace missing taskName',
  );

  const commandRequestDetailResult = await send('tools/call', {
    name: 'browser_request_detail',
    arguments: {
      requestId: assetRequest.requestId,
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandRequestDetailPayload = parseToolPayload(commandRequestDetailResult);
  assert(
    commandRequestDetailPayload.success === true,
    `browser_request_detail failed: ${JSON.stringify(commandRequestDetailPayload)}`,
  );
  assert(
    commandRequestDetailPayload.data?.requestId === assetRequest.requestId,
    'browser_request_detail did not return the requested requestId',
  );
  assert(
    typeof commandRequestDetailPayload.data?.url === 'string' &&
      commandRequestDetailPayload.data.url.includes('/asset.png?probe=1'),
    'browser_request_detail did not return the local asset URL',
  );
  assert(
    commandRequestDetailPayload.data?.status === 200,
    'browser_request_detail did not return the local asset status',
  );
  assert(
    commandRequestDetailPayload.data?.headers?.['X-Agent-Browser-Smoke'] === 'typed-headers',
    'browser_request_detail did not show the header set by browser_headers',
  );
  assert(
    typeof commandRequestDetailPayload.data?.responseBody === 'string',
    'browser_request_detail did not include a responseBody field',
  );
  assert(
    commandRequestDetailPayload.trace?.serviceName === serviceName,
    'browser_request_detail trace missing serviceName',
  );
  assert(
    commandRequestDetailPayload.trace?.agentName === agentName,
    'browser_request_detail trace missing agentName',
  );
  assert(
    commandRequestDetailPayload.trace?.taskName === taskName,
    'browser_request_detail trace missing taskName',
  );

  const commandReturnResult = await send('tools/call', {
    name: 'browser_command',
    arguments: {
      action: 'navigate',
      params: {
        url: pageUrl,
        waitUntil: 'load',
      },
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandReturnPayload = parseToolPayload(commandReturnResult);
  assert(
    commandReturnPayload.success === true,
    `browser_command return navigate failed: ${JSON.stringify(commandReturnPayload)}`,
  );
  assert(
    commandReturnPayload.data?.url === pageUrl,
    'browser_command return navigate did not restore the fixture page',
  );

  const toolResult = await send('tools/call', {
    name: 'browser_snapshot',
    arguments: {
      selector: '#main',
      interactive: true,
      urls: true,
      serviceName,
      agentName,
      taskName,
    },
  });
  const payload = parseToolPayload(toolResult);
  assert(payload.success === true, `browser_snapshot failed: ${JSON.stringify(payload)}`);
  assert(
    typeof payload.data?.snapshot === 'string' && payload.data.snapshot.includes('Ready'),
    'browser_snapshot payload did not include expected page content',
  );
  assert(payload.trace?.serviceName === serviceName, 'browser_snapshot trace missing serviceName');
  assert(payload.trace?.agentName === agentName, 'browser_snapshot trace missing agentName');
  assert(payload.trace?.taskName === taskName, 'browser_snapshot trace missing taskName');

  const urlResult = await send('tools/call', {
    name: 'browser_get_url',
    arguments: {
      serviceName,
      agentName,
      taskName,
    },
  });
  const urlPayload = parseToolPayload(urlResult);
  assert(urlPayload.success === true, `browser_get_url failed: ${JSON.stringify(urlPayload)}`);
  assert(urlPayload.data?.url === pageUrl, 'browser_get_url did not return the active page URL');
  assert(urlPayload.trace?.serviceName === serviceName, 'browser_get_url trace missing serviceName');
  assert(urlPayload.trace?.agentName === agentName, 'browser_get_url trace missing agentName');
  assert(urlPayload.trace?.taskName === taskName, 'browser_get_url trace missing taskName');

  const titleResult = await send('tools/call', {
    name: 'browser_get_title',
    arguments: {
      serviceName,
      agentName,
      taskName,
    },
  });
  const titlePayload = parseToolPayload(titleResult);
  assert(
    titlePayload.success === true,
    `browser_get_title failed: ${JSON.stringify(titlePayload)}`,
  );
  assert(
    titlePayload.data?.title === 'MCP Live Smoke',
    'browser_get_title did not return the active page title',
  );
  assert(
    titlePayload.trace?.serviceName === serviceName,
    'browser_get_title trace missing serviceName',
  );
  assert(titlePayload.trace?.agentName === agentName, 'browser_get_title trace missing agentName');
  assert(titlePayload.trace?.taskName === taskName, 'browser_get_title trace missing taskName');

  const tabsResult = await send('tools/call', {
    name: 'browser_tabs',
    arguments: {
      verbose: true,
      serviceName,
      agentName,
      taskName,
    },
  });
  const tabsPayload = parseToolPayload(tabsResult);
  assert(tabsPayload.success === true, `browser_tabs failed: ${JSON.stringify(tabsPayload)}`);
  const tabs = tabsPayload.data?.tabs;
  assert(Array.isArray(tabs), 'browser_tabs payload did not include tabs array');
  assert(
    tabs.some(
      (tab) =>
        tab.url === pageUrl &&
        tab.active === true &&
        tab.type === 'page' &&
        typeof tab.title === 'string' &&
        typeof tab.targetId === 'string' &&
        typeof tab.sessionId === 'string',
    ),
    `browser_tabs did not return the active live smoke tab with verbose metadata: ${JSON.stringify(tabs)}`,
  );
  assert(tabsPayload.trace?.serviceName === serviceName, 'browser_tabs trace missing serviceName');
  assert(tabsPayload.trace?.agentName === agentName, 'browser_tabs trace missing agentName');
  assert(tabsPayload.trace?.taskName === taskName, 'browser_tabs trace missing taskName');

  const tabNewResult = await send('tools/call', {
    name: 'browser_tab_new',
    arguments: {
      url: browserCommandUrl,
      serviceName,
      agentName,
      taskName,
    },
  });
  const tabNewPayload = parseToolPayload(tabNewResult);
  assert(tabNewPayload.success === true, `browser_tab_new failed: ${JSON.stringify(tabNewPayload)}`);
  assert(
    tabNewPayload.data?.url === browserCommandUrl,
    'browser_tab_new did not report the requested URL',
  );
  assert(tabNewPayload.trace?.serviceName === serviceName, 'browser_tab_new trace missing serviceName');

  const tabSwitchResult = await send('tools/call', {
    name: 'browser_tab_switch',
    arguments: {
      index: 0,
      serviceName,
      agentName,
      taskName,
    },
  });
  const tabSwitchPayload = parseToolPayload(tabSwitchResult);
  assert(
    tabSwitchPayload.success === true,
    `browser_tab_switch failed: ${JSON.stringify(tabSwitchPayload)}`,
  );
  assert(tabSwitchPayload.trace?.serviceName === serviceName, 'browser_tab_switch trace missing serviceName');

  const tabCloseResult = await send('tools/call', {
    name: 'browser_tab_close',
    arguments: {
      index: 1,
      serviceName,
      agentName,
      taskName,
    },
  });
  const tabClosePayload = parseToolPayload(tabCloseResult);
  assert(tabClosePayload.success === true, `browser_tab_close failed: ${JSON.stringify(tabClosePayload)}`);
  assert(tabClosePayload.trace?.serviceName === serviceName, 'browser_tab_close trace missing serviceName');

  const screenshotResult = await send('tools/call', {
    name: 'browser_screenshot',
    arguments: {
      selector: '#main',
      screenshotDir,
      serviceName,
      agentName,
      taskName,
    },
  });
  const screenshotPayload = parseToolPayload(screenshotResult);
  assert(
    screenshotPayload.success === true,
    `browser_screenshot failed: ${JSON.stringify(screenshotPayload)}`,
  );
  assert(
    typeof screenshotPayload.data?.path === 'string' &&
      screenshotPayload.data.path.startsWith(screenshotDir) &&
      existsSync(screenshotPayload.data.path),
    `browser_screenshot did not save an image in the requested directory: ${JSON.stringify(screenshotPayload)}`,
  );
  assert(
    screenshotPayload.trace?.serviceName === serviceName,
    'browser_screenshot trace missing serviceName',
  );
  assert(
    screenshotPayload.trace?.agentName === agentName,
    'browser_screenshot trace missing agentName',
  );
  assert(
    screenshotPayload.trace?.taskName === taskName,
    'browser_screenshot trace missing taskName',
  );

  const clickResult = await send('tools/call', {
    name: 'browser_click',
    arguments: {
      selector: '#ready',
      serviceName,
      agentName,
      taskName,
    },
  });
  const clickPayload = parseToolPayload(clickResult);
  assert(clickPayload.success === true, `browser_click failed: ${JSON.stringify(clickPayload)}`);
  assert(clickPayload.data?.clicked === '#ready', 'browser_click did not report clicked selector');
  assert(clickPayload.trace?.serviceName === serviceName, 'browser_click trace missing serviceName');
  assert(clickPayload.trace?.agentName === agentName, 'browser_click trace missing agentName');
  assert(clickPayload.trace?.taskName === taskName, 'browser_click trace missing taskName');

  const visibleResult = await send('tools/call', {
    name: 'browser_is_visible',
    arguments: {
      selector: '#ready',
      serviceName,
      agentName,
      taskName,
    },
  });
  const visiblePayload = parseToolPayload(visibleResult);
  assert(
    visiblePayload.success === true,
    `browser_is_visible failed: ${JSON.stringify(visiblePayload)}`,
  );
  assert(visiblePayload.data?.visible === true, 'browser_is_visible did not report visible state');
  assert(
    visiblePayload.trace?.serviceName === serviceName,
    'browser_is_visible trace missing serviceName',
  );
  assert(visiblePayload.trace?.agentName === agentName, 'browser_is_visible trace missing agentName');
  assert(visiblePayload.trace?.taskName === taskName, 'browser_is_visible trace missing taskName');

  const hiddenResult = await send('tools/call', {
    name: 'browser_is_visible',
    arguments: {
      selector: '#hidden-control',
      serviceName,
      agentName,
      taskName,
    },
  });
  const hiddenPayload = parseToolPayload(hiddenResult);
  assert(hiddenPayload.success === true, `hidden browser_is_visible failed: ${JSON.stringify(hiddenPayload)}`);
  assert(hiddenPayload.data?.visible === false, 'browser_is_visible did not report hidden state');

  const enabledResult = await send('tools/call', {
    name: 'browser_is_enabled',
    arguments: {
      selector: '#name',
      serviceName,
      agentName,
      taskName,
    },
  });
  const enabledPayload = parseToolPayload(enabledResult);
  assert(
    enabledPayload.success === true,
    `browser_is_enabled failed: ${JSON.stringify(enabledPayload)}`,
  );
  assert(enabledPayload.data?.enabled === true, 'browser_is_enabled did not report enabled state');
  assert(
    enabledPayload.trace?.serviceName === serviceName,
    'browser_is_enabled trace missing serviceName',
  );
  assert(enabledPayload.trace?.agentName === agentName, 'browser_is_enabled trace missing agentName');
  assert(enabledPayload.trace?.taskName === taskName, 'browser_is_enabled trace missing taskName');

  const disabledResult = await send('tools/call', {
    name: 'browser_is_enabled',
    arguments: {
      selector: '#disabled-name',
      serviceName,
      agentName,
      taskName,
    },
  });
  const disabledPayload = parseToolPayload(disabledResult);
  assert(disabledPayload.success === true, `disabled browser_is_enabled failed: ${JSON.stringify(disabledPayload)}`);
  assert(disabledPayload.data?.enabled === false, 'browser_is_enabled did not report disabled state');

  const focusResult = await send('tools/call', {
    name: 'browser_focus',
    arguments: {
      selector: '#name',
      serviceName,
      agentName,
      taskName,
    },
  });
  const focusPayload = parseToolPayload(focusResult);
  assert(focusPayload.success === true, `browser_focus failed: ${JSON.stringify(focusPayload)}`);
  assert(focusPayload.data?.focused === '#name', 'browser_focus did not report focused selector');
  assert(focusPayload.trace?.serviceName === serviceName, 'browser_focus trace missing serviceName');
  assert(focusPayload.trace?.agentName === agentName, 'browser_focus trace missing agentName');
  assert(focusPayload.trace?.taskName === taskName, 'browser_focus trace missing taskName');

  const clearResult = await send('tools/call', {
    name: 'browser_clear',
    arguments: {
      selector: '#name',
      serviceName,
      agentName,
      taskName,
    },
  });
  const clearPayload = parseToolPayload(clearResult);
  assert(clearPayload.success === true, `browser_clear failed: ${JSON.stringify(clearPayload)}`);
  assert(clearPayload.data?.cleared === '#name', 'browser_clear did not report cleared selector');
  assert(clearPayload.trace?.serviceName === serviceName, 'browser_clear trace missing serviceName');
  assert(clearPayload.trace?.agentName === agentName, 'browser_clear trace missing agentName');
  assert(clearPayload.trace?.taskName === taskName, 'browser_clear trace missing taskName');

  const fillResult = await send('tools/call', {
    name: 'browser_fill',
    arguments: {
      selector: '#name',
      value: 'Ada Lovelace',
      serviceName,
      agentName,
      taskName,
    },
  });
  const fillPayload = parseToolPayload(fillResult);
  assert(fillPayload.success === true, `browser_fill failed: ${JSON.stringify(fillPayload)}`);
  assert(fillPayload.data?.filled === '#name', 'browser_fill did not report filled selector');
  assert(fillPayload.trace?.serviceName === serviceName, 'browser_fill trace missing serviceName');
  assert(fillPayload.trace?.agentName === agentName, 'browser_fill trace missing agentName');
  assert(fillPayload.trace?.taskName === taskName, 'browser_fill trace missing taskName');

  const typeResult = await send('tools/call', {
    name: 'browser_type',
    arguments: {
      selector: '#name',
      text: ' Jr',
      delayMs: 1,
      serviceName,
      agentName,
      taskName,
    },
  });
  const typePayload = parseToolPayload(typeResult);
  assert(typePayload.success === true, `browser_type failed: ${JSON.stringify(typePayload)}`);
  assert(typePayload.data?.typed === ' Jr', 'browser_type did not report typed text');
  assert(typePayload.trace?.serviceName === serviceName, 'browser_type trace missing serviceName');
  assert(typePayload.trace?.agentName === agentName, 'browser_type trace missing agentName');
  assert(typePayload.trace?.taskName === taskName, 'browser_type trace missing taskName');

  const pressResult = await send('tools/call', {
    name: 'browser_press',
    arguments: {
      key: 'Enter',
      serviceName,
      agentName,
      taskName,
    },
  });
  const pressPayload = parseToolPayload(pressResult);
  assert(pressPayload.success === true, `browser_press failed: ${JSON.stringify(pressPayload)}`);
  assert(pressPayload.data?.pressed === 'Enter', 'browser_press did not report pressed key');
  assert(pressPayload.trace?.serviceName === serviceName, 'browser_press trace missing serviceName');
  assert(pressPayload.trace?.agentName === agentName, 'browser_press trace missing agentName');
  assert(pressPayload.trace?.taskName === taskName, 'browser_press trace missing taskName');

  const hoverResult = await send('tools/call', {
    name: 'browser_hover',
    arguments: {
      selector: '#hover-target',
      serviceName,
      agentName,
      taskName,
    },
  });
  const hoverPayload = parseToolPayload(hoverResult);
  assert(hoverPayload.success === true, `browser_hover failed: ${JSON.stringify(hoverPayload)}`);
  assert(
    hoverPayload.data?.hovered === '#hover-target',
    'browser_hover did not report hovered selector',
  );
  assert(hoverPayload.trace?.serviceName === serviceName, 'browser_hover trace missing serviceName');
  assert(hoverPayload.trace?.agentName === agentName, 'browser_hover trace missing agentName');
  assert(hoverPayload.trace?.taskName === taskName, 'browser_hover trace missing taskName');

  const selectResult = await send('tools/call', {
    name: 'browser_select',
    arguments: {
      selector: '#org',
      values: ['org-b'],
      serviceName,
      agentName,
      taskName,
    },
  });
  const selectPayload = parseToolPayload(selectResult);
  assert(
    selectPayload.success === true,
    `browser_select failed: ${JSON.stringify(selectPayload)}`,
  );
  assert(
    Array.isArray(selectPayload.data?.selected) && selectPayload.data.selected[0] === 'org-b',
    'browser_select did not report selected value',
  );
  assert(
    selectPayload.trace?.serviceName === serviceName,
    'browser_select trace missing serviceName',
  );
  assert(selectPayload.trace?.agentName === agentName, 'browser_select trace missing agentName');
  assert(selectPayload.trace?.taskName === taskName, 'browser_select trace missing taskName');

  const textResult = await send('tools/call', {
    name: 'browser_get_text',
    arguments: {
      selector: '#org-output',
      serviceName,
      agentName,
      taskName,
    },
  });
  const textPayload = parseToolPayload(textResult);
  assert(textPayload.success === true, `browser_get_text failed: ${JSON.stringify(textPayload)}`);
  assert(textPayload.data?.text === 'org-b', 'browser_get_text did not return selected output text');
  assert(textPayload.trace?.serviceName === serviceName, 'browser_get_text trace missing serviceName');
  assert(textPayload.trace?.agentName === agentName, 'browser_get_text trace missing agentName');
  assert(textPayload.trace?.taskName === taskName, 'browser_get_text trace missing taskName');

  const valueResult = await send('tools/call', {
    name: 'browser_get_value',
    arguments: {
      selector: '#name',
      serviceName,
      agentName,
      taskName,
    },
  });
  const valuePayload = parseToolPayload(valueResult);
  assert(valuePayload.success === true, `browser_get_value failed: ${JSON.stringify(valuePayload)}`);
  assert(
    valuePayload.data?.value === 'Ada Lovelace Jr',
    'browser_get_value did not return typed field value',
  );
  assert(valuePayload.trace?.serviceName === serviceName, 'browser_get_value trace missing serviceName');
  assert(valuePayload.trace?.agentName === agentName, 'browser_get_value trace missing agentName');
  assert(valuePayload.trace?.taskName === taskName, 'browser_get_value trace missing taskName');

  const attributeResult = await send('tools/call', {
    name: 'browser_get_attribute',
    arguments: {
      selector: 'a[href]',
      attribute: 'href',
      serviceName,
      agentName,
      taskName,
    },
  });
  const attributePayload = parseToolPayload(attributeResult);
  assert(
    attributePayload.success === true,
    `browser_get_attribute failed: ${JSON.stringify(attributePayload)}`,
  );
  assert(
    attributePayload.data?.value === 'https://example.com/',
    'browser_get_attribute did not return link href',
  );
  assert(
    attributePayload.trace?.serviceName === serviceName,
    'browser_get_attribute trace missing serviceName',
  );
  assert(
    attributePayload.trace?.agentName === agentName,
    'browser_get_attribute trace missing agentName',
  );
  assert(
    attributePayload.trace?.taskName === taskName,
    'browser_get_attribute trace missing taskName',
  );

  const htmlResult = await send('tools/call', {
    name: 'browser_get_html',
    arguments: {
      selector: '#box',
      serviceName,
      agentName,
      taskName,
    },
  });
  const htmlPayload = parseToolPayload(htmlResult);
  assert(htmlPayload.success === true, `browser_get_html failed: ${JSON.stringify(htmlPayload)}`);
  assert(
    htmlPayload.data?.html === 'Box target',
    'browser_get_html did not return element inner HTML',
  );
  assert(htmlPayload.trace?.serviceName === serviceName, 'browser_get_html trace missing serviceName');
  assert(htmlPayload.trace?.agentName === agentName, 'browser_get_html trace missing agentName');
  assert(htmlPayload.trace?.taskName === taskName, 'browser_get_html trace missing taskName');

  const stylesResult = await send('tools/call', {
    name: 'browser_get_styles',
    arguments: {
      selector: '#box',
      properties: ['display', 'width', 'height'],
      serviceName,
      agentName,
      taskName,
    },
  });
  const stylesPayload = parseToolPayload(stylesResult);
  assert(
    stylesPayload.success === true,
    `browser_get_styles failed: ${JSON.stringify(stylesPayload)}`,
  );
  assert(stylesPayload.data?.styles?.display === 'block', 'browser_get_styles did not return display');
  assert(stylesPayload.data?.styles?.width === '200px', 'browser_get_styles did not return width');
  assert(stylesPayload.data?.styles?.height === '100px', 'browser_get_styles did not return height');
  assert(
    stylesPayload.trace?.serviceName === serviceName,
    'browser_get_styles trace missing serviceName',
  );
  assert(stylesPayload.trace?.agentName === agentName, 'browser_get_styles trace missing agentName');
  assert(stylesPayload.trace?.taskName === taskName, 'browser_get_styles trace missing taskName');

  const countResult = await send('tools/call', {
    name: 'browser_count',
    arguments: {
      selector: '.item',
      serviceName,
      agentName,
      taskName,
    },
  });
  const countPayload = parseToolPayload(countResult);
  assert(countPayload.success === true, `browser_count failed: ${JSON.stringify(countPayload)}`);
  assert(countPayload.data?.count === 3, 'browser_count did not return matching item count');
  assert(countPayload.trace?.serviceName === serviceName, 'browser_count trace missing serviceName');
  assert(countPayload.trace?.agentName === agentName, 'browser_count trace missing agentName');
  assert(countPayload.trace?.taskName === taskName, 'browser_count trace missing taskName');

  const boxResult = await send('tools/call', {
    name: 'browser_get_box',
    arguments: {
      selector: '#box',
      serviceName,
      agentName,
      taskName,
    },
  });
  const boxPayload = parseToolPayload(boxResult);
  assert(boxPayload.success === true, `browser_get_box failed: ${JSON.stringify(boxPayload)}`);
  assert(boxPayload.data?.width === 200, 'browser_get_box did not return expected width');
  assert(boxPayload.data?.height === 100, 'browser_get_box did not return expected height');
  assert(typeof boxPayload.data?.x === 'number', 'browser_get_box did not return x coordinate');
  assert(typeof boxPayload.data?.y === 'number', 'browser_get_box did not return y coordinate');
  assert(boxPayload.trace?.serviceName === serviceName, 'browser_get_box trace missing serviceName');
  assert(boxPayload.trace?.agentName === agentName, 'browser_get_box trace missing agentName');
  assert(boxPayload.trace?.taskName === taskName, 'browser_get_box trace missing taskName');

  const initiallyCheckedResult = await send('tools/call', {
    name: 'browser_is_checked',
    arguments: {
      selector: '#remember',
      serviceName,
      agentName,
      taskName,
    },
  });
  const initiallyCheckedPayload = parseToolPayload(initiallyCheckedResult);
  assert(
    initiallyCheckedPayload.success === true,
    `initial browser_is_checked failed: ${JSON.stringify(initiallyCheckedPayload)}`,
  );
  assert(
    initiallyCheckedPayload.data?.checked === false,
    'browser_is_checked did not report initial unchecked state',
  );
  assert(
    initiallyCheckedPayload.trace?.serviceName === serviceName,
    'initial browser_is_checked trace missing serviceName',
  );
  assert(
    initiallyCheckedPayload.trace?.agentName === agentName,
    'initial browser_is_checked trace missing agentName',
  );
  assert(
    initiallyCheckedPayload.trace?.taskName === taskName,
    'initial browser_is_checked trace missing taskName',
  );

  const checkResult = await send('tools/call', {
    name: 'browser_check',
    arguments: {
      selector: '#remember',
      serviceName,
      agentName,
      taskName,
    },
  });
  const checkPayload = parseToolPayload(checkResult);
  assert(checkPayload.success === true, `browser_check failed: ${JSON.stringify(checkPayload)}`);
  assert(checkPayload.data?.checked === '#remember', 'browser_check did not report checked selector');
  assert(checkPayload.trace?.serviceName === serviceName, 'browser_check trace missing serviceName');
  assert(checkPayload.trace?.agentName === agentName, 'browser_check trace missing agentName');
  assert(checkPayload.trace?.taskName === taskName, 'browser_check trace missing taskName');

  const checkedStateResult = await send('tools/call', {
    name: 'browser_is_checked',
    arguments: {
      selector: '#remember',
      serviceName,
      agentName,
      taskName,
    },
  });
  const checkedStatePayload = parseToolPayload(checkedStateResult);
  assert(
    checkedStatePayload.success === true,
    `checked browser_is_checked failed: ${JSON.stringify(checkedStatePayload)}`,
  );
  assert(
    checkedStatePayload.data?.checked === true,
    'browser_is_checked did not report checked state after browser_check',
  );

  const uncheckResult = await send('tools/call', {
    name: 'browser_uncheck',
    arguments: {
      selector: '#remember',
      serviceName,
      agentName,
      taskName,
    },
  });
  const uncheckPayload = parseToolPayload(uncheckResult);
  assert(
    uncheckPayload.success === true,
    `browser_uncheck failed: ${JSON.stringify(uncheckPayload)}`,
  );
  assert(
    uncheckPayload.data?.unchecked === '#remember',
    'browser_uncheck did not report unchecked selector',
  );
  assert(
    uncheckPayload.trace?.serviceName === serviceName,
    'browser_uncheck trace missing serviceName',
  );
  assert(uncheckPayload.trace?.agentName === agentName, 'browser_uncheck trace missing agentName');
  assert(uncheckPayload.trace?.taskName === taskName, 'browser_uncheck trace missing taskName');

  const uncheckedStateResult = await send('tools/call', {
    name: 'browser_is_checked',
    arguments: {
      selector: '#remember',
      serviceName,
      agentName,
      taskName,
    },
  });
  const uncheckedStatePayload = parseToolPayload(uncheckedStateResult);
  assert(
    uncheckedStatePayload.success === true,
    `unchecked browser_is_checked failed: ${JSON.stringify(uncheckedStatePayload)}`,
  );
  assert(
    uncheckedStatePayload.data?.checked === false,
    'browser_is_checked did not report unchecked state after browser_uncheck',
  );

  const scrollResult = await send('tools/call', {
    name: 'browser_scroll',
    arguments: {
      direction: 'down',
      amount: 800,
      serviceName,
      agentName,
      taskName,
    },
  });
  const scrollPayload = parseToolPayload(scrollResult);
  assert(scrollPayload.success === true, `browser_scroll failed: ${JSON.stringify(scrollPayload)}`);
  assert(scrollPayload.data?.scrolled === true, 'browser_scroll did not report scrolled state');
  assert(
    scrollPayload.trace?.serviceName === serviceName,
    'browser_scroll trace missing serviceName',
  );
  assert(scrollPayload.trace?.agentName === agentName, 'browser_scroll trace missing agentName');
  assert(scrollPayload.trace?.taskName === taskName, 'browser_scroll trace missing taskName');

  const scrollWaitResult = await send('tools/call', {
    name: 'browser_wait',
    arguments: {
      function: 'window.scrollY > 0',
      timeoutMs: 5000,
      serviceName,
      agentName,
      taskName,
    },
  });
  const scrollWaitPayload = parseToolPayload(scrollWaitResult);
  assert(
    scrollWaitPayload.success === true,
    `post-scroll browser_wait failed: ${JSON.stringify(scrollWaitPayload)}`,
  );
  assert(
    scrollWaitPayload.data?.result === true,
    'post-scroll browser_wait did not confirm scroll position',
  );

  const scrollIntoViewResult = await send('tools/call', {
    name: 'browser_scroll_into_view',
    arguments: {
      selector: '#scroll-target',
      serviceName,
      agentName,
      taskName,
    },
  });
  const scrollIntoViewPayload = parseToolPayload(scrollIntoViewResult);
  assert(
    scrollIntoViewPayload.success === true,
    `browser_scroll_into_view failed: ${JSON.stringify(scrollIntoViewPayload)}`,
  );
  assert(
    scrollIntoViewPayload.data?.scrolled === '#scroll-target',
    'browser_scroll_into_view did not report scrolled selector',
  );
  assert(
    scrollIntoViewPayload.trace?.serviceName === serviceName,
    'browser_scroll_into_view trace missing serviceName',
  );
  assert(
    scrollIntoViewPayload.trace?.agentName === agentName,
    'browser_scroll_into_view trace missing agentName',
  );
  assert(
    scrollIntoViewPayload.trace?.taskName === taskName,
    'browser_scroll_into_view trace missing taskName',
  );

  const scrollTargetClickResult = await send('tools/call', {
    name: 'browser_click',
    arguments: {
      selector: '#scroll-target',
      serviceName,
      agentName,
      taskName,
    },
  });
  const scrollTargetClickPayload = parseToolPayload(scrollTargetClickResult);
  assert(
    scrollTargetClickPayload.success === true,
    `post-scroll-into-view browser_click failed: ${JSON.stringify(scrollTargetClickPayload)}`,
  );

  const copyNameResult = await send('tools/call', {
    name: 'browser_click',
    arguments: {
      selector: '#copy-name',
      serviceName,
      agentName,
      taskName,
    },
  });
  const copyNamePayload = parseToolPayload(copyNameResult);
  assert(
    copyNamePayload.success === true,
    `post-fill browser_click failed: ${JSON.stringify(copyNamePayload)}`,
  );
  assert(
    copyNamePayload.data?.clicked === '#copy-name',
    'post-fill browser_click did not report clicked selector',
  );

  const waitResult = await send('tools/call', {
    name: 'browser_wait',
    arguments: {
      text: 'Name copied',
      timeoutMs: 5000,
      serviceName,
      agentName,
      taskName,
    },
  });
  const waitPayload = parseToolPayload(waitResult);
  assert(waitPayload.success === true, `browser_wait failed: ${JSON.stringify(waitPayload)}`);
  assert(waitPayload.data?.waited === 'text', 'browser_wait did not report text wait');
  assert(waitPayload.data?.text === 'Name copied', 'browser_wait did not report waited text');
  assert(waitPayload.trace?.serviceName === serviceName, 'browser_wait trace missing serviceName');
  assert(waitPayload.trace?.agentName === agentName, 'browser_wait trace missing agentName');
  assert(waitPayload.trace?.taskName === taskName, 'browser_wait trace missing taskName');

  const clickedSnapshotResult = await send('tools/call', {
    name: 'browser_snapshot',
    arguments: {
      selector: '#main',
      serviceName,
      agentName,
      taskName,
    },
  });
  const clickedSnapshotPayload = parseToolPayload(clickedSnapshotResult);
  assert(
    clickedSnapshotPayload.success === true,
    `post-click browser_snapshot failed: ${JSON.stringify(clickedSnapshotPayload)}`,
  );
  assert(
    typeof clickedSnapshotPayload.data?.snapshot === 'string' &&
      clickedSnapshotPayload.data.snapshot.includes('Clicked'),
    'browser_click did not mutate the page state visible to a follow-up snapshot',
  );
  assert(
    typeof clickedSnapshotPayload.data?.snapshot === 'string' &&
      clickedSnapshotPayload.data.snapshot.includes('Ada Lovelace Jr'),
    'browser_fill did not mutate the page state visible to a follow-up snapshot',
  );
  assert(
    typeof clickedSnapshotPayload.data?.snapshot === 'string' &&
      clickedSnapshotPayload.data.snapshot.includes('Focused name'),
    'browser_focus did not mutate the page state visible to a follow-up snapshot',
  );
  assert(
    typeof clickedSnapshotPayload.data?.snapshot === 'string' &&
      clickedSnapshotPayload.data.snapshot.includes('Name cleared'),
    'browser_clear did not mutate the page state visible to a follow-up snapshot',
  );
  assert(
    typeof clickedSnapshotPayload.data?.snapshot === 'string' &&
      clickedSnapshotPayload.data.snapshot.includes('Pressed Enter'),
    'browser_press did not mutate the page state visible to a follow-up snapshot',
  );
  assert(
    typeof clickedSnapshotPayload.data?.snapshot === 'string' &&
      clickedSnapshotPayload.data.snapshot.includes('Hovered menu'),
    'browser_hover did not mutate the page state visible to a follow-up snapshot',
  );
  assert(
    typeof clickedSnapshotPayload.data?.snapshot === 'string' &&
      clickedSnapshotPayload.data.snapshot.includes('org-b'),
    'browser_select did not mutate the page state visible to a follow-up snapshot',
  );
  assert(
    typeof clickedSnapshotPayload.data?.snapshot === 'string' &&
      clickedSnapshotPayload.data.snapshot.includes('Remember unchecked'),
    'browser_uncheck did not mutate the page state visible to a follow-up snapshot',
  );
  assert(
    typeof clickedSnapshotPayload.data?.snapshot === 'string' &&
      clickedSnapshotPayload.data.snapshot.includes('Scrolled target clicked'),
    'browser_scroll_into_view did not make the offscreen target interactable',
  );

  const setContentResult = await send('tools/call', {
    name: 'browser_set_content',
    arguments: {
      html: '<main id="replacement"><h1>Typed set content</h1></main>',
      serviceName,
      agentName,
      taskName,
    },
  });
  const setContentPayload = parseToolPayload(setContentResult);
  assert(
    setContentPayload.success === true,
    `browser_set_content failed: ${JSON.stringify(setContentPayload)}`,
  );
  assert(setContentPayload.data?.set === true, 'browser_set_content did not report set state');
  assert(
    setContentPayload.trace?.serviceName === serviceName,
    'browser_set_content trace missing serviceName',
  );

  const setContentSnapshotResult = await send('tools/call', {
    name: 'browser_snapshot',
    arguments: {
      selector: '#replacement',
      serviceName,
      agentName,
      taskName,
    },
  });
  const setContentSnapshotPayload = parseToolPayload(setContentSnapshotResult);
  assert(
    setContentSnapshotPayload.success === true,
    `browser_set_content snapshot failed: ${JSON.stringify(setContentSnapshotPayload)}`,
  );
  assert(
    typeof setContentSnapshotPayload.data?.snapshot === 'string' &&
      setContentSnapshotPayload.data.snapshot.includes('Typed set content'),
    'browser_set_content did not replace the page content visible to a follow-up snapshot',
  );

  const jobs = await send('resources/read', { uri: 'agent-browser://jobs' });
  const jobPayload = JSON.parse(jobs.contents?.[0]?.text || '{}');
  const snapshotJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'snapshot' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(snapshotJob, 'Retained service job with browser_snapshot caller context was not found');
  assert(snapshotJob.state === 'succeeded', `Snapshot service job state was ${snapshotJob.state}`);
  const browserCommandJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'navigate' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(browserCommandJob, 'Retained service job with browser_command caller context was not found');
  assert(
    browserCommandJob.state === 'succeeded',
    `Browser command service job state was ${browserCommandJob.state}`,
  );
  const browserCommandRequestsJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'requests' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(
    browserCommandRequestsJob,
    'Retained service job with browser_command requests caller context was not found',
  );
  assert(
    browserCommandRequestsJob.state === 'succeeded',
    `Browser command requests service job state was ${browserCommandRequestsJob.state}`,
  );
  const browserRequestDetailJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'request_detail' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(
    browserRequestDetailJob,
    'Retained service job with browser_request_detail caller context was not found',
  );
  assert(
    browserRequestDetailJob.state === 'succeeded',
    `Browser request detail service job state was ${browserRequestDetailJob.state}`,
  );
  const browserHeadersJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'headers' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(browserHeadersJob, 'Retained service job with browser_headers caller context was not found');
  assert(
    browserHeadersJob.state === 'succeeded',
    `Browser headers service job state was ${browserHeadersJob.state}`,
  );
  const browserOfflineJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'offline' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(browserOfflineJob, 'Retained service job with browser_offline caller context was not found');
  assert(
    browserOfflineJob.state === 'succeeded',
    `Browser offline service job state was ${browserOfflineJob.state}`,
  );
  for (const [action, label] of [
    ['reload', 'browser_reload'],
    ['back', 'browser_back'],
    ['forward', 'browser_forward'],
    ['user_agent', 'browser_user_agent'],
    ['viewport', 'browser_viewport'],
    ['geolocation', 'browser_geolocation'],
    ['permissions', 'browser_permissions'],
    ['timezone', 'browser_timezone'],
    ['locale', 'browser_locale'],
    ['emulatemedia', 'browser_media'],
    ['dialog', 'browser_dialog'],
    ['upload', 'browser_upload'],
    ['har_start', 'browser_har_start'],
    ['har_stop', 'browser_har_stop'],
    ['route', 'browser_route'],
    ['unroute', 'browser_unroute'],
    ['console', 'browser_console'],
    ['errors', 'browser_errors'],
    ['pdf', 'browser_pdf'],
    ['cookies_set', 'browser_cookies_set'],
    ['cookies_get', 'browser_cookies_get'],
    ['cookies_clear', 'browser_cookies_clear'],
    ['storage_set', 'browser_storage_set'],
    ['storage_get', 'browser_storage_get'],
    ['storage_clear', 'browser_storage_clear'],
    ['tab_new', 'browser_tab_new'],
    ['tab_switch', 'browser_tab_switch'],
    ['tab_close', 'browser_tab_close'],
    ['setcontent', 'browser_set_content'],
  ]) {
    const job = jobPayload.jobs?.find(
      (candidate) =>
        candidate.action === action &&
        candidate.serviceName === serviceName &&
        candidate.agentName === agentName &&
        candidate.taskName === taskName,
    );
    assert(job, `Retained service job with ${label} caller context was not found`);
    assert(job.state === 'succeeded', `${label} service job state was ${job.state}`);
  }
  const urlJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'url' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(urlJob, 'Retained service job with browser_get_url caller context was not found');
  assert(urlJob.state === 'succeeded', `URL service job state was ${urlJob.state}`);
  const titleJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'title' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(titleJob, 'Retained service job with browser_get_title caller context was not found');
  assert(titleJob.state === 'succeeded', `Title service job state was ${titleJob.state}`);
  const tabsJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'tab_list' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(tabsJob, 'Retained service job with browser_tabs caller context was not found');
  assert(tabsJob.state === 'succeeded', `Tabs service job state was ${tabsJob.state}`);
  const screenshotJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'screenshot' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(
    screenshotJob,
    'Retained service job with browser_screenshot caller context was not found',
  );
  assert(
    screenshotJob.state === 'succeeded',
    `Screenshot service job state was ${screenshotJob.state}`,
  );
  const clickJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'click' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(clickJob, 'Retained service job with browser_click caller context was not found');
  assert(clickJob.state === 'succeeded', `Click service job state was ${clickJob.state}`);
  const isVisibleJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'isvisible' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(isVisibleJob, 'Retained service job with browser_is_visible caller context was not found');
  assert(
    isVisibleJob.state === 'succeeded',
    `Visible-state service job state was ${isVisibleJob.state}`,
  );
  const isEnabledJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'isenabled' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(isEnabledJob, 'Retained service job with browser_is_enabled caller context was not found');
  assert(
    isEnabledJob.state === 'succeeded',
    `Enabled-state service job state was ${isEnabledJob.state}`,
  );
  const focusJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'focus' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(focusJob, 'Retained service job with browser_focus caller context was not found');
  assert(focusJob.state === 'succeeded', `Focus service job state was ${focusJob.state}`);
  const clearJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'clear' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(clearJob, 'Retained service job with browser_clear caller context was not found');
  assert(clearJob.state === 'succeeded', `Clear service job state was ${clearJob.state}`);
  const fillJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'fill' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(fillJob, 'Retained service job with browser_fill caller context was not found');
  assert(fillJob.state === 'succeeded', `Fill service job state was ${fillJob.state}`);
  const typeJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'type' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(typeJob, 'Retained service job with browser_type caller context was not found');
  assert(typeJob.state === 'succeeded', `Type service job state was ${typeJob.state}`);
  const pressJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'press' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(pressJob, 'Retained service job with browser_press caller context was not found');
  assert(pressJob.state === 'succeeded', `Press service job state was ${pressJob.state}`);
  const hoverJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'hover' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(hoverJob, 'Retained service job with browser_hover caller context was not found');
  assert(hoverJob.state === 'succeeded', `Hover service job state was ${hoverJob.state}`);
  const selectJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'select' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(selectJob, 'Retained service job with browser_select caller context was not found');
  assert(selectJob.state === 'succeeded', `Select service job state was ${selectJob.state}`);
  const textJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'gettext' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(textJob, 'Retained service job with browser_get_text caller context was not found');
  assert(textJob.state === 'succeeded', `Text service job state was ${textJob.state}`);
  const valueJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'inputvalue' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(valueJob, 'Retained service job with browser_get_value caller context was not found');
  assert(valueJob.state === 'succeeded', `Value service job state was ${valueJob.state}`);
  const attributeJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'getattribute' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(
    attributeJob,
    'Retained service job with browser_get_attribute caller context was not found',
  );
  assert(
    attributeJob.state === 'succeeded',
    `Attribute service job state was ${attributeJob.state}`,
  );
  const htmlJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'innerhtml' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(htmlJob, 'Retained service job with browser_get_html caller context was not found');
  assert(htmlJob.state === 'succeeded', `HTML service job state was ${htmlJob.state}`);
  const stylesJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'styles' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(stylesJob, 'Retained service job with browser_get_styles caller context was not found');
  assert(stylesJob.state === 'succeeded', `Styles service job state was ${stylesJob.state}`);
  const countJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'count' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(countJob, 'Retained service job with browser_count caller context was not found');
  assert(countJob.state === 'succeeded', `Count service job state was ${countJob.state}`);
  const boxJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'boundingbox' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(boxJob, 'Retained service job with browser_get_box caller context was not found');
  assert(boxJob.state === 'succeeded', `Box service job state was ${boxJob.state}`);
  const checkJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'check' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(checkJob, 'Retained service job with browser_check caller context was not found');
  assert(checkJob.state === 'succeeded', `Check service job state was ${checkJob.state}`);
  const isCheckedJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'ischecked' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(isCheckedJob, 'Retained service job with browser_is_checked caller context was not found');
  assert(
    isCheckedJob.state === 'succeeded',
    `Checked-state service job state was ${isCheckedJob.state}`,
  );
  const uncheckJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'uncheck' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(uncheckJob, 'Retained service job with browser_uncheck caller context was not found');
  assert(uncheckJob.state === 'succeeded', `Uncheck service job state was ${uncheckJob.state}`);
  const scrollJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'scroll' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(scrollJob, 'Retained service job with browser_scroll caller context was not found');
  assert(scrollJob.state === 'succeeded', `Scroll service job state was ${scrollJob.state}`);
  const scrollIntoViewJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'scrollintoview' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(
    scrollIntoViewJob,
    'Retained service job with browser_scroll_into_view caller context was not found',
  );
  assert(
    scrollIntoViewJob.state === 'succeeded',
    `Scroll into view service job state was ${scrollIntoViewJob.state}`,
  );
  const waitJob = jobPayload.jobs?.find(
    (job) =>
      job.action === 'wait' &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
  assert(waitJob, 'Retained service job with browser_wait caller context was not found');
  assert(waitJob.state === 'succeeded', `Wait service job state was ${waitJob.state}`);

  const traceResult = await send('tools/call', {
    name: 'service_trace',
    arguments: {
      limit: 50,
      serviceName,
      agentName,
      taskName,
    },
  });
  const tracePayload = parseToolPayload(traceResult);
  assert(tracePayload.success === true, `service_trace failed: ${JSON.stringify(tracePayload)}`);
  assert(tracePayload.tool === 'service_trace', 'service_trace payload tool mismatch');
  assert(
    tracePayload.trace?.serviceName === serviceName,
    'service_trace response trace missing serviceName',
  );
  assert(tracePayload.trace?.agentName === agentName, 'service_trace response trace missing agentName');
  assert(tracePayload.trace?.taskName === taskName, 'service_trace response trace missing taskName');
  assert(
    tracePayload.data?.filters?.serviceName === serviceName,
    'service_trace filters missing serviceName',
  );
  assert(
    tracePayload.data?.filters?.agentName === agentName,
    'service_trace filters missing agentName',
  );
  assert(
    tracePayload.data?.filters?.taskName === taskName,
    'service_trace filters missing taskName',
  );
  assert(Array.isArray(tracePayload.data?.events), 'service_trace missing events array');
  assert(Array.isArray(tracePayload.data?.jobs), 'service_trace missing jobs array');
  assert(Array.isArray(tracePayload.data?.incidents), 'service_trace missing incidents array');
  assert(Array.isArray(tracePayload.data?.activity), 'service_trace missing activity array');
  assert(
    tracePayload.data.jobs.length > 0,
    'service_trace did not return retained jobs for the live MCP trace context',
  );
  assert(
    tracePayload.data.matched?.jobs >= tracePayload.data.jobs.length,
    'service_trace matched job count is inconsistent with returned jobs',
  );
  assert(
    tracePayload.data.jobs.some((job) => job.id === waitJob.id),
    'service_trace did not include a known retained MCP job',
  );
  assert(
    tracePayload.data.jobs.every(
      (job) =>
        job.serviceName === serviceName &&
        job.agentName === agentName &&
        job.taskName === taskName,
    ),
    'service_trace returned a job outside the requested trace context',
  );

  await cleanup();
  console.log('MCP live smoke passed');
} catch (err) {
  await fail(err.message);
}
