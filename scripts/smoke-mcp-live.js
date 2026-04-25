#!/usr/bin/env node

import { existsSync, mkdirSync } from 'node:fs';
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
const serviceName = 'McpLiveSmoke';
const agentName = 'smoke-agent';
const taskName = 'browserSnapshotSmoke';

mkdirSync(profileDir, { recursive: true });
mkdirSync(screenshotDir, { recursive: true });

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
    name: 'browser_command',
    arguments: {
      action: 'requests',
      params: {},
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandTrackRequestsPayload = parseToolPayload(commandTrackRequestsResult);
  assert(
    commandTrackRequestsPayload.success === true,
    `browser_command requests tracking failed: ${JSON.stringify(commandTrackRequestsPayload)}`,
  );
  assert(
    Array.isArray(commandTrackRequestsPayload.data?.requests),
    'browser_command requests did not return a request array',
  );

  const commandNavigateResult = await send('tools/call', {
    name: 'browser_command',
    arguments: {
      action: 'navigate',
      params: {
        url: browserCommandUrl,
        waitUntil: 'load',
      },
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandNavigatePayload = parseToolPayload(commandNavigateResult);
  assert(
    commandNavigatePayload.success === true,
    `browser_command navigate failed: ${JSON.stringify(commandNavigatePayload)}`,
  );
  assert(
    commandNavigatePayload.data?.url === browserCommandUrl,
    'browser_command navigate did not report the requested URL',
  );
  assert(
    commandNavigatePayload.trace?.serviceName === serviceName,
    'browser_command navigate trace missing serviceName',
  );
  assert(
    commandNavigatePayload.trace?.agentName === agentName,
    'browser_command navigate trace missing agentName',
  );
  assert(
    commandNavigatePayload.trace?.taskName === taskName,
    'browser_command navigate trace missing taskName',
  );

  const commandRequestsResult = await send('tools/call', {
    name: 'browser_command',
    arguments: {
      action: 'requests',
      params: {
        filter: '/asset.png',
        method: 'GET',
        status: '2xx',
      },
      serviceName,
      agentName,
      taskName,
    },
  });
  const commandRequestsPayload = parseToolPayload(commandRequestsResult);
  assert(
    commandRequestsPayload.success === true,
    `browser_command requests failed: ${JSON.stringify(commandRequestsPayload)}`,
  );
  const commandRequests = commandRequestsPayload.data?.requests;
  assert(Array.isArray(commandRequests), 'browser_command requests did not return requests array');
  assert(
    commandRequests.some(
      (request) =>
        typeof request.url === 'string' &&
        request.url.includes('/asset.png?probe=1') &&
        request.method === 'GET' &&
        request.status === 200,
    ),
    `browser_command requests did not capture the local asset request: ${JSON.stringify(commandRequests)}`,
  );
  assert(
    commandRequestsPayload.trace?.serviceName === serviceName,
    'browser_command requests trace missing serviceName',
  );
  assert(
    commandRequestsPayload.trace?.agentName === agentName,
    'browser_command requests trace missing agentName',
  );
  assert(
    commandRequestsPayload.trace?.taskName === taskName,
    'browser_command requests trace missing taskName',
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
