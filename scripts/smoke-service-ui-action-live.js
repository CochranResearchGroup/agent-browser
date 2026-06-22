#!/usr/bin/env node

import { request } from 'node:http';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-ui-action-',
  sessionPrefix: 'service-ui-action',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'ServiceUiActionSmoke';
const agentName = 'smoke-agent';
const taskName = 'plan0034UiAction';
const targetServiceId = 'generic-ui-site';
const browserId = `session:${session}`;
const html = `<!doctype html>
<html>
  <head>
    <title>Plan 0034 Generic UI</title>
    <style>
      #menu-panel[hidden] { display: none; }
    </style>
  </head>
  <body>
    <main>
      <label>Query <input id="query" /></label>
      <select id="choice">
        <option value="alpha">Alpha</option>
        <option value="beta">Beta</option>
      </select>
      <button id="apply">Apply</button>
      <button id="menu-button">Menu</button>
      <div id="menu-panel" hidden>
        <button id="menu-option">Menu option</button>
      </div>
      <button id="confirm-button">Confirm</button>
      <p id="status">Idle</p>
    </main>
    <script>
      const status = document.querySelector('#status');
      document.querySelector('#apply').addEventListener('click', () => {
        const query = document.querySelector('#query').value;
        const choice = document.querySelector('#choice').value;
        setTimeout(() => { status.textContent = 'Applied ' + query + ' ' + choice; }, 50);
      });
      document.querySelector('#menu-button').addEventListener('click', () => {
        document.querySelector('#menu-panel').hidden = false;
      });
      document.querySelector('#menu-option').addEventListener('click', () => {
        status.textContent = 'Menu option selected';
      });
      document.querySelector('#confirm-button').addEventListener('click', () => {
        setTimeout(() => {
          if (confirm('Proceed with generic dialog?')) {
            status.textContent = 'Dialog accepted';
          }
        }, 50);
      });
    </script>
  </body>
</html>`;
const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(html)}`;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service UI action live smoke to complete');
}, 180000);

let streamPort;

async function cleanup() {
  clearTimeout(timeout);
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
}

function httpJsonWithTimeout(port, method, path, body, timeoutMs) {
  return new Promise((resolve, reject) => {
    const rawBody = body === undefined ? undefined : JSON.stringify(body);
    const req = request(
      {
        host: '127.0.0.1',
        port,
        method,
        path,
        headers: rawBody
          ? {
              'content-type': 'application/json',
              'content-length': Buffer.byteLength(rawBody),
            }
          : undefined,
      },
      (res) => {
        let text = '';
        res.setEncoding('utf8');
        res.on('data', (chunk) => {
          text += chunk;
        });
        res.on('end', () => {
          try {
            const parsed = JSON.parse(text);
            if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
              reject(new Error(`HTTP ${method} ${path} returned ${res.statusCode}: ${text}`));
              return;
            }
            resolve(parsed);
          } catch (err) {
            reject(new Error(`Failed to parse HTTP ${method} ${path}: ${err.message}\n${text}`));
          }
        });
      },
    );
    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error(`HTTP ${method} ${path} timed out after ${timeoutMs}ms`));
    });
    req.on('error', reject);
    if (rawBody) req.write(rawBody);
    req.end();
  });
}

async function serviceRequest(body, label) {
  let response;
  try {
    response = await httpJsonWithTimeout(streamPort, 'POST', '/api/service/request', {
      serviceName,
      agentName,
      taskName,
      targetServiceId,
      jobTimeoutMs: 60000,
      ...body,
    }, 90000);
  } catch (err) {
    throw new Error(`${label} failed: ${err.message}`);
  }
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
  return response;
}

async function serviceTrace() {
  const result = await runCli(
    context,
    [
      '--json',
      '--session',
      session,
      'service',
      'trace',
      '--service-name',
      serviceName,
      '--agent-name',
      agentName,
      '--task-name',
      taskName,
      '--limit',
      '100',
    ],
    60000,
  );
  const trace = parseJsonOutput(result.stdout, 'service trace');
  assert(trace.success === true, `service trace failed: ${result.stdout}${result.stderr}`);
  return trace.data;
}

try {
  streamPort = await ensureStreamPort(context, 120000);

  const tabResponse = await serviceRequest(
    {
      action: 'tab_new',
      params: {
        headless: true,
        url: pageUrl,
        waitUntil: 'load',
      },
    },
    'tab_new',
  );
  const handle = tabResponse.data?.serviceTabHandle;
  assert(handle?.valid === true, `tab_new did not return a valid handle: ${JSON.stringify(tabResponse)}`);
  assert(handle?.browserId === browserId, `serviceTabHandle browser mismatch: ${JSON.stringify(handle)}`);

  const uiResponse = await serviceRequest(
    {
      action: 'ui_action',
      serviceTabHandle: handle,
      timeoutMs: 5000,
      maxTextBytes: 256,
      uiAction: {
        recipeId: 'generic-ui-smoke',
        maxActions: 12,
        steps: [
          { id: 'find-main', type: 'find', selector: 'main', maxCandidates: 1 },
          { id: 'focus-query', type: 'focus', selector: '#query' },
          { id: 'fill-query', type: 'fill', selector: '#query', value: 'service text' },
          { id: 'select-beta', type: 'select', selector: '#choice', value: 'beta' },
          { id: 'click-apply', type: 'click', selector: '#apply' },
          { id: 'wait-applied', type: 'wait', text: 'Applied service text beta' },
          { id: 'menu-select', type: 'menu_select', selector: '#menu-button', optionSelector: '#menu-option' },
          { id: 'wait-menu', type: 'wait', text: 'Menu option selected' },
        ],
      },
    },
    'ui_action',
  );
  const ui = uiResponse.data;
  assert(ui?.ok === true, `ui_action was not ok: ${JSON.stringify(uiResponse)}`);
  assert(ui?.action === 'ui_action', `ui_action action mismatch: ${JSON.stringify(ui)}`);
  assert(ui?.steps?.length === 8, `ui_action step count mismatch: ${JSON.stringify(ui?.steps)}`);
  assert(ui.steps.every((step) => step.ok === true), `ui_action step failure: ${JSON.stringify(ui.steps)}`);
  assert(ui.steps[0]?.result?.candidates?.[0]?.visible === true, `find evidence missing visible candidate: ${JSON.stringify(ui.steps[0])}`);

  const failureResponse = await serviceRequest(
    {
      action: 'ui_action',
      serviceTabHandle: handle,
      timeoutMs: 1000,
      maxTextBytes: 128,
      captureEvidenceOnFailure: true,
      uiAction: {
        recipeId: 'generic-ui-failure-smoke',
        includeDiagnosticsOnFailure: true,
        steps: [
          { id: 'missing-click', type: 'click', selector: '#missing-button' },
        ],
      },
    },
    'failed ui_action',
  );
  const failure = failureResponse.data;
  assert(failure?.ok === false, `failed ui_action unexpectedly succeeded: ${JSON.stringify(failureResponse)}`);
  assert(failure?.failedStepIndex === 0, `failed ui_action index mismatch: ${JSON.stringify(failure)}`);
  assert(failure?.diagnostics?.ok === true, `failed ui_action missing diagnostics: ${JSON.stringify(failure)}`);
  assert(failure?.diagnostics?.url, `failed ui_action diagnostics missing URL: ${JSON.stringify(failure?.diagnostics)}`);

  const trace = await serviceTrace();
  const uiJobs = (trace?.jobs ?? []).filter((job) => job.action === 'ui_action');
  assert(uiJobs.length >= 2, `service trace missing ui_action jobs: ${JSON.stringify(trace?.jobs ?? [])}`);

  await cleanup();
  console.log(`Service UI action live smoke passed (${browserId}, stream ${streamPort})`);
} catch (err) {
  await fail(err.stack || err.message);
}
