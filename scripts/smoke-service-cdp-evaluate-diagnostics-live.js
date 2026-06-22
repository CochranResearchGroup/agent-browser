#!/usr/bin/env node

import { existsSync } from 'node:fs';
import { request } from 'node:http';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-cdp-eval-diag-',
  sessionPrefix: 'cdp-eval-diag',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'CdpEvaluateDiagnosticsSmoke';
const agentName = 'smoke-agent';
const taskName = 'plan0033Bridge';
const browserId = `session:${session}`;
const pageUrl = smokeDataUrl('Plan 0033 Bridge Smoke', 'Plan 0033 Bridge Smoke');
const screenshotDir = join(context.tempHome, 'diagnostics');
const timeout = setTimeout(() => {
  fail('Timed out waiting for CDP evaluate diagnostics live smoke to complete');
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

async function serviceRequest(body, label) {
  let response;
  try {
    response = await httpJsonWithTimeout(streamPort, 'POST', '/api/service/request', {
      serviceName,
      agentName,
      taskName,
      jobTimeoutMs: 60000,
      ...body,
    }, 90000);
  } catch (err) {
    throw new Error(`${label} failed: ${err.message}`);
  }
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
  return response;
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

function assertTraceJobs(trace) {
  const actions = new Set(
    (trace?.jobs ?? [])
      .map((job) => job.action)
      .filter((action) => typeof action === 'string'),
  );
  for (const action of ['tab_new', 'cdp_attach', 'evaluate', 'diagnostics', 'cdp_detach']) {
    assert(actions.has(action), `service trace missing ${action} job: ${JSON.stringify(trace?.jobs ?? [])}`);
  }
  assert((trace?.matched?.jobs ?? 0) >= 5, `service trace matched job count is too low: ${JSON.stringify(trace)}`);
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
  assert(handle?.valid === true, `tab_new did not return a valid serviceTabHandle: ${JSON.stringify(tabResponse)}`);
  assert(handle?.browserId === browserId, `serviceTabHandle browser mismatch: ${JSON.stringify(handle)}`);
  assert(typeof handle?.targetId === 'string' && handle.targetId, `serviceTabHandle missing targetId: ${JSON.stringify(handle)}`);

  const attachResponse = await serviceRequest(
    {
      action: 'cdp_attach',
      cdpAttachmentAllowed: true,
      serviceTabHandle: handle,
    },
    'cdp_attach',
  );
  assert(attachResponse.data?.attached === true, `cdp_attach did not attach: ${JSON.stringify(attachResponse)}`);
  assert(
    attachResponse.data?.browserProcessPreserved === true,
    `cdp_attach did not preserve browser process: ${JSON.stringify(attachResponse)}`,
  );
  assert(
    attachResponse.data?.detachAction === 'cdp_detach',
    `cdp_attach did not advertise cdp_detach: ${JSON.stringify(attachResponse)}`,
  );

  const titleResponse = await serviceRequest(
    {
      action: 'evaluate',
      serviceTabHandle: handle,
      script: 'document.title',
      returnByValue: true,
      timeoutMs: 2000,
      maxReturnBytes: 128,
    },
    'evaluate title',
  );
  assert(titleResponse.data?.ok === true, `evaluate title was not ok: ${JSON.stringify(titleResponse)}`);
  assert(
    titleResponse.data?.result?.result?.value === 'Plan 0033 Bridge Smoke' ||
      titleResponse.data?.result === 'Plan 0033 Bridge Smoke',
    `evaluate title returned unexpected result: ${JSON.stringify(titleResponse)}`,
  );
  assert(titleResponse.data?.resultTruncated === false, `evaluate title unexpectedly truncated: ${JSON.stringify(titleResponse)}`);

  const truncatedResponse = await serviceRequest(
    {
      action: 'evaluate',
      serviceTabHandle: handle,
      script: "'x'.repeat(512)",
      returnByValue: true,
      timeoutMs: 2000,
      maxReturnBytes: 32,
    },
    'evaluate truncation',
  );
  assert(truncatedResponse.data?.ok === true, `evaluate truncation was not ok: ${JSON.stringify(truncatedResponse)}`);
  assert(
    truncatedResponse.data?.resultTruncated === true,
    `evaluate truncation did not report resultTruncated: ${JSON.stringify(truncatedResponse)}`,
  );
  assert(
    truncatedResponse.data?.resultBytes > truncatedResponse.data?.maxReturnBytes,
    `evaluate truncation byte counts are inconsistent: ${JSON.stringify(truncatedResponse)}`,
  );

  const diagnosticsResponse = await serviceRequest(
    {
      action: 'diagnostics',
      serviceTabHandle: handle,
      includeScreenshot: true,
      screenshotDir,
      maxConsoleEntries: 5,
      maxErrorEntries: 5,
      maxRequestEntries: 5,
    },
    'diagnostics',
  );
  assert(diagnosticsResponse.data?.ok === true, `diagnostics was not ok: ${JSON.stringify(diagnosticsResponse)}`);
  assert(diagnosticsResponse.data?.compact === true, `diagnostics was not compact: ${JSON.stringify(diagnosticsResponse)}`);
  assert(
    diagnosticsResponse.data?.serviceTabHandle?.tabId === handle.tabId,
    `diagnostics did not echo serviceTabHandle: ${JSON.stringify(diagnosticsResponse)}`,
  );
  assert(
    diagnosticsResponse.data?.screenshot?.captured === true,
    `diagnostics did not capture screenshot: ${JSON.stringify(diagnosticsResponse.data?.screenshot)}`,
  );
  assert(
    existsSync(diagnosticsResponse.data.screenshot.path),
    `diagnostics screenshot path does not exist: ${diagnosticsResponse.data.screenshot.path}`,
  );
  assert(Array.isArray(diagnosticsResponse.data?.console?.messages), 'diagnostics console messages missing');
  assert(Array.isArray(diagnosticsResponse.data?.errors?.errors), 'diagnostics errors missing');
  assert(Array.isArray(diagnosticsResponse.data?.requests?.items), 'diagnostics requests missing');
  assert(
    diagnosticsResponse.data?.traceFilter?.browserId === browserId,
    `diagnostics trace filter missing browserId: ${JSON.stringify(diagnosticsResponse.data?.traceFilter)}`,
  );

  const detachResponse = await serviceRequest(
    {
      action: 'cdp_detach',
      serviceTabHandle: handle,
    },
    'cdp_detach',
  );
  assert(detachResponse.data?.detached === true, `cdp_detach did not detach: ${JSON.stringify(detachResponse)}`);
  assert(
    detachResponse.data?.browserProcessPreserved === true,
    `cdp_detach did not preserve browser process: ${JSON.stringify(detachResponse)}`,
  );

  assertTraceJobs(await serviceTrace());

  await cleanup();
  console.log(`Service CDP evaluate diagnostics live smoke passed (${browserId}, stream ${streamPort})`);
} catch (err) {
  await fail(err.stack || err.message);
}
